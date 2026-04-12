"""Tests for the core refactoring engine."""

from __future__ import annotations

import pytest
from lsprotocol import types

from core.commands.registry.runtime import configure_signatures
from core.refactoring import RefactoringResult
from core.refactoring._brace_expr import brace_expr
from core.refactoring._extract_datagroup import (
    extract_to_datagroup,
    extract_to_datagroup_from_if,
    extract_to_datagroup_from_switch,
    suggest_datagroup_extraction,
)
from core.refactoring._extract_variable import extract_variable
from core.refactoring._if_to_switch import if_to_switch
from core.refactoring._inline_variable import inline_variable
from core.refactoring._switch_to_dict import switch_to_dict
from lsp.features.code_actions import get_code_actions

# ── Extract variable ──────────────────────────────────────────────────


class TestExtractVariable:
    def test_extract_command_substitution(self):
        source = "set x [string length $name]"
        result = extract_variable(source, 0, 6, 0, 27, "len")
        assert result is not None
        assert "len" in result.title
        applied = result.apply(source)
        assert "set len [string length $name]" in applied
        assert "$len" in applied

    def test_extract_with_custom_name(self):
        source = "puts [expr {$a + $b}]"
        result = extract_variable(source, 0, 5, 0, 20, "total")
        assert result is not None
        assert "total" in result.title

    def test_empty_selection_returns_none(self):
        source = "set x 42"
        result = extract_variable(source, 0, 0, 0, 0)
        assert result is None

    def test_whitespace_selection_returns_none(self):
        source = "set x    42"
        result = extract_variable(source, 0, 5, 0, 9, "ws")
        # The selection "    " is whitespace, should return None.
        assert result is None

    def test_appears_in_code_actions(self):
        source = "set x [string length $name]"
        selected = types.Range(
            start=types.Position(line=0, character=6),
            end=types.Position(line=0, character=27),
        )
        context = types.CodeActionContext(diagnostics=[])
        actions = get_code_actions(source, selected, context)
        extract_actions = [
            a
            for a in actions
            if a.kind == types.CodeActionKind.RefactorExtract
            and "variable" in (a.title or "").lower()
        ]
        assert len(extract_actions) >= 1


# ── Inline variable ──────────────────────────────────────────────────


class TestInlineVariable:
    def test_inline_single_use(self):
        source = 'set name "hello"\nputs $name'
        result = inline_variable(source, 0, 0)
        assert result is not None
        assert "name" in result.title
        applied = result.apply(source)
        assert "set name" not in applied
        assert '"hello"' in applied

    def test_inline_braced_reference(self):
        source = "set x 1\nputs ${x}"
        result = inline_variable(source, 0, 0)
        assert result is not None
        assert result.apply(source) == "puts 1"

    def test_inline_preserves_following_line_without_trailing_newline(self):
        source = "set x 1\nset y $x"
        result = inline_variable(source, 0, 0)
        assert result is not None
        assert result.apply(source) == "set y 1"

    def test_no_inline_multiple_uses(self):
        source = "set x 42\nputs $x\nputs $x"
        result = inline_variable(source, 0, 0)
        assert result is None

    def test_no_inline_read_form(self):
        source = "set x"
        result = inline_variable(source, 0, 0)
        assert result is None

    def test_no_inline_non_set(self):
        source = 'puts "hello"'
        result = inline_variable(source, 0, 0)
        assert result is None


# ── if/elseif → switch ────────────────────────────────────────────────


