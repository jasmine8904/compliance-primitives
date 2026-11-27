# pausable

A shared helper crate used by contracts that need an emergency pause flag. It is a library component, not a standalone deployable contract.

| Method | Purpose |
| --- | --- |
| `is_paused()` | Read the contract-local pause state. |
| `pause(admin)` | Set the pause flag after authorization by the host contract. May be authorized by the admin or the configured pauser. |
| `unpause(admin)` | Clear the pause flag after authorization by the host contract. Admin-only; the pauser cannot unpause. |
| `set_pauser(admin, pauser)` | Configure the pauser role. Admin-only. |
| `get_pauser()` | Read the currently configured pauser, if any. |
| `require_pause_authorized(caller)` | Return the host contract's authorization error unless `caller` is the admin or the configured pauser. Used by `pause`. |
| `require_not_paused()` | Return the host contract’s pause error when the flag is set. |

## Recommended key setup

Pausing is an incident-response action and should not require the (typically cold) admin key. Configure a separate hot "pauser" key that can only pause, never unpause:

- **Admin key**: cold storage / multisig. Used for `unpause`, `set_pauser`, and other privileged configuration. Rarely online.
- **Pauser key**: a hot key held by on-call operators. Used only for `pause`. Compromise of this key can halt the contract but cannot resume it or change configuration, so recovery remains under admin control.

To rotate or revoke the pauser, the admin calls `set_pauser` with the new key (or clears it). Because `unpause` is admin-only, a compromised pauser cannot undo an incident response.