# Store a description hash with each multisig proposal (#803)

## Summary

Proposal payloads are opaque bytes, so signers could not tell what they were approving. An optional `description_hash`
(sha256 of a human-readable proposal document) is now stored with each proposal and surfaced through `get_proposal` and
the creation event.

## What changed

- **`contracts/multisig-admin/src/lib.rs`**
  - `Proposal` gains `description_hash: Option<BytesN<32>>`.
  - New `propose_with_description(payload, expiry, description_hash)`; the existing `propose` delegates with `None`, so
    current callers are unchanged.
  - `ProposalCreated` includes `description_hash`.
  - `get_proposal` now returns the hash as a fourth tuple element.
- **`contracts/multisig-admin/src/test.rs`** — existing `get_proposal` destructuring updated; new test verifies a
  supplied hash is returned and omitted for legacy proposals.

## Acceptance criteria addressed

- Optional `description_hash: BytesN<32>` is stored.
- It is exposed in `get_proposal` and the `ProposalCreated` event.

## Validation

`cargo check -p multisig-admin` succeeds; the test target's pre-existing `ContractEvents` API errors remain unchanged.

closes #803
