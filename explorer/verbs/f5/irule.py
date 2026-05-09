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

    # ── lint ───────────────────────────────────────────────────────────
    lint_p = irule_sub.add_parser(
        "lint",
        help="Run iRule-only lint rules over a bigip.conf / SCF.",
        description=(
            "Apply only the irule-category lint rules from `f5 validate`: "
            "deprecated commands, empty when blocks, unknown events, etc."
        ),
    )
    lint_p.add_argument("paths", nargs="+", help="bigip.conf / SCF files (`-` for stdin).")
    lint_p.add_argument("--json", action="store_true", help="Emit JSON instead of text.")
    lint_p.add_argument(
        "--severity",
        choices=("error", "warning", "info"),
        help="Filter to one severity level.",
    )
    lint_p.add_argument("-o", "--output", metavar="FILE", help="Write here (default: stdout).")
    lint_p.set_defaults(handler=_run_irule_lint)

    # ── trace ──────────────────────────────────────────────────────────
    trace_p = irule_sub.add_parser(
        "trace",
        help="Static event-flow trace from a starting event.",
        description=(
            "List every command an iRule fires when a given event is "
            "triggered, plus references to pools / data-groups / "
            "persistence found inside the event's `when` block."
        ),
    )
    trace_p.add_argument("event", help="Starting event name (e.g. HTTP_REQUEST).")
    trace_p.add_argument("paths", nargs="+", help="bigip.conf / SCF files (`-` for stdin).")
    trace_p.add_argument("--json", action="store_true", help="Emit JSON instead of text.")
    trace_p.add_argument("-o", "--output", metavar="FILE", help="Write here (default: stdout).")
    trace_p.set_defaults(handler=_run_irule_trace)

    # ── extract ────────────────────────────────────────────────────────
    extract_p = irule_sub.add_parser(
        "extract",
        help="Write each iRule body to a standalone .tcl file.",
        description=(
            "For every `ltm rule` in the input config, write its source "
            "body to OUTDIR/<full-path-with-slashes-flattened>.tcl, "
            "ready to load into an LSP-aware editor for editing."
        ),
    )
    extract_p.add_argument("paths", nargs="+", help="bigip.conf / SCF files (`-` for stdin).")
    extract_p.add_argument("output", help="Output directory (created if needed).")
    extract_p.set_defaults(handler=_run_irule_extract)


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def _run_irule_lint(args: argparse.Namespace) -> int:
    import sys
    from pathlib import Path

    from core.bigip.lint import run_lint

    from ._paths import load_paths
    from .validate import _to_json, _to_text

    try:
        sources, configs = load_paths(args.paths)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    findings = run_lint(
        sources=sources,
        configs=configs,
        category="irule",
        severity=args.severity,
    )

    if args.json:
        output = json.dumps(_to_json(findings), indent=2) + "\n"
    else:
        output = _to_text(findings) + "\n"

    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)

    has_error = any(f.severity == "error" for f in findings)
    has_warning = any(f.severity == "warning" for f in findings)
    if has_error:
        return 2
    if has_warning:
        return 1
    return 0


def _run_irule_trace(args: argparse.Namespace) -> int:
    import re
    import sys
    from pathlib import Path

    from ._paths import load_paths

    try:
        _sources, configs = load_paths(args.paths)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    block_re = re.compile(
        rf"\bwhen\s+{re.escape(args.event)}\s*\{{",
        re.IGNORECASE,
    )

    traces: list[dict] = []
    for cfg in configs.values():
        for rule_path, rule in cfg.rules.items():
            match = block_re.search(rule.source)
            if not match:
                continue
            body = _slice_balanced_braces(rule.source, match.end() - 1)
            commands = _extract_commands(body)
            traces.append(
                {
                    "rule": rule_path,
                    "commandCount": len(commands),
                    "commands": commands,
                }
            )

    if args.json:
        out = json.dumps({"event": args.event, "traces": traces}, indent=2) + "\n"
    else:
        lines = [f"event {args.event}: {len(traces)} matching rule(s)"]
        for trace in traces:
            lines.append(f"  {trace['rule']} — {trace['commandCount']} command(s)")
            for cmd in trace["commands"]:
                lines.append(f"    {cmd}")
        out = "\n".join(lines) + "\n"

    if args.output:
        Path(args.output).write_text(out, encoding="utf-8")
    else:
        sys.stdout.write(out)
    return 0 if traces else 1


def _slice_balanced_braces(source: str, open_brace: int) -> str:
    """Return body text inside the `{...}` starting at *open_brace*."""
    if source[open_brace] != "{":
        return ""
    depth = 1
    i = open_brace + 1
    start = i
    while i < len(source) and depth > 0:
        ch = source[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        elif ch == '"':
            i += 1
            while i < len(source) and source[i] != '"':
                if source[i] == "\\":
                    i += 1
                i += 1
        i += 1
    return source[start : i - 1]


def _extract_commands(body: str) -> list[str]:
    """Yield the first token of each non-blank, non-comment line in *body*."""
    cmds: list[str] = []
    for raw in body.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        # Cheap tokenisation: first whitespace-delimited word.
        head = line.split(None, 1)[0]
        # Strip stray closing brace from one-liners
        head = head.rstrip("{};")
        if head:
            cmds.append(head)
    return cmds


def _run_irule_extract(args: argparse.Namespace) -> int:
    import sys
    from pathlib import Path

    from ._paths import load_paths

    try:
        _sources, configs = load_paths(args.paths)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    out_dir = Path(args.output)
    out_dir.mkdir(parents=True, exist_ok=True)

    written = 0
    for cfg in configs.values():
        for rule_path, rule in cfg.rules.items():
            flat = rule_path.lstrip("/").replace("/", "__")
            (out_dir / f"{flat}.tcl").write_text(rule.source + "\n", encoding="utf-8")
            written += 1

    print(f"extracted {written} iRule(s) to {out_dir}", file=sys.stderr)
    return 0


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
