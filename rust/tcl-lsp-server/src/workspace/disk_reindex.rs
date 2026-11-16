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

//! Reading and reindexing closed files from disk.

use super::folder_scan::WORKSPACE_ANALYSIS_MAX_CONCURRENCY;
use super::{
    AdmittedDiskBatch, Arc, Backend, DiskPublicationOutcome, DiskRemovalPolicy,
    DocumentPathAdmission, DocumentSourceResult, DocumentSourceUnavailable, FreshAnalysisDriver,
    HashMap, ScanAnalysisInputs, ScannedDocument, Semaphore, Uri, vfs,
};

pub(in super::super) enum DiskScanOutcome {
    Available(Box<ScannedDocument>),
    Absent(Arc<DocumentPathAdmission>),
    Unavailable(DocumentSourceUnavailable),
}

/// Drive `futures` to completion with at most `concurrency` live at once,
/// collecting the `Some` results (`None` — an unreadable / no-longer-a-file
/// entry — is dropped).  Used by the `did_change_watched_files` batch reindex
/// which analyses its whole (bounded, per-event) file set in one
/// pass; [`Backend::scan_workspace_folders`] needs to merge completed
/// analyses into the index in batches rather than all at once (a large tree's
/// `AnalysisResult`s must not all be live in memory together), so it runs the
/// same acquire-before-spawn dispatch inline instead of reusing this return-a-
/// `Vec` shape. Mirrors [`Backend::spawn_workspace_warm`]'s permit pattern: a
/// permit is held for the caller's whole future, so `concurrency` bounds files
/// genuinely in flight, not merely tasks parked waiting for one.
pub(super) async fn run_bounded<R, F>(futures: Vec<F>, concurrency: usize) -> Vec<R>
where
    R: Send + 'static,
    F: std::future::Future<Output = Option<R>> + Send + 'static,
{
    let permits = Arc::new(Semaphore::new(concurrency.max(1)));
    let mut tasks = crate::rt::JoinSet::new();
    for fut in futures {
        let Ok(permit) = Arc::clone(&permits).acquire_owned().await else {
            break;
        };
        tasks.spawn(async move {
            let _permit = permit;
            fut.await
        });
    }
    let mut out = Vec::with_capacity(tasks.len());
    while let Some(res) = tasks.join_next().await {
        if let Ok(Some(item)) = res {
            out.push(item);
        }
    }
    out
}

impl Backend {
    pub(in super::super) async fn current_scanned_rows(
        &self,
        scanned: Vec<ScannedDocument>,
    ) -> DocumentSourceResult<AdmittedDiskBatch> {
        let mut rows = AdmittedDiskBatch::default();
        for document in scanned {
            let expected_epoch = document.seed.analyser_inputs_epoch;
            if document
                .expected_analyser_inputs_epoch
                .is_some_and(|prescribed| prescribed != expected_epoch)
                || rows
                    .expected_analyser_inputs_epoch
                    .is_some_and(|previous| previous != expected_epoch)
            {
                continue;
            }
            // A missing caller-wide prescription does not waive custody of
            // this actual worker's analyser inputs at the final commit.
            rows.expected_analyser_inputs_epoch = Some(expected_epoch);
            if rows
                .expected_class_factory_generation
                .is_some_and(|previous| previous != document.seed.class_factory_generation)
            {
                continue;
            }
            rows.expected_class_factory_generation = Some(document.seed.class_factory_generation);
            document
                .seed
                .path_admission
                .check_for(&document.uri, self.store.as_ref())?;
            if self.fresh_index_generation_is_current(&document.seed).await
                && document
                    .expected_analyser_inputs_epoch
                    .is_none_or(|expected_epoch| {
                        expected_epoch == document.seed.analyser_inputs_epoch
                            && expected_epoch == self.diag_inputs_epoch()
                    })
                && document
                    .expected_class_factory_generation
                    .is_none_or(|expected_generation| {
                        expected_generation == document.seed.class_factory_generation
                            && self
                                .class_factory_generation
                                .try_read()
                                .is_ok_and(|generation| *generation == expected_generation)
                    })
                && (document
                    .seed
                    .analysis
                    .analysis_context_unavailable
                    .is_some()
                    || tcl_compiler::source_graph::current_analysis(
                        &document.text,
                        &document.seed.analysis,
                    )
                    .is_some())
            {
                rows.admissions
                    .push(Arc::clone(&document.seed.path_admission));
                rows.rows.push(document.into_index_row());
            }
        }
        Ok(rows)
    }

