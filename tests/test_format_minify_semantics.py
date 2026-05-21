"""Semantic-preservation tests for format and minify.

``test_formatter.py`` already covers formatting idempotency and ``test_minifier``
covers content preservation.  These add the cross-checks that prove the
transforms only change *layout*, never the meaning:

- formatting must not alter tokens — ``minify(format(x)) == minify(x)``;
- minifying must keep the same command structure (same command names in the
  same order, same word counts), ignoring stripped comments.
"""

from __future__ import annotations

import pytest

from compiler.parsing.command_segmenter import segment_commands
from tooling.formatter import format_tcl
from tooling.minifier import minify_tcl

_CORPUS = [
    "proc add {a b} {return [expr {$a+$b}]}\n",
    "if {$x==1} {puts hi} else {puts bye}\n",
    "set   x    42\nputs $x\n",
    "foreach i $list {\n  puts $i\n}\n",
    'set msg "hello $name"\nputs $msg\n',
    "namespace eval ::ns {\n proc foo {} {puts hi}\n}\n",
    "switch $x {\n a {puts a}\n b {puts b}\n default {puts c}\n}\n",
    "# leading comment\nset x 1\nset y 2\nputs $x$y\n",
]


def _command_signature(source: str) -> list[tuple[str, int]]:
    """(command name, word count) for each non-empty command."""
    return [(c.texts[0], len(c.texts)) for c in segment_commands(source) if c.texts]


class TestFormatPreservesSemantics:
    @pytest.mark.parametrize("source", _CORPUS)
    def test_format_changes_only_layout(self, source):
        # Minifying collapses both forms to the same canonical token stream,
        # so any token-level change introduced by formatting would show up
        # as a difference here.
        assert minify_tcl(format_tcl(source)) == minify_tcl(source), (
            f"formatting altered tokens for {source!r}"
        )


class TestMinifyPreservesStructure:
    @pytest.mark.parametrize("source", _CORPUS)
    def test_command_structure_is_preserved(self, source):
        # Comments are stripped, so drop comment-only commands from the
        # original signature before comparing.
        original = _command_signature(source)
        minified = _command_signature(minify_tcl(source))
        assert minified == original, (
            f"minify changed command structure for {source!r}:\n"
            f"  before: {original}\n  after:  {minified}"
        )
