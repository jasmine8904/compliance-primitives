# Upgrade Safety Checklist

Use this checklist for any PR that changes a contract with an `upgrade()`
entrypoint (or that adds one) and touches stored data. Policy details and the
definition of "breaking" live in [`STORAGE_VERSIONING.md`](../STORAGE_VERSIONING.md);
this page is the step-by-step gate. Upgrades swap code only. Storage is never
migrated or rolled back automatically, so each box must be checked before the
upgrade ships.

## 1. Storage key changes

- [ ] Every `DataKey` variant (and any other `#[contracttype]` used as a storage
      key) is listed in the PR description as added, renamed, removed, or unchanged.
- [ ] No shipped variant was renamed. Renaming changes its storage key, and old
      data becomes unreachable without any error.
- [ ] No shipped variant was removed unless a migration clears or transforms its
      entries first.
- [ ] No existing key's value changed shape or meaning in place. Use a new key.
- [ ] Each change is classified under
      [Change categories](../STORAGE_VERSIONING.md#change-categories). If unsure,
      treat it as breaking.

## 2. Schema version bump

- [ ] If the change is breaking, bump the contract's `SCHEMA_VERSION` constant.
      Contracts that expose `schema_version()` (for example `allowlist-token` and
      `denylist-gate`) return this value on-chain, so operators can confirm which
      schema a deployment is running.
- [ ] Contracts without `schema_version()` document the storage version in their
      `UPGRADE.md` instead.
- [ ] Non-breaking changes (new keys only, see the policy) do not bump the schema
      version.

## 3. Migration function

- [ ] A breaking change ships with a `migrate_*` function, or with read code that
      accepts both the old and new shapes until migration has finished.
- [ ] The migration is admin-authorized and runs as an explicit call after
      `upgrade()`. It never runs implicitly inside `upgrade()`.
- [ ] The migration is idempotent or guarded, so a second call cannot corrupt
      data that is already migrated.
- [ ] Unit tests cover the migration on state written by the old version.
- [ ] `UPGRADE.md` lists the exact call order: upload the wasm, `upgrade()`,
      then migration, then verification.

## 4. Testnet rehearsal

- [ ] The full sequence (deploy the old version, write representative data,
      upgrade, migrate, read back) has been run on testnet. Record the contract
      IDs and wasm hashes in the PR.
- [ ] Reads of pre-upgrade data return the same values after the upgrade.
- [ ] The rehearsal used the same wasm that will be deployed to mainnet. Its hash
      matches the reproducible build.
- [ ] Rollback is understood. There is no rollback, so the fix path is a second
      `upgrade()` with a corrected wasm. Confirm the team can run that quickly.

## 5. Sign-off

- [ ] The PR is labelled `complexity: high` if it is breaking (see
      [`CONTRIBUTING.md`](../CONTRIBUTING.md#complexity-labels-and-expected-pr-size)).
- [ ] An admin-authorized party (issuer or multisig) has reviewed the steps in
      `UPGRADE.md` before the upgrade is scheduled.
- [ ] Monitoring is in place for the `UpgradePerformed` event and for failures in
      the migration call.
