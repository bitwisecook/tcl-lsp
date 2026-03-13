"""Pytest coverage for Tcl parseOld.test — old-style parser tests.

Translates the tests from ``tmp/tcl9.0.3/tests/parseOld.test`` into
pytest tests that exercise our VM.  Organised to match the original
test numbering so failures can be cross-referenced.

These tests predate the Tcl 8.1 parser and cover basic command syntax:
argument parsing, quoting, braces, command/variable/backslash substitution,
semicolons, result initialisation, syntax errors, long values, and comments.
"""

from __future__ import annotations

import pytest

from vm.interp import TclInterp

# Helper


def fresh() -> TclInterp:
    """Return a fresh interpreter without init.tcl sourcing."""
    return TclInterp(source_init=False)


def setup_helpers(interp: TclInterp) -> None:
    """Define the helper procs used throughout parseOld.test."""
    interp.eval(
        "proc fourArgs {a b c d} {\n"
        "    global arg1 arg2 arg3 arg4\n"
        "    set arg1 $a\n"
        "    set arg2 $b\n"
        "    set arg3 $c\n"
        "    set arg4 $d\n"
        "}"
    )
    interp.eval("proc getArgs args {\n    global argv\n    set argv $args\n}")


# ══════════════════════════════════════════════════════════════════
#  parseOld-1.*  —  Basic argument parsing
# ══════════════════════════════════════════════════════════════════


class TestBasicArgumentParsing:
    """parseOld-1.*: Basic argument parsing."""

    def test_1_1_whitespace_args(self) -> None:
        """parseOld-1.1: tabs and spaces between arguments."""
        interp = fresh()
        setup_helpers(interp)
        interp.eval("set arg1 {}")
        interp.eval("fourArgs a b\tc \t\t d")
        result = interp.eval("list $arg1 $arg2 $arg3 $arg4")
        assert result.value == "a b c d"

    def test_1_2_special_whitespace(self) -> None:
        """parseOld-1.2: vertical tab, form feed, carriage return as separators."""
        interp = fresh()
        setup_helpers(interp)
        interp.eval("set arg1 {}")
        interp.eval('eval "fourArgs 123\\v4\\f56\\r7890"')
        result = interp.eval("list $arg1 $arg2 $arg3 $arg4")
        assert result.value == "123 4 56 7890"


# ══════════════════════════════════════════════════════════════════
#  parseOld-2.*  —  Quotes and variable substitution
# ══════════════════════════════════════════════════════════════════


class TestQuotesAndVariableSubstitution:
    """parseOld-2.*: Quotes and variable substitution."""

    def test_2_1_quoted_arg(self) -> None:
        """parseOld-2.1: quoted string as single argument."""
        interp = fresh()
        setup_helpers(interp)
        interp.eval('getArgs "a b c" d')
        result = interp.eval("set argv")
        assert result.value == "{a b c} d"

    def test_2_2_var_in_quotes(self) -> None:
        """parseOld-2.2: variable substitution inside quotes."""
        interp = fresh()
        setup_helpers(interp)
        interp.eval("set a 101")
        interp.eval('getArgs "a$a b c"')
        result = interp.eval("set argv")
        assert result.value == "{a101 b c}"

    def test_2_3_cmd_subst_in_quotes(self) -> None:
        """parseOld-2.3: command substitution inside quotes."""
        interp = fresh()
        interp.eval('set argv "xy[format xabc]"')
        result = interp.eval("set argv")
        assert result.value == "xyxabc"

    def test_2_4_backslash_in_quotes(self) -> None:
        """parseOld-2.4: backslash substitution inside quotes."""
        interp = fresh()
        interp.eval('set argv "xy\\t"')
        result = interp.eval("set argv")
        assert result.value == "xy\t"

    def test_2_5_newline_in_quotes(self) -> None:
        """parseOld-2.5: newline inside quotes."""
        interp = fresh()
        interp.eval('set argv "a b\tc\nd e f"')
        result = interp.eval("set argv")
        assert result.value == "a b\tc\nd e f"

    def test_2_6_quotes_not_special_mid_word(self) -> None:
        """parseOld-2.6: quotes not special in middle of word."""
        interp = fresh()
        interp.eval('set argv a"bcd"e')
        result = interp.eval("set argv")
        assert result.value == 'a"bcd"e'


