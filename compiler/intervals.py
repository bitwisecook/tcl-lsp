"""Integer interval abstract domain over SSA values (Phase 3).

A *parallel, non-perturbing* numeric analysis: it runs **after** SCCP and reads
its constant lattice, but does not feed back into ``LatticeValue``/``_join`` or
any existing consumer.  The result is an optional ``intervals`` map on
``FunctionAnalysis`` that downstream checks (bounds, divide-by-zero) may consult
for the *dynamic* (variable) cases the purely-syntactic checks skip.

Each SSA value ``(name, version)`` maps to an :class:`Interval` ``[lo, hi]``
over the integers, with ``None`` meaning unbounded (-inf / +inf).  Values that
are not provably integral, or whose range we cannot bound, are ``TOP``
(``[-inf, +inf]``) — always sound.  Loop-header phis are **widened** so the
fixpoint terminates: a bound that moves away from the merge is sent to the
corresponding infinity.  Tightening back is **query-driven**, not a separate
pass: :func:`refine_interval` intersects a value's interval with the dominating
guard conditions at a specific use site.  If the bounded fixpoint does not
converge within the iteration cap, the whole result degrades to ``TOP`` rather
than risk returning a still-ascending (unsound, too-narrow) interval.

The domain is deliberately small and conservative: soundness (never claim a
tighter range than reality) matters far more than precision, because the only
consumers turn a *proven* fact (index in range, divisor is exactly zero) into a
diagnostic decision.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from .cfg import CFGFunction
from .expr_ast import (
    BinOp,
    ExprBinary,
    ExprLiteral,
    ExprNode,
    ExprUnary,
    ExprVar,
    UnaryOp,
)
from .ir import IRAssignConst, IRAssignExpr, IRIncr
from .ssa import SSAFunction, SSAValueKey

# A bound is an int, or None for an infinity (sign given by position).
Bound = int | None


@dataclass(frozen=True, slots=True)
class Interval:
    """A closed integer interval ``[lo, hi]``; ``None`` bound = ±infinity.

    ``lo is None`` → -inf, ``hi is None`` → +inf.  The empty/unreached value
    is represented by ``BOTTOM`` (lo > hi sentinel handled via ``is_bottom``).
    """

    lo: Bound
    hi: Bound

    @property
    def is_top(self) -> bool:
        return self.lo is None and self.hi is None

    @property
    def is_bottom(self) -> bool:
        return self.lo is not None and self.hi is not None and self.lo > self.hi

    def contains(self, n: int) -> bool:
        return (self.lo is None or self.lo <= n) and (self.hi is None or n <= self.hi)

    def is_const(self) -> int | None:
        if self.lo is not None and self.lo == self.hi:
            return self.lo
        return None


TOP = Interval(None, None)
BOTTOM = Interval(1, 0)  # canonical empty (lo > hi)


def const(n: int) -> Interval:
    return Interval(n, n)


def nonneg() -> Interval:
    return Interval(0, None)


def join(a: Interval, b: Interval) -> Interval:
    """Least upper bound: the smallest interval containing both."""
    if a.is_bottom:
        return b
    if b.is_bottom:
        return a
    # lo = min(lo) treating None as -inf; hi = max(hi) treating None as +inf
    lo = None if (a.lo is None or b.lo is None) else min(a.lo, b.lo)
    hi = None if (a.hi is None or b.hi is None) else max(a.hi, b.hi)
    return Interval(lo, hi)


def widen(old: Interval, new: Interval) -> Interval:
    """Standard interval widening: any bound that moved outward jumps to ±inf.

    Guarantees termination of the loop fixpoint (the lattice has infinite
    ascending chains otherwise)."""
    if old.is_bottom:
        return new
    if new.is_bottom:
        return old
    # If new.lo is below old.lo (or old already -inf differs), drop to -inf.
    if old.lo is None or new.lo is None or new.lo < old.lo:
        lo: Bound = None
    else:
        lo = old.lo
    if old.hi is None or new.hi is None or new.hi > old.hi:
        hi: Bound = None
    else:
        hi = old.hi
    return Interval(lo, hi)


def add(a: Interval, b: Interval) -> Interval:
    if a.is_bottom or b.is_bottom:
        return BOTTOM
    lo = None if (a.lo is None or b.lo is None) else a.lo + b.lo
    hi = None if (a.hi is None or b.hi is None) else a.hi + b.hi
    return Interval(lo, hi)


def sub(a: Interval, b: Interval) -> Interval:
    if a.is_bottom or b.is_bottom:
        return BOTTOM
    # a - b = [a.lo - b.hi, a.hi - b.lo]
    lo = None if (a.lo is None or b.hi is None) else a.lo - b.hi
    hi = None if (a.hi is None or b.lo is None) else a.hi - b.lo
    return Interval(lo, hi)


def mul(a: Interval, b: Interval) -> Interval:
    if a.is_bottom or b.is_bottom:
        return BOTTOM
    # Only attempt when fully bounded; otherwise TOP (sound).
    alo, ahi, blo, bhi = a.lo, a.hi, b.lo, b.hi
    if alo is None or ahi is None or blo is None or bhi is None:
        return TOP
    corners = [alo * blo, alo * bhi, ahi * blo, ahi * bhi]
    return Interval(min(corners), max(corners))


def negate(a: Interval) -> Interval:
    if a.is_bottom:
        return BOTTOM
    lo = None if a.hi is None else -a.hi
    hi = None if a.lo is None else -a.lo
    return Interval(lo, hi)


def _literal_int(expr: ExprNode) -> int | None:
    """The integer value of an ``ExprLiteral`` (incl. bool), else ``None``."""
    if not isinstance(expr, ExprLiteral):
        return None
    t = expr.text.strip()
    if t in ("true", "yes", "on"):
        return 1
    if t in ("false", "no", "off"):
        return 0
    try:
        return int(t, 0) if t[:2].lower() in ("0x", "0o", "0b") else int(t)
    except (ValueError, IndexError):
        return None


def _eval_expr(expr: ExprNode, env: dict[str, Interval]) -> Interval:
    """Abstract-evaluate *expr* over the current interval environment."""
    match expr:
        case ExprLiteral():
            n = _literal_int(expr)
            return const(n) if n is not None else TOP
        case ExprVar(name=name):
            return env.get(name, TOP)
        case ExprUnary(op=op, operand=operand):
            inner = _eval_expr(operand, env)
            if op is UnaryOp.NEG:
                return negate(inner)
            if op is UnaryOp.POS:
                return inner
            return TOP
        case ExprBinary(op=op, left=left, right=right):
            la = _eval_expr(left, env)
            ra = _eval_expr(right, env)
            if op is BinOp.ADD:
                return add(la, ra)
            if op is BinOp.SUB:
                return sub(la, ra)
            if op is BinOp.MUL:
                return mul(la, ra)
            # Comparisons yield a boolean 0/1.
            if op in (
                BinOp.EQ,
                BinOp.NE,
                BinOp.LT,
                BinOp.LE,
                BinOp.GT,
                BinOp.GE,
                BinOp.AND,
                BinOp.OR,
            ):
                return Interval(0, 1)
            return TOP
        case _:
            return TOP


def intersect(a: Interval, b: Interval) -> Interval:
    """Greatest lower bound: the largest interval contained in both.

    Here ``None`` is the *identity* (-inf for a lower bound, +inf for an upper),
    the opposite of :func:`join` where it is absorbing — so the intersected
    lower bound is the tighter (larger) finite one and the upper the tighter
    (smaller) finite one."""
    if a.is_bottom or b.is_bottom:
        return BOTTOM
    if a.lo is None:
        lo = b.lo
    elif b.lo is None:
        lo = a.lo
    else:
        lo = max(a.lo, b.lo)
    if a.hi is None:
        hi = b.hi
    elif b.hi is None:
        hi = a.hi
    else:
        hi = min(a.hi, b.hi)
    return Interval(lo, hi)


def _guard_interval(op: BinOp, k: int, negate: bool) -> Interval | None:
    """The interval a value satisfies given ``value <op> k`` is true/false.

    Returns the half-line constraint, or ``None`` if *op* is not an
    order/equality comparison we can turn into an interval.
    """
    # Normalise the negated form to the opposite operator.
    if negate:
        flip = {
            BinOp.LT: BinOp.GE,
            BinOp.LE: BinOp.GT,
            BinOp.GT: BinOp.LE,
            BinOp.GE: BinOp.LT,
            BinOp.EQ: BinOp.NE,
            BinOp.NE: BinOp.EQ,
        }
        op = flip.get(op, op)
    match op:
        case BinOp.LT:
            return Interval(None, k - 1)
        case BinOp.LE:
            return Interval(None, k)
        case BinOp.GT:
            return Interval(k + 1, None)
        case BinOp.GE:
            return Interval(k, None)
        case BinOp.EQ:
            return const(k)
        case _:
            return None  # NE and non-comparisons give no single interval


def _const_operand(expr: ExprNode) -> int | None:
    return _literal_int(expr)


def _guard_constraint(cond: ExprNode, name: str, *, negate: bool) -> Interval | None:
    """If *cond* is ``$name <cmp> <int-const>`` (or the const on the left),
    return the interval *name* satisfies when *cond* is true (``negate`` for the
    false edge).  Only a top-level comparison against a literal int is handled —
    a symbolic RHS (``$i < [llength $l]``) needs relational analysis and yields
    ``None`` (sound: no refinement)."""
    if not isinstance(cond, ExprBinary):
        return None
    left, right, op = cond.left, cond.right, cond.op
    # $name <op> K
    if isinstance(left, ExprVar) and left.name == name:
        k = _const_operand(right)
        if k is not None:
            return _guard_interval(op, k, negate)
    # K <op> $name  → rewrite as $name <flipped-op> K
    if isinstance(right, ExprVar) and right.name == name:
        k = _const_operand(left)
        if k is not None:
            mirror = {
                BinOp.LT: BinOp.GT,
                BinOp.LE: BinOp.GE,
                BinOp.GT: BinOp.LT,
                BinOp.GE: BinOp.LE,
                BinOp.EQ: BinOp.EQ,
                BinOp.NE: BinOp.NE,
            }
            return _guard_interval(mirror.get(op, op), k, negate)
    return None


def refine_interval(
    base: dict[SSAValueKey, Interval],
    cfg: CFGFunction,
    ssa: SSAFunction,
    block: str,
    name: str,
    version: int,
) -> Interval:
    """Narrow ``base[(name, version)]`` by the constant-bound guards that hold
    on every path reaching *block*.

    For each dominator *D* of *block* that ends in a branch on a constant-bound
    comparison of *name* (at *D*'s exit version of *name* == *version*), if
    *block* is reached only via *D*'s true (resp. false) edge, intersect with
    the guard's true (resp. false) constraint.  This proves, e.g., that inside
    ``if {$i < 10} { … }`` the value ``$i`` lies in ``[lo, 9]``.
    """
    from .cfg import CFGBranch
    from .loops import dominates

    iv = base.get((name, version), TOP)
    for dn, dblock in cfg.blocks.items():
        if dn == block:
            continue
        term = dblock.terminator
        if not isinstance(term, CFGBranch):
            continue
        sb = ssa.blocks.get(dn)
        if sb is None or sb.exit_versions.get(name) != version:
            continue
        true_dom = dominates(ssa, term.true_target, block)
        false_dom = dominates(ssa, term.false_target, block)
        if true_dom and not false_dom:
            c = _guard_constraint(term.condition, name, negate=False)
        elif false_dom and not true_dom:
            c = _guard_constraint(term.condition, name, negate=True)
        else:
            continue
        if c is not None:
            iv = intersect(iv, c)
    return iv


def _const_int_from_value(text: str) -> int | None:
    """Parse a literal IR value word as an int, if it is exactly one."""
    s = text.strip()
    try:
        return int(s)
    except (ValueError, TypeError):
        return None


def compute_intervals(
    cfg: CFGFunction,
    ssa: SSAFunction,
    values: Mapping[SSAValueKey, object] | None = None,
) -> dict[SSAValueKey, Interval]:
    """Forward interval analysis over the SSA form.

    *values* is the SCCP lattice map (``LatticeValue``); when a value is a
    constant integer we seed ``[c, c]``.  Everything else starts ``BOTTOM`` and
    is refined to fixpoint with loop-header widening.  Unhandled definitions
    become ``TOP`` (sound).
    """
    from .core_analyses import LatticeKind  # local: avoid import cycle

    result: dict[SSAValueKey, Interval] = {}

    def seed_const(key: SSAValueKey) -> Interval | None:
        if values is None:
            return None
        lv = values.get(key)
        if lv is None:
            return None
        if getattr(lv, "kind", None) is LatticeKind.CONST:
            v = getattr(lv, "value", None)
            if isinstance(v, bool):
                return const(1 if v else 0)
            if isinstance(v, int):
                return const(v)
        return None

    def cur(key: SSAValueKey) -> Interval:
        return result.get(key, BOTTOM)

    from .loops import build_loop_forest  # local: avoid import cycle

    loop_headers = set(build_loop_forest(cfg, ssa).headers)
    order = cfg.reverse_postorder()

    # Bounded fixpoint: widen at loop-header phis so ascending chains terminate.
    converged = False
    for iteration in range(_MAX_ITERS):
        changed = False
        for bn in order:
            ssa_block = ssa.blocks.get(bn)
            if ssa_block is None:
                continue
            is_loop_header = bn in loop_headers
            for phi in ssa_block.phis:
                key = (phi.name, phi.version)
                merged = BOTTOM
                for inc in phi.incoming.values():
                    if inc > 0:
                        merged = join(merged, cur((phi.name, inc)))
                    else:
                        merged = join(merged, TOP)
                if is_loop_header:
                    merged = widen(cur(key), merged)
                if merged != cur(key):
                    result[key] = merged
                    changed = True
            for s in ssa_block.statements:
                env = {nm: cur((nm, ver)) for nm, ver in s.uses.items()}
                for nm, ver in s.defs.items():
                    key = (nm, ver)
                    seeded = seed_const(key)
                    if seeded is not None:
                        val = seeded
                    else:
                        val = _transfer(s.statement, nm, env, cur((nm, ver)))
                    if val != cur(key):
                        result[key] = val
                        changed = True
        if not changed:
            converged = True
            break

    if not converged:
        # The fixpoint did not stabilise within the iteration cap, so ``result``
        # may still be ascending — i.e. intervals NARROWER than the true runtime
        # range.  Reporting an out-of-range/div-zero finding from such an
        # under-approximation would be unsound (a false positive), violating this
        # module's "soundness over precision" contract.  Fall back to TOP for
        # every value so no consumer can derive a finding from unconverged data.
        return dict.fromkeys(result, TOP)

    # Final narrowing-ish pass: replace any remaining BOTTOM (unrefined) with
    # TOP so consumers never see the empty sentinel for a reachable value.
    for key, iv in list(result.items()):
        if iv.is_bottom:
            result[key] = TOP
    return result


_MAX_ITERS = 50


def _transfer(stmt: object, name: str, env: dict[str, Interval], old: Interval) -> Interval:
    """Interval produced by *stmt* for its def *name*."""
    if isinstance(stmt, IRAssignConst):
        n = _const_int_from_value(stmt.value)
        return const(n) if n is not None else TOP
    if isinstance(stmt, IRAssignExpr):
        return _eval_expr(stmt.expr, env)
    if isinstance(stmt, IRIncr):
        base = env.get(name, old)
        if base.is_bottom:
            base = TOP
        amt = const(1)
        if stmt.amount is not None:
            n = _const_int_from_value(stmt.amount)
            amt = const(n) if n is not None else TOP
        return add(base, amt)
    return TOP


__all__ = ["Interval", "TOP", "BOTTOM", "const", "compute_intervals"]
