use super::*;
use circuit_breaker::{CircuitBreaker, CircuitBreakerClient};
use compliance_aggregator::{ComplianceAggregator, ComplianceAggregatorClient};
use denylist_gate::{DenylistGate, DenylistGateClient};
use jurisdiction_flag::{JurisdictionFlag, JurisdictionFlagClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{vec, Env, String};

struct Fixture<'a> {
    gate_admin: Address,
    issuer: Address,
    breaker_admin: Address,
    agg_admin: Address,
    gate_id: Address,
    jurisdiction_id: Address,
    breaker_id: Address,
    aggregator: ComplianceAggregatorClient<'a>,
    token: RwaAggregatorTokenClient<'a>,
}

/// Deploys denylist-gate, jurisdiction-flag and circuit-breaker, registers all
/// three with a compliance-aggregator, and wires the token to the aggregator
/// only.
fn setup(env: &Env) -> Fixture<'_> {
    env.mock_all_auths();
    let gate_admin = Address::generate(env);
    let issuer = Address::generate(env);
    let breaker_admin = Address::generate(env);
    let agg_admin = Address::generate(env);

    let gate_id = env.register(DenylistGate, ());
    DenylistGateClient::new(env, &gate_id).initialize(&gate_admin);

    let jurisdiction_id = env.register(JurisdictionFlag, ());
    JurisdictionFlagClient::new(env, &jurisdiction_id).initialize(&issuer);

    let breaker_id = env.register(CircuitBreaker, ());
    CircuitBreakerClient::new(env, &breaker_id).initialize(&breaker_admin);

    let aggregator_id = env.register(ComplianceAggregator, ());
    let aggregator = ComplianceAggregatorClient::new(env, &aggregator_id);
    aggregator.initialize(
        &agg_admin,
        &Some(gate_id.clone()),
        &Some(jurisdiction_id.clone()),
        &Some(breaker_id.clone()),
    );

    let allowed_codes = vec![env, String::from_str(env, "US"), String::from_str(env, "CA")];
    let token_id = env.register(RwaAggregatorToken, ());
    let token = RwaAggregatorTokenClient::new(env, &token_id);
    token.initialize(&aggregator_id, &allowed_codes);

    Fixture { gate_admin, issuer, breaker_admin, agg_admin, gate_id, jurisdiction_id, breaker_id, aggregator, token }
}

fn onboard(env: &Env, fx: &Fixture<'_>, who: &Address, code: &str) {
    JurisdictionFlagClient::new(env, &fx.jurisdiction_id).set_jurisdiction(
        &fx.issuer,
        who,
        &String::from_str(env, code),
    );
}

fn two_onboarded_parties(env: &Env, fx: &Fixture<'_>) -> (Address, Address) {
    let alice = Address::generate(env);
    let bob = Address::generate(env);
    onboard(env, fx, &alice, "US");
    onboard(env, fx, &bob, "CA");
    fx.token.mint(&alice, &1_000);
    (alice, bob)
}

#[test]
fn test_token_is_wired_to_aggregator_only() {
    let env = Env::default();
    let fx = setup(&env);
    assert_eq!(fx.token.aggregator(), fx.aggregator.address);
}

#[test]
fn test_transfer_succeeds_when_all_checks_pass() {
    let env = Env::default();
    let fx = setup(&env);
    let (alice, bob) = two_onboarded_parties(&env, &fx);

    fx.token.transfer(&alice, &bob, &400);

    assert_eq!(fx.token.balance(&alice), 600);
    assert_eq!(fx.token.balance(&bob), 400);
}

#[test]
fn test_transfer_blocked_when_sender_denylisted() {
    let env = Env::default();
    let fx = setup(&env);
    let (alice, bob) = two_onboarded_parties(&env, &fx);
    DenylistGateClient::new(&env, &fx.gate_id).add_to_denylist(&fx.gate_admin, &alice);

    let result = fx.token.try_transfer(&alice, &bob, &400);
    assert_eq!(result, Err(Ok(Error::DeniedByGate)));
    assert_eq!(fx.token.balance(&alice), 1_000);
}

#[test]
fn test_transfer_blocked_when_recipient_denylisted() {
    let env = Env::default();
    let fx = setup(&env);
    let (alice, bob) = two_onboarded_parties(&env, &fx);
    DenylistGateClient::new(&env, &fx.gate_id).add_to_denylist(&fx.gate_admin, &bob);

    let result = fx.token.try_transfer(&alice, &bob, &400);
    assert_eq!(result, Err(Ok(Error::DeniedByGate)));
    assert_eq!(fx.token.balance(&bob), 0);
}

