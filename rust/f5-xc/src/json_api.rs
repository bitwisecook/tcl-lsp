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

//! Render an [`XCTranslationResult`] as JSON matching the ves.io API schema.
//! Object key order follows dict insertion order (`serde_json`
//! `preserve_order`).

use serde_json::{Map, Value, json};

use crate::model::{
    XCCookieMatch, XCHeaderAction, XCHeaderMatch, XCOriginPool, XCPathMatch, XCQueryMatch, XCRoute,
    XCServicePolicy, XCServicePolicyRule, XCTranslationResult, XCWafExclusionRule,
};

fn obj(entries: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in entries {
        m.insert(k.to_owned(), v);
    }
    Value::Object(m)
}

fn path_match_dict(m: &XCPathMatch) -> Value {
    let mut o = Map::new();
    o.insert(m.match_type.clone(), json!(m.value));
    if m.invert {
        o.insert("invert_matcher".to_owned(), json!(true));
    }
    Value::Object(o)
}

fn header_match_dict(m: &XCHeaderMatch) -> Value {
    let mut o = Map::new();
    o.insert("name".to_owned(), json!(m.name));
    if m.match_type == "presence" {
        o.insert("presence".to_owned(), json!(true));
    } else {
        o.insert(m.match_type.clone(), json!(m.value));
    }
    if m.invert {
        o.insert("invert_matcher".to_owned(), json!(true));
    }
    Value::Object(o)
}

fn cookie_match_dict(m: &XCCookieMatch) -> Value {
    let mut o = Map::new();
    o.insert("name".to_owned(), json!(m.name));
    if m.match_type == "presence" {
        o.insert("presence".to_owned(), json!(true));
    } else {
        o.insert(m.match_type.clone(), json!(m.value));
    }
    if m.invert {
        o.insert("invert_matcher".to_owned(), json!(true));
    }
    Value::Object(o)
}

fn query_match_dict(m: &XCQueryMatch) -> Value {
    let mut o = Map::new();
    o.insert(m.match_type.clone(), json!(m.value));
    Value::Object(o)
}

/// Build the `match` object for a route from every recorded criterion
/// (path, host, method, headers, query, cookies). Shared by the simple,
/// redirect, and direct-response route renderers so none silently drops a
/// criterion the translator recorded.
fn route_match_map(route: &XCRoute) -> Map<String, Value> {
    let mut m = Map::new();
    if let Some(pm) = &route.path_match {
        m.insert("path".to_owned(), path_match_dict(pm));
    }
    if let Some(hm) = &route.host_match {
        m.insert(
            "host".to_owned(),
            obj(vec![(hm.match_type.as_str(), json!(hm.value))]),
        );
    }
    if let Some(mm) = &route.method_match {
        let mut method = Map::new();
        method.insert("methods".to_owned(), json!(mm.methods));
        if mm.invert {
            method.insert("invert_matcher".to_owned(), json!(true));
        }
        m.insert("http_method".to_owned(), Value::Object(method));
    }
    if !route.header_matches.is_empty() {
        m.insert(
            "headers".to_owned(),
            Value::Array(route.header_matches.iter().map(header_match_dict).collect()),
        );
    }
    if let Some(qm) = &route.query_match {
        m.insert("query_params".to_owned(), json!([query_match_dict(qm)]));
    }
    if !route.cookie_matches.is_empty() {
        m.insert(
            "cookies".to_owned(),
            Value::Array(route.cookie_matches.iter().map(cookie_match_dict).collect()),
        );
    }
    m
}

fn render_origin_pool(pool: &XCOriginPool, namespace: &str) -> Value {
    json!({
        "metadata": { "name": pool.name, "namespace": namespace },
        "spec": {
            "origin_servers": [ { "public_name": { "dns_name": "example.com" } } ],
            "port": pool.port,
            "endpoint_selection": "LOCAL_PREFERRED",
            "loadbalancer_algorithm": "LB_OVERRIDE",
        },
    })
}

fn render_route(route: &XCRoute) -> Value {
    if route.redirect.is_some() {
        render_redirect_route(route)
    } else if route.direct_response.is_some() {
        render_direct_response_route(route)
    } else {
        render_simple_route(route)
    }
}

fn render_simple_route(route: &XCRoute) -> Value {
    let mut result = Map::new();
    result.insert("type".to_owned(), json!("simple_route"));

    let m = route_match_map(route);
    if !m.is_empty() {
        result.insert("match".to_owned(), Value::Object(m));
    }

    if let Some(op) = &route.origin_pool {
        result.insert(
            "origin_pools".to_owned(),
            json!([{ "pool": { "name": op.name } }]),
        );
    }

    // Group header actions by `{target}_headers_to_{operation}`.
    let mut headers: Map<String, Value> = Map::new();
    for action in &route.header_actions {
        let key = format!("{}_headers_to_{}", action.target, action.operation);
        let arr = headers
            .entry(key)
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(list) = arr.as_array_mut() {
            if action.operation == "remove" {
                list.push(json!(action.name));
            } else {
                list.push(json!({ "name": action.name, "value": action.value }));
            }
        }
    }
    for (k, v) in headers {
        result.insert(k, v);
    }

    Value::Object(result)
}

