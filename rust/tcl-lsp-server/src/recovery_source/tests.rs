// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use salsa::Setter as _;

async fn recover(backend: &Backend, uri: &Uri, source: &str) -> Arc<AnalysisResult> {
    recover_for_input(backend, uri, source, true).await
}

async fn recover_for_input(
    backend: &Backend,
    uri: &Uri,
    source: &str,
    tracked: bool,
) -> Arc<AnalysisResult> {
    assert!(!tcl_lexer::script_is_complete(source));
    backend
        .db_set_source(uri, source, "tcl8.6".to_owned())
        .await;
    let config = backend.resolved_db_config(uri).await;
    let file = backend.db_files.lock().await[uri];
    let (extra, mode, disabled) = {
        let db = backend.db.lock().await;
        (
            config.extra_commands(&*db).iter().cloned().collect(),
            config.non_ascii_mode(&*db),
            config.disabled_diagnostics(&*db).iter().cloned().collect(),
        )
    };
    let registry = backend.registry_for_dialect("tcl8.6").await;
    let dialects = provider_source::ProviderDialectInputs::capture(backend).await;
    let providers = ProviderCaptureInputs {
        db: &backend.db,
        files: &backend.db_files,
        global: &backend.db_config,
        folders: &backend.folder_db_configs,
        store: &backend.store,
        dialects: &dialects,
    };
    let recovery = RecoveryWidenCtx {
        cache: &backend.recovery_names,
        registry: &registry,
        workspace_index: &backend.workspace_index,
        package_resolver: &backend.package_resolver,
        provider_capture: &providers,
        prefer: backend.default_package_prefer().await,
    };
    let context = SalsaAnalysisCtx {
        db: &backend.db,
        uri,
        file: tracked.then_some(file),
        config,
        text: source,
        dialect: tcl_lsp_core::profile_for_dialect("tcl8.6"),
    };
    let ControlFlow::Continue(result) = compute_base_analysis(
        &backend.client,
        &context,
        &disabled,
        &extra,
        mode,
        &recovery,
    )
    .await
    else {
        panic!("the real malformed-document path did not settle");
    };
    result
}

async fn package_names(backend: &Backend) -> RecoveryNames {
    backend
        .recovery_names
        .lock()
        .await
        .widened
        .as_ref()
        .expect("the actual source requirement reached package recovery")
        .1
        .clone()
}

async fn register(backend: &Backend, source: &str) {
    backend
        .package_resolver
        .write()
        .await
        .add_original_pkg_index(
            source,
            Path::new("/library"),
            Path::new("/library/pkgIndex.tcl"),
            &|_| true,
            &|_| Vec::new(),
        );
}