#[test]
fn test_transfer_blocked_when_jurisdiction_not_permitted() {
    let env = Env::default();
    let fx = setup(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    onboard(&env, &fx, &alice, "US");
    // IR is outside the token's allowed codes (US, CA).
    onboard(&env, &fx, &bob, "IR");
    fx.token.mint(&alice, &1_000);

    let result = fx.token.try_transfer(&alice, &bob, &400);
    assert_eq!(result, Err(Ok(Error::JurisdictionNotPermitted)));
    assert_eq!(fx.token.balance(&alice), 1_000);
}

#[test]
fn test_transfer_blocked_when_jurisdiction_unset() {
    let env = Env::default();
    let fx = setup(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env); // never flagged
    onboard(&env, &fx, &alice, "US");
    fx.token.mint(&alice, &1_000);

    let result = fx.token.try_transfer(&alice, &bob, &400);
    assert_eq!(result, Err(Ok(Error::JurisdictionNotPermitted)));
}

#[test]
fn test_frozen_circuit_breaker_halts_transfers_through_aggregator() {
    let env = Env::default();
    let fx = setup(&env);
    let (alice, bob) = two_onboarded_parties(&env, &fx);

    CircuitBreakerClient::new(&env, &fx.breaker_id).freeze(&fx.breaker_admin, &None);

    let result = fx.token.try_transfer(&alice, &bob, &400);
    assert_eq!(result, Err(Ok(Error::CircuitBreakerFrozen)));
    assert_eq!(fx.token.balance(&alice), 1_000);
    assert_eq!(fx.token.balance(&bob), 0);

    // Unfreezing restores transfers with no change to the token.
    CircuitBreakerClient::new(&env, &fx.breaker_id).unfreeze(&fx.breaker_admin);
    fx.token.transfer(&alice, &bob, &400);
    assert_eq!(fx.token.balance(&bob), 400);
}

#[test]
fn test_breaker_takes_priority_over_denylist() {
    let env = Env::default();
    let fx = setup(&env);
    let (alice, bob) = two_onboarded_parties(&env, &fx);
    DenylistGateClient::new(&env, &fx.gate_id).add_to_denylist(&fx.gate_admin, &alice);
    CircuitBreakerClient::new(&env, &fx.breaker_id).freeze(&fx.breaker_admin, &None);

    let result = fx.token.try_transfer(&alice, &bob, &400);
    assert_eq!(result, Err(Ok(Error::CircuitBreakerFrozen)));
}

#[test]
fn test_swapping_primitive_in_aggregator_needs_no_token_change() {
    let env = Env::default();
    let fx = setup(&env);
    let (alice, bob) = two_onboarded_parties(&env, &fx);
    DenylistGateClient::new(&env, &fx.gate_id).add_to_denylist(&fx.gate_admin, &alice);
    assert_eq!(fx.token.try_transfer(&alice, &bob, &400), Err(Ok(Error::DeniedByGate)));

    // Point the aggregator at a fresh, empty denylist-gate.
    let new_gate_id = env.register(DenylistGate, ());
    DenylistGateClient::new(&env, &new_gate_id).initialize(&fx.gate_admin);
    fx.aggregator.set_denylist_gate(&fx.agg_admin, &new_gate_id);

    fx.token.transfer(&alice, &bob, &400);
    assert_eq!(fx.token.balance(&bob), 400);
}

#[test]
fn test_transfer_fails_closed_when_aggregator_has_no_checks() {
    let env = Env::default();
    env.mock_all_auths();
    let agg_admin = Address::generate(&env);
    let aggregator_id = env.register(ComplianceAggregator, ());
    ComplianceAggregatorClient::new(&env, &aggregator_id).initialize(&agg_admin, &None, &None, &None);

    let token_id = env.register(RwaAggregatorToken, ());
    let token = RwaAggregatorTokenClient::new(&env, &token_id);
    token.initialize(&aggregator_id, &vec![&env, String::from_str(&env, "US")]);

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    token.mint(&alice, &1_000);

    let result = token.try_transfer(&alice, &bob, &400);
    assert_eq!(result, Err(Ok(Error::ComplianceUnavailable)));
    assert_eq!(token.balance(&alice), 1_000);
}

#[test]
fn test_transfer_rejects_insufficient_balance() {
    let env = Env::default();
    let fx = setup(&env);
    let (alice, bob) = two_onboarded_parties(&env, &fx);

    let result = fx.token.try_transfer(&alice, &bob, &5_000);
    assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
}

#[test]
fn test_transfer_rejects_non_positive_amount() {
    let env = Env::default();
    let fx = setup(&env);
    let (alice, bob) = two_onboarded_parties(&env, &fx);

    assert_eq!(fx.token.try_transfer(&alice, &bob, &0), Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_double_initialize_fails() {
    let env = Env::default();
    let fx = setup(&env);
    let result = fx.token.try_initialize(&fx.aggregator.address, &vec![&env, String::from_str(&env, "US")]);
    assert_eq!(result, Err(Ok(Error::AlreadyInitialized)));
}
