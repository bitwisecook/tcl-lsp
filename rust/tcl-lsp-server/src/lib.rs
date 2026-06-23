//! Native LSP server backend for Tcl.
//!
//! Exposes a [`Backend`] that implements [`tower_lsp::LanguageServer`]
//! and is wrapped in an `LspService` by the binary. This crate is the
//! second consumer of [`tcl_lsp_core`] (the first is `tcl-lsp-rust`),
//! so the pure-Rust crate boundary now has both production drivers
//! exercising it.
//!
//! ARCH8 ships the bootstrap with the folding-range provider wired
//! end-to-end; the `S-document-symbols` follow-up adds the
//! document-symbol provider on top.  Every other LSP method returns
//! [`tower_lsp::jsonrpc::ErrorCode::MethodNotFound`] until later
//! chunks extend the provider set.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult, NonAsciiMode};
use tcl_lsp_core::bigip as core_bigip;
use tcl_lsp_core::call_hierarchy as core_call_hierarchy;
use tcl_lsp_core::code_actions as core_code_actions;
use tcl_lsp_core::code_lens as core_code_lens;
use tcl_lsp_core::completion::{
    self as core_completion, CompletionItem as CoreCompletionItem,
    CompletionKind as CoreCompletionKind,
};
use tcl_lsp_core::declaration as core_declaration;
use tcl_lsp_core::definition::{self as core_definition, LspRange as CoreLspRange};
use tcl_lsp_core::document_links as core_document_links;
use tcl_lsp_core::document_symbols::{self as core_symbols, SymbolKind as CoreSymbolKind};
use tcl_lsp_core::file_ops as core_file_ops;
use tcl_lsp_core::folding::FoldKind;
use tcl_lsp_core::formatting as core_formatting;
use tcl_lsp_core::hover::{self as core_hover, Hover as CoreHover, HoverKind as CoreHoverKind};
use tcl_lsp_core::implementation as core_implementation;
use tcl_lsp_core::inlay_hints as core_inlay_hints;
use tcl_lsp_core::linked_editing_range as core_linked_editing_range;
use tcl_lsp_core::minify as core_minify;
use tcl_lsp_core::references as core_references;
use tcl_lsp_core::rename as core_rename;
use tcl_lsp_core::selection_range as core_selection_range;
use tcl_lsp_core::semantic_tokens as core_semantic_tokens;
use tcl_lsp_core::signature_help::{
    self as core_sig, ParameterInformation as CoreParameterInformation,
    SignatureHelp as CoreSignatureHelp, SignatureInformation as CoreSignatureInformation,
};
use tcl_lsp_core::type_definition as core_type_definition;
use tcl_lsp_core::workspace_index as core_workspace_index;
use tcl_lsp_db::TclDb as _;
// type_hierarchy core provider lands when tower-lsp's
// LanguageServer trait exposes the type-hierarchy methods.
// Module is registered in `tcl_lsp_core` for downstream
// callers; intentionally not imported here yet.
// use tcl_lsp_core::type_hierarchy as core_type_hierarchy;
use tcl_lsp_core::workspace_symbols::{
    self as core_workspace_symbols, WorkspaceSymbolKind as CoreWorkspaceSymbolKind,
};
use tcl_registry::CommandRegistry;
use tcl_registry::dialects::DialectSet;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CallHierarchyServerCapability, CodeAction, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, CodeLens, CodeLensOptions, CodeLensParams, CompletionItem,
    CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse, ConfigurationItem,
    DeclarationCapability, DiagnosticOptions, DiagnosticServerCapabilities,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidChangeWatchedFilesRegistrationOptions, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentChanges,
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    DocumentFormattingParams, DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams,
    DocumentLink, DocumentLinkOptions, DocumentLinkParams, DocumentRangeFormattingParams,
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, Documentation,
    ExecuteCommandOptions, ExecuteCommandParams, FileChangeType, FileOperationFilter,
    FileOperationPattern, FileOperationRegistrationOptions, FileSystemWatcher, FoldingRange,
    FoldingRangeKind, FoldingRangeParams, FoldingRangeProviderCapability,
    FullDocumentDiagnosticReport, GlobPattern, GotoDefinitionParams, GotoDefinitionResponse, Hover,
    HoverContents, HoverParams, HoverProviderCapability, ImplementationProviderCapability,
    InitializeParams, InitializeResult, InitializedParams, InlayHint, InlayHintKind,
    InlayHintLabel, InlayHintParams, LinkedEditingRangeParams, LinkedEditingRanges, Location,
    MarkupContent, MarkupKind, MessageType, OneOf, OptionalVersionedTextDocumentIdentifier,
    ParameterInformation, ParameterLabel, Position, PositionEncodingKind, PrepareRenameResponse,
    Range, ReferenceParams, Registration, RelatedFullDocumentDiagnosticReport,
    RelatedUnchangedDocumentDiagnosticReport, RenameFilesParams, RenameOptions, RenameParams,
    SelectionRange, SelectionRangeParams, SelectionRangeProviderCapability,
    SemanticTokens as LspSemanticTokens, SemanticTokensDeltaParams, SemanticTokensFullDeltaResult,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams,
    SemanticTokensRangeParams, SemanticTokensRangeResult, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, SignatureHelp,
    SignatureHelpOptions, SignatureHelpParams, SignatureInformation, SymbolInformation, SymbolKind,
    TextDocumentEdit, TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextEdit, TypeDefinitionProviderCapability,
    UnchangedDocumentDiagnosticReport, Url, WatchKind, WillSaveTextDocumentParams,
    WorkDoneProgressOptions, WorkspaceDiagnosticParams, WorkspaceDiagnosticReport,
    WorkspaceDiagnosticReportResult, WorkspaceDocumentDiagnosticReport, WorkspaceEdit,
    WorkspaceFileOperationsServerCapabilities, WorkspaceFoldersServerCapabilities,
    WorkspaceFullDocumentDiagnosticReport, WorkspaceServerCapabilities, WorkspaceSymbolParams,
    WorkspaceUnchangedDocumentDiagnosticReport,
    request::{
        GotoDeclarationParams, GotoDeclarationResponse, GotoImplementationParams,
        GotoImplementationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse,
    },
};
use tower_lsp::{Client, LanguageServer};

/// Document store value: source text + dialect string.
#[derive(Debug, Clone)]
struct DocumentState {
    text: String,
    /// Persisted line-start index for `text`, patched in place on each edit
    /// (`LineIndex::apply_edit`) rather than rebuilt per position lookup
    /// (SRV-INCREMENTAL Task 1).  Kept in lock-step with `text`: every mutation
    /// of `text` updates this alongside it.
    line_index: tcl_lexer::LineIndex,
    dialect: String,
    /// The LSP `languageId` the client opened this document with (empty
    /// for documents materialised from disk by the folder scan).  Retained
    /// so a later dialect change — `workspace/didChangeConfiguration` from
    /// `tclLsp.selectDialect` — can re-resolve the document's dialect with
    /// the same precedence `dialect_for_open` applied at `didOpen` (an
    /// explicit language id still wins over the new session default).
    language_id: String,
    /// Monotonic local revision.  Incremented before each
    /// asynchronous analyser run so stale workers cannot publish
    /// analysis/cache state for an older edit.
    revision: u64,
    /// Last LSP document version reported by the client.  Forwarded
    /// with published diagnostics so clients can discard obsolete
    /// diagnostics if messages arrive out of order.
    version: Option<i32>,
}

impl DocumentState {
    fn new(text: String, dialect: String) -> Self {
        let line_index = tcl_lexer::LineIndex::new(&text);
        Self {
            text,
            line_index,
            dialect,
            language_id: String::new(),
            revision: 0,
            version: None,
        }
    }

    fn with_version(text: String, dialect: String, version: i32) -> Self {
        let line_index = tcl_lexer::LineIndex::new(&text);
        Self {
            text,
            line_index,
            dialect,
            language_id: String::new(),
            revision: 0,
            version: Some(version),
        }
    }

    /// Record the LSP `languageId` the document was opened with (builder
    /// form, so the existing constructors stay two-argument).
    fn with_language_id(mut self, language_id: String) -> Self {
        self.language_id = language_id;
        self
    }

    fn bump_revision(&mut self, version: i32) {
        self.revision = self.revision.saturating_add(1);
        self.version = Some(version);
    }
}

/// Debounce interval before a scheduled diagnostics run starts work.  A burst
/// of keystrokes within this window collapses to a single analysis (the
/// generation check supersedes earlier scheduled runs).  Small enough that
/// diagnostics still feel immediate after typing stops.
const DIAGNOSTICS_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(50);

/// One document edit's complete, self-consistent diagnostics input — captured
/// together so the analysed text, the published version, and the salsa input
/// handle all describe the *same* revision (no divergence between a `documents`
/// read and a separate salsa lookup).  `file` is `None` only if the salsa input
/// is somehow absent, in which case the run falls back to a direct `analyse`.
#[derive(Clone)]
struct DiagJob {
    text: String,
    dialect: String,
    /// The document's editor `language_id`, carried so the diagnostics worker
    /// can dispatch F5 dialect documents (iApp APL presentations are detected
    /// by language id / basename, not by the resolved dialect alone — see
    /// [`is_apl_source`]).
    language_id: String,
    revision: u64,
    version: Option<i32>,
    file: Option<tcl_lsp_db::SourceFile>,
    config: tcl_lsp_db::AnalyserConfig,
}

/// One document's last-published diagnostics plus the `result_id` that
/// identifies that exact set, for the pull-diagnostic path
/// (`textDocument/diagnostic` + `workspace/diagnostic`).  Kept in sync with
/// the push pipeline so a pull request returns the same diagnostics the editor
/// last received via `publish_diagnostics`, and an unchanged set can be
/// answered with a cheap `Unchanged` report.  Mirrors the Python server's
/// `_pull_diag_cache` / `_pull_diag_result_ids`.
#[derive(Clone)]
struct PullDiagEntry {
    result_id: String,
    /// The document revision the cached diagnostics were computed for.  The
    /// pull handler compares it against the live document so a cache entry from
    /// an older edit is recomputed rather than served as current.
    revision: u64,
    diagnostics: Vec<tower_lsp::lsp_types::Diagnostic>,
}

/// Monotonic `result_id` for the pull-diagnostic cache.  A fresh id each time a
/// document's diagnostics change lets a client's `previousResultId` be compared
/// for the `Unchanged` short-circuit.
fn next_pull_diag_result_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("pd-{id}")
}

/// Per-URI coalescing-scheduler slot (see [`Backend::diag_slots`]).
#[derive(Default)]
struct DiagSlot {
    /// An edit arrived that the running worker has not yet analysed.  Just a
    /// flag: the worker reads the document's *current* state at drain time, so
    /// it is robust to out-of-order edit processing (a late, lower-version edit
    /// can't clobber a captured job — there is no captured job).
    dirty: bool,
    /// A worker task is currently draining this document.
    running: bool,
}

/// Owned inputs for a detached diagnostics run — the document-independent
/// handles [`run_diagnostics_core`] (and the worker's per-drain [`DiagJob`]
/// capture) need without borrowing `Backend`, so the work can run on a
/// `tokio::spawn`ed task off the LSP event loop.  `Clone` so the coalescing
/// worker can reuse it across successive runs of one document.
#[derive(Clone)]
struct DiagInputs {
    client: Client,
    registry: Arc<CommandRegistry>,
    disabled: HashSet<String>,
    non_ascii_mode: NonAsciiMode,
    optimiser_enabled: bool,
    opt_disabled: HashSet<String>,
    documents: Arc<Mutex<HashMap<Url, DocumentState>>>,
    workspace_index: Arc<Mutex<core_workspace_index::WorkspaceIndex>>,
    /// The salsa db handle.  Each run clones a *fresh, short-lived* snapshot and
    /// drops it immediately, so an idle worker never holds a clone across the
    /// debounce sleep (which would block the next edit's `set_text` — salsa's
    /// exclusive-access write waits for all outstanding handles to drop).
    db: Arc<Mutex<tcl_lsp_db::TclDatabase>>,
    db_files: Arc<Mutex<HashMap<Url, tcl_lsp_db::SourceFile>>>,
    /// The salsa `Project` input handle (workspace file set), read by the worker
    /// for cross-file `project_diagnostics` when `xc_diagnostics` is enabled.
    db_project: Arc<Mutex<Option<tcl_lsp_db::Project>>>,
    db_config: Arc<Mutex<tcl_lsp_db::AnalyserConfig>>,
    /// Per-folder salsa `AnalyserConfig` handles (see
    /// [`Backend::folder_db_configs`]); `capture_job` resolves the right one by
    /// the document's URI so folder-scoped W-code suppression reaches the
    /// cached analysis, falling back to `db_config`.
    folder_db_configs: Arc<Mutex<Vec<(Url, tcl_lsp_db::AnalyserConfig)>>>,
    /// Pull-diagnostic cache, updated as each push run publishes so the
    /// `textDocument/diagnostic` / `workspace/diagnostic` paths return the
    /// last-published set.
    pull_diag_cache: Arc<Mutex<HashMap<Url, PullDiagEntry>>>,
    /// Whether the opt-in XC100-301 translatability diagnostics are enabled
    /// for this document (resolved per folder in [`Backend::diag_inputs`]).
    /// Only takes effect on `f5-irules` documents.
    xc_diagnostics: bool,
}

impl DiagInputs {
    /// Capture the document's current, self-consistent diagnostics input (live
    /// buffer + salsa input handle + config).  Reading at drain time — after the
    /// debounce, once a burst's edits have settled — makes the worker robust to
    /// out-of-order edit processing: it always analyses the latest committed
    /// state.  `None` when the document is not open.
    async fn capture_job(&self, uri: &Url) -> Option<DiagJob> {
        let (text, dialect, language_id, revision, version) = {
            let docs = self.documents.lock().await;
            let doc = docs.get(uri)?;
            (
                doc.text.clone(),
                doc.dialect.clone(),
                doc.language_id.clone(),
                doc.revision,
                doc.version,
            )
        };
        let file = self.db_files.lock().await.get(uri).copied();
        // Prefer a folder-scoped analyser config when the document sits under a
        // folder that overrides disabled codes / non-ASCII mode; else the
        // process-global config.
        let config = {
            let folder = self.folder_db_configs.lock().await;
            match longest_folder_match(&folder, uri) {
                Some(cfg) => *cfg,
                None => *self.db_config.lock().await,
            }
        };
        Some(DiagJob {
            text,
            dialect,
            language_id,
            revision,
            version,
            file,
            config,
        })
    }
}

/// Run the analyser + diagnostic lifts for one document and publish the result,
/// off the LSP event loop.  This is the detached body the old synchronous
/// `publish_analyser_diagnostics` became.
///
/// The base analysis runs through the cancellable salsa `file_analysis_incremental`
/// query (slice 5): the per-item walk is memoised and a concurrent edit's
/// `set_text` cancels an in-flight read at a per-item query boundary, so the
/// diagnostics path no longer needs the uncancellable direct-`analyse` detour
/// that previously decoupled it from salsa to avoid write-contention stalls.
#[allow(clippy::too_many_lines)]
async fn run_diagnostics_core(inputs: DiagInputs, uri: &Url, job: DiagJob) -> bool {
    // Returns whether this version is **settled** (published, intentionally
    // skipped as superseded, or a BIG-IP no-op).  `false` means the run was
    // cancelled mid-flight and the caller should retry the document's latest
    // state.
    let DiagInputs {
        client,
        registry,
        disabled,
        non_ascii_mode,
        optimiser_enabled,
        opt_disabled,
        documents,
        workspace_index,
        db,
        db_project,
        pull_diag_cache,
        xc_diagnostics,
        // The worker captures the job from these before calling us; unused here.
        db_files: _,
        db_config: _,
        folder_db_configs: _,
    } = inputs;
    let DiagJob {
        text,
        dialect,
        language_id,
        revision,
        version,
        file,
        config,
    } = job;

    // F5 dialect dispatch (FE-DIAG-F5): BIG-IP config and iApp APL
    // presentation documents are not Tcl source — they have model-level
    // validators (`BIGIP6001`-`6011`, `IAPP7001`-`7003`) rather than the Tcl
    // analyser.  Compute and publish their diagnostics here, before the
    // analyser, mirroring the file-type dispatch in Python
    // `server/diagnostics_pipeline.py`.
    if let Some(diags) =
        f5_dialect_diagnostics(uri, &text, &dialect, &language_id, &disabled, &documents).await
    {
        // Publish only when this version is still current (the same revision
        // guard the analyser path applies before publishing), and keep the
        // pull-diagnostic cache in lock-step so `textDocument/diagnostic`
        // returns the same set.
        let is_current = documents
            .lock()
            .await
            .get(uri)
            .is_some_and(|doc| doc.revision == revision);
        if is_current {
            pull_diag_cache.lock().await.insert(
                uri.clone(),
                PullDiagEntry {
                    result_id: next_pull_diag_result_id(),
                    revision,
                    diagnostics: diags.clone(),
                },
            );
            client
                .publish_diagnostics(uri.clone(), diags, version)
                .await;
        }
        return true;
    }

    let started = std::time::Instant::now();
    let uri_str = uri.to_string();
    let line_count = text.lines().count();

    // Base analysis: the cancellable salsa `file_analysis_incremental` query
    // (slice 5), off the LSP event loop.  This retires the old direct-`analyse`
    // detour: the per-item walk is memoised (a body edit recomputes one body +
    // the cheap shell + tail, not the whole file), and a concurrent edit's
    // `set_text` cancels this read at a per-item query boundary instead of
    // stalling behind an uncancellable analyse.  On cancellation we drop the
    // run — the superseding edit schedules a fresh one.  `file` is `None` only
    // if the salsa input is somehow absent; then fall back to a direct analyse.
    // Cross-file (SRV-INCREMENTAL Task 6): when `xcDiagnostics` is enabled, the
    // worker also computes `project_diagnostics` — `file_analysis`'s diagnostics
    // with W123 (unknown command) suppressed for commands defined as procs
    // elsewhere in the workspace.  Read the current `Project` handle now (it is
    // stable across re-sets); `None` ⇒ no cross-file pass (status-quo per-file
    // diagnostics).  Both halves come from the *same* snapshot so the cross-file
    // pass reuses the analyser read (a cache hit, not a second analysis).
    let project = if xc_diagnostics {
        *db_project.lock().await
    } else {
        None
    };
    let (analysis, analyser_diags): (Arc<AnalysisResult>, Vec<_>) = if let Some(file) = file {
        // Clone a fresh, short-lived snapshot for just this read and move it
        // into the worker; it drops when the read finishes, so it never holds
        // exclusive-access-blocking references across the debounce sleep.
        let snapshot = db.lock().await.clone();
        match tokio::task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| {
                let analysis = tcl_lsp_db::file_analysis_incremental(&snapshot, file, config);
                let diags = match project {
                    Some(project) => {
                        (*tcl_lsp_db::project_diagnostics(&snapshot, file, config, project)).clone()
                    }
                    None => analysis.diagnostics.clone(),
                };
                (analysis, diags)
            })
            .ok()
        })
        .await
        {
            Ok(Some(pair)) => pair,
            // Cancelled mid-read (a concurrent `set_text` on the shared db) or
            // the worker panicked — don't publish; signal a retry of the
            // document's latest state.
            Ok(None) | Err(_) => return false,
        }
    } else {
        let (a_text, a_dialect, a_disabled) = (text.clone(), dialect.clone(), disabled.clone());
        let analysis = tokio::task::spawn_blocking(move || {
            Arc::new(
                Backend::configured_analyser(a_disabled, non_ascii_mode)
                    .analyse(&a_text, &a_dialect)
                    .clone(),
            )
        })
        .await
        .unwrap_or_default();
        let diags = analysis.diagnostics.clone();
        (analysis, diags)
    };

    // Optimiser / compiler-checks diagnostics, also off the event loop via the
    // cancellable salsa `compiler_check_diagnostics` query: the unit's
    // per-procedure lattices are memoised by `function_lattice` and shared with
    // the analyser tail above.  On cancellation, drop the run like the base
    // analysis (the superseding edit reschedules a fresh one).  `file` is `None`
    // only if the salsa input is absent; then build directly (uncached).
    let compiler_diags: Arc<tcl_lsp_db::CompilerDiagnostics> = if let Some(file) = file {
        let snapshot = db.lock().await.clone();
        match tokio::task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| tcl_lsp_db::compiler_check_diagnostics(&snapshot, file)).ok()
        })
        .await
        {
            Ok(Some(d)) => d,
            Ok(None) | Err(_) => return false,
        }
    } else {
        let (c_text, c_dialect) = (text.clone(), dialect.clone());
        let c_registry = Arc::clone(&registry);
        tokio::task::spawn_blocking(move || {
            Arc::new(tcl_lsp_db::compiler_check_diagnostics_uncached(
                &c_text,
                &c_registry,
                &c_dialect,
            ))
        })
        .await
        .unwrap_or_else(|_| {
            Arc::new(tcl_lsp_db::CompilerDiagnostics {
                checks: Vec::new(),
                optimisations: Vec::new(),
            })
        })
    };

    let analysis_lifts = Arc::clone(&analysis);
    let lift_text = text.clone();
    let xc_for_irules = xc_diagnostics && dialect == "f5-irules";
    let result = tokio::task::spawn_blocking(move || {
        // `analyser_diags` is the cross-file-filtered set when `xcDiagnostics` is
        // on (else identical to `analysis_lifts.diagnostics`).
        let mut diagnostics = lift_analyser_diagnostics(&lift_text, &analyser_diags);
        diagnostics.extend(lift_compiler_diagnostics(
            &lift_text,
            &compiler_diags,
            optimiser_enabled,
            &opt_disabled,
            &disabled,
        ));
        diagnostics.extend(lift_source_style_diagnostics(
            &lift_text,
            &analysis_lifts.suppressed_lines,
            &disabled,
        ));
        // FE-DIAG-F5 opt-in: append the XC100-301 translatability diagnostics
        // for `f5-irules` documents when `xcDiagnostics` is enabled.
        if xc_for_irules {
            diagnostics.extend(lift_xc_diagnostics(
                &lift_text,
                &disabled,
                &analysis_lifts.suppressed_lines,
            ));
        }
        diagnostics
    })
    .await;

    match result {
        Ok(diags) => {
            {
                // Hold the `documents` lock across the currency re-check and the
                // workspace-index update so a concurrent `did_change`/`did_close`
                // (which also take `documents`) cannot interleave between them
                // and leave a stale index entry — the role the former global
                // `document_analysis_gate` served, now via the natural
                // `documents` → `workspace_index` lock order.
                let docs = documents.lock().await;
                let is_current = docs.get(uri).is_some_and(|doc| doc.revision == revision);
                if !is_current {
                    // Superseded by a newer edit, which has marked the document
                    // dirty — settled for this version; the worker will process
                    // the newer state.
                    return true;
                }
                let mut index = workspace_index.lock().await;
                index.remove_document(uri.as_str());
                index.add_document(uri.as_str(), &analysis);
            }
            let diag_count = diags.len();
            // Keep the pull-diagnostic cache in lock-step with the push: a
            // `textDocument/diagnostic` request now returns this exact set with
            // a fresh `result_id`, and an editor that already holds it gets a
            // cheap `Unchanged` report.
            pull_diag_cache.lock().await.insert(
                uri.clone(),
                PullDiagEntry {
                    result_id: next_pull_diag_result_id(),
                    revision,
                    diagnostics: diags.clone(),
                },
            );
            client
                .publish_diagnostics(uri.clone(), diags, version)
                .await;
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            client
                .log_message(
                    MessageType::LOG,
                    format!(
                        "[timing] workspace_state.update {elapsed_ms:.0}ms \
                         (uri={uri_str}, lines={line_count}, diags={diag_count})"
                    ),
                )
                .await;
            true
        }
        Err(err) => {
            client
                .log_message(
                    MessageType::WARNING,
                    format!("diagnostics worker panicked: {err}"),
                )
                .await;
            true
        }
    }
}

/// LSP server backend.
///
/// Holds the LSP `Client` for outbound notifications, a document
/// store keyed on URL, and a per-dialect [`CommandRegistry`]
/// cache. Constructed once per LSP session by
/// [`tower_lsp::LspService::new`].
///
/// Each dialect gets its own cached registry built lazily on the
/// first request that names it — feature providers in
/// `tcl-lsp-core` take a `&CommandRegistry` so no per-request
/// rebuild is needed. The cache also owns the plain-Tcl base
/// registry under the empty-string key, so registries are dropped
/// with the `Backend` rather than leaked for the process lifetime.
pub struct Backend {
    client: Client,
    /// Open documents. `Arc` so the detached diagnostics task can hold it.
    documents: Arc<Mutex<HashMap<Url, DocumentState>>>,
    /// Per-URI coalescing diagnostics scheduler state.  Each edit marks the
    /// document dirty and, if no worker is running, starts one; the single
    /// worker debounces, then repeatedly analyses the document's **latest**
    /// state until it has published the current version (retrying if a run was
    /// cancelled mid-flight).  This guarantees the final version's diagnostics
    /// are always published even under heavy edit bursts / CPU load — replacing
    /// the older per-edit generation-counter scheme, which could drop the last
    /// run with no retry.
    diag_slots: Arc<Mutex<HashMap<Url, DiagSlot>>>,
    /// Fallback dialect string used when ``did_open`` cannot derive
    /// one from the ``languageId`` and no per-session
    /// ``workspace/didChangeConfiguration`` has been received yet.
    /// Updated by ``did_change_configuration`` so editor reconfigures
    /// take effect for subsequently-opened documents.
    default_dialect: Mutex<String>,
    /// Workspace folder roots received from `initialize` /
    /// `workspace/didChangeWorkspaceFolders`.  Stored as
    /// `Url` (typically `file://...` directories).
    /// `S-workspace-init` minimal port — the folder list
    /// supports cross-document features (workspace symbols
    /// already walks every cached document; future
    /// `S-workspace-symbols-rich` extends this by scanning
    /// folder contents on disk).
    workspace_folders: Mutex<Vec<Url>>,
    /// Optional per-folder dialect override map keyed on the
    /// folder URL prefix (typically `file://...`).  Populated
    /// from the `initializationOptions.folderDialects` JSON
    /// object when present.  `S-workspace-init` / `SYNC-MAY19-
    /// dialect-contextvar`: enables multi-folder workspaces
    /// with mixed dialects to parse correctly by selecting the
    /// longest-prefix folder's dialect when a document is
    /// opened.
    folder_dialects: Mutex<Vec<(Url, String)>>,
    /// Per-folder editor configuration overrides (diagnostics, optimiser,
    /// formatting, feature toggles), keyed by folder URI and resolved by
    /// longest prefix at read time.  Populated by the per-folder
    /// `workspace/configuration` pull.  Empty in a single-root workspace, where
    /// every read falls back to the process-global fields below.  Mirrors the
    /// Python `editor_config_settings_per_folder`.
    folder_configs: Mutex<Vec<(Url, FolderConfig)>>,
    /// Per-folder salsa `AnalyserConfig` input handles, present only for folders
    /// that override the disabled-diagnostics set or non-ASCII mode.  The
    /// diagnostics path resolves the handle by longest prefix so a folder's
    /// W-code suppression reaches the cached `file_analysis` query, not just the
    /// server-side lift.  Folders without such overrides fall back to
    /// [`Backend::db_config`].
    folder_db_configs: Arc<Mutex<Vec<(Url, tcl_lsp_db::AnalyserConfig)>>>,
    /// W108 non-ASCII detection mode (`tclLsp.style.nonAscii`).
    /// [`NonAsciiMode::Default`] until an editor configures it via
    /// `initializationOptions` or `workspace/didChangeConfiguration`.
    /// Threaded into every `Analyser` the diagnostics path builds.
    non_ascii_mode: Mutex<NonAsciiMode>,
    /// Diagnostic codes the user has disabled (`tclLsp.diagnostics.<CODE>
    /// = false`). Threaded into every analyser build so the disabled
    /// codes are filtered, and consulted by the source-style pass.
    disabled_diagnostics: Mutex<HashSet<String>>,
    /// Cross-document proc / class definition index, maintained
    /// incrementally as documents open / change / close.  Lets
    /// completion enumerate procs from sibling files and
    /// (later) cross-document go-to-definition resolve symbols
    /// defined elsewhere.
    /// `Arc` so the detached diagnostics task can update it off the event loop.
    workspace_index: Arc<Mutex<core_workspace_index::WorkspaceIndex>>,
    /// Per-feature provider toggles (`tclLsp.features.*`).  Absent
    /// keys default to enabled, so a config that names only some
    /// features leaves the rest on.  Consulted by each provider
    /// entry point and surfaced by `getEffectiveConfig`.  Mirrors
    /// the Python server's `_FEATURE_TOGGLE_KEYS` resolution.
    feature_toggles: Mutex<FeatureToggles>,
    /// Optimiser master switch (`tclLsp.optimiser.enabled`).  When
    /// `false`, the `tcl-lsp.optimiseDocument` command yields no
    /// rewrites.  Default on.
    optimiser_enabled: Mutex<bool>,
    /// Optimisation profile (`tclLsp.optimiser.profile`) controlling which
    /// O-code categories surface as diagnostics. Default
    /// [`tcl_compiler::optimiser::profiles::DEFAULT_EDITOR_PROFILE`]
    /// (`readability`).
    optimiser_profile: Mutex<tcl_compiler::optimiser::profiles::OptimisationProfile>,
    /// Per-code optimiser overrides (`tclLsp.optimiser.<CODE>` = bool): a code
    /// mapped to `true` is force-*enabled* (removed from the profile's disabled
    /// set), `false` is force-*disabled* (added). Layered on top of the
    /// profile-derived set, mirroring Python's per-code `disabled_optimisations`
    /// adjustment in `settings.py`.
    optimiser_code_overrides: Mutex<HashMap<String, bool>>,
    /// Resolved formatter line length (`tclLsp.formatting.lineLength`).
    /// Surfaced by `getEffectiveConfig`; default 80.
    line_length: Mutex<u32>,
    /// Incremental query database (salsa 0.26) — the single memoised store of
    /// derived facts that is replacing the hand-maintained caches above.  The
    /// `db` handle is the write side (set inputs); reads clone it onto a
    /// worker thread and catch `salsa::Cancelled` when a newer edit supersedes
    /// the request.
    db: Arc<Mutex<tcl_lsp_db::TclDatabase>>,
    /// Per-URI salsa `SourceFile` input handles — the input-of-record the
    /// query graph reads.  Kept current by `did_open` / `did_change`.
    db_files: Arc<Mutex<HashMap<Url, tcl_lsp_db::SourceFile>>>,
    /// The salsa `Project` input — the workspace file set, kept in lock-step with
    /// `db_files` (re-set only when membership changes, on open/close), driving
    /// the cross-file `project_diagnostics` query (SRV-INCREMENTAL Task 6).
    /// `None` until the first document is tracked.
    db_project: Arc<Mutex<Option<tcl_lsp_db::Project>>>,
    /// The salsa `AnalyserConfig` input (disabled diagnostics + non-ASCII
    /// mode); `set_*` on `workspace/didChangeConfiguration`.  Used as the
    /// fallback when no per-folder override applies to the document.
    db_config: Arc<Mutex<tcl_lsp_db::AnalyserConfig>>,
    /// Last-published diagnostics per open document, keyed by URI, for the
    /// pull-diagnostic path.  Written by the push pipeline and read by the
    /// `textDocument/diagnostic` / `workspace/diagnostic` handlers; evicted on
    /// `did_close`.  Mirrors the Python `_pull_diag_cache`.
    pull_diag_cache: Arc<Mutex<HashMap<Url, PullDiagEntry>>>,
}

/// Resolved `tclLsp.features.*` toggle state.
///
/// Stores only the keys an editor has explicitly set; every other
/// feature resolves to enabled.  This matches the Python server's
/// "absent → default-on" semantics and the config-pull restore
/// contract (a pulled config only *sets* the keys it carries).
#[derive(Debug, Default, Clone)]
struct FeatureToggles {
    set: HashMap<String, bool>,
}

impl FeatureToggles {
    /// camelCase feature keys reported by `getEffectiveConfig`,
    /// mirroring Python's `_FEATURE_TOGGLE_KEYS`.
    const KEYS: &'static [&'static str] = &[
        "hover",
        "completion",
        "diagnostics",
        "semanticTokens",
        "codeActions",
        "definition",
        "references",
        "documentSymbols",
        "folding",
        "rename",
        "signatureHelp",
        "workspaceSymbols",
        // Inlay hints split into two independently-toggled families
        // (PR #643): inferred-type hints and parameter-name hints.  Both
        // default **off** (see `DEFAULT_OFF`).  The retired `inlayHints`
        // key is accepted on input as an alias for `inlayTypeHints`
        // (matching the Python server — see `apply`).
        "inlayTypeHints",
        "inlayParameterHints",
        "callHierarchy",
        "documentLinks",
        "selectionRange",
        "documentHighlight",
        "codeLens",
        "implementation",
        "typeDefinition",
        "declaration",
        "linkedEditingRange",
        // FE-DIAG-F5: opt-in XC100-301 translatability diagnostics for
        // `f5-irules` documents (default **off**, mirroring Python's
        // `tclLsp.xcDiagnostics.enabled` / `xc_diagnostics_enabled`).
        "xcDiagnostics",
    ];

    /// camelCase feature keys that default **off** (opt-in) rather than
    /// on.  `getEffectiveConfig` (`resolved_map`) and `is_enabled` must
    /// report these as disabled until an editor sets them, so the
    /// reported state matches the inlay handler's default-off gate
    /// (otherwise an exported `config.ini` would claim the hints are on
    /// and re-importing it would enable them).
    const DEFAULT_OFF: &'static [&'static str] =
        &["inlayTypeHints", "inlayParameterHints", "xcDiagnostics"];

    /// The default fallback for `feature` when no editor has set it:
    /// `false` for the opt-in [`Self::DEFAULT_OFF`] keys, `true`
    /// otherwise.
    fn default_enabled(feature: &str) -> bool {
        !Self::DEFAULT_OFF.contains(&feature)
    }

    /// Whether `feature` is enabled — explicitly-set value, else the
    /// per-feature default ([`Self::default_enabled`]).
    fn is_enabled(&self, feature: &str) -> bool {
        self.set
            .get(feature)
            .copied()
            .unwrap_or_else(|| Self::default_enabled(feature))
    }

    /// Like [`Self::is_enabled`] but with a default-**off** fallback, for
    /// opt-in features (e.g. `willSaveWaitUntil`) that the Python server
    /// leaves disabled unless an editor turns them on.
    fn is_enabled_default_off(&self, feature: &str) -> bool {
        self.set.get(feature).copied().unwrap_or(false)
    }

    /// Merge an editor-supplied `features` object, setting only the
    /// keys it carries (absent keys keep their last-applied value).
    ///
    /// The retired `inlayHints` key is a backward-compatible alias for
    /// `inlayTypeHints` (matching the Python server's
    /// `_FEATURE_TOGGLE_KEYS`): an existing explicit opt-in keeps showing
    /// the useful inferred-variable-type hints after the rename, while the
    /// verbose parameter-name hints stay off.  Applied first so an
    /// explicit new `inlayTypeHints` in the same object always wins when a
    /// config carries both.
    fn apply(&mut self, features: &serde_json::Map<String, serde_json::Value>) {
        if let Some(flag) = features
            .get("inlayHints")
            .and_then(serde_json::Value::as_bool)
        {
            self.set.insert("inlayTypeHints".to_owned(), flag);
        }
        for (key, value) in features {
            if key == "inlayHints" {
                continue;
            }
            if let Some(flag) = value.as_bool() {
                self.set.insert(key.clone(), flag);
            }
        }
    }

    /// Record a single feature flag explicitly — for toggles that live in a
    /// dedicated config *section* (e.g. `tclLsp.xcDiagnostics.enabled`)
    /// rather than the flat `features` object the editor sends to
    /// [`Self::apply`].
    fn set_flag(&mut self, feature: &str, value: bool) {
        self.set.insert(feature.to_owned(), value);
    }

    /// The full resolved `{feature: bool}` map for `getEffectiveConfig`.
    fn resolved_map(&self) -> serde_json::Map<String, serde_json::Value> {
        Self::KEYS
            .iter()
            .map(|&k| (k.to_owned(), serde_json::Value::Bool(self.is_enabled(k))))
            .collect()
    }
}

/// One workspace folder's editor configuration overrides.
///
/// Each field is `None` (or empty) when the folder's pulled `tclLsp` config
/// did not set it, in which case the resolver falls back to the process-global
/// value.  This is the Rust analogue of the Python server's
/// `editor_config_settings_per_folder`: in a multi-root workspace each root can
/// carry its own diagnostics, optimiser, formatting, and feature settings.
/// Per-folder *dialect* is handled separately by [`Backend::folder_dialects`].
#[derive(Clone, Default)]
struct FolderConfig {
    feature_toggles: FeatureToggles,
    disabled_diagnostics: Option<HashSet<String>>,
    non_ascii_mode: Option<NonAsciiMode>,
    optimiser_enabled: Option<bool>,
    optimiser_profile: Option<tcl_compiler::optimiser::profiles::OptimisationProfile>,
    optimiser_code_overrides: HashMap<String, bool>,
    line_length: Option<u32>,
}

/// Pick the value associated with the **longest** folder URI that `uri` sits
/// under, mirroring the Python server's longest-prefix folder resolution.
fn longest_folder_match<'a, T>(entries: &'a [(Url, T)], uri: &Url) -> Option<&'a T> {
    let mut best: Option<&'a (Url, T)> = None;
    for entry in entries {
        if uri_under_folder(uri.as_str(), entry.0.as_str())
            && best.is_none_or(|b| entry.0.as_str().len() > b.0.as_str().len())
        {
            best = Some(entry);
        }
    }
    best.map(|(_, value)| value)
}

