"""Tests for the iRule → F5 XC translator."""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.commands.registry.runtime import configure_signatures
from core.xc.translator import translate_irule
from core.xc.xc_model import TranslateStatus


def _setup():
    configure_signatures(dialect="f5-irules")


# Basic pool translation


class TestPoolTranslation:
    def test_simple_pool(self):
        _setup()
        src = "when HTTP_REQUEST {\n    pool my_pool\n}"
        result = translate_irule(src)
        assert len(result.origin_pools) == 1
        assert result.origin_pools[0].name == "my_pool"
        assert len(result.routes) == 1
        assert result.routes[0].origin_pool is not None
        assert result.routes[0].origin_pool.name == "my_pool"

    def test_multiple_pools_deduplicated(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::path] eq "/a"} {
        pool shared_pool
    } else {
        pool shared_pool
    }
}"""
        result = translate_irule(src)
        # Pools should be deduplicated
        assert len(result.origin_pools) == 1
        assert result.origin_pools[0].name == "shared_pool"
        # But routes created for each occurrence
        assert len(result.routes) == 2


# Switch on path → routes


class TestSwitchPathRoutes:
    def test_switch_path_glob(self):
        _setup()
        src = """when HTTP_REQUEST {
    switch -glob [HTTP::path] {
        "/api/*" { pool api_pool }
        "/static/*" { pool static_pool }
        default { pool default_pool }
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 3
        assert len(result.origin_pools) == 3

        # Check path matches
        api_route = result.routes[0]
        assert api_route.path_match is not None
        assert api_route.path_match.match_type == "prefix"
        assert api_route.path_match.value == "/api/"

        static_route = result.routes[1]
        assert static_route.path_match is not None
        assert static_route.path_match.match_type == "prefix"
        assert static_route.path_match.value == "/static/"

    def test_switch_host(self):
        _setup()
        src = """when HTTP_REQUEST {
    switch [HTTP::host] {
        "api.example.com" { pool api_pool }
        "www.example.com" { pool web_pool }
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 2

        api_route = result.routes[0]
        assert api_route.host_match is not None
        assert api_route.host_match.value == "api.example.com"


# HTTP::redirect


class TestRedirectTranslation:
    def test_simple_redirect(self):
        _setup()
        src = 'when HTTP_REQUEST {\n    HTTP::redirect "https://example.com/"\n}'
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.redirect is not None
        assert route.redirect.url == "https://example.com/"
        assert route.redirect.response_code == 302

    def test_redirect_in_switch(self):
        _setup()
        src = """when HTTP_REQUEST {
    switch -glob [HTTP::path] {
        "/old/*" { HTTP::redirect "https://example.com/new" }
        "/api/*" { pool api_pool }
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 2
        redirect_route = result.routes[0]
        assert redirect_route.redirect is not None
        assert redirect_route.path_match is not None
        assert redirect_route.path_match.value == "/old/"


# HTTP::respond → direct response or deny policy


class TestRespondTranslation:
    def test_respond_200(self):
        _setup()
        src = 'when HTTP_REQUEST {\n    HTTP::respond 200 content "OK"\n}'
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.direct_response is not None
        assert route.direct_response.status_code == 200
        assert route.direct_response.body == "OK"

    def test_respond_403_becomes_deny(self):
        _setup()
        src = "when HTTP_REQUEST {\n    HTTP::respond 403\n}"
        result = translate_irule(src)
        assert len(result.service_policies) == 1
        policy = result.service_policies[0]
        assert len(policy.rules) == 1
        assert policy.rules[0].action == "deny"


# Header manipulation


class TestHeaderTranslation:
    def test_header_insert_in_request(self):
        _setup()
        src = 'when HTTP_REQUEST {\n    HTTP::header insert "X-Custom" "value"\n}'
        result = translate_irule(src)
        assert len(result.header_actions) == 1
        action = result.header_actions[0]
        assert action.name == "X-Custom"
        assert action.value == "value"
        assert action.operation == "add"
        assert action.target == "request"

    def test_header_remove_in_response(self):
        _setup()
        src = 'when HTTP_RESPONSE {\n    HTTP::header remove "Server"\n}'
        result = translate_irule(src)
        assert len(result.header_actions) == 1
        action = result.header_actions[0]
        assert action.name == "Server"
        assert action.operation == "remove"
        assert action.target == "response"

    def test_header_replace(self):
        _setup()
        src = 'when HTTP_RESPONSE {\n    HTTP::header replace "X-Frame-Options" "DENY"\n}'
        result = translate_irule(src)
        assert len(result.header_actions) == 1
        action = result.header_actions[0]
        assert action.operation == "replace"


# Untranslatable events


class TestUntranslatableEvents:
    def test_rule_init(self):
        _setup()
        src = "when RULE_INIT {\n    set static::debug 0\n}"
        result = translate_irule(src)
        untranslatable = [i for i in result.items if i.status == TranslateStatus.UNTRANSLATABLE]
        assert len(untranslatable) >= 1
        assert any("RULE_INIT" in i.irule_command for i in untranslatable)

    def test_client_accepted(self):
        _setup()
        src = "when CLIENT_ACCEPTED {\n    set debug 0\n}"
        result = translate_irule(src)
        untranslatable = [i for i in result.items if i.status == TranslateStatus.UNTRANSLATABLE]
        assert any("CLIENT_ACCEPTED" in i.irule_command for i in untranslatable)

    def test_lb_failed(self):
        _setup()
        src = 'when LB_FAILED {\n    HTTP::respond 503 content "Down"\n}'
        result = translate_irule(src)
        untranslatable = [i for i in result.items if i.status == TranslateStatus.UNTRANSLATABLE]
        assert any("LB_FAILED" in i.irule_command for i in untranslatable)


# Advisory events


class TestAdvisoryEvents:
    def test_ssl_event(self):
        _setup()
        src = "when CLIENTSSL_HANDSHAKE {\n    SSL::cert 0\n}"
        result = translate_irule(src)
        advisory = [i for i in result.items if i.status == TranslateStatus.ADVISORY]
        assert len(advisory) >= 1
        assert any("CLIENTSSL" in i.irule_command for i in advisory)


# Untranslatable commands


class TestUntranslatableCommands:
    def test_eval_flagged(self):
        _setup()
        src = "when HTTP_REQUEST {\n    eval $dynamic_code\n}"
        result = translate_irule(src)
        untranslatable = [i for i in result.items if i.status == TranslateStatus.UNTRANSLATABLE]
        assert any("eval" in i.irule_command.lower() for i in untranslatable)

    def test_tcp_command_flagged(self):
        _setup()
        src = "when HTTP_REQUEST {\n    TCP::client_port\n}"
        result = translate_irule(src)
        untranslatable = [i for i in result.items if i.status == TranslateStatus.UNTRANSLATABLE]
        assert any("TCP::" in i.irule_command for i in untranslatable)


# Coverage calculation


class TestCoverage:
    def test_fully_translatable(self):
        _setup()
        src = """when HTTP_REQUEST {
    switch -glob [HTTP::path] {
        "/api/*" { pool api_pool }
        default { pool default_pool }
    }
}"""
        result = translate_irule(src)
        assert result.coverage_pct > 80.0

    def test_mixed_coverage(self):
        _setup()
        src = """when HTTP_REQUEST {
    pool web_pool
}
when CLIENT_ACCEPTED {
    set debug 0
}"""
        result = translate_irule(src)
        assert 0 < result.coverage_pct < 100

    def test_empty_source(self):
        _setup()
        src = ""
        result = translate_irule(src)
        assert result.coverage_pct == 100.0
        assert len(result.items) == 0


# Complex scenarios


class TestComplexScenarios:
    def test_full_irule(self):
        """Test the example iRule from the plan."""
        _setup()
        src = """when HTTP_REQUEST {
    switch -glob [HTTP::uri] {
        "/api/*" { pool api_pool }
        "/static/*" { pool static_pool }
        default { HTTP::redirect "https://example.com/" }
    }
    HTTP::header insert "X-Forwarded-Proto" "https"
}
when HTTP_RESPONSE {
    HTTP::header insert "Strict-Transport-Security" "max-age=31536000"
    HTTP::header remove "Server"
}"""
        result = translate_irule(src)
        assert len(result.origin_pools) == 2
        assert len(result.routes) == 3  # 2 pools + 1 redirect
        assert len(result.header_actions) == 3
        assert result.coverage_pct > 80.0


# Terraform rendering


class TestTerraformRendering:
    def test_renders_provider_block(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = "when HTTP_REQUEST {\n    pool my_pool\n}"
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert "required_providers" in hcl
        assert "volterraedge/volterra" in hcl

    def test_renders_origin_pool(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = "when HTTP_REQUEST {\n    pool my_pool\n}"
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert 'resource "volterra_origin_pool" "my_pool"' in hcl

    def test_renders_load_balancer(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = "when HTTP_REQUEST {\n    pool my_pool\n}"
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert 'resource "volterra_http_loadbalancer"' in hcl

    def test_renders_service_policy(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = "when HTTP_REQUEST {\n    HTTP::respond 403\n}"
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert 'resource "volterra_service_policy"' in hcl
        assert "DENY" in hcl

    def test_renders_redirect_route(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = 'when HTTP_REQUEST {\n    HTTP::redirect "https://example.com/"\n}'
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert "redirect_route" in hcl
        assert "example.com" in hcl

    def test_renders_coverage_summary(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = "when HTTP_REQUEST {\n    pool my_pool\n}"
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert "Coverage:" in hcl


# JSON API rendering


class TestJsonRendering:
    def test_renders_origin_pools(self):
        _setup()
        from core.xc.json_api import render_json

        src = "when HTTP_REQUEST {\n    pool my_pool\n}"
        result = translate_irule(src)
        data = render_json(result)
        assert "origin_pools" in data
        assert data["origin_pools"][0]["metadata"]["name"] == "my_pool"

    def test_renders_load_balancer(self):
        _setup()
        from core.xc.json_api import render_json

        src = "when HTTP_REQUEST {\n    pool my_pool\n}"
        result = translate_irule(src)
        data = render_json(result)
        assert "http_loadbalancer" in data
        assert "routes" in data["http_loadbalancer"]["spec"]

    def test_renders_service_policy(self):
        _setup()
        from core.xc.json_api import render_json

        src = "when HTTP_REQUEST {\n    HTTP::respond 403\n}"
        result = translate_irule(src)
        data = render_json(result)
        assert "service_policies" in data

    def test_renders_summary(self):
        _setup()
        from core.xc.json_api import render_json

        src = "when HTTP_REQUEST {\n    pool my_pool\n}"
        result = translate_irule(src)
        data = render_json(result)
        assert "summary" in data
        assert "coverage_pct" in data["summary"]

    def test_json_serialisable(self):
        """Ensure the output is JSON-serialisable."""
        _setup()
        from core.xc.json_api import render_json

        src = """when HTTP_REQUEST {
    switch -glob [HTTP::path] {
        "/api/*" { pool api_pool }
        default { HTTP::redirect "https://example.com/" }
    }
    HTTP::header insert "X-Custom" "val"
}"""
        result = translate_irule(src)
        data = render_json(result)
        # Should not raise
        serialised = json.dumps(data, indent=2)
        parsed = json.loads(serialised)
        assert parsed["summary"]["coverage_pct"] > 0


# XC diagnostics


class TestXcDiagnostics:
    def test_diagnostics_for_translatable(self):
        _setup()
        from core.xc.diagnostics import get_xc_diagnostics

        src = "when HTTP_REQUEST {\n    pool my_pool\n}"
        diags = get_xc_diagnostics(src)
        assert len(diags) > 0
        assert any(d.code.startswith("XC1") for d in diags)

    def test_diagnostics_for_untranslatable(self):
        _setup()
        from core.xc.diagnostics import get_xc_diagnostics

        src = "when CLIENT_ACCEPTED {\n    set debug 0\n}"
        diags = get_xc_diagnostics(src)
        assert any(d.code.startswith("XC2") for d in diags)

    def test_diagnostics_for_barrier(self):
        _setup()
        from core.xc.diagnostics import get_xc_diagnostics

        src = "when HTTP_REQUEST {\n    eval $code\n}"
        diags = get_xc_diagnostics(src)
        assert any(d.code.startswith("XC3") for d in diags)


# ASM::disable → WAF exclusion rule


class TestAsmDisableTranslation:
    def test_asm_disable_produces_waf_exclusion(self):
        _setup()
        src = "when HTTP_REQUEST {\n    ASM::disable\n}"
        result = translate_irule(src)
        assert len(result.waf_exclusion_rules) == 1
        rule = result.waf_exclusion_rules[0]
        assert rule.path_match is None
        assert rule.ip_prefix_set is None
        translated = [i for i in result.items if i.status == TranslateStatus.TRANSLATED]
        assert any("ASM::disable" in i.irule_command for i in translated)

    def test_asm_disable_with_path_and_ip_condition(self):
        """The user's real-world iRule: ASM::disable guarded by URI + IP match."""
        _setup()
        src = """when HTTP_REQUEST {
    if {[string tolower [HTTP::uri]] starts_with "/ce0587te" && [class match [IP::client_addr] equals AppCheck_TrentScanning_TrustedAddresses]} {
        ASM::disable
    }
}"""
        result = translate_irule(src)
        assert len(result.waf_exclusion_rules) == 1
        rule = result.waf_exclusion_rules[0]
        assert rule.path_match is not None
        assert rule.path_match.match_type == "prefix"
        assert rule.path_match.value == "/ce0587te"
        assert rule.ip_prefix_set == "AppCheck_TrentScanning_TrustedAddresses"
        assert result.coverage_pct == 100.0

    def test_asm_disable_with_path_only(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::uri] starts_with "/scanner"} {
        ASM::disable
    }
}"""
        result = translate_irule(src)
        assert len(result.waf_exclusion_rules) == 1
        rule = result.waf_exclusion_rules[0]
        assert rule.path_match is not None
        assert rule.path_match.value == "/scanner"
        assert rule.ip_prefix_set is None

    def test_asm_enable_is_noop(self):
        _setup()
        src = "when HTTP_REQUEST {\n    ASM::enable\n}"
        result = translate_irule(src)
        assert len(result.waf_exclusion_rules) == 0
        translated = [i for i in result.items if i.status == TranslateStatus.TRANSLATED]
        assert any("ASM::enable" in i.irule_command for i in translated)


# Compound condition decomposition


class TestCompoundConditions:
    def test_and_condition_extracts_path_and_ip(self):
        """Compound && decomposes into path match + IP prefix set."""
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::path] starts_with "/api" && [class match [IP::client_addr] equals TrustedIPs]} {
        pool trusted_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.path_match is not None
        assert route.path_match.match_type == "prefix"
        assert route.path_match.value == "/api"

    def test_string_tolower_recognised_as_http_getter(self):
        """[string tolower [HTTP::uri]] is treated as HTTP::uri."""
        _setup()
        src = """when HTTP_REQUEST {
    if {[string tolower [HTTP::uri]] starts_with "/test"} {
        pool test_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.path_match is not None
        assert route.path_match.match_type == "prefix"
        assert route.path_match.value == "/test"


# WAF exclusion rule rendering


class TestWafExclusionRendering:
    def test_json_renders_waf_exclusion(self):
        _setup()
        from core.xc.json_api import render_json

        src = """when HTTP_REQUEST {
    if {[HTTP::uri] starts_with "/scan" && [class match [IP::client_addr] equals ScannerIPs]} {
        ASM::disable
    }
}"""
        result = translate_irule(src)
        data = render_json(result)
        lb = data["http_loadbalancer"]
        assert "waf_exclusion_rules" in lb["spec"]
        waf_rules = lb["spec"]["waf_exclusion_rules"]
        assert len(waf_rules) == 1
        rule = waf_rules[0]
        assert rule["match_condition"]["path"]["prefix"] == "/scan"
        assert rule["match_condition"]["client_source"]["ip_prefix_set"]["ref"] == ["ScannerIPs"]
        assert rule["app_firewall_detection_control"]["exclude_all_attack_types"] is True

    def test_terraform_renders_waf_exclusion(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = """when HTTP_REQUEST {
    if {[HTTP::uri] starts_with "/scan"} {
        ASM::disable
    }
}"""
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert "waf_exclusion_rules" in hcl
        assert "exclude_all_attack_types" in hcl
        assert "/scan" in hcl


# Extended path matching (ends_with, contains, glob, regex)


class TestExtendedPathMatching:
    def test_ends_with_produces_suffix_match(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::uri] ends_with ".jpg"} {
        pool image_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.path_match is not None
        assert route.path_match.match_type == "suffix"
        assert route.path_match.value == ".jpg"

    def test_contains_produces_regex_match(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::path] contains "admin"} {
        HTTP::respond 403
    }
}"""
        result = translate_irule(src)
        assert len(result.service_policies) == 1
        rule = result.service_policies[0].rules[0]
        assert rule.path_match is not None
        assert rule.path_match.match_type == "regex"
        assert "admin" in rule.path_match.value

    def test_matches_regex_produces_regex_match(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::uri] matches_regex {^/api/v[0-9]+/}} {
        pool api_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.path_match is not None
        assert route.path_match.match_type == "regex"
        assert "api" in route.path_match.value

    def test_matches_glob_produces_regex_match(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::path] matches_glob "/img/*.png"} {
        pool image_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.path_match is not None
        assert route.path_match.match_type == "regex"
        # Glob /img/*.png → regex with .* for *
        assert ".*" in route.path_match.value


# Negation support


class TestNegation:
    def test_negated_path_match(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {!([HTTP::path] starts_with "/health")} {
        pool app_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.path_match is not None
        assert route.path_match.match_type == "prefix"
        assert route.path_match.value == "/health"
        assert route.path_match.invert is True

    def test_ne_method_produces_inverted_match(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::method] ne "GET"} {
        HTTP::respond 403
    }
}"""
        result = translate_irule(src)
        assert len(result.service_policies) == 1
        rule = result.service_policies[0].rules[0]
        assert rule.method_match is not None
        assert rule.method_match.methods == ("GET",)
        assert rule.method_match.invert is True


# Header matching


class TestHeaderMatching:
    def test_header_value_equality(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::header value "X-Forwarded-Proto"] eq "http"} {
        HTTP::redirect "https://[HTTP::host][HTTP::uri]"
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert len(route.header_matches) == 1
        hdr = route.header_matches[0]
        assert hdr.name == "X-Forwarded-Proto"
        assert hdr.match_type == "exact"
        assert hdr.value == "http"

    def test_header_existence(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::header exists "X-Debug"]} {
        pool debug_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert len(route.header_matches) == 1
        hdr = route.header_matches[0]
        assert hdr.name == "X-Debug"
        assert hdr.match_type == "presence"

    def test_header_ne_produces_inverted_match(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::header value "X-Internal"] ne "true"} {
        HTTP::respond 403
    }
}"""
        result = translate_irule(src)
        assert len(result.service_policies) == 1
        rule = result.service_policies[0].rules[0]
        assert len(rule.header_matches) == 1
        hdr = rule.header_matches[0]
        assert hdr.name == "X-Internal"
        assert hdr.invert is True


# Cookie matching


class TestCookieMatching:
    def test_cookie_value_equality(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::cookie "session_type"] eq "premium"} {
        pool premium_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert len(route.cookie_matches) == 1
        cookie = route.cookie_matches[0]
        assert cookie.name == "session_type"
        assert cookie.match_type == "exact"
        assert cookie.value == "premium"

    def test_cookie_existence(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::cookie exists "auth_token"]} {
        pool auth_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert len(route.cookie_matches) == 1
        cookie = route.cookie_matches[0]
        assert cookie.name == "auth_token"
        assert cookie.match_type == "presence"


# Query string matching


class TestQueryMatching:
    def test_query_contains(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::query] contains "debug=true"} {
        pool debug_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.query_match is not None
        assert route.query_match.match_type == "regex"
        assert "debug" in route.query_match.value

    def test_query_exact(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::query] eq "format=json"} {
        pool json_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.query_match is not None
        assert route.query_match.match_type == "exact"
        assert route.query_match.value == "format=json"


# Direct IP matching


class TestDirectIpMatching:
    def test_ip_eq_produces_ip_prefix_list(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[IP::client_addr] eq "10.0.0.1"} {
        HTTP::respond 403
    }
}"""
        result = translate_irule(src)
        assert len(result.service_policies) == 1
        rule = result.service_policies[0].rules[0]
        assert "10.0.0.1" in rule.ip_prefix_list


# OR condition decomposition


class TestOrDecomposition:
    def test_or_creates_multiple_routes(self):
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::path] starts_with "/api" || [HTTP::path] starts_with "/v2"} {
        pool api_pool
    }
}"""
        result = translate_irule(src)
        # OR decomposition should create two routes (one per branch)
        assert len(result.routes) == 2
        paths = {r.path_match.value for r in result.routes if r.path_match}
        assert "/api" in paths
        assert "/v2" in paths
        # Both should point to the same pool
        assert all(r.origin_pool and r.origin_pool.name == "api_pool" for r in result.routes)


# Combined condition matching


class TestCombinedConditions:
    def test_path_and_header_and_method(self):
        """Multiple AND conditions produce route with all match types."""
        _setup()
        src = """when HTTP_REQUEST {
    if {[HTTP::path] starts_with "/api" && [HTTP::method] eq "POST" && [HTTP::header value "Content-Type"] eq "application/json"} {
        pool api_pool
    }
}"""
        result = translate_irule(src)
        assert len(result.routes) == 1
        route = result.routes[0]
        assert route.path_match is not None
        assert route.path_match.match_type == "prefix"
        assert route.method_match is not None
        assert route.method_match.methods == ("POST",)
        assert len(route.header_matches) == 1
        assert route.header_matches[0].name == "Content-Type"


# Rendering of new match types


class TestNewMatchTypeRendering:
    def test_json_renders_header_match_on_route(self):
        _setup()
        from core.xc.json_api import render_json

        src = """when HTTP_REQUEST {
    if {[HTTP::header value "X-Forwarded-Proto"] eq "http"} {
        pool redirect_pool
    }
}"""
        result = translate_irule(src)
        data = render_json(result)
        route = data["http_loadbalancer"]["spec"]["routes"][0]
        assert "match" in route
        assert "headers" in route["match"]
        hdr = route["match"]["headers"][0]
        assert hdr["name"] == "X-Forwarded-Proto"
        assert hdr["exact"] == "http"

    def test_json_renders_cookie_match_on_route(self):
        _setup()
        from core.xc.json_api import render_json

        src = """when HTTP_REQUEST {
    if {[HTTP::cookie "tier"] eq "gold"} {
        pool gold_pool
    }
}"""
        result = translate_irule(src)
        data = render_json(result)
        route = data["http_loadbalancer"]["spec"]["routes"][0]
        assert "match" in route
        assert "cookies" in route["match"]
        cookie = route["match"]["cookies"][0]
        assert cookie["name"] == "tier"
        assert cookie["exact"] == "gold"

    def test_json_renders_query_match_on_route(self):
        _setup()
        from core.xc.json_api import render_json

        src = """when HTTP_REQUEST {
    if {[HTTP::query] eq "format=xml"} {
        pool xml_pool
    }
}"""
        result = translate_irule(src)
        data = render_json(result)
        route = data["http_loadbalancer"]["spec"]["routes"][0]
        assert "match" in route
        assert "query_params" in route["match"]

    def test_json_renders_inverted_path(self):
        _setup()
        from core.xc.json_api import render_json

        src = """when HTTP_REQUEST {
    if {!([HTTP::path] starts_with "/health")} {
        pool app_pool
    }
}"""
        result = translate_irule(src)
        data = render_json(result)
        route = data["http_loadbalancer"]["spec"]["routes"][0]
        path_match = route["match"]["path"]
        assert path_match["invert_matcher"] is True

    def test_json_renders_ip_prefix_list_on_policy(self):
        _setup()
        from core.xc.json_api import render_json

        src = """when HTTP_REQUEST {
    if {[IP::client_addr] eq "10.0.0.1"} {
        HTTP::respond 403
    }
}"""
        result = translate_irule(src)
        data = render_json(result)
        rule = data["service_policies"][0]["spec"]["rule_list"]["rules"][0]
        assert "match" in rule["spec"]
        assert "client_source" in rule["spec"]["match"]
        assert "10.0.0.1" in rule["spec"]["match"]["client_source"]["ip_prefix_list"]

    def test_terraform_renders_suffix_path(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = """when HTTP_REQUEST {
    if {[HTTP::uri] ends_with ".css"} {
        pool static_pool
    }
}"""
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert "suffix" in hcl
        assert '".css"' in hcl

    def test_terraform_renders_header_match(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = """when HTTP_REQUEST {
    if {[HTTP::header value "X-Test"] eq "yes"} {
        pool test_pool
    }
}"""
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert "headers {" in hcl
        assert '"X-Test"' in hcl

    def test_terraform_renders_cookie_match(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = """when HTTP_REQUEST {
    if {[HTTP::cookie "mode"] eq "debug"} {
        pool debug_pool
    }
}"""
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert "cookies {" in hcl
        assert '"mode"' in hcl

    def test_terraform_renders_inverted_path(self):
        _setup()
        from core.xc.terraform import render_terraform

        src = """when HTTP_REQUEST {
    if {!([HTTP::path] starts_with "/health")} {
        pool app_pool
    }
}"""
        result = translate_irule(src)
        hcl = render_terraform(result)
        assert "invert_matcher = true" in hcl
