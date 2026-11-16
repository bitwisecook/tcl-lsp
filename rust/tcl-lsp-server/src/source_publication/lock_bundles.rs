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

//! The write sets a publication acquires optimistically.

use super::super::Backend;
use super::super::tracked_locks::SnapshotDrainGuard;
use super::dependency_waits::LivePublicationWait;
use super::{
    ClosePublicationLocks, LiveSourceLocks, OpenPublicationLocks, PublicationLocks,
    ReopenPublicationLocks,
};

impl Backend {
    /// Optimistically acquire the complete source-publication write set.
    ///
    /// Returning the first unavailable dependency drops every partial
    /// acquisition. In particular, disk work never parks on `workspace_index`
    /// while retaining `db`, or parks on `db` while retaining the open-document
    /// map.
    pub(super) fn try_publication_locks(
        &self,
    ) -> Result<PublicationLocks<'_>, LivePublicationWait> {
        let db = self
            .db
            .try_lock()
            .map_err(|_| LivePublicationWait::Database)?;
        let files = self
            .db_files
            .try_lock()
            .map_err(|_| LivePublicationWait::Files)?;
        let tombstones = self
            .db_tombstones
            .try_lock()
            .map_err(|_| LivePublicationWait::Tombstones)?;
        let project_members = self
            .db_project_members
            .try_lock()
            .map_err(|_| LivePublicationWait::ProjectMembers)?;
        let project = self
            .db_project
            .try_lock()
            .map_err(|_| LivePublicationWait::Project)?;
        let index = self
            .workspace_index
            .try_write()
            .map_err(|_| LivePublicationWait::WorkspaceIndex)?;
        let seeds = self
            .rehomed_source_seeds
            .try_lock()
            .map_err(|_| LivePublicationWait::RehomedSourceSeeds)?;
        let global_config = self
            .db_config
            .try_lock()
            .map_err(|_| LivePublicationWait::GlobalConfig)?;
        let folder_configs = self
            .folder_db_configs
            .try_lock()
            .map_err(|_| LivePublicationWait::FolderConfigs)?;
        Ok(PublicationLocks {
            db,
            files,
            tombstones,
            project_members,
            project,
            index,
            seeds,
            global_config,
            folder_configs,
        })
    }

    /// Try the exact write set needed to publish one editor-opened buffer.
    ///
    /// Every partial acquisition is dropped on the first failure, and the
    /// typed error preserves which dependency was unavailable. Unlike the
    /// background bundle this deliberately excludes `rehomed_source_seeds`.
    pub(in super::super) fn try_live_source_locks(
        &self,
    ) -> Result<LiveSourceLocks<'_>, LivePublicationWait> {
        let db = self
            .db
            .try_lock()
            .map_err(|_| LivePublicationWait::Database)?;
        // Salsa setters synchronously wait in `cancel_others` while another
        // database clone lives. Once `db` is held, this zero is stable: every
        // tracked snapshot must acquire `db` before cloning it. Refuse the
        // bundle here so every live setter drops `db` and retries
        // asynchronously instead of retaining the remaining source stores.
        if self.db.snapshot_report().outstanding != 0 {
            return Err(LivePublicationWait::SalsaSnapshots);
        }
        let files = self
            .db_files
            .try_lock()
            .map_err(|_| LivePublicationWait::Files)?;
        let tombstones = self
            .db_tombstones
            .try_lock()
            .map_err(|_| LivePublicationWait::Tombstones)?;
        let project_members = self
            .db_project_members
            .try_lock()
            .map_err(|_| LivePublicationWait::ProjectMembers)?;
        let project = self
            .db_project
            .try_lock()
            .map_err(|_| LivePublicationWait::Project)?;
        let global_config = self
            .db_config
            .try_lock()
            .map_err(|_| LivePublicationWait::GlobalConfig)?;
        let folder_configs = self
            .folder_db_configs
            .try_lock()
            .map_err(|_| LivePublicationWait::FolderConfigs)?;
        Ok(LiveSourceLocks {
            db,
            files,
            tombstones,
            project_members,
            project,
            global_config,
            folder_configs,
        })
    }

    pub(super) fn try_open_publication_locks(
        &self,
    ) -> Result<OpenPublicationLocks<'_>, LivePublicationWait> {
        let source = self.try_live_source_locks()?;
        let index = self
            .workspace_index
            .try_write()
            .map_err(|_| LivePublicationWait::WorkspaceIndex)?;
        Ok(OpenPublicationLocks { source, index })
    }

    pub(super) fn try_reopen_publication_locks(
        &self,
    ) -> Result<ReopenPublicationLocks<'_>, LivePublicationWait> {
        let source = self.try_live_source_locks()?;
        let last_semantic_tokens = self
            .last_semantic_tokens
            .try_lock()
            .map_err(|_| LivePublicationWait::SemanticTokens)?;
        let semantic_tokens_refresh_asked = self
            .semantic_tokens_refresh_asked
            .try_lock()
            .map_err(|_| LivePublicationWait::SemanticRefreshRequests)?;
        let workspace_class_analyses = self
            .workspace_class_analyses
            .try_lock()
            .map_err(|_| LivePublicationWait::WorkspaceClassAnalyses)?;
        Ok(ReopenPublicationLocks {
            source,
            last_semantic_tokens,
            semantic_tokens_refresh_asked,
            workspace_class_analyses,
        })
    }

    /// Establish snapshot writer intent on the first non-empty census and keep
    /// it across transient `db` misses. A queued snapshot briefly owns `db`
    /// before it can observe the intent and back out; dropping the guard for
    /// that miss would wake the whole reader queue and replenish the census.
    /// Other dependency misses cannot be caused by a snapshot, so release the
    /// intent rather than blocking database readers behind unrelated stores.
    pub(super) fn update_snapshot_drain(
        &self,
        wait: LivePublicationWait,
        drain: &mut Option<SnapshotDrainGuard>,
    ) {
        match wait {
            LivePublicationWait::SalsaSnapshots => {
                drain.get_or_insert_with(|| self.db.begin_snapshot_drain());
            }
            LivePublicationWait::Database if drain.is_some() => {}
            _ => drop(drain.take()),
        }
    }

    pub(super) fn try_close_publication_locks(
        &self,
    ) -> Result<ClosePublicationLocks<'_>, LivePublicationWait> {
        let index = self
            .workspace_index
            .try_write()
            .map_err(|_| LivePublicationWait::WorkspaceIndex)?;
        let diag_slots = self
            .diag_slots
            .try_lock()
            .map_err(|_| LivePublicationWait::DiagnosticsSlots)?;
        let last_semantic_tokens = self
            .last_semantic_tokens
            .try_lock()
            .map_err(|_| LivePublicationWait::SemanticTokens)?;
        let semantic_tokens_refresh_asked = self
            .semantic_tokens_refresh_asked
            .try_lock()
            .map_err(|_| LivePublicationWait::SemanticRefreshRequests)?;
        let workspace_class_analyses = self
            .workspace_class_analyses
            .try_lock()
            .map_err(|_| LivePublicationWait::WorkspaceClassAnalyses)?;
        Ok(ClosePublicationLocks {
            index,
            diag_slots,
            last_semantic_tokens,
            semantic_tokens_refresh_asked,
            workspace_class_analyses,
        })
    }
}
