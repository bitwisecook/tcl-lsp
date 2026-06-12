"""Registry-dump verb: serialise the full command registry as JSON.

``tcl registry-dump`` emits a canonical, deterministic snapshot of the
command registry for one dialect (or every Tcl dialect with
``--all-dialects``).  The output is the language-agnostic shape contract
consumed by the golden fixtures under ``tests/baselines/registry/`` and
the front-end-behaviour tests; a Rust front-end that re-implements this
verb is validated against the same fixtures.
"""

from __future__ import annotations

import argparse
import json

from tooling.cli._utils import _write_text_output
from tooling.registry_snapshot import (
    TCL_DIALECTS,
    command_registry_snapshot,
    command_registry_snapshots,
)

from ._registry import verb


@verb(
    "registry-dump",
    aliases=("registrydump", "dump-registry"),
    help="Dump the full command registry (arities, traits, subcommands) as JSON.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_registry_dump(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    p.description = (
        "Serialise the command registry as canonical JSON: every command\n"
        "with its arity, switches, forms, subcommands, and all boolean\n"
        "trait properties.  This is the registry shape contract used by the\n"
        "front-end-behaviour tests and golden fixtures.\n"
    )
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} registry-dump --dialect tcl8.6\n"
        f"  {prog_name} registry-dump --all-dialects -o commands.json\n"
    )
    group = p.add_mutually_exclusive_group()
    group.add_argument(
        "--dialect",
        default=default_dialect,
        help=f"Dialect to dump (default: {default_dialect}).",
    )
    group.add_argument(
        "--all-dialects",
        action="store_true",
        help="Dump every Tcl dialect (tcl8.4-9.0) into a single document.",
    )
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit JSON (default and only format for this verb).",
    )
    p.add_argument(
        "--output",
        "-o",
        default="-",
        help="Output path ('-' for stdout).",
    )
    p.set_defaults(handler=_run_registry_dump)


def _run_registry_dump(args: argparse.Namespace) -> int:
    if args.all_dialects:
        payload = command_registry_snapshots(TCL_DIALECTS)
    else:
        payload = command_registry_snapshot(args.dialect)
    _write_text_output(args.output, json.dumps(payload, indent=2, sort_keys=True))
    return 0
