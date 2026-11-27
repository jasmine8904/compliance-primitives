# circuit-breaker

An emergency control that lets an authorized operator stop and resume protected flows.

| Method | Purpose |
| --- | --- |
| `initialize(admin)` | Configure the administrator. |
| `freeze(caller, reason)` | Freeze immediately; the admin or any configured guardian may call it. `reason` is optional. |
| `unfreeze(admin)` | Resume protected flows; admin-only. Guardians cannot unfreeze. |
| `is_frozen()` | Read the current breaker state. |
| `freeze_history(limit)` | Return up to `limit` of the newest freeze records, ordered oldest to newest. |
| `add_guardian(admin, guardian)` / `remove_guardian(admin, guardian)` | Manage freeze-only guardians. |
| `upgrade(admin, new_wasm_hash)` | Admin-only. Move the contract code to a new WASM hash; storage is preserved. |

Each history record contains `{ by, at, until, reason }`. `at` and `until` are
ledger timestamps; `until` is `None` while the breaker remains frozen and is
filled when the admin unfreezes it. The contract retains the most recent 10
records.
