# tokenized-fund

Reference tokenized fund share composing three compliance primitives in one
`transfer` path:

| Order | Check | Primitive | Failure |
| --- | --- | --- | --- |
| 1 | Circuit breaker not tripped | [`circuit-breaker`](../../contracts/circuit-breaker).`is_frozen` | `Error::CircuitBreakerFrozen` |
| 2 | Both parties allowlisted | [`allowlist-token`](../../contracts/allowlist-token).`is_allowed` | `Error::NotAllowlisted` |
| 3 | Both parties in permitted jurisdictions | [`jurisdiction-flag`](../../contracts/jurisdiction-flag).`is_permitted_jurisdiction` | `Error::JurisdictionNotPermitted` |

Checks are fail-fast, so an emergency stop is evaluated before any per-address
lookup. `mint` requires the issuer's authorization.

Unlike [`rwa-token`](../rwa-token), this example has no denylist gate.

## Build

```sh
cargo build -p tokenized-fund --target wasm32v1-none --release
```

## Caveats

- Issuance is demo-only: there is no supply cap, subscription, redemption or NAV accounting.
- The token keeps its own balances and gates only its own `transfer`. It does not
  wrap an unrestricted asset; a production wrapper must also block direct transfers
  of the underlying asset.
