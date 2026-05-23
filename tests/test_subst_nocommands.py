"""Tests for ``compiler.parsing.subst_nocommands.subst_nocommands``.

Hand-calibrated against the ``tclsh 9.0`` ``subst -nocommands``
output for each fixture.  The lowering pass (P7.3) uses this to
materialise ``[subst -nocommands {body}]`` templates at compile
time when every ``$var`` reference is const-known.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.subst_nocommands import subst_nocommands


class TestSubstNocommandsPasses:
    """Shapes where every reference is resolvable — the evaluator
    returns the substituted string."""

    def test_plain_text_passes_through(self):
        assert subst_nocommands("hello world", {}) == "hello world"

    def test_bare_var_substitutes(self):
        assert subst_nocommands("$name", {"name": "foo"}) == "foo"

    def test_braced_var_substitutes(self):
        assert subst_nocommands("${name}", {"name": "bar"}) == "bar"

    def test_multiple_vars(self):
        out = subst_nocommands("$a-$b/$c", {"a": "1", "b": "2", "c": "3"})
        assert out == "1-2/3"

    def test_mixed_bare_and_braced(self):
        out = subst_nocommands("$a${b}$c", {"a": "X", "b": "Y", "c": "Z"})
        assert out == "XYZ"

    def test_backslash_n_decodes_to_newline(self):
        out = subst_nocommands(r"line1\nline2", {})
        assert out == "line1\nline2"

    def test_backslash_t_decodes_to_tab(self):
        out = subst_nocommands(r"col1\tcol2", {})
        assert out == "col1\tcol2"

    def test_backslash_hex_escape(self):
        out = subst_nocommands(r"\x41\x42", {})
        assert out == "AB"

    def test_backslash_unicode_escape(self):
        out = subst_nocommands(r"\u00e9", {})
        assert out == "é"

    def test_backslash_dollar_escapes_substitution(self):
        # ``\$name`` → literal ``$name`` (no substitution).
        assert subst_nocommands(r"\$name", {"name": "X"}) == "$name"

    def test_backslash_bracket_keeps_bracket_literal(self):
        assert subst_nocommands(r"\[cmd\]", {}) == "[cmd]"

    def test_stray_dollar_then_nonname(self):
        # ``$$`` — first ``$`` is literal (no name follows), second
        # ``$`` starts ``$name``.
        assert subst_nocommands("$$name", {"name": "V"}) == "$V"

    def test_dollar_then_paren_is_literal(self):
        # ``$(`` — no name char follows, ``$`` stays literal.
        assert subst_nocommands("$(", {}) == "$("

    def test_bracket_run_passes_through_literally(self):
        # ``-nocommands`` means ``[…]`` is NOT evaluated — it's
        # kept verbatim in the output, including any ``$var``s
        # inside.  That's the defining feature of the flag.
        out = subst_nocommands("[puts $name]", {"name": "X"})
        assert out == "[puts $name]"

    def test_nested_brackets_balanced(self):
        out = subst_nocommands("[a [b c] d]", {})
        assert out == "[a [b c] d]"

    def test_closing_bracket_without_open_is_literal(self):
        assert subst_nocommands("trailing]", {}) == "trailing]"

    def test_tcltest_option_factory_shape(self):
        # The exact template shape from tcltest's ``Option`` factory
        # (simplified): declares an Option(...) var and sets its
        # default, plus defines accessor bodies that reference
        # ``$name`` / ``$default`` / ``$description``.
        template = (
            "variable Option($name) $default\nvariable OptionDesc($name) [list $description]\n"
        )
        const_map = {"name": "verbose", "default": "0", "description": "V flag"}
        out = subst_nocommands(template, const_map)
        assert out == (
            "variable Option(verbose) 0\nvariable OptionDesc(verbose) [list $description]\n"
        )


class TestSubstNocommandsRefuses:
    """Shapes the evaluator refuses by returning ``None`` — the
    caller falls back to runtime dispatch in these cases."""

    def test_missing_var(self):
        assert subst_nocommands("$missing", {}) is None

    def test_missing_braced_var(self):
        assert subst_nocommands("${missing}", {}) is None

    def test_array_reference(self):
        assert subst_nocommands("$a(b)", {"a": "X"}) is None

    def test_namespace_qualified_var(self):
        assert subst_nocommands("$foo::bar", {"foo": "X"}) is None

    def test_leading_double_colon_var_refused(self):
        # ``$::name`` is a Tcl namespace-qualified var reference
        # that real ``subst -nocommands`` would substitute at
        # runtime.  Our const-map doesn't carry qualified entries,
        # so refuse rather than emit the ``$`` as literal text
        # (which would silently diverge from tclsh semantics).
        assert subst_nocommands("$::name", {"name": "X"}) is None
        assert subst_nocommands("prefix $::foo suffix", {"foo": "X"}) is None

    def test_unclosed_braced_var(self):
        assert subst_nocommands("${name", {"name": "X"}) is None

    def test_unbalanced_open_bracket(self):
        assert subst_nocommands("[unclosed", {}) is None

    def test_braced_var_with_nested_dollar_refused(self):
        # ``${na$me}`` has a ``$`` inside the braces — our
        # evaluator refuses rather than decoding recursively.
        assert subst_nocommands("${na$me}", {"na$me": "X"}) is None

    def test_empty_braced_var(self):
        # ``${}`` is pathological — empty name never appears in any
        # real const-map.  Reject on refuse rather than treat as a
        # lookup miss.
        assert subst_nocommands("${}", {}) is None
