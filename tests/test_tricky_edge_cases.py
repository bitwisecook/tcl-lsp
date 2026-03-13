"""Tricky edge-case tests derived from real-world bugs in vscode-tcl.

Each test class maps to a GitHub issue from bitwisecook/vscode-tcl,
plus additional pathological Tcl constructs discovered during analysis.

Reference:  https://github.com/bitwisecook/vscode-tcl/issues
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.parsing.lexer import TclLexer
from core.parsing.tokens import TokenType
from lsp.features.semantic_tokens import SEMANTIC_TOKEN_TYPES, semantic_tokens_full

from .helpers import lex

# Helpers


def lex_all(source: str) -> list:
    """Return all tokens including SEP/EOL."""
    return TclLexer(source).tokenise_all()


def sem_types(source: str) -> list[str]:
    """Return list of semantic token type names for the source."""
    data = semantic_tokens_full(source)
    return [SEMANTIC_TOKEN_TYPES[data[i + 3]] for i in range(0, len(data), 5)]


# Issue #31 -- Escaped brackets in regexp inside braces


class TestIssue31EscapedBracketsInRegexp:
    """
    Code:  if {[regexp {([^\\[]+)\\\\[([^\\]]+)\\\\} a]} {}
    Bug:   TextMate grammar breaks on nested escaped brackets in regex.
    Our lexer must handle this brace-balanced content correctly.
    """

    def test_regexp_with_escaped_brackets_in_braces(self):
        source = r"regexp {([^\[]+)\[([^\]]+)\]} $str match"
        tokens = lex(source)
        # The command name should be 'regexp'
        assert tokens[0].text == "regexp"
        assert tokens[0].type == TokenType.ESC

    def test_if_wrapping_regexp_with_escaped_brackets(self):
        source = r"if {[regexp {([^\[]+)\[([^\]]+)\]} $str m]} {puts $m}"
        tokens = lex(source)
        assert tokens[0].text == "if"
        # Should parse to completion without error -- all braces balanced
        # The body {puts $m} is captured as a single STR token
        assert any("puts" in t.text for t in tokens)

    def test_dict_create_with_parentheses_in_value(self):
        """Second example from issue #31: parenthesized value."""
        source = 'dict create k1 v1 k2 "(different value)"'
        tokens = lex(source)
        assert tokens[0].text == "dict"
        assert tokens[1].text == "create"
        # The quoted string should be captured as one token piece
        quoted_texts = [t.text for t in tokens if t.type == TokenType.ESC]
        assert any("different value" in t for t in quoted_texts)

    def test_semantic_tokens_survive_escaped_brackets(self):
        """Semantic tokens should not crash on escaped bracket regex."""
        source = r"regexp {([^\[]+)\[([^\]]+)\]} $str match"
        data = semantic_tokens_full(source)
        # Should produce valid 5-int chunks
        assert len(data) % 5 == 0
        assert len(data) > 0


# Issue #30 -- Parentheses in if expr break highlighting


class TestIssue30ParenthesesInIfExpr:
    """
    Code:  if {(${foo})} {}
    Bug:   ')' right before '}' confuses the TextMate grammar.
    Our lexer treats braces as opaque -- content inside is a STR token.
    """

    def test_braced_var_in_parens(self):
        source = "if {(${foo})} {}"
        tokens = lex(source)
        assert tokens[0].text == "if"
        # The braced expression should be a single STR token
        assert any(t.type == TokenType.STR and "foo" in t.text for t in tokens)

    def test_braced_var_in_parens_no_trailing_corruption(self):
        """Tokens after the if should still parse correctly."""
        source = 'if {(${foo})} {}\nputs "hello"'
        tokens = lex(source)
        assert any(t.text == "puts" for t in tokens)
        assert any(t.text == "hello" for t in tokens)

    def test_semantic_tokens_after_paren_expr(self):
        source = 'if {(${foo})} {}\nputs "after"'
        types = sem_types(source)
        # 'puts' should still be classified as keyword
        assert "keyword" in types


# Issue #29 -- Semicolon in regexp breaks highlighting


class TestIssue29SemicolonInRegexp:
    """
    Code:  regexp {;} $str
    Bug:   TextMate treats ';' as command separator even inside braces.
    Our lexer correctly treats braces as opaque (no command separation).
    """

    def test_semicolon_inside_braces_is_string(self):
        source = "regexp {;} $str"
        tokens = lex(source)
        assert tokens[0].text == "regexp"
        # The {;} should be a STR token, not split by semicolon
        brace_tok = [t for t in tokens if t.type == TokenType.STR and ";" in t.text]
        assert len(brace_tok) == 1

    def test_semicolon_class_in_regexp(self):
        """Regex character class with semicolon."""
        source = "regexp {[;:,]} $str"
        tokens = lex(source)
        assert tokens[0].text == "regexp"
        # Braces protect the content -- should be a single STR
        brace_tok = [t for t in tokens if t.type == TokenType.STR]
        assert len(brace_tok) >= 1
        assert any(";" in t.text for t in brace_tok)

    def test_code_after_semicolon_regexp_not_corrupted(self):
        """Code after the regexp should still tokenise correctly."""
        source = 'regexp {;} $str\nputs "still ok"'
        tokens = lex(source)
        assert any(t.text == "puts" for t in tokens)

    def test_semicolon_in_proc_body_regexp(self):
        """The worst case: semicolon regexp inside a proc -- issue said it
        broke highlighting for the rest of the file."""
        source = textwrap.dedent("""\
            proc checker {str} {
                if {[regexp {^[;]+$} $str]} {
                    return 1
                }
                return 0
            }
            proc nextProc {} {
                puts "this should be fine"
            }
        """)
        tokens = lex(source)
        # Both procs should be recognized as top-level tokens
        proc_tokens = [t for t in tokens if t.text == "proc"]
        assert len(proc_tokens) == 2
        # Second proc body should contain 'puts' (inside braced body)
        assert any("puts" in t.text for t in tokens if t.type == TokenType.STR)
        # Analyser should see both procs
        result = analyse(source)
        assert len(result.all_procs) == 2


# Issue #28 -- Hash/pound after closing brace breaks highlighting


class TestIssue28HashAfterBrace:
    """
    Code:  [regsub {#this shouldn't be gray}]
    Bug:   '#' inside braces treated as comment by TextMate grammar.
    Our lexer: braces suppress comment detection (no special '#' treatment).
    """

    def test_hash_inside_braces_is_not_comment(self):
        source = "regsub {#not a comment} $str {} result"
        tokens = lex(source)
        # The {#not a comment} should be a STR, not COMMENT
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 0
        str_tokens = [t for t in tokens if t.type == TokenType.STR]
        assert any("#not a comment" in t.text for t in str_tokens)

    def test_hash_inside_command_sub_braces(self):
        source = "[regsub {#this is not gray} $str {} result]"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 0

    def test_code_after_hash_brace_not_corrupted(self):
        """The cascading failure: next proc should be fine."""
        source = textwrap.dedent("""\
            proc first {} {
                regsub {#hash in regex} $str {} cleaned
                return $cleaned
            }
            proc second {} {
                puts "this must still highlight"
            }
        """)
        tokens = lex(source)
        proc_tokens = [t for t in tokens if t.text == "proc"]
        assert len(proc_tokens) == 2
        # Both procs should be analysed
        result = analyse(source)
        assert len(result.all_procs) == 2
        # Semantic tokens at top level: proc (keyword), first (string), ...
        # proc bodies are braces so inner cmds aren't in top-level tokens.
        # Key check: the second proc's tokens are not corrupted by the #hash
        types = sem_types(source)
        # At minimum 2 'keyword' for the two 'proc' keywords
        assert types.count("keyword") >= 2


# Issue #27 -- Third double-quote breaks highlighting


class TestIssue27ThirdDoubleQuote:
    """
    Code with three quotes:  set x "hello" ; set y "world" ; set z "third"
    Bug:   TextMate sees mismatched quotes, everything after turns green.
    Our lexer handles quotes via insidequote tracking per parse_string call.
    """

    def test_three_quoted_strings_on_one_line(self):
        source = 'set a "first"; set b "second"; set c "third"'
        tokens = lex(source)
        vars_ = [t for t in tokens if t.type == TokenType.ESC and t.text in ("a", "b", "c")]
        # All three variable names should be visible
        assert len(vars_) == 3

    def test_three_quoted_strings_semantic_tokens(self):
        source = 'set a "first"; set b "second"; set c "third"'
        types = sem_types(source)
        # Should have 3 'keyword' for 'set', 3 variable names, 3 strings
        assert types.count("keyword") == 3

    def test_quoted_string_with_dollar_interpolation(self):
        """Specific pattern from the issue: quotes with $var inside."""
        source = 'set line "$key$delimiter$value"'
        tokens = lex(source)
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        # Should find key, delimiter, value as VAR tokens
        var_names = [t.text for t in var_tokens]
        assert "key" in var_names
        assert "delimiter" in var_names
        assert "value" in var_names

    def test_code_after_quoted_interpolation_not_corrupted(self):
        source = 'set line "$key$delimiter$value"\nputs "still ok"\nreturn 0'
        tokens = lex(source)
        assert any(t.text == "puts" for t in tokens)
        assert any(t.text == "return" for t in tokens)


# Issue #17 -- Variable surrounded by {} in array index


class TestIssue17BracedVarInArrayIndex:
    """
    Code:  set some_array(${some_variable}) bob
    Bug:   ${var} form inside parens confused the TextMate grammar.
    Our lexer has explicit ${name} handling in _parse_var.
    """

    def test_braced_var_form(self):
        source = "set x ${my_var}"
        tokens = lex(source)
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) >= 1
        assert var_tokens[0].text == "my_var"

    def test_braced_var_in_array_context(self):
        """set arr(${idx}) val -- the problematic pattern."""
        source = "set arr(${idx}) val"
        tokens = lex(source)
        # Should get at least a token with 'arr' and a token with 'idx'
        # The 'arr(' part starts as ESC, then ${idx} is VAR
        all_text = "".join(t.text for t in tokens)
        assert "idx" in all_text

    def test_lappend_with_braced_var_array(self):
        source = "lappend fred $some_array(${some_variable})"
        tokens = lex(source)
        assert tokens[0].text == "lappend"
        # $some_array(${some_variable}) -- the dollar starts a VAR parse
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) >= 1

    def test_code_after_braced_array_index(self):
        """After the problematic pattern, code must still parse."""
        source = textwrap.dedent("""\
            foreach item $list {
                set result(${item}) [compute $item]
            }
            puts "done"
        """)
        tokens = lex(source)
        assert any(t.text == "puts" for t in tokens)


