#!/usr/bin/env bash
# =============================================================================
# examples/stablecoin-sanctions-sync/sync.sh
#
# Off-chain half of the loop: makes the on-chain denylist-gate match a
# sanctions list produced by an off-chain oracle/adapter.
#
# Usage:
#   DENYLIST_GATE_ID=C... COMPLIANCE_SOURCE=issuer ADMIN=G... \
#   NETWORK=testnet ./sync.sh path/to/sanctions-list.txt
#
# The list file holds one Stellar account address (G...) per line. Blank
# lines and lines starting with '#' are ignored.
#
# Behaviour:
#   - Refuses to run on an empty list. An empty list from a failed oracle
#     fetch must not clear the denylist.
#   - Refuses any line that is not a G-address.
#   - Adds addresses that are in the list but not in the last-synced snapshot.
#   - Removes addresses that were in the snapshot but are no longer listed.
#   - Writes the snapshot only after every invoke succeeds. A failed run is
#     safe to re-run: add_to_denylist and remove_from_denylist are idempotent.
#
# Shortcut: the snapshot is a local file, so two sync hosts can drift.
# Upgrade to an on-chain or shared source of truth before running more than
# one syncer.
# =============================================================================
set -euo pipefail

LIST="${1:?usage: sync.sh <sanctions-list-file>}"
: "${DENYLIST_GATE_ID:?set DENYLIST_GATE_ID to the denylist-gate contract id}"
: "${COMPLIANCE_SOURCE:?set COMPLIANCE_SOURCE to the stellar CLI identity that signs}"
: "${ADMIN:?set ADMIN to the G-address of the admin or compliance officer}"
NETWORK="${NETWORK:-testnet}"
SNAPSHOT="${SNAPSHOT:-$(dirname "$LIST")/last-synced.txt}"

NEW=$(mktemp)
OLD=$(mktemp)
ADD=$(mktemp)
REMOVE=$(mktemp)
trap 'rm -f "$NEW" "$OLD" "$ADD" "$REMOVE"' EXIT

{ grep -v -e '^[[:space:]]*$' -e '^[[:space:]]*#' "$LIST" || true; } | sed 's/[[:space:]]//g' | sort -u > "$NEW"

if [ ! -s "$NEW" ]; then
  echo "refusing to sync: $LIST has no addresses" >&2
  exit 1
fi

while read -r addr; do
  if ! [[ "$addr" =~ ^G[A-Z2-7]{55}$ ]]; then
    echo "refusing to sync: invalid address in $LIST: $addr" >&2
    exit 1
  fi
done < "$NEW"

if [ -f "$SNAPSHOT" ]; then
  sort -u "$SNAPSHOT" > "$OLD"
fi

comm -23 "$NEW" "$OLD" > "$ADD"
comm -13 "$NEW" "$OLD" > "$REMOVE"
echo "to add: $(wc -l < "$ADD" | tr -d ' ')  to remove: $(wc -l < "$REMOVE" | tr -d ' ')"

invoke() {
  stellar contract invoke \
    --id "$DENYLIST_GATE_ID" \
    --source "$COMPLIANCE_SOURCE" \
    --network "$NETWORK" \
    -- "$1" \
    --admin "$ADMIN" \
    --address "$2" > /dev/null
}

while read -r addr; do
  [ -n "$addr" ] || continue
  invoke add_to_denylist "$addr"
  echo "denylisted $addr"
done < "$ADD"

while read -r addr; do
  [ -n "$addr" ] || continue
  invoke remove_from_denylist "$addr"
  echo "un-denylisted $addr"
done < "$REMOVE"

cp "$NEW" "$SNAPSHOT"
echo "sync complete; snapshot written to $SNAPSHOT"
