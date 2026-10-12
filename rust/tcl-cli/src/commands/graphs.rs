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

//! Analysis/graph verbs: `symbols`, `callgraph`, `symbolgraph`, and
//! `dataflow`.
//!
//! These verbs combine every resolved input into one source
//! (`combine_sources`), analyse it, and
//! emit a JSON-serialisable graph/symbol shape (or a plain-text summary).
//!
//! Native and hosted symbols use the shared independently current original
//! source outline. Labels are presentation; opaque and repeated declarations
//! retain separate source rows. Explicit lexical advice keeps its own scope
//! walker with defining-span order.

use serde::Serialize;
use serde_json::{Value, json};
use tcl_cli_support::{
    OutputTarget, combine_sources, combined_effective_dialect, ensure_ascii, read_input_documents,
    registry_for_dialect, write_text_output,
};
use tcl_compiler::analyser::{Analyser, AnalysisResult, ProcDef, Scope, ScopeKind, VarDef};
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

/// Project only current source cards; labels never select a declaration.
fn original_symbol_entries(source: &str, analysis: &AnalysisResult) -> Vec<SymbolEntry> {
    use tcl_lsp_core::document_symbols::{DocumentSymbol, SymbolKind};
    let index = LineIndex::new(source);
    let procedures =
        tcl_lsp_core::procedure_symbol::declarations(source, analysis).unwrap_or_default();
    fn collect(
        source: &str,
        index: &LineIndex,
        procedures: &[&ProcDef],
        cards: &[DocumentSymbol],
        depth: usize,
        in_procedure: bool,
        out: &mut Vec<SymbolEntry>,
    ) {
        for card in cards {
            let kind = match card.kind {
                SymbolKind::Function => Some("function"),
                SymbolKind::Namespace => Some("namespace"),
                SymbolKind::Variable if !in_procedure => Some("variable"),
                SymbolKind::Event => Some("event"),
                _ => None,
            };
            if let Some(kind) = kind {
                let params = (card.kind == SymbolKind::Function).then(|| {
                    let start = index.offset_at_utf16(
                        card.selection_range.start_line,
                        tcl_lexer::Utf16Col::new(card.selection_range.start_character),
                        source,
                    );
                    let end = index.offset_at_utf16(
                        card.selection_range.end_line,
                        tcl_lexer::Utf16Col::new(card.selection_range.end_character),
                        source,
                    );
                    let mut matching = procedures.iter().filter(|proc| {
                        proc.name_span.start() == start && proc.name_span.end() == end
                    });
                    let first = matching.next();
                    if matching.next().is_some() {
                        return Vec::new();
                    }
                    first.map_or_else(Vec::new, |proc| {
                        proc.params.iter().map(|param| param.name.clone()).collect()
                    })
                });
                out.push(SymbolEntry {
                    kind,
                    name: card.name.clone(),
                    line: Some(card.selection_range.start_line + 1),
                    depth,
                    params,
                });
            }
            let local = in_procedure
                || matches!(
                    card.kind,
                    SymbolKind::Function
                        | SymbolKind::Method
                        | SymbolKind::Constructor
                        | SymbolKind::Event
                );
            collect(
                source,
                index,
                procedures,
                &card.children,
                depth + 1,
                local,
                out,
            );
        }
    }
    let cards = tcl_lsp_core::document_symbols::document_symbols_from_analysis(source, analysis);
    let mut entries = Vec::new();
    collect(source, &index, &procedures, &cards, 0, false, &mut entries);
    entries
}