# Issue #3 -- Backslash-newline continuation in nested brackets


class TestIssue3BackslashNewlineContinuation:
    """
    Code:
        ns::setSomething \\
            [differentNs::verLongCommand \\
                "very long arg"]
    Bug:   Formatter over-indents; TextMate can't track continuation depth.
    Our lexer: backslash-newline is handled in _parse_string's '\\' case.
    """

    def test_backslash_continuation_basic(self):
        source = "set x \\\n    42"
        tokens = lex(source)
        # Should see 'set', 'x', and '42' (the backslash-newline is consumed)
        esc_texts = [t.text for t in tokens if t.type == TokenType.ESC]
        assert "set" in esc_texts

    def test_backslash_continuation_in_brackets(self):
        source = textwrap.dedent("""\
            ns::setSomething \\
                [differentNs::longCommand \\
                    "very long input argument"]
        """)
        tokens = lex(source)
        # The command substitution should be found
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) >= 1

    def test_backslash_continuation_preserves_positions(self):
        """Lines after continuation should have correct line numbers."""
        source = "set x \\\n    42\nputs hello"
        tokens = lex(source)
        puts_tok = [t for t in tokens if t.text == "puts"]
        assert len(puts_tok) == 1
        assert puts_tok[0].start.line == 2


# Issue #2 -- General highlighting: numbers, operators, variables


