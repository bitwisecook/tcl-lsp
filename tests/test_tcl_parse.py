"""Tests recreated from Tcl's official tests/parse.test.

These tests verify our TclLexer's behaviour against the parsing behaviours
specified in the official Tcl test suite (https://github.com/tcltk/tcl/blob/main/tests/parse.test).

Since our lexer is a tokeniser (not a full interpreter), we adapt the tests:
- Tests about command parsing become tests about correct tokenisation
- Tests about error handling verify no crashes and correct token production
- Tests about string length computation verify correct position tracking
- Tests about evaluation are adapted to verify correct token types/text
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.parsing.lexer import TclLexer
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


# Group 1: Tcl_ParseCommand -- string length and basic parsing (parse-1.x)


class TestParseCommand:
    """parse-1.x: Basic command parsing, leading space, null bytes."""

    def test_1_1_null_bytes_in_string(self):
        """parse-1.1: Computing string length with null bytes."""
        source = "foo\x00 bar"
        tokens = lex(source)
        assert len(tokens) > 0
        assert tokens[0].text == "foo\x00"

    def test_1_2_simple_command(self):
        """parse-1.2: Computing string length."""
        tokens = lex("foo bar")
        assert len(tokens) == 2
        assert tokens[0].text == "foo"
        assert tokens[1].text == "bar"

    def test_1_3_leading_whitespace(self):
        """parse-1.3: Leading space handling."""
        source = "  \n\t   foo"
        tokens = lex(source)
        assert tokens[0].text == "foo"

    def test_1_4_leading_special_whitespace(self):
        """parse-1.4: Form feed, carriage return, vertical tab before command."""
        source = "\f\r\vfoo"
        tokens = lex(source)
        assert len(tokens) > 0
        # \f and \v are not standard whitespace for Tcl separators

    def test_1_5_backslash_newline_leading_space(self):
        """parse-1.5: Backslash-newline in leading space."""
        source = "  \\\n foo"
        tokens = lex(source)
        assert any(t.text == "foo" for t in tokens)

    def test_1_7_missing_continuation_line(self):
        """parse-1.7: Missing continuation line in leading space."""
        source = "   \\\n"
        tokens = lex(source)
        # Should not crash -- may produce empty or escaped tokens
        assert isinstance(tokens, list)

    def test_1_8_eof_with_leading_space(self):
        """parse-1.8: EOF after leading space."""
        source = "      foo"
        tokens = lex(source)
        assert tokens[-1].text == "foo"

    def test_1_9_backslash_newline_newline(self):
        """parse-1.9: Backslash-newline + newline gives two commands."""
        source = "cmd1\\\n\ncmd2"
        tokens = lex(source)
        texts = [t.text for t in tokens]
        assert "cmd2" in texts

    def test_1_10_backslash_newline_multiple_words(self):
        """parse-1.10: Backslash-newline with multiple words.

        Note: our lexer treats backslash-newline as part of the word text
        (the escape is embedded in the ESC token text), so 'A' appears as
        part of a continuation token like '\\\\\\nA'.
        """
        source = "list \\\nA B\\\n\nlist C D"
        tokens = lex(source)
        texts = [t.text for t in tokens]
        assert texts.count("list") == 2
        assert "C" in texts
        assert "D" in texts


# Group 2: Comment processing (parse-2.x)


class TestComments:
    """parse-2.x: Comment processing."""

    def test_2_1_simple_comment(self):
        """parse-2.1: Simple comment followed by command."""
        source = "# foo bar\n foo"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1
        non_comment = [t for t in tokens if t.type not in (TokenType.COMMENT,)]
        assert any(t.text == "foo" for t in non_comment)

    def test_2_2_multiple_comments(self):
        """parse-2.2: Multiple comments before a command."""
        source = "# comment1\n# comment2\nfoo"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 2

    def test_2_3_backslash_newline_in_comment(self):
        """parse-2.3: Backslash-newline in comments (continuation)."""
        source = "# hello \\\nworld\nfoo"
        tokens = lex(source)
        # The continuation line 'world' is swallowed into the comment.
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1
        assert "hello" in comments[0].text
        assert "world" in comments[0].text
        # 'foo' is still a valid command after the continued comment.
        non_comment = [t for t in tokens if t.type not in (TokenType.COMMENT, TokenType.EOL)]
        assert any(t.text == "foo" for t in non_comment)

    def test_2_3b_chained_continuation_in_comment(self):
        """Multiple backslash-newline continuations in a comment."""
        source = "# line1 \\\nline2 \\\nline3\nfoo"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1
        assert "line1" in comments[0].text
        assert "line2" in comments[0].text
        assert "line3" in comments[0].text
        non_comment = [t for t in tokens if t.type not in (TokenType.COMMENT, TokenType.EOL)]
        assert any(t.text == "foo" for t in non_comment)

    def test_2_3c_continuation_swallows_code(self):
        """A puts after backslash-continuation is swallowed into the comment."""
        source = '# TODO: fix this later\\\nputs "hello world"\nset x 1'
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1
        assert "puts" in comments[0].text
        # 'set x 1' should still be a valid command.
        non_comment = [
            t for t in tokens if t.type not in (TokenType.COMMENT, TokenType.EOL, TokenType.SEP)
        ]
        assert any(t.text == "set" for t in non_comment)

    def test_2_3d_comment_multiline_token_positions(self):
        """A continued comment token spans the correct lines."""
        source = "# hello \\\nworld\nfoo"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1
        assert comments[0].start.line == 0
        assert comments[0].end.line == 1

    def test_2_4_missing_continuation_in_comment(self):
        """parse-2.4: Missing continuation line in comment."""
        source = "#   \\\n"
        tokens = lex(source)
        # Should not crash; the continuation consumes EOF gracefully.
        assert isinstance(tokens, list)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1

    def test_2_5_eof_in_comment(self):
        """parse-2.5: EOF within a comment, then command on next line."""
        source = " # foo bar\nfoo"
        tokens = lex(source)
        # The '#' is only a comment at start of command -- with leading space
        # it's still in command position after the implicit EOL
        assert len(tokens) > 0

    def test_comment_not_in_word(self):
        """A # in mid-command is not a comment."""
        source = "set x #notacomment"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 0

    def test_comment_after_semicolon(self):
        """# after semicolon is a comment."""
        source = "set x 1; # this is a comment"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1

    def test_hash_in_braces_not_comment(self):
        """# inside braces is literal, not a comment."""
        source = "set x {# this is not a comment}"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 0
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any("#" in t.text for t in strs)


