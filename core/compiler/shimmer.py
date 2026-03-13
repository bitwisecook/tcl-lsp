"""Shimmer and type-thunking detection.

Analyses the SSA type lattice (built by ``core_analyses._type_propagation``)
to find places where a Tcl variable's internal representation is destroyed
and recreated as a different type ("shimmering") and, more expensively,
where this happens repeatedly across loop iterations ("type thunking").

Diagnostic codes
================

- **S100** (INFO): single shimmer outside a loop.
- **S101** (WARNING): shimmer inside a loop body (per-iteration cost).
- **S102** (WARNING): variable oscillates between two types across loop
  iterations (type thunking).
"""

from __future__ import annotations

import logging
from dataclasses import dataclass

from ..analysis.semantic_model import Range
from ..commands.registry.runtime import TYPE_HINTS
from ..commands.registry.type_hints import CommandTypeHint, SubcommandTypeHint
from ..common.naming import normalise_var_name as _normalise_var_name
from ..parsing.tokens import SourcePosition
from .cfg import CFGBranch, CFGFunction, CFGGoto
from .compilation_unit import CompilationUnit, ensure_compilation_unit
from .execution_intent import CommandSubstitutionIntent, FunctionExecutionIntent
from .expr_ast import (
    BinOp,
    ExprBinary,
    ExprNode,
    ExprRaw,
    ExprUnary,
    ExprVar,
    UnaryOp,
)
from .ir import IRAssignExpr, IRAssignValue, IRCall, IRIncr
from .ssa import SSAFunction, SSAValueKey, SSAVersion
from .types import TclType, TypeKind, TypeLattice
from .value_shapes import parse_command_substitution

log = logging.getLogger(__name__)


@dataclass(frozen=True, slots=True)
class ShimmerWarning:
    """A use-site where a variable's intrep is converted."""

    range: Range
    variable: str
    from_type: TclType
    to_type: TclType
    command: str
    in_loop: bool
    code: str  # S100 or S101
    message: str
    related_ranges: tuple[tuple[Range, str], ...] = ()


@dataclass(frozen=True, slots=True)
class ThunkingWarning:
    """A variable that oscillates between two types across loop iterations."""

    range: Range
    variable: str
    type_a: TclType
    type_b: TclType
    code: str  # S102
    message: str
    related_ranges: tuple[tuple[Range, str], ...] = ()


def _type_name(t: TclType) -> str:
    """Human-readable name for a Tcl intrep type."""
    return t.name.lower()


def _loop_body_blocks(cfg: CFGFunction) -> set[str]:
    """Return the set of block names that are inside a loop body.

    A block is "in a loop" if it is reachable from a for_header block
    and can reach the header again (i.e., is on a cycle).
    """
    # Find back edges: edges that go from a block to its dominator
    # (which, for `for` loops, is header → body → step → header).
    # Simpler approach: find all blocks on any cycle.
    succs: dict[str, list[str]] = {bn: [] for bn in cfg.blocks}
    for bn, block in cfg.blocks.items():
        match block.terminator:
            case CFGGoto(target=target):
                if target in succs:
                    succs[bn].append(target)
            case CFGBranch(true_target=tt, false_target=ft):
                if tt in succs:
                    succs[bn].append(tt)
                if ft in succs:
                    succs[bn].append(ft)

    # For each block, check if it can reach itself (is on a cycle).
    loop_blocks: set[str] = set()
    for start in cfg.blocks:
        if start in loop_blocks:
            continue
        # BFS from start's successors to see if we can return to start.
        visited: set[str] = set()
        frontier = list(succs.get(start, []))
        found = False
        while frontier:
            bn = frontier.pop()
            if bn == start:
                found = True
                break
            if bn in visited:
                continue
            visited.add(bn)
            frontier.extend(succs.get(bn, []))
        if found:
            # All blocks on the cycle are loop blocks.
            # Mark start and everything that can reach start and be reached from start.
            loop_blocks.add(start)
            loop_blocks |= visited & _blocks_reaching(succs, start)

    return loop_blocks


