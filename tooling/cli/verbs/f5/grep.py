"""``f5 grep`` — find every BIG-IP object related to a name or regex."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from dialects.f5.bigip.grep import DIRECTIONS, compute_grep, report_to_dict
from dialects.f5.bigip.parser import parse_bigip_conf

from ._emit import add_format_arg
from ._registry import verb


@verb(
    "grep",
    aliases=("related",),
    help="List every BIG-IP object related to a given object path or regex.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Find every BIG-IP object reachable through reference edges from\n"
        "the search-match objects whose full path matches PATTERN.\n"
        "PATTERN is a substring match by default; pass --regex to treat\n"
        "it as a Python regular expression, or --cidr to match IP\n"
        "addresses and CIDR ranges anywhere in an object — including\n"
        "deep inside iRule script bodies.  The reference graph is the\n"
        "same one `f5 cleanup` walks — both configuration-property\n"
        "references and iRule body references (`pool`, `persist`,\n"
        "`class match ... <data-group>`) are tracked."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 grep vs_app bigip.conf                  # substring\n"
        "  f5 grep -e '/Common/vs_.*' bigip.conf      # regex\n"
        "  f5 grep --cidr 10.0.0.0/8 bigip.conf       # IP / CIDR\n"
        "  f5 grep vs_app bigip.conf --direction reverse\n"
        "  f5 grep vs_app bigip.conf --max-depth 2 --full\n"
        "  f5 grep vs_app bigip.conf --json -o related.json\n"
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
            "Which edges to traverse from each search match.  `forward` "
            "follows outgoing references (what the match depends on), "
            "`reverse` follows incoming references (what depends on the "
            "match), `both` walks both (default)."
        ),
    )
    p.add_argument(
        "--max-depth",
        type=int,
        default=None,
        metavar="N",
        help="Stop the BFS after N hops from each search match (default: unlimited).",
    )
    p.add_argument(
        "-r",
        "--recurse",
        dest="recurse",
        default=True,
        action=argparse.BooleanOptionalAction,
        help=(
            "Walk the related-object BFS from each search match "
            "(default).  Pass --no-recurse to skip the BFS and report "
            "only the objects that directly match PATTERN; --direction "
            "and --max-depth are then ignored."
        ),
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
    add_format_arg(p, tmsh_default_verb="create")
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
        recurse=args.recurse,
    )

    if args.json:
        output = json.dumps(report_to_dict(report, include_body=args.full), indent=2) + "\n"
    elif args.output_format == "tmsh":
        # tmsh mode strips the report scaffolding and emits each
        # matched stanza (seeds + related) as a `tmsh create` command.
        # We render directly from the GrepObject pair rather than
        # routing through parse_bigip_conf + emit_tmsh: the parser is
        # keyed by full path, so the same `/Common/foo` coming from
        # two different input files would collide there and one copy
        # would be silently dropped.
        output = _matched_stanzas_as_tmsh(report)
    else:
        output = report.text_report
        if not output.endswith("\n"):
            output += "\n"

    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)

    # Exit code: 0 when at least one match was found, 1 otherwise
    # (mirrors `grep` convention: empty match is a non-zero exit).
    return 0 if report.seeds else 1


def _matched_stanzas_as_tmsh(report) -> str:  # noqa: ANN001
    """Render every matched object as a one-line ``tmsh create`` command.

    Each :class:`GrepObject` carries its original ``header`` and
    ``body`` text, so the conversion is a straightforward textual
    rewrap: ``tmsh create <header> { <body> }``.  Dedupe is on
    :attr:`GrepObject.node_id` (which already encodes the source URI),
    so the same ``/Common/foo`` coming from two different input files
    survives as two stanzas — collapsing on ``full_path`` alone would
    silently drop one of them in multi-file runs.

    Order is seeds first, then related, matching the text report.
    """
    parts: list[str] = []
    seen: set[str] = set()
    for obj in tuple(report.seeds) + tuple(report.related):
        if obj.node_id in seen:
            continue
        seen.add(obj.node_id)
        header = obj.header.strip()
        body = " ".join(obj.body.split())
        parts.append(
            f"tmsh create {header} {{ {body} }}\n" if body else f"tmsh create {header} {{ }}\n"
        )
    return "".join(parts)
