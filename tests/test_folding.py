"""Tests for the folding range provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from lsp.features.folding import get_folding_ranges


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
