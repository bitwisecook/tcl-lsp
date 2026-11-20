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

//! XC match criteria read from `switch` arm patterns and from `if` conditions.

use super::source_advice::SourceExpressionAdvice;
use super::{EQUALITY_OPS, EnclosingContext, NEGATED_EQUALITY_OPS, is_http_path_command};
use crate::model::{
    XCCookieMatch, XCHeaderMatch, XCHostMatch, XCMethodMatch, XCPathMatch, XCQueryMatch,
};
use tcl_compiler::ir::SwitchMode;
use tcl_syntax::expr::ast::{BinOp, ExprNode, UnaryOp};

/// Classify what a switch subject switches on.
pub(super) fn switch_subject_kind(command: Option<String>) -> Option<&'static str> {
    let cmd = command?;
    if is_http_path_command(&cmd) {
        Some("path")
    } else if cmd == "HTTP::host" {
        Some("host")
    } else if cmd == "HTTP::method" {
        Some("method")
    } else if cmd == "HTTP::header" {
        Some("header")
    } else {
        None
    }
}

/// Convert a glob pattern like `/api/*` to an XC path match. A single
/// trailing `*` (with no other wildcard) becomes a `prefix` match; any
/// other glob wildcard becomes a `regex`; a wildcard-free pattern is
/// `exact`.
fn glob_to_path_match(pattern: &str) -> XCPathMatch {
    let no_q = !pattern.contains('?');
    let star_only_at_end = pattern.ends_with('*') && !pattern[..pattern.len() - 1].contains('*');
    if star_only_at_end && no_q {
        let prefix = &pattern[..pattern.len() - 1];
        if !prefix.is_empty() {
            return XCPathMatch {
                match_type: "prefix".to_owned(),
                value: prefix.to_owned(),
                invert: false,
            };
        }
    }
    if pattern.contains(['*', '?']) {
        return XCPathMatch {
            match_type: "regex".to_owned(),
            value: glob_to_regex(pattern),
            invert: false,
        };
    }
    XCPathMatch {
        match_type: "exact".to_owned(),
        value: pattern.to_owned(),
        invert: false,
    }
}

/// Prefix a regex value with the case-insensitive flag when `nocase` is set.
fn apply_nocase(regex: String, nocase: bool) -> String {
    if nocase {
        format!("(?i){regex}")
    } else {
        regex
    }
}

/// Anchored regex that matches `pattern` literally.
fn literal_to_regex(pattern: &str) -> String {
    format!("^{}$", re_escape(pattern))
}

/// Translate one `switch` arm pattern to an XC path match, honouring the
/// switch's [`SwitchMode`] and `nocase` flag.
pub(super) fn switch_path_match(pattern: &str, mode: SwitchMode, nocase: bool) -> XCPathMatch {
    match mode {
        SwitchMode::Regexp => XCPathMatch {
            match_type: "regex".to_owned(),
            value: apply_nocase(pattern.to_owned(), nocase),
            invert: false,
        },
        SwitchMode::Glob if nocase => XCPathMatch {
            match_type: "regex".to_owned(),
            value: apply_nocase(glob_to_regex(pattern), nocase),
            invert: false,
        },
        SwitchMode::Glob => glob_to_path_match(pattern),
        SwitchMode::Exact if nocase => XCPathMatch {
            match_type: "regex".to_owned(),
            value: apply_nocase(literal_to_regex(pattern), nocase),
            invert: false,
        },
        SwitchMode::Exact => XCPathMatch {
            match_type: "exact".to_owned(),
            value: pattern.to_owned(),
            invert: false,
        },
    }
}

/// Translate one `switch` arm pattern to an XC host match, honouring the
/// switch's [`SwitchMode`] and `nocase` flag. The host model supports only
/// `exact` and `regex`, so glob patterns are lowered to regex.
pub(super) fn switch_host_match(pattern: &str, mode: SwitchMode, nocase: bool) -> XCHostMatch {
    match mode {
        SwitchMode::Regexp => XCHostMatch {
            match_type: "regex".to_owned(),
            value: apply_nocase(pattern.to_owned(), nocase),
        },
        SwitchMode::Glob => XCHostMatch {
            match_type: "regex".to_owned(),
            value: apply_nocase(glob_to_regex(pattern), nocase),
        },
        SwitchMode::Exact if nocase => XCHostMatch {
            match_type: "regex".to_owned(),
            value: apply_nocase(literal_to_regex(pattern), nocase),
        },
        SwitchMode::Exact => XCHostMatch {
            match_type: "exact".to_owned(),
            value: pattern.to_owned(),
        },
    }
}