# Group 3: Word parsing and space skipping (parse-3.x)


class TestWordParsing:
    """parse-3.x: Word parsing with various separators."""

    def test_3_1_multiple_spaces_tabs(self):
        """parse-3.1: Multiple spaces/tabs between words."""
        tokens = lex("foo  bar\t\tx")
        texts = [t.text for t in tokens]
        assert texts == ["foo", "bar", "x"]

    def test_3_2_missing_continuation_line(self):
        """parse-3.2: Missing continuation line in leading space."""
        source = "abc  \\\n"
        tokens = lex(source)
        assert any(t.text == "abc" for t in tokens)

    def test_3_3_semicolon_separator(self):
        """parse-3.3: Command ends with semicolon."""
        source = "foo  ;  bar x"
        tokens = lex(source)
        texts = [t.text for t in tokens]
        assert "foo" in texts
        assert "bar" in texts
        assert "x" in texts

    def test_3_4_trailing_spaces(self):
        """parse-3.4: Command ends in trailing spaces."""
        tokens = lex("foo       ")
        texts = [t.text for t in tokens]
        assert texts == ["foo"]

    def test_3_5_quoted_words(self):
        """parse-3.5: Quoted words parsed as single units."""
        source = 'foo "a b c" d "efg"'
        tokens = lex(source)
        # "a b c" is a single token (ESC type)
        assert any("a b c" in t.text for t in tokens)

    def test_3_6_braced_words(self):
        """parse-3.6: Braces group words."""
        source = "foo {a $b [concat foo]} {c d}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any("$b" in t.text for t in strs)  # $ not substituted in braces
        assert any("c d" in t.text for t in strs)

    def test_3_7_concatenated_word(self):
        """parse-3.7: Concatenated word with $ in it."""
        source = "foo ${abc}"
        tokens = lex(source)
        assert tokens[0].text == "foo"
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) >= 1
        assert var_tokens[0].text == "abc"

    def test_words_with_semicolons(self):
        """Semicolons separate commands."""
        tokens = lex("set a 1; set b 2")
        texts = [t.text for t in tokens]
        assert texts.count("set") == 2

    def test_empty_command(self):
        """Multiple semicolons produce empty commands."""
        tokens = lex(";;;foo")
        texts = [t.text for t in tokens]
        assert "foo" in texts


