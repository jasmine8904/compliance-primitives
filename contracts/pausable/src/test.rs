use crate::{
    is_paused, pause, pause_with_reason, paused_since, pause_reason, require_not_paused,
    require_paused, unpause,
};
use soroban_sdk::testutils::{Address as _, Events as _, Ledger};
use soroban_sdk::{contract, contractimpl, Address, Env, String, Symbol, TryFromVal, Val};

/// Minimal harness contract so we can exercise the pausable helpers inside a
/// real Soroban `Env`. All calls go through `env.register(Harness, ())`,
/// which gives each helper a proper contract context with instance storage.
#[contract]
struct Harness;

#[contractimpl]
impl Harness {
    pub fn is_paused(env: Env) -> bool {
        crate::is_paused(&env)
    }

    pub fn pause(env: Env) {
        crate::pause(&env);
    }

    pub fn pause_with_reason(env: Env, reason: Option<String>) {
        crate::pause_with_reason(&env, reason);
    }

    pub fn unpause(env: Env) {
        crate::unpause(&env);
    }

    pub fn pause_until(env: Env, until: u64) {
        crate::pause_until(&env, until);
    }

    pub fn pause_until_at(env: Env) -> Option<u64> {
        crate::pause_until_at(&env)
    }

    pub fn paused_since(env: Env) -> Option<u64> {
        crate::paused_since(&env)
    }

    pub fn pause_reason(env: Env) -> Option<String> {
        crate::pause_reason(&env)
    }

    pub fn pause_scope(env: Env, scope: crate::PauseScope) {
        crate::pause_scope(&env, scope);
    }

    pub fn unpause_scope(env: Env, scope: crate::PauseScope) {
        crate::unpause_scope(&env, scope);
    }

    pub fn is_paused_scope(env: Env, scope: crate::PauseScope) -> bool {
        crate::is_paused_scope(&env, scope)
    }

    pub fn check_not_paused_scope(env: Env, scope: crate::PauseScope) -> bool {
        crate::require_not_paused_scope(&env, scope);
        true
    }

    /// Calls `require_not_paused` and returns `true` if the call succeeds
    /// (i.e. the contract is not paused). Used to verify the happy path
    /// without a separate entry point.
    pub fn check_not_paused(env: Env) -> bool {
        crate::require_not_paused(&env);
        true
    }

    /// Calls `require_paused` and returns `true` if the call succeeds
    /// (i.e. the contract is currently paused).
    pub fn check_paused(env: Env) -> bool {
        crate::require_paused(&env);
        true
    }

    // ─── pauser role ────────────────────────────────────────────────────────

    pub fn set_pauser(env: Env, admin: Address, pauser: Address) {
        crate::set_pauser(&env, &admin, &pauser);
    }

    pub fn clear_pauser(env: Env, admin: Address) {
        crate::clear_pauser(&env, &admin);
    }

    pub fn get_pauser(env: Env) -> Option<Address> {
        crate::get_pauser(&env)
    }

    pub fn pause_as_pauser(env: Env, pauser: Address) {
        crate::pause_as_pauser(&env, &pauser);
    }

    pub fn unpause_as_pauser(env: Env, pauser: Address) {
        crate::unpause_as_pauser(&env, &pauser);
    }
}

// ─── is_paused default ──────────────────────────────────────────────────────

#[test]
fn test_is_paused_default_is_false() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);
    assert!(!client.is_paused());
}

// ─── pause ──────────────────────────────────────────────────────────────────

#[test]
fn test_pause_sets_flag() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    assert!(!client.is_paused());
    client.pause();
    assert!(client.is_paused());
}

#[test]
fn test_pause_is_idempotent() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause();
    client.pause(); // second call must not panic or corrupt state
    assert!(client.is_paused());
}

// ─── unpause ────────────────────────────────────────────────────────────────

#[test]
fn test_unpause_clears_flag() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause();
    assert!(client.is_paused());

    client.unpause();
    assert!(!client.is_paused());
}

#[test]
fn test_unpause_when_not_paused_is_noop() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    // Never paused — unpause must not panic
    client.unpause();
    assert!(!client.is_paused());
}

#[test]
fn test_pause_unpause_roundtrip() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause();
    assert!(client.is_paused());
    client.unpause();
    assert!(!client.is_paused());
    client.pause();
    assert!(client.is_paused());
}

// ─── require_not_paused ─────────────────────────────────────────────────────

#[test]
fn test_require_not_paused_succeeds_when_active() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    // Not paused — must not panic
    assert!(client.check_not_paused());
}

#[test]
#[should_panic]
fn test_require_not_paused_panics_when_paused() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause();
    // This call must panic because the contract is paused
    client.check_not_paused();
}

