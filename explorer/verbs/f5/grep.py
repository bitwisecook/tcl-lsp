"""``f5 grep`` — find every BIG-IP object related to a name or regex."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from core.bigip.grep import DIRECTIONS, compute_grep, report_to_dict
from core.bigip.parser import parse_bigip_conf

from ._registry import verb


@verb(
    "grep",
    aliases=("related",),
    help="List every BIG-IP object related to a given object path or regex.",
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Find every BIG-IP object reachable through reference edges from "
        "the seed objects whose full path matches PATTERN.  PATTERN is "
        "a substring match by default; pass --regex to treat it as a "
        "Python regular expression, or --cidr to match IP addresses and "
        "CIDR ranges anywhere in an object — including deep inside iRule "
        "script bodies.  The reference graph is the same one "
        "`f5 cleanup` walks — both configuration-property references "
        "and iRule body references (`pool`, `persist`, `class match ... "
        "<data-group>`) are tracked."
    )
    p.add_argument(
        "pattern",
        help=(
            "Object full-path or substring by default; a Python regex with "
            "--regex; or one or more whitespace/comma-separated IP/CIDR "
            "values with --cidr (for example `10.0.0.0/8` or "
            "`10.0.0.0/8,192.168.0.0/16`)."
        ),
    )
    p.add_argument(
        "paths",
        nargs="+",
        help="bigip.conf / SCF files (one or more).  Pass `-` to read stdin.",
    )
    mode_group = p.add_mutually_exclusive_group()
    mode_group.add_argument(
        "-e",
        "--regex",
        action="store_true",
        help="Treat PATTERN as a Python regular expression (default: substring match).",
    )
    mode_group.add_argument(
        "-c",
        "--cidr",
        action="store_true",
        help=(
            "Treat PATTERN as one or more IPv4/IPv6 addresses or CIDR "
            "ranges (whitespace- or comma-separated).  An object matches "
            "when any IP literal or CIDR mentioned in its path, header, "
            "or body — including iRule script bodies — overlaps any "
            "requested network."
        ),
    )
    p.add_argument(
        "--direction",
        choices=sorted(DIRECTIONS),
        default="both",
        help=(
            "Which edges to traverse from each seed.  `forward` follows "
            "outgoing references (what the seed depends on), `reverse` "
            "follows incoming references (what depends on the seed), "
            "`both` walks both (default)."
        ),
    )
    p.add_argument(
        "--max-depth",
        type=int,
        default=None,
        metavar="N",
        help="Stop the BFS after N hops from each seed (default: unlimited).",
    )
    p.add_argument(
        "--max-nodes",
        type=int,
        default=1000,
        metavar="N",
        help="Cap the result at N objects (default: 1000).",
    )
    p.add_argument(
        "--full",
        action="store_true",
        help=(
            "Include each object's full body in the output.  In the text "
            "report bodies appear under each header; in JSON output bodies "
            "are embedded under the `body` key on every object."
        ),
    )
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit the grep report as JSON instead of the text report.",
    )
    p.add_argument(
        "-o",
        "--output",
        metavar="FILE",
        help="Write the report here (default: stdout).",
    )
    p.set_defaults(handler=_run_grep)


def _read_path(path_str: str) -> tuple[str, str]:
    """Return ``(uri, source)`` for *path_str*.  ``-`` reads stdin."""
    if path_str == "-":
        return ("stdin://input", sys.stdin.read())
    path = Path(path_str).resolve()
    if not path.is_file():
        raise FileNotFoundError(f"not a file: {path_str}")
    return (path.as_uri(), path.read_text(encoding="utf-8", errors="replace"))


def _run_grep(args: argparse.Namespace) -> int:
    sources: dict[str, str] = {}
    configs = {}
    for path_str in args.paths:
        try:
            uri, src = _read_path(path_str)
        except OSError as exc:
            print(f"error: {exc}", file=sys.stderr)
            return 2
        sources[uri] = src
        configs[uri] = parse_bigip_conf(src)

    report = compute_grep(
        sources=sources,
        configs=configs,
        pattern=args.pattern,
        use_regex=args.regex,
        use_cidr=args.cidr,
        direction=args.direction,
        max_depth=args.max_depth,
        max_nodes=args.max_nodes,
        include_body=args.full,
    )

    if args.json:
        output = json.dumps(report_to_dict(report, include_body=args.full), indent=2) + "\n"
    else:
        output = report.text_report
        if not output.endswith("\n"):
            output += "\n"

    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)

    # Exit code: 0 when at least one seed matched, 1 when no seeds matched
    # (mirrors `grep` convention: empty match is a non-zero exit).
    return 0 if report.seeds else 1
