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

"""Console explorer for Tcl compiler and optimiser internals.

Features:
- Accept Tcl script from file, stdin, or --source.
- Show lowered IR, CFG (pre-SSA and post-SSA), and core-analysis facts by function.
- Show TclOO method bodies (IR view) and interprocedural procedure + method summaries.
- Show optimiser rewrites and optional optimised source.
- Render source with Rust-style caret/arrow callouts for salient compiler facts.
"""

from __future__ import annotations

import argparse
import json
import sys

from tooling.cli.formatters import LineIndex
from tooling.explorer._render import Ansi, load_source, style
from tooling.explorer.pipeline import AVAILABLE_DIALECTS, run_pipeline
from tooling.explorer.report import ExploreOptions, resolve_views, run_text_report

# Re-exported for the test-suite and the explorer's ``__main__`` shim, which
# reach these helpers through the CLI module rather than their canonical homes.
__all__ = [
    "Ansi",
    "LineIndex",
    "expand_show",
    "load_source",
    "main",
    "parse_args",
    "run_pipeline",
    "style",
]


def expand_show(raw: str) -> frozenset[str]:
    """Expand a comma-separated ``--show`` value into a set of view names.

    Thin argparse adapter over
    :func:`tooling.explorer.report.resolve_views`: it translates the
    library's :class:`ValueError` for an unknown view into an
    :class:`argparse.ArgumentTypeError`, keeping the shared library
    argparse-free.
    """
    try:
        return resolve_views(raw)
    except ValueError as exc:
        raise argparse.ArgumentTypeError(str(exc)) from exc


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Explore Tcl compiler and optimiser internals.\n\n"
            "Views: ir, cfg, ssa, interproc, types, opt, gvn, shimmer, taint, irules, callouts\n"
            "Groups: all (default), compiler, optimiser"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "path",
        nargs="?",
        help="Path to Tcl file to inspect, or '-' to read from stdin.",
    )
    parser.add_argument("--source", help="Inline Tcl source to inspect.")
    parser.add_argument(
        "--show",
        default="all",
        help=(
            "Comma-separated list of views to display. "
            "Individual: greentree, cst, segments, ir, cfg, ssa, interproc, types, opt, gvn, "
            "shimmer, taint, irules, callouts. "
            "Groups: all (default), compiler, optimiser. "
            "Example: --show cst,segments,ir"
        ),
    )
    parser.add_argument(
        "--focus",
        choices=("all", "compiler", "optimiser"),
        default=None,
        help="Shortcut for a common --show group (compiler / optimiser / all).",
    )
    parser.add_argument(
        "--dialect",
        choices=AVAILABLE_DIALECTS,
        default="tcl8.6",
        help="Tcl dialect profile to use for compilation (default: tcl8.6).",
    )
    parser.add_argument(
        "--show-optimised-source",
        action="store_true",
        help="Print line-numbered optimised source when rewrites are found.",
    )
    parser.add_argument(
        "--opt",
        choices=("off", "on", "diff"),
        default="off",
        help="Optimisation lens for the IR/CFG/SSA/ASM/WASM views: render the "
        "original path (off), the optimised path (on), or a diff of the two "
        "(diff).  Other views ignore it.",
    )
    parser.add_argument(
        "--no-source-callouts",
        action="store_true",
        help="Disable source callouts with caret/arrow annotations.",
    )
    parser.add_argument(
        "--max-annotations",
        type=int,
        default=80,
        help="Maximum number of source callouts to render (-1 for unlimited).",
    )
    parser.add_argument(
        "--no-colour",
        "--no-color",
        dest="no_colour",
        action="store_true",
        help="Disable ANSI colours.",
    )
    fmt = parser.add_mutually_exclusive_group()
    fmt.add_argument(
        "--tui",
        dest="format",
        action="store_const",
        const="tui",
        help="Live Textual UI (default on an interactive terminal).",
    )
    fmt.add_argument(
        "--text",
        dest="format",
        action="store_const",
        const="text",
        help="Flat scrolling text (default when piped / non-interactive).",
    )
    fmt.add_argument(
        "--json",
        dest="format",
        action="store_const",
        const="json",
        help="Machine-readable JSON (same serialisation as the web explorer).",
    )
    parser.set_defaults(format=None)
    args = parser.parse_args(argv)

    show_raw = args.focus if args.focus is not None else args.show
    try:
        args.views = expand_show(show_raw)
    except argparse.ArgumentTypeError as exc:
        parser.error(str(exc))
    return args


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)

    try:
        source = load_source(args.path, args.source)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    # Resolve output mode: explicit flag wins; otherwise TUI on an interactive
    # terminal (and only if Textual is installed), else flat text.
    mode = args.format
    if mode is None:
        mode = "tui" if sys.stdout.isatty() and _textual_available() else "text"

    if mode == "json":
        from tooling.cli.serialise import serialise_result

        try:
            result = run_pipeline(source, dialect=args.dialect)
        except Exception as exc:
            print(f"error: compiler exploration failed: {exc}", file=sys.stderr)
            return 2
        print(json.dumps(serialise_result(result), indent=2))
        return 0

    if mode == "tui":
        # Imported lazily: the TUI pulls in rich/textual, which are optional
        # extras not bundled into the zipapp build.  Keeping this out of module
        # import scope lets `tcl explore --text`/`--json` (and `tcl --help`,
        # which loads every verb) work without those packages installed.
        from tooling.explorer.tui import run_tui

        return run_tui(source, args)

    # text
    options = ExploreOptions(
        views=args.views,
        dialect=args.dialect,
        opt=args.opt,
        show_optimised_source=args.show_optimised_source,
        no_source_callouts=args.no_source_callouts,
        max_annotations=args.max_annotations,
        no_colour=args.no_colour,
    )
    return run_text_report(source, options)


def _textual_available() -> bool:
    try:
        import textual  # noqa: F401

        return True
    except ImportError:
        return False
