"""``f5 graph`` — emit the BIG-IP reference graph as DOT/JSON/Mermaid."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from dialects.f5.bigip.graph_export import FORMATS, export_graph

from ._paths import load_paths
from ._registry import verb


@verb(
    "graph",
    aliases=("deps",),
    help="Emit the object reference graph as DOT, JSON, or Mermaid.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Walk every reference edge in the configuration (the same graph\n"
        "`f5 cleanup` and `f5 grep` use) and serialise it.  Pass --seed\n"
        "to limit output to a subgraph reachable from one or more\n"
        "objects; pair with --reverse to walk incoming references instead\n"
        "(what depends on each seed)."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 graph bigip.conf -o graph.dot\n"
        "  f5 graph bigip.conf --format mermaid -o graph.mmd\n"
        "  f5 graph bigip.conf --seed /Common/vs_app --max-depth 3\n"
        "  f5 graph bigip.conf --seed /Common/web_pool --reverse\n"
        "  f5 graph bigip.conf --format json | jq '.nodes | length'\n"
    )
    p.add_argument("paths", nargs="+", help="bigip.conf / SCF files (`-` for stdin).")
    p.add_argument(
        "--format",
        choices=FORMATS,
        default="dot",
        dest="fmt",
        help="Output format (default: dot).",
    )
    p.add_argument(
        "--seed",
        action="append",
        default=[],
        metavar="PATH",
        help="Object full-path to use as a seed (repeatable).  Without --seed the full graph is emitted.",
    )
    p.add_argument(
        "--reverse",
        action="store_true",
        help="Walk incoming references from each seed (what depends on it).",
    )
    p.add_argument(
        "--max-depth",
        type=int,
        metavar="N",
        help="Limit the BFS to N hops from each seed.",
    )
    p.add_argument("-o", "--output", metavar="FILE", help="Write here (default: stdout).")
    p.set_defaults(handler=_run_graph)


def _run_graph(args: argparse.Namespace) -> int:
    try:
        sources, configs = load_paths(args.paths)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    export = export_graph(
        sources=sources,
        configs=configs,
        fmt=args.fmt,
        seeds=tuple(args.seed),
        reverse=args.reverse,
        max_depth=args.max_depth,
    )
    if args.output:
        Path(args.output).write_text(export.text, encoding="utf-8")
    else:
        sys.stdout.write(export.text)
    return 0
