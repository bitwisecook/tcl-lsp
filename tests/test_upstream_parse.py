"""Tests ported from Tcl's official tests/parse.test.

These supplement the existing test_tcl_parse.py with additional coverage
derived from the upstream Tcl test suite
(https://github.com/tcltk/tcl/blob/main/tests/parse.test).

Areas covered that extend existing tests:
- Backslash substitution edge cases (parse-8.x / parse-20.x)
- Qualified command names with namespace separators
- Edge cases in brace/quote nesting
- Unicode and high-byte character handling
- Command substitution with nested structures
- Whitespace and separator edge cases
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.parsing.lexer import TclLexer
from core.parsing.tokens import TokenType

from .helpers import lex

# Helpers


def lex_all(source: str):
    """All tokens including SEP/EOL."""
    return TclLexer(source).tokenise_all()


def lex_texts(source: str) -> list[str]:
    return [t.text for t in lex(source)]


def lex_types(source: str) -> list[TokenType]:
    return [t.type for t in lex(source)]


# Backslash substitution (parse-8.x / parse-20.x additions)


class TestBackslashSubstitution:
    """Additional backslash escape tests from parse-8.x."""

    def test_backslash_a_alert(self):
        """parse-8.1: \\a (alert/bell) escape in quoted string."""
        tokens = lex('"\\a"')
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.ESC

    def test_backslash_b_backspace(self):
        """parse-8.2: \\b (backspace) escape."""
        tokens = lex('"\\b"')
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.ESC

    def test_backslash_f_formfeed(self):
        """parse-8.3: \\f (form feed) escape."""
        tokens = lex('"\\f"')
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.ESC

    def test_backslash_n_newline(self):
        """parse-8.4: \\n (newline) escape."""
        tokens = lex('"\\n"')
        assert len(tokens) == 1

    def test_backslash_r_return(self):
        """parse-8.5: \\r (carriage return) escape."""
        tokens = lex('"\\r"')
        assert len(tokens) == 1

    def test_backslash_t_tab(self):
        """parse-8.6: \\t (tab) escape."""
        tokens = lex('"\\t"')
        assert len(tokens) == 1

    def test_backslash_v_vertical_tab(self):
        """parse-8.7: \\v (vertical tab) escape."""
        tokens = lex('"\\v"')
        assert len(tokens) == 1

    def test_backslash_curly_brace_open(self):
        r"""parse-8.8: \{ escaped open brace."""
        source = '"\\{"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_backslash_curly_brace_close(self):
        r"""parse-8.9: \} escaped close brace."""
        source = '"\\}"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_backslash_bracket_open(self):
        r"""parse-8.10: \[ escaped open bracket."""
        source = '"\\["'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_backslash_bracket_close(self):
        r"""parse-8.11: \] escaped close bracket."""
        source = '"\\]"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_backslash_dollar(self):
        r"""parse-8.12: \$ escaped dollar sign."""
        source = '"\\$x"'
        tokens = lex(source)
        # The dollar should be escaped, not treated as a variable
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 0

    def test_backslash_semicolon(self):
        r"""parse-8.13: \; escaped semicolon."""
        source = '"\\;"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_backslash_space(self):
        r"""parse-8.14: \<space> escaped space."""
        source = "set x y\\ z"
        tokens = lex(source)
        # 'y\ z' should be a single word because backslash-space prevents splitting
        assert tokens[0].text == "set"

    def test_backslash_double_quote(self):
        r"""parse-8.15: \" escaped double quote."""
        source = r'"hello \" world"'
        tokens = lex(source)
        assert len(tokens) >= 1

    def test_backslash_backslash(self):
        r"""parse-8.16: \\ double backslash."""
        source = '"\\\\"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_hex_escape_2_digits(self):
        r"""parse-8.17: \xNN two-digit hex escape."""
        source = '"\\x41"'
        tokens = lex(source)
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.ESC

    def test_hex_escape_1_digit(self):
        r"""parse-8.18: \xN one-digit hex escape."""
        source = '"\\x9"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_octal_escape_3_digits(self):
        r"""parse-8.19: \NNN three-digit octal escape."""
        source = '"\\101"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_octal_escape_2_digits(self):
        r"""parse-8.20: \NN two-digit octal escape."""
        source = '"\\10"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_octal_escape_1_digit(self):
        r"""parse-8.21: \N one-digit octal escape."""
        source = '"\\0"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_unicode_escape_4_digits(self):
        r"""parse-8.22: \uNNNN four-digit Unicode escape."""
        source = '"\\u0041"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_unicode_escape_2_digits(self):
        r"""parse-8.23: \uNN short Unicode escape (Tcl pads to 4 digits)."""
        source = '"\\u41"'
        tokens = lex(source)
        assert len(tokens) == 1

    def test_unknown_escape_passes_through(self):
        r"""parse-8.24: \q unknown escape -- character passes through."""
        source = '"\\q"'
        tokens = lex(source)
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.ESC

    def test_backslash_at_eof_in_quotes(self):
        r"""parse-8.25: Backslash at end of quoted string."""
        source = '"abc\\'
        tokens = lex(source)
        # Should not crash, incomplete string is handled gracefully
        assert len(tokens) >= 1

    def test_backslash_newline_spaces_in_quotes(self):
        r"""parse-8.26: Backslash-newline with following spaces in quotes."""
        source = '"abc\\\n   def"'
        tokens = lex(source)
        assert len(tokens) >= 1
        all_text = "".join(t.text for t in tokens)
        assert "abc" in all_text
        assert "def" in all_text