def _blocks_reaching(succs: dict[str, list[str]], target: str) -> set[str]:
    """Return all blocks that can reach *target* via forward edges."""
    # Build reverse graph.
    preds: dict[str, list[str]] = {bn: [] for bn in succs}
    for bn, successors in succs.items():
        for s in successors:
            if s in preds:
                preds[s].append(bn)
    # BFS backward from target.
    visited: set[str] = set()
    frontier = list(preds.get(target, []))
    while frontier:
        bn = frontier.pop()
        if bn in visited:
            continue
        visited.add(bn)
        frontier.extend(preds.get(bn, []))
    return visited


def _arg_type_for_call(command: str, args: tuple[str, ...], arg_index: int) -> TclType | None:
    """Look up the expected type for an argument of a command."""
    hint = TYPE_HINTS.get(command)
    if hint is None:
        return None
    if isinstance(hint, SubcommandTypeHint):
        if not args:
            return None
        sub_hint = hint.subcommands.get(args[0])
        if sub_hint is None:
            return None
        # arg_index is relative to the full args tuple (after command name).
        # For subcommand hints, arg 0 is the subcommand itself, so
        # the subcommand's arg positions start at 1 in the full tuple.
        sub_arg_index = arg_index - 1
        arg_hint = sub_hint.arg_types.get(sub_arg_index)
        return arg_hint.expected if arg_hint else None
    if isinstance(hint, CommandTypeHint):
        arg_hint = hint.arg_types.get(arg_index)
        return arg_hint.expected if arg_hint else None
    return None


def _check_command_substitution_intent(
    intent: CommandSubstitutionIntent,
    ssa_uses: dict[str, int],
    types: dict[SSAValueKey, TypeLattice],
    stmt_range: Range,
    in_loop: bool,
) -> list[ShimmerWarning]:
    """Check shimmer warnings for a pre-parsed command substitution intent."""
    if intent.shimmer_pressure <= 0:
        return []
    return _check_args_for_shimmer(
        intent.command,
        intent.args,
        ssa_uses,
        types,
        stmt_range,
        in_loop,
    )


def _check_args_for_shimmer(
    command: str,
    args: tuple[str, ...],
    ssa_uses: dict[str, int],
    types: dict[SSAValueKey, TypeLattice],
    stmt_range: Range,
    in_loop: bool,
) -> list[ShimmerWarning]:
    """Check a command invocation's arguments for type mismatches."""
    warnings: list[ShimmerWarning] = []

    for arg_idx, arg_text in enumerate(args):
        stripped = arg_text.strip()
        if not stripped.startswith("$"):
            continue

        var_name = _normalise_var_name(stripped)
        ver = ssa_uses.get(var_name, 0)
        if ver <= 0:
            continue

        var_type = types.get((var_name, ver))
        if var_type is None or var_type.kind is not TypeKind.KNOWN:
            continue
        assert var_type.tcl_type is not None  # guaranteed by KNOWN

        expected = _arg_type_for_call(command, args, arg_idx)
        if expected is None:
            continue

        if var_type.tcl_type is expected:
            continue

        # Numeric promotions are not shimmers.
        if _is_numeric_compatible(var_type.tcl_type, expected):
            continue

        code = "S101" if in_loop else "S100"
        severity = "loop " if in_loop else ""
        msg = (
            f"Shimmer: ${var_name} has intrep "
            f"{_type_name(var_type.tcl_type)} but {command} "
            f"expects {_type_name(expected)} ({severity}S{code[-3:]})"
        )
        warnings.append(
            ShimmerWarning(
                range=stmt_range,
                variable=var_name,
                from_type=var_type.tcl_type,
                to_type=expected,
                command=command,
                in_loop=in_loop,
                code=code,
                message=msg,
            )
        )
    return warnings


