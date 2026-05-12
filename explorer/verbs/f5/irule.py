"""``f5 irule`` — verb group with iRules-specific sub-subcommands.

Sub-subcommands:

- ``f5 irule event-order`` — show iRules events in canonical firing order.
- ``f5 irule event-info``  — look up iRules event metadata and valid commands.
- ``f5 irule lint``        — apply iRule-only lint rules.
- ``f5 irule trace``       — static event-flow trace from a starting event.
- ``f5 irule extract``     — split a bigip.conf / SCF / UCS into per-rule files.
- ``f5 irule format``      — pretty-print iRule source.
- ``f5 irule minify``      — strip whitespace/comments from iRule source.

Every sub-subcommand except ``extract`` and ``event-info`` accepts the
same flexible mix of inputs: individual ``.irul`` / ``.irule`` / ``.tcl``
files, ``bigip.conf`` / SCF files containing ``ltm rule`` stanzas, UCS
archives, ``--source`` snippets, or stdin.  ``extract`` only consumes
configs that may *contain* iRules (bigip.conf / SCF / UCS).

These verbs are purely iRules-focused and are not available on the
general-purpose ``tcl`` CLI.  Generally-useful verbs (``command-info``,
``convert``, ``help``, …) remain on ``tcl`` and accept ``--dialect
f5-irules`` for iRules input.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from core.bigip.model import BigipConfig
from core.commands.registry.info import lookup_event_info
from core.commands.registry.namespace_data import event_multiplicity, order_events_for_file
from core.commands.registry.runtime import configure_signatures

from ...pipeline import AVAILABLE_DIALECTS
from .._utils import (
    _add_colour_arguments,
    _add_formatter_arguments,
    _resolve_formatter_config,
    _resolve_tab_width,
    _resolve_use_colour,
    _write_highlighted_output,
    _write_text_output,
)
from ._paths import IruleInput, load_irule_inputs


def _add_irule_input_arguments(
    parser: argparse.ArgumentParser,
    *,
    include_output: bool = True,
) -> None:
    """Add the standard input arguments shared by irule sub-verbs.

    Inputs may mix .irul / .irule / .tcl files, bigip.conf / SCF, UCS
    archives, and ``-`` for stdin.  ``--source`` adds inline iRule
    snippets (repeatable).
    """
    paths = parser.add_argument(
        "paths",
        nargs="*",
        help=("Input files: .tcl/.irul/.irule, bigip.conf/SCF, or .ucs.  Pass `-` to read stdin."),
    )
    try:
        from argcomplete.completers import FilesCompleter

        paths.completer = FilesCompleter(  # type: ignore[attr-defined]
            allowednames=("tcl", "irul", "irule", "conf", "scf", "ucs"),
            directories=True,
        )
    except ImportError:
        pass
    parser.add_argument(
        "--source",
        action="append",
        default=[],
        help="Inline iRule source text (can be repeated).",
    )
    parser.add_argument(
        "--dialect",
        choices=AVAILABLE_DIALECTS,
        default="f5-irules",
        help="Dialect profile (default: f5-irules).",
    )
    if include_output:
        parser.add_argument(
            "-o",
            "--output",
            default="-",
            help=(
                "Output path: '-' for stdout, an existing directory (or a "
                "trailing-slash path) for one file per iRule, or any other "
                "path for a single concatenated file."
            ),
        )


def add_irule_subparser(
    sub: argparse._SubParsersAction,  # noqa: SLF001
    *,
    prog_name: str = "f5",
    default_dialect: str = "f5-irules",  # noqa: ARG001 — kept for parity with other registrars
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
            f"  {prog_name} irule format rule.irule\n"
            f"  {prog_name} irule minify bigip.conf -o minified/\n"
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
    _add_irule_input_arguments(eo_p)
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
        help="Run iRule-only lint rules over iRule sources.",
        description=(
            "Apply only the irule-category lint rules from `f5 validate`: "
            "deprecated commands, empty when blocks, unknown events, etc.  "
            "Accepts .irul / .irule / .tcl files, bigip.conf / SCF, UCS, "
            "or --source snippets."
        ),
    )
    _add_irule_input_arguments(lint_p)
    lint_p.add_argument("--json", action="store_true", help="Emit JSON instead of text.")
    lint_p.add_argument(
        "--severity",
        choices=("error", "warning", "info"),
        help="Filter to one severity level.",
    )
    lint_p.set_defaults(handler=_run_irule_lint)

    # ── trace ──────────────────────────────────────────────────────────
    trace_p = irule_sub.add_parser(
        "trace",
        help="Static event-flow trace from a starting event.",
        description=(
            "List every command an iRule fires when a given event is "
            "triggered, plus references to pools / data-groups / "
            "persistence found inside the event's `when` block.  "
            "Accepts .irul / .irule / .tcl files, bigip.conf / SCF, UCS, "
            "or --source snippets."
        ),
    )
    trace_p.add_argument("event", help="Starting event name (e.g. HTTP_REQUEST).")
    _add_irule_input_arguments(trace_p)
    trace_p.add_argument("--json", action="store_true", help="Emit JSON instead of text.")
    trace_p.set_defaults(handler=_run_irule_trace)

    # ── extract ────────────────────────────────────────────────────────
    extract_p = irule_sub.add_parser(
        "extract",
        help="Write each iRule body in a config to a standalone .tcl file.",
        description=(
            "For every `ltm rule` in the input config, write its source "
            "body to OUTDIR/<full-path-with-slashes-flattened>.tcl, "
            "ready to load into an LSP-aware editor for editing.  "
            "Accepts bigip.conf / SCF / UCS only — standalone .irule "
            "files are already in the desired form."
        ),
    )
    extract_p.add_argument(
        "paths",
        nargs="+",
        help="bigip.conf / SCF / UCS files (`-` for stdin).",
    )
    extract_p.add_argument("output", help="Output directory (created if needed).")
    extract_p.set_defaults(handler=_run_irule_extract)

    # ── format ─────────────────────────────────────────────────────────
    fmt_p = irule_sub.add_parser(
        "format",
        aliases=["fmt"],
        help="Format iRule source and emit rewritten Tcl.",
        description=(
            "Pretty-print iRule source with canonical indentation and "
            "spacing.  Accepts .irul / .irule / .tcl files, bigip.conf "
            "/ SCF / UCS, --source snippets, or stdin.  When the input "
            "yields a single iRule, the formatted body is written to "
            "stdout (or -o FILE).  When it yields more than one, -o "
            "must point at a directory and one file per rule is written."
        ),
        epilog=(
            "Examples:\n"
            f"  {prog_name} irule format rule.irule\n"
            f"  {prog_name} irule format --source 'when HTTP_REQUEST {{ return }}'\n"
            f"  {prog_name} irule format bigip.conf -o formatted/\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    _add_irule_input_arguments(fmt_p)
    _add_colour_arguments(fmt_p)
    _add_formatter_arguments(fmt_p)
    fmt_p.set_defaults(handler=_run_irule_format)

    # ── minify ─────────────────────────────────────────────────────────
    min_p = irule_sub.add_parser(
        "minify",
        aliases=["min"],
        help="Minify iRule source: strip comments, collapse whitespace.",
        description=(
            "Minify iRule source: strip comments, collapse whitespace, "
            "and join commands with semicolons.  Accepts .irul / .irule "
            "/ .tcl files, bigip.conf / SCF / UCS, --source snippets, "
            "or stdin.  When the input yields more than one iRule, -o "
            "must point at a directory and one file per rule is written."
        ),
        epilog=(
            "Examples:\n"
            f"  {prog_name} irule minify rule.irule\n"
            f"  {prog_name} irule minify rule.irule -o min.irule\n"
            f"  {prog_name} irule minify --compact rule.irule --symbol-map symbols.txt\n"
            f"  {prog_name} irule minify --aggressive bigip.conf -o tiny/\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    _add_irule_input_arguments(min_p)
    min_p.add_argument(
        "--compact",
        action="store_true",
        help="Compact variable and proc names to short identifiers.",
    )
    min_p.add_argument(
        "--symbol-map",
        metavar="FILE",
        default=None,
        help="Write symbol map (original -> compacted names) to FILE.",
    )
    min_p.add_argument(
        "--aggressive",
        action="store_true",
        help="Maximum compression: run all optimiser passes, then compact names and minify.",
    )
    min_p.add_argument(
        "--isolated",
        action="store_true",
        help="Treat script as self-contained — also compact global-scope variable names.",
    )
    _add_colour_arguments(min_p)
    min_p.set_defaults(handler=_run_irule_minify)

    # ── context ────────────────────────────────────────────────────────
    ctx_p = irule_sub.add_parser(
        "context",
        help="Bundle each iRule with the BIG-IP objects it references.",
        description=(
            "For every input iRule, emit the rule body together with "
            "every BIG-IP object it references (pools, data groups, "
            "persistence profiles, SNAT pools, profiles, monitors, "
            "nodes, called rules) plus a one-hop transitive expansion "
            "(pool members → nodes; pool monitor → monitor).  "
            "Designed to be dropped into an LLM prompt or consumed by "
            "AI tooling that needs the full context for an iRule."
        ),
        epilog=(
            "Examples:\n"
            f"  {prog_name} irule context bigip.conf\n"
            f"  {prog_name} irule context bigip.conf --json\n"
            f"  {prog_name} irule context bigip.conf -o ctx/  # one file per rule\n"
            f"  {prog_name} irule context bigip.conf --rule /Common/foo\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    _add_irule_input_arguments(ctx_p)
    ctx_p.add_argument(
        "--rule",
        action="append",
        default=[],
        metavar="FULL_PATH",
        help="Limit output to rules with these full paths (repeatable).",
    )
    ctx_p.add_argument(
        "--no-transitive",
        action="store_true",
        help="Skip the one-hop transitive expansion (pool→node, pool→monitor).",
    )
    ctx_p.add_argument(
        "--json",
        action="store_true",
        help="Emit each bundle as a JSON object (default: Tcl-flavoured text).",
    )
    ctx_p.set_defaults(handler=_run_irule_context)


# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------


def _resolve_irule_inputs(
    args: argparse.Namespace,
) -> tuple[list[IruleInput], dict[str, BigipConfig], dict[str, str]] | int:
    """Run :func:`load_irule_inputs` with CLI error handling.

    Returns the loader's ``(inputs, configs, sources)`` tuple on
    success or an exit code (already-printed error) on failure.
    """
    paths = list(args.paths or [])
    inline = list(getattr(args, "source", None) or [])
    if not paths and not inline:
        print("error: no input provided; pass files, --source, or `-` for stdin", file=sys.stderr)
        return 2
    try:
        return load_irule_inputs(paths, inline_sources=inline)
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2


def _flatten_rule_path(rule_full_path: str | None, fallback_label: str) -> str:
    """Convert a rule path (or label) to a safe file-stem."""
    if rule_full_path:
        return rule_full_path.lstrip("/").replace("/", "__")
    stem = Path(fallback_label).name
    if stem in ("", "<stdin>", "-"):
        stem = "irule"
    if stem.endswith(tuple(_KNOWN_SUFFIXES)):
        stem = Path(stem).stem
    return stem.replace("/", "__")


_KNOWN_SUFFIXES = (".tcl", ".irul", ".irule", ".conf", ".scf", ".ucs")


def _is_directory_target(output: str) -> bool:
    """Decide whether *output* should be treated as a directory.

    Stdout (``-``) is never a directory.  Otherwise we treat *output*
    as a directory when it already exists as one or ends with a path
    separator — both single- and multi-rule input honour this so the
    operator picks the shape with the path syntax (``-o out/`` for a
    directory, ``-o out.irule`` for one concatenated file).
    """
    if output == "-":
        return False
    p = Path(output)
    return p.is_dir() or output.endswith(("/", "\\"))


# ---------------------------------------------------------------------------
# Handlers
# ---------------------------------------------------------------------------


def _run_irule_lint(args: argparse.Namespace) -> int:
    from core.bigip.lint import run_lint

    from .validate import _to_json, _to_text

    loaded = _resolve_irule_inputs(args)
    if isinstance(loaded, int):
        return loaded
    _inputs, configs, _sources = loaded

    # The iRule lint category only inspects rule bodies, so the
    # ``sources`` argument can be the rule bodies joined back together
    # by origin URI.
    sources_for_lint = {
        origin: "\n".join(rule.source for rule in cfg.rules.values())
        for origin, cfg in configs.items()
    }

    findings = run_lint(
        sources=sources_for_lint,
        configs=configs,
        category="irule",
        severity=args.severity,
    )

    if args.json:
        output = json.dumps(_to_json(findings), indent=2) + "\n"
    else:
        output = _to_text(findings) + "\n"

    _write_text_output(args.output, output)

    has_error = any(f.severity == "error" for f in findings)
    has_warning = any(f.severity == "warning" for f in findings)
    if has_error:
        return 2
    if has_warning:
        return 1
    return 0


def _run_irule_trace(args: argparse.Namespace) -> int:
    from core.bigip.irules_refs import extract_irules_object_references
    from core.bigip.lint import _classify_reference_kind, _resolve_reference

    loaded = _resolve_irule_inputs(args)
    if isinstance(loaded, int):
        return loaded
    inputs, configs, _sources = loaded
    merged = _merge_configs_for_trace(configs)

    block_re = re.compile(
        rf"\bwhen\s+{re.escape(args.event)}\s*\{{",
        re.IGNORECASE,
    )

    traces: list[dict] = []
    for entry in inputs:
        match = block_re.search(entry.source)
        if not match:
            continue
        body = _slice_balanced_braces(entry.source, match.end() - 1)
        commands = _extract_commands(body)
        rule_label = entry.rule_full_path or entry.label

        # Extract object references from the *event-handler body* so
        # `references` only covers what this event actually touches.
        refs_payload: list[dict] = []
        seen: set[tuple[str, str]] = set()
        for ref in extract_irules_object_references(body):
            kind = _classify_reference_kind(ref.kinds)
            if kind is None:
                continue
            key = (kind, ref.name)
            if key in seen:
                continue
            seen.add(key)
            resolved_path = _resolve_reference(merged, kind, ref.name)
            refs_payload.append(
                {
                    "kind": kind,
                    "name": ref.name,
                    "command": ref.command,
                    "resolved": resolved_path is not None,
                    "resolvedPath": resolved_path,
                }
            )

        traces.append(
            {
                "rule": rule_label,
                "commandCount": len(commands),
                "commands": commands,
                "references": refs_payload,
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
            for ref in trace["references"]:
                marker = "✓" if ref["resolved"] else "✗"
                target = ref["resolvedPath"] or ref["name"]
                lines.append(f"    {marker} {ref['kind']}: {target}")
        out = "\n".join(lines) + "\n"

    _write_text_output(args.output, out)
    return 0 if traces else 1


def _merge_configs_for_trace(configs: dict[str, BigipConfig]) -> BigipConfig:
    """Lazy import so we don't pull lint code at module load."""
    from core.bigip.lint import _merge_configs

    if not configs:
        return BigipConfig()
    if len(configs) == 1:
        return next(iter(configs.values()))
    return _merge_configs(configs)


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
        head = line.split(None, 1)[0]
        head = head.rstrip("{};")
        if head:
            cmds.append(head)
    return cmds