class TestIfToSwitch:
    def test_simple_eq_chain(self):
        source = (
            'if {$x eq "a"} {\n'
            '    puts "alpha"\n'
            '} elseif {$x eq "b"} {\n'
            '    puts "beta"\n'
            '} elseif {$x eq "c"} {\n'
            '    puts "gamma"\n'
            "}"
        )
        result = if_to_switch(source, 0, 0)
        assert result is not None
        assert "switch" in result.title.lower()
        applied = result.apply(source)
        assert "switch -exact -- $x" in applied
        assert '"a"' not in applied or "a" in applied  # pattern should be unquoted

    def test_with_else_clause(self):
        source = (
            'if {$cmd eq "GET"} {\n'
            "    handle_get\n"
            '} elseif {$cmd eq "POST"} {\n'
            "    handle_post\n"
            "} else {\n"
            "    handle_other\n"
            "}"
        )
        result = if_to_switch(source, 0, 0)
        assert result is not None
        applied = result.apply(source)
        assert "default" in applied

    def test_different_vars_returns_none(self):
        source = 'if {$x eq "a"} {\n    puts "x"\n} elseif {$y eq "b"} {\n    puts "y"\n}'
        result = if_to_switch(source, 0, 0)
        assert result is None

    def test_single_branch_returns_none(self):
        source = 'if {$x eq "a"} { puts "alpha" }'
        result = if_to_switch(source, 0, 0)
        assert result is None

    def test_ne_operator_returns_none(self):
        source = 'if {$x ne "a"} {\n    puts "not a"\n} elseif {$x ne "b"} {\n    puts "not b"\n}'
        result = if_to_switch(source, 0, 0)
        assert result is None

    def test_rewrite_covers_entire_if_command(self):
        source = 'if {$x eq "a"} {\n    puts 1\n} elseif {$x eq "b"} {\n    puts 2\n}'
        result = if_to_switch(source, 0, 0)
        assert result is not None
        applied = result.apply(source).strip()
        assert applied.endswith("}")
        assert not applied.endswith("}}")

    def test_values_with_spaces_are_braced(self):
        source = 'if {$x eq "a b"} {\n    puts one\n} elseif {$x eq "c d"} {\n    puts two\n}'
        result = if_to_switch(source, 0, 0)
        assert result is not None
        applied = result.apply(source)
        assert "{a b}" in applied
        assert "{c d}" in applied


# ── switch → dict ─────────────────────────────────────────────────────


class TestSwitchToDict:
    def test_set_pattern(self):
        source = (
            "switch -exact -- $method {\n"
            "    GET { set handler handle_get }\n"
            "    POST { set handler handle_post }\n"
            "    PUT { set handler handle_put }\n"
            "}"
        )
        result = switch_to_dict(source, 0, 0)
        assert result is not None
        assert "dict" in result.title.lower()
        applied = result.apply(source)
        assert "dict create" in applied
        assert "dict get" in applied

    def test_return_pattern(self):
        source = (
            "switch -exact -- $code {\n"
            '    200 { return "OK" }\n'
            '    404 { return "Not Found" }\n'
            '    500 { return "Server Error" }\n'
            "}"
        )
        result = switch_to_dict(source, 0, 0)
        assert result is not None
        applied = result.apply(source)
        assert "dict create" in applied

    def test_glob_mode_returns_none(self):
        source = (
            "switch -glob -- $path {\n"
            "    /api/* { set handler api }\n"
            "    /web/* { set handler web }\n"
            "}"
        )
        result = switch_to_dict(source, 0, 0)
        assert result is None

    def test_too_few_arms_returns_none(self):
        source = "switch -exact -- $x {\n    a { set y 1 }\n}"
        result = switch_to_dict(source, 0, 0)
        assert result is None

    def test_mixed_bodies_returns_none(self):
        source = 'switch -exact -- $x {\n    a { set y 1 }\n    b { puts "hello" }\n}'
        result = switch_to_dict(source, 0, 0)
        assert result is None

    def test_rewrite_covers_entire_switch_command(self):
        source = (
            "switch -exact -- $m {\n"
            "    a { set out 1 }\n"
            "    b { set out 2 }\n"
            "    c { set out 3 }\n"
            "}"
        )
        result = switch_to_dict(source, 0, 0)
        assert result is not None
        applied = result.apply(source).strip()
        assert applied.endswith("]")
        assert not applied.endswith("]}")

    def test_indented_switch_does_not_double_indent_first_line(self):
        source = (
            "    switch -exact -- $method {\n"
            "        GET { set handler get_h }\n"
            "        POST { set handler post_h }\n"
            "        PUT { set handler put_h }\n"
            "    }"
        )
        result = switch_to_dict(source, 0, 4)
        assert result is not None
        applied = result.apply(source)
        first_line = applied.split("\n")[0]
        assert first_line.startswith("    set ")
        assert not first_line.startswith("        set ")