def _find_use_site_shimmers(
    fu_intent: FunctionExecutionIntent,
    cfg: CFGFunction,
    ssa: SSAFunction,
    types: dict[SSAValueKey, TypeLattice],
    executable_blocks: set[str],
    loop_blocks: set[str],
) -> list[ShimmerWarning]:
    """Find use-sites where a known-typed variable is passed to a command
    expecting a different type."""
    warnings: list[ShimmerWarning] = []

    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue
        in_loop = bn in loop_blocks

        for idx, ssa_stmt in enumerate(ssa_block.statements):
            if idx >= len(block.statements):
                continue
            stmt = block.statements[idx]

            if isinstance(stmt, IRCall):
                warnings.extend(
                    _check_args_for_shimmer(
                        stmt.command,
                        stmt.args,
                        ssa_stmt.uses,
                        types,
                        stmt.range,
                        in_loop,
                    )
                )
            elif isinstance(stmt, IRAssignValue):
                # Check command substitutions: set n [llength $x]
                intent = fu_intent.command_substitutions.get((bn, idx))
                if intent is not None:
                    warnings.extend(
                        _check_command_substitution_intent(
                            intent,
                            ssa_stmt.uses,
                            types,
                            stmt.range,
                            in_loop,
                        )
                    )
                else:
                    parsed = parse_command_substitution(stmt.value)
                    if parsed is not None:
                        cmd_name, cmd_args = parsed
                        warnings.extend(
                            _check_args_for_shimmer(
                                cmd_name,
                                cmd_args,
                                ssa_stmt.uses,
                                types,
                                stmt.range,
                                in_loop,
                            )
                        )
            elif isinstance(stmt, IRIncr):
                # incr reads the target variable and expects INT.
                var_name = _normalise_var_name(stmt.name)
                ver = ssa_stmt.uses.get(var_name, 0)
                if ver > 0:
                    var_type = types.get((var_name, ver))
                    if (
                        var_type is not None
                        and var_type.kind is TypeKind.KNOWN
                        and var_type.tcl_type is not None
                        and not _is_numeric_compatible(var_type.tcl_type, TclType.INT)
                        and var_type.tcl_type is not TclType.INT
                    ):
                        code = "S101" if in_loop else "S100"
                        severity = "loop " if in_loop else ""
                        msg = (
                            f"Shimmer: ${var_name} has intrep "
                            f"{_type_name(var_type.tcl_type)} but incr "
                            f"expects int ({severity}S{code[-3:]})"
                        )
                        warnings.append(
                            ShimmerWarning(
                                range=stmt.range,
                                variable=var_name,
                                from_type=var_type.tcl_type,
                                to_type=TclType.INT,
                                command="incr",
                                in_loop=in_loop,
                                code=code,
                                message=msg,
                            )
                        )

                # incr's increment argument also expects INT
                # (Tcl 9 path: TclIncrObj → GetNumberFromObj → TclParseNumber).
                if stmt.amount is not None:
                    amt = stmt.amount.strip()
                    if amt.startswith("$"):
                        amt_name = _normalise_var_name(amt)
                        amt_ver = ssa_stmt.uses.get(amt_name, 0)
                        if amt_ver > 0:
                            amt_type = types.get((amt_name, amt_ver))
                            if (
                                amt_type is not None
                                and amt_type.kind is TypeKind.KNOWN
                                and amt_type.tcl_type is not None
                                and amt_type.tcl_type is not TclType.INT
                                and not _is_numeric_compatible(amt_type.tcl_type, TclType.INT)
                            ):
                                code = "S101" if in_loop else "S100"
                                severity = "loop " if in_loop else ""
                                msg = (
                                    f"Shimmer: ${amt_name} has intrep "
                                    f"{_type_name(amt_type.tcl_type)} but incr "
                                    f"expects int ({severity}S{code[-3:]})"
                                )
                                warnings.append(
                                    ShimmerWarning(
                                        range=stmt.range,
                                        variable=amt_name,
                                        from_type=amt_type.tcl_type,
                                        to_type=TclType.INT,
                                        command="incr",
                                        in_loop=in_loop,
                                        code=code,
                                        message=msg,
                                    )
                                )

    return warnings


def _is_numeric_compatible(actual: TclType | None, expected: TclType | None) -> bool:
    """Return True if actual is a subtype of expected in the numeric hierarchy."""
    if actual is None or expected is None:
        return False
    if actual is expected:
        return True
    _NUMERIC_SUBS: dict[TclType, set[TclType]] = {
        TclType.INT: {TclType.BOOLEAN},
        TclType.NUMERIC: {TclType.BOOLEAN, TclType.INT, TclType.DOUBLE},
        TclType.DOUBLE: set(),
        TclType.BOOLEAN: set(),
    }
    subs = _NUMERIC_SUBS.get(expected)
    if subs is not None and actual in subs:
        return True
    return False


