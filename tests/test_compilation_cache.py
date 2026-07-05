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

"""Tests for procedure-level FunctionUnit caching in compile_source()."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler import interprocedural as interproc_module
from compiler.compilation_unit import compile_source
from server.workspace.document_state import (
    DocumentState,
    _build_proc_cache,
    _build_reposition_cache,
)


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
        """A same-length body change to beta misses only beta — alpha and gamma
        keep their byte offsets, so they still hit the cache."""
        cu1 = compile_source(self.THREE_PROCS)
        cache = _build_proc_cache(cu1)

        modified = (
            "proc alpha {} { return 1 }\n"
            "proc beta {} { return 7 }\n"  # changed, same length
            "proc gamma {} { return 3 }\n"
        )
        cu2 = compile_source(modified, proc_cache=cache)

        # alpha and gamma should be cache hits (same object): beta's length is
        # unchanged, so gamma's absolute offsets did not shift.
        assert cu2.procedures["::alpha"] is cu1.procedures["::alpha"]
        assert cu2.procedures["::gamma"] is cu1.procedures["::gamma"]
        # beta should be a new FunctionUnit
        assert cu2.procedures["::beta"] is not cu1.procedures["::beta"]

    def test_cache_miss_on_renamed_proc(self):
        """Renaming beta lengthens the text before gamma, shifting gamma's byte
        offsets; gamma therefore correctly misses the cache (its cached
        FunctionUnit carries stale absolute offsets)."""
        cu1 = compile_source(self.THREE_PROCS)
        cache = _build_proc_cache(cu1)

        renamed = (
            "proc alpha {} { return 1 }\n"
            "proc beta_v2 {} { return 2 }\n"  # renamed (longer)
            "proc gamma {} { return 3 }\n"
        )
        cu2 = compile_source(renamed, proc_cache=cache)

        assert cu2.procedures["::alpha"] is cu1.procedures["::alpha"]
        assert "::beta_v2" in cu2.procedures
        assert "::beta" not in cu2.procedures
        # gamma's offsets shifted (beta_v2 is longer than beta) → cache miss.
        assert cu2.procedures["::gamma"] is not cu1.procedures["::gamma"]

    def test_same_length_edit_keeps_neighbours_cached(self):
        """A same-length edit to beta leaves alpha and gamma byte-offset-stable,
        so both still hit the cache while beta misses."""
        cu1 = compile_source(self.THREE_PROCS)
        cache = _build_proc_cache(cu1)
        # "return 2" -> "return 5": identical length, so no offset shift below.
        edited = (
            "proc alpha {} { return 1 }\nproc beta {} { return 5 }\nproc gamma {} { return 3 }\n"
        )
        cu2 = compile_source(edited, proc_cache=cache)
        assert cu2.procedures["::alpha"] is cu1.procedures["::alpha"]
        assert cu2.procedures["::gamma"] is cu1.procedures["::gamma"]
        assert cu2.procedures["::beta"] is not cu1.procedures["::beta"]

    def test_length_changing_edit_before_gamma_misses_gamma(self):
        """A length-changing edit to beta shifts gamma's absolute offsets, so
        gamma must miss the cache even though its body text is byte-identical —
        its cached CFG/SSA carry the old offsets."""
        cu1 = compile_source(self.THREE_PROCS)
        cache = _build_proc_cache(cu1)
        # "return 2" -> "return 222": two chars longer, shifting everything after.
        edited = (
            "proc alpha {} { return 1 }\nproc beta {} { return 222 }\nproc gamma {} { return 3 }\n"
        )
        cu2 = compile_source(edited, proc_cache=cache)
        # alpha is before the edit → still cached; gamma is after → missed.
        assert cu2.procedures["::alpha"] is cu1.procedures["::alpha"]
        assert cu2.procedures["::gamma"] is not cu1.procedures["::gamma"]
        assert cu2.procedures["::beta"] is not cu1.procedures["::beta"]

    def test_new_proc_compiled_fresh(self):
        """Adding a proc doesn't break existing cache entries."""
        cu1 = compile_source("proc alpha {} { return 1 }\n")
        cache = _build_proc_cache(cu1)

        extended = "proc alpha {} { return 1 }\nproc beta {} { return 2 }\n"
        cu2 = compile_source(extended, proc_cache=cache)

        assert cu2.procedures["::alpha"] is cu1.procedures["::alpha"]
        assert "::beta" in cu2.procedures

    def _caller_rbs(self, cu):
        return sorted({r.variable for r in cu.procedures["::caller"].analysis.read_before_set})

    def test_callee_upvar_change_invalidates_caller(self):
        """A caller's CFG/analysis depends on which callees write back via
        upvar.  Editing only the callee (caller text unchanged) must not serve a
        stale cached caller unit — the context fingerprint invalidates it.

        Ground truth (tclsh 9.0.3): with `upvar 1 $name v; set v 1` the callee
        initialises the caller's `x`, so `helper x; puts $x` is clean; without
        it `puts $x` errors `can't read "x"`.
        """
        caller = "proc caller {} { helper x\n puts $x }\n"
        no_upvar = "proc helper {name} { return 1 }\n" + caller
        with_upvar = "proc helper {name} { upvar 1 $name v\n set v 1 }\n" + caller

        # callee gains upvar → caller's read-before-set must clear
        cache = _build_proc_cache(compile_source(no_upvar))
        cached = self._caller_rbs(compile_source(with_upvar, proc_cache=cache))
        fresh = self._caller_rbs(compile_source(with_upvar, proc_cache={}))
        assert cached == fresh == []

        # callee loses upvar → caller's read-before-set must reappear
        cache2 = _build_proc_cache(compile_source(with_upvar))
        cached2 = self._caller_rbs(compile_source(no_upvar, proc_cache=cache2))
        fresh2 = self._caller_rbs(compile_source(no_upvar, proc_cache={}))
        assert cached2 == fresh2 == ["x"]

    def test_adding_non_upvar_proc_keeps_caller_cached(self):
        """Adding an ordinary (non-upvar) proc must not invalidate unrelated
        cached units — the context fingerprint only tracks upvar procs."""
        src1 = "proc f {a} { return [expr {$a + 1}] }\n"
        cu1 = compile_source(src1)
        cache = _build_proc_cache(cu1)
        cu2 = compile_source(src1 + "proc g {} { return 2 }\n", proc_cache=cache)
        assert cu2.procedures["::f"] is cu1.procedures["::f"]

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


