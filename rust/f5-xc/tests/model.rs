//! Model-level assertions for the `f5-xc` translator — cover a subset of
//! cases that check the *shape* of the produced XC constructs (which the
//! summary-only differential fixture does not capture).

use f5_xc::model::TranslateStatus;
use f5_xc::translate_irule;

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
