"""Core dataflow analyses over CFG + SSA.

This module runs the main analysis passes after SSA construction:

- **SCCP** (Sparse Conditional Constant Propagation): propagates
  integer/boolean constants through the SSA graph using a four-point
  lattice per variable version:

    - ``UNKNOWN`` – not yet analysed (bottom).
    - ``CONST(v)`` – provably the constant *v*.
    - ``CONSTSET(vs)`` – provably one of a finite set of constants.
    - ``OVERDEFINED`` – may hold more than one value (top).

  Values flow upward through the lattice; once a variable reaches
  OVERDEFINED it never narrows.  Branch conditions are evaluated
  against the lattice so unreachable paths are never explored.

- **Liveness**: backward dataflow computing which SSA values are
  live-in / live-out at each CFG block.

- **Dead store detection**: any ``(name, version)`` that is defined
  but never appears in any use set or phi incoming edge is dead.

- **Constant branch detection**: branches whose condition is fully
  determined by SCCP constants.

The public entry point is ``analyse_function`` / ``analyse_source``.
"""

# canonicalisation: audited #246

from __future__ import annotations

import math
import re
import time
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import TYPE_CHECKING

from compiler.parsing.command_segmenter import segment_commands
from compiler.parsing.lexer import TclLexer
from compiler.registry.runtime import (
    FOLD_HINTS,
    FOLD_SUBCOMMAND_HINTS,
    TYPE_HINTS,
    scope_alias_commands,
)
from compiler.registry.type_hints import CommandTypeHint, SubcommandTypeHint
from shared.naming import is_unqualified_var_name as _is_unqualified_var_name
from shared.naming import normalise_qualified_name as _normalise_qualified_name
from shared.naming import normalise_var_name as _normalise_var_name
from shared.naming import split_array_name
from shared.tokens import TokenType

from .cfg import CFGBranch, CFGFunction, CFGGoto, CFGReturn, build_cfg
from .eval_helpers import DECIMAL_INT_RE as _DECIMAL_INT_RE
from .expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprLiteral,
    ExprNode,
    ExprRaw,
    ExprString,
    ExprTernary,
    ExprUnary,
    UnaryOp,
    command_texts_in_expr_node,
    expr_text,
    vars_in_expr_node,
)
from .expr_types import infer_expr_type
from .ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRBlock,
    IRCall,
    IRExprEval,
    IRIncr,
    IRModule,
    IRReturn,
    IRStatement,
)
from .lowering import lower_to_ir
from .place import LOCAL_NS, Place, PlaceKind, overlap
from .place_bridge import (
    build_resolve_context,
    def_places,
    read_places,
    terminator_read_places,
)
from .ssa import (
    _COMPLEXITY_GUARD_BLOCKS,
    SSAFunction,
    SSAStatement,
    SSAValueKey,
    build_ssa,
    dynamic_target_name_reads,
    expr_substitution_read_names,
    is_complexity_guarded,  # noqa: F401  (re-exported for diagnostic-pass guards)
    statement_cmd_sub_write_names,
    statement_read_names,
    value_use_blocks,
)
from .static_loops import (
    evaluate_expr_with_constants,
    summarise_static_for_ir,
)
from .tcl_constants import TCL_BOOL_LITERALS as _BOOL_LITERALS
from .tcl_expr_eval import _split_tcl_list, eval_tcl_expr
from .types import TclType, TypeLattice, type_join
from .value_shapes import is_pure_var_ref
from .var_refs import VarReferenceScanner, body_write_names, command_sub_write_names

if TYPE_CHECKING:
    from .def_use import DefUseResult
    from .memory_ssa import MemorySSAFunction
    from .rendered_properties import RenderedValueProps
    from .taint import TaintLattice

_RETURN_VAR_SCANNER = VarReferenceScanner()


def _parse_cmd_subst(value: str) -> tuple[str, str] | None:
    """If *value* is exactly a ``[cmd args...]`` command substitution, return
    ``(command_name, raw_args_text)``; otherwise ``None``.

    Token-based replacement for the command-substitution regexes: the value
    must lex to a single command-substitution word, and the inner script is
    segmented so the command name and raw argument span come from the tokens.
    """
    v = value.strip()
    if not (v.startswith("[") and v.endswith("]")):
        return None
    lexer = TclLexer(v)
    tok = lexer.get_token()
    if tok is None or tok.type is not TokenType.CMD:
        return None
    nxt = lexer.get_token()
    while nxt is not None and nxt.type in (TokenType.EOL, TokenType.SEP):
        nxt = lexer.get_token()
    if nxt is not None:
        return None
    inner = v[1:-1]
    commands = segment_commands(inner)
    if len(commands) != 1 or not commands[0].texts:
        return None
    cmd = commands[0]
    args_text = inner[cmd.argv[1].start.offset :].rstrip() if len(cmd.argv) >= 2 else ""
    return cmd.texts[0], args_text


def _parse_safe_unset_ref(cmd_text: str) -> str | None:
    """Parse a command substitution that *references* a variable but does NOT
    require it to be set, and return the raw target name (else ``None``).

    Currently covers ``[vwait varName]`` (blocks waiting for the variable to be
    written; ``vwait forever`` is the canonical infinite-wait idiom — the whole
    point is that the variable is initially unset).  ``[info exists]`` /
    ``[array exists]`` are handled by :func:`_parse_existence_check`.

    Note: ``array get|names|size FOO`` also returns ``""``/``0`` cleanly on
    unset ``FOO`` (tclsh 9.0.3-verified), but the project deliberately keeps
    W210 firing there as a linting choice (the developer wrote
    ``array get FOO`` presumably wanting contents — silent empty is the
    classic masking-a-bug shape), pinned by
    ``test_array_get_is_a_value_read_still_flagged``.  vwait is different:
    its entire purpose is to wait for an as-yet-unset variable.
    """
    parsed = _parse_cmd_subst(cmd_text)
    if parsed is None:
        return None
    cmd_name, args_text = parsed
    base = cmd_name[2:] if cmd_name.startswith("::") else cmd_name
    try:
        parts = _split_tcl_list(args_text) if args_text.strip() else []
    except Exception:
        return None
    if base == "vwait" and parts:
        # vwait's options end at the first non-``-`` arg or at ``--``.
        # ``-readable``/``-writable``/``-timeout``/``-variable`` take a value.
        i = 0
        while i < len(parts) and parts[i].startswith("-") and parts[i] != "--":
            if parts[i] in ("-readable", "-writable", "-timeout", "-variable"):
                i += 2
            else:
                i += 1
        if i < len(parts) and parts[i] == "--":
            i += 1
        if i < len(parts):
            return parts[i].strip()
    return None


def _parse_existence_check(cmd_text: str) -> tuple[str, str] | None:
    """Parse ``[info exists X]`` / ``[array exists X]`` into ``(kind, raw_target)``.

    *kind* is ``"info"`` or ``"array"`` and *raw_target* is the **unnormalised**
    argument text (e.g. ``"X"``, ``"A(k)"``, ``"::ns::X"``).  Returns ``None``
    when the text is not an existence check.  Callers that fold or narrow must
    pass *raw_target* through :func:`_existence_scalar_name` so that array
    elements and qualified names are excluded — only a plain local scalar's
    existence is locally decidable.
    """
    parsed = _parse_cmd_subst(cmd_text)
    if parsed is None:
        return None
    cmd_name, args_text = parsed
    base = cmd_name[2:] if cmd_name.startswith("::") else cmd_name
    if base not in ("info", "array"):
        return None
    try:
        parts = _split_tcl_list(args_text) if args_text.strip() else []
    except Exception:
        return None
    if len(parts) != 2 or parts[0] != "exists":
        return None
    return base, parts[1].strip()


def _existence_scalar_name(raw_target: str) -> str | None:
    """Return the normalised name when *raw_target* is a plain local scalar.

    Array elements (``A(k)``), namespace-qualified names (``::ns::X``,
    ``ns::X``), and dynamic targets (``$name``) are not locally decidable and
    yield ``None``.
    """
    if not _is_unqualified_var_name(raw_target):
        return None
    name = _normalise_var_name(raw_target)
    return name or None


def _existence_skip_name(raw_target: str) -> str | None:
    """Like :func:`_existence_scalar_name` but also accepts a literal array
    element, returning its **base** name.

    For *read-before-set* suppression this is sound: ``info exists a(b)`` is a
    valid Tcl existence check that does not require ``a`` to be set
    (tclsh-verified — returns 0 cleanly when ``a`` is unset), so the base ``a``
    must not be flagged W210 there.  A *dynamic* index (``a($k)``) still reads
    ``k`` to form the name and is left reportable.

    Kept separate from the scalar form so the constant-folding / dead-arm path
    (``[info exists a(b)]`` → I230) stays strict: an array-element check is
    NOT locally folded to true/false (the base may or may not be set, and even
    if it is the element may not be — soundness gate
    ``test_array_element_is_not_folded``).
    """
    name = _existence_scalar_name(raw_target)
    if name is not None:
        return name
    base, index = split_array_name(raw_target)
    if base and index is not None and "$" not in index and "[" not in index:
        return _existence_scalar_name(base)
    return None


def _expr_has_command(node: ExprNode) -> bool:
    """Return True if *node* contains any command substitution."""
    match node:
        case ExprCommand():
            return True
        case ExprRaw(text=text):
            return "[" in text
        case ExprBinary(left=left, right=right):
            return _expr_has_command(left) or _expr_has_command(right)
        case ExprUnary(operand=operand):
            return _expr_has_command(operand)
        case ExprTernary(condition=c, true_branch=t, false_branch=f):
            return _expr_has_command(c) or _expr_has_command(t) or _expr_has_command(f)
        case ExprCall(args=args):
            return any(_expr_has_command(a) for a in args)
        case _:
            return False


# Short names: bn = block name (str), s = SSAStatement,
# m = regex Match object, r = Range, p = predecessor block (str).

_COMP_RE = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_:]*)\s*(==|!=|eq|ne|<=|>=|<|>)\s*(.+?)\s*$")


class LatticeKind(Enum):
    UNKNOWN = auto()
    CONST = auto()
    CONSTSET = auto()
    OVERDEFINED = auto()


# Maximum number of elements in a CONSTSET before we widen to OVERDEFINED.
_MAX_CONSTSET_SIZE = 32


@dataclass(frozen=True, slots=True)
class LatticeValue:
    kind: LatticeKind
    value: int | float | bool | str | None = None
    # For CONSTSET: the finite set of possible constant values.
    values: frozenset[int | float | bool | str] | None = None

    @staticmethod
    def unknown() -> "LatticeValue":
        return LatticeValue(LatticeKind.UNKNOWN, None)

    @staticmethod
    def overdefined() -> "LatticeValue":
        return LatticeValue(LatticeKind.OVERDEFINED, None)

    @staticmethod
    def const(value: int | float | bool | str) -> "LatticeValue":
        return LatticeValue(LatticeKind.CONST, value)

    @staticmethod
    def constset(vals: frozenset[int | float | bool | str]) -> "LatticeValue":
        """Create a CONSTSET lattice value from a finite set of constants.

        If the set has exactly one element, returns a CONST instead.
        If the set exceeds ``_MAX_CONSTSET_SIZE``, returns OVERDEFINED.
        """
        if len(vals) == 0:
            return OVERDEFINED
        if len(vals) == 1:
            return LatticeValue.const(next(iter(vals)))
        if len(vals) > _MAX_CONSTSET_SIZE:
            return OVERDEFINED
        return LatticeValue(LatticeKind.CONSTSET, None, vals)


UNKNOWN = LatticeValue.unknown()
OVERDEFINED = LatticeValue.overdefined()


def _to_set(lv: LatticeValue) -> frozenset[int | float | bool | str] | None:
    """Extract the set of possible values from a CONST or CONSTSET."""
    if lv.kind is LatticeKind.CONST and lv.value is not None:
        return frozenset((lv.value,))
    if lv.kind is LatticeKind.CONSTSET and lv.values is not None:
        return lv.values
    return None


def _join(old: LatticeValue, new: LatticeValue) -> LatticeValue:
    if new.kind is LatticeKind.UNKNOWN:
        return old
    if old.kind is LatticeKind.UNKNOWN:
        return new
    if old.kind is LatticeKind.OVERDEFINED or new.kind is LatticeKind.OVERDEFINED:
        return OVERDEFINED
    # Both are CONST or CONSTSET — merge the value sets.
    old_set = _to_set(old)
    new_set = _to_set(new)
    if old_set is not None and new_set is not None:
        merged = old_set | new_set
        if merged == old_set:
            return old
        return LatticeValue.constset(merged)
    # Fallback (should not happen): widen.
    return OVERDEFINED


@dataclass(frozen=True, slots=True)
class ConstantBranch:
    block: str
    condition: str
    value: bool
    taken_target: str
    not_taken_target: str


@dataclass(frozen=True, slots=True)
class DeadStore:
    block: str
    statement_index: int
    variable: str
    version: int


@dataclass(frozen=True, slots=True)
class ReadBeforeSet:
    block: str
    statement_index: int
    variable: str


@dataclass(frozen=True, slots=True)
class UnusedVariable:
    block: str
    statement_index: int
    variable: str


@dataclass(frozen=True, slots=True)
class FunctionAnalysis:
    live_in: dict[str, set[SSAValueKey]]
    live_out: dict[str, set[SSAValueKey]]
    dead_stores: tuple[DeadStore, ...]
    unreachable_blocks: set[str]
    constant_branches: tuple[ConstantBranch, ...]
    values: dict[SSAValueKey, LatticeValue]
    types: dict[SSAValueKey, TypeLattice] = field(default_factory=dict)
    taints: dict[SSAValueKey, "TaintLattice"] = field(default_factory=dict)
    read_before_set: tuple[ReadBeforeSet, ...] = ()
    unused_variables: tuple[UnusedVariable, ...] = ()
    unused_params: tuple[str, ...] = ()
    rendered_props: dict[SSAValueKey, "RenderedValueProps"] = field(default_factory=dict)
    def_use_chains: "DefUseResult | None" = None
    memory_ssa: "MemorySSAFunction | None" = None


@dataclass(frozen=True, slots=True)
class ModuleAnalysis:
    top_level: FunctionAnalysis
    procedures: dict[str, FunctionAnalysis]


def _compute_predecessors(cfg: CFGFunction) -> dict[str, set[str]]:
    """Build a predecessor map from the CFG terminators."""
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
    return preds


def _parse_literal_value(text: str) -> int | str:
    stripped = text.strip()
    if _DECIMAL_INT_RE.fullmatch(stripped):
        try:
            iv = int(stripped)
        except ValueError:
            return stripped
        # Only collapse to int when the textual identity round-trips:
        # ``"010"`` parses as int 10, but ``[expr {$a eq "10"}]`` for
        # ``a == "010"`` must yield 0 (string compare).  Storing it as
        # the canonical int would lose that string identity, and
        # downstream constant-substitution then folds the comparison
        # the wrong way.  ``" 5 "`` and ``+5`` round-trip differently
        # too — keep them as strings so SCCP doesn't change observable
        # behaviour.
        if str(iv) != stripped:
            return stripped
        return iv
    return stripped


def _substitute_expr_with_lattice(
    expr: ExprNode,
    uses: dict[str, int],
    values: dict[SSAValueKey, LatticeValue],
) -> LatticeValue:
    """Evaluate an expression AST with SCCP lattice values as variables."""
    env: dict[str, int | float | str] = {}
    for name, ver in uses.items():
        lv = values.get((name, ver), UNKNOWN)
        if lv.kind is LatticeKind.OVERDEFINED:
            return OVERDEFINED
        if lv.kind is LatticeKind.UNKNOWN:
            return UNKNOWN
        if isinstance(lv.value, bool):
            env[name] = int(lv.value)
        elif isinstance(lv.value, (int, float)):
            env[name] = lv.value
        else:
            return OVERDEFINED

    result = eval_tcl_expr(expr, env)
    if result is None:
        return OVERDEFINED
    # Tcl 9.0 raises ARITH DOMAIN for any expression that evaluates to NaN
    # (verified against tclsh 9.0.3: ``expr {NaN}`` → domain error).  Don't
    # propagate NaN as a constant — the runtime must be allowed to raise.
    if isinstance(result, float) and math.isnan(result):
        return OVERDEFINED
    return LatticeValue.const(result)


def _fold_interpolation(
    value: str,
    uses: dict[str, int],
    values: dict[SSAValueKey, LatticeValue],
) -> LatticeValue:
    """Constant-fold a Tcl word containing variable substitutions.

    Tokenises *value* with ``TclLexer``.  If every ``$var`` resolves to a
    known constant and there are no command substitutions, the pieces are
    concatenated and returned as a **string** constant — matching the Tcl
    runtime representation after interpolation.
    """
    pieces: list[str] = []
    lexer = TclLexer(value)
    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is TokenType.VAR:
            name = _normalise_var_name(tok.text)
            ver = uses.get(name, 0)
            lv = values.get((name, ver), UNKNOWN)
            if lv.kind is LatticeKind.OVERDEFINED:
                return OVERDEFINED
            if lv.kind is LatticeKind.UNKNOWN:
                return UNKNOWN
            pieces.append(str(lv.value))
        elif tok.type is TokenType.CMD:
            # Try folding the nested command substitution.
            cmd_text = f"[{tok.text}]"
            folded_cmd = _try_fold_cmd_subst(cmd_text, uses, values)
            if folded_cmd is not None and folded_cmd.kind is LatticeKind.CONST:
                pieces.append(str(folded_cmd.value))
            else:
                return OVERDEFINED
        else:
            pieces.append(tok.text)
    result = "".join(pieces)
    return LatticeValue.const(result)


