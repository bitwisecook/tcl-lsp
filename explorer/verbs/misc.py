"""Miscellaneous verbs: explore, convert."""

from __future__ import annotations

import argparse
import json

from core.analysis import analyse
from core.commands.registry.runtime import configure_signatures

from ..cli import main as explorer_main
from ._registry import verb
from ._utils import (
    _add_input_arguments,
    _combine_sources,
    _read_input_documents,
    _write_text_output,
)

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


@verb("explore", help="Run compiler-explorer views on aggregated input.")
def _configure_explore(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:
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


@verb("convert", help="Detect legacy patterns eligible for modernisation.")
def _configure_convert(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit convert findings as JSON.",
    )
    p.set_defaults(handler=_run_convert)


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


def _run_convert(args: argparse.Namespace) -> int:
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