// ─── require_paused ─────────────────────────────────────────────────────────

/// `require_paused` should succeed (not panic) when the contract IS paused.
#[test]
fn test_require_paused_succeeds_when_paused() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause();
    // check_paused calls require_paused — must not panic
    assert!(client.check_paused());
}

/// `require_paused` must panic when the contract is NOT paused.
#[test]
#[should_panic]
fn test_require_paused_panics_when_not_paused() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    // Not paused — require_paused must panic
    client.check_paused();
}

/// Verify symmetry: require_not_paused and require_paused are strict inverses.
#[test]
fn test_require_paused_and_require_not_paused_are_symmetric() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    // Initially not paused — only check_not_paused must succeed.
    assert!(client.check_not_paused());

    // After pause — only check_paused must succeed.
    client.pause();
    assert!(client.check_paused());

    // After unpause — only check_not_paused must succeed again.
    client.unpause();
    assert!(client.check_not_paused());
}

// ─── paused_since ───────────────────────────────────────────────────────────

#[test]
fn test_paused_since_none_when_not_paused() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    assert!(client.paused_since().is_none());
}

#[test]
fn test_paused_since_set_on_pause() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause();
    // paused_since should be set to the ledger timestamp
    let ts = client.paused_since();
    assert!(ts.is_some());
    // The default test env timestamp is 0; just verify it is Some.
    assert_eq!(ts, Some(env.ledger().timestamp()));
}

#[test]
fn test_paused_since_cleared_on_unpause() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause();
    assert!(client.paused_since().is_some());

    client.unpause();
    assert!(client.paused_since().is_none());
}

#[test]
fn test_paused_since_preserved_on_idempotent_pause() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause();
    let ts_first = client.paused_since();

    // Second pause call must not overwrite the original timestamp
    client.pause();
    let ts_second = client.paused_since();
    assert_eq!(ts_first, ts_second);
}

// ─── pause_reason ───────────────────────────────────────────────────────────

#[test]
fn test_pause_reason_none_when_not_paused() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    assert!(client.pause_reason().is_none());
}

#[test]
fn test_pause_with_reason_stores_reason() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let reason = String::from_str(&env, "emergency maintenance");
    client.pause_with_reason(&Some(reason.clone()));

    assert!(client.is_paused());
    assert_eq!(client.pause_reason(), Some(reason));
}

#[test]
fn test_pause_reason_cleared_on_unpause() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let reason = String::from_str(&env, "security incident");
    client.pause_with_reason(&Some(reason));
    assert!(client.pause_reason().is_some());

    client.unpause();
    assert!(client.pause_reason().is_none());
}

#[test]
fn test_pause_no_reason_leaves_reason_none() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause(); // no reason supplied
    assert!(client.is_paused());
    assert!(client.pause_reason().is_none());
}

// ─── Issue #804: automatic pause expiry ────────────────────────────────────

#[test]
fn test_pause_until_expires_automatically() {
    let env = Env::default();
    env.ledger().set_timestamp(1_700_000_000);
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause_until(&1_700_000_600);
    assert!(client.is_paused());
    assert_eq!(client.pause_until_at(), Some(1_700_000_600));
    assert_eq!(client.paused_since(), Some(1_700_000_000));
    assert!(client.check_paused());

    // Half a second before expiry it is still paused; exactly at expiry it
    // resumes without an explicit unpause.
    env.ledger().set_timestamp(1_700_000_599);
    assert!(client.is_paused());
    env.ledger().set_timestamp(1_700_000_600);
    assert!(!client.is_paused());
    assert!(client.check_not_paused());
    assert!(client.paused_since().is_none());
}

#[test]
fn test_pause_until_in_the_past_is_a_noop() {
    let env = Env::default();
    env.ledger().set_timestamp(1_700_000_000);
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause_until(&1_699_999_999);
    assert!(!client.is_paused());
    assert!(client.pause_until_at().is_none());
}

#[test]
fn test_unpause_clears_expiry() {
    let env = Env::default();
    env.ledger().set_timestamp(1_700_000_000);
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    client.pause_until(&1_700_001_000);
    client.unpause();
    assert!(!client.is_paused());
    assert!(client.pause_until_at().is_none());
}

// ─── Issue #805: feature-scoped pauses ─────────────────────────────────────

#[test]
fn test_scoped_pause_isolates_other_features() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);
    use crate::PauseScope;

    assert!(!client.is_paused());
    client.pause_scope(&PauseScope::Mint);
    assert!(!client.is_paused());
    assert!(client.is_paused_scope(&PauseScope::Mint));
    assert!(!client.is_paused_scope(&PauseScope::Redeem));
    assert!(client.check_not_paused_scope(&PauseScope::Redeem));
}

