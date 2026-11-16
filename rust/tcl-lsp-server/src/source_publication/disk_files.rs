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

//! Publication of disk-backed analyses to the query database and the workspace index.

use super::super::documents::{DocumentPathAdmission, DocumentSourceResult, DocumentsGuard};
use super::super::{Backend, DocumentState, vfs};
use super::dependency_waits::LivePublicationWait;
use super::{DiskRemovalPolicy, PublicationLocks, PublishedSourcePaths};
use std::collections::HashMap;
use std::sync::Arc;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::workspace_index as core_workspace_index;
use tower_lsp_server::ls_types::Uri;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum DiskPublicationOutcome {
    Published,
    NoCandidates,
    SkippedOpen,
    Unavailable,
}

type AdmittedDiskReplacement<'a> = (
    &'a (Uri, String, String, AnalysisResult),
    &'a Arc<DocumentPathAdmission>,
);

#[derive(Clone, Copy)]
pub(in super::super) struct DiskPublicationInputs<'a> {
    pub(in super::super) replacements: &'a [(Uri, String, String, AnalysisResult)],
    pub(in super::super) admissions: &'a [Arc<DocumentPathAdmission>],
    pub(in super::super) removals: &'a [Uri],
    pub(in super::super) removal_admissions: &'a [Arc<DocumentPathAdmission>],
    pub(in super::super) expected_analyser_inputs_epoch: Option<u64>,
    pub(in super::super) expected_class_factory_generation: Option<u64>,
}

#[derive(Default)]
pub(in super::super) struct AdmittedDiskBatch {
    pub(in super::super) removal_admissions: Vec<Arc<DocumentPathAdmission>>,
    pub(in super::super) rows: Vec<(Uri, String, String, AnalysisResult)>,
    pub(in super::super) admissions: Vec<Arc<DocumentPathAdmission>>,
    pub(in super::super) expected_analyser_inputs_epoch: Option<u64>,
    pub(in super::super) expected_class_factory_generation: Option<u64>,
}

impl Backend {
    /// Atomically publish disk-backed analyses to salsa and the workspace
    /// index without letting background work become a foreground lock holder.
    ///
    /// The expensive file reads and analyses happen before this function. It
    /// then waits for the rehoming transaction gate while holding nothing, and
    /// repeatedly *tries* the entire downstream write set. Any contention
    /// drops the partial set before yielding, so document sync always gets
    /// priority. Once all downstream stores are available, `documents` is only
    /// try-locked for the closed-file currency check and is released before
    /// any salsa or index mutation.
    ///
    /// The final commit contains no `await`. It also begins only while the
    /// tracked salsa snapshot census is empty. Because the db mutex is already
    /// held at that check, no new snapshot can start between it and the salsa
    /// setters; `cancel_others` therefore cannot turn this bounded commit into
    /// a synchronous multi-second wait.
    #[cfg(test)]
    pub(in super::super) async fn publish_disk_results(
        &self,
        replacements: &[(Uri, String, String, AnalysisResult)],
        removals: &[Uri],
        removal_policy: DiskRemovalPolicy<'_>,
        site: &'static str,
    ) -> DocumentSourceResult<Vec<Uri>> {
        if replacements.is_empty() && removals.is_empty() {
            return Ok(Vec::new());
        }

        let disk_publication = self.disk_publication_gate.lock().await;
        let rehoming_guard = self.rehoming_gate.lock().await;
        let (changed, _, _) = self
            .publish_disk_results_while_rehoming_locked(
                &disk_publication,
                rehoming_guard,
                replacements,
                removals,
                removal_policy,
                site,
            )
            .await?;
        Ok(changed)
    }