# Qualified command names (namespace separator handling in command position)


class TestQualifiedCommands:
    """Namespace-qualified command names in various contexts."""

    def test_absolute_qualified_command(self):
        """::ns::cmd should be lexed as a single token."""
        tokens = lex("::ns::cmd arg1")
        assert tokens[0].text == "::ns::cmd"

    def test_deeply_qualified_command(self):
        """::a::b::c::cmd with deep nesting."""
        tokens = lex("::a::b::c::cmd")
        assert tokens[0].text == "::a::b::c::cmd"

    def test_relative_qualified_command(self):
        """ns::cmd without leading ::."""
        tokens = lex("ns::cmd arg")
        assert tokens[0].text == "ns::cmd"

    def test_global_namespace_prefix(self):
        """:: alone is valid (global namespace)."""
        tokens = lex("::set x 42")
        assert tokens[0].text == "::set"

    def test_qualified_variable(self):
        """$::ns::var namespace-qualified variable."""
        tokens = lex("$::ns::var")
        assert tokens[0].type == TokenType.VAR
        assert "::ns::var" in tokens[0].text


# Brace nesting edge cases (from parse-14.x extensions)


class TestBraceNesting:
    """Additional brace nesting edge cases from parse.test."""

    def test_balanced_braces_in_word(self):
        """Balanced braces yield correct content."""
        tokens = lex("{a {b {c}} d}")
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) == 1
        assert strs[0].text == "a {b {c}} d"

    def test_escaped_brace_in_braces(self):
        r"""Escaped braces inside braced word: {a \{ b \} c}."""
        source = "{a \\{ b \\} c}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) == 1

    def test_nested_braces_preserve_content(self):
        """Nested braces preserve their inner braces as literal text."""
        tokens = lex("{outer {inner} end}")
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert strs[0].text == "outer {inner} end"

    def test_multiline_braced_string(self):
        """Braced string spanning multiple lines."""
        source = "set body {\n  line1\n  line2\n  line3\n}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        assert "line1" in strs[0].text
        assert "line3" in strs[0].text

    def test_braces_with_special_chars(self):
        """Braces with special characters inside."""
        tokens = lex('{$x [cmd] \\n "quoted"}')
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) == 1
        # All special chars are literal inside braces
        assert "$x" in strs[0].text
        assert "[cmd]" in strs[0].text

    def test_empty_nested_braces(self):
        """Empty nested braces: {{}}."""
        tokens = lex("{{}}")
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert strs[0].text == "{}"

    def test_adjacent_braced_words(self):
        """Two braced words as separate arguments."""
        tokens = lex("cmd {a b} {c d}")
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) == 2
        assert strs[0].text == "a b"
        assert strs[1].text == "c d"


