# Error code reference

This page is a combined reference of every `Error` enum across the
compliance primitives contracts and examples.

It is generated from the source enums by
`scripts/generate-errors-docs.py`. Do not edit by hand — regenerate
instead:

    python3 scripts/generate-errors-docs.py

Each entry lists the numeric discriminant, the variant name, and the
variant's doc comment (if any).

## Contracts

### allowlist-token

Source: `contracts/allowlist-token/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `NotAuthorized` |  |
| 4 | `InvalidInput` | Caller supplied an argument that is structurally invalid — e.g. a negative token amount. |
| 5 | `ContractPaused` |  |
| 6 | `NoPendingAdmin` |  |
| 7 | `PendingAdminMismatch` |  |
| 8 | `DelegationNotConfigured` |  |
| 9 | `InvalidSignature` |  |
| 10 | `InvalidNonce` |  |
| 11 | `ExpiredSignature` |  |
| 12 | `UpgradeNotReady` | `commit_upgrade` called with no pending upgrade, or before `activated_at`. |
| 13 | `BatchTooLarge` |  |
| 14 | `AddressFrozen` | `transfer` involves an address that is currently frozen. |
| 15 | `ClawbackRequiresFreeze` | `clawback` was called against an address that is not frozen while `clawback_requires_freeze` is enabled. |

### audit-log

Source: `contracts/audit-log/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `NotAuthorized` |  |
| 4 | `PageTooLarge` | `limit` passed to `list_entries` exceeds [`MAX_PAGE_SIZE`]. |
| 5 | `ContractPaused` | `record` was called while the contract is paused. |
| 6 | `DetailTooLong` | `detail` string passed to `record` exceeds [`MAX_DETAIL_LEN`] bytes. |

### circuit-breaker

Source: `contracts/circuit-breaker/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `NotAuthorized` |  |
| 4 | `NoPendingAdmin` |  |
| 5 | `PendingAdminMismatch` |  |

### compliance-aggregator

Source: `contracts/compliance-aggregator/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `NotAuthorized` |  |
| 4 | `NoChecksRegistered` |  |
| 5 | `EmptyAddressList` |  |
| 6 | `BatchTooLarge` |  |
| 7 | `ContractPaused` |  |

### denylist-gate

