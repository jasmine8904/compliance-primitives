//! Property-based fuzz harness for denylist-gate add/remove/check sequences.
//!
//! # Approach
//!
//! We use a lightweight in-process harness (plain `#[test]` + deterministic
//! PRNG) rather than `cargo-fuzz`/libFuzzer. A `#![no_std]` Soroban contract
//! crate cannot easily host a libFuzzer target in the same workspace without
//! pulling in `std`, and the invariant under test is simple enough that a
//! seeded loop gives strong coverage with fast CI-friendly defaults.
//!
//! # Invariant ("last write wins")
//!
//! For each address, after any sequence of `add_to_denylist`,
//! `remove_from_denylist`, and `check` calls:
//! - `check(addr)` returns `true` (clear) iff the most recent mutating call
//!   for `addr` was a remove, or `addr` was never added.
//! - `check(addr)` returns `false` (denied) iff the most recent mutating call
//!   for `addr` was an add.
//!
//! # Running
//!
//! Default (500 random sequences, suitable for `cargo test`):
//!
//! ```sh
//! cargo test -p denylist-gate fuzz_denylist_sequences
//! ```
//!
//! Longer local run: increase `DEFAULT_ITERATIONS` in this file (not wired into CI).

use super::*;
use proptest::prelude::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, Vec};

const DEFAULT_ITERATIONS: u32 = 500;
const MAX_ADDRESSES: usize = 5;
const MAX_SEQUENCE_LEN: usize = 64;

#[derive(Clone, Copy)]
enum Op {
    Add,
    Remove,
    Check,
}

/// Extended operation set used by the proptest suites. Covers the full state
/// machine: denylist mutations, pause/unpause, and admin rotation.
#[derive(Clone, Copy, Debug)]
enum PropOp {
    Add,
    Remove,
    Check,
    Pause,
    Unpause,
    ChangeAdmin,
}

impl PropOp {
    fn from_u8(v: u8) -> Self {
        match v % 6 {
            0 => PropOp::Add,
            1 => PropOp::Remove,
            2 => PropOp::Check,
            3 => PropOp::Pause,
            4 => PropOp::Unpause,
            _ => PropOp::ChangeAdmin,
        }
    }
}

/// Deterministic proptest configuration: fixed seed, fixed case count.
fn fixed_config() -> ProptestConfig {
    ProptestConfig {
        cases: 128,
        rng_seed: ProptestRngSeed::Fixed(0xDEAD_BEEF),
        ..ProptestConfig::default()
    }
}

/// Strategy producing a random sequence of `PropOp`s paired with an address
/// index in `0..MAX_ADDRESSES`.
fn op_sequence() -> impl Strategy<Value = std::vec::Vec<(u8, usize)>> {
    prop::collection::vec((any::<u8>(), 0usize..MAX_ADDRESSES), 1..=MAX_SEQUENCE_LEN)
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545_F491_4F6C_DD1D)) >> 32) as u32
    }

    fn gen_usize(&mut self, upper: usize) -> usize {
        (self.next_u32() as usize) % upper
    }
}

fn run_sequence(seed: u64) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(DenylistGate, ());
    let client = DenylistGateClient::new(&env, &contract_id);
    client.initialize(&admin);

    let mut rng = Rng::new(seed);
    let mut addresses = Vec::new(&env);
    for _ in 0..MAX_ADDRESSES {
        addresses.push_back(Address::generate(&env));
    }
    let mut denied = [false; MAX_ADDRESSES];

    let seq_len = rng.gen_usize(MAX_SEQUENCE_LEN) + 1;
    for _ in 0..seq_len {
        let addr_idx = rng.gen_usize(MAX_ADDRESSES);
        let address = addresses.get(addr_idx as u32).unwrap();
        let op = match rng.gen_usize(3) {
            0 => Op::Add,
            1 => Op::Remove,
            _ => Op::Check,
        };

        match op {
            Op::Add => {
                client.add_to_denylist(&admin, &address);
                denied[addr_idx] = true;
            }
            Op::Remove => {
                client.remove_from_denylist(&admin, &address);
                denied[addr_idx] = false;
            }
            Op::Check => {
                let expected_clear = !denied[addr_idx];
                assert_eq!(
                    client.check(&address),
                    expected_clear,
                    "seed={seed} addr_idx={addr_idx}"
                );
            }
        }
    }

    for idx in 0..MAX_ADDRESSES {
        let address = addresses.get(idx as u32).unwrap();
        assert_eq!(
            client.check(&address),
            !denied[idx],
            "seed={seed} final addr_idx={idx}"
        );
    }
}

#[test]
fn fuzz_denylist_sequences() {
    for seed in 0..DEFAULT_ITERATIONS {
        run_sequence(seed as u64);
    }
}

