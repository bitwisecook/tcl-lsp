"""Intra-procedural taint propagation over the SSA graph."""

from __future__ import annotations

from collections.abc import Callable

from compiler.parsing.green_tree import tokenise
from compiler.registry import REGISTRY
from compiler.registry.runtime import (
    canonical_list_commands,
    taint_double_encode_map,
    taint_transform_map,
)
from compiler.registry.taint_hints import TaintColour
from shared.naming import normalise_var_name as _normalise_var_name
from shared.tokens import TokenType

from ..cfg import CFGBranch, CFGFunction, CFGGoto
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRIncr,
)
from ..ssa import SSAFunction, SSAValueKey, value_use_blocks
from ..value_shapes import is_pure_var_ref, parse_command_substitution
from ._lattice import (
    _TAINTED,
    _UNTAINTED,
    TaintLattice,
    _is_sanitiser,
    _taint_source_colour,
    taint_join,
)

_CallReturnProvider = Callable[
    [str, tuple[str, ...], tuple[TaintLattice, ...], str | None],
    TaintLattice | None,
]


def _join_all_uses(
    ssa_stmt,
    taints: dict[SSAValueKey, TaintLattice],
) -> TaintLattice:
    """Join taint from all used variables in a statement."""
    return _join_uses_map(ssa_stmt.uses, taints)


def _taint_for_var_use(
    name: str,
    uses: dict[str, int],
    taints: dict[SSAValueKey, TaintLattice],
) -> TaintLattice:
    ver = uses.get(name, 0)
    if ver > 0:
        return taints.get((name, ver), _UNTAINTED)
    if ver == 0:
        return taints.get((name, 0), _UNTAINTED)
    return _UNTAINTED


def _join_uses_map(
    uses: dict[str, int],
    taints: dict[SSAValueKey, TaintLattice],
) -> TaintLattice:
    result = _UNTAINTED
    for name, ver in uses.items():
        if ver > 0:
            result = taint_join(result, taints.get((name, ver), _UNTAINTED))
        elif ver == 0 and (name, 0) in taints:
            result = taint_join(result, taints[(name, 0)])
    return result


def _with_extra_colours(
    taint: TaintLattice,
    extra: TaintColour,
) -> TaintLattice:
    """Add *extra* colour bits to a tainted lattice value."""
    if not taint.tainted or extra == TaintColour(0):
        return taint
    return TaintLattice.of(taint.colour | extra)


def _derive_transform_colours(
    command: str,
    args: tuple[str, ...],
    arg_taints: tuple[TaintLattice, ...],
) -> TaintColour:
    """Return conservative derived colours for known transformation commands."""
    if not any(t.tainted for t in arg_taints):
        return TaintColour(0)

    # Tcl list builders produce canonical list representations.
    if command in canonical_list_commands():
        return TaintColour.LIST_CANONICAL
    if (
        REGISTRY.is_produces_canonical_list(command)
        and arg_taints
        and all(t.tainted and bool(t.colour & TaintColour.LIST_CANONICAL) for t in arg_taints)
    ):
        return TaintColour.LIST_CANONICAL

    # Registry-driven transform lookup (sanitisers, encoders, path normalisers).
    transforms = taint_transform_map()
    # Check "cmd sub" compound form first (e.g. "file normalize").
    if args:
        compound = f"{command} {args[0]}"
        colour = transforms.get(compound)
        if colour is not None:
            return colour
    # Then bare command name.
    colour = transforms.get(command)
    if colour is not None:
        return colour

    return TaintColour(0)


_COLOUR_LABELS: dict[TaintColour, str] = {
    TaintColour.URL_ENCODED: "URL-encoded",
    TaintColour.HTML_ESCAPED: "HTML-escaped",
    TaintColour.REGEX_LITERAL: "regex-escaped",
}


