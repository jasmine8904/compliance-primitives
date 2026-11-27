# Security Policy

## Scope

These contracts are compliance-critical primitives meant for real issuers.
In scope for a security report:

- Anything that lets a transfer go through despite the sender or recipient
  being denylisted, not allowlisted, or in a disallowed jurisdiction.
- Anything that lets a non-admin/non-issuer bypass `require_auth()` and
  mutate allowlist, denylist, or jurisdiction state.
- Anything that causes incorrect or missing compliance events, in a way
  that would hide a compliance-relevant state change from off-chain
  monitoring.

Out of scope: issues in the example contracts under `/examples` that don't
affect the primitives themselves, and purely cosmetic/documentation issues
(please file those as a regular GitHub issue instead).

## Contract upgradeability

The following contracts expose an upgrade path:

- `allowlist-token` and `denylist-gate`: the admin must propose an upgrade,
  then wait until its configured ledger delay has elapsed before committing
  it. A pending upgrade can be cancelled by the admin during the delay.
- `jurisdiction-flag`: the issuer must call `propose_upgrade`, wait at least
  17,280 ledgers (~1 day), then call `commit_upgrade`. The issuer can cancel a
  pending proposal during the delay.

Soroban keeps the contract ID and its instance and persistent storage when
the Wasm is replaced. This includes the admin or issuer and compliance state
stored under compatible keys. An upgrade does not migrate, reinterpret, or
repair storage: the new code must remain compatible with existing key and
value types, or provide and invoke an explicit migration. In
`allowlist-token`, `denylist-gate`, and `jurisdiction-flag`, the
pending-upgrade record is removed when an upgrade is committed; it is not
carried forward as a pending action.
The previous Wasm is not retained as an automatic rollback target, and a
failed migration or incompatible schema requires a separately deployed
corrective upgrade.

Other contracts in this repository do not currently expose an upgrade
entrypoint. Review a contract's authorization and storage compatibility
before relying on an upgrade path for production deployments.

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Report privately via [GitHub Security Advisories](https://github.com/stellar-compliance-kit/compliance-primitives/security/advisories/new)
for this repository. Include the affected contract(s), a description of
the issue, and steps to reproduce if possible.

## What to expect

Our targets, measured in business days from when we receive a report:

- **Acknowledgement:** within 3 business days.
- **Triage and severity assessment:** within 7 business days. We will tell
  you whether we consider the report valid and its severity.
- **Fix or mitigation:** critical issues within 30 days, high-severity issues
  within 60 days, other valid issues within 90 days. Complex fixes that
  need more time are communicated to you with a revised date.
- **Status updates:** at least every 14 days until the issue is resolved.

We will coordinate disclosure timing with you before publishing anything.
By default, public disclosure happens once a fix is released, or 90 days
after the report if no fix is available, whichever comes first. We will
credit reporters in the advisory unless you ask us not to.