    pub(in super::super) async fn publish_scanned_disk_results(
        &self,
        batch: &AdmittedDiskBatch,
        removals: &[Uri],
        removal_policy: DiskRemovalPolicy<'_>,
        site: &'static str,
    ) -> DocumentSourceResult<(Vec<Uri>, DiskPublicationOutcome)> {
        assert_eq!(batch.rows.len(), batch.admissions.len());
        for (row, admission) in batch.rows.iter().zip(&batch.admissions) {
            admission.check_for(&row.0, self.store.as_ref())?;
        }
        assert_eq!(removals.len(), batch.removal_admissions.len());
        for (uri, admission) in removals.iter().zip(&batch.removal_admissions) {
            admission.check_for(uri, self.store.as_ref())?;
        }
        if batch.rows.is_empty() && removals.is_empty() {
            return Ok((Vec::new(), DiskPublicationOutcome::NoCandidates));
        }
        let disk_publication = self.disk_publication_gate.lock().await;
        let rehoming = self.rehoming_gate.lock().await;
        let (changed, _, currentness) = self
            .publish_admitted_disk_results_while_rehoming_locked(
                &disk_publication,
                rehoming,
                DiskPublicationInputs {
                    replacements: &batch.rows,
                    admissions: &batch.admissions,
                    removals,
                    removal_admissions: &batch.removal_admissions,
                    expected_analyser_inputs_epoch: batch.expected_analyser_inputs_epoch,
                    expected_class_factory_generation: batch.expected_class_factory_generation,
                },
                removal_policy,
                site,
            )
            .await?;
        Ok((changed, currentness))
    }