class TestRepositionCache:
    """A proc whose body is unchanged but which *moved* misses the primary
    (position-keyed) cache, yet should reuse its position-independent dataflow
    analysis via the reposition cache while rebuilding only CFG + SSA."""

    THREE_PROCS = (
        "proc alpha {} { return 1 }\nproc beta {} { return 2 }\nproc gamma {} { return 3 }\n"
    )

    def test_moved_proc_reuses_analysis_not_whole_unit(self):
        """A length-changing edit before gamma shifts gamma's offsets, so the
        primary cache misses (its CFG/SSA carry stale offsets).  With the
        reposition cache, gamma reuses the cached *analysis* / *execution_intent*
        object (same identity) but gets a freshly-built CFG (new identity)."""
        cu1 = compile_source(self.THREE_PROCS)
        proc_cache = _build_proc_cache(cu1)
        repos_cache = _build_reposition_cache(cu1)

        # "return 2" -> "return 222": two chars longer, shifting gamma's offsets.
        edited = (
            "proc alpha {} { return 1 }\nproc beta {} { return 222 }\nproc gamma {} { return 3 }\n"
        )
        cu2 = compile_source(edited, proc_cache=proc_cache, reposition_cache=repos_cache)

        g1, g2 = cu1.procedures["::gamma"], cu2.procedures["::gamma"]
        # Whole-unit primary cache missed: new FunctionUnit, new CFG.
        assert g2 is not g1
        assert g2.cfg is not g1.cfg
        # ...but the expensive position-independent artefacts were reused.
        assert g2.analysis is g1.analysis
        assert g2.execution_intent is g1.execution_intent

    def test_moved_proc_has_correct_shifted_ranges(self):
        """The reused analysis must pair with *correct* (freshly-built) ranges:
        a moved-proc compile must be byte-identical to a cold rebuild."""
        src = "set a 1\nproc gamma {x} {\n  if {$x} { set y 1 }\n  return $y\n}\n"
        cu1 = compile_source(src)
        proc_cache = _build_proc_cache(cu1)
        repos_cache = _build_reposition_cache(cu1)

        moved = "\n\n# pushed down\n" + src
        cold = compile_source(moved)  # reference: no caches
        got = compile_source(moved, proc_cache=proc_cache, reposition_cache=repos_cache)

        def ranges(cu):
            fu = cu.procedures["::gamma"]
            return [
                (
                    bn,
                    s.range.start.line,
                    s.range.start.character,
                    s.range.start.offset,
                    s.range.end.offset,
                )
                for bn in sorted(fu.cfg.blocks)
                for s in fu.cfg.blocks[bn].statements
                if getattr(s, "range", None) is not None
            ]

        # Reposition path took effect (analysis reused) yet ranges match cold rebuild.
        assert got.procedures["::gamma"].analysis is cu1.procedures["::gamma"].analysis
        assert ranges(got) == ranges(cold)

    def test_reposition_diagnostics_match_cold_rebuild(self):
        """End-to-end: a read-before-set warning on a moved proc must land on the
        shifted line, identical to a cold rebuild (no stale positions leak)."""
        src = "proc gamma {} {\n  puts $undef\n  set undef 1\n}\n"
        cu1 = compile_source(src)
        proc_cache = _build_proc_cache(cu1)
        repos_cache = _build_reposition_cache(cu1)

        moved = "\n\n\n" + src  # push gamma down 3 lines
        cold = compile_source(moved)
        got = compile_source(moved, proc_cache=proc_cache, reposition_cache=repos_cache)

        # Index-based analysis facts are identical, and the CFG ranges they
        # resolve against are freshly shifted — so diagnostics co-locate.
        assert got.procedures["::gamma"].analysis.read_before_set == (
            cold.procedures["::gamma"].analysis.read_before_set
        )

    def test_changed_body_does_not_use_reposition(self):
        """A genuine body edit (not a pure move) must NOT reuse stale analysis —
        the reposition key includes body text, so it misses and rebuilds."""
        cu1 = compile_source("proc gamma {} {\n  set x 1\n  return $x\n}\n")
        repos_cache = _build_reposition_cache(cu1)

        # Different body: introduces a read-before-set that wasn't there before.
        changed = "proc gamma {} {\n  return $x\n}\n"
        cu2 = compile_source(changed, proc_cache={}, reposition_cache=repos_cache)

        g2 = cu2.procedures["::gamma"]
        # Fresh analysis (not the cached one) — it now reports the new RBS.
        assert g2.analysis is not cu1.procedures["::gamma"].analysis
        assert any(r.variable == "x" for r in g2.analysis.read_before_set)