/// Translate one `switch` arm pattern to an XC method match. HTTP methods
/// are an enumerated, case-insensitive token set, so the XC method match
/// cannot express glob/regex patterns; the pattern is upper-cased and
/// matched as a literal method regardless of `mode`/`nocase`.
pub(super) fn switch_method_match(pattern: &str) -> XCMethodMatch {
    XCMethodMatch {
        methods: vec![pattern.to_uppercase()],
        invert: false,
    }
}

/// Escape only the ASCII regex metacharacters (and ASCII whitespace),
/// so emitted regex values are well-formed.
fn re_escape(s: &str) -> String {
    const SPECIAL: &[char] = &[
        '(', ')', '[', ']', '{', '}', '?', '*', '+', '-', '|', '^', '$', '\\', '.', '&', '~', '#',
        ' ', '\t', '\n', '\r', '\x0b', '\x0c',
    ];
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if SPECIAL.contains(&ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// Convert a glob pattern to a regex string.
fn glob_to_regex(pattern: &str) -> String {
    tcl_syntax::glob::glob_to_re(pattern)
}

/// Strip surrounding double quotes from `text` (only `"…"`; braced strings
/// are left untouched).
fn strip_dquotes(text: &str) -> &str {
    let bytes = text.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        &text[1..text.len() - 1]
    } else {
        text
    }
}

/// Extract the string value from a literal or string node (handles
/// `ExprString` and `ExprRaw`).
fn expr_string_value(node: &ExprNode) -> Option<String> {
    match node {
        // Only a fixed operand is literal text: `"/api$x"` substitutes, and
        // a braced one's value excludes its braces.
        ExprNode::String { text, .. } => {
            tcl_syntax::expr::fixed_string_operand(text).map(str::to_owned)
        }
        ExprNode::Raw { text } => {
            let trimmed = text.trim();
            Some(strip_dquotes(trimmed).to_owned())
        }
        _ => None,
    }
}

/// Unwrap a `!` / `not` prefix, returning `(inner, is_negated)`.
fn unwrap_negation(node: &ExprNode) -> (&ExprNode, bool) {
    if let ExprNode::Unary { op, operand } = node
        && matches!(op, UnaryOp::Not | UnaryOp::WordNot)
    {
        return (operand, true);
    }
    (node, false)
}

/// Flatten nested `&&` / `and` conditions.
pub(super) fn decompose_and_condition<'a>(node: &'a ExprNode, out: &mut Vec<&'a ExprNode>) {
    if let ExprNode::Binary { op, left, right } = node {
        if matches!(op, BinOp::And | BinOp::WordAnd) {
            decompose_and_condition(left, out);
            decompose_and_condition(right, out);
            return;
        }
    }
    out.push(node);
}

/// Flatten nested `||` / `or` conditions.
pub(super) fn decompose_or_condition<'a>(node: &'a ExprNode, out: &mut Vec<&'a ExprNode>) {
    if let ExprNode::Binary { op, left, right } = node {
        if matches!(op, BinOp::Or | BinOp::WordOr) {
            decompose_or_condition(left, out);
            decompose_or_condition(right, out);
            return;
        }
    }
    out.push(node);
}

// Condition extraction helpers

/// Pull the original command node and a literal value from either operand.
fn binary_cmd_and_str<'a>(
    left: &'a ExprNode,
    right: &'a ExprNode,
) -> (Option<&'a ExprNode>, Option<String>) {
    let cmd = [left, right]
        .into_iter()
        .find(|node| matches!(node, ExprNode::Command { .. }));
    let val = expr_string_value(right).or_else(|| expr_string_value(left));
    (cmd, val)
}

