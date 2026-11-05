#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Extract the byte-exact script from a Tcl evidence Markdown file."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from tcl_evidence_format import script_from_markdown


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence", type=Path, help="evidence Markdown file")
    parser.add_argument(
        "-o",
        "--output",
        required=True,
        help="destination file, or - for standard output",
    )
    args = parser.parse_args()
    try:
        # Read bytes explicitly: universal-newline conversion would destroy CRLF.
        document = args.evidence.read_bytes().decode("utf-8")
        if "\r" in document:
            raise ValueError("evidence Markdown must use LF line endings")
        script = script_from_markdown(document, args.evidence)
        if args.output == "-":
            sys.stdout.buffer.write(script)
        else:
            Path(args.output).write_bytes(script)
    except (OSError, UnicodeError, ValueError) as error:
        print(f"cannot extract evidence script: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
