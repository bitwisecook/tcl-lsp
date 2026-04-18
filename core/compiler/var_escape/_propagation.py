"""Transfer functions for the var-escape analysis.

Walks the per-proc IR tree, classifying each variable as ``LOCAL`` or
``FRAME`` and setting ``dynamic_barrier`` when the analysis can no longer
bound the set of variables that might be observed by name.
"""

from __future__ import annotations

import re
from typing import Iterable

from ...analysis.var_scoping import (
    global_declaration_indices,
    upvar_local_declaration_indices,
    variable_declaration_indices,
)
from ..expr_ast import ExprNode
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRBlock,
    IRCall,
    IRCatch,
    IRExprEval,
    IRFor,
    IRForeach,
    IRIf,
    IRIncr,
    IRReturn,
    IRScript,
    IRStatement,
    IRSwitch,
    IRTry,
    IRWhile,
)
from ..var_refs import VarReferenceScanner
from ._info_subcommands import (
    is_frame_inspecting_info_subcommand,
    is_safe_info_subcommand,
)
from ._types import EscapeTag, ProcEscapeSummary

# Commands whose first arg is the variable name (read/write/modify).
# Used to detect dynamic-name forms like ``set $n value`` — these must
# spill either the inferred target (cheap inference) or every var (fallback).
_NAME_FIRST_COMMANDS: frozenset[str] = frozenset({"set", "incr", "append", "lappend", "unset"})


def _is_literal_name(arg: str) -> bool:
    """True if ``arg`` is a plain identifier, not a substituted ref.

    Mirrors the "starts with ``$`` or ``[``" filter used throughout the
    memory-SSA alias detectors.
    """
    if not arg:
        return False
    if arg.startswith("$") or arg.startswith("["):
        return False
    # Defensive: brace-quoted names are literal but carry the braces in
    # ``args`` for some code paths. Strip them if present.
    return True


def _is_dynamic_token(arg: str) -> bool:
    """True if ``arg`` contains a runtime substitution."""
    return arg.startswith("$") or arg.startswith("[") or "$" in arg or "[" in arg


def _is_dynamic_name(name: str) -> bool:
    """True if ``name`` is an empty or interpolated variable name.

    Lowering preserves dynamic names as e.g. ``${user_var}`` literal text
    in the ``name`` field of ``IRAssignValue``/``IRIncr``. Treat both the
    empty-name form and any substituted form as dynamic.
    """
    if not name:
        return True
    return _is_dynamic_token(name)


# Match ``[info <subcmd> ...]`` command substitutions embedded in value
# strings. Used to detect frame-inspecting ``info`` calls that the lowering
# stuffed inside an ``IRAssignValue`` value rather than surfacing as a
# top-level ``IRCall``.
_INFO_SUBST_RE = re.compile(r"\[\s*info\s+(\w+)(?:\s+([^\]]*))?\]")


def _scan_value_for_info_hazards(value: str) -> tuple[bool, list[str]]:
    """Scan a value string for embedded ``[info ...]`` commands.

    Returns ``(pessimistic, escape_names)``. ``pessimistic`` is True when
    any match triggers the frame-inspecting rule (``info level``/``frame``/
    ``vars``/``locals`` or ``info exists $dynamic``). ``escape_names``
    collects the names referenced by ``info exists <literal>``.
    """
    if "[" not in value or "info" not in value:
        return False, []
    pessimistic = False
    escape_names: list[str] = []
    for match in _INFO_SUBST_RE.finditer(value):
        sub = match.group(1)
        rest = (match.group(2) or "").strip()
        if is_frame_inspecting_info_subcommand(sub):
            pessimistic = True
            continue
        if sub == "exists":
            if not rest:
                continue
            # Only the first whitespace-separated token is the var name.
            target = rest.split(None, 1)[0]
            if _is_dynamic_token(target):
                pessimistic = True
            else:
                escape_names.append(target)
            continue
        if is_safe_info_subcommand(sub):
            continue
        # Unknown info subcommand — conservative.
        pessimistic = True
    return pessimistic, escape_names


