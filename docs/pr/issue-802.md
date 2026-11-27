# Index and list pending multisig proposals (#802)

## Summary

Signers could only fetch a proposal if they already knew its ID. An open-proposal index now supports paginated listing
and is pruned on execute, cancel and expiry.

## What changed

- **`contracts/multisig-admin/src/lib.rs`**
  - New `DataKey::OpenProposals` plus `open_proposals()` / `remove_open_proposal()` helpers.
  - `propose` appends the new ID; `execute` removes it.
  - New `cancel_proposal(proposal_id, signer)` (signer-authenticated) removes the proposal and its index entry and emits
    `prop_cancel`.
  - New `list_proposals(start, limit)` returns a bounded page; expired entries are deleted from storage and pruned from
    the index during the listing, so expiry needs no separate sweep transaction.
- **`contracts/multisig-admin/src/test.rs`** — tests for listing, pagination, pruning on cancel/execute and pruning on
  expiry.

## Acceptance criteria addressed

- An index of open proposals exists and is pruned on execute/cancel/expiry.
- `list_proposals(start, limit)` lets signers discover pending proposals.

## Validation

`cargo check -p multisig-admin` succeeds. The test target has five pre-existing `ContractEvents` API errors in older
tests (lines 595–667), unrelated to this change; the new tests are at the end of the file.

closes #802
