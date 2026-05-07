"""Transfer functions for the var-escape analysis.

Walks the per-proc IR tree, classifying each variable as ``LOCAL`` or
``FRAME`` and setting ``dynamic_barrier`` when the analysis can no longer
bound the set of variables that might be observed by name.
"""

# canonicalisation: audited #246

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


# Commands whose codegen always uses a dedicated runtime helper and
# never falls back to the interpreter.  Calls to these commands do
# NOT require a runtime frame on the WASM side — they're straight-
# line primitives with no ``$var`` resolution needed at the callee.
# Keep this list audited: adding a command that secretly falls back
# will break eval-inside-proc semantics in escape-free procs.
_FRAMELESS_RUNTIME_COMMANDS: frozenset[str] = frozenset(
    {
        # List primitives
        "list",
        "lindex",
        "lrange",
        "linsert",
        "llength",
        "lsort",
        "lsearch",
        "lappend",
        "lreverse",
        "lreplace",
        "lrepeat",
        "lassign",
        "concat",
        # String primitives
        "split",
        "join",
        "string",
        # Arithmetic wrappers
        "expr",
        # Locally-handled scope declarations — no runtime frame needed.
        "global",
        "variable",
        "upvar",
        "namespace",
        # Self-assignment shapes already covered by IRAssign* / IRIncr.
        "set",
        "incr",
        "append",
        "unset",
        # I/O and diagnostics
        "puts",
        "return",
        "error",
        "continue",
        "break",
    }
)


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


