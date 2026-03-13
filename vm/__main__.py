"""Entry point for ``python -m vm``."""

from __future__ import annotations

import argparse
import sys

from core.compiler.codegen import format_module_asm

from .compiler import compile_script
from .interp import TclInterp
from .machine import _list_escape
from .repl import run_repl
from .types import TclError, TclReturn


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="tclvm",
        description="Tcl bytecode virtual machine",
    )
    parser.add_argument("script", nargs="?", help="Tcl script file to execute")
    parser.add_argument("-e", "--eval", dest="inline", help="evaluate inline script")
    parser.add_argument(
        "--disassemble",
        action="store_true",
        help="show bytecode disassembly instead of executing",
    )
    parser.add_argument(
        "--no-curses",
        action="store_true",
        help="force readline REPL (no curses)",
    )
    parser.add_argument(
        "--optimise",
        action="store_true",
        help="enable optimisation passes",
    )

    # Use parse_known_args so extra arguments pass through to the
    # Tcl script (available via $argc / $argv).
    args, script_args = parser.parse_known_args()
    interp = TclInterp(optimise=args.optimise, source_init=True)

    # Disassemble mode
    if args.disassemble:
        source = _get_source(args)
        if source is None:
            parser.error("--disassemble requires a script or -e argument")
        module_asm, _ir = compile_script(source, optimise=args.optimise)
        print(format_module_asm(module_asm))
        return

    # Inline eval
    if args.inline is not None:
        _setup_argv(interp, script_args)
        _run(interp, args.inline)
        return

    # Script file
    if args.script is not None:
        try:
            with open(args.script) as f:
                source = f.read()
        except OSError as e:
            print(f"error: {e}", file=sys.stderr)
            sys.exit(1)
        interp.script_file = args.script
        interp.global_frame.set_var("argv0", args.script)
        _setup_argv(interp, script_args)
        _run(interp, source)
        return

    # Interactive REPL
    run_repl(interp, use_curses=not args.no_curses)


def _setup_argv(interp: TclInterp, script_args: list[str]) -> None:
    """Set ``$argc`` and ``$argv`` from remaining command-line arguments."""
    interp.global_frame.set_var("argc", str(len(script_args)))
    interp.global_frame.set_var("argv", " ".join(_list_escape(a) for a in script_args))


def _get_source(args: argparse.Namespace) -> str | None:
    if args.inline is not None:
        return args.inline
    if args.script is not None:
        try:
            with open(args.script) as f:
                return f.read()
        except OSError:
            return None
    return None


def _run(interp: TclInterp, source: str) -> None:
    try:
        interp.eval(source)
        # In non-interactive mode, don't print the result
        # (like tclsh — only puts prints)
    except TclError as e:
        print(f"error: {e.message}", file=sys.stderr)
        sys.exit(1)
    except TclReturn:
        pass
    except SystemExit as e:
        sys.exit(e.code)


if __name__ == "__main__":
    main()
