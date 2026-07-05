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

"""Shared per-document position infrastructure.

``DocumentBuffer`` is the single source of truth for source text, version,
and position mapping.  Every LSP feature handler should use it instead of
recomputing ``source.split("\\n")`` or constructing ad-hoc ``SourceMap``
instances.

Position mapping is backed by a :class:`shared.rope.Rope` (a persistent
balanced tree): ``offset``↔``(line, col)`` are O(log n), and an edit can be
applied in O(log n) with structural reuse of the unchanged remainder via
:meth:`DocumentBuffer.from_edit`.

RAM model (why the apparent duplication is bounded and intentional):
  * ``source`` is the canonical text string — the contract many feature
    handlers depend on (they slice it directly), so it is kept eagerly.
  * ``rope`` is a *structural* second copy: its per-line chunks enable the
    O(log n) edit + cross-version sharing above, so an old version that is
    still pinned shares every untouched subtree with the current one rather
    than holding an independent full copy.
  * ``lines`` and ``line_starts`` are **lazy** flat caches — each is built
    (and a full split / index allocated) only the first time a direct
    consumer reads it on this version, never speculatively.  They exist
    because the rope answers point queries in O(log n) but several handlers
    want the whole flat index in one call (``server.py`` line slicing,
    ``definition.py`` offset lookup); recomputing per call would be worse.
Because only O(open documents) buffers are live at once (older versions are
GC'd once unreferenced — see the class docstring), this duplication is a small
constant per open file, not a per-edit leak.
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


@dataclass
class DocumentBuffer:
    """Shared per-document position infrastructure.

    Replaces scattered ``source.split("\\n")``, ``SourceMap(source)``,
    ``_chunk_line_range(source, chunk)``, and ``position_from_relative()``
    calls with a single cached object backed by a :class:`Rope`.

    Intentionally *not* a ``slots`` dataclass: instances must be weak-
    referenceable so the per-document MVCC version registry
    (:class:`~server.workspace.document_state.DocumentState`) can hold each
    version's buffer *weakly* — a version is reclaimed by Python's GC as soon as
    no in-flight reader (request handler / analysis task) still holds it, while
    the immutable rope's structural sharing keeps any still-pinned older version
    cheap (it shares every untouched subtree with the current one).  Only
    O(open documents) buffers are live at once, so the per-instance ``__dict__``
    is negligible.  (``slots=True, weakref_slot=True`` would be the slotted
    equivalent, but the type checker's bundled stubs don't yet model
    ``weakref_slot``.)
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
        """Flat line-starts tuple, computed lazily (for direct consumers).

        Materialised on first access only (see the module RAM-model note): the
        rope answers single point queries in O(log n), but a handler that wants
        the whole index in one shot (e.g. ``definition.py``) reads this instead.
        """
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
