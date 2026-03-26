"""Shared per-document position infrastructure.

``DocumentBuffer`` is the single source of truth for source text, version,
and line-start metadata.  Every LSP feature handler should use it instead
of recomputing ``source.split("\\n")`` or constructing ad-hoc ``SourceMap``
instances.
"""

from __future__ import annotations

from bisect import bisect_right
from dataclasses import dataclass, field

from ..analysis.semantic_model import Range
from ..parsing.tokens import SourcePosition

# ---------------------------------------------------------------------------
# EditDescriptor
# ---------------------------------------------------------------------------


@dataclass(frozen=True, slots=True)
class EditDescriptor:
    """Describes a single edit to a text buffer (tree-sitter style).

    ``start_offset`` is the byte offset where the edit begins in the
    *old* text.  ``old_length`` bytes are removed, then ``new_length``
    bytes are inserted.
    """

    start_offset: int
    old_length: int
    new_length: int

    @property
    def delta(self) -> int:
        return self.new_length - self.old_length


# ---------------------------------------------------------------------------
# Line-starts helpers
# ---------------------------------------------------------------------------


def compute_line_starts(source: str) -> tuple[int, ...]:
    """Build a line-starts index from scratch — O(len(source))."""
    starts = [0]
    for i, ch in enumerate(source):
        if ch == "\n":
            starts.append(i + 1)
    return tuple(starts)


def update_line_starts(
    old_starts: tuple[int, ...],
    edit: EditDescriptor,
    new_source: str,
) -> tuple[int, ...]:
    """Incrementally update *old_starts* after applying *edit*.

    1. Find affected line range via bisect on *old_starts*.
    2. Rescan only the edited region in *new_source* for newlines.
    3. Shift all lines after the edit by ``edit.delta``.
    4. Splice: ``head + new_region_starts + shifted_tail``.

    Cost: O(log n) bisect + O(edit_size) scan + O(lines_after_edit) shift.
    """
    edit_old_end = edit.start_offset + edit.old_length
    edit_new_end = edit.start_offset + edit.new_length

    # Lines whose start falls within the *old* edited region are invalidated.
    first_affected = bisect_right(old_starts, edit.start_offset) - 1
    first_affected = max(0, first_affected)

    # Find the first line that starts *after* the old edit region.
    after_edit = bisect_right(old_starts, edit_old_end)

    # Head: lines entirely before the edit (unaffected).
    head = list(old_starts[: first_affected + 1])

    # Rescan the edited region in the new source.
    region_starts: list[int] = []
    scan_start = old_starts[first_affected]
    for i in range(scan_start, min(edit_new_end, len(new_source))):
        if new_source[i] == "\n":
            region_starts.append(i + 1)

    # Tail: lines after the edit, shifted by delta.
    delta = edit.delta
    tail = [s + delta for s in old_starts[after_edit:]]

    return tuple(head + region_starts + tail)


# ---------------------------------------------------------------------------
# DocumentBuffer
# ---------------------------------------------------------------------------


@dataclass(slots=True)
class DocumentBuffer:
    """Shared per-document position infrastructure.

    Replaces scattered ``source.split("\\n")``, ``SourceMap(source)``,
    ``_chunk_line_range(source, chunk)``, and ``position_from_relative()``
    calls with a single cached object.
    """

    source: str
    version: int | None
    line_starts: tuple[int, ...]
    edit: EditDescriptor | None = None

    # Lazily cached derived data.
    _lines: list[str] | None = field(default=None, repr=False)

    # ------------------------------------------------------------------
    # Constructors
    # ------------------------------------------------------------------

    @classmethod
    def from_source(
        cls,
        source: str,
        version: int | None = None,
        edit: EditDescriptor | None = None,
    ) -> DocumentBuffer:
        """Create a buffer with a freshly computed line-starts index."""
        return cls(
            source=source,
            version=version,
            line_starts=compute_line_starts(source),
            edit=edit,
        )

    @classmethod
    def updated(
        cls,
        prev: DocumentBuffer,
        new_source: str,
        new_version: int | None = None,
        edit: EditDescriptor | None = None,
    ) -> DocumentBuffer:
        """Create a new buffer, incrementally updating line_starts if *edit* is given."""
        if edit is not None:
            new_starts = update_line_starts(prev.line_starts, edit, new_source)
        else:
            new_starts = compute_line_starts(new_source)
        return cls(
            source=new_source,
            version=new_version,
            line_starts=new_starts,
            edit=edit,
        )

    # ------------------------------------------------------------------
    # Cached properties
    # ------------------------------------------------------------------

    @property
    def lines(self) -> list[str]:
        """Source split by ``\\n``, cached for the buffer's lifetime."""
        if self._lines is None:
            self._lines = self.source.split("\n")
        return self._lines

    # ------------------------------------------------------------------
    # Position conversion
    # ------------------------------------------------------------------

    def offset_to_position(self, offset: int) -> SourcePosition:
        """O(log n) offset → (line, character, offset) via bisect."""
        safe = max(0, min(offset, len(self.source)))
        line = bisect_right(self.line_starts, safe) - 1
        line = max(0, line)
        col = safe - self.line_starts[line]
        return SourcePosition(line=line, character=col, offset=safe)

    def position_to_offset(self, line: int, character: int) -> int:
        """O(1) (line, character) → offset, with clamping."""
        if not self.line_starts:
            return 0
        safe_line = max(0, min(line, len(self.line_starts) - 1))
        line_start = self.line_starts[safe_line]
        # Clamp character to line length.
        if safe_line + 1 < len(self.line_starts):
            line_end = self.line_starts[safe_line + 1] - 1  # exclude '\n'
        else:
            line_end = len(self.source)
        line_length = max(0, line_end - line_start)
        safe_char = max(0, min(character, line_length))
        return line_start + safe_char

    def offset_to_line_col(self, offset: int) -> tuple[int, int]:
        """O(log n) offset → (line, col) tuple (no SourcePosition alloc)."""
        safe = max(0, min(offset, len(self.source)))
        line = bisect_right(self.line_starts, safe) - 1
        line = max(0, line)
        return line, safe - self.line_starts[line]

    def range_from_offsets(self, start: int, end_inclusive: int) -> Range:
        """Build a Range from inclusive source offsets."""
        if not self.source:
            pos = SourcePosition(line=0, character=0, offset=0)
            return Range(start=pos, end=pos)

        max_end = len(self.source) - 1
        safe_start = max(0, min(start, max_end))
        safe_end = max(0, min(end_inclusive, max_end))
        if safe_end < safe_start:
            safe_end = safe_start

        return Range(
            start=self.offset_to_position(safe_start),
            end=self.offset_to_position(safe_end),
        )

    def chunk_line_range(
        self,
        start_offset: int,
        end_offset: int,
    ) -> tuple[int, int, int, int]:
        """O(log n) replacement for the O(offset) ``_chunk_line_range()``.

        Returns ``(start_line, start_col, end_line, end_col)``.
        """
        sl, sc = self.offset_to_line_col(start_offset)
        el, ec = self.offset_to_line_col(end_offset)
        return sl, sc, el, ec
