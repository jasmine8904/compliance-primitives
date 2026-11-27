# Integrator FAQ

Answers to questions that come up repeatedly when integrating these
primitives. Each answer points at the code or design doc it is based on.
Where a claim is an estimate rather than something checked on a live network,
it says so.

Ledger-time figures assume ~5 seconds per ledger (mainnet).

---

## TTL and storage

### Do my compliance entries expire?

Yes. Persistent entries (per-address denylist, allowlist, and jurisdiction
records) carry a TTL. When it lapses, Soroban archives the entry. Archival is
not a delete, but a normal read no longer returns it. Each primitive's
archival behaviour is different, so check this before you choose one:

| Primitive | Archived entry read as | Effect |
|---|---|---|
| `denylist-gate::check` | not denied | **fail-open**: the address is treated as clear to transact |
| `allowlist-token::is_allowed` | not allowed | fail-closed |
| `jurisdiction-flag::is_permitted_jurisdiction` | no matching code | fail-closed |
| `circuit-breaker::is_frozen` | not frozen | **fail-open** |

Source: [`fail-open-fail-closed-audit.md`](./fail-open-fail-closed-audit.md).

### How long does an entry live, and what extends it?

- **Denylist entries** are written with `MAX_TTL = 6,311,520` ledgers, about
  one year (`contracts/denylist-gate/src/lib.rs`). The TTL is extended only by
  `add_to_denylist`. `check` does not renew it. An entry you added a year ago
  and never touched will lapse, and `check` will then fail open for that
  address.
- **Allowlist entries** are extended to about 90 days (`1,555,200` ledgers)
  when their remaining TTL drops below about 7 days (`120,960` ledgers). The
  extension happens on `add_to_allowlist`.
- **Instance storage** (admin, pause flag, signer set, and similar) is extended
  explicitly by write paths and by `denylist-gate::check`. Soroban does not
  extend instance TTL on its own.

### What should I do so entries don't lapse?

Treat TTL as an operational duty, not a set-and-forget setting:

1. Re-add long-lived denylist entries periodically (`add_to_denylist` or the
   batch variant). Re-adding resets the TTL.
2. Or run a keeper that extends TTLs before they expire.
3. Alert on remaining TTL. The fail-open audit suggests alerting at about 30
   days remaining, using the `/tools/indexer` component.
4. Treat an archived entry as a compliance incident. Restore it before
   relying on the gate again.

For the audit log specifically, see
[`audit-log-retention.md`](./audit-log-retention.md). Archived audit entries
are not readable through normal calls.

---

## Upgrades

### Can I upgrade a deployed primitive without losing state?

Soroban swaps the contract's code and keeps its storage. The new code must
still read what the old code wrote. So an upgrade is safe only if the storage
layout stays compatible. The rules are in
[`STORAGE_VERSIONING.md`](../STORAGE_VERSIONING.md). In short:

- Safe: adding new keys, reordering variants, adding functions that only use
  new keys.
- Breaking, needs a documented migration: renaming or removing a key, changing
  a stored value's shape, or changing what an existing key means.

Soroban has no rollback. Once new code runs, the only way back is another
upgrade.

### How does upgrade authority work in each contract?

Authority and delay differ by contract. Check the contract you deploy:

| Contract | Upgrade entrypoint | Delay |
|---|---|---|
| `denylist-gate`, `allowlist-token`, `policy-engine` | `propose_upgrade` / `commit_upgrade` (admin) | Timelocked. The proposal sets `delay_ledgers` and the admin can cancel. |
| `audit-log`, `circuit-breaker`, `compliance-aggregator`, `multisig-admin` | `upgrade` (admin) | Immediate. |
| `jurisdiction-flag` | `upgrade` (issuer) | **See the warning below.** |

For a timelocked contract, the window gives you time to notice a bad proposal
and cancel it. It does not protect against a compromised admin key. If the
proposal is not cancelled, it commits after the delay. Use a multisig admin for
stronger control.

> **Known issue: `jurisdiction-flag` upgrades are not working.** The current
> `upgrade()` in `contracts/jurisdiction-flag/src/lib.rs` checks issuer auth,
> clears a pending-upgrade key, and returns. It never calls
> `update_current_contract_wasm`, so **it does not change the deployed code.**
> `contracts/jurisdiction-flag/UPGRADE.md` describes a timelocked
> propose/commit/cancel flow that the code does not implement. Do not plan
> around a `jurisdiction-flag` upgrade until this is fixed and tested on
> testnet.

### How do I rehearse an upgrade?

Run the full sequence on testnet before mainnet: upload the new Wasm, propose
(or call `upgrade`), monitor and cancel if needed, commit, then check that
storage reads still return what you expect. The steps are in
`contracts/jurisdiction-flag/UPGRADE.md`. Breaking schema changes need a
documented migration that the issuer runs manually. Migrations never run
automatically inside `upgrade()`.

---

## Cost per check

### What does a compliance check cost?

These figures are **estimates** from [`BENCHMARKS.md`](../BENCHMARKS.md), not
live measurements. The benchmark table still has `(measured)` placeholders
for some rows.

| Check | Estimated CPU instructions | Share of a ~10M-instruction budget |
|---|---|---|
| Denylist, per address | ~125 | ~0.0025% |
| Allowlist, per address | ~200 | ~0.004% |
| Jurisdiction, per check | ~150 | not stated |

Most of each check is cross-contract call overhead, not compliance logic.
Batching checks into fewer calls reduces that overhead.

Actual fees depend on the network's current resource-fee schedule, which is
not fixed in this repo. Re-measure with `simulateTransaction` on the network
you target before you quote a number to a user.

### What about batch sizes?

Batch operations have caps (for example `MAX_BATCH_SIZE = 45` in
`allowlist-token`). `BENCHMARKS.md` shows that `jurisdiction-flag`'s
`remove_jurisdiction_multiple` exceeds the default budget at batch size 30 and
has no cap yet. Do not send large batches to that function until a cap is in
place.

---

## Pausing

### What does pause stop?

Pause is an admin-controlled switch. It blocks state-changing calls and leaves
read-only calls working, so compliance queries still answer while paused. The
per-contract breakdown is in [`pausable-design.md`](./pausable-design.md). For
example:

- `denylist-gate`: pause blocks `add_to_denylist` and `remove_from_denylist`.
  `check` keeps working. This is verified in
  `contracts/denylist-gate/src/lib.rs`.
- `allowlist-token`: pause blocks add/remove and `transfer`. `is_allowed` keeps
  working.
- `jurisdiction-flag`: pause blocks `set_jurisdiction` and related writes.

A paused `transfer` returns an error and rolls back its events. It does not
return `Ok(false)`.

### Does pausing stop the gate from enforcing rules?

No. Pausing stops changes to the lists. Existing entries still apply to
`check` and `is_allowed`, so a paused denylist still blocks the addresses on it.
Use pause to freeze the lists during an incident, not to switch enforcement
off.

### Who can pause?

Only the admin or issuer, the same principal that controls list membership.
There is no separate pauser role. A compromised admin key can therefore both
change lists and pause them, so protect that key accordingly.

---

## Where to look next

- [`PRIMITIVE_SELECTION_GUIDE.md`](./PRIMITIVE_SELECTION_GUIDE.md): which
  primitives to combine for your use case.
- [`TROUBLESHOOTING.md`](./TROUBLESHOOTING.md): local build problems.
- [`pre-mainnet-checklist.md`](./pre-mainnet-checklist.md): what to verify
  before going live.