class _EscapeState:
    """Mutable per-proc escape accumulator used during the IR walk."""

    __slots__ = ("tags", "dynamic_barrier", "known_names")

    def __init__(self, known_names: Iterable[str]) -> None:
        self.tags: dict[str, EscapeTag] = {}
        self.dynamic_barrier: bool = False
        # Names the compiler already knows about (params, assigns, upvars).
        # Used by the "spill all" branch of alias inference to target only
        # real proc-locals rather than every string that looks name-ish.
        self.known_names: set[str] = set(known_names)

    def escape(self, name: str) -> None:
        """Mark ``name`` as needing frame storage."""
        self.tags[name] = EscapeTag.FRAME
        self.known_names.add(name)

    def escape_all_known(self) -> None:
        """Spill every known proc-local name (not proc-pessimistic)."""
        for n in self.known_names:
            self.tags[n] = EscapeTag.FRAME

    def mark_pessimistic(self) -> None:
        """Mark the whole proc as needing a dynamic-barrier fallback."""
        self.dynamic_barrier = True


def _collect_known_names(
    params: Iterable[str],
    body: IRScript,
) -> set[str]:
    """Gather every Tcl-local name mentioned by the proc.

    These are the candidates for the "spill all" branch when alias
    inference fails on a dynamic-name command.
    """
    names: set[str] = set(params)

    def _visit(stmts: tuple[IRStatement, ...]) -> None:
        for stmt in stmts:
            if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue)):
                names.add(stmt.name)
            elif isinstance(stmt, IRIncr):
                names.add(stmt.name)
            elif isinstance(stmt, IRCall):
                names.update(stmt.defs)
                names.update(stmt.reads)
            elif isinstance(stmt, IRIf):
                for clause in stmt.clauses:
                    _visit(clause.body.statements)
                if stmt.else_body is not None:
                    _visit(stmt.else_body.statements)
            elif isinstance(stmt, IRFor):
                _visit(stmt.init.statements)
                _visit(stmt.next.statements)
                _visit(stmt.body.statements)
            elif isinstance(stmt, IRWhile):
                _visit(stmt.body.statements)
            elif isinstance(stmt, IRForeach):
                for var_list, _list_arg in stmt.iterators:
                    names.update(var_list)
                _visit(stmt.body.statements)
            elif isinstance(stmt, IRCatch):
                if stmt.result_var:
                    names.add(stmt.result_var)
                if stmt.options_var:
                    names.add(stmt.options_var)
                _visit(stmt.body.statements)
            elif isinstance(stmt, IRTry):
                _visit(stmt.body.statements)
                for handler in stmt.handlers:
                    if handler.var_name:
                        names.add(handler.var_name)
                    if handler.options_var:
                        names.add(handler.options_var)
                    _visit(handler.body.statements)
                if stmt.finally_body is not None:
                    _visit(stmt.finally_body.statements)
            elif isinstance(stmt, IRSwitch):
                for arm in stmt.arms:
                    if arm.body is not None:
                        _visit(arm.body.statements)
                if stmt.default_body is not None:
                    _visit(stmt.default_body.statements)
            elif isinstance(stmt, IRBlock):
                _visit(stmt.body.statements)

    _visit(body.statements)
    return names


def _has_expand_word(call: IRCall) -> bool:
    """True if any word in ``call`` is ``{*}``-expanded."""
    toks = call.tokens
    if toks is None or toks.expand_word is None:
        return False
    return any(toks.expand_word)


def _handle_upvar(call: IRCall, state: _EscapeState) -> None:
    """Escape local-side vars of ``upvar ?level? src dst ...``."""
    args = call.args
    if not args:
        return
    # Detect the level arg the same way var_scoping does.
    head = args[0]
    is_level_literal = head.lstrip("-").isdigit() or (head.startswith("#") and head[1:].isdigit())
    if args[0].startswith("$") and not is_level_literal:
        # Dynamic level — pessimistic.
        state.mark_pessimistic()
        return
    for idx in upvar_local_declaration_indices("upvar", args):
        state.escape(args[idx])
    # A level like ``$var`` for a non-first arg is a dynamic *source*
    # name, harmless for local escape (it names a caller var, not ours).


def _handle_global(call: IRCall, state: _EscapeState) -> None:
    """Escape every var named in ``global a b c``."""
    for idx in global_declaration_indices(call.args):
        state.escape(call.args[idx])


def _handle_variable(call: IRCall, state: _EscapeState) -> None:
    """Escape every var declared by ``variable name ?value? ...``."""
    for idx in variable_declaration_indices(call.args):
        state.escape(call.args[idx])


def _handle_namespace_call(call: IRCall, state: _EscapeState) -> None:
    """Handle the ``namespace`` compound command for escape purposes."""
    args = call.args
    if not args:
        return
    sub = args[0]
    if sub == "upvar":
        # ``namespace upvar ns src dst ...`` — escape the dst positions.
        for idx in upvar_local_declaration_indices("namespace", args):
            state.escape(args[idx])


