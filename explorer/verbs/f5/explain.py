"""``f5 explain`` — describe what a virtual or pool actually does."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from core.bigip.explain import compute_explain, report_to_dict

from ._paths import load_paths
from ._registry import verb


@verb(
    "explain",
    aliases=("describe",),
    help="Describe the resolved profile/iRule/persistence/SNAT/pool plan for a virtual or pool.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Resolve and print, for a given virtual server (or pool), the\n"
        "ordered list of attached profiles, the iRule chain with events,\n"
        "persistence, SNAT, default pool, and pool members.  Acts as the\n"
        "operator answer to 'what happens to traffic on this VIP?'.\n"
        "Use 'auto' as the kind to try virtual then pool."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 explain virtual /Common/vs_app bigip.conf\n"
        "  f5 explain virtual vs_app bigip.conf       # short name lookup\n"
        "  f5 explain pool /Common/web_pool bigip.conf\n"
        "  f5 explain auto vs_app bigip.conf --json\n"
    )
    p.add_argument(
        "kind",
        choices=("virtual", "pool", "auto"),
        help="What kind of object to explain (use 'auto' to try virtual then pool).",
    )
    p.add_argument(
        "target",
        help="Object full-path or short name (e.g. /Common/vs_app or vs_app).",
    )
    p.add_argument("paths", nargs="+", help="bigip.conf / SCF files (`-` for stdin).")
    p.add_argument("--json", action="store_true", help="Emit JSON instead of text.")
    p.add_argument("-o", "--output", metavar="FILE", help="Write report here (default: stdout).")
    p.set_defaults(handler=_run_explain)


def _run_explain(args: argparse.Namespace) -> int:
    try:
        _sources, configs = load_paths(args.paths)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    kind_hint = None if args.kind == "auto" else args.kind

    # Iterate every parsed config until one resolves the target.
    for cfg in configs.values():
        report = compute_explain(cfg, args.target, kind=kind_hint)
        if report.found:
            break
    else:
        report = compute_explain(next(iter(configs.values())), args.target, kind=kind_hint)

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
    return 0 if report.found else 1
