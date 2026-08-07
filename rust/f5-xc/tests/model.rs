// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Model-level assertions for the `f5-xc` translator — cover a subset of
//! cases that check the *shape* of the produced XC constructs (which the
//! summary-only differential fixture does not capture).

use f5_xc::model::TranslateStatus;
use f5_xc::{render_json, render_terraform, translate_irule};

/// Find the route whose origin pool has the given name.
fn route_for_pool<'a>(
    result: &'a f5_xc::XCTranslationResult,
    pool: &str,
) -> Option<&'a f5_xc::model::XCRoute> {
    result
        .routes
        .iter()
        .find(|r| r.origin_pool.as_ref().map(|p| p.name.as_str()) == Some(pool))
}

#[test]
fn simple_pool_builds_route_and_pool() {
    let src = "when HTTP_REQUEST {\n    pool my_pool\n}";
    let result = translate_irule(src);
    assert_eq!(result.origin_pools.len(), 1);
    assert_eq!(result.origin_pools[0].name, "my_pool");
    assert_eq!(result.routes.len(), 1);
    let pool = result.routes[0].origin_pool.as_ref().expect("origin pool");
    assert_eq!(pool.name, "my_pool");
}

#[test]
fn pools_are_deduplicated_but_routes_are_not() {
    let src = "when HTTP_REQUEST {\n    if {[HTTP::path] eq \"/a\"} {\n        pool shared_pool\n    } else {\n        pool shared_pool\n    }\n}";
    let result = translate_irule(src);
    assert_eq!(result.origin_pools.len(), 1);
    assert_eq!(result.origin_pools[0].name, "shared_pool");
    assert_eq!(result.routes.len(), 2);
}

#[test]
fn switch_path_glob_yields_prefix_matches() {
    let src = "when HTTP_REQUEST {\n    switch -glob [HTTP::path] {\n        \"/api/*\" { pool api_pool }\n        \"/static/*\" { pool static_pool }\n        default { pool default_pool }\n    }\n}";
    let result = translate_irule(src);
    assert_eq!(result.routes.len(), 3);
    let api = result.routes[0].path_match.as_ref().expect("path match");
    assert_eq!(api.match_type, "prefix");
    assert_eq!(api.value, "/api/");
    let stat = result.routes[1].path_match.as_ref().expect("path match");
    assert_eq!(stat.match_type, "prefix");
    assert_eq!(stat.value, "/static/");
}

#[test]
fn starts_with_path_is_prefix_match() {
    let src = "when HTTP_REQUEST {\n    if {[HTTP::path] starts_with \"/api\"} {\n        pool api_pool\n    }\n}";
    let result = translate_irule(src);
    let route = result
        .routes
        .iter()
        .find(|r| r.path_match.is_some())
        .expect("a route with a path match");
    let pm = route.path_match.as_ref().unwrap();
    assert_eq!(pm.match_type, "prefix");
    assert_eq!(pm.value, "/api");
}

#[test]
fn header_insert_records_request_action() {
    let src = "when HTTP_REQUEST {\n    HTTP::header insert \"X-Custom\" \"value\"\n}";
    let result = translate_irule(src);
    assert_eq!(result.header_actions.len(), 1);
    let a = &result.header_actions[0];
    assert_eq!(a.name, "X-Custom");
    assert_eq!(a.value, "value");
    assert_eq!(a.operation, "add");
    assert_eq!(a.target, "request");
}

#[test]
fn header_in_response_event_targets_response() {
    let src = "when HTTP_RESPONSE {\n    HTTP::header replace \"Server\" \"Acme\"\n}";
    let result = translate_irule(src);
    assert_eq!(result.header_actions.len(), 1);
    assert_eq!(result.header_actions[0].operation, "replace");
    assert_eq!(result.header_actions[0].target, "response");
}

#[test]
fn redirect_builds_redirect_route() {
    let src = "when HTTP_REQUEST {\n    if {[HTTP::path] eq \"/old\"} {\n        HTTP::redirect \"https://new.example.com\"\n    }\n}";
    let result = translate_irule(src);
    let route = result
        .routes
        .iter()
        .find(|r| r.redirect.is_some())
        .expect("a redirect route");
    let r = route.redirect.as_ref().unwrap();
    assert_eq!(r.url, "https://new.example.com");
    assert_eq!(r.response_code, 302);
}

