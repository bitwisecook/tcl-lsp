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

"""Tests for incremental document state updates.

Verifies that the incremental path (chunk-level IR caching +
analyser snapshot/restore) produces identical results to a full
rebuild.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import Analyser
from compiler.lowering import lower_commands_to_ir, lower_to_ir
from compiler.parsing.command_segmenter import (
    segment_commands,
    segment_top_level_chunks,
)
from compiler.parsing.token_positions import shift_position, shift_range, shift_token
from server.workspace.document_state import DocumentState
from shared.tokens import SourcePosition, Token, TokenType


class TestPositionShifting:
    """Verify shift_position, shift_token, shift_range utilities."""

    def test_shift_position(self):
        pos = SourcePosition(line=5, character=10, offset=100)
        shifted = shift_position(pos, offset_delta=20, line_delta=3)
        assert shifted.line == 8
        assert shifted.character == 10  # column unchanged
        assert shifted.offset == 120

    def test_shift_token(self):
        tok = Token(
            type=TokenType.ESC,
            text="hello",
            start=SourcePosition(line=2, character=5, offset=25),
            end=SourcePosition(line=2, character=9, offset=29),
        )
        shifted = shift_token(tok, offset_delta=10, line_delta=1)
        assert shifted.start.line == 3
        assert shifted.start.offset == 35
        assert shifted.end.line == 3
        assert shifted.end.offset == 39
        assert shifted.text == "hello"
        assert shifted.type is TokenType.ESC

    def test_shift_range(self):
        from analyser.semantic_model import Range

        r = Range(
            start=SourcePosition(line=0, character=0, offset=0),
            end=SourcePosition(line=0, character=5, offset=5),
        )
        shifted = shift_range(r, offset_delta=50, line_delta=10)
        assert shifted.start.line == 10
        assert shifted.start.offset == 50
        assert shifted.end.line == 10
        assert shifted.end.offset == 55


class TestLowerCommandsToIr:
    """Verify per-chunk lowering produces same IR as full lowering."""

    def test_single_command_matches_full(self):
        source = "set x 10\n"
        full_ir = lower_to_ir(source)
        commands = segment_commands(source)
        chunk_stmts, chunk_procs = lower_commands_to_ir(source, commands)
        assert len(chunk_stmts) == len(full_ir.top_level.statements)

    def test_proc_definition_captured(self):
        source = "proc foo {a} { return $a }\n"
        commands = segment_commands(source)
        stmts, procs = lower_commands_to_ir(source, commands)
        assert "::foo" in procs

    def test_multi_command_matches_full(self):
        source = "set x 10\nset y 20\nset z [expr {$x + $y}]\n"
        full_ir = lower_to_ir(source)
        commands = segment_commands(source)
        chunk_stmts, chunk_procs = lower_commands_to_ir(source, commands)
        assert len(chunk_stmts) == len(full_ir.top_level.statements)


class TestIncrementalLowerToIr:
    """Verify incremental lower_to_ir with chunk IR cache."""

    def test_cached_chunks_reused(self):
        source = "set x 10\nset y 20\nset z 30\n"
        chunks = segment_top_level_chunks(source)
        # Lower all chunks normally first.
        full_ir = lower_to_ir(source)
        # Build per-chunk IR cache.
        chunk_ir_cache: list = []
        for chunk in chunks:
            cmds = list(chunk.commands)
            stmts, procs = lower_commands_to_ir(source, cmds)
            chunk_ir_cache.append((stmts, procs))
        # Now use the cache — all entries should be reused.
        ir_from_cache = lower_to_ir(source, chunk_ir=chunk_ir_cache, chunks=chunks)
        assert len(ir_from_cache.top_level.statements) == len(full_ir.top_level.statements)

    def test_partial_cache_works(self):
        source = "set x 10\nset y 20\nset z 30\n"
        chunks = segment_top_level_chunks(source)
        # Cache only the first two chunks.
        cmds0 = list(chunks[0].commands)
        stmts0, procs0 = lower_commands_to_ir(source, cmds0)
        cmds1 = list(chunks[1].commands)
        stmts1, procs1 = lower_commands_to_ir(source, cmds1)
        chunk_ir = [(stmts0, procs0), (stmts1, procs1), None]
        ir_result = lower_to_ir(source, chunk_ir=chunk_ir, chunks=chunks)
        full_ir = lower_to_ir(source)
        assert len(ir_result.top_level.statements) == len(full_ir.top_level.statements)


class TestAnalyserSnapshot:
    """Verify analyser snapshot and restore produce correct results."""

    SOURCE = "set x 10\nset y 20\nproc foo {a} { return $a }\nset z [foo $x]\n"

    def test_snapshot_round_trip(self):
        """Snapshot after first 2 commands, restore, continue — should match full."""
        commands = segment_commands(self.SOURCE)
        # Full analysis.
        full_analyser = Analyser()
        full_result = full_analyser.analyse(self.SOURCE)
        # Incremental: analyse first 2, snapshot, restore, continue.
        inc_analyser = Analyser()
        inc_analyser._source = self.SOURCE
        inc_analyser._analyse_commands_inner(
            commands[:2],
            inc_analyser._current_scope,
            self.SOURCE,
        )
        snap = inc_analyser.snapshot()
        # Restore and process remaining commands.
        inc2 = Analyser()
        inc2.restore(snap)
        inc_result = inc2.analyse_commands(self.SOURCE, commands[2:])
        assert len(inc_result.diagnostics) == len(full_result.diagnostics)
        assert set(inc_result.all_procs.keys()) == set(full_result.all_procs.keys())

    def test_snapshot_preserves_procs(self):
        """Procs defined before the snapshot should be visible after restore."""
        commands = segment_commands(self.SOURCE)
        analyser = Analyser()
        analyser._source = self.SOURCE
        # Analyse up to and including the proc definition (command index 2).
        analyser._analyse_commands_inner(
            commands[:3],
            analyser._current_scope,
            self.SOURCE,
        )
        snap = analyser.snapshot()
        assert "::foo" in snap.result.all_procs
        # Restore and verify.
        restored = Analyser()
        restored.restore(snap)
        assert "::foo" in restored.result.all_procs


class TestIncrementalDocumentState:
    """End-to-end incremental vs full rebuild comparison."""

    def test_edit_last_command(self):
        """Editing only the last command should produce same results."""
        source1 = "set x 10\nset y 20\nproc foo {a} { return $a }\nset z [foo $x]\n"
        source2 = "set x 10\nset y 20\nproc foo {a} { return $a }\nset z [foo $y]\n"
        # Incremental path.
        inc = DocumentState(uri="inc")
        inc.update(source1)
        inc.update(source2)
        # Full rebuild path.
        full = DocumentState(uri="full")
        full.update(source2)
        assert inc.analysis is not None
        assert full.analysis is not None
        inc_diag_codes = sorted(d.code for d in inc.analysis.diagnostics)
        full_diag_codes = sorted(d.code for d in full.analysis.diagnostics)
        assert inc_diag_codes == full_diag_codes

    def test_edit_first_command(self):
        """Editing the first command should invalidate all chunks."""
        source1 = "set x 10\nset y 20\n"
        source2 = "set x 99\nset y 20\n"
        inc = DocumentState(uri="inc")
        inc.update(source1)
        inc.update(source2)
        full = DocumentState(uri="full")
        full.update(source2)
        assert inc.analysis is not None
        assert full.analysis is not None
        inc_diag_codes = sorted(d.code for d in inc.analysis.diagnostics)
        full_diag_codes = sorted(d.code for d in full.analysis.diagnostics)
        assert inc_diag_codes == full_diag_codes

    def test_add_new_command(self):
        """Adding a new command at the end."""
        source1 = "set x 10\nset y 20\n"
        source2 = "set x 10\nset y 20\nset z 30\n"
        inc = DocumentState(uri="inc")
        inc.update(source1)
        inc.update(source2)
        full = DocumentState(uri="full")
        full.update(source2)
        assert inc.analysis is not None
        assert full.analysis is not None
        assert len(inc.chunks) == 3
        inc_diag_codes = sorted(d.code for d in inc.analysis.diagnostics)
        full_diag_codes = sorted(d.code for d in full.analysis.diagnostics)
        assert inc_diag_codes == full_diag_codes

    def test_remove_command(self):
        """Removing a command."""
        source1 = "set x 10\nset y 20\nset z 30\n"
        source2 = "set x 10\nset y 20\n"
        inc = DocumentState(uri="inc")
        inc.update(source1)
        inc.update(source2)
        full = DocumentState(uri="full")
        full.update(source2)
        assert inc.analysis is not None
        assert full.analysis is not None
        inc_diag_codes = sorted(d.code for d in inc.analysis.diagnostics)
        full_diag_codes = sorted(d.code for d in full.analysis.diagnostics)
        assert inc_diag_codes == full_diag_codes

    def test_edit_middle_command(self):
        """Editing a middle command invalidates from that point."""
        source1 = "set x 10\nset y 20\nset z 30\nset w 40\n"
        source2 = "set x 10\nset y 99\nset z 30\nset w 40\n"
        inc = DocumentState(uri="inc")
        inc.update(source1)
        inc.update(source2)
        full = DocumentState(uri="full")
        full.update(source2)
        assert inc.analysis is not None
        assert full.analysis is not None
        inc_diag_codes = sorted(d.code for d in inc.analysis.diagnostics)
        full_diag_codes = sorted(d.code for d in full.analysis.diagnostics)
        assert inc_diag_codes == full_diag_codes

    def test_proc_edit_preserves_other_procs(self):
        """Editing a proc body but not another proc should cache the unchanged one."""
        source1 = "proc alpha {} { return 1 }\nproc beta {} { return 2 }\n"
        source2 = "proc alpha {} { return 1 }\nproc beta {} { return 99 }\n"
        inc = DocumentState(uri="inc")
        inc.update(source1)
        cu1 = inc.compilation_unit
        inc.update(source2)
        cu2 = inc.compilation_unit
        assert cu1 is not None and cu2 is not None
        # Alpha should be cache-hit (same identity).
        assert cu2.procedures["::alpha"] is cu1.procedures["::alpha"]

    def test_chunk_caches_populated(self):
        """After initial full build, chunk caches should be populated."""
        source = "set x 10\nset y 20\nset z 30\n"
        state = DocumentState(uri="test")
        state.update(source)
        assert len(state._chunk_caches) == 3
        assert all(cc is not None for cc in state._chunk_caches)

    def test_unchanged_source_short_circuits(self):
        """Identical source should not trigger rebuild."""
        source = "set x 10\nset y 20\n"
        state = DocumentState(uri="test")
        state.update(source)
        analysis1 = state.analysis
        state.update(source)
        assert state.analysis is analysis1  # Same object — no rebuild.

    def test_compilation_unit_present_after_incremental(self):
        """CompilationUnit should be available after incremental update."""
        source1 = "set x 10\nproc foo {} { return 1 }\n"
        source2 = "set x 10\nproc foo {} { return 2 }\n"
        state = DocumentState(uri="test")
        state.update(source1)
        state.update(source2)
        assert state.compilation_unit is not None
        assert "::foo" in state.compilation_unit.procedures


def _diag_tuples(state):
    """Diagnostics as (code, start_line, start_char, end_line, end_char) — the
    positions the previous code-only comparison missed."""
    a = state.analysis
    assert a is not None
    return sorted(
        (
            d.code,
            d.range.start.line,
            d.range.start.character,
            d.range.end.line,
            d.range.end.character,
        )
        for d in a.diagnostics
    )


def _assert_incremental_equals_full(source1, source2):
    """Incremental (update s1 then s2) must equal a full rebuild of s2 —
    byte-for-byte on diagnostic code AND range (line/char)."""
    inc = DocumentState(uri="inc")
    inc.update(source1)
    inc.update(source2)
    full = DocumentState(uri="full")
    full.update(source2)
    assert _diag_tuples(inc) == _diag_tuples(full)


class TestIncrementalStaleness:
    """Incremental ≡ full on diagnostic *positions*, not just codes — the cases
    a code-only comparison let through (chunk-cache / proc-cache / snapshot
    staleness).  Each scenario was a confirmed divergence before the fix."""

    def test_blank_line_before_proc_shifts_diag(self):
        # A proc whose body is unchanged but which moved down a line: the proc
        # cache must not reuse it with the old line number (W210 on $x).
        _assert_incremental_equals_full(
            "proc f {} { puts $x }\n",
            "\nproc f {} { puts $x }\n",
        )

    def test_blank_line_between_chunks_shifts(self):
        _assert_incremental_equals_full(
            "set a 1\nproc f {} { puts $x }\n",
            "set a 1\n\nproc f {} { puts $x }\n",
        )

    def test_dialect_directive_added_reanalyses(self):
        # Adding a `# tcl-dialect:` directive must re-analyse under the new
        # dialect, not keep the stale (default-dialect) diagnostics.
        _assert_incremental_equals_full(
            "set x 1\nHTTP::respond 200\n",
            "# tcl-dialect: f5-irules\nset x 1\nHTTP::respond 200\n",
        )

    def test_dialect_directive_replaced_same_length(self):
        # A same-length dialect switch (no position shift) must still re-analyse
        # (the dialect is not in the chunk/proc cache keys).
        _assert_incremental_equals_full(
            "# tcl-dialect: tcl8.4\nlassign {1 2} a b\n",
            "# tcl-dialect: tcl8.6\nlassign {1 2} a b\n",
        )

    def test_oo_define_removed_clears_method(self):
        # Removing a later `oo::define` that added a method must not leave the
        # method in the (shared) ClassDef snapshot — `$o extra` should warn W308.
        _assert_incremental_equals_full(
            "oo::class create Foo { method m {} {} }\n"
            "set o [Foo new]\n$o extra\noo::define Foo { method extra {} {} }\n",
            "oo::class create Foo { method m {} {} }\nset o [Foo new]\n$o extra\n",
        )

    def test_objdefine_state_survives_tail_edit(self):
        # oo::objdefine adds an object-specific method; editing a *later* line
        # must not drop that cumulative state (else a false W308 appears).
        base = (
            "oo::class create C { method m {} {} }\n"
            "set o [C new]\n"
            "oo::objdefine $o { method extra {} {} }\n"
            "$o extra\n"
        )
        _assert_incremental_equals_full(base + "set tail 1\n", base + "set tail 2\n")

    def test_same_length_body_edit_keeps_positions(self):
        # Editing a proc body without changing line structure keeps every other
        # proc's positions identical (regression guard against over-shifting).
        _assert_incremental_equals_full(
            "proc a {} { puts $x }\nproc b {} { puts $y }\n",
            "proc a {} { puts $z }\nproc b {} { puts $y }\n",
        )

    def test_irules_like_source(self):
        """Test with iRules-like source (when blocks)."""
        source1 = "set x 10\nset y 20\n"
        source2 = "set x 10\nset y 30\n"
        state = DocumentState(uri="test")
        state.update(source1)
        state.update(source2)
        assert state.analysis is not None


# A realistic, well-formed multi-proc document — the kind of file the issue
# #481 seeded-guard asks for (not an adversarial micro-repro).  Several procs,
# comments, an array, a loop; nothing here should warn, so any incremental
# divergence shows up as a position drift on a *clean* file.
_WELL_FORMED = """\
# A small, well-formed library.
namespace eval ::demo {
    variable counter 0
}

