#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Turn an exact script or output file into a readable Markdown block."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from tcl_evidence_format import markdown_block


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="script or output file to encode")
    parser.add_argument(
        "--kind",
        choices=("script", "output"),
        default="script",
        help="choose a script block (default) or an output block",
    )
    parser.add_argument(
        "-o", "--output", type=Path, help="write the Markdown block to this file"
    )
    args = parser.parse_args()
    try:
        language = "tcl-visible" if args.kind == "script" else "text-visible"
        block = markdown_block(args.input.read_bytes(), language)
        if args.output is None:
            sys.stdout.write(block)
        else:
            args.output.write_bytes(block.encode("utf-8"))
    except (OSError, ValueError) as error:
        print(f"cannot encode evidence block: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