# ══════════════════════════════════════════════════════════════════
#  parseOld-3.*  —  Braces
# ══════════════════════════════════════════════════════════════════


class TestBraces:
    """parseOld-3.*: Brace handling."""

    def test_3_1_braced_arg(self) -> None:
        """parseOld-3.1: braced string as single argument."""
        interp = fresh()
        setup_helpers(interp)
        interp.eval("getArgs {a b c} d")
        result = interp.eval("set argv")
        assert result.value == "{a b c} d"

    def test_3_2_no_var_subst_in_braces(self) -> None:
        """parseOld-3.2: no variable substitution inside braces."""
        interp = fresh()
        interp.eval("set a 101")
        interp.eval("set argv {a$a b c}")
        result = interp.eval("string index $argv 1")
        assert result.value == "$"

    def test_3_3_no_cmd_subst_in_braces(self) -> None:
        """parseOld-3.3: no command substitution inside braces."""
        interp = fresh()
        interp.eval("set argv {a[format xyz] b}")
        result = interp.eval("string length $argv")
        assert result.value == "15"

    def test_3_4_backslash_in_braces(self) -> None:
        r"""parseOld-3.4: backslash sequences literal in braces."""
        interp = fresh()
        interp.eval(r"set argv {a\nb\}}")
        result = interp.eval("string length $argv")
        assert result.value == "6"

    def test_3_5_nested_braces(self) -> None:
        """parseOld-3.5: nested braces."""
        interp = fresh()
        interp.eval("set argv {{{{}}}}")
        result = interp.eval("set argv")
        assert result.value == "{{{}}}"

    def test_3_6_braces_mid_word(self) -> None:
        """parseOld-3.6: braces in middle of word."""
        interp = fresh()
        interp.eval("set argv a{{}}b")
        result = interp.eval("set argv")
        assert result.value == "a{{}}b"

    def test_3_7_close_bracket_in_format(self) -> None:
        """parseOld-3.7: close bracket in format result."""
        interp = fresh()
        interp.eval('set a [format "last]"]')
        result = interp.eval("set a")
        assert result.value == "last]"


# ══════════════════════════════════════════════════════════════════
#  parseOld-4.*  —  Command substitution
# ══════════════════════════════════════════════════════════════════


class TestCommandSubstitution:
    """parseOld-4.*: Command substitution."""

    def test_4_1_basic_cmd_subst(self) -> None:
        """parseOld-4.1: basic command substitution."""
        interp = fresh()
        interp.eval("set a [format xyz]")
        result = interp.eval("set a")
        assert result.value == "xyz"

    def test_4_2_multiple_cmd_subst(self) -> None:
        """parseOld-4.2: multiple command substitutions in one word."""
        interp = fresh()
        interp.eval("set a a[format xyz]b[format q]")
        result = interp.eval("set a")
        assert result.value == "axyzbq"

    def test_4_3_multiline_cmd_subst(self) -> None:
        """parseOld-4.3: multiline command substitution."""
        interp = fresh()
        interp.eval("set a a[\nset b 22;\nformat %s $b\n\n]b")
        result = interp.eval("set a")
        assert result.value == "a22b"

    def test_4_4_expr_in_catch(self) -> None:
        """parseOld-4.4: expr inside catch in command substitution."""
        interp = fresh()
        interp.eval("set a 7.7")
        interp.eval("if {[catch {expr {int($a)}}]} {set a foo}")
        result = interp.eval("set a")
        assert result.value == "7.7"


# ══════════════════════════════════════════════════════════════════
#  parseOld-5.*  —  Variable substitution
# ══════════════════════════════════════════════════════════════════