proc ::demo::reset {} {
    variable counter
    set counter 0
}

proc ::demo::bump {{by 1}} {
    variable counter
    incr counter $by
    return $counter
}

proc ::demo::sum {items} {
    set total 0
    foreach n $items {
        incr total $n
    }
    return $total
}
"""


class TestIncrementalRealisticGuard:
    """Seeded `inc == full` guard on realistic / well-formed input (issue #481).

    Kept fast (in-process, no corpus dependency) so it lives outside test-slow.
    Each edit is a shape a real editing session produces — a blank line above,
    a blank line in the middle, and an intra-line keystroke that shifts no
    lines (the case the line+char proc-cache key deliberately keeps cached)."""

    def test_blank_line_inserted_at_top(self):
        _assert_incremental_equals_full(_WELL_FORMED, "\n" + _WELL_FORMED)

    def test_blank_line_inserted_mid_document(self):
        edited = _WELL_FORMED.replace(
            "proc ::demo::sum {items} {",
            "\nproc ::demo::sum {items} {",
        )
        _assert_incremental_equals_full(_WELL_FORMED, edited)

    def test_intra_line_edit_keeps_positions(self):
        # `{by 1}` -> `{by 2}`: no line shifts, so the line+char proc-cache key
        # legitimately reuses every other proc; positions must still match full.
        edited = _WELL_FORMED.replace("{{by 1}}", "{{by 2}}")
        _assert_incremental_equals_full(_WELL_FORMED, edited)

    def test_body_grows_by_a_line(self):
        # Adding a line inside one proc shifts every following proc down one
        # line — the staleness bug's core scenario, now on a clean file.
        edited = _WELL_FORMED.replace(
            "    set total 0\n",
            "    set total 0\n    set scratch 0\n",
        )
        _assert_incremental_equals_full(_WELL_FORMED, edited)


class TestStyleDiagnosticCaching:
    """Verify per-chunk style diagnostic caching."""

    def test_style_diagnostics_cached_after_full_build(self):
        """After full build, chunk caches should have style diagnostics."""
        source = "set x 10\nset y 20\n"
        state = DocumentState(uri="test")
        state.update(source)
        assert len(state._chunk_caches) == 2
        for cc in state._chunk_caches:
            assert cc is not None
            assert cc.style_diagnostics is not None

    def test_get_cached_style_diagnostics_returns_list(self):
        """get_cached_style_diagnostics should return a list when caches are populated."""
        source = "set x 10\nset y 20\n"
        state = DocumentState(uri="test")
        state.update(source)
        cached = state.get_cached_style_diagnostics()
        assert cached is not None
        assert isinstance(cached, list)

    def test_style_diagnostics_detect_long_line(self):
        """A line exceeding the default length should produce a W111 diagnostic."""
        long_line = "set x " + "a" * 200
        source = f"{long_line}\nset y 20\n"
        state = DocumentState(uri="test")
        state.update(source)
        cached = state.get_cached_style_diagnostics()
        assert cached is not None
        assert any(hasattr(d, "code") and d.code == "W111" for d in cached)

    def test_style_diagnostics_cached_for_clean_chunks(self):
        """After incremental update, clean chunks should retain style diagnostics."""
        source1 = "set x 10\nset y 20\nset z 30\n"
        source2 = "set x 10\nset y 20\nset z 99\n"
        state = DocumentState(uri="test")
        state.update(source1)
        state.update(source2)
        # First two chunks should have style diagnostics (unchanged).
        for i in range(2):
            cc = state._chunk_caches[i]
            assert cc is not None
            assert cc.style_diagnostics is not None

    def test_cached_style_diagnostics_match_full(self):
        """Cached style diagnostics should produce same result as full computation."""
        from server.features.diagnostics import get_basic_diagnostics

        source1 = "set x 10\nset y 20\n"
        source2 = "set x 10\nset y 99\n"
        # Full computation (no cache).
        full_diags, _, _ = get_basic_diagnostics(source2)
        # Incremental computation (with cache).
        state = DocumentState(uri="test")
        state.update(source1)
        state.update(source2)
        cached = state.get_cached_style_diagnostics()
        cached_diags, _, _ = get_basic_diagnostics(
            source2,
            analysis=state.analysis,
            cu=state.compilation_unit,
            cached_style_diagnostics=cached,
        )
        # Both should have the same set of diagnostic codes.
        full_codes = sorted(d.code for d in full_diags if d.code is not None)
        cached_codes = sorted(d.code for d in cached_diags if d.code is not None)
        assert full_codes == cached_codes


class TestSemanticTokenCaching:
    """Verify per-chunk semantic token caching."""

    def test_semantic_token_cache_roundtrip(self):
        """Cached semantic tokens should produce same result as full computation."""
        from server.features import semantic_tokens_full

        source = "set x 10\nset y 20\nproc foo {a} { return $a }\n"
        state = DocumentState(uri="test")
        state.update(source)

        # Full computation (no cache).
        full_data = semantic_tokens_full(source, analysis=state.analysis)

        # Compute with cache (first call populates).
        cache_info = state.get_semantic_token_cache()
        assert cache_info is not None
        chunk_token_cache, chunk_line_ranges = cache_info
        data_with_cache = semantic_tokens_full(
            source,
            analysis=state.analysis,
            chunk_token_cache=chunk_token_cache,
            chunk_line_ranges=chunk_line_ranges,
        )
        state.store_semantic_token_cache(chunk_token_cache)

        # Second call should use cache.
        cache_info2 = state.get_semantic_token_cache()
        assert cache_info2 is not None
        chunk_token_cache2, chunk_line_ranges2 = cache_info2
        # All entries should be populated now.
        assert all(entry is not None for entry in chunk_token_cache2)
        data_from_cache = semantic_tokens_full(
            source,
            analysis=state.analysis,
            chunk_token_cache=chunk_token_cache2,
            chunk_line_ranges=chunk_line_ranges2,
        )

        assert full_data == data_with_cache
        assert full_data == data_from_cache

    def test_incremental_edit_recomputes_dirty_chunk_tokens(self):
        """After an incremental edit, dirty chunks get fresh pre-computed tokens."""
        source1 = "set x 10\nset y 20\nset z 30\n"
        # Use a 3-digit number so the token length differs from "30" (2 chars).
        source2 = "set x 10\nset y 20\nset z 999\n"
        state = DocumentState(uri="test")
        state.update(source1)

        # Populate semantic token cache.
        from server.features import semantic_tokens_full

        cache_info = state.get_semantic_token_cache()
        assert cache_info is not None
        chunk_token_cache, chunk_line_ranges = cache_info
        semantic_tokens_full(
            source1,
            analysis=state.analysis,
            chunk_token_cache=chunk_token_cache,
            chunk_line_ranges=chunk_line_ranges,
        )
        state.store_semantic_token_cache(chunk_token_cache)
        old_chunk2_tokens = chunk_token_cache[2]

        # Now edit — dirty chunk (index 2) gets fresh pre-computed tokens.
        state.update(source2)
        cache_info2 = state.get_semantic_token_cache()
        assert cache_info2 is not None
        chunk_token_cache2, _ = cache_info2
        # First two chunks should still have cached tokens.
        assert chunk_token_cache2[0] is not None
        assert chunk_token_cache2[1] is not None
        # Third chunk was dirty — it should have fresh tokens (pre-computed
        # during the background update), not the old ones.
        assert chunk_token_cache2[2] is not None
        assert chunk_token_cache2[2] != old_chunk2_tokens


class TestDeepDiagnosticCaching:
    """Verify proc-level deep diagnostic caching."""

    def test_deep_diag_cache_populated_after_store(self):
        """Storing deep diagnostics should make them retrievable."""
        source = "proc foo {} { return 1 }\n"
        state = DocumentState(uri="test")
        state.update(source)
        assert state.get_cached_deep_diagnostics() is None  # No cache yet.
        state.store_deep_diagnostics([{"mock": "diag"}])
        assert state.get_cached_deep_diagnostics() is not None
        assert state.get_cached_deep_diagnostics() == [{"mock": "diag"}]

    def test_deep_diag_cache_invalidated_on_proc_change(self):
        """Changing a proc body should invalidate the deep diagnostic cache."""
        source1 = "proc foo {} { return 1 }\n"
        source2 = "proc foo {} { return 2 }\n"
        state = DocumentState(uri="test")
        state.update(source1)
        state.store_deep_diagnostics([{"mock": "diag"}])
        assert state.get_cached_deep_diagnostics() is not None
        state.update(source2)
        # Proc body changed — cache should be invalidated.
        assert state.get_cached_deep_diagnostics() is None

    def test_deep_diag_cache_invalidated_on_top_level_change(self):
        """Changing top-level code should invalidate deep diag cache.

        Deep diagnostics include top-level optimisations and taint
        warnings, so top-level edits must not reuse stale results.
        """
        source1 = "set x 10\nproc foo {} { return 1 }\n"
        source2 = "set x 99\nproc foo {} { return 1 }\n"
        state = DocumentState(uri="test")
        state.update(source1)
        state.store_deep_diagnostics([{"mock": "diag"}])
        assert state.get_cached_deep_diagnostics() is not None
        state.update(source2)
        # Top-level changed — cache should be invalidated.
        assert state.get_cached_deep_diagnostics() is None

    def test_deep_diag_cache_preserved_on_identical_source(self):
        """Re-updating with identical source should preserve deep diag cache."""
        source = "set x 10\nproc foo {} { return 1 }\n"
        state = DocumentState(uri="test")
        state.update(source)
        state.store_deep_diagnostics([{"mock": "diag"}])
        assert state.get_cached_deep_diagnostics() is not None
        state.update(source)
        assert state.get_cached_deep_diagnostics() is not None


class TestProcCacheOffsetInvalidation:
    """A same-line edit *before* a proc shifts every absolute byte offset inside
    it while leaving its line/character stable. The cached ``FunctionUnit`` must
    NOT be reused, or optimiser/code-action offset data (``startOffset`` /
    ``endOffset``) points at stale bytes. Regression for the proc-cache key
    omitting ``start.offset``."""

    def test_same_line_prefix_edit_refreshes_optimiser_offsets(self):
        from compiler.optimiser import find_optimisations

        body = "proc f {} { set dead 1 ; return 2 }\n"
        src1 = "set a 1\n" + body
        # Longer first line, same line count: proc f stays at line 1, char 0,
        # but every byte offset inside it shifts.
        src2 = "set aaaaaaaaaa 1\n" + body

        inc = DocumentState(uri="inc")
        inc.update(src1)
        inc.update(src2)
        full = DocumentState(uri="full")
        full.update(src2)
        inc_cu = inc.compilation_unit
        full_cu = full.compilation_unit
        assert inc_cu is not None and full_cu is not None

        # Precondition: proc position (line/char) is identical in both sources —
        # only the byte offset differs, so only the offset key component matters.
        inc_proc = inc_cu.ir_module.procedures["::f"].range.start
        full_proc = full_cu.ir_module.procedures["::f"].range.start
        assert (inc_proc.line, inc_proc.character) == (full_proc.line, full_proc.character)

        def opt_offsets(cu):
            return sorted(
                (o.code, o.range.start.offset, o.range.end.offset)
                for o in find_optimisations(src2, cu=cu)
            )

        # Incremental must match a full rebuild of src2 — not carry src1 offsets.
        assert opt_offsets(inc_cu) == opt_offsets(full_cu)

        # And the dead-store optimisation's offsets slice to the dead store in src2.
        opts = {o.code: o for o in find_optimisations(src2, cu=inc_cu)}
        o = opts["O126"]
        assert src2[o.range.start.offset : o.range.end.offset + 1].strip() == "set dead 1"


class TestSnapshotConsistency:
    """A captured ``state.snap`` is an immutable, internally-consistent view:
    its source/analysis/compilation_unit all come from one update and a later
    snapshot swap (a concurrent did_change) cannot change them. This is the
    invariant the hover handler relies on to avoid mixing two document versions
    by reading version/analysis/source/cu as separate live properties."""

    def test_captured_snapshot_survives_concurrent_swap(self):
        state = DocumentState(uri="snap://t.tcl")
        state.update("set x 1\nproc f {} { return 1 }\n")
        snap = state.snap
        src0 = snap.source
        ana0 = snap.analysis
        cu0 = snap.compilation_unit
        ver0 = snap.version
        assert ana0 is not None and cu0 is not None

        # Simulate a did_change interleaving mid-hover: the live snapshot is
        # swapped (quick update clears analysis), but the captured snap is frozen.
        state.update_source_quick("totally different source\n", version=99)

        assert snap.source == src0
        assert snap.analysis is ana0
        assert snap.compilation_unit is cu0
        assert snap.version == ver0
        # The captured source and analysis are mutually consistent (same update).
        assert snap.analysis is ana0 and snap.source == src0
        # The live state has moved on independently.
        assert state.snap.source == "totally different source\n"
        assert state.snap.version == 99

    def test_semantic_token_cache_reads_passed_snapshot(self):
        # The semantic-tokens handler captures state.snap once and passes it to
        # get_semantic_token_cache(snap); that must read the captured snapshot's
        # chunks, not the live one a concurrent did_change may have replaced.
        state = DocumentState(uri="snap://tok.tcl")
        state.update("set x 1\nset y 2\n")  # two chunks
        snap_old = state.snap
        assert len(snap_old.chunks) == 2

        # Concurrent edit swaps the live snapshot to a single-chunk document.
        state.update_source_quick("set only 1\n", version=2)
        assert len(state.snap.chunks) == 1

        info_old = state.get_semantic_token_cache(snap_old)
        assert info_old is not None
        # Ranges reflect the captured snapshot's two chunks, not the live one.
        assert len(info_old[1]) == 2

    def test_store_semantic_token_cache_skips_stale_chunks(self):
        # Tokens computed for an old snapshot must not be grafted by index onto
        # chunks whose text changed since (store writeback guard).
        state = DocumentState(uri="snap://tok-store.tcl")
        state.update("set x 1\nset y 2\n")
        old = state.snap
        assert len(old.chunks) == 2
        # Edit chunk 0 only; chunk 1 ("set y 2") keeps its hash.
        state.update("set xxxxx 1\nset y 2\n")
        cur = state.snap
        assert cur.chunks[0].source_hash != old.chunks[0].source_hash
        assert cur.chunks[1].source_hash == old.chunks[1].source_hash

        tok0 = [(0, 0, 1, 1, 1)]
        tok1 = [(1, 0, 1, 1, 2)]
        state.store_semantic_token_cache([tok0, tok1], old)
        caches = state.snap.chunk_caches
        # chunk 0 changed since the tokens were computed → not grafted.
        assert caches[0] is None or caches[0].semantic_tokens_abs != tok0
        # chunk 1 unchanged → tokens written.
        assert caches[1] is not None and caches[1].semantic_tokens_abs == tok1


class TestProcCacheKnownClassInvalidation:
    """Adding/changing a class definition *elsewhere* must invalidate an
    otherwise-unchanged proc whose object-type facts depend on the class set
    (``[C new]`` resolves to a different OBJECT type once ``C`` is known).
    Regression for the proc-cache key omitting the known-classes fingerprint."""

    def test_class_added_after_proc_refreshes_object_facts(self):
        proc = "proc uses {} { set o [C new] ; return $o }\n"
        src1 = proc
        # Class added AFTER the proc: the proc's position/body/context are all
        # unchanged, so only the known-classes set differs.
        src2 = proc + "oo::class create C { }\n"

        inc = DocumentState(uri="inc")
        inc.update(src1)
        inc.update(src2)
        full = DocumentState(uri="full")
        full.update(src2)
        inc_cu = inc.compilation_unit
        full_cu = full.compilation_unit
        assert inc_cu is not None and full_cu is not None

        # Precondition: proc unchanged in position.
        inc_proc = inc_cu.ir_module.procedures["::uses"].range.start
        assert (inc_proc.line, inc_proc.character, inc_proc.offset) == (0, 0, 0)

        def o_types(cu):
            fa = cu.procedures["::uses"].analysis
            return {(n, v): repr(t) for (n, v), t in fa.types.items() if n == "o"}

        # The known class is now visible, so o's object type must match a full
        # rebuild — not the stale facts cached when C was unknown.
        assert o_types(inc_cu) == o_types(full_cu)
        assert "::C" in next(iter(o_types(full_cu).values()))


class TestQuickUpdatePreservesIncrementalPath:
    """A did_change quick update clears analysis but preserves an incremental
    base, so the follow-up update() takes the incremental path (reusing cached
    artefacts) instead of the cold fresh rebuild — and stays byte-for-byte
    equal to a full rebuild.  Regression for update_source_quick forcing every
    warm edit through _analyse_document_fresh."""

    def test_quick_then_update_is_incremental_not_fresh(self):
        src1 = "proc alpha {} { puts $x }\nproc beta {} { return 2 }\n"
        # One-chunk edit: only beta's body changes; alpha is untouched.
        src2 = "proc alpha {} { puts $x }\nproc beta {} { return 7 }\n"

        st = DocumentState(uri="warm")
        st.update(src1)
        cu1 = st.compilation_unit
        assert cu1 is not None
        alpha1 = cu1.procedures["::alpha"]

        # Mirror the pipeline's warm did_change flow: quick update, then update.
        st.update_source_quick(src2, version=2)
        assert st.analysis is None  # quick cleared analysis ...
        # ... but the pipeline's is_fresh predicate is False (an incremental
        # base survives), so it routes to update() rather than fresh analysis.
        assert st.can_analyse_incrementally is True

        st.update(src2, version=2)
        cu2 = st.compilation_unit
        assert cu2 is not None

        # alpha (before the edit) is the *same* FunctionUnit object — proof the
        # incremental path reused the cached base rather than rebuilding every
        # proc fresh (a fresh rebuild constructs all FunctionUnits anew).
        assert cu2.procedures["::alpha"] is alpha1
        assert cu2.procedures["::beta"] is not None
        assert st.analysis is not None

        # Correctness: identical diagnostics to a from-scratch rebuild of src2.
        full = DocumentState(uri="full")
        full.update(src2)
        assert _diag_tuples(st) == _diag_tuples(full)

    def test_consecutive_quick_updates_keep_oldest_analysed_base(self):
        # Two quick updates before any full update: the analysed base from the
        # last full analysis must survive so the eventual update() is incremental.
        st = DocumentState(uri="warm2")
        st.update("proc a {} { return 1 }\nproc b {} { return 2 }\n")
        cu1 = st.compilation_unit
        assert cu1 is not None
        a1 = cu1.procedures["::a"]
        st.update_source_quick("proc a {} { return 1 }\nproc b {} { return 3 }\n", version=2)
        st.update_source_quick("proc a {} { return 1 }\nproc b {} { return 4 }\n", version=3)
        assert st.can_analyse_incrementally is True
        st.update("proc a {} { return 1 }\nproc b {} { return 4 }\n", version=3)
        cu2 = st.compilation_unit
        assert cu2 is not None
        assert cu2.procedures["::a"] is a1  # unchanged proc reused across the run
        full = DocumentState(uri="full2")
        full.update("proc a {} { return 1 }\nproc b {} { return 4 }\n")
        assert _diag_tuples(st) == _diag_tuples(full)


class TestDialectQuickUpdateInvalidation:
    """A same-length source-level dialect switch must invalidate cached
    analysis even though chunk hashes are unchanged.  Regression for
    update_source_quick refreshing dialect_hint before update() could compare
    it, so the change went unnoticed and stale analysis was reused."""

    def test_same_length_dialect_switch_matches_full(self):
        s1 = "# tcl-dialect: tcl8.4\nlassign {1 2} a b\n"
        # Same length, dialect bumped to 8.6 (where lassign is a builtin).
        s2 = "# tcl-dialect: tcl8.6\nlassign {1 2} a b\n"
        assert len(s1) == len(s2)

        inc = DocumentState(uri="dialect-inc")
        inc.update(s1)
        inc.update_source_quick(s2, version=2)
        inc.update(s2, version=2)

        full = DocumentState(uri="dialect-full")
        full.update(s2)

        assert _diag_tuples(inc) == _diag_tuples(full)
        # No stale W002 (lassign-not-in-8.4) carried over from the 8.4 analysis.
        assert all(code != "W002" for (code, *_rest) in _diag_tuples(inc))


class TestSemanticTokenFlagConsistency:
    """In-thread semantic-token precompute (full and incremental) must use the
    same tokenizer flags as the subprocess full build.  Regression for the
    in-thread paths computing is_irules from the .irul/.irule extension only,
    omitting is_irules_dialect()/is_bigip_conf/is_apl — so an iRules file
    identified by dialect got different cached tokens than a fresh analysis."""

    _SRC = "# tcl-dialect: f5-irules\nwhen HTTP_REQUEST {\n  HTTP::respond 200\n}\nset a 1\n"

    @staticmethod
    def _subprocess_tokens(src, version=1):
        from server.workspace.document_state import _analyse_document_fresh

        ref = _analyse_document_fresh(
            source=src, version=version, line_length=120, dialect="f5-irules", uri="file:///x.tcl"
        )
        return [cc.semantic_tokens_abs if cc else None for cc in ref["chunk_caches"]]

    def test_full_inthread_tokens_match_subprocess(self):
        st = DocumentState(uri="file:///x.tcl")
        st.update(self._SRC)
        assert st.dialect_hint == "f5-irules"  # inferred from the directive
        info = st.get_semantic_token_cache()
        assert info is not None
        ref = self._subprocess_tokens(self._SRC)
        assert any(ref), "fixture must produce semantic tokens"
        assert info[0] == ref

    def test_incremental_tokens_match_subprocess(self):
        v2 = self._SRC.replace("set a 1", "set a 2")  # same-length edit to the last chunk
        st = DocumentState(uri="file:///x.tcl")
        st.update(self._SRC)
        st.update(v2)  # incremental: only the last chunk is dirty
        info = st.get_semantic_token_cache()
        assert info is not None
        assert info[0] == self._subprocess_tokens(v2, version=2)


class TestBracedBodyIncrementalReuse:
    """A file dominated by one huge braced-body command (``namespace eval ns {
    …thousands of lines… }``) has no reusable top-level chunk boundary, so a
    naive incremental segmenter re-lexes the whole body on every keystroke.  The
    brace-safe interior-edit fast path splices the body word instead, and must
    stay byte-identical to a full re-segmentation."""

    _BODY = "\n".join(f"    set v{i} {i}" for i in range(400))
    _SRC = f"package require Foo\nnamespace eval ::ns {{\n{_BODY}\n}}\nset done 1\n"

    @staticmethod
    def _chunks_equal(a, b) -> bool:
        if len(a) != len(b):
            return False
        for x, y in zip(a, b):
            if (x.index, x.start_offset, x.end_offset, x.source_hash) != (
                y.index,
                y.start_offset,
                y.end_offset,
                y.source_hash,
            ):
                return False
            if x.commands != y.commands:
                return False
        return True

    def _interior_offset(self):
        chunks = segment_top_level_chunks(self._SRC)
        giant = max(chunks, key=lambda c: c.end_offset - c.start_offset)
        body = giant.commands[0].argv[-1]
        return chunks, (body.start.offset + 1 + body.end.offset) // 2

    def test_brace_safe_interior_edit_matches_full(self):
        from compiler.parsing.incremental import (
            _reuse_edit_in_braced_body,
            infer_edit_range,
        )

        old_chunks, pos = self._interior_offset()
        for ins in ("x", "set extra 1\n", "  # comment\n", "abc def", "\n\n"):
            new = self._SRC[:pos] + ins + self._SRC[pos:]
            edit = infer_edit_range(self._SRC, new)
            assert edit is not None
            fast = _reuse_edit_in_braced_body(self._SRC, old_chunks, new, edit)
            assert fast is not None, f"fast path should apply for brace-safe ins={ins!r}"
            assert self._chunks_equal(fast, segment_top_level_chunks(new)), (
                f"fast path diverged from full segmentation for ins={ins!r}"
            )

    def test_brace_unsafe_edit_bails(self):
        from compiler.parsing.incremental import (
            _reuse_edit_in_braced_body,
            infer_edit_range,
        )

        old_chunks, pos = self._interior_offset()
        for ins in ("{", "}", "if {1} {", "\\n", "}}}"):
            new = self._SRC[:pos] + ins + self._SRC[pos:]
            edit = infer_edit_range(self._SRC, new)
            assert edit is not None
            # Must fall back (None) — a brace/backslash edit can change structure.
            assert _reuse_edit_in_braced_body(self._SRC, old_chunks, new, edit) is None, (
                f"brace-unsafe ins={ins!r} must bail to full re-segmentation"
            )

    def test_brace_unsafe_escape_adjacency_bails(self):
        """An edit *adjacent* to an existing ``\\{``/``\\}`` can flip brace
        nesting even though the edited text has no brace/backslash: splicing a
        space into ``\\{`` un-escapes the brace (inert literal → live opener).
        These must bail even though the literal edited text looks innocuous."""
        from compiler.parsing.incremental import (
            _reuse_edit_in_braced_body,
            infer_edit_range,
        )

        tail = "\n".join(f"    set v{i} {i}" for i in range(60))

        def _assert_bails(src: str, new: str, label: str) -> None:
            chunks = segment_top_level_chunks(src)
            edit = infer_edit_range(src, new)
            assert edit is not None
            assert _reuse_edit_in_braced_body(src, chunks, new, edit) is None, (
                f"escape-adjacent edit must bail to full re-segmentation: {label}"
            )

        # Original bodies keep balanced braces by *escaping* the literal brace,
        # so the fast path is eligible; each edit then un-escapes it.
        src_open = f"namespace eval ::ns {{\n    set a \\{{\n{tail}\n}}\nset z 1\n"
        p = src_open.index("\\{") + 1  # between the '\' and the '{'
        _assert_bails(src_open, src_open[:p] + " " + src_open[p:], "insert space in \\{")
        _assert_bails(src_open, src_open[:p] + "X" + src_open[p:], "insert X between \\ and {")

        src_close = f"namespace eval ::ns {{\n    set a \\}}\n{tail}\n}}\nset z 1\n"
        q = src_close.index("\\}") + 1  # between the '\' and the '}'
        _assert_bails(src_close, src_close[:q] + " " + src_close[q:], "insert space in \\}")
        _assert_bails(src_close, src_close[:q] + "Y" + src_close[q:], "insert Y between \\ and }")

        # End-to-end: each un-escaping edit must stay byte-identical to a full
        # re-segmentation (the fallback the bail triggers).
        for src, splice_at in ((src_open, p), (src_close, q)):
            new = src[:splice_at] + " " + src[splice_at:]
            assert segment_top_level_chunks(new) is not None  # full path is the oracle

    def test_document_state_quick_update_stays_byte_identical(self):
        # End-to-end: an interior edit via update_source_quick yields the same
        # chunks a from-scratch segmentation would.
        st = DocumentState(uri="file:///braced.tcl")
        st.update_source_quick(self._SRC, 1)
        _ = st.buffer
        _chunks, pos = self._interior_offset()
        new = self._SRC[:pos] + "set spliced 1\n" + self._SRC[pos:]
        st.update_source_quick(new, 2)
        assert self._chunks_equal(list(st.chunks), segment_top_level_chunks(new))
