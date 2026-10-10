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

fn whole_source_range(source: &str) -> CoreLspRange {
    CoreLspRange {
        start_line: 0,
        start_character: 0,
        end_line: 0,
        end_character: u32::try_from(source.len()).unwrap(),
    }
}

async fn workspace_pack_backend() -> (Backend, Arc<CommandRegistry>) {
    let backend = crate::tests::test_backend();
    let registry = install_workspace_read_pack(&backend, 1, true).await;
    (backend, registry)
}

async fn install_workspace_read_pack(
    backend: &Backend,
    sequence: u64,
    reads: bool,
) -> Arc<CommandRegistry> {
    let role = if reads { "arg 0 -role VarRead" } else { "" };
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: PathBuf::from("/workspace/.tcl-lsp/unindexed-metadata.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        format!(
            "speclib unindexed_metadata 1.0 {{\ncommand unindexed_metadata::read {{\narity 1\n{role}\n}}\n}}\n"
        ),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &packs);
    *backend.spec_packs.lock().await = PublishedPackSet {
        seq: sequence,
        packs: Arc::new(packs),
    };
    backend.sync_db_config().await;
    registry
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unindexed_semantic_tokens_keep_workspace_pack_roles() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Authoring roles only; loading this spec grants no native command entry.
    let (backend, registry) = workspace_pack_backend().await;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unindexed_range_request_keeps_workspace_pack_roles() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The public viewport provider consumes the actual workspace source roles.
    let (backend, _) = workspace_pack_backend().await;
    let uri = Uri::from_str("file:///workspace/unindexed-viewport.tcl").unwrap();
    let source = "unindexed_metadata::read retained";
    let document = DocumentState::new(source.to_owned(), "tcl".to_owned());
    backend
        .documents
        .lock("test")
        .await
        .insert(uri.clone(), document.clone());
    assert!(!backend.db_files.lock().await.contains_key(&uri));
    let result = backend
        .semantic_tokens_range(SemanticTokensRangeParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(0, u32::try_from(source.len()).unwrap()),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        })
        .await
        .unwrap();
    let Some(SemanticTokensRangeResult::Tokens(tokens)) = result else {
        panic!("the unindexed document has a viewport response");
    };
    let full = backend
        .semantic_tokens_core_data(&uri, &document)
        .await
        .unwrap();
    assert!(is_variable(&full, source, "retained"));
    assert_eq!(tokens.data, lift_semantic_token_data(&full));
    assert!(!backend.db_files.lock().await.contains_key(&uri));
    assert!(
        backend
            .semantic_tokens_convergence
            .lock()
            .unwrap()
            .is_empty()
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
    assert!(is_variable(
        &unindexed_semantic_tokens_for_range(source, &analysis, Some(whole_source_range(source)))
            .data,
        source,
        "retained"
    ));
    assert!(!is_variable(
        &unindexed_semantic_tokens_for_range(
            source,
            &unavailable,
            Some(whole_source_range(source))
        )
        .data,
        source,
        "retained"
    ));
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    let mut unavailable_generation = analysis.clone();
    unavailable_generation.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl8.6".to_owned(),
        overlay: 0x347,
    });
    let mut stale = analysis.clone();
    stale.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    let mut foreign = analysis.clone();
    foreign.resolved_input = source_analysis(
        source,
        tcl_registry::model::resolve_environment("tcl9.1").default_context_registry(),
    )
    .resolved_input;
    for refused in [missing, unavailable_generation, stale, foreign] {
        assert!(unindexed_semantic_tokens(source, &refused).data.is_empty());
        assert!(
            unindexed_semantic_tokens_for_range(source, &refused, Some(whole_source_range(source)))
                .data
                .is_empty()
        );
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unindexed_deep_compiler_checks_borrow_the_shared_base_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Conditional Logical source advice retains its issuer; no execution entry.
    let backend = crate::tests::test_backend();
    let uri = Uri::from_str("file:///workspace/unindexed-deep.tcl").unwrap();
    let context = tcl_registry::model::resolve_environment("tcl8.6").default_context_registry();
    let registry = Arc::clone(context.commands());
    let source = "proc subject {} {if {1} {puts retained} else {puts omitted}}";
    let analysis = Arc::new(source_analysis(source, context));
    let ctx = SalsaAnalysisCtx {
        db: &backend.db,
        uri: &uri,
        file: None,
        config: *backend.db_config.lock().await,
        text: source,
        dialect: tcl_lsp_core::profile_for_dialect("tcl"),
    };
    let walks = std::sync::atomic::AtomicUsize::new(0);
    let base = async {
        walks.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        ControlFlow::Continue(Arc::clone(&analysis))
    }
    .shared();
    let (base_result, checks) = tokio::join!(
        base.clone(),
        compute_compiler_diags_after_base(&ctx, &registry, None, base),
    );
    assert_eq!(walks.load(std::sync::atomic::Ordering::Relaxed), 1);
    let ControlFlow::Continue(base_analysis) = base_result else {
        panic!("the original base analysis completed");
    };
    assert!(Arc::ptr_eq(&base_analysis, &analysis));
    let ControlFlow::Continue(checks) = checks else {
        panic!("the supplied compiler projection completed");
    };
    let diagnostic = checks
        .checks
        .iter()
        .find(|diagnostic| diagnostic.code == DiagCode::O100)
        .expect("the conditional source branch reaches the deep worker");
    let issuer = diagnostic.source_context.as_ref().unwrap();
    assert_eq!(
        issuer.lexer_config(),
        analysis.resolved_input.as_ref().unwrap().lexer_config()
    );
    assert_eq!(
        issuer.registry().semantic_key(),
        registry.snapshot().semantic_key()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unindexed_deep_compiler_checks_refuse_missing_base_and_preserve_cancellation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let backend = crate::tests::test_backend();
    let uri = Uri::from_str("file:///workspace/unindexed-deep-refused.tcl").unwrap();
    let context = tcl_registry::model::resolve_environment("tcl8.6").default_context_registry();
    let registry = Arc::clone(context.commands());
    let source = "proc subject {} {if {1} {puts retained} else {puts omitted}}";
    let analysis = source_analysis(source, context);
    let ctx = SalsaAnalysisCtx {
        db: &backend.db,
        uri: &uri,
        file: None,
        config: *backend.db_config.lock().await,
        text: source,
        dialect: tcl_lsp_core::profile_for_dialect("tcl"),
    };
    let mut missing = analysis.clone();
    missing.resolved_input = None;
    let mut unavailable = analysis.clone();
    unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl8.6".to_owned(),
        overlay: 0x347,
    });
    let mut stale = analysis;
    stale.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    for refused in [missing, unavailable, stale] {
        let result = compute_compiler_diags_after_base(
            &ctx,
            &registry,
            None,
            std::future::ready(ControlFlow::Continue(Arc::new(refused))),
        )
        .await;
        let ControlFlow::Continue(checks) = result else {
            panic!("refused metadata withholds only compiler advice");
        };
        assert!(checks.checks.is_empty() && checks.optimisations.is_empty());
    }
    let ControlFlow::Continue(checks) = compute_compiler_diags(&ctx, &registry, None, None).await
    else {
        panic!("missing supplied analysis withholds compiler advice");
    };
    assert!(checks.checks.is_empty() && checks.optimisations.is_empty());
    for settled in [false, true] {
        let result = compute_compiler_diags_after_base(
            &ctx,
            &registry,
            None,
            std::future::ready(ControlFlow::Break(settled)),
        )
        .await;
        assert!(matches!(result, ControlFlow::Break(actual) if actual == settled));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_class_analysis_keeps_actual_pack_input_and_reloads_changed_roles() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The real secondary consumer keeps source hints separate from Native class identity.
    let (backend, registry) = workspace_pack_backend().await;
    let uri = Uri::from_str("file:///workspace/workspace-class-metadata.tcl").unwrap();
    let source = "unindexed_metadata::read retained";
    backend.db_set_source(&uri, source, "tcl".to_owned()).await;
    let profile = tcl_lsp_core::profile_for_dialect("tcl");
    let analysis = backend
        .analyse_with_workspace_classes(&uri, source, profile)
        .await;
    assert_eq!(
        analysis
            .resolved_registry()
            .unwrap()
            .snapshot()
            .semantic_key(),
        registry.snapshot().semantic_key()
    );
    assert!(is_variable(
        &unindexed_semantic_tokens(source, &analysis).data,
        source,
        "retained"
    ));
    let cached = backend
        .analyse_with_workspace_classes(&uri, source, profile)
        .await;
    assert!(Arc::ptr_eq(&cached, &analysis));

    let changed_registry = install_workspace_read_pack(&backend, 2, false).await;
    let changed = backend
        .analyse_with_workspace_classes(&uri, source, profile)
        .await;
    assert!(!Arc::ptr_eq(&changed, &analysis));
    assert_eq!(
        changed
            .resolved_registry()
            .unwrap()
            .snapshot()
            .semantic_key(),
        changed_registry.snapshot().semantic_key()
    );
    assert!(!is_variable(
        &unindexed_semantic_tokens(source, &changed).data,
        source,
        "retained"
    ));
    assert_ne!(changed.resolved_input, analysis.resolved_input);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_class_analysis_refuses_withdrawn_base_before_cached_hints() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (backend, _) = workspace_pack_backend().await;
    let uri = Uri::from_str("file:///workspace/workspace-class-withdrawn.tcl").unwrap();
    let source = "unindexed_metadata::read retained";
    let profile = tcl_lsp_core::profile_for_dialect("tcl");
    let base = backend
        .fresh_analysis_for(&uri, Arc::from(source), "tcl".to_owned())
        .await;
    let current = backend
        .analyse_with_workspace_classes_from_analysis(&uri, source, profile, Arc::clone(&base))
        .await;
    assert!(is_variable(
        &unindexed_semantic_tokens(source, &current).data,
        source,
        "retained"
    ));
    let mut missing = (*base).clone();
    missing.resolved_input = None;
    let mut unavailable = (*base).clone();
    unavailable.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
        environment: "tcl".to_owned(),
        overlay: 0x350,
    });
    let mut stale = (*base).clone();
    stale.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    let mut foreign = (*base).clone();
    foreign.resolved_input = Some(
        source_analysis(
            source,
            tcl_registry::model::resolve_environment("tcl9.1").default_context_registry(),
        )
        .resolved_input
        .unwrap(),
    );
    for withdrawn in [missing, unavailable, stale, foreign] {
        let withdrawn = Arc::new(withdrawn);
        let result = backend
            .analyse_with_workspace_classes_from_analysis(
                &uri,
                source,
                profile,
                Arc::clone(&withdrawn),
            )
            .await;
        assert!(Arc::ptr_eq(&result, &withdrawn));
        assert!(tcl_compiler::source_graph::current_analysis(source, &result).is_none());
        assert!(Arc::ptr_eq(
            &backend
                .workspace_class_analyses
                .lock()
                .await
                .get(&uri)
                .unwrap()
                .analysis,
            &current
        ));
    }
    let stale_source = backend
        .analyse_with_workspace_classes_from_analysis(
            &uri,
            "unindexed_metadata::read changed",
            profile,
            Arc::clone(&base),
        )
        .await;
    assert!(Arc::ptr_eq(&stale_source, &base));
}
