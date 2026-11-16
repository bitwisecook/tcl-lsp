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

//! Retained source interpretations and their independent physical publications.
//!
//! Source-only interpretations retain physical leases independently of loading,
//! whole-program completeness, Native execution and edit authority.
//! A view key is a locator; only a live table join under the source lease answers.

mod symbol_sources;
pub(super) use symbol_sources::WorkspaceSymbolViewGuards;
pub(crate) use symbol_sources::{WorkspaceSymbolSourceView, WorkspaceSymbolSourceViewCache};

use super::{
    Arc, Backend, ReadonlySourceDocument, WorkspaceSymbolFreshInput, WorkspaceSymbolSourceAnalysis,
};
use std::collections::{HashMap, HashSet};
use tcl_compiler::auto_path_eval::FoldedPathConstants;
use tcl_compiler::command_binding::OriginalSourcePointEntry;
use tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceCurrentCut;
#[cfg(test)]
use tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceFrontier as SourceFrontier;
use tower_lsp_server::ls_types::Uri;

mod call_hierarchy;
mod command_source_worlds;
mod conditional_autoload;
mod conditional_regions;
mod draft_capture;
mod frontier_walk;
mod hover;
mod navigation;
mod publication;
mod root_reconciliation;
mod settled_scope;
mod view_drafts;
mod view_table;

