"""Semantic tokens, end-to-end against the packaged server.

Core-coverage port of ``tests/test_semantic_tokens.py``: a fresh document is
opened per case and ``textDocument/semanticTokens/full`` is requested once, then
the delta-encoded ``data`` is decoded and the legend (advertised in
``initialize``) maps type/modifier indices back to names — so a legend or
encoder drift in the *shipped* artifact is caught here.

This covers every distinct token type and modifier the encoder emits (keyword /
function / variable / string / number / operator / comment, the event /
decorator / namespace splits, the ``defaultLibrary`` and ``definition``
modifiers, escape sequences, regex / format / clock / glob sub-tokenisation, and
E100/E101 error recovery).  The exhaustive long-tail of near-duplicate
string-content variants and the BigIP / APL dialect cases (driven by the
``is_bigip_conf`` / ``is_apl`` encoder flags) remain in
``tests/test_semantic_tokens.py``; the F5 iRules comment cases live in
``tests/lsp_e2e/test_irules_e2e.py``.
"""

from __future__ import annotations

import pytest

from ._lsp_helpers import decode_semantic_tokens, semantic_token_violations


@pytest.fixture
def legend(lsp_server):
    return lsp_server.initialize_result["capabilities"]["semanticTokensProvider"]["legend"][
        "tokenTypes"
    ]


@pytest.fixture
def modifiers(lsp_server):
    return lsp_server.initialize_result["capabilities"]["semanticTokensProvider"]["legend"][
        "tokenModifiers"
    ]


def _typed(lsp_server, legend, uri):
    tokens = decode_semantic_tokens(lsp_server.semantic_tokens(uri))
    for tok in tokens:
        tok["type"] = legend[tok["type"]]
    return tokens


def _open(lsp_server, uri_factory, source):
    uri = uri_factory()
    lsp_server.open_ready(uri, source)
    return uri


# Representative + adversarial documents the encoder must keep coherent.  The
# point is not what tokens come back but that they always satisfy the universal
# invariants (ordered, non-overlapping, in-bounds, UTF-16-correct, legend-valid)
# — the "Overlapping semantic tokens detected" class of client warning.
_INVARIANT_CORPUS = {
    "empty": "",
    "simple": "puts hello\n",
    "vars_and_numbers": "set x 42\nset y $x\nputs $y\n",
    "proc_with_body": 'proc greet {name} {\n    puts "Hello $name"\n}\ngreet World\n',
    "nested_blocks": "proc p {} {\n  if {1} {\n    foreach x {a b c} {\n      puts $x\n    }\n  }\n}\n",
    "string_interp": 'set s "a $b [llength $c] d"\n',
    "comments": "# leading comment\nputs hi ;# trailing comment\n",
    "crlf": "proc p {} {\r\n    set x 1\r\n}\r\n",
    # Multibyte / emoji: every column the encoder emits must be UTF-16 units.
    "multibyte_string": 'set greeting "héllo wörld café"\nputs $greeting\n',
    "emoji_string": 'set e "😀 tcl 🚀 rocks 🐫"\nputs $e\n',
    "emoji_then_code": 'puts "🚀"\nset after 1\n',
    "unicode_after_var": 'set x 1\nputs "$x — résumé — 日本語"\n',
    # Recovery paths must still emit coherent tokens.
    "unterminated_bracket": "set x [foo bar\nputs hi\n",
    "unterminated_brace": "proc p {} {\n  set y [foo\n}\nset\n",
    "deep_nesting": "set x [a [b [c [d [e\nputs tail\n",
    "regexp_subtokens": "regexp {(\\d+)-(\\w+)} $s -> a b\n",
    "switch_braced": "switch $x {\n  {a b} { puts one }\n  default { puts def }\n}\n",
}


