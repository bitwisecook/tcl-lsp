#!/usr/bin/env python3
"""Temporary registry dumper — rust-branch tooling, NOT a shipped CLI surface.

This serialises the command / event / profile / object registries as
canonical JSON.  It lives here, as a standalone dev script, on purpose:
the JSON dump is **not** a surface we want to promise.  The committed
contract is the small CSV presence fixtures under
``tests/baselines/registry/`` plus the front-end-behaviour tests; the
full JSON shape is only useful while bringing up the Rust front-end on
the ``rust`` branch, where it is validated against those same fixtures.

Keep this script confined to the ``rust`` branch — it is deliberately not
wired into the ``tcl`` / ``f5`` CLI verb registries and must not be
merged to ``main`` as a public verb.

Usage::

    python scripts/registry/dump.py tcl --dialect tcl8.6
    python scripts/registry/dump.py tcl --all-dialects -o commands.json
    python scripts/registry/dump.py f5 --section events
    python scripts/registry/dump.py f5 --section all -o f5.json

The builders themselves live in :mod:`tooling.registry_snapshot`, which
*is* committed (the CSV baselines and contract tests build on it); this
script is only the thin JSON front-end over them.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from tooling.registry_snapshot import (  # noqa: E402
    ALL_DIALECTS,
    F5_DIALECTS,
    TCL_DIALECTS,
    command_registry_snapshot,
    command_registry_snapshots,
    event_graph_snapshot,
    object_graph_snapshot,
    profile_graph_snapshot,
)

_F5_SECTIONS = ("commands", "events", "profiles", "objects", "all")


def _write(text: str, output: str) -> None:
    if output == "-":
        print(text)
    else:
        with open(output, "w", encoding="utf-8") as fh:
            fh.write(text + "\n")


def _dump_tcl(args: argparse.Namespace) -> int:
    if args.all_dialects:
        payload = command_registry_snapshots(TCL_DIALECTS)
    else:
        payload = command_registry_snapshot(args.dialect)
    _write(json.dumps(payload, indent=2, sort_keys=True), args.output)
    return 0


def _build_f5(section: str) -> dict:
    if section == "commands":
        return {dialect: command_registry_snapshot(dialect) for dialect in F5_DIALECTS}
    if section == "events":
        return event_graph_snapshot()
    if section == "profiles":
        return profile_graph_snapshot()
    if section == "objects":
        return object_graph_snapshot()
    return {
        "commands": {dialect: command_registry_snapshot(dialect) for dialect in F5_DIALECTS},
        "events": event_graph_snapshot(),
        "profiles": profile_graph_snapshot(),
        "objects": object_graph_snapshot(),
    }


def _dump_f5(args: argparse.Namespace) -> int:
    _write(json.dumps(_build_f5(args.section), indent=2, sort_keys=True), args.output)
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        prog="registry-dump",
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    sub = parser.add_subparsers(dest="kind", required=True)

    tcl = sub.add_parser("tcl", help="Dump the core Tcl command registry.")
    group = tcl.add_mutually_exclusive_group()
    group.add_argument(
        "--dialect",
        choices=ALL_DIALECTS,
        default="tcl9.0",
        help="Dialect to dump (default: tcl9.0).",
    )
    group.add_argument(
        "--all-dialects",
        action="store_true",
        help="Dump every Tcl dialect (tcl8.4-9.0) into a single document.",
    )
    tcl.add_argument("--output", "-o", default="-", help="Output path ('-' for stdout).")
    tcl.set_defaults(handler=_dump_tcl)

    f5 = sub.add_parser("f5", help="Dump the F5 registries and graphs.")
    f5.add_argument(
        "--section",
        choices=_F5_SECTIONS,
        default="all",
        help="Which registry section to dump (default: all).",
    )
    f5.add_argument("--output", "-o", default="-", help="Output path ('-' for stdout).")
    f5.set_defaults(handler=_dump_f5)

    args = parser.parse_args(argv)
    return int(args.handler(args))


if __name__ == "__main__":
    raise SystemExit(main())