def _handle_info(call: IRCall, state: _EscapeState) -> None:
    """Classify ``info <subcmd> ...`` against the allow-list."""
    args = call.args
    if not args:
        # ``info`` with no subcommand: usage error at runtime — be safe.
        state.mark_pessimistic()
        return
    sub = args[0]
    if _is_dynamic_token(sub):
        state.mark_pessimistic()
        return
    if is_frame_inspecting_info_subcommand(sub):
        state.mark_pessimistic()
        return
    if sub == "exists":
        if len(args) < 2:
            return
        target = args[1]
        if _is_dynamic_token(target):
            state.mark_pessimistic()
            return
        # ``info exists name`` reads the name by string lookup — escape it.
        state.escape(target)
        return
    if is_safe_info_subcommand(sub):
        return
    # Unknown subcommand — be conservative.
    state.mark_pessimistic()


def _handle_dynamic_name_first(call: IRCall, state: _EscapeState) -> None:
    """Handle ``set`` / ``incr`` / ``append`` / ``lappend`` / ``unset``.

    The first arg is the variable name. If it is dynamic we escape the
    whole proc's known names (cheap over-approximation).

    Most of these forms are actually lowered to ``IRAssignValue`` /
    ``IRIncr`` / ``IRCall(command="append",...)`` before we see them;
    this catches the remaining ``IRCall`` fallthrough case.
    """
    args = call.args
    if not args:
        return
    name = args[0]
    if _is_dynamic_name(name):
        # No richer alias inference yet — spill all known names.
        state.escape_all_known()


def _escape_every_name_touched(
    stmts: tuple[IRStatement, ...],
    state: _EscapeState,
) -> None:
    """Escape every literal Tcl name the body writes, reads, or declares.

    Used for literal ``eval``/``uplevel #0`` bodies: the body runs through
    the interpreter which resolves names against the frame, so any name
    the body touches must be visible there.
    """
    for stmt in stmts:
        if state.dynamic_barrier:
            return
        if isinstance(stmt, (IRAssignConst, IRAssignValue, IRAssignExpr, IRIncr)):
            if not stmt.name or _is_dynamic_token(stmt.name):
                state.mark_pessimistic()
                return
            state.escape(stmt.name)
            # Still walk any embedded value hazards.
            if isinstance(stmt, (IRAssignConst, IRAssignValue)):
                _apply_value_scan(stmt.value, state)
            elif isinstance(stmt, IRAssignExpr):
                _apply_expr_scan(stmt.expr, state)
            elif isinstance(stmt, IRIncr) and stmt.amount:
                _apply_value_scan(stmt.amount, state)
        elif isinstance(stmt, IRCall):
            # Variables in defs/reads are targeted by name — escape them.
            for n in (*stmt.defs, *stmt.reads):
                if n and not _is_dynamic_token(n):
                    state.escape(n)
            _handle_call(stmt, state)
        elif isinstance(stmt, IRBarrier):
            _handle_barrier(stmt, state)
        elif isinstance(stmt, IRReturn):
            if stmt.value:
                _apply_value_scan(stmt.value, state)
            _apply_expr_scan(stmt.expr, state)
        elif isinstance(stmt, IRExprEval):
            _apply_expr_scan(stmt.expr, state)
        elif isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                _apply_expr_scan(clause.condition, state)
                _escape_every_name_touched(clause.body.statements, state)
            if stmt.else_body is not None:
                _escape_every_name_touched(stmt.else_body.statements, state)
        elif isinstance(stmt, IRFor):
            _escape_every_name_touched(stmt.init.statements, state)
            _apply_expr_scan(stmt.condition, state)
            _escape_every_name_touched(stmt.next.statements, state)
            _escape_every_name_touched(stmt.body.statements, state)
        elif isinstance(stmt, IRWhile):
            _apply_expr_scan(stmt.condition, state)
            _escape_every_name_touched(stmt.body.statements, state)
        elif isinstance(stmt, IRForeach):
            for var_list, list_arg in stmt.iterators:
                for n in var_list:
                    if n and not _is_dynamic_token(n):
                        state.escape(n)
                _apply_value_scan(list_arg, state)
            _escape_every_name_touched(stmt.body.statements, state)
        elif isinstance(stmt, IRCatch):
            if stmt.result_var:
                state.escape(stmt.result_var)
            if stmt.options_var:
                state.escape(stmt.options_var)
            _escape_every_name_touched(stmt.body.statements, state)
        elif isinstance(stmt, IRTry):
            _escape_every_name_touched(stmt.body.statements, state)
            for handler in stmt.handlers:
                if handler.var_name:
                    state.escape(handler.var_name)
                if handler.options_var:
                    state.escape(handler.options_var)
                _escape_every_name_touched(handler.body.statements, state)
            if stmt.finally_body is not None:
                _escape_every_name_touched(stmt.finally_body.statements, state)
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body is not None:
                    _escape_every_name_touched(arm.body.statements, state)
            if stmt.default_body is not None:
                _escape_every_name_touched(stmt.default_body.statements, state)
        elif isinstance(stmt, IRBlock):
            _escape_every_name_touched(stmt.body.statements, state)


