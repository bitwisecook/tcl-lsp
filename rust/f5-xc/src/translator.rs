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

use std::collections::BTreeMap;
use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::ir::{Module, Script, Statement, SwitchArm, SwitchMode};
use tcl_compiler::registry_invocation::source_structure::{
    OriginalRegistryWords, original_single_source_command_substitution,
};
use tcl_core_types::DiagCode;
#[cfg(test)]
use tcl_lexer::SourceImage;
use tcl_lexer::{NativeWord, Span, native_script_words_in};
use tcl_registry::CommandRegistry;
use tcl_syntax::expr::ast::{BinOp, ExprNode, UnaryOp, expr_text};
#[cfg(test)]
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

/// Original words from one complete source command under the translation grammar.
#[cfg(test)]
fn original_command_words(text: &str) -> Option<Vec<NativeWord>> {
    let config = tcl_lexer::LexerConfig::from_grammar(
        tcl_registry::model::resolve_environment("f5-irules").grammar(),
    );
    let image = SourceImage::document(text);
    let region = Span::new(0, u32::try_from(text.len()).ok()?);
    let mut plan = native_script_words_in(image.clone(), region, config).ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
        return None;
    }
    let words = plan.commands.pop()?.words;
    if words.iter().any(|word| word.group().expand) {
        return None;
    }
    if words.len() == 1 && text.trim_start().starts_with('[') {
        return Some(
            original_single_source_command_substitution(&words[0])?
                .command()
                .words
                .clone(),
        );
    }
    Some(words)
}

#[cfg(test)]
fn original_literal_value(word: &NativeWord) -> Option<String> {
    String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
        word,
    )?)
    .ok()
}

/// Scalar getter advice requires a complete zero-argument command or a sole
/// executable getter operand of a case conversion. Literal brackets are data.
#[cfg(test)]
fn is_http_getter(text: &str) -> Option<String> {
    scalar_getter(text).filter(|name| name.starts_with("HTTP::"))
}

#[cfg(test)]
fn scalar_getter(text: &str) -> Option<String> {
    scalar_getter_words(original_command_words(text)?)
}

#[cfg(test)]
fn scalar_getter_words(mut words: Vec<NativeWord>) -> Option<String> {
    loop {
        let command = original_literal_value(words.first()?)?;
        if words.len() == 1 {
            return Some(command);
        }
        if command != "string" || words.len() != 3 {
            return None;
        }
        let subcommand = original_literal_value(&words[1])?;
        if !matches!(subcommand.as_str(), "tolower" | "toupper") {
            return None;
        }
        words = original_single_source_command_substitution(&words[2])?
            .command()
            .words
            .clone();
    }
}

#[cfg(test)]
fn scalar_getter_operand(word: &NativeWord) -> Option<String> {
    scalar_getter_words(
        original_single_source_command_substitution(word)?
            .command()
            .words
            .clone(),
    )
}

/// Literal fields and sole executable command operands retain shared geometry.
/// Dynamic values and expanded words cannot supply static translation facts.
#[cfg(test)]
fn parse_command_parts(text: &str) -> Vec<String> {
    let Some(words) = original_command_words(text) else {
        return Vec::new();
    };
    let parts: Option<Vec<String>> = words
        .iter()
        .map(|word| {
            original_literal_value(word).or_else(|| {
                original_single_source_command_substitution(word)?;
                Some(
                    word.image()
                        .try_text()
                        .ok()?
                        .get(word.span().as_range())?
                        .to_owned(),
                )
            })
        })
        .collect();
    parts.unwrap_or_default()
}

/// Classify what a switch subject switches on.
fn switch_subject_kind(command: Option<String>) -> Option<&'static str> {
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

