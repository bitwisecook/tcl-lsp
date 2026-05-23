"""``f5 stats`` — high-level counts and summaries."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from dialects.f5.bigip.stats import compute_stats, report_to_dict

from ._paths import load_paths
from ._registry import verb


@verb(
    "stats",
    aliases=("summary",),
    help="Print object counts, partition breakdown, and top-references.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Aggregate counts of pools / virtuals / nodes / iRules / etc.\n"
        "across one or more bigip.conf / SCF files.  Useful first-look\n"
        "before running heavier analysis: how big is this config, where\n"
        "are objects concentrated, what's most referenced."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 stats bigip.conf\n"
        "  f5 stats bigip.conf --top 20\n"
        "  f5 stats bigip.conf --json -o stats.json\n"
        "  f5 stats prod.conf staging.conf   # combined across both\n"
    )
    p.add_argument("paths", nargs="+", help="bigip.conf / SCF files (`-` for stdin).")
    p.add_argument("--json", action="store_true", help="Emit JSON instead of text.")
    p.add_argument(
        "--top",
        type=int,
        default=10,
        metavar="N",
        help="Show top-N most-referenced objects (default 10).",
    )
    p.add_argument("-o", "--output", metavar="FILE", help="Write report here (default: stdout).")
    p.set_defaults(handler=_run_stats)


def _run_stats(args: argparse.Namespace) -> int:
    try:
        sources, configs = load_paths(args.paths)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    report = compute_stats(sources=sources, configs=configs, top_n=args.top)
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
    return 0
