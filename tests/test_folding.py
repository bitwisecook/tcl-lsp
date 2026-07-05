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

"""Tests for the folding range provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from server.features.folding import _normalise_overlaps, get_folding_ranges


class TestFoldingRanges:
    def test_proc_body(self):
        source = textwrap.dedent("""\
            proc greet {name} {
                puts "Hello"
                puts "$name"
            }
        """)
        ranges = get_folding_ranges(source)
        region_ranges = [r for r in ranges if r.kind == types.FoldingRangeKind.Region]
        assert len(region_ranges) >= 1
        # The proc body should fold starting at line 0
        body = region_ranges[0]
        assert body.start_line == 0
        assert body.end_line >= 2

    def test_namespace_body(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                proc helper {} { return }
            }
        """)
        ranges = get_folding_ranges(source)
        region_ranges = [r for r in ranges if r.kind == types.FoldingRangeKind.Region]
        # Should have at least the namespace fold
        start_lines = {r.start_line for r in region_ranges}
        assert 0 in start_lines  # namespace body

    def test_comment_block(self):
        source = textwrap.dedent("""\
            # This is a comment block
            # that spans multiple lines
            # explaining something important
            proc foo {} { return }
        """)
        ranges = get_folding_ranges(source)
        comment_ranges = [r for r in ranges if r.kind == types.FoldingRangeKind.Comment]
        assert len(comment_ranges) == 1
        assert comment_ranges[0].start_line == 0
        assert comment_ranges[0].end_line == 2

    def test_if_body(self):
        source = textwrap.dedent("""\
            if {1} {
                puts "yes"
                puts "really"
            }
        """)
        ranges = get_folding_ranges(source)
        region_ranges = [r for r in ranges if r.kind == types.FoldingRangeKind.Region]
        # The if-body spans from its opening line (0) to the line before the
        # closing brace (2), so a region fold must cover exactly that span.
        assert any(r.start_line == 0 and r.end_line == 2 for r in region_ranges), [
            (r.start_line, r.end_line) for r in region_ranges
        ]

    def test_single_line_no_fold(self):
        source = "proc foo {} { return 1 }\n"
        ranges = get_folding_ranges(source)
        # Single-line bodies should not create folding ranges
        region_ranges = [r for r in ranges if r.kind == types.FoldingRangeKind.Region]
        assert len(region_ranges) == 0

    def test_empty_file(self):
        ranges = get_folding_ranges("")
        assert ranges == []

    def test_single_comment_no_fold(self):
        source = "# Just one comment\n"
        ranges = get_folding_ranges(source)
        comment_ranges = [r for r in ranges if r.kind == types.FoldingRangeKind.Comment]
        assert len(comment_ranges) == 0

    def test_while_body(self):
        source = textwrap.dedent("""\
            while {1} {
                puts "loop"
                puts "again"
            }
        """)
        ranges = get_folding_ranges(source)
        region_ranges = [r for r in ranges if r.kind == types.FoldingRangeKind.Region]
        # The while-body folds from its opening line (0) to the line before
        # the closing brace (2).
        assert any(r.start_line == 0 and r.end_line == 2 for r in region_ranges), [
            (r.start_line, r.end_line) for r in region_ranges
        ]

    def test_if_else_bodies_are_disjoint(self):
        """Regression for #182: `} else {` must not put body1 and body2 on the same line."""
        source = textwrap.dedent("""\
            if {1} {
                puts "yes"
                puts "really"
            } else {
                puts "no"
                puts "nope"
            }
        """)
        ranges = get_folding_ranges(source)
        region_ranges = sorted(
            (r for r in ranges if r.kind == types.FoldingRangeKind.Region),
            key=lambda r: (r.start_line, r.end_line),
        )
        # Expect two sibling folds, one per branch, with no shared line.
        body_ranges = [r for r in region_ranges if r.start_line in (0, 3)]
        assert len(body_ranges) == 2
        first, second = body_ranges
        assert first.end_line < second.start_line, (
            f"body1 fold {first.start_line}..{first.end_line} overlaps "
            f"body2 fold {second.start_line}..{second.end_line}"
        )

    def test_nested_if_else_no_overlapping_siblings(self):
        """Deeply nested if/else with ``} else {`` on the same line stays well-formed."""
        source = textwrap.dedent("""\
            proc demo {x} {
                if {$x} {
                    if {$x > 1} {
                        puts "big"
                        puts "really big"
                    } else {
                        puts "small"
                        puts "really small"
                    }
                } else {
                    puts "zero"
                    puts "none"
                }
            }
        """)
        ranges = get_folding_ranges(source)

        def contains(outer, inner):
            return outer.start_line <= inner.start_line and inner.end_line <= outer.end_line

        for i, a in enumerate(ranges):
            for b in ranges[i + 1 :]:
                if a.start_line == b.start_line and a.end_line == b.end_line:
                    continue
                if contains(a, b) or contains(b, a):
                    continue
                overlaps = a.end_line >= b.start_line and a.start_line <= b.end_line
                assert not overlaps, (
                    f"non-nested overlap between {a.start_line}..{a.end_line} "
                    f"and {b.start_line}..{b.end_line}"
                )

    def test_if_else_proc_body_close_brace_visible(self):
        """Proc fold end should leave the outer closing ``}`` line visible."""
        source = textwrap.dedent("""\
            proc demo {} {
                if {1} {
                    puts "yes"
                } else {
                    puts "no"
                }
            }
        """)
        ranges = get_folding_ranges(source)
        proc_folds = [
            r for r in ranges if r.kind == types.FoldingRangeKind.Region and r.start_line == 0
        ]
        assert proc_folds
        # The proc's own closing ``}`` is the last line that is exactly ``}``
        # at column 0 — the inner ``if``/``else`` braces are indented.
        lines = source.split("\n")
        close_idx = max(i for i, line in enumerate(lines) if line == "}")
        for r in proc_folds:
            assert r.end_line < close_idx, (
                f"proc fold end {r.end_line} hides closing brace on line {close_idx}"
            )

    def test_incomplete_body_still_folds(self):
        """Unterminated braced body (user mid-edit) must still produce a fold."""
        # No closing ``}`` on the proc body.
        source = "proc foo {} {\n    puts hi\n    puts there\n"
        ranges = get_folding_ranges(source)
        region_ranges = [r for r in ranges if r.kind == types.FoldingRangeKind.Region]
        # The partial body should still be foldable so folding doesn't flicker
        # off every time the user deletes the trailing brace.
        assert region_ranges, "expected a fold range for an unterminated proc body"
        # Start line should be the ``{`` line.
        assert any(r.start_line == 0 for r in region_ranges)

    def test_elseif_chain_disjoint(self):
        """``if/elseif/elseif/else`` yields four disjoint sibling folds."""
        source = textwrap.dedent("""\
            if {1} {
                puts a
            } elseif {2} {
                puts b
            } elseif {3} {
                puts c
            } else {
                puts d
            }
        """)
        ranges = get_folding_ranges(source)
        region_ranges = sorted(
            (r for r in ranges if r.kind == types.FoldingRangeKind.Region),
            key=lambda r: (r.start_line, r.end_line),
        )
        # One fold per branch, each spanning two lines; must be pairwise disjoint.
        assert len(region_ranges) == 4, region_ranges
        for a, b in zip(region_ranges, region_ranges[1:]):
            assert a.end_line < b.start_line, (
                f"elseif sibling folds {a.start_line}..{a.end_line} and "
                f"{b.start_line}..{b.end_line} still share a line"
            )

    def test_normalise_overlaps_shared_boundary_trims_earlier(self):
        """Two sibling ranges sharing a boundary line must become disjoint."""
        ranges = [
            types.FoldingRange(
                start_line=0,
                end_line=5,
                kind=types.FoldingRangeKind.Region,
            ),
            types.FoldingRange(
                start_line=5,
                end_line=10,
                kind=types.FoldingRangeKind.Region,
            ),
        ]
        normalised = _normalise_overlaps(ranges)
        # Both ranges should survive, and they must not share any line.
        assert len(normalised) == 2
        a, b = sorted(normalised, key=lambda r: r.start_line)
        assert a.end_line < b.start_line, f"{a} and {b} still overlap"

    def test_normalise_overlaps_dedups_after_trimming(self):
        """Trimming must not leave duplicate ``(start, end, kind)`` entries."""
        # Parent [0, 10] with child [3, 8]; a sibling [3, 12] trims to [3, 10]
        # and another collector emitting [3, 10] natively would otherwise
        # survive as a duplicate.
        ranges = [
            types.FoldingRange(
                start_line=0,
                end_line=10,
                kind=types.FoldingRangeKind.Region,
            ),
            types.FoldingRange(
                start_line=3,
                end_line=10,
                kind=types.FoldingRangeKind.Region,
            ),
            types.FoldingRange(
                start_line=3,
                end_line=12,
                kind=types.FoldingRangeKind.Region,
            ),
        ]
        normalised = _normalise_overlaps(ranges)
        keys = [(r.start_line, r.end_line, r.kind) for r in normalised]
        assert len(keys) == len(set(keys)), f"duplicates in {keys}"


