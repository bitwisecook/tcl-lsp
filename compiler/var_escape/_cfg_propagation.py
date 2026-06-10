"""Flow-sensitive var-escape propagation over CFG + SSA.

The tree-walk path in ``_propagation.py`` is correct but collapses
every statement under a structured control-flow node into a single
per-proc state.  When a ``CompilationUnit`` is available we can do
better: walk the per-proc CFG in SSA form and key escape tags by
``SSAValueKey = (name, version)``.

The information is strictly monotone (a tag only ever rises from
``LOCAL`` to ``FRAME``), so a single walk in any block order produces
the final result. Phi nodes trivially satisfy the lattice: since
codegen collapses per-name anyway, a phi result is ``FRAME`` if any
incoming version is ``FRAME``.

Per-SSA-version precision is still recorded in the returned
:attr:`ssa_tags` map so downstream consumers (for example a future
register allocator) can use it. Codegen reads :attr:`name_tags`,
which is the join over every SSA version of each name.
"""

# canonicalisation: audited #246

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Iterable

from compiler.var_scoping import (
    global_declaration_indices,
    upvar_local_declaration_indices,
    variable_declaration_indices,
)

from ..cfg import CFGBranch, CFGFunction
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRIncr,
)
from ..ssa import SSABlock, SSAFunction, SSAStatement, SSAValueKey
from ._info_subcommands import (
    is_frame_inspecting_info_subcommand,
    is_safe_info_subcommand,
)
from ._propagation import (
    _CMD_SUBST_HEAD_RE,
    _NAME_FIRST_COMMANDS,
    _array_element_array_name,
    _is_dynamic_name,
    _is_dynamic_token,
    _is_frameless_runtime,
    _normalise_cmd_subst_head,
    _scan_value_for_info_hazards,
)
from ._types import (
    Barrier,
    BarrierKind,
    EscapeReason,
    EscapeReasonKind,
    EscapeTag,
)


@dataclass(slots=True)
class CfgEscapeResult:
    """Result of the CFG+SSA flow-sensitive analysis.

    ``ssa_tags`` is the authoritative per-version result. ``name_tags``
    is the per-name collapse used by codegen — a name is ``FRAME`` if
    any of its SSA versions was tagged ``FRAME``.
    """

    ssa_tags: dict[SSAValueKey, EscapeTag] = field(default_factory=dict)
    name_tags: dict[str, EscapeTag] = field(default_factory=dict)
    dynamic_barrier: bool = False
    upvar_source_names: set[str] = field(default_factory=set)
    unbounded_upvar_source: bool = False
    direct_callees: set[str] = field(default_factory=set)
    has_fallback: bool = False
    has_call_fallback: bool = False
    known_names: set[str] = field(default_factory=set)
    barriers: tuple[Barrier, ...] = ()
    tag_reasons: dict[str, tuple[EscapeReason, ...]] = field(default_factory=dict)


