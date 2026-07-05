# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Entry point for the Tcl debugger.

Usage::

    python -m debugger script.tcl                 # auto-detect backend
    python -m debugger --backend vm script.tcl     # force VM backend
    python -m debugger --backend tclsh script.tcl  # force tclsh
    python -m debugger --backend tkinter script.tcl
    echo 'puts hi' | python -m debugger -         # read from stdin
"""

from __future__ import annotations

import argparse
import sys


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="python -m debugger",
        description="Tcl interactive debugger",
    )
    parser.add_argument(
        "script",
        help="Path to Tcl script to debug, or '-' for stdin",
    )
    parser.add_argument(
        "--backend",
        choices=["auto", "vm", "tclsh", "tkinter"],
        default="auto",
        help="Execution backend (default: auto-detect)",
    )
    args = parser.parse_args()

    # Read source
    script_path = args.script
    source: str | None = None

    if script_path == "-":
        source = sys.stdin.read()
        script_path = "<stdin>"
    else:
        try:
            with open(script_path, encoding="utf-8") as f:
                source = f.read()
        except FileNotFoundError:
            print(f"Error: file not found: {script_path}", file=sys.stderr)
            sys.exit(1)
        except OSError as exc:
            print(f"Error: {exc}", file=sys.stderr)
            sys.exit(1)

    # Create backend
    from tooling.debugger.backends import create_backend

    try:
        backend = create_backend(args.backend)
    except RuntimeError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        sys.exit(1)

    # Run the CLI debugger
    from tooling.debugger.cli import CliDebugger

    debugger = CliDebugger(backend)
    debugger.run(script_path, source=source)


if __name__ == "__main__":
    main()