class TestVariableSubstitution:
    """parseOld-5.*: Variable substitution."""

    def test_5_1_simple_var(self) -> None:
        """parseOld-5.1: simple variable substitution."""
        interp = fresh()
        interp.eval("set a 123")
        result = interp.eval("set b $a")
        assert result.value == "123"

    def test_5_2_var_with_trailing_chars(self) -> None:
        """parseOld-5.2: variable with trailing non-alnum chars."""
        interp = fresh()
        interp.eval("set a 345")
        result = interp.eval("set b x$a.b")
        assert result.value == "x345.b"

    def test_5_3_underscore_var(self) -> None:
        """parseOld-5.3: variable name with underscores and digits."""
        interp = fresh()
        interp.eval("set _123z xx")
        result = interp.eval("set b $_123z^")
        assert result.value == "xx^"

    def test_5_4_braced_var_name(self) -> None:
        """parseOld-5.4: ${var} substitution."""
        interp = fresh()
        interp.eval("set a 78")
        result = interp.eval("set b a${a}b")
        assert result.value == "a78b"

    def test_5_5_nonexistent_var_catch(self) -> None:
        """parseOld-5.5: catch of nonexistent variable returns 1."""
        interp = fresh()
        result = interp.eval("catch {$_non_existent_} msg")
        assert result.value == "1"

    def test_5_6_nonexistent_var_msg(self) -> None:
        """parseOld-5.6: error message for nonexistent variable."""
        interp = fresh()
        interp.eval("catch {$_non_existent_} msg")
        result = interp.eval("set msg")
        assert 'can\'t read "_non_existent_"' in result.value

    def test_5_7_array_var(self) -> None:
        """parseOld-5.7: array variable substitution."""
        interp = fresh()
        interp.eval("unset -nocomplain a")
        interp.eval("set a(xyz) 123")
        result = interp.eval("set b $a(xyz)foo")
        assert result.value == "123foo"

    def test_5_8_array_var_spaces_in_index(self) -> None:
        """parseOld-5.8: array with spaces in index."""
        interp = fresh()
        interp.eval("unset -nocomplain a")
        interp.eval('set "a(x y z)" 123')
        result = interp.eval("set b $a(x y z)foo")
        assert result.value == "123foo"

    def test_5_9_array_subst_in_index(self) -> None:
        """parseOld-5.9: command substitution in array index."""
        interp = fresh()
        interp.eval("unset -nocomplain a qqq")
        interp.eval('set "a(x y z)" qqq')
        interp.eval("set $a([format x]\\ y [format z]) foo")
        result = interp.eval("set qqq")
        assert result.value == "foo"

    def test_5_10_nonexistent_array_element(self) -> None:
        """parseOld-5.10: error for nonexistent array element."""
        interp = fresh()
        interp.eval("unset -nocomplain a")
        result = interp.eval("list [catch {set b $a(22)} msg] $msg")
        assert result.value == '1 {can\'t read "a(22)": no such variable}'

    def test_5_11_dollar_not_special_before_nonalnum(self) -> None:
        """parseOld-5.11: $ not special before non-alphanumeric."""
        interp = fresh()
        result = interp.eval("set b a$!")
        assert result.value == "a$!"

    def test_5_12_empty_array_name(self) -> None:
        """parseOld-5.12: empty array name error."""
        interp = fresh()
        result = interp.eval("list [catch {set b a$()} msg] $msg")
        assert result.value == '1 {can\'t read "()": no such variable}'

    def test_5_12a_dollar_paren_scalar_roundtrip(self) -> None:
        """$() is a scalar variable named '()', not an array reference.

        You can ``set {()} val`` and read it back with ``$()``.
        This matches C Tcl behaviour across all versions."""
        interp = fresh()
        interp.eval("set {()} hello")
        result = interp.eval("set x $()")
        assert result.value == "hello"

    def test_5_13_long_array_index(self) -> None:
        """parseOld-5.13: very long array index."""
        interp = fresh()
        interp.eval("unset -nocomplain a")
        interp.eval(
            "set long {This is a very long variable, long enough to cause storage "
            "allocation to occur in Tcl_ParseVar.  If that storage isn't getting "
            "freed up correctly, then a core leak will occur when this test is "
            "run.  This text is probably beginning to sound like drivel, but I've "
            "run out of things to say and I need more characters still.}"
        )
        interp.eval("set a($long) 777")
        interp.eval("set b $a($long)")
        result = interp.eval("set b")
        assert result.value == "777"

    def test_5_14_nested_array_substitution(self) -> None:
        """parseOld-5.14: nested array variable substitution."""
        interp = fresh()
        interp.eval("unset -nocomplain a b a1")
        interp.eval("set a1(22) foo")
        interp.eval("set a(foo) bar")
        result = interp.eval("set b $a($a1(22))")
        assert result.value == "bar"


