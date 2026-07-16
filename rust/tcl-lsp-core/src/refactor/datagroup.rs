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

use tcl_compiler::segmenter::segment_commands_with_offset;
use tcl_lexer::LineIndex;
use tcl_registry::CommandRegistry;

use super::{
    RefactorEdit, Refactoring, command_span_offsets, find_command_at, line_indent, reindent_body,
    token_end_offset,
};
use crate::code_actions::ActionKind;

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
fn is_ip_or_cidr(value: &str) -> bool {
    let v = strip_quotes(value);
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
    let v = strip_quotes(value).trim();
    !v.is_empty() && v.parse::<i64>().is_ok()
}

/// Infer the data-group value type from a list of keys/values.
fn infer_value_type(values: &[String]) -> &'static str {
    if values.is_empty() {
        return "string";
    }
    if values.iter().all(|v| is_ip_or_cidr(strip_quotes(v))) {
        return "ip";
    }
    if values.iter().all(|v| is_integer(strip_quotes(v))) {
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
const EQ_OPS: &[&str] = &["==", "!=", "eq", "ne"];

/// Parse a simple equality test `(var, value, negated)`.  Used by the
/// datagroup transform; mirrors the shape of
/// `if_to_switch::parse_eq_test` minus the operator capture.
fn parse_eq(cond: &str) -> Option<(String, String, bool)> {
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
            && let Some(var) = parse_var_word(cond[..pos].trim())
        {
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
            && let Some(var) = parse_var_word(cond[pos + needle.len()..].trim())
        {
            let is_ne = *op == "ne" || *op == "!=";
            return Some((var, cond[..pos].trim().to_owned(), negated ^ is_ne));
        }
    }
    None
}

/// Parse `$var` / `${var}` / `"$var"` to its bare name.
fn parse_var_word(word: &str) -> Option<String> {
    let mut w = word.trim();
    if w.len() >= 2 && w.starts_with('"') && w.ends_with('"') {
        w = &w[1..w.len() - 1];
    }
    let w = w.strip_prefix('$')?;
    let w = w.strip_prefix('{').unwrap_or(w);
    let w = w.strip_suffix('}').unwrap_or(w);
    if !w.is_empty() && w.chars().all(|c| c.is_alphanumeric() || c == '_') {
        Some(w.to_owned())
    } else {
        None
    }
}

/// Detect `$var eq "a" || $var eq "b" || …` chains.
fn try_or_chain(condition: &str) -> Option<(String, Vec<String>)> {
    let mut cond = condition.trim();
    if cond.starts_with('{') && cond.ends_with('}') && cond.len() >= 2 {
        cond = cond[1..cond.len() - 1].trim();
    }
    let parts: Vec<&str> = cond.split("||").collect();
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

/// Extract a variable name from `$var` / `${var}` subject.
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
            .is_some_and(|c| c.is_alphabetic() || c == '_')
        && name.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return Some(name.to_owned());
    }
    None
}

// set/return body parsing

/// Parsed single-command arm body.
enum SetOrReturn {
    Set(String, String),
    Return(String),
}

/// Parse a single-command arm body via the tokeniser.
fn parse_set_or_return(text: &str) -> Option<SetOrReturn> {
    let commands = segment_commands_with_offset(text, 0);
    if commands.len() != 1 || commands[0].texts.is_empty() {
        return None;
    }
    let cmd = &commands[0];
    let raw = |index: usize| -> String {
        let tok = cmd.argv[index];
        text[tok.span.start() as usize..token_end_offset(text, tok) as usize].to_owned()
    };
    if cmd.texts[0] == "set" && cmd.texts.len() == 3 {
        return Some(SetOrReturn::Set(cmd.texts[1].clone(), raw(2)));
    }
    if cmd.texts[0] == "return" && cmd.texts.len() == 2 {
        return Some(SetOrReturn::Return(raw(1)));
    }
    None
}

