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

//! Extract to data-group — convert inline if/switch membership /
//! mapping patterns to iRules `class match` / `class lookup` against a
//! generated data-group.

use std::net::{Ipv4Addr, Ipv6Addr};

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use tcl_lexer::{LexerConfig, LineIndex};
use tcl_registry::{ArgRole, CommandRegistry};

use super::{
    RefactorEdit, Refactoring, command_span_offsets, find_command_at, line_indent, reindent_body,
    token_end_offset,
};
use crate::code_actions::ActionKind;

mod source_subject;
pub use source_subject::{
    OriginalExactCaseSource, OriginalExactSwitchSource, OriginalScalarVariableSubject,
    ScalarVariableSourceSyntax, original_exact_case_source_at_analysis,
    original_exact_switch_source_at_analysis, scalar_variable_source_syntax,
};

/// A generated data-group artefact (separate from the iRule edits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataGroupDefinition {
    /// Data-group name.
    pub name: String,
    /// Value type: `"string"`, `"ip"`, or `"integer"`.
    pub value_type: String,
    /// `(key, value)` records (value empty for a pure membership group).
    pub records: Vec<(String, String)>,
}

/// Render a [`DataGroupDefinition`] as a BIG-IP tmsh definition.
#[must_use]
pub fn data_group_tcl(dg: &DataGroupDefinition) -> String {
    let mut lines = vec![format!("ltm data-group internal {} {{", dg.name)];
    if !dg.records.is_empty() {
        lines.push("    records {".to_owned());
        for (key, value) in &dg.records {
            if value.is_empty() {
                lines.push(format!("        {key} {{ }}"));
            } else {
                lines.push(format!("        {key} {{"));
                lines.push(format!("            data {value}"));
                lines.push("        }".to_owned());
            }
        }
        lines.push("    }".to_owned());
    }
    lines.push(format!("    type {}", dg.value_type));
    lines.push("}".to_owned());
    lines.join("\n")
}

// Value-type inference

/// `true` when `value` looks like an IPv4/IPv6 address or CIDR range.
///
/// Quote stripping belongs to the caller (see [`infer_value_type`]); this
/// only tolerates the whitespace a quoted literal can still carry.
fn is_ip_or_cidr(value: &str) -> bool {
    let v = value.trim();
    is_ip_address(v) || is_ip_network(v)
}

fn is_ip_address(v: &str) -> bool {
    v.parse::<Ipv4Addr>().is_ok() || v.parse::<Ipv6Addr>().is_ok()
}

/// Parse a `addr/prefix` CIDR (host bits allowed), validating the
/// address family and prefix width.
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
    let v = value.trim();
    !v.is_empty() && v.parse::<i64>().is_ok()
}

