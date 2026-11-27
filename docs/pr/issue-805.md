# Support feature-scoped pauses (#805)

## Summary

The shared pausable crate only offered a global stop, so a contract could not pause minting while leaving redemptions
working. Scoped pauses now coexist with the global flag, which overrides all scopes.

## What changed

- **`contracts/pausable/src/lib.rs`**
  - New `PauseScope` enum (`Mint`, `Redeem`, `Transfer`, `Admin`) and `DataKey::PausedScopes`.
  - `pause_scope(scope)` / `unpause_scope(scope)` manage individual feature pauses; duplicate pauses are ignored.
  - `is_paused_scope(scope)`, `require_not_paused_scope(scope)` and `require_not_paused_scope_or(scope, err)` return the
    global pause OR the scope's state, so a global stop gates every feature.
  - `unpause` also clears all scoped pauses.
- **`contracts/pausable/src/test.rs`** — harness entry points and three tests: scope isolation, global override and
  scoped/global resume.

## Acceptance criteria addressed

- `pause_scope` / `require_not_paused_scope` exist and gate only their scope.
- Global pause overrides every scope.
- Tests cover isolation, override and cleanup.

## Validation

`cargo test -p compliance-pausable --lib` → 22 passed.

closes #805