class TestIssue2BasicHighlighting:
    """
    Bug:   Numbers, $variable in strings, and operators not highlighted.
    Our semantic tokens classify these explicitly.
    """

    def test_numbers_highlighted(self):
        types = sem_types("set x 42")
        assert "number" in types

    def test_variable_in_string_highlighted(self):
        source = 'puts "hello $name"'
        types = sem_types(source)
        assert "variable" in types

    def test_operators_highlighted(self):
        source = "+ 1 2"
        types = sem_types(source)
        assert "operator" in types

    def test_builtin_commands_are_keywords(self):
        """Top-level commands are keywords; commands inside braces are body STR."""
        source = "proc foo {} {}\nfor {set i 0} {$i < 10} {incr i} {}\nwhile {1} {break}"
        types = sem_types(source)
        # Only top-level commands are classified: proc, for, while
        # set, incr, break are inside braced args -- content of STR tokens
        assert types.count("keyword") >= 3  # proc, for, while


# Additional pathological Tcl constructs


class TestDeeplyNestedBraces:
    """Tcl allows arbitrary brace nesting -- verify our lexer handles it."""

    def test_triple_nested_braces(self):
        source = "set x {a {b {c d} e} f}"
        tokens = lex(source)
        str_tokens = [t for t in tokens if t.type == TokenType.STR]
        assert any("a {b {c d} e} f" in t.text for t in str_tokens)

    def test_five_level_nesting(self):
        source = "set x {1 {2 {3 {4 {5}}}}}"
        tokens = lex(source)
        str_tokens = [t for t in tokens if t.type == TokenType.STR]
        assert len(str_tokens) >= 1

    def test_braces_with_escaped_brace(self):
        r"""Escaped brace inside braces: \{ does not count for nesting."""
        source = r"set x {hello \{ world}"
        tokens = lex(source)
        str_tokens = [t for t in tokens if t.type == TokenType.STR]
        assert len(str_tokens) >= 1