class TestTokenInvariants:
    """Universal semantic-token invariants over a representative/adversarial corpus.

    ``edit_tracking_stress`` proves the *edited* buffer matches a fresh open, but
    if both are equally wrong (e.g. overlapping tokens) that oracle is blind.
    These pin the absolute contract a client depends on for every document.
    """

    @pytest.mark.parametrize("name", sorted(_INVARIANT_CORPUS))
    def test_tokens_satisfy_invariants(self, name, lsp_server, uri_factory, legend, modifiers):
        source = _INVARIANT_CORPUS[name]
        uri = _open(lsp_server, uri_factory, source)
        raw = lsp_server.semantic_tokens(uri)
        violations = semantic_token_violations(
            raw, source, token_types=legend, token_modifiers=modifiers
        )
        assert not violations, f"[{name}] semantic-token invariant violations:\n" + "\n".join(
            violations
        )

    def test_tokens_strictly_non_overlapping_dense_line(
        self, lsp_server, uri_factory, legend, modifiers
    ):
        # A single dense line with many adjacent tokens is the worst case for the
        # overlap/ordering invariant.
        source = 'set a 1;set b 2;puts "$a [expr {$a+$b}] $b";# tail\n'
        uri = _open(lsp_server, uri_factory, source)
        raw = lsp_server.semantic_tokens(uri)
        assert not semantic_token_violations(
            raw, source, token_types=legend, token_modifiers=modifiers
        )


class TestCoreTokens:
    def test_simple_puts(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "puts hello\n")
        tokens = _typed(lsp_server, legend, uri)
        assert len(tokens) == 2
        assert tokens[0]["type"] == "function"
        assert tokens[0]["length"] == 4
        assert tokens[1]["type"] == "string"

    def test_variable(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "set x $y\n")
        types = [t["type"] for t in _typed(lsp_server, legend, uri)]
        assert "function" in types
        assert "variable" in types

    def test_number(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "set x 42\n")
        nums = [t for t in _typed(lsp_server, legend, uri) if t["type"] == "number"]
        assert len(nums) == 1
        assert nums[0]["length"] == 2

    def test_comment(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "# hello world\n")
        assert _typed(lsp_server, legend, uri)[0]["type"] == "comment"

    def test_proc_as_keyword(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "proc foo {x} {}\n")
        assert _typed(lsp_server, legend, uri)[0]["type"] == "keyword"

    def test_user_command_as_function(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "mycommand arg1\n")
        assert _typed(lsp_server, legend, uri)[0]["type"] == "function"

    def test_operator(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "+ 3 4\n")
        assert _typed(lsp_server, legend, uri)[0]["type"] == "operator"

    def test_multiline_positions(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "set x 1\nset y 2\n")
        sets = [
            t
            for t in _typed(lsp_server, legend, uri)
            if t["type"] == "function" and t["length"] == 3
        ]
        assert len(sets) == 2
        assert sets[0]["line"] == 0
        assert sets[1]["line"] == 1

    def test_data_is_multiple_of_5(self, lsp_server, uri_factory):
        uri = _open(lsp_server, uri_factory, "set x [+ 1 2]\nputs $x\n")
        assert len(lsp_server.semantic_tokens(uri)["data"]) % 5 == 0

    def test_empty_source(self, lsp_server, uri_factory):
        uri = _open(lsp_server, uri_factory, "")
        assert lsp_server.semantic_tokens(uri)["data"] == []

    def test_string_in_quotes(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, 'puts "hello"\n')
        types = [t["type"] for t in _typed(lsp_server, legend, uri)]
        assert "function" in types
        assert "string" in types

    def test_braced_string(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "puts {hello world}\n")
        assert "string" in [t["type"] for t in _typed(lsp_server, legend, uri)]

    def test_if_elseif_else_body_recursion(self, lsp_server, uri_factory, legend):
        uri = _open(
            lsp_server,
            uri_factory,
            "if {$x} { set a 1 } elseif {$y} { set b 2 } else { set c 3 }\n",
        )
        sets = [
            t
            for t in _typed(lsp_server, legend, uri)
            if t["type"] == "function" and t["length"] == 3
        ]
        assert len(sets) == 3

    def test_if_expression_tokenised(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "if {$x > 0} { puts ok }\n")
        types = [t["type"] for t in _typed(lsp_server, legend, uri)]
        assert {"variable", "operator", "number"} <= set(types)

    def test_command_subst_inside_expression(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "set n [expr {[llength $xs] + 1}]\n")
        tokens = _typed(lsp_server, legend, uri)
        assert any(t["type"] == "function" and t["length"] == 7 for t in tokens)
        assert any(t["type"] == "variable" for t in tokens)


