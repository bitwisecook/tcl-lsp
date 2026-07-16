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

//! Core iRule → F5 XC static translator.
//!
//! Walks the IR produced by [`lower_to_ir`](tcl_compiler::lowering::lower_to_ir)
//! and builds an [`XCTranslationResult`] mapping iRule constructs to
//! XC-native equivalents (routes, service policies, header actions, origin
//! pools).

use std::borrow::Cow;
use std::collections::BTreeMap;

use tcl_compiler::ir::{Module, Script, Statement, SwitchArm, SwitchMode, when_event_name};
use tcl_compiler::lowering::lower_to_ir_with_config;
use tcl_compiler::segmenter::segment_commands;
use tcl_dialect::DialectSet;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;
use tcl_syntax::expr::ast::{BinOp, ExprNode, UnaryOp, expr_text};
use tcl_syntax::expr::parse_expr;

use crate::mapping::{
    UNTRANSLATABLE_PREFIXES, advisory_event, command_xc_map, header_subcommand_map,
    translatable_event, untranslatable_event,
};
use crate::model::{
    TranslateStatus, TranslationItem, XCConstructKind, XCCookieMatch, XCDirectResponseAction,
    XCHeaderAction, XCHeaderMatch, XCHostMatch, XCMethodMatch, XCOriginPool, XCOriginPoolRef,
    XCPathMatch, XCQueryMatch, XCRedirectAction, XCRoute, XCServicePolicy, XCServicePolicyRule,
    XCTranslationResult, XCWafExclusionRule,
};

const MAX_DEPTH: u32 = 10;

const EQUALITY_OPS: [BinOp; 3] = [BinOp::StrEq, BinOp::Eq, BinOp::StrEquals];
const NEGATED_EQUALITY_OPS: [BinOp; 2] = [BinOp::StrNe, BinOp::Ne];

fn is_http_path_command(cmd: &str) -> bool {
    cmd == "HTTP::path" || cmd == "HTTP::uri"
}

// Enclosing context (threaded through from enclosing if/switch)

/// Match criteria accumulated from enclosing conditions.
#[derive(Debug, Clone, Default)]
struct EnclosingContext {
    path: Option<XCPathMatch>,
    host: Option<XCHostMatch>,
    method: Option<XCMethodMatch>,
    ip_prefix_set: Option<String>,
    header_matches: Vec<XCHeaderMatch>,
    cookie_matches: Vec<XCCookieMatch>,
    query_match: Option<XCQueryMatch>,
    ip_prefix_list: Vec<String>,
}

// Low-level helpers

/// Strip a leading `[` / trailing `]` from a command-substitution text and
/// trim surrounding whitespace.
fn strip_brackets(text: &str) -> &str {
    let t = text.trim();
    if t.starts_with('[') && t.ends_with(']') && t.len() >= 2 {
        t[1..t.len() - 1].trim()
    } else {
        t
    }
}

/// Extract the command name from a `[CMD ...]` substitution text (naive
/// whitespace split).
fn extract_command_name(text: &str) -> Option<&str> {
    strip_brackets(text).split_whitespace().next()
}

/// If *text* is a `[HTTP::xxx]` getter, return the command name. Also
/// recognises `[string tolower [HTTP::xxx]]` / `[string toupper [...]]` as
/// case-insensitive wrappers.
fn is_http_getter(text: &str) -> Option<String> {
    let cmd = extract_command_name(text)?;
    if cmd.starts_with("HTTP::") {
        return Some(cmd.to_owned());
    }
    if cmd == "string" {
        // Split into three whitespace fields: the leading `string`, the
        // subcommand, and the argument (which keeps any embedded
        // whitespace). Recurse when it is a `tolower` / `toupper` wrapper.
        let inner = strip_brackets(text);
        if let Some((subcommand, arg)) = string_subcommand_and_arg(inner)
            && (subcommand == "tolower" || subcommand == "toupper")
        {
            return is_http_getter(arg);
        }
    }
    None
}

/// For a `string SUBCMD ARG…` text, return `(subcommand, argument)` where
/// the argument retains any embedded whitespace. Returns `None` if fewer
/// than three whitespace fields are present.
fn string_subcommand_and_arg(s: &str) -> Option<(&str, &str)> {
    let (_string, rest) = split_first_word(s.trim_start())?;
    let (subcommand, rest) = split_first_word(rest.trim_start())?;
    let arg = rest.trim_start().trim_end();
    if arg.is_empty() {
        return None;
    }
    Some((subcommand, arg))
}

/// Split off the first whitespace-delimited word, returning `(word, rest)`.
fn split_first_word(s: &str) -> Option<(&str, &str)> {
    if s.is_empty() {
        return None;
    }
    match s.find(char::is_whitespace) {
        Some(i) => Some((&s[..i], &s[i..])),
        None => Some((s, "")),
    }
}

/// Split `[cmd arg1 "arg2"]` into argv parts (command name + args), using
/// the Tcl segmenter's word reconstruction. Returns an empty vec when the
/// text is not exactly one command.
fn parse_command_parts(text: &str) -> Vec<String> {
    let inner = strip_brackets(text);
    let cmds = segment_commands(inner);
    if cmds.len() != 1 {
        return Vec::new();
    }
    let cmd = &cmds[0];
    let mut parts = Vec::with_capacity(cmd.args().len() + 1);
    parts.push(cmd.name().to_owned());
    parts.extend(cmd.args().iter().cloned());
    parts
}