# Group 4: Simple words (parse-4.x)


class TestSimpleWords:
    """parse-4.x: Individual word types."""

    def test_4_1_simple_word(self):
        """parse-4.1: Simple unquoted word."""
        tokens = lex("foo")
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.ESC
        assert tokens[0].text == "foo"

    def test_4_2_braced_word(self):
        """parse-4.2: Braced word."""
        tokens = lex("{abc}")
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.STR
        assert tokens[0].text == "abc"

    def test_4_3_quoted_word(self):
        """parse-4.3: Quoted word."""
        tokens = lex('"c d"')
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.ESC
        assert tokens[0].text == "c d"

    def test_4_4_word_with_variable(self):
        """parse-4.4: Word containing a variable reference."""
        tokens = lex("x$d")
        types = [t.type for t in tokens]
        # 'x' is ESC, '$d' is VAR -- they concatenate
        assert TokenType.ESC in types
        assert TokenType.VAR in types

    def test_4_5_word_with_command_sub(self):
        """parse-4.5: Word with command substitution."""
        tokens = lex('"a [foo] b"')
        types = [t.type for t in tokens]
        assert TokenType.CMD in types

    def test_4_6_simple_variable(self):
        """parse-4.6: Simple variable reference."""
        tokens = lex("$x")
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "x"


# Group 5: Word terminators (parse-5.x)


class TestWordTerminators:
    """parse-5.x: Backslash-newline, newline, semicolon terminators."""

    def test_5_1_backslash_newline_after_brace(self):
        """parse-5.1: Backslash-newline after braced word."""
        source = "{abc}\\\nfoo"
        tokens = lex(source)
        assert any(t.text == "abc" for t in tokens)

    def test_5_2_backslash_newline_in_word(self):
        """parse-5.2: Backslash-newline continues a word."""
        source = "foo\\\nbar"
        tokens = lex(source)
        # foo and bar may be concatenated or separate depending on context
        assert len(tokens) >= 1

    def test_5_3_newline_terminates_word(self):
        """parse-5.3: Newline terminates a word."""
        source = "foo\n bar"
        tokens = lex(source)
        texts = [t.text for t in tokens]
        assert "foo" in texts
        assert "bar" in texts

    def test_5_4_semicolon_terminates_word(self):
        """parse-5.4: Semicolon terminates a command."""
        source = "foo; bar"
        tokens = lex(source)
        texts = [t.text for t in tokens]
        assert "foo" in texts
        assert "bar" in texts

    def test_5_5_eof_terminates_word(self):
        """parse-5.5: EOF terminates the last word."""
        tokens = lex('"foo" bar')
        texts = [t.text for t in tokens]
        assert "foo" in texts
        assert "bar" in texts


# Group 5 continued: {*} Expansion (parse-5.11 through 5.31)


