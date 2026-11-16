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

//! The diagnostics pipeline: scheduling, the analysis passes, workspace refinement, the report and its publication.

use super::*;

mod analysis_passes;
mod closed_files;
mod command_refinement;
mod consumers;
mod deep_pass;
mod delivery;
mod f5_models;
mod inputs;
mod package_refinement;
mod publication;
mod publisher;
mod pull;
mod report;
mod requests;
mod run;
mod scheduling;

#[cfg(test)]
pub(super) use analysis_passes::overlay_miss_message;
pub(super) use analysis_passes::{
    PreparedCompletenessInput, compute_base_analysis,
    compute_base_analysis_with_read_cancellations, report_analysis_worker_panic,
    sync_evaluator_epoch,
};
#[cfg(test)]
pub(super) use analysis_passes::{compute_compiler_diags, compute_compiler_diags_after_base};
#[cfg(test)]
pub(super) use analysis_passes::{original_document_is_complete, prepare_base_completeness_input};
#[cfg(test)]
pub(super) use closed_files::CLOSED_DIAG_BADGE_CAP;
#[cfg(test)]
pub(super) use command_refinement::defined_original_commands;
#[cfg(test)]
pub(super) use command_refinement::refine_original_w123_diagnostics;
pub(super) use command_refinement::{
    ProcArityUnion, defined_original_commands_from_analysis, settle_call_against_workspace,
    workspace_reporting_context,
};
#[cfg(test)]
pub(super) use command_refinement::{
    cross_file_arity_diagnostics, refine_workspace_index_w123, settle_cross_file_calls,
};
#[cfg(test)]
pub(super) use consumers::{
    IndexedDiagnosticFacts, ReplacedSources, command_diagnostic_consumers,
    original_diagnostic_consumers, orphaned_fact_consumers, source_diagnostic_consumers,
    source_diagnostic_consumers_with_sources, unindexed_open_documents,
};
pub(super) use consumers::{OrphanCommandDependencies, WorkspaceCommandQueries};
#[cfg(test)]
pub(super) use deep_pass::deep_provider_completion;
pub(super) use deep_pass::{
    ProducedDiagnosticReport, compute_deep_diagnostic_report,
    compute_deep_diagnostic_report_with_read_cancellations,
};
pub(super) use delivery::DeliveryCtx;
pub(super) use f5_models::{F5ReportInputs, f5_model_document_report, is_apl_source};
#[cfg(test)]
pub(super) use f5_models::{
    apl_presentation_findings, bigip_config_findings, f5_model_report, model_findings,
};
#[cfg(test)]
pub(super) use package_refinement::W120Availability;
#[cfg(test)]
pub(super) use package_refinement::refine_w120_diagnostics;
#[cfg(test)]
pub(super) use package_refinement::{compute_inherited_requires, compute_source_inheritance};
pub(super) use publication::DiagnosticProviderResult;
#[cfg(test)]
pub(super) use publication::publish_diagnostics_result;
#[cfg(test)]
pub(super) use publication::publish_diagnostics_result_with_sources;
pub(super) use publisher::{
    DiagnosticPublishOutcome, DiagnosticPublisher, PackNoticeContext, PackNoticeEntry,
    PackNoticeSourceOwner, PreparedPackNotice,
};
pub(super) use pull::next_pull_diag_result_id;
#[cfg(test)]
pub(super) use report::lift_report;
pub(super) use report::{
    analyser_findings, document_policy, lifted_report, published_findings, published_report,
};
#[cfg(test)]
pub(super) use report::{compiler_findings, xc_findings};
#[cfg(test)]
pub(super) use run::run_diagnostics_core;
#[cfg(test)]
pub(super) use run::{FastTierGate, is_fast_tier};
#[cfg(test)]
pub(super) use scheduling::DIAGNOSTICS_DEBOUNCE;
#[cfg(test)]
pub(super) use scheduling::stable_diagnostics_generation;
pub(super) use scheduling::{refresh_can_change_result, reschedule_peers};

