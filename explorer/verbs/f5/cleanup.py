"""``f5 cleanup`` — generate ``tmsh delete`` commands for unreferenced objects."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from core.bigip.cleanup import compute_cleanup, report_to_dict
from core.bigip.parser import parse_bigip_conf
from core.commands.registry.runtime import configure_signatures

from ._registry import verb


@verb(
    "cleanup",
    aliases=("clean",),
    help="Generate `tmsh delete` commands for objects unreferenced by any virtual.",
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Walk the BIG-IP object reference graph from every ltm virtual / "
        "gtm wide-IP and emit a tmsh script that deletes everything else, "
        "in reverse-topological order so each delete runs only after the "
        "objects that reference its target have already been removed."
    )
    p.add_argument(
        "paths",
        nargs="+",
        help="bigip.conf / SCF files (one or more).  Pass `-` to read stdin.",
    )
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit the cleanup report as JSON instead of a tmsh script.",
    )
    p.add_argument(
        "--keep",
        action="append",
        default=[],
        metavar="PATH",
        help=(
            "Object full-path or partition prefix to retain (repeatable). "
            "Paths ending in '/' are treated as partition prefixes."
        ),
    )
    p.add_argument(
        "--no-keep-common",
        action="store_true",
        help="Do not auto-keep /Common/* (delete factory-shipped objects too).",
    )
    p.add_argument(
        "-o",
        "--output",
        metavar="FILE",
        help="Write the script/JSON here (default: stdout).",
    )
    p.set_defaults(handler=_run_cleanup)


def _read_path(path_str: str) -> tuple[str, str]:
    """Return ``(uri, source)`` for *path_str*.  ``-`` reads stdin."""
    if path_str == "-":
        return ("stdin://input", sys.stdin.read())
    path = Path(path_str).resolve()
    if not path.is_file():
        raise FileNotFoundError(f"not a file: {path_str}")
    return (path.as_uri(), path.read_text(encoding="utf-8", errors="replace"))


def _split_keep_arguments(keep: list[str]) -> tuple[frozenset[str], frozenset[str]]:
    paths: set[str] = set()
    prefixes: set[str] = set()
    for entry in keep:
        if entry.endswith("/"):
            prefixes.add(entry)
        else:
            paths.add(entry)
    return frozenset(paths), frozenset(prefixes)


def _run_cleanup(args: argparse.Namespace) -> int:
    # ``f5-irules`` registers the ``when`` event handler with ArgRole.BODY,
    # which the iRule body scan in ``core.bigip.link_extract`` needs to walk
    # nested commands and pick up `pool` / `class match` / `persist` / etc.
    # references inside the rule body.
    configure_signatures(dialect="f5-irules")

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

    keep_paths, extra_prefixes = _split_keep_arguments(args.keep)
    if args.no_keep_common:
        keep_partitions = extra_prefixes
    else:
        keep_partitions = extra_prefixes | frozenset({"/Common/"})

    report = compute_cleanup(
        sources=sources,
        configs=configs,
        keep_paths=keep_paths,
        keep_partitions=keep_partitions,
    )

    if args.json:
        output = json.dumps(report_to_dict(report), indent=2) + "\n"
    else:
        output = report.tmsh_script
        if not output.endswith("\n"):
            output += "\n"

    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)

    return 0
