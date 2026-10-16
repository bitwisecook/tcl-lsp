// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use salsa::Setter as _;
use tcl_lsp_core::package_resolver::{PackagePrefer, PackageRequirementAdvice};

async fn capture(backend: &Backend) -> Arc<ConfiguredProviderSources> {
    let dialects = ProviderDialectInputs::capture(backend).await;
    ProviderCaptureInputs {
        db: &backend.db,
        files: &backend.db_files,
        global: &backend.db_config,
        folders: &backend.folder_db_configs,
        store: &backend.store,
        source_path_admissions: &backend.source_path_admissions,
        dialects: &dialects,
        proposal: None,
    }
    .capture(&backend.package_resolver, true)
    .await
}

fn provider_pack() -> tcl_spectcl::pack::PackSet {
    let packs = tcl_spectcl::pack::load_in_memory(vec![(
        tcl_spectcl::PackFile {
            tier: tcl_spectcl::Tier::Workspace,
            path: PathBuf::from("/library/provider360.tclspec"),
            origin: tcl_spectcl::discovery::Origin::DotDir,
            dependency_tier: None,
        },
        "speclib provider360 1.0 {
command provider360::define {
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
    packs
}

async fn folder_configuration(backend: &Backend, overlay: u64) -> tcl_lsp_db::AnalyserConfig {
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
            overlay,
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

async fn register_packages(backend: &Backend, source: &str) {
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn indexed_package_providers_keep_their_own_pack_and_unavailable_status() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // The package scan consumes source structure, without a Native body entry.
    let backend = crate::tests::test_backend();
    let packs = provider_pack();
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl", &packs);
    let config = folder_configuration(&backend, packs.key).await;
    let uri = Uri::from_file_path("/library/owned.tcl").unwrap();
    let source = "provider360::define provided {one} {}";
    backend
        .db_set_source(&uri, source, "tcl".to_owned())
        .await
        .unwrap();
    register_packages(&backend, "package ifneeded own 1.0 {source owned.tcl}").await;
    let (file, expected) = {
        let db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        let file = files[&uri];
        (
            file,
            tcl_lsp_db::document_analysis_input(&*db, file, config).unwrap(),
        )
    };
    let supplied = capture(&backend).await;
    let resolver = backend.package_resolver.read().await;
    let current = supplied
        .inventory(
            Path::new("/library/owned.tcl"),
            Some(tcl_dialect::TclVersion::V8_4),
            &resolver,
        )
        .unwrap();
    assert_eq!(current.input.as_ref(), expected.as_ref());
    assert_eq!(
        current
            .input
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key(),
        registry.snapshot().semantic_key()
    );
    assert!(
        tcl_compiler::command_binding::SourceAnalysisEntry::for_logical_source(&current.input)
            .is_some()
    );
    assert!(
        tcl_compiler::registry_invocation::InvocationMetadataContext::for_source_input(
            &registry,
            &current.input,
            current.input.lexer_config(),
            Some(current.input.unit_profile()),
        )
        .is_some()
    );
    assert!(current.analysis.all_procs.contains_key("::provided"));
    assert!(current.is_current());
    assert!(
        !Analyser::new()
            .structure_only()
            .analyse(source, "tcl")
            .all_procs
            .contains_key("::provided")
    );
    drop(resolver);

    {
        let mut db = backend.db.lock().await;
        config.set_spec_pack_key(&mut *db).to(0);
    }
    let available_without_pack = capture(&backend).await;
    let resolver = backend.package_resolver.read().await;
    let changed = available_without_pack
        .inventory(
            Path::new("/library/owned.tcl"),
            Some(tcl_dialect::TclVersion::V8_4),
            &resolver,
        )
        .unwrap();
    assert!(!changed.analysis.all_procs.contains_key("::provided"));
    drop(resolver);
    {
        let mut db = backend.db.lock().await;
        config.set_spec_pack_key(&mut *db).to(u64::MAX - 360);
        assert!(tcl_lsp_db::document_analysis_input(&*db, file, config).is_err());
    }
    let withdrawn = capture(&backend).await;
    let resolver = backend.package_resolver.read().await;
    assert!(matches!(
        withdrawn.inventory(
            Path::new("/library/owned.tcl"),
            Some(tcl_dialect::TclVersion::V8_4),
            &resolver
        ),
        Err(ProviderSourceUnavailable::Overlay(_))
    ));
    // The prior immutable source snapshot retains its own input; the new
    // capture cannot borrow that receipt or the caller's available context.
    assert!(Arc::ptr_eq(
        &current,
        &supplied
            .inventory(
                Path::new("/library/owned.tcl"),
                Some(tcl_dialect::TclVersion::V8_4),
                &resolver
            )
            .unwrap()
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disk_package_providers_keep_source_dialect_and_refuse_uncaptured_paths() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let store = Arc::new(vfs::MemoryStore::new());
    store.upsert(
        "/library/own.tcl",
        b"# tcl-dialect: tcl9.0\nproc provided {} {}".to_vec(),
    );
    let mut backend = crate::tests::test_backend();
    backend.store = store.clone();
    backend.folder_dialects.lock().await.push((
        Uri::from_file_path("/library").unwrap(),
        "tcl8.6".to_owned(),
    ));
    register_packages(&backend, "package ifneeded own 1.0 {source own.tcl}\npackage ifneeded absent 1.0 {source absent.tcl}").await;
    let supplied = capture(&backend).await;
    let resolver = backend.package_resolver.read().await;
    let current = supplied
        .inventory(
            Path::new("/library/own.tcl"),
            Some(tcl_dialect::TclVersion::V8_4),
            &resolver,
        )
        .unwrap();
    assert_eq!(
        current.input.unit_profile(),
        tcl_lsp_core::profile_for_dialect("tcl9.0")
    );
    assert!(current.analysis.all_procs.contains_key("::provided"));
    assert!(!defined_original_commands_from_analysis(&current.text, &current.analysis).is_empty());
    assert!(matches!(
        supplied.inventory(
            Path::new("/library/absent.tcl"),
            Some(tcl_dialect::TclVersion::V8_4),
            &resolver
        ),
        Err(ProviderSourceUnavailable::MissingSource)
    ));
    assert!(matches!(
        supplied.inventory(
            Path::new("/library/unregistered.tcl"),
            Some(tcl_dialect::TclVersion::V8_4),
            &resolver
        ),
        Err(ProviderSourceUnavailable::MissingMapping)
    ));
    drop(resolver);
    register_packages(&backend, "package ifneeded later 1.0 {source later.tcl}").await;
    assert!(matches!(
        supplied.inventory(
            Path::new("/library/later.tcl"),
            Some(tcl_dialect::TclVersion::V8_4),
            &*backend.package_resolver.read().await
        ),
        Err(ProviderSourceUnavailable::ResolverChanged)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn configured_package_refinement_keeps_version_preference_and_unknown_providers() {
    // naming.server.original-package-consumer-resolution
    // docs/design/analysis/name-resolution-proofs/server-original-package-consumer-resolution.md
    // Source publications refine advice; this does not execute any loader.
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
    register_packages(&backend, "package ifneeded w 1.2 {source stable.tcl}\npackage ifneeded w 1.3b1 {source beta.tcl}\npackage ifneeded dep 1.1 {source dep-old.tcl}\npackage ifneeded dep 1.2b1 {source dep-beta.tcl}\npackage ifneeded absent 1.0 {source absent.tcl}").await;
    let supplied = capture(&backend).await;
    let sources = ProviderSourceAccess::Supplied(&supplied);
    let dialect = tcl_lsp_core::profile_for_dialect("tcl8.6");
    let source = "old_command\nbeta_command\ndep_old\ndep_beta";
    let calls = backend
        .fresh_analysis_for(
            &Uri::from_file_path("/caller/main.tcl").unwrap(),
            Arc::from(source),
            "tcl8.6".to_owned(),
        )
        .await
        .unwrap();
    assert_eq!(
        calls
            .diagnostics
            .iter()
            .filter_map(|diagnostic| diagnostic.unresolved_command())
            .count(),
        4
    );
    let exact = Analyser::new()
        .structure_only()
        .analyse("package require -exact w 1.2", "tcl8.6");
    let roots = exact
        .package_requires
        .iter()
        .map(|required| {
            PackageRequirementAdvice::from_source_requirement(required, PackagePrefer::Stable)
        })
        .collect::<Vec<_>>();
    assert_eq!(roots.len(), 1);
    let registry = backend.registry_for_dialect(dialect.name).await;
    let resolver = backend.package_resolver.read().await;
    let remaining = refine_original_w123_diagnostics(
        calls.diagnostics.clone(),
        &roots,
        &resolver,
        &sources,
        dialect,
    );
    assert_eq!(
        remaining
            .iter()
            .filter_map(|diagnostic| diagnostic.unresolved_command())
            .map(|subject| subject.name_input().bytes().to_vec())
            .collect::<HashSet<_>>(),
        HashSet::from([b"beta_command".to_vec(), b"dep_old".to_vec()])
    );
    let current = W120Availability::resolve_original(
        &roots,
        Some(tcl_dialect::TclVersion::V8_6),
        &resolver,
        &sources,
        &registry,
    );
    assert!(!current.unknowable);
    assert!(
        current
            .original_available
            .iter()
            .any(|key| key.matches_ascii("dep"))
    );
    let absent = Analyser::new()
        .structure_only()
        .analyse("package require absent", "tcl8.6");
    let unavailable_roots = absent
        .package_requires
        .iter()
        .map(|required| {
            PackageRequirementAdvice::from_source_requirement(required, PackagePrefer::Stable)
        })
        .collect::<Vec<_>>();
    assert_eq!(unavailable_roots.len(), 1);
    let unavailable = W120Availability::resolve_original(
        &unavailable_roots,
        Some(tcl_dialect::TclVersion::V8_6),
        &resolver,
        &sources,
        &registry,
    );
    assert!(unavailable.unknowable);
    assert!(
        refine_original_w123_diagnostics(
            calls.diagnostics.clone(),
            &unavailable_roots,
            &resolver,
            &sources,
            dialect
        )
        .iter()
        .all(|diagnostic| diagnostic.code != DiagCode::W123)
    );
    assert!(matches!(
        supplied.inventory(
            Path::new("/library/absent.tcl"),
            Some(tcl_dialect::TclVersion::V8_6),
            &resolver
        ),
        Err(ProviderSourceUnavailable::MissingSource)
    ));
}

async fn proposal_provider_fixture(indexed: bool) -> (Backend, Uri, PathBuf, PathBuf) {
    let library = std::env::temp_dir().join("tcl-lsp-proposal-provider-source/library");
    let caller_path = library.parent().unwrap().join("caller.tcl");
    let provider_path = library.join("owned.tcl");
    let caller = Uri::from_file_path(&caller_path).unwrap();
    let provider = Uri::from_file_path(&provider_path).unwrap();
    let store = Arc::new(vfs::MemoryStore::new());
    store.upsert_uri(
        caller.as_str(),
        &caller_path,
        b"package require own\nproc unfinished {".to_vec(),
    );
    store.upsert_uri(
        provider.as_str(),
        &provider_path,
        b"proc provided {} {}".to_vec(),
    );
    let mut backend = crate::tests::test_backend();
    backend.store = store;
    backend
        .db_set_source(
            &caller,
            "package require own\nproc unfinished {",
            "tcl8.6".to_owned(),
        )
        .await
        .unwrap();
    if indexed {
        backend
            .db_set_source(&provider, "proc provided {} {}", "tcl8.6".to_owned())
            .await
            .unwrap();
    }
    backend
        .package_resolver
        .write()
        .await
        .add_original_pkg_index(
            "package ifneeded own 1.0 {source owned.tcl}",
            &library,
            &library.join("pkgIndex.tcl"),
            &|_| true,
            &|_| Vec::new(),
        );
    (backend, caller, caller_path, provider_path)
}

async fn proposal_provider_mode(backend: &Backend, uri: &Uri) -> ProposalProviderSources {
    let file = backend.db_files.lock().await[uri];
    let admission = backend
        .source_path_admissions
        .lock()
        .unwrap()
        .get(uri)
        .unwrap()
        .admission
        .clone();
    ProposalProviderSources::new(uri.clone(), file, admission).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn proposal_selected_provider_refuses_old_schema_before_inventory_and_keeps_failure() {
    let (backend, _, _, path) = proposal_provider_fixture(true).await;
    let uri = Uri::from_file_path(&path).unwrap();
    let proposal = proposal_provider_mode(&backend, &uri).await;
    let dialects = ProviderDialectInputs::capture(&backend).await;
    let inputs = ProviderCaptureInputs {
        db: &backend.db,
        files: &backend.db_files,
        global: &backend.db_config,
        folders: &backend.folder_db_configs,
        store: &backend.store,
        source_path_admissions: &backend.source_path_admissions,
        dialects: &dialects,
        proposal: Some(&proposal),
    };
    let sources = inputs.capture(&backend.package_resolver, true).await;
    let resolver = backend.package_resolver.read().await;
    assert!(matches!(
        sources.inventory(&path, Some(tcl_dialect::TclVersion::V8_6), &resolver),
        Err(ProviderSourceUnavailable::ProposalSelectedSource)
    ));
    let report = ConfiguredProviderSourceReport::from_capture(sources);
    assert_eq!(
        report.source_failure(),
        Some(ProviderSourceUnavailable::ProposalSelectedSource)
    );
    assert!(!report.is_current(&resolver));
    assert!(report.proposal_admissions().is_none());
    let db = backend.db.lock().await;
    assert_eq!(
        proposal.selected.file.text(&*db).as_str(),
        "proc provided {} {}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn actual_recovery_and_cache_hit_provider_receipts_survive_empty_deep_capture() {
    let (backend, uri, _, provider_path) = proposal_provider_fixture(true).await;
    let proposal = proposal_provider_mode(&backend, &uri).await;
    let dialects = ProviderDialectInputs::capture(&backend).await;
    let providers = ProviderCaptureInputs {
        db: &backend.db,
        files: &backend.db_files,
        global: &backend.db_config,
        folders: &backend.folder_db_configs,
        store: &backend.store,
        source_path_admissions: &backend.source_path_admissions,
        dialects: &dialects,
        proposal: Some(&proposal),
    };
    let config = backend.resolved_db_config(&uri).await;
    let file = backend.db_files.lock().await[&uri];
    let (disabled, extra, mode) = {
        let db = backend.db.lock().await;
        (
            config.disabled_diagnostics(&*db).iter().cloned().collect(),
            config.extra_commands(&*db).iter().cloned().collect(),
            config.non_ascii_mode(&*db),
        )
    };
    let registry = backend.registry_for_dialect("tcl8.6").await;
    let recovery = RecoveryWidenCtx {
        cache: &backend.recovery_names,
        registry: &registry,
        workspace_index: &backend.workspace_index,
        package_resolver: &backend.package_resolver,
        provider_capture: &providers,
        prefer: tcl_lsp_core::package_resolver::PackagePrefer::Stable,
    };
    let ctx = SalsaAnalysisCtx {
        db: &backend.db,
        uri: &uri,
        file: Some(file),
        config,
        text: "package require own\nproc unfinished {",
        dialect: tcl_lsp_core::profile_for_dialect("tcl8.6"),
    };
    let mut recovered = None;
    for _ in 0..2 {
        let ControlFlow::Continue(analysis) =
            compute_base_analysis(&backend.client, &ctx, &disabled, &extra, mode, &recovery).await
        else {
            panic!("the actual incomplete-source recovery pass must settle");
        };
        recovered = Some(analysis);
    }
    let recovered = recovered.unwrap();
    let first_names = widen_recovery_extra_commands(&recovery, &extra, ctx.text, &recovered).await;
    let cached_names = widen_recovery_extra_commands(&recovery, &extra, ctx.text, &recovered).await;
    assert!(first_names.names.contains("provided"));
    assert!(
        Arc::ptr_eq(&first_names.names, &cached_names.names),
        "actual recognition cache hit must retain its newly captured provider inputs too"
    );
    let deep = providers.capture(&backend.package_resolver, false).await;
    let report = ConfiguredProviderSourceReport::from_capture(Arc::clone(&deep));
    assert_eq!(report.source_failure(), None);
    assert!(
        report
            .proposal_admissions()
            .unwrap()
            .iter()
            .any(|(_, _, admission)| admission.path() == Some(provider_path.as_path()))
    );
    let resolver = backend.package_resolver.read().await;
    {
        let db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        let published = backend.source_path_admissions.lock().unwrap();
        assert!(report.proposal_is_current(&resolver, &*db, &files, &published));
    }
    drop(resolver);
    let provider_uri = Uri::from_file_path(&provider_path).unwrap();
    backend
        .db_set_source(&provider_uri, "proc changed {} {}", "tcl8.6".to_owned())
        .await
        .unwrap();
    let resolver = backend.package_resolver.read().await;
    let db = backend.db.lock().await;
    let files = backend.db_files.lock().await;
    let published = backend.source_path_admissions.lock().unwrap();
    assert!(
        deep.proposal_is_current(&resolver, &*db, &files, &published),
        "empty later capture cannot replace an earlier dependency"
    );
    assert!(
        !report.proposal_is_current(&resolver, &*db, &files, &published),
        "actual earlier recovery inputs remain dependencies, including the cache-hit pass"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn proposal_unversioned_disk_provider_remains_unavailable_even_with_readable_bytes() {
    let (backend, uri, _, path) = proposal_provider_fixture(false).await;
    let proposal = proposal_provider_mode(&backend, &uri).await;
    let dialects = ProviderDialectInputs::capture(&backend).await;
    let sources = ProviderCaptureInputs {
        db: &backend.db,
        files: &backend.db_files,
        global: &backend.db_config,
        folders: &backend.folder_db_configs,
        store: &backend.store,
        source_path_admissions: &backend.source_path_admissions,
        dialects: &dialects,
        proposal: Some(&proposal),
    }
    .capture(&backend.package_resolver, true)
    .await;
    assert_eq!(
        backend.store.read_source(&path).unwrap().0,
        "proc provided {} {}"
    );
    let resolver = backend.package_resolver.read().await;
    assert!(matches!(
        sources.inventory(&path, Some(tcl_dialect::TclVersion::V8_6), &resolver),
        Err(ProviderSourceUnavailable::ProposalDiskSource)
    ));
    let report = ConfiguredProviderSourceReport::from_capture(sources);
    assert_eq!(
        report.source_failure(),
        Some(ProviderSourceUnavailable::ProposalDiskSource)
    );
    assert!(!report.is_current(&resolver));
    assert!(report.proposal_admissions().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lost_proposal_capture_owner_cannot_become_an_empty_source_report() {
    let (backend, uri, _, _) = proposal_provider_fixture(true).await;
    let dialects = ProviderDialectInputs::capture(&backend).await;
    let sources = {
        let proposal = proposal_provider_mode(&backend, &uri).await;
        ProviderCaptureInputs {
            db: &backend.db,
            files: &backend.db_files,
            global: &backend.db_config,
            folders: &backend.folder_db_configs,
            store: &backend.store,
            source_path_admissions: &backend.source_path_admissions,
            dialects: &dialects,
            proposal: Some(&proposal),
        }
        .capture(&backend.package_resolver, false)
        .await
    };
    let report = ConfiguredProviderSourceReport::from_capture(sources);
    let resolver = backend.package_resolver.read().await;
    assert_eq!(
        report.source_failure(),
        Some(ProviderSourceUnavailable::SourceCurrency)
    );
    assert!(!report.is_current(&resolver));
    assert!(report.proposal_admissions().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn poisoned_proposal_inventory_cannot_issue_empty_or_populated_source_completion() {
    for populated in [false, true] {
        let (backend, uri, _, provider_path) = proposal_provider_fixture(true).await;
        let proposal = proposal_provider_mode(&backend, &uri).await;
        let dialects = ProviderDialectInputs::capture(&backend).await;
        let sources = ProviderCaptureInputs {
            db: &backend.db,
            files: &backend.db_files,
            global: &backend.db_config,
            folders: &backend.folder_db_configs,
            store: &backend.store,
            source_path_admissions: &backend.source_path_admissions,
            dialects: &dialects,
            proposal: Some(&proposal),
        }
        .capture(&backend.package_resolver, populated)
        .await;
        let resolver = backend.package_resolver.read().await;
        if populated {
            assert!(
                sources
                    .inventory(
                        &provider_path,
                        Some(tcl_dialect::TclVersion::V8_6),
                        &resolver
                    )
                    .is_ok()
            );
        }
        let report = ConfiguredProviderSourceReport::from_capture(Arc::clone(&sources));
        assert_eq!(report.source_failure(), None);
        assert!(report.is_current(&resolver));
        assert_eq!(
            report.proposal_admissions().unwrap().len(),
            usize::from(populated)
        );
        let db = backend.db.lock().await;
        let files = backend.db_files.lock().await;
        let published = backend.source_path_admissions.lock().unwrap();
        assert!(report.proposal_is_current(&resolver, &*db, &files, &published));

        // Poison the real captured inventory mutex, without replacing its map
        // or synthesizing a report completion. Both vacuous and populated maps
        // must become unavailable through every proposal reporting accessor.
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _inventory = sources.inventories.lock().unwrap();
            panic!("actual provider inventory interrupted while locked");
        }));
        assert!(poisoned.is_err());
        assert!(sources.inventories.is_poisoned());
        assert_eq!(
            report.source_failure(),
            Some(ProviderSourceUnavailable::SourceCurrency)
        );
        assert!(!report.is_current(&resolver));
        assert!(!report.proposal_is_current(&resolver, &*db, &files, &published));
        assert!(report.proposal_admissions().is_none());
    }
}