_RE_TYPES = {
    "regexp",
    "regexpAnchor",
    "regexpCharClass",
    "regexpQuantifier",
    "regexpGroup",
    "regexpEscape",
    "regexpBackref",
    "regexpAlternation",
}


class TestRegexTokens:
    def test_regexp_pattern_braced(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "regexp {^[a-z]+$} $str\n")
        tokens = _typed(lsp_server, legend, uri)
        assert tokens[0]["type"] == "function"
        assert [t for t in tokens if t["type"] in _RE_TYPES]
        assert any(t["type"] == "variable" for t in tokens)

    def test_regexp_pattern_bare(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "regexp foo $str\n")
        tokens = _typed(lsp_server, legend, uri)
        assert len([t for t in tokens if t["type"] == "regexp"]) == 1

    def test_regexp_with_option_terminator(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "regexp -nocase -- {^test} $str\n")
        tokens = _typed(lsp_server, legend, uri)
        assert [t for t in tokens if t["type"] in _RE_TYPES]

    def test_regsub_pattern(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "regsub {\\d+} $str replacement result\n")
        tokens = _typed(lsp_server, legend, uri)
        assert tokens[0]["type"] == "function"
        cc = [t for t in tokens if t["type"] == "regexpCharClass"]
        assert any(t["length"] == 2 for t in cc)
        assert any(t["type"] == "regexpQuantifier" for t in tokens)

    def test_switch_regexp_braced_case_list(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "switch -regexp $x { {^a} {puts a} {^b} {puts b} }\n")
        tokens = _typed(lsp_server, legend, uri)
        assert len([t for t in tokens if t["type"] in _RE_TYPES]) >= 2

    def test_switch_glob_no_regexp_tokens(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "switch -glob $x {a*} {puts a} {b*} {puts b}\n")
        tokens = _typed(lsp_server, legend, uri)
        assert [t for t in tokens if t["type"] in _RE_TYPES] == []

    def test_regsub_backref_highlighted(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, r"regsub {(\w+)} $str {\1 text} result" + "\n")
        nums = [t for t in _typed(lsp_server, legend, uri) if t["type"] == "number"]
        assert any(t["length"] == 2 for t in nums)

    def test_regsub_multiple_backrefs(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, r"regsub {(a)(b)} $str {\2\1} result" + "\n")
        nums = [
            t for t in _typed(lsp_server, legend, uri) if t["type"] == "number" and t["length"] == 2
        ]
        assert len(nums) == 2


class TestEventDecoratorNamespace:
    def test_when_event_name_highlighted_as_event(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "when HTTP_REQUEST { puts hello }\n")
        events = [t for t in _typed(lsp_server, legend, uri) if t["type"] == "event"]
        assert len(events) == 1
        assert events[0]["length"] == len("HTTP_REQUEST")

    def test_when_lowercase_arg_not_event(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "when some_thing { puts ok }\n")
        assert [t for t in _typed(lsp_server, legend, uri) if t["type"] == "event"] == []

    def test_regexp_option_highlighted_as_decorator(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "regexp -nocase {pat} $str\n")
        decs = [t for t in _typed(lsp_server, legend, uri) if t["type"] == "decorator"]
        assert any(t["length"] == len("-nocase") for t in decs)

    def test_non_option_dash_word_not_decorator(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "puts -foo\n")
        assert [t for t in _typed(lsp_server, legend, uri) if t["type"] == "decorator"] == []

    def test_oo_class_split(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "oo::class create Dog {}\n")
        tokens = _typed(lsp_server, legend, uri)
        assert any(t["type"] == "namespace" and t["length"] == len("oo::") for t in tokens)
        assert any(t["type"] == "keyword" and t["length"] == len("class") for t in tokens)

    def test_global_qualified_command(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "::set x 1\n")
        ns = [t for t in _typed(lsp_server, legend, uri) if t["type"] == "namespace"]
        assert len(ns) == 1
        assert ns[0]["length"] == len("::")