    /// [`Self::publish_disk_results`] with the rehoming gate already held.
    ///
    /// The disk-only guard serialises captured inputs; the owned rehoming guard
    /// preserves the caller's transaction and can be released around a fair
    /// dependency wait, then returned for folder-removal cache cleanup.
    #[cfg(test)]
    async fn publish_disk_results_while_rehoming_locked<'a>(
        &'a self,
        disk_publication: &tokio::sync::MutexGuard<'_, ()>,
        rehoming_guard: tokio::sync::MutexGuard<'a, ()>,
        replacements: &[(Uri, String, String, AnalysisResult)],
        removals: &[Uri],
        removal_policy: DiskRemovalPolicy<'_>,
        site: &'static str,
    ) -> DocumentSourceResult<(
        Vec<Uri>,
        tokio::sync::MutexGuard<'a, ()>,
        DiskPublicationOutcome,
    )> {
        let admissions = replacements
            .iter()
            .map(|row| DocumentPathAdmission::capture(&row.0, self.store.as_ref()).map(Arc::new))
            .collect::<DocumentSourceResult<Vec<_>>>()?;
        let removal_admissions = removals
            .iter()
            .map(|uri| DocumentPathAdmission::capture(uri, self.store.as_ref()).map(Arc::new))
            .collect::<DocumentSourceResult<Vec<_>>>()?;
        self.publish_admitted_disk_results_while_rehoming_locked(
            disk_publication,
            rehoming_guard,
            DiskPublicationInputs {
                replacements,
                admissions: &admissions,
                removals,
                removal_admissions: &removal_admissions,
                expected_analyser_inputs_epoch: None,
                expected_class_factory_generation: None,
            },
            removal_policy,
            site,
        )
        .await
    }

    pub(in super::super) async fn publish_admitted_disk_results_while_rehoming_locked<'a>(
        &'a self,
        _disk_publication: &tokio::sync::MutexGuard<'_, ()>,
        rehoming_guard: tokio::sync::MutexGuard<'a, ()>,
        inputs: DiskPublicationInputs<'_>,
        removal_policy: DiskRemovalPolicy<'_>,
        site: &'static str,
    ) -> DocumentSourceResult<(
        Vec<Uri>,
        tokio::sync::MutexGuard<'a, ()>,
        DiskPublicationOutcome,
    )> {
        let DiskPublicationInputs {
            replacements,
            admissions,
            removals,
            removal_admissions,
            expected_analyser_inputs_epoch,
            expected_class_factory_generation,
        } = inputs;
        Self::check_disk_removal_admissions(removals, removal_admissions)?;
        // Serialise the final disk commit with live source publication. The
        // document-map check below can then be released before the bounded
        // Salsa setters without a didOpen/didChange source commit landing in
        // the check-to-mutation gap. A live mutation that becomes visible
        // while this waits either changes the removal classification below or
        // republishes after this transaction releases the gate.
        let mut rehoming_guard = rehoming_guard;
        let mut live_publication = self.live_publication_gate.lock().await;
        // Claim writer intent before the first optimistic bundle attempt.
        // Otherwise sustained readers queued on Tokio's fair `db` mutex can
        // prevent `try_lock` from ever reaching the census check below.
        let mut snapshot_drain = self.db.begin_snapshot_drain();
        loop {
            let (locks, docs) = match self.try_disk_publication_bundle(site) {
                Ok(bundle) => bundle,
                Err(wait) => {
                    (rehoming_guard, live_publication, snapshot_drain) = self
                        .restart_disk_publication(
                            rehoming_guard,
                            live_publication,
                            snapshot_drain,
                            site,
                            wait,
                        )
                        .await;
                    continue;
                }
            };
            let DiskPublicationSelection {
                had_replacements,
                replacements,
                index_only_removals,
                removals,
            } = Self::classify_disk_publication(
                replacements,
                admissions,
                removals,
                removal_policy,
                &docs,
            );
            // The crucial boundary: all potentially slow work is below, after
            // the global map is available to every handler again.
            drop(docs);

            // `try_read` is deliberate: these publication locks are already
            // held, so waiting for a config writer here could invert its lock
            // order. A writer in flight makes this closed batch unavailable.
            let _analyser_inputs_guard =
                if let Some(expected_epoch) = expected_analyser_inputs_epoch {
                    let Ok(guard) = self.analyser_inputs_gate.try_read() else {
                        return Ok(Self::unavailable_disk_publication(rehoming_guard));
                    };
                    if self.diag_inputs_epoch() != expected_epoch {
                        return Ok(Self::unavailable_disk_publication(rehoming_guard));
                    }
                    Some(guard)
                } else {
                    None
                };
            let _class_factory_guard =
                if let Some(expected_generation) = expected_class_factory_generation {
                    let Ok(guard) = self.class_factory_generation.try_read() else {
                        return Ok(Self::unavailable_disk_publication(rehoming_guard));
                    };
                    if *guard != expected_generation {
                        return Ok(Self::unavailable_disk_publication(rehoming_guard));
                    }
                    Some(guard)
                } else {
                    None
                };

            if replacements.is_empty() && removals.is_empty() && index_only_removals.is_empty() {
                let outcome = if had_replacements {
                    DiskPublicationOutcome::SkippedOpen
                } else {
                    DiskPublicationOutcome::NoCandidates
                };
                return Ok((Vec::new(), rehoming_guard, outcome));
            }

            Self::apply_disk_publication(
                locks,
                &removals,
                &index_only_removals,
                &replacements,
                removal_admissions,
                self.store.as_ref(),
                self.source_path_admissions.as_ref(),
            )?;
            drop(snapshot_drain);

            return Ok((
                Self::changed_disk_uris(
                    removals,
                    index_only_removals,
                    replacements.into_iter().map(|(row, _)| row).collect(),
                ),
                rehoming_guard,
                DiskPublicationOutcome::Published,
            ));
        }
    }

    /// Apply a fully-acquired disk publication bundle without awaiting.
    fn apply_disk_publication(
        locks: PublicationLocks<'_>,
        removals: &[&Uri],
        index_only_removals: &[&Uri],
        replacements: &[AdmittedDiskReplacement<'_>],
        removal_admissions: &[Arc<DocumentPathAdmission>],
        store: &dyn vfs::SourceStore,
        published_paths: &PublishedSourcePaths,
    ) -> DocumentSourceResult<()> {
        let admissions = replacements
            .iter()
            .map(|(_, admission)| admission.as_ref())
            .chain(removal_admissions.iter().map(Arc::as_ref))
            .collect::<Vec<_>>();
        let PublicationLocks {
            mut db,
            mut files,
            mut tombstones,
            mut project_members,
            mut project,
            global_config,
            folder_configs,
            mut index,
            mut seeds,
        } = locks;
        DocumentPathAdmission::with_current(&admissions, store, || {
            let removed_members = Self::remove_db_sources_locked(
                &mut db,
                &mut files,
                &mut tombstones,
                removals.iter().copied(),
                published_paths,
            );
            let added_members = Self::set_admitted_db_sources_locked(
                &mut db,
                &mut files,
                &mut tombstones,
                replacements
                    .iter()
                    .map(|(row, admission)| (admission.as_ref(), row.1.as_str(), row.2.as_str())),
                published_paths,
            );
            Self::sync_db_project_membership_with_configurations(
                &mut db,
                &files,
                &mut project_members,
                &mut project,
                (*global_config, &folder_configs),
                (
                    removals
                        .iter()
                        .copied()
                        .chain(index_only_removals.iter().copied()),
                    replacements.iter().map(|(row, _)| &row.0),
                    removed_members || added_members,
                ),
            );
            Self::sync_disk_index(
                &mut index,
                &mut seeds,
                removals
                    .iter()
                    .copied()
                    .chain(index_only_removals.iter().copied()),
                replacements.iter().map(|(row, _)| (&row.0, &row.3)),
            );
        })?;
        Ok(())
    }

    /// Apply one closed-file transaction to the workspace index and retire
    /// any source-site seeds that were built from its previous contents.
    fn sync_disk_index<'a>(
        index: &mut core_workspace_index::WorkspaceIndex,
        seeds: &mut HashMap<String, Vec<String>>,
        removals: impl IntoIterator<Item = &'a Uri>,
        replacements: impl IntoIterator<Item = (&'a Uri, &'a AnalysisResult)>,
    ) {
        for uri in removals {
            index.remove_document(uri.as_str());
            seeds.remove(uri.as_str());
        }
        for (uri, analysis) in replacements {
            index.replace_document(uri.as_str(), analysis);
            seeds.remove(uri.as_str());
        }
    }

    fn changed_disk_uris(
        removals: Vec<&Uri>,
        index_only_removals: Vec<&Uri>,
        replacements: Vec<&(Uri, String, String, AnalysisResult)>,
    ) -> Vec<Uri> {
        removals
            .into_iter()
            .chain(index_only_removals)
            .chain(replacements.into_iter().map(|entry| &entry.0))
            .cloned()
            .collect()
    }
}

