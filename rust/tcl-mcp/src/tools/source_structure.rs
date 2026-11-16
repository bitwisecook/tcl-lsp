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

//! The tools that report a source's structure: its graphs, diagram, symbols, dialect and event order.

use super::{DEFAULT_DIALECT, analyse, arg_str, registry, resolve_dialect};
use serde_json::{Value, json};
use std::collections::HashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::{LineIndex, Utf16Col};
use tcl_registry::CommandRegistry;
use tcl_registry::events::EventRegistry;

/// Serialise a control-flow document symbol tree (nested-range shape).
pub(super) fn doc_symbol_to_json(sym: &tcl_lsp_core::document_symbols::DocumentSymbol) -> Value {
    let range = |r: &tcl_lsp_core::document_symbols::LineRange| {
        json!({
            "start": { "line": r.start_line, "character": r.start_character },
            "end": { "line": r.end_line, "character": r.end_character },
        })
    };
    let mut node = json!({
        "name": sym.name,
        "kind": sym.kind.as_str(),
        "range": range(&sym.range),
        "selection_range": range(&sym.selection_range),
        "children": sym.children.iter().map(doc_symbol_to_json).collect::<Vec<_>>(),
    });
    if let Some(detail) = &sym.detail {
        node.as_object_mut()
            .expect("json object")
            .insert("detail".to_owned(), json!(detail));
    }
    node
}

/// The independently selected source event cards in this current analysis.
fn event_source_cards(
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<tcl_lsp_core::document_symbols::DocumentSymbol> {
    tcl_lsp_core::document_symbols::document_symbols_from_analysis(source, analysis)
        .into_iter()
        .filter(|card| card.kind == tcl_lsp_core::document_symbols::SymbolKind::Event)
        .collect()
}

/// Order supported authored event units after actual source-role selection.
/// The catalogue supplies presentation ordering, never runtime event authority.
pub(super) fn event_order_list(source: &str, analysis: &AnalysisResult) -> Vec<Value> {
    let events = EventRegistry::build();
    let mut seen = HashSet::new();
    let cards = event_source_cards(source, analysis);
    let Some(declarations) = tcl_lsp_core::vendor_declaration::declarations(source, analysis)
    else {
        return Vec::new();
    };
    let index = LineIndex::new(source);
    let names = declarations
        .into_iter()
        .filter(|row| {
            row.symbol_metadata()
                .is_some_and(|metadata| metadata.kind == tcl_registry::DefinedSymbolKind::Event)
        })
        .filter(|row| {
            cards.iter().any(|card| {
                index.offset_at_utf16(
                    card.selection_range.start_line,
                    Utf16Col::new(card.selection_range.start_character),
                    source,
                ) == row.span().start()
            })
        })
        .filter_map(|row| row.input().literal_units(row.purpose()))
        .filter(|units| units.is_ascii())
        .filter_map(|units| std::str::from_utf8(units).ok())
        .map(str::to_ascii_uppercase)
        .filter(|event| seen.insert(event.clone()))
        .collect::<Vec<_>>();
    events
        .order_events(&names)
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let multiplicity = events.event_multiplicity(&name);
            json!({"index":i+1,"name":name,"multiplicity":multiplicity})
        })
        .collect()
}

/// Readable selected source event headers; unavailable units remain source cards.
pub(super) fn detect_events(source: &str, analysis: &AnalysisResult) -> Vec<Value> {
    let mut seen = HashSet::new();
    event_source_cards(source, analysis)
        .into_iter()
        .filter(|card| seen.insert(card.name.clone()))
        .map(|card| json!({"name":card.name,"line":card.selection_range.start_line}))
        .collect()
}

// ── Individual tool handlers ──────────────────────────────────────────

type SourceGraphBuilder = fn(
    &str,
    &CommandRegistry,
    Option<&tcl_compiler::analyser::ResolvedAnalysisInput>,
) -> Result<Value, tcl_lsp_core::graphs::GraphSourceInputDecline>;

fn supplied_source_graph(args: &Value, build: SourceGraphBuilder) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let Some(input) = analysis.resolved_input.as_ref() else {
        return json!({ "error": "The graph's retained source input is unavailable" });
    };
    let context = input.context_registry();
    build(source, context.commands(), Some(input))
        .unwrap_or_else(|reason| json!({ "error": reason.to_string() }))
}

pub(super) fn call_graph(args: &Value) -> Value {
    supplied_source_graph(args, tcl_lsp_core::graphs::call_graph_with_source_input)
}

pub(super) fn symbol_graph(args: &Value) -> Value {
    supplied_source_graph(args, tcl_lsp_core::graphs::symbol_graph_with_source_input)
}

pub(super) fn dataflow_graph(args: &Value) -> Value {
    supplied_source_graph(args, tcl_lsp_core::graphs::dataflow_graph_with_source_input)
}

pub(super) fn def_use_chains(args: &Value) -> Value {
    let mut graph =
        supplied_source_graph(args, tcl_lsp_core::graphs::def_use_graph_with_source_input);
    // Optional variable filter.
    let variable = arg_str(args, "variable");
    if !variable.is_empty()
        && let Some(functions) = graph.get_mut("functions").and_then(Value::as_array_mut)
    {
        for func in functions {
            if let Some(nodes) = func.get_mut("nodes").and_then(Value::as_array_mut) {
                nodes.retain(|n| n.get("name").and_then(Value::as_str) == Some(variable));
            }
            if let Some(edges) = func.get_mut("edges").and_then(Value::as_array_mut) {
                edges.retain(|e| {
                    e.get("fromName").and_then(Value::as_str) == Some(variable)
                        || e.get("toName").and_then(Value::as_str) == Some(variable)
                });
            }
        }
    }
    graph
}

pub(super) fn memory_aliases(args: &Value) -> Value {
    supplied_source_graph(
        args,
        tcl_lsp_core::graphs::memory_alias_graph_with_source_input,
    )
}

pub(super) fn diagram(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    tcl_diagram::diagram_data_for_dialect(
        source,
        &registry(&dialect),
        tcl_lsp_core::profile_for_dialect(&dialect),
    )
}

pub(super) fn detect_dialect(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let filename = args.get("filename").and_then(Value::as_str);
    json!(tcl_registry::detect_dialect(
        source,
        filename,
        DEFAULT_DIALECT
    ))
}

pub(super) fn event_order(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let ordered = event_order_list(source, &analysis);
    json!({ "events": ordered, "total": ordered.len() })
}

pub(super) fn symbols(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysis = analyse(source, &dialect);
    let syms: Vec<Value> =
        tcl_lsp_core::document_symbols::document_symbols_from_analysis(source, &analysis)
            .iter()
            .map(doc_symbol_to_json)
            .collect();
    json!({ "symbols": syms })
}
