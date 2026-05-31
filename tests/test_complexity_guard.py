"""Tests for the deep-analysis complexity guard (Phase 6).

Pathologically large (machine-generated) bodies skip deep analysis (SSA /
dataflow / analyser-walk / compiler-checks / shimmer / taint / optimiser /
interproc) so a single giant generated proc can't cost tens of seconds.  The
guard is sized well above legitimate hand-written code, so normal procs are
unaffected.
"""

from __future__ import annotations

from compiler.cfg import CFGBlock, CFGFunction, CFGGoto, CFGReturn
from compiler.core_analyses import analyse_function
from compiler.lowering import lower_to_ir
from compiler.ssa import _COMPLEXITY_GUARD_BLOCKS, build_ssa, is_complexity_guarded
from server.features.diagnostics import get_diagnostics


def _chain_cfg(n: int) -> CFGFunction:
    """A straight-line chain of *n* blocks (b0→b1→…→return)."""
    blocks: dict[str, CFGBlock] = {}
    for i in range(n):
        name = f"b{i}"
        if i < n - 1:
            blocks[name] = CFGBlock(
                name=name, statements=(), terminator=CFGGoto(target=f"b{i + 1}")
            )
        else:
            blocks[name] = CFGBlock(name=name, statements=(), terminator=CFGReturn())
    return CFGFunction(name="::big", entry="b0", blocks=blocks)


class TestComplexityGuardBlocks:
    def test_below_threshold_is_analysed(self):
        cfg = _chain_cfg(100)  # well below the ceiling
        assert not is_complexity_guarded(cfg)
        ssa = build_ssa(cfg)
        assert len(ssa.blocks) == len(cfg.blocks)  # real SSA

    def test_above_threshold_is_guarded(self):
        cfg = _chain_cfg(_COMPLEXITY_GUARD_BLOCKS + 5)
        assert is_complexity_guarded(cfg)
        # build_ssa returns a trivial (empty) SSA — no expensive phi/rename walk.
        ssa = build_ssa(cfg)
        assert ssa.blocks == {}
        # analyse_function returns a trivial analysis — no facts.
        an = analyse_function(cfg, ssa)
        assert an.values == {}
        assert an.dead_stores == ()
        assert an.unreachable_blocks == set()  # nothing claimed unreachable


class TestComplexityGuardBodyBytes:
    # A >256KB body built from few, long statements: statement/block-light so
    # compile stays fast, but byte-large so the analyser-walk guard trips.  The
    # body is full of an unknown command (W123 is an analyser-walk diagnostic),
    # so the guard's effect is observable as W123 suppression.
    _LINE = 'nonexistentcmd_xyz "' + "A" * 250 + '"\n'  # ~272 bytes
    _HUGE_BODY = _LINE * 1100  # ~300 KB, 1100 statements

    def _src(self) -> str:
        return f"proc big {{}} {{\n{self._HUGE_BODY}}}\n"

    def test_huge_body_skips_analyser_walk(self):
        # The analyser would emit a W123 for each unknown command; the body
        # guard skips deep-walking the body, so none are emitted from it.
        codes = {d.code for d in get_diagnostics(self._src())}
        assert "W123" not in codes

    def test_normal_proc_unknown_command_still_flagged(self):
        # Control: a small proc with the same unknown command still gets W123.
        codes = {d.code for d in get_diagnostics("proc small {} { nonexistentcmd_xyz a }\n")}
        assert "W123" in codes

    def test_huge_body_does_not_crash_and_is_fast(self):
        import time

        t = time.perf_counter()
        get_diagnostics(self._src())  # must not raise / hang
        assert time.perf_counter() - t < 20.0  # generous; guarded path is fast

    def test_guarded_proc_still_recorded(self):
        # The proc itself is still recorded (resolves as a command) — only its
        # body is left un-walked.
        ir = lower_to_ir(self._src())
        assert "::big" in ir.procedures

    def test_byte_guarded_proc_skips_ssa_and_dataflow(self):
        # The byte guard is now decided BEFORE SSA/dataflow, so a block-light
        # but byte-huge proc gets a trivial FunctionUnit (empty SSA + no facts)
        # rather than a full SSA + dataflow walk built and then discarded.
        from compiler.compilation_unit import compile_source

        cu = compile_source(self._src())
        big = cu.procedures["::big"]
        # Block-light: the block-count guard would NOT fire on this body ...
        assert not is_complexity_guarded(big.cfg)
        # ... yet it is guarded (byte-heavy) and its SSA/dataflow were skipped.
        assert big.complexity_guarded is True
        assert big.ssa.blocks == {}
        assert big.analysis.values == {}


class TestForceGuardShortCircuit:
    """``force_guard=True`` makes build_ssa / analyse_function skip the expensive
    walk for a body the caller already judged too large (the byte guard, decided
    before SSA) — identical to the block-count short-circuit."""

    def test_force_guard_build_ssa_is_trivial(self):
        cfg = _chain_cfg(3)
        assert not is_complexity_guarded(cfg)
        assert len(build_ssa(cfg).blocks) == len(cfg.blocks)  # full SSA by default
        assert build_ssa(cfg, force_guard=True).blocks == {}  # forced trivial

    def test_force_guard_analyse_is_trivial(self):
        cfg = _chain_cfg(3)
        ssa = build_ssa(cfg, force_guard=True)
        an = analyse_function(cfg, ssa, force_guard=True)
        assert an.values == {}
        assert an.dead_stores == ()
        assert an.unreachable_blocks == set()
