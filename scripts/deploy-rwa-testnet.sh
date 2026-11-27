#!/usr/bin/env bash
# Build and deploy the full RWA reference stack to Stellar testnet:
#   allowlist-token, denylist-gate, jurisdiction-flag, rwa-token
#
# Prints a shell-friendly summary of all contract IDs, records them in
# testnet-ids.json under the current release, and refreshes the README
# testnet IDs table via scripts/update-testnet-ids.mjs. Redeploy after a
# testnet reset by re-running this script.
#
# Usage:
#   STELLAR_SOURCE=<identity> ./scripts/deploy-rwa-testnet.sh
#
# Optional:
#   STELLAR_NETWORK=testnet   (default)
#   ALLOWED_CODES='["US","CA"]'  — JSON array passed to rwa-token initialize
#   RELEASE=<label>           — release key in testnet-ids.json
#                               (default: package version from Cargo.toml)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

SOURCE_IDENTITY="${STELLAR_SOURCE:-default}"
NETWORK="${STELLAR_NETWORK:-testnet}"
ALLOWED_CODES="${ALLOWED_CODES:-[\"US\",\"CA\"]}"
TESTNET_IDS_FILE="$REPO_ROOT/testnet-ids.json"

if [ -z "${RELEASE:-}" ]; then
  RELEASE="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\(.*\)".*/\1/p' "$REPO_ROOT/Cargo.toml" | head -n 1)"
fi
if [ -z "$RELEASE" ]; then
  echo "error: could not determine release; set RELEASE=<label>" >&2
  exit 1
fi

need_wasm() {
  local path="$1"
  if [ ! -f "$path" ]; then
    echo "error: expected build artifact not found at $path" >&2
    exit 1
  fi
}

deploy_one() {
  local label="$1"
  local wasm="$2"
  echo "==> Deploying ${label}..." >&2
  stellar contract deploy \
    --wasm "$wasm" \
    --source "$SOURCE_IDENTITY" \
    --network "$NETWORK"
}

echo "==> Building workspace contracts to wasm..."
stellar contract build

ALLOWLIST_WASM="target/wasm32v1-none/release/allowlist_token.wasm"
GATE_WASM="target/wasm32v1-none/release/denylist_gate.wasm"
JURISDICTION_WASM="target/wasm32v1-none/release/jurisdiction_flag.wasm"
RWA_WASM="target/wasm32v1-none/release/rwa_token.wasm"

need_wasm "$ALLOWLIST_WASM"
need_wasm "$GATE_WASM"
need_wasm "$JURISDICTION_WASM"
need_wasm "$RWA_WASM"

ALLOWLIST_ID="$(deploy_one allowlist-token "$ALLOWLIST_WASM")"
GATE_ID="$(deploy_one denylist-gate "$GATE_WASM")"
JURISDICTION_ID="$(deploy_one jurisdiction-flag "$JURISDICTION_WASM")"
RWA_ID="$(deploy_one rwa-token "$RWA_WASM")"

# Placeholder underlying token address for allowlist-token.initialize —
# rwa-token only calls is_allowed(), so any valid address works. We reuse
# the allowlist contract id itself as a stable placeholder.
UNDERLYING_PLACEHOLDER="$ALLOWLIST_ID"

ISSUER="$(stellar keys address "$SOURCE_IDENTITY")"

echo "==> Initializing allowlist-token..."
stellar contract invoke \
  --id "$ALLOWLIST_ID" \
  --source "$SOURCE_IDENTITY" \
  --network "$NETWORK" \
  -- \
  initialize \
  --admin "$ISSUER" \
  --token "$UNDERLYING_PLACEHOLDER"

echo "==> Initializing denylist-gate..."
stellar contract invoke \
  --id "$GATE_ID" \
  --source "$SOURCE_IDENTITY" \
  --network "$NETWORK" \
  -- \
  initialize \
  --admin "$ISSUER"

echo "==> Initializing jurisdiction-flag..."
stellar contract invoke \
  --id "$JURISDICTION_ID" \
  --source "$SOURCE_IDENTITY" \
  --network "$NETWORK" \
  -- \
  initialize \
  --issuer "$ISSUER"

echo "==> Initializing rwa-token..."
stellar contract invoke \
  --id "$RWA_ID" \
  --source "$SOURCE_IDENTITY" \
  --network "$NETWORK" \
  -- \
  initialize \
  --allowlist "$ALLOWLIST_ID" \
  --gate "$GATE_ID" \
  --jurisdiction "$JURISDICTION_ID" \
  --allowed_codes "$ALLOWED_CODES"

echo "==> Recording contract IDs in testnet-ids.json (release: $RELEASE)..."
RELEASE="$RELEASE" \
NETWORK="$NETWORK" \
ALLOWLIST_ID="$ALLOWLIST_ID" \
GATE_ID="$GATE_ID" \
JURISDICTION_ID="$JURISDICTION_ID" \
RWA_ID="$RWA_ID" \
TESTNET_IDS_FILE="$TESTNET_IDS_FILE" \
node --input-type=module -e '
import { readFileSync, writeFileSync } from "node:fs";

const file = process.env.TESTNET_IDS_FILE;
const release = process.env.RELEASE;

let data = { releases: {} };
try {
  data = JSON.parse(readFileSync(file, "utf8"));
} catch (err) {
  if (err.code !== "ENOENT") throw err;
}
if (!data.releases) data.releases = {};

data.releases[release] = {
  network: process.env.NETWORK,
  deployedAt: new Date().toISOString(),
  contracts: {
    "allowlist-token": process.env.ALLOWLIST_ID,
    "denylist-gate": process.env.GATE_ID,
    "jurisdiction-flag": process.env.JURISDICTION_ID,
    "rwa-token": process.env.RWA_ID,
  },
};

writeFileSync(file, JSON.stringify(data, null, 2) + "\n");
'

echo "==> Updating README testnet IDs table..."
node "$REPO_ROOT/scripts/update-testnet-ids.mjs"

cat <<EOF

============================================================
RWA testnet reference deployment ($NETWORK)
============================================================
Release:            $RELEASE
Issuer / admin:     $ISSUER
allowlist-token:    $ALLOWLIST_ID
denylist-gate:      $GATE_ID
jurisdiction-flag:  $JURISDICTION_ID
rwa-token:          $RWA_ID
allowed_codes:      $ALLOWED_CODES
============================================================
Contract IDs recorded in testnet-ids.json and README updated.
EOF