def _array_element_array_name(name: str) -> str | None:
    """If ``name`` is an array-element ref (``arr(...)``) where the array
    name is a plain literal identifier, return that literal array name.

    ``set arr($k) v`` / ``unset arr($k)`` / ``incr arr($k)`` only touches
    the *array* by name — the index is dynamic but doesn't widen the
    proc's escape set beyond ``arr``.  Without this narrowing every such
    call falls into :func:`_handle_dynamic_name_first`'s fallback
    (``escape_all_known``) and forces the whole proc onto the slow
    frame path.

    Returns ``None`` for plain scalars, fully-dynamic names (``$x(y)``),
    and ``::``-qualified arrays (those route through the global table
    and don't need proc-local escape tracking).
    """
    if not name or "(" not in name or not name.endswith(")"):
        return None
    if name.startswith("$") or name.startswith("[") or name.startswith("::"):
        return None
    open_paren = name.find("(")
    if open_paren <= 0:
        return None
    arr = name[:open_paren]
    if "$" in arr or "[" in arr or "{" in arr or "}" in arr:
        return None
    return arr


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

    __slots__ = (
        "tags",
        "dynamic_barrier",
        "known_names",
        "upvar_source_names",
        "unbounded_upvar_source",
        "direct_callees",
        "has_fallback",
        "has_call_fallback",
        "literal_assigns",
    )

    def __init__(self, known_names: Iterable[str]) -> None:
        self.tags: dict[str, EscapeTag] = {}
        self.dynamic_barrier: bool = False
        # Names the compiler already knows about (params, assigns, upvars).
        # Used by the "spill all" branch of alias inference to target only
        # real proc-locals rather than every string that looks name-ish.
        self.known_names: set[str] = set(known_names)
        # Names in the caller's frame that this proc reaches by name via
        # ``upvar <positive-level>``.  Consumed by the interprocedural
        # pass to escape matching locals in callers.
        self.upvar_source_names: set[str] = set()
        # True if a dynamic-source upvar defeats enumeration (the proc
        # might name any caller var, so every caller local must spill).
        self.unbounded_upvar_source: bool = False
        # Statically-resolvable callees — qualified proc names this proc
        # invokes with a known command word.
        self.direct_callees: set[str] = set()
        # True if this proc definitely dispatches to the interpreter
        # at runtime regardless of what its callees look like —
        # IRBarrier, non-frameless command substitution in a value
        # string, ``{*}`` expansion, or a dynamic command word.
        # Codegen uses it to decide whether to emit frame_push / pop.
        self.has_fallback: bool = False
        # True if a non-frameless ``IRCall`` (with a statically
        # resolvable command word) was seen. Whether that actually
        # reaches the eval fallback depends on whether the callee is
        # itself a compiled proc — which only the interprocedural
        # pass can determine. The IR-only path treats this as a
        # fallback too (conservative); the CU-driven path downgrades
        # to ``False`` when every such callee resolves.
        self.has_call_fallback: bool = False
        # Sequential single-literal alias tracking.  Maps a var name to
        # the most recent literal identifier it was assigned, or to the
        # sentinel ``None`` once the var has been reassigned, had a
        # non-literal value written, or been declared escaped.  Used
        # by ``set $n`` alias inference to resolve ``$n`` back to the
        # literal when one writer was observed.
        self.literal_assigns: dict[str, str | None] = {}

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

    def record_upvar_source(self, src: str) -> None:
        """Record a literal caller-frame name this proc accesses via ``upvar``."""
        if src:
            self.upvar_source_names.add(src)

    def record_unbounded_upvar(self) -> None:
        """Record that the upvar source set cannot be statically enumerated."""
        self.unbounded_upvar_source = True

    def record_callee(self, qname: str) -> None:
        """Record a statically-resolvable callee."""
        if qname:
            self.direct_callees.add(qname)

    def record_fallback(self) -> None:
        """Record a definite interpreter dispatch (barrier-shaped)."""
        self.has_fallback = True

    def record_call_fallback(self) -> None:
        """Record a non-frameless ``IRCall`` with a static command word.

        Whether this really reaches the eval fallback is decided by
        the interprocedural pass; if the callee resolves to a
        compiled proc the downgrade there drops the flag.
        """
        self.has_call_fallback = True

    def note_literal_assign(self, name: str, value: str) -> None:
        """Record that ``name`` was assigned ``value`` via ``IRAssignConst``.

        Invalidates the tracked literal once a second writer appears:
        only the single-writer case is trusted for alias inference.
        """
        if not name or _is_dynamic_name(name):
            return
        if name in self.literal_assigns:
            # Second write — invalidate.
            self.literal_assigns[name] = None
            return
        self.literal_assigns[name] = value

    def invalidate_literal(self, name: str) -> None:
        """Mark ``name``'s tracked literal invalid (subsequent writer)."""
        if name:
            self.literal_assigns[name] = None

    def resolve_literal(self, token: str) -> str | None:
        """Resolve a dynamic var-name token (``$n`` / ``${n}``) to the
        literal it was assigned, if the single-writer rule holds.

        Returns ``None`` when the token isn't a simple ``$name``
        reference, when the tracked literal has been invalidated, or
        when the tracked value isn't a valid Tcl identifier.
        """
        if not token or not token.startswith("$"):
            return None
        # Strip ``$`` or ``${name}`` sigil.
        inner = token[2:-1] if token.startswith("${") and token.endswith("}") else token[1:]
        if not inner or any(ch in inner for ch in "[]$"):
            return None
        literal = self.literal_assigns.get(inner)
        if literal is None:
            return None
        # Guard against surprising values — only plain identifiers.
        if not literal or not literal.replace("_", "").replace(":", "").isalnum():
            return None
        return literal


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
    """Escape local-side vars of ``upvar ?level? src dst ...``.

    Also records the caller-frame *source* names when the level targets
    a caller (any level other than ``#0`` / ``0``). The interprocedural
    pass uses these to force matching vars in callers to ``FRAME``.
    """
    args = call.args
    if not args:
        return
    # Detect the level arg the same way var_scoping does.
    head = args[0]
    is_level_literal = head.lstrip("-").isdigit() or (head.startswith("#") and head[1:].isdigit())
    # Dynamic level detection — anything that isn't a literal integer
    # or ``#N`` form is potentially dynamic at runtime.  Previously
    # only ``$var`` was recognised; that missed ``upvar [expr {$n}]
    # ...`` / ``upvar "#${n}" ...`` / ``upvar [foo] ...`` /
    # ``upvar $::ns::level ...`` and let those callers stay in
    # WASM-local-only mirror state, breaking the upvar alias write-
    # back (Copilot review on PR #325).  Treat any non-literal head
    # as dynamic so the interprocedural pessimism kicks in.
    is_dynamic_level = not is_level_literal and (
        head.startswith("$")
        or head.startswith("[")
        or (head.startswith('"') and ("$" in head or "[" in head))
        or (head.startswith("#") and "$" in head)
        or (head.startswith("#") and "[" in head)
    )
    if is_dynamic_level:
        # Dynamic level — pessimistic.  Also flag the unbounded-upvar
        # source so the interprocedural pass forces callers to spill
        # every local: a dynamic level can resolve to any caller frame
        # and the source-name pairs that follow may name any of the
        # caller's vars.  Without this, ``proc Caller {} { set x 1;
        # TreeVars }; proc TreeVars {} { upvar $level $vname var; set
        # var 99 }`` left Caller's ``x`` as a WASM local mirror —
        # TreeVars's write to the runtime frame's ``x`` slot via
        # ``var`` never made it back to the WASM mirror, and ``$x``
        # in Caller still read 1.  See opt.test (OptTreeVars).
        state.mark_pessimistic()
        state.record_unbounded_upvar()
        return
    for idx in upvar_local_declaration_indices("upvar", args):
        state.escape(args[idx])

    # Determine pair-start offset and whether the target frame is the
    # caller (any non-``#0`` level) so we can collect source names.
    if is_level_literal:
        level_literal = head
        pairs_start = 1
    else:
        # No level → defaults to 1.
        level_literal = "1"
        pairs_start = 0
    targets_caller = level_literal not in ("#0", "0")
    if targets_caller:
        for i in range(pairs_start, len(args) - 1, 2):
            src = args[i]
            if _is_dynamic_token(src):
                state.record_unbounded_upvar()
            else:
                state.record_upvar_source(src)


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

    The first arg is the variable name. If the name is a ``$n``
    reference and ``n`` was assigned exactly one literal identifier
    earlier in the body, treat the call as targeting that literal
    (escape just that name). Otherwise fall back to spilling every
    known proc-local.

    Most of these forms are actually lowered to ``IRAssignValue`` /
    ``IRIncr`` / ``IRCall(command="append",...)`` before we see them;
    this catches the remaining ``IRCall`` fallthrough case.
    """
    args = call.args
    if not args:
        return
    name = args[0]
    if _is_dynamic_name(name):
        literal = state.resolve_literal(name)
        if literal is not None:
            state.escape(literal)
            return
        arr = _array_element_array_name(name)
        if arr is not None:
            # ``unset arr($k)`` / ``set arr($k) v`` / ``incr arr($k)`` —
            # only the array name escapes; the dynamic index doesn't
            # widen the touched-by-name set.  Without this narrowing,
            # the fallback below would spill every proc-local and
            # force the proc onto ``escape_all_known`` (and from there
            # often into the dynamic-barrier slow path), regressing
            # the foreach-loop-var visibility for any body that
            # mutates an array element by computed index.
            state.escape(arr)
            return
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
            if _is_eval_block(stmt):
                # Eval-shape IRBlock (barrier-relaxed ``eval {…}``)
                # — treat with the same escape pessimism as the
                # pre-relaxation :class:`IRBarrier` path.
                _handle_barrier(_synthesise_eval_barrier(stmt), state)
            else:
                _escape_every_name_touched(stmt.body.statements, state)


def _is_eval_block(stmt) -> bool:
    """True when an :class:`IRBlock` was produced by relaxing ``eval``.

    The two IRBlock shapes — ``namespace eval`` and ``eval`` — are
    distinguished by the leading command word recorded in
    ``source_tokens.argv_texts[0]``.  When tokens are missing (hand-
    crafted IR), default to the namespace-eval shape for back-compat.
    """
    tokens = getattr(stmt, "source_tokens", None)
    if tokens is None or not tokens.argv_texts:
        return False
    return tokens.argv_texts[0] == "eval"


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
    Everything else is proc-pessimistic — and a non-zero / dynamic
    level additionally flags the unbounded-upvar source so the
    interprocedural pass forces every caller to spill its locals
    (matches the ``upvar`` dynamic-source treatment; see
    ``::tcl::Lassign`` -> ``uplevel 1 [list ::set $vname value]``
    where the uplevel writes caller-side names the caller later
    reads via ``$varname``).
    """
    args = barrier.args
    if not args:
        state.mark_pessimistic()
        state.record_unbounded_upvar()
        return
    # Determine whether the first arg is a level specifier.
    first = args[0]
    is_level_literal = first.lstrip("-").isdigit() or (
        first.startswith("#") and first[1:].isdigit()
    )
    if not is_level_literal:
        # No explicit level ⇒ defaults to 1 (caller frame) — pessimistic.
        state.mark_pessimistic()
        state.record_unbounded_upvar()
        return
    # Only level #0 / 0 is safe (body runs in global scope, our locals
    # aren't visible). Any other literal level touches a non-global frame.
    if first not in ("#0", "0"):
        state.mark_pessimistic()
        state.record_unbounded_upvar()
        return
    body_parts = args[1:]
    if not body_parts:
        state.mark_pessimistic()
        state.record_unbounded_upvar()
        return
    body = body_parts[-1] if len(body_parts) == 1 else " ".join(body_parts)
    if _is_dynamic_token(body):
        # Even #0 with a dynamic body is pessimistic — it could redefine
        # globals that shadow our names, which is safe — but the body
        # might also reference our locals via arg-subst; safer to spill.
        state.mark_pessimistic()


