#!/usr/bin/env python3
"""Validate KCS markdown links and compiler index coverage.

Checks:
1) All local markdown links in docs/ and CONTRIBUTING.md resolve.
2) Every compiler KCS note (docs/kcs/compiler/kcs-*.md) is linked from:
   - docs/kcs/compiler/README.md
   - docs/kcs/README.md
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"

LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")


def _check_local_markdown_links() -> list[str]:
    files = list(DOCS.rglob("*.md")) + [ROOT / "CONTRIBUTING.md"]
    problems: list[str] = []
    for file_path in files:
        text = file_path.read_text(encoding="utf-8")
        for link in LINK_RE.findall(text):
            if link.startswith(("http://", "https://", "#", "mailto:")):
                continue
            target = (file_path.parent / link.split("#", 1)[0]).resolve()
            if not target.exists():
                rel = file_path.relative_to(ROOT)
                problems.append(f"broken link in {rel}: {link}")
    return problems


def _extract_link_targets(md_path: Path) -> set[str]:
    text = md_path.read_text(encoding="utf-8")
    return {target.split("#", 1)[0] for target in LINK_RE.findall(text)}


def _check_compiler_index_coverage() -> list[str]:
    compiler_dir = DOCS / "kcs" / "compiler"
    compiler_notes = sorted(p.name for p in compiler_dir.glob("kcs-*.md") if p.is_file())

    compiler_index = DOCS / "kcs" / "compiler" / "README.md"
    top_index = DOCS / "kcs" / "README.md"

    compiler_targets = _extract_link_targets(compiler_index)
    top_targets = _extract_link_targets(top_index)

    problems: list[str] = []
    for note in compiler_notes:
        if note not in compiler_targets:
            problems.append(f"compiler index missing link to docs/kcs/compiler/{note}")
        if f"compiler/{note}" not in top_targets:
            problems.append(f"top-level KCS index missing link to docs/kcs/compiler/{note}")

    return problems


def main() -> int:
    problems = []
    problems.extend(_check_local_markdown_links())
    problems.extend(_check_compiler_index_coverage())

    if problems:
        print("KCS docs checks failed:")
        for p in problems:
            print(f"- {p}")
        return 1

    print("KCS docs checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
