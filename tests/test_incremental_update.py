"""Tests for incremental document state updates.

Verifies that the incremental path (chunk-level IR caching +
analyser snapshot/restore) produces identical results to a full
rebuild.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import Analyser
from core.compiler.lowering import lower_commands_to_ir, lower_to_ir
from core.parsing.command_segmenter import (
    segment_commands,
    segment_top_level_chunks,
)
from core.parsing.token_positions import shift_position, shift_range, shift_token
from core.parsing.tokens import SourcePosition, Token, TokenType
from lsp.workspace.document_state import DocumentState


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
        from core.analysis.semantic_model import Range

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

    def test_irules_like_source(self):
        """Test with iRules-like source (when blocks)."""
        source1 = "set x 10\nset y 20\n"
        source2 = "set x 10\nset y 30\n"
        state = DocumentState(uri="test")
        state.update(source1)
        state.update(source2)
        assert state.analysis is not None


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
        from lsp.features.diagnostics import get_basic_diagnostics

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
        from lsp.features.semantic_tokens import semantic_tokens_full

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

    def test_incremental_edit_invalidates_dirty_chunk_tokens(self):
        """After an incremental edit, dirty chunks should have no cached tokens."""
        source1 = "set x 10\nset y 20\nset z 30\n"
        source2 = "set x 10\nset y 20\nset z 99\n"
        state = DocumentState(uri="test")
        state.update(source1)

        # Populate semantic token cache.
        from lsp.features.semantic_tokens import semantic_tokens_full

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

        # Now edit — dirty chunk (index 2) should lose its token cache.
        state.update(source2)
        cache_info2 = state.get_semantic_token_cache()
        assert cache_info2 is not None
        chunk_token_cache2, _ = cache_info2
        # First two chunks should still have cached tokens.
        assert chunk_token_cache2[0] is not None
        assert chunk_token_cache2[1] is not None
        # Third chunk was dirty — its cache should be None.
        assert chunk_token_cache2[2] is None


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
