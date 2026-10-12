// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly source declaration graphs. Node IDs distinguish inventory entries;
//! they grant no command publication, body execution or editable name geometry.

use crate::original_declaration::{OriginalDeclarationIdentity, invocation_targets_declaration};
use serde_json::{Value, json};
use tcl_compiler::analyser::{AnalysisResult, ProcDef};
use tcl_compiler::ir::Module;
use tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata;

const GRAPH_URI: &str = "<analysis-graph>";

pub(super) fn procedure_view(
    source: &str,
    analysis: &AnalysisResult,
    compilation: Option<(
        &Module,
        &tcl_compiler::interprocedural::InterproceduralAnalysis,
    )>,
) -> Option<Value> {
    if analysis.allows_lexical_declaration_advice() {
        return None;
    }
    OriginalDeclarationIdentity::for_document(GRAPH_URI, source, analysis)?;
    let declarations = analysis
        .original_procedure_declarations()
        .collect::<Vec<_>>();
    let identities = declarations
        .iter()
        .map(|row| OriginalDeclarationIdentity::for_procedure(GRAPH_URI, source, analysis, row))
        .collect::<Option<Vec<_>>>()?;
    let index = tcl_lexer::LineIndex::new(source);
    let labels = declarations
        .iter()
        .zip(&identities)
        .map(|(row, identity)| {
            row.name()
                .source_spelling()
                .unwrap_or_else(|| identity.label())
        })
        .collect::<Vec<_>>();
    let mut nodes = Vec::new();
    let mut references = Vec::new();
    for (ordinal, (row, identity)) in declarations.iter().zip(&identities).enumerate() {
        let proc = row.metadata();
        let summary = compilation.and_then(|(module, interproc)| {
            compiled_label(source, analysis, module, row)
                .and_then(|label| interproc.procedures.get(label))
        });
        nodes.push(json!({
            "id": format!("declaration-{ordinal}"),
            "name": labels[ordinal],
            "params": proc.params.iter().map(|param| &param.name).collect::<Vec<_>>(),
            "line": super::line0(&index, identity.span().start()),
            "span": {"start": identity.span().start(), "end": identity.span().end()},
            "pure": summary.map(|summary| summary.pure),
            "effects": summary.map(|summary| super::effect_region_str(summary.effect_reads | summary.effect_writes)),
        }));
        let mut sites = analysis
            .command_invocations
            .iter()
            .filter(|invocation| {
                invocation_targets_declaration(source, analysis, invocation, row, true)
            })
            .map(|invocation| invocation.range)
            .collect::<Vec<_>>();
        sites.sort_unstable_by_key(|span| (span.start(), span.end()));
        sites.dedup();
        references.push(json!({ "id": format!("declaration-{ordinal}"), "name": labels[ordinal],
            "sites": sites.iter().map(|span| super::pos_value(&index, source, span.start())).collect::<Vec<_>>() }));
    }
    let mut edges = Vec::<Value>::new();
    let mut incoming = vec![false; declarations.len()];
    let mut outgoing = vec![false; declarations.len()];
    for (callee, declaration) in declarations.iter().enumerate() {
        for invocation in &analysis.command_invocations {
            if !invocation.lookup.is_execution_site()
                || !invocation_targets_declaration(source, analysis, invocation, declaration, true)
            {
                continue;
            }
            let Some(caller) = caller(source, analysis, &declarations, invocation.range) else {
                continue;
            };
            let caller_id = caller.map_or_else(
                || super::TOP_LEVEL.to_owned(),
                |ordinal| format!("declaration-{ordinal}"),
            );
            let callee_id = format!("declaration-{callee}");
            let position = super::pos_value(&index, source, invocation.range.start());
            if let Some(edge) = edges
                .iter_mut()
                .find(|edge| edge["caller_id"] == caller_id && edge["callee_id"] == callee_id)
            {
                let sites = edge["call_sites"].as_array_mut()?;
                if !sites.contains(&position) {
                    sites.push(position);
                }
            } else {
                edges.push(json!({"caller_id": caller_id, "callee_id": callee_id,
                    "caller": caller.map_or(super::TOP_LEVEL, |ordinal| labels[ordinal].as_str()),
                    "callee": labels[callee], "call_sites": [position]}));
            }
            incoming[callee] = true;
            if let Some(caller) = caller {
                outgoing[caller] = true;
            }
        }
    }
    let root_ids = incoming
        .iter()
        .enumerate()
        .filter(|(_, called)| !**called)
        .map(|(ordinal, _)| format!("declaration-{ordinal}"))
        .collect::<Vec<_>>();
    let leaf_ids = outgoing
        .iter()
        .enumerate()
        .filter(|(_, calls)| !**calls)
        .map(|(ordinal, _)| format!("declaration-{ordinal}"))
        .collect::<Vec<_>>();
    let roots = labels
        .iter()
        .enumerate()
        .filter(|(ordinal, _)| !incoming[*ordinal])
        .map(|(_, label)| label.clone())
        .collect::<Vec<_>>();
    let leaves = labels
        .iter()
        .enumerate()
        .filter(|(ordinal, _)| !outgoing[*ordinal])
        .map(|(_, label)| label.clone())
        .collect::<Vec<_>>();
    let mut graph = json!({"projection": "original-source-declarations", "nodes": nodes, "edges": edges,
        "references": references, "roots": roots, "leaf_procs": leaves,
        "root_ids": root_ids, "leaf_ids": leaf_ids});
    append_hosted_procedures(source, analysis, &mut graph)?;
    Some(graph)
}

