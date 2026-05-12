"""``f5 rename`` — rename an object and update every reference.

The implementation is a thin shell over the ``f5 query`` engine: the
verb constructs a ``rename(OLD, NEW)`` expression and runs it through
:func:`core.bigip.query.run_query`.  Routing both verbs through the
same engine keeps the rename logic in one place — improvements to
``rename_object``, the iRule body walk, and the diff renderer are
inherited automatically.
"""

from __future__ import annotations

import argparse
import difflib
import sys
from pathlib import Path

from core.bigip.query import run_query
from core.bigip.query.errors import QueryError

from ._emit import add_format_arg, render_config
from ._paths import read_path
from ._registry import verb


@verb(
    "rename",
    aliases=("mv",),
    help="Rename a BIG-IP object full-path and update every reference.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Replace every occurrence of OLD with NEW in a bigip.conf / SCF\n"
        "— both the object's own header and every reference in property\n"
        "values, iRule bodies (`pool foo`, `class match ... <data-group>`),\n"
        "and pool member addresses.  Token-bounded so substring\n"
        "collisions don't fire.  Dry-run by default (emits a unified\n"
        "diff); pass --write or --in-place to persist.\n"
        "\n"
        "This verb is a shortcut for ``f5 query 'rename(OLD, NEW)' PATH``\n"
        "— the same rename engine backs both.  Reach for ``f5 query`` when\n"
        "you want to combine the rename with a filter or a property edit,\n"
        "or for the partition-cascade ``rename_partition()`` builtin."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 rename /Common/old_pool /Common/new_pool bigip.conf       # dry-run diff\n"
        "  f5 rename /Common/old_pool /Common/new_pool bigip.conf --write > new.conf\n"
        "  f5 rename /Common/old_pool /Common/new_pool bigip.conf --in-place\n"
        "  f5 rename /Common/old_pool /Common/new_pool bigip.conf -o new.conf\n"
    )
    p.add_argument("old", help="Old full-path (e.g. /Common/old_pool).")
    p.add_argument("new", help="New full-path (e.g. /Common/new_pool).")
    p.add_argument("path", help="bigip.conf / SCF file (`-` for stdin).")
    output_group = p.add_mutually_exclusive_group()
    output_group.add_argument(
        "-o",
        "--output",
        metavar="FILE",
        help="Write rewritten config here (default: stdout, dry-run).",
    )
    output_group.add_argument(
        "--in-place",
        action="store_true",
        help="Overwrite the input file with the rewritten config.",
    )
    p.add_argument(
        "--write",
        action="store_true",
        help="Print rewritten config to stdout instead of a unified-diff preview.",
    )
    add_format_arg(p, tmsh_default_verb="modify")
    p.set_defaults(handler=_run_rename)


def _dsl_string(value: str) -> str:
    """Wrap *value* as a DSL string literal, escaping ``\\`` and ``"``."""
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def _run_rename(args: argparse.Namespace) -> int:
    if args.path == "-" and args.in_place:
        print("error: --in-place requires a path argument, not stdin", file=sys.stderr)
        return 2
    if not args.old:
        print("error: old name is empty", file=sys.stderr)
        return 2
    if not args.new:
        print("error: new name is empty", file=sys.stderr)
        return 2
    try:
        uri, source = read_path(args.path)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    query = f"rename({_dsl_string(args.old)}, {_dsl_string(args.new)})"
    try:
        result = run_query(query, {uri: source})
    except QueryError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    applied = result.edits_per_file.get(uri)
    if applied is None or applied.new_source == applied.original:
        print(f"warning: no occurrences of {args.old!r} found", file=sys.stderr)
        return 1

    for report in applied.rename_reports:
        print(
            f"renamed {report.old!r} -> {report.new!r} ({report.occurrences} occurrence(s))",
            file=sys.stderr,
        )

    rewritten = render_config(applied.new_source, fmt=args.output_format, tmsh_verb="modify")

    if args.in_place:
        Path(args.path).write_text(rewritten, encoding="utf-8")
    elif args.output:
        Path(args.output).write_text(rewritten, encoding="utf-8")
    elif args.write:
        sys.stdout.write(rewritten)
    else:
        # Default: emit a unified diff so the user sees what would change.
        # The diff is always SCF↔SCF (no point diffing against a tmsh form
        # of the same content — the LHS is the source file as-is).
        diff = difflib.unified_diff(
            source.splitlines(keepends=True),
            applied.new_source.splitlines(keepends=True),
            fromfile=args.path,
            tofile=f"{args.path} (renamed)",
        )
        sys.stdout.writelines(diff)
    return 0