class _CfgState:
    """Mutable accumulator for the CFG walk.

    Tracks per-SSA-version tags plus per-name auxiliary data. The
    auxiliary data is monotonic — no need for block-level join state.
    """

    __slots__ = (
        "ssa_tags",
        "dynamic_barrier",
        "known_names",
        "upvar_source_names",
        "unbounded_upvar_source",
        "direct_callees",
        "has_fallback",
        "has_call_fallback",
        "literal_assigns",
        "_ssa_for_name",
        "barriers",
        "tag_reasons",
    )

    def __init__(self, known_names: Iterable[str]) -> None:
        self.ssa_tags: dict[SSAValueKey, EscapeTag] = {}
        self.dynamic_barrier: bool = False
        self.known_names: set[str] = set(known_names)
        self.upvar_source_names: set[str] = set()
        self.unbounded_upvar_source: bool = False
        self.direct_callees: set[str] = set()
        self.has_fallback: bool = False
        self.has_call_fallback: bool = False
        self.barriers: list[Barrier] = []
        self.tag_reasons: dict[str, list[EscapeReason]] = {}
        # Maps a var name to the literal it was last assigned, for the
        # cheap single-writer alias inference. ``None`` once invalidated.
        self.literal_assigns: dict[str, str | None] = {}
        # Tracks the most recent SSA version seen per name — used when
        # an escape is triggered at a statement that doesn't write
        # the var (e.g. upvar declaration that escapes named locals).
        self._ssa_for_name: dict[str, int] = {}

    def _version_for(self, name: str, defs: dict[str, int]) -> int:
        """Return the SSA version to tag for ``name`` at this statement.

        Prefers the statement's own ``defs`` (a definition on this
        line) before falling back to the most recent seen version.
        Version 0 is used as a sentinel when the name has never been
        versioned yet.
        """
        if name in defs:
            return defs[name]
        return self._ssa_for_name.get(name, 0)

    def remember_versions(self, stmt: SSAStatement) -> None:
        """Update the name → latest-version map from an SSAStatement."""
        for name, version in stmt.defs.items():
            self._ssa_for_name[name] = version
            self.known_names.add(name)
        for name, version in stmt.uses.items():
            # Keep the latest-seen version if the use is newer than the
            # def map — shouldn't happen in well-formed SSA but is
            # defensive.
            if version > self._ssa_for_name.get(name, 0):
                self._ssa_for_name[name] = version

    def escape(
        self,
        name: str,
        defs: dict[str, int] | None = None,
        reason: EscapeReason | None = None,
    ) -> None:
        """Mark ``name``'s current SSA version as FRAME."""
        if not name or _is_dynamic_name(name):
            return
        version = self._version_for(name, defs or {})
        self.ssa_tags[(name, version)] = EscapeTag.FRAME
        self.known_names.add(name)
        self._ssa_for_name.setdefault(name, version)
        if reason is not None:
            self.tag_reasons.setdefault(name, []).append(reason)

    def escape_all_known(
        self,
        defs: dict[str, int] | None = None,
        reason: EscapeReason | None = None,
    ) -> None:
        """Mark every known proc-local name as FRAME at its current version."""
        if reason is None:
            reason = EscapeReason(kind=EscapeReasonKind.ESCAPE_ALL_KNOWN)
        for name in list(self.known_names):
            if _is_dynamic_name(name):
                continue
            version = self._version_for(name, defs or {})
            self.ssa_tags[(name, version)] = EscapeTag.FRAME
            self.tag_reasons.setdefault(name, []).append(reason)

    def mark_pessimistic(self, barrier: Barrier | None = None) -> None:
        self.dynamic_barrier = True
        if barrier is not None:
            self.barriers.append(barrier)

    def record_upvar_source(self, src: str) -> None:
        if src:
            self.upvar_source_names.add(src)

    def record_unbounded_upvar(self) -> None:
        self.unbounded_upvar_source = True

    def record_callee(self, qname: str) -> None:
        if qname:
            self.direct_callees.add(qname)

    def record_fallback(self) -> None:
        self.has_fallback = True

    def record_call_fallback(self) -> None:
        self.has_call_fallback = True

    def note_literal_assign(self, name: str, value: str) -> None:
        if not name or _is_dynamic_name(name):
            return
        if name in self.literal_assigns:
            self.literal_assigns[name] = None
            return
        self.literal_assigns[name] = value

    def invalidate_literal(self, name: str) -> None:
        if name:
            self.literal_assigns[name] = None

    def resolve_literal(self, token: str) -> str | None:
        if not token or not token.startswith("$"):
            return None
        inner = token[2:-1] if token.startswith("${") and token.endswith("}") else token[1:]
        if not inner or any(ch in inner for ch in "[]$"):
            return None
        literal = self.literal_assigns.get(inner)
        if literal is None:
            return None
        if not literal or not literal.replace("_", "").replace(":", "").isalnum():
            return None
        return literal


