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
//! and is wrapped in an `LspService` by the binary. It is one of several
//! consumers of [`tcl_lsp_core`], alongside the `tcl` and `f5-query` CLIs,
//! `tcl-mcp` and the WASM hosts.
//!
//! LSP methods without a wired provider return
//! [`tower_lsp_server::jsonrpc::ErrorCode::MethodNotFound`].

// Retained analysis receipts require deeper auto-trait evaluation.
#![recursion_limit = "256"]
#![deny(missing_docs)]
#![forbid(unsafe_code)]
// Long linear fail-closed guard chains and scenario tests; splitting them is
// tracked as follow-up debt (Luna-41 notes) rather than done mid-merge.

/// The INI parse and the three-layer merge, shared with every surface
/// through `tcl-lsp-core`.
pub use tcl_lsp_core::config_ini;
mod current_diagnostics;
mod diagnostic_read_cancellation;
mod workspace_diagnostic_sources;
use workspace_diagnostic_sources::WorkspaceDiagnosticSources;
pub mod analysis_cooperation;
mod document_transform;
mod fresh_analysis_cache;
mod package_source;
mod proposal_diagnostics;
mod rewrite_wire;
mod studio_source;
use current_diagnostics::{DiagnosticPass, DiagnosticUnavailable};
mod environment_notice;
/// The exit watchdog that backstops `Server::serve` returning promptly once
/// the session is over. Native only: it wraps `tokio::io::Stdin` and hard
/// exits the process, neither of which apply to a browser worker.
#[cfg(not(target_family = "wasm"))]
pub mod exit_watchdog;
#[cfg(test)]
use tcl_lsp_core::profile_for_dialect;
#[cfg(test)]
mod original_document_metadata_tests;
#[cfg(test)]
mod original_document_path_admission_tests;
#[cfg(test)]
mod original_workspace_diagnostics_tests;
pub mod path_glob;
mod provider_source;
mod readonly_reference_reports;
mod recovery_source;
#[cfg(test)]
use recovery_source::widen_recovery_extra_commands;
use recovery_source::{RecoveryNameCache, RecoveryWidenCtx};
pub mod rt;
pub mod service;
/// The stdout decoupling pump the native binary serves through. Native only:
/// it is `tokio::io::Stdout` end to end, and a browser worker has no stdio.
#[cfg(not(target_family = "wasm"))]
pub mod stdio_pump;
pub mod transport_liveness;
pub mod uri_norm;
pub mod vfs;

/// The truth table's LSP and code-action passes.
#[cfg(test)]
mod policy_truth_table;

use std::collections::{HashMap, HashSet, VecDeque};
use std::future::Future;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::Poll;
// `FutureExt::shared` lets the progressive diagnostics path compute the base
// per-file analysis once and await it from both the deep pass and the fast tier.
#[cfg(test)]
use futures_util::future::FutureExt;
use sha2::{Digest, Sha256};
#[cfg(test)]
use tcl_compiler::compiler_checks::DiagCode;

use tcl_compiler::analyser::{Analyser, AnalysisResult, NonAsciiMode};
use tcl_lsp_core::bigip as core_bigip;
#[cfg(test)]
use tcl_lsp_core::call_hierarchy as core_call_hierarchy;
use tcl_lsp_core::code_actions as core_code_actions;
use tcl_lsp_core::code_lens as core_code_lens;
use tcl_lsp_core::completion as core_completion;
use tcl_lsp_core::declaration as core_declaration;
use tcl_lsp_core::definition::{self as core_definition, LspRange as CoreLspRange};
use tcl_lsp_core::diagnostic_policy as core_policy;
use tcl_lsp_core::diagnostic_report as core_report;
use tcl_lsp_core::document_links as core_document_links;
use tcl_lsp_core::document_symbols as core_symbols;
#[cfg(test)]
use tcl_lsp_core::document_symbols::SymbolKind as CoreSymbolKind;
use tcl_lsp_core::file_ops as core_file_ops;
#[cfg(test)]
use tcl_lsp_core::folding::FoldKind;
use tcl_lsp_core::formatting as core_formatting;
use tcl_lsp_core::hover as core_hover;
#[cfg(test)]
use tcl_lsp_core::hover::Hover as CoreHover;
use tcl_lsp_core::implementation as core_implementation;
use tcl_lsp_core::inlay_hints as core_inlay_hints;
use tcl_lsp_core::linked_editing_range as core_linked_editing_range;
use tcl_lsp_core::namespace_rename as core_namespace_rename;
use tcl_lsp_core::namespace_symbol as core_namespace_symbol;
use tcl_lsp_core::original_indexed_location::OriginalIndexedSourceLocation;
use tcl_lsp_core::package_resolver::PackageResolver;
use tcl_lsp_core::references as core_references;
use tcl_lsp_core::rename as core_rename;
use tcl_lsp_core::rename_safety as core_rename_safety;
use tcl_lsp_core::selection_range as core_selection_range;
#[cfg(test)]
use tcl_lsp_core::semantic_tokens as core_semantic_tokens;
use tcl_lsp_core::signature_help as core_sig;
use tcl_lsp_core::tcl_install as core_tcl_install;
use tcl_lsp_core::type_definition as core_type_definition;
use tcl_lsp_core::type_hierarchy as core_type_hierarchy;
use tcl_lsp_core::workspace_index as core_workspace_index;
#[cfg(test)]
use tcl_lsp_core::workspace_symbols as core_workspace_symbols;
use tcl_registry::CommandRegistry;
use tokio::sync::{Mutex, RwLock};
use tower_lsp_server::jsonrpc;
use tower_lsp_server::ls_types::request::{
    GotoDeclarationParams, GotoDeclarationResponse, GotoImplementationParams,
    GotoImplementationResponse, GotoTypeDefinitionParams, GotoTypeDefinitionResponse,
};
use tower_lsp_server::ls_types::{
    CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    CodeActionOrCommand, CodeActionParams, CodeLens, CodeLensParams, CompletionItem,
    CompletionParams, CompletionResponse, DidChangeConfigurationParams,
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidChangeWorkspaceFoldersParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DocumentChanges,
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    DocumentFormattingParams, DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams,
    DocumentLink, DocumentLinkParams, DocumentRangeFormattingParams, DocumentSymbol,
    DocumentSymbolParams, DocumentSymbolResponse, ExecuteCommandParams, FoldingRange,
    FoldingRangeParams, FullDocumentDiagnosticReport, GotoDefinitionParams, GotoDefinitionResponse,
    Hover, HoverParams, InitializeParams, InitializeResult, InitializedParams, InlayHint,
    InlayHintKind, InlayHintLabel, InlayHintParams, LinkedEditingRangeParams, LinkedEditingRanges,
    Location, MessageType, OneOf, OptionalVersionedTextDocumentIdentifier, Position,
    PrepareRenameResponse, Range, ReferenceParams, RelatedFullDocumentDiagnosticReport,
    RelatedUnchangedDocumentDiagnosticReport, RenameFilesParams, RenameParams, SelectionRange,
    SelectionRangeParams, SemanticTokens as LspSemanticTokens, SemanticTokensDeltaParams,
    SemanticTokensFullDeltaResult, SemanticTokensParams, SemanticTokensRangeParams,
    SemanticTokensRangeResult, SemanticTokensResult, ServerInfo, SignatureHelp,
    SignatureHelpParams, TextDocumentEdit, TextDocumentPositionParams, TextEdit, TypeHierarchyItem,
    TypeHierarchyPrepareParams, TypeHierarchySubtypesParams, TypeHierarchySupertypesParams,
    UnchangedDocumentDiagnosticReport, Uri, WillSaveTextDocumentParams, WorkspaceDiagnosticParams,
    WorkspaceDiagnosticReport, WorkspaceDiagnosticReportResult, WorkspaceDocumentDiagnosticReport,
    WorkspaceEdit, WorkspaceFullDocumentDiagnosticReport, WorkspaceSymbolParams,
    WorkspaceSymbolResponse, WorkspaceUnchangedDocumentDiagnosticReport,
};
#[cfg(test)]
use tower_lsp_server::ls_types::{
    DidChangeWatchedFilesRegistrationOptions, FileChangeType, FileEvent, FileSystemWatcher,
    GlobPattern, SymbolKind, WatchKind,
};
#[cfg(test)]
use tower_lsp_server::ls_types::{
    FoldingRangeKind, PositionEncodingKind, SemanticTokensFullOptions,
};
use tower_lsp_server::{Client, LanguageServer};

