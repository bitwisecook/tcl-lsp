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

use super::*;

mod analysis;
mod code_actions;
mod commands;
mod configuration;
mod diagnostics;
mod dialects;
mod documents;
mod language_server;
mod method_families;
mod navigation;
mod protocol;
mod rename;
mod semantic_tokens;
mod source_publication;
mod spec_packs;
mod tracked_locks;
mod workspace;

use tower_lsp_server::ls_types::{
    Diagnostic, NumberOrString, PartialResultParams, Range, ReferenceContext,
    TextDocumentIdentifier, WorkDoneProgressParams,
};

/// A throwaway directory under the system temp dir, removed on drop.
struct TmpWs(PathBuf);
impl TmpWs {
    fn new(tag: &str) -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static C: AtomicUsize = AtomicUsize::new(0);
        loop {
            let n = C.fetch_add(1, Ordering::Relaxed);
            let p =
                std::env::temp_dir().join(format!("tcl-lsp-w120-{tag}-{}-{n}", std::process::id()));
            match std::fs::create_dir(&p) {
                Ok(()) => return Self(p),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => panic!("creating owned test directory {}: {error}", p.display()),
            }
        }
    }
    fn write(&self, rel: &str, content: &str) {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }
}
impl Drop for TmpWs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn pos(line: u32, character: u32) -> Position {
    Position { line, character }
}

// F5 dialect diagnostics

/// Collect the string codes from a lifted diagnostic set.
fn diag_codes(diags: &[tower_lsp_server::ls_types::Diagnostic]) -> Vec<String> {
    diags
        .iter()
        .filter_map(|d| match &d.code {
            Some(tower_lsp_server::ls_types::NumberOrString::String(c)) => Some(c.clone()),
            _ => None,
        })
        .collect()
}

fn callback_source_wire_diagnostics(diags: &[Diagnostic]) -> Vec<Diagnostic> {
    diags
        .iter()
        .filter(|diagnostic| {
            let Some(NumberOrString::String(code)) = &diagnostic.code else {
                return false;
            };
            let Some(data) = &diagnostic.data else {
                return false;
            };
            tcl_lsp_core::diagnostic_subject::DiagnosticSubjectData::from_value(data, code)
                .is_some()
                && data
                    .pointer("/subject/kind")
                    .and_then(serde_json::Value::as_str)
                    == Some("callbackSourceArity")
        })
        .cloned()
        .collect()
}

async fn register_callback_source(
    backend: &Backend,
    uri: &Uri,
    source: &str,
) -> DocumentSourceResult<()> {
    register(backend, uri, source).await;
    backend
        .db_set_source(uri, source, "tcl8.6".to_owned())
        .await?;

    Ok(())
}

async fn callback_pull_report(
    backend: &Backend,
    uri: &Uri,
    source: &str,
) -> DocumentSourceResult<Vec<Diagnostic>> {
    Ok(callback_source_wire_diagnostics(
        &backend
            .full_diagnostics_for(uri, Arc::from(source), "tcl8.6".to_owned(), "tcl")
            .await?,
    ))
}

async fn set_callback_project_advice(backend: &Backend, enabled: bool) {
    backend.feature_toggles.lock().await.apply(
        serde_json::json!({"crossFileResolution": enabled})
            .as_object()
            .unwrap(),
    );
    backend.invalidate_diag_inputs();
}

/// Build a fresh `Backend` for unit tests that only
/// exercise the dialect-resolution helpers.  `Client`
/// isn't publicly constructible, so we build via
/// `LspService::new` and copy the wrapped `Client` into a
/// fresh `Backend` with reset state.
pub(super) fn test_backend() -> Backend {
    test_backend_over(tcl_lsp_db::TclDatabase::default())
}

/// [`test_backend`] over a caller-supplied query database, so a test can
/// supply one with an event logger attached and observe which salsa queries
/// a handler actually executes.  The `AnalyserConfig` input must be created
/// against the same database, which is why it cannot simply be swapped in
/// after the fact.
fn test_backend_over(db: tcl_lsp_db::TclDatabase) -> Backend {
    let (service, _socket) = tower_lsp_server::LspService::new(Backend::new);
    let client = service.inner().client.clone();
    let diagnostic_publisher = Arc::new(DiagnosticPublisher::new(client.clone()));
    let db_config = default_analyser_config(&db);
    Backend::from_state(test_backend_state_over(
        db,
        client,
        diagnostic_publisher,
        db_config,
    ))
}