#[test]
fn respond_403_builds_deny_policy_rule() {
    let src = "when HTTP_REQUEST {\n    if {[HTTP::path] starts_with \"/admin\"} {\n        HTTP::respond 403 content \"Forbidden\"\n    }\n}";
    let result = translate_irule(src);
    assert_eq!(result.service_policies.len(), 1);
    let policy = &result.service_policies[0];
    assert_eq!(policy.name, "translated-policy");
    assert_eq!(policy.rules.len(), 1);
    assert_eq!(policy.rules[0].action, "deny");
}

#[test]
fn asm_disable_builds_waf_exclusion() {
    let src = "when HTTP_REQUEST {\n    if {[HTTP::path] starts_with \"/api\"} {\n        ASM::disable\n    }\n}";
    let result = translate_irule(src);
    assert_eq!(result.waf_exclusion_rules.len(), 1);
    let waf = &result.waf_exclusion_rules[0];
    let pm = waf.path_match.as_ref().expect("path match on waf rule");
    assert_eq!(pm.value, "/api");
}

#[test]
fn untranslatable_event_is_flagged() {
    let src = "when CLIENT_ACCEPTED {\n    set foo 1\n}";
    let result = translate_irule(src);
    assert_eq!(result.untranslatable_count(), 1);
    assert!(
        result
            .items
            .iter()
            .any(|i| i.status == TranslateStatus::Untranslatable && i.diagnostic_code == "XC201")
    );
    assert!(result.coverage_pct.abs() < 1e-9);
}

#[test]
fn empty_rule_is_full_coverage() {
    let result = translate_irule("when HTTP_REQUEST {\n}");
    assert!(result.items.is_empty());
    assert!((result.coverage_pct - 100.0).abs() < 1e-9);
}

// An `else` body must keep the enclosing match criteria
// instead of translating to an unconditional catch-all route.
#[test]
fn else_body_keeps_enclosing_criteria() {
    let src = "when HTTP_REQUEST {\n    if {[HTTP::host] eq \"a.example.com\"} {\n        if {[HTTP::path] starts_with \"/api\"} {\n            pool a-api\n        } else {\n            pool a-web\n        }\n    }\n}";
    let result = translate_irule(src);

    let web = route_for_pool(&result, "a-web").expect("a-web route");
    let host = web
        .host_match
        .as_ref()
        .expect("else route must keep the enclosing host criterion");
    assert_eq!(host.value, "a.example.com");
    // The else branch is the *complement* of the inner `if`, so it must not
    // carry the inner path criterion.
    assert!(web.path_match.is_none());

    // The if-branch keeps both the outer host and its own path.
    let api = route_for_pool(&result, "a-api").expect("a-api route");
    assert_eq!(api.host_match.as_ref().unwrap().value, "a.example.com");
    assert_eq!(api.path_match.as_ref().unwrap().value, "/api");
}

// `switch -regexp` arms must translate to regex path
// matches, not prefix matches on the literal pattern.
#[test]
fn switch_regexp_path_is_regex_not_prefix() {
    let src = "when HTTP_REQUEST {\n    switch -regexp [HTTP::path] {\n        {^/api/.*} { pool api_pool }\n    }\n}";
    let result = translate_irule(src);
    let route = route_for_pool(&result, "api_pool").expect("api_pool route");
    let pm = route.path_match.as_ref().expect("path match");
    assert_eq!(pm.match_type, "regex");
    assert_eq!(pm.value, "^/api/.*");
}

// A plain (exact) `switch` arm containing `*` is a literal,
// not a prefix.
#[test]
fn switch_exact_path_with_star_is_literal() {
    let src = "when HTTP_REQUEST {\n    switch [HTTP::path] {\n        \"/a*\" { pool star_pool }\n    }\n}";
    let result = translate_irule(src);
    let route = route_for_pool(&result, "star_pool").expect("star_pool route");
    let pm = route.path_match.as_ref().expect("path match");
    assert_eq!(pm.match_type, "exact");
    assert_eq!(pm.value, "/a*");
}