class TestExpansion:
    """parse-5.11+: {*} expansion prefix tests.

    The lexer emits EXPAND tokens for the ``{*}`` prefix (Tcl 8.5+).
    """

    def test_expansion_basic(self):
        """{*} followed by a variable."""
        source = "cmd {*}$args"
        tokens = lex(source)
        assert tokens[0].text == "cmd"
        assert any(t.type == TokenType.EXPAND for t in tokens)
        assert any(t.type == TokenType.VAR for t in tokens)

    def test_expansion_with_list(self):
        """{*} with a braced list argument."""
        source = "cmd {*}{a b c}"
        tokens = lex(source)
        assert any(t.type == TokenType.EXPAND for t in tokens)
        assert any(t.type == TokenType.STR and t.text == "a b c" for t in tokens)

    def test_expansion_with_quotes(self):
        """{*} with a quoted argument."""
        source = 'cmd {*}"a b c"'
        tokens = lex(source)
        assert any(t.type == TokenType.EXPAND for t in tokens)
        assert any(t.type == TokenType.ESC and t.text == "a b c" for t in tokens)

    def test_expansion_with_bracket(self):
        """{*} with a command substitution."""
        source = "cmd {*}[list a b]"
        tokens = lex(source)
        assert any(t.type == TokenType.EXPAND for t in tokens)
        assert any(t.type == TokenType.CMD for t in tokens)

    # -- Edge cases: {*} should NOT expand --

    def test_no_expand_brace_star_space(self):
        """{* } is a braced string, not expansion (space inside)."""
        source = "cmd {* }"
        tokens = lex(source)
        assert not any(t.type == TokenType.EXPAND for t in tokens)
        assert any(t.type == TokenType.STR and t.text == "* " for t in tokens)

    def test_no_expand_nested_brace_star(self):
        """{a{*}} is a braced string containing literal {*}."""
        source = "cmd {a{*}}"
        tokens = lex(source)
        assert not any(t.type == TokenType.EXPAND for t in tokens)
        # The entire thing is a braced word
        assert any(t.type == TokenType.STR for t in tokens)

    def test_no_expand_at_eol(self):
        """{*} followed by separator or EOL is a braced string, not expansion."""
        source = "cmd {*}"
        tokens = lex(source)
        # {*} at word boundary followed by end of input → braced string "*"
        assert not any(t.type == TokenType.EXPAND for t in tokens)
        assert any(t.type == TokenType.STR and t.text == "*" for t in tokens)

    def test_no_expand_followed_by_space(self):
        """{*} followed by space is a braced string."""
        source = "cmd {*} arg"
        tokens = lex(source)
        assert not any(t.type == TokenType.EXPAND for t in tokens)
        assert any(t.type == TokenType.STR and t.text == "*" for t in tokens)

    def test_no_expand_in_84_dialect(self):
        """{*} should not expand when expand_syntax is disabled (Tcl 8.4 mode)."""
        from core.parsing.lexer import TclLexer

        old = TclLexer.expand_syntax
        TclLexer.expand_syntax = False
        try:
            tokens = lex("cmd {*}$args")
            assert not any(t.type == TokenType.EXPAND for t in tokens)
            # Should be treated as braced string "*" concatenated with $args
            assert any(t.type == TokenType.STR and t.text == "*" for t in tokens)
        finally:
            TclLexer.expand_syntax = old

    def test_expansion_followed_by_semicolon(self):
        """{*} followed by semicolon is a braced string, not expansion."""
        source = "cmd {*};"
        tokens = lex(source)
        assert not any(t.type == TokenType.EXPAND for t in tokens)


# Group 6: ParseTokens (parse-6.x)


class TestParseTokens:
    """parse-6.x: Token parsing within strings/quoted words."""

    def test_6_1_empty_quoted_string(self):
        """parse-6.1: Empty quoted word."""
        tokens = lex('""')
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.ESC
        assert tokens[0].text == ""

    def test_6_2_quoted_string_with_var(self):
        """parse-6.2: Quoted string with variable."""
        tokens = lex('"abc$x.e"')
        types = [t.type for t in tokens]
        assert TokenType.VAR in types

    def test_6_3_variable_references(self):
        """parse-6.3: Multiple variable references."""
        tokens = lex("abc$x.e $y(z)")
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) >= 2

    def test_6_5_command_substitution(self):
        """parse-6.5: Command substitution in word."""
        tokens = lex("[foo $x bar]z")
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) >= 1
        # 'z' is concatenated after the CMD
        all_text = "".join(t.text for t in tokens)
        assert "z" in all_text

    def test_6_6_escaped_bracket_in_cmd(self):
        """parse-6.6: Escaped bracket inside command substitution."""
        source = r"[foo \] [a b]]"
        tokens = lex(source)
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) >= 1

    def test_6_14_backslash_newline_unquoted(self):
        """parse-6.14: Backslash-newline in unquoted context."""
        source = "b\\\nc"
        tokens = lex(source)
        assert len(tokens) >= 1
        # b and c may be one or two tokens, but no crash
        all_text = "".join(t.text for t in tokens)
        assert "b" in all_text
        assert "c" in all_text

    def test_6_15_backslash_newline_quoted(self):
        """parse-6.15: Backslash-newline in quoted context."""
        source = '"b\\\nc"'
        tokens = lex(source)
        assert len(tokens) >= 1
        all_text = "".join(t.text for t in tokens)
        assert "b" in all_text

    def test_6_16_backslash_sequences(self):
        r"""parse-6.16: Backslash substitution (\n\a\x7f)."""
        source = r'"\n\a\x7f"'
        tokens = lex(source)
        assert len(tokens) >= 1
        # The text contains the literal backslash sequences
        assert tokens[0].type == TokenType.ESC

    def test_6_17_null_chars_in_parsing(self):
        """parse-6.17: Null characters in parsing."""
        source = "set x \x00"
        tokens = lex(source)
        assert tokens[0].text == "set"


# Group 7: Memory management (parse-7.x) -- adapted as stress test


