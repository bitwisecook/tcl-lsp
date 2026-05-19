"""Tests for the position-aware Tcl lexer."""

from __future__ import annotations

import sys
from pathlib import Path

# Allow imports from the server package
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.lexer import TclLexer
from compiler.parsing.substitution import backslash_subst
from compiler.parsing.tokens import Token, TokenType


def _tokens(source: str, *, include_sep: bool = False) -> list[Token]:
    """Tokenise source, optionally filtering out SEP/EOL tokens."""
    lexer = TclLexer(source)
    result = []
    while True:
        tok = lexer.get_token()
        if tok is None:
            break
        if not include_sep and tok.type in (TokenType.SEP, TokenType.EOL):
            continue
        result.append(tok)
    return result


def _texts(source: str) -> list[str]:
    return [t.text for t in _tokens(source)]


def _types(source: str) -> list[TokenType]:
    return [t.type for t in _tokens(source)]


class TestBasicTokens:
    def test_simple_command(self):
        assert _texts("puts hello") == ["puts", "hello"]

    def test_types_simple(self):
        types = _types("puts hello")
        assert types == [TokenType.ESC, TokenType.ESC]

    def test_variable(self):
        toks = _tokens("$foo")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "foo"

    def test_bare_dollar(self):
        toks = _tokens("$")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "$"

    def test_command_substitution(self):
        toks = _tokens("[+ 1 2]")
        assert toks[0].type == TokenType.CMD
        assert toks[0].text == "+ 1 2"

    def test_braces(self):
        toks = _tokens("{hello world}")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "hello world"

    def test_quoted_string(self):
        toks = _tokens('"hello world"')
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "hello world"

    def test_nested_braces(self):
        toks = _tokens("{a {b c} d}")
        assert toks[0].text == "a {b c} d"

    def test_nested_brackets(self):
        toks = _tokens("[+ 1 [+ 2 3]]")
        assert toks[0].type == TokenType.CMD
        assert toks[0].text == "+ 1 [+ 2 3]"

    def test_semicolon_separator(self):
        toks = _tokens("set a 1; set b 2", include_sep=True)
        types = [t.type for t in toks]
        assert TokenType.EOL in types

    def test_comment(self):
        toks = _tokens("# this is a comment\nputs hi")
        # Comment should be included as a COMMENT token
        assert toks[0].type == TokenType.COMMENT
        assert "this is a comment" in toks[0].text

    def test_empty_input(self):
        toks = _tokens("")
        assert toks == []

    def test_multiple_commands(self):
        assert _texts("set x 42") == ["set", "x", "42"]