def _fold_interpolation_set(
    value: str,
    uses: dict[str, int],
    values: dict[SSAValueKey, LatticeValue],
) -> frozenset[str] | None:
    """Resolve a Tcl word with variable substitutions to a set of strings.

    Like ``_fold_interpolation`` but handles CONSTSET variables by computing
    the Cartesian product of all possible interpolated strings.

    Returns ``None`` if any variable is unresolvable (OVERDEFINED/UNKNOWN).
    """
    # Each element is either a literal string or a set of possible values.
    segments: list[list[str]] = []
    lexer = TclLexer(value)
    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if tok.type is TokenType.VAR:
            name = _normalise_var_name(tok.text)
            ver = uses.get(name, 0)
            lv = values.get((name, ver), UNKNOWN)
            vs = _to_set(lv)
            if vs is None:
                return None  # OVERDEFINED or UNKNOWN
            segments.append([str(v) for v in vs])
        elif tok.type is TokenType.CMD:
            return None
        else:
            segments.append([tok.text])

    if not segments:
        return None

    # Compute Cartesian product, bounded by _MAX_CONSTSET_SIZE.
    current: list[str] = [""]
    for seg in segments:
        next_round: list[str] = []
        for prefix in current:
            for piece in seg:
                next_round.append(prefix + piece)
                if len(next_round) > _MAX_CONSTSET_SIZE:
                    return None  # too many combinations
        current = next_round
    return frozenset(current) if current else None


def _extract_foreach_elements(list_text: str) -> list[str] | None:
    """Extract constant list elements from a foreach list argument.

    Handles three patterns:
    - Braced literal list: ``{a b c}``
    - ``[list ...]`` command with all literal arguments: ``[list a b c]``
    - Bare literal list (braces already stripped by lowering): ``a b c``

    Returns ``None`` if the list cannot be statically resolved.
    """
    stripped = list_text.strip()
    if not stripped:
        return None

    # Pattern 1: braced literal list {a b c}
    if stripped.startswith("{") and stripped.endswith("}"):
        inner = stripped[1:-1].strip()
        if not inner:
            return None
        # Must not contain variable or command substitutions.
        if "$" in inner or "[" in inner:
            return None
        return _split_tcl_list(inner)

    # Pattern 2: [list elem1 elem2 ...] with all literal args
    parsed = _parse_cmd_subst(stripped)
    if parsed is not None and parsed[0] == "list" and parsed[1]:
        args_text = parsed[1]
        if "$" in args_text or "[" in args_text:
            return None
        return _split_tcl_list(args_text)

    # Pattern 3: bare literal list (braces stripped by IR lowering)
    # Must not contain variable or command substitutions.
    if "$" not in stripped and "[" not in stripped:
        return _split_tcl_list(stripped)

    return None


def _resolve_foreach_list_via_lattice(
    list_text: str,
    uses: dict[str, int],
    values: dict[SSAValueKey, LatticeValue],
) -> list[str] | None:
    """Resolve a foreach list argument through the SCCP lattice.

    Handles cases like ``foreach x $mylist`` where ``mylist`` has a known
    constant value, and ``foreach x "a $sep b"`` where interpolation
    produces a known constant string.
    """
    stripped = list_text.strip()
    if not stripped:
        return None

    # Case 1: pure variable reference — foreach x $mylist
    if is_pure_var_ref(stripped):
        name = _normalise_var_name(stripped)
        ver = uses.get(name, 0)
        lv = values.get((name, ver), UNKNOWN)
        if lv.kind is LatticeKind.CONST and isinstance(lv.value, str):
            return _split_tcl_list(lv.value)
        return None

    # Case 2: interpolated string — foreach x "prefix_${v}_suffix"
    # Only if it contains variable substitutions (no command subs).
    if "$" in stripped and "[" not in stripped:
        resolved = _fold_interpolation(stripped, uses, values)
        if resolved.kind is LatticeKind.CONST and isinstance(resolved.value, str):
            return _split_tcl_list(resolved.value)

    return None


def _try_fold_cmd_subst(
    value: str,
    uses: dict[str, int],
    values: dict[SSAValueKey, LatticeValue],
) -> LatticeValue | None:
    """Try to constant-fold a ``[cmd args...]`` command substitution.

    Uses registry-based fold callbacks from ``FOLD_HINTS`` /
    ``FOLD_SUBCOMMAND_HINTS``.  Returns a ``LatticeValue`` if the command
    is foldable with all-constant arguments, or ``None`` if not.
    """
    parsed = _parse_cmd_subst(value)
    if parsed is None:
        return None

    cmd_name, args_text = parsed

    # Look up fold callback — check subcommand hints first.
    fold_fn = FOLD_HINTS.get(cmd_name)
    subcmd_folds = FOLD_SUBCOMMAND_HINTS.get(cmd_name)
    if fold_fn is None and subcmd_folds is None:
        return None

    # For subcommand-based commands, extract the subcommand name.
    if subcmd_folds is not None and fold_fn is None:
        parts = args_text.split(None, 1)
        if not parts:
            return None
        sub_name = parts[0]
        fold_fn = subcmd_folds.get(sub_name)
        if fold_fn is None:
            return None
        args_text = parts[1] if len(parts) > 1 else ""

    # Narrow Optional for the type checker: every reachable path above
    # either returned or assigned a non-None fold_fn.
    assert fold_fn is not None

    # Split into individual arguments first (respecting braces/quotes),
    # then resolve variable references in each arg individually.
    try:
        arg_list = _split_tcl_list(args_text) if args_text.strip() else []
    except Exception:
        return None

    # Resolve variable references in each argument.
    resolved_args: list[str] = []
    for arg in arg_list:
        if "[" in arg:
            return None  # nested command substitution — can't fold
        if "$" in arg:
            folded = _fold_interpolation(arg, uses, values)
            if folded.kind is not LatticeKind.CONST:
                return None
            resolved_args.append(str(folded.value))
        else:
            resolved_args.append(arg)

    # Call the fold callback.
    try:
        result = fold_fn(tuple(resolved_args))
    except Exception:
        return None

    if result is None:
        return None
    # Reject results containing Tcl characters that would change
    # semantics in command substitution or quoting contexts.
    # Note: spaces are allowed — they're valid in SCCP constant values
    # (e.g. [list a b] → "a b") and the optimiser handles quoting.
    # We only reject characters that break command parsing.
    if any(ch in ';\n[$"\\' for ch in result):
        return None
    return LatticeValue.const(_parse_literal_value(result))


def _evaluate_def(
    stmt: IRStatement,
    ssa_stmt: SSAStatement,
    values: dict[SSAValueKey, LatticeValue],
) -> LatticeValue:
    match stmt:
        case IRAssignConst(value=value):
            return LatticeValue.const(_parse_literal_value(value))

        case IRAssignExpr(expr=expr):
            return _substitute_expr_with_lattice(expr, ssa_stmt.uses, values)

        case IRAssignValue(value=value):
            if not ssa_stmt.uses:
                # Only treat as constant if the value doesn't contain
                # command substitutions (which have runtime results).
                if "[" not in value:
                    return LatticeValue.const(_parse_literal_value(value))
                # Try registry-based constant folding for [cmd args...].
                folded = _try_fold_cmd_subst(value, {}, {})
                if folded is not None:
                    return folded
                return OVERDEFINED
            # Try registry-based constant folding with variable resolution.
            if "[" in value:
                folded = _try_fold_cmd_subst(value, ssa_stmt.uses, values)
                if folded is not None:
                    return folded
            stripped = value.strip()
            if is_pure_var_ref(stripped):
                name = _normalise_var_name(stripped)
                ver = ssa_stmt.uses.get(name, 0)
                return values.get((name, ver), UNKNOWN)
            if any(
                values.get((n, v), UNKNOWN).kind is LatticeKind.OVERDEFINED
                for n, v in ssa_stmt.uses.items()
            ):
                return OVERDEFINED
            return _fold_interpolation(value, ssa_stmt.uses, values)

        case IRIncr(name=raw_name, amount=amount_text):
            name = _normalise_var_name(raw_name)
            base_ver = ssa_stmt.uses.get(name, 0)
            base = values.get((name, base_ver), UNKNOWN)
            if base.kind is LatticeKind.OVERDEFINED:
                return OVERDEFINED
            if base.kind is LatticeKind.UNKNOWN:
                return UNKNOWN
            if not isinstance(base.value, int):
                return OVERDEFINED

            if amount_text is None:
                amount = 1
            else:
                stripped = amount_text.strip()
                if _DECIMAL_INT_RE.fullmatch(stripped):
                    amount = int(stripped)
                else:
                    amount = 0
                    matched = False
                    for used_name, used_ver in ssa_stmt.uses.items():
                        if used_name == name:
                            continue
                        if stripped in (f"${used_name}", f"${{{used_name}}}"):
                            lv = values.get((used_name, used_ver), UNKNOWN)
                            if lv.kind is LatticeKind.UNKNOWN:
                                return UNKNOWN
                            if lv.kind is LatticeKind.OVERDEFINED:
                                return OVERDEFINED
                            if isinstance(lv.value, int):
                                amount = lv.value
                            elif isinstance(lv.value, str) and _DECIMAL_INT_RE.fullmatch(lv.value):
                                amount = int(lv.value)
                            else:
                                return OVERDEFINED
                            matched = True
                            break
                    if not matched:
                        return OVERDEFINED

            return LatticeValue.const(base.value + amount)

        case IRCall(command=cmd, args=args, defs=defs) if (
            cmd
            in (
                "foreach",
                "lmap",
            )
            and len(defs) == 1
            and len(args) == 1
        ):
            # foreach x {a b c} or foreach x [list a b c] — extract the
            # set of constant values the iteration variable can take.
            elements = _extract_foreach_elements(args[0])
            if elements is None:
                # Try resolving variable references in the list arg
                # through the SCCP lattice (e.g. foreach x $mylist).
                elements = _resolve_foreach_list_via_lattice(
                    args[0],
                    ssa_stmt.uses,
                    values,
                )
            if elements is not None and len(elements) > 0:
                vals = frozenset(_parse_literal_value(e) for e in elements)
                return LatticeValue.constset(vals)
            return OVERDEFINED

        case _:
            return OVERDEFINED


def _condition_use_versions(condition: ExprNode, exit_versions: dict[str, int]) -> dict[str, int]:
    uses: dict[str, int] = {}
    for name in vars_in_expr_node(condition):
        ver = exit_versions.get(name, 0)
        if ver > 0:
            uses[name] = ver
    if not uses:
        text = expr_text(condition)
        m = _COMP_RE.match(text)
        if m:
            lhs = m.group(1)
            ver = exit_versions.get(lhs, 0)
            if ver > 0:
                uses[lhs] = ver
    return uses


def _evaluate_condition(
    condition: ExprNode,
    uses: dict[str, int],
    values: dict[SSAValueKey, LatticeValue],
) -> bool | None:
    if not uses:
        result = eval_tcl_expr(condition)
        if result is None:
            return None
        return bool(result)

    lv = _substitute_expr_with_lattice(condition, uses, values)
    if lv.kind is LatticeKind.CONST:
        if isinstance(lv.value, bool):
            return lv.value
        if isinstance(lv.value, (int, float)):
            return lv.value != 0

    # Fallback for string-valued constants: regex-based comparison
    cond_text = expr_text(condition)
    m = _COMP_RE.match(cond_text)
    if m and len(uses) == 1:
        lhs = m.group(1)
        op = m.group(2)
        rhs_text = m.group(3).strip()
        lhs_ver = uses.get(lhs, 0)
        lhs_val = values.get((lhs, lhs_ver), UNKNOWN)
        if lhs_val.kind is not LatticeKind.CONST:
            return None
        rhs_val = _parse_literal_value(rhs_text)
        lv_val = lhs_val.value
        if op in ("==", "eq"):
            return lv_val == rhs_val
        if op in ("!=", "ne"):
            return lv_val != rhs_val
        if not isinstance(lv_val, int) or not isinstance(rhs_val, int):
            return None
        if op == "<":
            return lv_val < rhs_val
        if op == "<=":
            return lv_val <= rhs_val
        if op == ">":
            return lv_val > rhs_val
        if op == ">=":
            return lv_val >= rhs_val
    return None


def _barrier_aware_env_for_block(
    cfg: CFGFunction,
    ssa: SSAFunction,
    block_name: str,
    values: dict[SSAValueKey, LatticeValue],
) -> dict[str, int | float | bool | str] | None:
    block = cfg.blocks.get(block_name)
    ssa_block = ssa.blocks.get(block_name)
    if block is None or ssa_block is None:
        return None

    env: dict[str, int | float | bool | str]
    loop_meta = cfg.loop_nodes.get(block_name)
    if loop_meta is not None:
        loop_start_block, loop_stmt = loop_meta
        loop_start_ssa = ssa.blocks.get(loop_start_block)
        if loop_start_ssa is None:
            return None
        start_env: dict[str, int | float | bool | str] = {}
        for name, ver in loop_start_ssa.exit_versions.items():
            lv = values.get((name, ver), UNKNOWN)
            if lv.kind is LatticeKind.CONST and isinstance(lv.value, (int, bool, str)):
                start_env[name] = lv.value
        summarised = summarise_static_for_ir(loop_stmt, initial_constants=start_env)
        if summarised is None:
            return None
        env = summarised
    else:
        env = {}
        for name, ver in ssa_block.entry_versions.items():
            lv = values.get((name, ver), UNKNOWN)
            if lv.kind is LatticeKind.CONST and isinstance(lv.value, (int, bool, str)):
                env[name] = lv.value
        # Also pick up seeded version-0 constants (e.g. interprocedural
        # parameter constants).  In normal analysis no version-0 values
        # exist in ``values``, so this is a no-op.
        for (vname, ver), lv in values.items():
            if ver == 0 and vname not in env:
                if lv.kind is LatticeKind.CONST and isinstance(lv.value, (int, bool, str)):
                    env[vname] = lv.value

    for idx, stmt in enumerate(block.statements):
        if isinstance(stmt, IRBarrier):
            return None

        if isinstance(stmt, IRCall):
            # Pure commands (e.g. string, list) cannot mutate variables,
            # so we can safely infer through them.  Unknown or impure
            # calls may mutate state through upvar/eval, so bail out.
            from .side_effects import classify_side_effects

            if not classify_side_effects(stmt.command, stmt.args).pure:
                return None

        if idx < len(ssa_block.statements):
            ssa_stmt = ssa_block.statements[idx]
            for name, ver in ssa_stmt.defs.items():
                lv = values.get((name, ver), UNKNOWN)
                if lv.kind is LatticeKind.CONST and isinstance(lv.value, (int, bool, str)):
                    env[name] = lv.value
                else:
                    env.pop(name, None)

    return env


def _is_bounded_scope_decl(stmt: IRStatement) -> bool:
    """True when *stmt* only declares its named ``global``/``variable``/``upvar``
    target(s) — a bounded effect captured by :func:`_escaping_var_names`, not an
    arbitrary local creation."""
    if not isinstance(stmt, (IRCall, IRBarrier)):
        return False
    from compiler.var_scoping import (
        global_declaration_indices,
        upvar_local_declaration_indices,
        variable_declaration_indices,
    )

    args = stmt.args
    if upvar_local_declaration_indices(stmt.command, args):
        return True
    if stmt.canonical_command == "::global" and global_declaration_indices(args):
        return True
    if stmt.canonical_command == "::variable" and variable_declaration_indices(args):
        return True
    return False


def _command_mutates_locals(cmd_text: str) -> bool:
    """True when the ``[cmd ...]`` substitution could create, modify, or remove
    a local variable.

    Such effects are invisible to SSA when the command is nested inside a
    value/expression (``set y [set X 1]``) rather than a top-level statement, so
    the existence folder must treat statements containing them as opaque.
    """
    parsed = _parse_cmd_subst(cmd_text)
    if parsed is None:
        return True  # multi-command / unparseable — assume it can
    cmd_name, args_text = parsed
    base = cmd_name[2:] if cmd_name.startswith("::") else cmd_name
    try:
        args = tuple(_split_command_args(args_text))
    except Exception:
        return True
    # ``unset`` / ``array unset`` change a variable's existence without a write
    # target that side-effect analysis records.
    if base == "unset" or (base == "array" and args and args[0] == "unset"):
        return True
    from compiler.registry.runtime import ArgRole, arg_indices_for_role
    from compiler.side_effects import SideEffectTarget, classify_side_effects

    # A command that runs an inline body could create locals (``[if {…} {set X …}]``).
    if arg_indices_for_role(cmd_name, list(args), ArgRole.BODY):
        return True
    se = classify_side_effects(cmd_name, args)
    if se.dynamic_barrier:
        return True
    return (
        SideEffectTarget.VARIABLE in se.write_targets
        or SideEffectTarget.UNKNOWN in se.write_targets
    )


