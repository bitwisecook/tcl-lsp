#!/usr/bin/env python3
"""Filter README.md for single-editor distribution.

Strips content between ``<!-- BEGIN:multi-editor -->`` and
``<!-- END:multi-editor -->`` markers, and applies inline replacements
from ``<!-- BEGIN:vscode-replace -->`` blocks.

Usage::

    python scripts/filter_readme.py README.md -o build/README.md
    python scripts/filter_readme.py README.md   # prints to stdout
"""

from __future__ import annotations

import argparse
import re
import sys


def filter_readme(text: str) -> str:
    """Remove multi-editor blocks and apply vscode-replace substitutions."""

    # 1. Strip multi-editor blocks (may span multiple lines).
    text = re.sub(
        r"<!-- BEGIN:multi-editor -->.*?<!-- END:multi-editor -->\n?",
        "",
        text,
        flags=re.DOTALL,
    )

    # 2. Apply vscode-replace inline substitutions.
    #    Pattern: <!-- BEGIN:vscode-replace -->ORIGINAL<!-- REPLACE-WITH:REPLACEMENT --><!-- END:vscode-replace -->
    text = re.sub(
        r"<!-- BEGIN:vscode-replace -->.*?<!-- REPLACE-WITH:(.*?) --><!-- END:vscode-replace -->",
        r"\1",
        text,
        flags=re.DOTALL,
    )

    # 3. Collapse runs of 3+ blank lines to 2 (single visual gap).
    text = re.sub(r"\n{3,}", "\n\n", text)

    return text


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", help="Input README.md path")
    parser.add_argument("-o", "--output", help="Output path (default: stdout)")
    args = parser.parse_args()

    with open(args.input, encoding="utf-8") as f:
        text = f.read()

    result = filter_readme(text)

    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(result)
    else:
        sys.stdout.write(result)


if __name__ == "__main__":
    main()