fn condition_to_method_match(
    node: &ExprNode,
    advice: &SourceExpressionAdvice<'_>,
) -> Option<XCMethodMatch> {
    let (inner, negated) = unwrap_negation(node);
    if let ExprNode::Binary { op, left, right } = inner {
        if EQUALITY_OPS.contains(op) {
            let (cmd, val) = binary_cmd_and_str(left, right);
            if let (Some(c), Some(v)) = (cmd, val) {
                if advice.http_getter(c).as_deref() == Some("HTTP::method") {
                    return Some(XCMethodMatch {
                        methods: vec![v.to_uppercase()],
                        invert: negated,
                    });
                }
            }
        } else if NEGATED_EQUALITY_OPS.contains(op) {
            let (cmd, val) = binary_cmd_and_str(left, right);
            if let (Some(c), Some(v)) = (cmd, val) {
                if advice.http_getter(c).as_deref() == Some("HTTP::method") {
                    return Some(XCMethodMatch {
                        methods: vec![v.to_uppercase()],
                        invert: !negated,
                    });
                }
            }
        }
    }
    None
}

pub(super) fn condition_to_path_match(
    node: &ExprNode,
    advice: &SourceExpressionAdvice<'_>,
) -> Option<XCPathMatch> {
    let (inner, negated) = unwrap_negation(node);
    let ExprNode::Binary { op, left, right } = inner else {
        return None;
    };
    let getter_is_path = |n: &ExprNode| {
        advice
            .http_getter(n)
            .is_some_and(|c| is_http_path_command(&c))
    };
    if EQUALITY_OPS.contains(op) {
        let (cmd, val) = binary_cmd_and_str(left, right);
        if let (Some(c), Some(v)) = (cmd, val) {
            if advice
                .http_getter(c)
                .is_some_and(|g| is_http_path_command(&g))
            {
                return Some(XCPathMatch {
                    match_type: "exact".to_owned(),
                    value: v,
                    invert: negated,
                });
            }
        }
        return None;
    }
    let pair = |mt: &str, value: String| XCPathMatch {
        match_type: mt.to_owned(),
        value,
        invert: negated,
    };
    match op {
        BinOp::StartsWith if getter_is_path(left) => {
            expr_string_value(right).map(|v| pair("prefix", v))
        }
        BinOp::EndsWith if getter_is_path(left) => {
            expr_string_value(right).map(|v| pair("suffix", v))
        }
        BinOp::Contains if getter_is_path(left) => {
            expr_string_value(right).map(|v| pair("regex", re_escape(&v)))
        }
        BinOp::MatchesRegex if getter_is_path(left) => {
            expr_string_value(right).map(|v| pair("regex", v))
        }
        BinOp::Matches | BinOp::MatchesGlob if getter_is_path(left) => {
            expr_string_value(right).map(|v| pair("regex", glob_to_regex(&v)))
        }
        _ => None,
    }
}

fn condition_to_host_match(
    node: &ExprNode,
    advice: &SourceExpressionAdvice<'_>,
) -> Option<XCHostMatch> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    let getter_is_host = |n: &ExprNode| advice.http_getter(n).as_deref() == Some("HTTP::host");
    if EQUALITY_OPS.contains(op) {
        let (cmd, val) = binary_cmd_and_str(left, right);
        if let (Some(c), Some(v)) = (cmd, val) {
            if advice.http_getter(c).as_deref() == Some("HTTP::host") {
                return Some(XCHostMatch {
                    match_type: "exact".to_owned(),
                    value: v,
                });
            }
        }
        return None;
    }
    match op {
        BinOp::MatchesRegex if getter_is_host(left) => {
            expr_string_value(right).map(|v| XCHostMatch {
                match_type: "regex".to_owned(),
                value: v,
            })
        }
        BinOp::Contains if getter_is_host(left) => expr_string_value(right).map(|v| XCHostMatch {
            match_type: "regex".to_owned(),
            value: re_escape(&v),
        }),
        _ => None,
    }
}

