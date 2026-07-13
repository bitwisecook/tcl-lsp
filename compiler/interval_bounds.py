# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Interval-driven dynamic bounds checking (Phase 3).

The syntactic bounds checks (``analyser/checks/_bounds.py``, W230/W231/string)
only fire when *both* the container and the index are literals.  This module
covers the **dynamic** cases they skip: an index that is a plain ``$var`` whose
:mod:`compiler.intervals` range — guard-narrowed at the use site — *proves* the
access is out of range, against a container length we can establish statically
(a literal list / ``[list …]`` element count, propagated per SSA version).

It is a *consumer* of the parallel interval analysis: it never perturbs SCCP or
any existing diagnostic, and it only emits on the dynamic shapes the syntactic
check leaves silent, so the two never double-fire.

Soundness rule: an :class:`~compiler.intervals.Interval` over-approximates the
runtime value, so a finding is reported **only** when the *whole* interval lies
outside the valid range — never on "might be out of range".  This matches the
Tcl-9.0.3 semantics the syntactic check already encodes:

* ``lindex`` out-of-range → silent empty string  → W230 (smell / likely bug).
* ``lset``  out-of-range → runtime error          → W231 (stronger).
  ``index == length`` is the legal *append* slot, so only ``index > length`` or
  ``index < 0`` is an error.
* ``string index`` out-of-range → silent empty    → string-range smell.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from dataclasses import dataclass

from shared.tcl_subst import backslash_subst

from .cfg import CFGBranch, CFGFunction, CFGReturn
from .execution_intent import FunctionExecutionIntent, _parse_command_substitution
from .expr_ast import (
    BinOp,
    ExprBinary,
    ExprCall,
    ExprCommand,
    ExprNode,
    ExprTernary,
    ExprUnary,
    _const_bool,
)
from .intervals import (
    Interval,
    _eval_expr,
    build_guard_index,
    compute_intervals,
    refine_interval,
)
from .ir import IRAssignConst, IRAssignExpr, IRAssignValue, IRBarrier, IRCall, IRExprEval, IRReturn
from .ssa import SSAFunction, SSAValueKey
from .tcl_expr_eval import _split_tcl_list

# Commands whose dynamic bounds we reason about.
_LINDEX = "lindex"
_LSET = "lset"
_STRING_INDEX = "string index"


@dataclass(frozen=True, slots=True)
class BoundsFinding:
    """A proven out-of-range dynamic index access."""

    block: str
    statement_index: int
    code: str  # "W230" | "W231"
    command: str  # "lindex" | "lset" | "string index"
    index_var: str  # the ``$var`` index name (display only)
    index_interval: Interval
    length: int
    reason: str  # "negative" | "past_end" | "past_append"


def _plain_var_name(arg: str) -> str | None:
    """The scalar variable name if *arg* is exactly ``$name`` / ``${name}``.

    Returns ``None`` for anything else (``end``, ``end-1``, ``$arr(i)``,
    ``[expr …]``, composites) — we only have an interval for a plain scalar.
    """
    s = arg.strip()
    if not s.startswith("$"):
        return None
    s = s[1:]
    if s.startswith("{") and s.endswith("}"):
        s = s[1:-1]
    if not s:
        return None
    # Reject array elements / composites / further substitution.
    if any(c in s for c in "([$ \t)"):
        return None
    if "]" in s:
        return None
    return s


def _literal_list_length(text: str) -> int | None:
    """Element count of a static Tcl list literal, or ``None`` if not literal."""
    if "$" in text or "[" in text:
        return None
    try:
        return len(_split_tcl_list(text))
    except Exception:
        return None


def _list_length_map(
    ssa: SSAFunction,
    intent: FunctionExecutionIntent,
) -> dict[SSAValueKey, int]:
    """Length (element count) of list-valued SSA versions we can establish.

    Seeds from literal-list assignments — ``set l {a b c}`` (IRAssignConst) and
    ``set l [list a b c]`` (an ``[list …]`` command substitution).  Conservative:
    only *exact* literal lengths are recorded; anything dynamic is absent (the
    consumer then simply does not fire for that version).
    """
    lengths: dict[SSAValueKey, int] = {}
    for bn, sb in ssa.blocks.items():
        for idx, s in enumerate(sb.statements):
            stmt = s.statement
            n: int | None = None
            if isinstance(stmt, IRAssignConst):
                n = _literal_list_length(stmt.value)
            elif isinstance(stmt, IRAssignValue):
                ci = intent.command_substitutions.get((bn, idx))
                if ci is not None and ci.command == "list":
                    # ``[list a b c]`` — element count is the arg count, but only
                    # when no ``{*}`` expansion or substitution muddies it.  A
                    # ``{*}`` arg collapses to one word in ``args`` yet expands
                    # to many elements at runtime (``[list {*}{a b}]`` is two),
                    # so ``has_expansion`` must veto the inference.
                    if not ci.has_expansion and all("$" not in a and "[" not in a for a in ci.args):
                        n = len(ci.args)
            if n is not None:
                for name, ver in s.defs.items():
                    lengths[(name, ver)] = n
    return lengths