def _word_mutation_free(text: str) -> bool:
    """True when evaluating word/script *text* cannot create/modify/remove a
    local — i.e. no command substitution within it does so."""
    if "[" not in text:
        return True
    try:
        tokens = TclLexer(text).tokenise_all()
    except Exception:
        return False
    for tok in tokens:
        if tok.type is TokenType.CMD:
            if _command_mutates_locals(f"[{tok.text}]"):
                return False
            if not _word_mutation_free(tok.text):  # nested substitutions
                return False
    return True


def _expr_mutation_free(node: ExprNode) -> bool:
    """True when evaluating expression *node* cannot create/modify/remove a local."""
    match node:
        case ExprCommand(text=text) | ExprRaw(text=text):
            return _word_mutation_free(text)
        case ExprBinary(left=left, right=right):
            return _expr_mutation_free(left) and _expr_mutation_free(right)
        case ExprUnary(operand=operand):
            return _expr_mutation_free(operand)
        case ExprTernary(condition=cond, true_branch=tb, false_branch=fb):
            return _expr_mutation_free(cond) and _expr_mutation_free(tb) and _expr_mutation_free(fb)
        case ExprCall(args=args):
            return all(_expr_mutation_free(a) for a in args)
        case _:
            return True


def _is_existence_transparent(stmt: IRStatement) -> bool:
    """True when *stmt* cannot create or destroy a local variable in a way the
    analysis cannot see — the precondition for folding existence checks.

    A value/expression statement whose value contains a command substitution
    that could create a local (``set y [set X 1]``) is *not* transparent: the
    nested write produces no SSA definition, so the folder would otherwise
    mistake the created variable for an absent one.
    """
    if isinstance(stmt, IRAssignConst):
        return True
    if isinstance(stmt, IRAssignValue):
        return _word_mutation_free(stmt.value)
    if isinstance(stmt, (IRAssignExpr, IRExprEval)):
        return _expr_mutation_free(stmt.expr)
    if isinstance(stmt, IRIncr):
        return stmt.amount is None or _word_mutation_free(stmt.amount)
    if isinstance(stmt, IRReturn):
        if stmt.value is not None and not _word_mutation_free(stmt.value):
            return False
        if stmt.expr is not None and not _expr_mutation_free(stmt.expr):
            return False
        return True
    if isinstance(stmt, IRCall):
        if _is_bounded_scope_decl(stmt):
            return True
        from compiler.registry.runtime import ArgRole, arg_indices_for_role
        from compiler.side_effects import SideEffectTarget, classify_side_effects

        se = classify_side_effects(stmt.command, stmt.args)
        if se.dynamic_barrier or SideEffectTarget.UNKNOWN in se.write_targets:
            return False
        # A command that runs an inline body in this scope (``namespace eval``,
        # ``clientside``, …) could set locals the analysis never sees.
        if arg_indices_for_role(stmt.command, list(stmt.args), ArgRole.BODY):
            return False
        # A nested command substitution in an argument can create a local too
        # (``puts [set X 1]``).
        return all(_word_mutation_free(arg) for arg in stmt.args)
    # IRBarrier, IRBlock, IRUpFrame, IRSwitch, IRCatch, IRTry, IRForeach, … —
    # opaque to local-variable creation.
    return False


def _existence_def_kind(stmt: IRStatement) -> str:
    """Classify the definition a statement produces for existence reasoning:
    ``"real"`` (a genuine assignment → exists), ``"unset"`` (→ absent), or
    ``"unknown"`` (alias/proc-call/array-unset → undecidable)."""
    if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr)):
        return "real"
    if isinstance(stmt, (IRCall, IRBarrier)) and stmt.canonical_command == "::unset":
        return "unset"
    return "unknown"


@dataclass(frozen=True, slots=True)
class _ExistenceFolder:
    """Folds ``[info exists X]`` / ``[array exists X]`` to a constant when the
    variable is provably absent or present in a single function.

    ``foldable`` is the per-function gate: ``False`` whenever any statement
    could create/destroy a local invisibly (dynamic ``eval``/``uplevel``,
    body-executing commands, unknown procs, …), in which case nothing folds.
    """

    foldable: bool
    params: frozenset[str]
    escaping: frozenset[str]
    version_kind: dict[SSAValueKey, str]

    @staticmethod
    def build(cfg: CFGFunction, ssa: SSAFunction, params: frozenset[str]) -> "_ExistenceFolder":
        foldable = all(
            _is_existence_transparent(stmt)
            for block in cfg.blocks.values()
            for stmt in block.statements
        )
        version_kind: dict[SSAValueKey, str] = {}
        if foldable:
            for bn, block in cfg.blocks.items():
                ssa_block = ssa.blocks.get(bn)
                if ssa_block is None:
                    continue
                for idx, ir_stmt in enumerate(block.statements):
                    if idx >= len(ssa_block.statements):
                        continue
                    kind = _existence_def_kind(ir_stmt)
                    for name, ver in ssa_block.statements[idx].defs.items():
                        if ver > 0:
                            version_kind[(name, ver)] = kind
        return _ExistenceFolder(foldable, params, _escaping_var_names(cfg), version_kind)

    def _classify(self, name: str, version: int) -> bool | None:
        """Return ``True`` (exists), ``False`` (absent), or ``None`` (unknown)."""
        if not self.foldable:
            return None
        if version == 0:
            if name in self.params:
                return True
            if name in self.escaping:
                return None
            return False
        kind = self.version_kind.get((name, version))
        if kind == "real":
            return True
        if kind == "unset":
            return False
        return None

    def fold_command(self, cmd_text: str, uses: dict[str, int]) -> str | None:
        parsed = _parse_existence_check(cmd_text)
        if parsed is None:
            return None
        kind, raw_target = parsed
        name = _existence_scalar_name(raw_target)
        if name is None:
            return None
        verdict = self._classify(name, uses.get(name, 0))
        if verdict is None:
            return None
        # A scalar assignment proves ``info exists`` but not ``array exists``.
        if kind == "array" and verdict:
            return None
        return "1" if verdict else "0"

    def fold_condition(self, condition: ExprNode, uses: dict[str, int]) -> ExprNode:
        if not self.foldable:
            return condition
        return _fold_existence_in_expr(condition, uses, self)


def _fold_existence_in_expr(
    expr: ExprNode, uses: dict[str, int], folder: "_ExistenceFolder"
) -> ExprNode:
    """Replace foldable existence-check substitutions in *expr* with ``0``/``1``
    literals.  ``info exists`` has no side effects, so the replacement is exact."""
    if isinstance(expr, ExprCommand):
        verdict = folder.fold_command(expr.text, uses)
        if verdict is not None:
            return ExprLiteral(text=verdict, start=expr.start, end=expr.end)
        return expr
    if isinstance(expr, ExprBinary):
        return ExprBinary(
            expr.op,
            _fold_existence_in_expr(expr.left, uses, folder),
            _fold_existence_in_expr(expr.right, uses, folder),
        )
    if isinstance(expr, ExprUnary):
        return ExprUnary(expr.op, _fold_existence_in_expr(expr.operand, uses, folder))
    if isinstance(expr, ExprTernary):
        return ExprTernary(
            _fold_existence_in_expr(expr.condition, uses, folder),
            _fold_existence_in_expr(expr.true_branch, uses, folder),
            _fold_existence_in_expr(expr.false_branch, uses, folder),
        )
    if isinstance(expr, ExprCall):
        return ExprCall(
            expr.function,
            tuple(_fold_existence_in_expr(a, uses, folder) for a in expr.args),
            expr.start,
            expr.end,
        )
    return expr


def _evaluate_branch_decision(
    cfg: CFGFunction,
    ssa: SSAFunction,
    block_name: str,
    condition: ExprNode,
    values: dict[SSAValueKey, LatticeValue],
    folder: "_ExistenceFolder | None" = None,
) -> bool | None:
    if folder is not None:
        uses0 = _condition_use_versions(condition, ssa.blocks[block_name].exit_versions)
        condition = folder.fold_condition(condition, uses0)
    env = _barrier_aware_env_for_block(cfg, ssa, block_name, values)
    if env is not None:
        result = evaluate_expr_with_constants(expr_text(condition), env)
        if isinstance(result, bool):
            return result
        if isinstance(result, int):
            return result != 0
        return None

    uses = _condition_use_versions(condition, ssa.blocks[block_name].exit_versions)
    return _evaluate_condition(condition, uses, values)


def _sccp(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    param_constants: dict[SSAValueKey, LatticeValue] | None = None,
    params: frozenset[str] = frozenset(),
) -> tuple[
    dict[SSAValueKey, LatticeValue], set[str], set[tuple[str, str]], tuple[ConstantBranch, ...]
]:
    preds = _compute_predecessors(cfg)
    # Fold ``[info exists X]`` / ``[array exists X]`` to a constant when the
    # variable is provably absent or present, so branches guarded by an
    # existence check resolve to a single edge (feeding I230 + DCE).
    existence_folder = _ExistenceFolder.build(cfg, ssa, params)

    # Global / namespace / upvar-aliased / traced variables are shared mutable
    # state observable and writable from other scopes, traces, and source
    # files.  Their value is therefore never a compile-time constant: folding
    # through one would be unsound across any opaque call (``set ::g 5; mut;
    # [expr {$::g + 1}]`` must NOT fold to 6 — ``mut`` may have rewritten
    # ``::g``).  Force every such definition to OVERDEFINED so SCCP never
    # propagates a constant through it; the read is still tracked for liveness.
    _escaping = _escaping_var_names(cfg)

    def _is_externally_mutable(name: str) -> bool:
        return name.startswith("::") or name in _escaping

    executable_blocks: set[str] = {cfg.entry} if cfg.entry in cfg.blocks else set()
    executable_edges: set[tuple[str, str]] = set()
    values: dict[SSAValueKey, LatticeValue] = {}
    # Keys whose current value is not yet OVERDEFINED.  A barrier widens
    # every tracked value to OVERDEFINED; tracking the live set lets each
    # barrier visit touch only the not-yet-widened keys instead of
    # re-scanning the whole ``values`` map on every fixpoint pass.
    non_overdefined: set[SSAValueKey] = set()
    if param_constants:
        for key, lv in param_constants.items():
            values[key] = lv
            if lv.kind is not LatticeKind.OVERDEFINED:
                non_overdefined.add(key)
    order = cfg.reverse_postorder()

    # ``try`` body→handler throw edges (analysis builds): the handler is
    # reachable when the block holding the ``try`` is, even though no CFG
    # terminator points to it.  SSA already gives the handler the pre-``try``
    # versions via these same edges; mirror them in SCCP reachability so the
    # handler body is not falsely "unreachable" (O107).
    exc_targets: dict[str, list[str]] = {}
    for _frm, _handler in cfg.exception_edges:
        exc_targets.setdefault(_frm, []).append(_handler)

    # `break` reachability (analysis-only).  `break`/`continue` are modelled as
    # plain jump *statements* (codegen emits them as jumps), not CFG edges — so
    # a `break` inside a constant-condition loop (`while 1 { … break … }`)
    # leaves the loop-exit block reachable only via the header's exit edge,
    # which SCCP prunes as dead (the condition is always true).  The exit block
    # is then wrongly unreachable (a false O107 + an unsound DCE).  Precompute,
    # per block that contains a `break`, the *innermost* enclosing loop's exit
    # block, and feed that as an extra executable edge in the fixpoint below.
    break_exits: dict[str, str] = {}
    if any(
        isinstance(s.statement, IRCall) and s.statement.canonical_command == "::break"
        for sb in ssa.blocks.values()
        for s in sb.statements
    ):
        from .loops import NaturalLoop, build_loop_forest

        forest = build_loop_forest(cfg, ssa)
        # innermost loop per block = smallest body set containing it.
        innermost: dict[str, NaturalLoop] = {}
        for loop in sorted(forest.loops, key=lambda lp: len(lp.blocks)):
            for b in loop.blocks:
                innermost.setdefault(b, loop)
        for bn2, sb2 in ssa.blocks.items():
            if not any(
                isinstance(s.statement, IRCall) and s.statement.canonical_command == "::break"
                for s in sb2.statements
            ):
                continue
            loop = innermost.get(bn2)
            if loop is None:
                continue
            hdr = cfg.blocks.get(loop.header)
            if hdr is None or not isinstance(hdr.terminator, CFGBranch):
                continue
            t = hdr.terminator
            exit_b = t.false_target if t.false_target not in loop.blocks else t.true_target
            if exit_b in cfg.blocks:
                break_exits[bn2] = exit_b

    def set_value(key: SSAValueKey, candidate: LatticeValue) -> bool:
        old = values.get(key, UNKNOWN)
        merged = _join(old, candidate)
        if merged != old:
            values[key] = merged
            if merged.kind is LatticeKind.OVERDEFINED:
                non_overdefined.discard(key)
            else:
                non_overdefined.add(key)
            return True
        return False

    # Seed live-in roots to OVERDEFINED.  A value that is *used* but never
    # *defined* anywhere in this function (a proc parameter, a global or
    # namespace variable read, an upvar target) holds a runtime-unknown
    # value, so it is ⊥, not ⊤.  Without this seeding such a root defaults
    # to UNKNOWN, and because UNKNOWN is the join identity it silently
    # vanishes from any phi it feeds — e.g. ``join(const 0, $runtime)``
    # would fold to ``0`` and make a genuinely runtime condition look
    # constant (a false-positive "always true/false" branch).
    defined_keys: set[SSAValueKey] = set()
    used_keys: set[SSAValueKey] = set()
    for sblock in ssa.blocks.values():
        for phi in sblock.phis:
            defined_keys.add((phi.name, phi.version))
            for inc in phi.incoming.values():
                if inc > 0:
                    used_keys.add((phi.name, inc))
        for s in sblock.statements:
            for var, ver in s.defs.items():
                defined_keys.add((var, ver))
            used_keys.update(s.uses.items())
    for term_bn, term_block in cfg.blocks.items():
        term = term_block.terminator
        if isinstance(term, CFGBranch):
            sb = ssa.blocks.get(term_bn)
            if sb is not None:
                used_keys.update(_condition_use_versions(term.condition, sb.exit_versions).items())
    for key in used_keys - defined_keys:
        if key not in values:
            values[key] = OVERDEFINED

    def branch_targets(block_name: str, condition: ExprNode, tt: str, ft: str) -> tuple[str, ...]:
        # Optimistic (Wegman–Zadeck) branch resolution: fold to a single
        # edge when the condition is constant; mark BOTH arms only when it
        # is genuinely non-constant (an OVERDEFINED operand, or
        # all-constant but unfoldable).  While any operand is still UNKNOWN
        # the condition may yet fold, so open NO edge and let a later pass
        # retry — this is what detects loop-carried constant conditions
        # instead of pessimistically opening both arms forever.  Soundness
        # relies on every runtime-unknown input already being OVERDEFINED
        # (live-in roots seeded above; undefined phi inputs joined as ⊥
        # below), so a still-UNKNOWN operand genuinely means "not yet
        # computed", never "unknowable".
        decision = _evaluate_branch_decision(
            cfg, ssa, block_name, condition, values, existence_folder
        )
        if decision is True:
            return (tt,)
        if decision is False:
            return (ft,)
        sb = ssa.blocks.get(block_name)
        exit_versions = sb.exit_versions if sb is not None else {}
        op_vals = [
            values.get((n, v), UNKNOWN)
            for n, v in _condition_use_versions(condition, exit_versions).items()
        ]
        if (
            op_vals
            and any(ov.kind is LatticeKind.UNKNOWN for ov in op_vals)
            and not any(ov.kind is LatticeKind.OVERDEFINED for ov in op_vals)
        ):
            return ()
        return (tt, ft)

    # Optimistic fixpoint over the RPO sweep, followed by a finalization
    # pass that forces both arms for any executable branch still stuck on
    # an UNKNOWN condition (defensive: a value defined only in
    # unreachable code could otherwise leave a successor spuriously
    # unreachable).  ``finalizing`` is monotone, so the outer loop runs at
    # most twice.
    finalizing = False
    while True:
        changed = True
        while changed:
            changed = False
            for bn in order:
                if bn not in executable_blocks:
                    continue
                ssa_block = ssa.blocks[bn]

                incoming_exec_preds = [
                    p for p in preds.get(bn, set()) if (p, bn) in executable_edges
                ]
                for phi in ssa_block.phis:
                    if bn == cfg.entry:
                        continue
                    if not incoming_exec_preds:
                        continue
                    phi_val = UNKNOWN
                    for pred in incoming_exec_preds:
                        incoming_ver = phi.incoming.get(pred, 0)
                        if incoming_ver <= 0:
                            # The variable is undefined / live-in on this
                            # executable edge — a runtime-unknown value, ⊥.
                            phi_val = _join(phi_val, OVERDEFINED)
                        else:
                            phi_val = _join(phi_val, values.get((phi.name, incoming_ver), UNKNOWN))
                    if set_value((phi.name, phi.version), phi_val):
                        changed = True

                for s in ssa_block.statements:
                    if isinstance(s.statement, IRBarrier):
                        # Barriers can modify any variable — widen all
                        # currently-tracked values to OVERDEFINED.  Only the
                        # not-yet-widened keys need touching.
                        if non_overdefined:
                            for key in non_overdefined:
                                values[key] = OVERDEFINED
                            non_overdefined.clear()
                            changed = True
                        continue
                    for var, ver in s.defs.items():
                        if _is_externally_mutable(var):
                            val = OVERDEFINED
                        else:
                            val = _evaluate_def(s.statement, s, values)
                        if set_value((var, ver), val):
                            changed = True

                match cfg.blocks[bn].terminator:
                    case CFGGoto(target=target):
                        targets: tuple[str, ...] = (target,)
                    case CFGBranch(condition=condition, true_target=tt, false_target=ft):
                        targets = branch_targets(bn, condition, tt, ft)
                        if not targets and finalizing:
                            targets = (tt, ft)
                    case _:
                        targets = ()
                # A `break` in this (executable) block reaches the innermost
                # loop's exit even when the header's normal exit edge is pruned.
                brk_exit = break_exits.get(bn)
                if brk_exit is not None:
                    targets = (*targets, brk_exit)
                # A `try` body block reaches its handler(s) via the throw path.
                exc = exc_targets.get(bn)
                if exc:
                    targets = (*targets, *exc)
                for tgt in targets:
                    edge = (bn, tgt)
                    if edge not in executable_edges:
                        executable_edges.add(edge)
                        changed = True
                    if tgt in cfg.blocks and tgt not in executable_blocks:
                        executable_blocks.add(tgt)
                        changed = True
            time.sleep(0)  # Yield GIL between fixed-point iterations
        if finalizing:
            break
        finalizing = True

    constant_branches: list[ConstantBranch] = []
    for bn in order:
        if bn not in executable_blocks:
            continue
        term = cfg.blocks[bn].terminator
        if not isinstance(term, CFGBranch):
            continue
        decision = _evaluate_branch_decision(
            cfg,
            ssa,
            bn,
            term.condition,
            values,
            existence_folder,
        )
        if decision is None:
            continue
        cond_text = expr_text(term.condition)
        if decision:
            constant_branches.append(
                ConstantBranch(
                    block=bn,
                    condition=cond_text,
                    value=True,
                    taken_target=term.true_target,
                    not_taken_target=term.false_target,
                )
            )
        else:
            constant_branches.append(
                ConstantBranch(
                    block=bn,
                    condition=cond_text,
                    value=False,
                    taken_target=term.false_target,
                    not_taken_target=term.true_target,
                )
            )

    return values, executable_blocks, executable_edges, tuple(constant_branches)