# Command substitution edge cases (from parse-6.x extensions)


class TestCommandSubstitution:
    """Command substitution edge cases from parse.test."""

    def test_empty_command_substitution(self):
        """Empty brackets: []."""
        tokens = lex("set x []")
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) >= 1

    def test_nested_command_substitution(self):
        """Nested command: [cmd1 [cmd2 arg]]."""
        tokens = lex("[cmd1 [cmd2 arg]]")
        assert tokens[0].type == TokenType.CMD
        assert "cmd2" in tokens[0].text

    def test_deeply_nested_commands(self):
        """Three-level nesting: [a [b [c]]]."""
        tokens = lex("[a [b [c]]]")
        assert tokens[0].type == TokenType.CMD
        assert "a" in tokens[0].text
        assert "c" in tokens[0].text

    def test_command_substitution_in_variable_index(self):
        """Array variable with command substitution in index."""
        tokens = lex("$arr([llength $list])")
        assert tokens[0].type == TokenType.VAR

    def test_multiple_command_substitutions_in_word(self):
        """Word with multiple command substitutions."""
        tokens = lex('"[cmd1][cmd2][cmd3]"')
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) == 3

    def test_command_with_braced_arg(self):
        """Command substitution with braced argument: [expr {1+1}]."""
        tokens = lex("[expr {1+1}]")
        assert tokens[0].type == TokenType.CMD
        assert "expr" in tokens[0].text

    def test_command_substitution_with_semicolons(self):
        """Semicolons inside command substitution."""
        tokens = lex("[set a 1; set b 2]")
        assert tokens[0].type == TokenType.CMD

    def test_command_substitution_with_newlines(self):
        """Newlines inside command substitution."""
        source = "[set a 1\nset b 2]"
        tokens = lex(source)
        assert tokens[0].type == TokenType.CMD


# Whitespace and separator edge cases


class TestWhitespace:
    """Whitespace handling edge cases from parse.test."""

    def test_tab_as_separator(self):
        """Tab character separates words."""
        tokens = lex("foo\tbar")
        texts = [t.text for t in tokens]
        assert texts == ["foo", "bar"]

    def test_multiple_tabs(self):
        """Multiple tabs between words."""
        tokens = lex("foo\t\t\tbar")
        texts = [t.text for t in tokens]
        assert texts == ["foo", "bar"]

    def test_mixed_whitespace(self):
        """Mixed spaces and tabs."""
        tokens = lex("foo \t bar")
        texts = [t.text for t in tokens]
        assert texts == ["foo", "bar"]

    def test_trailing_whitespace_ignored(self):
        """Trailing spaces after last word."""
        tokens = lex("foo bar   ")
        texts = [t.text for t in tokens]
        assert texts == ["foo", "bar"]

    def test_leading_and_trailing_whitespace(self):
        """Leading and trailing spaces."""
        tokens = lex("   foo bar   ")
        texts = [t.text for t in tokens]
        assert texts == ["foo", "bar"]

    def test_only_whitespace(self):
        """Only whitespace produces no tokens."""
        tokens = lex("   \t  \t  ")
        assert tokens == []

    def test_semicolons_and_whitespace(self):
        """Semicolons with surrounding whitespace."""
        tokens = lex("  ;  ;  foo  ;  ")
        texts = [t.text for t in tokens]
        assert texts == ["foo"]

    def test_newlines_between_commands(self):
        """Multiple newlines between commands."""
        source = "cmd1\n\n\ncmd2"
        tokens = lex(source)
        texts = [t.text for t in tokens]
        assert texts == ["cmd1", "cmd2"]


# Quote parsing edge cases (from parse-15.x extensions)


