"""Tests for the folding range provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from lsp.features.folding import _normalise_overlaps, get_folding_ranges


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
        assert len(region_ranges) >= 1

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
        assert len(region_ranges) >= 1

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
