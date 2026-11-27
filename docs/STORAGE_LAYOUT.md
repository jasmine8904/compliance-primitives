# Storage Layout Reference

This document lists every storage key, storage tier, TTL policy, and
removal condition for each contract in this workspace.  It is the
companion to [STORAGE_VERSIONING.md](../STORAGE_VERSIONING.md), which
explains the compatibility commitments; this file tells you what is
actually stored and where.

**Key encoding note.** Soroban `#[contracttype]` encodes enum variants by
name as an XDR symbol vector, not by position.  Renaming a variant changes
its on-chain key permanently.  Reordering or adding variants is safe.

---

## Table of contents

1. [allowlist-token](#allowlist-token)
2. [audit-log](#audit-log)
3. [circuit-breaker](#circuit-breaker)
4. [compliance-aggregator](#compliance-aggregator)
5. [denylist-gate](#denylist-gate)
6. [jurisdiction-flag](#jurisdiction-flag)
7. [multisig-admin](#multisig-admin)
8. [pausable](#pausable)
9. [policy-engine](#policy-engine)

---

## allowlist-token

TTL constants (source: `contracts/allowlist-token/src/lib.rs`):

| Constant                  | Ledgers   | Approximate wall time |
|---------------------------|-----------|-----------------------|
| `INSTANCE_TTL_THRESHOLD`  | 120 960   | ~7 days               |
| `INSTANCE_TTL_EXTEND_TO`  | 1 555 200 | ~90 days              |
| `ALLOWED_TTL_THRESHOLD`   | 120 960   | ~7 days               |
| `ALLOWED_TTL_EXTEND_TO`   | 1 555 200 | ~90 days              |

| Key                          | Type        | Tier       | TTL policy                                                   | Removed when                            |
|------------------------------|-------------|------------|--------------------------------------------------------------|-----------------------------------------|
| `Admin`                      | `Address`   | Instance   | Extended on every write path; threshold 120 960, target 1 555 200 | Never (required for all admin calls)    |
| `ComplianceOfficer`          | `Address`   | Instance   | Shares instance TTL                                          | Explicit `remove_compliance_officer`    |
| `Token`                      | `Address`   | Instance   | Shares instance TTL                                          | Never                                   |
| `Paused`                     | `bool`      | Instance   | Shares instance TTL                                          | Never (toggled between `true`/`false`)  |
| `PendingAdmin`               | `Address`   | Instance   | Shares instance TTL                                          | Cleared on `accept_admin` or `cancel_admin_transfer` |
| `DelegatedAdminPubKey`       | `Bytes`     | Instance   | Shares instance TTL                                          | Explicit removal                        |
| `ClawbackRequiresFreeze`     | `bool`      | Instance   | Shares instance TTL                                          | Never (toggled)                         |
| `PendingUpgrade`             | `UpgradeState` | Instance | Shares instance TTL                                         | Cleared on `apply_upgrade` or `cancel_upgrade` |
| `Allowed(Address)`           | `bool`      | Persistent | Extended on write; threshold 120 960, target 1 555 200       | Explicit `revoke` or `clawback`         |
| `Frozen(Address)`            | `bool`      | Persistent | Extended on write; threshold 120 960, target 1 555 200       | Explicit `unfreeze`                     |
| `DelegatedNonce(Address)`    | `u64`       | Persistent | Extended on write; threshold 120 960, target 1 555 200       | Never (monotonically increasing)        |

---

## audit-log

TTL constants (source: `contracts/audit-log/src/lib.rs`):

| Constant              | Ledgers   | Approximate wall time |
|-----------------------|-----------|-----------------------|
| `ENTRY_TTL_THRESHOLD` | 120 960   | ~7 days               |
| `ENTRY_TTL_EXTEND_TO` | 1 555 200 | ~90 days              |

Instance TTL is **not** explicitly extended on writes; `Admin` and
`EntryCount` share the contract's default instance TTL.

| Key            | Type       | Tier       | TTL policy                                           | Removed when                     |
|----------------|------------|------------|------------------------------------------------------|----------------------------------|
| `Admin`        | `Address`  | Instance   | Default instance TTL (not explicitly extended)       | Never                            |
| `EntryCount`   | `u64`      | Instance   | Default instance TTL                                 | Never (monotonically increasing) |
| `Entry(u64)`   | `LogEntry` | Persistent | Extended on `record`; threshold 120 960, target 1 555 200 | Never (immutable trail); may expire if not renewed |

> **Operational note.** Once written, entries are never deleted by the
> contract.  An entry that is not renewed before its TTL expires will be
> archived on-chain.  Run an external TTL-extension process if long-term
> retention beyond ~90 days per entry is required.

---

## circuit-breaker

TTL constants (source: `contracts/circuit-breaker/src/lib.rs`):

| Constant                 | Ledgers   | Approximate wall time |
|--------------------------|-----------|-----------------------|
| `INSTANCE_TTL_THRESHOLD` | 120 960   | ~7 days               |
| `INSTANCE_TTL_EXTEND_TO` | 1 555 200 | ~90 days              |

All state is **instance** storage.  Every write path extends the instance
TTL (threshold 120 960, target 1 555 200).

| Key            | Type            | Tier     | TTL policy            | Removed when                                      |
|----------------|-----------------|----------|-----------------------|---------------------------------------------------|
| `Admin`        | `Address`       | Instance | Extended on every write | Never                                           |
| `Frozen`       | `bool`          | Instance | Extended on every write | Never (toggled between `true`/`false`)          |
| `PendingAdmin` | `Address`       | Instance | Extended on every write | Cleared on `accept_admin` or `cancel_admin_transfer` |
| `Guardians`    | `Vec<Address>`  | Instance | Extended on every write | Never (can be emptied via `remove_guardian`)    |

---

## compliance-aggregator

TTL constants (source: `contracts/compliance-aggregator/src/lib.rs`):

| Constant                  | Ledgers   | Approximate wall time |
|---------------------------|-----------|-----------------------|
| `INSTANCE_TTL_THRESHOLD`  | 120 960   | ~7 days               |
| `INSTANCE_TTL_EXTEND_TO`  | 1 555 200 | ~90 days              |

All state is **instance** storage.  Every write path extends the instance
TTL.

| Key                | Type      | Tier     | TTL policy              | Removed when                                  |
|--------------------|-----------|----------|-------------------------|-----------------------------------------------|
| `Admin`            | `Address` | Instance | Extended on every write | Never                                         |
| `DenylistGate`     | `Address` | Instance | Extended on every write | Explicit `unset_denylist_gate`                |
| `JurisdictionFlag` | `Address` | Instance | Extended on every write | Explicit `unset_jurisdiction_flag`            |
| `CircuitBreaker`   | `Address` | Instance | Extended on every write | Explicit removal via `set_circuit_breaker(None)` or equivalent |

---

## denylist-gate

TTL constants (source: `contracts/denylist-gate/src/lib.rs`):

| Constant                  | Ledgers   | Approximate wall time |
|---------------------------|-----------|-----------------------|
| `INSTANCE_TTL_THRESHOLD`  | 120 960   | ~7 days               |
| `INSTANCE_TTL_EXTEND_TO`  | 1 555 200 | ~90 days              |
| `APPROVAL_TTL_LEDGERS`    | 120 960   | ~7 days               |

| Key                              | Type               | Tier       | TTL policy                               | Removed when                                     |
|----------------------------------|--------------------|------------|------------------------------------------|--------------------------------------------------|
| `Admin`                          | `Address`          | Instance   | Extended on every write                  | Never                                            |
| `PendingAdmin`                   | `Address`          | Instance   | Extended on every write                  | Cleared on `accept_admin` or `cancel_admin_transfer` |
| `Paused`                         | `bool`             | Instance   | Extended on every write                  | Never (toggled)                                  |
| `ComplianceOfficer`              | `Address`          | Instance   | Extended on every write                  | Explicit `remove_compliance_officer`             |
| `AuditLog`                       | `Address`          | Instance   | Extended on every write                  | Explicit `unset_audit_log`                       |
| `SignerSet`                      | `SignerSet`        | Instance   | Extended on every write                  | Never (replaced by `initialize_multisig`)        |
| `PendingUpgrade`                 | `UpgradeState`     | Instance   | Extended on every write                  | Cleared on `apply_upgrade` or `cancel_upgrade`   |
| `Denied(Address)`                | `bool`             | Persistent | Not explicitly extended after write      | Explicit `allow` (removes the deny entry)        |
| `PendingSignerAction(SignerAction)` | `PendingApproval` | Persistent | TTL set to `APPROVAL_TTL_LEDGERS` on write | Cleared once action reaches threshold or expires |

---

## jurisdiction-flag

TTL constants (source: `contracts/jurisdiction-flag/src/lib.rs`):

| Constant                 | Ledgers   | Approximate wall time |
|--------------------------|-----------|-----------------------|
| `TTL_THRESHOLD`          | 120 960   | ~7 days               |
| `TTL_EXTEND_TO`          | 1 555 200 | ~90 days              |
| `INSTANCE_TTL_THRESHOLD` | 120 960   | ~7 days               |
| `INSTANCE_TTL_EXTEND_TO` | 1 555 200 | ~90 days              |

| Key                        | Type            | Tier       | TTL policy                                     | Removed when                                              |
|----------------------------|-----------------|------------|------------------------------------------------|-----------------------------------------------------------|
| `Issuer`                   | `Address`       | Instance   | Extended on every write                        | Never                                                     |
| `PendingIssuer`            | `Address`       | Instance   | Extended on every write                        | Cleared on `accept_issuer` or `cancel_issuer_transfer`    |
| `ComplianceOfficer`        | `Address`       | Instance   | Extended on every write                        | Explicit `remove_compliance_officer`                      |
| `Paused`                   | `bool`          | Instance   | Extended on every write                        | Never (toggled)                                           |
| `PendingUpgrade`           | `UpgradeState`  | Instance   | Extended on every write                        | Cleared on `apply_upgrade` or `cancel_upgrade`            |
| `Jurisdiction(Address)`    | `String`        | Persistent | Extended on write; threshold 120 960, target 1 555 200 | Explicit `revoke_jurisdiction` (legacy single-code key)   |
| `Jurisdictions(Address)`   | `Vec<String>`   | Persistent | Extended on write; threshold 120 960, target 1 555 200 | Explicit `revoke_jurisdiction`                            |
| `ValidUntil(Address)`      | `u32`           | Persistent | Extended on write; threshold 120 960, target 1 555 200 | Cleared on `revoke_jurisdiction` or when expiry is removed |

> **Legacy key.** `Jurisdiction(Address)` stored a single jurisdiction code
> (pre-multi-jurisdiction).  New writes use `Jurisdictions(Address)`.  Both
> keys are read during migration; `Jurisdiction` entries are removed once
> migrated.

---

## multisig-admin

TTL constants (source: `contracts/multisig-admin/src/lib.rs`):

| Constant                 | Ledgers | Approximate wall time |
|--------------------------|---------|-----------------------|
| `INSTANCE_TTL_THRESHOLD` | 17 280  | ~1 day                |
| `INSTANCE_TTL_EXTEND_TO` | 518 400 | ~30 days              |

All state is **instance** storage.  Every write path extends the instance
TTL (threshold 17 280, target 518 400).  Note the shorter horizon compared
to other contracts; plan TTL-extension operations accordingly.

| Key               | Type        | Tier     | TTL policy              | Removed when                                      |
|-------------------|-------------|----------|-------------------------|---------------------------------------------------|
| `Signers`         | `Vec<Address>` | Instance | Extended on every write | Never (replaced on signer-set updates)           |
| `Threshold`       | `u32`       | Instance | Extended on every write | Never (updated in place)                          |
| `NextProposalId`  | `u64`       | Instance | Extended on every write | Never (monotonically increasing)                  |
| `Proposal(u64)`   | `Proposal`  | Instance | Extended on every write | Removed once proposal is executed or cancelled    |

---

## pausable

The `pausable` crate is a shared library; it does not set its own TTL.
Contracts that embed it inherit their own instance TTL policy.

| Key           | Type     | Tier     | TTL policy           | Removed when                        |
|---------------|----------|----------|----------------------|-------------------------------------|
| `Paused`      | `bool`   | Instance | Inherits caller's TTL | Never (toggled)                    |
| `PausedSince` | `u64`    | Instance | Inherits caller's TTL | Cleared by `unpause`               |
| `PauseReason` | `String` | Instance | Inherits caller's TTL | Cleared by `unpause`; absent when no reason was recorded |

---

## policy-engine

TTL constants (source: `contracts/policy-engine/src/lib.rs`):

| Constant                 | Ledgers   | Approximate wall time |
|--------------------------|-----------|-----------------------|
| `INSTANCE_TTL_THRESHOLD` | 120 960   | ~7 days               |
| `INSTANCE_TTL_EXTEND_TO` | 1 555 200 | ~90 days              |

All state is **instance** storage.  Every write path extends the instance
TTL.

| Key              | Type            | Tier     | TTL policy              | Removed when                                     |
|------------------|-----------------|----------|-------------------------|--------------------------------------------------|
| `Admin`          | `Address`       | Instance | Extended on every write | Never                                            |
| `Checks`         | `Vec<CheckKind>`| Instance | Extended on every write | Never (replaced in place)                        |
| `CombineOp`      | `CombineOp`     | Instance | Extended on every write | Never (updated in place)                         |
| `CircuitBreaker` | `Address`       | Instance | Extended on every write | Explicit `set_circuit_breaker(None)` equivalent  |
| `PendingUpgrade` | `UpgradeState`  | Instance | Extended on every write | Cleared on `apply_upgrade` or `cancel_upgrade`   |

---

## Storage tier summary

| Tier       | Scope per contract instance | Archived when TTL lapses | Restorable |
|------------|-----------------------------|--------------------------|------------|
| Instance   | Shared with the contract code entry; archiving the instance archives all keys at once | Yes | Yes |
| Persistent | Per-key TTL; each entry is archived independently when its own TTL expires | Yes (per entry) | Yes |
| Temporary  | Not used by any contract in this workspace | n/a | No |