    /// Read `uri` from disk, resolve its dialect and run a full uncached
    /// `Analyser::analyse`, all on one blocking-pool call. Returns `Absent`
    /// for a genuinely missing path or non-source directory. Other read errors
    /// remain unavailable. The retained absence receipt routes only its actual
    /// selected mapping through the
    /// tombstone retire path, exactly like a `DELETED` watched-file event.
    /// Shared by [`Self::reindex_index_from_disk_using`] (the single-file path used
    /// by `did_close` / `didRenameFiles`) and the watched-files batch
    /// reindex, so both agree on exactly how a disk-backed file is
    /// turned into an index/salsa entry.
    pub(in super::super) async fn scan_disk_file(
        store: Arc<dyn vfs::SourceStore>,
        uri: Uri,
        document_overrides: Arc<HashMap<String, String>>,
        folder_dialects: Arc<Vec<(Uri, String)>>,
        default_dialect: Arc<String>,
        driver: Box<FreshAnalysisDriver>,
    ) -> DiskScanOutcome {
        let original_uri = uri.clone();
        match crate::rt::spawn_blocking(move || {
            if let Err(error) = driver
                .path_admission
                .check_for(&uri, driver.source_store.as_ref())
            {
                return DiskScanOutcome::Unavailable(error);
            }
            let Some(path) = driver.path_admission.path() else {
                return DiskScanOutcome::Absent(Arc::clone(&driver.path_admission));
            };
            let (raw, _) = match store.read_source_as(path, driver.analyser.script_encoding) {
                Ok(source) => source,
                Err(cause) if Self::source_read_is_absent(store.as_ref(), path, &cause) => {
                    return match driver.path_admission.check_for(&uri, store.as_ref()) {
                        Ok(()) => DiskScanOutcome::Absent(Arc::clone(&driver.path_admission)),
                        Err(error) => DiskScanOutcome::Unavailable(error),
                    };
                }
                Err(cause) => {
                    return DiskScanOutcome::Unavailable(DocumentSourceUnavailable::Read {
                        uri,
                        cause: Arc::new(cause),
                    });
                }
            };
            let text = tcl_lexer::normalise_lone_cr(&raw).into_owned();
            let dialect = Self::dialect_for_closed_sync(
                &uri,
                &text,
                document_overrides.get(uri.as_str()).map(String::as_str),
                &folder_dialects,
                &default_dialect,
            );
            let seed = match driver.analyse(&text, &dialect, None) {
                Ok(seed) => seed,
                Err(error) => return DiskScanOutcome::Unavailable(error),
            };
            DiskScanOutcome::Available(Box::new(ScannedDocument {
                uri,
                text,
                dialect,
                expected_analyser_inputs_epoch: Some(seed.analyser_inputs_epoch),
                expected_class_factory_generation: Some(seed.class_factory_generation),
                seed,
            }))
        })
        .await
        {
            Ok(outcome) => outcome,
            Err(_) => DiskScanOutcome::Unavailable(DocumentSourceUnavailable::Worker {
                uri: original_uri,
            }),
        }
    }

