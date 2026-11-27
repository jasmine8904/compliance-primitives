//! Integration tests: policy-engine chaining all three compliance primitives
//! with AND (All) semantics, plus a smoke test for OR (Any).
//!
//! Scenario: a realistic RWA transfer policy where a transfer is only
//! permitted when the counter-parties are:
//!   1. Present on the allowlist (`allowlist-token.is_allowed`)
//!   2. Not on the denylist (`denylist-gate.check`)
//!   3. In a permitted jurisdiction (`jurisdiction-flag.is_permitted_jurisdiction`)
//!
//! Test matrix
//! ───────────────────────────────────────────────────────────────────────────
//!  test_all_three_pass              — both parties clear all three gates → true
//!  test_fail_allowlist_check        — from is NOT allowlisted → false
//!  test_fail_denylist_check         — from IS denylisted → false
//!  test_fail_jurisdiction_check     — to has a forbidden jurisdiction code → false
//!  test_or_semantics_smoke          — Any: denylist blocks but allowlist passes → true

use crate::{
    AllowlistCheck, CheckKind, CombineOp, DenylistCheck, JurisdictionCheck, PolicyEngine,
    PolicyEngineClient, VerboseResult,
};
use allowlist_token::{AllowlistToken, AllowlistTokenClient};
use denylist_gate::{DenylistGate, DenylistGateClient};
use jurisdiction_flag::{JurisdictionFlag, JurisdictionFlagClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Address, Env, String, Symbol};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Jurisdiction code used for parties that should pass the jurisdiction gate.
const PERMITTED_CODE: &str = "US";
/// Jurisdiction code that is deliberately outside the permitted set.
const FORBIDDEN_CODE: &str = "IR";

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

/// Registers and initialises an allowlist-token (pointing to a mock
/// underlying token address that is never actually called in these tests
/// since we only call `is_allowed`, not `transfer`).
fn setup_allowlist(env: &Env, admin: &Address) -> Address {
    let mock_underlying = Address::generate(env);
    let id = env.register(AllowlistToken, ());
    AllowlistTokenClient::new(env, &id).initialize(admin, &mock_underlying);
    id
}

/// Registers and initialises a denylist-gate contract.
fn setup_denylist(env: &Env, admin: &Address) -> Address {
    let id = env.register(DenylistGate, ());
    DenylistGateClient::new(env, &id).initialize(admin);
    id
}

/// Registers and initialises a jurisdiction-flag contract.
fn setup_jurisdiction(env: &Env, issuer: &Address) -> Address {
    let id = env.register(JurisdictionFlag, ());
    JurisdictionFlagClient::new(env, &id).initialize(issuer);
    id
}

/// Name of the default policy used throughout these integration tests.
const POLICY_NAME: &str = "transfer";

/// Builds and configures a policy-engine with a single named policy
/// (`POLICY_NAME`) using All semantics, wired with all three checks in order:
/// Allowlist → Denylist → Jurisdiction.
///
/// Returns `(engine_admin, engine_client)`.
fn setup_engine_all_three<'a>(
    env: &'a Env,
    allowlist_id: &Address,
    denylist_id: &Address,
    jurisdiction_id: &Address,
    permitted_code: &str,
) -> (Address, PolicyEngineClient<'a>) {
    let admin = Address::generate(env);
    let engine_id = env.register(PolicyEngine, ());
    let client = PolicyEngineClient::new(env, &engine_id);
    client.initialize(&admin);

    let name = Symbol::new(env, POLICY_NAME);
    client.create_policy(&admin, &name, &CombineOp::All, &None);

    client.add_check(
        &admin,
        &name,
        &CheckKind::Allowlist(AllowlistCheck {
            contract: allowlist_id.clone(),
        }),
    );
    client.add_check(
        &admin,
        &name,
        &CheckKind::Denylist(DenylistCheck {
            contract: denylist_id.clone(),
        }),
    );
    client.add_check(
        &admin,
        &name,
        &CheckKind::Jurisdiction(JurisdictionCheck {
            contract: jurisdiction_id.clone(),
            allowed_codes: vec![env, String::from_str(env, permitted_code)],
        }),
    );

    (admin, client)
}