def _collect_known_names_from_cfg(
    params: Iterable[str],
    cfg: CFGFunction,
    ssa: SSAFunction,
) -> set[str]:
    """Gather every name that has at least one SSA definition or appearance."""
    names: set[str] = set(params)
    for block in ssa.blocks.values():
        for phi in block.phis:
            names.add(phi.name)
        for stmt in block.statements:
            names.update(stmt.uses)
            names.update(stmt.defs)
    return names


def _has_expand_word(call: IRCall) -> bool:
    toks = call.tokens
    if toks is None or toks.expand_word is None:
        return False
    return any(toks.expand_word)


def _apply_value_scan(value: str, state: _CfgState, defs: dict[str, int]) -> None:
    if not value:
        return
    if "[" in value:
        for match in _CMD_SUBST_HEAD_RE.finditer(value):
            head = _normalise_cmd_subst_head(match.group(1))
            if not _is_frameless_runtime(head):
                state.record_fallback()
                # Also record the embedded command head as a direct
                # callee so the interprocedural pass can propagate
                # the callee's ``unbounded_upvar_source`` /
                # ``upvar_source_names`` back to this caller.  Without
                # this, ``set lg [::tcl::Lassign $item varname ...]``
                # in OptNormalizeOne records no callee for Lassign
                # and Lassign's pessimistic taint never reaches its
                # caller.
                if head and not _is_dynamic_token(head):
                    state.record_callee(head)
                break
    pessimistic, names = _scan_value_for_info_hazards(value)
    if pessimistic:
        state.mark_pessimistic(Barrier(BarrierKind.INFO, detail="bracketed [info ...] in value"))
        return
    for n in names:
        state.escape(
            n,
            defs,
            EscapeReason(kind=EscapeReasonKind.INFO_EXISTS, detail=f"info exists {n}"),
        )


def _apply_expr_scan(expr: object, state: _CfgState, defs: dict[str, int]) -> None:
    if expr is None:
        return
    text = getattr(expr, "source_text", None) or str(expr)
    _apply_value_scan(text, state, defs)


def _handle_upvar(call: IRCall, state: _CfgState, defs: dict[str, int]) -> None:
    args = call.args
    if not args:
        return
    head = args[0]
    is_level_literal = head.lstrip("-").isdigit() or (head.startswith("#") and head[1:].isdigit())
    # Dynamic level detection — see the matching block in
    # ``_propagation._handle_upvar`` for the full Copilot-review
    # rationale.  Any non-literal head shape (``[expr ...]``,
    # ``"#${n}"``, ``$::ns::level``, …) is potentially dynamic at
    # runtime; recognising only ``$var`` would let those callsites
    # bypass the pessimism and leave WASM-local mirrors stale
    # after an alias write.
    is_dynamic_level = not is_level_literal and (
        head.startswith("$")
        or head.startswith("[")
        or (head.startswith('"') and ("$" in head or "[" in head))
        or (head.startswith("#") and "$" in head)
        or (head.startswith("#") and "[" in head)
    )
    if is_dynamic_level:
        # Dynamic level — pessimistic.  Also flag the unbounded-upvar
        # source so the interprocedural pass forces every caller to
        # spill its locals.  See ``_propagation._handle_upvar`` for the
        # full rationale (opt.test OptTreeVars).
        state.mark_pessimistic(Barrier(BarrierKind.UPVAR, detail="upvar $level"))
        state.record_unbounded_upvar()
        return
    for idx in upvar_local_declaration_indices("upvar", args):
        state.escape(
            args[idx],
            defs,
            EscapeReason(
                kind=EscapeReasonKind.UPVAR_SOURCE,
                detail=f"upvar bind {args[idx]}",
            ),
        )

    if is_level_literal:
        level_literal = head
        pairs_start = 1
    else:
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


