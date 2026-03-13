"""Tests for the semantic token encoder."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from lsp.features.semantic_tokens import (
    SEMANTIC_TOKEN_MODIFIERS,
    SEMANTIC_TOKEN_TYPES,
    semantic_tokens_full,
)


def _decode_tokens(data: list[int]) -> list[dict]:
    """Decode the 5-int encoded tokens back into readable dicts."""
    tokens = []
    line = 0
    char = 0
    for i in range(0, len(data), 5):
        delta_line, delta_char, length, type_idx, modifiers = data[i : i + 5]
        if delta_line > 0:
            line += delta_line
            char = delta_char
        else:
            char += delta_char
        tokens.append(
            {
                "line": line,
                "char": char,
                "length": length,
                "type": SEMANTIC_TOKEN_TYPES[type_idx],
                "modifiers": modifiers,
            }
        )
    return tokens


class TestSemanticTokens:
    def test_simple_puts(self):
        tokens = _decode_tokens(semantic_tokens_full("puts hello"))
        assert len(tokens) == 2
        assert tokens[0]["type"] == "keyword"
        assert tokens[0]["length"] == 4
        assert tokens[1]["type"] == "string"

    def test_variable(self):
        tokens = _decode_tokens(semantic_tokens_full("set x $y"))
        types = [t["type"] for t in tokens]
        assert "keyword" in types  # 'set'
        assert "variable" in types  # '$y'

    def test_number(self):
        tokens = _decode_tokens(semantic_tokens_full("set x 42"))
        num_tokens = [t for t in tokens if t["type"] == "number"]
        assert len(num_tokens) == 1
        assert num_tokens[0]["length"] == 2

    def test_comment(self):
        tokens = _decode_tokens(semantic_tokens_full("# hello world"))
        assert tokens[0]["type"] == "comment"

    def test_comment_with_namespace_qualifiers(self):
        """Comments containing :: should remain single comment tokens, not namespace-split."""
        source = "# TCP::collect / TCP::payload / TCP::release"
        tokens = _decode_tokens(semantic_tokens_full(source, is_irules=True))
        assert len(tokens) == 1
        assert tokens[0]["type"] == "comment"
        assert tokens[0]["length"] == len(source)

    def test_comment_header_block_all_comments(self):
        """Multi-line comment header with :: refs should produce only comment tokens."""
        source = (
            "# Flow:\n"
            "#   1. CLIENT_ACCEPTED / SERVER_CONNECTED -> TCP::collect\n"
            "#   2. CLIENT_DATA    / SERVER_DATA      -> TCP::payload ... TCP::release\n"
        )
        tokens = _decode_tokens(semantic_tokens_full(source, is_irules=True))
        assert all(t["type"] == "comment" for t in tokens), (
            f"Expected all comment tokens, got: {[(t['type'], t['line']) for t in tokens]}"
        )

    def test_proc_as_keyword(self):
        tokens = _decode_tokens(semantic_tokens_full("proc foo {x} {}"))
        assert tokens[0]["type"] == "keyword"  # 'proc'

    def test_user_command_as_function(self):
        tokens = _decode_tokens(semantic_tokens_full("mycommand arg1"))
        assert tokens[0]["type"] == "function"  # unknown command = function

    def test_multiline_positions(self):
        source = "set x 1\nset y 2"
        tokens = _decode_tokens(semantic_tokens_full(source))
        # Second 'set' should be on line 1
        set_tokens = [t for t in tokens if t["type"] == "keyword"]
        assert len(set_tokens) == 2
        assert set_tokens[0]["line"] == 0
        assert set_tokens[1]["line"] == 1

    def test_operator(self):
        tokens = _decode_tokens(semantic_tokens_full("+ 3 4"))
        assert tokens[0]["type"] == "operator"

    def test_data_is_multiple_of_5(self):
        data = semantic_tokens_full("set x [+ 1 2]\nputs $x")
        assert len(data) % 5 == 0

    def test_empty_source(self):
        assert semantic_tokens_full("") == []

    def test_string_in_quotes(self):
        tokens = _decode_tokens(semantic_tokens_full('puts "hello"'))
        types = [t["type"] for t in tokens]
        assert "keyword" in types
        assert "string" in types

    def test_braced_string(self):
        tokens = _decode_tokens(semantic_tokens_full("puts {hello world}"))
        types = [t["type"] for t in tokens]
        assert "string" in types

    def test_if_elseif_else_body_recursion(self):
        source = "if {$x} { set a 1 } elseif {$y} { set b 2 } else { set c 3 }"
        tokens = _decode_tokens(semantic_tokens_full(source))
        set_keywords = [t for t in tokens if t["type"] == "keyword" and t["length"] == 3]
        assert len(set_keywords) == 3

    def test_if_expression_tokenised(self):
        source = "if {$x > 0} { puts ok }"
        tokens = _decode_tokens(semantic_tokens_full(source))
        types = [t["type"] for t in tokens]
        assert "variable" in types
        assert "operator" in types
        assert "number" in types

    def test_command_subst_inside_expression_tokenised(self):
        source = "set n [expr {[llength $xs] + 1}]"
        tokens = _decode_tokens(semantic_tokens_full(source))
        assert any(t["type"] == "keyword" and t["length"] == 7 for t in tokens)  # llength
        assert any(t["type"] == "variable" for t in tokens)

    def test_regexp_pattern_braced(self):
        source = "regexp {^[a-z]+$} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        assert tokens[0]["type"] == "keyword"  # 'regexp'
        # Sub-tokenised: ^ → anchor, [a-z] → charClass, + → quantifier, $ → anchor
        re_types = {
            "regexp",
            "regexpAnchor",
            "regexpCharClass",
            "regexpQuantifier",
            "regexpGroup",
            "regexpEscape",
            "regexpBackref",
            "regexpAlternation",
        }
        regex_tokens = [t for t in tokens if t["type"] in re_types]
        assert len(regex_tokens) >= 1  # at least one regex-domain token
        assert any(t["type"] == "variable" for t in tokens)  # $str

    def test_regexp_pattern_bare(self):
        source = "regexp foo $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        assert tokens[0]["type"] == "keyword"  # 'regexp'
        # Bare literal "foo" with no metacharacters → single regexp token
        regex_tokens = [t for t in tokens if t["type"] == "regexp"]
        assert len(regex_tokens) == 1

    def test_regexp_with_options(self):
        source = "regexp -nocase {pattern} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        assert tokens[0]["type"] == "keyword"  # 'regexp'
        regex_tokens = [t for t in tokens if t["type"] == "regexp"]
        assert len(regex_tokens) == 1

    def test_regexp_with_option_terminator(self):
        source = "regexp -nocase -- {^test} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        re_types = {
            "regexp",
            "regexpAnchor",
            "regexpCharClass",
            "regexpQuantifier",
            "regexpGroup",
            "regexpEscape",
            "regexpBackref",
            "regexpAlternation",
        }
        regex_tokens = [t for t in tokens if t["type"] in re_types]
        assert len(regex_tokens) >= 1

    def test_regsub_pattern(self):
        source = "regsub {\\d+} $str replacement result"
        tokens = _decode_tokens(semantic_tokens_full(source))
        assert tokens[0]["type"] == "keyword"  # 'regsub'
        # \d+ is sub-tokenized: \d → regexpCharClass, + → regexpQuantifier
        cc_tokens = [t for t in tokens if t["type"] == "regexpCharClass"]
        assert any(t["length"] == 2 for t in cc_tokens)  # \d
        assert any(t["type"] == "regexpQuantifier" for t in tokens)  # +

    def test_regexp_with_start_option(self):
        source = "regexp -start 5 {pattern} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        regex_tokens = [t for t in tokens if t["type"] == "regexp"]
        assert len(regex_tokens) == 1

    def test_switch_regexp_braced_case_list(self):
        source = "switch -regexp $x { {^a} {puts a} {^b} {puts b} }"
        tokens = _decode_tokens(semantic_tokens_full(source))
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
        regex_tokens = [t for t in tokens if t["type"] in _RE_TYPES]
        # ^a and ^b → 2 anchors + 2 literals = 4
        assert len(regex_tokens) >= 2

    def test_switch_regexp_inline_form(self):
        source = "switch -regexp $x {^a} {puts a} {^b} {puts b}"
        tokens = _decode_tokens(semantic_tokens_full(source))
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
        regex_tokens = [t for t in tokens if t["type"] in _RE_TYPES]
        assert len(regex_tokens) >= 2

    def test_switch_glob_no_regexp_tokens(self):
        source = "switch -glob $x {a*} {puts a} {b*} {puts b}"
        tokens = _decode_tokens(semantic_tokens_full(source))
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
        regex_tokens = [t for t in tokens if t["type"] in _RE_TYPES]
        assert len(regex_tokens) == 0

    def test_switch_regexp_default_not_regex(self):
        source = "switch -regexp $x { {^a} {puts a} default {puts other} }"
        tokens = _decode_tokens(semantic_tokens_full(source))
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
        regex_tokens = [t for t in tokens if t["type"] in _RE_TYPES]
        # Only ^a → 1 anchor + 1 literal
        assert len(regex_tokens) >= 1

    def test_tcloo_class_body_and_method_body_recurse(self):
        source = (
            "oo::class create Dog {\n"
            "    constructor {n} {\n"
            "        set name $n\n"
            "    }\n"
            "    method bark {} {\n"
            "        puts $name\n"
            "    }\n"
            "}"
        )
        tokens = _decode_tokens(semantic_tokens_full(source))

        # oo::class is split into namespace (oo::) + keyword (class)
        assert any(t["type"] == "namespace" and t["length"] == len("oo::") for t in tokens)
        assert any(t["type"] == "keyword" and t["length"] == len("class") for t in tokens)
        assert any(t["type"] == "keyword" and t["length"] == len("constructor") for t in tokens)
        assert any(t["type"] == "keyword" and t["length"] == len("method") for t in tokens)

        # Nested method/constructor bodies should recurse so these inner commands
        # are highlighted as keywords (instead of one big string token).
        assert any(t["type"] == "keyword" and t["length"] == len("set") for t in tokens)
        assert any(t["type"] == "keyword" and t["length"] == len("puts") for t in tokens)
        assert any(t["type"] == "variable" for t in tokens)

    def test_tcloo_define_method_body_recurses(self):
        source = "oo::define Dog method bark {} {\n    set message bark\n}"
        tokens = _decode_tokens(semantic_tokens_full(source))
        assert any(t["type"] == "namespace" and t["length"] == len("oo::") for t in tokens)
        assert any(t["type"] == "keyword" and t["length"] == len("define") for t in tokens)
        assert any(t["type"] == "keyword" and t["length"] == len("set") for t in tokens)

    def test_switch_braced_case_list_recurses_case_bodies(self):
        source = "switch $x {a {set a 1} b {set b 2} default {set c 3}}"
        tokens = _decode_tokens(semantic_tokens_full(source))
        set_keywords = [t for t in tokens if t["type"] == "keyword" and t["length"] == len("set")]
        assert len(set_keywords) == 3

    def test_proc_parameter_list_highlighted_as_parameters(self):
        source = 'proc greet {name {title Mr} args} { return "$title $name" }'
        tokens = _decode_tokens(semantic_tokens_full(source))
        params = [t for t in tokens if t["type"] == "parameter"]
        lengths = sorted(t["length"] for t in params)
        assert lengths == sorted([len("name"), len("title"), len("args")])

    def test_proc_name_highlighted_as_function_definition(self):
        source = "proc greet {name} { return $name }"
        tokens = _decode_tokens(semantic_tokens_full(source))
        definition_bit = 1 << 1  # SEMANTIC_TOKEN_MODIFIERS['definition']
        assert any(
            t["type"] == "function"
            and t["length"] == len("greet")
            and (t["modifiers"] & definition_bit) != 0
            for t in tokens
        )

    def test_oo_method_name_highlighted_as_function_definition(self):
        source = "oo::class create Dog { method bark {} { puts bark } }"
        tokens = _decode_tokens(semantic_tokens_full(source))
        definition_bit = 1 << 1  # SEMANTIC_TOKEN_MODIFIERS['definition']
        assert any(
            t["type"] == "function"
            and t["length"] == len("bark")
            and (t["modifiers"] & definition_bit) != 0
            for t in tokens
        )

    def test_binary_format_specifier_tokenised(self):
        source = "binary format c2a*I value bytes count"
        tokens = _decode_tokens(semantic_tokens_full(source))
        spec_tokens = [t for t in tokens if t["type"] == "binarySpec"]
        count_tokens = [t for t in tokens if t["type"] == "binaryCount"]
        flag_tokens = [t for t in tokens if t["type"] == "binaryFlag"]
        # c, a, I are specifiers
        assert len(spec_tokens) == 3
        # * is a flag
        assert len(flag_tokens) == 1
        # 2 repeat count
        assert any(t["length"] == 1 for t in count_tokens)

    def test_binary_scan_specifier_tokenised(self):
        source = "binary scan $blob c2a*I c a count"
        tokens = _decode_tokens(semantic_tokens_full(source))
        spec_tokens = [t for t in tokens if t["type"] == "binarySpec"]
        count_tokens = [t for t in tokens if t["type"] == "binaryCount"]
        flag_tokens = [t for t in tokens if t["type"] == "binaryFlag"]
        assert len(spec_tokens) == 3
        assert len(flag_tokens) == 1
        assert any(t["length"] == 1 for t in count_tokens)

    def test_binary_format_unsigned_modifier_tokenised(self):
        source = "binary format su2 $val"
        tokens = _decode_tokens(semantic_tokens_full(source))
        spec_tokens = [t for t in tokens if t["type"] == "binarySpec"]
        flag_tokens = [t for t in tokens if t["type"] == "binaryFlag"]
        count_tokens = [t for t in tokens if t["type"] == "binaryCount"]
        # s specifier
        assert len(spec_tokens) == 1
        # u modifier is a flag
        assert len(flag_tokens) == 1
        # 2 repeat count
        assert any(t["length"] == 1 for t in count_tokens)

    def test_binary_format_signed_modifier_tokenised(self):
        source = "binary format is $val"
        tokens = _decode_tokens(semantic_tokens_full(source))
        spec_tokens = [t for t in tokens if t["type"] == "binarySpec"]
        flag_tokens = [t for t in tokens if t["type"] == "binaryFlag"]
        # i specifier, s modifier (flag)
        assert len(spec_tokens) == 1
        assert len(flag_tokens) == 1

    def test_subcommand_highlighted_as_keyword(self):
        source = "string length $value"
        tokens = _decode_tokens(semantic_tokens_full(source))
        assert any(t["type"] == "keyword" and t["length"] == len("string") for t in tokens)
        assert any(t["type"] == "keyword" and t["length"] == len("length") for t in tokens)

    def test_namespace_eval_subcommand_highlighted(self):
        source = "namespace eval ns { set x 1 }"
        tokens = _decode_tokens(semantic_tokens_full(source))
        assert any(t["type"] == "keyword" and t["length"] == len("namespace") for t in tokens)
        assert any(t["type"] == "keyword" and t["length"] == len("eval") for t in tokens)


class TestAnalysisDrivenRegexHighlighting:
    """Tests that the semantic token provider uses analysis regex_patterns
    to highlight variables (and their defining ``set`` sites) as ``regexp``
    when the analyser determines they hold regex values."""

    def test_variable_used_in_regexp_highlighted_as_regexp(self):
        source = "set pat {^foo}\nregexp $pat $str"
        analysis = analyse(source)
        tokens = _decode_tokens(semantic_tokens_full(source, analysis=analysis))
        # The $pat reference in regexp should be highlighted as regexp
        regexp_tokens = [t for t in tokens if t["type"] == "regexp"]
        assert len(regexp_tokens) >= 1
        # One of the regexp tokens should be at the $pat variable position
        assert any(t["line"] == 1 and t["length"] == len("$pat") for t in regexp_tokens)

    def test_defining_set_value_highlighted_as_regexp(self):
        source = "set pat {^foo}\nregexp $pat $str"
        analysis = analyse(source)
        tokens = _decode_tokens(semantic_tokens_full(source, analysis=analysis))
        # The {^foo} literal in the set command should also be regexp
        regexp_tokens = [t for t in tokens if t["type"] == "regexp"]
        # There should be at least 2 regexp tokens: the set value and the $pat use
        assert len(regexp_tokens) >= 2

    def test_variable_in_regsub_highlighted_as_regexp(self):
        source = "set re {\\d+}\nregsub $re $str replacement"
        analysis = analyse(source)
        tokens = _decode_tokens(semantic_tokens_full(source, analysis=analysis))
        regexp_tokens = [t for t in tokens if t["type"] == "regexp"]
        assert len(regexp_tokens) >= 1
        assert any(t["line"] == 1 and t["length"] == len("$re") for t in regexp_tokens)

    def test_variable_in_switch_regexp_highlighted(self):
        source = "set pat {^test}\nswitch -regexp $x $pat {puts match}"
        analysis = analyse(source)
        tokens = _decode_tokens(semantic_tokens_full(source, analysis=analysis))
        regexp_tokens = [t for t in tokens if t["type"] == "regexp"]
        assert len(regexp_tokens) >= 1

    def test_non_regex_variable_not_highlighted_as_regexp(self):
        source = "set x hello\nputs $x"
        analysis = analyse(source)
        tokens = _decode_tokens(semantic_tokens_full(source, analysis=analysis))
        regexp_tokens = [t for t in tokens if t["type"] == "regexp"]
        assert len(regexp_tokens) == 0

    def test_without_analysis_no_extra_regexp_tokens(self):
        source = "set pat {^foo}\nregexp $pat $str"
        # Without analysis, $pat is just a variable
        tokens = _decode_tokens(semantic_tokens_full(source))
        regexp_tokens = [t for t in tokens if t["type"] == "regexp"]
        # Only the direct pattern in regexp might be highlighted, but $pat
        # as a VAR token won't be treated as pattern by the standalone provider
        pat_var_regexp = [t for t in regexp_tokens if t["line"] == 1 and t["length"] == len("$pat")]
        assert len(pat_var_regexp) == 0


class TestEventTokenType:
    """Tests for iRule event name highlighting."""

    def test_when_event_name_highlighted_as_event(self):
        source = "when HTTP_REQUEST { puts hello }"
        tokens = _decode_tokens(semantic_tokens_full(source))
        event_tokens = [t for t in tokens if t["type"] == "event"]
        assert len(event_tokens) == 1
        assert event_tokens[0]["length"] == len("HTTP_REQUEST")

    def test_when_event_allcaps_pattern(self):
        source = "when CLIENT_ACCEPTED { set x 1 }"
        tokens = _decode_tokens(semantic_tokens_full(source))
        event_tokens = [t for t in tokens if t["type"] == "event"]
        assert len(event_tokens) == 1

    def test_when_lowercase_arg_not_event(self):
        source = "when some_thing { puts ok }"
        tokens = _decode_tokens(semantic_tokens_full(source))
        event_tokens = [t for t in tokens if t["type"] == "event"]
        assert len(event_tokens) == 0


class TestDecoratorTokenType:
    """Tests for command option/flag highlighting."""

    def test_regexp_option_highlighted_as_decorator(self):
        source = "regexp -nocase {pat} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        dec_tokens = [t for t in tokens if t["type"] == "decorator"]
        assert any(t["length"] == len("-nocase") for t in dec_tokens)

    def test_switch_option_highlighted_as_decorator(self):
        source = "switch -exact $x {a {puts a}}"
        tokens = _decode_tokens(semantic_tokens_full(source))
        dec_tokens = [t for t in tokens if t["type"] == "decorator"]
        assert any(t["length"] == len("-exact") for t in dec_tokens)

    def test_non_option_dash_word_not_decorator(self):
        # "puts" has no OPTION roles, so -foo should stay as string
        source = "puts -foo"
        tokens = _decode_tokens(semantic_tokens_full(source))
        dec_tokens = [t for t in tokens if t["type"] == "decorator"]
        assert len(dec_tokens) == 0


class TestNamespaceTokenType:
    """Tests for namespace-qualified command splitting."""

    def test_oo_class_split(self):
        source = "oo::class create Dog {}"
        tokens = _decode_tokens(semantic_tokens_full(source))
        assert any(t["type"] == "namespace" and t["length"] == len("oo::") for t in tokens)
        assert any(t["type"] == "keyword" and t["length"] == len("class") for t in tokens)

    def test_non_qualified_command_no_namespace(self):
        source = "puts hello"
        tokens = _decode_tokens(semantic_tokens_full(source))
        ns_tokens = [t for t in tokens if t["type"] == "namespace"]
        assert len(ns_tokens) == 0

    def test_global_qualified_command(self):
        source = "::set x 1"
        tokens = _decode_tokens(semantic_tokens_full(source))
        ns_tokens = [t for t in tokens if t["type"] == "namespace"]
        assert len(ns_tokens) == 1
        assert ns_tokens[0]["length"] == len("::")


class TestDefaultLibraryModifier:
    """Tests for the defaultLibrary modifier on built-in tokens."""

    _DL_BIT = 1 << SEMANTIC_TOKEN_MODIFIERS.index("defaultLibrary")

    def test_builtin_command_has_default_library(self):
        tokens = _decode_tokens(semantic_tokens_full("puts hello"))
        cmd = next(t for t in tokens if t["type"] == "keyword")
        assert cmd["modifiers"] & self._DL_BIT

    def test_user_command_no_default_library(self):
        tokens = _decode_tokens(semantic_tokens_full("mycommand arg"))
        cmd = next(t for t in tokens if t["type"] == "function")
        assert not (cmd["modifiers"] & self._DL_BIT)

    def test_subcommand_has_default_library(self):
        tokens = _decode_tokens(semantic_tokens_full("string length foo"))
        # "length" subcommand at char 7 should be keyword + defaultLibrary
        sub = [t for t in tokens if t["type"] == "keyword" and t["char"] == 7]
        assert len(sub) == 1
        assert sub[0]["modifiers"] & self._DL_BIT

    def test_expr_function_has_default_library(self):
        tokens = _decode_tokens(semantic_tokens_full("expr {abs(-1)}"))
        fn = [t for t in tokens if t["type"] == "function" and t["length"] == len("abs")]
        assert len(fn) == 1
        assert fn[0]["modifiers"] & self._DL_BIT

    def test_proc_definition_no_default_library(self):
        tokens = _decode_tokens(semantic_tokens_full("proc greet {name} {}"))
        defn_bit = 1 << SEMANTIC_TOKEN_MODIFIERS.index("definition")
        fn = next(t for t in tokens if t["type"] == "function" and (t["modifiers"] & defn_bit))
        assert not (fn["modifiers"] & self._DL_BIT)


class TestEscapeSequenceTokens:
    """Tests for escape-sequence sub-tokenization inside strings."""

    def test_backslash_n_highlighted(self):
        source = r"set x hello\nworld"
        tokens = _decode_tokens(semantic_tokens_full(source))
        esc_tokens = [t for t in tokens if t["type"] == "escape"]
        assert len(esc_tokens) == 1
        assert esc_tokens[0]["length"] == 2  # \n

    def test_backslash_t_highlighted(self):
        source = r"set x col1\tcol2"
        tokens = _decode_tokens(semantic_tokens_full(source))
        esc_tokens = [t for t in tokens if t["type"] == "escape"]
        assert len(esc_tokens) == 1
        assert esc_tokens[0]["length"] == 2  # \t

    def test_hex_escape_highlighted(self):
        source = r"set x \x0a"
        tokens = _decode_tokens(semantic_tokens_full(source))
        esc_tokens = [t for t in tokens if t["type"] == "escape"]
        assert len(esc_tokens) == 1
        assert esc_tokens[0]["length"] == 4  # \x0a

    def test_unicode_escape_highlighted(self):
        source = r"set x \u00e9"
        tokens = _decode_tokens(semantic_tokens_full(source))
        esc_tokens = [t for t in tokens if t["type"] == "escape"]
        assert len(esc_tokens) == 1
        assert esc_tokens[0]["length"] == 6  # \u00e9

    def test_multiple_escapes_in_one_word(self):
        source = r"set x hello\n\tworld"
        tokens = _decode_tokens(semantic_tokens_full(source))
        esc_tokens = [t for t in tokens if t["type"] == "escape"]
        assert len(esc_tokens) == 2

    def test_no_escapes_in_plain_string(self):
        source = "set x hello"
        tokens = _decode_tokens(semantic_tokens_full(source))
        esc_tokens = [t for t in tokens if t["type"] == "escape"]
        assert len(esc_tokens) == 0

    def test_surrounding_text_still_string(self):
        source = r"set x hello\nworld"
        tokens = _decode_tokens(semantic_tokens_full(source))
        # "hello" and "world" parts should be string tokens
        str_tokens = [t for t in tokens if t["type"] == "string"]
        str_texts = [(t["char"], t["length"]) for t in str_tokens]
        # hello = 5 chars, world = 5 chars
        assert any(length == 5 for _, length in str_texts)

    def test_braced_string_no_escape_tokens(self):
        source = r"set x {hello\nworld}"
        tokens = _decode_tokens(semantic_tokens_full(source))
        esc_tokens = [t for t in tokens if t["type"] == "escape"]
        # Braced strings don't process escapes — should be 0
        assert len(esc_tokens) == 0

    def test_octal_escape_highlighted(self):
        source = r"set x \012"
        tokens = _decode_tokens(semantic_tokens_full(source))
        esc_tokens = [t for t in tokens if t["type"] == "escape"]
        assert len(esc_tokens) == 1
        assert esc_tokens[0]["length"] == 4  # \012


class TestFormatScanStringTokens:
    """Tests for format/scan specifier sub-tokenization inside strings."""

    def test_format_specifier_highlighted(self):
        source = 'format "%s %d" "hello" 123'
        tokens = _decode_tokens(semantic_tokens_full(source))
        pct_tokens = [t for t in tokens if t["type"] == "formatPercent"]
        spec_tokens = [t for t in tokens if t["type"] == "formatSpec"]
        assert len(pct_tokens) == 2  # Two %
        assert len(spec_tokens) == 2  # s, d

    def test_format_specifier_with_positional(self):
        # Must use braces so the lexer doesn't treat $s/$d as variable refs
        source = 'format {%2$s %1$d} "hello" 123'
        tokens = _decode_tokens(semantic_tokens_full(source))
        width_tokens = [t for t in tokens if t["type"] == "formatWidth"]
        # Brace at char 7, so %2$s starts at char 8; '2' at char 9, '1' at char 14
        assert any(t["length"] == 1 and t["char"] == 9 for t in width_tokens)
        assert any(t["length"] == 1 and t["char"] == 14 for t in width_tokens)

    def test_scan_specifier_highlighted(self):
        source = 'scan "123" "%d" myvar'
        tokens = _decode_tokens(semantic_tokens_full(source))
        pct_tokens = [t for t in tokens if t["type"] == "formatPercent"]
        spec_tokens = [t for t in tokens if t["type"] == "formatSpec"]
        assert len(pct_tokens) == 1  # %
        assert len(spec_tokens) == 1  # d

    def test_format_specifier_flags_width_precision(self):
        source = 'format "%-10.5f" 3.14159'
        tokens = _decode_tokens(semantic_tokens_full(source))
        pct_tokens = [t for t in tokens if t["type"] == "formatPercent"]
        flag_tokens = [t for t in tokens if t["type"] == "formatFlag"]
        spec_tokens = [t for t in tokens if t["type"] == "formatSpec"]
        width_tokens = [t for t in tokens if t["type"] == "formatWidth"]
        # %: 1 formatPercent
        assert len(pct_tokens) == 1
        # -: flag, .: flag → 2 flags
        assert len(flag_tokens) == 2
        # f: 1 formatSpec
        assert len(spec_tokens) == 1
        # 10, 5: 2 formatWidth
        assert len(width_tokens) == 2
        assert any(t["length"] == 2 for t in width_tokens)  # 10
        assert any(t["length"] == 1 for t in width_tokens)  # 5

    def test_format_not_confused_by_escaped_percent(self):
        source = 'format "%% %d" 123'
        tokens = _decode_tokens(semantic_tokens_full(source))
        pct_tokens = [t for t in tokens if t["type"] == "formatPercent"]
        spec_tokens = [t for t in tokens if t["type"] == "formatSpec"]
        # %% → % (formatPercent) + % (formatSpec, since '%' is a type char)
        # %d → % (formatPercent) + d (formatSpec)
        assert len(pct_tokens) == 2
        assert len(spec_tokens) == 2


class TestClockFormatStringTokens:
    """Tests for clock format specifier sub-tokenization."""

    def test_clock_format_specifiers(self):
        source = 'clock format $t -format "%Y-%m-%d"'
        tokens = _decode_tokens(semantic_tokens_full(source))
        pct_tokens = [t for t in tokens if t["type"] == "clockPercent"]
        spec_tokens = [t for t in tokens if t["type"] == "clockSpec"]
        # %Y, %m, %d → 3 clockPercent + 3 clockSpec
        assert len(pct_tokens) == 3
        assert len(spec_tokens) == 3

    def test_clock_scan_format_specifiers(self):
        source = 'clock scan $s -format "%Y-%m-%d"'
        tokens = _decode_tokens(semantic_tokens_full(source))
        pct_tokens = [t for t in tokens if t["type"] == "clockPercent"]
        spec_tokens = [t for t in tokens if t["type"] == "clockSpec"]
        assert len(pct_tokens) == 3
        assert len(spec_tokens) == 3

    def test_clock_format_literal_dashes_are_strings(self):
        source = 'clock format $t -format "%Y-%m-%d"'
        tokens = _decode_tokens(semantic_tokens_full(source))
        str_tokens = [t for t in tokens if t["type"] == "string"]
        # Dashes between specifiers
        dash_tokens = [t for t in str_tokens if t["length"] == 1]
        assert len(dash_tokens) >= 2

    def test_clock_format_escaped_percent(self):
        source = 'clock format $t -format "100%%"'
        tokens = _decode_tokens(semantic_tokens_full(source))
        pct_tokens = [t for t in tokens if t["type"] == "clockPercent"]
        spec_tokens = [t for t in tokens if t["type"] == "clockSpec"]
        # %% → % (clockPercent) + % (clockSpec)
        assert len(pct_tokens) == 1
        assert len(spec_tokens) == 1

    def test_clock_without_format_option(self):
        source = "clock format $t -gmt true"
        tokens = _decode_tokens(semantic_tokens_full(source))
        pct_tokens = [t for t in tokens if t["type"] == "clockPercent"]
        assert len(pct_tokens) == 0

    def test_clock_format_locale_modifier(self):
        source = 'clock format $t -format "%EY"'
        tokens = _decode_tokens(semantic_tokens_full(source))
        pct_tokens = [t for t in tokens if t["type"] == "clockPercent"]
        mod_tokens = [t for t in tokens if t["type"] == "clockModifier"]
        spec_tokens = [t for t in tokens if t["type"] == "clockSpec"]
        # % (clockPercent) + E (clockModifier) + Y (clockSpec)
        assert len(pct_tokens) == 1
        assert len(mod_tokens) == 1
        assert len(spec_tokens) == 1


class TestRegsubSubspecTokens:
    """Tests for regsub substitution spec backreference highlighting."""

    def test_regsub_backref_highlighted(self):
        source = r"regsub {(\w+)} $str {\1 text} result"
        tokens = _decode_tokens(semantic_tokens_full(source))
        num_tokens = [t for t in tokens if t["type"] == "number"]
        # \1 should be highlighted as number
        assert any(t["length"] == 2 for t in num_tokens)

    def test_regsub_ampersand_ref(self):
        source = r"regsub {pat} $str {replaced: \&} result"
        tokens = _decode_tokens(semantic_tokens_full(source))
        op_tokens = [t for t in tokens if t["type"] == "operator"]
        assert any(t["length"] == 2 for t in op_tokens)  # \&

    def test_regsub_with_options(self):
        source = r"regsub -nocase {pat} $str {\1} result"
        tokens = _decode_tokens(semantic_tokens_full(source))
        num_tokens = [t for t in tokens if t["type"] == "number"]
        assert any(t["length"] == 2 for t in num_tokens)

    def test_regsub_pattern_still_regexp(self):
        source = r"regsub {(\w+)} $str {\1} result"
        tokens = _decode_tokens(semantic_tokens_full(source))
        # (\w+) is sub-tokenized: ( → regexpGroup, \w → regexpCharClass,
        # + → regexpQuantifier, ) → regexpGroup
        regex_related = [
            t
            for t in tokens
            if t["type"] in ("regexpGroup", "regexpCharClass", "regexpQuantifier", "regexp")
        ]
        assert len(regex_related) >= 3  # at least (, \w, +, )

    def test_regsub_multiple_backrefs(self):
        source = r"regsub {(a)(b)} $str {\2\1} result"
        tokens = _decode_tokens(semantic_tokens_full(source))
        num_tokens = [t for t in tokens if t["type"] == "number"]
        backref_nums = [t for t in num_tokens if t["length"] == 2]
        assert len(backref_nums) == 2


class TestGlobPatternTokens:
    """Tests for glob pattern metacharacter highlighting."""

    def test_string_match_star(self):
        source = "string match {*.tcl} $f"
        tokens = _decode_tokens(semantic_tokens_full(source))
        op_tokens = [t for t in tokens if t["type"] == "operator"]
        assert any(t["length"] == 1 for t in op_tokens)  # *

    def test_string_match_question(self):
        source = "string match {?.tcl} $f"
        tokens = _decode_tokens(semantic_tokens_full(source))
        op_tokens = [t for t in tokens if t["type"] == "operator"]
        assert any(t["length"] == 1 for t in op_tokens)  # ?

    def test_string_match_char_class(self):
        source = "string match {[abc].tcl} $f"
        tokens = _decode_tokens(semantic_tokens_full(source))
        regexp_tokens = [t for t in tokens if t["type"] == "regexp"]
        assert len(regexp_tokens) >= 1  # [abc]

    def test_glob_command_pattern(self):
        source = "glob {*.txt}"
        tokens = _decode_tokens(semantic_tokens_full(source))
        op_tokens = [t for t in tokens if t["type"] == "operator"]
        assert any(t["length"] == 1 for t in op_tokens)  # *

    def test_glob_with_options(self):
        source = "glob -directory $dir {*.txt}"
        tokens = _decode_tokens(semantic_tokens_full(source))
        op_tokens = [t for t in tokens if t["type"] == "operator"]
        assert any(t["length"] == 1 for t in op_tokens)  # *

    def test_lsearch_glob_pattern(self):
        source = "lsearch $list {foo*}"
        tokens = _decode_tokens(semantic_tokens_full(source))
        op_tokens = [t for t in tokens if t["type"] == "operator"]
        assert any(t["length"] == 1 for t in op_tokens)  # *

    def test_lsearch_regexp_no_glob(self):
        source = "lsearch -regexp $list {^foo}"
        tokens = _decode_tokens(semantic_tokens_full(source))
        # With -regexp, no glob metachar operators should appear
        op_tokens = [t for t in tokens if t["type"] == "operator"]
        assert len(op_tokens) == 0

    def test_lsearch_exact_no_glob(self):
        source = "lsearch -exact $list foo"
        tokens = _decode_tokens(semantic_tokens_full(source))
        op_tokens = [t for t in tokens if t["type"] == "operator"]
        assert len(op_tokens) == 0

    def test_string_match_nocase_option(self):
        source = "string match -nocase {*.TCL} $f"
        tokens = _decode_tokens(semantic_tokens_full(source))
        op_tokens = [t for t in tokens if t["type"] == "operator"]
        assert any(t["length"] == 1 for t in op_tokens)  # *

    def test_glob_escape_sequence(self):
        source = r"string match {\*.tcl} $f"
        tokens = _decode_tokens(semantic_tokens_full(source))
        esc_tokens = [t for t in tokens if t["type"] == "escape"]
        assert len(esc_tokens) >= 1  # \*


class TestStringMapPairTokens:
    """Tests for string map mapping list alternating pair colours."""

    def test_two_pairs_alternate(self):
        source = "string map {a b c d} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        # Pair 0 (a, b) → string; Pair 1 (c, d) → parameter
        pair_tokens = [t for t in tokens if t["type"] in ("string", "parameter")]
        assert len(pair_tokens) == 4
        assert pair_tokens[0]["type"] == "string"  # a
        assert pair_tokens[1]["type"] == "string"  # b
        assert pair_tokens[2]["type"] == "parameter"  # c
        assert pair_tokens[3]["type"] == "parameter"  # d

    def test_three_pairs_wrap_around(self):
        source = "string map {a b c d e f} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        pair_tokens = [t for t in tokens if t["type"] in ("string", "parameter")]
        assert len(pair_tokens) == 6
        # Pair 0 → string, Pair 1 → parameter, Pair 2 → string again
        assert pair_tokens[0]["type"] == "string"  # a
        assert pair_tokens[1]["type"] == "string"  # b
        assert pair_tokens[2]["type"] == "parameter"  # c
        assert pair_tokens[3]["type"] == "parameter"  # d
        assert pair_tokens[4]["type"] == "string"  # e
        assert pair_tokens[5]["type"] == "string"  # f

    def test_nocase_option_still_works(self):
        source = "string map -nocase {a b c d} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        # Mapping elements start inside the braces (char >= 20)
        pair_tokens = [
            t for t in tokens if t["type"] in ("string", "parameter") and t["char"] >= 20
        ]
        assert len(pair_tokens) == 4
        assert pair_tokens[0]["type"] == "string"
        assert pair_tokens[2]["type"] == "parameter"

    def test_single_pair_no_alternation(self):
        source = "string map {a b} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        pair_tokens = [t for t in tokens if t["type"] in ("string", "parameter")]
        assert len(pair_tokens) == 2
        assert all(t["type"] == "string" for t in pair_tokens)

    def test_braced_elements(self):
        source = "string map {{hello world} replacement {foo bar} baz} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        pair_tokens = [t for t in tokens if t["type"] in ("string", "parameter")]
        assert len(pair_tokens) == 4
        # {hello world} and replacement = pair 0 → string
        assert pair_tokens[0]["type"] == "string"
        assert pair_tokens[0]["length"] == len("{hello world}")
        assert pair_tokens[1]["type"] == "string"
        # {foo bar} and baz = pair 1 → parameter
        assert pair_tokens[2]["type"] == "parameter"
        assert pair_tokens[3]["type"] == "parameter"

    def test_non_braced_mapping_falls_through(self):
        # Variable mapping — can't sub-tokenize, should not crash
        source = "string map $mapping $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        # $mapping should be a variable token, not pair-coloured
        var_tokens = [t for t in tokens if t["type"] == "variable"]
        assert len(var_tokens) >= 1

    def test_too_few_elements_falls_through(self):
        # Single-element mapping — not enough for pairs
        source = "string map {x} $str"
        tokens = _decode_tokens(semantic_tokens_full(source))
        # Should fall through to default string handling
        str_tokens = [t for t in tokens if t["type"] == "string"]
        assert len(str_tokens) >= 1


class TestE100StrayBracketRecovery:
    """E100: stray ']' recovery for semantic token provider."""

    def test_e100_switch_body_highlighted_after_recovery(self):
        """Switch body content gets proper tokens after E100 recovery."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}"
        )
        tokens = _decode_tokens(semantic_tokens_full(source))
        types = [t["type"] for t in tokens]
        # 'set' inside the switch body should be a keyword (not buried in a string)
        keyword_texts = [
            source.split("\n")[t["line"]][t["char"] : t["char"] + t["length"]]
            for t in tokens
            if t["type"] == "keyword"
        ]
        assert "set" in keyword_texts
        # The virtual CMD content should produce namespace + keyword tokens
        assert "namespace" in types

    def test_e100_recovery_matches_valid_switch(self):
        """E100 recovery produces the same token types as a valid switch."""
        valid_source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}"
        )
        broken_source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            "    }\n"
            "}"
        )
        valid_tokens = _decode_tokens(semantic_tokens_full(valid_source))
        broken_tokens = _decode_tokens(semantic_tokens_full(broken_source))
        # Same number and types of tokens
        assert len(valid_tokens) == len(broken_tokens)
        assert [t["type"] for t in valid_tokens] == [t["type"] for t in broken_tokens]


