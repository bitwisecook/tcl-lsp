"""Tests for the persistent balanced-tree rope (``shared/rope.py``).

Philosophy: equivalence with the old flat ``line_starts`` index was proven once
and is now guarded broadly by the full LSP suite (which runs every position
feature through this rope).  So this file keeps only a small **contract pin**
(edge cases + clamping that document the exact position semantics the codebase
relies on) and focuses on the rope's own robustness: creation, heavy editing,
multi-editing, and **peer-data-structure tracking** (rope ↔ ``AnchorTable``
consistency — the invariant the rebase architecture rests on).

Oracles are independent of the rope: a hand-written flat line index for the
contract pin, and plain Python string splicing for editing.
"""

from __future__ import annotations

import random
import sys
from bisect import bisect_right
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from shared.rope import AnchorBase, AnchorTable, Rope

# --- independent oracles -----------------------------------------------------


def _flat_line_starts(source: str) -> list[int]:
    starts = [0]
    for i, ch in enumerate(source):
        if ch == "\n":
            starts.append(i + 1)
    return starts


def _flat_offset_to_line_col(source: str, offset: int) -> tuple[int, int]:
    ls = _flat_line_starts(source)
    safe = max(0, min(offset, len(source)))
    line = max(0, bisect_right(ls, safe) - 1)
    return line, safe - ls[line]


def _flat_line_col_to_offset(source: str, line: int, character: int) -> int:
    ls = _flat_line_starts(source)
    safe_line = max(0, min(line, len(ls) - 1))
    line_start = ls[safe_line]
    line_end = ls[safe_line + 1] - 1 if safe_line + 1 < len(ls) else len(source)
    line_length = max(0, line_end - line_start)
    safe_char = max(0, min(character, line_length))
    return line_start + safe_char


# --- creation ----------------------------------------------------------------

_SAMPLES = [
    "",
    "x",
    "abc\n",
    "\n\n\n",
    "a\nb\nc",
    "line1\nline2\nline3\n",
    "café\nrésumé\n",  # multi-byte (offsets are codepoint indices)
    "no newline at end",
    "\nleading",
    "trailing\n\n",
    "x" * 5000,  # many leaf chunks, no newlines
    "\n".join(f"line {i}" for i in range(2000)),  # many lines + chunks
]


class TestCreation:
    def test_round_trip_text_len_linecount(self):
        for text in _SAMPLES:
            rope = Rope.from_text(text)
            assert rope.text == text
            assert len(rope) == len(text)
            assert rope.line_count == len(_flat_line_starts(text))

    def test_randomised_shapes_are_output_stable(self):
        # The treap's shape is randomised; identical text must yield identical
        # observable behaviour regardless of shape.
        text = "alpha\nbeta\ngamma\ndelta\n"
        ropes = [Rope.from_text(text) for _ in range(5)]
        for off in range(len(text) + 1):
            cols = {r.offset_to_line_col(off) for r in ropes}
            assert len(cols) == 1


# --- contract pin (the one-time equivalence, kept small) ---------------------


class TestPositionContract:
    """A focused pin of the exact flat-index semantics (clamping, edges)."""

    _CASES = ["", "x", "a\nbb\nccc\n", "\n", "no\ntrailing", "\n\n"]

    def test_offset_to_line_col_including_clamps(self):
        for text in self._CASES:
            rope = Rope.from_text(text)
            for off in range(-3, len(text) + 4):
                assert rope.offset_to_line_col(off) == _flat_offset_to_line_col(text, off)

    def test_line_col_to_offset_including_clamps(self):
        for text in self._CASES:
            rope = Rope.from_text(text)
            n_lines = len(_flat_line_starts(text))
            for line in range(-1, n_lines + 2):
                for char in range(-1, 6):
                    assert rope.line_col_to_offset(line, char) == _flat_line_col_to_offset(
                        text, line, char
                    )


# --- slicing -----------------------------------------------------------------


class TestSlice:
    def test_short_exhaustive(self):
        for text in ("", "abc", "a\nb\nc\n", "\n\n"):
            rope = Rope.from_text(text)
            n = len(text)
            for a in range(-1, n + 1):
                for b in range(a, n + 2):
                    assert rope.slice(a, b) == text[max(0, a) : max(0, b)]

    def test_large_random(self):
        text = "\n".join(f"row {i}" for i in range(500))
        rope = Rope.from_text(text)
        rng = random.Random(7)
        for _ in range(300):
            a = rng.randint(0, len(text))
            b = rng.randint(a, len(text))
            assert rope.slice(a, b) == text[a:b]