#[cfg(test)]
fn detect_event_entries(
    source: &str,
    _line_index: &LineIndex,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<SymbolEntry> {
    let analysis = Analyser::new().analyse(source, dialect.name);
    original_symbol_entries(source, &analysis)
        .into_iter()
        .filter(|entry| entry.kind == "event")
        .collect()
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
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let source = combine_sources(&documents);
    let result = Analyser::new()
        .with_pack_overlay(tcl_cli_support::spec_pack_key(dialect.name))
        .analyse(&source, dialect.name);
    let line_index = LineIndex::new(&source);

    let entries = if result.allows_lexical_declaration_advice() {
        let mut entries = Vec::new();
        collect_scope_entries(&result.global_scope, 0, &line_index, &mut entries);
        entries
    } else {
        original_symbol_entries(&source, &result)
    };

    let target = OutputTarget::from_arg(input.output.as_deref());

    if json {
        let payload = SymbolsPayload {
            count: entries.len(),
            dialect: dialect.name.to_owned(),
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
fn append_symbolgraph_scope(
    lines: &mut Vec<String>,
    scope: &Value,
    depth: usize,
    show_procedures: bool,
) {
    let indent = "  ".repeat(depth);
    let kind = scope.get("kind").and_then(Value::as_str).unwrap_or("?");
    let name = scope.get("name").and_then(Value::as_str).unwrap_or("?");
    lines.push(format!("{indent}{kind} {name}"));

    if show_procedures && let Some(procs) = scope.get("procs").and_then(Value::as_array) {
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
            append_symbolgraph_scope(lines, child, depth + 1, show_procedures);
        }
    }
}

/// Retain the CLI's selected profile and pack store before graph projection.
fn graph_analysis(source: &str, profile: &'static tcl_dialect::DialectProfile) -> AnalysisResult {
    let registry = registry_for_dialect(profile.name);
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        std::sync::Arc::new(context.with_command_store(registry)),
        tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

/// `tcl symbolgraph` — scope hierarchy with proc/variable references.
pub fn run_symbolgraph(input: &InputArgs, json_out: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let source = combine_sources(&documents);
    let analysis = graph_analysis(&source, dialect);
    let source_input = analysis
        .resolved_input
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("the graph's retained source input is unavailable"))?;
    let context = source_input.context_registry();
    let data =
        graphs::symbol_graph_with_source_input(&source, context.commands(), Some(source_input))?;

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
    let original = data
        .get("declarations")
        .filter(|declarations| declarations.is_object());
    if let Some(nodes) = original.and_then(|declarations| declarations["nodes"].as_array()) {
        lines.push("procedures:".to_owned());
        for node in nodes {
            let name = node["name"].as_str().unwrap_or("?");
            let id = node["id"].as_str().unwrap_or("?");
            let references = original
                .and_then(|declarations| declarations["references"].as_array())
                .and_then(|rows| rows.iter().find(|row| row["id"] == id))
                .and_then(|row| row["sites"].as_array())
                .map_or(0, Vec::len);
            lines.push(format!("  {name} [{id}] [{references} refs]"));
        }
    }
    if let Some(scope_list) = data.get("scopes").and_then(Value::as_array)
        && !scope_list.is_empty()
    {
        lines.push("scopes:".to_string());
        for scope in scope_list {
            append_symbolgraph_scope(&mut lines, scope, 1, original.is_none());
        }
    }
    write_text_output(&target, &lines.join("\n"))?;
    Ok(0)
}

/// `tcl callgraph` — caller→callee graph across every proc in the input.
#[allow(clippy::similar_names)] // caller / callee mirror the domain
pub fn run_callgraph(input: &InputArgs, json_out: bool) -> anyhow::Result<u8> {
    let documents = read_input_documents(&input.inputs, &input.source, !input.no_recursive)?;
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let source = combine_sources(&documents);
    let analysis = graph_analysis(&source, dialect);
    let source_input = analysis
        .resolved_input
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("the graph's retained source input is unavailable"))?;
    let context = source_input.context_registry();
    let data =
        graphs::call_graph_with_source_input(&source, context.commands(), Some(source_input))?;

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
            let identity_suffix = node
                .get("id")
                .and_then(Value::as_str)
                .map_or_else(String::new, |id| format!(" [{id}]"));
            let pure_suffix = if node.get("pure").and_then(Value::as_bool) == Some(true) {
                " [pure]"
            } else {
                ""
            };
            lines.push(format!(
                "  {name}({params}){line_suffix}{identity_suffix}{pure_suffix}"
            ));
        }
    }
    if !edges.is_empty() {
        lines.push("edges:".to_owned());
        for edge in edges {
            let caller = edge.get("caller").and_then(Value::as_str).unwrap_or("?");
            let callee = edge.get("callee").and_then(Value::as_str).unwrap_or("?");
            let caller_id = edge
                .get("caller_id")
                .and_then(Value::as_str)
                .map_or_else(String::new, |id| format!(" [{id}]"));
            let callee_id = edge
                .get("callee_id")
                .and_then(Value::as_str)
                .map_or_else(String::new, |id| format!(" [{id}]"));
            lines.push(format!("  {caller}{caller_id} -> {callee}{callee_id}"));
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
    let dialect = combined_effective_dialect(&documents, input.dialect_profile()?);
    let source = combine_sources(&documents);
    let analysis = graph_analysis(&source, dialect);
    let source_input = analysis
        .resolved_input
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("the graph's retained source input is unavailable"))?;
    let context = source_input.context_registry();
    let data =
        graphs::dataflow_graph_with_source_input(&source, context.commands(), Some(source_input))?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_analysis_retains_selected_profile_grammar_and_source_projection() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let profile = Box::leak(Box::new(tcl_dialect::DialectProfile {
            grammar: tcl_dialect::LexerGrammar {
                escapes: tcl_dialect::EscapeSyntax::Tcl84,
                ..tcl_dialect::DialectProfile::plain_tcl().grammar
            },
            ..tcl_dialect::DialectProfile::plain_tcl().clone()
        }));
        let source = "proc p {} {}";
        let analysis = graph_analysis(source, profile);
        let input = analysis.resolved_input.as_ref().unwrap();
        assert!(std::ptr::eq(input.unit_profile(), profile));
        assert_eq!(
            input.lexer_config().escapes,
            tcl_dialect::EscapeSyntax::Tcl84
        );
        let context = input.context_registry();
        let graph = graphs::symbol_graph_with_source_input(source, context.commands(), Some(input))
            .unwrap();
        assert_eq!(graph["summary"]["total_procs"].as_u64(), Some(1));
    }

    #[test]
    fn event_symbols_use_top_level_rooted_normalised_owner() {
        let source = "::when http_request {\n  if {1} { :::when client_data {} }\n}";
        let entries = detect_event_entries(
            source,
            &LineIndex::new(source),
            tcl_cli_support::environment::profile_for_dialect("f5-irules"),
        );
        assert_eq!(
            entries
                .iter()
                .map(|entry| (entry.name.as_str(), entry.line))
                .collect::<Vec<_>>(),
            [("http_request", Some(1))]
        );
    }

    #[test]
    fn event_symbols_ignore_inert_braced_and_quoted_when_text() {
        let source = "set payload {when CLIENT_DATA {}}\nset q \"when SERVER_DATA {}\"\nwhen HTTP_REQUEST {}";
        let entries = detect_event_entries(
            source,
            &LineIndex::new(source),
            tcl_cli_support::environment::profile_for_dialect("f5-irules"),
        );
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST"]
        );
    }
    #[test]
    fn original_cli_symbols_preserve_repeated_opaque_headers_and_current_source() {
        // Implementation contract: naming.consumer.original-cli-source-symbols
        // docs/design/analysis/name-resolution-proofs/original-cli-source-symbols.md
        let source = r"proc p\uD800 {} {}; proc p\uD801 {} {}; proc p\uD800 {arg} {}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        let rows = original_symbol_entries(source, &analysis);
        assert_eq!(rows.iter().filter(|row| row.kind == "function").count(), 3);
        assert_ne!(rows[0].name, rows[1].name);
        assert_eq!(
            rows[2].params.as_deref(),
            Some(["arg".to_owned()].as_slice())
        );
        assert!(original_symbol_entries(&format!("# changed\n{source}"), &analysis).is_empty());
        let hosted = "proc helper {} {}\nproc p\\uD800 {} {}\nwhen HTTP_REQUEST {}";
        let mut analysis = Analyser::new().analyse(hosted, "f5-irules");
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        analysis.all_defined_symbols.clear();
        analysis.global_scope.defined_symbols.clear();
        let rows = original_symbol_entries(hosted, &analysis);
        assert_eq!(rows.iter().filter(|row| row.kind == "function").count(), 2);
        assert_eq!(rows.iter().filter(|row| row.kind == "event").count(), 1);
    }
}
