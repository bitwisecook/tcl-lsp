"""Reference-lookup verbs: event-order, event-info, command-info, help."""

from __future__ import annotations

import argparse
import json
import sys

from core.commands.registry.info import lookup_command_info, lookup_event_info
from core.commands.registry.namespace_data import event_multiplicity, order_events_for_file
from core.commands.registry.runtime import configure_signatures

from ._registry import verb
from ._utils import (
    _add_input_arguments,
    _combine_sources,
    _read_input_documents,
    _write_text_output,
    TclCliError,
)
from ..pipeline import AVAILABLE_DIALECTS

_HELP_DIALECT_TERMS: dict[str, tuple[str, ...]] = {
    "synopsys-eda-tcl": (
        "synopsys",
        "dc_shell",
        "design_compiler",
        "primetime",
        "icc2",
        "formality",
    ),
    "cadence-eda-tcl": (
        "cadence",
        "genus",
        "innovus",
        "tempus",
        "xcelium",
        "encounter",
    ),
    "xilinx-eda-tcl": (
        "xilinx",
        "vivado",
        "vitis",
        "amd",
        "fpga",
        "ise",
    ),
    "intel-quartus-eda-tcl": (
        "quartus",
        "intel",
        "altera",
        "fpga",
        "quartus_sh",
    ),
    "mentor-eda-tcl": (
        "mentor",
        "siemens",
        "modelsim",
        "questa",
        "calibre",
        "vsim",
    ),
    "f5-iapps": (
        "iapps",
        "iapp",
        "f5",
        "big-ip",
    ),
    "f5-bigip": (
        "bigip",
        "big-ip",
        "bigip.conf",
        "f5",
        "ltm",
        "gtm",
    ),
    "f5-irules": (
        "irules",
        "irule",
        "f5",
        "big-ip",
        "tmm",
        "event",
    ),
    "tcl8.4": (
        "tcl",
        "tk",
    ),
    "tcl8.5": (
        "tcl",
        "tk",
    ),
    "tcl8.6": (
        "tcl",
        "tk",
    ),
    "tcl9.0": (
        "tcl",
        "tk",
    ),
}


@verb(
    "event-order",
    aliases=("eventorder",),
    help="Show iRules events in canonical firing order.",
)
def _configure_event_order(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    _add_input_arguments(p, include_output=True, default_dialect=default_dialect)
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit event ordering as JSON.",
    )
    p.set_defaults(handler=_run_event_order)


@verb(
    "event-info",
    aliases=("eventinfo",),
    help="Look up iRules event metadata and valid commands.",
)
def _configure_event_info(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    p.add_argument(
        "event",
        help="iRules event name (for example: HTTP_REQUEST).",
    )
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit event metadata as JSON.",
    )
    p.add_argument(
        "--output",
        "-o",
        default="-",
        help="Output path ('-' for stdout).",
    )
    p.set_defaults(handler=_run_event_info)


@verb(
    "command-info",
    aliases=("commandinfo", "cmd-info"),
    help="Look up command registry metadata.",
    formatter_class=argparse.HelpFormatter,
)
def _configure_command_info(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    p.description = "Look up command registry metadata."
    p.add_argument(
        "command",
        help="Command name to query (for example: HTTP::uri or string).",
    )
    p.add_argument(
        "--dialect",
        choices=AVAILABLE_DIALECTS,
        default=default_dialect,
        help=(f"Dialect profile for command metadata lookup (default: {default_dialect})."),
    )
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit command metadata as JSON.",
    )
    p.add_argument(
        "--output",
        "-o",
        default="-",
        help="Output path ('-' for stdout).",
    )
    p.set_defaults(handler=_run_command_info)