def _string_length_map(ssa: SSAFunction) -> dict[SSAValueKey, int]:
    """Character length of string-valued SSA versions we can establish.

    Seeds from literal assignments — ``set s "hello"`` / ``set s {abc}``.  The
    length is the Tcl *character* count: a quoted/bare value
    (``IRAssignValue.value_needs_backsubst``) has its backslash escapes resolved
    at runtime, so ``set s "a\\nb"`` is **3** chars (``a``, newline, ``b``), not
    the 4 source characters; a braced value (``IRAssignConst``) and a bare value
    with no escapes keep their source length verbatim.  Dynamic / substituted
    values are absent.
    """
    lengths: dict[SSAValueKey, int] = {}
    for sb in ssa.blocks.values():
        for s in sb.statements:
            stmt = s.statement
            if isinstance(stmt, (IRAssignConst, IRAssignValue)):
                if "$" in stmt.value or "[" in stmt.value:
                    continue
                value = stmt.value
                if isinstance(stmt, IRAssignValue) and stmt.value_needs_backsubst and "\\" in value:
                    value = backslash_subst(value)
                for name, ver in s.defs.items():
                    lengths[(name, ver)] = len(value)
    return lengths


def _reaching_versions(sb_entry: dict[str, int], stmts, upto: int) -> dict[str, int]:
    """Versions of each name reaching statement index *upto* within a block.

    Walks the block's straight-line def sequence from its entry versions; used
    for ``lset``, whose target list is recorded as a *def* (not a use) so its
    pre-mutation version is not in ``ssa_stmt.uses``.
    """
    cur = dict(sb_entry)
    for i in range(upto):
        for name, ver in stmts[i].defs.items():
            # The reaching version is the value *before* this def, so record the
            # prior version: the def becomes visible only to later statements.
            cur[name] = ver
    return cur


def _classify(index: Interval, length: int, *, is_lset: bool) -> str | None:
    """Reason string if *index* is wholly out of range for *length*, else None.

    Sound: requires the entire interval to be invalid.  ``lset`` permits the
    append slot (``index == length``); ``lindex`` does not.
    """
    # Provably negative: the whole interval is below 0.
    if index.hi is not None and index.hi < 0:
        return "negative"
    # Provably past the end.
    if index.lo is not None:
        if is_lset:
            if index.lo > length:
                return "past_append"
        elif index.lo >= length:
            return "past_end"
    return None


# A candidate is one index access: (command, list_arg, index_arg, is_lset).
_Candidate = tuple[str, str, str, bool]


def _parse_index_sub(text: str) -> _Candidate | None:
    """If *text* is exactly a ``[lindex …]`` / ``[string index …]`` command
    substitution, return its candidate tuple; else ``None``.

    ``lset`` is excluded here: its first arg is a variable *name* needing
    reaching-version resolution, so it is handled only as a top-level statement.
    """
    s = text.strip()
    if not (s.startswith("[") and s.endswith("]")):
        return None
    ci = _parse_command_substitution(s)
    if ci is None:
        return None
    if ci.command == _LINDEX and len(ci.args) == 2:
        return (_LINDEX, ci.args[0], ci.args[1], False)
    if ci.command == "string" and len(ci.args) == 3 and ci.args[0] == "index":
        return (_STRING_INDEX, ci.args[1], ci.args[2], False)
    return None