/// Hosted headers remain source cards without borrowing C/Jim publication or
/// compiled body identity. Unsupported names retain their original span and ID.
fn append_hosted_procedures(
    source: &str,
    analysis: &AnalysisResult,
    graph: &mut Value,
) -> Option<()> {
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    let index = tcl_lexer::LineIndex::new(source);
    for (ordinal, row) in analysis
        .original_vendor_procedure_declarations()
        .enumerate()
    {
        let input = row.name_input();
        if !input.matches_source(&image, config) {
            return None;
        }
        let id = format!("hosted-declaration-{ordinal}");
        let name = input
            .literal_units(row.purpose())
            .and_then(|units| std::str::from_utf8(units).ok())
            .map_or_else(|| id.clone(), str::to_owned);
        let span = input.span();
        graph["nodes"].as_array_mut()?.push(json!({
            "id": id, "name": name,
            "projection": "hosted-source-declaration",
            "params": row.metadata().params.iter().map(|param| &param.name).collect::<Vec<_>>(),
            "line": super::line0(&index, span.start()),
            "span": {"start": span.start(), "end": span.end()},
            "pure": null, "effects": null,
        }));
        graph["references"]
            .as_array_mut()?
            .push(json!({"id": id, "name": name, "sites": []}));
        for key in ["roots", "leaf_procs"] {
            graph[key].as_array_mut()?.push(json!(name));
        }
        for key in ["root_ids", "leaf_ids"] {
            graph[key].as_array_mut()?.push(json!(id));
        }
    }
    Some(())
}

/// Original declarations with no independently matched compiled body. These
/// cards preserve authored functions while withholding every execution summary.
pub(super) fn uncompiled_procedure_nodes(
    source: &str,
    analysis: &AnalysisResult,
    module: &Module,
) -> Vec<Value> {
    let Some(graph) = procedure_view(source, analysis, None) else {
        return Vec::new();
    };
    let represented = analysis
        .original_procedure_declarations()
        .enumerate()
        .filter(|(_, row)| compiled_label(source, analysis, module, row).is_some())
        .map(|(ordinal, _)| format!("declaration-{ordinal}"))
        .collect::<std::collections::HashSet<_>>();
    graph["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|node| {
            node["id"]
                .as_str()
                .is_some_and(|id| !represented.contains(id))
        })
        .cloned()
        .collect()
}

/// A compiled unit must independently retain the same original implementation
/// body, namespace and allocation. Labels only index those authenticated units.
pub(super) fn compiled_label<'a>(
    source: &str,
    analysis: &AnalysisResult,
    module: &'a Module,
    declaration: &SourceDeclarationMetadata<ProcDef>,
) -> Option<&'a str> {
    tcl_compiler::source_graph::procedure_body_for_declaration(
        source,
        analysis,
        module,
        declaration,
    )
    .map(|(label, _)| label)
}

