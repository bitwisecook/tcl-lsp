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

//! AI-enhanced data-group extraction scan.
//!
//! Scans iRules source for `if`/`switch` patterns that could become
//! data-groups and returns structured context (pattern type, inferred value
//! type, CIDR detection, body-shape analysis, confidence) for an LLM to
//! refine, plus whether the deterministic extractor
//! ([`extract_to_datagroup`]) can handle the construct at its cursor.
//!
//! This is an AI-only heuristic: segmentation comes from [`walk_commands`] and
//! the static-extractability check from [`extract_to_datagroup`].

use std::net::{Ipv4Addr, Ipv6Addr};

use regex::Regex;
use serde_json::{Value, json};
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LexerConfig, LineIndex};
use tcl_lsp_core::refactor::{extract_to_datagroup, walk_commands};
use tcl_syntax::switch_body::parse_braced_pairs;

const DIALECT: &str = "f5-irules";

/// This module's whole job is scanning iRules source (`DIALECT` is fixed),
/// so its lexing grammar is resolved once, here, rather than per call.
fn config() -> LexerConfig {
    LexerConfig::from_grammar(crate::environment::profile_for_dialect(DIALECT).grammar)
}

/// MCP handler: `{candidates:[…], total:int}` for the `source` argument.
pub fn suggest_datagroup_extractions(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");
    let candidates = suggest(source);
    json!({ "candidates": candidates, "total": candidates.len() })
}

/// A structured extraction candidate — the wire entry (minus the
/// non-serialisable `static_result`, replaced by `has_static_extraction`).
struct Candidate {
    line: u32,
    pattern_type: &'static str,
    variable: String,
    values: Vec<String>,
    inferred_type: &'static str,
    has_cidr: bool,
    body_shape: &'static str,
    suggested_name: String,
    confidence: &'static str,
    value_count: usize,
    has_static_extraction: bool,
}

impl Candidate {
    fn into_json(self) -> Value {
        json!({
            "line": self.line,
            "pattern_type": self.pattern_type,
            "variable": self.variable,
            "values": self.values,
            "inferred_type": self.inferred_type,
            "has_cidr": self.has_cidr,
            "body_shape": self.body_shape,
            "suggested_name": self.suggested_name,
            "confidence": self.confidence,
            "value_count": self.value_count,
            "has_static_extraction": self.has_static_extraction,
        })
    }
}

/// Scan `source` for `if`/`switch` patterns extractable to data-groups.
fn suggest(source: &str) -> Vec<Value> {
    let registry = crate::environment::store_for_dialect(DIALECT);
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();

    for (texts, line, character) in walk_commands(source, registry, config()) {
        let Some(head) = texts.first() else { continue };
        let mut cand = match head.as_str() {
            "if" => analyse_if_chain(&texts, line),
            "switch" => analyse_switch(&texts, line),
            _ => None,
        };
        if let Some(c) = cand.as_mut() {
            let cursor =
                line_index.offset_at_utf16(line, tcl_lexer::Utf16Col::new(character), source);
            c.has_static_extraction =
                extract_to_datagroup(source, cursor, "", registry, &line_index, config()).is_some();
        }
        if let Some(c) = cand {
            out.push(c.into_json());
        }
    }
    out
}

// ── Value-type inference ──────────────────────────────────────────────

/// Strip a single layer of surrounding double quotes (after trimming).
fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() >= 2 && b[0] == b'"' && b[b.len() - 1] == b'"' {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// `true` when `value` looks like an IPv4/IPv6 address or CIDR range,
/// reproducing `ipaddress.ip_network` / `ip_address` acceptance.
fn is_ip_or_cidr(value: &str) -> bool {
    let v = strip_quotes(value);
    is_ip_network(v) || is_ip_address(v)
}

fn is_ip_address(v: &str) -> bool {
    v.parse::<Ipv4Addr>().is_ok() || v.parse::<Ipv6Addr>().is_ok()
}

/// Parse an `addr/prefix` CIDR (host bits allowed, matching `strict=False`),
/// validating the address family and prefix width.
fn is_ip_network(v: &str) -> bool {
    let Some((addr, prefix)) = v.split_once('/') else {
        return false;
    };
    let Ok(width) = prefix.parse::<u32>() else {
        return false;
    };
    if addr.parse::<Ipv4Addr>().is_ok() {
        width <= 32
    } else if addr.parse::<Ipv6Addr>().is_ok() {
        width <= 128
    } else {
        false
    }
}

fn is_integer(value: &str) -> bool {
    let v = strip_quotes(value).trim();
    !v.is_empty() && v.parse::<i64>().is_ok()
}

/// Infer the data-group value type: `ip` if every value is an address/CIDR,
/// else `integer` if every value parses as an integer, else `string`.
fn infer_value_type(values: &[String]) -> &'static str {
    if values.is_empty() {
        return "string";
    }
    let stripped: Vec<&str> = values.iter().map(|v| strip_quotes(v)).collect();
    if stripped.iter().all(|v| is_ip_or_cidr(v)) {
        return "ip";
    }
    if stripped.iter().all(|v| is_integer(v)) {
        return "integer";
    }
    "string"
}

