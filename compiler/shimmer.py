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

from compiler.registry.runtime import TYPE_HINTS
from compiler.registry.type_hints import CommandTypeHint, SubcommandTypeHint
from shared.codes import diag
from shared.diagnostic import Range
from shared.naming import normalise_var_name as _normalise_var_name
from shared.tokens import SourcePosition

from .cfg import CFGBranch, CFGFunction, CFGGoto
from .compilation_unit import CompilationUnit, FunctionUnit, ensure_compilation_unit
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
from .ir import IRAssignConst, IRAssignExpr, IRAssignValue, IRCall, IRIncr
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
    """Look up the expected type for an argument that causes a shimmer.

    Only returns a type when the registry declares ``shimmers=True`` for
    that argument position — the shimmer detector is entirely driven by
    command registry metadata.
    """
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
        if arg_hint is None and sub_hint.arg_type_resolver is not None:
            resolved = sub_hint.arg_type_resolver(args[1:])
            arg_hint = resolved.get(sub_arg_index)
        return arg_hint.expected if arg_hint and arg_hint.shimmers else None
    if isinstance(hint, CommandTypeHint):
        arg_hint = hint.arg_types.get(arg_index)
        if arg_hint is None and hint.arg_type_resolver is not None:
            resolved = hint.arg_type_resolver(args)
            arg_hint = resolved.get(arg_index)
        return arg_hint.expected if arg_hint and arg_hint.shimmers else None
    return None


def _check_command_substitution_intent(
    intent: CommandSubstitutionIntent,
    ssa_uses: dict[str, int],
    types: dict[SSAValueKey, TypeLattice],
    stmt_range: Range,
    in_loop: bool,
    already_coerced: set[tuple[str, int, TclType]] | None = None,
    loop_def_names: frozenset[str] | None = None,
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
        already_coerced,
        loop_def_names,
    )