# ``&&`` / ``||`` (and their word forms) short-circuit their right operand, and a
# ternary evaluates only the selected arm.  Tcl's ``expr`` honours this laziness:
# ``expr {0 ? 1/0 : 7}`` and ``expr {1 || 1/0}`` complete without touching the
# dead sub-expression.  A *guaranteed*-runtime diagnostic (divide-by-zero W233,
# or an out-of-range smell) must therefore never be drawn from a sub-expression
# that may not execute.
#
# A lazy arm whose guard is a **compile-time constant** is the opposite case: it
# is *forced* to run, so ``expr {1 && 1/0}`` / ``expr {0 || 1/0}`` /
# ``expr {1 ? 1/0 : 7}`` genuinely raise at run time and a guaranteed-error
# finding drawn from the forced arm is sound.  :func:`_walk_eager` resolves a
# constant guard to walk exactly the forced arm; a *non-constant* guard leaves
# the arm maybe-dead and it stays skipped (no false positive).
_AND_BINOPS = frozenset({BinOp.AND, BinOp.WORD_AND})
_OR_BINOPS = frozenset({BinOp.OR, BinOp.WORD_OR})
_LAZY_BINOPS = _AND_BINOPS | _OR_BINOPS


def _walk_eager(expr: ExprNode, visit: Callable[[ExprNode], None]) -> None:
    """Visit *expr* and every **guaranteed-to-evaluate** sub-expression.

    The short-circuit (right) operand of ``&&``/``||``/``and``/``or`` and the
    arms of a ternary run only when selected.  This walk resolves a
    **constant** guard so a *forced* lazy arm is still visited (its sub-tree is
    guaranteed to run, so a guaranteed-error diagnostic drawn from it is sound):

    * ``a && b`` — ``b`` is forced iff ``a`` is a constant **true**;
    * ``a || b`` — ``b`` is forced iff ``a`` is a constant **false**;
    * ``c ? t : f`` — ``t`` forced iff ``c`` constant-true, ``f`` iff constant-false.

    A *non-constant* guard leaves the arm maybe-dead, so it is skipped and no
    guaranteed-error finding is drawn from it (preserves the dead-arm
    suppressions, e.g. FP-BND-05).  The constant test is env-independent
    (``_const_bool`` over a literal number / bool keyword — handling floats and
    case-insensitive booleans like ``True``/``1.0``, returning ``None`` when not
    statically decidable so the arm stays safely skipped), so this is sound for
    the candidate-collection callers that have no interval environment.  It is
    the same constant-truth engine ``expr_ast.dead_command_ranges`` uses for the
    W123 dead-arm check, so the two agree on which arm is forced/dead.
    """
    visit(expr)
    if isinstance(expr, ExprBinary):
        _walk_eager(expr.left, visit)
        if expr.op not in _LAZY_BINOPS:
            _walk_eager(expr.right, visit)
            return
        guard = _const_bool(expr.left)
        if guard is None:
            return  # maybe-dead RHS — leave it skipped
        # ``a && b`` forces b iff a is constant-true; ``a || b`` iff a is false.
        forced = guard if expr.op in _AND_BINOPS else (not guard)
        if forced:
            _walk_eager(expr.right, visit)
    elif isinstance(expr, ExprUnary):
        _walk_eager(expr.operand, visit)
    elif isinstance(expr, ExprTernary):
        _walk_eager(expr.condition, visit)
        cond = _const_bool(expr.condition)
        if cond is None:
            return  # both arms maybe-dead — leave them skipped
        _walk_eager(expr.true_branch if cond else expr.false_branch, visit)
    elif isinstance(expr, ExprCall):
        for a in expr.args:
            _walk_eager(a, visit)


def _index_subs_in_expr(expr: ExprNode) -> list[_Candidate]:
    """Index accesses embedded as command substitutions inside an expression.

    Reaches ``set u [expr {[lindex $l $i] + 1}]`` — the ``[lindex …]`` is an
    ``ExprCommand`` node whose text is a command substitution.  Only
    unconditionally-evaluated positions are reported (see :func:`_walk_eager`),
    so a ``[lindex …]`` in a dead ternary arm or short-circuited ``||`` operand
    is not flagged.
    """
    out: list[_Candidate] = []

    def visit(e: ExprNode) -> None:
        if isinstance(e, ExprCommand):
            sub = _parse_index_sub(e.text)
            if sub is not None:
                out.append(sub)

    _walk_eager(expr, visit)
    return out