    /// Batched counterpart of [`Self::reindex_index_from_disk`] for the
    /// `did_change_watched_files` CREATED/CHANGED set: reads and
    /// analyses every URI in `uris` across the bounded worker pool
    /// [`run_bounded`] drives (shared with `scan_workspace_folders`),
    /// then applies one atomic salsa/workspace-index publication instead of
    /// `uris.len()` sequential full analyses.
    ///
    /// Currency: `documents` is re-checked once, right before the batched
    /// apply — the same guard [`Self::reindex_index_from_disk`] uses (closed
    /// re-check under the lock) — so a `did_open` racing in
    /// for one of the URIs mid-batch still wins for that URI: its scanned
    /// copy is dropped rather than clobbering the live buffer, exactly as the
    /// single-file path behaves for that same race. An entry that reads as
    /// genuinely missing / no-longer-a-file (named by the event but gone by
    /// its read) is folded into the same batched retire call a
    /// `DELETED` entry uses, rather than a separate per-file `db_remove_source`.
    #[cfg(test)]
    pub(in super::super) async fn batch_reindex_from_disk(
        &self,
        uris: &[Uri],
    ) -> DocumentSourceResult<()> {
        self.batch_reindex_from_disk_using(uris, None).await
    }

    async fn closed_disk_scan_inputs(&self) -> ScanAnalysisInputs {
        let _inputs = self.analyser_inputs_gate.read().await;
        let class_factory_generation = *self.class_factory_generation.read().await;
        ScanAnalysisInputs {
            analyser_inputs_epoch: self.diag_inputs_epoch(),
            class_factory_generation,
            document_dialect_overrides: self.document_dialect_override_snapshot().await,
            folder_dialects: self.folder_dialect_overrides().await,
            default_dialect: self.session_dialect().await,
        }
    }

