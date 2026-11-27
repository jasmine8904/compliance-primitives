# How Primitives Compose

This page shows three common ways to wire the compliance primitives into a
token, with a Mermaid diagram for each. [`ARCHITECTURE.md`](../ARCHITECTURE.md)
lists what each contract is; this page shows how they call each other at
runtime. For which primitives to pick, see
[`PRIMITIVE_SELECTION_GUIDE.md`](PRIMITIVE_SELECTION_GUIDE.md).

Every link in these diagrams is a cross-contract call, except `pausable`,
which is compiled into each primitive and never appears as a runtime call.

## Setup 1: Token → Aggregator → Checks

The consumer token asks `compliance-aggregator` for a verdict on each party.
The aggregator calls `denylist-gate` and `jurisdiction-flag` and AND-combines
the results. Use this when you want the fixed two-check shape and the fewest
moving parts. See [`examples/rwa-token-aggregator`](../examples/rwa-token-aggregator).

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Token as Consumer token
    participant Agg as compliance-aggregator
    participant Deny as denylist-gate
    participant Juris as jurisdiction-flag

    User->>Token: transfer(from, to, amount)
    Token->>Agg: check_address(from, allowed_codes)
    Agg->>Deny: check(from)
    Agg->>Juris: is_permitted_jurisdiction(from, allowed_codes)
    Agg-->>Token: (all_passed, checks)
    Token->>Agg: check_address(to, allowed_codes)
    Agg->>Deny: check(to)
    Agg->>Juris: is_permitted_jurisdiction(to, allowed_codes)
    Agg-->>Token: (all_passed, checks)
    alt both parties passed
        Token-->>User: transfer proceeds
    else either party failed
        Token-->>User: rejected
    end
```

The aggregator only AND-combines its two checks. If you need the aggregator to
be paused or frozen as well, register a `circuit-breaker` with
`set_circuit_breaker`; `check_address` honours it.

## Setup 2: Token → Policy Engine → Checks

The consumer token makes one call to `policy-engine.evaluate(from, to)`. The
engine runs an admin-managed list of checks against both addresses and combines
them with `All` (AND) or `Any` (OR). Use this when the check list must change
without a redeploy, or when you need OR logic. See
[`examples/circuit-breaker-policy-engine`](../examples/circuit-breaker-policy-engine)
and [`examples/policy-engine-audit`](../examples/policy-engine-audit).

```mermaid
flowchart LR
    Token["Consumer token<br/>transfer(from, to, amount)"]
    PE["policy-engine<br/>evaluate(from, to)"]
    Op{{"CombineOp<br/>All = AND · Any = OR"}}

    subgraph Checks["Admin-managed CheckKind list"]
        direction TB
        C1["Denylist<br/>denylist-gate.check(addr)"]
        C2["Jurisdiction<br/>jurisdiction-flag.is_permitted_jurisdiction(addr, codes)"]
        C3["Allowlist<br/>allowlist-token.is_allowed(addr)"]
        C4["CircuitBreaker<br/>circuit-breaker.is_frozen()"]
    end

    Token -->|"evaluate(from, to)"| PE
    PE --> Op
    Op -. "each check runs on from and to" .-> Checks
    Checks -->|"pass / fail per check"| Op
    Op -->|"bool"| Token
```

`evaluate` returns a single `bool`. Use `evaluate_verbose` when you need to
know which check failed, for example in an audit log.

The `CircuitBreaker` check is the only one that does not look at an address.
It fails the whole policy while the breaker is frozen.

## Setup 3: Multisig Admin

Every primitive takes an `admin` address at `initialize` time (`allowlist-token`
takes it alongside the wrapped `token`). Setting that address to a
`multisig-admin` contract, instead of a single key, means every admin call
needs M-of-N signer approval. No primitive code changes are needed. See
[`DESIGN_MULTISIG_ADMIN.md`](../DESIGN_MULTISIG_ADMIN.md) and
[`examples/multisig-governed-allowlist`](../examples/multisig-governed-allowlist).

```mermaid
flowchart TB
    Signers["M of N signers<br/>(each signs an auth entry)"]
    MA["multisig-admin<br/>__check_auth: threshold check"]

    subgraph Primitives["Primitives with multisig-admin as admin"]
        direction LR
        DG["denylist-gate"]
        JF["jurisdiction-flag"]
        AL["allowlist-token"]
    end

    Compose["policy-engine or compliance-aggregator<br/>(admin = multisig-admin)"]
    Breaker["circuit-breaker<br/>(admin = multisig-admin)"]

    Signers -->|"approvals"| MA
    MA -->|"admin.require_auth() succeeds"| DG
    MA -->|"admin.require_auth() succeeds"| JF
    MA -->|"admin.require_auth() succeeds"| AL
    MA -->|"admin.require_auth() succeeds"| Compose
    MA -->|"admin.require_auth() succeeds"| Breaker
```

Each admin-only call (for example `denylist-gate` add or remove, or
`jurisdiction-flag` set or remove) reaches `require_auth` on the admin address.
That re-enters `multisig-admin.__check_auth`, which needs `threshold` signer
approvals. The primitive itself does not know the admin is a multisig.

Check the threshold before you go live. A 1-of-N setup is a single key with
extra steps.

## Choosing a setup

| If you need | Use |
|---|---|
| Denylist and jurisdiction checks, fixed shape, minimal moving parts | Setup 1 (aggregator) |
| Check list that changes at runtime, or OR logic, or a freeze check inside the policy | Setup 2 (policy engine) |
| Any admin action to need more than one signer | Setup 3 (multisig admin), combined with either setup above |

Setups 1 and 2 are alternatives: they do the same job at different levels of
flexibility. Setup 3 is orthogonal and can wrap either one.
