//! TTL coverage for audit-log.
use audit_log::{AuditLog, AuditLogClient, ENTRY_TTL_EXTEND_TO};
use soroban_sdk::testutils::{storage::Persistent as _, Address as _, Ledger as _};
use soroban_sdk::{Address, Env, IntoVal, Symbol, Val, String};

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
fn record_extends_entry_ttl() {
    let env = env();
    let admin = Address::generate(&env);
    let id = env.register(AuditLog, ());
    let client = AuditLogClient::new(&env, &id);
    client.initialize(&admin);
    let kind = Symbol::new(&env, "deny_add");
    for index in 0..2u64 {
        client.record(&admin, &kind, &admin, &String::from_str(&env, "x"));
        assert_eq!(ttl(&env, &id, (Symbol::new(&env, "Entry"), index)), ENTRY_TTL_EXTEND_TO);
    }
}
