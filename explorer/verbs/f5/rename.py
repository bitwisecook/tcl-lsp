"""``f5 rename`` — rename an object and update every reference."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.bigip.rewrite import rename_object

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
        "diff); pass --write or --in-place to persist."
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
    p.set_defaults(handler=_run_rename)


def _run_rename(args: argparse.Namespace) -> int:
    if args.path == "-" and args.in_place:
        print("error: --in-place requires a path argument, not stdin", file=sys.stderr)
        return 2
    try:
        _, source = read_path(args.path)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    try:
        report = rename_object(source, args.old, args.new)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if report.occurrences == 0:
        print(f"warning: no occurrences of {args.old!r} found", file=sys.stderr)
        return 1

    print(
        f"renamed {args.old!r} -> {args.new!r} ({report.occurrences} occurrence(s))",
        file=sys.stderr,
    )

    if args.in_place:
        Path(args.path).write_text(report.new_source, encoding="utf-8")
    elif args.output:
        Path(args.output).write_text(report.new_source, encoding="utf-8")
    elif args.write:
        sys.stdout.write(report.new_source)
    else:
        # Default: emit a unified diff so the user sees what would change.
        import difflib

        diff = difflib.unified_diff(
            source.splitlines(keepends=True),
            report.new_source.splitlines(keepends=True),
            fromfile=args.path,
            tofile=f"{args.path} (renamed)",
        )
        sys.stdout.writelines(diff)
    return 0
