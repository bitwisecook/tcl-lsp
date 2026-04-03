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

from __future__ import annotations

import re
import time
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import TYPE_CHECKING

from ..commands.registry.runtime import FOLD_HINTS, FOLD_SUBCOMMAND_HINTS, TYPE_HINTS
from ..commands.registry.type_hints import CommandTypeHint, SubcommandTypeHint
from ..common.naming import normalise_var_name as _normalise_var_name
from ..parsing.lexer import TclLexer
from ..parsing.tokens import TokenType
from .cfg import CFGBranch, CFGFunction, CFGGoto, CFGReturn, build_cfg
from .eval_helpers import DECIMAL_INT_RE as _DECIMAL_INT_RE
from .expr_ast import (
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprNode,
    ExprRaw,
    ExprTernary,
    ExprUnary,
    expr_text,
    vars_in_expr_node,
)
from .expr_types import infer_expr_type
from .ir import (
    IRAssignConst,
    IRAssignExpr,
    IRAssignValue,
    IRBarrier,
    IRCall,
    IRIncr,
    IRModule,
    IRStatement,
)
from .lowering import lower_to_ir
from .ssa import SSAFunction, SSAStatement, SSAValueKey, build_ssa
from .static_loops import (
    evaluate_expr_with_constants,
    summarise_static_for_ir,
)
from .tcl_expr_eval import _split_tcl_list, eval_tcl_expr
from .types import TclType, TypeLattice, type_join
from .value_shapes import is_pure_var_ref
from .var_refs import VarReferenceScanner

if TYPE_CHECKING:
    from .def_use import DefUseResult
    from .memory_ssa import MemorySSAFunction
    from .rendered_properties import RenderedValueProps
    from .taint import TaintLattice

_RETURN_VAR_SCANNER = VarReferenceScanner()


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


def _cfg_order(cfg: CFGFunction) -> list[str]:
    seen: set[str] = set()
    order: list[str] = []
    stack = [cfg.entry]
    while stack:
        bn = stack.pop()
        if bn in seen or bn not in cfg.blocks:
            continue
        seen.add(bn)
        order.append(bn)
        match cfg.blocks[bn].terminator:
            case CFGGoto(target=target):
                succs = [target]
            case CFGBranch(true_target=tt, false_target=ft):
                succs = [tt, ft]
            case _:
                succs = []
        for succ in reversed(succs):
            stack.append(succ)
    for bn in cfg.blocks:
        if bn not in seen:
            order.append(bn)
    return order