fn condition_to_path_match(
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
        BinOp::MatchesGlob if getter_is_path(left) => {
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

fn condition_to_ip_prefix_set(
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

fn condition_to_class_match_method(
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
fn extract_all_matches(
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
struct TranslationContext<'a> {
    source: &'a str,
    registry: &'a CommandRegistry,
    analysis: &'a AnalysisResult,
    module: &'a Module,
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

impl<'a> TranslationContext<'a> {
    fn new(
        source: &'a str,
        analysis: &'a AnalysisResult,
        module: &'a Module,
        registry: &'a CommandRegistry,
    ) -> Self {
        Self {
            source,
            registry,
            analysis,
            module,
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
            diagnostic_code: DiagCode::Xc100,
        });
    }
}

fn source_xc_advice_allowed(words: &OriginalRegistryWords, analysis: &AnalysisResult) -> bool {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    words
        .with_source_schema(&input.context_registry(), |schema| {
            let descriptor = schema.authored_source_descriptors().command;
            matches!(
                tcl_registry::registry::spec_pack_of(descriptor),
                Some("irules" | "tcl")
            ) && descriptor.xc_translatable != Some(false)
        })
        .is_some_and(|allowed| allowed)
}

struct SourceExpressionAdvice<'a> {
    source: &'a str,
    analysis: &'a AnalysisResult,
    module: &'a Module,
    registry: &'a CommandRegistry,
    base: Option<u32>,
}

impl<'a> SourceExpressionAdvice<'a> {
    fn new(context: &TranslationContext<'a>, base: Option<u32>) -> Self {
        Self {
            source: context.source,
            analysis: context.analysis,
            module: context.module,
            registry: context.registry,
            base,
        }
    }

    fn original_operand(&self, span: Span) -> Option<NativeWord> {
        let mut plan =
            native_script_words_in(self.module.source.clone(), span, self.module.lexer_config)
                .ok()?;
        if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
            return None;
        }
        let mut words = plan.commands.pop()?.words;
        if words.len() != 1 || words[0].span() != span || words[0].group().expand {
            return None;
        }
        words.pop()
    }

    fn expression_operand(&self, node: &ExprNode) -> Option<NativeWord> {
        let ExprNode::Command { text, start, end } = node else {
            return None;
        };
        let base = self.base?;
        if start > end {
            return None;
        }
        let span = Span::new(base.checked_add(*start)?, base.checked_add(*end)?);
        (self.source.get(span.as_range())? == text).then_some(())?;
        self.original_operand(span)
    }

    fn command_for_operand(&self, operand: &NativeWord) -> Option<OriginalRegistryWords> {
        let selected = tcl_compiler::source_graph::registry_words_for_original_substitution(
            self.source,
            self.analysis,
            self.module,
            operand,
            self.registry,
        )?;
        source_xc_advice_allowed(&selected, self.analysis).then_some(selected)
    }

    fn command(&self, node: &ExprNode) -> Option<OriginalRegistryWords> {
        self.command_for_operand(&self.expression_operand(node)?)
    }

    fn parts(&self, node: &ExprNode) -> Option<Vec<String>> {
        let selected = self.command(node)?;
        let mut parts = vec![selected.command().to_owned()];
        for argument in selected.arguments() {
            parts.push(
                std::str::from_utf8(argument.literal_bytes()?)
                    .ok()?
                    .to_owned(),
            );
        }
        Some(parts)
    }

    fn scalar_operand(&self, operand: &NativeWord) -> Option<String> {
        let mut selected = self.command_for_operand(operand)?;
        loop {
            if selected.arguments().is_empty() {
                return Some(selected.command().to_owned());
            }
            if selected.command() != "string" || selected.arguments().len() != 2 {
                return None;
            }
            let subcommand = selected.arguments()[0].literal_bytes()?;
            if !matches!(subcommand, b"tolower" | b"toupper") {
                return None;
            }
            selected = self.command_for_operand(selected.original_argument_word(1)?)?;
        }
    }

    fn scalar(&self, node: &ExprNode) -> Option<String> {
        self.scalar_operand(&self.expression_operand(node)?)
    }

    fn command_owners_available(&self, node: &ExprNode) -> bool {
        let mut nodes = vec![node];
        while let Some(node) = nodes.pop() {
            match node {
                ExprNode::Command { .. } => {
                    let Some(operand) = self.expression_operand(node) else {
                        return false;
                    };
                    let mut operands = vec![operand];
                    while let Some(operand) = operands.pop() {
                        if self.command_for_operand(&operand).is_none() {
                            return false;
                        }
                        let Some(original) = original_single_source_command_substitution(&operand)
                        else {
                            return false;
                        };
                        for word in &original.command().words {
                            for part in word.executable_parts().all_parts() {
                                if matches!(part.part, tcl_lexer::ExecutablePart::Command { .. }) {
                                    let Some(child) = self.original_operand(part.span) else {
                                        return false;
                                    };
                                    operands.push(child);
                                }
                            }
                        }
                    }
                }
                ExprNode::Binary { left, right, .. } => {
                    nodes.extend([left.as_ref(), right.as_ref()]);
                }
                ExprNode::Unary { operand, .. } => nodes.push(operand),
                ExprNode::Ternary {
                    condition,
                    true_branch,
                    false_branch,
                } => nodes.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]),
                ExprNode::Call { args, .. } => nodes.extend(args),
                _ => {}
            }
        }
        true
    }

    fn http_getter(&self, node: &ExprNode) -> Option<String> {
        self.scalar(node)
            .filter(|command| command.starts_with("HTTP::"))
    }
}

