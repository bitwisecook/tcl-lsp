"""``f5 query`` — jq-flavoured DSL for inspecting and rewriting BIG-IP configs.

The grammar, value model, and builtin library all live in
:mod:`core.bigip.query`; this module is the argparse plumbing and the
custom help actions that surface the DSL reference, builtin catalogue,
and worked-example cookbook on the terminal.

Help layers:

- ``--help`` (the default) — verb summary, flag table, and pointers to
  the deeper help.  ``RawDescriptionHelpFormatter`` keeps the example
  block readable.
- ``--help-dsl`` — prints :func:`core.bigip.query.format_grammar`, the
  same grammar reference that lives in ``docs/design/f5-query-dsl.md``.
- ``--help-builtins [NAME]`` — full builtin catalogue, or just one
  function when a name is given.  Generated from the same registry the
  evaluator dispatches against, so docs and code cannot drift.
- ``--help-examples`` — :func:`core.bigip.query.format_examples` —
  a cookbook of common one-liners.  Every example is also exercised
  by ``tests/test_f5_query.py`` so a broken example fails CI before it
  ever ships.
"""

from __future__ import annotations

import argparse
import difflib
import sys
from pathlib import Path

from core.bigip.query import (
    format_builtins,
    format_examples,
    format_grammar,
    run_query,
)
from core.bigip.query.errors import QueryError
from core.bigip.query.output import render

from ._emit import add_format_arg, render_config
from ._paths import read_path
from ._registry import verb

_DESCRIPTION = (
    "Inspect and rewrite BIG-IP configuration with a small jq-flavoured "
    "DSL.  Queries navigate the parsed object tree (``.ltm.virtual[]``, "
    '``.ltm.pool["/Common/web_pool"]``, ``.ltm.rule[]``), filter with '
    "``select(...)``, project fields, and — with ``=`` / ``|=`` / "
    "``+=`` / ``-=`` — rewrite matched values.  Identity-field "
    "writes auto-route through the same engine ``f5 rename`` uses, so "
    "renaming a pool also moves every reference to it.\n"
    "\n"
    "Default behaviour is a dry-run preview: a unified diff for "
    "mutating queries, or the projected values for read-only ones.  "
    "Pass ``--write`` to print the rewritten config to stdout, "
    "``--in-place`` to overwrite the input.\n"
)


_EPILOG = (
    "Examples:\n"
    "  # List every VS's default pool\n"
    "  f5 query '.ltm.virtual[] | .pool' bigip.conf\n"
    "\n"
    "  # VSes whose pool member is in 10.0.0.0/8\n"
    "  f5 query '.ltm.virtual[] | select(any(.pool.members[].address "
    '| in_cidr(., "10.0.0.0/8"))) | .name\' bigip.conf\n'
    "\n"
    "  # Readdress every VS into 192.168.9.0/24 (dry-run diff)\n"
    "  f5 query '.ltm.virtual[] | .destination |= ip(\"192.168.9.0/24\", .)' "
    "bigip.conf\n"
    "\n"
    "  # Rename a pool everywhere (header + references)\n"
    '  f5 query \'.ltm.pool["/Common/old"].name = "/Common/new"\' '
    "--write bigip.conf > new.conf\n"
    "\n"
    "Run --help-dsl for the grammar, --help-builtins for the function\n"
    "catalogue, and --help-examples for a longer cookbook.\n"
)


class _HelpDslAction(argparse.Action):
    """Print the DSL grammar reference and exit."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        sys.stdout.write(format_grammar())
        parser.exit()


class _HelpBuiltinsAction(argparse.Action):
    """Print the builtin catalogue (optionally narrowed to one name)."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        name = values if isinstance(values, str) and values else None
        sys.stdout.write(format_builtins(name))
        parser.exit()


class _HelpExamplesAction(argparse.Action):
    """Print the cookbook of worked examples."""

    def __call__(self, parser, namespace, values, option_string=None):  # noqa: ARG002
        sys.stdout.write(format_examples())
        parser.exit()


