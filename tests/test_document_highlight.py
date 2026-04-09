"""Tests for the document-highlight provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from core.analysis.analyser import analyse
from lsp.features.document_highlight import get_document_highlights


def _highlights(source: str, line: int, character: int, uri: str = "file:///test.tcl"):
    """Helper: run get_document_highlights with a fresh analysis."""
    return get_document_highlights(source, uri, line, character, analysis=analyse(source))


TEST_URI = "file:///test.tcl"
OTHER_URI = "file:///other.tcl"


class TestDocumentHighlightProc:
    def test_highlights_proc_definition_and_calls(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
            greet Everyone
        """)
        highlights = get_document_highlights(source, TEST_URI, 0, 6)
        assert len(highlights) >= 2
        assert all(isinstance(h, types.DocumentHighlight) for h in highlights)
        assert all(h.kind == types.DocumentHighlightKind.Text for h in highlights)

    def test_highlights_include_declaration(self):
        source = textwrap.dedent("""\
            proc greet {} { return }
            greet
            greet
        """)
        highlights = get_document_highlights(source, TEST_URI, 0, 6)
        lines = {h.range.start.line for h in highlights}
        # Declaration (line 0) plus two call sites
        assert 0 in lines
        assert 1 in lines
        assert 2 in lines

    def test_cursor_on_whitespace_returns_empty(self):
        source = "proc greet {} { return }\n"
        highlights = get_document_highlights(source, TEST_URI, 0, 0)
        assert highlights == []

    def test_no_duplicate_ranges(self):
        source = textwrap.dedent("""\
            proc greet {} { return }
            greet
        """)
        highlights = get_document_highlights(source, TEST_URI, 0, 6)
        keys = [
            (
                h.range.start.line,
                h.range.start.character,
                h.range.end.line,
                h.range.end.character,
            )
            for h in highlights
        ]
        assert len(keys) == len(set(keys))


class TestDocumentHighlightVariable:
    def test_highlights_variable_uses(self):
        source = textwrap.dedent("""\
            proc sum {} {
                set x 1
                set y [expr {$x + 2}]
                return $x
            }
        """)
        # Cursor on `$x` in `return $x` (line 3).
        highlights = _highlights(source, 3, 12)
        assert len(highlights) >= 1
        # All highlights are in-file.
        assert all(h.range.start.line >= 1 for h in highlights)

    def test_write_kind_on_definition_read_kind_on_use(self):
        """Definition site is marked Write; reference sites are Read."""
        source = textwrap.dedent("""\
            proc p {} {
                set x 1
                return $x
            }
        """)
        # Cursor on `$x` at line 2.
        highlights = _highlights(source, 2, 12)
        # Expect at least one Write (the `set x`) and one Read (the `$x`).
        kinds = {h.kind for h in highlights}
        assert types.DocumentHighlightKind.Write in kinds, (
            f"Expected Write kind for `set x`, got {kinds}"
        )
        assert types.DocumentHighlightKind.Read in kinds, (
            f"Expected Read kind for `$x` reference, got {kinds}"
        )

    def test_scope_isolation_between_global_and_local(self):
        """A local `x` inside a proc must not highlight the global `x`.

        Regression guard: the analyser's scope tree is how we distinguish
        shadowed names; this test locks in that we use it.
        """
        source = textwrap.dedent("""\
            set x 1
            proc uses_x {} {
                set x 2
                puts $x
            }
            puts $x
        """)
        # Cursor on `$x` inside `uses_x` at line 3 (the `puts $x`).
        highlights = _highlights(source, 3, 10)
        # All highlights should be lines 2 (set x) and 3 (puts $x) —
        # the global x at line 0 and the outer puts at line 5 are in a
        # different scope.
        lines = sorted({h.range.start.line for h in highlights})
        assert 0 not in lines, f"Global `x` at line 0 should not leak into local scope, got {lines}"
        assert 5 not in lines, f"Outer `$x` at line 5 should not leak into local scope, got {lines}"
        # The local set and read must both be present.
        assert 2 in lines and 3 in lines