def _run_irule_extract(args: argparse.Namespace) -> int:
    # Extract pulls iRule bodies *out* of a config — standalone .irule
    # / .irul / .tcl files are already in the desired form, so reject
    # them up front to match the documented contract.
    paths = list(args.paths or [])
    if not paths:
        print("error: no input provided; pass bigip.conf / SCF / UCS files", file=sys.stderr)
        return 2
    standalone = {".tcl", ".irul", ".irule"}
    bad = [p for p in paths if p != "-" and Path(p).suffix.lower() in standalone]
    if bad:
        joined = ", ".join(bad)
        print(
            f"error: extract only accepts bigip.conf / SCF / UCS; refusing standalone "
            f"iRule file(s): {joined}",
            file=sys.stderr,
        )
        return 2
    try:
        _inputs, configs, _sources = load_irule_inputs(paths)
    except (OSError, ValueError) as exc:
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
    loaded = _resolve_irule_inputs(args)
    if isinstance(loaded, int):
        return loaded
    inputs, _configs, _sources = loaded

    configure_signatures(dialect=args.dialect)
    combined = "\n\n".join(entry.source.rstrip("\n") for entry in inputs)

    ordered = order_events_for_file(combined)
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


# ---------------------------------------------------------------------------
# format / minify
# ---------------------------------------------------------------------------


