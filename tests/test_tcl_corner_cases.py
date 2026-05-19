"""Corner-case tests pinning every row in ``docs/kcs/kcs-tcl-corner-cases.md``.

Each test corresponds to exactly one row in the KCS table.  When real Tcl
behaviour and our analyser/lexer agree, the test asserts the matching
behaviour.  When they diverge, the test pins the documented divergence
so we notice if the lexer/analyser drifts.

Reference: ``tests/data/tcl_probes_full.tcl`` (machine-runnable probe set
against tclsh 9.0.3).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from compiler.parsing.lexer import TclLexer
from compiler.parsing.tokens import TokenType
from core.analysis import analyse


def lex(source: str) -> list:
    lx = TclLexer(source)
    out = []
    while True:
        t = lx.get_token()
        if t is None:
            break
        if t.type in (TokenType.SEP, TokenType.EOL, TokenType.EOF):
            continue
        out.append(t)
    return out


# =============================================================
# §A. Bare ``$name`` substitution
# =============================================================


class TestBareVarSubstitution:
    def test_bare_simple(self):
        toks = lex("$foo")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "foo"

    def test_bare_underscore(self):
        toks = lex("$foo_bar")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "foo_bar"

    def test_bare_digit(self):
        toks = lex("$foo123")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "foo123"

    def test_bare_global_qualifier(self):
        toks = lex("$::g")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "::g"

    def test_bare_ns_chain(self):
        toks = lex("$::ns::x")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "::ns::x"

    def test_bare_stops_at_dot(self):
        # ``$foo.bar`` -- VAR(``foo``) + ESC(``.bar``).
        toks = lex("$foo.bar")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "foo"
        assert toks[1].type == TokenType.ESC
        assert toks[1].text == ".bar"

    def test_bare_stops_at_hyphen(self):
        toks = lex("$foo-bar")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "foo"
        assert toks[1].text == "-bar"

    def test_bare_double_dollar(self):
        # ``$$x`` -- first ``$`` has no valid name (next char is ``$``);
        # emitted as literal ``$``, then VAR(``x``).
        toks = lex("$$x")
        # Literal ``$`` then VAR.
        assert any(t.type == TokenType.VAR and t.text == "x" for t in toks)

    def test_bare_just_double_colon(self):
        # ``$::`` -- ``::`` parses as the var name; runtime errors.
        toks = lex("$::")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "::"

    def test_bare_trailing_double_colon(self):
        toks = lex("$x::")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "x::"


# =============================================================
# §B. Brace ``${name}`` substitution
# =============================================================


class TestBraceVarSubstitution:
    def test_brace_simple(self):
        toks = lex("${foo}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "foo"

    def test_brace_hyphen(self):
        toks = lex("${weird-name}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "weird-name"

    def test_brace_space(self):
        toks = lex("${with space}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "with space"

    def test_brace_dot(self):
        toks = lex("${a.b}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "a.b"

    def test_brace_utf8(self):
        toks = lex("${café}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "café"

    def test_brace_empty_name(self):
        # ``${}`` -- empty variable name; valid in Tcl 9.0.3.
        toks = lex("${}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == ""

    def test_brace_with_newline(self):
        toks = lex("${a\nb}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "a\nb"

    def test_brace_array_element(self):
        toks = lex("${arr(x)}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr(x)"

    def test_brace_escape_close_brace(self):
        # ``${a\}b}`` -- per Tcl 9.0.3 ``Tcl_ParseVarName``, ``\X``
        # consumes 2 chars; ``}`` does NOT close.  Name = ``a\}b``.
        toks = lex("${a\\}b}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "a\\}b"

    def test_brace_inner_balanced(self):
        # ``${a{b}c}`` -- inner braces tracked with brace counting.
        toks = lex("${a{b}c}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "a{b}c"


# =============================================================
# §C. ``${::var}`` vs ``::${var}`` -- adjacency
# =============================================================


class TestNamespaceAdjacency:
    def test_brace_qualified(self):
        # ``${::tracevar}`` is one VAR token.
        toks = lex("${::tracevar}")
        assert len(toks) == 1
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "::tracevar"

    def test_literal_colons_then_brace(self):
        # ``::${tracevar}`` is two tokens -- literal ``::`` + VAR.
        toks = lex("::${tracevar}")
        types = [t.type for t in toks]
        assert TokenType.ESC in types
        assert TokenType.VAR in types
        var_tok = next(t for t in toks if t.type == TokenType.VAR)
        assert var_tok.text == "tracevar"

    def test_literal_colons_then_qualified_brace(self):
        # ``::${::tracevar}`` is two tokens -- literal ``::`` + VAR(::tracevar).
        toks = lex("::${::tracevar}")
        var_tok = next(t for t in toks if t.type == TokenType.VAR)
        assert var_tok.text == "::tracevar"


# =============================================================
# §D. Bare/brace composition
# =============================================================


class TestBareBraceComposition:
    def test_bare_qualified(self):
        # ``$::myns::x`` reads ``::myns::x`` as one VAR token.
        toks = lex("$::myns::x")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "::myns::x"

    def test_brace_qualified(self):
        # ``${::myns::x}`` reads ``::myns::x`` as one VAR token.
        toks = lex("${::myns::x}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "::myns::x"

    def test_bare_then_brace_does_not_compose(self):
        # ``$::myns::${suffix}`` is two tokens: bare-form ``::myns::``
        # (with trailing ``::``) + brace-form ``suffix``.  Bare form
        # stops at ``$`` of ``${...}``.
        toks = lex("$::myns::${suffix}")
        var_toks = [t for t in toks if t.type == TokenType.VAR]
        assert len(var_toks) == 2
        assert var_toks[0].text == "::myns::"
        assert var_toks[1].text == "suffix"


# =============================================================
# §E. Array element substitution forms
# =============================================================


class TestArrayElementForms:
    def test_bare_array_literal(self):
        toks = lex("$arr(x)")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr(x)"

    def test_bare_array_with_dollar_index(self):
        toks = lex("$arr($k)")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr($k)"

    def test_brace_array_literal_index(self):
        toks = lex("${arr(idx)}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr(idx)"

    def test_brace_array_with_dollar_index(self):
        # ``${arr($k)}`` parses as one VAR token; runtime treats $k
        # literally (the W216 trap, see TestW216 below).
        toks = lex("${arr($k)}")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr($k)"

    def test_brace_then_paren_separate_tokens(self):
        # ``${arr}(idx)`` -- VAR(``arr``) + ESC(``(idx)``).  W216.
        toks = lex("${arr}(idx)")
        assert toks[0].type == TokenType.VAR
        assert toks[0].text == "arr"
        assert toks[1].type == TokenType.ESC
        assert toks[1].text == "(idx)"


# =============================================================
# §F. Globals & namespaces (analyser-level)
# =============================================================


class TestGlobalsAndNamespaces:
    def test_global_unqualified_offered_qualified_from_namespace(self):
        # ``set foo 1`` at top level creates an unqualified global.
        # From inside ``namespace eval ::ns`` (or any non-global
        # scope), Tcl's runtime resolution does NOT make bare ``$foo``
        # reach the global -- attempting it errors with ``can't read
        # "foo"``.  Completion must therefore offer the qualified
        # form ``$::foo`` from those scopes.
        from lsp.features.completion import get_completions

        source = "set foo 1\nnamespace eval ::ns {\n    puts $\n}"
        items = get_completions(source, 2, 10)
        labels = [i.label for i in items]
        assert "$::foo" in labels, f"expected ``$::foo`` qualified, got {labels}"
        # Bare ``$foo`` would not reach the global -- must NOT appear.
        assert "$foo" not in labels, f"bare ``$foo`` from namespace is unreachable; got {labels}"

    def test_global_unqualified_stays_bare_at_top_level(self):
        # At the global scope itself, bare ``$foo`` is correct.
        from lsp.features.completion import get_completions

        source = "set foo 1\nputs $"
        items = get_completions(source, 1, 6)
        labels = [i.label for i in items]
        assert "$foo" in labels

    def test_global_decl_bare(self):
        a = analyse("set ::g 5\nproc f {} {global g; return $g}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "g" in proc_scope.variables

    def test_global_decl_qualified_strips_qualifier(self):
        # ``global ::g`` creates local alias ``g`` (qualifier stripped).
        a = analyse("set ::g 5\nproc f {} {global ::g; return $g}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "g" in proc_scope.variables, (
            f"expected ``g`` (qualifier stripped) in proc scope, got {list(proc_scope.variables)}"
        )

    def test_global_decl_namespaced(self):
        # ``global ::ns::x`` creates local alias ``x`` (last segment).
        a = analyse("namespace eval ::ns {set ::ns::x 7}\nproc f {} {global ::ns::x; return $x}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "x" in proc_scope.variables

    def test_variable_namespace_eval(self):
        a = analyse("namespace eval ::ns {variable v 1}")
        ns_scope = next(c for c in a.global_scope.children if c.kind == "namespace")
        assert "v" in ns_scope.variables

    def test_variable_decl_in_proc(self):
        # ``variable v`` inside a proc body aliases the namespace var.
        a = analyse(
            "namespace eval ::ns {\n    variable v 11\n    proc f {} {variable v; return $v}\n}"
        )
        ns_scope = next(c for c in a.global_scope.children if c.kind == "namespace")
        proc_scope = next(c for c in ns_scope.children if c.kind == "proc")
        assert "v" in proc_scope.variables


# =============================================================
# §G. upvar / namespace upvar (analyser-level)
# =============================================================


class TestUpvarRegistration:
    def test_upvar_explicit_level(self):
        a = analyse("proc f {var} {upvar 1 $var local; set local 9}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "local" in proc_scope.variables

    def test_upvar_no_level_default_1(self):
        a = analyse("proc f {var} {upvar $var local; set local 9}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "local" in proc_scope.variables

    def test_upvar_zero_current_scope(self):
        # ``upvar 0 x y`` aliases y to x in the current scope -- aliases
        # are pairs after the level; ``y`` is at index 2.
        a = analyse("proc f {} {set x 1; upvar 0 x y; return $y}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "y" in proc_scope.variables

    def test_upvar_pound_zero(self):
        a = analyse("proc f {} {upvar #0 ::g local; return $local}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "local" in proc_scope.variables

    def test_upvar_multiple_pairs(self):
        a = analyse("proc f {a b} {upvar 1 $a la $b lb; set la 1; set lb 2}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "la" in proc_scope.variables
        assert "lb" in proc_scope.variables

    def test_namespace_upvar_alias(self):
        a = analyse(
            "namespace eval ::nsu {variable x 1}\n"
            "proc f {} {namespace upvar ::nsu x local; set local 99}"
        )
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "local" in proc_scope.variables


# =============================================================
# §H. uplevel
# =============================================================


class TestUplevel:
    def test_uplevel_zero_synthesises_global_scope(self):
        # ``uplevel #0`` body is analysed in a synthetic Scope whose
        # parent-walk redirects to the global scope.
        a = analyse("proc f {} {uplevel #0 {set ::created 99}}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        synth = next((c for c in proc_scope.children if c.kind == "uplevel"), None)
        assert synth is not None
        assert synth.body_range is not None

    def test_uplevel_zero_writes_register_in_synthetic_scope(self):
        # ``set ::created 99`` inside ``uplevel #0`` is currently
        # recorded against the synthetic ``Scope(kind="uplevel")`` that
        # wraps the body, not against the global scope directly.  The
        # parent-walk redirect ensures completion *inside* the body
        # sees the var, but cross-file consumers reading
        # ``global_scope.variables`` won't find it.  Pinned as a
        # documented limitation.
        a = analyse("proc f {} {uplevel #0 {set ::created 99}}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        synth = next(c for c in proc_scope.children if c.kind == "uplevel")
        assert "::created" in synth.variables

    def test_uplevel_one_keeps_caller_scope(self):
        # ``uplevel 1`` (or default) doesn't synthesise an alternate
        # scope -- best-effort behaviour leaves the body in the calling
        # proc's scope.
        a = analyse("proc f {} {set local 5; uplevel 1 {puts $local}}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        # No uplevel synthetic scope.
        assert all(c.kind != "uplevel" for c in proc_scope.children)


# =============================================================
# §I. dict bindings (analyser-level)
# =============================================================


class TestDictBindings:
    def test_dict_for_loop_vars(self):
        a = analyse("proc f {} {dict for {k v} {a 1 b 2} {puts $k$v}}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "k" in proc_scope.variables
        assert "v" in proc_scope.variables

    def test_dict_update_aliases(self):
        a = analyse("proc f {} {set d {a 1 b 2}; dict update d a la b lb {set la 0}}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        assert "la" in proc_scope.variables
        assert "lb" in proc_scope.variables

    def test_dict_with_const_literal(self):
        a = analyse("proc f {} {set d {name alice age 30}; dict with d {puts $name}}")
        proc_scope = next(c for c in a.global_scope.children if c.kind == "proc")
        # Const-dict-aware extraction puts ``name`` and ``age`` in the
        # proc scope so the body's ``$name`` reference resolves.
        assert "name" in proc_scope.variables
        assert "age" in proc_scope.variables


# =============================================================
# §J. Quoting interactions
# =============================================================


class TestQuotingInteractions:
    def test_dollar_substitutes_in_quotes(self):
        toks = lex('"x is $x"')
        # First ESC ``x is `` then VAR ``x`` then ESC ``""``.
        assert any(t.type == TokenType.VAR and t.text == "x" for t in toks)

    def test_dollar_literal_in_braces(self):
        toks = lex("{x is $x}")
        # Braced literal -- one STR token, no VAR.
        assert all(t.type != TokenType.VAR for t in toks)

    def test_command_subst_in_quotes(self):
        toks = lex('"[expr {2+2}] is four"')
        assert any(t.type == TokenType.CMD for t in toks)


# =============================================================
# §K. The W216 trap (broken brace-form array element refs)
# =============================================================


class TestW216:
    def test_w216_pattern1_brace_then_paren(self):
        # ``${arr}(name)`` -- W216 fires.
        a = analyse("set arr(name) 1\nputs ${arr}(name)")
        w216 = [d for d in a.diagnostics if d.code == "W216"]
        assert any("scalar" in d.message for d in w216)

    def test_w216_pattern2_brace_with_dollar_index(self):
        # ``${arr($foo)}`` -- W216 fires when index has substitution.
        a = analyse("set foo bar\nputs ${arr($foo)}")
        w216 = [d for d in a.diagnostics if d.code == "W216"]
        assert any("does not substitute" in d.message for d in w216)

    def test_w216_does_not_fire_on_literal_index(self):
        # ``${arr(name)}`` (literal index) is correct -- no W216.
        a = analyse("set arr(name) 1\nputs ${arr(name)}")
        assert not any(d.code == "W216" for d in a.diagnostics)

    def test_w216_fix_uses_bare_when_possible(self):
        a = analyse("puts ${arr}(name)")
        w216 = next(d for d in a.diagnostics if d.code == "W216")
        assert w216.fixes
        assert w216.fixes[0].new_text == "$arr(name)"

    def test_w216_fix_falls_back_to_set_for_funny_name(self):
        a = analyse('set "funny name" 1\nputs ${funny name($foo)}')
        w216 = next(d for d in a.diagnostics if d.code == "W216")
        assert w216.fixes[0].new_text == '[set "funny name($foo)"]'


# =============================================================
# §L. The W215 trap (var names unreachable via $-substitution)
# =============================================================


class TestW215:
    def test_w215_brace_in_name(self):
        a = analyse('set "weird\\}name" 1')
        w215 = [d for d in a.diagnostics if d.code == "W215"]
        assert any("not reachable via $-substitution" in d.message for d in w215)

    def test_w215_paren_in_array_index(self):
        a = analyse('set "arr(weird)stuff)" 1')
        w215 = [d for d in a.diagnostics if d.code == "W215"]
        assert len(w215) >= 1
        assert any("not reachable via $-substitution" in d.message for d in w215)

    def test_w215_does_not_fire_on_normal_names(self):
        a = analyse('set "foo-bar" 1\nset normal 2\nset arr(name) 4')
        assert not any(d.code == "W215" for d in a.diagnostics)


# =============================================================
# §M. Backslash counting (quoted vs braced ``set``)
# =============================================================


class TestBackslashCounting:
    def test_quoted_arg_three_backslashes_lex_raw(self):
        # The lexer captures the RAW source (12 bytes between quotes).
        # Tcl runtime would apply backslash subst to produce a 10-byte
        # name -- this is a known divergence from runtime semantics.
        toks = lex('"back\\\\\\slash"')
        # ESC token's text has all three backslashes (raw).
        esc = next(t for t in toks if t.type == TokenType.ESC)
        assert "\\\\\\" in esc.text  # 3 backslashes raw

    def test_braced_arg_preserves_literally(self):
        # ``set {back\slash} 1`` -- braces preserve every byte.
        # Lexer's STR token has the raw inner content.
        toks = lex("set {back\\slash} 1")
        str_tok = next(t for t in toks if t.type == TokenType.STR)
        # Single backslash in source preserved.
        assert str_tok.text == "back\\slash"

    def test_braced_arg_three_backslashes_preserved(self):
        toks = lex("set {back\\\\\\slash} 1")
        str_tok = next(t for t in toks if t.type == TokenType.STR)
        # Three backslashes preserved.
        assert str_tok.text == "back\\\\\\slash"
