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

//! Analyses of one document: query-database reads, fresh analysers and the registry they run under.

use super::Backend;
use super::documents::DocumentPathAdmission;
use std::collections::HashMap;
use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::workspace_index as core_workspace_index;
use tower_lsp_server::ls_types::Uri;

mod analysers;
mod fresh;
mod queries;

pub(super) use analysers::default_analyser_config;
pub(super) use fresh::FreshAnalysisDriver;

#[derive(Clone)]
pub(super) struct FreshAnalysisSeed {
    pub(super) path_admission: Arc<DocumentPathAdmission>,
    pub(super) analysis: Arc<AnalysisResult>,
    pub(super) analyser_inputs_epoch: u64,
    pub(super) class_factory_generation: u64,
}

impl Backend {
    /// The workspace class-factory oracle currently published to the project,
    /// or `None` when the workspace has no metaclass (the overwhelmingly
    /// common case, and the state every file starts in).
    ///
    /// One index serves every file, so any file carrying one carries *the* one.
    /// Mid-round the publish is applied file by file, so a few may still hold
    /// the previous, smaller index; the widest published index is the right
    /// seed, and the next compare-then-set round corrects it either way.
    pub(super) fn published_class_factories(
        db: &tcl_lsp_db::TclDatabase,
        files: &HashMap<Uri, tcl_lsp_db::SourceFile>,
    ) -> Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>> {
        files
            .values()
            .filter_map(|file| file.workspace_class_factories(db).clone())
            .max_by_key(|index| index.len())
    }

    /// The workspace class-factory oracle this document is analysed under,
    /// cloned out of the salsa db so a `spawn_blocking` analysis can carry it.
    ///
    /// `None` when the document is not in the salsa db, which is the honest
    /// answer — no workspace view — rather than a claim of one.  Lock order is
    /// `db` → `db_files`, matching [`Self::db_set_source`].
    pub(super) async fn class_factories_for(
        &self,
        uri: &Uri,
    ) -> Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>> {
        let db = self.db.lock().await;
        let files = self.db_files.lock().await;
        files.get(uri)?.workspace_class_factories(&*db).clone()
    }

    /// The widest class-factory oracle currently published anywhere in the
    /// project, for standalone analysis that may run before `uri` has its
    /// first Salsa handle.
    ///
    /// Unlike [`Self::class_factories_for`], this deliberately is not keyed by
    /// the consumer URI: the oracle is a project-wide fact, and a brand-new or
    /// untitled pending buffer is entitled to the same view as existing files
    /// even though it cannot carry that view on a `SourceFile` yet.
    pub(super) async fn published_class_factories_for_fresh_analysis(
        &self,
    ) -> (u64, Option<Arc<tcl_compiler::analyser::ClassFactoryIndex>>) {
        let generation = self.class_factory_generation.read().await;
        let db = self.db.lock().await;
        let files = self.db_files.lock().await;
        (*generation, Self::published_class_factories(&db, &files))
    }

    /// This document's cross-file call-site evidence, cloned out of the salsa
    /// db so a `spawn_blocking` build can carry it.
    ///
    /// The tracked `compilation_unit` query reads
    /// [`tcl_lsp_db::SourceFile::external_call_sites`] itself, but every
    /// **uncached** build — the pull-diagnostics path and code actions — makes
    /// a standalone unit that cannot. Without this a pull-mode client gets a
    /// correct analyser I230 alongside an unsound optimiser O101 for the same
    /// shape.
    ///
    /// `None` when the document is not in the salsa db (no workspace view to
    /// speak of), which is the honest answer rather than a claim of one.
    /// Lock order is `db` → `db_files`, matching [`Self::db_set_source`].
    pub(super) async fn cross_file_evidence_for(
        &self,
        uri: &Uri,
    ) -> Option<Arc<tcl_compiler::unit_scope::CallSiteEvidence>> {
        let db = self.db.lock().await;
        let files = self.db_files.lock().await;
        files.get(uri)?.external_call_sites(&*db).clone()
    }

    /// The workspace's `namespace export` records, as the whole-program
    /// oracle the single-document providers consult.
    ///
    /// Taken under the index lock and returned owned, so it can be moved into
    /// the `spawn_blocking` worker that runs the provider with the lock
    /// released. It is a derived view of the index, built at most once per
    /// index generation, so calling this per request costs an `Arc` clone.
    pub(super) async fn export_snapshot(
        &self,
    ) -> Arc<core_workspace_index::NamespaceExportSnapshot> {
        self.workspace_index.read().await.export_snapshot()
    }
}
