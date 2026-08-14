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

//! Analysis/graph verbs: `symbols`, `callgraph`, `symbolgraph`, `dataflow`,
//! and `diagram`.
//!
//! These verbs combine every resolved input into one source
//! (`combine_sources`), analyse it, and
//! emit a JSON-serialisable graph/symbol shape (or a plain-text summary).
//!
//! Ordering note: symbols are produced by iterating `Scope.procs` then
//! `Scope.variables` in insertion order, so they come out in
//! source-definition order.
//! The Rust analyser stores them in `HashMap`s, so we sort by the defining
//! token's source offset to recover that deterministic ordering.

use serde::Serialize;
use serde_json::{Value, json};
use tcl_cli_support::{
    OutputTarget, combine_sources, combined_effective_dialect, ensure_ascii, read_input_documents,
    registry_for_dialect, write_text_output,
};
use tcl_compiler::analyser::{Analyser, ProcDef, Scope, ScopeKind, VarDef};
use tcl_lexer::LineIndex;
use tcl_lsp_core::graphs;

use crate::cli::InputArgs;

/// One symbol entry in the `symbols` payload.
///
/// Fields are emitted in order (`kind`, `name`, `line`, `depth`, then the
/// function-only `params`). `params` is emitted only for functions (the key
/// is omitted entirely for other kinds), and `line` may be `null` for a proc
/// with no name token.
#[derive(Serialize)]
struct SymbolEntry {
    kind: &'static str,
    name: String,
    line: Option<u32>,
    depth: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Vec<String>>,
}

/// `symbols --json` payload.
#[derive(Serialize)]
struct SymbolsPayload {
    count: usize,
    dialect: String,
    inputs: Vec<String>,
    symbols: Vec<SymbolEntry>,
}

/// 1-based source line of a span's start offset.
fn line_of(line_index: &LineIndex, offset: u32) -> u32 {
    line_index.position_at(offset).line + 1
}