class TestQuoteEdgeCases:
    """Quoted string edge cases from parse.test."""

    def test_empty_quoted_string(self):
        """Empty quoted string: \"\"."""
        tokens = lex('""')
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.ESC
        assert tokens[0].text == ""

    def test_quoted_string_with_variable_and_text(self):
        """\"prefix${var}suffix\" -- variable interpolation."""
        tokens = lex('"prefix${var}suffix"')
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 1
        assert var_tokens[0].text == "var"

    def test_quoted_string_with_escaped_newline(self):
        r"""Quoted string with literal \n."""
        source = '"line1\\nline2"'
        tokens = lex(source)
        assert len(tokens) >= 1

    def test_quoted_multiline_string(self):
        """Quoted string spanning actual newlines."""
        source = '"line1\nline2"'
        tokens = lex(source)
        assert len(tokens) >= 1
        # Should contain both lines
        all_text = "".join(t.text for t in tokens)
        assert "line1" in all_text
        assert "line2" in all_text

    def test_adjacent_quoted_strings(self):
        """Two quoted strings as separate arguments."""
        tokens = lex('"hello" "world"')
        assert len(tokens) == 2

    def test_quoted_string_with_braces(self):
        """Braces inside quotes are literal."""
        tokens = lex('"a {b} c"')
        assert len(tokens) >= 1
        all_text = "".join(t.text for t in tokens)
        assert "{b}" in all_text

    def test_quoted_string_with_escaped_bracket(self):
        r"""Escaped bracket in quoted string: \[."""
        source = '"hello \\[world\\]"'
        tokens = lex(source)
        # No command substitution should occur
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) == 0


# Variable name parsing edge cases (from parse-12.x extensions)


class TestVarNameEdgeCases:
    """Additional variable name parsing from parse-12.x."""

    def test_global_namespace_var(self):
        """$::globalvar -- global namespace variable."""
        tokens = lex("$::globalvar")
        assert tokens[0].type == TokenType.VAR
        assert "::globalvar" in tokens[0].text

    def test_deeply_nested_namespace_var(self):
        """$::a::b::c::d -- deeply nested namespace variable."""
        tokens = lex("$::a::b::c::d")
        assert tokens[0].type == TokenType.VAR
        assert "::a::b::c::d" in tokens[0].text

    def test_var_followed_by_dot(self):
        """$x.y -- dot terminates variable name."""
        tokens = lex("$x.y")
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "x"

    def test_var_followed_by_comma(self):
        """$x,y -- comma terminates variable name."""
        tokens = lex("$x,y")
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "x"

    def test_var_with_underscore(self):
        """$my_var -- underscores in variable names."""
        tokens = lex("$my_var")
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "my_var"

    def test_var_with_digits(self):
        """$var123 -- digits in variable names."""
        tokens = lex("$var123")
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "var123"

    def test_var_starts_with_digit_not_var(self):
        """$1 -- digit-only variable."""
        tokens = lex("$1")
        # Numbers after $ may not form a valid variable name in Tcl
        # but the lexer should not crash
        assert len(tokens) >= 1

    def test_braced_var_with_spaces(self):
        """${my var} -- braced variable with spaces."""
        tokens = lex("${my var}")
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "my var"

    def test_braced_var_with_special_chars(self):
        """${a.b-c} -- braced variable with special characters."""
        tokens = lex("${a.b-c}")
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "a.b-c"

    def test_array_with_empty_index(self):
        """$arr() -- array with empty index."""
        tokens = lex("$arr()")
        assert tokens[0].type == TokenType.VAR

    def test_array_with_nested_var_index(self):
        """$arr($key) -- array with variable index."""
        tokens = lex("$arr($key)")
        assert tokens[0].type == TokenType.VAR

    def test_consecutive_vars(self):
        """$a$b$c -- multiple variables concatenated."""
        tokens = lex("$a$b$c")
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 3

    def test_var_in_braces_literal(self):
        """$x inside braces is literal, not a variable."""
        tokens = lex("{$x}")
        assert tokens[0].type == TokenType.STR
        assert "$x" in tokens[0].text
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 0


