"""Tests for Tcl VM control flow: nested loops, switch, try/on/finally, foreach variants."""

from __future__ import annotations

import pytest

from vm.interp import TclInterp
from vm.types import TclError


class TestNestedLoops:
    """Tests for nested loop constructs."""

    def test_nested_for_loops(self) -> None:
        interp = TclInterp()
        interp.eval("set sum 0")
        interp.eval(
            "for {set i 0} {$i < 3} {incr i} {"
            "  for {set j 0} {$j < 3} {incr j} {"
            "    set sum [expr {$sum + 1}]"
            "  }"
            "}"
        )
        result = interp.eval("set sum")
        assert result.value == "9"

    def test_break_inner_loop_only(self) -> None:
        interp = TclInterp()
        interp.eval("set result {}")
        interp.eval(
            "for {set i 0} {$i < 3} {incr i} {\n"
            "  for {set j 0} {$j < 5} {incr j} {\n"
            "    if {$j == 2} break\n"
            "  }\n"
            "  lappend result $j\n"
            "}"
        )
        result = interp.eval("set result")
        assert result.value == "2 2 2"

    def test_continue_inner_loop(self) -> None:
        interp = TclInterp()
        interp.eval("set sum 0")
        interp.eval(
            "for {set i 0} {$i < 3} {incr i} {\n"
            "  for {set j 0} {$j < 4} {incr j} {\n"
            "    if {$j == 2} continue\n"
            "    set sum [expr {$sum + 1}]\n"
            "  }\n"
            "}"
        )
        result = interp.eval("set sum")
        # 3 outer * (4 inner - 1 skipped) = 9
        assert result.value == "9"

    def test_while_with_break(self) -> None:
        interp = TclInterp()
        interp.eval("set i 0")
        interp.eval("while {1} { if {$i >= 5} break; incr i }")
        result = interp.eval("set i")
        assert result.value == "5"

    def test_while_with_continue(self) -> None:
        interp = TclInterp()
        interp.eval("set sum 0")
        interp.eval("set i 0")
        interp.eval("while {$i < 5} { incr i; if {$i == 3} continue; set sum [expr {$sum + $i}] }")
        result = interp.eval("set sum")
        # 1 + 2 + 4 + 5 = 12  (skip 3)
        assert result.value == "12"


class TestForeachVariants:
    """Tests for foreach with various argument forms."""

    def test_foreach_multiple_vars(self) -> None:
        interp = TclInterp()
        interp.eval("set result {}")
        interp.eval("foreach {a b} {1 2 3 4 5 6} { lappend result [expr {$a + $b}] }")
        result = interp.eval("set result")
        assert result.value == "3 7 11"

    def test_foreach_multiple_lists(self) -> None:
        interp = TclInterp()
        interp.eval("set result {}")
        interp.eval("foreach x {a b c} y {1 2 3} { lappend result $x$y }")
        result = interp.eval("set result")
        assert result.value == "a1 b2 c3"

    def test_foreach_uneven_list(self) -> None:
        interp = TclInterp()
        interp.eval("set result {}")
        interp.eval("foreach {a b} {1 2 3} { lappend result $a/$b }")
        result = interp.eval("set result")
        # Third iteration: a=3, b="" (empty default)
        assert result.value == "1/2 3/"

    def test_foreach_break(self) -> None:
        interp = TclInterp()
        interp.eval("set result {}")
        interp.eval('foreach x {a b c d e} { if {$x eq "c"} break; lappend result $x }')
        result = interp.eval("set result")
        assert result.value == "a b"

    def test_foreach_continue(self) -> None:
        interp = TclInterp()
        interp.eval("set result {}")
        interp.eval('foreach x {a b c d e} { if {$x eq "c"} continue; lappend result $x }')
        result = interp.eval("set result")
        assert result.value == "a b d e"