/// Conditional globals issued from this exact live parent's pre-handler cut.
/// The values supply neither runtime stores nor absence of namespace shadows.
#[derive(Clone)]
struct IncomingSourcePathAdvice {
    parent: Arc<PublishedOriginalSourceView>,
    cut: Arc<OriginalProjectSourceCurrentCut>,
    constants: Option<FoldedPathConstants>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct OriginalSourceViewId(u64);

#[derive(Clone)]
struct InheritedSourceView {
    parent: Arc<PublishedOriginalSourceView>,
    point: Arc<OriginalSourcePointEntry>,
}

#[derive(Clone)]
struct ConditionalRegionSourceView {
    parent: Arc<PublishedOriginalSourceView>,
    child_prefix: Arc<PublishedOriginalSourceView>,
    completion_dependencies: Vec<Arc<PublishedOriginalSourceView>>,
    chain: Arc<tcl_compiler::command_binding::OriginalSourceConditionalRegionSet>,
    region: tcl_compiler::command_binding::OriginalSourceConditionalRegion,
}

#[derive(Clone)]
pub(crate) struct OriginalSourceViewDraft {
    physical: Arc<ReadonlySourceDocument>,
    producer: Arc<WorkspaceSymbolFreshInput>,
    inherited: Option<InheritedSourceView>,
    region: Option<ConditionalRegionSourceView>,
    incoming_paths: Option<Arc<IncomingSourcePathAdvice>>,
}

pub(crate) struct PublishedOriginalSourceView {
    id: OriginalSourceViewId,
    draft: OriginalSourceViewDraft,
}

/// Tentative ordinary authoring input, held privately until actual source
/// associations select roots. It supplies no locator or child interpretation.
#[derive(Clone)]
struct OriginalSourceRootCandidate {
    draft: OriginalSourceViewDraft,
    published: Option<Arc<PublishedOriginalSourceView>>,
}

struct OriginalSourceRootWork {
    candidates: Vec<OriginalSourceRootCandidate>,
    pending: HashSet<Uri>,
    associated: HashSet<Uri>,
    incoming: HashMap<Uri, HashSet<Uri>>,
}

struct OriginalSourceRootReconciliation {
    bootstrap: Vec<OriginalSourceViewDraft>,
    scopes: Vec<OriginalSourceRootScope>,
    excluded: Vec<OriginalSourceRootCandidate>,
    incoming: HashMap<Uri, HashSet<Uri>>,
}

/// Session-owned sequence is never reset when the workspace index is replaced.
#[derive(Default)]
pub(crate) struct OriginalSourceViewTable {
    next_id: u64,
    revision: u64,
    revision_exhausted: bool,
    by_id: HashMap<OriginalSourceViewId, Arc<PublishedOriginalSourceView>>,
    by_uri: HashMap<Uri, Vec<OriginalSourceViewId>>,
}

/// Publication currency can be recaptured; absent semantic producers cannot.
/// This carries no authority until the existing final source/index lease joins.
pub(crate) enum OriginalSourceViewAttempt<T> {
    Ready(T),
    RetryCapture,
    Unavailable,
}

impl<T> OriginalSourceViewAttempt<T> {
    fn from_semantic(value: Option<T>) -> Self {
        value.map_or(Self::Unavailable, Self::Ready)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SourceViewPublicationMode {
    ReplaceFirstEdge,
    AppendInherited,
    AppendRegions,
}

/// A compiler-issued completion retains every physical view used to derive it.
/// A saved outcome alone supplies no host publication currency.
#[derive(Clone)]
struct OriginalSourceViewCompletion {
    prefix: Arc<PublishedOriginalSourceView>,
    outcome: tcl_compiler::command_binding::OriginalSourceConditionalOutcome,
    dependencies: Vec<Arc<PublishedOriginalSourceView>>,
}

struct OriginalSourceRegionWork {
    parent: Arc<PublishedOriginalSourceView>,
    child_prefix: Arc<PublishedOriginalSourceView>,
    chain: Arc<tcl_compiler::command_binding::OriginalSourceConditionalRegionSet>,
    views: Vec<Arc<PublishedOriginalSourceView>>,
    completion_dependencies: Vec<Arc<PublishedOriginalSourceView>>,
}

/// One unresolved source association belongs to the exact retained view.
/// It can move to a successor only through the compiler's direct replay join.
#[derive(Clone)]
struct OriginalSourceViewDebt {
    owner: Arc<PublishedOriginalSourceView>,
    edge: tcl_lsp_core::workspace_index::WorkspaceSource,
}

#[derive(Clone)]
struct OriginalSourceViewEdges {
    edges: Vec<tcl_lsp_core::workspace_index::WorkspaceSource>,
    replayed: Vec<OriginalSourceViewDebt>,
}

struct OriginalSourceViewAssociation {
    target: Uri,
    owner: Arc<PublishedOriginalSourceView>,
    edge: tcl_lsp_core::workspace_index::WorkspaceSource,
}

struct OriginalSourceViewWorkState {
    tasks: Vec<SourceViewWork>,
    active: HashSet<OriginalSourceViewId>,
    finished: HashMap<OriginalSourceViewId, Option<OriginalSourceViewCompletion>>,
    membership: HashSet<OriginalSourceViewId>,
    unknown: Vec<OriginalSourceViewDebt>,
    visited: HashSet<Uri>,
    association_owners: Vec<OriginalSourceViewAssociation>,
    conditional_obligations: bool,
}

enum OriginalSourceViewFrontierStep {
    NoCut,
    Pending,
    RetryCapture,
    Cut(Box<tcl_lsp_core::workspace_index::WorkspaceSource>),
}

enum SourceViewWork {
    Enter(Arc<PublishedOriginalSourceView>),
    AfterChild {
        parent: Arc<PublishedOriginalSourceView>,
        child: Arc<PublishedOriginalSourceView>,
        load: Arc<tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad>,
    },
    AfterRegion {
        work: OriginalSourceRegionWork,
        region: Arc<PublishedOriginalSourceView>,
    },
}

/// Host membership of one bounded exact source-view reconciliation. Kernel
/// conditional outcomes remain separate from physical/Native completion.
#[derive(Clone, Copy)]
enum PartialDefinitionReason {
    UnfinishedSuffixWithoutFurtherCut,
}

/// Recapture eligibility only; supplies no source, index or completion proof.
struct OriginalSourceViewRecapture;

struct OriginalSourceRootScope {
    recapture: Option<OriginalSourceViewRecapture>,
    root: Option<OriginalSourceViewId>,
    root_uri: Uri,
    required: HashSet<Uri>,
    association_owners: Vec<OriginalSourceViewAssociation>,
    ids: Vec<OriginalSourceViewId>,
    complete: bool,
    partial_definition: Option<PartialDefinitionReason>,
    completion: Option<OriginalSourceViewCompletion>,
    eligibility: HashSet<Uri>,
    conditional_obligations: bool,
    /// A root without a source cut whose own command world may hold an
    /// unknown mutation. Only positive references consume it.
    root_world_uncertain: bool,
}

pub(crate) struct OriginalSourceViewSettlement {
    scopes: Vec<OriginalSourceRootScope>,
    revision: u64,
}

pub(crate) struct OriginalCommandSourceWorldCapture {
    settlement: OriginalSourceViewSettlement,
}

struct OriginalSourceScopeProjection {
    ids: Vec<OriginalSourceViewId>,
    revision: u64,
    partial_definition_groups: Vec<Vec<OriginalSourceViewId>>,
}

/// Bound on source-view navigation attempts refused by table contention.
pub(crate) const SOURCE_VIEW_CONTENTION_ATTEMPTS: usize = 40;
/// Pause between those attempts.
pub(crate) const SOURCE_VIEW_CONTENTION_BACKOFF: std::time::Duration =
    std::time::Duration::from_millis(25);

/// Independent current provider header for conditional deferred-body advice.
/// It is not installed in a source view or promoted to a runtime binding.
struct ConditionalAutoloadProviderSource {
    uri: Uri,
    source: Arc<str>,
    admission: Arc<super::super::DocumentPathAdmission>,
    publication: super::super::LibraryPublicationSource,
}

struct ConditionalAutoloadProvider {
    source: ConditionalAutoloadProviderSource,
    owner: WorkspaceSymbolSourceAnalysis,
}

#[cfg(all(test, not(target_family = "wasm")))]
mod completed_world_owner_controls;

#[cfg(all(test, not(target_family = "wasm")))]
mod closed_source_child_ingress_tests;
