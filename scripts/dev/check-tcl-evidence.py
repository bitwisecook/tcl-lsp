#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Check the structure and reversible blocks of Tcl execution evidence."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from tcl_evidence_format import validate_document, validate_fixture


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[2],
        help="repository root containing evidence/tcl",
    )
    args = parser.parse_args()
    root = args.root.resolve()
    directory = root / "evidence/tcl"
    files = sorted(directory.rglob("*.md")) if directory.is_dir() else []
    if not files:
        print("no Tcl execution evidence files found", file=sys.stderr)
        return 1
    errors: list[str] = []
    for path in files:
        try:
            document = path.read_bytes().decode("utf-8")
            if path.name == "README.md":
                continue
            found = (
                validate_fixture(document)
                if "fixtures" in path.relative_to(directory).parts
                else validate_document(document, path)
            )
            errors.extend(f"{path.relative_to(root)}: {error}" for error in found)
        except (OSError, UnicodeError) as error:
            errors.append(f"{path.relative_to(root)}: {error}")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Checked {len(files)} Tcl execution evidence file(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
