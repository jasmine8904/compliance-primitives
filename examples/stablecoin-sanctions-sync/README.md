# stablecoin-sanctions-sync

A stablecoin issuer that refuses transfers to or from sanctioned addresses,
with the denylist kept in step with an off-chain sanctions feed.

The loop has two halves:

1. **On-chain gate.** A SEP-41 token ([`denylist-gate-sep41`](../denylist-gate-sep41))
   calls `denylist-gate.check(address)` on every transfer. A listed address
   cannot send or receive.
2. **Off-chain sync.** An oracle/adapter turns the sanctions feed into a
   plain list of addresses. [`sync.sh`](./sync.sh) makes the on-chain denylist
   match that list: it adds new entries, removes delisted ones, and records
   what it last applied.

```
sanctions feed ──► adapter (compliance-adapters) ──► list.txt ──► sync.sh ──► denylist-gate
                                                                               ▲
                                  stablecoin transfer ──► check(from), check(to)┘
```

The feed-to-list adapter is not part of this repo. See
[**compliance-adapters**](https://github.com/stellar-compliance-kit/compliance-adapters)
for sanctions-oracle adapters that produce the list file this script consumes.

## Setup

Build the contracts and deploy the gate and the token:

```sh
stellar contract build

# 1. The denylist.
GATE_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/denylist_gate.wasm \
  --source issuer --network testnet)
stellar contract invoke --id "$GATE_ID" --source issuer --network testnet \
  -- initialize --admin "$ISSUER_ADDRESS"

# 2. The stablecoin, gated by the denylist.
TOKEN_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/denylist_gate_sep41.wasm \
  --source issuer --network testnet)
stellar contract invoke --id "$TOKEN_ID" --source issuer --network testnet \
  -- initialize --admin "$ISSUER_ADDRESS" --gate "$GATE_ID" \
  --decimal 7 --name "Example USD" --symbol "EUSD"
```

## Sync

Each run takes the current sanctions list and applies the difference against
the last successful run:

```sh
DENYLIST_GATE_ID="$GATE_ID" \
COMPLIANCE_SOURCE=issuer \
ADMIN="$ISSUER_ADDRESS" \
NETWORK=testnet \
  ./examples/stablecoin-sanctions-sync/sync.sh ./sanctions/list.txt
```

`list.txt` is one `G...` address per line. `#` comments and blank lines are
allowed. The snapshot is written to `last-synced.txt` next to the list, after
every invoke succeeds.

Run it on a schedule (cron, a CI job, or a worker). A failed run is safe to
repeat.

## Safety properties

- **Empty list is refused.** A failed oracle fetch that writes an empty file
  will not clear the denylist.
- **Malformed input is refused.** Any line that is not a 56-character `G...`
  address stops the sync before any on-chain write.
- **Idempotent writes.** `add_to_denylist` and `remove_from_denylist` can be
  repeated without harm, so a partial run is recovered by re-running.
- **Audit trail.** Each add and remove emits a `deny_add`/`deny_remove` entry
  in the configured audit log, if one is set on the gate.

## Limits (shortcuts)

- The snapshot is a local file. Two syncers on different hosts can drift.
  Keep one scheduled writer, or move the snapshot to shared storage.
- Writes are one invoke per address. Large list changes are slow. Batch
  removals with `remove_multiple_from_denylist` when the volume justifies it.
- The issuer key is a single signer. For production, make the admin a
  `multisig-admin` account (see [`multisig-governed-allowlist`](../multisig-governed-allowlist)).
- This example does not run the sync against a live network. It was written
  against the contract entry points in `contracts/denylist-gate` and
  `examples/denylist-gate-sep41`; run it on testnet before relying on it.
