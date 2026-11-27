# policy-engine

A configurable policy composition contract that evaluates denylist and jurisdiction checks together.

| Method | Purpose |
| --- | --- |
| `initialize(admin, combine_op)` | Initialize governance and combine semantics. |
| `propose_upgrade(admin, new_wasm, delay_ledgers)` | Propose an upgrade that is installable after the ledger delay. |
| `commit_upgrade(admin)` | Install the pending upgrade after its delay. |
| `cancel_upgrade(admin)` | Cancel a pending upgrade. |
| `create_policy(admin, name, combine_op)` | Create a named policy with its own combine semantics and admin. |
| `set_policy_admin(admin, name, new_admin)` | Transfer admin rights for a single named policy. |
| `remove_policy(admin, name)` | Remove a named policy and its configured checks. |
| `set_max_checks(admin, max_checks)` | Set the maximum number of checks per policy, bounded by `MAX_CHECKS_HARD_LIMIT`. |
| `add_check(admin, name, check)` | Add a configured compliance check to a named policy. |
| `remove_check(admin, name, index)` | Remove a configured check from a named policy. |
| `evaluate_policy(name, address) -> bool` | Return the combined decision for the named policy against `address`. |
| `evaluate_with_amount(name, address, amount) -> bool` | Return the combined decision for the named policy, passing an optional amount to amount-aware checks. |
| `evaluate(from, to) -> bool` | Return the combined policy decision for the default policy. |
| `evaluate_verbose(from, to) -> (bool, Option<CheckFailure>)` | Return the combined policy decision together with failure details when the policy rejects. |
| `get_policy(name)` | Read a named policy, including its `policy_version` and `max_checks`. |
| `get_checks(name)` | Read the configured checks for a named policy. |
| `get_op(name)` | Read whether the named policy's checks are combined with all/any semantics. |
| `is_paused()` | Read whether the policy engine is currently paused. |

## Multiple named policies

A single engine hosts multiple named policies so integrators can apply different rules per action (e.g. `mint`, `transfer`, `redeem`) without deploying one engine per policy. Each policy has its own combine operation, check list, `max_checks` limit, and `policy_version`.

### Per-policy admin semantics

Each named policy has its own admin. Only that policy's admin may mutate the policy (add/remove checks, change its admin, remove the policy). The engine-level admin from `initialize` governs engine-wide settings such as `set_max_checks` and upgrades. `set_policy_admin` transfers admin rights for a single policy without affecting other policies.

## Check limits

`set_max_checks` lets the admin configure the maximum number of checks a policy may hold. The value is clamped to `MAX_CHECKS_HARD_LIMIT`, a hard upper bound enforced in code that cannot be exceeded regardless of admin input. Attempts to add checks beyond the effective limit fail.

## Policy versioning

Every policy carries a monotonic `policy_version` that is bumped on every mutation of that policy (adding or removing a check, changing its admin, changing combine semantics, or changing `max_checks`). The current version is returned by `get_policy(name)` and included in `PolicyResult` events, so off-chain systems can tell which policy version approved a transfer.

## `PolicyResult` event

Evaluation emits a `PolicyResult` event with the following fields:

| Field | Description |
| --- | --- |
| `name` | The named policy that was evaluated. |
| `address` | The address the policy was evaluated against. |
| `amount` | The optional amount passed to amount-aware checks, if any. |
| `allowed` | The combined decision. |
| `policy_version` | The policy version at evaluation time. |
| `check_index` | Index of the failing check, or `u32::MAX` when no specific check applies. |
| `kind` | Failure kind, or empty when the policy accepted. |

## Amount-aware checks

`evaluate_with_amount(name, address, amount)` passes an optional amount to checks that implement the extended trait in `compliance-check`, enabling amount limits such as travel-rule thresholds. Checks that are not amount-aware ignore the amount.

## Failure reporting semantics

`evaluate_verbose` returns the combined decision plus an optional `CheckFailure` describing why the policy rejected. The reported failure depends on the combine operation:

- **`All`**: the policy fails as soon as a check fails, so the first failing check is the meaningful cause. The returned `CheckFailure` carries that check's `check_index` and `kind`.
- **`Any`**: the policy only fails when *every* check fails, so no single check is to blame. In this case the returned `CheckFailure` uses `check_index = u32::MAX` (a sentinel meaning "no specific check") and `kind = "AllFailed"`, indicating that all checks failed.

When the policy accepts, the `Option<CheckFailure>` is `None`.