def _handle_eval(barrier: IRBarrier, state: _EscapeState) -> None:
    """Handle ``eval body``.

    Literal body → lower and treat every name-access inside as an escape.
    Non-literal body → proc-pessimistic.
    """
    args = barrier.args
    if not args:
        # ``eval`` with no arg: usage error — be safe.
        state.mark_pessimistic()
        return
    body = args[-1] if len(args) == 1 else " ".join(args)
    if _is_dynamic_token(body):
        state.mark_pessimistic()
        return
    # Cheap scan first — any ``$var`` reference escapes that name.
    try:
        refs = VarReferenceScanner().scan_script(body)
    except Exception:  # noqa: BLE001 — any parse failure → pessimistic
        state.mark_pessimistic()
        return
    for ref in refs:
        state.escape(ref)
    # Recurse into the literal body and escape every name it touches.
    try:
        from ..lowering import lower_to_ir  # local import to avoid cycle

        sub_module = lower_to_ir(body)
    except Exception:  # noqa: BLE001 — any lowering failure → pessimistic
        state.mark_pessimistic()
        return
    _escape_every_name_touched(sub_module.top_level.statements, state)


def _handle_uplevel(barrier: IRBarrier, state: _EscapeState) -> None:
    """Handle ``uplevel ?level? body``.

    Only ``#0``/``0`` with a literal body is safe (body runs at global).
    Everything else is proc-pessimistic.
    """
    args = barrier.args
    if not args:
        state.mark_pessimistic()
        return
    # Determine whether the first arg is a level specifier.
    first = args[0]
    is_level_literal = first.lstrip("-").isdigit() or (
        first.startswith("#") and first[1:].isdigit()
    )
    if not is_level_literal:
        # No explicit level ⇒ defaults to 1 (caller frame) — pessimistic.
        state.mark_pessimistic()
        return
    # Only level #0 / 0 is safe (body runs in global scope, our locals
    # aren't visible). Any other literal level touches a non-global frame.
    if first not in ("#0", "0"):
        state.mark_pessimistic()
        return
    body_parts = args[1:]
    if not body_parts:
        state.mark_pessimistic()
        return
    body = body_parts[-1] if len(body_parts) == 1 else " ".join(body_parts)
    if _is_dynamic_token(body):
        # Even #0 with a dynamic body is pessimistic — it could redefine
        # globals that shadow our names, which is safe — but the body
        # might also reference our locals via arg-subst; safer to spill.
        state.mark_pessimistic()


def _handle_barrier(barrier: IRBarrier, state: _EscapeState) -> None:
    """Dispatch on the barrier command."""
    cmd = barrier.command
    if cmd == "eval":
        _handle_eval(barrier, state)
    elif cmd == "uplevel":
        _handle_uplevel(barrier, state)
    else:
        # Any other barrier (subst, trace, catch reraise, ...) — be safe.
        state.mark_pessimistic()


def _handle_call(call: IRCall, state: _EscapeState) -> None:
    """Dispatch on a normal ``IRCall``."""
    cmd = call.command
    # ``{*}``-expansion in an unknown call defeats argument-index-based
    # analysis (we can't tell where the name arg landed).
    if _has_expand_word(call) and cmd not in ("list", "concat"):
        state.mark_pessimistic()
        return

    if cmd == "upvar":
        _handle_upvar(call, state)
    elif cmd == "global":
        _handle_global(call, state)
    elif cmd == "variable":
        _handle_variable(call, state)
    elif cmd == "namespace":
        _handle_namespace_call(call, state)
    elif cmd == "info":
        _handle_info(call, state)
    elif cmd in _NAME_FIRST_COMMANDS:
        _handle_dynamic_name_first(call, state)
    else:
        # Unknown command with no {*} expansion: its ``defs`` and
        # ``reads`` already list the vars it touches by name. Those
        # don't escape — they're statically resolved writes.
        pass


