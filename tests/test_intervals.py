"""Tests for the integer interval abstract domain (Phase 3)."""

from __future__ import annotations

from compiler.cfg import CFGBranch, build_cfg
from compiler.core_analyses import _sccp
from compiler.intervals import (
    TOP,
    Interval,
    add,
    compute_intervals,
    const,
    intersect,
    join,
    mul,
    negate,
    refine_interval,
    sub,
    widen,
)
from compiler.lowering import lower_to_ir
from compiler.ssa import build_ssa


class TestIntervalArithmetic:
    def test_add(self):
        assert add(const(2), const(3)) == const(5)
        assert add(Interval(0, 10), Interval(1, 2)) == Interval(1, 12)
        # +inf bound stays unbounded.
        assert add(Interval(0, None), const(1)) == Interval(1, None)

    def test_sub(self):
        assert sub(const(5), const(3)) == const(2)
        # [a.lo - b.hi, a.hi - b.lo]
        assert sub(Interval(0, 10), Interval(1, 2)) == Interval(-2, 9)

    def test_mul(self):
        assert mul(const(2), const(3)) == const(6)
        # negative * range picks the right corners
        assert mul(Interval(-2, 3), Interval(4, 5)) == Interval(-10, 15)
        # unbounded operand -> TOP (sound)
        assert mul(Interval(0, None), const(2)) is TOP

    def test_negate(self):
        assert negate(Interval(1, 5)) == Interval(-5, -1)
        assert negate(Interval(None, 0)) == Interval(0, None)

    def test_join(self):
        assert join(const(1), const(3)) == Interval(1, 3)
        assert join(Interval(0, 5), Interval(10, 20)) == Interval(0, 20)
        # join with an unbounded side stays unbounded on that end.
        assert join(Interval(0, None), const(3)) == Interval(0, None)

    def test_widen_sends_growing_bound_to_infinity(self):
        # Upper bound moved out -> +inf; lower stable -> kept.
        assert widen(Interval(0, 0), Interval(0, 1)) == Interval(0, None)
        # Lower bound moved out -> -inf.
        assert widen(Interval(0, 5), Interval(-1, 5)) == Interval(None, 5)
        # Stable -> unchanged.
        assert widen(Interval(0, 5), Interval(0, 5)) == Interval(0, 5)


def _intervals(src: str, proc: str):
    m = lower_to_ir(src)
    cm = build_cfg(m)
    cfg = cm.procedures[proc]
    ssa = build_ssa(cfg)
    values, _, _, _ = _sccp(cfg, ssa)
    return compute_intervals(cfg, ssa, values)


class TestComputeIntervals:
    def test_constant_arithmetic(self):
        iv = _intervals(
            "proc f {} { set i 0\n set n 10\n set j [expr {$i + $n}]\n return $j }", "::f"
        )
        assert iv[("j", 1)] == const(10)

    def test_loop_induction_widens_soundly(self):
        iv = _intervals("proc f {} { for {set i 0} {$i < 10} {incr i} { puts $i } }", "::f")
        # The loop-header phi for i is widened to [0, +inf): sound (i >= 0),
        # the upper bound is unbounded without guard narrowing.
        header = iv[("i", 2)]
        assert header.lo == 0 and header.hi is None

    def test_unknown_is_top(self):
        # A value derived from an opaque call has no bound.
        iv = _intervals("proc f {} { set x [someUnknownCmd]\n return $x }", "::f")
        assert iv[("x", 1)].is_top


class TestIntersect:
    def test_intersect_tightens_both_bounds(self):
        # None is the identity here (-inf low / +inf high), opposite of join.
        assert intersect(Interval(0, None), Interval(None, 9)) == Interval(0, 9)
        assert intersect(Interval(0, 100), Interval(5, 50)) == Interval(5, 50)
        assert intersect(Interval(0, None), Interval(0, None)) == Interval(0, None)


class TestGuardNarrowing:
    def _setup(self, src: str, proc: str):
        m = lower_to_ir(src)
        cm = build_cfg(m)
        cfg = cm.procedures[proc]
        ssa = build_ssa(cfg)
        values, _, _, _ = _sccp(cfg, ssa)
        return cfg, ssa, compute_intervals(cfg, ssa, values)

    def test_constant_bound_loop_body_narrows(self):
        # Inside `for {set i 0} {$i < 10} {incr i}` the body is dominated by the
        # header's true edge, so $i narrows from [0,+inf) (widened) to [0,9].
        cfg, ssa, iv = self._setup(
            "proc f {} { for {set i 0} {$i < 10} {incr i} { puts $i } }", "::f"
        )
        body = next(
            b.terminator.true_target
            for b in cfg.blocks.values()
            if isinstance(b.terminator, CFGBranch)
        )
        r = refine_interval(iv, cfg, ssa, body, "i", 2)
        assert (r.lo, r.hi) == (0, 9)

    def test_symbolic_bound_does_not_narrow(self):
        # `$i < [llength $l]` has a symbolic RHS — a non-relational interval
        # domain cannot turn it into a constant bound, so no (unsound) narrowing.
        cfg, ssa, iv = self._setup(
            "proc f {l} { for {set i 0} {$i < [llength $l]} {incr i} { puts $i } }", "::f"
        )
        body = next(
            b.terminator.true_target
            for b in cfg.blocks.values()
            if isinstance(b.terminator, CFGBranch)
        )
        r = refine_interval(iv, cfg, ssa, body, "i", 2)
        # Lower bound still 0 (from init/widen); upper stays unbounded.
        assert r.lo == 0 and r.hi is None
