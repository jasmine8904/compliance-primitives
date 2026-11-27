#!/usr/bin/env bash
# Ensure that a version bump in Cargo.toml is accompanied by a CHANGELOG.md entry.
#
# This script compares the workspace version in Cargo.toml against the entries
# in CHANGELOG.md and fails if one changed without the other in the diff.
#
# Exit codes:
#  0: version and changelog are in sync
#  1: version bumped but CHANGELOG.md not updated
#  2: CHANGELOG.md updated but version not bumped
#  3: script error

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Get the current version from Cargo.toml
get_current_version() {
    grep -E '^\[workspace\.package\]' Cargo.toml > /dev/null || {
        echo "error: workspace.package not found in Cargo.toml" >&2
        exit 3
    }
    grep -A 50 '^\[workspace\.package\]' Cargo.toml \
        | grep -E '^version = ' \
        | head -1 \
        | sed 's/version = "\(.*\)"/\1/'
}

# Check if Cargo.toml version was changed
if git diff HEAD~1 HEAD -- Cargo.toml | grep -q '^[+-].*version = '; then
    CARGO_VERSION_CHANGED=1
else
    CARGO_VERSION_CHANGED=0
fi

# Check if CHANGELOG.md was changed
if git diff HEAD~1 HEAD -- CHANGELOG.md | grep -q .; then
    CHANGELOG_CHANGED=1
else
    CHANGELOG_CHANGED=0
fi

# Validate that every release section in CHANGELOG.md contains per-contract
# headings. Each release section starts with a "## " heading (e.g. "## [1.2.3]")
# and must contain at least one "### " per-contract heading before the next
# release section.
validate_per_contract_sections() {
    if [ ! -f CHANGELOG.md ]; then
        echo "error: CHANGELOG.md not found" >&2
        exit 3
    fi

    local in_release=0
    local release_heading=""
    local has_contract_heading=0
    local failed=0
    local line

    while IFS= read -r line || [ -n "$line" ]; do
        if [[ "$line" =~ ^##[[:space:]] ]]; then
            # Close out the previous release section
            if [ "$in_release" -eq 1 ] && [ "$has_contract_heading" -eq 0 ]; then
                echo "error: release section '$release_heading' in CHANGELOG.md has no per-contract headings" >&2
                failed=1
            fi
            in_release=1
            release_heading="$line"
            has_contract_heading=0
        elif [[ "$line" =~ ^###[[:space:]] ]] && [ "$in_release" -eq 1 ]; then
            has_contract_heading=1
        fi
    done < CHANGELOG.md

    # Close out the final release section
    if [ "$in_release" -eq 1 ] && [ "$has_contract_heading" -eq 0 ]; then
        echo "error: release section '$release_heading' in CHANGELOG.md has no per-contract headings" >&2
        failed=1
    fi

    if [ "$failed" -eq 1 ]; then
        echo "Please add a '### <contract>' heading for each affected contract under every release." >&2
        exit 1
    fi
}

validate_per_contract_sections

# If this is the first commit or HEAD~1 doesn't exist, skip the check
if ! git rev-parse HEAD~1 > /dev/null 2>&1; then
    echo "info: skipping changelog check on initial commit"
    exit 0
fi

# Report results
if [ "$CARGO_VERSION_CHANGED" -eq 1 ] && [ "$CHANGELOG_CHANGED" -eq 0 ]; then
    echo "error: Cargo.toml version bumped but CHANGELOG.md not updated" >&2
    echo "Please add an entry to CHANGELOG.md for the new version." >&2
    exit 1
fi

if [ "$CARGO_VERSION_CHANGED" -eq 0 ] && [ "$CHANGELOG_CHANGED" -eq 1 ]; then
    echo "error: CHANGELOG.md updated but Cargo.toml version not bumped" >&2
    echo "Please update the version in Cargo.toml's [workspace.package] section." >&2
    exit 2
fi

# Both changed or neither changed — OK
exit 0
