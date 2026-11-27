# Shared Pausable Capability — Design Document

> Closes #106. Coordinates with #84 (denylist-gate pausability) and #85
> (jurisdiction-flag pausability) by implementing both as part of a single
> shared helper rather than independently.

## Problem

Three contracts in this workspace — `denylist-gate`, `jurisdiction-flag`, and
`allowlist-token` — all need an admin-controlled pause switch that:

- blocks state-mutating operations while active,
- leaves read-only operations unaffected,
- is toggleable only by the contract's admin/issuer,
- emits auditable events on state change.

Implementing this three times independently creates identical logic with three
independent audit surfaces. Issues #84 and #85 each proposed adding pause to a
single contract; this issue (#106) asks for a shared implementation that all
three can adopt.

## Where the shared code lives

A new workspace crate: **`contracts/pausable`** (`compliance-pausable`).

It is a plain `#![no_std]` Rust library. It has:

- No `#[contract]` attribute.
- No `#[contractimpl]` block.
- No wasm function exports of any kind.

It exports four free functions:

```rust
pub fn is_paused(env: &Env) -> bool
pub fn pause(env: &Env)
pub fn unpause(env: &Env)
pub fn require_not_paused<E: Copy>(env: &Env, paused_err: E) -> Result<(), E>
```

Each function reads/writes a single `bool` stored under the symbol key
`"Paused"` in the calling contract's *instance* storage. Instance storage was
chosen because:

1. The pause flag is contract-global (not per-address), matching the semantics
   of the `Admin`/`Issuer` key already stored there.
2. It shares TTL management with the admin key — no separate TTL extension is
   needed.
3. It is cheap: one `get`/`set`/`remove` on a slot the contract already holds
   open.

## Why this doesn't hit the wasm-export-collision issue

The [`examples/denylist-gate-consumer`](../examples/denylist-gate-consumer/src/lib.rs)
notes this constraint:

> "this deliberately does NOT depend on the `denylist-gate` crate directly:
> linking another contract's crate pulls its `#[contractimpl]` wasm exports
> into this binary too and the two export sets collide at link time."

That constraint applies **only** to crates that carry a `#[contract]` /
`#[contractimpl]` annotation. Those macros emit `#[no_mangle] pub extern "C"`
symbols (the wasm host functions). If two such crates are compiled into the
same binary, the linker sees duplicate symbol definitions and errors out.

`compliance-pausable` has no `#[contract]`, no `#[contractimpl]`, and
therefore emits no wasm exports. It is ordinary Rust code; the Rust compiler
inlines it into each contract binary at compile time, contributing zero
additional exported symbols. There is no collision risk.

## How each contract plugs in

Each contract:

1. Adds `compliance-pausable = { workspace = true }` to its `Cargo.toml`.
2. Adds `ContractPaused = 4` to its own `Error` enum (no shared error type
   crossing contract boundaries — each contract owns its error set).
3. Adds `Paused` and `Unpaused` `#[contractevent]` structs (contract-local,
   so event schemas stay self-contained in each binary).
4. Adds `pause`, `unpause`, and `is_paused` methods to its `#[contractimpl]`
   block, each gated by the contract's existing `require_admin` /
   `require_issuer` helper.
5. Calls `compliance_pausable::require_not_paused(&env, Error::ContractPaused)?`
   at the top of every state-mutating method.

Read-only methods (`check`, `get_jurisdiction`, `is_permitted_jurisdiction`,
`is_allowed`) are deliberately **not** gated — compliance queries must always
be answerable regardless of operational status.

### Per-contract gating summary

| Contract           | Paused blocks                                                   | Paused does NOT block                                    |
|--------------------|-----------------------------------------------------------------|----------------------------------------------------------|
| `denylist-gate`    | `add_to_denylist`, `remove_from_denylist`                       | `check`                                                  |
| `jurisdiction-flag`| `set_jurisdiction`                                              | `get_jurisdiction`, `is_permitted_jurisdiction`          |
| `allowlist-token`  | `add_to_allowlist`, `remove_from_allowlist`, `transfer`         | `is_allowed`                                             |

For `allowlist-token`, `transfer` is additionally gated because a paused token
wrapper should not be forwarding funds. Note that a paused `transfer` returns
`Err(ContractPaused)` (which rolls back all state including events), not
`Ok(false)` — `Ok(false)` is reserved for the "address not allowlisted" path
where the audit `Blocked` event must survive.

## Auth model

Unpausing is gated by the same `require_admin` / `require_issuer` helper that
guards all other mutating operations. Pausing is additionally allowed for an
optional, separately-configured **pauser** address (see below), so that
incident response does not require the (typically cold) admin key.

### Pauser role

`compliance-pausable` exports helpers for an optional pauser address:

```rust
pub fn get_pauser(env: &Env) -> Option<Address>
pub fn set_pauser(env: &Env, pauser: &Address)
pub fn clear_pauser(env: &Env)
pub fn is_pauser(env: &Env, who: &Address) -> bool
```

- **Storage key**: the pauser address is stored under the fixed
  `symbol_short!("Pauser")` key in the calling contract's *instance* storage,
  alongside the `"Paused"` flag. It is absent by default; `get_pauser` returns
  `None` until a contract sets one.
- **Authorization rules**:
  - `pause` is allowed for the admin/issuer **or** the configured pauser.
  - `unpause` is **admin/issuer only**. The pauser can halt the contract but
    cannot resume it — resuming is a deliberate, reviewed action.
  - Setting or clearing the pauser is **admin/issuer only**.
- **Events**: each contract keeps its own `#[contractevent]` structs. Contracts
  that support the pauser role emit a distinct event (e.g. `PausedByPauser`)
  when the pause is performed by the pauser rather than the admin, so audit
  trails distinguish the two paths. `Unpaused` is always emitted with the
  admin/issuer as the topic. `PauserChanged` is emitted on
  `set_pauser`/`clear_pauser`.

### Recommended key setup

For incident response, separate the keys:

- **Admin / issuer (cold key)**: held offline or in a multisig. Used for
  unpausing, pauser rotation, and all other privileged mutations.
- **Pauser (hot key)**: held by the on-call responder. It can only pause, so
  compromise of the hot key cannot resume a halted contract or mutate list
  membership. Rotate it after any suspected exposure via `set_pauser`.

If a contract does not configure a pauser, the model degrades to the original
admin-only behaviour: only the admin/issuer can pause or unpause.
=======

## `#![no_std]` and wasm compatibility

`compliance-pausable` is `#![no_std]`. All four helpers operate purely through
the `soroban_sdk::Env` reference they receive — no heap allocation, no
`std::error::Error`, no trait objects. The `require_not_paused` function is
generic over the error type `E: Copy` rather than over a trait, which works
cleanly in `no_std` without any `std::error::Error` bound.

## What was considered but rejected

**A `Pausable` Rust trait** (as the issue title suggests): a trait with
`fn pause(&self, env: &Env)` etc. was explored. In `#![no_std]` Soroban
contracts, implementing a trait on a `#[contract]` struct and then calling
into it from the `#[contractimpl]` block is syntactically possible, but it
provides no advantage over free functions — the trait itself cannot be used
for dynamic dispatch (no `dyn Pausable`) without `alloc`, and static dispatch
adds generic parameters that propagate through the call sites. Free functions
with an `&Env` parameter are simpler, zero-cost, and achieve the same
deduplication goal.

**A shared `DataKey::Paused` variant in a common enum**: rejected because it
would require each contract to import an external `DataKey` type, coupling
storage schemas across contract crates. Instead, each contract keeps its own
`DataKey` enum; the shared functions use a fixed `symbol_short!("Paused")` key
that is guaranteed not to collide with any address-keyed persistent storage.

**Duplicated but templated code**: the issue specified this as the fallback
if a shared crate proved impractical. It proved practical, so duplication was
not needed.

## Complete End-to-End Wiring Example

A copy-pasteable minimal example contract showing all five wiring steps applied end-to-end (see also [`examples/pausable-consumer`](../examples/pausable-consumer)):

### 1. `Cargo.toml`
```toml
[dependencies]
soroban-sdk = { workspace = true }
compliance-pausable = { workspace = true }

[dev-dependencies]
soroban-sdk = { workspace = true, features = ["testutils"] }
compliance-pausable = { workspace = true, features = ["testutils"] }
```

### 2. Contract Code (`src/lib.rs`)
```rust
#![no_std]

use soroban_sdk::{
    contract, contracterror, contractevent, contractimpl, contracttype, Address, Env, String,
};

// Step 1: Error Variant
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    NotAuthorized = 3,
    ContractPaused = 4, // <-- Added for pausable support
}

// Step 2: Events
#[contractevent]
pub struct Paused {
    #[topic]
    pub admin: Address,
}

#[contractevent]
pub struct Unpaused {
    #[topic]
    pub admin: Address,
}

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    StoredValue,
}

#[contract]
pub struct MinimalPausableContract;

#[contractimpl]
impl MinimalPausableContract {
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }

    // Step 3: Admin-gated pause / unpause / is_paused methods
    pub fn pause(env: Env, admin: Address) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;
        compliance_pausable::pause(&env);
        Paused { admin }.publish(&env);
        Ok(())
    }

    pub fn unpause(env: Env, admin: Address) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;
        compliance_pausable::unpause(&env);
        Unpaused { admin }.publish(&env);
        Ok(())
    }

    pub fn is_paused(env: Env) -> bool {
        compliance_pausable::is_paused(&env)
    }

    // Step 4: Guard placement on every state-mutating method
    pub fn set_value(env: Env, admin: Address, new_value: u32) -> Result<(), Error> {
        compliance_pausable::require_not_paused(&env, Error::ContractPaused)?;
        Self::require_admin(&env, &admin)?;
        env.storage().instance().set(&DataKey::StoredValue, &new_value);
        Ok(())
    }

    // Step 5: Read-only query methods deliberately NOT gated
    pub fn get_value(env: Env) -> u32 {
        env.storage().instance().get(&DataKey::StoredValue).unwrap_or(0)
    }

    fn require_admin(env: &Env, admin: &Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if stored_admin != *admin {
            return Err(Error::NotAuthorized);
        }
        Ok(())
    }
}
```

## Testing

Each contract's test module gains dedicated pausable tests covering:

- `is_paused` defaults to `false`
- admin/issuer can pause and unpause
- every state-mutating method returns `ContractPaused` while paused
- every read-only method succeeds while paused
- mutations succeed again after unpausing
- non-admin/non-issuer cannot pause or unpause
- `pause` and `unpause` each emit the expected event
- the configured pauser can pause but cannot unpause
- the pauser cannot set or clear the pauser address
- `set_pauser` / `clear_pauser` emit `PauserChanged`
- a pause performed by the pauser emits the pauser-specific event
=======

`compliance-pausable` itself has unit tests that exercise all four helpers
against a stub contract.