def _detect_double_encoding(
    command: str,
    arg_taints: tuple[TaintLattice, ...],
) -> TaintColour | None:
    """Return the redundant colour if *command* re-encodes already-encoded data.

    Returns None when no double-encoding is detected.
    """
    colour = taint_double_encode_map().get(command)
    if colour is None:
        return None
    # Check whether *any* tainted argument already carries this colour.
    for t in arg_taints:
        if t.tainted and bool(t.colour & colour):
            return colour
    return None


def _evaluate_command_subst_taint(
    command_text: str,
    uses: dict[str, int],
    taints: dict[SSAValueKey, TaintLattice],
    *,
    caller_qname: str | None = None,
    call_return_provider: _CallReturnProvider | None = None,
) -> TaintLattice:
    """Evaluate taint for a single ``[...]`` command substitution."""
    parsed = parse_command_substitution(f"[{command_text}]")
    if parsed is None:
        return _UNTAINTED

    cmd_name, cmd_args = parsed
    if _is_sanitiser(cmd_name, cmd_args):
        return _UNTAINTED

    source_taint = _taint_source_colour(cmd_name, cmd_args)
    if source_taint is not None:
        return source_taint

    arg_taints = tuple(
        _evaluate_word_taint(
            arg,
            uses,
            taints,
            caller_qname=caller_qname,
            call_return_provider=call_return_provider,
        )
        for arg in cmd_args
    )
    if call_return_provider is not None:
        from_summary = call_return_provider(
            cmd_name,
            cmd_args,
            arg_taints,
            caller_qname,
        )
        if from_summary is not None:
            return _with_extra_colours(
                from_summary,
                _derive_transform_colours(cmd_name, cmd_args, arg_taints),
            )

    result = _UNTAINTED
    for arg_taint in arg_taints:
        result = taint_join(result, arg_taint)
    return _with_extra_colours(
        result,
        _derive_transform_colours(cmd_name, cmd_args, arg_taints),
    )


def _leading_literal_prefix_char(value: str) -> str | None:
    """Return the leading literal char of *value* or ``None`` for dynamic start.

    ESC tokens are rendered via ``backslash_subst()`` so that escape
    sequences like ``\\x2f`` (``/``) are correctly resolved.
    """
    from shared.tcl_subst import backslash_subst as _bss

    _it = iter(tokenise(value, 0, 0, 0)[0])
    while True:
        tok = next(_it, None)
        if tok is None or tok.type is TokenType.EOL:
            return None
        if tok.type is TokenType.ESC:
            rendered = _bss(tok.text) if "\\" in tok.text else tok.text
            if rendered:
                return rendered[0]
            continue
        if tok.type is TokenType.STR:
            if tok.text:
                return tok.text[0]
            continue
        if tok.type in (TokenType.VAR, TokenType.CMD):
            return None


def _literal_contains_crlf(value: str) -> bool:
    """Return True when any rendered literal fragment in *value* contains CR/LF.

    ESC tokens are rendered via ``backslash_subst()`` so that escape
    sequences like ``\\n`` are correctly resolved to actual newlines.
    """
    from shared.tcl_subst import backslash_subst as _bss

    _it = iter(tokenise(value, 0, 0, 0)[0])
    while True:
        tok = next(_it, None)
        if tok is None or tok.type is TokenType.EOL:
            return False
        if tok.type is TokenType.ESC:
            rendered = _bss(tok.text) if "\\" in tok.text else tok.text
            if "\r" in rendered or "\n" in rendered:
                return True
        elif tok.type is TokenType.STR:
            if "\r" in tok.text or "\n" in tok.text:
                return True