def _handle_global(call: IRCall, state: _CfgState, defs: dict[str, int]) -> None:
    for idx in global_declaration_indices(call.args):
        state.escape(
            call.args[idx],
            defs,
            EscapeReason(
                kind=EscapeReasonKind.UPVAR_SOURCE,
                detail=f"global {call.args[idx]}",
            ),
        )


def _handle_variable(call: IRCall, state: _CfgState, defs: dict[str, int]) -> None:
    for idx in variable_declaration_indices(call.args):
        state.escape(
            call.args[idx],
            defs,
            EscapeReason(
                kind=EscapeReasonKind.UPVAR_SOURCE,
                detail=f"variable {call.args[idx]}",
            ),
        )


def _handle_namespace_call(call: IRCall, state: _CfgState, defs: dict[str, int]) -> None:
    args = call.args
    if not args:
        return
    sub = args[0]
    if sub == "upvar":
        for idx in upvar_local_declaration_indices("namespace", args):
            state.escape(
                args[idx],
                defs,
                EscapeReason(
                    kind=EscapeReasonKind.UPVAR_SOURCE,
                    detail=f"namespace upvar bind {args[idx]}",
                ),
            )


def _handle_info(call: IRCall, state: _CfgState, defs: dict[str, int]) -> None:
    args = call.args
    if not args:
        state.mark_pessimistic(Barrier(BarrierKind.INFO, detail="info (no subcommand)"))
        return
    sub = args[0]
    if _is_dynamic_token(sub):
        state.mark_pessimistic(Barrier(BarrierKind.INFO, detail="info $dynamic"))
        return
    if is_frame_inspecting_info_subcommand(sub):
        state.mark_pessimistic(Barrier(BarrierKind.INFO, detail=f"info {sub}"))
        return
    if sub == "exists":
        if len(args) < 2:
            return
        target = args[1]
        if _is_dynamic_token(target):
            state.mark_pessimistic(Barrier(BarrierKind.INFO, detail="info exists $dynamic"))
            return
        state.escape(
            target,
            defs,
            EscapeReason(kind=EscapeReasonKind.INFO_EXISTS, detail=f"info exists {target}"),
        )
        return
    if is_safe_info_subcommand(sub):
        return
    state.mark_pessimistic(Barrier(BarrierKind.INFO, detail=f"info {sub} (unclassified)"))


def _handle_dynamic_name_first(
    call: IRCall,
    state: _CfgState,
    defs: dict[str, int],
) -> None:
    args = call.args
    if not args:
        return
    name = args[0]
    if _is_dynamic_name(name):
        literal = state.resolve_literal(name)
        if literal is not None:
            state.escape(
                literal,
                defs,
                EscapeReason(
                    kind=EscapeReasonKind.DYN_NAME_RESOLVED,
                    detail=f"{call.command} {name} → {literal}",
                ),
            )
            return
        arr = _array_element_array_name(name)
        if arr is not None:
            # See ``_propagation._handle_dynamic_name_first`` for the
            # rationale: array-element refs only touch the array name.
            state.escape(
                arr,
                defs,
                EscapeReason(
                    kind=EscapeReasonKind.DYN_NAME_RESOLVED,
                    detail=f"{call.command} {arr}(...)",
                ),
            )
            return
        state.escape_all_known(
            defs,
            EscapeReason(
                kind=EscapeReasonKind.ESCAPE_ALL_KNOWN,
                detail=f"{call.command} {name}",
            ),
        )