def _narrow_range(r: Range) -> Range:
    """Collapse a multi-line range to its first line.

    CFGGoto terminators carry the full body_range of if/while/for bodies
    which can span many lines.  When used as a phi definition range this
    produces unreasonably wide diagnostic spans that propagate outward
    through the phi chain.  Narrowing to the first line keeps the
    diagnostic anchored without losing location information.
    """
    if r.start.line == r.end.line:
        return r
    return Range(
        start=r.start,
        end=SourcePosition(
            line=r.start.line,
            character=r.start.character,
            offset=r.start.offset,
        ),
    )


def _build_def_ranges(
    ssa: SSAFunction,
    cfg: CFGFunction,
) -> dict[SSAValueKey, Range]:
    """Map each SSA definition to its source range."""
    result: dict[SSAValueKey, Range] = {}
    for bn, ssa_block in ssa.blocks.items():
        cfg_block = cfg.blocks.get(bn)
        # Statement definitions: direct range from the IR statement.
        for s in ssa_block.statements:
            r = getattr(s.statement, "range", None)
            if r is None:
                continue
            for name, ver in s.defs.items():
                result[(name, ver)] = r
        # Phi-defined versions: use first statement in the block as a
        # fallback (the phi itself has no source range).
        if ssa_block.phis and cfg_block is not None:
            phi_range = None
            if cfg_block.statements:
                phi_range = getattr(cfg_block.statements[0], "range", None)
            if phi_range is None and cfg_block.terminator is not None:
                phi_range = getattr(cfg_block.terminator, "range", None)
            if phi_range is not None:
                phi_range = _narrow_range(phi_range)
                for phi in ssa_block.phis:
                    result.setdefault((phi.name, phi.version), phi_range)
    return result


def _find_phi_shimmers(
    cfg: CFGFunction,
    ssa: SSAFunction,
    types: dict[SSAValueKey, TypeLattice],
    executable_blocks: set[str],
    loop_blocks: set[str],
    def_ranges: dict[SSAValueKey, Range] | None = None,
) -> list[ShimmerWarning]:
    """Find phi nodes where the merged type is SHIMMERED."""
    warnings: list[ShimmerWarning] = []
    if def_ranges is None:
        def_ranges = _build_def_ranges(ssa, cfg)

    for bn in executable_blocks:
        ssa_block = ssa.blocks.get(bn)
        if ssa_block is None:
            continue
        block = cfg.blocks.get(bn)
        if block is None:
            continue
        in_loop = bn in loop_blocks

        for phi in ssa_block.phis:
            key = (phi.name, phi.version)
            phi_type = types.get(key)
            if phi_type is None or phi_type.kind is not TypeKind.SHIMMERED:
                continue
            assert phi_type.tcl_type is not None and phi_type.from_type is not None

            # Classify incoming edges by the type they contribute.
            from_range: Range | None = None  # where it was from_type
            to_range: Range | None = None  # where it becomes to_type
            for _pred, incoming_ver in phi.incoming.items():
                if incoming_ver <= 0:
                    continue
                inc_type = types.get((phi.name, incoming_ver))
                if inc_type is None or inc_type.kind is TypeKind.UNKNOWN:
                    continue
                inc_range = def_ranges.get((phi.name, incoming_ver))
                if inc_range is None:
                    continue
                if inc_type.kind is TypeKind.KNOWN:
                    if inc_type.tcl_type is phi_type.tcl_type and to_range is None:
                        to_range = inc_range
                    elif inc_type.tcl_type is phi_type.from_type and from_range is None:
                        from_range = inc_range
                elif inc_type.kind is TypeKind.SHIMMERED:
                    # A SHIMMERED incoming version carries both types;
                    # use its range as a fallback for whichever side we
                    # haven't found yet.
                    if to_range is None:
                        to_range = inc_range
                    elif from_range is None:
                        from_range = inc_range

            # Primary range = where it becomes to_type; fall back to
            # from_range or the old heuristic.
            r = to_range or from_range or _phi_range(cfg, block, ssa_block)
            if r is None:
                continue

            related: list[tuple[Range, str]] = []
            if from_range is not None and from_range is not r:
                related.append(
                    (
                        from_range,
                        f"${phi.name} has intrep {_type_name(phi_type.from_type)} here",
                    )
                )
            if to_range is not None and to_range is not r:
                related.append(
                    (
                        to_range,
                        f"${phi.name} becomes {_type_name(phi_type.tcl_type)} here",
                    )
                )

            code = "S101" if in_loop else "S100"
            severity = "loop " if in_loop else ""
            msg = (
                f"Shimmer: ${phi.name} changes intrep from "
                f"{_type_name(phi_type.from_type)} to "
                f"{_type_name(phi_type.tcl_type)} at merge point "
                f"({severity}S{code[-3:]})"
            )
            warnings.append(
                ShimmerWarning(
                    range=r,
                    variable=phi.name,
                    from_type=phi_type.from_type,
                    to_type=phi_type.tcl_type,
                    command="<phi>",
                    in_loop=in_loop,
                    code=code,
                    message=msg,
                    related_ranges=tuple(related),
                )
            )

    return warnings


