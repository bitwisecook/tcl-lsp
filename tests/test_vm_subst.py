"""Tests for Tcl VM substitution: variables, commands, backslash escapes."""

from __future__ import annotations

import io

from vm.interp import TclInterp


class TestVariableSubstitution:
    """Tests for $var and ${var} substitution in various contexts."""

    def test_dollar_in_double_quotes(self) -> None:
        interp = TclInterp()
        interp.eval("set name World")
        result = interp.eval('set greeting "Hello, $name!"')
        assert result.value == "Hello, World!"

    def test_dollar_braces_in_double_quotes(self) -> None:
        interp = TclInterp()
        interp.eval("set name World")
        result = interp.eval('set greeting "Hello, ${name}!"')
        assert result.value == "Hello, World!"

    def test_dollar_in_bare_word(self) -> None:
        interp = TclInterp()
        interp.eval("set x 42")
        result = interp.eval("expr {$x + 1}")
        assert result.value == "43"

    def test_multiple_vars_in_string(self) -> None:
        interp = TclInterp()
        interp.eval("set first John")
        interp.eval("set last Doe")
        result = interp.eval('set full "$first $last"')
        assert result.value == "John Doe"

    def test_braces_suppress_substitution(self) -> None:
        interp = TclInterp()
        interp.eval("set x 42")
        # In braces, $x should remain literal when used as a string
        result = interp.eval("set y {$x}")
        assert result.value == "$x"

    def test_var_in_expr(self) -> None:
        interp = TclInterp()
        interp.eval("set a 10")
        interp.eval("set b 20")
        result = interp.eval("expr {$a * $b}")
        assert result.value == "200"


class TestCommandSubstitution:
    """Tests for [cmd ...] command substitution."""

    def test_command_subst_in_set(self) -> None:
        interp = TclInterp()
        result = interp.eval("set x [expr {2 + 3}]")
        assert result.value == "5"

    def test_nested_command_subst(self) -> None:
        interp = TclInterp()
        result = interp.eval("set x [expr {[expr {2 + 3}] * 2}]")
        assert result.value == "10"

    def test_command_subst_in_string(self) -> None:
        interp = TclInterp()
        result = interp.eval('set x "result is [expr {6 * 7}]"')
        assert result.value == "result is 42"

    def test_command_subst_in_if_body(self) -> None:
        interp = TclInterp()
        interp.eval("set x 5")
        interp.eval("if {$x > 0} { set y [expr {$x * 2}] }")
        result = interp.eval("set y")
        assert result.value == "10"

    def test_command_subst_in_proc_return(self) -> None:
        interp = TclInterp()
        interp.eval("proc add {a b} { return [expr {$a + $b}] }")
        result = interp.eval("add 3 4")
        assert result.value == "7"


class TestBackslashSubstitution:
    """Tests for backslash escape sequences."""

    def test_newline_escape(self) -> None:
        interp = TclInterp()
        buf = io.StringIO()
        interp.channels["stdout"] = buf
        interp.eval(r'puts "line1\nline2"')
        assert buf.getvalue() == "line1\nline2\n"

    def test_tab_escape(self) -> None:
        interp = TclInterp()
        buf = io.StringIO()
        interp.channels["stdout"] = buf
        interp.eval(r'puts "col1\tcol2"')
        assert buf.getvalue() == "col1\tcol2\n"

    def test_backslash_escape(self) -> None:
        interp = TclInterp()
        result = interp.eval(r'set x "a\\b"')
        assert result.value == "a\\b"

    def test_hex_escape(self) -> None:
        interp = TclInterp()
        result = interp.eval(r'set x "\x41"')
        assert result.value == "A"

    def test_unicode_escape(self) -> None:
        interp = TclInterp()
        result = interp.eval(r'set x "\u0041"')
        assert result.value == "A"

    def test_octal_escape(self) -> None:
        interp = TclInterp()
        result = interp.eval(r'set x "\101"')
        assert result.value == "A"


class TestSubstCommand:
    """Tests for the subst command itself."""

    def test_subst_variable(self) -> None:
        interp = TclInterp()
        interp.eval("set name World")
        result = interp.eval("subst {Hello, $name!}")
        assert result.value == "Hello, World!"

    def test_subst_command(self) -> None:
        interp = TclInterp()
        result = interp.eval("subst {2+3 = [expr {2+3}]}")
        assert result.value == "2+3 = 5"

    def test_subst_nocommands(self) -> None:
        interp = TclInterp()
        result = interp.eval("subst -nocommands {[expr {1+2}]}")
        assert result.value == "[expr {1+2}]"

    def test_subst_novariables(self) -> None:
        interp = TclInterp()
        interp.eval("set x 42")
        result = interp.eval("subst -novariables {x is $x}")
        assert result.value == "x is $x"