def _statement_candidates(stmt: object) -> list[_Candidate]:
    """All index accesses a statement performs.

    Covers top-level ``lindex``/``lset`` calls *and* ``lindex``/``string index``
    command substitutions nested in any argument value — ``puts [lindex $l $i]``,
    ``lappend out [lindex $l $i]``, ``set x [lindex $l $i]`` — the positions the
    command-substitution intent (``set x = [...]`` only) does not reach.
    """
    out: list[_Candidate] = []
    if isinstance(stmt, IRCall):
        if stmt.command == _LINDEX and len(stmt.args) == 2:
            out.append((_LINDEX, stmt.args[0], stmt.args[1], False))
        elif stmt.command == _LSET and len(stmt.args) == 3:
            out.append((_LSET, stmt.args[0], stmt.args[1], True))
        for a in stmt.args:
            sub = _parse_index_sub(a)
            if sub is not None:
                out.append(sub)
    elif isinstance(stmt, IRAssignValue):
        sub = _parse_index_sub(stmt.value)
        if sub is not None:
            out.append(sub)
    elif isinstance(stmt, IRBarrier):
        for a in stmt.args:
            sub = _parse_index_sub(a)
            if sub is not None:
                out.append(sub)
    elif isinstance(stmt, (IRAssignExpr, IRExprEval)):
        out += _index_subs_in_expr(stmt.expr)
    if isinstance(stmt, IRReturn) and stmt.expr is not None:
        out += _index_subs_in_expr(stmt.expr)
    return out


def _has_candidate(cfg: CFGFunction, ssa: SSAFunction) -> bool:
    """Cheap pre-scan: any index access with a plain ``$var`` index?"""
    for sb in ssa.blocks.values():
        for s in sb.statements:
            for _cmd, _list, index_arg, _ls in _statement_candidates(s.statement):
                if _plain_var_name(index_arg) is not None:
                    return True
    for block in cfg.blocks.values():
        term = block.terminator
        cands: list[_Candidate] = []
        if isinstance(term, CFGReturn):
            if term.value:
                sub = _parse_index_sub(term.value)
                if sub is not None:
                    cands.append(sub)
            if term.expr is not None:
                cands += _index_subs_in_expr(term.expr)
        elif isinstance(term, CFGBranch):
            cands += _index_subs_in_expr(term.condition)
        for _cmd, _list, index_arg, _ls in cands:
            if _plain_var_name(index_arg) is not None:
                return True
    return False