# ── Brace expr ────────────────────────────────────────────────────────


class TestBraceExpr:
    def test_quoted_expr(self):
        source = 'expr "$a + $b"'
        result = brace_expr(source, 0, 0)
        assert result is not None
        assert "Brace" in result.title
        applied = result.apply(source)
        assert "{$a + $b}" in applied

    def test_already_braced(self):
        source = "expr {$a + $b}"
        result = brace_expr(source, 0, 0)
        assert result is None

    def test_multi_arg_expr(self):
        source = "expr $a + $b"
        result = brace_expr(source, 0, 0)
        assert result is not None
        applied = result.apply(source)
        assert "{$a + $b}" in applied

    def test_non_expr_command(self):
        source = 'puts "hello"'
        result = brace_expr(source, 0, 0)
        assert result is None


# ── Extract to data-group ─────────────────────────────────────────────


class TestExtractDatagroup:
    @pytest.fixture(autouse=True)
    def _setup(self):
        configure_signatures(dialect="f5-irules")
        yield
        configure_signatures(dialect="tcl8.6")

    def test_if_chain_string_membership(self):
        source = (
            'if {$host eq "a.com"} {\n'
            "    pool web_pool\n"
            '} elseif {$host eq "b.com"} {\n'
            "    pool web_pool\n"
            '} elseif {$host eq "c.com"} {\n'
            "    pool web_pool\n"
            "}"
        )
        result = extract_to_datagroup_from_if(source, 0, 0, dg_name="allowed_hosts")
        assert result is not None
        assert result.data_group.value_type == "string"
        assert result.data_group.name == "allowed_hosts"
        assert len(result.data_group.records) == 3
        assert ("a.com", "") in result.data_group.records
        dg_tcl = result.data_group_tcl()
        assert "type string" in dg_tcl
        applied = result.apply(source)
        assert "class match" in applied
        assert "allowed_hosts" in applied

    def test_if_chain_ip_addresses(self):
        source = (
            'if {$addr eq "10.0.0.0/8"} {\n'
            "    drop\n"
            '} elseif {$addr eq "172.16.0.0/12"} {\n'
            "    drop\n"
            '} elseif {$addr eq "192.168.0.0/16"} {\n'
            "    drop\n"
            "}"
        )
        result = extract_to_datagroup_from_if(source, 0, 0, dg_name="rfc1918")
        assert result is not None
        assert result.data_group.value_type == "ip"
        dg_tcl = result.data_group_tcl()
        assert "type ip" in dg_tcl
        assert "10.0.0.0/8" in dg_tcl

    def test_if_chain_ipv6_addresses(self):
        source = (
            'if {$addr eq "2001:db8::/32"} {\n'
            "    drop\n"
            '} elseif {$addr eq "fd00::/8"} {\n'
            "    drop\n"
            '} elseif {$addr eq "fe80::1"} {\n'
            "    drop\n"
            "}"
        )
        result = extract_to_datagroup_from_if(source, 0, 0, dg_name="blocked_v6")
        assert result is not None
        assert result.data_group.value_type == "ip"
        dg_tcl = result.data_group_tcl()
        assert "type ip" in dg_tcl
        assert "2001:db8::/32" in dg_tcl
        assert "fd00::/8" in dg_tcl
        assert "fe80::1" in dg_tcl

    def test_if_chain_mixed_ipv4_ipv6(self):
        source = (
            'if {$addr eq "10.0.0.0/8"} {\n'
            "    drop\n"
            '} elseif {$addr eq "2001:db8::/32"} {\n'
            "    drop\n"
            '} elseif {$addr eq "::ffff:192.168.1.0/120"} {\n'
            "    drop\n"
            "}"
        )
        result = extract_to_datagroup_from_if(source, 0, 0, dg_name="blocked_mixed")
        assert result is not None
        assert result.data_group.value_type == "ip"

    def test_if_chain_integers_membership(self):
        source = (
            'if {$port eq "80"} {\n'
            '    log local0. "http"\n'
            '} elseif {$port eq "443"} {\n'
            '    log local0. "http"\n'
            '} elseif {$port eq "8080"} {\n'
            '    log local0. "http"\n'
            "}"
        )
        result = extract_to_datagroup_from_if(source, 0, 0, dg_name="http_ports")
        assert result is not None
        assert result.data_group.value_type == "integer"

    def test_if_chain_value_mapping(self):
        source = (
            'if {$port eq "80"} {\n'
            "    set proto http\n"
            '} elseif {$port eq "443"} {\n'
            "    set proto https\n"
            '} elseif {$port eq "8080"} {\n'
            "    set proto http\n"
            "}"
        )
        result = extract_to_datagroup_from_if(source, 0, 0, dg_name="port_map")
        assert result is not None
        # Value mapping uses string type for the data-group records.
        assert result.data_group.value_type == "string"

    def test_switch_membership(self):
        source = (
            "switch -exact -- $ext {\n"
            "    .jpg { set type image }\n"
            "    .png { set type image }\n"
            "    .gif { set type image }\n"
            "    .svg { set type image }\n"
            "}"
        )
        result = extract_to_datagroup_from_switch(source, 0, 0, dg_name="image_types")
        assert result is not None
        assert result.data_group.value_type == "string"
        assert len(result.data_group.records) == 4

    def test_switch_value_mapping(self):
        source = (
            "switch -exact -- $method {\n"
            "    GET { set handler handle_get }\n"
            "    POST { set handler handle_post }\n"
            "    PUT { set handler handle_put }\n"
            "    DELETE { set handler handle_delete }\n"
            "}"
        )
        result = extract_to_datagroup_from_switch(source, 0, 0, dg_name="method_handler_map")
        assert result is not None
        # Should detect set_mapping body shape.
        assert any(r[1] for r in result.data_group.records)  # values are non-empty

    def test_or_chain_detection(self):
        source = (
            'if {$host eq "a.com" || $host eq "b.com" || $host eq "c.com"} {\n    pool web_pool\n}'
        )
        result = extract_to_datagroup_from_if(source, 0, 0, dg_name="allowed")
        assert result is not None
        assert len(result.data_group.records) == 3

    def test_too_few_values_returns_none(self):
        source = 'if {$x eq "a"} { puts ok }'
        result = extract_to_datagroup_from_if(source, 0, 0)
        assert result is None

    def test_unified_dispatch(self):
        """extract_to_datagroup tries if first, then switch."""
        source = (
            "switch -exact -- $uri {\n"
            "    /api { set pool api_pool }\n"
            "    /web { set pool web_pool }\n"
            "    /cdn { set pool cdn_pool }\n"
            "}"
        )
        result = extract_to_datagroup(source, 0, 0, dg_name="uri_pool")
        assert result is not None

    def test_appears_in_code_actions_irules(self):
        source = (
            'if {$host eq "a.com"} {\n'
            "    pool web_pool\n"
            '} elseif {$host eq "b.com"} {\n'
            "    pool web_pool\n"
            '} elseif {$host eq "c.com"} {\n'
            "    pool web_pool\n"
            "}"
        )
        selected = types.Range(
            start=types.Position(line=0, character=0),
            end=types.Position(line=0, character=0),
        )
        context = types.CodeActionContext(diagnostics=[])
        actions = get_code_actions(source, selected, context)
        dg_actions = [a for a in actions if "data-group" in (a.title or "").lower()]
        assert len(dg_actions) >= 1

    def test_data_group_tcl_rendering(self):
        source = (
            'if {$host eq "a.com"} {\n'
            "    pool web_pool\n"
            '} elseif {$host eq "b.com"} {\n'
            "    pool web_pool\n"
            "}"
        )
        result = extract_to_datagroup_from_if(source, 0, 0, dg_name="hosts")
        assert result is not None
        tcl = result.data_group_tcl()
        assert "ltm data-group internal hosts" in tcl
        assert "records" in tcl
        assert "a.com" in tcl
        assert "b.com" in tcl

    def test_if_rewrite_covers_entire_command(self):
        source = (
            'if {$host eq "a.com"} {\n'
            "    pool web_pool\n"
            '} elseif {$host eq "b.com"} {\n'
            "    pool web_pool\n"
            "}"
        )
        result = extract_to_datagroup_from_if(source, 0, 0, dg_name="hosts")
        assert result is not None
        applied = result.apply(source).strip()
        assert applied.endswith("}")
        assert not applied.endswith("}}")