class TestMemoryManagement:
    """parse-7.x: Tests that triggered memory allocation."""

    def test_7_1_repeated_variables(self):
        """parse-7.1: Many variable references force token array expansion."""
        vars_list = " ".join(f"${chr(97 + i % 26)}{i}" for i in range(50))
        source = f"cmd {vars_list}"
        tokens = lex(source)
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 50


# Group 10: Tcl_EvalTokens (parse-10.x) -- adapted for tokenisation


class TestEvalTokens:
    """parse-10.x: Token evaluation -- adapted to verify token production."""

    def test_10_1_simple_text(self):
        """parse-10.1: Simple text tokens."""
        tokens = lex("concat foo")
        assert tokens[0].text == "concat"
        assert tokens[1].text == "foo"

    def test_10_2_backslash_sequences(self):
        r"""parse-10.2: Octal escapes \063\062."""
        source = r'set x "\063\062"'
        tokens = lex(source)
        assert tokens[0].text == "set"

    def test_10_3_nested_command(self):
        """parse-10.3: Nested command substitution [expr {2 + 6}]."""
        tokens = lex("[expr {2 + 6}]")
        assert tokens[0].type == TokenType.CMD
        assert "expr" in tokens[0].text

    def test_10_5_simple_variable(self):
        """parse-10.5: Simple variable $a."""
        tokens = lex("$a")
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "a"

    def test_10_6_array_variable(self):
        """parse-10.6: Array variable $a(12)."""
        tokens = lex("$a(12)")
        assert tokens[0].type == TokenType.VAR
        # The text should include the array name and index
        assert "a" in tokens[0].text

    def test_10_7_array_computed_index(self):
        """parse-10.7: Array with computed index $a(1[expr {3-1}])."""
        source = "$a(1[expr {3-1}])"
        tokens = lex(source)
        assert tokens[0].type == TokenType.VAR

    def test_10_11_multiple_variables(self):
        """parse-10.11: Multiple variable references $a$a$a."""
        tokens = lex("$a$a$a")
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 3

    def test_10_12_multiple_commands(self):
        """parse-10.12: Multiple command substitutions."""
        source = "[expr {2}][expr {4}][expr {6}]"
        tokens = lex(source)
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) == 3

    def test_10_13_quoted_with_embedded_quotes(self):
        """parse-10.13: Handling of quotes in text."""
        source = 'set x {a" b"}'
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any('"' in t.text for t in strs)

    def test_10_14_concat_with_variables(self):
        """parse-10.14: String with multiple var references."""
        source = "set result x$a.$a.$a"
        tokens = lex(source)
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 3


# Group 11: Tcl_EvalEx (parse-11.x) -- adapted for tokenisation


class TestEvalEx:
    """parse-11.x: Multiple commands, errors, memory."""

    def test_11_7_multiple_commands(self):
        """parse-11.7: Two commands separated by semicolon."""
        source = "set a b; set c d"
        tokens = lex(source)
        sets = [t for t in tokens if t.text == "set"]
        assert len(sets) == 2

    def test_11_8_newline_separated(self):
        """parse-11.8: Commands separated by newlines."""
        source = "set a b\nset c d"
        tokens = lex(source)
        sets = [t for t in tokens if t.text == "set"]
        assert len(sets) == 2

    def test_11_10_trailing_semicolon(self):
        """parse-11.10: Trailing semicolon with spaces."""
        source = "concat xyz;   "
        tokens = lex(source)
        assert tokens[0].text == "concat"
        assert tokens[1].text == "xyz"

    def test_11_11_empty_commands_and_comments(self):
        """parse-11.11: Empty statements and comments."""
        source = "  ;\n; # hello\n  "
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1

    def test_11_12_empty_script(self):
        """parse-11.12: Empty script."""
        tokens = lex("")
        assert tokens == []


# Group 12: Tcl_ParseVarName (parse-12.x)


