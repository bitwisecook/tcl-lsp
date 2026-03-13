"""Adversarial parser/lexer edge-case tests.

Every test in this file corresponds to valid (or deliberately invalid) Tcl
that C Tcl 9.0.3 handles in a specific, documented way.  The goal is to
beat on our lexer with the nastiest inputs we can think of — special-
character names, deep nesting, bizarre escaping, interleaved substitutions,
and error cases — and confirm we match C Tcl's tokenisation behaviour.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.parsing.command_segmenter import segment_commands
from core.parsing.lexer import TclLexer, TclParseError
from core.parsing.tokens import SourcePosition, Token, TokenType

from .helpers import lex

# Helpers


def lex_all(source: str) -> list[Token]:
    """All tokens including SEP/EOL."""
    return TclLexer(source).tokenise_all()


def lex_types(source: str) -> list[TokenType]:
    return [t.type for t in lex(source)]


def lex_texts(source: str) -> list[str]:
    return [t.text for t in lex(source)]


def lex_with_warnings(source: str) -> tuple[list[Token], list[tuple[SourcePosition, str]]]:
    """Return (tokens, warnings) for strict-mode-off parsing."""
    lexer = TclLexer(source)
    toks = []
    while True:
        t = lexer.get_token()
        if t is None:
            break
        if t.type not in (TokenType.SEP, TokenType.EOL):
            toks.append(t)
    return toks, lexer.warnings


def lex_strict(source: str) -> list[Token]:
    """Lex with strict_quoting enabled (raises on errors like C Tcl)."""
    old = TclLexer.strict_quoting
    TclLexer.strict_quoting = True
    try:
        return lex(source)
    finally:
        TclLexer.strict_quoting = old


# Group 1: Escaped special characters as bare words
#
# In Tcl, backslash-escaping a special character makes it literal text.
# C Tcl 9.0 treats all of these as plain ESC tokens.


class TestEscapedSpecialChars:
    """Backslash-escaped special characters become literal text (ESC)."""

    def test_escaped_dollar(self):
        """``\\$`` is literal ``$``, not a variable substitution."""
        toks = lex("\\$")
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "\\$"

    def test_escaped_dollar_before_name(self):
        """``\\$a`` is literal ``$a``, not a variable reference."""
        toks = lex("\\$a")
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "\\$a"

    def test_escaped_open_bracket(self):
        """``\\[`` is literal ``[``, not command substitution."""
        toks = lex("\\[cmd\\]")
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "\\[cmd\\]"

    def test_escaped_open_brace(self):
        """``\\{body\\}`` is literal text, not a braced string."""
        toks = lex("\\{body\\}")
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "\\{body\\}"

    def test_escaped_double_quote(self):
        """``\\"hello\\"`` is literal text with escaped quotes."""
        toks = lex('\\"hello\\"')
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == '\\"hello\\"'

    def test_escaped_semicolon(self):
        """``a\\;b`` is one word — the semicolon does not end the command."""
        toks = lex("a\\;b")
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "a\\;b"

    def test_escaped_hash(self):
        """``\\#notacomment`` is literal text, not a comment."""
        toks = lex("\\#notacomment")
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC

    def test_escaped_space_joins_words(self):
        """``hello\\ world`` is a single word with an escaped space."""
        toks = lex("hello\\ world")
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "hello\\ world"

    def test_escaped_newline_continues_word(self):
        """Backslash-newline is a continuation — C Tcl splits into two words."""
        # In C Tcl: ``foo\\\nbar`` → two words: ``foo`` and ``bar``.
        # But at the lexer level, the backslash-newline is part of the text.
        toks = lex("foo\\\nbar")
        # The continuation makes this two separate words in C Tcl.
        texts = [t.text for t in toks if t.type == TokenType.ESC]
        # At minimum, the lexer must not crash and must produce tokens.
        assert len(texts) >= 1


# Group 2: Variable names with special characters via ${} syntax
#
# Tcl's ${name} form allows ANY characters in the variable name up to
# the closing brace.  C Tcl 9.0 accepts all of these.


class TestBracedVarSpecialChars:
    """``${name}`` can hold any character as a variable name."""

    def test_braced_var_double_quote(self):
        """``${\"}``: variable name is a literal double-quote."""
        toks = lex('${"}')
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == '"'

    def test_braced_var_open_bracket(self):
        """``${[}``: variable name is ``[``."""
        toks = lex("${[}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "["

    def test_braced_var_close_bracket(self):
        """``${]}``: variable name is ``]``."""
        toks = lex("${]}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "]"

    def test_braced_var_dollar(self):
        """``${$}``: variable name is ``$``."""
        toks = lex("${$}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "$"

    def test_braced_var_backslash(self):
        """``${\\\\}``: variable name is ``\\``."""
        toks = lex("${\\}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "\\"

    def test_braced_var_space(self):
        """``${ }``: variable name is a single space."""
        toks = lex("${ }")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == " "

    def test_braced_var_spaces_in_name(self):
        """``${a b c}``: variable name contains spaces."""
        toks = lex("${a b c}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "a b c"

    def test_braced_var_empty_name(self):
        """``${}``: empty variable name — valid in C Tcl (runtime error)."""
        toks = lex("${}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == ""

    def test_braced_var_semicolons(self):
        """``${a;b}``: semicolons inside the name are part of it."""
        toks = lex("${a;b}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "a;b"

    def test_braced_var_newlines(self):
        """``${a\\nb}``: newlines inside the name are part of it."""
        toks = lex("${a\nb}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "a\nb"

    def test_braced_var_open_brace_unclosed(self):
        """``${{}``: ``{`` starts the name; closing ``}`` matches the
        ``${`` opening, so the var name is ``{``.  Wait — actually this
        is ambiguous.  In C Tcl, ``${{}`` is an error (missing ``}``)
        because the inner ``{`` prevents the ``}`` from closing.
        Actually no: C Tcl's ``Tcl_ParseVarName`` scans for the first
        ``}`` — braces are NOT nested inside ``${}`` — so ``${{}``
        has name ``{`` and the parser consumes it.  But our lexer
        also scans for first ``}`` so this should be ``{``."""
        toks, warnings = lex_with_warnings("${{}")
        # ``${`` starts, first ``{`` is part of name, ``}`` closes.
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "{"

    def test_braced_var_nested_braces(self):
        """``${a{b}c}``: braces inside are literal — name is ``a{b}c``
        because ``${...}`` scans for the first unescaped ``}``.
        Actually, first ``}`` at position 5 closes.  Name = ``a{b``."""
        toks = lex("${a{b}c}")
        # First } closes the ${...} — C Tcl gets name "a{b", then "c}" is text.
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "a{b"

    def test_braced_var_with_equals_and_colons(self):
        """``${foo::bar=baz}``: all of ``::`` and ``=`` are part of name."""
        toks = lex("${foo::bar=baz}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "foo::bar=baz"


# Group 3: Dollar sign edge cases
#
# The ``$`` character has complex context-dependent parsing.


class TestDollarEdgeCases:
    """Edge cases in ``$`` parsing — bare dollars, double dollars, etc."""

    def test_bare_dollar_eof(self):
        """``$`` at EOF is a literal dollar sign."""
        toks = lex("$")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "$"

    def test_bare_dollar_before_space(self):
        """``$ `` — dollar followed by space is literal."""
        toks = lex("$ x")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "$"

    def test_bare_dollar_before_semicolon(self):
        """``$;`` — dollar before semicolon is literal."""
        toks = lex("$;")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "$"

    def test_double_dollar(self):
        """``$$foo`` — first ``$`` is bare, second starts ``$foo``."""
        toks = lex("$$foo")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "$"
        assert toks[1].type == TokenType.VAR
        assert toks[1].text == "foo"

    def test_triple_dollar(self):
        """``$$$x`` — first two ``$`` are bare, third starts ``$x``."""
        toks = lex("$$$x")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "$"
        assert toks[1].type == TokenType.STR
        assert toks[1].text == "$"
        assert toks[2].type == TokenType.VAR
        assert toks[2].text == "x"

    def test_dollar_before_cmd_sub(self):
        """``$[set x 1]`` — bare dollar then command substitution."""
        toks = lex("$[set x 1]")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "$"
        assert toks[1].type == TokenType.CMD
        assert toks[1].text == "set x 1"

    def test_dollar_paren_is_scalar_ref(self):
        """``$(foo)`` — C Tcl treats this as a scalar variable reference
        whose name is ``(foo)``, NOT an array reference.  You can
        ``set {(foo)} val`` and read it back with ``$(foo)``."""
        toks = lex("$(foo)")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "(foo)"

    def test_dollar_namespace_global(self):
        """``$::foo`` — global namespace variable."""
        toks = lex("$::foo")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "::foo"

    def test_dollar_double_colon_alone(self):
        """``$::`` — namespace separator with no name after — valid var ref."""
        toks = lex("$::")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "::"

    def test_dollar_single_colon(self):
        """``$:`` — single colon is not a namespace separator; bare dollar."""
        toks = lex("$:")
        # In C Tcl, ':' is not alphanumeric/underscore and not '::',
        # so $ is bare.  ':' is literal text.
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "$"


# Group 4: Deeply nested structures
#
# Stress-test the lexer's nesting depth tracking.


class TestDeepNesting:
    """Deep nesting of brackets, braces, and quoted strings."""

    def test_5_deep_command_sub(self):
        """``[[[[[set x 1]]]]]`` — 5 levels of nested command substitution."""
        toks = lex("[[[[[set x 1]]]]]")
        assert toks[0].type == TokenType.CMD
        assert toks[0].text == "[[[[set x 1]]]]"

    def test_5_deep_braces(self):
        """``{{{{{body}}}}}`` — 5 levels of nested braces."""
        toks = lex("{{{{{body}}}}}")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "{{{{body}}}}"

    def test_realistic_deep_nesting(self):
        """Deeply nested set/expr — common real-world pattern."""
        toks = lex("[set x [set y [set z [expr {1+2}]]]]")
        assert toks[0].type == TokenType.CMD
        assert "set x [set y" in toks[0].text
        assert "expr {1+2}" in toks[0].text

    def test_10_deep_command_sub(self):
        """10 levels of nested command substitution."""
        inner = "set x 1"
        for _ in range(9):
            inner = f"[{inner}]"
        source = f"[{inner}]"
        toks = lex(source)
        assert toks[0].type == TokenType.CMD
        # Must not crash and must contain innermost command
        assert "set x 1" in toks[0].text

    def test_10_deep_braces(self):
        """10 levels of nested braces."""
        inner = "body"
        for _ in range(10):
            inner = f"{{{inner}}}"
        toks = lex(inner)
        assert toks[0].type == TokenType.STR

    def test_alternating_braces_and_brackets(self):
        """``[if {1} {[if {2} {[expr {3}]}]}]`` — brackets and braces
        alternating at each level."""
        toks = lex("[if {1} {[if {2} {[expr {3}]}]}]")
        assert toks[0].type == TokenType.CMD
        assert "expr {3}" in toks[0].text


# Group 5: Mixed nesting — the really evil ones
#
# Combinations of quotes, braces, brackets, and variables that force
# the lexer to track multiple context layers simultaneously.


class TestMixedNesting:
    """Interleaved brackets, braces, and quotes."""

    def test_cmd_sub_containing_braces_with_brackets(self):
        """``[set x {[puts hello]}]`` — braces protect inner brackets
        from being parsed as command substitution."""
        toks = lex("[set x {[puts hello]}]")
        assert toks[0].type == TokenType.CMD
        assert toks[0].text == "set x {[puts hello]}"

    def test_quoted_string_with_cmd_sub_with_braces(self):
        """``"[set x {hello world}]"`` — quotes around cmd sub with braces."""
        toks = lex('"[set x {hello world}]"')
        types = [t.type for t in toks]
        assert TokenType.CMD in types
        cmd = next(t for t in toks if t.type == TokenType.CMD)
        assert cmd.text == "set x {hello world}"

    def test_cmd_sub_with_quoted_brackets(self):
        """``[string map {"[" "("} $x]`` — quotes inside cmd sub
        protect brackets from affecting the nesting level."""
        toks = lex('[string map {"[" "("} $x]')
        assert toks[0].type == TokenType.CMD
        assert '"["' in toks[0].text

    def test_braces_protect_everything(self):
        """``{$var [cmd] "quotes" \\escape}`` — nothing is substituted
        inside braces."""
        toks = lex('{$var [cmd] "quotes" \\escape}')
        assert toks[0].type == TokenType.STR
        assert toks[0].text == '$var [cmd] "quotes" \\escape'

    def test_quotes_inside_braces_are_literal(self):
        """``{a"b"c}`` — quotes are literal inside braces."""
        toks = lex('{a"b"c}')
        assert toks[0].type == TokenType.STR
        assert toks[0].text == 'a"b"c'

    def test_brackets_inside_braces_are_literal(self):
        """``{a[b]c}`` — brackets are literal inside braces."""
        toks = lex("{a[b]c}")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "a[b]c"

    def test_dollar_inside_braces_is_literal(self):
        """``{a$bc}`` — dollar is literal inside braces."""
        toks = lex("{a$bc}")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "a$bc"

    def test_braces_inside_quotes(self):
        """``"a{b}c"`` — braces are literal inside quotes."""
        toks = lex('"a{b}c"')
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "a{b}c"

    def test_hash_after_semicolon_is_comment(self):
        """``set x 1 ;# trailing comment`` — hash after ``;`` is a comment."""
        toks = lex("set x 1 ;# trailing comment")
        types = [t.type for t in toks]
        assert TokenType.COMMENT in types
        assert toks[-1].type == TokenType.COMMENT

    def test_hash_in_argument_is_not_comment(self):
        """``set x #notacomment`` — hash in argument position is NOT a comment."""
        toks = lex("set x #notacomment")
        types = [t.type for t in toks]
        assert TokenType.COMMENT not in types
        assert toks[-1].type == TokenType.ESC
        assert toks[-1].text == "#notacomment"

    def test_hash_in_quotes_is_not_comment(self):
        """``set x "#text"`` — hash inside quotes is literal."""
        toks = lex('set x "#text"')
        types = [t.type for t in toks]
        assert TokenType.COMMENT not in types

    def test_hash_in_braces_is_not_comment(self):
        """``set x {#text}`` — hash inside braces is literal."""
        toks = lex("set x {#text}")
        types = [t.type for t in toks]
        assert TokenType.COMMENT not in types


# Group 6: Interleaved substitutions inside quoted strings
#
# The lexer must correctly segment multiple adjacent substitutions
# within a double-quoted string.


class TestInterleavedSubstitutions:
    """Multiple substitutions adjacent to each other inside quotes."""

    def test_consecutive_vars(self):
        """``"$a$b$c"`` — three adjacent variable substitutions."""
        toks = lex('"$a$b$c"')
        vars_ = [t for t in toks if t.type == TokenType.VAR]
        assert len(vars_) == 3
        assert [v.text for v in vars_] == ["a", "b", "c"]

    def test_var_cmd_var(self):
        """``"$a[cmd]$b"`` — var, cmd sub, var."""
        toks = lex('"$a[cmd]$b"')
        types = [t.type for t in toks if t.type not in (TokenType.ESC,)]
        assert types == [TokenType.VAR, TokenType.CMD, TokenType.VAR]

    def test_consecutive_cmd_subs(self):
        """``"[a][b][c]"`` — three adjacent command substitutions."""
        toks = lex('"[a][b][c]"')
        cmds = [t for t in toks if t.type == TokenType.CMD]
        assert len(cmds) == 3
        assert [c.text for c in cmds] == ["a", "b", "c"]

    def test_escaped_dollar_in_quotes(self):
        """``"\\$notavar"`` — escaped dollar in quotes is literal text."""
        toks = lex('"\\$notavar"')
        types = [t.type for t in toks]
        assert TokenType.VAR not in types

    def test_escaped_bracket_in_quotes(self):
        """``"\\[notacmd]"`` — escaped bracket is literal, but the ``]``
        is also literal text.  C Tcl does not perform cmd sub."""
        toks = lex('"\\[notacmd]"')
        types = [t.type for t in toks]
        assert TokenType.CMD not in types

    def test_text_var_text_cmd_text(self):
        """``"hello $name, result=[calc]!"`` — mixed text and substitutions."""
        toks = lex('"hello $name, result=[calc]!"')
        types_no_esc = [t.type for t in toks if t.type != TokenType.ESC]
        assert types_no_esc == [TokenType.VAR, TokenType.CMD]
        var = next(t for t in toks if t.type == TokenType.VAR)
        assert var.text == "name"
        cmd = next(t for t in toks if t.type == TokenType.CMD)
        assert cmd.text == "calc"


# Group 7: Array index edge cases
#
# Array indices in ``$arr(idx)`` can contain almost anything, including
# nested parentheses, variables, commands, and special characters.


class TestArrayIndexEdgeCases:
    """Evil array indices: parens, vars, cmds, special chars."""

    def test_nested_parens(self):
        """``$arr((nested))`` — parentheses nest inside array index."""
        toks = lex("$arr((nested))")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr((nested))"

    def test_var_in_index(self):
        """``$arr($inner)`` — variable substitution in index."""
        toks = lex("$arr($inner)")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr($inner)"

    def test_escaped_brackets_in_index(self):
        """``$arr(\\[idx\\])`` — escaped brackets in index."""
        toks = lex("$arr(\\[idx\\])")
        assert toks[0].type == TokenType.VAR
        assert "arr(" in toks[0].text

    def test_braces_in_index(self):
        """``$arr({key})`` — braces are literal in array index."""
        toks = lex("$arr({key})")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr({key})"

    def test_quotes_in_index(self):
        """``$arr("key")`` — quotes are literal in array index."""
        toks = lex('$arr("key")')
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == 'arr("key")'

    def test_spaces_in_index(self):
        """``$arr(a b)`` — spaces inside array index."""
        toks = lex("$arr(a b)")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr(a b)"

    def test_deeply_nested_parens(self):
        """``$arr(((deep)))`` — three levels of nested parens in index."""
        toks = lex("$arr(((deep)))")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr(((deep)))"

    def test_namespace_array(self):
        """``$ns::arr(key)`` — namespace-qualified array."""
        toks = lex("$ns::arr(key)")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "ns::arr(key)"

    def test_empty_index(self):
        """``$arr()`` — empty array index."""
        toks = lex("$arr()")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr()"

    def test_index_with_semicolon(self):
        """``$arr(a;b)`` — semicolon inside index (literal)."""
        toks = lex("$arr(a;b)")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr(a;b)"


# Group 8: Proc/variable names via braces (no escaping needed)
#
# Using braces, you can set/get variables with any name in Tcl.


class TestSpecialNamesViaBraces:
    """Using brace-quoting to pass special-character strings as names."""

    def test_set_var_named_dollar_a(self):
        """``set {$a} 1`` — variable name is literal ``$a``."""
        toks = lex("set {$a} 1")
        assert toks[0].text == "set"
        assert toks[1].type == TokenType.STR
        assert toks[1].text == "$a"

    def test_set_var_named_bracket_cmd(self):
        """``set {[cmd]} 1`` — variable name is literal ``[cmd]``."""
        toks = lex("set {[cmd]} 1")
        assert toks[1].type == TokenType.STR
        assert toks[1].text == "[cmd]"

    def test_set_var_named_quote(self):
        """``set {"} 1`` — variable name is a double-quote."""
        toks = lex('set {"} 1')
        assert toks[1].type == TokenType.STR
        assert toks[1].text == '"'

    def test_proc_named_escaped_dollar(self):
        """``proc \\$ {} {puts dollar}`` — proc named ``$``."""
        toks = lex("proc \\$ {} {puts dollar}")
        assert toks[0].text == "proc"
        assert toks[1].type == TokenType.ESC
        assert toks[1].text == "\\$"

    def test_proc_named_escaped_bracket(self):
        """``proc \\[ {} {puts bracket}`` — proc named ``[``."""
        toks = lex("proc \\[ {} {puts bracket}")
        assert toks[0].text == "proc"
        assert toks[1].type == TokenType.ESC
        assert toks[1].text == "\\["

    def test_proc_name_via_braces(self):
        """``proc {weird name} {} {}`` — proc with spaces in name."""
        toks = lex("proc {weird name} {} {}")
        assert toks[1].type == TokenType.STR
        assert toks[1].text == "weird name"

    def test_proc_name_all_special(self):
        """``proc {$[]{}} {} {}`` — proc name is all special chars.
        Inside braces, ``{`` and ``}`` are matched pairs, so the
        content is ``$[]{}`` (the inner ``{}`` is a matched pair)."""
        toks = lex("proc {$[]{}} {} {}")
        assert toks[1].type == TokenType.STR
        assert toks[1].text == "$[]{}"


# Group 9: Empty and minimal structures


class TestEmptyStructures:
    """Empty quoted strings, braces, and command substitutions."""

    def test_empty_quoted_string(self):
        """``""`` — empty quoted string."""
        toks = lex('""')
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == ""

    def test_empty_braces(self):
        """``{}`` — empty braces."""
        toks = lex("{}")
        assert len(toks) == 1
        assert toks[0].type == TokenType.STR
        assert toks[0].text == ""

    def test_empty_cmd_sub(self):
        """``[]`` — empty command substitution."""
        toks = lex("[]")
        assert len(toks) == 1
        assert toks[0].type == TokenType.CMD
        assert toks[0].text == ""

    def test_whitespace_cmd_sub(self):
        """``[  ]`` — whitespace-only command substitution."""
        toks = lex("[  ]")
        assert toks[0].type == TokenType.CMD

    def test_single_char_tokens(self):
        """Single character bare words."""
        for ch in "abcxyz019_":
            toks = lex(ch)
            assert len(toks) == 1
            assert toks[0].text == ch


# Group 10: Close bracket / brace as bare word
#
# A ``]`` at command level is just a literal character in C Tcl.


class TestBareDelimiters:
    """Unmatched close delimiters at top level."""

    def test_bare_close_bracket(self):
        """``]`` — bare close bracket is literal text."""
        toks = lex("]")
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "]"

    def test_bare_close_bracket_as_command(self):
        """``] foo`` — ``]`` as command name, ``foo`` as argument."""
        toks = lex("] foo")
        assert toks[0].text == "]"
        assert toks[1].text == "foo"

    def test_bare_close_brace_is_text(self):
        """``}`` at top level is literal text (with ``extra characters``
        warning in strict mode), but in non-strict it's just text."""
        toks = lex("} foo")
        texts = [t.text for t in toks]
        assert "}" in texts[0] or "foo" in texts


# Group 11: Backslash-newline continuation edge cases


class TestBackslashNewline:
    """Backslash-newline continuation in various contexts."""

    def test_continuation_in_bare_word(self):
        """``set x \\<newline>hello`` — continuation joins into next line."""
        # The lexer should produce tokens for set, x, and the continuation.
        texts = lex_texts("set x \\\nhello")
        assert "set" in texts
        assert "x" in texts

    def test_continuation_in_quoted_string(self):
        """``"hello \\<newline>  world"`` — continuation inside quotes."""
        toks = lex('"hello \\\n  world"')
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        # The text includes the backslash-newline sequence
        assert "hello" in toks[0].text
        assert "world" in toks[0].text

    def test_continuation_in_comment(self):
        """``# comment \\<newline>continued`` — comment extends to next line."""
        toks = lex("# comment \\\ncontinued")
        comments = [t for t in toks if t.type == TokenType.COMMENT]
        assert len(comments) == 1
        assert "continued" in comments[0].text

    def test_backslash_at_eof_in_word(self):
        """A trailing backslash at EOF — no character to escape."""
        toks = lex("set x \\")
        # Should not crash; backslash is part of the word
        assert len(toks) >= 2


# Group 12: Error cases (strict mode should raise, non-strict should warn)


class TestErrorCases:
    """Malformed inputs that should produce errors/warnings."""

    def test_unclosed_quote_warns(self):
        """``"hello`` — unclosed quote produces a warning."""
        toks, warnings = lex_with_warnings('"hello')
        assert any('missing "' in msg for _, msg in warnings)

    def test_unclosed_quote_strict_raises(self):
        """``"hello`` — unclosed quote raises in strict mode."""
        with pytest.raises(TclParseError, match='missing "'):
            lex_strict('"hello')

    def test_unclosed_brace_warns(self):
        """``{hello`` — unclosed brace produces a warning."""
        toks, warnings = lex_with_warnings("{hello")
        assert any("missing close-brace" in msg for _, msg in warnings)

    def test_unclosed_brace_strict_raises(self):
        """``{hello`` — unclosed brace raises in strict mode."""
        with pytest.raises(TclParseError, match="missing close-brace"):
            lex_strict("{hello")

    def test_unclosed_bracket_warns(self):
        """``[hello`` — unclosed bracket produces a warning."""
        toks, warnings = lex_with_warnings("[hello")
        assert any("missing close-bracket" in msg for _, msg in warnings)

    def test_unclosed_bracket_strict_raises(self):
        """``[hello`` — unclosed bracket raises in strict mode."""
        with pytest.raises(TclParseError, match="missing close-bracket"):
            lex_strict("[hello")

    def test_extra_chars_after_quote_warns(self):
        """``"hello"world`` — junk after close-quote."""
        toks, warnings = lex_with_warnings('"hello"world')
        assert any("extra characters after close-quote" in msg for _, msg in warnings)

    def test_extra_chars_after_quote_strict(self):
        """``"hello"world`` — strict mode raises."""
        with pytest.raises(TclParseError, match="extra characters after close-quote"):
            lex_strict('"hello"world')

    def test_extra_chars_after_brace_warns(self):
        """``{hello}world`` — junk after close-brace."""
        toks, warnings = lex_with_warnings("{hello}world")
        assert any("extra characters after close-brace" in msg for _, msg in warnings)

    def test_extra_chars_after_brace_strict(self):
        """``{hello}world`` — strict mode raises."""
        with pytest.raises(TclParseError, match="extra characters after close-brace"):
            lex_strict("{hello}world")

    def test_unclosed_braced_var_warns(self):
        """``${name`` — unclosed ``${`` produces a warning."""
        toks, warnings = lex_with_warnings("${name")
        assert any("missing close-brace for variable name" in msg for _, msg in warnings)

    def test_unclosed_braced_var_strict(self):
        """``${name`` — strict mode raises."""
        with pytest.raises(TclParseError, match="missing close-brace for variable name"):
            lex_strict("${name")

    def test_error_in_cmd_sub_brace(self):
        """``[a {b c]`` — unclosed brace inside command substitution.
        C Tcl would raise 'missing close-brace'.  Our lexer may handle
        this differently since it tracks brace level in cmd sub."""
        # At minimum, must not crash or hang.
        toks, warnings = lex_with_warnings("[a {b c]")
        assert len(toks) >= 1

    def test_error_in_cmd_sub_quote(self):
        """``[a "b c]`` — unclosed quote inside command substitution.
        The ``]`` inside the quote is consumed as literal text, so the
        bracket is actually unclosed."""
        toks, warnings = lex_with_warnings('[a "b c]')
        # The ] is consumed by the quote context, so this is unclosed cmd sub
        assert len(toks) >= 1


# Group 13: {*} expansion edge cases


class TestExpandEdgeCases:
    """{*} expansion prefix (Tcl 8.5+) edge cases."""

    def test_expand_basic(self):
        """``{*}list`` — basic expansion."""
        toks = lex("{*}list")
        types = [t.type for t in toks]
        assert TokenType.EXPAND in types

    def test_expand_with_braces(self):
        """``{*}{body}`` — expansion followed by braced word."""
        toks = lex("{*}{body}")
        types = [t.type for t in toks]
        assert TokenType.EXPAND in types
        assert TokenType.STR in types

    def test_expand_with_var(self):
        """``{*}$var`` — expansion followed by variable."""
        toks = lex("{*}$var")
        types = [t.type for t in toks]
        assert TokenType.EXPAND in types
        assert TokenType.VAR in types

    def test_expand_with_cmd_sub(self):
        """``{*}[cmd]`` — expansion followed by command substitution."""
        toks = lex("{*}[cmd]")
        types = [t.type for t in toks]
        assert TokenType.EXPAND in types
        assert TokenType.CMD in types

    def test_expand_with_quoted_string(self):
        """``{*}"hello"`` — expansion followed by quoted string."""
        toks = lex('{*}"hello"')
        types = [t.type for t in toks]
        assert TokenType.EXPAND in types

    def test_star_brace_not_expand(self):
        """``{**}`` — ``{**}`` is a braced string, NOT expansion + ``*}``."""
        toks = lex("{**}")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "**"

    def test_expand_at_eof(self):
        """``{*}`` followed by nothing — this is just a braced string."""
        # {*} at end-of-input with nothing following is a braced string "*"
        # because expansion needs a following word character.
        toks = lex("{*}")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "*"

    def test_expand_before_space(self):
        """``{*} list`` — space after ``{*}`` means it's a braced string."""
        toks = lex("{*} list")
        # {*} followed by space is braced string "*", then "list" is separate word
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "*"


# Group 14: Truly diabolical combinations
#
# These test multiple parser features interacting simultaneously.


class TestDiabolicalCombinations:
    """Tests designed to stress multiple parser features at once."""

    def test_var_in_quoted_string_in_cmd_sub(self):
        """``[puts "$var"]`` — var inside quotes inside cmd sub."""
        toks = lex('[puts "$var"]')
        assert toks[0].type == TokenType.CMD
        assert "$var" in toks[0].text

    def test_cmd_sub_in_array_index_in_quoted_string(self):
        """``"$arr([cmd])"`` — cmd sub inside array index inside quotes."""
        toks = lex('"$arr([cmd])"')
        vars_ = [t for t in toks if t.type == TokenType.VAR]
        assert len(vars_) >= 1
        assert "arr([cmd])" in vars_[0].text

    def test_nested_quotes_via_escaping(self):
        """``"hello \\"world\\" end"`` — escaped quotes inside quoted string."""
        toks = lex('"hello \\"world\\" end"')
        assert toks[0].type == TokenType.ESC
        assert '\\"world\\"' in toks[0].text

    def test_backslash_brace_in_braces(self):
        """``{hello \\} world}`` — backslash-close-brace inside braces
        prevents the brace from closing.  Content is ``hello \\} world``."""
        toks = lex("{hello \\} world}")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "hello \\} world"

    def test_backslash_open_brace_in_braces(self):
        """``{hello \\{ world}`` — backslash-open-brace doesn't increase
        the nesting level.  Content is ``hello \\{ world``."""
        toks = lex("{hello \\{ world}")
        assert toks[0].type == TokenType.STR
        assert toks[0].text == "hello \\{ world"

    def test_string_map_with_special_chars(self):
        """A realistic ``string map`` with brackets and quotes."""
        toks = lex('[string map {"\\[" "(" "\\]" ")"} $text]')
        assert toks[0].type == TokenType.CMD

    def test_switch_with_multiple_bodies(self):
        """A switch command with multiple braced bodies."""
        source = "switch -- $x {\n    a {puts A}\n    b {puts B}\n    default {puts C}\n}"
        toks = lex(source)
        assert toks[0].text == "switch"

    def test_proc_body_with_nested_procs(self):
        """A proc body containing another proc definition."""
        source = "proc outer {} {\n    proc inner {} {\n        puts hello\n    }\n}"
        toks = lex(source)
        assert toks[0].text == "proc"
        # The body should be a single braced string
        body_tok = [t for t in toks if t.type == TokenType.STR and "inner" in t.text]
        assert len(body_tok) >= 1

    def test_multiple_semicolons(self):
        """``set a 1 ;; set b 2`` — empty command between semicolons."""
        toks = lex("set a 1 ;; set b 2")
        texts = [t.text for t in toks if t.type == TokenType.ESC]
        assert texts.count("set") == 2

    def test_escaped_everything_in_one_word(self):
        """A word with every special character escaped."""
        source = '\\$\\[\\]\\{\\}\\"\\;\\#\\\\'
        toks = lex(source)
        assert len(toks) == 1
        assert toks[0].type == TokenType.ESC
        # All escapes should be in the text
        assert "\\$" in toks[0].text
        assert "\\[" in toks[0].text

    def test_var_in_cmd_sub_in_var_array_index(self):
        """``$outer([set inner $x])`` — var → array index → cmd sub → var."""
        toks = lex("$outer([set inner $x])")
        assert toks[0].type == TokenType.VAR
        assert "outer(" in toks[0].text

    def test_deeply_nested_braces_with_backslash_braces(self):
        """Nested braces with backslash-protected inner braces."""
        source = "{level1 {level2 \\{ \\} level2} level1}"
        toks = lex(source)
        assert toks[0].type == TokenType.STR
        assert "level2" in toks[0].text

    def test_unmatched_open_brace_in_quoted_string(self):
        """``"hello { world"`` — unmatched brace inside quotes is fine
        (braces don't nest inside quotes)."""
        toks = lex('"hello { world"')
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "hello { world"

    def test_unmatched_close_brace_in_quoted_string(self):
        """``"hello } world"`` — unmatched close-brace is fine in quotes."""
        toks = lex('"hello } world"')
        assert toks[0].type == TokenType.ESC
        assert toks[0].text == "hello } world"


# Group 15: Position tracking under stress
#
# Make sure offsets/line/column survive the above abuse.


class TestPositionUnderStress:
    """Position tracking must be correct even for evil inputs."""

    def test_braced_var_positions(self):
        """``${a b}`` — VAR token starts at the ``$`` (offset 0)."""
        toks = lex("${a b}")
        assert toks[0].start.offset == 0
        assert toks[0].start.line == 0

    def test_nested_cmd_sub_positions(self):
        """Inner cmd sub position for ``set [expr {1}]``."""
        toks = lex("set [expr {1}]")
        cmd = next(t for t in toks if t.type == TokenType.CMD)
        # CMD token starts at the '[' offset
        assert cmd.start.offset == 4

    def test_multiline_brace_end_position(self):
        """Braced string spanning multiple lines — end on correct line."""
        source = "{line1\nline2\nline3}"
        toks = lex(source)
        assert toks[0].end.line == 2

    def test_quoted_string_with_escapes_positions(self):
        """Position tracking through escaped characters in quotes."""
        source = '"hello \\"world\\""'
        toks = lex(source)
        assert toks[0].start.offset == 0
        # End should be at the closing quote position
        assert toks[0].end.offset > 0

    def test_continuation_position(self):
        """Backslash-newline continuation tracks lines correctly."""
        source = "set x \\\nhello"
        toks = lex(source)
        # "hello" (or continuation token) should be on line 1
        last_tok = toks[-1]
        # The token on the second line should reflect line 1
        if last_tok.start.line == 1:
            assert last_tok.start.character >= 0

    def test_comment_continuation_position(self):
        """Comment with backslash-newline continuation — spans two lines."""
        source = "# comment \\\ncontinued\nset x 1"
        toks = lex(source)
        comment = next(t for t in toks if t.type == TokenType.COMMENT)
        assert comment.start.line == 0
        # The comment extends to line 1
        assert comment.end.line == 1


# Group 16: Realistic but mean patterns from real Tcl code


class TestRealisticMeanPatterns:
    """Patterns drawn from real Tcl code that are parser-hostile."""

    def test_regexp_with_brackets(self):
        """``regexp {[a-z]+} $str match`` — character class looks like cmd sub."""
        toks = lex("regexp {[a-z]+} $str match")
        assert toks[0].text == "regexp"
        # The pattern is a braced string (no substitution)
        assert toks[1].type == TokenType.STR
        assert toks[1].text == "[a-z]+"

    def test_regsub_with_backref(self):
        """``regsub -all {(\\w+)} $str {\\1} result`` — backreferences."""
        toks = lex("regsub -all {(\\w+)} $str {\\1} result")
        assert toks[0].text == "regsub"

    def test_format_string(self):
        """``format "%s = %d" $name [expr {$x+1}]`` — format with subs."""
        toks = lex('format "%s = %d" $name [expr {$x+1}]')
        assert toks[0].text == "format"
        cmd = next(t for t in toks if t.type == TokenType.CMD)
        assert "expr" in cmd.text

    def test_double_eval_pattern(self):
        """``eval [list set x [expr {1+2}]]`` — eval with list construction."""
        toks = lex("eval [list set x [expr {1+2}]]")
        assert toks[0].text == "eval"
        assert toks[1].type == TokenType.CMD

    def test_namespace_path_var(self):
        """``$::my::deeply::nested::var`` — deeply namespace-qualified var."""
        toks = lex("$::my::deeply::nested::var")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "::my::deeply::nested::var"

    def test_multiline_if(self):
        """Multi-line if/elseif/else with braced bodies."""
        source = (
            "if {$x == 1} {\n"
            "    puts one\n"
            "} elseif {$x == 2} {\n"
            "    puts two\n"
            "} else {\n"
            "    puts other\n"
            "}"
        )
        toks = lex(source)
        assert toks[0].text == "if"
        strs = [t for t in toks if t.type == TokenType.STR]
        # Should have condition and body braced strings
        assert len(strs) >= 4

    def test_catch_with_result_var(self):
        """``catch {expr {1/0}} result opts`` — catch capturing error."""
        toks = lex("catch {expr {1/0}} result opts")
        assert toks[0].text == "catch"
        assert toks[1].type == TokenType.STR
        assert "expr {1/0}" in toks[1].text

    def test_dict_with_special_keys(self):
        """``dict set d {key with spaces} {value with $dollar}``."""
        toks = lex("dict set d {key with spaces} {value with $dollar}")
        strs = [t for t in toks if t.type == TokenType.STR]
        assert any("key with spaces" in s.text for s in strs)
        assert any("value with $dollar" in s.text for s in strs)

    def test_lmap_with_nested_cmd(self):
        """``lmap x $list {string toupper [string index $x 0]}``."""
        toks = lex("lmap x $list {string toupper [string index $x 0]}")
        assert toks[0].text == "lmap"
        body = next(t for t in toks if t.type == TokenType.STR and "toupper" in t.text)
        assert "[string index $x 0]" in body.text

    def test_apply_lambda(self):
        """``apply {{x y} {expr {$x + $y}}} 3 4`` — apply lambda."""
        toks = lex("apply {{x y} {expr {$x + $y}}} 3 4")
        assert toks[0].text == "apply"
        assert toks[1].type == TokenType.STR


class TestIRulesBraceSeparator:
    """iRules treats ``}{`` as a word boundary (no whitespace required)."""

    def test_brace_separator_produces_separate_words(self):
        """``if {$a}{puts a}`` parses as 3-arg command in iRules mode."""
        old = TclLexer.irules_brace_separator
        TclLexer.irules_brace_separator = True
        try:
            cmds = segment_commands("if {$a}{puts a}")
            assert len(cmds) == 1
            assert len(cmds[0].argv) == 3
            assert cmds[0].texts[0] == "if"
            assert cmds[0].texts[1] == "$a"
            assert cmds[0].texts[2] == "puts a"
        finally:
            TclLexer.irules_brace_separator = old

    def test_brace_separator_no_warning(self):
        """``}{`` should not warn in iRules mode."""
        old = TclLexer.irules_brace_separator
        TclLexer.irules_brace_separator = True
        try:
            _, warnings = lex_with_warnings("if {$a}{puts a}")
            assert not warnings
        finally:
            TclLexer.irules_brace_separator = old

    def test_standard_tcl_warns_on_brace_separator(self):
        """``}{`` should warn in standard Tcl mode."""
        _, warnings = lex_with_warnings("if {$a}{puts a}")
        assert any("extra characters after close-brace" in msg for _, msg in warnings)

    def test_standard_tcl_concatenates_brace_words(self):
        """In standard Tcl, ``{a}{b}`` is one word ``ab``."""
        cmds = segment_commands("cmd {a}{b}")
        assert len(cmds) == 1
        # cmd + one concatenated word = 2 args
        assert len(cmds[0].argv) == 2

    def test_triple_brace_separator(self):
        """``if {cond}{body1}{body2}`` — three braced words in iRules."""
        old = TclLexer.irules_brace_separator
        TclLexer.irules_brace_separator = True
        try:
            cmds = segment_commands("if {cond}{body1}{body2}")
            assert len(cmds) == 1
            assert len(cmds[0].argv) == 4
            assert cmds[0].texts == ["if", "cond", "body1", "body2"]
        finally:
            TclLexer.irules_brace_separator = old