def _apply_value_scan(value: str, state: _EscapeState) -> None:
    """Apply the info-hazard scan to an embedded value string."""
    if not value:
        return
    pessimistic, names = _scan_value_for_info_hazards(value)
    if pessimistic:
        state.mark_pessimistic()
        return
    for n in names:
        state.escape(n)


def _apply_expr_scan(expr: ExprNode | None, state: _EscapeState) -> None:
    """Apply the info-hazard scan to an expression's rendered source."""
    if expr is None:
        return
    text = getattr(expr, "source_text", None) or str(expr)
    _apply_value_scan(text, state)


def _walk(stmts: tuple[IRStatement, ...], state: _EscapeState) -> None:
    """Recurse into structured IR, visiting every statement."""
    for stmt in stmts:
        if state.dynamic_barrier:
            # Short-circuit: once pessimistic, the rest of the walk is
            # redundant — every var is already effectively FRAME.
            return
        if isinstance(stmt, IRCall):
            _handle_call(stmt, state)
        elif isinstance(stmt, IRBarrier):
            _handle_barrier(stmt, state)
        elif isinstance(stmt, IRAssignConst):
            if _is_dynamic_name(stmt.name):
                # Dynamic-name ``set``: name was a $var — spill everything.
                state.escape_all_known()
            _apply_value_scan(stmt.value, state)
        elif isinstance(stmt, IRAssignValue):
            if _is_dynamic_name(stmt.name):
                state.escape_all_known()
            _apply_value_scan(stmt.value, state)
        elif isinstance(stmt, IRAssignExpr):
            if _is_dynamic_name(stmt.name):
                state.escape_all_known()
            _apply_expr_scan(stmt.expr, state)
        elif isinstance(stmt, IRIncr):
            if _is_dynamic_name(stmt.name):
                state.escape_all_known()
            if stmt.amount:
                _apply_value_scan(stmt.amount, state)
        elif isinstance(stmt, IRReturn):
            if stmt.value:
                _apply_value_scan(stmt.value, state)
            _apply_expr_scan(stmt.expr, state)
        elif isinstance(stmt, IRExprEval):
            _apply_expr_scan(stmt.expr, state)
        elif isinstance(stmt, IRIf):
            for clause in stmt.clauses:
                _apply_expr_scan(clause.condition, state)
                _walk(clause.body.statements, state)
            if stmt.else_body is not None:
                _walk(stmt.else_body.statements, state)
        elif isinstance(stmt, IRFor):
            _walk(stmt.init.statements, state)
            _apply_expr_scan(stmt.condition, state)
            _walk(stmt.next.statements, state)
            _walk(stmt.body.statements, state)
        elif isinstance(stmt, IRWhile):
            _apply_expr_scan(stmt.condition, state)
            _walk(stmt.body.statements, state)
        elif isinstance(stmt, IRForeach):
            for _var_list, list_arg in stmt.iterators:
                _apply_value_scan(list_arg, state)
            _walk(stmt.body.statements, state)
        elif isinstance(stmt, IRCatch):
            _walk(stmt.body.statements, state)
        elif isinstance(stmt, IRTry):
            _walk(stmt.body.statements, state)
            for handler in stmt.handlers:
                _walk(handler.body.statements, state)
            if stmt.finally_body is not None:
                _walk(stmt.finally_body.statements, state)
        elif isinstance(stmt, IRSwitch):
            for arm in stmt.arms:
                if arm.body is not None:
                    _walk(arm.body.statements, state)
            if stmt.default_body is not None:
                _walk(stmt.default_body.statements, state)
        elif isinstance(stmt, IRBlock):
            _walk(stmt.body.statements, state)


def analyse_script(
    body: IRScript,
    params: Iterable[str] = (),
) -> ProcEscapeSummary:
    """Run the escape analysis over a single IR script body."""
    known = _collect_known_names(params, body)
    state = _EscapeState(known_names=known)
    _walk(body.statements, state)
    frame_needed = state.dynamic_barrier or any(
        tag is EscapeTag.FRAME for tag in state.tags.values()
    )
    return ProcEscapeSummary(
        tags=dict(state.tags),
        dynamic_barrier=state.dynamic_barrier,
        frame_needed=frame_needed,
    )
