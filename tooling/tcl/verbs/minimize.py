"""Minimize verb: reduce a diagnostic to a minimal reproducer.

``tcl minimize script.tcl W220`` prints the smallest self-contained snippet
from ``script.tcl`` that still fires ``W220``, with identifiers renamed to short
``a b c …`` names — ready to paste into a bug report.
"""

from __future__ import annotations

import argparse
import json
import sys

from compiler.registry.runtime import configure_signatures
from tooling.cli._utils import _add_input_arguments, _read_input_documents

from ._registry import verb


@verb(
    "minimize",
    aliases=("minimise", "repro"),
    help="Reduce a diagnostic to a minimal reproducer for bug reports.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_minimize(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    p.description = (
        "Delta-debug the input down to the minimum self-contained code that\n"
        "still fires the given diagnostic CODE, then rename identifiers to\n"
        "short a/b/c names.  Every reduction/rename is verified to still\n"
        "reproduce, so the output always fires CODE — paste it into an issue.\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} minimize script.tcl W220\n"
        f"  {prog_name} minimize script.tcl W210 --no-rename\n"
        f"  {prog_name} minimize script.tcl W307 --json\n"
    )
    _add_input_arguments(p, default_dialect=default_dialect)
    p.add_argument("code", help="Diagnostic code to minimise a reproducer for (e.g. W220).")
    p.add_argument(
        "--no-rename",
        action="store_true",
        help="Keep original identifier names (skip the a/b/c rename pass).",
    )
    p.add_argument("--json", action="store_true", help="Emit the result as JSON.")
    p.set_defaults(handler=_run_minimize)


def _run_minimize(args: argparse.Namespace) -> int:
    from server.features.minimize import minimize_diagnostic

    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)

    code = args.code
    exit_code = 0
    results = []
    for document in documents:
        try:
            r = minimize_diagnostic(document.source, code, rename=not args.no_rename)
        except ValueError:
            # Code doesn't fire on this document — skip it.
            continue
        results.append(
            {
                "file": document.label,
                "code": r.code,
                "original_lines": r.original_lines,
                "reduced_lines": r.reduced_lines,
                "renamed": r.renamed,
                "reproduces": r.reproduces,
                "source": r.source,
            }
        )

    if not results:
        print(f"{code} does not fire on any input.", file=sys.stderr)
        return 1

    if args.json:
        print(json.dumps(results, indent=2))
        return exit_code

    for res in results:
        print(
            f"# {res['file']}: {res['code']} "
            f"({res['original_lines']}→{res['reduced_lines']} lines, "
            f"renamed={res['renamed']})"
        )
        print(res["source"])
        print()
    return exit_code
