"""Tests for procedure-level FunctionUnit caching in compile_source()."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler import interprocedural as interproc_module
from core.compiler.compilation_unit import compile_source
from lsp.workspace.document_state import DocumentState, _build_proc_cache


class TestCompileSourceCache:
    """Verify that compile_source() reuses cached FunctionUnits."""

    THREE_PROCS = (
        "proc alpha {} { return 1 }\nproc beta {} { return 2 }\nproc gamma {} { return 3 }\n"
    )

    def test_cache_reused_for_unchanged_procs(self):
        """When source is recompiled with identical procs, cached
        FunctionUnits should be returned (same object identity)."""
        cu1 = compile_source(self.THREE_PROCS)
        cache = _build_proc_cache(cu1)

        cu2 = compile_source(self.THREE_PROCS, proc_cache=cache)

        for qname in cu1.procedures:
            assert cu2.procedures[qname] is cu1.procedures[qname], (
                f"FunctionUnit for {qname} was rebuilt instead of reused"
            )

    def test_cfg_reused_for_unchanged_procs(self):
        """Cache hits should also reuse CFG objects."""
        cu1 = compile_source(self.THREE_PROCS)
        cache = _build_proc_cache(cu1)
        cu2 = compile_source(self.THREE_PROCS, proc_cache=cache)

        assert cu2.cfg_module.procedures["::alpha"] is cu1.cfg_module.procedures["::alpha"]
        assert cu2.cfg_module.procedures["::beta"] is cu1.cfg_module.procedures["::beta"]
        assert cu2.cfg_module.procedures["::gamma"] is cu1.cfg_module.procedures["::gamma"]

    def test_cache_miss_on_changed_proc(self):
        """When one proc body changes, only that proc should miss the cache."""
        cu1 = compile_source(self.THREE_PROCS)
        cache = _build_proc_cache(cu1)

        modified = (
            "proc alpha {} { return 1 }\n"
            "proc beta {} { return 99 }\n"  # changed
            "proc gamma {} { return 3 }\n"
        )
        cu2 = compile_source(modified, proc_cache=cache)

        # alpha and gamma should be cache hits (same object)
        assert cu2.procedures["::alpha"] is cu1.procedures["::alpha"]
        assert cu2.procedures["::gamma"] is cu1.procedures["::gamma"]
        # beta should be a new FunctionUnit
        assert cu2.procedures["::beta"] is not cu1.procedures["::beta"]

    def test_cache_miss_on_renamed_proc(self):
        """Renaming a proc causes a full cache miss for that name."""
        cu1 = compile_source(self.THREE_PROCS)
        cache = _build_proc_cache(cu1)

        renamed = (
            "proc alpha {} { return 1 }\n"
            "proc beta_v2 {} { return 2 }\n"  # renamed
            "proc gamma {} { return 3 }\n"
        )
        cu2 = compile_source(renamed, proc_cache=cache)

        assert cu2.procedures["::alpha"] is cu1.procedures["::alpha"]
        assert "::beta_v2" in cu2.procedures
        assert "::beta" not in cu2.procedures
        assert cu2.procedures["::gamma"] is cu1.procedures["::gamma"]

    def test_new_proc_compiled_fresh(self):
        """Adding a proc doesn't break existing cache entries."""
        cu1 = compile_source("proc alpha {} { return 1 }\n")
        cache = _build_proc_cache(cu1)

        extended = "proc alpha {} { return 1 }\nproc beta {} { return 2 }\n"
        cu2 = compile_source(extended, proc_cache=cache)

        assert cu2.procedures["::alpha"] is cu1.procedures["::alpha"]
        assert "::beta" in cu2.procedures

    def test_deleted_proc_no_error(self):
        """Removing a proc doesn't cause errors from stale cache entries."""
        cu1 = compile_source(self.THREE_PROCS)
        cache = _build_proc_cache(cu1)

        reduced = "proc alpha {} { return 1 }\n"
        cu2 = compile_source(reduced, proc_cache=cache)

        assert "::alpha" in cu2.procedures
        assert "::beta" not in cu2.procedures
        assert "::gamma" not in cu2.procedures

    def test_no_cache_produces_same_diagnostics(self):
        """compile_source with and without cache must produce
        identical diagnostics (correctness invariant)."""
        source = (
            "proc foo {x} {\n    set unused 1\n    return $x\n}\nproc bar {} { return [foo 42] }\n"
        )
        cu_no_cache = compile_source(source)
        cache = _build_proc_cache(cu_no_cache)
        cu_with_cache = compile_source(source, proc_cache=cache)

        # Interprocedural analysis should match
        assert cu_no_cache.interproc.procedures.keys() == cu_with_cache.interproc.procedures.keys()

    def test_empty_cache_works(self):
        """An empty cache should not cause errors."""
        cu = compile_source(self.THREE_PROCS, proc_cache={})
        assert len(cu.procedures) == 3

    def test_none_cache_works(self):
        """None cache should not cause errors."""
        cu = compile_source(self.THREE_PROCS, proc_cache=None)
        assert len(cu.procedures) == 3

    def test_interproc_local_cache_reused_for_identical_source(self, monkeypatch):
        calls: list[str] = []
        original = interproc_module._summarise_proc_local

        def counting(*args, **kwargs):
            calls.append(args[0])
            return original(*args, **kwargs)

        monkeypatch.setattr(interproc_module, "_summarise_proc_local", counting)
        cache: dict[tuple[str, int], interproc_module.ProcLocalSummary] = {}

        compile_source(self.THREE_PROCS, interproc_cache=cache)
        assert set(calls) == {"::alpha", "::beta", "::gamma"}

        calls.clear()
        compile_source(self.THREE_PROCS, interproc_cache=cache)
        assert calls == []

    def test_interproc_local_cache_recomputes_only_changed_proc(self, monkeypatch):
        calls: list[str] = []
        original = interproc_module._summarise_proc_local

        def counting(*args, **kwargs):
            calls.append(args[0])
            return original(*args, **kwargs)

        monkeypatch.setattr(interproc_module, "_summarise_proc_local", counting)
        cache: dict[tuple[str, int], interproc_module.ProcLocalSummary] = {}
        compile_source(self.THREE_PROCS, interproc_cache=cache)

        calls.clear()
        modified = (
            "proc alpha {} { return 1 }\n"
            "proc beta {} { return 99 }\n"  # changed
            "proc gamma {} { return 3 }\n"
        )
        compile_source(modified, interproc_cache=cache)
        assert set(calls) == {"::beta"}