/// Parse `[HTTP::header value "name"]` or `[HTTP::header "name"]` → header
/// name.
fn parse_header_getter(node: &ExprNode, advice: &SourceExpressionAdvice<'_>) -> Option<String> {
    let parts = advice.parts(node)?;
    if parts.len() < 2 || parts[0] != "HTTP::header" {
        return None;
    }
    if parts[1] == "value" && parts.len() >= 3 {
        return Some(parts[2].clone());
    }
    if !matches!(
        parts[1].as_str(),
        "insert" | "replace" | "remove" | "set" | "exists" | "values"
    ) {
        return Some(parts[1].clone());
    }
    None
}

/// Parse `[HTTP::header exists "name"]` → header name.
fn parse_header_exists(node: &ExprNode, advice: &SourceExpressionAdvice<'_>) -> Option<String> {
    let parts = advice.parts(node)?;
    if parts.len() >= 3 && parts[0] == "HTTP::header" && parts[1] == "exists" {
        Some(parts[2].clone())
    } else {
        None
    }
}

fn condition_to_header_match(
    node: &ExprNode,
    advice: &SourceExpressionAdvice<'_>,
) -> Option<XCHeaderMatch> {
    let (inner, negated) = unwrap_negation(node);
    match inner {
        ExprNode::Binary { op, left, right } if EQUALITY_OPS.contains(op) => {
            let hdr =
                parse_header_getter(left, advice).or_else(|| parse_header_getter(right, advice));
            let val = expr_string_value(right).or_else(|| expr_string_value(left));
            if let (Some(h), Some(v)) = (hdr, val) {
                return Some(XCHeaderMatch {
                    name: h,
                    match_type: "exact".to_owned(),
                    value: v,
                    invert: negated,
                });
            }
            None
        }
        ExprNode::Binary { op, left, right } if NEGATED_EQUALITY_OPS.contains(op) => {
            let hdr =
                parse_header_getter(left, advice).or_else(|| parse_header_getter(right, advice));
            let val = expr_string_value(right).or_else(|| expr_string_value(left));
            if let (Some(h), Some(v)) = (hdr, val) {
                return Some(XCHeaderMatch {
                    name: h,
                    match_type: "exact".to_owned(),
                    value: v,
                    invert: !negated,
                });
            }
            None
        }
        ExprNode::Binary {
            op: BinOp::Contains,
            left,
            right,
        } => {
            let hdr = parse_header_getter(left, advice);
            let val = expr_string_value(right);
            if let (Some(h), Some(v)) = (hdr, val) {
                return Some(XCHeaderMatch {
                    name: h,
                    match_type: "regex".to_owned(),
                    value: re_escape(&v),
                    invert: negated,
                });
            }
            None
        }
        ExprNode::Binary {
            op: BinOp::MatchesRegex,
            left,
            right,
        } => {
            let hdr = parse_header_getter(left, advice);
            let val = expr_string_value(right);
            if let (Some(h), Some(v)) = (hdr, val) {
                return Some(XCHeaderMatch {
                    name: h,
                    match_type: "regex".to_owned(),
                    value: v,
                    invert: negated,
                });
            }
            None
        }
        ExprNode::Command { .. } => {
            let exists_name = parse_header_exists(inner, advice)?;
            Some(XCHeaderMatch {
                name: exists_name,
                match_type: "presence".to_owned(),
                value: String::new(),
                invert: negated,
            })
        }
        _ => None,
    }
}

/// Parse `[HTTP::cookie "name"]` / `[HTTP::cookie value "name"]` → cookie
/// name.
fn parse_cookie_getter(node: &ExprNode, advice: &SourceExpressionAdvice<'_>) -> Option<String> {
    let parts = advice.parts(node)?;
    if parts.len() < 2 || parts[0] != "HTTP::cookie" {
        return None;
    }
    if parts[1] == "value" && parts.len() >= 3 {
        return Some(parts[2].clone());
    }
    if !matches!(parts[1].as_str(), "insert" | "remove" | "exists" | "names") {
        return Some(parts[1].clone());
    }
    None
}

fn parse_cookie_exists(node: &ExprNode, advice: &SourceExpressionAdvice<'_>) -> Option<String> {
    let parts = advice.parts(node)?;
    if parts.len() >= 3 && parts[0] == "HTTP::cookie" && parts[1] == "exists" {
        Some(parts[2].clone())
    } else {
        None
    }
}

