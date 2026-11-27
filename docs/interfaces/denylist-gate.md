# denylist-gate

A standalone denylist that other contracts can call before allowing a transfer.

| Method | Purpose |
| --- | --- |
| `initialize(admin)` | Configure the administrator. |
| `get_admin() -> Address` | Read the configured administrator. |
| `add_to_denylist(caller, address)` | Deny an address. Authorized by the admin/compliance officer, or — when a signer set is installed — by threshold signer approvals. |
| `remove_from_denylist(caller, address)` | Clear an address. Authorized by the admin/compliance officer, or — when a signer set is installed — by threshold signer approvals. |
| `check(address) -> bool` | Return whether the address is clear to transact. |
| `initialize_multisig(admin, signers, threshold)` | Install a signer set with the given signers and approval threshold. |
| `add_signer(caller, signer)` | Add a signer to the installed signer set. |
| `remove_signer(caller, signer)` | Remove a signer from the installed signer set. |
| `get_signers() -> Vec<Address>` | Read the current signer set. |
| `get_threshold() -> u32` | Read the required approval threshold. |
| `is_signer(address) -> bool` | Return whether the address is a member of the signer set. |

## Authorization

Denylist mutations (`add_to_denylist` / `remove_from_denylist`) are gated by `require_compliance_authority`, which permits the configured admin or compliance officer.

When a signer set has been installed via `initialize_multisig`, mutations additionally require threshold signer approvals: the caller must be a member of the signer set (verified via `verify_caller_is_signer`) and the required number of signer approvals (the configured threshold) must be collected before the mutation is applied. In this mode the signer set governs who can mutate the denylist; the admin/compliance-officer path remains available for administrative recovery.