/// Document store value: source text + dialect string.
///
/// # Snapshot ownership and revision currency
///
/// This type is both the **mutable owner** of an open document (under
/// `crate::BackendState::documents`' mutex) and the **immutable snapshot** every request
/// handler works from ([`Backend::read_document`] clones one out and drops the
/// lock). Those two roles are what make the sharing rules below load-bearing:
///
/// * The large fields — `text` and the `line_index` built from it — are shared
///   handles, so a clone is two reference-count bumps rather than a copy of the
///   document and its index. With `R` concurrent requests against a `B`-byte,
///   `L`-line document that is `O(B + 4L + R)` live bytes instead of
///   `O(R × (B + 4L))`.
/// * Every mutation is **build-and-swap**, never in place. A snapshot therefore
///   observes the revision it was taken at, for its whole lifetime, and an
///   in-flight request can safely finish against an older revision while the
///   editor keeps typing.
/// * `text` and `line_index` are **one snapshot**, not two fields: they must be
///   installed together, so no reader can ever pair text from revision `N` with
///   an index from revision `N-1` and resolve a position against the wrong
///   offsets. [`apply_content_change_indexed`] is the single place that
///   replaces them, and it replaces both.
///
/// Cloning is otherwise unchanged: `dialect` and `language_id` are short
/// strings, and the rest is `Copy`.
#[derive(Debug, Clone)]
struct DocumentState {
    /// URI-bound admission retained by genuine protocol/disk ingress.
    path_admission: Option<Arc<DocumentPathAdmission>>,
    /// Actual diagnostic workspace-command read dependencies, never runtime reachability.
    orphan_command_dependencies: Option<Arc<OrphanCommandDependencies>>,
    /// The document's text **as analysis sees it**, behind an [`Arc`] so a
    /// snapshot handed to a request handler ([`Backend::read_document`]) is a
    /// reference-count bump rather than a copy of the whole buffer.
    ///
    /// Inside [`crate::BackendState::documents`] this is byte-for-byte what the client
    /// sent (the shadow buffer every incremental `didChange` splices into).
    /// [`Backend::read_document`] hands out a snapshot whose lone `\r`s have
    /// been rewritten to `\n` ([`tcl_lexer::normalise_lone_cr`]) and stashes
    /// the client's bytes in `raw_text`, so every provider that re-lexes this
    /// field sees the same script `tclsh` would (a channel opened with
    /// `-translation auto` turns each `\r` into a command terminator before
    /// the parser runs) and lands on the same lines as the client. The
    /// rewrite is byte-length preserving, so spans and offsets are
    /// interchangeable between the two.
    ///
    /// Mutated by build-and-swap, never in place: [`apply_content_change_indexed`]
    /// splices into an owned `String` and installs a **new** `Arc`, so a
    /// snapshot taken before the edit keeps the exact bytes it was given.
    /// That, and not a lock, is what lets an in-flight request finish against
    /// the revision it started on while the editor keeps typing — and it is
    /// why `text` and `line_index` must only ever be replaced together (see
    /// the type-level invariant on [`DocumentState`]).
    text: Arc<str>,
    /// The client's Unicode text or the selected file's decoded text, when it
    /// differs from `text` — only for an old-Mac / mixed document handed out
    /// by [`Backend::read_document`]. This is not original file bytes.
    /// Read it via [`DocumentState::raw`] for anything that must see the real
    /// terminators: the W118 line-ending lint and the formatter (whose output
    /// EOL and whose replace range are both computed from the real document).
    raw_text: Option<Arc<str>>,
    /// Exact selected saved bytes, retained only by closed-source reads.
    saved_source_bytes: Option<Arc<[u8]>>,
    /// Persisted line-start index for `text`, built with the **LSP** EOL model
    /// ([`tcl_lexer::LineIndex::new_lsp`]) so the server's `(line, character)`
    /// coordinates match the client's (`\n`, `\r\n`, and lone `\r` all break a
    /// line). Kept in lock-step with `text`: every mutation of `text` rebuilds
    /// this alongside it (see [`apply_content_change_indexed`]).
    line_index: tcl_lexer::LineIndex,
    /// Whether `text` contains a bare `\r` — [`tcl_lexer::LineIndex::contains_bare_cr`]
    /// at the last point `line_index` was rebuilt from scratch. `did_change`
    /// uses this to decide whether [`tcl_lexer::LineIndex::apply_edit`] remains
    /// sound for the next incremental edit (see [`apply_content_change_indexed`]).
    has_bare_cr: bool,
    /// Byte-level decoding evidence from the selected closed-file read, or
    /// a backing file that decoded to this exact buffer at `didOpen`. The
    /// report describes the pre-normalisation decoded text returned by `raw`,
    /// not original-byte identity for the analysis view. It is cleared by an
    /// edit: LSP carries text, not source bytes, so an edited buffer must not
    /// inherit a verdict about an older on-disk version.
    decode_report: Option<tcl_lsp_core::source_decode::DecodeReport>,
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
    /// The backing file was deleted on disk while this buffer stayed open —
    /// an out-of-band `mv`/`rm`/branch switch, for which no editor sends a
    /// `didClose`.
    ///
    /// The buffer keeps working (its own diagnostics, hover, symbols all still
    /// resolve from `text`), but it stops contributing to the **cross-document**
    /// workspace index: the path it claims no longer exists, so leaving it
    /// indexed duplicates every proc of the file it was renamed to and sends
    /// go-to-definition to a path the editor cannot open.  Cleared when the
    /// path comes back (`didOpen`, a save, or a watched `CREATED`/`CHANGED`).
    backing_file_deleted: bool,
    /// How far deferred publication has caught up with this exact live
    /// revision. Semantic tokens need the Salsa boundary; cross-document
    /// providers need the indexed boundary.
    publication: DocumentPublication,
    /// Monotonic request to re-resolve `dialect` from the current language id,
    /// text hints, folder settings, and session default. A later edit carries
    /// an unresolved generation forward instead of losing the earlier change.
    dialect_resolution_generation: u64,
    /// Last [`Self::dialect_resolution_generation`] resolved against this
    /// exact document revision. Inequality is the persistent dirty bit; using
    /// a generation prevents two racing configuration changes from clearing
    /// each other's work.
    resolved_dialect_generation: u64,
}

macro_rules! workspace_scan_trace {
    ($($arg:tt)*) => {
        if std::env::var_os("TCL_LSP_TRACE_WORKSPACE_SCAN").is_some() {
            eprintln!("[workspace_scan] {}", format_args!($($arg)*));
        }
    };
}

mod analysis;
mod code_actions;
mod commands;
mod configuration;
mod diagnostics;
mod dialects;
mod document_features;
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