#[test]
fn test_global_pause_overrides_every_scope() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);
    use crate::PauseScope;

    client.pause();
    for scope in [
        PauseScope::Mint,
        PauseScope::Redeem,
        PauseScope::Transfer,
        PauseScope::Admin,
    ] {
        assert!(client.is_paused_scope(&scope));
    }
}

#[test]
fn test_unpause_scope_and_global_unpause() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);
    use crate::PauseScope;

    client.pause_scope(&PauseScope::Mint);
    client.pause_scope(&PauseScope::Redeem);
    client.unpause_scope(&PauseScope::Mint);
    assert!(!client.is_paused_scope(&PauseScope::Mint));
    assert!(client.is_paused_scope(&PauseScope::Redeem));

    client.unpause();
    assert!(!client.is_paused_scope(&PauseScope::Redeem));
}

/// Returns `true` if the event log contains an event whose first topic is
/// `name` and whose data equals `data`.
fn has_event(env: &Env, name: &str, data: &Val) -> bool {
    let target = Symbol::new(env, name);
    env.events().all().iter().any(|(_, topics, value)| {
        let topic_matches = topics
            .get(0)
            .map(|t| Symbol::try_from_val(env, &t) == Ok(target.clone()))
            .unwrap_or(false);
        topic_matches && value == *data
    })
}

#[test]
fn test_get_pauser_default_is_none() {
    let env = Env::default();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    assert!(client.get_pauser().is_none());
}

#[test]
fn test_admin_can_set_pauser() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let admin = Address::generate(&env);
    let pauser = Address::generate(&env);

    client.set_pauser(&admin, &pauser);
    assert_eq!(client.get_pauser(), Some(pauser));
}

#[test]
fn test_pauser_can_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let admin = Address::generate(&env);
    let pauser = Address::generate(&env);
    client.set_pauser(&admin, &pauser);

    assert!(!client.is_paused());
    client.pause_as_pauser(&pauser);
    assert!(client.is_paused());
}

#[test]
#[should_panic]
fn test_non_pauser_cannot_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let admin = Address::generate(&env);
    let pauser = Address::generate(&env);
    client.set_pauser(&admin, &pauser);

    let stranger = Address::generate(&env);
    client.pause_as_pauser(&stranger);
}

#[test]
#[should_panic]
fn test_pauser_cannot_unpause() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let admin = Address::generate(&env);
    let pauser = Address::generate(&env);
    client.set_pauser(&admin, &pauser);

    client.pause_as_pauser(&pauser);
    assert!(client.is_paused());

    // Pauser must not be able to unpause.
    client.unpause_as_pauser(&pauser);
}

#[test]
#[should_panic]
fn test_non_admin_cannot_set_pauser() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let admin = Address::generate(&env);
    let pauser = Address::generate(&env);
    client.set_pauser(&admin, &pauser);

    // The pauser is not the admin and must not be able to rotate itself.
    let other = Address::generate(&env);
    client.set_pauser(&pauser, &other);
}

#[test]
#[should_panic]
fn test_non_admin_cannot_clear_pauser() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let admin = Address::generate(&env);
    let pauser = Address::generate(&env);
    client.set_pauser(&admin, &pauser);

    client.clear_pauser(&pauser);
}

#[test]
fn test_admin_can_clear_pauser() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let admin = Address::generate(&env);
    let pauser = Address::generate(&env);
    client.set_pauser(&admin, &pauser);
    assert_eq!(client.get_pauser(), Some(pauser));

    client.clear_pauser(&admin);
    assert!(client.get_pauser().is_none());
}

#[test]
#[should_panic]
fn test_cleared_pauser_loses_pause_authority() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let admin = Address::generate(&env);
    let pauser = Address::generate(&env);
    client.set_pauser(&admin, &pauser);
    client.clear_pauser(&admin);

    // The previously authorized pauser must no longer be able to pause.
    client.pause_as_pauser(&pauser);
}

#[test]
fn test_pauser_change_events_emitted() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(Harness, ());
    let client = HarnessClient::new(&env, &id);

    let admin = Address::generate(&env);
    let pauser = Address::generate(&env);

    client.set_pauser(&admin, &pauser);
    let set_data = Val::try_from_val(&env, &Some(pauser.clone())).unwrap();
    assert!(has_event(&env, "pauser_set", &set_data));

    client.clear_pauser(&admin);
    let cleared_data = Val::try_from_val(&env, &Option::<Address>::None).unwrap();
    assert!(has_event(&env, "pauser_cleared", &cleared_data));
}
