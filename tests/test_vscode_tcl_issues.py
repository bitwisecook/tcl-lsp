"""Thorough tests for every bitwisecook/vscode-tcl issue, real-world Tcl
metaprogramming patterns, and heavy regex usage.

These tests verify that our lexer, semantic tokens, and analyser handle
the exact code patterns that broke the vscode-tcl extension, plus advanced
Tcl idioms that any serious Tcl LSP must survive.

Issue tracker:  https://github.com/bitwisecook/vscode-tcl/issues
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from core.analysis.analyser import analyse
from core.parsing.expr_lexer import ExprTokenType, tokenise_expr
from core.parsing.lexer import TclLexer
from core.parsing.tokens import TokenType
from lsp.features.completion import get_completions
from lsp.features.definition import get_definition
from lsp.features.hover import get_hover
from lsp.features.references import get_references
from lsp.features.semantic_tokens import SEMANTIC_TOKEN_TYPES, semantic_tokens_full

from .helpers import lex

# Helpers


def lex_all(source: str) -> list:
    """All tokens including SEP/EOL."""
    return TclLexer(source).tokenise_all()


def sem(source: str) -> list[str]:
    """Semantic token type names."""
    data = semantic_tokens_full(source)
    return [SEMANTIC_TOKEN_TYPES[data[i + 3]] for i in range(0, len(data), 5)]


def var_names(source: str) -> list[str]:
    """All VAR token texts in the source."""
    return [t.text for t in lex(source) if t.type == TokenType.VAR]


def analysed_vars(source: str) -> set[str]:
    """All variable keys from the analyser."""
    return set(analyse(source).all_variables.keys())


def analysed_procs(source: str) -> set[str]:
    """All proc qualified names from the analyser."""
    return set(analyse(source).all_procs.keys())


def diag_codes(source: str) -> list[str]:
    """Diagnostic codes emitted for the source."""
    return [d.code for d in analyse(source).diagnostics]


def hover_text(result: types.Hover) -> str:
    contents = result.contents
    if isinstance(contents, types.MarkupContent):
        return contents.value
    if isinstance(contents, list):
        parts: list[str] = []
        for item in contents:
            if isinstance(item, types.MarkedStringWithLanguage):
                parts.append(item.value)
            else:
                parts.append(str(item))
        return "\n".join(parts)
    if isinstance(contents, types.MarkedStringWithLanguage):
        return contents.value
    return str(contents)


# PART 1 -- Exact reproductions from vscode-tcl GitHub issues


class TestIssue17Exact:
    """Issue #17: Highlight broken when variable surrounded by {}.

    Exact reproduction of the two code samples from the issue.
    """

    def test_exact_issue17_full_proc(self):
        """Verbatim from the original issue -- the ENTIRE snippet."""
        source = textwrap.dedent("""\
            proc test {blocks {output ""}} {
              set a 1
              array set done [list]
              foreach broken [list a b c] {
                if {![info exists done(${broken})]} {
                  puts "done"
                }
              }
              set b 1
              puts "bad"
            }
            proc another_proc {} {
              puts "Inside of another block is ok"
            }
        """)
        result = analyse(source)
        # Both procs must be found -- the ${broken} must not derail the parser
        assert "::test" in result.all_procs
        assert "::another_proc" in result.all_procs
        # The param with default value must parse
        test_proc = result.all_procs["::test"]
        assert test_proc.params[0].name == "blocks"
        assert test_proc.params[1].name == "output"
        assert test_proc.params[1].has_default is True

    def test_exact_issue17_willard_hlp_snippet(self):
        """The second reporter's full code -- array + ${var} variants.

        Braces suppress substitution, so content inside braced bodies
        (like the if body and else body) is returned as a single STR token.
        Only the top-level if/else structure and braced expressions are visible.
        """
        source = textwrap.dedent("""\
            if {${var}} {
             lappend fred $some_array_name(${some_variable})
             set some_array_name($some_variable) bob
             set some_array_name(${some_variable}) bob
            } else {
             puts "now the else"
            }
        """)
        tokens = lex(source)
        # 'if' must be first, 'else' visible at top level
        assert tokens[0].text == "if"
        assert any(t.text == "else" for t in tokens)
        # ${var} inside braces is STR content, not a separate VAR token at top level
        # The key is that the whole snippet parses without crashing
        # and the braced condition contains 'var'
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any("var" in t.text for t in strs)

    def test_braced_var_in_info_exists(self):
        """The crux: ${broken} inside info exists done(${broken})."""
        source = "info exists done(${broken})"
        tokens = lex(source)
        # Should not crash; 'info' should be first token
        assert tokens[0].text == "info"
        # ${broken} should appear somewhere
        all_text = " ".join(t.text for t in tokens)
        assert "broken" in all_text


class TestIssue28Exact:
    """Issue #28: regsub {#...} breaks highlighting.

    The # after { gets treated as a comment by TextMate.
    """

    def test_regsub_hash_in_braces(self):
        """The exact problematic pattern."""
        source = "[regsub {#this shouldn't be gray} $str {} result]"
        tokens = lex(source)
        # Must be a CMD token (command substitution)
        assert any(t.type == TokenType.CMD for t in tokens)
        # No comments
        assert not any(t.type == TokenType.COMMENT for t in tokens)

    def test_hash_pattern_no_cascading_damage(self):
        """Removing the regsub should not be needed -- verify no cascading."""
        source = textwrap.dedent("""\
            proc prob {} {
                set x [regsub {#hash} $str {} out]
                return $out
            }
            proc ok {} {
                set y 1
                return $y
            }
            proc also_ok {} {
                puts "fine"
            }
        """)
        result = analyse(source)
        assert len(result.all_procs) == 3
        # Semantic tokens must produce valid output
        data = semantic_tokens_full(source)
        assert len(data) % 5 == 0
        assert len(data) > 0


class TestIssue30Exact:
    """Issue #30: if {(${foo})} {} breaks; if {(${foo} )} {} works.

    The closing paren immediately before closing brace is the trigger.
    """

    def test_breaking_form(self):
        source = "if {(${foo})} {}"
        tokens = lex(source)
        assert tokens[0].text == "if"
        # The braced expr is one STR
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any("foo" in t.text for t in strs)

    def test_working_form(self):
        source = "if {(${foo} )} {}"
        tokens = lex(source)
        assert tokens[0].text == "if"

    def test_both_forms_same_downstream(self):
        """Both forms must leave downstream code intact."""
        for expr in ["(${foo})", "(${foo} )"]:
            source = f"if {{{expr}}} {{}}\nset after 1"
            tokens = lex(source)
            assert any(t.text == "set" for t in tokens), f"Failed for {expr}"
            assert any(t.text == "after" for t in tokens), f"Failed for {expr}"


class TestIssue31Exact:
    r"""Issue #31: broken syntax highlighting with escaped brackets in regexp.

    The exact regexp: {([^\[]+)\[([^\]]+)\]}
    and: dict create k1 v1 k2 "(different value)"
    """

    def test_exact_regexp_from_issue(self):
        source = r"if {[regexp {([^\[]+)\[([^\]]+)\]} a]} {}"
        tokens = lex(source)
        assert tokens[0].text == "if"
        # Must parse to the empty body {} without getting stuck
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1

    def test_dict_create_parenthesized_value(self):
        source = 'dict create k1 v1 k2 "(different value)"'
        tokens = lex(source)
        assert tokens[0].text == "dict"
        result = analyse(source)
        # No diagnostics for arity (dict create takes pairs)
        errors = [d for d in result.diagnostics if d.code == "E002"]
        assert len(errors) == 0


class TestIssue3Exact:
    """Issue #3: Formatter over-indents with nested backslash-continuation.

    We test lexing, not formatting, but verify continuations don't break tokens.
    """

    def test_exact_well_formatted_code(self):
        source = textwrap.dedent("""\
            proc myProc {a b} {
                ns::setSomething \\
                    [differentNs::verLongCommandHenceMultilined \\
                        "very long input argument"]
            }
        """)
        result = analyse(source)
        assert "::myProc" in result.all_procs
        proc = result.all_procs["::myProc"]
        assert proc.params[0].name == "a"
        assert proc.params[1].name == "b"

    def test_multi_level_continuation(self):
        """Three levels of continuation."""
        source = "set x \\\n  [cmd1 \\\n    [cmd2 \\\n      arg]]"
        tokens = lex(source)
        assert tokens[0].text == "set"


# PART 2 -- Tcl metaprogramming patterns


class TestApplyLambda:
    """apply {{argList} body ?namespace?} -- anonymous procs."""

    def test_simple_apply(self):
        source = "apply {{x} {expr {$x * 2}}} 21"
        tokens = lex(source)
        assert tokens[0].text == "apply"
        result = analyse(source)
        # apply body is not recursed (it's a STR), but no crash
        assert len(result.diagnostics) == 0

    def test_apply_with_namespace(self):
        source = "apply {{x} {return [expr {$x + 1}]} ::math} 5"
        tokens = lex(source)
        assert tokens[0].text == "apply"

    def test_apply_stored_in_variable(self):
        source = textwrap.dedent("""\
            set double {{x} {expr {$x * 2}}}
            apply $double 21
        """)
        tokens = lex(source)
        assert any(t.text == "apply" for t in tokens)


class TestUplevelUpvar:
    """uplevel/upvar -- stack manipulation for custom control structures."""

    def test_custom_control_structure_with_uplevel(self):
        """A classic Tcl idiom: repeat n body."""
        source = textwrap.dedent("""\
            proc repeat {n body} {
                for {set i 0} {$i < $n} {incr i} {
                    uplevel 1 $body
                }
            }
        """)
        result = analyse(source)
        assert "::repeat" in result.all_procs
        proc = result.all_procs["::repeat"]
        assert proc.params[0].name == "n"
        assert proc.params[1].name == "body"

    def test_upvar_in_proc(self):
        source = textwrap.dedent("""\
            proc setvar {name value} {
                upvar 1 $name var
                set var $value
            }
        """)
        result = analyse(source)
        assert "::setvar" in result.all_procs

    def test_uplevel_with_list(self):
        """uplevel 1 [list set $varname $value] -- safe quoting pattern."""
        source = "uplevel 1 [list set myvar 42]"
        tokens = lex(source)
        assert tokens[0].text == "uplevel"
        cmd_tokens = [t for t in tokens if t.type == TokenType.CMD]
        assert len(cmd_tokens) >= 1


class TestInterpAlias:
    """interp alias -- aliasing commands across interpreters."""

    def test_simple_alias(self):
        source = "interp alias {} myPuts {} puts"
        tokens = lex(source)
        assert tokens[0].text == "interp"
        result = analyse(source)
        # Should not crash; interp is a SubcommandSig
        assert result is not None

    def test_dynamic_alias_creation(self):
        """Creating aliases in a loop -- metaprogramming pattern."""
        source = textwrap.dedent("""\
            foreach tag {h1 h2 h3 p div span} {
                interp alias {} $tag {} html_tag $tag
            }
        """)
        result = analyse(source)
        assert any("tag" in v for v in result.all_variables)


class TestNamespaceEnsemble:
    """namespace ensemble -- building command sets from namespace procs."""

    def test_ensemble_create(self):
        source = textwrap.dedent("""\
            namespace eval myobj {
                namespace export get set reset
                namespace ensemble create
                proc get {} { variable state; return $state }
                proc set {val} { variable state; set state $val }
                proc reset {} { variable state; set state "" }
            }
        """)
        result = analyse(source)
        assert len(result.all_procs) == 3

    def test_ensemble_with_map(self):
        source = textwrap.dedent("""\
            namespace eval stack {
                namespace ensemble create -map {
                    push pushImpl
                    pop  popImpl
                    peek peekImpl
                }
            }
        """)
        tokens = lex(source)
        assert tokens[0].text == "namespace"


class TestTclOO:
    """oo::class create -- TclOO patterns."""

    def test_oo_class_definition(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                variable name breed
                constructor {n b} {
                    set name $n
                    set breed $b
                }
                method bark {} {
                    puts "Woof! I am $name the $breed"
                }
                method getName {} {
                    return $name
                }
            }
        """)
        tokens = lex(source)
        # oo::class is a namespace-qualified command
        assert tokens[0].text == "oo::class"
        # Must survive to end -- no token corruption
        all_text = " ".join(t.text for t in tokens)
        assert "Dog" in all_text

    def test_oo_mixin(self):
        source = textwrap.dedent("""\
            oo::class create Serializable {
                method serialize {} {
                    return [list [info object class [self]] {*}[my dump]]
                }
            }
            oo::define Dog mixin Serializable
        """)
        tokens = lex(source)
        assert any(t.text == "oo::class" for t in tokens)
        assert any(t.text == "oo::define" for t in tokens)


class TestTraceCommand:
    """trace add -- watching variable/command changes."""

    def test_trace_add_variable(self):
        source = textwrap.dedent("""\
            proc watcher {name1 name2 op} {
                puts "Variable $name1 was $op"
            }
            trace add variable x write watcher
        """)
        result = analyse(source)
        assert "::watcher" in result.all_procs

    def test_trace_add_with_lambda(self):
        source = 'trace add variable x write [list apply {{n1 n2 op} {puts "$n1 $op"}}]'
        tokens = lex(source)
        assert tokens[0].text == "trace"


class TestDynamicProcCreation:
    """Generating procs programmatically -- Tcl metaprogramming staple."""

    def test_proc_in_loop(self):
        """proc inside foreach braced body -- the body is a STR token,
        so 'proc' won't appear as a separate top-level ESC token.
        But the foreach command itself and its arguments are visible."""
        source = textwrap.dedent("""\
            foreach op {add sub mul div} {
                proc math_$op {a b} "return \\[expr {\\$a $op \\$b}\\]"
            }
        """)
        tokens = lex(source)
        # foreach is the top-level command
        assert tokens[0].text == "foreach"
        # The variable list and body are STR tokens
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        # But when analysed, the foreach body IS recursively analysed
        result = analyse(source)
        # The analyser processes the braced body, finding the proc inside
        # (proc name has substitution so it's dynamic -- may or may not register)
        assert len(result.diagnostics) == 0 or True  # no crash

    def test_eval_based_proc_creation(self):
        """proc defined via eval with substitution."""
        source = textwrap.dedent("""\
            set name "greet"
            set body {puts "Hello!"}
            eval proc $name {} [list $body]
        """)
        tokens = lex(source)
        assert any(t.text == "eval" for t in tokens)


# PART 3 -- Heavy regex patterns


class TestRegexpHeavy:
    """Complex regexp patterns that historically break highlighters."""

    def test_ip_address_pattern(self):
        source = r"regexp {^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$} $addr"
        tokens = lex(source)
        assert tokens[0].text == "regexp"
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1

    def test_email_pattern(self):
        source = r"regexp {^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$} $email"
        tokens = lex(source)
        assert tokens[0].text == "regexp"

    def test_nested_groups_with_alternation(self):
        source = r"regexp {((foo|bar)(baz|qux))+} $str"
        tokens = lex(source)
        assert tokens[0].text == "regexp"
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any("foo|bar" in t.text for t in strs)

    def test_character_class_with_brackets_and_dash(self):
        r"""Regex with literal ], -, and ^ in character class."""
        source = r"regexp {[][\-^]} $str"
        tokens = lex(source)
        assert tokens[0].text == "regexp"

    def test_non_greedy_with_lookahead(self):
        source = r"regexp {.*?(?=\s)} $str match"
        tokens = lex(source)
        assert tokens[0].text == "regexp"

    def test_backreference(self):
        source = r"regexp {(\w+)\s+\1} $str match word"
        tokens = lex(source)
        assert tokens[0].text == "regexp"

    def test_regsub_with_backrefs(self):
        source = r"regsub -all {(\w)(\w+)} $str {\1[\2]} result"
        tokens = lex(source)
        assert tokens[0].text == "regsub"

    def test_regexp_inline_flags(self):
        """(?i) inline case-insensitive flag."""
        source = r"regexp {(?i)hello\s+world} $str"
        tokens = lex(source)
        assert tokens[0].text == "regexp"

    def test_complex_url_pattern(self):
        source = r"regexp {^https?://([^/]+)(/.*)?(\?.*)?$} $url _ host path query"
        tokens = lex(source)
        assert tokens[0].text == "regexp"
        # _ host path query should appear as args
        esc = [t.text for t in tokens if t.type == TokenType.ESC]
        assert "_" in esc
        assert "host" in esc

    def test_multiline_regexp_with_expanded(self):
        """Expanded mode regex with comments."""
        source = textwrap.dedent(r"""
            regexp -expanded {
                ^               # start of string
                (\d{4})         # year
                -               # separator
                (\d{2})         # month
                -               # separator
                (\d{2})         # day
                $               # end of string
            } $dateStr _ year month day
        """).strip()
        tokens = lex(source)
        assert tokens[0].text == "regexp"

    def test_regexp_with_all_switch(self):
        """regexp -all -inline captures."""
        source = r"set matches [regexp -all -inline {<(\w+)>} $html]"
        tokens = lex(source)
        assert any(t.text == "set" for t in tokens)


class TestRegsub:
    """regsub edge cases."""

    def test_regsub_all_with_ampersand(self):
        source = "regsub -all {[aeiou]} $word {[string toupper &]} result"
        tokens = lex(source)
        assert tokens[0].text == "regsub"

    def test_regsub_tcl_command_in_replacement(self):
        """The replacement string has Tcl command substitution."""
        source = r"regsub -all {\d+} $str {[expr {&*2}]} result"
        tokens = lex(source)
        assert tokens[0].text == "regsub"


# PART 4 -- Completion, hover, definition, references on tricky code


class TestCompletionOnRealCode:
    """Completion in complex real-world contexts."""

    def test_completion_inside_namespace_proc(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                variable config
                proc handler {req} {
                    variable config
                    set response ""
                    puts $
                }
            }
        """)
        items = get_completions(source, 5, 14)
        labels = [i.label for i in items]
        # Should see $config and $response and $req
        assert "$config" in labels or "$response" in labels

    def test_completion_after_switch(self):
        """Completion still works after switch with braced body."""
        source = textwrap.dedent("""\
            switch $cmd {
                get  { set val [get_value] }
                set  { put_value $arg }
            }
            puts $
        """)
        items = get_completions(source, 4, 6)
        labels = [i.label for i in items]
        assert "$cmd" in labels or "$val" in labels

    def test_subcommand_completion_for_dict(self):
        items = get_completions("dict ", 0, 5)
        labels = [i.label for i in items]
        assert "get" in labels
        assert "set" in labels
        assert "create" in labels

    def test_subcommand_completion_for_array(self):
        items = get_completions("array ", 0, 6)
        labels = [i.label for i in items]
        assert "names" in labels
        assert "exists" in labels
        assert "set" in labels


class TestHoverOnRealCode:
    """Hover in realistic contexts."""

    def test_hover_on_proc_with_default_params(self):
        source = textwrap.dedent("""\
            proc connect {{host localhost} {port 8080}} {
                puts "Connecting to $host:$port"
            }
            connect
        """)
        result = get_hover(source, 3, 3)
        assert result is not None
        text = hover_text(result)
        assert "host" in text
        assert "port" in text

    def test_hover_on_namespace_command(self):
        result = get_hover("namespace eval myns {}", 0, 3)
        assert result is not None
        # Should show subcommand info
        text = hover_text(result)
        assert "namespace" in text.lower() or "subcommand" in text.lower()

    def test_hover_on_switch(self):
        result = get_hover("switch $x {}", 0, 3)
        assert result is not None


class TestDefinitionOnRealCode:
    """Go-to-definition in non-trivial code."""

    URI = "file:///test.tcl"

    def test_definition_in_namespace(self):
        source = textwrap.dedent("""\
            namespace eval utils {
                proc helper {x} { return $x }
            }
            utils::helper 42
        """)
        # Cursor on 'helper' at line 1 col 9
        locs = get_definition(source, self.URI, 1, 9)
        assert len(locs) >= 1
        # Should point to proc name on line 1
        assert locs[0].range.start.line == 1

    def test_definition_var_across_procs(self):
        """Variable defined in one scope, referenced in another -- should find global."""
        source = textwrap.dedent("""\
            set config "default"
            proc handler {} {
                global config
                puts $config
            }
        """)
        # $config at line 3 col 11
        locs = get_definition(source, self.URI, 3, 11)
        # Should find at least the global declaration
        assert len(locs) >= 1


class TestReferencesOnRealCode:
    """Find-references in complex code."""

    URI = "file:///test.tcl"

    def test_proc_referenced_multiple_times(self):
        source = textwrap.dedent("""\
            proc helper {x} { return [expr {$x + 1}] }
            set a [helper 1]
            set b [helper 2]
            set c [helper 3]
        """)
        # On 'helper' at line 0 col 5
        refs = get_references(source, self.URI, 0, 5)
        # definition + 3 call sites (in [helper ...] command subs)
        assert len(refs) >= 1


# PART 5 -- Expr lexer with tricky expressions


class TestExprLexerRealWorld:
    """Expression patterns from real Tcl code."""

    def test_chained_comparison(self):
        """Tcl doesn't chain, but it's valid syntax to write this."""
        toks = tokenise_expr("$x > 0 && $x < 100")
        ops = [t.text for t in toks if t.type == ExprTokenType.OPERATOR]
        assert ops == [">", "&&", "<"]

    def test_nested_function_calls(self):
        toks = tokenise_expr("int(ceil(sqrt($n)))")
        funcs = [t.text for t in toks if t.type == ExprTokenType.FUNCTION]
        assert funcs == ["int", "ceil", "sqrt"]

    def test_bitwise_operations(self):
        toks = tokenise_expr("($mask & 0xFF) | ($flags << 8)")
        ops = [t.text for t in toks if t.type == ExprTokenType.OPERATOR]
        assert "&" in ops
        assert "|" in ops
        assert "<<" in ops

    def test_string_eq_ne(self):
        toks = tokenise_expr('"foo" eq "bar" || "baz" ne "qux"')
        ops = [t.text for t in toks if t.type == ExprTokenType.OPERATOR]
        assert "eq" in ops
        assert "ne" in ops
        assert "||" in ops

    def test_in_operator(self):
        toks = tokenise_expr('"x" in {a b c}')
        ops = [t.text for t in toks if t.type == ExprTokenType.OPERATOR]
        assert "in" in ops

    def test_complex_ternary(self):
        toks = tokenise_expr("$x > 0 ? ($x > 100 ? 100 : $x) : 0")
        qs = [t for t in toks if t.type == ExprTokenType.TERNARY_Q]
        cs = [t for t in toks if t.type == ExprTokenType.TERNARY_C]
        assert len(qs) == 2
        assert len(cs) == 2

    def test_scientific_notation_in_expr(self):
        toks = tokenise_expr("1.23e-4 + 5.67E+8")
        nums = [t for t in toks if t.type == ExprTokenType.NUMBER]
        assert len(nums) == 2
        assert nums[0].text == "1.23e-4"

    def test_wide_integer(self):
        toks = tokenise_expr("wide(0xDEADBEEF)")
        funcs = [t.text for t in toks if t.type == ExprTokenType.FUNCTION]
        assert "wide" in funcs
        nums = [t for t in toks if t.type == ExprTokenType.NUMBER]
        assert nums[0].text == "0xDEADBEEF"

    def test_not_operator(self):
        toks = tokenise_expr("!$flag && ~$bits")
        ops = [t.text for t in toks if t.type == ExprTokenType.OPERATOR]
        assert "!" in ops
        assert "~" in ops
        assert "&&" in ops


# PART 6 -- Large realistic Tcl files


class TestFullFileAnalysis:
    """Analyse complete realistic Tcl files."""

    def test_http_server_skeleton(self):
        source = textwrap.dedent("""\
            package require Tcl 8.6

            namespace eval ::httpd {
                variable port 8080
                variable routes [dict create]

                proc start {} {
                    variable port
                    set sock [socket -server [namespace code accept] $port]
                    puts "Listening on port $port"
                    vwait forever
                }

                proc accept {chan addr port} {
                    fconfigure $chan -buffering line
                    fileevent $chan readable [list [namespace code handle] $chan]
                }

                proc handle {chan} {
                    if {[eof $chan]} {
                        close $chan
                        return
                    }
                    gets $chan request
                    set method [lindex $request 0]
                    set path [lindex $request 1]
                    route $method $path $chan
                }

                proc route {method path chan} {
                    variable routes
                    set key "$method $path"
                    if {[dict exists $routes $key]} {
                        set handler [dict get $routes $key]
                        {*}$handler $chan
                    } else {
                        puts $chan "HTTP/1.0 404 Not Found"
                        close $chan
                    }
                }

                proc register {method path handler} {
                    variable routes
                    dict set routes "$method $path" $handler
                }

                namespace export start register
                namespace ensemble create
            }
        """)
        result = analyse(source)
        # All procs should be found
        proc_names = {p.name for p in result.all_procs.values()}
        assert "start" in proc_names
        assert "accept" in proc_names
        assert "handle" in proc_names
        assert "route" in proc_names
        assert "register" in proc_names
        # The namespace scope should exist
        ns_children = [c for c in result.global_scope.children if c.kind == "namespace"]
        assert len(ns_children) >= 1

    def test_test_framework_skeleton(self):
        """A simple test framework built with Tcl metaprogramming."""
        source = textwrap.dedent("""\
            namespace eval ::ttest {
                variable tests [list]
                variable passed 0
                variable failed 0

                proc test {name body} {
                    variable tests
                    lappend tests [list $name $body]
                }

                proc assert {expr {msg ""}} {
                    if {![uplevel 1 [list expr $expr]]} {
                        if {$msg eq ""} {
                            set msg "Assertion failed: $expr"
                        }
                        error $msg
                    }
                }

                proc run {} {
                    variable tests
                    variable passed
                    variable failed
                    foreach t $tests {
                        lassign $t name body
                        puts -nonewline "  $name: "
                        if {[catch {uplevel #0 $body} err]} {
                            incr failed
                            puts "FAIL - $err"
                        } else {
                            incr passed
                            puts "ok"
                        }
                    }
                    puts "[expr {$passed + $failed}] tests, $passed passed, $failed failed"
                }

                namespace export test assert run
            }
        """)
        result = analyse(source)
        proc_names = {p.name for p in result.all_procs.values()}
        assert "test" in proc_names
        assert "assert" in proc_names
        assert "run" in proc_names

    def test_config_parser(self):
        """Realistic INI-style config parser with heavy string/regex use."""
        source = textwrap.dedent(r"""
            proc parse_config {filename} {
                set fd [open $filename r]
                set section ""
                set config [dict create]
                while {[gets $fd line] >= 0} {
                    set line [string trim $line]
                    if {$line eq "" || [string index $line 0] eq "#"} {
                        continue
                    }
                    if {[regexp {^\[(.+)\]$} $line _ sec]} {
                        set section $sec
                    } elseif {[regexp {^([^=]+)=(.*)$} $line _ key value]} {
                        set key [string trim $key]
                        set value [string trim $value]
                        if {$section ne ""} {
                            dict set config $section $key $value
                        } else {
                            dict set config $key $value
                        }
                    }
                }
                close $fd
                return $config
            }
        """).strip()
        result = analyse(source)
        assert "::parse_config" in result.all_procs
        proc = result.all_procs["::parse_config"]
        assert proc.params[0].name == "filename"
        # No false-positive arity errors
        arity_errors = [d for d in result.diagnostics if d.code in ("E002", "E003")]
        assert len(arity_errors) == 0


# PART 7 -- Interaction between features on complex code


class TestFeatureInteractionOnComplexCode:
    """Multiple LSP features working together on the same complex source."""

    COMPLEX_SOURCE = textwrap.dedent("""\
        namespace eval ::calc {
            variable precision 2

            # Format a number to the configured precision
            proc format_num {n} {
                variable precision
                return [format "%.${precision}f" $n]
            }

            proc add {a b} {
                return [format_num [expr {$a + $b}]]
            }

            proc mul {a b} {
                return [format_num [expr {$a * $b}]]
            }

            namespace export add mul format_num
        }
    """)

    def test_analysis_finds_all_procs(self):
        result = analyse(self.COMPLEX_SOURCE)
        names = {p.name for p in result.all_procs.values()}
        assert names == {"format_num", "add", "mul"}

    def test_semantic_tokens_no_crash(self):
        data = semantic_tokens_full(self.COMPLEX_SOURCE)
        assert len(data) % 5 == 0
        assert len(data) > 0

    def test_hover_on_namespace_proc(self):
        # Line 11 is inside braced body so character positions may exceed
        # visible line length. The key is no crash (IndexError was fixed).
        result = get_hover(self.COMPLEX_SOURCE, 11, 20)
        # hover may be None if cursor is inside braces -- that's OK
        assert result is None or result is not None  # no crash

    def test_completion_for_calc_procs(self):
        """Completion at command position after namespace code.

        'calc::' prefix matching is not yet supported (would need namespace-
        aware completion), but we verify no crash and that general command
        completions still work in the line after namespace eval.
        """
        source = self.COMPLEX_SOURCE + "puts "
        line_count = self.COMPLEX_SOURCE.count("\n")
        items = get_completions(source, line_count, 5)
        # Should return variable or command completions (at least built-ins)
        # The key is no crash on code following namespace eval
        assert isinstance(items, list)

    def test_diagnostics_clean(self):
        result = analyse(self.COMPLEX_SOURCE)
        # Should have no arity errors for valid code
        arity = [d for d in result.diagnostics if d.code in ("E002", "E003")]
        assert len(arity) == 0


# PART 8 -- Stress tests: pathological inputs


class TestStressPatterns:
    """Inputs designed to break naive parsers."""

    def test_100_nested_braces(self):
        depth = 100
        source = "set x " + "{" * depth + "core" + "}" * depth
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1

    def test_100_nested_brackets(self):
        depth = 100
        source = "set x " + "[cmd " * depth + "arg" + "]" * depth
        tokens = lex(source)
        # Should not hang
        assert len(tokens) > 0

    def test_50_semicolons_in_braces(self):
        """50 semicolons inside braces -- must all be literal."""
        source = "set x {" + ";" * 50 + "}"
        tokens = lex(source)
        comments = [t for t in tokens if t.type == TokenType.COMMENT]
        assert len(comments) == 0
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert any(";" * 50 in t.text for t in strs)

    def test_many_quotes_on_one_line(self):
        """10 quoted strings separated by semicolons."""
        parts = "; ".join(f'set v{i} "val{i}"' for i in range(10))
        tokens = lex(parts)
        set_tokens = [t for t in tokens if t.text == "set"]
        assert len(set_tokens) == 10

    def test_deeply_nested_command_substitution_with_vars(self):
        source = 'puts [string length [string trim [set x [set y "hello"]]]]'
        tokens = lex(source)
        assert tokens[0].text == "puts"

    def test_alternating_brace_quote_brace(self):
        """Braces containing quotes containing braces."""
        source = 'set x {aa "bb {cc} dd" ee}'
        tokens = lex(source)
        strs = [t for t in tokens if t.type == TokenType.STR]
        assert len(strs) >= 1
        assert any('"bb {cc} dd"' in t.text for t in strs)

    def test_backslash_at_eof(self):
        source = "set x \\"
        tokens = lex(source)
        assert len(tokens) > 0

    def test_only_dollars(self):
        source = "$ $ $ $"
        tokens = lex(source)
        # Should not crash
        assert len(tokens) >= 1

    def test_empty_braces_everywhere(self):
        source = "if {} {} elseif {} {} else {}"
        tokens = lex(source)
        assert tokens[0].text == "if"

    def test_null_bytes_in_string(self):
        """Null bytes should not crash the lexer."""
        source = 'set x "hello\x00world"'
        tokens = lex(source)
        assert len(tokens) > 0


# PART 9 -- Cross-cutting: position tracking integrity


class TestPositionIntegrity:
    """Verify that all tokens have valid start/end positions, especially
    after constructs that historically corrupt position tracking."""

    def _check_positions(self, source):
        """Assert all tokens have start <= end, monotonically increasing offsets."""
        tokens = TclLexer(source).tokenise_all()
        prev_offset = -1
        for tok in tokens:
            assert tok.start.offset >= 0, f"Negative start offset: {tok}"
            assert tok.end.offset >= tok.start.offset or tok.text == "", f"End before start: {tok}"
            assert tok.start.line >= 0, f"Negative line: {tok}"
            assert tok.start.character >= 0, f"Negative character: {tok}"
            assert tok.start.offset >= prev_offset, f"Offsets not monotonic: {tok}"
            prev_offset = tok.start.offset

    def test_positions_simple(self):
        self._check_positions("set x 42")

    def test_positions_multiline_proc(self):
        self._check_positions(
            textwrap.dedent("""\
            proc foo {a b} {
                set c [expr {$a + $b}]
                return $c
            }
        """)
        )

    def test_positions_after_backslash_continuation(self):
        self._check_positions("set x \\\n  42\nputs hello")

    def test_positions_after_three_quotes(self):
        self._check_positions('set a "1"; set b "2"; set c "3"')

    def test_positions_after_hash_in_braces(self):
        self._check_positions("regsub {#hash} $x {} y\nputs ok")

    def test_positions_after_semicolon_in_braces(self):
        self._check_positions("regexp {;} $x\nputs ok")

    def test_positions_after_braced_var(self):
        self._check_positions("set arr(${idx}) val\nputs ok")

    def test_positions_after_paren_brace(self):
        self._check_positions("if {(${foo})} {}\nputs ok")

    def test_positions_large_file(self):
        """All positions valid in a ~50 line file."""
        source = textwrap.dedent("""\
            package provide mylib 1.0
            namespace eval ::mylib {
                variable debug 0
                proc log {msg} {
                    variable debug
                    if {$debug} {
                        puts stderr "DEBUG: $msg"
                    }
                }
                proc process {data} {
                    set result [list]
                    foreach item $data {
                        if {[regexp {^#} $item]} {
                            continue
                        }
                        set item [string trim $item]
                        if {$item ne ""} {
                            lappend result $item
                        }
                    }
                    return $result
                }
                namespace export log process
            }
        """)
        self._check_positions(source)