/// Derive a clean data-group name from a descriptive string:
/// non-`[A-Za-z0-9_]` → `_`, collapse runs, trim, lower-case.
fn normalise_dg_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_underscore = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch.to_ascii_lowercase());
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "extracted_dg".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// `true` when any address/CIDR value carries a `/` (i.e. is a range).
fn any_cidr(values: &[String]) -> bool {
    values
        .iter()
        .filter(|v| is_ip_or_cidr(strip_quotes(v)))
        .any(|v| v.contains('/'))
}

// ── Equality-condition parsing ────────────────────────────────────────

/// Lazily-compiled regexes for the forward/reverse equality conditions.
struct EqRegexes {
    forward: Regex,
    reverse: Regex,
    or_split: Regex,
}

fn eq_regexes() -> &'static EqRegexes {
    static RE: std::sync::OnceLock<EqRegexes> = std::sync::OnceLock::new();
    RE.get_or_init(|| EqRegexes {
        // `$var OP value` / `"$var" OP value`.
        forward: Regex::new(
            r#"(?x) ^ \s* (?: \$ \{? (\w+) \}? | " \$ \{? (\w+) \}? " ) \s+ (eq|==|ne|!=) \s+ (.+?) \s* $ "#,
        )
        .expect("forward eq regex"),
        // `value OP $var` / `value OP "$var"`.
        reverse: Regex::new(
            r#"(?x) ^ \s* (.+?) \s+ (eq|==|ne|!=) \s+ (?: \$ \{? (\w+) \}? | " \$ \{? (\w+) \}? " ) \s* $ "#,
        )
        .expect("reverse eq regex"),
        or_split: Regex::new(r"\s*\|\|\s*").expect("or-split regex"),
    })
}

/// Parse a simple equality test into `(var, value, negated)` (unwraps
/// `{ … }`, handles a leading `!( … )` negation).
fn parse_eq(cond: &str) -> Option<(String, String, bool)> {
    let mut cond = cond.trim();
    if cond.len() >= 2 && cond.starts_with('{') && cond.ends_with('}') {
        cond = cond[1..cond.len() - 1].trim();
    }

    let mut negated = false;
    if let Some(rest) = cond.strip_prefix('!') {
        let inner = rest.trim();
        if inner.len() >= 2 && inner.starts_with('(') && inner.ends_with(')') {
            cond = inner[1..inner.len() - 1].trim();
            negated = true;
        }
    }

    let res = eq_regexes();
    if let Some(m) = res.forward.captures(cond) {
        let var = m.get(1).or_else(|| m.get(2))?.as_str().to_owned();
        let op = m.get(3)?.as_str();
        let is_ne = op == "ne" || op == "!=";
        let value = m.get(4)?.as_str().trim().to_owned();
        return Some((var, value, negated ^ is_ne));
    }
    if let Some(m) = res.reverse.captures(cond) {
        let var = m.get(3).or_else(|| m.get(4))?.as_str().to_owned();
        let op = m.get(2)?.as_str();
        let is_ne = op == "ne" || op == "!=";
        let value = m.get(1)?.as_str().trim().to_owned();
        return Some((var, value, negated ^ is_ne));
    }
    None
}

/// Detect `$var eq "a" || $var eq "b" || …` chains — returns the shared
/// variable and its compared values.
fn try_or_chain(condition: &str) -> Option<(String, Vec<String>)> {
    let mut cond = condition.trim();
    if cond.len() >= 2 && cond.starts_with('{') && cond.ends_with('}') {
        cond = cond[1..cond.len() - 1].trim();
    }
    let parts: Vec<&str> = eq_regexes().or_split.split(cond).collect();
    if parts.len() < 2 {
        return None;
    }
    let mut target_var: Option<String> = None;
    let mut values = Vec::new();
    for part in parts {
        let (var, value, negated) = parse_eq(part.trim())?;
        if negated {
            return None;
        }
        match &target_var {
            None => target_var = Some(var),
            Some(v) if *v != var => return None,
            _ => {}
        }
        values.push(value);
    }
    let target_var = target_var?;
    if values.len() < 2 {
        return None;
    }
    Some((target_var, values))
}

/// Extract a variable name from a `$var` / `${var}` switch subject.
fn extract_var_name(subject: &str) -> Option<String> {
    let s = subject.trim();
    if let Some(inner) = s.strip_prefix("${").and_then(|x| x.strip_suffix('}')) {
        return Some(inner.to_owned());
    }
    if let Some(name) = s.strip_prefix('$')
        && !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Some(name.to_owned());
    }
    None
}

