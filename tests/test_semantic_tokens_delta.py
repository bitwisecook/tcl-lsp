"""Tests for semantic tokens delta encoding and edit handling.

Covers:
- Delta diff computation (compute_semantic_tokens_edits)
- Various edit patterns: insert, delete, replace at start/middle/end
- Multi-cursor edits (multiple simultaneous changes)
- update_source_quick for fast source updates
- arg_indices_for_roles batching
- Semantic token correctness across incremental edits
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.registry.runtime import ArgRole, arg_indices_for_role, arg_indices_for_roles
from server.features import compute_semantic_tokens_edits, semantic_tokens_full
from server.workspace.document_state import DocumentState


class TestComputeSemanticTokensEdits:
    """Verify the delta diff algorithm for semantic token arrays."""

    def test_identical_arrays(self):
        old = [0, 0, 3, 1, 0, 1, 0, 4, 2, 0]
        new = list(old)
        edits = compute_semantic_tokens_edits(old, new)
        assert edits == []

    def test_empty_to_nonempty(self):
        old: list[int] = []
        new = [0, 0, 3, 1, 0]
        edits = compute_semantic_tokens_edits(old, new)
        assert len(edits) == 1
        start, delete_count, insert_data = edits[0]
        assert start == 0
        assert delete_count == 0
        assert insert_data == new

    def test_nonempty_to_empty(self):
        old = [0, 0, 3, 1, 0]
        new: list[int] = []
        edits = compute_semantic_tokens_edits(old, new)
        assert len(edits) == 1
        start, delete_count, insert_data = edits[0]
        assert start == 0
        assert delete_count == 5
        assert insert_data == []

    def test_change_single_token(self):
        # Two tokens, change the second one's type (index 3 of second group).
        old = [0, 0, 3, 1, 0, 0, 5, 4, 2, 0]
        new = [0, 0, 3, 1, 0, 0, 5, 4, 7, 0]  # type 2 -> 7
        edits = compute_semantic_tokens_edits(old, new)
        assert len(edits) == 1
        start, delete_count, insert_data = edits[0]
        # The first token (5 ints) is shared prefix.
        assert start == 5
        assert delete_count == 5
        assert insert_data == [0, 5, 4, 7, 0]

    def test_insert_token_at_end(self):
        old = [0, 0, 3, 1, 0]
        new = [0, 0, 3, 1, 0, 1, 0, 5, 2, 0]  # added a token
        edits = compute_semantic_tokens_edits(old, new)
        assert len(edits) == 1
        start, delete_count, insert_data = edits[0]
        assert start == 5
        assert delete_count == 0
        assert insert_data == [1, 0, 5, 2, 0]

    def test_delete_token_at_end(self):
        old = [0, 0, 3, 1, 0, 1, 0, 5, 2, 0]
        new = [0, 0, 3, 1, 0]
        edits = compute_semantic_tokens_edits(old, new)
        assert len(edits) == 1
        start, delete_count, insert_data = edits[0]
        assert start == 5
        assert delete_count == 5
        assert insert_data == []

    def test_change_first_token(self):
        old = [0, 0, 3, 1, 0, 1, 0, 5, 2, 0]
        new = [0, 0, 6, 1, 0, 1, 0, 5, 2, 0]  # length 3 -> 6
        edits = compute_semantic_tokens_edits(old, new)
        assert len(edits) == 1
        start, delete_count, insert_data = edits[0]
        assert start == 0
        assert delete_count == 5
        assert insert_data == [0, 0, 6, 1, 0]

    def test_change_in_middle(self):
        # Three tokens, change the middle one.
        old = [0, 0, 3, 1, 0, 0, 5, 4, 2, 0, 1, 0, 2, 3, 0]
        new = [0, 0, 3, 1, 0, 0, 5, 9, 2, 0, 1, 0, 2, 3, 0]  # type 4->9
        edits = compute_semantic_tokens_edits(old, new)
        assert len(edits) == 1
        start, delete_count, insert_data = edits[0]
        assert start == 5
        assert delete_count == 5
        assert insert_data == [0, 5, 9, 2, 0]

    def test_applying_edit_produces_new_data(self):
        """Applying the edit to old should produce new."""
        old = [0, 0, 3, 1, 0, 0, 5, 4, 2, 0, 1, 0, 2, 3, 0]
        new = [0, 0, 3, 1, 0, 0, 5, 9, 8, 0, 1, 0, 2, 3, 0]
        edits = compute_semantic_tokens_edits(old, new)
        # Apply edits to old.
        result = list(old)
        for start, delete_count, insert_data in reversed(edits):
            result[start : start + delete_count] = insert_data
        assert result == new

    def test_complete_replacement(self):
        """When all tokens change, the edit covers everything."""
        old = [0, 0, 3, 1, 0]
        new = [1, 2, 3, 4, 5]
        edits = compute_semantic_tokens_edits(old, new)
        assert len(edits) == 1
        result = list(old)
        for start, delete_count, insert_data in reversed(edits):
            result[start : start + delete_count] = insert_data
        assert result == new

    def test_large_array_common_prefix_suffix(self):
        """Large arrays that differ only in the middle."""
        prefix = list(range(0, 50))  # 10 tokens
        suffix = list(range(100, 150))  # 10 tokens
        middle_old = [0, 0, 5, 1, 0]
        middle_new = [0, 0, 5, 2, 0]
        old = prefix + middle_old + suffix
        new = prefix + middle_new + suffix
        edits = compute_semantic_tokens_edits(old, new)
        assert len(edits) == 1
        start, delete_count, insert_data = edits[0]
        assert start == 50
        assert delete_count == 5
        assert insert_data == middle_new


class TestSemanticTokensDeltaEndToEnd:
    """End-to-end delta tests using actual Tcl source."""

    def test_single_line_edit_produces_delta(self):
        """Editing a variable value should produce a small delta."""
        source1 = "set x 10\nset y 20\nset z 30\n"
        source2 = "set x 10\nset y 999\nset z 30\n"
        tokens1 = semantic_tokens_full(source1)
        tokens2 = semantic_tokens_full(source2)
        edits = compute_semantic_tokens_edits(tokens1, tokens2)
        # Should have changes but not replace everything.
        if edits:
            total_insert = sum(len(ins) for _, _, ins in edits)
            assert total_insert < len(tokens2)

    def test_append_command_produces_delta(self):
        """Adding a command at the end."""
        source1 = "set x 10\n"
        source2 = "set x 10\nset y 20\n"
        tokens1 = semantic_tokens_full(source1)
        tokens2 = semantic_tokens_full(source2)
        edits = compute_semantic_tokens_edits(tokens1, tokens2)
        assert len(edits) >= 1

    def test_delete_command_produces_delta(self):
        source1 = "set x 10\nset y 20\nset z 30\n"
        source2 = "set x 10\nset z 30\n"
        tokens1 = semantic_tokens_full(source1)
        tokens2 = semantic_tokens_full(source2)
        edits = compute_semantic_tokens_edits(tokens1, tokens2)
        result = list(tokens1)
        for start, delete_count, insert_data in reversed(edits):
            result[start : start + delete_count] = insert_data
        assert result == tokens2

    def test_proc_body_edit(self):
        """Editing inside a proc body."""
        source1 = "proc foo {a} {\n    return $a\n}\n"
        source2 = "proc foo {a} {\n    set b $a\n    return $b\n}\n"
        tokens1 = semantic_tokens_full(source1)
        tokens2 = semantic_tokens_full(source2)
        edits = compute_semantic_tokens_edits(tokens1, tokens2)
        result = list(tokens1)
        for start, delete_count, insert_data in reversed(edits):
            result[start : start + delete_count] = insert_data
        assert result == tokens2

    def test_no_change_no_delta(self):
        source = "set x 10\nset y 20\n"
        tokens = semantic_tokens_full(source)
        edits = compute_semantic_tokens_edits(tokens, tokens)
        assert edits == []


class TestUpdateSourceQuick:
    """Test the fast source-only update path."""

    def test_quick_update_sets_source(self):
        state = DocumentState(uri="test")
        state.update("set x 10\n")
        assert state.analysis is not None
        changed = state.update_source_quick("set x 20\n")
        assert changed is True
        assert state.source == "set x 20\n"
        assert state.analysis is None  # cleared
        # Dirty chunks get None entries; clean prefix caches are carried forward.
        assert all(cc is None for cc in state._chunk_caches)

    def test_quick_update_returns_false_when_unchanged(self):
        state = DocumentState(uri="test")
        state.update("set x 10\n")
        changed = state.update_source_quick("set x 10\n")
        assert changed is False
        assert state.analysis is not None  # preserved

    def test_quick_update_segments_chunks(self):
        state = DocumentState(uri="test")
        state.update("set x 10\n")
        state.update_source_quick("set x 10\nset y 20\n")
        assert len(state.chunks) == 2

    def test_quick_update_clears_compilation_unit(self):
        state = DocumentState(uri="test")
        state.update("proc foo {} { return 1 }\n")
        assert state.compilation_unit is not None
        state.update_source_quick("proc foo {} { return 2 }\n")
        assert state.compilation_unit is None

    def test_full_update_after_quick(self):
        """Full update after quick should produce correct analysis."""
        source1 = "set x 10\n"
        source2 = "set x 20\nset y 30\n"
        state = DocumentState(uri="test")
        state.update(source1)
        state.update_source_quick(source2)
        assert state.analysis is None
        state.update(source2)
        assert state.analysis is not None
        assert len(state.chunks) == 2

    def test_semantic_tokens_work_after_quick_update(self):
        """Semantic tokens should work even without analysis."""
        state = DocumentState(uri="test")
        state.update("set x 10\n")
        state.update_source_quick("set x 20\nset y 30\n")
        # Tokens should work without analysis.
        tokens = semantic_tokens_full("set x 20\nset y 30\n", analysis=None)
        assert len(tokens) > 0


class TestMultiCursorEdits:
    """Simulate multi-cursor edits: multiple changes applied to a document.

    In multi-cursor editing, the editor sends multiple content changes
    in a single didChange notification.  After the server applies them
    (pygls handles this), we get the final source text.  These tests
    verify that the incremental pipeline handles the resulting source
    correctly.
    """

    def _apply_edits(self, source: str, edits: list[tuple[int, int, str]]) -> str:
        """Apply multiple (offset, old_len, new_text) edits to source.

        Edits are applied in reverse offset order to avoid shifting.
        """
        edits_sorted = sorted(edits, key=lambda e: e[0], reverse=True)
        for offset, old_len, new_text in edits_sorted:
            source = source[:offset] + new_text + source[offset + old_len :]
        return source

    def test_multi_cursor_rename_variable(self):
        """Rename a variable at multiple cursor positions."""
        source = "set foo 10\nputs $foo\nset bar $foo\n"
        # Rename 'foo' to 'baz' at all 3 occurrences.
        # offsets: 'foo' at 4, 17, 30
        edits = [
            (4, 3, "baz"),  # set baz 10
            (17, 3, "baz"),  # puts $baz
            (30, 3, "baz"),  # set bar $baz
        ]
        new_source = self._apply_edits(source, edits)
        assert new_source == "set baz 10\nputs $baz\nset bar $baz\n"

        # Verify incremental pipeline handles this.
        state = DocumentState(uri="test")
        state.update(source)
        state.update(new_source)
        assert state.analysis is not None

        # Verify tokens are correct.
        tokens_full = semantic_tokens_full(new_source, analysis=state.analysis)
        assert len(tokens_full) > 0

    def test_multi_cursor_insert_at_multiple_lines(self):
        """Insert text at the beginning of multiple lines."""
        source = "set a 1\nset b 2\nset c 3\nset d 4\n"
        # Add '#' comment prefix to lines 2 and 4 (0-indexed offsets).
        edits = [
            (8, 0, "# "),  # before 'set b 2'
            (24, 0, "# "),  # before 'set d 4'
        ]
        new_source = self._apply_edits(source, edits)
        assert "# set b 2" in new_source
        assert "# set d 4" in new_source

        state = DocumentState(uri="test")
        state.update(source)
        state.update(new_source)
        assert state.analysis is not None

    def test_multi_cursor_delete_at_multiple_positions(self):
        """Delete characters at multiple cursor positions."""
        source = "set aa 10\nset bb 20\nset cc 30\n"
        # Delete the doubled letters (second char of each variable name).
        edits = [
            (5, 1, ""),  # 'aa' -> 'a'
            (15, 1, ""),  # 'bb' -> 'b'
            (25, 1, ""),  # 'cc' -> 'c'
        ]
        new_source = self._apply_edits(source, edits)
        assert "set a 10" in new_source
        assert "set b 20" in new_source
        assert "set c 30" in new_source

        state = DocumentState(uri="test")
        state.update(source)
        state.update(new_source)
        assert state.analysis is not None

    def test_multi_cursor_replace_different_lengths(self):
        """Replace with different lengths at multiple positions."""
        source = 'set x "hello"\nset y "world"\n'
        # Replace quoted strings with different-length values.
        edits = [
            (6, 7, '"hi"'),  # "hello" -> "hi"
            (20, 7, '"everyone"'),  # "world" -> "everyone"
        ]
        new_source = self._apply_edits(source, edits)
        assert 'set x "hi"' in new_source
        assert 'set y "everyone"' in new_source

        state = DocumentState(uri="test")
        state.update(source)
        state.update(new_source)
        assert state.analysis is not None

    def test_multi_cursor_tokens_match_full_rebuild(self):
        """Multi-cursor edit tokens should match a full rebuild."""
        source1 = "set x 10\nset y 20\nset z 30\n"
        source2 = "set x 99\nset y 88\nset z 77\n"

        # Incremental path.
        inc = DocumentState(uri="inc")
        inc.update(source1)
        inc.update(source2)
        inc_tokens = semantic_tokens_full(source2, analysis=inc.analysis)

        # Full rebuild.
        full = DocumentState(uri="full")
        full.update(source2)
        full_tokens = semantic_tokens_full(source2, analysis=full.analysis)

        assert inc_tokens == full_tokens

    def test_multi_cursor_delta_correctness(self):
        """Delta from multi-cursor edit should reconstruct correctly."""
        source1 = "set a 1\nset b 2\nset c 3\n"
        source2 = "set a 100\nset b 200\nset c 300\n"
        tokens1 = semantic_tokens_full(source1)
        tokens2 = semantic_tokens_full(source2)
        edits = compute_semantic_tokens_edits(tokens1, tokens2)
        result = list(tokens1)
        for start, delete_count, insert_data in reversed(edits):
            result[start : start + delete_count] = insert_data
        assert result == tokens2


class TestArgIndicesForRolesBatch:
    """Test the batched role lookup function."""

    def test_batch_matches_individual(self):
        """Batched lookup should return same results as individual calls."""
        cmd = "proc"
        args = ["myproc", "{a b}", "{ return $a }"]
        roles = (ArgRole.BODY, ArgRole.EXPR, ArgRole.VAR_WRITE, ArgRole.VAR_READ, ArgRole.PATTERN)
        batch_results = arg_indices_for_roles(cmd, args, roles)
        for i, role in enumerate(roles):
            individual = arg_indices_for_role(cmd, args, role)
            assert batch_results[i] == individual, f"Mismatch for {role}"

    def test_batch_if_command(self):
        """if command has BODY and EXPR roles."""
        cmd = "if"
        args = ["$x", "{ puts hi }"]
        roles = (ArgRole.BODY, ArgRole.EXPR, ArgRole.VAR_WRITE)
        batch = arg_indices_for_roles(cmd, args, roles)
        for i, role in enumerate(roles):
            assert batch[i] == arg_indices_for_role(cmd, args, role)

    def test_batch_set_command(self):
        """set command with VAR_WRITE role."""
        cmd = "set"
        args = ["myvar", "myvalue"]
        roles = (ArgRole.BODY, ArgRole.EXPR, ArgRole.VAR_WRITE, ArgRole.VAR_READ, ArgRole.PATTERN)
        batch = arg_indices_for_roles(cmd, args, roles)
        for i, role in enumerate(roles):
            assert batch[i] == arg_indices_for_role(cmd, args, role)

    def test_batch_unknown_command(self):
        """Unknown command should return empty sets."""
        cmd = "nonexistent_command_xyz"
        args = ["a", "b"]
        roles = (ArgRole.BODY, ArgRole.EXPR, ArgRole.VAR_WRITE)
        batch = arg_indices_for_roles(cmd, args, roles)
        assert all(s == set() for s in batch)

    def test_batch_foreach(self):
        """foreach has BODY at last arg."""
        cmd = "foreach"
        args = ["item", "$list", "{ puts $item }"]
        roles = (ArgRole.BODY, ArgRole.EXPR, ArgRole.VAR_WRITE, ArgRole.VAR_READ)
        batch = arg_indices_for_roles(cmd, args, roles)
        for i, role in enumerate(roles):
            assert batch[i] == arg_indices_for_role(cmd, args, role)

    def test_batch_regexp(self):
        """regexp has PATTERN role."""
        cmd = "regexp"
        args = ["\\d+", "$str", "match"]
        roles = (ArgRole.BODY, ArgRole.EXPR, ArgRole.PATTERN)
        batch = arg_indices_for_roles(cmd, args, roles)
        for i, role in enumerate(roles):
            assert batch[i] == arg_indices_for_role(cmd, args, role)


class TestVariousEditPatterns:
    """Test various edit patterns for correctness of incremental pipeline."""

    def _verify_incremental_matches_full(self, source1: str, source2: str):
        """Verify that incremental update matches a full rebuild."""
        inc = DocumentState(uri="inc")
        inc.update(source1)
        inc.update(source2)
        full = DocumentState(uri="full")
        full.update(source2)
        assert inc.analysis is not None
        assert full.analysis is not None
        inc_codes = sorted(d.code for d in inc.analysis.diagnostics)
        full_codes = sorted(d.code for d in full.analysis.diagnostics)
        assert inc_codes == full_codes
        # Compare token output.
        inc_tokens = semantic_tokens_full(source2, analysis=inc.analysis)
        full_tokens = semantic_tokens_full(source2, analysis=full.analysis)
        assert inc_tokens == full_tokens

    def test_insert_at_beginning(self):
        self._verify_incremental_matches_full(
            "set y 20\nset z 30\n",
            "set x 10\nset y 20\nset z 30\n",
        )

    def test_insert_at_end(self):
        self._verify_incremental_matches_full(
            "set x 10\nset y 20\n",
            "set x 10\nset y 20\nset z 30\n",
        )

    def test_insert_in_middle(self):
        self._verify_incremental_matches_full(
            "set x 10\nset z 30\n",
            "set x 10\nset y 20\nset z 30\n",
        )

    def test_delete_first_command(self):
        self._verify_incremental_matches_full(
            "set x 10\nset y 20\nset z 30\n",
            "set y 20\nset z 30\n",
        )

    def test_delete_last_command(self):
        self._verify_incremental_matches_full(
            "set x 10\nset y 20\nset z 30\n",
            "set x 10\nset y 20\n",
        )

    def test_delete_middle_command(self):
        self._verify_incremental_matches_full(
            "set x 10\nset y 20\nset z 30\n",
            "set x 10\nset z 30\n",
        )

    def test_replace_variable_name(self):
        self._verify_incremental_matches_full(
            "set foo 10\nputs $foo\n",
            "set bar 10\nputs $bar\n",
        )

    def test_add_proc(self):
        self._verify_incremental_matches_full(
            "set x 10\n",
            'set x 10\nproc greet {name} {\n    puts "Hello $name"\n}\n',
        )

    def test_remove_proc(self):
        self._verify_incremental_matches_full(
            'proc greet {name} {\n    puts "Hello $name"\n}\nset x 10\n',
            "set x 10\n",
        )

    def test_edit_proc_body(self):
        self._verify_incremental_matches_full(
            "proc add {a b} {\n    expr {$a + $b}\n}\n",
            "proc add {a b} {\n    set result [expr {$a + $b}]\n    return $result\n}\n",
        )

    def test_add_comment(self):
        self._verify_incremental_matches_full(
            "set x 10\nset y 20\n",
            "# This is x\nset x 10\nset y 20\n",
        )

    def test_change_string_to_variable(self):
        self._verify_incremental_matches_full(
            'set x "hello"\n',
            "set x $greeting\n",
        )

    def test_empty_to_content(self):
        self._verify_incremental_matches_full(
            "",
            "set x 10\n",
        )

    def test_content_to_empty(self):
        self._verify_incremental_matches_full(
            "set x 10\n",
            "",
        )

    def test_whitespace_only_change(self):
        """Changing only whitespace between commands."""
        self._verify_incremental_matches_full(
            "set x 10\nset y 20\n",
            "set x 10\n\nset y 20\n",
        )

    def test_nested_command_edit(self):
        self._verify_incremental_matches_full(
            "set x [expr {1 + 2}]\n",
            "set x [expr {3 + 4}]\n",
        )

    def test_if_else_edit(self):
        self._verify_incremental_matches_full(
            "if {$x > 0} {\n    puts positive\n}\n",
            "if {$x > 0} {\n    puts positive\n} else {\n    puts negative\n}\n",
        )

    def test_switch_edit(self):
        self._verify_incremental_matches_full(
            "switch $x {\n    a { puts 1 }\n    b { puts 2 }\n}\n",
            "switch $x {\n    a { puts 1 }\n    b { puts 2 }\n    c { puts 3 }\n}\n",
        )

    def test_foreach_edit(self):
        self._verify_incremental_matches_full(
            "foreach item $list {\n    puts $item\n}\n",
            "foreach item $list {\n    set upper [string toupper $item]\n    puts $upper\n}\n",
        )

    def test_multiple_sequential_edits(self):
        """Apply multiple sequential edits to the same DocumentState."""
        state = DocumentState(uri="test")
        state.update("set x 10\n")
        state.update("set x 10\nset y 20\n")
        state.update("set x 10\nset y 20\nset z 30\n")
        state.update("set x 99\nset y 20\nset z 30\n")
        state.update("set x 99\nset y 20\n")
        assert state.analysis is not None
        assert len(state.chunks) == 2

    def test_rapid_typing_simulation(self):
        """Simulate typing a command character by character."""
        state = DocumentState(uri="test")
        state.update("set x 10\n")
        base = "set x 10\n"
        typing = "puts hello"
        for i in range(1, len(typing) + 1):
            source = base + typing[:i] + "\n"
            state.update(source)
        assert state.analysis is not None
        final_source = base + typing + "\n"
        assert state.source == final_source

    def test_multiline_paste(self):
        """Simulate pasting multiple lines at once."""
        source1 = "set x 10\n"
        paste = "proc compute {a b} {\n    set sum [expr {$a + $b}]\n    return $sum\n}\n"
        source2 = source1 + paste
        self._verify_incremental_matches_full(source1, source2)

    def test_large_file_edit_near_end(self):
        """Edit near the end of a large file — should reuse most chunks."""
        lines = [f"set var{i} {i}" for i in range(50)]
        source1 = "\n".join(lines) + "\n"
        lines[-1] = "set var49 999"
        source2 = "\n".join(lines) + "\n"
        state = DocumentState(uri="test")
        state.update(source1)
        old_caches = list(state._chunk_caches)
        state.update(source2)
        # Most caches should be reused (same objects for clean chunks).
        reused = sum(
            1
            for i, cc in enumerate(state._chunk_caches)
            if i < len(old_caches) and cc is old_caches[i] and cc is not None
        )
        assert reused >= 48  # at least 48 of 50 chunks reused


class TestChunkBoundaryTokenAssignment:
    """Verify that tokens at chunk boundaries are assigned to exactly one chunk.

    The bisect_left partitioning in semantic_tokens_full must not duplicate
    or drop tokens when they fall exactly on a chunk boundary line/col.
    """

    def test_tokens_at_boundary_not_duplicated_or_lost(self):
        """Tokens produced via chunk cache must match tokens without cache."""
        # Two commands on separate lines — each becomes its own chunk.
        source = "set x 1\nset y 2\n"
        state = DocumentState(uri="test://boundary.tcl")
        state.update(source)

        # Tokens without cache (baseline).
        tokens_no_cache = semantic_tokens_full(source, analysis=state.analysis)

        # Tokens with chunk cache.
        cache_info = state.get_semantic_token_cache()
        assert cache_info is not None
        chunk_token_cache, chunk_line_ranges = cache_info
        tokens_with_cache = semantic_tokens_full(
            source,
            analysis=state.analysis,
            chunk_token_cache=chunk_token_cache,
            chunk_line_ranges=chunk_line_ranges,
        )

        assert tokens_no_cache == tokens_with_cache

    def test_semicolon_separated_same_line(self):
        """Chunks sharing a line must partition tokens correctly."""
        source = "set x 1; set y 2\n"
        state = DocumentState(uri="test://semicolon.tcl")
        state.update(source)

        tokens_no_cache = semantic_tokens_full(source, analysis=state.analysis)

        cache_info = state.get_semantic_token_cache()
        if cache_info is not None:
            chunk_token_cache, chunk_line_ranges = cache_info
            tokens_with_cache = semantic_tokens_full(
                source,
                analysis=state.analysis,
                chunk_token_cache=chunk_token_cache,
                chunk_line_ranges=chunk_line_ranges,
            )
            assert tokens_no_cache == tokens_with_cache
