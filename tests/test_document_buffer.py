"""Tests for DocumentBuffer and compute_line_starts."""

from __future__ import annotations

from core.parsing.tokens import SourcePosition
from shared.document_buffer import (
    DocumentBuffer,
    compute_line_starts,
)

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

        from core.parsing.command_segmenter import segment_top_level_chunks

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
        from core.parsing.command_segmenter import segment_top_level_chunks

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
        from core.parsing.command_segmenter import segment_top_level_chunks

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
        from lsp.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "set x 1\nset y 2\n"
        buf = state.buffer
        assert buf.source == state.source
        assert buf.line_starts == compute_line_starts(state.source)

    def test_buffer_caches(self):
        from lsp.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "set x 1\n"
        buf1 = state.buffer
        buf2 = state.buffer
        assert buf1 is buf2  # same instance

    def test_buffer_invalidated_on_source_change(self):
        from lsp.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "set x 1\n"
        buf1 = state.buffer
        state.source = "set y 2\n"
        state._buffer = None
        buf2 = state.buffer
        assert buf1 is not buf2
        assert buf2.source == "set y 2\n"

    def test_lines_delegates_to_buffer(self):
        from lsp.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "line1\nline2\nline3"
        assert state.lines == ["line1", "line2", "line3"]

    def test_update_source_quick_invalidates_buffer(self):
        from lsp.workspace.document_state import DocumentState

        state = DocumentState(uri="test://file.tcl")
        state.source = "set x 1\n"
        buf1 = state.buffer
        state.update_source_quick("set y 2\n", version=2)
        buf2 = state.buffer
        assert buf1 is not buf2
        assert buf2.source == "set y 2\n"