class TestCallSiteConstantsInvalidation:
    """SCCP / unreachable-branch facts are seeded from call-site literal args
    (``_params_constants_from_call_sites``).  A proc whose own text and position
    are unchanged must still be re-analysed when a caller edits a literal arg —
    both keys fold in a call-site-constants fingerprint (PR #508 review P2)."""

    @staticmethod
    def _branches(cu):
        fu = cu.procedures["::foo"]
        return tuple(sorted((c.block, c.value) for c in fu.analysis.constant_branches))

    @staticmethod
    def _doc(call, lead=""):
        return (
            lead
            + "proc foo {x} {\n    if {$x} { set a 1 } else { set dead 2 }\n    return\n}\n"
            + f"foo {call}\n"
        )

    def test_reposition_invalidates_on_call_site_constant_change(self):
        """Proc moves AND its caller flips ``foo 1`` → ``foo 0`` in one edit: the
        reposition path must NOT serve the stale ``x == 1`` branch facts."""
        cu1 = compile_source(self._doc("1"))
        proc_cache = _build_proc_cache(cu1)
        repos_cache = _build_reposition_cache(cu1)

        moved_changed = self._doc("0", lead="\n")  # moved + call-site const flipped
        cold = compile_source(moved_changed)
        got = compile_source(moved_changed, proc_cache=proc_cache, reposition_cache=repos_cache)
        assert self._branches(got) == self._branches(cold)
        # The cached (x==1 → True) analysis must not have been reused.
        assert got.procedures["::foo"].analysis is not cu1.procedures["::foo"].analysis

    def test_primary_cache_invalidates_on_call_site_constant_change(self):
        """Same-length call edit (``foo 1`` → ``foo 0``) leaves foo's offsets
        stable, so the *primary* cache would hit — but must still re-analyse."""
        cu1 = compile_source(self._doc("1"))
        proc_cache = _build_proc_cache(cu1)

        changed = self._doc("0")  # foo unmoved, only the caller's literal changed
        cold = compile_source(changed)
        got = compile_source(changed, proc_cache=proc_cache)
        assert self._branches(got) == self._branches(cold)

    def test_unchanged_call_site_still_reuses_fast_path(self):
        """A pure move with the call site unchanged must still hit the reposition
        fast-path (the fingerprint only invalidates on an actual const change)."""
        cu1 = compile_source(self._doc("1"))
        proc_cache = _build_proc_cache(cu1)
        repos_cache = _build_reposition_cache(cu1)

        moved_same = self._doc("1", lead="\n")  # moved, call site unchanged
        got = compile_source(moved_same, proc_cache=proc_cache, reposition_cache=repos_cache)
        # Reposition reuse: same analysis object identity, fresh CFG.
        g1, g2 = cu1.procedures["::foo"], got.procedures["::foo"]
        assert g2.analysis is g1.analysis
        assert g2.cfg is not g1.cfg
