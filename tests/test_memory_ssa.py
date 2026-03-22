"""Tests for memory-SSA construction and alias detection."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.cfg import build_cfg
from core.compiler.lowering import lower_to_ir
from core.compiler.memory_ssa import (
    MemoryLocationKind,
    MemoryOpKind,
    build_memory_ssa,
    compute_aliases,
)
from core.compiler.ssa import build_ssa


def _build_proc(source: str, proc_name: str | None = None):
    """Helper: source → SSA for a procedure → memory-SSA."""
    mod = lower_to_ir(source)
    cfg_mod = build_cfg(mod)
    if proc_name:
        for qname, cfg in cfg_mod.procedures.items():
            if proc_name in qname:
                ssa = build_ssa(cfg)
                return build_memory_ssa(ssa), ssa
    # Use top-level
    ssa = build_ssa(cfg_mod.top_level)
    return build_memory_ssa(ssa), ssa


def _build_top(source: str):
    """Helper: source → SSA for top-level → memory-SSA."""
    mod = lower_to_ir(source)
    cfg_mod = build_cfg(mod)
    ssa = build_ssa(cfg_mod.top_level)
    return build_memory_ssa(ssa), ssa


class TestAliasDetection:
    def test_global_alias(self):
        source = "proc foo {} { global gvar\n set gvar 42 }"
        mem, _ = _build_proc(source, "foo")
        assert len(mem.alias_sets) == 1
        aset = mem.alias_sets[0]
        assert aset.reason == "global"
        assert "gvar" in aset.names

    def test_upvar_literal_alias(self):
        source = "proc foo {} { upvar 1 caller_x local_x\n set local_x 42 }"
        mem, _ = _build_proc(source, "foo")
        assert len(mem.alias_sets) == 1
        aset = mem.alias_sets[0]
        assert aset.reason == "upvar"
        assert "local_x" in aset.names
        assert "caller_x" in aset.names

    def test_no_aliases_for_local_only(self):
        source = "proc foo {} { set x 1\n set y $x }"
        mem, _ = _build_proc(source, "foo")
        assert len(mem.alias_sets) == 0

    def test_multiple_globals(self):
        source = "proc foo {} { global a b\n set a 1\n set b 2 }"
        mem, _ = _build_proc(source, "foo")
        assert len(mem.alias_sets) == 2

    def test_may_alias(self):
        source = "proc foo {} { global gvar\n set gvar 42 }"
        mem, _ = _build_proc(source, "foo")
        assert mem.may_alias("gvar", "gvar")

    def test_aliased_names(self):
        source = "proc foo {} { global a\n upvar 1 b local_b }"
        mem, _ = _build_proc(source, "foo")
        names = mem.aliased_names
        assert "a" in names
        assert "local_b" in names or "b" in names


class TestMemoryOps:
    def test_global_memory_defs(self):
        source = "proc foo {} { global gvar\n set gvar 42 }"
        mem, _ = _build_proc(source, "foo")
        assert mem.total_memory_defs >= 1

    def test_no_memory_ops_for_local(self):
        source = "proc foo {} { set x 1 }"
        mem, _ = _build_proc(source, "foo")
        assert len(mem.memory_ops) == 0

    def test_clobber_on_eval(self):
        source = "proc foo {} { global x\n eval {set x 1} }"
        mem, _ = _build_proc(source, "foo")
        assert mem.total_clobbers >= 1


class TestMemorySSAIntegration:
    def test_function_analysis_has_memory_ssa(self):
        """Verify memory-SSA is available through FunctionAnalysis."""
        from core.compiler.core_analyses import analyse_function

        source = "proc foo {} { global gvar\n set gvar 42 }"
        mod = lower_to_ir(source)
        cfg_mod = build_cfg(mod)
        for qname, cfg in cfg_mod.procedures.items():
            ssa = build_ssa(cfg)
            analysis = analyse_function(cfg, ssa)
            assert analysis.memory_ssa is not None
            assert len(analysis.memory_ssa.alias_sets) >= 1
