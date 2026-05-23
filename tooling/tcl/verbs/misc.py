"""Miscellaneous verbs: explore, find-legacy."""

from __future__ import annotations

import argparse
import json

from analyser import analyse
from compiler.registry.runtime import configure_signatures
from tooling.cli._utils import (
    _add_input_arguments,
    _combine_sources,
    _read_input_documents,
    _write_text_output,
)
from tooling.explorer.cli import main as explorer_main

from ._registry import verb

_CONVERTIBLE_CODES = frozenset(
    {
        "W100",
        "W104",
        "W110",
        "W304",
        "IRULE2001",
        "IRULE5001",
    }
)
_CONVERSION_MAP: dict[str, str] = {
    "W100": "Unbraced expr -> braced expr",
    "W104": "String concat for lists -> lappend",
    "W110": "== / != for strings -> eq / ne",
    "W304": "Missing -- option terminator -> add --",
    "IRULE2001": "Deprecated matchclass -> class match",
    "IRULE5001": "Ungated log in hot event -> add debug gating",
}


@verb(
    "explore",
    help="Run compiler-explorer views on aggregated input.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_explore(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:
    p.description = (
        "Render compiler-explorer views (IR, CFG, SSA, interprocedural\n"
        "summaries, optimiser rewrites, type inference, taint analysis,\n"
        "GVN, shimmer, codegen, WASM, ...) for the resolved input.  Useful\n"
        "for inspecting what the compiler sees and why a particular\n"
        "optimisation fired (or didn't).  Run `tcl explore --show all` for\n"
        "every view, or pick specific ones (e.g. --show 'cfg,opt,asm').\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} explore script.tcl\n"
        f"  {prog_name} explore script.tcl --show cfg,ssa,opt\n"
        f"  {prog_name} explore src/ --show all --show-optimised-source\n"
        f"  {prog_name} explore script.tcl --show asm,wasm --no-colour\n"
        f"  {prog_name} explore script.tcl --max-annotations 200\n"
        "\n"
        "Views: ir, cfg, ssa, interproc, types, opt, gvn, shimmer, taint,\n"
        "       irules, callouts, asm, wasm, all, compiler, optimiser\n"
    )
    _add_input_arguments(p, default_dialect=default_dialect)
    p.add_argument(
        "--show",
        default="all",
        help="Compiler explorer views to show (default: all).",
    )
    p.add_argument(
        "--show-optimised-source",
        action="store_true",
        help="Include line-numbered optimised source output.",
    )
    p.add_argument(
        "--no-source-callouts",
        action="store_true",
        help="Disable source callout rendering.",
    )
    p.add_argument(
        "--max-annotations",
        type=int,
        default=80,
        help="Maximum source callouts to render (-1 for unlimited).",
    )
    p.add_argument(
        "--no-colour",
        "--no-color",
        dest="no_colour",
        action="store_true",
        help="Disable ANSI colour output.",
    )
    p.set_defaults(handler=_run_explore)


@verb(
    "find-legacy",
    help="Report legacy patterns eligible for modernisation (detection only).",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_find_legacy(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    p.description = (
        "Scan source for legacy patterns that have a known mechanical\n"
        "modernisation.  This verb only *reports* the findings — it does\n"
        "not modify source.  Use `tcl opt` to actually rewrite.\n"
        "\n"
        "Detected patterns:\n"
        "  W100        Unbraced expr            -> braced expr\n"
        "  W104        String concat for lists  -> lappend\n"
        "  W110        == / != for strings      -> eq / ne\n"
        "  W304        Missing -- terminator    -> add --\n"
        "  IRULE2001   Deprecated matchclass    -> class match\n"
        "  IRULE5001   Ungated log in hot event -> guard with [log level ...]\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} find-legacy old_irule.irul\n"
        f"  {prog_name} find-legacy src/ --dialect f5-irules\n"
        f"  {prog_name} find-legacy script.tcl --json -o report.json\n"
    )
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit findings as JSON (count, dialect, issues[]).",
    )
    p.set_defaults(handler=_run_find_legacy)


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def _run_explore(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    source = _combine_sources(documents)

    explorer_args = [
        "--source",
        source,
        "--show",
        args.show,
        "--dialect",
        args.dialect,
        "--max-annotations",
        str(args.max_annotations),
    ]
    if args.show_optimised_source:
        explorer_args.append("--show-optimised-source")
    if args.no_source_callouts:
        explorer_args.append("--no-source-callouts")
    if args.no_colour:
        explorer_args.append("--no-colour")

    return explorer_main(explorer_args)


def _run_find_legacy(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)

    diagnostics = analyse(source).diagnostics
    convertible = [diag for diag in diagnostics if (diag.code or "") in _CONVERTIBLE_CODES]
    issues = [
        {
            "code": diag.code or "",
            "line": diag.range.start.line + 1,
            "column": diag.range.start.character + 1,
            "message": diag.message,
            "conversion": _CONVERSION_MAP.get(diag.code or "", "modernise"),
        }
        for diag in convertible
    ]
    payload = {
        "count": len(issues),
        "dialect": args.dialect,
        "issues": issues,
    }
    if args.json:
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 0

    if not issues:
        _write_text_output(args.output, "no legacy patterns detected")
        return 0

    lines = [f"legacy patterns: {len(issues)}"]
    for issue in issues:
        lines.append(f"  {issue['code']} line {issue['line']}:{issue['column']} {issue['message']}")
        lines.append(f"    conversion: {issue['conversion']}")
    _write_text_output(args.output, "\n".join(lines))
    return 0
