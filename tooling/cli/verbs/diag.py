"""Diagnostic verbs: diag, lint, validate."""

from __future__ import annotations

import argparse
import json
import sys
from typing import Any, cast

from analyser import analyse
from analyser.semantic_model import Severity
from compiler.registry.runtime import configure_signatures

from ._registry import verb
from ._utils import (
    _add_input_arguments,
    _add_toggle_arguments,
    _format_diagnostic_line,
    _read_input_documents,
    _resolve_disabled_diagnostics,
)

_PROBLEM_SEVERITIES = frozenset({Severity.ERROR, Severity.WARNING})


@verb(
    "diag",
    aliases=("diagnostics",),
    help="Run diagnostics across all resolved inputs.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_diag(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:
    p.description = (
        "Run the analyser over each resolved input and print every emitted\n"
        "diagnostic (errors, warnings, and info notes).  Exit code is 1 if\n"
        "any error- or warning-level diagnostic is reported, 0 otherwise.\n"
        "Use --disable / --enable to toggle individual codes (e.g. W100,W110).\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} diag script.tcl\n"
        f"  {prog_name} diag src/ --dialect f5-irules\n"
        f"  {prog_name} diag script.tcl --json\n"
        f"  {prog_name} diag script.tcl --disable W100,W110\n"
    )
    _add_input_arguments(p, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit diagnostics as JSON.",
    )
    _add_toggle_arguments(p, kind="diagnostic")
    p.set_defaults(handler=_run_diag)


@verb(
    "lint",
    help="Run lint diagnostics across all resolved inputs.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_lint(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:
    p.description = (
        "Alias-flavour of `diag`: runs the same analyser and emits the same\n"
        "diagnostic set, kept as a separate verb so it slots into editors,\n"
        "CI gates, and habit memory that expect a `lint` command.  Use the\n"
        "same --disable / --enable / --json switches.\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} lint script.tcl\n"
        f"  {prog_name} lint src/ --dialect f5-irules --json\n"
    )
    _add_input_arguments(p, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit diagnostics as JSON.",
    )
    _add_toggle_arguments(p, kind="diagnostic")
    p.set_defaults(handler=_run_diag)


@verb(
    "validate",
    help="Validate source (error-level diagnostics only).",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_validate(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    p.description = (
        "Stricter cousin of `diag`: only error-severity diagnostics are\n"
        "reported, and the exit code is non-zero whenever any error is\n"
        "found.  Intended for CI gates that want to fail fast on syntax /\n"
        "structural problems without being noisy about style warnings.\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} validate script.tcl\n"
        f"  {prog_name} validate src/ --json\n"
        f"  {prog_name} validate src/ --dialect f5-irules\n"
    )
    _add_input_arguments(p, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit validation results as JSON.",
    )
    _add_toggle_arguments(p, kind="diagnostic")
    p.set_defaults(handler=_run_validate)


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def _run_diag(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    report: list[dict[str, Any]] = []
    problem_count = 0
    diagnostic_count = 0

    disabled = _resolve_disabled_diagnostics(args)

    for document in documents:
        diagnostics = [d for d in analyse(document.source).diagnostics if d.code not in disabled]
        if diagnostics:
            diagnostic_count += len(diagnostics)
            problem_count += sum(1 for diag in diagnostics if diag.severity in _PROBLEM_SEVERITIES)
        report.append(
            {
                "file": document.label,
                "diagnostics": [
                    {
                        "line": diag.range.start.line + 1,
                        "column": diag.range.start.character + 1,
                        "severity": diag.severity.name.lower(),
                        "code": diag.code or "",
                        "message": diag.message,
                    }
                    for diag in diagnostics
                ],
            }
        )

    if args.json:
        print(json.dumps(report, indent=2))
    else:
        for item in report:
            diagnostics = cast(list[dict[str, Any]], item["diagnostics"])
            for diag in diagnostics:
                print(
                    f"{item['file']}:{diag['line']}:{diag['column']}: "
                    f"{diag['severity']:<7} {diag['code'] or '-':<8} {diag['message']}"
                )
        if diagnostic_count == 0:
            print("no diagnostics")

    print(
        f"diagnostics={diagnostic_count} across {len(documents)} input(s)",
        file=sys.stderr,
    )
    return 1 if problem_count else 0


def _run_validate(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    disabled = _resolve_disabled_diagnostics(args)
    errors = []
    for document in documents:
        diagnostics = analyse(document.source).diagnostics
        errors.extend(
            (document, diag)
            for diag in diagnostics
            if diag.severity is Severity.ERROR and diag.code not in disabled
        )

    if args.json:
        payload = {
            "ok": not errors,
            "inputs": len(documents),
            "error_count": len(errors),
            "errors": [
                {
                    "file": document.label,
                    "line": diagnostic.range.start.line + 1,
                    "column": diagnostic.range.start.character + 1,
                    "severity": diagnostic.severity.name.lower(),
                    "code": diagnostic.code or "",
                    "message": diagnostic.message,
                }
                for document, diagnostic in errors
            ],
        }
        print(json.dumps(payload, indent=2))
        return 1 if errors else 0

    if errors:
        for document, diagnostic in errors:
            print(_format_diagnostic_line(document, diagnostic))
        print(f"validation failed: {len(errors)} error(s)", file=sys.stderr)
        return 1

    print("validation ok", file=sys.stderr)
    return 0