def find_interval_bounds(
    cfg: CFGFunction,
    ssa: SSAFunction,
    intent: FunctionExecutionIntent,
    values: Mapping[SSAValueKey, object] | None,
    executable_blocks: set[str] | None = None,
    *,
    intervals: dict[SSAValueKey, Interval] | None = None,
    guard_index: dict[SSAValueKey, list[str]] | None = None,
) -> list[BoundsFinding]:
    """Dynamic out-of-range findings for this function (empty if none).

    *executable_blocks* (the SCCP-reachable blocks, ``set(cfg.blocks) -
    analysis.unreachable_blocks``) restricts findings to live code: an index
    access in a statically-unreachable block (e.g. ``if {0} { ... }``) must not
    warn.  ``None`` means "no reachability filter" (every block considered),
    matching ``find_divide_by_zero``'s executability discipline when provided.

    *intervals* lets a caller that also runs :func:`find_divide_by_zero` pass the
    interval fixpoint computed once (see :func:`find_interval_findings`) instead
    of each function recomputing it; ``None`` computes it here.
    """
    # Perf gate: only pay for the interval fixpoint when there is a candidate
    # (an index access with a plain ``$var`` index).
    if not _has_candidate(cfg, ssa):
        return []
    if intervals is None:
        intervals = compute_intervals(cfg, ssa, values)
    lengths = _list_length_map(ssa, intent)
    str_lengths = _string_length_map(ssa)
    findings: list[BoundsFinding] = []

    def length_for_list(
        list_arg: str,
        is_lset: bool,
        version_map: Mapping[str, int],
        entry_versions: Mapping[str, int],
        block_stmts,
        stmt_idx: int,
    ) -> int | None:
        if is_lset:
            # ``lset``'s first arg is a variable *name*, recorded as a def — use
            # the version reaching this statement (block entry + prior defs).
            lname = list_arg.strip()
            if "$" in lname or "[" in lname:
                return None
            reaching = _reaching_versions(dict(entry_versions), block_stmts, stmt_idx)
            lver = reaching.get(lname)
            return lengths.get((lname, lver)) if lver is not None else None
        # A *value* arg: literal list, or ``$l``.
        lit = _literal_list_length(list_arg)
        if lit is not None:
            return lit
        lvar = _plain_var_name(list_arg)
        if lvar is None:
            return None
        lver = version_map.get(lvar)
        return lengths.get((lvar, lver)) if lver is not None else None

    def process(
        cand: _Candidate,
        bn: str,
        stmt_index: int,
        version_map: Mapping[str, int],
        entry_versions: Mapping[str, int],
        block_stmts,
    ) -> None:
        command, list_arg, index_arg, is_lset = cand
        ivar = _plain_var_name(index_arg)
        if ivar is None:
            return
        iver = version_map.get(ivar)
        if iver is None or iver <= 0:
            return
        if command == _STRING_INDEX:
            # First arg is a *value* (string); use the per-version char-length.
            svar = _plain_var_name(list_arg)
            length = (
                str_lengths.get((svar, version_map[svar]))
                if svar is not None and svar in version_map
                else None
            )
        else:
            length = length_for_list(
                list_arg, is_lset, version_map, entry_versions, block_stmts, stmt_index
            )
        if length is None:
            return
        iv = refine_interval(intervals, cfg, ssa, bn, ivar, iver, guard_index)
        if iv.is_top or iv.is_bottom:
            return
        reason = _classify(iv, length, is_lset=is_lset)
        if reason is None:
            return
        code = "W231" if is_lset else ("W232" if command == _STRING_INDEX else "W230")
        findings.append(
            BoundsFinding(
                block=bn,
                statement_index=stmt_index,
                code=code,
                command=command,
                index_var=ivar,
                index_interval=iv,
                length=length,
                reason=reason,
            )
        )

    for bn, sb in ssa.blocks.items():
        if executable_blocks is not None and bn not in executable_blocks:
            continue  # skip statically-unreachable code (e.g. `if {0} { … }`)
        for idx, s in enumerate(sb.statements):
            for cand in _statement_candidates(s.statement):
                process(cand, bn, idx, s.uses, sb.entry_versions, sb.statements)
        # Index accesses in a ``return [lindex …]`` value: the read versions are
        # the block's exit versions; anchor the finding on the terminator (-1).
        cfg_block = cfg.blocks.get(bn)
        term = cfg_block.terminator if cfg_block is not None else None
        if isinstance(term, CFGReturn):
            if term.value:
                sub = _parse_index_sub(term.value)
                if sub is not None:
                    process(sub, bn, -1, sb.exit_versions, sb.exit_versions, sb.statements)
            if term.expr is not None:
                for cand in _index_subs_in_expr(term.expr):
                    process(cand, bn, -1, sb.exit_versions, sb.exit_versions, sb.statements)
        elif isinstance(term, CFGBranch):
            for cand in _index_subs_in_expr(term.condition):
                process(cand, bn, -1, sb.exit_versions, sb.exit_versions, sb.statements)
    return findings


@dataclass(frozen=True, slots=True)
class DivZeroFinding:
    """A division/modulo whose divisor is provably zero (a runtime error)."""

    block: str
    statement_index: int  # -1 == terminator (branch condition / return expr)
    op: str  # "/" | "%"


def _expr_of(stmt: object) -> ExprNode | None:
    """The expression AST a statement evaluates, if any."""
    if isinstance(stmt, (IRAssignExpr, IRExprEval)):
        return stmt.expr
    if isinstance(stmt, IRReturn):
        return stmt.expr
    return None


def _divisors(expr: ExprNode) -> list[tuple[BinOp, ExprNode]]:
    """Every unconditionally-evaluated ``/`` / ``%`` divisor (right operand).

    Only divisions on the always-evaluated spine are returned — a ``1/0`` in a
    dead ternary arm or a short-circuited ``&&``/``||`` operand never executes
    in Tcl, so it must not yield a guaranteed-divide-by-zero finding.
    """
    out: list[tuple[BinOp, ExprNode]] = []

    def visit(e: ExprNode) -> None:
        if isinstance(e, ExprBinary) and e.op in (BinOp.DIV, BinOp.MOD):
            out.append((e.op, e.right))

    _walk_eager(expr, visit)
    return out


def _has_division(ssa: SSAFunction, cfg: CFGFunction) -> bool:
    """Cheap pre-scan: does any expression contain a ``/`` or ``%``?"""
    for sb in ssa.blocks.values():
        for s in sb.statements:
            e = _expr_of(s.statement)
            if e is not None and _divisors(e):
                return True
    for block in cfg.blocks.values():
        term = block.terminator
        if isinstance(term, CFGBranch) and _divisors(term.condition):
            return True
        if isinstance(term, CFGReturn) and term.expr is not None and _divisors(term.expr):
            return True
    return False


