"""Dialect awareness of ``{*}`` argument expansion.

``{*}`` word expansion was introduced in Tcl 8.5.  In Tcl 8.4 — and therefore
in the 8.4-based F5 iRules / iApps dialects — ``{*}`` is **not** special: it is
an ordinary brace-quoted word.  These tests pin that behaviour at both the
segmenter (unit) and analyser (integration) levels so a dialect regression
cannot silently treat ``{*}`` as expansion where it must be inert.
"""

from __future__ import annotations

import pytest

from core.common.dialect import dialect_scope
from core.parsing.command_segmenter import segment_commands

# {*} is special from Tcl 8.5 on (and in 8.5-based dialects such as f5-iapps);
# it is inert in Tcl 8.4 and the 8.4-based F5 iRules dialect.
_EXPANDS = ["tcl8.5", "tcl8.6", "tcl9.0", "f5-iapps"]
_INERT = ["tcl8.4", "f5-irules"]


def _first_command(source: str, dialect: str):
    with dialect_scope(dialect):
        return segment_commands(source)[0]


class TestExpansionIsDialectGated:
    @pytest.mark.parametrize("dialect", _EXPANDS)
    def test_expansion_recognised_in_85_plus(self, dialect):
        cmd = _first_command("lappend l {*}$args\n", dialect)
        assert cmd.expand_word is not None
        # The third word ($args) carries the expansion marker.
        assert cmd.expand_word == [False, False, True]
        assert cmd.texts == ["lappend", "l", "${args}"]

    @pytest.mark.parametrize("dialect", _INERT)
    def test_expansion_inert_in_84_and_irules(self, dialect):
        cmd = _first_command("lappend l {*}$args\n", dialect)
        # No expansion markers at all — ``{*}`` is a literal brace word.
        assert cmd.expand_word is None
        # ``{*}$args`` parses as the literal ``*`` concatenated with $args,
        # NOT as an expansion of $args into multiple words.
        assert cmd.texts == ["lappend", "l", "*${args}"]

    @pytest.mark.parametrize("dialect", _INERT)
    def test_command_word_expansion_inert(self, dialect):
        # ``{*}$cmd args`` would, under 8.5+, expand the command word; in 8.4
        # it must stay a single literal-prefixed word.
        cmd = _first_command("{*}$cmd a b\n", dialect)
        assert cmd.expand_word is None


class TestExpansionAffectsArity:
    """Integration: arity checking must treat expansion as unknown count only
    where the dialect supports it."""

    @pytest.mark.parametrize("dialect", _EXPANDS)
    def test_expanded_args_suppress_arity_error(self, dialect):
        from lsp.features.diagnostics import get_diagnostics

        with dialect_scope(dialect):
            # ``incr`` takes 1-2 args; the expanded word is an unknown count,
            # so no arity error must fire.
            codes = {
                (d.code if isinstance(d.code, str) else str(d.code))
                for d in get_diagnostics("incr {*}$pair\n")
            }
        assert "E002" not in codes
        assert "E003" not in codes

    @pytest.mark.parametrize("dialect", _INERT)
    def test_literal_brace_word_counts_as_one_arg(self, dialect):
        from lsp.features.diagnostics import get_diagnostics

        with dialect_scope(dialect):
            # Here ``{*}$pair`` is a single literal word, so ``incr`` sees
            # exactly one argument — which is valid (1-2 args).
            codes = {
                (d.code if isinstance(d.code, str) else str(d.code))
                for d in get_diagnostics("incr {*}$pair\n")
            }
        assert "E002" not in codes
