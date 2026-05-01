"""Regression tests for ``.test``-file bootstrap failures.

Several Tcl built-ins (``lsearch -regexp``, ``string compare -length``,
``switch -regexp``, ``array names -regexp``) used to leak host Python
exceptions — ``re.error`` and ``ValueError`` — when handed inputs
that look invalid to the Python ``re`` engine or the bare ``int()``
parser.  Those leaks killed the tcltest harness early in the test
file, so files like ``lsearch.test`` and ``string.test`` would record
``Total=0`` even though many test cases were able to run.

This module pins the conversion-to-``TclError`` contract for each
command so a future change can't reintroduce the leak.
"""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError


class TestLsearchRegexpHostExceptions:
    """``lsearch -regexp`` must convert Python ``re.error`` to ``TclError``."""

    def test_invalid_quantifier_raises_tcl_error(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("lsearch -regexp {xyz bbcc *bc*} *bc*")
        assert "compile regular expression pattern" in str(exc.value)

    def test_unbalanced_paren_raises_tcl_error(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("lsearch -regexp {a b c} {(unclosed}")
        assert "compile regular expression pattern" in str(exc.value)

    def test_valid_regexp_still_works(self) -> None:
        interp = TclInterp()
        result = interp.eval("lsearch -regexp {abc def ghi} {^d}")
        assert result.value == "1"

    def test_regexp_with_nocase(self) -> None:
        interp = TclInterp()
        result = interp.eval("lsearch -regexp -nocase {ABC DEF GHI} {^d}")
        assert result.value == "1"


class TestStringCompareLengthHostExceptions:
    """``string compare -length`` must convert ``ValueError`` to ``TclError``."""

    def test_length_with_non_integer_value(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("string compare -length -nocase str1 str2")
        assert 'expected integer but got "-nocase"' in str(exc.value)

    def test_length_with_garbage_value(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("string compare -length abc str1 str2")
        assert 'expected integer but got "abc"' in str(exc.value)

    def test_string_equal_length_with_non_integer(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("string equal -length -nocase str1 str2")
        assert 'expected integer but got "-nocase"' in str(exc.value)

    def test_compare_bad_option(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("string compare a b c")
        assert 'bad option "a"' in str(exc.value)

    def test_compare_too_many_args(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("string compare -length 10 -nocase a b c")
        assert "wrong # args" in str(exc.value)

    def test_compare_length_eats_strings(self) -> None:
        # ``-length 10 10`` looks like there's only one string left
        # after option parsing — must surface as wrong-args, not raw
        # ``ValueError``.
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("string compare -length 10 10")
        assert "wrong # args" in str(exc.value)

    def test_compare_basic_still_works(self) -> None:
        interp = TclInterp()
        assert interp.eval("string compare abc abc").value == "0"
        assert interp.eval("string compare abc abd").value == "-1"
        assert interp.eval("string compare -nocase ABC abc").value == "0"
        assert interp.eval("string compare -length 3 abcd abce").value == "0"

    def test_compare_combined_nocase_and_length(self) -> None:
        # ``string compare -nocase -length N s1 s2`` is the maximum
        # legal form — five tokens after the subcommand.  Reject the
        # earlier ``len(rest) > 4`` guard that broke this combination.
        interp = TclInterp()
        assert interp.eval("string compare -nocase -length 1 A a").value == "0"
        assert interp.eval("string compare -length 1 -nocase A a").value == "0"
        assert interp.eval("string equal -nocase -length 1 A a").value == "1"
        assert interp.eval("string equal -length 1 -nocase A a").value == "1"


class TestSwitchRegexpHostExceptions:
    """``switch -regexp`` must convert Python ``re.error`` to ``TclError``."""

    def test_invalid_pattern(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("switch -regexp foo {*bad* { puts hi }}")
        assert "compile regular expression pattern" in str(exc.value)


class TestArrayNamesRegexpHostExceptions:
    """``array names -regexp`` must convert Python ``re.error`` to ``TclError``."""

    def test_invalid_pattern(self) -> None:
        interp = TclInterp()
        interp.eval("array set a {x 1 y 2}")
        with pytest.raises(TclError) as exc:
            interp.eval("array names a -regexp {*bad*}")
        assert "compile regular expression pattern" in str(exc.value)


class TestStringRepeatHostExceptions:
    """``string repeat`` must surface a Tcl error on a non-integer count."""

    def test_non_integer_count(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("string repeat foo bar")
        assert 'expected integer but got "bar"' in str(exc.value)


class TestLrepeatHostExceptions:
    """``lrepeat`` must surface a Tcl error on a non-integer count."""

    def test_non_integer_count(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError) as exc:
            interp.eval("lrepeat foo a b")
        assert 'expected integer but got "foo"' in str(exc.value)