use analysis::{FreshAnalysisDriver, FreshAnalysisSeed, default_analyser_config};
use code_actions::lift_versioned_code_actions;
#[cfg(test)]
use code_actions::{CodeActionReportInputs, action_command_to_lsp, code_action_report};
use commands::sha256_digest_hex;
#[cfg(test)]
use commands::{
    TclInstallationSelectionUnavailable, config_ini_path, non_ascii_mode_str,
    tcl_installation_selection_paths,
};
#[cfg(test)]
use configuration::resolve_ilx_specs;
#[cfg(test)]
use configuration::{
    BigipVersionSetting, CONFIG_RELOAD_DEBOUNCE, FolderGenericPatterns, IlxPluginSpec,
    collapse_inlay_alias, parse_bigip_version, parse_folder_config, parse_non_ascii_mode,
    read_ini_layer,
};
use configuration::{
    ConfigReloadSlot, FeatureToggles, FolderConfig, PolicyLayers, ResourceAnalyserInputs,
    apply_formatting_object, default_optimiser_profile, formatter_config_from,
    longest_folder_match, normalize_config_payload, settings_environment_kind_enabled,
    settings_non_ascii_mode, workspace_trust_in,
};
pub use diagnostics::SourceInheritance;
use diagnostics::{
    AnalyserPathInputs, DeliveryCtx, DiagCurrency, DiagInputs, DiagJob, DiagSlot,
    DiagnosticPublisher, F5ReportInputs, LiftInputs, OrphanCommandDependencies, PackNoticeContext,
    PreparedCompletenessInput, ProducedDiagnosticReport, PullDiagEntry, SalsaAnalysisCtx,
    WorkspaceCommandQueries, compute_base_analysis, compute_base_analysis_with_read_cancellations,
    compute_deep_diagnostic_report, compute_deep_diagnostic_report_with_read_cancellations,
    defined_original_commands_from_analysis, f5_model_document_report, is_apl_source,
    next_pull_diag_result_id, report_analysis_worker_panic,
};
#[cfg(test)]
use diagnostics::{
    CLOSED_DIAG_BADGE_CAP, DIAGNOSTICS_DEBOUNCE, DiagnosticPublicationSources,
    DiagnosticPublishOutcome, PublishTiming, W120Availability, deep_provider_completion,
    original_document_is_complete, prepare_base_completeness_input,
    publish_diagnostics_result_with_sources, published_findings, published_report,
    refine_original_w123_diagnostics, run_diagnostics_core,
};
#[cfg(test)]
use diagnostics::{
    CrossFileCalls, F5ModelDocument, FastTierGate, IndexedDiagnosticChange, IndexedDiagnosticFacts,
    ReplacedSources, apl_presentation_findings, bigip_config_findings,
    command_diagnostic_consumers, compiler_findings, compute_compiler_diags,
    compute_compiler_diags_after_base, compute_inherited_requires, compute_source_inheritance,
    cross_file_arity_diagnostics, defined_original_commands, f5_model_report, is_fast_tier,
    lift_report, model_findings, original_diagnostic_consumers, orphaned_fact_consumers,
    overlay_miss_message, publish_diagnostics_result, refine_w120_diagnostics,
    refine_workspace_index_w123, settle_cross_file_calls, source_diagnostic_consumers,
    source_diagnostic_consumers_with_sources, stable_diagnostics_generation,
    unindexed_open_documents, xc_findings,
};
#[cfg(test)]
use diagnostics::{PackNoticeEntry, PackNoticeSourceOwner};
#[cfg(test)]
use diagnostics::{
    settle_call_against_workspace, sync_evaluator_epoch, workspace_reporting_context,
};
use dialects::{
    DEFAULT_SESSION_DIALECT, change_could_affect_dialect_hint, merged_folder_dialect_overrides,
    session_dialect_from,
};
#[cfg(test)]
use dialects::{IRULES_DIALECT, LanguageDialect, folder_dialect_for, unknown_dialect_error};
#[cfg(test)]
use documents::apply_content_change_indexed;
pub use documents::canonical_file_uri;
#[cfg(test)]
use documents::constructed_file_uri;
#[cfg(test)]
use documents::{
    DOCUMENTS_CONTENTION_SAMPLE_GAP, DocumentReadiness, DocumentsContention, HeldFor,
    TURN_PHASE_GRANTED, WaiterSnapshot, WaiterTelemetry, apply_content_change,
    describe_documents_contention, describe_documents_waiters, edit_introduces_bare_cr,
};
use documents::{
    DocumentPathAdmission, DocumentPublication, DocumentSourceResult, DocumentSourceUnavailable,
    DocumentStore, DocumentsGuard, EditOrder, SavedSourceBytes,
};
#[cfg(not(target_family = "wasm"))]
#[cfg(test)]
use documents::{NUDGE_WAITER_THRESHOLD, NudgeTarget, nudge_reason, unpark_watchdog_step};
#[cfg(not(target_family = "wasm"))]
pub use documents::{UNPARK_NUDGES, spawn_unpark_watchdog};
use method_families::WorkspaceClassAnalysis;
#[cfg(test)]
use method_families::{ConsumerCoverage, ConsumerTarget, workspace_class_analysis_fingerprint};
use navigation::{
    WorkspaceSymbolDiagnostic, dedup_locations, dedup_span_targets, implementation_inventory,
    position_is_command_head, type_definition_locations,
};
#[cfg(test)]
use protocol::rename_file_operation_options;
use protocol::{
    FoldingRangeRefreshRequest, bounded_client_request, build_server_capabilities,
    client_lacks_utf16_support, client_supports_message_action_items,
    client_supports_pull_diagnostics, client_supports_relative_watch_patterns,
    client_supports_show_document, lift_bigip_range, lift_completion_item, lift_document_symbol,
    lift_folding_range, lift_hover, lift_lsp_range, lift_signature_help, lift_span,
    line_col_to_byte_offset, materialise_selection_range, negotiate_position_encoding,
    position_within_line,
};
#[cfg(test)]
use protocol::{bounded_client_request_with_timeout, lift_symbol_kind, semantic_tokens_capability};
#[cfg(test)]
use rename::{
    FileRenameInputSnapshot, FileRenameSourcePublication, FileRenameSources, HeldFileRename,
    annotated_conditional_variable_edit, coalesce_conditional_variable_edits,
    describe_variable_collision,
};
use rename::{OriginalDocumentSnapshot, client_supports_conditional_rename};
use semantic_tokens::{
    ConvergenceGuard, ConvergenceInFlight, EnrichedRefreshAsked, RangeConvergenceInputs,
    RangeSettleOutcome, SemanticTokensCache, coarse_semantic_source_changed, core_semantic_range,
    lift_semantic_token_data, log_range_convergence_settled, range_token_data,
};
#[cfg(test)]
use semantic_tokens::{
    EnrichedDelivery, FullSettleOutcome, SEMANTIC_TOKENS_REFRESH_DEBOUNCE,
    configured_coarse_semantic_tokens, describe_refresh_reasons, refresh_if_coalesced,
    unindexed_semantic_tokens, unindexed_semantic_tokens_for_range,
};
#[cfg(test)]
use semantic_tokens::{SemanticTokensRefreshCtx, SemanticTokensRefreshReason};
#[cfg(test)]
use source_publication::DiskPublicationOutcome;
use source_publication::{
    AdmittedDiskBatch, DiskRemovalPolicy, PublishedSourcePath, PublishedSourcePaths,
};
#[cfg(test)]
use source_publication::{LivePublicationRetry, LivePublicationWait, LiveSourceContents};
#[cfg(test)]
use spec_packs::sync_overlay_epoch;
use spec_packs::{PackConfigurationPublication, PublishedPackSet, ReloadTrigger};
#[cfg(test)]
use spec_packs::{install_pack_overlays, whole_line_range};
#[cfg(test)]
use tracked_locks::{RwLockAccess, SnapshotCancellation, SnapshotCensus};
use tracked_locks::{TrackedMutex, TrackedRwLock};
#[cfg(test)]
use tracked_locks::{TrackedMutexGuard, cancellable_catch};
use workspace::{
    CLASS_FACTORY_SYNC_ROUNDS, CrossFileEvidenceSnapshot, EvidenceHandles,
    LibraryPublicationSource, RehomingDocumentSnapshot, WORKSPACE_SCAN_FILE_CAP,
    WatchedFileChanges, WorkspaceScanReadiness, WorkspaceStartupGuard,
    capture_cross_file_evidence_snapshot, new_workspace_index, partition_watched_file_changes,
};
#[cfg(test)]
use workspace::{
    ClassFactoryRound, DiskScanOutcome, PACKAGE_LOCKFILE_GLOB, PROJECT_CONFIG_GLOB,
    ScanAnalysisInputs, ScannedDocument, WORKSPACE_SCAN_DIR_CAP,
    apply_cross_file_evidence_if_current, apply_workspace_class_factories_if_current,
    build_package_resolver, collect_tcl_files, document_auto_path_dirs,
    files_with_covered_load_targets, is_config_file, is_package_metadata_file, is_skipped_scan_dir,
    is_spec_pack_file, reindex_unopened_factory_consumers, resolve_source_uri,
    sync_cross_file_evidence, sync_workspace_class_factories_round, tcl_source_watch_glob,
};
#[cfg(test)]
use workspace::{
    effective_auto_path, entry_point_uri, is_tcl_source, refresh_cross_file_evidence,
    resolve_source_edge, uri_under_folder,
};
#[cfg(test)]
use workspace::{sync_workspace_class_factories, tcl_source_glob};

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
#[derive(Clone)]
pub struct Backend {
    state: Arc<BackendState>,
    analysis_cooperation: Option<Arc<analysis_cooperation::AnalysisCooperation>>,
}

