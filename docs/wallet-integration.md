# Wallet Integration Guide

This guide describes how wallets integrate with the compliance contracts before building a transfer, and how to surface compliance errors to users.

All samples use the generated TypeScript bindings (produced by `stellar contract bindings typescript`) together with `@stellar/stellar-sdk`.

## Overview

Before a wallet builds and submits a transfer, it should perform a set of read-only view calls to determine whether the transfer is permitted and, if not, why. These calls are cheap, do not modify state, and can be batched or cached briefly.

The checks are:

| Contract | View | Purpose |
| --- | --- | --- |
| `circuit-breaker` | `is_frozen` | Global pause / freeze check |
| `denylist-gate` | `check` | Denylist check for each party (sender and recipient) |
| `jurisdiction-flag` | `is_permitted_jurisdiction` | Jurisdiction eligibility for each party |
| `policy-engine` | `evaluate_verbose` | Full policy evaluation with detailed reasons |
| `compliance-aggregator` | `evaluate` | Aggregate compliance decision |

## Setup

Install the SDK and the generated bindings:

```bash
npm install @stellar/stellar-sdk
npm install ./bindings/circuit-breaker ./bindings/denylist-gate \
  ./bindings/jurisdiction-flag ./bindings/policy-engine \
  ./bindings/compliance-aggregator
```

Create a shared `SorobanRpc.Server` and the contract clients:

```ts
import { Contract, SorobanRpc, TransactionBuilder, Networks, Account } from "@stellar/stellar-sdk";
import { Client as CircuitBreakerClient } from "circuit-breaker";
import { Client as DenylistGateClient } from "denylist-gate";
import { Client as JurisdictionFlagClient } from "jurisdiction-flag";
import { Client as PolicyEngineClient } from "policy-engine";
import { Client as ComplianceAggregatorClient } from "compliance-aggregator";

const RPC_URL = "https://soroban-testnet.stellar.org";
const NETWORK_PASSPHRASE = Networks.TESTNET;

const server = new SorobanRpc.Server(RPC_URL);

// Contract IDs are configured per deployment.
const circuitBreaker = new CircuitBreakerClient({
  contractId: CIRCUIT_BREAKER_ID,
  networkPassphrase: NETWORK_PASSPHRASE,
  rpcUrl: RPC_URL,
});
const denylistGate = new DenylistGateClient({
  contractId: DENYLIST_GATE_ID,
  networkPassphrase: NETWORK_PASSPHRASE,
  rpcUrl: RPC_URL,
});
const jurisdictionFlag = new JurisdictionFlagClient({
  contractId: JURISDICTION_FLAG_ID,
  networkPassphrase: NETWORK_PASSPHRASE,
  rpcUrl: RPC_URL,
});
const policyEngine = new PolicyEngineClient({
  contractId: POLICY_ENGINE_ID,
  networkPassphrase: NETWORK_PASSPHRASE,
  rpcUrl: RPC_URL,
});
const complianceAggregator = new ComplianceAggregatorClient({
  contractId: COMPLIANCE_AGGREGATOR_ID,
  networkPassphrase: NETWORK_PASSPHRASE,
  rpcUrl: RPC_URL,
});
```

## Pre-transfer checks

Run the checks in the order below. Short-circuit on the first failure so the user sees the most specific reason.

### 1. Circuit breaker

```ts
const frozen = await circuitBreaker.is_frozen();
if (frozen) {
  throw new ComplianceError("Frozen", "Transfers are temporarily disabled.");
}
```

### 2. Denylist gate (both parties)

```ts
const senderDenied = await denylistGate.check({ address: sender });
if (senderDenied) {
  throw new ComplianceError("Denied", "The sender address is not permitted to transact.");
}

const recipientDenied = await denylistGate.check({ address: recipient });
if (recipientDenied) {
  throw new ComplianceError("Denied", "The recipient address is not permitted to transact.");
}
```

### 3. Jurisdiction flag (both parties)

```ts
const senderJurisdictionOk = await jurisdictionFlag.is_permitted_jurisdiction({
  address: sender,
});
if (!senderJurisdictionOk) {
  throw new ComplianceError(
    "JurisdictionNotPermitted",
    "The sender's jurisdiction is not permitted.",
  );
}

const recipientJurisdictionOk = await jurisdictionFlag.is_permitted_jurisdiction({
  address: recipient,
});
if (!recipientJurisdictionOk) {
  throw new ComplianceError(
    "JurisdictionNotPermitted",
    "The recipient's jurisdiction is not permitted.",
  );
}
```

### 4. Policy engine (verbose)