/// Source containment groups calls under a genuine braced original body word.
/// This grouping supplies no procedure activation or runtime caller frame.
fn caller(
    source: &str,
    analysis: &AnalysisResult,
    declarations: &[&SourceDeclarationMetadata<ProcDef>],
    range: tcl_lexer::Span,
) -> Option<Option<usize>> {
    let image = tcl_lexer::SourceImage::document(source);
    let config = analysis.body_lexer_config?;
    let realm = analysis.retained_command_realm()?;
    let mut owners = Vec::new();
    for (ordinal, declaration) in declarations.iter().enumerate() {
        let proc = declaration.metadata();
        if !(proc.body_span.start() <= range.start() && range.end() <= proc.body_span.end()) {
            continue;
        }
        let input =
            realm.original_written_name_input_at_span_in_source(&image, proc.body_span, config)?;
        let word = input.original_word_key()?.original_word();
        if word.group().kind != tcl_lexer::WordKind::Braced {
            return None;
        }
        let content = word.content_span().ok()?;
        if content.start() <= range.start() && range.end() <= content.end() {
            owners.push((ordinal, content));
        }
    }
    owners.sort_by_key(|(_, span)| span.len());
    match owners.as_slice() {
        [] => Some(None),
        [(ordinal, _)] => Some(Some(*ordinal)),
        [(ordinal, first), (_, second), ..] if first.len() < second.len() => Some(Some(*ordinal)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    // Implementation contract: naming.core.original-declaration-graphs
    // docs/design/analysis/name-resolution-proofs/original-declaration-graphs.md
    fn original_graph_nodes_preserve_opaque_redefinitions_and_canonical_edges_without_ui_maps() {
        let source = "proc p\\uD800 {} {return FIRST}; p\\uD800; proc p\\uD801 {} {return OTHER}; p\\uD801; proc p\\uD800 {} {return LAST}; p\\uD800";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        let graph = procedure_view(source, &analysis, None).unwrap();
        let nodes = graph["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 3, "{graph}");
        assert_eq!(
            nodes
                .iter()
                .map(|node| node["id"].as_str().unwrap())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            3
        );
        assert_eq!(nodes[0]["name"], nodes[2]["name"]);
        assert_ne!(nodes[0]["name"], nodes[1]["name"]);
        assert!(
            nodes
                .iter()
                .all(|node| node["pure"].is_null() && node["effects"].is_null())
        );
        let edges = graph["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 3, "{graph}");
        for (ordinal, edge) in edges.iter().enumerate() {
            assert_eq!(edge["callee_id"], format!("declaration-{ordinal}"));
            assert_eq!(edge["caller_id"], super::super::TOP_LEVEL);
        }
        assert!(procedure_view(&format!("#{source}"), &analysis, None).is_none());
        let mut absent = analysis.clone();
        for invocation in &mut absent.command_invocations {
            invocation.original_lookup = None;
        }
        assert!(
            procedure_view(source, &absent, None).unwrap()["edges"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn original_graph_alias_edges_select_terminal_allocation_without_reporting_names() {
        // Implementation contract: naming.core.original-declaration-graphs
        // docs/design/analysis/name-resolution-proofs/original-declaration-graphs.md
        let source = "proc target {} {return ok}; proc alias {} {return displaced}; interp alias {} alias {} target; alias";
        let mut analysis = Analyser::new().analyse(source, "tcl9.0").clone();
        analysis.all_procs.clear();
        for invocation in &mut analysis.command_invocations {
            invocation.name = "counterfactual".to_owned();
        }
        let graph = procedure_view(source, &analysis, None).unwrap();
        let edges = graph["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1, "{graph}");
        assert_eq!(edges[0]["callee_id"], "declaration-0");
    }
    #[test]
    fn original_graph_summary_requires_body_allocation_even_when_ir_labels_agree() {
        // Implementation contract: naming.core.original-declaration-graphs
        // docs/design/analysis/name-resolution-proofs/original-declaration-graphs.md
        let source = "proc p {} {return SAFE}; p";
        let profile = crate::profile_for_dialect("tcl8.6");
        let registry = crate::registry_for_dialect("tcl8.6");
        let input = super::super::standalone_graph_input(registry, profile);
        let unit = super::super::document_unit(source, registry, &input)
            .with_interprocedural(registry, Some(profile));
        let analysis = Analyser::new().analyse(source, "tcl8.6").clone();
        let mut module = unit.ir_module.clone();
        module.procedure_implementation_bodies = std::sync::Arc::from([]);
        module.original_declaration_body_units.clear();
        module.installed_procedure_body_units.clear();
        let graph = procedure_view(
            source,
            &analysis,
            Some((&module, unit.interproc.as_ref().unwrap())),
        )
        .unwrap();
        assert!(
            graph["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|node| node["pure"].is_null() && node["effects"].is_null()),
            "{graph}"
        );
    }
}