def _write_iRule_outputs(
    *,
    args: argparse.Namespace,
    inputs: list[IruleInput],
    transformed: list[str],
    file_extension: str,
) -> int:
    """Common output dispatcher for irule format/minify.

    Output shape is selected by the path passed to ``-o`` regardless of
    how many iRules came out of the loader:

    - ``-`` (default) → write the concatenated text to stdout.
    - existing directory or trailing slash (``out/``) → write one file
      per rule, named ``<flatten(rule_full_path)><file_extension>``.
    - any other path → write the concatenated text to that single file.

    For the concatenated forms, multi-rule output is separated by a
    ``# ===== <rule_full_path or label> =====`` header per rule so the
    boundary is obvious in the combined stream.
    """
    target_is_dir = _is_directory_target(args.output)
    use_colour = _resolve_use_colour(args)
    tab_width = _resolve_tab_width(args)

    if target_is_dir:
        out_dir = Path(args.output)
        out_dir.mkdir(parents=True, exist_ok=True)
        for entry, text in zip(inputs, transformed, strict=True):
            stem = _flatten_rule_path(entry.rule_full_path, entry.label)
            target = out_dir / f"{stem}{file_extension}"
            target.write_text(
                text if text.endswith("\n") else text + "\n",
                encoding="utf-8",
            )
        print(f"wrote {len(transformed)} iRule(s) to {out_dir}", file=sys.stderr)
        return 0

    # Single-file / stdout: emit the concatenation.  For one rule this
    # is just the text; for many it gets per-rule banners.
    if len(transformed) <= 1:
        text = transformed[0] if transformed else ""
    else:
        chunks: list[str] = []
        for entry, body in zip(inputs, transformed, strict=True):
            label = entry.rule_full_path or entry.label
            chunks.append(f"# ===== {label} =====\n{body.rstrip()}\n")
        text = "\n".join(chunks)
    _write_highlighted_output(args.output, text, use_colour=use_colour, tab_width=tab_width)
    return 0


