"""iRules control-flow checks.

Walks the token stream of ``when`` bodies to detect:

- **IRULE1005** (WARNING): ``*_DATA`` event handler without a matching
  ``*::collect`` call elsewhere in the iRule — the event will never fire.
- **IRULE1006** (WARNING): ``*::payload`` access without a matching
  ``*::collect`` call — the payload buffer will be empty.
- **IRULE1007** (ERROR): ``*::collect`` without a matching ``*::release``
  on the same connection side — collected data is never released.
- **IRULE1008** (ERROR): ``*::release`` without a matching ``*::collect``
  on the same connection side — nothing was collected.
- **IRULE1201** (WARNING): HTTP command used after ``HTTP::respond`` or
  ``HTTP::redirect`` in the same sequential code path.
- **IRULE1202** (WARNING): Multiple ``HTTP::respond``/``HTTP::redirect``
  calls reachable on different branches (neither dominates the other).
- **IRULE5002** (WARNING): ``drop``/``reject``/``discard`` without
  ``event disable all`` or ``return`` — other iRules continue executing.
- **IRULE5004** (WARNING): ``DNS::return`` without ``return`` — iRule
  processing continues after DNS::return.
- **IRULE4004** (INFORMATION): ``set var value`` in a per-request event
  could be hoisted to an earlier once-per-connection event because the
  value does not depend on per-request data.

These diagnostics fire only in ``f5-irules`` dialect.

Note: **IRULE2102** (repeated expensive calls) has been retired —
subsumed by **O105** (GVN/CSE) in ``compiler.gvn``.
"""

from __future__ import annotations

import logging
import re
from collections import defaultdict
from dataclasses import dataclass

from ..analysis.semantic_model import CodeFix, Range
from ..commands.registry import REGISTRY
from ..commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from ..common.codes import diag
from ..common.dialect import active_dialect
from ..common.ranges import position_from_relative, range_from_token
from ..parsing.lexer import TclLexer
from ..parsing.tokens import SourcePosition, Token, TokenType
from .compilation_unit import CompilationUnit, ensure_compilation_unit
from .ir import (
    IRAssignConst,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRReturn,
    IRScript,
    IRSwitch,
    IRTry,
    IRWhile,
)
from .lowering import lower_to_ir
from .value_shapes import parse_command_substitution

log = logging.getLogger(__name__)

# iRules flow diagnostic codes
diag("IRULE1005", "Data event without a matching `*::collect` call.", section="irules")
diag("IRULE1006", "`*::payload` without a matching `*::collect` call.", section="irules")
diag(
    "IRULE1007",
    "`*::collect` without a matching `*::release` on the same connection side.",
    section="irules",
)
diag(
    "IRULE1008",
    "`*::release` without a matching `*::collect` on the same connection side.",
    section="irules",
)
diag(
    "IRULE1201",
    "HTTP command used after `HTTP::respond`/`HTTP::redirect`.",
    section="irules",
)
diag(
    "IRULE1202",
    "Multiple `HTTP::respond`/`HTTP::redirect` on different branches.",
    section="irules",
)
diag(
    "IRULE4002",
    "Generic `static::` variable name — collision likely across iRules.",
    section="irules_variable",
)
diag(
    "IRULE4004",
    "Constant `set` in per-request event could be hoisted to an earlier once-per-connection event.",
    section="irules_variable",
)
diag(
    "IRULE5002",
    "`drop`/`reject`/`discard` without `event disable all` or `return`.",
    section="irules",
)
diag("IRULE5004", "`DNS::return` without `return`.", section="irules")


def _when_qualified(event: str, occurrence: int) -> str:
    """Build the ``::when::`` qualified name for the *occurrence*-th handler."""
    return f"::when::{event}" if occurrence == 0 else f"::when::{event}#{occurrence}"


def _when_qualified_names(
    when_bodies: list[tuple[str, int, str, Token, Token]],
) -> list[str]:
    """Return the ``::when::`` qualified name for each entry in *when_bodies*.

    Entries for the same event are numbered to match lowering order.
    """
    counts: dict[str, int] = {}
    result: list[str] = []
    for event, *_ in when_bodies:
        n = counts.get(event, 0)
        counts[event] = n + 1
        result.append(_when_qualified(event, n))
    return result


# Registry-backed command sets (cached on first use)

_COMMITS_RESPONSE: frozenset[str] | None = None
_HTTP_NS: frozenset[str] | None = None


def _commits_response_commands() -> frozenset[str]:
    """Return commands that commit an HTTP response (lazy-cached)."""
    from .side_effects import SideEffectTarget

    global _COMMITS_RESPONSE
    if _COMMITS_RESPONSE is None:
        result: set[str] = set()
        for name in REGISTRY.specs_by_name:
            hints = REGISTRY.side_effect_hints(name, dialect="f5-irules")
            if hints and any(
                h.target is SideEffectTarget.RESPONSE_COMMIT and h.writes for h in hints
            ):
                result.add(name)
        _COMMITS_RESPONSE = frozenset(result)
    return _COMMITS_RESPONSE


def _http_namespace_commands() -> frozenset[str]:
    """Return HTTP:: namespace commands relevant after response (lazy-cached)."""
    global _HTTP_NS
    if _HTTP_NS is None:
        result: set[str] = set()
        for name in REGISTRY.specs_by_name:
            if name.startswith("HTTP::"):
                result.add(name)
        _HTTP_NS = frozenset(result)
    return _HTTP_NS


# Warning dataclasses


@dataclass(frozen=True, slots=True)
class IrulesFlowWarning:
    """HTTP command used after response is committed."""

    range: Range
    code: str  # IRULE1201 or IRULE1202
    message: str
    respond_range: Range | None = None  # location of the respond/redirect
    fixes: tuple[CodeFix, ...] = ()


# Analysis


def _walk_body_commands(body_text: str, base_offset: int, base_line: int, base_col: int):
    """Yield (cmd_name, cmd_token, all_tokens) for each top-level command in body_text."""
    lexer = TclLexer(body_text, base_offset=base_offset, base_line=base_line, base_col=base_col)
    argv: list[Token] = []
    argv_texts: list[str] = []
    all_tokens: list[Token] = []
    prev_type = TokenType.EOL

    def flush():
        if argv and all_tokens:
            yield (argv_texts[0], argv[0], list(all_tokens))
        argv.clear()
        argv_texts.clear()
        all_tokens.clear()

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        match tok.type:
            case TokenType.COMMENT | TokenType.SEP:
                prev_type = tok.type
                continue
            case TokenType.EOL:
                yield from flush()
                prev_type = tok.type
                continue
            case _:
                text = tok.text

        all_tokens.append(tok)
        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv.append(tok)
            argv_texts.append(text)
        else:
            if argv_texts:
                argv_texts[-1] += text
        prev_type = tok.type

    yield from flush()


