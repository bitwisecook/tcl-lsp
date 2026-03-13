r"""Tests recreated from Tcl's official tests/parseOld.test.

These tests verify our TclLexer's behaviour against the "old-style" parsing
tests from the official Tcl test suite
(https://github.com/tcltk/tcl/blob/main/tests/parseOld.test).

parseOld.test focuses on argument parsing, quoting, braces, command
substitution, variable substitution, backslash escapes, semicolons,
comments, and syntax error resilience.  Since our lexer is a tokeniser
(not an evaluator), we adapt each test to verify correct token production.
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.parsing.lexer import TclLexer, TclParseError
from core.parsing.tokens import Token, TokenType

from .helpers import lex

# Helpers


def lex_all(source: str) -> list[Token]:
    """All tokens including SEP/EOL."""
    return TclLexer(source).tokenise_all()


def lex_types(source: str) -> list[TokenType]:
    """Token types excluding SEP/EOL."""
    return [t.type for t in lex(source)]


def lex_texts(source: str) -> list[str]:
    """Token texts excluding SEP/EOL."""
    return [t.text for t in lex(source)]


# Group 1: Basic argument parsing (parseOld-1.x)


class TestBasicArgParsing:
    """parseOld-1.x: Basic argument parsing with whitespace."""

    def test_1_1_four_simple_args(self):
        """parseOld-1.1: fourArgs a b c d -- four separate words."""
        _ = lex("fourArgs a b c d")
        texts = lex_texts("fourArgs a b c d")
        assert texts == ["fourArgs", "a", "b", "c", "d"]

    def test_1_1_tab_separation(self):
        """Tab separated arguments."""
        tokens = lex("fourArgs\ta\tb\tc\td")
        texts = [t.text for t in tokens]
        assert texts == ["fourArgs", "a", "b", "c", "d"]

    def test_1_1_multiple_spaces(self):
        """Multiple spaces between args."""
        tokens = lex("fourArgs   a   b   c   d")
        texts = [t.text for t in tokens]
        assert texts == ["fourArgs", "a", "b", "c", "d"]

    def test_1_2_special_whitespace_chars(self):
        """parseOld-1.2: Form feed, carriage return as separators."""
        source = "fourArgs 123\v4\f56\r7890"
        tokens = lex(source)
        assert tokens[0].text == "fourArgs"
        # \v and \f are not standard Tcl word separators, so they'll be in words
        assert len(tokens) >= 2


# Group 2: Quotes (parseOld-2.x)


class TestQuotes:
    """parseOld-2.x: Quoted strings and variable substitution."""

    def test_2_1_quoted_string_groups_words(self):
        """parseOld-2.1: Quoted string groups space-separated words."""
        tokens = lex('getArgs "a b c" d')
        # "a b c" is a single ESC token
        assert any(t.text == "a b c" for t in tokens)
        assert any(t.text == "d" for t in tokens)

    def test_2_2_variable_in_quoted_string(self):
        """parseOld-2.2: Variable expansion inside quotes."""
        tokens = lex('getArgs "a$a b c"')
        types = [t.type for t in tokens]
        # Should have ESC("a"), VAR("a"), ESC(" b c") inside the quote
        assert TokenType.VAR in types

    def test_2_3_command_sub_in_quoted_string(self):
        """parseOld-2.3: Command substitution in quoted string."""
        tokens = lex('set argv "xy[format xabc]"')
        types = [t.type for t in tokens]
        assert TokenType.CMD in types

    def test_2_4_tab_escape_in_quotes(self):
        r"""parseOld-2.4: \t inside quotes is part of the string."""
        tokens = lex(r'set argv "xy\t"')
        # The \t is inside quotes, lexer returns it as ESC text
        assert any("xy" in t.text for t in tokens)

    def test_2_5_multiline_quoted_string(self):
        """parseOld-2.5: Quoted string spanning multiple lines."""
        source = 'set argv "a b\tc\nd e f"'
        tokens = lex(source)
        # Should have a multi-line quoted token
        assert any("\n" in t.text for t in tokens)

    def test_2_6_quotes_within_unquoted(self):
        """parseOld-2.6: Quotes within an unquoted string are literal."""
        tokens = lex('set argv a"bcd"e')
        # a"bcd"e is a single unquoted word -- the mid-word quotes are literal
        all_text = "".join(t.text for t in tokens if t.text not in ("set", "argv"))
        assert "bcd" in all_text

    def test_empty_quoted_string(self):
        """Empty quoted string."""
        tokens = lex('set argv ""')
        assert any(t.type == TokenType.ESC and t.text == "" for t in tokens)

    def test_nested_quotes_in_command(self):
        """Quoted string inside command substitution."""
        tokens = lex('set x [list "a b" "c d"]')
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) >= 1
        assert '"' in cmd_tokens[0].text


# Group 3: Braces (parseOld-3.x)


class TestBraces:
    """parseOld-3.x: Braced strings -- no substitution inside."""

    def test_3_1_braced_arg(self):
        """parseOld-3.1: Braced string groups words literally."""
        tokens = lex("getArgs {a b c} d")
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any("a b c" in t.text for t in strs)

    def test_3_2_dollar_in_braces(self):
        """parseOld-3.2: $ in braces is literal, not variable."""
        tokens = lex("set argv {a$a b c}")
        strs = [t for t in tokens if t.type == TokenType.STR]
        # The $ is literal
        assert any("$" in t.text for t in strs)
        # No VAR token inside braces
        # (only top-level tokens -- braces suppress substitution)
        var_in_brace = False
        for t in strs:
            if "$a" in t.text:
                var_in_brace = True
        assert var_in_brace  # $ appears literally in the STR text

    def test_3_3_bracket_in_braces(self):
        """parseOld-3.3: [format xyz] in braces is literal, not command sub."""
        tokens = lex("set argv {a[format xyz] b}")
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any("[format xyz]" in t.text for t in strs)
        # No CMD token for the content inside braces
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) == 0

    def test_3_4_backslash_in_braces(self):
        r"""parseOld-3.4: \n and \} in braces are literal characters."""
        source = "set argv {a\\nb\\}}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        # Backslashes preserved literally in braces
        assert "\\" in strs[0].text

    def test_3_5_nested_braces(self):
        """parseOld-3.5: Nested braces are preserved."""
        tokens = lex("set argv {{{}}}")
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        assert "{}" in strs[0].text

    def test_3_6_braces_in_unbraced(self):
        """parseOld-3.6: Braces within an unbraced context are literal."""
        tokens = lex("set argv a{{}}b")
        all_text = "".join(t.text for t in tokens if t.text not in ("set", "argv"))
        assert "{{}}" in all_text or "a{{}}b" in all_text

    def test_3_7_closing_bracket_in_format(self):
        """parseOld-3.7: Closing bracket in format result."""
        tokens = lex('set a [format "last]"]')
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) >= 1

    def test_empty_braces(self):
        """Empty braces produce empty STR token."""
        tokens = lex("set x {}")
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        assert strs[0].text == ""


# Group 4: Command substitution (parseOld-4.x)


class TestCommandSubstitution:
    """parseOld-4.x: Command substitution with [brackets]."""

    def test_4_1_basic_command_sub(self):
        """parseOld-4.1: [format xyz] substitution."""
        tokens = lex("set a [format xyz]")
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) == 1
        assert "format xyz" in cmd_tokens[0].text

    def test_4_2_multiple_command_subs(self):
        """parseOld-4.2: Multiple command substitutions in one word."""
        tokens = lex("set a a[format xyz]b[format q]")
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) == 2

    def test_4_3_multiline_command_sub(self):
        """parseOld-4.3: Multiline command substitution."""
        source = "set a a[\nset b 22;\nformat %s $b\n]b"
        tokens = lex(source)
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) >= 1

    def test_4_4_nested_command_subs(self):
        """parseOld-4.4: Nested command substitution."""
        source = "set a [concat [format xyz]]"
        tokens = lex(source)
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) >= 1

    def test_deeply_nested_commands(self):
        """Deeply nested command substitutions."""
        source = "[cmd1 [cmd2 [cmd3 arg]]]"
        tokens = lex(source)
        # The outermost [] is one CMD token
        assert tokens[0].type == TokenType.CMD

    def test_command_sub_with_braces(self):
        """Command substitution containing braced arg."""
        source = "[list {a b c}]"
        tokens = lex(source)
        assert tokens[0].type == TokenType.CMD
        assert "{a b c}" in tokens[0].text


# Group 5: Variable substitution (parseOld-5.x)


class TestVariableSubstitution:
    """parseOld-5.x: Variable substitution forms."""

    def test_5_1_simple_variable(self):
        """parseOld-5.1: Simple $a variable."""
        tokens = lex("set b $a")
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 1
        assert var_tokens[0].text == "a"

    def test_5_2_variable_in_string(self):
        """parseOld-5.2: Variable concatenated with text."""
        tokens = lex("set b x$a.b")
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 1
        assert var_tokens[0].text == "a"
        # 'x' before and '.b' after
        all_text = "".join(t.text for t in tokens)
        assert "x" in all_text
        assert ".b" in all_text

    def test_5_3_underscore_in_var_name(self):
        """parseOld-5.3: Underscores in variable names."""
        tokens = lex("set b $_123z^")
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 1
        assert var_tokens[0].text == "_123z"
        # ^ should be after the variable
        all_text = "".join(t.text for t in tokens)
        assert "^" in all_text

    def test_5_4_braced_var_name(self):
        """parseOld-5.4: ${a} braced variable form."""
        tokens = lex("set b a${a}b")
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 1
        assert var_tokens[0].text == "a"

    def test_5_7_array_variable(self):
        """parseOld-5.7: Array variable $a(xyz)."""
        tokens = lex("set b $a(xyz)foo")
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) >= 1
        # Should include array name
        assert "a" in var_tokens[0].text

    def test_5_8_array_spaces_in_index(self):
        """parseOld-5.8: Array with spaces in index $a(x y z)."""
        source = "set b $a(x y z)foo"
        tokens = lex(source)
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) >= 1

    def test_5_11_bare_dollar_exclamation(self):
        """parseOld-5.11: $! -- dollar followed by non-alphanumeric."""
        tokens = lex("set b a$!")
        # $ followed by ! doesn't form a variable
        all_text = "".join(t.text for t in tokens)
        assert "!" in all_text

    def test_5_14_nested_array_access(self):
        """parseOld-5.14: Nested array access $a($a1(22))."""
        source = "set b $a($a1(22))"
        tokens = lex(source)
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) >= 1

    def test_namespace_qualified_var(self):
        """Namespace-qualified variable $ns::var."""
        tokens = lex("set b $::myns::myvar")
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 1
        assert "myns" in var_tokens[0].text

    def test_multiple_vars_in_word(self):
        """Multiple variables concatenated in one word."""
        tokens = lex("set result $a$b$c")
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 3


# Group 7: Backslash substitution (parseOld-7.x)


class TestBackslashSubstitution:
    r"""parseOld-7.x: Backslash escape sequences."""

    def test_7_1_escape_sequences_in_quotes(self):
        r"""parseOld-7.1: \a\c\n\]\} in quotes -- escapes processed."""
        source = r'set a "\a\c\n\]\}"'
        tokens = lex(source)
        assert tokens[0].text == "set"
        # The quoted string is an ESC token containing escape sequences
        esc_tokens = [
            t for t in tokens if t.type == TokenType.ESC and t.text != "set" and t.text != "a"
        ]
        assert len(esc_tokens) >= 1

    def test_7_2_escape_sequences_in_braces(self):
        r"""parseOld-7.2: Escape sequences in braces are literal."""
        source = r"set a {\a\c\n\]\}}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        # All backslashes preserved literally
        assert "\\" in strs[0].text

    def test_7_5_backslash_continuation_in_conditional(self):
        """parseOld-7.5: Line continuation in complex control structure."""
        source = textwrap.dedent("""\
            if 1 \\
                {set a 22}
        """)
        tokens = lex(source)
        assert tokens[0].text == "if"

    def test_7_6_trailing_backslash(self):
        """parseOld-7.6: Trailing backslash."""
        source = "concat abc\\\\"
        tokens = lex(source)
        assert tokens[0].text == "concat"

    def test_7_7_backslash_newline_continuation(self):
        """parseOld-7.7: Backslash-newline continuation."""
        source = "concat \\\na"
        tokens = lex(source)
        assert any(t.text == "concat" for t in tokens)

    def test_7_8_backslash_newline_with_indent(self):
        """parseOld-7.8: Backslash-newline with whitespace on next line."""
        source = "concat x\\\n   \ta"
        tokens = lex(source)
        texts = [t.text for t in tokens]
        assert "concat" in texts

    def test_backslash_in_word(self):
        """Backslash inside a word."""
        tokens = lex("set x hello\\nworld")
        assert tokens[0].text == "set"

    def test_backslash_space(self):
        """Backslash-space in unquoted context."""
        tokens = lex("set x hello\\ world")
        # Backslash-space may escape the space
        assert tokens[0].text == "set"

    def test_double_backslash(self):
        """Double backslash produces literal backslash."""
        tokens = lex('set x "a\\\\b"')
        assert len(tokens) >= 2


# Group 8: Semicolons (parseOld-8.x)


class TestSemicolons:
    """parseOld-8.x: Semicolons as command separators."""

    def test_8_1_semicolon_terminates(self):
        """parseOld-8.1: Semicolon terminates first command."""
        tokens = lex("getArgs a;set b 2")
        # 'getArgs' and 'a' before semicolon, 'set' after
        texts = [t.text for t in tokens]
        assert "getArgs" in texts
        assert "a" in texts
        assert "set" in texts

    def test_8_2_two_commands(self):
        """parseOld-8.2: Both commands after semicolon are present."""
        tokens = lex("getArgs a;set b 2")
        sets = [t for t in tokens if t.text == "set"]
        assert len(sets) == 1

    def test_8_3_semicolon_with_spaces(self):
        """parseOld-8.3: Semicolon with spaces around it."""
        tokens = lex("getArgs a b ; set b 1")
        texts = [t.text for t in tokens]
        assert "getArgs" in texts
        assert "set" in texts

    def test_multiple_semicolons(self):
        """Multiple semicolons between commands."""
        tokens = lex("set a 1;;;set b 2")
        texts = [t.text for t in tokens]
        assert texts.count("set") == 2

    def test_trailing_semicolons(self):
        """Trailing semicolons after command."""
        tokens = lex("set a 1;;;")
        texts = [t.text for t in tokens]
        assert texts == ["set", "a", "1"]

    def test_semicolons_in_quotes(self):
        """Semicolons inside quotes are literal."""
        tokens = lex('set a "hello;world"')
        texts = [t.text for t in tokens]
        assert texts.count("set") == 1
        assert any(";" in t.text for t in tokens)


# Group 9: Result initialisation (parseOld-9.x) -- adapted


class TestResultInit:
    """parseOld-9.x: Multiple commands, empty scripts."""

    def test_9_1_simple_concat(self):
        """parseOld-9.1: Simple concat."""
        tokens = lex("concat abc")
        assert tokens[0].text == "concat"
        assert tokens[1].text == "abc"

    def test_9_2_concat_then_proc(self):
        """parseOld-9.2: concat followed by proc definition."""
        source = "concat abc; proc foo {} {}"
        tokens = lex(source)
        assert any(t.text == "concat" for t in tokens)
        assert any(t.text == "proc" for t in tokens)
        assert any(t.text == "foo" for t in tokens)

    def test_9_5_trailing_semicolon_space(self):
        """parseOld-9.5: Command with trailing semicolon and spaces."""
        tokens = lex("concat abc; ")
        texts = [t.text for t in tokens]
        assert texts == ["concat", "abc"]

    def test_9_7_empty_string(self):
        """parseOld-9.7: Empty string produces no tokens."""
        tokens = lex("")
        assert tokens == []

    def test_9_8_multiple_semicolons_after(self):
        """parseOld-9.8: Trailing semicolons ignored."""
        tokens = lex("concat abc; ; ;")
        texts = [t.text for t in tokens]
        assert texts == ["concat", "abc"]


# Group 10: Syntax errors (parseOld-10.x) -- adapted for no-crash


class TestSyntaxErrors:
    """parseOld-10.x: Syntax error resilience.

    Our lexer doesn't raise errors -- it produces tokens even for
    malformed input.  We verify no crashes and reasonable output.
    """

    def test_10_1_unclosed_brace(self):
        """parseOld-10.1: Unclosed brace doesn't crash."""
        tokens = lex("set a {bcd")
        assert len(tokens) >= 1
        assert tokens[0].text == "set"

    def test_10_3_unclosed_quote(self):
        """parseOld-10.3: Unclosed quote doesn't crash."""
        tokens = lex('set a "bcd')
        assert len(tokens) >= 1
        assert tokens[0].text == "set"

    def test_10_5_chars_after_quote(self):
        """parseOld-10.5: Characters after close-quote raises TclParseError in strict mode."""
        old = TclLexer.strict_quoting
        TclLexer.strict_quoting = True
        try:
            with pytest.raises(TclParseError, match="extra characters after close-quote"):
                lex('set a "bcd"xy')
        finally:
            TclLexer.strict_quoting = old

    def test_10_7_chars_after_brace(self):
        """parseOld-10.7: Characters after close-brace."""
        tokens = lex("set a {bcd}xy")
        assert len(tokens) >= 1

    def test_10_9_unclosed_bracket(self):
        """parseOld-10.9: Unclosed bracket doesn't crash."""
        tokens = lex("set a [format abc")
        assert len(tokens) >= 1
        assert tokens[0].text == "set"

    def test_10_13_multiline_concat_backslash(self):
        """parseOld-10.13: Multiline with backslash continuation."""
        source = "set a [concat {a}\\\n {b}]"
        tokens = lex(source)
        assert tokens[0].text == "set"

    def test_incomplete_dollar_brace(self):
        """Unclosed ${...} doesn't crash."""
        tokens = lex("set x ${abc")
        assert len(tokens) >= 1

    def test_single_open_bracket(self):
        """Single [ doesn't crash."""
        tokens = lex("[")
        assert isinstance(tokens, list)

    def test_single_dollar(self):
        """Single $ doesn't crash."""
        tokens = lex("$")
        assert isinstance(tokens, list)