def _find_thunking(
    cfg: CFGFunction,
    ssa: SSAFunction,
    types: dict[SSAValueKey, TypeLattice],
    executable_blocks: set[str],
    loop_blocks: set[str],
    def_ranges: dict[SSAValueKey, Range] | None = None,
) -> list[ThunkingWarning]:
    """Find variables that oscillate between types across loop iterations.

    At a loop header, if a phi has SHIMMERED(A, B) type and the loop body
    re-introduces type A (so the phi gets SHIMMERED again next iteration),
    that is type thunking.
    """
    warnings: list[ThunkingWarning] = []
    if def_ranges is None:
        def_ranges = _build_def_ranges(ssa, cfg)

    # Loop headers are blocks that have a back edge (predecessor is in loop).
    # We identify them as blocks that have a predecessor which is also in
    # the loop.
    for bn in executable_blocks:
        if bn not in loop_blocks:
            continue
        ssa_block = ssa.blocks.get(bn)
        if ssa_block is None:
            continue
        block = cfg.blocks.get(bn)
        if block is None:
            continue

        for phi in ssa_block.phis:
            phi_type = types.get((phi.name, phi.version))
            if phi_type is None or phi_type.kind is not TypeKind.SHIMMERED:
                continue
            assert phi_type.tcl_type is not None and phi_type.from_type is not None

            # Classify incoming edges: body (inside loop) vs entry (outside).
            body_range: Range | None = None
            body_type_name: str | None = None
            entry_range: Range | None = None
            entry_type_name: str | None = None
            has_body_incoming = False

            for pred, incoming_ver in phi.incoming.items():
                if incoming_ver <= 0:
                    continue
                incoming_type = types.get((phi.name, incoming_ver))
                if incoming_type is None:
                    continue
                inc_range = def_ranges.get((phi.name, incoming_ver))
                if pred in loop_blocks:
                    if incoming_type.kind is TypeKind.KNOWN:
                        has_body_incoming = True
                        if body_range is None and inc_range is not None:
                            body_range = inc_range
                            body_type_name = (
                                _type_name(incoming_type.tcl_type)
                                if incoming_type.tcl_type
                                else None
                            )
                else:
                    if entry_range is None and inc_range is not None:
                        entry_range = inc_range
                        entry_type_name = (
                            _type_name(incoming_type.tcl_type)
                            if (incoming_type.kind is TypeKind.KNOWN and incoming_type.tcl_type)
                            else None
                        )

            if not has_body_incoming:
                continue

            # Primary range = body definition (where oscillation is
            # re-introduced); fall back to old heuristic.
            r = body_range or _phi_range(cfg, block, ssa_block)
            if r is None:
                continue

            related: list[tuple[Range, str]] = []
            if entry_range is not None and entry_range is not r:
                label = (
                    f"${phi.name} initially {entry_type_name}"
                    if entry_type_name
                    else f"${phi.name} initial type here"
                )
                related.append((entry_range, label))
            if body_range is not None and body_range is not r:
                label = (
                    f"${phi.name} becomes {body_type_name} here"
                    if body_type_name
                    else f"${phi.name} re-typed here"
                )
                related.append((body_range, label))

            msg = (
                f"Type thunking: ${phi.name} oscillates between "
                f"{_type_name(phi_type.from_type)} and "
                f"{_type_name(phi_type.tcl_type)} each loop iteration"
            )
            warnings.append(
                ThunkingWarning(
                    range=r,
                    variable=phi.name,
                    type_a=phi_type.from_type,
                    type_b=phi_type.tcl_type,
                    code="S102",
                    message=msg,
                    related_ranges=tuple(related),
                )
            )

    return warnings