    pub(in super::super) async fn batch_reindex_from_disk_using(
        &self,
        uris: &[Uri],
        retained: Option<&HashMap<Uri, Arc<DocumentPathAdmission>>>,
    ) -> DocumentSourceResult<()> {
        if uris.is_empty() {
            return Ok(());
        }
        // Retain the actual ingress receipts across retries. A replacement
        // provider mapping is not a donor for this already-issued operation.
        let admissions = uris
            .iter()
            .map(|uri| {
                let admission = match retained {
                    Some(admissions) => Arc::clone(admissions.get(uri).ok_or_else(|| {
                        DocumentSourceUnavailable::Store {
                            uri: uri.clone(),
                            cause: vfs::SourceUriPathUnavailable::ProviderUnavailable,
                        }
                    })?),
                    None => Arc::new(DocumentPathAdmission::capture(uri, self.store.as_ref())?),
                };
                admission.check_for(uri, self.store.as_ref())?;
                Ok((uri.clone(), admission))
            })
            .collect::<DocumentSourceResult<Vec<_>>>()?;
        let concurrency =
            crate::rt::available_parallelism().min(WORKSPACE_ANALYSIS_MAX_CONCURRENCY);
        'attempt: for _ in 0..3 {
            let inputs = self.closed_disk_scan_inputs().await;
            let folders = Arc::new(inputs.folder_dialects);
            let default_dialect = Arc::new(inputs.default_dialect);
            let mut futures = Vec::with_capacity(admissions.len());
            for (uri, admission) in &admissions {
                admission.check_for(uri, self.store.as_ref())?;
                let driver = self.fresh_analysis_driver(Arc::clone(admission)).await;
                if driver.analyser_inputs_epoch != inputs.analyser_inputs_epoch
                    || driver.class_factory_generation != inputs.class_factory_generation
                {
                    continue 'attempt;
                }
                let uri = uri.clone();
                let store = Arc::clone(&self.store);
                let overrides = Arc::clone(&inputs.document_dialect_overrides);
                let folders = Arc::clone(&folders);
                let default_dialect = Arc::clone(&default_dialect);
                futures.push(async move {
                    let scanned = Self::scan_disk_file(
                        store,
                        uri.clone(),
                        overrides,
                        folders,
                        default_dialect,
                        driver,
                    )
                    .await;
                    Some((uri, scanned))
                });
            }
            let results = Box::pin(run_bounded(futures, concurrency)).await;
            let mut replacements = Vec::new();
            let mut removals = Vec::new();
            let mut removal_admissions = Vec::new();
            for (uri, scanned) in results {
                match scanned {
                    DiskScanOutcome::Available(document) => replacements.push(*document),
                    DiskScanOutcome::Absent(admission) => {
                        removals.push(uri);
                        removal_admissions.push(admission);
                    }
                    DiskScanOutcome::Unavailable(error) => return Err(error),
                }
            }
            let produced = replacements.len();
            let mut batch = self.current_scanned_rows(replacements).await?;
            if batch.rows.len() != produced {
                continue;
            }
            batch.removal_admissions = removal_admissions;
            batch.expected_analyser_inputs_epoch = Some(inputs.analyser_inputs_epoch);
            batch.expected_class_factory_generation = Some(inputs.class_factory_generation);
            let (_, outcome) = self
                .publish_scanned_disk_results(
                    &batch,
                    &removals,
                    DiskRemovalPolicy::ClosedOnly,
                    "batch_reindex_from_disk",
                )
                .await?;
            if outcome != DiskPublicationOutcome::Unavailable {
                return Ok(());
            }
        }
        Err(DocumentSourceUnavailable::ScanWorker {
            cause: "closed disk batch inputs did not settle after three attempts".to_owned(),
        })
    }

    #[cfg(test)]
    pub(in super::super) async fn reindex_index_from_disk(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<()> {
        self.reindex_index_from_disk_using(uri, None).await
    }

    pub(in super::super) async fn reindex_index_from_disk_using(
        &self,
        uri: &Uri,
        retained: Option<Arc<DocumentPathAdmission>>,
    ) -> DocumentSourceResult<()> {
        let admission = match retained {
            Some(admission) => admission,
            None => Arc::new(DocumentPathAdmission::capture(uri, self.store.as_ref())?),
        };
        for _ in 0..3 {
            admission.check_for(uri, self.store.as_ref())?;
            let inputs = self.closed_disk_scan_inputs().await;
            let driver = self.fresh_analysis_driver(Arc::clone(&admission)).await;
            if driver.analyser_inputs_epoch != inputs.analyser_inputs_epoch
                || driver.class_factory_generation != inputs.class_factory_generation
            {
                continue;
            }
            let scanned = Self::scan_disk_file(
                Arc::clone(&self.store),
                uri.clone(),
                inputs.document_dialect_overrides,
                Arc::new(inputs.folder_dialects),
                Arc::new(inputs.default_dialect),
                driver,
            )
            .await;
            let (batch, removals) = match scanned {
                DiskScanOutcome::Unavailable(error) => return Err(error),
                DiskScanOutcome::Available(scanned) => {
                    let batch = self.current_scanned_rows(vec![*scanned]).await?;
                    if batch.rows.len() != 1 {
                        continue;
                    }
                    (batch, Vec::new())
                }
                DiskScanOutcome::Absent(admission) => (
                    AdmittedDiskBatch {
                        removal_admissions: vec![admission],
                        expected_analyser_inputs_epoch: Some(inputs.analyser_inputs_epoch),
                        expected_class_factory_generation: Some(inputs.class_factory_generation),
                        ..Default::default()
                    },
                    vec![uri.clone()],
                ),
            };
            let (_, outcome) = self
                .publish_scanned_disk_results(
                    &batch,
                    &removals,
                    DiskRemovalPolicy::ClosedOnly,
                    "reindex_index_from_disk",
                )
                .await?;
            if outcome != DiskPublicationOutcome::Unavailable {
                return Ok(());
            }
        }
        Err(DocumentSourceUnavailable::ScanWorker {
            cause: format!(
                "closed disk inputs did not settle after three attempts for {}",
                uri.as_str()
            ),
        })
    }
}