impl std::fmt::Debug for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backend").finish_non_exhaustive()
    }
}

impl Backend {
    /// Build a fresh backend wrapping the given client.
    ///
    /// Dialect-specific registries (including the plain-Tcl base)
    /// are loaded lazily from [`Backend::registry_for_dialect`] on
    /// the first request that names them. Pre-loading every
    /// dialect into one shared registry would let dialect-specific
    /// command overrides (e.g. iRules's `proc` shape) shadow the
    /// Tcl spec for requests on unrelated dialects.
    ///
    /// `default_dialect` is the dialect string forwarded to
    /// providers when the document store has no override.
    #[must_use]
    pub fn new(client: Client) -> Self {
        let db = tcl_lsp_db::TclDatabase::default();
        let db_config = tcl_lsp_db::AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        Self {
            client,
            documents: Arc::new(Mutex::new(HashMap::new())),
            diag_slots: Arc::new(Mutex::new(HashMap::new())),
            default_dialect: Mutex::new("tcl8.6".to_owned()),
            workspace_folders: Mutex::new(Vec::new()),
            folder_dialects: Mutex::new(Vec::new()),
            folder_configs: Mutex::new(Vec::new()),
            folder_db_configs: Arc::new(Mutex::new(Vec::new())),
            non_ascii_mode: Mutex::new(NonAsciiMode::Default),
            disabled_diagnostics: Mutex::new(HashSet::new()),
            workspace_index: Arc::new(Mutex::new(core_workspace_index::WorkspaceIndex::new())),
            feature_toggles: Mutex::new(FeatureToggles::default()),
            optimiser_enabled: Mutex::new(true),
            optimiser_profile: Mutex::new(default_optimiser_profile()),
            optimiser_code_overrides: Mutex::new(HashMap::new()),
            line_length: Mutex::new(80),
            db: Arc::new(Mutex::new(db)),
            db_files: Arc::new(Mutex::new(HashMap::new())),
            db_project: Arc::new(Mutex::new(None)),
            db_config: Arc::new(Mutex::new(db_config)),
            pull_diag_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create or update the salsa `SourceFile` input for `uri`.  Called by
    /// `did_open` / `did_change` so the query graph always reads current text.
    /// Lock order is always `db` then `db_files`.
    async fn db_set_source(&self, uri: &Url, text: String, dialect: String) {
        use salsa::Setter as _;
        let mut db = self.db.lock().await;
        let mut files = self.db_files.lock().await;
        if let Some(&file) = files.get(uri) {
            // Text/dialect edit: membership unchanged, so the `Project` input is
            // untouched (a body edit then backdates `project_proc_names`).
            file.set_text(&mut *db).to(text);
            file.set_dialect(&mut *db).to(dialect);
        } else {
            let file = tcl_lsp_db::SourceFile::new(&*db, text, dialect);
            files.insert(uri.clone(), file);
            // Membership changed — re-set the `Project` file set.
            let mut project = self.db_project.lock().await;
            Self::sync_db_project(&mut db, &files, &mut project);
        }
    }

    /// Drop the salsa `SourceFile` input for `uri` (on `did_close`).  Lock order
    /// is `db` → `db_files` → `db_project`, matching [`Self::db_set_source`].
    async fn db_remove_source(&self, uri: &Url) {
        let mut db = self.db.lock().await;
        let mut files = self.db_files.lock().await;
        if files.remove(uri).is_some() {
            let mut project = self.db_project.lock().await;
            Self::sync_db_project(&mut db, &files, &mut project);
        }
    }

    /// Re-set the salsa [`tcl_lsp_db::Project`] input to the current `db_files`
    /// set (sorted by URI for a stable, iteration-order-independent `Vec` so the
    /// input only changes when membership does — not on `HashMap` reshuffles).
    /// Called only when membership changes (open/close), so a text edit never
    /// re-derives the project aggregates.  The `Project` handle is stable across
    /// re-sets, so workers holding it keep reading the current value.
    fn sync_db_project(
        db: &mut tcl_lsp_db::TclDatabase,
        files: &HashMap<Url, tcl_lsp_db::SourceFile>,
        project: &mut Option<tcl_lsp_db::Project>,
    ) {
        use salsa::Setter as _;
        let mut entries: Vec<(&Url, &tcl_lsp_db::SourceFile)> = files.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        let sources: Vec<tcl_lsp_db::SourceFile> = entries.into_iter().map(|(_, &f)| f).collect();
        match *project {
            Some(p) => {
                p.set_files(db).to(sources);
            }
            None => *project = Some(tcl_lsp_db::Project::new(&*db, sources)),
        }
    }

    /// Re-resolve every open document's dialect after a session-default
    /// change and reschedule diagnostics for those whose dialect actually
    /// moved. An explicit `languageId` / BIG-IP basename / per-folder
    /// override still wins (it did at `didOpen`), so only documents that
    /// fell back to the session default change. Mirrors the Python
    /// `_apply_merged_settings_now` re-resolve-and-reanalyse loop.
    async fn reresolve_open_document_dialects(&self) {
        // Snapshot `(uri, language_id, current dialect)` first — the async
        // `dialect_for_open` calls lock `folder_dialects` / `default_dialect`,
        // so they must not run while the `documents` lock is held.
        let snapshot: Vec<(Url, String, String)> = {
            let docs = self.documents.lock().await;
            docs.iter()
                .map(|(uri, doc)| (uri.clone(), doc.language_id.clone(), doc.dialect.clone()))
                .collect()
        };
        for (uri, language_id, old_dialect) in snapshot {
            let new_dialect = self.dialect_for_open(&uri, &language_id).await;
            if new_dialect == old_dialect {
                continue;
            }
            // Commit the new dialect to the live document + salsa input while
            // holding `documents` (the same `documents` → `db` /
            // `workspace_index` ordering `did_open` uses; `db_set_source`
            // never locks `documents`, so the nesting is cycle-free).
            {
                let mut docs = self.documents.lock().await;
                let Some(doc) = docs.get_mut(&uri) else {
                    continue; // closed between snapshot and commit
                };
                doc.dialect.clone_from(&new_dialect);
                let text = doc.text.clone();
                self.db_set_source(&uri, text, new_dialect.clone()).await;
                self.workspace_index
                    .lock()
                    .await
                    .remove_document(uri.as_str());
            }
            self.schedule_diagnostics(uri, new_dialect).await;
        }
    }

    /// Mirror the editor analyser config (disabled diagnostics + non-ASCII
    /// mode) onto the salsa `AnalyserConfig` input.  The disabled set is
    /// sorted so the input value is stable (a `HashSet` iteration order change
    /// must not look like an edit to salsa).
    async fn sync_db_config(&self) {
        use salsa::Setter as _;
        let mut disabled: Vec<String> = self
            .disabled_diagnostics
            .lock()
            .await
            .iter()
            .cloned()
            .collect();
        disabled.sort();
        let mode = *self.non_ascii_mode.lock().await;
        let config = *self.db_config.lock().await;
        let mut db = self.db.lock().await;
        config.set_disabled_diagnostics(&mut *db).to(disabled);
        config.set_non_ascii_mode(&mut *db).to(mode);
    }

    /// Run the salsa `document_symbols` query for `uri` on a worker thread,
    /// reading the current `SourceFile` input.  Returns `None` when the input
    /// is absent or a concurrent edit cancels the read, so the caller can fall
    /// back to a direct computation (behaviour preserved).
    async fn db_document_symbols(
        &self,
        uri: &Url,
    ) -> Option<Vec<tcl_lsp_core::document_symbols::DocumentSymbol>> {
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let config = *self.db_config.lock().await;
        let snapshot = self.db.lock().await.clone();
        tokio::task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| tcl_lsp_db::document_symbols(&snapshot, file, config)).ok()
        })
        .await
        .ok()
        .flatten()
    }

    /// Run the salsa `file_analysis` query for `uri` on a worker thread,
    /// reading the current `SourceFile` input.  Returns `None` when the input
    /// is absent or a concurrent edit cancels the read.  The returned `Arc`
    /// shares the memoised analysis (no deep clone of `AnalysisResult`).
    async fn db_file_analysis(
        &self,
        uri: &Url,
    ) -> Option<Arc<tcl_compiler::analyser::AnalysisResult>> {
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let config = *self.db_config.lock().await;
        let snapshot = self.db.lock().await.clone();
        tokio::task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| tcl_lsp_db::file_analysis(&snapshot, file, config)).ok()
        })
        .await
        .ok()
        .flatten()
    }

    /// Run the salsa `semantic_tokens` query for `uri` on a worker thread.
    /// `None` when the input is absent or a concurrent edit cancels the read.
    async fn db_semantic_tokens(
        &self,
        uri: &Url,
    ) -> Option<tcl_lsp_core::semantic_tokens::SemanticTokens> {
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let snapshot = self.db.lock().await.clone();
        tokio::task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| tcl_lsp_db::semantic_tokens(&snapshot, file)).ok()
        })
        .await
        .ok()
        .flatten()
    }

    /// Read the memoised `AnalysisResult` for `uri` from the query database, if
    /// the document has a `SourceFile` input.  Returns an owned clone so the
    /// caller can move it into a `spawn_blocking` worker.  `None` when there is
    /// no input (the caller analyses fresh) or a concurrent edit cancelled the
    /// read.
    async fn cached_analysis(&self, uri: &Url) -> Option<tcl_compiler::analyser::AnalysisResult> {
        self.db_file_analysis(uri).await.map(|a| (*a).clone())
    }

    /// Resolve an analysis for the document.  Consults the
    /// cache first; computes a fresh analysis when no entry
    /// exists.  Returns owned data the caller can move into
    /// a `spawn_blocking` worker.
    async fn analysis_for(
        &self,
        uri: &Url,
        text: String,
        dialect: String,
    ) -> tcl_compiler::analyser::AnalysisResult {
        if let Some(cached) = self.cached_analysis(uri).await {
            return cached;
        }
        let (disabled, na_mode) = self.analyser_config().await;
        tokio::task::spawn_blocking(move || {
            let mut analyser = Self::configured_analyser(disabled, na_mode);
            analyser.analyse(&text, &dialect).clone()
        })
        .await
        .unwrap_or_default()
    }

    /// Return a snapshot of the current workspace folder
    /// URLs.  Used by cross-document features (workspace
    /// symbols, cross-doc references / rename / call-
    /// hierarchy).
    pub async fn workspace_folder_urls(&self) -> Vec<Url> {
        self.workspace_folders.lock().await.clone()
    }

    /// `S-workspace-init`: copy the workspace folders the
    /// editor sent into `self.workspace_folders` so cross-
    /// document features can resolve relative paths and (in
    /// the future) scan folder contents for a workspace index.
    /// Both the newer `workspace_folders` field and the
    /// single-root `root_uri` fallback are supported.
    async fn apply_workspace_folders(&self, params: &InitializeParams) {
        if let Some(folders) = &params.workspace_folders {
            let urls: Vec<Url> = folders.iter().map(|f| f.uri.clone()).collect();
            *self.workspace_folders.lock().await = urls;
        } else if let Some(root) = &params.root_uri {
            *self.workspace_folders.lock().await = vec![root.clone()];
        }
    }

    /// `SYNC-MAY19-dialect-contextvar`: read per-folder dialect
    /// overrides from `params.initialization_options`.
    ///
    /// Expected shape:
    /// `{ "folderDialects": { "file:///path": "f5-irules",
    ///                        "file:///other": "tcl9.0" } }`.
    ///
    /// Unknown dialect names and unparseable folder URLs are
    /// dropped silently rather than failing the entire
    /// initialise — the server keeps starting up with whatever
    /// valid entries it could pull from the editor.
    async fn apply_initialization_options(&self, params: &InitializeParams) {
        let Some(opts) = &params.initialization_options else {
            return;
        };
        // `folderDialects` map (per-folder dialect overrides).
        if let Some(entries) = opts
            .as_object()
            .and_then(|m| m.get("folderDialects"))
            .and_then(serde_json::Value::as_object)
        {
            let mut parsed: Vec<(Url, String)> = Vec::new();
            for (folder_url, dialect_val) in entries {
                let Ok(url) = Url::parse(folder_url) else {
                    continue;
                };
                let Some(dialect) = dialect_val.as_str() else {
                    continue;
                };
                if DialectSet::parse(dialect).is_none() {
                    continue;
                }
                parsed.push((url, dialect.to_owned()));
            }
            *self.folder_dialects.lock().await = parsed;
        }
        // Diagnostic settings (`tclLsp.style.nonAscii` /
        // `tclLsp.diagnostics.<CODE>`), accepted in either the nested or
        // flat-dotted shape.
        if let Some(mode) = settings_non_ascii_mode(opts) {
            *self.non_ascii_mode.lock().await = mode;
        }
        if let Some(disabled) = settings_disabled_diagnostics(opts) {
            *self.disabled_diagnostics.lock().await = disabled;
        }
    }

    /// Resolve the dialect string a freshly opened document should
    /// be tagged with.
    ///
    /// Resolution order (`SYNC-MAY19-dialect-contextvar`):
    ///
    /// 1. The LSP ``languageId`` field — when it names a known
    ///    dialect (``"tcl-irule"`` / ``"f5-irules"`` / ``"tcl9.0"``
    ///    / etc.), use it directly.
    /// 2. The per-folder override map (`folder_dialects`) — when
    ///    the document URI sits under one of the configured folder
    ///    URLs, use the deepest-matching folder's dialect.
    /// 3. The session-wide ``default_dialect`` fallback.
    async fn dialect_for_open(&self, uri: &Url, language_id: &str) -> String {
        // An explicit BIG-IP language id (`tcl-bigip`, advertised by the VS
        // Code extension, or the canonical `f5-bigip`) selects the BIG-IP
        // config dialect even when the basename is not a canonical
        // `bigip*.conf` name — e.g. a user who manually picks the BIG-IP
        // language mode on a differently-named config file. `f5-bigip` is a
        // custom config format, not a `DialectSet`-parseable Tcl dialect, so it
        // is resolved here rather than via `dialect_from_language_id` (which is
        // restricted to Tcl dialects by its `debug_assert`). Mirrors the
        // BIG-IP routing the `is_bigip_conf_name` basename branch below feeds.
        if matches!(language_id, "tcl-bigip" | "f5-bigip") {
            return "f5-bigip".to_owned();
        }
        let lang_dialect = Self::dialect_from_language_id(language_id);
        // A canonical BIG-IP basename (``bigip.conf``, ``bigip_base.conf``,
        // …) routes to ``f5-bigip`` ahead of a *generic* Tcl ``languageId``,
        // mirroring Python ``infer_document_dialect``: only an explicit
        // non-Tcl dialect id (``f5-irules`` / ``f5-iapps`` / ``expect`` /
        // EDA / ``tk``) wins over the basename. This is what lets the test
        // harness open ``bigip.conf`` with ``languageId: "tcl"`` and still
        // get the BIG-IP path (parse / outline + general-Tcl-diagnostic
        // suppression).
        let explicit_non_tcl = matches!(lang_dialect, Some(d) if !d.starts_with("tcl"));
        if !explicit_non_tcl && core_bigip::is_bigip_conf_name(uri.as_str()) {
            return "f5-bigip".to_owned();
        }
        if let Some(d) = lang_dialect {
            return d.to_owned();
        }
        if let Some(d) = self.resolve_folder_dialect(uri).await {
            return d;
        }
        self.default_dialect.lock().await.clone()
    }

    /// Whether `dialect` denotes an F5 BIG-IP config document — the
    /// signal every BIG-IP-specific branch (analysis suppression,
    /// config outline) keys on.
    fn is_bigip_dialect(dialect: &str) -> bool {
        dialect == "f5-bigip"
    }

    /// Look up the per-folder dialect override for `uri`,
    /// preferring the deepest (longest-prefix) match so nested
    /// folders shadow their parents.
    async fn resolve_folder_dialect(&self, uri: &Url) -> Option<String> {
        let folders = self.folder_dialects.lock().await;
        folder_dialect_for(uri, &folders)
    }

    /// Map an LSP ``languageId`` string to a dialect name accepted
    /// by ``DialectSet::parse`` / the providers' ``dialect`` arg.
    ///
    /// Recognises the editor-extension language ids
    /// (``tcl-irule`` → ``f5-irules``, etc.) plus the canonical
    /// dialect names used by the MCP bridge / direct callers
    /// (``f5-irules``, ``tcl9.0``, …).  Returns ``None`` when the
    /// language id does not name a known dialect — the caller falls
    /// back to the session default.
    fn dialect_from_language_id(language_id: &str) -> Option<&'static str> {
        // Group editor language ids alongside the canonical dialect
        // name they map to so callers in either world land on the
        // string the registry / provider trait expects.
        let mapped = match language_id {
            "tcl" | "tcl8.6" => "tcl8.6",
            "tcl8.4" => "tcl8.4",
            "tcl8.5" => "tcl8.5",
            "tcl9.0" => "tcl9.0",
            "tcl-irule" | "f5-irules" => "f5-irules",
            // `tcl-apl` is the APL (iApp presentation language) editor id — an
            // iApp sublanguage that Python treats under the iApps extension
            // set, so it analyses as `f5-iapps` rather than falling through to
            // the default Tcl dialect.
            "tcl-iapp" | "f5-iapps" | "tcl-apl" => "f5-iapps",
            "tcl-expect" | "expect" => "expect",
            "tcl-synopsys" | "synopsys-eda-tcl" => "synopsys-eda-tcl",
            "tcl-cadence" | "cadence-eda-tcl" => "cadence-eda-tcl",
            "tcl-xilinx" | "xilinx-eda-tcl" => "xilinx-eda-tcl",
            "tcl-quartus" | "intel-quartus-eda-tcl" => "intel-quartus-eda-tcl",
            "tcl-mentor" | "mentor-eda-tcl" => "mentor-eda-tcl",
            "tk" => "tk",
            _ => return None,
        };
        // Sanity-check the mapped string is something the registry
        // accepts; guards against typos in the table.
        debug_assert!(DialectSet::parse(mapped).is_some());
        Some(mapped)
    }

    async fn read_document(&self, url: &Url) -> Option<DocumentState> {
        if let Some(doc) = self.documents.lock().await.get(url).cloned() {
            return Some(doc);
        }
        // On-disk fallback: files the folder scan indexed but the
        // editor hasn't opened aren't in the open-document map.
        // Read them from disk so cross-document span→range
        // resolution (references / rename / call-hierarchy) can
        // reach their sources.  Non-`file://` URLs and unreadable
        // paths fall through to `None`.
        let path = url.to_file_path().ok()?;
        let text = tokio::task::spawn_blocking(move || std::fs::read_to_string(path))
            .await
            .ok()?
            .ok()?;
        let dialect = match self.resolve_folder_dialect(url).await {
            Some(d) => d,
            None => self.default_dialect.lock().await.clone(),
        };
        Some(DocumentState::new(text, dialect))
    }

    /// Re-index a (now-closed) document from its on-disk contents so
    /// its definitions / calls remain visible to cross-document
    /// features.  `scan_workspace_folders` only runs at
    /// `initialized`, so on `did_close` we must refresh the index
    /// from disk rather than dropping the entry — otherwise opening
    /// then closing a file would erase it from cross-document
    /// definition / references / rename / call-hierarchy until the
    /// server restarts.  Falls back to removing the entry when the
    /// URI isn't a readable file (untitled buffer, deleted file).
    ///
    /// Disk IO and analysis deliberately run before taking any lock; the final
    /// index update holds the `documents` lock across the still-closed re-check
    /// so a slow disk read cannot overwrite a newly reopened unsaved buffer.
    /// Remove from the cross-document index every entry whose URI sits under
    /// one of `folders` and is not currently open in the editor.  Used when a
    /// workspace folder is removed (`did_change_workspace_folders`) so its
    /// files stop contributing definitions / references / symbols.
    async fn drop_index_under_folders(&self, folders: &[Url]) {
        if folders.is_empty() {
            return;
        }
        let folder_strs: Vec<String> = folders.iter().map(ToString::to_string).collect();
        // Hold `documents` across the open-set read + index removals (the
        // `documents` → `workspace_index` order used since the global gate was
        // retired) so a file opening mid-reconcile keeps its open-buffer entry.
        let docs = self.documents.lock().await;
        let open: HashSet<String> = docs.keys().map(ToString::to_string).collect();
        let mut index = self.workspace_index.lock().await;
        let to_remove: Vec<String> = index
            .document_uris()
            .into_iter()
            .filter(|uri| {
                !open.contains(uri)
                    && folder_strs
                        .iter()
                        .any(|folder| uri_under_folder(uri, folder))
            })
            .collect();
        for uri in to_remove {
            index.remove_document(&uri);
        }
    }

    async fn reindex_index_from_disk(&self, uri: &Url) {
        let analysed: Option<AnalysisResult> = if let Ok(path) = uri.to_file_path() {
            let dialect = match self.resolve_folder_dialect(uri).await {
                Some(d) => d,
                None => self.default_dialect.lock().await.clone(),
            };
            tokio::task::spawn_blocking(move || {
                let text = std::fs::read_to_string(path).ok()?;
                Some(Analyser::new().analyse(&text, &dialect).clone())
            })
            .await
            .ok()
            .flatten()
        } else {
            None
        };
        // Hold `documents` across the still-closed re-check and the index
        // update (the `documents` → `workspace_index` order used everywhere
        // since the global gate was retired) so a concurrent `did_open` cannot
        // open the buffer between them and have its index entry clobbered by
        // this disk-backed one.
        let docs = self.documents.lock().await;
        if docs.contains_key(uri) {
            return;
        }
        let mut index = self.workspace_index.lock().await;
        index.remove_document(uri.as_str());
        if let Some(analysis) = analysed {
            index.add_document(uri.as_str(), &analysis);
        }
    }

    /// Shared helper for the goto-definition family — runs the
    /// pure-CPU `tcl_lsp_core::definition::definition` provider
    /// off the LSP event loop and returns the matched ranges.
    async fn compute_definition(&self, uri: &Url, pos: Position) -> jsonrpc::Result<Vec<Location>> {
        let Some(doc) = self.read_document(uri).await else {
            return Ok(Vec::new());
        };
        let analysis = self
            .analysis_for(uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // The cross-document fallback should only fire when the cursor
        // sits on a command head (a call site), matching what the
        // in-document provider resolves.  Compute that here, while
        // `analysis` is still in scope (it is moved into the worker
        // below).  Without this gate, an argument word that happens to
        // collide with a sibling proc/class name produces a
        // false-positive go-to-definition jump.
        let on_command_head = position_is_command_head(&doc.text, pos, &analysis);
        let text = doc.text.clone();
        let in_doc = tokio::task::spawn_blocking(move || {
            core_definition::definition(&text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("definition worker panicked: {err}").into(),
            data: None,
        })?;
        if !in_doc.is_empty() {
            // Resolved within the current document.
            return Ok(in_doc
                .into_iter()
                .map(|r| Location {
                    uri: uri.clone(),
                    range: lift_lsp_range(r),
                })
                .collect());
        }
        if !on_command_head {
            return Ok(Vec::new());
        }
        // Cross-document fallback: resolve a proc / class
        // defined in a sibling document via the workspace index.
        self.cross_document_definition(uri, &doc.text, pos).await
    }

    /// Resolve the symbol at `pos` against the workspace index
    /// when the current document has no local definition.  Only
    /// fires on bare command words (not `$var` references), and
    /// returns `Location`s pointing into the *defining*
    /// documents.
    async fn cross_document_definition(
        &self,
        uri: &Url,
        source: &str,
        pos: Position,
    ) -> jsonrpc::Result<Vec<Location>> {
        // A `$var` reference can't resolve to a cross-document
        // proc / class.
        if core_hover::find_var_at_position(source, pos.line, pos.character).is_some() {
            return Ok(Vec::new());
        }
        let Some((word, _, _)) =
            core_hover::find_word_span_at_position(source, pos.line, pos.character)
        else {
            return Ok(Vec::new());
        };
        // Collect (uri, name_span) targets from the index.
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.lock().await;
            index
                .proc_definitions(&word, uri.as_str())
                .into_iter()
                .map(|p| (p.uri.clone(), p.name_span))
                .chain(
                    index
                        .class_definitions(&word, uri.as_str())
                        .into_iter()
                        .map(|c| (c.uri.clone(), c.name_span)),
                )
                .collect()
        };
        Ok(self.resolve_target_locations(targets).await)
    }

    /// Resolve a list of `(target-uri, byte-span)` pairs to LSP
    /// `Location`s, converting each span against the *target*
    /// document's source.  Targets whose document isn't in the
    /// store, or whose URI doesn't parse, are dropped.
    async fn resolve_target_locations(
        &self,
        targets: Vec<(String, tcl_lexer::Span)>,
    ) -> Vec<Location> {
        let mut locations = Vec::new();
        for (target_uri, span) in targets {
            let Ok(parsed) = Url::parse(&target_uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let line_index = target_doc.line_index.clone();
            let start = line_index.position_at_utf16(span.start(), &target_doc.text);
            let end = line_index.position_at_utf16(span.end(), &target_doc.text);
            locations.push(Location {
                uri: parsed,
                range: Range {
                    start: Position {
                        line: start.line,
                        character: start.character,
                    },
                    end: Position {
                        line: end.line,
                        character: end.character,
                    },
                },
            });
        }
        locations
    }

    /// Resolve the proc / class symbol at `pos` (a bare command
    /// word, not a `$var`) to its `(simple_name,
    /// qualified_name)`, consulting the current document's
    /// analysis first and the workspace index second.  Used by
    /// cross-document references / rename to identify which
    /// symbol's call sites to gather.
    async fn resolve_workspace_symbol(
        &self,
        uri: &Url,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> Option<(String, String)> {
        if core_hover::find_var_at_position(source, pos.line, pos.character).is_some() {
            return None;
        }
        let (word, _, _) = core_hover::find_word_span_at_position(source, pos.line, pos.character)?;
        // Current document: proc, then class.
        for (qname, proc_def) in &analysis.all_procs {
            if proc_def.name == word || qname == &word || qname == &format!("::{word}") {
                return Some((proc_def.name.clone(), proc_def.qualified_name.clone()));
            }
        }
        for class_def in analysis.all_classes.values() {
            if class_def.name == word
                || class_def.qualified_name == word
                || class_def.qualified_name == format!("::{word}")
            {
                return Some((class_def.name.clone(), class_def.qualified_name.clone()));
            }
        }
        // Otherwise the symbol may be defined in a sibling
        // document — resolve its qualified name from the index.
        // Gate this on the cursor actually sitting on a command
        // invocation head in *this* document: without it, a
        // coincidental bareword argument (`puts hello`) would resolve
        // against any sibling document that happens to define a proc /
        // class of the same name, producing spurious cross-document
        // references / rename edits.
        if !position_is_command_head(source, pos, analysis) {
            return None;
        }
        let index = self.workspace_index.lock().await;
        if let Some(p) = index.proc_definitions(&word, uri.as_str()).first() {
            return Some((p.name.clone(), p.qualified_name.clone()));
        }
        if let Some(c) = index.class_definitions(&word, uri.as_str()).first() {
            return Some((c.name.clone(), c.qualified_name.clone()));
        }
        None
    }

    /// Cross-document references for the proc / class at `pos`:
    /// every invocation site in *other* documents, plus the
    /// definition sites in other documents when
    /// `include_declaration`.  Returns `Location`s resolved
    /// against their defining documents.
    async fn cross_document_references(
        &self,
        uri: &Url,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        include_declaration: bool,
    ) -> Vec<Location> {
        let Some((simple, qualified)) = self
            .resolve_workspace_symbol(uri, source, analysis, pos)
            .await
        else {
            return Vec::new();
        };
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.lock().await;
            let mut t: Vec<(String, tcl_lexer::Span)> = index
                .invocations_of(&simple, &qualified, uri.as_str())
                .into_iter()
                .map(|i| (i.uri.clone(), i.range))
                .collect();
            if include_declaration {
                for p in index.proc_definitions(&simple, uri.as_str()) {
                    t.push((p.uri.clone(), p.name_span));
                }
                for c in index.class_definitions(&simple, uri.as_str()) {
                    t.push((c.uri.clone(), c.name_span));
                }
            }
            t
        };
        self.resolve_target_locations(targets).await
    }

    /// Add cross-document rename edits for the proc / class at
    /// `pos` into `changes`.  Resolves the symbol, asks the core
    /// rename provider for the namespace-aware sibling-document
    /// edit intents (call sites + definition sites), converts
    /// each byte span to a range against its target document,
    /// and merges into the per-URI edit map (deduped).
    async fn add_cross_document_rename_edits(
        &self,
        uri: &Url,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
        changes: &mut std::collections::HashMap<Url, Vec<TextEdit>>,
    ) {
        let Some((simple, qualified)) = self
            .resolve_workspace_symbol(uri, source, analysis, pos)
            .await
        else {
            return;
        };
        let intents = {
            let index = self.workspace_index.lock().await;
            core_rename::cross_document_symbol_edits(
                &simple,
                &qualified,
                new_name,
                &index,
                uri.as_str(),
            )
        };
        for intent in intents {
            let Ok(parsed) = Url::parse(&intent.uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let line_index = target_doc.line_index.clone();
            let start = line_index.position_at_utf16(intent.span.start(), &target_doc.text);
            let end = line_index.position_at_utf16(intent.span.end(), &target_doc.text);
            let edit = TextEdit {
                range: Range {
                    start: Position {
                        line: start.line,
                        character: start.character,
                    },
                    end: Position {
                        line: end.line,
                        character: end.character,
                    },
                },
                new_text: intent.new_text,
            };
            let bucket = changes.entry(parsed).or_default();
            if !bucket.iter().any(|e| e.range == edit.range) {
                bucket.push(edit);
            }
        }
    }

    /// Cross-document incoming calls to the proc whose
    /// qualified name is `qualified`: run the externally-
    /// targeted incoming-calls pass over every analysed
    /// document *other than* `current_uri`, tagging each result
    /// with its document's URI.  Documents that don't define
    /// the proc still contribute their call sites (passing
    /// `None` for the declaration span so nothing is skipped).
    async fn cross_document_incoming_calls(
        &self,
        current_uri: &Url,
        qualified: &str,
    ) -> Vec<(Url, core_call_hierarchy::IncomingCall)> {
        let simple = qualified.rsplit("::").next().unwrap_or(qualified);
        // Collect (uri, source, dialect) for every document *other
        // than* the current one: first the open buffers, then the
        // indexed-but-unopened files the folder scan discovered
        // (read from disk).  Without the latter, callers living in
        // files the editor never opened would be missing from the
        // incoming-call results even though they are indexed for
        // every other cross-document feature.
        let mut open_uris: HashSet<Url> = HashSet::new();
        let mut docs: Vec<(Url, String, String)> = {
            let store = self.documents.lock().await;
            store
                .iter()
                .filter(|(u, _)| *u != current_uri)
                .map(|(u, d)| {
                    open_uris.insert(u.clone());
                    (u.clone(), d.text.clone(), d.dialect.clone())
                })
                .collect()
        };
        let indexed = self.workspace_index.lock().await.document_uris();
        for uri_str in indexed {
            let Ok(uri) = Url::parse(&uri_str) else {
                continue;
            };
            if uri == *current_uri || open_uris.contains(&uri) {
                continue;
            }
            if let Some(doc) = self.read_document(&uri).await {
                docs.push((uri, doc.text, doc.dialect));
            }
        }
        let mut out = Vec::new();
        for (doc_uri, source, dialect) in docs {
            let analysis = self
                .cached_analysis(&doc_uri)
                .await
                .unwrap_or_else(|| Analyser::new().analyse(&source, &dialect).clone());
            let calls = core_call_hierarchy::incoming_calls_for_target(
                &source, &analysis, simple, qualified, None,
            );
            for c in calls {
                out.push((doc_uri.clone(), c));
            }
        }
        out
    }

    /// Cross-document outgoing calls from the queried proc /
    /// method: the call sites inside its body whose callee is
    /// defined in a *sibling* document.  The local
    /// [`core_call_hierarchy::outgoing_calls`] pass only resolves
    /// callees in the current document; this resolves the
    /// remaining heads against the workspace index, builds a `to`
    /// item pointing at the sibling definition (URI + name span →
    /// range against that document's source), and keeps the call
    /// ranges from the *current* document.
    async fn cross_document_outgoing_calls(
        &self,
        current_uri: &Url,
        source: &str,
        dialect: &str,
        item: &core_call_hierarchy::CallHierarchyItem,
        analysis: &AnalysisResult,
    ) -> Vec<CallHierarchyOutgoingCall> {
        let unresolved = {
            let source = source.to_owned();
            let dialect = dialect.to_owned();
            let item = item.clone();
            let analysis = analysis.clone();
            tokio::task::spawn_blocking(move || {
                core_call_hierarchy::unresolved_outgoing_calls(&source, &dialect, &item, &analysis)
            })
            .await
            .unwrap_or_default()
        };
        let mut out = Vec::new();
        for call in unresolved {
            // Resolve the callee head against the index, preferring
            // the analyser's resolved qualified name when present.
            let Some((target_uri, qualified, name_span, detail)) = ({
                let index = self.workspace_index.lock().await;
                let mut found = None;
                for key in [
                    call.resolved_qualified_name.as_deref(),
                    Some(call.name.as_str()),
                ]
                .into_iter()
                .flatten()
                {
                    if let Some(p) = index.proc_definitions(key, current_uri.as_str()).first() {
                        found = Some((
                            p.uri.clone(),
                            p.qualified_name.clone(),
                            p.name_span,
                            Some(format!("({} params)", p.param_count)),
                        ));
                        break;
                    }
                    if let Some(c) = index.class_definitions(key, current_uri.as_str()).first() {
                        found = Some((c.uri.clone(), c.qualified_name.clone(), c.name_span, None));
                        break;
                    }
                }
                found
            }) else {
                continue;
            };
            // Convert the sibling definition's name span to a range
            // against that document's source.
            let Ok(parsed) = Url::parse(&target_uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let line_index = target_doc.line_index.clone();
            let start = line_index.position_at_utf16(name_span.start(), &target_doc.text);
            let end = line_index.position_at_utf16(name_span.end(), &target_doc.text);
            let name_range = Range {
                start: Position {
                    line: start.line,
                    character: start.character,
                },
                end: Position {
                    line: end.line,
                    character: end.character,
                },
            };
            out.push(CallHierarchyOutgoingCall {
                to: CallHierarchyItem {
                    name: qualified,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail,
                    uri: parsed,
                    range: name_range,
                    selection_range: name_range,
                    data: None,
                },
                from_ranges: call.from_ranges.into_iter().map(lift_lsp_range).collect(),
            });
        }
        out
    }

    /// Handle the `tcl-lsp.minifyDocument` workspace command.
    ///
    /// Arguments (positional, matching the editor extensions):
    /// `[uri: string, compact: bool, aggressive: bool, isolated: bool]`.
    /// Returns a JSON object with `source`, `originalLength`,
    /// `minifiedLength`, and — for the compact / aggressive tiers —
    /// `symbolMap` (and `optimisationsApplied` for aggressive),
    /// mirroring `lsp/commands.py::on_minify_document`.
    async fn minify_document_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Url::parse(uri_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let compact = args
            .get(1)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let aggressive = args
            .get(2)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let isolated = args
            .get(3)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let registry = self.registry_for_dialect(&doc.dialect).await;

        // Pure-CPU work; run off the LSP event loop.
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let value = tokio::task::spawn_blocking(move || {
            if aggressive {
                let res = core_minify::minify_tcl_aggressive(&text, &dialect, isolated, &registry);
                serde_json::json!({
                    "source": res.source,
                    "originalLength": res.original_length,
                    "minifiedLength": res.minified_length(),
                    "symbolMap": res.symbol_map.format(),
                    "optimisationsApplied": res.optimisations_applied,
                })
            } else if compact {
                let (minified, symbol_map) =
                    core_minify::minify_tcl_compact(&text, &dialect, isolated, &registry);
                serde_json::json!({
                    "source": minified,
                    "originalLength": text.len(),
                    "minifiedLength": minified.len(),
                    "symbolMap": symbol_map.format(),
                })
            } else {
                let minified = core_minify::minify_tcl(&text, &dialect, &registry);
                serde_json::json!({
                    "source": minified,
                    "originalLength": text.len(),
                    "minifiedLength": minified.len(),
                })
            }
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("minify worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(value))
    }

    /// Handle the `tcl-lsp.optimiseDocument` workspace command.
    ///
    /// Arguments: `[uri, profile?]`.  Runs the optimiser over the document
    /// (multi-pass for the `"full"` profile, single-pass otherwise),
    /// applies the rewrites, and returns `{source, optimisations}` — the
    /// optimised text plus the list of applied optimisation suggestions.
    /// Mirrors the Python `tcl-lsp.optimiseDocument` command.
    async fn optimise_document_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Url::parse(uri_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // The `"full"` profile iterates to a fixpoint; anything else is a
        // single pass.  Default to `"full"`.
        let profile = args
            .get(1)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("full")
            .to_owned();
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let value = tokio::task::spawn_blocking(move || {
            let dialect_opt = Some(dialect.as_str());
            let (source, opts) = if profile == "full" {
                tcl_compiler::optimiser::optimise_source_multipass(&text, &registry, dialect_opt, 5)
            } else {
                let opts =
                    tcl_compiler::optimiser::optimise_with_dialect(&text, &registry, dialect_opt);
                let applied = tcl_compiler::optimiser::apply_optimisations(&text, &opts);
                (applied, opts)
            };
            let line_index = tcl_lexer::LineIndex::new(&text);
            let items: Vec<serde_json::Value> = opts
                .iter()
                .map(|o| {
                    let start = line_index.position_at_utf16(o.span.start(), &text);
                    let end = line_index.position_at_utf16(o.span.end(), &text);
                    serde_json::json!({
                        "code": o.code,
                        "message": o.message,
                        "startLine": start.line,
                        "startCharacter": start.character,
                        "endLine": end.line,
                        "endCharacter": end.character,
                        "replacement": o.replacement,
                        "group": o.group,
                        "hintOnly": o.hint_only,
                    })
                })
                .collect();
            serde_json::json!({
                "source": source,
                "optimisations": items,
            })
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("optimise worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(value))
    }

    /// Handle the `tcl-lsp.unminifyError` workspace command.
    ///
    /// Arguments: `[errorMessage, symbolMapText, minifiedSource?,
    /// originalSource?]`.  Translates compacted names in the error
    /// message back to originals via the symbol map and returns
    /// `{originalError, translatedError, changed}`.  Source-
    /// correlated line remapping (the `minified` / `original`
    /// arguments) is a follow-up.
    fn unminify_error_command(args: &[serde_json::Value]) -> Option<serde_json::Value> {
        let error_message = args.first().and_then(serde_json::Value::as_str)?;
        let symbol_map_text = args
            .get(1)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let symbol_map = core_minify::SymbolMap::parse(symbol_map_text);
        let mut translated = core_minify::unminify_error(error_message, &symbol_map);
        // When both the minified and original sources are supplied,
        // remap minified line references to approximate original lines.
        let minified = args.get(2).and_then(serde_json::Value::as_str);
        let original = args.get(3).and_then(serde_json::Value::as_str);
        if let (Some(minified), Some(original)) = (minified, original)
            && !minified.is_empty()
            && !original.is_empty()
        {
            translated = core_minify::remap_line_references(&translated, minified, original);
        }
        Some(serde_json::json!({
            "originalError": error_message,
            "translatedError": translated,
            "changed": translated != error_message,
        }))
    }

    /// Handle `tcl-lsp.describeIruleEvent`: deterministic registry metadata
    /// for an iRules event.  Mirrors `server/commands.py::on_describe_irule_event`.
    /// `validCommandCount` counts the iRules commands (those carrying
    /// `event_requires`) not excluded from the event.
    async fn describe_irule_event_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let event = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_owned();
        let events = tcl_registry::events::EventRegistry::build();
        let known = !event.is_empty() && events.is_known(&event);
        let (count, sample) = if known {
            let registry = self.registry_for_dialect("f5-irules").await;
            let profiles = tcl_registry::profiles::ProfileRegistry::build();
            // Mirror Python `_build_event_set`: every f5-irules command valid
            // in the event, not just those that carry `event_requires`.
            let names = registry.valid_irules_commands_for_event(&event, &events, &profiles);
            let count = names.len();
            let sample: Vec<String> = names.into_iter().take(80).map(str::to_owned).collect();
            (count, sample)
        } else {
            (0, Vec::new())
        };
        // The contract derives `deprecated` from the `when` event-completion
        // detail (`get_event_detail`), which never contains the word — so the
        // describe-event surface always reports false, deliberately *not*
        // `EventProps.deprecated` (the orthogonal F5 deprecation fact).  See
        // `server/commands.py::on_describe_irule_event`.
        let deprecated = false;
        Ok(Some(serde_json::json!({
            "event": event,
            "known": known,
            "deprecated": deprecated,
            "validCommandCount": count,
            "sampleCommands": sample,
        })))
    }

    /// Handle `tcl-lsp.describeIruleCommand`: registry metadata for an iRules
    /// command (exact match, then case-insensitive).  Mirrors
    /// `server/commands.py::on_describe_irule_command`.
    async fn describe_irule_command_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let name = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_owned();
        let registry = self.registry_for_dialect("f5-irules").await;
        let resolved = if !name.is_empty() && registry.get(&name).is_some() {
            Some(name.clone())
        } else if name.is_empty() {
            None
        } else {
            let lowered = name.to_lowercase();
            registry
                .command_names()
                .find(|c| c.to_lowercase() == lowered)
                .map(str::to_owned)
        };
        let value = match resolved {
            Some(canonical) => {
                let summary = registry
                    .get(&canonical)
                    .and_then(|s| s.hover.as_ref())
                    .map(|h| h.summary.to_owned());
                serde_json::json!({
                    "found": true,
                    "command": canonical,
                    "summary": summary,
                })
            }
            None => serde_json::json!({ "found": false, "command": name }),
        };
        Ok(Some(value))
    }

    /// Handle `tcl-lsp.listIruleEvents`: all known iRules event names (sorted).
    /// Mirrors `server/commands.py::on_list_irule_events`.
    fn list_irule_events_command() -> serde_json::Value {
        let events = tcl_registry::events::EventRegistry::build();
        let mut names: Vec<&str> = events.all_event_names();
        names.sort_unstable();
        serde_json::json!({ "events": names })
    }

    /// Handle `tcl-lsp.diagramData`: extract the `when EVENT` event names from
    /// a source string.  Mirrors `server/commands.py::on_diagram_data`'s event
    /// extraction (the events slice the e2e test asserts).
    fn diagram_data_command(args: &[serde_json::Value]) -> Option<serde_json::Value> {
        let source = args.first().and_then(serde_json::Value::as_str)?;
        let events: Vec<serde_json::Value> =
            tcl_lsp_core::irules_context::scan_file_events(source, "f5-irules")
                .into_iter()
                .map(|name| serde_json::json!({ "name": name }))
                .collect();
        Some(serde_json::json!({ "events": events }))
    }

    /// Handle `tcl-lsp.fixAllSafeIssues`: apply every non-overlapping safe
    /// diagnostic fix iteratively until the source stabilises.  Mirrors
    /// `server/commands.py::on_fix_all_safe_issues` (the `_SAFE_FIX_CODES`
    /// whitelist + multi-pass apply).
    async fn fix_all_safe_issues_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Url::parse(uri_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let (disabled, na_mode) = self.analyser_config().await;
        let dialect = doc.dialect.clone();
        let mut source = doc.text.clone();
        let value = tokio::task::spawn_blocking(move || {
            const SAFE: &[&str] = &["W100", "W105", "W108", "W110", "W201", "W304", "IRULE2001"];
            let mut applied: Vec<serde_json::Value> = Vec::new();
            for _ in 0..4 {
                let mut analyser = Self::configured_analyser(disabled.clone(), na_mode);
                let analysis = analyser.analyse(&source, &dialect).clone();
                // First fix per safe diagnostic, sorted by start offset.
                let mut fixes: Vec<(u32, u32, String, String, String)> = Vec::new();
                for d in &analysis.diagnostics {
                    if !SAFE.contains(&d.code.as_str()) {
                        continue;
                    }
                    if let Some(f) = d.fixes.first() {
                        fixes.push((
                            f.span.start(),
                            f.span.end(),
                            f.new_text.clone(),
                            d.code.clone(),
                            f.description.clone(),
                        ));
                    }
                }
                fixes.sort_by_key(|f| f.0);
                // Keep non-overlapping fixes in order.
                let mut chosen: Vec<(u32, u32, String, String, String)> = Vec::new();
                let mut last_end = 0u32;
                for f in fixes {
                    if f.0 >= last_end {
                        last_end = f.1;
                        chosen.push(f);
                    }
                }
                if chosen.is_empty() {
                    break;
                }
                // Apply from the end so earlier byte offsets stay valid.
                let mut new_source = source.clone();
                for f in chosen.iter().rev() {
                    let (s, e) = (f.0 as usize, f.1 as usize);
                    if s <= e && e <= new_source.len() {
                        new_source.replace_range(s..e, &f.2);
                    }
                }
                if new_source == source {
                    break;
                }
                for f in &chosen {
                    applied.push(serde_json::json!({ "code": f.3, "description": f.4 }));
                }
                source = new_source;
            }
            serde_json::json!({ "source": source, "applied": applied })
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("fix-all worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(value))
    }

    /// Handle `tcl-lsp.getEffectiveConfig`: the resolved per-document config —
    /// active dialect, the resolved `features` toggle map, the optimiser
    /// switch, line length, and analyser settings.  Mirrors
    /// `server/commands.py::on_get_effective_config`.  Tests poll this command
    /// after a `tclLsp.features.X = false` config change to confirm the server
    /// has applied the toggle without sleeping on wall-clock time.
    async fn get_effective_config_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let uri_str = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        // Resolve the dialect via the open document when the URI names one,
        // else the session default — never `null`, so a polling client always
        // sees a concrete value.
        let dialect = match Url::parse(uri_str) {
            Ok(uri) => match self.read_document(&uri).await {
                Some(doc) => doc.dialect,
                None => self.default_dialect.lock().await.clone(),
            },
            Err(_) => self.default_dialect.lock().await.clone(),
        };
        let features = self.feature_toggles.lock().await.resolved_map();
        let optimiser_enabled = *self.optimiser_enabled.lock().await;
        let line_length = *self.line_length.lock().await;
        let (disabled, mode) = self.analyser_config().await;
        let mut disabled_sorted: Vec<String> = disabled.into_iter().collect();
        disabled_sorted.sort();
        Ok(Some(serde_json::json!({
            "uri": uri_str,
            "dialect": dialect,
            "features": features,
            "optimiser_enabled": optimiser_enabled,
            "line_length": line_length,
            "non_ascii_mode": non_ascii_mode_str(mode),
            "disabled_diagnostics": disabled_sorted,
        })))
    }

    /// Pull the `tclLsp` configuration section from the client and apply it.
    ///
    /// The editor (and the e2e harness) answer `workspace/configuration` with
    /// the resolved `tclLsp` settings; a bare `didChangeConfiguration` with an
    /// empty payload is the signal to re-pull.  Applies `features.*`, the
    /// optimiser switch, line length, dialect, and the analyser knobs (W108
    /// mode + disabled diagnostics) — only the keys the reply carries, so
    /// omitted keys keep their last-applied value.
    async fn pull_and_apply_config(&self) {
        let items = vec![ConfigurationItem {
            scope_uri: None,
            section: Some("tclLsp".to_owned()),
        }];
        let Ok(values) = self.client.configuration(items).await else {
            return;
        };
        let Some(cfg) = values.into_iter().next() else {
            return;
        };
        if !cfg.is_object() {
            return;
        }
        if let Some(features) = cfg.get("features").and_then(serde_json::Value::as_object) {
            self.feature_toggles.lock().await.apply(features);
        }
        // `tclLsp.xcDiagnostics.enabled` is a dedicated config section (the
        // shipped VS Code setting "XC Migration: Enabled"), not a `features.*`
        // key, so it must be mapped onto the `xcDiagnostics` feature toggle
        // here — mirroring Python `settings.py`'s `xcDiagnostics` handling.
        if let Some(flag) = cfg
            .get("xcDiagnostics")
            .and_then(|x| x.get("enabled"))
            .and_then(serde_json::Value::as_bool)
        {
            self.feature_toggles
                .lock()
                .await
                .set_flag("xcDiagnostics", flag);
        }
        if let Some(flag) = cfg
            .get("optimiser")
            .and_then(|o| o.get("enabled"))
            .and_then(serde_json::Value::as_bool)
        {
            *self.optimiser_enabled.lock().await = flag;
        }
        if let Some(profile) = cfg
            .get("optimiser")
            .and_then(|o| o.get("profile"))
            .and_then(serde_json::Value::as_str)
        {
            *self.optimiser_profile.lock().await =
                tcl_compiler::optimiser::profiles::OptimisationProfile::parse(profile);
        }
        // Per-code overrides: any `optimiser.<CODE>` boolean other than the
        // `enabled` / `profile` keys force-enables (true) or force-disables
        // (false) that O-code on top of the profile. Mirrors Python `settings.py`.
        if let Some(opt) = cfg.get("optimiser").and_then(serde_json::Value::as_object) {
            let mut overrides = self.optimiser_code_overrides.lock().await;
            for (key, val) in opt {
                if key == "enabled" || key == "profile" {
                    continue;
                }
                if let Some(b) = val.as_bool() {
                    overrides.insert(key.clone(), b);
                }
            }
        }
        if let Some(len) = cfg
            .get("formatting")
            .and_then(|f| f.get("lineLength"))
            .or_else(|| cfg.get("lineLength"))
            .and_then(serde_json::Value::as_u64)
        {
            *self.line_length.lock().await = u32::try_from(len).unwrap_or(80);
        }
        if let Some(dialect) = cfg.get("dialect").and_then(serde_json::Value::as_str) {
            *self.default_dialect.lock().await = dialect.to_owned();
        }
        // The pulled value is the *content* of the `tclLsp` section; the
        // `settings_*` helpers expect it wrapped (they look under `tclLsp`),
        // so re-wrap before reusing them for the W108 mode + disabled codes.
        let wrapped = serde_json::json!({ "tclLsp": cfg });
        if let Some(mode) = settings_non_ascii_mode(&wrapped) {
            *self.non_ascii_mode.lock().await = mode;
        }
        if let Some(disabled) = settings_disabled_diagnostics(&wrapped) {
            *self.disabled_diagnostics.lock().await = disabled;
        }
        // Mirror the applied analyser knobs onto the salsa config input so the
        // query graph recomputes against the latest settings.
        self.sync_db_config().await;
        // Per-folder editor configuration: VS Code resolves `tclLsp` settings
        // per scope, so pull each folder's resolved config and store it for
        // longest-prefix resolution at read time.  Mirrors the Python
        // `editor_config_settings_per_folder`.  A single-root / no-folder
        // session skips this — the global pull above is the whole story.
        let folders = self.workspace_folder_urls().await;
        if !folders.is_empty() {
            let items: Vec<ConfigurationItem> = folders
                .iter()
                .map(|f| ConfigurationItem {
                    scope_uri: Some(f.clone()),
                    section: Some("tclLsp".to_owned()),
                })
                .collect();
            if let Ok(values) = self.client.configuration(items).await {
                let parsed: Vec<(Url, FolderConfig)> = folders
                    .into_iter()
                    .zip(values)
                    .filter_map(|(folder, cfg)| parse_folder_config(&cfg).map(|fc| (folder, fc)))
                    .collect();
                self.apply_folder_configs(parsed).await;
            }
        }
    }

    /// Store the per-folder editor configs and refresh the per-folder salsa
    /// `AnalyserConfig` handles.  A handle is created only for a folder that
    /// overrides the disabled-diagnostics set or non-ASCII mode (others inherit
    /// [`Backend::db_config`]); existing handles are reused across re-pulls so
    /// the salsa store does not accumulate dead config inputs.
    async fn apply_folder_configs(&self, parsed: Vec<(Url, FolderConfig)>) {
        use salsa::Setter as _;
        {
            let mut global_disabled: Vec<String> = self
                .disabled_diagnostics
                .lock()
                .await
                .iter()
                .cloned()
                .collect();
            global_disabled.sort();
            let global_mode = *self.non_ascii_mode.lock().await;
            let mut db = self.db.lock().await;
            let mut handles = self.folder_db_configs.lock().await;
            let mut next: Vec<(Url, tcl_lsp_db::AnalyserConfig)> = Vec::new();
            for (folder, fc) in &parsed {
                if fc.disabled_diagnostics.is_none() && fc.non_ascii_mode.is_none() {
                    continue;
                }
                let mut disabled: Vec<String> = match &fc.disabled_diagnostics {
                    Some(d) => d.iter().cloned().collect(),
                    None => global_disabled.clone(),
                };
                disabled.sort();
                let mode = fc.non_ascii_mode.unwrap_or(global_mode);
                if let Some((_, handle)) = handles.iter().find(|(u, _)| u == folder) {
                    handle.set_disabled_diagnostics(&mut *db).to(disabled);
                    handle.set_non_ascii_mode(&mut *db).to(mode);
                    next.push((folder.clone(), *handle));
                } else {
                    let handle = tcl_lsp_db::AnalyserConfig::new(&*db, disabled, mode);
                    next.push((folder.clone(), handle));
                }
            }
            *handles = next;
        }
        *self.folder_configs.lock().await = parsed;
    }

    /// Whether the named `tclLsp.features.*` provider is enabled.
    async fn feature_enabled(&self, feature: &str, uri: &Url) -> bool {
        // A folder that explicitly sets the toggle wins for documents under it;
        // otherwise fall back to the process-global toggle state.
        {
            let configs = self.folder_configs.lock().await;
            if let Some(fc) = longest_folder_match(&configs, uri)
                && let Some(flag) = fc.feature_toggles.set.get(feature).copied()
            {
                return flag;
            }
        }
        self.feature_toggles.lock().await.is_enabled(feature)
    }

    /// Whether opt-in format-on-save (`tclLsp.features.willSaveWaitUntil`)
    /// is enabled for the document at `uri`.  Resolves per folder like
    /// [`Self::feature_enabled`] so a folder-scoped opt-in works in a
    /// multi-root workspace, but — unlike the default-on feature toggles — it
    /// defaults **off** to match the Python server, so it cannot reuse
    /// `feature_enabled` (whose absent-key fallback is `true`).
    async fn will_save_format_enabled(&self, uri: &Url) -> bool {
        {
            let configs = self.folder_configs.lock().await;
            if let Some(fc) = longest_folder_match(&configs, uri)
                && let Some(flag) = fc.feature_toggles.set.get("willSaveWaitUntil").copied()
            {
                return flag;
            }
        }
        self.feature_toggles
            .lock()
            .await
            .is_enabled_default_off("willSaveWaitUntil")
    }

    /// Whether the given opt-in inlay family is enabled for `uri`.
    ///
    /// Inlay hints are opt-in and default **off**, matching the Python
    /// server's default-off contract (PR #643).  The two families —
    /// `inlayTypeHints` (inferred variable types + format-string specifier
    /// labels) and `inlayParameterHints` (call-site parameter-name labels)
    /// — gate independently; the retired `inlayHints` key is normalised to
    /// `inlayTypeHints` on input (see [`FeatureToggles::apply`]).  Resolves
    /// per folder like [`Self::feature_enabled`] so a folder-scoped opt-in
    /// works in a multi-root workspace.
    async fn inlay_family_enabled(&self, uri: &Url, family: &str) -> bool {
        {
            let configs = self.folder_configs.lock().await;
            if let Some(fc) = longest_folder_match(&configs, uri)
                && let Some(flag) = fc.feature_toggles.set.get(family).copied()
            {
                return flag;
            }
        }
        self.feature_toggles
            .lock()
            .await
            .is_enabled_default_off(family)
    }

    /// Whether the opt-in XC100-301 translatability diagnostics
    /// (`xcDiagnostics`) are enabled for `uri`.  Default **off**, resolved
    /// per folder like [`Self::inlay_family_enabled`], mirroring the Python
    /// server's `xc_diagnostics_enabled` gate.  Surfaced only on
    /// `f5-irules` documents (the only dialect the `f5-xc` translator runs
    /// on).
    async fn xc_diagnostics_enabled(&self, uri: &Url) -> bool {
        self.inlay_family_enabled(uri, "xcDiagnostics").await
    }

    /// Handle `tcl-lsp.listSubcommands`: subcommand metadata for `command`
    /// from the registry.  Mirrors `server/commands.py::on_list_subcommands`.
    /// An unknown command (or one with no subcommands) yields an empty list.
    async fn list_subcommands_command(
        &self,
        args: &[serde_json::Value],
    ) -> Option<serde_json::Value> {
        let name = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let dialect = self.default_dialect.lock().await.clone();
        let registry = self.registry_for_dialect(&dialect).await;
        let parsed = DialectSet::parse(&dialect).unwrap_or(DialectSet::ALL_TCL);
        let mut subs: Vec<serde_json::Value> = registry
            .get_for_dialect(&name, parsed)
            .map(|spec| {
                spec.subcommands
                    .iter()
                    .map(|sub| {
                        serde_json::json!({
                            "name": sub.name,
                            "detail": sub.detail,
                            "synopsis": sub.synopsis,
                            "pure": sub.pure,
                            "mutator": sub.mutator,
                            // The registry tracks deprecation at command level,
                            // not per subcommand — report the shape with a
                            // conservative default.
                            "deprecated": false,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        subs.sort_by(|a, b| {
            a.get("name")
                .and_then(serde_json::Value::as_str)
                .cmp(&b.get("name").and_then(serde_json::Value::as_str))
        });
        Some(serde_json::json!({ "command": name, "subcommands": subs }))
    }

    /// Handle `tcl-lsp.listKnownPackages`.  The native server has no on-disk
    /// `package require` resolver yet (a follow-up), so it reports the empty
    /// set — the contract is a `packages` list, which downstream callers can
    /// rely on regardless of population.
    fn list_known_packages_command() -> serde_json::Value {
        serde_json::json!({ "packages": serde_json::Value::Array(vec![]) })
    }

    /// Handle `tcl-lsp.suggestPackagesForSymbol`.  Echoes the symbol and an
    /// (empty, pending the package resolver) suggestion list.
    fn suggest_packages_for_symbol_command(args: &[serde_json::Value]) -> serde_json::Value {
        let symbol = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_owned();
        serde_json::json!({ "symbol": symbol, "suggestions": serde_json::Value::Array(vec![]) })
    }

    /// Handle `tcl-lsp.exportConfig`: write the resolved settings to the
    /// user config file (`$XDG_CONFIG_HOME/tcl-lsp/config.ini`, else
    /// `~/.config/tcl-lsp/config.ini`) and return its path.  Mirrors
    /// `server/commands.py::on_export_config`; only the resolved
    /// feature / optimiser / style / disabled-diagnostic state is written.
    async fn export_config_command(&self) -> serde_json::Value {
        let path = config_ini_path();
        let contents = self.render_config_ini().await;
        let result = path.parent().map_or_else(
            || Err(std::io::Error::other("no config directory")),
            std::fs::create_dir_all,
        );
        match result.and_then(|()| std::fs::write(&path, contents)) {
            Ok(()) => serde_json::json!({
                "success": true,
                "path": path.to_string_lossy(),
            }),
            Err(err) => serde_json::json!({
                "success": false,
                "error": err.to_string(),
            }),
        }
    }

    /// Render the resolved config as an INI document (the format the server
    /// reads from `config.ini`): `[features]`, `[optimiser]`, `[style]`, and
    /// the disabled `[diagnostics]` codes.
    async fn render_config_ini(&self) -> String {
        use std::fmt::Write as _;
        let features = self.feature_toggles.lock().await.resolved_map();
        let optimiser_enabled = *self.optimiser_enabled.lock().await;
        let line_length = *self.line_length.lock().await;
        let mut disabled: Vec<String> = self
            .disabled_diagnostics
            .lock()
            .await
            .iter()
            .cloned()
            .collect();
        disabled.sort();

        let mut out = String::from("# tcl-lsp configuration (exported)\n\n[features]\n");
        let mut feature_keys: Vec<&String> = features.keys().collect();
        feature_keys.sort();
        for key in feature_keys {
            let enabled = features.get(key).and_then(serde_json::Value::as_bool) == Some(true);
            let _ = writeln!(out, "{key} = {enabled}");
        }
        let _ = write!(out, "\n[optimiser]\nenabled = {optimiser_enabled}\n");
        let _ = write!(out, "\n[style]\nlineLength = {line_length}\n");
        if !disabled.is_empty() {
            out.push_str("\n[diagnostics]\n");
            for code in disabled {
                let _ = writeln!(out, "{code} = false");
            }
        }
        out
    }

    /// Snapshot the user-configured analyser settings (disabled
    /// diagnostic codes + W108 non-ASCII mode) so a blocking analysis
    /// worker can build its `Analyser` without holding any async mutex.
    async fn analyser_config(&self) -> (HashSet<String>, NonAsciiMode) {
        let disabled = self.disabled_diagnostics.lock().await.clone();
        let mode = *self.non_ascii_mode.lock().await;
        (disabled, mode)
    }

    /// Resolve the diagnostics-affecting settings for `uri`: a per-folder editor
    /// config (longest-prefix match) overrides the process-global fields
    /// field-by-field; unset folder fields inherit the global value.  Returns
    /// `(disabled_diagnostics, non_ascii_mode, optimiser_enabled,
    /// optimiser_disabled_codes)`.  In a single-root workspace (no per-folder
    /// configs) this is exactly the global state.
    async fn resolved_analysis_settings(
        &self,
        uri: &Url,
    ) -> (HashSet<String>, NonAsciiMode, bool, HashSet<String>) {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).cloned()
        };
        let disabled = match folder.as_ref().and_then(|f| f.disabled_diagnostics.clone()) {
            Some(d) => d,
            None => self.disabled_diagnostics.lock().await.clone(),
        };
        let non_ascii_mode = match folder.as_ref().and_then(|f| f.non_ascii_mode) {
            Some(m) => m,
            None => *self.non_ascii_mode.lock().await,
        };
        let optimiser_enabled = match folder.as_ref().and_then(|f| f.optimiser_enabled) {
            Some(b) => b,
            None => *self.optimiser_enabled.lock().await,
        };
        let profile = match folder.as_ref().and_then(|f| f.optimiser_profile) {
            Some(p) => p,
            None => *self.optimiser_profile.lock().await,
        };
        let mut opt_disabled: HashSet<String> =
            tcl_compiler::optimiser::profiles::profile_to_disabled(profile)
                .into_iter()
                .map(str::to_owned)
                .collect();
        // Global per-code overrides first, then the folder's (folder wins).
        for (code, enabled) in self.optimiser_code_overrides.lock().await.iter() {
            if *enabled {
                opt_disabled.remove(code);
            } else {
                opt_disabled.insert(code.clone());
            }
        }
        if let Some(f) = folder.as_ref() {
            for (code, enabled) in &f.optimiser_code_overrides {
                if *enabled {
                    opt_disabled.remove(code);
                } else {
                    opt_disabled.insert(code.clone());
                }
            }
        }
        (disabled, non_ascii_mode, optimiser_enabled, opt_disabled)
    }

    /// Resolve the formatter line length for `uri` (per-folder override, else
    /// the global `tclLsp.formatting.lineLength`).
    async fn resolved_line_length(&self, uri: &Url) -> u32 {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).and_then(|f| f.line_length)
        };
        match folder {
            Some(len) => len,
            None => *self.line_length.lock().await,
        }
    }

    /// Build an `Analyser` carrying the configured disabled-diagnostics
    /// set and W108 mode.
    fn configured_analyser(disabled: HashSet<String>, mode: NonAsciiMode) -> Analyser {
        Analyser::with_disabled_diagnostics(disabled).with_non_ascii_mode(mode)
    }

    /// Return an `Arc<CommandRegistry>` with `dialect` loaded on
    /// top of the default Tcl + stdlib + tcllib specs.
    ///
    /// The result is cached per canonical dialect key. Concurrent
    /// first-use requests may build the same registry more than once,
    /// but only one `Arc` is inserted; keeping construction outside the
    /// mutex avoids blocking unrelated cache hits on spec loading.
    /// Unparseable dialect strings collapse to the empty-string key so
    /// they share a single cached "plain Tcl" registry rather than
    /// leaking a fresh allocation per typo.
    async fn registry_for_dialect(&self, dialect: &str) -> Arc<CommandRegistry> {
        // The dialect-loaded registry lives on the query database as a durable
        // (non-salsa) value, built once per canonical dialect key and shared.
        // Clone the db handle (cheap; shares the registry map) so the lookup /
        // one-time build doesn't hold the db mutex.
        let db = self.db.lock().await.clone();
        db.registry(dialect)
    }

    /// Run the analyser on `text` and push the resulting
    /// diagnostics to the LSP client for `uri`.  Mirrors
    /// the Python server's `publish_diagnostics` flow.  Runs
    /// the analyser on a `spawn_blocking` worker so the LSP
    /// event loop stays responsive.  `S-async-diagnostics`
    /// minimal port — the cached-analysis surface and
    /// debounced streaming contract lands in a follow-up.
    /// Gather the document-independent handles a detached diagnostics run needs
    /// (per-edit state travels in a [`DiagJob`]).
    async fn diag_inputs(&self, uri: &Url, dialect: &str) -> DiagInputs {
        let (disabled, non_ascii_mode, optimiser_enabled, opt_disabled) =
            self.resolved_analysis_settings(uri).await;
        let registry = self.registry_for_dialect(dialect).await;
        let xc_diagnostics = self.xc_diagnostics_enabled(uri).await;
        DiagInputs {
            client: self.client.clone(),
            registry,
            disabled,
            non_ascii_mode,
            optimiser_enabled,
            opt_disabled,
            documents: Arc::clone(&self.documents),
            workspace_index: Arc::clone(&self.workspace_index),
            db: Arc::clone(&self.db),
            db_files: Arc::clone(&self.db_files),
            db_project: Arc::clone(&self.db_project),
            db_config: Arc::clone(&self.db_config),
            folder_db_configs: Arc::clone(&self.folder_db_configs),
            pull_diag_cache: Arc::clone(&self.pull_diag_cache),
            xc_diagnostics,
        }
    }

    /// Build the **complete** diagnostic set for a document — analyser +
    /// compiler/optimiser + source-style — exactly as the push path
    /// ([`run_diagnostics_core`]) assembles it. The pull handler
    /// (`textDocument/diagnostic`) uses this so an editor on the pull path is
    /// not silently missing the compiler, optimiser, and source-style
    /// warnings that `publish_diagnostics` adds (the capability is advertised
    /// unconditionally, so `vscode-languageclient` and peers may choose pull).
    ///
    /// Compiler checks run **uncached** here: pull requests are on demand
    /// (hover/save), not per-keystroke, so they need no salsa file handle, and
    /// the analyser result still comes through the shared [`Self::analysis_for`]
    /// cache.
    async fn full_diagnostics_for(
        &self,
        uri: &Url,
        text: String,
        dialect: String,
        language_id: &str,
    ) -> Vec<tower_lsp::lsp_types::Diagnostic> {
        let (disabled, _non_ascii_mode, optimiser_enabled, opt_disabled) =
            self.resolved_analysis_settings(uri).await;
        // F5 dialect dispatch (FE-DIAG-F5): BIG-IP config / iApp APL
        // presentation documents have model-level validators, not the Tcl
        // analyser — mirror the push path's dispatch so a pull-mode editor
        // receives the same BIGIP/IAPP diagnostics.
        if let Some(diags) = f5_dialect_diagnostics(
            uri,
            &text,
            &dialect,
            language_id,
            &disabled,
            &self.documents,
        )
        .await
        {
            return diags;
        }
        let analysis = self.analysis_for(uri, text.clone(), dialect.clone()).await;
        let registry = self.registry_for_dialect(&dialect).await;

        // Cross-file (SRV-INCREMENTAL Task 6): the project's proc arities, so the
        // pull path resolves cross-file W123 / emits cross-file W124 exactly as the
        // push path does.  Only gathered when `xcDiagnostics` is enabled.
        let xc_on = self.xc_diagnostics_enabled(uri).await;
        let project_arities = if xc_on {
            let db = self.db.lock().await.clone();
            (*self.db_project.lock().await).map(|p| tcl_lsp_db::project_command_arities(&db, p))
        } else {
            None
        };

        let compiler_diags = {
            let (c_text, c_dialect, c_registry) =
                (text.clone(), dialect.clone(), Arc::clone(&registry));
            tokio::task::spawn_blocking(move || {
                tcl_lsp_db::compiler_check_diagnostics_uncached(&c_text, &c_registry, &c_dialect)
            })
            .await
            .unwrap_or_else(|_| tcl_lsp_db::CompilerDiagnostics {
                checks: Vec::new(),
                optimisations: Vec::new(),
            })
        };

        let xc_for_irules = dialect == "f5-irules" && xc_on;
        // Cross-file resolution (Task 6) — W123 suppression + W124 arity, matching
        // the push path.
        let analyser_diags = match &project_arities {
            Some(arities) => tcl_lsp_db::apply_cross_file_resolution(
                &analysis.diagnostics,
                &analysis.command_invocations,
                arities,
                disabled.contains("W124"),
            ),
            None => analysis.diagnostics.clone(),
        };
        tokio::task::spawn_blocking(move || {
            let mut diagnostics = lift_analyser_diagnostics(&text, &analyser_diags);
            diagnostics.extend(lift_compiler_diagnostics(
                &text,
                &compiler_diags,
                optimiser_enabled,
                &opt_disabled,
                &disabled,
            ));
            diagnostics.extend(lift_source_style_diagnostics(
                &text,
                &analysis.suppressed_lines,
                &disabled,
            ));
            // FE-DIAG-F5 opt-in: XC100-301 translatability diagnostics for
            // `f5-irules` documents when `xcDiagnostics` is enabled (mirrors
            // the push path).
            if xc_for_irules {
                diagnostics.extend(lift_xc_diagnostics(
                    &text,
                    &disabled,
                    &analysis.suppressed_lines,
                ));
            }
            diagnostics
        })
        .await
        .unwrap_or_default()
    }

    /// Compute and publish diagnostics synchronously (awaited).  Used by the
    /// on-close/disk paths and tests; the interactive open/change paths use the
    /// detached [`Self::schedule_diagnostics`] instead so they never block the
    /// LSP message loop.
    #[cfg_attr(not(test), allow(dead_code))]
    async fn publish_analyser_diagnostics(
        &self,
        uri: Url,
        _text: String,
        dialect: String,
        _revision: u64,
        _version: Option<i32>,
    ) {
        let inputs = self.diag_inputs(&uri, &dialect).await;
        let Some(job) = inputs.capture_job(&uri).await else {
            return;
        };
        run_diagnostics_core(inputs, &uri, job).await;
    }

    /// Schedule a debounced, detached diagnostics run for `uri`.  Returns
    /// immediately (the analyser work runs on a `tokio::spawn`ed task), so
    /// `did_open` / `did_change` never block the message loop on analysis.
    ///
    /// Coalescing per-URI worker: the edit marks the document dirty and, if no
    /// worker is already draining `uri`, starts one.  The worker debounces, then
    /// captures and analyses the document's **current** state (so a burst — even
    /// one whose `didChange` notifications are processed out of order — settles
    /// on the latest committed version), retrying if a run is cancelled
    /// mid-flight by a concurrent `set_text`.  This guarantees the final
    /// version's diagnostics are always published, even under a heavy edit burst
    /// on a loaded machine, and never runs two analyses for one document
    /// concurrently (so an edit's write is never blocked behind a stale read).
    async fn schedule_diagnostics(&self, uri: Url, dialect: String) {
        let start_worker = {
            let mut slots = self.diag_slots.lock().await;
            let slot = slots.entry(uri.clone()).or_default();
            slot.dirty = true;
            if slot.running {
                false
            } else {
                slot.running = true;
                true
            }
        };
        if !start_worker {
            return;
        }
        let inputs = self.diag_inputs(&uri, &dialect).await;
        let slots = Arc::clone(&self.diag_slots);
        tokio::spawn(async move {
            loop {
                // Debounce, then claim the dirty flag.  A burst collapses to one
                // run; with nothing dirty we retire the worker (under the lock,
                // so a concurrent edit either sees `running` and skips the spawn
                // while we run, or sees `!running` and starts a fresh one).
                tokio::time::sleep(DIAGNOSTICS_DEBOUNCE).await;
                {
                    let mut guard = slots.lock().await;
                    let Some(slot) = guard.get_mut(&uri) else {
                        return;
                    };
                    if slot.dirty {
                        slot.dirty = false;
                    } else {
                        slot.running = false;
                        return;
                    }
                }
                // Capture + analyse the document's current state.  If it is gone
                // (closed) there is nothing to publish — retire.
                let Some(job) = inputs.capture_job(&uri).await else {
                    let mut guard = slots.lock().await;
                    if let Some(slot) = guard.get_mut(&uri) {
                        slot.running = false;
                    }
                    return;
                };
                let settled = run_diagnostics_core(inputs.clone(), &uri, job).await;
                if !settled {
                    // Cancelled mid-flight — re-mark dirty so we retry the latest
                    // state next loop.
                    if let Some(slot) = slots.lock().await.get_mut(&uri) {
                        slot.dirty = true;
                    }
                }
            }
        });
    }

    /// Best-effort dynamic registration of
    /// `workspace/didChangeWatchedFiles` for Tcl source files, so external
    /// on-disk changes reach [`LanguageServer::did_change_watched_files`].
    /// The glob mirrors the Python server's watched-file pattern.  A client
    /// without dynamic-registration support rejects the request; that is
    /// logged and ignored (the startup scan still seeds the index).
    async fn register_file_watchers(&self) {
        let registration = Registration {
            id: "tcl-lsp-watched-files".to_owned(),
            method: "workspace/didChangeWatchedFiles".to_owned(),
            register_options: serde_json::to_value(DidChangeWatchedFilesRegistrationOptions {
                watchers: vec![FileSystemWatcher {
                    glob_pattern: GlobPattern::String("**/*.{tcl,tm,itcl,irule,irul}".to_owned()),
                    kind: Some(WatchKind::Create | WatchKind::Change | WatchKind::Delete),
                }],
            })
            .ok(),
        };
        if let Err(err) = self.client.register_capability(vec![registration]).await {
            self.client
                .log_message(
                    MessageType::LOG,
                    format!("file-watcher registration declined by client: {err}"),
                )
                .await;
        }
    }

    /// On-disk workspace-folder scan: analyse `.tcl` / `.tm`
    /// files in the workspace folders that the editor hasn't
    /// opened, merging their definitions / invocation sites into
    /// the cross-document workspace index.
    ///
    /// Editor-opened documents already feed the index through
    /// `publish_analyser_diagnostics`; this fills the gap for
    /// project files the user hasn't opened yet so cross-document
    /// features (references / rename / call-hierarchy /
    /// completion) see the whole project, not just open tabs.
    ///
    /// The directory walk, file reads and per-file analysis run on
    /// a `spawn_blocking` worker so the LSP event loop stays
    /// responsive.  URIs already present in `self.documents` are
    /// skipped so the on-disk copy never clobbers a live (and
    /// possibly unsaved) editor buffer.  The walk is capped at
    /// [`WORKSPACE_SCAN_FILE_CAP`] files so a large tree can't
    /// stall start-up.
    async fn scan_workspace_folders(&self) {
        let folders = self.workspace_folder_urls().await;
        let roots: Vec<PathBuf> = folders
            .iter()
            .filter_map(|f| f.to_file_path().ok())
            .collect();
        if roots.is_empty() {
            return;
        }
        // Snapshot the dialect-resolution inputs and the set of
        // open documents so the blocking worker can run without
        // touching any async mutex.
        let open: HashSet<Url> = self.documents.lock().await.keys().cloned().collect();
        let folder_dialects = self.folder_dialects.lock().await.clone();
        let default_dialect = self.default_dialect.lock().await.clone();

        let analysed = tokio::task::spawn_blocking(move || {
            let mut files: Vec<PathBuf> = Vec::new();
            for root in &roots {
                collect_tcl_files(root, WORKSPACE_SCAN_FILE_CAP, &mut files);
            }
            let mut out: Vec<(String, AnalysisResult)> = Vec::new();
            for path in files {
                let Ok(uri) = Url::from_file_path(&path) else {
                    continue;
                };
                if open.contains(&uri) {
                    continue;
                }
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let dialect = folder_dialect_for(&uri, &folder_dialects)
                    .unwrap_or_else(|| default_dialect.clone());
                let mut analyser = Analyser::new();
                let analysis = analyser.analyse(&text, &dialect).clone();
                out.push((uri.to_string(), analysis));
            }
            out
        })
        .await
        .unwrap_or_default();

        self.merge_workspace_scan_results(&analysed).await;
    }

    /// Merge disk-backed workspace scan results into the shared index.
    ///
    /// The blocking scan snapshots open documents before it starts, but
    /// an editor can open a file while the scan is still running. Recheck
    /// the live document map at publication time so stale on-disk
    /// analysis cannot overwrite the open-buffer entry.
    async fn merge_workspace_scan_results(&self, analysed: &[(String, AnalysisResult)]) {
        // Hold `documents` across the open-set read + index merge (the
        // `documents` → `workspace_index` order that replaced the global gate)
        // so a file opening mid-merge can't have its open-buffer index entry
        // overwritten by this disk-backed scan result.
        let docs = self.documents.lock().await;
        let open: HashSet<String> = docs.keys().map(ToString::to_string).collect();
        let mut index = self.workspace_index.lock().await;
        for (uri, analysis) in analysed {
            if open.contains(uri) {
                continue;
            }
            index.remove_document(uri);
            index.add_document(uri, analysis);
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        self.apply_workspace_folders(&params).await;
        self.apply_initialization_options(&params).await;
        let position_encoding = negotiate_position_encoding(&params);
        if client_lacks_utf16_support(&params) {
            // The client advertised position encodings without UTF-16. The
            // server emits UTF-16 columns only, so it proceeds in UTF-16
            // regardless (rather than failing the session over a baseline
            // every conformant client supports) — but warn so a genuine
            // UTF-8/UTF-32-only client's mis-mapped non-ASCII ranges are
            // diagnosable instead of silent.
            self.client
                .log_message(
                    MessageType::WARNING,
                    "client advertised position encodings without UTF-16; this server \
                     emits UTF-16 columns and will use UTF-16 regardless",
                )
                .await;
        }
        Ok(InitializeResult {
            capabilities: build_server_capabilities(position_encoding),
            server_info: Some(ServerInfo {
                // Protocol identity must match the Python server / editor
                // expectations ("tcl-lsp"), not the crate/binary name.
                name: "tcl-lsp".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "tcl-lsp-server initialised")
            .await;
        // Pull the resolved `tclLsp` config once the client is ready so
        // feature toggles / optimiser switch / analyser knobs are in effect
        // before the first request.
        self.pull_and_apply_config().await;
        // Ask the client to watch Tcl source files so external edits
        // (`git checkout`, generated files, deletions) reach
        // `did_change_watched_files` and refresh the cross-document index.
        // Best-effort: clients without dynamic-registration support reject
        // this, which is harmless — the startup scan still seeds the index.
        self.register_file_watchers().await;
        // Seed the cross-document index with on-disk project files
        // the editor hasn't opened yet.
        self.scan_workspace_folders().await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let dialect = self
            .dialect_for_open(&params.text_document.uri, &params.text_document.language_id)
            .await;
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        let version = params.text_document.version;
        let dialect_for_diags = dialect.clone();
        {
            // Hold `documents` across the salsa input set + the index removal
            // (the `documents` → `workspace_index` order that replaced the
            // global gate): a reader must never see the new `doc.text` with a
            // stale query result for the old text, and no diagnostics run may
            // re-add a stale index entry between them.
            let mut docs = self.documents.lock().await;
            docs.insert(
                uri.clone(),
                DocumentState::with_version(params.text_document.text, dialect, version)
                    .with_language_id(params.text_document.language_id.clone()),
            );
            self.db_set_source(&uri, text.clone(), dialect_for_diags.clone())
                .await;
            self.workspace_index
                .lock()
                .await
                .remove_document(uri.as_str());
            drop(docs);
        }
        self.schedule_diagnostics(uri, dialect_for_diags).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // INCREMENTAL sync: each content change is either a full-document
        // replacement (`range == None`) or a ranged edit applied to the
        // current text. Apply them in order. (The re-analysis below is
        // still whole-document; bounding it to `reparse_window` is a
        // documented follow-up — the primitives exist in `tcl-lexer`.)
        let uri = params.text_document.uri.clone();
        if params.content_changes.is_empty() {
            return;
        }
        let default_dialect = self.default_dialect.lock().await.clone();
        let change_version = params.text_document.version;
        let dialect = {
            let mut docs = self.documents.lock().await;
            let entry = docs
                .entry(uri.clone())
                // didChange before didOpen — start from empty text and the
                // session default dialect (no languageId is available here).
                .or_insert_with(|| DocumentState::new(String::new(), default_dialect));
            let mut text = std::mem::take(&mut entry.text);
            // Patch the persisted `LineIndex` alongside each splice instead of
            // rebuilding it per edit / per position lookup (SRV-INCREMENTAL
            // Task 1).  Take it out, patch through the edit sequence, put it back.
            let mut index = std::mem::replace(&mut entry.line_index, tcl_lexer::LineIndex::new(""));
            for change in &params.content_changes {
                text = apply_content_change_indexed(&text, change.range, &change.text, &mut index);
            }
            entry.text = text.clone();
            entry.line_index = index;
            entry.bump_revision(change_version);
            let dialect = entry.dialect.clone();
            // Update the salsa `SourceFile` input + the cross-document index
            // WHILE still holding the `documents` lock (the `documents` →
            // `workspace_index` order that replaced the global gate): no
            // concurrent request can observe the new `doc.text` (from
            // `read_document`) with a stale query result for the old text, and
            // no diagnostics run can re-add a stale index entry between the
            // commit and the removal.  (db_set_source locks db→db_files, never
            // documents, so this nesting introduces no lock-order cycle.)
            self.db_set_source(&uri, text, dialect.clone()).await;
            self.workspace_index
                .lock()
                .await
                .remove_document(uri.as_str());
            drop(docs);
            dialect
        };
        self.schedule_diagnostics(uri, dialect).await;
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        // Accept ``{"tclLsp": {"dialect": "<name>"}}`` in either the
        // VS Code-style nested shape or the flat ``{"dialect":
        // "<name>"}`` shape (used by the MCP bridge). Update the
        // session default so newly opened documents pick up the change,
        // then re-resolve every already-open document under the new
        // default and reschedule diagnostics for any whose dialect
        // changed — `tclLsp.selectDialect` pushes a dialect-only config
        // change for a buffer that is already open, and without this the
        // buffer would keep being parsed/diagnosed under the dialect it
        // was opened with until reopened. Mirrors the Python server's
        // `_apply_merged_settings_now`, which snapshots each open
        // document's resolved dialect and force-reanalyses the ones that
        // change.
        let dialect = params
            .settings
            .get("tclLsp")
            .and_then(|v| v.get("dialect"))
            .or_else(|| params.settings.get("dialect"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        if let Some(d) = dialect {
            let changed = {
                let mut default = self.default_dialect.lock().await;
                let was = std::mem::replace(&mut *default, d.clone());
                was != d
            };
            // A document's resolved dialect only depends on the session
            // default as a fallback (an explicit language id / BIG-IP
            // basename / folder override still wins), so nothing changes
            // when the default is unchanged.
            if changed {
                self.reresolve_open_document_dialects().await;
            }
        }
        // W108 mode + disabled-diagnostics reconfiguration. Existing
        // documents pick the change up on their next analyse (the
        // analyses cache is keyed per URI and refreshed on edit).
        if let Some(mode) = settings_non_ascii_mode(&params.settings) {
            *self.non_ascii_mode.lock().await = mode;
        }
        if let Some(disabled) = settings_disabled_diagnostics(&params.settings) {
            *self.disabled_diagnostics.lock().await = disabled;
        }
        // VS Code (and the e2e harness) push an empty/partial payload as a
        // signal to re-pull the full resolved config via
        // `workspace/configuration`.  Always re-pull so `features.*`, the
        // optimiser switch, and the analyser knobs reflect the latest editor
        // settings — the inline `params.settings` handling above covers the
        // flat MCP-bridge shape that carries the values directly.
        self.pull_and_apply_config().await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = &params.text_document.uri;
        {
            // Remove the live document + its index entry while holding
            // `documents` (the `documents` → `workspace_index` order that
            // replaced the global gate): an in-flight diagnostics run re-checks
            // currency under `documents` and, finding the doc gone, will not
            // re-add a stale index entry or publish for it.
            let mut docs = self.documents.lock().await;
            docs.remove(uri);
            self.workspace_index
                .lock()
                .await
                .remove_document(uri.as_str());
            drop(docs);
        }
        // Clear any previously-published diagnostics so a later didOpen for the
        // same URI starts clean.  Published outside the lock; a diagnostics run
        // for the now-closed doc no longer publishes (its currency re-check
        // fails), so this empty publish is the last word for the URI.
        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
        // Drop the pull-diagnostic cache entry so a `workspace/diagnostic`
        // sweep no longer reports the now-closed document and a later reopen
        // recomputes from scratch.
        self.pull_diag_cache.lock().await.remove(uri);
        self.db_remove_source(uri).await;
        // Re-index the file from disk rather than dropping it: the
        // file still exists on disk and was (or would be) part of
        // the on-disk index, so cross-document definition /
        // references / rename / call-hierarchy must keep seeing it
        // after the editor closes the buffer.  `scan_workspace_folders`
        // only runs at `initialized`, so a plain `remove_document`
        // here would make the file vanish until restart. The helper
        // rechecks that this URI is still closed before publishing
        // the disk-backed index entry.
        self.reindex_index_from_disk(uri).await;
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        // Update the tracked folder set, then reconcile the cross-document
        // index and configuration so multi-root behaviour is not frozen at the
        // `initialize` snapshot.  Mirrors the Python `didChangeWorkspaceFolders`
        // handler (drop removed-folder state, load/scan added folders, re-pull
        // config).
        let removed: Vec<Url> = params.event.removed.iter().map(|f| f.uri.clone()).collect();
        {
            let mut folders = self.workspace_folders.lock().await;
            for uri in &removed {
                folders.retain(|f| f != uri);
            }
            for added in &params.event.added {
                if !folders.iter().any(|f| f == &added.uri) {
                    folders.push(added.uri.clone());
                }
            }
        }
        // Drop index entries for files under removed folders so their symbols
        // stop resolving across the workspace.  Open documents are left alone —
        // their live buffer is the source of truth regardless of folder set.
        if !removed.is_empty() {
            self.drop_index_under_folders(&removed).await;
        }
        // Pick up files under any newly-added folder.  `scan_workspace_folders`
        // walks the *current* folder list and skips open documents, so it is
        // safe to call here.
        if !params.event.added.is_empty() {
            self.scan_workspace_folders().await;
        }
        // Per-folder config may differ between roots; re-pull so the resolved
        // settings reflect the new folder set.
        self.pull_and_apply_config().await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        // External (non-editor) file changes — `git checkout`, a generated
        // file, a deletion — must refresh the cross-document index so
        // definition / references / rename / call-hierarchy keep seeing the
        // project's true on-disk state between restarts.  Mirrors the Python
        // `didChangeWatchedFiles` handler.
        for change in params.changes {
            // Files the editor has open are driven by did_open/did_change; their
            // unsaved buffer must not be clobbered by the on-disk copy.
            // `reindex_index_from_disk` re-checks this under the lock as well.
            if self.documents.lock().await.contains_key(&change.uri) {
                continue;
            }
            if change.typ == FileChangeType::DELETED {
                self.workspace_index
                    .lock()
                    .await
                    .remove_document(change.uri.as_str());
            } else {
                // CREATED or CHANGED: re-analyse from disk (a Tcl source file)
                // or drop it if it no longer reads as one.
                self.reindex_index_from_disk(&change.uri).await;
            }
        }
    }

    async fn will_save_wait_until(
        &self,
        params: WillSaveTextDocumentParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        // Opt-in format-on-save, mirroring the Python server's
        // `willSaveWaitUntil` handler (`server/server.py`): gated behind
        // `tclLsp.features.willSaveWaitUntil`, which is **off by default**.
        // The capability is advertised unconditionally; the runtime guard
        // here returns no edits when the toggle is unset, matching the
        // repo's "always register, guard in the body" feature pattern.
        if !self
            .will_save_format_enabled(&params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let config = core_formatting::FormatterConfig {
            max_line_length: self.resolved_line_length(&params.text_document.uri).await as usize,
            ..core_formatting::FormatterConfig::default()
        };
        let edits = core_formatting::formatting_with(&doc.text, &config, &registry);
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        if !self
            .feature_enabled("folding", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // Pure-CPU tokenise/segment work; run on a worker so a parser panic
        // is contained as a JSON-RPC error rather than unwinding the event
        // loop (review-findings C3 — defence in depth).
        let ranges = tokio::task::spawn_blocking(move || {
            tcl_lsp_core::folding::folding_ranges(&doc.text, &doc.dialect, &registry)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("folding worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(ranges.into_iter().map(lift_folding_range).collect()))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        if !self
            .feature_enabled("documentSymbols", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        // BIG-IP config documents get a `module → kind → object` outline
        // built from their stanza tree rather than the Tcl scope walk
        // (which would find nothing in non-Tcl config text). Nameless
        // singletons fall back to their kind label so no outline symbol
        // ever carries an empty `name` (#534).
        // The two compute branches run pure-CPU on a worker so a parser
        // panic is contained as a JSON-RPC error (review-findings C3).
        let symbols = if Self::is_bigip_dialect(&doc.dialect) {
            let text = doc.text.clone();
            tokio::task::spawn_blocking(move || core_bigip::document_symbols(&text))
                .await
                .map_err(|err| jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: format!("document_symbol worker panicked: {err}").into(),
                    data: None,
                })?
        } else if let Some(symbols) = self.db_document_symbols(&params.text_document.uri).await {
            // Served from the salsa query graph (memoised; reuses the tracked
            // file_analysis instead of re-running the full analyser).
            symbols
        } else {
            // Cold / cancelled fallback: compute directly so the request never
            // returns empty due to a concurrent edit (behaviour preserved).
            let analysis = self
                .analysis_for(
                    &params.text_document.uri,
                    doc.text.clone(),
                    doc.dialect.clone(),
                )
                .await;
            let text = doc.text.clone();
            tokio::task::spawn_blocking(move || {
                core_symbols::document_symbols_from_analysis(&text, &analysis)
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("document_symbol worker panicked: {err}").into(),
                data: None,
            })?
        };
        let lifted: Vec<DocumentSymbol> = symbols.into_iter().map(lift_document_symbol).collect();
        Ok(Some(DocumentSymbolResponse::Nested(lifted)))
    }

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        if !self
            .feature_enabled(
                "completion",
                &params.text_document_position.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position.position;
        // Fetch the cached per-dialect registry on the async
        // path before spawning so the worker can read built-in
        // command names alongside the user-defined procs the
        // analyser surfaces.  Threads through as
        // `S-completion-rich`: minimal completion only knows
        // user procs / vars; rich completion adds the registry's
        // command set.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // Snapshot the cross-document index so the worker can
        // enumerate procs from sibling files.  Cloning is the
        // price of moving it into `spawn_blocking`; the index
        // holds only definition metadata, not document text.
        let workspace = self.workspace_index.lock().await.clone();
        // Pure-CPU work; spawn_blocking off the LSP event loop.
        let items = tokio::task::spawn_blocking(move || {
            core_completion::completions(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
                Some(&workspace),
                &doc.dialect,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("completion worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted: Vec<CompletionItem> = items
            .into_iter()
            .map(|item| lift_completion_item(item, pos.line))
            .collect();
        Ok(Some(CompletionResponse::Array(lifted)))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        if !self
            .feature_enabled(
                "definition",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let locations = self.compute_definition(&uri, pos).await?;
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDefinitionResponse::Array(locations)))
    }

    async fn goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<GotoDeclarationResponse>> {
        // GAP-B3 strip 3: for a `$var`, go-to-declaration resolves the
        // visible `global` / `variable` / `upvar` scoping statement that
        // declares the name; for any other symbol it defers to plain
        // go-to-definition (handled inside `core_declaration`).
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let ranges = tokio::task::spawn_blocking(move || {
            core_declaration::declaration(
                &text,
                pos.line,
                pos.character,
                &dialect,
                &analysis,
                &registry,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("declaration worker panicked: {err}").into(),
            data: None,
        })?;
        let locations: Vec<Location> = if ranges.is_empty() {
            // No in-document declaration / definition — try the
            // cross-document index (a sibling-file proc / class), the
            // same fallback go-to-definition uses.
            self.compute_definition(&uri, pos).await?
        } else {
            ranges
                .into_iter()
                .map(|r| Location {
                    uri: uri.clone(),
                    range: lift_lsp_range(r),
                })
                .collect()
        };
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDeclarationResponse::Array(locations)))
    }

    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoTypeDefinitionResponse>> {
        // GAP-B3 strip 2: type-definition jumps to the class that types
        // the symbol (a `$obj` instance's class, or a method's owning
        // class) — not the plain definition site it used to alias.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let ranges = tokio::task::spawn_blocking(move || {
            core_type_definition::type_definition(&text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("type-definition worker panicked: {err}").into(),
            data: None,
        })?;
        if ranges.is_empty() {
            return Ok(None);
        }
        let locations: Vec<Location> = ranges
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: lift_lsp_range(r),
            })
            .collect();
        Ok(Some(GotoTypeDefinitionResponse::Array(locations)))
    }

    async fn goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> jsonrpc::Result<Option<GotoImplementationResponse>> {
        // GAP-B3 strip 1: go-to-implementation is the TclOO subclass /
        // method-override fan-out, not the plain definition site it used
        // to alias.  Resolve in-document via `core_implementation`.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let ranges = tokio::task::spawn_blocking(move || {
            core_implementation::implementation(&text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("implementation worker panicked: {err}").into(),
            data: None,
        })?;
        if ranges.is_empty() {
            return Ok(None);
        }
        let locations: Vec<Location> = ranges
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: lift_lsp_range(r),
            })
            .collect();
        Ok(Some(GotoImplementationResponse::Array(locations)))
    }

    async fn references(&self, params: ReferenceParams) -> jsonrpc::Result<Option<Vec<Location>>> {
        if !self
            .feature_enabled(
                "references",
                &params.text_document_position.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let include_decl = params.context.include_declaration;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let analysis_for_worker = analysis.clone();
        let ranges = tokio::task::spawn_blocking(move || {
            core_references::references(
                &text,
                &dialect,
                pos.line,
                pos.character,
                &analysis_for_worker,
                include_decl,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("references worker panicked: {err}").into(),
            data: None,
        })?;
        // Current-document hits.
        let mut locations: Vec<Location> = ranges
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: lift_lsp_range(r),
            })
            .collect();
        // Cross-document call sites (and, when requested,
        // sibling-document definition sites).
        let cross = self
            .cross_document_references(&uri, &doc.text, &analysis, pos, include_decl)
            .await;
        locations.extend(cross);
        dedup_locations(&mut locations);
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(locations))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> jsonrpc::Result<Option<Vec<DocumentHighlight>>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let entries = tokio::task::spawn_blocking(move || {
            // `S-document-highlight-rich`: the kinded entry
            // point tags variable defining spans as `Write` and
            // their reads as `Read`; command-invocation heads
            // stay `Text`.
            core_references::document_highlights(
                &doc.text,
                &doc.dialect,
                pos.line,
                pos.character,
                &analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("document_highlight worker panicked: {err}").into(),
            data: None,
        })?;
        if entries.is_empty() {
            return Ok(None);
        }
        let highlights = entries
            .into_iter()
            .map(|(r, kind)| DocumentHighlight {
                range: lift_lsp_range(r),
                kind: Some(match kind {
                    core_references::HighlightKind::Read => DocumentHighlightKind::READ,
                    core_references::HighlightKind::Write => DocumentHighlightKind::WRITE,
                    core_references::HighlightKind::Text => DocumentHighlightKind::TEXT,
                }),
            })
            .collect();
        Ok(Some(highlights))
    }

    async fn prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyItem>>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let items = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::prepare(&doc.text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("call_hierarchy worker panicked: {err}").into(),
            data: None,
        })?;
        if items.is_empty() {
            return Ok(None);
        }
        let lifted = items
            .into_iter()
            .map(|i| CallHierarchyItem {
                name: i.name,
                kind: SymbolKind::FUNCTION,
                tags: None,
                detail: i.detail,
                uri: uri.clone(),
                range: lift_lsp_range(i.range),
                selection_range: lift_lsp_range(i.selection_range),
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyIncomingCall>>> {
        let item = params.item;
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let core_item = core_call_hierarchy::CallHierarchyItem {
            name: item.name,
            detail: item.detail,
            range: CoreLspRange {
                start_line: item.range.start.line,
                start_character: item.range.start.character,
                end_line: item.range.end.line,
                end_character: item.range.end.character,
            },
            selection_range: CoreLspRange {
                start_line: item.selection_range.start.line,
                start_character: item.selection_range.start.character,
                end_line: item.selection_range.end.line,
                end_character: item.selection_range.end.character,
            },
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let qualified = core_item.name.clone();
        let doc_text = doc.text.clone();
        let doc_dialect = doc.dialect.clone();
        let local_item = core_item.clone();
        let local_analysis = analysis.clone();
        let local = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::incoming_calls(
                &doc_text,
                &doc_dialect,
                &local_item,
                &local_analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("incoming_calls worker panicked: {err}").into(),
            data: None,
        })?;
        // Tag local results with the request URI.
        let mut tagged: Vec<(Url, core_call_hierarchy::IncomingCall)> =
            local.into_iter().map(|c| (uri.clone(), c)).collect();
        // Cross-document callers: run the externally-targeted
        // incoming-calls pass over every *other* analysed doc.
        tagged.extend(self.cross_document_incoming_calls(&uri, &qualified).await);
        let lifted = tagged
            .into_iter()
            .map(|(doc_uri, c)| CallHierarchyIncomingCall {
                from: CallHierarchyItem {
                    name: c.from.name,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail: c.from.detail,
                    uri: doc_uri,
                    range: lift_lsp_range(c.from.range),
                    selection_range: lift_lsp_range(c.from.selection_range),
                    data: None,
                },
                from_ranges: c.from_ranges.into_iter().map(lift_lsp_range).collect(),
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        let item = params.item;
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let core_item = core_call_hierarchy::CallHierarchyItem {
            name: item.name,
            detail: item.detail,
            range: CoreLspRange {
                start_line: item.range.start.line,
                start_character: item.range.start.character,
                end_line: item.range.end.line,
                end_character: item.range.end.character,
            },
            selection_range: CoreLspRange {
                start_line: item.selection_range.start.line,
                start_character: item.selection_range.start.character,
                end_line: item.selection_range.end.line,
                end_character: item.selection_range.end.character,
            },
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // Cross-document edges: callees defined in sibling files.
        let cross = self
            .cross_document_outgoing_calls(&uri, &doc.text, &doc.dialect, &core_item, &analysis)
            .await;
        let local_uri = uri.clone();
        let local_analysis = analysis.clone();
        let outgoing = tokio::task::spawn_blocking(move || {
            core_call_hierarchy::outgoing_calls(
                &doc.text,
                &doc.dialect,
                &core_item,
                &local_analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("outgoing_calls worker panicked: {err}").into(),
            data: None,
        })?;
        let mut lifted: Vec<CallHierarchyOutgoingCall> = outgoing
            .into_iter()
            .map(|c| CallHierarchyOutgoingCall {
                to: CallHierarchyItem {
                    name: c.to.name,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail: c.to.detail,
                    uri: local_uri.clone(),
                    range: lift_lsp_range(c.to.range),
                    selection_range: lift_lsp_range(c.to.selection_range),
                    data: None,
                },
                from_ranges: c.from_ranges.into_iter().map(lift_lsp_range).collect(),
            })
            .collect();
        lifted.extend(cross);
        Ok(Some(lifted))
    }

    // Note: type_hierarchy LSP methods aren't exposed by
    // tower-lsp 0.20's `LanguageServer` trait. The core
    // provider in `tcl_lsp_core::type_hierarchy` is ready;
    // wiring lands once we move to a tower-lsp version
    // that surfaces these methods (tracked under
    // `S-type-hierarchy-rich`).

    async fn linked_editing_range(
        &self,
        params: LinkedEditingRangeParams,
    ) -> jsonrpc::Result<Option<LinkedEditingRanges>> {
        // `S-linked-editing-range-rich`: when the cursor sits
        // on a proc declaration's name or inside the proc's
        // body, return every range that should be edited in
        // lock-step (the declaration plus any self-call sites
        // inside the body).  Editors then paint these as
        // linked-edit chips so renaming one updates the others
        // as the user types.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_linked_editing_range::linked_editing_ranges(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("linked_editing_range worker panicked: {err}").into(),
            data: None,
        })?;
        let Some(linked) = result else {
            return Ok(None);
        };
        Ok(Some(LinkedEditingRanges {
            ranges: linked.ranges.into_iter().map(lift_lsp_range).collect(),
            word_pattern: Some(linked.word_pattern),
        }))
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> jsonrpc::Result<DocumentDiagnosticReportResult> {
        // `S-diagnostics-pipeline`: pull-based diagnostic
        // report.  Editors that support
        // `textDocument/diagnostic` request diagnostics on
        // demand rather than relying on the
        // `publish_diagnostics` push.  We run the analyser via
        // the cached-analysis surface (the same shape every
        // other request uses) and lift each analyser
        // diagnostic to the LSP wire shape.
        let uri = params.text_document.uri.clone();
        let previous = params.previous_result_id;
        // Read the live document first so a cache entry from an older edit is
        // never served as current: a pull arriving between `did_change` and the
        // debounced worker's refresh must recompute, not return stale (or worse,
        // `Unchanged`) diagnostics for the edited buffer.
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(empty_diagnostic_report());
        };
        // BIG-IP config / iApp APL documents carry no general Tcl diagnostics
        // — the analyser never runs on them.  Their model-level validator
        // diagnostics (`BIGIP6xxx` / `IAPP7xxx`) are computed by the F5
        // dispatch inside `full_diagnostics_for` below (mirroring the push
        // path), so they flow through the same cache + report path as the Tcl
        // diagnostics rather than being short-circuited to empty here.
        // Serve from the push-maintained cache only when it matches the live
        // document revision: pull then mirrors the exact set the editor last
        // received via `publish_diagnostics`, and an unchanged document is
        // answered with a cheap `Unchanged` report (no diagnostics re-sent).
        // Mirrors the Python `on_document_diagnostic`.
        if let Some(entry) = self.pull_diag_cache.lock().await.get(&uri).cloned()
            && entry.revision == doc.revision
        {
            if previous.as_deref() == Some(entry.result_id.as_str()) {
                return Ok(DocumentDiagnosticReportResult::Report(
                    DocumentDiagnosticReport::Unchanged(RelatedUnchangedDocumentDiagnosticReport {
                        related_documents: None,
                        unchanged_document_diagnostic_report: UnchangedDocumentDiagnosticReport {
                            result_id: entry.result_id,
                        },
                    }),
                ));
            }
            return Ok(DocumentDiagnosticReportResult::Report(
                DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                    related_documents: None,
                    full_document_diagnostic_report: FullDocumentDiagnosticReport {
                        result_id: Some(entry.result_id),
                        items: entry.diagnostics,
                    },
                }),
            ));
        }
        // Cache miss or stale (a pull arrived before the push settled, or push
        // is disabled).  Compute the full set on demand, prime the cache with
        // the revision it was computed for, and return it.  Mirror the push
        // path's full set (analyser + compiler/optimiser + source-style) so
        // pull-mode editors don't lose the compiler / optimiser / style
        // warnings.
        let items = self
            .full_diagnostics_for(
                &uri,
                doc.text.clone(),
                doc.dialect.clone(),
                &doc.language_id,
            )
            .await;
        let result_id = next_pull_diag_result_id();
        self.pull_diag_cache.lock().await.insert(
            uri.clone(),
            PullDiagEntry {
                result_id: result_id.clone(),
                revision: doc.revision,
                diagnostics: items.clone(),
            },
        );
        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: Some(result_id),
                    items,
                },
            }),
        ))
    }

    async fn workspace_diagnostic(
        &self,
        params: WorkspaceDiagnosticParams,
    ) -> jsonrpc::Result<WorkspaceDiagnosticReportResult> {
        // Workspace pull: report every open document's last-published set from
        // the cache, honouring the per-URI `previousResultId` the client sends
        // so unchanged documents cost nothing.  Mirrors the Python
        // `on_workspace_diagnostic`.
        let previous: HashMap<Url, String> = params
            .previous_result_ids
            .into_iter()
            .map(|p| (p.uri, p.value))
            .collect();
        // Snapshot the cache (clone out) so we don't hold its lock while also
        // taking the `documents` lock for versions.
        let entries: Vec<(Url, PullDiagEntry)> = self
            .pull_diag_cache
            .lock()
            .await
            .iter()
            .map(|(uri, entry)| (uri.clone(), entry.clone()))
            .collect();
        let versions: HashMap<Url, Option<i64>> = {
            let docs = self.documents.lock().await;
            entries
                .iter()
                .map(|(uri, _)| {
                    (
                        uri.clone(),
                        docs.get(uri).and_then(|d| d.version.map(i64::from)),
                    )
                })
                .collect()
        };
        let items = entries
            .into_iter()
            .map(|(uri, entry)| {
                let version = versions.get(&uri).copied().flatten();
                if previous.get(&uri).map(String::as_str) == Some(entry.result_id.as_str()) {
                    WorkspaceDocumentDiagnosticReport::Unchanged(
                        WorkspaceUnchangedDocumentDiagnosticReport {
                            uri,
                            version,
                            unchanged_document_diagnostic_report:
                                UnchangedDocumentDiagnosticReport {
                                    result_id: entry.result_id,
                                },
                        },
                    )
                } else {
                    WorkspaceDocumentDiagnosticReport::Full(WorkspaceFullDocumentDiagnosticReport {
                        uri,
                        version,
                        full_document_diagnostic_report: FullDocumentDiagnosticReport {
                            result_id: Some(entry.result_id),
                            items: entry.diagnostics,
                        },
                    })
                }
            })
            .collect();
        Ok(WorkspaceDiagnosticReportResult::Report(
            WorkspaceDiagnosticReport { items },
        ))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // `S-semantic-tokens-rich`: real classification, served from the
        // memoised `semantic_tokens` query (packed integer stream, 5 ints per
        // token `[deltaLine, deltaCol, length, type, modifiers]`).  The
        // cold/cancelled fallback computes directly.
        let core_data = if let Some(tokens) = self.db_semantic_tokens(&uri).await {
            tokens.data
        } else {
            let registry = self.registry_for_dialect(&doc.dialect).await;
            // Cold/cancelled fallback runs the tokeniser on a worker so a
            // parser panic is contained as a JSON-RPC error (C3).
            let (text, dialect) = (doc.text.clone(), doc.dialect.clone());
            tokio::task::spawn_blocking(move || {
                core_semantic_tokens::full(&text, &dialect, &registry).data
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("semantic_tokens worker panicked: {err}").into(),
                data: None,
            })?
        };
        let result_id = next_semantic_tokens_id();
        Ok(Some(SemanticTokensResult::Tokens(LspSemanticTokens {
            result_id: Some(result_id),
            data: lift_semantic_token_data(&core_data),
        })))
    }

    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> jsonrpc::Result<Option<SemanticTokensFullDeltaResult>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // The server no longer keeps a per-URI token snapshot to diff against
        // (that hand-invalidated cache is gone).  A `full/delta` request is
        // answered with the full token set from the memoised query — the LSP
        // spec accepts `Tokens` in place of `TokensDelta`.
        let core_data = if let Some(tokens) = self.db_semantic_tokens(&uri).await {
            tokens.data
        } else {
            let registry = self.registry_for_dialect(&doc.dialect).await;
            // Cold/cancelled fallback runs the tokeniser on a worker so a
            // parser panic is contained as a JSON-RPC error (C3).
            let (text, dialect) = (doc.text.clone(), doc.dialect.clone());
            tokio::task::spawn_blocking(move || {
                core_semantic_tokens::full(&text, &dialect, &registry).data
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("semantic_tokens worker panicked: {err}").into(),
                data: None,
            })?
        };
        Ok(Some(SemanticTokensFullDeltaResult::Tokens(
            LspSemanticTokens {
                result_id: Some(next_semantic_tokens_id()),
                data: lift_semantic_token_data(&core_data),
            },
        )))
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<SemanticTokensRangeResult>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        // `S-semantic-tokens-rich`: filter tokens to those that
        // start inside the requested range.  Editors call this
        // when only the visible viewport needs colouring.
        let core_range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // Pure-CPU tokenisation on a worker so a parser panic is contained
        // as a JSON-RPC error (review-findings C3).
        let (text, dialect) = (doc.text.clone(), doc.dialect.clone());
        let core_data = tokio::task::spawn_blocking(move || {
            core_semantic_tokens::range(&text, &dialect, core_range, &registry).data
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("semantic_tokens_range worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(SemanticTokensRangeResult::Tokens(LspSemanticTokens {
            result_id: None,
            data: lift_semantic_token_data(&core_data),
        })))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<Vec<SymbolInformation>>> {
        // Walk every cached document and collect matching
        // symbols.  The minimal port iterates the document
        // store on the LSP loop (acquiring the mutex briefly);
        // a future workspace-index chunk will move this to a
        // pre-computed index.
        let docs = self.documents.lock().await.clone();
        if docs.is_empty() {
            return Ok(None);
        }
        let query = params.query;
        let mut all: Vec<SymbolInformation> = Vec::new();
        for (uri, doc) in docs {
            let text = doc.text.clone();
            let dialect = doc.dialect.clone();
            let q = query.clone();
            let analysis = self.analysis_for(&uri, text.clone(), dialect.clone()).await;
            let symbols = tokio::task::spawn_blocking(move || {
                core_workspace_symbols::workspace_symbols(&text, &q, &analysis)
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("workspace_symbols worker panicked: {err}").into(),
                data: None,
            })?;
            for s in symbols {
                #[allow(deprecated)]
                all.push(SymbolInformation {
                    name: s.name,
                    kind: match s.kind {
                        CoreWorkspaceSymbolKind::Function => SymbolKind::FUNCTION,
                        CoreWorkspaceSymbolKind::Class => SymbolKind::CLASS,
                        CoreWorkspaceSymbolKind::Method => SymbolKind::METHOD,
                        CoreWorkspaceSymbolKind::Constructor => SymbolKind::CONSTRUCTOR,
                    },
                    tags: None,
                    deprecated: None,
                    location: Location {
                        uri: uri.clone(),
                        range: lift_lsp_range(s.range),
                    },
                    container_name: s.container_name,
                });
            }
        }
        if all.is_empty() {
            return Ok(None);
        }
        Ok(Some(all))
    }

    async fn document_link(
        &self,
        params: DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<DocumentLink>>> {
        if !self
            .feature_enabled("documentLinks", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // `S-document-links-rich`: pass the document's
        // enclosing directory as the workspace root so
        // relative `source <path>` arguments resolve.  When
        // the URI isn't a `file://` URL we leave the workspace
        // root unset and only absolute paths surface as links.
        let workspace_root = uri
            .to_file_path()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
            .and_then(|p| p.to_str().map(str::to_owned));
        // Pure-CPU segmentation on a worker so a parser panic is contained
        // as a JSON-RPC error (review-findings C3).
        let (text, dialect) = (doc.text.clone(), doc.dialect.clone());
        let links = tokio::task::spawn_blocking(move || {
            core_document_links::document_links(&text, &dialect, workspace_root.as_deref())
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("document_link worker panicked: {err}").into(),
            data: None,
        })?;
        if links.is_empty() {
            return Ok(None);
        }
        let lifted = links
            .into_iter()
            .map(|l| DocumentLink {
                range: Range {
                    start: Position {
                        line: l.start_line,
                        character: l.start_character,
                    },
                    end: Position {
                        line: l.end_line,
                        character: l.end_character,
                    },
                },
                target: Url::parse(&l.target).ok(),
                tooltip: l.tooltip,
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> jsonrpc::Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri.clone();
        // Inlay hints are opt-in (default off), with the type-hint and
        // parameter-hint families gated independently.  The provider must
        // still answer with a well-formed (empty) list — never an error or
        // `null` — when both are disabled, mirroring the Python default-off
        // contract (`on_inlay_hint` returns `[]`).
        let type_hints = self.inlay_family_enabled(&uri, "inlayTypeHints").await;
        let parameter_hints = self.inlay_family_enabled(&uri, "inlayParameterHints").await;
        if !type_hints && !parameter_hints {
            return Ok(Some(Vec::new()));
        }
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // Build analysis on a worker so the inlay-hints provider can surface
        // inferred-type hints (`inlayTypeHints`) and parameter-name hints at
        // user-proc call sites plus built-in command synopsis hints
        // (`inlayParameterHints`).  When the analyser surfaces an empty
        // all_procs map (no user procs in the document), the provider still
        // returns built-in hints from the registry.
        let hints = tokio::task::spawn_blocking(move || {
            core_inlay_hints::inlay_hints(
                &doc.text,
                &doc.dialect,
                range,
                Some(&analysis),
                Some(&registry),
                type_hints,
                parameter_hints,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("inlay_hint worker panicked: {err}").into(),
            data: None,
        })?;
        if hints.is_empty() {
            return Ok(None);
        }
        let lifted = hints
            .into_iter()
            .map(|h| InlayHint {
                position: Position {
                    line: h.position_line,
                    character: h.position_character,
                },
                label: InlayHintLabel::String(h.label),
                kind: Some(match h.kind {
                    core_inlay_hints::InlayHintKind::Type => InlayHintKind::TYPE,
                    core_inlay_hints::InlayHintKind::Parameter => InlayHintKind::PARAMETER,
                }),
                text_edits: None,
                tooltip: None,
                padding_left: h.padding_left.then_some(true),
                padding_right: None,
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    async fn code_lens(&self, params: CodeLensParams) -> jsonrpc::Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // S-code-lens-rich: surface per-proc reference counts
        // above each definition.  The provider walks
        // `analysis.command_invocations` per proc, plus the
        // workspace index for cross-document call sites, so the
        // count reflects workspace-wide usage.
        let workspace = self.workspace_index.lock().await.clone();
        let uri_str = uri.to_string();
        let worker_uri = uri_str.clone();
        let lenses = tokio::task::spawn_blocking(move || {
            core_code_lens::code_lenses(
                &doc.text,
                &doc.dialect,
                Some(&analysis),
                Some(&workspace),
                &worker_uri,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("code_lens worker panicked: {err}").into(),
            data: None,
        })?;
        if lenses.is_empty() {
            return Ok(None);
        }
        let lifted = lenses
            .into_iter()
            .map(|l| CodeLens {
                range: lift_lsp_range(l.range),
                // Carry the qualified name *and* the document URI so
                // `codeLens/resolve` can recompute the count against the live
                // workspace (mirrors Python's `_LensData{kind, uri, qname}`).
                // Method / class-member lenses have no qname and stay
                // informational (their eager title is authoritative).
                data: (!l.qname.is_empty())
                    .then(|| serde_json::json!({ "qname": l.qname, "uri": uri_str.clone() })),
                command: Some(tower_lsp::lsp_types::Command {
                    title: l.command_title,
                    command: l.command,
                    arguments: None,
                }),
            })
            .collect();
        Ok(Some(lifted))
    }

    /// Resolve a code lens to its authoritative reference-count title.
    ///
    /// Rust port of `server.py::on_code_lens_resolve` /
    /// `code_lens.py::resolve_code_lens`.  The server advertises lenses
    /// eagerly with a count, but the client calls `codeLens/resolve`
    /// before display; recomputing here against the *current* document and
    /// workspace keeps the title consistent with Find All References even
    /// when the workspace changed since the lens was produced (issue #637).
    ///
    /// A lens with no `{qname, uri}` data (the informational method /
    /// class-member lenses) is returned unchanged — its eager title stands.
    async fn code_lens_resolve(&self, lens: CodeLens) -> jsonrpc::Result<CodeLens> {
        let Some((qname, uri)) = lens
            .data
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .and_then(|data| {
                let qname = data.get("qname").and_then(serde_json::Value::as_str)?;
                let uri = data.get("uri").and_then(serde_json::Value::as_str)?;
                Some((qname.to_owned(), Url::parse(uri).ok()?))
            })
        else {
            return Ok(lens);
        };
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(lens);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let workspace = self.workspace_index.lock().await.clone();
        let uri_str = uri.to_string();
        let lenses = tokio::task::spawn_blocking(move || {
            core_code_lens::code_lenses(
                &doc.text,
                &doc.dialect,
                Some(&analysis),
                Some(&workspace),
                &uri_str,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("code_lens_resolve worker panicked: {err}").into(),
            data: None,
        })?;
        let mut lens = lens;
        if let Some(matching) = lenses.into_iter().find(|l| l.qname == qname) {
            lens.command = Some(tower_lsp::lsp_types::Command {
                title: matching.command_title,
                command: matching.command,
                arguments: None,
            });
        }
        Ok(lens)
    }

    #[allow(clippy::too_many_lines)]
    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<Vec<CodeActionOrCommand>>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // `S-code-actions-rich`: walks the analyser's
        // diagnostics for fixes whose span overlaps the
        // requested range, plus fuzzy `package require`
        // suggestions for the word at the cursor.  Run on a worker.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // The resolved per-check disabled set — the same one baked into the
        // analyser build above — so the compiler-checks code-action path does
        // not re-surface a quick-fix for a diagnostic the user disabled.
        let (disabled_codes, _) = self.analyser_config().await;
        // Lift the request-context diagnostics (the editor sends the ones it
        // currently shows) so context-driven quick-fixes — e.g. the iRules
        // taint encode-wrap fixes — can act on them even when the analyser
        // didn't re-emit them.
        let context_diags: Vec<core_code_actions::ContextDiagnostic> = params
            .context
            .diagnostics
            .iter()
            .filter_map(|d| {
                let code = match d.code.as_ref()? {
                    tower_lsp::lsp_types::NumberOrString::String(s) => s.clone(),
                    tower_lsp::lsp_types::NumberOrString::Number(n) => n.to_string(),
                };
                Some(core_code_actions::ContextDiagnostic {
                    code,
                    message: d.message.clone(),
                    range: CoreLspRange {
                        start_line: d.range.start.line,
                        start_character: d.range.start.character,
                        end_line: d.range.end.line,
                        end_character: d.range.end.character,
                    },
                })
            })
            .collect();
        let dialect = doc.dialect.clone();
        let actions = tokio::task::spawn_blocking(move || {
            let mut actions = core_code_actions::code_actions(&doc.text, range, Some(&analysis));
            actions.extend(core_code_actions::package_require_actions(
                &doc.text, range, &registry,
            ));
            actions.extend(core_code_actions::context_diagnostic_actions(
                &doc.text,
                &context_diags,
            ));
            // iRules-only: the `# Profiles:` header source action plus the
            // control-flow quick-fixes (IRULE5002 unguarded drop / IRULE5004
            // DNS::return) the analyser produces through the compiler-checks
            // pass rather than `AnalysisResult.diagnostics`.  Only the iRules
            // dialect's checks carry fixes, so the (re-)lowering is gated here.
            if dialect == "f5-irules" {
                if let Some(a) = core_code_actions::profiles_action(&doc.text, &analysis, &registry)
                {
                    actions.push(a);
                }
                let checks =
                    tcl_lsp_db::compiler_check_diagnostics_uncached(&doc.text, &registry, &dialect);
                actions.extend(core_code_actions::check_diagnostic_actions(
                    &doc.text,
                    range,
                    &checks.checks,
                    &disabled_codes,
                ));
            }
            actions
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("code_action worker panicked: {err}").into(),
            data: None,
        })?;
        if actions.is_empty() {
            return Ok(None);
        }
        // Honour the client's `only` filter (e.g. `["refactor.extract"]`): an
        // action is kept when its kind prefix-matches a requested kind in
        // either direction (`refactor` matches `refactor.extract` and vice
        // versa).
        let only: Option<Vec<String>> = params.context.only.as_ref().map(|kinds| {
            kinds
                .iter()
                .map(|k| k.as_str().to_owned())
                .collect::<Vec<_>>()
        });
        let lifted: Vec<CodeActionOrCommand> = actions
            .into_iter()
            .filter(|a| {
                only.as_ref().is_none_or(|wanted| {
                    let k = a.kind.as_str();
                    // Keep an action when its kind exactly matches a requested
                    // kind, or is a *subtype* of one (`refactor.extract`
                    // satisfies a `refactor` request). A requested
                    // `refactor.extract` must NOT pull in a generic `refactor`
                    // action — the requested kind is not a parent of the
                    // action's kind in that direction (LSP CodeActionKind
                    // matching is prefix-on-the-action-side only).
                    wanted
                        .iter()
                        .any(|w| k == w || k.starts_with(&format!("{w}.")))
                })
            })
            .map(|a| {
                let mut changes = std::collections::HashMap::new();
                let lifted_edits: Vec<TextEdit> = a
                    .edits
                    .into_iter()
                    .map(|e| TextEdit {
                        range: lift_lsp_range(e.range),
                        new_text: e.new_text,
                    })
                    .collect();
                changes.insert(uri.clone(), lifted_edits);
                let command = a.command.map(|c| tower_lsp::lsp_types::Command {
                    title: a.title.clone(),
                    command: c.command,
                    arguments: Some(c.args.into_iter().map(serde_json::Value::from).collect()),
                });
                // Surface the rendered tmsh data-group definition (the
                // extract-to-datagroup refactor) as the action's `data`
                // payload, mirroring Python's
                // `_datagroup_to_code_action`'s `data={"data_group_definition": …}`.
                let data = a
                    .data_group_definition
                    .map(|def| serde_json::json!({ "data_group_definition": def }));
                CodeActionOrCommand::CodeAction(CodeAction {
                    title: a.title,
                    kind: Some(tower_lsp::lsp_types::CodeActionKind::new(a.kind.as_str())),
                    diagnostics: None,
                    edit: Some(WorkspaceEdit {
                        changes: Some(changes),
                        document_changes: None,
                        change_annotations: None,
                    }),
                    command,
                    is_preferred: None,
                    disabled: None,
                    data,
                })
            })
            .collect();
        if lifted.is_empty() {
            return Ok(None);
        }
        Ok(Some(lifted))
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "tcl-lsp.minifyDocument" => self.minify_document_command(&params.arguments).await,
            "tcl-lsp.optimiseDocument" => self.optimise_document_command(&params.arguments).await,
            "tcl-lsp.unminifyError" => Ok(Self::unminify_error_command(&params.arguments)),
            "tcl-lsp.describeIruleEvent" => {
                self.describe_irule_event_command(&params.arguments).await
            }
            "tcl-lsp.describeIruleCommand" => {
                self.describe_irule_command_command(&params.arguments).await
            }
            "tcl-lsp.listIruleEvents" => Ok(Some(Self::list_irule_events_command())),
            "tcl-lsp.diagramData" => Ok(Self::diagram_data_command(&params.arguments)),
            "tcl-lsp.getEffectiveConfig" => {
                self.get_effective_config_command(&params.arguments).await
            }
            "tcl-lsp.fixAllSafeIssues" => self.fix_all_safe_issues_command(&params.arguments).await,
            "tcl-lsp.listSubcommands" => Ok(self.list_subcommands_command(&params.arguments).await),
            "tcl-lsp.listKnownPackages" => Ok(Some(Self::list_known_packages_command())),
            "tcl-lsp.suggestPackagesForSymbol" => Ok(Some(
                Self::suggest_packages_for_symbol_command(&params.arguments),
            )),
            "tcl-lsp.exportConfig" => Ok(Some(self.export_config_command().await)),
            _ => Ok(None),
        }
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // An explicit client `FormattingOptions.tabSize` / `insertSpaces`
        // overrides the server's indentation by LSP contract.
        let config = formatter_config_from_options(&params.options);
        // Pure-CPU formatting on a worker so a parser panic is contained as
        // a JSON-RPC error (review-findings C3).
        let text = doc.text.clone();
        let edits = tokio::task::spawn_blocking(move || {
            core_formatting::formatting_with(&text, &config, &registry)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("formatting worker panicked: {err}").into(),
            data: None,
        })?;
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // Pure-CPU formatting on a worker so a parser panic is contained as
        // a JSON-RPC error (review-findings C3).
        let text = doc.text.clone();
        let edits = tokio::task::spawn_blocking(move || {
            core_formatting::range_formatting(
                &text,
                range,
                &core_formatting::FormatterConfig::default(),
                &registry,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("range_formatting worker panicked: {err}").into(),
            data: None,
        })?;
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<SelectionRange>>> {
        if !self
            .feature_enabled("selectionRange", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let positions = params.positions;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            positions
                .into_iter()
                .map(|pos| {
                    let chain = core_selection_range::selection_range(
                        &doc.text,
                        pos.line,
                        pos.character,
                        Some(&analysis),
                    );
                    materialise_selection_range(&chain)
                })
                .collect::<Vec<Option<SelectionRange>>>()
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("selection_range worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted: Vec<SelectionRange> = result.into_iter().flatten().collect();
        if lifted.is_empty() {
            return Ok(None);
        }
        Ok(Some(lifted))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_rename::prepare_rename(&doc.text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("prepare_rename worker panicked: {err}").into(),
            data: None,
        })?;
        let Some(p) = result else {
            return Ok(None);
        };
        Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
            range: lift_lsp_range(p.range),
            placeholder: p.placeholder,
        }))
    }

    async fn rename(&self, params: RenameParams) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // Per-dialect cached registry for `S-rename-rich`
        // safety gating — proc renames refuse to overwrite
        // built-in command names.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let analysis_for_worker = analysis.clone();
        let new_name_worker = new_name.clone();
        let registry_worker = Arc::clone(&registry);
        let edits = tokio::task::spawn_blocking(move || {
            core_rename::rename(
                &text,
                &dialect,
                pos.line,
                pos.character,
                &new_name_worker,
                &analysis_for_worker,
                Some(&registry_worker),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("rename worker panicked: {err}").into(),
            data: None,
        })?;
        let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
            std::collections::HashMap::new();
        // An empty in-document result means the rename was *rejected*
        // (collision with an existing symbol, an unsafe / built-in target
        // name) or the cursor isn't on a renameable symbol.  In every one
        // of those cases the rename must abort wholesale — adding
        // cross-document edits for a locally-rejected rename would leak a
        // partial, inconsistent edit set into sibling documents.
        let local_rejected = edits.is_empty();
        if !edits.is_empty() {
            let lifted: Vec<TextEdit> = edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect();
            changes.insert(uri.clone(), lifted);
        }
        // Cross-document rename: rewrite the symbol's call /
        // definition sites in sibling documents.  Gated on the
        // same safety checks as the in-document path
        // (`is_safe_symbol_name`, no built-in shadow) so a
        // cross-doc rename can't produce an unsafe edit set — and
        // skipped entirely when the in-document rename was rejected.
        if !local_rejected
            && core_rename::is_safe_symbol_name(&new_name)
            && !core_rename::is_builtin_command_name(&new_name, &registry)
        {
            self.add_cross_document_rename_edits(
                &uri,
                &doc.text,
                &analysis,
                pos,
                &new_name,
                &mut changes,
            )
            .await;
        }
        if changes.is_empty() {
            return Ok(None);
        }
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }))
    }

    /// `workspace/willRenameFiles` (GAP-A9): when `.tcl` files are
    /// renamed in the editor, rewrite every dependent file's `source`
    /// literal so the workspace still loads.  The pure core
    /// (`core_file_ops::compute_rename_edits`) returns byte-span edits
    /// keyed by dependent URI; here we resolve each span to an LSP
    /// range against the dependent's current text and assemble a
    /// `WorkspaceEdit` (one `TextDocumentEdit` per dependent, matching
    /// the Python `compute_batch_rename_edits`).
    async fn will_rename_files(
        &self,
        params: RenameFilesParams,
    ) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        // Collect the byte-span edits for every rename in the batch.
        let roots: Vec<String> = self
            .workspace_folder_urls()
            .await
            .iter()
            .map(Url::to_string)
            .collect();
        let raw_edits: Vec<core_file_ops::RenameEdit> = {
            let index = self.workspace_index.lock().await;
            params
                .files
                .iter()
                .flat_map(|f| {
                    core_file_ops::compute_rename_edits(&f.old_uri, &f.new_uri, &index, &roots)
                })
                .collect()
        };
        if raw_edits.is_empty() {
            return Ok(None);
        }
        // Group by dependent URI, resolving each span against that
        // document's source (open buffer or on-disk fallback).
        let mut by_dep: HashMap<Url, Vec<TextEdit>> = HashMap::new();
        for edit in raw_edits {
            let Ok(dep_url) = Url::parse(&edit.uri) else {
                continue;
            };
            let Some(doc) = self.read_document(&dep_url).await else {
                continue;
            };
            let line_index = doc.line_index.clone();
            by_dep.entry(dep_url).or_default().push(TextEdit {
                range: lift_span(&doc.text, &line_index, edit.span),
                new_text: edit.new_text,
            });
        }
        if by_dep.is_empty() {
            return Ok(None);
        }
        let document_changes: Vec<TextDocumentEdit> = by_dep
            .into_iter()
            .map(|(uri, edits)| TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier { uri, version: None },
                edits: edits.into_iter().map(OneOf::Left).collect(),
            })
            .collect();
        Ok(Some(WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Edits(document_changes)),
            change_annotations: None,
        }))
    }

    /// `workspace/didRenameFiles` (GAP-A9): after the client applies a
    /// rename on disk, refresh the workspace index — drop the old URI's
    /// entries and re-index the renamed file from its new path so
    /// cross-document features (including future renames) stay current.
    /// Mirrors the Python `on_did_rename_files`.
    async fn did_rename_files(&self, params: RenameFilesParams) {
        for f in &params.files {
            if let Ok(old_url) = Url::parse(&f.old_uri) {
                self.workspace_index
                    .lock()
                    .await
                    .remove_document(old_url.as_str());
            }
            if let Ok(new_url) = Url::parse(&f.new_uri) {
                self.reindex_index_from_disk(&new_url).await;
            }
        }
    }

    async fn signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> jsonrpc::Result<Option<SignatureHelp>> {
        if !self
            .feature_enabled(
                "signatureHelp",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        // Per-dialect cached registry — same source the
        // completion handler uses.  Threading it through lets
        // `S-signature-help-rich` surface signatures for
        // built-in commands (e.g. `puts`, `lsearch`) without
        // requiring a user proc with the same name.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_sig::signature_help(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("signature_help worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(result.map(lift_signature_help))
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        if !self
            .feature_enabled(
                "hover",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        // Run the provider over the memoised `file_analysis` query (no separate
        // hover result cache — walking the cached analysis is cheap, and the
        // query graph already avoids re-analysing).  Worker-offload via
        // `spawn_blocking` keeps the LSP event loop responsive.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let result = tokio::task::spawn_blocking(move || {
            core_hover::hover(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("hover worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(result.map(lift_hover))
    }
}

fn lift_completion_kind(k: CoreCompletionKind) -> CompletionItemKind {
    match k {
        CoreCompletionKind::Variable => CompletionItemKind::VARIABLE,
        CoreCompletionKind::Function => CompletionItemKind::FUNCTION,
        CoreCompletionKind::EnumValue => CompletionItemKind::ENUM_MEMBER,
        CoreCompletionKind::Snippet => CompletionItemKind::SNIPPET,
    }
}

fn lift_completion_item(item: CoreCompletionItem, line: u32) -> CompletionItem {
    // An explicit single-line replacement edit (var / switch / array) — the
    // editor applies it verbatim so the `$`/`-` prefix isn't dropped/duplicated.
    let text_edit = item.text_edit.map(|e| {
        tower_lsp::lsp_types::CompletionTextEdit::Edit(tower_lsp::lsp_types::TextEdit {
            range: tower_lsp::lsp_types::Range {
                start: tower_lsp::lsp_types::Position::new(line, e.start_char),
                end: tower_lsp::lsp_types::Position::new(line, e.end_char),
            },
            new_text: e.new_text,
        })
    });
    let documentation = item
        .documentation
        .map(tower_lsp::lsp_types::Documentation::String);
    CompletionItem {
        label: item.label,
        kind: Some(lift_completion_kind(item.kind)),
        insert_text: Some(item.insert_text),
        detail: item.detail,
        documentation,
        sort_text: item.sort_text,
        // GAP-A9: snippet items carry VS Code tabstop syntax and
        // filter on their `tcl-…` prefix.
        insert_text_format: item
            .is_snippet
            .then_some(tower_lsp::lsp_types::InsertTextFormat::SNIPPET),
        filter_text: item.filter_text,
        text_edit,
        ..CompletionItem::default()
    }
}

/// Materialise the `tcl-lsp-core::selection_range` flat-vector
/// representation into a `tower_lsp` `SelectionRange` tree.
///
/// The chain in `core` is innermost first, with each link
/// pointing at its parent index; the LSP wire shape recurses
/// outward via `parent: Option<Box<SelectionRange>>`.
fn materialise_selection_range(
    chain: &[core_selection_range::SelectionRange],
) -> Option<SelectionRange> {
    if chain.is_empty() {
        return None;
    }
    // Build LSP records bottom-up so the parent box for index
    // `i` is already constructed when we wrap index `i-1`.
    let mut wrapped: Vec<Option<SelectionRange>> = chain.iter().map(|_| None).collect();
    // Process in reverse (outermost first) so each link's
    // parent — at a higher index — is already populated.
    for (i, link) in chain.iter().enumerate().rev() {
        let parent_box = link
            .parent_index
            .and_then(|p| wrapped[p].clone().map(Box::new));
        wrapped[i] = Some(SelectionRange {
            range: lift_lsp_range(link.range),
            parent: parent_box,
        });
    }
    wrapped.into_iter().next().flatten()
}

/// Byte offset of the LSP `(line, character)` cursor in `source`.
/// `character` is a UTF-16 code-unit offset. `None` when the line is
/// out of range.
fn line_col_to_byte_offset(source: &str, line: u32, col: u32) -> Option<usize> {
    let index = tcl_lexer::LineIndex::new(source);
    if line as usize >= index.line_count() {
        return None;
    }
    Some(index.offset_at_utf16(line, col, source) as usize)
}

/// Whether the cursor at `pos` sits on a command head (the first
/// word of a command), per the analyser's recorded command
/// invocations.  Used to gate the cross-document go-to-definition
/// fallback so it only fires on call sites, not on argument words
/// that happen to collide with a sibling proc/class name.
fn position_is_command_head(source: &str, pos: Position, analysis: &AnalysisResult) -> bool {
    let Some(offset) = line_col_to_byte_offset(source, pos.line, pos.character) else {
        return false;
    };
    analysis.command_invocations.iter().any(|inv| {
        let (start, end) = (inv.range.start() as usize, inv.range.end() as usize);
        offset >= start && offset < end
    })
}

fn lift_lsp_range(r: CoreLspRange) -> Range {
    Range {
        start: Position {
            line: r.start_line,
            character: r.start_character,
        },
        end: Position {
            line: r.end_line,
            character: r.end_character,
        },
    }
}

/// Deduplicate `Location`s by `(uri, range)`, preserving first-
/// seen order.  Used to merge current-document and cross-
/// document reference hits without double-listing a location.
fn dedup_locations(locations: &mut Vec<Location>) {
    let mut seen: std::collections::HashSet<(String, u32, u32, u32, u32)> =
        std::collections::HashSet::new();
    locations.retain(|loc| {
        seen.insert((
            loc.uri.to_string(),
            loc.range.start.line,
            loc.range.start.character,
            loc.range.end.line,
            loc.range.end.character,
        ))
    });
}

/// Lift the analyser's diagnostic records into the LSP wire
/// shape.  Shared by both the push-based `publish_diagnostics`
/// path (via `publish_analyser_diagnostics`) and the pull-
/// based `textDocument/diagnostic` handler.
/// Resolve a byte `span` to an LSP `Range` through `line_index`.
/// Shared by every `lift_*_diagnostics` helper so the offset →
/// (line, character) mapping is identical for analyser, compiler-
/// check, and optimiser diagnostics.
/// The startup optimisation profile: the `TCL_LSP_OPTIMISER_PROFILE` env
/// override (used by the fp/ground-truth battery harness to request the
/// `full` profile), else the editor default
/// ([`tcl_compiler::optimiser::profiles::DEFAULT_EDITOR_PROFILE`]).
/// A `tclLsp.optimiser.profile` config pull overrides it at runtime.
fn default_optimiser_profile() -> tcl_compiler::optimiser::profiles::OptimisationProfile {
    std::env::var("TCL_LSP_OPTIMISER_PROFILE").map_or(
        tcl_compiler::optimiser::profiles::DEFAULT_EDITOR_PROFILE,
        |s| tcl_compiler::optimiser::profiles::OptimisationProfile::parse(&s),
    )
}

/// Map a `tclLsp.style.nonAscii` setting string to a [`NonAsciiMode`].
/// An unknown / absent value resolves to [`NonAsciiMode::Default`] (the
/// per-dialect auto behaviour).
fn parse_non_ascii_mode(s: &str) -> NonAsciiMode {
    match s {
        "off" => NonAsciiMode::Off,
        "strict" => NonAsciiMode::Strict,
        "confusables" => NonAsciiMode::Confusables,
        "common" => NonAsciiMode::Common,
        _ => NonAsciiMode::Default,
    }
}

/// Build a [`core_formatting::FormatterConfig`] from an LSP request's
/// [`FormattingOptions`].  An explicit client `tabSize` / `insertSpaces`
/// overrides the server's default indentation by LSP contract; every
/// other formatter knob keeps its default.
fn formatter_config_from_options(
    options: &tower_lsp::lsp_types::FormattingOptions,
) -> core_formatting::FormatterConfig {
    let indent_size = usize::try_from(options.tab_size).unwrap_or(4).max(1);
    let indent_style = if options.insert_spaces {
        core_formatting::IndentStyle::Spaces
    } else {
        core_formatting::IndentStyle::Tabs
    };
    core_formatting::FormatterConfig {
        indent_size,
        indent_style,
        continuation_indent: indent_size,
        ..core_formatting::FormatterConfig::default()
    }
}

/// Resolve the user config-file path for `tcl-lsp.exportConfig`:
/// `$XDG_CONFIG_HOME/tcl-lsp/config.ini`, falling back to
/// `$HOME/.config/tcl-lsp/config.ini`.  Both env vars are honoured on every
/// platform so tests can isolate the server's config via `XDG_CONFIG_HOME`.
fn config_ini_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));
    base.join("tcl-lsp").join("config.ini")
}

/// Render a [`NonAsciiMode`] for `getEffectiveConfig`.  The unset
/// [`NonAsciiMode::Default`] reports as JSON `null` (the per-dialect
/// auto behaviour, mirroring the Python server's `None`); every explicit
/// mode reports its setting string.
fn non_ascii_mode_str(mode: NonAsciiMode) -> serde_json::Value {
    let label = match mode {
        NonAsciiMode::Default => return serde_json::Value::Null,
        NonAsciiMode::Off => "off",
        NonAsciiMode::Strict => "strict",
        NonAsciiMode::Confusables => "confusables",
        NonAsciiMode::Common => "common",
    };
    serde_json::Value::String(label.to_owned())
}

/// Extract `tclLsp.style.nonAscii` from an LSP settings payload, accepting
/// both the nested (`{"tclLsp":{"style":{"nonAscii":"…"}}}`) and the
/// flat-dotted (`{"tclLsp.style.nonAscii":"…"}`) shapes editors send.
fn settings_non_ascii_mode(settings: &serde_json::Value) -> Option<NonAsciiMode> {
    let nested = settings
        .get("tclLsp")
        .and_then(|v| v.get("style"))
        .and_then(|v| v.get("nonAscii"));
    nested
        .or_else(|| settings.get("tclLsp.style.nonAscii"))
        .and_then(serde_json::Value::as_str)
        .map(parse_non_ascii_mode)
}

/// Extract the disabled diagnostic codes from a settings payload — the
/// `tclLsp.diagnostics.<CODE>` booleans whose value is `false`. Accepts
/// the nested object (`{"tclLsp":{"diagnostics":{"W001":false}}}`) and
/// the flat-dotted (`{"tclLsp.diagnostics.W001":false}`) shapes. Returns
/// `None` when no diagnostics config is present (so the caller leaves the
/// current set untouched).
fn settings_disabled_diagnostics(settings: &serde_json::Value) -> Option<HashSet<String>> {
    if let Some(map) = settings
        .get("tclLsp")
        .and_then(|v| v.get("diagnostics"))
        .and_then(serde_json::Value::as_object)
    {
        return Some(
            map.iter()
                .filter(|(_, v)| v.as_bool() == Some(false))
                .map(|(k, _)| k.clone())
                .collect(),
        );
    }
    let obj = settings.as_object()?;
    let mut set = HashSet::new();
    let mut found = false;
    for (k, v) in obj {
        if let Some(code) = k.strip_prefix("tclLsp.diagnostics.") {
            found = true;
            if v.as_bool() == Some(false) {
                set.insert(code.to_owned());
            }
        }
    }
    found.then_some(set)
}

/// Parse one folder's resolved `tclLsp` config object into a [`FolderConfig`]
/// of overrides.  Keys absent from the object stay `None` / empty so the
/// resolver inherits the process-global value.  Returns `None` when `cfg` is
/// not a JSON object (the folder pull returned nothing usable).  Mirrors the
/// key handling of [`Backend::pull_and_apply_config`]'s global pull.
fn parse_folder_config(cfg: &serde_json::Value) -> Option<FolderConfig> {
    let obj = cfg.as_object()?;
    let mut fc = FolderConfig::default();
    if let Some(features) = obj.get("features").and_then(serde_json::Value::as_object) {
        fc.feature_toggles.apply(features);
    }
    // `xcDiagnostics.enabled` is a config section, not a `features.*` key
    // (see `pull_and_apply_config`) — map it onto the toggle for the folder.
    if let Some(flag) = obj
        .get("xcDiagnostics")
        .and_then(|x| x.get("enabled"))
        .and_then(serde_json::Value::as_bool)
    {
        fc.feature_toggles.set_flag("xcDiagnostics", flag);
    }
    if let Some(opt) = obj.get("optimiser").and_then(serde_json::Value::as_object) {
        if let Some(b) = opt.get("enabled").and_then(serde_json::Value::as_bool) {
            fc.optimiser_enabled = Some(b);
        }
        if let Some(p) = opt.get("profile").and_then(serde_json::Value::as_str) {
            fc.optimiser_profile =
                Some(tcl_compiler::optimiser::profiles::OptimisationProfile::parse(p));
        }
        for (key, val) in opt {
            if key == "enabled" || key == "profile" {
                continue;
            }
            if let Some(b) = val.as_bool() {
                fc.optimiser_code_overrides.insert(key.clone(), b);
            }
        }
    }
    if let Some(len) = obj
        .get("formatting")
        .and_then(|f| f.get("lineLength"))
        .or_else(|| obj.get("lineLength"))
        .and_then(serde_json::Value::as_u64)
    {
        fc.line_length = Some(u32::try_from(len).unwrap_or(80));
    }
    // The disabled-diagnostics and non-ASCII helpers expect the value wrapped
    // under `tclLsp`; the per-folder pull hands us the section content directly.
    let wrapped = serde_json::json!({ "tclLsp": cfg });
    fc.non_ascii_mode = settings_non_ascii_mode(&wrapped);
    fc.disabled_diagnostics = settings_disabled_diagnostics(&wrapped);
    Some(fc)
}

/// Apply one LSP content change to `text` (incremental document sync).
///
/// A `None` range is a full-document replacement; a `Some` range is a
/// ranged edit whose `(line, character)` UTF-16 positions are resolved to
/// byte offsets via [`tcl_lexer::LineIndex::offset_at_utf16`] and spliced.
/// Offsets are clamped and ordered so the splice indices are always valid
/// char boundaries within `text`.
#[cfg(test)]
fn apply_content_change(text: &str, range: Option<Range>, new_text: &str) -> String {
    // Throwaway index for the splice-behaviour tests; the live path persists one
    // through [`apply_content_change_indexed`].
    let mut index = tcl_lexer::LineIndex::new(text);
    apply_content_change_indexed(text, range, new_text, &mut index)
}

/// [`apply_content_change`] that resolves the edit through a **persisted**
/// [`tcl_lexer::LineIndex`] and **patches it in place** to match the returned
/// text (SRV-INCREMENTAL Task 1) — so neither the offset resolution nor the
/// index maintenance rescans the whole document.  A full-document replacement
/// (`range == None`) rebuilds the index from the new text.
fn apply_content_change_indexed(
    text: &str,
    range: Option<Range>,
    new_text: &str,
    index: &mut tcl_lexer::LineIndex,
) -> String {
    let Some(range) = range else {
        *index = tcl_lexer::LineIndex::new(new_text);
        return new_text.to_owned();
    };
    let a = index.offset_at_utf16(range.start.line, range.start.character, text) as usize;
    let b = index.offset_at_utf16(range.end.line, range.end.character, text) as usize;
    let len = text.len();
    let start = a.min(b).min(len);
    let end = a.max(b).min(len);
    // Patch the index for the same [start, end) → new_text splice applied below.
    index.apply_edit(
        u32::try_from(start).expect("offset fits u32"),
        u32::try_from(end).expect("offset fits u32"),
        new_text,
    );
    let mut out = String::with_capacity(len - (end - start) + new_text.len());
    out.push_str(&text[..start]);
    out.push_str(new_text);
    out.push_str(&text[end..]);
    out
}

fn lift_span(source: &str, line_index: &tcl_lexer::LineIndex, span: tcl_lexer::Span) -> Range {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    Range {
        start: Position {
            line: start.line,
            character: start.character,
        },
        end: Position {
            line: end.line,
            character: end.character,
        },
    }
}

/// Diagnostic codes that are *default-off* in the editor catalogue —
/// the analyser emits them (matching the Python `analyse` contract) but
/// the LSP layer is the consuming filter, so they are dropped from the
/// published set unless explicitly enabled.  Mirrors Python's
/// default-disabled code handling; the Rust server has no per-code
/// enable config yet, so these are simply never published.
const DEFAULT_OFF_CODES: &[&str] = &["W242"];

/// Default BIG-IP partition assumed when a config carries no explicit
/// one, matching Python `parse_bigip_conf`'s `default_partition="Common"`.
const BIGIP_DEFAULT_PARTITION: &str = "Common";

/// Lift a [`tcl_bigip::validator::ConfigDiagnostic`] (the output of the
/// BIG-IP config / iApp model-level validators) to the LSP wire shape,
/// mirroring Python `server/features/diagnostics.py::_to_lsp_diagnostic`
/// for these validators.  A `tcl_bigip` [`tcl_bigip::Range`] carries
/// UTF-16 columns (LSP convention) with an **inclusive** end, so — exactly
/// as Python's `to_lsp_range` — the LSP end column is `end.character + 1`.
fn lift_config_diagnostic(
    d: &tcl_bigip::validator::ConfigDiagnostic,
) -> tower_lsp::lsp_types::Diagnostic {
    use tower_lsp::lsp_types::{DiagnosticSeverity, NumberOrString};
    tower_lsp::lsp_types::Diagnostic {
        range: Range {
            start: Position {
                line: d.range.start.line,
                character: d.range.start.character,
            },
            end: Position {
                line: d.range.end.line,
                character: d.range.end.character + 1,
            },
        },
        severity: Some(match d.severity {
            tcl_bigip::validator::DiagSeverity::Warning => DiagnosticSeverity::WARNING,
            tcl_bigip::validator::DiagSeverity::Hint => DiagnosticSeverity::HINT,
        }),
        code: Some(NumberOrString::String(d.code.clone())),
        code_description: None,
        source: Some("tcl-lsp".to_string()),
        message: d.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Whether `code` is suppressed at `line` by an inline `# noqa` or a
/// top-of-file `# tcl-lsp: disable=…` directive — the same `_is_suppressed`
/// contract `tcl_lsp_core::source_style` applies (a `"*"` entry suppresses
/// every code; the file-level `-1` bucket is document-wide).
fn xc_is_suppressed(
    code: &str,
    line: i32,
    suppressed: &std::collections::HashMap<i32, HashSet<String>>,
) -> bool {
    if let Some(file_codes) = suppressed.get(&-1)
        && (file_codes.contains("*") || file_codes.contains(code))
    {
        return true;
    }
    suppressed
        .get(&line)
        .is_some_and(|codes| codes.contains("*") || codes.contains(code))
}

/// FE-DIAG-F5 (opt-in): lift the `f5-xc` XC100-301 translatability
/// diagnostics into LSP diagnostics for an `f5-irules` document — the
/// analogue of the `xc_diagnostics_enabled` block in Python
/// `server/features/diagnostics.py`. Codes the editor disabled
/// (`tclLsp.diagnostics.<CODE> = false`) are filtered, and the same `# noqa`
/// / file-level suppression the analyser honours is applied. `XcSeverity`
/// maps `Hint` → `HINT` and `Info` → `INFORMATION`.
fn lift_xc_diagnostics(
    source: &str,
    disabled: &HashSet<String>,
    suppressed: &std::collections::HashMap<i32, HashSet<String>>,
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    use tower_lsp::lsp_types::{DiagnosticSeverity, NumberOrString};
    f5_xc::get_xc_diagnostics(source)
        .into_iter()
        .filter(|d| !disabled.contains(&d.code))
        .filter(|d| {
            !xc_is_suppressed(
                &d.code,
                i32::try_from(d.range.start.line).unwrap_or(i32::MAX),
                suppressed,
            )
        })
        .map(|d| tower_lsp::lsp_types::Diagnostic {
            range: Range {
                start: Position {
                    line: d.range.start.line,
                    character: d.range.start.character,
                },
                end: Position {
                    line: d.range.end.line,
                    character: d.range.end.character,
                },
            },
            severity: Some(match d.severity {
                f5_xc::XcSeverity::Hint => DiagnosticSeverity::HINT,
                f5_xc::XcSeverity::Info => DiagnosticSeverity::INFORMATION,
            }),
            code: Some(NumberOrString::String(d.code)),
            code_description: None,
            source: Some("tcl-lsp".to_string()),
            message: d.message,
            related_information: None,
            tags: None,
            data: None,
        })
        .collect()
}

/// FE-DIAG-F5 consumer-wiring: BIG-IP config diagnostics (`BIGIP6001`–
/// `BIGIP6011`).  BIG-IP `.conf` text is not Tcl source — it has its own
/// model-level validator
/// ([`tcl_bigip::validator::validate_bigip_source`]), the analogue of the
/// `get_bigip_diagnostics` layer in Python
/// `server/diagnostics_pipeline.py::_publish_bigip_diagnostics`.  Codes the
/// editor disabled via `tclLsp.diagnostics.<CODE> = false` are filtered,
/// mirroring Python's `disabled_codes`.
fn bigip_config_diagnostics(
    text: &str,
    disabled: &HashSet<String>,
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    tcl_bigip::validator::validate_bigip_source(text, BIGIP_DEFAULT_PARTITION)
        .into_iter()
        .filter(|d| !disabled.contains(&d.code))
        .map(|d| lift_config_diagnostic(&d))
        .collect()
}

/// FE-DIAG-F5 consumer-wiring: iApp APL presentation diagnostics
/// (`IAPP7001`–`IAPP7003`).  Parses the APL presentation, optionally
/// cross-checks it against the sibling implementation's
/// `$::section__field` references, and lifts the validator output, the
/// analogue of Python
/// `server/diagnostics_pipeline.py::_publish_apl_diagnostics`.  The
/// validator is gated on the `f5-iapps` dialect (we only reach here for
/// APL sources, so the gate is always satisfied — see [`is_apl_source`]).
fn apl_presentation_diagnostics(
    text: &str,
    impl_var_refs: Option<&[tcl_bigip::apl::IappVarRef]>,
    disabled: &HashSet<String>,
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let model = tcl_bigip::apl::parse_apl(text);
    tcl_bigip::apl::validate_iapp_presentation(&model, impl_var_refs, "f5-iapps")
        .into_iter()
        .filter(|d| !disabled.contains(&d.code))
        .map(|d| lift_config_diagnostic(&d))
        .collect()
}

/// Whether `uri` (with its editor `language_id`) is an iApp APL
/// presentation document.  Mirrors Python
/// `server/diagnostics_pipeline.py::_is_apl_source`: an explicit APL
/// language id, or a basename of `*.apl` / `presentation`.
fn is_apl_source(uri: &Url, language_id: &str) -> bool {
    if matches!(
        language_id.to_ascii_lowercase().as_str(),
        "tcl-apl" | "apl-lang" | "apl"
    ) {
        return true;
    }
    let path = uri.as_str();
    let basename = path.rsplit('/').next().unwrap_or(path);
    if basename.eq_ignore_ascii_case("presentation") {
        return true;
    }
    Path::new(basename)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("apl"))
}

/// Locate the iApp *implementation* that pairs with the APL presentation
/// at `uri` and extract its `$::section__field` variable references,
/// mirroring Python `_find_sibling_impl_vars`.  Prefers an open buffer in
/// the same directory (so unsaved edits to the implementation are
/// reflected), then falls back to reading a sibling from disk.  The sibling
/// is named `implementation` or carries a `.iapp` / `.iappimpl` / `.impl`
/// extension.
async fn find_sibling_impl_vars(
    uri: &Url,
    documents: &Mutex<HashMap<Url, DocumentState>>,
) -> Option<Vec<tcl_bigip::apl::IappVarRef>> {
    let path = uri.to_file_path().ok()?;
    let dir = path.parent()?.to_path_buf();
    let is_impl_name = |p: &Path| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("implementation"))
            || p.extension().and_then(|e| e.to_str()).is_some_and(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "iapp" | "iappimpl" | "impl"
                )
            })
    };

    // 1. Prefer an open implementation buffer in the same directory (the
    //    native analogue of the scanner's open-document index): an unsaved
    //    edit to the implementation must drive the presentation's cross-file
    //    diagnostics, not a stale on-disk copy.
    {
        let docs = documents.lock().await;
        for (doc_uri, doc) in docs.iter() {
            if doc_uri == uri {
                continue;
            }
            let Ok(doc_path) = doc_uri.to_file_path() else {
                continue;
            };
            if doc_path.parent() != Some(dir.as_path()) {
                continue;
            }
            if is_impl_name(&doc_path) {
                return Some(tcl_bigip::apl::extract_iapp_var_refs(&doc.text));
            }
        }
    }

    // 2. Fall back to a sibling on disk — the literal `implementation` file
    //    first, then any `.iapp` / `.iappimpl` / `.impl` file (sorted for a
    //    deterministic pick), matching Python's candidate order.
    let impl_path = dir.join("implementation");
    if impl_path.is_file()
        && let Ok(content) = std::fs::read_to_string(&impl_path)
    {
        return Some(tcl_bigip::apl::extract_iapp_var_refs(&content));
    }
    let mut ext_candidates: Vec<PathBuf> = std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            *p != path
                && p.extension().and_then(|s| s.to_str()).is_some_and(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "iapp" | "iappimpl" | "impl"
                    )
                })
        })
        .collect();
    ext_candidates.sort();
    for cand in ext_candidates {
        if let Ok(content) = std::fs::read_to_string(&cand) {
            return Some(tcl_bigip::apl::extract_iapp_var_refs(&content));
        }
    }
    None
}

/// FE-DIAG-F5 file-type dispatch: if `uri` is a non-Tcl F5 dialect document
/// (BIG-IP config or iApp APL presentation), compute its model-level
/// validator diagnostics; otherwise return `None` so the caller runs the
/// normal Tcl analyser path.  Mirrors the BIG-IP / APL file-type dispatch
/// in Python `server/diagnostics_pipeline.py` (`_is_bigip_conf` /
/// `_is_apl_source` → the specialised publishers).  The validator runs on
/// `spawn_blocking` for the same parser-panic containment the analyser path
/// uses.
async fn f5_dialect_diagnostics(
    uri: &Url,
    text: &str,
    dialect: &str,
    language_id: &str,
    disabled: &HashSet<String>,
    documents: &Mutex<HashMap<Url, DocumentState>>,
) -> Option<Vec<tower_lsp::lsp_types::Diagnostic>> {
    if Backend::is_bigip_dialect(dialect) {
        let (t, dis) = (text.to_owned(), disabled.clone());
        let diags = tokio::task::spawn_blocking(move || bigip_config_diagnostics(&t, &dis))
            .await
            .unwrap_or_default();
        return Some(diags);
    }
    if is_apl_source(uri, language_id) {
        let impl_refs = find_sibling_impl_vars(uri, documents).await;
        let (t, dis) = (text.to_owned(), disabled.clone());
        let diags = tokio::task::spawn_blocking(move || {
            apl_presentation_diagnostics(&t, impl_refs.as_deref(), &dis)
        })
        .await
        .unwrap_or_default();
        return Some(diags);
    }
    None
}

fn lift_analyser_diagnostics(
    text: &str,
    diagnostics: &[tcl_compiler::analyser::Diagnostic],
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let line_index = tcl_lexer::LineIndex::new(text);
    diagnostics
        .iter()
        .filter(|d| !DEFAULT_OFF_CODES.contains(&d.code.as_str()))
        .cloned()
        .map(|d| tower_lsp::lsp_types::Diagnostic {
            range: lift_span(text, &line_index, d.span),
            severity: Some(match d.severity {
                tcl_compiler::analyser::Severity::Error => {
                    tower_lsp::lsp_types::DiagnosticSeverity::ERROR
                }
                tcl_compiler::analyser::Severity::Warning => {
                    tower_lsp::lsp_types::DiagnosticSeverity::WARNING
                }
                tcl_compiler::analyser::Severity::Hint
                | tcl_compiler::analyser::Severity::Suggestion => {
                    tower_lsp::lsp_types::DiagnosticSeverity::HINT
                }
            }),
            code: Some(tower_lsp::lsp_types::NumberOrString::String(d.code)),
            code_description: None,
            source: Some("tcl-lsp".to_string()),
            message: d.message,
            related_information: None,
            tags: None,
            data: None,
        })
        .collect()
}

/// GAP-C1 strip 2: lift the source-style pass (W111 line length,
/// W112 trailing whitespace, W115 comment continuation, W118 line
/// endings) into LSP diagnostics.  These are pure source-text
/// checks (no analyser / compiler unit needed); see
/// `tcl_lsp_core::source_style`.
///
/// `suppressed` is the analyser's `suppressed_lines` map — it
/// carries both inline `# noqa` line suppressions and the
/// file-level (`-1`) `# tcl-lsp: disable=…` directive set, so the
/// style pass honours the same suppression the analyser diagnostics
/// do.  The checks run with the Python defaults (line length 120,
/// expected line ending `\n`); a per-check feature-config surface is
/// a documented GAP-C1 strip-2 follow-up.
fn lift_source_style_diagnostics(
    text: &str,
    suppressed: &std::collections::HashMap<i32, std::collections::HashSet<String>>,
    user_disabled: &std::collections::HashSet<String>,
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    use tcl_lsp_core::source_style::{
        DEFAULT_LINE_ENDING, DEFAULT_LINE_LENGTH, StyleSeverity, style_diagnostics,
    };
    use tower_lsp::lsp_types::{DiagnosticSeverity, NumberOrString};

    // The file-level (`-1`) directive bucket doubles as a per-code
    // disabled set (mirrors how the Rust analyser folds a
    // `# tcl-lsp: disable=…` directive into both `suppressed_lines`
    // and its internal `disabled_diagnostics`); union it with the
    // user's `tclLsp.diagnostics.<CODE> = false` settings so W111 /
    // W112 / W115 / W118 can be turned off from the editor.
    let mut disabled = suppressed.get(&-1).cloned().unwrap_or_default();
    disabled.extend(user_disabled.iter().cloned());

    style_diagnostics(
        text,
        DEFAULT_LINE_LENGTH,
        DEFAULT_LINE_ENDING,
        &disabled,
        suppressed,
    )
    .into_iter()
    .map(|d| tower_lsp::lsp_types::Diagnostic {
        range: Range {
            start: Position {
                line: d.range.start_line,
                character: d.range.start_character,
            },
            end: Position {
                line: d.range.end_line,
                character: d.range.end_character,
            },
        },
        severity: Some(match d.severity {
            StyleSeverity::Warning => DiagnosticSeverity::WARNING,
            StyleSeverity::Hint => DiagnosticSeverity::HINT,
        }),
        code: Some(NumberOrString::String(d.code.to_string())),
        code_description: None,
        source: Some("tcl-lsp".to_string()),
        message: d.message,
        related_information: None,
        tags: None,
        data: None,
    })
    .collect()
}

/// GAP-C1: lift the compiler-checks pipeline (GVN redundancies,
/// shimmer / thunking, taint W2xx / T1xx, iRules control-flow
/// IRULE1xxx-5xxx, SCCP constant branches) **and** the optimiser
/// O-codes into LSP diagnostics.  These analyses are implemented in
/// `tcl-compiler` but were previously only reachable through the
/// `PyO3` bridge — the native server published nothing from them.
///
/// `dialect` is the resolved per-document dialect string (empty for
/// plain Tcl); `registry` must already carry that dialect's specs
/// loaded (the caller resolves it via `registry_for_dialect`).
///
/// Note: this builds a `CompilationUnit` for the checks and the
/// optimiser builds its own internally, so the source is lowered
/// twice.  That is acceptable for the initial wiring (the analyses
/// themselves are the cost); sharing a single lowered unit across
/// both is a follow-up once the document-store lands.
fn lift_compiler_diagnostics(
    text: &str,
    diags: &tcl_lsp_db::CompilerDiagnostics,
    optimiser_enabled: bool,
    disabled_optimisations: &std::collections::HashSet<String>,
    disabled_diagnostics: &std::collections::HashSet<String>,
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    use tcl_compiler::compiler_checks::Severity as CheckSeverity;
    use tower_lsp::lsp_types::{DiagnosticSeverity, NumberOrString};

    let line_index = tcl_lexer::LineIndex::new(text);
    let mut out: Vec<tower_lsp::lsp_types::Diagnostic> = Vec::new();

    // Compiler checks: GVN / shimmer / thunking / taint / iRules-flow / SCCP,
    // all keyed off a single interprocedurally-summarised compilation unit whose
    // per-procedure lattices are memoised by the salsa-native `function_lattice`
    // query (so an unchanged procedure is built once and reused across edits
    // *and* shared with the analyser tail).  The unit is built by the
    // `compiler_check_diagnostics` query; here we only filter + lift.
    // An optimiser O-code (`O1xx`) is gated by the `tclLsp.optimiser.enabled`
    // master switch and the profile + per-code `disabled_optimisations` set,
    // wherever it is emitted (some — e.g. the constant-branch `O100` — come
    // from `run_all_checks` rather than `optimise_with_dialect`). Mirrors
    // Python, whose `optimiser_enabled` gate covers every O-code.
    let optimiser_suppressed = |code: &str| {
        code.starts_with('O') && (!optimiser_enabled || disabled_optimisations.contains(code))
    };
    for d in &diags.checks {
        let d = d.clone();
        if optimiser_suppressed(&d.code) {
            continue;
        }
        // GAP-C1: per-check feature toggle (`tclLsp.diagnostics.<CODE> = false`).
        // The analyser path bakes the disabled set into its build, but the
        // compiler-checks (S1xx shimmer, T1xx / W2xx taint, IRULE1xxx-5xxx flow,
        // GVN, SCCP constant-branch) come through this separate lift, so the
        // toggle must be applied here too — mirrors the uniform
        // `d.code in disabled_diagnostics` filter in
        // `server/features/diagnostics.py`.
        if disabled_diagnostics.contains(&d.code) {
            continue;
        }
        out.push(tower_lsp::lsp_types::Diagnostic {
            range: lift_span(text, &line_index, d.span),
            severity: Some(match d.severity {
                CheckSeverity::Error => DiagnosticSeverity::ERROR,
                CheckSeverity::Warning => DiagnosticSeverity::WARNING,
                CheckSeverity::Info => DiagnosticSeverity::INFORMATION,
                CheckSeverity::Hint | CheckSeverity::Suggestion => DiagnosticSeverity::HINT,
            }),
            code: Some(NumberOrString::String(d.code)),
            code_description: None,
            source: Some("tcl-lsp".to_string()),
            message: d.message,
            related_information: None,
            tags: None,
            data: None,
        });
    }

    // Optimiser O-codes — actionable + hint-only rewrites, surfaced
    // as HINT-severity suggestions (the editor renders the code-action
    // fix from the diagnostic; the fix plumbing itself is GAP-C3). The
    // `tclLsp.optimiser.enabled=false` master switch suppresses the whole
    // block, mirroring Python's `if optimiser_enabled:` gate.
    for o in diags
        .optimisations
        .iter()
        .filter(|_| optimiser_enabled)
        .cloned()
    {
        // Profile + per-code gate: the active profile disables whole O-code
        // categories (the default `readability` profile surfaces only
        // readability rewrites) and per-code `tclLsp.optimiser.O1xx=false`
        // overrides add to that. Mirrors the Python server's
        // `disabled_optimisations` filter.
        if disabled_optimisations.contains(o.code.as_str()) {
            continue;
        }
        // Surface the fold/rewrite text as the quick-fix `data.replacement`
        // (mirrors Python's `Optimisation.replacement` -> diagnostic data), so
        // editors and the e2e battery can apply the suggested replacement.
        let data = (!o.replacement.is_empty()).then(|| {
            serde_json::json!({
                "replacement": o.replacement,
                "startOffset": o.span.start(),
                "endOffset": o.span.end(),
            })
        });
        out.push(tower_lsp::lsp_types::Diagnostic {
            range: lift_span(text, &line_index, o.span),
            severity: Some(DiagnosticSeverity::HINT),
            code: Some(NumberOrString::String(o.code)),
            code_description: None,
            source: Some("tcl-lsp".to_string()),
            message: o.message,
            related_information: None,
            tags: None,
            data,
        });
    }

    out
}

/// Build an empty `DocumentDiagnosticReportResult` for callers
/// that hit a no-document or no-analysis path.
fn empty_diagnostic_report() -> DocumentDiagnosticReportResult {
    DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
        RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id: None,
                items: Vec::new(),
            },
        },
    ))
}

/// Resolve the per-folder dialect override for the given URI. The
/// longest matching folder prefix wins so a nested folder mapping
/// shadows its parent. Returns `None` when no folder covers the
/// URI, in which case the caller falls back to the session
/// default.
fn folder_dialect_for(uri: &Url, folders: &[(Url, String)]) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    let target = uri.as_str();
    for (folder, dialect) in folders {
        if !uri_under_folder(target, folder.as_str()) {
            continue;
        }
        let len = folder.as_str().len();
        let take = match best {
            Some((existing, _)) => len > existing,
            None => true,
        };
        if take {
            best = Some((len, dialect.as_str()));
        }
    }
    best.map(|(_, d)| d.to_owned())
}

/// Upper bound on the number of files the on-disk workspace scan
/// will analyse, so a pathologically large tree can't stall
/// start-up.  Open documents are always indexed regardless of
/// this cap (they flow through `publish_analyser_diagnostics`).
const WORKSPACE_SCAN_FILE_CAP: usize = 2000;

/// `true` when `path` names a directory the workspace scan should
/// not descend into: hidden directories (dot-prefixed) and the
/// common vendor / build / scratch trees that never hold
/// first-party Tcl sources worth indexing.
fn is_skipped_scan_dir(path: &Path) -> bool {
    match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.starts_with('.') || matches!(name, "node_modules" | "target" | "tmp"),
        // No representable file name (e.g. `..`): skip to be safe.
        None => true,
    }
}

/// `true` when `path` has a Tcl-family source extension the analyser
/// can usefully index.  Mirrors the Python scanner's `TCL_EXTENSIONS`
/// (`server/workspace/scanner.py`) so the startup workspace scan picks
/// up the same unopened files — otherwise cross-document definition /
/// references / rename / call-hierarchy / workspace-symbols miss
/// definitions that live in `.itcl`/`.irule`/`.iapp`/… files until they
/// are opened.
fn is_tcl_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some(
            "tcl"
                | "tk"
                | "itcl"
                | "tm"
                | "irul"
                | "irule"
                | "iapp"
                | "iappimpl"
                | "impl"
                | "exp"
                | "apl"
        )
    )
}