def _run_irule_format(args: argparse.Namespace) -> int:
    from core.formatting import format_tcl

    loaded = _resolve_irule_inputs(args)
    if isinstance(loaded, int):
        return loaded
    inputs, _configs, _sources = loaded

    configure_signatures(dialect=args.dialect)
    fmt_cfg = _resolve_formatter_config(args)
    formatted = [format_tcl(entry.source, config=fmt_cfg) for entry in inputs]
    return _write_iRule_outputs(
        args=args,
        inputs=inputs,
        transformed=formatted,
        file_extension=".irule",
    )


def _run_irule_minify(args: argparse.Namespace) -> int:
    from core.minifier import minify_tcl

    loaded = _resolve_irule_inputs(args)
    if isinstance(loaded, int):
        return loaded
    inputs, _configs, _sources = loaded

    configure_signatures(dialect=args.dialect)
    isolated = bool(getattr(args, "isolated", False))

    minified: list[str] = []
    symbol_maps: list[str] = []
    if args.aggressive:
        for entry in inputs:
            result = minify_tcl(
                entry.source,
                aggressive=True,
                isolated=isolated,
                dialect=args.dialect,
            )
            minified.append(result.source)
            symbol_maps.append(result.symbol_map.format())
    elif args.compact:
        for entry in inputs:
            text, symbol_map = minify_tcl(
                entry.source,
                compact_names=True,
                isolated=isolated,
                dialect=args.dialect,
            )
            minified.append(text)
            symbol_maps.append(symbol_map.format())
    else:
        for entry in inputs:
            minified.append(minify_tcl(entry.source, dialect=args.dialect))

    rc = _write_iRule_outputs(
        args=args,
        inputs=inputs,
        transformed=minified,
        file_extension=".irule",
    )
    if rc != 0:
        return rc

    if args.symbol_map and symbol_maps:
        joined = "\n\n".join(
            f"# {entry.rule_full_path or entry.label}\n{m}".rstrip()
            for entry, m in zip(inputs, symbol_maps, strict=True)
            if m
        )
        _write_text_output(args.symbol_map, joined + ("\n" if joined else ""))
    return 0


