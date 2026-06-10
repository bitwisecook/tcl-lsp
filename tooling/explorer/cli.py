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

try:
    from shared._build_info import BUILD_TIMESTAMP, FULL_VERSION
except ImportError:
    FULL_VERSION = "dev"
    BUILD_TIMESTAMP = ""

from tooling.cli.formatters import LineIndex
from tooling.explorer._render import (
    _VIEW_ORDER,
    Ansi,
    _summary_parts,
    load_source,
    optimised_result,
    render_view_opt,
    style,
)
from tooling.explorer.pipeline import (
    ALL_VIEWS,
    AVAILABLE_DIALECTS,
    VIEW_GROUPS,
    CompilerExplorerResult,
    run_pipeline,
)


def expand_show(raw: str) -> frozenset[str]:
    """Expand a comma-separated ``--show`` value into a set of view names.

    Lives in the CLI adapter (not the pipeline library) because it raises
    :class:`argparse.ArgumentTypeError` for an unknown view — argparse is
    an adapter concern, so the pipeline module stays argparse-free.
    """
    views: set[str] = set()
    for token in raw.split(","):
        token = token.strip()
        if not token:
            continue
        if token in VIEW_GROUPS:
            views |= VIEW_GROUPS[token]
        elif token in ALL_VIEWS:
            views.add(token)
        else:
            raise argparse.ArgumentTypeError(
                f"unknown view {token!r}; choose from: "
                f"{', '.join(sorted(ALL_VIEWS | set(VIEW_GROUPS)))}"
            )
    return frozenset(views)


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


def _render_text(
    result: CompilerExplorerResult, source: str, args: argparse.Namespace, *, use_colour: bool
) -> int:
    line_index = LineIndex(source)
    _version = FULL_VERSION + (f" ({BUILD_TIMESTAMP})" if BUILD_TIMESTAMP else "")
    print(style(f"compiler-optimiser-explorer {_version}", Ansi.BOLD, use_colour))
    print(style(" ".join(_summary_parts(result, args.dialect)), Ansi.DIM, use_colour))
    opt_label = "" if args.opt == "off" else f"  opt={args.opt}"
    print(style(f"views: {','.join(sorted(args.views))}{opt_label}", Ansi.DIM, use_colour))
    print()
    opt = optimised_result(result, args.dialect) if args.opt != "off" else None
    for view in _VIEW_ORDER:
        if view not in args.views:
            continue
        if view == "callouts" and args.no_source_callouts:
            continue
        render_view_opt(
            view,
            result,
            source,
            use_colour=use_colour,
            line_index=line_index,
            opt_mode=args.opt,
            opt=opt,
            views=args.views,
            show_optimised_source=args.show_optimised_source,
            max_annotations=args.max_annotations,
        )
        print()
    return 0


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
    use_colour = (not args.no_colour) and sys.stdout.isatty()
    try:
        result = run_pipeline(source, dialect=args.dialect)
    except Exception as exc:
        print(f"error: compiler exploration failed: {exc}", file=sys.stderr)
        return 2
    return _render_text(result, source, args, use_colour=use_colour)


def _textual_available() -> bool:
    try:
        import textual  # noqa: F401

        return True
    except ImportError:
        return False
