"""iRules control-flow checks.

Walks the token stream of ``when`` bodies to detect:

- **IRULE1005** (WARNING): ``*_DATA`` event handler without a matching
  ``*::collect`` call elsewhere in the iRule — the event will never fire.
- **IRULE1006** (WARNING): ``*::payload`` access without a matching
  ``*::collect`` call — the payload buffer will be empty.
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
from dataclasses import dataclass

from ..analysis.semantic_model import CodeFix, Range
from ..commands.registry import REGISTRY
from ..commands.registry.namespace_registry import NAMESPACE_REGISTRY as EVENT_REGISTRY
from ..common.dialect import active_dialect
from ..common.ranges import position_from_relative, range_from_token
from ..parsing.lexer import TclLexer
from ..parsing.tokens import SourcePosition, Token, TokenType
from .compilation_unit import CompilationUnit, ensure_compilation_unit
from .ir import (
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRCatch,
    IRFor,
    IRForeach,
    IRIf,
    IRReturn,
    IRScript,
    IRSwitch,
    IRTry,
    IRWhile,
)
from .lowering import lower_to_ir

log = logging.getLogger(__name__)

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

    Yields ``(event_name, priority, body_text, body_token, event_token)``.

    Uses the Tcl lexer to reliably parse the source and find ``when`` commands.
    Priority is extracted from ``when EVENT priority N { body }``; defaults to 500.
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


def _analyse_repeated_calls(
    event: str,
    body_text: str,
    body_tok: Token,
) -> list[IrulesFlowWarning]:
    """Walk a ``when`` body and flag repeated expensive command invocations.

    IRULE2102: when the same expensive command (e.g. ``HTTP::uri``) is called
    more than once with identical arguments, suggest extracting to a variable.
    """
    # Track (cmd_name, args_key) → first occurrence token
    seen: dict[str, Token] = {}
    warnings: list[IrulesFlowWarning] = []

    for cmd_name, cmd_tok, all_tokens in _walk_body_commands(
        body_text,
        base_offset=body_tok.start.offset + 1,
        base_line=body_tok.start.line,
        base_col=body_tok.start.character + 1,
    ):
        if not REGISTRY.is_cse_candidate(cmd_name):
            continue
        # Build a key from command name + args text
        args_text = " ".join(t.text for t in all_tokens[1:])
        call_key = f"{cmd_name} {args_text}"
        if call_key in seen:
            warnings.append(
                IrulesFlowWarning(
                    range=range_from_token(cmd_tok),
                    code="IRULE2102",
                    message=(
                        f"'{cmd_name}' called multiple times with the same arguments. "
                        "Consider storing the result in a local variable."
                    ),
                    respond_range=range_from_token(seen[call_key]),
                )
            )
        else:
            seen[call_key] = cmd_tok

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

_COLLECT_RE = re.compile(r"\b(\w+)::collect\b")
_PAYLOAD_RE = re.compile(r"\b(\w+)::payload\b")
_SIDE_SWITCH_RE = re.compile(r"\b(clientside|serverside)\s*\{", re.IGNORECASE)


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


def _find_matching_brace(text: str, open_index: int) -> int | None:
    """Return index of the brace matching text[open_index], or None."""
    depth = 0
    idx = open_index
    while idx < len(text):
        ch = text[idx]
        escaped = idx > 0 and text[idx - 1] == "\\"
        if ch == "{" and not escaped:
            depth += 1
        elif ch == "}" and not escaped:
            depth -= 1
            if depth == 0:
                return idx
        idx += 1
    return None


def _collect_side_spans(body_text: str) -> list[tuple[int, int, str]]:
    """Return [(start, end, side)] spans for clientside/serverside blocks."""
    spans: list[tuple[int, int, str]] = []
    for match in _SIDE_SWITCH_RE.finditer(body_text):
        side = "client" if match.group(1).lower() == "clientside" else "server"
        open_brace = match.end() - 1
        close_brace = _find_matching_brace(body_text, open_brace)
        if close_brace is None:
            continue
        spans.append((open_brace + 1, close_brace, side))
    return spans


def _side_for_offset(offset: int, default_side: str, spans: list[tuple[int, int, str]]) -> str:
    """Return effective side at *offset* using explicit side-switch spans."""
    chosen: tuple[int, int, str] | None = None
    for span in spans:
        start, end, _side = span
        if start <= offset < end:
            if chosen is None or (end - start) < (chosen[1] - chosen[0]):
                chosen = span
    if chosen is not None:
        return chosen[2]
    return default_side


def _find_collect_flow_warnings(
    when_bodies: list[tuple[str, int, str, Token, Token]],
) -> list[IrulesFlowWarning]:
    """Find *_DATA events and *::payload access without matching *::collect.

    IRULE1005: A ``when CLIENT_DATA`` (etc.) handler exists but no
    ``*::collect`` for the corresponding protocol appears anywhere in the
    iRule, so the DATA event will never fire.

    IRULE1006: A ``*::payload`` call appears but no ``*::collect`` for
    that protocol exists in the iRule, so the payload buffer is empty.
    """
    # Scan all when bodies for ::collect calls and track which side context
    # they execute under (client/server).  A collect on the wrong side does
    # not satisfy CLIENT_DATA/SERVER_DATA requirements.
    collected_protocol_sides: dict[str, set[str]] = {}
    for event, _priority, body_text, _body_tok, _event_tok in when_bodies:
        default_side = _default_collect_side(event)
        side_spans = _collect_side_spans(body_text)
        for m in _COLLECT_RE.finditer(body_text):
            protocol = m.group(1).upper()
            side = _side_for_offset(m.start(), default_side, side_spans)
            collected_protocol_sides.setdefault(protocol, set()).add(side)

    warnings: list[IrulesFlowWarning] = []

    for event, _priority, body_text, body_tok, event_tok in when_bodies:
        default_side = _default_collect_side(event)
        side_spans = _collect_side_spans(body_text)

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

        # IRULE1006: ::payload without matching collect
        base_offset = body_tok.start.offset + 1  # +1 for opening brace
        base_line = body_tok.start.line
        base_col = body_tok.start.character + 1
        for m in _PAYLOAD_RE.finditer(body_text):
            protocol = m.group(1).upper()
            payload_side = _side_for_offset(m.start(), default_side, side_spans)
            if payload_side not in collected_protocol_sides.get(protocol, set()):
                start = position_from_relative(
                    body_text,
                    m.start(),
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_offset,
                )
                end = position_from_relative(
                    body_text,
                    m.end(),
                    base_line=base_line,
                    base_col=base_col,
                    base_offset=base_offset,
                )
                warnings.append(
                    IrulesFlowWarning(
                        range=Range(start=start, end=end),
                        code="IRULE1006",
                        message=(
                            f"'{protocol}::payload' without a "
                            f"{payload_side} {protocol}::collect call. "
                            "The payload buffer will be empty."
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


def _value_hoistable_to(
    word_types: list[TokenType],
    word_texts: list[str],
    target_event: str,
) -> bool:
    """Return True if the value word tokens can run in *target_event*.

    Checks that all runtime dependencies (command substitutions) are
    available at the target event.  Variable references always fail.
    """
    # Any direct variable reference → not hoistable.
    if any(t is TokenType.VAR for t in word_types):
        return False

    # No command substitutions → pure literal → always hoistable.
    cmd_indices = [i for i, t in enumerate(word_types) if t is TokenType.CMD]
    if not cmd_indices:
        return True

    # Check each command substitution token.
    for idx in cmd_indices:
        cmd_text = word_texts[idx]

        # Variable reference inside command → can't determine availability.
        if "$" in cmd_text:
            return False

        # Check outer command (first word of the substitution).
        outer_cmd = cmd_text.split(None, 1)[0] if cmd_text.strip() else ""
        if outer_cmd and REGISTRY.get(outer_cmd) is None:
            return False  # unknown command → conservative

        if outer_cmd and not _cmd_available_at_event(outer_cmd, target_event):
            return False

        # Check all namespace::subcommand references in full text
        # (catches nested [HTTP::uri] inside [string tolower [HTTP::uri]]).
        for m in _NS_CMD_RE.finditer(cmd_text):
            ns_cmd = m.group(1)
            if not _cmd_available_at_event(ns_cmd, target_event):
                return False

    return True


def _find_hoistable_constants(
    when_bodies: list[tuple[str, int, str, Token, Token]],
) -> list[IrulesFlowWarning]:
    """IRULE4004: constant set in per-request event hoistable to per-connection.

    Scans top-level ``set var value`` commands in PER_REQUEST events.
    A set is hoistable when the value is a pure literal or all its command
    substitutions are available at the target once-per-connection event.
    """
    file_events = frozenset(event for event, *_ in when_bodies)
    warnings: list[IrulesFlowWarning] = []

    for event, _priority, body_text, body_tok, _event_tok in when_bodies:
        if not EVENT_REGISTRY.is_per_request(event):
            continue

        # Build a list of candidate target events (earliest-first) where
        # per-request set statements could be hoisted.  We want the highest
        # (earliest) event in the chain where the value's commands are still
        # available, so candidates are ordered from earliest to latest.
        # Use flow-chain compatibility to avoid suggesting events from
        # incompatible profile stacks (e.g. ACCESS events for DNS contexts).
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
        # Candidates available; scan body below to refine per-statement.

        # Scan top-level commands in the body.
        base_offset = body_tok.start.offset + 1
        base_line = body_tok.start.line
        base_col = body_tok.start.character + 1

        lexer = TclLexer(body_text, base_offset=base_offset, base_line=base_line, base_col=base_col)
        argv: list[Token] = []
        argv_texts: list[str] = []
        word_types: list[list[TokenType]] = []
        word_texts: list[list[str]] = []
        prev_type = TokenType.EOL

        def _flush():
            if (
                len(argv) == 3
                and argv_texts[0] == "set"
                and not argv_texts[1].startswith("static::")
            ):
                val_types = word_types[2]
                val_texts = word_texts[2]
                # Try each candidate event (earliest first) and pick
                # the highest one where the value's commands are available.
                best_event = None
                best_exists = False
                for cand, cand_exists in candidates:
                    if _value_hoistable_to(val_types, val_texts, cand):
                        best_event = cand
                        best_exists = cand_exists
                        break
                if best_event is not None:
                    val_display = argv_texts[2]
                    if len(val_display) > 40:
                        val_display = val_display[:37] + "..."
                    suffix = (
                        "(once per connection)."
                        if best_exists
                        else "(once per connection; event not yet in this iRule)."
                    )
                    warnings.append(
                        IrulesFlowWarning(
                            range=range_from_token(argv[0]),
                            code="IRULE4004",
                            message=(
                                f"'set {argv_texts[1]} {val_display}' in "
                                f"{event} (per-request) could be hoisted to "
                                f"{best_event} {suffix}"
                            ),
                        )
                    )
            argv.clear()
            argv_texts.clear()
            word_types.clear()
            word_texts.clear()

        while True:
            tok = lexer.get_token()
            if tok is None:
                break
            match tok.type:
                case TokenType.COMMENT | TokenType.SEP:
                    prev_type = tok.type
                    continue
                case TokenType.EOL:
                    _flush()
                    prev_type = tok.type
                    continue
                case _:
                    text = tok.text

            if prev_type in (TokenType.SEP, TokenType.EOL):
                # Start of a new word.
                argv.append(tok)
                argv_texts.append(text)
                word_types.append([tok.type])
                word_texts.append([text])
            else:
                if argv_texts:
                    argv_texts[-1] += text
                    word_types[-1].append(tok.type)
                    word_texts[-1].append(text)
                else:
                    argv.append(tok)
                    argv_texts.append(text)
                    word_types.append([tok.type])
                    word_texts.append([text])
            prev_type = tok.type

        _flush()

    return warnings


def find_irules_flow_warnings(
    source: str,
    *,
    cu: CompilationUnit | None = None,
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
    for event, _priority, body_text, body_tok, _event_tok in when_bodies:
        warnings.extend(_analyse_when_body(event, body_text, body_tok))
        when_ir_body: IRScript | None = None
        if cu is not None:
            ir_proc = cu.ir_module.procedures.get(f"::when::{event}")
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
        # Note: IRULE2102 (repeated expensive calls) has been subsumed by
        # the GVN/CSE pass (O105) which handles both standalone and embedded
        # command invocations with richer analysis.
    warnings.extend(_find_collect_flow_warnings(when_bodies))
    warnings.extend(_find_hoistable_constants(when_bodies))
    return warnings


# Event ordering for explorer


@dataclass(frozen=True, slots=True)
class EventOrderEntry:
    """One ``when EVENT`` block with its position in the firing order."""

    event: str
    priority: int
    multiplicity: str  # "once", "per-request", or "init"
    range: Range  # source range of the event token


def extract_event_order(source: str) -> list[EventOrderEntry]:
    """Return events found in *source* in canonical firing order.

    Each entry carries the event name, its declared priority, its
    multiplicity class, and the source range of the ``when EVENT`` token
    so the explorer can navigate to it.
    """
    when_bodies = list(_find_when_bodies(source))
    if not when_bodies:
        return []

    # Collect info per event (first occurrence wins for range/priority)
    seen: dict[str, tuple[int, Range]] = {}
    for event, priority, _body, _body_tok, event_tok in when_bodies:
        if event not in seen:
            seen[event] = (priority, range_from_token(event_tok))

    ordered = EVENT_REGISTRY.order_events(frozenset(seen))
    result: list[EventOrderEntry] = []
    for evt in ordered:
        priority, rng = seen[evt]
        mult = EVENT_REGISTRY.event_multiplicity(evt)
        result.append(
            EventOrderEntry(
                event=evt,
                priority=priority,
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
    priority: int
    range: Range
    is_array: bool = False


def _is_cross_rule_var(name: str) -> bool:
    """Return True if *name* is a cross-rule variable (``::var`` or ``static::var``)."""
    return name.startswith("::") or name.startswith("static::")


def extract_rule_init_vars(source: str) -> list[RuleInitExport]:
    """Extract global and static variables set in ``when RULE_INIT`` blocks."""
    exports: list[RuleInitExport] = []
    for event, priority, body_text, body_tok, _event_tok in _find_when_bodies(source):
        if event != "RULE_INIT":
            continue
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
                            priority=priority,
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
                                priority=priority,
                                range=range_from_token(arr_tok),
                                is_array=True,
                            )
                        )
    return exports