@verb(
    "query",
    aliases=("q",),
    help="jq-flavoured DSL for inspecting and rewriting BIG-IP configs.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = _DESCRIPTION
    p.epilog = _EPILOG

    p.add_argument(
        "expression",
        nargs="?",
        help=("Query expression.  Use ``-f FILE`` to read a multi-line query from a file instead."),
    )
    p.add_argument(
        "paths",
        nargs="*",
        help=(
            "bigip.conf / SCF files (one or more).  Pass ``-`` to read a single config from stdin."
        ),
    )
    p.add_argument(
        "-f",
        "--from-file",
        metavar="FILE",
        help=(
            "Read the query expression from FILE.  Mutually exclusive "
            "with passing the expression as the first positional argument."
        ),
    )

    out_group = p.add_mutually_exclusive_group()
    out_group.add_argument(
        "--scf",
        dest="output_mode",
        action="store_const",
        const="scf",
        help=(
            "Render every selected value as an SCF stanza when possible.  "
            "Useful for ``f5 query ... | f5 cleanup``-style pipelines."
        ),
    )
    out_group.add_argument(
        "--raw",
        dest="output_mode",
        action="store_const",
        const="raw",
        help="Render scalar values one per line, no quoting.",
    )
    out_group.add_argument(
        "--paths-only",
        dest="output_mode",
        action="store_const",
        const="paths",
        help="Print only the full-path of each object / reference produced.",
    )
    out_group.add_argument(
        "--json",
        dest="output_mode",
        action="store_const",
        const="json",
        help="Render the result as a JSON array.",
    )

    write_group = p.add_mutually_exclusive_group()
    write_group.add_argument(
        "--write",
        action="store_true",
        help=(
            "When the query mutates, print the rewritten config to stdout "
            "(default: print a unified-diff preview)."
        ),
    )
    write_group.add_argument(
        "--in-place",
        action="store_true",
        help=("When the query mutates, overwrite each input file with the rewritten config."),
    )

    add_format_arg(p, tmsh_default_verb="modify")

    p.add_argument(
        "--help-dsl",
        nargs=0,
        action=_HelpDslAction,
        default=argparse.SUPPRESS,
        help="Show the DSL grammar reference and exit.",
    )
    p.add_argument(
        "--help-builtins",
        nargs="?",
        const="",
        metavar="NAME",
        action=_HelpBuiltinsAction,
        default=argparse.SUPPRESS,
        help=(
            "Show the catalogue of builtin functions and exit.  Pass a "
            "function name to narrow the output (e.g. "
            "``--help-builtins ip``)."
        ),
    )
    p.add_argument(
        "--help-examples",
        nargs=0,
        action=_HelpExamplesAction,
        default=argparse.SUPPRESS,
        help="Show a cookbook of worked example queries and exit.",
    )

    p.set_defaults(handler=_run_query, output_mode="auto")


def _run_query(args: argparse.Namespace) -> int:
    expression = _resolve_expression(args)
    if expression is None:
        print(
            "error: no query expression supplied (positional or --from-file)",
            file=sys.stderr,
        )
        return 2
    if not args.paths:
        print("error: no input files (pass '-' to read stdin)", file=sys.stderr)
        return 2

    sources: dict[str, str] = {}
    path_for_uri: dict[str, str] = {}
    for path_str in args.paths:
        if path_str == "-" and args.in_place:
            print("error: --in-place requires a path, not stdin", file=sys.stderr)
            return 2
        try:
            uri, src = read_path(path_str)
        except OSError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 2
        sources[uri] = src
        path_for_uri[uri] = path_str

    try:
        result = run_query(expression, sources)
    except QueryError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if result.has_mutation:
        return _emit_mutation(args, sources, result, path_for_uri)
    return _emit_values(args, result, sources)


def _resolve_expression(args: argparse.Namespace) -> str | None:
    if args.from_file:
        # When --from-file is set the positional ``expression`` is
        # always the first input file (argparse fills the positional
        # slot before it gets to ``paths``).  Promote it unconditionally
        # so commands like ``f5 query -f q.fq a.conf b.conf`` see both
        # files; the previous ``and not args.paths`` guard silently
        # dropped ``a.conf`` when more than one input was given.
        if args.expression:
            args.paths = [args.expression, *args.paths]
            args.expression = None
        try:
            return Path(args.from_file).read_text(encoding="utf-8")
        except OSError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return None
    return args.expression


def _emit_mutation(
    args: argparse.Namespace,
    sources: dict[str, str],
    result,
    path_for_uri: dict[str, str],
) -> int:
    any_changed = False
    for uri, applied in result.edits_per_file.items():
        for rep in applied.rename_reports:
            print(
                f"renamed {rep.old!r} -> {rep.new!r} ({rep.occurrences} occurrence(s))",
                file=sys.stderr,
            )
        # A "mutating" query that produced no actual textual change
        # (e.g. ``rename_partition(...)`` on a source with no matches)
        # should report no-op via exit code 1, mirroring the
        # convention ``f5 rename`` already uses.
        if applied.new_source == applied.original:
            continue
        any_changed = True
        path_str = path_for_uri.get(uri, uri)
        # ``--format tmsh`` re-renders the rewritten SCF as a
        # ``tmsh modify`` script for persisting onto a live device.
        # The unified-diff path stays SCF↔SCF because the on-disk
        # source is always SCF.
        rewritten = render_config(applied.new_source, fmt=args.output_format, tmsh_verb="modify")
        if args.in_place and path_str != "-":
            Path(path_str).write_text(rewritten, encoding="utf-8")
            continue
        if args.write:
            sys.stdout.write(rewritten)
            continue
        diff = difflib.unified_diff(
            applied.original.splitlines(keepends=True),
            applied.new_source.splitlines(keepends=True),
            fromfile=path_str,
            tofile=f"{path_str} (modified)",
        )
        sys.stdout.writelines(diff)
    return 0 if any_changed else 1


def _emit_values(
    args: argparse.Namespace,
    result,
    sources: dict[str, str],
) -> int:
    any_emitted = False
    multi = len(sources) > 1
    for uri, values in result.values_per_file.items():
        if multi:
            sys.stdout.write(f"# === {uri} ===\n")
        text = render(values, mode=args.output_mode)
        if text:
            any_emitted = True
        sys.stdout.write(text)
    return 0 if any_emitted else 1