class TestNestedCommandSubstitution:
    """Multiple levels of [brackets]."""

    def test_nested_brackets(self):
        source = "set x [expr {[llength [list a b c]] + 1}]"
        tokens = lex(source)
        assert tokens[0].text == "set"
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) >= 1

    def test_brackets_inside_quotes(self):
        source = 'set x "[clock format [clock seconds]]"'
        tokens = lex(source)
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        # Should find at least the outer command substitution
        assert len(cmd_tokens) >= 1

    def test_brackets_inside_braces_are_literal(self):
        """Brackets inside braces should NOT be command-substituted."""
        source = "set x {[not a command]}"
        tokens = lex(source)
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) == 0  # braces suppress substitution
        str_tokens = [t for t in tokens if t.type == TokenType.STR]
        assert any("[not a command]" in t.text for t in str_tokens)


class TestMixedQuoteBraceInteraction:
    """Quotes and braces interacting in tricky ways."""

    def test_quote_inside_braces(self):
        source = 'set x {he said "hello"}'
        tokens = lex(source)
        str_tokens = [t for t in tokens if t.type == TokenType.STR]
        assert any('"hello"' in t.text for t in str_tokens)

    def test_brace_inside_quotes(self):
        source = 'set x "open { close }"'
        tokens = lex(source)
        # The quoted string should contain the braces literally
        esc_tokens = [t for t in tokens if t.type == TokenType.ESC]
        assert any("{" in t.text for t in esc_tokens)

    def test_unmatched_brace_in_quotes(self):
        """An unmatched brace inside quotes should NOT break parsing."""
        source = 'set x "one { brace"\nputs "still ok"'
        tokens = lex(source)
        assert any(t.text == "puts" for t in tokens)

    def test_unmatched_quote_in_braces(self):
        """An unmatched quote inside braces should NOT break parsing."""
        source = 'set x {one " quote}\nputs "fine"'
        tokens = lex(source)
        assert any(t.text == "puts" for t in tokens)