// A `switch -glob` host arm must become a regex host match,
// not an exact match on the glob text (which would never match).
#[test]
fn switch_glob_host_is_regex() {
    let src = "when HTTP_REQUEST {\n    switch -glob [HTTP::host] {\n        \"*.example.com\" { pool wild_pool }\n    }\n}";
    let result = translate_irule(src);
    let route = route_for_pool(&result, "wild_pool").expect("wild_pool route");
    let hm = route.host_match.as_ref().expect("host match");
    assert_eq!(hm.match_type, "regex");
    assert_eq!(hm.value, "^.*\\.example\\.com$");
}

// `-nocase` must be honoured, producing a case-insensitive
// regex.
#[test]
fn switch_nocase_glob_host_is_case_insensitive_regex() {
    let src = "when HTTP_REQUEST {\n    switch -glob -nocase [HTTP::host] {\n        \"*.Example.com\" { pool ci_pool }\n    }\n}";
    let result = translate_irule(src);
    let route = route_for_pool(&result, "ci_pool").expect("ci_pool route");
    let hm = route.host_match.as_ref().expect("host match");
    assert_eq!(hm.match_type, "regex");
    assert!(
        hm.value.starts_with("(?i)"),
        "expected case-insensitive regex, got {}",
        hm.value
    );
}

// A fall-through arm (`"/a*" -`) must attach its pattern to
// the shared body, so no pattern's traffic is silently dropped.
#[test]
fn switch_fallthrough_arm_routes_all_patterns() {
    let src = "when HTTP_REQUEST {\n    switch -glob [HTTP::path] {\n        \"/a*\" - \"/b*\" { pool shared }\n    }\n}";
    let result = translate_irule(src);
    let mut prefixes: Vec<String> = result
        .routes
        .iter()
        .filter(|r| r.origin_pool.as_ref().map(|p| p.name.as_str()) == Some("shared"))
        .filter_map(|r| r.path_match.as_ref())
        .map(|pm| {
            assert_eq!(pm.match_type, "prefix");
            pm.value.clone()
        })
        .collect();
    prefixes.sort();
    assert_eq!(prefixes, vec!["/a".to_owned(), "/b".to_owned()]);
}

// A fall-through pattern that runs into the final `default`
// arm (`"/old" - default { … }`) shares the default's body in Tcl, so the
// translator must emit both a `/old`-specific route and the catch-all route.
#[test]
fn switch_fallthrough_into_default_routes_pattern_and_catch_all() {
    let src = "when HTTP_REQUEST {\n    switch [HTTP::path] {\n        \"/old\" - default { pool legacy }\n    }\n}";
    let result = translate_irule(src);
    let legacy: Vec<Option<String>> = result
        .routes
        .iter()
        .filter(|r| r.origin_pool.as_ref().map(|p| p.name.as_str()) == Some("legacy"))
        .map(|r| r.path_match.as_ref().map(|pm| pm.value.clone()))
        .collect();
    // The `/old` pattern must produce its own path-matched route...
    assert!(
        legacy.iter().any(|p| p.as_deref() == Some("/old")),
        "fall-through pattern into default lost its /old route: {legacy:?}"
    );
    // ...and the catch-all default route (no path criterion) must remain.
    assert!(
        legacy.iter().any(Option::is_none),
        "catch-all default route missing: {legacy:?}"
    );
}

// The Terraform simple-route renderer must emit host and
// method criteria the translator recorded.
#[test]
fn terraform_simple_route_emits_host_and_method() {
    let src = "when HTTP_REQUEST {\n    if {[HTTP::host] eq \"h.example.com\" && [HTTP::method] eq \"POST\"} {\n        pool p\n    }\n}";
    let result = translate_irule(src);
    let tf = render_terraform(&result, "ns", "lb");
    assert!(tf.contains("host {"), "TF missing host block:\n{tf}");
    assert!(tf.contains("h.example.com"), "TF missing host value:\n{tf}");
    assert!(
        tf.contains("http_method {"),
        "TF missing http_method block:\n{tf}"
    );
    assert!(tf.contains("\"POST\""), "TF missing method value:\n{tf}");
}