def _find_when_bodies(source: str):
    """Find ``when EVENT { body }`` blocks.

    Yields ``(event_name, base_priority, body_text, body_token, event_token)``.

    Uses the Tcl lexer to reliably parse the source and find ``when`` commands.
    Base priority is extracted from ``when EVENT priority N { body }``; defaults
    to 500.
    """
    lexer = TclLexer(source)
    argv: list[Token] = []
    argv_texts: list[str] = []
    prev_type = TokenType.EOL

    def flush():
        if len(argv) >= 3 and argv_texts[0] == "when":
            event_name = argv_texts[1]
            event_tok = argv[1]
            body_tok = argv[-1]
            if body_tok.type is TokenType.STR:
                priority = 500
                if len(argv_texts) >= 5 and argv_texts[2] == "priority":
                    try:
                        priority = int(argv_texts[3])
                    except ValueError:
                        pass
                yield (event_name, priority, argv_texts[-1], body_tok, event_tok)
        argv.clear()
        argv_texts.clear()

    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        match tok.type:
            case TokenType.COMMENT | TokenType.SEP:
                prev_type = tok.type
                continue
            case TokenType.EOL:
                yield from flush()
                prev_type = tok.type
                continue
            case _:
                text = tok.text

        if prev_type in (TokenType.SEP, TokenType.EOL):
            argv.append(tok)
            argv_texts.append(text)
        else:
            if argv_texts:
                argv_texts[-1] += text
            else:
                argv.append(tok)
                argv_texts.append(text)
        prev_type = tok.type

    yield from flush()


