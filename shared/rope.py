"""A persistent balanced-tree rope with O(log n) edits and position mapping.

The problem: map between byte ``offset`` and ``(line, column)`` on a document
and apply edits, *without* O(document) work per edit.  A flat ``line_starts``
array (see :func:`shared.document_buffer.compute_line_starts`) answers lookups
in O(log n) but is rebuilt in O(len) on every edit, and a flat chunk list with
cumulative skip arrays still costs O(#chunks) per edit (every offset after the
edit shifts) — so a session of N edits is O(N·#chunks).

This structure is a **persistent implicit treap** (a randomised balanced binary
tree).  Each node owns a small text chunk and caches its subtree's total byte
length and newline count.  ``split`` and ``merge`` are O(log n) and *path-copy*
(they allocate only the O(log n) nodes on the touched path and **share every
untouched subtree by reference**), so :meth:`Rope.replace` is O(log n + edit
size) and an edit early in the file reuses the whole unaffected remainder.

``offset → (line, col)`` and ``(line, col) → offset`` are computed by **newline
rank/select** over the cached per-subtree newline counts (O(log n)); leaves need
not align to line boundaries.  Results match the flat ``DocumentBuffer`` index
byte-for-byte (the differential gate in ``tests/test_rope.py``).

The structure is *value-immutable*: :meth:`Rope.replace` returns a new ``Rope``
and never mutates ``self``, so a cached snapshot's rope stays valid.
"""

from __future__ import annotations

import random
from bisect import bisect_left
from dataclasses import dataclass

# Max chars per leaf chunk.  Bounds leaf size so the tree stays shallow and a
# split touches a bounded amount of text; small enough that re-lexing a leaf is
# cheap, large enough that #nodes stays modest.
_MAX_LEAF = 2048

_rng = random.Random(0x726F7065)  # "rope"
# Treap priorities only. Shared across threads, but the GIL makes getrandbits()
# atomic so there's no corruption; the only cross-thread effect is that tree
# *shape* depends on global call order. Shape is unobservable (split/merge keep
# the in-order text identical), so the rope's *content/queries* are fully
# deterministic — the shape is not.


@dataclass(frozen=True, slots=True)
class _Node:
    """One node of the implicit treap: a text chunk plus two subtrees.

    Aggregates (``length``/``newlines``) cover the *whole subtree* rooted here
    (this node's chunk plus both children), enabling O(log n) rank/select.
    """

    length: int  # total bytes in subtree
    newlines: int  # total '\n' in subtree
    priority: int  # max-heap key that randomises (balances) the tree
    text: str  # this node's chunk (always non-empty)
    nl_offsets: tuple[int, ...]  # relative offsets of each '\n' in text
    left: _Node | None
    right: _Node | None


def _leaf(text: str) -> _Node:
    nl = tuple(i for i, ch in enumerate(text) if ch == "\n")
    return _Node(len(text), len(nl), _rng.getrandbits(62), text, nl, None, None)


def _mk(node: _Node, left: _Node | None, right: _Node | None) -> _Node:
    """Rebuild *node* with new children and recomputed subtree aggregates."""
    length = len(node.text) + (left.length if left else 0) + (right.length if right else 0)
    newlines = (
        len(node.nl_offsets) + (left.newlines if left else 0) + (right.newlines if right else 0)
    )
    return _Node(length, newlines, node.priority, node.text, node.nl_offsets, left, right)


def _merge(a: _Node | None, b: _Node | None) -> _Node | None:
    """Concatenate two treaps (all of *a* precedes all of *b*).  O(log n)."""
    if a is None:
        return b
    if b is None:
        return a
    if a.priority >= b.priority:
        return _mk(a, a.left, _merge(a.right, b))
    return _mk(b, _merge(a, b.left), b.right)


def _split(t: _Node | None, k: int) -> tuple[_Node | None, _Node | None]:
    """Split *t* so the left result holds the first *k* bytes.  O(log n)."""
    if t is None:
        return (None, None)
    left_len = t.left.length if t.left else 0
    tlen = len(t.text)
    if k <= left_len:
        lo, hi = _split(t.left, k)
        return (lo, _mk(t, hi, t.right))
    if k >= left_len + tlen:
        lo, hi = _split(t.right, k - left_len - tlen)
        return (_mk(t, t.left, lo), hi)
    # k falls inside this node's own chunk — cut the chunk in two.
    cut = k - left_len
    left = _merge(t.left, _leaf(t.text[:cut]))
    right = _merge(_leaf(t.text[cut:]), t.right)
    return (left, right)