def find_divide_by_zero(
    cfg: CFGFunction,
    ssa: SSAFunction,
    values: Mapping[SSAValueKey, object] | None,
    executable_blocks: set[str],
    *,
    intervals: dict[SSAValueKey, Interval] | None = None,
    guard_index: dict[SSAValueKey, list[str]] | None = None,
) -> list[DivZeroFinding]:
    """Divisions/modulo whose divisor is provably ``[0, 0]`` (a runtime error).

    Sound: the divisor's interval (guard-narrowed at the use site) must be
    exactly ``[0, 0]``, and the block must be **SCCP-executable**.  A
    ``$d != 0`` guard cannot narrow an interval (``!=`` is non-convex), but a
    provably-zero ``$d`` makes ``$d != 0`` constant-false, so SCCP marks the
    guarded division's block unreachable — the executability filter excludes
    it, so no false positive.

    *intervals* lets a caller share the fixpoint with :func:`find_interval_bounds`
    (see :func:`find_interval_findings`); ``None`` computes it here.
    """
    if not _has_division(ssa, cfg):
        return []
    # Bind a guaranteed-non-None local so the env_for closure below sees a
    # definite ``dict`` type (the optional param can't be narrowed into a
    # nested function).
    ivals = intervals if intervals is not None else compute_intervals(cfg, ssa, values)
    findings: list[DivZeroFinding] = []

    def env_for(uses: Mapping[str, int], bn: str) -> dict[str, Interval]:
        return {
            name: refine_interval(ivals, cfg, ssa, bn, name, ver, guard_index)
            for name, ver in uses.items()
            if ver > 0
        }

    def check(expr: ExprNode, bn: str, stmt_index: int, env: dict[str, Interval]) -> None:
        for op, divisor in _divisors(expr):
            iv = _eval_expr(divisor, env)
            if iv.lo == 0 and iv.hi == 0:
                findings.append(DivZeroFinding(bn, stmt_index, op.value))

    for bn, sb in ssa.blocks.items():
        if bn not in executable_blocks:
            continue
        for idx, s in enumerate(sb.statements):
            expr = _expr_of(s.statement)
            if expr is not None:
                check(expr, bn, idx, env_for(s.uses, bn))
        term = cfg.blocks[bn].terminator
        if isinstance(term, CFGBranch):
            check(term.condition, bn, -1, env_for(sb.exit_versions, bn))
        elif isinstance(term, CFGReturn) and term.expr is not None:
            check(term.expr, bn, -1, env_for(sb.exit_versions, bn))
    return findings


def find_interval_findings(
    cfg: CFGFunction,
    ssa: SSAFunction,
    intent: FunctionExecutionIntent,
    values: Mapping[SSAValueKey, object] | None,
    executable_blocks: set[str],
) -> tuple[list[BoundsFinding], list[DivZeroFinding]]:
    """Both interval-driven passes (bounds + divide-by-zero) over one fixpoint.

    ``find_interval_bounds`` and ``find_divide_by_zero`` each compute the
    (potentially expensive) interval fixpoint.  A function with both a dynamic
    index and a division paid for it twice.  This computes it once — gated on the
    cheap pre-scans so a function with neither still skips it entirely — and
    feeds it to both.  Returns ``(bounds_findings, divzero_findings)``.
    """
    has_candidate = _has_candidate(cfg, ssa)
    has_division = _has_division(ssa, cfg)
    if not (has_candidate or has_division):
        return [], []
    intervals = compute_intervals(cfg, ssa, values)
    # Build the guard index once and share it: refine_interval would otherwise
    # rescan every CFG block per use, per statement, in both passes.
    guard_index = build_guard_index(cfg, ssa)
    bounds = (
        find_interval_bounds(
            cfg,
            ssa,
            intent,
            values,
            executable_blocks,
            intervals=intervals,
            guard_index=guard_index,
        )
        if has_candidate
        else []
    )
    divzero = (
        find_divide_by_zero(
            cfg, ssa, values, executable_blocks, intervals=intervals, guard_index=guard_index
        )
        if has_division
        else []
    )
    return bounds, divzero


__all__ = [
    "BoundsFinding",
    "DivZeroFinding",
    "find_interval_bounds",
    "find_divide_by_zero",
    "find_interval_findings",
]