def _synthesise_eval_barrier(stmt) -> IRBarrier:
    """Reconstitute an IRBarrier view of an eval-shape :class:`IRBlock`.

    See :func:`_synthesise_uplevel_barrier` for the rationale.
    ``source_args`` preserves the original eval word sequence (the
    braced body as the single argument), so ``_handle_eval`` can
    treat it the same way it does on the classic IRBarrier path.
    """
    return IRBarrier(
        range=stmt.range,
        reason="eval relaxed",
        command="eval",
        canonical_command="::eval",
        args=tuple(stmt.source_args) if stmt.source_args else (),
        tokens=stmt.source_tokens,
    )


def _synthesise_uplevel_barrier(stmt) -> IRBarrier:
    """Reconstitute an IRBarrier from an :class:`IRUpFrame` for analysis.

    Only used by the escape-propagation walk — the resulting barrier
    is discarded after the dispatch.  ``source_tokens.argv_texts``
    preserves the original ``uplevel ?level? body`` word sequence, so
    ``_handle_uplevel`` sees the same shape as before the relaxation.
    """
    tokens = stmt.source_tokens
    args: tuple[str, ...] = ()
    if tokens is not None and tokens.argv_texts:
        args = tuple(tokens.argv_texts[1:])
    return IRBarrier(
        range=stmt.range,
        reason="uplevel relaxed",
        command="uplevel",
        canonical_command="::uplevel",
        args=args,
        tokens=tokens,
    )


