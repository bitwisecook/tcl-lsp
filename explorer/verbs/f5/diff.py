"""``f5 diff`` — semantic diff between two BIG-IP configurations."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from core.bigip.diff import compute_diff, report_to_dict
from core.bigip.parser import parse_bigip_conf
from core.bigip.tmsh_parse import looks_like_tmsh_output, normalise_tmsh_to_scf

from ._paths import read_path
from ._registry import verb


@verb(
    "diff",
    aliases=("changes",),
    help="Object-aware diff between two SCF or tmsh-output files.",
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Compare two parsed BIG-IP configurations and report objects "
        "added, removed, or modified.  Each input may be either an SCF "
        "/ bigip.conf stanza dump or a tmsh command script "
        "(`tmsh create` / `tmsh modify` lines, as emitted by `f5 tmsh` "
        "or pasted from a real BIG-IP shell) — the two formats may even "
        "be mixed across the two sides.  Property ordering and "
        "whitespace are ignored; iRule bodies are compared after "
        "stripping comments and collapsing whitespace.  Exits 1 when "
        "the two configs differ (makes the verb easy to use in scripts)."
    )
    p.add_argument(
        "before",
        help="Old config (bigip.conf / SCF or tmsh-output, or `-` for stdin).",
    )
    p.add_argument(
        "after",
        help="New config (bigip.conf / SCF or tmsh-output, or `-` for stdin).",
    )
    p.add_argument("--json", action="store_true", help="Emit JSON instead of text.")
    p.add_argument("-o", "--output", metavar="FILE", help="Write report here (default: stdout).")
    p.set_defaults(handler=_run_diff)


def _to_scf(source: str) -> str:
    return normalise_tmsh_to_scf(source) if looks_like_tmsh_output(source) else source


def _run_diff(args: argparse.Namespace) -> int:
    try:
        _, before_src = read_path(args.before)
        _, after_src = read_path(args.after)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    before = parse_bigip_conf(_to_scf(before_src))
    after = parse_bigip_conf(_to_scf(after_src))
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
