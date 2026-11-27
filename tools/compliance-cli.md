# compliance-cli

`tools/compliance-cli` wraps the common `stellar contract invoke` operations for the
allowlist, denylist, jurisdiction, freeze, and pause admin flows.

Copy `tools/compliance-cli.env.example` to `compliance-cli.env` and fill in the
deployed contract IDs and local Stellar identity:

```env
ALLOWLIST_CONTRACT_ID=C...
DENYLIST_CONTRACT_ID=C...
JURISDICTION_CONTRACT_ID=C...
CIRCUIT_BREAKER_CONTRACT_ID=C...
STELLAR_NETWORK=testnet
STELLAR_SOURCE=my-testnet-key
STELLAR_ADMIN=G...
```

Examples:

```sh
node tools/compliance-cli allowlist add GUSER
node tools/compliance-cli allowlist remove GUSER
node tools/compliance-cli denylist add GUSER
node tools/compliance-cli jurisdiction set GUSER US
node tools/compliance-cli jurisdiction remove GUSER US
node tools/compliance-cli freeze freeze GUSER --contract allowlist
node tools/compliance-cli freeze unfreeze GUSER --contract allowlist
node tools/compliance-cli pause pause --contract denylist
node tools/compliance-cli pause unpause --contract jurisdiction
```

Use `--config <file>` to select another environment or JSON config file.
Use `--dry-run` to print the exact Stellar CLI command without submitting it.