class TestModifiers:
    def _bit(self, modifiers, name):
        return 1 << modifiers.index(name)

    def test_builtin_command_has_default_library(self, lsp_server, uri_factory, legend, modifiers):
        uri = _open(lsp_server, uri_factory, "puts hello\n")
        cmd = next(t for t in _typed(lsp_server, legend, uri) if t["type"] == "function")
        assert cmd["modifiers"] & self._bit(modifiers, "defaultLibrary")

    def test_user_command_no_default_library(self, lsp_server, uri_factory, legend, modifiers):
        uri = _open(lsp_server, uri_factory, "mycommand arg\n")
        cmd = next(t for t in _typed(lsp_server, legend, uri) if t["type"] == "function")
        assert not (cmd["modifiers"] & self._bit(modifiers, "defaultLibrary"))

    def test_subcommand_has_default_library(self, lsp_server, uri_factory, legend, modifiers):
        uri = _open(lsp_server, uri_factory, "string length foo\n")
        sub = [
            t for t in _typed(lsp_server, legend, uri) if t["type"] == "keyword" and t["char"] == 7
        ]
        assert len(sub) == 1
        assert sub[0]["modifiers"] & self._bit(modifiers, "defaultLibrary")

    def test_expr_function_has_default_library(self, lsp_server, uri_factory, legend, modifiers):
        uri = _open(lsp_server, uri_factory, "expr {abs(-1)}\n")
        fn = [
            t
            for t in _typed(lsp_server, legend, uri)
            if t["type"] == "function" and t["length"] == 3
        ]
        assert len(fn) == 1
        assert fn[0]["modifiers"] & self._bit(modifiers, "defaultLibrary")

    def test_proc_definition_no_default_library(self, lsp_server, uri_factory, legend, modifiers):
        uri = _open(lsp_server, uri_factory, "proc greet {name} {}\n")
        defn = self._bit(modifiers, "definition")
        fn = next(
            t
            for t in _typed(lsp_server, legend, uri)
            if t["type"] == "function" and (t["modifiers"] & defn)
        )
        assert not (fn["modifiers"] & self._bit(modifiers, "defaultLibrary"))


class TestStringContentTokens:
    def test_backslash_n_highlighted(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, r"set x hello\nworld" + "\n")
        esc = [t for t in _typed(lsp_server, legend, uri) if t["type"] == "escape"]
        assert len(esc) == 1
        assert esc[0]["length"] == 2

    def test_format_specifier_highlighted(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, 'format "%s %d" "hello" 123\n')
        tokens = _typed(lsp_server, legend, uri)
        assert len([t for t in tokens if t["type"] == "formatPercent"]) == 2
        assert len([t for t in tokens if t["type"] == "formatSpec"]) == 2

    def test_format_flags_width_precision(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, 'format "%-10.5f" 3.14159\n')
        tokens = _typed(lsp_server, legend, uri)
        assert len([t for t in tokens if t["type"] == "formatPercent"]) == 1
        assert len([t for t in tokens if t["type"] == "formatFlag"]) == 2
        assert len([t for t in tokens if t["type"] == "formatSpec"]) == 1
        assert len([t for t in tokens if t["type"] == "formatWidth"]) == 2

    def test_clock_format_specifiers(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, 'clock format $t -format "%Y-%m-%d"\n')
        tokens = _typed(lsp_server, legend, uri)
        assert len([t for t in tokens if t["type"] == "clockPercent"]) == 3
        assert len([t for t in tokens if t["type"] == "clockSpec"]) == 3

    def test_clock_format_locale_modifier(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, 'clock format $t -format "%EY"\n')
        tokens = _typed(lsp_server, legend, uri)
        assert len([t for t in tokens if t["type"] == "clockPercent"]) == 1
        assert len([t for t in tokens if t["type"] == "clockModifier"]) == 1
        assert len([t for t in tokens if t["type"] == "clockSpec"]) == 1

    def test_clock_without_format_option(self, lsp_server, uri_factory, legend):
        uri = _open(lsp_server, uri_factory, "clock format $t -gmt true\n")
        assert [t for t in _typed(lsp_server, legend, uri) if t["type"] == "clockPercent"] == []
