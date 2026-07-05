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

//! Evaluate parsed BIG-IP LTM policies against captured request state.
//!
//! Purely declarative — no
//! runtime needed; the captured request fully determines the outcome. Operates
//! on the parsed [`BigipPolicy`] model and a [`RequestState`] derived from an
//! explain-flow [`Session`].

use regex::RegexBuilder;

use crate::flow::Session;
use crate::model::{BigipPolicy, BigipPolicyAction, BigipPolicyCondition};

/// The captured request state a policy condition can read. Header names are
/// stored lowercased for case-insensitive lookups.
#[derive(Clone, Debug, Default)]
pub struct RequestState {
    /// Request method.
    pub method: String,
    /// Effective host (Host header, else captured host field).
    pub host: String,
    /// Full request URI.
    pub uri: String,
    /// URI path component.
    pub path: String,
    /// URI query component.
    pub query: String,
    /// Lowercase-keyed request headers.
    pub headers: Vec<(String, String)>,
    /// TLS SNI.
    pub sni: String,
    /// Negotiated/legacy TLS version.
    pub tls_version: String,
    /// Client source address.
    pub client_addr: String,
    /// Server (VIP) address.
    pub server_addr: String,
}

impl RequestState {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Per-condition trace: what the condition saw and whether it matched.
#[derive(Clone, Debug)]
pub struct ConditionTrace {
    /// Condition operand (e.g. `http-host`).
    pub operand: String,
    /// Operand selector (e.g. `path`).
    pub selector: String,
    /// Match operator (e.g. `starts-with`).
    pub operator: String,
    /// Expected values.
    pub expected: Vec<String>,
    /// Observed captured value.
    pub actual: String,
    /// Whether the condition matched.
    pub matched: bool,
    /// `http-header` / `ssl-extension` target name.
    pub name: String,
    /// Whether the condition is negated.
    pub negate: bool,
    /// Condition event phase.
    pub event: String,
    /// Populated when the condition couldn't be evaluated.
    pub note: String,
}

/// An action that fired (only present on matched rules).
#[derive(Clone, Debug)]
pub struct ActionTrace {
    /// Action target (e.g. `forward`).
    pub target: String,
    /// Action verb (e.g. `select`).
    pub verb: String,
    /// Most-relevant single value: pool / location / `name=value`.
    pub value: String,
}

/// Per-rule decision: which conditions matched and (if fired) which actions ran.
#[derive(Clone, Debug)]
pub struct RuleDecision {
    /// Rule name.
    pub rule: String,
    /// Rule ordinal.
    pub ordinal: i64,
    /// Whether every condition matched.
    pub matched: bool,
    /// Whether the rule's actions fired under the policy strategy.
    pub fired: bool,
    /// Per-condition traces.
    pub conditions: Vec<ConditionTrace>,
    /// Action traces (non-empty only on fired rules).
    pub actions: Vec<ActionTrace>,
}

/// Per-policy decision: the rule walk plus the strategy that picked the winner.
#[derive(Clone, Debug)]
pub struct PolicyDecision {
    /// Policy full path.
    pub policy: String,
    /// Reported strategy (`first-match`/`all-match`/`best-match-approx`).
    pub strategy: String,
    /// Per-rule decisions.
    pub rules: Vec<RuleDecision>,
}

/// Evaluate `policy` against `state` and return per-rule decisions.
#[must_use]
pub fn evaluate_policy(policy: &BigipPolicy, state: &RequestState) -> PolicyDecision {
    let mut rules_built: Vec<RuleDecision> = Vec::new();
    for rule in &policy.rules {
        let cond_traces: Vec<ConditionTrace> = rule
            .conditions
            .iter()
            .map(|c| evaluate_condition(c, state))
            .collect();
        // A rule with no conditions is unconditional (matches always).
        let all_ok = cond_traces.iter().all(|t| t.matched);
        rules_built.push(RuleDecision {
            rule: rule.name.clone(),
            ordinal: rule.ordinal,
            matched: all_ok,
            fired: false,
            conditions: cond_traces,
            actions: Vec::new(),
        });
    }

    let raw_strategy = if policy.strategy.is_empty() {
        "first-match".to_owned()
    } else {
        policy.strategy.to_lowercase()
    };
    let mut reported_strategy = raw_strategy.clone();
    let mut matched_indexes: Vec<usize> = Vec::new();
    if raw_strategy == "first-match" {
        if let Some(i) = rules_built.iter().position(|d| d.matched) {
            matched_indexes.push(i);
        }
    } else if raw_strategy == "all-match" {
        matched_indexes = rules_built
            .iter()
            .enumerate()
            .filter(|(_, d)| d.matched)
            .map(|(i, _)| i)
            .collect();
    } else {
        // best-match approximation (also used for unknown strategies). The
        // first matched rule wins ties (the score comparison is strict), so a
        // larger condition count is needed to displace an earlier winner.
        "best-match-approx".clone_into(&mut reported_strategy);
        let mut best: Option<usize> = None;
        let mut best_score: usize = 0;
        for (i, decision) in rules_built.iter().enumerate() {
            if !decision.matched {
                continue;
            }
            let score = decision.conditions.len();
            if best.is_none() || score > best_score {
                best_score = score;
                best = Some(i);
            }
        }
        if let Some(i) = best {
            matched_indexes.push(i);
        }
    }

    let mut final_rules: Vec<RuleDecision> = Vec::with_capacity(rules_built.len());
    for (i, mut decision) in rules_built.into_iter().enumerate() {
        if matched_indexes.contains(&i) {
            decision.matched = true;
            decision.fired = true;
            decision.actions = policy.rules[i].actions.iter().map(action_trace).collect();
        }
        final_rules.push(decision);
    }

    PolicyDecision {
        policy: policy.full_path.clone(),
        strategy: reported_strategy,
        rules: final_rules,
    }
}

/// Derive a [`RequestState`] from an explain-flow session (front-side client
/// flow only).
#[must_use]
pub fn request_state_from_session(session: &Session) -> RequestState {
    let fc = &session.front.client;
    let mut headers: Vec<(String, String)> = Vec::new();
    for (k, v) in &fc.http_request_headers {
        let key = k.to_lowercase();
        if let Some(slot) = headers.iter_mut().find(|(hk, _)| *hk == key) {
            v.clone_into(&mut slot.1);
        } else {
            headers.push((key, v.clone()));
        }
    }
    let host = {
        let h = headers
            .iter()
            .find(|(k, _)| k == "host")
            .map(|(_, v)| v.as_str())
            .filter(|v| !v.is_empty())
            .unwrap_or(&fc.http_host);
        h.trim().to_owned()
    };
    let path = if fc.http_path.is_empty() {
        split_path(&fc.http_uri)
    } else {
        fc.http_path.clone()
    };
    let query = if fc.http_query.is_empty() {
        split_query(&fc.http_uri)
    } else {
        fc.http_query.clone()
    };
    let tls_version = if fc.tls_chosen_version.is_empty() {
        fc.tls_version.clone()
    } else {
        fc.tls_chosen_version.clone()
    };
    RequestState {
        method: fc.http_method.clone(),
        host,
        uri: fc.http_uri.clone(),
        path,
        query,
        headers,
        sni: fc.tls_sni.clone(),
        tls_version,
        client_addr: fc.src_ip.clone(),
        server_addr: fc.dst_ip.clone(),
    }
}

fn evaluate_condition(cond: &BigipPolicyCondition, state: &RequestState) -> ConditionTrace {
    let (actual, present, note, unevaluable) = extract_actual(cond, state);
    let matched = if unevaluable {
        false
    } else {
        let raw = if cond.operator == "exists" {
            present
        } else if cond.operator == "missing" {
            !present
        } else {
            apply_operator(&cond.operator, &actual, &cond.values, cond.case_insensitive)
        };
        if cond.negate { !raw } else { raw }
    };
    ConditionTrace {
        operand: cond.operand.clone(),
        selector: cond.selector.clone(),
        operator: cond.operator.clone(),
        expected: cond.values.clone(),
        actual,
        matched,
        name: cond.name.clone(),
        negate: cond.negate,
        event: cond.event.clone(),
        note,
    }
}

/// Return `(observed_value, present, note, unevaluable)`.
fn extract_actual(
    cond: &BigipPolicyCondition,
    state: &RequestState,
) -> (String, bool, String, bool) {
    let op = cond.operand.as_str();
    let sel = cond.selector.as_str();
    match op {
        "http-host" => {
            let present = !state.host.is_empty();
            (
                state.host.clone(),
                present,
                note_for(present, "no Host captured"),
                false,
            )
        }
        "http-uri" => extract_http_uri(sel, state),
        "http-method" => {
            let present = !state.method.is_empty();
            (
                state.method.clone(),
                present,
                note_for(present, "no method captured"),
                false,
            )
        }
        "http-header" => extract_http_header(cond, state),
        "ssl-extension" => extract_ssl_extension(cond, state),
        "tcp" => extract_tcp(sel, state),
        _ => (
            String::new(),
            false,
            format!("operand {} not supported in v1", py_repr(op)),
            true,
        ),
    }
}

/// Empty when `present`, else the message — shared note helper for
/// [`extract_actual`].
fn note_for(present: bool, msg: &str) -> String {
    if present {
        String::new()
    } else {
        msg.to_owned()
    }
}

fn extract_http_uri(sel: &str, state: &RequestState) -> (String, bool, String, bool) {
    match sel {
        "path" => {
            let present = !state.path.is_empty();
            (
                state.path.clone(),
                present,
                note_for(present, "no URI captured"),
                false,
            )
        }
        "query" => {
            let present = !state.query.is_empty();
            (
                state.query.clone(),
                present,
                note_for(present, "no query string captured"),
                false,
            )
        }
        "host" => {
            let present = !state.host.is_empty();
            (
                state.host.clone(),
                present,
                note_for(present, "no Host captured"),
                false,
            )
        }
        _ => {
            let present = !state.uri.is_empty();
            (
                state.uri.clone(),
                present,
                note_for(present, "no URI captured"),
                false,
            )
        }
    }
}

fn extract_http_header(
    cond: &BigipPolicyCondition,
    state: &RequestState,
) -> (String, bool, String, bool) {
    if cond.name.is_empty() {
        return (
            String::new(),
            false,
            "http-header condition has no `name` — cannot evaluate".to_owned(),
            true,
        );
    }
    let key = cond.name.to_lowercase();
    let present = state.header(&key).is_some();
    let value = state.header(&key).unwrap_or("").to_owned();
    let note = if present {
        String::new()
    } else {
        format!("header {} not seen in capture", py_repr(&cond.name))
    };
    (value, present, note, false)
}

fn extract_ssl_extension(
    cond: &BigipPolicyCondition,
    state: &RequestState,
) -> (String, bool, String, bool) {
    if !cond.name.is_empty() && cond.name.to_lowercase() != "server-name" {
        return (
            String::new(),
            false,
            format!("ssl-extension {} not supported in v1", py_repr(&cond.name)),
            true,
        );
    }
    let present = !state.sni.is_empty();
    (
        state.sni.clone(),
        present,
        note_for(present, "no SNI captured"),
        false,
    )
}

fn extract_tcp(sel: &str, state: &RequestState) -> (String, bool, String, bool) {
    if sel == "address" {
        (
            state.client_addr.clone(),
            !state.client_addr.is_empty(),
            String::new(),
            false,
        )
    } else {
        (
            String::new(),
            false,
            format!("tcp selector {} not supported in v1", py_repr(sel)),
            true,
        )
    }
}

fn apply_operator(operator: &str, actual: &str, values: &[String], case_insensitive: bool) -> bool {
    if values.is_empty() {
        return false;
    }
    let (a, vs): (String, Vec<String>) = if case_insensitive {
        (
            actual.to_lowercase(),
            values.iter().map(|v| v.to_lowercase()).collect(),
        )
    } else {
        (actual.to_owned(), values.to_vec())
    };
    match operator {
        "equals" => vs.contains(&a),
        "starts-with" => vs.iter().any(|v| a.starts_with(v)),
        "ends-with" => vs.iter().any(|v| a.ends_with(v)),
        "contains" => vs.iter().any(|v| a.contains(v.as_str())),
        // `matches` uses the original-case values + actual with an IGNORECASE
        // flag. An invalid regex no-matches here (it is flagged separately).
        "matches" => values.iter().any(|v| {
            RegexBuilder::new(v)
                .case_insensitive(case_insensitive)
                .build()
                .is_ok_and(|re| re.is_match(actual))
        }),
        "less" => numeric_compare(actual, values, |x, y| x < y),
        "greater" => numeric_compare(actual, values, |x, y| x > y),
        "less-or-equal" => numeric_compare(actual, values, |x, y| x <= y),
        "greater-or-equal" => numeric_compare(actual, values, |x, y| x >= y),
        _ => false,
    }
}

fn numeric_compare(actual: &str, values: &[String], op: impl Fn(f64, f64) -> bool) -> bool {
    let Ok(a) = actual.trim().parse::<f64>() else {
        return false;
    };
    values
        .iter()
        .any(|v| v.trim().parse::<f64>().is_ok_and(|fv| op(a, fv)))
}

fn action_trace(action: &BigipPolicyAction) -> ActionTrace {
    ActionTrace {
        target: action.target.clone(),
        verb: action.verb.clone(),
        value: action_summary_value(action),
    }
}

fn action_summary_value(action: &BigipPolicyAction) -> String {
    if action.target == "forward" && !action.pool.is_empty() {
        return action.pool.clone();
    }
    if action.target == "http-reply" && !action.location.is_empty() {
        return action.location.clone();
    }
    if action.target == "http-uri" && action.verb == "replace" {
        if !action.path.is_empty() {
            return format!("path={}", action.path);
        }
        if !action.query.is_empty() {
            return format!("query={}", action.query);
        }
        if !action.host.is_empty() {
            return format!("host={}", action.host);
        }
    }
    if action.target == "http-header" {
        if action.verb == "insert" {
            return format!("{}={}", action.name, action.value);
        }
        if action.verb == "remove" {
            return action.name.clone();
        }
    }
    if action.target == "tcp" && action.verb == "reset" {
        return "RST".to_owned();
    }
    for v in [
        &action.pool,
        &action.location,
        &action.name,
        &action.value,
        &action.path,
        &action.query,
        &action.host,
    ] {
        if !v.is_empty() {
            return v.clone();
        }
    }
    String::new()
}

/// The URL path component, with a fallback to `uri.split("?",1)[0]` for a bare path.
fn split_path(uri: &str) -> String {
    if uri.is_empty() {
        return String::new();
    }
    let (path, _query) = urlsplit(uri);
    if path.is_empty() {
        uri.split_once('?').map_or(uri, |(p, _)| p).to_owned()
    } else {
        path
    }
}

/// The URL query component, with a fallback for a bare `?query`.
fn split_query(uri: &str) -> String {
    if uri.is_empty() {
        return String::new();
    }
    let (_path, query) = urlsplit(uri);
    if query.is_empty() {
        uri.split_once('?')
            .map_or(String::new(), |(_, q)| q.to_owned())
    } else {
        query
    }
}

/// Minimal URL splitter for request URIs: strips a `#fragment`, an
/// optional `scheme:` prefix and `//netloc`, then splits `path?query`. Exotic
/// forms (`;params`) are not modelled, but `request_state_from_session` reaches
/// here only when the walker left `http_path`/`http_query` empty.
fn urlsplit(url: &str) -> (String, String) {
    let url = url.split_once('#').map_or(url, |(a, _)| a);
    let mut rest = url;
    if let Some(colon) = rest.find(':') {
        let scheme = &rest[..colon];
        let is_scheme = !scheme.is_empty()
            && scheme
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.');
        if is_scheme {
            rest = &rest[colon + 1..];
        }
    }
    if let Some(after) = rest.strip_prefix("//") {
        let end = after.find(['/', '?']).unwrap_or(after.len());
        rest = &after[end..];
    }
    let (path, query) = rest.split_once('?').map_or((rest, ""), |(p, q)| (p, q));
    (path.to_owned(), query.to_owned())
}

/// Render a string in `repr()` form: single-quoted, with the standard
/// escapes, used in the unevaluable-condition notes.
fn py_repr(s: &str) -> String {
    let use_double = s.contains('\'') && !s.contains('"');
    let quote = if use_double { '"' } else { '\'' };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

#[cfg(test)]
mod tests {
    use super::{
        ActionTrace, BigipPolicyAction, RequestState, action_summary_value, apply_operator,
        evaluate_policy, split_path, split_query, urlsplit,
    };
    use crate::model::{BigipPolicy, BigipPolicyCondition, BigipPolicyRule};

    fn vals(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| (*s).to_string()).collect()
    }

    fn cond(
        operand: &str,
        selector: &str,
        operator: &str,
        values: &[&str],
    ) -> BigipPolicyCondition {
        BigipPolicyCondition {
            index: 0,
            operand: operand.into(),
            selector: selector.into(),
            operator: operator.into(),
            values: vals(values),
            name: String::new(),
            negate: false,
            case_insensitive: false,
            event: String::new(),
        }
    }

    fn fwd(pool: &str) -> BigipPolicyAction {
        BigipPolicyAction {
            index: 0,
            target: "forward".into(),
            verb: "select".into(),
            pool: pool.into(),
            location: String::new(),
            name: String::new(),
            value: String::new(),
            path: String::new(),
            query: String::new(),
            host: String::new(),
            event: String::new(),
        }
    }

    fn rule(
        name: &str,
        ordinal: i64,
        conditions: Vec<BigipPolicyCondition>,
        actions: Vec<BigipPolicyAction>,
    ) -> BigipPolicyRule {
        BigipPolicyRule {
            name: name.into(),
            ordinal,
            conditions,
            actions,
        }
    }

    fn policy(strategy: &str, rules: Vec<BigipPolicyRule>) -> BigipPolicy {
        BigipPolicy {
            name: "p".into(),
            full_path: "/Common/p".into(),
            strategy: strategy.into(),
            requires: Vec::new(),
            controls: Vec::new(),
            rules,
            description: String::new(),
            status: String::new(),
            last_modified: String::new(),
            range: None,
        }
    }

    #[test]
    fn apply_operator_covers_string_and_numeric_ops() {
        assert!(apply_operator(
            "equals",
            "GET",
            &vals(&["GET", "POST"]),
            false
        ));
        assert!(!apply_operator("equals", "PUT", &vals(&["GET"]), false));
        assert!(apply_operator(
            "starts-with",
            "/api/v1",
            &vals(&["/api"]),
            false
        ));
        assert!(apply_operator(
            "ends-with",
            "/login",
            &vals(&[".com", "/login"]),
            false
        ));
        assert!(apply_operator("contains", "abcdef", &vals(&["cde"]), false));
        // `matches` honours the case-insensitive flag (regex over the original).
        assert!(apply_operator("matches", "GET", &vals(&["^g.t$"]), true));
        assert!(!apply_operator("matches", "GET", &vals(&["^g.t$"]), false));
        // Numeric comparisons parse both sides as f64.
        assert!(apply_operator("greater", "443", &vals(&["80"]), false));
        assert!(apply_operator("less", "80", &vals(&["443"]), false));
        assert!(apply_operator(
            "greater-or-equal",
            "443",
            &vals(&["443"]),
            false
        ));
        assert!(apply_operator("less-or-equal", "80", &vals(&["80"]), false));
        // Edge cases: empty values, unknown operator, non-numeric operand → no match.
        assert!(!apply_operator("equals", "x", &[], false));
        assert!(!apply_operator("frobnicate", "x", &vals(&["x"]), false));
        assert!(!apply_operator("greater", "notnum", &vals(&["1"]), false));
    }

    #[test]
    fn apply_operator_case_insensitive_folds_both_sides() {
        assert!(apply_operator("equals", "GET", &vals(&["get"]), true));
        assert!(apply_operator("contains", "ABCDEF", &vals(&["cde"]), true));
        assert!(!apply_operator("equals", "GET", &vals(&["get"]), false));
    }

    #[test]
    fn first_match_fires_only_the_first_matching_rule() {
        let state = RequestState {
            method: "GET".into(),
            path: "/api/x".into(),
            ..Default::default()
        };
        let r1 = rule(
            "r1",
            0,
            vec![cond("http-method", "", "equals", &["GET"])],
            vec![fwd("/Common/pool1")],
        );
        let r2 = rule(
            "r2",
            1,
            vec![cond("http-uri", "path", "starts-with", &["/api"])],
            vec![fwd("/Common/pool2")],
        );
        let d = evaluate_policy(&policy("first-match", vec![r1, r2]), &state);
        assert_eq!(d.strategy, "first-match");
        assert!(d.rules[0].matched && d.rules[0].fired);
        // r2 matches too, but first-match means only r1 fires.
        assert!(d.rules[1].matched && !d.rules[1].fired);
        assert_eq!(d.rules[0].actions[0].value, "/Common/pool1");
        assert!(d.rules[1].actions.is_empty());
    }

    #[test]
    fn all_match_fires_every_matching_rule() {
        let state = RequestState {
            method: "GET".into(),
            path: "/api/x".into(),
            ..Default::default()
        };
        let r1 = rule(
            "r1",
            0,
            vec![cond("http-method", "", "equals", &["GET"])],
            vec![],
        );
        let r2 = rule(
            "r2",
            1,
            vec![cond("http-uri", "path", "starts-with", &["/api"])],
            vec![],
        );
        let d = evaluate_policy(&policy("all-match", vec![r1, r2]), &state);
        assert_eq!(d.strategy, "all-match");
        assert!(d.rules[0].fired && d.rules[1].fired);
    }

    #[test]
    fn best_match_prefers_the_rule_with_more_conditions() {
        let state = RequestState {
            method: "GET".into(),
            path: "/api/x".into(),
            ..Default::default()
        };
        let ra = rule(
            "a",
            0,
            vec![cond("http-method", "", "equals", &["GET"])],
            vec![],
        );
        let rb = rule(
            "b",
            1,
            vec![
                cond("http-method", "", "equals", &["GET"]),
                cond("http-uri", "path", "starts-with", &["/api"]),
            ],
            vec![],
        );
        // An unknown strategy name also reports as best-match-approx.
        let d = evaluate_policy(&policy("weighted", vec![ra, rb]), &state);
        assert_eq!(d.strategy, "best-match-approx");
        assert!(!d.rules[0].fired && d.rules[1].fired);
    }

    #[test]
    fn unconditional_rule_always_matches() {
        let r = rule("catchall", 0, vec![], vec![fwd("/Common/default")]);
        let d = evaluate_policy(&policy("first-match", vec![r]), &RequestState::default());
        assert!(d.rules[0].matched && d.rules[0].fired);
    }

    #[test]
    fn negate_inverts_a_condition() {
        let mut c = cond("http-method", "", "equals", &["POST"]);
        c.negate = true;
        let state = RequestState {
            method: "GET".into(),
            ..Default::default()
        };
        let d = evaluate_policy(
            &policy("first-match", vec![rule("r", 0, vec![c], vec![])]),
            &state,
        );
        // method is GET, "equals POST" is false, negated → matches.
        assert!(d.rules[0].matched);
    }

    #[test]
    fn exists_and_missing_test_header_presence() {
        let mut exists = cond("http-header", "", "exists", &[]);
        exists.name = "X-Foo".into();
        let mut missing = cond("http-header", "", "missing", &[]);
        missing.name = "X-Bar".into();
        let state = RequestState {
            headers: vec![("x-foo".into(), "1".into())],
            ..Default::default()
        };
        let d = evaluate_policy(
            &policy(
                "all-match",
                vec![
                    rule("has", 0, vec![exists], vec![]),
                    rule("lacks", 1, vec![missing], vec![]),
                ],
            ),
            &state,
        );
        assert!(d.rules[0].matched, "X-Foo exists");
        assert!(d.rules[1].matched, "X-Bar is missing");
    }

    #[test]
    fn unsupported_operand_is_unevaluable_and_noted() {
        let r = rule("r", 0, vec![cond("geoip", "", "equals", &["US"])], vec![]);
        let d = evaluate_policy(&policy("first-match", vec![r]), &RequestState::default());
        assert!(!d.rules[0].matched);
        assert!(
            d.rules[0].conditions[0]
                .note
                .contains("not supported in v1")
        );
    }

    #[test]
    fn action_summary_value_picks_the_salient_field_per_target() {
        let act = |f: &dyn Fn(&mut BigipPolicyAction)| {
            let mut a = BigipPolicyAction {
                index: 0,
                target: String::new(),
                verb: String::new(),
                pool: String::new(),
                location: String::new(),
                name: String::new(),
                value: String::new(),
                path: String::new(),
                query: String::new(),
                host: String::new(),
                event: String::new(),
            };
            f(&mut a);
            a
        };
        let fwd = act(&|a| {
            a.target = "forward".into();
            a.pool = "/Common/p".into();
        });
        assert_eq!(action_summary_value(&fwd), "/Common/p");
        let reply = act(&|a| {
            a.target = "http-reply".into();
            a.location = "https://x".into();
        });
        assert_eq!(action_summary_value(&reply), "https://x");
        let rewrite = act(&|a| {
            a.target = "http-uri".into();
            a.verb = "replace".into();
            a.path = "/new".into();
        });
        assert_eq!(action_summary_value(&rewrite), "path=/new");
        let insert = act(&|a| {
            a.target = "http-header".into();
            a.verb = "insert".into();
            a.name = "X".into();
            a.value = "1".into();
        });
        assert_eq!(action_summary_value(&insert), "X=1");
        let reset = act(&|a| {
            a.target = "tcp".into();
            a.verb = "reset".into();
        });
        assert_eq!(action_summary_value(&reset), "RST");
    }

    #[test]
    fn action_trace_is_returned_on_fired_rules() {
        let d = evaluate_policy(
            &policy(
                "first-match",
                vec![rule("r", 0, vec![], vec![fwd("/Common/pool")])],
            ),
            &RequestState::default(),
        );
        let ActionTrace { target, value, .. } = &d.rules[0].actions[0];
        assert_eq!(
            (target.as_str(), value.as_str()),
            ("forward", "/Common/pool")
        );
    }

    #[test]
    fn uri_splitting_matches_urlsplit_with_fallbacks() {
        assert_eq!(
            urlsplit("http://h/a/b?x=1"),
            ("/a/b".to_string(), "x=1".to_string())
        );
        assert_eq!(urlsplit("/p#frag"), ("/p".to_string(), String::new()));
        assert_eq!(split_path("/p?q=1"), "/p");
        assert_eq!(split_query("/p?q=1"), "q=1");
        assert_eq!(split_path(""), "");
        assert_eq!(split_query("/nopath"), "");
    }
}