// A pool name carrying HCL-invalid characters (`/`, `-`) must
// be sanitised into a valid resource label, and the route's reference must use
// the SAME sanitised label — while the real name is preserved in `name = "…"`.
#[test]
fn terraform_pool_name_with_slashes_is_sanitised() {
    let src = "when HTTP_REQUEST {\n    pool /Common/web-pool\n}";
    let result = translate_irule(src);
    let tf = render_terraform(&result, "ns", "lb");
    // The resource label must not contain the raw `/` (invalid HCL identifier).
    assert!(
        !tf.contains("volterra_origin_pool\" \"/Common/web-pool\""),
        "raw slashed name leaked into resource label:\n{tf}"
    );
    // The real object name is preserved as the `name` attribute.
    assert!(
        tf.contains("name      = \"/Common/web-pool\""),
        "TF dropped the real pool name:\n{tf}"
    );
    // The label and the route reference must agree. Extract the label from the
    // resource header and confirm the reference uses it verbatim.
    let header = tf
        .lines()
        .find(|l| l.contains("resource \"volterra_origin_pool\""))
        .expect("origin_pool resource header");
    let label = header
        .split('"')
        .nth(3)
        .expect("resource label between quotes");
    assert!(
        label.chars().enumerate().all(|(i, c)| if i == 0 {
            c.is_ascii_alphabetic() || c == '_'
        } else {
            c.is_ascii_alphanumeric() || c == '_' || c == '-'
        }),
        "sanitised label is not a valid HCL identifier: {label:?}"
    );
    assert!(
        tf.contains(&format!("volterra_origin_pool.{label}.name")),
        "route reference does not use the sanitised label {label:?}:\n{tf}"
    );
}

// The Terraform redirect-route renderer must emit the host
// criterion, otherwise every request is redirected.
#[test]
fn terraform_redirect_route_emits_host() {
    let src = "when HTTP_REQUEST {\n    if {[HTTP::host] eq \"old.example.com\"} {\n        HTTP::redirect \"https://new.example.com\"\n    }\n}";
    let result = translate_irule(src);
    let tf = render_terraform(&result, "ns", "lb");
    assert!(tf.contains("redirect_route {"), "no redirect route:\n{tf}");
    assert!(
        tf.contains("host {"),
        "redirect route dropped host criterion:\n{tf}"
    );
    assert!(
        tf.contains("old.example.com"),
        "TF missing host value:\n{tf}"
    );
}

// The JSON redirect-route renderer must emit the host
// criterion.
#[test]
fn json_redirect_route_emits_host() {
    let src = "when HTTP_REQUEST {\n    if {[HTTP::host] eq \"old.example.com\"} {\n        HTTP::redirect \"https://new.example.com\"\n    }\n}";
    let result = translate_irule(src);
    let json = render_json(&result, "ns", "lb");
    let routes = json["http_loadbalancer"]["spec"]["routes"]
        .as_array()
        .expect("routes array");
    let redirect = routes
        .iter()
        .find(|r| r["type"] == "redirect_route")
        .expect("redirect route");
    assert_eq!(redirect["match"]["host"]["exact"], "old.example.com");
}

// The JSON simple-route renderer must emit the method
// criterion.
#[test]
fn json_simple_route_emits_method() {
    let src =
        "when HTTP_REQUEST {\n    if {[HTTP::method] eq \"POST\"} {\n        pool p\n    }\n}";
    let result = translate_irule(src);
    let json = render_json(&result, "ns", "lb");
    let routes = json["http_loadbalancer"]["spec"]["routes"]
        .as_array()
        .expect("routes array");
    let simple = routes
        .iter()
        .find(|r| r["type"] == "simple_route")
        .expect("simple route");
    let methods = simple["match"]["http_method"]["methods"]
        .as_array()
        .expect("methods array");
    assert_eq!(methods[0], "POST");
}