/// Classify what a switch subject switches on.
fn switch_subject_kind(subject: &str) -> Option<&'static str> {
    let cmd = is_http_getter(subject)?;
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
fn switch_path_match(pattern: &str, mode: SwitchMode, nocase: bool) -> XCPathMatch {
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
fn switch_host_match(pattern: &str, mode: SwitchMode, nocase: bool) -> XCHostMatch {
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
fn switch_method_match(pattern: &str) -> XCMethodMatch {
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
    let mut parts = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => parts.push_str(".*"),
            '?' => parts.push('.'),
            other => parts.push_str(&re_escape(&other.to_string())),
        }
    }
    parts.push('$');
    parts
}

/// Extract raw text from an `ExprNode::Command` node.
fn expr_command_text(node: &ExprNode) -> Option<&str> {
    match node {
        ExprNode::Command { text, .. } => Some(text),
        _ => None,
    }
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
        ExprNode::String { text, .. } => Some(strip_dquotes(text).to_owned()),
        ExprNode::Raw { text } => {
            let trimmed = text.trim();
            Some(strip_dquotes(trimmed).to_owned())
        }
        _ => None,
    }
}

/// Normalise an `if` / loop condition to the iRules-parsed expression.
///
/// The lowering parses structured-command conditions dialect-agnostically
/// (`parse_expr(arg, None)`), so an iRules-specific operator
/// (`starts_with` / `ends_with` / `contains` / `matches_glob`) leaves the
/// whole condition as an [`ExprNode::Raw`]. Conditions must be parsed under
/// the iRules grammar to recover their structure, so re-parse any `Raw`
/// fallback under `f5-irules`; an already-structured node is returned
/// untouched.
fn normalize_condition(node: &ExprNode) -> Cow<'_, ExprNode> {
    match node {
        ExprNode::Raw { text } => Cow::Owned(parse_expr(text, Some("f5-irules"))),
        other => Cow::Borrowed(other),
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
fn decompose_and_condition<'a>(node: &'a ExprNode, out: &mut Vec<&'a ExprNode>) {
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
fn decompose_or_condition<'a>(node: &'a ExprNode, out: &mut Vec<&'a ExprNode>) {
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

/// Helper: pull `(command_text, string_value)` out of a binary node where
/// either operand may be the command/string (the `left or right` pattern
/// for symmetric equality).
fn binary_cmd_and_str<'a>(
    left: &'a ExprNode,
    right: &'a ExprNode,
) -> (Option<&'a str>, Option<String>) {
    let cmd = expr_command_text(left).or_else(|| expr_command_text(right));
    let val = expr_string_value(right).or_else(|| expr_string_value(left));
    (cmd, val)
}

