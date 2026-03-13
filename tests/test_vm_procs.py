"""Tests for Tcl VM procedures: args, uplevel, upvar, apply, rename, info."""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError


class TestProcVariadic:
    """Tests for variadic procedures using 'args'."""

    def test_args_collects_remaining(self) -> None:
        interp = TclInterp()
        interp.eval("proc mylist {args} { return $args }")
        result = interp.eval("mylist a b c")
        assert result.value == "a b c"

    def test_args_with_fixed_params(self) -> None:
        interp = TclInterp()
        interp.eval('proc myfunc {a b args} { return "$a $b $args" }')
        result = interp.eval("myfunc 1 2 3 4 5")
        assert result.value == "1 2 3 4 5"

    def test_args_empty(self) -> None:
        interp = TclInterp()
        interp.eval("proc noargs {args} { llength $args }")
        result = interp.eval("noargs")
        assert result.value == "0"

    def test_default_and_args(self) -> None:
        interp = TclInterp()
        interp.eval('proc f {{x 10} args} { return "$x $args" }')
        result = interp.eval("f")
        assert result.value == "10 "
        result = interp.eval("f 5 a b")
        assert result.value == "5 a b"


class TestUplevel:
    """Tests for uplevel command."""

    def test_uplevel_basic(self) -> None:
        interp = TclInterp()
        interp.eval("proc setcaller {name val} { uplevel 1 [list set $name $val] }")
        interp.eval("setcaller x 42")
        result = interp.eval("set x")
        assert result.value == "42"

    def test_uplevel_in_proc(self) -> None:
        interp = TclInterp()
        interp.eval("proc outer {} { set local 99; inner; return $local }")
        interp.eval("proc inner {} { uplevel 1 { set local 77 } }")
        result = interp.eval("outer")
        assert result.value == "77"

    def test_uplevel_hash0(self) -> None:
        """uplevel #0 evaluates in global scope."""
        interp = TclInterp()
        interp.eval("proc setglobal {name val} { uplevel #0 [list set $name $val] }")
        interp.eval("setglobal g 100")
        result = interp.eval("set g")
        assert result.value == "100"


class TestUpvar:
    """Tests for upvar command."""

    def test_upvar_basic(self) -> None:
        interp = TclInterp()
        interp.eval("proc doublevar {varname} { upvar 1 $varname v; set v [expr {$v * 2}] }")
        interp.eval("set x 21")
        interp.eval("doublevar x")
        result = interp.eval("set x")
        assert result.value == "42"

    def test_upvar_read(self) -> None:
        interp = TclInterp()
        interp.eval("proc readvar {varname} { upvar 1 $varname v; return $v }")
        interp.eval("set myval hello")
        result = interp.eval("readvar myval")
        assert result.value == "hello"


class TestApply:
    """Tests for apply (anonymous procedures)."""

    def test_apply_basic(self) -> None:
        interp = TclInterp()
        result = interp.eval("apply {{x} { expr {$x * 2} }} 21")
        assert result.value == "42"

    def test_apply_with_multiple_args(self) -> None:
        interp = TclInterp()
        result = interp.eval("apply {{a b} { expr {$a + $b} }} 3 4")
        assert result.value == "7"

    def test_apply_closure_like(self) -> None:
        interp = TclInterp()
        result = interp.eval('apply {{x} { return "got $x" }} hello')
        assert result.value == "got hello"


class TestRename:
    """Tests for rename command."""

    def test_rename_command(self) -> None:
        interp = TclInterp()
        interp.eval("proc hello {} { return hi }")
        interp.eval("rename hello greet")
        result = interp.eval("greet")
        assert result.value == "hi"
        with pytest.raises(TclError, match="invalid command"):
            interp.eval("hello")

    def test_rename_to_empty_deletes(self) -> None:
        interp = TclInterp()
        interp.eval("proc temp {} { return x }")
        interp.eval('rename temp ""')
        with pytest.raises(TclError, match="invalid command"):
            interp.eval("temp")


