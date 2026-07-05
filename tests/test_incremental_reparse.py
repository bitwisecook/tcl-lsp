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

"""Property + unit tests for incremental top-level reparse (#477 phase 3-4).

The contract: when ``incremental_top_level_chunks`` returns a chunk list, it
must be byte-identical to a from-scratch ``segment_top_level_chunks`` of the
new source.  When it cannot prove equivalence it returns ``None`` and the
caller falls back to full segmentation.
"""

from __future__ import annotations

import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.command_segmenter import (
    SegmentedCommand,
    TopLevelChunk,
    segment_top_level_chunks,
)
from compiler.parsing.incremental import (
    EditRange,
    incremental_top_level_chunks,
    infer_edit_range,
)

# --- edit-range inference ----------------------------------------------------


class TestInferEditRange:
    def test_identical_is_none(self):
        assert infer_edit_range("abc", "abc") is None

    def test_pure_insertion(self):
        e = infer_edit_range("abcd", "abXYcd")
        assert e is not None
        assert e == EditRange(start=2, old_end=2, new_end=4)
        assert e.offset_delta == 2

    def test_pure_deletion(self):
        e = infer_edit_range("abXYcd", "abcd")
        assert e is not None
        assert e == EditRange(start=2, old_end=4, new_end=2)
        assert e.offset_delta == -2

    def test_replacement(self):
        e = infer_edit_range("abXcd", "abYZcd")
        assert e == EditRange(start=2, old_end=3, new_end=4)

    def test_append(self):
        e = infer_edit_range("abc", "abcdef")
        assert e == EditRange(start=3, old_end=3, new_end=6)

    def test_prepend(self):
        e = infer_edit_range("def", "abcdef")
        assert e == EditRange(start=0, old_end=0, new_end=3)

    def test_round_trips(self):
        for old, new in [
            ("hello world", "hello cruel world"),
            ("aaa", "aa"),
            ("", "x"),
            ("x", ""),
            ("line1\nline2\n", "line1\nLINE2\n"),
        ]:
            e = infer_edit_range(old, new)
            assert e is not None
            assert old[: e.start] == new[: e.start]
            assert old[e.old_end :] == new[e.new_end :]
            assert new == old[: e.start] + new[e.start : e.new_end] + old[e.old_end :]

    def test_edits_across_scan_block_boundary(self):
        # Common prefix/suffix far larger than the scan block: the block-wise
        # scan must land on the exact divergence regardless of where it falls
        # relative to a block boundary.
        from compiler.parsing.incremental import _SCAN_BLOCK

        pad = "a" * (_SCAN_BLOCK * 2 + 7)
        for k in (0, 1, _SCAN_BLOCK - 1, _SCAN_BLOCK, _SCAN_BLOCK + 1, len(pad)):
            old = pad
            new = pad[:k] + "Z" + pad[k:]  # single-char insertion at offset k
            e = infer_edit_range(old, new)
            assert e is not None
            assert e == EditRange(start=k, old_end=k, new_end=k + 1)
            assert new == old[: e.start] + new[e.start : e.new_end] + old[e.old_end :]


class TestCommonPrefixSuffixLen:
    """The block-wise prefix/suffix scans must equal a naive char-by-char
    reference across boundaries — the inputs run on every keystroke, so the
    low-allocation rewrite must preserve exact semantics."""

    @staticmethod
    def _ref_prefix(a, b, hi):
        k = 0
        while k < hi and a[k] == b[k]:
            k += 1
        return k

    @staticmethod
    def _ref_suffix(a, b, hi):
        na, nb, k = len(a), len(b), 0
        while k < hi and a[na - 1 - k] == b[nb - 1 - k]:
            k += 1
        return k

    def test_matches_reference_random_and_boundaries(self):
        import random

        from compiler.parsing.incremental import (
            _SCAN_BLOCK,
            _common_prefix_len,
            _common_suffix_len,
        )

        base = "a" * (_SCAN_BLOCK * 2 + 5)
        cases = [
            ("", ""),
            ("a", ""),
            ("", "a"),
            ("abc", "abc"),
            ("abc", "abd"),
            (base, base),
            (base, base[:-1] + "b"),
            ("z" + base, "z" + base[:-3] + "qqq"),
            (base[:_SCAN_BLOCK] + "X" + base, base[:_SCAN_BLOCK] + "Y" + base),
        ]
        rng = random.Random(1234)
        for _ in range(1500):
            a = "".join(rng.choice("ab") for _ in range(rng.randint(0, 40)))
            b = "".join(rng.choice("ab") for _ in range(rng.randint(0, 40)))
            cases.append((a, b))
        for a, b in cases:
            hi = min(len(a), len(b))
            assert _common_prefix_len(a, b, hi) == self._ref_prefix(a, b, hi), (a[:20], b[:20])
            assert _common_suffix_len(a, b, hi) == self._ref_suffix(a, b, hi), (a[:20], b[:20])


# --- chunk equivalence helpers ----------------------------------------------


def _tok_tuple(t):
    return (
        t.type,
        t.text,
        t.start.offset,
        t.start.line,
        t.start.character,
        t.end.offset,
        t.end.line,
        t.end.character,
    )