class TestLineContinuationFolds:
    """Backslash line-continuation folding (issue #493).

    A command stretched across physical lines by trailing ``\\`` joins is a
    single logical command; the provider folds the run down to its opening
    line.  Originally added in #494; the feature and its tests were dropped by
    the #498 folding refactor and are restored here with the failure-mode
    (fix-holds) cases *and* the semantically-similar-but-correct cases that
    must **not** fold (an escaped ``\\\\`` is a literal backslash, not a
    continuation; a dangling ``\\`` at EOF joins nothing).
    """

    @staticmethod
    def _regions(source: str) -> set[tuple[int, int]]:
        return {
            (r.start_line, r.end_line)
            for r in get_folding_ranges(source)
            if r.kind == types.FoldingRangeKind.Region
        }

    @staticmethod
    def _spans(source: str) -> set[tuple[int, int]]:
        return {(r.start_line, r.end_line) for r in get_folding_ranges(source)}

    # ── fix holds: continuations fold ────────────────────────────────────

    def test_line_continuation_fold(self):
        """A backslash-continued command folds to its opening line."""
        source = textwrap.dedent("""\
            MyProcCall $var1 \\
                       $var2 \\
                       $var3
        """)
        assert (0, 2) in self._regions(source), self._regions(source)

    def test_line_continuation_two_lines(self):
        """A single continuation still produces a fold over both lines."""
        assert (0, 1) in self._regions("set x $a \\\n    $b\n")

    def test_crlf_line_continuation_fold(self):
        """CRLF (Windows) buffers fold too: the join tokenises as ``\\\r\n``."""
        assert (0, 2) in self._regions("MyProcCall $a \\\r\n   $b \\\r\n   $c\r\n")

    def test_separate_continuation_runs(self):
        """Two distinct continued commands yield two separate folds."""
        source = "foo $a \\\n   $b\nputs sep\nbar $c \\\n   $d\n"
        regions = self._regions(source)
        assert {(0, 1), (3, 4)} <= regions, regions

    def test_continuation_inside_body_folds(self):
        """A continued command inside a braced body folds, not just top-level."""
        source = textwrap.dedent("""\
            proc foo {} {
                MyProcCall $a \\
                           $b \\
                           $c
            }
        """)
        spans = self._spans(source)
        assert (0, 3) in spans, spans  # the proc body
        assert (1, 3) in spans, spans  # the continued command inside it

    def test_continuation_inside_nested_body_folds(self):
        """Continuations fold even when nested several blocks deep."""
        source = textwrap.dedent("""\
            proc p {} {
                if {1} {
                    cmd $a \\
                        $b
                }
            }
        """)
        assert (2, 3) in self._spans(source), self._spans(source)

    # ── known-incorrect look-alikes: must NOT fold ───────────────────────

    def test_escaped_backslash_is_not_a_continuation(self):
        r"""A literal ``\\`` at end of line is an escaped backslash, not a line
        continuation (matches tclsh: the two lines are separate commands)."""
        assert self._regions("puts foo\\\\\nputs bar\n") == set()

    def test_dangling_continuation_at_eof_no_degenerate_fold(self):
        """A trailing ``\\`` with nothing after it must not fold empty space."""
        assert get_folding_ranges("foo a \\\n") == []
