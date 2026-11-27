//! # Tokenized Fund Example
//!
//! Reference tokenized fund share that composes three compliance primitives in
//! one `transfer` path: allowlist, jurisdiction, and circuit breaker.
//!
//! Check order in `transfer` (fail-fast, specific error per gate):
//! 1. **Circuit breaker** — `circuit-breaker.is_frozen()` halts all transfers
//! 2. **Allowlist** — both `from` and `to` must pass `allowlist-token.is_allowed`
//! 3. **Jurisdiction** — both parties must pass
//!    `jurisdiction-flag.is_permitted_jurisdiction` against `allowed_codes`
//!
//! Like `/examples/rwa-token`, this crate does not link the primitive crates'
//! `#[contractimpl]` exports; it calls them through `#[contractclient]` traits
//! that describe only the call shape.
//!
//! shortcut: `mint` is issuer-gated demo issuance with no supply cap, subscription
//! or NAV accounting; upgrade to a real issuance flow before production use.
#![no_std]

use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, Address, Env, String, Vec,
};

#[contractclient(name = "AllowlistClient")]
pub trait AllowlistInterface {
    fn is_allowed(env: Env, address: Address) -> bool;
}

#[contractclient(name = "JurisdictionClient")]
pub trait JurisdictionFlagInterface {
    fn is_permitted_jurisdiction(env: Env, address: Address, allowed_codes: Vec<String>) -> bool;
}

#[contractclient(name = "CircuitBreakerClient")]
pub trait CircuitBreakerInterface {
    fn is_frozen(env: Env) -> bool;
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Issuer,
    Allowlist,
    Jurisdiction,
    AllowedCodes,
    CircuitBreaker,
    Balance(Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    InsufficientBalance = 3,
    CircuitBreakerFrozen = 4,
    NotAllowlisted = 5,
    JurisdictionNotPermitted = 6,
}

#[contract]
pub struct TokenizedFund;

#[contractimpl]
impl TokenizedFund {
    /// Wire this fund token to deployed primitive instances, the issuer that
    /// may mint shares, and the jurisdiction codes that may hold/transfer them.
    pub fn initialize(
        env: Env,
        issuer: Address,
        allowlist: Address,
        jurisdiction: Address,
        circuit_breaker: Address,
        allowed_codes: Vec<String>,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Issuer) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Issuer, &issuer);
        env.storage().instance().set(&DataKey::Allowlist, &allowlist);
        env.storage().instance().set(&DataKey::Jurisdiction, &jurisdiction);
        env.storage().instance().set(&DataKey::CircuitBreaker, &circuit_breaker);
        env.storage().instance().set(&DataKey::AllowedCodes, &allowed_codes);
        Ok(())
    }

    /// Issue `amount` shares to `to`. Requires the issuer's authorization.
    pub fn mint(env: Env, to: Address, amount: i128) -> Result<(), Error> {
        let issuer: Address = env.storage().instance().get(&DataKey::Issuer).ok_or(Error::NotInitialized)?;
        issuer.require_auth();

        let balance = Self::balance(env.clone(), to.clone());
        env.storage().persistent().set(&DataKey::Balance(to), &(balance + amount));
        Ok(())
    }

    pub fn balance(env: Env, address: Address) -> i128 {
        env.storage().persistent().get(&DataKey::Balance(address)).unwrap_or(0)
    }

    /// Transfer `amount` shares from `from` to `to` after all compliance checks
    /// clear. Errors identify which gate failed.
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) -> Result<(), Error> {
        from.require_auth();

        let circuit_breaker_addr: Address =
            env.storage().instance().get(&DataKey::CircuitBreaker).ok_or(Error::NotInitialized)?;
        if CircuitBreakerClient::new(&env, &circuit_breaker_addr).is_frozen() {
            return Err(Error::CircuitBreakerFrozen);
        }

        let allowlist_addr: Address =
            env.storage().instance().get(&DataKey::Allowlist).ok_or(Error::NotInitialized)?;
        let allowlist = AllowlistClient::new(&env, &allowlist_addr);
        if !allowlist.is_allowed(&from) || !allowlist.is_allowed(&to) {
            return Err(Error::NotAllowlisted);
        }

        let jurisdiction_addr: Address =
            env.storage().instance().get(&DataKey::Jurisdiction).ok_or(Error::NotInitialized)?;
        let allowed_codes: Vec<String> =
            env.storage().instance().get(&DataKey::AllowedCodes).ok_or(Error::NotInitialized)?;
        let jurisdiction = JurisdictionClient::new(&env, &jurisdiction_addr);
        let from_permitted =
            matches!(jurisdiction.try_is_permitted_jurisdiction(&from, &allowed_codes), Ok(Ok(true)));
        let to_permitted =
            matches!(jurisdiction.try_is_permitted_jurisdiction(&to, &allowed_codes), Ok(Ok(true)));
        if !from_permitted || !to_permitted {
            return Err(Error::JurisdictionNotPermitted);
        }

        let from_balance = Self::balance(env.clone(), from.clone());
        if from_balance < amount {
            return Err(Error::InsufficientBalance);
        }
        let to_balance = Self::balance(env.clone(), to.clone());

        env.storage().persistent().set(&DataKey::Balance(from), &(from_balance - amount));
        env.storage().persistent().set(&DataKey::Balance(to), &(to_balance + amount));
        Ok(())
    }
}