fn render_redirect_route(route: &XCRoute) -> Value {
    let mut result = Map::new();
    result.insert("type".to_owned(), json!("redirect_route"));
    let m = route_match_map(route);
    if !m.is_empty() {
        result.insert("match".to_owned(), Value::Object(m));
    }
    if let Some(redirect) = &route.redirect {
        let mut action = Map::new();
        action.insert("host_redirect".to_owned(), json!(redirect.url));
        let code = if redirect.response_code == 301 {
            "MOVED_PERMANENTLY"
        } else {
            "FOUND"
        };
        action.insert("response_code".to_owned(), json!(code));
        result.insert("redirect_action".to_owned(), Value::Object(action));
    }
    Value::Object(result)
}

fn render_direct_response_route(route: &XCRoute) -> Value {
    let mut result = Map::new();
    result.insert("type".to_owned(), json!("direct_response_route"));
    let m = route_match_map(route);
    if !m.is_empty() {
        result.insert("match".to_owned(), Value::Object(m));
    }
    if let Some(dr) = &route.direct_response {
        let mut action = Map::new();
        action.insert("status".to_owned(), json!(dr.status_code.to_string()));
        if !dr.body.is_empty() {
            action.insert("content".to_owned(), json!(dr.body));
        }
        result.insert("direct_response_action".to_owned(), Value::Object(action));
    }
    Value::Object(result)
}

fn render_service_policy_rule(rule: &XCServicePolicyRule) -> Value {
    let mut rule_spec = Map::new();
    rule_spec.insert("action".to_owned(), json!(rule.action.to_uppercase()));

    let mut m = Map::new();
    if let Some(pm) = &rule.path_match {
        m.insert("path".to_owned(), path_match_dict(pm));
    }
    if let Some(hm) = &rule.host_match {
        m.insert(
            "host".to_owned(),
            obj(vec![(hm.match_type.as_str(), json!(hm.value))]),
        );
    }
    if let Some(mm) = &rule.method_match {
        let mut method = Map::new();
        method.insert("methods".to_owned(), json!(mm.methods));
        if mm.invert {
            method.insert("invert_matcher".to_owned(), json!(true));
        }
        m.insert("http_method".to_owned(), Value::Object(method));
    }
    if !rule.header_matches.is_empty() {
        m.insert(
            "headers".to_owned(),
            Value::Array(rule.header_matches.iter().map(header_match_dict).collect()),
        );
    }
    if let Some(qm) = &rule.query_match {
        m.insert("query_params".to_owned(), json!([query_match_dict(qm)]));
    }
    if !rule.cookie_matches.is_empty() {
        m.insert(
            "cookies".to_owned(),
            Value::Array(rule.cookie_matches.iter().map(cookie_match_dict).collect()),
        );
    }
    if !rule.ip_prefix_list.is_empty() {
        m.insert(
            "client_source".to_owned(),
            obj(vec![("ip_prefix_list", json!(rule.ip_prefix_list))]),
        );
    }
    if !m.is_empty() {
        rule_spec.insert("match".to_owned(), Value::Object(m));
    }

    let mut rule_metadata = Map::new();
    rule_metadata.insert("name".to_owned(), json!(rule.name));
    if !rule.description.is_empty() {
        rule_metadata.insert("description".to_owned(), json!(rule.description));
    }

    obj(vec![
        ("metadata", Value::Object(rule_metadata)),
        ("spec", Value::Object(rule_spec)),
    ])
}

fn render_waf_exclusion_rule(rule: &XCWafExclusionRule) -> Value {
    let mut metadata = Map::new();
    metadata.insert("name".to_owned(), json!(rule.name));
    if !rule.description.is_empty() {
        metadata.insert("description".to_owned(), json!(rule.description));
    }
    let mut result = Map::new();
    result.insert("metadata".to_owned(), Value::Object(metadata));

    let mut match_condition = Map::new();
    if let Some(pm) = &rule.path_match {
        match_condition.insert("path".to_owned(), path_match_dict(pm));
    }
    if let Some(ip_set) = &rule.ip_prefix_set {
        match_condition.insert(
            "client_source".to_owned(),
            obj(vec![("ip_prefix_set", json!({ "ref": [ip_set] }))]),
        );
    }
    if !match_condition.is_empty() {
        result.insert("match_condition".to_owned(), Value::Object(match_condition));
    }

    result.insert(
        "app_firewall_detection_control".to_owned(),
        json!({ "exclude_all_attack_types": true }),
    );
    Value::Object(result)
}