// IR walking

fn walk_script(
    script: &Script,
    ctx: &mut TranslationContext<'_>,
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
    ctx: &mut TranslationContext<'_>,
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
            subject_span,
            subject_braced,
            arms,
            default_body,
            span,
            mode,
            nocase,
            ..
        } => walk_switch(
            switch_subject_kind(
                (!subject_braced)
                    .then(|| {
                        let advice = SourceExpressionAdvice::new(ctx, None);
                        advice.scalar_operand(&advice.original_operand(*subject_span)?)
                    })
                    .flatten(),
            ),
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
            if clauses.iter().any(|clause| {
                !SourceExpressionAdvice::new(ctx, clause.condition_base)
                    .command_owners_available(&clause.condition)
            }) {
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Untranslatable,
                    kind: XCConstructKind::Note,
                    irule_command: "if".to_owned(),
                    irule_range: Some(*span),
                    xc_description: "Condition commands have no current original source owner"
                        .to_owned(),
                    note: String::new(),
                    diagnostic_code: DiagCode::Xc203,
                });
                return;
            }
            let mut any_matched = false;
            for clause in clauses {
                let cond = &clause.condition;
                let advice = SourceExpressionAdvice::new(ctx, clause.condition_base);
                let mut or_branches: Vec<&ExprNode> = Vec::new();
                decompose_or_condition(cond, &mut or_branches);
                if or_branches.len() > 1 {
                    for branch in or_branches {
                        let mut and_subs: Vec<&ExprNode> = Vec::new();
                        decompose_and_condition(branch, &mut and_subs);
                        let matches = extract_all_matches(&and_subs, &advice);
                        let merged = merge_enclosing(enclosing, &matches);
                        if enclosing_has_matches(&matches) {
                            any_matched = true;
                        }
                        walk_script(&clause.body, ctx, registry, depth + 1, &merged);
                    }
                } else {
                    let mut and_subs: Vec<&ExprNode> = Vec::new();
                    decompose_and_condition(cond, &mut and_subs);
                    let matches = extract_all_matches(&and_subs, &advice);
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
                // not become an unconditional catch-all route.
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
                    diagnostic_code: DiagCode::Xc102,
                });
            } else {
                let cond_text = clauses
                    .first()
                    .map_or_else(|| "?".to_owned(), |c| expr_text(&c.condition));
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Partial,
                    kind: XCConstructKind::ServicePolicyRule,
                    irule_command: format!("if {{{cond_text}}}"),
                    irule_range: Some(*span),
                    xc_description: "Conditional logic — review manually for XC match criteria"
                        .to_owned(),
                    note: "Complex condition could not be automatically mapped".to_owned(),
                    diagnostic_code: DiagCode::Xc203,
                });
            }
        }
        Statement::Call {
            command,
            span,
            tokens,
            ..
        } => {
            let selected = tokens.as_ref().and_then(|tokens| {
                tcl_compiler::source_graph::registry_words_for_original_call(
                    ctx.source,
                    ctx.analysis,
                    ctx.module,
                    tokens,
                    registry,
                )
            });
            let static_arguments: Option<Vec<String>> = selected.as_ref().and_then(|words| {
                words
                    .arguments()
                    .iter()
                    .map(|argument| {
                        std::str::from_utf8(argument.literal_bytes()?)
                            .ok()
                            .map(str::to_owned)
                    })
                    .collect()
            });
            if let (Some(selected), Some(arguments)) = (selected, static_arguments)
                && source_xc_advice_allowed(&selected, ctx.analysis)
            {
                walk_call(
                    selected.command(),
                    &arguments,
                    *span,
                    ctx,
                    registry,
                    enclosing,
                );
            } else {
                ctx.items.push(TranslationItem {
                    status: TranslateStatus::Untranslatable,
                    kind: XCConstructKind::Note,
                    irule_command: command.clone(),
                    irule_range: Some(*span),
                    xc_description: "Command or operands have no current static source owner"
                        .to_owned(),
                    note: String::new(),
                    diagnostic_code: DiagCode::Xc203,
                });
            }
        }
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
                diagnostic_code: DiagCode::Xc300,
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
                diagnostic_code: DiagCode::Xc300,
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
/// pattern routes to an action. Each arm's pattern is
/// translated honouring the switch mode/nocase, and the
/// default body keeps `enclosing` with only the switch's own key cleared.
fn walk_switch_arms(
    spec: SwitchArmSpec,
    arms: &[SwitchArm],
    default_body: Option<&Script>,
    ctx: &mut TranslationContext<'_>,
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
        // pending pattern before the catch-all route with the key cleared.
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
    kind: Option<&'static str>,
    subject: &str,
    arms: &[SwitchArm],
    default_body: Option<&Script>,
    mode: SwitchMode,
    nocase: bool,
    span: Span,
    ctx: &mut TranslationContext<'_>,
    registry: &CommandRegistry,
    depth: u32,
    enclosing: &EnclosingContext,
) {
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
                DiagCode::Xc101,
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
                DiagCode::Xc101,
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
                diagnostic_code: DiagCode::Xc102,
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
                diagnostic_code: DiagCode::Xc200,
            });
            // The subject is dynamic, so no arm pattern can be mapped to a
            // criterion — but the enclosing criteria still apply, so preserve
            // them rather than dropping to a catch-all.
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
    code: DiagCode,
) -> TranslationItem {
    TranslationItem {
        status: TranslateStatus::Translated,
        kind: XCConstructKind::Route,
        irule_command,
        irule_range: Some(span),
        xc_description: xc_description.to_owned(),
        note: String::new(),
        diagnostic_code: code,
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
    ctx: &mut TranslationContext<'_>,
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
                DiagCode::Xc101,
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
                    diagnostic_code: DiagCode::Xc102,
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
                    DiagCode::Xc101,
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
                        diagnostic_code: DiagCode::Xc103,
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
                diagnostic_code: DiagCode::Xc105,
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
                diagnostic_code: DiagCode::Xc106,
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
                diagnostic_code: DiagCode::Xc107,
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
                    diagnostic_code: DiagCode::Xc300,
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
                        diagnostic_code: DiagCode::Xc301,
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
                    diagnostic_code: DiagCode::Xc100,
                });
            }
        }
    }
}