def _cmd_key(c: SegmentedCommand):
    return (
        c.range.start.offset,
        c.range.start.line,
        c.range.start.character,
        c.range.end.offset,
        c.range.end.line,
        c.range.end.character,
        tuple(c.texts),
        tuple(c.single_token_word),
        tuple(_tok_tuple(t) for t in c.argv),
        tuple(_tok_tuple(t) for t in c.all_tokens),
        c.is_partial,
        c.partial_delimiter,
        tuple(c.expand_word) if c.expand_word else None,
        c.subcommand,
        c.preceding_comment,
    )


def _chunk_key(ch: TopLevelChunk):
    return (
        ch.index,
        ch.start_offset,
        ch.end_offset,
        ch.source_hash,
        tuple(_cmd_key(c) for c in ch.commands),
    )


def assert_chunks_equal(a: list[TopLevelChunk], b: list[TopLevelChunk]) -> None:
    assert len(a) == len(b), f"chunk count {len(a)} != {len(b)}"
    for i, (x, y) in enumerate(zip(a, b)):
        assert _chunk_key(x) == _chunk_key(y), (
            f"chunk {i} differs:\n{_chunk_key(x)}\n!=\n{_chunk_key(y)}"
        )


# --- targeted unit cases ----------------------------------------------------


_BASE = """\
set greeting "hello"
proc add {a b} {
    return [expr {$a + $b}]
}
puts [add 1 2]
set xs {1 2 3 4}
foreach x $xs {
    puts $x
}
"""


def _check(old: str, new: str):
    old_chunks = segment_top_level_chunks(old)
    edit = infer_edit_range(old, new)
    if edit is None:
        return
    inc = incremental_top_level_chunks(old, old_chunks, new, edit)
    full = segment_top_level_chunks(new)
    if inc is not None:
        assert_chunks_equal(inc, full)


class TestIncrementalUnit:
    def test_edit_in_middle_command(self):
        old = _BASE
        new = old.replace("puts $x", "puts $x.value")
        _check(old, new)

    def test_edit_first_command(self):
        old = _BASE
        new = old.replace('"hello"', '"hi there"')
        _check(old, new)

    def test_insert_whole_command_midfile(self):
        old = _BASE
        new = old.replace("puts [add 1 2]\n", "puts [add 1 2]\nset extra 99\n")
        _check(old, new)

    def test_delete_command_midfile(self):
        old = _BASE
        new = old.replace("set xs {1 2 3 4}\n", "")
        _check(old, new)

    def test_edit_last_line_returns_none_or_matches(self):
        old = "set a 1\nset b 2"
        new = "set a 1\nset b 22"
        _check(old, new)  # last line, no clean suffix → likely None, still safe

    def test_no_old_chunks(self):
        assert incremental_top_level_chunks("", [], "set a 1", EditRange(0, 0, 7)) is None


class TestChunkHashStaleness:
    """A change to a command's final character must change its chunk hash,
    or dirty-chunk detection silently reuses stale analysis (#480 review)."""

    def test_last_char_change_changes_hash(self):
        a = segment_top_level_chunks("set a 1\nset b 2\n")
        b = segment_top_level_chunks("set a 1\nset b 3\n")
        # chunk 0 (set a 1) unchanged; chunk 1 (set b 2 -> set b 3) must differ
        assert a[0].source_hash == b[0].source_hash
        assert a[1].source_hash != b[1].source_hash

    def test_trailing_command_last_char(self):
        a = segment_top_level_chunks("puts hi\nputs lo")
        b = segment_top_level_chunks("puts hi\nputs lX")
        assert a[1].source_hash != b[1].source_hash

    def test_proc_cache_key_detects_bare_body_last_char(self):
        # A bare-word proc body whose last char changes (same length) must
        # produce a different proc cache key.
        from compiler.interprocedural import _cache_key_for_proc
        from compiler.lowering import lower_to_ir

        src_a = "proc f {} abc\n"
        src_b = "proc f {} abd\n"
        ma = lower_to_ir(src_a)
        mb = lower_to_ir(src_b)
        pa = next(iter(ma.procedures.values()))
        pb = next(iter(mb.procedures.values()))
        ka = _cache_key_for_proc(src_a, "::f", pa)
        kb = _cache_key_for_proc(src_b, "::f", pb)
        assert ka is not None and kb is not None
        assert ka != kb


# --- randomised property test -----------------------------------------------


_SNIPPETS = [
    "set a 1\n",
    'set msg "hello world"\n',
    "proc foo {x y} {\n    return [expr {$x + $y}]\n}\n",
    "if {$a > 0} {\n    puts yes\n} else {\n    puts no\n}\n",
    "foreach item $list {\n    puts $item\n}\n",
    "while {$i < 10} {\n    incr i\n}\n",
    "set d [dict create a 1 b 2]\n",
    "# a comment\n",
    "lappend xs 1 2 3\n",
    "puts [string length $s]\n",
    "\n",
    "namespace eval ns {\n    variable v 0\n}\n",
    # Escaped braces: a random edit landing adjacent to a ``\\{``/``\\}`` can
    # flip brace nesting, exercising the brace-fast-path's escape-adjacency
    # guard (incremental must still equal full).
    "set lit \\{\n",
    "set lit \\}\n",
    "namespace eval ns {\n    set lit \\{\n    set y 2\n}\n",
]