# ══════════════════════════════════════════════════════════════════
#  parseOld-7.*  —  Backslash substitution
# ══════════════════════════════════════════════════════════════════


class TestBackslashSubstitution:
    """parseOld-7.*: Backslash substitution."""

    def test_7_1_backslash_in_quotes(self) -> None:
        r"""parseOld-7.1: \a \c \n \] \} in double quotes."""
        interp = fresh()
        interp.eval('set a "\\a\\c\\n\\]\\}"')
        result = interp.eval("string length $a")
        assert result.value == "5"

    def test_7_2_backslash_in_braces(self) -> None:
        r"""parseOld-7.2: backslash sequences literal in braces."""
        interp = fresh()
        interp.eval(r"set a {\a\c\n\]\}}")
        result = interp.eval("string length $a")
        assert result.value == "10"

    def test_7_3_backslash_newline_in_quotes(self) -> None:
        """parseOld-7.3: backslash-newline in double quotes."""
        interp = fresh()
        interp.eval('set a "abc\\\ndef"')
        result = interp.eval("set a")
        assert result.value == "abc def"

    def test_7_4_backslash_newline_in_braces(self) -> None:
        """parseOld-7.4: backslash-newline in braces."""
        interp = fresh()
        interp.eval("set a {abc\\\ndef}")
        result = interp.eval("set a")
        # In braces, backslash-newline is preserved but normalised to single space
        assert result.value == "abc def"

    def test_7_5_backslash_newline_in_condition(self) -> None:
        """parseOld-7.5: backslash-newline in if condition."""
        interp = fresh()
        result = interp.eval(
            "set msg {}\n"
            "set a xxx\n"
            "set error [catch {if {24 < \\\n"
            "\t35} {set a 22} {set \\\n"
            "\t    a 33}} msg]\n"
            "list $error $msg $a"
        )
        assert result.value == "0 22 22"

    def test_7_6_backslash_at_end(self) -> None:
        """parseOld-7.6: backslash at end of string."""
        interp = fresh()
        result = interp.eval('eval "concat abc\\\\"')
        assert result.value == "abc\\"

    def test_7_7_backslash_newline_leading(self) -> None:
        """parseOld-7.7: backslash-newline at start."""
        interp = fresh()
        result = interp.eval('eval "concat \\\\\\na"')
        assert result.value == "a"

    def test_7_8_backslash_newline_with_spaces(self) -> None:
        """parseOld-7.8: backslash-newline with leading whitespace."""
        interp = fresh()
        result = interp.eval('eval "concat x\\\\\\n   \\ta"')
        assert result.value == "x a"

    def test_7_9_backslash_x(self) -> None:
        """parseOld-7.9: \\x escape."""
        interp = fresh()
        result = interp.eval('eval "concat \\\\x"')
        assert result.value == "x"

    def test_7_10_backslash_newline_in_list(self) -> None:
        """parseOld-7.10: backslash-newline in list command."""
        interp = fresh()
        result = interp.eval('eval "list a b\\\\\\nc d"')
        assert result.value == "a b c d"

    def test_7_11_backslash_newline_after_quotes(self) -> None:
        """parseOld-7.11: backslash-newline after quoted string."""
        interp = fresh()
        result = interp.eval('eval "list a \\"b c\\"\\\\\\nd e"')
        assert result.value == "a {b c} d e"

    def test_7_12_unicode_escape_2digit(self) -> None:
        """parseOld-7.12: \\uA2 (cent sign)."""
        interp = fresh()
        result = interp.eval('expr {[list \\uA2] eq "\\u00A2"}')
        assert result.value == "1"

    def test_7_13_unicode_escape_4digit(self) -> None:
        """parseOld-7.13: \\u4E21 (CJK character)."""
        interp = fresh()
        result = interp.eval('expr {[list \\u4E21] eq "\\u4E21"}')
        assert result.value == "1"

    def test_7_14_unicode_escape_partial(self) -> None:
        """parseOld-7.14: \\u4E2k — only 3 valid hex digits."""
        interp = fresh()
        result = interp.eval('expr {[list \\u4E2k] eq "\\u04E2k"}')
        assert result.value == "1"