def _escape_every_name_touched_tree(
    stmts: tuple,
    state: _CfgState,
    defs: dict[str, int],
) -> None:
    """Tree-walk variant used inside ``eval {literal}`` bodies.

    Eval bodies lower to a fresh sub-IRModule whose statements aren't
    part of the enclosing SSA. For that sub-walk we reuse the plain
    per-statement handlers; any name touched escapes at the caller's
    current SSA version.
    """
    for stmt in stmts:
        if state.dynamic_barrier:
            return
        if isinstance(stmt, (IRAssignConst, IRAssignValue, IRAssignExpr, IRIncr)):
            if not stmt.name or _is_dynamic_name(stmt.name):
                state.mark_pessimistic(
                    Barrier(BarrierKind.DYN_NAME, detail="dynamic name in eval body")
                )
                return
            state.escape(
                stmt.name,
                defs,
                EscapeReason(
                    kind=EscapeReasonKind.EVAL_REFERENCE,
                    detail=f"assigned in eval body: {stmt.name}",
                ),
            )
            if isinstance(stmt, (IRAssignConst, IRAssignValue)):
                _apply_value_scan(stmt.value, state, defs)
            elif isinstance(stmt, IRAssignExpr):
                _apply_expr_scan(stmt.expr, state, defs)
            elif isinstance(stmt, IRIncr) and stmt.amount:
                _apply_value_scan(stmt.amount, state, defs)
        elif isinstance(stmt, IRCall):
            for n in (*stmt.defs, *stmt.reads):
                if n and not _is_dynamic_token(n):
                    state.escape(
                        n,
                        defs,
                        EscapeReason(
                            kind=EscapeReasonKind.EVAL_REFERENCE,
                            detail=f"referenced in eval body: {n}",
                        ),
                    )
            _handle_call(stmt, state, defs)
        elif isinstance(stmt, IRBarrier):
            _handle_barrier(stmt, state, defs)
        else:
            from ..ir import IRBlock as _IRBlock
            from ..ir import IRUpFrame

            if isinstance(stmt, IRUpFrame):
                _handle_barrier(_synthesise_uplevel_barrier(stmt), state, defs)
            elif isinstance(stmt, _IRBlock) and _is_eval_block(stmt):
                _handle_barrier(_synthesise_eval_barrier(stmt), state, defs)


def _handle_eval(
    barrier: IRBarrier,
    state: _CfgState,
    defs: dict[str, int],
) -> None:
    args = barrier.args
    if not args:
        state.mark_pessimistic(Barrier(BarrierKind.EVAL, detail="eval (no body)"))
        return
    body = args[-1] if len(args) == 1 else " ".join(args)
    if _is_dynamic_token(body):
        state.mark_pessimistic(Barrier(BarrierKind.EVAL, detail="eval $body (dynamic)"))
        return
    try:
        from ..var_refs import VarReferenceScanner

        refs = VarReferenceScanner().scan_script(body)
    except Exception:  # noqa: BLE001
        state.mark_pessimistic(Barrier(BarrierKind.EVAL, detail="eval body parse failure"))
        return
    for ref in refs:
        state.escape(
            ref,
            defs,
            EscapeReason(
                kind=EscapeReasonKind.EVAL_REFERENCE,
                detail=f"$ref in eval body: {ref}",
            ),
        )
    try:
        from ..lowering import lower_to_ir

        sub_module = lower_to_ir(body)
    except Exception:  # noqa: BLE001
        state.mark_pessimistic(Barrier(BarrierKind.EVAL, detail="eval body lowering failure"))
        return
    _escape_every_name_touched_tree(sub_module.top_level.statements, state, defs)