@diag(
    "S100",
    "Single shimmer outside a loop — object internal representation changed.",
    section="shimmer",
)
@diag(
    "S101",
    "Shimmer inside a loop body — per-iteration representation conversion cost.",
    section="shimmer",
)
def _check_args_for_shimmer(
    command: str,
    args: tuple[str, ...],
    ssa_uses: dict[str, int],
    types: dict[SSAValueKey, TypeLattice],
    stmt_range: Range,
    in_loop: bool,
    already_coerced: set[tuple[str, int, TclType]] | None = None,
    loop_def_names: frozenset[str] | None = None,
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

        # If a prior use in the same block already coerced this SSA version
        # to the expected type, the runtime intrep has already changed — no
        # second shimmer occurs.
        coercion_key = (var_name, ver, expected)
        if already_coerced is not None and coercion_key in already_coerced:
            continue

        # In-loop classification: a use inside a loop is "per-iteration cost"
        # (S101) ONLY if the variable's intrep can be reset each iteration.
        # If the variable is **loop-invariant** (no def of it inside the loop
        # body), the runtime intrep converts once at the first iteration and
        # is cached for the rest — that is S100 (one-time conversion), not
        # S101 (recurring).  ``loop_def_names`` carries the set of variable
        # names ever defined inside the enclosing loop's blocks; absence means
        # invariant.  None means the caller didn't supply the lattice, so we
        # fall back to the historical pessimistic classification.
        effective_in_loop = in_loop
        if in_loop and loop_def_names is not None and var_name not in loop_def_names:
            effective_in_loop = False  # loop-invariant → one-time, not per-iteration

        code = "S101" if effective_in_loop else "S100"
        severity = "loop " if effective_in_loop else ""
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
        if already_coerced is not None:
            already_coerced.add(coercion_key)
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

    # Loop-invariance lattice for the in-loop classification: a variable used
    # inside a loop is "per-iteration" (S101) only if its intrep can be reset
    # each iteration — i.e. SOMETHING in the loop body defines it.  A
    # loop-invariant variable (no def inside the loop) shimmers once at first
    # iteration and is cached for the rest, so the right code is S100
    # (one-time), not S101 (per-iteration).  Build the set of names ever
    # defined in any loop block — absence means invariant.  Cheap, single
    # pass; only computed when loops exist.
    loop_def_names: frozenset[str] = frozenset()
    if loop_blocks:
        defs: set[str] = set()
        for lbn in loop_blocks:
            sb = ssa.blocks.get(lbn)
            if sb is None:
                continue
            for st in sb.statements:
                defs.update(st.defs.keys())
            for phi in sb.phis:
                defs.add(phi.name)
        loop_def_names = frozenset(defs)

    for bn in executable_blocks:
        block = cfg.blocks.get(bn)
        ssa_block = ssa.blocks.get(bn)
        if block is None or ssa_block is None:
            continue
        in_loop = bn in loop_blocks
        # Track (var, ver, target_type) coercions within a block so that
        # the second list command using the same variable after a shimmer
        # does not produce a duplicate warning — the runtime intrep has
        # already been converted by the first use.
        already_coerced: set[tuple[str, int, TclType]] = set()

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
                        already_coerced,
                        loop_def_names,
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
                            already_coerced,
                            loop_def_names,
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
                                already_coerced,
                                loop_def_names,
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


def _empty_literal_versions(ssa: SSAFunction, name: str) -> set[SSAVersion]:
    """SSA versions of ``name`` defined by an empty literal (``set x {}`` / ``""``).

    The empty literal is the typeless-empty value — a fresh object that is
    simultaneously a valid empty string, list and dict.  An assignment to it
    (the accumulator-*reset* idiom ``set row {}`` at the top of an outer loop,
    then filled by ``lappend``) drops the prior value rather than coercing its
    intrep, so it is not a shimmer/thunk cost and must not be counted as a
    distinct type at the merge point.
    """
    versions: set[SSAVersion] = set()
    for ssa_block in ssa.blocks.values():
        for s in ssa_block.statements:
            ver = s.defs.get(name)
            if ver is None:
                continue
            stmt = s.statement
            if isinstance(stmt, (IRAssignConst, IRAssignValue)) and stmt.value == "":
                versions.add(ver)
    return versions


def _loop_body_def_types(
    ssa: SSAFunction,
    loop_blocks: set[str],
    name: str,
    types: dict[SSAValueKey, TypeLattice],
) -> set[TclType]:
    """Distinct intreps the variable ``name`` is *defined as* inside the loop.

    A loop-header phi only observes the variable's value at the *end* of the
    body (the back-edge incoming version), so it cannot tell a one-time
    accumulator promotion (``set r {}``; body does ``lappend r …`` once →
    LIST forever) from genuine per-iteration oscillation (``set x [expr …]``;
    then ``set x "s"`` — two type-distinct defs *within* one iteration).  This
    scans every statement def of ``name`` across the loop blocks and collects
    the KNOWN types, so ``len(...) >= 2`` distinguishes the two.
    """
    body_types: set[TclType] = set()
    for bn in loop_blocks:
        ssa_block = ssa.blocks.get(bn)
        if ssa_block is None:
            continue
        for s in ssa_block.statements:
            ver = s.defs.get(name)
            if ver is None:
                continue
            stmt = s.statement
            # The empty literal ``{}`` / ``""`` is the typeless-empty value
            # (a fresh object that is simultaneously a valid empty string,
            # list and dict).  The accumulator-reset idiom — ``set row {}``
            # at the top of an outer loop, filled by ``lappend`` — is *not*
            # an intrep conversion: the old value is dropped, not coerced.
            # Excluding it stops the reset being counted as a second type.
            if isinstance(stmt, (IRAssignConst, IRAssignValue)) and stmt.value == "":
                continue
            t = types.get((name, ver))
            # Count only KNOWN-typed defs.  A SHIMMERED *def* (e.g.
            # ``set t [expr {min(max($k,$lo),$hi)}]`` where an operand is
            # imprecisely typed STRING) is type-inference uncertainty about a
            # single assignment, not two real assignments — counting its two
            # candidate types would manufacture a phantom oscillation.
            if t is not None and t.kind is TypeKind.KNOWN and t.tcl_type is not None:
                body_types.add(t.tcl_type)
    return body_types


def _destructure_foreach_blocks(
    ssa: SSAFunction,
    cfg,
) -> set[str]:
    """Identify SSA blocks whose foreach is a destructure (single-iter
    via ``foreach VARS LIST break``).

    The destructure idiom is Tcl's pre-8.5 ``lassign`` equivalent: a
    foreach with a literal ``break`` body that runs ONCE to bind
    multiple vars at once.  The foreach is technically a loop in the
    CFG, but semantically it's a one-time multi-assign — its var
    bindings should NOT be classified as "loop body types" because
    they don't oscillate; the binding happens once like a plain ``set``.

    Returns the set of foreach-header block names whose body is
    ``break``-only.  Callers exclude these blocks' defs from the
    loop-body-types map so the bindings don't pollute the per-iter
    shimmer reasoning in any encompassing real loop.
    """
    from compiler.cfg import CFGBranch, CFGGoto
    from compiler.ir import IRCall

    destructure: set[str] = set()
    for bname, block in cfg.blocks.items():
        # Must be a foreach header.
        ssa_block = ssa.blocks.get(bname)
        if ssa_block is None:
            continue
        if not block.statements:
            continue
        first = block.statements[0]
        if not isinstance(first, IRCall) or first.command not in ("foreach", "lmap"):
            continue
        # Find the foreach BODY successor.  CFG layout: header → body →
        # ... → end.  The body block is one of the successors (the other
        # is the loop exit).
        term = block.terminator
        successors: list[str] = []
        if isinstance(term, CFGBranch):
            if term.true_target:
                successors.append(term.true_target)
            if term.false_target:
                successors.append(term.false_target)
        elif isinstance(term, CFGGoto):
            if term.target:
                successors.append(term.target)
        # Check each successor: a destructure body is just ``break``.
        body_is_break_only = False
        for succ in successors:
            succ_block = cfg.blocks.get(succ)
            if succ_block is None:
                continue
            stmts = succ_block.statements
            if len(stmts) == 1:
                s = stmts[0]
                if isinstance(s, IRCall) and s.command == "break":
                    body_is_break_only = True
                    break
        if body_is_break_only:
            destructure.add(bname)
    return destructure


def _build_shimmer_name_index(
    ssa: SSAFunction,
    types: dict[SSAValueKey, TypeLattice],
    loop_blocks: set[str],
    cfg=None,
) -> tuple[dict[str, set[SSAVersion]], dict[str, set[TclType]]]:
    """One pass producing the two per-name maps the phi/thunking passes need.

    ``_empty_literal_versions(ssa, name)`` and ``_loop_body_def_types(ssa,
    loop_blocks, name, types)`` each rescan every def of ``name`` and were called
    once *per phi*, so the work was O(phis × blocks × stmts).  Both depend only
    on ``name`` (the thunking/phi passes pass the same function-wide
    ``loop_blocks`` for every phi), so compute them once here:

    * ``empty_by_name`` — versions of each name defined by the empty literal
      (``set x {}`` / ``""``), over the *whole* function;
    * ``loop_body_types`` — the KNOWN intreps each name is *defined as* inside
      the loop blocks (empty literal excluded).  When ``cfg`` is provided,
      destructure foreach blocks (single-iter via ``break``) are excluded —
      their var bindings are one-time multi-assigns (lassign-equivalent),
      not per-iteration body types that should drive S102 shimmer.

    Byte-for-byte equal to the per-call helpers (same predicates, hoisted)."""
    empty_by_name: dict[str, set[SSAVersion]] = {}
    loop_body_types: dict[str, set[TclType]] = {}
    destructure_blocks: set[str] = (
        _destructure_foreach_blocks(ssa, cfg) if cfg is not None else set()
    )
    for bn, ssa_block in ssa.blocks.items():
        in_loop = bn in loop_blocks and bn not in destructure_blocks
        for s in ssa_block.statements:
            stmt = s.statement
            is_empty = isinstance(stmt, (IRAssignConst, IRAssignValue)) and stmt.value == ""
            for name, ver in s.defs.items():
                if is_empty:
                    empty_by_name.setdefault(name, set()).add(ver)
                    continue
                if in_loop:
                    t = types.get((name, ver))
                    if t is not None and t.kind is TypeKind.KNOWN and t.tcl_type is not None:
                        loop_body_types.setdefault(name, set()).add(t.tcl_type)
    return empty_by_name, loop_body_types


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
    empty_by_name: dict[str, set[SSAVersion]] | None = None,
    loop_body_types: dict[str, set[TclType]] | None = None,
) -> list[ShimmerWarning]:
    """Find phi nodes where the merged type is SHIMMERED."""
    warnings: list[ShimmerWarning] = []
    if def_ranges is None:
        def_ranges = _build_def_ranges(ssa, cfg)
    if empty_by_name is None or loop_body_types is None:
        empty_by_name, loop_body_types = _build_shimmer_name_index(ssa, types, loop_blocks, cfg)

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
            entry_types: set[TclType] = set()  # types from non-loop (entry) edges
            body_types: set[TclType] = set()  # types from loop (body) edges
            nonempty_known: set[TclType] = set()  # KNOWN types from non-empty incomings
            has_shimmered_incoming = False
            empty_vers = empty_by_name.get(phi.name, frozenset())
            for _pred, incoming_ver in phi.incoming.items():
                if incoming_ver <= 0:
                    continue
                inc_type = types.get((phi.name, incoming_ver))
                if inc_type is None or inc_type.kind is TypeKind.UNKNOWN:
                    continue
                in_body = _pred in loop_blocks
                if (
                    inc_type.kind is TypeKind.KNOWN
                    and inc_type.tcl_type is not None
                    and incoming_ver not in empty_vers
                ):
                    (body_types if in_body else entry_types).add(inc_type.tcl_type)
                    nonempty_known.add(inc_type.tcl_type)
                elif inc_type.kind is TypeKind.SHIMMERED:
                    has_shimmered_incoming = True
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

            # A loop-header phi is SHIMMERED whenever the entry type differs from
            # the stable body type — but the empty-string accumulator idiom
            # (``set r {}; foreach … {lappend r …}``) promotes ``r`` STRING→LIST
            # exactly once and then stabilises; that is not a per-iteration
            # (S101) cost.  Only keep the loop shimmer when the body re-introduces
            # an entry type (re-shimmers each iteration) or itself produces ≥2
            # types.  (Out-of-loop S100 single-shimmers are always reported.)
            if in_loop:
                all_body_types = body_types | loop_body_types.get(phi.name, set())
                oscillates = bool(entry_types & all_body_types) or len(all_body_types) >= 2
                if not oscillates:
                    continue
            elif not has_shimmered_incoming and len(nonempty_known) <= 1:
                # Out-of-loop branch merge whose only SHIMMERED appearance comes
                # from an empty-literal incoming (``set r {}`` on one arm,
                # ``set r [list …]`` on another): the empty value is the typeless
                # value (valid empty string/list/dict), so the merge is not a real
                # intrep conversion.  A genuine merge has ≥2 distinct non-empty
                # types, or a SHIMMERED incoming propagating a real prior shimmer.
                continue

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