/// What "still the current state" means for a diagnostics run at publish time,
/// and therefore what the currency guard re-checks under the `documents` lock
/// before delivering.
///
/// A run against an **open** buffer is current only while the document is still
/// open at the revision it was captured for.  A run against a **closed** but
/// on-disk workspace file (so its Problems / File-Explorer badge survives
/// the editor tab closing) is current only while the document is still closed:
/// a concurrent `did_open` makes the open buffer authoritative, so a late
/// closed-file publish must not land on top of it.
#[derive(Clone, Copy)]
pub(super) enum DiagCurrency {
    /// Open buffer at this revision.
    Open(u64),
    /// A closed workspace file analysed from its on-disk contents at this
    /// per-URI generation. The generation lets the publish-time guard
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
pub(super) struct DiagJob {
    pub(super) diagnostic_pass: Option<Arc<DiagnosticPass>>,
    /// The document snapshot's shared text handle — a reference-count bump, not
    /// a copy of the buffer, so scheduling a diagnostics run for a large file
    /// costs nothing in memory.
    pub(super) text: Arc<str>,
    /// Decoding evidence for precisely this text, when it came from matching
    /// on-disk bytes. See [`DocumentState::decode_report`].
    pub(super) decode_report: Option<tcl_lsp_core::source_decode::DecodeReport>,
    pub(super) dialect: String,
    /// The document's editor `language_id`, carried so the diagnostics worker
    /// can dispatch F5 dialect documents (iApp APL presentations are detected
    /// by language id / basename, not by the resolved dialect alone — see
    /// [`is_apl_source`]).
    pub(super) language_id: String,
    /// Whether this run targets the open buffer or a closed on-disk file,
    /// deciding what the publish-time currency guard re-checks.
    pub(super) currency: DiagCurrency,
    pub(super) version: Option<i32>,
    pub(super) file: Option<tcl_lsp_db::SourceFile>,
    pub(super) config: tcl_lsp_db::AnalyserConfig,
}

/// One document's last-published diagnostics plus the `result_id` that
/// identifies that exact set, for the pull-diagnostic path
/// (`textDocument/diagnostic` + `workspace/diagnostic`).  Kept in sync with
/// the push pipeline so a pull request returns the same diagnostics the editor
/// last received via `publish_diagnostics`, and an unchanged set can be
/// answered with a cheap `Unchanged` report.
#[derive(Clone)]
pub(super) struct PullDiagEntry {
    pub(super) currency: Option<DiagnosticCacheReceipt>,
    pub(super) result_id: String,
    /// The document revision the cached diagnostics were computed for.  The
    /// pull handler compares it against the live document so a cache entry from
    /// an older edit is recomputed rather than served as current.
    pub(super) revision: u64,
    pub(super) diagnostics: Vec<tower_lsp_server::ls_types::Diagnostic>,
}

/// Per-URI coalescing-scheduler slot (see [`crate::BackendState::diag_slots`]).
#[derive(Default)]
pub(super) struct DiagSlot {
    /// An edit arrived that the running worker has not yet analysed.  Just a
    /// flag: the worker reads the document's *current* state at drain time, so
    /// it is robust to out-of-order edit processing (a late, lower-version edit
    /// can't clobber a captured job — there is no captured job).
    pub(super) dirty: bool,
    /// Advances whenever new work is scheduled.  The worker samples this
    /// before its debounce sleep and only starts analysis when the generation
    /// remains unchanged for the whole window, making the debounce trailing-
    /// edge rather than a fixed delay from the burst's first edit.
    pub(super) generation: u64,
    /// A worker task is currently draining this document.
    pub(super) running: bool,
    /// The freshest analyser inputs (registry + per-URI config toggles).  The
    /// worker reads this at drain time rather than a snapshot captured when it
    /// spawned, so a config change (optimiser / diagnostics toggle) that arrives
    /// while the worker is mid-flight is analysed under the *new* toggles, not
    /// the stale ones — otherwise a disabled optimiser would keep emitting
    /// O-codes until the next document edit.
    ///
    /// Resolving these is the expensive half of scheduling, so an ordinary edit
    /// reuses what is here rather than re-reading a dozen config mutexes per
    /// keystroke.  [`Self::inputs_epoch`] is what keeps that reuse honest.
    pub(super) latest_inputs: Option<DiagInputs>,
    /// The [`Backend::diag_inputs_epoch`] value `latest_inputs` was resolved at.
    ///
    /// The cache above may only be reused while the configuration it was read
    /// from is still current.  Without this stamp the edit path (which does not
    /// force a refresh) would silently analyse under whatever config was in
    /// force at the *previous* schedule — see
    /// [`Backend::invalidate_diag_inputs`] for the window that makes that
    /// observable.
    ///
    /// Two invariants make the stamp trustworthy, both enforced in
    /// [`Backend::schedule_diagnostics_impl`]: the epoch was the same before and
    /// after the resolve that produced `latest_inputs` (so the snapshot is not
    /// half from each side of an apply), and the pair only ever moves *forwards*
    /// (so a slow resolve landing after a faster one cannot stamp the slot back
    /// into the past). Without the second, the reuse check above would go on to
    /// call the older snapshot current.
    pub(super) inputs_epoch: u64,
}

impl DiagSlot {
    pub(super) fn mark_dirty(&mut self) {
        self.dirty = true;
        self.generation = self.generation.wrapping_add(1);
    }
}

/// The two independent, both-default-off cross-file-adjacent toggles,
/// grouped out of [`DiagToggles`] to stay under its flat-bool-count lint:
/// `xcDiagnostics` (the opt-in XC100-301 translatability diagnostics —
/// only takes effect on `f5-irules` documents) and `crossFileResolution`
/// (opt-in cross-file W120/W123 suppression + cross-file E002/E003
/// arity — every dialect). Deliberately independent of each other; see
/// `Backend::xc_diagnostics_enabled` / `Backend::cross_file_resolution_enabled`.
#[derive(Clone, Copy)]
pub(super) struct XcToggles {
    pub(super) xc_diagnostics: bool,
    pub(super) cross_file_resolution: bool,
}

/// The per-run analyser feature toggles for a diagnostics run, grouped so the
/// owned [`DiagInputs`] doesn't accumulate a flat row of `bool` fields.  Each is
/// resolved per folder in [`Backend::diag_inputs`].  (The client-capability
/// `client_supports_pull` is kept separate — it is a transport choice, not an
/// analyser feature.)
#[derive(Clone, Copy)]
pub(super) struct DiagToggles {
    /// Master diagnostics switch (`tclLsp.features.diagnostics`). When `false`
    /// the pipeline publishes an empty set (clearing squiggles) instead of
    /// analysing.
    pub(super) diagnostics_enabled: bool,
    /// `tclLsp.diagnostics.exclude` matched this document: the pipeline
    /// publishes an empty set (clearing squiggles) instead of analysing,
    /// exactly as the master switch does.
    pub(super) excluded: bool,
    /// The `xcDiagnostics` / `crossFileResolution` pair — see [`XcToggles`].
    pub(super) xc: XcToggles,
}

/// Owned inputs for a detached diagnostics run — the document-independent
/// handles [`run_diagnostics_core`] (and the worker's per-drain [`DiagJob`]
/// capture) need without borrowing `Backend`, so the work can run on a
/// `tokio::spawn`ed task off the LSP event loop.  `Clone` so the coalescing
/// worker can reuse it across successive runs of one document.
#[derive(Clone)]
pub(super) struct DiagInputs {
    pub(super) inputs_epoch: Option<u64>,
    pub(super) configuration_epoch: u64,
    pub(super) sidecar_revision: Arc<fresh_analysis_cache::SidecarRevision>,
    pub(super) source_inputs_epoch: Arc<std::sync::atomic::AtomicU64>,
    pub(super) source_inputs_gate: Arc<tokio::sync::RwLock<()>>,
    pub(super) source_path_admissions: Arc<PublishedSourcePaths>,
    pub(super) client: Client,
    pub(super) diagnostic_publisher: Arc<DiagnosticPublisher>,
    pub(super) registry: Arc<CommandRegistry>,
    /// Scheduled runs leave the checked open-document registry to the drain,
    /// which already holds the document map: the scheduler must not take it.
    pub(super) registry_at_drain: bool,
    /// The exact pack publication selected with the registry for this run.
    pub(super) pack_seq: u64,
    pub(super) pack_set: Arc<tcl_spectcl::PackSet>,
    pub(super) spec_packs: Arc<Mutex<PublishedPackSet>>,
    /// The analyser's production-time skip for this document's folder (see
    /// [`LiftInputs::disabled`]).
    pub(super) disabled: HashSet<String>,
    /// The configuration layers this document's folder resolves its policy
    /// from — every decision that hides or relabels a finding is built from
    /// these by [`document_policy`], per run.
    pub(super) policy_layers: PolicyLayers,
    /// `tclLsp.extraCommands` — names treated as known by W123.
    pub(super) extra_commands: HashSet<String>,
    /// `tclLsp.diagnostics.genericVariablePatterns` — IRULE4002 generic-name
    /// patterns (`None` → default set, `Some` → replace). Only consulted on the
    /// no-salsa-input uncached fallback; the memoised path reads them off the
    /// salsa `AnalyserConfig`.
    pub(super) generic_variable_patterns: Option<Vec<String>>,
    /// Resolved `tclLsp.style.lineLength` (W111 max line length) for this
    /// document's folder; fed to the source-style checks. Distinct from the
    /// formatter's `tclLsp.formatting.lineLength`.
    pub(super) style_line_length: u32,
    pub(super) non_ascii_mode: NonAsciiMode,
    /// `tclLsp.sourceEncoding`, for reading saved sources the run compares.
    pub(super) script_encoding: tcl_registry::value_transfer::ScriptEncoding,
    pub(super) documents: Arc<DocumentStore>,
    /// Open-document diagnostic workers, used to invalidate only consumers of
    /// workspace facts changed by this publication.
    pub(super) diag_slots: Arc<Mutex<HashMap<Uri, DiagSlot>>>,
    pub(super) workspace_index: Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    /// The applied source-site seed record (see [`Backend`]); the publish
    /// path invalidates a document's entry when it re-indexes it standalone,
    /// so the next cross-document query re-applies the seeded views.
    pub(super) rehomed_source_seeds: Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// Serialises standalone index publication with source-site reconciliation
    /// and the workspace-wide query snapshot that consumes it. See
    /// [`crate::BackendState::rehoming_gate`].
    pub(super) rehoming_gate: Arc<tokio::sync::Mutex<()>>,
    /// Serialises the coverage/project snapshot with live source membership
    /// publication. See [`crate::BackendState::live_publication_gate`].
    pub(super) live_publication_gate: Arc<Mutex<()>>,
    /// Package database for the W120 workspace-refinement post-filter.
    pub(super) package_resolver: Arc<RwLock<PackageResolver>>,
    pub(super) package_prefer: tcl_lsp_core::package_resolver::PackagePrefer,
    pub(super) provider_dialects: provider_source::ProviderDialectSources,
    /// Memo for the unclosed-delimiter recovery path's widened known-command
    /// set (see [`RecoveryNameCache`]).
    pub(super) recovery_names: Arc<Mutex<RecoveryNameCache>>,
    /// `.tcl-lsp.ini [project] entryPoints` for this document's folder:
    /// when non-empty, the W120 refinement inherits these entries' requires and
    /// disables the automatic `source`-graph inheritance.
    pub(super) entry_points: Vec<String>,
    /// The document's workspace-folder root, used to resolve relative
    /// `entry_points` to file URIs. `None` for a no-folder session.
    pub(super) folder_root: Option<PathBuf>,
    /// The salsa db handle.  Each run clones a *fresh, short-lived* snapshot and
    /// drops it immediately, so an idle worker never holds a clone across the
    /// debounce sleep (which would block the next edit's `set_text` — salsa's
    /// exclusive-access write waits for all outstanding handles to drop).
    pub(super) db: Arc<TrackedMutex<tcl_lsp_db::TclDatabase>>,
    pub(super) db_files: Arc<TrackedMutex<HashMap<Uri, tcl_lsp_db::SourceFile>>>,
    /// The admitted cross-document subset of `db_files`; local-only orphan
    /// handles must not make a load target look covered.
    pub(super) db_project_members: Arc<Mutex<HashSet<Uri>>>,
    /// The salsa `Project` input handle (workspace file set), read by the worker
    /// for opt-in cross-file callback-arity validation.
    pub(super) db_project: Arc<Mutex<Option<tcl_lsp_db::Project>>>,
    pub(super) class_factory_generation: Arc<tokio::sync::RwLock<u64>>,
    pub(super) db_config: Arc<Mutex<tcl_lsp_db::AnalyserConfig>>,
    /// Per-folder salsa `AnalyserConfig` handles (see
    /// [`crate::BackendState::folder_db_configs`]); `capture_job` resolves the right one by
    /// the document's URI so folder-scoped W-code suppression reaches the
    /// cached analysis, falling back to `db_config`.
    pub(super) folder_db_configs: Arc<Mutex<Vec<(Uri, tcl_lsp_db::AnalyserConfig)>>>,
    /// Pull-diagnostic cache, updated as each push run publishes so the
    /// `textDocument/diagnostic` / `workspace/diagnostic` paths return the
    /// last-published set.
    pub(super) pull_diag_cache: Arc<Mutex<HashMap<Uri, PullDiagEntry>>>,
    /// Per-URI generation counter for **closed**-file diagnostics runs.
    /// Each `publish_closed_file_diagnostics` bumps it and captures the new
    /// value into its `DiagCurrency::ClosedFromDisk`; the publish-time currency
    /// guard drops any closed run whose captured generation is no longer the
    /// latest, so an older run finishing after a newer one (rapid re-saves /
    /// overlapping close + watched-change) can never overwrite the current set
    /// with stale diagnostics — the closed-file equivalent of the open path's
    /// `revision` guard.
    pub(super) closed_diag_gen: Arc<Mutex<HashMap<Uri, u64>>>,
    /// Per-run analyser feature toggles (diagnostics master switch, optimiser,
    /// `xcDiagnostics`, `crossFileResolution`), grouped to keep this owned
    /// struct's flat `bool` count low.
    pub(super) toggles: DiagToggles,
    /// Snapshot of [`Backend::client_supports_pull_diagnostics`].  When `true`
    /// the worker keeps the pull cache current and asks the client to re-pull
    /// instead of pushing — see that field for the rationale.
    pub(super) client_supports_pull: bool,
    /// Where the recovery-widening package read gets its bytes — see
    /// [`crate::vfs`].
    pub(super) store: Arc<dyn vfs::SourceStore>,
}

/// Exact cache/mailbox publication identity, transported with its own ack.
pub(super) struct IssuedDiagnosticPublication {
    pub(super) result_id: String,
    pub(super) receipt: tokio::sync::oneshot::Receiver<DiagnosticPublishOutcome>,
}

/// The salsa handles + per-edit identity shared by the two cancellable analysis
/// passes ([`compute_base_analysis`], [`compute_compiler_diags`]).  Borrows for
/// the duration of one `run_diagnostics_core` call.
pub(super) struct SalsaAnalysisCtx<'a> {
    pub(super) db: &'a Arc<TrackedMutex<tcl_lsp_db::TclDatabase>>,
    pub(super) uri: &'a Uri,
    pub(super) file: Option<tcl_lsp_db::SourceFile>,
    pub(super) config: tcl_lsp_db::AnalyserConfig,
    pub(super) text: &'a str,
    pub(super) dialect: &'static tcl_dialect::DialectProfile,
}