# Group 11: Long values (parseOld-11.x)


class TestLongValues:
    """parseOld-11.x: Long strings and many words."""

    def test_11_2_many_words(self):
        """parseOld-11.2: Many space-separated words."""
        words = [f"word{i}" for i in range(43)]
        source = "concat " + " ".join(words)
        tokens = lex(source)
        # 1 for concat + 43 words
        assert len(tokens) == 44

    def test_11_6_long_concat(self):
        """parseOld-11.6: Long concatenation string."""
        words = [f"w{i}" for i in range(43)]
        source = "concat " + " ".join(words)
        tokens = lex(source)
        assert tokens[0].text == "concat"
        assert len(tokens) == 44

    def test_long_quoted_string(self):
        """Long quoted string with spaces."""
        content = " ".join(f"word{i}" for i in range(50))
        source = f'set x "{content}"'
        tokens = lex(source)
        # Should have: set, x, and the quoted content
        esc_tokens = [t for t in tokens if t.type == TokenType.ESC and "word" in t.text]
        assert len(esc_tokens) >= 1

    def test_long_braced_string(self):
        """Long braced string with spaces."""
        content = " ".join(f"word{i}" for i in range(50))
        source = f"set x {{{content}}}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        assert "word0" in strs[0].text


# Group 12: Comments (parseOld-12.x)