def _phi_range(cfg: CFGFunction, block, ssa_block) -> Range | None:
    """Best source range for a phi node.

    Tries: first statement in block, terminator, first statement in a
    successor, or predecessor terminator ranges.
    """
    if block.statements:
        r = getattr(block.statements[0], "range", None)
        if r is not None:
            return r
    if block.terminator is not None:
        r = getattr(block.terminator, "range", None)
        if r is not None:
            return r
    # Try successors.
    match block.terminator:
        case CFGGoto(target=target):
            succs = (target,)
        case CFGBranch(true_target=tt, false_target=ft):
            succs = (tt, ft)
        case _:
            succs = ()
    for succ_name in succs:
        succ = cfg.blocks.get(succ_name)
        if succ is None:
            continue
        if succ.statements:
            r = getattr(succ.statements[0], "range", None)
            if r is not None:
                return r
    # Try predecessors (look at their terminators).
    # Narrow multi-line ranges because CFGGoto terminators carry the full
    # body_range which would produce excessively wide diagnostics.
    for pred_name, pred_block in cfg.blocks.items():
        match pred_block.terminator:
            case CFGGoto(target=target) if target == block.name:
                r = getattr(pred_block.terminator, "range", None)
                if r is not None:
                    return _narrow_range(r)
            case CFGBranch(true_target=tt, false_target=ft) if block.name in (tt, ft):
                r = getattr(pred_block.terminator, "range", None)
                if r is not None:
                    return r
    return None


# Operators that require numeric operands — STRING → shimmer.
_NUMERIC_OPS = frozenset(
    {
        BinOp.ADD,
        BinOp.SUB,
        BinOp.MUL,
        BinOp.DIV,
        BinOp.MOD,
        BinOp.POW,
        BinOp.LSHIFT,
        BinOp.RSHIFT,
        BinOp.BIT_AND,
        BinOp.BIT_OR,
        BinOp.BIT_XOR,
        BinOp.AND,
        BinOp.OR,
        BinOp.EQ,
        BinOp.NE,
        BinOp.LT,
        BinOp.LE,
        BinOp.GT,
        BinOp.GE,
    }
)

# Operators that require string operands — INT/DOUBLE → shimmer.
_STRING_OPS = frozenset(
    {
        BinOp.STR_EQ,
        BinOp.STR_NE,
        BinOp.STR_LT,
        BinOp.STR_LE,
        BinOp.STR_GT,
        BinOp.STR_GE,
    }
)

_NUMERIC_UNARY_OPS = frozenset({UnaryOp.NEG, UnaryOp.POS, UnaryOp.BIT_NOT, UnaryOp.NOT})


def _collect_expr_shimmers(
    node: ExprNode,
    uses: dict[str, SSAVersion],
    types: dict[SSAValueKey, TypeLattice],
    out: list[tuple[str, TclType, TclType]],
) -> None:
    """Walk an expression AST and collect (variable, from_type, to_type) shimmer triples."""
    match node:
        case ExprBinary(op=op, left=left, right=right):
            if op in _NUMERIC_OPS:
                _check_operand_shimmer(left, uses, types, TclType.NUMERIC, out)
                _check_operand_shimmer(right, uses, types, TclType.NUMERIC, out)
            elif op in _STRING_OPS:
                _check_operand_shimmer(left, uses, types, TclType.STRING, out)
                _check_operand_shimmer(right, uses, types, TclType.STRING, out)
            # Recurse into sub-expressions.
            _collect_expr_shimmers(left, uses, types, out)
            _collect_expr_shimmers(right, uses, types, out)

        case ExprUnary(op=op, operand=operand):
            if op in _NUMERIC_UNARY_OPS:
                _check_operand_shimmer(operand, uses, types, TclType.NUMERIC, out)
            _collect_expr_shimmers(operand, uses, types, out)

        case _:
            pass  # Atoms, calls, ternary, raw — no operator shimmer check


