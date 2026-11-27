#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

use circuit_breaker::{CircuitBreakerContract, CircuitBreakerContractClient};

#[derive(Arbitrary, Debug)]
struct GuardianSequence {
    ops: Vec<u8>,
}

const OP_ADD: u8 = 0;
const OP_REMOVE: u8 = 1;
const OP_FREEZE: u8 = 2;
const OP_UNFREEZE: u8 = 3;

const POOL_SIZE: usize = 6;

fn setup() -> (Env, CircuitBreakerContractClient<'static>, [Address; POOL_SIZE]) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, CircuitBreakerContract);
    let client = CircuitBreakerContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    let mut pool: [Address; POOL_SIZE] = [
        admin.clone(),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
        Address::generate(&env),
    ];

    // Seed a couple of guardians so freeze paths are exercised.
    client.add_guardian(&admin, &pool[1]);
    client.add_guardian(&admin, &pool[2]);

    (env, client, pool)
}

fn is_guardian(client: &CircuitBreakerContractClient, who: &Address) -> bool {
    client.is_guardian(who)
}

fn check_no_duplicates(client: &CircuitBreakerContractClient, pool: &[Address; POOL_SIZE]) {
    // The guardian list must not contain duplicates. We verify by counting how
    // many distinct pool entries report as guardians and comparing against the
    // reported guardian list length.
    let mut distinct: u32 = 0;
    let mut seen: Vec<Address> = Vec::new(client.env());
    for addr in pool.iter() {
        if is_guardian(client, addr) {
            let mut already = false;
            for s in seen.iter() {
                if s == *addr {
                    already = true;
                    break;
                }
            }
            if !already {
                seen.push_back(addr.clone());
                distinct += 1;
            }
        }
    }
    let listed = client.get_guardians().len();
    assert_eq!(
        listed, distinct,
        "guardian list contains duplicates: listed={} distinct={}",
        listed, distinct
    );
}

fuzz_target!(|input: GuardianSequence| {
    let (_env, client, pool) = setup();

    let admin = pool[0].clone();

    let mut i = 0usize;
    while i + 1 < input.ops.len() {
        let op = input.ops[i];
        let caller_idx = (input.ops[i + 1] as usize) % POOL_SIZE;
        let caller = pool[caller_idx].clone();
        i += 2;

        match op % 4 {
            OP_ADD => {
                let target_idx = (input.ops.get(i as u32).copied().unwrap_or(0) as usize)
                    % POOL_SIZE;
                let target = pool[target_idx].clone();
                // Only admin may add guardians.
                let res = client.try_add_guardian(&caller, &target);
                if caller == admin {
                    assert!(
                        res.is_ok(),
                        "admin add_guardian should succeed"
                    );
                } else {
                    assert!(
                        res.is_err(),
                        "non-admin add_guardian should fail"
                    );
                }
            }
            OP_REMOVE => {
                let target_idx = (input.ops.get(i as u32).copied().unwrap_or(0) as usize)
                    % POOL_SIZE;
                let target = pool[target_idx].clone();
                let res = client.try_remove_guardian(&caller, &target);
                if caller == admin {
                    // Removing a non-guardian may or may not be an error
                    // depending on contract semantics; only assert it does not
                    // panic and that invariants hold afterwards.
                    let _ = res;
                } else {
                    assert!(
                        res.is_err(),
                        "non-admin remove_guardian should fail"
                    );
                }
            }
            OP_FREEZE => {
                let was_guardian = is_guardian(&client, &caller);
                let res = client.try_freeze(&caller);
                if caller == admin || was_guardian {
                    assert!(
                        res.is_ok(),
                        "admin/guardian freeze should succeed"
                    );
                } else {
                    assert!(
                        res.is_err(),
                        "non-guardian freeze should be NotAuthorized"
                    );
                }
            }
            OP_UNFREEZE => {
                let was_guardian = is_guardian(&client, &caller);
                let res = client.try_unfreeze(&caller);
                if caller == admin || was_guardian {
                    assert!(
                        res.is_ok(),
                        "admin/guardian unfreeze should succeed"
                    );
                } else {
                    assert!(
                        res.is_err(),
                        "non-guardian unfreeze should be NotAuthorized"
                    );
                }
            }
            _ => unreachable!(),
        }

        check_no_duplicates(&client, &pool);
    }
});