// Public API

/// The profile-stamped iRules [`CommandRegistry`], suitable for lowering and
/// translating an iRule (the §9 operator-head exclusion applies to every
/// dialect-availability query made through it, and every spec's own surface
/// decides the rest).
fn irules_registry() -> &'static CommandRegistry {
    tcl_registry::model::ingress::static_context_for_profile(tcl_dialect::DialectProfile::irules())
        .commands()
}

fn translation_source(
    source: &str,
    registry: &CommandRegistry,
) -> (Arc<CompilationUnit>, AnalysisResult) {
    let environment = tcl_registry::model::resolve_environment("f5-irules");
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let availability = Arc::new(
        environment
            .default_context_registry()
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input = ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        availability,
        config,
    );
    let mut analyser = Analyser::new().with_resolved_input(input);
    let input = analyser
        .prepare_analysis_input("f5-irules")
        .expect("explicit registry context is available");
    let unit = Arc::new(CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(environment.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    ));
    analyser.set_cu_override(Arc::clone(&unit));
    let analysis = analyser.analyse(source, "f5-irules");
    (unit, analysis)
}

/// Translate an iRule to F5 XC configuration. Analyses the iRule source,
/// walks the IR, and returns an [`XCTranslationResult`] with routes,
/// service policies, origin pools, header actions, and a coverage report.
#[must_use]
pub fn translate_irule(source: &str) -> XCTranslationResult {
    translate_irule_with_registry(source, irules_registry())
}

