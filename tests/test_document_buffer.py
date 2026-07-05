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

"""Tests for DocumentBuffer and compute_line_starts."""

from __future__ import annotations

from shared.document_buffer import (
    DocumentBuffer,
    compute_line_starts,
)
from shared.tokens import SourcePosition

# compute_line_starts


class TestComputeLineStarts:
    def test_empty(self):
        assert compute_line_starts("") == (0,)

    def test_single_line(self):
        assert compute_line_starts("hello") == (0,)

    def test_two_lines(self):
        assert compute_line_starts("hello\nworld") == (0, 6)

    def test_trailing_newline(self):
        assert compute_line_starts("hello\n") == (0, 6)

    def test_multiple_lines(self):
        assert compute_line_starts("a\nb\nc\nd") == (0, 2, 4, 6)

    def test_empty_lines(self):
        assert compute_line_starts("\n\n\n") == (0, 1, 2, 3)

    def test_crlf(self):
        # \r is treated as a regular character; only \n starts new lines
        assert compute_line_starts("a\r\nb\r\n") == (0, 3, 6)


# DocumentBuffer.from_source


class TestDocumentBufferFromSource:
    def test_basic(self):
        buf = DocumentBuffer.from_source("hello\nworld", version=1)
        assert buf.source == "hello\nworld"
        assert buf.version == 1
        assert buf.line_starts == (0, 6)

    def test_empty(self):
        buf = DocumentBuffer.from_source("")
        assert buf.source == ""
        assert buf.line_starts == (0,)

    def test_lines_cached(self):
        buf = DocumentBuffer.from_source("a\nb\nc")
        lines = buf.lines
        assert lines == ["a", "b", "c"]
        assert buf.lines is lines  # same object


# offset_to_position


