//! TTL and event coverage for jurisdiction-flag.
use jurisdiction_flag::{JurisdictionFlag, JurisdictionFlagClient, JurisdictionSet};
use soroban_sdk::testutils::{storage::Persistent as _, Address as _, Events as _, Ledger as _};
use soroban_sdk::{vec, Address, Env, IntoVal, Symbol, Val, Event, String};

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
fn every_write_extends_ttl_and_reports_previous_codes() {
    const EXTEND_TO: u32 = 1_555_200;
    let env = env();
    let issuer = Address::generate(&env);
    let id = env.register(JurisdictionFlag, ());
    let client = JurisdictionFlagClient::new(&env, &id);
    client.initialize(&issuer);
    let address = Address::generate(&env);
    let s = |c: &str| String::from_str(&env, c);
    let check = |code: &str, previous: &[&str]| {
        let mut prev = vec![&env];
        for c in previous {
            prev.push_back(s(c));
        }
        let expected = JurisdictionSet { address: address.clone(), code: s(code), previous: prev };
        assert_eq!(
            env.events().all().filter_by_contract(&id),
            [expected.to_xdr(&env, &id)]
        );
        for key in ["Jurisdictions", "Jurisdiction"] {
            let key = (Symbol::new(&env, key), address.clone());
            assert_eq!(ttl(&env, &id, key), EXTEND_TO);
        }
        // Drop below the extension threshold so the next write must extend.
        env.ledger().with_mut(|l| l.sequence_number += EXTEND_TO - 1_000);
    };

    client.set_jurisdiction(&issuer, &address, &s("US"));
    check("US", &[]);
    client.add_jurisdiction(&issuer, &address, &s("GB"));
    check("GB", &["US"]);
    client.set_multiple_jurisdictions(&issuer, &vec![&env, (address.clone(), s("FR"))]);
    check("FR", &["US", "GB"]);
    let until = env.ledger().sequence() + 10;
    client.set_jurisdiction_until(&issuer, &address, &s("DE"), &until);
    let key = (Symbol::new(&env, "ValidUntil"), address.clone());
    // events() only holds the last invocation, so check it before the TTL read.
    check("DE", &["FR"]);
    assert_eq!(ttl(&env, &id, key), 1_000);
    client.set_jurisdiction(&issuer, &address, &s("US"));
    check("US", &["DE"]);
}