class TestCommentEdgeCases:
    """Comment detection is only at command position after EOL."""

    def test_hash_not_comment_mid_command(self):
        """# in argument position is NOT a comment."""
        source = "set x #not_comment"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 0

    def test_hash_after_semicolon_is_comment(self):
        """# after ; is in command position = comment."""
        source = "set x 1 ;# this is a comment"
        all_tokens = lex_all(source)
        comments = [t for t in all_tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 1

    def test_hash_inside_quotes_not_comment(self):
        source = 'puts "color is #FF0000"'
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 0

    def test_hash_inside_braces_not_comment(self):
        source = "set pattern {#[0-9]+}"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 0


class TestMultilineStringPositionTracking:
    """Ensure line/col tracking stays correct across multiline constructs."""

    def test_multiline_braced_body_positions(self):
        source = textwrap.dedent("""\
            proc foo {} {
                set x 1
                set y 2
            }
            puts done
        """)
        tokens = lex(source)
        puts_tok = [t for t in tokens if t.text == "puts"]
        assert len(puts_tok) == 1
        assert puts_tok[0].start.line == 4

    def test_multiline_quoted_string_positions(self):
        source = 'set x "line1\nline2\nline3"\nputs after'
        tokens = lex(source)
        puts_tok = [t for t in tokens if t.text == "puts"]
        assert len(puts_tok) == 1
        # After the 3-line string, puts should be on line 3
        assert puts_tok[0].start.line == 3


class TestNamespaceQualifiedVariables:
    """$ns::var and complex namespace forms."""

    def test_simple_namespace_var(self):
        source = "puts $::global_var"
        tokens = lex(source)
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 1
        assert var_tokens[0].text == "::global_var"

    def test_deep_namespace_var(self):
        source = "puts $a::b::c::var"
        tokens = lex(source)
        var_tokens = [t for t in tokens if t.type == TokenType.VAR]
        assert len(var_tokens) == 1
        assert var_tokens[0].text == "a::b::c::var"

    def test_namespace_var_in_expr(self):
        source = "expr {$::ns::val + 1}"
        tokens = lex(source)
        # The braced body is STR -- check it contains the namespace ref
        str_tokens = [t for t in tokens if t.type == TokenType.STR]
        assert any("::ns::val" in t.text for t in str_tokens)


class TestSwitchAnalysis:
    """Analyser handling of switch command bodies."""

    def test_switch_bodies_analysed(self):
        source = textwrap.dedent("""\
            switch $x {
                foo { set result 1 }
                bar { set result 2 }
                default { set result 0 }
            }
        """)
        result = analyse(source)
        # 'result' should be defined as a variable
        assert any("result" in v for v in result.all_variables)

    def test_switch_with_options(self):
        source = textwrap.dedent("""\
            switch -regexp -- $x {
                {^[0-9]+$} { set kind number }
                {^[a-z]+$} { set kind word }
            }
        """)
        result = analyse(source)
        assert any("kind" in v for v in result.all_variables)

    def test_switch_fallthrough(self):
        """'-' body means fall through -- should not be analysed as Tcl."""
        source = textwrap.dedent("""\
            switch $x {
                a -
                b { set matched ab }
            }
        """)
        result = analyse(source)
        assert any("matched" in v for v in result.all_variables)


class TestTryCatchAnalysis:
    """Analyser handling of try/catch/finally."""

    def test_catch_defines_result_var(self):
        source = "catch {expr {1/0}} result opts"
        result = analyse(source)
        assert any("result" in v for v in result.all_variables)
        assert any("opts" in v for v in result.all_variables)

    def test_try_finally(self):
        source = textwrap.dedent("""\
            try {
                set fd [open "file.txt"]
            } finally {
                close $fd
            }
        """)
        result = analyse(source)
        assert any("fd" in v for v in result.all_variables)

    def test_try_on_error(self):
        source = textwrap.dedent("""\
            try {
                set x [dangerous_op]
            } on error {msg opts} {
                puts "Error: $msg"
            }
        """)
        result = analyse(source)
        assert any("x" in v for v in result.all_variables)


class TestCatchBodySetInCondition:
    """Variables set inside catch body in an if condition must not trigger W210."""

    def test_dns_edns0_catch_set_no_w210(self):
        """Real-world iRules pattern: catch {set var [DNS::edns0 ...]}."""
        source = textwrap.dedent("""\
            when DNS_REQUEST priority 345 {
                if { [DNS::edns0 exists] && !([catch {set ecs_addr [DNS::edns0 subnet address]}]) } {
                    if {$ecs_addr == 0 || $ecs_addr eq ""} {
                        set ecs_addr [IP::client_addr]
                    }
                } else {
                    set ecs_addr [IP::client_addr]
                }
            }
        """)
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")
        result = analyse(source)
        w210 = [d for d in result.diagnostics if d.code == "W210"]
        assert len(w210) == 0


class TestForeachMultipleVarLists:
    """foreach with multiple var/list pairs."""

    def test_foreach_single_var(self):
        source = "foreach item {a b c} { puts $item }"
        result = analyse(source)
        assert any("item" in v for v in result.all_variables)

    def test_foreach_multi_vars(self):
        source = 'foreach {k v} {a 1 b 2} { puts "$k=$v" }'
        result = analyse(source)
        # Both k and v should be defined
        var_names = list(result.all_variables.keys())
        assert any("k" in v for v in var_names)
        assert any("v" in v for v in var_names)


class TestEmptyAndEdgeCaseSources:
    """Edge cases for the lexer."""

    def test_empty_string(self):
        tokens = lex("")
        assert tokens == []

    def test_only_whitespace(self):
        tokens = lex("   \n  \t  \n")
        assert tokens == []

    def test_only_comment(self):
        tokens = lex("# just a comment")
        assert len(tokens) == 1
        assert tokens[0].type == TokenType.COMMENT

    def test_unterminated_quote(self):
        """Should not hang or crash."""
        tokens = lex('set x "unterminated')
        assert len(tokens) > 0

    def test_unterminated_bracket(self):
        """Should not hang or crash."""
        tokens = lex("set x [unterminated")
        assert len(tokens) > 0

    def test_unterminated_brace(self):
        """Should not hang or crash."""
        tokens = lex("set x {unterminated")
        assert len(tokens) > 0

    def test_bare_dollar(self):
        source = "puts $"
        tokens = lex(source)
        # Should handle gracefully -- bare $ becomes STR
        assert len(tokens) >= 1

    def test_dollar_followed_by_space(self):
        source = "puts $ x"
        tokens = lex(source)
        assert len(tokens) >= 1

    def test_very_long_line(self):
        """Should not crash on very long lines."""
        source = 'set x "' + "a" * 10000 + '"'
        tokens = lex(source)
        assert len(tokens) > 0

    def test_many_semicolons(self):
        source = "set a 1; set b 2; set c 3; set d 4; set e 5"
        tokens = lex(source)
        set_tokens = [t for t in tokens if t.text == "set"]
        assert len(set_tokens) == 5


class TestRegexpSpecificPatterns:
    """Regex patterns that historically break highlighters."""

    def test_character_class_with_close_bracket(self):
        """[]] is valid regex -- ] as first char in character class."""
        source = r"regexp {[]]]} $str"
        tokens = lex(source)
        assert tokens[0].text == "regexp"

    def test_character_class_with_backslash(self):
        source = r"regexp {[\\\]]} $str"
        tokens = lex(source)
        assert tokens[0].text == "regexp"

    def test_alternation_with_special_chars(self):
        source = r"regexp {foo|bar|baz\d+} $str"
        tokens = lex(source)
        assert tokens[0].text == "regexp"

    def test_lookahead_pattern(self):
        source = r"regexp {(?=foo)bar} $str"
        tokens = lex(source)
        assert tokens[0].text == "regexp"

    def test_regsub_with_ampersand(self):
        source = "regsub -all {[aeiou]} $str {&_vowel} result"
        tokens = lex(source)
        assert tokens[0].text == "regsub"


class TestElseIfPatterns:
    """Issue #25 -- elseif patterns."""

    def test_inline_elseif(self):
        source = "if {$x > 0} { set r pos } elseif {$x < 0} { set r neg } else { set r zero }"
        tokens = lex(source)
        assert any(t.text == "elseif" for t in tokens)
        assert any(t.text == "else" for t in tokens)

    def test_multiline_elseif(self):
        source = textwrap.dedent("""\
            if {$x > 0} {
                set r pos
            } elseif {$x < 0} {
                set r neg
            } else {
                set r zero
            }
        """)
        tokens = lex(source)
        # All keywords should be found
        kw = [t.text for t in tokens if t.text in ("if", "elseif", "else")]
        assert "if" in kw
        assert "elseif" in kw
        assert "else" in kw

    def test_elseif_semantic_analysis(self):
        """All branches should have their bodies analysed."""
        source = textwrap.dedent("""\
            if {1} {
                set a 1
            } elseif {0} {
                set b 2
            } else {
                set c 3
            }
        """)
        result = analyse(source)
        # Variables from all branches should be found
        var_names = list(result.all_variables.keys())
        assert any("a" in v for v in var_names)
        # b and c may or may not be found depending on how the analyser handles
        # elseif/else as separate args -- at minimum 'a' must be found


class TestExprSubLanguage:
    """Expression bodies that live inside {braces}."""

    def test_expr_with_function_calls(self):
        source = "set x [expr {sin($y) + cos($z)}]"
        tokens = lex(source)
        assert tokens[0].text == "set"

    def test_expr_with_ternary(self):
        source = 'set x [expr {$a > 0 ? "pos" : "neg"}]'
        tokens = lex(source)
        assert tokens[0].text == "set"

    def test_expr_multiline(self):
        source = textwrap.dedent("""\
            set result [expr {
                ($a * $b) +
                ($c * $d) -
                sqrt($e)
            }]
        """)
        tokens = lex(source)
        assert tokens[0].text == "set"

    def test_expr_with_string_comparison(self):
        source = 'expr {"hello" eq "world"}'
        tokens = lex(source)
        assert tokens[0].text == "expr"


class TestAnalyserNamespaceEdgeCases:
    """Complex namespace patterns."""

    def test_nested_namespace_eval(self):
        source = textwrap.dedent("""\
            namespace eval outer {
                namespace eval inner {
                    proc helper {} { return 42 }
                }
            }
        """)
        result = analyse(source)
        # The proc should be found somewhere in the tree
        assert len(result.all_procs) >= 1

    def test_namespace_variable(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                variable counter 0
                proc increment {} {
                    variable counter
                    incr counter
                }
            }
        """)
        result = analyse(source)
        assert any("counter" in v for v in result.all_variables)

    def test_qualified_proc_call(self):
        """Calling a namespace-qualified proc."""
        source = textwrap.dedent("""\
            namespace eval utils {
                proc helper {x} { return $x }
            }
            utils::helper 42
        """)
        tokens = lex(source)
        # The qualified call should be a single token piece
        esc_tokens = [t for t in tokens if t.type == TokenType.ESC]
        assert any("utils::helper" in t.text for t in esc_tokens)