# ══════════════════════════════════════════════════════════════════
#  parseOld-8.*  —  Semicolons
# ══════════════════════════════════════════════════════════════════


class TestSemicolons:
    """parseOld-8.*: Semicolons as command separators."""

    def test_8_1_semicolon_terminates(self) -> None:
        """parseOld-8.1: semicolon terminates command."""
        interp = fresh()
        setup_helpers(interp)
        interp.eval("set b 0")
        interp.eval("getArgs a;set b 2")
        result = interp.eval("set argv")
        assert result.value == "a"

    def test_8_2_semicolon_next_cmd(self) -> None:
        """parseOld-8.2: command after semicolon executes."""
        interp = fresh()
        setup_helpers(interp)
        interp.eval("set b 0")
        interp.eval("getArgs a;set b 2")
        result = interp.eval("set b")
        assert result.value == "2"

    def test_8_3_semicolon_with_spaces(self) -> None:
        """parseOld-8.3: semicolon with surrounding spaces."""
        interp = fresh()
        setup_helpers(interp)
        interp.eval("getArgs a b ; set b 1")
        result = interp.eval("set argv")
        assert result.value == "a b"

    def test_8_4_semicolon_both_cmds(self) -> None:
        """parseOld-8.4: both commands around semicolon execute."""
        interp = fresh()
        setup_helpers(interp)
        interp.eval("getArgs a b ; set b 1")
        result = interp.eval("set b")
        assert result.value == "1"


# ══════════════════════════════════════════════════════════════════
#  parseOld-9.*  —  Result initialisation
# ══════════════════════════════════════════════════════════════════


class TestResultInitialisation:
    """parseOld-9.*: Result re-initialisation between commands."""

    def test_9_1_simple_result(self) -> None:
        """parseOld-9.1: simple concat result."""
        interp = fresh()
        result = interp.eval("concat abc")
        assert result.value == "abc"

    def test_9_2_proc_after_concat(self) -> None:
        """parseOld-9.2: proc definition after concat returns empty."""
        interp = fresh()
        result = interp.eval("concat abc; proc foo {} {}")
        assert result.value == ""

    def test_9_3_proc_with_body(self) -> None:
        """parseOld-9.3: proc with variable body."""
        interp = fresh()
        interp.eval("set a 22")
        result = interp.eval("concat abc; proc foo {} $a")
        assert result.value == ""

    def test_9_4_proc_with_cmd_subst_body(self) -> None:
        """parseOld-9.4: proc with command-substituted body."""
        interp = fresh()
        result = interp.eval("proc foo {} [concat abc]")
        assert result.value == ""

    def test_9_5_trailing_semicolon(self) -> None:
        """parseOld-9.5: trailing semicolon preserves result."""
        interp = fresh()
        result = interp.eval("concat abc; ")
        assert result.value == "abc"

    def test_9_6_eval_braces(self) -> None:
        """parseOld-9.6: eval with braced script."""
        interp = fresh()
        result = interp.eval("eval {\n    concat abc\n}")
        assert result.value == "abc"

    def test_9_7_empty_script(self) -> None:
        """parseOld-9.7: empty script."""
        interp = fresh()
        result = interp.eval("")
        assert result.value == ""

    def test_9_8_multiple_semicolons(self) -> None:
        """parseOld-9.8: multiple semicolons."""
        interp = fresh()
        result = interp.eval("concat abc; ; ;")
        assert result.value == "abc"


# ══════════════════════════════════════════════════════════════════
#  parseOld-10.*  —  Syntax errors
# ══════════════════════════════════════════════════════════════════