def _evaluate_interpolated_word_taint(
    value: str,
    uses: dict[str, int],
    taints: dict[SSAValueKey, TaintLattice],
    *,
    caller_qname: str | None = None,
    call_return_provider: _CallReturnProvider | None = None,
) -> TaintLattice:
    """Evaluate taint for words that contain interpolation/concatenation."""
    result = _UNTAINTED
    _it = iter(tokenise(value, 0, 0, 0)[0])
    while True:
        tok = next(_it, None)
        if tok is None or tok.type is TokenType.EOL:
            break
        if tok.type is TokenType.VAR:
            var_name = _normalise_var_name(tok.text)
            if var_name:
                result = taint_join(result, _taint_for_var_use(var_name, uses, taints))
        elif tok.type is TokenType.CMD:
            result = taint_join(
                result,
                _evaluate_command_subst_taint(
                    tok.text,
                    uses,
                    taints,
                    caller_qname=caller_qname,
                    call_return_provider=call_return_provider,
                ),
            )

    if not result.tainted:
        return _UNTAINTED

    colour = result.colour

    # Interpolation/concatenation invalidates structural guarantees unless
    # explicitly re-established below.
    colour &= ~(
        TaintColour.LIST_CANONICAL
        | TaintColour.PATH_NORMALISED
        | TaintColour.PATH_BOUNDED
        | TaintColour.HEADER_TOKEN_SAFE
        | TaintColour.HTML_ESCAPED
        | TaintColour.URL_ENCODED
        | TaintColour.REGEX_LITERAL
        | TaintColour.SHELL_ATOM
    )
    if _literal_contains_crlf(value):
        colour &= ~TaintColour.CRLF_FREE

    # Leading literal controls option-prefix safety.
    lead = _leading_literal_prefix_char(value)
    if lead == "/":
        colour |= TaintColour.PATH_PREFIXED | TaintColour.NON_DASH_PREFIXED
    elif lead is not None and lead != "-":
        colour |= TaintColour.NON_DASH_PREFIXED
        colour &= ~TaintColour.PATH_PREFIXED
    elif lead == "-":
        colour &= ~(TaintColour.NON_DASH_PREFIXED | TaintColour.PATH_PREFIXED)

    return TaintLattice.of(colour | TaintColour.TAINTED)


def _word_uses_from_versions(
    text: str,
    versions: dict[str, int],
) -> dict[str, int]:
    uses: dict[str, int] = {}
    _it = iter(tokenise(text, 0, 0, 0)[0])
    while True:
        tok = next(_it, None)
        if tok is None:
            break
        if tok.type is not TokenType.VAR:
            continue
        name = _normalise_var_name(tok.text)
        if not name:
            continue
        uses[name] = versions.get(name, 0)
    return uses


def _evaluate_word_taint(
    value: str,
    uses: dict[str, int],
    taints: dict[SSAValueKey, TaintLattice],
    *,
    caller_qname: str | None = None,
    call_return_provider: _CallReturnProvider | None = None,
) -> TaintLattice:
    stripped = value.strip()
    if is_pure_var_ref(stripped):
        var_name = _normalise_var_name(stripped)
        return _taint_for_var_use(var_name, uses, taints)

    parsed = parse_command_substitution(stripped)
    if parsed is not None:
        cmd_name, cmd_args = parsed
        if _is_sanitiser(cmd_name, cmd_args):
            return _UNTAINTED
        source_taint = _taint_source_colour(cmd_name, cmd_args)
        if source_taint is not None:
            return source_taint
        arg_taints = tuple(
            _evaluate_word_taint(
                arg,
                uses,
                taints,
                caller_qname=caller_qname,
                call_return_provider=call_return_provider,
            )
            for arg in cmd_args
        )
        if call_return_provider is not None:
            from_summary = call_return_provider(
                cmd_name,
                cmd_args,
                arg_taints,
                caller_qname,
            )
            if from_summary is not None:
                return _with_extra_colours(
                    from_summary,
                    _derive_transform_colours(cmd_name, cmd_args, arg_taints),
                )
        # Join arg taints (not top-level uses) — variables inside the
        # command substitution are captured in arg_taints, matching the
        # logic in _evaluate_command_subst_taint.
        result = _UNTAINTED
        for arg_taint in arg_taints:
            result = taint_join(result, arg_taint)
        return _with_extra_colours(
            result,
            _derive_transform_colours(cmd_name, cmd_args, arg_taints),
        )

    if "$" in value or "[" in value:
        return _evaluate_interpolated_word_taint(
            value,
            uses,
            taints,
            caller_qname=caller_qname,
            call_return_provider=call_return_provider,
        )
    return _UNTAINTED


