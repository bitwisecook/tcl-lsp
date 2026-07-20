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

//! Native LSP server backend for Tcl.
//!
//! Exposes a [`Backend`] that implements [`tower_lsp_server::LanguageServer`]
//! and is wrapped in an `LspService` by the binary. This crate is the
//! second consumer of [`tcl_lsp_core`] (the first is `tcl-lsp-rust`),
//! so the pure-Rust crate boundary now has both production drivers
//! exercising it.
//!
//! LSP methods without a wired provider return
//! [`tower_lsp_server::jsonrpc::ErrorCode::MethodNotFound`].

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod config_ini;

use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
// `FutureExt::shared` lets the progressive diagnostics path compute the base
// per-file analysis once and await it from both the deep pass and the fast tier.
use futures_util::future::FutureExt;
use tcl_compiler::compiler_checks::DiagCode;

use tcl_compiler::analyser::{Analyser, AnalysisResult, NonAsciiMode};
use tcl_dialect::DialectSet;
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
use tcl_lsp_core::package_resolver::PackageResolver;
use tcl_lsp_core::references as core_references;
use tcl_lsp_core::rename as core_rename;
use tcl_lsp_core::selection_range as core_selection_range;
use tcl_lsp_core::semantic_tokens as core_semantic_tokens;
use tcl_lsp_core::signature_help::{
    self as core_sig, ParameterInformation as CoreParameterInformation,
    SignatureHelp as CoreSignatureHelp, SignatureInformation as CoreSignatureInformation,
};
use tcl_lsp_core::tcl_install as core_tcl_install;
use tcl_lsp_core::type_definition as core_type_definition;
use tcl_lsp_core::type_hierarchy as core_type_hierarchy;
use tcl_lsp_core::workspace_index as core_workspace_index;
use tcl_lsp_core::workspace_symbols::{
    self as core_workspace_symbols, WorkspaceSymbolKind as CoreWorkspaceSymbolKind,
};
use tcl_lsp_db::TclDb as _;
use tcl_registry::CommandRegistry;
use tokio::sync::{Mutex, RwLock, Semaphore};
use tower_lsp_server::jsonrpc;
use tower_lsp_server::ls_types::{
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CallHierarchyServerCapability, CodeAction, CodeActionOrCommand, CodeActionParams,
    CodeActionProviderCapability, CodeLens, CodeLensOptions, CodeLensParams, CompletionItem,
    CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse, ConfigurationItem,
    DeclarationCapability, DidChangeConfigurationParams, DidChangeTextDocumentParams,
    DidChangeWatchedFilesParams, DidChangeWatchedFilesRegistrationOptions,
    DidChangeWorkspaceFoldersParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentChanges, DocumentDiagnosticParams, DocumentDiagnosticReport,
    DocumentDiagnosticReportResult, DocumentFormattingParams, DocumentHighlight,
    DocumentHighlightKind, DocumentHighlightParams, DocumentLink, DocumentLinkOptions,
    DocumentLinkParams, DocumentRangeFormattingParams, DocumentSymbol, DocumentSymbolParams,
    DocumentSymbolResponse, Documentation, ExecuteCommandOptions, ExecuteCommandParams,
    FileChangeType, FileOperationFilter, FileOperationPattern, FileOperationRegistrationOptions,
    FileSystemWatcher, FoldingRange, FoldingRangeKind, FoldingRangeParams,
    FoldingRangeProviderCapability, FullDocumentDiagnosticReport, GlobPattern,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams,
    HoverProviderCapability, ImplementationProviderCapability, InitializeParams, InitializeResult,
    InitializedParams, InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams,
    LinkedEditingRangeParams, LinkedEditingRanges, Location, MarkupContent, MarkupKind,
    MessageType, OneOf, OptionalVersionedTextDocumentIdentifier, ParameterInformation,
    ParameterLabel, Position, PositionEncodingKind, PrepareRenameResponse, Range, ReferenceParams,
    Registration, RelatedFullDocumentDiagnosticReport, RelatedUnchangedDocumentDiagnosticReport,
    RenameFilesParams, RenameOptions, RenameParams, SelectionRange, SelectionRangeParams,
    SelectionRangeProviderCapability, SemanticTokens as LspSemanticTokens, SemanticTokensDelta,
    SemanticTokensDeltaParams, SemanticTokensEdit, SemanticTokensFullDeltaResult,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams,
    SemanticTokensRangeParams, SemanticTokensRangeResult, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, SignatureHelp,
    SignatureHelpOptions, SignatureHelpParams, SignatureInformation, SymbolInformation, SymbolKind,
    TextDocumentEdit, TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, TextEdit, TypeDefinitionProviderCapability, TypeHierarchyItem,
    TypeHierarchyPrepareParams, TypeHierarchySubtypesParams, TypeHierarchySupertypesParams,
    UnchangedDocumentDiagnosticReport, Uri, WatchKind, WillSaveTextDocumentParams,
    WorkDoneProgressOptions, WorkspaceDiagnosticParams, WorkspaceDiagnosticReport,
    WorkspaceDiagnosticReportResult, WorkspaceDocumentDiagnosticReport, WorkspaceEdit,
    WorkspaceFileOperationsServerCapabilities, WorkspaceFoldersServerCapabilities,
    WorkspaceFullDocumentDiagnosticReport, WorkspaceServerCapabilities, WorkspaceSymbolParams,
    WorkspaceSymbolResponse, WorkspaceUnchangedDocumentDiagnosticReport,
    request::{
        GotoDeclarationParams, GotoDeclarationResponse, GotoImplementationParams,
        GotoImplementationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse,
    },
};
use tower_lsp_server::{Client, LanguageServer};

/// Document store value: source text + dialect string.
#[derive(Debug, Clone)]
struct DocumentState {
    text: String,
    /// Persisted line-start index for `text`, built with the **LSP** EOL model
    /// ([`tcl_lexer::LineIndex::new_lsp`]) so the server's `(line, character)`
    /// coordinates match the client's (`\n`, `\r\n`, and lone `\r` all break a
    /// line). Kept in lock-step with `text`: every mutation of `text` rebuilds
    /// this alongside it (see [`apply_content_change_indexed`]).
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
        let line_index = tcl_lexer::LineIndex::new_lsp(&text);
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
        let line_index = tcl_lexer::LineIndex::new_lsp(&text);
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

/// Upper bound `semantic_tokens_full`/`_delta` wait for the fully enriched
/// (SSA/SCCP-informed: regex-source retagging, user-class object-method
/// resolution) token stream before falling back to the cheap coarse tier
/// (segmenter + registry only — no `CompilationUnit`/analysis) and letting the
/// enriched computation finish in the background (issue #829: semantic tokens
/// must not block indefinitely on a large/cold file's whole-file analysis).
/// Comparable to [`DIAGNOSTICS_DEBOUNCE`]: in the common case the diagnostics
/// worker has already primed the shared per-item analysis for this revision,
/// so the enriched query is a cache hit well inside this budget and callers
/// never see the fallback.
const SEMANTIC_TOKENS_FAST_PATH_BUDGET: std::time::Duration = std::time::Duration::from_millis(40);

/// Upper bound the diagnostics pipeline waits for the full deep pass — the
/// compiler / optimiser checks, the cross-file resolution, the W120 / W123
/// workspace refinement, and the lift — before publishing the cheap,
/// flicker-safe **fast tier** (#844): the workspace-independent syntax /
/// structural / style diagnostics, computed from the per-file analyser walk
/// alone.  A document whose whole pipeline settles inside this budget — a small
/// or warm file — never reaches the fast tier, so it costs no redundant publish
/// round-trip; only a large / cold document, whose deep pass overruns the
/// budget, gets the early tier while the deep pass keeps running and later
/// replaces it for the same version.  Sized like
/// [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`] (the two paths share the memoised
/// per-item analysis) and comfortably under [`DIAGNOSTICS_DEBOUNCE`].
///
/// The budget is a wall-clock race and so is only the *warm-file* half of the
/// debounce-skip; [`DIAGNOSTICS_FAST_TIER_MIN_LINES`] is the timing-independent
/// half that keeps trivial files a single publish regardless of cold-start.
const DIAGNOSTICS_FAST_TIER_BUDGET: std::time::Duration = std::time::Duration::from_millis(40);

/// Documents below this line count never get a fast tier — they go straight to
/// the single deep publish.  This is the **timing-independent** floor of the
/// debounce-skip: a trivial document's deep-pass wall-clock is dominated by
/// one-time warm-up (registry construction, the first salsa query) rather than
/// per-file work, so even its first analysis can overrun
/// [`DIAGNOSTICS_FAST_TIER_BUDGET`] on a cold server — yet a fast tier there is
/// a pure redundant round-trip, since the deep result is milliseconds behind.
/// Gating on size as well as the elapsed budget keeps small files a single
/// publish regardless of machine speed or cold-start jitter (the property the
/// `diagnostics_delivery_smoke` tests pin), while the budget race still
/// suppresses the fast tier for *large but warm* files whose memoised deep pass
/// lands inside the budget.  Set well above any trivial file yet far below the
/// multi-thousand-line documents #844 targets.
const DIAGNOSTICS_FAST_TIER_MIN_LINES: usize = 500;

/// The iRules dialect key.  A BIG-IP config's `ltm rule { … }` bodies are iRules
/// code, so they are tokenised against this registry rather than the config's
/// own `f5-bigip` one.
const IRULES_DIALECT: &str = "f5-irules";

/// The iApps dialect key — an APL presentation's embedded `[ … ]` Tcl.
const IAPPS_DIALECT: &str = "f5-iapps";

/// Ceiling on how many project files the background workspace warm (#844 Gap 3)
/// analyses concurrently.  The warm pre-populates the memoised per-file analysis
/// across the blocking pool so a cold workspace's first enriched
/// `semantic_tokens_project` finds cache hits instead of serially walking every
/// file; the cap keeps it from monopolising the blocking pool (or holding too
/// many salsa snapshots at once, which would stall a concurrent edit's
/// `set_text`) on a huge workspace, and it is clamped to the machine's parallelism
/// so small hosts stay responsive.
const WORKSPACE_WARM_MAX_CONCURRENCY: usize = 16;

/// Debounce window for coalescing `workspace/semanticTokens/refresh` pushes
/// (see [`SemanticTokensRefreshCtx::request_refresh_coalesced`]). Comparable
/// to [`DIAGNOSTICS_DEBOUNCE`]: many cold large documents finishing their
/// enriched computation around the same time (e.g. right after `initialized`
/// restores several tabs) would otherwise each fire their own workspace-wide
/// refresh; this collapses a burst into one fire per window.
const SEMANTIC_TOKENS_REFRESH_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(50);

/// What "still the current state" means for a diagnostics run at publish time,
/// and therefore what the currency guard re-checks under the `documents` lock
/// before delivering (`RUST_ISSUE_098`).
///
/// A run against an **open** buffer is current only while the document is still
/// open at the revision it was captured for.  A run against a **closed** but
/// on-disk workspace file (#865 — so its Problems / File-Explorer badge survives
/// the editor tab closing) is current only while the document is still closed:
/// a concurrent `did_open` makes the open buffer authoritative, so a late
/// closed-file publish must not land on top of it.
#[derive(Clone, Copy)]
enum DiagCurrency {
    /// Open buffer at this revision.
    Open(u64),
    /// A closed workspace file analysed from its on-disk contents at this
    /// per-URI generation (#865). The generation lets the publish-time guard
    /// drop a closed run that a newer close / watched-change refresh has
    /// superseded, so an older run cannot overwrite the current set.
    ClosedFromDisk(u64),
}

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
    /// Whether this run targets the open buffer or a closed on-disk file (#865),
    /// deciding what the publish-time currency guard re-checks.
    currency: DiagCurrency,
    version: Option<i32>,
    file: Option<tcl_lsp_db::SourceFile>,
    config: tcl_lsp_db::AnalyserConfig,
}

/// One document's last-published diagnostics plus the `result_id` that
/// identifies that exact set, for the pull-diagnostic path
/// (`textDocument/diagnostic` + `workspace/diagnostic`).  Kept in sync with
/// the push pipeline so a pull request returns the same diagnostics the editor
/// last received via `publish_diagnostics`, and an unchanged set can be
/// answered with a cheap `Unchanged` report.
#[derive(Clone)]
struct PullDiagEntry {
    result_id: String,
    /// The document revision the cached diagnostics were computed for.  The
    /// pull handler compares it against the live document so a cache entry from
    /// an older edit is recomputed rather than served as current.
    revision: u64,
    diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
}

/// Owned handles for the detached semantic-tokens background continuation —
/// the enriched (SSA/SCCP-informed) computation [`Backend::semantic_tokens_core_data`]
/// keeps running after [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`] elapses, so it can
/// still tell the client once it lands.  A small owned-handles struct (the
/// [`DiagInputs`] pattern) rather than borrowing `Backend`, since the
/// continuation runs on a detached `tokio::spawn` task with no lifetime tied to
/// the originating request.
struct SemanticTokensRefreshCtx {
    client: Client,
    last_semantic_tokens: SemanticTokensCache,
    refresh_pending: Arc<std::sync::atomic::AtomicBool>,
}

/// Per-URI cache of the last semantic-token stream served: `(resultId, packed
/// integer data)`. Shared type alias for [`Backend::last_semantic_tokens`] and
/// [`SemanticTokensRefreshCtx::last_semantic_tokens`] — the same cache, read
/// and written from both the request-handling and the detached-continuation
/// side.
type SemanticTokensCache = Arc<Mutex<HashMap<Uri, (String, Vec<u32>)>>>;

/// What a timed-out `semantic_tokens_range` request (#844 Gap 4) hands to its
/// convergence continuation: the partial CU / analysis results plus their reads.
/// Each `Option<Option<..>>` slot is `Some` iff that read landed within the
/// budget — in which case it is reused directly and its `JoinHandle` (still
/// carried, but spent) is never awaited again; a `None` slot means the read is
/// still un-consumed and the continuation awaits its handle. Capturing the slots
/// is what stops a CU that landed before the timeout from being lost (and its
/// spent handle re-polled) when the analysis read is the one that overran.
type RangeConvergencePending = (
    Option<Option<Arc<tcl_compiler::compilation_unit::CompilationUnit>>>,
    Option<Option<Arc<tcl_compiler::analyser::AnalysisResult>>>,
    tokio::task::JoinHandle<Option<Arc<tcl_compiler::compilation_unit::CompilationUnit>>>,
    tokio::task::JoinHandle<Option<Arc<tcl_compiler::analyser::AnalysisResult>>>,
);

/// The document-side inputs `spawn_range_convergence` needs to recompute the
/// enriched viewport off the LSP event loop and diff it against the coarse tier
/// already served (#844 Gap 4). Bundled so the detach helper stays under the
/// argument-count lint rather than threading six positional clones.
struct RangeConvergenceInputs {
    /// The document URI, for the settled-marker log line tests key on.
    uri: String,
    /// The coarse token stream already served, to diff the enriched recompute against.
    served: Vec<u32>,
    registry: Arc<CommandRegistry>,
    text: String,
    dialect: String,
    range: CoreLspRange,
}

impl SemanticTokensRefreshCtx {
    /// Compare the just-landed enriched token stream against whatever `uri`'s
    /// cache currently holds (the coarse tier served while the enriched
    /// computation was still running, or an earlier enriched stream); if it
    /// differs, ask the client to re-request semantic tokens so the
    /// enrichment (regex-source retagging, user-class object-method
    /// resolution) reaches the editor without waiting for the next edit.
    /// Deliberately does **not** write the cache itself — only a served
    /// `full`/`full/delta` response does that (see [`Backend::last_semantic_tokens`]);
    /// this only decides whether a refresh is worth asking for. Best-effort:
    /// a client without `workspace/semanticTokens/refresh` support rejects the
    /// request, which is harmless.
    async fn deliver_if_changed(&self, uri: &Uri, data: &[u32]) {
        let changed = {
            let cache = self.last_semantic_tokens.lock().await;
            cache.get(uri).is_none_or(|(_, cached)| cached != data)
        };
        if changed {
            self.request_refresh_coalesced();
        }
    }

    /// Coalesce concurrent refresh asks into one debounced
    /// `workspace/semanticTokens/refresh` per [`SEMANTIC_TOKENS_REFRESH_DEBOUNCE`]
    /// window. The request carries no URI — it asks the client to re-pull
    /// tokens for every document it has open — so collapsing N pending
    /// refresh intents into one fire loses nothing; the single re-request
    /// covers every URI. Guards against many cold large documents finishing
    /// their enriched computation near-simultaneously (e.g. right after
    /// `initialized` restores several tabs) each firing their own
    /// workspace-wide refresh.
    ///
    /// The first caller to flip `refresh_pending` false→true owns the fire:
    /// it spawns a task that sleeps out the debounce window, clears the flag,
    /// then sends the refresh. Every other caller during that window sees the
    /// flag already `true` and returns immediately — absorbed into the
    /// upcoming fire. Clearing the flag *before* the RPC (not after) means a
    /// result landing while the RPC itself is in flight schedules a fresh
    /// debounced fire rather than being silently dropped: at worst one extra
    /// fire, never a missed one.
    fn request_refresh_coalesced(&self) {
        if self
            .refresh_pending
            .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            return; // a fire is already scheduled — this result rides along.
        }
        let client = self.client.clone();
        let pending = Arc::clone(&self.refresh_pending);
        tokio::spawn(async move {
            tokio::time::sleep(SEMANTIC_TOKENS_REFRESH_DEBOUNCE).await;
            pending.store(false, std::sync::atomic::Ordering::Release);
            let _ = client.semantic_tokens_refresh().await;
        });
    }
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
    /// The freshest analyser inputs (registry + per-URI config toggles),
    /// refreshed by every [`Backend::schedule_diagnostics`] call.  The worker
    /// reads this at drain time rather than a snapshot captured when it spawned,
    /// so a config change (optimiser / diagnostics toggle) that arrives while
    /// the worker is mid-flight is analysed under the *new* toggles, not the
    /// stale ones — otherwise a disabled optimiser would keep emitting O-codes
    /// until the next document edit.
    latest_inputs: Option<DiagInputs>,
}

/// The two independent, both-default-off cross-file-adjacent toggles,
/// grouped out of [`DiagToggles`] to stay under its flat-bool-count lint:
/// `xcDiagnostics` (the opt-in XC100-301 translatability diagnostics —
/// only takes effect on `f5-irules` documents) and `crossFileResolution`
/// (opt-in cross-file W120/W123 suppression + cross-file E002/E003
/// arity — every dialect). Deliberately independent of each other; see
/// `Backend::xc_diagnostics_enabled` / `Backend::cross_file_resolution_enabled`.
#[derive(Clone, Copy)]
struct XcToggles {
    xc_diagnostics: bool,
    cross_file_resolution: bool,
}

/// The per-run analyser feature toggles for a diagnostics run, grouped so the
/// owned [`DiagInputs`] doesn't accumulate a flat row of `bool` fields.  Each is
/// resolved per folder in [`Backend::diag_inputs`].  (The client-capability
/// `client_supports_pull` is kept separate — it is a transport choice, not an
/// analyser feature.)
#[derive(Clone, Copy)]
struct DiagToggles {
    /// Master diagnostics switch (`tclLsp.features.diagnostics`). When `false`
    /// the pipeline publishes an empty set (clearing squiggles) instead of
    /// analysing.
    diagnostics_enabled: bool,
    /// `tclLsp.optimiser.enabled`: gates the optimiser/perf-hint diagnostics.
    optimiser_enabled: bool,
    /// The `xcDiagnostics` / `crossFileResolution` pair — see [`XcToggles`].
    xc: XcToggles,
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
    /// `tclLsp.diagnosticSeverity.<CODE>` per-code LSP severity overrides,
    /// resolved for this document's folder. Applied as a display-side re-label
    /// to the lifted diagnostics; empty ⇒ no overrides.
    severity_overrides: HashMap<String, tower_lsp_server::ls_types::DiagnosticSeverity>,
    /// `tclLsp.extraCommands` — names treated as known by W123.
    extra_commands: HashSet<String>,
    /// `tclLsp.diagnostics.genericVariablePatterns` — IRULE4002 generic-name
    /// patterns (`None` → default set, `Some` → replace). Only consulted on the
    /// no-salsa-input uncached fallback; the memoised path reads them off the
    /// salsa `AnalyserConfig`.
    generic_variable_patterns: Option<Vec<String>>,
    /// Resolved `tclLsp.style.lineLength` (W111 max line length) for this
    /// document's folder; fed to the source-style checks. Distinct from the
    /// formatter's `tclLsp.formatting.lineLength`.
    style_line_length: u32,
    non_ascii_mode: NonAsciiMode,
    opt_disabled: HashSet<String>,
    documents: Arc<Mutex<HashMap<Uri, DocumentState>>>,
    workspace_index: Arc<RwLock<core_workspace_index::WorkspaceIndex>>,
    /// M9: the applied source-site seed record (see [`Backend`]); the publish
    /// path invalidates a document's entry when it re-indexes it standalone,
    /// so the next cross-document query re-applies the seeded views.
    rehomed_source_seeds: Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// Package database for the W120 workspace-refinement post-filter (#723).
    package_resolver: Arc<RwLock<PackageResolver>>,
    /// `.tcl-lsp.ini [project] entryPoints` for this document's folder (#804):
    /// when non-empty, the W120 refinement inherits these entries' requires and
    /// disables the automatic `source`-graph inheritance.
    entry_points: Vec<String>,
    /// The document's workspace-folder root, used to resolve relative
    /// `entry_points` to file URIs. `None` for a no-folder session.
    folder_root: Option<PathBuf>,
    /// The salsa db handle.  Each run clones a *fresh, short-lived* snapshot and
    /// drops it immediately, so an idle worker never holds a clone across the
    /// debounce sleep (which would block the next edit's `set_text` — salsa's
    /// exclusive-access write waits for all outstanding handles to drop).
    db: Arc<Mutex<tcl_lsp_db::TclDatabase>>,
    db_files: Arc<Mutex<HashMap<Uri, tcl_lsp_db::SourceFile>>>,
    /// The salsa `Project` input handle (workspace file set), read by the worker
    /// for cross-file `project_diagnostics` when `crossFileResolution` is enabled.
    db_project: Arc<Mutex<Option<tcl_lsp_db::Project>>>,
    db_config: Arc<Mutex<tcl_lsp_db::AnalyserConfig>>,
    /// Per-folder salsa `AnalyserConfig` handles (see
    /// [`Backend::folder_db_configs`]); `capture_job` resolves the right one by
    /// the document's URI so folder-scoped W-code suppression reaches the
    /// cached analysis, falling back to `db_config`.
    folder_db_configs: Arc<Mutex<Vec<(Uri, tcl_lsp_db::AnalyserConfig)>>>,
    /// Pull-diagnostic cache, updated as each push run publishes so the
    /// `textDocument/diagnostic` / `workspace/diagnostic` paths return the
    /// last-published set.
    pull_diag_cache: Arc<Mutex<HashMap<Uri, PullDiagEntry>>>,
    /// Per-URI generation counter for **closed**-file diagnostics runs (#865).
    /// Each `publish_closed_file_diagnostics` bumps it and captures the new
    /// value into its `DiagCurrency::ClosedFromDisk`; the publish-time currency
    /// guard drops any closed run whose captured generation is no longer the
    /// latest, so an older run finishing after a newer one (rapid re-saves /
    /// overlapping close + watched-change) can never overwrite the current set
    /// with stale diagnostics — the closed-file equivalent of the open path's
    /// `revision` guard.
    closed_diag_gen: Arc<Mutex<HashMap<Uri, u64>>>,
    /// Per-run analyser feature toggles (diagnostics master switch, optimiser,
    /// `xcDiagnostics`, `crossFileResolution`), grouped to keep this owned
    /// struct's flat `bool` count low.
    toggles: DiagToggles,
    /// Snapshot of [`Backend::client_supports_pull_diagnostics`].  When `true`
    /// the worker keeps the pull cache current and asks the client to re-pull
    /// instead of pushing — see that field for the rationale (#721).
    client_supports_pull: bool,
}

impl DiagInputs {
    /// Capture the document's current, self-consistent diagnostics input (live
    /// buffer + salsa input handle + config).  Reading at drain time — after the
    /// debounce, once a burst's edits have settled — makes the worker robust to
    /// out-of-order edit processing: it always analyses the latest committed
    /// state.  `None` when the document is not open.
    async fn capture_job(&self, uri: &Uri) -> Option<DiagJob> {
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
            currency: DiagCurrency::Open(revision),
            version,
            file,
            config,
        })
    }

    /// The folder-scoped salsa [`tcl_lsp_db::AnalyserConfig`] handle for `uri`
    /// (longest matching folder override, else the process-global config) — the
    /// same resolution [`Self::capture_job`] applies, reused for the closed-file
    /// job capture (#865) so a closed file honours the same per-folder
    /// disabled-code / non-ASCII settings it did while open.
    async fn closed_file_config(&self, uri: &Uri) -> tcl_lsp_db::AnalyserConfig {
        let folder = self.folder_db_configs.lock().await;
        match longest_folder_match(&folder, uri) {
            Some(cfg) => *cfg,
            None => *self.db_config.lock().await,
        }
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
/// Deliver a freshly-computed diagnostic set to the client.
///
/// The pull cache is always updated by the caller *before* this runs, so a
/// `textDocument/diagnostic` request reflects the latest set either way. How
/// the client is *notified* then depends on what it supports:
///
/// - **Pull-capable client** (`client_supports_pull`): don't push. Ask the
///   client to re-pull via `workspace/diagnostic/refresh`; it then issues a
///   `textDocument/diagnostic` and reads the cache we just primed. Pushing
///   *and* pulling the same set makes such clients show every diagnostic
///   twice (#721); a refresh also covers cross-file (`crossFileResolution`)
///   updates the client would otherwise not know to re-pull.
/// - **Push-only client**: publish as before, the only channel it has.
///
/// The `publish_diagnostics(..).await` here must stay an **inline await**, never
/// a detached `tokio::spawn`: the transport's bounded, backpressured channel is
/// what guarantees no diagnostic is dropped or unboundedly buffered under a slow
/// client (the await *is* the wait-for-drain). Making it fire-and-forget would
/// discard that backpressure — reordering publishes and hiding a
/// state-gated/disconnected drop. See the delivery invariant in `main.rs`.
async fn deliver_diagnostics(
    client: &Client,
    uri: &Uri,
    diags: Vec<tower_lsp_server::ls_types::Diagnostic>,
    version: Option<i32>,
    client_supports_pull: bool,
) {
    if client_supports_pull {
        // Best-effort: a client that advertised pull support is expected to
        // honour the refresh, but a transport error here must not abort the
        // worker (the primed cache still serves the next manual pull).
        let _ = client.workspace_diagnostic_refresh().await;
    } else {
        client
            .publish_diagnostics(uri.clone(), diags, version)
            .await;
    }
}

/// The publish-side handles + per-edit identity shared by every settled
/// `run_diagnostics_core` path (master-off, F5, the analyser tail).  Borrows for
/// one call so the early-return paths and the final publish all deliver through
/// the same currency-guarded channel.
struct DeliveryCtx<'a> {
    client: &'a Client,
    documents: &'a Arc<Mutex<HashMap<Uri, DocumentState>>>,
    pull_diag_cache: &'a Arc<Mutex<HashMap<Uri, PullDiagEntry>>>,
    /// The closed-file generation map, consulted by the currency guard for a
    /// [`DiagCurrency::ClosedFromDisk`] run (#865).
    closed_diag_gen: &'a Arc<Mutex<HashMap<Uri, u64>>>,
    uri: &'a Uri,
    currency: DiagCurrency,
    version: Option<i32>,
    client_supports_pull: bool,
}

impl DeliveryCtx<'_> {
    /// Whether this run is still the current state for `uri`, evaluated against a
    /// held `documents` snapshot.  An open run is current while the buffer is
    /// open at the captured revision; a closed-file run (#865) is current while
    /// the buffer stays closed *and* no newer closed run has bumped the per-URI
    /// generation past the one this run captured — a reopen hands authority back
    /// to the open path, and a superseding close / watched-change refresh drops
    /// this older run so it cannot publish stale diagnostics.
    ///
    /// The `closed_diag_gen` lock is taken *inside* the held `documents` lock
    /// (the `documents` → `closed_diag_gen` order; the generation is only ever
    /// bumped without `documents` held), so the nesting is cycle-free.
    async fn is_current(&self, docs: &HashMap<Uri, DocumentState>) -> bool {
        match self.currency {
            DiagCurrency::Open(revision) => docs
                .get(self.uri)
                .is_some_and(|doc| doc.revision == revision),
            DiagCurrency::ClosedFromDisk(generation) => {
                !docs.contains_key(self.uri)
                    && self.closed_diag_gen.lock().await.get(self.uri) == Some(&generation)
            }
        }
    }

    /// The `PullDiagEntry.revision` to cache this set under.  An open run caches
    /// its editor revision so a later pull can match it; a closed-file run caches
    /// the `u64::MAX` sentinel, which no live editor revision reaches — so a
    /// reopened document's pull always recomputes rather than serving a stale
    /// closed-file set.
    fn revision_for_cache(&self) -> u64 {
        match self.currency {
            DiagCurrency::Open(revision) => revision,
            DiagCurrency::ClosedFromDisk(_) => u64::MAX,
        }
    }

    /// Publish `diags` iff this run is still current for `uri`, holding the
    /// `documents` lock across the currency check AND the pull-cache/publish
    /// delivery so a concurrent `did_close`/`did_change` cannot interleave
    /// between them — otherwise a `did_close` that lands in that window clears
    /// the squiggles and drops the pull-cache entry, only for this run's late
    /// delivery to re-publish and re-cache them for the now-closed document
    /// (`RUST_ISSUE_098`).  Returns whether it published — a superseded (no
    /// longer current) run returns `false` without touching the client.
    async fn deliver_if_current(&self, diags: Vec<tower_lsp_server::ls_types::Diagnostic>) -> bool {
        let docs = self.documents.lock().await;
        if self.is_current(&docs).await {
            self.cache_and_deliver(diags).await;
            true
        } else {
            false
        }
    }

    /// Deliver the #844 progressive **fast tier** — push-only, and only to a
    /// push client — iff the document is still at this run's `revision`.
    ///
    /// Deliberately *not* [`deliver_if_current`]: the fast tier must **never**
    /// prime the pull-diagnostic cache (trap #3).  The pull path
    /// (`textDocument/diagnostic`) always serves or computes the *complete* deep
    /// set; a cache primed with the incomplete fast tier would let a pull in the
    /// window return a partial report.  A pull-capable client is skipped outright
    /// — its "early" signal would be a `workspace/diagnostic/refresh`, but a
    /// re-pull recomputes the full deep set synchronously, which defeats the fast
    /// tier's whole purpose, so such a client just gets the deep tier's refresh
    /// as before.  Currency-guarded under the `documents` lock exactly like the
    /// deep publish (held across the push), so a superseding edit or a
    /// `did_close` in the window can never let a stale fast tier land
    /// (`RUST_ISSUE_098`).
    /// Publish the fast tier for a push client, iff this run's revision is still
    /// current. Skipped for a pull client (the pull path always serves the
    /// complete deep set, never the reduced fast tier).
    ///
    /// Like [`publish_diagnostics_result`], this holds the `documents` lock
    /// **across** `publish_diagnostics().await`, and that is load-bearing: the
    /// lock-across-send is what closes `RUST_ISSUE_098`. Releasing `documents`
    /// before the send would let a `did_close` clearing-publish land between the
    /// currency check and the send, repainting squiggles on a now-closed document
    /// that nothing downstream clears (the deep pass then finds `!is_current` and
    /// returns without publishing). This is a *second* lock-held-across-send
    /// publish per cycle, so under a transiently slow client (the bounded
    /// `channel(1)` transport parks the send) it would otherwise freeze every other
    /// document's `did_change`/hover for however long that client takes to drain.
    /// To cap that, the send is bounded by `timeout(DIAGNOSTICS_FAST_TIER_BUDGET,
    /// …)`: the lock is held for at most the budget, after which this best-effort
    /// push is dropped (the deep tier still supersedes it — the same outcome as the
    /// already-accepted lift-worker-panic drop). The lock is *not* released before
    /// the send — that would reopen `RUST_ISSUE_098` — only time-bounded.
    async fn deliver_fast_tier_if_current(
        &self,
        diags: Vec<tower_lsp_server::ls_types::Diagnostic>,
    ) {
        if self.client_supports_pull {
            return;
        }
        let docs = self.documents.lock().await;
        if self.is_current(&docs).await {
            // Best-effort/droppable (see above): cap the lock hold so a slow client
            // can't hold `documents` hostage for every other document's edits.
            // Dropping the parked `channel(1)` send on timeout is atomic — the push
            // is simply lost and the deep tier supersedes it.
            let _ = tokio::time::timeout(
                DIAGNOSTICS_FAST_TIER_BUDGET,
                self.client
                    .publish_diagnostics(self.uri.clone(), diags, self.version),
            )
            .await;
        }
    }

    /// Update the pull-diagnostic cache for `uri` to `diags` at this run's
    /// `revision`, with a fresh `result_id`, then notify the client (push or
    /// refresh) — the publish half shared by every settled path.
    async fn cache_and_deliver(&self, diags: Vec<tower_lsp_server::ls_types::Diagnostic>) {
        self.pull_diag_cache.lock().await.insert(
            self.uri.clone(),
            PullDiagEntry {
                result_id: next_pull_diag_result_id(),
                revision: self.revision_for_cache(),
                diagnostics: diags.clone(),
            },
        );
        deliver_diagnostics(
            self.client,
            self.uri,
            diags,
            self.version,
            self.client_supports_pull,
        )
        .await;
    }
}

/// Master switch (`tclLsp.features.diagnostics = false`): publish an empty set
/// so any existing squiggles clear, then settle — the analyser, compiler
/// checks, and F5 validators are all skipped.  Always settled (`true`).
async fn run_diagnostics_master_off(delivery: &DeliveryCtx<'_>) -> bool {
    if delivery.deliver_if_current(Vec::new()).await {
        // Observability parity with the deep path's `[timing] deep diagnostics`
        // marker (emitted only after a real publish).  An empty-set publish
        // onto an already-empty collection may fire no client-side
        // diagnostics-changed event, so this marker is the reliable signal
        // that the master-switch-off pass actually ran and cleared this URI —
        // the VS Code harness' `waitForMasterOffDiagnostics` keys on this exact
        // line + `uri=`, and it aids field debugging of "why are my squiggles
        // gone" the same way the deep marker does.
        delivery
            .client
            .log_message(
                MessageType::LOG,
                format!(
                    "[timing] diagnostics master-off 0ms (uri={uri}, diags=0)",
                    uri = delivery.uri.as_str(),
                ),
            )
            .await;
    }
    true
}

/// F5 dialect dispatch: BIG-IP config and iApp APL presentation documents are
/// not Tcl source — they have model-level validators (`BIGIP6001`-`6011`,
/// `IAPP7001`-`7003`) rather than the Tcl analyser.  Compute and publish their
/// diagnostics here, before the analyser.  Returns `Some(true)` (settled) when
/// the document is an F5 model dialect; `None` to continue to the Tcl analyser.
async fn run_diagnostics_f5_dialect(
    delivery: &DeliveryCtx<'_>,
    disabled: &HashSet<String>,
    text: &str,
    dialect: &str,
    language_id: &str,
) -> Option<bool> {
    let diags = f5_dialect_diagnostics(
        delivery.uri,
        text,
        dialect,
        language_id,
        disabled,
        delivery.documents,
    )
    .await?;
    // Publish only when this version is still current (the same revision
    // guard the analyser path applies before publishing), and keep the
    // pull-diagnostic cache in lock-step so `textDocument/diagnostic`
    // returns the same set.
    delivery.deliver_if_current(diags).await;
    Some(true)
}

/// The salsa handles + per-edit identity shared by the two cancellable analysis
/// passes ([`compute_base_analysis`], [`compute_compiler_diags`]).  Borrows for
/// the duration of one `run_diagnostics_core` call.
struct SalsaAnalysisCtx<'a> {
    db: &'a Arc<Mutex<tcl_lsp_db::TclDatabase>>,
    uri: &'a Uri,
    file: Option<tcl_lsp_db::SourceFile>,
    config: tcl_lsp_db::AnalyserConfig,
    text: &'a str,
    dialect: &'a str,
}

/// Base analysis: the cancellable salsa `file_analysis_incremental` query
/// (slice 5), off the LSP event loop — the whole-file per-item walk that
/// dominates the deep pass and feeds *both* the workspace-independent fast tier
/// (#844) and the deep tier.  `ctx.file` is `None` only if the salsa input is
/// somehow absent; then fall back to a direct (uncached) analyse.  The cross-file
/// `project_diagnostics` pass is now a separate query ([`compute_project_diags`])
/// so it can run concurrently with this one and so the fast tier can publish
/// before it.
///
/// `Continue(analysis)` carries the result; `Break(settled)` is the early return
/// for the deep pass — `Break(false)` on a genuine salsa cancellation (retry the
/// latest state), `Break(true)` on a deterministic worker panic (settle rather
/// than livelock the debounce loop).
async fn compute_base_analysis(
    ctx: &SalsaAnalysisCtx<'_>,
    disabled: &HashSet<String>,
    extra_commands: &HashSet<String>,
    non_ascii_mode: NonAsciiMode,
    registry: &CommandRegistry,
    workspace_index: &Arc<RwLock<core_workspace_index::WorkspaceIndex>>,
    package_resolver: &Arc<RwLock<PackageResolver>>,
) -> ControlFlow<bool, Arc<AnalysisResult>> {
    let &SalsaAnalysisCtx {
        db,
        uri,
        file,
        config,
        text,
        dialect,
    } = ctx;

    // A document with an unclosed delimiter needs a wider "known command"
    // universe than the folder/global-scoped `extra_commands` salsa input
    // carries: the workspace's own procs/classes and the commands *this*
    // document's `package require`s resolve to (the recovery known-command
    // hierarchy — see `tcl_compiler::analyser::utils::recovery_known_commands`,
    // which this widens one layer up). Resolving that is real work (a
    // workspace-index read, package-database lookup + file reads) and is
    // inherently per-document — a document's own `package require`s decide
    // which package commands apply, which the shared, folder/global-scoped
    // salsa `AnalyserConfig` cannot express — so this bypasses the cached path
    // entirely and only runs on this rare, edit-transient branch, mirroring
    // `recovery_known_commands`'s own `script_is_complete` gate: a
    // well-formed document pays nothing extra.
    if !tcl_lexer::script_is_complete(text) {
        let widened = widen_recovery_extra_commands(
            extra_commands,
            text,
            dialect,
            registry,
            workspace_index,
            package_resolver,
        )
        .await;
        let (a_text, a_dialect, a_disabled) =
            (text.to_owned(), dialect.to_owned(), disabled.clone());
        return match tokio::task::spawn_blocking(move || {
            Backend::configured_analyser(a_disabled, non_ascii_mode, widened)
                .analyse(&a_text, &a_dialect)
                .clone()
        })
        .await
        {
            Ok(analysis) => ControlFlow::Continue(Arc::new(analysis)),
            Err(e) => {
                eprintln!(
                    "tcl-lsp: recovery-path diagnostics worker panicked for {} (is_panic={}); \
                     skipping this document's diagnostics to avoid a retry livelock",
                    uri.as_str(),
                    e.is_panic(),
                );
                ControlFlow::Break(true)
            }
        };
    }

    if let Some(file) = file {
        // Clone a fresh, short-lived snapshot for just this read and move it
        // into the worker; it drops when the read finishes, so it never holds
        // exclusive-access-blocking references across the debounce sleep.
        let snapshot = db.lock().await.clone();
        match tokio::task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| {
                tcl_lsp_db::file_analysis_incremental(&snapshot, file, config)
            })
            .ok()
        })
        .await
        {
            Ok(Some(analysis)) => ControlFlow::Continue(analysis),
            // A genuine salsa cancellation (a concurrent `set_text` on the
            // shared db) — don't publish; signal a retry of the document's
            // latest state.
            Ok(None) => ControlFlow::Break(false),
            // The worker PANICKED. A panic in the analysis pipeline is
            // deterministic for this document, so retrying it livelocks the
            // debounce loop (~20 failed analyses/second, diagnostics never
            // published). Treat the run as *settled* so the scheduler stops
            // re-marking the slot dirty; the document keeps its prior
            // diagnostics rather than spinning the CPU.
            Err(e) => {
                eprintln!(
                    "tcl-lsp: diagnostics worker panicked for {} (is_panic={}); \
                     skipping this document's diagnostics to avoid a retry livelock",
                    uri.as_str(),
                    e.is_panic(),
                );
                ControlFlow::Break(true)
            }
        }
    } else {
        let (a_text, a_dialect, a_disabled) =
            (text.to_owned(), dialect.to_owned(), disabled.clone());
        let a_extra = extra_commands.clone();
        let analysis = tokio::task::spawn_blocking(move || {
            Arc::new(
                Backend::configured_analyser(a_disabled, non_ascii_mode, a_extra)
                    .analyse(&a_text, &a_dialect)
                    .clone(),
            )
        })
        .await
        .unwrap_or_default();
        ControlFlow::Continue(analysis)
    }
}

/// Widen `base` (the resolved `tclLsp.extraCommands`) with the workspace's
/// own proc/class names and the commands available to `text` through package
/// resolution — the LSP-layer name-resolution hierarchy the unclosed-
/// delimiter recovery heuristics need beyond what a single-file `Analyser`
/// can see on its own. `tcl_compiler::analyser::utils::recovery_known_commands`
/// unions the registry with the document's own signature scan; this unions
/// one layer further out: every workspace-indexed proc/class (regardless of
/// which file defines it — `WorkspaceIndex::procs`/`classes`), every
/// auto-loadable command the scanned library paths provide (`tclIndex`-style,
/// no `package require` needed — mirrors the #832 W123 refinement), and, when
/// `text` itself `package require`s something, the commands that package's
/// resolved implementation files define.
///
/// `text`'s own `package require`s are read via a fresh signature scan
/// (mirroring `recovery_known_commands`'s own in-file scan) rather than
/// `WorkspaceIndex::package_requires_for` — the index only learns a
/// document's requires from an *already-published* analysis, which this
/// document, being analysed for the first time right now, cannot yet have.
///
/// Workspace names go through `tcl_compiler::analyser::utils::insert_qualified_and_tail`
/// — the same three-form (as-is / `::`-stripped / tail) insertion
/// `recovery_known_commands` uses for a document's own procs/classes/
/// aliases/renames — rather than a second, hand-rolled copy: a workspace
/// proc referenced by its absolute `::ns::name` form needs recognising just
/// as much as one referenced relatively.
///
/// Only called from [`compute_base_analysis`]'s `!script_is_complete`
/// recovery branch — never on the well-formed-document hot path.
async fn widen_recovery_extra_commands(
    base: &HashSet<String>,
    text: &str,
    dialect: &str,
    registry: &CommandRegistry,
    workspace_index: &Arc<RwLock<core_workspace_index::WorkspaceIndex>>,
    package_resolver: &Arc<RwLock<PackageResolver>>,
) -> HashSet<String> {
    let mut names = base.clone();
    {
        let index = workspace_index.read().await;
        for p in index.procs() {
            tcl_compiler::analyser::utils::insert_qualified_and_tail(&mut names, &p.qualified_name);
        }
        for c in index.classes() {
            tcl_compiler::analyser::utils::insert_qualified_and_tail(&mut names, &c.qualified_name);
        }
    }
    let requires: Vec<String> = tcl_compiler::signature_scan::extract_signatures(text, registry)
        .package_requires
        .into_iter()
        .map(|pr| pr.name)
        .collect();
    let resolver = package_resolver.read().await;
    for name in resolver.auto_command_names() {
        names.insert(name.trim_start_matches("::").to_owned());
    }
    if !requires.is_empty() {
        let commands = resolver.package_defined_commands(&requires, &|path| {
            std::fs::read_to_string(path)
                .map(|text| defined_command_tails(&text, dialect))
                .unwrap_or_default()
        });
        names.extend(commands);
    }
    names
}

/// The cross-file analyser diagnostics for the deep tier: `project_diagnostics`
/// — the per-file set with a workspace-proc-resolvable W123 suppressed and a
/// cross-file `E002`/`E003` arity error synthesised for a bad-arity call to a
/// workspace proc — when a `Project` is indexed and this document has a salsa
/// input.  `Continue(None)` means "no cross-file pass" (`crossFileResolution`
/// off, or no salsa input): the caller falls back to the per-file
/// `analysis.diagnostics`, matching the pre-split behaviour.
///
/// Split out of [`compute_base_analysis`] so it can run concurrently with the
/// base walk and the compiler checks (#844 Gap 2) and so the workspace
/// -independent fast tier can publish before this workspace resolution lands.
/// It reuses the memoised per-item analysis rather than re-walking the file, so
/// the split adds no second whole-file analysis.  `Break(settled)` mirrors
/// [`compute_base_analysis`] — `Break(false)` on cancellation (retry),
/// `Break(true)` on a deterministic worker panic (settle).
async fn compute_project_diags(
    ctx: &SalsaAnalysisCtx<'_>,
    project: Option<tcl_lsp_db::Project>,
) -> ControlFlow<bool, Option<Vec<tcl_compiler::analyser::Diagnostic>>> {
    let &SalsaAnalysisCtx {
        db,
        uri,
        file,
        config,
        ..
    } = ctx;
    let (Some(project), Some(file)) = (project, file) else {
        return ControlFlow::Continue(None);
    };
    let snapshot = db.lock().await.clone();
    match tokio::task::spawn_blocking(move || {
        salsa::Cancelled::catch(|| {
            (*tcl_lsp_db::project_diagnostics(&snapshot, file, config, project)).clone()
        })
        .ok()
    })
    .await
    {
        Ok(Some(d)) => ControlFlow::Continue(Some(d)),
        Ok(None) => ControlFlow::Break(false),
        Err(e) => {
            eprintln!(
                "tcl-lsp: cross-file diagnostics worker panicked for {} (is_panic={}); \
                 skipping this document's diagnostics to avoid a retry livelock",
                uri.as_str(),
                e.is_panic(),
            );
            ControlFlow::Break(true)
        }
    }
}

/// Optimiser / compiler-checks diagnostics, also off the event loop via the
/// cancellable salsa `compiler_check_diagnostics` query: the unit's
/// per-procedure lattices are memoised by `function_lattice` and shared with
/// the analyser tail.  `file` is `None` only if the salsa input is absent; then
/// build directly (uncached).
///
/// `Break(false)` on cancellation (retry), `Break(true)` on a deterministic
/// worker panic (settle) — matching [`compute_base_analysis`].
async fn compute_compiler_diags(
    ctx: &SalsaAnalysisCtx<'_>,
    registry: &Arc<CommandRegistry>,
    generic_variable_patterns: Option<&[String]>,
) -> ControlFlow<bool, Arc<tcl_lsp_db::CompilerDiagnostics>> {
    let &SalsaAnalysisCtx {
        db,
        uri,
        file,
        config,
        text,
        dialect,
    } = ctx;
    if let Some(file) = file {
        let snapshot = db.lock().await.clone();
        match tokio::task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| {
                tcl_lsp_db::compiler_check_diagnostics(&snapshot, file, config)
            })
            .ok()
        })
        .await
        {
            Ok(Some(d)) => ControlFlow::Continue(d),
            // Genuine cancellation — retry the latest state.
            Ok(None) => ControlFlow::Break(false),
            // Deterministic worker panic — settle instead of livelocking.
            Err(e) => {
                eprintln!(
                    "tcl-lsp: compiler-check worker panicked for {} (is_panic={}); \
                     skipping this document's diagnostics to avoid a retry livelock",
                    uri.as_str(),
                    e.is_panic(),
                );
                ControlFlow::Break(true)
            }
        }
    } else {
        let (c_text, c_dialect) = (text.to_owned(), dialect.to_owned());
        let c_registry = Arc::clone(registry);
        let c_generic = generic_variable_patterns.map(<[String]>::to_vec);
        ControlFlow::Continue(
            tokio::task::spawn_blocking(move || {
                Arc::new(tcl_lsp_db::compiler_check_diagnostics_uncached(
                    &c_text,
                    &c_registry,
                    &c_dialect,
                    c_generic.as_deref(),
                ))
            })
            .await
            .unwrap_or_else(|_| {
                Arc::new(tcl_lsp_db::CompilerDiagnostics {
                    checks: Vec::new(),
                    optimisations: Vec::new(),
                })
            }),
        )
    }
}

async fn run_diagnostics_core(inputs: DiagInputs, uri: &Uri, job: DiagJob) -> bool {
    // Returns whether this version is **settled** (published, intentionally
    // skipped as superseded, or a BIG-IP no-op).  `false` means the run was
    // cancelled mid-flight and the caller should retry the document's latest
    // state.
    let DiagInputs {
        client,
        registry,
        disabled,
        severity_overrides,
        extra_commands,
        generic_variable_patterns,
        style_line_length,
        non_ascii_mode,
        opt_disabled,
        documents,
        workspace_index,
        rehomed_source_seeds,
        package_resolver,
        entry_points,
        folder_root,
        db,
        db_project,
        pull_diag_cache,
        closed_diag_gen,
        toggles,
        client_supports_pull,
        // The worker captures the job from these before calling us; unused here.
        db_files: _,
        db_config: _,
        folder_db_configs: _,
    } = inputs;
    let DiagToggles {
        diagnostics_enabled,
        optimiser_enabled,
        xc: XcToggles {
            xc_diagnostics,
            cross_file_resolution,
        },
    } = toggles;
    let DiagJob {
        text,
        dialect,
        language_id,
        currency,
        version,
        file,
        config,
    } = job;

    let delivery = DeliveryCtx {
        client: &client,
        documents: &documents,
        pull_diag_cache: &pull_diag_cache,
        closed_diag_gen: &closed_diag_gen,
        uri,
        currency,
        version,
        client_supports_pull,
    };

    if !diagnostics_enabled {
        return run_diagnostics_master_off(&delivery).await;
    }

    if let Some(settled) =
        run_diagnostics_f5_dialect(&delivery, &disabled, &text, &dialect, &language_id).await
    {
        return settled;
    }

    let salsa_ctx = SalsaAnalysisCtx {
        db: &db,
        uri,
        file,
        config,
        text: &text,
        dialect: &dialect,
    };
    let lift_inputs = LiftInputs {
        text: &text,
        dialect: &dialect,
        disabled: &disabled,
        severity_overrides: &severity_overrides,
        opt_disabled: &opt_disabled,
        optimiser_enabled,
        style_line_length,
        xc_diagnostics,
    };
    run_diagnostics_analyser_path(
        &delivery,
        &salsa_ctx,
        &lift_inputs,
        &AnalyserPathInputs {
            registry: &registry,
            extra_commands: &extra_commands,
            generic_variable_patterns: generic_variable_patterns.as_deref(),
            non_ascii_mode,
            db_project: &db_project,
            workspace_index: &workspace_index,
            rehomed_source_seeds: &rehomed_source_seeds,
            package_resolver: &package_resolver,
            entry_points: &entry_points,
            folder_root: folder_root.as_deref(),
            cross_file_resolution,
        },
    )
    .await
}

/// The non-document-buffer handles the analyser/compiler/publish path needs.
struct AnalyserPathInputs<'a> {
    registry: &'a Arc<CommandRegistry>,
    extra_commands: &'a HashSet<String>,
    generic_variable_patterns: Option<&'a [String]>,
    non_ascii_mode: NonAsciiMode,
    db_project: &'a Arc<Mutex<Option<tcl_lsp_db::Project>>>,
    workspace_index: &'a Arc<RwLock<core_workspace_index::WorkspaceIndex>>,
    /// M9 applied-seed record; the publish path invalidates entries.
    rehomed_source_seeds: &'a Arc<Mutex<HashMap<String, Vec<String>>>>,
    package_resolver: &'a Arc<RwLock<PackageResolver>>,
    /// #804 W120 inheritance for this document's folder (see [`DiagInputs`]).
    entry_points: &'a [String],
    folder_root: Option<&'a Path>,
    /// Whether opt-in cross-file resolution is enabled — see
    /// `Backend::cross_file_resolution_enabled`. Independent of
    /// `LiftInputs::xc_diagnostics` (the unrelated f5-irules-specific
    /// XC100-301 translatability lints).
    cross_file_resolution: bool,
}

/// The Tcl analyser path, made **progressive** (#844): the deep pass — base
/// analysis, compiler checks, and cross-file resolution (all cancellable, off
/// the event loop), the W120/W123 workspace refinement, the diagnostic lifts,
/// and the final currency-guarded publish — runs as one future, raced against
/// [`DIAGNOSTICS_FAST_TIER_BUDGET`].  If it settles inside the budget (a small
/// or warm file) the client sees a single publish, exactly as before.  If it
/// overruns (a large or cold file), the workspace-independent **fast tier**
/// (syntax / structural / style diagnostics, everything but W120/W123) is
/// published first so the user gets the bulk of the diagnostics without waiting
/// on the compiler/optimiser + cross-file + refinement passes; the deep pass
/// then finishes and replaces it for the same version.
///
/// Returns `false` only on a genuine salsa cancellation (the caller retries the
/// document's latest state); the fast tier never changes that verdict — the deep
/// pass is always the authority on whether the version settled.
async fn run_diagnostics_analyser_path(
    delivery: &DeliveryCtx<'_>,
    salsa_ctx: &SalsaAnalysisCtx<'_>,
    lift_inputs: &LiftInputs<'_>,
    inputs: &AnalyserPathInputs<'_>,
) -> bool {
    let started = std::time::Instant::now();
    let uri_str = delivery.uri.to_string();
    let line_count = salsa_ctx.text.lines().count();

    // The `Project` handle for the cross-file pass (stable across re-sets); `None`
    // ⇒ no cross-file pass (status-quo per-file diagnostics).
    let project = if inputs.cross_file_resolution {
        *inputs.db_project.lock().await
    } else {
        None
    };
    let timing = PublishTiming {
        started,
        uri_str: &uri_str,
        line_count,
    };

    // Compute the base per-file analysis once and share it (`.shared()`): the deep
    // pass awaits it inside its `join!` (concurrently with the compiler and
    // cross-file passes, #844 Gap 2), and — when the budget elapses — the fast tier
    // awaits the *same* future for its coarse publish. `Shared` lets any polled
    // clone drive the read, so the fast-tier await below still makes progress while
    // the deep future is parked. This is the explicit form of what salsa's
    // block-and-share gave implicitly, but without the second `compute_base_analysis`
    // call parking a blocking-pool thread on the already-in-flight base read.
    let base = compute_base_analysis(
        salsa_ctx,
        lift_inputs.disabled,
        inputs.extra_commands,
        inputs.non_ascii_mode,
        inputs.registry,
        inputs.workspace_index,
        inputs.package_resolver,
    )
    .shared();

    // Trivial documents skip the progressive machinery entirely and take the
    // single-publish path, so cold-start warm-up can never turn a one-line file
    // into two publishes (see [`DIAGNOSTICS_FAST_TIER_MIN_LINES`]).
    if line_count < DIAGNOSTICS_FAST_TIER_MIN_LINES {
        return run_deep_diagnostics(
            delivery,
            salsa_ctx,
            lift_inputs,
            inputs,
            project,
            timing,
            base,
        )
        .await;
    }

    // The authoritative deep pass performs the full publish and reports whether
    // the version settled.  Race it against the fast-tier budget.
    let fast_tier_deadline = tokio::time::Instant::now() + DIAGNOSTICS_FAST_TIER_BUDGET;
    let deep = run_deep_diagnostics(
        delivery,
        salsa_ctx,
        lift_inputs,
        inputs,
        project,
        timing,
        base.clone(),
    );
    tokio::pin!(deep);

    tokio::select! {
        biased;
        // The whole pipeline finished inside the budget: the deep publish is the
        // one and only publish, so the fast tier is never sent (no redundant
        // round-trip on small / warm files — the debounce-skip trap #844 calls
        // out).  `biased` prefers this arm so a deep pass that lands right on the
        // deadline still skips the fast tier.
        settled = &mut deep => return settled,
        () = tokio::time::sleep_until(fast_tier_deadline) => {}
    }

    // Budget elapsed with the deep pass still running: publish the flicker-safe
    // fast tier now, from the shared base future the deep pass is already awaiting
    // — one in-flight read, never a second whole-file walk. A cancellation here is
    // harmless: the deep pass observes the same edit and settles it.
    if let ControlFlow::Continue(analysis) = base.await {
        publish_fast_tier(delivery, &analysis, lift_inputs).await;
    }
    deep.await
}

/// The deep diagnostics pass: the three independent whole-file analyses run
/// concurrently (#844 Gap 2) — the per-file analyser walk, the compiler /
/// optimiser checks, and the cross-file resolution — then the W120/W123
/// workspace refinement, the diagnostic lifts, and the single authoritative
/// currency-guarded publish.  Only the downstream refine + lift consume all
/// three passes, so overlapping them collapses the deep pass towards its longest
/// single pass.  The one thing given up is fail-fast on a base-analysis
/// cancellation — the compiler / cross-file passes may do a little wasted work
/// before observing the same cancellation, the trade #844 explicitly accepts.
///
/// `base` is the per-file analyser walk, supplied by the caller (as a `Shared`
/// future) rather than started here, so the progressive fast tier can await the
/// *same* computation instead of issuing a second one; it is simply the first arm
/// of the `join!`.
///
/// Returns whether the version **settled** (published or superseded), matching
/// the old serial path — `false` only on a genuine salsa cancellation. A
/// deterministic worker panic in a *secondary* pass (compiler / cross-file)
/// degrades that pass to its empty / per-file fallback and still publishes the
/// currency-guarded deep tier, so the fast tier — which may already have replaced
/// the client's set with its reduced subset — is never left as the terminal
/// state (#844).
async fn run_deep_diagnostics(
    delivery: &DeliveryCtx<'_>,
    salsa_ctx: &SalsaAnalysisCtx<'_>,
    lift_inputs: &LiftInputs<'_>,
    inputs: &AnalyserPathInputs<'_>,
    project: Option<tcl_lsp_db::Project>,
    timing: PublishTiming<'_>,
    base: impl std::future::Future<Output = ControlFlow<bool, Arc<AnalysisResult>>>,
) -> bool {
    let (base_result, compiler_result, project_result) = tokio::join!(
        base,
        compute_compiler_diags(salsa_ctx, inputs.registry, inputs.generic_variable_patterns),
        compute_project_diags(salsa_ctx, project),
    );
    let analysis: Arc<AnalysisResult> = match base_result {
        ControlFlow::Continue(a) => a,
        ControlFlow::Break(settled) => return settled,
    };
    let compiler_diags: Arc<tcl_lsp_db::CompilerDiagnostics> = match compiler_result {
        ControlFlow::Continue(d) => d,
        // Cancellation → retry the latest state.
        ControlFlow::Break(false) => return false,
        // Deterministic worker panic → degrade to no compiler diags but STILL
        // publish below. The fast tier may already have replaced the client's
        // complete set with its reduced subset; an early return here would leave
        // that reduced set as the terminal state (settled ⇒ no retry, #844) —
        // stripping the O1xx/refined-W12x/cross-file findings the user had. The
        // degraded deep publish is still a strict superset of the fast tier, just
        // without the compiler/optimiser hints.
        ControlFlow::Break(true) => Arc::new(tcl_lsp_db::CompilerDiagnostics {
            checks: Vec::new(),
            optimisations: Vec::new(),
        }),
    };
    // `Some` is the cross-file-resolved set (`crossFileResolution` on, salsa
    // input present); `None` falls back to the per-file set, matching the pre-split
    // behaviour — as does a deterministic worker panic (`Break(true)`), which
    // degrades to the per-file set and still publishes rather than stranding the
    // fast tier (see the compiler arm above). `Break(false)` is cancellation.
    let analyser_diags: Vec<_> = match project_result {
        ControlFlow::Continue(Some(d)) => d,
        ControlFlow::Continue(None) | ControlFlow::Break(true) => analysis.diagnostics.clone(),
        ControlFlow::Break(false) => return false,
    };

    // #804: extra requires this document inherits from configured entry points
    // or its `source` ancestors, resolved against the live workspace index.
    // Only computed when there is a W120 or W123 to refine (#832 uses inherited
    // requires for its package-source resolution) — otherwise the index lock and
    // `source`-graph walk are wasted work on the hot diagnostics path.
    let inherited_requires = if analyser_diags
        .iter()
        .any(|d| d.code == DiagCode::W120 || d.code == DiagCode::W123)
    {
        let index = inputs.workspace_index.read().await;
        compute_inherited_requires(
            &index,
            delivery.uri,
            inputs.entry_points,
            inputs.folder_root,
        )
    } else {
        Vec::new()
    };
    let result = refine_and_lift_diagnostics(
        &analysis,
        analyser_diags,
        &compiler_diags,
        &inherited_requires,
        inputs.package_resolver,
        inputs.registry,
        lift_inputs,
    )
    .await;

    publish_diagnostics_result(
        delivery,
        inputs.workspace_index,
        inputs.rehomed_source_seeds,
        &analysis,
        result,
        timing,
    )
    .await
}

/// Whether a diagnostic code belongs to the workspace-independent **fast tier**
/// (#844) that [`publish_fast_tier`] delivers ahead of the deep pass.
///
/// The classification lives on [`DiagCode::refined_by_workspace`] (the single
/// source of truth for which codes a workspace / cross-file pass can retract) —
/// this consumer only *asks* it, never re-encodes the set.  A code is fast
/// unless the deep pass might refine it away; the deep pass only ever *removes*
/// those (currently W120/W123) and *adds* new diagnostics (compiler/optimiser,
/// synthesised cross-file arity), so the fast tier is a strict subset of the
/// deep tier — no fast-tier diagnostic is ever contradicted (no false-positive
/// flicker).
const fn is_fast_tier(code: DiagCode) -> bool {
    !code.refined_by_workspace()
}

/// Publish the flicker-safe **fast tier** (#844): the workspace-independent
/// analyser diagnostics ([`is_fast_tier`]) plus the pure source-style lints,
/// lifted off the event loop.  Delivered push-only through
/// [`DeliveryCtx::deliver_fast_tier_if_current`] (never priming the pull cache,
/// see that method), currency-guarded so a superseding edit can never let this
/// land after the deep tier for the same version.  A lift-worker panic just
/// means the deep tier is the first thing the client sees — no worse than before
/// the fast tier existed.
async fn publish_fast_tier(
    delivery: &DeliveryCtx<'_>,
    analysis: &Arc<AnalysisResult>,
    lift_inputs: &LiftInputs<'_>,
) {
    let fast: Vec<tcl_compiler::analyser::Diagnostic> = analysis
        .diagnostics
        .iter()
        .filter(|d| is_fast_tier(d.code))
        .cloned()
        .collect();
    let analysis_lifts = Arc::clone(analysis);
    let text = lift_inputs.text.to_owned();
    let disabled = lift_inputs.disabled.clone();
    let severity_overrides = lift_inputs.severity_overrides.clone();
    let style_line_length = lift_inputs.style_line_length;
    let lifted = tokio::task::spawn_blocking(move || {
        let mut diagnostics = lift_analyser_diagnostics(&text, &fast);
        diagnostics.extend(lift_source_style_diagnostics(
            &text,
            &analysis_lifts.suppressed_lines,
            &disabled,
            style_line_length as usize,
        ));
        apply_severity_overrides(&mut diagnostics, &severity_overrides);
        diagnostics
    })
    .await;
    if let Ok(diagnostics) = lifted {
        delivery.deliver_fast_tier_if_current(diagnostics).await;
    }
}

/// The document-style toggles + buffer the diagnostic lifts read; borrows for
/// one `run_diagnostics_core` call.
struct LiftInputs<'a> {
    text: &'a str,
    dialect: &'a str,
    disabled: &'a HashSet<String>,
    /// `tclLsp.diagnosticSeverity.<CODE>` per-code LSP severity overrides,
    /// applied as a display-side re-label once the lift completes; empty ⇒
    /// no overrides (see [`apply_severity_overrides`]).
    severity_overrides:
        &'a std::collections::HashMap<String, tower_lsp_server::ls_types::DiagnosticSeverity>,
    opt_disabled: &'a HashSet<String>,
    optimiser_enabled: bool,
    style_line_length: u32,
    xc_diagnostics: bool,
}

/// Refine the analyser's single-file W120 against the workspace package
/// database (#723), then lift the analyser / compiler / source-style / XC
/// diagnostics into LSP diagnostics on a `spawn_blocking` worker.  Returns the
/// join result so the caller distinguishes a worker panic from a clean set.
async fn refine_and_lift_diagnostics(
    analysis: &Arc<AnalysisResult>,
    analyser_diags: Vec<tcl_compiler::analyser::Diagnostic>,
    compiler_diags: &Arc<tcl_lsp_db::CompilerDiagnostics>,
    inherited_requires: &[String],
    package_resolver: &Arc<RwLock<PackageResolver>>,
    registry: &Arc<CommandRegistry>,
    inputs: &LiftInputs<'_>,
) -> Result<Vec<tower_lsp_server::ls_types::Diagnostic>, tokio::task::JoinError> {
    // #723 + #804: refine the analyser's single-file W120 against the workspace
    // package database and the requires inherited from entry points / `source`
    // ancestors (shared with the pull path via `refine_workspace_w120`).
    let analyser_diags = refine_workspace_w120(
        analyser_diags,
        analysis.as_ref(),
        inherited_requires,
        package_resolver,
        registry,
    )
    .await;
    // #832: drop any W123 (unknown command) the package database can resolve —
    // an auto-loaded library command (`tclIndex`) or a command an available
    // package's implementation defines. Always on, like the W120 refinement.
    let analyser_diags = refine_workspace_w123(
        analyser_diags,
        analysis.as_ref(),
        inherited_requires,
        package_resolver,
        inputs.dialect,
    )
    .await;

    let analysis_lifts = Arc::clone(analysis);
    let lift_text = inputs.text.to_owned();
    let disabled = inputs.disabled.clone();
    let severity_overrides = inputs.severity_overrides.clone();
    let opt_disabled = inputs.opt_disabled.clone();
    let optimiser_enabled = inputs.optimiser_enabled;
    let style_line_length = inputs.style_line_length;
    let xc_for_irules = inputs.xc_diagnostics && inputs.dialect == "f5-irules";
    let compiler_diags = Arc::clone(compiler_diags);
    tokio::task::spawn_blocking(move || {
        // `analyser_diags` is the cross-file-filtered set when `crossFileResolution`
        // is on (else identical to `analysis_lifts.diagnostics`).
        let mut diagnostics = lift_analyser_diagnostics(&lift_text, &analyser_diags);
        append_brace_expr_perf_hints(&mut diagnostics, optimiser_enabled, &opt_disabled);
        diagnostics.extend(lift_compiler_diagnostics(
            &lift_text,
            &compiler_diags,
            optimiser_enabled,
            &opt_disabled,
            &disabled,
            &analysis_lifts.suppressed_lines,
        ));
        diagnostics.extend(lift_source_style_diagnostics(
            &lift_text,
            &analysis_lifts.suppressed_lines,
            &disabled,
            style_line_length as usize,
        ));
        // Opt-in: append the XC100-301 translatability diagnostics
        // for `f5-irules` documents when `xcDiagnostics` is enabled.
        if xc_for_irules {
            diagnostics.extend(lift_xc_diagnostics(
                &lift_text,
                &disabled,
                &analysis_lifts.suppressed_lines,
            ));
        }
        apply_severity_overrides(&mut diagnostics, &severity_overrides);
        diagnostics
    })
    .await
}

/// Timing for the `[timing] workspace_state.update` log line.
struct PublishTiming<'a> {
    started: std::time::Instant,
    uri_str: &'a str,
    line_count: usize,
}

/// Publish the lifted diagnostics: re-check currency, refresh the workspace
/// index, prime the pull cache, deliver to the client, and log timing.  Always
/// settled (`true`) — a stale revision or a lift-worker panic both keep the
/// prior diagnostics rather than retrying.
async fn publish_diagnostics_result(
    delivery: &DeliveryCtx<'_>,
    workspace_index: &Arc<RwLock<core_workspace_index::WorkspaceIndex>>,
    rehomed_source_seeds: &Arc<Mutex<HashMap<String, Vec<String>>>>,
    analysis: &Arc<AnalysisResult>,
    result: Result<Vec<tower_lsp_server::ls_types::Diagnostic>, tokio::task::JoinError>,
    timing: PublishTiming<'_>,
) -> bool {
    let diags = match result {
        Ok(diags) => diags,
        Err(err) => {
            delivery
                .client
                .log_message(
                    MessageType::WARNING,
                    format!("diagnostics worker panicked: {err}"),
                )
                .await;
            return true;
        }
    };
    let diag_count = diags.len();
    {
        // Hold the `documents` lock across the currency re-check, the
        // workspace-index update, AND the pull-cache/publish delivery so a
        // concurrent `did_change`/`did_close` (which also take `documents`)
        // cannot interleave between them — the role the former global
        // `document_analysis_gate` served, now via the natural
        // `documents` → `workspace_index` and `documents` → `pull_diag_cache`
        // lock order. Delivering *inside* the lock is what stops a `did_close`
        // that ran between the currency check and the delivery from having its
        // clearing empty publish overwritten (and its pull-cache removal
        // undone) by this run's late squiggles (RUST_ISSUE_098).
        let docs = delivery.documents.lock().await;
        if !delivery.is_current(&docs).await {
            // Superseded by a newer edit (open run), a reopen, or a newer closed
            // run (generation bumped), which has taken authority for this URI —
            // settled for this version; the authoritative path publishes the
            // newer state.
            return true;
        }
        {
            let mut index = workspace_index.write().await;
            index.remove_document(delivery.uri.as_str());
            index.add_document(delivery.uri.as_str(), analysis);
        }
        // The document is now indexed standalone: invalidate its applied
        // source-site seed record (M9) so the next cross-document query
        // re-applies the seeded views.
        rehomed_source_seeds
            .lock()
            .await
            .remove(delivery.uri.as_str());
        // Keep the pull-diagnostic cache in lock-step with the push: a
        // `textDocument/diagnostic` request now returns this exact set with
        // a fresh `result_id`, and an editor that already holds it gets a
        // cheap `Unchanged` report.
        delivery.cache_and_deliver(diags).await;
    }
    let elapsed_ms = timing.started.elapsed().as_secs_f64() * 1000.0;
    // The analyser runs a single, full ("deep") pass per publish — there is
    // no separate fast/deep split.  Emit the
    // `[timing] deep diagnostics` marker anyway so tooling that waits for the
    // deep pass to finish (the VS Code test harness' `waitForDeepDiagnostics`,
    // which keys on this exact line + `uri=`) has a reliable signal that O1xx /
    // analysis-level diagnostics for this URI have been published.
    delivery
        .client
        .log_message(
            MessageType::LOG,
            format!(
                "[timing] deep diagnostics {elapsed_ms:.0}ms \
                 (uri={uri_str}, diags={diag_count})",
                uri_str = timing.uri_str,
            ),
        )
        .await;
    delivery
        .client
        .log_message(
            MessageType::LOG,
            format!(
                "[timing] workspace_state.update {elapsed_ms:.0}ms \
                 (uri={uri_str}, lines={line_count}, diags={diag_count})",
                uri_str = timing.uri_str,
                line_count = timing.line_count,
            ),
        )
        .await;
    true
}

/// LSP server backend.
///
/// Holds the LSP `Client` for outbound notifications, a document
/// store keyed on URL, and a per-dialect [`CommandRegistry`]
/// cache. Constructed once per LSP session by
/// [`tower_lsp_server::LspService::new`].
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
    documents: Arc<Mutex<HashMap<Uri, DocumentState>>>,
    /// Per-URI coalescing diagnostics scheduler state.  Each edit marks the
    /// document dirty and, if no worker is running, starts one; the single
    /// worker debounces, then repeatedly analyses the document's **latest**
    /// state until it has published the current version (retrying if a run was
    /// cancelled mid-flight).  This guarantees the final version's diagnostics
    /// are always published even under heavy edit bursts / CPU load — replacing
    /// the older per-edit generation-counter scheme, which could drop the last
    /// run with no retry.
    diag_slots: Arc<Mutex<HashMap<Uri, DiagSlot>>>,
    /// Fallback dialect string used when ``did_open`` cannot derive
    /// one from the ``languageId`` and no per-session
    /// ``workspace/didChangeConfiguration`` has been received yet.
    /// Updated by ``did_change_configuration`` so editor reconfigures
    /// take effect for subsequently-opened documents.
    default_dialect: Mutex<String>,
    /// Workspace folder roots received from `initialize` /
    /// `workspace/didChangeWorkspaceFolders`.  Stored as
    /// `Uri` (typically `file://...` directories).
    /// The folder list
    /// supports cross-document features (workspace symbols
    /// already walks every cached document).
    workspace_folders: Mutex<Vec<Uri>>,
    /// Optional per-folder dialect override map keyed on the
    /// folder URL prefix (typically `file://...`).  Populated
    /// from the `initializationOptions.folderDialects` JSON
    /// object when present.  Enables multi-folder workspaces
    /// with mixed dialects to parse correctly by selecting the
    /// longest-prefix folder's dialect when a document is
    /// opened.
    folder_dialects: Mutex<Vec<(Uri, String)>>,
    /// Per-folder editor configuration overrides (diagnostics, optimiser,
    /// formatting, feature toggles), keyed by folder URI and resolved by
    /// longest prefix at read time.  Populated by the per-folder
    /// `workspace/configuration` pull.  Empty in a single-root workspace, where
    /// every read falls back to the process-global fields below.
    folder_configs: Mutex<Vec<(Uri, FolderConfig)>>,
    /// Per-folder salsa `AnalyserConfig` input handles, present only for folders
    /// that override the disabled-diagnostics set or non-ASCII mode.  The
    /// diagnostics path resolves the handle by longest prefix so a folder's
    /// W-code suppression reaches the cached `file_analysis` query, not just the
    /// server-side lift.  Folders without such overrides fall back to
    /// [`Backend::db_config`].
    folder_db_configs: Arc<Mutex<Vec<(Uri, tcl_lsp_db::AnalyserConfig)>>>,
    /// W108 non-ASCII detection mode (`tclLsp.style.nonAscii`).
    /// [`NonAsciiMode::Default`] until an editor configures it via
    /// `initializationOptions` or `workspace/didChangeConfiguration`.
    /// Threaded into every `Analyser` the diagnostics path builds.
    non_ascii_mode: Mutex<NonAsciiMode>,
    /// Diagnostic codes the user has disabled (`tclLsp.diagnostics.<CODE>
    /// = false`). Threaded into every analyser build so the disabled
    /// codes are filtered, and consulted by the source-style pass.
    disabled_diagnostics: Mutex<HashSet<String>>,
    /// Per-code LSP severity overrides (`tclLsp.diagnosticSeverity.<CODE>`).
    /// A purely display-side re-labelling applied to the lifted diagnostics
    /// after analysis: a listed code is published at the chosen severity,
    /// leaving its range / message / code untouched. Empty ⇒ no overrides
    /// (the analyser's emitted severity stands).
    severity_overrides: Mutex<HashMap<String, tower_lsp_server::ls_types::DiagnosticSeverity>>,
    /// Cross-document proc / class definition index, maintained
    /// incrementally as documents open / change / close.  Lets
    /// completion enumerate procs from sibling files.
    /// `Arc` so the detached diagnostics task can update it off the event loop.
    workspace_index: Arc<RwLock<core_workspace_index::WorkspaceIndex>>,
    /// Tcl package database scanned from the workspace + `TCLLIBPATH`: the
    /// `pkgIndex.tcl` / `tclIndex` index used to resolve a `package require`
    /// to the files it loads (and transitively what *they* require). The
    /// diagnostics worker consults it to refine W120 — e.g. to see that a
    /// `package require myTkPackage` (transitively) pulls in Tk (#723).
    /// Rebuilt by `scan_workspace_folders`.
    package_resolver: Arc<RwLock<PackageResolver>>,
    /// Library files the autoload tier (M8) has merged into the workspace
    /// index on demand, so references / rename keep reaching them.  Cleared
    /// (and their index entries dropped) whenever the package database is
    /// rebuilt, so a `libraryPaths` change cannot leave stale library
    /// definitions behind.
    autoloaded_library_uris: Arc<Mutex<HashSet<String>>>,
    /// M9: per-document source-site namespace seeds currently merged into the
    /// index (`uri → sorted seeds`; `"::"` = the standalone view).  `source`
    /// evaluates a file in the caller's namespace, so a document sourced from
    /// `namespace eval ::x` is indexed under a `::x`-seeded analysis; this
    /// map records what is applied so [`Backend::refresh_source_rehoming`]
    /// only re-analyses on change, and so declaration-side queries can map a
    /// standalone name to its re-homed twin.
    rehomed_source_seeds: Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// Tcl installations discovered by scanning common install locations on
    /// disk (never by executing `tclsh`). Cached once per session. The package
    /// database scans these (plus configured `libraryPaths`) so it can see
    /// system-installed packages (Tk, tcllib, …), not just workspace ones.
    discovered_tcl: Arc<std::sync::OnceLock<Vec<core_tcl_install::TclInstallation>>>,
    /// Editor-provided `tclLsp.libraryPaths` (the `auto_path` the user picked /
    /// typed). Merged with the config-file layers (`config.ini [global]`,
    /// `.tcl-lsp.ini [project]`) and discovery when building the package
    /// database.
    editor_library_paths: Mutex<Vec<String>>,
    /// User-declared extra command names (`tclLsp.extraCommands`) treated as
    /// known by the unknown-command (W123) check; mirrored onto the salsa
    /// `AnalyserConfig`.
    extra_commands: Mutex<Vec<String>>,
    /// `tclLsp.bigipVersion` — the session's target BIG-IP release for the
    /// keyed library-version axis (`None` = the oldest-supported default).
    bigip_version: Mutex<Option<String>>,
    /// Generic `static::` variable-name patterns for IRULE4002
    /// (`tclLsp.diagnostics.genericVariablePatterns`). `None` keeps the built-in
    /// default set; `Some(list)` replaces it (an empty list disables the check).
    /// Mirrored onto the salsa `AnalyserConfig`.
    generic_variable_patterns: Mutex<Option<Vec<String>>>,
    /// Resolved `tclLsp.formatting` settings object (the whole section), used
    /// to build the formatter `FormatterConfig` for the document-formatting
    /// handlers. `Null`/absent keys keep the formatter defaults.
    formatting_settings: Mutex<serde_json::Value>,
    /// Per-feature provider toggles (`tclLsp.features.*`).  Absent
    /// keys default to enabled, so a config that names only some
    /// features leaves the rest on.  Consulted by each provider
    /// entry point and surfaced by `getEffectiveConfig`.
    feature_toggles: Mutex<FeatureToggles>,
    /// Optimiser master switch (`tclLsp.optimiser.enabled`).  When
    /// `false`, the `tcl-lsp.optimiseDocument` command yields no
    /// rewrites.  Default on.
    optimiser_enabled: Mutex<bool>,
    /// Shimmer-detection master switch (`tclLsp.shimmer.enabled`). When off,
    /// the Shimmer-family diagnostics (`S100`–`S110`) are suppressed. Default
    /// on.
    shimmer_enabled: Mutex<bool>,
    /// Optimisation profile (`tclLsp.optimiser.profile`) controlling which
    /// O-code categories surface as diagnostics. Default
    /// [`tcl_compiler::optimiser::profiles::DEFAULT_EDITOR_PROFILE`]
    /// (`readability`).
    optimiser_profile: Mutex<tcl_compiler::optimiser::profiles::OptimisationProfile>,
    /// Per-code optimiser overrides (`tclLsp.optimiser.<CODE>` = bool): a code
    /// mapped to `true` is force-*enabled* (removed from the profile's disabled
    /// set), `false` is force-*disabled* (added). Layered on top of the
    /// profile-derived set.
    optimiser_code_overrides: Mutex<HashMap<String, bool>>,
    /// Resolved formatter line length (`tclLsp.formatting.lineLength`).
    /// Surfaced by `getEffectiveConfig`; default 80.
    line_length: Mutex<u32>,
    /// Resolved source-style line length (`tclLsp.style.lineLength`) — the W111
    /// "line too long" threshold, distinct from the formatter width above.
    /// Default 120, matching
    /// [`tcl_lsp_core::source_style::DEFAULT_LINE_LENGTH`].
    style_line_length: Mutex<u32>,
    /// Incremental query database (salsa 0.26) — the single memoised store of
    /// derived facts that is replacing the hand-maintained caches above.  The
    /// `db` handle is the write side (set inputs); reads clone it onto a
    /// worker thread and catch `salsa::Cancelled` when a newer edit supersedes
    /// the request.
    db: Arc<Mutex<tcl_lsp_db::TclDatabase>>,
    /// Per-URI salsa `SourceFile` input handles — the input-of-record the
    /// query graph reads.  Kept current by `did_open` / `did_change`.
    db_files: Arc<Mutex<HashMap<Uri, tcl_lsp_db::SourceFile>>>,
    /// The salsa `Project` input — the workspace file set, kept in lock-step with
    /// `db_files` (re-set only when membership changes, on open/close), driving
    /// the cross-file `project_diagnostics` query.
    /// `None` until the first document is tracked.
    db_project: Arc<Mutex<Option<tcl_lsp_db::Project>>>,
    /// The salsa `AnalyserConfig` input (disabled diagnostics + non-ASCII
    /// mode); `set_*` on `workspace/didChangeConfiguration`.  Used as the
    /// fallback when no per-folder override applies to the document.
    db_config: Arc<Mutex<tcl_lsp_db::AnalyserConfig>>,
    /// Last-published diagnostics per open document, keyed by URI, for the
    /// pull-diagnostic path.  Written by the push pipeline and read by the
    /// `textDocument/diagnostic` / `workspace/diagnostic` handlers; evicted on
    /// `did_close`.
    pull_diag_cache: Arc<Mutex<HashMap<Uri, PullDiagEntry>>>,
    /// Monotonic per-URI generation for **closed**-file diagnostics runs (#865),
    /// so overlapping close / watched-change refreshes cannot let an older run
    /// publish stale diagnostics over a newer one — see [`DiagInputs::closed_diag_gen`].
    closed_diag_gen: Arc<Mutex<HashMap<Uri, u64>>>,
    /// Whether the client advertised pull-diagnostic support
    /// (`textDocument.diagnostic` client capability) at `initialize`.
    ///
    /// When `true` the worker stops *pushing* diagnostics via
    /// `publish_diagnostics` and instead keeps the pull cache current and asks
    /// the client to re-pull (`workspace/diagnostic/refresh`).  A client that
    /// supports both — `vscode-languageclient` does — otherwise routes the
    /// server's push **and** its own pull into two separate diagnostic
    /// collections and renders every diagnostic twice (#721).  Editors that
    /// only understand push (no pull capability) keep receiving the push.
    client_supports_pull_diagnostics: std::sync::atomic::AtomicBool,
    /// Per-URI cache of the last semantic-token stream we served — its
    /// `resultId` and the packed integer data.  Lets
    /// `textDocument/semanticTokens/full/delta` answer with a minimal
    /// token-aligned [`core_semantic_tokens::diff`] edit instead of resending
    /// the whole stream, the incremental behaviour rust-analyzer / clangd use.
    /// Every editor that speaks `full/delta` (VS Code, Zed, Neovim, eglot, …)
    /// benefits: a keystroke transmits a few changed tokens rather than the
    /// entire document, which keeps the client's token round-trip — and, for
    /// eglot, its stale-repaint window (issue #333) — small on large files.
    /// Keyed by URI; the entry is refreshed on every `full` / `full/delta`
    /// response and evicted on `did_close`.  `Arc` so the detached
    /// semantic-tokens background continuation (see
    /// [`Backend::semantic_tokens_core_data`]) can compare the enriched result
    /// it lands against what was last served, without borrowing `Backend`.
    last_semantic_tokens: SemanticTokensCache,
    /// Set while a debounced `workspace/semanticTokens/refresh` fire is
    /// scheduled (see [`SemanticTokensRefreshCtx::request_refresh_coalesced`]).
    /// The refresh carries no data — a client that receives it simply
    /// re-pulls current tokens for every document it has open — so any
    /// number of enriched results landing while a fire is already scheduled
    /// ride along with it; there is nothing to lose by not scheduling a
    /// second one. Without this, many cold large tabs finishing their
    /// enriched computation around the same time (e.g. on startup) would
    /// each fire their own workspace-wide refresh, and a client that does
    /// not coalesce them itself (VS Code does; eglot may not) would re-pull
    /// every open document once per refresh.
    semantic_tokens_refresh_pending: Arc<std::sync::atomic::AtomicBool>,
    /// Abort handle for the most recent [`Backend::spawn_workspace_warm`] task, so
    /// a fresh warm (from `initialize` / folder-add / config-change) supersedes any
    /// still-running one. Without it, overlapping warms each hold their own
    /// `WORKSPACE_WARM_MAX_CONCURRENCY` snapshots, so the global snapshot bound
    /// would hold only per-warm. A `std::sync::Mutex`: locked only briefly (swap +
    /// abort) from the sync `spawn_workspace_warm`, never across an await.
    warm_task: std::sync::Mutex<Option<tokio::task::AbortHandle>>,
    /// Sequences the document-sync notification handlers (`did_open` /
    /// `did_change` / `did_close`) so their buffer mutations apply in the exact
    /// order the client sent them. See [`EditOrder`].
    edit_order: EditOrder,
}

/// Applies document-sync notifications in arrival order.
///
/// `tower_lsp_server` drives incoming messages through
/// `buffer_unordered(max_concurrency)`, which *first-polls* the handler futures
/// in stream order but lets their **awaits** resume in whatever order the
/// runtime schedules. Sequencing on a `Mutex` taken as the first await is
/// therefore not enough: handlers were measured entering `did_change` as
/// 14,15,16,17 and acquiring the lock as 14,16,15,17. An incremental
/// `didChange` is a *range* edit computed against the previous version, so
/// applying one out of order splices it into text it was never computed against
/// and corrupts the buffer permanently — every later feature then reads a
/// document the client never had.
///
/// The ticket is drawn synchronously, before the handler's first `await` — the
/// last point at which arrival order is still known — and each handler then
/// waits for its turn. Request handlers draw no ticket; they only wait for the
/// edits already received ([`EditOrder::settled`]), so read concurrency is
/// unaffected.
#[derive(Debug, Default)]
struct EditOrder {
    next_ticket: std::sync::atomic::AtomicU64,
    now_serving: std::sync::atomic::AtomicU64,
    advanced: tokio::sync::Notify,
}

impl EditOrder {
    /// Draw the next ticket. **Must** be called before the handler's first
    /// `await`, while the future is still being first-polled in stream order.
    fn take_ticket(&self) -> u64 {
        self.next_ticket
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
    }

    /// Wait for `ticket`'s turn. The guard releases it on drop — including on an
    /// early return or a dropped (cancelled) handler future, so a turn is never
    /// lost.
    async fn wait_turn(&self, ticket: u64) -> EditTurn<'_> {
        loop {
            let advanced = self.advanced.notified();
            tokio::pin!(advanced);
            // Register before re-reading, so a turn granted between the check and
            // the await cannot be missed.
            advanced.as_mut().enable();
            if self.now_serving.load(std::sync::atomic::Ordering::Acquire) == ticket {
                return EditTurn { order: self };
            }
            advanced.await;
        }
    }

    /// Wait until every edit that had drawn a ticket *before this call* has been
    /// applied. Edits arriving later do not hold the caller up.
    async fn settled(&self) {
        let target = self.next_ticket.load(std::sync::atomic::Ordering::Acquire);
        loop {
            let advanced = self.advanced.notified();
            tokio::pin!(advanced);
            advanced.as_mut().enable();
            if self.now_serving.load(std::sync::atomic::Ordering::Acquire) >= target {
                return;
            }
            advanced.await;
        }
    }
}

/// A held turn in the [`EditOrder`] sequence; hands it on when dropped.
struct EditTurn<'a> {
    order: &'a EditOrder,
}

impl Drop for EditTurn<'_> {
    fn drop(&mut self) {
        self.order
            .now_serving
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        self.order.advanced.notify_waiters();
    }
}

/// Resolved `tclLsp.features.*` toggle state.
///
/// Stores only the keys an editor has explicitly set; every other
/// feature resolves to enabled.  This follows
/// "absent → default-on" semantics and the config-pull restore
/// contract (a pulled config only *sets* the keys it carries).
#[derive(Debug, Default, Clone)]
struct FeatureToggles {
    set: HashMap<String, bool>,
}

impl FeatureToggles {
    /// camelCase feature keys reported by `getEffectiveConfig`.
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
        // Inlay hints split into two independently-toggled families:
        // inferred-type hints and parameter-name hints.  Both
        // default **off** (see `DEFAULT_OFF`).  The retired `inlayHints`
        // key is accepted on input as an alias for `inlayTypeHints`
        // (see `apply`).
        "inlayTypeHints",
        "inlayParameterHints",
        "callHierarchy",
        "documentLinks",
        "selectionRange",
        "documentHighlight",
        "codeLens",
        "workspaceFileOps",
        "implementation",
        "typeDefinition",
        "declaration",
        "linkedEditingRange",
        // Opt-in XC100-301 translatability diagnostics for
        // `f5-irules` documents (default **off**).
        "xcDiagnostics",
        // Opt-in cross-file resolution: cross-file W120/W123 suppression
        // and cross-file E002/E003 arity, for *every* dialect (default
        // **off** — see `Backend::cross_file_resolution_enabled`).
        // Deliberately separate from `xcDiagnostics`, which gates only the
        // f5-irules-specific XC100-301 translatability lints — the two
        // toggles used to be one, which meant a plain Tcl project had no
        // way to opt into cross-file analysis without also opting into an
        // unrelated F5 migration feature.
        "crossFileResolution",
    ];

    /// camelCase feature keys that default **off** (opt-in) rather than
    /// on.  `getEffectiveConfig` (`resolved_map`) and `is_enabled` must
    /// report these as disabled until an editor sets them, so the
    /// reported state matches the inlay handler's default-off gate
    /// (otherwise an exported `config.ini` would claim the hints are on
    /// and re-importing it would enable them).
    const DEFAULT_OFF: &'static [&'static str] = &[
        "inlayTypeHints",
        "inlayParameterHints",
        "xcDiagnostics",
        "crossFileResolution",
    ];

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
    /// opt-in features (e.g. `willSaveWaitUntil`) that stay disabled
    /// unless an editor turns them on.
    fn is_enabled_default_off(&self, feature: &str) -> bool {
        self.set.get(feature).copied().unwrap_or(false)
    }

    /// Merge an editor-supplied `features` object, setting only the
    /// keys it carries (absent keys keep their last-applied value).
    ///
    /// The retired `inlayHints` key is a backward-compatible alias for
    /// `inlayTypeHints`: an existing explicit opt-in keeps showing
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

/// A folder's `tclLsp.diagnostics.genericVariablePatterns` override (IRULE4002).
///
/// A 3-state resolution, distinguishing "the folder said nothing" from "the
/// folder explicitly asked for the built-in defaults" from "the folder supplied
/// its own list".  Replaces a former `Option<Option<Vec<String>>>`, preserving
/// its exact semantics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum FolderGenericPatterns {
    /// The folder did not set the value (former outer `None`): inherit the
    /// process-global `genericVariablePatterns`.
    #[default]
    Inherit,
    /// The folder set the value to the built-in default patterns (former
    /// `Some(None)`): the analyser uses its built-in generic-name set.
    BuiltinDefaults,
    /// The folder supplied its own list (former `Some(Some(list))`): replace the
    /// built-in patterns with this list.
    Replace(Vec<String>),
}

/// One workspace folder's editor configuration overrides.
///
/// Each field is `None` (or empty) when the folder's pulled `tclLsp` config
/// did not set it, in which case the resolver falls back to the process-global
/// value.  In a multi-root workspace each root can
/// carry its own diagnostics, optimiser, formatting, and feature settings.
/// Per-folder *dialect* is handled separately by [`Backend::folder_dialects`].
#[derive(Clone, Default)]
struct FolderConfig {
    feature_toggles: FeatureToggles,
    disabled_diagnostics: Option<HashSet<String>>,
    /// `tclLsp.diagnosticSeverity` per-code LSP severity overrides; `None`
    /// inherits the process-global map (`Some`, possibly empty, when the
    /// folder's config sets the section in either shape).
    severity_overrides: Option<HashMap<String, tower_lsp_server::ls_types::DiagnosticSeverity>>,
    non_ascii_mode: Option<NonAsciiMode>,
    optimiser_enabled: Option<bool>,
    optimiser_profile: Option<tcl_compiler::optimiser::profiles::OptimisationProfile>,
    optimiser_code_overrides: HashMap<String, bool>,
    line_length: Option<u32>,
    /// `tclLsp.formatting` section override for the formatter; `None` inherits
    /// the global formatting settings.
    formatting: Option<serde_json::Value>,
    /// `tclLsp.style.lineLength` override (W111 threshold); `None` inherits the
    /// global value.
    style_line_length: Option<u32>,
    /// `tclLsp.shimmer.enabled` override; `None` inherits the global value.
    shimmer_enabled: Option<bool>,
    /// `tclLsp.extraCommands` override; `None` inherits the global set.
    extra_commands: Option<Vec<String>>,
    /// `tclLsp.diagnostics.genericVariablePatterns` override. See
    /// [`FolderGenericPatterns`]: `Inherit` falls back to the global value,
    /// `BuiltinDefaults` selects the analyser's built-in set, and `Replace`
    /// supplies the folder's own list.
    generic_variable_patterns: FolderGenericPatterns,
    /// `tclLsp.libraryPaths` override; `None` inherits the global set.
    library_paths: Option<Vec<String>>,
    /// `.tcl-lsp.ini [project] entryPoints` — the project's "main" files (paths
    /// relative to the folder root). When set, the W120 workspace refinement
    /// treats these entries' `package require`s as available across the folder
    /// and disables the automatic `source`-graph inheritance. `None` / empty
    /// leaves auto-detection on.
    entry_points: Option<Vec<String>>,
}

/// Pick the value associated with the **longest** folder URI that `uri` sits
/// under.
fn longest_folder_match<'a, T>(entries: &'a [(Uri, T)], uri: &Uri) -> Option<&'a T> {
    let mut best: Option<&'a (Uri, T)> = None;
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

/// The rename call-site facts
/// [`Backend::extend_rename_with_cross_document_edits`] needs, bundled to
/// stay within clippy's argument-count budget.
struct RenameContext<'a> {
    uri: &'a Uri,
    source: &'a str,
    analysis: &'a AnalysisResult,
    pos: Position,
    new_name: &'a str,
    registry: &'a CommandRegistry,
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
        let db_config = tcl_lsp_db::AnalyserConfig::new(
            &db,
            default_disabled_set().into_iter().collect(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
        );
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
            disabled_diagnostics: Mutex::new(default_disabled_set()),
            severity_overrides: Mutex::new(HashMap::new()),
            workspace_index: Arc::new(RwLock::new(core_workspace_index::WorkspaceIndex::new())),
            package_resolver: Arc::new(RwLock::new(PackageResolver::new())),
            autoloaded_library_uris: Arc::new(Mutex::new(HashSet::new())),
            rehomed_source_seeds: Arc::new(Mutex::new(HashMap::new())),
            discovered_tcl: Arc::new(std::sync::OnceLock::new()),
            editor_library_paths: Mutex::new(Vec::new()),
            extra_commands: Mutex::new(Vec::new()),
            bigip_version: Mutex::new(None),
            generic_variable_patterns: Mutex::new(None),
            formatting_settings: Mutex::new(serde_json::Value::Null),
            feature_toggles: Mutex::new(FeatureToggles::default()),
            optimiser_enabled: Mutex::new(true),
            shimmer_enabled: Mutex::new(true),
            optimiser_profile: Mutex::new(default_optimiser_profile()),
            optimiser_code_overrides: Mutex::new(HashMap::new()),
            line_length: Mutex::new(80),
            style_line_length: Mutex::new(120),
            db: Arc::new(Mutex::new(db)),
            db_files: Arc::new(Mutex::new(HashMap::new())),
            db_project: Arc::new(Mutex::new(None)),
            db_config: Arc::new(Mutex::new(db_config)),
            pull_diag_cache: Arc::new(Mutex::new(HashMap::new())),
            closed_diag_gen: Arc::new(Mutex::new(HashMap::new())),
            client_supports_pull_diagnostics: std::sync::atomic::AtomicBool::new(false),
            last_semantic_tokens: Arc::new(Mutex::new(HashMap::new())),
            semantic_tokens_refresh_pending: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            warm_task: std::sync::Mutex::new(None),
            edit_order: EditOrder::default(),
        }
    }

    /// Create or update the salsa `SourceFile` input for `uri`.  Called by
    /// `did_open` / `did_change` so the query graph always reads current text.
    /// Lock order is always `db` then `db_files`.
    async fn db_set_source(&self, uri: &Uri, text: String, dialect: String) {
        use salsa::Setter as _;
        let mut db = self.db.lock().await;
        let mut files = self.db_files.lock().await;
        if let Some(&file) = files.get(uri) {
            // Text/dialect edit: membership unchanged, so the `Project` input is
            // untouched (a body edit then backdates `project_proc_names`).
            file.set_text(&mut *db).to(text);
            file.set_dialect(&mut *db).to(dialect);
        } else {
            let path = uri.to_file_path().map(|p| p.display().to_string());
            let file = tcl_lsp_db::SourceFile::new(&*db, text, dialect, path);
            files.insert(uri.clone(), file);
            // Membership changed — re-set the `Project` file set.
            let mut project = self.db_project.lock().await;
            Self::sync_db_project(&mut db, &files, &mut project);
        }
    }

    /// Drop the salsa `SourceFile` input for `uri` (on `did_close`).  Lock order
    /// is `db` → `db_files` → `db_project`, matching [`Self::db_set_source`].
    async fn db_remove_source(&self, uri: &Uri) {
        let mut db = self.db.lock().await;
        let mut files = self.db_files.lock().await;
        if files.remove(uri).is_some() {
            let mut project = self.db_project.lock().await;
            Self::sync_db_project(&mut db, &files, &mut project);
        }
    }

    /// Add/update many salsa `SourceFile` inputs at once (disk-backed workspace
    /// files from the startup scan), re-setting the [`tcl_lsp_db::Project`] **once**
    /// if membership changed — not once per file (which would be O(files²) over a
    /// large tree).  Used so cross-file diagnostics resolve against the *whole*
    /// workspace (matching `workspace_index`), not only open documents.  Lock order
    /// is `db` → `db_files` → `db_project`, as everywhere.
    async fn db_set_sources_batch(&self, entries: &[(Uri, String, String)]) {
        use salsa::Setter as _;
        if entries.is_empty() {
            return;
        }
        let mut db = self.db.lock().await;
        let mut files = self.db_files.lock().await;
        let mut membership_changed = false;
        for (uri, text, dialect) in entries {
            if let Some(&file) = files.get(uri) {
                file.set_text(&mut *db).to(text.clone());
                file.set_dialect(&mut *db).to(dialect.clone());
            } else {
                let path = uri.to_file_path().map(|p| p.display().to_string());
                let file = tcl_lsp_db::SourceFile::new(&*db, text.clone(), dialect.clone(), path);
                files.insert(uri.clone(), file);
                membership_changed = true;
            }
        }
        if membership_changed {
            let mut project = self.db_project.lock().await;
            Self::sync_db_project(&mut db, &files, &mut project);
        }
    }

    /// Drop many salsa `SourceFile` inputs at once (files under a removed workspace
    /// folder), re-setting the `Project` once if membership changed.  Lock order is
    /// `db` → `db_files` → `db_project`.
    async fn db_remove_sources_batch(&self, uris: &[Uri]) {
        if uris.is_empty() {
            return;
        }
        let mut db = self.db.lock().await;
        let mut files = self.db_files.lock().await;
        let mut membership_changed = false;
        for uri in uris {
            if files.remove(uri).is_some() {
                membership_changed = true;
            }
        }
        if membership_changed {
            let mut project = self.db_project.lock().await;
            Self::sync_db_project(&mut db, &files, &mut project);
        }
    }

    /// Re-set the salsa [`tcl_lsp_db::Project`] input to the current `db_files`
    /// set (sorted by URI for a stable, iteration-order-independent `Vec` so the
    /// input only changes when membership does — not on `HashMap` reshuffles).
    /// Called when membership changes (open/close/scan), so a text edit never
    /// re-derives the project aggregates.  The `Project` handle is stable across
    /// re-sets, so workers holding it keep reading the current value.
    fn sync_db_project(
        db: &mut tcl_lsp_db::TclDatabase,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
        project: &mut Option<tcl_lsp_db::Project>,
    ) {
        use salsa::Setter as _;
        let mut entries: Vec<(&Uri, &tcl_lsp_db::SourceFile)> = files.iter().collect();
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
    /// fell back to the session default change.
    async fn reresolve_open_document_dialects(&self) {
        // Snapshot `(uri, language_id, current dialect)` first — the async
        // `dialect_for_open` calls lock `folder_dialects` / `default_dialect`,
        // so they must not run while the `documents` lock is held.
        let snapshot: Vec<(Uri, String, String, String)> = {
            let docs = self.documents.lock().await;
            docs.iter()
                .map(|(uri, doc)| {
                    (
                        uri.clone(),
                        doc.language_id.clone(),
                        doc.dialect.clone(),
                        doc.text.clone(),
                    )
                })
                .collect()
        };
        for (uri, language_id, old_dialect, text) in snapshot {
            let new_dialect = self.dialect_for_open(&uri, &language_id, &text).await;
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
                    .write()
                    .await
                    .remove_document(uri.as_str());
            }
            self.reschedule_diagnostics(uri, new_dialect).await;
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
        let extra = self.extra_commands.lock().await.clone();
        let generic = self.generic_variable_patterns.lock().await.clone();
        let config = *self.db_config.lock().await;
        let mut db = self.db.lock().await;
        config.set_disabled_diagnostics(&mut *db).to(disabled);
        config.set_non_ascii_mode(&mut *db).to(mode);
        config.set_extra_commands(&mut *db).to(extra);
        config.set_generic_variable_patterns(&mut *db).to(generic);
        let bigip = self.bigip_version.lock().await.clone();
        config.set_bigip_version(&mut *db).to(bigip);
    }

    /// Run the salsa `document_symbols` query for `uri` on a worker thread,
    /// reading the current `SourceFile` input.  Returns `None` when the input
    /// is absent or a concurrent edit cancels the read, so the caller can fall
    /// back to a direct computation (behaviour preserved).
    async fn db_document_symbols(
        &self,
        uri: &Uri,
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

    /// Resolve the salsa `AnalyserConfig` handle for `uri`, mirroring
    /// [`DiagInputs::capture_job`]: a folder that overrides the disabled-codes
    /// set / non-ASCII mode (and so has its own handle in `folder_db_configs`)
    /// wins by longest-prefix match; otherwise the process-global
    /// [`Self::db_config`].  Using the *same* handle the diagnostics path uses
    /// keeps the on-demand feature analyses (hover / definition / references /
    /// completion / code-actions) consistent with the published squiggles in a
    /// multi-root workspace.
    async fn resolved_db_config(&self, uri: &Uri) -> tcl_lsp_db::AnalyserConfig {
        let folder = self.folder_db_configs.lock().await;
        match longest_folder_match(&folder, uri) {
            Some(cfg) => *cfg,
            None => *self.db_config.lock().await,
        }
    }

    /// Run the salsa `file_analysis_incremental` query for `uri` on a worker
    /// thread, reading the current `SourceFile` input.  Returns `None` when the
    /// input is absent or a concurrent edit cancels the read.  The returned
    /// `Arc` shares the memoised analysis (no deep clone of `AnalysisResult`).
    ///
    /// Uses the incremental, per-item-memoised, cancellable query (not the
    /// coarse `file_analysis`) so every caller of this general-purpose
    /// accessor — hover, completion, `semantic_tokens_range`'s fallback, and
    /// anything else routed through [`Backend::cached_analysis`] —
    /// shares the diagnostics worker's already-computed analysis for this
    /// revision instead of paying for an independent whole-file walk
    /// (issue #829).
    async fn db_file_analysis(
        &self,
        uri: &Uri,
    ) -> Option<Arc<tcl_compiler::analyser::AnalysisResult>> {
        // Await the `JoinHandle` variant so the spawn-and-catch lives in one
        // place; callers that need to race it against a budget take the handle
        // directly (see [`db_file_analysis_handle`]).
        self.db_file_analysis_handle(uri)
            .await?
            .await
            .ok()
            .flatten()
    }

    /// [`db_file_analysis`], but returns the worker `JoinHandle` immediately
    /// instead of awaiting it — so `semantic_tokens_range` can race the read
    /// against the fast-path budget and, on timeout, keep awaiting it from a
    /// detached convergence continuation (#844 Gap 4) rather than dropping it and
    /// losing the enriched result.  Same cancellable per-item query and the same
    /// liveness invariant as [`db_semantic_tokens`] (the read is unwound at a
    /// per-item boundary by a concurrent `set_text`).  `None` when there is no
    /// salsa input for `uri`.
    async fn db_file_analysis_handle(
        &self,
        uri: &Uri,
    ) -> Option<tokio::task::JoinHandle<Option<Arc<tcl_compiler::analyser::AnalysisResult>>>> {
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let config = self.resolved_db_config(uri).await;
        let snapshot = self.db.lock().await.clone();
        Some(tokio::task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| {
                tcl_lsp_db::file_analysis_incremental(&snapshot, file, config)
            })
            .ok()
        }))
    }

    /// [`db_compilation_unit`], but returns the worker `JoinHandle` immediately
    /// (see [`db_file_analysis_handle`] for why #844 Gap 4 needs it).  `None`
    /// when there is no salsa input for `uri`.
    async fn db_compilation_unit_handle(
        &self,
        uri: &Uri,
    ) -> Option<tokio::task::JoinHandle<Option<Arc<tcl_compiler::compilation_unit::CompilationUnit>>>>
    {
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let snapshot = self.db.lock().await.clone();
        Some(tokio::task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| tcl_lsp_db::document_compilation_unit(&snapshot, file)).ok()
        }))
    }

    /// Kick off the salsa `semantic_tokens` query for `uri` on a worker
    /// thread, returning its `JoinHandle` immediately rather than awaiting it.
    /// Lets a caller race the computation against a deadline (see
    /// [`Backend::semantic_tokens_core_data`]) without losing the ability to
    /// keep waiting on it — the task keeps running (and its result stays
    /// salsa-memoised) whether or not the caller ever polls the handle again.
    /// `None` when there is no salsa input for `uri` (nothing to kick off);
    /// the handle itself resolves to `None` if a concurrent edit cancels the
    /// read.
    ///
    /// **Invariant this task's liveness depends on:** when
    /// `semantic_tokens_core_data` loses the fast-path race, this handle is
    /// awaited from a *detached* `tokio::spawn` task that can run for
    /// seconds on a large cold file, holding an active salsa read-handle the
    /// whole time. Salsa 0.27 blocks a concurrent `set_text` until every
    /// active read-handle is released, and `set_text` runs under the global
    /// `db` mutex — so a stalled read here would stall the write, and the
    /// write holds the lock every other read waits on too. This only stays
    /// live because `semantic_tokens` calls the cancellable, per-item
    /// [`tcl_lsp_db::file_analysis_incremental`] (checked at each proc/method
    /// body boundary), not the coarse whole-file
    /// [`tcl_lsp_db::file_analysis`]: an edit's `set_text` flips the cancel
    /// flag, this detached read unwinds at its next item boundary, and the
    /// write proceeds promptly. Routing this query back through the coarse
    /// `file_analysis` would reintroduce a worse version of #829's symptom —
    /// a single cold background token computation could serialise every
    /// subsequent keystroke behind a whole-file walk. See
    /// `docs/design/rust/lsp-performance.md` §7.
    async fn db_semantic_tokens(
        &self,
        uri: &Uri,
    ) -> Option<tokio::task::JoinHandle<Option<tcl_lsp_core::semantic_tokens::SemanticTokens>>>
    {
        let file = (*self.db_files.lock().await).get(uri).copied()?;
        let config = self.resolved_db_config(uri).await;
        // When the workspace has been indexed into a `Project`, resolve object
        // dispatches against the *cross-file* class index so a `$obj method` on
        // a class defined in another file highlights; otherwise fall back to the
        // local (single-file) hierarchy.
        let project = *self.db_project.lock().await;
        let snapshot = self.db.lock().await.clone();
        Some(tokio::task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| match project {
                Some(project) => {
                    tcl_lsp_db::semantic_tokens_project(&snapshot, file, config, project)
                }
                None => tcl_lsp_db::semantic_tokens(&snapshot, file, config),
            })
            .ok()
        }))
    }

    /// Read the memoised `AnalysisResult` for `uri` from the query database, if
    /// the document has a `SourceFile` input.  Returns the shared `Arc` so the
    /// caller can move a refcounted handle into a `spawn_blocking` worker (and
    /// deref it there) rather than paying an O(file) deep copy per feature
    /// request.  `None` when there is no input (the caller analyses fresh) or a
    /// concurrent edit cancelled the read.
    async fn cached_analysis(
        &self,
        uri: &Uri,
    ) -> Option<Arc<tcl_compiler::analyser::AnalysisResult>> {
        self.db_file_analysis(uri).await
    }

    /// Resolve an analysis for the document.  Consults the
    /// cache first; computes a fresh analysis when no entry
    /// exists.  Returns a shared `Arc` the caller can move into
    /// a `spawn_blocking` worker (providers deref it via `&`).
    async fn analysis_for(
        &self,
        uri: &Uri,
        text: String,
        dialect: String,
    ) -> Arc<tcl_compiler::analyser::AnalysisResult> {
        if let Some(cached) = self.cached_analysis(uri).await {
            return cached;
        }
        // No salsa input for this document (e.g. an unindexed buffer): analyse
        // fresh with the same per-folder config the cached path would have used,
        // so the on-demand result still honours folder-scoped suppression.
        let config = self.resolved_db_config(uri).await;
        let (disabled, na_mode, extra) = {
            let db = self.db.lock().await;
            (
                config.disabled_diagnostics(&*db).iter().cloned().collect(),
                config.non_ascii_mode(&*db),
                config.extra_commands(&*db).iter().cloned().collect(),
            )
        };
        tokio::task::spawn_blocking(move || {
            let mut analyser = Self::configured_analyser(disabled, na_mode, extra);
            Arc::new(analyser.analyse(&text, &dialect).clone())
        })
        .await
        .unwrap_or_default()
    }

    /// Return a snapshot of the current workspace folder
    /// URLs.  Used by cross-document features (workspace
    /// symbols, cross-doc references / rename / call-
    /// hierarchy).
    pub async fn workspace_folder_urls(&self) -> Vec<Uri> {
        self.workspace_folders.lock().await.clone()
    }

    /// Copy the workspace folders the
    /// editor sent into `self.workspace_folders` so cross-
    /// document features can resolve relative paths.
    /// Both the newer `workspace_folders` field and the
    /// single-root `root_uri` fallback are supported.
    #[allow(deprecated)] // `root_uri` is the documented single-root fallback when a client sends no `workspace_folders`.
    async fn apply_workspace_folders(&self, params: &InitializeParams) {
        if let Some(folders) = &params.workspace_folders {
            let urls: Vec<Uri> = folders.iter().map(|f| f.uri.clone()).collect();
            *self.workspace_folders.lock().await = urls;
        } else if let Some(root) = &params.root_uri {
            *self.workspace_folders.lock().await = vec![root.clone()];
        }
    }

    /// Read per-folder dialect
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
            let mut parsed: Vec<(Uri, String)> = Vec::new();
            for (folder_url, dialect_val) in entries {
                let Ok(url) = Uri::from_str(folder_url) else {
                    continue;
                };
                let Some(dialect) = dialect_val.as_str() else {
                    continue;
                };
                // Valid names: any catalog profile (now including the
                // config-only f5-tmsh / f5-bigip / bpf, which the old
                // DialectSet::parse check wrongly rejected) plus `tk`
                // (a parseable library shell, not a profile — §7.2).
                if tcl_dialect::DialectProfile::find(dialect).is_none()
                    && DialectSet::parse(dialect).is_none()
                {
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
        if let Some(overrides) = settings_severity_overrides(opts) {
            *self.severity_overrides.lock().await = overrides;
        }
    }

    /// Resolve the dialect string a freshly opened document should
    /// be tagged with.
    ///
    /// Resolution order:
    ///
    /// 1. The LSP ``languageId`` field — when it names a known
    ///    dialect (``"tcl-irule"`` / ``"f5-irules"`` / ``"tcl9.0"``
    ///    / etc.), use it directly.
    /// 2. The per-folder override map (`folder_dialects`) — when
    ///    the document URI sits under one of the configured folder
    ///    URLs, use the deepest-matching folder's dialect.
    /// 3. The session-wide ``default_dialect`` fallback.
    async fn dialect_for_open(&self, uri: &Uri, language_id: &str, text: &str) -> String {
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
        // …) routes to ``f5-bigip`` ahead of a *generic* Tcl ``languageId``:
        // only an explicit
        // non-Tcl dialect id (``f5-irules`` / ``f5-iapps`` / ``expect`` /
        // EDA / ``tk``) wins over the basename. This is what lets the test
        // harness open ``bigip.conf`` with ``languageId: "tcl"`` and still
        // get the BIG-IP path (parse / outline + general-Tcl-diagnostic
        // suppression).
        let explicit_non_tcl = matches!(lang_dialect, Some(d) if !d.starts_with("tcl"));
        if !explicit_non_tcl && core_bigip::is_bigip_conf_name(uri.as_str()) {
            return "f5-bigip".to_owned();
        }
        // In-source dialect hints — a `# tcl-dialect:` directive, a
        // `#!…tclshX.Y` shebang, or a `package require Tcl X.Y` line — are
        // authoritative for a *generically* opened Tcl buffer.  VS Code always
        // sends `languageId: "tcl"` for `.tcl` files (both `"tcl"` and the
        // explicit `"tcl8.6"` id map to `tcl8.6` above), so an in-file
        // directive is the only way a user can pin 8.4 / 8.5 / 9.x, and it must
        // win over the generic `tcl8.6` default — otherwise completion offers
        // 8.6+-only commands like `try` in an 8.4/8.5 file.  An explicit
        // versioned or non-Tcl `languageId` still takes precedence (it is a
        // deliberate editor choice), so only the bare `"tcl"` id defers here.
        // Delegates to `detect_dialect_from_source`
        // (directive > shebang > `package require Tcl`).
        if language_id == "tcl"
            && let Some(d) = tcl_registry::detect_dialect_from_source(text)
        {
            return d.to_owned();
        }
        // A *versioned* or non-Tcl language id (`tcl8.4`, `tcl9.0`,
        // `f5-irules`, …) is a deliberate, specific choice and wins over the
        // per-folder and session (config-file) dialect below. The bare `"tcl"`
        // id is different: editors send it for *every* `.tcl` file, so it names
        // no specific version and must NOT be treated as authoritative — it has
        // to defer to the folder override and the session `default_dialect`
        // (which the `dialect =` key of `config.ini` / `.tcl-lsp.ini` sets).
        // Without this deferral a config-file dialect would never take effect
        // for a normally-opened `.tcl` buffer (issue #805); the session default
        // is itself `tcl8.6` unless configured, so an unconfigured file still
        // resolves exactly as the old direct `"tcl"` → `tcl8.6` mapping did.
        if language_id != "tcl"
            && let Some(d) = lang_dialect
        {
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
    async fn resolve_folder_dialect(&self, uri: &Uri) -> Option<String> {
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
            "tcl9.1" => "tcl9.1",
            "tcl-irule" | "f5-irules" => "f5-irules",
            // `tcl-apl` is the APL (iApp presentation language) editor id — an
            // iApp sublanguage, so it analyses as `f5-iapps` rather than
            // falling through to the default Tcl dialect.
            "tcl-iapp" | "f5-iapps" | "tcl-apl" => "f5-iapps",
            // First-class since Milestone 6 (D8/D7): tmsh scripts and the
            // bpf framework dialect analyse under their own profiles.
            "tcl-tmsh" | "f5-tmsh" => "f5-tmsh",
            "tcl-bpf" | "bpf" => "bpf",
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

    /// Barrier: block until every document-sync notification the client sent
    /// *before* the calling request has been applied to the document store.
    ///
    /// `tower-lsp-server` drives requests and notifications through one
    /// `buffer_unordered` pool (`transport.rs`), so without this a request
    /// handler can run to completion while an earlier `didChange` is still in
    /// flight and answer from a buffer several edits stale — semantic tokens
    /// whose lines/lengths describe text the client has already replaced, and
    /// positions (hover, completion, definition) resolved against the wrong
    /// offsets. LSP requires a request to observe every notification that
    /// preceded it.
    ///
    /// Waits only for the edits that had already arrived ([`EditOrder::settled`]),
    /// so a later edit does not hold this reader up and readers never serialise
    /// against one another.
    async fn edits_settled(&self) {
        self.edit_order.settled().await;
    }

    async fn read_document(&self, url: &Uri) -> Option<DocumentState> {
        self.edits_settled().await;
        if let Some(doc) = self.documents.lock().await.get(url).cloned() {
            return Some(doc);
        }
        // On-disk fallback: files the folder scan indexed but the
        // editor hasn't opened aren't in the open-document map.
        // Read them from disk so cross-document span→range
        // resolution (references / rename / call-hierarchy) can
        // reach their sources.  Non-`file://` URLs and unreadable
        // paths fall through to `None`.  `to_file_path` yields a borrowed
        // `Cow<Path>`; take ownership so the path can move into the blocking task.
        let path = url.to_file_path()?.into_owned();
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

    /// Cross-file super/subtype targets for the type-hierarchy walk, resolved
    /// against the workspace class index.  Each entry is
    /// `(document-uri, qualified-name, name-span)` for a class defined in some
    /// (possibly other) document.
    ///
    /// For **subtypes**, the index resolves every class declaring `class_name`
    /// as a direct owner-aware superclass/mixin.  For **supertypes**, the
    /// written super/mixin names are read from the definition matching
    /// `selected_uri` (the class under the cursor), so a homonymous `::C` in
    /// an unrelated file can't contribute its own parents and the result is
    /// deterministic; the union over every homonym is used only as a fallback
    /// when the selected document isn't indexed.
    async fn cross_hierarchy_targets(
        &self,
        subtypes: bool,
        class_name: &str,
        selected_uri: &str,
    ) -> Vec<(String, String, tcl_lexer::Span)> {
        let index = self.workspace_index.read().await;
        let list = if subtypes {
            index.subclasses_of(class_name)
        } else {
            let selected: Vec<&core_workspace_index::WorkspaceClass> = index
                .classes_named(class_name)
                .into_iter()
                .filter(|c| c.uri == selected_uri)
                .collect();
            let definitions = if selected.is_empty() {
                index.classes_named(class_name)
            } else {
                selected
            };
            let mut acc: Vec<&core_workspace_index::WorkspaceClass> = Vec::new();
            let mut seen_super: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for c in definitions {
                for s in index.supertype_classes(c) {
                    if seen_super.insert(s.qualified_name.clone()) {
                        acc.push(s);
                    }
                }
            }
            acc
        };
        list.into_iter()
            .map(|c| (c.uri.clone(), c.qualified_name.clone(), c.name_span))
            .collect()
    }

    /// Shared supertype (`subtypes = false`) / subtype (`subtypes = true`)
    /// walk for one type-hierarchy item, resolving against the item's
    /// document.  Returns an empty list (not an error) when the document or
    /// class cannot be resolved, so the editor's hierarchy view degrades
    /// cleanly.
    async fn type_hierarchy_walk(
        &self,
        item: TypeHierarchyItem,
        subtypes: bool,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
        let uri = item.uri.clone();
        let class_name = item.name.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(Some(Vec::new()));
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let class_name_for_blk = class_name.clone();
        let items = tokio::task::spawn_blocking(move || {
            if subtypes {
                core_type_hierarchy::subtypes(&class_name_for_blk, &doc.text, &analysis)
            } else {
                core_type_hierarchy::supertypes(&class_name_for_blk, &doc.text, &analysis)
            }
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("type_hierarchy worker panicked: {err}").into(),
            data: None,
        })?;
        let mut lifted: Vec<TypeHierarchyItem> = items
            .into_iter()
            .map(|i| TypeHierarchyItem {
                name: i.name,
                kind: SymbolKind::CLASS,
                tags: None,
                detail: i.detail,
                uri: uri.clone(),
                range: lift_lsp_range(i.range),
                selection_range: lift_lsp_range(i.selection_range),
                data: None,
            })
            .collect();

        // Cross-file: resolve super/subtypes defined in *other* workspace
        // documents via the class index, converting each target's name span
        // in its own source.  De-duplicated against the same-document
        // results by qualified name.
        let mut seen: std::collections::HashSet<String> =
            lifted.iter().map(|i| i.name.clone()).collect();
        seen.insert(class_name.clone());
        let targets = self
            .cross_hierarchy_targets(subtypes, &class_name, uri.as_str())
            .await;
        for (turi, qname, name_span) in targets {
            if !seen.insert(qname.clone()) {
                continue;
            }
            let Ok(target_uri) = turi.parse::<Uri>() else {
                continue;
            };
            let Some(tdoc) = self.read_document(&target_uri).await else {
                continue;
            };
            let li = tcl_lexer::LineIndex::new_lsp(&tdoc.text);
            let start = li.position_at_utf16(name_span.start(), &tdoc.text);
            let end = li.position_at_utf16(name_span.end(), &tdoc.text);
            let range = Range {
                start: Position {
                    line: start.line,
                    character: start.character.get(),
                },
                end: Position {
                    line: end.line,
                    character: end.character.get(),
                },
            };
            lifted.push(TypeHierarchyItem {
                name: qname,
                kind: SymbolKind::CLASS,
                tags: None,
                detail: None,
                uri: target_uri,
                range,
                selection_range: range,
                data: None,
            });
        }
        Ok(Some(lifted))
    }

    /// Compute the packed semantic-token stream for `uri`, prioritising a
    /// prompt response over waiting for the fully enriched result (issue
    /// #829).
    ///
    /// Races the memoised, SSA/SCCP-enriched `semantic_tokens` salsa query
    /// against [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`]. When it lands in time —
    /// the common case, since the diagnostics worker has usually already
    /// primed the shared per-item analysis for this revision — it is returned
    /// directly, identical to a bare `db_semantic_tokens` call. When it does
    /// not (a cold or very large file whose whole-file analysis is still
    /// running, or a concurrent edit that cancelled the read), this returns
    /// the cheap segmenter+registry-only tier
    /// (`core_semantic_tokens::full`, no `CompilationUnit`/analysis) — the
    /// bulk of the highlighting, immediately — while the enriched computation
    /// keeps running in the background: it is salsa-memoised and shared with
    /// the diagnostics worker regardless of who consumes it, so nothing is
    /// wasted, and a `workspace/semanticTokens/refresh` follows once it lands
    /// and actually differs from what was served, so the enrichment (regex
    /// -source retagging, user-class object-method resolution) reaches the
    /// editor without waiting for the next edit.
    ///
    /// When a `Project` is indexed, the background computation is
    /// `semantic_tokens_project`, whose cross-file class/proc-role indices
    /// (`project_class_index` / `project_proc_var_index`) analyse every
    /// project file, not just this one — on a large workspace this can take
    /// far longer than a single file's own analysis. That cost has always
    /// existed; what this fast path changes is that it can no longer block a
    /// token response — it only delays how soon the *refresh* follows.
    /// Viewport tokens for a document that is **not Tcl** — an APL presentation
    /// or a BIG-IP config — or `None` when the document *is* Tcl and belongs on
    /// the normal pipeline.
    ///
    /// Each carries *embedded* Tcl — a `ltm rule { … }` body, an APL `[ … ]`
    /// bracket expression — so each takes the registry for the dialect that code
    /// is written in.  Fetched lazily, so an ordinary Tcl document pays for
    /// neither.
    async fn non_tcl_range_tokens(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        range: CoreLspRange,
    ) -> Option<core_semantic_tokens::SemanticTokens> {
        if is_apl_source(uri, &doc.language_id) {
            let registry = self.registry_for_dialect(IAPPS_DIALECT).await;
            return Some(core_semantic_tokens::apl_range(&doc.text, range, &registry));
        }
        if Self::is_bigip_dialect(&doc.dialect) {
            let registry = self.registry_for_dialect(IRULES_DIALECT).await;
            return Some(core_semantic_tokens::bigip_conf_range(
                &doc.text, range, &registry,
            ));
        }
        None
    }

    async fn semantic_tokens_core_data(
        &self,
        uri: &Uri,
        doc: &DocumentState,
    ) -> jsonrpc::Result<Vec<u32>> {
        // APL (iApp presentation) and BIG-IP config are not Tcl — each has its
        // own declarative grammar and its own token set, so both bypass the Tcl
        // pipeline (segmenter / compilation unit / analyser) entirely.  Without
        // these branches the Tcl tokenizer reads each braced block as one
        // literal word and emits whole *lines* as `string` tokens, which
        // mis-colours the file rather than merely under-colouring it.  Both
        // lexers are cheap and pure — line-oriented — so neither needs salsa
        // memoisation.
        if is_apl_source(uri, &doc.language_id) {
            // The iApps registry: an APL `[ … ]` bracket expression is iApp Tcl.
            let registry = self.registry_for_dialect(IAPPS_DIALECT).await;
            return Ok(core_semantic_tokens::apl_full(&doc.text, &registry).data);
        }
        if Self::is_bigip_dialect(&doc.dialect) {
            // The iRules registry, not the BIG-IP one: a `ltm rule { … }` body is
            // iRules code embedded in the config, and is walked as such.
            let registry = self.registry_for_dialect(IRULES_DIALECT).await;
            return Ok(core_semantic_tokens::bigip_conf_full(&doc.text, &registry).data);
        }

        let Some(mut enriched) = self.db_semantic_tokens(uri).await else {
            let registry = self.registry_for_dialect(&doc.dialect).await;
            let (text, dialect) = (doc.text.clone(), doc.dialect.clone());
            // No salsa input for this document (unindexed buffer): build the
            // unit and analysis fresh so regex-source highlighting and
            // user-class object-method resolution still apply, matching the
            // salsa path below.
            return tokio::task::spawn_blocking(move || {
                let cu = tcl_compiler::compilation_unit::CompilationUnit::build_for(
                    &text, &registry, false,
                );
                let analysis = tcl_compiler::analyser::Analyser::new().analyse(&text, &dialect);
                core_semantic_tokens::full_with_cu_and_analysis(
                    &text,
                    &dialect,
                    &registry,
                    Some(&cu),
                    Some(&analysis),
                )
                .data
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("semantic_tokens worker panicked: {err}").into(),
                data: None,
            });
        };

        tokio::select! {
            biased;
            result = &mut enriched => {
                if let Ok(Some(tokens)) = result {
                    return Ok(tokens.data);
                }
                // A genuine salsa cancellation (a concurrent edit landed) or a
                // worker panic — either way, don't retry inline or surface an
                // error: fall through to the cheap coarse tier below so the
                // caller still gets a prompt result. The edit that cancelled
                // this read has already scheduled its own diagnostics run,
                // which will prime a fresh enriched result for the next
                // request.
            }
            () = tokio::time::sleep(SEMANTIC_TOKENS_FAST_PATH_BUDGET) => {
                // Too slow for a synchronous first paint. Detach a
                // continuation that keeps waiting on the still-running
                // enriched computation and asks the client to re-request once
                // it lands, instead of blocking this response on it.
                let refresh_ctx = SemanticTokensRefreshCtx {
                    client: self.client.clone(),
                    last_semantic_tokens: Arc::clone(&self.last_semantic_tokens),
                    refresh_pending: Arc::clone(&self.semantic_tokens_refresh_pending),
                };
                let uri = uri.clone();
                tokio::spawn(async move {
                    if let Ok(Some(tokens)) = enriched.await {
                        refresh_ctx.deliver_if_changed(&uri, &tokens.data).await;
                    }
                });
            }
        }

        let registry = self.registry_for_dialect(&doc.dialect).await;
        let (text, dialect) = (doc.text.clone(), doc.dialect.clone());
        tokio::task::spawn_blocking(move || {
            core_semantic_tokens::full(&text, &dialect, &registry).data
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("semantic_tokens worker panicked: {err}").into(),
            data: None,
        })
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
    async fn drop_index_under_folders(&self, folders: &[Uri]) {
        if folders.is_empty() {
            return;
        }
        let folder_strs: Vec<String> = folders.iter().map(|u| u.as_str().to_owned()).collect();
        // Hold `documents` across the open-set read + index removals (the
        // `documents` → `workspace_index` order used since the global gate was
        // retired) so a file opening mid-reconcile keeps its open-buffer entry.
        let docs = self.documents.lock().await;
        let open: HashSet<String> = docs.keys().map(|u| u.as_str().to_owned()).collect();
        let mut index = self.workspace_index.write().await;
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
        for uri in &to_remove {
            index.remove_document(uri);
        }
        // Release workspace_index before taking the db locks (never hold
        // workspace_index while acquiring db), but keep `documents` held so the
        // open-file filter above stays current across the db removal too.
        drop(index);
        // Drop the same files from the salsa `Project` so their procs stop
        // resolving cross-file.  Lock order: documents (held) → db → db_files →
        // db_project, matching `did_open`.
        let removed_urls: Vec<Uri> = to_remove
            .iter()
            .filter_map(|u| Uri::from_str(u).ok())
            .collect();
        self.db_remove_sources_batch(&removed_urls).await;
        drop(docs);
        // Clear the Problems / File-Explorer badge of any removed-folder file
        // that still carried one (#865) — it is no longer part of the workspace,
        // so a retained closed-file badge would be stale.
        for uri in &removed_urls {
            if self.pull_diag_cache.lock().await.contains_key(uri) {
                self.clear_closed_diagnostics(uri).await;
            }
        }
    }

    async fn reindex_index_from_disk(&self, uri: &Uri) {
        // Read + analyse off-lock.  Keep the source text + dialect too: the salsa
        // db (cross-file diagnostics) must track the same on-disk population as the
        // workspace index, so a proc defined in a closed/never-opened file still
        // suppresses W123 / drives the arity error in its siblings.
        let scanned: Option<(String, String, AnalysisResult)> =
            if let Some(path) = uri.to_file_path().map(std::borrow::Cow::into_owned) {
                // Read the on-disk text off-lock first, then resolve the dialect
                // from its content the way `did_open` would (#865): a BIG-IP
                // config, an iRule, or a `# tcl-dialect:`-pinned file keeps its
                // real dialect rather than defaulting to generic Tcl.  The salsa
                // `file_analysis_incremental` base pass reads the dialect from the
                // stored `SourceFile`, so it must be right here — not just on the
                // `publish_closed_file_diagnostics` lift path.
                match tokio::task::spawn_blocking(move || std::fs::read_to_string(path).ok())
                    .await
                    .ok()
                    .flatten()
                {
                    Some(text) => {
                        let dialect = self.dialect_for_closed(uri, &text).await;
                        let (a_text, a_dialect) = (text.clone(), dialect.clone());
                        match tokio::task::spawn_blocking(move || {
                            Analyser::new().analyse(&a_text, &a_dialect).clone()
                        })
                        .await
                        {
                            Ok(analysis) => Some((text, dialect, analysis)),
                            Err(_) => None,
                        }
                    }
                    None => None,
                }
            } else {
                None
            };
        // Hold `documents` across the still-closed re-check and both updates (the
        // `documents` → db → `workspace_index` order established by `did_open`) so a
        // concurrent `did_open` cannot open the buffer between them and have its
        // live text clobbered by this disk-backed copy.
        let docs = self.documents.lock().await;
        if docs.contains_key(uri) {
            return;
        }
        // Salsa db first (locks db → db_files → db_project, all *before* the
        // workspace_index lock) to preserve the global lock order.
        match &scanned {
            Some((text, dialect, _)) => {
                self.db_set_source(uri, text.clone(), dialect.clone()).await;
            }
            // No readable on-disk copy (untitled / deleted) — drop it from the
            // project so a stale definition can't keep resolving cross-file.
            None => self.db_remove_source(uri).await,
        }
        let mut index = self.workspace_index.write().await;
        index.remove_document(uri.as_str());
        if let Some((_, _, analysis)) = &scanned {
            index.add_document(uri.as_str(), analysis);
        }
        drop(index);
        // Indexed standalone: invalidate the M9 applied-seed record so the
        // next cross-document query re-applies the source-site views.
        self.rehomed_source_seeds.lock().await.remove(uri.as_str());
    }

    /// Compute and publish a **closed** workspace file's diagnostics from its
    /// on-disk contents (#865), so a file that was opened and had its editor tab
    /// closed keeps its Problems / File-Explorer badge instead of losing it the
    /// moment the tab closes.  Runs the *same* [`run_diagnostics_core`] pipeline
    /// the open path uses — analyser, compiler checks, source-style, and the
    /// cross-file W120/W123 refinement — so a closed file shows exactly the set
    /// it showed while open, kept accurate against disk.
    ///
    /// Callers must have refreshed the on-disk salsa source first (via
    /// [`Self::reindex_index_from_disk`]).  When the URI has no readable on-disk
    /// `SourceFile` (an untitled buffer, or a file deleted between the reindex and
    /// here), there is nothing to analyse and the stale squiggles are cleared
    /// instead — matching the pre-#865 close behaviour for such files.  The
    /// dialect is resolved from the on-disk content (matching the `SourceFile`
    /// dialect the reindex stored), and the run captures a fresh per-URI
    /// generation so a newer refresh supersedes it.
    async fn publish_closed_file_diagnostics(&self, uri: &Uri) {
        // An open buffer is owned by the open path; never shadow it from disk.
        if self.documents.lock().await.contains_key(uri) {
            return;
        }
        // The on-disk-backed salsa source `reindex_index_from_disk` primed; its
        // absence means the URI is untitled / deleted, so clear instead.
        let Some(file) = self.db_files.lock().await.get(uri).copied() else {
            self.clear_closed_diagnostics(uri).await;
            return;
        };
        // Read the handle's text under the `db` lock alone (the `db_files` lock
        // is released), preserving the global `db` → `db_files` order.
        let text = {
            let db = self.db.lock().await;
            file.text(&*db).clone()
        };
        // Resolve the dialect from the on-disk source the same way `did_open`
        // does (basename / BIG-IP, in-source `# tcl-dialect:` directive, folder
        // override, session default) rather than folder-or-default alone, so a
        // closed BIG-IP config or a version-pinned Tcl file keeps the dialect it
        // had while open instead of being re-analysed as generic Tcl.
        let dialect = self.dialect_for_closed(uri, &text).await;
        // Bump this URI's closed-run generation and capture it, so a newer close
        // / watched-change refresh supersedes this run at publish time.
        let generation = self.next_closed_diag_generation(uri).await;
        let inputs = self.diag_inputs(uri, &dialect).await;
        let config = inputs.closed_file_config(uri).await;
        let job = DiagJob {
            text,
            dialect,
            // A closed file carries no editor `language_id`; F5 model dialects are
            // routed by the resolved `dialect` + basename resolved above.
            language_id: String::new(),
            currency: DiagCurrency::ClosedFromDisk(generation),
            version: None,
            file: Some(file),
            config,
        };
        run_diagnostics_core(inputs, uri, job).await;
    }

    /// Resolve the dialect for a **closed** on-disk file the way
    /// [`Self::dialect_for_open`] resolves a freshly opened one, but without an
    /// editor `language_id`: synthesise one from the file extension
    /// ([`tcl_registry::dialect_from_extension`], falling back to the bare
    /// `"tcl"` id that triggers in-source directive detection) so the basename
    /// (BIG-IP), `# tcl-dialect:` / shebang / `package require Tcl` hint,
    /// per-folder override, and session default all still apply (#865).
    async fn dialect_for_closed(&self, uri: &Uri, text: &str) -> String {
        let language_id = tcl_registry::dialect_from_extension(uri.as_str()).unwrap_or("tcl");
        self.dialect_for_open(uri, language_id, text).await
    }

    /// Bump and return `uri`'s closed-file diagnostics generation (#865). Each
    /// closed run captures the value this returns; the publish-time currency
    /// guard ([`DeliveryCtx::is_current`]) drops any run whose captured
    /// generation is no longer the latest, so an older run finishing after a
    /// newer one cannot republish stale diagnostics.
    async fn next_closed_diag_generation(&self, uri: &Uri) -> u64 {
        let mut gens = self.closed_diag_gen.lock().await;
        let slot = gens.entry(uri.clone()).or_insert(0);
        *slot = slot.wrapping_add(1);
        *slot
    }

    /// Clear any previously-published diagnostics for a closed URI that no longer
    /// has an on-disk source (untitled / deleted), and drop its pull-cache entry.
    /// Guarded on the document still being closed, so a `did_open` racing in
    /// cannot have this empty publish blank a freshly reopened buffer.
    async fn clear_closed_diagnostics(&self, uri: &Uri) {
        let docs = self.documents.lock().await;
        if docs.contains_key(uri) {
            return;
        }
        // Hold `documents` across the clearing publish + pull-cache drop (the
        // `documents` → `pull_diag_cache` order), exactly as the open publish
        // holds it, so a concurrent reopen cannot interleave (`RUST_ISSUE_098`).
        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
        self.pull_diag_cache.lock().await.remove(uri);
        // Drop the closed-run generation too, so the map does not accumulate an
        // entry for a URI that no longer carries a badge.
        self.closed_diag_gen.lock().await.remove(uri);
    }

    /// Refresh the diagnostics of every **closed** file that currently carries a
    /// badge (i.e. has a pull-cache entry but is not open), after a workspace-wide
    /// change (a config / master-switch toggle, a disabled-code change) that the
    /// open-document reschedule does not cover.  A closed file that is now clean,
    /// or whose diagnostics are disabled by the new config, is emptied by the
    /// same pipeline; one that still has problems is republished so its badge
    /// stays accurate.
    async fn reschedule_closed_file_diagnostics(&self) {
        // Snapshot the closed-and-badged URIs under `documents` → `pull_diag_cache`
        // (the established lock order) before doing any per-file work.
        let closed: Vec<Uri> = {
            let docs = self.documents.lock().await;
            let cache = self.pull_diag_cache.lock().await;
            cache
                .keys()
                .filter(|uri| !docs.contains_key(*uri))
                .cloned()
                .collect()
        };
        for uri in closed {
            self.publish_closed_file_diagnostics(&uri).await;
        }
    }

    /// Re-run diagnostics for every open document — used after a change to
    /// shared workspace-wide state (config, folder set, or the on-disk
    /// `workspace_index` / `package_resolver` domain) that did **not** originate
    /// from an open document's own edit, so push-diagnostic clients refresh
    /// results that depend on that state (cross-file `crossFileResolution`, and
    /// the always-on W120/W123 workspace refinement) instead of showing stale
    /// diagnostics until the caller is next touched. Unconditional — the W120/W123
    /// refinement runs for every document regardless of the `crossFileResolution`
    /// toggle.
    async fn reschedule_all_open_documents(&self) {
        let snapshot: Vec<(Uri, String)> = {
            let docs = self.documents.lock().await;
            docs.iter()
                .map(|(uri, doc)| (uri.clone(), doc.dialect.clone()))
                .collect()
        };
        for (uri, dialect) in snapshot {
            self.reschedule_diagnostics(uri, dialect).await;
        }
    }

    /// Shared helper for the goto-definition family — runs the
    /// pure-CPU `tcl_lsp_core::definition::definition` provider
    /// off the LSP event loop and returns the matched ranges.
    async fn compute_definition(&self, uri: &Uri, pos: Position) -> jsonrpc::Result<Vec<Location>> {
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
        let analysis_worker = Arc::clone(&analysis);
        let in_doc = tokio::task::spawn_blocking(move || {
            core_definition::definition(&text, pos.line, pos.character, &analysis_worker)
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
        // Cross-file TclOO method definition: a `$obj method` / `my method`
        // call (or a method-name cursor in a class body) whose defining class
        // lives in another file — e.g. an inherited method declared in the
        // base class's own document.  A method call's head is `$obj`, not the
        // method token, so this is not gated on `on_command_head`.  Resolved
        // oracle-aware so a pure-consumer receiver (class defined elsewhere)
        // still identifies the method, and access-context-aware so an
        // external call resolves the exported dispatch entry only
        // (issue #945 faults 4 + 6).
        if let Some((class_q, method, access)) = self
            .resolve_method_target(&doc.text, &doc.dialect, &analysis, pos)
            .await
        {
            let method_defs = self
                .cross_file_method_definition(uri, &class_q, &method, access)
                .await;
            if !method_defs.is_empty() {
                return Ok(method_defs);
            }
        }
        if !on_command_head {
            return Ok(Vec::new());
        }
        // Cross-document fallback: resolve a proc / class
        // defined in a sibling document via the workspace index.
        let cross = self
            .cross_document_definition(uri, &doc.text, pos, &analysis)
            .await?;
        if !cross.is_empty() {
            return Ok(cross);
        }
        // Autoload tier (M8): the command is defined nowhere in the open
        // workspace, but the package / auto-load database may know which
        // library file (`tclIndex` / `pkgIndex.tcl` on the configured
        // `libraryPaths` / `TCLLIBPATH`) defines it.  Resolve that file, analyse
        // it on demand (memoised by `analysis_for`), and jump to the proc.
        self.autoload_definition(&doc.text, pos, &analysis).await
    }

    /// Autoload-tier go-to-definition (M8): resolve a command head that the
    /// workspace index cannot place to the library file the auto-load / package
    /// database says defines it, then jump to that proc's declaration.
    ///
    /// The library file need not be open — [`Self::ensure_autoload_indexed`]
    /// reads it from disk, analyses it, and merges it into the workspace index,
    /// so the jump (and every later references / rename / definition query)
    /// answers from the shared index.  Only fires on a genuine miss, so it
    /// never overrides an in-workspace definition.
    async fn autoload_definition(
        &self,
        source: &str,
        pos: Position,
        analysis: &AnalysisResult,
    ) -> jsonrpc::Result<Vec<Location>> {
        let Some((word, namespace)) = core_definition::command_head_and_namespace_at(
            source,
            analysis,
            pos.line,
            pos.character,
        ) else {
            return Ok(Vec::new());
        };
        let Some(qualified) = self.ensure_autoload_indexed(&word, &namespace).await else {
            return Ok(Vec::new());
        };
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.read().await;
            index
                .proc_definitions_qualified(&qualified, "")
                .into_iter()
                .map(|p| (p.uri.clone(), p.name_span))
                .chain(
                    index
                        .class_definitions_qualified(&qualified, "")
                        .into_iter()
                        .map(|c| (c.uri.clone(), c.name_span)),
                )
                .collect()
        };
        Ok(self.resolve_target_locations(targets).await)
    }

    /// Merge the library file(s) the auto-load / package database says define
    /// `word` into the shared workspace index (M8's second half): references,
    /// rename, and definition then reach library definitions through the same
    /// index queries that serve workspace files.
    ///
    /// Returns the qualified name the command auto-loads under once it is
    /// indexed.  Idempotent — when any auto-qualified candidate already
    /// resolves in the index (a previous merge, or a real workspace
    /// definition), it answers from the index without touching the disk, so a
    /// workspace definition always wins over a same-named library one and
    /// repeated queries stay cheap.  The merged URIs are remembered so a
    /// package-database rebuild can drop them (see
    /// [`Self::scan_workspace_folders`]).
    async fn ensure_autoload_indexed(&self, word: &str, namespace: &str) -> Option<String> {
        let (files, candidates) = {
            let resolver = self.package_resolver.read().await;
            (
                resolver.resolve_auto_command(word, namespace),
                tcl_lsp_core::package_resolver::auto_qualify(word, namespace),
            )
        };
        if files.is_empty() {
            return None;
        }
        // The index and `all_procs` key absolute (`::`-prefixed) names, while
        // `auto_qualify` yields a bare name for a global command.
        let absolute: Vec<String> = candidates
            .iter()
            .map(|cand| {
                if cand.starts_with("::") {
                    cand.clone()
                } else {
                    format!("::{cand}")
                }
            })
            .collect();
        {
            let index = self.workspace_index.read().await;
            if let Some(existing) = absolute
                .iter()
                .find(|cand| index.workspace_command_exists(cand))
            {
                return Some(existing.clone());
            }
        }
        let mut resolved = None;
        for path in files {
            let Some(target_uri) = Uri::from_file_path(&path) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&target_uri).await else {
                continue;
            };
            let file_analysis = self
                .analysis_for(
                    &target_uri,
                    target_doc.text.clone(),
                    target_doc.dialect.clone(),
                )
                .await;
            {
                let mut index = self.workspace_index.write().await;
                index.remove_document(target_uri.as_str());
                index.add_document(target_uri.as_str(), &file_analysis);
            }
            self.autoloaded_library_uris
                .lock()
                .await
                .insert(target_uri.as_str().to_owned());
            if resolved.is_none() {
                resolved = absolute
                    .iter()
                    .find(|cand| {
                        file_analysis.all_procs.contains_key(*cand)
                            || file_analysis.all_classes.contains_key(*cand)
                    })
                    .cloned();
            }
        }
        resolved
    }

    /// Resolve the symbol at `pos` against the workspace index
    /// when the current document has no local definition.  Only
    /// fires on bare command words (not `$var` references), and
    /// returns `Location`s pointing into the *defining*
    /// documents.
    async fn cross_document_definition(
        &self,
        uri: &Uri,
        source: &str,
        pos: Position,
        analysis: &AnalysisResult,
    ) -> jsonrpc::Result<Vec<Location>> {
        // Reconcile the index with the source graph first (M9): sourced
        // documents answer under their source-site namespaces.
        self.refresh_source_rehoming().await;
        // Resolve the call at the cursor to its qualified identity set
        // through the workspace oracle, then jump to *those* symbols'
        // definitions — never every same-simple-name proc/class across the
        // project, which a bare name lookup would surface as spurious extra
        // jump targets.  (A multi-seeded declaration cursor yields several
        // identities sharing one physical definition — the target set
        // dedupes to the physical site.)
        let symbols = self
            .resolve_workspace_symbols(uri, source, analysis, pos)
            .await;
        if symbols.is_empty() {
            return Ok(Vec::new());
        }
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.read().await;
            let mut targets: Vec<(String, tcl_lexer::Span)> = Vec::new();
            for qualified in &symbols {
                let classes = index.class_definitions_qualified(qualified, "");
                // Prefer real `oo::class create` sites over cross-file
                // `oo::define` extension stubs; fall back to every site only
                // when no true creation site is indexed (the class is defined
                // solely by `oo::define`, e.g. on a built-in).
                let creation_sites: Vec<_> = classes.iter().filter(|c| !c.via_define).collect();
                let class_targets: Vec<(String, tcl_lexer::Span)> = if creation_sites.is_empty() {
                    classes
                        .iter()
                        .map(|c| (c.uri.clone(), c.name_span))
                        .collect()
                } else {
                    creation_sites
                        .iter()
                        .map(|c| (c.uri.clone(), c.name_span))
                        .collect()
                };
                targets.extend(
                    index
                        .proc_definitions_qualified(qualified, "")
                        .into_iter()
                        .map(|p| (p.uri.clone(), p.name_span))
                        .chain(class_targets),
                );
            }
            targets.sort_by_key(|(u, s)| (u.clone(), s.start(), s.end()));
            targets.dedup();
            targets
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
            let Ok(parsed) = Uri::from_str(&target_uri) else {
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
                        character: start.character.get(),
                    },
                    end: Position {
                        line: end.line,
                        character: end.character.get(),
                    },
                },
            });
        }
        locations
    }

    /// Resolve the proc / class symbol at `pos` (a bare command word, not a
    /// `$var`) to its **runtime identity set**.  Used by cross-document
    /// references / rename / definition to identify which symbols' call
    /// sites to gather.
    ///
    /// Two cursor shapes resolve, both namespace-exact:
    ///
    /// 1. On a proc / class **declaration name** in this document — the symbol
    ///    whose name span covers the cursor.  A declaration in a document
    ///    sourced under several namespaces is one physical token with one
    ///    runtime identity **per source-site view** (issue #945 fault 3), so
    ///    every seed-mapped identity is returned, never an arbitrary first.
    /// 2. On a **command-head call** — the invocation's ordered resolution
    ///    candidates (caller namespace, each `namespace path` entry, then
    ///    global) walked in Tcl priority order; the first defined in the current
    ///    document or anywhere in the workspace is the call's target.  This is
    ///    the workspace-scoped resolution oracle, replacing a namespace-blind
    ///    `name == word` scan and an arbitrary same-simple-name sibling pick.
    ///
    /// Anything else (a bareword argument, whitespace, a `$var`) resolves to
    /// nothing (an empty set), so no coincidental word links to a sibling
    /// symbol.
    async fn resolve_workspace_symbols(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> Vec<String> {
        if core_hover::find_var_at_position(source, pos.line, pos.character).is_some() {
            return Vec::new();
        }
        let Some(offset) = line_col_to_byte_offset(source, pos.line, pos.character) else {
            return Vec::new();
        };
        let covers =
            |sp: tcl_lexer::Span| (sp.start() as usize) <= offset && offset < (sp.end() as usize);

        // A declaration in a *sourced* document resolves standalone to its
        // global-rooted name; the index holds one source-site re-homed twin
        // per seed (M9), so map through the applied seeds to the full set.
        if let Some(proc_def) = analysis.all_procs.values().find(|p| covers(p.name_span)) {
            return self
                .seed_mapped_symbols(uri, proc_def.qualified_name.clone())
                .await;
        }
        if let Some(class_def) = analysis.all_classes.values().find(|c| covers(c.name_span)) {
            return self
                .seed_mapped_symbols(uri, class_def.qualified_name.clone())
                .await;
        }

        let Some(inv) = analysis
            .command_invocations
            .iter()
            .find(|i| covers(i.range))
        else {
            return Vec::new();
        };
        {
            let index = self.workspace_index.read().await;
            let registry = tcl_registry::registry_for_dialect(&analysis.dialect);
            if let Some(cand) = inv.resolution_candidates.iter().find(|cand| {
                let cand = cand.as_str();
                // A candidate naming a real registry builtin only counts a
                // same-file or cross-file proc definition when that
                // definition isn't itself nested inside another proc's or
                // class's body — the "rename the builtin away, install a
                // same-named shadow, restore it" idiom otherwise makes the
                // shadow permanently outrank the builtin for every call site
                // in the workspace, including ones that run strictly after
                // the shadow has been renamed back off. Mirrors the same
                // gate `resolve_called_proc` (tcl-lsp-core) already applies;
                // `resolve_workspace_symbols` is a separate resolver (the
                // declaration-vs-call-site / cross-file oracle), not a
                // caller of it, so it needs its own copy of the same check.
                let has_builtin = registry.get(cand.trim_start_matches("::")).is_some();
                let same_file_hit = analysis.all_procs.get(cand).is_some_and(|p| {
                    !has_builtin
                        || !analysis.offset_is_inside_any_definition_body(p.name_span.start())
                });
                same_file_hit
                    || analysis.all_classes.contains_key(cand)
                    || index.workspace_command_exists_for_call(cand, has_builtin)
            }) {
                // A candidate that resolves through an `interp alias` /
                // `rename` / `namespace import` names the linked command
                // locally; follow the link so the cursor resolves to the
                // command it ultimately runs, gathering its references with
                // that command's.
                return vec![index.resolve_command_target(cand)];
            }
        }
        // Autoload tier (M8): the command resolves nowhere in the open
        // workspace.  Ask the auto-load / package database, merging the
        // defining library file into the index so this query — and every
        // later references / rename / definition — sees its definitions.
        let Some((word, namespace)) = core_definition::command_head_and_namespace_at(
            source,
            analysis,
            pos.line,
            pos.character,
        ) else {
            return Vec::new();
        };
        self.ensure_autoload_indexed(&word, &namespace)
            .await
            .into_iter()
            .collect()
    }

    /// Cross-document references for the proc / class at `pos`:
    /// every invocation site in *other* documents, plus the
    /// definition sites in other documents when
    /// `include_declaration`.  Returns `Location`s resolved
    /// against their defining documents.
    async fn cross_document_references(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        include_declaration: bool,
    ) -> Vec<Location> {
        self.refresh_source_rehoming().await;
        let symbols = self
            .resolve_workspace_symbols(uri, source, analysis, pos)
            .await;
        if symbols.is_empty() {
            return Vec::new();
        }
        // A multi-seeded declaration names several runtime identities
        // (issue #945 fault 3) — the reference set is the **union over
        // every view**, so a `::x::helper` caller and a `::y::helper`
        // caller both surface from the one physical declaration.
        let targets: Vec<(String, tcl_lexer::Span)> = {
            let index = self.workspace_index.read().await;
            let mut t: Vec<(String, tcl_lexer::Span)> = Vec::new();
            for qualified in &symbols {
                // References follow command name-links: a call reaching the
                // target through an `interp alias` / `rename` / `namespace
                // import` is a use of it, as is the word that names it in
                // such a declaration.
                t.extend(
                    index
                        .linked_invocations_of(qualified, uri.as_str())
                        .into_iter()
                        .map(|i| (i.uri.clone(), i.range)),
                );
                t.extend(index.link_target_spans(qualified, uri.as_str()));
                if include_declaration {
                    // Match the declaration sites by *qualified* name — a same
                    // simple name in an unrelated namespace/file is a different
                    // symbol and must not be surfaced as this one's declaration.
                    for p in index.proc_definitions_qualified(qualified, uri.as_str()) {
                        t.push((p.uri.clone(), p.name_span));
                    }
                    for c in index.class_definitions_qualified(qualified, uri.as_str()) {
                        t.push((c.uri.clone(), c.name_span));
                    }
                }
            }
            t.sort_by_key(|(u, s)| (u.clone(), s.start(), s.end()));
            t.dedup();
            t
        };
        self.resolve_target_locations(targets).await
    }

    /// Resolve the call-site reference [`Location`]s for the symbol whose name
    /// starts at `position` — the locations the code-lens peek
    /// (`tcl-lsp.showReferences`) opens.  Mirrors the [`Self::references`]
    /// handler (local hits + cross-document call sites, deduped) but never
    /// includes the declaration, matching the lens's "N references" call-site
    /// count.
    async fn reference_locations_at(
        &self,
        uri: &Uri,
        text: &str,
        dialect: &str,
        analysis: &AnalysisResult,
        position: Position,
    ) -> Vec<Location> {
        let owned_text = text.to_owned();
        let owned_dialect = dialect.to_owned();
        let analysis_for_worker = analysis.clone();
        let ranges = tokio::task::spawn_blocking(move || {
            core_references::references(
                &owned_text,
                &owned_dialect,
                position.line,
                position.character,
                &analysis_for_worker,
                false,
            )
        })
        .await
        .unwrap_or_default();
        let mut locations: Vec<Location> = ranges
            .into_iter()
            .map(|r| Location {
                uri: uri.clone(),
                range: lift_lsp_range(r),
            })
            .collect();
        let cross = self
            .cross_document_references(uri, text, analysis, position, false)
            .await;
        locations.extend(cross);
        dedup_locations(&mut locations);
        locations
    }

    /// Analyse `source` with the **workspace class set** supplied to instance
    /// inference, so a constructor whose class lives in another file
    /// (`set d [::other::Cls new]`) still records `d`'s class.  Used only by the
    /// cross-file method reference / definition path; the normal cached
    /// analysis (which drives diagnostics) leaves the oracle empty.
    async fn analyse_with_workspace_classes(
        &self,
        source: &str,
        dialect: &str,
    ) -> tcl_compiler::analyser::AnalysisResult {
        let workspace_classes = self.workspace_index.read().await.all_class_qnames();
        let source = source.to_owned();
        let dialect = dialect.to_owned();
        tokio::task::spawn_blocking(move || {
            tcl_compiler::analyser::Analyser::new()
                .with_workspace_classes(workspace_classes)
                .analyse(&source, &dialect)
                .clone()
        })
        .await
        .unwrap_or_default()
    }

    /// Resolve the `TclOO` method `(class, name, access)` under the cursor.
    /// Tries the cached (single-file) analysis first; if that finds nothing
    /// — the common pure-consumer case where `$obj`'s class is defined in
    /// another file and so is invisible to the file-local instance
    /// inference — retries against an analysis carrying the workspace
    /// class oracle.
    async fn resolve_method_target(
        &self,
        source: &str,
        dialect: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> Option<(String, String, core_workspace_index::MethodAccess)> {
        if let Some(target) =
            core_rename::method_target_with_access(source, pos.line, pos.character, analysis)
        {
            return Some(target);
        }
        let oracle = self.analyse_with_workspace_classes(source, dialect).await;
        core_rename::method_target_with_access(source, pos.line, pos.character, &oracle)
    }

    /// Cross-file references from **pure-consumer** documents: `$obj method`
    /// sites where `$obj` is an instance of a class in `(seed_class, method)`'s
    /// override family / inheritor set, in a document that only *uses* the class
    /// (defines no part of it) and so is invisible to
    /// [`Self::cross_file_method_references`].
    ///
    /// Bounds the scan to documents that construct a family instance (via the
    /// index's invocation records) plus the current document — whose consumer
    /// sites the single-document provider also missed — and re-analyses each
    /// with the workspace class oracle so `instance_classes` resolves the
    /// cross-file constructor.  Declarations are never here (a consumer declares
    /// none), so this is independent of `include_declaration`.
    async fn cross_file_consumer_method_references(
        &self,
        current_uri: &Uri,
        current_source: &str,
        current_dialect: &str,
        seed_class: &str,
        method: &str,
    ) -> Vec<Location> {
        // The family + inheritor classes whose instances dispatch `method` to
        // the family, the workspace class oracle, and the candidate consumer
        // documents — collected under one index read.
        let (family, consumer_uris) = {
            let index = self.workspace_index.read().await;
            let mut family: Vec<String> = index
                .method_override_family(seed_class, method)
                .iter()
                .map(|wc| wc.qualified_name.clone())
                .collect();
            family.extend(
                index
                    .method_inheritor_classes(seed_class, method)
                    .iter()
                    .map(|wc| wc.qualified_name.clone()),
            );
            if family.is_empty() {
                return Vec::new();
            }
            let family_norm: std::collections::HashSet<&str> =
                family.iter().map(|s| s.trim_start_matches("::")).collect();
            let consumer_uris = index.documents_invoking_classes(&family_norm);
            (family, consumer_uris)
        };
        let mut out = Vec::new();
        let mut scanned: std::collections::HashSet<String> = std::collections::HashSet::new();
        for u in consumer_uris
            .into_iter()
            .chain(std::iter::once(current_uri.as_str().to_owned()))
        {
            if !scanned.insert(u.clone()) {
                continue;
            }
            let (parsed, source, dialect, line_index, text) = if u == current_uri.as_str() {
                let li = tcl_lexer::LineIndex::new(current_source);
                (
                    current_uri.clone(),
                    current_source.to_owned(),
                    current_dialect.to_owned(),
                    li,
                    current_source.to_owned(),
                )
            } else {
                let Ok(parsed) = Uri::from_str(&u) else {
                    continue;
                };
                let Some(doc) = self.read_document(&parsed).await else {
                    continue;
                };
                (
                    parsed,
                    doc.text.clone(),
                    doc.dialect.clone(),
                    doc.line_index.clone(),
                    doc.text.clone(),
                )
            };
            let analysis = self.analyse_with_workspace_classes(&source, &dialect).await;
            let family_cl = family.clone();
            let method_owned = method.to_owned();
            let spans: Vec<tcl_lexer::Span> = tokio::task::spawn_blocking(move || {
                let mut all: Vec<tcl_lexer::Span> = Vec::new();
                for cq in &family_cl {
                    all.extend(core_references::obj_method_call_sites(
                        &source,
                        &dialect,
                        &analysis,
                        cq,
                        &method_owned,
                    ));
                }
                all
            })
            .await
            .unwrap_or_default();
            for span in spans {
                let start = line_index.position_at_utf16(span.start(), &text);
                let end = line_index.position_at_utf16(span.end(), &text);
                out.push(Location {
                    uri: parsed.clone(),
                    range: Range {
                        start: Position {
                            line: start.line,
                            character: start.character.get(),
                        },
                        end: Position {
                            line: end.line,
                            character: end.character.get(),
                        },
                    },
                });
            }
        }
        out
    }

    /// Cross-file rename of a `TclOO` method across its override family.
    ///
    /// Resolves the workspace-wide override family of `(seed_class, method)`
    /// via the class index, then — for every document that defines a family
    /// class — re-analyses it and collects the method's declaration,
    /// intra-class `my method` calls, and resolvable `$obj method` sites,
    /// converting each to an edit in that document.  Returns the per-URI
    /// edit map (empty when the family is empty, e.g. the class isn't
    /// indexed yet, so the caller can fall back to the single-document
    /// path).
    ///
    /// Coverage is bounded by the analyser's single-document instance
    /// tracking: a `$obj method` site is only rewritten in a document the
    /// index knows defines or inherits the family class (the same constraint
    /// under which the site is resolvable at all), so no unresolved site is
    /// silently left pointing at the old name that the analysis could have
    /// caught.  Documents holding a purely-inheriting subclass (a family
    /// member's descendant that doesn't override the method) are visited too,
    /// so their `my method` / `$obj method` sites are not missed just because
    /// the subclass lives in a different file from the definer.
    async fn cross_file_method_rename(
        &self,
        seed_class: &str,
        method: &str,
        new_name: &str,
    ) -> std::collections::HashMap<Uri, Vec<TextEdit>> {
        let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
            std::collections::HashMap::new();
        // Classes grouped by document: the override-family definers (whose
        // declaration + call sites rename) and the pure inheritors (whose
        // `my method` / `$obj method` sites rename, but which declare no copy
        // of the method).  A document may contribute either or both.
        let by_uri: std::collections::HashMap<String, (Vec<String>, Vec<String>)> = {
            let index = self.workspace_index.read().await;
            let mut m: std::collections::HashMap<String, (Vec<String>, Vec<String>)> =
                std::collections::HashMap::new();
            for wc in index.method_override_family(seed_class, method) {
                m.entry(wc.uri.clone())
                    .or_default()
                    .0
                    .push(wc.qualified_name.clone());
            }
            for wc in index.method_inheritor_classes(seed_class, method) {
                m.entry(wc.uri.clone())
                    .or_default()
                    .1
                    .push(wc.qualified_name.clone());
            }
            m
        };
        for (u, (definers, inheritors)) in by_uri {
            let Ok(parsed) = Uri::from_str(&u) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let analysis = self
                .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect.clone())
                .await;
            let src = target_doc.text.clone();
            let dialect = target_doc.dialect.clone();
            let method_owned = method.to_owned();
            let spans: Vec<tcl_lexer::Span> = tokio::task::spawn_blocking(move || {
                let mut all: Vec<tcl_lexer::Span> = Vec::new();
                for cq in &definers {
                    all.extend(core_rename::method_spans_in_document(
                        &src,
                        &dialect,
                        &analysis,
                        cq,
                        &method_owned,
                    ));
                }
                for cq in &inheritors {
                    all.extend(core_rename::inherited_method_spans_in_document(
                        &src,
                        &dialect,
                        &analysis,
                        cq,
                        &method_owned,
                    ));
                }
                all
            })
            .await
            .unwrap_or_default();
            if spans.is_empty() {
                continue;
            }
            let line_index = target_doc.line_index.clone();
            let bucket = changes.entry(parsed).or_default();
            for span in spans {
                let start = line_index.position_at_utf16(span.start(), &target_doc.text);
                let end = line_index.position_at_utf16(span.end(), &target_doc.text);
                let edit = TextEdit {
                    range: Range {
                        start: Position {
                            line: start.line,
                            character: start.character.get(),
                        },
                        end: Position {
                            line: end.line,
                            character: end.character.get(),
                        },
                    },
                    new_text: new_name.to_owned(),
                };
                if !bucket.iter().any(|e| e.range == edit.range) {
                    bucket.push(edit);
                }
            }
        }
        changes
    }

    /// Cross-document references for a `TclOO` method across its override
    /// family — the reference analogue of [`Self::cross_file_method_rename`].
    ///
    /// Resolves the workspace-wide override family of `(seed_class, method)`,
    /// then for every *sibling* document that defines a family class collects
    /// the method's `$obj method` / `my method` call sites (and, when
    /// `include_declaration`, its declaration), plus the inherited call sites in
    /// documents holding a purely-inheriting subclass.  The current document is
    /// excluded — [`Self::references`] already gathers its method sites from the
    /// single-document provider.  Coverage is bounded the same way rename is: a
    /// site resolves only in a document the index knows defines or inherits the
    /// family class.
    async fn cross_file_method_references(
        &self,
        current_uri: &Uri,
        seed_class: &str,
        method: &str,
        include_declaration: bool,
    ) -> Vec<Location> {
        let by_uri: std::collections::HashMap<String, (Vec<String>, Vec<String>)> = {
            let index = self.workspace_index.read().await;
            let mut m: std::collections::HashMap<String, (Vec<String>, Vec<String>)> =
                std::collections::HashMap::new();
            for wc in index.method_override_family(seed_class, method) {
                m.entry(wc.uri.clone())
                    .or_default()
                    .0
                    .push(wc.qualified_name.clone());
            }
            for wc in index.method_inheritor_classes(seed_class, method) {
                m.entry(wc.uri.clone())
                    .or_default()
                    .1
                    .push(wc.qualified_name.clone());
            }
            m
        };
        let mut out = Vec::new();
        for (u, (definers, inheritors)) in by_uri {
            if u == current_uri.as_str() {
                continue;
            }
            let Ok(parsed) = Uri::from_str(&u) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let analysis = self
                .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect.clone())
                .await;
            let src = target_doc.text.clone();
            let dialect = target_doc.dialect.clone();
            let method_owned = method.to_owned();
            let spans: Vec<tcl_lexer::Span> = tokio::task::spawn_blocking(move || {
                let mut all: Vec<tcl_lexer::Span> = Vec::new();
                for cq in &definers {
                    all.extend(core_references::method_reference_spans_in_document(
                        &src,
                        &dialect,
                        &analysis,
                        cq,
                        &method_owned,
                        include_declaration,
                    ));
                }
                for cq in &inheritors {
                    all.extend(core_rename::inherited_method_spans_in_document(
                        &src,
                        &dialect,
                        &analysis,
                        cq,
                        &method_owned,
                    ));
                }
                all
            })
            .await
            .unwrap_or_default();
            let line_index = target_doc.line_index.clone();
            for span in spans {
                let start = line_index.position_at_utf16(span.start(), &target_doc.text);
                let end = line_index.position_at_utf16(span.end(), &target_doc.text);
                out.push(Location {
                    uri: parsed.clone(),
                    range: Range {
                        start: Position {
                            line: start.line,
                            character: start.character.get(),
                        },
                        end: Position {
                            line: end.line,
                            character: end.character.get(),
                        },
                    },
                });
            }
        }
        out
    }

    /// Cross-file go-to-definition for a `TclOO` method: the declaration
    /// site of the **dispatch entry** — the first implementation on the
    /// receiver class's C-faithful linearisation that is callable under
    /// `access` (issue #945 fault 6: a definition request identifies the
    /// implementation the call actually enters, never the whole override
    /// family; fault 4: an externally-uncallable method resolves to
    /// nothing, mirroring C's `unknown method`).
    ///
    /// The current document participates: a mixin override in this file
    /// outranks the receiver class's own method in another.  (Same-file
    /// simple cases are answered by the in-document provider before this
    /// runs, with the same chain rule.)
    async fn cross_file_method_definition(
        &self,
        _current_uri: &Uri,
        class_q: &str,
        method: &str,
        access: core_workspace_index::MethodAccess,
    ) -> Vec<Location> {
        let chain: Vec<(String, String)> = {
            let index = self.workspace_index.read().await;
            index
                .method_dispatch_chain(class_q, method, access)
                .into_iter()
                .map(|wc| (wc.uri.clone(), wc.qualified_name.clone()))
                .collect()
        };
        // The chain's first record is the dispatch entry; later records are
        // the `next` chain, reachable through the call-hierarchy /
        // references views rather than definition.
        for (u, cq) in chain {
            let Ok(parsed) = Uri::from_str(&u) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await else {
                continue;
            };
            let analysis = self
                .analysis_for(&parsed, target_doc.text.clone(), target_doc.dialect.clone())
                .await;
            if let Some(class_def) = analysis.all_classes.get(&cq)
                && let Some(m) = class_def
                    .methods
                    .get(method)
                    .or_else(|| class_def.class_methods.get(method))
            {
                return self.resolve_target_locations(vec![(u, m.name_span)]).await;
            }
        }
        Vec::new()
    }

    /// Add cross-document rename edits for the proc / class at
    /// `pos` into `changes`.  Resolves the symbol, asks the core
    /// rename provider for the namespace-aware sibling-document
    /// edit intents (call sites + definition sites), converts
    /// each byte span to a range against its target document,
    /// and merges into the per-URI edit map (deduped).
    /// Returns `true` when the rename must be **aborted wholesale**: a
    /// sibling document holds an indirect dispatch of this symbol whose
    /// contributing constants are not all source-writable (issue #945
    /// fault 1) — no edit set can keep that dispatch alive, so not even
    /// the in-document edits may apply.
    ///
    /// A multi-seeded declaration (issue #945 fault 3) is an explicit
    /// **multi-symbol rename**: the one physical token names every
    /// source-site identity, so the edit set is the union over all of
    /// them — each view's callers rewritten — keeping every runtime
    /// identity consistent with the edited declaration.
    async fn add_cross_document_rename_edits(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
        changes: &mut std::collections::HashMap<Uri, Vec<TextEdit>>,
    ) -> bool {
        self.refresh_source_rehoming().await;
        let symbols = self
            .resolve_workspace_symbols(uri, source, analysis, pos)
            .await;
        let intents = {
            let index = self.workspace_index.read().await;
            if symbols.iter().any(|q| index.rename_blocked(q)) {
                return true;
            }
            let mut intents: Vec<core_rename::WorkspaceTextEdit> = Vec::new();
            for qualified in &symbols {
                intents.extend(core_rename::cross_document_symbol_edits(
                    qualified,
                    new_name,
                    &index,
                    uri.as_str(),
                ));
            }
            intents
        };
        self.merge_rename_intents(intents, changes).await;
        false
    }

    /// Consumer-document rename (M8): the cursor's command has no local
    /// definition, so the in-document rename had nothing to resolve against —
    /// not a *rejection* when the workspace (or a library file the autoload
    /// tier merges on demand) defines the symbol.  Resolve through the
    /// workspace oracle and build the whole edit set from the index, the
    /// current document's own call sites included.  Adds nothing when the new
    /// name would shadow an existing workspace command (the same collision
    /// discipline as the in-document rename).
    async fn add_workspace_resolved_rename_edits(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
        changes: &mut std::collections::HashMap<Uri, Vec<TextEdit>>,
    ) {
        self.refresh_source_rehoming().await;
        let symbols = self
            .resolve_workspace_symbols(uri, source, analysis, pos)
            .await;
        if symbols.is_empty() {
            return;
        }
        // The open document's own call sites are edited from the index here (its
        // in-document rename found nothing local to resolve against), and only
        // its *focused* analysis carries the resolution candidates a
        // cross-document match needs.  The background workspace scan can leave
        // this file indexed without that focused walk, dropping the consumer's
        // own edit while the library declaration (matched by name) still lands
        // — a partial rename.  Commit the live focused analysis into the index
        // before reading it, so the current document's edits are deterministic
        // rather than dependent on scan timing.
        {
            let mut index = self.workspace_index.write().await;
            index.remove_document(uri.as_str());
            index.add_document(uri.as_str(), analysis);
        }
        let intents = {
            let index = self.workspace_index.read().await;
            // Every identity of a multi-seeded declaration must accept the
            // rename (no collision, no unwritable indirect dispatch) — a
            // partial multi-symbol edit would leave the views inconsistent,
            // so one refusal aborts them all (issue #945 faults 1 + 3).
            let mut intents: Vec<core_rename::WorkspaceTextEdit> = Vec::new();
            for qualified in &symbols {
                let Some(symbol_intents) =
                    core_rename::workspace_symbol_rename_edits(qualified, new_name, &index)
                else {
                    return;
                };
                intents.extend(symbol_intents);
            }
            intents
        };
        self.merge_rename_intents(intents, changes).await;
    }

    /// Extend `changes` with cross-document rename edits — or resolve
    /// through the workspace oracle when the in-document rename found
    /// nothing local to resolve against (the consumer-document shape).
    /// Extracted from [`LanguageServer::rename`] to keep it within the line
    /// budget.  Returns `true` when the caller must abort the whole rename
    /// (`Ok(None)`): a sibling document dispatches this symbol through a
    /// value whose provenance is not fully writable (issue #945 fault 1).
    ///
    /// Gated on the same safety checks as the in-document path
    /// (`is_safe_symbol_name`, no built-in shadow) so a cross-doc rename
    /// can't produce an unsafe edit set — and skipped entirely when the
    /// in-document rename was rejected (collision, unrenameable cursor;
    /// adding cross-document edits for a locally-rejected rename would leak
    /// a partial, inconsistent edit set into sibling documents).
    async fn extend_rename_with_cross_document_edits(
        &self,
        ctx: RenameContext<'_>,
        local_rejected: bool,
        changes: &mut std::collections::HashMap<Uri, Vec<TextEdit>>,
    ) -> bool {
        let new_name_safe = core_rename::is_safe_symbol_name(ctx.new_name)
            && !core_rename::is_builtin_command_name(ctx.new_name, ctx.registry);
        if !new_name_safe {
            return false;
        }
        if !local_rejected {
            return self
                .add_cross_document_rename_edits(
                    ctx.uri,
                    ctx.source,
                    ctx.analysis,
                    ctx.pos,
                    ctx.new_name,
                    changes,
                )
                .await;
        }
        self.add_workspace_resolved_rename_edits(
            ctx.uri,
            ctx.source,
            ctx.analysis,
            ctx.pos,
            ctx.new_name,
            changes,
        )
        .await;
        false
    }

    /// Resolve each byte-span edit intent to an LSP range against its target
    /// document's source (open buffer or on-disk fallback) and merge it into
    /// the per-URI edit map, deduping identical ranges.
    async fn merge_rename_intents(
        &self,
        intents: Vec<core_rename::WorkspaceTextEdit>,
        changes: &mut std::collections::HashMap<Uri, Vec<TextEdit>>,
    ) {
        for intent in intents {
            let Ok(parsed) = Uri::from_str(&intent.uri) else {
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
                        character: start.character.get(),
                    },
                    end: Position {
                        line: end.line,
                        character: end.character.get(),
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
        current_uri: &Uri,
        qualified: &str,
    ) -> Vec<(Uri, core_call_hierarchy::IncomingCall)> {
        let simple = tcl_compiler::naming::key_tail(qualified);
        // Collect (uri, source, dialect) for every document *other
        // than* the current one: first the open buffers, then the
        // indexed-but-unopened files the folder scan discovered
        // (read from disk).  Without the latter, callers living in
        // files the editor never opened would be missing from the
        // incoming-call results even though they are indexed for
        // every other cross-document feature.
        let mut open_uris: HashSet<Uri> = HashSet::new();
        let mut docs: Vec<(Uri, String, String)> = {
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
        let indexed = self.workspace_index.read().await.document_uris();
        for uri_str in indexed {
            let Ok(uri) = Uri::from_str(&uri_str) else {
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
                .unwrap_or_else(|| Arc::new(Analyser::new().analyse(&source, &dialect).clone()));
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
        current_uri: &Uri,
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
                let index = self.workspace_index.read().await;
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
            let Ok(parsed) = Uri::from_str(&target_uri) else {
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
                    character: start.character.get(),
                },
                end: Position {
                    line: end.line,
                    character: end.character.get(),
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
    /// `symbolMap` (and `optimisationsApplied` for aggressive).
    async fn minify_document_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Uri::from_str(uri_str) else {
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
    async fn optimise_document_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Uri::from_str(uri_str) else {
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
            let line_index = tcl_lexer::LineIndex::new_lsp(&text);
            let items: Vec<serde_json::Value> = opts
                .iter()
                .map(|o| {
                    let start = line_index.position_at_utf16(o.span.start(), &text);
                    let end = line_index.position_at_utf16(o.span.end(), &text);
                    serde_json::json!({
                        "code": o.code.as_str(),
                        "message": o.message,
                        "startLine": start.line,
                        "startCharacter": start.character.get(),
                        "endLine": end.line,
                        "endCharacter": end.character.get(),
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
    /// arguments) is not implemented.
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
    /// for an iRules event.
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
            // Every f5-irules command valid
            // in the event, not just those that carry `event_requires`.
            let names = registry.valid_irules_commands_for_event(&event, &events, &profiles, None);
            let count = names.len();
            let sample: Vec<String> = names.into_iter().take(80).map(str::to_owned).collect();
            (count, sample)
        } else {
            (0, Vec::new())
        };
        // The contract derives `deprecated` from the `when` event-completion
        // detail (`get_event_detail`), which never contains the word — so the
        // describe-event surface always reports false, deliberately *not*
        // `EventProps.deprecated` (the orthogonal F5 deprecation fact).
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
    /// command (exact match, then case-insensitive).
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
    /// Handle `tcl-lsp.listIruleEvents`: the sorted list of all registry
    /// event names.
    fn list_irule_events_command() -> serde_json::Value {
        let events = tcl_registry::events::EventRegistry::build();
        let mut names: Vec<&str> = events.all_event_names();
        names.sort_unstable();
        serde_json::json!({ "events": names })
    }

    /// Handle `tcl-lsp.diagramData`: extract the `when EVENT` event names from
    /// a source string.
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
    /// diagnostic fix iteratively until the source stabilises (a whitelist of
    /// safe fix codes, applied over multiple passes).
    async fn fix_all_safe_issues_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Uri::from_str(uri_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let (disabled, na_mode) = self.analyser_config().await;
        let extra: HashSet<String> = self.extra_commands.lock().await.iter().cloned().collect();
        let dialect = doc.dialect.clone();
        let mut source = doc.text.clone();
        let value = tokio::task::spawn_blocking(move || {
            const SAFE: &[&str] = &["W100", "W105", "W108", "W110", "W201", "W304", "IRULE2001"];
            let mut applied: Vec<serde_json::Value> = Vec::new();
            for _ in 0..4 {
                let mut analyser =
                    Self::configured_analyser(disabled.clone(), na_mode, extra.clone());
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
                            d.code.to_string(),
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
    /// switch, line length, and analyser settings.  Tests poll this command
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
        let dialect = match Uri::from_str(uri_str) {
            Ok(uri) => match self.read_document(&uri).await {
                Some(doc) => doc.dialect,
                None => self.default_dialect.lock().await.clone(),
            },
            Err(_) => self.default_dialect.lock().await.clone(),
        };
        let features = self.feature_toggles.lock().await.resolved_map();
        let optimiser_enabled = *self.optimiser_enabled.lock().await;
        // The optimiser *profile* and the editor `libraryPaths` are as much a
        // part of "what config is in effect" as the master switch, and a caller
        // tracing a surprising diagnostic needs them.  They are also the settle
        // signal the e2e harness polls to know a pulled config has been
        // *applied*: `initialized` pulls the config concurrently with any
        // `didOpen` the client already queued, so without an observable
        // post-apply signal a test cannot know whether its document was
        // analysed before or after its config landed.
        let optimiser_profile = self.optimiser_profile.lock().await.name();
        let library_paths = self.editor_library_paths.lock().await.clone();
        let line_length = *self.line_length.lock().await;
        // Report the *per-folder* analyser settings (the same resolver the
        // feature/diagnostics paths use), not the process-global ones: in a
        // multi-root workspace a folder may override the disabled-codes set /
        // non-ASCII mode, and this "trace where a setting comes from" tool must
        // reflect what actually applies to `uri_str`.  A URI naming no
        // overriding folder (or a single-root workspace) falls back to the
        // global `db_config`, matching the previous behaviour exactly.
        let (mut disabled_sorted, mode) = if let Ok(uri) = Uri::from_str(uri_str) {
            let config = self.resolved_db_config(&uri).await;
            let db = self.db.lock().await;
            (
                config.disabled_diagnostics(&*db).clone(),
                config.non_ascii_mode(&*db),
            )
        } else {
            let (disabled, mode) = self.analyser_config().await;
            (disabled.into_iter().collect::<Vec<String>>(), mode)
        };
        disabled_sorted.sort();
        Ok(Some(serde_json::json!({
            "uri": uri_str,
            "dialect": dialect,
            "features": features,
            "optimiser_enabled": optimiser_enabled,
            "optimiser_profile": optimiser_profile,
            "library_paths": library_paths,
            "line_length": line_length,
            "non_ascii_mode": non_ascii_mode_str(mode),
            "disabled_diagnostics": disabled_sorted,
        })))
    }

    /// Handle `tcl-lsp.setDialect`: switch the session default dialect and
    /// re-resolve every open document under it (so buffers that fell back to
    /// the default re-analyse immediately).  Returns `{success, dialect}`, or
    /// `{success: false, error}` for an unknown dialect.  Drives the VS Code
    /// `tclLsp.selectDialect` command.
    async fn set_dialect_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(dialect) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(Some(serde_json::json!({
                "success": false,
                "error": "setDialect requires a dialect-name argument",
            })));
        };
        if !tcl_dialect::available_dialects().contains(&dialect) {
            return Ok(Some(serde_json::json!({
                "success": false,
                "error": format!("unknown dialect: {dialect}"),
            })));
        }
        *self.default_dialect.lock().await = dialect.to_owned();
        self.reresolve_open_document_dialects().await;
        Ok(Some(
            serde_json::json!({ "success": true, "dialect": dialect }),
        ))
    }

    /// Handle `tcl-lsp.compilerExplorer`: run the compiler pipeline
    /// (lexer → green tree → IR → CFG → SSA → codegen) on `args[0]` under the
    /// dialect in `args[1]` (default: the session dialect) and return the same
    /// serialised JSON the `tcl explore` CLI produces.  Empty/blank source
    /// returns `{error}` — the front-end's "nothing to compile" contract.
    async fn compiler_explorer_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let source = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if source.trim().is_empty() {
            return Ok(Some(serde_json::json!({ "error": "no source to compile" })));
        }
        let dialect = match args.get(1).and_then(serde_json::Value::as_str) {
            Some(d) => d.to_owned(),
            None => self.default_dialect.lock().await.clone(),
        };
        // The pipeline is heavy pure-CPU work — run it off the LSP event loop.
        // A parser panic is contained and surfaced as an `{error}` object
        // rather than tearing down the worker.
        let value = tokio::task::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let result = tcl_explorer::run_pipeline(&source, &dialect);
                tcl_explorer::serialise_result(&result)
            }))
        })
        .await;
        match value {
            Ok(Ok(v)) => Ok(Some(v)),
            _ => Ok(Some(serde_json::json!({
                "error": "compiler explorer failed to analyse the source",
            }))),
        }
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
        // Accept nested / flat-dotted / unwrapped payload shapes (some clients,
        // e.g. JetBrains, don't send the nested `tclLsp` object).
        let cfg = normalize_config_payload(&cfg);
        // Layer the editor's pulled
        // `tclLsp` settings between the user `config.ini` `[global]` (lowest)
        // and the project `.tcl-lsp.ini` `[project]` (highest), applying the
        // documented precedence and merge rules.
        let global_ini = read_ini_layer(
            core_tcl_install::user_config_path(),
            config_ini::Layer::Global,
        );
        let folders = self.workspace_folder_urls().await;
        // Session-level apply uses the primary root's project file (the whole
        // story for a single-root workspace; per-folder applies below refine
        // the analyser knobs for multi-root).
        let primary_project = read_ini_layer(
            folders
                .first()
                .and_then(|f| f.to_file_path().map(std::borrow::Cow::into_owned))
                .map(|root| core_tcl_install::project_config_path(&root)),
            config_ini::Layer::Project,
        );
        // Collapse the retired `features.inlayHints` alias to `inlayTypeHints`
        // *within each layer* before merging. The alias must be resolved
        // per-layer because `merge_settings` is layer-agnostic: a lower layer
        // (the global `config.ini`) carrying an explicit `inlayTypeHints` would
        // otherwise win over a higher layer (the editor) that only sets the
        // `inlayHints` alias, inverting precedence (#728).
        let mut global_ini = global_ini;
        let mut cfg = cfg;
        let mut primary_project = primary_project;
        collapse_inlay_alias(&mut global_ini);
        collapse_inlay_alias(&mut cfg);
        collapse_inlay_alias(&mut primary_project);
        let merged = config_ini::merge_settings(
            &config_ini::merge_settings(&global_ini, &cfg),
            &primary_project,
        );
        self.apply_global_config(&merged).await;
        // Per-folder editor configuration: VS Code resolves `tclLsp` settings
        // per scope, so pull each folder's resolved config, layer it between the
        // global `config.ini` and that folder's `.tcl-lsp.ini`, and store it for
        // longest-prefix resolution at read time.  A single-root / no-folder
        // session skips this — the global pull above is the whole story.
        if !folders.is_empty() {
            let items: Vec<ConfigurationItem> = folders
                .iter()
                .map(|f| ConfigurationItem {
                    scope_uri: Some(f.clone()),
                    section: Some("tclLsp".to_owned()),
                })
                .collect();
            if let Ok(values) = self.client.configuration(items).await {
                let parsed: Vec<(Uri, FolderConfig)> = folders
                    .into_iter()
                    .zip(values)
                    .filter_map(|(folder, editor_cfg)| {
                        let project = read_ini_layer(
                            folder
                                .to_file_path()
                                .map(std::borrow::Cow::into_owned)
                                .map(|root| core_tcl_install::project_config_path(&root)),
                            config_ini::Layer::Project,
                        );
                        let merged = config_ini::merge_settings(
                            &config_ini::merge_settings(&global_ini, &editor_cfg),
                            &project,
                        );
                        parse_folder_config(&merged).map(|fc| (folder, fc))
                    })
                    .collect();
                self.apply_folder_configs(parsed).await;
            }
        }
    }

    /// Apply the *content* of a pulled `tclLsp` config section (`cfg`) onto the
    /// session's global state: feature toggles, the `xcDiagnostics` section
    /// flag, the optimiser switch / profile / per-code overrides, the formatter
    /// line length, the default dialect, the W108 non-ASCII mode, and the
    /// disabled-diagnostic set — then mirror the analyser knobs onto the salsa
    /// config input. A non-object `cfg` is a no-op.
    ///
    /// Split out of [`Self::pull_and_apply_config`] (which fetches `cfg` from
    /// the client and then handles per-folder configs) so the apply logic is
    /// unit-testable without a live editor client.
    // A long but flat sequence of independent `tclLsp.*` knob applications.
    async fn apply_global_config(&self, cfg: &serde_json::Value) {
        if !cfg.is_object() {
            return;
        }
        if let Some(features) = cfg.get("features").and_then(serde_json::Value::as_object) {
            self.feature_toggles.lock().await.apply(features);
        }
        self.apply_global_library_paths(cfg).await;
        self.apply_global_toggles(cfg).await;
        self.apply_global_formatting(cfg).await;
        self.apply_global_analyser_knobs(cfg).await;
        // Mirror the applied analyser knobs onto the salsa config input so the
        // query graph recomputes against the latest settings.
        self.sync_db_config().await;
    }

    /// `tclLsp.libraryPaths` — the editor layer of the package database's
    /// `auto_path` (the user's picked installation / hand-entered paths).
    /// A change rebuilds the database so the new paths take effect immediately.
    async fn apply_global_library_paths(&self, cfg: &serde_json::Value) {
        if let Some(paths) = cfg
            .get("libraryPaths")
            .and_then(serde_json::Value::as_array)
        {
            let libs: Vec<String> = paths
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            let changed = {
                let mut guard = self.editor_library_paths.lock().await;
                if *guard == libs {
                    false
                } else {
                    *guard = libs;
                    true
                }
            };
            if changed {
                self.scan_workspace_folders().await;
            }
        }
    }

    /// The boolean / enum feature switches: `xcDiagnostics`, optimiser
    /// enable/profile/per-code overrides, and the `shimmer` master switch.
    async fn apply_global_toggles(&self, cfg: &serde_json::Value) {
        // `tclLsp.xcDiagnostics.enabled` is a dedicated config section (the
        // shipped VS Code setting "XC Migration: Enabled"), not a `features.*`
        // key, so it must be mapped onto the `xcDiagnostics` feature toggle
        // here.
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
        // `tclLsp.shimmer.enabled` — master switch for the Shimmer family.
        if let Some(flag) = cfg
            .get("shimmer")
            .and_then(|s| s.get("enabled"))
            .and_then(serde_json::Value::as_bool)
        {
            *self.shimmer_enabled.lock().await = flag;
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
        // (false) that O-code on top of the profile.  The pulled `optimiser`
        // section is *authoritative* — rebuild the map from scratch so a code
        // whose override was cleared (the setting reverted to its default)
        // reverts to the profile default instead of retaining the last value.
        // Merging (insert-only) would leak a one-off `optimiser.O100 = true`
        // into every later document once the override is removed.
        if let Some(opt) = cfg.get("optimiser").and_then(serde_json::Value::as_object) {
            let mut overrides = self.optimiser_code_overrides.lock().await;
            overrides.clear();
            for (key, val) in opt {
                if key == "enabled" || key == "profile" {
                    continue;
                }
                if let Some(b) = val.as_bool() {
                    overrides.insert(key.clone(), b);
                }
            }
        }
    }

    /// The formatter / style-width / default-dialect knobs.
    async fn apply_global_formatting(&self, cfg: &serde_json::Value) {
        if let Some(len) = cfg
            .get("formatting")
            .and_then(|f| f.get("lineLength"))
            .or_else(|| cfg.get("formatting").and_then(|f| f.get("maxLineLength")))
            .or_else(|| cfg.get("lineLength"))
            .and_then(serde_json::Value::as_u64)
        {
            *self.line_length.lock().await = u32::try_from(len).unwrap_or(80);
        }
        // The whole `tclLsp.formatting` section drives the formatter config
        // (indent size/style, brace/line-length, blank-line policy, …).
        if let Some(formatting) = cfg.get("formatting") {
            *self.formatting_settings.lock().await = formatting.clone();
        }
        // `tclLsp.style.lineLength` — the W111 threshold (distinct from the
        // formatter width above). A positive value replaces the default 120.
        if let Some(len) = cfg
            .get("style")
            .and_then(|s| s.get("lineLength"))
            .and_then(serde_json::Value::as_u64)
            .filter(|&len| len > 0)
        {
            *self.style_line_length.lock().await = u32::try_from(len).unwrap_or(120);
        }
        if let Some(dialect) = cfg.get("dialect").and_then(serde_json::Value::as_str) {
            *self.default_dialect.lock().await = dialect.to_owned();
        }
    }

    /// The analyser inputs: `extraCommands`, `genericVariablePatterns`, the
    /// non-ASCII (W108) mode, and the disabled-diagnostics set.
    async fn apply_global_analyser_knobs(&self, cfg: &serde_json::Value) {
        // `tclLsp.extraCommands` — names treated as known commands (no W123).
        if let Some(cmds) = cfg
            .get("extraCommands")
            .and_then(serde_json::Value::as_array)
        {
            let extra: Vec<String> = cmds
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            *self.extra_commands.lock().await = extra;
        }
        // `tclLsp.bigipVersion` — the target BIG-IP release for the keyed
        // library-version axis (an empty string clears the pin back to the
        // oldest-supported default).
        if let Some(version) = cfg.get("bigipVersion").and_then(serde_json::Value::as_str) {
            let version = version.trim();
            *self.bigip_version.lock().await = (!version.is_empty()).then(|| version.to_owned());
        }
        // `tclLsp.diagnostics.genericVariablePatterns` — replaces the built-in
        // IRULE4002 generic-name set (an explicit empty list disables the
        // check; an absent key leaves the default).
        if let Some(patterns) = cfg
            .get("diagnostics")
            .and_then(serde_json::Value::as_object)
            .and_then(|d| d.get("genericVariablePatterns"))
            .and_then(serde_json::Value::as_array)
        {
            let patterns: Vec<String> = patterns
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect();
            *self.generic_variable_patterns.lock().await = Some(patterns);
        }
        // The pulled value is the *content* of the `tclLsp` section; the
        // `settings_*` helpers expect it wrapped (they look under `tclLsp`),
        // so re-wrap before reusing them for the W108 mode + disabled codes.
        let wrapped = serde_json::json!({ "tclLsp": cfg.clone() });
        if let Some(mode) = settings_non_ascii_mode(&wrapped) {
            *self.non_ascii_mode.lock().await = mode;
        }
        if let Some(disabled) = settings_disabled_diagnostics(&wrapped) {
            *self.disabled_diagnostics.lock().await = disabled;
        }
        if let Some(overrides) = settings_severity_overrides(&wrapped) {
            *self.severity_overrides.lock().await = overrides;
        }
    }

    /// Store the per-folder editor configs and refresh the per-folder salsa
    /// `AnalyserConfig` handles.  A handle is created only for a folder that
    /// overrides the disabled-diagnostics set or non-ASCII mode (others inherit
    /// [`Backend::db_config`]); existing handles are reused across re-pulls so
    /// the salsa store does not accumulate dead config inputs.
    async fn apply_folder_configs(&self, parsed: Vec<(Uri, FolderConfig)>) {
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
            // Per-folder `extraCommands` / `genericVariablePatterns` override the
            // process-global value when set; otherwise the folder inherits it.
            let global_extra = self.extra_commands.lock().await.clone();
            let global_generic = self.generic_variable_patterns.lock().await.clone();
            let mut db = self.db.lock().await;
            let mut handles = self.folder_db_configs.lock().await;
            let mut next: Vec<(Uri, tcl_lsp_db::AnalyserConfig)> = Vec::new();
            for (folder, fc) in &parsed {
                // A handle is only needed when the folder overrides one of the
                // analyser-config inputs.
                if fc.disabled_diagnostics.is_none()
                    && fc.non_ascii_mode.is_none()
                    && fc.extra_commands.is_none()
                    && matches!(fc.generic_variable_patterns, FolderGenericPatterns::Inherit)
                {
                    continue;
                }
                let mut disabled: Vec<String> = match &fc.disabled_diagnostics {
                    Some(d) => d.iter().cloned().collect(),
                    None => global_disabled.clone(),
                };
                disabled.sort();
                let mode = fc.non_ascii_mode.unwrap_or(global_mode);
                let extra = fc
                    .extra_commands
                    .clone()
                    .unwrap_or_else(|| global_extra.clone());
                let generic = match &fc.generic_variable_patterns {
                    FolderGenericPatterns::Inherit => global_generic.clone(),
                    FolderGenericPatterns::BuiltinDefaults => None,
                    FolderGenericPatterns::Replace(list) => Some(list.clone()),
                };
                if let Some((_, handle)) = handles.iter().find(|(u, _)| u == folder) {
                    handle.set_disabled_diagnostics(&mut *db).to(disabled);
                    handle.set_non_ascii_mode(&mut *db).to(mode);
                    handle.set_extra_commands(&mut *db).to(extra);
                    handle.set_generic_variable_patterns(&mut *db).to(generic);
                    next.push((folder.clone(), *handle));
                } else {
                    let handle =
                        tcl_lsp_db::AnalyserConfig::new(&*db, disabled, mode, extra, generic, None);
                    next.push((folder.clone(), handle));
                }
            }
            *handles = next;
        }
        *self.folder_configs.lock().await = parsed;
    }

    /// Whether the named `tclLsp.features.*` provider is enabled.
    async fn feature_enabled(&self, feature: &str, uri: &Uri) -> bool {
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
    /// defaults **off**, so it cannot reuse
    /// `feature_enabled` (whose absent-key fallback is `true`).
    async fn will_save_format_enabled(&self, uri: &Uri) -> bool {
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
    /// Inlay hints are opt-in and default **off**.  The two families —
    /// `inlayTypeHints` (inferred variable types + format-string specifier
    /// labels) and `inlayParameterHints` (call-site parameter-name labels)
    /// — gate independently; the retired `inlayHints` key is normalised to
    /// `inlayTypeHints` on input (see [`FeatureToggles::apply`]).  Resolves
    /// per folder like [`Self::feature_enabled`] so a folder-scoped opt-in
    /// works in a multi-root workspace.
    async fn inlay_family_enabled(&self, uri: &Uri, family: &str) -> bool {
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
    /// per folder like [`Self::inlay_family_enabled`].  Surfaced only on
    /// `f5-irules` documents (the only dialect the `f5-xc` translator runs
    /// on) — see [`Self::cross_file_resolution_enabled`] for the separate,
    /// dialect-general cross-file toggle.
    async fn xc_diagnostics_enabled(&self, uri: &Uri) -> bool {
        self.inlay_family_enabled(uri, "xcDiagnostics").await
    }

    /// Whether opt-in cross-file resolution (cross-file W120/W123
    /// suppression + cross-file E002/E003 arity) is enabled for `uri`.
    /// Default **off** (the underlying salsa `project` query walks every
    /// file in the workspace, so it's opt-in for perf), resolved per
    /// folder like [`Self::inlay_family_enabled`]. Applies to *every*
    /// dialect — deliberately independent of [`Self::xc_diagnostics_enabled`],
    /// which gates only the unrelated, f5-irules-specific XC100-301
    /// translatability lints.
    async fn cross_file_resolution_enabled(&self, uri: &Uri) -> bool {
        self.inlay_family_enabled(uri, "crossFileResolution").await
    }

    /// Handle `tcl-lsp.listSubcommands`: subcommand metadata for `command`
    /// from the registry.
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
        let profile = tcl_dialect::DialectProfile::by_name(&dialect);
        let mut subs: Vec<serde_json::Value> = {
            use tcl_registry::ProfileQueries;
            profile.resolve_command(&registry, &name)
        }
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
    /// `package require` resolver, so it reports the empty
    /// set — the contract is a `packages` list, which downstream callers can
    /// rely on regardless of population.
    fn list_known_packages_command() -> serde_json::Value {
        serde_json::json!({ "packages": serde_json::Value::Array(vec![]) })
    }

    /// `tcl-lsp.listTclInstallations` — report the Tcl installations discovered
    /// on disk (for the editor's "select a Tcl installation" picker) plus the
    /// `auto_path` currently feeding the package database. The editor writes
    /// the chosen `tcl_library` (or a custom path) back to
    /// `tclLsp.libraryPaths`.
    async fn list_tcl_installations_command(&self) -> serde_json::Value {
        let discovered = Arc::clone(&self.discovered_tcl);
        let editor_paths = self.editor_library_paths.lock().await.clone();
        let roots: Vec<PathBuf> = self
            .workspace_folder_urls()
            .await
            .iter()
            .filter_map(|f| f.to_file_path().map(std::borrow::Cow::into_owned))
            .collect();
        tokio::task::spawn_blocking(move || {
            let installs = discovered.get_or_init(|| {
                core_tcl_install::discover(&core_tcl_install::default_search_bases())
            });
            let active = effective_auto_path(&roots, &editor_paths, installs);
            serde_json::json!({
                "installations": installs
                    .iter()
                    .map(|i| serde_json::json!({
                        "version": i.version,
                        "tclLibrary": i.tcl_library.to_string_lossy(),
                        "autoPath": i.auto_path.iter()
                            .map(|p| p.to_string_lossy().into_owned())
                            .collect::<Vec<_>>(),
                    }))
                    .collect::<Vec<_>>(),
                "activeAutoPath": active.iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect::<Vec<_>>(),
                "editorLibraryPaths": editor_paths,
            })
        })
        .await
        .unwrap_or(serde_json::Value::Null)
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
    /// `~/.config/tcl-lsp/config.ini`) and return its path.  Only the resolved
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
        uri: &Uri,
    ) -> (HashSet<String>, NonAsciiMode, bool, HashSet<String>) {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).cloned()
        };
        let mut disabled = match folder.as_ref().and_then(|f| f.disabled_diagnostics.clone()) {
            Some(d) => d,
            None => self.disabled_diagnostics.lock().await.clone(),
        };
        // `tclLsp.shimmer.enabled = false` suppresses the whole Shimmer family
        // (S100–S110); fold those codes into the effective disabled set so the
        // compiler-check lift drops them (the analyser never emits them).
        let shimmer_enabled = match folder.as_ref().and_then(|f| f.shimmer_enabled) {
            Some(b) => b,
            None => *self.shimmer_enabled.lock().await,
        };
        if !shimmer_enabled {
            for code in ["S100", "S101", "S102", "S103", "S110"] {
                disabled.insert(code.to_owned());
            }
        }
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

    /// Resolve the per-code severity overrides for `uri`: the longest-matching
    /// folder's `tclLsp.diagnosticSeverity` map when set, else the process-global
    /// map. Mirrors the `disabled_diagnostics` resolution in
    /// [`Self::resolved_analysis_settings`].
    async fn resolved_severity_overrides(
        &self,
        uri: &Uri,
    ) -> std::collections::HashMap<String, tower_lsp_server::ls_types::DiagnosticSeverity> {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).cloned()
        };
        match folder.and_then(|f| f.severity_overrides) {
            Some(m) => m,
            None => self.severity_overrides.lock().await.clone(),
        }
    }

    /// Resolve the formatter line length for `uri` (per-folder override, else
    /// the global `tclLsp.formatting.lineLength`).
    async fn resolved_line_length(&self, uri: &Uri) -> u32 {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).and_then(|f| f.line_length)
        };
        match folder {
            Some(len) => len,
            None => *self.line_length.lock().await,
        }
    }

    /// The resolved `tclLsp.extraCommands` for `uri`: a folder override wins,
    /// else the process-global set.
    async fn resolved_extra_commands(&self, uri: &Uri) -> Vec<String> {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).and_then(|f| f.extra_commands.clone())
        };
        match folder {
            Some(v) => v,
            None => self.extra_commands.lock().await.clone(),
        }
    }

    /// The resolved `tclLsp.diagnostics.genericVariablePatterns` for `uri`: a
    /// folder override wins, else the process-global value.
    async fn resolved_generic_variable_patterns(&self, uri: &Uri) -> Option<Vec<String>> {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).map(|f| f.generic_variable_patterns.clone())
        };
        match folder {
            // `Replace`/`BuiltinDefaults` are the folder's own override; only
            // `Inherit` (or no matching folder) falls back to the global value.
            Some(FolderGenericPatterns::Replace(list)) => Some(list),
            Some(FolderGenericPatterns::BuiltinDefaults) => None,
            Some(FolderGenericPatterns::Inherit) | None => {
                self.generic_variable_patterns.lock().await.clone()
            }
        }
    }

    /// The resolved `tclLsp.formatting` settings object for `uri`: a folder
    /// override wins, else the process-global section (or `Null`).
    async fn resolved_formatting(&self, uri: &Uri) -> serde_json::Value {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).and_then(|f| f.formatting.clone())
        };
        match folder {
            Some(v) => v,
            None => self.formatting_settings.lock().await.clone(),
        }
    }

    /// The resolved W111 source-style line length (`tclLsp.style.lineLength`)
    /// for `uri`: a folder override wins, else the process-global value.
    /// Distinct from [`Self::resolved_line_length`] (the formatter width).
    async fn resolved_style_line_length(&self, uri: &Uri) -> u32 {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).and_then(|f| f.style_line_length)
        };
        match folder {
            Some(len) => len,
            None => *self.style_line_length.lock().await,
        }
    }

    /// Build an `Analyser` carrying the configured disabled-diagnostics
    /// set and W108 mode.
    fn configured_analyser(
        disabled: HashSet<String>,
        mode: NonAsciiMode,
        extra_commands: HashSet<String>,
    ) -> Analyser {
        Analyser::with_disabled_diagnostics(disabled)
            .with_non_ascii_mode(mode)
            .with_extra_commands(extra_commands)
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
    /// diagnostics to the LSP client for `uri`.  Runs
    /// the analyser on a `spawn_blocking` worker so the LSP
    /// event loop stays responsive.
    /// Gather the document-independent handles a detached diagnostics run needs
    /// (per-edit state travels in a [`DiagJob`]).
    async fn diag_inputs(&self, uri: &Uri, dialect: &str) -> DiagInputs {
        let (disabled, non_ascii_mode, optimiser_enabled, opt_disabled) =
            self.resolved_analysis_settings(uri).await;
        let severity_overrides = self.resolved_severity_overrides(uri).await;
        let registry = self.registry_for_dialect(dialect).await;
        let xc_diagnostics = self.xc_diagnostics_enabled(uri).await;
        let cross_file_resolution = self.cross_file_resolution_enabled(uri).await;
        let extra_commands = self
            .resolved_extra_commands(uri)
            .await
            .into_iter()
            .collect();
        let generic_variable_patterns = self.resolved_generic_variable_patterns(uri).await;
        let style_line_length = self.resolved_style_line_length(uri).await;
        let diagnostics_enabled = self.feature_enabled("diagnostics", uri).await;
        let (entry_points, folder_root) = self.w120_inheritance_config(uri).await;
        DiagInputs {
            client: self.client.clone(),
            registry,
            disabled,
            severity_overrides,
            extra_commands,
            generic_variable_patterns,
            style_line_length,
            non_ascii_mode,
            opt_disabled,
            documents: Arc::clone(&self.documents),
            workspace_index: Arc::clone(&self.workspace_index),
            rehomed_source_seeds: Arc::clone(&self.rehomed_source_seeds),
            package_resolver: Arc::clone(&self.package_resolver),
            entry_points,
            folder_root,
            db: Arc::clone(&self.db),
            db_files: Arc::clone(&self.db_files),
            db_project: Arc::clone(&self.db_project),
            db_config: Arc::clone(&self.db_config),
            folder_db_configs: Arc::clone(&self.folder_db_configs),
            pull_diag_cache: Arc::clone(&self.pull_diag_cache),
            closed_diag_gen: Arc::clone(&self.closed_diag_gen),
            toggles: DiagToggles {
                diagnostics_enabled,
                optimiser_enabled,
                xc: XcToggles {
                    xc_diagnostics,
                    cross_file_resolution,
                },
            },
            client_supports_pull: self
                .client_supports_pull_diagnostics
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// The project entry points and folder root that govern the #804 W120
    /// inheritance for `uri`: the longest matching workspace folder's
    /// `.tcl-lsp.ini [project] entryPoints` (empty ⇒ automatic `source`-graph
    /// mode) and that folder's filesystem root (to resolve relative entry
    /// paths).
    async fn w120_inheritance_config(&self, uri: &Uri) -> (Vec<String>, Option<PathBuf>) {
        let configs = self.folder_configs.lock().await;
        let mut best: Option<&(Uri, FolderConfig)> = None;
        for entry in configs.iter() {
            if uri_under_folder(uri.as_str(), entry.0.as_str())
                && best.is_none_or(|b| entry.0.as_str().len() > b.0.as_str().len())
            {
                best = Some(entry);
            }
        }
        match best {
            Some((folder, fc)) => (
                fc.entry_points.clone().unwrap_or_default(),
                folder.to_file_path().map(std::borrow::Cow::into_owned),
            ),
            None => (Vec::new(), None),
        }
    }

    /// The extra `package require` names `uri` inherits for the W120 refinement
    /// (#804), resolved live against the current workspace index. Empty in the
    /// common single-file / no-project case.
    async fn inherited_package_requires(&self, uri: &Uri) -> Vec<String> {
        let (entry_points, folder_root) = self.w120_inheritance_config(uri).await;
        let index = self.workspace_index.read().await;
        compute_inherited_requires(&index, uri, &entry_points, folder_root.as_deref())
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
    /// The project's proc arities for cross-file resolution, or `None` when
    /// `crossFileResolution` is off.  Computed inside `spawn_blocking`: on a
    /// cold cache this tracked query can demand `item_sigs` / `item_tree` for
    /// every project file, so it must not run on the async event-loop thread.
    async fn project_arities_if(
        &self,
        cross_file_on: bool,
    ) -> Option<Arc<HashMap<String, Vec<(usize, usize)>>>> {
        if !cross_file_on {
            return None;
        }
        let db = self.db.lock().await.clone();
        let project = *self.db_project.lock().await;
        match project {
            Some(p) => {
                tokio::task::spawn_blocking(move || tcl_lsp_db::project_command_arities(&db, p))
                    .await
                    .ok()
            }
            None => None,
        }
    }

    /// Run the compiler-check diagnostics for `text` off the event-loop thread.
    async fn compiler_diagnostics_for(
        &self,
        uri: &Uri,
        text: &str,
        dialect: &str,
        registry: &Arc<CommandRegistry>,
    ) -> tcl_lsp_db::CompilerDiagnostics {
        let (c_text, c_dialect, c_registry) =
            (text.to_owned(), dialect.to_owned(), Arc::clone(registry));
        // URI-scoped (folder/project override aware), matching the push
        // path's `resolved_generic_variable_patterns(uri)` so IRULE4002
        // honours a folder's `diagnostics.genericVariablePatterns` override.
        let c_generic = self.resolved_generic_variable_patterns(uri).await;
        tokio::task::spawn_blocking(move || {
            tcl_lsp_db::compiler_check_diagnostics_uncached(
                &c_text,
                &c_registry,
                &c_dialect,
                c_generic.as_deref(),
            )
        })
        .await
        .unwrap_or_else(|_| tcl_lsp_db::CompilerDiagnostics {
            checks: Vec::new(),
            optimisations: Vec::new(),
        })
    }

    async fn full_diagnostics_for(
        &self,
        uri: &Uri,
        text: String,
        dialect: String,
        language_id: &str,
    ) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
        // Master switch off (`tclLsp.features.diagnostics = false`): the pull
        // path returns an empty report, matching the push path's empty-publish.
        if !self.feature_enabled("diagnostics", uri).await {
            return Vec::new();
        }
        let (disabled, _non_ascii_mode, optimiser_enabled, opt_disabled) =
            self.resolved_analysis_settings(uri).await;
        // F5 dialect dispatch: BIG-IP config / iApp APL
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

        // Cross-file: the project's proc arities, so the
        // pull path resolves cross-file W123 / emits the cross-file arity error
        // exactly as the push path does.  Only gathered when `crossFileResolution`
        // is enabled.  Computed inside `spawn_blocking`: on a cold cache (or after a
        // signature change) this tracked query can demand `item_sigs` / `item_tree`
        // for every project file — now the *whole* workspace — so it must not run
        // on the async event-loop thread and stall other LSP traffic.
        let cross_file_on = self.cross_file_resolution_enabled(uri).await;
        let project_arities = self.project_arities_if(cross_file_on).await;
        let compiler_diags = self
            .compiler_diagnostics_for(uri, &text, &dialect, &registry)
            .await;

        // XC100-301 translatability lints — independent toggle, f5-irules only.
        let xc_on = self.xc_diagnostics_enabled(uri).await;
        let xc_for_irules = dialect == "f5-irules" && xc_on;
        // Cross-file resolution — W123 suppression + E002/E003 arity,
        // matching the push path.  Arity keys off `unresolved_command_sites`, which
        // the analyser records regardless of the W123 toggle, so disabling W123
        // does not also silence cross-file arity.
        let analyser_diags = match &project_arities {
            Some(arities) => tcl_lsp_db::apply_cross_file_resolution(
                &analysis.diagnostics,
                &analysis.unresolved_command_sites,
                &analysis.command_invocations,
                arities,
                |code| disabled.contains(code),
            ),
            None => analysis.diagnostics.clone(),
        };
        // #723: refine the single-file W120 against the workspace package
        // database, mirroring the push path's `refine_and_lift_diagnostics`, so a
        // workspace whose `pkgIndex.tcl`/`libraryPaths` prove a required package
        // transitively provides the flagged package suppresses the false W120.
        // #804: also inherit the requires of the project's entry files / the
        // `source` ancestors of this document. Only computed when there is a
        // W120 to refine, matching the push path — otherwise the workspace-index
        // lock and `source`-graph walk are avoidable work.
        let inherited_requires = if analyser_diags
            .iter()
            .any(|d| d.code == DiagCode::W120 || d.code == DiagCode::W123)
        {
            self.inherited_package_requires(uri).await
        } else {
            Vec::new()
        };
        let analyser_diags = refine_workspace_w120(
            analyser_diags,
            analysis.as_ref(),
            &inherited_requires,
            &self.package_resolver,
            &registry,
        )
        .await;
        // #832: drop any W123 the package database can resolve (auto-loaded
        // library command, or an available package's defined command), mirroring
        // the push path so pull and push stay behaviour-identical.
        let analyser_diags = refine_workspace_w123(
            analyser_diags,
            analysis.as_ref(),
            &inherited_requires,
            &self.package_resolver,
            &dialect,
        )
        .await;
        let style_line_length = self.resolved_style_line_length(uri).await;
        let severity_overrides = self.resolved_severity_overrides(uri).await;
        tokio::task::spawn_blocking(move || {
            let mut diagnostics = lift_analyser_diagnostics(&text, &analyser_diags);
            append_brace_expr_perf_hints(&mut diagnostics, optimiser_enabled, &opt_disabled);
            diagnostics.extend(lift_compiler_diagnostics(
                &text,
                &compiler_diags,
                optimiser_enabled,
                &opt_disabled,
                &disabled,
                &analysis.suppressed_lines,
            ));
            diagnostics.extend(lift_source_style_diagnostics(
                &text,
                &analysis.suppressed_lines,
                &disabled,
                style_line_length as usize,
            ));
            // Opt-in: XC100-301 translatability diagnostics for
            // `f5-irules` documents when `xcDiagnostics` is enabled (mirrors
            // the push path).
            if xc_for_irules {
                diagnostics.extend(lift_xc_diagnostics(
                    &text,
                    &disabled,
                    &analysis.suppressed_lines,
                ));
            }
            apply_severity_overrides(&mut diagnostics, &severity_overrides);
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
        uri: Uri,
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
    async fn schedule_diagnostics(&self, uri: Uri, dialect: String) {
        // Edit path: config is unchanged, so reuse the slot's cached inputs (the
        // worker still reads the document's *current* content at drain time).
        self.schedule_diagnostics_impl(uri, dialect, false).await;
    }

    /// Like [`Self::schedule_diagnostics`] but forces a re-resolve of the
    /// config-sensitive inputs, so an already-running worker drains under the
    /// *new* toggles.  Used by the config-change paths (`optimiser.enabled`,
    /// `features.diagnostics`, dialect switch) where the cached inputs are stale
    /// even though the document text has not changed.
    async fn reschedule_diagnostics(&self, uri: Uri, dialect: String) {
        self.schedule_diagnostics_impl(uri, dialect, true).await;
    }

    async fn schedule_diagnostics_impl(&self, uri: Uri, dialect: String, force_refresh: bool) {
        // Only (re)resolve the relatively expensive `diag_inputs` when the
        // worker has none yet or a config change forces it — an edit reuses the
        // cached inputs.  Peek that decision under the lock without mutating.
        let need_inputs = {
            let mut slots = self.diag_slots.lock().await;
            let slot = slots.entry(uri.clone()).or_default();
            force_refresh || slot.latest_inputs.is_none()
        };
        // Resolve the fresh inputs *before* marking the slot dirty. Marking
        // dirty first and storing `latest_inputs` only after the `await` let a
        // running worker drain the dirty flag with the *stale* inputs in that
        // window, silently dropping a config change (e.g. squiggles the user
        // just disabled would persist until the next keystroke) —
        // RUST_ISSUE_102. Resolving first means `dirty` and `latest_inputs` are
        // published together, atomically, so the worker never observes one
        // without the other.
        let fresh_inputs = if need_inputs {
            Some(self.diag_inputs(&uri, &dialect).await)
        } else {
            None
        };
        // Commit `latest_inputs` (if freshly resolved) and mark dirty in a
        // single locked section — no `await` between them — so a storm of rapid
        // edits still coalesces predictably.
        let start_worker = {
            let mut slots = self.diag_slots.lock().await;
            let slot = slots.entry(uri.clone()).or_default();
            if let Some(inputs) = fresh_inputs {
                slot.latest_inputs = Some(inputs);
            }
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
        let slots = Arc::clone(&self.diag_slots);
        tokio::spawn(async move {
            loop {
                // Debounce, then claim the dirty flag *and* the freshest inputs.
                // A burst collapses to one run; with nothing dirty we retire the
                // worker (under the lock, so a concurrent edit either sees
                // `running` and skips the spawn while we run, or sees
                // `!running` and starts a fresh one).
                tokio::time::sleep(DIAGNOSTICS_DEBOUNCE).await;
                let inputs = {
                    let mut guard = slots.lock().await;
                    let Some(slot) = guard.get_mut(&uri) else {
                        return;
                    };
                    if slot.dirty {
                        slot.dirty = false;
                        slot.latest_inputs.clone()
                    } else {
                        slot.running = false;
                        return;
                    }
                };
                // `latest_inputs` is set alongside `dirty`, so this is `Some`;
                // guard defensively and retire if it is somehow absent.
                let Some(inputs) = inputs else {
                    if let Some(slot) = slots.lock().await.get_mut(&uri) {
                        slot.running = false;
                    }
                    return;
                };
                // Capture + analyse the document's current state.  If it is gone
                // (closed) there is nothing to publish — retire.
                let Some(job) = inputs.capture_job(&uri).await else {
                    let mut guard = slots.lock().await;
                    if let Some(slot) = guard.get_mut(&uri) {
                        slot.running = false;
                    }
                    return;
                };
                let settled = run_diagnostics_core(inputs, &uri, job).await;
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
    /// A client
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
    /// M9: bring the index's per-document views in line with the *source
    /// graph* — `source` evaluates a file in the caller's namespace, so a
    /// document sourced from `namespace eval ::x` must be indexed under a
    /// `::x`-seeded analysis (its bare `proc helper` is really
    /// `::x::helper`).  Computes the desired seed set per sourced document
    /// (literal paths, plus statically-foldable `[file join …]` forms —
    /// stage 9.2), re-analyses documents whose applied seeds differ, and
    /// merges each seeded view into the index (one document may carry
    /// several views when sourced from several namespaces — all true at
    /// run time).  Iterates to a fixpoint (bounded) because a seeded parent
    /// records *composed* namespaces for its own nested `source` calls.
    async fn refresh_source_rehoming(&self) {
        for _round in 0..4 {
            let desired = {
                let index = self.workspace_index.read().await;
                if !index.has_source_edges() && self.rehomed_source_seeds.lock().await.is_empty() {
                    return;
                }
                index.source_seed_map(resolve_source_edge)
            };
            let recorded = self.rehomed_source_seeds.lock().await.clone();
            let mut work: Vec<(String, Vec<String>)> = Vec::new();
            for (uri, seeds) in &desired {
                let seeds: Vec<String> = seeds.iter().cloned().collect();
                if recorded.get(uri) != Some(&seeds) {
                    work.push((uri.clone(), seeds));
                }
            }
            for (uri, seeds) in &recorded {
                if !desired.contains_key(uri) && seeds != &["::".to_owned()] {
                    // No longer sourced from anywhere: restore the standalone view.
                    work.push((uri.clone(), vec!["::".to_owned()]));
                }
            }
            if work.is_empty() {
                return;
            }
            for (uri_s, seeds) in work {
                let Ok(uri) = Uri::from_str(&uri_s) else {
                    continue;
                };
                let Some(doc) = self.read_document(&uri).await else {
                    continue;
                };
                let text = doc.text.clone();
                let dialect = doc.dialect.clone();
                let seeds_for_worker = seeds.clone();
                let Ok(analyses) = tokio::task::spawn_blocking(move || {
                    seeds_for_worker
                        .iter()
                        .map(|seed| {
                            let mut analyser = Analyser::new();
                            if seed == "::" {
                                analyser.analyse(&text, &dialect).clone()
                            } else {
                                analyser
                                    .analyse_with_source_namespace(&text, &dialect, seed)
                                    .clone()
                            }
                        })
                        .collect::<Vec<AnalysisResult>>()
                })
                .await
                else {
                    continue;
                };
                {
                    let mut index = self.workspace_index.write().await;
                    index.remove_document(&uri_s);
                    for analysis in &analyses {
                        index.add_document(&uri_s, analysis);
                    }
                }
                if seeds == ["::".to_owned()] {
                    self.rehomed_source_seeds.lock().await.remove(&uri_s);
                } else {
                    self.rehomed_source_seeds.lock().await.insert(uri_s, seeds);
                }
            }
        }
    }

    /// M9, declaration side: a cursor on a definition inside a *sourced*
    /// document resolves, in that document's standalone analysis, to its
    /// global-rooted name — but the index holds one re-homed twin **per
    /// source-site namespace**.  One physical declaration is several
    /// runtime identities (`namespace eval ::x {source b.tcl}` +
    /// `namespace eval ::y {source b.tcl}` creates both `::x::helper`
    /// and `::y::helper` — tclsh 9.0.4), so the mapping returns the
    /// **full identity set**, never an arbitrary first seed (issue #945
    /// fault 3): references union every view's call sites, and a rename
    /// of the one physical token is explicitly a multi-symbol edit.
    async fn seed_mapped_symbols(&self, uri: &Uri, qualified: String) -> Vec<String> {
        let seeds = self
            .rehomed_source_seeds
            .lock()
            .await
            .get(uri.as_str())
            .cloned()
            .unwrap_or_default();
        if seeds.is_empty() {
            return vec![qualified];
        }
        let index = self.workspace_index.read().await;
        let mut out: Vec<String> = Vec::new();
        for seed in &seeds {
            let mapped = if seed == "::" {
                // A standalone / global-sourced view exists alongside; the
                // plain name is one of the real identities.
                qualified.clone()
            } else {
                format!("{seed}{qualified}")
            };
            if index.workspace_command_exists(&mapped) && !out.contains(&mapped) {
                out.push(mapped);
            }
        }
        if out.is_empty() {
            // No seeded twin materialised (stale index edge): the plain
            // name is still the best identity.
            out.push(qualified);
        }
        out
    }

    async fn scan_workspace_folders(&self) {
        let folders = self.workspace_folder_urls().await;
        let roots: Vec<PathBuf> = folders
            .iter()
            .filter_map(|f| f.to_file_path().map(std::borrow::Cow::into_owned))
            .collect();
        // Note: we deliberately do NOT early-return when `roots.is_empty()`.
        // The package resolver must still be (re)built from the editor library
        // paths + `TCLLIBPATH` + discovered installations so single-file /
        // no-folder sessions resolve `package require` against the user's picked
        // "Select Tcl Installation" library paths.  Only the per-folder
        // workspace *indexing* (collecting/analysing on-disk files) is skipped
        // below when there are no roots.
        // Snapshot the dialect-resolution inputs and the set of
        // open documents so the blocking worker can run without
        // touching any async mutex.
        let open: HashSet<Uri> = self.documents.lock().await.keys().cloned().collect();
        let folder_dialects = self.folder_dialects.lock().await.clone();
        let default_dialect = self.default_dialect.lock().await.clone();
        // Inputs for the package-database `auto_path`: the editor's
        // `tclLsp.libraryPaths` (config.ini / .tcl-lsp.ini layers are read from
        // disk in the worker) and the discovered-installation cache.  Per-folder
        // `libraryPaths` overrides are unioned in so a folder's configured
        // package directories contribute to the shared package database (the
        // database is additive — more known packages only refines W120).
        let mut editor_library_paths = self.editor_library_paths.lock().await.clone();
        {
            let folder_configs = self.folder_configs.lock().await;
            for (_, fc) in folder_configs.iter() {
                if let Some(paths) = &fc.library_paths {
                    for p in paths {
                        if !editor_library_paths.contains(p) {
                            editor_library_paths.push(p.clone());
                        }
                    }
                }
            }
        }
        let discovered_cell = Arc::clone(&self.discovered_tcl);

        let resolver_roots = roots.clone();
        let analysed = tokio::task::spawn_blocking(move || {
            // Build the package database: the workspace trees plus the resolved
            // `auto_path` (editor + config-file `libraryPaths`, discovered Tcl
            // installations, and `TCLLIBPATH`). Discovery is cached for the
            // session.
            let discovered = discovered_cell.get_or_init(|| {
                core_tcl_install::discover(&core_tcl_install::default_search_bases())
            });
            let resolver = build_package_resolver(
                &resolver_roots,
                &editor_library_paths,
                discovered,
                WORKSPACE_SCAN_DIR_CAP,
            );

            let mut files: Vec<PathBuf> = Vec::new();
            for root in &roots {
                collect_tcl_files(root, WORKSPACE_SCAN_FILE_CAP, &mut files);
            }
            // Carry the source text + dialect alongside the analysis so the salsa
            // db (cross-file diagnostics) can index the same disk-backed files.
            let mut out: Vec<(Uri, String, String, AnalysisResult)> = Vec::new();
            for path in files {
                let Some(uri) = Uri::from_file_path(&path) else {
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
                out.push((uri, text, dialect, analysis));
            }
            (resolver, out)
        })
        .await
        .unwrap_or_else(|_| (PackageResolver::new(), Vec::new()));
        let (resolver, analysed) = analysed;

        // Publish the freshly-scanned package database for the diagnostics
        // worker, then merge the per-file analysis into the index + salsa db.
        *self.package_resolver.write().await = resolver;
        // The package database changed: drop the library files the autoload
        // tier (M8) merged under the previous database.  A stale entry would
        // keep answering for a library no longer on the resolved `auto_path`;
        // dropping is cheap and the next query re-merges on demand.
        let stale_library_uris: Vec<String> =
            self.autoloaded_library_uris.lock().await.drain().collect();
        if !stale_library_uris.is_empty() {
            let mut index = self.workspace_index.write().await;
            for uri in &stale_library_uris {
                index.remove_document(uri);
            }
        }
        self.merge_workspace_scan_results(&analysed).await;
        // Re-home sourced documents under their source-site namespaces (M9).
        self.refresh_source_rehoming().await;
        // Now that the `Project` covers the whole workspace, warm the salsa
        // per-file analysis in parallel (#844 Gap 3) so the first cross-file /
        // enriched-token query finds cache hits instead of a serial cold walk.
        self.spawn_workspace_warm();
    }

    /// Merge disk-backed workspace scan results into the shared index **and** the
    /// salsa `Project`, so cross-file diagnostics resolve against the whole
    /// workspace (not just open documents), matching the other cross-document
    /// features.
    ///
    /// The blocking scan snapshots open documents before it starts, but
    /// an editor can open a file while the scan is still running. Recheck
    /// the live document map at publication time so stale on-disk
    /// analysis cannot overwrite the open-buffer entry in either store.
    async fn merge_workspace_scan_results(
        &self,
        analysed: &[(Uri, String, String, AnalysisResult)],
    ) {
        // Hold `documents` across the open-set read, the salsa-db batch, and the
        // index merge (the `documents` → db → `workspace_index` order established
        // by `did_open`) so a file opening mid-merge can't have its live buffer
        // overwritten by this disk-backed scan result in either store.
        let docs = self.documents.lock().await;
        let open: HashSet<String> = docs.keys().map(|u| u.as_str().to_owned()).collect();
        // Salsa db first (db → db_files → db_project), before the workspace_index
        // lock, to preserve the global lock order.
        let db_entries: Vec<(Uri, String, String)> = analysed
            .iter()
            .filter(|(uri, _, _, _)| !open.contains(uri.as_str()))
            .map(|(uri, text, dialect, _)| (uri.clone(), text.clone(), dialect.clone()))
            .collect();
        self.db_set_sources_batch(&db_entries).await;
        let mut index = self.workspace_index.write().await;
        for (uri, _, _, analysis) in analysed {
            if open.contains(uri.as_str()) {
                continue;
            }
            index.remove_document(uri.as_str());
            index.add_document(uri.as_str(), analysis);
        }
        drop(index);
        // Every scanned document is now indexed standalone: reset the M9
        // applied-seed record so the next reconciliation re-applies the
        // source-site views.
        self.rehomed_source_seeds.lock().await.clear();
    }

    /// Kick off a detached, concurrency-bounded parallel **warm** of the salsa
    /// per-file analysis for every project file (#844 Gap 3).
    ///
    /// On a cold workspace the first enriched `semantic_tokens_project` otherwise
    /// serially analyses every file inside `project_class_index` /
    /// `project_proc_var_index`.  Pre-populating the memoised
    /// `file_analysis_incremental` across the blocking pool collapses that serial
    /// cold walk to a parallel one, so the tracked query's loop is all cache hits
    /// by the time the enriched token / cross-file diagnostics query runs — the
    /// enrichment side's analogue of the diagnostics deep pass's `join!`.
    ///
    /// Purely an optimisation, and safe by construction:
    /// - **Correctness**: it only primes the salsa cache; a concurrent real read
    ///   of the same `(file, config)` dedups against it (salsa blocks the second
    ///   requester and shares the memoised result — never a double analysis or a
    ///   divergent one), so results are identical to the serial walk.
    /// - **Never blocks a request**: fire-and-forget on a detached task.
    /// - **Never stalls an edit**: at most `WORKSPACE_WARM_MAX_CONCURRENCY`
    ///   snapshots are live at once (each warm clones its snapshot only after
    ///   acquiring a permit and drops it as soon as its analysis returns), so a
    ///   concurrent `set_text` waits on at most that many in-flight reads — and
    ///   each is the cancellable per-item query, so `set_text` cancels them at a
    ///   per-item boundary rather than waiting them out. A fresh warm aborts the
    ///   previous one (`warm_task`), so overlapping scans (initialize / folder-add
    ///   / config-change) keep that bound *global*, not merely per-warm, and drop
    ///   the redundant re-walk.
    ///
    /// Warms under every distinct config a request could resolve to — the global
    /// `db_config` plus each folder-scoped override in `folder_db_configs` —
    /// because `project_class_index` / `project_proc_var_index` apply a single
    /// `resolved_db_config(uri)` uniformly to *every* project file. A
    /// global-config-only warm would therefore miss the whole-project enrichment
    /// loop for every file the moment any folder sets an override (not just the
    /// overridden folder's files), since the loop keys `file_analysis_incremental`
    /// on that one resolved config. Deduped, so a single-config workspace (the
    /// common case) still warms each file exactly once; for the handful of override
    /// folders a real workspace has, the extra `files × configs` primes are cheap
    /// salsa cache fills.
    fn spawn_workspace_warm(&self) {
        let db = Arc::clone(&self.db);
        let db_project = Arc::clone(&self.db_project);
        let db_config = Arc::clone(&self.db_config);
        let folder_db_configs = Arc::clone(&self.folder_db_configs);
        let task = tokio::spawn(async move {
            let Some(project) = *db_project.lock().await else {
                return;
            };
            // Every config a request could resolve to: the global one plus each
            // folder override, deduped (a single-config workspace warms once).
            let mut configs = vec![*db_config.lock().await];
            for (_, folder_config) in folder_db_configs.lock().await.iter() {
                if !configs.contains(folder_config) {
                    configs.push(*folder_config);
                }
            }
            let files: Vec<tcl_lsp_db::SourceFile> = {
                let snapshot = db.lock().await.clone();
                project.files(&snapshot).clone()
            };
            if files.is_empty() {
                return;
            }
            let concurrency = std::thread::available_parallelism()
                .map_or(4, std::num::NonZeroUsize::get)
                .min(WORKSPACE_WARM_MAX_CONCURRENCY);
            let permits = Arc::new(Semaphore::new(concurrency));
            let mut warms = tokio::task::JoinSet::new();
            for config in configs {
                for file in files.iter().copied() {
                    // Acquire the permit *before* spawning so at most `concurrency`
                    // warm tasks (and thus snapshots) exist at once: the loop
                    // backpressures on a very large workspace rather than parking one
                    // pending task per item on the semaphore. `acquire_owned` moves
                    // the permit into the task, which holds it until its analysis
                    // returns.
                    let Ok(permit) = Arc::clone(&permits).acquire_owned().await else {
                        break;
                    };
                    let db = Arc::clone(&db);
                    warms.spawn(async move {
                        let _permit = permit;
                        // Clone the snapshot only after the permit is held, so
                        // `set_text` never waits on more than `concurrency` in-flight
                        // reads.
                        let snapshot = db.lock().await.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            let _ = salsa::Cancelled::catch(|| {
                                tcl_lsp_db::file_analysis_incremental(&snapshot, file, config)
                            });
                        })
                        .await;
                    });
                }
            }
            while warms.join_next().await.is_some() {}
        });
        // Supersede any still-running warm so overlapping scans (initialize /
        // folder-add / config-change) can't stack their snapshots or redundantly
        // re-walk the workspace. Aborting at the per-item await boundaries (with
        // the existing `Cancelled::catch`) keeps the global snapshot bound at
        // `WORKSPACE_WARM_MAX_CONCURRENCY`.
        if let Ok(mut guard) = self.warm_task.lock()
            && let Some(prev) = guard.replace(task.abort_handle())
        {
            prev.abort();
        }
    }

    /// Race the memoised CU + analysis reads for `uri` against
    /// [`SEMANTIC_TOKENS_FAST_PATH_BUDGET`] for a viewport (`range`) request.
    ///
    /// On a warm/small document both reads land well inside the budget and the
    /// caller colours the viewport with the enriched tier (`pending` is `None`);
    /// on a cold/large one the budget wins and the caller serves the cheap
    /// segmenter+registry-only tier immediately (`cached_cu`/`cached_analysis`
    /// both `None`) rather than blocking the viewport on a whole-file analysis
    /// (issue #829). The reads are taken as `JoinHandle`s so the ones the budget
    /// drops are **not** lost: on timeout they ride out to the detached
    /// convergence continuation (#844 Gap 4) via `pending`, which keeps awaiting
    /// the enriched unit/analysis and pushes a coalesced
    /// `workspace/semanticTokens/refresh` once the enriched viewport genuinely
    /// differs from the coarse tier served — the range analogue of
    /// `semantic_tokens_full`'s converge-later behaviour.
    async fn race_range_enriched_reads(
        &self,
        uri: &Uri,
    ) -> (
        Option<Arc<tcl_compiler::compilation_unit::CompilationUnit>>,
        Option<Arc<tcl_compiler::analyser::AnalysisResult>>,
        Option<RangeConvergencePending>,
    ) {
        // Normally both handles are `Some` (indexed document) or `None` (unindexed
        // buffer → coarse only, nothing to converge to). They are read under
        // *separate* `db_files` locks, though, so a concurrent
        // `did_close`/`db_remove_source` between the two can leave one `Some` and
        // the other `None`; the `_ => None` arm then degrades to coarse-only
        // (`pending = None`) and the orphaned read detaches but self-cancels on the
        // same `Project`-input bump.
        let handles = match (
            self.db_compilation_unit_handle(uri).await,
            self.db_file_analysis_handle(uri).await,
        ) {
            (Some(cu), Some(analysis)) => Some((cu, analysis)),
            _ => None,
        };
        match handles {
            Some((mut cu_handle, mut analysis_handle)) => {
                // Race the enriched reads against the budget, capturing each result
                // into an outer slot *as it lands* so a partial completion survives
                // the dropped race future. The two reads are polled concurrently
                // (`join!`), so each slot fills independently of the other's landing
                // order. A slot is `Some` iff its handle was awaited to completion —
                // so a `None` slot means the handle is still un-consumed (the
                // continuation awaits it), while a `Some` slot is reused directly,
                // never re-awaiting a spent handle. Without the slots, a read that
                // landed within budget while its sibling overran would be consumed
                // and then lost when the timeout drops the race future, and its spent
                // handle re-awaited in the continuation (a re-poll of a completed
                // `JoinHandle`) — losing the enrichment and the convergence refresh.
                let mut cu_slot: Option<
                    Option<Arc<tcl_compiler::compilation_unit::CompilationUnit>>,
                > = None;
                let mut analysis_slot: Option<Option<Arc<tcl_compiler::analyser::AnalysisResult>>> =
                    None;
                let both_ready = tokio::select! {
                    biased;
                    () = async {
                        tokio::join!(
                            async { cu_slot = Some((&mut cu_handle).await.ok().flatten()) },
                            async {
                                analysis_slot = Some((&mut analysis_handle).await.ok().flatten());
                            },
                        );
                    } => true,
                    () = tokio::time::sleep(SEMANTIC_TOKENS_FAST_PATH_BUDGET) => false,
                };
                if both_ready {
                    // Both landed inside the budget — serve the enriched viewport.
                    (cu_slot.flatten(), analysis_slot.flatten(), None)
                } else {
                    // Budget won: serve coarse, handing whatever partial result
                    // landed plus the still-running (un-consumed) reads to the
                    // convergence continuation.
                    (
                        None,
                        None,
                        Some((cu_slot, analysis_slot, cu_handle, analysis_handle)),
                    )
                }
            }
            None => (None, None, None),
        }
    }

    /// Detach the #844 Gap 4 convergence continuation for a range request served
    /// the coarse tier: await the enriched CU / analysis (reusing any that landed
    /// within the budget via its slot, never re-awaiting a spent handle),
    /// recompute the viewport-filtered range, and fire a coalesced
    /// `workspace/semanticTokens/refresh` if it genuinely differs from the coarse
    /// `served` stream. The range stream is never in `last_semantic_tokens`, so
    /// the diff is against the exact `served` bytes rather than the token cache.
    fn spawn_range_convergence(
        &self,
        inputs: RangeConvergenceInputs,
        pending: RangeConvergencePending,
    ) {
        let RangeConvergenceInputs {
            uri,
            served,
            registry,
            text,
            dialect,
            range,
        } = inputs;
        let (cu_slot, analysis_slot, cu_handle, analysis_handle) = pending;
        let refresh_ctx = SemanticTokensRefreshCtx {
            client: self.client.clone(),
            last_semantic_tokens: Arc::clone(&self.last_semantic_tokens),
            refresh_pending: Arc::clone(&self.semantic_tokens_refresh_pending),
        };
        tokio::spawn(async move {
            // Reuse a result that already landed within the budget; otherwise await
            // the still-running read. A `None` slot means the handle was never
            // awaited to completion, so awaiting it here is safe — never a re-poll
            // of a spent handle.
            let cu = match cu_slot {
                Some(cu) => cu,
                None => cu_handle.await.ok().flatten(),
            };
            let analysis = match analysis_slot {
                Some(analysis) => analysis,
                None => analysis_handle.await.ok().flatten(),
            };
            // Both `None` ⇒ the reads were cancelled by a concurrent edit, which
            // schedules its own token refresh — nothing to converge. Every other
            // path makes the coarse-vs-enriched decision below.
            let refreshed = if cu.is_none() && analysis.is_none() {
                false
            } else {
                let enriched = tokio::task::spawn_blocking(move || {
                    core_semantic_tokens::range_with_cu_and_analysis(
                        &text,
                        &dialect,
                        range,
                        &registry,
                        cu.as_deref(),
                        analysis.as_deref(),
                    )
                    .data
                })
                .await;
                match enriched {
                    Ok(enriched) if enriched != served => {
                        refresh_ctx.request_refresh_coalesced();
                        true
                    }
                    _ => false,
                }
            };
            // The convergence decision is now made (a refresh was asked for, or the
            // enriched viewport matched the coarse tier we served, or the reads were
            // cancelled). Emit the settled marker so a waiter can key on this exact
            // line + `uri=` to know the compare finished — the message-passing
            // signal that lets a test assert on the *absence* of a
            // `workspace/semanticTokens/refresh` deterministically instead of racing
            // a fixed sleep. `refresh=` records the outcome for observability.
            refresh_ctx
                .client
                .log_message(
                    MessageType::LOG,
                    format!(
                        "[timing] semantic_tokens.range_convergence.settled \
                         (uri={uri}, refresh={refreshed})"
                    ),
                )
                .await;
        });
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        self.apply_workspace_folders(&params).await;
        self.apply_initialization_options(&params).await;
        // Push is the sole diagnostics channel by default: pull is opt-in and
        // the server does not advertise `diagnosticProvider` (see
        // `build_server_capabilities`).  A client that *supports* pull will not
        // actually pull unless the server advertises it, so the #721
        // "stop pushing when the client pulls" suppression must stay OFF here —
        // otherwise a pull-capable client (VS Code advertises the capability)
        // gets neither push (suppressed) nor pull (unadvertised), i.e. zero
        // diagnostics.  When pull is opted back in this flag is set from the
        // client's actual `textDocument/diagnostic` support via
        // `client_supports_pull_diagnostics(&params)`.
        let _ = client_supports_pull_diagnostics(&params);
        self.client_supports_pull_diagnostics
            .store(false, std::sync::atomic::Ordering::Relaxed);
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
                // Protocol identity must match the editor's
                // expectations ("tcl-lsp"), not the crate/binary name.
                name: "tcl-lsp".to_owned(),
                // The release version from the tag, not the manifest's 0.1.0.
                version: Some(tcl_version::VERSION.to_owned()),
            }),
            ..Default::default()
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
        // Type-hierarchy support is advertised statically by the
        // `inject_type_hierarchy_provider` service layer (see `main.rs`) — a
        // dynamic `client/registerCapability` here would both duplicate it and
        // fail to surface in the client's `initializeResult.capabilities`.
        // Seed the cross-document index with on-disk project files
        // the editor hasn't opened yet.
        self.scan_workspace_folders().await;
        // `initialized` fires concurrently with any `textDocument/didOpen` the
        // client already queued for its restored tabs (`buffer_unordered`, see
        // `edit_serialize`'s doc comment) — a document's first debounced
        // diagnostics run (50ms) routinely completes before this scan does on
        // anything but a trivial workspace. The always-on W120/W123 workspace
        // refinement (`refine_workspace_w120`/`refine_workspace_w123`) reads
        // `workspace_index` / `package_resolver`, which this scan is what
        // populates — so a document opened at startup can publish a diagnostic
        // (e.g. a false W120 for a command whose `package require` lives in an
        // unopened `source` ancestor) computed against the still-empty stores,
        // and nothing republishes it once the scan finishes. Reschedule every
        // open document now so any diagnostics published before the scan
        // completed are refreshed against the now-populated workspace state —
        // the same pattern `did_change_watched_files` already uses after its
        // own `scan_workspace_folders` call on a config change.
        self.reschedule_all_open_documents().await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        // Apply document-sync mutations in arrival order (see `EditOrder`). The
        // ticket must be drawn before the first await.
        let ticket = self.edit_order.take_ticket();
        let _turn = self.edit_order.wait_turn(ticket).await;
        let dialect = self
            .dialect_for_open(
                &params.text_document.uri,
                &params.text_document.language_id,
                &params.text_document.text,
            )
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
                .write()
                .await
                .remove_document(uri.as_str());
            drop(docs);
        }
        // Opening a document is a config-context boundary, not an edit. A closed
        // URI keeps its `DiagSlot` (only the live document + index entry are
        // dropped on close), so `slot.latest_inputs` can carry the toggle values
        // captured when the file was last analysed — a stale
        // `features.diagnostics` / dialect / `optimiser.enabled` / disabled-code
        // set if any of those changed while the file was closed. The
        // reuse-cached-inputs fast path is valid only on the edit path
        // (`did_change`), where config genuinely has not changed. Force a fresh
        // input resolve so the open reads the *current* toggles; otherwise
        // reopening a file after the diagnostics master switch was turned off
        // republishes its pre-toggle squiggles from the stale inputs (#104).
        self.reschedule_diagnostics(uri, dialect_for_diags).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // Incremental sync: each content change is either a full-document
        // replacement (`range == None`) or a ranged edit applied to the
        // current text. Apply them in order. (The re-analysis below is
        // still whole-document and is not bounded to `reparse_window`,
        // though the primitives exist in `tcl-lexer`.)
        // Apply document-sync mutations in arrival order (see `EditOrder`) —
        // otherwise two rapid incremental edits splice out of order and corrupt
        // the buffer. The ticket must be drawn before the first await.
        let ticket = self.edit_order.take_ticket();
        let _turn = self.edit_order.wait_turn(ticket).await;
        let uri = params.text_document.uri.clone();
        if params.content_changes.is_empty() {
            return;
        }
        let change_version = params.text_document.version;
        let (dialect, language_id) = {
            let mut docs = self.documents.lock().await;
            // tower-lsp-server 0.23 dispatches notification handlers
            // concurrently (`buffer_unordered(4)`), so a `didChange` can be
            // processed before its `didOpen` or after its `didClose`. Only
            // mutate an already-open document: resurrecting a closed one (or
            // splicing a ranged edit against a phantom empty buffer that
            // `didOpen` then silently overwrites at the wrong version) would
            // corrupt state and keep publishing diagnostics for a closed
            // document (RUST_ISSUE_099). A dropped change is safe — `didOpen`
            // carries the authoritative full text, and a post-close change has
            // nothing to apply to.
            let Some(entry) = docs.get_mut(&uri) else {
                return;
            };
            let mut text = std::mem::take(&mut entry.text);
            // Patch the persisted `LineIndex` alongside each splice instead of
            // rebuilding it per edit / per position lookup.
            // Take it out, patch through the edit sequence, put it back.
            let mut index = std::mem::replace(&mut entry.line_index, tcl_lexer::LineIndex::new(""));
            for change in &params.content_changes {
                text = apply_content_change_indexed(&text, change.range, &change.text, &mut index);
            }
            entry.text = text.clone();
            entry.line_index = index;
            entry.bump_revision(change_version);
            let dialect = entry.dialect.clone();
            let language_id = entry.language_id.clone();
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
                .write()
                .await
                .remove_document(uri.as_str());
            drop(docs);
            (dialect, language_id)
        };
        self.schedule_diagnostics(uri.clone(), dialect.clone())
            .await;
        // Re-resolve in-source dialect hints (a `# tcl-dialect:` directive, a
        // `#!…tclshX.Y` shebang, or `package require Tcl X.Y`) after the edit —
        // adding or changing one in an already-open buffer must take effect
        // without reopening.  Only a generically-opened `tcl` buffer consults
        // the source (an explicit versioned / non-Tcl languageId is fixed), and
        // only the source hint is edit-sensitive, so this is the sole case that
        // can change.  When it does, commit the new dialect and re-analyse.
        if language_id == "tcl" {
            // Read the open buffer directly: `read_document` waits for this
            // handler's own turn to finish (see `edits_settled`), so routing
            // through it here would self-deadlock. The document is open — a
            // closed one returned above.
            let text = match self.documents.lock().await.get(&uri) {
                Some(entry) => entry.text.clone(),
                None => return,
            };
            let new_dialect = self.dialect_for_open(&uri, &language_id, &text).await;
            if new_dialect != dialect {
                {
                    let mut docs = self.documents.lock().await;
                    let Some(entry) = docs.get_mut(&uri) else {
                        return;
                    };
                    entry.dialect.clone_from(&new_dialect);
                    let text = entry.text.clone();
                    self.db_set_source(&uri, text, new_dialect.clone()).await;
                    self.workspace_index
                        .write()
                        .await
                        .remove_document(uri.as_str());
                }
                self.reschedule_diagnostics(uri, new_dialect).await;
            }
        }
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
        // was opened with until reopened. This snapshots each open
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
        if let Some(overrides) = settings_severity_overrides(&params.settings) {
            *self.severity_overrides.lock().await = overrides;
        }
        // VS Code (and the e2e harness) push an empty/partial payload as a
        // signal to re-pull the full resolved config via
        // `workspace/configuration`.  Always re-pull so `features.*`, the
        // optimiser switch, and the analyser knobs reflect the latest editor
        // settings — the inline `params.settings` handling above covers the
        // flat MCP-bridge shape that carries the values directly.
        self.pull_and_apply_config().await;
        // The re-pull may have flipped `features.diagnostics`,
        // `optimiser.enabled`, or the disabled-diagnostics set — none of which
        // changes a document's dialect, so `reresolve_open_document_dialects`
        // above skips them.  Re-run diagnostics for every open buffer so the
        // new toggles take effect immediately (clearing squiggles when the
        // master switch goes off, dropping O-codes when the optimiser goes off)
        // rather than lingering until the next keystroke.
        self.reschedule_all_open_documents().await;
        // The same toggles govern closed files that still carry a badge (#865):
        // a master-switch-off must clear their squiggles too, and a disabled-code
        // change must re-lint them — the open-document reschedule alone would
        // leave a closed file's badge frozen at its pre-toggle set.
        self.reschedule_closed_file_diagnostics().await;
        // On-demand providers cache their last result client-side and only
        // re-request on a document edit — a bare config change (e.g. toggling
        // `features.folding` off) would otherwise leave stale folding ranges /
        // code lenses on screen.  Ask the client to refresh them.  Best-effort:
        // a client without refresh support rejects the request, which is
        // harmless.  `foldingRange/refresh` (LSP 3.18) is not in ls-types, so it
        // is sent through a locally-defined request type.
        let _ = self
            .client
            .send_request::<FoldingRangeRefreshRequest>(())
            .await;
        let _ = self.client.code_lens_refresh().await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        // Apply document-sync mutations in arrival order (see `EditOrder`) so a
        // close can't be reordered ahead of an in-flight edit. The ticket must be
        // drawn before the first await.
        let ticket = self.edit_order.take_ticket();
        let _turn = self.edit_order.wait_turn(ticket).await;
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
                .write()
                .await
                .remove_document(uri.as_str());
            drop(docs);
        }
        // Drop the cached semantic-token baseline so a reopened document starts
        // from a fresh `full` rather than diffing against a stale stream.
        self.last_semantic_tokens.lock().await.remove(uri);
        // Re-index the file from disk rather than dropping it: the file still
        // exists on disk and was (or would be) part of the on-disk index, so
        // cross-document definition / references / rename / call-hierarchy — and
        // cross-file diagnostics (the salsa `Project`) — must keep seeing it after
        // the editor closes the buffer.  `scan_workspace_folders` only runs at
        // `initialized`, so dropping it here would make the file vanish until
        // restart.  The helper rechecks that this URI is still closed, then
        // refreshes both the salsa db source and the disk-backed index entry (or
        // drops both when the URI is not a readable file).
        self.reindex_index_from_disk(uri).await;
        // #865: keep the file's Problems / File-Explorer badge after its editor
        // tab closes.  Rather than the old unconditional empty publish — which
        // made a closed-but-on-disk workspace file lose its diagnostics until it
        // was reopened — republish its on-disk diagnostics through the same
        // pipeline the open path uses (so the set is identical, kept accurate
        // against disk).  For a URI with no readable on-disk source (untitled
        // buffer, deleted file) this clears the squiggles and drops the pull-cache
        // entry, exactly as before.  The reindex above primed the salsa source it
        // reads; both re-check the document is still closed under the `documents`
        // lock, so a racing `did_open` can never have a stale closed publish land
        // on a freshly reopened buffer (`RUST_ISSUE_098`).
        self.publish_closed_file_diagnostics(uri).await;
        // #865 sync guarantee: the VS Code e2e harness (`waitForDeepDiagnostics`)
        // keys on the `[timing] deep diagnostics (uri=…)` marker to know the
        // close's republish settled before it asserts the retained badge. That
        // marker is emitted *inside* the currency-gated publish, so a close run
        // legitimately superseded by a racing config / watched-file refresh —
        // which bumps the per-URI closed generation — or one that settles an
        // empty file emits none, and the harness times out even though the badge
        // settled correctly (the source of the `test-ext` flakiness). Emit an
        // unconditional completion marker here, ordered after the republish above
        // has delivered its publish, so the signal is reliable regardless of the
        // internal delivery outcome. Notifications are ordered on the client, so
        // any publish sent above is applied before this marker is observed; the
        // count is read back from the pull cache the publish primed. A duplicate
        // of the in-pipeline marker on the common (current) path is harmless —
        // the harness matches on presence, never a count.
        let uri_str = uri.to_string();
        let diag_count = self
            .pull_diag_cache
            .lock()
            .await
            .get(uri)
            .map_or(0, |entry| entry.diagnostics.len());
        self.client
            .log_message(
                MessageType::LOG,
                format!("[timing] deep diagnostics 0ms (uri={uri_str}, diags={diag_count})"),
            )
            .await;
    }

    async fn did_change_workspace_folders(&self, params: DidChangeWorkspaceFoldersParams) {
        // Update the tracked folder set, then reconcile the cross-document
        // index and configuration so multi-root behaviour is not frozen at the
        // `initialize` snapshot.  Drops removed-folder state, loads/scans
        // added folders, and re-pulls config.
        let removed: Vec<Uri> = params.event.removed.iter().map(|f| f.uri.clone()).collect();
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
        // A folder add/remove shifts the salsa `Project`, `workspace_index` and
        // `package_resolver` (the cross-file resolution domain *and* the
        // always-on W120/W123 workspace-refinement inputs) without an open
        // document's own edit, so reschedule every open document — otherwise a
        // push-diagnostic client keeps a stale suppressed-W123 / cross-file
        // arity, or a false W120 the refinement would now resolve, from a
        // now-removed (or newly-added) folder. Unconditional
        // `reschedule_all_open_documents` (not a narrower `crossFileResolution`-only
        // helper): the W120/W123 refinement runs for every document regardless
        // of that opt-in toggle.
        if !removed.is_empty() || !params.event.added.is_empty() {
            self.reschedule_all_open_documents().await;
        }
        // Per-folder config may differ between roots; re-pull so the resolved
        // settings reflect the new folder set.
        self.pull_and_apply_config().await;
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        // External (non-editor) file changes — `git checkout`, a generated
        // file, a deletion — must refresh the cross-document index so
        // definition / references / rename / call-hierarchy keep seeing the
        // project's true on-disk state between restarts.
        let mut domain_changed = false;
        let mut config_changed = false;
        // Closed files that already carry a badge (a pull-cache entry) and whose
        // on-disk change must refresh (#865) or clear that badge.  Collected here
        // and applied after the whole batch has updated the resolution domain, so
        // each refresh analyses against the final on-disk state — never a
        // half-applied one when several files change together.
        let mut closed_badge_refresh: Vec<Uri> = Vec::new();
        let mut closed_badge_clear: Vec<Uri> = Vec::new();
        for change in params.changes {
            // A project `.tcl-lsp.ini` (or user `config.ini`) edit re-applies the
            // layered config — a live-reload for these files.
            if is_config_file(&change.uri) {
                config_changed = true;
                continue;
            }
            // Files the editor has open are driven by did_open/did_change; their
            // unsaved buffer must not be clobbered by the on-disk copy.
            // `reindex_index_from_disk` re-checks this under the lock as well.
            if self.documents.lock().await.contains_key(&change.uri) {
                continue;
            }
            // Only a file that already has a badge (was opened, so it has a
            // pull-cache entry) has its diagnostics refreshed/cleared here — a
            // never-opened file that changes on disk gains no surprise badge.
            let had_badge = self.pull_diag_cache.lock().await.contains_key(&change.uri);
            if change.typ == FileChangeType::DELETED {
                self.workspace_index
                    .write()
                    .await
                    .remove_document(change.uri.as_str());
                // Drop it from the salsa `Project` too, so a deleted file's procs
                // stop suppressing W123 / driving the arity error cross-file.
                self.db_remove_source(&change.uri).await;
                if had_badge {
                    closed_badge_clear.push(change.uri.clone());
                }
            } else {
                // CREATED or CHANGED: re-analyse from disk (a Tcl source file)
                // or drop it if it no longer reads as one.
                self.reindex_index_from_disk(&change.uri).await;
                if had_badge {
                    closed_badge_refresh.push(change.uri.clone());
                }
            }
            domain_changed = true;
        }
        // Refresh/clear the badges of the closed files that changed on disk, now
        // the resolution domain has settled (#865).
        for uri in &closed_badge_clear {
            self.clear_closed_diagnostics(uri).await;
        }
        for uri in &closed_badge_refresh {
            self.publish_closed_file_diagnostics(uri).await;
        }
        // A watched (non-open) file's create/change/delete shifts the cross-file
        // resolution domain *and* `workspace_index` (the always-on W120/W123
        // workspace-refinement input), but no open document's own edit triggered
        // it — so a push-diagnostic client would keep stale cross-file results (a
        // suppressed W123, an arity error sourced from the now-changed file, or a
        // false W120 the refinement would now resolve via a `source` ancestor)
        // until the caller is next edited. Reschedule every open document — not
        // just `crossFileResolution`-enabled ones, since the W120/W123 refinement
        // is unconditional — so both the cross-file pass and the refinement re-run
        // against the new domain.
        if domain_changed {
            self.reschedule_all_open_documents().await;
        }
        // Re-read config.ini / .tcl-lsp.ini and re-apply with full precedence,
        // then rebuild the package database (libraryPaths may have changed) and
        // re-run open documents so the new settings take effect immediately.
        if config_changed {
            self.pull_and_apply_config().await;
            self.scan_workspace_folders().await;
            self.reschedule_all_open_documents().await;
            // Closed files that carry a badge follow the same reconfigured
            // disabled-code / master-switch state (#865).
            self.reschedule_closed_file_diagnostics().await;
        }
    }

    async fn will_save_wait_until(
        &self,
        params: WillSaveTextDocumentParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        // Opt-in format-on-save, gated behind
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
        // Honour the user's resolved `tclLsp.formatting` settings, exactly as
        // `formatting()` does — otherwise format-on-save re-indents with
        // defaults and fights an explicit Format Document (RUST_ISSUE_101).
        // `WillSaveTextDocumentParams` carries no `FormattingOptions`, so the
        // settings object is the sole source; the resolved formatter width is
        // then applied on top, preserving prior behaviour.
        let formatting = self.resolved_formatting(&params.text_document.uri).await;
        let mut config = core_formatting::FormatterConfig {
            lexer_config: tcl_lexer::LexerConfig::for_dialect(&doc.dialect),
            ..core_formatting::FormatterConfig::default()
        };
        if let Some(obj) = formatting.as_object() {
            apply_formatting_object(obj, &mut config);
        }
        config.max_line_length =
            self.resolved_line_length(&params.text_document.uri).await as usize;
        config.indent_size = config.indent_size.max(1);
        // Run on a worker so a formatter panic is contained as a JSON-RPC
        // error rather than unwinding the event loop, matching every sibling
        // handler.
        let text = doc.text.clone();
        let edits = tokio::task::spawn_blocking(move || {
            core_formatting::formatting_with(&text, &config, &registry)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("will_save_wait_until worker panicked: {err}").into(),
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

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        if !self
            .feature_enabled("folding", &params.text_document.uri)
            .await
        {
            // Return an authoritative *empty* set, not `None`: a `None` result
            // makes VS Code fall back to its built-in indentation folding (so
            // the ranges reappear), whereas an empty list is honoured as "this
            // provider has no folding ranges", suppressing folding as the toggle
            // intends.
            return Ok(Some(Vec::new()));
        }
        let Some(doc) = self.read_document(&params.text_document.uri).await else {
            return Ok(None);
        };
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // Pure-CPU tokenise/segment work; run on a worker so a parser panic
        // is contained as a JSON-RPC error rather than unwinding the event
        // loop (defence in depth).
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
        // ever carries an empty `name`.
        // The two compute branches run pure-CPU on a worker so a parser
        // panic is contained as a JSON-RPC error.
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
        // Completion here only knows user procs / vars, not the
        // registry's command set.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // Share the cross-document index with the worker so it can
        // enumerate procs from sibling files.  Move a refcounted
        // handle in and take the read lock inside the blocking
        // closure: the heavy walk holds a shared read lock, letting
        // other readers proceed concurrently while only writers wait.
        let workspace_index = Arc::clone(&self.workspace_index);
        // Pure-CPU work; spawn_blocking off the LSP event loop.
        let items = tokio::task::spawn_blocking(move || {
            let workspace = workspace_index.blocking_read();
            core_completion::completions(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
                Some(&*workspace),
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
        if !self
            .feature_enabled(
                "declaration",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // For a `$var`, go-to-declaration resolves the
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
        if !self
            .feature_enabled(
                "typeDefinition",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // Type-definition jumps to the class that types
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
        if !self
            .feature_enabled(
                "implementation",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // Go-to-implementation is the TclOO subclass /
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
        // BIG-IP `.conf`/`.scf`: a reference is every identifier-bounded
        // occurrence of the TMSH path token at the cursor (tcl-bigip::refs),
        // a textual search the Tcl symbol analyser can't perform on a
        // non-Tcl config. Same-document only — cross-file references await a
        // workspace config index.
        if Self::is_bigip_dialect(&doc.dialect) {
            let text = doc.text.clone();
            let bigip_ranges = tokio::task::spawn_blocking(move || {
                tcl_bigip::refs::references_at(&text, pos.line, pos.character, include_decl)
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("bigip references worker panicked: {err}").into(),
                data: None,
            })?;
            if bigip_ranges.is_empty() {
                return Ok(None);
            }
            let locations = bigip_ranges
                .into_iter()
                .map(|r| Location {
                    uri: uri.clone(),
                    range: lift_bigip_range(r),
                })
                .collect();
            return Ok(Some(locations));
        }
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
        // Cross-document TclOO method sites: when the cursor names a method
        // (its declaration inside a class body, or an `$obj method` / `my
        // method` call), gather the method's sites across its override family in
        // sibling documents — the single-document provider above only sees this
        // file.  The target is resolved oracle-aware so a pure-consumer cursor
        // (`$obj`'s class defined in another file) still identifies the method.
        if let Some((seed_class, method, _access)) = self
            .resolve_method_target(&doc.text, &doc.dialect, &analysis, pos)
            .await
        {
            let method_cross = self
                .cross_file_method_references(&uri, &seed_class, &method, include_decl)
                .await;
            locations.extend(method_cross);
            // Pure-consumer documents (including this one) whose `$obj method`
            // sites neither the single-document provider nor the family pass
            // sees without the workspace class oracle.
            let consumer = self
                .cross_file_consumer_method_references(
                    &uri,
                    &doc.text,
                    &doc.dialect,
                    &seed_class,
                    &method,
                )
                .await;
            locations.extend(consumer);
        }
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
        if !self
            .feature_enabled(
                "documentHighlight",
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
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let entries = tokio::task::spawn_blocking(move || {
            // The kinded entry
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
        if !self
            .feature_enabled(
                "callHierarchy",
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
        if !self
            .feature_enabled("callHierarchy", &params.item.uri)
            .await
        {
            return Ok(None);
        }
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
        let mut tagged: Vec<(Uri, core_call_hierarchy::IncomingCall)> =
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
        if !self
            .feature_enabled("callHierarchy", &params.item.uri)
            .await
        {
            return Ok(None);
        }
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

    async fn prepare_type_hierarchy(
        &self,
        params: TypeHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
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
            core_type_hierarchy::prepare(&doc.text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("type_hierarchy worker panicked: {err}").into(),
            data: None,
        })?;
        if items.is_empty() {
            return Ok(None);
        }
        let lifted = items
            .into_iter()
            .map(|i| TypeHierarchyItem {
                name: i.name,
                kind: SymbolKind::CLASS,
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

    async fn supertypes(
        &self,
        params: TypeHierarchySupertypesParams,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
        self.type_hierarchy_walk(params.item, false).await
    }

    async fn subtypes(
        &self,
        params: TypeHierarchySubtypesParams,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
        self.type_hierarchy_walk(params.item, true).await
    }

    async fn linked_editing_range(
        &self,
        params: LinkedEditingRangeParams,
    ) -> jsonrpc::Result<Option<LinkedEditingRanges>> {
        if !self
            .feature_enabled(
                "linkedEditingRange",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
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
        // so unchanged documents cost nothing.
        let previous: HashMap<Uri, String> = params
            .previous_result_ids
            .into_iter()
            .map(|p| (p.uri, p.value))
            .collect();
        // Snapshot the cache (clone out) so we don't hold its lock while also
        // taking the `documents` lock for versions.
        let entries: Vec<(Uri, PullDiagEntry)> = self
            .pull_diag_cache
            .lock()
            .await
            .iter()
            .map(|(uri, entry)| (uri.clone(), entry.clone()))
            .collect();
        let versions: HashMap<Uri, Option<i64>> = {
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
        if !self
            .feature_enabled("semanticTokens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // `S-semantic-tokens-rich`: real classification, served from the
        // memoised `semantic_tokens` query (packed integer stream, 5 ints per
        // token `[deltaLine, deltaCol, length, type, modifiers]`).
        let core_data = self.semantic_tokens_core_data(&uri, &doc).await?;
        let result_id = next_semantic_tokens_id();
        // Remember this stream so a later `full/delta` with this `resultId` can
        // diff against it.
        self.last_semantic_tokens
            .lock()
            .await
            .insert(uri.clone(), (result_id.clone(), core_data.clone()));
        Ok(Some(SemanticTokensResult::Tokens(LspSemanticTokens {
            result_id: Some(result_id),
            data: lift_semantic_token_data(&core_data),
        })))
    }

    async fn semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> jsonrpc::Result<Option<SemanticTokensFullDeltaResult>> {
        if !self
            .feature_enabled("semanticTokens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let new_data = self.semantic_tokens_core_data(&uri, &doc).await?;
        let result_id = next_semantic_tokens_id();
        // Swap the cache to the freshly-served stream and grab the previous one
        // *only if* its `resultId` matches the client's `previousResultId` — a
        // stale / unknown id means we can't safely diff, so fall back to a full
        // stream.  Doing the take-and-replace under one lock keeps concurrent
        // requests for the same URI consistent.
        let baseline = {
            let mut cache = self.last_semantic_tokens.lock().await;
            let prev = cache
                .remove(&uri)
                .filter(|(id, _)| *id == params.previous_result_id);
            cache.insert(uri.clone(), (result_id.clone(), new_data.clone()));
            prev
        };
        if let Some((_, old_data)) = baseline {
            // Matching baseline: answer with the minimal token-aligned edit
            // (an empty edit list when nothing changed) rather than the whole
            // stream — the incremental path every `full/delta`-capable editor
            // benefits from.
            let edits = match core_semantic_tokens::diff(&old_data, &new_data) {
                Some(edit) => vec![SemanticTokensEdit {
                    start: edit.start,
                    delete_count: edit.delete_count,
                    data: Some(lift_semantic_token_data(&edit.data)),
                }],
                None => Vec::new(),
            };
            Ok(Some(SemanticTokensFullDeltaResult::TokensDelta(
                SemanticTokensDelta {
                    result_id: Some(result_id),
                    edits,
                },
            )))
        } else {
            Ok(Some(SemanticTokensFullDeltaResult::Tokens(
                LspSemanticTokens {
                    result_id: Some(result_id),
                    data: lift_semantic_token_data(&new_data),
                },
            )))
        }
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<SemanticTokensRangeResult>> {
        // Gate on the same `semanticTokens` toggle as `full` / `full_delta`, so
        // disabling semantic tokens also silences viewport (range) requests —
        // otherwise range-request clients keep rendering highlights the user
        // turned off (issue 174).
        if !self
            .feature_enabled("semanticTokens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
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
        if let Some(tokens) = self
            .non_tcl_range_tokens(&params.text_document.uri, &doc, core_range)
            .await
        {
            return Ok(Some(SemanticTokensRangeResult::Tokens(LspSemanticTokens {
                result_id: None,
                data: lift_semantic_token_data(&tokens.data),
            })));
        }
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let uri = &params.text_document.uri;
        // Race the memoised unit + analysis against the fast-path budget; on
        // timeout `pending` carries the still-running reads to the #844 Gap 4
        // convergence continuation below (see `race_range_enriched_reads`).
        let (cached_cu, cached_analysis, pending) = self.race_range_enriched_reads(uri).await;
        // Pure-CPU tokenisation on a worker so a parser panic is contained
        // as a JSON-RPC error.
        let (text, dialect) = (doc.text.clone(), doc.dialect.clone());
        let serve_registry = Arc::clone(&registry);
        let core_data = tokio::task::spawn_blocking(move || {
            core_semantic_tokens::range_with_cu_and_analysis(
                &text,
                &dialect,
                core_range,
                &serve_registry,
                cached_cu.as_deref(),
                cached_analysis.as_deref(),
            )
            .data
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("semantic_tokens_range worker panicked: {err}").into(),
            data: None,
        })?;

        // Convergence (#844 Gap 4): if we served the coarse tier because the
        // enriched reads overran the budget, detach a continuation that awaits
        // them and refreshes once the enriched viewport differs.
        if let Some(pending) = pending {
            self.spawn_range_convergence(
                RangeConvergenceInputs {
                    uri: uri.as_str().to_owned(),
                    served: core_data.clone(),
                    registry: Arc::clone(&registry),
                    text: doc.text.clone(),
                    dialect: doc.dialect.clone(),
                    range: core_range,
                },
                pending,
            );
        }

        Ok(Some(SemanticTokensRangeResult::Tokens(LspSemanticTokens {
            result_id: None,
            data: lift_semantic_token_data(&core_data),
        })))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> jsonrpc::Result<Option<WorkspaceSymbolResponse>> {
        // Workspace-wide query (no document URI): gate on the process-global
        // `workspaceSymbols` toggle.
        if !self
            .feature_toggles
            .lock()
            .await
            .is_enabled("workspaceSymbols")
        {
            return Ok(None);
        }
        // Walk every cached document and collect matching
        // symbols.  This iterates the document
        // store on the LSP loop (acquiring the mutex briefly).
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
                        // A tcltest case has no dedicated LSP kind; it lists as
                        // a function alongside real functions.
                        CoreWorkspaceSymbolKind::Function | CoreWorkspaceSymbolKind::Test => {
                            SymbolKind::FUNCTION
                        }
                        CoreWorkspaceSymbolKind::Class => SymbolKind::CLASS,
                        CoreWorkspaceSymbolKind::Method => SymbolKind::METHOD,
                        CoreWorkspaceSymbolKind::Constructor => SymbolKind::CONSTRUCTOR,
                        CoreWorkspaceSymbolKind::Constant => SymbolKind::CONSTANT,
                        CoreWorkspaceSymbolKind::Operator => SymbolKind::OPERATOR,
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
        Ok(Some(WorkspaceSymbolResponse::Flat(all)))
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
        // BIG-IP `.conf`/`.scf`: links are object references (iRule-body and
        // migrated TMSH property values) resolving to the target object's
        // stanza in the same document — a `uri#L<line>` fragment anchor. This
        // is a different engine from the Tcl `source`/`package require` link
        // scanner below.
        if Self::is_bigip_dialect(&doc.dialect) {
            let text = doc.text.clone();
            let bigip_links =
                tokio::task::spawn_blocking(move || tcl_bigip::links::document_links(&text))
                    .await
                    .map_err(|err| jsonrpc::Error {
                        code: jsonrpc::ErrorCode::InternalError,
                        message: format!("bigip document_link worker panicked: {err}").into(),
                        data: None,
                    })?;
            if bigip_links.is_empty() {
                return Ok(None);
            }
            let uri_str = uri.as_str();
            let lifted = bigip_links
                .into_iter()
                .map(|l| DocumentLink {
                    range: lift_bigip_range(l.range),
                    // Resolved → a `uri#L<1-based-line>` fragment the editor
                    // navigates to; unresolved → no target (hover-only).
                    target: l
                        .target_line
                        .and_then(|line| Uri::from_str(&format!("{uri_str}#L{}", line + 1)).ok()),
                    tooltip: Some(l.tooltip),
                    data: None,
                })
                .collect();
            return Ok(Some(lifted));
        }
        // Pass the document's
        // enclosing directory as the workspace root so
        // relative `source <path>` arguments resolve.  When
        // the URI isn't a `file://` URL we leave the workspace
        // root unset and only absolute paths surface as links.
        let workspace_root = uri
            .to_file_path()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
            .and_then(|p| p.to_str().map(str::to_owned));
        // Pure-CPU segmentation on a worker so a parser panic is contained
        // as a JSON-RPC error.
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
                target: Uri::from_str(&l.target).ok(),
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
        // `null` — when both are disabled.
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
        if !self
            .feature_enabled("codeLens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
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
        // count reflects workspace-wide usage.  Share the index with
        // the worker via a refcounted handle and take the read lock
        // inside the blocking closure, so the count walk holds a
        // shared read lock instead of a private copy.
        let workspace_index = Arc::clone(&self.workspace_index);
        let uri_str = uri.to_string();
        let worker_uri = uri_str.clone();
        let lenses = tokio::task::spawn_blocking(move || {
            let workspace = workspace_index.blocking_read();
            core_code_lens::code_lenses(
                &doc.text,
                &doc.dialect,
                Some(&analysis),
                Some(&*workspace),
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
            .map(|l| {
                let has_qname = !l.qname.is_empty();
                CodeLens {
                    range: lift_lsp_range(l.range),
                    // Carry the qualified name *and* the document URI so
                    // `codeLens/resolve` can recompute the count against the
                    // live workspace and attach the clickable command.
                    // Method / class-member lenses have no qname and stay
                    // informational (their eager title is authoritative).
                    data: has_qname
                        .then(|| serde_json::json!({ "qname": l.qname, "uri": uri_str.clone() })),
                    // A reference-count lens is returned WITHOUT a command so the
                    // client calls `codeLens/resolve`, which attaches the
                    // clickable `tcl-lsp.showReferences` command with its
                    // `[uri, position, locations]` arguments.  Setting a command
                    // here would mark the lens resolved, the client would skip
                    // `resolve`, and the lens would render as an inert bare title
                    // (#724 — "reference is not active").  Informational lenses
                    // keep their eager title+command (they are never resolved).
                    command: (!has_qname).then_some(tower_lsp_server::ls_types::Command {
                        title: l.command_title,
                        command: l.command,
                        arguments: None,
                    }),
                }
            })
            .collect();
        Ok(Some(lifted))
    }

    /// Resolve a code lens to its authoritative reference-count title.
    ///
    /// The server advertises lenses
    /// eagerly with a count, but the client calls `codeLens/resolve`
    /// before display; recomputing here against the *current* document and
    /// workspace keeps the title consistent with Find All References even
    /// when the workspace changed since the lens was produced.
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
                Some((qname.to_owned(), Uri::from_str(uri).ok()?))
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
        // Share the workspace index with the worker via a refcounted
        // handle and take the read lock inside the blocking closure,
        // so the recomputed count walk holds a shared read lock
        // rather than a private copy.
        let workspace_index = Arc::clone(&self.workspace_index);
        let uri_str = uri.to_string();
        let analysis_for_count = Arc::clone(&analysis);
        let count_text = doc.text.clone();
        let count_dialect = doc.dialect.clone();
        let lenses = tokio::task::spawn_blocking(move || {
            let workspace = workspace_index.blocking_read();
            core_code_lens::code_lenses(
                &count_text,
                &count_dialect,
                Some(&analysis_for_count),
                Some(&*workspace),
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
            // Resolve the actual reference locations so clicking the lens opens
            // a peek (the lens title alone is informational — a bare title with
            // no command is rendered but inert, the "reference is not active"
            // regression of #724).  The locations feed the client-side
            // `tcl-lsp.showReferences` wrapper, which converts them and
            // delegates to the built-in `editor.action.showReferences`.
            let position = lens.range.start;
            let locations = self
                .reference_locations_at(&uri, &doc.text, &doc.dialect, &analysis, position)
                .await;
            let arguments = vec![
                serde_json::Value::String(uri.to_string()),
                serde_json::to_value(position).unwrap_or(serde_json::Value::Null),
                serde_json::to_value(&locations).unwrap_or(serde_json::Value::Null),
            ];
            lens.command = Some(tower_lsp_server::ls_types::Command {
                title: matching.command_title,
                command: "tcl-lsp.showReferences".to_owned(),
                arguments: Some(arguments),
            });
        }
        Ok(lens)
    }

    async fn code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<Vec<CodeActionOrCommand>>> {
        if !self
            .feature_enabled("codeActions", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
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
        let context_diags = lift_context_diagnostics(&params.context.diagnostics);
        let dialect = doc.dialect.clone();
        let uri_str = uri.as_str().to_string();
        // IRULE4002 generic-name patterns for the iRules-only compiler-checks
        // code-action lowering below (uncached, off the salsa path).
        let generic_patterns = self.generic_variable_patterns.lock().await.clone();
        let actions = tokio::task::spawn_blocking(move || {
            let mut actions = core_code_actions::code_actions(&doc.text, range, Some(&analysis));
            actions.extend(core_code_actions::package_require_actions(
                &doc.text, range, &registry,
            ));
            actions.extend(core_code_actions::context_diagnostic_actions(
                &doc.text,
                &context_diags,
            ));
            // BIG-IP `.conf`/`.scf`: the dialect-specific rename-partition /
            // rename-object / extract-rule actions (`tcl-lsp-core::bigip_code_actions`).
            // The generic Tcl code-action path above yields nothing useful on a
            // non-Tcl config, so extend rather than replace — mirroring the
            // references / document-links BIG-IP routing.
            if Backend::is_bigip_dialect(&dialect) {
                actions.extend(core_code_actions::bigip_code_actions(
                    &doc.text, range, &uri_str,
                ));
            }
            // iRules-only: the `# Profiles:` header source action.
            if dialect == "f5-irules"
                && let Some(a) = core_code_actions::profiles_action(&doc.text, &analysis, &registry)
            {
                actions.push(a);
            }
            // Compiler-check quick-fixes: the iRules control-flow fixes
            // (IRULE5002 unguarded drop / IRULE5004 DNS::return) plus the
            // shimmer-family noqa-suppress action (S100/S101/S102/S110) —
            // every dialect's checks are lowered here, not just iRules'; a
            // plain-Tcl document's checks simply carry no IRULE-family fixes.
            let checks = tcl_lsp_db::compiler_check_diagnostics_uncached(
                &doc.text,
                &registry,
                &dialect,
                generic_patterns.as_deref(),
            );
            actions.extend(core_code_actions::check_diagnostic_actions(
                &doc.text,
                range,
                &checks.checks,
                &disabled_codes,
            ));
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
        let lifted = lift_code_actions(actions, &uri, params.context.only.as_ref());
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
            "tcl-lsp.listTclInstallations" => Ok(Some(self.list_tcl_installations_command().await)),
            "tcl-lsp.setDialect" => self.set_dialect_command(&params.arguments).await,
            "tcl-lsp.compilerExplorer" => self.compiler_explorer_command(&params.arguments).await,
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
        // Build from the resolved `tclLsp.formatting` settings; the client's
        // `FormattingOptions.tabSize` / `insertSpaces` override indentation by
        // LSP contract.
        let formatting = self.resolved_formatting(&params.text_document.uri).await;
        let config = formatter_config_from(&formatting, &params.options, &doc.dialect);
        // Pure-CPU formatting on a worker so a parser panic is contained as
        // a JSON-RPC error.
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
        let formatting = self.resolved_formatting(&params.text_document.uri).await;
        let config = formatter_config_from(&formatting, &params.options, &doc.dialect);
        // Pure-CPU formatting on a worker so a parser panic is contained as
        // a JSON-RPC error.
        let text = doc.text.clone();
        let edits = tokio::task::spawn_blocking(move || {
            core_formatting::range_formatting(&text, range, &config, &registry)
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
                    // The LSP spec requires `result[i]` to answer
                    // `positions[i]`, so every position must yield a range.
                    // When no chain is found (e.g. the cursor sits on empty
                    // space), fall back to a degenerate range at the cursor
                    // itself rather than dropping the entry, which would
                    // misalign the client's cursor-to-range pairing
                    // (RUST_ISSUE_100).
                    materialise_selection_range(&chain).unwrap_or(SelectionRange {
                        range: Range {
                            start: pos,
                            end: pos,
                        },
                        parent: None,
                    })
                })
                .collect::<Vec<SelectionRange>>()
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("selection_range worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(result))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<PrepareRenameResponse>> {
        if !self
            .feature_enabled("rename", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        let pos = params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        let text = doc.text.clone();
        let analysis_for_worker = analysis.clone();
        let result = tokio::task::spawn_blocking(move || {
            core_rename::prepare_rename(&text, pos.line, pos.character, &analysis_for_worker)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("prepare_rename worker panicked: {err}").into(),
            data: None,
        })?;
        if let Some(p) = result {
            return Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                range: lift_lsp_range(p.range),
                placeholder: p.placeholder,
            }));
        }
        // Consumer-document fall-through (M8): the local analysis has no
        // declaration to anchor prepare on, but the cursor symbol may still
        // resolve through the workspace index (a sibling document or an
        // autoloaded library defines it) — exactly the case the rename
        // handler's workspace-resolved branch serves.  VS Code gates every
        // rename behind prepare, so refusing here would make that branch
        // unreachable from the editor.  Accept with the call-site word's own
        // range when the workspace resolves it.
        let Some(word_p) = core_rename::word_prepare_at(&doc.text, pos.line, pos.character) else {
            return Ok(None);
        };
        if !self
            .resolve_workspace_symbols(&uri, &doc.text, &analysis, pos)
            .await
            .is_empty()
        {
            return Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                range: lift_lsp_range(word_p.range),
                placeholder: word_p.placeholder,
            }));
        }
        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        if !self
            .feature_enabled("rename", &params.text_document_position.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        let Some(doc) = self.read_document(&uri).await else {
            return Ok(None);
        };
        // Per-dialect cached registry for safety gating — proc renames
        // refuse to overwrite built-in command names.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await;
        // Method rename → cross-file override-family path.  A method
        // (re)defined up or down the hierarchy is one polymorphic name, and
        // the override family can span files, so this handles the current
        // document *and* every sibling that defines a family class
        // uniformly (the single-document family is incomplete when the
        // connecting ancestor lives in another file).  Falls through to the
        // single-document path when the family is empty (e.g. the index is
        // not yet populated) so nothing regresses.
        if core_rename::is_safe_symbol_name(&new_name)
            && let Some((seed_class, method)) =
                core_rename::method_rename_target(&doc.text, pos.line, pos.character, &analysis)
        {
            let changes = self
                .cross_file_method_rename(&seed_class, &method, &new_name)
                .await;
            if !changes.is_empty() {
                return Ok(Some(WorkspaceEdit {
                    changes: Some(changes),
                    document_changes: None,
                    change_annotations: None,
                }));
            }
        }
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
        let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
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
        // Cross-document rename: rewrite the symbol's call / definition
        // sites in sibling documents (or resolve through the workspace
        // oracle when the in-document rename found nothing local to
        // resolve against).  Aborts the whole rename when a sibling's
        // provenance isn't fully writable (issue #945 fault 1).
        if self
            .extend_rename_with_cross_document_edits(
                RenameContext {
                    uri: &uri,
                    source: &doc.text,
                    analysis: &analysis,
                    pos,
                    new_name: &new_name,
                    registry: &registry,
                },
                local_rejected,
                &mut changes,
            )
            .await
        {
            return Ok(None);
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

    /// `workspace/willRenameFiles`: when `.tcl` files are
    /// renamed in the editor, rewrite every dependent file's `source`
    /// literal so the workspace still loads.  The pure core
    /// (`core_file_ops::compute_rename_edits`) returns byte-span edits
    /// keyed by dependent URI; here we resolve each span to an LSP
    /// range against the dependent's current text and assemble a
    /// `WorkspaceEdit` (one `TextDocumentEdit` per dependent).
    async fn will_rename_files(
        &self,
        params: RenameFilesParams,
    ) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        // Workspace-wide file operation: gate on the process-global
        // `workspaceFileOps` toggle.
        if !self
            .feature_toggles
            .lock()
            .await
            .is_enabled("workspaceFileOps")
        {
            return Ok(None);
        }
        // Collect the byte-span edits for every rename in the batch.
        let roots: Vec<String> = self
            .workspace_folder_urls()
            .await
            .iter()
            .map(|u| u.as_str().to_owned())
            .collect();
        let raw_edits: Vec<core_file_ops::RenameEdit> = {
            let index = self.workspace_index.read().await;
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
        let mut by_dep: HashMap<Uri, Vec<TextEdit>> = HashMap::new();
        for edit in raw_edits {
            let Ok(dep_url) = Uri::from_str(&edit.uri) else {
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

    /// `workspace/didRenameFiles`: after the client applies a
    /// rename on disk, refresh the workspace index — drop the old URI's
    /// entries and re-index the renamed file from its new path so
    /// cross-document features (including future renames) stay current.
    async fn did_rename_files(&self, params: RenameFilesParams) {
        // Same workspace-file-ops gate as `will_rename_files`.
        if !self
            .feature_toggles
            .lock()
            .await
            .is_enabled("workspaceFileOps")
        {
            return;
        }
        for f in &params.files {
            if let Ok(old_url) = Uri::from_str(&f.old_uri) {
                self.workspace_index
                    .write()
                    .await
                    .remove_document(old_url.as_str());
            }
            if let Ok(new_url) = Uri::from_str(&f.new_uri) {
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
        let hover_profile = tcl_dialect::DialectProfile::by_name(&doc.dialect);
        let result = tokio::task::spawn_blocking(move || {
            core_hover::hover_with_profile(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
                hover_profile,
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
        tower_lsp_server::ls_types::CompletionTextEdit::Edit(tower_lsp_server::ls_types::TextEdit {
            range: tower_lsp_server::ls_types::Range {
                start: tower_lsp_server::ls_types::Position::new(line, e.start_char),
                end: tower_lsp_server::ls_types::Position::new(line, e.end_char),
            },
            new_text: e.new_text,
        })
    });
    let documentation = item
        .documentation
        .map(tower_lsp_server::ls_types::Documentation::String);
    CompletionItem {
        label: item.label,
        kind: Some(lift_completion_kind(item.kind)),
        insert_text: Some(item.insert_text),
        detail: item.detail,
        documentation,
        sort_text: item.sort_text,
        // Snippet items carry VS Code tabstop syntax and
        // filter on their `tcl-…` prefix.
        insert_text_format: item
            .is_snippet
            .then_some(tower_lsp_server::ls_types::InsertTextFormat::SNIPPET),
        filter_text: item.filter_text,
        text_edit,
        ..CompletionItem::default()
    }
}

/// Materialise the `tcl-lsp-core::selection_range` flat-vector
/// representation into an `ls_types` `SelectionRange` tree.
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
    let index = tcl_lexer::LineIndex::new_lsp(source);
    if line as usize >= index.line_count() {
        return None;
    }
    Some(index.offset_at_utf16(line, tcl_lexer::Utf16Col::new(col), source) as usize)
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

/// Lift the editor's request-context diagnostics (the ones it currently shows)
/// into the core's [`core_code_actions::ContextDiagnostic`] shape so
/// context-driven quick-fixes can act on them even when the analyser did not
/// re-emit them.
fn lift_context_diagnostics(
    diagnostics: &[tower_lsp_server::ls_types::Diagnostic],
) -> Vec<core_code_actions::ContextDiagnostic> {
    diagnostics
        .iter()
        .filter_map(|d| {
            let code = match d.code.as_ref()? {
                tower_lsp_server::ls_types::NumberOrString::String(s) => s.clone(),
                tower_lsp_server::ls_types::NumberOrString::Number(n) => n.to_string(),
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
        .collect()
}

/// Filter the computed actions by the client's `only` kinds and lift each into
/// an LSP [`CodeActionOrCommand`] for document `uri`.
fn lift_code_actions(
    actions: Vec<core_code_actions::CodeAction>,
    uri: &Uri,
    only: Option<&Vec<tower_lsp_server::ls_types::CodeActionKind>>,
) -> Vec<CodeActionOrCommand> {
    // Honour the client's `only` filter (e.g. `["refactor.extract"]`): an
    // action is kept when its kind prefix-matches a requested kind in
    // either direction (`refactor` matches `refactor.extract` and vice
    // versa).
    let only: Option<Vec<String>> = only.map(|kinds| {
        kinds
            .iter()
            .map(|k| k.as_str().to_owned())
            .collect::<Vec<_>>()
    });
    actions
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
            let command = a.command.map(|c| action_command_to_lsp(a.title.clone(), c));
            // Surface the rendered tmsh data-group definition (the
            // extract-to-datagroup refactor) as the action's `data`
            // payload.
            let data = a
                .data_group_definition
                .map(|def| serde_json::json!({ "data_group_definition": def }));
            CodeActionOrCommand::CodeAction(CodeAction {
                title: a.title,
                kind: Some(tower_lsp_server::ls_types::CodeActionKind::new(
                    a.kind.as_str(),
                )),
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
        .collect()
}

/// Lift a [`tcl_bigip::Range`] (UTF-16 columns, **inclusive** end — the last
/// covered code unit) to an LSP [`Range`] (exclusive end). BIG-IP object
/// stanzas and reference spans are single-line tokens, so the exclusive end
/// column is `end.character + 1` (the same convention
/// [`lift_config_diagnostic`] uses for validator ranges).
fn lift_bigip_range(r: tcl_bigip::Range) -> Range {
    Range {
        start: Position {
            line: r.start.line,
            character: r.start.character,
        },
        end: Position {
            line: r.end.line,
            character: r.end.character + 1,
        },
    }
}

/// Convert a core [`ActionCommand`](core_code_actions::ActionCommand) into an
/// LSP [`Command`](tower_lsp_server::ls_types::Command), forwarding **both**
/// argument kinds.
///
/// A given action command carries exactly one kind:
/// * integer-position commands (`tclLsp.renameSymbolAtPosition`) use `args`
///   (`[line, start, end]`), and
/// * BIG-IP / editor string commands use `string_args` —
///   `tclLsp.renamePartition` → `[uri, partition]`, `editor.action.rename`
///   → `[uri]`.
///
/// String args are emitted first, then int args. Because each command uses
/// only one kind, the concatenation yields the exact `arguments` array the
/// client expects, without this
/// conversion needing to know which kind it is handling. Forwarding
/// `string_args` here is what stops the BIG-IP rename actions from reaching
/// the editor argument-less.
fn action_command_to_lsp(
    title: String,
    c: core_code_actions::ActionCommand,
) -> tower_lsp_server::ls_types::Command {
    let mut arguments: Vec<serde_json::Value> = c
        .string_args
        .into_iter()
        .map(serde_json::Value::from)
        .collect();
    arguments.extend(c.args.into_iter().map(serde_json::Value::from));
    tower_lsp_server::ls_types::Command {
        title,
        command: c.command,
        arguments: Some(arguments),
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

/// Build a [`core_formatting::FormatterConfig`] from the resolved
/// `tclLsp.formatting` settings object, then apply the request's LSP
/// `FormattingOptions` (`tabSize` / `insertSpaces`) as the indentation override
/// (the editor's per-request indentation wins, per the LSP contract). Applies
/// the formatter field mapping so editor- and
/// config-file-set formatter options actually take effect.
/// Apply the `tclLsp.formatting.*` JSON object (camelCase keys) onto `cfg`,
/// coercing each key per its type. Split out of [`formatter_config_from`] so
/// the per-key mapping stays a single flat block without tripping the
/// `too_many_lines` lint on the caller.
fn apply_formatting_object(
    obj: &serde_json::Map<String, serde_json::Value>,
    cfg: &mut core_formatting::FormatterConfig,
) {
    use core_formatting::IndentStyle;
    let as_usize = |v: &serde_json::Value| v.as_u64().map(|n| usize::try_from(n).unwrap_or(0));
    if let Some(n) = obj.get("indentSize").and_then(as_usize) {
        cfg.indent_size = n.max(1);
    }
    if let Some(s) = obj.get("indentStyle").and_then(serde_json::Value::as_str) {
        cfg.indent_style = match s.to_ascii_lowercase().as_str() {
            "tabs" | "tab" => IndentStyle::Tabs,
            _ => IndentStyle::Spaces,
        };
    }
    if let Some(n) = obj.get("continuationIndent").and_then(as_usize) {
        cfg.continuation_indent = n;
    }
    // `braceStyle` is accepted but only K&R is implemented, so it stays the
    // default (a no-op rather than an error).
    if let Some(b) = obj
        .get("spaceBetweenBraces")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.space_between_braces = b;
    }
    // Both `maxLineLength` and the legacy `lineLength` map to the hard limit.
    if let Some(n) = obj
        .get("maxLineLength")
        .or_else(|| obj.get("lineLength"))
        .and_then(as_usize)
    {
        cfg.max_line_length = n.max(1);
    }
    if let Some(n) = obj.get("goalLineLength").and_then(as_usize) {
        cfg.goal_line_length = n.max(1);
    }
    if let Some(b) = obj
        .get("spaceAfterCommentHash")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.space_after_comment_hash = b;
    }
    if let Some(b) = obj
        .get("trimTrailingWhitespace")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.trim_trailing_whitespace = b;
    }
    if let Some(b) = obj
        .get("enforceBracedVariables")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.enforce_braced_variables = b;
    }
    if let Some(s) = obj.get("lineEnding").and_then(serde_json::Value::as_str) {
        cfg.line_ending = match s.to_ascii_lowercase().as_str() {
            "crlf" => "\r\n".to_owned(),
            "cr" => "\r".to_owned(),
            "lf" => "\n".to_owned(),
            other => other.to_owned(),
        };
    }
    if let Some(b) = obj
        .get("ensureFinalNewline")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.ensure_final_newline = b;
    }
    if let Some(b) = obj
        .get("expandSingleLineBodies")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.expand_single_line_bodies = b;
    }
    // These four were shipped by every editor but never mapped here, so
    // toggling them did nothing (RUST_ISSUE_133). `minBodyCommandsForExpansion`
    // and `replaceSemicolonsWithNewlines` are engine-consumed;
    // `enforceBracedExpr` / `alignCommentsToCode` are carried through so the
    // resolved config round-trips (and so they take effect once the engine
    // consumes them).
    if let Some(n) = obj.get("minBodyCommandsForExpansion").and_then(as_usize) {
        cfg.min_body_commands_for_expansion = n.max(1);
    }
    if let Some(b) = obj
        .get("replaceSemicolonsWithNewlines")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.replace_semicolons_with_newlines = b;
    }
    if let Some(b) = obj
        .get("enforceBracedExpr")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.enforce_braced_expr = b;
    }
    if let Some(b) = obj
        .get("alignCommentsToCode")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.align_comments_to_code = b;
    }
    if let Some(n) = obj.get("blankLinesBetweenProcs").and_then(as_usize) {
        cfg.blank_lines_between_procs = n;
    }
    if let Some(n) = obj.get("blankLinesBetweenBlocks").and_then(as_usize) {
        cfg.blank_lines_between_blocks = n;
    }
    if let Some(n) = obj.get("maxConsecutiveBlankLines").and_then(as_usize) {
        cfg.max_consecutive_blank_lines = n;
    }
    apply_docstring_formatting(obj, cfg);
}

/// Read the `docstring*` `tclLsp.formatting.*` settings into `cfg`. Kept for
/// config compatibility (round-tripped into the config) even though the docstring
/// rewriter that would consume them is not yet implemented. Split out of
/// [`apply_formatting_object`] to keep each per-key block under the
/// `too_many_lines` lint.
fn apply_docstring_formatting(
    obj: &serde_json::Map<String, serde_json::Value>,
    cfg: &mut core_formatting::FormatterConfig,
) {
    if let Some(s) = obj
        .get("docstringStyle")
        .and_then(serde_json::Value::as_str)
    {
        cfg.docstring_style = match s.to_ascii_lowercase().as_str() {
            "preceding" => core_formatting::DocstringStyle::Preceding,
            "body" => core_formatting::DocstringStyle::Body,
            _ => core_formatting::DocstringStyle::None,
        };
    }
    if let Some(s) = obj
        .get("docstringTagStyle")
        .and_then(serde_json::Value::as_str)
    {
        cfg.docstring_tag_style = match s.to_ascii_lowercase().as_str() {
            "plain" => core_formatting::DocstringTagStyle::Plain,
            "none" => core_formatting::DocstringTagStyle::None,
            _ => core_formatting::DocstringTagStyle::Doxygen,
        };
    }
    if let Some(b) = obj
        .get("docstringDecoration")
        .and_then(serde_json::Value::as_bool)
    {
        cfg.docstring_decoration = b;
    }
    if let Some(c) = obj
        .get("docstringDecorationChar")
        .and_then(serde_json::Value::as_str)
        .and_then(|s| s.chars().next())
    {
        cfg.docstring_decoration_char = c;
    }
    if let Some(n) = obj
        .get("docstringDecorationWidth")
        .and_then(|v| v.as_u64().map(|n| usize::try_from(n).unwrap_or(0)))
    {
        cfg.docstring_decoration_width = n;
    }
}

fn formatter_config_from(
    formatting: &serde_json::Value,
    options: &tower_lsp_server::ls_types::FormattingOptions,
    dialect: &str,
) -> core_formatting::FormatterConfig {
    use core_formatting::IndentStyle;
    let mut cfg = core_formatting::FormatterConfig {
        // Tokenise with the document's dialect so, e.g., an `.irul` file's
        // `if {expr}{body}` (`}{` valid in TMM) is parsed and re-emitted as
        // `} {` rather than left unchanged by the stock-Tcl lexer.
        lexer_config: tcl_lexer::LexerConfig::for_dialect(dialect),
        ..Default::default()
    };
    if let Some(obj) = formatting.as_object() {
        apply_formatting_object(obj, &mut cfg);
    }
    // LSP `FormattingOptions` indentation overrides the config (per contract);
    // a real editor always sends `tabSize >= 1`, so a degenerate 0 falls back to
    // the configured indent size rather than clamping it away.
    if options.tab_size >= 1 {
        cfg.indent_size = usize::try_from(options.tab_size).unwrap_or(cfg.indent_size);
        cfg.indent_style = if options.insert_spaces {
            IndentStyle::Spaces
        } else {
            IndentStyle::Tabs
        };
    }
    cfg.indent_size = cfg.indent_size.max(1);
    cfg
}

/// Whether `uri` names a tcl-lsp config file (`.tcl-lsp.ini` project config or
/// the user `config.ini`) — a watched-file change to one triggers a config
/// re-apply.
fn is_config_file(uri: &Uri) -> bool {
    let s = uri.as_str();
    s.ends_with("/.tcl-lsp.ini")
        || s.ends_with("\\.tcl-lsp.ini")
        || s.ends_with("/tcl-lsp/config.ini")
        || s.ends_with("\\tcl-lsp\\config.ini")
}

/// Read an INI config file at `path` (if present/readable) into the
/// editor-shape `tclLsp` settings JSON for its `layer`. Missing or unreadable
/// files yield an empty object, so a layer simply contributes nothing.
fn read_ini_layer(path: Option<PathBuf>, layer: config_ini::Layer) -> serde_json::Value {
    path.and_then(|p| std::fs::read_to_string(p).ok())
        .map_or_else(
            || serde_json::json!({}),
            |content| config_ini::settings_from_ini(&content, layer),
        )
}

/// Normalise a pulled configuration payload into the unwrapped `tclLsp`-content
/// object [`Backend::apply_global_config`] expects, accepting three shapes:
///
/// 1. **Nested** — `{ "tclLsp": { "optimiser": { "O109": false } } }`.
/// 2. **Flat dotted** — `{ "tclLsp.optimiser.O109": false }` (or without the
///    `tclLsp.` prefix), folded into nested objects.
/// 3. **Unwrapped** — `{ "optimiser": { "O109": false }, "dialect": "tcl8.6" }`
///    (e.g. a `JetBrains` pull-model response with no `tclLsp` prefix).
///
/// The shapes compose: a nested `tclLsp` object is merged with any flat dotted
/// keys and any unwrapped top-level sections.
/// Collapse the retired `features.inlayHints` boolean into `inlayTypeHints`
/// in-place, for one settings layer. `inlayHints` is the backward-compatible
/// alias for `inlayTypeHints`; an explicit `inlayTypeHints` in the *same* layer
/// still wins (`or_insert` keeps it). Done per-layer before [`merge_settings`]
/// so cross-layer precedence stays correct — see the call site in
/// [`Backend::pull_and_apply_config`].
fn collapse_inlay_alias(settings: &mut serde_json::Value) {
    if let Some(features) = settings
        .get_mut("features")
        .and_then(serde_json::Value::as_object_mut)
        && let Some(alias) = features.remove("inlayHints")
    {
        features.entry("inlayTypeHints").or_insert(alias);
    }
}

fn normalize_config_payload(payload: &serde_json::Value) -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut out = Map::new();
    let Some(obj) = payload.as_object() else {
        return Value::Object(out);
    };
    // 1. A nested `tclLsp` object contributes its keys directly.
    if let Some(nested) = obj.get("tclLsp").and_then(Value::as_object) {
        for (k, v) in nested {
            out.insert(k.clone(), v.clone());
        }
    }
    for (key, value) in obj {
        if key == "tclLsp" {
            continue;
        }
        // 2. Flat dotted keys (`tclLsp.a.b` or `a.b`) → nested objects.
        let path = key.strip_prefix("tclLsp.").unwrap_or(key);
        if path.contains('.') {
            let segments: Vec<&str> = path.split('.').collect();
            let mut cursor = &mut out;
            for seg in &segments[..segments.len() - 1] {
                let slot = cursor
                    .entry((*seg).to_owned())
                    .or_insert_with(|| Value::Object(Map::new()));
                // A colliding non-object at this segment — a mixed-shape payload
                // that supplied both a scalar/object prefix and a dotted child
                // (e.g. `tclLsp.features: false` alongside
                // `tclLsp.features.semanticTokens: true`) — is replaced by an
                // object so the deeper key can be inserted (last-writer-wins),
                // rather than panicking on the descent.
                if !slot.is_object() {
                    *slot = Value::Object(Map::new());
                }
                cursor = slot
                    .as_object_mut()
                    .expect("slot was just ensured to be an object");
            }
            cursor.insert(segments[segments.len() - 1].to_owned(), value.clone());
        } else if key.starts_with("tclLsp.") {
            // `tclLsp.dialect` with no further nesting → top-level key.
            out.insert(path.to_owned(), value.clone());
        } else {
            // 3. Unwrapped top-level section/scalar.
            out.insert(key.clone(), value.clone());
        }
    }
    Value::Object(out)
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
/// auto behaviour); every explicit
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
    // Every resolution starts from the opt-in default-off set; a `false` value
    // disables a code and a `true` value *enables* one (removing it from the
    // set, so a default-off code like W242 can be turned on). Mirrors
    // `server/settings.py`'s `new_disabled = set(default_disabled())` + per-code
    // add/discard.
    if let Some(map) = settings
        .get("tclLsp")
        .and_then(|v| v.get("diagnostics"))
        .and_then(serde_json::Value::as_object)
    {
        let mut set = default_disabled_set();
        for (code, v) in map {
            match v.as_bool() {
                Some(false) => {
                    set.insert(code.clone());
                }
                Some(true) => {
                    set.remove(code);
                }
                None => {}
            }
        }
        return Some(set);
    }
    let obj = settings.as_object()?;
    let mut set = default_disabled_set();
    let mut found = false;
    for (k, v) in obj {
        if let Some(code) = k.strip_prefix("tclLsp.diagnostics.") {
            found = true;
            match v.as_bool() {
                Some(false) => {
                    set.insert(code.to_owned());
                }
                Some(true) => {
                    set.remove(code);
                }
                None => {}
            }
        }
    }
    found.then_some(set)
}

/// Map a `tclLsp.diagnosticSeverity.<CODE>` config value to an LSP severity
/// (case-insensitive). `"error"`, `"warning"`, `"information"` / `"info"`, and
/// `"hint"` select the matching [`DiagnosticSeverity`]; anything else —
/// including `"default"` and `""` — yields `None`, meaning "no override" (the
/// analyser's emitted severity stands).
fn parse_severity_value(s: &str) -> Option<tower_lsp_server::ls_types::DiagnosticSeverity> {
    use tower_lsp_server::ls_types::DiagnosticSeverity;
    if s.eq_ignore_ascii_case("error") {
        Some(DiagnosticSeverity::ERROR)
    } else if s.eq_ignore_ascii_case("warning") {
        Some(DiagnosticSeverity::WARNING)
    } else if s.eq_ignore_ascii_case("information") || s.eq_ignore_ascii_case("info") {
        Some(DiagnosticSeverity::INFORMATION)
    } else if s.eq_ignore_ascii_case("hint") {
        Some(DiagnosticSeverity::HINT)
    } else {
        None
    }
}

/// Parse per-code LSP severity overrides from a `tclLsp` settings payload,
/// accepting the nested object (`{"tclLsp":{"diagnosticSeverity":{"W211":"warning"}}}`)
/// and the flat-dotted (`{"tclLsp.diagnosticSeverity.W211":"warning"}`) shapes.
/// Returns `Some(map)` (possibly empty) when the section is present in either
/// shape, else `None` (so the caller leaves the current map untouched). Entries
/// whose value is not a recognised severity string ([`parse_severity_value`])
/// are skipped, so the analyser's emitted severity stands for them. Mirrors
/// [`settings_disabled_diagnostics`].
fn settings_severity_overrides(
    settings: &serde_json::Value,
) -> Option<std::collections::HashMap<String, tower_lsp_server::ls_types::DiagnosticSeverity>> {
    if let Some(map) = settings
        .get("tclLsp")
        .and_then(|v| v.get("diagnosticSeverity"))
        .and_then(serde_json::Value::as_object)
    {
        let mut overrides = HashMap::new();
        for (code, v) in map {
            if let Some(severity) = v.as_str().and_then(parse_severity_value) {
                overrides.insert(code.clone(), severity);
            }
        }
        return Some(overrides);
    }
    let obj = settings.as_object()?;
    let mut overrides = HashMap::new();
    let mut found = false;
    for (k, v) in obj {
        if let Some(code) = k.strip_prefix("tclLsp.diagnosticSeverity.") {
            found = true;
            if let Some(severity) = v.as_str().and_then(parse_severity_value) {
                overrides.insert(code.to_owned(), severity);
            }
        }
    }
    found.then_some(overrides)
}

/// Parse one folder's resolved `tclLsp` config object into a [`FolderConfig`]
/// of overrides.  Keys absent from the object stay `None` / empty so the
/// resolver inherits the process-global value.  Returns `None` when `cfg` is
/// not a JSON object (the folder pull returned nothing usable).  Mirrors the
/// key handling of [`Backend::pull_and_apply_config`]'s global pull.
/// Parse the `optimiser` section of a folder config: enable flag, profile, and
/// the per-`O-code` boolean overrides.
fn parse_folder_optimiser(obj: &serde_json::Map<String, serde_json::Value>, fc: &mut FolderConfig) {
    let Some(opt) = obj.get("optimiser").and_then(serde_json::Value::as_object) else {
        return;
    };
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

/// Parse the formatter line length, the whole `formatting` section, and the
/// `style.lineLength` (W111) threshold of a folder config.
fn parse_folder_formatting(
    obj: &serde_json::Map<String, serde_json::Value>,
    fc: &mut FolderConfig,
) {
    if let Some(len) = obj
        .get("formatting")
        .and_then(|f| f.get("lineLength"))
        .or_else(|| obj.get("formatting").and_then(|f| f.get("maxLineLength")))
        .or_else(|| obj.get("lineLength"))
        .and_then(serde_json::Value::as_u64)
    {
        fc.line_length = Some(u32::try_from(len).unwrap_or(80));
    }
    if let Some(formatting) = obj.get("formatting") {
        fc.formatting = Some(formatting.clone());
    }
    if let Some(len) = obj
        .get("style")
        .and_then(|s| s.get("lineLength"))
        .and_then(serde_json::Value::as_u64)
        .filter(|&len| len > 0)
    {
        fc.style_line_length = Some(u32::try_from(len).unwrap_or(120));
    }
}

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
    parse_folder_optimiser(obj, &mut fc);
    if let Some(b) = obj
        .get("shimmer")
        .and_then(|s| s.get("enabled"))
        .and_then(serde_json::Value::as_bool)
    {
        fc.shimmer_enabled = Some(b);
    }
    parse_folder_formatting(obj, &mut fc);
    // `tclLsp.extraCommands` per-folder override.
    if let Some(cmds) = obj
        .get("extraCommands")
        .and_then(serde_json::Value::as_array)
    {
        fc.extra_commands = Some(
            cmds.iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        );
    }
    // `tclLsp.diagnostics.genericVariablePatterns` per-folder override. A present
    // array replaces the patterns (`Replace`); an explicit `null` requests the
    // analyser's built-in defaults (`BuiltinDefaults`); an absent key leaves the
    // value inheriting the global (`Inherit`, the default).
    if let Some(value) = obj
        .get("diagnostics")
        .and_then(serde_json::Value::as_object)
        .and_then(|d| d.get("genericVariablePatterns"))
    {
        if let Some(patterns) = value.as_array() {
            fc.generic_variable_patterns = FolderGenericPatterns::Replace(
                patterns
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect(),
            );
        } else if value.is_null() {
            fc.generic_variable_patterns = FolderGenericPatterns::BuiltinDefaults;
        }
    }
    // `tclLsp.libraryPaths` per-folder override.
    if let Some(paths) = obj
        .get("libraryPaths")
        .and_then(serde_json::Value::as_array)
    {
        fc.library_paths = Some(
            paths
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        );
    }
    // `.tcl-lsp.ini [project] entryPoints` per-folder value.
    if let Some(points) = obj.get("entryPoints").and_then(serde_json::Value::as_array) {
        fc.entry_points = Some(
            points
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        );
    }
    // The disabled-diagnostics and non-ASCII helpers expect the value wrapped
    // under `tclLsp`; the per-folder pull hands us the section content directly.
    let wrapped = serde_json::json!({ "tclLsp": cfg });
    fc.non_ascii_mode = settings_non_ascii_mode(&wrapped);
    fc.disabled_diagnostics = settings_disabled_diagnostics(&wrapped);
    fc.severity_overrides = settings_severity_overrides(&wrapped);
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
    let mut index = tcl_lexer::LineIndex::new_lsp(text);
    apply_content_change_indexed(text, range, new_text, &mut index)
}

/// [`apply_content_change`] that resolves the edit through a **persisted**
/// [`tcl_lexer::LineIndex`] built with the LSP end-of-line model
/// ([`tcl_lexer::LineIndex::new_lsp`]), then rebuilds it from the spliced
/// result so it stays consistent with the returned text.
///
/// The index **must** use the LSP EOL model (`\n`, `\r\n`, *and lone `\r`*):
/// the client resolves the incoming `range` against that model, so a `\n`-only
/// index would resolve a bare-`\r` file's edit to the wrong byte offset and
/// corrupt the shadow buffer. The `\n`-only [`tcl_lexer::LineIndex::apply_edit`]
/// cannot maintain an LSP index incrementally across CR/LF boundary ambiguity,
/// and re-analysis after a change is whole-document regardless, so the index is
/// rebuilt (not patched) — correctness over a micro-optimisation that never
/// dominated the change handler's cost.
fn apply_content_change_indexed(
    text: &str,
    range: Option<Range>,
    new_text: &str,
    index: &mut tcl_lexer::LineIndex,
) -> String {
    let Some(range) = range else {
        *index = tcl_lexer::LineIndex::new_lsp(new_text);
        return new_text.to_owned();
    };
    let a = index.offset_at_utf16(
        range.start.line,
        tcl_lexer::Utf16Col::new(range.start.character),
        text,
    ) as usize;
    let b = index.offset_at_utf16(
        range.end.line,
        tcl_lexer::Utf16Col::new(range.end.character),
        text,
    ) as usize;
    let len = text.len();
    let start = a.min(b).min(len);
    let end = a.max(b).min(len);
    let mut out = String::with_capacity(len - (end - start) + new_text.len());
    out.push_str(&text[..start]);
    out.push_str(new_text);
    out.push_str(&text[end..]);
    // Rebuild the LSP-EOL index from the spliced document.
    *index = tcl_lexer::LineIndex::new_lsp(&out);
    out
}

fn lift_span(source: &str, line_index: &tcl_lexer::LineIndex, span: tcl_lexer::Span) -> Range {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    Range {
        start: Position {
            line: start.line,
            character: start.character.get(),
        },
        end: Position {
            line: end.line,
            character: end.character.get(),
        },
    }
}

/// Diagnostic codes that are *default-off* (opt-in) in the editor catalogue.
/// They are seeded into the resolved disabled-diagnostics set so the analyser
/// suppresses them by default, and `tclLsp.diagnostics.<CODE>: true` removes a
/// code from the disabled set to enable it.
const DEFAULT_OFF_CODES: &[&str] = &["W242"];

/// A fresh disabled-diagnostics set seeded with the opt-in [`DEFAULT_OFF_CODES`]
/// — the starting point every resolution builds on.
fn default_disabled_set() -> HashSet<String> {
    DEFAULT_OFF_CODES.iter().map(|c| (*c).to_owned()).collect()
}

/// Default BIG-IP partition assumed when a config carries no explicit
/// one.
const BIGIP_DEFAULT_PARTITION: &str = "Common";

/// Lift a [`tcl_bigip::validator::ConfigDiagnostic`] (the output of the
/// BIG-IP config / iApp model-level validators) to the LSP wire shape.
/// A `tcl_bigip` [`tcl_bigip::Range`] carries
/// UTF-16 columns (LSP convention) with an **inclusive** end, so the LSP
/// end column is `end.character + 1`.
fn lift_config_diagnostic(
    d: &tcl_bigip::validator::ConfigDiagnostic,
) -> tower_lsp_server::ls_types::Diagnostic {
    use tower_lsp_server::ls_types::{DiagnosticSeverity, NumberOrString};
    tower_lsp_server::ls_types::Diagnostic {
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
/// top-of-file `# tcl-lsp: disable=…` directive — the same `is_suppressed`
/// contract `tcl_lsp_core::source_style` applies (a `"*"` entry suppresses
/// every code; the file-level `-1` bucket is document-wide). Shared by every
/// diagnostic family this module lifts directly (XC, compiler-checks);
/// `lift_analyser_diagnostics` / `lift_source_style_diagnostics` apply the
/// same contract via `tcl_lsp_core`'s own (private) copy.
fn line_suppressed(
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

/// Opt-in: lift the `f5-xc` XC100-301 translatability
/// diagnostics into LSP diagnostics for an `f5-irules` document. Codes the editor disabled
/// (`tclLsp.diagnostics.<CODE> = false`) are filtered, and the same `# noqa`
/// / file-level suppression the analyser honours is applied. `XcSeverity`
/// maps `Hint` → `HINT` and `Info` → `INFORMATION`.
fn lift_xc_diagnostics(
    source: &str,
    disabled: &HashSet<String>,
    suppressed: &std::collections::HashMap<i32, HashSet<String>>,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    use tower_lsp_server::ls_types::{DiagnosticSeverity, NumberOrString};
    f5_xc::get_xc_diagnostics(source)
        .into_iter()
        .filter(|d| !disabled.contains(&d.code))
        .filter(|d| {
            !line_suppressed(
                &d.code,
                i32::try_from(d.range.start.line).unwrap_or(i32::MAX),
                suppressed,
            )
        })
        .map(|d| tower_lsp_server::ls_types::Diagnostic {
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
            code: Some(NumberOrString::String(d.code.clone())),
            code_description: None,
            source: Some("tcl-lsp".to_string()),
            message: d.message,
            related_information: None,
            tags: None,
            data: None,
        })
        .collect()
}

/// BIG-IP config diagnostics (`BIGIP6001`–
/// `BIGIP6011`).  BIG-IP `.conf` text is not Tcl source — it has its own
/// model-level validator
/// ([`tcl_bigip::validator::validate_bigip_source`]).  Codes the
/// editor disabled via `tclLsp.diagnostics.<CODE> = false` are filtered.
fn bigip_config_diagnostics(
    text: &str,
    disabled: &HashSet<String>,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    tcl_bigip::validator::validate_bigip_source(text, BIGIP_DEFAULT_PARTITION)
        .into_iter()
        .filter(|d| !disabled.contains(&d.code))
        .map(|d| lift_config_diagnostic(&d))
        .collect()
}

/// iApp APL presentation diagnostics
/// (`IAPP7001`–`IAPP7003`).  Parses the APL presentation, optionally
/// cross-checks it against the sibling implementation's
/// `$::section__field` references, and lifts the validator output.  The
/// validator is gated on the `f5-iapps` dialect (we only reach here for
/// APL sources, so the gate is always satisfied — see [`is_apl_source`]).
fn apl_presentation_diagnostics(
    text: &str,
    impl_var_refs: Option<&[tcl_bigip::apl::IappVarRef]>,
    disabled: &HashSet<String>,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    let model = tcl_bigip::apl::parse_apl(text);
    tcl_bigip::apl::validate_iapp_presentation(&model, impl_var_refs, "f5-iapps")
        .into_iter()
        .filter(|d| !disabled.contains(&d.code))
        .map(|d| lift_config_diagnostic(&d))
        .collect()
}

/// Whether `uri` (with its editor `language_id`) is an iApp APL
/// presentation document: an explicit APL
/// language id, or a basename of `*.apl` / `presentation`.
fn is_apl_source(uri: &Uri, language_id: &str) -> bool {
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
/// at `uri` and extract its `$::section__field` variable references.
/// Prefers an open buffer in
/// the same directory (so unsaved edits to the implementation are
/// reflected), then falls back to reading a sibling from disk.  The sibling
/// is named `implementation` or carries a `.iapp` / `.iappimpl` / `.impl`
/// extension.
async fn find_sibling_impl_vars(
    uri: &Uri,
    documents: &Mutex<HashMap<Uri, DocumentState>>,
) -> Option<Vec<tcl_bigip::apl::IappVarRef>> {
    let path = uri.to_file_path()?;
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

    // 1. Prefer an open implementation buffer in the same directory: an unsaved
    //    edit to the implementation must drive the presentation's cross-file
    //    diagnostics, not a stale on-disk copy.
    {
        let docs = documents.lock().await;
        for (doc_uri, doc) in docs.iter() {
            if doc_uri == uri {
                continue;
            }
            let Some(doc_path) = doc_uri.to_file_path() else {
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
    //    deterministic pick).
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

/// File-type dispatch: if `uri` is a non-Tcl F5 dialect document
/// (BIG-IP config or iApp APL presentation), compute its model-level
/// validator diagnostics; otherwise return `None` so the caller runs the
/// normal Tcl analyser path.  The validator runs on
/// `spawn_blocking` for the same parser-panic containment the analyser path
/// uses.
async fn f5_dialect_diagnostics(
    uri: &Uri,
    text: &str,
    dialect: &str,
    language_id: &str,
    disabled: &HashSet<String>,
    documents: &Mutex<HashMap<Uri, DocumentState>>,
) -> Option<Vec<tower_lsp_server::ls_types::Diagnostic>> {
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

/// Workspace-level refinement of the analyser's single-file W120
/// (missing-`package require`) diagnostics, using the scanned package database.
///
/// This is where #723 is resolved precisely. The analyser only knows the
/// packages required/provided *in the document*; here we additionally know,
/// from the workspace + `TCLLIBPATH` `pkgIndex.tcl` files, what each
/// `package require` (transitively) pulls in — exactly the knowledge C Tcl
/// gains by running the `ifneeded` scripts.
///
/// Two rules, mirroring C Tcl's reality that a package's load script can
/// register arbitrary commands:
///
/// 1. **Conservative.** If any required package is *unknowable* — neither the
///    command registry nor the scanned database can resolve it, and it isn't
///    the core `Tcl` version pseudo-package — it may load anything, so every
///    W120 is dropped. (A wrapper package not present in the workspace /
///    `auto_path` lands here, keeping #723 fixed even with an empty database.)
/// 2. **Precise.** Otherwise a W120 for package `P` is a false positive exactly
///    when `P` is in the transitive closure of what the document's requires
///    pull in — e.g. `package require myTkPackage`, whose implementation does
///    `package require Tk`, makes `Tk` available and suppresses its W120, while
///    a required package that does *not* pull in `Tk` leaves the W120 standing.
fn refine_w120_diagnostics(
    diags: Vec<tcl_compiler::analyser::Diagnostic>,
    package_requires: &[String],
    resolver: &PackageResolver,
    registry: &CommandRegistry,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    let unknowable = package_requires
        .iter()
        .any(|p| p != "Tcl" && !registry.provides_package(p) && !resolver.provides(p));
    if unknowable {
        return diags
            .into_iter()
            .filter(|d| d.code != DiagCode::W120)
            .collect();
    }
    let available = resolver
        .transitive_available_packages(package_requires, &|p| std::fs::read_to_string(p).ok());
    diags
        .into_iter()
        .filter(|d| {
            if d.code != DiagCode::W120 {
                return true;
            }
            match w120_required_package(d) {
                Some(pkg) => !available.contains(pkg),
                None => true,
            }
        })
        .collect()
}

/// Apply the #723 workspace W120 refinement to `analyser_diags`: resolve the
/// document's `package require`s through the shared package database and drop
/// any W120 whose flagged package is transitively available. Shared by the push
/// path (`refine_and_lift_diagnostics`) and the pull path
/// (`Backend::full_diagnostics_for`) so both stay behavior-identical.
///
/// The common case (no W120, or no `package require`) skips the resolver lock
/// entirely; otherwise the (bounded) transitive scan reads only the required
/// packages' implementation files.
async fn refine_workspace_w120(
    analyser_diags: Vec<tcl_compiler::analyser::Diagnostic>,
    analysis: &AnalysisResult,
    inherited_requires: &[String],
    package_resolver: &Arc<RwLock<PackageResolver>>,
    registry: &CommandRegistry,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    if !analyser_diags.iter().any(|d| d.code == DiagCode::W120) {
        return analyser_diags;
    }
    // The document's own `package require`s plus any inherited from the
    // project's entry files / `source` ancestors (#804): a module `source`d by
    // an entry that already required the package should not be flagged.
    let mut pkg_requires: Vec<String> = analysis
        .package_requires
        .iter()
        .map(|pr| pr.name.clone())
        .collect();
    pkg_requires.extend(inherited_requires.iter().cloned());
    if pkg_requires.is_empty() {
        return analyser_diags;
    }
    let resolver = package_resolver.read().await;
    refine_w120_diagnostics(analyser_diags, &pkg_requires, &resolver, registry)
}

/// Extract the unknown-command name from a W123 message
/// (`"Unknown command 'NAME'"`, optionally `+ "; did you mean 'X'?"`) — the
/// first single-quoted token, the bare name the analyser failed to resolve.
/// Mirrors `tcl_lsp_db`'s private `w123_command`.
fn w123_command_name(message: &str) -> Option<&str> {
    let start = message.find('\'')? + 1;
    let rest = &message[start..];
    let end = rest.find('\'')?;
    Some(&rest[..end])
}

/// The bare (unqualified) names of every command a source file defines — procs
/// and classes — discovered through the analyser's registry-driven
/// symbol-definer walk. The set of *defining* commands (`proc`, `oo::class`,
/// `interp alias`, an ensemble, …) comes from each command spec's
/// [`SymbolDef`](tcl_registry::symbol_def::SymbolDef) in the command registry,
/// never a hand-rolled `proc`-name scan — so a library that defines commands
/// with any registry-known definer is understood the same way. `structure_only`
/// skips diagnostic emission (the dominant cost) while building the identical
/// declaration structure.
fn defined_command_tails(text: &str, dialect: &str) -> Vec<String> {
    let mut analyser = Analyser::new().structure_only();
    let result = analyser.analyse(text, dialect);
    result
        .all_procs
        .values()
        .map(|p| p.name.clone())
        .chain(result.all_classes.values().map(|c| c.name.clone()))
        .filter(|n| !n.is_empty())
        .collect()
}

/// Pure core of the workspace W123 refinement: drop every unknown-command
/// (W123) diagnostic whose command the package database can resolve.
///
/// A command is resolvable when either
/// * the scanned `auto_path` auto-loads it — a `tclIndex` maps its bare name,
///   the "command defined in library path" case of issue #832 (a BLT/Rbc-style
///   library whose procs auto-load with no `package require`), or
/// * one of the packages available to the document (`available`) defines it in
///   an implementation source file (a `pkgIndex`-only package with no
///   `tclIndex`).
///
/// This is the diagnostic dual of go-to-definition: a command the server can
/// resolve to a real library definition must not be reported "unknown". The
/// check is entirely data-driven — the resolver is queried *by the flagged
/// name*, never matched against a hard-coded command list.
fn refine_w123_diagnostics(
    diags: Vec<tcl_compiler::analyser::Diagnostic>,
    available: &[String],
    resolver: &PackageResolver,
    dialect: &str,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    // Command names the document's available packages define via their
    // `pkgIndex` implementation files. Empty in the common no-`package require`
    // case (where auto-load alone carries the fix), so no file is read then.
    let package_commands = if available.is_empty() {
        HashSet::new()
    } else {
        resolver.package_defined_commands(available, &|path| {
            std::fs::read_to_string(path)
                .map(|text| defined_command_tails(&text, dialect))
                .unwrap_or_default()
        })
    };
    diags
        .into_iter()
        .filter(|d| {
            if d.code != DiagCode::W123 {
                return true;
            }
            let Some(name) = w123_command_name(&d.message) else {
                return true;
            };
            // A bare W123 head is a global-namespace call (the analyser skips
            // `::`-qualified heads), so resolve auto-load against `::`.
            !(resolver.auto_loads_command(name, "::") || package_commands.contains(name))
        })
        .collect()
}

/// Apply the issue-#832 workspace W123 refinement to `analyser_diags`: resolve
/// each unknown-command diagnostic against the shared package database and drop
/// any whose command an installed library / available package provides. Shared
/// by the push path ([`refine_and_lift_diagnostics`]) and the pull path
/// ([`Backend::full_diagnostics_for`]) so both stay behaviour-identical, and —
/// like the W120 refinement — always on, independent of the
/// `crossFileResolution` toggle: a library-provided command is ambient, like
/// a built-in, so suppressing its false "unknown" is pure precision, not a
/// cross-file inference the user opts into.
///
/// The common case (no W123) skips the resolver lock and all filesystem work.
async fn refine_workspace_w123(
    analyser_diags: Vec<tcl_compiler::analyser::Diagnostic>,
    analysis: &AnalysisResult,
    inherited_requires: &[String],
    package_resolver: &Arc<RwLock<PackageResolver>>,
    dialect: &str,
) -> Vec<tcl_compiler::analyser::Diagnostic> {
    if !analyser_diags.iter().any(|d| d.code == DiagCode::W123) {
        return analyser_diags;
    }
    // Packages available to the document: its own `package require`s (empty
    // whenever a W123 survived — the analyser drops every W123 once a file has
    // any `package require`) plus those inherited from entry points / `source`
    // ancestors (#804).
    let mut available: Vec<String> = analysis
        .package_requires
        .iter()
        .map(|pr| pr.name.clone())
        .collect();
    available.extend(inherited_requires.iter().cloned());
    let resolver = package_resolver.read().await;
    refine_w123_diagnostics(analyser_diags, &available, &resolver, dialect)
}

/// The extra `package require` names available to `uri` for the #804 W120
/// refinement: from the project's configured entry points when set (which
/// disables auto-detection), else from the workspace `source` graph — every
/// file that transitively `source`s `uri` shares its requires.  Operates on an
/// already-locked index so both the push and pull paths can call it.
fn compute_inherited_requires(
    index: &core_workspace_index::WorkspaceIndex,
    uri: &Uri,
    entry_points: &[String],
    folder_root: Option<&Path>,
) -> Vec<String> {
    if entry_points.is_empty() {
        return index.source_ancestor_package_requires(uri.as_str(), resolve_source_uri);
    }
    // Explicit entry points: the union of their requires, project-wide.
    let mut out: Vec<String> = Vec::new();
    for ep in entry_points {
        if let Some(entry_uri) = entry_point_uri(ep, folder_root) {
            out.extend(index.package_requires_for(&entry_uri));
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Resolve a literal `source` path written in `parent_uri` to the child
/// document's URI string, keyed the same way the workspace index keys
/// documents (`Uri::from_file_path`).  `None` for a non-`file:` URI or an
/// unmappable path.
fn resolve_source_uri(parent_uri: &str, raw_path: &str) -> Option<String> {
    let parent = Uri::from_str(parent_uri).ok()?;
    let parent_path = parent.to_file_path()?;
    let child = tcl_lsp_core::source_graph::resolve_source_target(parent_path.as_ref(), raw_path);
    Uri::from_file_path(&child).map(|u| u.as_str().to_owned())
}

/// [`resolve_source_uri`] extended with the M9 stage-9.2 computed-path tier:
/// a literal resolves as before; a computed path is statically folded through
/// [`tcl_compiler::auto_path_eval::evaluate_auto_path_expr`] (`[file join …]`
/// / `[file dirname [info script]]` forms, with the parent file standing in
/// for `[info script]`) and resolved when the fold succeeds.  Anything the
/// folder cannot prove returns `None` — never a guess.
fn resolve_source_edge(parent_uri: &str, raw_path: &str, is_literal: bool) -> Option<String> {
    if is_literal {
        return resolve_source_uri(parent_uri, raw_path);
    }
    let parent = Uri::from_str(parent_uri).ok()?;
    let parent_path = parent.to_file_path()?;
    let folded =
        tcl_compiler::auto_path_eval::evaluate_auto_path_expr(raw_path, parent_path.to_str())?;
    let child = tcl_lsp_core::source_graph::resolve_source_target(parent_path.as_ref(), &folded);
    Uri::from_file_path(&child).map(|u| u.as_str().to_owned())
}

/// Resolve a configured entry-point path (relative to `folder_root`, or
/// absolute) to its document URI string.
fn entry_point_uri(entry: &str, folder_root: Option<&Path>) -> Option<String> {
    let raw = Path::new(entry);
    let path = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        tcl_lsp_core::source_graph::resolve_under(folder_root?, entry)
    };
    Uri::from_file_path(&path).map(|u| u.as_str().to_owned())
}

/// The package name a W120 says is missing, read from its quick-fix
/// (`package require {pkg}\n`) — the structured, deterministic carrier the
/// analyser emits.
fn w120_required_package(d: &tcl_compiler::analyser::Diagnostic) -> Option<&str> {
    let fix = d.fixes.first()?;
    fix.new_text
        .trim()
        .strip_prefix("package require ")
        .map(str::trim)
}

fn lift_analyser_diagnostics(
    text: &str,
    diagnostics: &[tcl_compiler::analyser::Diagnostic],
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    let line_index = tcl_lexer::LineIndex::new_lsp(text);
    // Default-off codes are suppressed at the analyser via the seeded disabled
    // set (see `default_disabled_set` / `settings_disabled_diagnostics`), so no
    // publish-time filter is needed here — and removing it is what lets
    // `tclLsp.diagnostics.<CODE>: true` actually enable an opt-in code.
    diagnostics
        .iter()
        .cloned()
        .map(|d| tower_lsp_server::ls_types::Diagnostic {
            range: lift_span(text, &line_index, d.span),
            severity: Some(match d.severity {
                tcl_compiler::analyser::Severity::Error => {
                    tower_lsp_server::ls_types::DiagnosticSeverity::ERROR
                }
                tcl_compiler::analyser::Severity::Warning => {
                    tower_lsp_server::ls_types::DiagnosticSeverity::WARNING
                }
                tcl_compiler::analyser::Severity::Info => {
                    tower_lsp_server::ls_types::DiagnosticSeverity::INFORMATION
                }
                tcl_compiler::analyser::Severity::Hint
                | tcl_compiler::analyser::Severity::Suggestion => {
                    tower_lsp_server::ls_types::DiagnosticSeverity::HINT
                }
            }),
            code: Some(tower_lsp_server::ls_types::NumberOrString::String(
                d.code.to_string(),
            )),
            code_description: None,
            source: Some("tcl-lsp".to_string()),
            message: d.message,
            related_information: None,
            tags: None,
            data: None,
        })
        .collect()
}

/// Append the O111 "brace expression performance" hint next to every W100
/// (unbraced-expression) diagnostic: when the optimiser is enabled and O111
/// is not disabled, each W100 gets a paired `Information` hint at the same
/// range suggesting the user brace the expression for bytecode compilation.
fn append_brace_expr_perf_hints(
    diagnostics: &mut Vec<tower_lsp_server::ls_types::Diagnostic>,
    optimiser_enabled: bool,
    opt_disabled: &std::collections::HashSet<String>,
) {
    use tower_lsp_server::ls_types::{Diagnostic, DiagnosticSeverity, NumberOrString};
    if !optimiser_enabled || opt_disabled.contains("O111") {
        return;
    }
    let hints: Vec<Diagnostic> = diagnostics
        .iter()
        .filter(|d| matches!(&d.code, Some(NumberOrString::String(c)) if c == "W100"))
        .map(|w100| Diagnostic {
            range: w100.range,
            severity: Some(DiagnosticSeverity::INFORMATION),
            code: Some(NumberOrString::String("O111".to_string())),
            code_description: None,
            source: Some("tcl-lsp".to_string()),
            message: "Brace expression text (for example, `expr {...}` / `if {...}`) to pass \
                      a single static argument, enabling bytecode compilation and avoiding \
                      per-evaluation substitution/parsing overhead."
                .to_string(),
            related_information: None,
            tags: None,
            data: None,
        })
        .collect();
    diagnostics.extend(hints);
}

/// Apply user severity overrides (`tclLsp.diagnosticSeverity.<CODE>`) to the
/// lifted diagnostics: a code present in `overrides` is re-published at the
/// chosen [`DiagnosticSeverity`], leaving its range/message/code untouched.
/// A no-op when `overrides` is empty (the common case), so the hot path pays
/// nothing. The analyser's emitted severity stands for any code not listed.
fn apply_severity_overrides(
    diagnostics: &mut [tower_lsp_server::ls_types::Diagnostic],
    overrides: &std::collections::HashMap<String, tower_lsp_server::ls_types::DiagnosticSeverity>,
) {
    use tower_lsp_server::ls_types::NumberOrString;
    if overrides.is_empty() {
        return;
    }
    for d in diagnostics.iter_mut() {
        if let Some(NumberOrString::String(code)) = &d.code
            && let Some(&severity) = overrides.get(code)
        {
            d.severity = Some(severity);
        }
    }
}

/// Lift the source-style pass (W111 line length,
/// W112 trailing whitespace, W115 comment continuation, W118 line
/// endings) into LSP diagnostics.  These are pure source-text
/// checks (no analyser / compiler unit needed); see
/// `tcl_lsp_core::source_style`.
///
/// `suppressed` is the analyser's `suppressed_lines` map — it
/// carries both inline `# noqa` line suppressions and the
/// file-level (`-1`) `# tcl-lsp: disable=…` directive set, so the
/// style pass honours the same suppression the analyser diagnostics
/// do.  The checks run with default settings (line length 120,
/// expected line ending `\n`); there is no per-check feature-config
/// surface.
fn lift_source_style_diagnostics(
    text: &str,
    suppressed: &std::collections::HashMap<i32, std::collections::HashSet<String>>,
    user_disabled: &std::collections::HashSet<String>,
    line_length: usize,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    use tcl_lsp_core::source_style::{DEFAULT_LINE_ENDING, StyleSeverity, style_diagnostics};
    use tower_lsp_server::ls_types::{DiagnosticSeverity, NumberOrString};

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
        line_length,
        DEFAULT_LINE_ENDING,
        &disabled,
        suppressed,
    )
    .into_iter()
    .map(|d| tower_lsp_server::ls_types::Diagnostic {
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

/// Lift the compiler-checks pipeline (GVN redundancies,
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
/// twice.  That is acceptable because the analyses themselves
/// dominate the cost.
fn lift_compiler_diagnostics(
    text: &str,
    diags: &tcl_lsp_db::CompilerDiagnostics,
    optimiser_enabled: bool,
    disabled_optimisations: &std::collections::HashSet<String>,
    disabled_diagnostics: &std::collections::HashSet<String>,
    suppressed_lines: &std::collections::HashMap<i32, HashSet<String>>,
) -> Vec<tower_lsp_server::ls_types::Diagnostic> {
    use tcl_compiler::compiler_checks::Severity as CheckSeverity;
    use tower_lsp_server::ls_types::{DiagnosticSeverity, NumberOrString};

    let line_index = tcl_lexer::LineIndex::new_lsp(text);
    let mut out: Vec<tower_lsp_server::ls_types::Diagnostic> = Vec::new();

    // Compiler checks: GVN / shimmer / thunking / taint / iRules-flow / SCCP,
    // all keyed off a single interprocedurally-summarised compilation unit whose
    // per-procedure lattices are memoised by the salsa-native `function_lattice`
    // query (so an unchanged procedure is built once and reused across edits
    // *and* shared with the analyser tail).  The unit is built by the
    // `compiler_check_diagnostics` query; here we only filter + lift.
    // An optimiser O-code (`O1xx`) is gated by the `tclLsp.optimiser.enabled`
    // master switch and the profile + per-code `disabled_optimisations` set,
    // wherever it is emitted (some — e.g. the constant-branch `O100` — come
    // from `run_all_checks` rather than `optimise_with_dialect`).
    let optimiser_suppressed = |code: DiagCode| {
        code.is_optimisation()
            && (!optimiser_enabled || disabled_optimisations.contains(code.as_str()))
    };
    for d in &diags.checks {
        let d = d.clone();
        if optimiser_suppressed(d.code) {
            continue;
        }
        // Per-check feature toggle (`tclLsp.diagnostics.<CODE> = false`).
        // The analyser path bakes the disabled set into its build, but the
        // compiler-checks (S1xx shimmer, T1xx / W2xx taint, IRULE1xxx-5xxx flow,
        // GVN, SCCP constant-branch) come through this separate lift, so the
        // toggle must be applied here too.
        if disabled_diagnostics.contains(d.code.as_str()) {
            continue;
        }
        let range = lift_span(text, &line_index, d.span);
        // Inline `# noqa` / top-of-file suppression. The analyser path bakes
        // `suppressed_lines` into its own build; this separate lift needs the
        // same check applied explicitly — previously missing entirely, so
        // `# noqa: S100` (and every other compiler-check code) had no effect.
        let start_line = i32::try_from(range.start.line).unwrap_or(i32::MAX);
        if line_suppressed(d.code.as_str(), start_line, suppressed_lines) {
            continue;
        }
        out.push(tower_lsp_server::ls_types::Diagnostic {
            range,
            severity: Some(match d.severity {
                CheckSeverity::Error => DiagnosticSeverity::ERROR,
                CheckSeverity::Warning => DiagnosticSeverity::WARNING,
                CheckSeverity::Info => DiagnosticSeverity::INFORMATION,
                CheckSeverity::Hint | CheckSeverity::Suggestion => DiagnosticSeverity::HINT,
            }),
            code: Some(NumberOrString::String(d.code.to_string())),
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
    // fix from the diagnostic). The
    // `tclLsp.optimiser.enabled=false` master switch suppresses the whole
    // block.
    for o in diags
        .optimisations
        .iter()
        .filter(|_| optimiser_enabled)
        .cloned()
    {
        // Profile + per-code gate: the active profile disables whole O-code
        // categories (the default `readability` profile surfaces only
        // readability rewrites) and per-code `tclLsp.optimiser.O1xx=false`
        // overrides add to that.
        if disabled_optimisations.contains(o.code.as_str()) {
            continue;
        }
        let range = lift_span(text, &line_index, o.span);
        // Inline `# noqa` / top-of-file suppression — see the `.checks` loop
        // above for why this was previously missing.
        let start_line = i32::try_from(range.start.line).unwrap_or(i32::MAX);
        if line_suppressed(o.code.as_str(), start_line, suppressed_lines) {
            continue;
        }
        // Surface the fold/rewrite text as the quick-fix `data.replacement`, so
        // editors and the e2e battery can apply the suggested replacement.
        // Never for a `hint_only` optimisation: its span covers the whole
        // consuming statement (there is no precise sub-span to target), so
        // splicing `replacement` in at `[startOffset, endOffset)` would
        // replace the entire statement with a bare fragment — e.g. an O102
        // hint on `set v [expr {$a * 2}]` carries replacement `"5"` and
        // would corrupt the statement into the standalone literal `5` if
        // ever applied. A hint-only diagnostic is informational only; it
        // must never advertise an auto-apply payload.
        let data = (!o.hint_only && !o.replacement.is_empty()).then(|| {
            serde_json::json!({
                "replacement": o.replacement,
                "startOffset": o.span.start(),
                "endOffset": o.span.end(),
            })
        });
        out.push(tower_lsp_server::ls_types::Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::HINT),
            code: Some(NumberOrString::String(o.code.to_string())),
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
fn folder_dialect_for(uri: &Uri, folders: &[(Uri, String)]) -> Option<String> {
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

/// Upper bound on the number of directories the package-database tree scan
/// visits per workspace root, so a huge tree can't stall the package scan.
const WORKSPACE_SCAN_DIR_CAP: usize = 4000;

/// Build the package database (`pkgIndex.tcl` / `tclIndex`) the W120 refinement
/// consults: the workspace trees (recursive — an IDE nests packages
/// arbitrarily) plus the resolved `auto_path` ([`effective_auto_path`]) scanned
/// with C Tcl's immediate-subdir rule. Pure filesystem work, so it runs on the
/// scan worker.
fn build_package_resolver(
    roots: &[PathBuf],
    editor_library_paths: &[String],
    discovered: &[core_tcl_install::TclInstallation],
    dir_cap: usize,
) -> PackageResolver {
    let mut resolver = PackageResolver::new();
    for root in roots {
        resolver.scan_tree(root, dir_cap);
    }
    for dir in effective_auto_path(roots, editor_library_paths, discovered) {
        resolver.scan_path(&dir);
    }
    resolver
}

/// The effective `auto_path` for the package database, layering the configured
/// sources plus on-disk discovery (deduped, in priority order):
///
/// 1. editor `tclLsp.libraryPaths`,
/// 2. user config `config.ini` `[global] libraryPaths`,
/// 3. per-workspace `.tcl-lsp.ini` `[project] libraryPaths`,
/// 4. discovered Tcl installations' `auto_path`,
/// 5. `TCLLIBPATH`.
fn effective_auto_path(
    roots: &[PathBuf],
    editor_library_paths: &[String],
    discovered: &[core_tcl_install::TclInstallation],
) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let push = |p: PathBuf, out: &mut Vec<PathBuf>| {
        if !out.contains(&p) {
            out.push(p);
        }
    };
    for p in editor_library_paths {
        push(PathBuf::from(p), &mut out);
    }
    if let Some(cfg) = core_tcl_install::user_config_path()
        && let Ok(content) = std::fs::read_to_string(&cfg)
    {
        for p in core_tcl_install::library_paths_from_ini(&content, "global") {
            push(PathBuf::from(p), &mut out);
        }
    }
    for root in roots {
        let proj = core_tcl_install::project_config_path(root);
        if let Ok(content) = std::fs::read_to_string(&proj) {
            for p in core_tcl_install::library_paths_from_ini(&content, "project") {
                push(PathBuf::from(p), &mut out);
            }
        }
    }
    for inst in discovered {
        for p in &inst.auto_path {
            push(p.clone(), &mut out);
        }
    }
    for p in core_tcl_install::tcllibpath_dirs() {
        push(p, &mut out);
    }
    out
}

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
/// can usefully index, so the startup workspace scan picks
/// up unopened files — otherwise cross-document definition /
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
/// match `file:///workspace/app2/...`.
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

/// Whether the client advertised pull-diagnostic support
/// (`textDocument.diagnostic`).  Such a client (e.g. `vscode-languageclient`)
/// issues `textDocument/diagnostic` requests itself, so the server must not
/// *also* push diagnostics — pushing and pulling the same set lands them in
/// two diagnostic collections and shows each one twice (#721).
fn client_supports_pull_diagnostics(params: &InitializeParams) -> bool {
    params
        .capabilities
        .text_document
        .as_ref()
        .and_then(|td| td.diagnostic.as_ref())
        .is_some()
}

/// The `workspace/foldingRange/refresh` server→client request (LSP 3.18).
///
/// `ls-types` 0.0.6 predates this method, so it is declared locally to be sent
/// via [`Client::send_request`].  Params and result are both `()` per the spec.
/// Asks the client to re-request folding ranges for all editors — used after a
/// config change flips `features.folding`, since the client otherwise keeps its
/// cached ranges until the next document edit.
enum FoldingRangeRefreshRequest {}

impl tower_lsp_server::ls_types::request::Request for FoldingRangeRefreshRequest {
    type Params = ();
    type Result = ();
    const METHOD: &'static str = "workspace/foldingRange/refresh";
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
        // `ServerCapabilities` (ls-types 0.0.6) carries no
        // `type_hierarchy_provider` field, and dynamic
        // `client/registerCapability` does not surface in the client's
        // `initializeResult.capabilities` (which editors inspect to decide the
        // provider is present).  The type-hierarchy capability is therefore
        // injected into the serialised `initialize` response by the
        // `inject_type_hierarchy_provider` service layer in `main.rs`; the
        // request handlers (`prepare_type_hierarchy` / `supertypes` /
        // `subtypes`) back it here.
        semantic_tokens_provider: Some(semantic_tokens_capability()),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        linked_editing_range_provider: Some(
            tower_lsp_server::ls_types::LinkedEditingRangeServerCapabilities::Simple(true),
        ),
        // Pull-model diagnostics are **opt-in** and intentionally NOT
        // advertised by default: `vscode-languageclient` (and most clients)
        // switch to pull mode the moment `diagnosticProvider` is present, which
        // silently disables our richer push pipeline (`publish_diagnostics`)
        // and makes clients render each diagnostic twice (#721).  The
        // `textDocument/diagnostic` + `workspace/diagnostic` handlers still
        // exist for a client that opts in via `tclLsp.features.pullDiagnostics`
        // (registered dynamically on that path), but the default capability set
        // leaves this absent so push stays the sole delivery channel.
        diagnostic_provider: None,
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
                "tcl-lsp.listTclInstallations".to_owned(),
                "tcl-lsp.setDialect".to_owned(),
                "tcl-lsp.compilerExplorer".to_owned(),
            ],
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),
        // Advertise willRename /
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
/// source extensions (`.tcl` / `.tm` / `.itcl` / `.irule` / `.irul`).
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

/// Build the semantic-tokens capability advertising the classifier's legend,
/// `range` support, and `full` **with** `delta` support.
///
/// The `full/delta` handler ([`Backend::semantic_tokens_full_delta`]) returns a
/// real minimal edit (via [`core_semantic_tokens::diff`]) whenever the client's
/// `previousResultId` matches the last stream we served, so advertising delta
/// is a genuine incremental win for every client that uses it — the same shape
/// rust-analyzer and clangd advertise.
fn semantic_tokens_capability() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions::default(),
        legend: SemanticTokensLegend {
            token_types: core_semantic_tokens::legend_token_types()
                .into_iter()
                .map(tower_lsp_server::ls_types::SemanticTokenType::new)
                .collect(),
            token_modifiers: core_semantic_tokens::legend_token_modifiers()
                .into_iter()
                .map(tower_lsp_server::ls_types::SemanticTokenModifier::new)
                .collect(),
        },
        range: Some(true),
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
fn lift_semantic_token_data(data: &[u32]) -> Vec<tower_lsp_server::ls_types::SemanticToken> {
    let mut tokens: Vec<tower_lsp_server::ls_types::SemanticToken> =
        Vec::with_capacity(data.len() / 5);
    for chunk in data.chunks_exact(5) {
        tokens.push(tower_lsp_server::ls_types::SemanticToken {
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
        // A tcltest case has no dedicated LSP kind; it lists as a function.
        CoreSymbolKind::Function | CoreSymbolKind::Test => SymbolKind::FUNCTION,
        CoreSymbolKind::Method => SymbolKind::METHOD,
        CoreSymbolKind::Class => SymbolKind::CLASS,
        CoreSymbolKind::Property => SymbolKind::PROPERTY,
        CoreSymbolKind::Constructor => SymbolKind::CONSTRUCTOR,
        CoreSymbolKind::Namespace => SymbolKind::NAMESPACE,
        CoreSymbolKind::Variable => SymbolKind::VARIABLE,
        // tcltest constraints / custom-match modes.
        CoreSymbolKind::Constant => SymbolKind::CONSTANT,
        CoreSymbolKind::Operator => SymbolKind::OPERATOR,
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
    use tower_lsp_server::ls_types::{
        PartialResultParams, ReferenceContext, TextDocumentIdentifier, WorkDoneProgressParams,
    };

    // ---- #723 W120 workspace-refinement helpers ----------------------------

    /// A throwaway directory under the system temp dir, removed on drop.
    struct TmpWs(PathBuf);
    impl TmpWs {
        fn new(tag: &str) -> Self {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static C: AtomicUsize = AtomicUsize::new(0);
            let n = C.fetch_add(1, Ordering::Relaxed);
            let mut p = std::env::temp_dir();
            p.push(format!("tcl-lsp-w120-{tag}-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
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

    fn w120_diag(pkg: &str) -> tcl_compiler::analyser::Diagnostic {
        use tcl_compiler::analyser::types::{CodeFix, Severity};
        tcl_compiler::analyser::Diagnostic {
            code: DiagCode::W120,
            span: tcl_lexer::Span::new(0, 1),
            message: format!("\"cmd\" requires `package require {pkg}`"),
            severity: Severity::Warning,
            fixes: vec![CodeFix {
                span: tcl_lexer::Span::new(0, 0),
                new_text: format!("package require {pkg}\n"),
                description: format!("Add 'package require {pkg}'"),
            }],
        }
    }

    fn has_w120(diags: &[tcl_compiler::analyser::Diagnostic]) -> bool {
        diags.iter().any(|d| d.code == DiagCode::W120)
    }

    /// #844 acceptance criterion (a): the progressive fast tier must exclude
    /// exactly the two workspace-refined analyser codes — W120 (missing
    /// `package require`) and W123 (unresolved command) — and nothing else.
    /// Publishing either un-refined would resurface the startup false-positive
    /// W120 that #841's `reschedule_all_open_documents` fix eliminated.
    #[test]
    fn fast_tier_excludes_only_workspace_refined_codes() {
        // The two deferred codes are the whole exclusion set.
        assert!(!is_fast_tier(DiagCode::W120));
        assert!(!is_fast_tier(DiagCode::W123));

        // A representative sweep of the codes the analyser walk produces —
        // syntax errors, structural errors, local arity, style, and variable
        // lints — must all be fast: they are workspace-independent and the deep
        // pass never removes them.  Local arity (E002/E003) is fast; the deep
        // pass only ever *synthesises* additional cross-file arity, never a
        // per-file one the fast tier already showed.
        for code in [
            DiagCode::E002,
            DiagCode::E003,
            DiagCode::W100,
            DiagCode::W111,
            DiagCode::W112,
            DiagCode::W210,
            DiagCode::W211,
            DiagCode::W121,
            DiagCode::W124,
        ] {
            assert!(is_fast_tier(code), "{code} should be in the fast tier");
        }

        // Belt and braces: iterate the entire code catalogue and assert the
        // exclusion set never silently grows to something the deep pass does not
        // actually refine (which would delay a stable diagnostic for no reason).
        for code in DiagCode::ALL {
            let excluded = !is_fast_tier(*code);
            let is_workspace_refined = matches!(code, DiagCode::W120 | DiagCode::W123);
            assert_eq!(
                excluded, is_workspace_refined,
                "{code}: fast-tier exclusion must match exactly the W120/W123 \
                 workspace-refined set",
            );
        }
    }

    #[test]
    fn is_config_file_recognises_project_and_user_config() {
        assert!(is_config_file(
            &Uri::from_str("file:///ws/.tcl-lsp.ini").unwrap()
        ));
        assert!(is_config_file(
            &Uri::from_str("file:///home/me/.config/tcl-lsp/config.ini").unwrap()
        ));
        assert!(!is_config_file(
            &Uri::from_str("file:///ws/foo.tcl").unwrap()
        ));
        assert!(!is_config_file(
            &Uri::from_str("file:///ws/settings.ini").unwrap()
        ));
    }

    #[tokio::test]
    async fn shimmer_disabled_folds_shimmer_family_into_disabled() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///s.tcl").unwrap();
        // Default (shimmer on): no S-codes forced into the disabled set.
        let (disabled, ..) = backend.resolved_analysis_settings(&uri).await;
        assert!(!disabled.contains("S100"));
        // `tclLsp.shimmer.enabled = false` applies and forces the family off.
        backend
            .apply_global_config(&serde_json::json!({ "shimmer": { "enabled": false } }))
            .await;
        assert!(!*backend.shimmer_enabled.lock().await);
        let (disabled, ..) = backend.resolved_analysis_settings(&uri).await;
        for code in ["S100", "S101", "S102", "S103", "S110"] {
            assert!(disabled.contains(code), "{code} should be suppressed");
        }
    }

    #[test]
    fn collapse_inlay_alias_resolves_per_layer() {
        // Alias-only: `inlayHints` becomes `inlayTypeHints`.
        let mut alias_only = serde_json::json!({ "features": { "inlayHints": true } });
        collapse_inlay_alias(&mut alias_only);
        assert_eq!(
            alias_only["features"]["inlayTypeHints"],
            serde_json::json!(true)
        );
        assert!(alias_only["features"].get("inlayHints").is_none());

        // Both present in one layer: the explicit `inlayTypeHints` wins.
        let mut both = serde_json::json!({
            "features": { "inlayHints": true, "inlayTypeHints": false }
        });
        collapse_inlay_alias(&mut both);
        assert_eq!(both["features"]["inlayTypeHints"], serde_json::json!(false));
        assert!(both["features"].get("inlayHints").is_none());

        // Neither: untouched (negative case).
        let mut neither = serde_json::json!({ "features": { "codeActions": true } });
        collapse_inlay_alias(&mut neither);
        assert!(neither["features"].get("inlayTypeHints").is_none());
    }

    #[test]
    fn inlay_alias_in_editor_layer_beats_global_inlay_type_hints() {
        // Regression (#728): the global `config.ini` layer (e.g. written by
        // `exportConfig`) carries an explicit `inlayTypeHints: false`, while the
        // higher-precedence editor layer sets only the legacy `inlayHints`
        // alias. After per-layer collapse the editor must win → type hints on.
        let mut global = serde_json::json!({
            "features": { "inlayTypeHints": false, "inlayParameterHints": false, "codeActions": true }
        });
        let mut editor = serde_json::json!({ "features": { "inlayHints": true } });
        collapse_inlay_alias(&mut global);
        collapse_inlay_alias(&mut editor);
        let merged = config_ini::merge_settings(&global, &editor);
        assert_eq!(
            merged["features"]["inlayTypeHints"],
            serde_json::json!(true),
            "editor inlayHints alias must beat global inlayTypeHints:false",
        );
        // Parameter hints (only in the global layer) stay off; no stray alias key.
        assert_eq!(
            merged["features"]["inlayParameterHints"],
            serde_json::json!(false)
        );
        assert!(merged["features"].get("inlayHints").is_none());
    }

    #[test]
    fn normalize_config_payload_handles_nested_flat_and_unwrapped() {
        // Nested `tclLsp` object.
        let nested = serde_json::json!({ "tclLsp": { "optimiser": { "O109": false } } });
        assert_eq!(
            normalize_config_payload(&nested),
            serde_json::json!({ "optimiser": { "O109": false } }),
        );
        // Flat dotted keys (with and without the `tclLsp.` prefix).
        let flat = serde_json::json!({
            "tclLsp.optimiser.O109": false,
            "tclLsp.dialect": "tcl8.6",
        });
        assert_eq!(
            normalize_config_payload(&flat),
            serde_json::json!({ "optimiser": { "O109": false }, "dialect": "tcl8.6" }),
        );
        // Unwrapped top-level sections (e.g. JetBrains).
        let unwrapped =
            serde_json::json!({ "optimiser": { "enabled": false }, "dialect": "tcl9.0" });
        assert_eq!(
            normalize_config_payload(&unwrapped),
            serde_json::json!({ "optimiser": { "enabled": false }, "dialect": "tcl9.0" }),
        );
        // Composed: nested + flat merge key-by-key.
        let composed = serde_json::json!({
            "tclLsp": { "optimiser": { "O109": false } },
            "tclLsp.optimiser.O110": false,
        });
        assert_eq!(
            normalize_config_payload(&composed),
            serde_json::json!({ "optimiser": { "O109": false, "O110": false } }),
        );
    }

    #[test]
    fn normalize_config_payload_scalar_object_collision_does_not_panic() {
        // RUST_ISSUE_032: a flat scalar key that collides with a deeper dotted
        // key in the same payload must not panic the server; the nested
        // structure wins over the conflicting scalar.
        let collide = serde_json::json!({
            "tclLsp.optimiser": true,
            "tclLsp.optimiser.enabled": false,
        });
        assert_eq!(
            normalize_config_payload(&collide),
            serde_json::json!({ "optimiser": { "enabled": false } }),
        );
        // The reverse spelling (scalar under a nested prefix) also survives.
        let collide2 = serde_json::json!({
            "tclLsp": { "style": "x" },
            "tclLsp.style.nonAscii": "escape",
        });
        assert_eq!(
            normalize_config_payload(&collide2),
            serde_json::json!({ "style": { "nonAscii": "escape" } }),
        );
    }

    #[test]
    fn normalize_config_payload_survives_mixed_shape_collision() {
        // A client that supplies both a scalar prefix and a dotted child at the
        // same segment (`tclLsp.features` = false *and*
        // `tclLsp.features.semanticTokens` = true) must not panic; the deeper
        // key replaces the colliding scalar rather than crashing the server.
        // (`serde_json`'s default `Map` orders keys, so `tclLsp.features` is
        // folded before its dotted child regardless of literal order.)
        let payload = serde_json::json!({
            "tclLsp.features": false,
            "tclLsp.features.semanticTokens": true,
        });
        assert_eq!(
            normalize_config_payload(&payload),
            serde_json::json!({ "features": { "semanticTokens": true } }),
        );
        // When the prefix is already an object, the dotted child merges into it
        // (no collision, both keys kept) — the non-panicking path we must not
        // regress.
        let obj_prefix = serde_json::json!({
            "tclLsp.features": { "hover": true },
            "tclLsp.features.semanticTokens": true,
        });
        assert_eq!(
            normalize_config_payload(&obj_prefix),
            serde_json::json!({ "features": { "hover": true, "semanticTokens": true } }),
        );
    }

    #[tokio::test]
    async fn generic_variable_patterns_apply_through_global_config() {
        let backend = test_backend();
        // Default: unset (built-in IRULE4002 pattern set).
        assert!(backend.generic_variable_patterns.lock().await.is_none());
        // `tclLsp.diagnostics.genericVariablePatterns` replaces the default set.
        backend
            .apply_global_config(&serde_json::json!({
                "diagnostics": { "genericVariablePatterns": ["^myapp_"] }
            }))
            .await;
        assert_eq!(
            *backend.generic_variable_patterns.lock().await,
            Some(vec!["^myapp_".to_owned()]),
        );
        // The salsa `AnalyserConfig` input mirrors the value.
        {
            let db = backend.db.lock().await;
            let cfg = *backend.db_config.lock().await;
            assert_eq!(
                cfg.generic_variable_patterns(&*db).as_deref(),
                Some(["^myapp_".to_owned()].as_slice()),
            );
        }
        // An explicit empty list disables the check (Some(empty), not None).
        backend
            .apply_global_config(&serde_json::json!({
                "diagnostics": { "genericVariablePatterns": [] }
            }))
            .await;
        assert_eq!(
            *backend.generic_variable_patterns.lock().await,
            Some(Vec::new()),
        );
    }

    #[tokio::test]
    async fn config_file_settings_apply_through_global_config() {
        // A `config.ini`-shaped file, read and applied through the same path as
        // editor settings, takes effect.
        let backend = test_backend();
        let ws = TmpWs::new("cfgfile");
        let path = ws.0.join("config.ini");
        std::fs::write(
            &path,
            "[global]\ndialect = tcl9.0\n[optimiser]\nenabled = false\n",
        )
        .unwrap();
        let global = read_ini_layer(Some(path), config_ini::Layer::Global);
        backend.apply_global_config(&global).await;
        assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
        assert!(!*backend.optimiser_enabled.lock().await);
    }

    #[tokio::test]
    async fn config_precedence_project_over_editor_over_global() {
        // global config.ini < editor < project .tcl-lsp.ini.
        let backend = test_backend();
        let global = config_ini::settings_from_ini(
            "[global]\ndialect = tcl8.5\n",
            config_ini::Layer::Global,
        );
        let editor = serde_json::json!({ "dialect": "tcl8.6" });
        let project = config_ini::settings_from_ini(
            "[project]\ndialect = tcl9.0\n",
            config_ini::Layer::Project,
        );
        let merged =
            config_ini::merge_settings(&config_ini::merge_settings(&global, &editor), &project);
        backend.apply_global_config(&merged).await;
        assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
    }

    #[test]
    fn effective_auto_path_layers_editor_project_and_discovery() {
        // editor libraryPaths → global → project .tcl-lsp.ini → discovered →
        // TCLLIBPATH, deduped and in priority order.
        let ws = TmpWs::new("ap");
        ws.write(".tcl-lsp.ini", "[project]\nlibraryPaths = /proj/lib\n");
        let editor = vec!["/editor/lib".to_owned()];
        let discovered = vec![core_tcl_install::TclInstallation {
            version: "8.6".to_owned(),
            tcl_library: PathBuf::from("/sys/lib/tcl8.6"),
            auto_path: vec![PathBuf::from("/sys/lib"), PathBuf::from("/sys/lib/tcl8.6")],
        }];
        let ap = effective_auto_path(std::slice::from_ref(&ws.0), &editor, &discovered);
        assert!(ap.contains(&PathBuf::from("/editor/lib")));
        assert!(ap.contains(&PathBuf::from("/proj/lib")));
        assert!(ap.contains(&PathBuf::from("/sys/lib")));
        let pos = |p: &str| ap.iter().position(|x| x == &PathBuf::from(p)).unwrap();
        assert!(pos("/editor/lib") < pos("/proj/lib"));
        assert!(pos("/proj/lib") < pos("/sys/lib"));
    }

    #[test]
    fn build_package_resolver_picks_up_a_discovered_installation_package() {
        // A discovered installation whose `auto_path` holds a package's
        // pkgIndex.tcl is scanned, so the database can resolve it — the
        // mechanism that lets W120 refinement see system-installed Tk.
        let sys = TmpWs::new("sys");
        sys.write(
            "lib/tk8.6/pkgIndex.tcl",
            "package ifneeded Tk 8.6 [list load [file join $dir libtk8.6.so] Tk]\n",
        );
        // The package has no source file (a C extension loaded via `load`); the
        // fallback lists the dir's *.tcl — none here, so give it a marker .tcl.
        sys.write("lib/tk8.6/tk.tcl", "# tk\n");
        let lib_root = sys.0.join("lib");
        let discovered = vec![core_tcl_install::TclInstallation {
            version: "8.6".to_owned(),
            tcl_library: lib_root.join("tcl8.6"),
            auto_path: vec![lib_root.clone()],
        }];
        let resolver = build_package_resolver(&[], &[], &discovered, 100);
        assert!(
            resolver.provides("Tk"),
            "discovered install's Tk package should be in the database"
        );
    }

    #[test]
    fn refine_w120_conservative_drops_all_for_unknowable_require() {
        // `package require myTkPackage` is unknown to both the registry and an
        // empty package database, so it may load anything — drop the W120.
        let registry = CommandRegistry::build_default();
        let resolver = PackageResolver::new();
        let out = refine_w120_diagnostics(
            vec![w120_diag("Tk")],
            &["myTkPackage".to_owned()],
            &resolver,
            &registry,
        );
        assert!(
            !has_w120(&out),
            "unknowable require ⇒ W120 dropped: {out:?}"
        );
    }

    #[test]
    fn refine_w120_suppressed_when_wrapper_transitively_requires_tk() {
        // The precise #723 case: a workspace package whose implementation does
        // `package require Tk` makes Tk available, so the Tk W120 is a false
        // positive.
        let ws = TmpWs::new("wrap");
        ws.write(
            "mytk/mytk.tcl",
            "package provide myTkPackage 1.0\npackage require Tk\nproc mytk::go {} {}\n",
        );
        ws.write(
            "mytk/pkgIndex.tcl",
            "package ifneeded myTkPackage 1.0 [list source [file join $dir mytk.tcl]]\n",
        );
        let mut resolver = PackageResolver::new();
        resolver.scan_tree(&ws.0, 100);
        let registry = CommandRegistry::build_default();
        let out = refine_w120_diagnostics(
            vec![w120_diag("Tk")],
            &["myTkPackage".to_owned()],
            &resolver,
            &registry,
        );
        assert!(
            !has_w120(&out),
            "wrapper transitively requires Tk ⇒ W120 suppressed: {out:?}"
        );
    }

    #[test]
    fn refine_w120_kept_when_resolvable_require_does_not_provide_tk() {
        // A resolvable package that does NOT pull in Tk leaves the Tk W120
        // standing — the refinement is precise, not a blanket suppression.
        let ws = TmpWs::new("plain");
        ws.write(
            "plain/plain.tcl",
            "package provide plain 1.0\nproc plain::p {} {}\n",
        );
        ws.write(
            "plain/pkgIndex.tcl",
            "package ifneeded plain 1.0 [list source [file join $dir plain.tcl]]\n",
        );
        let mut resolver = PackageResolver::new();
        resolver.scan_tree(&ws.0, 100);
        let registry = CommandRegistry::build_default();
        let out = refine_w120_diagnostics(
            vec![w120_diag("Tk")],
            &["plain".to_owned()],
            &resolver,
            &registry,
        );
        assert!(
            has_w120(&out),
            "plain doesn't provide Tk ⇒ W120 kept: {out:?}"
        );
    }

    // ---- #804 W120 entry-point / source-graph inheritance ------------------

    fn ws_index(docs: &[(&Uri, &str)]) -> core_workspace_index::WorkspaceIndex {
        let analyses: Vec<(String, tcl_compiler::analyser::AnalysisResult)> = docs
            .iter()
            .map(|(uri, src)| {
                let mut a = tcl_compiler::analyser::Analyser::new();
                ((*uri).as_str().to_owned(), a.analyse(src, "tcl8.6").clone())
            })
            .collect();
        core_workspace_index::WorkspaceIndex::from_documents(
            analyses.iter().map(|(u, a)| (u.as_str(), a)),
        )
    }

    #[test]
    fn source_uri_resolution_matches_from_file_path() {
        // The resolver must produce the exact URI string the index keys the
        // child document by, or the graph edge won't connect.
        let app = Uri::from_file_path("/proj/app.tcl").unwrap();
        let child = resolve_source_uri(app.as_str(), "lib/util.tcl").unwrap();
        let expected = Uri::from_file_path("/proj/lib/util.tcl").unwrap();
        assert_eq!(child, expected.as_str());
    }

    #[test]
    fn auto_source_graph_inherits_entry_requires() {
        // app.tcl requires Tk and sources lib/util.tcl; util inherits Tk, so a
        // Tk W120 in util is suppressed with no configuration at all.
        let app = Uri::from_file_path("/proj/app.tcl").unwrap();
        let util = Uri::from_file_path("/proj/lib/util.tcl").unwrap();
        let index = ws_index(&[
            (&app, "package require Tk\nsource lib/util.tcl\n"),
            (&util, "proc u {} {}\n"),
        ]);
        let inherited = compute_inherited_requires(&index, &util, &[], None);
        assert_eq!(inherited, vec!["Tk".to_owned()]);
        // The entry file itself inherits nothing.
        assert!(compute_inherited_requires(&index, &app, &[], None).is_empty());
    }

    #[test]
    fn explicit_entry_points_override_source_graph() {
        // main.tcl requires Tk but does NOT source other.tcl; with main.tcl set
        // as an entry point, other.tcl still inherits Tk (project-wide), and the
        // auto source-graph is bypassed entirely.
        let root = PathBuf::from("/proj");
        let main = Uri::from_file_path("/proj/main.tcl").unwrap();
        let other = Uri::from_file_path("/proj/other.tcl").unwrap();
        let index = ws_index(&[(&main, "package require Tk\n"), (&other, "proc o {} {}\n")]);
        // Auto mode: other.tcl is not sourced by main, so it inherits nothing.
        assert!(compute_inherited_requires(&index, &other, &[], Some(&root)).is_empty());
        // Explicit entry point: other.tcl inherits main.tcl's Tk.
        let entries = vec!["main.tcl".to_owned()];
        let inherited = compute_inherited_requires(&index, &other, &entries, Some(&root));
        assert_eq!(inherited, vec!["Tk".to_owned()]);
    }

    #[test]
    fn entry_point_paths_resolve_relative_and_absolute() {
        let root = PathBuf::from("/proj");
        let rel = entry_point_uri("src/main.tcl", Some(&root)).unwrap();
        assert_eq!(
            rel,
            Uri::from_file_path("/proj/src/main.tcl").unwrap().as_str()
        );
        let abs = entry_point_uri("/opt/app.tcl", Some(&root)).unwrap();
        assert_eq!(abs, Uri::from_file_path("/opt/app.tcl").unwrap().as_str());
        // A relative entry with no folder root can't be resolved.
        assert!(entry_point_uri("main.tcl", None).is_none());
    }

    #[test]
    fn folder_config_parses_entry_points() {
        let cfg = serde_json::json!({
            "entryPoints": ["main.tcl", "src/app.tcl"],
        });
        let fc = parse_folder_config(&cfg).unwrap();
        assert_eq!(
            fc.entry_points,
            Some(vec!["main.tcl".to_owned(), "src/app.tcl".to_owned()])
        );
    }

    #[test]
    fn default_off_codes_are_seeded_into_the_disabled_set() {
        // The opt-in default-off codes (e.g. W242) start in the resolved
        // disabled set, so the analyser suppresses them by default.
        assert!(default_disabled_set().contains("W242"));
        // An empty config keeps the default-off seed.
        let none = settings_disabled_diagnostics(&serde_json::json!({}));
        assert!(none.is_none(), "no diagnostics section ⇒ inherit default");
        // A `false` for some other code keeps W242 disabled too.
        let with_false = settings_disabled_diagnostics(
            &serde_json::json!({ "tclLsp": { "diagnostics": { "W111": false } } }),
        )
        .expect("set");
        assert!(with_false.contains("W242"), "W242 stays default-off");
        assert!(with_false.contains("W111"));
        // `tclLsp.diagnostics.W242: true` enables it (removes from disabled).
        let enabled = settings_disabled_diagnostics(
            &serde_json::json!({ "tclLsp": { "diagnostics": { "W242": true } } }),
        )
        .expect("set");
        assert!(
            !enabled.contains("W242"),
            "W242 enabled via config: {enabled:?}"
        );
    }

    #[tokio::test]
    async fn default_off_w242_hidden_by_default_enableable_via_config() {
        // End-to-end: a default-off W242 is not published by default, but
        // `tclLsp.diagnostics.W242: true` turns it on (the server has a
        // per-code enable path).
        let backend = test_backend();
        let uri = Uri::from_str("file:///w242.tcl").unwrap();
        let src = "while {$x < 10} {puts hi}\n"; // emits W242 from the analyser
        register(&backend, &uri, src).await;

        let off = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            !off.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W242"
            )),
            "W242 hidden by default: {off:?}",
        );

        backend
            .apply_global_config(&serde_json::json!({ "diagnostics": { "W242": true } }))
            .await;
        let on = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            on.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W242"
            )),
            "W242 should appear once enabled: {on:?}",
        );
    }

    /// The constant-true `if` is folded by SCCP and surfaced
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
            &tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, "", None),
            true,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
            &std::collections::HashMap::new(),
        );
        assert!(
            diags.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "O100"
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
        let is_o100 = |d: &tower_lsp_server::ls_types::Diagnostic| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "O100");
        // Master switch off: no optimiser O-codes at all (compiler checks still run).
        let off = lift_compiler_diagnostics(
            src,
            &tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, "", None),
            false,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
            &std::collections::HashMap::new(),
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
            &tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, "", None),
            true,
            &disabled,
            &std::collections::HashSet::new(),
            &std::collections::HashMap::new(),
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
        let registry = tcl_registry::registry_for_profile(tcl_dialect::DialectProfile::irules());
        let src = "set u [HTTP::uri]\nHTTP::respond 200 content $u\n";
        let cdiags =
            tcl_lsp_db::compiler_check_diagnostics_uncached(src, registry, "f5-irules", None);
        let diags = lift_compiler_diagnostics(
            src,
            &cdiags,
            true,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
            &std::collections::HashMap::new(),
        );
        assert!(
            diags.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "IRULE3001"
            )),
            "expected IRULE3001, got: {:?}",
            diags.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
    }

    /// A per-check feature toggle (`tclLsp.diagnostics.<CODE> = false`)
    /// must suppress a compiler-*check* code — not just the analyser families.
    /// IRULE3001 comes through the compiler-checks lift, so disabling it via the
    /// `disabled_diagnostics` set must drop it from the published set while
    /// leaving other codes untouched.
    #[test]
    fn lift_compiler_diagnostics_honours_per_check_disable() {
        let registry = tcl_registry::registry_for_profile(tcl_dialect::DialectProfile::irules());
        let src = "set u [HTTP::uri]\nHTTP::respond 200 content $u\n";
        let cdiags =
            tcl_lsp_db::compiler_check_diagnostics_uncached(src, registry, "f5-irules", None);
        let is_irule3001 = |d: &tower_lsp_server::ls_types::Diagnostic| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "IRULE3001");
        // Baseline: IRULE3001 is present with no disabled codes.
        let baseline = lift_compiler_diagnostics(
            src,
            &cdiags,
            true,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
            &std::collections::HashMap::new(),
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
            &std::collections::HashMap::new(),
        );
        assert!(
            !filtered.iter().any(is_irule3001),
            "IRULE3001 must be suppressed when disabled per-check: {:?}",
            filtered.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );
    }

    /// `# noqa: S100` on the line before the shimmering command must
    /// suppress it through the live compiler-checks lift — previously
    /// `lift_compiler_diagnostics` never consulted `suppressed_lines` at
    /// all, so `# noqa` had no effect on any compiler-check code (S1xx
    /// shimmer, T1xx taint, IRULE1xxx-5xxx, O1xx, GVN, SCCP).
    #[test]
    fn lift_compiler_diagnostics_honours_inline_noqa_suppression() {
        let registry = CommandRegistry::build_default();
        let src = "set x hello\nincr x\n";
        let is_s100 = |d: &tower_lsp_server::ls_types::Diagnostic| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "S100");

        // Baseline: S100 fires with no suppression.
        let baseline = lift_compiler_diagnostics(
            src,
            &tcl_lsp_db::compiler_check_diagnostics_uncached(src, &registry, "", None),
            true,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
            &std::collections::HashMap::new(),
        );
        assert!(baseline.iter().any(is_s100), "expected S100 baseline");

        // `# noqa: S100` on the line before `incr x` suppresses it.
        let suppressed_src = "set x hello\n# noqa: S100\nincr x\n";
        let suppressed_lines = Analyser::new()
            .analyse(suppressed_src, "tcl8.6")
            .suppressed_lines
            .clone();
        let filtered = lift_compiler_diagnostics(
            suppressed_src,
            &tcl_lsp_db::compiler_check_diagnostics_uncached(suppressed_src, &registry, "", None),
            true,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
            &suppressed_lines,
        );
        assert!(
            !filtered.iter().any(is_s100),
            "S100 must be suppressed by a preceding '# noqa: S100', got: {:?}",
            filtered.iter().map(|d| d.code.clone()).collect::<Vec<_>>(),
        );

        // TN control: a `# noqa` for an unrelated code must not incidentally
        // suppress S100.
        let unrelated_src = "set x hello\n# noqa: W999\nincr x\n";
        let unrelated_suppressed = Analyser::new()
            .analyse(unrelated_src, "tcl8.6")
            .suppressed_lines
            .clone();
        let unfiltered = lift_compiler_diagnostics(
            unrelated_src,
            &tcl_lsp_db::compiler_check_diagnostics_uncached(unrelated_src, &registry, "", None),
            true,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
            &unrelated_suppressed,
        );
        assert!(
            unfiltered.iter().any(is_s100),
            "S100 must still fire when the preceding noqa names an unrelated code: {:?}",
            unfiltered
                .iter()
                .map(|d| d.code.clone())
                .collect::<Vec<_>>(),
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
    fn semantic_tokens_capability_advertises_delta_and_range() {
        use tower_lsp_server::ls_types::SemanticTokensServerCapabilities as Cap;
        let Cap::SemanticTokensOptions(o) = semantic_tokens_capability() else {
            panic!("expected SemanticTokensOptions");
        };
        assert_eq!(o.range, Some(true));
        assert!(matches!(
            o.full,
            Some(SemanticTokensFullOptions::Delta { delta: Some(true) })
        ));
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
    fn settings_severity_overrides_nested_and_flat() {
        use tower_lsp_server::ls_types::DiagnosticSeverity;
        // Nested shape: recognised values map (case-insensitively); "default"
        // and unknown values mean "no override" and are skipped.
        let nested = serde_json::json!({
            "tclLsp": {"diagnosticSeverity": {
                "W211": "warning",
                "W220": "Error",
                "W210": "info",
                "W214": "default",
                "W111": "loud",
            }}
        });
        let got = settings_severity_overrides(&nested).unwrap();
        assert_eq!(got.get("W211"), Some(&DiagnosticSeverity::WARNING));
        assert_eq!(got.get("W220"), Some(&DiagnosticSeverity::ERROR));
        assert_eq!(got.get("W210"), Some(&DiagnosticSeverity::INFORMATION));
        assert!(!got.contains_key("W214"), "'default' must not override");
        assert!(!got.contains_key("W111"), "unknown value must be skipped");
        // Flat-dotted shape.
        let flat = serde_json::json!({
            "tclLsp.diagnosticSeverity.W211": "hint",
            "tclLsp.diagnosticSeverity.S100": "warning",
        });
        let got = settings_severity_overrides(&flat).unwrap();
        assert_eq!(got.get("W211"), Some(&DiagnosticSeverity::HINT));
        assert_eq!(got.get("S100"), Some(&DiagnosticSeverity::WARNING));
        // No diagnosticSeverity section -> None (leave current map untouched);
        // an explicit empty section -> Some(empty) (clear all overrides).
        assert!(settings_severity_overrides(&serde_json::json!({"x": 1})).is_none());
        let cleared =
            settings_severity_overrides(&serde_json::json!({"tclLsp": {"diagnosticSeverity": {}}}))
                .unwrap();
        assert!(cleared.is_empty());
    }

    #[test]
    fn apply_severity_overrides_relabels_only_listed_codes() {
        use tower_lsp_server::ls_types::{
            Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range,
        };
        let mk = |code: &str, sev: DiagnosticSeverity| Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
            severity: Some(sev),
            code: Some(NumberOrString::String(code.to_owned())),
            code_description: None,
            source: Some("tcl-lsp".to_owned()),
            message: String::new(),
            related_information: None,
            tags: None,
            data: None,
        };
        let mut diags = vec![
            mk("W211", DiagnosticSeverity::HINT),
            mk("W220", DiagnosticSeverity::HINT),
        ];
        let overrides =
            std::collections::HashMap::from([("W211".to_owned(), DiagnosticSeverity::WARNING)]);
        apply_severity_overrides(&mut diags, &overrides);
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::WARNING));
        // A code not in the map keeps its emitted severity.
        assert_eq!(diags[1].severity, Some(DiagnosticSeverity::HINT));
        // An empty map is a no-op.
        let before = diags.clone();
        apply_severity_overrides(&mut diags, &std::collections::HashMap::new());
        assert_eq!(diags, before);
    }

    #[test]
    fn position_encoding_negotiation() {
        use tower_lsp_server::ls_types::{ClientCapabilities, GeneralClientCapabilities};
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
    fn pull_diagnostic_capability_detection() {
        use tower_lsp_server::ls_types::{
            ClientCapabilities, DiagnosticClientCapabilities, TextDocumentClientCapabilities,
        };
        // No `textDocument.diagnostic` → push-only client (no pull).
        assert!(!client_supports_pull_diagnostics(
            &InitializeParams::default()
        ));
        // A client advertising `textDocument/diagnostic` support → pull-capable,
        // so the worker must stop pushing to avoid the #721 double-display.
        let pull_params = InitializeParams {
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    diagnostic: Some(DiagnosticClientCapabilities::default()),
                    ..TextDocumentClientCapabilities::default()
                }),
                ..ClientCapabilities::default()
            },
            ..InitializeParams::default()
        };
        assert!(client_supports_pull_diagnostics(&pull_params));
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
        // handler's default-off gate).
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
        // The retired `inlayHints` key maps to `inlayTypeHints`: an existing
        // explicit opt-in keeps showing the
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
    fn list_irule_events_command_returns_sorted_known_events() {
        let out = Backend::list_irule_events_command();
        let names: Vec<&str> = out
            .get("events")
            .and_then(|v| v.as_array())
            .expect("events array")
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();
        assert!(names.contains(&"HTTP_REQUEST"), "{names:?}");
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "events must be sorted");
    }

    #[test]
    fn diagram_data_command_extracts_when_events() {
        let src = "when HTTP_REQUEST {\n}\nwhen CLIENT_ACCEPTED {\n}\n";
        let out = Backend::diagram_data_command(&[serde_json::json!(src)]).expect("some");
        let events: Vec<&str> = out
            .get("events")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .filter_map(|e| e.get("name").and_then(serde_json::Value::as_str))
            .collect();
        assert!(events.contains(&"HTTP_REQUEST"), "{events:?}");
        assert!(events.contains(&"CLIENT_ACCEPTED"), "{events:?}");
        // No string argument → None (not an empty result).
        assert!(Backend::diagram_data_command(&[]).is_none());
    }

    #[tokio::test]
    async fn describe_irule_event_command_reports_known_and_unknown() {
        let backend = test_backend();
        let known = backend
            .describe_irule_event_command(&[serde_json::json!("HTTP_REQUEST")])
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(
            known.get("known").and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert!(
            known
                .get("validCommandCount")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                > 0,
            "a known event should report valid commands: {known:?}",
        );
        let unknown = backend
            .describe_irule_event_command(&[serde_json::json!("NOPE_EVENT")])
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(
            unknown.get("known").and_then(serde_json::Value::as_bool),
            Some(false),
        );
    }

    #[tokio::test]
    async fn describe_irule_command_command_resolves_case_insensitively() {
        let backend = test_backend();
        let found = backend
            .describe_irule_command_command(&[serde_json::json!("HTTP::header")])
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(
            found.get("found").and_then(serde_json::Value::as_bool),
            Some(true)
        );
        let canonical = found.get("command").and_then(serde_json::Value::as_str);
        // A differently-cased spelling resolves to the same canonical command.
        let ci = backend
            .describe_irule_command_command(&[serde_json::json!("http::HEADER")])
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(
            ci.get("found").and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            ci.get("command").and_then(serde_json::Value::as_str),
            canonical
        );
        // An unknown command reports `found: false`.
        let missing = backend
            .describe_irule_command_command(&[serde_json::json!("NoSuchCmd")])
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(
            missing.get("found").and_then(serde_json::Value::as_bool),
            Some(false)
        );
    }

    #[tokio::test]
    async fn list_subcommands_command_returns_sorted_subcommands() {
        let backend = test_backend();
        let out = backend
            .list_subcommands_command(&[serde_json::json!("string")])
            .await
            .expect("some");
        assert_eq!(
            out.get("command").and_then(serde_json::Value::as_str),
            Some("string")
        );
        let subs: Vec<&str> = out
            .get("subcommands")
            .and_then(|v| v.as_array())
            .unwrap()
            .iter()
            .filter_map(|s| s.get("name").and_then(serde_json::Value::as_str))
            .collect();
        assert!(subs.contains(&"length"), "string subcommands: {subs:?}");
        let mut sorted = subs.clone();
        sorted.sort_unstable();
        assert_eq!(subs, sorted, "subcommands must be sorted");
    }

    #[tokio::test]
    async fn render_config_ini_emits_known_sections() {
        let backend = test_backend();
        let ini = backend.render_config_ini().await;
        for section in ["[features]", "[optimiser]", "[style]"] {
            assert!(ini.contains(section), "missing {section} in:\n{ini}");
        }
    }

    #[tokio::test]
    async fn minify_document_command_returns_source_and_handles_missing_doc() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///m.tcl").unwrap();
        register(&backend, &uri, "set x 1\nputs $x\n").await;
        let out = backend
            .minify_document_command(&[serde_json::json!(uri.as_str())])
            .await
            .expect("ok")
            .expect("some");
        assert!(
            out.get("source")
                .and_then(serde_json::Value::as_str)
                .is_some(),
            "minify result should carry a source string: {out:?}",
        );
        // No argument → None; an unregistered URI → None.
        assert!(
            backend
                .minify_document_command(&[])
                .await
                .expect("ok")
                .is_none()
        );
        assert!(
            backend
                .minify_document_command(&[serde_json::json!("file:///nope.tcl")])
                .await
                .expect("ok")
                .is_none(),
        );
    }

    #[tokio::test]
    async fn optimise_document_command_returns_source_for_registered_doc() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///o.tcl").unwrap();
        register(&backend, &uri, "set x [expr {1 + 2}]\nputs $x\n").await;
        let out = backend
            .optimise_document_command(&[serde_json::json!(uri.as_str())])
            .await
            .expect("ok")
            .expect("some");
        assert!(
            out.get("source")
                .and_then(serde_json::Value::as_str)
                .is_some(),
            "optimise result should carry a source string: {out:?}",
        );
        assert!(
            backend
                .optimise_document_command(&[])
                .await
                .expect("ok")
                .is_none()
        );
    }

    #[test]
    fn unminify_error_command_echoes_original_and_flags_change() {
        // No argument → None.
        assert!(Backend::unminify_error_command(&[]).is_none());
        let out = Backend::unminify_error_command(&[serde_json::json!("oops at a")]).expect("some");
        assert_eq!(
            out.get("originalError").and_then(serde_json::Value::as_str),
            Some("oops at a"),
        );
        assert!(
            out.get("translatedError")
                .and_then(serde_json::Value::as_str)
                .is_some()
        );
        assert!(
            out.get("changed")
                .and_then(serde_json::Value::as_bool)
                .is_some()
        );
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
    /// VS Code opt-in actually reaches `xc_diagnostics_enabled`.
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
    fn action_command_forwards_string_args_for_bigip_rename_partition() {
        // The BIG-IP code-action provider emits `tclLsp.renamePartition`
        // with `string_args = [uri, partition]`.  The server conversion must
        // forward those — dropping `string_args` left the rename action
        // argument-less and the client could not run it.
        let cmd = core_code_actions::ActionCommand {
            command: "tclLsp.renamePartition".to_string(),
            args: Vec::new(),
            string_args: vec!["file:///c.conf".to_string(), "MyPartition".to_string()],
        };
        let lsp = action_command_to_lsp("Rename partition".to_string(), cmd);
        assert_eq!(lsp.command, "tclLsp.renamePartition");
        assert_eq!(
            lsp.arguments,
            Some(vec![
                serde_json::json!("file:///c.conf"),
                serde_json::json!("MyPartition"),
            ]),
        );
    }

    #[test]
    fn action_command_forwards_string_args_for_editor_rename() {
        // `editor.action.rename` carries just `[uri]`.
        let cmd = core_code_actions::ActionCommand {
            command: "editor.action.rename".to_string(),
            args: Vec::new(),
            string_args: vec!["file:///c.conf".to_string()],
        };
        let lsp = action_command_to_lsp("Rename".to_string(), cmd);
        assert_eq!(
            lsp.arguments,
            Some(vec![serde_json::json!("file:///c.conf")]),
        );
    }

    #[test]
    fn action_command_preserves_integer_position_args() {
        // The post-extract rename (`tclLsp.renameSymbolAtPosition`) carries
        // integer position args `[line, start, end]` and no string args —
        // the existing behaviour must be unchanged by the string_args
        // forwarding (the ints stay JSON numbers, in order).
        let cmd = core_code_actions::ActionCommand {
            command: "tclLsp.renameSymbolAtPosition".to_string(),
            args: vec![0, 4, 11],
            string_args: Vec::new(),
        };
        let lsp = action_command_to_lsp("Rename symbol".to_string(), cmd);
        assert_eq!(
            lsp.arguments,
            Some(vec![
                serde_json::json!(0),
                serde_json::json!(4),
                serde_json::json!(11),
            ]),
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
        let mut opts = tower_lsp_server::ls_types::FormattingOptions {
            tab_size: 2,
            insert_spaces: true,
            ..Default::default()
        };
        let cfg = formatter_config_from(&serde_json::Value::Null, &opts, "tcl");
        assert_eq!(cfg.indent_size, 2);
        assert_eq!(cfg.indent_style, core_formatting::IndentStyle::Spaces);
        // A degenerate zero tabSize is ignored (editors always send >= 1), so
        // the configured / default indent size stands (4 here).
        opts.tab_size = 0;
        assert_eq!(
            formatter_config_from(&serde_json::Value::Null, &opts, "tcl").indent_size,
            4
        );
        // insertSpaces=false selects tab indentation.
        opts.tab_size = 4;
        opts.insert_spaces = false;
        let cfg = formatter_config_from(&serde_json::Value::Null, &opts, "tcl");
        assert_eq!(cfg.indent_style, core_formatting::IndentStyle::Tabs);
    }

    #[test]
    fn formatter_config_from_consumes_formatting_settings() {
        // `tclLsp.formatting.*` settings flow into the FormatterConfig, while
        // the request's LSP indentation options override indent size/style.
        let formatting = serde_json::json!({
            "maxLineLength": 100,
            "goalLineLength": 90,
            "indentSize": 8,
            "indentStyle": "tabs",
            "spaceBetweenBraces": false,
            "lineEnding": "crlf",
            "blankLinesBetweenProcs": 2,
        });
        let opts = tower_lsp_server::ls_types::FormattingOptions {
            tab_size: 2,
            insert_spaces: true,
            ..Default::default()
        };
        let cfg = formatter_config_from(&formatting, &opts, "tcl");
        assert_eq!(cfg.max_line_length, 100);
        assert_eq!(cfg.goal_line_length, 90);
        assert!(!cfg.space_between_braces);
        assert_eq!(cfg.line_ending, "\r\n");
        assert_eq!(cfg.blank_lines_between_procs, 2);
        // LSP options win for indentation (tabSize=2, insertSpaces=true).
        assert_eq!(cfg.indent_size, 2);
        assert_eq!(cfg.indent_style, core_formatting::IndentStyle::Spaces);
        // A null formatting object falls back to defaults + LSP options.
        let dflt = formatter_config_from(&serde_json::Value::Null, &opts, "tcl");
        assert_eq!(dflt.max_line_length, 120);
    }

    #[test]
    fn formatter_config_consumes_previously_dropped_settings() {
        // RUST_ISSUE_133: these four `tclLsp.formatting.*` settings were shipped
        // by every editor but never mapped, so toggling them did nothing. They
        // must now flow into the FormatterConfig.
        let opts = tower_lsp_server::ls_types::FormattingOptions::default();
        let cfg = formatter_config_from(
            &serde_json::json!({
                "minBodyCommandsForExpansion": 3,
                "replaceSemicolonsWithNewlines": false,
                "enforceBracedExpr": true,
                "alignCommentsToCode": false,
            }),
            &opts,
            "tcl",
        );
        assert_eq!(cfg.min_body_commands_for_expansion, 3);
        assert!(!cfg.replace_semicolons_with_newlines);
        assert!(cfg.enforce_braced_expr);
        assert!(!cfg.align_comments_to_code);
    }

    #[test]
    fn formatter_config_round_trips_docstring_settings() {
        // The docstring knobs are carried for config compatibility (not yet
        // consumed by the engine); a settings object still flows them into the config.
        let opts = tower_lsp_server::ls_types::FormattingOptions::default();
        let cfg = formatter_config_from(
            &serde_json::json!({
                "docstringStyle": "preceding",
                "docstringTagStyle": "plain",
                "docstringDecoration": true,
                "docstringDecorationChar": "=",
                "docstringDecorationWidth": 80,
            }),
            &opts,
            "tcl",
        );
        assert_eq!(
            cfg.docstring_style,
            core_formatting::DocstringStyle::Preceding
        );
        assert_eq!(
            cfg.docstring_tag_style,
            core_formatting::DocstringTagStyle::Plain
        );
        assert!(cfg.docstring_decoration);
        assert_eq!(cfg.docstring_decoration_char, '=');
        assert_eq!(cfg.docstring_decoration_width, 80);
        // Defaults are style none, tag doxygen, char '.'.
        let dflt = core_formatting::FormatterConfig::default();
        assert_eq!(dflt.docstring_style, core_formatting::DocstringStyle::None);
        assert_eq!(
            dflt.docstring_tag_style,
            core_formatting::DocstringTagStyle::Doxygen
        );
        assert_eq!(dflt.docstring_decoration_char, '.');
        assert_eq!(dflt.docstring_decoration_width, 70);
    }

    #[tokio::test]
    async fn resolved_formatting_applies_through_global_config() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///f.tcl").unwrap();
        backend
            .apply_global_config(&serde_json::json!({
                "formatting": { "indentSize": 8, "maxLineLength": 70 }
            }))
            .await;
        let formatting = backend.resolved_formatting(&uri).await;
        assert_eq!(formatting["indentSize"], serde_json::json!(8));
        // With insertSpaces and no explicit tabSize override (tab_size: 0 → the
        // config's indentSize is used since LSP clamps 0 away).
        let opts = tower_lsp_server::ls_types::FormattingOptions {
            tab_size: 0,
            insert_spaces: true,
            ..Default::default()
        };
        let cfg = formatter_config_from(&formatting, &opts, "tcl");
        assert_eq!(cfg.max_line_length, 70);
        // tab_size 0 → unwrap_or(cfg.indent_size=8).max(1) = 8.
        assert_eq!(cfg.indent_size, 8);
    }

    #[test]
    fn configured_analyser_threads_mode_and_disabled() {
        // `Off` mode suppresses W108 entirely.
        let mut a = Backend::configured_analyser(HashSet::new(), NonAsciiMode::Off, HashSet::new());
        let r = a.analyse("set x \u{201c}hi\u{201d}\n", "tcl8.6");
        assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W108));
        // A disabled code is filtered from the analyser's output.
        let mut disabled = HashSet::new();
        disabled.insert("W108".to_string());
        let mut b = Backend::configured_analyser(disabled, NonAsciiMode::Strict, HashSet::new());
        let r = b.analyse("set x \u{201c}hi\u{201d}\n", "tcl8.6");
        assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W108));
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

        let line_index = tcl_lexer::LineIndex::new_lsp(text);
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

    /// Live path: a persisted `LineIndex` driven through
    /// `apply_content_change_indexed` over a sequence of ranged + full-replace
    /// edits must stay byte-identical to a rebuild from the final text — the
    /// invariant `DocumentState` relies on for its persisted index.
    #[test]
    fn apply_content_change_indexed_keeps_line_index_consistent() {
        let line_starts = |idx: &tcl_lexer::LineIndex| {
            (0..idx.line_count())
                .map(|l| idx.line_start(u32::try_from(l).unwrap_or(u32::MAX)))
                .collect::<Vec<_>>()
        };
        let mut text = "abc\ndef\nghi\n".to_string();
        let mut index = tcl_lexer::LineIndex::new_lsp(&text);
        let edits = [
            (
                Some(Range {
                    start: pos(0, 1),
                    end: pos(0, 1),
                }),
                "X\nY",
            ), // insert with newline
            (
                Some(Range {
                    start: pos(2, 0),
                    end: pos(3, 2),
                }),
                "",
            ), // multi-line deletion
            (None, "p\nq\nr\ns"), // full replacement
            (
                Some(Range {
                    start: pos(1, 1),
                    end: pos(2, 0),
                }),
                "Z",
            ), // collapse a line
        ];
        for (range, new_text) in edits {
            text = apply_content_change_indexed(&text, range, new_text, &mut index);
            assert_eq!(
                line_starts(&index),
                line_starts(&tcl_lexer::LineIndex::new_lsp(&text)),
                "persisted index diverged from rebuild after edit {new_text:?} -> {text:?}"
            );
        }
    }

    /// `RUST_ISSUE_033`: an incremental edit on an old-Mac (bare-`\r`) buffer must
    /// resolve against the LSP EOL model so the splice lands at the right byte
    /// and the shadow buffer stays correct.
    #[test]
    fn apply_content_change_indexed_handles_bare_cr_document() {
        // Client models "a\rb\rc" as 3 lines (0="a", 1="b", 2="c").
        let mut text = "a\rb\rc".to_string();
        let mut index = tcl_lexer::LineIndex::new_lsp(&text);
        // Replace line 1 ("b") with "BB": range (1,0)..(1,1).
        text = apply_content_change_indexed(
            &text,
            Some(Range {
                start: pos(1, 0),
                end: pos(1, 1),
            }),
            "BB",
            &mut index,
        );
        assert_eq!(text, "a\rBB\rc", "edit spliced at the wrong offset");
        // The persisted index matches a fresh LSP rebuild.
        let rebuilt = tcl_lexer::LineIndex::new_lsp(&text);
        assert_eq!(index.line_count(), rebuilt.line_count());
        for l in 0..u32::try_from(index.line_count()).unwrap() {
            assert_eq!(index.line_start(l), rebuilt.line_start(l), "line {l}");
        }
    }

    /// The source-style pass must reach the
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
            tcl_lsp_core::source_style::DEFAULT_LINE_LENGTH,
        );
        let codes: Vec<String> = diags
            .iter()
            .filter_map(|d| match &d.code {
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) => Some(c.clone()),
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
        let diags = lift_source_style_diagnostics(
            &src,
            &suppressed,
            &std::collections::HashSet::new(),
            tcl_lsp_core::source_style::DEFAULT_LINE_LENGTH,
        );
        let codes: Vec<String> = diags
            .iter()
            .filter_map(|d| match &d.code {
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) => Some(c.clone()),
                _ => None,
            })
            .collect();
        assert!(
            !codes.iter().any(|c| c == "W111"),
            "W111 should be suppressed"
        );
        assert!(codes.iter().any(|c| c == "W112"), "W112 should remain");
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
    /// filtered from the lifted set.
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
    /// basename.
    #[test]
    fn is_apl_source_detects_apl_documents() {
        let apl_ext = Uri::from_str("file:///app/iapp/foo.apl").unwrap();
        let presentation = Uri::from_str("file:///app/iapp/presentation").unwrap();
        let plain = Uri::from_str("file:///app/util.tcl").unwrap();
        assert!(is_apl_source(&apl_ext, "tcl"));
        assert!(is_apl_source(&presentation, "tcl"));
        // An explicit APL language id wins regardless of basename.
        assert!(is_apl_source(&plain, "tcl-apl"));
        // A plain Tcl document is not an APL source.
        assert!(!is_apl_source(&plain, "tcl"));
    }

    /// A `tcl_bigip` validator range carries an *inclusive* end column; the
    /// LSP lift makes it exclusive (`end.character + 1`).
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
            Some(tower_lsp_server::ls_types::DiagnosticSeverity::WARNING)
        );
    }

    /// End-to-end: the pull path's [`Backend::full_diagnostics_for`] routes a
    /// `f5-bigip` document to the BIG-IP validator instead of the Tcl
    /// analyser, so the editor receives `BIGIP6xxx` codes.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn full_diagnostics_for_routes_bigip_to_validator() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///Common/bigip.conf").unwrap();
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
        let uri = Uri::from_str("file:///rule.irul").unwrap();
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

    /// Server wiring: a command unresolved in file A but
    /// defined as a `proc` in file B (tracked in the salsa `Project`) must have
    /// its W123 suppressed once `crossFileResolution` is enabled — and remain
    /// present when it is off.  Exercises the live `db_set_source` →
    /// `Project`-sync → `project_proc_tails` path end to end.  Deliberately a
    /// plain `tcl8.6` document, not `f5-irules` — this is the general-purpose
    /// pass, independent of the f5-irules-only `xcDiagnostics` toggle.
    #[tokio::test]
    async fn cross_file_w123_suppressed_when_workspace_defines_proc() {
        let backend = test_backend();
        let a = Uri::from_str("file:///a.tcl").unwrap();
        let b = Uri::from_str("file:///b.tcl").unwrap();
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

        // crossFileResolution OFF: `helper` is unresolved in A → W123 present.
        let off = backend
            .full_diagnostics_for(&a, a_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            diag_codes(&off).iter().any(|c| c == "W123"),
            "W123 must be present when crossFileResolution is off, got: {:?}",
            diag_codes(&off),
        );

        // crossFileResolution ON: `helper` resolves cross-file (B defines it) → no W123.
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "crossFileResolution": true })
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

    /// Fix #1: the pull path (`full_diagnostics_for`) must feed the IRULE4002
    /// compiler check the *URI-scoped* generic-variable patterns, so a folder's
    /// `diagnostics.genericVariablePatterns` override applies on the pull path
    /// exactly as it does on the push path.  POSITIVE: a folder that disables the
    /// check (empty pattern list) suppresses IRULE4002.  NEGATIVE: a doc outside
    /// that folder keeps the default-pattern IRULE4002.
    #[tokio::test]
    async fn pull_path_honours_folder_generic_variable_patterns() {
        let backend = test_backend();
        let folder = Uri::from_str("file:///proj").unwrap();
        let inside = Uri::from_str("file:///proj/rule.tcl").unwrap();
        let outside = Uri::from_str("file:///other/rule.tcl").unwrap();
        *backend.workspace_folders.lock().await = vec![folder.clone()];
        // `static::debug` matches the built-in generic set ⇒ IRULE4002 by default.
        let src = "when RULE_INIT { set static::debug 1 }\n";

        // NEGATIVE (no folder override anywhere): IRULE4002 fires with defaults.
        let baseline = backend
            .full_diagnostics_for(
                &outside,
                src.to_owned(),
                "f5-irules".to_owned(),
                "tcl-irule",
            )
            .await;
        assert!(
            diag_codes(&baseline).iter().any(|c| c == "IRULE4002"),
            "IRULE4002 should fire with the default generic patterns, got: {:?}",
            diag_codes(&baseline),
        );

        // The folder explicitly empties the generic-pattern set (disables the
        // check).  On the push path this is `resolved_generic_variable_patterns`
        // returning `Some(vec![])`; the pull path must consult the same resolver.
        let fc = FolderConfig {
            generic_variable_patterns: FolderGenericPatterns::Replace(Vec::new()),
            ..FolderConfig::default()
        };
        backend
            .apply_folder_configs(vec![(folder.clone(), fc)])
            .await;

        // POSITIVE: a doc *inside* the folder no longer reports IRULE4002.
        let suppressed = backend
            .full_diagnostics_for(&inside, src.to_owned(), "f5-irules".to_owned(), "tcl-irule")
            .await;
        assert!(
            !diag_codes(&suppressed).iter().any(|c| c == "IRULE4002"),
            "folder's empty genericVariablePatterns must disable IRULE4002 on the \
             pull path, got: {:?}",
            diag_codes(&suppressed),
        );

        // NEGATIVE: a doc *outside* the folder still gets the default IRULE4002,
        // proving the override is folder-scoped, not process-global.
        let still_fires = backend
            .full_diagnostics_for(
                &outside,
                src.to_owned(),
                "f5-irules".to_owned(),
                "tcl-irule",
            )
            .await;
        assert!(
            diag_codes(&still_fires).iter().any(|c| c == "IRULE4002"),
            "a doc outside the folder must keep the default IRULE4002, got: {:?}",
            diag_codes(&still_fires),
        );
    }

    /// Fix #2: the pull path must apply the #723 W120 package refinement that the
    /// push path applies, so a workspace whose package database proves a required
    /// package transitively provides the flagged package suppresses the false
    /// W120.  POSITIVE: with a resolver proving `http` is transitively available,
    /// the W120 is refined away.  NEGATIVE: with the default empty resolver, the
    /// (genuine) W120 still publishes.
    #[tokio::test]
    async fn pull_path_applies_w120_package_refinement() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///w120.tcl").unwrap();
        // `package require mywrap` (so `package_requires` is non-empty) plus a
        // call to `http::register`, which the registry marks as requiring the
        // `http` package ⇒ single-file W120 for `http`.
        let src = "package require mywrap\nhttp::register foo 80 bar\n";

        // NEGATIVE: empty resolver can't prove `mywrap` provides `http`.  The
        // require is *unknowable* (unknown to registry + empty database), so the
        // conservative refinement drops W120 — to assert the *unrefined* W120 we
        // need a resolver that knows `mywrap` but where `mywrap` does NOT pull in
        // `http`.  Build that first.
        let plain_ws = TmpWs::new("w120-plain");
        plain_ws.write(
            "mywrap/mywrap.tcl",
            "package provide mywrap 1.0\nproc mywrap::go {} {}\n",
        );
        plain_ws.write(
            "mywrap/pkgIndex.tcl",
            "package ifneeded mywrap 1.0 [list source [file join $dir mywrap.tcl]]\n",
        );
        {
            let mut resolver = PackageResolver::new();
            resolver.scan_tree(&plain_ws.0, 100);
            *backend.package_resolver.write().await = resolver;
        }
        let unrefined = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            diag_codes(&unrefined).iter().any(|c| c == "W120"),
            "a resolvable wrapper that does NOT provide http must keep the W120, \
             got: {:?}",
            diag_codes(&unrefined),
        );

        // POSITIVE: a wrapper whose implementation does `package require http`
        // makes `http` transitively available ⇒ the W120 is refined away.
        let wrap_ws = TmpWs::new("w120-wrap");
        wrap_ws.write(
            "mywrap/mywrap.tcl",
            "package provide mywrap 1.0\npackage require http\nproc mywrap::go {} {}\n",
        );
        wrap_ws.write(
            "mywrap/pkgIndex.tcl",
            "package ifneeded mywrap 1.0 [list source [file join $dir mywrap.tcl]]\n",
        );
        // Also provide an `http` package so it is resolvable in the database.
        wrap_ws.write(
            "http/http.tcl",
            "package provide http 2.9\nproc http::register {a b c} {}\n",
        );
        wrap_ws.write(
            "http/pkgIndex.tcl",
            "package ifneeded http 2.9 [list source [file join $dir http.tcl]]\n",
        );
        {
            let mut resolver = PackageResolver::new();
            resolver.scan_tree(&wrap_ws.0, 100);
            *backend.package_resolver.write().await = resolver;
        }
        let refined = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            !diag_codes(&refined).iter().any(|c| c == "W120"),
            "the pull path must refine away the W120 once the workspace proves \
             http is transitively available, got: {:?}",
            diag_codes(&refined),
        );
    }

    /// Issue #832 (the reported bug): a command defined in a library on the
    /// `auto_path` — a `tclIndex` auto-loads it by bare name, the BLT/Rbc idiom —
    /// must NOT be flagged "Unknown command" (W123), *with `xcDiagnostics` and
    /// `crossFileResolution` both left off* (their default), because the
    /// package database resolves it exactly as go-to-definition does.
    ///
    /// * TN (must-stay-silent): the caller uses `Rbc_ActiveLegend`, which the
    ///   scanned `tclIndex` declares → no W123.
    /// * TP (must-fire control): a typo the index does not declare → W123 stands.
    /// * FN-guard: an empty database makes the real command genuinely unknowable
    ///   → W123 fires — proving suppression is data-driven, not a name allowlist.
    #[tokio::test]
    async fn autoload_library_command_suppresses_w123_issue_832() {
        let backend = test_backend();
        // A library dir on the auto_path: global procs registered by a `tclIndex`,
        // auto-loadable by bare name with no `package require`.
        let lib = TmpWs::new("rbc-lib");
        lib.write(
            "rbc/graph.tcl",
            "proc Rbc_ActiveLegend {graph} {}\nproc Rbc_ZoomStack {graph args} {}\n",
        );
        lib.write(
            "rbc/tclIndex",
            "# Tcl autoload index file, version 2.0\n\
             set auto_index(Rbc_ActiveLegend) [list source [file join $dir graph.tcl]]\n\
             set auto_index(Rbc_ZoomStack) [list source [file join $dir graph.tcl]]\n",
        );
        {
            let mut resolver = PackageResolver::new();
            resolver.scan_tree(&lib.0, 100);
            *backend.package_resolver.write().await = resolver;
        }
        let uri = Uri::from_str("file:///app.tcl").unwrap();

        // TN: the real library commands — no `package require` in the caller —
        // must not be flagged, with xcDiagnostics/crossFileResolution at their
        // default (off).
        let ok_src = "Rbc_ActiveLegend .g\nRbc_ZoomStack .g\n";
        let diags = backend
            .full_diagnostics_for(&uri, ok_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            !diag_codes(&diags).iter().any(|c| c == "W123"),
            "a command the auto_path tclIndex provides must not be W123 (#832), got: {:?}",
            diag_codes(&diags),
        );

        // TP control: a typo the index does not declare stays flagged.
        let typo_src = "Rbc_ActveLegend .g\n";
        let typo = backend
            .full_diagnostics_for(&uri, typo_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            diag_codes(&typo).iter().any(|c| c == "W123"),
            "a command no index provides must still be W123, got: {:?}",
            diag_codes(&typo),
        );

        // FN-guard: with an EMPTY database the real command is unknowable ⇒ W123
        // fires — suppression is driven by the database, not a name allowlist.
        *backend.package_resolver.write().await = PackageResolver::new();
        let empty = backend
            .full_diagnostics_for(&uri, ok_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            diag_codes(&empty).iter().any(|c| c == "W123"),
            "with no package database the command is genuinely unknown ⇒ W123, got: {:?}",
            diag_codes(&empty),
        );
    }

    /// Issue #832 secondary path: a `pkgIndex`-only package (no `tclIndex`) whose
    /// implementation defines the command, made available to a sourced module by
    /// an entry file's `package require`, suppresses the module's W123. The
    /// package's source files are consulted through the analyser's
    /// registry-driven definer walk (`proc` / `oo::class` / … from the command
    /// registry's `SymbolDef`s), not a `proc`-name scan.
    #[tokio::test]
    async fn pkgindex_package_source_command_suppresses_w123() {
        let backend = test_backend();
        // A pkgIndex-only package whose source defines a global proc.
        let ws = TmpWs::new("pkgsrc-w123");
        ws.write(
            "mylib/mylib.tcl",
            "package provide mylib 1.0\nproc draw_widget {w} {}\n",
        );
        ws.write(
            "mylib/pkgIndex.tcl",
            "package ifneeded mylib 1.0 [list source [file join $dir mylib.tcl]]\n",
        );
        {
            let mut resolver = PackageResolver::new();
            resolver.scan_tree(&ws.0, 100);
            *backend.package_resolver.write().await = resolver;
        }
        let util = Uri::from_file_path("/proj/lib/util.tcl").unwrap();
        // The module calls `draw_widget` with no local `package require`.
        let util_src = "draw_widget .w\n";

        // Control (FN would be a bug here): without an entry file requiring mylib,
        // the module inherits nothing, so `draw_widget` is genuinely unknown ⇒ W123.
        let unrefined = backend
            .full_diagnostics_for(&util, util_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            diag_codes(&unrefined).iter().any(|c| c == "W123"),
            "without an inherited require the command is unknown ⇒ W123, got: {:?}",
            diag_codes(&unrefined),
        );

        // Index an entry file that requires mylib and sources lib/util.tcl, so the
        // module inherits the `mylib` require via the workspace `source` graph.
        {
            let mut a = tcl_compiler::analyser::Analyser::new();
            let analysis = a
                .analyse("package require mylib\nsource lib/util.tcl\n", "tcl8.6")
                .clone();
            let app = Uri::from_file_path("/proj/app.tcl").unwrap();
            backend
                .workspace_index
                .write()
                .await
                .add_document(app.as_str(), &analysis);
        }

        // TN: the inherited `mylib` require makes `draw_widget` resolvable via the
        // package's implementation source ⇒ the W123 is refined away.
        let refined = backend
            .full_diagnostics_for(&util, util_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            !diag_codes(&refined).iter().any(|c| c == "W123"),
            "an available package's source-defined command must not be W123, got: {:?}",
            diag_codes(&refined),
        );
    }

    /// #804: a module `source`d by an entry file that ran `package require`
    /// inherits that require, so the sourced module's W120 for the same package
    /// is suppressed by the automatic workspace `source` graph — no config.
    #[tokio::test]
    async fn source_graph_inheritance_suppresses_w120_in_sourced_module() {
        let backend = test_backend();
        // Resolver that knows `http` (so the require is not "unknowable").
        let http_ws = TmpWs::new("srcgraph-http");
        http_ws.write(
            "http/http.tcl",
            "package provide http 2.9\nproc http::register {a b c} {}\n",
        );
        http_ws.write(
            "http/pkgIndex.tcl",
            "package ifneeded http 2.9 [list source [file join $dir http.tcl]]\n",
        );
        {
            let mut resolver = PackageResolver::new();
            resolver.scan_tree(&http_ws.0, 100);
            *backend.package_resolver.write().await = resolver;
        }
        let app = Uri::from_file_path("/proj/app.tcl").unwrap();
        let util = Uri::from_file_path("/proj/lib/util.tcl").unwrap();
        // util.tcl uses http::register with no local `package require`.
        let util_src = "http::register foo 80 bar\n";

        // Control: with the entry file NOT indexed, util inherits nothing, so
        // the single-file W120 stands.
        let unrefined = backend
            .full_diagnostics_for(&util, util_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            diag_codes(&unrefined).iter().any(|c| c == "W120"),
            "without the entry file indexed the W120 must stand, got: {:?}",
            diag_codes(&unrefined),
        );

        // Index the entry file: it requires http and sources lib/util.tcl.
        {
            let mut a = tcl_compiler::analyser::Analyser::new();
            let analysis = a
                .analyse("package require http\nsource lib/util.tcl\n", "tcl8.6")
                .clone();
            backend
                .workspace_index
                .write()
                .await
                .add_document(app.as_str(), &analysis);
        }
        let refined = backend
            .full_diagnostics_for(&util, util_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            !diag_codes(&refined).iter().any(|c| c == "W120"),
            "the sourced module must inherit the entry file's `package require http`, got: {:?}",
            diag_codes(&refined),
        );
    }

    /// #804: an explicitly configured `[project] entryPoints` makes that entry
    /// file's requires available project-wide even when it does NOT `source` the
    /// module — and it disables the automatic source-graph path.
    #[tokio::test]
    async fn explicit_entry_point_config_suppresses_w120_project_wide() {
        let backend = test_backend();
        let http_ws = TmpWs::new("entrypt-http");
        http_ws.write(
            "http/http.tcl",
            "package provide http 2.9\nproc http::register {a b c} {}\n",
        );
        http_ws.write(
            "http/pkgIndex.tcl",
            "package ifneeded http 2.9 [list source [file join $dir http.tcl]]\n",
        );
        {
            let mut resolver = PackageResolver::new();
            resolver.scan_tree(&http_ws.0, 100);
            *backend.package_resolver.write().await = resolver;
        }
        let folder = Uri::from_file_path("/proj").unwrap();
        let main = Uri::from_file_path("/proj/main.tcl").unwrap();
        let other = Uri::from_file_path("/proj/other.tcl").unwrap();
        // main.tcl requires http but does NOT source other.tcl.
        {
            let mut a = tcl_compiler::analyser::Analyser::new();
            let analysis = a.analyse("package require http\n", "tcl8.6").clone();
            backend
                .workspace_index
                .write()
                .await
                .add_document(main.as_str(), &analysis);
        }
        // Configure main.tcl as the project entry point.
        {
            let fc = FolderConfig {
                entry_points: Some(vec!["main.tcl".to_owned()]),
                ..FolderConfig::default()
            };
            *backend.folder_configs.lock().await = vec![(folder.clone(), fc)];
        }
        let other_src = "http::register foo 80 bar\n";
        let refined = backend
            .full_diagnostics_for(&other, other_src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            !diag_codes(&refined).iter().any(|c| c == "W120"),
            "an explicit entry point's requires must apply project-wide, got: {:?}",
            diag_codes(&refined),
        );
    }

    /// Fix #3: `scan_workspace_folders` must (re)build the package resolver from
    /// the editor library paths / discovered installations EVEN with no workspace
    /// roots, so a single-file / no-folder session's W120 refinement sees the
    /// user's "Select Tcl Installation" library paths.  POSITIVE: with no roots
    /// but a library path that provides a package, the resolver is populated.
    /// NEGATIVE (control): with roots and no library paths, it scans the tree.
    #[tokio::test]
    async fn scan_workspace_folders_builds_resolver_with_no_roots() {
        // POSITIVE: no workspace folders, but a `tclLsp.libraryPaths` directory
        // containing a package.  Before the fix `scan_workspace_folders`
        // early-returned and left the resolver empty.
        let lib_ws = TmpWs::new("noroots-lib");
        lib_ws.write(
            "mypkg/pkgIndex.tcl",
            "package ifneeded mypkg 1.0 [list source [file join $dir mypkg.tcl]]\n",
        );
        lib_ws.write("mypkg/mypkg.tcl", "package provide mypkg 1.0\n");

        let backend = test_backend();
        // No workspace folders at all (single-file / no-folder session).
        assert!(backend.workspace_folders.lock().await.is_empty());
        *backend.editor_library_paths.lock().await = vec![lib_ws.0.to_string_lossy().into_owned()];

        backend.scan_workspace_folders().await;

        assert!(
            backend.package_resolver.read().await.provides("mypkg"),
            "with no roots, the resolver must still be built from the editor \
             library paths so single-file sessions resolve packages",
        );

        // NEGATIVE / control: a fresh backend with a root tree and no library
        // paths still scans the workspace tree (the indexing path is unaffected).
        let root_ws = TmpWs::new("withroot");
        root_ws.write(
            "pkgIndex.tcl",
            "package ifneeded rootpkg 1.0 [list source [file join $dir r.tcl]]\n",
        );
        root_ws.write("r.tcl", "package provide rootpkg 1.0\n");
        let backend2 = test_backend();
        let root_uri = Uri::from_file_path(&root_ws.0).unwrap();
        *backend2.workspace_folders.lock().await = vec![root_uri];
        backend2.scan_workspace_folders().await;
        assert!(
            backend2.package_resolver.read().await.provides("rootpkg"),
            "with a root and no library paths, the tree scan must still populate \
             the resolver",
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
        assert_eq!(lift_symbol_kind(CoreSymbolKind::Test), SymbolKind::FUNCTION);
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Constant),
            SymbolKind::CONSTANT
        );
        assert_eq!(
            lift_symbol_kind(CoreSymbolKind::Operator),
            SymbolKind::OPERATOR
        );
        assert_eq!(lift_symbol_kind(CoreSymbolKind::Module), SymbolKind::MODULE);
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
        let (service, _socket) = tower_lsp_server::LspService::new(Backend::new);
        let db = tcl_lsp_db::TclDatabase::default();
        let db_config = tcl_lsp_db::AnalyserConfig::new(
            &db,
            default_disabled_set().into_iter().collect(),
            NonAsciiMode::Default,
            Vec::new(),
            None,
            None,
        );
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
            disabled_diagnostics: Mutex::new(default_disabled_set()),
            severity_overrides: Mutex::new(HashMap::new()),
            workspace_index: Arc::new(RwLock::new(core_workspace_index::WorkspaceIndex::new())),
            package_resolver: Arc::new(RwLock::new(PackageResolver::new())),
            autoloaded_library_uris: Arc::new(Mutex::new(HashSet::new())),
            rehomed_source_seeds: Arc::new(Mutex::new(HashMap::new())),
            discovered_tcl: Arc::new(std::sync::OnceLock::new()),
            editor_library_paths: Mutex::new(Vec::new()),
            extra_commands: Mutex::new(Vec::new()),
            bigip_version: Mutex::new(None),
            generic_variable_patterns: Mutex::new(None),
            formatting_settings: Mutex::new(serde_json::Value::Null),
            feature_toggles: Mutex::new(FeatureToggles::default()),
            optimiser_enabled: Mutex::new(true),
            shimmer_enabled: Mutex::new(true),
            optimiser_profile: Mutex::new(default_optimiser_profile()),
            optimiser_code_overrides: Mutex::new(HashMap::new()),
            line_length: Mutex::new(80),
            style_line_length: Mutex::new(120),
            db: Arc::new(Mutex::new(db)),
            db_files: Arc::new(Mutex::new(HashMap::new())),
            db_project: Arc::new(Mutex::new(None)),
            db_config: Arc::new(Mutex::new(db_config)),
            pull_diag_cache: Arc::new(Mutex::new(HashMap::new())),
            closed_diag_gen: Arc::new(Mutex::new(HashMap::new())),
            client_supports_pull_diagnostics: std::sync::atomic::AtomicBool::new(false),
            last_semantic_tokens: Arc::new(Mutex::new(HashMap::new())),
            semantic_tokens_refresh_pending: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            warm_task: std::sync::Mutex::new(None),
            edit_order: EditOrder::default(),
        }
    }

    // ---- Document-lifecycle + diagnostic-core internals -------------------
    // These exercise the previously-untested lifecycle path: the
    // `did_open` / `did_change` / `did_close` handlers and the synchronous
    // diagnostic driver `publish_analyser_diagnostics` → `run_diagnostics_core`
    // (the biggest untested function), asserting on observable state
    // (`documents`, `pull_diag_cache`) since the test client's socket is
    // detached so published notifications are no-ops.

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn publish_analyser_diagnostics_caches_tcl_diagnostics() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///life.tcl").unwrap();
        // `y` is set but never read → W211.
        let src = "proc foo {} { set y 1 }\n";
        register(&backend, &uri, src).await;
        backend
            .db_set_source(&uri, src.to_owned(), "tcl8.6".to_owned())
            .await;
        backend
            .publish_analyser_diagnostics(
                uri.clone(),
                src.to_owned(),
                "tcl8.6".to_owned(),
                0,
                Some(1),
            )
            .await;
        let cache = backend.pull_diag_cache.lock().await;
        let entry = cache
            .get(&uri)
            .expect("run_diagnostics_core should populate the pull cache");
        assert!(
            entry.diagnostics.iter().any(|d| matches!(
                &d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W211"
            )),
            "expected W211 in cached diagnostics, got {:?}",
            entry.diagnostics,
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn publish_analyser_diagnostics_routes_bigip_dialect_to_pull_cache() {
        // A BIG-IP document takes the `f5_dialect_diagnostics` branch of
        // `run_diagnostics_core` (model validators, not the Tcl analyser) and
        // still lands a pull-cache entry.
        let backend = test_backend();
        let uri = Uri::from_str("file:///life.conf").unwrap();
        let src = "ltm pool /Common/p {\n    members {\n        /Common/n:80 { }\n    }\n}\n";
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "f5-bigip".to_owned()),
        );
        backend
            .publish_analyser_diagnostics(
                uri.clone(),
                src.to_owned(),
                "f5-bigip".to_owned(),
                0,
                Some(1),
            )
            .await;
        assert!(
            backend.pull_diag_cache.lock().await.contains_key(&uri),
            "BIG-IP dialect should populate the pull cache via the f5 branch",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn did_open_stores_document() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///open.tcl").unwrap();
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: tower_lsp_server::ls_types::TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "tcl".to_owned(),
                    version: 7,
                    text: "set x 1\n".to_owned(),
                },
            })
            .await;
        let docs = backend.documents.lock().await;
        let doc = docs.get(&uri).expect("did_open should store the document");
        assert_eq!(doc.text, "set x 1\n");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn did_change_replaces_document_text() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///chg.tcl").unwrap();
        backend
            .did_open(DidOpenTextDocumentParams {
                text_document: tower_lsp_server::ls_types::TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "tcl".to_owned(),
                    version: 1,
                    text: "set x 1\n".to_owned(),
                },
            })
            .await;
        // A full-document replacement (`range: None`).
        backend
            .did_change(DidChangeTextDocumentParams {
                text_document: tower_lsp_server::ls_types::VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 2,
                },
                content_changes: vec![tower_lsp_server::ls_types::TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "set y 2\n".to_owned(),
                }],
            })
            .await;
        assert_eq!(
            backend.documents.lock().await.get(&uri).expect("doc").text,
            "set y 2\n",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn did_close_removes_document_and_clears_pull_cache() {
        // `file:///close.tcl` has no on-disk source, so #865's closed-file
        // republish finds nothing to analyse and falls back to clearing the
        // badge — the untitled / deleted-file path.
        let backend = test_backend();
        let uri = Uri::from_str("file:///close.tcl").unwrap();
        register(&backend, &uri, "set x 1\n").await;
        backend.pull_diag_cache.lock().await.insert(
            uri.clone(),
            PullDiagEntry {
                result_id: next_pull_diag_result_id(),
                revision: 0,
                diagnostics: Vec::new(),
            },
        );
        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;
        assert!(
            !backend.documents.lock().await.contains_key(&uri),
            "did_close should remove the open document",
        );
        assert!(
            !backend.pull_diag_cache.lock().await.contains_key(&uri),
            "a closed URI with no on-disk source should drop its pull-cache entry",
        );
    }

    /// #104 regression: reopening a document after the diagnostics master
    /// switch (`tclLsp.features.diagnostics`) was turned off *while the file
    /// was closed* must analyse under the current (off) switch and clear the
    /// squiggles — not republish the file's pre-toggle diagnostics.
    ///
    /// Root cause: `did_close` retains the URI's [`DiagSlot`] (only the live
    /// document and index entry are dropped), so `slot.latest_inputs` keeps the
    /// `diagnostics_enabled = true` captured on the pre-close analysis. `did_open`
    /// used to take the reuse-cached-inputs fast path (`schedule_diagnostics`,
    /// `force_refresh = false`), so the reopen's worker drained under those stale
    /// on-switch inputs and republished the diagnostics even though the master
    /// switch was now off (the `test-ext` `#104` flake, which only lined up under
    /// full-suite load). Opening a document is a config-context boundary, so it
    /// now force-refreshes the inputs; the slot's post-reopen
    /// `diagnostics_enabled` reflecting the *current* toggle proves it.
    ///
    /// Asserted on the slot's captured inputs (set synchronously by
    /// `schedule_diagnostics_impl` before the worker spawns) rather than a
    /// published set, since the test client's socket is detached.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reopen_after_master_switch_off_reresolves_diagnostics_inputs() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///reopen-master-off.tcl").unwrap();
        let open = |version| DidOpenTextDocumentParams {
            text_document: tower_lsp_server::ls_types::TextDocumentItem {
                uri: uri.clone(),
                language_id: "tcl".to_owned(),
                version,
                text: "set y 1\n".to_owned(),
            },
        };
        let captured_switch = || async {
            backend
                .diag_slots
                .lock()
                .await
                .get(&uri)
                .and_then(|s| s.latest_inputs.as_ref())
                .map(|i| i.toggles.diagnostics_enabled)
        };

        // 1. Open with the master switch on (the default): the slot captures
        //    `diagnostics_enabled = true`.
        backend.did_open(open(1)).await;
        assert_eq!(
            captured_switch().await,
            Some(true),
            "sanity: the initial open captures the master switch as on",
        );

        // 2. Close the tab — the DiagSlot (with its now-stale inputs) is
        //    retained, so the file keeps a badge (#865). The switch is still on
        //    at close time, so the retained inputs are `true`.
        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;
        assert_eq!(
            captured_switch().await,
            Some(true),
            "did_close retains the DiagSlot with its (now-stale) on-switch inputs",
        );

        // 3. Turn the diagnostics master switch off while the file is closed —
        //    the toggle store changes, but nothing re-resolves the closed URI's
        //    retained slot inputs.
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "diagnostics": false })
                .as_object()
                .unwrap(),
        );

        // 4. Reopen. The open path must re-resolve the config-sensitive inputs
        //    rather than reuse the stale on-switch ones, so the worker drains
        //    under the master switch's *current* (off) state.
        backend.did_open(open(2)).await;
        assert_eq!(
            captured_switch().await,
            Some(false),
            "reopening after the master switch went off must re-resolve the \
             diagnostics inputs to the current (off) toggle, not republish the \
             file's pre-toggle squiggles from the retained slot (#104)",
        );
    }

    // #865 — a workspace file that was opened and then had its editor tab closed
    // must keep its Problems / File-Explorer badge (the diagnostics computed from
    // its on-disk contents), instead of the old unconditional empty publish that
    // dropped the badge until the file was reopened.  Observed through the
    // pull-cache the push path keeps in lock-step (the test client's socket is
    // detached, so the notification itself is a no-op).

    /// TP: a closed on-disk file that has a problem keeps a non-empty badge.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn did_close_retains_diagnostics_for_on_disk_file_with_problems() {
        let root = unique_scratch_dir("close-retain");
        let on_disk = root.join("warn.tcl");
        // `y` is assigned but never read → W211, a stable Tcl-dialect diagnostic.
        std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
        let backend = test_backend();
        let uri = Uri::from_file_path(&on_disk).unwrap();
        register(&backend, &uri, "proc foo {} { set y 1 }\n").await;
        backend
            .db_set_source(
                &uri,
                "proc foo {} { set y 1 }\n".to_owned(),
                "tcl8.6".to_owned(),
            )
            .await;

        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;

        assert!(
            !backend.documents.lock().await.contains_key(&uri),
            "did_close still removes the open buffer",
        );
        let cache = backend.pull_diag_cache.lock().await;
        let entry = cache
            .get(&uri)
            .expect("a closed on-disk file with problems must keep a pull-cache entry");
        assert!(
            entry.diagnostics.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W211"
            )),
            "the retained badge must carry the file's W211: {:?}",
            entry.diagnostics,
        );
        drop(cache);
        std::fs::remove_dir_all(&root).ok();
    }

    /// TP + disk-accuracy: the retained badge reflects the *on-disk* file, not the
    /// (possibly discarded) buffer that was open — a buffer edited clean then
    /// closed-without-saving still shows the on-disk problem.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn did_close_badge_reflects_on_disk_not_discarded_buffer() {
        let root = unique_scratch_dir("close-disk-accurate");
        let on_disk = root.join("warn.tcl");
        std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
        let backend = test_backend();
        let uri = Uri::from_file_path(&on_disk).unwrap();
        // The open buffer is clean; only disk has the W211.  The retained badge
        // must come from disk.
        register(&backend, &uri, "proc foo {} { return 1 }\n").await;
        backend
            .db_set_source(
                &uri,
                "proc foo {} { return 1 }\n".to_owned(),
                "tcl8.6".to_owned(),
            )
            .await;

        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;

        let cache = backend.pull_diag_cache.lock().await;
        let entry = cache.get(&uri).expect("closed on-disk file keeps an entry");
        assert!(
            entry.diagnostics.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W211"
            )),
            "badge must reflect the on-disk W211, not the clean discarded buffer: {:?}",
            entry.diagnostics,
        );
        drop(cache);
        std::fs::remove_dir_all(&root).ok();
    }

    /// TN: a closed on-disk file with no problems gets an empty badge (no false
    /// File-Explorer decoration).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn did_close_publishes_empty_badge_for_clean_on_disk_file() {
        let root = unique_scratch_dir("close-clean");
        let on_disk = root.join("clean.tcl");
        // A defined-and-called proc with no unused variables — no diagnostics.
        let clean = "proc greet {} { puts hello }\ngreet\n";
        std::fs::write(&on_disk, clean).unwrap();
        let backend = test_backend();
        let uri = Uri::from_file_path(&on_disk).unwrap();
        register(&backend, &uri, clean).await;
        backend
            .db_set_source(&uri, clean.to_owned(), "tcl8.6".to_owned())
            .await;

        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;

        let cache = backend.pull_diag_cache.lock().await;
        let entry = cache
            .get(&uri)
            .expect("a clean closed on-disk file still primes the cache (empty)");
        assert!(
            entry.diagnostics.is_empty(),
            "a clean file must not gain a badge: {:?}",
            entry.diagnostics,
        );
        drop(cache);
        std::fs::remove_dir_all(&root).ok();
    }

    /// FP-guard: a file deleted before its tab closes loses its badge (no stale
    /// decoration for a file that is gone).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn did_close_clears_badge_when_file_deleted_before_close() {
        let root = unique_scratch_dir("close-deleted");
        let on_disk = root.join("gone.tcl");
        std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
        let backend = test_backend();
        let uri = Uri::from_file_path(&on_disk).unwrap();
        register(&backend, &uri, "proc foo {} { set y 1 }\n").await;
        backend
            .db_set_source(
                &uri,
                "proc foo {} { set y 1 }\n".to_owned(),
                "tcl8.6".to_owned(),
            )
            .await;
        backend.pull_diag_cache.lock().await.insert(
            uri.clone(),
            PullDiagEntry {
                result_id: next_pull_diag_result_id(),
                revision: 0,
                diagnostics: Vec::new(),
            },
        );
        // The file vanishes from disk before the tab closes.
        std::fs::remove_file(&on_disk).unwrap();

        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;

        assert!(
            !backend.pull_diag_cache.lock().await.contains_key(&uri),
            "a deleted file must not keep a stale badge",
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// FN-guard: the closed-file publish never shadows a document that is open
    /// again — a `did_open` racing in front of the closed run keeps authority, so
    /// a closed publish is a no-op for an open buffer and cannot blank or
    /// overwrite it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn publish_closed_is_noop_for_open_document() {
        let root = unique_scratch_dir("close-reopen");
        let on_disk = root.join("buf.tcl");
        std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
        let backend = test_backend();
        let uri = Uri::from_file_path(&on_disk).unwrap();
        register(&backend, &uri, "proc foo {} { set y 1 }\n").await;
        backend
            .db_set_source(
                &uri,
                "proc foo {} { set y 1 }\n".to_owned(),
                "tcl8.6".to_owned(),
            )
            .await;
        // A sentinel cache entry standing in for the open buffer's own published
        // set. The closed publish must leave it untouched (the doc is open).
        let sentinel = next_pull_diag_result_id();
        backend.pull_diag_cache.lock().await.insert(
            uri.clone(),
            PullDiagEntry {
                result_id: sentinel.clone(),
                revision: 7,
                diagnostics: Vec::new(),
            },
        );

        backend.publish_closed_file_diagnostics(&uri).await;

        let cache = backend.pull_diag_cache.lock().await;
        let entry = cache
            .get(&uri)
            .expect("open doc's cache entry must survive");
        assert_eq!(
            entry.result_id, sentinel,
            "a closed publish must not overwrite an open document's diagnostics",
        );
        drop(cache);
        std::fs::remove_dir_all(&root).ok();
    }

    /// Codex #3: a closed file's dialect is resolved from its on-disk source the
    /// way `did_open` would — an in-source directive, a BIG-IP basename, and a
    /// dialect-specific extension all survive the close instead of defaulting to
    /// generic Tcl.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dialect_for_closed_honours_directive_basename_and_extension() {
        let backend = test_backend();
        // In-source `# tcl-dialect:` directive pins the version.
        let versioned = Uri::from_str("file:///v.tcl").unwrap();
        assert_eq!(
            backend
                .dialect_for_closed(&versioned, "# tcl-dialect: tcl8.4\nset x 1\n")
                .await,
            "tcl8.4",
        );
        // A canonical BIG-IP basename routes to the config dialect.
        let bigip = Uri::from_str("file:///bigip.conf").unwrap();
        assert_eq!(
            backend
                .dialect_for_closed(&bigip, "ltm virtual v { }\n")
                .await,
            "f5-bigip",
        );
        // A dialect-specific extension routes without any editor language id.
        let irule = Uri::from_str("file:///r.irule").unwrap();
        assert_eq!(
            backend
                .dialect_for_closed(&irule, "when HTTP_REQUEST { }\n")
                .await,
            "f5-irules",
        );
        // A plain `.tcl` with no hint falls back to the session default.
        let plain = Uri::from_str("file:///p.tcl").unwrap();
        assert_eq!(
            backend.dialect_for_closed(&plain, "set x 1\n").await,
            "tcl8.6",
        );
    }

    /// Codex #3 end-to-end: `reindex_index_from_disk` (run on close) must store
    /// the source-directed dialect on the salsa `SourceFile`, since the cached
    /// base analysis reads its dialect from there — not the folder/default alone.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reindex_stores_source_directed_dialect() {
        let root = unique_scratch_dir("reindex-dialect");
        let on_disk = root.join("app.tcl");
        // Pinned to f5-irules by directive; the folder/default would give tcl8.6.
        std::fs::write(
            &on_disk,
            "# tcl-dialect: f5-irules\nwhen HTTP_REQUEST { }\n",
        )
        .unwrap();
        let backend = test_backend();
        let uri = Uri::from_file_path(&on_disk).unwrap();

        backend.reindex_index_from_disk(&uri).await;

        let file = backend
            .db_files
            .lock()
            .await
            .get(&uri)
            .copied()
            .expect("reindex must index the on-disk file");
        let db = backend.db.lock().await;
        assert_eq!(
            file.dialect(&*db).as_str(),
            "f5-irules",
            "reindex must store the directive-derived dialect the base analysis reads",
        );
        drop(db);
        std::fs::remove_dir_all(&root).ok();
    }

    /// Codex #2: a closed run whose generation has been superseded by a newer
    /// close / watched-change refresh must not publish — so an older run
    /// finishing late cannot overwrite the current set with stale diagnostics.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn superseded_closed_run_does_not_publish() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///gen.tcl").unwrap();
        // The URI is closed (absent from `documents`); the latest generation is 5.
        backend.closed_diag_gen.lock().await.insert(uri.clone(), 5);

        let diag = vec![tower_lsp_server::ls_types::Diagnostic::default()];
        let ctx = |generation: u64| DeliveryCtx {
            client: &backend.client,
            documents: &backend.documents,
            pull_diag_cache: &backend.pull_diag_cache,
            closed_diag_gen: &backend.closed_diag_gen,
            uri: &uri,
            currency: DiagCurrency::ClosedFromDisk(generation),
            version: None,
            client_supports_pull: false,
        };

        // A stale run (generation 3 < 5) is dropped.
        ctx(3).deliver_if_current(diag.clone()).await;
        assert!(
            backend.pull_diag_cache.lock().await.get(&uri).is_none(),
            "a superseded closed run must not publish",
        );
        // The current run (generation 5) publishes.
        ctx(5).deliver_if_current(diag).await;
        assert!(
            backend.pull_diag_cache.lock().await.get(&uri).is_some(),
            "the latest closed run publishes",
        );
    }

    /// Master switch off must clear a closed file's retained badge too — the
    /// open-document reschedule alone would freeze it at its pre-toggle set.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reschedule_closed_file_diagnostics_clears_badge_when_master_off() {
        let root = unique_scratch_dir("close-master-off");
        let on_disk = root.join("warn.tcl");
        std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
        let backend = test_backend();
        let uri = Uri::from_file_path(&on_disk).unwrap();
        // Prime the on-disk salsa source + a non-empty badge, as a prior close would.
        backend
            .db_set_source(
                &uri,
                "proc foo {} { set y 1 }\n".to_owned(),
                "tcl8.6".to_owned(),
            )
            .await;
        backend.pull_diag_cache.lock().await.insert(
            uri.clone(),
            PullDiagEntry {
                result_id: next_pull_diag_result_id(),
                revision: u64::MAX,
                diagnostics: vec![tower_lsp_server::ls_types::Diagnostic {
                    range: tower_lsp_server::ls_types::Range::default(),
                    code: Some(tower_lsp_server::ls_types::NumberOrString::String(
                        "W211".to_owned(),
                    )),
                    ..Default::default()
                }],
            },
        );

        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "diagnostics": false })
                .as_object()
                .unwrap(),
        );
        backend.reschedule_closed_file_diagnostics().await;

        let cache = backend.pull_diag_cache.lock().await;
        let entry = cache.get(&uri).expect("entry stays, now emptied");
        assert!(
            entry.diagnostics.is_empty(),
            "master-switch-off must clear the closed file's badge: {:?}",
            entry.diagnostics,
        );
        drop(cache);
        std::fs::remove_dir_all(&root).ok();
    }

    /// An external on-disk change to a closed, badged file refreshes its badge.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watched_change_refreshes_closed_file_badge() {
        let root = unique_scratch_dir("watched-refresh");
        let on_disk = root.join("warn.tcl");
        // Starts clean; a prior open/close primed an (empty) badge.
        std::fs::write(&on_disk, "set x 1\n").unwrap();
        let backend = test_backend();
        let uri = Uri::from_file_path(&on_disk).unwrap();
        backend
            .db_set_source(&uri, "set x 1\n".to_owned(), "tcl8.6".to_owned())
            .await;
        backend.pull_diag_cache.lock().await.insert(
            uri.clone(),
            PullDiagEntry {
                result_id: next_pull_diag_result_id(),
                revision: u64::MAX,
                diagnostics: Vec::new(),
            },
        );
        // An external edit introduces a W211.
        std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();

        backend
            .did_change_watched_files(DidChangeWatchedFilesParams {
                changes: vec![tower_lsp_server::ls_types::FileEvent {
                    uri: uri.clone(),
                    typ: FileChangeType::CHANGED,
                }],
            })
            .await;

        let cache = backend.pull_diag_cache.lock().await;
        let entry = cache.get(&uri).expect("badge stays cached");
        assert!(
            entry.diagnostics.iter().any(|d| matches!(
                &d.code,
                Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W211"
            )),
            "an external change must refresh a closed file's badge: {:?}",
            entry.diagnostics,
        );
        drop(cache);
        std::fs::remove_dir_all(&root).ok();
    }

    /// Deleting a closed, badged file on disk clears its badge.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watched_delete_clears_closed_file_badge() {
        let root = unique_scratch_dir("watched-delete");
        let on_disk = root.join("warn.tcl");
        std::fs::write(&on_disk, "proc foo {} { set y 1 }\n").unwrap();
        let backend = test_backend();
        let uri = Uri::from_file_path(&on_disk).unwrap();
        backend
            .db_set_source(
                &uri,
                "proc foo {} { set y 1 }\n".to_owned(),
                "tcl8.6".to_owned(),
            )
            .await;
        backend.pull_diag_cache.lock().await.insert(
            uri.clone(),
            PullDiagEntry {
                result_id: next_pull_diag_result_id(),
                revision: u64::MAX,
                diagnostics: vec![tower_lsp_server::ls_types::Diagnostic::default()],
            },
        );
        std::fs::remove_file(&on_disk).unwrap();

        backend
            .did_change_watched_files(DidChangeWatchedFilesParams {
                changes: vec![tower_lsp_server::ls_types::FileEvent {
                    uri: uri.clone(),
                    typ: FileChangeType::DELETED,
                }],
            })
            .await;

        assert!(
            !backend.pull_diag_cache.lock().await.contains_key(&uri),
            "deleting a closed file must clear its badge",
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn did_change_on_unopened_document_is_dropped() {
        // RUST_ISSUE_099: notification handlers run concurrently, so a
        // `didChange` can be processed before its `didOpen` or after its
        // `didClose`. It must NOT resurrect/create a phantom document.
        let backend = test_backend();
        let uri = Uri::from_str("file:///phantom.tcl").unwrap();
        backend
            .did_change(DidChangeTextDocumentParams {
                text_document: tower_lsp_server::ls_types::VersionedTextDocumentIdentifier {
                    uri: uri.clone(),
                    version: 5,
                },
                content_changes: vec![tower_lsp_server::ls_types::TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "set y 2\n".to_owned(),
                }],
            })
            .await;
        assert!(
            !backend.documents.lock().await.contains_key(&uri),
            "did_change on an unopened URI must not create a document",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn selection_range_returns_one_entry_per_position() {
        // RUST_ISSUE_100: the LSP spec requires `result[i]` to answer
        // `positions[i]`. Any position that yields no chain must still produce
        // a range (a degenerate fallback at the cursor) rather than being
        // dropped, which would misalign every later cursor in a multi-cursor
        // Expand Selection. Assert the handler always returns exactly one range
        // per requested position, including out-of-range cursors.
        let backend = test_backend();
        let uri = Uri::from_str("file:///sel.tcl").unwrap();
        register(&backend, &uri, "set x 1\n").await;
        let positions = vec![
            Position::new(0, 4),  // on `x`
            Position::new(99, 0), // far past the buffer
            Position::new(0, 0),  // line start
        ];
        let result = backend
            .selection_range(SelectionRangeParams {
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                positions: positions.clone(),
            })
            .await
            .expect("ok")
            .expect("some");
        assert_eq!(
            result.len(),
            positions.len(),
            "one selection range per requested position",
        );
    }

    #[test]
    fn materialise_selection_range_none_falls_back_to_cursor() {
        // The fallback the handler applies when a position yields no chain:
        // `materialise_selection_range` returns `None` for an empty chain, and
        // the handler substitutes a degenerate range at the cursor so the
        // position is never dropped (RUST_ISSUE_100).
        assert!(materialise_selection_range(&[]).is_none());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn did_change_configuration_applies_inline_settings() {
        let backend = test_backend();
        let params = DidChangeConfigurationParams {
            settings: serde_json::json!({
                "dialect": "tcl9.0",
                "tclLsp.style.nonAscii": "strict",
                "tclLsp.diagnostics.W211": false,
            }),
        };
        // `did_change_configuration` ends by pulling config from the client,
        // which errors fast against the test's detached socket; the timeout is
        // a backstop so a future transport change can't hang the suite.
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            backend.did_change_configuration(params),
        )
        .await
        .expect("did_change_configuration should not hang");
        assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
        assert_eq!(*backend.non_ascii_mode.lock().await, NonAsciiMode::Strict);
        assert!(
            backend.disabled_diagnostics.lock().await.contains("W211"),
            "W211 should be disabled by the inline settings",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn apply_global_config_applies_every_knob() {
        let backend = test_backend();
        let cfg = serde_json::json!({
            "features": { "hover": false },
            "xcDiagnostics": { "enabled": true },
            "optimiser": { "enabled": false, "profile": "full", "O100": false },
            "formatting": { "lineLength": 120 },
            "dialect": "tcl9.0",
            "style": { "nonAscii": "strict" },
            "diagnostics": { "W211": false },
        });
        backend.apply_global_config(&cfg).await;
        assert!(!backend.feature_toggles.lock().await.is_enabled("hover"));
        assert!(
            backend
                .feature_toggles
                .lock()
                .await
                .is_enabled("xcDiagnostics"),
            "xcDiagnostics section flag should map onto the toggle",
        );
        assert!(!*backend.optimiser_enabled.lock().await);
        assert_eq!(
            backend.optimiser_code_overrides.lock().await.get("O100"),
            Some(&false),
            "optimiser.O100=false should record a force-disable override",
        );
        assert_eq!(*backend.line_length.lock().await, 120);
        assert_eq!(*backend.default_dialect.lock().await, "tcl9.0");
        assert_eq!(*backend.non_ascii_mode.lock().await, NonAsciiMode::Strict);
        assert!(backend.disabled_diagnostics.lock().await.contains("W211"));
    }

    /// Fix #4 regression guard: the retired `features.inlayHints` alias must
    /// survive the *whole* config-apply → effective-config wiring, not just the
    /// `FeatureToggles::apply` unit. This mirrors the `lsp-e2e`
    /// `test_legacy_inlay_hints_alias_enables_type_only` flow without a live
    /// editor: apply `{"features": {"inlayHints": true}}` through
    /// `apply_global_config` (the same call `pull_and_apply_config` makes after a
    /// `didChangeConfiguration` re-pull), then read it back through
    /// `get_effective_config_command` (the `getEffectiveConfig` handler). The
    /// alias must resolve to `inlayTypeHints: true` while `inlayParameterHints`
    /// stays off.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn apply_global_config_inlay_alias_reaches_effective_config() {
        let backend = test_backend();
        // Default-off before any config: both inlay families report disabled.
        let before = backend
            .get_effective_config_command(&[serde_json::json!("file:///inlay.tcl")])
            .await
            .expect("effective config")
            .expect("config payload");
        assert_eq!(
            before["features"]["inlayTypeHints"],
            serde_json::Value::Bool(false),
            "type hints default off before any config: {before}",
        );

        backend
            .apply_global_config(&serde_json::json!({ "features": { "inlayHints": true } }))
            .await;

        let after = backend
            .get_effective_config_command(&[serde_json::json!("file:///inlay.tcl")])
            .await
            .expect("effective config")
            .expect("config payload");
        assert_eq!(
            after["features"]["inlayTypeHints"],
            serde_json::Value::Bool(true),
            "the `inlayHints` alias must enable `inlayTypeHints` end-to-end: {after}",
        );
        assert_eq!(
            after["features"]["inlayParameterHints"],
            serde_json::Value::Bool(false),
            "the alias enables type hints only; parameter hints stay off: {after}",
        );
        // And the gate the inlay handler reads agrees with the reported config.
        let uri = Uri::from_str("file:///inlay.tcl").unwrap();
        assert!(
            backend.inlay_family_enabled(&uri, "inlayTypeHints").await,
            "the inlay-type gate must see the alias-enabled toggle",
        );
        assert!(
            !backend
                .inlay_family_enabled(&uri, "inlayParameterHints")
                .await,
            "parameter hints must stay gated off",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn apply_global_config_ignores_non_object() {
        // A non-object config (e.g. a `null` from an editor with no settings)
        // is a no-op — the defaults survive.
        let backend = test_backend();
        backend.apply_global_config(&serde_json::Value::Null).await;
        assert_eq!(*backend.default_dialect.lock().await, "tcl8.6");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn resolved_analysis_settings_falls_back_to_global_defaults() {
        // With no folder config registered, every knob resolves from the
        // backend's global state, and the optimiser per-code overrides are
        // folded into the profile's disabled set.
        let backend = test_backend();
        backend
            .disabled_diagnostics
            .lock()
            .await
            .insert("W211".to_owned());
        *backend.non_ascii_mode.lock().await = NonAsciiMode::Strict;
        *backend.optimiser_enabled.lock().await = false;
        backend
            .optimiser_code_overrides
            .lock()
            .await
            .insert("O100".to_owned(), false);
        let uri = Uri::from_str("file:///settings.tcl").unwrap();
        let (disabled, non_ascii, opt_enabled, opt_disabled) =
            backend.resolved_analysis_settings(&uri).await;
        assert!(
            disabled.contains("W211"),
            "global disabled set should apply"
        );
        assert_eq!(non_ascii, NonAsciiMode::Strict);
        assert!(!opt_enabled, "global optimiser switch should apply");
        assert!(
            opt_disabled.contains("O100"),
            "a force-disable per-code override should land in opt_disabled",
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pull_diagnostics_include_compiler_and_optimiser_codes() {
        // Regression: the pull handler (`textDocument/diagnostic`) must return
        // the same full set as the push path — analyser + compiler/optimiser +
        // source-style — so editors on the pull path don't lose O-codes.
        let backend = test_backend();
        *backend.optimiser_profile.lock().await =
            tcl_compiler::optimiser::profiles::OptimisationProfile::Full;
        let uri = Uri::from_str("file:///pull.tcl").unwrap();
        let src = "if {1} { set x 1 } else { set y 2 }\n";
        let full = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        let analyser_only = {
            let mut a = Analyser::new();
            let analysis = a.analyse(src, "tcl8.6").clone();
            lift_analyser_diagnostics(src, &analysis.diagnostics)
        };
        let has_o100 = |ds: &[tower_lsp_server::ls_types::Diagnostic]| {
            ds.iter().any(|d| {
                matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "O100")
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

    fn doc_diag_params(uri: &Uri, previous: Option<&str>) -> DocumentDiagnosticParams {
        DocumentDiagnosticParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier { uri: uri.clone() },
            identifier: None,
            previous_result_id: previous.map(str::to_owned),
            work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(
            ),
            partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
        }
    }

    /// The pull handler returns a `Full` report carrying a `result_id` and,
    /// when the client echoes that same id back as `previousResultId` and the
    /// document has not changed, an `Unchanged` report naming the same id.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pull_diagnostic_returns_unchanged_when_result_id_matches() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///pull-unchanged.tcl").unwrap();
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
        let uri = Uri::from_str("file:///ws-diag.tcl").unwrap();
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
            previous_result_ids: vec![tower_lsp_server::ls_types::PreviousResultId {
                uri: uri.clone(),
                value: result_id.clone(),
            }],
            work_done_progress_params: tower_lsp_server::ls_types::WorkDoneProgressParams::default(
            ),
            partial_result_params: tower_lsp_server::ls_types::PartialResultParams::default(),
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
        let uri = Uri::from_str("file:///watched-gone.tcl").unwrap();
        // Index a file that is NOT open (no `documents` entry).
        {
            let mut a = Analyser::new();
            let analysis = a.analyse("proc gone {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .write()
                .await
                .add_document(uri.as_str(), &analysis);
        }
        assert!(
            backend
                .workspace_index
                .read()
                .await
                .document_uris()
                .iter()
                .any(|u| u == uri.as_str()),
            "precondition: file is indexed",
        );

        backend
            .did_change_watched_files(DidChangeWatchedFilesParams {
                changes: vec![tower_lsp_server::ls_types::FileEvent {
                    uri: uri.clone(),
                    typ: FileChangeType::DELETED,
                }],
            })
            .await;

        assert!(
            !backend
                .workspace_index
                .read()
                .await
                .document_uris()
                .iter()
                .any(|u| u == uri.as_str()),
            "a DELETED watched-file event must drop the index entry",
        );
    }

    /// A watched-file create/change/delete shifts the cross-file resolution domain
    /// without an open document's own edit, so open documents with cross-file
    /// diagnostics enabled must be rescheduled — otherwise a push-diagnostic client
    /// keeps stale W123/arity after a defining file disappears (git checkout /
    /// delete).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watched_file_delete_reschedules_open_xc_documents() {
        let backend = test_backend();
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "crossFileResolution": true })
                .as_object()
                .unwrap(),
        );
        // An open caller with crossFileResolution enabled.
        let caller = Uri::from_str("file:///caller.tcl").unwrap();
        backend.documents.lock().await.insert(
            caller.clone(),
            DocumentState::new("helper x\n".to_owned(), "tcl8.6".to_owned()),
        );

        // A watched, non-open file is deleted — its procs leave the `Project`.
        let deleted = Uri::from_str("file:///lib.tcl").unwrap();
        backend
            .did_change_watched_files(DidChangeWatchedFilesParams {
                changes: vec![tower_lsp_server::ls_types::FileEvent {
                    uri: deleted,
                    typ: FileChangeType::DELETED,
                }],
            })
            .await;

        // The open caller was rescheduled — a diagnostics slot now exists for it.
        assert!(
            backend.diag_slots.lock().await.contains_key(&caller),
            "an open crossFileResolution document must be rescheduled after a watched-file delete",
        );
    }

    /// Issue #829 regression: the always-on W120/W123 workspace refinement
    /// (`refine_workspace_w120`/`refine_workspace_w123`) is not gated by
    /// `crossFileResolution`, so a watched-file domain change must reschedule
    /// *every* open document — not just `crossFileResolution`-enabled ones.
    /// Before the fix, `did_change_watched_files` rescheduled only the
    /// narrower cross-file subset, leaving a plain document's stale W120
    /// (e.g. from a `source` ancestor that only just appeared on disk)
    /// unrefreshed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn watched_file_delete_reschedules_non_xc_document_too() {
        let backend = test_backend();
        // No `crossFileResolution` toggle — a plain document.
        let caller = Uri::from_str("file:///plain-caller.tcl").unwrap();
        backend.documents.lock().await.insert(
            caller.clone(),
            DocumentState::new("helper x\n".to_owned(), "tcl8.6".to_owned()),
        );

        let deleted = Uri::from_str("file:///lib.tcl").unwrap();
        backend
            .did_change_watched_files(DidChangeWatchedFilesParams {
                changes: vec![tower_lsp_server::ls_types::FileEvent {
                    uri: deleted,
                    typ: FileChangeType::DELETED,
                }],
            })
            .await;

        assert!(
            backend.diag_slots.lock().await.contains_key(&caller),
            "a plain (non-crossFileResolution) open document must also be rescheduled \
             after a watched-file delete, since the W120/W123 workspace \
             refinement is unconditional",
        );
    }

    /// Issue #829 regression, folder-add variant of the test above: adding a
    /// workspace folder scans it into `workspace_index` / `package_resolver`
    /// (the always-on W120/W123 refinement's inputs), so every open document
    /// must be rescheduled — not just `crossFileResolution`-enabled ones.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn workspace_folder_add_reschedules_non_xc_document_too() {
        let backend = test_backend();
        let caller = Uri::from_str("file:///plain-caller2.tcl").unwrap();
        backend.documents.lock().await.insert(
            caller.clone(),
            DocumentState::new("helper x\n".to_owned(), "tcl8.6".to_owned()),
        );
        let root = unique_scratch_dir("folder-add-reschedule");

        backend
            .did_change_workspace_folders(DidChangeWorkspaceFoldersParams {
                event: tower_lsp_server::ls_types::WorkspaceFoldersChangeEvent {
                    added: vec![tower_lsp_server::ls_types::WorkspaceFolder {
                        uri: Uri::from_file_path(&root).unwrap(),
                        name: "folder-add-reschedule".to_owned(),
                    }],
                    removed: Vec::new(),
                },
            })
            .await;

        assert!(
            backend.diag_slots.lock().await.contains_key(&caller),
            "a plain (non-crossFileResolution) open document must also be rescheduled \
             after a workspace-folder add",
        );
    }

    /// Issue #829 root-cause regression test: reproduces the exact race from
    /// the reported bug. `initialized()` kicks off `scan_workspace_folders`
    /// (which can take a while on a real workspace) but, before the fix,
    /// never rescheduled already-open documents once it completed — so a
    /// document opened at the same time as `initialized` fires (a client's
    /// normal startup sequence: `initialize` -> `initialized` with
    /// `didOpen` for restored tabs arriving concurrently, see
    /// `edit_serialize`'s doc comment) could have its first diagnostics
    /// published against the still-empty `workspace_index` /
    /// `package_resolver`, and nothing ever corrected it. Asserting the
    /// document is rescheduled after `initialized()` proves the fix; that the
    /// refinement itself is correct once rescheduled is proven separately by
    /// `source_graph_inheritance_suppresses_w120_in_sourced_module`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn initialized_reschedules_open_documents_after_workspace_scan() {
        let backend = test_backend();
        // A document referencing a package-gated command with no local
        // `package require` — the same shape as the reported
        // `::report::defstyle` false positive.
        let caller = Uri::from_str("file:///startup-caller.tcl").unwrap();
        backend.documents.lock().await.insert(
            caller.clone(),
            DocumentState::new("helper x\n".to_owned(), "tcl8.6".to_owned()),
        );
        assert!(
            backend.diag_slots.lock().await.is_empty(),
            "sanity: no diagnostics scheduled before initialized() runs",
        );

        backend.initialized(InitializedParams {}).await;

        assert!(
            backend.diag_slots.lock().await.contains_key(&caller),
            "an open document must be rescheduled once initialized()'s \
             scan_workspace_folders completes, so a diagnostics run that \
             raced ahead of the scan gets corrected instead of standing \
             indefinitely",
        );
    }

    /// Removing a workspace folder drops index entries for its (closed) files
    /// while leaving files under other folders intact.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn workspace_folder_removal_drops_index_under_folder() {
        let backend = test_backend();
        let gone = Uri::from_str("file:///proj-a/lib.tcl").unwrap();
        let kept = Uri::from_str("file:///proj-b/lib.tcl").unwrap();
        {
            let mut a = Analyser::new();
            let analysis = a.analyse("proc p {} {}\n", "tcl8.6").clone();
            let mut index = backend.workspace_index.write().await;
            index.add_document(gone.as_str(), &analysis);
            index.add_document(kept.as_str(), &analysis);
        }
        *backend.workspace_folders.lock().await = vec![
            Uri::from_str("file:///proj-a").unwrap(),
            Uri::from_str("file:///proj-b").unwrap(),
        ];

        backend
            .did_change_workspace_folders(DidChangeWorkspaceFoldersParams {
                event: tower_lsp_server::ls_types::WorkspaceFoldersChangeEvent {
                    added: Vec::new(),
                    removed: vec![tower_lsp_server::ls_types::WorkspaceFolder {
                        uri: Uri::from_str("file:///proj-a").unwrap(),
                        name: "proj-a".to_owned(),
                    }],
                },
            })
            .await;

        let uris = backend.workspace_index.read().await.document_uris();
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

    /// Removing a workspace folder shifts the salsa `Project` without an open
    /// document's own edit, so open documents with cross-file diagnostics enabled
    /// must be rescheduled (matching the watched-file path) — otherwise a
    /// push-diagnostic client keeps stale cross-file results.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn folder_removal_reschedules_open_xc_documents() {
        let backend = test_backend();
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "crossFileResolution": true })
                .as_object()
                .unwrap(),
        );
        let caller = Uri::from_str("file:///proj/caller.tcl").unwrap();
        backend.documents.lock().await.insert(
            caller.clone(),
            DocumentState::new("helper x\n".to_owned(), "tcl8.6".to_owned()),
        );
        *backend.workspace_folders.lock().await = vec![Uri::from_str("file:///proj").unwrap()];

        backend
            .did_change_workspace_folders(DidChangeWorkspaceFoldersParams {
                event: tower_lsp_server::ls_types::WorkspaceFoldersChangeEvent {
                    added: Vec::new(),
                    removed: vec![tower_lsp_server::ls_types::WorkspaceFolder {
                        uri: Uri::from_str("file:///proj").unwrap(),
                        name: "proj".to_owned(),
                    }],
                },
            })
            .await;

        assert!(
            backend.diag_slots.lock().await.contains_key(&caller),
            "an open crossFileResolution document must be rescheduled after a folder removal",
        );
    }

    /// A per-folder editor config overrides the process-global settings for
    /// documents under that folder, while documents under other folders keep
    /// the global values (longest-prefix resolution).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn per_folder_config_overrides_global_settings() {
        let backend = test_backend();
        let folder_a = Uri::from_str("file:///proj-a").unwrap();
        let folder_b = Uri::from_str("file:///proj-b").unwrap();
        let inside = Uri::from_str("file:///proj-a/sub/file.tcl").unwrap();
        let outside = Uri::from_str("file:///proj-b/file.tcl").unwrap();
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

    #[test]
    fn parse_folder_config_reads_extra_library_and_generic_patterns() {
        let cfg = serde_json::json!({
            "extraCommands": ["mylib::send", "mylib::recv"],
            "libraryPaths": ["/proj/lib"],
            "diagnostics": { "genericVariablePatterns": ["^proj_"] },
        });
        let fc = parse_folder_config(&cfg).expect("folder config");
        assert_eq!(
            fc.extra_commands.as_deref(),
            Some(["mylib::send".to_owned(), "mylib::recv".to_owned()].as_slice()),
        );
        assert_eq!(
            fc.library_paths.as_deref(),
            Some(["/proj/lib".to_owned()].as_slice())
        );
        assert_eq!(
            fc.generic_variable_patterns,
            FolderGenericPatterns::Replace(vec!["^proj_".to_owned()]),
        );
    }

    /// An explicit `genericVariablePatterns: null` selects the analyser's
    /// built-in defaults (`BuiltinDefaults`), distinct from an absent key
    /// (`Inherit`) and from a present array (`Replace`). Pins the third state.
    #[test]
    fn parse_folder_config_generic_patterns_null_is_builtin_defaults() {
        let with_null = serde_json::json!({
            "diagnostics": { "genericVariablePatterns": null },
        });
        let fc = parse_folder_config(&with_null).expect("folder config");
        assert_eq!(
            fc.generic_variable_patterns,
            FolderGenericPatterns::BuiltinDefaults,
            "null requests the built-in defaults",
        );

        // Negative: an absent key leaves the default `Inherit` (no override).
        let absent = serde_json::json!({ "extraCommands": ["x"] });
        let fc_absent = parse_folder_config(&absent).expect("folder config");
        assert_eq!(
            fc_absent.generic_variable_patterns,
            FolderGenericPatterns::Inherit,
            "an absent key inherits the global value",
        );
    }

    #[tokio::test]
    async fn per_folder_extra_commands_and_patterns_are_isolated() {
        let backend = test_backend();
        let folder_a = Uri::from_str("file:///proj-a").unwrap();
        let folder_b = Uri::from_str("file:///proj-b").unwrap();
        let inside = Uri::from_str("file:///proj-a/sub/file.tcl").unwrap();
        let outside = Uri::from_str("file:///proj-b/file.tcl").unwrap();
        *backend.workspace_folders.lock().await = vec![folder_a.clone(), folder_b.clone()];

        // Global sets one extra command; proj-a overrides with its own.
        *backend.extra_commands.lock().await = vec!["globalcmd".to_owned()];
        let fc = FolderConfig {
            extra_commands: Some(vec!["projacmd".to_owned()]),
            generic_variable_patterns: FolderGenericPatterns::Replace(vec!["^proja_".to_owned()]),
            ..FolderConfig::default()
        };
        backend
            .apply_folder_configs(vec![(folder_a.clone(), fc)])
            .await;

        // extraCommands resolve per folder.
        assert_eq!(
            backend.resolved_extra_commands(&inside).await,
            vec!["projacmd".to_owned()],
            "proj-a uses its own extraCommands",
        );
        assert_eq!(
            backend.resolved_extra_commands(&outside).await,
            vec!["globalcmd".to_owned()],
            "proj-b inherits the global extraCommands",
        );
        // genericVariablePatterns resolve per folder.
        assert_eq!(
            backend.resolved_generic_variable_patterns(&inside).await,
            Some(vec!["^proja_".to_owned()]),
        );
        assert_eq!(
            backend.resolved_generic_variable_patterns(&outside).await,
            None,
            "proj-b inherits the global (default) patterns",
        );
        // proj-a gets a salsa handle (it overrides analyser-config inputs).
        assert!(
            backend
                .folder_db_configs
                .lock()
                .await
                .iter()
                .any(|(u, _)| u == &folder_a),
            "proj-a must have a per-folder AnalyserConfig handle for extraCommands",
        );
    }

    /// The three `FolderGenericPatterns` states each resolve as documented, and
    /// a doc outside every folder falls back to the global value. Pins the
    /// 3-state replacement for the former `Option<Option<Vec<String>>>`.
    #[tokio::test]
    async fn resolved_generic_variable_patterns_three_state() {
        let backend = test_backend();
        let folder_replace = Uri::from_str("file:///replace").unwrap();
        let folder_builtin = Uri::from_str("file:///builtin").unwrap();
        let folder_inherit = Uri::from_str("file:///inherit").unwrap();
        *backend.workspace_folders.lock().await = vec![
            folder_replace.clone(),
            folder_builtin.clone(),
            folder_inherit.clone(),
        ];
        // Global supplies a list, so an inheriting folder (and a doc outside any
        // folder) must observe it — distinguishing `Inherit` from
        // `BuiltinDefaults` (which forces the built-in set, i.e. `None`).
        *backend.generic_variable_patterns.lock().await = Some(vec!["^global_".to_owned()]);

        backend
            .apply_folder_configs(vec![
                (
                    folder_replace.clone(),
                    FolderConfig {
                        generic_variable_patterns: FolderGenericPatterns::Replace(vec![
                            "^repl_".to_owned(),
                        ]),
                        ..FolderConfig::default()
                    },
                ),
                (
                    folder_builtin.clone(),
                    FolderConfig {
                        generic_variable_patterns: FolderGenericPatterns::BuiltinDefaults,
                        ..FolderConfig::default()
                    },
                ),
                (
                    folder_inherit.clone(),
                    FolderConfig {
                        // `Inherit` is the default; set another field so the
                        // folder still parses as a real (non-empty) override.
                        optimiser_enabled: Some(true),
                        ..FolderConfig::default()
                    },
                ),
            ])
            .await;

        // Positive: a folder that replaces the patterns gets exactly its list.
        let in_replace = Uri::from_str("file:///replace/a.tcl").unwrap();
        assert_eq!(
            backend
                .resolved_generic_variable_patterns(&in_replace)
                .await,
            Some(vec!["^repl_".to_owned()]),
            "Replace yields the folder's own list",
        );
        // Negative (vs Inherit): BuiltinDefaults forces the analyser built-ins
        // (`None`) even though the global is set.
        let in_builtin = Uri::from_str("file:///builtin/a.tcl").unwrap();
        assert_eq!(
            backend
                .resolved_generic_variable_patterns(&in_builtin)
                .await,
            None,
            "BuiltinDefaults forces the built-in set, ignoring the global list",
        );
        // Inherit folder observes the global list.
        let in_inherit = Uri::from_str("file:///inherit/a.tcl").unwrap();
        assert_eq!(
            backend
                .resolved_generic_variable_patterns(&in_inherit)
                .await,
            Some(vec!["^global_".to_owned()]),
            "Inherit falls back to the global patterns",
        );
        // A doc under no folder also inherits the global.
        let outside = Uri::from_str("file:///elsewhere/a.tcl").unwrap();
        assert_eq!(
            backend.resolved_generic_variable_patterns(&outside).await,
            Some(vec!["^global_".to_owned()]),
            "a doc outside every folder inherits the global patterns",
        );
    }

    /// A pull arriving after an edit (revision bumped) but before the debounced
    /// worker refreshes the cache must recompute, not serve the stale cached
    /// report — and must not answer `Unchanged` even when the client echoes the
    /// stale `result_id`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pull_diagnostic_recomputes_after_edit_bumps_revision() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///pull-stale.tcl").unwrap();
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
        let folder = Uri::from_str("file:///proj-a").unwrap();
        let inside = Uri::from_str("file:///proj-a/file.tcl").unwrap();
        let outside = Uri::from_str("file:///other/file.tcl").unwrap();
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

        let params = |uri: &Uri| WillSaveTextDocumentParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier { uri: uri.clone() },
            reason: tower_lsp_server::ls_types::TextDocumentSaveReason::MANUAL,
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
        let uri = Uri::from_str("file:///will-save.tcl").unwrap();
        register(&backend, &uri, "set    x     1\n").await;
        let params = WillSaveTextDocumentParams {
            text_document: tower_lsp_server::ls_types::TextDocumentIdentifier { uri: uri.clone() },
            reason: tower_lsp_server::ls_types::TextDocumentSaveReason::MANUAL,
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
        let uri = Uri::from_str("file:///stale.tcl").unwrap();
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
                .write()
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

        let index = backend.workspace_index.read().await;
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
        let root_url = Uri::from_file_path(&root).unwrap();
        *backend.workspace_folders.lock().await = vec![root_url];

        backend.scan_workspace_folders().await;

        let index = backend.workspace_index.read().await;
        let defs = index.proc_definitions("greet", "greet");
        assert!(
            !defs.is_empty(),
            "expected the unopened lib.tcl proc to be indexed",
        );
        // The call site in main.tcl should be indexed too (no
        // current-URI exclusion here).
        let invs = index.invocations_of("greet", "");
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
        let root_url = Uri::from_file_path(&root).unwrap();
        let uri = Uri::from_file_path(&on_disk).unwrap();
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
                .write()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        backend.scan_workspace_folders().await;

        let index = backend.workspace_index.read().await;
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
        let uri = Uri::from_str("file:///workspace/live.tcl").unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
        );

        let mut analyser = Analyser::new();
        let fresh_analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &fresh_analysis);

        let mut analyser = Analyser::new();
        let stale_analysis = analyser.analyse("proc stale {} {}\n", "tcl8.6").clone();
        let scan_results = vec![(
            uri.clone(),
            "proc stale {} {}\n".to_owned(),
            "tcl8.6".to_owned(),
            stale_analysis,
        )];

        backend.merge_workspace_scan_results(&scan_results).await;

        let index = backend.workspace_index.read().await;
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
        let uri = Uri::from_file_path(&on_disk).unwrap();
        // Seed the index with a now-closed buffer version (proc `fresh`),
        // standing in for what did_open would have indexed.
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .write()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        // What did_close does: refresh from disk instead of removing.
        backend.reindex_index_from_disk(&uri).await;

        let index = backend.workspace_index.read().await;
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
        let uri = Uri::from_file_path(&on_disk).unwrap();
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("proc fresh {} {}\n".to_owned(), "tcl8.6".to_owned()),
        );
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc fresh {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .write()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        backend.reindex_index_from_disk(&uri).await;

        let index = backend.workspace_index.read().await;
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
        let uri = Uri::from_str("file:///does/not/exist/gone.tcl").unwrap();
        {
            let mut analyser = Analyser::new();
            let analysis = analyser.analyse("proc ghost {} {}\n", "tcl8.6").clone();
            backend
                .workspace_index
                .write()
                .await
                .add_document(uri.as_str(), &analysis);
        }

        backend.reindex_index_from_disk(&uri).await;

        let index = backend.workspace_index.read().await;
        assert!(
            index.proc_definitions("ghost", "other").is_empty(),
            "an entry whose file no longer exists must be dropped",
        );
    }

    /// Whole-workspace scope: a proc defined in a file
    /// that is on disk but **not open** in the editor (driven here through
    /// `reindex_index_from_disk`, the `did_close` / scan / watched-file path) must
    /// still suppress a sibling's W123 and drive its arity error — cross-file diagnostics
    /// tracks the same on-disk population as the workspace index, not only open
    /// documents.
    #[tokio::test]
    async fn cross_file_resolves_against_disk_backed_file() {
        let root = unique_scratch_dir("xc-disk");
        let on_disk = root.join("lib.tcl");
        std::fs::write(&on_disk, "proc helper {x y} { return $x }\n").unwrap();

        let backend = test_backend();
        let b = Uri::from_file_path(&on_disk).unwrap();
        let a = Uri::from_str("file:///caller.tcl").unwrap();

        // B is on disk but never opened — the did_close / scan path indexes it
        // into both the workspace index and the salsa `Project`.
        backend.reindex_index_from_disk(&b).await;
        assert!(
            backend.db_project.lock().await.is_some(),
            "a disk-backed file must seed the salsa Project"
        );
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "crossFileResolution": true })
                .as_object()
                .unwrap(),
        );

        // Correct arity (2 args) → resolves cross-file, no W123, proving the
        // never-opened disk file is in the resolution domain.
        let ok = backend
            .full_diagnostics_for(
                &a,
                "helper foo bar\n".to_owned(),
                "tcl8.6".to_owned(),
                "tcl",
            )
            .await;
        assert!(
            !diag_codes(&ok).iter().any(|c| c == "W123"),
            "W123 must be suppressed against a disk-backed (unopened) file, got: {:?}",
            diag_codes(&ok),
        );

        // Wrong arity (3 args to the 2-param proc) → E003 (too many) from the
        // disk-backed proc's arity signature.
        let bad = backend
            .full_diagnostics_for(&a, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            diag_codes(&bad).iter().any(|c| c == "E003"),
            "E003 must fire against a disk-backed proc's arity, got: {:?}",
            diag_codes(&bad),
        );

        std::fs::remove_dir_all(&root).ok();
    }

    /// When a disk-backed workspace file disappears (deletion / no longer readable)
    /// and is reindexed, it must drop out of the salsa `Project` too, so its procs
    /// stop resolving cross-file (a sibling's W123 returns).  Guards the removal
    /// half of the whole-workspace scope.
    #[tokio::test]
    async fn cross_file_drops_disk_backed_file_when_gone() {
        let root = unique_scratch_dir("xc-disk-gone");
        let on_disk = root.join("lib.tcl");
        std::fs::write(&on_disk, "proc helper {x y} { return $x }\n").unwrap();

        let backend = test_backend();
        let b = Uri::from_file_path(&on_disk).unwrap();
        let a = Uri::from_str("file:///caller.tcl").unwrap();
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "crossFileResolution": true })
                .as_object()
                .unwrap(),
        );

        // Present on disk → resolves cross-file (no W123).
        backend.reindex_index_from_disk(&b).await;
        let present = backend
            .full_diagnostics_for(
                &a,
                "helper foo bar\n".to_owned(),
                "tcl8.6".to_owned(),
                "tcl",
            )
            .await;
        assert!(
            !diag_codes(&present).iter().any(|c| c == "W123"),
            "precondition: resolves while the file is present, got: {:?}",
            diag_codes(&present),
        );

        // Delete the file and reindex — it must leave the Project, so `helper` is
        // unresolved again (W123 returns).
        std::fs::remove_file(&on_disk).unwrap();
        backend.reindex_index_from_disk(&b).await;
        let gone = backend
            .full_diagnostics_for(
                &a,
                "helper foo bar\n".to_owned(),
                "tcl8.6".to_owned(),
                "tcl",
            )
            .await;
        assert!(
            diag_codes(&gone).iter().any(|c| c == "W123"),
            "W123 must return once the defining file is gone, got: {:?}",
            diag_codes(&gone),
        );

        std::fs::remove_dir_all(&root).ok();
    }

    /// Pull path, W123 disabled: cross-file arity must
    /// stay independent of the W123 toggle on the pull path too — disabling W123
    /// while keeping E002/E003 must still report the cross-file `E003` (matching
    /// the push path / local arity).
    #[tokio::test]
    async fn cross_file_arity_survives_w123_disabled_on_pull_path() {
        let backend = test_backend();
        *backend.disabled_diagnostics.lock().await = ["W123".to_owned()].into_iter().collect();
        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "crossFileResolution": true })
                .as_object()
                .unwrap(),
        );
        let a = Uri::from_str("file:///caller.tcl").unwrap();
        let b = Uri::from_str("file:///lib.tcl").unwrap();
        backend
            .db_set_source(
                &b,
                "proc helper {x y} { return $x }\n".to_owned(),
                "tcl8.6".to_owned(),
            )
            .await;
        // 3 args to a 2-param cross-file proc → E003, even with W123 disabled.
        let diags = backend
            .full_diagnostics_for(&a, "helper a b c\n".to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        let codes = diag_codes(&diags);
        assert!(
            codes.iter().any(|c| c == "E003"),
            "cross-file E003 must survive W123 disabled on the pull path, got: {codes:?}",
        );
        assert!(
            !codes.iter().any(|c| c == "W123"),
            "W123 stays disabled, got: {codes:?}",
        );
    }

    #[tokio::test]
    async fn resolve_folder_dialect_picks_deepest_prefix() {
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![
            (
                Uri::from_str("file:///workspace/").unwrap(),
                "tcl9.0".to_owned(),
            ),
            (
                Uri::from_str("file:///workspace/irules/").unwrap(),
                "f5-irules".to_owned(),
            ),
        ];
        let inside = Uri::from_str("file:///workspace/irules/rule.tcl").expect("parse target uri");
        assert_eq!(
            backend.resolve_folder_dialect(&inside).await,
            Some("f5-irules".to_owned()),
        );
        let outside_irules = Uri::from_str("file:///workspace/main.tcl").expect("parse target uri");
        assert_eq!(
            backend.resolve_folder_dialect(&outside_irules).await,
            Some("tcl9.0".to_owned()),
        );
        let unrelated = Uri::from_str("file:///elsewhere/x.tcl").unwrap();
        assert_eq!(backend.resolve_folder_dialect(&unrelated).await, None);
    }

    #[tokio::test]
    async fn resolve_folder_dialect_respects_directory_boundary() {
        // A prefix-only
        // match would incorrectly select `file:///workspace/app`'s
        // dialect for a document inside `file:///workspace/app2/`.
        let backend = test_backend();
        *backend.folder_dialects.lock().await = vec![
            (
                Uri::from_str("file:///workspace/app").unwrap(),
                "f5-irules".to_owned(),
            ),
            (
                Uri::from_str("file:///workspace/app2/").unwrap(),
                "tcl9.0".to_owned(),
            ),
        ];
        let sibling = Uri::from_str("file:///workspace/app2/main.tcl").unwrap();
        assert_eq!(
            backend.resolve_folder_dialect(&sibling).await,
            Some("tcl9.0".to_owned()),
            "sibling folder must not inherit prefix-matched dialect",
        );
        let inside_app = Uri::from_str("file:///workspace/app/inner.tcl").unwrap();
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
            (Uri::from_str("file:///ws/").unwrap(), "tcl8.6".to_owned()),
            (
                Uri::from_str("file:///ws/irules/").unwrap(),
                "f5-irules".to_owned(),
            ),
        ];
        // A file under the nested folder picks up the nested
        // (deepest-prefix) dialect, not the parent's.
        let nested = Uri::from_str("file:///ws/irules/app.tcl").unwrap();
        assert_eq!(
            folder_dialect_for(&nested, &folders).as_deref(),
            Some("f5-irules"),
        );
        // A file only under the root folder gets the root dialect.
        let top = Uri::from_str("file:///ws/util.tcl").unwrap();
        assert_eq!(
            folder_dialect_for(&top, &folders).as_deref(),
            Some("tcl8.6"),
        );
        // A file outside every folder has no override.
        let outside = Uri::from_str("file:///other/x.tcl").unwrap();
        assert_eq!(folder_dialect_for(&outside, &folders), None);
    }

    #[test]
    fn is_tcl_source_matches_the_full_tcl_family_extension_set() {
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
            Uri::from_str("file:///workspace/").unwrap(),
            "f5-irules".to_owned(),
        )];
        let doc = Uri::from_str("file:///workspace/main.tcl").unwrap();
        // The bare `tcl` language id names no specific version (editors send it
        // for every `.tcl` file), so the folder override *is* consulted — it is
        // the more specific signal.
        assert_eq!(
            backend.dialect_for_open(&doc, "tcl", "").await,
            "f5-irules".to_owned(),
        );
        // An unknown language id (`plaintext`) lets the folder
        // override take effect.
        assert_eq!(
            backend.dialect_for_open(&doc, "plaintext", "").await,
            "f5-irules".to_owned(),
        );
        // An explicit *versioned* language id is a deliberate choice and still
        // wins over the folder override.
        assert_eq!(
            backend.dialect_for_open(&doc, "tcl9.0", "").await,
            "tcl9.0".to_owned(),
        );
    }

    /// A `dialect =` key in `config.ini` / `.tcl-lsp.ini` sets the session
    /// `default_dialect`; a normally-opened `.tcl` buffer (language id `"tcl"`,
    /// which every editor sends) must resolve to it rather than pinning
    /// `tcl8.6`. Regression test for issue #805.
    #[tokio::test]
    async fn dialect_for_open_respects_config_default_dialect() {
        let backend = test_backend();
        *backend.default_dialect.lock().await = "tcl9.0".to_owned();
        let doc = Uri::from_str("file:///workspace/main.tcl").unwrap();
        assert_eq!(
            backend.dialect_for_open(&doc, "tcl", "").await,
            "tcl9.0".to_owned(),
        );
        // An in-source dialect directive is still the most specific signal and
        // overrides the configured default.
        assert_eq!(
            backend
                .dialect_for_open(&doc, "tcl", "# tcl-dialect: tcl8.4\n")
                .await,
            "tcl8.4".to_owned(),
        );
    }

    /// An explicit BIG-IP language id resolves to `f5-bigip` even when the
    /// document basename is not a canonical `bigip*.conf` name — the
    /// manual-language-mode case the `is_bigip_conf_name` basename branch
    /// alone would miss.
    #[tokio::test]
    async fn dialect_for_open_maps_bigip_language_id() {
        let backend = test_backend();
        let doc = Uri::from_str("file:///workspace/device_config.txt").unwrap();
        assert_eq!(
            backend.dialect_for_open(&doc, "tcl-bigip", "").await,
            "f5-bigip".to_owned(),
        );
        assert_eq!(
            backend.dialect_for_open(&doc, "f5-bigip", "").await,
            "f5-bigip".to_owned(),
        );
        // A generic Tcl language id on the same non-conf name does not get the
        // BIG-IP dialect (no basename match, no explicit BIG-IP id).
        assert_ne!(
            backend.dialect_for_open(&doc, "tcl", "").await,
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

        let plain = Uri::from_str("file:///plain.tcl").unwrap();
        let irule = Uri::from_str("file:///app.irule").unwrap();
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
        let lib_uri = Uri::from_str("file:///lib.tcl").unwrap();
        let main_uri = Uri::from_str("file:///main.tcl").unwrap();
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
                .write()
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
        let uri = Uri::from_str("file:///m.tcl").unwrap();
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
        let uri = Uri::from_str("file:///m.tcl").unwrap();
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
    async fn cross_document_definition_prefers_class_creation_over_define_stub() {
        // `::C` is created in a.tcl and extended by a cross-file `oo::define` in
        // b.tcl.  Go-to-definition on a `C` call jumps to the real creation site
        // (a.tcl), not the extension stub (b.tcl).
        let backend = test_backend();
        let a = Uri::from_str("file:///a.tcl").unwrap();
        let b = Uri::from_str("file:///b.tcl").unwrap();
        register(&backend, &a, "oo::class create C {}\n").await;
        register(&backend, &b, "oo::define C {\n    method foo {} {}\n}\n").await;
        let main_src = "C new\n";
        let main = Uri::from_str("file:///main.tcl").unwrap();
        let analysis = {
            let mut an = Analyser::new();
            an.analyse(main_src, "tcl8.6").clone()
        };
        // Cursor on the `C` call head.
        let defs = backend
            .cross_document_definition(&main, main_src, Position::new(0, 0), &analysis)
            .await
            .expect("ok");
        assert_eq!(defs.len(), 1, "{defs:?}");
        assert_eq!(defs[0].uri, a, "{defs:?}");
    }

    #[tokio::test]
    async fn cross_document_definition_skipped_when_local_match_exists() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///main.tcl").unwrap();
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
    /// A request that reads a document must not observe a buffer that is missing
    /// an already-received edit.
    ///
    /// `tower-lsp-server` drives requests and notifications through one
    /// `buffer_unordered` pool, so a request handler can be polled while an
    /// earlier `didChange` is still in flight. `read_document` must therefore
    /// wait for the outstanding edit rather than racing ahead and answering from
    /// the pre-edit text (which yielded semantic tokens whose lines/lengths
    /// described text the client had already replaced).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn read_document_waits_for_an_in_flight_edit() {
        let backend = Arc::new(test_backend());
        let uri = Uri::from_str("file:///ordering.tcl").unwrap();
        register(&backend, &uri, "set x 1\n").await;

        // Stand in for a `didChange` that has arrived and taken its turn, but has
        // not yet spliced the buffer.
        let ticket = backend.edit_order.take_ticket();
        let edit_in_flight = backend.edit_order.wait_turn(ticket).await;

        let mut reader = tokio::spawn({
            let backend = Arc::clone(&backend);
            let uri = uri.clone();
            async move { backend.read_document(&uri).await.map(|d| d.text) }
        });

        // While the edit is in flight the read must not resolve.
        let elapsed = std::time::Duration::from_millis(300);
        if let Ok(done) = tokio::time::timeout(elapsed, &mut reader).await {
            panic!("read_document returned {done:?} while an edit was still in flight");
        }

        // Land the edit, then release the ordering lock: the read must now
        // resolve against the *settled* buffer, never the pre-edit text.
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new("set x 2\n".to_owned(), "tcl8.6".to_owned()),
        );
        drop(edit_in_flight);
        let text = tokio::time::timeout(std::time::Duration::from_secs(5), reader)
            .await
            .expect("read_document must resolve once the edit lands")
            .expect("reader task panicked");
        assert_eq!(text.as_deref(), Some("set x 2\n"));
    }

    async fn register(backend: &Backend, uri: &Uri, src: &str) {
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "tcl8.6".to_owned()),
        );
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        backend
            .workspace_index
            .write()
            .await
            .add_document(uri.as_str(), &analysis);
    }

    #[tokio::test]
    async fn cross_document_references_finds_sibling_call_sites() {
        let backend = test_backend();
        let lib = Uri::from_str("file:///lib.tcl").unwrap();
        let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
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
            &[],
            &[libdir.display().to_string()],
            &[],
            WORKSPACE_SCAN_DIR_CAP,
        );
        Uri::from_file_path(libdir.join("graph.tcl")).expect("library uri")
    }

    /// M8's second half: the autoload tier merges the defining library file
    /// into the workspace index, so cross-document **references** reach the
    /// library declaration and the library's own internal call sites — not
    /// just go-to-definition.
    #[tokio::test]
    async fn autoload_merge_makes_references_reach_the_library_m8() {
        let backend = test_backend();
        let lib_uri = seed_autoload_library(&backend, "autoload-refs").await;
        // The workspace document only *calls* the library command.
        let app = Uri::from_str("file:///app.tcl").unwrap();
        let app_src = "Rbc_ActiveLegend .g\n";
        register(&backend, &app, app_src).await;
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(app_src, "tcl8.6").clone()
        };
        let refs = backend
            .cross_document_references(&app, app_src, &analysis, Position::new(0, 3), true)
            .await;
        assert!(
            refs.iter()
                .any(|l| l.uri == lib_uri && l.range.start.line == 0),
            "declaration in the library file: {refs:?}"
        );
        assert!(
            refs.iter()
                .any(|l| l.uri == lib_uri && l.range.start.line == 1),
            "library-internal call site: {refs:?}"
        );
    }

    /// M8's second half, rename leg: a rename triggered from the workspace
    /// call site rewrites the library declaration and the library-internal
    /// call site, so the whole family stays consistent.
    #[tokio::test]
    async fn autoload_merge_makes_rename_reach_the_library_m8() {
        let backend = test_backend();
        let lib_uri = seed_autoload_library(&backend, "autoload-rename").await;
        let app = Uri::from_str("file:///app.tcl").unwrap();
        let app_src = "Rbc_ActiveLegend .g\n";
        register(&backend, &app, app_src).await;
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(app_src, "tcl8.6").clone()
        };
        let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
            std::collections::HashMap::new();
        backend
            .add_cross_document_rename_edits(
                &app,
                app_src,
                &analysis,
                Position::new(0, 3),
                "Rbc_Shiny",
                &mut changes,
            )
            .await;
        let lib_edits = changes.get(&lib_uri).cloned().unwrap_or_default();
        assert_eq!(
            lib_edits.len(),
            2,
            "library declaration + internal call: {lib_edits:?}"
        );
    }

    /// FP guard: when the workspace itself defines the command, the autoload
    /// tier must not fire — the workspace definition wins and no library file
    /// is merged into the index.
    #[tokio::test]
    async fn autoload_merge_abstains_when_the_workspace_defines_the_command() {
        let backend = test_backend();
        let lib_uri = seed_autoload_library(&backend, "autoload-abstain").await;
        let def = Uri::from_str("file:///def.tcl").unwrap();
        register(&backend, &def, "proc Rbc_ActiveLegend {graph} {}\n").await;
        let app = Uri::from_str("file:///app.tcl").unwrap();
        let app_src = "Rbc_ActiveLegend .g\n";
        register(&backend, &app, app_src).await;
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(app_src, "tcl8.6").clone()
        };
        let refs = backend
            .cross_document_references(&app, app_src, &analysis, Position::new(0, 3), true)
            .await;
        assert!(
            refs.iter().all(|l| l.uri != lib_uri),
            "the same-named library must stay unmerged: {refs:?}"
        );
        assert!(
            backend.autoloaded_library_uris.lock().await.is_empty(),
            "no library merge may be recorded"
        );
    }

    /// M9: `source` runs a file in the caller's namespace, so a bare
    /// `proc helper` in a file sourced inside `namespace eval ::x` is really
    /// `::x::helper` — cross-document references from a correctly-qualified
    /// call site must reach the sourced file's declaration.
    #[tokio::test]
    async fn sourced_file_defs_rehome_under_the_source_site_namespace_m9() {
        let backend = test_backend();
        let a = Uri::from_str("file:///proj/a.tcl").unwrap();
        let b = Uri::from_str("file:///proj/b.tcl").unwrap();
        register(&backend, &b, "proc helper {} {}\nhelper\n").await;
        let a_src = "namespace eval ::x { source b.tcl }\n::x::helper\n";
        register(&backend, &a, a_src).await;
        let analysis = {
            let mut an = Analyser::new();
            an.analyse(a_src, "tcl8.6").clone()
        };
        let refs = backend
            .cross_document_references(&a, a_src, &analysis, Position::new(1, 3), true)
            .await;
        assert!(
            refs.iter().any(|l| l.uri == b && l.range.start.line == 0),
            "the sourced declaration must be a reference target: {refs:?}"
        );
        assert!(
            refs.iter().any(|l| l.uri == b && l.range.start.line == 1),
            "the sourced file's own bare call re-homes too: {refs:?}"
        );
    }

    /// M9, declaration side: references from the sourced file's own
    /// declaration cursor reach the sourcing document's qualified call.
    #[tokio::test]
    async fn sourced_file_declaration_finds_qualified_callers_m9() {
        let backend = test_backend();
        let a = Uri::from_str("file:///proj/a.tcl").unwrap();
        let b = Uri::from_str("file:///proj/b.tcl").unwrap();
        let b_src = "proc helper {} {}\n";
        register(&backend, &b, b_src).await;
        register(
            &backend,
            &a,
            "namespace eval ::x { source b.tcl }\n::x::helper\n",
        )
        .await;
        // Reconcile, then resolve from b.tcl's declaration.
        backend.refresh_source_rehoming().await;
        let analysis = {
            let mut an = Analyser::new();
            an.analyse(b_src, "tcl8.6").clone()
        };
        let refs = backend
            .cross_document_references(&b, b_src, &analysis, Position::new(0, 6), false)
            .await;
        assert!(
            refs.iter().any(|l| l.uri == a && l.range.start.line == 1),
            "the qualified call in the sourcing document is a reference: {refs:?}"
        );
    }

    /// Issue #945 fault 3: a file sourced into **several** namespaces is one
    /// physical declaration with one runtime identity per source site
    /// (tclsh 9.0.4: `namespace eval ::x {source b.tcl}` + `namespace eval
    /// ::y {source b.tcl}` yields both `::x::helper` and `::y::helper`).
    /// Declaration-side navigation must union every view — never map the
    /// cursor to the first sorted seed and lose the other callers.
    #[tokio::test]
    async fn multi_seeded_declaration_unions_every_runtime_identity_945() {
        let backend = test_backend();
        let a = Uri::from_str("file:///proj/a.tcl").unwrap();
        let b = Uri::from_str("file:///proj/b.tcl").unwrap();
        let b_src = "proc helper {} {namespace current}\n";
        register(&backend, &b, b_src).await;
        register(
            &backend,
            &a,
            "namespace eval ::x { source b.tcl }\nnamespace eval ::y { source b.tcl }\n::x::helper\n::y::helper\n",
        )
        .await;
        backend.refresh_source_rehoming().await;
        let analysis = {
            let mut an = Analyser::new();
            an.analyse(b_src, "tcl8.6").clone()
        };
        let refs = backend
            .cross_document_references(&b, b_src, &analysis, Position::new(0, 6), false)
            .await;
        let caller_lines: std::collections::BTreeSet<u32> = refs
            .iter()
            .filter(|l| l.uri == a)
            .map(|l| l.range.start.line)
            .collect();
        assert_eq!(
            caller_lines,
            [2, 3].into_iter().collect(),
            "both seeded views' callers must surface: {refs:?}"
        );
        // Rename from the declaration is an explicit multi-symbol edit:
        // the one physical token changes, and *every* view's callers
        // follow — leaving no runtime identity dispatching the old name.
        let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> =
            std::collections::HashMap::new();
        let blocked = backend
            .add_cross_document_rename_edits(
                &b,
                b_src,
                &analysis,
                Position::new(0, 6),
                "assist",
                &mut changes,
            )
            .await;
        assert!(!blocked, "nothing blocks this rename");
        let a_edit_lines: std::collections::BTreeSet<u32> = changes
            .get(&a)
            .map(|edits| edits.iter().map(|e| e.range.start.line).collect())
            .unwrap_or_default();
        assert_eq!(
            a_edit_lines,
            [2, 3].into_iter().collect(),
            "both views' callers must be rewritten: {changes:?}"
        );
    }

    /// M9 stage 9.2: a statically-foldable computed source path
    /// (`[file join [file dirname [info script]] b.tcl]`) resolves like a
    /// literal; an unfoldable one abstains.
    #[tokio::test]
    async fn computed_source_paths_fold_statically_m9() {
        let backend = test_backend();
        let a = Uri::from_str("file:///proj/a.tcl").unwrap();
        let b = Uri::from_str("file:///proj/b.tcl").unwrap();
        register(&backend, &b, "proc helper {} {}\n").await;
        let a_src = "namespace eval ::x {\n    source [file join [file dirname [info script]] b.tcl]\n}\n::x::helper\n";
        register(&backend, &a, a_src).await;
        let analysis = {
            let mut an = Analyser::new();
            an.analyse(a_src, "tcl8.6").clone()
        };
        let refs = backend
            .cross_document_references(&a, a_src, &analysis, Position::new(3, 3), true)
            .await;
        assert!(
            refs.iter().any(|l| l.uri == b && l.range.start.line == 0),
            "the folded path re-homes the sourced file: {refs:?}"
        );
        // Unfoldable: a `$var` path must abstain (no re-homing, no panic).
        let c = Uri::from_str("file:///proj/c.tcl").unwrap();
        register(&backend, &c, "namespace eval ::y { source $dynamic }\n").await;
        backend.refresh_source_rehoming().await;
        assert!(
            !backend
                .workspace_index
                .read()
                .await
                .workspace_command_exists("::y::helper"),
            "an unfoldable path must not re-home anything"
        );
    }

    /// A package-database rebuild drops previously-merged library files from
    /// the index (they may no longer be on the resolved `auto_path`); the next
    /// query re-merges on demand.
    #[tokio::test]
    async fn autoload_merge_is_dropped_when_the_package_database_rebuilds() {
        let backend = test_backend();
        seed_autoload_library(&backend, "autoload-rebuild").await;
        let resolved = backend
            .ensure_autoload_indexed("Rbc_ActiveLegend", "::")
            .await;
        assert_eq!(resolved.as_deref(), Some("::Rbc_ActiveLegend"));
        assert!(
            backend
                .workspace_index
                .read()
                .await
                .workspace_command_exists("::Rbc_ActiveLegend")
        );
        // Rebuild with no library paths: the merged entries must be dropped.
        backend.scan_workspace_folders().await;
        assert!(
            !backend
                .workspace_index
                .read()
                .await
                .workspace_command_exists("::Rbc_ActiveLegend"),
            "stale library definitions must not survive a package-database rebuild"
        );
        assert!(backend.autoloaded_library_uris.lock().await.is_empty());
    }

    /// Build the three-file #923 workspace: `::mymod::helper`, an unrelated
    /// `::other::helper`, and an `app.tcl` that reaches `::mymod::helper` via
    /// `namespace path`.  Returns the backend and the three URIs.
    async fn register_namespace_path_workspace() -> (Backend, Uri, Uri, Uri) {
        let backend = test_backend();
        let mymod = Uri::from_str("file:///mymod.tcl").unwrap();
        let other = Uri::from_str("file:///other.tcl").unwrap();
        let app = Uri::from_str("file:///app.tcl").unwrap();
        register(
            &backend,
            &mymod,
            "namespace eval ::mymod { proc helper {} {} }\n",
        )
        .await;
        register(
            &backend,
            &other,
            "namespace eval ::other { proc helper {} {} }\n",
        )
        .await;
        register(
            &backend,
            &app,
            "namespace eval ::app {\n    namespace path ::mymod\n    proc run {} { helper }\n}\n",
        )
        .await;
        (backend, mymod, other, app)
    }

    #[tokio::test]
    async fn cross_document_references_resolve_namespace_path_collision() {
        // The confirmed #923 trigger: references on `::mymod::helper`'s
        // declaration must include the bare `helper` call in app.tcl (reached
        // via `namespace path`), even though `::other` defines the same simple
        // name and the call's file-local guess settles to `::app::helper`.
        let (backend, mymod, _other, app) = register_namespace_path_workspace().await;
        let mymod_src = "namespace eval ::mymod { proc helper {} {} }\n";
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(mymod_src, "tcl8.6").clone()
        };
        // Cursor on `helper` in `::mymod`'s declaration (col 30).
        let refs = backend
            .cross_document_references(&mymod, mymod_src, &analysis, Position::new(0, 32), false)
            .await;
        assert_eq!(refs.len(), 1, "{refs:?}");
        assert_eq!(refs[0].uri, app);
    }

    #[tokio::test]
    async fn cross_document_references_do_not_cross_link_colliding_namespace() {
        // References on the *unrelated* `::other::helper` must be empty: the
        // app.tcl call resolves to `::mymod::helper` via the namespace path, so
        // it is never a reference of `::other::helper`.
        let (backend, _mymod, other, _app) = register_namespace_path_workspace().await;
        let other_src = "namespace eval ::other { proc helper {} {} }\n";
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(other_src, "tcl8.6").clone()
        };
        let refs = backend
            .cross_document_references(&other, other_src, &analysis, Position::new(0, 32), false)
            .await;
        assert!(refs.is_empty(), "{refs:?}");
    }

    /// Build a two-file `namespace import` workspace: `::mymod` exports
    /// `helper`, `::app` imports it and calls a bare `helper`.
    async fn register_namespace_import_workspace() -> (Backend, Uri, Uri) {
        let backend = test_backend();
        let mymod = Uri::from_str("file:///mymod.tcl").unwrap();
        let app = Uri::from_str("file:///app.tcl").unwrap();
        register(
            &backend,
            &mymod,
            "namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n",
        )
        .await;
        register(
            &backend,
            &app,
            "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\n",
        )
        .await;
        (backend, mymod, app)
    }

    #[tokio::test]
    async fn cross_document_references_follow_namespace_import() {
        // References on `::mymod::helper` must include app.tcl's imported bare
        // call (reached through the import link) and the `namespace import`
        // pattern token that names it.
        let (backend, mymod, app) = register_namespace_import_workspace().await;
        let mymod_src = "namespace eval ::mymod { proc helper {} {}\n namespace export helper }\n";
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(mymod_src, "tcl8.6").clone()
        };
        // Cursor on `helper` in `::mymod`'s declaration (col 32).
        let refs = backend
            .cross_document_references(&mymod, mymod_src, &analysis, Position::new(0, 32), false)
            .await;
        assert_eq!(refs.len(), 2, "imported call + import pattern: {refs:?}");
        assert!(refs.iter().all(|l| l.uri == app));
    }

    #[tokio::test]
    async fn cross_document_references_from_imported_call_reach_source() {
        // A cursor on the imported bare `helper` call resolves *through* the
        // import to `::mymod::helper`, so its references gather with the source
        // declaration's — go-to-references works from the consumer side too.
        let (backend, mymod, app) = register_namespace_import_workspace().await;
        let app_src = "namespace eval ::app {\n    namespace import ::mymod::helper\n    proc run {} { helper }\n}\n";
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(app_src, "tcl8.6").clone()
        };
        // Cursor on the imported `helper` call (line 2, col 20), with the
        // declaration included.
        let refs = backend
            .cross_document_references(&app, app_src, &analysis, Position::new(2, 20), true)
            .await;
        assert!(
            refs.iter().any(|l| l.uri == mymod),
            "imported call should reach the source declaration: {refs:?}",
        );
    }

    #[tokio::test]
    async fn cross_document_definition_follows_namespace_path_not_collision() {
        // Go-to-definition on the bare `helper` call in app.tcl jumps to
        // `::mymod::helper` (reached via `namespace path`), never the same-named
        // `::other::helper` — the previous simple-name lookup surfaced both.
        let (backend, mymod, _other, app) = register_namespace_path_workspace().await;
        let app_src =
            "namespace eval ::app {\n    namespace path ::mymod\n    proc run {} { helper }\n}\n";
        let analysis = {
            let mut a = Analyser::new();
            a.analyse(app_src, "tcl8.6").clone()
        };
        // Cursor on the `helper` call in `run`'s body (line 2, col 20).
        let defs = backend
            .cross_document_definition(&app, app_src, Position::new(2, 20), &analysis)
            .await
            .expect("ok");
        assert_eq!(defs.len(), 1, "{defs:?}");
        assert_eq!(defs[0].uri, mymod);
    }

    #[tokio::test]
    async fn code_lens_resolve_wires_show_references_command() {
        // Regression for #724: the proc reference-count lens must resolve to a
        // *clickable* `tcl-lsp.showReferences` command carrying the URI,
        // anchor position, and reference locations — not a bare, inert title.
        let backend = test_backend();
        let uri = Uri::from_str("file:///refs.tcl").unwrap();
        // `helper` defined once and called twice → 2 references.
        register(&backend, &uri, "proc helper {} {}\nhelper\nhelper\n").await;
        let lens_params = CodeLensParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let lenses = backend
            .code_lens(lens_params)
            .await
            .expect("ok")
            .expect("some lenses");
        let lens = lenses
            .into_iter()
            .find(|l| {
                l.data
                    .as_ref()
                    .and_then(serde_json::Value::as_object)
                    .and_then(|d| d.get("qname"))
                    .and_then(serde_json::Value::as_str)
                    == Some("::helper")
            })
            .expect("a lens for ::helper");
        let resolved = backend.code_lens_resolve(lens).await.expect("resolve ok");
        let command = resolved.command.expect("resolved lens carries a command");
        assert_eq!(
            command.command, "tcl-lsp.showReferences",
            "lens must invoke the show-references wrapper, got {command:?}",
        );
        assert!(
            command.title.contains("references"),
            "title should show the count: {command:?}",
        );
        let args = command.arguments.expect("showReferences needs arguments");
        // [uriString, position, locations]
        assert_eq!(args.len(), 3, "{args:?}");
        assert_eq!(args[0], serde_json::Value::String(uri.to_string()));
        let locations = args[2].as_array().expect("locations array");
        assert_eq!(locations.len(), 2, "two call sites: {locations:?}");
    }

    #[tokio::test]
    async fn references_handler_merges_local_and_cross_document() {
        let backend = test_backend();
        let lib = Uri::from_str("file:///lib.tcl").unwrap();
        let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
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
        assert_eq!(result.iter().filter(|l| l.uri == lib).count(), 2);
    }

    #[tokio::test]
    async fn references_handler_bigip_conf_finds_path_occurrences() {
        // A BIG-IP `.conf` document (dialect `f5-bigip`) routes references
        // through the TMSH-path textual search, not the Tcl analyser.
        let backend = test_backend();
        let uri = Uri::from_str("file:///bigip.conf").unwrap();
        let src = "ltm pool /Common/p {\n}\nltm virtual /Common/v {\n    pool /Common/p\n}\n";
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "f5-bigip".to_owned()),
        );
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                // On `/Common/p` in the virtual's `pool /Common/p` line.
                position: Position::new(3, 9),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration: true,
            },
        };
        let result = backend.references(params).await.expect("ok").expect("some");
        // Pool declaration (line 0) + the virtual's `pool /Common/p` (line 3).
        assert_eq!(result.len(), 2, "{result:?}");
        assert!(result.iter().all(|l| l.uri == uri));
        let lines: Vec<u32> = result.iter().map(|l| l.range.start.line).collect();
        assert!(lines.contains(&0) && lines.contains(&3), "{lines:?}");
    }

    #[tokio::test]
    async fn document_link_handler_bigip_conf_links_irule_pool_ref() {
        // A BIG-IP `.conf` document routes document links through the object-
        // reference resolver: the iRule's `pool /Common/web_pool` resolves to
        // the pool stanza on line 0 (a `#L1` fragment target).
        let backend = test_backend();
        let uri = Uri::from_str("file:///bigip.conf").unwrap();
        let src = "ltm pool /Common/web_pool { }\nltm rule /Common/r {\nwhen HTTP_REQUEST { pool /Common/web_pool }\n}\n";
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "f5-bigip".to_owned()),
        );
        let params = DocumentLinkParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let result = backend
            .document_link(params)
            .await
            .expect("ok")
            .expect("some");
        let link = result
            .iter()
            .find(|l| l.tooltip.as_deref() == Some("Go to /Common/web_pool"))
            .expect("pool ref link");
        let target = link.target.as_ref().expect("resolved target");
        assert!(target.as_str().contains("#L1"), "target = {target:?}");
    }

    #[tokio::test]
    async fn code_action_handler_bigip_conf_offers_partition_rename() {
        // A BIG-IP `.conf` document routes code actions through the dialect
        // provider (`tcl-lsp-core::bigip_code_actions`): an `auth partition`
        // stanza offers the `tclLsp.renamePartition` command, which the generic
        // Tcl code-action path never would. Guards the provider's wiring into
        // `Backend::code_action`.
        let backend = test_backend();
        let uri = Uri::from_str("file:///bigip.conf").unwrap();
        let src = "auth partition Team1 {\n    description test\n}\n";
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "f5-bigip".to_owned()),
        );
        let params = CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            context: tower_lsp_server::ls_types::CodeActionContext::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let result = backend
            .code_action(params)
            .await
            .expect("ok")
            .expect("some");
        let has_rename_partition = result.iter().any(|a| match a {
            CodeActionOrCommand::CodeAction(ca) => ca
                .command
                .as_ref()
                .is_some_and(|c| c.command == "tclLsp.renamePartition"),
            CodeActionOrCommand::Command(c) => c.command == "tclLsp.renamePartition",
        });
        assert!(
            has_rename_partition,
            "expected a renamePartition code action: {result:?}",
        );
    }

    #[tokio::test]
    async fn rename_edits_span_multiple_documents() {
        let backend = test_backend();
        let lib = Uri::from_str("file:///lib.tcl").unwrap();
        let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
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

    /// A rename triggered from a **consumer** document — the command's
    /// definition lives only in a sibling — resolves through the workspace
    /// oracle and rewrites the sibling declaration plus every call site,
    /// including the consumer's own (M8's rename leg).
    #[tokio::test]
    async fn rename_from_a_consumer_document_rewrites_the_defining_sibling() {
        let backend = test_backend();
        let lib = Uri::from_str("file:///lib.tcl").unwrap();
        let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\nhelper\n").await;
        register(&backend, &consumer, "helper\nhelper\n").await;
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: consumer.clone(),
                },
                position: Position::new(0, 2),
            },
            new_name: "do_it".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let edit = backend.rename(params).await.expect("ok").expect("some");
        let changes = edit.changes.expect("changes");
        // lib.tcl: declaration + its own call site; consumer.tcl: both calls.
        assert_eq!(
            changes.get(&lib).map(Vec::len),
            Some(2),
            "lib decl + call: {changes:?}"
        );
        assert_eq!(
            changes.get(&consumer).map(Vec::len),
            Some(2),
            "consumer call sites: {changes:?}"
        );
    }

    /// Collision discipline holds on the consumer-document path: renaming onto
    /// a name the workspace already defines is refused wholesale — no partial
    /// edit set leaks out.
    #[tokio::test]
    async fn rename_from_a_consumer_document_refuses_a_workspace_collision() {
        let backend = test_backend();
        let lib = Uri::from_str("file:///lib.tcl").unwrap();
        let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
        register(&backend, &lib, "proc helper {} {}\nproc do_it {} {}\n").await;
        register(&backend, &consumer, "helper\n").await;
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: consumer.clone(),
                },
                position: Position::new(0, 2),
            },
            new_name: "do_it".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let edit = backend.rename(params).await.expect("ok");
        assert!(
            edit.is_none(),
            "a collision with an existing workspace command must refuse: {edit:?}"
        );
    }

    #[tokio::test]
    async fn method_rename_spans_override_family_across_documents() {
        // Animal::speak in animal.tcl; Dog (subclass) overrides it in
        // dog.tcl and calls it via `$d speak`.  Renaming from the base decl
        // must rewrite the override *and* the external call site in the
        // sibling document.
        let backend = test_backend();
        let animal = Uri::from_str("file:///animal.tcl").unwrap();
        let dog = Uri::from_str("file:///dog.tcl").unwrap();
        register(
            &backend,
            &animal,
            "oo::class create Animal {\n    method speak {} {}\n}\n",
        )
        .await;
        register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n    method speak {} {}\n}\nset d [Dog new]\n$d speak\n",
        )
        .await;
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: animal.clone(),
                },
                position: Position::new(1, 11), // on `speak` in Animal's decl
            },
            new_name: "vocalise".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let edit = backend.rename(params).await.expect("ok").expect("some");
        let changes = edit.changes.expect("changes");
        assert!(
            changes.contains_key(&animal),
            "base doc edits missing: {changes:?}"
        );
        assert!(
            changes.contains_key(&dog),
            "override doc edits missing: {changes:?}"
        );
        // Every edit renames to `vocalise`.
        assert!(changes.values().flatten().all(|e| e.new_text == "vocalise"));
        // dog.tcl: the override declaration (line 2) + the `$d speak` call
        // site (line 5) are both rewritten.
        let dog_lines: Vec<u32> = changes[&dog].iter().map(|e| e.range.start.line).collect();
        assert!(
            dog_lines.contains(&2) && dog_lines.contains(&5),
            "expected override decl (l2) + call site (l5) in dog.tcl; got {:?}",
            changes[&dog],
        );
    }

    #[tokio::test]
    async fn method_rename_reaches_subclass_only_document() {
        // Animal::speak in animal.tcl; Dog *inherits* speak (no override) in
        // dog.tcl and calls it via `my speak` and `$d speak`.  dog.tcl holds
        // no definer, so the family-only pass never opened it; the inheritor
        // pass must, or those sites silently keep the old name.
        let backend = test_backend();
        let animal = Uri::from_str("file:///animal.tcl").unwrap();
        let dog = Uri::from_str("file:///dog.tcl").unwrap();
        register(
            &backend,
            &animal,
            "oo::class create Animal {\n    method speak {} {}\n}\n",
        )
        .await;
        register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n    method describe {} { my speak }\n}\nset d [Dog new]\n$d speak\n",
        )
        .await;
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: animal.clone(),
                },
                position: Position::new(1, 11), // on `speak` in Animal's decl
            },
            new_name: "vocalise".to_owned(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        let edit = backend.rename(params).await.expect("ok").expect("some");
        let changes = edit.changes.expect("changes");
        assert!(
            changes.contains_key(&animal),
            "base doc edits missing: {changes:?}"
        );
        assert!(
            changes.contains_key(&dog),
            "subclass-only doc edits missing: {changes:?}",
        );
        assert!(changes.values().flatten().all(|e| e.new_text == "vocalise"));
        // dog.tcl: `my speak` (line 2) + `$d speak` (line 5) rewritten; there
        // is no declaration to rewrite in this file.
        let dog_lines: Vec<u32> = changes[&dog].iter().map(|e| e.range.start.line).collect();
        assert!(
            dog_lines.contains(&2) && dog_lines.contains(&5),
            "expected `my speak` (l2) + `$d speak` (l5) in dog.tcl; got {:?}",
            changes[&dog],
        );
    }

    /// Register `animal.tcl` (`Animal::speak`) and `dog.tcl` (a `Dog` subclass
    /// that overrides `speak` and calls `$d speak`).  Returns the backend and
    /// the two URIs.
    async fn register_method_family_workspace() -> (Backend, Uri, Uri) {
        let backend = test_backend();
        let animal = Uri::from_str("file:///animal.tcl").unwrap();
        let dog = Uri::from_str("file:///dog.tcl").unwrap();
        register(
            &backend,
            &animal,
            "oo::class create Animal {\n    method speak {} {}\n}\n",
        )
        .await;
        register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n    method speak {} {}\n}\nset d [Dog new]\n$d speak\n",
        )
        .await;
        (backend, animal, dog)
    }

    #[tokio::test]
    async fn cross_file_method_references_span_override_family() {
        // References on `Animal::speak` reach the override declaration and the
        // `$d speak` call site in the sibling dog.tcl — previously TclOO methods
        // had no cross-file reference support at all.
        let (backend, animal, dog) = register_method_family_workspace().await;
        let refs = backend
            .cross_file_method_references(&animal, "::Animal", "speak", true)
            .await;
        let dog_lines: Vec<u32> = refs
            .iter()
            .filter(|l| l.uri == dog)
            .map(|l| l.range.start.line)
            .collect();
        assert!(
            dog_lines.contains(&2),
            "override decl (l2) missing: {refs:?}"
        );
        assert!(dog_lines.contains(&5), "`$d speak` (l5) missing: {refs:?}");
        // The current document is excluded — the single-document provider
        // already covers its own method sites.
        assert!(
            refs.iter().all(|l| l.uri != animal),
            "current document must be excluded: {refs:?}",
        );
    }

    #[tokio::test]
    async fn cross_file_method_references_exclude_declaration() {
        // With `include_declaration` false, the sibling's override *declaration*
        // (l2) is dropped but the `$d speak` call site (l5) stays.
        let (backend, animal, dog) = register_method_family_workspace().await;
        let refs = backend
            .cross_file_method_references(&animal, "::Animal", "speak", false)
            .await;
        let dog_lines: Vec<u32> = refs
            .iter()
            .filter(|l| l.uri == dog)
            .map(|l| l.range.start.line)
            .collect();
        assert!(!dog_lines.contains(&2), "decl should be excluded: {refs:?}");
        assert!(dog_lines.contains(&5), "call site (l5) missing: {refs:?}");
    }

    #[tokio::test]
    async fn cross_file_method_references_reach_inheritor_document() {
        // Dog *inherits* speak (no override) and calls it via `my speak` and
        // `$d speak` in a document holding no definer.  The inheritor pass must
        // still reach those sites.
        let backend = test_backend();
        let animal = Uri::from_str("file:///animal.tcl").unwrap();
        let dog = Uri::from_str("file:///dog.tcl").unwrap();
        register(
            &backend,
            &animal,
            "oo::class create Animal {\n    method speak {} {}\n}\n",
        )
        .await;
        register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n    method run {} { my speak }\n}\nset d [Dog new]\n$d speak\n",
        )
        .await;
        let refs = backend
            .cross_file_method_references(&animal, "::Animal", "speak", false)
            .await;
        let dog_lines: Vec<u32> = refs
            .iter()
            .filter(|l| l.uri == dog)
            .map(|l| l.range.start.line)
            .collect();
        // `my speak` (l2) and `$d speak` (l5).
        assert!(dog_lines.contains(&2), "`my speak` (l2) missing: {refs:?}");
        assert!(dog_lines.contains(&5), "`$d speak` (l5) missing: {refs:?}");
    }

    #[tokio::test]
    async fn cross_file_method_references_empty_for_unrelated_method() {
        // A method name no family class defines yields nothing — no spurious
        // cross-file sites.
        let (backend, animal, _dog) = register_method_family_workspace().await;
        let refs = backend
            .cross_file_method_references(&animal, "::Animal", "nonexistent", true)
            .await;
        assert!(refs.is_empty(), "{refs:?}");
    }

    #[tokio::test]
    async fn references_handler_includes_cross_file_method_sites() {
        // End-to-end through the `references` handler: the cursor on
        // `Animal::speak`'s declaration surfaces the override + `$d speak` in
        // dog.tcl alongside the current document's own sites.
        let (backend, animal, dog) = register_method_family_workspace().await;
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: animal.clone(),
                },
                position: Position::new(1, 11), // on `speak` in Animal's decl
            },
            context: ReferenceContext {
                include_declaration: true,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let refs = backend
            .references(params)
            .await
            .expect("ok")
            .expect("some references");
        assert!(
            refs.iter().any(|l| l.uri == dog && l.range.start.line == 5),
            "cross-file `$d speak` (dog.tcl l5) missing: {refs:?}",
        );
        assert!(
            refs.iter().any(|l| l.uri == animal),
            "current-document declaration missing: {refs:?}",
        );
    }

    #[tokio::test]
    async fn cross_file_method_definition_jumps_to_inherited_declaration() {
        // `Dog` inherits `speak` from `Animal` in another file.  Go-to-def on a
        // `Dog` instance's `speak` call resolves to `Animal::speak`'s
        // declaration in animal.tcl.
        let backend = test_backend();
        let animal = Uri::from_str("file:///animal.tcl").unwrap();
        let dog = Uri::from_str("file:///dog.tcl").unwrap();
        register(
            &backend,
            &animal,
            "oo::class create Animal {\n    method speak {} {}\n}\n",
        )
        .await;
        register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n}\nset d [Dog new]\n$d speak\n",
        )
        .await;
        let defs = backend
            .cross_file_method_definition(
                &dog,
                "::Dog",
                "speak",
                core_workspace_index::MethodAccess::External,
            )
            .await;
        assert_eq!(defs.len(), 1, "{defs:?}");
        assert_eq!(defs[0].uri, animal);
        assert_eq!(defs[0].range.start.line, 1, "{defs:?}");
    }

    #[tokio::test]
    async fn goto_definition_follows_inherited_method_cross_file() {
        // End-to-end through `compute_definition`: the cursor on `$d speak`
        // (dog.tcl) jumps to the inherited `Animal::speak` declaration in
        // animal.tcl — a method-call cursor is not a command head, so the
        // dedicated method path (not the command-head cross-doc fallback)
        // resolves it.
        let backend = test_backend();
        let animal = Uri::from_str("file:///animal.tcl").unwrap();
        let dog = Uri::from_str("file:///dog.tcl").unwrap();
        register(
            &backend,
            &animal,
            "oo::class create Animal {\n    method speak {} {}\n}\n",
        )
        .await;
        register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n}\nset d [Dog new]\n$d speak\n",
        )
        .await;
        // `$d speak` is on line 4; `speak` starts at column 3.
        let defs = backend
            .compute_definition(&dog, Position::new(4, 4))
            .await
            .expect("ok");
        assert_eq!(defs.len(), 1, "{defs:?}");
        assert_eq!(defs[0].uri, animal);
        assert_eq!(defs[0].range.start.line, 1, "{defs:?}");
    }

    #[tokio::test]
    async fn cross_file_definition_selects_the_dispatch_entry_945() {
        // Issue #945 fault 6: with `Animal::speak` overridden by
        // `Dog::speak` in another file, a `Dog` receiver's definition
        // request identifies the runtime entry (`Dog::speak`) only —
        // never the whole override family (tclsh 9.0.4: `info object
        // call` = Dog then Animal).
        let backend = test_backend();
        let animal = Uri::from_str("file:///animal.tcl").unwrap();
        let dog = Uri::from_str("file:///dog.tcl").unwrap();
        let main = Uri::from_str("file:///main.tcl").unwrap();
        register(
            &backend,
            &animal,
            "oo::class create Animal {\n    method speak {} { return animal }\n}\n",
        )
        .await;
        register(
            &backend,
            &dog,
            "oo::class create Dog {\n    superclass Animal\n    method speak {} { return dog }\n}\n",
        )
        .await;
        register(&backend, &main, "set d [Dog new]\n$d speak\n").await;
        let defs = backend
            .compute_definition(&main, Position::new(1, 4))
            .await
            .expect("ok");
        assert_eq!(
            defs.len(),
            1,
            "go-to-definition must select Dog::speak only: {defs:?}"
        );
        assert_eq!(defs[0].uri, dog, "{defs:?}");
        assert_eq!(defs[0].range.start.line, 2, "{defs:?}");
    }

    #[tokio::test]
    async fn cross_file_definition_refuses_an_unexported_method_945() {
        // Issue #945 fault 4: `Vault::_secret` is default-unexported
        // (tclsh 9.0.4: `unknown method "_secret"`), so a consumer file's
        // external `$v _secret` resolves to nothing — cross-file
        // navigation must not resolve what C rejects.
        let backend = test_backend();
        let vault = Uri::from_str("file:///vault.tcl").unwrap();
        let main = Uri::from_str("file:///main.tcl").unwrap();
        register(
            &backend,
            &vault,
            "oo::class create Vault {\n    method _secret {} { return hidden }\n}\n",
        )
        .await;
        register(&backend, &main, "set v [Vault new]\n$v _secret\n").await;
        let defs = backend
            .compute_definition(&main, Position::new(1, 5))
            .await
            .expect("ok");
        assert!(
            defs.is_empty(),
            "an externally unexported TclOO method is not callable: {defs:?}"
        );
    }

    /// Register `animal.tcl` (`Animal::speak`) and a pure-consumer `main.tcl`
    /// that only creates and uses an `Animal` instance — it defines no part of
    /// the class.
    async fn register_consumer_workspace() -> (Backend, Uri, Uri) {
        let backend = test_backend();
        let animal = Uri::from_str("file:///animal.tcl").unwrap();
        let main = Uri::from_str("file:///main.tcl").unwrap();
        register(
            &backend,
            &animal,
            "oo::class create Animal {\n    method speak {} {}\n}\n",
        )
        .await;
        register(&backend, &main, "set a [Animal new]\n$a speak\n").await;
        (backend, animal, main)
    }

    #[tokio::test]
    async fn references_reach_pure_consumer_document() {
        // References on `Animal::speak` include the `$a speak` call in the
        // pure-consumer main.tcl, which defines no part of `Animal` and so is
        // invisible to the family/inheritor pass — only the workspace class
        // oracle resolves `a` to `::Animal` there.
        let (backend, animal, main) = register_consumer_workspace().await;
        let refs = backend
            .cross_file_consumer_method_references(
                &animal,
                "oo::class create Animal {\n    method speak {} {}\n}\n",
                "tcl8.6",
                "::Animal",
                "speak",
            )
            .await;
        assert!(
            refs.iter()
                .any(|l| l.uri == main && l.range.start.line == 1),
            "`$a speak` in the consumer document missing: {refs:?}",
        );
    }

    #[tokio::test]
    async fn references_handler_from_consumer_cursor_finds_family() {
        // End-to-end: the cursor on `$a speak` in the pure-consumer main.tcl
        // resolves the method oracle-aware and finds `Animal::speak`'s
        // declaration in animal.tcl plus its own call site.
        let (backend, animal, main) = register_consumer_workspace().await;
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: main.clone() },
                position: Position::new(1, 4), // on `speak` in `$a speak`
            },
            context: ReferenceContext {
                include_declaration: true,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let refs = backend
            .references(params)
            .await
            .expect("ok")
            .expect("some references");
        assert!(
            refs.iter()
                .any(|l| l.uri == animal && l.range.start.line == 1),
            "cross-file declaration `Animal::speak` missing: {refs:?}",
        );
        assert!(
            refs.iter()
                .any(|l| l.uri == main && l.range.start.line == 1),
            "consumer call `$a speak` missing: {refs:?}",
        );
    }

    #[tokio::test]
    async fn goto_definition_from_consumer_cursor_finds_cross_file_method() {
        // Go-to-definition on `$a speak` in the pure-consumer main.tcl resolves
        // `a`'s class cross-file (oracle-aware) and jumps to `Animal::speak`.
        let (backend, animal, main) = register_consumer_workspace().await;
        let defs = backend
            .compute_definition(&main, Position::new(1, 4))
            .await
            .expect("ok");
        assert_eq!(defs.len(), 1, "{defs:?}");
        assert_eq!(defs[0].uri, animal);
        assert_eq!(defs[0].range.start.line, 1, "{defs:?}");
    }

    /// Build a backend with one document registered, then disable the named
    /// `tclLsp.features.*` toggle.  Returns the backend plus the document
    /// identifier and a cursor-position params for the request handlers.
    async fn backend_with_feature_disabled(
        feature: &str,
    ) -> (Backend, TextDocumentIdentifier, TextDocumentPositionParams) {
        let backend = test_backend();
        let uri = Uri::from_str("file:///g.tcl").unwrap();
        register(&backend, &uri, "proc helper {} {}\nhelper\n").await;
        backend
            .feature_toggles
            .lock()
            .await
            .apply(serde_json::json!({ feature: false }).as_object().unwrap());
        let td = TextDocumentIdentifier { uri };
        let pos_params = TextDocumentPositionParams {
            text_document: td.clone(),
            position: Position::new(0, 5),
        };
        (backend, td, pos_params)
    }

    #[tokio::test]
    async fn disabled_semantic_tokens_toggle_yields_none() {
        let (backend, td, _) = backend_with_feature_disabled("semanticTokens").await;
        assert!(
            backend
                .semantic_tokens_full(SemanticTokensParams {
                    text_document: td,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .expect("ok")
                .is_none(),
            "semanticTokens disabled must yield None",
        );
    }

    /// Negative: with the `semanticTokens` toggle left at its default (enabled),
    /// the same handler returns a token set — proving the gate, not the handler,
    /// is what suppresses the result above.
    #[tokio::test]
    async fn enabled_semantic_tokens_toggle_yields_some() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///g.tcl").unwrap();
        register(&backend, &uri, "proc helper {} {}\nhelper\n").await;
        assert!(
            backend
                .semantic_tokens_full(SemanticTokensParams {
                    text_document: TextDocumentIdentifier { uri },
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .expect("ok")
                .is_some(),
            "semanticTokens enabled must yield a token set",
        );
    }

    /// Issue #829: for an *indexed* document (a real `didOpen`-style session,
    /// via `db_set_source`), `semantic_tokens_full` must always serve one of
    /// the two well-defined tiers `semantic_tokens_core_data` can produce —
    /// the enriched result (when the race in `SEMANTIC_TOKENS_FAST_PATH_BUDGET`
    /// is won) or the cheap coarse tier (when it is not) — never anything
    /// else (garbage, empty, or a third shape). Deliberately does not assert
    /// *which* tier wins: `spawn_blocking` scheduling latency under a loaded
    /// test runtime can legitimately push even a trivial document past the
    /// budget, which is the fast-path/fallback design working as intended,
    /// not a bug — `semantic_tokens_retags_constant_regex_source_true_positive`
    /// (tcl-lsp-db) is the deterministic proof that the enriched tier itself
    /// produces the richer result when computed directly.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn semantic_tokens_full_serves_a_well_defined_tier_when_indexed() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///indexed.tcl").unwrap();
        let src = "set my_re \".*abc\"\nregexp $my_re $s\n";
        backend
            .db_set_source(&uri, src.to_owned(), "tcl9.0".to_owned())
            .await;
        backend.documents.lock().await.insert(
            uri.clone(),
            DocumentState::new(src.to_owned(), "tcl9.0".to_owned()),
        );

        let served = backend
            .semantic_tokens_full(SemanticTokensParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            })
            .await
            .expect("ok")
            .expect("semanticTokens enabled must yield a token set");
        let SemanticTokensResult::Tokens(served) = served else {
            panic!("expected a full token stream, not a delta");
        };

        let enriched = backend
            .db_semantic_tokens(&uri)
            .await
            .expect("indexed document has a JoinHandle")
            .await
            .expect("worker did not panic")
            .expect("not cancelled");
        let registry = backend.registry_for_dialect("tcl9.0").await;
        let coarse = core_semantic_tokens::full(src, "tcl9.0", &registry);

        assert!(!served.data.is_empty(), "must never serve an empty stream");
        assert!(
            served.data == lift_semantic_token_data(&enriched.data)
                || served.data == lift_semantic_token_data(&coarse.data),
            "served tokens must be exactly the enriched tier or exactly the \
             coarse tier, never a third shape",
        );
    }

    /// Direct unit coverage of `SemanticTokensRefreshCtx::deliver_if_changed`
    /// — the detached continuation's only logic. It must never write the
    /// shared cache (only a served `full`/`full/delta` response does that),
    /// regardless of whether the landed result matches what is cached.
    #[tokio::test]
    async fn semantic_tokens_refresh_ctx_never_mutates_the_cache() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///refresh-ctx.tcl").unwrap();
        let ctx = SemanticTokensRefreshCtx {
            client: backend.client.clone(),
            last_semantic_tokens: Arc::clone(&backend.last_semantic_tokens),
            refresh_pending: Arc::clone(&backend.semantic_tokens_refresh_pending),
        };

        // No cache entry yet: "changed" (there is nothing to match), but still
        // must not write the cache itself.
        ctx.deliver_if_changed(&uri, &[1, 2, 3]).await;
        assert!(
            backend
                .last_semantic_tokens
                .lock()
                .await
                .get(&uri)
                .is_none(),
            "deliver_if_changed must never write the semantic-tokens cache",
        );

        // Seed a cache entry as if a coarse response had just been served,
        // then land an enriched result that matches it exactly.
        backend
            .last_semantic_tokens
            .lock()
            .await
            .insert(uri.clone(), ("coarse-result-id".to_owned(), vec![1, 2, 3]));
        ctx.deliver_if_changed(&uri, &[1, 2, 3]).await;
        assert_eq!(
            backend.last_semantic_tokens.lock().await.get(&uri).cloned(),
            Some(("coarse-result-id".to_owned(), vec![1, 2, 3])),
            "an unchanged landed result must leave the served cache entry \
             exactly as it was",
        );

        // A landed result that differs from what was served: still must not
        // overwrite the cache — only the next served response does that.
        ctx.deliver_if_changed(&uri, &[9, 9, 9]).await;
        assert_eq!(
            backend.last_semantic_tokens.lock().await.get(&uri).cloned(),
            Some(("coarse-result-id".to_owned(), vec![1, 2, 3])),
            "deliver_if_changed must not overwrite the cache even when the \
             landed result differs from what was served",
        );
    }

    /// A fire already scheduled must absorb a second request rather than
    /// scheduling a duplicate: the `workspace/semanticTokens/refresh`
    /// notification carries no data, so a client that receives it re-pulls
    /// current tokens for every open document — any other document's
    /// enriched result landing during the debounce window rides along with
    /// the fire already scheduled. Guards against the "many cold large tabs
    /// finish around the same time" thundering-herd case.
    #[tokio::test]
    async fn semantic_tokens_refresh_ctx_dedupes_concurrent_refreshes() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///refresh-dedup.tcl").unwrap();
        let ctx = SemanticTokensRefreshCtx {
            client: backend.client.clone(),
            last_semantic_tokens: Arc::clone(&backend.last_semantic_tokens),
            refresh_pending: Arc::clone(&backend.semantic_tokens_refresh_pending),
        };

        // Simulate a fire already scheduled (e.g. by a different document's
        // enriched result landing moments earlier).
        backend
            .semantic_tokens_refresh_pending
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // A changed result must ride along with the fire already scheduled,
        // not schedule a second one.
        ctx.deliver_if_changed(&uri, &[1, 2, 3]).await;
        assert!(
            backend
                .semantic_tokens_refresh_pending
                .load(std::sync::atomic::Ordering::Relaxed),
            "a fire already scheduled must be left untouched, not \
             scheduled a second time",
        );

        // Reset to simulate that fire having landed, then let a new changed
        // result schedule its own.
        backend
            .semantic_tokens_refresh_pending
            .store(false, std::sync::atomic::Ordering::Relaxed);
        ctx.deliver_if_changed(&uri, &[4, 5, 6]).await;
        assert!(
            backend
                .semantic_tokens_refresh_pending
                .load(std::sync::atomic::Ordering::Relaxed),
            "scheduling a fire must set the flag immediately, before its \
             debounce window elapses",
        );

        // Once the debounce window elapses and the fire completes, the flag
        // must clear so a later change can schedule the next one.
        tokio::time::sleep(SEMANTIC_TOKENS_REFRESH_DEBOUNCE * 2).await;
        assert!(
            !backend
                .semantic_tokens_refresh_pending
                .load(std::sync::atomic::Ordering::Relaxed),
            "the flag must clear once the debounced fire completes, so a \
             later change can schedule the next one",
        );
    }

    /// A continuation whose document closes while it is still running lands
    /// against an evicted cache entry (`did_close` removes it). This pins the
    /// current, deliberate behaviour: `deliver_if_changed` treats the missing
    /// entry as "changed" and schedules a refresh for a now-closed document.
    /// That refresh is a harmless no-op (there is nothing left to re-request
    /// for this URI, and the workspace-wide push is dataless), so it is not
    /// worth distinguishing from the alternative reading of a missing cache
    /// entry — a genuine (if narrow) race where this continuation's result
    /// lands *before* `semantic_tokens_full`'s own cache write completes,
    /// where treating "missing" as "unchanged" would risk suppressing a
    /// refresh a still-open document genuinely needs.
    #[tokio::test]
    async fn semantic_tokens_refresh_ctx_after_did_close_is_harmless() {
        let backend = test_backend();
        let uri = Uri::from_str("file:///close-during-refresh.tcl").unwrap();
        register(&backend, &uri, "set x 1\n").await;
        backend
            .last_semantic_tokens
            .lock()
            .await
            .insert(uri.clone(), ("coarse-result-id".to_owned(), vec![1, 2, 3]));

        backend
            .did_close(DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
            })
            .await;
        assert!(
            backend
                .last_semantic_tokens
                .lock()
                .await
                .get(&uri)
                .is_none(),
            "did_close must have evicted the cache entry",
        );

        let ctx = SemanticTokensRefreshCtx {
            client: backend.client.clone(),
            last_semantic_tokens: Arc::clone(&backend.last_semantic_tokens),
            refresh_pending: Arc::clone(&backend.semantic_tokens_refresh_pending),
        };
        ctx.deliver_if_changed(&uri, &[9, 9, 9]).await;
        assert!(
            backend
                .semantic_tokens_refresh_pending
                .load(std::sync::atomic::Ordering::Relaxed),
            "a result landing for a closed document schedules a refresh -- \
             harmless (dataless, workspace-wide), not incorrect",
        );
        assert!(
            backend
                .last_semantic_tokens
                .lock()
                .await
                .get(&uri)
                .is_none(),
            "deliver_if_changed must not resurrect a cache entry for a \
             closed document",
        );
    }

    #[tokio::test]
    async fn disabled_code_actions_toggle_yields_none() {
        let (backend, td, _) = backend_with_feature_disabled("codeActions").await;
        assert!(
            backend
                .code_action(CodeActionParams {
                    text_document: td,
                    range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                    context: tower_lsp_server::ls_types::CodeActionContext::default(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .expect("ok")
                .is_none(),
            "codeActions disabled must yield None",
        );
    }

    #[tokio::test]
    async fn disabled_rename_toggle_yields_none() {
        let (backend, _, pos_params) = backend_with_feature_disabled("rename").await;
        assert!(
            backend
                .rename(RenameParams {
                    text_document_position: pos_params,
                    new_name: "do_it".to_owned(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await
                .expect("ok")
                .is_none(),
            "rename disabled must yield None",
        );
    }

    #[tokio::test]
    async fn disabled_document_highlight_toggle_yields_none() {
        let (backend, _, pos_params) = backend_with_feature_disabled("documentHighlight").await;
        assert!(
            backend
                .document_highlight(DocumentHighlightParams {
                    text_document_position_params: pos_params,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .expect("ok")
                .is_none(),
            "documentHighlight disabled must yield None",
        );
    }

    #[tokio::test]
    async fn disabled_code_lens_toggle_yields_none() {
        let (backend, td, _) = backend_with_feature_disabled("codeLens").await;
        assert!(
            backend
                .code_lens(CodeLensParams {
                    text_document: td,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .expect("ok")
                .is_none(),
            "codeLens disabled must yield None",
        );
    }

    #[tokio::test]
    async fn disabled_linked_editing_range_toggle_yields_none() {
        let (backend, _, pos_params) = backend_with_feature_disabled("linkedEditingRange").await;
        assert!(
            backend
                .linked_editing_range(LinkedEditingRangeParams {
                    text_document_position_params: pos_params,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await
                .expect("ok")
                .is_none(),
            "linkedEditingRange disabled must yield None",
        );
    }

    #[tokio::test]
    async fn disabled_call_hierarchy_toggle_yields_none() {
        let (backend, _, pos_params) = backend_with_feature_disabled("callHierarchy").await;
        assert!(
            backend
                .prepare_call_hierarchy(CallHierarchyPrepareParams {
                    text_document_position_params: pos_params,
                    work_done_progress_params: WorkDoneProgressParams::default(),
                })
                .await
                .expect("ok")
                .is_none(),
            "callHierarchy disabled must yield None",
        );
    }

    #[tokio::test]
    async fn disabled_workspace_symbols_toggle_yields_none() {
        let (backend, _, _) = backend_with_feature_disabled("workspaceSymbols").await;
        assert!(
            backend
                .symbol(WorkspaceSymbolParams {
                    query: "helper".to_owned(),
                    work_done_progress_params: WorkDoneProgressParams::default(),
                    partial_result_params: PartialResultParams::default(),
                })
                .await
                .expect("ok")
                .is_none(),
            "workspaceSymbols disabled must yield None",
        );
    }

    #[tokio::test]
    async fn diagnostics_master_switch_clears_the_report() {
        // `tclLsp.features.diagnostics = false` yields an empty diagnostic
        // report (clearing squiggles).
        let backend = test_backend();
        let uri = Uri::from_str("file:///d.tcl").unwrap();
        // Trailing whitespace → W112 (a source-style hint, on by default and
        // not opt-in) gives a guaranteed diagnostic to suppress.
        let src = "set x 1  \n";
        register(&backend, &uri, src).await;

        let on = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            !on.is_empty(),
            "expected at least one diagnostic with the feature on: {on:?}",
        );

        backend.feature_toggles.lock().await.apply(
            serde_json::json!({ "diagnostics": false })
                .as_object()
                .unwrap(),
        );
        let off = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            off.is_empty(),
            "diagnostics disabled must yield an empty report: {off:?}",
        );
    }

    #[tokio::test]
    async fn o111_brace_expr_hint_pairs_with_w100() {
        // An O111 "brace your expression" Information hint is paired with
        // every W100; the pairing is gated on the optimiser being enabled and
        // O111 not being disabled.
        let backend = test_backend();
        let uri = Uri::from_str("file:///o111.tcl").unwrap();
        let src = "set y [expr $a + $b]\n"; // unbraced expr → W100
        register(&backend, &uri, src).await;

        let on = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        let codes = |ds: &[tower_lsp_server::ls_types::Diagnostic]| -> Vec<String> {
            ds.iter()
                .filter_map(|d| match &d.code {
                    Some(tower_lsp_server::ls_types::NumberOrString::String(c)) => Some(c.clone()),
                    _ => None,
                })
                .collect()
        };
        let on_codes = codes(&on);
        assert!(
            on_codes.iter().any(|c| c == "W100"),
            "expected W100: {on_codes:?}"
        );
        assert!(
            on_codes.iter().any(|c| c == "O111"),
            "expected the paired O111 hint: {on_codes:?}",
        );

        // Disabling the optimiser drops the O111 hint but keeps W100.
        backend
            .apply_global_config(&serde_json::json!({ "optimiser": { "enabled": false } }))
            .await;
        let off = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        let off_codes = codes(&off);
        assert!(
            off_codes.iter().any(|c| c == "W100"),
            "W100 still expected: {off_codes:?}"
        );
        assert!(
            !off_codes.iter().any(|c| c == "O111"),
            "O111 must be gated off when the optimiser is disabled: {off_codes:?}",
        );
    }

    #[tokio::test]
    async fn style_line_length_setting_drives_w111() {
        // `tclLsp.style.lineLength` is the W111 threshold, distinct from the
        // formatter width. A short threshold makes an otherwise-fine line long.
        let backend = test_backend();
        let uri = Uri::from_str("file:///w111.tcl").unwrap();
        let src = "set x 1234567890\n"; // 16 chars on line 0
        register(&backend, &uri, src).await;

        // Default (120): no W111.
        let none = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            !none
                .iter()
                .any(|d| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W111")),
            "no W111 at the default threshold: {none:?}",
        );

        // Lower the style threshold to 8 → the 16-char line now trips W111.
        backend
            .apply_global_config(&serde_json::json!({ "style": { "lineLength": 8 } }))
            .await;
        assert_eq!(*backend.style_line_length.lock().await, 8);
        let some = backend
            .full_diagnostics_for(&uri, src.to_owned(), "tcl8.6".to_owned(), "tcl")
            .await;
        assert!(
            some.iter()
                .any(|d| matches!(&d.code, Some(tower_lsp_server::ls_types::NumberOrString::String(c)) if c == "W111")),
            "W111 should fire once the style line length is lowered: {some:?}",
        );
    }

    #[tokio::test]
    async fn rename_to_builtin_blocked_cross_document() {
        let backend = test_backend();
        let lib = Uri::from_str("file:///lib.tcl").unwrap();
        let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
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
        let main = Uri::from_str("file:///proj/main.tcl").unwrap();
        // main.tcl sources lib/old.tcl via a relative literal.
        register(&backend, &main, "source lib/old.tcl\nputs hi\n").await;
        let params = RenameFilesParams {
            files: vec![tower_lsp_server::ls_types::FileRename {
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
        let main = Uri::from_str("file:///proj/main.tcl").unwrap();
        register(&backend, &main, "source other.tcl\n").await;
        let params = RenameFilesParams {
            files: vec![tower_lsp_server::ls_types::FileRename {
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
        let old = Uri::from_str("file:///gone.tcl").unwrap();
        register(&backend, &old, "proc helper {} {}\n").await;
        assert_eq!(backend.workspace_index.read().await.procs().len(), 1);
        let params = RenameFilesParams {
            files: vec![tower_lsp_server::ls_types::FileRename {
                old_uri: old.to_string(),
                // New path doesn't exist on disk (test env) — reindex
                // drops the stale old entry and finds nothing to add.
                new_uri: "file:///moved.tcl".to_owned(),
            }],
        };
        backend.did_rename_files(params).await;
        // The old document's proc is gone from the index.
        let index = backend.workspace_index.read().await;
        assert!(index.procs().iter().all(|p| p.uri != old.as_str()));
    }

    #[tokio::test]
    async fn incoming_calls_span_multiple_documents() {
        let backend = test_backend();
        let lib = Uri::from_str("file:///lib.tcl").unwrap();
        let consumer = Uri::from_str("file:///consumer.tcl").unwrap();
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
        let lib = Uri::from_str("file:///lib.tcl").unwrap();
        let main = Uri::from_str("file:///main.tcl").unwrap();
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