def _random_source(rng: random.Random) -> str:
    return "".join(rng.choice(_SNIPPETS) for _ in range(rng.randint(4, 16)))


def _random_edit(rng: random.Random, src: str) -> str:
    if not src:
        return rng.choice(_SNIPPETS)
    kind = rng.randint(0, 3)
    n = len(src)
    i = rng.randint(0, n)
    if kind == 0:  # insert
        ins = rng.choice(["x", " ", "\n", "$y", "set z 5\n", "puts hi\n", "}", "{", '"'])
        return src[:i] + ins + src[i:]
    if kind == 1:  # delete a span
        j = min(n, i + rng.randint(1, 8))
        return src[:i] + src[j:]
    if kind == 2:  # replace a span
        j = min(n, i + rng.randint(1, 6))
        rep = rng.choice(["AAA", "set q 1\n", " ", "\n"])
        return src[:i] + rep + src[j:]
    # whole-line-ish insertion at a newline boundary
    nls = [k for k, c in enumerate(src) if c == "\n"]
    if not nls:
        return src + rng.choice(_SNIPPETS)
    at = rng.choice(nls) + 1
    return src[:at] + rng.choice(_SNIPPETS) + src[at:]


class TestIncrementalProperty:
    def test_incremental_equals_full_randomised(self):
        rng = random.Random(0xC0FFEE)
        checked = 0
        matched = 0
        for _ in range(4000):
            old = _random_source(rng)
            new = _random_edit(rng, old)
            edit = infer_edit_range(old, new)
            if edit is None:
                continue
            # Verify the inferred edit actually reconstructs new.
            assert new == old[: edit.start] + new[edit.start : edit.new_end] + old[edit.old_end :]
            old_chunks = segment_top_level_chunks(old)
            inc = incremental_top_level_chunks(old, old_chunks, new, edit)
            full = segment_top_level_chunks(new)
            checked += 1
            if inc is not None:
                matched += 1
                assert_chunks_equal(inc, full)
        # Sanity: the fast path must actually fire on a meaningful number of
        # cases, otherwise the test isn't exercising it.  (It bails whenever
        # there is no clean multi-chunk suffix below the edit's last line, so
        # the rate is well under 100% on randomised edits — correctness, not
        # fire rate, is the contract.)
        assert checked > 1000
        assert matched > checked // 3


class TestIncrementalRecovery:
    """When the old segmentation involved error recovery (a partial command),
    the fast path must bail — the window is re-segmented with recovery disabled
    and cannot reproduce a recovered command, so reusing it would diverge from a
    from-scratch pass (which re-runs recovery).  Regression for the #533 fuzz
    finding: a partial command sitting in the *window* region (between the prefix
    and the validation chunk) used to slip past the per-region partial checks.
    """

    def test_old_partial_command_bails(self):
        # An unclosed leading ``{`` makes command 0 partial via recovery; the
        # recovered ``puts`` command follows.  Any edit must take the full
        # fallback (return None) rather than a wrong incremental result.
        old = '{[x\n{*}"{}\n  [{*}puts hi\n]\nputs hi  {\n{*}\n }  '
        new = '{[x\n{*}"{}\n  [{*}puts hi\n]\nputs hi  {\n{*}[\n }  '
        old_chunks = segment_top_level_chunks(old)
        assert any(c.is_partial for ch in old_chunks for c in ch.commands)
        edit = infer_edit_range(old, new)
        assert edit is not None
        assert incremental_top_level_chunks(old, old_chunks, new, edit) is None

    def test_recovery_inputs_never_diverge(self):
        # Property check over heavily-unbalanced (recovery-triggering) sources:
        # whenever the fast path returns non-None it must equal a full pass.
        atoms = [
            "{",
            "}",
            "[",
            "]",
            '"',
            "$v",
            "puts hi",
            "{*}",
            "set a 1",
            "a b",
            ";",
            "\n",
            " ",
            "x",
        ]
        rng = random.Random(0x5EED)
        partial_seen = 0
        for _ in range(6000):
            old = "".join(rng.choice(atoms) for _ in range(rng.randint(0, 24)))
            i = rng.randint(0, len(old))
            ins = rng.choice(["{", "}", "[", "x", " ", "\n", '"', ";"])
            new = old[:i] + ins + old[i:]
            old_chunks = segment_top_level_chunks(old)
            if any(c.is_partial for ch in old_chunks for c in ch.commands):
                partial_seen += 1
            edit = infer_edit_range(old, new)
            if edit is None:
                continue
            inc = incremental_top_level_chunks(old, old_chunks, new, edit)
            if inc is not None:
                assert_chunks_equal(inc, segment_top_level_chunks(new))
        assert partial_seen > 50  # the corpus really does exercise recovery
