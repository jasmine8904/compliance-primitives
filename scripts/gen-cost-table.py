#!/usr/bin/env python3
"""Regenerate the per-entry-point cost table in BENCHMARKS.md from budget-baselines.toml.

Usage: python3 scripts/gen-cost-table.py

Rewrites the block between the cost-table markers in BENCHMARKS.md. CI runs this
script and fails if the committed table is stale.

Each `[contract.entry_point]` section may set `cpu`, `memory`, `read` and `write`.
Only `cpu` and `memory` are enforced by the budget_regression tests today; a
missing `read`/`write` renders as "—" until the harness records them.
"""

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINES = ROOT / "budget-baselines.toml"
BENCHMARKS = ROOT / "BENCHMARKS.md"
START = "<!-- cost-table:start -->"
END = "<!-- cost-table:end -->"
# A baseline of exactly 1 is an unmeasured placeholder, not a real cost.
PLACEHOLDER = 1


def cell(value):
    if value is None:
        return "—"
    if value == PLACEHOLDER:
        return "not measured"
    return f"{value:,}"


def build_table():
    with BASELINES.open("rb") as f:
        data = tomllib.load(f)
    lines = [
        "| Contract | Entry point | CPU instructions | Memory (bytes) | Read (bytes) | Write (bytes) |",
        "|---|---|---:|---:|---:|---:|",
    ]
    for contract, entries in data.items():
        for entry_point, costs in entries.items():
            lines.append(
                f"| `{contract}` | `{entry_point}` | {cell(costs.get('cpu'))} "
                f"| {cell(costs.get('memory'))} | {cell(costs.get('read'))} "
                f"| {cell(costs.get('write'))} |"
            )
    return "\n".join(lines)


def main():
    text = BENCHMARKS.read_text(encoding="utf-8")
    pattern = re.compile(re.escape(START) + r".*?" + re.escape(END), re.S)
    if not pattern.search(text):
        sys.exit(f"error: {START} / {END} markers not found in BENCHMARKS.md")
    table = build_table()
    updated = pattern.sub(lambda _: f"{START}\n{table}\n{END}", text, count=1)
    if updated != text:
        BENCHMARKS.write_text(updated, encoding="utf-8")
        print(f"updated {BENCHMARKS.relative_to(ROOT)}")
    else:
        print("cost table already up to date")


if __name__ == "__main__":
    main()