/// Iteratively walk `root`, appending the paths of Tcl source
/// files to `out` until `out` reaches `cap` entries.  Skips the
/// directories [`is_skipped_scan_dir`] rejects and silently
/// ignores unreadable directories so a single permission error
/// doesn't abort the whole scan.  Iterative (explicit stack) to
/// avoid unbounded recursion on deep trees.
fn collect_tcl_files(root: &Path, cap: usize, out: &mut Vec<PathBuf>) {
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if out.len() >= cap {
            return;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if file_type.is_dir() {
                if !is_skipped_scan_dir(&path) {
                    stack.push(path);
                }
            } else if file_type.is_file() && is_tcl_source(&path) {
                if out.len() >= cap {
                    return;
                }
                out.push(path);
            }
        }
    }
}

/// `true` when `uri` sits under the workspace folder `folder`.
///
/// Performs the directory-boundary check the prefix-only
/// `target.starts_with(folder)` form gets wrong — without
/// the boundary check `file:///workspace/app` would also
/// match `file:///workspace/app2/...` (PR #454 Codex review P2).
/// The match succeeds when:
///
/// * `uri == folder`, OR
/// * `uri == folder` with the folder's trailing slash trimmed, OR
/// * `uri` starts with `folder + "/"`.
fn uri_under_folder(uri: &str, folder: &str) -> bool {
    let folder_trimmed = folder.trim_end_matches('/');
    if uri == folder_trimmed || uri == folder {
        return true;
    }
    // The boundary check: the byte immediately after the
    // folder prefix in `uri` must be `/`.  Compare against the
    // trimmed form so a trailing slash on `folder` doesn't
    // change the behaviour.
    let needle = format!("{folder_trimmed}/");
    uri.starts_with(&needle)
}