def _evaluate_taint_def(
    stmt,
    ssa_stmt,
    taints: dict[SSAValueKey, TaintLattice],
    *,
    caller_qname: str | None = None,
    call_return_provider: _CallReturnProvider | None = None,
) -> TaintLattice:
    """Determine taint of a variable definition."""
    match stmt:
        case IRAssignConst():
            return _UNTAINTED

        case IRAssignExpr():
            return _join_all_uses(ssa_stmt, taints)

        case IRAssignValue(value=value):
            return _evaluate_word_taint(
                value,
                ssa_stmt.uses,
                taints,
                caller_qname=caller_qname,
                call_return_provider=call_return_provider,
            )

        case IRIncr():
            return _join_all_uses(ssa_stmt, taints)

        case IRCall(command=cmd, args=call_args) if stmt.defs:
            if _is_sanitiser(cmd, call_args):
                return _UNTAINTED
            source_taint = _taint_source_colour(cmd, call_args)
            if source_taint is not None:
                return source_taint
            arg_taints = tuple(
                _evaluate_word_taint(
                    arg,
                    ssa_stmt.uses,
                    taints,
                    caller_qname=caller_qname,
                    call_return_provider=call_return_provider,
                )
                for arg in call_args
            )
            if call_return_provider is not None:
                from_summary = call_return_provider(
                    cmd,
                    call_args,
                    arg_taints,
                    caller_qname,
                )
                if from_summary is not None:
                    return from_summary
            # Propagate taint from arguments.
            return _join_all_uses(ssa_stmt, taints)

        case _:
            return _UNTAINTED