def _handle_uplevel(
    barrier: IRBarrier,
    state: _CfgState,
    defs: dict[str, int],
) -> None:
    args = barrier.args
    if not args:
        state.mark_pessimistic(Barrier(BarrierKind.UPVAR, detail="uplevel (no args)"))
        state.record_unbounded_upvar()
        return
    first = args[0]
    is_level_literal = first.lstrip("-").isdigit() or (
        first.startswith("#") and first[1:].isdigit()
    )
    if not is_level_literal:
        # Dynamic level — body could write anywhere up the frame stack.
        state.mark_pessimistic(Barrier(BarrierKind.UPVAR, detail="uplevel $level (dynamic)"))
        state.record_unbounded_upvar()
        return
    if first not in ("#0", "0"):
        # Caller-frame uplevel — body runs in a caller's frame and
        # may write any local there.  Treat as unbounded so callers
        # spill every local.  (See ``::tcl::Lassign`` -> ``uplevel 1
        # [list ::set $vname value]``: the uplevel writes
        # caller-side names that callers (e.g. OptNormalizeOne) read
        # via ``$varname`` afterwards.)
        state.mark_pessimistic(Barrier(BarrierKind.UPVAR, detail=f"uplevel {first}"))
        state.record_unbounded_upvar()
        return
    body_parts = args[1:]
    if not body_parts:
        state.mark_pessimistic(Barrier(BarrierKind.UPVAR, detail="uplevel #0 (no body)"))
        state.record_unbounded_upvar()
        return
    body = body_parts[-1] if len(body_parts) == 1 else " ".join(body_parts)
    if _is_dynamic_token(body):
        # ``uplevel #0 $body`` — body content is unknown, may touch
        # any global.  Globals don't need ``upvar_source_names``
        # (they're handled by global storage), so just go pessimistic.
        state.mark_pessimistic(Barrier(BarrierKind.UPVAR, detail="uplevel #0 $body (dynamic)"))


