# Denylist-Gate Authorization Model

`denylist-gate` supports direct single-admin governance and composition with the
standalone `multisig-admin` contract. The compliance officer is a delegated
operational role, not a governance role.

## Authority matrix

| Operation | Single admin | Officer configured | Multisig-backed admin | Paused |
| --- | --- | --- | --- | --- |
| `check`, `is_denylisted`, and views | Public read | Public read | Public read | Works |
| `add_to_denylist`, `remove_from_denylist`, `remove_multiple_from_denylist` | Admin | Admin or officer | Threshold of distinct signer approvals | Rejected |
| `set_compliance_officer`, `revoke_compliance_officer` | Admin only | Admin only | Multisig authorization of admin | Available to admin |
| `set_audit_log` | Admin only | Admin only | Multisig authorization of admin | Available to admin |
| `pause`, `unpause` | Admin only | Admin only | Multisig authorization of admin | Pause blocks denylist mutations |
| `initialize_multisig` | Current admin once | Current admin once | Not applicable after conversion | Available to admin |

The officer cannot assign or revoke itself, change the audit-log destination,
change signer governance, or pause the contract. It can add or remove denylist
entries, individually or in batch (`remove_multiple_from_denylist`, up to 45
addresses per call), while the contract is not paused. Single and batch removal
use the same authority check, so a batch call never fails for an officer where
the equivalent sequence of single calls would succeed.

## Denylist mutation authority in multisig mode

The `SignerSet` is the authoritative gate for denylist mutations. When a
`SignerSet` is installed (via `initialize_multisig`), `add_to_denylist`,
`remove_from_denylist`, and `remove_multiple_from_denylist` require the
configured threshold of distinct signer approvals instead of the single-admin
or officer check. Each approval is validated per signer through
`verify_caller_is_signer`, and the operation proceeds only once the number of
distinct valid signers meets or exceeds the stored threshold.

In multisig mode the compliance officer role is bypassed: an officer cannot
mutate the denylist on its own, because the signer-set threshold is the sole
authorization path for these operations. The officer remains usable for
read-only compliance checks and for any operation that does not mutate the
denylist.

### Mode transitions and fallback

- **Single-admin to multisig.** `initialize_multisig` installs the `SignerSet`
  and threshold. From that point on, denylist mutations are gated by the signer
  set; the previous single-admin/officer path no longer authorizes them.
- **Multisig to single-admin.** If the `SignerSet` is removed, the contract
  falls back to the single-admin/officer authorization path described above.
- **No signer set installed.** When no `SignerSet` exists, the contract behaves
  as a single-admin deployment and `require_compliance_authority` (admin or
  officer) governs denylist mutations.

Signer governance changes (`initialize_multisig`, `add_signer`,
`remove_signer`) remain admin-only and are not gated by the signer threshold
itself, so a misconfigured signer set can always be corrected by the admin.

## Multisig composition

`multisig-admin` is used as the `admin` address during deployment or migration.
When the gate calls `admin.require_auth()`, Soroban invokes the multisig
contract's M-of-N authorization hook. Signer membership and threshold
validation therefore belong to `multisig-admin`; the gate does not duplicate
signature counting. The officer role is additive and must not be treated as a
bypass for governance changes.

Denylist mutations are the exception to the "delegate to `multisig-admin`"
pattern: when a `SignerSet` is installed in the gate itself, the gate enforces
the threshold directly through `verify_caller_is_signer` rather than relying on
the admin address's authorization hook. This keeps denylist authority explicit
and auditable within the gate, and means the officer role is bypassed for these
operations in multisig mode.

## Audit log and storage lifetime

Audit logging is opt-in. After `set_audit_log`, each successful individual add
or remove records an entry whose source is the gate contract. Without a
configured destination, no cross-contract call is made.

`Admin`, `Paused`, `ComplianceOfficer`, `SignerSet`, and `AuditLog` are instance
storage keys. Soroban bumps the instance entry TTL when the contract is
invoked, so explicit per-operation TTL extension is unnecessary for the
expected continuously operated deployment. Denylist entries remain persistent
and receive an explicit long TTL extension because their reads can occur
independently of instance-storage activity.

Pause affects mutations only. Read-only compliance checks continue to work
while paused, allowing consumers to fail closed or apply their own pause policy.
