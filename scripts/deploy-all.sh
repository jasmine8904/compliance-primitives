#!/usr/bin/env bash
# Deploy and wire all compliance-primitives contracts in one shot.
#
# Reads network and key configuration from a JSON config file (default:
# scripts/deploy-config.json) and writes a JSON file of deployed contract IDs
# to the path specified by --output (default: deployed-ids.json).
#
# Usage:
#   scripts/deploy-all.sh [--config <path>] [--output <path>]
#
# Config file keys (all optional except network/source):
#   network        Stellar network to target (testnet | mainnet | futurenet)
#   source         Stellar CLI identity name used to sign transactions
#   rpc_url        Custom RPC URL (overrides the network default)
#   network_passphrase  Custom network passphrase (required when rpc_url is set)
#
# Requires: stellar-cli, jq
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
CONFIG_FILE="scripts/deploy-config.json"
OUTPUT_FILE="deployed-ids.json"

# ---------------------------------------------------------------------------
# Argument parsing
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --config)  CONFIG_FILE="$2"; shift 2 ;;
    --output)  OUTPUT_FILE="$2"; shift 2 ;;
    *) echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

if [ ! -f "$CONFIG_FILE" ]; then
  echo "error: config file not found at $CONFIG_FILE" >&2
  echo "Create one based on scripts/deploy-config.example.json" >&2
  exit 1
fi

jq_required() {
  local key="$1"
  jq -r --exit-status "$key" "$CONFIG_FILE" 2>/dev/null || {
    echo "error: required field $key missing from $CONFIG_FILE" >&2
    exit 1
  }
}

jq_optional() {
  local key="$1" default="${2:-}"
  val="$(jq -r "$key // empty" "$CONFIG_FILE" 2>/dev/null)"
  echo "${val:-$default}"
}

NETWORK="$(jq_required '.network')"
SOURCE_IDENTITY="$(jq_required '.source')"
RPC_URL="$(jq_optional '.rpc_url')"
NETWORK_PASSPHRASE="$(jq_optional '.network_passphrase')"

# Build the common stellar CLI flags
STELLAR_FLAGS=(--source "$SOURCE_IDENTITY" --network "$NETWORK")
if [ -n "$RPC_URL" ]; then
  STELLAR_FLAGS+=(--rpc-url "$RPC_URL")
fi
if [ -n "$NETWORK_PASSPHRASE" ]; then
  STELLAR_FLAGS+=(--network-passphrase "$NETWORK_PASSPHRASE")
fi

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
echo "==> Building all contracts..."
stellar contract build

need_wasm() {
  local wasm="$1"
  if [ ! -f "$wasm" ]; then
    echo "error: build artifact not found: $wasm" >&2
    exit 1
  fi
}

WASM_DIR="target/wasm32v1-none/release"

need_wasm "$WASM_DIR/circuit_breaker.wasm"
need_wasm "$WASM_DIR/policy_engine.wasm"
need_wasm "$WASM_DIR/compliance_aggregator.wasm"
need_wasm "$WASM_DIR/denylist_gate.wasm"
need_wasm "$WASM_DIR/jurisdiction_flag.wasm"
need_wasm "$WASM_DIR/allowlist_token.wasm"
need_wasm "$WASM_DIR/audit_log.wasm"
need_wasm "$WASM_DIR/multisig_admin.wasm"

ADMIN_ADDRESS="$(stellar keys address "$SOURCE_IDENTITY")"

# ---------------------------------------------------------------------------
# Deploy helper
# ---------------------------------------------------------------------------
deploy_contract() {
  local label="$1" wasm="$2"
  echo "==> Deploying $label..." >&2
  stellar contract deploy \
    --wasm "$wasm" \
    "${STELLAR_FLAGS[@]}"
}

invoke_contract() {
  local label="$1" id="$2"
  shift 2
  echo "==> Initializing $label..." >&2
  stellar contract invoke \
    --id "$id" \
    "${STELLAR_FLAGS[@]}" \
    -- "$@"
}

# ---------------------------------------------------------------------------
# Deploy every contract
# ---------------------------------------------------------------------------
CIRCUIT_BREAKER_ID="$(deploy_contract circuit-breaker  "$WASM_DIR/circuit_breaker.wasm")"
POLICY_ENGINE_ID="$(deploy_contract    policy-engine    "$WASM_DIR/policy_engine.wasm")"
AGGREGATOR_ID="$(deploy_contract       compliance-aggregator "$WASM_DIR/compliance_aggregator.wasm")"
DENYLIST_ID="$(deploy_contract         denylist-gate    "$WASM_DIR/denylist_gate.wasm")"
JURISDICTION_ID="$(deploy_contract     jurisdiction-flag "$WASM_DIR/jurisdiction_flag.wasm")"
ALLOWLIST_ID="$(deploy_contract        allowlist-token  "$WASM_DIR/allowlist_token.wasm")"
AUDIT_LOG_ID="$(deploy_contract        audit-log        "$WASM_DIR/audit_log.wasm")"

# ---------------------------------------------------------------------------
# Initialize contracts (order matters: dependencies first)
# ---------------------------------------------------------------------------
invoke_contract circuit-breaker  "$CIRCUIT_BREAKER_ID" \
  initialize --admin "$ADMIN_ADDRESS"

invoke_contract denylist-gate    "$DENYLIST_ID" \
  initialize --admin "$ADMIN_ADDRESS"

invoke_contract jurisdiction-flag "$JURISDICTION_ID" \
  initialize --issuer "$ADMIN_ADDRESS"

invoke_contract audit-log        "$AUDIT_LOG_ID" \
  initialize --admin "$ADMIN_ADDRESS"

# policy-engine: combine_op defaults to "and" (0 in Soroban XDR enum)
invoke_contract policy-engine    "$POLICY_ENGINE_ID" \
  initialize \
  --admin "$ADMIN_ADDRESS" \
  --op And \
  --circuit_breaker "$CIRCUIT_BREAKER_ID"

# Wire the aggregator with all three primitives
invoke_contract compliance-aggregator "$AGGREGATOR_ID" \
  initialize \
  --admin "$ADMIN_ADDRESS" \
  --denylist_gate "$DENYLIST_ID" \
  --jurisdiction_flag "$JURISDICTION_ID" \
  --circuit_breaker "$CIRCUIT_BREAKER_ID"

# ---------------------------------------------------------------------------
# Write output JSON
# ---------------------------------------------------------------------------
cat > "$OUTPUT_FILE" <<JSON
{
  "network": "${NETWORK}",
  "admin": "${ADMIN_ADDRESS}",
  "circuit_breaker": "${CIRCUIT_BREAKER_ID}",
  "denylist_gate": "${DENYLIST_ID}",
  "jurisdiction_flag": "${JURISDICTION_ID}",
  "allowlist_token": "${ALLOWLIST_ID}",
  "audit_log": "${AUDIT_LOG_ID}",
  "policy_engine": "${POLICY_ENGINE_ID}",
  "compliance_aggregator": "${AGGREGATOR_ID}"
}
JSON

echo ""
echo "============================================================"
echo "Deployed contracts ($NETWORK)"
echo "============================================================"
jq . "$OUTPUT_FILE"
echo "============================================================"
echo "Output written to $OUTPUT_FILE"
