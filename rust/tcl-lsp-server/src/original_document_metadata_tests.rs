// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Real document consumers preserve complete source inputs and typed findings.

use super::*;
use tcl_compiler::analyser::ResolvedAnalysisInput;
use tower_lsp_server::ls_types::{
    PartialResultParams, TextDocumentIdentifier, WorkDoneProgressParams,
};

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

fn index_metadata_directory() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let next = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("tcl-lsp-index352-{}-{next}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

async fn index_reads(backend: &Backend, name: &str, uri: &Uri) -> Vec<tcl_lexer::Span> {
    backend
        .workspace_index
        .read()
        .await
        .variable_refs_of(name, "")
        .into_iter()
        .filter(|row| row.uri == uri.as_str())
        .map(|row| row.span)
        .collect()
}

fn index_evidence_handles(backend: &Backend) -> EvidenceHandles {
    EvidenceHandles {
        db_config: Arc::clone(&backend.db_config),
        folder_db_configs: Arc::clone(&backend.folder_db_configs),
        db: Arc::clone(&backend.db),
        db_files: Arc::clone(&backend.db_files),
        db_project_members: Arc::clone(&backend.db_project_members),
        db_project: Arc::clone(&backend.db_project),
        workspace_index: Arc::clone(&backend.workspace_index),
        documents: Arc::clone(&backend.documents),
        rehoming_gate: Arc::clone(&backend.rehoming_gate),
        live_publication_gate: Arc::clone(&backend.live_publication_gate),
        class_factory_generation: Arc::clone(&backend.class_factory_generation),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closed_disk_index_keeps_workspace_roles_and_withdraws_changed_pack() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // These are published source references, without a Native command entry.
    let (backend, _) = workspace_pack_backend().await;
    let root = index_metadata_directory();
    let file = root.join("closed.tcl");
    let source = "unindexed_metadata::read ::retained\n";
    std::fs::write(&file, source).unwrap();
    let uri = Uri::from_file_path(&file).unwrap();
    backend.reindex_index_from_disk(&uri).await;
    let reads = index_reads(&backend, "::retained", &uri).await;
    assert_eq!(reads.len(), 1);
    assert_eq!(&source[reads[0].as_range()], "::retained");

    install_workspace_read_pack(&backend, 2, false).await;
    backend.invalidate_diag_inputs();
    backend
        .batch_reindex_from_disk(std::slice::from_ref(&uri))
        .await;
    assert!(index_reads(&backend, "::retained", &uri).await.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_index_and_factory_reindex_keep_checked_document_roles() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (backend, _) = workspace_pack_backend().await;
    let root = index_metadata_directory();
    let file = root.join("consumer.tcl");
    let source = "::Meta create Thing {}\nunindexed_metadata::read ::retained\n";
    std::fs::write(&file, source).unwrap();
    let uri = Uri::from_file_path(&file).unwrap();
    *backend.workspace_folders.lock().await = vec![Uri::from_file_path(&root).unwrap()];
    backend.scan_workspace_folders().await;
    assert_eq!(index_reads(&backend, "::retained", &uri).await.len(), 1);
    let handles = index_evidence_handles(&backend);
    assert!(
        backend
            .workspace_index
            .read()
            .await
            .documents_invoking_classes(&HashSet::from(["Meta"]))
            .contains(uri.as_str())
    );
    let affected = HashSet::from(["::Meta".to_owned()]);
    reindex_unopened_factory_consumers(&handles, &affected).await;
    assert_eq!(index_reads(&backend, "::retained", &uri).await.len(), 1);

    install_workspace_read_pack(&backend, 2, false).await;
    backend.invalidate_diag_inputs();
    reindex_unopened_factory_consumers(&handles, &affected).await;
    assert!(index_reads(&backend, "::retained", &uri).await.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn source_rehoming_keeps_workspace_roles_in_the_actual_seed_namespace() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The written source namespace is conditional source advice, not a Native frame.
    let (backend, _) = workspace_pack_backend().await;
    let root = index_metadata_directory();
    let caller_path = root.join("caller.tcl");
    let sourced_path = root.join("sourced.tcl");
    std::fs::write(&caller_path, "namespace eval ::app {source sourced.tcl}\n").unwrap();
    std::fs::write(
        &sourced_path,
        "set retained 1\nunindexed_metadata::read retained\n",
    )
    .unwrap();
    let sourced = Uri::from_file_path(&sourced_path).unwrap();
    *backend.workspace_folders.lock().await = vec![Uri::from_file_path(&root).unwrap()];
    backend.scan_workspace_folders().await;
    assert!(
        backend
            .rehomed_source_seeds
            .lock()
            .await
            .get(sourced.as_str())
            .is_some_and(|seeds| seeds.iter().any(|seed| seed == "::app"))
    );
    let before = index_reads(&backend, "::app::retained", &sourced).await;
    assert!(!before.is_empty());
    let read_offset = u32::try_from("set retained 1\nunindexed_metadata::read ".len()).unwrap();
    assert!(before.iter().any(|span| span.start() == read_offset));

    install_workspace_read_pack(&backend, 2, false).await;
    backend.invalidate_diag_inputs();
    backend
        .rehomed_source_seeds
        .lock()
        .await
        .remove(sourced.as_str());
    backend.refresh_source_rehoming().await;
    let after = index_reads(&backend, "::app::retained", &sourced).await;
    assert!(!after.iter().any(|span| span.start() == read_offset));
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn workspace_index_seed_refuses_stale_and_withdrawn_source_owners() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let (backend, _) = workspace_pack_backend().await;
    let uri = Uri::from_str("file:///workspace/index-seed-currency.tcl").unwrap();
    let source = "unindexed_metadata::read ::retained";
    let current = backend
        .fresh_analysis_seed(&uri, Arc::from(source), "tcl".to_owned())
        .await;
    assert!(backend.fresh_index_seed_is_current(source, &current).await);
    let mut missing = current.analysis.as_ref().clone();
    missing.resolved_input = None;
    let mut stale = current.analysis.as_ref().clone();
    stale.body_lexer_config.as_mut().unwrap().strict_quoting ^= true;
    let mut foreign = current.analysis.as_ref().clone();
    foreign.resolved_input = source_analysis(
        source,
        tcl_registry::model::resolve_environment("tcl9.1").default_context_registry(),
    )
    .resolved_input;
    let mut unavailable_with_receipts = current.analysis.as_ref().clone();
    unavailable_with_receipts.analysis_context_unavailable =
        Some(tcl_registry::model::OverlayMiss {
            environment: "tcl".to_owned(),
            overlay: u64::MAX - 352,
        });
    for analysis in [missing, stale, foreign, unavailable_with_receipts] {
        let seed = FreshAnalysisSeed {
            analysis: Arc::new(analysis),
            analyser_inputs_epoch: current.analyser_inputs_epoch,
            class_factory_generation: current.class_factory_generation,
        };
        assert!(!backend.fresh_index_seed_is_current(source, &seed).await);
    }
    assert!(
        !backend
            .fresh_index_seed_is_current("unindexed_metadata::read ::changed", &current)
            .await
    );
    backend.invalidate_diag_inputs();
    assert!(!backend.fresh_index_seed_is_current(source, &current).await);

    use salsa::Setter as _;
    {
        let mut db = backend.db.lock().await;
        backend
            .db_config
            .lock()
            .await
            .set_spec_pack_key(&mut *db)
            .to(u64::MAX - 352);
    }
    let unavailable = backend
        .fresh_analysis_seed(&uri, Arc::from(source), "tcl".to_owned())
        .await;
    assert!(unavailable.analysis.analysis_context_unavailable.is_some());
    assert!(unavailable.analysis.resolved_input.is_none());
    assert!(unavailable.analysis.command_invocations.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fresh_index_driver_keeps_keyed_availability_on_the_same_store() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // No appliance execution or handler entry is inferred from configured availability.
    let backend = crate::tests::test_backend();
    let uri = Uri::from_str("file:///workspace/index-availability.tcl").unwrap();
    let source = "when CLIENTSSL_HANDSHAKE {SSL::c3d cert_lifespan 24}";
    let older = backend
        .fresh_analysis_seed(&uri, Arc::from(source), "f5-irules".to_owned())
        .await;
    let older_input = older.analysis.resolved_input.as_ref().unwrap();
    assert!(
        older
            .analysis
            .diagnostics
            .iter()
            .any(|row| row.code == DiagCode::W150)
    );
    *backend.bigip_version.lock().await = Some("21.1.0".to_owned());
    backend.sync_db_config().await;
    backend.invalidate_diag_inputs();
    let current = backend
        .fresh_analysis_seed(&uri, Arc::from(source), "f5-irules".to_owned())
        .await;
    let current_input = current.analysis.resolved_input.as_ref().unwrap();
    assert_ne!(current_input, older_input);
    assert!(Arc::ptr_eq(
        current_input.borrowed_context_registry().commands(),
        older_input.borrowed_context_registry().commands()
    ));
    assert!(backend.fresh_index_seed_is_current(source, &current).await);
    assert!(!backend.fresh_index_seed_is_current(source, &older).await);
    assert!(
        !current
            .analysis
            .diagnostics
            .iter()
            .any(|row| row.code == DiagCode::W150)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn optimise_document_keeps_workspace_roles_and_refuses_unavailable_input() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Source rewrites retain authored pack roles without a Native entry.
    let (backend, registry) = workspace_pack_backend().await;
    let uri = Uri::from_str("file:///workspace/supplied-optimiser.tcl").unwrap();
    let source =
        "proc subject {} {if {1} {unindexed_metadata::read ::retained} else {puts omitted}}";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(source.to_owned(), "tcl".to_owned()),
    );
    backend
        .apply_global_config(&serde_json::json!({
            "optimiser": { "profile": "aggressive" }
        }))
        .await;
    let analysis = backend
        .analysis_for(&uri, Arc::from(source), "tcl".to_owned())
        .await;
    assert!(
        analysis
            .qualified_var_refs
            .iter()
            .any(|row| row.qualified_name == "::retained")
    );
    let mut policy = tcl_lsp_core::diagnostic_policy::PolicyBuilder::new().build();
    policy.optimiser = tcl_lsp_core::diagnostic_policy::OptimiserPolicy::all_on();
    let typed =
        core_report::optimise_under_policy_from_analysis(source, &registry, &analysis, 3, &policy);
    let branch = typed
        .applied
        .iter()
        .find(|row| row.code == DiagCode::O100)
        .expect("the actual pack-backed input produces a source branch rewrite");
    assert_eq!(
        analysis
            .resolved_registry()
            .unwrap()
            .snapshot()
            .semantic_key(),
        registry.snapshot().semantic_key()
    );
    assert!(analysis.matches_original_source_image(
        &tcl_lexer::SourceImage::from(source),
        analysis.body_lexer_config.unwrap()
    ));
    assert!(source.get(branch.span.as_range()).is_some());
    assert!(
        branch
            .replacement
            .contains("unindexed_metadata::read ::retained")
    );
    let response = backend
        .optimise_document_command(&[serde_json::json!(uri.as_str())])
        .await
        .unwrap()
        .unwrap();
    assert!(
        response["optimisations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["code"] == "O100")
    );
    let rewritten = response["source"].as_str().unwrap();
    assert!(rewritten.contains("unindexed_metadata::read ::retained"));
    assert!(!rewritten.contains("omitted"));
    let rebuilt = backend
        .fresh_analysis_for(&uri, Arc::from(rewritten), "tcl".to_owned())
        .await;
    assert!(
        rebuilt
            .qualified_var_refs
            .iter()
            .any(|row| row.qualified_name == "::retained")
    );
    assert_eq!(rebuilt.resolved_input, analysis.resolved_input);
    assert!(!rebuilt.matches_original_source_image(
        &tcl_lexer::SourceImage::from(source),
        rebuilt.body_lexer_config.unwrap()
    ));

    let changed_registry = install_workspace_read_pack(&backend, 2, false).await;
    backend.invalidate_diag_inputs();
    let changed = backend
        .analysis_for(&uri, Arc::from(source), "tcl".to_owned())
        .await;
    assert_ne!(changed.resolved_input, analysis.resolved_input);
    assert_eq!(
        changed
            .resolved_registry()
            .unwrap()
            .snapshot()
            .semantic_key(),
        changed_registry.snapshot().semantic_key()
    );
    assert!(
        !changed
            .qualified_var_refs
            .iter()
            .any(|row| row.qualified_name == "::retained")
    );

    use salsa::Setter as _;
    {
        let mut db = backend.db.lock().await;
        backend
            .db_config
            .lock()
            .await
            .set_spec_pack_key(&mut *db)
            .to(u64::MAX - 353);
    }
    backend.invalidate_diag_inputs();
    let unavailable = backend
        .analysis_for(&uri, Arc::from(source), "tcl".to_owned())
        .await;
    assert!(unavailable.analysis_context_unavailable.is_some());
    let refused = backend
        .optimise_document_command(&[serde_json::json!(uri.as_str())])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(refused["source"], source);
    assert!(refused["optimisations"].as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn project_tokens_use_published_folder_inputs_and_refuse_a_missing_generation() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The real token worker consumes per-file source advice without Native entry.
    use salsa::Setter as _;
    let (backend, _) = workspace_pack_backend().await;
    let folder = Uri::from_str("file:///project-input354/library").unwrap();
    let library_uri = Uri::from_str("file:///project-input354/library/reader.tcl").unwrap();
    let caller_uri = Uri::from_str("file:///project-input354/caller.tcl").unwrap();
    backend
        .apply_folder_configs(vec![(
            folder.clone(),
            FolderConfig {
                non_ascii_mode: Some(NonAsciiMode::Strict),
                ..FolderConfig::default()
            },
        )])
        .await;
    backend
        .db_set_source(
            &library_uri,
            "proc ::reader {name} {unindexed_metadata::read $name}",
            "tcl".to_owned(),
        )
        .await;
    let source = "reader retained";
    backend
        .db_set_source(&caller_uri, source, "tcl".to_owned())
        .await;
    let folder_config = backend.resolved_db_config(&library_uri).await;
    let global_config = backend.resolved_db_config(&caller_uri).await;
    assert!(folder_config != global_config);
    let library = *backend.db_files.lock().await.get(&library_uri).unwrap();
    let caller = *backend.db_files.lock().await.get(&caller_uri).unwrap();
    let project = backend.db_project.lock().await.unwrap();
    let original_mapping = {
        let snapshot = backend.db.snapshot("project_token_mapping354").await;
        let mapping = project.token_configurations(&*snapshot).clone().unwrap();
        assert!(mapping.contains(&(library, folder_config)));
        assert!(mapping.contains(&(caller, global_config)));
        mapping
    };
    let tokens = backend
        .db_semantic_tokens(&caller_uri)
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    assert!(is_variable(&tokens.data, source, "retained"));

    {
        let mut db = backend.db.lock().await;
        folder_config.set_spec_pack_key(&mut *db).to(u64::MAX - 354);
    }
    let tokens = backend
        .db_semantic_tokens(&caller_uri)
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    assert!(
        !is_variable(&tokens.data, source, "retained"),
        "the caller must not donate its available input to the library"
    );
    backend.sync_db_config().await;
    let restored = backend
        .db_semantic_tokens(&caller_uri)
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();
    assert!(is_variable(&restored.data, source, "retained"));

    // Removing the folder override republishes handles without changing sources.
    backend.apply_folder_configs(Vec::new()).await;
    let snapshot = backend
        .db
        .snapshot("project_token_changed_mapping354")
        .await;
    let mapping = project.token_configurations(&*snapshot).as_ref().unwrap();
    assert!(mapping.contains(&(library, global_config)));
    assert!(!mapping.contains(&(library, folder_config)));
    assert!(original_mapping.contains(&(library, folder_config)));
    assert!(!original_mapping.contains(&(library, global_config)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn project_token_config_publication_drops_partial_locks_and_preserves_snapshots() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let backend = crate::tests::test_backend();
    let first_uri = Uri::from_str("file:///project-publication354/first.tcl").unwrap();
    backend
        .db_set_source(&first_uri, "proc first {} {}", "tcl".to_owned())
        .await;
    let project = backend.db_project.lock().await.unwrap();
    let global = *backend.db_config.lock().await;
    let first = *backend.db_files.lock().await.get(&first_uri).unwrap();
    let snapshot = backend
        .db
        .snapshot("project_token_before_publication354")
        .await;
    assert!(project.token_configurations(&*snapshot).as_ref().unwrap() == &vec![(first, global)]);
    assert!(matches!(
        backend.try_live_source_locks(),
        Err(LivePublicationWait::SalsaSnapshots)
    ));
    drop(snapshot);
    let folder_guard = backend.folder_db_configs.lock().await;
    assert!(matches!(
        backend.try_live_source_locks(),
        Err(LivePublicationWait::FolderConfigs)
    ));
    assert!(backend.db.try_lock().is_ok());
    assert!(backend.db_files.try_lock().is_ok());
    let snapshot = backend
        .db
        .snapshot("project_token_while_config_locked354")
        .await;
    assert!(project.token_configurations(&*snapshot).as_ref().unwrap() == &vec![(first, global)]);
    drop(snapshot);
    drop(folder_guard);
    let second_uri = Uri::from_str("file:///project-publication354/second.tcl").unwrap();
    let locks = backend.try_live_source_locks().ok().unwrap();
    Backend::set_live_db_source_locked(locks, &second_uri, "proc second {} {}", "tcl", true);
    let second = *backend.db_files.lock().await.get(&second_uri).unwrap();
    let snapshot = backend
        .db
        .snapshot("project_token_after_publication354")
        .await;
    let mapping = project.token_configurations(&*snapshot).as_ref().unwrap();
    assert_eq!(mapping.len(), 2);
    assert!(mapping.contains(&(first, global)));
    assert!(mapping.contains(&(second, global)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callback_project_worker_uses_each_librarys_published_configuration() {
    // naming.database.original-project-callback-projection
    // docs/design/analysis/name-resolution-proofs/database-original-project-callback-projection.md
    // Authentic source headers/counts remain independent of callback execution.
    use salsa::Setter as _;
    let backend = crate::tests::test_backend();
    let folder = Uri::from_str("file:///project-source356/library").unwrap();
    let library_uri = Uri::from_str("file:///project-source356/library/header.tcl").unwrap();
    let caller_uri = Uri::from_str("file:///project-source356/caller.tcl").unwrap();
    backend
        .apply_folder_configs(vec![(
            folder,
            FolderConfig {
                non_ascii_mode: Some(NonAsciiMode::Strict),
                ..FolderConfig::default()
            },
        )])
        .await;
    backend
        .db_set_source(&library_uri, "proc external {one} {}", "tcl8.6".to_owned())
        .await;
    let source = "lsort -command ::external {2 1}";
    backend
        .db_set_source(&caller_uri, source, "tcl8.6".to_owned())
        .await;
    let library_config = backend.resolved_db_config(&library_uri).await;
    let caller_config = backend.resolved_db_config(&caller_uri).await;
    assert!(library_config != caller_config);
    let analysis = backend
        .analysis_for(&caller_uri, Arc::from(source), "tcl8.6".to_owned())
        .await;
    assert!(
        analysis
            .command_invocations
            .iter()
            .any(|row| row.original_callback_signature_lookup.is_some())
    );
    let current = backend
        .project_callback_diagnostics_if(true, source, &analysis, &HashSet::new())
        .await
        .unwrap();
    assert!(
        current
            .iter()
            .any(|row| row.code == DiagCode::E003 && row.callback_source_arity().is_some())
    );
    assert!(
        backend
            .project_callback_diagnostics_if(false, source, &analysis, &HashSet::new())
            .await
            .is_none()
    );
    {
        let mut db = backend.db.lock().await;
        library_config
            .set_spec_pack_key(&mut *db)
            .to(u64::MAX - 3356);
    }
    let withdrawn = backend
        .project_callback_diagnostics_if(true, source, &analysis, &HashSet::new())
        .await
        .unwrap();
    assert!(
        withdrawn
            .iter()
            .all(|row| row.callback_source_arity().is_none())
    );
    {
        let db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        assert!(
            tcl_lsp_db::document_analysis_input(&*db, files[&caller_uri], caller_config).is_ok()
        );
        assert!(
            tcl_lsp_db::document_analysis_input(&*db, files[&library_uri], library_config).is_err()
        );
    }
    {
        let mut db = backend.db.lock().await;
        library_config.set_spec_pack_key(&mut *db).to(0);
    }
    let restored = backend
        .project_callback_diagnostics_if(true, source, &analysis, &HashSet::new())
        .await
        .unwrap();
    assert!(
        restored
            .iter()
            .any(|row| row.callback_source_arity().is_some())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn configured_factory_rounds_refuse_stale_publication_and_withdraw_unavailable_library_hints()
{
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The real fixpoint publishes conditional manufacturers, not Native receivers.
    use salsa::Setter as _;
    let (backend, _) = workspace_pack_backend().await;
    let folder = Uri::from_str("file:///project-source356/factories").unwrap();
    let library_uri = Uri::from_str("file:///project-source356/factories/meta.tcl").unwrap();
    let caller_uri = Uri::from_str("file:///project-source356/made.tcl").unwrap();
    backend
        .apply_folder_configs(vec![(
            folder,
            FolderConfig {
                non_ascii_mode: Some(NonAsciiMode::Strict),
                ..FolderConfig::default()
            },
        )])
        .await;
    backend.db_set_source(&library_uri,
        "oo::class create ::Meta {superclass oo::class; self method create {name body} {next $name $body}}", "tcl".to_owned()).await;
    backend
        .db_set_source(&caller_uri, "::Meta create ::Made {}", "tcl".to_owned())
        .await;
    let library_config = backend.resolved_db_config(&library_uri).await;
    let caller_config = backend.resolved_db_config(&caller_uri).await;
    assert!(library_config != caller_config);
    let handles = index_evidence_handles(&backend);
    let sync = sync_workspace_class_factories(&handles, None).await;
    assert!(!sync.moved.is_empty());
    let (library, caller, revision, prior) = {
        let db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        let library = files[&library_uri];
        let caller = files[&caller_uri];
        let prior = caller.workspace_class_factories(&*db).clone().unwrap();
        assert!(prior.contains_key("::Meta"));
        (library, caller, tcl_lsp_db::database_revision(&db), prior)
    };
    assert!(matches!(
        sync_workspace_class_factories_round(&handles).await,
        ClassFactoryRound::Settled
    ));
    {
        let mut db = backend.db.lock().await;
        library_config
            .set_spec_pack_key(&mut *db)
            .to(u64::MAX - 4356);
    }
    let stale = apply_workspace_class_factories_if_current(
        &handles,
        revision,
        Some(Arc::clone(&prior)),
        vec![(library_uri.clone(), library), (caller_uri.clone(), caller)],
    )
    .await;
    assert!(matches!(stale, ClassFactoryRound::Cancelled));
    sync_workspace_class_factories(&handles, None).await;
    {
        let db = backend.db.lock().await;
        assert!(tcl_lsp_db::document_analysis_input(&*db, library, library_config).is_err());
        assert!(tcl_lsp_db::document_analysis_input(&*db, caller, caller_config).is_ok());
        assert!(library.workspace_class_factories(&*db).is_none());
        assert!(caller.workspace_class_factories(&*db).is_none());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn project_evidence_publication_withdraws_incomplete_checked_contributors() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Actual host publication retains conditional caller facts, not Native entry.
    use salsa::Setter as _;
    let backend = crate::tests::test_backend();
    let folder = Uri::from_str("file:///project-call357/callers").unwrap();
    let library_uri = Uri::from_str("file:///project-call357/library.tcl").unwrap();
    let caller_uri = Uri::from_str("file:///project-call357/callers/main.tcl").unwrap();
    backend
        .apply_folder_configs(vec![(
            folder,
            FolderConfig {
                non_ascii_mode: Some(NonAsciiMode::Strict),
                ..FolderConfig::default()
            },
        )])
        .await;
    backend
        .db_set_source(
            &library_uri,
            "proc helper {mode} {return $mode}",
            "tcl".to_owned(),
        )
        .await;
    backend
        .db_set_source(
            &caller_uri,
            "source ../library.tcl\n::helper kept",
            "tcl".to_owned(),
        )
        .await;
    let library_config = backend.resolved_db_config(&library_uri).await;
    let caller_config = backend.resolved_db_config(&caller_uri).await;
    assert!(library_config != caller_config);
    let handles = index_evidence_handles(&backend);
    let first = sync_cross_file_evidence(&handles).await;
    assert!(!first.changed.is_empty());
    let library = *backend.db_files.lock().await.get(&library_uri).unwrap();
    let caller = *backend.db_files.lock().await.get(&caller_uri).unwrap();
    {
        let db = backend.db.lock().await;
        assert!(
            library
                .external_call_sites(&*db)
                .as_ref()
                .unwrap()
                .get("::helper")
                .is_some()
        );
    }
    {
        let mut db = backend.db.lock().await;
        caller_config
            .set_spec_pack_key(&mut *db)
            .to(u64::MAX - 2357);
    }
    sync_cross_file_evidence(&handles).await;
    {
        let db = backend.db.lock().await;
        assert!(tcl_lsp_db::document_analysis_input(&*db, library, library_config).is_ok());
        assert!(tcl_lsp_db::document_analysis_input(&*db, caller, caller_config).is_err());
        assert!(library.external_call_sites(&*db).is_none());
        assert!(caller.external_call_sites(&*db).is_none());
    }
    let captured = capture_cross_file_evidence_snapshot(&handles)
        .await
        .unwrap();
    let covered = files_with_covered_load_targets(
        &captured.snapshot,
        &captured.files,
        &captured.members,
        captured.project,
    );
    assert!(!covered.contains(&caller_uri));
    drop(captured);
    {
        let mut db = backend.db.lock().await;
        caller_config.set_spec_pack_key(&mut *db).to(0);
    }
    sync_cross_file_evidence(&handles).await;
    {
        let db = backend.db.lock().await;
        assert!(library.external_call_sites(&*db).is_some());
        assert!(caller.external_call_sites(&*db).is_some());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tk_preview_keeps_configured_constructor_and_reports_unavailable_overlay() {
    // naming.core.original-tk-source-context
    // docs/design/analysis/name-resolution-proofs/core-original-tk-source-context.md
    // This public command projects source hints, without creating a widget.
    use salsa::Setter as _;
    let backend = crate::tests::test_backend();
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: PathBuf::from("/workspace/.tcl-lsp/preview361.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        "speclib preview361 1.0 {
command preview361::frame {
arity 1
creates_instance_at 0
required_package Tk
}
}"
        .to_owned(),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
    *backend.spec_packs.lock().await = PublishedPackSet {
        seq: 361,
        packs: Arc::new(packs),
    };
    backend.sync_db_config().await;
    let uri = Uri::from_file_path("/workspace/preview361.tcl").unwrap();
    let source = "package require Tk\npreview361::frame .kept";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(source.to_owned(), "tcl8.6".to_owned()),
    );
    backend
        .db_set_source(&uri, source, "tcl8.6".to_owned())
        .await;
    let analysis = backend
        .analysis_for(&uri, Arc::from(source), "tcl8.6".to_owned())
        .await;
    assert_eq!(
        analysis
            .resolved_registry()
            .unwrap()
            .snapshot()
            .semantic_key(),
        registry.snapshot().semantic_key()
    );
    let arguments = [serde_json::json!({ "uri": uri.as_str() })];
    let model = backend
        .tk_preview_command(&arguments)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(model["widget_count"], 2, "{model}");
    assert_eq!(model["root"]["children"][0]["path"], ".kept", "{model}");

    let config = *backend.db_config.lock().await;
    {
        let mut db = backend.db.lock().await;
        config.set_spec_pack_key(&mut *db).to(u64::MAX - 361);
    }
    let unavailable = backend
        .tk_preview_command(&arguments)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unavailable["widget_count"], 0, "{unavailable}");
    assert!(unavailable.get("root").is_none(), "{unavailable}");
    assert_eq!(
        unavailable["uncertainties"][0]["kind"], "source_context_unavailable",
        "{unavailable}"
    );
}

async fn formatter_request_edits(backend: &Backend, uri: &Uri) -> [Option<Vec<TextEdit>>; 3] {
    let full: DocumentFormattingParams = serde_json::from_value(serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
        "options": { "tabSize": 2, "insertSpaces": true }
    }))
    .unwrap();
    let range: DocumentRangeFormattingParams = serde_json::from_value(serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
        "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 3, "character": 0 } },
        "options": { "tabSize": 2, "insertSpaces": true }
    }))
    .unwrap();
    let save: WillSaveTextDocumentParams = serde_json::from_value(serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
        "reason": 1
    }))
    .unwrap();
    [
        backend.formatting(full).await.unwrap(),
        backend.range_formatting(range).await.unwrap(),
        backend.will_save_wait_until(save).await.unwrap(),
    ]
}

fn assert_formatter_request_edit(edits: Option<Vec<TextEdit>>, ending: &str) {
    let edits = edits.expect("the current whole source supports a nonempty formatting edit");
    assert_eq!(edits.len(), 1);
    let edit = &edits[0];
    assert_eq!(edit.range.start, Position::new(0, 0));
    assert_eq!(edit.range.end, Position::new(3, 0));
    assert!(
        edit.new_text
            .contains(&format!("{ending}  SSL::c3d cert_lifespan 24{ending}")),
        "{}",
        edit.new_text
    );
    assert!(edit.new_text.ends_with(ending));
    if ending == "\n" {
        assert!(!edit.new_text.contains('\r'));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn original_formatter_requests_keep_checked_availability_raw_coordinates_and_withdrawn_input()
{
    // naming.editor.original-source-formatting
    // docs/design/analysis/name-resolution-proofs/editor-original-source-formatting.md
    // These public requests retain actual source/configuration; no appliance
    // execution or Native body entry is supplied by the event/source roles.
    use salsa::Setter as _;
    let backend = crate::tests::test_backend();
    let uri = Uri::from_str("file:///workspace/formatter-source-currency.tcl").unwrap();
    let source = "when CLIENTSSL_HANDSHAKE {\r\nSSL::c3d cert_lifespan 24\r\n}\r\n";
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(source.to_owned(), "f5-irules".to_owned()),
    );
    backend
        .db_set_source(&uri, source, "f5-irules".to_owned())
        .await;
    assert!(
        backend
            .apply_global_formatting(&serde_json::json!({
                "formatting": { "indentSize": 2, "lineEnding": "auto" }
            }))
            .await
            .is_none()
    );
    let older = backend
        .analysis_for(&uri, Arc::from(source), "f5-irules".to_owned())
        .await;
    let older_input = older
        .resolved_input
        .as_ref()
        .expect("the actual older document input exists");
    assert!(
        older
            .diagnostics
            .iter()
            .any(|row| row.code == DiagCode::W150)
    );
    assert!(!older.command_invocations.is_empty());
    let [full, range, save] = formatter_request_edits(&backend, &uri).await;
    assert_formatter_request_edit(full, "\r\n");
    assert_formatter_request_edit(range, "\r\n");
    assert!(save.is_none(), "format-on-save is opt-in");
    backend
        .feature_toggles
        .lock()
        .await
        .set
        .insert("willSaveWaitUntil".to_owned(), true);

    *backend.bigip_version.lock().await = Some("21.1.0".to_owned());
    backend.sync_db_config().await;
    backend.invalidate_diag_inputs();
    let current = backend
        .analysis_for(&uri, Arc::from(source), "f5-irules".to_owned())
        .await;
    let current_input = current.resolved_input.as_ref().unwrap();
    assert_ne!(current_input, older_input);
    assert!(Arc::ptr_eq(
        current_input.borrowed_context_registry().commands(),
        older_input.borrowed_context_registry().commands(),
    ));
    assert!(
        !current
            .diagnostics
            .iter()
            .any(|row| row.code == DiagCode::W150)
    );
    assert_eq!(
        current.body_lexer_config,
        Some(current_input.lexer_config())
    );
    assert_eq!(
        backend.read_local_document(&uri).await.unwrap().raw(),
        source
    );
    for edits in formatter_request_edits(&backend, &uri).await {
        assert_formatter_request_edit(edits, "\r\n");
    }
    assert!(
        backend
            .apply_global_formatting(&serde_json::json!({
                "formatting": { "indentSize": 2, "lineEnding": "lf" }
            }))
            .await
            .is_none()
    );
    for edits in formatter_request_edits(&backend, &uri).await {
        assert_formatter_request_edit(edits, "\n");
    }
    assert_eq!(
        backend.read_local_document(&uri).await.unwrap().raw(),
        source
    );

    // Withdraw the actual generation through its DB input. Previously retained
    // populated analysis stays independent and cannot donate a formatter owner.
    {
        let mut db = backend.db.lock().await;
        backend
            .db_config
            .lock()
            .await
            .set_spec_pack_key(&mut *db)
            .to(u64::MAX - 379);
    }
    backend.invalidate_diag_inputs();
    let withdrawn = backend
        .analysis_for(&uri, Arc::from(source), "f5-irules".to_owned())
        .await;
    assert!(withdrawn.analysis_context_unavailable.is_some());
    assert!(withdrawn.resolved_input.is_none());
    assert!(!current.command_invocations.is_empty());
    assert!(current.resolved_input.is_some());
    for edits in formatter_request_edits(&backend, &uri).await {
        assert!(
            edits.is_none(),
            "an actual unavailable/missing document owner supplies no formatting edits"
        );
    }
}
