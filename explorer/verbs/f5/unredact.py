"""``f5 unredact`` — reverse a redaction using a map file.

Useful when F5 support replies referencing a redacted IP and you need
the real one, or when round-tripping a redacted artefact through an
internal review process.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.bigip.redact_map import RedactionMap, apply_map

from ._paths import read_path
from ._registry import verb


@verb(
    "unredact",
    aliases=("unmap",),
    help="Reverse a previous `f5 redact` using its sidecar map file.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Read the TOML map file produced by `f5 redact` and rewrite\n"
        "every IPv4 / IPv6 literal in the input back to its original\n"
        "value.  The input does not have to be a complete config —\n"
        "this works on any text (including support emails, ticket\n"
        "comments, log snippets) so long as the IPs in it match the\n"
        "map.  Tokens that aren't in the map are passed through\n"
        "unchanged."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 unredact map.toml redacted.conf -o original.conf\n"
        "  f5 unredact map.toml support-ticket.txt   # works on prose, not just configs\n"
        "  cat redacted.log | f5 unredact map.toml -\n"
    )
    p.add_argument(
        "map_file",
        help="The TOML map file produced by `f5 redact` (`-` for stdin).",
    )
    p.add_argument("path", help="Text containing redacted IPs (`-` for stdin).")
    p.add_argument(
        "-o", "--output", metavar="FILE", help="Write recovered text here (default: stdout)."
    )
    p.set_defaults(handler=_run_unredact)


def _run_unredact(args: argparse.Namespace) -> int:
    if args.map_file == "-" and args.path == "-":
        print("error: cannot read both map file and input from stdin", file=sys.stderr)
        return 2
    try:
        _, map_text = read_path(args.map_file)
        _, source = read_path(args.path)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    try:
        rm = RedactionMap.from_toml(map_text)
    except (ValueError, RuntimeError) as exc:
        print(f"error: cannot load map: {exc}", file=sys.stderr)
        return 2

    out, count = apply_map(rm, source, reverse=True)

    if args.output:
        Path(args.output).write_text(out, encoding="utf-8")
    else:
        sys.stdout.write(out)

    print(f"unredacted: {count} IP(s) recovered", file=sys.stderr)
    return 0