fn test_backend_state_over(
    db: tcl_lsp_db::TclDatabase,
    client: Client,
    diagnostic_publisher: Arc<DiagnosticPublisher>,
    db_config: tcl_lsp_db::AnalyserConfig,
) -> BackendState {
    BackendState {
        client,
        diagnostic_publisher,
        document_transforms: std::sync::Mutex::new(document_transform::TransformLedger::default()),
        documents: Arc::new(DocumentStore::default()),
        diag_slots: Arc::new(Mutex::new(HashMap::new())),
        diagnostics_admission: crate::diagnostics::AnalysisAdmission::shared(),
        default_dialect: Arc::new(Mutex::new("tcl8.6".to_owned())),
        default_dialect_explicit: Mutex::new(false),
        warned_session_dialect: Mutex::new(None),
        session_dialect_override: Arc::new(Mutex::new(None)),
        document_dialect_overrides: Arc::new(Mutex::new(HashMap::new())),
        config_reload: Mutex::new(ConfigReloadSlot::default()),
        workspace_folders: Mutex::new(Vec::new()),
        folder_dialects: Arc::new(Mutex::new(Vec::new())),
        folder_configs: Arc::new(Mutex::new(Vec::new())),
        folder_db_configs: Arc::new(Mutex::new(Vec::new())),
        folder_db_config_tombstones: Arc::new(Mutex::new(HashMap::new())),
        non_ascii_mode: Mutex::new(NonAsciiMode::Default),
        disabled_diagnostics: Mutex::new(PolicyLayers::default().production_skip()),
        diagnostics_exclude: Mutex::new(Vec::new()),
        policy_layers: Mutex::new(PolicyLayers::default()),
        workspace_index: Arc::new(TrackedRwLock::new("workspace_index", new_workspace_index())),
        original_declaration_registry: Mutex::new(
            tcl_lsp_core::original_declaration::OriginalDeclarationRegistry::default(),
        ),
        package_resolver: Arc::new(RwLock::new(PackageResolver::new())),
        recovery_names: Arc::new(Mutex::new(RecoveryNameCache::default())),
        workspace_scan_gate: tokio::sync::Mutex::new(()),
        workspace_scan_ready: Arc::new(WorkspaceScanReadiness::default()),
        autoloaded_library_uris: Arc::new(Mutex::new(HashSet::new())),
        rehomed_source_seeds: Arc::new(Mutex::new(HashMap::new())),
        rehoming_gate: Arc::new(tokio::sync::Mutex::new(())),
        original_source_view_build_gate: Arc::new(tokio::sync::Mutex::new(())),
        original_source_views: Arc::new(std::sync::Mutex::new(
            readonly_reference_reports::source_views::OriginalSourceViewTable::default(),
        )),
        // The test harness never surveys the machine's own Tcl
        // installation: what is installed varies by host and its index
        // survey dominated server test time (user decision
        // 2026-10-19T11:00Z `product_fixes`: "test backend not scanning
        // the host install"). Tests that exercise discovered installations
        // supply a pinned fixture install explicitly.
        discovered_tcl: Arc::new(std::sync::OnceLock::from(Vec::new())),
        editor_library_paths: Mutex::new(Vec::new()),
        workspace_scan_max_files: Mutex::new(WORKSPACE_SCAN_FILE_CAP),
        package_prefer_latest_default: Mutex::new(false),
        package_provides: Mutex::new(Vec::new()),
        script_encoding: std::sync::RwLock::new(tcl_registry::value_transfer::ScriptEncoding::Auto),
        extra_commands: Mutex::new(Vec::new()),
        signature_help_disabled_commands: Mutex::new(Vec::new()),
        spec_pack_paths: Mutex::new(Vec::new()),
        workspace_trust: Mutex::new(tcl_dialect::model::WorkspaceTrust::default()),
        spec_packs: Arc::new(Mutex::new(PublishedPackSet::default())),
        spec_pack_reload: Arc::new(Mutex::new(None)),
        studio_sources: Mutex::new(studio_source::StudioSources::default()),
        spec_pack_reload_seq: std::sync::atomic::AtomicU64::new(0),
        external_pack_watch_globs: Arc::new(Mutex::new(Vec::new())),
        pack_source_extensions: Arc::new(Mutex::new(Vec::new())),
        bigip_version: Mutex::new(None),
        declared_targets: Mutex::new(Vec::new()),
        generic_variable_patterns: Mutex::new(None),
        formatting_settings: Mutex::new(serde_json::Value::Null),
        feature_toggles: Mutex::new(FeatureToggles::default()),
        optimiser_enabled: Mutex::new(true),
        diag_inputs_epoch: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        fresh_analysis_cache: std::sync::Mutex::default(),
        analyser_inputs_gate: Arc::new(tokio::sync::RwLock::new(())),
        edit_barrier_stall_reported: std::sync::atomic::AtomicU64::new(u64::MAX),
        optimiser_profile: Mutex::new(default_optimiser_profile()),
        line_length: Mutex::new(80),
        style_line_length: Mutex::new(120),
        db: Arc::new(TrackedMutex::new("db", db)),
        db_files: Arc::new(TrackedMutex::new("db_files", HashMap::new())),
        source_path_admissions: Arc::new(std::sync::Mutex::new(HashMap::new())),
        db_tombstones: Arc::new(Mutex::new(HashMap::new())),
        db_project_members: Arc::new(Mutex::new(HashSet::new())),
        db_project: Arc::new(Mutex::new(None)),
        db_config: Arc::new(Mutex::new(db_config)),
        pull_diag_cache: Arc::new(Mutex::new(HashMap::new())),
        closed_diag_gen: Arc::new(Mutex::new(HashMap::new())),
        closed_diag_order: Arc::new(Mutex::new(VecDeque::new())),
        client_supports_pull_diagnostics: std::sync::atomic::AtomicBool::new(false),
        client_supports_relative_watch_patterns: std::sync::atomic::AtomicBool::new(false),
        conditional_variable_rename_supported: std::sync::atomic::AtomicBool::new(false),
        environment_notice: Arc::new(environment_notice::EnvironmentNotice::in_memory()),
        last_semantic_tokens: Arc::new(Mutex::new(HashMap::new())),
        semantic_tokens_refresh_asked: Arc::new(Mutex::new(HashMap::new())),
        workspace_class_analyses: Arc::new(Mutex::new(HashMap::new())),
        class_factory_generation: Arc::new(tokio::sync::RwLock::new(0)),
        semantic_tokens_refresh_pending: Arc::new(std::sync::atomic::AtomicU8::new(0)),
        semantic_tokens_convergence: Arc::new(std::sync::Mutex::new(HashMap::new())),
        warm_task: std::sync::Mutex::new(None),
        edit_order: EditOrder::default(),
        watched_file_change_gate: Arc::new(Mutex::new(())),
        disk_publication_gate: Mutex::new(()),
        live_publication_advanced: Arc::new(tokio::sync::Notify::new()),
        live_publication_invalidated: Arc::new(tokio::sync::Notify::new()),
        live_publication_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        live_publication_uri_generations: std::sync::Mutex::new(HashMap::new()),
        live_publication_gate: Arc::new(Mutex::new(())),
        store: Arc::new(vfs::NativeStore),
    }
}