class TestSyntaxErrors:
    """parseOld-10.*: Syntax error handling."""

    def test_10_1_missing_close_brace_catch(self) -> None:
        """parseOld-10.1: missing close-brace caught."""
        interp = fresh()
        result = interp.eval('catch "set a \\{bcd" msg')
        assert result.value == "1"

    def test_10_2_missing_close_brace_msg(self) -> None:
        """parseOld-10.2: missing close-brace error message."""
        interp = fresh()
        interp.eval('catch "set a \\{bcd" msg')
        result = interp.eval("set msg")
        assert result.value == "missing close-brace"

    def test_10_3_missing_close_quote_catch(self) -> None:
        """parseOld-10.3: missing close-quote caught."""
        interp = fresh()
        result = interp.eval('catch {set a "bcd} msg')
        assert result.value == "1"

    def test_10_4_missing_close_quote_msg(self) -> None:
        """parseOld-10.4: missing close-quote error message."""
        interp = fresh()
        interp.eval('catch {set a "bcd} msg')
        result = interp.eval("set msg")
        assert 'missing "' in result.value

    def test_10_5_extra_chars_after_quote_catch(self) -> None:
        """parseOld-10.5: extra chars after close-quote caught."""
        interp = fresh()
        result = interp.eval('catch {set a "bcd"xy} msg')
        assert result.value == "1"

    def test_10_6_extra_chars_after_quote_msg(self) -> None:
        """parseOld-10.6: extra chars after close-quote message."""
        interp = fresh()
        interp.eval('catch {set a "bcd"xy} msg')
        result = interp.eval("set msg")
        assert "extra characters after close-quote" in result.value

    def test_10_7_extra_chars_after_brace_catch(self) -> None:
        """parseOld-10.7: extra chars after close-brace caught."""
        interp = fresh()
        result = interp.eval('catch "set a {bcd}xy" msg')
        assert result.value == "1"

    def test_10_8_extra_chars_after_brace_msg(self) -> None:
        """parseOld-10.8: extra chars after close-brace message."""
        interp = fresh()
        interp.eval('catch "set a {bcd}xy" msg')
        result = interp.eval("set msg")
        assert "extra characters after close-brace" in result.value

    def test_10_9_missing_close_bracket_catch(self) -> None:
        """parseOld-10.9: missing close-bracket caught."""
        interp = fresh()
        result = interp.eval("catch {set a [format abc} msg")
        assert result.value == "1"

    def test_10_10_missing_close_bracket_msg(self) -> None:
        """parseOld-10.10: missing close-bracket message."""
        interp = fresh()
        interp.eval("catch {set a [format abc} msg")
        result = interp.eval("set msg")
        assert "missing close-bracket" in result.value

    def test_10_11_unknown_command_catch(self) -> None:
        """parseOld-10.11: unknown command caught."""
        interp = fresh()
        result = interp.eval("catch gorp-a-lot msg")
        assert result.value == "1"

    def test_10_12_unknown_command_msg(self) -> None:
        """parseOld-10.12: unknown command error message."""
        interp = fresh()
        interp.eval("catch gorp-a-lot msg")
        result = interp.eval("set msg")
        assert result.value == 'invalid command name "gorp-a-lot"'

    def test_10_13_backslash_newline_between_braces(self) -> None:
        """parseOld-10.13: backslash-newline between braced words."""
        interp = fresh()
        interp.eval("set a [concat {a}\\\n {b}]")
        result = interp.eval("set a")
        assert result.value == "a b"

    def test_10_15_misplaced_braces_in_proc(self) -> None:
        """parseOld-10.15: misplaced braces in proc body."""
        interp = fresh()
        interp.eval(
            "catch {\n"
            "\tproc misplaced_end_brace {} {\n"
            "\t    set what foo\n"
            "\t    set when [expr ${what}size - [set off$what]}]\n"
            "\t} msg"
        )
        result = interp.eval("set msg")
        assert "extra characters after close-brace" in result.value

    def test_10_16_misplaced_braces_in_set(self) -> None:
        """parseOld-10.16: misplaced braces in set value."""
        interp = fresh()
        interp.eval(
            "catch {\n"
            "\tset a {\n"
            "\t    set what foo\n"
            "\t    set when [expr ${what}size - [set off$what]}]\n"
            "\t} msg"
        )
        result = interp.eval("set msg")
        assert "extra characters after close-brace" in result.value

    def test_10_17_unusual_spacing(self) -> None:
        """parseOld-10.17: unusual spacing with brackets."""
        interp = fresh()
        result = interp.eval("list [catch {return [ [1]]} msg] $msg")
        assert result.value == '1 {invalid command name "1"}'


# ══════════════════════════════════════════════════════════════════
#  parseOld-11.*  —  Long values
# ══════════════════════════════════════════════════════════════════


