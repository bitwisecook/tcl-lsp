"""Intra-procedural taint propagation over the SSA graph."""

from __future__ import annotations

from collections.abc import Callable

from ...commands.registry.taint_hints import TaintColour
from ...common.naming import normalise_var_name as _normalise_var_name
from ...parsing.lexer import TclLexer
from ...parsing.tokens import TokenType
from ..cfg import CFGBranch, CFGFunction, CFGGoto
from ..ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRIncr,
)
from ..ssa import SSAFunction, SSAValueKey
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
    if command == "list":
        return TaintColour.LIST_CANONICAL
    if (
        command == "concat"
        and arg_taints
        and all(t.tainted and bool(t.colour & TaintColour.LIST_CANONICAL) for t in arg_taints)
    ):
        return TaintColour.LIST_CANONICAL

    # URI/HTML encoders are contextual sanitisers.
    if command in {"URI::encode", "URI::encode_component", "URI::escape"}:
        return TaintColour.URL_ENCODED | TaintColour.CRLF_FREE
    if command in {"HTML::encode", "htmlencode", "html_escape", "html_encode"}:
        return TaintColour.HTML_ESCAPED | TaintColour.CRLF_FREE

    # Path canonicalisation.
    if command == "file" and args and args[0] == "normalize":
        return TaintColour.PATH_NORMALISED

    # Conservative regex-escape wrappers (project/custom helper names).
    if command in {"regex::quote", "regexp::quote", "re_quote", "regex_quote"}:
        return TaintColour.REGEX_LITERAL

    return TaintColour(0)


# Mapping: command → colour that indicates data has already been encoded.
# Used by T106 (double-encoding) detection.
_DOUBLE_ENCODE_MAP: dict[str, TaintColour] = {
    "URI::encode": TaintColour.URL_ENCODED,
    "URI::encode_component": TaintColour.URL_ENCODED,
    "URI::escape": TaintColour.URL_ENCODED,
    "HTML::encode": TaintColour.HTML_ESCAPED,
    "htmlencode": TaintColour.HTML_ESCAPED,
    "html_escape": TaintColour.HTML_ESCAPED,
    "html_encode": TaintColour.HTML_ESCAPED,
    "regex::quote": TaintColour.REGEX_LITERAL,
    "regexp::quote": TaintColour.REGEX_LITERAL,
    "re_quote": TaintColour.REGEX_LITERAL,
    "regex_quote": TaintColour.REGEX_LITERAL,
}

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
    colour = _DOUBLE_ENCODE_MAP.get(command)
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
    """Return the leading literal char of *value* or ``None`` for dynamic start."""
    lexer = TclLexer(value)
    while True:
        tok = lexer.get_token()
        if tok is None or tok.type is TokenType.EOL:
            return None
        if tok.type in (TokenType.ESC, TokenType.STR):
            if tok.text:
                return tok.text[0]
            continue
        if tok.type in (TokenType.VAR, TokenType.CMD):
            return None


def _literal_contains_crlf(value: str) -> bool:
    """Return True when any literal fragment in *value* contains CR/LF."""
    lexer = TclLexer(value)
    while True:
        tok = lexer.get_token()
        if tok is None or tok.type is TokenType.EOL:
            return False
        if tok.type in (TokenType.ESC, TokenType.STR):
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
    lexer = TclLexer(value)
    while True:
        tok = lexer.get_token()
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
    lexer = TclLexer(text)
    while True:
        tok = lexer.get_token()
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

    # Compute traversal order (DFS over CFG, same algorithm as core_analyses).
    _seen: set[str] = set()
    order: list[str] = []
    _stack = [cfg.entry]
    while _stack:
        _bn = _stack.pop()
        if _bn in _seen or _bn not in cfg.blocks:
            continue
        _seen.add(_bn)
        order.append(_bn)
        match cfg.blocks[_bn].terminator:
            case CFGGoto(target=_t):
                _s_list = [_t]
            case CFGBranch(true_target=_tt, false_target=_ft):
                _s_list = [_tt, _ft]
            case _:
                _s_list = []
        for _s in reversed(_s_list):
            _stack.append(_s)

    def set_taint(key: SSAValueKey, candidate: TaintLattice) -> bool:
        old = taints.get(key, _UNTAINTED)
        merged = taint_join(old, candidate)
        if merged != old:
            taints[key] = merged
            return True
        return False

    changed = True
    while changed:
        changed = False
        for bn in order:
            if bn not in executable_blocks:
                continue
            ssa_block = ssa.blocks.get(bn)
            if ssa_block is None:
                continue

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
                if set_taint((phi.name, phi.version), phi_taint):
                    changed = True

            # Statements
            for s in ssa_block.statements:
                stmt = s.statement
                if isinstance(stmt, IRBarrier):
                    # Barriers conservatively taint all defs.
                    for var, ver in s.defs.items():
                        if set_taint((var, ver), _TAINTED):
                            changed = True
                    continue
                for var, ver in s.defs.items():
                    inferred = _evaluate_taint_def(
                        stmt,
                        s,
                        taints,
                        caller_qname=cfg.name,
                        call_return_provider=call_return_provider,
                    )
                    if set_taint((var, ver), inferred):
                        changed = True

    return taints
