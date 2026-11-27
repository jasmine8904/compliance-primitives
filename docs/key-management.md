# Key Management for Admins, Officers, and Guardians

This guide covers custody recommendations for every privileged role defined
across the compliance-primitives contracts.  It is written for two audiences:
small issuers (a single team, one to five people) and large issuers (an
institution with dedicated operations staff and formal approval workflows).

---

## Roles at a glance

| Role               | Contracts that use it                                   | What it can do                                                                                 |
|--------------------|---------------------------------------------------------|-----------------------------------------------------------------------------------------------|
| **Admin**          | All contracts                                           | Full control: initialize, configure, upgrade, transfer admin role, pause/unpause, etc.         |
| **Compliance officer** | `allowlist-token`, `denylist-gate`, `jurisdiction-flag` | Revoke allowlist entries, deny addresses, or revoke jurisdiction codes — no other admin power |
| **Guardian**       | `circuit-breaker`                                       | Freeze the breaker (halt all flows); cannot unfreeze, upgrade, or transfer admin               |
| **Issuer**         | `jurisdiction-flag`                                     | Same as admin for that contract; set/revoke jurisdiction codes, propose upgrades               |
| **Multisig signers** | `multisig-admin`                                      | Sign proposals; individually cannot execute; collectively execute when threshold is met        |

---

## Admin key

The admin key is the highest-privilege key for a contract.  Compromise or
loss locks out the deployment permanently (or until an on-chain upgrade
replaces the admin logic).

### Small issuers

- Use a **hardware wallet** (Ledger, Trezor) for the admin key.  Never
  import the mnemonic into any internet-connected machine.
- Keep the hardware wallet in a physically secure location (locked office,
  safe).  Store a backup mnemonic in a separate physical location (sealed
  envelope, safety-deposit box) accessible only to two or more trusted
  people together.
- Use the **propose/accept two-step** admin transfer that all contracts
  support.  This means admin rotation requires two separate signed
  transactions; a single compromised key cannot silently transfer control.

### Large issuers

- Use a **multisig-admin** contract as the admin address for each primitive.
  Set a threshold of M-of-N (e.g. 3-of-5) with keys held by distinct
  individuals across organizational boundaries (legal, ops, security).
- Distribute hardware wallets to each signer.  Never allow any one signer
  to hold more than one key.
- Apply the same threshold and hardware-wallet requirement to the
  `multisig-admin` contract's own admin role (the address that can add or
  remove signers).  Consider making that meta-admin a different multisig with
  a higher threshold (e.g. 4-of-5).
- Record key holders and backup procedures in your organization's runbook.
  Rotate at least annually or on any signer departure.

### Key rotation procedure

1. Generate the new key on a hardware wallet (never software).
2. Call `propose_admin(new_admin)` from the current admin key.
3. Call `accept_admin()` from the new admin key to confirm acceptance.
4. Verify the on-chain admin matches the new address before decommissioning
   the old key.
5. Update your runbook and notify all relevant stakeholders.

---

## Compliance officer key

The compliance officer has narrower power than the admin but is called more
frequently (day-to-day allowlist/denylist maintenance).

### Small issuers

- A **dedicated software wallet** stored on a hardened workstation is
  acceptable, provided the workstation has full-disk encryption, MFA login,
  and no browser extensions.
- Keep the compliance officer key separate from admin keys — different
  device, different mnemonic.
- Rotate the key when any person with access leaves the team.

### Large issuers

- Use a **hardware wallet** for the compliance officer key.
- If compliance operations are high-volume, consider a dedicated **HSM**
  (hardware security module) integrated with your compliance tooling via the
  Stellar SDK.
- Restrict the compliance workstation to compliance tooling only; no general
  internet browsing.
- Audit all compliance officer transactions via the `audit-log` contract.
  Route `audit-log` events to your SIEM or compliance platform.

### Officer rotation procedure

1. Have the current admin call `set_compliance_officer(new_officer)` to
   replace the officer.
2. If the officer must be removed immediately (e.g. incident response), have
   the admin call `remove_compliance_officer()`.
3. Document the change in your incident log.

---

## Guardian key