def _handle_barrier(barrier: IRBarrier, state: _EscapeState) -> None:
    """Dispatch on the barrier command."""
    # Any IRBarrier means the codegen can dispatch to the interpreter;
    # the proc prologue must push a frame so the fallback sees locals.
    state.record_fallback()
    cmd = barrier.canonical_command
    if cmd == "::eval":
        _handle_eval(barrier, state)
    elif cmd == "::uplevel":
        _handle_uplevel(barrier, state)
    else:
        # Any other barrier (subst, trace, catch reraise, ...) — be safe.
        state.mark_pessimistic()


def _handle_call(call: IRCall, state: _EscapeState) -> None:
    """Dispatch on a normal ``IRCall``.

    Matches the canonical command form (``::ns::cmd``) for builtin-
    detection branches so qualified spellings, ``interp alias`` aliases,
    and namespace-imported builtins all hit the same path.  See issue
    #246.
    """
    canonical = call.canonical_command or call.command
    cmd = call.command
    # Commands outside the frameless-runtime allow-list could reach
    # the eval fallback via ``_emit_eval_fallback``; the proc must
    # keep its runtime frame so the fallback can resolve ``$var``
    # against our locals.  Frameless commands (list / string / expr
    # / etc.) are dispatched to pure runtime helpers and don't need
    # a frame on the caller side.  Both bare and ``::``-prefixed
    # forms qualify (``::puts`` and ``puts`` both resolve to the
    # same global builtin).
    bare_cmd = canonical[2:] if canonical.startswith("::") else canonical
    if bare_cmd not in _FRAMELESS_RUNTIME_COMMANDS:
        # A dynamic command word can't be resolved interprocedurally —
        # treat it as a definite fallback.  Static command words
        # (bare names or ``::``-qualified identifiers) go into the
        # downgradable call-fallback bucket instead, so the
        # interprocedural pass can drop the flag when the callee
        # turns out to be a compiled proc.
        if not cmd or _is_dynamic_token(cmd):
            state.record_fallback()
        else:
            state.record_call_fallback()

    # ``{*}``-expansion in an unknown call defeats argument-index-based
    # analysis (we can't tell where the name arg landed).
    if _has_expand_word(call) and bare_cmd not in ("list", "concat"):
        state.mark_pessimistic()
        return

    if canonical == "::upvar":
        _handle_upvar(call, state)
    elif canonical == "::global":
        _handle_global(call, state)
    elif canonical == "::variable":
        _handle_variable(call, state)
    elif canonical == "::namespace":
        _handle_namespace_call(call, state)
    elif canonical == "::info":
        _handle_info(call, state)
    elif bare_cmd in _NAME_FIRST_COMMANDS:
        _handle_dynamic_name_first(call, state)
    else:
        # Unknown command with no {*} expansion: its ``defs`` and
        # ``reads`` already list the vars it touches by name. Those
        # don't escape — they're statically resolved writes.
        pass

    # Record statically resolvable callees for interprocedural propagation.
    # Bare or ``::``-qualified command words are candidates; anything that
    # looks like a substitution (``$foo`` / ``[bar]``) is ignored.
    if cmd and not _is_dynamic_token(cmd):
        state.record_callee(cmd)