# ---------------------------------------------------------------------------
# context
# ---------------------------------------------------------------------------


def _run_irule_context(args: argparse.Namespace) -> int:
    from ai.shared.irule_context import (
        build_irule_context,
        context_bundle_to_dict,
        context_bundle_to_text,
    )

    loaded = _resolve_irule_inputs(args)
    if isinstance(loaded, int):
        return loaded
    _inputs, configs, sources_by_origin = loaded

    configure_signatures(dialect=args.dialect)

    # Merge configs once so cross-file references (e.g. SCF split into
    # multiple files) all resolve.
    merged = _merge_configs_for_trace(configs)

    rule_filter = set(args.rule or [])
    transitive = not args.no_transitive

    bundles = []
    for cfg_origin, cfg in configs.items():
        for rule_path, rule in cfg.rules.items():
            if rule_filter and rule_path not in rule_filter:
                continue
            bundle = build_irule_context(
                rule,
                merged,
                transitive=transitive,
                sources=sources_by_origin or None,
                config_origin=cfg_origin,
                dialect=args.dialect,
            )
            bundles.append((rule_path, bundle))

    if not bundles:
        print("error: no iRules found in input", file=sys.stderr)
        return 1

    target_is_dir = _is_directory_target(args.output)

    if target_is_dir:
        out_dir = Path(args.output)
        out_dir.mkdir(parents=True, exist_ok=True)
        suffix = ".json" if args.json else ".tcl"
        for rule_path, bundle in bundles:
            flat = rule_path.lstrip("/").replace("/", "__")
            if args.json:
                payload = json.dumps(context_bundle_to_dict(bundle), indent=2) + "\n"
            else:
                payload = context_bundle_to_text(bundle)
            (out_dir / f"{flat}{suffix}").write_text(payload, encoding="utf-8")
        print(f"wrote {len(bundles)} iRule context bundle(s) to {out_dir}", file=sys.stderr)
        return 0

    # Single-file / stdout: concatenate.  JSON gets a single ``bundles``
    # array; text mode keeps each block contiguous so the existing
    # ``# ===== iRule …`` headers separate them naturally.
    if args.json:
        payload = json.dumps(
            {"bundles": [context_bundle_to_dict(b) for _, b in bundles]},
            indent=2,
        )
        _write_text_output(args.output, payload + "\n")
    else:
        chunks = [context_bundle_to_text(b).rstrip() + "\n" for _, b in bundles]
        _write_text_output(args.output, "\n".join(chunks))
    return 0