/// Return a fresh `result_id` for a semantic-tokens response.
/// Monotonically increasing via an atomic counter so each
/// snapshot we hand out is uniquely identifiable, letting
/// clients consult our cache via
/// `semanticTokens/full/delta { previousResultId }`.
/// Negotiate the position encoding the server will use against the
/// client's advertised `general.positionEncodings`.
///
/// Every provider in this server computes columns in UTF-16 code units
/// (via [`tcl_lexer::LineIndex::position_at_utf16`]), which is also the
/// LSP default — so we always operate in UTF-16 and only advertise it
/// explicitly when the client listed it (declaring an encoding the client
/// didn't offer is a protocol violation; omitting the field falls back to
/// the UTF-16 default either way).
fn negotiate_position_encoding(params: &InitializeParams) -> Option<PositionEncodingKind> {
    let supported = params
        .capabilities
        .general
        .as_ref()
        .and_then(|g| g.position_encodings.as_ref())?;
    supported
        .contains(&PositionEncodingKind::UTF16)
        .then_some(PositionEncodingKind::UTF16)
}

/// True when the client advertised a non-empty `general.positionEncodings`
/// list that omits UTF-16.
///
/// This is the one case the server cannot satisfy cleanly: every provider
/// emits UTF-16 columns, so a client that explicitly excludes UTF-16 would
/// mis-map any non-ASCII range.  The server still proceeds in UTF-16 (the
/// LSP default for an omitted server `positionEncoding`) rather than
/// refusing the session — UTF-16 is the universal LSP baseline and no
/// real client ships without it — but [`Backend::initialize`] logs a
/// warning so the mismatch is not silent.
fn client_lacks_utf16_support(params: &InitializeParams) -> bool {
    params
        .capabilities
        .general
        .as_ref()
        .and_then(|g| g.position_encodings.as_ref())
        .is_some_and(|encs| !encs.is_empty() && !encs.contains(&PositionEncodingKind::UTF16))
}