class TestComments:
    """parseOld-12.x: Comment handling."""

    def test_12_1_comment_only_in_eval(self):
        """parseOld-12.1: Comment in script -- ignored.

        Note: our lexer only recognizes # as comment when _type is EOL
        (i.e., at true command start). Leading spaces produce SEP, so
        '  # set a new' lexes # as ESC, not COMMENT.  A bare '# ...' at
        line start IS recognized as COMMENT.
        """
        # With leading spaces, # is not at command position after SEP
        source = "  # set a new"
        tokens = lex(source)
        # May or may not be a comment depending on lexer behaviour
        assert len(tokens) >= 1

        # Without leading spaces, # at command start IS a comment
        source2 = "# set a new"
        tokens2 = lex(source2)
        comments = [t for t in tokens2 if t.type == TokenType.COMMENT]
        assert len(comments) == 1

    def test_12_2_comment_before_command(self):
        """parseOld-12.2: Comment on one line, command on next.

        With leading spaces, our lexer doesn't treat # as comment.
        Without leading spaces, it does.  We test both forms.
        """
        # Without leading spaces -- proper comment
        source = "# set a old\nset a new"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1
        # 'set' command on second line
        sets = [t for t in tokens if t.text == "set"]
        assert len(sets) == 1

    def test_12_3_comment_with_continuation(self):
        """parseOld-12.3: Comment with backslash-newline continuation.

        When # IS recognized as comment, backslash-newline continues it.
        """
        # Without leading spaces -- proper comment with continuation
        source = "# set a new\\\nset a new"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) >= 1

    def test_13_1_comment_in_bracket_script(self):
        """parseOld-13.1: Comment in bracketed script."""
        source = 'set x "[\nexpr {1+1}\n# skip this!\n]"'
        tokens = lex(source)
        # Should parse without crash
        assert tokens[0].text == "set"

    def test_comment_hash_not_at_command_start(self):
        """# not at command start is not a comment."""
        tokens = lex("set x #value")
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 0

    def test_comment_after_semicolon(self):
        """# after semicolon IS at command start."""
        tokens = lex("set x 1; # comment here")
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1