def _block_use_def(
    cfg: CFGFunction,
    ssa: SSAFunction,
) -> tuple[dict[str, set[SSAValueKey]], dict[str, set[SSAValueKey]]]:
    use: dict[str, set[SSAValueKey]] = {bn: set() for bn in cfg.blocks}
    defs: dict[str, set[SSAValueKey]] = {bn: set() for bn in cfg.blocks}

    for bn, block in ssa.blocks.items():
        seen_defs: set[SSAValueKey] = set()
        for phi in block.phis:
            key = (phi.name, phi.version)
            defs[bn].add(key)
            seen_defs.add(key)
        for stmt in block.statements:
            for n, v in stmt.uses.items():
                key = (n, v)
                if key not in seen_defs:
                    use[bn].add(key)
            for n, v in stmt.defs.items():
                key = (n, v)
                defs[bn].add(key)
                seen_defs.add(key)

        term = cfg.blocks[bn].terminator
        if isinstance(term, CFGBranch):
            term_uses = _condition_use_versions(term.condition, block.exit_versions)
            for n, v in term_uses.items():
                key = (n, v)
                if key not in seen_defs:
                    use[bn].add(key)

    return use, defs


def _liveness(
    cfg: CFGFunction, ssa: SSAFunction
) -> tuple[dict[str, set[SSAValueKey]], dict[str, set[SSAValueKey]]]:
    use, defs = _block_use_def(cfg, ssa)
    live_in: dict[str, set[SSAValueKey]] = {bn: set() for bn in cfg.blocks}
    live_out: dict[str, set[SSAValueKey]] = {bn: set() for bn in cfg.blocks}
    order = list(reversed(cfg.reverse_postorder()))
    preds = _compute_predecessors(cfg)

    # Backward dataflow worklist: a block's live_out is built from its
    # successors' live_in, so when live_in[bn] changes only bn's
    # predecessors need recomputing — re-enqueue those instead of
    # re-scanning every block each pass.
    worklist: list[str] = list(order)
    queued: set[str] = set(worklist)
    yield_counter = 0
    while worklist:
        bn = worklist.pop()
        queued.discard(bn)
        match cfg.blocks[bn].terminator:
            case CFGGoto(target=target):
                succs = (target,)
            case CFGBranch(true_target=tt, false_target=ft):
                succs = (tt, ft)
            case _:
                succs = ()

        out: set[SSAValueKey] = set()
        for succ in succs:
            if succ not in cfg.blocks:
                continue
            edge_live = set(live_in[succ])
            for phi in ssa.blocks[succ].phis:
                edge_live.discard((phi.name, phi.version))
                incoming = phi.incoming.get(bn, 0)
                if incoming > 0:
                    edge_live.add((phi.name, incoming))
            out |= edge_live

        new_in = use[bn] | (out - defs[bn])
        live_out[bn] = out
        if new_in != live_in[bn]:
            live_in[bn] = new_in
            for p in preds.get(bn, set()):
                if p not in queued:
                    worklist.append(p)
                    queued.add(p)
        yield_counter += 1
        if yield_counter % 256 == 0:
            time.sleep(0)  # Yield GIL periodically

    return live_in, live_out


def _vars_in_return(value: str) -> set[str]:
    """Extract variable names from a return value string.

    Uses ``VarReferenceScanner`` so that command substitutions like
    ``[string length $x]`` are recursed into correctly.
    """
    return set(_RETURN_VAR_SCANNER.scan_script(value))


def _ir_statement_words(ir_stmt: IRStatement) -> list[str]:
    """Return the text words of *ir_stmt* that may contain ``[expr {...}]``
    command substitutions (for expr-internal read recovery)."""
    if isinstance(ir_stmt, (IRCall, IRBarrier)):
        words = [ir_stmt.command, *ir_stmt.args]
        return [w for w in words if w]
    if isinstance(ir_stmt, IRAssignValue):
        return [ir_stmt.value] if ir_stmt.value else []
    if isinstance(ir_stmt, IRIncr):
        return [ir_stmt.amount] if ir_stmt.amount else []
    return []


def _block_local_reads(stmt: IRStatement, names: set[str]) -> None:
    """Collect reads inside an ``IRBlock`` (``eval``/``namespace eval``) body.

    A plain ``eval {…}`` body runs in the **current** scope, but the block is
    opaque to the CFG, so reads like ``$x`` in ``eval {puts $x}`` never reach
    ``stmt.uses`` and the assignment feeding them looks dead/unused — recover
    those reads name-level.

    A ``namespace eval ns {…}`` body (``caller_scope=False``) runs in ``ns``,
    not the caller frame: an unqualified ``$x`` there resolves in ``ns`` (Tcl
    errors ``can't read "x"`` when absent), so it is *not* a read of the
    caller's local ``x``.  Recovering it would wrongly suppress a genuine
    unused-parameter (W214) / dead-store finding, so the whole sub-tree is
    skipped (everything lexically inside runs in ``ns`` or deeper).
    """
    if not isinstance(stmt, IRBlock) or not stmt.caller_scope:
        return
    for inner in stmt.body.statements:
        names.update(statement_read_names(inner))
        names |= expr_substitution_read_names(_ir_statement_words(inner))
        _block_local_reads(inner, names)


def _extra_local_reads(cfg: CFGFunction, considered: set[str]) -> frozenset[str]:
    """Variable names read in places the shallow ``stmt.uses`` scan misses —
    inside ``[expr {...}]`` command substitutions (e.g. ``incr i [expr {$x}]``),
    inside a BODY-role script nested in a substitution (``$sock`` in
    ``[catch {close $sock} e]``, including such a ``[catch]`` in an ``if`` /
    ``while`` condition or an expr-valued assignment), and inside
    ``eval``/``namespace eval`` (``IRBlock``) bodies.  Used name-level by the
    dead-store / set-but-never-used / unused-param checks to suppress false
    positives."""
    names: set[str] = set()
    for bn, block in cfg.blocks.items():
        if bn not in considered:
            continue
        for stmt in block.statements:
            names |= expr_substitution_read_names(_ir_statement_words(stmt))
            # A dynamic assignment/incr target (``set $X v``, ``set ${tok}(k) v``,
            # ``incr $C``) reads the name variable — recover it so the value
            # feeding the name isn't seen as a dead store.
            names |= dynamic_target_name_reads(stmt)
            # Expr-valued statements (``set x [expr {…}]`` / bare ``expr``)
            # carry their command substitutions in a parsed expr AST, not in a
            # word — recover reads hidden in those subs' EXPR/BODY scripts.
            if isinstance(stmt, (IRAssignExpr, IRExprEval)):
                names |= expr_substitution_read_names(command_texts_in_expr_node(stmt.expr))
            _block_local_reads(stmt, names)
        term = block.terminator
        # ``if`` / ``while`` / ``for`` conditions are exprs that may embed a
        # substitution whose body reads vars (``if {[catch {f $x} e]} …``).
        if isinstance(term, CFGBranch):
            names |= expr_substitution_read_names(command_texts_in_expr_node(term.condition))
        if isinstance(term, CFGReturn):
            if term.value:
                names |= expr_substitution_read_names([term.value])
            if term.expr is not None:
                names |= expr_substitution_read_names(command_texts_in_expr_node(term.expr))
    return frozenset(names)


def _place_read_names(cfg: CFGFunction, ctx, considered: set[str]) -> frozenset[str]:
    """Names read anywhere in the function, resolved structurally via the
    IR→Place bridge — the single-source replacement for the ad-hoc
    :func:`_extra_local_reads` recovery (expr command-subs, dynamic target/index
    name vars, ``eval`` bodies, branch/return command-subs).

    Returned name-level for the dead-store / set-but-never-used / unused-param
    suppression sets: an over-approximation is sound there (it can only suppress
    a finding, never create one) and never feeds SSA ``uses`` / read-before-set.
    """
    names: set[str] = set()
    for bn, block in cfg.blocks.items():
        if bn not in considered:
            continue
        for stmt in block.statements:
            for p in read_places(stmt, ctx):
                if p.name:
                    names.add(p.name)
        for p in terminator_read_places(block.terminator, ctx):
            if p.name:
                names.add(p.name)
    return frozenset(names)


def _collect_used_names(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    executable_blocks: set[str] | None = None,
    executable_edges: set[tuple[str, str]] | None = None,
    include_return_vars: bool = False,
    resolve_ctx=None,
    place_read_names: frozenset[str] | None = None,
) -> set[str]:
    """Collect all variable names that appear as uses across the function.

    Scans statement operands, branch conditions, phi incoming edges,
    and optionally return value variable references.
    """
    considered = executable_blocks if executable_blocks is not None else set(cfg.blocks)
    used_names: set[str] = set()

    for bn, block in ssa.blocks.items():
        if bn not in considered:
            continue
        for stmt in block.statements:
            used_names.update(stmt.uses.keys())

        term = cfg.blocks[bn].terminator
        if isinstance(term, CFGBranch):
            used_names.update(vars_in_expr_node(term.condition))
        if include_return_vars and isinstance(term, CFGReturn) and not term.braced:
            if term.value is not None:
                for name in _vars_in_return(term.value):
                    used_names.add(name)
            if term.expr is not None:
                used_names.update(vars_in_expr_node(term.expr))

    for bn, block in ssa.blocks.items():
        if bn not in considered:
            continue
        for phi in block.phis:
            for pred, incoming_ver in phi.incoming.items():
                if incoming_ver > 0:
                    if pred not in considered:
                        continue
                    if executable_edges is not None and (pred, bn) not in executable_edges:
                        continue
                    used_names.add(phi.name)

    # Reads the shallow stmt.uses scan misses.  Two complementary recoverers,
    # unioned so neither's blind spot leaks a false "unused": the legacy per-word
    # deep scanner (`_extra_local_reads`) descends arbitrary nested scripts even
    # for unregistered commands, while the Place bridge resolves structurally and
    # recovers reads the deep scanner misses (e.g. ``upvar``-output params read
    # via the alias).  Both only *add* reads, so this can only suppress a finding.
    used_names |= _extra_local_reads(cfg, set(considered))
    if place_read_names is None:
        if resolve_ctx is None:
            resolve_ctx = build_resolve_context(cfg)
        place_read_names = _place_read_names(cfg, resolve_ctx, set(considered))
    used_names |= place_read_names

    return used_names


def _escaping_var_names(cfg: CFGFunction) -> frozenset[str]:
    """Local names that alias storage outside the current frame.

    Covers ``upvar`` (incl. ``upvar #0`` and multi-pair forms),
    ``namespace upvar``, ``global``, ``variable``, and **variables under a
    ``trace``** (``trace add variable NAME ...`` / 8.4 ``trace variable
    NAME ...``).  A write to such a name is observable elsewhere — in another
    scope (caller frame / namespace / global) or via a trace callback that
    fires on the write (verified: a write trace fires on ``set``) — so it must
    never be reported as a dead store (W220) or set-but-never-used variable
    (W211), nor eliminated by DCE, even when the local analysis sees no local
    read.

    Uses the shared :mod:`compiler.var_scoping` grammar so every alias
    form is recognised identically to memory-SSA alias detection — a single
    source of truth rather than ad-hoc command-name matching (which misses
    ``namespace upvar``, whose IR command is just ``namespace``).
    """
    from compiler.var_scoping import (
        global_declaration_indices,
        upvar_local_declaration_indices,
        variable_declaration_indices,
    )

    names: set[str] = set()
    for block in cfg.blocks.values():
        for stmt in block.statements:
            if not isinstance(stmt, (IRCall, IRBarrier)):
                continue
            args = stmt.args
            for i in upvar_local_declaration_indices(stmt.command, args, allow_dynamic_target=True):
                if 0 <= i < len(args):
                    names.add(_normalise_var_name(args[i]))
            if stmt.canonical_command == "::global":
                for i in global_declaration_indices(args):
                    if 0 <= i < len(args):
                        names.add(_normalise_var_name(args[i]))
            elif stmt.canonical_command == "::variable":
                for i in variable_declaration_indices(args):
                    if 0 <= i < len(args):
                        names.add(_normalise_var_name(args[i]))
            elif stmt.canonical_command == "::trace":
                # `trace add variable NAME ...` (8.5+) or `trace variable NAME
                # ...` (8.4): any trace makes accesses to NAME observable, so
                # its writes are not dead.  Conservative — covers all ops.
                target: str | None = None
                if len(args) >= 3 and args[0] == "add" and args[1] == "variable":
                    target = args[2]
                elif len(args) >= 2 and args[0] == "variable":
                    target = args[1]
                if target is not None:
                    nm = _normalise_var_name(target)
                    if nm:
                        names.add(nm)
    return frozenset(names)


def _reportable_local(place: Place) -> bool:
    """A def *place* that a dead-store / unused-variable check may report.

    Subsumes the three legacy name-string guards (`::`-prefix global,
    a dynamic-target skip, and the ``escaping_names``
    membership test): a write is reportable only when it targets a plain
    proc-*local* scalar/array (not a global/namespace var → ``ns != LOCAL_NS``,
    not an ``upvar`` alias or instance var → wrong ``kind``, not under a
    ``trace`` → ``observed``, and not a dynamic/runtime-named target →
    ``UNKNOWN``/``dynamic``).  Anything else is observable outside the frame or
    untrackable, so it must never be flagged."""
    if place.kind not in (PlaceKind.SCALAR, PlaceKind.ARRAY_ELEM, PlaceKind.ARRAY_WHOLE):
        return False
    # ``owner`` is set when an array element's *base* binds through an ``upvar``
    # alias or an instance var (``upvar #1 cursors cursors; set cursors($i) …`` —
    # the element lives in the caller's frame); such a write escapes the frame.
    return place.ns == LOCAL_NS and not place.observed and not place.dynamic and not place.owner


def _def_place(ir_stmt: IRStatement, ctx) -> Place | None:
    """The single def place of an assignment statement (``None`` if not a
    single-target assignment)."""
    places = def_places(ir_stmt, ctx)
    return places[0] if len(places) == 1 else None


_TCL_REGEX_METACHARS = frozenset(r"\^$.|?*+()[]{}")