/// Build the `ServerCapabilities` advertised in the response
/// to `initialize`.  Kept as a free function so the
/// `LanguageServer::initialize` handler stays focused on
/// state setup and result construction — the long capability
/// literal lives here rather than inside the trait method.
fn build_server_capabilities(
    position_encoding: Option<PositionEncodingKind>,
) -> ServerCapabilities {
    ServerCapabilities {
        // UTF-16 columns, matching the LSP default and what every provider
        // emits.  Advertised explicitly only when the client offered it
        // (see `negotiate_position_encoding`).
        position_encoding,
        // Options form (rather than the bare `Kind`) so we can advertise
        // `willSaveWaitUntil` alongside incremental sync. The handler is
        // gated by `tclLsp.features.willSaveWaitUntil` (default off).
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::INCREMENTAL),
                will_save: Some(false),
                will_save_wait_until: Some(true),
                save: None,
            },
        )),
        folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["$".to_owned()]),
            ..CompletionOptions::default()
        }),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec![" ".to_owned()]),
            retrigger_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        definition_provider: Some(OneOf::Left(true)),
        declaration_provider: Some(DeclarationCapability::Simple(true)),
        type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
        implementation_provider: Some(ImplementationProviderCapability::Simple(true)),
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: Some(OneOf::Left(true)),
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
        document_formatting_provider: Some(OneOf::Left(true)),
        document_range_formatting_provider: Some(OneOf::Left(true)),
        document_link_provider: Some(DocumentLinkOptions {
            resolve_provider: Some(false),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        inlay_hint_provider: Some(OneOf::Left(true)),
        code_lens_provider: Some(CodeLensOptions {
            resolve_provider: Some(true),
        }),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        call_hierarchy_provider: Some(CallHierarchyServerCapability::Simple(true)),
        // Note: tower-lsp 0.20 does not expose
        // `type_hierarchy_provider` on `ServerCapabilities`;
        // the type-hierarchy core provider lives in
        // `tcl-lsp-core::type_hierarchy` and will be wired
        // once we upgrade tower-lsp (tracked under
        // `S-type-hierarchy-rich`).
        semantic_tokens_provider: Some(semantic_tokens_capability()),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        linked_editing_range_provider: Some(
            tower_lsp::lsp_types::LinkedEditingRangeServerCapabilities::Simple(true),
        ),
        // `S-diagnostics-pipeline`: advertise pull-based
        // diagnostics so editors that support it can request
        // diagnostics on demand alongside the existing
        // push-based `publish_diagnostics`.
        diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
            identifier: Some("tcl-lsp".to_string()),
            inter_file_dependencies: false,
            // Workspace pull is backed by the per-URI pull-diagnostic cache the
            // push pipeline maintains (`workspace_diagnostic` reports each open
            // document's last-published set).
            workspace_diagnostics: true,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),
        // Editor-invoked workspace commands (currently the
        // minify-document command family).
        execute_command_provider: Some(ExecuteCommandOptions {
            commands: vec![
                "tcl-lsp.minifyDocument".to_owned(),
                "tcl-lsp.optimiseDocument".to_owned(),
                "tcl-lsp.unminifyError".to_owned(),
                "tcl-lsp.describeIruleEvent".to_owned(),
                "tcl-lsp.describeIruleCommand".to_owned(),
                "tcl-lsp.listIruleEvents".to_owned(),
                "tcl-lsp.diagramData".to_owned(),
                "tcl-lsp.getEffectiveConfig".to_owned(),
                "tcl-lsp.fixAllSafeIssues".to_owned(),
                "tcl-lsp.listSubcommands".to_owned(),
                "tcl-lsp.listKnownPackages".to_owned(),
                "tcl-lsp.suggestPackagesForSymbol".to_owned(),
                "tcl-lsp.exportConfig".to_owned(),
            ],
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        // `S-workspace-file-ops` (GAP-A9): advertise willRename /
        // didRename so the editor consults us before/after a `.tcl`
        // rename — the willRename handler rewrites dependents' `source`
        // literals, the didRename handler reindexes the moved file.
        workspace: Some(WorkspaceServerCapabilities {
            // Advertise multi-root support and ask the client to send
            // `workspace/didChangeWorkspaceFolders` so folders added or removed
            // mid-session update the index and configuration (the
            // `did_change_workspace_folders` handler), not just at startup.
            workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                supported: Some(true),
                change_notifications: Some(OneOf::Left(true)),
            }),
            file_operations: Some(WorkspaceFileOperationsServerCapabilities {
                will_rename: Some(rename_file_operation_options()),
                did_rename: Some(rename_file_operation_options()),
                ..WorkspaceFileOperationsServerCapabilities::default()
            }),
        }),
        ..ServerCapabilities::default()
    }
}

