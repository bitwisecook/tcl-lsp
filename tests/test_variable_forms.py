"""All-forms variable resolution tests (find-references / rename surface).

Tcl variables appear in several syntactic forms — bare ``$name``, braced
``${name}``, array ``$arr(key)``, global ``$::name``, and namespace-qualified
``$ns::name`` / ``$::a::b::c``.  Anything that resolves a variable must treat
every form as the same symbol.  These tests exercise the find-references
provider (which backs go-to-references and rename) across the forms.

Known gaps tracked separately (not asserted here):
- bareword command-argument uses (``incr count``, ``set count``) are not yet
  recorded as references;
- relative namespace-qualified reads (``$ns::v``) do not yet resolve to a
  ``namespace eval ns { variable v }`` definition.
"""

from __future__ import annotations

from lsp.features.references import get_references
from lsp.features.symbol_resolution import find_var_at_position

URI = "file:///forms.tcl"


def _ref_starts(source: str, line: int, char: int) -> set[tuple[int, int]]:
    return {
        (r.range.start.line, r.range.start.character)
        for r in get_references(source, URI, line, char)
    }


class TestFindVarAtPositionForms:
    def test_plain_dollar(self):
        assert find_var_at_position("puts $count\n", 0, 7) == "count"

    def test_braced(self):
        assert find_var_at_position("puts ${count}\n", 0, 8) == "count"

    def test_braced_cursor_on_open_brace(self):
        # Cursor just after ``${`` still resolves the braced name.
        assert find_var_at_position("puts ${count}\n", 0, 7) == "count"

    def test_array_plain(self):
        assert find_var_at_position("puts $arr(k)\n", 0, 7) == "arr"

    def test_array_braced_returns_base(self):
        assert find_var_at_position("puts ${arr(k)}\n", 0, 8) == "arr"

    def test_global_qualified(self):
        assert find_var_at_position("puts $::g\n", 0, 7) == "::g"

    def test_nested_namespace_qualified(self):
        assert find_var_at_position("puts $::a::b::c\n", 0, 9) == "::a::b::c"

    def test_not_a_variable(self):
        assert find_var_at_position("puts hello\n", 0, 7) is None


class TestReferencesAcrossForms:
    def test_plain_dollar_uses(self):
        src = "proc p {} {\n set v 1\n puts $v\n puts $v\n}\n"
        assert _ref_starts(src, 2, 7) == {(1, 5), (2, 6), (3, 6)}

    def test_braced_use_resolves(self):
        src = "proc p {} {\n set v 1\n puts ${v}\n}\n"
        # Definition + the braced use.
        assert _ref_starts(src, 2, 8) == {(1, 5), (2, 6)}

    def test_mixed_plain_and_braced_share_one_symbol(self):
        src = "proc p {} {\n set v 1\n puts $v\n puts ${v}\n}\n"
        expected = {(1, 5), (2, 6), (3, 6)}
        # Cursor on the plain use and on the braced use must agree.
        assert _ref_starts(src, 2, 7) == expected
        assert _ref_starts(src, 3, 8) == expected

    def test_array_base_variable(self):
        src = "proc p {} {\n set a(k) 1\n puts $a(k)\n}\n"
        starts = _ref_starts(src, 2, 7)
        assert (1, 5) in starts  # definition
        assert (2, 6) in starts  # use

    def test_global_qualified(self):
        src = "set ::g 1\nputs $::g\n"
        starts = _ref_starts(src, 1, 5)
        assert (0, 4) in starts
        assert (1, 5) in starts

    def test_nested_namespace_global(self):
        src = "set ::a::b::c 1\nputs $::a::b::c\n"
        starts = _ref_starts(src, 1, 5)
        assert (0, 4) in starts
        assert (1, 5) in starts