fn condition_to_cookie_match(
    node: &ExprNode,
    advice: &SourceExpressionAdvice<'_>,
) -> Option<XCCookieMatch> {
    let (inner, negated) = unwrap_negation(node);
    match inner {
        ExprNode::Binary { op, left, right } if EQUALITY_OPS.contains(op) => {
            let cookie =
                parse_cookie_getter(left, advice).or_else(|| parse_cookie_getter(right, advice));
            let val = expr_string_value(right).or_else(|| expr_string_value(left));
            if let (Some(c), Some(v)) = (cookie, val) {
                return Some(XCCookieMatch {
                    name: c,
                    match_type: "exact".to_owned(),
                    value: v,
                    invert: negated,
                });
            }
            None
        }
        ExprNode::Binary {
            op: BinOp::Contains,
            left,
            right,
        } => {
            let cookie = parse_cookie_getter(left, advice);
            let val = expr_string_value(right);
            if let (Some(c), Some(v)) = (cookie, val) {
                return Some(XCCookieMatch {
                    name: c,
                    match_type: "regex".to_owned(),
                    value: re_escape(&v),
                    invert: negated,
                });
            }
            None
        }
        ExprNode::Binary {
            op: BinOp::MatchesRegex,
            left,
            right,
        } => {
            let cookie = parse_cookie_getter(left, advice);
            let val = expr_string_value(right);
            if let (Some(c), Some(v)) = (cookie, val) {
                return Some(XCCookieMatch {
                    name: c,
                    match_type: "regex".to_owned(),
                    value: v,
                    invert: negated,
                });
            }
            None
        }
        ExprNode::Command { .. } => {
            let exists_name = parse_cookie_exists(inner, advice)?;
            Some(XCCookieMatch {
                name: exists_name,
                match_type: "presence".to_owned(),
                value: String::new(),
                invert: negated,
            })
        }
        _ => None,
    }
}

fn condition_to_query_match(
    node: &ExprNode,
    advice: &SourceExpressionAdvice<'_>,
) -> Option<XCQueryMatch> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    let getter_is_query = |n: &ExprNode| advice.http_getter(n).as_deref() == Some("HTTP::query");
    if EQUALITY_OPS.contains(op) {
        let (cmd, val) = binary_cmd_and_str(left, right);
        if let (Some(c), Some(v)) = (cmd, val) {
            if advice.http_getter(c).as_deref() == Some("HTTP::query") {
                return Some(XCQueryMatch {
                    match_type: "exact".to_owned(),
                    value: v,
                });
            }
        }
        return None;
    }
    match op {
        BinOp::Contains if getter_is_query(left) => {
            expr_string_value(right).map(|v| XCQueryMatch {
                match_type: "regex".to_owned(),
                value: re_escape(&v),
            })
        }
        BinOp::MatchesRegex if getter_is_query(left) => {
            expr_string_value(right).map(|v| XCQueryMatch {
                match_type: "regex".to_owned(),
                value: v,
            })
        }
        _ => None,
    }
}

fn condition_to_direct_ip(node: &ExprNode, advice: &SourceExpressionAdvice<'_>) -> Option<String> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    if !EQUALITY_OPS.contains(op) {
        return None;
    }
    let (cmd, val) = binary_cmd_and_str(left, right);
    let cmd_text = cmd?;
    if advice.scalar(cmd_text).as_deref() == Some("IP::client_addr") {
        return val;
    }
    None
}

pub(super) fn condition_to_ip_prefix_set(
    node: &ExprNode,
    advice: &SourceExpressionAdvice<'_>,
) -> Option<String> {
    let selected = advice.command(node)?;
    if selected.command() != "class"
        || selected.arguments().len() != 4
        || selected.arguments()[0].literal_bytes()? != b"match"
        || advice
            .scalar_operand(selected.original_argument_word(1)?)
            .as_deref()
            != Some("IP::client_addr")
    {
        return None;
    }
    std::str::from_utf8(selected.arguments()[3].literal_bytes()?)
        .ok()
        .map(str::to_owned)
}

