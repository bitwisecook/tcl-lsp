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

//! The workspace beyond open documents: folder scans, disk reindexing, watched files, libraries and project-wide facts.

use super::*;

mod class_factories;
mod disk_reindex;
mod folder_scan;
mod folders;
mod library_indexing;
mod package_database;
mod project_inputs;
mod scan_batches;
mod source_files;
mod source_graph;
mod source_rehoming;
mod warm;
mod watched_files;

pub(super) use class_factories::{
    CLASS_FACTORY_SYNC_ROUNDS, reindex_unopened_factory_consumers, sync_workspace_class_factories,
};
#[cfg(test)]
pub(super) use class_factories::{
    ClassFactoryRound, apply_workspace_class_factories_if_current,
    sync_workspace_class_factories_round,
};
#[cfg(test)]
pub(super) use disk_reindex::DiskScanOutcome;
pub(super) use folder_scan::{WorkspaceScanReadiness, WorkspaceStartupGuard};
pub(super) use library_indexing::LibraryPublicationSource;
use package_database::extend_resolver_with_document_auto_paths;
pub(super) use package_database::{
    build_package_resolver, document_auto_path_dirs, effective_auto_path,
};
pub(super) use project_inputs::{
    CrossFileEvidenceSnapshot, capture_cross_file_evidence_snapshot, refresh_cross_file_evidence,
};
#[cfg(test)]
pub(super) use project_inputs::{
    apply_cross_file_evidence_if_current, files_with_covered_load_targets, sync_cross_file_evidence,
};
use project_inputs::{
    sync_workspace_declared_commands, sync_workspace_source_modules,
    sync_workspace_subclass_methods,
};
use scan_batches::{WorkspaceScanCandidate, WorkspaceScanTaskWindow};
#[cfg(test)]
pub(super) use source_files::is_skipped_scan_dir;
pub(super) use source_files::{
    WORKSPACE_SCAN_DIR_CAP, WORKSPACE_SCAN_FILE_CAP, collect_tcl_files, is_tcl_source,
    tcl_source_glob, tcl_source_watch_glob, uri_under_folder,
};
#[cfg(test)]
pub(super) use source_graph::resolve_source_uri;
pub(super) use source_graph::{entry_point_uri, new_workspace_index, resolve_source_edge};
pub(super) use source_rehoming::RehomingDocumentSnapshot;
#[cfg(test)]
pub(super) use watched_files::{
    PACKAGE_LOCKFILE_GLOB, PROJECT_CONFIG_GLOB, is_config_file, is_package_metadata_file,
    is_spec_pack_file,
};
pub(super) use watched_files::{WatchedFileChanges, partition_watched_file_changes};

pub(super) struct ScanAnalysisInputs {
    pub(super) analyser_inputs_epoch: u64,
    pub(super) class_factory_generation: u64,
    pub(super) document_dialect_overrides: Arc<HashMap<String, String>>,
    pub(super) folder_dialects: Vec<(Uri, String)>,
    pub(super) default_dialect: String,
}

pub(super) struct ScannedDocument {
    pub(super) uri: Uri,
    pub(super) text: String,
    pub(super) dialect: String,
    pub(super) expected_analyser_inputs_epoch: Option<u64>,
    pub(super) expected_class_factory_generation: Option<u64>,
    pub(super) seed: FreshAnalysisSeed,
}

impl ScannedDocument {
    fn into_index_row(self) -> (Uri, String, String, AnalysisResult) {
        let analysis = match self.seed.analysis.analysis_context_unavailable.as_ref() {
            Some(miss) => {
                let mut result = AnalysisResult::default();
                result.dialect.clone_from(&self.dialect);
                result.analysis_context_unavailable = Some(miss.clone());
                result
            }
            None => self.seed.analysis.as_ref().clone(),
        };
        (self.uri, self.text, self.dialect, analysis)
    }
}

/// The handles [`sync_cross_file_evidence`] needs, cloned off a [`DiagInputs`]
/// before [`run_diagnostics_core`] consumes it.
pub(super) struct EvidenceHandles {
    pub(super) db_config: Arc<Mutex<tcl_lsp_db::AnalyserConfig>>,
    pub(super) folder_db_configs: Arc<Mutex<Vec<(Uri, tcl_lsp_db::AnalyserConfig)>>>,
    pub(super) db: Arc<TrackedMutex<tcl_lsp_db::TclDatabase>>,
    pub(super) db_files: Arc<TrackedMutex<HashMap<Uri, tcl_lsp_db::SourceFile>>>,
    pub(super) db_project_members: Arc<Mutex<HashSet<Uri>>>,
    pub(super) db_project: Arc<Mutex<Option<tcl_lsp_db::Project>>>,
    pub(super) workspace_index: Arc<TrackedRwLock<core_workspace_index::WorkspaceIndex>>,
    /// The open-document map, so the unopened-consumer re-index
    /// can tell an open buffer — whose own diagnostics worker republishes it —
    /// from a disk-backed file that nothing else will ever revisit.
    pub(super) documents: Arc<DocumentStore>,
    /// Serialises that re-index with source-site reconciliation, exactly as
    /// [`Backend::publish_scanned_disk_results`] does.
    pub(super) rehoming_gate: Arc<tokio::sync::Mutex<()>>,
    /// Keeps admitted membership and its Salsa `Project` input in the same
    /// generation while cross-file evidence captures both.
    pub(super) live_publication_gate: Arc<Mutex<()>>,
    /// Serialises class-factory oracle changes with standalone index seeds.
    pub(super) class_factory_generation: Arc<tokio::sync::RwLock<u64>>,
}

/// The publication transition produced by a class-factory fixpoint.
///
/// `affected_names` is the union of the oracle keys before and after the
/// transition.  Consumers of a removed factory therefore remain discoverable
/// even when the new oracle is empty.
pub(super) struct ClassFactorySync {
    pub(super) moved: Vec<Uri>,
    affected_names: HashSet<String>,
}

/// Where [`sync_workspace_class_factories`] sends each round's invalidated
/// peers, so a later round never sits in front of an earlier round's republish.
pub(super) struct RoundReschedule<'a> {
    pub(super) slots: &'a Arc<Mutex<HashMap<Uri, DiagSlot>>>,
    /// The document driving this pass; [`reschedule_peers`] skips it, because
    /// its own worker decides separately whether to re-run it.
    self_uri: &'a Uri,
}

struct WorkspaceScanWorker {
    pub(super) permit: tokio::sync::OwnedSemaphorePermit,
    pub(super) open: Arc<HashSet<Uri>>,
    pub(super) uri: Uri,
    pub(super) driver: Box<FreshAnalysisDriver>,
    pub(super) store: Arc<dyn vfs::SourceStore>,
    pub(super) path: PathBuf,
    scan_analyser_inputs_epoch: u64,
    scan_class_factory_generation: u64,
    pub(super) document_dialect_overrides: Arc<HashMap<String, String>>,
    pub(super) folder_dialects: Arc<Vec<(Uri, String)>>,
    pub(super) default_dialect: String,
}

struct PublishedWorkspaceScanBatch {
    batch: AdmittedDiskBatch,
    published: HashSet<Uri>,
    pub(super) rows: Vec<(Uri, String, String, AnalysisResult)>,
}
