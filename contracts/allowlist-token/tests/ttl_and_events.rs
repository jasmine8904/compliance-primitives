//! TTL and event-actor coverage for allowlist-token.
use allowlist_token::{AllowAdd, AllowRemove, AllowlistToken, AllowlistTokenClient};
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
    let id = env.register(AllowlistToken, ());
    let client = AllowlistTokenClient::new(&env, &id);
    client.initialize(&admin, &Address::generate(&env));
    client.set_compliance_officer(&admin, &officer);
    let address = Address::generate(&env);
    for by in [admin.clone(), officer] {
        // Adds are admin-only; officers may remove.
        client.add_to_allowlist(&admin, &address, &None);
        assert_eq!(
            env.events().all().filter_by_contract(&id),
            [AllowAdd { address: address.clone(), by: admin.clone() }.to_xdr(&env, &id)]
        );
        client.remove_from_allowlist(&by, &address);
        assert_eq!(
            env.events().all().filter_by_contract(&id),
            [AllowRemove { address: address.clone(), by: by.clone() }.to_xdr(&env, &id)]
        );
        client.remove_multiple_from_allowlist(&by, &vec![&env, address.clone()]);
        assert_eq!(
            env.events().all().filter_by_contract(&id),
            [AllowRemove { address: address.clone(), by: by.clone() }.to_xdr(&env, &id)]
        );
    }
}

#[test]
fn every_persistent_write_extends_ttl() {
    const EXTEND_TO: u32 = 1_555_200;
    let env = env();
    let admin = Address::generate(&env);
    let id = env.register(AllowlistToken, ());
    let client = AllowlistTokenClient::new(&env, &id);
    client.initialize(&admin, &Address::generate(&env));
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    client.add_to_allowlist(&admin, &alice, &None);
    client.add_multiple_to_allowlist(&admin, &vec![&env, bob.clone()], &None);
    client.freeze(&admin, &alice);
    for (name, who) in [("Allowed", &alice), ("Allowed", &bob), ("Frozen", &alice)] {
        assert_eq!(ttl(&env, &id, (Symbol::new(&env, name), who.clone())), EXTEND_TO);
    }
}