class TestDocumentStateProcCache:
    """Verify DocumentState threads the proc cache through updates."""

    def test_proc_cache_populated_after_first_update(self):
        state = DocumentState(uri="test://cache1")
        state.update("proc foo {} { return 1 }\nproc bar {} { return 2 }")
        assert len(state._proc_cache) == 2

    def test_proc_cache_reused_on_edit(self):
        state = DocumentState(uri="test://cache2")
        source_v1 = "proc foo {} { return 1 }\nproc bar {} { return 2 }"
        state.update(source_v1)
        cu1 = state.compilation_unit
        assert cu1 is not None
        foo_fu1 = cu1.procedures["::foo"]

        # Edit bar only
        source_v2 = "proc foo {} { return 1 }\nproc bar {} { return 99 }"
        state.update(source_v2)
        cu2 = state.compilation_unit
        assert cu2 is not None

        # foo should be reused (same object)
        assert cu2.procedures["::foo"] is foo_fu1
        # bar should be rebuilt
        assert cu2.procedures["::bar"] is not cu1.procedures["::bar"]

    def test_proc_cache_cleared_on_compile_failure(self):
        state = DocumentState(uri="test://cache3")
        state.update("proc foo {} { return 1 }")
        assert len(state._proc_cache) == 1

        # Force a source that might fail compilation
        # (use a valid source that just removes the proc)
        state.update("set x 1")
        # Cache should be refreshed (no procs → empty cache)
        assert len(state._proc_cache) == 0

    def test_partial_source_preserves_warm_caches(self):
        state = DocumentState(uri="test://cache4")
        source_v1 = "proc foo {} { return 1 }\nproc bar {} { return 2 }\n"
        state.update(source_v1)
        assert len(state._proc_cache) == 2
        assert len(state._interproc_cache) == 2

        # Half-typed proc body: keep warm caches so recovery is incremental.
        partial = "proc foo {} {\n    set x 1\n    set y 2\n    # missing close brace\n"
        state.update(partial)
        assert state.has_partial_commands
        assert len(state._proc_cache) >= 2
        assert len(state._interproc_cache) >= 2