/// Infer the data-group value type from a list of keys/values.
///
/// `values` are the records as they will be written to the data group,
/// which both call sites have already normalised with [`strip_quotes`] —
/// stripping again here would peel a second pair off a value whose quotes
/// *are* part of the record, and type the group as `ip`/`integer` when the
/// record itself is the quoted string.
fn infer_value_type(values: &[String]) -> &'static str {
    if values.is_empty() {
        return "string";
    }
    if values.iter().all(|v| is_ip_or_cidr(v)) {
        return "ip";
    }
    if values.iter().all(|v| is_integer(v)) {
        return "integer";
    }
    "string"
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() >= 2 && b[0] == b'"' && b[b.len() - 1] == b'"' {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Derive a clean data-group name from a descriptive string:
/// non-alphanumeric → `_`, collapse runs, trim, lower-case.
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

// Equality-condition parsing

/// Recognised equality operators.
// registry-axis-ok: irreducible — `eq` / `ne` are expr comparison-operator
// spellings (`tcl_syntax::expr::operators`'s own vocabulary), read here as
// parsed condition text; they coincide with `::tcl::mathop::eq` / `ne`'s
// bare registration only by spelling; until never
const EQ_OPS: &[&str] = &["==", "!=", "eq", "ne"];

/// Parse a simple equality test `(var, value, negated)`.  Used by the
/// datagroup transform; mirrors the shape of
/// `if_to_switch::parse_eq_test` minus the operator capture.
fn parse_eq(cond: &str, config: LexerConfig) -> Option<(ScalarVariableSourceSyntax, String, bool)> {
    let mut cond = cond.trim();
    if cond.starts_with('{') && cond.ends_with('}') && cond.len() >= 2 {
        cond = cond[1..cond.len() - 1].trim();
    }
    let mut negated = false;
    if let Some(rest) = cond.strip_prefix('!') {
        let inner = rest.trim();
        if inner.starts_with('(') && inner.ends_with(')') && inner.len() >= 2 {
            cond = inner[1..inner.len() - 1].trim();
            negated = true;
        }
    }

    // `$var OP value`.
    for op in EQ_OPS {
        let needle = format!(" {op} ");
        if let Some(pos) = cond.find(&needle)
            && let Some(var) = scalar_variable_source_syntax(cond[..pos].trim(), config)
        {
            // registry-axis-ok: irreducible — expr operator text, not a
            // command name; until never
            let is_ne = *op == "ne" || *op == "!=";
            return Some((
                var,
                cond[pos + needle.len()..].trim().to_owned(),
                negated ^ is_ne,
            ));
        }
    }
    // `value OP $var`.
    for op in EQ_OPS {
        let needle = format!(" {op} ");
        if let Some(pos) = cond.rfind(&needle)
            && let Some(var) =
                scalar_variable_source_syntax(cond[pos + needle.len()..].trim(), config)
        {
            // registry-axis-ok: irreducible — expr operator text, not a
            // command name; until never
            let is_ne = *op == "ne" || *op == "!=";
            return Some((var, cond[..pos].trim().to_owned(), negated ^ is_ne));
        }
    }
    None
}

/// Detect `$var eq "a" || $var eq "b" || …` chains.
fn try_or_chain(
    condition: &str,
    config: LexerConfig,
) -> Option<(ScalarVariableSourceSyntax, Vec<String>)> {
    let mut cond = condition.trim();
    if cond.starts_with('{') && cond.ends_with('}') && cond.len() >= 2 {
        cond = cond[1..cond.len() - 1].trim();
    }
    let parts: Vec<&str> = cond.split("||").collect();
    if parts.len() < 2 {
        return None;
    }
    let mut target_var: Option<ScalarVariableSourceSyntax> = None;
    let mut values = Vec::new();
    for part in parts {
        let (var, value, negated) = parse_eq(part.trim(), config)?;
        if negated {
            return None;
        }
        match &target_var {
            None => target_var = Some(var),
            Some(v) if v.name() != var.name() => return None,
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

// set/return body parsing

/// Parsed single-command arm body.
enum SetOrReturn {
    Set(String, String),
    Return(String),
}

/// Parse a single-command arm body via the tokeniser.
///
/// `config` is the document's [`LexerConfig`].
fn parse_set_or_return(text: &str, config: LexerConfig) -> Option<SetOrReturn> {
    let commands = segment_commands_with_offset_and_config(text, 0, config);
    if commands.len() != 1 || commands[0].texts.is_empty() {
        return None;
    }
    let cmd = &commands[0];
    let raw = |index: usize| -> String {
        let tok = cmd.argv[index];
        text[tok.span.start() as usize..token_end_offset(text, tok) as usize].to_owned()
    };
    // registry-axis-ok: representation — this renderer accepts the literal
    // proposal template syntax. The actual-document route independently
    // validates each original arm's selected Set/Return operation before
    // calling it; these spellings establish no command identity or effects.
    if cmd.texts[0] == "set" && cmd.texts.len() == 3 {
        return Some(SetOrReturn::Set(cmd.texts[1].clone(), raw(2)));
    }
    // registry-axis-ok: representation — same proposal-template boundary.
    if cmd.texts[0] == "return" && cmd.texts.len() == 2 {
        return Some(SetOrReturn::Return(raw(1)));
    }
    None
}

/// Extract `(target_var, use_return, values)` from arm bodies.
fn extract_set_or_return_values(
    pairs: &[(String, String)],
    config: LexerConfig,
) -> Option<(String, bool, Vec<String>)> {
    let mut target_var: Option<String> = None;
    let mut use_return: Option<bool> = None;
    let mut values = Vec::new();
    for (_pattern, body) in pairs {
        let mut text = body.trim();
        if text.starts_with('{') && text.ends_with('}') && text.len() >= 2 {
            text = text[1..text.len() - 1].trim();
        }
        match parse_set_or_return(text, config) {
            Some(SetOrReturn::Set(var, val)) => {
                match &target_var {
                    None => {
                        target_var = Some(var);
                        use_return = Some(false);
                    }
                    Some(v) if *v != var || use_return == Some(true) => return None,
                    _ => {}
                }
                values.push(val);
            }
            Some(SetOrReturn::Return(val)) => {
                match use_return {
                    None => {
                        use_return = Some(true);
                        target_var = Some("__return__".to_owned());
                    }
                    Some(false) => return None,
                    Some(true) => {}
                }
                values.push(val);
            }
            None => return None,
        }
    }
    let target_var = target_var?;
    if values.is_empty() {
        return None;
    }
    Some((target_var, use_return.unwrap_or(false), values))
}

/// Extract the value from a single arm body.
fn extract_single_value(
    body: &str,
    use_return: bool,
    target_var: &str,
    config: LexerConfig,
) -> Option<String> {
    let mut text = body.trim();
    if text.starts_with('{') && text.ends_with('}') && text.len() >= 2 {
        text = text[1..text.len() - 1].trim();
    }
    match parse_set_or_return(text, config)? {
        SetOrReturn::Return(val) if use_return => Some(val),
        SetOrReturn::Set(var, val) if !use_return && var == target_var => Some(val),
        _ => None,
    }
}

// The output words are explicitly authored proposals, not original source
// inputs. Availability is selected by this Registry; it does not prove their
// current command-table implementation or permit Native insertion/movement.
fn emitted_forms_available(registry: &CommandRegistry) -> bool {
    [
        ["match", "ITEM", "equals", "GROUP"].as_slice(),
        ["lookup", "ITEM", "GROUP"].as_slice(),
    ]
    .iter()
    .all(|arguments| {
        registry
            .resolve_invocation("class", arguments, registry.own_surface_query())
            .is_some_and(|selected| selected.subcommand.is_resolved())
    })
}

// if-chain extraction

/// Extract an if/elseif chain comparing one variable to literals.
#[must_use]
pub fn extract_to_datagroup_from_if(
    source: &str,
    cursor: u32,
    dg_name: &str,
    registry: &CommandRegistry,
    line_index: &LineIndex,
    config: LexerConfig,
) -> Option<Refactoring> {
    if !emitted_forms_available(registry) {
        return None;
    }
    let cmd = find_command_at(source, cursor, Some("if"), registry, config)?;
    let chain = parse_if_chain(&cmd.texts, registry, config)?;
    render_if_extraction(
        source,
        &cmd,
        chain,
        dg_name,
        line_index,
        config,
        || Some(()),
    )
}

fn render_if_extraction(
    source: &str,
    cmd: &SegmentedCommand,
    chain: IfChain,
    dg_name: &str,
    line_index: &LineIndex,
    config: LexerConfig,
    mapping_selected: impl FnOnce() -> Option<()>,
) -> Option<Refactoring> {
    if chain.values.len() < 2 {
        return None;
    }

    let stripped_values: Vec<String> = chain
        .values
        .iter()
        .map(|v| strip_quotes(v).to_owned())
        .collect();
    let value_type = infer_value_type(&stripped_values);
    let dg_name = resolve_dg_name(dg_name, &format!("{}_whitelist", chain.target_var.name()));

    let indent = command_indent(source, cmd, line_index).to_owned();
    let bodies_identical = {
        let set: std::collections::BTreeSet<&str> = chain.bodies.iter().map(|b| b.trim()).collect();
        set.len() <= 1
    };

    let (data_group, replacement) = if bodies_identical && !chain.bodies.is_empty() {
        membership_extraction(
            &stripped_values,
            value_type,
            &dg_name,
            chain.target_var.reference(),
            &chain.bodies[0],
            chain.else_body.as_deref(),
            &indent,
        )
    } else {
        mapping_selected()?;
        let pairs: Vec<(String, String)> = chain
            .values
            .iter()
            .cloned()
            .zip(chain.bodies.iter().cloned())
            .collect();
        let (set_var, use_return, value_entries) = extract_set_or_return_values(&pairs, config)?;
        let records = zip_records(&stripped_values, &value_entries);
        let dg = DataGroupDefinition {
            name: dg_name.clone(),
            value_type: "string".to_owned(),
            records,
        };
        let replacement = if use_return {
            format!(
                "return [class lookup {} {dg_name}]",
                chain.target_var.reference()
            )
        } else {
            format!(
                "set {set_var} [class lookup {} {dg_name}]",
                chain.target_var.reference()
            )
        };
        (dg, replacement)
    };

    complete_single_command_source(&replacement, config)?;
    Some(build_result(
        source,
        cmd,
        &dg_name,
        value_type,
        replacement,
        data_group,
    ))
}

/// Proposed syntax is checked independently of scalar grammar and permissions.
fn complete_single_command_source(source: &str, config: LexerConfig) -> Option<()> {
    let generated = tcl_lexer::native_script_words_in(
        tcl_lexer::SourceImage::document(source),
        tcl_lexer::Span::new(0, u32::try_from(source.len()).ok()?),
        config,
    )
    .ok()?;
    (generated.fatal_tail.is_none() && generated.commands.len() == 1).then_some(())
}

/// A parsed if/elseif equality chain.
struct IfChain {
    target_var: ScalarVariableSourceSyntax,
    values: Vec<String>,
    bodies: Vec<String>,
    else_body: Option<String>,
}

/// Parse the if/elseif chain (OR-chain in a single condition, or an
/// `elseif` ladder) through `if`'s own clause grammar — the condition and
/// body of each clause, and the default (`else`, or its optional-keyword
/// bare final body) — rather than comparing keyword spellings by hand.
fn parse_if_chain(
    texts: &[String],
    registry: &CommandRegistry,
    config: LexerConfig,
) -> Option<IfChain> {
    let args: Vec<&str> = texts.get(1..)?.iter().map(String::as_str).collect();
    let resolved = registry.resolve_call("if", &args, None)?;
    let plan = resolved.clause_plan(&args, None)?;
    if_chain_from_plan(texts, &plan, config)
}

fn if_chain_from_plan(
    texts: &[String],
    plan: &tcl_registry::clause_grammar::ClausePlan,
    config: LexerConfig,
) -> Option<IfChain> {
    let args: Vec<&str> = texts.get(1..)?.iter().map(String::as_str).collect();
    if plan.defect.is_some() {
        return None;
    }

    // OR-chain in a single condition: `if {$x eq "a" || $x eq "b"} {…}` is
    // one clause whose *condition* carries the `||`, which the grammar
    // reads as one opaque `Expr` operand — tried on the head clause first
    // and, when it matches, returned regardless of what follows, exactly
    // as the retired word walk did.
    if let Some(head) = plan.clauses.first()
        && !head.is_default
        && let Some(cond_idx) = head.operand(ArgRole::Expr)
        && let Some(body_idx) = head.operand(ArgRole::Body)
        && let Some((target_var, values)) = try_or_chain(args[cond_idx], config)
    {
        return Some(IfChain {
            target_var,
            values,
            bodies: vec![args[body_idx].to_owned()],
            else_body: None,
        });
    }

    let mut target_var: Option<ScalarVariableSourceSyntax> = None;
    let mut values: Vec<String> = Vec::new();
    let mut bodies: Vec<String> = Vec::new();
    let mut else_body: Option<String> = None;
    for clause in &plan.clauses {
        let body = args[clause.operand(ArgRole::Body)?].to_owned();
        if clause.is_default {
            else_body = Some(body);
            continue;
        }
        let condition = args[clause.operand(ArgRole::Expr)?];
        let (var, value, negated) = parse_eq(condition, config)?;
        if negated {
            return None;
        }
        match &target_var {
            None => target_var = Some(var),
            Some(v) if v.name() != var.name() => return None,
            _ => {}
        }
        values.push(value);
        bodies.push(body);
    }
    Some(IfChain {
        target_var: target_var?,
        values,
        bodies,
        else_body,
    })
}

/// Build a membership-test (`class match`) extraction.
fn membership_extraction(
    stripped_values: &[String],
    value_type: &str,
    dg_name: &str,
    subject_reference: &str,
    body: &str,
    else_body: Option<&str>,
    indent: &str,
) -> (DataGroupDefinition, String) {
    let records: Vec<(String, String)> = stripped_values
        .iter()
        .map(|v| (v.clone(), String::new()))
        .collect();
    let dg = DataGroupDefinition {
        name: dg_name.to_owned(),
        value_type: value_type.to_owned(),
        records,
    };
    let body_text = reindent_body(body, &format!("{indent}    "));
    let replacement = if let Some(eb) = else_body {
        let else_text = reindent_body(eb, &format!("{indent}    "));
        format!(
            "if {{ [class match {subject_reference} equals {dg_name}] }} {{\n{body_text}\n{indent}}} else {{\n{else_text}\n{indent}}}"
        )
    } else {
        format!(
            "if {{ [class match {subject_reference} equals {dg_name}] }} {{\n{body_text}\n{indent}}}"
        )
    };
    (dg, replacement)
}

/// Pair completed keys with stripped body values into data-group records.
fn zip_records(keys: &[String], values: &[String]) -> Vec<(String, String)> {
    keys.iter()
        .zip(values.iter())
        .map(|(k, v)| (k.clone(), strip_quotes(v).to_owned()))
        .collect()
}

/// Resolve a possibly-empty `dg_name` against a generated default.
fn resolve_dg_name(dg_name: &str, default_descriptor: &str) -> String {
    if dg_name.is_empty() {
        normalise_dg_name(default_descriptor)
    } else {
        dg_name.to_owned()
    }
}

/// Indent of the command's first line.
fn command_indent<'a>(
    source: &'a str,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
    line_index: &LineIndex,
) -> &'a str {
    let cmd_line = line_index.line_at(cmd.span.start());
    source
        .split('\n')
        .nth(cmd_line as usize)
        .map_or("", line_indent)
}

/// Assemble the final [`Refactoring`] from the computed replacement.
fn build_result(
    source: &str,
    cmd: &tcl_compiler::segmenter::SegmentedCommand,
    dg_name: &str,
    value_type: &str,
    replacement: String,
    data_group: DataGroupDefinition,
) -> Refactoring {
    let (start, end) = command_span_offsets(source, cmd);
    Refactoring {
        title: format!("Extract to data-group '{dg_name}' ({value_type})"),
        edits: vec![RefactorEdit {
            start,
            end,
            new_text: replacement,
        }],
        kind: ActionKind::RefactorExtract,
        data_group: Some(data_group),
        disabled: None,
    }
}

// switch extraction

/// Extract a switch statement with many literal arms to a data-group.
#[must_use]
pub fn extract_to_datagroup_from_switch(
    source: &str,
    cursor: u32,
    dg_name: &str,
    registry: &CommandRegistry,
    line_index: &LineIndex,
    config: LexerConfig,
) -> Option<Refactoring> {
    if !emitted_forms_available(registry) {
        return None;
    }
    let cmd = find_command_at(source, cursor, Some("switch"), registry, config)?;
    let original = source_subject::standalone_exact_switch_source(source, &cmd, registry, config)?;
    let case_list = registry.get("switch").and_then(|spec| spec.case_list)?;
    render_switch_extraction(
        source,
        &cmd,
        dg_name,
        line_index,
        config,
        SwitchExtraction {
            original,
            default_word: case_list
                .keyword_patterns
                .first()
                .map(|word| (*word).to_owned()),
            fallthrough_word: case_list.fallthrough_body.map(str::to_owned),
        },
        || Some(()),
    )
}

struct SwitchExtraction {
    original: OriginalExactSwitchSource,
    default_word: Option<String>,
    fallthrough_word: Option<String>,
}

fn render_switch_extraction(
    source: &str,
    cmd: &SegmentedCommand,
    dg_name: &str,
    line_index: &LineIndex,
    config: LexerConfig,
    selected: SwitchExtraction,
    mapping_selected: impl FnOnce() -> Option<()>,
) -> Option<Refactoring> {
    let subject = selected.original.subject();
    let pairs = selected.original.pairs();
    if pairs.len() < 3 {
        return None;
    }
    let default_word = selected.default_word.as_deref();
    let fallthrough_word = selected.fallthrough_word.as_deref();

    // Separate default from regular arms.
    let mut default_body: Option<String> = None;
    let mut regular_pairs: Vec<(String, String)> = Vec::new();
    for (pattern, body) in pairs {
        if Some(pattern.as_str()) == default_word {
            default_body = Some(body.clone());
        } else if fallthrough_word.is_some_and(|marker| body.trim() == marker) {
            return None; // fallthrough — can't map
        } else {
            regular_pairs.push((pattern.clone(), body.clone()));
        }
    }
    if regular_pairs.len() < 3 {
        return None;
    }

    let keys: Vec<String> = regular_pairs.iter().map(|(p, _)| p.clone()).collect();
    let value_type = infer_value_type(&keys);
    let dg_name = resolve_dg_name(dg_name, &format!("{}_map", subject.name()));
    let indent = command_indent(source, cmd, line_index).to_owned();

    let all_same = {
        let set: std::collections::BTreeSet<&str> =
            regular_pairs.iter().map(|(_, b)| b.trim()).collect();
        set.len() == 1
    };

    let (data_group, replacement) = if all_same {
        membership_extraction(
            &keys,
            value_type,
            &dg_name,
            subject.reference(),
            &regular_pairs[0].1,
            default_body.as_deref(),
            &indent,
        )
    } else {
        mapping_selected()?;
        switch_mapping_extraction(
            &regular_pairs,
            &keys,
            subject.reference(),
            &dg_name,
            default_body.as_deref(),
            &indent,
            config,
        )?
    };

    complete_single_command_source(&replacement, config)?;
    Some(build_result(
        source,
        cmd,
        &dg_name,
        value_type,
        replacement,
        data_group,
    ))
}

/// Build the value-mapping (`class lookup`) extraction for a switch.
fn switch_mapping_extraction(
    regular_pairs: &[(String, String)],
    keys: &[String],
    subject_reference: &str,
    dg_name: &str,
    default_body: Option<&str>,
    indent: &str,
    config: LexerConfig,
) -> Option<(DataGroupDefinition, String)> {
    let (target_var, use_return, value_entries) =
        extract_set_or_return_values(regular_pairs, config)?;
    let records = zip_records(keys, &value_entries);
    let dg = DataGroupDefinition {
        name: dg_name.to_owned(),
        value_type: "string".to_owned(),
        records,
    };
    let replacement = if let Some(default) = default_body {
        // A default that is not a clean value mapping in the same form/target
        // (it sets a *different* variable, or runs a command) cannot be
        // represented as the `else` value without changing semantics — e.g.
        // `default { set other zzz }` would become `set h { set other zzz }`,
        // assigning the literal string instead of running the command. Decline
        // the refactor rather than emit a lossy, semantics-changing edit.
        let default_val = extract_single_value(default, use_return, &target_var, config)?;
        if use_return {
            format!(
                "if {{ [class match {subject_reference} equals {dg_name}] }} {{\n{indent}    return [class lookup {subject_reference} {dg_name}]\n{indent}}} else {{\n{indent}    return {default_val}\n{indent}}}"
            )
        } else {
            format!(
                "if {{ [class match {subject_reference} equals {dg_name}] }} {{\n{indent}    set {target_var} [class lookup {subject_reference} {dg_name}]\n{indent}}} else {{\n{indent}    set {target_var} {default_val}\n{indent}}}"
            )
        }
    } else if use_return {
        format!("return [class lookup {subject_reference} {dg_name}]")
    } else {
        format!("set {target_var} [class lookup {subject_reference} {dg_name}]")
    };
    Some((dg, replacement))
}

/// Try all static extraction patterns at the cursor — if/elseif first,
/// then switch.
#[must_use]
pub fn extract_to_datagroup(
    source: &str,
    cursor: u32,
    dg_name: &str,
    registry: &CommandRegistry,
    line_index: &LineIndex,
    config: LexerConfig,
) -> Option<Refactoring> {
    extract_to_datagroup_from_if(source, cursor, dg_name, registry, line_index, config).or_else(
        || extract_to_datagroup_from_switch(source, cursor, dg_name, registry, line_index, config),
    )
}

/// Compatibility proposal from a complete current Logical document. The
/// selected source descriptor supplies layout only; Native insertion, movement
/// and execution permissions remain outside this authoring API.
#[must_use]
pub fn extract_to_datagroup_with_analysis(
    source: &str,
    cursor: u32,
    dg_name: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Option<Refactoring> {
    if !analysis.allows_retained_logical_declaration_advice() {
        return None;
    }
    let current = crate::original_context::CurrentSourceContext::capture(source, analysis)?;
    let walk = super::FrameWalk::new(source, analysis)?;
    let cmd = super::find_original_command_at(source, cursor, analysis)?;
    let words = walk.source_words(source, &cmd)?;
    // These proposal parsers retain written clause/arm spelling. An alias may
    // rename the head; captured operands cannot borrow those written positions.
    if !written_operands_match(&words)
        || !selected_output_forms_available(analysis, current.registry(), cmd.span.start())
    {
        return None;
    }
    let hook =
        words.with_source_schema(&current.context(), |schema| schema.semantics.lowering_hook)??;
    let mapping = || original_mapping_arms(&walk, source, &cmd);
    match hook {
        tcl_registry::hooks::LoweringHookId::If => {
            let plan =
                words.with_source_schema(&current.context(), |schema| schema.clause_plan())??;
            let chain = if_chain_from_plan(&cmd.texts, &plan, current.config())?;
            render_if_extraction(
                source,
                &cmd,
                chain,
                dg_name,
                line_index,
                current.config(),
                mapping,
            )
        }
        tcl_registry::hooks::LoweringHookId::Switch => {
            let original =
                original_exact_switch_source_at_analysis(source, analysis, cmd.span.start())?;
            let (default_word, fallthrough_word) =
                words.with_source_schema(&current.context(), |schema| {
                    let cases = schema.semantics.options.case_list?;
                    Some((
                        cases
                            .keyword_patterns
                            .first()
                            .map(|word| (*word).to_owned()),
                        cases.fallthrough_body.map(str::to_owned),
                    ))
                })??;
            render_switch_extraction(
                source,
                &cmd,
                dg_name,
                line_index,
                current.config(),
                SwitchExtraction {
                    original,
                    default_word,
                    fallthrough_word,
                },
                mapping,
            )
        }
        _ => None,
    }
}

fn written_operands_match(
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
) -> bool {
    words.origins().iter().enumerate().skip(1).all(|(index, origin)| {
        matches!(origin, tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written) if *written == index)
    })
}

/// Newly authored output words have no original invocation to project. This
/// positive Logical query retains the current descriptor at the proposal
/// horizon, independently of output execution or insertion permissions.
fn selected_proposed_spec(
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    at: u32,
    name: &str,
) -> Option<&'static tcl_registry::CommandSpec> {
    if !analysis.allows_retained_logical_declaration_advice() {
        return None;
    }
    let context = analysis.resolved_input.as_ref()?.availability_context();
    let expected = context.resolve_spec(registry, name)?;
    let retained = match analysis.retained_command_realm()?.binding_at(name, at) {
        tcl_compiler::realm::RealmBindingFact::Unchanged => true,
        tcl_compiler::realm::RealmBindingFact::Command(target) => context
            .resolve_spec(registry, target)
            .is_some_and(|selected| std::ptr::eq(selected, expected)),
        tcl_compiler::realm::RealmBindingFact::Rebound => false,
    };
    retained.then_some(expected)
}

fn selected_output_forms_available(
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    at: u32,
) -> bool {
    let Some(input) = analysis.resolved_input.as_ref() else {
        return false;
    };
    if selected_proposed_spec(analysis, registry, at, "class").is_none()
        || !selected_proposed_spec(analysis, registry, at, "if").is_some_and(|selected| {
            selected.lowering_hook == Some(tcl_registry::hooks::LoweringHookId::If)
        })
    {
        return false;
    }
    [
        ["match", "ITEM", "equals", "GROUP"].as_slice(),
        ["lookup", "ITEM", "GROUP"].as_slice(),
    ]
    .iter()
    .all(|arguments| {
        tcl_registry::model::resolve_invocation_in_context(
            registry,
            Some(input.availability_context()),
            "class",
            arguments,
        )
        .is_some_and(|selected| selected.subcommand.is_resolved())
    })
}

/// Mapping syntax may name a primitive only after every genuine original arm
/// selects that operation at its own horizon. Matching text is insufficient.
fn original_mapping_arms(
    walk: &super::FrameWalk<'_>,
    source: &str,
    command: &SegmentedCommand,
) -> Option<()> {
    let regions = walk.same_frame_regions(source, command);
    if regions.is_empty() || !walk.complete() {
        return None;
    }
    for (start, end) in regions {
        let commands = walk.segment(source.get(start..end)?, u32::try_from(start).ok()?);
        let [command] = commands.as_slice() else {
            return None;
        };
        let words = walk.source_words(source, command)?;
        if !written_operands_match(&words) {
            return None;
        }
        let hook = words.with_source_schema(&walk.source_context(), |schema| {
            schema.semantics.lowering_hook
        })??;
        match (hook, words.arguments().len()) {
            (tcl_registry::hooks::LoweringHookId::Set, 2)
            | (tcl_registry::hooks::LoweringHookId::Return, 1) => {}
            _ => return None,
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn selected_if_scalar_source_keeps_reference_spelling_in_proposals() {
        // Implementation contract: naming.refactor.selected-if-scalar-source-syntax
        // docs/design/analysis/name-resolution-proofs/selected-if-scalar-source-syntax.md
        for subject in [
            "${a b}",
            "\"${café}\"",
            "${literal$name}",
            "${a(k)tail}",
            r"${a\b}",
        ] {
            let source = format!(
                "if {{{subject} eq \"a\"}} {{drop}} elseif {{{subject} eq \"b\"}} {{drop}}"
            );
            let action = if_dg(&source, "subjects").expect("selected scalar membership syntax");
            let proposed = action.apply(&source);
            assert!(
                proposed.contains(&format!("class match {subject} equals subjects")),
                "{proposed}"
            );
            let source = format!(
                "if {{{subject} eq \"a\"}} {{return one}} elseif {{{subject} eq \"b\"}} {{return two}}"
            );
            let proposed = if_dg(&source, "subjects")
                .expect("selected scalar mapping syntax")
                .apply(&source);
            assert_eq!(
                proposed,
                format!("return [class lookup {subject} subjects]")
            );
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
            assert!(if_dg(&source, "subjects").is_none(), "{subject:?}");
        }
    }

    #[test]
    fn selected_if_scalar_conditions_keep_eq_and_or_shape_under_actual_config() {
        // Implementation contract: naming.refactor.selected-if-scalar-source-syntax
        // docs/design/analysis/name-resolution-proofs/selected-if-scalar-source-syntax.md
        let first = LexerConfig::for_dialect("tcl8.6");
        let nested = LexerConfig::for_dialect("tcl9.1");
        let jim = LexerConfig::for_dialect("jim");
        assert!(parse_eq("${a{b}c} eq one", first).is_none());
        assert_eq!(
            parse_eq("${a{b}c} eq one", nested).unwrap().0.name(),
            "a{b}c"
        );
        assert!(parse_eq("$café eq one", first).is_none());
        assert_eq!(parse_eq("one eq $café", jim).unwrap().0.name(), "café");
        let (subject, values) = try_or_chain("${a b} eq one || \"${a b}\" eq two", first).unwrap();
        assert_eq!(subject.name(), "a b");
        assert_eq!(subject.reference(), "${a b}");
        assert_eq!(values, ["one", "two"]);
        assert!(try_or_chain("$x eq one || $y eq two", first).is_none());
        assert!(try_or_chain("$x eq one || !($x eq two)", first).is_none());
        for subject in ["$a(k)", "${a(k)}", "${missing", "\"$x[set y]\""] {
            assert!(parse_eq(&format!("{subject} eq one"), first).is_none());
        }
    }

    use super::*;
    use tcl_dialect::model::{Family, SurfaceLayer};

    fn reg() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
        r
    }

    /// This module's transforms only ever fire meaningfully over iRules
    /// text (`class match` / `class lookup` are iRules-only), so the tests
    /// lex under the iRules grammar, matching `reg()`.
    fn config() -> LexerConfig {
        LexerConfig::from_grammar(tcl_registry::model::resolve_environment("f5-irules").grammar())
    }

    fn if_dg(source: &str, name: &str) -> Option<Refactoring> {
        let r = reg();
        let li = LineIndex::new(source);
        extract_to_datagroup_from_if(source, 0, name, &r, &li, config())
    }

    fn switch_dg(source: &str, name: &str) -> Option<Refactoring> {
        let r = reg();
        let li = LineIndex::new(source);
        extract_to_datagroup_from_switch(source, 0, name, &r, &li, config())
    }

    fn authored_output_registry() -> std::sync::Arc<CommandRegistry> {
        // An independently authored source surface, not an F5 execution model.
        const OUTPUTS: &[tcl_registry::SubCommand] = &[
            tcl_registry::SubCommand {
                name: "match",
                arity: tcl_registry::Arity::exact(3),
                ..tcl_registry::SubCommand::DEFAULT
            },
            tcl_registry::SubCommand {
                name: "lookup",
                arity: tcl_registry::Arity::exact(2),
                ..tcl_registry::SubCommand::DEFAULT
            },
        ];
        let mut registry = CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "class",
            surface: Some(tcl_registry::model::SpecSurface::TCL90_PLUS),
            subcommands: OUTPUTS,
            ..tcl_registry::CommandSpec::DEFAULT
        });
        std::sync::Arc::new(registry)
    }

    #[test]
    fn supplied_datagroup_advice_keeps_actual_output_availability_and_arm_horizons() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let source = "if {$x eq one} {set answer FIRST} elseif {$x eq two} {set answer SECOND}";
        let registry = authored_output_registry();
        let analysis = super::super::test_logical_analysis(source, "tcl9.0", registry.clone());
        let index = LineIndex::new(source);
        let action = extract_to_datagroup_with_analysis(source, 0, "answers", &analysis, &index)
            .expect("current authored output metadata and genuine original arm operations");
        assert_eq!(action.apply(source), "set answer [class lookup $x answers]");
        assert_eq!(dg(&action).records.len(), 2);
        let older = super::super::test_logical_analysis(source, "tcl8.4", registry.clone());
        assert!(extract_to_datagroup_with_analysis(source, 0, "answers", &older, &index).is_none());
        for prefix in ["proc set args {}; ", "proc class args {}; "] {
            let replaced = format!("{prefix}{source}");
            let analysis =
                super::super::test_logical_analysis(&replaced, "tcl9.0", registry.clone());
            let cursor = u32::try_from(prefix.len()).unwrap();
            assert!(
                extract_to_datagroup_with_analysis(
                    &replaced,
                    cursor,
                    "answers",
                    &analysis,
                    &LineIndex::new(&replaced)
                )
                .is_none(),
                "{prefix}"
            );
        }
    }

    #[test]
    fn supplied_datagroup_advice_keeps_written_clauses_and_current_document_owner() {
        // naming.core.original-dispatch-region-context
        // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
        let tail = "choose {$x eq one} {puts SAME} elseif {$x eq two} {puts SAME}";
        let source = format!("interp alias {{}} choose {{}} if\n{tail}");
        let registry = authored_output_registry();
        let analysis = super::super::test_logical_analysis(&source, "tcl9.0", registry.clone());
        let cursor = u32::try_from(source.rfind("choose").unwrap()).unwrap();
        let index = LineIndex::new(&source);
        assert!(
            extract_to_datagroup_with_analysis(&source, cursor, "answers", &analysis, &index)
                .is_some()
        );
        let captured = "interp alias {} choose {} if {$x eq one}\nchoose {puts SAME} elseif {$x eq two} {puts SAME}";
        let captured_analysis = super::super::test_logical_analysis(captured, "tcl9.0", registry);
        assert!(
            extract_to_datagroup_with_analysis(
                captured,
                u32::try_from(captured.rfind("choose").unwrap()).unwrap(),
                "answers",
                &captured_analysis,
                &LineIndex::new(captured)
            )
            .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let mut stale = analysis.clone();
        stale.body_lexer_config.as_mut().unwrap().strict_quoting =
            !analysis.body_lexer_config.unwrap().strict_quoting;
        let mut foreign = analysis.clone();
        let input = analysis.resolved_input.as_ref().unwrap();
        foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            input.analyser_profile(),
            input.unit_profile(),
            tcl_registry::model::resolve_environment("tcl9.0").default_context_registry(),
            input.lexer_config(),
        ));
        for unavailable in [missing, stale, foreign] {
            assert!(
                extract_to_datagroup_with_analysis(
                    &source,
                    cursor,
                    "answers",
                    &unavailable,
                    &index
                )
                .is_none()
            );
        }
        assert!(
            extract_to_datagroup_with_analysis("# stale", cursor, "answers", &analysis, &index)
                .is_none()
        );
    }

    #[test]
    fn supplied_datagroup_alias_does_not_restore_a_removed_output_handler() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let source = "rename if saved_if\ninterp alias {} choose {} saved_if\nchoose {$x eq one} {puts SAME} elseif {$x eq two} {puts SAME}";
        let registry = authored_output_registry();
        let analysis = super::super::test_logical_analysis(source, "tcl9.0", registry);
        let cursor = u32::try_from(source.rfind("choose").unwrap()).unwrap();
        let walk = super::super::FrameWalk::new(source, &analysis).unwrap();
        let command = super::super::find_original_command_at(source, cursor, &analysis).unwrap();
        assert_eq!(
            walk.source_words(source, &command)
                .unwrap()
                .with_source_schema(&walk.source_context(), |schema| schema
                    .semantics
                    .lowering_hook),
            Some(Some(tcl_registry::hooks::LoweringHookId::If))
        );
        assert!(
            extract_to_datagroup_with_analysis(
                source,
                cursor,
                "answers",
                &analysis,
                &LineIndex::new(source)
            )
            .is_none()
        );
        for dialect in ["tcl9.0", "f5-irules"] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
            assert!(!analysis.allows_retained_logical_declaration_advice());
            assert!(
                extract_to_datagroup_with_analysis(
                    source,
                    cursor,
                    "answers",
                    &analysis,
                    &LineIndex::new(source)
                )
                .is_none()
            );
        }
    }

    fn dg(r: &Refactoring) -> &DataGroupDefinition {
        r.data_group.as_ref().expect("data group")
    }

    #[test]
    fn original_datagroup_proposals_require_the_selected_output_surface() {
        let source = "if {$host eq \"a.com\"} {pool web_pool} elseif {$host eq \"b.com\"} {pool web_pool} elseif {$host eq \"c.com\"} {pool web_pool}";
        let index = LineIndex::new(source);
        for (dialect, available) in [("tcl8.6", false), ("f5-irules", true)] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, dialect);
            let registry = analysis.resolved_registry().unwrap();
            assert_eq!(emitted_forms_available(registry), available, "{dialect}");
            assert_eq!(
                extract_to_datagroup(
                    source,
                    0,
                    "",
                    registry,
                    &index,
                    analysis.body_lexer_config.unwrap()
                )
                .is_some(),
                available,
                "{dialect}"
            );
        }
    }

    #[test]
    fn if_chain_string_membership() {
        let source = "if {$host eq \"a.com\"} {\n    pool web_pool\n} elseif {$host eq \"b.com\"} {\n    pool web_pool\n} elseif {$host eq \"c.com\"} {\n    pool web_pool\n}";
        let r = if_dg(source, "allowed_hosts").expect("result");
        let g = dg(&r);
        assert_eq!(g.value_type, "string");
        assert_eq!(g.name, "allowed_hosts");
        assert_eq!(g.records.len(), 3);
        assert!(g.records.contains(&("a.com".to_owned(), String::new())));
        let tcl = data_group_tcl(g);
        assert!(tcl.contains("type string"));
        let applied = r.apply(source);
        assert!(applied.contains("class match"), "{applied:?}");
        assert!(applied.contains("allowed_hosts"));
    }

    #[test]
    fn if_chain_ip_addresses() {
        let source = "if {$addr eq \"10.0.0.0/8\"} {\n    drop\n} elseif {$addr eq \"172.16.0.0/12\"} {\n    drop\n} elseif {$addr eq \"192.168.0.0/16\"} {\n    drop\n}";
        let r = if_dg(source, "rfc1918").expect("result");
        let g = dg(&r);
        assert_eq!(g.value_type, "ip");
        let tcl = data_group_tcl(g);
        assert!(tcl.contains("type ip"));
        assert!(tcl.contains("10.0.0.0/8"));
    }

    #[test]
    fn if_chain_ipv6_addresses() {
        let source = "if {$addr eq \"2001:db8::/32\"} {\n    drop\n} elseif {$addr eq \"fd00::/8\"} {\n    drop\n} elseif {$addr eq \"fe80::1\"} {\n    drop\n}";
        let r = if_dg(source, "blocked_v6").expect("result");
        let g = dg(&r);
        assert_eq!(g.value_type, "ip");
        let tcl = data_group_tcl(g);
        assert!(tcl.contains("2001:db8::/32"));
        assert!(tcl.contains("fd00::/8"));
        assert!(tcl.contains("fe80::1"));
    }

    #[test]
    fn if_chain_integers_membership() {
        let source = "if {$port eq \"80\"} {\n    log local0. \"http\"\n} elseif {$port eq \"443\"} {\n    log local0. \"http\"\n} elseif {$port eq \"8080\"} {\n    log local0. \"http\"\n}";
        let r = if_dg(source, "http_ports").expect("result");
        assert_eq!(dg(&r).value_type, "integer");
    }

    #[test]
    fn if_chain_value_mapping() {
        let source = "if {$port eq \"80\"} {\n    set proto http\n} elseif {$port eq \"443\"} {\n    set proto https\n} elseif {$port eq \"8080\"} {\n    set proto http\n}";
        let r = if_dg(source, "port_map").expect("result");
        assert_eq!(dg(&r).value_type, "string");
    }

    #[test]
    fn switch_membership() {
        let source = "switch -exact -- $ext {\n    .jpg { set type image }\n    .png { set type image }\n    .gif { set type image }\n    .svg { set type image }\n}";
        let r = switch_dg(source, "image_types").expect("result");
        let g = dg(&r);
        assert_eq!(g.value_type, "string");
        assert_eq!(g.records.len(), 4);
    }

    #[test]
    fn switch_value_mapping() {
        let source = "switch -exact -- $method {\n    GET { set handler handle_get }\n    POST { set handler handle_post }\n    PUT { set handler handle_put }\n    DELETE { set handler handle_delete }\n}";
        let r = switch_dg(source, "method_handler_map").expect("result");
        assert!(dg(&r).records.iter().any(|(_, v)| !v.is_empty()));
    }

    #[test]
    fn or_chain_detection() {
        let source = "if {$host eq \"a.com\" || $host eq \"b.com\" || $host eq \"c.com\"} {\n    pool web_pool\n}";
        let r = if_dg(source, "allowed").expect("result");
        assert_eq!(dg(&r).records.len(), 3);
    }

    #[test]
    fn too_few_values_returns_none() {
        assert!(if_dg("if {$x eq \"a\"} { puts ok }", "").is_none());
    }

    #[test]
    fn unified_dispatch_tries_switch() {
        let source = "switch -exact -- $uri {\n    /api { set pool api_pool }\n    /web { set pool web_pool }\n    /cdn { set pool cdn_pool }\n}";
        let r = reg();
        let li = LineIndex::new(source);
        assert!(extract_to_datagroup(source, 0, "uri_pool", &r, &li, config()).is_some());
    }

    #[test]
    fn data_group_tcl_rendering() {
        let source = "if {$host eq \"a.com\"} {\n    pool web_pool\n} elseif {$host eq \"b.com\"} {\n    pool web_pool\n}";
        let r = if_dg(source, "hosts").expect("result");
        let tcl = data_group_tcl(dg(&r));
        assert!(tcl.contains("ltm data-group internal hosts"));
        assert!(tcl.contains("records"));
        assert!(tcl.contains("a.com"));
        assert!(tcl.contains("b.com"));
    }

    #[test]
    fn if_rewrite_covers_entire_command() {
        let source = "if {$host eq \"a.com\"} {\n    pool web_pool\n} elseif {$host eq \"b.com\"} {\n    pool web_pool\n}";
        let r = if_dg(source, "hosts").expect("result");
        let applied = r.apply(source);
        // Verify the full applied output (apply + tmsh).
        assert_eq!(
            applied,
            "if { [class match $host equals hosts] } {\n    pool web_pool\n}"
        );
        assert_eq!(
            data_group_tcl(dg(&r)),
            "ltm data-group internal hosts {\n    records {\n        a.com { }\n        b.com { }\n    }\n    type string\n}"
        );
        let trimmed = applied.trim();
        assert!(trimmed.ends_with('}'));
        assert!(!trimmed.ends_with("}}"), "{trimmed:?}");
    }

    #[test]
    fn inside_when_body() {
        let source = "when HTTP_REQUEST {\n    if {$host eq \"a.com\"} {\n        pool web_pool\n    } elseif {$host eq \"b.com\"} {\n        pool web_pool\n    } elseif {$host eq \"c.com\"} {\n        pool web_pool\n    }\n}";
        let r = reg();
        let li = LineIndex::new(source);
        let cursor = u32::try_from(source.find("if {").unwrap()).unwrap();
        let res =
            extract_to_datagroup(source, cursor, "", &r, &li, config()).expect("nested result");
        let g = dg(&res);
        assert_eq!(g.value_type, "string");
        assert_eq!(g.records.len(), 3);
    }

    /// Stock Tcl accepts an optional `then` between a condition and its
    /// body (`if {$x} then {…}`), including after each `elseif`.
    #[test]
    fn if_chain_then_keyword() {
        let source = "if {$host eq \"a.com\"} then {\n    pool web_pool\n} elseif {$host eq \"b.com\"} then {\n    pool web_pool\n} elseif {$host eq \"c.com\"} then {\n    pool web_pool\n}";
        let r = if_dg(source, "allowed_hosts").expect("result");
        let g = dg(&r);
        assert_eq!(g.value_type, "string");
        assert_eq!(g.records.len(), 3);
        assert!(g.records.contains(&("b.com".to_owned(), String::new())));
        let applied = r.apply(source);
        assert_eq!(
            applied,
            "if { [class match $host equals allowed_hosts] } {\n    pool web_pool\n}"
        );
    }

    /// `then` on the leading arm only, with a trailing `else`.
    #[test]
    fn if_chain_then_keyword_with_else() {
        let source = "if {$host eq \"a.com\"} then {\n    pool a_pool\n} elseif {$host eq \"b.com\"} {\n    pool a_pool\n} else {\n    pool default_pool\n}";
        let r = if_dg(source, "hosts").expect("result");
        let g = dg(&r);
        assert_eq!(g.records.len(), 2);
        let applied = r.apply(source);
        assert!(applied.contains("pool a_pool"), "{applied:?}");
        assert!(applied.contains("pool default_pool"), "{applied:?}");
        assert!(!applied.contains("then"), "{applied:?}");
    }

    /// The OR-chain shortcut reads the body straight off the word after the
    /// condition, so it has to skip `then` too — otherwise it rewrites the
    /// command with `then` as the body and drops the real one.
    #[test]
    fn or_chain_then_keyword() {
        let source = "if {$host eq \"a.com\" || $host eq \"b.com\" || $host eq \"c.com\"} then {\n    pool web_pool\n}";
        let r = if_dg(source, "allowed").expect("result");
        assert_eq!(dg(&r).records.len(), 3);
        let applied = r.apply(source);
        assert_eq!(
            applied,
            "if { [class match $host equals allowed] } {\n    pool web_pool\n}"
        );
    }

    /// `infer_value_type` is handed values its callers have already run
    /// through `strip_quotes`, so a value that still carries quotes is a
    /// string literal, not a number.  (Before the fix a second strip here
    /// peeled that pair off and typed the group `integer`, disagreeing with
    /// the record actually written out.)
    #[test]
    fn infer_value_type_does_not_restrip() {
        assert_eq!(
            infer_value_type(&["80".to_owned(), "443".to_owned()]),
            "integer"
        );
        assert_eq!(
            infer_value_type(&["\"80\"".to_owned(), "\"443\"".to_owned()]),
            "string"
        );
        assert_eq!(
            infer_value_type(&["10.0.0.0/8".to_owned(), "192.168.0.0/16".to_owned()]),
            "ip"
        );
        assert_eq!(
            infer_value_type(&["\"10.0.0.0/8\"".to_owned(), "\"192.168.0.0/16\"".to_owned()]),
            "string"
        );
        // Whitespace inside a quoted literal survives the caller's strip;
        // the predicates still tolerate it.
        assert_eq!(infer_value_type(&[" 80 ".to_owned()]), "integer");
        assert_eq!(infer_value_type(&[" 10.0.0.0/8 ".to_owned()]), "ip");
    }
}