class TestE101MissingBraceRecovery:
    """E101: orphaned switch case semantic token recovery."""

    def test_e101_orphaned_case_body_highlighted(self):
        """After E101 recovery, orphaned case body gets keyword tokens."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] \n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    }\n"
            "}"
        )
        tokens = _decode_tokens(semantic_tokens_full(source))
        keyword_texts = [
            source.split("\n")[t["line"]][t["char"] : t["char"] + t["length"]]
            for t in tokens
            if t["type"] == "keyword"
        ]
        # Both "set" commands should be keywords
        assert keyword_texts.count("set") == 2
        # Orphaned case pattern should be a string, not a function
        line5_tokens = [t for t in tokens if t["line"] == 5]
        assert any(t["type"] == "string" for t in line5_tokens)
        assert not any(t["type"] == "function" for t in line5_tokens)

    def test_e101_recovery_body_contents_match_valid(self):
        """E101 recovery produces same body token types as valid switch."""
        valid_source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] {\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    }\n"
            "}"
        )
        broken_source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id] \n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    }\n"
            "}"
        )
        valid_tokens = _decode_tokens(semantic_tokens_full(valid_source))
        broken_tokens = _decode_tokens(semantic_tokens_full(broken_source))
        # The body content tokens (set, x, 1, set, y, 2) should be identical.
        # Valid has 12 tokens (no case patterns shown), broken has 15 (patterns + stray })
        # But the keyword/variable/number tokens for body content should match.
        valid_body = [t for t in valid_tokens if t["type"] in ("keyword", "variable", "number")]
        broken_body = [t for t in broken_tokens if t["type"] in ("keyword", "variable", "number")]
        assert [t["type"] for t in valid_body] == [t["type"] for t in broken_body]

    def test_e101_no_trailing_space_recovery(self):
        """E101 recovery works when there's no trailing space after ]."""
        source = (
            "when ACCESS_POLICY_AGENT_EVENT {\n"
            "    switch [ACCESS::policy agent_id]\n"
            '        "get_totp_key" {\n'
            "            set x 1\n"
            "        }\n"
            '        "other_key" {\n'
            "            set y 2\n"
            "        }\n"
            "    }\n"
            "}"
        )
        tokens = _decode_tokens(semantic_tokens_full(source))
        keyword_texts = [
            source.split("\n")[t["line"]][t["char"] : t["char"] + t["length"]]
            for t in tokens
            if t["type"] == "keyword"
        ]
        # Both "set" commands should be keywords
        assert keyword_texts.count("set") == 2