fn condition_to_method_match(node: &ExprNode) -> Option<XCMethodMatch> {
    let (inner, negated) = unwrap_negation(node);
    if let ExprNode::Binary { op, left, right } = inner {
        if EQUALITY_OPS.contains(op) {
            let (cmd, val) = binary_cmd_and_str(left, right);
            if let (Some(c), Some(v)) = (cmd, val) {
                if is_http_getter(c).as_deref() == Some("HTTP::method") {
                    return Some(XCMethodMatch {
                        methods: vec![v.to_uppercase()],
                        invert: negated,
                    });
                }
            }
        } else if NEGATED_EQUALITY_OPS.contains(op) {
            let (cmd, val) = binary_cmd_and_str(left, right);
            if let (Some(c), Some(v)) = (cmd, val) {
                if is_http_getter(c).as_deref() == Some("HTTP::method") {
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

fn condition_to_path_match(node: &ExprNode) -> Option<XCPathMatch> {
    let (inner, negated) = unwrap_negation(node);
    let ExprNode::Binary { op, left, right } = inner else {
        return None;
    };
    let getter_is_path = |n: &ExprNode| {
        expr_command_text(n)
            .and_then(is_http_getter)
            .is_some_and(|c| is_http_path_command(&c))
    };
    if EQUALITY_OPS.contains(op) {
        let (cmd, val) = binary_cmd_and_str(left, right);
        if let (Some(c), Some(v)) = (cmd, val) {
            if is_http_getter(c).is_some_and(|g| is_http_path_command(&g)) {
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
        BinOp::MatchesGlob if getter_is_path(left) => {
            expr_string_value(right).map(|v| pair("regex", glob_to_regex(&v)))
        }
        _ => None,
    }
}

fn condition_to_host_match(node: &ExprNode) -> Option<XCHostMatch> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    let getter_is_host = |n: &ExprNode| {
        expr_command_text(n).and_then(is_http_getter).as_deref() == Some("HTTP::host")
    };
    if EQUALITY_OPS.contains(op) {
        let (cmd, val) = binary_cmd_and_str(left, right);
        if let (Some(c), Some(v)) = (cmd, val) {
            if is_http_getter(c).as_deref() == Some("HTTP::host") {
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
fn parse_header_getter(node: &ExprNode) -> Option<String> {
    let ExprNode::Command { text, .. } = node else {
        return None;
    };
    let parts = parse_command_parts(text);
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
fn parse_header_exists(text: &str) -> Option<String> {
    let parts = parse_command_parts(text);
    if parts.len() >= 3 && parts[0] == "HTTP::header" && parts[1] == "exists" {
        Some(parts[2].clone())
    } else {
        None
    }
}

fn condition_to_header_match(node: &ExprNode) -> Option<XCHeaderMatch> {
    let (inner, negated) = unwrap_negation(node);
    match inner {
        ExprNode::Binary { op, left, right } if EQUALITY_OPS.contains(op) => {
            let hdr = parse_header_getter(left).or_else(|| parse_header_getter(right));
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
            let hdr = parse_header_getter(left).or_else(|| parse_header_getter(right));
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
            let hdr = parse_header_getter(left);
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
            let hdr = parse_header_getter(left);
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
        ExprNode::Command { text, .. } => {
            let exists_name = parse_header_exists(text)?;
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
fn parse_cookie_getter(node: &ExprNode) -> Option<String> {
    let ExprNode::Command { text, .. } = node else {
        return None;
    };
    let parts = parse_command_parts(text);
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

fn parse_cookie_exists(text: &str) -> Option<String> {
    let parts = parse_command_parts(text);
    if parts.len() >= 3 && parts[0] == "HTTP::cookie" && parts[1] == "exists" {
        Some(parts[2].clone())
    } else {
        None
    }
}

fn condition_to_cookie_match(node: &ExprNode) -> Option<XCCookieMatch> {
    let (inner, negated) = unwrap_negation(node);
    match inner {
        ExprNode::Binary { op, left, right } if EQUALITY_OPS.contains(op) => {
            let cookie = parse_cookie_getter(left).or_else(|| parse_cookie_getter(right));
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
            let cookie = parse_cookie_getter(left);
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
            let cookie = parse_cookie_getter(left);
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
        ExprNode::Command { text, .. } => {
            let exists_name = parse_cookie_exists(text)?;
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

fn condition_to_query_match(node: &ExprNode) -> Option<XCQueryMatch> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    let getter_is_query = |n: &ExprNode| {
        expr_command_text(n).and_then(is_http_getter).as_deref() == Some("HTTP::query")
    };
    if EQUALITY_OPS.contains(op) {
        let (cmd, val) = binary_cmd_and_str(left, right);
        if let (Some(c), Some(v)) = (cmd, val) {
            if is_http_getter(c).as_deref() == Some("HTTP::query") {
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

fn condition_to_direct_ip(node: &ExprNode) -> Option<String> {
    let ExprNode::Binary { op, left, right } = node else {
        return None;
    };
    if !EQUALITY_OPS.contains(op) {
        return None;
    }
    let (cmd, val) = binary_cmd_and_str(left, right);
    let cmd_text = cmd?;
    if extract_command_name(cmd_text) == Some("IP::client_addr") {
        return val;
    }
    None
}

fn condition_to_ip_prefix_set(node: &ExprNode) -> Option<String> {
    let ExprNode::Command { text, .. } = node else {
        return None;
    };
    let inner = strip_brackets(text);
    let parts: Vec<&str> = inner.split_whitespace().collect();
    if parts.len() >= 5
        && parts[0] == "class"
        && parts[1] == "match"
        && inner.contains("IP::client_addr")
    {
        return parts.last().map(|s| (*s).to_owned());
    }
    None
}

fn condition_to_class_match_method(node: &ExprNode) -> Option<XCMethodMatch> {
    let ExprNode::Command { text, .. } = node else {
        return None;
    };
    let parts = parse_command_parts(text);
    if parts.len() >= 5
        && parts[0] == "class"
        && parts[1] == "match"
        && text.contains("HTTP::method")
    {
        // Data-group name is last part — cannot resolve statically. Return a
        // placeholder match indicating method matching.
        return Some(XCMethodMatch {
            methods: Vec::new(),
            invert: false,
        });
    }
    None
}

/// Extract all recognisable match criteria from a list of AND
/// sub-conditions.
fn extract_all_matches(subconditions: &[&ExprNode]) -> EnclosingContext {
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
            path = condition_to_path_match(sub);
            if path.is_some() {
                continue;
            }
        }
        if host.is_none() {
            host = condition_to_host_match(sub);
            if host.is_some() {
                continue;
            }
        }
        if method.is_none() {
            method = condition_to_method_match(sub);
            if method.is_some() {
                continue;
            }
        }
        if method.is_none() {
            method = condition_to_class_match_method(sub);
            if method.is_some() {
                continue;
            }
        }
        if ip_prefix_set.is_none() {
            ip_prefix_set = condition_to_ip_prefix_set(sub);
            if ip_prefix_set.is_some() {
                continue;
            }
        }
        if let Some(hdr) = condition_to_header_match(sub) {
            headers.push(hdr);
            continue;
        }
        if let Some(cookie) = condition_to_cookie_match(sub) {
            cookies.push(cookie);
            continue;
        }
        if query.is_none() {
            query = condition_to_query_match(sub);
            if query.is_some() {
                continue;
            }
        }
        if let Some(ip) = condition_to_direct_ip(sub) {
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
fn merge_enclosing(parent: &EnclosingContext, child: &EnclosingContext) -> EnclosingContext {
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

fn enclosing_has_matches(enc: &EnclosingContext) -> bool {
    enc.path.is_some()
        || enc.host.is_some()
        || enc.method.is_some()
        || enc.ip_prefix_set.is_some()
        || !enc.header_matches.is_empty()
        || !enc.cookie_matches.is_empty()
        || enc.query_match.is_some()
        || !enc.ip_prefix_list.is_empty()
}

// Translation state

/// Mutable accumulator for translation results.
struct TranslationContext {
    routes: Vec<XCRoute>,
    policy_rules: Vec<XCServicePolicyRule>,
    origin_pools: BTreeMap<String, XCOriginPool>,
    origin_pool_order: Vec<String>,
    header_actions: Vec<XCHeaderAction>,
    waf_exclusion_rules: Vec<XCWafExclusionRule>,
    items: Vec<TranslationItem>,
    rule_counter: u32,
    current_event: String,
}

impl TranslationContext {
    fn new() -> Self {
        Self {
            routes: Vec::new(),
            policy_rules: Vec::new(),
            origin_pools: BTreeMap::new(),
            origin_pool_order: Vec::new(),
            header_actions: Vec::new(),
            waf_exclusion_rules: Vec::new(),
            items: Vec::new(),
            rule_counter: 0,
            current_event: String::new(),
        }
    }

    fn next_rule_name(&mut self) -> String {
        self.rule_counter += 1;
        format!("rule-{}", self.rule_counter)
    }

    fn add_origin_pool(&mut self, name: &str, source_range: Option<Span>) {
        if !self.origin_pools.contains_key(name) {
            self.origin_pools.insert(
                name.to_owned(),
                XCOriginPool {
                    name: name.to_owned(),
                    port: 80,
                },
            );
            self.origin_pool_order.push(name.to_owned());
        }
        self.items.push(TranslationItem {
            status: TranslateStatus::Translated,
            kind: XCConstructKind::OriginPool,
            irule_command: format!("pool {name}"),
            irule_range: source_range,
            xc_description: format!("Origin pool: {name}"),
            note: String::new(),
            diagnostic_code: "XC100".to_owned(),
        });
    }
}

// IR walking

fn walk_script(
    script: &Script,
    ctx: &mut TranslationContext,
    registry: &CommandRegistry,
    depth: u32,
    enclosing: &EnclosingContext,
) {
    for stmt in &script.statements {
        walk_statement(stmt, ctx, registry, depth, enclosing);
    }
}

// One flat arm per statement kind in the IR walk; splitting obscures the dispatch.
#[allow(clippy::too_many_lines)]
fn walk_statement(
    stmt: &Statement,
    ctx: &mut TranslationContext,
    registry: &CommandRegistry,
    depth: u32,
    enclosing: &EnclosingContext,
) {
    if depth > MAX_DEPTH {
        return;
    }

    match stmt {
        Statement::Switch {
            subject,
            arms,
            default_body,
            span,
            mode,
            nocase,
            ..
        } => walk_switch(
            subject,
            arms,
            default_body.as_ref(),
            *mode,
            *nocase,
            *span,
            ctx,
            registry,
            depth,
            enclosing,
        ),
        Statement::If {
            clauses,
            else_body,
            span,
            ..
        } => {
            let mut any_matched = false;
            for clause in clauses {
                let cond = normalize_condition(&clause.condition);
                let mut or_branches: Vec<&ExprNode> = Vec::new();
                decompose_or_condition(cond.as_ref(), &mut or_branches);
                if or_branches.len() > 1 {
                    for branch in or_branches {
                        let mut and_subs: Vec<&ExprNode> = Vec::new();
                        decompose_and_condition(branch, &mut and_subs);
                        let matches = extract_all_matches(&and_subs);
                        let merged = merge_enclosing(enclosing, &matches);
                        if enclosing_has_matches(&matches) {
                            any_matched = true;
                        }
                        walk_script(&clause.body, ctx, registry, depth + 1, &merged);
                    }
                } else {
                    let mut and_subs: Vec<&ExprNode> = Vec::new();
                    decompose_and_condition(cond.as_ref(), &mut and_subs);
                    let matches = extract_all_matches(&and_subs);
                    let merged = merge_enclosing(enclosing, &matches);
                    if enclosing_has_matches(&matches) {
                        any_matched = true;
                    }
                    walk_script(&clause.body, ctx, registry, depth + 1, &merged);
                }
            }
            if let Some(eb) = else_body {
                // The `else` runs when every clause condition is false, but the
                // outer (enclosing) criteria still hold — mirror `walk_switch`'s
                // default-body handling, which keeps `enclosing` and only clears
                // its own key. Preserve `enclosing` so an `else`-body action does
                // not become an unconditional catch-all route (RUST_ISSUE_044).
                // Any criterion the `if` clauses matched on is not part of
                // `enclosing`, so it is already excluded here.
                walk_script(eb, ctx, registry, depth + 1, enclosing);
            }
            if any_matched {
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Translated,
                    kind: XCConstructKind::ServicePolicyRule,
                    irule_command: "if condition".to_owned(),
                    irule_range: Some(*span),
                    xc_description: "XC service policy rule with match criteria".to_owned(),
                    note: String::new(),
                    diagnostic_code: "XC102".to_owned(),
                });
            } else {
                let cond_text = clauses.first().map_or_else(
                    || "?".to_owned(),
                    |c| expr_text(normalize_condition(&c.condition).as_ref()),
                );
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Partial,
                    kind: XCConstructKind::ServicePolicyRule,
                    irule_command: format!("if {{{cond_text}}}"),
                    irule_range: Some(*span),
                    xc_description: "Conditional logic — review manually for XC match criteria"
                        .to_owned(),
                    note: "Complex condition could not be automatically mapped".to_owned(),
                    diagnostic_code: "XC203".to_owned(),
                });
            }
        }
        Statement::Call {
            command,
            args,
            span,
            ..
        } => walk_call(command, args, *span, ctx, registry, enclosing),
        Statement::Barrier { command, span, .. } => {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Untranslatable,
                kind: XCConstructKind::Note,
                irule_command: command.clone(),
                irule_range: Some(*span),
                xc_description: "Dynamic construct — cannot be statically translated".to_owned(),
                note: format!(
                    "'{command}' creates dynamic behaviour with no XC equivalent. \
                     Consider App Stack for this logic."
                ),
                diagnostic_code: "XC300".to_owned(),
            });
        }
        Statement::Return { .. } => {}
        Statement::For { body, span, .. }
        | Statement::While { body, span, .. }
        | Statement::Foreach { body, span, .. } => {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Untranslatable,
                kind: XCConstructKind::Note,
                irule_command: "loop".to_owned(),
                irule_range: Some(*span),
                xc_description: "Loop construct — no XC equivalent".to_owned(),
                note: "XC policies are declarative, not procedural. \
                       Consider App Stack for iterative logic."
                    .to_owned(),
                diagnostic_code: "XC300".to_owned(),
            });
            walk_script(body, ctx, registry, depth + 1, &EnclosingContext::default());
        }
        Statement::Catch { body, .. } | Statement::Try { body, .. } => {
            walk_script(body, ctx, registry, depth + 1, &EnclosingContext::default());
        }
        // Assignments and other statements (Incr, ExprEval, Block, UpFrame)
        // carry no XC construct — ignore.
        _ => {}
    }
}

/// Which criterion a recognised `switch` subject constrains.
#[derive(Debug, Clone, Copy)]
enum SwitchTarget {
    Path,
    Host,
    Method,
}

/// The pattern-matching semantics of a recognised `switch`, derived from its
/// subject and its IR-carried `mode`/`nocase`.
#[derive(Debug, Clone, Copy)]
struct SwitchArmSpec {
    target: SwitchTarget,
    mode: SwitchMode,
    nocase: bool,
}

/// Layer one arm's `pattern` (translated per the switch's mode/nocase) onto
/// `base`, replacing only the target criterion.
fn arm_context(base: &EnclosingContext, spec: SwitchArmSpec, pattern: &str) -> EnclosingContext {
    let mut enc = base.clone();
    match spec.target {
        SwitchTarget::Path => enc.path = Some(switch_path_match(pattern, spec.mode, spec.nocase)),
        SwitchTarget::Host => enc.host = Some(switch_host_match(pattern, spec.mode, spec.nocase)),
        SwitchTarget::Method => enc.method = Some(switch_method_match(pattern)),
    }
    enc
}

/// Walk the arms (and default body) of a recognised `switch`.
///
/// Fall-through arms (`"/a*" -`) carry `body: None`; their pattern must be
/// attached to the following non-fall-through arm's body so that every
/// pattern routes to an action (`RUST_ISSUE_047`). Each arm's pattern is
/// translated honouring the switch mode/nocase (`RUST_ISSUE_046`), and the
/// default body keeps `enclosing` with only the switch's own key cleared.
fn walk_switch_arms(
    spec: SwitchArmSpec,
    arms: &[SwitchArm],
    default_body: Option<&Script>,
    ctx: &mut TranslationContext,
    registry: &CommandRegistry,
    depth: u32,
    enclosing: &EnclosingContext,
) {
    // Patterns of preceding fall-through arms awaiting a shared body.
    let mut pending: Vec<&str> = Vec::new();
    for arm in arms {
        match arm.body.as_ref() {
            // Fall-through arm: defer its pattern to the next arm with a body.
            None => pending.push(&arm.pattern),
            Some(body) => {
                pending.push(&arm.pattern);
                for pattern in pending.drain(..) {
                    let enc = arm_context(enclosing, spec, pattern);
                    walk_script(body, ctx, registry, depth + 1, &enc);
                }
            }
        }
    }
    if let Some(db) = default_body {
        // Fall-through patterns immediately preceding `default`
        // (`"/old" - default { … }`) share the default arm's body: lowering
        // peels `default` into `default_body` while the preceding
        // fall-through arm stays in `arms` with `body: None`, so `pending`
        // still holds those patterns here. Tcl runs the default body for each
        // such pattern too, so emit a criterion-specific route for every
        // pending pattern before the catch-all route with the key cleared
        // (`RUST_ISSUE_047`).
        for pattern in pending.drain(..) {
            let enc = arm_context(enclosing, spec, pattern);
            walk_script(db, ctx, registry, depth + 1, &enc);
        }
        let mut enc = enclosing.clone();
        match spec.target {
            SwitchTarget::Path => enc.path = None,
            SwitchTarget::Host => enc.host = None,
            SwitchTarget::Method => enc.method = None,
        }
        walk_script(db, ctx, registry, depth + 1, &enc);
    }
}

// Threads the full translation context (registry, depth, enclosing scope, sink)
// positionally through the recursive IR walk; a struct would just re-wrap it.
#[allow(clippy::too_many_arguments)]
fn walk_switch(
    subject: &str,
    arms: &[SwitchArm],
    default_body: Option<&Script>,
    mode: SwitchMode,
    nocase: bool,
    span: Span,
    ctx: &mut TranslationContext,
    registry: &CommandRegistry,
    depth: u32,
    enclosing: &EnclosingContext,
) {
    let kind = switch_subject_kind(subject);
    match kind {
        Some("path") => {
            let spec = SwitchArmSpec {
                target: SwitchTarget::Path,
                mode,
                nocase,
            };
            walk_switch_arms(spec, arms, default_body, ctx, registry, depth, enclosing);
            ctx.items.push(translated_route_item(
                format!("switch {subject}"),
                span,
                "XC L7 routes with path matching",
                "XC101",
            ));
        }
        Some("host") => {
            let spec = SwitchArmSpec {
                target: SwitchTarget::Host,
                mode,
                nocase,
            };
            walk_switch_arms(spec, arms, default_body, ctx, registry, depth, enclosing);
            ctx.items.push(translated_route_item(
                format!("switch {subject}"),
                span,
                "XC L7 routes with host matching",
                "XC101",
            ));
        }
        Some("method") => {
            let spec = SwitchArmSpec {
                target: SwitchTarget::Method,
                mode,
                nocase,
            };
            walk_switch_arms(spec, arms, default_body, ctx, registry, depth, enclosing);
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Translated,
                kind: XCConstructKind::ServicePolicyRule,
                irule_command: format!("switch {subject}"),
                irule_range: Some(span),
                xc_description: "XC service policy rules with method matching".to_owned(),
                note: String::new(),
                diagnostic_code: "XC102".to_owned(),
            });
        }
        _ => {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Partial,
                kind: XCConstructKind::Route,
                irule_command: format!("switch {subject}"),
                irule_range: Some(span),
                xc_description: "Switch on dynamic value — needs manual XC config".to_owned(),
                note: "Cannot statically determine match criteria".to_owned(),
                diagnostic_code: "XC200".to_owned(),
            });
            // The subject is dynamic, so no arm pattern can be mapped to a
            // criterion — but the enclosing criteria still apply, so preserve
            // them rather than dropping to a catch-all (RUST_ISSUE_044).
            for arm in arms {
                if let Some(body) = arm.body.as_ref() {
                    walk_script(body, ctx, registry, depth + 1, enclosing);
                }
            }
            if let Some(db) = default_body {
                walk_script(db, ctx, registry, depth + 1, enclosing);
            }
        }
    }
}

fn translated_route_item(
    irule_command: String,
    span: Span,
    xc_description: &str,
    code: &str,
) -> TranslationItem {
    TranslationItem {
        status: TranslateStatus::Translated,
        kind: XCConstructKind::Route,
        irule_command,
        irule_range: Some(span),
        xc_description: xc_description.to_owned(),
        note: String::new(),
        diagnostic_code: code.to_owned(),
    }
}

/// Parse a base-10 integer the way `int()` does for these args:
/// trim surrounding whitespace, accept an optional sign. Returns `None` on
/// failure, in which case the caller keeps its default.
fn py_int(arg: Option<&String>) -> Option<i64> {
    arg.and_then(|s| s.trim().parse::<i64>().ok())
}

// One flat arm per translated command name; splitting obscures the dispatch.
#[allow(clippy::too_many_lines)]
fn walk_call(
    command: &str,
    args: &[String],
    span: Span,
    ctx: &mut TranslationContext,
    registry: &CommandRegistry,
    enclosing: &EnclosingContext,
) {
    match command {
        "pool" if !args.is_empty() => {
            let pool_name = args[0].clone();
            ctx.add_origin_pool(&pool_name, Some(span));
            ctx.routes.push(XCRoute {
                path_match: enclosing.path.clone(),
                host_match: enclosing.host.clone(),
                method_match: enclosing.method.clone(),
                header_matches: enclosing.header_matches.clone(),
                query_match: enclosing.query_match.clone(),
                cookie_matches: enclosing.cookie_matches.clone(),
                origin_pool: Some(XCOriginPoolRef {
                    name: pool_name,
                    source_range: Some(span),
                }),
                source_range: Some(span),
                ..XCRoute::default()
            });
        }
        "HTTP::redirect" if !args.is_empty() => {
            let url = args[0].clone();
            let code = py_int(args.get(1)).unwrap_or(302);
            ctx.routes.push(XCRoute {
                path_match: enclosing.path.clone(),
                host_match: enclosing.host.clone(),
                method_match: enclosing.method.clone(),
                header_matches: enclosing.header_matches.clone(),
                query_match: enclosing.query_match.clone(),
                cookie_matches: enclosing.cookie_matches.clone(),
                redirect: Some(XCRedirectAction {
                    url: url.clone(),
                    response_code: code,
                    source_range: Some(span),
                }),
                source_range: Some(span),
                ..XCRoute::default()
            });
            ctx.items.push(translated_route_item(
                format!("HTTP::redirect {url}"),
                span,
                "XC redirect route",
                "XC101",
            ));
        }
        "HTTP::respond" if !args.is_empty() => {
            let status_code = py_int(args.first()).unwrap_or(200);
            let mut body = String::new();
            for (i, arg) in args.iter().enumerate() {
                if arg == "content" && i + 1 < args.len() {
                    body.clone_from(&args[i + 1]);
                    break;
                }
            }
            if matches!(status_code, 403 | 401 | 429) {
                let name = ctx.next_rule_name();
                ctx.policy_rules.push(XCServicePolicyRule {
                    name,
                    action: "deny".to_owned(),
                    path_match: enclosing.path.clone(),
                    host_match: enclosing.host.clone(),
                    method_match: enclosing.method.clone(),
                    header_matches: enclosing.header_matches.clone(),
                    query_match: enclosing.query_match.clone(),
                    cookie_matches: enclosing.cookie_matches.clone(),
                    ip_prefix_list: enclosing.ip_prefix_list.clone(),
                    description: format!("Deny with {status_code} (from HTTP::respond)"),
                    source_range: Some(span),
                });
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Translated,
                    kind: XCConstructKind::ServicePolicyRule,
                    irule_command: format!("HTTP::respond {status_code}"),
                    irule_range: Some(span),
                    xc_description: format!("XC service policy deny rule ({status_code})"),
                    note: String::new(),
                    diagnostic_code: "XC102".to_owned(),
                });
            } else {
                ctx.routes.push(XCRoute {
                    path_match: enclosing.path.clone(),
                    host_match: enclosing.host.clone(),
                    method_match: enclosing.method.clone(),
                    header_matches: enclosing.header_matches.clone(),
                    query_match: enclosing.query_match.clone(),
                    cookie_matches: enclosing.cookie_matches.clone(),
                    direct_response: Some(XCDirectResponseAction {
                        status_code,
                        body,
                        source_range: Some(span),
                    }),
                    source_range: Some(span),
                    ..XCRoute::default()
                });
                ctx.items.push(translated_route_item(
                    format!("HTTP::respond {status_code}"),
                    span,
                    &format!("XC direct response route ({status_code})"),
                    "XC101",
                ));
            }
        }
        "HTTP::header" if !args.is_empty() => {
            let subcommand = args[0].to_lowercase();
            if let Some(operation) = header_subcommand_map(&subcommand) {
                if args.len() >= 2 {
                    let header_name = args[1].clone();
                    let header_value = args.get(2).cloned().unwrap_or_default();
                    let target = if ctx.current_event == "HTTP_RESPONSE" {
                        "response"
                    } else {
                        "request"
                    };
                    ctx.header_actions.push(XCHeaderAction {
                        name: header_name.clone(),
                        value: header_value,
                        operation: operation.to_owned(),
                        target: target.to_owned(),
                        source_range: Some(span),
                    });
                    ctx.items.push(TranslationItem {
                        status: TranslateStatus::Translated,
                        kind: XCConstructKind::HeaderAction,
                        irule_command: format!("HTTP::header {subcommand} {header_name}"),
                        irule_range: Some(span),
                        xc_description: format!("XC {target} header {operation}: {header_name}"),
                        note: String::new(),
                        diagnostic_code: "XC103".to_owned(),
                    });
                }
            }
        }
        "class" if !args.is_empty() && args[0] == "match" => {
            let joined = args[1..args.len().min(3)].join(" ");
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Translated,
                kind: XCConstructKind::ServicePolicyRule,
                irule_command: format!("class match {joined}"),
                irule_range: Some(span),
                xc_description: "XC service policy rule (data-group matching)".to_owned(),
                note: "Each data-group entry may need a separate XC rule".to_owned(),
                diagnostic_code: "XC105".to_owned(),
            });
        }
        "ASM::disable" => {
            let name = ctx.next_rule_name();
            ctx.waf_exclusion_rules.push(XCWafExclusionRule {
                name,
                path_match: enclosing.path.clone(),
                ip_prefix_set: enclosing.ip_prefix_set.clone(),
                description: "Disable WAF (from ASM::disable)".to_owned(),
                source_range: Some(span),
            });
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Translated,
                kind: XCConstructKind::WafExclusion,
                irule_command: "ASM::disable".to_owned(),
                irule_range: Some(span),
                xc_description: "XC WAF exclusion rule (disable App Firewall)".to_owned(),
                note: String::new(),
                diagnostic_code: "XC106".to_owned(),
            });
        }
        "ASM::enable" => {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Translated,
                kind: XCConstructKind::Note,
                irule_command: "ASM::enable".to_owned(),
                irule_range: Some(span),
                xc_description: "No action needed — XC App Firewall is enabled by default"
                    .to_owned(),
                note: String::new(),
                diagnostic_code: "XC107".to_owned(),
            });
        }
        _ => {
            // Other commands: registry-driven translatability.
            if registry.is_xc_never_translatable(command) {
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Untranslatable,
                    kind: XCConstructKind::Note,
                    irule_command: command.to_owned(),
                    irule_range: Some(span),
                    xc_description: format!("'{command}' has no XC equivalent"),
                    note: "Consider App Stack for this logic.".to_owned(),
                    diagnostic_code: "XC300".to_owned(),
                });
            } else if UNTRANSLATABLE_PREFIXES
                .iter()
                .any(|p| command.starts_with(p))
            {
                if !registry.is_xc_translatable_override(command) {
                    ctx.items.push(TranslationItem {
                        status: TranslateStatus::Untranslatable,
                        kind: XCConstructKind::Note,
                        irule_command: command.to_owned(),
                        irule_range: Some(span),
                        xc_description: format!("'{command}' has no XC equivalent"),
                        note: "L4/protocol-specific command. Consider App Stack.".to_owned(),
                        diagnostic_code: "XC301".to_owned(),
                    });
                }
            } else if let Some(mapping) = command_xc_map(command) {
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Translated,
                    kind: mapping.kind,
                    irule_command: command.to_owned(),
                    irule_range: Some(span),
                    xc_description: mapping.xc_description.to_owned(),
                    note: mapping.note.to_owned(),
                    diagnostic_code: "XC100".to_owned(),
                });
            }
        }
    }
}

// Public API

/// Build a [`CommandRegistry`] with the iRules dialect loaded, suitable for
/// lowering and translating an iRule.
fn irules_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::build_default();
    registry.load_dialect(DialectSet::IRULES);
    registry
}