def _regexp_literal_no_match(pat: str, inp: str) -> bool:
    """Return True iff Tcl's ``regexp PATTERN INPUT`` provably returns 0.

    Sound only when *pat* is a pure-literal pattern: it contains no
    Tcl Advanced Regular Expression (ARE) metacharacters.  In that case
    Tcl ARE's match reduces to substring search and ``pat in inp``
    is the exact predicate.

    Python's ``re.search`` is NOT used as the oracle: Python regex
    and Tcl ARE differ on several constructs (``\\w`` Unicode handling,
    locale class names, atomic groups, etc.), so a Python "no match"
    does NOT imply a Tcl "no match" for non-literal patterns.  The
    literal-only restriction keeps the W210 emission sound for the
    deep-review test case (``regexp {x} y -> v``) while passing on any
    pattern with structure we'd have to interpret.
    """
    if any(c in _TCL_REGEX_METACHARS for c in pat):
        return False
    return pat not in inp


def _must_alias_killed_in_block(
    block_name: str,
    def_idx: int,
    def_place: "Place",
    ssa,
    cfg,
    resolve_ctx,
) -> bool:
    """Return True iff *def_place* (an ARRAY_ELEM/DICT_PATH write at
    ``block.statements[def_idx]``) is overwritten by a *later* write
    in the same block to the EXACT same place, with no intervening
    read of that specific element.

    PR #498 deep-review G15: prior code's overlap relation is suppress-
    only -- it can't say "this earlier write was killed" because the
    folded-name version stream doesn't preserve element identity.  For
    *literal-key* writes we can detect must-alias by comparing the
    Place's (ns, name, index) tuple directly.  Dynamic keys (where
    ``index.dynamic`` is true) fall back to the conservative behaviour.
    """
    if def_place.index is None or getattr(def_place.index, "dynamic", False):
        return False
    cfg_block = cfg.blocks.get(block_name)
    ssa_block = ssa.blocks.get(block_name)
    if cfg_block is None or ssa_block is None:
        return False
    # Walk forward in the block.  A read of the same place between us
    # and the killing write disqualifies the kill.
    for j in range(def_idx + 1, len(cfg_block.statements)):
        ir_stmt = cfg_block.statements[j]
        # Intervening read of the same element?
        for rp in read_places(ir_stmt, resolve_ctx):
            if (
                rp.kind is def_place.kind
                and rp.ns == def_place.ns
                and rp.name == def_place.name
                and rp.index is not None
                and not getattr(rp.index, "dynamic", False)
                and rp.index == def_place.index
            ):
                return False
        # Killing write?
        kill_place = _def_place(ir_stmt, resolve_ctx)
        if (
            kill_place is not None
            and kill_place.kind is def_place.kind
            and kill_place.ns == def_place.ns
            and kill_place.name == def_place.name
            and kill_place.index is not None
            and not getattr(kill_place.index, "dynamic", False)
            and kill_place.index == def_place.index
        ):
            return True
    return False


def _make_element_observed(cfg, ssa, resolve_ctx, considered_blocks):
    """A lazy predicate: is an ``ARRAY_ELEM``/``DICT_PATH`` def-place observed by
    any read in the function under ``place.overlap``?

    Array elements / dict paths fold to one SSA name with a single strong-update
    version stream (``set a(k) 1`` then ``set a(j) 2`` makes the a(k) write look
    dead because ``$a(k)`` reads the *latest* name version), so the
    version-precise ``used`` set cannot see element granularity.  The sound
    fallback is ``overlap`` (version-insensitive) — 8E-refined so a dynamic
    upvar-aliased array no longer overlaps *unrelated* arrays, and a possibly-
    aliased local is conservatively kept (the suppress-only contract).  Shared
    by ``_dead_stores`` (W220) and ``_unused_variables`` (W211) so the analyser
    and the optimiser DCE (O109/O126, which read those lists) stay consistent.
    Computed lazily — only when an element/dict candidate is actually hit.

    The overlap scan is bucketed by base ``(ns, name)``.  For an
    ARRAY_ELEM/DICT_PATH def, a read can observe it under :func:`overlap` only
    when it shares the def's base name (structural element/key overlap), OR — for
    a *dynamic*-aliased def — when it is itself a dynamic alias of any name
    (``overlap`` returns ``p.dynamic and q.dynamic`` there), OR when it is a
    *wildcard* read (``UNKNOWN`` / ``UPVAR_ALIAS``, which overlaps any reportable
    def).  So each query touches only the same-base bucket plus the
    wildcard/dynamic checks, instead of an O(reads) scan per candidate
    (O(defs × reads) overall).  Exactly equivalent to
    ``any(overlap(p_def, rp) for rp in reads)`` for those def kinds."""
    index: tuple[bool, dict[tuple[str, str], list[Place]], list[Place]] | None = None

    def _build() -> tuple[bool, dict[tuple[str, str], list[Place]], list[Place]]:
        wildcard = False
        by_base: dict[tuple[str, str], list[Place]] = {}
        dynamic_nonwild: list[Place] = []
        for b2 in considered_blocks:
            sb2 = ssa.blocks.get(b2)
            if sb2 is None:
                continue
            rps: list[Place] = []
            for s2 in sb2.statements:
                rps.extend(read_places(s2.statement, resolve_ctx))
            term2 = cfg.blocks[b2].terminator
            rps.extend(terminator_read_places(term2, resolve_ctx))
            for rp in rps:
                if rp.kind is PlaceKind.UNKNOWN or rp.kind is PlaceKind.UPVAR_ALIAS:
                    wildcard = True
                    continue
                by_base.setdefault((rp.ns, rp.name), []).append(rp)
                if rp.dynamic:
                    dynamic_nonwild.append(rp)
        return wildcard, by_base, dynamic_nonwild

    def observed(p_def: Place) -> bool:
        nonlocal index
        if index is None:
            index = _build()
        wildcard, by_base, dynamic_nonwild = index
        # A wildcard read (UNKNOWN / UPVAR_ALIAS) overlaps any reportable def.
        if wildcard:
            return True
        if p_def.kind not in (PlaceKind.ARRAY_ELEM, PlaceKind.DICT_PATH):
            # The bucketing assumes the documented def kinds; for anything else
            # fall back to a full scan (no wildcards remain, so by_base is the
            # complete read set).  Defensive — the callers never hit this.
            return any(overlap(p_def, rp) for bucket in by_base.values() for rp in bucket)
        key = (p_def.ns, p_def.name)
        for rp in by_base.get(key, ()):  # same base: structural element/key overlap
            if overlap(p_def, rp):
                return True
        if p_def.dynamic:
            # A dynamic def overlaps any *different*-named dynamic read
            # (``overlap`` returns ``p.dynamic and q.dynamic`` == True there).
            for rp in dynamic_nonwild:
                if (rp.ns, rp.name) != key:
                    return True
        return False

    return observed


def _dead_stores(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    executable_blocks: set[str] | None = None,
    executable_edges: set[tuple[str, str]] | None = None,
    resolve_ctx=None,
    element_observed=None,
) -> tuple[DeadStore, ...]:
    if resolve_ctx is None:
        resolve_ctx = build_resolve_context(cfg)
    considered_blocks = set(executable_blocks) if executable_blocks is not None else set(cfg.blocks)
    # Names read in positions the version-precise `used` set can't see —
    # command-subs / EXPR-or-BODY-role scripts the shallow scan misses (per-word
    # deep-minus-shallow).  A write to such a name is not a dead store even when
    # its version looks unused, so suppress at name level.  (This stays on the
    # per-word recovery: the Place bridge's whole-function read set can't
    # reproduce the per-word "deep-only" distinction a dead store needs — a name
    # read shallowly at one site and only deep at another must still suppress.)
    expr_sub_reads = _extra_local_reads(cfg, considered_blocks)
    # Array elements / dict paths fold to one SSA name with a single
    # strong-update version stream (``set a(k) 1`` then ``set a(j) 2`` makes the
    # a(k) write look dead because ``$a(k)`` reads the *latest* name version).
    # The version-precise ``used`` set therefore cannot see element granularity;
    # fall back to the sound ``place.overlap`` relation (version-insensitive):
    # an element/dict write is observed if *any* read-place could alias it.
    # 8E's refined overlap keeps this precise — a dynamic upvar-aliased array no
    # longer overlaps unrelated arrays, so this no longer over-suppresses.
    # Over-approximating (may miss a genuine same-element reassignment) but never
    # falsely flags; computed lazily (only when an element/dict candidate hits).
    # Reuse the caller's shared predicate (analyse_function builds it once for
    # both _dead_stores and _unused_variables over the same executable set) so
    # the read-place walk + bucketed index aren't built twice per function.
    _element_observed = element_observed or _make_element_observed(
        cfg, ssa, resolve_ctx, considered_blocks
    )

    used: set[SSAValueKey] = set()
    for bn, block in ssa.blocks.items():
        if bn not in considered_blocks:
            continue
        for stmt in block.statements:
            used.update((n, v) for n, v in stmt.uses.items())

        term = cfg.blocks[bn].terminator
        if isinstance(term, CFGBranch):
            term_uses = _condition_use_versions(term.condition, block.exit_versions)
            used.update((n, v) for n, v in term_uses.items())
        elif isinstance(term, CFGReturn) and not term.braced:
            # ``return $x`` / ``return [expr {...}]`` reads its variables at the
            # block's exit versions — exactly like a branch condition.  Without
            # this, an assignment whose only consumer is the return value is
            # wrongly reported as a dead store (W220).
            ret_names: set[str] = set()
            if term.value is not None:
                ret_names |= _vars_in_return(term.value)
            if term.expr is not None:
                ret_names |= vars_in_expr_node(term.expr)
            for name in ret_names:
                used.add((name, block.exit_versions.get(name, 0)))

    for bn, block in ssa.blocks.items():
        if bn not in considered_blocks:
            continue
        for phi in block.phis:
            for pred, incoming in phi.incoming.items():
                if incoming > 0:
                    if pred not in considered_blocks:
                        continue
                    if executable_edges is not None and (pred, bn) not in executable_edges:
                        continue
                    used.add((phi.name, incoming))

    dead: list[DeadStore] = []
    for bn, block in ssa.blocks.items():
        if bn not in considered_blocks:
            continue
        for idx, stmt in enumerate(block.statements):
            for n, v in stmt.defs.items():
                key = (n, v)
                if key in used:
                    continue
                ir_stmt = stmt.statement
                # A write is reportable only if it targets a plain proc-local
                # scalar/array — not a global/namespace/upvar/instance/traced or
                # dynamic (runtime-named) place.  Subsumes the legacy `::`-prefix,
                # dynamic-target, and escaping guards via the Place.
                place = _def_place(ir_stmt, resolve_ctx)
                if place is None or not _reportable_local(place):
                    continue
                # Element / dict writes can't trust the folded-name `used` set —
                # use place-overlap (8E-refined) instead.  G15 closure:
                # before suppressing on overlap, check for a must-alias
                # kill -- a later write to the SAME (ns, name, index)
                # in the same block with no intervening read of THAT
                # exact element overwrites this store.
                if place.kind in (PlaceKind.ARRAY_ELEM, PlaceKind.DICT_PATH):
                    if _must_alias_killed_in_block(bn, idx, place, ssa, cfg, resolve_ctx):
                        # Killed by a later same-element write -> dead.
                        pass
                    elif _element_observed(place):
                        continue
                # Read inside a command-substituted expr (missed by stmt.uses).
                if n in expr_sub_reads:
                    continue
                if isinstance(ir_stmt, IRAssignConst):
                    dead.append(DeadStore(block=bn, statement_index=idx, variable=n, version=v))
                elif isinstance(ir_stmt, IRAssignValue) and "[" not in ir_stmt.value:
                    dead.append(DeadStore(block=bn, statement_index=idx, variable=n, version=v))
                elif isinstance(ir_stmt, IRAssignExpr) and not _expr_has_command(ir_stmt.expr):
                    dead.append(DeadStore(block=bn, statement_index=idx, variable=n, version=v))
    return tuple(dead)


# Read-before-set detection

# Variables implicitly available in Tcl (set by the runtime or interpreter).
_IMPLICIT_VARS = frozenset(
    {
        "argc",
        "argv",
        "argv0",
        "auto_path",
        "env",
        "errorCode",
        "errorInfo",
        "errorResult",
        "tcl_interactive",
        "tcl_library",
        "tcl_patchLevel",
        "tcl_pkgPath",
        "tcl_platform",
        "tcl_precision",
        "tcl_rcFileName",
        "tcl_version",
        "tcl_wordchars",
        "tcl_nonwordchars",
        # Common iRules implicit variables
        "static",
    }
)


def _qualified_variable_alias_tails(cfg: CFGFunction, considered: set[str]) -> frozenset[str]:
    """Local-alias tail names linked by a *qualified* ``variable`` declaration.

    ``variable ns::tail`` (and the dynamic-prefix form ``variable
    ${name}::children`` common in ``struct::tree`` and other generic ADTs)
    links a local variable named by the **tail** (``children``) to the
    namespace variable — verified in tclsh: ``variable ${name}::children``
    then ``$children`` reads ``${name}::children``.  The declaration's def is
    recorded under the qualified spelling (and the ``$``-prefixed dynamic form
    is dropped entirely by ``variable_declaration_indices``), so the bare
    ``$children`` read shows up as a spurious version-0 read-before-set.
    Exempt the static tail (suppress-only, name-level)."""
    names: set[str] = set()
    for bn in considered:
        block = cfg.blocks.get(bn)
        if block is None:
            continue
        for stmt in block.statements:
            if not isinstance(stmt, (IRCall, IRBarrier)):
                continue
            if stmt.canonical_command != "::variable":
                continue
            # ``variable`` alternates (name, value?) pairs — names at even args.
            for i in range(0, len(stmt.args), 2):
                text = stmt.args[i]
                if "::" not in text:
                    # Unqualified: the def is already recorded under the bare
                    # name and matches the read — nothing to recover.
                    continue
                tail = text.rsplit("::", 1)[-1]
                base, _ = split_array_name(tail)
                if base and "$" not in base and "[" not in base and "{" not in base:
                    names.add(_normalise_var_name(base))
    return frozenset(names)


def _iter_expr_commands(expr: ExprNode, out: list[ExprCommand]) -> None:
    """Collect every ``ExprCommand`` node reachable in *expr* (no short-circuit
    pruning — we want all existence checks, taken or not)."""
    if isinstance(expr, ExprCommand):
        out.append(expr)
    elif isinstance(expr, ExprBinary):
        _iter_expr_commands(expr.left, out)
        _iter_expr_commands(expr.right, out)
    elif isinstance(expr, ExprUnary):
        _iter_expr_commands(expr.operand, out)
    elif isinstance(expr, ExprTernary):
        _iter_expr_commands(expr.condition, out)
        _iter_expr_commands(expr.true_branch, out)
        _iter_expr_commands(expr.false_branch, out)
    elif isinstance(expr, ExprCall):
        for arg in expr.args:
            _iter_expr_commands(arg, out)


def _collect_existence_in_word(text: str, out: set[str]) -> None:
    """Find ``[info exists X]`` / ``[array exists X]`` substitutions inside a
    word and add their targets to *out* (recursing into nested substitutions)."""
    if "[" not in text:
        return
    try:
        tokens = TclLexer(text).tokenise_all()
    except Exception:
        return
    for tok in tokens:
        if tok.type is TokenType.CMD:
            parsed = _parse_existence_check(f"[{tok.text}]")
            if parsed is not None:
                # Only a literal plain-scalar target is existence-tested without
                # a value read; a dynamic target like ``info exists $name``
                # reads ``name`` to form the name, so it must stay reportable.
                nm = _existence_skip_name(parsed[1])
                if nm:
                    out.add(nm)
            else:
                # Other safe-on-unset references: ``[array get|names|size FOO]``
                # / ``[vwait varName]`` — the variable need not be set.
                safe = _parse_safe_unset_ref(f"[{tok.text}]")
                if safe is not None:
                    nm = _existence_skip_name(safe)
                    if nm:
                        out.add(nm)
                _collect_existence_in_word(tok.text, out)


def _existence_checks_in_expr(expr: ExprNode) -> set[str]:
    """Names existence-checked (or otherwise safe-on-unset) anywhere in *expr*."""
    names: set[str] = set()
    cmds: list[ExprCommand] = []
    _iter_expr_commands(expr, cmds)
    for c in cmds:
        parsed = _parse_existence_check(c.text)
        if parsed is not None:
            nm = _existence_skip_name(parsed[1])
            if nm:
                names.add(nm)
            continue
        # Other safe-on-unset references inside an expression: ``expr {[array
        # size FOO] > 0}`` etc.
        safe = _parse_safe_unset_ref(c.text)
        if safe is not None:
            nm = _existence_skip_name(safe)
            if nm:
                names.add(nm)
    return names