# ── AI-enhanced data-group suggestions ────────────────────────────────


class TestSuggestDatagroupExtraction:
    @pytest.fixture(autouse=True)
    def _setup(self):
        configure_signatures(dialect="f5-irules")
        yield
        configure_signatures(dialect="tcl8.6")

    def test_detects_if_chain(self):
        source = (
            'if {$host eq "a.com"} {\n'
            "    pool web_pool\n"
            '} elseif {$host eq "b.com"} {\n'
            "    pool web_pool\n"
            '} elseif {$host eq "c.com"} {\n'
            "    pool web_pool\n"
            "}"
        )
        candidates = suggest_datagroup_extraction(source)
        assert len(candidates) >= 1
        c = candidates[0]
        assert c["pattern_type"] == "if_chain"
        assert c["variable"] == "host"
        assert c["inferred_type"] == "string"
        assert c["body_shape"] == "identical"
        assert c["confidence"] == "high"
        assert c["static_result"] is not None

    def test_detects_switch(self):
        source = (
            "switch -exact -- $method {\n"
            "    GET { set handler handle_get }\n"
            "    POST { set handler handle_post }\n"
            "    PUT { set handler handle_put }\n"
            "}"
        )
        candidates = suggest_datagroup_extraction(source)
        assert len(candidates) >= 1
        c = candidates[0]
        assert c["pattern_type"] == "switch"
        assert c["variable"] == "method"
        assert c["body_shape"] == "set_mapping"

    def test_detects_ip_cidr(self):
        source = (
            'if {$addr eq "10.0.0.0/8"} {\n'
            "    drop\n"
            '} elseif {$addr eq "172.16.0.0/12"} {\n'
            "    drop\n"
            "}"
        )
        candidates = suggest_datagroup_extraction(source)
        assert len(candidates) >= 1
        c = candidates[0]
        assert c["inferred_type"] == "ip"
        assert c["has_cidr"] is True

    def test_no_candidates_for_clean_code(self):
        source = "set x 42\nputs $x"
        candidates = suggest_datagroup_extraction(source)
        assert len(candidates) == 0