/// Translate an iRule to F5 XC configuration. Analyses the iRule source,
/// walks the IR, and returns an [`XCTranslationResult`] with routes,
/// service policies, origin pools, header actions, and a coverage report.
#[must_use]
pub fn translate_irule(source: &str) -> XCTranslationResult {
    let registry = irules_registry();
    translate_irule_with_registry(source, &registry)
}

/// [`translate_irule`] against a caller-provided registry (must have the
/// iRules dialect loaded).
#[must_use]
pub fn translate_irule_with_registry(
    source: &str,
    registry: &CommandRegistry,
) -> XCTranslationResult {
    // Lower under the iRules dialect so the iRules expr operators
    // (`starts_with` / `ends_with` / `contains` / `matches_glob` / …) parse
    // as the corresponding `BinOp`s rather than falling back to `Raw`.
    let config = tcl_lexer::LexerConfig::for_dialect("f5-irules");
    let module: Module = lower_to_ir_with_config(source, registry, config);

    let mut ctx = TranslationContext::new();

    // Walk each event handler (procedures keyed `::when::EVENT`), in a
    // deterministic order (BTreeMap-sorted qualified name) so the item order
    // is stable. The resulting item set is order-independent for
    // diagnostics, and the differential harness compares as a multiset.
    let mut event_procs: Vec<(&String, &tcl_compiler::ir::Procedure)> = module
        .procedures
        .iter()
        .filter(|(k, _)| k.starts_with("::when::"))
        .collect();
    event_procs.sort_by(|a, b| a.0.cmp(b.0));

    let mut regular_procs: Vec<(&String, &tcl_compiler::ir::Procedure)> = module
        .procedures
        .iter()
        .filter(|(k, _)| !k.starts_with("::when::"))
        .collect();
    regular_procs.sort_by(|a, b| a.0.cmp(b.0));

    for (qualified_name, proc) in event_procs {
        let event_name = when_event_name(qualified_name).to_owned();
        ctx.current_event.clone_from(&event_name);

        if translatable_event(&event_name).is_some() {
            walk_script(
                &proc.body,
                &mut ctx,
                registry,
                0,
                &EnclosingContext::default(),
            );
        } else if let Some(desc) = untranslatable_event(&event_name) {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Untranslatable,
                kind: XCConstructKind::Note,
                irule_command: format!("when {event_name}"),
                irule_range: Some(proc.span),
                xc_description: desc.to_owned(),
                note: "This entire event handler has no XC equivalent.".to_owned(),
                diagnostic_code: "XC201".to_owned(),
            });
        } else if let Some(desc) = advisory_event(&event_name) {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Advisory,
                kind: XCConstructKind::Note,
                irule_command: format!("when {event_name}"),
                irule_range: Some(proc.span),
                xc_description: desc.to_owned(),
                note: "This event maps to a separate XC feature.".to_owned(),
                diagnostic_code: "XC250".to_owned(),
            });
        } else {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Untranslatable,
                kind: XCConstructKind::Note,
                irule_command: format!("when {event_name}"),
                irule_range: Some(proc.span),
                xc_description: format!("Event '{event_name}' has no known XC equivalent."),
                note: String::new(),
                diagnostic_code: "XC201".to_owned(),
            });
        }
    }

    // Also walk regular procs that may be called from event handlers.
    for (_, proc) in regular_procs {
        ctx.current_event = String::new();
        walk_script(
            &proc.body,
            &mut ctx,
            registry,
            0,
            &EnclosingContext::default(),
        );
    }

    // Build service policy from collected rules.
    let service_policies = if ctx.policy_rules.is_empty() {
        Vec::new()
    } else {
        vec![XCServicePolicy {
            name: "translated-policy".to_owned(),
            algo: "FIRST_MATCH".to_owned(),
            rules: std::mem::take(&mut ctx.policy_rules),
        }]
    };

    // Coverage.
    let total = ctx.items.len();
    let coverage = if total == 0 {
        100.0
    } else {
        let translated = ctx
            .items
            .iter()
            .filter(|i| {
                matches!(
                    i.status,
                    TranslateStatus::Translated | TranslateStatus::Advisory
                )
            })
            .count();
        let partial = ctx
            .items
            .iter()
            .filter(|i| i.status == TranslateStatus::Partial)
            .count();
        // Small item counts; the f64 coverage percentage is display-only.
        #[allow(clippy::cast_precision_loss)]
        let cov = ((translated as f64 + partial as f64 * 0.5) / total as f64) * 100.0;
        cov
    };

    let origin_pools: Vec<XCOriginPool> = ctx
        .origin_pool_order
        .iter()
        .filter_map(|n| ctx.origin_pools.get(n).cloned())
        .collect();

    XCTranslationResult {
        routes: ctx.routes,
        service_policies,
        origin_pools,
        header_actions: ctx.header_actions,
        waf_exclusion_rules: ctx.waf_exclusion_rules,
        items: ctx.items,
        coverage_pct: round_1dp(coverage),
    }
}

/// Round to one decimal place using round-half-to-even (banker's rounding).
fn round_1dp(x: f64) -> f64 {
    let scaled = x * 10.0;
    let rounded = scaled.round_ties_even();
    rounded / 10.0
}