class TestSwitch:
    """Tests for switch command."""

    def test_switch_exact_match(self) -> None:
        interp = TclInterp()
        interp.eval('set x "b"')
        interp.eval('switch $x {  a { set r "first" }  b { set r "second" }  c { set r "third" }}')
        result = interp.eval("set r")
        assert result.value == "second"

    def test_switch_default(self) -> None:
        interp = TclInterp()
        interp.eval('switch "z" {  a { set r "first" }  default { set r "none" }}')
        result = interp.eval("set r")
        assert result.value == "none"

    def test_switch_glob_mode(self) -> None:
        interp = TclInterp()
        interp.eval('switch -glob "hello" {  h* { set r "matched" }  default { set r "nope" }}')
        interp_r = interp.eval("set r")
        assert interp_r.value == "matched"

    def test_switch_no_match(self) -> None:
        interp = TclInterp()
        interp.eval("set r before")
        interp.eval('switch "z" {  a { set r "first" }  b { set r "second" }}')
        result = interp.eval("set r")
        assert result.value == "before"

    def test_switch_fallthrough(self) -> None:
        interp = TclInterp()
        interp.eval('switch "a" {  a -  b -  c { set r "got it" }}')
        result = interp.eval("set r")
        assert result.value == "got it"


class TestTryOnFinally:
    """Tests for try/on/finally error handling."""

    def test_try_on_error(self) -> None:
        interp = TclInterp()
        interp.eval('try { error "boom" } on error {msg} { set result $msg }')
        result = interp.eval("set result")
        assert result.value == "boom"

    def test_try_on_ok(self) -> None:
        interp = TclInterp()
        interp.eval("try { set x 42 } on ok {val} { set result $val }")
        result = interp.eval("set result")
        assert result.value == "42"

    def test_try_finally(self) -> None:
        interp = TclInterp()
        interp.eval("set cleaned 0")
        interp.eval('catch { try { error "fail" } finally { set cleaned 1 } }')
        result = interp.eval("set cleaned")
        assert result.value == "1"

    def test_try_on_error_with_finally(self) -> None:
        interp = TclInterp()
        interp.eval("set cleaned 0")
        interp.eval(
            'try { error "fail" } on error {msg} { set err $msg } finally { set cleaned 1 }'
        )
        err = interp.eval("set err")
        assert err.value == "fail"
        cleaned = interp.eval("set cleaned")
        assert cleaned.value == "1"

    def test_try_no_error(self) -> None:
        interp = TclInterp()
        result = interp.eval("try { expr {6 * 7} }")
        assert result.value == "42"

    def test_try_catch_rethrows_unhandled(self) -> None:
        interp = TclInterp()
        # try with no matching handler should re-raise
        with pytest.raises(TclError, match="oops"):
            interp.eval('try { error "oops" } on ok {v} { set x ok }')


class TestCatchVariants:
    """Tests for catch command edge cases."""

    def test_catch_with_options_var(self) -> None:
        interp = TclInterp()
        interp.eval('catch { error "boom" } msg opts')
        msg = interp.eval("set msg")
        assert msg.value == "boom"
        opts = interp.eval("set opts")
        assert "-code 1" in opts.value

    def test_catch_returns_code(self) -> None:
        interp = TclInterp()
        result = interp.eval("catch { set x 42 }")
        assert result.value == "0"
        result = interp.eval('catch { error "fail" }')
        assert result.value == "1"

    def test_catch_break(self) -> None:
        interp = TclInterp()
        result = interp.eval("catch { break }")
        assert result.value == "3"  # TCL_BREAK

    def test_catch_continue(self) -> None:
        interp = TclInterp()
        result = interp.eval("catch { continue }")
        assert result.value == "4"  # TCL_CONTINUE

    def test_catch_return(self) -> None:
        interp = TclInterp()
        result = interp.eval("catch { return foo }")
        assert result.value == "2"  # TCL_RETURN


class TestEvalSource:
    """Tests for eval command."""

    def test_eval_simple(self) -> None:
        interp = TclInterp()
        result = interp.eval("eval { set x 42 }")
        assert result.value == "42"

    def test_eval_concat_args(self) -> None:
        interp = TclInterp()
        result = interp.eval("eval set x 42")
        assert result.value == "42"

    def test_eval_multi_commands(self) -> None:
        interp = TclInterp()
        interp.eval("eval { set a 1; set b 2 }")
        a = interp.eval("set a")
        b = interp.eval("set b")
        assert a.value == "1"
        assert b.value == "2"


class TestTimeCommand:
    """Tests for the time command."""

    def test_time_returns_microseconds(self) -> None:
        interp = TclInterp()
        result = interp.eval("time { set x 1 } 10")
        assert "microseconds per iteration" in result.value

    def test_time_default_count(self) -> None:
        interp = TclInterp()
        result = interp.eval("time { expr {1+1} }")
        assert "microseconds per iteration" in result.value
