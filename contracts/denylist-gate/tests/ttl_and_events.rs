//! TTL and event-actor coverage for denylist-gate.
use denylist_gate::{DenyAdd, DenyRemove, DenylistGate, DenylistGateClient, SignerAction, APPROVAL_TTL_LEDGERS};
use soroban_sdk::testutils::{storage::Persistent as _, Address as _, Events as _, Ledger as _};
use soroban_sdk::{vec, Address, Env, IntoVal, Symbol, Val, Event};

fn env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|l| {
        l.sequence_number = 100;
        l.min_persistent_entry_ttl = 500;
        l.max_entry_ttl = 6_312_000;
    });
    env
}

// Storage keys are private enums; `#[contracttype]` encodes a variant as a
// `(Symbol, fields...)` tuple, so tests can address entries directly.
fn ttl<K: IntoVal<Env, Val>>(env: &Env, id: &Address, key: K) -> u32 {
    env.as_contract(id, || env.storage().persistent().get_ttl(&key))
}

#[test]
fn events_name_the_acting_admin_or_officer() {
    let env = env();
    let admin = Address::generate(&env);
    let officer = Address::generate(&env);
    let id = env.register(DenylistGate, ());
    let client = DenylistGateClient::new(&env, &id);
    client.initialize(&admin);
    client.set_compliance_officer(&admin, &officer);
    let address = Address::generate(&env);
    for by in [admin, officer] {
        client.add_to_denylist(&by, &address);
        assert_eq!(
            env.events().all().filter_by_contract(&id),
            [DenyAdd { address: address.clone(), by: by.clone() }.to_xdr(&env, &id)]
        );
        client.remove_from_denylist(&by, &address);
        assert_eq!(
            env.events().all().filter_by_contract(&id),
            [DenyRemove { address: address.clone(), by: by.clone() }.to_xdr(&env, &id)]
        );
        client.remove_multiple_from_denylist(&by, &vec![&env, address.clone()]);
        assert_eq!(
            env.events().all().filter_by_contract(&id),
            [DenyRemove { address: address.clone(), by: by.clone() }.to_xdr(&env, &id)]
        );
    }
}

#[test]
fn every_persistent_write_extends_ttl() {
    let env = env();
    let admin = Address::generate(&env);
    let id = env.register(DenylistGate, ());
    let client = DenylistGateClient::new(&env, &id);
    client.initialize(&admin);
    let address = Address::generate(&env);
    client.add_to_denylist(&admin, &address);
    let key = (Symbol::new(&env, "Denied"), address.clone());
    assert_eq!(ttl(&env, &id, key), 6_311_520);

    let other = Address::generate(&env);
    client.initialize_multisig(&admin, &vec![&env, admin.clone(), other.clone()], &2);
    client.add_signer(&admin, &address);
    let action = SignerAction::Add(address);
    let key = (Symbol::new(&env, "PendingSignerAction"), action);
    assert_eq!(ttl(&env, &id, key), APPROVAL_TTL_LEDGERS * 2);
}