/// Extract `(target_var, use_return, values)` from arm bodies.
fn extract_set_or_return_values(pairs: &[(String, String)]) -> Option<(String, bool, Vec<String>)> {
    let mut target_var: Option<String> = None;
    let mut use_return: Option<bool> = None;
    let mut values = Vec::new();
    for (_pattern, body) in pairs {
        let mut text = body.trim();
        if text.starts_with('{') && text.ends_with('}') && text.len() >= 2 {
            text = text[1..text.len() - 1].trim();
        }
        match parse_set_or_return(text) {
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
fn extract_single_value(body: &str, use_return: bool, target_var: &str) -> Option<String> {
    let mut text = body.trim();
    if text.starts_with('{') && text.ends_with('}') && text.len() >= 2 {
        text = text[1..text.len() - 1].trim();
    }
    match parse_set_or_return(text)? {
        SetOrReturn::Return(val) if use_return => Some(val),
        SetOrReturn::Set(var, val) if !use_return && var == target_var => Some(val),
        _ => None,
    }
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
) -> Option<Refactoring> {
    let cmd = find_command_at(source, cursor, Some("if"), registry)?;
    let chain = parse_if_chain(&cmd.texts)?;
    if chain.values.len() < 2 {
        return None;
    }

    let stripped_values: Vec<String> = chain
        .values
        .iter()
        .map(|v| strip_quotes(v).to_owned())
        .collect();
    let value_type = infer_value_type(&stripped_values);
    let dg_name = resolve_dg_name(dg_name, &format!("{}_whitelist", chain.target_var));

    let indent = command_indent(source, &cmd, line_index).to_owned();
    let bodies_identical = {
        let set: std::collections::BTreeSet<&str> = chain.bodies.iter().map(|b| b.trim()).collect();
        set.len() <= 1
    };

    let (data_group, replacement) = if bodies_identical && !chain.bodies.is_empty() {
        membership_extraction(
            &stripped_values,
            value_type,
            &dg_name,
            &chain.target_var,
            &chain.bodies[0],
            chain.else_body.as_deref(),
            &indent,
        )
    } else {
        let pairs: Vec<(String, String)> = chain
            .values
            .iter()
            .cloned()
            .zip(chain.bodies.iter().cloned())
            .collect();
        let (set_var, use_return, value_entries) = extract_set_or_return_values(&pairs)?;
        let records = zip_records(&stripped_values, &value_entries);
        let dg = DataGroupDefinition {
            name: dg_name.clone(),
            value_type: "string".to_owned(),
            records,
        };
        let replacement = if use_return {
            format!("return [class lookup ${} {dg_name}]", chain.target_var)
        } else {
            format!(
                "set {set_var} [class lookup ${} {dg_name}]",
                chain.target_var
            )
        };
        (dg, replacement)
    };

    Some(build_result(
        source,
        &cmd,
        &dg_name,
        value_type,
        replacement,
        data_group,
    ))
}

/// A parsed if/elseif equality chain.
struct IfChain {
    target_var: String,
    values: Vec<String>,
    bodies: Vec<String>,
    else_body: Option<String>,
}

/// Parse the if/elseif chain (OR-chain in a single condition, or an
/// `elseif` ladder).
fn parse_if_chain(texts: &[String]) -> Option<IfChain> {
    // OR-chain in a single condition.
    if texts.len() >= 3
        && let Some((target_var, values)) = try_or_chain(&texts[1])
    {
        return Some(IfChain {
            target_var,
            values,
            bodies: vec![texts[2].clone()],
            else_body: None,
        });
    }

    let mut target_var: Option<String> = None;
    let mut values: Vec<String> = Vec::new();
    let mut bodies: Vec<String> = Vec::new();
    let mut else_body: Option<String> = None;
    let mut i = 1;
    while i < texts.len() {
        let word = &texts[i];
        if word == "elseif" || word == "then" {
            i += 1;
            continue;
        }
        if word == "else" {
            if i + 1 < texts.len() {
                else_body = Some(texts[i + 1].clone());
            }
            break;
        }
        if i + 1 >= texts.len() {
            return None;
        }
        let (var, value, negated) = parse_eq(word)?;
        let body = texts[i + 1].clone();
        i += 2;
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
    target_var: &str,
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
            "if {{ [class match ${target_var} equals {dg_name}] }} {{\n{body_text}\n{indent}}} else {{\n{else_text}\n{indent}}}"
        )
    } else {
        format!("if {{ [class match ${target_var} equals {dg_name}] }} {{\n{body_text}\n{indent}}}")
    };
    (dg, replacement)
}

/// Pair stripped keys with stripped values into data-group records.
fn zip_records(keys: &[String], values: &[String]) -> Vec<(String, String)> {
    keys.iter()
        .zip(values.iter())
        .map(|(k, v)| (strip_quotes(k).to_owned(), strip_quotes(v).to_owned()))
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
) -> Option<Refactoring> {
    let cmd = find_command_at(source, cursor, Some("switch"), registry)?;
    let (subject, pairs) = super::parse_exact_switch(&cmd.texts)?;
    let subject_var = extract_var_name(&subject)?;
    if pairs.len() < 3 {
        return None;
    }

    // Separate default from regular arms.
    let mut default_body: Option<String> = None;
    let mut regular_pairs: Vec<(String, String)> = Vec::new();
    for (pattern, body) in &pairs {
        if pattern == "default" {
            default_body = Some(body.clone());
        } else if body.trim() == "-" {
            return None; // fallthrough — can't map
        } else {
            regular_pairs.push((pattern.clone(), body.clone()));
        }
    }
    if regular_pairs.len() < 3 {
        return None;
    }

    let keys: Vec<String> = regular_pairs
        .iter()
        .map(|(p, _)| strip_quotes(p).to_owned())
        .collect();
    let value_type = infer_value_type(&keys);
    let dg_name = resolve_dg_name(dg_name, &format!("{subject_var}_map"));
    let indent = command_indent(source, &cmd, line_index).to_owned();

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
            &subject_var,
            &regular_pairs[0].1,
            default_body.as_deref(),
            &indent,
        )
    } else {
        switch_mapping_extraction(
            &regular_pairs,
            &keys,
            &subject_var,
            &dg_name,
            default_body.as_deref(),
            &indent,
        )?
    };

    Some(build_result(
        source,
        &cmd,
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
    subject_var: &str,
    dg_name: &str,
    default_body: Option<&str>,
    indent: &str,
) -> Option<(DataGroupDefinition, String)> {
    let (target_var, use_return, value_entries) = extract_set_or_return_values(regular_pairs)?;
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
        let default_val = extract_single_value(default, use_return, &target_var)?;
        if use_return {
            format!(
                "if {{ [class match ${subject_var} equals {dg_name}] }} {{\n{indent}    return [class lookup ${subject_var} {dg_name}]\n{indent}}} else {{\n{indent}    return {default_val}\n{indent}}}"
            )
        } else {
            format!(
                "if {{ [class match ${subject_var} equals {dg_name}] }} {{\n{indent}    set {target_var} [class lookup ${subject_var} {dg_name}]\n{indent}}} else {{\n{indent}    set {target_var} {default_val}\n{indent}}}"
            )
        }
    } else if use_return {
        format!("return [class lookup ${subject_var} {dg_name}]")
    } else {
        format!("set {target_var} [class lookup ${subject_var} {dg_name}]")
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
) -> Option<Refactoring> {
    extract_to_datagroup_from_if(source, cursor, dg_name, registry, line_index)
        .or_else(|| extract_to_datagroup_from_switch(source, cursor, dg_name, registry, line_index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg() -> CommandRegistry {
        let mut r = CommandRegistry::build_default();
        r.load_dialect(tcl_dialect::DialectSet::IRULES);
        r
    }

    fn if_dg(source: &str, name: &str) -> Option<Refactoring> {
        let r = reg();
        let li = LineIndex::new(source);
        extract_to_datagroup_from_if(source, 0, name, &r, &li)
    }

    fn switch_dg(source: &str, name: &str) -> Option<Refactoring> {
        let r = reg();
        let li = LineIndex::new(source);
        extract_to_datagroup_from_switch(source, 0, name, &r, &li)
    }

    fn dg(r: &Refactoring) -> &DataGroupDefinition {
        r.data_group.as_ref().expect("data group")
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
        assert!(extract_to_datagroup(source, 0, "uri_pool", &r, &li).is_some());
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
        let res = extract_to_datagroup(source, cursor, "", &r, &li).expect("nested result");
        let g = dg(&res);
        assert_eq!(g.value_type, "string");
        assert_eq!(g.records.len(), 3);
    }
}