// ── Body-shape analysis ───────────────────────────────────────────────

/// A single-command arm body parsed as `set var val` or `return val`.
enum SetOrReturn {
    Set(String),
    Return,
}

/// Parse a single-command arm body via the segmenter (like the Rust static
/// extractor's `parse_set_or_return`, keeping only the kind + `set` variable).
fn parse_set_or_return(text: &str) -> Option<SetOrReturn> {
    let commands = segment_commands_with_offset_and_config(text, 0, config());
    if commands.len() != 1 || commands[0].texts.is_empty() {
        return None;
    }
    let texts = &commands[0].texts;
    if texts[0] == "set" && texts.len() == 3 {
        return Some(SetOrReturn::Set(texts[1].clone()));
    }
    if texts[0] == "return" && texts.len() == 2 {
        return Some(SetOrReturn::Return);
    }
    None
}

/// Classify a set of arm bodies as `set_mapping`, `return_mapping`, or
/// `complex`.
fn classify_body_shape(bodies: &[String]) -> &'static str {
    let mut target_var: Option<String> = None;
    let mut use_return: Option<bool> = None;

    for body in bodies {
        let mut text = body.trim();
        if text.len() >= 2 && text.starts_with('{') && text.ends_with('}') {
            text = text[1..text.len() - 1].trim();
        }
        match parse_set_or_return(text) {
            Some(SetOrReturn::Set(var)) => match &target_var {
                None => {
                    target_var = Some(var);
                    use_return = Some(false);
                }
                Some(v) if *v != var || use_return == Some(true) => return "complex",
                _ => {}
            },
            Some(SetOrReturn::Return) => match use_return {
                None => use_return = Some(true),
                Some(true) => {}
                Some(false) => return "complex",
            },
            None => return "complex",
        }
    }

    if use_return == Some(true) {
        "return_mapping"
    } else if target_var.is_some() {
        "set_mapping"
    } else {
        "complex"
    }
}

/// The body shape: `identical` when every body is the same trimmed text, else
/// the [`classify_body_shape`] result.
fn body_shape(bodies: &[String]) -> &'static str {
    let set: std::collections::BTreeSet<&str> = bodies.iter().map(|b| b.trim()).collect();
    if set.len() == 1 {
        "identical"
    } else {
        classify_body_shape(bodies)
    }
}

fn confidence_for(shape: &str) -> &'static str {
    if matches!(shape, "identical" | "set_mapping" | "return_mapping") {
        "high"
    } else {
        "medium"
    }
}

// ── Pattern analysis ──────────────────────────────────────────────────

/// Analyse an `if`/`elseif` chain (or single OR-chain condition) comparing one
/// variable to literals.
fn analyse_if_chain(texts: &[String], line: u32) -> Option<Candidate> {
    if texts.len() < 3 {
        return None;
    }

    let mut target_var: Option<String> = None;
    let mut values: Vec<String> = Vec::new();
    let mut bodies: Vec<String> = Vec::new();

    if let Some((var, or_values)) = try_or_chain(&texts[1]) {
        target_var = Some(var);
        values = or_values;
        bodies.push(texts[2].clone());
    } else {
        let mut i = 1;
        while i < texts.len() {
            let word = &texts[i];
            if word == "elseif" || word == "then" {
                i += 1;
                continue;
            }
            if word == "else" {
                break;
            }
            if i + 1 >= texts.len() {
                break;
            }
            let body = texts[i + 1].clone();
            i += 2;

            let (var, value, negated) = parse_eq(word)?;
            if negated {
                return None;
            }
            match &target_var {
                None => target_var = Some(var),
                Some(v) if *v != var => return None,
                _ => {}
            }
            values.push(value);
            bodies.push(body);
        }
    }

    let target_var = target_var?;
    if values.len() < 2 {
        return None;
    }

    let stripped: Vec<String> = values.iter().map(|v| strip_quotes(v).to_owned()).collect();
    let value_type = infer_value_type(&stripped);
    let has_cidr = any_cidr(&stripped);
    let shape = body_shape(&bodies);
    let value_count = values.len();

    Some(Candidate {
        line,
        pattern_type: "if_chain",
        suggested_name: normalise_dg_name(&format!("{target_var}_whitelist")),
        variable: target_var,
        values: stripped,
        inferred_type: value_type,
        has_cidr,
        body_shape: shape,
        confidence: confidence_for(shape),
        value_count,
        has_static_extraction: false,
    })
}