// ---------------------------------------------------------------------------
// proptest suites
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(fixed_config())]

    /// Invariant: after any sequence of add/remove/check, `check(addr)` is
    /// `true` iff the address is not in the denylist, and the tracked set of
    /// denied addresses matches the contract state exactly (count consistency).
    #[test]
    fn prop_denylist_membership_matches_set(ops in op_sequence()) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(DenylistGate, ());
        let client = DenylistGateClient::new(&env, &contract_id);
        client.initialize(&admin);

        let mut addresses = Vec::new(&env);
        for _ in 0..MAX_ADDRESSES {
            addresses.push_back(Address::generate(&env));
        }
        let mut denied = [false; MAX_ADDRESSES];

        for (raw, idx) in ops {
            let address = addresses.get(idx as u32).unwrap();
            match PropOp::from_u8(raw) {
                PropOp::Add => {
                    client.add_to_denylist(&admin, &address);
                    denied[idx] = true;
                }
                PropOp::Remove => {
                    client.remove_from_denylist(&admin, &address);
                    denied[idx] = false;
                }
                PropOp::Check => {
                    prop_assert_eq!(client.check(&address), !denied[idx]);
                }
                // Pause/unpause/admin changes do not affect membership.
                PropOp::Pause | PropOp::Unpause | PropOp::ChangeAdmin => {}
            }
        }

        // Count consistency: exactly the tracked addresses are denied.
        let mut denied_count = 0usize;
        for idx in 0..MAX_ADDRESSES {
            let address = addresses.get(idx as u32).unwrap();
            let clear = client.check(&address);
            prop_assert_eq!(clear, !denied[idx]);
            if !clear {
                denied_count += 1;
            }
        }
        let expected_count = denied.iter().filter(|d| **d).count();
        prop_assert_eq!(denied_count, expected_count);
    }

    /// Invariant: while paused, every write (add/remove) is rejected and the
    /// denylist state is unchanged. Unpausing restores write access.
    #[test]
    fn prop_pause_blocks_writes(ops in op_sequence()) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(DenylistGate, ());
        let client = DenylistGateClient::new(&env, &contract_id);
        client.initialize(&admin);

        let mut addresses = Vec::new(&env);
        for _ in 0..MAX_ADDRESSES {
            addresses.push_back(Address::generate(&env));
        }
        let mut denied = [false; MAX_ADDRESSES];
        let mut paused = false;

        for (raw, idx) in ops {
            let address = addresses.get(idx as u32).unwrap();
            match PropOp::from_u8(raw) {
                PropOp::Add => {
                    if paused {
                        // Writes must be rejected while paused.
                        let res = client.try_add_to_denylist(&admin, &address);
                        prop_assert!(res.is_err());
                    } else {
                        client.add_to_denylist(&admin, &address);
                        denied[idx] = true;
                    }
                }
                PropOp::Remove => {
                    if paused {
                        let res = client.try_remove_from_denylist(&admin, &address);
                        prop_assert!(res.is_err());
                    } else {
                        client.remove_from_denylist(&admin, &address);
                        denied[idx] = false;
                    }
                }
                PropOp::Check => {
                    prop_assert_eq!(client.check(&address), !denied[idx]);
                }
                PropOp::Pause => {
                    client.pause(&admin);
                    paused = true;
                }
                PropOp::Unpause => {
                    client.unpause(&admin);
                    paused = false;
                }
                PropOp::ChangeAdmin => {}
            }
        }
    }

    /// Invariant: add/remove are idempotent — repeating the same mutation does
    /// not change observable state.
    #[test]
    fn prop_add_remove_idempotent(ops in op_sequence()) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(DenylistGate, ());
        let client = DenylistGateClient::new(&env, &contract_id);
        client.initialize(&admin);

        let mut addresses = Vec::new(&env);
        for _ in 0..MAX_ADDRESSES {
            addresses.push_back(Address::generate(&env));
        }

        for (raw, idx) in ops {
            let address = addresses.get(idx as u32).unwrap();
            match PropOp::from_u8(raw) {
                PropOp::Add => {
                    client.add_to_denylist(&admin, &address);
                    let after_first = client.check(&address);
                    client.add_to_denylist(&admin, &address);
                    prop_assert_eq!(client.check(&address), after_first);
                    prop_assert!(!after_first);
                }
                PropOp::Remove => {
                    client.remove_from_denylist(&admin, &address);
                    let after_first = client.check(&address);
                    client.remove_from_denylist(&admin, &address);
                    prop_assert_eq!(client.check(&address), after_first);
                    prop_assert!(after_first);
                }
                _ => {}
            }
        }
    }

    /// Invariant: admin-only paths (add/remove/pause/unpause/change_admin)
    /// reject callers that are not the current admin.
    #[test]
    fn prop_admin_only_paths(ops in op_sequence()) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(DenylistGate, ());
        let client = DenylistGateClient::new(&env, &contract_id);
        client.initialize(&admin);

        let mut addresses = Vec::new(&env);
        for _ in 0..MAX_ADDRESSES {
            addresses.push_back(Address::generate(&env));
        }
        let mut current_admin = admin.clone();

        for (raw, idx) in ops {
            let address = addresses.get(idx as u32).unwrap();
            let outsider = Address::generate(&env);
            match PropOp::from_u8(raw) {
                PropOp::Add => {
                    prop_assert!(client
                        .try_add_to_denylist(&outsider, &address)
                        .is_err());
                }
                PropOp::Remove => {
                    prop_assert!(client
                        .try_remove_from_denylist(&outsider, &address)
                        .is_err());
                }
                PropOp::Pause => {
                    prop_assert!(client.try_pause(&outsider).is_err());
                }
                PropOp::Unpause => {
                    prop_assert!(client.try_unpause(&outsider).is_err());
                }
                PropOp::ChangeAdmin => {
                    let new_admin = Address::generate(&env);
                    prop_assert!(client
                        .try_change_admin(&outsider, &new_admin)
                        .is_err());
                    // The legitimate admin can rotate.
                    client.change_admin(&current_admin, &new_admin);
                    current_admin = new_admin;
                }
                PropOp::Check => {}
            }
        }
    }
}