pub(super) fn condition_to_class_match_method(
    node: &ExprNode,
    advice: &SourceExpressionAdvice<'_>,
) -> Option<XCMethodMatch> {
    let selected = advice.command(node)?;
    if selected.command() != "class"
        || selected.arguments().len() != 4
        || selected.arguments()[0].literal_bytes()? != b"match"
        || advice
            .scalar_operand(selected.original_argument_word(1)?)
            .as_deref()
            != Some("HTTP::method")
    {
        return None;
    }
    Some(XCMethodMatch {
        methods: Vec::new(),
        invert: false,
    })
}

/// Extract all recognisable match criteria from a list of AND
/// sub-conditions.
pub(super) fn extract_all_matches(
    subconditions: &[&ExprNode],
    advice: &SourceExpressionAdvice<'_>,
) -> EnclosingContext {
    let mut path: Option<XCPathMatch> = None;
    let mut host: Option<XCHostMatch> = None;
    let mut method: Option<XCMethodMatch> = None;
    let mut ip_prefix_set: Option<String> = None;
    let mut headers: Vec<XCHeaderMatch> = Vec::new();
    let mut cookies: Vec<XCCookieMatch> = Vec::new();
    let mut query: Option<XCQueryMatch> = None;
    let mut ips: Vec<String> = Vec::new();

    for sub in subconditions {
        if path.is_none() {
            path = condition_to_path_match(sub, advice);
            if path.is_some() {
                continue;
            }
        }
        if host.is_none() {
            host = condition_to_host_match(sub, advice);
            if host.is_some() {
                continue;
            }
        }
        if method.is_none() {
            method = condition_to_method_match(sub, advice);
            if method.is_some() {
                continue;
            }
        }
        if method.is_none() {
            method = condition_to_class_match_method(sub, advice);
            if method.is_some() {
                continue;
            }
        }
        if ip_prefix_set.is_none() {
            ip_prefix_set = condition_to_ip_prefix_set(sub, advice);
            if ip_prefix_set.is_some() {
                continue;
            }
        }
        if let Some(hdr) = condition_to_header_match(sub, advice) {
            headers.push(hdr);
            continue;
        }
        if let Some(cookie) = condition_to_cookie_match(sub, advice) {
            cookies.push(cookie);
            continue;
        }
        if query.is_none() {
            query = condition_to_query_match(sub, advice);
            if query.is_some() {
                continue;
            }
        }
        if let Some(ip) = condition_to_direct_ip(sub, advice) {
            ips.push(ip);
        }
    }

    EnclosingContext {
        path,
        host,
        method,
        ip_prefix_set,
        header_matches: headers,
        cookie_matches: cookies,
        query_match: query,
        ip_prefix_list: ips,
    }
}

/// Merge child matches into parent — child wins on non-None fields.
pub(super) fn merge_enclosing(
    parent: &EnclosingContext,
    child: &EnclosingContext,
) -> EnclosingContext {
    EnclosingContext {
        path: child.path.clone().or_else(|| parent.path.clone()),
        host: child.host.clone().or_else(|| parent.host.clone()),
        method: child.method.clone().or_else(|| parent.method.clone()),
        ip_prefix_set: child
            .ip_prefix_set
            .clone()
            .or_else(|| parent.ip_prefix_set.clone()),
        header_matches: if child.header_matches.is_empty() {
            parent.header_matches.clone()
        } else {
            child.header_matches.clone()
        },
        cookie_matches: if child.cookie_matches.is_empty() {
            parent.cookie_matches.clone()
        } else {
            child.cookie_matches.clone()
        },
        query_match: child
            .query_match
            .clone()
            .or_else(|| parent.query_match.clone()),
        ip_prefix_list: if child.ip_prefix_list.is_empty() {
            parent.ip_prefix_list.clone()
        } else {
            child.ip_prefix_list.clone()
        },
    }
}

pub(super) fn enclosing_has_matches(enc: &EnclosingContext) -> bool {
    enc.path.is_some()
        || enc.host.is_some()
        || enc.method.is_some()
        || enc.ip_prefix_set.is_some()
        || !enc.header_matches.is_empty()
        || !enc.cookie_matches.is_empty()
        || enc.query_match.is_some()
        || !enc.ip_prefix_list.is_empty()
}