def _existence_checks_in_stmt(stmt: IRStatement) -> set[str]:
    """Names existence-checked (or otherwise safe-on-unset) by *stmt* itself —
    the reference does not require the variable to have a value, so it is never
    a read-before-set.

    Covers ``info exists`` / ``array exists`` (the canonical idiom) plus a small
    set of other commands tclsh-verified to accept an unset variable:
    ``vwait varName`` (blocks waiting for the variable to be written — its whole
    purpose is to monitor an as-yet-unset variable; ``vwait forever`` is the
    canonical infinite-wait idiom — the whole point is that the variable is
    initially unset and vwait blocks until something writes it).
    """
    names: set[str] = set()
    if isinstance(stmt, (IRCall, IRBarrier)):
        args = stmt.args
        if (
            stmt.canonical_command in ("::info", "::array")
            and len(args) >= 2
            and args[0] == "exists"
        ):
            nm = _existence_skip_name(args[1])
            if nm:
                names.add(nm)
        # ``vwait varName`` waits for the variable to be written; it does not
        # need to be set beforehand (``vwait forever`` is the canonical idiom).
        # vwait's options end at the first non-``-`` arg; ``--`` ends them too.
        # We need the variable arg(s) — they are the trailing positionals.
        elif stmt.canonical_command == "::vwait" and args:
            i = 0
            while i < len(args) and args[i].startswith("-") and args[i] != "--":
                # ``-readable``/``-writable``/``-timeout``/``-variable`` take a value.
                if args[i] in ("-readable", "-writable", "-timeout", "-variable"):
                    i += 2
                else:
                    i += 1
            if i < len(args) and args[i] == "--":
                i += 1
            for arg in args[i:]:
                nm = _existence_skip_name(arg)
                if nm:
                    names.add(nm)
        for arg in args:
            _collect_existence_in_word(arg, names)
    value = getattr(stmt, "value", None)
    if isinstance(value, str):
        _collect_existence_in_word(value, names)
    expr = getattr(stmt, "expr", None)
    if expr is not None and not isinstance(expr, str):
        names |= _existence_checks_in_expr(expr)
    return names


# ``info vars`` / ``info locals`` are existence probes too — they return the
# matching visible variable names rather than reading a value.  Only an exact
# (non-glob) name is statically decidable; a glob pattern would need a
# variable-lifetime matrix we do not have, so those are skipped.


def _info_vars_exact_target(cmd_text: str) -> str | None:
    """``[info vars X]`` / ``[info locals X]`` → exact local name ``X``."""
    parsed = _parse_cmd_subst(cmd_text)
    if parsed is None:
        return None
    cmd_name, args_text = parsed
    base = cmd_name[2:] if cmd_name.startswith("::") else cmd_name
    if base != "info":
        return None
    try:
        parts = _split_tcl_list(args_text) if args_text.strip() else []
    except Exception:
        return None
    if len(parts) != 2 or parts[0] not in ("vars", "locals"):
        return None
    return _existence_skip_name(parts[1])


def _is_info_vars_full_list(cmd_text: str) -> bool:
    """True when *cmd_text* is exactly ``[info vars]`` / ``[info locals]``."""
    parsed = _parse_cmd_subst(cmd_text)
    if parsed is None:
        return False
    cmd_name, args_text = parsed
    base = cmd_name[2:] if cmd_name.startswith("::") else cmd_name
    if base != "info":
        return False
    try:
        parts = _split_tcl_list(args_text) if args_text.strip() else []
    except Exception:
        return False
    return parts in (["vars"], ["locals"])


def _split_command_args(args_text: str) -> list[str]:
    """Split a command's argument text into words, keeping ``[cmd]``, ``{str}``
    and ``$var`` substitutions as single words (unlike ``_split_tcl_list``,
    which treats ``[`` / ``]`` as ordinary characters)."""
    words: list[str] = []
    prev_sep = True
    try:
        tokens = TclLexer(args_text).tokenise_all()
    except Exception:
        return []
    for tok in tokens:
        if tok.type in (TokenType.SEP, TokenType.EOL, TokenType.EOF):
            prev_sep = True
            continue
        piece = tok.text
        if tok.type is TokenType.CMD:
            piece = f"[{tok.text}]"
        elif tok.type is TokenType.STR:
            piece = f"{{{tok.text}}}"
        elif tok.type is TokenType.VAR:
            piece = f"${tok.text}"
        if prev_sep or not words:
            words.append(piece)
        else:
            words[-1] += piece
        prev_sep = False
    return words


_LSEARCH_SAFE_OPTS = frozenset({"-exact", "-glob", "-sorted"})


def _lsearch_info_vars_needle(cmd_text: str) -> str | None:
    """``[lsearch ?-exact? [info vars] X]`` → exact needle name ``X``.

    Only option flags that do not change which exact name matches are allowed
    (``-regexp`` / ``-nocase`` / ``-all`` etc. would be unsound).
    """
    parsed = _parse_cmd_subst(cmd_text)
    if parsed is None:
        return None
    cmd_name, args_text = parsed
    base = cmd_name[2:] if cmd_name.startswith("::") else cmd_name
    if base != "lsearch":
        return None
    parts = _split_command_args(args_text)
    options = [p for p in parts if p.startswith("-")]
    if any(opt not in _LSEARCH_SAFE_OPTS for opt in options):
        return None
    positional = [p for p in parts if not p.startswith("-")]
    if len(positional) != 2 or not _is_info_vars_full_list(positional[0]):
        return None
    return _existence_skip_name(positional[1])


def _llength_info_vars_target(cmd_text: str) -> str | None:
    """``[llength [info vars X]]`` → exact name ``X`` (truthy ⇒ X exists)."""
    parsed = _parse_cmd_subst(cmd_text)
    if parsed is None:
        return None
    cmd_name, args_text = parsed
    base = cmd_name[2:] if cmd_name.startswith("::") else cmd_name
    if base != "llength":
        return None
    return _info_vars_exact_target(args_text.strip())


def _is_empty_string_literal(node: ExprNode) -> bool:
    if not isinstance(node, ExprString):
        return False
    text = node.text
    return text in ("", '""', "{}") or (len(text) >= 2 and text[0] in '"{' and text[1:-1] == "")


def _int_const(node: ExprNode) -> int | None:
    if isinstance(node, ExprLiteral):
        try:
            return int(node.text)
        except ValueError:
            return None
    if (
        isinstance(node, ExprUnary)
        and node.op is UnaryOp.NEG
        and isinstance(node.operand, ExprLiteral)
    ):
        try:
            return -int(node.operand.text)
        except ValueError:
            return None
    return None


def _catch_pure_read_target(cmd_text: str) -> str | None:
    """``[catch {set _ $X}]`` → ``X``, when the body is exactly a pure read of a
    single plain scalar.

    A zero result (no error) proves the read succeeded, so ``X`` exists and is
    scalar-readable.  A non-zero result is ambiguous (the variable could be
    missing *or* be an array read as a scalar), so callers only use the
    succeeded (false) branch.
    """
    parsed = _parse_cmd_subst(cmd_text)
    if parsed is None:
        return None
    cmd_name, args_text = parsed
    base = cmd_name[2:] if cmd_name.startswith("::") else cmd_name
    if base != "catch":
        return None
    words = _split_command_args(args_text)
    # body, plus optional result / options variable names (Tcl 8.5+).
    if not (1 <= len(words) <= 3):
        return None
    body = words[0]
    if not (len(body) >= 2 and body[0] == "{" and body[-1] == "}"):
        return None
    inner = _split_command_args(body[1:-1].strip())
    if len(inner) != 3 or inner[0] != "set" or not inner[2].startswith("$"):
        return None
    return _existence_skip_name(inner[2][1:])


_SWAP_COMPARISON = {
    BinOp.GT: BinOp.LT,
    BinOp.LT: BinOp.GT,
    BinOp.GE: BinOp.LE,
    BinOp.LE: BinOp.GE,
    BinOp.EQ: BinOp.EQ,
    BinOp.NE: BinOp.NE,
}


def _lsearch_found_when_true(op: BinOp, value: int, cmd_on_left: bool) -> bool | None:
    """For ``[lsearch …] <op> <value>``, does *true* mean the needle was found
    (index ≥ 0)?  Returns ``True`` (found), ``False`` (not found), or ``None``."""
    if not cmd_on_left:
        op = _SWAP_COMPARISON.get(op, op)
    if (
        (op is BinOp.GT and value == -1)
        or (op is BinOp.GE and value == 0)
        or (op is BinOp.NE and value == -1)
    ):
        return True
    if (
        (op is BinOp.EQ and value == -1)
        or (op is BinOp.LT and value == 0)
        or (op is BinOp.LE and value == -1)
    ):
        return False
    return None


def _comparison_existence(expr: ExprBinary) -> tuple[set[str], set[str]] | None:
    """Existence facts proved by a comparison: ``[info vars X] ne ""`` and
    ``[lsearch [info vars] X] > -1`` (and their negative/`eq`/`==` variants)."""
    # ``[info vars X] ne|eq ""``
    for cmd_side, other in ((expr.left, expr.right), (expr.right, expr.left)):
        if isinstance(cmd_side, ExprCommand) and _is_empty_string_literal(other):
            name = _info_vars_exact_target(cmd_side.text)
            if name is not None:
                if expr.op in (BinOp.STR_NE, BinOp.NE):
                    return {name}, set()
                if expr.op in (BinOp.STR_EQ, BinOp.EQ):
                    return set(), {name}
    # ``[lsearch [info vars] X] > -1`` (membership search)
    for cmd_side, other in ((expr.left, expr.right), (expr.right, expr.left)):
        if isinstance(cmd_side, ExprCommand):
            needle = _lsearch_info_vars_needle(cmd_side.text)
            value = _int_const(other)
            if needle is not None and value is not None:
                found = _lsearch_found_when_true(expr.op, value, cmd_side is expr.left)
                if found is True:
                    return {needle}, set()
                if found is False:
                    return set(), {needle}
    return None


def _existence_implications(expr: ExprNode) -> tuple[set[str], set[str]]:
    """Return ``(exists_if_true, exists_if_false)`` — names a condition proves
    to exist when it evaluates true vs false.

    A bare ``[info exists X]`` proves ``X`` when true; ``!`` swaps the arms;
    ``&&`` proves its operands' positive facts only when the whole thing is
    true; ``||`` proves their negative facts only when the whole thing is
    false.  ``info vars`` / ``info locals`` membership idioms
    (``ne ""``, ``[llength …]``, ``[lsearch [info vars] …] > -1``) are also
    recognised.  Everything else proves nothing.
    """
    if isinstance(expr, ExprCommand):
        parsed = _parse_existence_check(expr.text)
        name = _existence_scalar_name(parsed[1]) if parsed is not None else None
        if name is None:
            name = _llength_info_vars_target(expr.text)
        if name is not None:
            return {name}, set()
        # ``catch {set _ $X}`` — a zero (false) result proves X exists.
        catch_var = _catch_pure_read_target(expr.text)
        if catch_var is not None:
            return set(), {catch_var}
        return set(), set()
    if isinstance(expr, ExprUnary) and expr.op in (UnaryOp.NOT, UnaryOp.WORD_NOT):
        true_set, false_set = _existence_implications(expr.operand)
        return false_set, true_set
    if isinstance(expr, ExprBinary):
        comparison = _comparison_existence(expr)
        if comparison is not None:
            return comparison
        left_true, left_false = _existence_implications(expr.left)
        right_true, right_false = _existence_implications(expr.right)
        # The right operand is evaluated after the left; if it can mutate a
        # local (e.g. ``[info exists X] && [unset X; expr 1]``) the left's facts
        # may no longer hold when control reaches the branch, so drop them.
        right_safe = _expr_mutation_free(expr.right)
        if expr.op in (BinOp.AND, BinOp.WORD_AND):
            carry = left_true if right_safe else set()
            return carry | right_true, set()
        if expr.op in (BinOp.OR, BinOp.WORD_OR):
            carry = left_false if right_safe else set()
            return set(), carry | right_false
    return set(), set()


def _block_dominates(ssa: SSAFunction, dominator: str, node: str) -> bool:
    """True when *dominator* dominates *node* in the SSA dominator tree."""
    current: str | None = node
    while current is not None:
        if current == dominator:
            return True
        current = ssa.idom.get(current)
    return False


def _existence_narrowed_blocks(
    cfg: CFGFunction, ssa: SSAFunction, considered: set[str]
) -> dict[str, set[str]]:
    """Map each block to the names a dominating existence guard proves to exist.

    For ``if {[info exists X]} { … }`` the true successor's dominated region
    knows ``X`` exists; a read there is not a read-before-set.  The false region
    of a *negated* guard is handled symmetrically.  Only edges whose successor
    is entered solely from the guard are used, so the proof is sound.
    """
    preds = _compute_predecessors(cfg)
    known: dict[str, set[str]] = {}
    for bn in considered:
        block = cfg.blocks.get(bn)
        if block is None or not isinstance(block.terminator, CFGBranch):
            continue
        exists_true, exists_false = _existence_implications(block.terminator.condition)
        for target, names in (
            (block.terminator.true_target, exists_true),
            (block.terminator.false_target, exists_false),
        ):
            if not names or preds.get(target) != {bn}:
                continue
            for d in considered:
                if _block_dominates(ssa, target, d):
                    known.setdefault(d, set()).update(names)
    return known