async fn provider_pack(backend: &Backend) -> tcl_lsp_db::AnalyserConfig {
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: PathBuf::from("/library/recovery362.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        "speclib recovery362 1.0 {
command recovery362::define {
arity 3
arg 0 -role Name
arg 1 -role Params
arg 2 -role Body
analyser_hook -native Proc
command_table_effect DefinesProcedure
}
}"
        .to_owned(),
    )]);
    assert!(packs.notices.is_empty(), "{:#?}", packs.notices);
    let _ = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &packs);
    let config = {
        let mut db = backend.db.lock().await;
        tcl_lsp_db::set_overlay_epoch(&mut db, tcl_registry::overlay_epoch());
        tcl_lsp_db::AnalyserConfig::new(
            &*db,
            Vec::new(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
            packs.key,
            Vec::new(),
            Vec::new(),
        )
    };
    backend
        .folder_db_configs
        .lock()
        .await
        .push((Uri::from_file_path("/library").unwrap(), config));
    config
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_recovery_keeps_provider_input_source_and_unavailable_status() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // These are parser recognition candidates, not installed command receipts.
    let backend = crate::tests::test_backend();
    let config = provider_pack(&backend).await;
    let library = Uri::from_file_path("/library/owned.tcl").unwrap();
    backend
        .db_set_source(
            &library,
            "recovery362::define provided {} {}",
            "tcl".to_owned(),
        )
        .await;
    register(&backend, "package ifneeded own 1.0 {source owned.tcl}").await;
    let uri = Uri::from_file_path("/caller/incomplete.tcl").unwrap();
    let source = "package require own\nprovided\nproc unfinished {";
    let analysis = recover(&backend, &uri, source).await;
    let first = package_names(&backend).await;
    assert!(first.names.contains("provided"));
    let caller_config = backend.resolved_db_config(&uri).await;
    let expected = {
        let db = backend.db.lock().await;
        let file = backend.db_files.lock().await[&uri];
        tcl_lsp_db::document_analysis_input(&*db, file, caller_config).unwrap()
    };
    assert_eq!(analysis.resolved_input.as_ref(), Some(expected.as_ref()));
    assert!(analysis.matches_original_source_image(
        &tcl_lexer::SourceImage::document(source),
        expected.lexer_config()
    ));
    recover(&backend, &uri, source).await;
    assert!(Arc::ptr_eq(
        &first.names,
        &package_names(&backend).await.names
    ));

    let untracked = recover_for_input(&backend, &uri, source, false).await;
    let untracked_input = untracked.resolved_input.as_ref().unwrap();
    assert_eq!(untracked_input.unit_profile(), expected.unit_profile());
    assert_eq!(untracked_input.lexer_config(), expected.lexer_config());
    assert!(untracked.matches_original_source_image(
        &tcl_lexer::SourceImage::document(source),
        untracked_input.lexer_config()
    ));
    assert!(package_names(&backend).await.names.contains("provided"));

    // Only the provider's text changes; neither resolver/index generation nor
    // its configured overlay changes. The old name allocation cannot survive.
    backend
        .db_set_source(
            &library,
            "recovery362::define replacement {} {}",
            "tcl".to_owned(),
        )
        .await;
    recover(&backend, &uri, source).await;
    let changed = package_names(&backend).await;
    assert!(!Arc::ptr_eq(&first.names, &changed.names));
    assert!(!changed.names.contains("provided"));
    assert!(changed.names.contains("replacement"));
    {
        let mut db = backend.db.lock().await;
        config.set_spec_pack_key(&mut *db).to(u64::MAX - 362);
    }
    let unavailable = recover(&backend, &uri, source).await;
    let names = package_names(&backend).await;
    assert!(!names.names.contains("provided"));
    assert!(!names.names.contains("replacement"));
    assert!(names.incomplete);
    assert!(unavailable.has_dynamic_providers);
    assert!(
        backend
            .recovery_names
            .lock()
            .await
            .widened
            .as_ref()
            .unwrap()
            .0
            .providers
            .iter()
            .any(|provider| matches!(
                provider.source.as_ref(),
                Err(ProviderSourceUnavailable::Overlay(_))
            ))
    );

    assert_unavailable_caller(&backend, &uri, source).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_recovery_keeps_exact_preference_and_unknown_provider_barriers() {
    // naming.server.original-package-consumer-resolution
    // docs/design/analysis/name-resolution-proofs/server-original-package-consumer-resolution.md
    // The real recovery entry retains each provider's independent source input.
    let store = Arc::new(vfs::MemoryStore::new());
    store.upsert(
        "/library/stable.tcl",
        b"proc old_command {} {}\npackage prefer latest\npackage require dep 1.0".to_vec(),
    );
    store.upsert("/library/beta.tcl", b"proc beta_command {} {}".to_vec());
    store.upsert("/library/dep-old.tcl", b"proc dep_old {} {}".to_vec());
    store.upsert("/library/dep-beta.tcl", b"proc dep_beta {} {}".to_vec());
    let mut backend = crate::tests::test_backend();
    backend.store = store;
    register(&backend, "package ifneeded w 1.2 {source stable.tcl}\npackage ifneeded w 1.3b1 {source beta.tcl}\npackage ifneeded dep 1.1 {source dep-old.tcl}\npackage ifneeded dep 1.2b1 {source dep-beta.tcl}\npackage ifneeded absent 1.0 {source absent.tcl}").await;
    let uri = Uri::from_file_path("/caller/versioned.tcl").unwrap();
    recover(
        &backend,
        &uri,
        "package require -exact w 1.2\nproc unfinished {",
    )
    .await;
    let stable = package_names(&backend).await;
    assert!(stable.names.contains("old_command"));
    assert!(stable.names.contains("dep_beta"));
    assert!(!stable.names.contains("beta_command"));
    assert!(!stable.names.contains("dep_old"));
    recover(
        &backend,
        &uri,
        "package prefer latest\npackage require w 1.0\nproc unfinished {",
    )
    .await;
    let beta = package_names(&backend).await;
    assert!(beta.names.contains("beta_command"));
    assert!(!beta.names.contains("old_command"));
    assert!(!Arc::ptr_eq(&stable.names, &beta.names));
    let absent = recover(&backend, &uri, "package require absent\nproc unfinished {").await;
    assert!(absent.has_dynamic_providers);
    let names = package_names(&backend).await;
    assert!(names.incomplete);
    assert!(!names.names.contains("old_command"));
    assert!(!names.names.contains("beta_command"));
    let dynamic = recover(
        &backend,
        &uri,
        "package require $computed\nproc unfinished {",
    )
    .await;
    assert!(dynamic.has_dynamic_providers);

    let builtin = recover(&backend, &uri, "package require Tcl\nproc unfinished {").await;
    assert!(!package_names(&backend).await.incomplete);
    assert!(!builtin.has_dynamic_providers);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn malformed_recovery_refuses_populated_unavailable_missing_and_foreign_sources() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // Copied source facts cannot bypass the complete current-input guard.
    let store = Arc::new(vfs::MemoryStore::new());
    store.upsert("/library/owned.tcl", b"proc provided {} {}".to_vec());
    let mut backend = crate::tests::test_backend();
    backend.store = store;
    register(&backend, "package ifneeded own 1.0 {source owned.tcl}").await;
    let uri = Uri::from_file_path("/caller/guards.tcl").unwrap();
    let source = "package require own\nproc unfinished {";
    let current = recover(&backend, &uri, source).await;
    assert!(package_names(&backend).await.names.contains("provided"));
    assert!(!current.package_requires.is_empty());
    let registry = backend.registry_for_dialect("tcl8.6").await;
    let dialects = provider_source::ProviderDialectInputs::capture(&backend).await;
    let providers = ProviderCaptureInputs {
        db: &backend.db,
        files: &backend.db_files,
        global: &backend.db_config,
        folders: &backend.folder_db_configs,
        store: &backend.store,
        dialects: &dialects,
    };
    let context = RecoveryWidenCtx {
        cache: &backend.recovery_names,
        registry: &registry,
        workspace_index: &backend.workspace_index,
        package_resolver: &backend.package_resolver,
        provider_capture: &providers,
        prefer: PackagePrefer::Stable,
    };
    let base = HashSet::from(["configured_only".to_owned()]);
    let mut missing = current.as_ref().clone();
    missing.resolved_input = None;
    let mut unavailable = current.as_ref().clone();
    unavailable.analysis_context_unavailable = Some(
        Analyser::new()
            .with_pack_overlay(u64::MAX - 364)
            .prepare_analysis_input("tcl8.6")
            .unwrap_err(),
    );
    let mut foreign = current.as_ref().clone();
    foreign.resolved_input = backend
        .fresh_analysis_for(&uri, Arc::from(source), "tcl8.4".to_owned())
        .await
        .resolved_input
        .clone();
    let mut stale = current.as_ref().clone();
    let original = current.resolved_input.as_ref().unwrap();
    let mut config = original.lexer_config();
    config.strict_quoting = !config.strict_quoting;
    stale.resolved_input = Some(ResolvedAnalysisInput::new(
        original.analyser_profile(),
        original.unit_profile(),
        original.context_registry(),
        config,
    ));
    for refused in [missing, unavailable, foreign, stale] {
        let names = widen_recovery_extra_commands(&context, &base, source, &refused).await;
        assert_eq!(names.names.as_ref(), &base);
        assert!(names.incomplete);
    }
    let changed_source = format!("# changed\n{source}");
    let refused = widen_recovery_extra_commands(&context, &base, &changed_source, &current).await;
    assert_eq!(refused.names.as_ref(), &base);
    assert!(refused.incomplete);
}

async fn assert_unavailable_caller(backend: &Backend, uri: &Uri, source: &str) {
    // An unavailable caller does not walk the source or borrow the last
    // provider capture, even though its configured known-name cache exists.
    let caller_config = backend.resolved_db_config(uri).await;
    {
        let mut db = backend.db.lock().await;
        caller_config.set_spec_pack_key(&mut *db).to(u64::MAX - 363);
    }
    let refused = recover(backend, uri, source).await;
    assert!(refused.analysis_context_unavailable.is_some());
    assert!(refused.resolved_input.is_none());
    assert!(refused.all_procs.is_empty());
}
