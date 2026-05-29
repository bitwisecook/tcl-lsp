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
from compiler.registry.runtime import FOLD_HINTS, FOLD_SUBCOMMAND_HINTS, TYPE_HINTS
from compiler.registry.type_hints import CommandTypeHint, SubcommandTypeHint
from shared.naming import normalise_var_name as _normalise_var_name
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
    ExprTernary,
    ExprUnary,
    UnaryOp,
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
    IRExprEval,
    IRIncr,
    IRModule,
    IRReturn,
    IRStatement,
)
from .lowering import lower_to_ir
from .ssa import SSAFunction, SSAStatement, SSAValueKey, build_ssa, value_use_blocks
from .static_loops import (
    evaluate_expr_with_constants,
    summarise_static_for_ir,
)
from .tcl_constants import TCL_BOOL_LITERALS as _BOOL_LITERALS
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


# A plain, unqualified local scalar name — the only shape whose existence is
# locally decidable.  Namespace-qualified (``::ns::x``, ``ns::x``), array
# element (``a(k)``), and dynamic (``$name``) targets are excluded.
_EXISTENCE_LOCAL_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


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
    if not _EXISTENCE_LOCAL_RE.match(raw_target):
        return None
    name = _normalise_var_name(raw_target)
    return name or None


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


def _is_existence_transparent(stmt: IRStatement) -> bool:
    """True when *stmt* cannot create or destroy a local variable in a way the
    analysis cannot see — the precondition for folding existence checks."""
    if isinstance(stmt, (IRAssignConst, IRAssignExpr, IRAssignValue, IRIncr, IRReturn, IRExprEval)):
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
        return True
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


def _escaping_var_names(cfg: CFGFunction) -> frozenset[str]:
    """Local names that alias storage outside the current frame.

    Covers ``upvar`` (incl. ``upvar #0`` and multi-pair forms),
    ``namespace upvar``, ``global``, and ``variable``.  A write to such a
    name is observable in another scope (the caller's frame, a namespace,
    or the global frame), so it must never be reported as a dead store
    (W220) or set-but-never-used variable (W211), nor eliminated by DCE,
    even when the local analysis sees no local read.

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
            for i in upvar_local_declaration_indices(stmt.command, args):
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
    return frozenset(names)


def _dead_stores(
    cfg: CFGFunction,
    ssa: SSAFunction,
    *,
    executable_blocks: set[str] | None = None,
    executable_edges: set[tuple[str, str]] | None = None,
    escaping_names: frozenset[str] = frozenset(),
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
                # upvar/global/variable aliases escape the local frame —
                # a write is observable in another scope.
                if n in escaping_names:
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
                nm = _normalise_var_name(parsed[1])
                if nm:
                    out.add(nm)
            else:
                _collect_existence_in_word(tok.text, out)


def _existence_checks_in_expr(expr: ExprNode) -> set[str]:
    """Names existence-checked (``info``/``array exists``) anywhere in *expr*."""
    names: set[str] = set()
    cmds: list[ExprCommand] = []
    _iter_expr_commands(expr, cmds)
    for c in cmds:
        parsed = _parse_existence_check(c.text)
        if parsed is not None:
            nm = _normalise_var_name(parsed[1])
            if nm:
                names.add(nm)
    return names


def _existence_checks_in_stmt(stmt: IRStatement) -> set[str]:
    """Names existence-checked by *stmt* itself — the check reference does not
    read the variable's value, so it is never a read-before-set."""
    names: set[str] = set()
    if isinstance(stmt, (IRCall, IRBarrier)):
        args = stmt.args
        if (
            stmt.canonical_command in ("::info", "::array")
            and len(args) >= 2
            and args[0] == "exists"
        ):
            nm = _normalise_var_name(args[1])
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


def _existence_implications(expr: ExprNode) -> tuple[set[str], set[str]]:
    """Return ``(exists_if_true, exists_if_false)`` — names a condition proves
    to exist when it evaluates true vs false.

    A bare ``[info exists X]`` proves ``X`` when true; ``!`` swaps the arms;
    ``&&`` proves its operands' positive facts only when the whole thing is
    true; ``||`` proves their negative facts only when the whole thing is
    false.  Everything else proves nothing.
    """
    if isinstance(expr, ExprCommand):
        parsed = _parse_existence_check(expr.text)
        name = _existence_scalar_name(parsed[1]) if parsed is not None else None
        if name is not None:
            return {name}, set()
        return set(), set()
    if isinstance(expr, ExprUnary) and expr.op in (UnaryOp.NOT, UnaryOp.WORD_NOT):
        true_set, false_set = _existence_implications(expr.operand)
        return false_set, true_set
    if isinstance(expr, ExprBinary):
        left_true, left_false = _existence_implications(expr.left)
        right_true, right_false = _existence_implications(expr.right)
        if expr.op in (BinOp.AND, BinOp.WORD_AND):
            return left_true | right_true, set()
        if expr.op in (BinOp.OR, BinOp.WORD_OR):
            return set(), left_false | right_false
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
                and stmt.canonical_command == "::dict"
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
                if _has_dict_with and name not in explicitly_defined:
                    continue
                if name.startswith("::") or name.startswith("static::"):
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
                if name.startswith("::") or name.startswith("static::"):
                    continue
                if name in block_known or name in cond_checks:
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
    escaping_names: frozenset[str] = frozenset(),
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
                # Global variables are consumed externally.
                if name.startswith("::"):
                    continue
                # upvar/global/variable aliases escape the local frame —
                # a write is observable in another scope.
                if name in escaping_names:
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
    order = cfg.reverse_postorder()
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
            set_type((phi.name, phi.version), phi_type)

        # Statements
        for s in ssa_block.statements:
            stmt = s.statement
            if isinstance(stmt, IRBarrier):
                # Barriers widen all defs to OVERDEFINED
                for var, ver in s.defs.items():
                    set_type((var, ver), _TYPE_OVERDEFINED)
                continue
            for var, ver in s.defs.items():
                inferred = _evaluate_type_def(stmt, s, values, types, known_classes)
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
) -> FunctionAnalysis:
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
    escaping_names = _escaping_var_names(cfg)
    dead = _dead_stores(
        cfg,
        ssa,
        executable_blocks=executable_blocks,
        executable_edges=executable_edges,
        escaping_names=escaping_names,
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
        escaping_names=escaping_names,
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