def _check_operand_shimmer(
    operand: ExprNode,
    uses: dict[str, SSAVersion],
    types: dict[SSAValueKey, TypeLattice],
    expected_kind: TclType,
    out: list[tuple[str, TclType, TclType]],
) -> None:
    """If *operand* is a variable with a type incompatible with *expected_kind*, record it."""
    if not isinstance(operand, ExprVar):
        return
    var_name = operand.name
    ver = uses.get(var_name, 0)
    if ver <= 0:
        return
    var_type = types.get((var_name, ver))
    if var_type is None or var_type.kind is not TypeKind.KNOWN:
        return
    actual = var_type.tcl_type

    if expected_kind is TclType.NUMERIC:
        # STRING used in numeric context → shimmer.
        if actual is TclType.STRING:
            out.append((var_name, actual, TclType.INT))
    elif expected_kind is TclType.STRING:
        # INT/DOUBLE/NUMERIC used in string context → shimmer.
        if actual in (TclType.INT, TclType.DOUBLE, TclType.NUMERIC):
            out.append((var_name, actual, TclType.STRING))


def _find_expr_shimmers(
    cfg: CFGFunction,
    ssa: SSAFunction,
    types: dict[SSAValueKey, TypeLattice],
    executable_blocks: set[str],
    loop_blocks: set[str],
) -> list[ShimmerWarning]:
    """Find expression-level shimmers where operator expects incompatible operand type."""
    warnings: list[ShimmerWarning] = []

    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue
        in_loop = bn in loop_blocks

        for idx, ssa_stmt in enumerate(ssa_block.statements):
            if idx >= len(block.statements):
                continue
            stmt = block.statements[idx]

            if not isinstance(stmt, IRAssignExpr):
                continue
            if isinstance(stmt.expr, ExprRaw):
                continue

            shimmer_triples: list[tuple[str, TclType, TclType]] = []
            _collect_expr_shimmers(stmt.expr, ssa_stmt.uses, types, shimmer_triples)

            # Deduplicate by variable name (report each variable once per statement).
            seen_vars: set[str] = set()
            for var_name, from_type, to_type in shimmer_triples:
                if var_name in seen_vars:
                    continue
                seen_vars.add(var_name)

                code = "S101" if in_loop else "S100"
                severity = "loop " if in_loop else ""
                msg = (
                    f"Shimmer: ${var_name} has intrep "
                    f"{_type_name(from_type)} but expr operator "
                    f"expects {_type_name(to_type)} ({severity}S{code[-3:]})"
                )
                warnings.append(
                    ShimmerWarning(
                        range=stmt.range,
                        variable=var_name,
                        from_type=from_type,
                        to_type=to_type,
                        command="expr",
                        in_loop=in_loop,
                        code=code,
                        message=msg,
                    )
                )

    return warnings


def find_shimmer_warnings(
    source: str,
    cu: CompilationUnit | None = None,
) -> list[ShimmerWarning | ThunkingWarning]:
    """Run the full compiler pipeline and return shimmer/thunking diagnostics."""
    cu = ensure_compilation_unit(source, cu, logger=log, context="shimmer")
    if cu is None:
        return []

    all_warnings: list[ShimmerWarning | ThunkingWarning] = []
    for fu in [cu.top_level, *cu.procedures.values()]:
        executable = set(fu.cfg.blocks) - fu.analysis.unreachable_blocks
        loops = _loop_body_blocks(fu.cfg)
        dr = _build_def_ranges(fu.ssa, fu.cfg)
        all_warnings.extend(
            _find_use_site_shimmers(
                fu.execution_intent,
                fu.cfg,
                fu.ssa,
                fu.analysis.types,
                executable,
                loops,
            )
        )
        all_warnings.extend(
            _find_expr_shimmers(
                fu.cfg,
                fu.ssa,
                fu.analysis.types,
                executable,
                loops,
            )
        )
        all_warnings.extend(
            _find_phi_shimmers(
                fu.cfg,
                fu.ssa,
                fu.analysis.types,
                executable,
                loops,
                dr,
            )
        )
        all_warnings.extend(
            _find_thunking(
                fu.cfg,
                fu.ssa,
                fu.analysis.types,
                executable,
                loops,
                dr,
            )
        )

    return all_warnings
