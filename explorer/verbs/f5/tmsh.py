"""``f5 tmsh`` — emit a tmsh script that recreates an SCF on a device."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from core.bigip.parser import parse_bigip_conf
from core.bigip.tmsh_emit import emit_tmsh

from ._paths import read_path
from ._registry import verb


@verb(
    "tmsh",
    aliases=("scf2tmsh",),
    help="Emit a tmsh script that creates every object in the input config.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.epilog = (
        "Examples:\n"
        "  f5 tmsh bigip.conf -o create.tmsh\n"
        "  f5 tmsh bigip.conf --modify -o modify.tmsh\n"
        "  f5 tmsh bigip.conf --include pool --include virtual -o vs-and-pools.tmsh\n"
        "  f5 grep vs_app bigip.conf | f5 tmsh -\n"
    )
    p.description = (
        "Render an SCF (or a single object stanza extracted via "
        "`f5 grep --no-recurse`) as a sequence of `tmsh create` commands "
        "in dependency order — partitions / route-domains first, then "
        "nodes, monitors, data-groups, profiles, snat-pools, persistence, "
        "pools, iRules, and finally virtuals.  Useful for migrating a "
        "config to a fresh device, recreating just the objects matched "
        "by a grep, or building a fixture for a lab.  Pass --modify to "
        "switch every command to `tmsh modify` for partial in-place "
        "updates."
    )
    p.add_argument("path", help="bigip.conf / SCF file (`-` for stdin).")
    p.add_argument(
        "-o",
        "--output",
        metavar="FILE",
        help="Write the tmsh script here (default: stdout).",
    )
    p.add_argument(
        "--modify",
        action="store_true",
        help="Emit `tmsh modify` instead of `tmsh create` (in-place updates).",
    )
    p.add_argument(
        "--include",
        action="append",
        default=[],
        metavar="KIND",
        help=(
            "Limit to one or more kinds (repeatable).  Choices: "
            "generic-foundation, node, monitor, data-group, profile, "
            "snatpool, persistence, pool, rule, virtual.  Default: all."
        ),
    )
    p.set_defaults(handler=_run_tmsh)


_VALID_KINDS = (
    "generic-foundation",
    "node",
    "monitor",
    "data-group",
    "profile",
    "snatpool",
    "persistence",
    "pool",
    "rule",
    "virtual",
)


def _run_tmsh(args: argparse.Namespace) -> int:
    try:
        _, source = read_path(args.path)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if args.include:
        for kind in args.include:
            if kind not in _VALID_KINDS:
                print(
                    f"error: unknown kind {kind!r} (expected one of {', '.join(_VALID_KINDS)})",
                    file=sys.stderr,
                )
                return 2
        include = tuple(args.include)
    else:
        include = _VALID_KINDS

    cfg = parse_bigip_conf(source)
    script = emit_tmsh(
        cfg,
        source=source,
        include=include,
        use_modify_for_existing=args.modify,
    )

    if args.output:
        Path(args.output).write_text(script.text, encoding="utf-8")
    else:
        sys.stdout.write(script.text)

    print(f"tmsh: emitted {script.command_count} command(s)", file=sys.stderr)
    return 0