def taint_propagation(
    cfg: CFGFunction,
    ssa: SSAFunction,
    executable_blocks: set[str],
    executable_edges: set[tuple[str, str]],
    *,
    param_taints: dict[str, TaintLattice] | None = None,
    call_return_provider: _CallReturnProvider | None = None,
) -> dict[SSAValueKey, TaintLattice]:
    """Run taint propagation over the SSA graph.

    Same fixed-point loop structure as ``_type_propagation``.
    """
    preds: dict[str, set[str]] = {bn: set() for bn in cfg.blocks}
    for bn, block in cfg.blocks.items():
        match block.terminator:
            case CFGGoto(target=target):
                succs = (target,)
            case CFGBranch(true_target=tt, false_target=ft):
                succs = (tt, ft)
            case _:
                succs = ()
        for succ in succs:
            if succ in preds:
                preds[succ].add(bn)

    taints: dict[SSAValueKey, TaintLattice] = {}
    if param_taints:
        for name, t in param_taints.items():
            if t.tainted:
                taints[(name, 0)] = t

    # Alias groups: names that share the same underlying storage within this
    # frame (``upvar 1 c a; upvar 1 c b`` makes ``a`` and ``b`` the same cell).
    # Taint written through one name must be observable through the others, so
    # we close the taint map over each group on every fixpoint pass.  This is
    # flow-insensitive across the group (a member is tainted if *any* member is
    # ever tainted) — the conservative, security-safe direction.  Only groups
    # with two or more distinct names matter; a lone ``global g`` shares its
    # name with itself and already flows through ordinary SSA.
    alias_groups: list[frozenset[str]] = []
    try:
        from ..memory_ssa import compute_aliases

        for aset in compute_aliases(ssa):
            names = aset.names
            if len(names) >= 2:
                alias_groups.append(names)
    except Exception:
        alias_groups = []

    # Every (name, version) an aliased member takes anywhere in the SSA — defs,
    # uses, phis, and block exit versions.  A read site may reference a version
    # produced by the ``upvar`` declaration itself (e.g. ``b#1``), so seeding
    # only version 0 is not enough; the closure broadcasts to all of these.
    aliased_member_names: set[str] = set()
    for group in alias_groups:
        aliased_member_names |= group
    member_versions: dict[str, set[int]] = {n: {0} for n in aliased_member_names}
    if aliased_member_names:
        for block in ssa.blocks.values():
            for phi in block.phis:
                if phi.name in member_versions:
                    member_versions[phi.name].add(phi.version)
            for name, ver in getattr(block, "exit_versions", {}).items():
                if name in member_versions:
                    member_versions[name].add(ver)
            for s in block.statements:
                for name, ver in s.defs.items():
                    if name in member_versions:
                        member_versions[name].add(ver)
                for name, ver in s.uses.items():
                    if name in member_versions:
                        member_versions[name].add(ver)

    # Forward dataflow over the shared reverse-postorder.
    order = cfg.reverse_postorder()

    # Forward dataflow worklist: a value's change re-enqueues only the
    # blocks that read it (precomputed def→use map) instead of re-scanning
    # every block each pass.  The alias closure stays a whole-function
    # step run between worklist drains until both reach a fixpoint.
    deps = value_use_blocks(ssa)
    changed_keys: list[SSAValueKey] = []

    def set_taint(key: SSAValueKey, candidate: TaintLattice) -> bool:
        old = taints.get(key, _UNTAINTED)
        merged = taint_join(old, candidate)
        if merged != old:
            taints[key] = merged
            changed_keys.append(key)
            return True
        return False

    worklist: list[str] = [bn for bn in order if bn in executable_blocks]
    queued: set[str] = set(worklist)

    def enqueue_readers() -> None:
        for key in changed_keys:
            for ub in deps.get(key, ()):
                if ub in executable_blocks and ub not in queued:
                    worklist.append(ub)
                    queued.add(ub)

    work_pending = True
    while work_pending:
        while worklist:
            bn = worklist.pop()
            queued.discard(bn)
            ssa_block = ssa.blocks.get(bn)
            if ssa_block is None:
                continue
            changed_keys.clear()

            # Phi nodes
            incoming_exec_preds = [p for p in preds.get(bn, set()) if (p, bn) in executable_edges]
            for phi in ssa_block.phis:
                if bn == cfg.entry:
                    continue
                if not incoming_exec_preds:
                    continue
                phi_taint = _UNTAINTED
                for pred in incoming_exec_preds:
                    incoming_ver = phi.incoming.get(pred, 0)
                    if incoming_ver <= 0:
                        continue
                    phi_taint = taint_join(
                        phi_taint,
                        taints.get((phi.name, incoming_ver), _UNTAINTED),
                    )
                set_taint((phi.name, phi.version), phi_taint)

            # Statements
            for s in ssa_block.statements:
                stmt = s.statement
                if isinstance(stmt, IRBarrier):
                    # Barriers conservatively taint all defs.
                    for var, ver in s.defs.items():
                        set_taint((var, ver), _TAINTED)
                    continue
                for var, ver in s.defs.items():
                    inferred = _evaluate_taint_def(
                        stmt,
                        s,
                        taints,
                        caller_qname=cfg.name,
                        call_return_provider=call_return_provider,
                    )
                    set_taint((var, ver), inferred)

            enqueue_readers()

        # Alias closure: share taint across names that alias the same
        # storage.  Run once the per-block worklist drains; if it taints
        # new values, re-enqueue their readers and drain again.
        work_pending = False
        if alias_groups:
            changed_keys.clear()
            for group in alias_groups:
                group_taint = _UNTAINTED
                for (name, _ver), t in taints.items():
                    if name in group and t.tainted:
                        group_taint = taint_join(group_taint, t)
                if not group_taint.tainted:
                    continue
                # Broadcast the group taint to every version each member takes,
                # so a read through any alias name (incl. the version produced
                # by its own ``upvar`` declaration) observes it.
                for name in group:
                    for ver in member_versions.get(name, (0,)):
                        set_taint((name, ver), group_taint)
            if changed_keys:
                # The closure changed taint — re-enqueue any readers and
                # loop so the worklist drains and the closure runs again
                # (it reads every taint, so overlapping groups may still
                # be propagating).
                enqueue_readers()
                work_pending = True

    return taints
