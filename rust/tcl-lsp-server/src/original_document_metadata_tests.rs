// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Real document consumers preserve complete source inputs and typed findings.

use super::*;
use tcl_compiler::analyser::ResolvedAnalysisInput;

fn source_analysis(
    source: &str,
    context: Arc<tcl_registry::model::ContextRegistry>,
) -> AnalysisResult {
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let input = ResolvedAnalysisInput::new(
        profile,
        profile,
        context,
        tcl_lexer::LexerConfig {
            braced_var: tcl_dialect::BracedVarStyle::FirstClose,
            ..tcl_lexer::LexerConfig::for_profile(Some(profile))
        },
    );
    Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name)
}

fn is_variable(data: &[u32], source: &str, value: &str) -> bool {
    let offset = u32::try_from(source.find(value).unwrap()).unwrap();
    let mut line = 0;
    let mut column = 0;
    data.chunks_exact(5).any(|token| {
        line += token[0];
        column = if token[0] == 0 {
            column + token[1]
        } else {
            token[1]
        };
        line == 0 && column == offset && token[3] == 2
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unindexed_semantic_tokens_keep_workspace_pack_roles() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Authoring roles only; loading this spec grants no native command entry.
    let backend = crate::tests::test_backend();
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: PathBuf::from("/workspace/.tcl-lsp/unindexed-metadata.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        "speclib unindexed_metadata 1.0 {
command unindexed_metadata::read {
arity 1
arg 0 -role VarRead
}
}
"
        .to_owned(),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &packs);
    *backend.spec_packs.lock().await = PublishedPackSet {
        seq: 1,
        packs: Arc::new(packs),
    };
    backend.sync_db_config().await;
    let uri = Uri::from_str("file:///workspace/unindexed-metadata.tcl").unwrap();
    assert!(!backend.db_files.lock().await.contains_key(&uri));
    let source = "unindexed_metadata::read retained";
    let document = DocumentState::new(source.to_owned(), "tcl".to_owned());
    let tokens = backend
        .semantic_tokens_core_data(&uri, &document)
        .await
        .unwrap();
    assert!(is_variable(&tokens, source, "retained"));
    let analysis = backend
        .fresh_analysis_for(&uri, Arc::clone(&document.text), document.dialect.clone())
        .await;
    assert_eq!(
        analysis
            .resolved_registry()
            .unwrap()
            .snapshot()
            .semantic_key(),
        registry.snapshot().semantic_key()
    );
}

#[test]
fn unindexed_tokens_refuse_withdrawn_source_metadata_and_availability() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let mut registry = baseline
        .commands()
        .project_for_profile(tcl_registry::model::resolve_environment("tcl8.6").unit_profile());
    let mut read = registry.get("set").unwrap().clone();
    read.name = "metadata_read";
    read.surface = registry.get("dict").unwrap().surface;
    registry.insert(read);
    let current = Arc::new(baseline.with_command_store(Arc::new(registry)));
    let source = "metadata_read retained";
    let analysis = source_analysis(source, Arc::clone(&current));
    assert!(is_variable(
        &unindexed_semantic_tokens(source, &analysis).data,
        source,
        "retained"
    ));
    let older = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(Arc::clone(current.commands())),
    );
    let unavailable = source_analysis(source, older);
    assert!(!is_variable(
        &unindexed_semantic_tokens(source, &unavailable).data,
        source,
        "retained"
    ));
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    let mut stale = analysis.clone();
    stale.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    let mut foreign = analysis.clone();
    foreign.resolved_input = source_analysis(
        source,
        tcl_registry::model::resolve_environment("tcl9.1").default_context_registry(),
    )
    .resolved_input;
    for refused in [missing, stale, foreign] {
        assert!(unindexed_semantic_tokens(source, &refused).data.is_empty());
    }
    assert!(
        unindexed_semantic_tokens("metadata_read changed", &analysis)
            .data
            .is_empty()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pull_compiler_and_code_actions_keep_supplied_diagnostic_context() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let backend = crate::tests::test_backend();
    let uri = Uri::from_str("file:///workspace/retained-compiler.tcl").unwrap();
    let context = tcl_registry::model::resolve_environment("tcl8.6").default_context_registry();
    let registry = Arc::clone(context.commands());
    let source = "proc subject {} {if {1} {puts retained} else {puts omitted}}";
    let analysis = Arc::new(source_analysis(source, context));
    let checks = backend
        .compiler_diagnostics_for(&uri, source, &analysis, &registry)
        .await;
    let original = checks
        .checks
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::O100)
        .expect("conditional Logical source branch advice reaches the pull adapter");
    assert!(original.source_context.is_some());
    let inputs = CodeActionReportInputs {
        published: Vec::new(),
        registry,
        generic_patterns: None,
        evidence: None,
        layers: PolicyLayers::default(),
        style_line_length: 120,
        xc_for_irules: false,
    };
    let document = DocumentState::new(source.to_owned(), "tcl".to_owned());
    let report = code_action_report(&document, &analysis, &inputs);
    let forwarded = report
        .iter()
        .find_map(|(finding, _)| {
            finding
                .original_compiler_diagnostic()
                .filter(|diagnostic| diagnostic.code == DiagCode::O100)
        })
        .expect("the lightbulb retains the typed compiler issuer");
    assert_eq!(forwarded, original);

    let mut missing = analysis.as_ref().clone();
    missing.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl8.6".to_owned(),
        overlay: 0x345,
    });
    let report = code_action_report(&document, &missing, &inputs);
    assert!(
        report
            .iter()
            .all(|(finding, _)| finding.original_compiler_diagnostic().is_none())
    );
    assert_eq!(
        report.analysis_context_unavailable(),
        missing.analysis_context_unavailable.as_ref()
    );
    let checks = backend
        .compiler_diagnostics_for(&uri, source, &Arc::new(missing), &inputs.registry)
        .await;
    assert!(checks.checks.is_empty() && checks.optimisations.is_empty());
}