class TestParseVarName:
    """parse-12.x: Variable name parsing."""

    def test_12_3_simple_var(self):
        """parse-12.3: Simple variable $abcd."""
        tokens = lex("$abcd")
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "abcd"

    def test_12_6_braced_var(self):
        """parse-12.6: Braced variable name ${..[]b}."""
        source = "${..[]b}"
        tokens = lex(source)
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "..[]b"

    def test_12_7_braced_var_with_backslash(self):
        r"""parse-12.7: Braced variable ${{\\}}."""
        source = "${{\\\\.}}"
        tokens = lex(source)
        assert tokens[0].type == TokenType.VAR

    def test_12_11_var_name_chars(self):
        """parse-12.11: Variable names with alphanumeric and underscores."""
        tokens = lex("$az_AZ")
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "az_AZ"

    def test_12_13_namespace_var(self):
        """parse-12.13: Namespace-qualified variable $xyz::ab."""
        tokens = lex("$xyz::ab")
        assert tokens[0].type == TokenType.VAR
        assert tokens[0].text == "xyz::ab"

    def test_12_14_multiple_colons(self):
        """parse-12.14: Multiple colons $xyz:::::c."""
        tokens = lex("$xyz:::::c")
        assert tokens[0].type == TokenType.VAR

    def test_12_15_single_colon(self):
        """parse-12.15: Single colon stops var name $ab:cd."""
        tokens = lex("$ab:cd")
        assert tokens[0].type == TokenType.VAR
        # Single colon is NOT a namespace separator
        assert tokens[0].text == "ab"

    def test_12_18_bare_dollar(self):
        """parse-12.18: Bare dollar signs $$ $."""
        tokens = lex("$$ $.")
        # Bare $ is not a variable -- treated as string
        assert len(tokens) >= 1

    def test_12_20_array_reference(self):
        """parse-12.20: Array reference $x(abc)."""
        tokens = lex("$x(abc)")
        assert tokens[0].type == TokenType.VAR
        # Text should include array name
        assert "x" in tokens[0].text

    def test_12_21_array_with_var_index(self):
        """parse-12.21: Array with variable in index $x(ab$cde[foo bar])."""
        source = "$x(ab$cde[foo bar])"
        tokens = lex(source)
        assert tokens[0].type == TokenType.VAR

    def test_12_25_nested_array(self):
        """parse-12.25: Nested array reference $x(a$y(b))."""
        source = "$x(a$y(b))"
        tokens = lex(source)
        # Should not crash, produces VAR token(s)
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) >= 1


# Group 14: Tcl_ParseBraces (parse-14.x)


class TestParseBraces:
    """parse-14.x: Braced string parsing."""

    def test_14_1_null_in_braces(self):
        """parse-14.1: Null bytes in braces."""
        source = "{foo\x00 bar}"
        tokens = lex(source)
        assert tokens[0].type == TokenType.STR
        assert "foo" in tokens[0].text

    def test_14_3_nested_braces(self):
        """parse-14.3: Nested braces preserved."""
        source = "foo {a $b [concat foo]} {c d}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        # $ is not substituted inside braces
        assert any("$b" in t.text for t in strs)

    def test_14_4_empty_nested_braces(self):
        """parse-14.4: Empty nested braces."""
        source = "foo {{}}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        assert strs[0].text == "{}"

    def test_14_5_deeply_nested_braces(self):
        """parse-14.5: Deeply nested braces."""
        source = "foo {{a {b} c} {} {d e}}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1

    def test_14_6_backslash_in_braces(self):
        """parse-14.6: Backslashes in braced words are literal."""
        source = "foo {a \\n\\{}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        assert "\\n" in strs[0].text

    def test_14_8_backslash_newline_in_braces(self):
        """parse-14.8: Backslash-newline in braces."""
        source = "foo {\\\nx}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1

    def test_14_9_backslash_newline_spaces_in_braces(self):
        """parse-14.9: Backslash-newline with spaces in braces."""
        source = "foo {a \\\n   b}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1

    def test_14_10_backslash_newline_at_end_of_braces(self):
        """parse-14.10: Backslash-newline at end of braced content."""
        source = "foo {xyz\\\n }"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1

    def test_14_11_empty_braces(self):
        """parse-14.11: Empty braced string."""
        source = "foo {}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        assert strs[0].text == ""


# Group 15: Quoted strings and CommandComplete (parse-15.x)


