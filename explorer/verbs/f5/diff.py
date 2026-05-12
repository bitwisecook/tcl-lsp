"""``f5 diff`` — semantic diff between two BIG-IP configurations."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from core.bigip.diff import compute_diff, report_to_dict
from core.bigip.parser import parse_bigip_conf

from ._paths import read_path
from ._registry import verb


@verb(
    "diff",
    aliases=("changes",),
    help="Object-aware diff between two bigip.conf / SCF files.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Compare two parsed BIG-IP configurations and report objects\n"
        "added, removed, or modified.  Property ordering and whitespace\n"
        "are ignored; iRule bodies are compared after stripping comments\n"
        "and collapsing whitespace.  Exits 1 when the two configs differ\n"
        "(makes the verb easy to use in CI gates and pre-flight checks)."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 diff old.conf new.conf\n"
        "  f5 diff old.conf new.conf --json -o changes.json\n"
        "  f5 diff prev.scf current.scf && echo no changes\n"
    )
    p.add_argument("before", help="Old config (bigip.conf / SCF, or `-` for stdin).")
    p.add_argument("after", help="New config (bigip.conf / SCF, or `-` for stdin).")
    p.add_argument("--json", action="store_true", help="Emit JSON instead of text.")
    p.add_argument("-o", "--output", metavar="FILE", help="Write report here (default: stdout).")
    p.set_defaults(handler=_run_diff)


def _run_diff(args: argparse.Namespace) -> int:
    try:
        _, before_src = read_path(args.before)
        _, after_src = read_path(args.after)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    before = parse_bigip_conf(before_src)
    after = parse_bigip_conf(after_src)
    report = compute_diff(before, after)

    if args.json:
        output = json.dumps(report_to_dict(report), indent=2) + "\n"
    else:
        output = report.text_report
        if not output.endswith("\n"):
            output += "\n"
    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)
    return 1 if report.has_changes else 0