impl std::ops::Deref for Backend {
    type Target = BackendState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

/// Shared server state; fields retain their original locks and ownership.
/// Backend clones retain this allocation, without cloning any input or cache.
#[doc(hidden)]
pub struct BackendState {
    client: Client,
    /// Actual source-bound transform results awaiting a final editor admission.
    document_transforms: std::sync::Mutex<document_transform::TransformLedger>,
    /// Persistent latest-wins diagnostics delivery, isolated from every
    /// request-critical lock.
    diagnostic_publisher: Arc<DiagnosticPublisher>,
    /// Open documents. `Arc` so the detached diagnostics task can hold it.
    documents: Arc<DocumentStore>,
    /// Per-URI coalescing diagnostics scheduler state.  Each edit marks the
    /// document dirty and, if no worker is running, starts one; the single
    /// worker debounces, then repeatedly analyses the document's **latest**
    /// state until it has published the current version (retrying if a run was
    /// cancelled mid-flight).  This guarantees the final version's diagnostics
    /// are always published even under heavy edit bursts / CPU load.
    diag_slots: Arc<Mutex<HashMap<Uri, DiagSlot>>>,
    /// Bounds how many documents' diagnostics analyses run at once, so a
    /// reload that reschedules every open document queues them instead of
    /// running them all side by side on the blocking pool.
    diagnostics_admission: Arc<diagnostics::AnalysisAdmission>,
    /// Fallback dialect string used when ``did_open`` cannot derive
    /// one from the ``languageId`` and no per-session
    /// ``workspace/didChangeConfiguration`` has been received yet.
    /// Updated by ``did_change_configuration`` so editor reconfigures
    /// take effect for subsequently-opened documents.
    default_dialect: Arc<Mutex<String>>,
    /// Whether [`crate::BackendState::default_dialect`] was set by an explicit
    /// `tclLsp.dialect` — a pulled/pushed configuration value or the
    /// `tcl-lsp.setDialect` command — rather than still being the built-in
    /// fallback.  Reported as `dialect_explicitly_set` by
    /// `tcl-lsp.getEffectiveConfig` so a caller tracing a surprising dialect can
    /// tell "nobody configured one" from "someone configured exactly this".
    default_dialect_explicit: Mutex<bool>,
    /// The configured `tclLsp.dialect` last warned about as naming no
    /// environment, so a value that persists across configuration pulls is
    /// reported once rather than on every pull.
    warned_session_dialect: Mutex<Option<String>>,
    /// A deliberate, temporary session dialect that outranks
    /// [`crate::BackendState::default_dialect`] and is **immune to configuration**.
    ///
    /// The chat commands run a buffer under a fixed dialect (`f5-irules`) and
    /// restore the previous one afterwards.  Doing that by pushing a
    /// `didChangeConfiguration` that rewrote `default_dialect` would let the
    /// very next `workspace/configuration` pull re-apply the user's configured
    /// `tclLsp.dialect` and silently revert the override, giving it a lifetime
    /// of "until anything touches settings".  Held in its own slot, the
    /// override survives every pull and is cleared only by the command that set
    /// it (`tcl-lsp.setSessionDialect` with a `null` argument).
    ///
    /// Same *tier* as `default_dialect` — the last-resort fallback, below an
    /// explicit language id, a BIG-IP basename, and a folder override — so
    /// setting one changes nothing about which documents it can reach; it only
    /// changes how long it lasts.
    session_dialect_override: Arc<Mutex<Option<String>>>,
    /// Deliberate per-document dialect overrides, keyed by document URI
    /// string, installed by `tcl-lsp.setDocumentDialectOverride`.
    ///
    /// The strongest tier in the ladder: a host that names a URI and a dialect
    /// outright has made a choice no inference should second-guess, so this is
    /// consulted ahead of the language id, the BIG-IP basename, and the
    /// in-source `# tcl-dialect:` directive.  Every other override is either
    /// session-wide ([`crate::BackendState::session_dialect_override`]) or folder-wide
    /// ([`crate::BackendState::folder_dialects`]); re-tagging one buffer through either of
    /// those re-tags every other buffer with it, which is precisely what a
    /// per-document override must not do.
    ///
    /// Keyed by URI *string* rather than [`Uri`], matching how
    /// `folder_dialect_for` compares folder URIs.  Survives a document
    /// retirement: the entry belongs to the host session that installed it
    /// (the Spec Studio panel), not to any one `didOpen`/`didClose` cycle, and
    /// is removed only by a `null` argument to the command that set it.
    /// A per-document override is an analyser input even for a closed URI;
    /// mutations join the same epoch/gate as all other analyser inputs.
    document_dialect_overrides: Arc<Mutex<HashMap<String, String>>>,
    /// Coalescing state for `workspace/didChangeConfiguration` — see
    /// [`Backend::coalesced_config_reload`].
    config_reload: Mutex<ConfigReloadSlot>,
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
    folder_dialects: Arc<Mutex<Vec<(Uri, String)>>>,
    /// Per-folder editor configuration overrides (diagnostics, optimiser,
    /// formatting, feature toggles), keyed by folder URI and resolved by
    /// longest prefix at read time.  Populated by the per-folder
    /// `workspace/configuration` pull.  Empty in a single-root workspace, where
    /// every read falls back to the process-global fields below.
    folder_configs: Arc<Mutex<Vec<(Uri, FolderConfig)>>>,
    /// Per-folder salsa `AnalyserConfig` input handles, present only for folders
    /// that override the disabled-diagnostics set or non-ASCII mode.  The
    /// diagnostics path resolves the handle by longest prefix so a folder's
    /// W-code suppression reaches the cached `file_analysis` query, not just the
    /// server-side lift.  Folders without such overrides fall back to
    /// [`crate::BackendState::db_config`].
    folder_db_configs: Arc<Mutex<Vec<(Uri, tcl_lsp_db::AnalyserConfig)>>>,
    /// Retired per-folder `AnalyserConfig` handles, keyed by folder URI.
    ///
    /// The `AnalyserConfig` half of [`crate::BackendState::db_tombstones`], and there for
    /// the same reason: a folder whose override set empties (or which leaves the
    /// workspace) drops its handle, and re-enabling an override would allocate a
    /// second one salsa can never reclaim.  A `.tcl-lsp.ini` watcher re-pulls the
    /// whole layered config on every save, so the churn is per keystroke-save,
    /// not per session.
    folder_db_config_tombstones: Arc<Mutex<HashMap<Uri, tcl_lsp_db::AnalyserConfig>>>,
    /// W108 non-ASCII detection mode (`tclLsp.style.nonAscii`).
    /// [`NonAsciiMode::Default`] until an editor configures it via
    /// `initializationOptions` or `workspace/didChangeConfiguration`.
    /// Threaded into every `Analyser` the diagnostics path builds.
    non_ascii_mode: Mutex<NonAsciiMode>,
    /// Diagnostic codes the user has disabled (`tclLsp.diagnostics.<CODE>
    /// = false`). Threaded into every analyser build so the disabled
    /// codes are filtered, and consulted by the source-style pass.
    disabled_diagnostics: Mutex<HashSet<String>>,
    /// Glob patterns naming files that produce **no** diagnostics at all
    /// (`tclLsp.diagnostics.exclude`), matched against the
    /// workspace-folder-relative path (or the file name alone for `/`-less
    /// patterns).  The process-global fallback; a folder whose merged config
    /// sets the key overrides it via [`FolderConfig::diagnostics_exclude`].
    diagnostics_exclude: Mutex<Vec<String>>,
    /// The session's configuration layers — the global `config.ini`, the
    /// editor's `tclLsp` settings and the primary root's `.tcl-lsp.ini` —
    /// that a document's diagnostic policy is built from when its folder has
    /// no layers of its own ([`FolderConfig::policy_layers`]).
    policy_layers: Mutex<PolicyLayers>,
    /// Cross-document proc / class definition index, maintained
    /// incrementally as documents open / change / close.  Lets
    /// completion enumerate procs from sibling files.
    /// `Arc` so the detached diagnostics task can update it off the event loop.
    workspace_index: Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    /// Bounded readonly source declaration locators for editor wire round trips.
    original_declaration_registry:
        Mutex<tcl_lsp_core::original_declaration::OriginalDeclarationRegistry>,
    /// Tcl package database scanned from the workspace + `TCLLIBPATH`: the
    /// `pkgIndex.tcl` / `tclIndex` index used to resolve a `package require`
    /// to the files it loads (and transitively what *they* require). The
    /// diagnostics worker consults it to refine W120 — e.g. to see that a
    /// `package require myTkPackage` (transitively) pulls in Tk.
    /// Rebuilt by `scan_workspace_folders`.
    package_resolver: Arc<RwLock<PackageResolver>>,
    /// The malformed-document recognition memo shares workspace/index labels
    /// across edits. Its package layer additionally retains the complete caller
    /// input, typed requirements/preferences and each captured provider's source,
    /// configuration or unavailable status. Recognition grants no binding.
    recovery_names: Arc<Mutex<RecoveryNameCache>>,
    /// Held for the duration of every `scan_workspace_folders` call (the
    /// blocking tree walk + analysis that rebuilds `package_resolver`).
    /// `ensure_autoload_indexed` acquires (and immediately releases) this
    /// before consulting `package_resolver`, so a request that lands while a
    /// scan is in flight waits for that scan rather than reading a
    /// possibly-still-empty resolver — otherwise a rename / go-to-definition
    /// fired shortly after startup (or a workspace-folder / config change)
    /// could race the scan and silently find nothing, even though the same
    /// request moments later would have resolved correctly.
    workspace_scan_gate: tokio::sync::Mutex<()>,
    /// Released once the first `scan_workspace_folders` pass has completed —
    /// the "the scan has not started yet" half of the startup race
    /// `workspace_scan_gate` alone cannot cover.  See
    /// [`WorkspaceScanReadiness`].
    workspace_scan_ready: Arc<WorkspaceScanReadiness>,
    /// Library files the autoload tier has merged into the workspace
    /// index on demand, so references / rename keep reaching them.  Cleared
    /// (and their index entries dropped) whenever the package database is
    /// rebuilt, so a `libraryPaths` change cannot leave stale library
    /// definitions behind.
    autoloaded_library_uris: Arc<Mutex<HashSet<String>>>,
    /// Per-document source-site namespace seeds currently merged into the
    /// index (`uri → sorted seeds`; `"::"` = the standalone view).  `source`
    /// evaluates a file in the caller's namespace, so a document sourced from
    /// `namespace eval ::x` is indexed under a `::x`-seeded analysis; this
    /// map records what is applied so [`Backend::refresh_source_rehoming`]
    /// only re-analyses on change, and so declaration-side queries can map a
    /// standalone name to its re-homed twin.
    ///
    /// URI retirement clears these retained namespace-seed records.
    rehomed_source_seeds: Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// Serialises [`Backend::refresh_source_rehoming`] passes.
    ///
    /// A dozen request handlers (references, definition, hover, rename, …)
    /// each call `refresh_source_rehoming` at their own entry, and
    /// `scan_workspace_folders` calls it again after every merge. Without
    /// this gate, concurrent callers each run the full (up to 4-round)
    /// convergence loop independently — on a `source`-heavy workspace, every
    /// one of them re-reads and re-analyses the same re-homed documents and
    /// takes its own turn on `workspace_index`'s write lock, so N concurrent
    /// requests pay N times the real reconciliation cost, serialise against
    /// each other on that lock, and head-of-line-block unrelated requests for
    /// as long as it takes. With the gate, a caller that lands while a pass is
    /// already
    /// running simply waits for it — `refresh_source_rehoming`'s own
    /// early-return ("nothing left to reconcile") then answers instantly for
    /// every waiter but the one that did the real work. Same "wait out the
    /// in-flight pass" idiom as [`Self::workspace_scan_gate`].
    rehoming_gate: Arc<tokio::sync::Mutex<()>>,
    /// Serialises source-view construction, never held by final publication.
    original_source_view_build_gate: Arc<tokio::sync::Mutex<()>>,
    /// Source-only view ownership; URI invalidation retires descendants synchronously.
    original_source_views:
        Arc<std::sync::Mutex<readonly_reference_reports::source_views::OriginalSourceViewTable>>,
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
    /// `tclLsp.workspaceScan.maxFiles` — how many on-disk files the workspace
    /// scan reads and indexes, across every folder (issue #2021).  Session
    /// scoped, not per document: one scan serves the whole session, so there
    /// is no folder to resolve it through.  Defaults to
    /// [`WORKSPACE_SCAN_FILE_CAP`]; a change re-runs the scan, exactly as a
    /// `libraryPaths` change does.
    workspace_scan_max_files: Mutex<usize>,
    /// `tclLsp.packages.preferLatest` — the interpreter's **starting**
    /// `package prefer` mode.
    ///
    /// C Tcl starts at `stable` unless `TCL_PKG_PREFER_LATEST` is set in the
    /// environment (or, on 9.0+, the running Tcl is itself an unstable build).
    /// Neither is a fact about the source tree, so it is carried as a setting
    /// rather than inferred from the server's own environment — which is not
    /// the environment the user's interpreter runs in.
    package_prefer_latest_default: Mutex<bool>,
    /// `tclLsp.packages.provides` — declared "requiring this package also
    /// loads these" edges, as `(package, packages it also loads)` pairs in
    /// declaration order; mirrored onto the salsa `AnalyserConfig`.
    ///
    /// A binary extension whose `Init` calls `Tcl_PkgRequire` — or links Tk
    /// through `Tk_InitStubs` — makes a package available with nothing in any
    /// Tcl source saying so, so the workspace `pkgIndex.tcl` scan that finds a
    /// Tcl wrapper's own requires has nothing to read. Declaring the edge is
    /// the only way to state it.
    package_provides: Mutex<Vec<(String, Vec<String>)>>,
    /// `tclLsp.sourceEncoding` — the encoding the target reads scripts in.
    /// Session-wide: closed files are decoded with it, and every analysis
    /// input carries it, so the two never disagree about a document's text.
    script_encoding: std::sync::RwLock<tcl_registry::value_transfer::ScriptEncoding>,
    /// User-declared extra command names (`tclLsp.extraCommands`) treated as
    /// known by the unknown-command (W123) check; mirrored onto the salsa
    /// `AnalyserConfig`.
    extra_commands: Mutex<Vec<String>>,
    /// Built-in command names whose automatic signature help is suppressed
    /// (`tclLsp.signatureHelp.disabledCommands`). User-defined proc signatures
    /// are deliberately unaffected.
    signature_help_disabled_commands: Mutex<Vec<String>>,
    /// `tclLsp.specPacks` — extra `.tclspec` files or directories to load as
    /// `SpecTcl` packs, on top of the ones discovery finds by convention.
    spec_pack_paths: Mutex<Vec<String>>,
    /// The editor's Workspace Trust state for the open folders — what decides
    /// whether a workspace pack's hook bodies run
    /// (`docs/design/registry/spec-packs.md` § *Workspace trust*).
    ///
    /// Only the client says it: `initializationOptions.workspaceTrust` at
    /// start, and a top-level `workspaceTrust` in a `didChangeConfiguration`
    /// push when the editor grants trust ([`workspace_trust_in`]). Never a
    /// pulled or synchronised `tclLsp` setting, which a workspace's own
    /// settings file can write. A client that says nothing is trusted.
    workspace_trust: Mutex<tcl_dialect::model::WorkspaceTrust>,
    /// The workspace's loaded `SpecTcl` packs (`docs/design/registry/spec-packs.md`).
    ///
    /// Workspace scope, deliberately: the set is loaded at `initialized` and
    /// reloaded when a pack file changes, **never** per document and never per
    /// edit of the code that uses it.  Held behind an `Arc` inside the mutex so
    /// a reader clones the handle and releases the lock immediately — a pack
    /// set is read on every `registry_for_dialect` call, which is every
    /// request.
    ///
    /// Carries the generation that published it ([`PublishedPackSet`]) so a
    /// reload can tell whether its own snapshot is newer than what is already
    /// out there.
    spec_packs: Arc<Mutex<PublishedPackSet>>,
    /// Monotonic reload generation, handed out at the top of every
    /// [`Backend::reload_spec_packs`] — *before* discovery, so the number
    /// orders reloads by **when they read the disk**, not by when they
    /// finished.
    ///
    /// A reload publishes only if its number is greater than the one stamped
    /// on the current set, so a snapshot taken before a write can never
    /// overwrite one taken after it, however long its load took.
    ///
    /// This is **armour, not the mechanism**. With `spec_pack_reload` held
    /// across discover → load → publish, reloads cannot overlap at all and this
    /// comparison can never fail; it exists so the ordering invariant — newest
    /// snapshot wins — survives on its own if that mutex is ever narrowed or
    /// removed by someone who reads the serialisation as mere throughput
    /// insurance. `fetch_add` on a single atomic is totally ordered even under
    /// `Relaxed`, so the numbers stay unique and monotonic without the mutex;
    /// the compare-and-publish itself is atomic because it happens under the
    /// `spec_packs` lock.
    spec_pack_reload_seq: std::sync::atomic::AtomicU64,
    /// Held across a whole pack reload — discovery, load, and publish.
    ///
    /// Two reloads can be in flight at once: `initialized` starts one, and a
    /// `.tclspec` appearing (or `tclLsp.specPacks` moving) starts another.
    /// Each takes its own filesystem snapshot and then publishes if the key
    /// differs from what is current, which is last-*writer*-wins with no
    /// regard for which snapshot is newer. The startup reload parses the
    /// bundled EDA loadables and is by far the slower of the two, so on a
    /// loaded machine it finishes second and republishes its pre-write view,
    /// silently discarding the workspace pack that arrived in between. Nothing
    /// re-triggers afterwards — the file is already on disk and its one event
    /// has been consumed — so the pack stays missing until the next edit.
    ///
    /// Serialising makes every reload observe the disk *after* the previous
    /// one published, so whichever runs last re-reads the true state. It costs
    /// nothing on the common path: reloads are workspace-scoped, not per-edit.
    spec_pack_reload: Arc<Mutex<Option<PackConfigurationPublication>>>,
    /// Active Studio publications belong to this Backend, never to a directory.
    studio_sources: Mutex<studio_source::StudioSources>,
    /// Absolute watch globs currently registered for pack directories that lie
    /// **outside every workspace folder** — the per-user tier, and any
    /// absolute `tclLsp.specPacks` entry.
    ///
    /// The main watcher registration is a workspace-relative pattern, which a
    /// client only ever matches inside the workspace folders, so without these
    /// an edit to a user-tier pack reaches nothing and the specs stay stale
    /// until an unrelated config reload or a restart. Remembered here so a
    /// reload re-registers only when the set actually moves, since each change
    /// costs an unregister/register round trip to the client.
    external_pack_watch_globs: Arc<Mutex<Vec<String>>>,
    /// The extensions loaded packs claim that the static
    /// [`TCL_SOURCE_EXTENSIONS`] set does not, as of the last reload.
    ///
    /// Remembered for the same reason as the field above: the watcher and
    /// rename-filter registrations built from it cost an unregister/register
    /// round trip, so they are refreshed only when the set actually moves.
    /// Also the trigger for re-scanning the workspace — a pack that has just
    /// started claiming `.irulex` needs the `.irulex` files already on disk
    /// indexed, which only a fresh scan does.
    pack_source_extensions: Arc<Mutex<Vec<String>>>,
    /// `tclLsp.bigipVersion` — the session's target BIG-IP release for the
    /// keyed library-version axis (`None` = the oldest-supported default).
    bigip_version: Mutex<Option<String>>,
    /// `tclLsp.targets` — declared version-target ranges as `(provider, range)`
    /// pairs; empty = range mode off. Mirrored onto the salsa `AnalyserConfig`.
    declared_targets: Mutex<Vec<(String, String)>>,
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
    ///
    /// That command is the batch form of the optimiser code actions, so it
    /// applies the whole publish-path policy and not just this switch: the
    /// profile and per-code disabled set, and the inline suppression
    /// directives, exactly as the published report does (#2119).
    optimiser_enabled: Mutex<bool>,
    /// Monotonic stamp for "the configuration the diagnostics scheduler caches
    /// has moved".  See [`Backend::invalidate_diag_inputs`].
    diag_inputs_epoch: Arc<std::sync::atomic::AtomicU64>,
    /// Fresh per-document analyses keyed on their complete input revision
    /// (Server108). A bounded LRU, validated against the full key on every
    /// hit; see [`fresh_analysis_cache`].
    fresh_analysis_cache: std::sync::Mutex<fresh_analysis_cache::FreshAnalysisCache>,
    /// Serialises analyser-input mutations with the final standalone-index
    /// epoch check and replacement. Writers hold this across the mutation and
    /// epoch bump; seed publishers hold a reader through their no-await commit.
    analyser_inputs_gate: Arc<tokio::sync::RwLock<()>>,
    /// The `now_serving` position the last document-sync barrier stall was
    /// reported at, so a wedge logs once rather than once per blocked request.
    /// See [`Backend::report_edit_barrier_stall`].
    edit_barrier_stall_reported: std::sync::atomic::AtomicU64,
    /// Optimisation profile (`tclLsp.optimiser.profile`) controlling which
    /// O-code categories surface as diagnostics. Default
    /// [`tcl_compiler::optimiser::profiles::DEFAULT_EDITOR_PROFILE`]
    /// (`readability`).
    optimiser_profile: Mutex<tcl_compiler::optimiser::profiles::OptimisationProfile>,
    /// Resolved formatter line length (`tclLsp.formatting.lineLength`).
    /// Surfaced by `getEffectiveConfig`; default 80.
    line_length: Mutex<u32>,
    /// Resolved source-style line length (`tclLsp.style.lineLength`) — the W111
    /// "line too long" threshold, distinct from the formatter width above.
    /// Default 120, matching
    /// [`tcl_lsp_core::source_style::DEFAULT_LINE_LENGTH`].
    style_line_length: Mutex<u32>,
    /// Incremental query database (salsa 0.28) — the single memoised store of
    /// derived facts that is replacing the hand-maintained caches above.  The
    /// `db` handle is the write side (set inputs); reads clone it onto a
    /// worker thread and catch `salsa::Cancelled` when a newer edit supersedes
    /// the request.
    db: Arc<TrackedMutex<tcl_lsp_db::TclDatabase>>,
    /// Per-URI salsa `SourceFile` input handles — the input-of-record the
    /// query graph reads.  Kept current by `did_open` / `did_change`.
    db_files: Arc<TrackedMutex<HashMap<Uri, tcl_lsp_db::SourceFile>>>,
    /// Original provider receipt paired with the actual published `SourceFile`.
    source_path_admissions: Arc<PublishedSourcePaths>,
    /// Retired `SourceFile` handles, keyed by the URI they belonged to.
    ///
    /// Salsa never reclaims an input: `SourceFile::new` only ever allocates, so
    /// a URI that is removed and later re-created would strand its old input —
    /// full text and every memo keyed on it — for the process's life.  A watched
    /// `DELETED`→`CREATED` pair (a `git checkout` retriggers one per file), an
    /// untitled buffer closing, a workspace folder being removed and re-added,
    /// and a rename's [`Backend::retire_renamed_uri`] all take that path.
    ///
    /// So a removal *retires* the handle here with its payload cleared instead
    /// of dropping it, and [`Backend::db_set_source`] revives it rather than
    /// allocating a second one.  The cost is one input per distinct URI the
    /// session ever saw, each holding near-nothing while retired.  Retired
    /// handles are never in `db_files`, so they never reach the salsa
    /// [`tcl_lsp_db::Project`] file set either.
    db_tombstones: Arc<Mutex<HashMap<Uri, tcl_lsp_db::SourceFile>>>,
    /// URIs admitted to the Salsa project. An edited open orphan keeps its
    /// local `db_files` handle but is deliberately absent here, so its dead
    /// path cannot contribute cross-document facts.
    db_project_members: Arc<Mutex<HashSet<Uri>>>,
    /// The salsa `Project` input — the cross-document workspace file set,
    /// re-set only when `db_project_members` changes.
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
    /// Monotonic per-URI generation for **closed**-file diagnostics runs,
    /// so overlapping close / watched-change refreshes cannot let an older run
    /// publish stale diagnostics over a newer one — see [`DiagInputs::closed_diag_gen`].
    closed_diag_gen: Arc<Mutex<HashMap<Uri, u64>>>,
    /// Publish order of the **closed** files that currently hold a badge record,
    /// oldest first — the recency list backing [`CLOSED_DIAG_BADGE_CAP`].
    /// Only [`Backend::record_closed_diag_badge`] touches it, so an open
    /// document's cache entry is never queued and never evicted.
    closed_diag_order: Arc<Mutex<VecDeque<Uri>>>,
    /// Whether the client advertised pull-diagnostic support
    /// (`textDocument.diagnostic` client capability) at `initialize`.
    ///
    /// When `true` the worker stops *pushing* diagnostics via
    /// `publish_diagnostics` and instead keeps the pull cache current and asks
    /// the client to re-pull (`workspace/diagnostic/refresh`).  A client that
    /// supports both — `vscode-languageclient` does — otherwise routes the
    /// server's push **and** its own pull into two separate diagnostic
    /// collections and renders every diagnostic twice.  Editors that
    /// only understand push (no pull capability) keep receiving the push.
    client_supports_pull_diagnostics: std::sync::atomic::AtomicBool,
    /// Snapshot of [`client_supports_relative_watch_patterns`], read by
    /// [`Backend::refresh_external_pack_watchers`] long after `initialize`.
    client_supports_relative_watch_patterns: std::sync::atomic::AtomicBool,
    conditional_variable_rename_supported: std::sync::atomic::AtomicBool,
    /// The one-time explanation shown when a document resolves to a tool
    /// environment (Vivado, Quartus, …), with the state that keeps it to once
    /// per environment per session and never again after a dismissal. Shared
    /// with the task that presents it, so no lock of the backend's is held
    /// while the client is awaited. See [`environment_notice`].
    environment_notice: Arc<environment_notice::EnvironmentNotice>,
    /// Per-URI cache of the last semantic-token stream we served — its
    /// `resultId` and the packed integer data.  Lets
    /// `textDocument/semanticTokens/full/delta` answer with a minimal
    /// token-aligned [`core_semantic_tokens::diff`] edit instead of resending
    /// the whole stream, the incremental behaviour rust-analyzer / clangd use.
    /// Every editor that speaks `full/delta` (`VS Code`, Zed, Neovim, eglot, …)
    /// benefits: a keystroke transmits a few changed tokens rather than the
    /// entire document, which keeps the client's token round-trip — and, for
    /// eglot, its stale-repaint window — small on large files.
    /// Keyed by URI; the entry is refreshed on every `full` / `full/delta`
    /// response and evicted on `did_close`.  `Arc` so the detached
    /// semantic-tokens background continuation (see
    /// [`Backend::semantic_tokens_core_data`]) can compare the enriched result
    /// it lands against what was last served, without borrowing `Backend`.
    last_semantic_tokens: SemanticTokensCache,
    /// Per-URI hash of the enriched token stream a convergence continuation has
    /// already asked the client to re-pull — the termination bound on the
    /// converge → refresh → re-request cycle.  See [`EnrichedRefreshAsked`].
    semantic_tokens_refresh_asked: EnrichedRefreshAsked,
    /// Per-URI memo of [`Backend::analyse_with_workspace_classes`] — the
    /// analysis that resolves a cross-file constructor (`set d [::other::Cls
    /// new]`) by feeding the workspace class set to instance inference.
    ///
    /// Unlike the ordinary document analysis this one is not a salsa query:
    /// it depends on the workspace class set as well as the document, which
    /// is not a salsa input.  Without a memo the consumer scan re-ran it once
    /// per candidate document per request, so one code-lens resolve would cost
    /// (documents × analysis), paid again for every lens the editor has on
    /// screen.  Entries validate against a fingerprint of both
    /// inputs rather than a revision counter, so a stale entry is impossible
    /// however the document or the index changed; the URI key just keeps the
    /// map small and evictable on `did_close`.
    workspace_class_analyses: Arc<Mutex<HashMap<Uri, WorkspaceClassAnalysis>>>,
    /// Oracle currency shared with every project-wide class-factory publisher.
    /// A standalone analysis carries this generation and holds a read guard
    /// through its index commit, so an older oracle can never overwrite facts
    /// published after the analysis started.
    class_factory_generation: Arc<tokio::sync::RwLock<u64>>,
    /// Set while a debounced `workspace/semanticTokens/refresh` fire is
    /// scheduled (see [`SemanticTokensRefreshCtx::request_refresh_coalesced`]).
    /// The refresh carries no data — a client that receives it simply
    /// re-pulls current tokens for every document it has open — so any
    /// number of enriched results landing while a fire is already scheduled
    /// ride along with it; there is nothing to lose by not scheduling a
    /// second one. Without this, many cold large tabs finishing their
    /// enriched computation around the same time (e.g. on startup) would
    /// each fire their own workspace-wide refresh, and a client that does
    /// not coalesce them itself (`VS Code` does; eglot may not) would re-pull
    /// every open document once per refresh.
    semantic_tokens_refresh_pending: Arc<std::sync::atomic::AtomicU8>,
    /// URIs with a detached semantic-token convergence continuation in flight.
    /// `semantic_tokens_refresh_pending` above coalesces the resulting
    /// *notification*; this bounds the **work**, which is what holds a document
    /// copy and a queued blocking job.  Shared by the `range` and `full` timeout
    /// paths, since both settle into the same workspace-scoped refresh — see
    /// [`ConvergenceGuard`].
    semantic_tokens_convergence: ConvergenceInFlight,
    /// Abort handle for the most recent [`Backend::spawn_workspace_warm`] task, so
    /// a fresh warm (from `initialize` / folder-add / config-change) supersedes any
    /// still-running one. Without it, overlapping warms each hold their own
    /// `WORKSPACE_WARM_MAX_CONCURRENCY` snapshots, so the global snapshot bound
    /// would hold only per-warm. A `std::sync::Mutex`: locked only briefly (swap +
    /// abort) from the sync `spawn_workspace_warm`, never across an await.
    warm_task: std::sync::Mutex<Option<crate::rt::AbortHandle>>,
    /// Sequences the document-sync notification handlers (`did_open` /
    /// `did_change` / `did_close`) so their buffer mutations apply in the exact
    /// order the client sent them. See [`EditOrder`].
    edit_order: EditOrder,
    /// Serialises watched-file batches. Notification handlers run
    /// concurrently, but an older delete must not retire a path after a later
    /// create has already revived and republished its open buffer.
    watched_file_change_gate: Arc<Mutex<()>>,
    /// Serialises disk-backed commits while their rehoming/live gates may be
    /// released around contention. A later disk read must not overtake and
    /// publish before an older captured replacement resumes and overwrites it;
    /// this gate is disk-only and never delays editor-buffer publication.
    disk_publication_gate: Mutex<()>,
    /// Wakes requests waiting for either tier of the matching open document's
    /// deferred publication. `DocumentState::publication` is the condition;
    /// this notify supplies only the race-free wakeup edge.
    live_publication_advanced: Arc<tokio::sync::Notify>,
    /// Cancels a deferred publisher's periodic wait in a contended lock queue
    /// when any authoritative live-document identity changes. The generation
    /// distinguishes a publisher's own preceding invalidation from a newer
    /// one; the notify supplies the wakeup edge only.
    live_publication_invalidated: Arc<tokio::sync::Notify>,
    live_publication_generation: Arc<std::sync::atomic::AtomicU64>,
    /// Per-URI invalidation generations let a publisher distinguish its own
    /// obsolescence from an unrelated global wake without acquiring the
    /// contended document map it may currently be queued behind.
    live_publication_uri_generations: std::sync::Mutex<HashMap<Uri, u64>>,
    /// Serialises the deferred source/index commits after document-sync turns
    /// have been released. This preserves edit order without making the gate a
    /// request barrier or retaining the document map while Salsa cancels old
    /// snapshots.
    live_publication_gate: Arc<Mutex<()>>,
    /// Selected source provider for document URI paths and closed-file I/O — see
    /// [`crate::vfs`]. Natively this is [`vfs::NativeStore`], a direct
    /// delegation to `std::fs`; a browser worker supplies a
    /// [`vfs::MemoryStore`] the host fills in over `postMessage`. `Arc` so a
    /// blocking task can carry it.
    store: Arc<dyn vfs::SourceStore>,
}

impl std::fmt::Debug for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backend").finish_non_exhaustive()
    }
}

