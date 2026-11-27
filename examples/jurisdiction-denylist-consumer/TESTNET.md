# Jurisdiction + denylist consumer — Stellar testnet walkthrough

Deploy the `denylist-gate`, `jurisdiction-flag`, and composing
`jurisdiction-denylist-consumer` contracts, then exercise a passing transfer
and each compliance rejection on testnet.

> **CLI requirement**: contracts use `soroban-sdk` 27. Use **stellar-cli ≥ 23**
> (ideally **27.x**) to deploy and invoke them.

## Current reference addresses

The canonical list of deployed testnet IDs lives in the repository root:

- `README.md` — "Deployed testnet contract IDs" table (one row per release)
- `testnet-ids.json` — machine-readable copy of the same data

The deploy scripts write the freshly deployed IDs into both files
automatically, so you normally do not need to copy them by hand. The table
below is only a local scratchpad for the walkthrough in this document.

| Contract / account | Testnet address |
| --- | --- |
| Issuer / admin | _fill after deploy_ |
| Alice (sender) | _fill after account generation_ |
| Bob (recipient) | _fill after account generation_ |
| `denylist-gate` | _fill after deploy_ |
| `jurisdiction-flag` | _fill after deploy_ |
| `jurisdiction-denylist-consumer` | _fill after deploy_ |
| Permitted jurisdictions | `["US","CA"]` |

Testnet resets wipe contract state. After a reset (or when these IDs go stale),
repeat the deployment steps below; the deploy scripts refresh the root
`README.md` table and `testnet-ids.json`, and you can copy the new values into
the scratchpad above.

## Deploy the contracts

Generate funded testnet identities for the issuer and transfer participants:

```sh
stellar keys generate jurisdiction-issuer --network testnet --fund
stellar keys generate jurisdiction-alice --network testnet --fund
stellar keys generate jurisdiction-bob --network testnet --fund
```

Build the three contracts:

```sh
cargo build \
  -p denylist-gate \
  -p jurisdiction-flag \
  -p jurisdiction-denylist-consumer \
  --target wasm32v1-none \
  --release
```

Deploy each Wasm artifact. The deploy scripts record the resulting IDs in the
root `README.md` testnet table and `testnet-ids.json`; copy them into the
scratchpad above for the walkthrough:

```sh
stellar contract deploy \
  --wasm target/wasm32v1-none/release/denylist_gate.wasm \
  --source jurisdiction-issuer --network testnet

stellar contract deploy \
  --wasm target/wasm32v1-none/release/jurisdiction_flag.wasm \
  --source jurisdiction-issuer --network testnet

stellar contract deploy \
  --wasm target/wasm32v1-none/release/jurisdiction_denylist_consumer.wasm \
  --source jurisdiction-issuer --network testnet
```

Set shell aliases from the deployed IDs and generated account addresses:

```sh
export NETWORK=testnet
export ISSUER=jurisdiction-issuer
export ISSUER_ADDRESS="$(stellar keys address "$ISSUER")"
export ALICE="$(stellar keys address jurisdiction-alice)"
export BOB="$(stellar keys address jurisdiction-bob)"
export GATE=<denylist-gate-id>
export JURISDICTION=<jurisdiction-flag-id>
export CONSUMER=<jurisdiction-denylist-consumer-id>
```

Initialize the denylist and jurisdiction contracts with the issuer, then wire
both addresses and the permitted jurisdiction codes into the consumer:

```sh
stellar contract invoke --id "$GATE" --source "$ISSUER" --network "$NETWORK" -- \
  initialize --admin "$ISSUER_ADDRESS"

stellar contract invoke --id "$JURISDICTION" --source "$ISSUER" --network "$NETWORK" -- \
  initialize --issuer "$ISSUER_ADDRESS"

stellar contract invoke --id "$CONSUMER" --source "$ISSUER" --network "$NETWORK" -- \
  initialize \
  --gate "$GATE" \
  --jurisdiction "$JURISDICTION" \
  --allowed_jurisdictions '["US","CA"]'
```

## Walkthrough (stellar-cli)

The consumer checks the denylist for both `from` and `to`, then checks the
sender's jurisdiction against the configured list. The current example does
not require a jurisdiction flag for the recipient.

### 1. Assign the sender's jurisdiction and mint a balance

```sh
stellar contract invoke --id "$JURISDICTION" --source "$ISSUER" --network "$NETWORK" -- \
  set_jurisdiction \
  --issuer "$ISSUER_ADDRESS" \
  --address "$ALICE" \
  --code '"US"'

stellar contract invoke --id "$CONSUMER" --source "$ISSUER" --network "$NETWORK" -- \
  mint --to "$ALICE" --amount 1000
```

### 2. Successful transfer

Alice authorizes the transfer using her testnet identity:

```sh
stellar contract invoke --id "$CONSUMER" --source jurisdiction-alice --network "$NETWORK" -- \
  transfer --from "$ALICE" --to "$BOB" --amount 400
# → Ok; balances 600 / 400
```

### 3. Blocked: denylist

Add Alice to the denylist, then retry the transfer. It should fail with
`DeniedByGate` (error 4):

```sh
stellar contract invoke --id "$GATE" --source "$ISSUER" --network "$NETWORK" -- \
  add_to_denylist --admin "$ISSUER_ADDRESS" --address "$ALICE"

stellar contract invoke --id "$CONSUMER" --source jurisdiction-alice --network "$NETWORK" -- \
  transfer --from "$ALICE" --to "$BOB" --amount 10
# → Error::DeniedByGate (4)

stellar contract invoke --id "$GATE" --source "$ISSUER" --network "$NETWORK" -- \
  remove_from_denylist --admin "$ISSUER_ADDRESS" --address "$ALICE"
```

### 4. Blocked: jurisdiction

Change Alice's jurisdiction to a code outside the configured list, then retry.
It should fail with `DeniedByJurisdiction` (error 5):

```sh
stellar contract invoke --id "$JURISDICTION" --source "$ISSUER" --network "$NETWORK" -- \
  set_jurisdiction \
  --issuer "$ISSUER_ADDRESS" \
  --address "$ALICE" \
  --code '"GB"'

stellar contract invoke --id "$CONSUMER" --source jurisdiction-alice --network "$NETWORK" -- \
  transfer --from "$ALICE" --to "$BOB" --amount 10
# → Error::DeniedByJurisdiction (5)

# Restore Alice to an allowed jurisdiction.
stellar contract invoke --id "$JURISDICTION" --source "$ISSUER" --network "$NETWORK" -- \
  set_jurisdiction \
  --issuer "$ISSUER_ADDRESS" \
  --address "$ALICE" \
  --code '"US"'
```

## Keeping this doc fresh

1. After a testnet reset (or when invokes fail because a contract is missing),
   rebuild and redeploy the three contracts.
2. Confirm the deploy scripts updated the root `README.md` testnet table and
   `testnet-ids.json` with the newly deployed contract IDs, then refresh the
   scratchpad above with those IDs and the account addresses.
3. Reinitialize each contract and repeat the walkthrough to confirm the
   documented calls still match the example.