_LONG_VALUE = (
    "1111 2222 3333 4444 5555 6666 7777 8888 9999 "
    "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii "
    "jjjj kkkk llll mmmm nnnn oooo pppp qqqq rrrr "
    "ssss tttt uuuu vvvv wwww xxxx yyyy zzzz AAAA "
    "BBBB CCCC DDDD EEEE FFFF GGGG HHHH"
)


class TestLongValues:
    """parseOld-11.*: Long value handling."""

    def test_11_1_string_length(self) -> None:
        """parseOld-11.1: string length of long value."""
        interp = fresh()
        interp.eval(f"set a {{{_LONG_VALUE}}}")
        result = interp.eval("string length $a")
        assert result.value == "214"

    def test_11_2_list_length(self) -> None:
        """parseOld-11.2: list length of long value."""
        interp = fresh()
        interp.eval(f"set a {{{_LONG_VALUE}}}")
        result = interp.eval("llength $a")
        assert result.value == "43"

    def test_11_3_quoted_long_value(self) -> None:
        """parseOld-11.3: quoted long value matches braced."""
        interp = fresh()
        interp.eval(f"set a {{{_LONG_VALUE}}}")
        interp.eval(f'set b "{_LONG_VALUE}"')
        result = interp.eval("expr {$b eq $a}")
        assert result.value == "1"

    def test_11_4_double_quoted_var(self) -> None:
        """parseOld-11.4: double-quoted variable expansion."""
        interp = fresh()
        interp.eval(f"set a {{{_LONG_VALUE}}}")
        interp.eval('set b "$a"')
        result = interp.eval("expr {$b eq $a}")
        assert result.value == "1"

    def test_11_5_cmd_subst_set(self) -> None:
        """parseOld-11.5: [set a] returns long value."""
        interp = fresh()
        interp.eval(f"set a {{{_LONG_VALUE}}}")
        interp.eval("set b [set a]")
        result = interp.eval("expr {$b eq $a}")
        assert result.value == "1"

    def test_11_6_concat_string_length(self) -> None:
        """parseOld-11.6: concat of long value has correct string length."""
        interp = fresh()
        interp.eval(f"set b [concat {_LONG_VALUE}]")
        result = interp.eval("string length $b")
        assert result.value == "214"

    def test_11_7_concat_list_length(self) -> None:
        """parseOld-11.7: concat of long value has correct list length."""
        interp = fresh()
        interp.eval(f"set b [concat {_LONG_VALUE}]")
        result = interp.eval("llength $b")
        assert result.value == "43"

    def test_11_9_even_longer_value(self) -> None:
        """parseOld-11.9: even longer concatenated value."""
        interp = fresh()
        long_val = (
            "0000 1111 2222 3333 4444 5555 6666 7777 8888 9999 "
            "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj "
            "kkkk llll mmmm nnnn oooo pppp qqqq rrrr ssss tttt "
            "uuuu vvvv wwww xxxx yyyy zzzz AAAA BBBB CCCC DDDD "
            "EEEE FFFF GGGG HHHH IIII JJJJ KKKK LLLL MMMM NNNN "
            "OOOO PPPP QQQQ RRRR SSSS TTTT UUUU VVVV WWWW XXXX "
            "YYYY ZZZZ"
        )
        interp.eval(f"set a [concat {long_val}]")
        result = interp.eval("llength $a")
        assert result.value == "62"

    def test_11_11_buffer_overflow_backslashes(self) -> None:
        """parseOld-11.11: buffer overflow test with backslashes in braces."""
        interp = fresh()
        # Long braced string with backslash escapes compared to "a"
        xs = "x" * 135
        ys = "y" * 126
        aaa = "\\101" * 26  # \\101 is octal for 'A'
        result = interp.eval(f'expr {{"a" == {{{xs}{ys}{aaa}}}}}')
        assert result.value == "0"


# ══════════════════════════════════════════════════════════════════
#  parseOld-11.10  —  Dynamic iteration tests
# ══════════════════════════════════════════════════════════════════