def _build(text: str) -> _Node | None:
    """Build a treap from *text*, chunked at ``_MAX_LEAF``.  O(n)."""
    if not text:
        return None
    root: _Node | None = None
    for i in range(0, len(text), _MAX_LEAF):
        root = _merge(root, _leaf(text[i : i + _MAX_LEAF]))
    return root


def _text_into(t: _Node | None, out: list[str]) -> None:
    if t is None:
        return
    _text_into(t.left, out)
    out.append(t.text)
    _text_into(t.right, out)


def _count_newlines_before(t: _Node | None, off: int) -> int:
    """Number of ``'\\n'`` at positions ``< off``.  O(log n)."""
    res = 0
    while t is not None:
        left = t.left
        left_len = left.length if left else 0
        if off <= left_len:
            t = left
        else:
            res += left.newlines if left else 0
            local = off - left_len
            if local <= len(t.text):
                return res + bisect_left(t.nl_offsets, local)
            res += len(t.nl_offsets)
            off = local - len(t.text)
            t = t.right
    return res


def _select_newline(t: _Node | None, k: int) -> int:
    """Absolute offset of the ``k``-th (0-based) ``'\\n'``.  O(log n)."""
    base = 0
    while t is not None:
        left = t.left
        left_nl = left.newlines if left else 0
        left_len = left.length if left else 0
        if k < left_nl:
            t = left
            continue
        k -= left_nl
        if k < len(t.nl_offsets):
            return base + left_len + t.nl_offsets[k]
        k -= len(t.nl_offsets)
        base += left_len + len(t.text)
        t = t.right
    raise IndexError("newline index out of range")


@dataclass(frozen=True, slots=True)
class RopeEdit:
    """The byte/line delta of a single :meth:`Rope.replace_with_edit`.

    ``[start, old_end)`` of the old text was replaced by ``[start, new_end)`` of
    the new text.  ``line_delta`` is the change in newline count.  These feed
    :meth:`AnchorTable.rebased` and the incremental reparse's suffix shift.
    """

    start: int
    old_end: int
    new_end: int
    line_delta: int

    @property
    def offset_delta(self) -> int:
        return self.new_end - self.old_end


class Rope:
    """An immutable, balanced-tree text buffer with O(log n) edits + indexing."""

    __slots__ = ("_root",)

    def __init__(self, root: _Node | None) -> None:
        self._root = root

    # Construction

    @classmethod
    def from_text(cls, text: str) -> Rope:
        return cls(_build(text))

    # Basic accessors

    @property
    def text(self) -> str:
        out: list[str] = []
        _text_into(self._root, out)
        return "".join(out)

    def __len__(self) -> int:
        return self._root.length if self._root else 0

    @property
    def line_count(self) -> int:
        """Number of lines in the ``line_starts`` sense (``#newlines + 1``)."""
        return (self._root.newlines if self._root else 0) + 1

    # Position conversion (byte-for-byte parity with DocumentBuffer)

    def offset_to_line_col(self, offset: int) -> tuple[int, int]:
        length = self._root.length if self._root else 0
        safe = 0 if offset < 0 else (length if offset > length else offset)
        line = _count_newlines_before(self._root, safe)
        line_start = 0 if line == 0 else _select_newline(self._root, line - 1) + 1
        return line, safe - line_start

    def line_col_to_offset(self, line: int, character: int) -> int:
        length = self._root.length if self._root else 0
        total_nl = self._root.newlines if self._root else 0
        safe_line = 0 if line < 0 else (total_nl if line > total_nl else line)
        line_start = 0 if safe_line == 0 else _select_newline(self._root, safe_line - 1) + 1
        if safe_line < total_nl:
            line_end = _select_newline(self._root, safe_line)  # offset of the terminating '\n'
        else:
            line_end = length
        line_length = line_end - line_start
        if line_length < 0:
            line_length = 0
        safe_char = 0 if character < 0 else (line_length if character > line_length else character)
        return line_start + safe_char

    def slice(self, start: int, end: int) -> str:
        length = self._root.length if self._root else 0
        a = max(0, min(start, length))
        b = max(a, min(end, length))
        if a == b:
            return ""
        _, rest = _split(self._root, a)
        mid, _ = _split(rest, b - a)
        out: list[str] = []
        _text_into(mid, out)
        return "".join(out)

    # Editing

    def replace(self, start: int, end: int, replacement: str) -> Rope:
        """Return a new ``Rope`` with ``text[start:end]`` replaced.  O(log n + |replacement|).

        Untouched subtrees are shared by reference with ``self``; only the
        O(log n) nodes on the split/merge path are allocated.
        """
        length = self._root.length if self._root else 0
        a = max(0, min(start, length))
        b = max(a, min(end, length))
        left, rest = _split(self._root, a)
        _, right = _split(rest, b - a)
        mid = _build(replacement)
        return Rope(_merge(_merge(left, mid), right))

    def replace_with_edit(self, start: int, end: int, replacement: str) -> tuple[Rope, RopeEdit]:
        """:meth:`replace`, also returning the :class:`RopeEdit` delta.

        The delta carries the clamped span and the newline-count change, which
        the incremental reparse and :class:`AnchorTable` use to shift the
        unaffected suffix without re-scanning.
        """
        length = self._root.length if self._root else 0
        a = max(0, min(start, length))
        b = max(a, min(end, length))
        removed = self.slice(a, b)
        line_delta = replacement.count("\n") - removed.count("\n")
        new_rope = self.replace(a, b, replacement)
        return new_rope, RopeEdit(
            start=a, old_end=b, new_end=a + len(replacement), line_delta=line_delta
        )


