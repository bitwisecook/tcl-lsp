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
    from debugger.backends import create_backend

    try:
        backend = create_backend(args.backend)
    except RuntimeError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        sys.exit(1)

    # Run the CLI debugger
    from debugger.cli import CliDebugger

    debugger = CliDebugger(backend)
    debugger.run(script_path, source=source)


if __name__ == "__main__":
    main()