def _analyse_when_body(
    event: str,
    body_text: str,
    body_tok: Token,
) -> list[IrulesFlowWarning]:
    """Path-sensitive IR walk of a ``when`` body for HTTP response flow."""
    warnings: list[IrulesFlowWarning] = []

    # Only relevant for HTTP events.
    if not event.startswith("HTTP"):
        return warnings

    try:
        ir_body = lower_to_ir(body_text).top_level
    except Exception:
        log.debug("irules_flow: IR lowering failed, using sequential fallback", exc_info=True)
        # Fallback: keep a conservative sequential scan.
        responded = False
        respond_range: Range | None = None
        for cmd_name, cmd_tok, _all_tokens in _walk_body_commands(
            body_text,
            base_offset=body_tok.start.offset + 1,
            base_line=body_tok.start.line,
            base_col=body_tok.start.character + 1,
        ):
            if cmd_name in _commits_response_commands():
                if responded:
                    warnings.append(
                        IrulesFlowWarning(
                            range=range_from_token(cmd_tok),
                            code="IRULE1202",
                            message=(
                                f"Multiple '{cmd_name}' calls possible in {event}. "
                                "Only the first response takes effect."
                            ),
                            respond_range=respond_range,
                        )
                    )
                else:
                    responded = True
                    respond_range = range_from_token(cmd_tok)
            elif responded and cmd_name in _http_namespace_commands():
                warnings.append(
                    IrulesFlowWarning(
                        range=range_from_token(cmd_tok),
                        code="IRULE1201",
                        message=(
                            f"'{cmd_name}' used after response is committed. "
                            "HTTP context is invalid after HTTP::respond/HTTP::redirect."
                        ),
                        respond_range=respond_range,
                    )
                )
            elif cmd_name == "return":
                responded = False
                respond_range = None
        return warnings

    base_offset = body_tok.start.offset + 1
    base_line = body_tok.start.line
    base_col = body_tok.start.character + 1

    @dataclass(frozen=True, slots=True)
    class _FlowState:
        responded: bool
        respond_range: Range | None

    def _translate_range(r: Range) -> Range:
        start = position_from_relative(
            body_text,
            r.start.offset,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        end = position_from_relative(
            body_text,
            r.end.offset,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        return Range(start=start, end=end)

    def _dedupe(states: list[_FlowState]) -> list[_FlowState]:
        deduped: list[_FlowState] = []
        seen: set[tuple[bool, int]] = set()
        for s in states:
            rr = s.respond_range.start.offset if s.respond_range is not None else -1
            key = (s.responded, rr)
            if key in seen:
                continue
            seen.add(key)
            deduped.append(s)
        return deduped

    def _apply_command(state: _FlowState, command: str, cmd_range: Range) -> _FlowState:
        if command in _commits_response_commands():
            if state.responded:
                warnings.append(
                    IrulesFlowWarning(
                        range=cmd_range,
                        code="IRULE1202",
                        message=(
                            f"Multiple '{command}' calls possible in {event}. "
                            "Only the first response takes effect."
                        ),
                        respond_range=state.respond_range,
                    )
                )
                return state
            return _FlowState(responded=True, respond_range=cmd_range)

        if state.responded and command in _http_namespace_commands():
            warnings.append(
                IrulesFlowWarning(
                    range=cmd_range,
                    code="IRULE1201",
                    message=(
                        f"'{command}' used after response is committed. "
                        "HTTP context is invalid after HTTP::respond/HTTP::redirect."
                    ),
                    respond_range=state.respond_range,
                )
            )
        return state

    def _stmt_command(stmt) -> tuple[str, Range] | None:
        if isinstance(stmt, (IRCall, IRBarrier)):
            return stmt.command, _translate_range(stmt.range)
        if isinstance(stmt, IRAssignValue):
            parsed = stmt.value.strip()
            if parsed.startswith("[") and parsed.endswith("]"):
                inner = parsed[1:-1].strip()
                if inner:
                    command = inner.split(None, 1)[0]
                    return command, _translate_range(stmt.range)
        return None

    def _analyse_script(script: IRScript, in_states: list[_FlowState]) -> list[_FlowState]:
        states = list(in_states)
        for stmt in script.statements:
            next_states: list[_FlowState] = []
            if isinstance(stmt, IRReturn):
                # return terminates the current path.
                states = []
                break
            if isinstance(stmt, IRIf):
                for st in states:
                    branch_out: list[_FlowState] = []
                    for clause in stmt.clauses:
                        branch_out.extend(_analyse_script(clause.body, [st]))
                    if stmt.else_body is not None:
                        branch_out.extend(_analyse_script(stmt.else_body, [st]))
                    else:
                        # condition false path
                        branch_out.append(st)
                    next_states.extend(branch_out)
                states = _dedupe(next_states)
                continue
            if isinstance(stmt, IRSwitch):
                for st in states:
                    branch_out: list[_FlowState] = [st]
                    for arm in stmt.arms:
                        if arm.body is not None:
                            branch_out.extend(_analyse_script(arm.body, [st]))
                    if stmt.default_body is not None:
                        branch_out.extend(_analyse_script(stmt.default_body, [st]))
                    next_states.extend(branch_out)
                states = _dedupe(next_states)
                continue
            if isinstance(stmt, IRFor):
                for st in states:
                    iter_states = _analyse_script(stmt.body, [st])
                    next_states.append(st)  # zero iterations
                    next_states.extend(iter_states)
                states = _dedupe(next_states)
                continue
            if isinstance(stmt, IRWhile):
                for st in states:
                    iter_states = _analyse_script(stmt.body, [st])
                    next_states.append(st)  # zero iterations
                    next_states.extend(iter_states)
                states = _dedupe(next_states)
                continue
            if isinstance(stmt, IRForeach):
                for st in states:
                    iter_states = _analyse_script(stmt.body, [st])
                    next_states.append(st)
                    next_states.extend(iter_states)
                states = _dedupe(next_states)
                continue
            if isinstance(stmt, IRCatch):
                for st in states:
                    next_states.extend(_analyse_script(stmt.body, [st]))
                states = _dedupe(next_states)
                continue
            if isinstance(stmt, IRTry):
                for st in states:
                    branch_out: list[_FlowState] = []
                    branch_out.extend(_analyse_script(stmt.body, [st]))
                    for handler in stmt.handlers:
                        branch_out.extend(_analyse_script(handler.body, [st]))
                    if stmt.finally_body is not None:
                        final_out: list[_FlowState] = []
                        for mid in branch_out:
                            final_out.extend(_analyse_script(stmt.finally_body, [mid]))
                        branch_out = final_out
                    next_states.extend(branch_out or [st])
                states = _dedupe(next_states)
                continue

            for st in states:
                applied = st
                cmd = _stmt_command(stmt)
                if cmd is not None:
                    applied = _apply_command(applied, cmd[0], cmd[1])
                next_states.append(applied)
            states = _dedupe(next_states)

        return states

    _analyse_script(
        ir_body,
        [_FlowState(responded=False, respond_range=None)],
    )
    return warnings


# IRULE1005: *_DATA event without matching *::collect
# IRULE1006: *::payload access without matching *::collect

# Map DATA events to the protocol(s) whose ::collect triggers them and the
# side context required for that collect call.
DATA_EVENT_REQUIREMENTS: dict[str, tuple[tuple[str, ...], str]] = {
    "CLIENT_DATA": (("TCP", "UDP"), "client"),
    "SERVER_DATA": (("TCP", "UDP"), "server"),
    "HTTP_REQUEST_DATA": (("HTTP",), "client"),
    "HTTP_RESPONSE_DATA": (("HTTP",), "server"),
    "CLIENTSSL_DATA": (("SSL",), "client"),
    "SERVERSSL_DATA": (("SSL",), "server"),
}


def _default_collect_side(event: str) -> str:
    """Infer the default side context for commands in *event*."""
    props = EVENT_REGISTRY.get_props(event)
    if props is not None:
        if props.client_side and not props.server_side:
            return "client"
        if props.server_side and not props.client_side:
            return "server"

    event_upper = event.upper()
    if event_upper.startswith("SERVER"):
        return "server"
    if event_upper.startswith("CLIENT"):
        return "client"
    if "_RESPONSE" in event_upper:
        return "server"
    if "_REQUEST" in event_upper:
        return "client"
    return "client"


def _iter_ir_commands(script: IRScript):
    """Yield ``(command, range, stmt)`` for every IR command, recursing into bodies.

    Built on :func:`_iter_all_ir_statements` — filters to command-like
    statements only.  Note: clientside/serverside body lowering is handled
    in ``_scan_ir_event``, not here, because this function doesn't carry
    side context.
    """
    for stmt in _iter_all_ir_statements(script, lower_side_switches=False):
        if isinstance(stmt, (IRCall, IRBarrier)):
            yield stmt.command, stmt.range, stmt
        elif isinstance(stmt, IRAssignValue):
            parsed = parse_command_substitution(stmt.value)
            if parsed is not None:
                yield parsed[0], stmt.range, stmt


def _classify_command(
    cmd: str,
    rng: Range,
    side: str,
    collected: dict[str, set[str]],
    released: dict[str, set[str]],
    collect_calls: list[tuple[str, str, Range]],
    release_calls: list[tuple[str, str, Range]],
    payload_calls: list[tuple[str, str, Range]],
) -> None:
    """Classify a command as collect, release, or payload and record it."""
    if cmd.endswith("::collect"):
        protocol = cmd[: -len("::collect")].upper()
        collected.setdefault(protocol, set()).add(side)
        collect_calls.append((protocol, side, rng))
    elif cmd.endswith("::release"):
        protocol = cmd[: -len("::release")].upper()
        released.setdefault(protocol, set()).add(side)
        release_calls.append((protocol, side, rng))
    elif cmd.endswith("::payload"):
        protocol = cmd[: -len("::payload")].upper()
        payload_calls.append((protocol, side, rng))


def _lower_side_switch_body(stmt: IRCall) -> IRScript | None:
    """Lower a ``clientside``/``serverside`` body arg to IR.

    Returns ``None`` if the body cannot be lowered (e.g. syntax errors).
    """
    if not stmt.args:
        return None
    body_text = stmt.args[0]
    try:
        return lower_to_ir(body_text).top_level
    except Exception:
        return None


def _scan_ir_event(
    event: str,
    ir_body: IRScript,
    collected: dict[str, set[str]],
    released: dict[str, set[str]],
    collect_calls: list[tuple[str, str, Range]],
    release_calls: list[tuple[str, str, Range]],
    payload_calls: list[tuple[str, str, Range]],
) -> None:
    """Walk an IR event body and record collect/release/payload calls."""
    default_side = _default_collect_side(event)
    _scan_ir_body_with_side(
        ir_body,
        default_side,
        collected,
        released,
        collect_calls,
        release_calls,
        payload_calls,
    )


def _scan_ir_body_with_side(
    ir_body: IRScript,
    current_side: str,
    collected: dict[str, set[str]],
    released: dict[str, set[str]],
    collect_calls: list[tuple[str, str, Range]],
    release_calls: list[tuple[str, str, Range]],
    payload_calls: list[tuple[str, str, Range]],
) -> None:
    """Classify commands in *ir_body*, recursing into nested side-switches."""
    for cmd, rng, ir_stmt in _iter_ir_commands(ir_body):
        # ``clientside { ... }`` / ``serverside { ... }`` — lower the body
        # and recurse with the switched side context.
        if cmd in ("clientside", "serverside"):
            inner_side = "client" if cmd == "clientside" else "server"
            if isinstance(ir_stmt, IRCall):
                inner_ir = _lower_side_switch_body(ir_stmt)
                if inner_ir is not None:
                    _scan_ir_body_with_side(
                        inner_ir,
                        inner_side,
                        collected,
                        released,
                        collect_calls,
                        release_calls,
                        payload_calls,
                    )
            continue
        _classify_command(
            cmd,
            rng,
            current_side,
            collected,
            released,
            collect_calls,
            release_calls,
            payload_calls,
        )


def _find_collect_flow_warnings(
    when_bodies: list[tuple[str, int, str, Token, Token]],
    cu: CompilationUnit | None = None,
) -> list[IrulesFlowWarning]:
    """Find collect/release pairing and DATA-event issues.

    Uses the IR from *cu* when available for precise command matching;
    falls back to regex over raw text when the CU is unavailable.

    IRULE1005: A ``when CLIENT_DATA`` (etc.) handler exists but no
    ``*::collect`` for the corresponding protocol appears anywhere in the
    iRule, so the DATA event will never fire.

    IRULE1006: A ``*::payload`` call appears but no ``*::collect`` for
    that protocol exists in the iRule, so the payload buffer is empty.

    IRULE1007: A ``*::collect`` call has no matching ``*::release`` on the
    same connection side — collected data is never released.

    IRULE1008: A ``*::release`` call has no matching ``*::collect`` on the
    same connection side — nothing was collected.
    """
    # ── Pass 1: scan all when bodies for collect/release/payload calls ──
    collected_protocol_sides: dict[str, set[str]] = {}
    released_protocol_sides: dict[str, set[str]] = {}
    collect_calls: list[tuple[str, str, Range]] = []  # (protocol, side, range)
    release_calls: list[tuple[str, str, Range]] = []
    payload_calls: list[tuple[str, str, Range]] = []

    qnames = _when_qualified_names(when_bodies)
    for (event, _priority, body_text, body_tok, _event_tok), qname in zip(
        when_bodies, qnames, strict=True
    ):
        ir_proc = cu.ir_module.procedures.get(qname) if cu else None
        ir_body: IRScript | None = ir_proc.body if ir_proc is not None else None
        if ir_body is None:
            # CU unavailable — lower the body text to IR ourselves.
            try:
                ir_body = lower_to_ir(body_text).top_level
            except Exception:
                log.debug("collect_flow: IR lowering failed for %s, skipping", event)
                continue
        _scan_ir_event(
            event,
            ir_body,
            collected_protocol_sides,
            released_protocol_sides,
            collect_calls,
            release_calls,
            payload_calls,
        )

    # ── Pass 2: emit warnings ──────────────────────────────────────────
    warnings: list[IrulesFlowWarning] = []

    for event, _priority, _body_text, _body_tok, event_tok in when_bodies:
        # IRULE1005: DATA event without matching collect
        required = DATA_EVENT_REQUIREMENTS.get(event)
        if required is not None:
            protocols, required_side = required
            if not any(
                required_side in collected_protocol_sides.get(protocol, set())
                for protocol in protocols
            ):
                proto_hint = " or ".join(f"{protocol}::collect" for protocol in protocols)
                warnings.append(
                    IrulesFlowWarning(
                        range=range_from_token(event_tok),
                        code="IRULE1005",
                        message=(
                            f"'{event}' will never fire without a "
                            f"{required_side} {proto_hint} call in another event."
                        ),
                    )
                )

    # IRULE1006: payload without matching collect
    for protocol, side, rng in payload_calls:
        if side not in collected_protocol_sides.get(protocol, set()):
            warnings.append(
                IrulesFlowWarning(
                    range=rng,
                    code="IRULE1006",
                    message=(
                        f"'{protocol}::payload' without a "
                        f"{side} {protocol}::collect call. "
                        "The payload buffer will be empty."
                    ),
                )
            )

    # IRULE1007: collect without matching release (side-aware)
    for protocol, side, rng in collect_calls:
        if side not in released_protocol_sides.get(protocol, set()):
            cmd_name = f"{protocol}::collect"
            warnings.append(
                IrulesFlowWarning(
                    range=rng,
                    code="IRULE1007",
                    message=(
                        f"{cmd_name} without matching "
                        f"{protocol}::release on the {side} side; "
                        "collected data is never released"
                    ),
                )
            )

    # IRULE1008: release without matching collect (side-aware)
    for protocol, side, rng in release_calls:
        if side not in collected_protocol_sides.get(protocol, set()):
            cmd_name = f"{protocol}::release"
            warnings.append(
                IrulesFlowWarning(
                    range=rng,
                    code="IRULE1008",
                    message=(
                        f"{cmd_name} without matching "
                        f"{protocol}::collect on the {side} side; "
                        "no data was collected"
                    ),
                )
            )

    return warnings


# IRULE5002: drop/reject/discard without event disable all or return
# IRULE5004: DNS::return without return


def _drops_connection_commands() -> frozenset[str]:
    """Return commands that terminate the connection (lazy-cached)."""
    from .side_effects import SideEffectTarget

    result: set[str] = set()
    for name in REGISTRY.specs_by_name:
        hints = REGISTRY.side_effect_hints(name, dialect="f5-irules")
        if hints and any(
            h.target is SideEffectTarget.CONNECTION_CONTROL and h.writes for h in hints
        ):
            result.add(name)
    return frozenset(result)


_DROP_COMMANDS_CACHE: frozenset[str] | None = None


def _get_drop_commands() -> frozenset[str]:
    global _DROP_COMMANDS_CACHE
    if _DROP_COMMANDS_CACHE is None:
        _DROP_COMMANDS_CACHE = _drops_connection_commands()
    return _DROP_COMMANDS_CACHE


def _is_event_disable_all(stmt) -> bool:
    """Return True if *stmt* is ``event disable all``."""
    if not isinstance(stmt, (IRCall, IRBarrier)):
        return False
    if stmt.command != "event":
        return False
    args = stmt.args
    return len(args) >= 2 and args[0] == "disable" and args[1] == "all"


def _analyse_drop_without_disable(
    event: str,
    body_text: str,
    body_tok: Token,
    *,
    ir_body: IRScript | None = None,
) -> list[IrulesFlowWarning]:
    """IRULE5002 + IRULE5004: path-sensitive IR walk for unguarded drops / DNS::return."""
    if ir_body is None:
        try:
            ir_body = lower_to_ir(body_text).top_level
        except Exception:
            log.debug("irules_flow: IR lowering failed for unguarded-drop analysis", exc_info=True)
            return []

    warnings: list[IrulesFlowWarning] = []
    base_offset = body_tok.start.offset + 1
    base_line = body_tok.start.line
    base_col = body_tok.start.character + 1

    @dataclass(frozen=True, slots=True)
    class _DropState:
        # IRULE5002
        dropped: bool = False
        drop_command: str = ""
        drop_range: Range | None = None
        # IRULE5004
        dns_returned: bool = False
        dns_return_range: Range | None = None

    def _translate(r: Range) -> Range:
        start = position_from_relative(
            body_text,
            r.start.offset,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        end = position_from_relative(
            body_text,
            r.end.offset,
            base_line=base_line,
            base_col=base_col,
            base_offset=base_offset,
        )
        return Range(start=start, end=end)

    def _dedupe(states: list[_DropState]) -> list[_DropState]:
        seen: set[tuple[bool, bool]] = set()
        out: list[_DropState] = []
        for s in states:
            key = (s.dropped, s.dns_returned)
            if key not in seen:
                seen.add(key)
                out.append(s)
        return out

    def _walk(script: IRScript, in_states: list[_DropState]) -> list[_DropState]:
        states = list(in_states)
        for stmt in script.statements:
            next_states: list[_DropState] = []

            if isinstance(stmt, IRReturn):
                # return terminates this path — any pending drop/dns_return
                # is safe because the rule stops here.
                states = []
                break

            if isinstance(stmt, IRIf):
                for st in states:
                    branch_out: list[_DropState] = []
                    for clause in stmt.clauses:
                        branch_out.extend(_walk(clause.body, [st]))
                    if stmt.else_body is not None:
                        branch_out.extend(_walk(stmt.else_body, [st]))
                    else:
                        branch_out.append(st)
                    next_states.extend(branch_out)
                states = _dedupe(next_states)
                continue

            if isinstance(stmt, IRSwitch):
                for st in states:
                    branch_out: list[_DropState] = [st]
                    for arm in stmt.arms:
                        if arm.body is not None:
                            branch_out.extend(_walk(arm.body, [st]))
                    if stmt.default_body is not None:
                        branch_out.extend(_walk(stmt.default_body, [st]))
                    next_states.extend(branch_out)
                states = _dedupe(next_states)
                continue

            if isinstance(stmt, (IRFor, IRWhile, IRForeach)):
                for st in states:
                    iter_states = _walk(stmt.body, [st])
                    next_states.append(st)
                    next_states.extend(iter_states)
                states = _dedupe(next_states)
                continue

            if isinstance(stmt, IRCatch):
                for st in states:
                    next_states.extend(_walk(stmt.body, [st]))
                states = _dedupe(next_states)
                continue

            if isinstance(stmt, IRTry):
                for st in states:
                    branch_out: list[_DropState] = []
                    branch_out.extend(_walk(stmt.body, [st]))
                    for handler in stmt.handlers:
                        branch_out.extend(_walk(handler.body, [st]))
                    if stmt.finally_body is not None:
                        final_out: list[_DropState] = []
                        for mid in branch_out:
                            final_out.extend(_walk(stmt.finally_body, [mid]))
                        branch_out = final_out
                    next_states.extend(branch_out or [st])
                states = _dedupe(next_states)
                continue

            # Leaf statement: check for drop / event disable all / DNS::return
            if _is_event_disable_all(stmt):
                for st in states:
                    next_states.append(
                        _DropState(
                            dropped=False,
                            drop_command=st.drop_command,
                            drop_range=st.drop_range,
                            dns_returned=st.dns_returned,
                            dns_return_range=st.dns_return_range,
                        )
                    )
                states = _dedupe(next_states)
                continue

            if isinstance(stmt, (IRCall, IRBarrier)):
                cmd = stmt.command
                cmd_range = _translate(stmt.range)
                for st in states:
                    if cmd in _get_drop_commands():
                        next_states.append(
                            _DropState(
                                dropped=True,
                                drop_command=cmd,
                                drop_range=cmd_range,
                                dns_returned=st.dns_returned,
                                dns_return_range=st.dns_return_range,
                            )
                        )
                    elif cmd == "DNS::return":
                        next_states.append(
                            _DropState(
                                dropped=st.dropped,
                                drop_command=st.drop_command,
                                drop_range=st.drop_range,
                                dns_returned=True,
                                dns_return_range=cmd_range,
                            )
                        )
                    else:
                        next_states.append(st)
                states = _dedupe(next_states)
                continue

            # Non-command statements pass through.
            for st in states:
                next_states.append(st)
            states = _dedupe(next_states)

        return states

    final_states = _walk(ir_body, [_DropState()])

    for st in final_states:
        if st.dropped and st.drop_range is not None:
            # Insertion range: end = start - 1 so LSP end.char+1 == start.char
            ins_start = SourcePosition(
                line=st.drop_range.end.line,
                character=st.drop_range.end.character + 1,
                offset=st.drop_range.end.offset + 1,
            )
            ins_end = st.drop_range.end
            warnings.append(
                IrulesFlowWarning(
                    range=st.drop_range,
                    code="IRULE5002",
                    message=(
                        f"'{st.drop_command}' without 'event disable all' or "
                        "'return' \u2014 other iRules and later priorities in "
                        "this event will still execute, which may cause "
                        "TCL errors."
                    ),
                    fixes=(
                        CodeFix(
                            range=Range(start=ins_start, end=ins_end),
                            new_text="\n    event disable all\n    return",
                            description="Add 'event disable all' + 'return'",
                        ),
                    ),
                )
            )
        if st.dns_returned and st.dns_return_range is not None:
            ins_start = SourcePosition(
                line=st.dns_return_range.end.line,
                character=st.dns_return_range.end.character + 1,
                offset=st.dns_return_range.end.offset + 1,
            )
            ins_end = st.dns_return_range.end
            warnings.append(
                IrulesFlowWarning(
                    range=st.dns_return_range,
                    code="IRULE5004",
                    message=(
                        "'DNS::return' must be followed by 'return' to stop iRule processing."
                    ),
                    fixes=(
                        CodeFix(
                            range=Range(start=ins_start, end=ins_end),
                            new_text="\n    return",
                            description="Add 'return' after DNS::return",
                        ),
                    ),
                )
            )

    return warnings


# IRULE4004: Hoist constant set from per-request to once-per-connection

_NS_CMD_RE = re.compile(r"\b(\w+::\w+)\b")


def _cmd_available_at_event(cmd_name: str, target_event: str) -> bool:
    """Return True if *cmd_name* can run in *target_event*."""
    target_props = EVENT_REGISTRY.get_props(target_event)
    if target_props is None:
        return False
    spec = REGISTRY.get(cmd_name)
    if spec is None:
        return False  # unknown command → conservative
    requires = spec.event_requires
    if requires is None:
        return True  # no profile requirement → works anywhere
    return EVENT_REGISTRY.event_satisfies(target_props, requires, event_name=target_event)


def _ir_value_hoistable_to(
    stmt: IRAssignConst | IRAssignValue | IRIncr | IRCall,
    target_event: str,
) -> bool:
    """Return True if the IR statement's value can run in *target_event*.

    ``IRAssignConst`` is always hoistable (pure literal).
    ``IRIncr`` with a literal amount is always hoistable.
    ``IRAssignValue`` is hoistable when its value contains no variable
    references and all command substitutions are available at the target.
    ``IRCall`` is hoistable when the command itself and all its args are
    available at the target (no variable refs, commands are reachable).
    """
    if isinstance(stmt, IRAssignConst):
        return True

    if isinstance(stmt, IRIncr):
        # incr with a literal amount — always hoistable.
        return True

    if isinstance(stmt, IRCall):
        # Check the command itself is available at the target.
        if not _cmd_available_at_event(stmt.command, target_event):
            return False
        # Check all args for variable refs and command availability.
        for arg in stmt.args:
            if "$" in arg:
                return False
            for m in _NS_CMD_RE.finditer(arg):
                if not _cmd_available_at_event(m.group(1), target_event):
                    return False
        return True

    # IRAssignValue
    value = stmt.value
    # Variable reference → not hoistable.
    if "$" in value:
        return False

    # No command substitutions → pure literal → always hoistable.
    if "[" not in value:
        return True

    # Check all namespace::subcommand references in the value.
    for m in _NS_CMD_RE.finditer(value):
        ns_cmd = m.group(1)
        if not _cmd_available_at_event(ns_cmd, target_event):
            return False

    # Check the outer command if parseable.
    parsed = parse_command_substitution(value)
    if parsed is not None:
        outer_cmd = parsed[0]
        if REGISTRY.get(outer_cmd) is None:
            return False  # unknown command → conservative
        if not _cmd_available_at_event(outer_cmd, target_event):
            return False

    return True


def _find_hoistable_constants(
    when_bodies: list[tuple[str, int, str, Token, Token]],
    cu: CompilationUnit | None = None,
) -> list[IrulesFlowWarning]:
    """IRULE4004: constant set in per-request event hoistable to per-connection.

    Walks the IR for top-level ``set var value`` statements (``IRAssignConst``
    and ``IRAssignValue``) in PER_REQUEST events.  A set is hoistable when
    the value is a pure literal or all its command substitutions are
    available at the target once-per-connection event.
    """
    file_events = frozenset(event for event, *_ in when_bodies)
    warnings: list[IrulesFlowWarning] = []
    qnames = _when_qualified_names(when_bodies)

    for (event, _priority, body_text, body_tok, _event_tok), qname in zip(
        when_bodies, qnames, strict=True
    ):
        if not EVENT_REGISTRY.is_per_request(event):
            continue

        # Build a list of candidate target events (earliest-first) where
        # per-request set statements could be hoisted.
        compatible = EVENT_REGISTRY.compatible_connection_predecessors(event)
        candidates: list[tuple[str, bool]] = []  # (event_name, already_in_file)
        for pred in EVENT_REGISTRY.events_before(event, file_events):
            if EVENT_REGISTRY.is_once_per_connection(pred) and pred != "RULE_INIT":
                if not compatible or pred in compatible:
                    candidates.append((pred, True))
        if not candidates and compatible:
            for pred in EVENT_REGISTRY.events_before(event, compatible):
                if EVENT_REGISTRY.is_once_per_connection(pred) and pred != "RULE_INIT":
                    candidates.append((pred, False))
        if not candidates:
            continue

        # Get IR body for this event.
        ir_proc = cu.ir_module.procedures.get(qname) if cu else None
        ir_body: IRScript | None = ir_proc.body if ir_proc is not None else None
        if ir_body is None:
            try:
                ir_body = lower_to_ir(body_text).top_level
            except Exception:
                log.debug("hoistable_constants: IR lowering failed for %s, skipping", event)
                continue

        # Walk only top-level statements (not inside if/for/etc).
        for stmt in ir_body.statements:
            # Determine variable name(s) and display text for each store type.
            var_name: str | None = None
            display: str | None = None
            if isinstance(stmt, (IRAssignConst, IRAssignValue)):
                var_name = stmt.name
                display = f"set {stmt.name} {stmt.value}"
            elif isinstance(stmt, IRIncr):
                var_name = stmt.name
                amt = f" {stmt.amount}" if stmt.amount else ""
                display = f"incr {stmt.name}{amt}"
            elif isinstance(stmt, IRCall) and stmt.defs:
                # Commands like binary scan, regexp, scan that define variables.
                # Only flag if the command itself + all args are hoistable.
                var_name = stmt.defs[0]  # first defined var for display
                display = f"{stmt.command} ..."

            if var_name is None or display is None:
                continue
            # Skip static:: variables — they're already scoped.
            if var_name.startswith("static::"):
                continue

            # Try each candidate event (earliest first).
            hoistable_stmt: IRAssignConst | IRAssignValue | IRIncr | IRCall = stmt  # type: ignore[assignment]
            best_event = None
            best_exists = False
            for cand, cand_exists in candidates:
                if _ir_value_hoistable_to(hoistable_stmt, cand):
                    best_event = cand
                    best_exists = cand_exists
                    break
            if best_event is not None:
                if len(display) > 50:
                    display = display[:47] + "..."
                suffix = (
                    "(once per connection)."
                    if best_exists
                    else "(once per connection; event not yet in this iRule)."
                )
                warnings.append(
                    IrulesFlowWarning(
                        range=stmt.range,
                        code="IRULE4004",
                        message=(
                            f"'{display}' in "
                            f"{event} (per-request) could be hoisted to "
                            f"{best_event} {suffix}"
                        ),
                    )
                )

    return warnings


def _find_generic_static_names(
    when_bodies: list[tuple[str, int, str, Token, Token]],
    cu: CompilationUnit | None = None,
    generic_variable_patterns: list[str] | None = None,
) -> list[IrulesFlowWarning]:
    """IRULE4002: generic ``static::`` variable name that will collide.

    Walks the CFG (or IR if CFG unavailable) for every statement that
    defines a ``static::`` variable and checks the bare name against
    configurable regex patterns.

    Using the CFG rather than the raw IR ensures that upvar-augmented
    ``defs`` are visible — e.g. a proc that does
    ``upvar 1 static::debug local`` will have ``static::debug`` added to
    the call's ``defs`` by the CFG builder.
    """
    from ..analysis.irules_checks import _is_generic_static_name

    warnings: list[IrulesFlowWarning] = []
    seen: set[str] = set()  # deduplicate across events

    def _check_var(var_name: str, rng: Range) -> None:
        if not var_name.startswith("static::"):
            return
        if var_name in seen:
            return
        if not _is_generic_static_name(var_name, generic_variable_patterns):
            return
        seen.add(var_name)
        bare = var_name.removeprefix("static::")
        warnings.append(
            IrulesFlowWarning(
                range=rng,
                code="IRULE4002",
                message=(
                    f"'{var_name}' is a generic name that will collide "
                    f"with other iRules. static:: variables are shared "
                    f"across every iRule on the BIG-IP system — prefix "
                    f"with the application or rule name "
                    f"(e.g. 'static::<app>_{bare}')."
                ),
            )
        )

    qnames = _when_qualified_names(when_bodies)
    for (event, _priority, body_text, _body_tok, _event_tok), qname in zip(
        when_bodies, qnames, strict=True
    ):
        # Prefer the CFG (has upvar-augmented defs) over raw IR.
        fu = cu.procedures.get(qname) if cu else None
        if fu is not None:
            # Walk CFG blocks — statements here have upvar-resolved defs.
            for block in fu.cfg.blocks.values():
                for stmt in block.statements:
                    if isinstance(stmt, (IRAssignConst, IRAssignValue, IRIncr)):
                        _check_var(stmt.name, stmt.range)
                    elif isinstance(stmt, IRCall) and stmt.defs:
                        for var_name in stmt.defs:
                            _check_var(var_name, stmt.range)
                    # Descend into clientside/serverside body args.
                    if isinstance(stmt, IRCall) and stmt.command in ("clientside", "serverside"):
                        inner_ir = _lower_side_switch_body(stmt)
                        if inner_ir is not None:
                            for inner_stmt in _iter_all_ir_statements(inner_ir):
                                if isinstance(inner_stmt, (IRAssignConst, IRAssignValue, IRIncr)):
                                    _check_var(inner_stmt.name, inner_stmt.range)
                                elif isinstance(inner_stmt, IRCall) and inner_stmt.defs:
                                    for var_name in inner_stmt.defs:
                                        _check_var(var_name, inner_stmt.range)
        else:
            # Fallback to raw IR.
            ir_proc = cu.ir_module.procedures.get(qname) if cu else None
            ir_body = ir_proc.body if ir_proc is not None else None
            if ir_body is None:
                try:
                    ir_body = lower_to_ir(body_text).top_level
                except Exception:
                    continue
            for stmt in _iter_all_ir_statements(ir_body):
                if isinstance(stmt, (IRAssignConst, IRAssignValue, IRIncr)):
                    _check_var(stmt.name, stmt.range)
                elif isinstance(stmt, IRCall) and stmt.defs:
                    for var_name in stmt.defs:
                        _check_var(var_name, stmt.range)

    return warnings


def _iter_all_ir_statements(script: IRScript, *, lower_side_switches: bool = True):
    """Yield every IR statement, recursing into structured bodies.

    When *lower_side_switches* is ``True`` (default), ``clientside``/
    ``serverside`` body arguments are lowered to IR and recursed into.
    """
    for stmt in script.statements:
        yield stmt
        if isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                yield from _iter_all_ir_statements(
                    clause.body, lower_side_switches=lower_side_switches
                )
            if stmt.else_body is not None:
                yield from _iter_all_ir_statements(
                    stmt.else_body, lower_side_switches=lower_side_switches
                )
        elif isinstance(stmt, IRFor):
            yield from _iter_all_ir_statements(stmt.init, lower_side_switches=lower_side_switches)
            yield from _iter_all_ir_statements(stmt.body, lower_side_switches=lower_side_switches)
            yield from _iter_all_ir_statements(stmt.next, lower_side_switches=lower_side_switches)
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body is not None:
                    yield from _iter_all_ir_statements(
                        arm.body, lower_side_switches=lower_side_switches
                    )
            if stmt.default_body is not None:
                yield from _iter_all_ir_statements(
                    stmt.default_body, lower_side_switches=lower_side_switches
                )
        elif isinstance(stmt, IRWhile):
            yield from _iter_all_ir_statements(stmt.body, lower_side_switches=lower_side_switches)
        elif isinstance(stmt, IRForeach):
            yield from _iter_all_ir_statements(stmt.body, lower_side_switches=lower_side_switches)
        elif isinstance(stmt, IRCatch):
            yield from _iter_all_ir_statements(stmt.body, lower_side_switches=lower_side_switches)
        elif isinstance(stmt, IRTry):
            yield from _iter_all_ir_statements(stmt.body, lower_side_switches=lower_side_switches)
            for handler in stmt.handlers:
                yield from _iter_all_ir_statements(
                    handler.body, lower_side_switches=lower_side_switches
                )
            if stmt.finally_body is not None:
                yield from _iter_all_ir_statements(
                    stmt.finally_body, lower_side_switches=lower_side_switches
                )
        elif (
            lower_side_switches
            and isinstance(stmt, IRCall)
            and stmt.command in ("clientside", "serverside")
        ):
            inner_ir = _lower_side_switch_body(stmt)
            if inner_ir is not None:
                yield from _iter_all_ir_statements(
                    inner_ir, lower_side_switches=lower_side_switches
                )


def find_irules_flow_warnings(
    source: str,
    *,
    cu: CompilationUnit | None = None,
    generic_variable_patterns: list[str] | None = None,
) -> list[IrulesFlowWarning]:
    """Find all iRules flow and performance warnings in source."""
    if active_dialect() != "f5-irules":
        return []

    cu = ensure_compilation_unit(
        source,
        cu,
        logger=log,
        context="irules_flow",
        failure_detail="compilation failed; using token-only iRules flow analysis",
    )

    warnings: list[IrulesFlowWarning] = []
    when_bodies = list(_find_when_bodies(source))
    qnames = _when_qualified_names(when_bodies)
    for (event, _priority, body_text, body_tok, _event_tok), qname in zip(
        when_bodies, qnames, strict=True
    ):
        warnings.extend(_analyse_when_body(event, body_text, body_tok))
        when_ir_body: IRScript | None = None
        if cu is not None:
            ir_proc = cu.ir_module.procedures.get(qname)
            if ir_proc is not None:
                when_ir_body = ir_proc.body
        warnings.extend(
            _analyse_drop_without_disable(
                event,
                body_text,
                body_tok,
                ir_body=when_ir_body,
            )
        )
    warnings.extend(_find_collect_flow_warnings(when_bodies, cu=cu))
    warnings.extend(_find_hoistable_constants(when_bodies, cu=cu))
    warnings.extend(
        _find_generic_static_names(
            when_bodies,
            cu=cu,
            generic_variable_patterns=generic_variable_patterns,
        )
    )
    return warnings


# Event ordering for explorer


@dataclass(frozen=True, slots=True)
class EventOrderEntry:
    """One ``when EVENT`` block with its position in the firing order."""

    event: str
    base_priority: int
    priority_offset: int  # 0 for first handler at this priority, +1 per tie
    multiplicity: str  # "once", "per-request", or "init"
    range: Range  # source range of the event token


def extract_event_order(source: str) -> list[EventOrderEntry]:
    """Return events found in *source* in canonical firing order.

    Each entry carries the event name, its declared base priority, the
    priority offset (tie-breaker among handlers sharing the same event and
    base priority, derived from file order), the multiplicity class, and
    the source range of the ``when EVENT`` token so the explorer can
    navigate to it.

    When the same event appears more than once, each handler gets its
    own entry, ordered by base priority (lowest first, then file order for
    equal priorities).
    """
    when_bodies = list(_find_when_bodies(source))
    if not when_bodies:
        return []

    # Collect all handlers per event, preserving file order.
    per_event: dict[str, list[tuple[int, int, Range]]] = defaultdict(list)
    for idx, (event, priority, _body, _body_tok, event_tok) in enumerate(when_bodies):
        per_event[event].append((priority, idx, range_from_token(event_tok)))

    # Sort each event's handlers: lowest priority first, file order breaks ties.
    for handlers in per_event.values():
        handlers.sort(key=lambda t: (t[0], t[1]))

    ordered = EVENT_REGISTRY.order_events(frozenset(per_event))
    result: list[EventOrderEntry] = []
    for evt in ordered:
        mult = EVENT_REGISTRY.event_multiplicity(evt)
        prev_pri: int | None = None
        offset = 0
        for base_pri, _idx, rng in per_event[evt]:
            if base_pri == prev_pri:
                offset += 1
            else:
                offset = 0
                prev_pri = base_pri
            result.append(
                EventOrderEntry(
                    event=evt,
                    base_priority=base_pri,
                    priority_offset=offset,
                    multiplicity=mult,
                    range=rng,
                )
            )
    return result


# RULE_INIT cross-file variable extraction


@dataclass(frozen=True, slots=True)
class RuleInitExport:
    """A variable or array set in a RULE_INIT block."""

    name: str  # e.g. "::my_var"
    base_priority: int
    range: Range
    is_array: bool = False


def _is_cross_rule_var(name: str) -> bool:
    """Return True if *name* is a cross-rule variable (``::var`` or ``static::var``)."""
    return name.startswith("::") or name.startswith("static::")


def extract_rule_init_vars(
    source: str,
    *,
    cu: CompilationUnit | None = None,
) -> list[RuleInitExport]:
    """Extract global and static variables set in ``when RULE_INIT`` blocks.

    When *cu* is available, walks the IR (including upvar-augmented defs from
    the CFG) instead of scanning tokens.  Falls back to the token walk when
    the compilation unit is not available.
    """
    exports: list[RuleInitExport] = []
    when_bodies = list(_find_when_bodies(source))
    qnames = _when_qualified_names(when_bodies)
    for (event, priority, body_text, body_tok, _event_tok), qname in zip(
        when_bodies, qnames, strict=True
    ):
        if event != "RULE_INIT":
            continue

        # IR-based path (preferred)
        fu = cu.procedures.get(qname) if cu else None
        if fu is not None:
            seen: set[str] = set()
            for block in fu.cfg.blocks.values():
                for stmt in block.statements:
                    names: list[str] = []
                    is_array = False
                    if isinstance(stmt, (IRAssignConst, IRAssignValue)):
                        names.append(stmt.name)
                    elif isinstance(stmt, IRIncr):
                        names.append(stmt.name)
                    elif isinstance(stmt, IRCall):
                        if stmt.command == "array" and stmt.args and stmt.args[0] == "set":
                            # array set <name> <list> — name is in args[1]
                            if len(stmt.args) >= 2:
                                names.append(stmt.args[1])
                                is_array = True
                        if stmt.defs:
                            names.extend(stmt.defs)
                    for name in names:
                        if name in seen:
                            continue
                        if _is_cross_rule_var(name):
                            seen.add(name)
                            exports.append(
                                RuleInitExport(
                                    name=name,
                                    base_priority=priority,
                                    range=stmt.range,
                                    is_array=is_array,
                                )
                            )
            continue

        # Fallback: token walk
        for cmd_name, _cmd_tok, all_tokens in _walk_body_commands(
            body_text,
            base_offset=body_tok.start.offset + 1,
            base_line=body_tok.start.line,
            base_col=body_tok.start.character + 1,
        ):
            if cmd_name == "set" and len(all_tokens) >= 2:
                var_tok = all_tokens[1]
                var_name = var_tok.text
                if _is_cross_rule_var(var_name):
                    exports.append(
                        RuleInitExport(
                            name=var_name,
                            base_priority=priority,
                            range=range_from_token(var_tok),
                        )
                    )
            elif cmd_name == "array" and len(all_tokens) >= 3:
                if all_tokens[1].text == "set":
                    arr_tok = all_tokens[2]
                    arr_name = arr_tok.text
                    if _is_cross_rule_var(arr_name):
                        exports.append(
                            RuleInitExport(
                                name=arr_name,
                                base_priority=priority,
                                range=range_from_token(arr_tok),
                                is_array=True,
                            )
                        )
    return exports