@verb(
    "help",
    aliases=("docs",),
    help="Search KCS help docs from the bundled SQLite index.",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure_help(
    p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str
) -> None:
    help_default_dialect = "f5-irules" if default_dialect == "f5-irules" else "all"
    p.description = "Search KCS help docs from the bundled SQLite index."
    p.epilog = (
        "Examples:\n"
        f"  {prog_name} help taint\n"
        f"  {prog_name} help event --dialect f5-irules\n"
        f"  {prog_name} help --dialect tcl8.6 --json\n"
    )
    p.add_argument(
        "query",
        nargs="*",
        help="Search terms (omit to list available help sections).",
    )
    p.add_argument(
        "--dialect",
        choices=("all", *AVAILABLE_DIALECTS),
        default=help_default_dialect,
        help=(f"Filter help matches by dialect context (default: {help_default_dialect})."),
    )
    p.add_argument(
        "--limit",
        type=int,
        default=20,
        help="Maximum number of help search matches (default: 20).",
    )
    p.add_argument(
        "--json",
        action="store_true",
        help="Emit help results as JSON.",
    )
    p.set_defaults(handler=_run_help)


# ---------------------------------------------------------------------------
# Help query helpers
# ---------------------------------------------------------------------------


def _load_help_queries():
    try:
        from core.help.kcs_db import list_features, search_help
    except Exception as exc:
        raise TclCliError(
            "KCS help database is unavailable. Build it with 'make kcs-db' "
            "and rebuild the zipapp artifact."
        ) from exc
    return list_features, search_help


def _print_help_catalogue(catalogue: dict[str, list[dict[str, object]]]) -> None:
    if not catalogue:
        print("no help entries found")
        return

    for category in sorted(catalogue.keys(), key=str.lower):
        features = sorted(
            catalogue[category],
            key=lambda item: str(item.get("name", "")).lower(),
        )
        print(f"{category} ({len(features)}):")
        for feature in features:
            name = str(feature.get("name", "")).strip() or "<unnamed>"
            summary = str(feature.get("summary", "")).strip()
            if summary:
                print(f"  {name}: {summary}")
            else:
                print(f"  {name}")


def _print_help_search_results(query: str, results: list[dict[str, object]]) -> None:
    if not results:
        print(f"no KCS help matches for '{query}'", file=sys.stderr)
        return

    match_word = "match" if len(results) == 1 else "matches"
    print(f"{len(results)} {match_word} for '{query}':")
    for result in results:
        name = str(result.get("name", "")).strip() or "<unnamed>"
        category = str(result.get("category", "")).strip()
        summary = str(result.get("summary", "")).strip()
        file_name = str(result.get("file", "")).strip()

        heading = name if not category else f"{name} [{category}]"
        print(f"- {heading}")
        if summary:
            print(f"  {summary}")
        if file_name:
            print(f"  file: {file_name}")


def _dialect_help_terms(dialect: str) -> tuple[str, ...]:
    if dialect == "all":
        return ()
    return _HELP_DIALECT_TERMS.get(dialect, ())


def _help_entry_matches_dialect(entry: dict[str, object], dialect: str) -> bool:
    terms = _dialect_help_terms(dialect)
    if not terms:
        return True

    searchable_parts = (
        str(entry.get("name", "")),
        str(entry.get("summary", "")),
        str(entry.get("surface", "")),
        str(entry.get("category", "")),
        str(entry.get("how_to_use", "")),
        str(entry.get("file", "")),
    )
    searchable = " ".join(searchable_parts).lower()
    return any(term in searchable for term in terms)


def _filter_help_results_by_dialect(
    results: list[dict[str, object]],
    *,
    dialect: str,
    limit: int,
) -> list[dict[str, object]]:
    filtered = [item for item in results if _help_entry_matches_dialect(item, dialect)]
    return filtered[:limit]


def _filter_catalogue_by_dialect(
    catalogue: dict[str, list[dict[str, object]]],
    *,
    dialect: str,
) -> dict[str, list[dict[str, object]]]:
    if dialect == "all":
        return catalogue

    filtered_catalogue: dict[str, list[dict[str, object]]] = {}
    for category, features in catalogue.items():
        matched_features = []
        for feature in features:
            feature_with_category = dict(feature)
            feature_with_category.setdefault("category", category)
            if _help_entry_matches_dialect(feature_with_category, dialect):
                matched_features.append(feature)
        if matched_features:
            filtered_catalogue[category] = matched_features
    return filtered_catalogue


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


def _run_command_info(args: argparse.Namespace) -> int:
    try:
        info = lookup_command_info(args.command, dialect=args.dialect)
    except ValueError as exc:
        raise TclCliError(str(exc)) from exc

    if not info.found:
        payload = {
            "found": False,
            "command": args.command.strip(),
            "dialect": args.dialect,
        }
        if args.json:
            _write_text_output(args.output, json.dumps(payload, indent=2))
        else:
            _write_text_output(
                args.output,
                f"command not found: {args.command.strip()} (dialect={args.dialect})",
            )
        return 1

    payload = {
        "found": True,
        "command": info.command,
        "dialect": info.dialect,
        "summary": info.summary,
        "synopsis": list(info.synopsis),
        "switches": list(info.switches),
        "validEvents": list(info.valid_events),
    }
    if args.json:
        _write_text_output(args.output, json.dumps(payload, indent=2))
        return 0

    lines = [f"command: {info.command}", f"dialect: {args.dialect}"]
    if payload["summary"]:
        lines.append(f"summary: {payload['summary']}")
    if payload["synopsis"]:
        lines.extend(f"synopsis: {item}" for item in payload["synopsis"])
    if info.switches:
        lines.append(f"switches: {', '.join(info.switches)}")
    if info.valid_events:
        lines.append(
            f"valid events ({len(info.valid_events)}): {', '.join(info.valid_events[:20])}"
        )
    _write_text_output(args.output, "\n".join(lines))
    return 0


def _run_help(args: argparse.Namespace) -> int:
    if args.limit < 1:
        raise TclCliError("--limit must be >= 1")

    import explorer.tcl_cli as _cli_mod

    list_features, search_help = _cli_mod._load_help_queries()
    query = " ".join(args.query).strip()

    if not query:
        catalogue = list_features()
        catalogue = _filter_catalogue_by_dialect(catalogue, dialect=args.dialect)
        if args.json:
            print(json.dumps(catalogue, indent=2))
        else:
            _print_help_catalogue(catalogue)
        return 0 if catalogue else 1

    search_limit = args.limit if args.dialect == "all" else max(args.limit * 5, args.limit)
    results = search_help(query, limit=search_limit)
    results = _filter_help_results_by_dialect(
        results,
        dialect=args.dialect,
        limit=args.limit,
    )
    if args.json:
        print(json.dumps(results, indent=2))
    else:
        _print_help_search_results(query, results)

    if results:
        return 0

    if not args.json:
        catalogue = list_features()
        if catalogue:
            print("available sections:", file=sys.stderr)
            for category in sorted(catalogue.keys(), key=str.lower):
                print(f"  {category}", file=sys.stderr)
    return 1