class TestExtendedVars:
    def test_braced_var(self):
        toks = _tokens("${my var}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "my var"

    def test_namespace_var(self):
        toks = _tokens("$ns::var")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "ns::var"

    def test_nested_namespace_var(self):
        toks = _tokens("$a::b::c")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "a::b::c"

    def test_array_var(self):
        toks = _tokens("$arr(idx)")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr(idx)"

    def test_array_with_namespace(self):
        toks = _tokens("$ns::arr(key)")
        assert toks[0].type == TokenType.VAR
        assert "ns::arr(key)" == toks[0].text


class TestPositions:
    def test_first_token_at_origin(self):
        toks = _tokens("puts hello")
        assert toks[0].start.line == 0
        assert toks[0].start.character == 0
        assert toks[0].start.offset == 0

    def test_second_token_offset(self):
        toks = _tokens("puts hello")
        # "hello" starts at offset 5
        assert toks[1].start.offset == 5
        assert toks[1].start.character == 5

    def test_multiline_positions(self):
        source = "set x 1\nset y 2"
        toks = _tokens(source)
        # "set" on second line
        second_set = [t for t in toks if t.text == "set"][1]
        assert second_set.start.line == 1
        assert second_set.start.character == 0

    def test_var_position(self):
        toks = _tokens("set x $y")
        var_tok = [t for t in toks if t.type == TokenType.VAR][0]
        # start captures the '$' position for highlighting purposes
        assert var_tok.start.offset == 6
        assert var_tok.text == "y"

    def test_cmd_position(self):
        toks = _tokens("set x [+ 1 2]")
        cmd = [t for t in toks if t.type == TokenType.CMD][0]
        assert cmd.text == "+ 1 2"

    def test_comment_position(self):
        source = "# comment\nputs hi"
        toks = _tokens(source)
        assert toks[0].type == TokenType.COMMENT
        assert toks[0].start.line == 0

    def test_end_position(self):
        toks = _tokens("puts")
        assert toks[0].end.character == 3  # 'p' at 0, 's' at 3
        assert toks[0].end.offset == 3

    def test_multiline_braced_string(self):
        source = "{line1\nline2}"
        toks = _tokens(source)
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "line1\nline2"
        # End should be on line 1
        assert toks[0].end.line == 1


class TestTclConstructs:
    def test_if_else(self):
        source = "if {== 1 1} {puts yes} else {puts no}"
        texts = _texts(source)
        assert texts[0] == "if"
        assert "else" in texts

    def test_while_loop(self):
        source = "while {< $i 10} {\n  set i [+ $i 1]\n}"
        toks = _tokens(source)
        assert toks[0].text == "while"

    def test_proc_definition(self):
        source = "proc double {x} {return [* $x 2]}"
        texts = _texts(source)
        assert texts[0] == "proc"
        assert texts[1] == "double"

    def test_string_interpolation(self):
        source = '"hello $name, result is [+ 1 2]!"'
        toks = _tokens(source)
        types = [t.type for t in toks]
        assert TokenType.VAR in types
        assert TokenType.CMD in types

    def test_backslash_in_string(self):
        source = '"line1\\nline2"'
        toks = _tokens(source)
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC

    def test_expr_body(self):
        source = "expr {$x + $y * 2}"
        toks = _tokens(source)
        assert toks[0].text == "expr"
        assert toks[1].type == TokenType.STR

    def test_namespace_eval(self):
        source = "namespace eval myns { proc foo {} {} }"
        texts = _texts(source)
        assert texts[0] == "namespace"

    def test_foreach(self):
        source = "foreach item $list { puts $item }"
        texts = _texts(source)
        assert texts[0] == "foreach"

    def test_switch(self):
        source = "switch $x { a {puts A} b {puts B} }"
        texts = _texts(source)
        assert texts[0] == "switch"


class TestTrailingWhitespace:
    """Trailing spaces before a newline must not swallow the EOL."""

    def test_sep_does_not_consume_newline(self):
        """Trailing spaces after '}' must not merge the next line into
        the same command (regression: _parse_sep consumed '\\n')."""
        source = "set a {body}    \nset b val"
        toks = _tokens(source, include_sep=True)
        types = [t.type for t in toks]
        # The newline must appear as an EOL, not be absorbed into a SEP.
        assert TokenType.EOL in types
        # Both 'set' commands must be separate.
        esc_texts = [t.text for t in toks if t.type == TokenType.ESC]
        assert esc_texts.count("set") == 2

    def test_command_after_brace_with_trailing_spaces(self):
        """A command on the line after a brace-body with trailing spaces
        must be tokenised as a separate command."""
        source = "switch -- $foo {\n    default {}\n}    \ntable set foo1 bar1"
        toks = _tokens(source)
        texts = [t.text for t in toks]
        # 'table' must appear as its own ESC token (new command),
        # not absorbed as an argument to switch.
        assert "table" in texts
        table_tok = next(t for t in toks if t.text == "table")
        assert table_tok.type == TokenType.ESC


class TestBackslashNewlineMidWord:
    """Regression tests for mid-word backslash-newline in unquoted context.

    When a backslash-newline appears mid-word (e.g. ``foo\\<newline>{bar}``),
    the lexer must emit a word boundary (SEP) so that ``{`` on the next line
    is recognised as a brace delimiter and tokenised as STR.
    """

    def test_midword_backslash_newline_brace_is_str(self):
        """foo\\<NL>{bar} must tokenise {bar} as STR, not absorb it."""
        source = "foo\\\n{bar}"
        toks = _tokens(source, include_sep=True)
        non_sep = [t for t in toks if t.type not in (TokenType.SEP, TokenType.EOL)]
        # First token is the word before the backslash.
        assert non_sep[0].type == TokenType.ESC
        assert non_sep[0].text == "foo"
        # Second token is the braced string.
        assert non_sep[1].type == TokenType.STR
        assert non_sep[1].text == "bar"

    def test_midword_backslash_newline_emits_sep(self):
        """A SEP must appear between the word and the braced string."""
        source = "foo\\\n{bar}"
        toks = _tokens(source, include_sep=True)
        types = [t.type for t in toks]
        foo_idx = next(i for i, t in enumerate(toks) if t.text == "foo")
        bar_idx = next(i for i, t in enumerate(toks) if t.text == "bar")
        # There must be at least one SEP between foo and {bar}.
        assert TokenType.SEP in types[foo_idx + 1 : bar_idx]

    def test_midword_backslash_newline_quoted_no_split(self):
        """Inside double quotes, backslash-newline does NOT split the word."""
        source = '"foo\\\nbar"'
        toks = _tokens(source)
        # The whole thing is a single token (ESC).
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC

    def test_cmd_backslash_newline_brace_arg(self):
        """Realistic case: command with backslash-newline before braced arg."""
        source = "testCmd arg1 \\\n{braced body}"
        toks = _tokens(source)
        texts = [t.text for t in toks]
        assert texts[0] == "testCmd"
        assert texts[1] == "arg1"
        brace_tok = next(t for t in toks if t.text == "braced body")
        assert brace_tok.type == TokenType.STR


class TestBackslashCRLFContinuation:
    """Backslash followed by CR, CRLF, or LF must all act as line continuations.

    Mirrors C Tcl's ``TclParseBackslash`` behaviour where ``\\<CR>``,
    ``\\<CR><LF>``, and ``\\<LF>`` are all valid line continuations.
    """

    # -- Unquoted context: word splitting ------------------------------------

    def test_crlf_continuation_splits_word(self):
        """foo\\<CR><LF>{bar} must split into foo + {bar} like \\<LF>."""
        source = "foo\\\r\n{bar}"
        toks = _tokens(source, include_sep=True)
        non_sep = [t for t in toks if t.type not in (TokenType.SEP, TokenType.EOL)]
        assert non_sep[0].text == "foo"
        assert non_sep[1].type == TokenType.STR
        assert non_sep[1].text == "bar"

    def test_cr_continuation_splits_word(self):
        """foo\\<CR>{bar} must split into foo + {bar} like \\<LF>."""
        source = "foo\\\r{bar}"
        toks = _tokens(source, include_sep=True)
        non_sep = [t for t in toks if t.type not in (TokenType.SEP, TokenType.EOL)]
        assert non_sep[0].text == "foo"
        assert non_sep[1].type == TokenType.STR
        assert non_sep[1].text == "bar"

    def test_crlf_continuation_emits_sep(self):
        """A SEP must appear between the word and the next token."""
        source = "foo\\\r\n{bar}"
        toks = _tokens(source, include_sep=True)
        types = [t.type for t in toks]
        foo_idx = next(i for i, t in enumerate(toks) if t.text == "foo")
        bar_idx = next(i for i, t in enumerate(toks) if t.text == "bar")
        assert TokenType.SEP in types[foo_idx + 1 : bar_idx]

    # -- Command continuation ------------------------------------------------

    def test_crlf_cmd_continuation(self):
        """A command split across CRLF continuation must parse correctly."""
        source = "testCmd arg1 \\\r\n{braced body}"
        toks = _tokens(source)
        texts = [t.text for t in toks]
        assert texts[0] == "testCmd"
        assert texts[1] == "arg1"
        brace_tok = next(t for t in toks if t.text == "braced body")
        assert brace_tok.type == TokenType.STR

    def test_cr_cmd_continuation(self):
        """A command split across CR continuation must parse correctly."""
        source = "testCmd arg1 \\\r{braced body}"
        toks = _tokens(source)
        texts = [t.text for t in toks]
        assert texts[0] == "testCmd"
        assert texts[1] == "arg1"
        brace_tok = next(t for t in toks if t.text == "braced body")
        assert brace_tok.type == TokenType.STR

    # -- Inside double quotes ------------------------------------------------

    def test_crlf_inside_quotes_no_split(self):
        """Inside double quotes, \\<CRLF> does NOT split the word."""
        source = '"foo\\\r\nbar"'
        toks = _tokens(source)
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC

    def test_cr_inside_quotes_no_split(self):
        """Inside double quotes, \\<CR> does NOT split the word."""
        source = '"foo\\\rbar"'
        toks = _tokens(source)
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC

    # -- Inside braces -------------------------------------------------------

    def test_crlf_inside_braces(self):
        """Backslash-CRLF inside braces is preserved (braces are literal)."""
        source = "{foo\\\r\nbar}"
        toks = _tokens(source)
        assert len(toks) == 1
        assert toks[0].type == TokenType.STR

    # -- Comments ------------------------------------------------------------

    def test_crlf_comment_continuation(self):
        """Backslash-CRLF in a comment continues to the next line."""
        source = "# hello \\\r\nworld\nputs hi"
        toks = _tokens(source)
        comment_tok = next(t for t in toks if t.type == TokenType.COMMENT)
        # Comment should swallow "world" via continuation.
        assert "world" in comment_tok.text

    def test_cr_comment_continuation(self):
        """Backslash-CR in a comment continues to the next line."""
        source = "# hello \\\rworld\nputs hi"
        toks = _tokens(source)
        comment_tok = next(t for t in toks if t.type == TokenType.COMMENT)
        assert "world" in comment_tok.text


class TestBackslashSubstCRLF:
    """backslash_subst must handle CR and CRLF line continuations."""

    def test_lf_continuation(self):
        """Standard \\<LF> continuation produces a single space."""
        assert backslash_subst("hello\\\nworld") == "hello world"

    def test_crlf_continuation(self):
        """\\<CRLF> continuation produces a single space."""
        assert backslash_subst("hello\\\r\nworld") == "hello world"

    def test_cr_continuation(self):
        """\\<CR> continuation produces a single space."""
        assert backslash_subst("hello\\\rworld") == "hello world"

    def test_crlf_continuation_strips_leading_whitespace(self):
        """Leading whitespace after \\<CRLF> is consumed."""
        assert backslash_subst("hello\\\r\n   world") == "hello world"

    def test_r_escape_preserved(self):
        r"""The two-character sequence \\r (backslash + letter r) still produces literal CR."""
        assert backslash_subst("hello\\rworld") == "hello\rworld"
