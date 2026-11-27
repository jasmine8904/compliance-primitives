//! Lightweight sequence fuzzer for `policy-engine` tree configuration and evaluation.
extern crate std;

use super::*;
use denylist_gate::{DenylistGate, DenylistGateClient};
use jurisdiction_flag::{JurisdictionFlag, JurisdictionFlagClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Env};
use std::string::ToString;

/// Tiny xorshift32 so we don't need an extra RNG crate in tests.
fn next_u32(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = if x == 0 { 0x9E37_79B9 } else { x };
    *state
}

fn next_usize(state: &mut u32, upper: usize) -> usize {
    (next_u32(state) as usize) % upper
}

#[test]
fn fuzz_policy_engine_tree_and_evaluation() {
    let iterations: u32 = std::env::var("FUZZ_ITERATIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(128);
    let ops_per_iter: u32 = std::env::var("FUZZ_OPS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(32);

    let permitted_codes = ["US", "CA"];
    let all_codes = ["US", "CA", "DE", "JP", "FR"];

    for seed in 1..=iterations {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let engine_id = env.register(PolicyEngine, ());
        let client = PolicyEngineClient::new(&env, &engine_id);

        let mut rng = seed;

        // 1. Determine CombineOp semantics randomly
        let op = if next_u32(&mut rng) % 2 == 0 {
            CombineOp::All
        } else {
            CombineOp::Any
        };
        client.initialize(&admin, &op, &None);

        // 1b. Create a named policy and add checks to it. The fuzzer exercises
        // both the default (unnamed) policy and this named policy.
        let policy_name = String::from_str(&env, "fuzz-policy");
        client.create_policy(&admin, &policy_name, &op, &None);

        // 2. Setup multiple denylist gates and jurisdiction flags
        let num_gates = 2;
        let mut gates = std::vec::Vec::new();
        let gate_admin = Address::generate(&env);
        for _ in 0..num_gates {
            let id = env.register(DenylistGate, ());
            DenylistGateClient::new(&env, &id).initialize(&gate_admin);
            gates.push(id);
        }

        let num_flags = 2;
        let mut flags = std::vec::Vec::new();
        let flag_issuer = Address::generate(&env);
        for _ in 0..num_flags {
            let id = env.register(JurisdictionFlag, ());
            JurisdictionFlagClient::new(&env, &id).initialize(&flag_issuer);
            flags.push(id);
        }

        // 3. Pool of addresses
        let num_addresses = 5;
        let mut addresses = std::vec::Vec::new();
        for _ in 0..num_addresses {
            addresses.push(Address::generate(&env));
        }

        // Track states locally:
        // - denylist_states[gate_index][addr_index] = is_denylisted
        let mut denylist_states = std::vec![std::vec![false; num_addresses]; num_gates];
        // - jurisdiction_states[flag_index][addr_index] = code_index
        // We initialize all addresses to code 0 ("US") initially.
        let mut jurisdiction_states = std::vec![std::vec![0; num_addresses]; num_flags];
        for f_i in 0..num_flags {
            for addr_i in 0..num_addresses {
                let code_str = all_codes[0];
                JurisdictionFlagClient::new(&env, &flags[f_i]).set_jurisdiction(
                    &flag_issuer,
                    &addresses[addr_i],
                    &String::from_str(&env, code_str),
                );
            }
        }

        // Model representation of registered checks (default policy)
        let mut registered_checks = std::vec::Vec::new();
        // Model representation of checks registered under the named policy
        let mut named_checks = std::vec::Vec::new();

        // 4. Random operations loop
        for _ in 0..ops_per_iter {
            let action = next_u32(&mut rng) % 5;
            match action {
                0 => {
                    // Add check, either to the default policy or the named one.
                    let to_named = next_u32(&mut rng) % 2 == 0;
                    let check = if next_u32(&mut rng) % 2 == 0 {
                        let gate_idx = next_usize(&mut rng, num_gates);
                        CheckKind::Denylist(DenylistCheck {
                            contract: gates[gate_idx].clone(),
                        })
                    } else {
                        let flag_idx = next_usize(&mut rng, num_flags);
                        let allowed_codes_vec = vec![
                            &env,
                            String::from_str(&env, permitted_codes[0]),
                            String::from_str(&env, permitted_codes[1]),
                        ];
                        CheckKind::Jurisdiction(JurisdictionCheck {
                            contract: flags[flag_idx].clone(),
                            allowed_codes: allowed_codes_vec,
                        })
                    };
                    if to_named {
                        client.add_policy_check(&admin, &policy_name, &check);
                        named_checks.push(check);
                    } else {
                        client.add_check(&admin, &check);
                        registered_checks.push(check);
                    }
                }
                1 => {
                    // Remove check from either the default or named policy.
                    let from_named = next_u32(&mut rng) % 2 == 0;
                    if from_named {
                        if !named_checks.is_empty() {
                            let idx = next_usize(&mut rng, named_checks.len());
                            client.remove_policy_check(&admin, &policy_name, &(idx as u32));
                            named_checks.remove(idx);
                        }
                    } else if !registered_checks.is_empty() {
                        let idx = next_usize(&mut rng, registered_checks.len());
                        client.remove_check(&admin, &(idx as u32));
                        registered_checks.remove(idx);
                    }
                }
                2 => {
                    // Modify denylist state
                    let gate_idx = next_usize(&mut rng, num_gates);
                    let addr_idx = next_usize(&mut rng, num_addresses);
                    let current = denylist_states[gate_idx][addr_idx];
                    let gate_client = DenylistGateClient::new(&env, &gates[gate_idx]);
                    if current {
                        gate_client.remove_from_denylist(&gate_admin, &addresses[addr_idx]);
                    } else {
                        gate_client.add_to_denylist(&gate_admin, &addresses[addr_idx]);
                    }
                    denylist_states[gate_idx][addr_idx] = !current;
                }
                3 => {
                    // Modify jurisdiction state
                    let flag_idx = next_usize(&mut rng, num_flags);
                    let addr_idx = next_usize(&mut rng, num_addresses);
                    let new_code_idx = next_usize(&mut rng, all_codes.len());
                    let code_str = all_codes[new_code_idx];
                    let flag_client = JurisdictionFlagClient::new(&env, &flags[flag_idx]);
                    flag_client.set_jurisdiction(
                        &flag_issuer,
                        &addresses[addr_idx],
                        &String::from_str(&env, code_str),
                    );
                    jurisdiction_states[flag_idx][addr_idx] = new_code_idx;
                }
                4 => {
                    // Evaluate random transfer pair
                    let from_idx = next_usize(&mut rng, num_addresses);
                    let to_idx = next_usize(&mut rng, num_addresses);

                    let result = client.evaluate(&addresses[from_idx], &addresses[to_idx]);

                    // Verify against model
                    let expected = evaluate_model(
                        &op,
                        &registered_checks,
                        from_idx,
                        to_idx,
                        &gates,
                        &flags,
                        &denylist_states,
                        &jurisdiction_states,
                        &permitted_codes,
                        &all_codes,
                    );
                    assert_eq!(
                        result, expected,
                        "seed={seed}: mismatch on evaluate from={from_idx} to={to_idx}"
                    );

                    // Verify verbose evaluation agrees with the bool result and
                    // follows the documented failure-reporting semantics.
                    let verbose =
                        client.evaluate_verbose(&addresses[from_idx], &addresses[to_idx]);
                    assert_eq!(
                        verbose.allowed, result,
                        "seed={seed}: verbose allowed mismatch from={from_idx} to={to_idx}"
                    );
                    match (&op, result) {
                        (CombineOp::All, false) => {
                            let failure = verbose
                                .failure
                                .expect("All failure must report a CheckFailure");
                            assert!(
                                (failure.check_index as usize) < registered_checks.len(),
                                "seed={seed}: All failure check_index {} out of range (len {})",
                                failure.check_index,
                                registered_checks.len()
                            );
                            let idx = failure.check_index as usize;
                            assert!(
                                !check_passes_model(
                                    &registered_checks[idx],
                                    from_idx,
                                    to_idx,
                                    &gates,
                                    &flags,
                                    &denylist_states,
                                    &jurisdiction_states,
                                    &all_codes,
                                ),
                                "seed={seed}: All failure blamed a passing check at index {idx}"
                            );
                        }
                        (CombineOp::Any, false) => {
                            let failure = verbose
                                .failure
                                .expect("Any failure must report a CheckFailure");
                            assert_eq!(
                                failure.check_index,
                                u32::MAX,
                                "seed={seed}: Any failure must use check_index u32::MAX"
                            );
                            assert_eq!(
                                failure.kind,
                                String::from_str(&env, "AllFailed"),
                                "seed={seed}: Any failure must use kind AllFailed"
                            );
                        }
                        (_, true) => {
                            assert!(
                                verbose.failure.is_none(),
                                "seed={seed}: passing evaluation must not report a failure"
                            );
                        }
                        _ => {}
                    }

                    // Test batch evaluate
                    let mut pairs = Vec::new(&env);
                    pairs.push_back(AddressPair {
                        from: addresses[from_idx].clone(),
                        to: addresses[to_idx].clone(),
                    });
                    pairs.push_back(AddressPair {
                        from: addresses[to_idx].clone(),
                        to: addresses[from_idx].clone(),
                    });

                    let batch_results = client.batch_evaluate(&pairs);
                    assert_eq!(batch_results.len(), 2);
                    assert_eq!(batch_results.get(0).unwrap(), result);

                    let expected_rev = evaluate_model(
                        &op,
                        &registered_checks,
                        to_idx,
                        from_idx,
                        &gates,
                        &flags,
                        &denylist_states,
                        &jurisdiction_states,
                        &permitted_codes,
                        &all_codes,
                    );
                    assert_eq!(batch_results.get(1).unwrap(), expected_rev);

                    // Evaluate the named policy and verify against its model.
                    let named_result =
                        client.evaluate_policy(&policy_name, &addresses[from_idx], &addresses[to_idx]);
                    let named_expected = evaluate_model(
                        &op,
                        &named_checks,
                        from_idx,
                        to_idx,
                        &gates,
                        &flags,
                        &denylist_states,
                        &jurisdiction_states,
                        &permitted_codes,
                        &all_codes,
                    );
                    assert_eq!(
                        named_result, named_expected,
                        "seed={seed}: named policy mismatch from={from_idx} to={to_idx}"
                    );

                    // Exercise the amount-aware entry point. The fuzz harness
                    // registers no amount-sensitive checks, so the result must
                    // match the amount-less evaluation.
                    let amount = (next_u32(&mut rng) % 1_000_000) as i128;
                    let with_amount = client.evaluate_with_amount(
                        &addresses[from_idx],
                        &addresses[to_idx],
                        &Some(amount),
                    );
                    assert_eq!(
                        with_amount, result,
                        "seed={seed}: amount-aware evaluate mismatch from={from_idx} to={to_idx}"
                    );

                    let named_with_amount = client.evaluate_policy_with_amount(
                        &policy_name,
                        &addresses[from_idx],
                        &addresses[to_idx],
                        &Some(amount),
                    );
                    assert_eq!(
                        named_with_amount, named_expected,
                        "seed={seed}: named amount-aware evaluate mismatch from={from_idx} to={to_idx}"
                    );
                }
                _ => unreachable!(),
            }
        }
    }
}

/// Returns whether a single check passes for the given transfer under the model.
#[allow(clippy::too_many_arguments)]
fn check_passes_model(
    check: &CheckKind,
    from_idx: usize,
    to_idx: usize,
    gates: &[Address],
    flags: &[Address],
    denylist_states: &[std::vec::Vec<bool>],
    jurisdiction_states: &[std::vec::Vec<usize>],
    all_codes: &[&str],
) -> bool {
    let run_check_model = |check: &CheckKind, addr_idx: usize| -> bool {
        match check {
            CheckKind::Denylist(inner) => {
                let gate_idx = gates.iter().position(|id| id == &inner.contract).unwrap();
                let is_denylisted = denylist_states[gate_idx][addr_idx];
                !is_denylisted
            }
            CheckKind::Jurisdiction(inner) => {
                let flag_idx = flags.iter().position(|id| id == &inner.contract).unwrap();
                let code_idx = jurisdiction_states[flag_idx][addr_idx];
                let code_str = all_codes[code_idx];
                inner.allowed_codes.iter().any(|code| code.to_string() == code_str)
            }
            CheckKind::Allowlist(_) => true,
            // The fuzz harness never registers circuit-breaker checks.
            CheckKind::CircuitBreaker(_) => true,
        }
    };

    run_check_model(check, from_idx) && run_check_model(check, to_idx)
}

#[allow(clippy::too_many_arguments)]
fn evaluate_model(
    op: &CombineOp,
    checks: &[CheckKind],
    from_idx: usize,
    to_idx: usize,
    gates: &[Address],
    flags: &[Address],
    denylist_states: &[std::vec::Vec<bool>],
    jurisdiction_states: &[std::vec::Vec<usize>],
    _permitted_codes: &[&str],
    all_codes: &[&str],
) -> bool {
    let run_check_model = |check: &CheckKind, addr_idx: usize| -> bool {
        match check {
            CheckKind::Denylist(inner) => {
                let gate_idx = gates.iter().position(|id| id == &inner.contract).unwrap();
                let is_denylisted = denylist_states[gate_idx][addr_idx];
                !is_denylisted
            }
            CheckKind::Jurisdiction(inner) => {
                let flag_idx = flags.iter().position(|id| id == &inner.contract).unwrap();
                let code_idx = jurisdiction_states[flag_idx][addr_idx];
                let code_str = all_codes[code_idx];
                inner.allowed_codes.iter().any(|code| code.to_string() == code_str)
            }
            CheckKind::Allowlist(_) => true,
            // The fuzz harness never registers circuit-breaker checks.
            CheckKind::CircuitBreaker(_) => true,
        }
    };

    match op {
        CombineOp::All => {
            let mut all_pass = true;
            for check in checks {
                if !run_check_model(check, from_idx) || !run_check_model(check, to_idx) {
                    all_pass = false;
                    break;
                }
            }
            all_pass
        }
        CombineOp::Any => {
            if checks.is_empty() {
                false
            } else {
                let mut any_pass = false;
                for check in checks {
                    if run_check_model(check, from_idx) && run_check_model(check, to_idx) {
                        any_pass = true;
                        break;
                    }
                }
                any_pass
            }
        }
    }
}