class TestInfoCommand:
    """Tests for info subcommands."""

    def test_info_exists(self) -> None:
        interp = TclInterp()
        result = interp.eval("info exists x")
        assert result.value == "0"
        interp.eval("set x 1")
        result = interp.eval("info exists x")
        assert result.value == "1"

    def test_info_commands(self) -> None:
        interp = TclInterp()
        result = interp.eval("info commands set")
        assert "set" in result.value

    def test_info_procs(self) -> None:
        interp = TclInterp()
        interp.eval("proc myproc {} { return 1 }")
        result = interp.eval("info procs myproc")
        assert "myproc" in result.value

    def test_info_body(self) -> None:
        interp = TclInterp()
        interp.eval("proc f {x} { expr {$x + 1} }")
        result = interp.eval("info body f")
        assert "expr" in result.value

    def test_info_args(self) -> None:
        interp = TclInterp()
        interp.eval("proc f {a b c} { return ok }")
        result = interp.eval("info args f")
        assert result.value == "a b c"

    def test_info_default(self) -> None:
        interp = TclInterp()
        interp.eval("proc f {a {b 42}} { return ok }")
        result = interp.eval("info default f b d")
        assert result.value == "1"
        val = interp.eval("set d")
        assert val.value == "42"

    def test_info_level(self) -> None:
        interp = TclInterp()
        result = interp.eval("info level")
        assert result.value == "0"

    def test_info_level_in_proc(self) -> None:
        interp = TclInterp()
        interp.eval("proc f {} { info level }")
        result = interp.eval("f")
        assert result.value == "1"


class TestNestedProcs:
    """Tests for procs calling other procs."""

    def test_mutual_recursion(self) -> None:
        interp = TclInterp()
        interp.eval("proc even? {n} { if {$n == 0} { return 1 }; odd? [expr {$n - 1}] }")
        interp.eval("proc odd? {n} { if {$n == 0} { return 0 }; even? [expr {$n - 1}] }")
        assert interp.eval("even? 4").value == "1"
        assert interp.eval("even? 3").value == "0"
        assert interp.eval("odd? 3").value == "1"

    def test_proc_calling_proc(self) -> None:
        interp = TclInterp()
        interp.eval("proc add {a b} { expr {$a + $b} }")
        interp.eval("proc add3 {a b c} { add [add $a $b] $c }")
        result = interp.eval("add3 1 2 3")
        assert result.value == "6"

    def test_proc_local_scope(self) -> None:
        """Variables in procs are local by default."""
        interp = TclInterp()
        interp.eval("set x global_val")
        interp.eval("proc f {} { set x local_val; return $x }")
        result = interp.eval("f")
        assert result.value == "local_val"
        result = interp.eval("set x")
        assert result.value == "global_val"

    def test_global_command(self) -> None:
        interp = TclInterp()
        interp.eval("set counter 0")
        interp.eval("proc bump {} { global counter; incr counter }")
        interp.eval("bump")
        interp.eval("bump")
        interp.eval("bump")
        result = interp.eval("set counter")
        assert result.value == "3"


class TestReturnCommand:
    """Tests for return with various options."""

    def test_return_value(self) -> None:
        interp = TclInterp()
        interp.eval("proc f {} { return 42 }")
        result = interp.eval("f")
        assert result.value == "42"

    def test_return_empty(self) -> None:
        interp = TclInterp()
        interp.eval("proc f {} { return }")
        result = interp.eval("f")
        assert result.value == ""

    def test_return_stops_execution(self) -> None:
        interp = TclInterp()
        interp.eval("proc f {} { set x 1; return $x; set x 2 }")
        result = interp.eval("f")
        assert result.value == "1"


class TestErrorCommand:
    """Tests for error and throw commands."""

    def test_error_raises(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="test error"):
            interp.eval('error "test error"')

    def test_throw_raises(self) -> None:
        interp = TclInterp()
        with pytest.raises(TclError, match="test throw"):
            interp.eval('throw {CUSTOM ERROR} "test throw"')

    def test_error_caught_by_catch(self) -> None:
        interp = TclInterp()
        result = interp.eval('catch { error "oops" } msg')
        assert result.value == "1"
        msg = interp.eval("set msg")
        assert msg.value == "oops"