@dataclass(frozen=True, slots=True)
class AnchorBase:
    """The absolute position of an anchor's origin in the current document.

    An analysis artefact stores its positions *relative* to an anchor; the
    absolute position of any inner coordinate is the anchor's base plus the
    relative offset (with the line-0-vs-later-line rule applied by the rebase
    helpers in ``shared`` — see the unified rebase module).
    """

    offset: int
    line: int
    character: int


class AnchorTable:
    """Maps an anchor id to its current :class:`AnchorBase`.

    The side index that makes "an edit early in the file shifts everything below
    it" an O(#anchors) base update rather than a re-analysis: on an edit,
    :meth:`rebased` shifts only the anchors lying on lines strictly *below* the
    edit (their byte offset and line move by the edit's deltas; their column is
    unchanged because they sit on later lines — the same exact-shift condition
    the incremental reparse relies on in ``compiler/parsing/incremental.py``).
    Anchors on or above the edit's lines are left untouched here; whether they
    are reusable is the dependency graph's decision (rebase vs recompute), not
    the table's.

    Value-immutable: :meth:`rebased` returns a new table.
    """

    __slots__ = ("_bases",)

    def __init__(self, bases: dict[int, AnchorBase] | None = None) -> None:
        self._bases: dict[int, AnchorBase] = dict(bases) if bases else {}

    def set(self, anchor_id: int, base: AnchorBase) -> None:
        self._bases[anchor_id] = base

    def get(self, anchor_id: int) -> AnchorBase | None:
        return self._bases.get(anchor_id)

    def __len__(self) -> int:
        return len(self._bases)

    def rebased(self, edit: RopeEdit, *, boundary_offset: int) -> AnchorTable:
        """Return a new table with anchors at/after *boundary_offset* shifted.

        *boundary_offset* is the first line-start at or after the edit's end (the
        suffix boundary): anchors there sit on lines strictly below the edit, so
        shifting offset + line (column unchanged) is exact.

        A **negative** *boundary_offset* is the "no safe suffix boundary"
        sentinel — the edit lies on the final line with no trailing newline, so
        no anchor sits on a strictly-later line and *none* can be shifted by a
        pure delta (their column would change). In that case no anchor is
        rebaseable: every anchor is carried forward unchanged and the consumer
        (dependency graph) recomputes whatever the edit touched. Without this
        guard ``base.offset >= -1`` matches every anchor, corrupting anchors that
        precede the edit on the final line.
        """
        if boundary_offset < 0:
            return AnchorTable(self._bases)
        offset_delta = edit.offset_delta
        line_delta = edit.line_delta
        new: dict[int, AnchorBase] = {}
        for aid, base in self._bases.items():
            if base.offset >= boundary_offset:
                new[aid] = AnchorBase(
                    offset=base.offset + offset_delta,
                    line=base.line + line_delta,
                    character=base.character,
                )
            else:
                new[aid] = base
        return AnchorTable(new)