/// The non-document-buffer handles the analyser/compiler/publish path needs.
pub(super) struct AnalyserPathInputs<'a> {
    pub(super) sidecar_revision: &'a fresh_analysis_cache::SidecarRevision,
    pub(super) registry: Arc<CommandRegistry>,
    pub(super) extra_commands: &'a HashSet<String>,
    pub(super) generic_variable_patterns: Option<&'a [String]>,
    pub(super) non_ascii_mode: NonAsciiMode,
    pub(super) script_encoding: tcl_registry::value_transfer::ScriptEncoding,
    pub(super) db_project: &'a Arc<Mutex<Option<tcl_lsp_db::Project>>>,
    pub(super) workspace_index: &'a Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    /// The applied source-site seed record; the publish path invalidates
    /// entries.
    pub(super) rehomed_source_seeds: &'a Arc<Mutex<HashMap<String, Vec<String>>>>,
    /// Publication/query transaction gate; see [`crate::BackendState::rehoming_gate`].
    pub(super) rehoming_gate: &'a Arc<tokio::sync::Mutex<()>>,
    pub(super) package_resolver: &'a Arc<RwLock<PackageResolver>>,
    pub(super) package_prefer: tcl_lsp_core::package_resolver::PackagePrefer,
    pub(super) provider_capture: provider_source::ProviderCaptureInputs<'a>,
    /// Memo for the unclosed-delimiter recovery path's widened known-command
    /// set (see [`RecoveryNameCache`]).
    pub(super) recovery_names: &'a Arc<Mutex<RecoveryNameCache>>,
    /// W120 inheritance for this document's folder (see [`DiagInputs`]).
    pub(super) entry_points: &'a [String],
    pub(super) folder_root: Option<&'a Path>,
    /// Whether opt-in cross-file resolution is enabled — see
    /// `Backend::cross_file_resolution_enabled`. Independent of
    /// `LiftInputs::xc_diagnostics` (the unrelated f5-irules-specific
    /// XC100-301 translatability lints).
    pub(super) cross_file_resolution: bool,
}