@diag("S102", "Variable oscillates between two types across loop iterations.", section="shimmer")
def _find_thunking(
    cfg: CFGFunction,
    ssa: SSAFunction,
    types: dict[SSAValueKey, TypeLattice],
    executable_blocks: set[str],
    loop_blocks: set[str],
    def_ranges: dict[SSAValueKey, Range] | None = None,
    empty_by_name: dict[str, set[SSAVersion]] | None = None,
    loop_body_types: dict[str, set[TclType]] | None = None,
) -> list[ThunkingWarning]:
    """Find variables that oscillate between types across loop iterations.

    At a loop header, if a phi has SHIMMERED(A, B) type and the loop body
    re-introduces type A (so the phi gets SHIMMERED again next iteration),
    that is type thunking.
    """
    warnings: list[ThunkingWarning] = []
    if def_ranges is None:
        def_ranges = _build_def_ranges(ssa, cfg)
    if empty_by_name is None or loop_body_types is None:
        empty_by_name, loop_body_types = _build_shimmer_name_index(ssa, types, loop_blocks, cfg)

    # Build PER-LOOP body_types so that a destructure foreach's STRING
    # binding doesn't leak into a real loop's type set (and vice versa
    # across sibling loops in the same proc).  The function-wide
    # ``loop_body_types`` map is an over-approximation that triggers
    # S102 FPs when sibling loops have different type effects on the
    # same name.
    from compiler.loops import build_loop_forest

    forest = build_loop_forest(cfg, ssa, executable_blocks)
    destructure_set = _destructure_foreach_blocks(ssa, cfg)
    per_header_body_types: dict[str, dict[str, set[TclType]]] = {}
    for loop in forest.loops:
        per_loop_types: dict[str, set[TclType]] = {}
        for lbn in loop.blocks:
            if lbn in destructure_set:
                continue
            lssa = ssa.blocks.get(lbn)
            if lssa is None:
                continue
            for ls in lssa.statements:
                lstmt = ls.statement
                is_empty = isinstance(lstmt, (IRAssignConst, IRAssignValue)) and lstmt.value == ""
                for name, ver in ls.defs.items():
                    if is_empty:
                        continue
                    t = types.get((name, ver))
                    if t is not None and t.kind is TypeKind.KNOWN and t.tcl_type is not None:
                        per_loop_types.setdefault(name, set()).add(t.tcl_type)
        per_header_body_types[loop.header] = per_loop_types

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
            entry_types: set[TclType] = set()
            body_types: set[TclType] = set()
            empty_vers = empty_by_name.get(phi.name, frozenset())

            for pred, incoming_ver in phi.incoming.items():
                if incoming_ver <= 0:
                    continue
                incoming_type = types.get((phi.name, incoming_ver))
                if incoming_type is None:
                    continue
                # An empty-literal incoming (``set x {}``) is the typeless reset,
                # not a real type at the merge — skip its type contribution so the
                # nested accumulator-reset idiom (``set row {}`` in an outer loop,
                # ``lappend row …`` in an inner loop, the flat loop-block set making
                # the reset look like a body edge) is not counted as oscillation.
                is_empty = incoming_ver in empty_vers
                inc_range = def_ranges.get((phi.name, incoming_ver))
                if pred in loop_blocks:
                    if incoming_type.kind is TypeKind.KNOWN and not is_empty:
                        has_body_incoming = True
                        if incoming_type.tcl_type is not None:
                            body_types.add(incoming_type.tcl_type)
                        if body_range is None and inc_range is not None:
                            body_range = inc_range
                            body_type_name = (
                                _type_name(incoming_type.tcl_type)
                                if incoming_type.tcl_type
                                else None
                            )
                else:
                    if (
                        incoming_type.kind is TypeKind.KNOWN
                        and incoming_type.tcl_type is not None
                        and not is_empty
                    ):
                        entry_types.add(incoming_type.tcl_type)
                    if entry_range is None and inc_range is not None:
                        entry_range = inc_range
                        entry_type_name = (
                            _type_name(incoming_type.tcl_type)
                            if (incoming_type.kind is TypeKind.KNOWN and incoming_type.tcl_type)
                            else None
                        )

            if not has_body_incoming:
                continue

            # A loop-header phi is SHIMMERED whenever the entry type differs from
            # the body-exit type — but that includes the *one-time* promotion of
            # an empty-string accumulator (``set r {}; foreach … {lappend r …}``):
            # ``r`` is STRING once (the entry) then LIST forever (the body
            # stabilises).  That is not per-iteration oscillation.  Genuine
            # thunking requires the body to *re-introduce* the entry type (so the
            # phi re-shimmers each iteration) or to produce ≥2 conflicting types
            # itself.  Otherwise the var stabilises at the body type → suppress.
            # Use PER-LOOP body_types (this loop only) instead of the
            # function-wide map — a sibling loop's type effect on the same
            # name must not pollute this loop's oscillation check.
            per_loop = per_header_body_types.get(bn, {}).get(phi.name, set())
            all_body_types = body_types | per_loop
            oscillates = bool(entry_types & all_body_types) or len(all_body_types) >= 2
            if not oscillates:
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
    *,
    target_procs: frozenset[str] | None = None,
) -> list[ShimmerWarning | ThunkingWarning]:
    """Run the full compiler pipeline and return shimmer/thunking diagnostics.

    Shimmer is **body-local**: each proc's warnings depend only on its own
    ``FunctionUnit`` (cfg/ssa/types), never on other procs.  When *target_procs*
    is given, emit only for the top level + those proc qnames (the incremental
    path reuses cached warnings for the rest); ``None`` emits for everything.
    """
    cu = ensure_compilation_unit(source, cu, logger=log, context="shimmer")
    if cu is None:
        return []

    all_warnings: list[ShimmerWarning | ThunkingWarning] = []
    units: list[tuple[str | None, FunctionUnit]] = [(None, cu.top_level)]
    units.extend((qname, fu) for qname, fu in cu.procedures.items())
    for qname, fu in units:
        if target_procs is not None and qname is not None and qname not in target_procs:
            continue
        if fu.complexity_guarded:
            continue  # deep analysis skipped for this body (see core_analyses)
        executable = set(fu.cfg.blocks) - fu.analysis.unreachable_blocks
        loops = _loop_body_blocks(fu.cfg)
        dr = _build_def_ranges(fu.ssa, fu.cfg)
        # Per-name empty-literal versions + loop-body KNOWN types, computed once
        # and shared by the phi + thunking passes (was recomputed per phi).
        empty_by_name, loop_body_types = _build_shimmer_name_index(
            fu.ssa, fu.analysis.types, loops, fu.cfg
        )
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
                empty_by_name,
                loop_body_types,
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
                empty_by_name,
                loop_body_types,
            )
        )

    return all_warnings