class TestQuotedStrings:
    """parse-15.x: Quoted string parsing."""

    def test_15_1_simple_quoted(self):
        """Simple quoted string."""
        tokens = lex('"hello world"')
        assert tokens[0].type == TokenType.ESC
        assert tokens[0].text == "hello world"

    def test_15_2_quoted_with_variable(self):
        """Quoted string with variable substitution."""
        tokens = lex('"hello $name"')
        types = [t.type for t in tokens]
        assert TokenType.VAR in types

    def test_15_3_quoted_with_command(self):
        """Quoted string with command substitution."""
        tokens = lex('"value is [expr {1+1}]"')
        types = [t.type for t in tokens]
        assert TokenType.CMD in types

    def test_15_4_quoted_with_backslash(self):
        """Quoted string with backslash sequences."""
        tokens = lex('"hello\\nworld"')
        assert len(tokens) >= 1

    def test_15_5_incomplete_braces(self):
        """info complete: various completeness checks (adapted)."""
        # Verify we can lex incomplete-looking inputs without crashing
        for source in ["set x {", 'set x "hello', "set x [cmd"]:
            tokens = lex(source)
            assert isinstance(tokens, list)


# Group 16: Bug fixes (parse-16.x, parse-17.x)


class TestBugFixes:
    """parse-16.x through 17.x: Bug fixes."""

    def test_16_1_eval_return_with_text(self):
        """parse-16.1: [eval {return foo}]bar."""
        source = "[eval {return foo}]bar"
        tokens = lex(source)
        # CMD token + ESC 'bar' concatenated
        assert any(t.type == TokenType.CMD for t in tokens)
        all_text = "".join(t.text for t in tokens)
        assert "bar" in all_text

    def test_command_with_nested_braces_and_brackets(self):
        """Complex nesting doesn't corrupt state."""
        source = "if {[string match {*foo*} $bar]} {puts ok}"
        tokens = lex(source)
        assert tokens[0].text == "if"
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1


# Group 20: TclParseBackslash (parse-20.x)


class TestParseBackslash:
    """parse-20.x: Backslash escape handling."""

    def test_20_backslash_n(self):
        """Backslash-n in quoted string."""
        tokens = lex('"hello\\nworld"')
        assert len(tokens) >= 1
        text = "".join(t.text for t in tokens)
        assert "hello" in text

    def test_20_backslash_t(self):
        """Backslash-t in string."""
        tokens = lex('"a\\tb"')
        assert len(tokens) >= 1

    def test_20_backslash_unicode(self):
        """Backslash unicode escape."""
        tokens = lex('"\\u0041"')
        assert len(tokens) >= 1

    def test_20_backslash_hex(self):
        """Backslash hex escape."""
        tokens = lex('"\\x41"')
        assert len(tokens) >= 1

    def test_20_backslash_octal(self):
        """Backslash octal escape."""
        tokens = lex('"\\101"')
        assert len(tokens) >= 1

    def test_20_backslash_continuation(self):
        """Backslash-newline continuation."""
        source = "set x \\\n  42"
        tokens = lex(source)
        assert tokens[0].text == "set"

    def test_20_backslash_at_eof(self):
        """Backslash at end of input."""
        tokens = lex("set x \\")
        assert len(tokens) >= 1

    def test_20_double_backslash(self):
        """Double backslash produces literal backslash."""
        tokens = lex('"a\\\\b"')
        assert len(tokens) >= 1


# Position tracking tests (derived from parse test requirements)


class TestPositionTracking:
    """Verify position tracking matches parse.test expectations."""

    def test_simple_word_positions(self):
        """Positions for 'foo bar' are correct."""
        tokens = lex_all("foo bar")
        # foo starts at 0:0
        foo = tokens[0]
        assert foo.start.line == 0
        assert foo.start.character == 0
        assert foo.start.offset == 0
        assert foo.text == "foo"

    def test_multiline_positions(self):
        """Line tracking across newlines."""
        source = "foo\nbar\nbaz"
        tokens = lex(source)
        texts_with_lines = [(t.text, t.start.line) for t in tokens]
        assert ("foo", 0) in texts_with_lines
        assert ("bar", 1) in texts_with_lines
        assert ("baz", 2) in texts_with_lines

    def test_positions_after_comment(self):
        """Positions correct after skipping a comment."""
        source = "# comment\nfoo"
        tokens = lex(source)
        foo = [t for t in tokens if t.text == "foo"][0]
        assert foo.start.line == 1
        assert foo.start.character == 0

    def test_positions_in_quoted_multiline(self):
        """Positions track through multiline quoted strings."""
        source = '"hello\nworld" foo'
        tokens = lex(source)
        # 'foo' should be on line 1
        foo_tokens = [t for t in tokens if t.text == "foo"]
        assert len(foo_tokens) == 1
        assert foo_tokens[0].start.line == 1

    def test_positions_after_backslash_continuation(self):
        """Positions track through backslash-newline."""
        source = "set x \\\n  42\nputs ok"
        tokens = lex(source)
        puts = [t for t in tokens if t.text == "puts"][0]
        assert puts.start.line == 2

    def test_all_positions_monotonic(self):
        """Offsets increase monotonically through complex source."""
        source = textwrap.dedent("""\
            proc foo {a b} {
                set c [expr {$a + $b}]
                return $c
            }
            set x "hello world"
            puts [foo 1 2]
        """)
        tokens = TclLexer(source).tokenise_all()
        prev_offset = -1
        for tok in tokens:
            assert tok.start.offset >= 0
            assert tok.end.offset >= tok.start.offset or tok.text == ""
            assert tok.start.offset >= prev_offset
            prev_offset = tok.start.offset


