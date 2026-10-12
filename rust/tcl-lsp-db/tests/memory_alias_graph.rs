// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Public graph projections from the document database's actual source input.

use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_db::{AnalyserConfig, SourceFile, TclDatabase, file_analysis_incremental};

fn document_analysis(source: &str, dialect: &str) -> Arc<AnalysisResult> {
    let db = TclDatabase::default();
    let config = AnalyserConfig::new(
        &db,
        Vec::new(),
        tcl_compiler::analyser::NonAsciiMode::Default,
        Vec::new(),
        None,
        None,
        0,
        Vec::new(),
        Vec::new(),
    );
    let file = SourceFile::new(&db, source.to_owned(), dialect.to_owned(), None);
    file_analysis_incremental(&db, file, config)
}

#[test]
fn actual_document_memory_graph_keeps_conditional_aliases_and_opaque_residuals() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // MAY exposure and unresolved effects are source facts; no Native frame or link is proved.
    for (source, has_global_pair) in [
        ("proc aliaser {} {global ::n::shared; set shared 1}", true),
        (
            "proc global args {return}; proc aliaser {} {global ::n::shared; set shared 1}",
            false,
        ),
        ("proc aliaser {name} {global $name; set local 1}", false),
    ] {
        let analysis = document_analysis(source, "tcl");
        let input = analysis
            .resolved_input
            .as_ref()
            .expect("actual document input");
        let context = input.context_registry();
        let graph = tcl_lsp_core::graphs::memory_alias_graph_with_source_input(
            source,
            context.commands(),
            Some(input),
        )
        .expect("matching actual source input");
        let function = graph["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["name"] == "::aliaser")
            .expect("original authored procedure remains a graph function");
        let aliases = function["alias_sets"].as_array().unwrap();
        let global_pair = aliases.iter().any(|alias| {
            alias["reason"] == "global-cell"
                && alias["names"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|name| name == "::n::shared")
                && alias["names"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|name| name == "shared")
        });
        assert_eq!(global_pair, has_global_pair, "{source}: {graph}");
        assert_eq!(
            function["has_wildcard_aliasing"].as_bool(),
            Some(true),
            "{source}"
        );
    }
}

#[test]
fn actual_document_memory_graph_refuses_missing_and_foreign_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let source = "proc aliaser {} {global ::shared; set shared 1}";
    let logical = document_analysis(source, "tcl");
    let native = document_analysis(source, "tcl9.1");
    let input = logical.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    assert!(matches!(
        tcl_lsp_core::graphs::memory_alias_graph_with_source_input(
            source,
            context.commands(),
            None
        ),
        Err(tcl_lsp_core::graphs::GraphSourceInputDecline::MissingSourceInput)
    ));
    assert!(matches!(
        tcl_lsp_core::graphs::memory_alias_graph_with_source_input(
            source,
            context.commands(),
            native.resolved_input.as_ref(),
        ),
        Err(tcl_lsp_core::graphs::GraphSourceInputDecline::ForeignCommandStore)
    ));
    // Explicit standalone profile requests remain separate from an absent actual input.
    let standalone =
        tcl_lsp_core::graphs::memory_alias_graph(source, context.commands(), input.unit_profile());
    assert!(
        standalone["functions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|function| function["name"] == "::aliaser")
    );
}

#[test]
fn actual_document_graphs_keep_native_jim_and_hosted_source_cards() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // naming.core.original-declaration-graphs
    // docs/design/analysis/name-resolution-proofs/original-declaration-graphs.md
    // Readonly headers do not establish a compiled function, runtime frame or executed effect.
    let source = "proc graph_subject {first} {global ::n::shared; set shared $first}";
    for dialect in ["tcl8.4", "tcl9.1", "jim", "f5-irules"] {
        let analysis = document_analysis(source, dialect);
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let registry = context.commands();
        let symbols =
            tcl_lsp_core::graphs::symbol_graph_with_source_input(source, registry, Some(input))
                .unwrap();
        assert_eq!(symbols["summary"]["total_procs"], 1, "{dialect}: {symbols}");
        let calls =
            tcl_lsp_core::graphs::call_graph_with_source_input(source, registry, Some(input))
                .unwrap();
        let nodes = calls["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 1, "{dialect}: {calls}");
        assert_eq!(nodes[0]["params"], serde_json::json!(["first"]));
        assert!(nodes[0]["pure"].is_null() && nodes[0]["effects"].is_null());
        let memory = tcl_lsp_core::graphs::memory_alias_graph_with_source_input(
            source,
            registry,
            Some(input),
        )
        .unwrap();
        let card = source_function_card(&memory).unwrap_or_else(|| panic!("{dialect}: {memory}"));
        assert_eq!(card["has_wildcard_aliasing"], true);
        assert!(card["alias_sets"].as_array().unwrap().is_empty());
        let definitions =
            tcl_lsp_core::graphs::def_use_graph_with_source_input(source, registry, Some(input))
                .unwrap();
        let card = source_function_card(&definitions)
            .unwrap_or_else(|| panic!("{dialect}: {definitions}"));
        assert_eq!(card["has_wildcard_aliasing"], true);
        assert!(card["summary"].is_null());
        assert!(card["nodes"].as_array().unwrap().is_empty());
        let effects =
            tcl_lsp_core::graphs::dataflow_graph_with_source_input(source, registry, Some(input))
                .unwrap();
        let card = effects["proc_effects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|card| card["source_declaration"]["params"] == serde_json::json!(["first"]))
            .unwrap_or_else(|| panic!("{dialect}: {effects}"));
        assert!(card["pure"].is_null() && card["reads"].is_null() && card["writes"].is_null());
    }
}

fn source_function_card(graph: &serde_json::Value) -> Option<&serde_json::Value> {
    graph["functions"]
        .as_array()?
        .iter()
        .find(|card| card["source_declaration"]["params"] == serde_json::json!(["first"]))
}

#[test]
fn actual_document_graph_cards_respect_a_known_replaced_definer() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // naming.core.original-declaration-graphs
    // docs/design/analysis/name-resolution-proofs/original-declaration-graphs.md
    let source = "proc proc args {return}; proc graph_subject {first} {return}";
    let analysis = document_analysis(source, "tcl9.1");
    let input = analysis.resolved_input.as_ref().unwrap();
    let context = input.context_registry();
    let calls =
        tcl_lsp_core::graphs::call_graph_with_source_input(source, context.commands(), Some(input))
            .unwrap();
    let nodes = calls["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1, "{calls}");
    assert_eq!(nodes[0]["params"], serde_json::json!(["args"]));
    let memory = tcl_lsp_core::graphs::memory_alias_graph_with_source_input(
        source,
        context.commands(),
        Some(input),
    )
    .unwrap();
    assert!(source_function_card(&memory).is_none(), "{memory}");
    let definitions = tcl_lsp_core::graphs::def_use_graph_with_source_input(
        source,
        context.commands(),
        Some(input),
    )
    .unwrap();
    assert!(
        source_function_card(&definitions).is_none(),
        "{definitions}"
    );
}
