"""``f5 irule`` — verb group with iRules-specific sub-subcommands.

Sub-subcommands:

- ``f5 irule event-order`` — show iRules events in canonical firing order.
- ``f5 irule event-info``  — look up iRules event metadata and valid commands.

These verbs are purely iRules-focused and are not available on the
general-purpose ``tcl`` CLI.  Generally-useful verbs (``command-info``,
``convert``, ``help``, …) remain on ``tcl`` and accept ``--dialect
f5-irules`` for iRules input.
"""

from __future__ import annotations

import argparse
import json

from core.commands.registry.info import lookup_event_info
from core.commands.registry.namespace_data import event_multiplicity, order_events_for_file
from core.commands.registry.runtime import configure_signatures

from .._utils import (
    _add_input_arguments,
    _combine_sources,
    _read_input_documents,
    _write_text_output,
)


def add_irule_subparser(
    sub: argparse._SubParsersAction,  # noqa: SLF001
    *,
    prog_name: str = "f5",
    default_dialect: str = "f5-irules",
) -> None:
    """Register the ``irule`` verb group and its sub-subparsers."""
    irule_p = sub.add_parser(
        "irule",
        help="iRules-specific analysis (event-order, event-info, ...).",
        description=(
            "iRules-specific verbs.  All sub-subcommands default to the f5-irules dialect."
        ),
        epilog=(
            "Examples:\n"
            f"  {prog_name} irule event-order rules/policy.irule\n"
            f"  {prog_name} irule event-info HTTP_REQUEST\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    irule_sub = irule_p.add_subparsers(dest="irule_action", required=True)

    # ── event-order ────────────────────────────────────────────────────
    eo_p = irule_sub.add_parser(
        "event-order",
        aliases=["eventorder"],
        help="Show iRules events in canonical firing order.",
    )
    _add_input_arguments(eo_p, include_output=True, default_dialect="f5-irules")
    eo_p.add_argument(
        "--json",
        action="store_true",
        help="Emit event ordering as JSON.",
    )
    eo_p.set_defaults(handler=_run_event_order)

    # ── event-info ─────────────────────────────────────────────────────
    ei_p = irule_sub.add_parser(
        "event-info",
        aliases=["eventinfo"],
        help="Look up iRules event metadata and valid commands.",
    )
    ei_p.add_argument(
        "event",
        help="iRules event name (for example: HTTP_REQUEST).",
    )
    ei_p.add_argument(
        "--json",
        action="store_true",
        help="Emit event metadata as JSON.",
    )
    ei_p.add_argument(
        "--output",
        "-o",
        default="-",
        help="Output path ('-' for stdout).",
    )
    ei_p.set_defaults(handler=_run_event_info)


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def _run_event_order(args: argparse.Namespace) -> int:
    documents = _read_input_documents(
        args.inputs,
        inline_sources=args.source,
        package_paths=args.package_path,
        recursive=not args.no_recursive,
    )
    configure_signatures(dialect=args.dialect)
    source = _combine_sources(documents)

    ordered = order_events_for_file(source)
    events = [
        {
            "index": index,
            "name": event_name,
            "multiplicity": event_multiplicity(event_name),
        }
        for index, event_name in enumerate(ordered, start=1)
    ]
    payload = {
        "count": len(events),
        "dialect": args.dialect,
        "events": events,
    }
    if args.json:
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 0

    lines = [f"event order: {len(events)} event(s)"]
    for item in events:
        lines.append(f"  {item['index']}. {item['name']} ({item['multiplicity']})")
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_event_info(args: argparse.Namespace) -> int:
    info = lookup_event_info(args.event, dialect="f5-irules")
    payload = {
        "event": info.event,
        "known": info.known,
        "deprecated": info.deprecated,
        "multiplicity": info.multiplicity,
        "description": info.description,
        "side": info.side,
        "transport": info.transport,
        "impliedProfiles": list(info.implied_profiles),
        "validCommandCount": info.valid_command_count,
        "validCommands": list(info.valid_commands),
    }

    if args.json:
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 0 if info.known else 1

    lines = [
        f"event: {info.event}",
        f"known: {'yes' if info.known else 'no'}",
        f"deprecated: {'yes' if payload['deprecated'] else 'no'}",
        f"multiplicity: {payload['multiplicity']}",
    ]
    if info.description:
        lines.append(f"description: {info.description}")
    lines.append(f"side: {payload['side']}")
    if payload["transport"]:
        lines.append(f"transport: {payload['transport']}")
    if payload["impliedProfiles"]:
        lines.append(f"profiles: {', '.join(str(item) for item in payload['impliedProfiles'])}")
    lines.append(f"valid commands: {info.valid_command_count}")
    _write_text_output(args.output, "\n".join(lines))
    return 0 if info.known else 1