# Completeness / edge cases from parse.test


class TestCompleteness:
    """Adapted from parse-15.5 through parse-15.60: Script completeness."""

    def test_empty_string(self):
        tokens = lex("")
        assert tokens == []

    def test_whitespace_only(self):
        tokens = lex("   \n\t  ")
        assert tokens == []

    def test_comment_only(self):
        tokens = lex("# just a comment")
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.COMMENT

    def test_incomplete_quote(self):
        """Incomplete quote doesn't crash."""
        tokens = lex('"hello')
        assert len(tokens) >= 1

    def test_incomplete_brace(self):
        """Incomplete brace doesn't crash."""
        tokens = lex("{hello")
        assert len(tokens) >= 1

    def test_incomplete_bracket(self):
        """Incomplete bracket doesn't crash."""
        tokens = lex("[hello")
        assert len(tokens) >= 1

    def test_nested_incomplete(self):
        """Nested incomplete structures."""
        tokens = lex('"hello [cmd {arg')
        assert len(tokens) >= 1

    def test_multiple_semicolons(self):
        tokens = lex(";;;")
        # All semicolons, no real commands
        non_sep = [t for t in tokens if t.type not in (TokenType.SEP, TokenType.EOL)]
        assert len(non_sep) == 0

    def test_semicolons_with_commands(self):
        tokens = lex("a; b; c")
        texts = [t.text for t in tokens]
        assert "a" in texts
        assert "b" in texts
        assert "c" in texts

    def test_brace_in_middle_of_word(self):
        """Braces in middle of word are literal."""
        tokens = lex("a{b}c")
        # This is a single word 'a{b}c' since braces only group at word start
        all_text = "".join(t.text for t in tokens)
        assert "a" in all_text

    def test_quote_in_middle_of_word(self):
        """Quotes in middle of word are literal."""
        tokens = lex('a"b"c')
        all_text = "".join(t.text for t in tokens)
        assert "b" in all_text

    def test_deeply_nested_structures(self):
        """Very deep nesting doesn't crash."""
        depth = 50
        source = "[" * depth + "cmd" + "]" * depth
        tokens = lex(source)
        assert len(tokens) >= 1

    def test_many_variables_in_string(self):
        """Many variables in a quoted string."""
        vars_str = " ".join(f"$v{i}" for i in range(20))
        source = f'puts "{vars_str}"'
        tokens = lex(source)
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 20

    def test_alternating_braces_and_commands(self):
        """Alternating braced and command tokens."""
        source = "set x {a}; set y [cmd]; set z {b}"
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        cmds = [t for t in tokens if t.type == TokenType.CMD]
        assert len(strs) >= 2
        assert len(cmds) >= 1


class TestVariableShapeMalformedNestedSubstitutions:
    """Malformed nested substitutions in mixed quote/braced contexts."""

    def test_quoted_missing_close_bracket_with_array_ref(self):
        tokens = lex('puts "value [set a(1)"')
        assert any(t.type == TokenType.CMD for t in tokens)
        assert any(t.type == TokenType.ESC and "value" in t.text for t in tokens)

    def test_quoted_missing_close_bracket_with_namespaced_array_ref(self):
        tokens = lex('puts "value [set ::ns::arr(k)"')
        assert any(t.type == TokenType.CMD for t in tokens)
        assert any(t.type == TokenType.ESC and "value" in t.text for t in tokens)

    def test_braced_word_keeps_scalar_like_array_name_literal(self):
        tokens = lex("puts {${a(1)} [set x}")
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any("${a(1)}" in t.text for t in strs)

    def test_braced_word_keeps_unbraced_array_ref_literal(self):
        tokens = lex("puts {$a(1) [set x}")
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any("$a(1)" in t.text for t in strs)
