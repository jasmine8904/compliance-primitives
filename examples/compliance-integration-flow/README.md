# compliance-integration-flow

Integration example showing how a jurisdiction policy, an emergency circuit
breaker, multisig administration, and an audit log fit into one transfer
decision. Unlike [`rwa-compliance-flow`](../rwa-compliance-flow), which
demonstrates the allowlist/denylist/jurisdiction checks together, this example
uses `policy-engine` to evaluate jurisdiction rules, checks the circuit breaker
first, and records each outcome in `audit-log`. It is a contract-composition
example, not a deployable RWA token like [`rwa-token`](../rwa-token).

## Primitives used

- `jurisdiction-flag` — stores jurisdiction codes for addresses.
- `policy-engine` — evaluates whether both transfer parties satisfy the
  configured jurisdiction check.
- `compliance-aggregator` — provides an independent per-address check in the
  integration tests.
- `multisig-admin` — administers the circuit breaker.
- `circuit-breaker` — halts the transfer flow before policy evaluation.
- `audit-log` — records allowed, blocked, and halted outcomes.
- `pausable` — used internally by `jurisdiction-flag`.

The flow is intentionally scoped to these components; it does not compose
`allowlist-token` or `denylist-gate`.

## Run

From the repository root:

```sh
cargo test -p compliance-integration-flow
cargo build -p compliance-integration-flow --target wasm32v1-none --release
```

## Expected output

The test command should finish successfully with three passing integration
tests. They cover an allowed transfer, a transfer rejected by the jurisdiction
policy, and a circuit-breaker halt; each outcome is checked in the audit log.
The build command should produce the example contract's Wasm artifact under
`target/wasm32v1-none/release/`.

## Wallet integration

Wallets integrating with this flow should follow the
[wallet integration guide](../../docs/wallet-integration.md) for the general
pre-transfer sequence and the generated bindings. The mapping for this example
is:

1. **Circuit breaker first.** Call the `circuit-breaker` view (for example
   `is_halted`) before anything else. If it reports halted, surface the
   compliance error to the user and do not build a transfer; the audit log
   will record the halted outcome.
2. **Policy evaluation.** Call the `policy-engine` view (for example
   `evaluate`) with the sender and recipient addresses. A failing result maps
   to the compliance error returned by the engine; show it verbatim so the
   user sees which jurisdiction rule was violated.
3. **Build and submit.** Only when both views pass should the wallet build the
   transfer. After submission, the `audit-log` records the allowed or blocked
   outcome, which can be queried for display in the wallet's activity view.

The code samples in the wallet integration guide use the generated bindings
for these contracts, so the view names and error types match what this example
exposes.