/// The document-style toggles + buffer the diagnostic lifts read; borrows for
/// one `run_diagnostics_core` call.
pub(super) struct LiftInputs<'a> {
    pub(super) rewrite_source: Option<rewrite_wire::RewriteWireSource>,
    pub(super) text: &'a str,
    pub(super) decode_report: Option<tcl_lsp_core::source_decode::DecodeReport>,
    pub(super) dialect: &'static tcl_dialect::DialectProfile,
    /// The analyser's production-time skip — the codes it may leave
    /// uncomputed (`docs/design/compiler/diagnostic-policy.md` § Producers
    /// that change), declared to the report. Never a presentation filter:
    /// what the document shows is the policy step's decision alone.
    pub(super) disabled: &'a HashSet<String>,
    /// The configuration layers the document's policy is built from.
    pub(super) policy_layers: &'a PolicyLayers,
    pub(super) style_line_length: u32,
    pub(super) xc_diagnostics: bool,
}

/// The workspace-wide inputs the W120 / W123 refinements resolve against,
/// grouped so [`refine_and_lift_diagnostics`] takes them as one parameter.
struct RefinementInputs<'a> {
    inheritance: &'a SourceInheritance,
    pub(super) package_resolver: &'a Arc<RwLock<PackageResolver>>,
    /// Each provider's own checked source/configuration, captured before resolution.
    pub(super) provider_sources: &'a provider_source::ProviderSourceAccess<'a>,
    pub(super) registry: &'a CommandRegistry,
    /// The document's exact source-owned call candidates and their
    /// conditional source arity projections.
    settled_calls: CrossFileCalls,
}