fn render_service_policy(policy: &XCServicePolicy, namespace: &str) -> Value {
    json!({
        "metadata": { "name": policy.name, "namespace": namespace },
        "spec": {
            "algo": policy.algo,
            "rule_list": {
                "rules": policy.rules.iter().map(render_service_policy_rule).collect::<Vec<_>>(),
            },
        },
    })
}

fn render_load_balancer(result: &XCTranslationResult, namespace: &str, name: &str) -> Value {
    let mut lb_spec = Map::new();
    lb_spec.insert("domains".to_owned(), json!(["example.com"]));
    lb_spec.insert("http".to_owned(), json!({ "dns_volterra_managed": false }));

    if !result.routes.is_empty() {
        lb_spec.insert(
            "routes".to_owned(),
            Value::Array(result.routes.iter().map(render_route).collect()),
        );
    }

    let ha = &result.header_actions;
    let is = |a: &&XCHeaderAction, target: &str, remove: bool| {
        a.target == target && ((a.operation == "remove") == remove)
    };
    let add_pairs = |acts: Vec<&XCHeaderAction>| -> Value {
        Value::Array(
            acts.into_iter()
                .map(|a| json!({ "name": a.name, "value": a.value }))
                .collect(),
        )
    };
    let names = |acts: Vec<&XCHeaderAction>| -> Value {
        Value::Array(acts.into_iter().map(|a| json!(a.name)).collect())
    };

    let request_add: Vec<_> = ha.iter().filter(|a| is(a, "request", false)).collect();
    let request_remove: Vec<_> = ha.iter().filter(|a| is(a, "request", true)).collect();
    let response_add: Vec<_> = ha.iter().filter(|a| is(a, "response", false)).collect();
    let response_remove: Vec<_> = ha.iter().filter(|a| is(a, "response", true)).collect();

    if !request_add.is_empty() {
        lb_spec.insert("request_headers_to_add".to_owned(), add_pairs(request_add));
    }
    if !request_remove.is_empty() {
        lb_spec.insert(
            "request_headers_to_remove".to_owned(),
            names(request_remove),
        );
    }
    if !response_add.is_empty() {
        lb_spec.insert(
            "response_headers_to_add".to_owned(),
            add_pairs(response_add),
        );
    }
    if !response_remove.is_empty() {
        lb_spec.insert(
            "response_headers_to_remove".to_owned(),
            names(response_remove),
        );
    }

    if !result.waf_exclusion_rules.is_empty() {
        lb_spec.insert(
            "waf_exclusion_rules".to_owned(),
            Value::Array(
                result
                    .waf_exclusion_rules
                    .iter()
                    .map(render_waf_exclusion_rule)
                    .collect(),
            ),
        );
    }

    if !result.service_policies.is_empty() {
        let policies: Vec<Value> = result
            .service_policies
            .iter()
            .map(|p| json!({ "name": p.name, "namespace": namespace }))
            .collect();
        lb_spec.insert(
            "active_service_policies".to_owned(),
            json!({ "policies": policies }),
        );
    }

    json!({
        "metadata": { "name": name, "namespace": namespace },
        "spec": Value::Object(lb_spec),
    })
}

/// Render an XC translation result as ves.io JSON-API objects: an object with
/// `origin_pools`, `service_policies`, `http_loadbalancer`, and `summary`.
#[must_use]
pub fn render_json(result: &XCTranslationResult, namespace: &str, lb_name: &str) -> Value {
    let mut output = Map::new();

    if !result.origin_pools.is_empty() {
        output.insert(
            "origin_pools".to_owned(),
            Value::Array(
                result
                    .origin_pools
                    .iter()
                    .map(|p| render_origin_pool(p, namespace))
                    .collect(),
            ),
        );
    }
    if !result.service_policies.is_empty() {
        output.insert(
            "service_policies".to_owned(),
            Value::Array(
                result
                    .service_policies
                    .iter()
                    .map(|p| render_service_policy(p, namespace))
                    .collect(),
            ),
        );
    }
    output.insert(
        "http_loadbalancer".to_owned(),
        render_load_balancer(result, namespace, lb_name),
    );

    let items: Vec<Value> = result
        .items
        .iter()
        .map(|item| {
            json!({
                "status": item.status.as_str(),
                "kind": item.kind.as_str(),
                "command": item.irule_command,
                "xc_description": item.xc_description,
                "note": item.note,
                "diagnostic_code": item.diagnostic_code,
            })
        })
        .collect();
    output.insert(
        "summary".to_owned(),
        json!({
            "coverage_pct": result.coverage_pct,
            "translatable": result.translatable_count(),
            "partial": result.partial_count(),
            "untranslatable": result.untranslatable_count(),
            "advisory": result.advisory_count(),
            "items": items,
        }),
    );

    Value::Object(output)
}
