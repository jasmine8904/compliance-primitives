# examples/js-client

A minimal TypeScript example showing how to call a deployed
`compliance-primitives` contract from off-chain code using
[`@stellar/stellar-sdk`](https://github.com/stellar/js-stellar-sdk).

Every other example in `/examples` is itself a Soroban contract (Rust). This
one is the **off-chain counterpart**: a Node.js script that invokes a deployed
`denylist-gate` contract's `check` function from JavaScript/TypeScript.

## What it demonstrates

- Connecting to a Soroban RPC endpoint (testnet by default)
- Building a read-only transaction that calls `denylist-gate.check(address)`
- Simulating the transaction — no signing or fees required for read-only calls
- Decoding the `bool` return value from XDR to a native JS value

## Prerequisites

- Node.js >= 20
- A deployed `denylist-gate` contract on testnet
  (see [`scripts/deploy-testnet.sh`](../../scripts/deploy-testnet.sh) or
  the root [QUICKSTART.md](../../QUICKSTART.md))

## Quick start

```sh
# 1. Install dependencies
cd examples/js-client
npm install

# 2. Configure
cp .env.example .env
# Edit .env: set DENYLIST_GATE_CONTRACT_ID and ADDRESS_TO_CHECK

# 3. Run
npm run check
```

Expected output (address not on denylist):

```
RPC endpoint : https://soroban-testnet.stellar.org
Contract     : CXXXXXXX...
Address      : GAAZI4T...

denylist-gate.check(GAAZI4T...) => ✅ allowed
```

If the address is on the denylist:

```
denylist-gate.check(GAAZI4T...) => 🚫 denied
```

## Extending this example

The same pattern works for any read-only function in any of the
compliance-primitives contracts. To call a different function:

1. Change `contract.call("check", ...)` to the function name and arguments
   you need.
2. Adjust the `scValToNative` decode to match the return type.

For `jurisdiction-flag.is_permitted_jurisdiction(address, allowed_codes)` you
would pass two arguments: an `address` ScVal and a `vec<string>` ScVal.
For `circuit-breaker.is_frozen()` you would pass no arguments at all.

## Wallet integration

Wallets should call the read-only views **before** building a transfer, then
surface any compliance error to the user. The full walkthrough lives in
[`docs/wallet-integration.md`](../../docs/wallet-integration.md); the code
below is the condensed version, starting from
[`check.ts`](./check.ts).

### 1. Pre-transfer view calls

Call these views (all read-only, so simulate — no signing or fees) before
constructing a transfer:

| Contract | View | Purpose |
| --- | --- | --- |
| `denylist-gate` | `check(address)` | Is the sender/recipient allowed? |
| `jurisdiction-flag` | `is_permitted_jurisdiction(address, allowed_codes)` | Is the counterparty in an allowed jurisdiction? |
| `circuit-breaker` | `is_frozen()` | Has the contract been paused? |

```ts
import { Contract, nativeToScVal, scValToNative, TransactionBuilder } from "@stellar/stellar-sdk";

async function preTransferChecks(
  server: SorobanRpc.Server,
  source: Account,
  denylistGateId: string,
  jurisdictionFlagId: string,
  circuitBreakerId: string,
  sender: string,
  recipient: string,
  allowedCodes: string[],
) {
  // 1. Circuit breaker — bail out early if the contract is paused.
  const frozen = await simulate(server, source, circuitBreakerId, "is_frozen", []);
  if (frozen) throw new ComplianceError("CONTRACT_FROZEN", "Transfers are paused");

  // 2. Denylist gate — both parties must pass.
  for (const addr of [sender, recipient]) {
    const allowed = await simulate(server, source, denylistGateId, "check", [
      nativeToScVal(addr, { type: "address" }),
    ]);
    if (!allowed) throw new ComplianceError("DENIED", `${addr} is on the denylist`);
  }

  // 3. Jurisdiction flag — recipient must be in an allowed jurisdiction.
  const permitted = await simulate(server, source, jurisdictionFlagId, "is_permitted_jurisdiction", [
    nativeToScVal(recipient, { type: "address" }),
    nativeToScVal(allowedCodes, { type: "vec" }),
  ]);
  if (!permitted) throw new ComplianceError("JURISDICTION", `${recipient} is not permitted`);
}

// Shared helper: build a read-only tx, simulate it, decode the result.
async function simulate(
  server: SorobanRpc.Server,
  source: Account,
  contractId: string,
  fn: string,
  args: xdr.ScVal[],
) {
  const contract = new Contract(contractId);
  const tx = new TransactionBuilder(source, { fee: "100", networkPassphrase: Networks.TESTNET })
    .addOperation(contract.call(fn, ...args))
    .setTimeout(30)
    .build();
  const sim = await server.simulateTransaction(tx);
  if (SorobanRpc.Api.isSimulationError(sim)) throw new Error(sim.error);
  return scValToNative(sim.result!.retval);
}
```

### 2. Handling compliance errors

When a view returns `false` (or the contract traps), do **not** build the
transfer. Map the failure to a user-facing message and a stable error code so
the UI can render it consistently:

```ts
class ComplianceError extends Error {
  constructor(public code: "DENIED" | "JURISDICTION" | "CONTRACT_FROZEN", message: string) {
    super(message);
  }
}

try {
  await preTransferChecks(/* ... */);
  // Safe to build and submit the transfer here.
} catch (err) {
  if (err instanceof ComplianceError) {
    // e.g. show a banner: "Transfer blocked: <message>"
    showComplianceBanner(err.code, err.message);
  } else {
    throw err;
  }
}
```

See [`docs/wallet-integration.md`](../../docs/wallet-integration.md) for the
full guide, including error-code reference and UI guidance.

## Relation to `tools/indexer`

[`tools/indexer`](../../tools/indexer) is a long-running event indexer that
uses the raw JSON-RPC interface directly. This example uses the higher-level
`@stellar/stellar-sdk` client, which is a better starting point when you
need to make individual contract calls rather than stream events.
