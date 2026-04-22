"""Diagnostic verbs: diag, lint, validate."""

from __future__ import annotations

import argparse
import json
import sys
from typing import Any, cast

from core.analysis.analyser import analyse
from core.analysis.semantic_model import Severity
from core.commands.registry.runtime import configure_signatures

from ._registry import verb
from ._utils import (
    _add_input_arguments,
    _add_toggle_arguments,
    _format_diagnostic_line,
    _read_input_documents,
    _resolve_disabled_diagnostics,
    _write_text_output,
)

_PROBLEM_SEVERITIES = frozenset({Severity.ERROR, Severity.WARNING})


@verb("diag", aliases=("diagnostics",), help="Run diagnostics across all resolved inputs.")
def _configure_diag(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    _add_input_arguments(p, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit diagnostics as JSON.",
    )
    _add_toggle_arguments(p, kind="diagnostic")
    p.set_defaults(handler=_run_diag)


@verb("lint", help="Run lint diagnostics across all resolved inputs.")
def _configure_lint(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    _add_input_arguments(p, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit diagnostics as JSON.",
    )
    _add_toggle_arguments(p, kind="diagnostic")
    p.set_defaults(handler=_run_diag)


@verb("validate", help="Validate source (error-level diagnostics only).")
def _configure_validate(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
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