# Match the leading command word of a ``[cmd …]`` substitution.
# ``[a-zA-Z_][\w:]*`` accepts namespace-qualified heads (``::set``,
# ``::ns::cmd``) as well as bare identifiers; missing them would
# leave ``[::set …]`` looking like a frameless call and let the codegen
# elide the frame even though ``set`` is going to dispatch through the
# eval fallback.  The leading-character class rejects digits so we don't
# match accidental ``[0`` sequences (which can't be commands anyway).
_CMD_SUBST_HEAD_RE = re.compile(r"\[\s*((?:::)?[a-zA-Z_][\w:]*)")


def _normalise_cmd_subst_head(head: str) -> str:
    """Strip the leading ``::`` from a substitution head for allow-list lookup.

    The frameless allow-list keys commands by their bare identifier
    (``set``, ``list``, …); a substitution writing ``[::set foo 1]``
    is the same command, just qualified.
    """
    return head.removeprefix("::") if head.startswith("::") else head


def _apply_value_scan(value: str, state: _EscapeState) -> None:
    """Apply the info-hazard scan to an embedded value string."""
    if not value:
        return
    # Walk every ``[cmd …]`` substitution's leading command word.
    # Substitutions whose head isn't in the frameless allow-list may
    # reach the eval fallback (or a command that needs a frame for
    # ``$var`` resolution) — flag ``has_fallback`` so the codegen
    # keeps the runtime frame around.  Also record the head as a
    # direct callee so the interprocedural pass can propagate the
    # callee's pessimistic taint back to this caller (e.g.
    # ``set lg [::tcl::Lassign $item varname ...]`` — Lassign
    # uplevel-writes ``varname`` and OptNormalizeOne later reads
    # ``$varname``; without recording Lassign as a callee here, the
    # propagation can't make OptNormalizeOne pessimistic and the
    # frame-bridge sync/readback never fires around the [Lassign...]
    # call site).
    if "[" in value:
        for match in _CMD_SUBST_HEAD_RE.finditer(value):
            head = _normalise_cmd_subst_head(match.group(1))
            if head not in _FRAMELESS_RUNTIME_COMMANDS:
                state.record_fallback()
                if head and not _is_dynamic_token(head):
                    state.record_callee(head)
                break
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
    from ..ir import IRUpFrame

    for stmt in stmts:
        if state.dynamic_barrier:
            # Short-circuit: once pessimistic, the rest of the walk is
            # redundant — every var is already effectively FRAME.
            return
        if isinstance(stmt, IRCall):
            _handle_call(stmt, state)
        elif isinstance(stmt, IRBarrier):
            _handle_barrier(stmt, state)
        elif isinstance(stmt, IRUpFrame):
            # Barrier-relaxed ``uplevel`` — reconstitute an IRBarrier
            # view of the original call so ``_handle_uplevel`` can
            # apply the same pessimism rules it always did.  The
            # parsed body we carry in IRUpFrame.body is an
            # optimisation for future passes; for escape analysis
            # the classic barrier logic is still the source of truth.
            _handle_barrier(_synthesise_uplevel_barrier(stmt), state)
        elif isinstance(stmt, IRAssignConst):
            if _is_dynamic_name(stmt.name):
                literal = state.resolve_literal(stmt.name)
                if literal is not None:
                    state.escape(literal)
                else:
                    state.escape_all_known()
            else:
                state.note_literal_assign(stmt.name, stmt.value)
            _apply_value_scan(stmt.value, state)
        elif isinstance(stmt, IRAssignValue):
            if _is_dynamic_name(stmt.name):
                literal = state.resolve_literal(stmt.name)
                if literal is not None:
                    state.escape(literal)
                else:
                    state.escape_all_known()
            else:
                # A plain-literal value (no ``$var`` / ``[cmd]``) is
                # still trusted as an alias literal for the single-
                # writer case — lowering picks IRAssignValue over
                # IRAssignConst for non-numeric barewords.
                if stmt.value and not _is_dynamic_token(stmt.value):
                    state.note_literal_assign(stmt.name, stmt.value)
                else:
                    state.invalidate_literal(stmt.name)
            _apply_value_scan(stmt.value, state)
        elif isinstance(stmt, IRAssignExpr):
            if _is_dynamic_name(stmt.name):
                literal = state.resolve_literal(stmt.name)
                if literal is not None:
                    state.escape(literal)
                else:
                    state.escape_all_known()
            else:
                state.invalidate_literal(stmt.name)
            _apply_expr_scan(stmt.expr, state)
        elif isinstance(stmt, IRIncr):
            if _is_dynamic_name(stmt.name):
                literal = state.resolve_literal(stmt.name)
                if literal is not None:
                    state.escape(literal)
                else:
                    state.escape_all_known()
            else:
                state.invalidate_literal(stmt.name)
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
            if _is_eval_block(stmt):
                _handle_barrier(_synthesise_eval_barrier(stmt), state)
            else:
                _walk(stmt.body.statements, state)