class TestOffsetToPosition:
    def test_start_of_file(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        pos = buf.offset_to_position(0)
        assert pos == SourcePosition(line=0, character=0, offset=0)

    def test_middle_of_first_line(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        pos = buf.offset_to_position(3)
        assert pos == SourcePosition(line=0, character=3, offset=3)

    def test_start_of_second_line(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        pos = buf.offset_to_position(6)
        assert pos == SourcePosition(line=1, character=0, offset=6)

    def test_end_of_file(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        pos = buf.offset_to_position(11)
        assert pos == SourcePosition(line=1, character=5, offset=11)

    def test_newline_char(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        pos = buf.offset_to_position(5)
        assert pos == SourcePosition(line=0, character=5, offset=5)

    def test_clamp_negative(self):
        buf = DocumentBuffer.from_source("hello")
        pos = buf.offset_to_position(-1)
        assert pos.offset == 0

    def test_clamp_past_end(self):
        buf = DocumentBuffer.from_source("hello")
        pos = buf.offset_to_position(100)
        assert pos.offset == 5

    def test_empty_source(self):
        buf = DocumentBuffer.from_source("")
        pos = buf.offset_to_position(0)
        assert pos == SourcePosition(line=0, character=0, offset=0)

    def test_multiline(self):
        buf = DocumentBuffer.from_source("line1\nline2\nline3")
        # Start of line3
        pos = buf.offset_to_position(12)
        assert pos == SourcePosition(line=2, character=0, offset=12)
        # Middle of line3
        pos = buf.offset_to_position(15)
        assert pos == SourcePosition(line=2, character=3, offset=15)


# position_to_offset


class TestPositionToOffset:
    def test_start(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        assert buf.position_to_offset(0, 0) == 0

    def test_middle(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        assert buf.position_to_offset(0, 3) == 3

    def test_second_line(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        assert buf.position_to_offset(1, 0) == 6

    def test_clamp_line(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        # Line 5 doesn't exist, clamps to last line
        offset = buf.position_to_offset(5, 0)
        assert offset == 6  # start of last line

    def test_clamp_character(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        # Character 100 on line 0 clamps to end of line
        offset = buf.position_to_offset(0, 100)
        assert offset == 5  # end of "hello" (before \n)

    def test_empty(self):
        buf = DocumentBuffer.from_source("")
        assert buf.position_to_offset(0, 0) == 0

    def test_roundtrip(self):
        source = "proc foo {bar} {\n    set x 1\n    return $x\n}\n"
        buf = DocumentBuffer.from_source(source)
        for offset in range(len(source)):
            pos = buf.offset_to_position(offset)
            back = buf.position_to_offset(pos.line, pos.character)
            assert back == offset, f"roundtrip failed at offset {offset}"


# offset_to_line_col


class TestOffsetToLineCol:
    def test_basic(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        assert buf.offset_to_line_col(0) == (0, 0)
        assert buf.offset_to_line_col(3) == (0, 3)
        assert buf.offset_to_line_col(6) == (1, 0)
        assert buf.offset_to_line_col(8) == (1, 2)


# chunk_line_range


class TestChunkLineRange:
    def test_single_chunk(self):
        source = "set x 1\n"
        buf = DocumentBuffer.from_source(source)
        assert buf.chunk_line_range(0, 8) == (0, 0, 1, 0)

    def test_multiple_chunks(self):
        source = "set x 1\nset y 2\n"
        buf = DocumentBuffer.from_source(source)
        assert buf.chunk_line_range(0, 8) == (0, 0, 1, 0)
        assert buf.chunk_line_range(8, 16) == (1, 0, 2, 0)

    def test_matches_old_implementation(self):
        """Verify chunk_line_range matches the old O(offset) implementation."""
        source = "proc foo {} {\n    set x 1\n}\n\nproc bar {} {\n    set y 2\n}\n"
        buf = DocumentBuffer.from_source(source)

        from compiler.parsing.command_segmenter import segment_top_level_chunks

        chunks = segment_top_level_chunks(source)
        for chunk in chunks:
            # Old implementation
            prefix = source[: chunk.start_offset]
            start_line = prefix.count("\n")
            last_nl = prefix.rfind("\n")
            start_col = chunk.start_offset - (last_nl + 1)
            end_prefix = source[: chunk.end_offset]
            end_line = end_prefix.count("\n")
            last_nl_end = end_prefix.rfind("\n")
            end_col = chunk.end_offset - (last_nl_end + 1)
            old = (start_line, start_col, end_line, end_col)

            # New implementation
            new = buf.chunk_line_range(chunk.start_offset, chunk.end_offset)
            assert new == old, f"chunk {chunk}: old={old}, new={new}"


# range_from_offsets


class TestRangeFromOffsets:
    def test_basic(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        r = buf.range_from_offsets(0, 4)
        assert r.start.line == 0
        assert r.start.character == 0
        assert r.end.line == 0
        assert r.end.character == 4

    def test_cross_line(self):
        buf = DocumentBuffer.from_source("hello\nworld")
        r = buf.range_from_offsets(3, 8)
        assert r.start.line == 0
        assert r.start.character == 3
        assert r.end.line == 1
        assert r.end.character == 2

    def test_empty_source(self):
        buf = DocumentBuffer.from_source("")
        r = buf.range_from_offsets(0, 0)
        assert r.start.line == 0
        assert r.end.line == 0


# Compatibility with old _chunk_line_range


class TestChunkLineRangeCompat:
    """Verify DocumentBuffer.chunk_line_range matches old implementation for real code."""

    def test_semicolon_separated(self):
        source = "set x 1; set y 2\n"
        buf = DocumentBuffer.from_source(source)
        from compiler.parsing.command_segmenter import segment_top_level_chunks

        chunks = segment_top_level_chunks(source)
        for chunk in chunks:
            # Old implementation
            prefix = source[: chunk.start_offset]
            start_line = prefix.count("\n")
            last_nl = prefix.rfind("\n")
            start_col = chunk.start_offset - (last_nl + 1)
            end_prefix = source[: chunk.end_offset]
            end_line = end_prefix.count("\n")
            last_nl_end = end_prefix.rfind("\n")
            end_col = chunk.end_offset - (last_nl_end + 1)
            old = (start_line, start_col, end_line, end_col)
            new = buf.chunk_line_range(chunk.start_offset, chunk.end_offset)
            assert new == old

    def test_proc_with_body(self):
        source = "proc foo {a b} {\n    set x $a\n    return $x\n}\n"
        buf = DocumentBuffer.from_source(source)
        from compiler.parsing.command_segmenter import segment_top_level_chunks

        chunks = segment_top_level_chunks(source)
        for chunk in chunks:
            prefix = source[: chunk.start_offset]
            start_line = prefix.count("\n")
            last_nl = prefix.rfind("\n")
            start_col = chunk.start_offset - (last_nl + 1)
            end_prefix = source[: chunk.end_offset]
            end_line = end_prefix.count("\n")
            last_nl_end = end_prefix.rfind("\n")
            end_col = chunk.end_offset - (last_nl_end + 1)
            old = (start_line, start_col, end_line, end_col)
            new = buf.chunk_line_range(chunk.start_offset, chunk.end_offset)
            assert new == old


# Integration with DocumentState


class TestDocumentStateBuffer:
    def test_buffer_property(self):
        from server.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "set x 1\nset y 2\n"
        buf = state.buffer
        assert buf.source == state.source
        assert buf.line_starts == compute_line_starts(state.source)

    def test_buffer_caches(self):
        from server.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "set x 1\n"
        buf1 = state.buffer
        buf2 = state.buffer
        assert buf1 is buf2  # same instance

    def test_buffer_invalidated_on_source_change(self):
        from server.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "set x 1\n"
        buf1 = state.buffer
        state.source = "set y 2\n"
        state._buffer = None
        buf2 = state.buffer
        assert buf1 is not buf2
        assert buf2.source == "set y 2\n"

    def test_lines_delegates_to_buffer(self):
        from server.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "line1\nline2\nline3"
        assert state.lines == ["line1", "line2", "line3"]

    def test_update_source_quick_invalidates_buffer(self):
        from server.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "set x 1\n"
        buf1 = state.buffer
        state.update_source_quick("set y 2\n", version=2)
        buf2 = state.buffer
        assert buf1 is not buf2
        assert buf2.source == "set y 2\n"

    def test_update_source_quick_buffer_matches_fresh(self):
        # The rope-reusing quick path must yield position mappings identical to
        # a from-scratch build of the new source.
        from server.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "proc f {} {\n  set x 1\n}\n"
        _ = state.buffer  # materialise the initial rope so the edit reuses it
        new_src = "\nproc f {} {\n  set x 1\n  set y 2\n}\n"
        state.update_source_quick(new_src, version=2)
        buf = state.buffer
        want = DocumentBuffer.from_source(new_src)
        assert buf.source == new_src
        assert buf.rope.text == new_src
        for off in range(len(new_src) + 1):
            assert buf.offset_to_line_col(off) == want.offset_to_line_col(off), off

    def test_full_update_reuses_quick_path_buffer(self):
        # The full-analysis update must carry the quick path's rope-backed
        # buffer through rather than rebuilding the position index from scratch.
        from server.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "proc f {} { set x 1 }\n"
        _ = state.buffer
        new_src = "\nproc f {} { set x 1 }\n"
        state.update_source_quick(new_src, version=2)
        quick_buf = state.buffer
        state.update(new_src, version=2)
        assert state.buffer is quick_buf
        assert state.buffer.rope.text == new_src


class TestFromEditEquivalence:
    """A rope-spliced ``from_edit`` buffer must be byte-identical to a fresh
    ``from_source`` buffer — same text and same mapping at every offset.  This
    is the safety net for reusing the rope across edits in
    ``update_source_quick`` (the rope is correctness-critical: a wrong splice
    would mis-place every diagnostic/hover position)."""

    CASES = [
        ("set x 1\n", "set y 2\n"),  # same-length replace
        ("set x 1\n", "\nset x 1\n"),  # insert blank line above
        ("set x 1\n", "set x 1\nset y 2\n"),  # append a line
        ("proc f {} {\n  set x 1\n}\n", "proc f {} {\n  set x 1\n  set y 2\n}\n"),  # mid insert
        ("abc\ndef\nghi\n", "abc\nghi\n"),  # delete a line
        ("hello world", "hello"),  # truncate, no trailing newline
        ("a\nb\nc", "a\nB\nc"),  # single-char change mid-document
        ("", "new content\n"),  # grow from empty
        ("old\n", ""),  # shrink to empty
    ]

    @staticmethod
    def _via_edit(old_src: str, new_src: str) -> DocumentBuffer:
        from compiler.parsing.incremental import infer_edit_range
        from shared.rope import RopeEdit

        prev = DocumentBuffer.from_source(old_src, version=1)
        edit = infer_edit_range(old_src, new_src)
        assert edit is not None, f"no edit inferred for {old_src!r} -> {new_src!r}"
        line_delta = new_src[edit.start : edit.new_end].count("\n") - old_src[
            edit.start : edit.old_end
        ].count("\n")
        return DocumentBuffer.from_edit(
            prev,
            new_src,
            RopeEdit(edit.start, edit.old_end, edit.new_end, line_delta),
            version=2,
        )

    def test_equivalence_across_edits(self):
        for old_src, new_src in self.CASES:
            got = self._via_edit(old_src, new_src)
            want = DocumentBuffer.from_source(new_src, version=2)
            assert got.source == new_src, (old_src, new_src)
            assert got.rope.text == new_src, (old_src, new_src)  # splice reproduced the text
            assert got.line_starts == want.line_starts, (old_src, new_src)
            assert got.lines == want.lines, (old_src, new_src)
            for off in range(len(new_src) + 1):
                assert got.offset_to_line_col(off) == want.offset_to_line_col(off), (
                    old_src,
                    new_src,
                    off,
                )

    def test_chained_edits_stay_consistent(self):
        # Apply several edits in sequence, reusing each result's rope — the
        # production pattern across consecutive keystrokes.
        from compiler.parsing.incremental import infer_edit_range
        from shared.rope import RopeEdit

        buf = DocumentBuffer.from_source("proc f {} {}\n", version=1)
        steps = [
            "proc f {} {}\nset a 1\n",
            "proc f {} {}\nset a 1\nset b 2\n",
            "proc f {x} {}\nset a 1\nset b 2\n",
            "set b 2\n",
        ]
        for i, new_src in enumerate(steps, start=2):
            edit = infer_edit_range(buf.source, new_src)
            assert edit is not None
            line_delta = new_src[edit.start : edit.new_end].count("\n") - buf.source[
                edit.start : edit.old_end
            ].count("\n")
            buf = DocumentBuffer.from_edit(
                buf,
                new_src,
                RopeEdit(edit.start, edit.old_end, edit.new_end, line_delta),
                version=i,
            )
            want = DocumentBuffer.from_source(new_src)
            assert buf.rope.text == new_src
            for off in range(len(new_src) + 1):
                assert buf.offset_to_line_col(off) == want.offset_to_line_col(off), (new_src, off)