/// The resolved per-document settings an F5 model dialect's pull report reads
/// — everything [`Backend::f5_pull_report`] needs beyond the buffers and the
/// URI.
#[cfg(test)]
struct F5PullInputs<'a> {
    pub(super) profile: &'static tcl_dialect::DialectProfile,
    pub(super) policy_layers: &'a PolicyLayers,
    pub(super) decode_report: Option<tcl_lsp_core::source_decode::DecodeReport>,
}

/// The buffers and facts of a non-Tcl F5 model document, for
/// [`f5_model_report`].
pub(super) struct F5ModelDocument<'a> {
    pub(super) text: &'a str,
    pub(super) analysis_text: &'a str,
    pub(super) decode_report: Option<&'a tcl_lsp_core::source_decode::DecodeReport>,
    pub(super) dialect: &'static tcl_dialect::DialectProfile,
}

/// The document-local settings shared by every pull-diagnostic workspace
/// refinement. Keeping them together makes the pull path's central
/// finalisation entry point explicit without repeating its configuration at
/// each call site.
struct PullRefinementInputs<'a> {
    pub(super) dialect: &'static tcl_dialect::DialectProfile,
    pub(super) registry: &'a CommandRegistry,
    pub(super) disabled: &'a HashSet<String>,
}

/// Timing for the `[timing] workspace_state.update` log line.
pub(super) struct PublishTiming<'a> {
    pub(super) started: crate::rt::Instant,
    pub(super) uri_str: &'a str,
    pub(super) line_count: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct IndexedDiagnosticChange {
    pub(super) original_changed: bool,
    pub(super) original_commands: Vec<(
        tcl_core_types::ByteCommandSlot,
        tcl_syntax::naming::NamePolicyProtocol,
    )>,
    pub(super) original_sites: Vec<tcl_compiler::command_binding::CommandAllocationSite>,
    pub(super) command_names: HashSet<String>,
    pub(super) source_or_package_changed: bool,
}