struct DiskPublicationSelection<'a> {
    had_replacements: bool,
    replacements: Vec<AdmittedDiskReplacement<'a>>,
    index_only_removals: Vec<&'a Uri>,
    removals: Vec<&'a Uri>,
}

impl Backend {
    fn check_disk_removal_admissions(
        removals: &[Uri],
        removal_admissions: &[Arc<DocumentPathAdmission>],
    ) -> DocumentSourceResult<()> {
        assert_eq!(removals.len(), removal_admissions.len());
        for (uri, admission) in removals.iter().zip(removal_admissions) {
            admission.check_uri(uri)?;
        }
        Ok(())
    }

    fn classify_disk_publication<'a>(
        replacements: &'a [(Uri, String, String, AnalysisResult)],
        admissions: &'a [Arc<DocumentPathAdmission>],
        removals: &'a [Uri],
        removal_policy: DiskRemovalPolicy<'_>,
        docs: &HashMap<Uri, DocumentState>,
    ) -> DiskPublicationSelection<'a> {
        let had_replacements = !replacements.is_empty();
        let replacements: Vec<_> = replacements
            .iter()
            .zip(admissions)
            .filter(|(row, _)| !docs.contains_key(&row.0))
            .collect();
        let mut index_only_removals = Vec::new();
        let removals: Vec<_> = removals
            .iter()
            .filter(|uri| match removal_policy {
                DiskRemovalPolicy::RenamedPath => true,
                DiskRemovalPolicy::ClosedOnly => !docs.contains_key(*uri),
                DiskRemovalPolicy::IncludeOpen(deleted_revisions) => {
                    let Some(doc) = docs.get(*uri) else {
                        return true;
                    };
                    if !doc.backing_file_deleted {
                        return false;
                    }
                    if deleted_revisions.get(*uri) == Some(&doc.revision) {
                        true
                    } else {
                        // A post-deletion edit owns the current Salsa
                        // source, but the filesystem identity is still
                        // dead and must leave the cross-document index.
                        index_only_removals.push(*uri);
                        false
                    }
                }
            })
            .collect();
        DiskPublicationSelection {
            had_replacements,
            replacements,
            index_only_removals,
            removals,
        }
    }

    fn try_disk_publication_bundle(
        &self,
        site: &'static str,
    ) -> Result<(PublicationLocks<'_>, DocumentsGuard<'_>), LivePublicationWait> {
        let locks = self.try_publication_locks()?;
        // A salsa setter blocks the worker thread inside `cancel_others`
        // while any clone lives. Do not enter that synchronous wait from a
        // background transaction. Holding `db` makes this zero stable.
        if self.db.snapshot_report().outstanding != 0 {
            drop(locks);
            return Err(LivePublicationWait::SalsaSnapshots);
        }
        let Some(docs) = self.documents.try_lock(site) else {
            drop(locks);
            return Err(LivePublicationWait::Documents);
        };
        Ok((locks, docs))
    }

    fn unavailable_disk_publication(
        rehoming_guard: tokio::sync::MutexGuard<'_, ()>,
    ) -> (
        Vec<Uri>,
        tokio::sync::MutexGuard<'_, ()>,
        DiskPublicationOutcome,
    ) {
        (
            Vec::new(),
            rehoming_guard,
            DiskPublicationOutcome::Unavailable,
        )
    }
}