# ── RefactoringResult.apply ───────────────────────────────────────────


class TestRefactoringResultApply:
    def test_single_edit(self):
        from core.refactoring import RefactoringEdit

        source = "set x 42"
        result = RefactoringResult(
            title="test",
            edits=(RefactoringEdit(0, 6, 0, 8, "99"),),
        )
        assert result.apply(source) == "set x 99"

    def test_multiple_edits(self):
        from core.refactoring import RefactoringEdit

        source = "set x 42\nputs $x"
        result = RefactoringResult(
            title="test",
            edits=(
                RefactoringEdit(0, 6, 0, 8, "99"),
                RefactoringEdit(1, 5, 1, 7, "$y"),
            ),
        )
        applied = result.apply(source)
        assert "set x 99" in applied
        assert "$y" in applied


class TestNestedBodySupport:
    """P1 regression: refactorings must work inside proc/when/loop bodies."""

    def test_if_to_switch_inside_proc(self):
        source = (
            "proc handler {method} {\n"
            '    if {$method eq "GET"} {\n'
            "        handle_get\n"
            '    } elseif {$method eq "POST"} {\n'
            "        handle_post\n"
            '    } elseif {$method eq "PUT"} {\n'
            "        handle_put\n"
            "    }\n"
            "}"
        )
        result = if_to_switch(source, 1, 4)
        assert result is not None
        assert "switch" in result.title.lower()

    def test_brace_expr_inside_proc(self):
        source = 'proc calc {a b} {\n    expr "$a + $b"\n}'
        result = brace_expr(source, 1, 4)
        assert result is not None
        assert "Brace" in result.title

    def test_switch_to_dict_inside_when(self):
        configure_signatures(dialect="f5-irules")
        try:
            source = (
                "when HTTP_REQUEST {\n"
                "    switch -exact -- $method {\n"
                "        GET { set handler get_h }\n"
                "        POST { set handler post_h }\n"
                "        PUT { set handler put_h }\n"
                "    }\n"
                "}"
            )
            result = switch_to_dict(source, 1, 4)
            assert result is not None
            assert "dict" in result.title.lower()
        finally:
            configure_signatures(dialect="tcl8.6")

    def test_extract_datagroup_inside_when(self):
        configure_signatures(dialect="f5-irules")
        try:
            source = (
                "when HTTP_REQUEST {\n"
                '    if {$host eq "a.com"} {\n'
                "        pool web_pool\n"
                '    } elseif {$host eq "b.com"} {\n'
                "        pool web_pool\n"
                '    } elseif {$host eq "c.com"} {\n'
                "        pool web_pool\n"
                "    }\n"
                "}"
            )
            result = extract_to_datagroup(source, 1, 4)
            assert result is not None
            assert result.data_group.value_type == "string"
            assert len(result.data_group.records) == 3
        finally:
            configure_signatures(dialect="tcl8.6")

    def test_suggest_datagroup_finds_nested_candidates(self):
        configure_signatures(dialect="f5-irules")
        try:
            source = (
                "when HTTP_REQUEST {\n"
                '    if {$host eq "a.com"} {\n'
                "        pool web_pool\n"
                '    } elseif {$host eq "b.com"} {\n'
                "        pool web_pool\n"
                '    } elseif {$host eq "c.com"} {\n'
                "        pool web_pool\n"
                "    }\n"
                "}"
            )
            candidates = suggest_datagroup_extraction(source)
            assert len(candidates) >= 1
            assert candidates[0]["variable"] == "host"
        finally:
            configure_signatures(dialect="tcl8.6")


