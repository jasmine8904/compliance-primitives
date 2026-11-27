# Add automatic pause expiry to compliance-pausable (#804)

## Summary

Emergency pauses were permanent until an explicit `unpause`, so short incident pauses were easy to forget. `pause_until`
now stores an expiry and `is_paused` resumes automatically when the ledger clock reaches it.

## What changed

- **`contracts/pausable/src/lib.rs`**
  - New `DataKey::PauseUntil`, `pause_until(env, until)` and `pause_until_at(env)` helpers.
  - `is_paused` treats a pause as inactive once `now >= until`; `paused_since` and `pause_reason` likewise return `None`
    after expiry so stale audit metadata is not presented as current.
  - `unpause` clears the expiry alongside the flag/timestamp/reason.
  - A `pause_until` at or before the current timestamp is a no-op.
- **`contracts/pausable/src/test.rs`** — harness entry points and three tests: boundary expiry (still paused one second
  before, inactive exactly at `until`), past-timestamp no-op, and `unpause` clearing the expiry.

## Acceptance criteria addressed

- `pause_until(ledger)` makes `is_paused` false after the deadline without an explicit unpause.
- Tests cover the boundary and no-op cases.

## Validation

`cargo test -p compliance-pausable --lib` → 22 passed. A pre-existing integration-test target has unrelated compile
errors (`Symbol::new` signature drift) and was not touched.

closes #804