/// [`translate_irule`] against a caller-provided registry (must have the
/// iRules dialect loaded).
#[must_use]
pub fn translate_irule_with_registry(
    source: &str,
    registry: &CommandRegistry,
) -> XCTranslationResult {
    let (unit, analysis) = translation_source(source, registry);
    translate_unit(source, &unit, &analysis, registry)
}

/// Translate an open document's iRule under the document's own retained
/// analysis and compilation unit, rather than reparsing its text under the
/// standalone iRules defaults [`translate_irule`] uses.
///
/// The registry is the one the analysis retained, so the document's
/// configured availability, packs and lexer configuration are the ones the
/// translation walks. `None` when the analysis does not describe exactly
/// `source` as an iRules document, or `unit` was built from other text: a
/// document translation never falls back to the standalone profile.
#[must_use]
pub fn translate_irule_for_document(
    source: &str,
    unit: &CompilationUnit,
    analysis: &AnalysisResult,
) -> Option<XCTranslationResult> {
    if analysis.analysis_context_unavailable.is_some() || unit.source != source {
        return None;
    }
    tcl_compiler::source_graph::current_analysis(source, analysis)?;
    let input = analysis.resolved_input.as_ref()?;
    if !input.unit_profile().is_irules() {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    Some(translate_unit(source, unit, analysis, registry))
}

fn translate_unit(
    source: &str,
    unit: &CompilationUnit,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> XCTranslationResult {
    let module = &unit.ir_module;
    let mut ctx = TranslationContext::new(source, analysis, module, registry);

    // Original event ownership selects handlers; display names only order output.
    let mut event_procs = Vec::new();
    let mut regular_procs = Vec::new();
    for (id, procedure) in module.procedures.iter() {
        if let Some(event) =
            tcl_compiler::source_graph::event_body_for_procedure(module, id, procedure, registry)
        {
            event_procs.push((id, procedure, event));
        } else {
            regular_procs.push((id, procedure));
        }
    }
    event_procs.sort_by(|a, b| {
        a.1.qualified_name
            .cmp(&b.1.qualified_name)
            .then(a.0.cmp(b.0))
    });
    regular_procs.sort_by(|a, b| {
        a.1.qualified_name
            .cmp(&b.1.qualified_name)
            .then(a.0.cmp(b.0))
    });

    for (_, proc, event) in event_procs {
        let event_name = event.event().to_owned();
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
                diagnostic_code: DiagCode::Xc201,
            });
        } else if let Some(desc) = advisory_event(&event_name) {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Advisory,
                kind: XCConstructKind::Note,
                irule_command: format!("when {event_name}"),
                irule_range: Some(proc.span),
                xc_description: desc.to_owned(),
                note: "This event maps to a separate XC feature.".to_owned(),
                diagnostic_code: DiagCode::Xc250,
            });
        } else {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Untranslatable,
                kind: XCConstructKind::Note,
                irule_command: format!("when {event_name}"),
                irule_range: Some(proc.span),
                xc_description: format!("Event '{event_name}' has no known XC equivalent."),
                note: String::new(),
                diagnostic_code: DiagCode::Xc201,
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

#[cfg(test)]
mod original_getter_geometry_tests {
    use super::*;
    fn condition_to_ip_prefix_set(node: &ExprNode) -> Option<String> {
        let ExprNode::Command { text, .. } = node else {
            return None;
        };
        let parts = parse_command_parts(text);
        if parts.len() == 5
            && parts[0] == "class"
            && parts[1] == "match"
            && original_command_words(text)
                .and_then(|words| scalar_getter_operand(words.get(2)?))
                .as_deref()
                == Some("IP::client_addr")
        {
            return parts.last().cloned();
        }
        None
    }

    fn condition_to_class_match_method(node: &ExprNode) -> Option<XCMethodMatch> {
        let ExprNode::Command { text, .. } = node else {
            return None;
        };
        let parts = parse_command_parts(text);
        if parts.len() == 5
            && parts[0] == "class"
            && parts[1] == "match"
            && original_command_words(text)
                .and_then(|words| scalar_getter_operand(words.get(2)?))
                .as_deref()
                == Some("HTTP::method")
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

    #[test]
    fn scalar_getter_advice_matches_original_command_word_geometry() {
        for text in [
            "[HTTP::path]",
            "[{HTTP::path}]",
            "[\"HTTP::path\"]",
            r"[HTTP\x3a\x3apath]",
            "[string tolower [HTTP::path]]",
            "[string {tolower} [HTTP::path]]",
        ] {
            assert_eq!(
                is_http_getter(text).as_deref(),
                Some("HTTP::path"),
                "{text}"
            );
        }
        for text in [
            "[string tolower {[HTTP::path]}]",
            "[HTTP::path extra]",
            "[HTTP::path; HTTP::method]",
            "[HTTP::path]suffix",
            "[$getter]",
            "[HTTP::path",
            "[{*}HTTP::path]",
            "[string tolower [HTTP::path] extra]",
        ] {
            assert_eq!(is_http_getter(text), None, "{text}");
        }
    }

    #[test]
    fn class_match_advice_reads_the_original_operand_instead_of_substrings() {
        let node = |text: &str| parse_expr(text, Some("f5-irules"));
        assert_eq!(
            condition_to_ip_prefix_set(&node("[class match [IP::client_addr] equals {allow set}]")),
            Some("allow set".into())
        );
        assert!(
            condition_to_ip_prefix_set(&node("[class match literal equals IP::client_addr]"))
                .is_none()
        );
        assert!(
            condition_to_class_match_method(&node("[class match [HTTP::method] equals methods]"))
                .is_some()
        );
        assert!(
            condition_to_class_match_method(&node("[class match literal equals HTTP::method]"))
                .is_none()
        );
        assert!(
            condition_to_ip_prefix_set(&node("[class match {[IP::client_addr]} equals allow]"))
                .is_none()
        );
        assert!(
            condition_to_class_match_method(&node("[class match {[HTTP::method]} equals methods]"))
                .is_none()
        );
    }
}

#[cfg(test)]
mod original_call_owner_tests {
    use super::*;

    #[test]
    fn translated_actions_use_original_source_owners_and_decline_shadowed_heads() {
        let available = translate_irule("when HTTP_REQUEST {pool real_pool}");
        assert!(
            available
                .origin_pools
                .iter()
                .any(|pool| pool.name == "real_pool")
        );
        let shadowed =
            translate_irule("proc pool args {return}; when HTTP_REQUEST {pool fake_pool}");
        assert!(
            !shadowed
                .origin_pools
                .iter()
                .any(|pool| pool.name == "fake_pool")
        );
        let dynamic = translate_irule("when HTTP_REQUEST {pool $selected_pool}");
        assert!(dynamic.origin_pools.is_empty());
        let quoted = translate_irule("when HTTP_REQUEST {{pool} {literal pool}}");
        assert!(
            quoted
                .origin_pools
                .iter()
                .any(|pool| pool.name == "literal pool")
        );
    }
}

#[cfg(test)]
mod original_expression_owner_tests {
    use super::*;

    fn first_clause(module: &Module) -> &tcl_compiler::ir::IfClause {
        module
            .procedures
            .values()
            .flat_map(|procedure| &procedure.body.statements)
            .find_map(|statement| match statement {
                Statement::If { clauses, .. } => clauses.first(),
                _ => None,
            })
            .expect("original fixture condition")
    }

    #[test]
    fn getter_source_advice_retains_original_positions_input_and_module() {
        let source = r#"when HTTP_REQUEST {if {[HTTP::path] eq "/a"} {pool route}}"#;
        let registry = irules_registry();
        let (unit, analysis) = translation_source(source, registry);
        let clause = first_clause(&unit.ir_module);
        let context = TranslationContext::new(source, &analysis, &unit.ir_module, registry);
        let advice = SourceExpressionAdvice::new(&context, clause.condition_base);
        assert_eq!(
            condition_to_path_match(&clause.condition, &advice)
                .unwrap()
                .value,
            "/a"
        );
        let unavailable = SourceExpressionAdvice::new(&context, None);
        assert!(condition_to_path_match(&clause.condition, &unavailable).is_none());
        let stale = TranslationContext::new(
            "when HTTP_REQUEST {if {[HTTP::path] eq \"/changed\"} {pool route}}",
            &analysis,
            &unit.ir_module,
            registry,
        );
        assert!(
            condition_to_path_match(
                &clause.condition,
                &SourceExpressionAdvice::new(&stale, clause.condition_base)
            )
            .is_none()
        );
        let mut missing = unit.ir_module.clone();
        missing.retained_source_bindings = None;
        let missing_context = TranslationContext::new(source, &analysis, &missing, registry);
        assert!(
            condition_to_path_match(
                &clause.condition,
                &SourceExpressionAdvice::new(&missing_context, clause.condition_base)
            )
            .is_none()
        );
        let mut changed_config = unit.ir_module.clone();
        changed_config.lexer_config.strict_quoting = !changed_config.lexer_config.strict_quoting;
        let changed_context = TranslationContext::new(source, &analysis, &changed_config, registry);
        assert!(
            condition_to_path_match(
                &clause.condition,
                &SourceExpressionAdvice::new(&changed_context, clause.condition_base)
            )
            .is_none()
        );
        let mut report_changed = unit.ir_module.clone();
        for procedure in report_changed.procedures.values_mut() {
            procedure.qualified_name = "counterfactual".into();
        }
        let report_context = TranslationContext::new(source, &analysis, &report_changed, registry);
        assert_eq!(
            condition_to_path_match(
                &clause.condition,
                &SourceExpressionAdvice::new(&report_context, clause.condition_base)
            )
            .unwrap()
            .value,
            "/a"
        );
        let foreign_registry =
            tcl_registry::model::resolve_environment("tcl8.6").default_context_registry();
        let foreign = TranslationContext::new(
            source,
            &analysis,
            &unit.ir_module,
            foreign_registry.commands(),
        );
        assert!(
            condition_to_path_match(
                &clause.condition,
                &SourceExpressionAdvice::new(&foreign, clause.condition_base)
            )
            .is_none()
        );
    }

    #[test]
    fn replaced_getters_and_literal_switch_brackets_issue_no_getter_route() {
        let replaced = translate_irule(
            r#"proc HTTP::path {} {return /fake}; when HTTP_REQUEST {if {[HTTP::path] eq "/a"} {pool route}}"#,
        );
        assert!(replaced.routes.is_empty());
        let literal =
            translate_irule(r#"when HTTP_REQUEST {switch {[HTTP::path]} {"/a" {pool route}}}"#);
        assert!(
            literal
                .routes
                .iter()
                .all(|route| route.path_match.is_none())
        );
        let nested = translate_irule(
            r#"when HTTP_REQUEST {if {[string tolower [HTTP::path]] eq "/a"} {pool route}}"#,
        );
        assert!(nested.routes.iter().any(|route| {
            route
                .path_match
                .as_ref()
                .is_some_and(|path| path.value == "/a")
        }));
    }
}

#[cfg(test)]
mod supplied_source_registry_tests {
    use super::*;

    fn assert_selected_override(
        source: &str,
        registry: &CommandRegistry,
        offset: u32,
        name: &str,
        flag: Option<bool>,
    ) {
        let (_, analysis) = translation_source(source, registry);
        let words = tcl_compiler::registry_invocation::source_structure::source_registry_words_at(
            source, &analysis, offset,
        )
        .expect("original selected source occurrence");
        let input = analysis.resolved_input.as_ref().unwrap();
        assert_eq!(
            words.with_source_schema(&input.context_registry(), |schema| {
                let actual = schema.authored_source_descriptors().command;
                assert!(
                    std::ptr::eq(actual, registry.get(name).unwrap()),
                    "actual supplied descriptor"
                );
                assert_eq!(tcl_registry::registry::spec_pack_of(actual), None);
                actual.xc_translatable
            }),
            Some(flag)
        );
    }

    #[test]
    fn supplied_registry_source_availability_cannot_borrow_shipped_translation() {
        let mut registry =
            irules_registry().project_for_profile(tcl_dialect::DialectProfile::irules());
        registry.insert(tcl_registry::CommandSpec {
            name: "pool",
            xc_translatable: Some(false),
            ..irules_registry().get("pool").unwrap().clone()
        });
        let source = "when HTTP_REQUEST {pool retained_custom_pool}";
        let (unit, analysis) = translation_source(source, &registry);
        assert_eq!(
            unit.ir_module
                .registry_snapshot
                .as_ref()
                .unwrap()
                .semantic_key(),
            registry.snapshot().semantic_key()
        );
        assert_eq!(
            analysis
                .resolved_registry()
                .unwrap()
                .snapshot()
                .semantic_key(),
            registry.snapshot().semantic_key()
        );
        assert_selected_override(source, &registry, 19, "pool", Some(false));
        let result = translate_irule_with_registry(source, &registry);
        assert!(result.origin_pools.is_empty());
        assert!(result.routes.is_empty());
        assert!(!result.items.is_empty());
        for override_flag in [None, Some(true)] {
            let mut custom =
                irules_registry().project_for_profile(tcl_dialect::DialectProfile::irules());
            custom.insert(tcl_registry::CommandSpec {
                name: "pool",
                xc_translatable: override_flag,
                ..irules_registry().get("pool").unwrap().clone()
            });
            assert_selected_override(source, &custom, 19, "pool", override_flag);
            let result = translate_irule_with_registry(source, &custom);
            assert!(result.origin_pools.is_empty());
            assert!(result.routes.is_empty());
        }
        let getter_source = r#"when HTTP_REQUEST {if {[HTTP::path] eq "/a"} {pool p}}"#;
        let shipped = irules_registry().project_for_profile(tcl_dialect::DialectProfile::irules());
        let original = translate_irule_with_registry(getter_source, &shipped);
        assert_eq!(original.routes.len(), 1);
        assert_eq!(original.origin_pools.len(), 1);
        for override_flag in [None, Some(true), Some(false)] {
            let mut custom =
                irules_registry().project_for_profile(tcl_dialect::DialectProfile::irules());
            custom.insert(tcl_registry::CommandSpec {
                name: "HTTP::path",
                xc_translatable: override_flag,
                ..irules_registry().get("HTTP::path").unwrap().clone()
            });
            assert_selected_override(getter_source, &custom, 24, "HTTP::path", override_flag);
            let result = translate_irule_with_registry(getter_source, &custom);
            assert!(
                result.routes.is_empty(),
                "custom getter flag {override_flag:?}"
            );
            assert!(!result.items.is_empty());
        }
    }

    #[test]
    fn document_translation_walks_the_documents_retained_registry() {
        let source = "when HTTP_REQUEST {pool retained_custom_pool}";
        let standalone = translate_irule(source);
        assert_eq!(standalone.origin_pools.len(), 1, "shipped pool translates");

        let mut registry =
            irules_registry().project_for_profile(tcl_dialect::DialectProfile::irules());
        registry.insert(tcl_registry::CommandSpec {
            name: "pool",
            xc_translatable: Some(false),
            ..irules_registry().get("pool").unwrap().clone()
        });
        let (unit, analysis) = translation_source(source, &registry);
        let document = translate_irule_for_document(source, &unit, &analysis)
            .expect("an iRules document analysis of this exact source");
        let supplied = translate_irule_with_registry(source, &registry);
        assert!(document.origin_pools.is_empty(), "the document's own spec");
        assert_eq!(document.items, supplied.items);
        assert_eq!(document.coverage_pct, supplied.coverage_pct);

        let (shipped_unit, shipped_analysis) = translation_source(source, irules_registry());
        let shipped = translate_irule_for_document(source, &shipped_unit, &shipped_analysis)
            .expect("shipped iRules document");
        assert_eq!(shipped.items, standalone.items);
        assert_eq!(shipped.origin_pools.len(), 1);
    }

    #[test]
    fn document_translation_refuses_other_source_or_a_non_irules_input() {
        let source = "when HTTP_REQUEST {pool p}";
        let (unit, analysis) = translation_source(source, irules_registry());
        let edited = "when HTTP_REQUEST {pool q}";
        assert!(translate_irule_for_document(edited, &unit, &analysis).is_none());
        let (edited_unit, _) = translation_source(edited, irules_registry());
        assert!(translate_irule_for_document(source, &edited_unit, &analysis).is_none());
        assert!(translate_irule_for_document(edited, &edited_unit, &analysis).is_none());

        let tcl = Analyser::new().analyse(source, "tcl8.6");
        assert!(translate_irule_for_document(source, &unit, &tcl).is_none());
    }
}