struct BackendInitialState {
    db: tcl_lsp_db::TclDatabase,
    db_config: tcl_lsp_db::AnalyserConfig,
    diagnostic_publisher: Arc<DiagnosticPublisher>,
    environment_notice: Arc<environment_notice::EnvironmentNotice>,
}

impl BackendInitialState {
    fn new(client: &Client, store: &dyn vfs::SourceStore) -> Self {
        let db = tcl_lsp_db::TclDatabase::default();
        let db_config = default_analyser_config(&db);
        let diagnostic_publisher = Arc::new(DiagnosticPublisher::new(client.clone()));
        // What the user asked not to be told again is read once, here.
        let environment_notice = Arc::new(environment_notice::EnvironmentNotice::load(
            store,
            core_tcl_install::user_notices_dir(),
        ));
        Self {
            db,
            db_config,
            diagnostic_publisher,
            environment_notice,
        }
    }
}

impl Backend {
    fn from_state(state: BackendState) -> Self {
        Self {
            state: Arc::new(state),
            analysis_cooperation: None,
        }
    }

    /// Attach the actual transport scheduling owner before sharing this backend.
    #[must_use]
    pub fn with_analysis_cooperation(
        mut self,
        cooperation: Arc<analysis_cooperation::AnalysisCooperation>,
    ) -> Self {
        self.analysis_cooperation = Some(cooperation);
        self
    }

