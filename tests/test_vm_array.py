"""Tests for Tcl VM array command."""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError


class TestArraySetGet:
    """Tests for array set and array get."""

    def test_set_and_get(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {a 1 b 2 c 3}")
        result = interp.eval("array get arr")
        assert "a 1" in result.value
        assert "b 2" in result.value
        assert "c 3" in result.value

    def test_set_empty(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {}")
        result = interp.eval("array size arr")
        assert result.value == "0"

    def test_set_odd_elements(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="even number"):
            interp.eval("array set arr {a 1 b}")

    def test_get_pattern(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {alpha 1 beta 2 gamma 3}")
        result = interp.eval('array get arr "a*"')
        assert "alpha 1" in result.value
        assert "beta" not in result.value

    def test_get_nonexistent(self) -> None:
        interp = TclInterp()
        result = interp.eval("array get noarray")
        assert result.value == ""

    def test_element_access(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {x 10 y 20}")
        result = interp.eval("dict get [array get arr] x")
        assert result.value == "10"

    def test_element_write_via_array_set(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {x 42}")
        result = interp.eval("dict get [array get arr] x")
        assert result.value == "42"

    def test_element_overwrite(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {x 10}")
        interp.eval("array set arr {x 99}")
        result = interp.eval("dict get [array get arr] x")
        assert result.value == "99"


class TestArrayNames:
    """Tests for array names."""

    def test_names(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {a 1 b 2 c 3}")
        result = interp.eval("array names arr")
        names = result.value.split()
        assert sorted(names) == ["a", "b", "c"]

    def test_names_pattern(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {alpha 1 beta 2 gamma 3}")
        result = interp.eval('array names arr "a*"')
        assert result.value == "alpha"

    def test_names_nonexistent(self) -> None:
        interp = TclInterp()
        result = interp.eval("array names noarray")
        assert result.value == ""

    def test_names_exact_mode(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {abc 1 abd 2 xyz 3}")
        result = interp.eval("array names arr -exact abc")
        assert result.value == "abc"


class TestArraySize:
    """Tests for array size."""

    def test_size(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {a 1 b 2 c 3}")
        result = interp.eval("array size arr")
        assert result.value == "3"

    def test_size_empty(self) -> None:
        interp = TclInterp()
        result = interp.eval("array size noarray")
        assert result.value == "0"


class TestArrayExists:
    """Tests for array exists."""

    def test_exists_true(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {a 1}")
        result = interp.eval("array exists arr")
        assert result.value == "1"

    def test_exists_false(self) -> None:
        interp = TclInterp()
        result = interp.eval("array exists noarray")
        assert result.value == "0"

    def test_exists_scalar(self) -> None:
        interp = TclInterp()
        interp.eval("set x 42")
        result = interp.eval("array exists x")
        assert result.value == "0"


class TestArrayUnset:
    """Tests for array unset."""

    def test_unset_all(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {a 1 b 2 c 3}")
        interp.eval("array unset arr")
        result = interp.eval("array exists arr")
        assert result.value == "0"

    def test_unset_pattern(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {alpha 1 beta 2 gamma 3}")
        interp.eval('array unset arr "a*"')
        result = interp.eval("array exists arr")
        assert result.value == "1"
        result = interp.eval("array size arr")
        assert result.value == "2"

    def test_unset_nonexistent(self) -> None:
        interp = TclInterp()
        # Should not error
        interp.eval("array unset noarray")


class TestArraySearch:
    """Tests for array search operations."""

    def test_startsearch_nextelement(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {a 1 b 2}")
        interp.eval("set sid [array startsearch arr]")
        e1 = interp.eval("array nextelement arr $sid")
        e2 = interp.eval("array nextelement arr $sid")
        e3 = interp.eval("array nextelement arr $sid")
        assert sorted([e1.value, e2.value]) == ["a", "b"]
        assert e3.value == ""

    def test_anymore(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {x 1}")
        interp.eval("set sid [array startsearch arr]")
        result = interp.eval("array anymore arr $sid")
        assert result.value == "1"
        interp.eval("array nextelement arr $sid")
        result = interp.eval("array anymore arr $sid")
        assert result.value == "0"

    def test_donesearch(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {x 1}")
        interp.eval("set sid [array startsearch arr]")
        interp.eval("array donesearch arr $sid")
        # After donesearch, nextelement should fail
        with pytest.raises(TclError, match="couldn't find"):
            interp.eval("array nextelement arr $sid")

    def test_startsearch_not_array(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="isn't an array"):
            interp.eval("array startsearch noarray")


class TestArrayStatistics:
    """Tests for array statistics."""

    def test_statistics(self) -> None:
        interp = TclInterp()
        interp.eval("array set arr {a 1 b 2 c 3}")
        result = interp.eval("array statistics arr")
        assert "3 entries" in result.value

    def test_statistics_not_array(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="isn't an array"):
            interp.eval("array statistics noarray")


class TestParray:
    """Tests for parray command."""

    def test_parray_output(self) -> None:
        import io as stdlib_io

        interp = TclInterp()
        buf = stdlib_io.StringIO()
        interp.channels["stdout"] = buf
        interp.eval("array set arr {b 2 a 1}")
        interp.eval("parray arr")
        output = buf.getvalue()
        assert "arr(a)" in output
        assert "arr(b)" in output
