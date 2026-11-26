# compliance-aggregator

A composition point for evaluating multiple compliance primitives as one
AND-composed decision.

| Method | Purpose |
| --- | --- |
| `initialize(admin, denylist_gate, jurisdiction_flag, circuit_breaker)` | Configure the initial checks and administrator. |
| `set_allowlist(admin, address)` / `clear_allowlist(admin)` | Register or unregister the `allowlist-token` check. |
| `check_address(address, allowed_jurisdictions)` | Backwards-compatible overall boolean and per-check pass/fail results. |
| `check_address_with_failures(address, allowed_jurisdictions)` | Return the overall boolean and a `Vec<CheckFailure>` with the source and code for each failed check. |
| `check_all(addresses, allowed_jurisdictions)` | Backwards-compatible batch results with overall and per-check booleans. |
| `check_all_with_failures(addresses, allowed_jurisdictions)` | Return one detailed per-address result, including structured failure reasons. |
| `batch_check(addresses, allowed_jurisdictions)` | Return only the overall booleans. |
| `get_checks_summary()` | Return `(denylist, jurisdiction, circuit_breaker, allowlist)` configured flags. |
| `get_config()` | Inspect the configured check contract addresses. |

`CheckFailure.source` identifies the check (`Denylist`, `Jurisdiction`,
`CircuitBreaker`, or `Allowlist`). `CheckFailure.code` is a stable
`CheckFailureCode`: `Denylisted`, `JurisdictionNotPermitted`,
`CircuitBreakerFrozen`, or `NotAllowlisted`.
