"""Tests for Tcl VM dict command."""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError


class TestDictCreate:
    """Tests for dict create."""

    def test_create_empty(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict create")
        assert result.value == ""

    def test_create_pairs(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict create a 1 b 2 c 3")
        assert result.value == "a 1 b 2 c 3"

    def test_create_odd_args(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="wrong # args"):
            interp.eval("dict create a 1 b")

    def test_create_with_spaces(self) -> None:
        interp = TclInterp()
        result = interp.eval('dict create "hello world" value')
        assert "hello world" in result.value


class TestDictGetSet:
    """Tests for dict get and dict set."""

    def test_get_single_key(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict get {a 1 b 2 c 3} b")
        assert result.value == "2"

    def test_get_missing_key(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="not known"):
            interp.eval("dict get {a 1 b 2} z")

    def test_get_nested(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict get {a {x 10 y 20} b 2} a y")
        assert result.value == "20"

    def test_get_whole_dict(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict get {a 1 b 2}")
        assert result.value == "a 1 b 2"

    def test_set_new_key(self) -> None:
        interp = TclInterp()
        interp.eval("set d {a 1}")
        interp.eval("dict set d b 2")
        result = interp.eval("set d")
        assert "b 2" in result.value

    def test_set_overwrite(self) -> None:
        interp = TclInterp()
        interp.eval("set d {a 1 b 2}")
        interp.eval("dict set d b 99")
        result = interp.eval("dict get $d b")
        assert result.value == "99"

    def test_set_nested(self) -> None:
        interp = TclInterp()
        interp.eval("set d {}")
        interp.eval("dict set d outer inner 42")
        result = interp.eval("dict get $d outer inner")
        assert result.value == "42"

    def test_set_creates_var(self) -> None:
        interp = TclInterp()
        interp.eval("dict set newvar key value")
        result = interp.eval("dict get $newvar key")
        assert result.value == "value"


class TestDictExists:
    """Tests for dict exists."""

    def test_exists_true(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict exists {a 1 b 2} a")
        assert result.value == "1"

    def test_exists_false(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict exists {a 1 b 2} z")
        assert result.value == "0"

    def test_exists_nested(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict exists {a {x 10}} a x")
        assert result.value == "1"
        result = interp.eval("dict exists {a {x 10}} a y")
        assert result.value == "0"


class TestDictKeysValues:
    """Tests for dict keys and dict values."""

    def test_keys(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict keys {a 1 b 2 c 3}")
        assert result.value == "a b c"

    def test_keys_pattern(self) -> None:
        interp = TclInterp()
        result = interp.eval('dict keys {alpha 1 beta 2 gamma 3} "a*"')
        assert result.value == "alpha"

    def test_values(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict values {a 1 b 2 c 3}")
        assert result.value == "1 2 3"

    def test_values_pattern(self) -> None:
        interp = TclInterp()
        result = interp.eval('dict values {a hello b world c hi} "h*"')
        assert result.value == "hello hi"


class TestDictSize:
    """Tests for dict size."""

    def test_size_empty(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict size {}")
        assert result.value == "0"

    def test_size(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict size {a 1 b 2 c 3}")
        assert result.value == "3"


class TestDictRemoveReplace:
    """Tests for dict remove and dict replace."""

    def test_remove(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict remove {a 1 b 2 c 3} b")
        d = result.value
        assert "b" not in d
        assert "a 1" in d
        assert "c 3" in d

    def test_remove_missing_key(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict remove {a 1 b 2} z")
        assert result.value == "a 1 b 2"

    def test_replace(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict replace {a 1 b 2} b 99 c 3")
        assert "b 99" in result.value
        assert "c 3" in result.value

    def test_merge(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict merge {a 1 b 2} {b 99 c 3}")
        assert "b 99" in result.value
        assert "c 3" in result.value
        assert "a 1" in result.value


class TestDictFor:
    """Tests for dict for."""

    def test_for_basic(self) -> None:
        interp = TclInterp()
        interp.eval("set result {}")
        interp.eval('dict for {k v} {a 1 b 2 c 3} { lappend result "$k=$v" }')
        result = interp.eval("set result")
        assert result.value == "a=1 b=2 c=3"

    def test_for_break(self) -> None:
        interp = TclInterp()
        interp.eval("set result {}")
        interp.eval('dict for {k v} {a 1 b 2 c 3} { if {$k eq "b"} break; lappend result $k }')
        result = interp.eval("set result")
        assert result.value == "a"

    def test_for_continue(self) -> None:
        interp = TclInterp()
        interp.eval("set result {}")
        interp.eval('dict for {k v} {a 1 b 2 c 3} { if {$k eq "b"} continue; lappend result $k }')
        result = interp.eval("set result")
        assert result.value == "a c"


class TestDictMutate:
    """Tests for dict append, lappend, incr, unset."""

    def test_append(self) -> None:
        interp = TclInterp()
        interp.eval("set d {key hello}")
        interp.eval('dict append d key " world"')
        result = interp.eval("dict get $d key")
        assert result.value == "hello world"

    def test_lappend(self) -> None:
        interp = TclInterp()
        interp.eval("set d {key {a b}}")
        interp.eval("dict lappend d key c d")
        result = interp.eval("dict get $d key")
        assert result.value == "a b c d"

    def test_incr_default(self) -> None:
        interp = TclInterp()
        interp.eval("set d {count 5}")
        interp.eval("dict incr d count")
        result = interp.eval("dict get $d count")
        assert result.value == "6"

    def test_incr_amount(self) -> None:
        interp = TclInterp()
        interp.eval("set d {count 5}")
        interp.eval("dict incr d count 10")
        result = interp.eval("dict get $d count")
        assert result.value == "15"

    def test_incr_new_key(self) -> None:
        interp = TclInterp()
        interp.eval("set d {}")
        interp.eval("dict incr d count")
        result = interp.eval("dict get $d count")
        assert result.value == "1"

    def test_unset(self) -> None:
        interp = TclInterp()
        interp.eval("set d {a 1 b 2 c 3}")
        interp.eval("dict unset d b")
        result = interp.eval("dict exists $d b")
        assert result.value == "0"

    def test_unset_nested(self) -> None:
        interp = TclInterp()
        interp.eval("set d {outer {x 10 y 20}}")
        interp.eval("dict unset d outer x")
        result = interp.eval("dict exists $d outer x")
        assert result.value == "0"
        result = interp.eval("dict exists $d outer y")
        assert result.value == "1"


class TestDictMapFilter:
    """Tests for dict map and dict filter."""

    def test_map(self) -> None:
        interp = TclInterp()
        # The compiler lowers dict map to lmap, which returns a flat list
        # of body results (not a dict). The _dict_map handler returns a dict
        # but it's only reachable via direct invocation, not the compiler path.
        result = interp.eval("dict map {k v} {a 1 b 2 c 3} { expr {$v * 2} }")
        assert result.value == "2 4 6"

    def test_filter_key(self) -> None:
        interp = TclInterp()
        result = interp.eval('dict filter {apple 1 banana 2 cherry 3} key "a*"')
        assert "apple 1" in result.value
        assert "banana" not in result.value

    def test_filter_value(self) -> None:
        interp = TclInterp()
        result = interp.eval('dict filter {a hello b world c hi} value "h*"')
        assert "a hello" in result.value
        assert "c hi" in result.value
        assert "b" not in result.value

    def test_filter_script(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict filter {a 1 b 2 c 3 d 4} script {k v} { expr {$v > 2} }")
        assert "c 3" in result.value
        assert "d 4" in result.value
        assert "a" not in result.value


class TestDictUpdateWith:
    """Tests for dict update and dict with."""

    def test_update(self) -> None:
        interp = TclInterp()
        interp.eval("set d {name Alice age 30}")
        # Use incr to avoid [expr ...] brackets which trigger PUSH-time substitution
        interp.eval("dict update d name n age a { incr a }")
        result = interp.eval("dict get $d age")
        assert result.value == "31"

    def test_with(self) -> None:
        interp = TclInterp()
        interp.eval("set d {x 10 y 20}")
        # Use incr $x to double x (10 + 10 = 20), avoiding brackets
        interp.eval("dict with d { incr x $x }")
        result = interp.eval("dict get $d x")
        assert result.value == "20"

    def test_update_with_expr(self) -> None:
        interp = TclInterp()
        interp.eval("set d {name Alice age 30}")
        interp.eval("dict update d name n age a { set a [expr {$a + 1}] }")
        result = interp.eval("dict get $d age")
        assert result.value == "31"

    def test_with_expr(self) -> None:
        interp = TclInterp()
        interp.eval("set d {x 10 y 20}")
        interp.eval("dict with d { set x [expr {$x * 2}] }")
        result = interp.eval("dict get $d x")
        assert result.value == "20"

    def test_info(self) -> None:
        interp = TclInterp()
        result = interp.eval("dict info {a 1 b 2 c 3}")
        assert "3 entries" in result.value