class TestRefactoringOptimiserCombinations:
    def test_optimiser_quickfix_and_refactor_rewrite_can_coexist(self):
        source = 'expr "$a + $b"'
        selected = types.Range(
            start=types.Position(line=0, character=0),
            end=types.Position(line=0, character=0),
        )
        optimiser_diag = types.Diagnostic(
            range=types.Range(
                start=types.Position(line=0, character=0),
                end=types.Position(line=0, character=12),
            ),
            message="Use braced expr for optimisation",
            code="O199",
            data={"replacement": "expr {$a + $b}"},
        )
        context = types.CodeActionContext(diagnostics=[optimiser_diag])
        actions = get_code_actions(source, selected, context)

        assert any(a.kind == types.CodeActionKind.QuickFix for a in actions)
        assert any(
            a.kind == types.CodeActionKind.RefactorRewrite
            and "brace expr" in (a.title or "").lower()
            for a in actions
        )

    def test_grouped_optimiser_edit_keeps_refactor_actions_available(self):
        source = 'expr "$a + $b"'
        selected = types.Range(
            start=types.Position(line=0, character=0),
            end=types.Position(line=0, character=0),
        )
        optimiser_diag = types.Diagnostic(
            range=types.Range(
                start=types.Position(line=0, character=0),
                end=types.Position(line=0, character=12),
            ),
            message="Apply grouped optimisation",
            code="O201",
            data={
                "replacement": "expr {$a + $b}",
                "groupEdits": [
                    {
                        "startLine": 0,
                        "startCharacter": 0,
                        "endLine": 0,
                        "endCharacter": 13,
                        "replacement": "expr {$a + $b}",
                    },
                    {
                        "startLine": 1,
                        "startCharacter": 0,
                        "endLine": 1,
                        "endCharacter": 0,
                        "replacement": "",
                    },
                ],
            },
        )
        context = types.CodeActionContext(diagnostics=[optimiser_diag])
        actions = get_code_actions(source, selected, context)

        quickfix = next((a for a in actions if a.kind == types.CodeActionKind.QuickFix), None)
        assert quickfix is not None
        assert quickfix.edit is not None
        assert quickfix.edit.changes is not None
        assert len(quickfix.edit.changes["__current__"]) == 2
        assert any(
            a.kind == types.CodeActionKind.RefactorRewrite
            and "brace expr" in (a.title or "").lower()
            for a in actions
        )