    #[cfg(test)]
    fn state_mut_for_test(&mut self) -> &mut BackendState {
        Arc::get_mut(&mut self.state).expect("fixture fields are set before Backend is cloned")
    }

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
        Self::with_store(client, Arc::new(vfs::NativeStore))
    }

    /// Build a fresh backend that reads every closed file through `store`.
    ///
    /// [`Backend::new`] is this with [`vfs::NativeStore`], which is the whole
    /// of the native server. A host with no filesystem — a browser worker —
    /// passes its own [`vfs::SourceStore`] here instead; see [`crate::vfs`]
    /// for what that does and does not cover.
    #[must_use]
    pub fn with_store(client: Client, store: Arc<dyn vfs::SourceStore>) -> Self {
        let initial = BackendInitialState::new(&client, store.as_ref());
        Self::from_state(BackendState {
            client,
            diagnostic_publisher: initial.diagnostic_publisher,
            document_transforms: std::sync::Mutex::new(
                document_transform::TransformLedger::default(),
            ),
            documents: Arc::new(DocumentStore::default()),
            diag_slots: Arc::new(Mutex::new(HashMap::new())),
            diagnostics_admission: diagnostics::AnalysisAdmission::shared(),
            default_dialect: Arc::new(Mutex::new(DEFAULT_SESSION_DIALECT.to_owned())),
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
            discovered_tcl: Arc::new(std::sync::OnceLock::new()),
            editor_library_paths: Mutex::new(Vec::new()),
            workspace_scan_max_files: Mutex::new(WORKSPACE_SCAN_FILE_CAP),
            package_prefer_latest_default: Mutex::new(false),
            package_provides: Mutex::new(Vec::new()),
            script_encoding: std::sync::RwLock::default(),
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
            db: Arc::new(TrackedMutex::new("db", initial.db)),
            db_files: Arc::new(TrackedMutex::new("db_files", HashMap::new())),
            source_path_admissions: Arc::new(std::sync::Mutex::new(HashMap::new())),
            db_tombstones: Arc::new(Mutex::new(HashMap::new())),
            db_project_members: Arc::new(Mutex::new(HashSet::new())),
            db_project: Arc::new(Mutex::new(None)),
            db_config: Arc::new(Mutex::new(initial.db_config)),
            pull_diag_cache: Arc::new(Mutex::new(HashMap::new())),
            closed_diag_gen: Arc::new(Mutex::new(HashMap::new())),
            closed_diag_order: Arc::new(Mutex::new(VecDeque::new())),
            client_supports_pull_diagnostics: std::sync::atomic::AtomicBool::new(false),
            client_supports_relative_watch_patterns: std::sync::atomic::AtomicBool::new(false),
            conditional_variable_rename_supported: std::sync::atomic::AtomicBool::new(false),
            environment_notice: initial.environment_notice,
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
            store,
        })
    }
}

