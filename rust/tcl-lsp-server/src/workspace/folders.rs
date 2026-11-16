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

//! The workspace folder set and what leaving a folder retires.

use super::source_files::uri_under_folder;
use super::{
    Arc, Backend, DiskPublicationInputs, DiskRemovalPolicy, DocumentSourceResult, FromStr,
    InitializeParams, Uri,
};

impl Backend {
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
    pub(in super::super) async fn apply_workspace_folders(&self, params: &InitializeParams) {
        let _inputs = self.analyser_inputs_gate.write().await;
        if let Some(folders) = &params.workspace_folders {
            let urls: Vec<Uri> = folders.iter().map(|f| f.uri.clone()).collect();
            *self.workspace_folders.lock().await = urls;
        } else if let Some(root) = &params.root_uri {
            *self.workspace_folders.lock().await = vec![root.clone()];
        }
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
    /// Disk IO and analysis deliberately run before taking any lock. The final
    /// publication optimistically acquires its complete write set, briefly
    /// try-locks `documents` to re-check that the file is still closed, and
    /// releases that map before the no-`await` commit; a newly reopened unsaved
    /// buffer still wins without background work pinning every handler.
    /// Remove from the cross-document index every entry whose URI sits under
    /// one of `folders` and is not currently open in the editor.  Used when a
    /// workspace folder is removed (`did_change_workspace_folders`) so its
    /// files stop contributing definitions / references / symbols.
    ///
    /// Documents the editor still has open are deliberately excluded
    /// throughout: their live buffer is the source of truth regardless of which
    /// folders the workspace currently holds, so neither their index entry nor
    /// any of their per-URI state is touched here.
    ///
    /// Every per-URI cache of the *closed* files goes through the shared
    /// [`Self::forget_uri_states`]: clearing the rehoming seeds by hand here
    /// would leave `last_semantic_tokens`, `semantic_tokens_refresh_asked` and
    /// `workspace_class_analyses` holding entries for files that are no longer
    /// in any workspace folder.
    pub(in super::super) async fn drop_index_under_folders(
        &self,
        folders: &[Uri],
    ) -> DocumentSourceResult<()> {
        if folders.is_empty() {
            return Ok(());
        }
        let folder_strs: Vec<String> = folders.iter().map(|u| u.as_str().to_owned()).collect();
        // Keep source-site reconciliation/query snapshots atomic with the
        // removal. Derive candidates under the gate, then let the shared
        // publication transaction re-check the open set without ever waiting
        // while it owns that map.
        let disk_publication = self.disk_publication_gate.lock().await;
        let mut removed_urls = Vec::new();
        loop {
            let rehoming_guard = self.rehoming_gate.lock().await;
            let candidates: Vec<Uri> = self
                .workspace_index
                .read()
                .await
                .document_uris()
                .into_iter()
                .filter(|uri| {
                    folder_strs
                        .iter()
                        .any(|folder| uri_under_folder(uri, folder))
                })
                .filter_map(|uri| Uri::from_str(&uri).ok())
                .collect();
            if candidates.is_empty() {
                break;
            }
            let mut admissions = Vec::with_capacity(candidates.len());
            for uri in &candidates {
                admissions.push(self.document_path_admission(uri).await?);
            }
            let (removed, rehoming_guard, _) = self
                .publish_admitted_disk_results_while_rehoming_locked(
                    &disk_publication,
                    rehoming_guard,
                    DiskPublicationInputs {
                        replacements: &[],
                        admissions: &[],
                        removals: &candidates,
                        removal_admissions: &admissions,
                        expected_analyser_inputs_epoch: None,
                        expected_class_factory_generation: None,
                    },
                    DiskRemovalPolicy::ClosedOnly,
                    "drop_index_under_folders",
                )
                .await?;
            let removed_admissions = removed
                .iter()
                .map(|uri| {
                    Arc::clone(
                        &admissions[candidates
                            .iter()
                            .position(|candidate| candidate == uri)
                            .expect("removed rows came from the retained candidates")],
                    )
                })
                .collect::<Vec<_>>();
            // The publication may release and reacquire its outer gates while
            // contended. Re-snapshot until no newly published closed entry
            // remains under the removed folders. Open buffers are preserved;
            // when only those remain, `removed` is empty and the loop settles.
            self.forget_uri_states(&removed, &removed_admissions)
                .await?;
            drop(rehoming_guard);
            if removed.is_empty() {
                break;
            }
            removed_urls.extend(removed);
        }
        // Clear the Problems / File-Explorer badge of any removed-folder file
        // that still carried one — it is no longer part of the workspace,
        // so a retained closed-file badge would be stale.
        let _: () = for uri in &removed_urls {
            if self.pull_diag_cache.lock().await.contains_key(uri) {
                self.clear_closed_diagnostics(uri).await;
            }
        };
        Ok(())
    }
}