/// File-operation filter for `willRename` / `didRename`: match the Tcl
/// source extensions (`.tcl` / `.tm` / `.itcl` / `.irule` / `.irul`),
/// mirroring the Python `_RENAME_FILE_OPERATION_OPTIONS` glob.
fn rename_file_operation_options() -> FileOperationRegistrationOptions {
    FileOperationRegistrationOptions {
        filters: vec![FileOperationFilter {
            scheme: Some("file".to_owned()),
            pattern: FileOperationPattern {
                glob: "**/*.{tcl,tm,itcl,irule,irul}".to_owned(),
                matches: None,
                options: None,
            },
        }],
    }
}

/// Build the semantic-tokens capability advertising the
/// classifier's legend, range support, and delta support.
fn semantic_tokens_capability() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions::default(),
        legend: SemanticTokensLegend {
            token_types: core_semantic_tokens::legend_token_types()
                .into_iter()
                .map(tower_lsp::lsp_types::SemanticTokenType::new)
                .collect(),
            token_modifiers: core_semantic_tokens::legend_token_modifiers()
                .into_iter()
                .map(tower_lsp::lsp_types::SemanticTokenModifier::new)
                .collect(),
        },
        range: Some(true),
        // Delta support — the handler implements the minimal
        // valid shape (returns full tokens when
        // previousResultId doesn't match, otherwise empty
        // edits).
        full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
    })
}

fn next_semantic_tokens_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("st-{id}")
}

/// Convert the core's `Vec<u32>` packed semantic-tokens
/// stream (`[deltaLine, deltaCol, length, type, modifiers]`
/// per token) into a `Vec<SemanticToken>` for the wire layer.
fn lift_semantic_token_data(data: &[u32]) -> Vec<tower_lsp::lsp_types::SemanticToken> {
    let mut tokens: Vec<tower_lsp::lsp_types::SemanticToken> = Vec::with_capacity(data.len() / 5);
    for chunk in data.chunks_exact(5) {
        tokens.push(tower_lsp::lsp_types::SemanticToken {
            delta_line: chunk[0],
            delta_start: chunk[1],
            length: chunk[2],
            token_type: chunk[3],
            token_modifiers_bitset: chunk[4],
        });
    }
    tokens
}

fn lift_parameter_information(p: CoreParameterInformation) -> ParameterInformation {
    ParameterInformation {
        label: ParameterLabel::Simple(p.label),
        documentation: None,
    }
}

fn lift_signature_information(s: CoreSignatureInformation) -> SignatureInformation {
    SignatureInformation {
        label: s.label,
        documentation: s.documentation.map(|d| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: d,
            })
        }),
        parameters: Some(
            s.parameters
                .into_iter()
                .map(lift_parameter_information)
                .collect(),
        ),
        active_parameter: None,
    }
}

fn lift_signature_help(h: CoreSignatureHelp) -> SignatureHelp {
    SignatureHelp {
        signatures: h
            .signatures
            .into_iter()
            .map(lift_signature_information)
            .collect(),
        active_signature: Some(h.active_signature),
        active_parameter: Some(h.active_parameter),
    }
}

fn lift_hover(h: CoreHover) -> Hover {
    let kind = match h.kind {
        CoreHoverKind::Markdown => MarkupKind::Markdown,
    };
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind,
            value: h.value,
        }),
        range: None,
    }
}

fn lift_line_range(r: core_symbols::LineRange) -> Range {
    Range {
        start: Position {
            line: r.start_line,
            character: r.start_character,
        },
        end: Position {
            line: r.end_line,
            character: r.end_character,
        },
    }
}

fn lift_symbol_kind(k: CoreSymbolKind) -> SymbolKind {
    match k {
        CoreSymbolKind::Function => SymbolKind::FUNCTION,
        CoreSymbolKind::Method => SymbolKind::METHOD,
        CoreSymbolKind::Class => SymbolKind::CLASS,
        CoreSymbolKind::Property => SymbolKind::PROPERTY,
        CoreSymbolKind::Constructor => SymbolKind::CONSTRUCTOR,
        CoreSymbolKind::Namespace => SymbolKind::NAMESPACE,
        CoreSymbolKind::Variable => SymbolKind::VARIABLE,
        CoreSymbolKind::Module => SymbolKind::MODULE,
    }
}

fn lift_document_symbol(s: core_symbols::DocumentSymbol) -> DocumentSymbol {
    let children = if s.children.is_empty() {
        None
    } else {
        Some(s.children.into_iter().map(lift_document_symbol).collect())
    };
    #[allow(deprecated)] // `deprecated` is required by lsp-types but unused.
    DocumentSymbol {
        name: s.name,
        detail: s.detail,
        kind: lift_symbol_kind(s.kind),
        tags: None,
        deprecated: None,
        range: lift_line_range(s.range),
        selection_range: lift_line_range(s.selection_range),
        children,
    }
}