fn doc_diag_params(uri: &Uri, previous: Option<&str>) -> DocumentDiagnosticParams {
    DocumentDiagnosticParams {
        text_document: tower_lsp_server::ls_types::TextDocumentIdentifier { uri: uri.clone() },
        identifier: None,
        previous_result_id: previous.map(str::to_owned),
        work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(),
        partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
    }
}

/// Poll `cond` until it holds or ~2s elapse, reporting whether it held.
/// For assertions on state a detached background task settles (a debounced
/// diagnostics worker retiring, say) rather than the awaited call itself.
async fn wait_for<F, Fut>(mut cond: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    for _ in 0..200 {
        if cond().await {
            return true;
        }
        crate::rt::sleep(std::time::Duration::from_millis(10)).await;
    }
    false
}

/// A scratch directory removed when dropped, including when the test panics.
pub(crate) struct ScratchDir(PathBuf);

impl ScratchDir {
    /// Own `path`, a directory or a file: it is removed when this drops.
    pub(crate) fn at(path: PathBuf) -> Self {
        Self(path)
    }
}

impl std::ops::Deref for ScratchDir {
    type Target = PathBuf;
    fn deref(&self) -> &PathBuf {
        &self.0
    }
}

impl AsRef<std::path::Path> for ScratchDir {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        if std::fs::remove_dir_all(&self.0).is_err() {
            let _ = std::fs::remove_file(&self.0);
        }
    }
}

/// Build a unique scratch directory under the system temp dir for a
/// disk-walk test; it is removed when the returned guard drops.
fn unique_scratch_dir(tag: &str) -> ScratchDir {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    loop {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("tcl-lsp-scan-{tag}-{}-{n}", std::process::id()));
        match std::fs::create_dir(&dir) {
            Ok(()) => return ScratchDir(dir),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => panic!(
                "creating owned scratch directory {}: {error}",
                dir.display()
            ),
        }
    }
}

/// Wait for the single document-sync handler under test to draw its
/// `EditOrder` ticket **and** hand it on, or give up.
///
/// `edits_settled` alone cannot express this: called before the handler has
/// drawn its ticket it returns immediately and proves nothing, so the wait
/// is on the counters directly.
async fn one_edit_turn_released(backend: &Backend) -> bool {
    use std::sync::atomic::Ordering;
    crate::rt::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let order = &backend.edit_order;
            if order.next_ticket.load(Ordering::Acquire) == 1
                && order.now_serving.load(Ordering::Acquire) == 1
            {
                return;
            }
            crate::rt::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .is_ok()
}

