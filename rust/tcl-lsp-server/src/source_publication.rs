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

//! Publishing sources to the query database and the workspace index: lock bundles, live buffers and disk files.

use super::diagnostics::DiagSlot;
use super::documents::DocumentPathAdmission;
use super::method_families::WorkspaceClassAnalysis;
use super::tracked_locks::{TrackedMutexGuard, TrackedWriteGuard};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_lsp_core::workspace_index as core_workspace_index;
use tower_lsp_server::ls_types::Uri;

mod db_sources;
mod dependency_waits;
mod disk_files;
mod live_buffers;
mod live_index;
mod lock_bundles;

#[cfg(test)]
pub(super) use dependency_waits::{LivePublicationRetry, LivePublicationWait};
pub(super) use disk_files::{AdmittedDiskBatch, DiskPublicationInputs, DiskPublicationOutcome};

#[derive(Clone)]
pub(super) struct PublishedSourcePath {
    pub(super) file: tcl_lsp_db::SourceFile,
    pub(super) admission: Arc<DocumentPathAdmission>,
}

pub(super) type PublishedSourcePaths = std::sync::Mutex<HashMap<Uri, PublishedSourcePath>>;

#[derive(Clone, Copy)]
pub(super) struct LiveSourceContents<'a> {
    pub(super) admission: &'a DocumentPathAdmission,
    pub(super) text: &'a str,
    pub(super) dialect: &'a str,
    pub(super) project_member: bool,
}

/// Every mutable store a source publication changes, acquired as one
/// optimistic bundle before it inspects the open-document map.
///
/// The bundle is deliberately built with `try_*` calls: no publisher queues
/// for one lock while retaining another. That makes foreground document sync
/// the priority path and keeps each final no-`await` commit small.
struct PublicationLocks<'a> {
    pub(super) db: TrackedMutexGuard<'a, tcl_lsp_db::TclDatabase>,
    pub(super) files: TrackedMutexGuard<'a, HashMap<Uri, tcl_lsp_db::SourceFile>>,
    tombstones: tokio::sync::MutexGuard<'a, HashMap<Uri, tcl_lsp_db::SourceFile>>,
    project_members: tokio::sync::MutexGuard<'a, HashSet<Uri>>,
    pub(super) project: tokio::sync::MutexGuard<'a, Option<tcl_lsp_db::Project>>,
    pub(super) index: TrackedWriteGuard<'a, core_workspace_index::WorkspaceIndex>,
    seeds: tokio::sync::MutexGuard<'a, HashMap<String, Vec<String>>>,
    pub(super) global_config: tokio::sync::MutexGuard<'a, tcl_lsp_db::AnalyserConfig>,
    pub(super) folder_configs: tokio::sync::MutexGuard<'a, Vec<(Uri, tcl_lsp_db::AnalyserConfig)>>,
}

/// The smaller write set an editor-opened buffer needs for its deferred Salsa
/// and workspace-index publication.
///
/// `rehomed_source_seeds` is intentionally absent: opening a live buffer does
/// not read or mutate that map, so retaining its guard would only widen the
/// foreground contention surface.
pub(super) struct LiveSourceLocks<'a> {
    pub(super) db: TrackedMutexGuard<'a, tcl_lsp_db::TclDatabase>,
    pub(super) files: TrackedMutexGuard<'a, HashMap<Uri, tcl_lsp_db::SourceFile>>,
    tombstones: tokio::sync::MutexGuard<'a, HashMap<Uri, tcl_lsp_db::SourceFile>>,
    project_members: tokio::sync::MutexGuard<'a, HashSet<Uri>>,
    pub(super) project: tokio::sync::MutexGuard<'a, Option<tcl_lsp_db::Project>>,
    pub(super) global_config: tokio::sync::MutexGuard<'a, tcl_lsp_db::AnalyserConfig>,
    pub(super) folder_configs: tokio::sync::MutexGuard<'a, Vec<(Uri, tcl_lsp_db::AnalyserConfig)>>,
}

struct OpenPublicationLocks<'a> {
    pub(super) source: LiveSourceLocks<'a>,
    pub(super) index: TrackedWriteGuard<'a, core_workspace_index::WorkspaceIndex>,
}

/// The source stores plus semantic-token lifecycle caches reset by a reopen.
/// Acquiring them as one optimistic bundle makes the reset visible before the
/// reopened document reaches Salsa readiness without awaiting under `EditOrder`.
struct ReopenPublicationLocks<'a> {
    pub(super) source: LiveSourceLocks<'a>,
    pub(super) last_semantic_tokens: tokio::sync::MutexGuard<'a, HashMap<Uri, (String, Vec<u32>)>>,
    pub(super) semantic_tokens_refresh_asked: tokio::sync::MutexGuard<'a, HashMap<Uri, u64>>,
    pub(super) workspace_class_analyses:
        tokio::sync::MutexGuard<'a, HashMap<Uri, WorkspaceClassAnalysis>>,
}

/// Every derived live-buffer cache retired by `didClose`, acquired
/// optimistically so the final closed-state check and cleanup are atomic with
/// respect to a racing `didOpen`.
struct ClosePublicationLocks<'a> {
    pub(super) index: TrackedWriteGuard<'a, core_workspace_index::WorkspaceIndex>,
    pub(super) diag_slots: tokio::sync::MutexGuard<'a, HashMap<Uri, DiagSlot>>,
    pub(super) last_semantic_tokens: tokio::sync::MutexGuard<'a, HashMap<Uri, (String, Vec<u32>)>>,
    pub(super) semantic_tokens_refresh_asked: tokio::sync::MutexGuard<'a, HashMap<Uri, u64>>,
    pub(super) workspace_class_analyses:
        tokio::sync::MutexGuard<'a, HashMap<Uri, WorkspaceClassAnalysis>>,
}

/// Whether a disk-side removal is allowed to retire an editor-open URI.
#[derive(Clone, Copy)]
pub(super) enum DiskRemovalPolicy<'a> {
    /// The actual selected old path from a didRenameFiles notification.
    /// Its original behavior retires the old source even if its buffer is open.
    RenamedPath,
    /// Folder removal and stale scans preserve live, possibly-unsaved buffers.
    ClosedOnly,
    /// A filesystem `DELETED` event retires the dead path even if its buffer is
    /// still open; the document remains usable but is marked orphaned.
    IncludeOpen(&'a HashMap<Uri, u64>),
}