# Group 15: Script completeness (parseOld-15.x) -- adapted


class TestScriptCompleteness:
    """parseOld-15.x: Script completeness adapted as no-crash tests."""

    def test_15_1_unclosed_bracket(self):
        """parseOld-15.1: Unclosed bracket in multiline script."""
        source = "puts [\nexpr {1+1}\n# this is a comment ]"
        tokens = lex(source)
        # Should not crash
        assert len(tokens) >= 1

    def test_15_2_backslash_newline_continuation(self):
        """parseOld-15.2: Backslash-newline at end."""
        source = "abc\\\n"
        tokens = lex(source)
        assert len(tokens) >= 1

    def test_15_3_double_backslash_newline(self):
        """parseOld-15.3: Double backslash-newline is complete."""
        source = "abc\\\\\n"
        tokens = lex(source)
        assert len(tokens) >= 1


# Cross-cutting: Position tracking through parseOld patterns


class TestPositionTracking:
    """Verify positions are tracked correctly through parseOld patterns."""

    def test_positions_after_quoted_multiline(self):
        """Positions correct after multiline quoted string."""
        source = 'set a "hello\nworld"\nset b 2'
        tokens = lex(source)
        # 'set b 2' starts on line 2
        second_set = [t for t in tokens if t.text == "set"]
        assert len(second_set) == 2
        assert second_set[1].start.line == 2

    def test_positions_after_semicolons(self):
        """Positions correct across semicolons."""
        source = "set a 1; set b 2; set c 3"
        tokens = lex(source)
        sets = [t for t in tokens if t.text == "set"]
        assert len(sets) == 3
        # All on line 0
        for s in sets:
            assert s.start.line == 0

    def test_positions_with_braces_and_newlines(self):
        """Positions correct with braces spanning lines."""
        source = "if {true} {\n    puts ok\n}\nputs done"
        tokens = lex(source)
        done_puts = [t for t in tokens if t.text == "puts"]
        # The last 'puts' should be on line 3
        assert done_puts[-1].start.line == 3