Source: `contracts/denylist-gate/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `NotAuthorized` |  |
| 4 | `ContractPaused` |  |
| 5 | `BatchTooLarge` |  |
| 6 | `ThresholdNotMet` |  |
| 7 | `InvalidThreshold` |  |
| 8 | `InvalidSignerSet` |  |
| 9 | `SignerNotInSet` |  |
| 10 | `UpgradeNotReady` |  |
| 11 | `SignerAlreadyExists` |  |
| 12 | `MultisigNotEnabled` |  |
| 13 | `NoPendingAdmin` |  |
| 14 | `PendingAdminMismatch` |  |
| 15 | `UpgradeDelayTooShort` |  |

### jurisdiction-flag

Source: `contracts/jurisdiction-flag/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `NotAuthorized` |  |
| 4 | `InvalidInput` | Caller supplied an argument that is structurally invalid. |
| 5 | `ContractPaused` |  |
| 6 | `InvalidJurisdictionCode` | `set_jurisdiction` was given a `code` that is not exactly two uppercase ASCII letters (ISO 3166-1 alpha-2). Also covers the empty string (#81), so there is one variant for every malformed code. |
| 7 | `EmptyAllowedCodes` | The `allowed_codes` list passed to `is_permitted_jurisdiction` was empty. |
| 8 | `NoPendingIssuer` | `accept_issuer` was called when no transfer is in progress. |
| 9 | `PendingIssuerMismatch` | The address calling `accept_issuer` does not match the pending issuer stored by `propose_issuer`. |
| 10 | `TooManyCodes` | `add_jurisdiction` was called when the address already has [`MAX_CODES_PER_ADDRESS`] codes attached. |

### multisig-admin

Source: `contracts/multisig-admin/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` | Contract has not been initialized yet. |
| 2 | `AlreadyInitialized` | `initialize` was called more than once. |
| 3 | `ThresholdNotMet` | The number of valid signatures in the provided set is below threshold. |
| 4 | `InvalidThreshold` | Threshold value is invalid (zero, or greater than signer count). |
| 5 | `SignerNotFound` | The address to remove is not currently in the signer set. |
| 6 | `AlreadySigner` | The address to add is already in the signer set. |
| 7 | `DuplicateSignature` | The same signer address appears more than once in the provided signature set for a single `__check_auth` call. |
| 8 | `ContractPaused` | The contract is paused; no mutating operations are allowed. |
| 9 | `ProposalNotFound` | The referenced proposal does not exist. |
| 10 | `ExpiredProposal` | The proposal has passed its expiry ledger sequence. |
| 11 | `DuplicateSigner` | A duplicate signer address was found in the `signers` list passed to `initialize`. Duplicates are rejected because `__check_auth` counts distinct signer addresses, so a duplicated entry can make the threshold impossible to reach. |
| 12 | `NotASigner` | The caller is not in the current signer set. |
| 13 | `AlreadyApproved` | The signer has already approved this proposal. |

### policy-engine

Source: `contracts/policy-engine/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `NotAuthorized` |  |
| 4 | `PolicyViolation` |  |
| 5 | `MaxDepthExceeded` | Returned by `add_check` when the number of registered checks would exceed `MAX_CHECKS`. Keeps per-evaluation resource cost bounded and prevents unbounded storage growth. |
| 6 | `IndexOutOfBounds` | Returned by `swap_checks` when either index `i` or `j` is out of range for the current check list. |
| 7 | `ContractPaused` | Returned by state-changing entry points while the contract is paused. |
| 8 | `CheckIndexOutOfRange` | Returned by `get_check` when the requested index is out of range. |
| 9 | `UpgradeNotReady` | Returned by `commit_upgrade` when no upgrade is pending or its delay has not elapsed. |

## Examples

### circuit-breaker-consumer

Source: `examples/circuit-breaker-consumer/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `InsufficientBalance` |  |
| 4 | `FrozenByBreaker` |  |

### circuit-breaker-policy-engine

Source: `examples/circuit-breaker-policy-engine/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `CircuitBreakerFrozen` |  |
| 2 | `PolicyViolation` |  |

### denylist-gate-consumer

Source: `examples/denylist-gate-consumer/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `InsufficientBalance` |  |
| 4 | `DeniedByGate` |  |
| 5 | `FrozenByBreaker` |  |

### denylist-gate-sep41

Source: `examples/denylist-gate-sep41/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `NotAuthorized` |  |
| 4 | `InsufficientBalance` |  |
| 5 | `InsufficientAllowance` |  |
| 6 | `DeniedByGate` |  |

### jurisdiction-denylist-consumer

Source: `examples/jurisdiction-denylist-consumer/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `InsufficientBalance` |  |
| 4 | `DeniedByGate` |  |
| 5 | `DeniedByJurisdiction` |  |

### jurisdiction-flag-consumer

Source: `examples/jurisdiction-flag-consumer/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `InsufficientBalance` |  |
| 4 | `JurisdictionNotPermitted` |  |

### multisig-aggregator

Source: `examples/multisig-aggregator/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |

### pausable-consumer

Source: `examples/pausable-consumer/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `NotAuthorized` |  |
| 4 | `ContractPaused` | Returned whenever a state-mutating method is invoked while the contract is paused. |

### policy-engine-audit

Source: `examples/policy-engine-audit/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `PolicyViolation` |  |

### rwa-token

Source: `examples/rwa-token/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `InsufficientBalance` |  |
| 4 | `NotAllowlisted` |  |
| 5 | `DeniedByGate` |  |
| 6 | `JurisdictionNotPermitted` |  |
| 7 | `CircuitBreakerFrozen` |  |

### rwa-token-aggregator

Source: `examples/rwa-token-aggregator/src/lib.rs`

| Code | Error | Description |
|------|-------|-------------|
| 1 | `NotInitialized` |  |
| 2 | `AlreadyInitialized` |  |
| 3 | `InsufficientBalance` |  |
| 4 | `InvalidAmount` |  |
| 5 | `CircuitBreakerFrozen` |  |
| 6 | `DeniedByGate` |  |
| 7 | `JurisdictionNotPermitted` |  |
| 8 | `ComplianceUnavailable` | The aggregator call itself failed (misconfiguration or a downstream trap). Treated as a denial: the token fails closed. |

