"""Rendered Value Properties -- string content analysis over SSA.

Computes rendered string content properties for every SSA value after
Tcl backslash substitution.  This pass runs after SCCP and before
taint propagation so that downstream consumers can query properties
like "does this value contain a forward slash after escape rendering?"
without reimplementing lexer traversal and escape logic.

The analysis uses two property groups with different join semantics:

* **may** properties (union at phi joins): overapproximate.  If *any*
  incoming edge has ``HAS_FORWARD_SLASH``, the merged value has it.
* **must** properties (intersection at phi joins): underapproximate.
  ``STARTS_WITH_SLASH`` only survives if *all* incoming edges agree.

This follows the standard abstract interpretation design of using a
reduced product of simple boolean domains (cf. Costantini, Ferrara &
Cortesi 2011, "A Suite of Abstract Domains for Static Analysis of
String Values"; Amadini et al. 2017, SAFE/SAFEstr).
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Flag, auto

from compiler.parsing.lexer import TclLexer
from compiler.parsing.substitution import backslash_subst
from compiler.parsing.tokens import TokenType

from ..commands.registry import REGISTRY
from .cfg import CFGBranch, CFGFunction, CFGGoto
from .ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRIncr,
)
from .ssa import SSAFunction, SSAStatement, SSAValueKey
from .value_shapes import is_pure_var_ref, parse_command_substitution


class RenderedProperties(Flag):
    """Properties of a rendered (post-backslash-subst) SSA value.

    Each bit represents a "may-have" or "must-have" property of the
    string after Tcl escape processing.
    """

    NONE = 0

    # --- "may" properties (union at joins) ---
    HAS_FORWARD_SLASH = auto()  # rendered literal text contains '/'
    HAS_BACKSLASH = auto()  # rendered literal text contains '\\' (path sep)
    HAS_CRLF = auto()  # rendered literal text contains \\r or \\n
    HAS_INTERPOLATION = auto()  # value contains $var or [cmd]
    HAS_DOUBLE_ESCAPE = auto()  # rendered text contains already-escaped sequences
    HAS_NULL = auto()  # rendered text contains \\x00 / \\0
    WAS_UNESCAPED = auto()  # value passed through subst / URI::decode / b64decode etc.
    DOUBLE_UNESCAPED = auto()  # value was already WAS_UNESCAPED then unescaped again
    FULLY_NORMALISED = auto()  # value fully canonical — no residual encoding (e.g. -normalized)

    # --- "must" properties (intersection at joins) ---
    STARTS_WITH_SLASH = auto()  # first rendered literal char is '/'
    STARTS_WITH_DASH = auto()  # first rendered literal char is '-'


# Masks for the two property groups.
_MAY_MASK = (
    RenderedProperties.HAS_FORWARD_SLASH
    | RenderedProperties.HAS_BACKSLASH
    | RenderedProperties.HAS_CRLF
    | RenderedProperties.HAS_INTERPOLATION
    | RenderedProperties.HAS_DOUBLE_ESCAPE
    | RenderedProperties.HAS_NULL
)
# Provenance bits are NOT in _MAY_MASK — they are only set explicitly
# by unescape commands and propagated through copies/phis.
_PROVENANCE_MASK = (
    RenderedProperties.WAS_UNESCAPED
    | RenderedProperties.DOUBLE_UNESCAPED
    | RenderedProperties.FULLY_NORMALISED
)
_MUST_MASK = RenderedProperties.STARTS_WITH_SLASH | RenderedProperties.STARTS_WITH_DASH
_ALL_MUST = _MUST_MASK  # bottom: assume all must-props until disproven


@dataclass(frozen=True, slots=True)
class RenderedValueProps:
    """Rendered string content properties for a single SSA value.

    Attributes:
        may: Properties that *may* hold (union at joins).
        must: Properties that *must* hold (intersection at joins).
    """

    may: RenderedProperties = RenderedProperties.NONE
    must: RenderedProperties = _ALL_MUST


_BOTTOM = RenderedValueProps(may=RenderedProperties.NONE, must=_ALL_MUST)
_TOP = RenderedValueProps(may=_MAY_MASK, must=RenderedProperties.NONE)


def rendered_join(a: RenderedValueProps, b: RenderedValueProps) -> RenderedValueProps:
    """Join two rendered-property lattice values.

    * ``may`` bits use **union** (overapproximate).
    * ``must`` bits use **intersection** (underapproximate).
    """
    return RenderedValueProps(may=a.may | b.may, must=a.must & b.must)


def _is_unescape_command(cmd: str, args: tuple[str, ...] | list[str]) -> bool:
    """Return True if the command performs partial unescaping / decoding.

    Does NOT include -normalized getters — those produce fully canonical
    values with no residual encoding (safe, like ``file normalize``).
    """
    if REGISTRY.is_unescape_command(cmd):
        return True
    unescape_subs = REGISTRY.unescape_subcommands(cmd)
    if unescape_subs is not None and args and args[0] in unescape_subs:
        return True
    return False


def _is_normalised_getter(cmd: str, args: tuple[str, ...] | list[str]) -> bool:
    """Return True if the command returns a fully normalised (no encoding) value."""
    return REGISTRY.is_unnormalized_http_getter(cmd) and "-normalized" in args


def _is_path_returning(cmd: str, args: tuple[str, ...] | list[str]) -> bool:
    """Return True if the command returns filesystem path content."""
    if REGISTRY.is_path_returning(cmd):
        return True
    path_subs = REGISTRY.path_returning_subcommands(cmd)
    if path_subs is not None and args and args[0] in path_subs:
        return True
    return False


def _cmd_may_return_path(cmd_text: str) -> bool:
    """Return True if a command substitution is known to return path content."""
    parsed = parse_command_substitution(f"[{cmd_text}]")
    if parsed is None:
        return False
    cmd, args = parsed
    return _is_path_returning(cmd, args)


def _render_esc_text(text: str) -> str:
    """Render escape sequences in an ESC token's text.

    Only applies ``backslash_subst`` when the text actually contains a
    backslash, avoiding unnecessary work for the common case.
    """
    return backslash_subst(text) if "\\" in text else text


def _has_double_escape(rendered: str) -> bool:
    """Detect double-escaping: rendered text still contains escape sequences.

    If after rendering ``\\n`` -> newline, the result still contains
    ``\\n`` (literal backslash-n), the value was double-escaped.
    """
    if "\\" not in rendered:
        return False
    # After rendering, any remaining backslash followed by a known
    # escape character suggests double-escaping.
    for i in range(len(rendered) - 1):
        if rendered[i] == "\\" and rendered[i + 1] in 'abfnrtv\\{}[]$"; xuU01234567':
            return True
    return False


def _evaluate_rendered_props_for_value(value: str) -> RenderedValueProps:
    """Compute rendered properties for an ``IRAssignValue`` expression.

    Lexes the value string, renders ESC tokens via ``backslash_subst()``,
    and accumulates properties from literal and dynamic tokens.
    """
    stripped = value.strip()

    # Pure variable reference: unknown content -> top-like for may, no must.
    if is_pure_var_ref(stripped):
        return RenderedValueProps(
            may=RenderedProperties.HAS_INTERPOLATION,
            must=RenderedProperties.NONE,
        )

    # Pure command substitution: opaque result.
    parsed = parse_command_substitution(stripped)
    if parsed is not None:
        may = RenderedProperties.HAS_INTERPOLATION
        must = RenderedProperties.NONE
        cmd, args = parsed
        # If the command returns path content, set HAS_FORWARD_SLASH.
        if _is_path_returning(cmd, args):
            may |= RenderedProperties.HAS_FORWARD_SLASH
        # Track unescape provenance for subst, URI::decode, b64decode,
        # encoding convertfrom.
        if _is_unescape_command(cmd, args):
            may |= RenderedProperties.WAS_UNESCAPED
        # -normalized getters produce fully canonical values (no residual
        # encoding).  Still tainted (HTTP input) but encoding-safe.
        if _is_normalised_getter(cmd, args):
            may |= RenderedProperties.FULLY_NORMALISED
        return RenderedValueProps(may=may, must=must)

    may = RenderedProperties.NONE
    must = _ALL_MUST
    leading_resolved = False  # have we seen the first content token?

    lexer = TclLexer(stripped)
    while True:
        tok = lexer.get_token()
        if tok is None or tok.type in (TokenType.EOL, TokenType.EOF):
            break

        if tok.type is TokenType.SEP:
            continue

        if tok.type is TokenType.VAR:
            may |= RenderedProperties.HAS_INTERPOLATION
            if not leading_resolved:
                # Variable at the start: can't guarantee STARTS_WITH_*.
                must = RenderedProperties.NONE
                leading_resolved = True
            continue

        if tok.type is TokenType.CMD:
            may |= RenderedProperties.HAS_INTERPOLATION
            if _cmd_may_return_path(tok.text):
                may |= RenderedProperties.HAS_FORWARD_SLASH
            if not leading_resolved:
                must = RenderedProperties.NONE
                leading_resolved = True
            continue

        # ESC or STR -- literal text.
        is_esc = tok.type is TokenType.ESC
        raw_text = tok.text
        rendered = _render_esc_text(raw_text) if is_esc else raw_text

        if "/" in rendered:
            may |= RenderedProperties.HAS_FORWARD_SLASH
        if "\\" in rendered:
            may |= RenderedProperties.HAS_BACKSLASH
        if "\r" in rendered or "\n" in rendered:
            may |= RenderedProperties.HAS_CRLF
        if "\x00" in rendered:
            may |= RenderedProperties.HAS_NULL
        if is_esc and _has_double_escape(rendered):
            may |= RenderedProperties.HAS_DOUBLE_ESCAPE

        if not leading_resolved and rendered:
            first_char = rendered[0]
            new_must = RenderedProperties.NONE
            if first_char == "/":
                new_must |= RenderedProperties.STARTS_WITH_SLASH
            if first_char == "-":
                new_must |= RenderedProperties.STARTS_WITH_DASH
            must &= new_must | ~_MUST_MASK
            leading_resolved = True

    if not leading_resolved:
        # Empty value: no must-properties.
        must = RenderedProperties.NONE

    return RenderedValueProps(may=may, must=must)


def _evaluate_rendered_props_for_const(value: str) -> RenderedValueProps:
    """Compute rendered properties for an ``IRAssignConst`` literal."""
    may = RenderedProperties.NONE
    must = RenderedProperties.NONE

    if "/" in value:
        may |= RenderedProperties.HAS_FORWARD_SLASH
    if "\\" in value:
        may |= RenderedProperties.HAS_BACKSLASH
    if "\r" in value or "\n" in value:
        may |= RenderedProperties.HAS_CRLF
    if "\x00" in value:
        may |= RenderedProperties.HAS_NULL

    if value:
        if value[0] == "/":
            must |= RenderedProperties.STARTS_WITH_SLASH
        if value[0] == "-":
            must |= RenderedProperties.STARTS_WITH_DASH

    return RenderedValueProps(may=may, must=must)


def _evaluate_rendered_def(
    stmt: object,
    ssa_stmt: SSAStatement,
    props: dict[SSAValueKey, RenderedValueProps],
) -> RenderedValueProps:
    """Determine rendered properties of a variable definition.

    For pure variable references (``set x $y``), the properties of the
    source SSA value are propagated via the ``props`` dict and the SSA
    ``uses`` map, enabling copy propagation through the lattice.
    """
    match stmt:
        case IRAssignConst(value=value):
            return _evaluate_rendered_props_for_const(
                value if isinstance(value, str) else str(value)
            )

        case IRAssignValue(value=value):
            # Copy propagation: if the value is a pure variable ref,
            # inherit the source variable's rendered properties.
            stripped = value.strip()
            if is_pure_var_ref(stripped) and ssa_stmt.uses:
                for use_name, use_ver in ssa_stmt.uses.items():
                    src = props.get((use_name, use_ver))
                    if src is not None:
                        return src
            base = _evaluate_rendered_props_for_value(value)
            # Escalate to DOUBLE_UNESCAPED: if the value itself is an
            # unescape command (WAS_UNESCAPED) and any SSA input was
            # already WAS_UNESCAPED, the result is doubly unescaped.
            # Exception: FULLY_NORMALISED inputs have no residual encoding,
            # so decoding them again is harmless (not a double-unescape).
            if base.may & RenderedProperties.WAS_UNESCAPED and ssa_stmt.uses:
                for use_name, use_ver in ssa_stmt.uses.items():
                    src = props.get((use_name, use_ver))
                    if (
                        src is not None
                        and src.may & RenderedProperties.WAS_UNESCAPED
                        and not (src.may & RenderedProperties.FULLY_NORMALISED)
                    ):
                        return RenderedValueProps(
                            may=base.may | RenderedProperties.DOUBLE_UNESCAPED,
                            must=base.must,
                        )
            return base

        case IRAssignExpr():
            # Expression results are numeric -- no path separators.
            return RenderedValueProps(
                may=RenderedProperties.NONE,
                must=RenderedProperties.NONE,
            )

        case IRIncr():
            # Always produces an integer.
            return RenderedValueProps(
                may=RenderedProperties.NONE,
                must=RenderedProperties.NONE,
            )

        case IRCall(command=cmd, args=args):
            # -normalized getters: fully canonical, no residual encoding.
            if _is_normalised_getter(cmd, args):
                return RenderedValueProps(
                    may=_MAY_MASK | RenderedProperties.FULLY_NORMALISED,
                    must=RenderedProperties.NONE,
                )
            # Track unescape provenance across SSA.
            if _is_unescape_command(cmd, args):
                input_may = RenderedProperties.NONE
                for _use_name, _use_ver in ssa_stmt.uses.items():
                    src = props.get((_use_name, _use_ver))
                    if src is not None:
                        input_may |= src.may
                result_may = _MAY_MASK | RenderedProperties.WAS_UNESCAPED
                # Escalate unless input was FULLY_NORMALISED (nothing left
                # to decode, so the second decode is harmless).
                if input_may & RenderedProperties.WAS_UNESCAPED and not (
                    input_may & RenderedProperties.FULLY_NORMALISED
                ):
                    result_may |= RenderedProperties.DOUBLE_UNESCAPED
                return RenderedValueProps(may=result_may, must=RenderedProperties.NONE)
            # Generic command: conservative.
            return _TOP

        case _:
            return _TOP


def rendered_properties_propagation(
    cfg: CFGFunction,
    ssa: SSAFunction,
    executable_blocks: set[str],
    executable_edges: set[tuple[str, str]],
) -> dict[SSAValueKey, RenderedValueProps]:
    """Run rendered-value-properties propagation over the SSA graph.

    Follows the same fixed-point loop structure as ``_type_propagation``
    in ``core_analyses.py``.
    """
    # Build predecessor map.
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

    props: dict[SSAValueKey, RenderedValueProps] = {}

    # DFS traversal order.
    seen: set[str] = set()
    order: list[str] = []
    stack = [cfg.entry]
    while stack:
        _bn = stack.pop()
        if _bn in seen or _bn not in cfg.blocks:
            continue
        seen.add(_bn)
        order.append(_bn)
        match cfg.blocks[_bn].terminator:
            case CFGGoto(target=_t):
                _s_list = [_t]
            case CFGBranch(true_target=_tt, false_target=_ft):
                _s_list = [_tt, _ft]
            case _:
                _s_list = []
        for _s in reversed(_s_list):
            stack.append(_s)

    def set_props(key: SSAValueKey, candidate: RenderedValueProps) -> bool:
        old = props.get(key, _BOTTOM)
        merged = rendered_join(old, candidate)
        if merged != old:
            props[key] = merged
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

            # Phi nodes.
            incoming_exec_preds = [p for p in preds.get(bn, set()) if (p, bn) in executable_edges]
            for phi in ssa_block.phis:
                if bn == cfg.entry:
                    continue
                if not incoming_exec_preds:
                    continue
                phi_props = _BOTTOM
                for pred in incoming_exec_preds:
                    incoming_ver = phi.incoming.get(pred, 0)
                    if incoming_ver <= 0:
                        continue
                    phi_props = rendered_join(
                        phi_props,
                        props.get((phi.name, incoming_ver), _BOTTOM),
                    )
                if set_props((phi.name, phi.version), phi_props):
                    changed = True

            # Statements.
            for s in ssa_block.statements:
                stmt = s.statement
                if isinstance(stmt, IRBarrier):
                    for var, ver in s.defs.items():
                        if set_props((var, ver), _TOP):
                            changed = True
                    continue
                for var, ver in s.defs.items():
                    inferred = _evaluate_rendered_def(stmt, s, props)
                    if set_props((var, ver), inferred):
                        changed = True

    return props