/// Analyse a `switch -exact` over literal patterns.
fn analyse_switch(texts: &[String], line: u32) -> Option<Candidate> {
    let mut i = 1;
    let mut mode = "exact";
    while i < texts.len() && texts[i].starts_with('-') {
        let flag = texts[i].as_str();
        match flag {
            "-exact" | "-glob" | "-regexp" => mode = &flag[1..],
            "--" => {
                i += 1;
                break;
            }
            _ => {}
        }
        i += 1;
    }

    if mode != "exact" || i >= texts.len() {
        return None;
    }

    let subject_var = extract_var_name(&texts[i])?;
    i += 1;

    let pairs: Vec<(String, String)> = if i + 1 == texts.len() {
        parse_braced_pairs(&texts[i])
    } else {
        let mut p = Vec::new();
        while i + 1 < texts.len() {
            p.push((texts[i].clone(), texts[i + 1].clone()));
            i += 2;
        }
        p
    };

    let regular: Vec<(String, String)> = pairs
        .into_iter()
        .filter(|(pat, body)| pat != "default" && body.trim() != "-")
        .collect();
    if regular.len() < 3 {
        return None;
    }

    let keys: Vec<String> = regular
        .iter()
        .map(|(p, _)| strip_quotes(p).to_owned())
        .collect();
    let value_type = infer_value_type(&keys);
    let has_cidr = any_cidr(&keys);
    let bodies: Vec<String> = regular.iter().map(|(_, b)| b.trim().to_owned()).collect();
    let shape = body_shape(&bodies);
    let value_count = regular.len();

    Some(Candidate {
        line,
        pattern_type: "switch",
        suggested_name: normalise_dg_name(&format!("{subject_var}_map")),
        variable: subject_var,
        values: keys,
        inferred_type: value_type,
        has_cidr,
        body_shape: shape,
        confidence: confidence_for(shape),
        value_count,
        has_static_extraction: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates(source: &str) -> Vec<Value> {
        suggest(source)
    }

    #[test]
    fn if_chain_string_membership() {
        let source = "if {$host eq \"a.com\"} {\n    pool p\n} elseif {$host eq \"b.com\"} {\n    pool p\n} elseif {$host eq \"c.com\"} {\n    pool p\n}";
        let c = candidates(source);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0]["pattern_type"], "if_chain");
        assert_eq!(c[0]["variable"], "host");
        assert_eq!(c[0]["inferred_type"], "string");
        assert_eq!(c[0]["body_shape"], "identical");
        assert_eq!(c[0]["confidence"], "high");
        assert_eq!(c[0]["value_count"], 3);
        assert_eq!(c[0]["suggested_name"], "host_whitelist");
        assert_eq!(c[0]["has_static_extraction"], true);
    }

    #[test]
    fn if_chain_ip_cidr() {
        let source = "if {$addr eq \"10.0.0.0/8\"} {\n    drop\n} elseif {$addr eq \"172.16.0.0/12\"} {\n    drop\n}";
        let c = candidates(source);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0]["inferred_type"], "ip");
        assert_eq!(c[0]["has_cidr"], true);
    }

    #[test]
    fn or_chain() {
        let source =
            "if {$host eq \"a.com\" || $host eq \"b.com\" || $host eq \"c.com\"} {\n    pool p\n}";
        let c = candidates(source);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0]["value_count"], 3);
    }

    #[test]
    fn switch_mapping() {
        let source = "switch -exact -- $method {\n    GET { set h a }\n    POST { set h b }\n    PUT { set h c }\n}";
        let c = candidates(source);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0]["pattern_type"], "switch");
        assert_eq!(c[0]["variable"], "method");
        assert_eq!(c[0]["body_shape"], "set_mapping");
        assert_eq!(c[0]["suggested_name"], "method_map");
    }

    #[test]
    fn switch_membership_integers() {
        let source = "switch -exact -- $port {\n    80 { set x a }\n    443 { set x a }\n    8080 { set x a }\n}";
        let c = candidates(source);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0]["inferred_type"], "integer");
        assert_eq!(c[0]["body_shape"], "identical");
    }

    #[test]
    fn too_few_values() {
        assert!(candidates("if {$x eq \"a\"} { puts ok }").is_empty());
        assert!(
            candidates("switch -exact -- $x {\n    a { puts 1 }\n    b { puts 2 }\n}").is_empty()
        );
    }

    #[test]
    fn glob_switch_declined() {
        let source =
            "switch -glob -- $x {\n    a* { set y 1 }\n    b* { set y 2 }\n    c* { set y 3 }\n}";
        assert!(candidates(source).is_empty());
    }

    #[test]
    fn wire_shape() {
        let source =
            "if {$host eq \"a.com\"} {\n    pool p\n} elseif {$host eq \"b.com\"} {\n    pool p\n}";
        let out = suggest_datagroup_extractions(&json!({ "source": source }));
        assert_eq!(out["total"], 1);
        assert!(out["candidates"].is_array());
    }
}