# --- heavy editing (vs the Python-string oracle) -----------------------------


class TestHeavyEditing:
    def test_long_random_edit_sequence(self):
        rng = random.Random(0xC0FFEE)
        for seed in ("line1\nline2\nline3\nline4\n", "abcdef", "", "x" * 3000):
            rope = Rope.from_text(seed)
            ref = seed
            for _ in range(500):
                n = len(ref)
                start = rng.randint(0, n)
                end = rng.randint(start, n)
                repl = rng.choice(["", "x", "x\ny", "\n", "abc\n\ndef", "Z", "Q" * 100])
                rope = rope.replace(start, end, repl)
                ref = ref[:start] + repl + ref[end:]
                assert rope.text == ref
            # Conversions still match the flat oracle after heavy editing.
            step = max(1, len(ref) // 400)
            for off in range(0, len(ref) + 1, step):
                assert rope.offset_to_line_col(off) == _flat_offset_to_line_col(ref, off)

    def test_edit_at_top_then_bottom(self):
        rope = Rope.from_text("body\nmore\n")
        rope = rope.replace(0, 0, "header\n")  # insert at top
        rope = rope.replace(len(rope.text), len(rope.text), "footer\n")  # append
        assert rope.text == "header\nbody\nmore\nfooter\n"
        for off in range(len(rope.text) + 1):
            assert rope.offset_to_line_col(off) == _flat_offset_to_line_col(rope.text, off)


# --- multi-editing (batch / multi-cursor) ------------------------------------


class TestMultiEditing:
    def test_batch_of_simultaneous_edits(self):
        # Multi-cursor: several edits described against the same base text,
        # applied right-to-left so earlier offsets stay valid.
        base = "aa\nbb\ncc\ndd\n"
        edits = [(0, 0, "X"), (3, 3, "Y"), (6, 6, "Z"), (9, 9, "W")]
        rope = Rope.from_text(base)
        ref = base
        for start, end, repl in sorted(edits, key=lambda e: e[0], reverse=True):
            rope = rope.replace(start, end, repl)
            ref = ref[:start] + repl + ref[end:]
        assert rope.text == ref == "Xaa\nYbb\nZcc\nWdd\n"
        for off in range(len(ref) + 1):
            assert rope.offset_to_line_col(off) == _flat_offset_to_line_col(ref, off)

    def test_interleaved_insert_delete_batch(self):
        base = "\n".join(f"item{i}" for i in range(20)) + "\n"
        rope = Rope.from_text(base)
        ref = base
        rng = random.Random(99)
        # 50 rounds of "apply k edits, then verify" — simulates rapid multi-edits.
        for _ in range(50):
            k = rng.randint(2, 5)
            for _ in range(k):
                n = len(ref)
                start = rng.randint(0, n)
                end = rng.randint(start, min(n, start + 4))
                repl = rng.choice(["", "\n", "ZZ", "a\nb"])
                rope = rope.replace(start, end, repl)
                ref = ref[:start] + repl + ref[end:]
            assert rope.text == ref


# --- peer-data-structure tracking (rope ↔ AnchorTable) -----------------------


def _suffix_boundary(old_source: str, old_end: int) -> int:
    """First line-start at/after *old_end* — the offset from which content is
    on a strictly-later line than the edit and so shifts by a pure delta.
    Mirrors ``incremental.py``'s ``suffix_min``."""
    nl = old_source.find("\n", old_end)
    return -1 if nl == -1 else nl + 1


class TestAnchorPeerTracking:
    """The rope and a peer ``AnchorTable`` must stay consistent across an edit:
    an anchor on a line strictly below the edit, after rebasing, points at the
    same content in the new rope as a fresh computation would."""

    def test_anchors_track_blank_line_insert_at_top(self):
        src = "AAA\nBBB\nCCC\nDDD\n"
        rope = Rope.from_text(src)
        line_offsets = [0, 4, 8, 12]  # starts of AAA, BBB, CCC, DDD
        table = AnchorTable()
        for i, off in enumerate(line_offsets):
            line, col = rope.offset_to_line_col(off)
            table.set(i, AnchorBase(off, line, col))

        new_rope, edit = rope.replace_with_edit(0, 0, "\n")  # blank line at top
        boundary = _suffix_boundary(src, edit.old_end)  # == 4 (after "AAA\n")
        rebased = table.rebased(edit, boundary_offset=boundary)

        # Anchors strictly below the edit (BBB, CCC, DDD) rebased correctly:
        for i, off in enumerate(line_offsets):
            if off < boundary:
                continue  # on the edit's line — the consumer recomputes it
            base = rebased.get(i)
            assert base is not None
            # The base's stored (line, col) matches a fresh lookup at its offset.
            assert new_rope.offset_to_line_col(base.offset) == (base.line, base.character)
            # And it still points at the same 3-char content.
            assert new_rope.slice(base.offset, base.offset + 3) == src[off : off + 3]

    def test_anchors_track_multiline_insert(self):
        src = "p\nq\nr\ns\nt\n"
        rope = Rope.from_text(src)
        # Anchor every line start.
        starts = [i for i, off in enumerate(_flat_line_starts(src)[:-1])]
        offs = _flat_line_starts(src)[:-1]
        table = AnchorTable()
        for i, off in zip(starts, offs):
            line, col = rope.offset_to_line_col(off)
            table.set(i, AnchorBase(off, line, col))

        # Insert two lines after the first line.
        new_rope, edit = rope.replace_with_edit(2, 2, "X\nY\n")
        boundary = _suffix_boundary(src, edit.old_end)  # after "q\n" -> offset 4
        rebased = table.rebased(edit, boundary_offset=boundary)

        for i, off in zip(starts, offs):
            if off < boundary:
                continue
            base = rebased.get(i)
            assert base is not None
            assert new_rope.offset_to_line_col(base.offset) == (base.line, base.character)
            assert new_rope.slice(base.offset, base.offset + 1) == src[off]

    def test_rebase_is_value_immutable(self):
        from shared.rope import RopeEdit

        table = AnchorTable({1: AnchorBase(50, 5, 0)})
        edit = RopeEdit(start=0, old_end=0, new_end=1, line_delta=1)
        rebased = table.rebased(edit, boundary_offset=0)
        assert rebased.get(1) == AnchorBase(51, 6, 0)
        assert table.get(1) == AnchorBase(50, 5, 0)  # original untouched

    def test_no_safe_boundary_leaves_all_anchors_unshifted(self):
        # Edit on the final line with no trailing newline: _suffix_boundary
        # returns -1.  No anchor sits on a strictly-later line, so none may be
        # shifted — including anchors *before* the edit on that final line.
        src = "AAA\nBBB CCC"  # final line "BBB CCC" has no trailing newline
        rope = Rope.from_text(src)
        # Anchor at "BBB" (offset 4) and "CCC" (offset 8), both on the last line.
        table = AnchorTable()
        for aid, off in ((0, 4), (1, 8)):
            line, col = rope.offset_to_line_col(off)
            table.set(aid, AnchorBase(off, line, col))

        # Append " DDD" at end-of-file (offset 10), on the same final line.
        _new_rope, edit = rope.replace_with_edit(len(src), len(src), " DDD")
        boundary = _suffix_boundary(src, edit.old_end)
        assert boundary == -1  # precondition: no safe suffix boundary

        rebased = table.rebased(edit, boundary_offset=boundary)
        # Every anchor is carried forward unchanged (not shifted by the -1 bug).
        assert rebased.get(0) == AnchorBase(4, 1, 0)
        assert rebased.get(1) == AnchorBase(8, 1, 4)

    def test_no_safe_boundary_does_not_shift_anchor_before_edit(self):
        # An anchor that PRECEDES a same-final-line edit must stay put — the
        # -1 sentinel previously matched ``offset >= -1`` and shifted it.
        src = "x = 1"  # single line, no newline
        rope = Rope.from_text(src)
        table = AnchorTable({0: AnchorBase(0, 0, 0)})  # anchor at the line start
        _new_rope, edit = rope.replace_with_edit(len(src), len(src), " + 2")
        boundary = _suffix_boundary(src, edit.old_end)
        assert boundary == -1
        rebased = table.rebased(edit, boundary_offset=boundary)
        assert rebased.get(0) == AnchorBase(0, 0, 0)  # unchanged, not shifted
