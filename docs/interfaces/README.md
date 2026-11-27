# Contract interfaces

This page lists the public interfaces exposed by the compliance primitives. The site version is generated from the Markdown files in this directory.

## allowlist-token

The wrapper forwards a transfer only when both participants are on the allowlist.

```text
add_to_allowlist(admin, address, expiration_ledger)
is_allowed(address) -> bool
transfer(from, to, amount) -> bool
```

## denylist-gate

A composable gate that other contracts call before moving funds.

```text
add_to_denylist(admin, address)
check(address) -> bool
```

## jurisdiction-flag

Stores issuer-controlled jurisdiction codes and checks permitted code lists.

```text
set_jurisdiction(issuer, address, code)
is_permitted_jurisdiction(address, allowed_codes) -> bool
```

## Wallet integration

Wallets should query the compliance views before building a transfer so they can
fail fast and show a clear message instead of submitting a transaction that will
be rejected.

Before a transfer, call the read-only views for both participants:

- `is_allowed(address)` on the allowlist token for the sender and the recipient.
- `check(address)` on the denylist gate for the sender and the recipient.
- `is_permitted_jurisdiction(address, allowed_codes)` on the jurisdiction flag
  contract for the sender and the recipient, using the issuer's permitted code
  list.

If any view returns `false`, block the transfer in the UI and surface a
compliance error naming the failing check and the affected address. If all views
return `true`, build and submit the transfer as usual.

See [Wallet integration](../wallet-integration.md) for a full guide with code
samples using the generated bindings.