def _read_before_set(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    executable_blocks: set[str] | None = None,
    params: frozenset[str] = frozenset(),
    values: dict[SSAValueKey, LatticeValue] | None = None,
) -> tuple[ReadBeforeSet, ...]:
    """Find variables that are read before being set.

    A variable use with SSA version 0 means no prior definition was found
    during SSA renaming — the variable is read before set on that path.
    """
    considered = executable_blocks if executable_blocks is not None else set(cfg.blocks)
    skip = _IMPLICIT_VARS | params
    # A dominating ``info exists`` / ``array exists`` guard proves a variable
    # exists in its true region, so a read there is not a read-before-set.
    narrowed = _existence_narrowed_blocks(cfg, ssa, considered)

    # SCCP values fallback: when not passed, treat as empty (no const
    # evidence -> conservative behaviour for the dict-with key check).
    if values is None:
        values = {}

    # dict with/update creates local variables from dict keys at runtime.
    # When the dict value is statically known (CONST/CONSTSET), use its
    # ACTUAL keys as the suppression set -- a tight scope that doesn't
    # hide unrelated bugs.  When the dict is unknown, fall back to the
    # conservative whole-function suppression (matches pre-fix behaviour)
    # but limit it via ``_has_dict_with`` semantics.
    #
    # PR #498 deep-review finding 6 / G9: pre-fix, ANY ``dict with`` in
    # the function silenced every version-0 unknown-variable read in
    # the entire proc -- hiding unrelated W210 / W307 noise.
    _has_dict_with = False
    _dict_with_known_keys: set[str] = set()
    _dict_with_any_unknown = False

    def _harvest_dict_keys(value_str: str) -> bool:
        """Extract dict keys from *value_str* into ``_dict_with_known_keys``.

        Returns True if parsing succeeded (even for empty), False on
        a Tcl list-parse error (caller should mark unknown).  Uses the
        shared :func:`compiler.tcl_expr_eval._split_tcl_list` parser
        which handles braces, quotes, and backslash escapes correctly
        (no hand-rolled list splitter here).
        """
        from compiler.tcl_expr_eval import _split_tcl_list

        try:
            elements = _split_tcl_list(value_str)
        except Exception:
            return False
        # Dict = alternating key/value; only even-index elements are keys.
        for i in range(0, len(elements), 2):
            _dict_with_known_keys.add(elements[i])
        return True

    for bn in considered:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue
        for stmt_idx, stmt in enumerate(block.statements):
            if (
                isinstance(stmt, IRBarrier)
                and stmt.canonical_command == "::dict"
                and stmt.args
                and stmt.args[0] in ("with", "update")
            ):
                _has_dict_with = True
                dict_var = stmt.args[1] if len(stmt.args) >= 2 else ""
                if not dict_var:
                    continue
                skip = skip | {dict_var}
                # Look up the SCCP value of the SPECIFIC version of
                # dict_var being read by this dict-with (the IRBarrier's
                # ``uses`` records ``{dict_var: input_version}``).  This
                # is the value BEFORE the dict-with widens it; using the
                # function-wide ``values[dict_var]`` would catch the
                # post-widening OVERDEFINED.
                # SCCP retroactively widens ``d#1`` to OVERDEFINED when
                # any IRBarrier touches it (the conservative side-effect
                # model).  Fall back to the raw IR: scan backwards in
                # the SAME block for the most recent IRAssignConst /
                # IRAssignValue of ``dict_var`` and use its literal value
                # if statically known.
                literal_value: str | None = None
                for prev_idx in range(stmt_idx - 1, -1, -1):
                    prev_stmt = block.statements[prev_idx]
                    if isinstance(prev_stmt, IRAssignConst):
                        # IRAssignConst's defined variable is in
                        # ``prev_stmt`` itself (the IR doesn't have a
                        # direct ``name`` field on the statement; check
                        # the SSA defs).
                        prev_ssa = (
                            ssa_block.statements[prev_idx]
                            if prev_idx < len(ssa_block.statements)
                            else None
                        )
                        if prev_ssa is None:
                            continue
                        if dict_var in prev_ssa.defs and isinstance(prev_stmt.value, str):
                            literal_value = prev_stmt.value
                            break
                    elif isinstance(prev_stmt, (IRBarrier, IRCall)):
                        # An IRBarrier or impure call between us and
                        # the literal assignment invalidates the trace.
                        # IRCall: only break for impure calls (pure
                        # commands can't mutate dict_var).
                        if isinstance(prev_stmt, IRBarrier):
                            break
                        from compiler.side_effects import classify_side_effects

                        if not classify_side_effects(
                            getattr(prev_stmt, "command", ""),
                            getattr(prev_stmt, "args", ()),
                        ).pure:
                            break
                if literal_value is None:
                    _dict_with_any_unknown = True
                elif not _harvest_dict_keys(literal_value):
                    _dict_with_any_unknown = True

    # When dict with/update is present, collect all variable names that
    # have an explicit definition somewhere in the function.  Variables
    # that are ONLY seen via version-0 uses are likely dict-unpacked
    # keys and should be exempted.
    if _has_dict_with:
        explicitly_defined: set[str] = set()
        for bn2 in considered:
            ssa_block2 = ssa.blocks.get(bn2)
            if ssa_block2 is None:
                continue
            for s in ssa_block2.statements:
                for n, v in s.defs.items():
                    if v > 0:
                        explicitly_defined.add(n)
            for phi in ssa_block2.phis:
                if phi.version > 0:
                    explicitly_defined.add(phi.name)

    # Variables written by command substitutions (``set e [catch {...} msg
    # opts]`` writes ``msg``/``opts``) are invisible to SSA def tracking — the
    # ``[...]`` is one opaque value word — so a later ``$msg`` read shows up as
    # a spurious version-0 read-before-set.  Collect those written names and
    # exempt them (name-level, like the dict-with exemption above).
    #
    # Also scan the block's terminator: a return value or branch condition can
    # contain ``[set x ...]`` (e.g.
    # ``return [read [set x [open $f r]]][close $x]`` — the standard
    # open-read-close idiom in tcllib's installer).  Without scanning the
    # terminator, the ``$x`` read inside ``[close $x]`` (extracted by
    # ``_vars_in_return``) looks unset because ``set x`` was inside an opaque
    # command-sub of the same return value.
    for bn in considered:
        block = cfg.blocks.get(bn)
        if block is None:
            continue
        for stmt in block.statements:
            cs_writes = statement_cmd_sub_write_names(stmt)
            if cs_writes:
                skip = skip | cs_writes
        term = block.terminator
        if isinstance(term, CFGReturn):
            if term.value is not None:
                skip = skip | command_sub_write_names(term.value)
            if term.expr is not None:
                for cmd_text in command_texts_in_expr_node(term.expr):
                    skip = skip | command_sub_write_names(cmd_text)
        elif isinstance(term, CFGBranch):
            for cmd_text in command_texts_in_expr_node(term.condition):
                skip = skip | command_sub_write_names(cmd_text)

    # ``info exists``/``array exists`` guards — main's #502 added per-block flow-
    # sensitive narrowing (``_existence_narrowed_blocks`` above; consulted via
    # ``block_known`` in the per-statement loop) and ``_existence_checks_in_stmt``
    # for the bare-statement form, both strictly more precise than stage-1's
    # earlier function-wide name-level skip (which over-suppressed in the false
    # arm of a guard).  We've dropped the broad skip and rely on the narrowing.

    # `variable ns::tail` / `variable ${name}::children` links a local alias
    # named by the static tail (tclsh-verified); exempt those names.
    skip = skip | _qualified_variable_alias_tails(cfg, considered)

    # An *un-lowered* loop keeps its body opaque to the CFG, so the body's
    # reads are recovered name-level (→ version-0 uses) but its writes/loop-vars
    # are not — making every body-local variable look read-before-set.  Three
    # cases stay un-lowered: a *frozen* loop (``while``/``for`` with a
    # command-substitution condition, e.g. ``while {[gets $fp line] >= 0} {…}``,
    # an IRBarrier for tclsh-parity codegen); a *fully-qualified* builtin
    # (``::foreach {k v} $d {…}`` — the lowering dispatch matches only bare
    # names, so it stays an IRCall); and ``dict for`` / ``dict map`` (Tcl 9
    # compiles those as a generic ensemble invoke, so cfg.py emits an
    # IRBarrier carrying ``{k v} dictVal body`` as args, with ``::tcl::dict::for``
    # / ``::tcl::dict::map`` as the canonical command).  Recover their body
    # writes and loop vars to balance the read recovery (name-level, suppress-only).
    _UNLOWERED_LOOP_CMDS = (
        "::while",
        "::for",
        "::foreach",
        "::lmap",
        "::tcl::dict::for",
        "::tcl::dict::map",
    )
    for bn in considered:
        block = cfg.blocks.get(bn)
        if block is None:
            continue
        for stmt in block.statements:
            if not isinstance(stmt, (IRCall, IRBarrier)):
                continue
            cc = stmt.canonical_command
            if cc not in _UNLOWERED_LOOP_CMDS or not stmt.args:
                continue
            args = list(stmt.args)
            for a in args:
                skip = skip | body_write_names(a)
            # foreach/lmap loop variables live in the varlist args at even
            # positions before the trailing body (``::foreach {k v} $d body``).
            # ``dict for``/``dict map`` after the subcommand-strip in cfg.py
            # have the same shape: ``{k v} dictValue body``.
            if (
                cc in ("::foreach", "::lmap", "::tcl::dict::for", "::tcl::dict::map")
                and len(args) >= 3
            ):
                for i in range(0, len(args) - 1, 2):
                    for v in args[i].split():
                        base, _ = split_array_name(v)
                        if base and "$" not in base and "[" not in base:
                            nm = _normalise_var_name(base)
                            if nm:
                                skip = skip | {nm}

    # Track which version-0 variables we've already reported to avoid
    # duplicate warnings for the same variable in the same function.
    reported: set[str] = set()
    result: list[ReadBeforeSet] = []

    order = cfg.reverse_postorder()
    for bn in order:
        if bn not in considered:
            continue
        ssa_block = ssa.blocks.get(bn)
        if ssa_block is None:
            continue

        block_known = narrowed.get(bn, frozenset())

        for idx, stmt in enumerate(ssa_block.statements):
            occ_checks: set[str] | None = None
            for name, ver in stmt.uses.items():
                if ver != 0:
                    continue
                if name in skip or name in reported:
                    continue
                # In dict-with scopes, suppress for variables that have
                # no explicit definition — they were likely unpacked.
                # G9 closure: when SCCP knows the dict's keys (e.g. for
                # ``set d {}; dict with d {}``, the empty dict has NO
                # keys), only suppress for names in that key set.  When
                # the dict's value is unknown, fall back to the previous
                # broad suppression (preserves pre-fix behaviour for
                # dynamically-built dicts).
                if _has_dict_with and name not in explicitly_defined:
                    if _dict_with_any_unknown or name in _dict_with_known_keys:
                        continue
                # A ``::`` anywhere marks a namespace-qualified reference
                # (Tcl local names cannot contain ``::``) — e.g. a direct read
                # of a namespace var ``array names ${name}::parent`` resolves
                # to ``{name}::parent``.  Such storage lives in its namespace,
                # not this frame, so the local read-before-set check never
                # applies (covers the former ``::``/``static::`` prefixes too).
                if "::" in name:
                    continue
                # Proven to exist here by a dominating existence guard.
                if name in block_known:
                    continue
                # The statement is itself an existence check of this name
                # (``info exists X``) — that does not read the value.
                if occ_checks is None:
                    occ_checks = _existence_checks_in_stmt(cfg.blocks[bn].statements[idx])
                if name in occ_checks:
                    continue
                reported.add(name)
                result.append(ReadBeforeSet(block=bn, statement_index=idx, variable=name))

        # Also check branch conditions for version-0 uses.
        # The condition's variable versions come from exit_versions.
        term = cfg.blocks[bn].terminator
        if isinstance(term, CFGBranch):
            cond_checks = _existence_checks_in_expr(term.condition)
            for name in vars_in_expr_node(term.condition):
                ver = ssa_block.exit_versions.get(name, 0)
                if ver != 0:
                    continue
                if name in skip or name in reported:
                    continue
                # A ``::`` anywhere marks a namespace-qualified reference
                # (Tcl local names cannot contain ``::``) — e.g. a direct read
                # of a namespace var ``array names ${name}::parent`` resolves
                # to ``{name}::parent``.  Such storage lives in its namespace,
                # not this frame, so the local read-before-set check never
                # applies (covers the former ``::``/``static::`` prefixes too).
                if "::" in name:
                    continue
                if name in block_known or name in cond_checks:
                    continue
                reported.add(name)
                # Use statement_index=-1 and block to signal condition-level use.
                # The range comes from the terminator itself.
                result.append(ReadBeforeSet(block=bn, statement_index=-1, variable=name))

        # Return values read variables too — ``return $x`` / ``return [expr
        # {$x+1}]`` error in tclsh when x is unset, so a version-0 read in a
        # return terminator is read-before-set just like a branch condition.
        elif isinstance(term, CFGReturn):
            return_reads: set[str] = set()
            if term.value is not None:
                return_reads |= _vars_in_return(term.value)
            if term.expr is not None:
                return_reads |= vars_in_expr_node(term.expr)
            for name in return_reads:
                if ssa_block.exit_versions.get(name, 0) != 0:
                    continue
                if name in skip or name in reported:
                    continue
                if _has_dict_with and name not in explicitly_defined:
                    continue
                if "::" in name:
                    continue
                reported.add(name)
                result.append(ReadBeforeSet(block=bn, statement_index=-1, variable=name))

    # PR #498 deep-review G1/G2 closure: regexp/scan output vars are
    # CONDITIONAL defs (only written on success).  When the pattern +
    # input are both statically known AND can be statically proven not
    # to match, the def is provably unreached and any subsequent read
    # is a real W210.  This is the precise approach (only fire when
    # SCCP proves no-match), not the naive approach (treat every
    # regexp/scan output as conditional, which breaks trust-the-match
    # Tcl idioms like ``regexp -- \$WordBreakRE(after) abc result``).
    provably_unset: dict[str, tuple[str, int]] = {}  # var_name -> (block, stmt_idx)
    from compiler.registry.runtime import options_with_value, skip_options
    from compiler.scan_format import scan_provably_no_match

    for bn in considered:
        block = cfg.blocks.get(bn)
        if block is None:
            continue
        for stmt_idx, stmt in enumerate(block.statements):
            if not isinstance(stmt, IRCall):
                continue
            if stmt.command not in ("regexp", "scan"):
                continue
            if not stmt.defs:
                continue
            tokens = stmt.tokens
            if tokens is None:
                continue
            pos = skip_options(list(stmt.args), options_with_value(stmt.command))
            if pos + 1 >= len(stmt.args):
                continue
            # Use the LEXER's representative token type for each arg --
            # purely literal words are STR (braced ``{...}``) or ESC
            # (bare / escaped); VAR (``$x``) and CMD (``[c]``) are
            # substitutions whose value is unknown statically.  Only the
            # STR/ESC case lets us reason about the runtime value.
            from shared.tokens import TokenType

            argv = tokens.argv
            argv_texts = tokens.argv_texts
            # ``argv``/``argv_texts`` count the command word at index 0;
            # ``stmt.args`` does not, so add 1 to translate.
            t_pos = pos + 1
            if t_pos + 1 >= len(argv):
                continue
            tok_a, tok_b = argv[t_pos], argv[t_pos + 1]
            if tok_a.type not in (TokenType.STR, TokenType.ESC):
                continue
            if tok_b.type not in (TokenType.STR, TokenType.ESC):
                continue
            text_a = argv_texts[t_pos]
            text_b = argv_texts[t_pos + 1]
            # Arg ordering differs:
            #   regexp ?switches? PATTERN STRING ?VAR...?
            #   scan   STRING FORMAT ?VAR...?
            if stmt.command == "regexp":
                pat, inp = text_a, text_b
            else:
                inp, pat = text_a, text_b
            no_match = False
            if stmt.command == "regexp":
                no_match = _regexp_literal_no_match(pat, inp)
            else:
                no_match = scan_provably_no_match(pat, inp)
            if no_match:
                for def_var in stmt.defs:
                    provably_unset[def_var] = (bn, stmt_idx)
    # Now fire W210 for any USE of a provably-unset var that's
    # reachable from the regexp/scan call.  The use must come AFTER
    # the call (later in the same block, OR in an EXECUTABLE descendant
    # block dominated by the call's block).  SCCP keeps the post-fixpoint
    # ``considered`` set restricted to executable blocks, so a use
    # inside an ``if {[regexp ...]}`` true-branch is naturally excluded
    # when the regexp is statically no-match (SCCP marks the condition
    # constant-false and the then-branch unreachable).  Conversely the
    # ``if {![regexp ...]}`` false-arm IS executable and a use there is
    # a real W210.  (Finding 2 of the PR #498/#499 follow-up review.)
    if provably_unset:
        # Build a reverse postorder traversal of the call block's
        # forward-reachable executable subgraph -- cheap once per
        # provably-unset variable.  Then walk every executable block;
        # fire if (a) the block is dominated by the call's block or
        # (b) it IS the call's block and the use is after the call.
        for bn in considered:
            ssa_block = ssa.blocks.get(bn)
            if ssa_block is None:
                continue
            for idx, stmt in enumerate(ssa_block.statements):
                for name in stmt.uses:
                    if name not in provably_unset or name in reported:
                        continue
                    def_block, def_idx = provably_unset[name]
                    in_def_block_after_def = bn == def_block and idx > def_idx
                    # ``_block_dominates`` is the same predicate used
                    # for the existence-narrowing pass; the def-block
                    # dominates exactly when every path to ``bn``
                    # passes through it, which guarantees the regexp /
                    # scan call has already executed (or its truth-arm
                    # has already been chosen) by the time ``bn`` runs.
                    dominated_use = bn != def_block and _block_dominates(ssa, def_block, bn)
                    if in_def_block_after_def or dominated_use:
                        reported.add(name)
                        result.append(ReadBeforeSet(block=bn, statement_index=idx, variable=name))

    return tuple(result)


# Unused variable detection


def _unused_variables(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    executable_blocks: set[str] | None = None,
    executable_edges: set[tuple[str, str]] | None = None,
    params: frozenset[str] = frozenset(),
    resolve_ctx=None,
    place_read_names: frozenset[str] | None = None,
    element_observed=None,
) -> tuple[UnusedVariable, ...]:
    """Find variables that are set but never used across the entire function.

    Unlike dead stores (W220) which detect individual assignments that are
    never read, this detects variables where *no* version is ever used —
    meaning the variable is entirely pointless.
    """
    considered = executable_blocks if executable_blocks is not None else set(cfg.blocks)
    if resolve_ctx is None:
        resolve_ctx = build_resolve_context(cfg)

    used_names = _collect_used_names(
        cfg,
        ssa,
        executable_blocks=executable_blocks,
        executable_edges=executable_edges,
        include_return_vars=True,
        resolve_ctx=resolve_ctx,
        place_read_names=place_read_names,
    )

    # Now find variables that are defined but never used.
    # Report at the first definition site.
    reported: set[str] = set()
    result: list[UnusedVariable] = []
    # Same element/dict overlap fallback as ``_dead_stores`` — so W211 and the
    # optimiser's O126 (which reads ``unused_variables``) agree with W220/O109
    # on element-granular and possibly-aliased writes.  Reuse the caller's shared
    # predicate (see _dead_stores) so the read-place index is built once.
    _element_observed = element_observed or _make_element_observed(
        cfg, ssa, resolve_ctx, considered
    )

    order = cfg.reverse_postorder()
    for bn in order:
        if bn not in considered:
            continue
        block = ssa.blocks.get(bn)
        if block is None:
            continue

        for idx, stmt in enumerate(block.statements):
            for name in stmt.defs:
                if name in used_names or name in reported:
                    continue
                if name in params:
                    continue
                if name.startswith("_"):
                    continue
                # Only report for safe (side-effect-free) assignments.
                ir_stmt = stmt.statement
                if isinstance(ir_stmt, IRBarrier):
                    continue
                if isinstance(ir_stmt, IRCall):
                    continue
                # Reportable only for a plain proc-local scalar/array place —
                # subsumes the `::`-prefix, dynamic-target, and
                # `escaping_names` guards (global/ns/upvar/instance/traced/dynamic
                # all resolve to a non-reportable Place).
                place = _def_place(ir_stmt, resolve_ctx)
                if place is None or not _reportable_local(place):
                    continue
                # Element/dict writes that overlap a read (incl. via a dynamic
                # alias) are observed even though the folded name is never read.
                if place.kind in (PlaceKind.ARRAY_ELEM, PlaceKind.DICT_PATH) and _element_observed(
                    place
                ):
                    continue
                reported.add(name)
                result.append(UnusedVariable(block=bn, statement_index=idx, variable=name))

    return tuple(result)


def _unused_parameters(
    cfg: CFGFunction,
    ssa: SSAFunction,
    params: frozenset[str],
    *,
    executable_blocks: set[str] | None = None,
    resolve_ctx=None,
    place_read_names: frozenset[str] | None = None,
) -> tuple[str, ...]:
    """Find proc parameters that are never read in the function body.

    Skips ``args`` (Tcl's variadic catch-all) and parameters whose name
    starts with ``_`` (conventional "intentionally unused" marker).
    """
    if not params:
        return ()

    used_names = _collect_used_names(
        cfg,
        ssa,
        executable_blocks=executable_blocks,
        include_return_vars=True,
        resolve_ctx=resolve_ctx,
        place_read_names=place_read_names,
    )

    result: list[str] = []
    for p in params:
        if p == "args":
            continue
        if p.startswith("_"):
            continue
        if p not in used_names:
            result.append(p)
    return tuple(result)