/// The Tcl-family source extensions the analyser can usefully index — the
/// **single** list behind [`is_tcl_source`] (the workspace scan's on-disk
/// filter), the `workspace/didChangeWatchedFiles` registration, and the
/// `willRename` / `didRename` file-operation filter.
///
/// One list because the three must agree: a file the scan indexes but the
/// watcher ignores goes stale the moment it changes outside the editor, and a
/// file the scan indexes but the rename filter ignores keeps its old `source`
/// references after a rename.  They did not agree — the watcher and rename
/// filter must list every one of them.
///
/// `test` is the standard `tcltest` test-file extension (tcllib's own suite,
/// and every mined corpus, use it throughout — e.g. `test/argparse.test`).
/// Omitting it makes a proc's call sites inside an un-opened `.test` file
/// invisible to the background workspace scan, so cross-document
/// find-references / rename-safety silently miss them — even though opening
/// the file directly works fine, since that path doesn't go through this
/// filter at all.
#[cfg(test)]
use tcl_registry::dialects::TCL_SOURCE_EXTENSIONS;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod retained_class_family_receiver_tests;

#[cfg(test)]
mod closed_document_source_context_tests;

#[cfg(test)]
mod pack_configuration_publication_tests;

#[cfg(test)]
mod pack_configuration_input_gate_tests;

#[cfg(test)]
mod native_class_type_source_revision_tests;

#[cfg(test)]
mod native_cross_document_snapshot_tests;

#[cfg(test)]
mod orphan_dependency_currency_tests;

#[cfg(test)]
mod original_method_final_source_batch_tests;

#[cfg(test)]
mod backend_shared_ownership;

#[cfg(test)]
mod closed_scan_input_custody;

#[cfg(test)]
mod file_rename_publication_custody;

#[cfg(test)]
mod conditional_rename_annotation_tests;

#[cfg(test)]
mod cooperative_fresh_analysis_tests;

#[cfg(test)]
mod virtual_workspace_uri_tests_1734;