def _parse_literal_value(text: str) -> int | str:
    stripped = text.strip()
    if _DECIMAL_INT_RE.fullmatch(stripped):
        try:
            return int(stripped)
        except ValueError:
            pass
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

    Like ``_fold_interpolation`` but handles ``CONSTSET`` variables by
    computing the Cartesian product of all possible interpolations.
    Returns ``None`` if any variable is unresolvable or the result set
    would be too large.
    """
    # Collect pieces: each piece is either a literal string or a set of
    # possible string values (from CONST or CONSTSET variables).
    pieces: list[str | frozenset[str]] = []
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
                return None
            str_vals = frozenset(str(v) for v in vs)
            if len(str_vals) == 1:
                pieces.append(next(iter(str_vals)))
            else:
                pieces.append(str_vals)
        elif tok.type is TokenType.CMD:
            return None
        else:
            pieces.append(tok.text)

    # Compute the Cartesian product of all pieces.
    results: list[str] = [""]
    for piece in pieces:
        if isinstance(piece, str):
            results = [r + piece for r in results]
        else:
            new_results: list[str] = []
            for r in results:
                for v in piece:
                    new_results.append(r + v)
            results = new_results
            if len(results) > _MAX_CONSTSET_SIZE:
                return None

    return frozenset(results) if results else None


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


_LIST_CMD_RE = re.compile(r"^\s*\[\s*list\s+(.*?)\s*\]\s*$", re.DOTALL)


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
    m = _LIST_CMD_RE.match(stripped)
    if m:
        args_text = m.group(1)
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


_CMD_SUBST_RE = re.compile(r"^\s*\[\s*(\S+)(?:\s+(.*?))?\s*\]\s*$", re.DOTALL)


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
    m = _CMD_SUBST_RE.match(value)
    if m is None:
        return None

    cmd_name = m.group(1)
    args_text = m.group(2) or ""

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

    if fold_fn is None:
        return None

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
    # Reject results containing Tcl-special characters that would
    # change semantics if the optimiser inlines them as bare words.
    if any(ch in result for ch in ';\n[$"\\'):
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
        env = summarise_static_for_ir(loop_stmt, initial_constants=start_env)
        if env is None:
            return None
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


def _evaluate_branch_decision(
    cfg: CFGFunction,
    ssa: SSAFunction,
    block_name: str,
    condition: ExprNode,
    values: dict[SSAValueKey, LatticeValue],
) -> bool | None:
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
) -> tuple[
    dict[SSAValueKey, LatticeValue], set[str], set[tuple[str, str]], tuple[ConstantBranch, ...]
]:
    preds = _compute_predecessors(cfg)

    executable_blocks: set[str] = {cfg.entry} if cfg.entry in cfg.blocks else set()
    executable_edges: set[tuple[str, str]] = set()
    values: dict[SSAValueKey, LatticeValue] = {}
    if param_constants:
        for key, lv in param_constants.items():
            values[key] = lv
    order = _cfg_order(cfg)

    def set_value(key: SSAValueKey, candidate: LatticeValue) -> bool:
        old = values.get(key, UNKNOWN)
        merged = _join(old, candidate)
        if merged != old:
            values[key] = merged
            return True
        return False

    changed = True
    while changed:
        changed = False
        for bn in order:
            if bn not in executable_blocks:
                continue
            ssa_block = ssa.blocks[bn]

            incoming_exec_preds = [p for p in preds.get(bn, set()) if (p, bn) in executable_edges]
            for phi in ssa_block.phis:
                if bn == cfg.entry:
                    continue
                if not incoming_exec_preds:
                    continue
                phi_val = UNKNOWN
                for pred in incoming_exec_preds:
                    incoming_ver = phi.incoming.get(pred, 0)
                    if incoming_ver <= 0:
                        continue
                    phi_val = _join(phi_val, values.get((phi.name, incoming_ver), UNKNOWN))
                if set_value((phi.name, phi.version), phi_val):
                    changed = True

            for s in ssa_block.statements:
                if isinstance(s.statement, IRBarrier):
                    # Barriers can modify any variable — widen all
                    # currently-tracked values to OVERDEFINED.
                    for key in list(values):
                        if set_value(key, OVERDEFINED):
                            changed = True
                    continue
                for var, ver in s.defs.items():
                    val = _evaluate_def(s.statement, s, values)
                    if set_value((var, ver), val):
                        changed = True

            match cfg.blocks[bn].terminator:
                case CFGGoto(target=target):
                    edge = (bn, target)
                    if edge not in executable_edges:
                        executable_edges.add(edge)
                        changed = True
                    if target in cfg.blocks and target not in executable_blocks:
                        executable_blocks.add(target)
                        changed = True
                case CFGBranch(condition=condition, true_target=tt, false_target=ft):
                    decision = _evaluate_branch_decision(
                        cfg,
                        ssa,
                        bn,
                        condition,
                        values,
                    )
                    if decision is True:
                        targets = (tt,)
                    elif decision is False:
                        targets = (ft,)
                    else:
                        targets = (tt, ft)
                    for tgt in targets:
                        edge = (bn, tgt)
                        if edge not in executable_edges:
                            executable_edges.add(edge)
                            changed = True
                        if tgt in cfg.blocks and tgt not in executable_blocks:
                            executable_blocks.add(tgt)
                            changed = True
                case _:
                    pass
        time.sleep(0)  # Yield GIL between fixed-point iterations

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
    order = list(reversed(_cfg_order(cfg)))

    changed = True
    while changed:
        changed = False
        for bn in order:
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
            if new_in != live_in[bn] or out != live_out[bn]:
                live_in[bn] = new_in
                live_out[bn] = out
                changed = True
        time.sleep(0)  # Yield GIL between fixed-point iterations

    return live_in, live_out


def _vars_in_return(value: str) -> set[str]:
    """Extract variable names from a return value string.

    Uses ``VarReferenceScanner`` so that command substitutions like
    ``[string length $x]`` are recursed into correctly.
    """
    return set(_RETURN_VAR_SCANNER.scan_script(value))


def _collect_used_names(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    executable_blocks: set[str] | None = None,
    executable_edges: set[tuple[str, str]] | None = None,
    include_return_vars: bool = False,
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

    return used_names


def _dead_stores(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    executable_blocks: set[str] | None = None,
    executable_edges: set[tuple[str, str]] | None = None,
) -> tuple[DeadStore, ...]:
    considered_blocks = set(executable_blocks) if executable_blocks is not None else set(cfg.blocks)
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
                # Global variables are consumed externally.
                if n.startswith("::"):
                    continue
                ir_stmt = stmt.statement
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


def _read_before_set(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    executable_blocks: set[str] | None = None,
    params: frozenset[str] = frozenset(),
) -> tuple[ReadBeforeSet, ...]:
    """Find variables that are read before being set.

    A variable use with SSA version 0 means no prior definition was found
    during SSA renaming — the variable is read before set on that path.
    """
    considered = executable_blocks if executable_blocks is not None else set(cfg.blocks)
    skip = _IMPLICIT_VARS | params

    # dict with/update creates local variables from dict keys at runtime.
    # We cannot know which variables statically, so suppress
    # read-before-set for variables that could have been unpacked.
    # Collect dict variable names and mark the function as having dict-with.
    _has_dict_with = False
    for bn in considered:
        block = cfg.blocks.get(bn)
        if block is None:
            continue
        for stmt in block.statements:
            if (
                isinstance(stmt, IRBarrier)
                and stmt.command == "dict"
                and stmt.args
                and stmt.args[0] in ("with", "update")
            ):
                _has_dict_with = True
                # The dict variable itself is read by dict with.
                dict_var = stmt.args[1] if len(stmt.args) >= 2 else ""
                if dict_var:
                    skip = skip | {dict_var}

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

    # Track which version-0 variables we've already reported to avoid
    # duplicate warnings for the same variable in the same function.
    reported: set[str] = set()
    result: list[ReadBeforeSet] = []

    order = _cfg_order(cfg)
    for bn in order:
        if bn not in considered:
            continue
        ssa_block = ssa.blocks.get(bn)
        if ssa_block is None:
            continue

        for idx, stmt in enumerate(ssa_block.statements):
            for name, ver in stmt.uses.items():
                if ver != 0:
                    continue
                if name in skip or name in reported:
                    continue
                # In dict-with scopes, suppress for variables that have
                # no explicit definition — they were likely unpacked.
                if _has_dict_with and name not in explicitly_defined:
                    continue
                if name.startswith("::") or name.startswith("static::"):
                    continue
                reported.add(name)
                result.append(ReadBeforeSet(block=bn, statement_index=idx, variable=name))

        # Also check branch conditions for version-0 uses.
        # The condition's variable versions come from exit_versions.
        term = cfg.blocks[bn].terminator
        if isinstance(term, CFGBranch):
            for name in vars_in_expr_node(term.condition):
                ver = ssa_block.exit_versions.get(name, 0)
                if ver != 0:
                    continue
                if name in skip or name in reported:
                    continue
                if name.startswith("::") or name.startswith("static::"):
                    continue
                reported.add(name)
                # Use statement_index=-1 and block to signal condition-level use.
                # The range comes from the terminator itself.
                result.append(ReadBeforeSet(block=bn, statement_index=-1, variable=name))

    return tuple(result)


# Unused variable detection


def _unused_variables(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    executable_blocks: set[str] | None = None,
    executable_edges: set[tuple[str, str]] | None = None,
    params: frozenset[str] = frozenset(),
) -> tuple[UnusedVariable, ...]:
    """Find variables that are set but never used across the entire function.

    Unlike dead stores (W220) which detect individual assignments that are
    never read, this detects variables where *no* version is ever used —
    meaning the variable is entirely pointless.
    """
    considered = executable_blocks if executable_blocks is not None else set(cfg.blocks)

    used_names = _collect_used_names(
        cfg,
        ssa,
        executable_blocks=executable_blocks,
        executable_edges=executable_edges,
        include_return_vars=True,
    )

    # Now find variables that are defined but never used.
    # Report at the first definition site.
    reported: set[str] = set()
    result: list[UnusedVariable] = []

    order = _cfg_order(cfg)
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
                # Global variables are consumed externally.
                if name.startswith("::"):
                    continue
                # Only report for safe (side-effect-free) assignments.
                ir_stmt = stmt.statement
                if isinstance(ir_stmt, IRBarrier):
                    continue
                if isinstance(ir_stmt, IRCall):
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
_BOOL_LITERALS = frozenset({"true", "false", "yes", "no", "on", "off"})

_TYPE_UNKNOWN = TypeLattice.unknown()
_TYPE_OVERDEFINED = TypeLattice.overdefined()


def _return_type_for_command(
    command: str,
    args: tuple[str, ...],
    known_classes: frozenset[str] = frozenset(),
) -> TypeLattice:
    """Look up the return type of a command from TYPE_HINTS.

    When *known_classes* is provided, user-defined TclOO class names
    followed by ``new`` or ``create`` are recognised as returning
    ``TypeLattice.object_of(class_name)``.
    """
    hint = TYPE_HINTS.get(command)
    if hint is None:
        # Check TclOO class constructors.
        if args and args[0] in ("new", "create"):
            # Try the command as-is (qualified) and with :: prefix.
            if known_classes:
                if command in known_classes:
                    return TypeLattice.object_of(command)
                qualified = f"::{command}" if not command.startswith("::") else command
                if qualified in known_classes:
                    return TypeLattice.object_of(qualified)
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
                    return _return_type_for_command(cmd_name, cmd_args, known_classes)
            # String interpolation or complex value → STRING
            if "$" in value or "[" in value:
                return TypeLattice.of(TclType.STRING)
            # Plain literal
            return _literal_type(value)

        case IRIncr():
            return TypeLattice.of(TclType.INT)

        case IRCall(command=cmd, args=call_args) if stmt.defs:
            return _return_type_for_command(cmd, call_args, known_classes)

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

    types: dict[SSAValueKey, TypeLattice] = {}
    order = _cfg_order(cfg)

    def set_type(key: SSAValueKey, candidate: TypeLattice) -> bool:
        old = types.get(key, _TYPE_UNKNOWN)
        merged = type_join(old, candidate)
        if merged != old:
            types[key] = merged
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
                phi_type = _TYPE_UNKNOWN
                for pred in incoming_exec_preds:
                    incoming_ver = phi.incoming.get(pred, 0)
                    if incoming_ver <= 0:
                        continue
                    phi_type = type_join(
                        phi_type,
                        types.get((phi.name, incoming_ver), _TYPE_UNKNOWN),
                    )
                if set_type((phi.name, phi.version), phi_type):
                    changed = True

            # Statements
            for s in ssa_block.statements:
                stmt = s.statement
                if isinstance(stmt, IRBarrier):
                    # Barriers widen all defs to OVERDEFINED
                    for var, ver in s.defs.items():
                        if set_type((var, ver), _TYPE_OVERDEFINED):
                            changed = True
                    continue
                for var, ver in s.defs.items():
                    inferred = _evaluate_type_def(stmt, s, values, types, known_classes)
                    if set_type((var, ver), inferred):
                        changed = True
        time.sleep(0)  # Yield GIL between fixed-point iterations

    return types


def analyse_function(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    params: frozenset[str] = frozenset(),
    param_constants: dict[SSAValueKey, LatticeValue] | None = None,
    known_classes: frozenset[str] = frozenset(),
) -> FunctionAnalysis:
    values, executable_blocks, executable_edges, constant_branches = _sccp(
        cfg, ssa, param_constants=param_constants
    )
    inferred_types = _type_propagation(
        cfg, ssa, values, executable_blocks, executable_edges, known_classes
    )

    from .rendered_properties import rendered_properties_propagation

    rendered = rendered_properties_propagation(cfg, ssa, executable_blocks, executable_edges)

    from .taint import taint_propagation  # late import to avoid circular dependency

    inferred_taints = taint_propagation(cfg, ssa, executable_blocks, executable_edges)

    live_in, live_out = _liveness(cfg, ssa)
    dead = _dead_stores(
        cfg,
        ssa,
        executable_blocks=executable_blocks,
        executable_edges=executable_edges,
    )
    reachable_cfg = set(cfg.blocks)
    unreachable = reachable_cfg - executable_blocks

    rbs = _read_before_set(
        cfg,
        ssa,
        executable_blocks=executable_blocks,
        params=params,
    )
    unused = _unused_variables(
        cfg,
        ssa,
        executable_blocks=executable_blocks,
        executable_edges=executable_edges,
        params=params,
    )
    unused_p = _unused_parameters(
        cfg,
        ssa,
        params,
        executable_blocks=executable_blocks,
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