`evaluate_verbose` returns a structured result including the decision and the reasons behind it. Use it to build a precise message.

```ts
const policyResult = await policyEngine.evaluate_verbose({
  from: sender,
  to: recipient,
  amount,
});

if (!policyResult.allowed) {
  throw new ComplianceError(
    policyResult.reason ?? "Denied",
    policyResult.message ?? "The transfer does not satisfy the compliance policy.",
  );
}
```

### 5. Compliance aggregator

The aggregator combines the individual checks into a single decision. It is the authoritative pre-flight check.

```ts
const decision = await complianceAggregator.evaluate({
  from: sender,
  to: recipient,
  amount,
});

if (!decision.allowed) {
  throw new ComplianceError(
    decision.reason ?? "Denied",
    decision.message ?? "The transfer was rejected by the compliance aggregator.",
  );
}
```

### Putting it together

```ts
export async function preflightTransfer(
  sender: string,
  recipient: string,
  amount: bigint,
): Promise<void> {
  if (await circuitBreaker.is_frozen()) {
    throw new ComplianceError("Frozen", "Transfers are temporarily disabled.");
  }

  if (await denylistGate.check({ address: sender })) {
    throw new ComplianceError("Denied", "The sender address is not permitted to transact.");
  }
  if (await denylistGate.check({ address: recipient })) {
    throw new ComplianceError("Denied", "The recipient address is not permitted to transact.");
  }

  if (!(await jurisdictionFlag.is_permitted_jurisdiction({ address: sender }))) {
    throw new ComplianceError(
      "JurisdictionNotPermitted",
      "The sender's jurisdiction is not permitted.",
    );
  }
  if (!(await jurisdictionFlag.is_permitted_jurisdiction({ address: recipient }))) {
    throw new ComplianceError(
      "JurisdictionNotPermitted",
      "The recipient's jurisdiction is not permitted.",
    );
  }

  const policyResult = await policyEngine.evaluate_verbose({ from: sender, to: recipient, amount });
  if (!policyResult.allowed) {
    throw new ComplianceError(
      policyResult.reason ?? "Denied",
      policyResult.message ?? "The transfer does not satisfy the compliance policy.",
    );
  }

  const decision = await complianceAggregator.evaluate({ from: sender, to: recipient, amount });
  if (!decision.allowed) {
    throw new ComplianceError(
      decision.reason ?? "Denied",
      decision.message ?? "The transfer was rejected by the compliance aggregator.",
    );
  }
}
```

## Handling compliance errors

If a transfer is submitted and the contract rejects it, the transaction result will contain a `ContractError`. Map the variant to a user-facing message using the table below.

```ts
export class ComplianceError extends Error {
  constructor(public readonly code: string, message: string) {
    super(message);
    this.name = "ComplianceError";
  }
}

const COMPLIANCE_MESSAGES: Record<string, string> = {
  NotAllowlisted: "This address is not on the allowlist and cannot transact.",
  Denied: "This address is not permitted to transact.",
  JurisdictionNotPermitted: "Transfers involving this jurisdiction are not permitted.",
  Frozen: "Transfers are temporarily disabled. Please try again later.",
  ContractPaused: "Compliance checks are paused. Please try again later.",
};

export function toUserMessage(code: string): string {
  return COMPLIANCE_MESSAGES[code] ?? "The transfer could not be completed due to a compliance restriction.";
}
```

### Error reference

| `ContractError` variant | Meaning | Suggested user-facing message |
| --- | --- | --- |
| `NotAllowlisted` | Address is not on the allowlist | "This address is not on the allowlist and cannot transact." |
| `Denied` | Address is on the denylist | "This address is not permitted to transact." |
| `JurisdictionNotPermitted` | Jurisdiction is not permitted | "Transfers involving this jurisdiction are not permitted." |
| `Frozen` | Circuit breaker is engaged | "Transfers are temporarily disabled. Please try again later." |
| `ContractPaused` | Contract is paused | "Compliance checks are paused. Please try again later." |
| `PolicyViolation` | Policy engine rejected the transfer | "The transfer does not satisfy the compliance policy." |
| `InsufficientAllowance` | Transfer exceeds an allowance | "The transfer exceeds the permitted allowance." |
| `Unauthorized` | Caller is not authorized | "You are not authorized to perform this action." |

## Notes

- View calls are read-only and can be cached for a short period (for example, a few seconds) to reduce RPC load. Do not cache across user actions that change compliance state.
- Always run the pre-flight checks immediately before building the transfer transaction, so the decision reflects current state.
- The compliance aggregator is the authoritative check; the individual checks are useful for producing specific error messages.