Guardians can freeze the `circuit-breaker` without admin involvement.  This
is intentional: guardians are meant to respond to incidents faster than a
multisig quorum can assemble.  Unfreezing always requires the admin, so the
risk of a rogue guardian is a temporary halt, not a permanent takeover.

### Small issuers

- Assign one or two guardians: one for a primary on-call engineer, one for
  a backup.
- A **software wallet on a mobile device with biometric authentication** is
  acceptable for the guardian role, given its limited blast radius.
- Treat guardian keys as "break-glass" credentials: document them, audit
  their use, and rotate on personnel changes.

### Large issuers

- Assign three to five guardians across time zones so freeze coverage is
  continuous.
- Use hardware wallets or HSM-backed keys for each guardian.
- Integrate guardian freeze capability into your incident-response runbook
  with explicit escalation criteria (e.g. "freeze if on-chain volume exceeds
  2× 7-day average within one hour").
- Log and alert on every `freeze` and `unfreeze` event.

### Guardian rotation procedure

1. Admin calls `add_guardian(new_guardian)` to add the replacement.
2. Admin calls `remove_guardian(old_guardian)` to remove the departing key.
3. These are independent operations; always add before removing to ensure
   continuous coverage.

---

## Multisig-admin signers

`multisig-admin` turns the admin role itself into an M-of-N multisig.
Key management for signers follows the same principles as admin keys.

### Threshold selection

| Team size | Recommended threshold | Notes                                              |
|-----------|-----------------------|----------------------------------------------------|
| 3 signers | 2-of-3                | One absent signer cannot block operations          |
| 5 signers | 3-of-5                | Tolerates two simultaneous unavailable signers     |
| 7 signers | 4-of-7                | Suitable for institutions with global ops teams    |

Avoid 1-of-N (defeats the purpose) and N-of-N (a single lost key locks the
contract permanently).

### Signer rotation

1. Admin (or the existing quorum) calls the appropriate `replace_signer` or
   `add_signer`/`remove_signer` function, reaching the threshold.
2. Remove the old signer's key from hardware wallets and revoke access to
   any shared tooling.
3. Update your signer roster documentation.

### Key storage

- Each signer must hold their key on a hardware wallet.
- No two signers should share a device or backup location.
- Store mnemonics in tamper-evident sealed envelopes in separate physical
  locations (not all in the same building or city for large issuers).

---

## General recommendations

### Never do these things

- Import a production admin or officer mnemonic into a browser extension
  wallet or any internet-connected hot wallet.
- Store mnemonics in a password manager, cloud storage, or email.
- Reuse keys across contracts or networks (testnet vs. mainnet keys must
  be fully separate).
- Skip the propose/accept two-step for admin rotation — the confirmation
  step prevents one-sided admin hijacking.

### Monitoring and alerting

Set up monitoring on the following events for every deployed contract:

| Event              | Severity | Action                                      |
|--------------------|----------|---------------------------------------------|
| `AdminSet`         | High     | Verify it matches an authorized rotation    |
| `AdminTransferProposed` | High | Notify all admin key holders           |
| `Freeze`           | High     | Page on-call; begin incident response       |
| `Unfreeze`         | High     | Confirm it was authorized; log decision     |
| `ComplianceOfficerSet` | Medium | Verify it matches an authorized change  |
| `UpgradeProposed`  | High     | Review WASM hash; confirm with security team|

Use the `audit-log` contract to retain a tamper-evident on-chain record of
compliance events for regulatory purposes.

### Key loss recovery

- If the admin key is lost and no propose/accept is pending, the contract
  can only be recovered by deploying a new instance (all state is lost).
  This underscores the importance of hardware wallet backups and multisig.
- If a compliance officer or guardian key is lost, the admin can assign a
  replacement immediately.
- For multisig-admin, losing fewer than `threshold` keys is recoverable.
  Losing `N - threshold + 1` or more keys locks the contract permanently.
  Plan your threshold and backup strategy around this limit.

---

## Network separation

Always use separate key material for testnet and mainnet.  A testnet key
compromise is harmless; a shared key compromise affects mainnet.  Label keys
clearly and never reuse mnemonics across networks.
