"""Shared per-document position infrastructure.

``DocumentBuffer`` is the single source of truth for source text, version,
and position mapping.  Every LSP feature handler should use it instead of
recomputing ``source.split("\\n")`` or constructing ad-hoc ``SourceMap``
instances.

Position mapping is backed by a :class:`shared.rope.Rope` (a persistent
balanced tree): ``offset``↔``(line, col)`` are O(log n), and an edit can be
applied in O(log n) with structural reuse of the unchanged remainder via
:meth:`DocumentBuffer.from_edit`.  The flat ``line_starts`` tuple is still
available (lazily) for the few consumers that read it directly.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from shared.diagnostic import Range
from shared.rope import Rope, RopeEdit
from shared.tokens import SourcePosition


def compute_line_starts(source: str) -> tuple[int, ...]:
    """Build a line-starts index from scratch — O(len(source))."""
    starts = [0]
    for i, ch in enumerate(source):
        if ch == "\n":
            starts.append(i + 1)
    return tuple(starts)


@dataclass(slots=True)
class DocumentBuffer:
    """Shared per-document position infrastructure.

    Replaces scattered ``source.split("\\n")``, ``SourceMap(source)``,
    ``_chunk_line_range(source, chunk)``, and ``position_from_relative()``
    calls with a single cached object backed by a :class:`Rope`.
    """

    source: str
    version: int | None
    rope: Rope

    # Lazily cached derived data.
    _lines: list[str] | None = field(default=None, repr=False)
    _line_starts: tuple[int, ...] | None = field(default=None, repr=False)

    # Constructors

    @classmethod
    def from_source(
        cls,
        source: str,
        version: int | None = None,
    ) -> DocumentBuffer:
        """Create a buffer with a freshly built rope (O(len(source)))."""
        return cls(source=source, version=version, rope=Rope.from_text(source))

    @classmethod
    def from_edit(
        cls,
        prev: DocumentBuffer,
        new_source: str,
        edit: RopeEdit,
        version: int | None = None,
    ) -> DocumentBuffer:
        """Create the post-edit buffer reusing *prev*'s rope structure.

        Applies *edit* to ``prev.rope`` in O(log n + |edit|), sharing every
        untouched subtree by reference — the incremental path that avoids
        re-scanning the whole document on each keystroke.  *edit* must describe
        the change from ``prev.source`` to *new_source*.
        """
        replacement = new_source[edit.start : edit.new_end]
        new_rope = prev.rope.replace(edit.start, edit.old_end, replacement)
        return cls(source=new_source, version=version, rope=new_rope)

    # Cached properties

    @property
    def lines(self) -> list[str]:
        """Source split by ``\\n``, cached for the buffer's lifetime."""
        if self._lines is None:
            self._lines = self.source.split("\n")
        return self._lines

    @property
    def line_starts(self) -> tuple[int, ...]:
        """Flat line-starts tuple, computed lazily (for direct consumers)."""
        if self._line_starts is None:
            self._line_starts = compute_line_starts(self.source)
        return self._line_starts

    # Position conversion (delegated to the rope; O(log n))

    def offset_to_position(self, offset: int) -> SourcePosition:
        """offset → (line, character, offset), clamped to the document."""
        n = len(self.source)
        safe = 0 if offset < 0 else (n if offset > n else offset)
        line, col = self.rope.offset_to_line_col(safe)
        return SourcePosition(line=line, character=col, offset=safe)

    def position_to_offset(self, line: int, character: int) -> int:
        """(line, character) → offset, with clamping."""
        return self.rope.line_col_to_offset(line, character)

    def offset_to_line_col(self, offset: int) -> tuple[int, int]:
        """offset → (line, col) tuple (no SourcePosition alloc)."""
        return self.rope.offset_to_line_col(offset)

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
