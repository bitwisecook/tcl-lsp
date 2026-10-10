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
//! refine. Static extraction is advertised only when the actual current
//! analysis independently selects lexical editing advice and the deterministic
//! extractor ([`extract_to_datagroup_with_analysis`]) accepts the construct.
//!
//! Syntax traversal retains the actual source and selected grammar. Switch
//! subjects and case layouts use Core's shared original source receipt; body
//! descriptions remain heuristic advice with independent edit eligibility.

use std::net::{Ipv4Addr, Ipv6Addr};

use regex::Regex;
use serde_json::{Value, json};
use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LexerConfig, LineIndex};
use tcl_lsp_core::SourceSyntaxStructure;
use tcl_lsp_core::refactor::{
    OriginalExactSwitchSource, extract_to_datagroup_with_analysis,
    original_exact_switch_source_at_analysis, scalar_variable_source_syntax,
};
use tcl_registry::{ArgRole, CommandRegistry};

const DIALECT: &str = "f5-irules";

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
    let analysis = crate::tools::analyse(source, DIALECT);
    suggest_for_analysis(source, &analysis)
}

/// Heuristic report candidates and independent current extraction eligibility.
fn suggest_for_analysis(source: &str, analysis: &AnalysisResult) -> Vec<Value> {
    let Some((_, config)) = tcl_compiler::source_graph::current_analysis(source, analysis) else {
        return Vec::new();
    };
    let Some(registry) = analysis.resolved_registry() else {
        return Vec::new();
    };
    let line_index = LineIndex::new(source);
    let mut out = Vec::new();

    let Some(structure) = SourceSyntaxStructure::capture(source, analysis) else {
        return out;
    };
    for command in structure.commands() {
        let Some(head) = command.argv.first() else {
            continue;
        };
        let cursor = head.span.start();
        let line = line_index.position_at_utf16(cursor, source).line;
        let mut cand = original_exact_switch_source_at_analysis(source, analysis, cursor)
            .and_then(|original| analyse_switch(&original, line, config));
        if cand.is_none() && command.texts.first().is_some_and(|head| head == "if") {
            cand = analyse_if_chain(&command.texts, line, registry, config);
        }
        if let Some(c) = cand.as_mut() {
            c.has_static_extraction =
                extract_to_datagroup_with_analysis(source, cursor, "", analysis, &line_index)
                    .is_some();
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
        forward: Regex::new(r#"(?x) ^ \s* (.+?) \s+ (eq|==|ne|!=) \s+ (.+?) \s* $ "#)
            .expect("forward eq regex"),
        // `value OP $var` / `value OP "$var"`.
        reverse: Regex::new(r#"(?x) ^ \s* (.+) \s+ (eq|==|ne|!=) \s+ (.+?) \s* $ "#)
            .expect("reverse eq regex"),
        or_split: Regex::new(r"\s*\|\|\s*").expect("or-split regex"),
    })
}

/// Parse a simple equality test into `(var, value, negated)` (unwraps
/// `{ … }`, handles a leading `!( … )` negation).
fn parse_eq(cond: &str, config: LexerConfig) -> Option<(String, String, bool)> {
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
    if let Some(m) = res.forward.captures(cond)
        && let Some(subject) = scalar_variable_source_syntax(m.get(1)?.as_str().trim(), config)
    {
        let op = m.get(2)?.as_str();
        let is_ne = op == "ne" || op == "!=";
        let value = m.get(3)?.as_str().trim().to_owned();
        return Some((subject.name().to_owned(), value, negated ^ is_ne));
    }
    if let Some(m) = res.reverse.captures(cond)
        && let Some(subject) = scalar_variable_source_syntax(m.get(3)?.as_str().trim(), config)
    {
        let op = m.get(2)?.as_str();
        // registry-axis-ok: irreducible — same expr-operator text; until
        // never
        let is_ne = op == "ne" || op == "!=";
        let value = m.get(1)?.as_str().trim().to_owned();
        return Some((subject.name().to_owned(), value, negated ^ is_ne));
    }
    None
}

/// Detect `$var eq "a" || $var eq "b" || …` chains — returns the shared
/// variable and its compared values.
fn try_or_chain(condition: &str, config: LexerConfig) -> Option<(String, Vec<String>)> {
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
        let (var, value, negated) = parse_eq(part.trim(), config)?;
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

// ── Body-shape analysis ───────────────────────────────────────────────

/// A single-command arm body parsed as `set var val` or `return val`.
enum SetOrReturn {
    Set(String),
    Return,
}

/// Parse a single-command arm body via the segmenter (like the Rust static
/// extractor's `parse_set_or_return`, keeping only the kind + `set` variable).
fn parse_set_or_return(text: &str, config: LexerConfig) -> Option<SetOrReturn> {
    let commands = segment_commands_with_offset_and_config(text, 0, config);
    if commands.len() != 1 || commands[0].texts.is_empty() {
        return None;
    }
    let texts = &commands[0].texts;
    // registry-axis-ok: irreducible — `set` and `return` are recognised as
    // Tcl's own primitive syntax for this one-command-body shape, not as a
    // pack-authorable command; until never
    if texts[0] == "set" && texts.len() == 3 {
        return Some(SetOrReturn::Set(texts[1].clone()));
    }
    // registry-axis-ok: irreducible — same primitive-syntax reason; until
    // never
    if texts[0] == "return" && texts.len() == 2 {
        return Some(SetOrReturn::Return);
    }
    None
}

/// Classify a set of arm bodies as `set_mapping`, `return_mapping`, or
/// `complex`.
fn classify_body_shape(bodies: &[String], config: LexerConfig) -> &'static str {
    let mut target_var: Option<String> = None;
    let mut use_return: Option<bool> = None;

    for body in bodies {
        let mut text = body.trim();
        if text.len() >= 2 && text.starts_with('{') && text.ends_with('}') {
            text = text[1..text.len() - 1].trim();
        }
        match parse_set_or_return(text, config) {
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
fn body_shape(bodies: &[String], config: LexerConfig) -> &'static str {
    let set: std::collections::BTreeSet<&str> = bodies.iter().map(|b| b.trim()).collect();
    if set.len() == 1 {
        "identical"
    } else {
        classify_body_shape(bodies, config)
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
/// variable to literals — through `if`'s own clause grammar rather than
/// comparing keyword spellings by hand.
fn analyse_if_chain(
    texts: &[String],
    line: u32,
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<Candidate> {
    if texts.len() < 3 {
        return None;
    }
    let args: Vec<&str> = texts[1..].iter().map(String::as_str).collect();
    let resolved = registry.resolve_call("if", &args, None)?;
    let plan = resolved.clause_plan(&args, None)?;
    if plan.defect.is_some() {
        return None;
    }

    let mut target_var: Option<String> = None;
    let mut values: Vec<String> = Vec::new();
    let mut bodies: Vec<String> = Vec::new();

    if let Some((var, or_values)) = try_or_chain(&texts[1], config) {
        target_var = Some(var);
        values = or_values;
        bodies.push(texts[2].clone());
    } else {
        for clause in &plan.clauses {
            if clause.is_default {
                break;
            }
            let body = args[clause.operand(ArgRole::Body)?].to_owned();
            let condition = args[clause.operand(ArgRole::Expr)?];

            let (var, value, negated) = parse_eq(condition, config)?;
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
    let shape = body_shape(&bodies, config);
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
fn analyse_switch(
    original: &OriginalExactSwitchSource,
    line: u32,
    config: LexerConfig,
) -> Option<Candidate> {
    let subject_var = original.subject().name().to_owned();
    let pairs = original.pairs();

    let regular: Vec<&(String, String)> = pairs
        .iter()
        .filter(|(pat, body)| pat != "default" && body.trim() != "-")
        .collect();
    if regular.len() < 3 {
        return None;
    }

    let keys: Vec<String> = regular.iter().map(|(p, _)| p.clone()).collect();
    let value_type = infer_value_type(&keys);
    let has_cidr = any_cidr(&keys);
    let bodies: Vec<String> = regular.iter().map(|(_, b)| b.trim().to_owned()).collect();
    let shape = body_shape(&bodies, config);
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

    #[test]
    fn selected_if_suggestions_share_actual_scalar_reference_grammar() {
        // Implementation contract: naming.refactor.selected-if-scalar-source-syntax
        // docs/design/analysis/name-resolution-proofs/selected-if-scalar-source-syntax.md
        for (dialect, subject, root) in [
            ("tcl8.6", "${a b}", "a b"),
            ("tcl9.1", "${a{b}c}", "a{b}c"),
            ("f5-irules", "\"${café}\"", "café"),
            ("jim", "$café", "café"),
            ("tcl8.6", "${a(k)tail}", "a(k)tail"),
            ("tcl8.6", "${literal$name}", "literal$name"),
        ] {
            let source = format!("if {{{subject} eq \"a\" || {subject} eq \"b\"}} {{drop}}");
            let analysis = crate::tools::analyse(&source, dialect);
            let candidates = suggest_for_analysis(&source, &analysis);
            assert_eq!(candidates.len(), 1, "{dialect}: {source}");
            assert_eq!(candidates[0]["variable"], root);
            assert_eq!(candidates[0]["value_count"], 2);
            assert_eq!(candidates[0]["has_static_extraction"], false);
        }
        for subject in [
            "$a(k)",
            "${a(k)}",
            "$café",
            "\"$x[set y]\"",
            "{$x}",
            "$x.tail",
        ] {
            let source = format!(
                "if {{{subject} eq \"a\"}} {{drop}} elseif {{{subject} eq \"b\"}} {{drop}}"
            );
            let analysis = crate::tools::analyse(&source, "tcl8.6");
            assert!(
                suggest_for_analysis(&source, &analysis).is_empty(),
                "{subject:?}"
            );
        }
        let first = LexerConfig::for_dialect("tcl8.6");
        let jim = LexerConfig::for_dialect("jim");
        assert!(parse_eq("$café eq one", first).is_none());
        assert_eq!(parse_eq("one\teq\t$café", jim).unwrap().0, "café");
        assert!(parse_eq("${missing eq one", first).is_none());
        assert!(try_or_chain("$x eq one || $y eq two", first).is_none());
        assert!(try_or_chain("$x eq one || !($x eq two)", first).is_none());
    }

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
        assert_eq!(c[0]["has_static_extraction"], false);
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
    fn original_datagroup_suggestions_separate_heuristics_from_current_edit_eligibility() {
        // Implementation contract: naming.mcp.original-datagroup-suggestion-eligibility
        // docs/design/analysis/name-resolution-proofs/original-datagroup-suggestion-eligibility.md
        let source = "if {$host eq \"a.com\"} {pool p} elseif {$host eq \"b.com\"} {pool p} elseif {$host eq \"c.com\"} {pool p}";
        let hosted = crate::tools::analyse(source, DIALECT);
        let native = crate::tools::analyse(source, "tcl8.6");
        for analysis in [&hosted, &native] {
            let candidates = suggest_for_analysis(source, analysis);
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0]["has_static_extraction"], false);
            assert!(suggest_for_analysis(&format!("# changed\n{source}"), analysis).is_empty());
        }

        let point =
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79);
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "datagroup-explicit-lexical",
            &[],
            "Logical data-group source advice",
            point,
        )
        .intern();
        let registry = hosted.resolved_registry().unwrap();
        let context = tcl_lsp_core::context_for_dialect_profile(profile)
            .with_command_store(registry.snapshot().shared_registry());
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(context),
            LexerConfig::for_profile(Some(profile)),
        );
        let mut lexical = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        assert!(lexical.allows_lexical_declaration_advice());
        let candidates = suggest_for_analysis(source, &lexical);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0]["has_static_extraction"], true);
        // The shared supplied-source extractor checks the output handler at
        // this source horizon. A report candidate alone cannot restore it.
        let replaced_source = format!("proc class args {{}}; {source}");
        let replaced = tcl_compiler::analyser::Analyser::new()
            .with_resolved_input(lexical.resolved_input.as_ref().unwrap().clone())
            .analyse(&replaced_source, profile.name);
        let replaced_candidates = suggest_for_analysis(&replaced_source, &replaced);
        assert_eq!(replaced_candidates.len(), 1);
        assert_eq!(replaced_candidates[0]["has_static_extraction"], false);
        let config = lexical.body_lexer_config.as_mut().unwrap();
        config.strict_quoting = !config.strict_quoting;
        assert!(suggest_for_analysis(source, &lexical).is_empty());
    }

    #[test]
    fn wire_shape() {
        let source =
            "if {$host eq \"a.com\"} {\n    pool p\n} elseif {$host eq \"b.com\"} {\n    pool p\n}";
        let out = suggest_datagroup_extractions(&json!({ "source": source }));
        assert_eq!(out["total"], 1);
        assert!(out["candidates"].is_array());
    }

    #[test]
    fn original_switch_suggestions_share_scalar_subject_and_literal_case_values() {
        // Implementation contract: naming.refactor.original-datagroup-variable-subject
        // docs/design/analysis/name-resolution-proofs/original-datagroup-variable-subject.md
        for (subject, name) in [
            ("${café}", "café"),
            ("\"${a b}\"", "a b"),
            ("${literal$name}", "literal$name"),
            (r"${a\b}", r"a\b"),
            ("${a(}", "a("),
            ("${a(k)tail}", "a(k)tail"),
        ] {
            let source =
                format!("switch -exact -- {subject} {{GET {{drop}} POST {{drop}} PUT {{drop}}}}");
            let c = candidates(&source);
            assert_eq!(c.len(), 1, "{subject}");
            assert_eq!(c[0]["variable"], name);
            assert_eq!(c[0]["values"], json!(["GET", "POST", "PUT"]));
            assert_eq!(c[0]["has_static_extraction"], false);
        }
        for subject in [
            "{$x}", "{${x}}", r"\$x", "$x-tail", "$a(k)", "${a(k)}", "[get]", "$café",
        ] {
            let source =
                format!("switch -exact -- {subject} {{GET {{drop}} POST {{drop}} PUT {{drop}}}}");
            assert!(candidates(&source).is_empty(), "{subject}");
        }
        let source = "switch -exact -- $x {{\"quoted\"} {drop} plain {drop} third {drop}}";
        let c = candidates(source);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0]["values"], json!(["\"quoted\"", "plain", "third"]));
    }

    #[test]
    fn original_switch_suggestions_keep_selected_alias_and_current_analysis() {
        // Implementation contract: naming.refactor.original-datagroup-variable-subject
        // docs/design/analysis/name-resolution-proofs/original-datagroup-variable-subject.md
        let source = "interp alias {} select {} switch -exact --\nselect ${café} {GET {drop} POST {drop} PUT {drop}}";
        let mut analysis = crate::tools::analyse(source, DIALECT);
        let c = suggest_for_analysis(source, &analysis);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0]["variable"], "café");
        assert_eq!(c[0]["line"], 1);
        assert_eq!(c[0]["has_static_extraction"], false);
        assert!(suggest_for_analysis(&format!("# changed\n{source}"), &analysis).is_empty());
        analysis.resolved_input = None;
        assert!(suggest_for_analysis(source, &analysis).is_empty());
        let shadow =
            "proc switch {args} {}\nswitch -exact -- $x {GET {drop} POST {drop} PUT {drop}}";
        assert!(candidates(shadow).is_empty());
        for options in ["-nocase --", "-glob --", "$options --", "{*}$options --"] {
            let source = format!("switch {options} $x {{GET {{drop}} POST {{drop}} PUT {{drop}}}}");
            assert!(candidates(&source).is_empty(), "{options}");
        }
        let source = "switch -exact -- $x {GET {drop} POST {drop} PUT {drop}}";
        let mut analysis = crate::tools::analyse(source, DIALECT);
        assert_eq!(suggest_for_analysis(source, &analysis).len(), 1);
        let config = analysis.body_lexer_config.as_mut().unwrap();
        config.strict_quoting = !config.strict_quoting;
        assert!(suggest_for_analysis(source, &analysis).is_empty());
    }
}