def analyse_script(
    body: IRScript,
    params: Iterable[str] = (),
) -> ProcEscapeSummary:
    """Run the escape analysis over a single IR script body.

    The returned summary is *intraprocedural* — callee-induced escapes
    haven't been folded in yet. Run the interprocedural pass
    (``solve_interprocedural_escape``) to produce the final summary
    the codegen should consume.
    """
    known = _collect_known_names(params, body)
    state = _EscapeState(known_names=known)
    _walk(body.statements, state)
    frame_needed = state.dynamic_barrier or any(
        tag is EscapeTag.FRAME for tag in state.tags.values()
    )
    # S3.4: tentative pure_leaf — finalised by the interprocedural
    # pass, which can only downgrade.  Pure means: no escape (no
    # FRAME tag, no dynamic_barrier), no upvar source out, no
    # fallback, no unresolved upvar source.  A proc that ticks all
    # those locally STILL needs every callee to be pure_leaf
    # before this flag stays True after the IPA fixpoint — that
    # downgrade happens in ``_interprocedural.py``.
    pure_leaf = (
        not frame_needed
        and not state.has_fallback
        and not state.has_call_fallback
        and not state.upvar_source_names
        and not state.unbounded_upvar_source
    )
    return ProcEscapeSummary(
        tags=dict(state.tags),
        dynamic_barrier=state.dynamic_barrier,
        frame_needed=frame_needed,
        upvar_source_names=frozenset(state.upvar_source_names),
        unbounded_upvar_source=state.unbounded_upvar_source,
        direct_callees=frozenset(state.direct_callees),
        # ``has_fallback`` records definite intraprocedural fallback
        # reasons only (IRBarrier, non-frameless cmd substitutions,
        # ``{*}`` expansion, dynamic command words).  Non-frameless
        # static-name IRCalls go in ``has_call_fallback`` instead so
        # the interprocedural pass can downgrade when each callee
        # resolves to a compiled proc.
        has_fallback=state.has_fallback,
        has_call_fallback=state.has_call_fallback,
        pure_leaf=pure_leaf,
    )
