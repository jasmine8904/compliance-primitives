#!/usr/bin/env python3
"""Generate docs/ERRORS.md from the Error enums in contracts/ and examples/.

Usage:
    python3 scripts/generate-errors-docs.py

The output is written to docs/ERRORS.md. Re-run after changing any Error enum
and commit the regenerated file so the CI check stays green.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "docs" / "ERRORS.md"

ENUM_RE = re.compile(r"pub\s+enum\s+Error\s*\{(.*?)\}", re.DOTALL)
VAR_RE = re.compile(r"^\s*(\w+)\s*=\s*(\d+)\s*,?\s*$")
DOC_RE = re.compile(r"^\s*/{3}\s?(?P<text>.*)$")


def parse_enum(text: str) -> list[dict]:
    """Return [{name, value, doc}] for each variant of the Error enum."""
    match = ENUM_RE.search(text)
    if match is None:
        return []

    variants: list[dict] = []
    pending_doc: list[str] = []

    for line in match.group(1).splitlines():
        doc_match = DOC_RE.match(line)
        if doc_match:
            pending_doc.append(doc_match.group("text"))
            continue

        var_match = VAR_RE.match(line)
        if var_match:
            variants.append(
                {
                    "name": var_match.group(1),
                    "value": int(var_match.group(2)),
                    "doc": " ".join(pending_doc),
                }
            )
            pending_doc = []
            continue

        # Any other line breaks the doc-comment attachment (rustdoc only
        # attaches comments immediately adjacent to the variant).
        pending_doc = []

    return variants


def section_title(directory: str) -> str:
    return "Contracts" if directory == "contracts" else "Examples"


def build_page() -> str:
    lines: list[str] = [
        "# Error code reference",
        "",
        "This page is a combined reference of every `Error` enum across the",
        "compliance primitives contracts and examples.",
        "",
        "It is generated from the source enums by",
        "`scripts/generate-errors-docs.py`. Do not edit by hand — regenerate",
        "instead:",
        "",
        "    python3 scripts/generate-errors-docs.py",
        "",
        "Each entry lists the numeric discriminant, the variant name, and the",
        "variant's doc comment (if any).",
        "",
    ]

    for section_dir in ("contracts", "examples"):
        base = ROOT / section_dir
        crates = sorted(
            d for d in base.iterdir() if d.is_dir() and (d / "src" / "lib.rs").exists()
        )
        if not crates:
            continue

        lines.append(f"## {section_title(section_dir)}")
        lines.append("")

        for crate_dir in crates:
            lib = crate_dir / "src" / "lib.rs"
            text = lib.read_text(encoding="utf-8")
            variants = parse_enum(text)
            if not variants:
                continue

            name = crate_dir.name
            rel = lib.relative_to(ROOT).as_posix()
            lines.append(f"### {name}")
            lines.append("")
            lines.append(f"Source: `{rel}`")
            lines.append("")
            lines.append("| Code | Error | Description |")
            lines.append("|------|-------|-------------|")
            for v in variants:
                doc = v["doc"].replace("|", "\\|")
                lines.append(f"| {v['value']} | `{v['name']}` | {doc} |")
            lines.append("")

    return "\n".join(lines) + "\n"


def main() -> int:
    content = build_page()
    OUT.write_text(content, encoding="utf-8")
    print(f"Wrote {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