def _synthesise_uplevel_barrier(stmt) -> IRBarrier:
    """Reconstitute an IRBarrier from an :class:`IRUpFrame`.

    Mirrors the propagation-side helper; see
    :func:`compiler.var_escape._propagation._synthesise_uplevel_barrier`.
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


def _is_eval_block(stmt) -> bool:
    """True when an :class:`IRBlock` was produced by relaxing ``eval``."""
    tokens = getattr(stmt, "source_tokens", None)
    if tokens is None or not tokens.argv_texts:
        return False
    return tokens.argv_texts[0] == "eval"


def _synthesise_eval_barrier(stmt) -> IRBarrier:
    """Reconstitute an IRBarrier view of an eval-shape :class:`IRBlock`."""
    return IRBarrier(
        range=stmt.range,
        reason="eval relaxed",
        command="eval",
        canonical_command="::eval",
        args=tuple(stmt.source_args) if stmt.source_args else (),
        tokens=stmt.source_tokens,
    )


def _handle_barrier(
    barrier: IRBarrier,
    state: _CfgState,
    defs: dict[str, int],
) -> None:
    state.record_fallback()
    cmd = barrier.canonical_command
    if cmd == "::eval":
        _handle_eval(barrier, state, defs)
    elif cmd == "::uplevel":
        _handle_uplevel(barrier, state, defs)
    else:
        state.mark_pessimistic(
            Barrier(BarrierKind.UNKNOWN, detail=f"barrier: {cmd or barrier.command!r}")
        )


def _handle_call(call: IRCall, state: _CfgState, defs: dict[str, int]) -> None:
    cmd = call.canonical_command or call.command
    bare = cmd[2:] if cmd.startswith("::") else cmd
    if not _is_frameless_runtime(bare):
        if not bare or _is_dynamic_token(bare):
            state.record_fallback()
        else:
            state.record_call_fallback()

    if _has_expand_word(call) and bare not in ("list", "concat"):
        state.mark_pessimistic(Barrier(BarrierKind.EXPAND, detail=f"{{*}} in {bare}"))
        return

    if cmd == "::upvar":
        _handle_upvar(call, state, defs)
    elif cmd == "::global":
        _handle_global(call, state, defs)
    elif cmd == "::variable":
        _handle_variable(call, state, defs)
    elif cmd == "::namespace":
        _handle_namespace_call(call, state, defs)
    elif cmd == "::info":
        _handle_info(call, state, defs)
    elif bare in _NAME_FIRST_COMMANDS:
        _handle_dynamic_name_first(call, state, defs)

    # Record the raw written form for the interprocedural resolver: a
    # bare ``leaf`` from inside ``::ns`` must remain bare here so
    # ``_name_candidates`` can walk up the caller's namespace and resolve
    # to ``::ns::leaf`` rather than locking onto the global ``::leaf``.
    raw = call.command
    if raw and not _is_dynamic_token(raw):
        state.record_callee(raw)


def _handle_statement(
    ssa_stmt: SSAStatement,
    state: _CfgState,
) -> None:
    stmt = ssa_stmt.statement
    defs = ssa_stmt.defs
    state.remember_versions(ssa_stmt)

    from ..ir import IRBlock as _IRBlock
    from ..ir import IRUpFrame

    if isinstance(stmt, IRCall):
        _handle_call(stmt, state, defs)
    elif isinstance(stmt, IRBarrier):
        _handle_barrier(stmt, state, defs)
    elif isinstance(stmt, IRUpFrame):
        _handle_barrier(_synthesise_uplevel_barrier(stmt), state, defs)
    elif isinstance(stmt, _IRBlock) and _is_eval_block(stmt):
        _handle_barrier(_synthesise_eval_barrier(stmt), state, defs)
    elif isinstance(stmt, IRAssignConst):
        if _is_dynamic_name(stmt.name):
            literal = state.resolve_literal(stmt.name)
            if literal is not None:
                state.escape(
                    literal,
                    defs,
                    EscapeReason(
                        kind=EscapeReasonKind.DYN_NAME_RESOLVED,
                        detail=f"set {stmt.name} → {literal}",
                    ),
                )
            else:
                state.escape_all_known(
                    defs,
                    EscapeReason(
                        kind=EscapeReasonKind.ESCAPE_ALL_KNOWN,
                        detail=f"set {stmt.name} ...",
                    ),
                )
        else:
            state.note_literal_assign(stmt.name, stmt.value)
        _apply_value_scan(stmt.value, state, defs)
    elif isinstance(stmt, IRAssignValue):
        if _is_dynamic_name(stmt.name):
            literal = state.resolve_literal(stmt.name)
            if literal is not None:
                state.escape(
                    literal,
                    defs,
                    EscapeReason(
                        kind=EscapeReasonKind.DYN_NAME_RESOLVED,
                        detail=f"set {stmt.name} → {literal}",
                    ),
                )
            else:
                state.escape_all_known(
                    defs,
                    EscapeReason(
                        kind=EscapeReasonKind.ESCAPE_ALL_KNOWN,
                        detail=f"set {stmt.name} ...",
                    ),
                )
        else:
            if stmt.value and not _is_dynamic_token(stmt.value):
                state.note_literal_assign(stmt.name, stmt.value)
            else:
                state.invalidate_literal(stmt.name)
        _apply_value_scan(stmt.value, state, defs)
    elif isinstance(stmt, IRAssignExpr):
        if _is_dynamic_name(stmt.name):
            literal = state.resolve_literal(stmt.name)
            if literal is not None:
                state.escape(
                    literal,
                    defs,
                    EscapeReason(
                        kind=EscapeReasonKind.DYN_NAME_RESOLVED,
                        detail=f"set-expr {stmt.name} → {literal}",
                    ),
                )
            else:
                state.escape_all_known(
                    defs,
                    EscapeReason(
                        kind=EscapeReasonKind.ESCAPE_ALL_KNOWN,
                        detail=f"set-expr {stmt.name} ...",
                    ),
                )
        else:
            state.invalidate_literal(stmt.name)
        _apply_expr_scan(stmt.expr, state, defs)
    elif isinstance(stmt, IRIncr):
        if _is_dynamic_name(stmt.name):
            literal = state.resolve_literal(stmt.name)
            if literal is not None:
                state.escape(
                    literal,
                    defs,
                    EscapeReason(
                        kind=EscapeReasonKind.DYN_NAME_RESOLVED,
                        detail=f"incr {stmt.name} → {literal}",
                    ),
                )
            else:
                state.escape_all_known(
                    defs,
                    EscapeReason(
                        kind=EscapeReasonKind.ESCAPE_ALL_KNOWN,
                        detail=f"incr {stmt.name}",
                    ),
                )
        else:
            state.invalidate_literal(stmt.name)
        if stmt.amount:
            _apply_value_scan(stmt.amount, state, defs)


def _walk_block(
    block: SSABlock,
    state: _CfgState,
    terminator_condition: object | None,
) -> None:
    """Process every SSA statement in the block.

    Phi nodes don't introduce escape by themselves — the join is
    handled automatically by the per-SSA-version tags plus the
    per-name collapse at the end.

    ``terminator_condition`` is the expression from a ``CFGBranch``
    terminator; escape-relevant substitutions (for example,
    ``[info exists x]`` inside an ``if`` condition) get scanned so
    they don't slip past the per-statement walk.
    """
    for stmt in block.statements:
        if state.dynamic_barrier:
            return
        _handle_statement(stmt, state)
    if terminator_condition is not None:
        # The terminator's condition doesn't live in any statement's
        # ``defs``, so we have no SSA version to tag.  Pass an empty
        # defs map — the escape helper falls back to the latest seen
        # version for any named var.
        _apply_expr_scan(terminator_condition, state, {})


def _block_order(cfg: CFGFunction) -> list[str]:
    """Return a reverse-postorder (approx.) block ordering for traversal.

    The escape analysis is monotone so any traversal order produces
    the same final tags; RPO gives tests a deterministic walk order
    when a developer wants to eyeball intermediate state.
    """
    order: list[str] = []
    visited: set[str] = set()

    def _visit(name: str) -> None:
        if name in visited or name not in cfg.blocks:
            return
        visited.add(name)
        term = cfg.blocks[name].terminator
        if term is None:
            order.append(name)
            return
        succ: tuple[str, ...] = ()
        from ..cfg import CFGGoto

        if isinstance(term, CFGGoto):
            succ = (term.target,)
        elif isinstance(term, CFGBranch):
            succ = (term.true_target, term.false_target)
        for s in succ:
            _visit(s)
        order.append(name)

    _visit(cfg.entry)
    order.reverse()
    return order


def analyse_cfg_function(
    cfg: CFGFunction,
    ssa: SSAFunction,
    params: Iterable[str] = (),
) -> CfgEscapeResult:
    """Run the flow-sensitive per-SSA-version escape analysis."""
    known = _collect_known_names_from_cfg(params, cfg, ssa)
    state = _CfgState(known_names=known)

    for block_name in _block_order(cfg):
        block = ssa.blocks.get(block_name)
        if block is None:
            continue
        cfg_block = cfg.blocks.get(block_name)
        term_cond: object | None = None
        if cfg_block is not None and isinstance(cfg_block.terminator, CFGBranch):
            term_cond = cfg_block.terminator.condition
        _walk_block(block, state, term_cond)

    # Collapse per-SSA-version tags to per-name tags.
    name_tags: dict[str, EscapeTag] = {}
    for (name, _version), tag in state.ssa_tags.items():
        if tag is EscapeTag.FRAME:
            name_tags[name] = EscapeTag.FRAME

    return CfgEscapeResult(
        ssa_tags=dict(state.ssa_tags),
        name_tags=name_tags,
        dynamic_barrier=state.dynamic_barrier,
        upvar_source_names=set(state.upvar_source_names),
        unbounded_upvar_source=state.unbounded_upvar_source,
        direct_callees=set(state.direct_callees),
        has_fallback=state.has_fallback,
        has_call_fallback=state.has_call_fallback,
        known_names=state.known_names,
        barriers=tuple(state.barriers),
        tag_reasons={k: tuple(v) for k, v in state.tag_reasons.items()},
    )