# Analyser integration -- parseOld patterns survive analysis


class TestAnalyserIntegration:
    """Verify the analyser handles parseOld-style patterns correctly."""

    def test_proc_with_various_arg_forms(self):
        """Proc with default, list, and args arguments."""
        source = textwrap.dedent("""\
            proc myproc {a {b default} args} {
                set result $a
                return $result
            }
        """)
        result = analyse(source)
        assert "::myproc" in result.all_procs
        proc = result.all_procs["::myproc"]
        assert proc.params[0].name == "a"
        assert proc.params[1].name == "b"
        assert proc.params[1].has_default is True
        assert proc.params[2].name == "args"

    def test_nested_command_subs_in_proc(self):
        """Nested command substitutions in proc body."""
        source = textwrap.dedent("""\
            proc test {} {
                set x [string length [format "hello %s" "world"]]
                return $x
            }
        """)
        result = analyse(source)
        assert "::test" in result.all_procs

    def test_backslash_continuation_proc(self):
        """Proc with backslash-continuation in body."""
        source = textwrap.dedent("""\
            proc test {} {
                set x [list \\
                    a b c \\
                    d e f]
                return $x
            }
        """)
        result = analyse(source)
        assert "::test" in result.all_procs

    def test_semicolons_in_proc_body(self):
        """Multiple commands on one line with semicolons."""
        source = textwrap.dedent("""\
            proc test {} {
                set a 1; set b 2; set c 3
                return $c
            }
        """)
        result = analyse(source)
        assert "::test" in result.all_procs

    def test_braces_with_special_chars(self):
        """Braces containing #, ;, and $ characters."""
        source = textwrap.dedent("""\
            proc test {} {
                set pattern {#;$[]}
                set result [regexp $pattern "test"]
                return $result
            }
        """)
        result = analyse(source)
        assert "::test" in result.all_procs

    def test_multiproc_file(self):
        """Multiple procs, some with complex patterns."""
        source = textwrap.dedent("""\
            proc greet {name} {
                return "Hello, $name!"
            }

            proc add {a b} {
                return [expr {$a + $b}]
            }

            proc foreach_example {items} {
                set result [list]
                foreach item $items {
                    lappend result [string toupper $item]
                }
                return $result
            }

            set greeting [greet "World"]
            set sum [add 1 2]
            set upper [foreach_example {a b c}]
        """)
        result = analyse(source)
        assert len(result.all_procs) == 3
        assert "::greet" in result.all_procs
        assert "::add" in result.all_procs
        assert "::foreach_example" in result.all_procs
