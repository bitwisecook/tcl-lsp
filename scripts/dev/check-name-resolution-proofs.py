#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Compatibility entry point for the Tcl execution evidence check."""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument(
        "--structure-only",
        action="store_true",
        help="Retired legacy option; use the Tcl evidence check instead.",
    )
    parser.add_argument(
        "--write-inventory",
        action="store_true",
        help="Retired legacy option; do not create digest inventories.",
    )
    args = parser.parse_args()
    if args.structure_only or args.write_inventory:
        parser.error(
            "the legacy proof inventory and structural audit are retired; use check-tcl-evidence.py"
        )
    return subprocess.run(
        [
            sys.executable,
            str(Path(__file__).with_name("check-tcl-evidence.py")),
            "--root",
            str(args.root.resolve()),
        ],
        check=False,
    ).returncode


if __name__ == "__main__":
    raise SystemExit(main())