class TestBigipSemanticTokens:
    """Tests for BIG-IP config specific semantic token overlays."""

    def test_highlights_bigip_value_categories(self):
        source = (
            "auth partition foo { }\n"
            "auth user adminUser {\n"
            "    shell bash\n"
            "}\n"
            "auth radius-server /Common/system_auth_name1 {\n"
            "    secret $M$mn$9uYx+bTjLD8YSGZAUEfFwHvpSDwZsL25kZxdn5NZmCI=\n"
            "    server 10.200.244.1\n"
            "}\n"
            "ltm node /Common/api.example.com {\n"
            "    fqdn { name api.example.com }\n"
            "}\n"
            "ltm pool /Common/web_pool {\n"
            "    members { /Common/api.example.com:443 { } }\n"
            "}\n"
            "ltm virtual /Common/app_https_vs {\n"
            "    destination /Common/192.0.2.10%12:443\n"
            "    pool /Common/web_pool\n"
            "    monitor /Common/http\n"
            "    profile /Common/tcp-optimized\n"
            "    vlan /Common/external\n"
            "}\n"
            "net interface 1.1 {\n"
            "    lldp-admin txonly\n"
            "}\n"
        )
        tokens = _decode_tokens(semantic_tokens_full(source, is_bigip_conf=True))
        token_types = {t["type"] for t in tokens}
        assert "object" in token_types
        assert "fqdn" in token_types
        assert "ipAddress" in token_types
        assert "port" in token_types
        assert "routeDomain" in token_types
        assert "partition" in token_types
        assert "username" in token_types
        assert "encrypted" in token_types
        assert "pool" in token_types
        assert "monitor" in token_types
        assert "profile" in token_types
        assert "vlan" in token_types
        assert "interface" in token_types

    def test_highlights_irules_object_references(self):
        source = (
            "when HTTP_REQUEST {\n"
            "    class match [HTTP::host] equals /Common/host_dg\n"
            "    snatpool /Common/sp1\n"
            "    pool /Common/web_pool\n"
            "}\n"
        )
        tokens = _decode_tokens(semantic_tokens_full(source, is_irules=True))
        object_spans = [
            source.split("\n")[t["line"]][t["char"] : t["char"] + t["length"]]
            for t in tokens
            if t["type"] == "object"
        ]
        assert "/Common/host_dg" in object_spans
        assert "/Common/sp1" in object_spans
        assert "/Common/web_pool" in object_spans

    def test_highlights_embedded_irule_object_references_in_bigip_conf(self):
        source = (
            "ltm rule /Common/r1 {\n"
            "    when HTTP_REQUEST {\n"
            "        pool /Common/web_pool\n"
            "    }\n"
            "}\n"
        )
        tokens = _decode_tokens(semantic_tokens_full(source, is_bigip_conf=True))
        object_spans = [
            source.split("\n")[t["line"]][t["char"] : t["char"] + t["length"]]
            for t in tokens
            if t["type"] == "object"
        ]
        assert "/Common/web_pool" in object_spans

    def test_bigip_keyed_refs_use_specific_token_types(self):
        """Keyed references like 'pool X' should use pool token, not generic object."""
        source = (
            "ltm virtual /Common/vs1 {\n"
            "    pool /Common/web_pool\n"
            "    monitor /Common/http\n"
            "    profile /Common/tcp-optimized\n"
            "    vlan /Common/external\n"
            "}\n"
        )
        tokens = _decode_tokens(semantic_tokens_full(source, is_bigip_conf=True))
        lines = source.split("\n")

        def spans_for(typ: str) -> list[str]:
            return [
                lines[t["line"]][t["char"] : t["char"] + t["length"]]
                for t in tokens
                if t["type"] == typ
            ]

        assert "web_pool" in spans_for("pool")
        assert "http" in spans_for("monitor")
        assert "tcp-optimized" in spans_for("profile")
        assert "external" in spans_for("vlan")

    def test_bigip_top_level_declaration_types(self):
        """Top-level declarations like 'ltm pool /Common/x {' use specific types."""
        source = (
            "ltm pool /Common/web_pool {\n"
            "}\n"
            "ltm monitor http /Common/custom_http {\n"
            "}\n"
            "net vlan /Common/external {\n"
            "}\n"
            "net interface 1.1 {\n"
            "}\n"
        )
        tokens = _decode_tokens(semantic_tokens_full(source, is_bigip_conf=True))
        lines = source.split("\n")

        def spans_for(typ: str) -> list[str]:
            return [
                lines[t["line"]][t["char"] : t["char"] + t["length"]]
                for t in tokens
                if t["type"] == typ
            ]

        assert "web_pool" in spans_for("pool")
        assert "custom_http" in spans_for("monitor")
        assert "external" in spans_for("vlan")
        assert "1.1" in spans_for("interface")

    def test_embedded_irule_gets_tcl_tokens(self):
        """Embedded iRule bodies in bigip.conf get full Tcl semantic tokens."""
        source = (
            "ltm rule /Common/my_irule {\n"
            "    when HTTP_REQUEST {\n"
            "        set uri [HTTP::uri]\n"
            '        if { $uri eq "/" } {\n'
            "            pool /Common/web_pool\n"
            "        }\n"
            "    }\n"
            "}\n"
        )
        tokens = _decode_tokens(semantic_tokens_full(source, is_bigip_conf=True))
        token_types = {t["type"] for t in tokens}
        # The embedded body should have full Tcl tokens.
        assert "keyword" in token_types  # 'set', 'if', 'when'
        assert "variable" in token_types  # $uri
        assert "event" in token_types  # HTTP_REQUEST

    def test_embedded_iapp_gets_tcl_tokens(self):
        """iApp implementation bodies in bigip.conf get full Tcl semantic tokens."""
        source = (
            "sys application template /Common/my_tmpl {\n"
            "    actions {\n"
            "        definition {\n"
            "            implementation {\n"
            "                set port 443\n"
            "                if { $port > 0 } {\n"
            "                    puts ok\n"
            "                }\n"
            "            }\n"
            "        }\n"
            "    }\n"
            "}\n"
        )
        tokens = _decode_tokens(semantic_tokens_full(source, is_bigip_conf=True))
        token_types = {t["type"] for t in tokens}
        assert "keyword" in token_types  # 'set', 'if', 'puts'
        assert "variable" in token_types  # $port