/// Detect `when EVENT` iRules entries via the regex
/// `\bwhen\s+([A-Z_][A-Z0-9_]*)`, deduplicated, in first-seen order.
///
/// Runs unconditionally (every dialect),
/// each entry at depth 0 with its 1-based line.
fn detect_event_entries(source: &str, line_index: &LineIndex) -> Vec<SymbolEntry> {
    fn is_word(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }
    fn is_upper_start(b: u8) -> bool {
        b.is_ascii_uppercase() || b == b'_'
    }
    fn is_upper_rest(b: u8) -> bool {
        b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'
    }

    let bytes = source.as_bytes();
    let mut entries = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut i = 0usize;
    while i + 4 <= bytes.len() {
        // `\bwhen`: the four chars "when" with a word boundary before.
        if &bytes[i..i + 4] == b"when" && (i == 0 || !is_word(bytes[i - 1])) {
            let mut j = i + 4;
            // `\s+`
            let ws_start = j;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j > ws_start && j < bytes.len() && is_upper_start(bytes[j]) {
                let name_start = j;
                j += 1;
                while j < bytes.len() && is_upper_rest(bytes[j]) {
                    j += 1;
                }
                let name = &source[name_start..j];
                if seen.insert(name.to_string()) {
                    entries.push(SymbolEntry {
                        kind: "event",
                        name: name.to_string(),
                        line: Some(line_of(line_index, u32::try_from(i).unwrap_or(u32::MAX))),
                        depth: 0,
                        params: None,
                    });
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    entries
}

/// Recursively collect proc/variable/namespace symbol entries from a scope.
///
/// Procs first (source order), then
/// global/namespace-scope variables, then children — namespace children get
/// an entry and recurse, proc children only recurse (to surface nested procs).
fn collect_scope_entries(
    scope: &Scope,
    depth: usize,
    line_index: &LineIndex,
    out: &mut Vec<SymbolEntry>,
) {
    let mut procs: Vec<&ProcDef> = scope.procs.values().collect();
    procs.sort_by_key(|p| p.name_span.start());
    for proc in procs {
        let params = proc.params.iter().map(|p| p.name.clone()).collect();
        out.push(SymbolEntry {
            kind: "function",
            name: proc.name.clone(),
            line: Some(line_of(line_index, proc.name_span.start())),
            depth,
            params: Some(params),
        });
    }

    if matches!(scope.kind, ScopeKind::Global | ScopeKind::Namespace) {
        let mut vars: Vec<&VarDef> = scope.variables.values().collect();
        vars.sort_by_key(|v| v.definition_span.start());
        for var in vars {
            out.push(SymbolEntry {
                kind: "variable",
                name: var.name.clone(),
                line: Some(line_of(line_index, var.definition_span.start())),
                depth,
                params: None,
            });
        }
    }

    for child in &scope.children {
        match child.kind {
            ScopeKind::Namespace => {
                if let Some(body) = child.body_span {
                    out.push(SymbolEntry {
                        kind: "namespace",
                        name: child.name.clone(),
                        line: Some(line_of(line_index, body.start())),
                        depth,
                        params: None,
                    });
                    collect_scope_entries(child, depth + 1, line_index, out);
                }
            }
            ScopeKind::Proc => {
                collect_scope_entries(child, depth + 1, line_index, out);
            }
            _ => {}
        }
    }
}

/// `tcl symbols` — list every declared symbol (procs, namespaces, variables,
/// iRules `when` events) across the combined input.
pub fn run_symbols(input: &InputArgs, json: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let source = combine_sources(&documents);
    let result = Analyser::new()
        .with_pack_overlay(tcl_cli_support::spec_pack_key(&dialect))
        .analyse(&source, &dialect);
    let line_index = LineIndex::new(&source);

    let mut entries = detect_event_entries(&source, &line_index);
    collect_scope_entries(&result.global_scope, 0, &line_index, &mut entries);

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json {
        let payload = SymbolsPayload {
            count: entries.len(),
            dialect: dialect.clone(),
            inputs: documents.iter().map(|d| d.label.clone()).collect(),
            symbols: entries,
        };
        let text = ensure_ascii(&serde_json::to_string_pretty(&payload)?);
        write_text_output(&target, &text)?;
        return Ok(0);
    }

    if entries.is_empty() {
        write_text_output(&target, "no symbols")?;
        return Ok(0);
    }

    let mut lines = vec![format!("symbols: {}", entries.len())];
    for entry in &entries {
        let indent = "  ".repeat(entry.depth);
        let line_suffix = entry
            .line
            .map_or_else(String::new, |l| format!(" (line {l})"));
        if entry.kind == "function" {
            let params = entry
                .params
                .as_ref()
                .map(|p| p.join(", "))
                .unwrap_or_default();
            lines.push(format!(
                "{indent}function {}({params}){line_suffix}",
                entry.name
            ));
        } else {
            lines.push(format!(
                "{indent}{} {}{line_suffix}",
                entry.kind, entry.name
            ));
        }
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}

// symbolgraph

/// Render the text form of a serialised scope.
fn append_symbolgraph_scope(lines: &mut Vec<String>, scope: &Value, depth: usize) {
    let indent = "  ".repeat(depth);
    let kind = scope.get("kind").and_then(Value::as_str).unwrap_or("?");
    let name = scope.get("name").and_then(Value::as_str).unwrap_or("?");
    lines.push(format!("{indent}{kind} {name}"));

    if let Some(procs) = scope.get("procs").and_then(Value::as_array) {
        for proc in procs {
            let params = proc
                .get("params")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let line_suffix = proc
                .get("line")
                .and_then(Value::as_i64)
                .map_or_else(String::new, |l| format!(" (line {})", l + 1));
            let refs = proc.get("ref_count").and_then(Value::as_i64).unwrap_or(0);
            let pname = proc.get("name").and_then(Value::as_str).unwrap_or("?");
            lines.push(format!(
                "{indent}  proc {pname}({params}){line_suffix} [{refs} refs]"
            ));
        }
    }

    if let Some(variables) = scope.get("variables").and_then(Value::as_array) {
        for var in variables {
            let line_suffix = var
                .get("line")
                .and_then(Value::as_i64)
                .map_or_else(String::new, |l| format!(" (line {})", l + 1));
            let refs = var
                .get("references")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            let vname = var.get("name").and_then(Value::as_str).unwrap_or("?");
            lines.push(format!("{indent}  var {vname}{line_suffix} [{refs} refs]"));
        }
    }

    if let Some(children) = scope.get("children").and_then(Value::as_array) {
        for child in children {
            append_symbolgraph_scope(lines, child, depth + 1);
        }
    }
}

/// `tcl symbolgraph` — scope hierarchy with proc/variable references.
pub fn run_symbolgraph(input: &InputArgs, json_out: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let source = combine_sources(&documents);
    let data = graphs::symbol_graph(&source, &dialect);

    let summary = data.get("summary").cloned().unwrap_or_else(|| json!({}));
    let count = |key: &str| summary.get(key).and_then(Value::as_i64).unwrap_or(0);
    let total_procs = count("total_procs");
    let total_variables = count("total_variables");
    let total_namespaces = count("total_namespaces");

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json_out {
        let text = ensure_ascii(&serde_json::to_string_pretty(&data)?);
        write_text_output(&target, &text)?;
        return Ok(0);
    }

    let mut lines = vec![format!(
        "symbol graph: procs={total_procs} variables={total_variables} namespaces={total_namespaces}"
    )];
    if let Some(scope_list) = data.get("scopes").and_then(Value::as_array)
        && !scope_list.is_empty()
    {
        lines.push("scopes:".to_string());
        for scope in scope_list {
            append_symbolgraph_scope(&mut lines, scope, 1);
        }
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}

/// `tcl callgraph` — caller→callee graph across every proc in the input.
#[allow(clippy::similar_names)] // caller / callee mirror the domain
pub fn run_callgraph(input: &InputArgs, json_out: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let source = combine_sources(&documents);
    let registry = registry_for_dialect(&dialect);
    let data = graphs::call_graph(&source, &registry, &dialect);

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json_out {
        let text = ensure_ascii(&serde_json::to_string_pretty(&data)?);
        write_text_output(&target, &text)?;
        return Ok(0);
    }

    let empty: Vec<Value> = Vec::new();
    let nodes = data
        .get("nodes")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let edges = data
        .get("edges")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let roots = data
        .get("roots")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let leaves = data
        .get("leaf_procs")
        .and_then(Value::as_array)
        .unwrap_or(&empty);

    let mut lines = vec![format!(
        "call graph: procs={} edges={}",
        nodes.len(),
        edges.len()
    )];
    if !nodes.is_empty() {
        lines.push("procs:".to_owned());
        for node in nodes {
            let name = node.get("name").and_then(Value::as_str).unwrap_or("?");
            let params = node
                .get("params")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_owned))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let line_suffix = node
                .get("line")
                .and_then(Value::as_i64)
                .map_or_else(String::new, |l| format!(" (line {})", l + 1));
            let pure_suffix = if node.get("pure").and_then(Value::as_bool) == Some(true) {
                " [pure]"
            } else {
                ""
            };
            lines.push(format!("  {name}({params}){line_suffix}{pure_suffix}"));
        }
    }
    if !edges.is_empty() {
        lines.push("edges:".to_owned());
        for edge in edges {
            let caller = edge.get("caller").and_then(Value::as_str).unwrap_or("?");
            let callee = edge.get("callee").and_then(Value::as_str).unwrap_or("?");
            lines.push(format!("  {caller} -> {callee}"));
        }
    }
    if !roots.is_empty() {
        let names: Vec<&str> = roots.iter().filter_map(Value::as_str).collect();
        lines.push(format!("roots: {}", names.join(", ")));
    }
    if !leaves.is_empty() {
        let names: Vec<&str> = leaves.iter().filter_map(Value::as_str).collect();
        lines.push(format!("leaves: {}", names.join(", ")));
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}

// dataflow

/// `tcl dataflow` — taint warnings, tainted variables, and per-proc
/// side-effect classification.
pub fn run_dataflow(input: &InputArgs, json_out: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect.as_deref());
    let source = combine_sources(&documents);
    let registry = registry_for_dialect(&dialect);
    let data = graphs::dataflow_graph(&source, &registry, &dialect);

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json_out {
        let text = ensure_ascii(&serde_json::to_string_pretty(&data)?);
        write_text_output(&target, &text)?;
        return Ok(0);
    }

    let summary = data.get("summary").cloned().unwrap_or_else(|| json!({}));
    let get = |key: &str| summary.get(key).and_then(Value::as_i64).unwrap_or(0);
    let mut lines = vec![format!(
        "dataflow: taintWarnings={} taintedVars={} pure={} impure={}",
        get("total_taint_warnings"),
        get("tainted_variable_count"),
        get("pure_proc_count"),
        get("impure_proc_count"),
    )];

    let empty: Vec<Value> = Vec::new();
    let warnings = data
        .get("taint_warnings")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    if !warnings.is_empty() {
        lines.push("taint warnings:".to_owned());
        for warning in warnings {
            let line_no = warning
                .get("line")
                .and_then(Value::as_i64)
                .map_or_else(|| "?".to_owned(), |l| (l + 1).to_string());
            let code = warning.get("code").and_then(Value::as_str).unwrap_or("");
            let message = warning.get("message").and_then(Value::as_str).unwrap_or("");
            lines.push(format!("  {code} line {line_no}: {message}"));
        }
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}