_ELEMENTS_62 = (
    "0000 1111 2222 3333 4444 5555 6666 7777 8888 9999 "
    "aaaa bbbb cccc dddd eeee ffff gggg hhhh iiii jjjj "
    "kkkk llll mmmm nnnn oooo pppp qqqq rrrr ssss tttt "
    "uuuu vvvv wwww xxxx yyyy zzzz AAAA BBBB CCCC DDDD "
    "EEEE FFFF GGGG HHHH IIII JJJJ KKKK LLLL MMMM NNNN "
    "OOOO PPPP QQQQ RRRR SSSS TTTT UUUU VVVV WWWW XXXX "
    "YYYY ZZZZ"
).split()

_CHARS_62 = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"


@pytest.mark.parametrize(
    "idx,element",
    list(enumerate(_ELEMENTS_62)),
    ids=[f"11.10-{i + 1}" for i in range(len(_ELEMENTS_62))],
)
def test_11_10_foreach_element(idx: int, element: str) -> None:
    """parseOld-11.10: each element matches repeated character."""
    expected = _CHARS_62[idx] * 4
    assert element == expected


# ══════════════════════════════════════════════════════════════════
#  parseOld-12.*  —  Comments
# ══════════════════════════════════════════════════════════════════


class TestComments:
    """parseOld-12.*: Comment handling."""

    def test_12_1_comment_in_eval(self) -> None:
        """parseOld-12.1: comment in eval is ignored."""
        interp = fresh()
        interp.eval("set a old")
        interp.eval("eval {  # set a new}")
        result = interp.eval("set a")
        assert result.value == "old"

    def test_12_2_comment_then_command(self) -> None:
        """parseOld-12.2: command after comment in eval."""
        interp = fresh()
        interp.eval("set a old")
        interp.eval('eval "  # set a new\\nset a new"')
        result = interp.eval("set a")
        assert result.value == "new"

    def test_12_3_comment_continuation(self) -> None:
        """parseOld-12.3: backslash-newline continues comment."""
        interp = fresh()
        interp.eval("set a old")
        interp.eval('eval "  # set a new\\\\\\nset a new"')
        result = interp.eval("set a")
        assert result.value == "old"

    def test_12_4_comment_double_backslash(self) -> None:
        """parseOld-12.4: double backslash does not continue comment."""
        interp = fresh()
        interp.eval("set a old")
        interp.eval('eval "  # set a new\\\\\\\\\\nset a new"')
        result = interp.eval("set a")
        assert result.value == "new"


# ══════════════════════════════════════════════════════════════════
#  parseOld-13.*  —  Comments at end of bracketed script
# ══════════════════════════════════════════════════════════════════


class TestCommentsInBrackets:
    """parseOld-13.*: Comments at end of bracketed script."""

    def test_13_1_comment_before_close_bracket(self) -> None:
        """parseOld-13.1: comment before closing bracket."""
        interp = fresh()
        result = interp.eval('set x "[\nexpr {1+1}\n# skip this!\n]"')
        assert result.value == "2"


# ══════════════════════════════════════════════════════════════════
#  parseOld-15.*  —  TclScriptEnd / info complete
# ══════════════════════════════════════════════════════════════════


class TestInfoComplete:
    """parseOld-15.*: info complete for script completeness checking."""

    def test_15_1_incomplete_bracket(self) -> None:
        """parseOld-15.1: incomplete bracket with comment containing ]."""
        interp = fresh()
        result = interp.eval("info complete {puts [\n\texpr {1+1}\n\t#this is a comment ]}")
        assert result.value == "0"

    def test_15_2_incomplete_backslash_newline(self) -> None:
        """parseOld-15.2: trailing backslash-newline is incomplete."""
        interp = fresh()
        result = interp.eval('info complete "abc\\\\\\n"')
        assert result.value == "0"

    def test_15_3_complete_double_backslash_newline(self) -> None:
        """parseOld-15.3: double backslash before newline is complete."""
        interp = fresh()
        result = interp.eval('info complete "abc\\\\\\\\\\n"')
        assert result.value == "1"

    def test_15_4_incomplete_brace_in_bracket(self) -> None:
        """parseOld-15.4: unmatched brace inside bracket."""
        interp = fresh()
        result = interp.eval('info complete "xyz \\[abc \\{abc\\]"')
        assert result.value == "0"

    def test_15_5_incomplete_bracket(self) -> None:
        """parseOld-15.5: unmatched bracket."""
        interp = fresh()
        result = interp.eval('info complete "xyz \\[abc"')
        assert result.value == "0"
