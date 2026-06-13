"""``f5 registry-dump`` — serialise the F5 registries and graphs as JSON.

Emits a canonical, deterministic snapshot of:

- the F5 command registries (``f5-irules`` and ``f5-iapps``);
- the iRules event graph (per-event protocol props, firing order, flow
  chains, and the commands valid in each event);
- the protocol profile graph;
- the BIG-IP object graph (object kinds and reference edges).

This is the language-agnostic shape contract behind the golden fixtures
and the front-end-behaviour tests.  A Rust front-end re-implementing
this verb is validated against the very same fixtures.
"""

from __future__ import annotations

import argparse
import json

from tooling.registry_snapshot import (
    F5_DIALECTS,
    command_registry_snapshot,
    event_graph_snapshot,
    object_graph_snapshot,
    profile_graph_snapshot,
)

from ._registry import verb

_SECTIONS = ("commands", "events", "profiles", "objects", "all")


@verb(
    "registry-dump",
    aliases=("registrydump", "dump-registry"),
    help="Dump the F5 command registry and event/profile/object graphs as JSON.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Serialise the F5 registries as canonical JSON: the iRules / iApps\n"
        "command registries, the iRules event graph, the profile graph, and\n"
        "the BIG-IP object graph.  This is the registry shape contract used\n"
        "by the front-end-behaviour tests and golden fixtures.\n"
    )
    p.epilog = (
        "Examples:\n"
        "  f5 registry-dump --section events\n"
        "  f5 registry-dump --section objects -o objects.json\n"
        "  f5 registry-dump --section all\n"
    )
    p.add_argument(
        "--section",
        choices=_SECTIONS,
        default="all",
        help="Which registry section to dump (default: all).",
    )
    p.add_argument("--json", action="store_true", help="Emit JSON (default and only format).")
    p.add_argument("--output", "-o", default="-", help="Output path ('-' for stdout).")
    p.set_defaults(handler=_run)


def _build(section: str) -> dict:
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


def _run(args: argparse.Namespace) -> int:
    payload = _build(args.section)
    text = json.dumps(payload, indent=2, sort_keys=True)
    if args.output == "-":
        print(text)
    else:
        with open(args.output, "w", encoding="utf-8") as fh:
            fh.write(text + "\n")
    return 0