async fn register(backend: &Backend, uri: &Uri, src: &str) {
    register_in_dialect(backend, uri, src, "tcl8.6").await;
}

/// [`register`] under a named dialect, for a fixture whose caller and
/// provider must share one recipe (the lenient `tcl` environment).
async fn register_in_dialect(backend: &Backend, uri: &Uri, src: &str, dialect: &str) {
    backend.documents.lock("test").await.insert(
        uri.clone(),
        DocumentState::new(src.to_owned(), dialect.to_owned()),
    );
    backend
        .db_set_source(uri, src, dialect.to_owned())
        .await
        .expect("the fixture retains its actual checked SourceFile owner");
    let analysis = backend
        .analysis_for(uri, Arc::from(src), dialect.to_owned())
        .await
        .expect("the fixture analyses its actual current source and full input");
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &analysis);
}

/// Build an on-disk autoload library (`tclIndex` + defining file, like a
/// `TCLLIBPATH` entry) whose `Rbc_Wire` also calls `Rbc_ActiveLegend`
/// internally, and point the backend's package database at it.  Returns
/// the library file's URI.
async fn seed_autoload_library(backend: &Backend, tag: &str) -> Uri {
    let libdir = unique_scratch_dir(tag);
    std::fs::write(
        libdir.join("graph.tcl"),
        "proc Rbc_ActiveLegend {graph} {}\nproc Rbc_Wire {} { Rbc_ActiveLegend .g }\n",
    )
    .unwrap();
    std::fs::write(
        libdir.join("tclIndex"),
        "# Tcl autoload index file, version 2.0\n\
             set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n\
             set auto_index(Rbc_Wire) [list source [file join $dir graph.tcl]]\n",
    )
    .unwrap();
    *backend.package_resolver.write().await = build_package_resolver(
        &vfs::NativeStore,
        &[],
        &[libdir
            .to_str()
            .expect("library path configuration requires UTF-8")
            .to_owned()],
        &[],
        WORKSPACE_SCAN_DIR_CAP,
    );
    crate::canonical_file_uri(libdir.join("graph.tcl")).expect("library uri")
}

/// Put an entry for `uri` in every cache [`Backend::forget_uri_states`]
/// clears, so a retirement test can assert on the whole group rather than
/// on whichever member a given retirement path happens to miss.
async fn seed_per_uri_caches(backend: &Backend, uri: &Uri) {
    backend
        .autoloaded_library_uris
        .lock()
        .await
        .insert(uri.as_str().to_owned());
    backend
        .rehomed_source_seeds
        .lock()
        .await
        .insert(uri.as_str().to_owned(), vec!["::seeded".to_owned()]);
    backend
        .last_semantic_tokens
        .lock()
        .await
        .insert(uri.clone(), ("st-1".to_owned(), Vec::new()));
    backend
        .semantic_tokens_refresh_asked
        .lock()
        .await
        .insert(uri.clone(), 7);
    backend.workspace_class_analyses.lock().await.insert(
        uri.clone(),
        WorkspaceClassAnalysis {
            fingerprint: (0, 0, 0, 0),
            source_analysis: None,
            resource_inputs: None,
            analysis: Arc::default(),
        },
    );
}

/// Whether each cache [`seed_per_uri_caches`] fills still holds `uri`, in
/// the same order, so an assertion reads `[false; 5]` / `[true; 5]`.
async fn per_uri_cache_hits(backend: &Backend, uri: &Uri) -> [bool; 5] {
    [
        backend
            .autoloaded_library_uris
            .lock()
            .await
            .contains(uri.as_str()),
        backend
            .rehomed_source_seeds
            .lock()
            .await
            .contains_key(uri.as_str()),
        backend.last_semantic_tokens.lock().await.contains_key(uri),
        backend
            .semantic_tokens_refresh_asked
            .lock()
            .await
            .contains_key(uri),
        backend
            .workspace_class_analyses
            .lock()
            .await
            .contains_key(uri),
    ]
}

async fn register_logical_hierarchy(backend: &Backend, uri: &Uri, source: &str) {
    backend
        .documents
        .lock("test logical hierarchy")
        .await
        .insert(
            uri.clone(),
            DocumentState::new(source.to_owned(), "tcl".to_owned()),
        );
    let analysis = Analyser::new().analyse(source, "tcl");
    backend
        .workspace_index
        .write()
        .await
        .add_document(uri.as_str(), &analysis);
}