/// Publish the lifted diagnostics: re-check currency, refresh the workspace
/// index, atomically commit the pull cache + mailbox state, await delivery
/// outside all global guards, and log timing. Always
/// settled (`true`) — a stale revision or a lift-revision panic both keep the
/// prior diagnostics rather than retrying.
pub(super) struct DiagnosticPublicationSources {
    pub(super) result: DiagnosticProviderResult,
    pub(super) workspace_sources: Option<Arc<WorkspaceDiagnosticSources>>,
}

/// Everything the workspace `source` graph contributes to one document's
/// W120 / W123 verdicts — both directions of the graph plus the abstention
/// flag.
///
/// See [`tcl_lsp_core::source_graph`] for why the two directions differ: the
/// **down** direction (what my callers loaded before running me) is ambient
/// over the whole document, while the **up** direction (what the files I
/// `source` loaded for me) starts at the `source` statement and so is placed.
#[derive(Debug, Default, Clone)]
pub struct SourceInheritance {
    pub(super) original: Option<OriginalSourceInheritance>,
    /// Requires that hold for the whole document: the configured project
    /// entry points' requires, or — in automatic mode — the requires of every
    /// file that transitively `source`s this one.
    pub(super) ambient: Vec<String>,
    /// Requires this document acquires by `source`ing other files, each at the
    /// offset of the `source` statement that brings it in.
    pub(super) placed: Vec<tcl_lsp_core::source_graph::PlacedRequire>,
    /// This document has a `source` whose target the server could not pin to
    /// an indexed document — a path it cannot fold statically, a file outside
    /// the workspace, or one that does not exist.
    ///
    /// That file may `package require` anything and define any command, so the
    /// honest answer to "is this command/package missing?" becomes *unknown*
    /// and W120 / W123 abstain document-wide. Exactly the bargain
    /// `handle_namespace_unknown_command` and the dynamic-pattern branch of
    /// `handle_namespace_import_command` already strike by flipping
    /// `has_dynamic_providers`.
    pub(super) unresolvable_source: bool,
}

#[derive(Debug, Default, Clone)]
pub(super) struct OriginalSourceInheritance {
    pub(super) prefer: tcl_lsp_core::package_resolver::PackagePrefer,
    ambient: Vec<tcl_lsp_core::package_resolver::PackageRequirementAdvice>,
    pub(super) placed: Vec<
        tcl_lsp_core::source_graph::PlacedRequirement<
            tcl_lsp_core::package_resolver::PackageRequirementAdvice,
        >,
    >,
}

#[derive(Debug, Default)]
pub(super) struct CrossFileCalls {
    pub(super) source_candidates: Vec<core_workspace_index::LogicalWorkspaceCommandCandidate>,
    /// Actual workspace source-header queries, never runtime script coverage.
    pub(super) command_queries: WorkspaceCommandQueries,
    pub(super) resolved: HashSet<(u32, u32)>,
    pub(super) arity: HashMap<(u32, u32), ProcArityUnion>,
}
