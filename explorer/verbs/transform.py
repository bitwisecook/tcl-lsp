"""Source-transformation verbs: opt, format, minify, unminify-error."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.commands.registry.runtime import configure_signatures
from core.compiler.optimiser import apply_optimisations, find_optimisations
from core.formatting import format_tcl

from ._registry import verb
from ._utils import (
    _add_colour_arguments,
    _add_formatter_arguments,
    _add_input_arguments,
    _add_toggle_arguments,
    _combine_sources,
    _read_input_documents,
    _resolve_disabled_optimisations,
    _resolve_formatter_config,
    _resolve_tab_width,
    _resolve_use_colour,
    _write_highlighted_output,
    _write_text_output,
)


@verb(
    "opt",
    aliases=("optimise", "optimize"),
    help="Optimise source and emit rewritten Tcl.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_opt(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:
    p.description = (
        "Run the optimiser over each input and write the rewritten source.\n"
        "Applies the codes selected by --profile (default: full): constant\n"
        "folding, dead-code elimination, redundant-load removal, switch\n"
        "lowering, etc.  Each rewrite is a known, locally-verifiable\n"
        "transform — see the O100-O126 catalogue.  Use --disable to drop\n"
        "specific codes, --enable to bring back ones disabled in config.\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} opt script.tcl\n"
        f"  {prog_name} opt script.tcl --profile standard -o opt.tcl\n"
        f"  {prog_name} opt src/ --dialect f5-irules --disable O108,O115\n"
        f"  {prog_name} opt script.tcl --profile aggressive\n"
        "\n"
        "Profiles: off, readability, standard, full (default), aggressive\n"
    )
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    _add_colour_arguments(p)
    p.add_argument(
        "--profile",
        default=None,
        choices=["off", "readability", "standard", "full", "aggressive"],
        help="Optimisation profile (default: full). Overrides config file.",
    )
    _add_toggle_arguments(p, kind="optimisation")
    p.set_defaults(handler=_run_opt)


@verb(
    "format",
    aliases=("fmt",),
    help="Format source and emit canonical rewritten Tcl.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_format(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:
    p.description = (
        "Pretty-print each input with the canonical style rules: consistent\n"
        "indent, balanced brace placement, optional brace-body expansion,\n"
        "and configurable line-length goal / hard limit.  Style knobs are\n"
        "overridable per-invocation (--indent-size, --indent-style, ...) or\n"
        "via the [formatter] section in `~/.config/tcl.ini`.\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} format script.tcl\n"
        f"  {prog_name} format src/ --dialect f5-irules -o formatted.tcl\n"
        f"  {prog_name} format script.tcl --indent-size 2 --indent-style spaces\n"
        f"  {prog_name} format script.tcl --max-line-length 100 --expand-bodies\n"
    )
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    _add_colour_arguments(p)
    _add_formatter_arguments(p)
    p.set_defaults(handler=_run_format)


@verb(
    "minify",
    aliases=("min",),
    help="Minify source: strip comments, collapse whitespace, join commands.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_minify(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:
    p.description = (
        "Minify Tcl source code: strip comments, collapse whitespace, "
        "join commands with semicolons."
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} minify script.tcl\n"
        f"  {prog_name} minify script.tcl -o minified.tcl\n"
        f"  {prog_name} minify --compact script.tcl --symbol-map symbols.txt\n"
        f"  {prog_name} minify --aggressive script.tcl -o tiny.tcl\n"
        f"  {prog_name} minify src/ --dialect f5-irules\n"
    )
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--compact",
        action="store_true",
        help="Compact variable and proc names to short identifiers.",
    )
    p.add_argument(
        "--symbol-map",
        metavar="FILE",
        default=None,
        help="Write symbol map (original -> compacted names) to FILE.",
    )
    p.add_argument(
        "--aggressive",
        action="store_true",
        help="Maximum compression: run all optimiser passes, then compact names and minify.",
    )
    p.add_argument(
        "--isolated",
        action="store_true",
        help="Treat script as self-contained — also compact global-scope variable names.",
    )
    _add_colour_arguments(p)
    p.set_defaults(handler=_run_minify)


@verb(
    "unminify-error",
    aliases=("umerr",),
    help="Translate a minified-code error message back to original names.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_unminify_error(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    p.description = (
        "Translate Tcl or iRule error messages produced by minified code "
        "back to the original variable, proc, and command names using a "
        "saved symbol map."
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} unminify-error --symbol-map map.txt --error 'can\\'t read \"a\": no such variable'\n"
        f"  {prog_name} unminify-error --symbol-map map.txt --error-file /var/log/ltm\n"
        f"  {prog_name} unminify-error --symbol-map map.txt --minified min.tcl --original src.tcl --error-file err.log\n"
    )
    p.add_argument(
        "--symbol-map",
        metavar="FILE",
        required=True,
        help="Path to the symbol map file produced by minify --symbol-map.",
    )
    p.add_argument(
        "--error",
        "-e",
        metavar="TEXT",
        help="Error message text to translate (inline).",
    )
    p.add_argument(
        "--error-file",
        metavar="FILE",
        help="File containing error messages to translate ('-' for stdin).",
    )
    p.add_argument(
        "--minified",
        metavar="FILE",
        help="The minified source file (for line-number remapping).",
    )
    p.add_argument(
        "--original",
        metavar="FILE",
        help="The original source file (for line-number remapping).",
    )
    p.add_argument(
        "--output",
        "-o",
        default="-",
        help="Output path ('-' for stdout).",
    )
    p.set_defaults(handler=_run_unminify_error)


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def _run_opt(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    disabled, multi_pass, max_iterations = _resolve_disabled_optimisations(args)
    if multi_pass:
        from core.compiler.optimiser import optimise_source_multipass

        optimised_source, optimisations, _iters = optimise_source_multipass(
            source,
            max_iterations=max_iterations,
            disabled=frozenset(disabled),
        )
    else:
        all_optimisations = find_optimisations(source)
        optimisations = [o for o in all_optimisations if o.code not in disabled]
        optimised_source = apply_optimisations(source, optimisations)

    if args.output == "-" and optimisations:
        lines = [
            "\n\n# -------------",
            f"# optimised: {len(optimisations)} rewrite(s)",
        ]
        for opt in optimisations:
            lines.append(f"# {opt.code}  {opt.message}")
        optimised_source = optimised_source.rstrip("\n") + "\n" + "\n".join(lines) + "\n"

    _write_highlighted_output(
        args.output,
        optimised_source,
        use_colour=_resolve_use_colour(args),
        tab_width=_resolve_tab_width(args),
    )

    if args.output != "-":
        print(
            f"optimised {len(documents)} input(s); rewrites={len(optimisations)}",
            file=sys.stderr,
        )
    return 0


def _run_format(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    fmt_cfg = _resolve_formatter_config(args)
    formatted_source = format_tcl(source, config=fmt_cfg)
    _write_highlighted_output(
        args.output,
        formatted_source,
        use_colour=_resolve_use_colour(args),
        tab_width=_resolve_tab_width(args),
    )
    return 0


def _run_minify(args: argparse.Namespace) -> int:
    from core.minifier import minify_tcl

    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    source = _combine_sources(documents)
    use_colour = _resolve_use_colour(args)
    isolated = getattr(args, "isolated", False)

    if args.aggressive:
        result = minify_tcl(source, aggressive=True, isolated=isolated)
        _write_highlighted_output(
            args.output, result.source, use_colour=use_colour, tab_width=_resolve_tab_width(args)
        )
        if args.symbol_map:
            _write_text_output(args.symbol_map, result.symbol_map.format())
        if sys.stderr.isatty():
            pct = f"{result.savings_pct:.1f}"
            print(
                f"{result.original_length} → {result.minified_length} chars "
                f"({pct}% reduction, {result.optimisations_applied} optimisations)",
                file=sys.stderr,
            )
    elif args.compact:
        minified, symbol_map = minify_tcl(source, compact_names=True, isolated=isolated)
        _write_highlighted_output(
            args.output, minified, use_colour=use_colour, tab_width=_resolve_tab_width(args)
        )
        if args.symbol_map:
            _write_text_output(args.symbol_map, symbol_map.format())
        elif sys.stderr.isatty():
            map_text = symbol_map.format()
            if map_text:
                print(map_text, file=sys.stderr)
    else:
        minified = minify_tcl(source)
        _write_highlighted_output(
            args.output, minified, use_colour=use_colour, tab_width=_resolve_tab_width(args)
        )
    return 0


def _run_unminify_error(args: argparse.Namespace) -> int:
    from core.minifier import unminify_error

    symbol_map_text = Path(args.symbol_map).read_text(encoding="utf-8")

    if args.error:
        error_text = args.error
    elif args.error_file:
        if args.error_file == "-":
            error_text = sys.stdin.read()
        else:
            error_text = Path(args.error_file).read_text(encoding="utf-8")
    else:
        print("error: provide --error TEXT or --error-file FILE", file=sys.stderr)
        return 1

    minified_source = None
    original_source = None
    if args.minified:
        minified_source = Path(args.minified).read_text(encoding="utf-8")
    if args.original:
        original_source = Path(args.original).read_text(encoding="utf-8")

    translated = unminify_error(
        error_text,
        symbol_map=symbol_map_text,
        minified_source=minified_source,
        original_source=original_source,
    )
    _write_text_output(args.output, translated)
    return 0
