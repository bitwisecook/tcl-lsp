#!/usr/bin/env python3
"""Validate KCS and design docs markdown links and index coverage.

Checks:
1) All local markdown links in docs/ and CONTRIBUTING.md resolve.
2) Every design doc under docs/design/ (excluding README.md files and
   templates/) is linked from docs/design/README.md OR from a README.md
   inside its parent directory (which is itself linked from the top
   design index).
3) Every top-level KCS note under docs/kcs/ (depth 1, excluding
   README.md, STYLE.md, and templates/) is linked from docs/kcs/README.md.
4) Every KCS note contains a `> **Audience:**` blockquote line (warning
   only, does not fail the build).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOCS = ROOT / "docs"

LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
AUDIENCE_RE = re.compile(r"^>\s*\*\*Audience:\*\*", re.MULTILINE)
FENCE_RE = re.compile(r"^(```|~~~)", re.MULTILINE)


def _strip_fenced_code(text: str) -> str:
    """Remove fenced code blocks from markdown so links inside code
    examples are not validated. Both ``` and ~~~ fences are supported.
    An unterminated fence at end of file is treated as extending to EOF.
    """
    out: list[str] = []
    in_fence = False
    fence_marker = ""
    for line in text.splitlines(keepends=True):
        stripped = line.lstrip()
        if not in_fence:
            if stripped.startswith(("```", "~~~")):
                in_fence = True
                fence_marker = "```" if stripped.startswith("```") else "~~~"
                continue
            out.append(line)
        else:
            if stripped.startswith(fence_marker):
                in_fence = False
                fence_marker = ""
            # drop the fenced line either way
    return "".join(out)


def _check_local_markdown_links() -> list[str]:
    files = list(DOCS.rglob("*.md")) + [ROOT / "CONTRIBUTING.md", ROOT / "AGENTS.md"]
    problems: list[str] = []
    for file_path in files:
        # docs/archive/ holds historical snapshots of completed projects.
        # Their links reference the codebase as it stood at the time of
        # capture and are expected to rot; skip link-validation for them.
        if "archive" in file_path.parts:
            continue
        text = _strip_fenced_code(file_path.read_text(encoding="utf-8"))
        for link in LINK_RE.findall(text):
            if link.startswith(("http://", "https://", "#", "mailto:")):
                continue
            target = (file_path.parent / link.split("#", 1)[0]).resolve()
            if not target.exists():
                rel = file_path.relative_to(ROOT)
                problems.append(f"broken link in {rel}: {link}")
    return problems


def _extract_link_targets(md_path: Path) -> set[str]:
    if not md_path.exists():
        return set()
    text = _strip_fenced_code(md_path.read_text(encoding="utf-8"))
    return {target.split("#", 1)[0] for target in LINK_RE.findall(text)}


def _check_design_index_coverage() -> list[str]:
    design_dir = DOCS / "design"
    if not design_dir.exists():
        return []

    top_index = design_dir / "README.md"
    top_targets = _extract_link_targets(top_index)

    problems: list[str] = []
    for note in sorted(design_dir.rglob("*.md")):
        if note.name == "README.md":
            continue
        # Templates are linked from docs/design/templates/README.md.
        rel = note.relative_to(design_dir)
        rel_str = str(rel).replace("\\", "/")

        if rel_str in top_targets:
            continue

        # Accept a link from the parent-directory README.md if that
        # README.md is itself linked from the top design index.
        parent_readme = note.parent / "README.md"
        parent_rel = parent_readme.relative_to(design_dir)
        parent_rel_str = str(parent_rel).replace("\\", "/")
        if parent_readme.exists() and parent_rel_str in top_targets:
            parent_targets = _extract_link_targets(parent_readme)
            if note.name in parent_targets:
                continue

        problems.append(f"design index missing link to docs/design/{rel_str}")
    return problems


def _check_kcs_index_coverage() -> list[str]:
    kcs_dir = DOCS / "kcs"
    if not kcs_dir.exists():
        return []

    top_index = kcs_dir / "README.md"
    top_targets = _extract_link_targets(top_index)

    skip_names = {"README.md", "STYLE.md"}
    skip_dirs = {"templates", "features", "screenshots"}

    problems: list[str] = []
    for note in sorted(kcs_dir.glob("*.md")):
        if note.name in skip_names:
            continue
        if note.name in top_targets:
            continue
        problems.append(f"KCS index missing link to docs/kcs/{note.name}")

    # features/ has its own index read by the help tool at runtime.
    features_dir = kcs_dir / "features"
    if features_dir.exists():
        features_index = features_dir / "README.md"
        features_targets = _extract_link_targets(features_index)
        for note in sorted(features_dir.glob("*.md")):
            if note.name == "README.md":
                continue
            if note.name in features_targets:
                continue
            problems.append(f"features index missing link to docs/kcs/features/{note.name}")

    _ = skip_dirs  # reserved for future directory-level rules
    return problems


def _check_kcs_audience_headers() -> list[str]:
    """Warn (not fail) on KCS notes missing an Audience header."""
    kcs_dir = DOCS / "kcs"
    if not kcs_dir.exists():
        return []

    warnings: list[str] = []
    candidates = list(kcs_dir.glob("kcs-*.md")) + list((kcs_dir / "features").glob("kcs-*.md"))
    for note in sorted(candidates):
        text = note.read_text(encoding="utf-8")
        if not AUDIENCE_RE.search(text):
            rel = note.relative_to(ROOT)
            warnings.append(f"KCS note missing `> **Audience:**` header: {rel}")
    return warnings


def main() -> int:
    problems: list[str] = []
    problems.extend(_check_local_markdown_links())
    problems.extend(_check_design_index_coverage())
    problems.extend(_check_kcs_index_coverage())

    warnings = _check_kcs_audience_headers()

    if problems:
        print("KCS docs checks failed:")
        for p in problems:
            print(f"- {p}")
        if warnings:
            print()
            print(f"(plus {len(warnings)} audience-header warnings)")
        return 1

    if warnings:
        print(f"KCS docs checks passed with {len(warnings)} warnings:")
        for w in warnings:
            print(f"- {w}")
        # Warnings do not fail the build yet; this will tighten later.

    print("KCS docs checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