# Multi-command scripts


class TestMultiCommandScripts:
    """Parsing of multi-command scripts from parse.test."""

    def test_semicolon_separated(self):
        """Multiple commands separated by semicolons."""
        tokens = lex("set a 1; set b 2; set c 3")
        sets = [t for t in tokens if t.text == "set"]
        assert len(sets) == 3

    def test_newline_separated(self):
        """Multiple commands separated by newlines."""
        source = "set a 1\nset b 2\nset c 3"
        tokens = lex(source)
        sets = [t for t in tokens if t.text == "set"]
        assert len(sets) == 3

    def test_mixed_separators(self):
        """Semicolons and newlines mixed."""
        source = "set a 1; set b 2\nset c 3"
        tokens = lex(source)
        sets = [t for t in tokens if t.text == "set"]
        assert len(sets) == 3

    def test_comment_between_commands(self):
        """Comment between commands."""
        source = "set a 1\n# middle comment\nset b 2"
        tokens = lex(source)
        sets = [t for t in tokens if t.text == "set"]
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(sets) == 2
        assert len(comments) == 1

    def test_continuation_across_lines(self):
        """Command continuation with backslash-newline."""
        source = "set longvar \\\n  longvalue"
        tokens = lex(source)
        assert tokens[0].text == "set"

    def test_nested_command_bodies(self):
        """Typical Tcl script with nested command bodies."""
        source = "proc foo {} {\n  if {1} {\n    puts hello\n  }\n}"
        tokens = lex(source)
        assert tokens[0].text == "proc"
        assert tokens[1].text == "foo"

    def test_empty_script(self):
        """Empty script produces no tokens."""
        tokens = lex("")
        assert tokens == []

    def test_script_with_only_comments(self):
        """Script with only comments."""
        source = "# comment 1\n# comment 2\n# comment 3"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 3
        non_comments = [t for t in tokens if t.type != TokenType.COMMENT]
        assert len(non_comments) == 0


# Robustness: incomplete and malformed input (from completeness tests)


class TestIncompleteInput:
    """Robustness tests for incomplete/malformed input."""

    def test_unclosed_brace(self):
        """Unclosed brace should not crash."""
        tokens = lex("{abc")
        assert len(tokens) >= 1

    def test_unclosed_quote(self):
        """Unclosed quote should not crash."""
        tokens = lex('"abc')
        assert len(tokens) >= 1

    def test_unclosed_bracket(self):
        """Unclosed bracket should not crash."""
        tokens = lex("[abc")
        assert len(tokens) >= 1

    def test_unclosed_nested(self):
        """Nested unclosed structures."""
        tokens = lex('"hello [cmd {arg')
        assert len(tokens) >= 1

    def test_lone_dollar(self):
        """Lone $ sign."""
        tokens = lex("$")
        assert len(tokens) >= 1

    def test_lone_backslash(self):
        """Lone backslash at end of input."""
        tokens = lex("\\")
        assert len(tokens) >= 1

    def test_multiple_unclosed_braces(self):
        """Multiple unclosed braces."""
        tokens = lex("{{{")
        assert len(tokens) >= 1

    def test_mismatched_close_brace(self):
        """Close brace without open should not crash."""
        tokens = lex("abc}")
        assert len(tokens) >= 1

    def test_mismatched_close_bracket(self):
        """Close bracket without open should not crash."""
        tokens = lex("abc]")
        assert len(tokens) >= 1

    def test_very_long_word(self):
        """Very long word should not crash."""
        word = "a" * 10000
        tokens = lex(word)
        assert len(tokens) == 1
        assert len(tokens[0].text) == 10000

    def test_deeply_nested_braces(self):
        """Very deeply nested braces."""
        depth = 100
        source = "{" * depth + "content" + "}" * depth
        tokens = lex(source)
        assert len(tokens) >= 1

    def test_many_semicolons(self):
        """Many consecutive semicolons."""
        tokens = lex(";" * 100)
        # All are separators, no content tokens
        assert len(tokens) == 0