fn lift_folding_range(r: tcl_lsp_core::folding::FoldingRange) -> FoldingRange {
    FoldingRange {
        start_line: r.start_line,
        start_character: None,
        end_line: r.end_line,
        end_character: None,
        kind: Some(match r.kind {
            FoldKind::Region => FoldingRangeKind::Region,
            FoldKind::Comment => FoldingRangeKind::Comment,
        }),
        collapsed_text: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::{
        PartialResultParams, ReferenceContext, TextDocumentIdentifier, WorkDoneProgressParams,
    };

    #[test]
    fn default_off_w242_is_not_published() {
        // `while {$x < 10} {puts hi}` emits the default-off W242 hint
        // from the analyser, which the lift layer must drop (no consuming
        // config exists in the Rust server) while keeping other codes.
        let src = "while {$x < 10} {puts hi}\n";
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        assert!(
            analysis.diagnostics.iter().any(|d| d.code == "W242"),
            "analyser should still emit W242"
        );
        let lifted = lift_analyser_diagnostics(src, &analysis.diagnostics);
        assert!(
            !lifted.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "W242"
            )),
            "W242 must be filtered from the published set"
        );
    }

    /// GAP-C1: the constant-true `if` is folded by SCCP and surfaced
    /// as an `O100` constant-branch diagnostic from `run_all_checks`.
    /// The base analyser never emits it, so a non-empty result with an
    /// O-code proves `lift_compiler_diagnostics` carries the compiler-
    /// check pipeline's output into the published diagnostic set.
    #[test]
    fn lift_compiler_diagnostics_surfaces_compiler_check_codes() {
        let registry = CommandRegistry::build_default();
        let src = "if {1} { set x 1 } else { set y 2 }\n";
        let diags = lift_compiler_diagnostics(
            src,
            &tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, ""),
            true,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
        );
        assert!(
            diags.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "O100"
            )),
            "expected an O100 constant-branch diagnostic, got: {:?}",
            diags.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
        // Every lifted diagnostic carries the shared source tag.
        assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));
    }

    #[test]
    fn lift_compiler_diagnostics_honours_optimiser_master_switch_and_per_code() {
        let registry = CommandRegistry::build_default();
        let src = "if {1} { set x 1 } else { set y 2 }\n";
        let is_o100 = |d: &tower_lsp::lsp_types::Diagnostic| matches!(&d.code, Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "O100");
        // Master switch off: no optimiser O-codes at all (compiler checks still run).
        let off = lift_compiler_diagnostics(
            src,
            &tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, ""),
            false,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
        );
        assert!(
            !off.iter().any(is_o100),
            "O100 must be suppressed when the optimiser master switch is off: {:?}",
            off.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
        // Per-code disable: O100 specifically suppressed even with the optimiser on.
        let mut disabled = std::collections::HashSet::new();
        disabled.insert("O100".to_string());
        let per_code = lift_compiler_diagnostics(
            src,
            &tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, ""),
            true,
            &disabled,
            &std::collections::HashSet::new(),
        );
        assert!(
            !per_code.iter().any(is_o100),
            "O100 must be suppressed when disabled per-code: {:?}",
            per_code.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
    }

    /// An iRules taint flow (`HTTP::uri` → `HTTP::respond`) is an
    /// IRULE3001 the base analyser does not emit; the dialect-aware
    /// registry path must surface it through `lift_compiler_diagnostics`.
    #[test]
    fn lift_compiler_diagnostics_surfaces_irules_taint_flow() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let src = "set u [HTTP::uri]\nHTTP::respond 200 content $u\n";
        let cdiags = tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, "f5-irules");
        let diags = lift_compiler_diagnostics(
            src,
            &cdiags,
            true,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
        );
        assert!(
            diags.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "IRULE3001"
            )),
            "expected IRULE3001, got: {:?}",
            diags.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
    }

    /// GAP-C1: a per-check feature toggle (`tclLsp.diagnostics.<CODE> = false`)
    /// must suppress a compiler-*check* code — not just the analyser families.
    /// IRULE3001 comes through the compiler-checks lift, so disabling it via the
    /// `disabled_diagnostics` set must drop it from the published set while
    /// leaving other codes untouched.
    #[test]
    fn lift_compiler_diagnostics_honours_per_check_disable() {
        let mut registry = CommandRegistry::build_default();
        registry.load_irules();
        let src = "set u [HTTP::uri]\nHTTP::respond 200 content $u\n";
        let cdiags = tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, "f5-irules");
        let is_irule3001 = |d: &tower_lsp::lsp_types::Diagnostic| matches!(&d.code, Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "IRULE3001");
        // Baseline: IRULE3001 is present with no disabled codes.
        let baseline = lift_compiler_diagnostics(
            src,
            &cdiags,
            true,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
        );
        assert!(
            baseline.iter().any(is_irule3001),
            "expected IRULE3001 baseline"
        );
        // Disable IRULE3001 via the per-check toggle: it must disappear.
        let mut disabled_diagnostics = std::collections::HashSet::new();
        disabled_diagnostics.insert("IRULE3001".to_string());
        let filtered = lift_compiler_diagnostics(
            src,
            &cdiags,
            true,
            &std::collections::HashSet::new(),
            &disabled_diagnostics,
        );
        assert!(
            !filtered.iter().any(is_irule3001),
            "IRULE3001 must be suppressed when disabled per-check: {:?}",
            filtered.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
    }

    fn pos(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    #[test]
    fn parse_non_ascii_mode_maps_settings() {
        assert_eq!(parse_non_ascii_mode("off"), NonAsciiMode::Off);
        assert_eq!(parse_non_ascii_mode("strict"), NonAsciiMode::Strict);
        assert_eq!(
            parse_non_ascii_mode("confusables"),
            NonAsciiMode::Confusables
        );
        assert_eq!(parse_non_ascii_mode("common"), NonAsciiMode::Common);
        assert_eq!(parse_non_ascii_mode("bogus"), NonAsciiMode::Default);
    }

    #[test]
    fn settings_non_ascii_mode_nested_and_flat() {
        let nested = serde_json::json!({"tclLsp": {"style": {"nonAscii": "common"}}});
        assert_eq!(settings_non_ascii_mode(&nested), Some(NonAsciiMode::Common));
        let flat = serde_json::json!({"tclLsp.style.nonAscii": "off"});
        assert_eq!(settings_non_ascii_mode(&flat), Some(NonAsciiMode::Off));
        let none = serde_json::json!({"tclLsp": {"dialect": "tcl9.0"}});
        assert_eq!(settings_non_ascii_mode(&none), None);
    }

    #[test]
    fn settings_disabled_diagnostics_nested_and_flat() {
        let nested = serde_json::json!({
            "tclLsp": {"diagnostics": {"W001": true, "W108": false, "W111": false}}
        });
        let got = settings_disabled_diagnostics(&nested).unwrap();
        assert!(got.contains("W108") && got.contains("W111") && !got.contains("W001"));
        let flat = serde_json::json!({
            "tclLsp.diagnostics.W210": false, "tclLsp.diagnostics.W211": true
        });
        let got = settings_disabled_diagnostics(&flat).unwrap();
        assert!(got.contains("W210") && !got.contains("W211"));
        // No diagnostics config -> None (leave current set untouched).
        assert!(settings_disabled_diagnostics(&serde_json::json!({"x": 1})).is_none());
    }

    #[test]
    fn position_encoding_negotiation() {
        use tower_lsp::lsp_types::{ClientCapabilities, GeneralClientCapabilities};
        let params_with = |encs: Option<Vec<PositionEncodingKind>>| InitializeParams {
            capabilities: ClientCapabilities {
                general: Some(GeneralClientCapabilities {
                    position_encodings: encs,
                    ..GeneralClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        };
        let with_encodings = |encs: Option<Vec<PositionEncodingKind>>| {
            negotiate_position_encoding(&params_with(encs))
        };
        // Client offers UTF-16 → advertise it explicitly.
        assert_eq!(
            with_encodings(Some(vec![PositionEncodingKind::UTF16])),
            Some(PositionEncodingKind::UTF16)
        );
        // Client lists only UTF-8 → don't advertise an encoding it can't
        // honour; omitting it falls back to the UTF-16 default.
        assert_eq!(with_encodings(Some(vec![PositionEncodingKind::UTF8])), None);
        // No `general` capability at all → omit (default UTF-16).
        assert_eq!(
            negotiate_position_encoding(&InitializeParams::default()),
            None
        );

        // `client_lacks_utf16_support`: only true for a non-empty list that
        // omits UTF-16 — the case `initialize` warns about.
        assert!(client_lacks_utf16_support(&params_with(Some(vec![
            PositionEncodingKind::UTF8
        ]))));
        assert!(client_lacks_utf16_support(&params_with(Some(vec![
            PositionEncodingKind::UTF8,
            PositionEncodingKind::UTF32,
        ]))));
        // A list containing UTF-16, an empty list, and no list at all are all
        // fine — UTF-16 is available (or defaulted) so no warning fires.
        assert!(!client_lacks_utf16_support(&params_with(Some(vec![
            PositionEncodingKind::UTF8,
            PositionEncodingKind::UTF16,
        ]))));
        assert!(!client_lacks_utf16_support(&params_with(Some(vec![]))));
        assert!(!client_lacks_utf16_support(&InitializeParams::default()));
    }

    #[test]
    fn feature_toggles_default_on_and_merge() {
        let mut toggles = FeatureToggles::default();
        // Absent keys default to enabled.
        assert!(toggles.is_enabled("hover"));
        assert!(toggles.is_enabled("completion"));
        // Applying a partial `features` object sets only the named keys.
        let features = serde_json::json!({"hover": false, "folding": false});
        toggles.apply(features.as_object().unwrap());
        assert!(!toggles.is_enabled("hover"));
        assert!(!toggles.is_enabled("folding"));
        // Unnamed keys keep their default-on value.
        assert!(toggles.is_enabled("completion"));
        // Re-enabling merges (does not reset the rest).
        toggles.apply(serde_json::json!({"hover": true}).as_object().unwrap());
        assert!(toggles.is_enabled("hover"));
        assert!(!toggles.is_enabled("folding"));
    }

    #[test]
    fn inlay_keys_default_off() {
        // The opt-in inlay families default off, so `getEffectiveConfig`
        // reports them disabled until an editor sets them (matching the
        // handler's default-off gate; PR #646 review).
        let toggles = FeatureToggles::default();
        assert!(!toggles.is_enabled("inlayTypeHints"));
        assert!(!toggles.is_enabled("inlayParameterHints"));
        let map = toggles.resolved_map();
        assert_eq!(
            map.get("inlayTypeHints")
                .and_then(serde_json::Value::as_bool),
            Some(false)
        );
        assert_eq!(
            map.get("inlayParameterHints")
                .and_then(serde_json::Value::as_bool),
            Some(false),
        );
    }

    #[test]
    fn legacy_inlay_hints_key_maps_to_type_family() {
        // The retired `inlayHints` key maps to `inlayTypeHints` (matching the
        // Python server): an existing explicit opt-in keeps showing the
        // inferred-variable-type hints after the rename, while the verbose
        // parameter-name hints stay off.
        let mut toggles = FeatureToggles::default();
        toggles.apply(serde_json::json!({"inlayHints": true}).as_object().unwrap());
        assert!(toggles.is_enabled("inlayTypeHints"));
        // It does not touch the parameter-name family.
        assert!(!toggles.is_enabled("inlayParameterHints"));
    }

    #[test]
    fn explicit_new_inlay_key_wins_over_legacy_alias() {
        // When a config carries both, the explicit new `inlayTypeHints` key
        // takes precedence over the `inlayHints` alias.
        let mut toggles = FeatureToggles::default();
        toggles.apply(
            serde_json::json!({"inlayHints": true, "inlayTypeHints": false})
                .as_object()
                .unwrap(),
        );
        assert!(!toggles.is_enabled("inlayTypeHints"));
    }

    #[test]
    fn feature_toggles_resolved_map_covers_known_keys() {
        let mut toggles = FeatureToggles::default();
        toggles.apply(serde_json::json!({"hover": false}).as_object().unwrap());
        let map = toggles.resolved_map();
        // Every advertised key is present and boolean.  Default-on keys
        // report `true` (except the one we disabled); the opt-in
        // `DEFAULT_OFF` inlay keys report `false`.
        for key in FeatureToggles::KEYS {
            let expected = *key != "hover" && !FeatureToggles::DEFAULT_OFF.contains(key);
            assert_eq!(
                map.get(*key).and_then(serde_json::Value::as_bool),
                Some(expected),
                "feature {key}"
            );
        }
        // The toggleable keys the e2e contract checks are all reported.
        for key in [
            "hover",
            "completion",
            "documentSymbols",
            "definition",
            "references",
            "signatureHelp",
            "folding",
            "selectionRange",
            "documentLinks",
        ] {
            assert!(map.contains_key(key), "missing reported feature {key}");
        }
    }

    /// The shipped `tclLsp.xcDiagnostics.enabled` setting is a dedicated
    /// config *section*, not a `features.*` key — `parse_folder_config` must
    /// still map it onto the `xcDiagnostics` feature toggle so the advertised
    /// VS Code opt-in actually reaches `xc_diagnostics_enabled` (Codex #689
    /// P2).
    #[test]
    fn folder_config_maps_xc_diagnostics_section_to_toggle() {
        let cfg = serde_json::json!({ "xcDiagnostics": { "enabled": true } });
        let fc = parse_folder_config(&cfg).expect("folder config");
        assert_eq!(
            fc.feature_toggles.set.get("xcDiagnostics").copied(),
            Some(true),
            "tclLsp.xcDiagnostics.enabled must set the xcDiagnostics toggle",
        );
        assert!(fc.feature_toggles.is_enabled_default_off("xcDiagnostics"));

        // Absent section leaves the (default-off) toggle unset.
        let empty = parse_folder_config(&serde_json::json!({})).expect("folder config");
        assert_eq!(empty.feature_toggles.set.get("xcDiagnostics"), None);
        assert!(
            !empty
                .feature_toggles
                .is_enabled_default_off("xcDiagnostics")
        );
    }

    #[test]
    fn non_ascii_mode_str_renders_null_for_default() {
        assert!(non_ascii_mode_str(NonAsciiMode::Default).is_null());
        assert_eq!(non_ascii_mode_str(NonAsciiMode::Off), "off");
        assert_eq!(non_ascii_mode_str(NonAsciiMode::Strict), "strict");
        assert_eq!(non_ascii_mode_str(NonAsciiMode::Common), "common");
        assert_eq!(non_ascii_mode_str(NonAsciiMode::Confusables), "confusables");
    }

    #[test]
    fn suggest_packages_echoes_symbol_with_list() {
        let args = vec![serde_json::json!("json::write")];
        let out = Backend::suggest_packages_for_symbol_command(&args);
        assert_eq!(
            out.get("symbol").and_then(|v| v.as_str()),
            Some("json::write")
        );
        assert!(
            out.get("suggestions")
                .is_some_and(serde_json::Value::is_array)
        );
    }

    #[test]
    fn list_known_packages_reports_a_list() {
        let out = Backend::list_known_packages_command();
        assert!(out.get("packages").is_some_and(serde_json::Value::is_array));
    }

    #[test]
    fn config_ini_path_honours_xdg() {
        // `temp_env` sets + restores `XDG_CONFIG_HOME` and serialises against
        // other env-touching tests via its own global lock, so this crate stays
        // unsafe-free (`set_var` is `unsafe` on edition 2024).
        temp_env::with_var("XDG_CONFIG_HOME", Some("/tmp/xdg-probe"), || {
            assert_eq!(
                config_ini_path(),
                std::path::PathBuf::from("/tmp/xdg-probe/tcl-lsp/config.ini")
            );
        });
    }

    #[test]
    fn formatter_config_honours_request_indent() {
        let mut opts = tower_lsp::lsp_types::FormattingOptions {
            tab_size: 2,
            insert_spaces: true,
            ..Default::default()
        };
        let cfg = formatter_config_from_options(&opts);
        assert_eq!(cfg.indent_size, 2);
        assert_eq!(cfg.indent_style, core_formatting::IndentStyle::Spaces);
        // A zero tabSize is clamped to a usable minimum.
        opts.tab_size = 0;
        assert_eq!(formatter_config_from_options(&opts).indent_size, 1);
        // insertSpaces=false selects tab indentation.
        opts.tab_size = 4;
        opts.insert_spaces = false;
        let cfg = formatter_config_from_options(&opts);
        assert_eq!(cfg.indent_style, core_formatting::IndentStyle::Tabs);
    }

    #[test]
    fn configured_analyser_threads_mode_and_disabled() {
        // `Off` mode suppresses W108 entirely.
        let mut a = Backend::configured_analyser(HashSet::new(), NonAsciiMode::Off);
        let r = a.analyse("set x \u{201c}hi\u{201d}\n", "tcl8.6");
        assert!(!r.diagnostics.iter().any(|d| d.code == "W108"));
        // A disabled code is filtered from the analyser's output.
        let mut disabled = HashSet::new();
        disabled.insert("W108".to_string());
        let mut b = Backend::configured_analyser(disabled, NonAsciiMode::Strict);
        let r = b.analyse("set x \u{201c}hi\u{201d}\n", "tcl8.6");
        assert!(!r.diagnostics.iter().any(|d| d.code == "W108"));
    }

    #[test]
    fn apply_content_change_full_replacement() {
        // A `None` range replaces the whole document.
        assert_eq!(apply_content_change("old text", None, "new"), "new");
    }

    #[test]
    fn apply_content_change_ranged_edits() {
        // Replace "hi" with "bye" on line 1.
        let text = "set x 1\nputs hi\n";
        let r = Range {
            start: pos(1, 5),
            end: pos(1, 7),
        };
        assert_eq!(
            apply_content_change(text, Some(r), "bye"),
            "set x 1\nputs bye\n"
        );

        // Pure insertion (empty range) at the start of line 1.
        let r = Range {
            start: pos(1, 0),
            end: pos(1, 0),
        };
        assert_eq!(
            apply_content_change(text, Some(r), "# "),
            "set x 1\n# puts hi\n",
        );

        // Multi-line deletion: drop from mid-line-0 through mid-line-1.
        let r = Range {
            start: pos(0, 4),
            end: pos(1, 5),
        };
        assert_eq!(apply_content_change(text, Some(r), ""), "set hi\n");
    }

    #[test]
    fn lsp_position_helpers_use_utf16_columns() {
        let text = "é😀x\n";
        assert_eq!(line_col_to_byte_offset(text, 0, 0), Some(0));
        assert_eq!(line_col_to_byte_offset(text, 0, 1), Some(2));
        assert_eq!(line_col_to_byte_offset(text, 0, 3), Some(6));

        let line_index = tcl_lexer::LineIndex::new(text);
        let range = lift_span(text, &line_index, tcl_lexer::Span::new(6, 7));
        assert_eq!(range.start, pos(0, 3));
        assert_eq!(range.end, pos(0, 4));
    }

    #[test]
    fn apply_content_change_sequence_matches_full_text() {
        // Applying a sequence of ranged edits (as INCREMENTAL sync
        // delivers them) reproduces the intended final document.
        let mut text = "abc\ndef\n".to_string();
        // Edit 1: insert "X" at (0,1) -> "aXbc".
        text = apply_content_change(
            &text,
            Some(Range {
                start: pos(0, 1),
                end: pos(0, 1),
            }),
            "X",
        );
        assert_eq!(text, "aXbc\ndef\n");
        // Edit 2: replace "def" with "ghij".
        text = apply_content_change(
            &text,
            Some(Range {
                start: pos(1, 0),
                end: pos(1, 3),
            }),
            "ghij",
        );
        assert_eq!(text, "aXbc\nghij\n");
    }

    /// SRV-INCREMENTAL Task 1 (live path): a persisted `LineIndex` driven through
    /// `apply_content_change_indexed` over a sequence of ranged + full-replace
    /// edits must stay byte-identical to a rebuild from the final text — the
    /// invariant `DocumentState` relies on for its persisted index.
    #[test]
    #[allow(clippy::cast_possible_truncation)] // line_count fits u32 (4 GiB budget)
    fn apply_content_change_indexed_keeps_line_index_consistent() {
        let line_starts =
            |idx: &tcl_lexer::LineIndex| (0..idx.line_count()).map(|l| idx.line_start(l as u32)).collect::<Vec<_>>();
        let mut text = "abc\ndef\nghi\n".to_string();
        let mut index = tcl_lexer::LineIndex::new(&text);
        let edits = [
            (Some(Range { start: pos(0, 1), end: pos(0, 1) }), "X\nY"), // insert with newline
            (Some(Range { start: pos(2, 0), end: pos(3, 2) }), ""),     // multi-line deletion
            (None, "p\nq\nr\ns"),                                       // full replacement
            (Some(Range { start: pos(1, 1), end: pos(2, 0) }), "Z"),    // collapse a line
        ];
        for (range, new_text) in edits {
            text = apply_content_change_indexed(&text, range, new_text, &mut index);
            assert_eq!(
                line_starts(&index),
                line_starts(&tcl_lexer::LineIndex::new(&text)),
                "persisted index diverged from rebuild after edit {new_text:?} -> {text:?}"
            );
        }
    }

    /// GAP-C1 strip 2: the source-style pass must reach the
    /// published set.  A long line + trailing whitespace + CRLF
    /// endings exercise W111 / W112 / W118 — none of which the
    /// analyser or compiler-check pipelines emit — so a non-empty
    /// result with those codes proves `lift_source_style_diagnostics`
    /// is wired in.
    #[test]
    fn lift_source_style_diagnostics_surfaces_style_codes() {
        let long = "x".repeat(130);
        let src = format!("{long}  \r\nputs ok\r\n");
        let diags = lift_source_style_diagnostics(
            &src,
            &std::collections::HashMap::new(),
            &std::collections::HashSet::new(),
        );
        let codes: Vec<String> = diags
            .iter()
            .filter_map(|d| match &d.code {
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) => Some(c.clone()),
                _ => None,
            })
            .collect();
        for want in ["W111", "W112", "W118"] {
            assert!(
                codes.iter().any(|c| c == want),
                "expected {want}, got: {codes:?}",
            );
        }
        assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));
    }

    /// A file-level `# tcl-lsp: disable=W111` directive (recorded by
    /// the analyser against the `-1` suppression bucket) must drop
    /// W111 from the lifted style set while leaving the other style
    /// codes intact.
    #[test]
    fn lift_source_style_diagnostics_honours_file_suppression() {
        let long = "y".repeat(130);
        let src = format!("{long}  \n");
        let mut suppressed: std::collections::HashMap<i32, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        suppressed.insert(-1, std::iter::once("W111".to_string()).collect());
        let diags =
            lift_source_style_diagnostics(&src, &suppressed, &std::collections::HashSet::new());
        let codes: Vec<String> = diags
            .iter()
            .filter_map(|d| match &d.code {
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) => Some(c.clone()),
                _ => None,
            })
            .collect();
        assert!(
            !codes.iter().any(|c| c == "W111"),
            "W111 should be suppressed"
        );
        assert!(codes.iter().any(|c| c == "W112"), "W112 should remain");
    }

    // ── FE-DIAG-F5 consumer-wiring ──────────────────────────────────────

    /// Collect the string codes from a lifted diagnostic set.
    fn diag_codes(diags: &[tower_lsp::lsp_types::Diagnostic]) -> Vec<String> {
        diags
            .iter()
            .filter_map(|d| match &d.code {
                Some(tower_lsp::lsp_types::NumberOrString::String(c)) => Some(c.clone()),
                _ => None,
            })
            .collect()
    }

    /// A BIG-IP config that references a pool not defined in the config
    /// surfaces `BIGIP6002` through the validator-lift, tagged `tcl-lsp`.
    #[test]
    fn bigip_config_diagnostics_surfaces_codes() {
        let src =
            "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    pool /Common/no_such_pool\n  }\n}\n";
        let diags = bigip_config_diagnostics(src, &HashSet::new());
        let codes = diag_codes(&diags);
        assert!(
            codes.iter().any(|c| c == "BIGIP6002"),
            "expected BIGIP6002, got: {codes:?}",
        );
        assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));
    }

    /// A disabled BIG-IP code (`tclLsp.diagnostics.BIGIP6002 = false`) is
    /// filtered from the lifted set, mirroring Python's `disabled_codes`.
    #[test]
    fn bigip_config_diagnostics_honours_disabled() {
        let src =
            "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    pool /Common/no_such_pool\n  }\n}\n";
        let disabled: HashSet<String> = std::iter::once("BIGIP6002".to_owned()).collect();
        let codes = diag_codes(&bigip_config_diagnostics(src, &disabled));
        assert!(
            !codes.iter().any(|c| c == "BIGIP6002"),
            "BIGIP6002 should be filtered, got: {codes:?}",
        );
    }

    /// An iApp APL presentation field never referenced by the (cross-file)
    /// implementation surfaces `IAPP7002`.
    #[test]
    fn apl_presentation_diagnostics_surfaces_codes() {
        let refs = tcl_bigip::apl::extract_iapp_var_refs("set x $::basic__addr");
        let diags = apl_presentation_diagnostics(
            "section basic {\n  string addr\n  string port\n}\n",
            Some(&refs),
            &HashSet::new(),
        );
        let codes = diag_codes(&diags);
        assert!(
            codes.iter().any(|c| c == "IAPP7002"),
            "expected IAPP7002 for the unreferenced 'port' field, got: {codes:?}",
        );
        assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));
    }

    /// `is_apl_source` matches by APL language id or `*.apl` / `presentation`
    /// basename, mirroring Python `_is_apl_source`.
    #[test]
    fn is_apl_source_detects_apl_documents() {
        let apl_ext = Url::parse("file:///app/iapp/foo.apl").unwrap();
        let presentation = Url::parse("file:///app/iapp/presentation").unwrap();
        let plain = Url::parse("file:///app/util.tcl").unwrap();
        assert!(is_apl_source(&apl_ext, "tcl"));
        assert!(is_apl_source(&presentation, "tcl"));
        // An explicit APL language id wins regardless of basename.
        assert!(is_apl_source(&plain, "tcl-apl"));
        // A plain Tcl document is not an APL source.
        assert!(!is_apl_source(&plain, "tcl"));
    }

    /// A `tcl_bigip` validator range carries an *inclusive* end column; the
    /// LSP lift makes it exclusive (`end.character + 1`), matching Python's
    /// `to_lsp_range`.
    #[test]
    fn lift_config_diagnostic_makes_end_exclusive() {
        let pos = tcl_bigip::Position {
            line: 3,
            character: 5,
            offset: 0,
        };
        let end = tcl_bigip::Position {
            line: 3,
            character: 9,
            offset: 0,
        };
        let d = tcl_bigip::validator::ConfigDiagnostic {
            code: "BIGIP6002".to_owned(),
            message: "x".to_owned(),
            severity: tcl_bigip::validator::DiagSeverity::Warning,
            range: tcl_bigip::Range { start: pos, end },
        };
        let lifted = lift_config_diagnostic(&d);
        assert_eq!(lifted.range.start.line, 3);
        assert_eq!(lifted.range.start.character, 5);
        assert_eq!(lifted.range.end.line, 3);
        assert_eq!(lifted.range.end.character, 10);
        assert_eq!(
            lifted.severity,
            Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING)
        );
    }

    /// End-to-end: the pull path's [`Backend::full_diagnostics_for`] routes a
    /// `f5-bigip` document to the BIG-IP validator instead of the Tcl
    /// analyser, so the editor receives `BIGIP6xxx` codes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn full_diagnostics_for_routes_bigip_to_validator() {
        let backend = test_backend();
        let uri = Url::parse("file:///Common/bigip.conf").unwrap();
        let src =
            "ltm rule /Common/r {\n  when HTTP_REQUEST {\n    pool /Common/no_such_pool\n  }\n}\n";
        let diags = backend
            .full_diagnostics_for(&uri, src.to_owned(), "f5-bigip".to_owned(), "tcl-bigip")
            .await;
        let codes = diag_codes(&diags);
        assert!(
            codes.iter().any(|c| c == "BIGIP6002"),
            "expected BIGIP6002 via the pull path, got: {codes:?}",
        );
    }

    /// `lift_xc_diagnostics` surfaces the XC translatability codes for an
    /// iRule and filters those the editor disabled.
    #[test]
    fn lift_xc_diagnostics_surfaces_and_filters_codes() {
        let src = "when HTTP_REQUEST {\n    pool my_pool\n}";
        let no_suppress = std::collections::HashMap::new();
        let diags = lift_xc_diagnostics(src, &HashSet::new(), &no_suppress);
        let codes = diag_codes(&diags);
        assert!(
            codes.iter().any(|c| c == "XC100"),
            "expected XC100, got: {codes:?}",
        );
        assert!(diags.iter().all(|d| d.source.as_deref() == Some("tcl-lsp")));

        // Disabling XC100 drops it from the lifted set.
        let disabled: HashSet<String> = std::iter::once("XC100".to_owned()).collect();
        let filtered = lift_xc_diagnostics(src, &disabled, &no_suppress);
        assert!(
            !diag_codes(&filtered).iter().any(|c| c == "XC100"),
            "XC100 should be filtered when disabled",
        );
    }

    /// The opt-in `xcDiagnostics` toggle gates whether XC100-301 reach the
    /// `f5-irules` diagnostics pull path: off by default, present once
    /// enabled.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn xc_diagnostics_are_opt_in_on_irules_documents() {
        let uri = Url::parse("file:///rule.irul").unwrap();
        let src = "when HTTP_REQUEST {\n    pool my_pool\n}";

        // Default-off: no XC codes surface.
        let backend = test_backend();
        let off = backend
            .full_diagnostics_for(&uri, src.to_owned(), "f5-irules".to_owned(), "tcl-irule")
            .await;
        assert!(
            !diag_codes(&off).iter().any(|c| c.starts_with("XC")),
            "XC diagnostics must be off by default, got: {:?}",
            diag_codes(&off),
        );

        // Opt in via the `xcDiagnostics` feature toggle.
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "xcDiagnostics": true })
                .as_object()
                .unwrap(),
        );
        let on = backend
            .full_diagnostics_for(&uri, src.to_owned(), "f5-irules".to_owned(), "tcl-irule")
            .await;
        assert!(
            diag_codes(&on).iter().any(|c| c == "XC100"),
            "expected XC100 once xcDiagnostics is enabled, got: {:?}",
            diag_codes(&on),
        );
    }

    /// SRV-INCREMENTAL Task 6 (server wiring): a command unresolved in file A but
    /// defined as a `proc` in file B (tracked in the salsa `Project`) must have
    /// its W123 suppressed once `xcDiagnostics` is enabled — and remain present
    /// when it is off.  Exercises the live `db_set_source` → `Project`-sync →
    /// `project_proc_tails` path end to end.
    #[tokio::test]
    async fn cross_file_w123_suppressed_when_workspace_defines_proc() {
        let backend = test_backend();
        let a = Url::parse("file:///a.tcl").unwrap();
        let b = Url::parse("file:///b.tcl").unwrap();
        // Track B (defines `proc helper`) so the `Project` input includes it.
        backend
            .db_set_source(
                &b,
                "proc helper {x y} { return $x }\n".to_owned(),
                "tcl8.6".to_owned(),
            )
            .await;
        // The `Project` input is now maintained with B.
        assert!(
            backend.db_project.lock().await.is_some(),
            "Project input must be created when the first file is tracked"
        );
        let a_src = "helper foo bar\n";

        // xcDiagnostics OFF: `helper` is unresolved in A → W123 present.
        let off = backend
            .full_diagnostics_for(&a, a_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            diag_codes(&off).iter().any(|c| c == "W123"),
            "W123 must be present when xcDiagnostics is off, got: {:?}",
            diag_codes(&off),
        );

        // xcDiagnostics ON: `helper` resolves cross-file (B defines it) → no W123.
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "xcDiagnostics": true })
                .as_object()
                .unwrap(),
        );
        let on = backend
            .full_diagnostics_for(&a, a_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            !diag_codes(&on).iter().any(|c| c == "W123"),
            "W123 must be suppressed cross-file when B defines proc helper, got: {:?}",
            diag_codes(&on),
        );
    }

    #[test]
    fn dialect_from_language_id_recognises_editor_ids() {
        assert_eq!(
            Backend::dialect_from_language_id("tcl-irule"),
            Some("f5-irules"),
        );
        assert_eq!(
            Backend::dialect_from_language_id("tcl-iapp"),
            Some("f5-iapps"),
        );
        // APL presentation files are an iApp sublanguage → f5-iapps.
        assert_eq!(
            Backend::dialect_from_language_id("tcl-apl"),
            Some("f5-iapps"),
        );
        assert_eq!(Backend::dialect_from_language_id("tcl"), Some("tcl8.6"));
    }

    #[test]
    fn dialect_from_language_id_passes_through_canonical_names() {
        assert_eq!(
            Backend::dialect_from_language_id("f5-irules"),
            Some("f5-irules"),
        );
        assert_eq!(Backend::dialect_from_language_id("tcl9.0"), Some("tcl9.0"),);
        assert_eq!(
            Backend::dialect_from_language_id("synopsys-eda-tcl"),
            Some("synopsys-eda-tcl"),
        );
    }

    #[test]
    fn dialect_from_language_id_returns_none_for_unknown_ids() {
        assert!(Backend::dialect_from_language_id("plaintext").is_none());
        assert!(Backend::dialect_from_language_id("").is_none());
        // ``tcl-bigip`` is a BIG-IP config language id with no Tcl
        // dialect counterpart — falls back to the session default.
        assert!(Backend::dialect_from_language_id("tcl-bigip").is_none());
    }

    /// The folding-range lift maps `FoldKind::Comment` to the LSP
    /// `Comment` kind and `FoldKind::Region` to `Region`. The
    /// `start_character` / `end_character` fields stay `None`
    /// because the core provider returns line-only ranges.
    #[test]
    fn lift_folding_range_preserves_kind_and_lines() {
        let r = tcl_lsp_core::folding::FoldingRange {
            start_line: 2,
            end_line: 5,
            kind: FoldKind::Region,
        };
        let lifted = lift_folding_range(r);
        assert_eq!(lifted.start_line, 2);
        assert_eq!(lifted.end_line, 5);
        assert_eq!(lifted.kind, Some(FoldingRangeKind::Region));
        assert!(lifted.start_character.is_none());
        assert!(lifted.end_character.is_none());

        let comment = tcl_lsp_core::folding::FoldingRange {
            start_line: 0,
            end_line: 3,
            kind: FoldKind::Comment,
        };
        assert_eq!(
            lift_folding_range(comment).kind,
            Some(FoldingRangeKind::Comment),
        );
    }

    /// `lift_symbol_kind` maps every `CoreSymbolKind` variant to the
    /// matching LSP `SymbolKind` constant. The mapping is exhaustive
    /// so additions to `CoreSymbolKind` force an explicit decision
    /// about the corresponding LSP `SymbolKind`.
    #[test]
    fn lift_symbol_kind_covers_all_variants() {
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Function),
            SymbolKind::FUNCTION
        );
        assert_eq!(lift_symbol_kind(CoreSymbolKind::Method), SymbolKind::METHOD);
        assert_eq!(lift_symbol_kind(CoreSymbolKind::Class), SymbolKind::CLASS);
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Property),
            SymbolKind::PROPERTY
        );
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Constructor),
            SymbolKind::CONSTRUCTOR,
        );
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Namespace),
            SymbolKind::NAMESPACE
        );
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Variable),
            SymbolKind::VARIABLE
        );
    }

    /// `lift_document_symbol` preserves the name / detail / kind /
    /// range / selection-range fields and recursively lifts
    /// children. An empty child list maps to `None` rather than
    /// `Some(Vec::new())` so the serialised JSON omits the field
    /// entirely (per `#[serde(skip_serializing_if)]` on the LSP
    /// type).
    #[test]
    fn lift_document_symbol_preserves_fields_and_recurses() {
        let inner = core_symbols::DocumentSymbol {
            name: "child".to_owned(),
            detail: None,
            kind: CoreSymbolKind::Variable,
            range: core_symbols::LineRange {
                start_line: 2,
                start_character: 4,
                end_line: 2,
                end_character: 9,
            },
            selection_range: core_symbols::LineRange {
                start_line: 2,
                start_character: 4,
                end_line: 2,
                end_character: 9,
            },
            children: Vec::new(),
        };
        let outer = core_symbols::DocumentSymbol {
            name: "demo".to_owned(),
            detail: Some("()".to_owned()),
            kind: CoreSymbolKind::Function,
            range: core_symbols::LineRange {
                start_line: 0,
                start_character: 0,
                end_line: 4,
                end_character: 1,
            },
            selection_range: core_symbols::LineRange {
                start_line: 0,
                start_character: 5,
                end_line: 0,
                end_character: 9,
            },
            children: vec![inner],
        };

        let lifted = lift_document_symbol(outer);
        assert_eq!(lifted.name, "demo");
        assert_eq!(lifted.detail.as_deref(), Some("()"));
        assert_eq!(lifted.kind, SymbolKind::FUNCTION);
        assert_eq!(
            lifted.range.start,
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            lifted.range.end,
            Position {
                line: 4,
                character: 1
            }
        );
        assert_eq!(
            lifted.selection_range.start,
            Position {
                line: 0,
                character: 5
            },
        );
        let kids = lifted.children.expect("non-empty children lift to Some");
        assert_eq!(kids.len(), 1);
        assert_eq!(kids[0].name, "child");
        assert_eq!(kids[0].kind, SymbolKind::VARIABLE);
        assert!(
            kids[0].children.is_none(),
            "leaf symbol's empty children must lift to None",
        );
    }

    /// Build a fresh `Backend` for unit tests that only
    /// exercise the dialect-resolution helpers.  `Client`
    /// isn't publicly constructible, so we build via
    /// `LspService::new` and copy the wrapped `Client` into a
    /// fresh `Backend` with reset state.
    fn test_backend() -> Backend {
        let (service, _socket) = tower_lsp::LspService::new(Backend::new);
        let db = tcl_lsp_db::TclDatabase::default();
        let db_config = tcl_lsp_db::AnalyserConfig::new(&db, Vec::new(), NonAsciiMode::Default);
        Backend {
            client: service.inner().client.clone(),
            documents: Arc::new(Mutex::new(HashMap::new())),
            diag_slots: Arc::new(Mutex::new(HashMap::new())),
            default_dialect: Mutex::new("tcl8.6".to_owned()),
            workspace_folders: Mutex::new(Vec::new()),
            folder_dialects: Mutex::new(Vec::new()),
            folder_configs: Mutex::new(Vec::new()),
            folder_db_configs: Arc::new(Mutex::new(Vec::new())),
            non_ascii_mode: Mutex::new(NonAsciiMode::Default),
            disabled_diagnostics: Mutex::new(HashSet::new()),
            workspace_index: Arc::new(Mutex::new(core_workspace_index::WorkspaceIndex::new())),
            feature_toggles: Mutex::new(FeatureToggles::default()),
            optimiser_enabled: Mutex::new(true),
            optimiser_profile: Mutex::new(default_optimiser_profile()),
            optimiser_code_overrides: Mutex::new(HashMap::new()),
            line_length: Mutex::new(80),
            db: Arc::new(Mutex::new(db)),
            db_files: Arc::new(Mutex::new(HashMap::new())),
            db_project: Arc::new(Mutex::new(None)),
            db_config: Arc::new(Mutex::new(db_config)),
            pull_diag_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pull_diagnostics_include_compiler_and_optimiser_codes() {
        // Regression: the pull handler (`textDocument/diagnostic`) must return
        // the same full set as the push path — analyser + compiler/optimiser +
        // source-style — so editors on the pull path don't lose O-codes.
        let backend = test_backend();
        *backend.optimiser_profile.lock().await =
            tcl_compiler::optimiser::profiles::OptimisationProfile::Full;
        let uri = Url::parse("file:///pull.tcl").unwrap();
        let src = "if {1} { set x 1 } else { set y 2 }\n";
        let full = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        let analyser_only = {
            let mut a = Analyser::new();
            let analysis = a.analyse(src, "tcl8.6").clone();
            lift_analyser_diagnostics(src, &analysis.diagnostics)
        };
        let has_o100 = |ds: &[tower_lsp::lsp_types::Diagnostic]| {
            ds.iter().any(|d| {
                matches!(&d.code, Some(tower_lsp::lsp_types::NumberOrString::String(c)) if c == "O100")
            })
        };
        assert!(
            has_o100(&full),
            "pull diagnostics must include the optimiser O100, got {:?}",
            full.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
        assert!(
            !has_o100(&analyser_only),
            "the analyser-only path should not emit O100 (sanity)",
        );
    }

    fn doc_diag_params(uri: &Url, previous: Option<&str>) -> DocumentDiagnosticParams {
        DocumentDiagnosticParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            identifier: None,
            previous_result_id: previous.map(str::to_owned),
            work_done_progress_params: tower_lsp::lsp_types::WorkDoneProgressParams::default(),
            partial_result_params: tower_lsp::lsp_types::PartialResultParams::default(),
        }
    }

    /// The pull handler returns a `Full` report carrying a `result_id` and,
    /// when the client echoes that same id back as `previousResultId` and the
    /// document has not changed, an `Unchanged` report naming the same id.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pull_diagnostic_returns_unchanged_when_result_id_matches() {
        let backend = test_backend();
        let uri = Url::parse("file:///pull-unchanged.tcl").unwrap();
        register(&backend, &uri, "set x 1\n").await;

        let first = backend
            .diagnostic(doc_diag_params(&uri, None))
            .await
            .unwrap();
        let result_id = match first {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => full
                .full_document_diagnostic_report
                .result_id
                .expect("first pull must carry a result_id"),
            other => panic!("expected a Full report, got {other:?}"),
        };

        let second = backend
            .diagnostic(doc_diag_params(&uri, Some(&result_id)))
            .await
            .unwrap();
        match second {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Unchanged(u)) => {
                assert_eq!(
                    u.unchanged_document_diagnostic_report.result_id, result_id,
                    "Unchanged report must name the cached result_id",
                );
            }
            other => panic!("expected an Unchanged report, got {other:?}"),
        }
    }

    /// `workspace/diagnostic` reports each cached (open) document, and honours a
    /// matching `previousResultId` with an `Unchanged` per-document report.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn workspace_diagnostic_reports_cached_documents() {
        let backend = test_backend();
        let uri = Url::parse("file:///ws-diag.tcl").unwrap();
        register(&backend, &uri, "set x 1\n").await;
        // Prime the pull cache through the document handler.
        let first = backend
            .diagnostic(doc_diag_params(&uri, None))
            .await
            .unwrap();
        let result_id = match first {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
                full.full_document_diagnostic_report.result_id.unwrap()
            }
            other => panic!("expected Full, got {other:?}"),
        };

        let params = WorkspaceDiagnosticParams {
            identifier: None,
            previous_result_ids: vec![tower_lsp::lsp_types::PreviousResultId {
                uri: uri.clone(),
                value: result_id.clone(),
            }],
            work_done_progress_params: tower_lsp::lsp_types::WorkDoneProgressParams::default(),
            partial_result_params: tower_lsp::lsp_types::PartialResultParams::default(),
        };
        let report = backend.workspace_diagnostic(params).await.unwrap();
        let WorkspaceDiagnosticReportResult::Report(report) = report else {
            panic!("expected a full workspace report");
        };
        assert_eq!(
            report.items.len(),
            1,
            "the one open document must be reported"
        );
        match &report.items[0] {
            WorkspaceDocumentDiagnosticReport::Unchanged(u) => {
                assert_eq!(u.uri, uri);
                assert_eq!(u.unchanged_document_diagnostic_report.result_id, result_id);
            }
            WorkspaceDocumentDiagnosticReport::Full(full) => {
                panic!(
                    "matching previousResultId must yield Unchanged, got Full for {:?}",
                    full.uri,
                )
            }
        }
    }

    /// `did_change_watched_files` drops a deleted file from the cross-document
    /// index and re-indexes a (closed) changed file from disk.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watched_files_deletion_drops_index_entry() {
        let backend = test_backend();
        let uri = Url::parse("file:///watched-gone.tcl").unwrap();
        // Index a file that is NOT open (no `documents` entry).
        {
            let mut a = Analyser::new();
            let analysis = a.analyse("proc gone {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &analysis);
        }
        assert!(
            backend
                .workspace_index
                .lock()
                .await
                .document_uris()
                .iter()
                .any(|u| u == uri.as_str()),
            "precondition: file is indexed",
        );

        backend
            .did_change_watched_files(DidChangeWatchedFilesParams {
                changes: vec![tower_lsp::lsp_types::FileEvent {
                    uri: uri.clone(),
                    typ: FileChangeType::DELETED,
                }],
            })
            .await;

        assert!(
            !backend
                .workspace_index
                .lock()
                .await
                .document_uris()
                .iter()
                .any(|u| u == uri.as_str()),
            "a DELETED watched-file event must drop the index entry",
        );
    }

    /// Removing a workspace folder drops index entries for its (closed) files
    /// while leaving files under other folders intact.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn workspace_folder_removal_drops_index_under_folder() {
        let backend = test_backend();
        let gone = Url::parse("file:///proj-a/lib.tcl").unwrap();
        let kept = Url::parse("file:///proj-b/lib.tcl").unwrap();
        {
            let mut a = Analyser::new();
            let analysis = a.analyse("proc p {} {}\n", "tcl8.6").clone();
            let mut index = backend.workspace_index.lock().await;
            index.add_document(gone.as_str(), &analysis);
            index.add_document(kept.as_str(), &analysis);
        }
        *backend.workspace_folders.lock().await = vec![
            Url::parse("file:///proj-a").unwrap(),
            Url::parse("file:///proj-b").unwrap(),
        ];

        backend
            .did_change_workspace_folders(DidChangeWorkspaceFoldersParams {
                event: tower_lsp::lsp_types::WorkspaceFoldersChangeEvent {
                    added: Vec::new(),
                    removed: vec![tower_lsp::lsp_types::WorkspaceFolder {
                        uri: Url::parse("file:///proj-a").unwrap(),
                        name: "proj-a".to_owned(),
                    }],
                },
            })
            .await;

        let uris = backend.workspace_index.lock().await.document_uris();
        assert!(
            !uris.iter().any(|u| u == gone.as_str()),
            "files under the removed folder must be dropped",
        );
        assert!(
            uris.iter().any(|u| u == kept.as_str()),
            "files under a retained folder must remain",
        );
        // The folder list itself no longer carries the removed root.
        assert!(
            !backend
                .workspace_folders
                .lock()
                .await
                .iter()
                .any(|f| f.as_str() == "file:///proj-a"),
        );
    }

    /// A per-folder editor config overrides the process-global settings for
    /// documents under that folder, while documents under other folders keep
    /// the global values (longest-prefix resolution).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn per_folder_config_overrides_global_settings() {
        let backend = test_backend();
        let folder_a = Url::parse("file:///proj-a").unwrap();
        let folder_b = Url::parse("file:///proj-b").unwrap();
        let inside = Url::parse("file:///proj-a/sub/file.tcl").unwrap();
        let outside = Url::parse("file:///proj-b/file.tcl").unwrap();
        *backend.workspace_folders.lock().await = vec![folder_a.clone(), folder_b.clone()];

        // proj-a turns hover off and disables W100; the global state keeps both
        // on / empty.
        let feature_toggles = {
            let mut t = FeatureToggles::default();
            t.set.insert("hover".to_owned(), false);
            t
        };
        let fc = FolderConfig {
            feature_toggles,
            disabled_diagnostics: Some(std::iter::once("W100".to_owned()).collect()),
            ..FolderConfig::default()
        };
        backend
            .apply_folder_configs(vec![(folder_a.clone(), fc)])
            .await;

        // Feature gating resolves per folder.
        assert!(
            !backend.feature_enabled("hover", &inside).await,
            "hover must be off under proj-a",
        );
        assert!(
            backend.feature_enabled("hover", &outside).await,
            "hover must stay on under proj-b (global default)",
        );

        // Disabled-diagnostics resolve per folder.
        let (disabled_in, ..) = backend.resolved_analysis_settings(&inside).await;
        assert!(disabled_in.contains("W100"), "proj-a disables W100");
        let (disabled_out, ..) = backend.resolved_analysis_settings(&outside).await;
        assert!(
            !disabled_out.contains("W100"),
            "proj-b inherits the empty global disabled set",
        );

        // A folder that overrides the disabled set gets its own salsa config
        // handle, so the suppression also reaches the cached analysis.
        assert!(
            backend
                .folder_db_configs
                .lock()
                .await
                .iter()
                .any(|(u, _)| u == &folder_a),
            "proj-a must have a per-folder AnalyserConfig handle",
        );
    }

    /// A pull arriving after an edit (revision bumped) but before the debounced
    /// worker refreshes the cache must recompute, not serve the stale cached
    /// report — and must not answer `Unchanged` even when the client echoes the
    /// stale `result_id`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pull_diagnostic_recomputes_after_edit_bumps_revision() {
        let backend = test_backend();
        let uri = Url::parse("file:///pull-stale.tcl").unwrap();
        register(&backend, &uri, "set x 1\n").await;
        let first = backend
            .diagnostic(doc_diag_params(&uri, None))
            .await
            .unwrap();
        let stale_id = match first {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
                full.full_document_diagnostic_report.result_id.unwrap()
            }
            other => panic!("expected Full, got {other:?}"),
        };

        // Simulate `did_change`: bump the revision (and text) without running
        // the worker, so the cache entry is now for an older revision.
        {
            let mut docs = backend.documents.lock().await;
            let doc = docs.get_mut(&uri).unwrap();
            doc.text = "set x 2\n".to_owned();
            doc.bump_revision(1);
        }

        let second = backend
            .diagnostic(doc_diag_params(&uri, Some(&stale_id)))
            .await
            .unwrap();
        match second {
            DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
                assert_ne!(
                    full.full_document_diagnostic_report.result_id,
                    Some(stale_id),
                    "a stale result_id must not be reused after an edit",
                );
            }
            other => panic!("an edited buffer must recompute, not serve Unchanged: {other:?}"),
        }
    }

    /// Format-on-save honours a folder-scoped opt-in: a document under a folder
    /// that turns on `willSaveWaitUntil` formats, while documents outside it
    /// stay inert under the (off) global default.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn will_save_format_respects_per_folder_opt_in() {
        let backend = test_backend();
        let folder = Url::parse("file:///proj-a").unwrap();
        let inside = Url::parse("file:///proj-a/file.tcl").unwrap();
        let outside = Url::parse("file:///other/file.tcl").unwrap();
        *backend.workspace_folders.lock().await = vec![folder.clone()];
        register(&backend, &inside, "set    x     1\n").await;
        register(&backend, &outside, "set    x     1\n").await;

        // Global toggle stays off; only proj-a opts in.
        let feature_toggles = {
            let mut t = FeatureToggles::default();
            t.set.insert("willSaveWaitUntil".to_owned(), true);
            t
        };
        let fc = FolderConfig {
            feature_toggles,
            ..FolderConfig::default()
        };
        backend
            .apply_folder_configs(vec![(folder.clone(), fc)])
            .await;

        let params = |uri: &Url| WillSaveTextDocumentParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            reason: tower_lsp::lsp_types::TextDocumentSaveReason::MANUAL,
        };
        assert!(
            backend
                .will_save_wait_until(params(&inside))
                .await
                .unwrap()
                .is_some_and(|edits| !edits.is_empty()),
            "a folder-scoped opt-in must format on save",
        );
        assert!(
            backend
                .will_save_wait_until(params(&outside))
                .await
                .unwrap()
                .is_none(),
            "outside the opted-in folder, save stays inert under the global default",
        );
    }

    /// `willSaveWaitUntil` is opt-in: with the feature toggle unset it returns
    /// no edits even for mis-formatted source; once enabled it formats.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn will_save_format_is_opt_in() {
        let backend = test_backend();
        let uri = Url::parse("file:///will-save.tcl").unwrap();
        register(&backend, &uri, "set    x     1\n").await;
        let params = WillSaveTextDocumentParams {
            text_document: tower_lsp::lsp_types::TextDocumentIdentifier { uri: uri.clone() },
            reason: tower_lsp::lsp_types::TextDocumentSaveReason::MANUAL,
        };

        let disabled = backend.will_save_wait_until(params.clone()).await.unwrap();
        assert!(
            disabled.is_none(),
            "format-on-save must be inert until the feature toggle is set",
        );

        backend
            .feature_toggles
            .lock()
            .await
            .set
            .insert("willSaveWaitUntil".to_owned(), true);
        let enabled = backend.will_save_wait_until(params).await.unwrap();
        assert!(
            enabled.is_some_and(|edits| !edits.is_empty()),
            "with the toggle on, mis-formatted source must yield edits",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn stale_diagnostics_worker_cannot_overwrite_current_analysis() {
        let backend = test_backend();
        let uri = Url::parse("file:///stale.tcl").unwrap();
        let current_src = "proc current {} {}\n";
        let stale_src = "proc stale {} {}\n";

        let mut analyser = Analyser::new();
        let current_analysis = analyser.analyse(current_src, "tcl8.6").clone();
        // The current document is at revision 2; its analysis lives in the
        // query database (set via the SourceFile input) and the workspace index.
        backend
            .db_set_source(&uri, current_src.to_owned(), "tcl8.6".to_owned())
            .await;
        {
            let mut doc =
                DocumentState::with_version(current_src.to_owned(), "tcl8.6".to_owned(), 2);
            doc.revision = 2;
            backend.documents.lock().await.insert(uri.clone(), doc);
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &current_analysis);
        }

        // A stale worker (revision 1) must not overwrite state for revision 2.
        backend
            .publish_analyser_diagnostics(
                uri.clone(),
                stale_src.to_owned(),
                "tcl8.6".to_owned(),
                1,
                Some(1),
            )
            .await;

        // The query database still reflects the current document (its input was
        // never changed to the stale text).
        let analysis = backend
            .cached_analysis(&uri)
            .await
            .expect("current analysis available from the query database");
        assert!(analysis.all_procs.contains_key("::current"));
        assert!(!analysis.all_procs.contains_key("::stale"));

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("current", "other").is_empty(),
            "current document should remain indexed",
        );
        assert!(
            index.proc_definitions("stale", "other").is_empty(),
            "stale worker must not re-index obsolete text",
        );
    }

    #[tokio::test]
    async fn scan_workspace_folders_indexes_unopened_files() {
        let root = unique_scratch_dir("scan");
        std::fs::write(
            root.join("lib.tcl"),
            "proc greet {name} { puts \"hi $name\" }\n",
        )
        .unwrap();
        std::fs::write(root.join("main.tcl"), "greet world\n").unwrap();

        let backend = test_backend();
        let root_url = Url::from_file_path(&root).unwrap();
        *backend.workspace_folders.lock().await = vec![root_url];

        backend.scan_workspace_folders().await;

        let index = backend.workspace_index.lock().await;
        let defs = index.proc_definitions("greet", "greet");
        assert!(
            !defs.is_empty(),
            "expected the unopened lib.tcl proc to be indexed",
        );
        // The call site in main.tcl should be indexed too (no
        // current-URI exclusion here).
        let invs = index.invocations_of("greet", "greet", "");
        assert!(
            !invs.is_empty(),
            "expected the unopened main.tcl call site to be indexed",
        );
        drop(index);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn scan_workspace_folders_skips_open_documents() {
        let root = unique_scratch_dir("scan-open");
        let on_disk = root.join("buf.tcl");
        // The on-disk copy defines `stale`; the live buffer (below)
        // defines `fresh`. After the scan the index must reflect the
        // live buffer, not the on-disk copy.
        std::fs::write(&on_disk, "proc stale {} {}\n").unwrap();

        let backend = test_backend();
        let root_url = Url::from_file_path(&root).unwrap();
        let uri = Url::from_file_path(&on_disk).unwrap();
        *backend.workspace_folders.lock().await = vec![root_url];
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
        );
        // Seed the index from the live buffer the way did_open would.
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        backend.scan_workspace_folders().await;

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("fresh", "fresh").is_empty(),
            "live buffer's proc must survive the scan",
        );
        assert!(
            index.proc_definitions("stale", "stale").is_empty(),
            "on-disk copy must not clobber the open document",
        );
        drop(index);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn workspace_scan_merge_does_not_clobber_open_document() {
        let backend = test_backend();
        let uri = Url::parse("file:///workspace/live.tcl").unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
        );

        let mut analyser = Analyser::new();
        let fresh_analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
        backend
            .workspace_index
            .lock()
            .await
            .add_document(uri.as_str(), &fresh_analysis);

        let mut analyser = Analyser::new();
        let stale_analysis = analyser.analyse("proc stale {} {}\n", "tcl8.6").clone();
        let scan_results = vec![(uri.to_string(), stale_analysis)];

        backend.merge_workspace_scan_results(&scan_results).await;

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("fresh", "other").is_empty(),
            "open-buffer analysis must survive stale workspace scan results",
        );
        assert!(
            index.proc_definitions("stale", "other").is_empty(),
            "workspace scan must not overwrite a live buffer",
        );
    }

    #[tokio::test]
    async fn reindex_from_disk_refreshes_index_rather_than_dropping() {
        let root = unique_scratch_dir("reindex");
        let on_disk = root.join("lib.tcl");
        std::fs::write(&on_disk, "proc helper {} {}\n").unwrap();

        let backend = test_backend();
        let uri = Url::from_file_path(&on_disk).unwrap();
        // Seed the index with a now-closed buffer version (proc `fresh`),
        // standing in for what did_open would have indexed.
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        // What did_close does: refresh from disk instead of removing.
        backend.reindex_index_from_disk(&uri).await;

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("helper", "other").is_empty(),
            "on-disk proc must remain indexed after the buffer closes",
        );
        assert!(
            index.proc_definitions("fresh", "other").is_empty(),
            "stale buffer-only proc must be gone after reindex from disk",
        );
        drop(index);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn reindex_from_disk_does_not_clobber_open_document() {
        let root = unique_scratch_dir("reindex-open");
        let on_disk = root.join("buf.tcl");
        std::fs::write(&on_disk, "proc stale {} {}\n").unwrap();

        let backend = test_backend();
        let uri = Url::from_file_path(&on_disk).unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
        );
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        backend.reindex_index_from_disk(&uri).await;

        let index = backend.workspace_index.lock().await;
        assert!(
            !index.proc_definitions("fresh", "other").is_empty(),
            "open-buffer definitions must remain indexed",
        );
        assert!(
            index.proc_definitions("stale", "other").is_empty(),
            "disk reindex must not overwrite a live buffer",
        );
        drop(index);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn reindex_from_disk_drops_entry_when_file_is_gone() {
        let backend = test_backend();
        let uri = Url::parse("file:///does/not/exist/gone.tcl").unwrap();
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc ghost {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        backend.reindex_index_from_disk(&uri).await;

        let index = backend.workspace_index.lock().await;
        assert!(
            index.proc_definitions("ghost", "other").is_empty(),
            "an entry whose file no longer exists must be dropped",
        );
    }

    #[tokio::test]
    async fn resolve_folder_dialect_picks_deepest_prefix() {
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![
            (
                Url::parse("file:///workspace/").unwrap(),
                "tcl9.0".to_owned(),
            ),
            (
                Url::parse("file:///workspace/irules/").unwrap(),
                "f5-irules".to_owned(),
            ),
        ];
        let inside = Url::parse("file:///workspace/irules/rule.tcl").expect("parse target uri");
        assert_eq!(
            backend.resolve_folder_dialect(&inside).await,
            Some("f5-irules".to_owned()),
        );
        let outside_irules = Url::parse("file:///workspace/main.tcl").expect("parse target uri");
        assert_eq!(
            backend.resolve_folder_dialect(&outside_irules).await,
            Some("tcl9.0".to_owned()),
        );
        let unrelated = Url::parse("file:///elsewhere/x.tcl").unwrap();
        assert_eq!(backend.resolve_folder_dialect(&unrelated).await, None);
    }

    #[tokio::test]
    async fn resolve_folder_dialect_respects_directory_boundary() {
        // Regression for PR #454 Codex review P2: a prefix-only
        // match would incorrectly select `file:///workspace/app`'s
        // dialect for a document inside `file:///workspace/app2/`.
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![
            (
                Url::parse("file:///workspace/app").unwrap(),
                "f5-irules".to_owned(),
            ),
            (
                Url::parse("file:///workspace/app2/").unwrap(),
                "tcl9.0".to_owned(),
            ),
        ];
        let sibling = Url::parse("file:///workspace/app2/main.tcl").unwrap();
        assert_eq!(
            backend.resolve_folder_dialect(&sibling).await,
            Some("tcl9.0".to_owned()),
            "sibling folder must not inherit prefix-matched dialect",
        );
        let inside_app = Url::parse("file:///workspace/app/inner.tcl").unwrap();
        assert_eq!(
            backend.resolve_folder_dialect(&inside_app).await,
            Some("f5-irules".to_owned()),
        );
    }

    #[test]
    fn uri_under_folder_handles_trailing_slash_and_boundary() {
        assert!(uri_under_folder(
            "file:///ws/app/file.tcl",
            "file:///ws/app/",
        ));
        assert!(uri_under_folder(
            "file:///ws/app/file.tcl",
            "file:///ws/app",
        ));
        // The folder itself counts as a match.
        assert!(uri_under_folder("file:///ws/app", "file:///ws/app/",));
        // A sibling folder does not.
        assert!(!uri_under_folder(
            "file:///ws/app2/file.tcl",
            "file:///ws/app",
        ));
        assert!(!uri_under_folder(
            "file:///ws/app2/file.tcl",
            "file:///ws/app/",
        ));
    }

    #[test]
    fn folder_dialect_for_prefers_deepest_match() {
        let folders = vec![
            (Url::parse("file:///ws/").unwrap(), "tcl8.6".to_owned()),
            (
                Url::parse("file:///ws/irules/").unwrap(),
                "f5-irules".to_owned(),
            ),
        ];
        // A file under the nested folder picks up the nested
        // (deepest-prefix) dialect, not the parent's.
        let nested = Url::parse("file:///ws/irules/app.tcl").unwrap();
        assert_eq!(
            folder_dialect_for(&nested, &folders).as_deref(),
            Some("f5-irules"),
        );
        // A file only under the root folder gets the root dialect.
        let top = Url::parse("file:///ws/util.tcl").unwrap();
        assert_eq!(
            folder_dialect_for(&top, &folders).as_deref(),
            Some("tcl8.6"),
        );
        // A file outside every folder has no override.
        let outside = Url::parse("file:///other/x.tcl").unwrap();
        assert_eq!(folder_dialect_for(&outside, &folders), None);
    }

    #[test]
    fn is_tcl_source_matches_the_full_tcl_family_extension_set() {
        // Mirrors Python's `TCL_EXTENSIONS` (server/workspace/scanner.py).
        for ext in [
            "tcl", "tk", "itcl", "tm", "irul", "irule", "iapp", "iappimpl", "impl", "exp", "apl",
        ] {
            assert!(
                is_tcl_source(Path::new(&format!("/a/b.{ext}"))),
                "expected .{ext} to be a Tcl-family source",
            );
        }
        assert!(!is_tcl_source(Path::new("/a/b.txt")));
        assert!(!is_tcl_source(Path::new("/a/b")));
    }

    #[test]
    fn is_skipped_scan_dir_skips_vendor_and_hidden() {
        assert!(is_skipped_scan_dir(Path::new("/a/.git")));
        assert!(is_skipped_scan_dir(Path::new("/a/node_modules")));
        assert!(is_skipped_scan_dir(Path::new("/a/target")));
        assert!(is_skipped_scan_dir(Path::new("/a/tmp")));
        assert!(!is_skipped_scan_dir(Path::new("/a/src")));
        assert!(!is_skipped_scan_dir(Path::new("/a/irules")));
    }

    /// Build a unique scratch directory under the system temp dir
    /// for a disk-walk test, returning its path. The caller removes
    /// it when done.
    fn unique_scratch_dir(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("tcl-lsp-scan-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn collect_tcl_files_walks_recursively_and_skips_vendor() {
        let root = unique_scratch_dir("walk");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("top.tcl"), "set x 1\n").unwrap();
        std::fs::write(root.join("mod.tm"), "set y 2\n").unwrap();
        std::fs::write(root.join("readme.txt"), "not tcl\n").unwrap();
        std::fs::write(root.join("sub/nested.tcl"), "set z 3\n").unwrap();
        std::fs::write(root.join("node_modules/dep.tcl"), "set q 4\n").unwrap();
        std::fs::write(root.join(".git/hook.tcl"), "set w 5\n").unwrap();

        let mut out = Vec::new();
        collect_tcl_files(&root, 100, &mut out);
        let mut names: Vec<String> = out
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        names.sort();

        std::fs::remove_dir_all(&root).ok();

        assert_eq!(names, vec!["mod.tm", "nested.tcl", "top.tcl"]);
    }

    #[test]
    fn collect_tcl_files_respects_cap() {
        let root = unique_scratch_dir("cap");
        for i in 0..10 {
            std::fs::write(root.join(format!("f{i}.tcl")), "set x 1\n").unwrap();
        }
        let mut out = Vec::new();
        collect_tcl_files(&root, 3, &mut out);
        let got = out.len();
        std::fs::remove_dir_all(&root).ok();
        assert_eq!(got, 3);
    }

    #[tokio::test]
    async fn dialect_for_open_falls_back_to_folder_dialect() {
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![(
            Url::parse("file:///workspace/").unwrap(),
            "f5-irules".to_owned(),
        )];
        let doc = Url::parse("file:///workspace/main.tcl").unwrap();
        // A `tcl` language id maps to `tcl8.6` directly, so the
        // folder override is *not* consulted (language_id is the
        // most specific signal we have).
        assert_eq!(
            backend.dialect_for_open(&doc, "tcl").await,
            "tcl8.6".to_owned(),
        );
        // An unknown language id (`plaintext`) lets the folder
        // override take effect.
        assert_eq!(
            backend.dialect_for_open(&doc, "plaintext").await,
            "f5-irules".to_owned(),
        );
    }

    /// An explicit BIG-IP language id resolves to `f5-bigip` even when the
    /// document basename is not a canonical `bigip*.conf` name — the
    /// manual-language-mode case the `is_bigip_conf_name` basename branch
    /// alone would miss.
    #[tokio::test]
    async fn dialect_for_open_maps_bigip_language_id() {
        let backend = test_backend();
        let doc = Url::parse("file:///workspace/device_config.txt").unwrap();
        assert_eq!(
            backend.dialect_for_open(&doc, "tcl-bigip").await,
            "f5-bigip".to_owned(),
        );
        assert_eq!(
            backend.dialect_for_open(&doc, "f5-bigip").await,
            "f5-bigip".to_owned(),
        );
        // A generic Tcl language id on the same non-conf name does not get the
        // BIG-IP dialect (no basename match, no explicit BIG-IP id).
        assert_ne!(
            backend.dialect_for_open(&doc, "tcl").await,
            "f5-bigip".to_owned(),
        );
    }

    /// A session-default dialect change (as `tclLsp.selectDialect` pushes via
    /// `workspace/didChangeConfiguration`) re-resolves already-open documents:
    /// one that fell back to the default moves to the new default, while one
    /// opened with an explicit language id keeps its dialect.
    #[tokio::test]
    async fn dialect_config_change_reresolves_open_documents() {
        let backend = test_backend();
        *backend.default_dialect.lock().await = "tcl8.6".to_owned();

        let plain = Url::parse("file:///plain.tcl").unwrap();
        let irule = Url::parse("file:///app.irule").unwrap();
        {
            let mut docs = backend.documents.lock().await;
            // No recognised language id → opened under the session default.
            docs.insert(
                plain.clone(),
                DocumentState::new("set x 1\n".to_owned(), "tcl8.6".to_owned())
                    .with_language_id("plaintext".to_owned()),
            );
            // An explicit iRules language id → resolved independently of the
            // default.
            docs.insert(
                irule.clone(),
                DocumentState::new("when HTTP_REQUEST {}\n".to_owned(), "f5-irules".to_owned())
                    .with_language_id("tcl-irule".to_owned()),
            );
        }

        // The user switches the session dialect to tcl9.0.
        *backend.default_dialect.lock().await = "tcl9.0".to_owned();
        backend.reresolve_open_document_dialects().await;

        let docs = backend.documents.lock().await;
        assert_eq!(
            docs.get(&plain).unwrap().dialect,
            "tcl9.0",
            "a default-resolved document should follow the new session default",
        );
        assert_eq!(
            docs.get(&irule).unwrap().dialect,
            "f5-irules",
            "an explicit-language-id document keeps its dialect",
        );
    }

    #[tokio::test]
    async fn registry_for_dialect_reuses_cached_arc() {
        let backend = test_backend();
        let first = backend.registry_for_dialect("f5-irules").await;
        let second = backend.registry_for_dialect("f5-irules").await;

        assert!(
            Arc::ptr_eq(&first, &second),
            "repeated dialect lookups should return the cached registry",
        );
    }

    #[test]
    fn command_head_gate_distinguishes_head_from_argument() {
        let src = "proc greet {x} {}\ngreet arg\n";
        let analysis = Analyser::new().analyse(src, "tcl8.6").clone();
        // `greet` at line 1 is a command head (the call site).
        assert!(position_is_command_head(
            src,
            Position {
                line: 1,
                character: 2
            },
            &analysis,
        ));
        // `arg` at line 1 is an argument word, not a command head, so
        // the cross-document fallback must not fire on it.
        assert!(!position_is_command_head(
            src,
            Position {
                line: 1,
                character: 7
            },
            &analysis,
        ));
    }

    #[tokio::test]
    async fn cross_document_definition_resolves_sibling_proc() {
        let backend = test_backend();
        let lib_uri = Url::parse("file:///lib.tcl").unwrap();
        let main_uri = Url::parse("file:///main.tcl").unwrap();
        let lib_src = "proc shared_helper {} {}\n";
        let main_src = "shared_helper\n";
        // Register both documents.
        {
            let mut docs = backend.documents.lock().await;
            docs.insert(
                lib_uri.clone(),
                DocumentState::new(lib_src.to_owned(), "tcl8.6".to_owned()),
            );
            docs.insert(
                main_uri.clone(),
                DocumentState::new(main_src.to_owned(), "tcl8.6".to_owned()),
            );
        }
        // Index the library document's proc.
        {
            let mut a = Analyser::new();
            let lib_analysis = a.analyse(lib_src, "tcl8.6").clone();
            backend
                .workspace_index
                .lock()
                .await
                .add_document(lib_uri.as_str(), &lib_analysis);
        }
        // From main.tcl, jumping on `shared_helper` resolves to
        // lib.tcl's declaration.
        let locs = backend
            .compute_definition(&main_uri, Position::new(0, 0))
            .await
            .expect("definition ok");
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].uri, lib_uri);
        // `proc shared_helper` — name token starts at column 5.
        assert_eq!(locs[0].range.start.line, 0);
        assert_eq!(locs[0].range.start.character, 5);
    }

    #[tokio::test]
    async fn minify_document_command_returns_minified_source() {
        let backend = test_backend();
        let uri = Url::parse("file:///m.tcl").unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("# c\nputs   hi\n".to_owned(), "tcl8.6".to_owned()),
        );
        // Plain minify (compact=false, aggressive=false).
        let args = vec![
            serde_json::Value::String(uri.to_string()),
            serde_json::Value::Bool(false),
            serde_json::Value::Bool(false),
            serde_json::Value::Bool(false),
        ];
        let result = backend
            .minify_document_command(&args)
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(result["source"], serde_json::json!("puts hi"));
        assert_eq!(result["minifiedLength"], serde_json::json!(7));
        assert!(result.get("symbolMap").is_none());
    }

    #[tokio::test]
    async fn minify_document_command_compact_includes_symbol_map() {
        let backend = test_backend();
        let uri = Url::parse("file:///m.tcl").unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(
                "proc greet {name} {\n    return $name\n}\n".to_owned(),
                "tcl8.6".to_owned(),
            ),
        );
        let args = vec![
            serde_json::Value::String(uri.to_string()),
            serde_json::Value::Bool(true), // compact
            serde_json::Value::Bool(false),
            serde_json::Value::Bool(false),
        ];
        let result = backend
            .minify_document_command(&args)
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(
            result["source"],
            serde_json::json!("proc a {a} {return $a}")
        );
        assert!(
            result["symbolMap"]
                .as_str()
                .is_some_and(|s| s.contains("a <- greet")),
            "{result:?}"
        );
    }

    #[test]
    fn unminify_error_command_translates_names() {
        let args = vec![
            serde_json::Value::String("can't read \"a\": no such variable".to_owned()),
            serde_json::Value::String("# Procs\n  a <- greet".to_owned()),
        ];
        let result = Backend::unminify_error_command(&args).expect("some");
        assert_eq!(
            result["translatedError"],
            serde_json::json!("can't read \"greet\": no such variable")
        );
        assert_eq!(result["changed"], serde_json::json!(true));
    }

    #[tokio::test]
    async fn cross_document_definition_skipped_when_local_match_exists() {
        let backend = test_backend();
        let uri = Url::parse("file:///main.tcl").unwrap();
        let src = "proc greet {} {}\ngreet\n";
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "tcl8.6".to_owned()),
        );
        // Cursor on the `greet` call (line 1) resolves locally;
        // the result points at the same document, line 0.
        let locs = backend
            .compute_definition(&uri, Position::new(1, 0))
            .await
            .expect("definition ok");
        assert_eq!(locs.len(), 1, "{locs:?}");
        assert_eq!(locs[0].uri, uri);
        assert_eq!(locs[0].range.start.line, 0);
    }

    /// Register a document in the store and the workspace index.
    async fn register(backend: &Backend, uri: &Url, src: &str) {
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "tcl8.6".to_owned()),
        );
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        backend
            .workspace_index
            .lock()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    #[tokio::test]
    async fn cross_document_references_finds_sibling_call_sites() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        let lib_src = "proc helper {} {}\nhelper\n";
        register(&backend, &lib, lib_src).await;
        register(&backend, &consumer, "helper\nhelper\n").await;
        // From lib.tcl's `helper` declaration, cross-doc refs
        // are the two calls in consumer.tcl.
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(lib_src, "tcl8.6").clone()
        };
        let cross = backend
            .cross_document_references(&lib, lib_src, &analysis, Position::new(0, 5), false)
            .await;
        assert_eq!(cross.len(), 2, "{cross:?}");
        assert!(cross.iter().all(|l| l.uri == consumer));
    }

    #[tokio::test]
    async fn references_handler_merges_local_and_cross_document() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        // lib.tcl: declaration + one local call.
        register(&backend, &lib, "proc helper {} {}\nhelper\n").await;
        // consumer.tcl: one cross-doc call.
        register(&backend, &consumer, "helper\n").await;
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: lib.clone() },
                position: Position::new(0, 5),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };
        let result = backend.references(params).await.expect("ok").expect("some");
        // Declaration (lib:0) + local call (lib:1) + cross-doc
        // call (consumer:0) = 3 locations across 2 files.
        assert_eq!(result.len(), 3, "{result:?}");
        assert!(result.iter().any(|l| l.uri == consumer));
        assert!(result.iter().filter(|l| l.uri == lib).count() == 2);
    }

    #[tokio::test]
    async fn rename_edits_span_multiple_documents() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\nhelper\n").await;
        register(&backend, &consumer, "helper\nhelper\n").await;
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: lib.clone() },
                position: Position::new(0, 5),
            },
            new_name: "do_it".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let edit = backend.rename(params).await.expect("ok").expect("some");
        let changes = edit.changes.expect("changes");
        // Both documents get edits.
        assert!(changes.contains_key(&lib), "lib edits missing: {changes:?}");
        assert!(
            changes.contains_key(&consumer),
            "consumer edits missing: {changes:?}",
        );
        // consumer.tcl: two call sites rewritten to `do_it`.
        let consumer_edits = &changes[&consumer];
        assert_eq!(consumer_edits.len(), 2, "{consumer_edits:?}");
        assert!(consumer_edits.iter().all(|e| e.new_text == "do_it"));
    }

    #[tokio::test]
    async fn rename_to_builtin_blocked_cross_document() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\n").await;
        register(&backend, &consumer, "helper\n").await;
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: lib.clone() },
                position: Position::new(0, 5),
            },
            // `if` is a built-in command — renaming to it must be
            // refused (no edits in any document).
            new_name: "if".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let result = backend.rename(params).await.expect("ok");
        assert!(result.is_none(), "rename to a built-in should be refused");
    }

    #[tokio::test]
    async fn will_rename_rewrites_dependent_source_literal() {
        let backend = test_backend();
        let main = Url::parse("file:///proj/main.tcl").unwrap();
        // main.tcl sources lib/old.tcl via a relative literal.
        register(&backend, &main, "source lib/old.tcl\nputs hi\n").await;
        let params = RenameFilesParams {
            files: vec![tower_lsp::lsp_types::FileRename {
                old_uri: "file:///proj/lib/old.tcl".to_owned(),
                new_uri: "file:///proj/lib/new.tcl".to_owned(),
            }],
        };
        let edit = backend
            .will_rename_files(params)
            .await
            .expect("ok")
            .expect("some");
        let DocumentChanges::Edits(doc_edits) = edit.document_changes.expect("document_changes")
        else {
            panic!("expected Edits");
        };
        assert_eq!(doc_edits.len(), 1);
        assert_eq!(doc_edits[0].text_document.uri, main);
        assert_eq!(doc_edits[0].edits.len(), 1);
        let OneOf::Left(text_edit) = &doc_edits[0].edits[0] else {
            panic!("expected plain TextEdit");
        };
        assert_eq!(text_edit.new_text, "lib/new.tcl");
        // The edit range covers the original `lib/old.tcl` literal on
        // the first line.
        assert_eq!(text_edit.range.start.line, 0);
    }

    #[tokio::test]
    async fn will_rename_returns_none_without_dependents() {
        let backend = test_backend();
        let main = Url::parse("file:///proj/main.tcl").unwrap();
        register(&backend, &main, "source other.tcl\n").await;
        let params = RenameFilesParams {
            files: vec![tower_lsp::lsp_types::FileRename {
                old_uri: "file:///proj/lib/old.tcl".to_owned(),
                new_uri: "file:///proj/lib/new.tcl".to_owned(),
            }],
        };
        // Nothing sources lib/old.tcl → no edit.
        assert!(
            backend
                .will_rename_files(params)
                .await
                .expect("ok")
                .is_none()
        );
    }

    #[tokio::test]
    async fn did_rename_reindexes_moved_document() {
        let backend = test_backend();
        let old = Url::parse("file:///gone.tcl").unwrap();
        register(&backend, &old, "proc helper {} {}\n").await;
        assert_eq!(backend.workspace_index.lock().await.procs().len(), 1);
        let params = RenameFilesParams {
            files: vec![tower_lsp::lsp_types::FileRename {
                old_uri: old.to_string(),
                // New path doesn't exist on disk (test env) — reindex
                // drops the stale old entry and finds nothing to add.
                new_uri: "file:///moved.tcl".to_owned(),
            }],
        };
        backend.did_rename_files(params).await;
        // The old document's proc is gone from the index.
        let index = backend.workspace_index.lock().await;
        assert!(index.procs().iter().all(|p| p.uri != old.as_str()));
    }

    #[tokio::test]
    async fn incoming_calls_span_multiple_documents() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let consumer = Url::parse("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\n").await;
        register(&backend, &consumer, "proc caller {} { helper }\n").await;
        // prepare the `helper` item from lib.tcl.
        let prep = {
            let mut a = Analyser::new();
            let analysis = a.analyse("proc helper {} {}\n", "tcl8.6").clone();
            core_call_hierarchy::prepare("proc helper {} {}\n", 0, 5, &analysis)
        };
        let core_item = prep.into_iter().next().expect("prepared item");
        // Call-hierarchy items now carry the short display name.
        assert_eq!(core_item.name, "helper");
        let cross = backend
            .cross_document_incoming_calls(&lib, &core_item.name)
            .await;
        // The only caller is `caller` in consumer.tcl.
        assert_eq!(cross.len(), 1, "{cross:?}");
        assert_eq!(cross[0].0, consumer);
        assert_eq!(cross[0].1.from.name, "caller");
    }

    #[tokio::test]
    async fn outgoing_calls_resolve_sibling_document_callee() {
        let backend = test_backend();
        let lib = Url::parse("file:///lib.tcl").unwrap();
        let main = Url::parse("file:///main.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\n").await;
        // `caller` calls a builtin (`puts`) and the sibling-file
        // proc `helper`; only `helper` resolves cross-document.
        let main_src = "proc caller {} { puts hi\n helper }\n";
        register(&backend, &main, main_src).await;

        let item = core_call_hierarchy::CallHierarchyItem {
            name: "::caller".to_owned(),
            detail: None,
            range: CoreLspRange {
                start_line: 0,
                start_character: 0,
                end_line: 0,
                end_character: 0,
            },
            selection_range: CoreLspRange {
                start_line: 0,
                start_character: 5,
                end_line: 0,
                end_character: 11,
            },
        };
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(main_src, "tcl8.6").clone()
        };
        let cross = backend
            .cross_document_outgoing_calls(&main, main_src, "tcl", &item, &analysis)
            .await;
        assert_eq!(cross.len(), 1, "{cross:?}");
        assert_eq!(cross[0].to.name, "::helper");
        assert_eq!(cross[0].to.uri, lib);
        assert_eq!(cross[0].from_ranges.len(), 1, "{cross:?}");
    }
}