# Type propagation

_FLOAT_RE = re.compile(r"^[+-]?(\d+\.\d*|\.\d+)([eE][+-]?\d+)?\s*$")
# Tcl integer literals: decimal (handled via _DECIMAL_INT_RE elsewhere)
# plus the explicit-prefix forms hex (``0x...``) and binary (``0b...``).
# Tcl 9 dropped the legacy "leading-0 means octal" rule so we don't try
# to recognise it here (would be dialect-dependent and error-prone).
_HEX_INT_RE = re.compile(r"^[+-]?0[xX][0-9a-fA-F]+\s*$")
_BIN_INT_RE = re.compile(r"^[+-]?0[bB][01]+\s*$")

_TYPE_UNKNOWN = TypeLattice.unknown()
_TYPE_OVERDEFINED = TypeLattice.overdefined()


def _return_type_for_command(
    command: str,
    args: tuple[str, ...],
    known_classes: frozenset[str] = frozenset(),
    namespace: str = "::",
) -> TypeLattice:
    """Look up the return type of a command from TYPE_HINTS.

    When *known_classes* is provided, user-defined TclOO class names
    followed by ``new`` or ``create`` are recognised as returning
    ``TypeLattice.object_of(class_name)``.  *namespace* is the call site's
    enclosing namespace, used to resolve a *relative* class name (``[Foo new]``
    inside ``namespace eval ns`` → ``::ns::Foo``).
    """
    hint = TYPE_HINTS.get(command)
    if hint is None:
        # Check TclOO / snit class constructors.  ``new``/``create`` cover
        # TclOO and snit's explicit ``Type create name``; ``%AUTO%`` and a
        # leading ``.`` (a Tk widget path) cover snit's other instance-creation
        # idioms (``Type %AUTO%``, ``Widget .path``).
        #
        # Known imprecision (low impact): only ``snit::widget`` /
        # ``snit::widgetadaptor`` support ``Type .path`` instance creation; a
        # plain ``snit::type Foo`` does not, so ``Foo .config`` is not really an
        # instance and should not be typed OBJECT here.  ``known_classes`` is a
        # flat name set with no definer-kind, so the ``.``-prefix branch can't
        # distinguish them without threading a widget-class subset through every
        # ``_return_type_for_command`` caller.  Deferred — ``Type .x`` on a
        # non-widget snit type is virtually never written, and the only effect is
        # an over-suppressed W307 on the (non-existent) result's dispatch.
        if args and (
            args[0] in ("new", "create") or args[0] == "%AUTO%" or args[0].startswith(".")
        ):
            # Try the command as-is (qualified) and with :: prefix.
            if known_classes:
                if command in known_classes:
                    return TypeLattice.object_of(command)
                qualified = f"::{command}" if not command.startswith("::") else command
                if qualified in known_classes:
                    return TypeLattice.object_of(qualified)
                # A relative name resolves against the call-site namespace:
                # ``[Foo new]`` inside ``namespace eval ns`` constructs
                # ``::ns::Foo`` (tclsh), so try that spelling too.
                if namespace != "::" and not command.startswith("::"):
                    ns_qualified = _normalise_qualified_name(f"{namespace}::{command}")
                    if ns_qualified in known_classes:
                        return TypeLattice.object_of(ns_qualified)
            # ``new`` is unique to TclOO — treat unknown commands calling
            # ``new`` as external class constructors so the type lattice
            # suppresses W307 for ``$obj method`` patterns.
            if args[0] == "new":
                return TypeLattice.object_of(command)
        return _TYPE_OVERDEFINED
    if isinstance(hint, SubcommandTypeHint):
        if not args:
            return _TYPE_OVERDEFINED
        sub_hint = hint.subcommands.get(args[0])
        if sub_hint is None or sub_hint.return_type is None:
            return _TYPE_OVERDEFINED
        return TypeLattice.of(sub_hint.return_type)
    if isinstance(hint, CommandTypeHint):
        if hint.return_type is None:
            return _TYPE_OVERDEFINED
        return TypeLattice.of(hint.return_type)
    return _TYPE_OVERDEFINED


def _literal_type(text: str) -> TypeLattice:
    """Infer the intrep type from a literal string value."""
    stripped = text.strip()
    if _DECIMAL_INT_RE.fullmatch(stripped):
        return TypeLattice.of(TclType.INT)
    # Tcl recognises hex (``0x...``) and binary (``0b...``) prefixes as
    # integer literals — they convert to INT on first numeric use.
    # Typing as INT prevents string→int shimmer FPs on patterns like
    # ``variable initial_n 0x80; set n \$initial_n; incr n`` (idna.tcl).
    if _HEX_INT_RE.fullmatch(stripped) or _BIN_INT_RE.fullmatch(stripped):
        return TypeLattice.of(TclType.INT)
    if _FLOAT_RE.fullmatch(stripped):
        return TypeLattice.of(TclType.DOUBLE)
    if stripped.lower() in _BOOL_LITERALS:
        return TypeLattice.of(TclType.BOOLEAN)
    return TypeLattice.of(TclType.STRING)


def _evaluate_type_def(
    stmt: IRStatement,
    ssa_stmt: SSAStatement,
    values: dict[SSAValueKey, LatticeValue],
    types: dict[SSAValueKey, TypeLattice],
    known_classes: frozenset[str] = frozenset(),
    namespace: str = "::",
) -> TypeLattice:
    """Determine the type of a variable definition."""
    match stmt:
        case IRAssignConst(value=value):
            return _literal_type(value)

        case IRAssignExpr(expr=expr):
            # Walk the expression AST with operator-aware type rules.
            var_types_for_expr: dict[str, TypeLattice] = {}
            for name, ver in ssa_stmt.uses.items():
                if ver > 0:
                    t = types.get((name, ver))
                    if t is not None:
                        var_types_for_expr[name] = t
            return infer_expr_type(expr, var_types_for_expr)

        case IRAssignValue(value=value):
            stripped = value.strip()
            # Pure variable reference: inherit type
            if is_pure_var_ref(stripped):
                name = _normalise_var_name(stripped)
                ver = ssa_stmt.uses.get(name, 0)
                if ver > 0:
                    return types.get((name, ver), _TYPE_UNKNOWN)
                return _TYPE_UNKNOWN
            # Command substitution: [cmd ...]
            if stripped.startswith("[") and stripped.endswith("]"):
                cmd_text = stripped[1:-1].strip()
                parts = cmd_text.split(None, 1)
                if parts:
                    cmd_name = parts[0]
                    cmd_args = tuple(parts[1].split()) if len(parts) > 1 else ()
                    return _return_type_for_command(cmd_name, cmd_args, known_classes, namespace)
            # String interpolation or complex value → STRING
            if "$" in value or "[" in value:
                return TypeLattice.of(TclType.STRING)
            # Plain literal
            return _literal_type(value)

        case IRIncr():
            return TypeLattice.of(TclType.INT)

        case IRCall(command=cmd, args=call_args) if stmt.defs:
            return _return_type_for_command(cmd, call_args, known_classes, namespace)

        case _:
            return _TYPE_OVERDEFINED


def _type_propagation(
    cfg: CFGFunction,
    ssa: SSAFunction,
    values: dict[SSAValueKey, LatticeValue],
    executable_blocks: set[str],
    executable_edges: set[tuple[str, str]],
    known_classes: frozenset[str] = frozenset(),
) -> dict[SSAValueKey, TypeLattice]:
    """Run type propagation over the SSA graph."""
    preds = _compute_predecessors(cfg)

    # The function's own namespace (``::ns`` for a proc ``::ns::p``; ``::`` for
    # top-level or a global proc) resolves *relative* class names at command
    # substitutions inside it — ``[Foo new]`` in ``::ns::p`` constructs
    # ``::ns::Foo`` (tclsh), so object typing must look there.
    namespace = _normalise_qualified_name(cfg.name).rsplit("::", 1)[0] or "::"

    types: dict[SSAValueKey, TypeLattice] = {}
    order = cfg.reverse_postorder()
    # Scope-alias declaration commands (``global``/``variable``/``upvar``/
    # ``namespace upvar``) import an *externally-determined* variable; the
    # alias's intrep is unknown (the value lives in another scope), so its def
    # must be OVERDEFINED rather than the command's nominal STRING return type.
    # Computed once here to keep the registry scan out of the hot per-def loop.
    alias_cmds = scope_alias_commands()
    # Forward dataflow worklist: when a value's type changes, re-enqueue
    # only the blocks that read it (precomputed def→use map) instead of
    # re-scanning every block each pass.
    deps = value_use_blocks(ssa)
    changed_keys: list[SSAValueKey] = []

    def set_type(key: SSAValueKey, candidate: TypeLattice) -> bool:
        old = types.get(key, _TYPE_UNKNOWN)
        merged = type_join(old, candidate)
        if merged != old:
            types[key] = merged
            changed_keys.append(key)
            return True
        return False

    worklist: list[str] = [bn for bn in order if bn in executable_blocks]
    queued: set[str] = set(worklist)
    yield_counter = 0
    while worklist:
        bn = worklist.pop()
        queued.discard(bn)
        ssa_block = ssa.blocks.get(bn)
        if ssa_block is None:
            continue
        changed_keys.clear()

        # Phi nodes.  Sort the predecessor list: ``preds`` holds *sets*, so an
        # unsorted iteration order is hash-seed-dependent — and ``type_join`` is
        # not associative across >2 incompatible types (numeric promotion can
        # collapse two numerics into one, so the order in which a STRING and two
        # numerics merge decides SHIMMERED-vs-OVERDEFINED).  Sorting makes the
        # fold deterministic across runs.
        incoming_exec_preds = sorted(p for p in preds.get(bn, set()) if (p, bn) in executable_edges)
        for phi in ssa_block.phis:
            if bn == cfg.entry:
                continue
            if not incoming_exec_preds:
                continue
            phi_type = _TYPE_UNKNOWN
            for pred in incoming_exec_preds:
                incoming_ver = phi.incoming.get(pred, 0)
                if incoming_ver <= 0:
                    continue
                phi_type = type_join(
                    phi_type,
                    types.get((phi.name, incoming_ver), _TYPE_UNKNOWN),
                )
            set_type((phi.name, phi.version), phi_type)

        # Statements
        for s in ssa_block.statements:
            stmt = s.statement
            if isinstance(stmt, IRBarrier):
                # Barriers widen all defs to OVERDEFINED
                for var, ver in s.defs.items():
                    set_type((var, ver), _TYPE_OVERDEFINED)
                continue
            # canonicalisation: bucket-iii — ``namespace upvar`` lowers to an
            # IRCall whose ``command`` is the bare ``namespace`` (the compound
            # lives in ``scope_alias_commands()`` as ``"namespace upvar"``), so
            # the subcommand must be matched on the literal here.
            if isinstance(stmt, IRCall) and (
                stmt.command in alias_cmds
                # canonicalisation: bucket-iii — ``namespace upvar`` lowers to command ``namespace``
                or (stmt.command == "namespace" and stmt.args and stmt.args[0] == "upvar")
            ):
                # Alias declaration: the imported variable's intrep is external
                # and unknown — widen to OVERDEFINED so use-site/merge shimmer
                # checks do not fire on a nominally-STRING-typed alias.
                for var, ver in s.defs.items():
                    set_type((var, ver), _TYPE_OVERDEFINED)
                continue
            for var, ver in s.defs.items():
                inferred = _evaluate_type_def(stmt, s, values, types, known_classes, namespace)
                set_type((var, ver), inferred)

        for key in changed_keys:
            for ub in deps.get(key, ()):
                if ub in executable_blocks and ub not in queued:
                    worklist.append(ub)
                    queued.add(ub)
        yield_counter += 1
        if yield_counter % 256 == 0:
            time.sleep(0)  # Yield GIL periodically

    return types


def analyse_function(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    params: frozenset[str] = frozenset(),
    param_constants: dict[SSAValueKey, LatticeValue] | None = None,
    known_classes: frozenset[str] = frozenset(),
    force_guard: bool = False,
) -> FunctionAnalysis:
    # Complexity guard: a pathologically large body (almost always machine-
    # generated — e.g. fumagic's 33 940-block nested-if/emit dispatch tree)
    # would cost tens of seconds of SCCP/type/taint/liveness dataflow for
    # near-zero useful findings.  Skip deep analysis above the block ceiling and
    # return a trivial (all-reachable, no-facts) analysis; downstream per-proc
    # diagnostic passes then emit nothing for it.  Callers that want to surface
    # this consult ``is_complexity_guarded(cfg)``.  *force_guard* lets the caller
    # skip the dataflow when it has already decided the body is too large (e.g.
    # byte-huge but block-light), matching the block-count short-circuit.
    if force_guard or len(cfg.blocks) > _COMPLEXITY_GUARD_BLOCKS:
        return FunctionAnalysis(
            live_in={},
            live_out={},
            dead_stores=(),
            unreachable_blocks=set(),
            constant_branches=(),
            values={},
        )
    values, executable_blocks, executable_edges, constant_branches = _sccp(
        cfg, ssa, param_constants=param_constants, params=params
    )
    inferred_types = _type_propagation(
        cfg, ssa, values, executable_blocks, executable_edges, known_classes
    )

    from .rendered_properties import rendered_properties_propagation

    rendered = rendered_properties_propagation(cfg, ssa, executable_blocks, executable_edges)

    from .taint import taint_propagation  # late import to avoid circular dependency

    inferred_taints = taint_propagation(cfg, ssa, executable_blocks, executable_edges)

    live_in, live_out = _liveness(cfg, ssa)
    resolve_ctx = build_resolve_context(cfg)
    # Structural read-name recovery (command-subs, dynamic names, eval bodies) —
    # computed once via the Place bridge and shared by every name-level
    # suppression consumer below.
    place_read_names = _place_read_names(cfg, resolve_ctx, set(executable_blocks))
    # The element/dict overlap predicate walks read_places + builds the bucketed
    # index once; share it between _dead_stores (W220) and _unused_variables
    # (W211) so that walk isn't repeated (lazy — built on first element/dict hit).
    element_observed = _make_element_observed(cfg, ssa, resolve_ctx, set(executable_blocks))
    dead = _dead_stores(
        cfg,
        ssa,
        executable_blocks=executable_blocks,
        executable_edges=executable_edges,
        resolve_ctx=resolve_ctx,
        element_observed=element_observed,
    )
    reachable_cfg = set(cfg.blocks)
    unreachable = reachable_cfg - executable_blocks

    rbs = _read_before_set(
        cfg,
        ssa,
        executable_blocks=executable_blocks,
        params=params,
        values=values,
    )
    unused = _unused_variables(
        cfg,
        ssa,
        executable_blocks=executable_blocks,
        executable_edges=executable_edges,
        params=params,
        resolve_ctx=resolve_ctx,
        place_read_names=place_read_names,
        element_observed=element_observed,
    )
    unused_p = _unused_parameters(
        cfg,
        ssa,
        params,
        executable_blocks=executable_blocks,
        resolve_ctx=resolve_ctx,
        place_read_names=place_read_names,
    )

    # Build def-use chains and memory-SSA (graceful degradation on error).
    # Failures are logged but do not prevent the rest of analysis from
    # completing — downstream consumers check for None.
    import logging as _logging

    _log = _logging.getLogger(__name__)

    du_result = None
    mem_ssa = None
    try:
        from .def_use import build_def_use_chains

        du_result = build_def_use_chains(ssa, cfg=cfg)
    except Exception:
        _log.warning("def-use chain construction failed", exc_info=True)
    try:
        from .memory_ssa import build_memory_ssa

        mem_ssa = build_memory_ssa(ssa)
    except Exception:
        _log.warning("memory-SSA construction failed", exc_info=True)

    return FunctionAnalysis(
        live_in=live_in,
        live_out=live_out,
        dead_stores=dead,
        unreachable_blocks=unreachable,
        constant_branches=constant_branches,
        values=values,
        types=inferred_types,
        taints=inferred_taints,
        rendered_props=rendered,
        read_before_set=rbs,
        unused_variables=unused,
        unused_params=unused_p,
        def_use_chains=du_result,
        memory_ssa=mem_ssa,
    )


def analyse_ir_module(
    ir_module: IRModule,
    known_classes: frozenset[str] = frozenset(),
) -> ModuleAnalysis:
    cfg_module = build_cfg(ir_module)
    top_ssa = build_ssa(cfg_module.top_level)
    top = analyse_function(cfg_module.top_level, top_ssa, known_classes=known_classes)

    procs: dict[str, FunctionAnalysis] = {}
    for qname, cfg in cfg_module.procedures.items():
        ssa = build_ssa(cfg)
        ir_proc = ir_module.procedures.get(qname)
        proc_params = frozenset(ir_proc.params) if ir_proc else frozenset()
        procs[qname] = analyse_function(cfg, ssa, params=proc_params, known_classes=known_classes)

    return ModuleAnalysis(top_level=top, procedures=procs)


def analyse_source(source: str) -> ModuleAnalysis:
    """Lower source to IR and run Phase 3 core analyses."""
    ir_module = lower_to_ir(source)
    return analyse_ir_module(ir_module)