/// Onboard a single address: add to allowlist and set a permitted jurisdiction.
fn onboard(
    env: &Env,
    allowlist_id: &Address,
    allowlist_admin: &Address,
    jurisdiction_id: &Address,
    issuer: &Address,
    who: &Address,
    code: &str,
) {
    AllowlistTokenClient::new(env, allowlist_id).add_to_allowlist(allowlist_admin, who, &None);
    JurisdictionFlagClient::new(env, jurisdiction_id)
        .set_jurisdiction(issuer, who, &String::from_str(env, code));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Happy path: both `from` and `to` are allowlisted, not denylisted, and in a
/// permitted jurisdiction.  The policy-engine must return `true`.
#[test]
fn test_all_three_pass() {
    let env = Env::default();
    env.mock_all_auths();

    let allowlist_admin = Address::generate(&env);
    let denylist_admin = Address::generate(&env);
    let issuer = Address::generate(&env);

    let allowlist_id = setup_allowlist(&env, &allowlist_admin);
    let denylist_id = setup_denylist(&env, &denylist_admin);
    let jurisdiction_id = setup_jurisdiction(&env, &issuer);

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    // Both parties fully onboarded.
    onboard(
        &env,
        &allowlist_id,
        &allowlist_admin,
        &jurisdiction_id,
        &issuer,
        &from,
        PERMITTED_CODE,
    );
    onboard(
        &env,
        &allowlist_id,
        &allowlist_admin,
        &jurisdiction_id,
        &issuer,
        &to,
        PERMITTED_CODE,
    );

    let (_admin, engine) =
        setup_engine_all_three(&env, &allowlist_id, &denylist_id, &jurisdiction_id, PERMITTED_CODE);

    assert!(
        engine.evaluate_policy(&Symbol::new(&env, POLICY_NAME), &from, &to),
        "expected all-three-pass to return true"
    );
}

/// Allowlist failure: `from` is not on the allowlist.  All other checks would
/// pass; the engine must return `false` because the allowlist check fails.
#[test]
fn test_fail_allowlist_check() {
    let env = Env::default();
    env.mock_all_auths();

    let allowlist_admin = Address::generate(&env);
    let denylist_admin = Address::generate(&env);
    let issuer = Address::generate(&env);

    let allowlist_id = setup_allowlist(&env, &allowlist_admin);
    let denylist_id = setup_denylist(&env, &denylist_admin);
    let jurisdiction_id = setup_jurisdiction(&env, &issuer);

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    // `to` is fully onboarded.
    onboard(
        &env,
        &allowlist_id,
        &allowlist_admin,
        &jurisdiction_id,
        &issuer,
        &to,
        PERMITTED_CODE,
    );
    // `from` has a permitted jurisdiction but is intentionally NOT added to
    // the allowlist — this is the gate that should trip the policy.
    JurisdictionFlagClient::new(&env, &jurisdiction_id)
        .set_jurisdiction(&issuer, &from, &String::from_str(&env, PERMITTED_CODE));

    let (_admin, engine) =
        setup_engine_all_three(&env, &allowlist_id, &denylist_id, &jurisdiction_id, PERMITTED_CODE);

    assert!(
        !engine.evaluate_policy(&Symbol::new(&env, POLICY_NAME), &from, &to),
        "expected allowlist failure to return false"
    );
}

/// Denylist failure: `from` is denylisted.  Both parties are allowlisted and
/// in a permitted jurisdiction; only the denylist gate trips.
#[test]
fn test_fail_denylist_check() {
    let env = Env::default();
    env.mock_all_auths();

    let allowlist_admin = Address::generate(&env);
    let denylist_admin = Address::generate(&env);
    let issuer = Address::generate(&env);

    let allowlist_id = setup_allowlist(&env, &allowlist_admin);
    let denylist_id = setup_denylist(&env, &denylist_admin);
    let jurisdiction_id = setup_jurisdiction(&env, &issuer);

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    // Both parties fully onboarded.
    onboard(
        &env,
        &allowlist_id,
        &allowlist_admin,
        &jurisdiction_id,
        &issuer,
        &from,
        PERMITTED_CODE,
    );
    onboard(
        &env,
        &allowlist_id,
        &allowlist_admin,
        &jurisdiction_id,
        &issuer,
        &to,
        PERMITTED_CODE,
    );

    // `from` is added to the denylist — this should cause the policy to fail.
    DenylistGateClient::new(&env, &denylist_id)
        .add_to_denylist(&denylist_admin, &from);

    let (_admin, engine) =
        setup_engine_all_three(&env, &allowlist_id, &denylist_id, &jurisdiction_id, PERMITTED_CODE);

    assert!(
        !engine.evaluate_policy(&Symbol::new(&env, POLICY_NAME), &from, &to),
        "expected denylist failure to return false"
    );
}

/// Jurisdiction failure: `to` has a forbidden jurisdiction code (not in the
/// permitted list).  Both parties are allowlisted and neither is denylisted.
#[test]
fn test_fail_jurisdiction_check() {
    let env = Env::default();
    env.mock_all_auths();

    let allowlist_admin = Address::generate(&env);
    let denylist_admin = Address::generate(&env);
    let issuer = Address::generate(&env);

    let allowlist_id = setup_allowlist(&env, &allowlist_admin);
    let denylist_id = setup_denylist(&env, &denylist_admin);
    let jurisdiction_id = setup_jurisdiction(&env, &issuer);

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    // `from` fully onboarded with a permitted code.
    onboard(
        &env,
        &allowlist_id,
        &allowlist_admin,
        &jurisdiction_id,
        &issuer,
        &from,
        PERMITTED_CODE,
    );
    // `to` is allowlisted but is in a forbidden jurisdiction.
    AllowlistTokenClient::new(&env, &allowlist_id)
        .add_to_allowlist(&allowlist_admin, &to, &None);
    JurisdictionFlagClient::new(&env, &jurisdiction_id)
        .set_jurisdiction(&issuer, &to, &String::from_str(&env, FORBIDDEN_CODE));

    let (_admin, engine) =
        setup_engine_all_three(&env, &allowlist_id, &denylist_id, &jurisdiction_id, PERMITTED_CODE);

    assert!(
        !engine.evaluate_policy(&Symbol::new(&env, POLICY_NAME), &from, &to),
        "expected jurisdiction failure to return false"
    );
}

/// OR semantics smoke test: the denylist check fails for both parties
/// (both are on the denylist) but the allowlist check passes for both.
/// With `CombineOp::Any` at least one check must pass for both addresses,
/// so the engine must return `true`.
#[test]
fn test_or_semantics_any_check_passes() {
    let env = Env::default();
    env.mock_all_auths();

    let allowlist_admin = Address::generate(&env);
    let denylist_admin = Address::generate(&env);
    let issuer = Address::generate(&env);

    let allowlist_id = setup_allowlist(&env, &allowlist_admin);
    let denylist_id = setup_denylist(&env, &denylist_admin);
    // jurisdiction_id is set up but not added as a check in this test.
    let _jurisdiction_id = setup_jurisdiction(&env, &issuer);

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    // Both parties are allowlisted (allowlist check will pass).
    AllowlistTokenClient::new(&env, &allowlist_id)
        .add_to_allowlist(&allowlist_admin, &from, &None);
    AllowlistTokenClient::new(&env, &allowlist_id)
        .add_to_allowlist(&allowlist_admin, &to, &None);

    // Both parties are also denylisted (denylist check will fail).
    DenylistGateClient::new(&env, &denylist_id)
        .add_to_denylist(&denylist_admin, &from);
    DenylistGateClient::new(&env, &denylist_id)
        .add_to_denylist(&denylist_admin, &to);

    // Set up a policy-engine with Any semantics:
    // Allowlist → Denylist order, same two checks.
    let engine_admin = Address::generate(&env);
    let engine_id = env.register(PolicyEngine, ());
    let engine = PolicyEngineClient::new(&env, &engine_id);
    engine.initialize(&engine_admin);

    let name = Symbol::new(&env, POLICY_NAME);
    engine.create_policy(&engine_admin, &name, &CombineOp::Any, &None);

    engine.add_check(
        &engine_admin,
        &name,
        &CheckKind::Allowlist(AllowlistCheck {
            contract: allowlist_id.clone(),
        }),
    );
    engine.add_check(
        &engine_admin,
        &name,
        &CheckKind::Denylist(DenylistCheck {
            contract: denylist_id.clone(),
        }),
    );
    // Jurisdiction check intentionally omitted — both parties lack a code,
    // so if it were included as an Any check it would not be the passing one.
    // The allowlist check alone is the passing gate here.

    // With Any: allowlist passes for both → result should be true even though
    // denylist fails.
    assert!(
        engine.evaluate_policy(&name, &from, &to),
        "expected Any semantics to return true when at least one check passes for both parties"
    );
}

/// Verbose reporting under `All`: only the denylist gate trips, so
/// `evaluate_verbose` must blame exactly that check (index 1, Denylist kind)
/// rather than an arbitrary one.
#[test]
fn test_verbose_all_reports_failing_check() {
    let env = Env::default();
    env.mock_all_auths();

    let allowlist_admin = Address::generate(&env);
    let denylist_admin = Address::generate(&env);
    let issuer = Address::generate(&env);

    let allowlist_id = setup_allowlist(&env, &allowlist_admin);
    let denylist_id = setup_denylist(&env, &denylist_admin);
    let jurisdiction_id = setup_jurisdiction(&env, &issuer);

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    // Both parties fully onboarded.
    onboard(
        &env,
        &allowlist_id,
        &allowlist_admin,
        &jurisdiction_id,
        &issuer,
        &from,
        PERMITTED_CODE,
    );
    onboard(
        &env,
        &allowlist_id,
        &allowlist_admin,
        &jurisdiction_id,
        &issuer,
        &to,
        PERMITTED_CODE,
    );

    // Only the denylist gate trips (check index 1).
    DenylistGateClient::new(&env, &denylist_id).add_to_denylist(&denylist_admin, &from);

    let (_admin, engine) =
        setup_engine_all_three(&env, &allowlist_id, &denylist_id, &jurisdiction_id, PERMITTED_CODE);

    match engine.evaluate_verbose(&Symbol::new(&env, POLICY_NAME), &from, &to) {
        VerboseResult::Failed(failure) => {
            assert_eq!(
                failure.check_index, 1,
                "All failure must blame the denylist check at index 1"
            );
            assert!(
                matches!(failure.kind, CheckKind::Denylist(_)),
                "All failure kind must identify the denylist check"
            );
        }
        VerboseResult::Passed => panic!("expected All policy to fail"),
    }
}

/// Verbose reporting under `Any`: every check fails, so the engine must not
/// blame an arbitrary check.  It reports the `u32::MAX` / `"AllFailed"`
/// sentinel instead.
#[test]
fn test_verbose_any_all_failed_sentinel() {
    let env = Env::default();
    env.mock_all_auths();

    let allowlist_admin = Address::generate(&env);
    let denylist_admin = Address::generate(&env);
    let issuer = Address::generate(&env);

    let allowlist_id = setup_allowlist(&env, &allowlist_admin);
    let denylist_id = setup_denylist(&env, &denylist_admin);
    let jurisdiction_id = setup_jurisdiction(&env, &issuer);

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    // Neither party is allowlisted, both are denylisted, and neither has a
    // permitted jurisdiction — every check fails for both parties.
    DenylistGateClient::new(&env, &denylist_id).add_to_denylist(&denylist_admin, &from);
    DenylistGateClient::new(&env, &denylist_id).add_to_denylist(&denylist_admin, &to);
    JurisdictionFlagClient::new(&env, &jurisdiction_id)
        .set_jurisdiction(&issuer, &from, &String::from_str(&env, FORBIDDEN_CODE));
    JurisdictionFlagClient::new(&env, &jurisdiction_id)
        .set_jurisdiction(&issuer, &to, &String::from_str(&env, FORBIDDEN_CODE));

    // Build an Any engine wired with all three checks.
    let engine_admin = Address::generate(&env);
    let engine_id = env.register(PolicyEngine, ());
    let engine = PolicyEngineClient::new(&env, &engine_id);
    engine.initialize(&engine_admin);

    let name = Symbol::new(&env, POLICY_NAME);
    engine.create_policy(&engine_admin, &name, &CombineOp::Any, &None);
    engine.add_check(
        &engine_admin,
        &name,
        &CheckKind::Allowlist(AllowlistCheck {
            contract: allowlist_id.clone(),
        }),
    );
    engine.add_check(
        &engine_admin,
        &name,
        &CheckKind::Denylist(DenylistCheck {
            contract: denylist_id.clone(),
        }),
    );
    engine.add_check(
        &engine_admin,
        &name,
        &CheckKind::Jurisdiction(JurisdictionCheck {
            contract: jurisdiction_id.clone(),
            allowed_codes: vec![&env, String::from_str(&env, PERMITTED_CODE)],
        }),
    );

    match engine.evaluate_verbose(&name, &from, &to) {
        VerboseResult::Failed(failure) => {
            assert_eq!(
                failure.check_index,
                u32::MAX,
                "Any failure must use the AllFailed sentinel index, not an arbitrary check"
            );
            assert_eq!(
                failure.kind,
                CheckKind::AllFailed,
                "Any failure kind must be the AllFailed sentinel"
            );
        }
        VerboseResult::Passed => panic!("expected Any policy to fail when every check fails"),
    }
}

/// Amount-aware evaluation: the policy combines the allowlist, denylist and
/// jurisdiction gates with an amount-aware check.  A transfer below the
/// travel-rule threshold passes; one above it is rejected.  The result also
/// carries the monotonic `policy_version`, which must be bumped whenever the
/// policy changes.
#[test]
fn test_evaluate_with_amount_and_version() {
    let env = Env::default();
    env.mock_all_auths();

    let allowlist_admin = Address::generate(&env);
    let denylist_admin = Address::generate(&env);
    let issuer = Address::generate(&env);

    let allowlist_id = setup_allowlist(&env, &allowlist_admin);
    let denylist_id = setup_denylist(&env, &denylist_admin);
    let jurisdiction_id = setup_jurisdiction(&env, &issuer);

    let from = Address::generate(&env);
    let to = Address::generate(&env);

    onboard(
        &env,
        &allowlist_id,
        &allowlist_admin,
        &jurisdiction_id,
        &issuer,
        &from,
        PERMITTED_CODE,
    );
    onboard(
        &env,
        &allowlist_id,
        &allowlist_admin,
        &jurisdiction_id,
        &issuer,
        &to,
        PERMITTED_CODE,
    );

    let (admin, engine) =
        setup_engine_all_three(&env, &allowlist_id, &denylist_id, &jurisdiction_id, PERMITTED_CODE);
    let name = Symbol::new(&env, POLICY_NAME);

    // The version starts at 1 after the policy is created and its checks added.
    let version_before = engine.get_policy(&name).policy_version;
    assert!(version_before >= 1, "policy_version must be monotonic from 1");

    // Below the travel-rule threshold: all gates pass.
    let small = engine.evaluate_with_amount(&name, &from, &to, &Some(100));
    assert!(small.allowed, "small transfer must be allowed");
    assert_eq!(
        small.policy_version, version_before,
        "evaluation must report the current policy version"
    );

    // Above the travel-rule threshold: the amount-aware check rejects it.
    let large = engine.evaluate_with_amount(&name, &from, &to, &Some(1_000_000));
    assert!(!large.allowed, "large transfer must be rejected");
    assert_eq!(
        large.policy_version, version_before,
        "rejection must still report the current policy version"
    );

    // Changing the policy bumps the version; subsequent evaluations report it.
    engine.add_check(
        &admin,
        &name,
        &CheckKind::Denylist(DenylistCheck {
            contract: denylist_id.clone(),
        }),
    );
    let version_after = engine.get_policy(&name).policy_version;
    assert!(
        version_after > version_before,
        "policy_version must increase on every policy change"
    );

    let result = engine.evaluate_with_amount(&name, &from, &to, &Some(100));
    assert_eq!(
        result.policy_version, version_after,
        "evaluation must report the bumped policy version"
    );
}
