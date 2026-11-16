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

//! The bounded worker window a scan analyses files through, and the batches it publishes.

use super::disk_reindex::DiskScanOutcome;
use super::{
    Arc, Backend, DiskPublicationOutcome, DiskRemovalPolicy, DocumentSourceResult,
    DocumentSourceUnavailable, HashSet, PublishedWorkspaceScanBatch, ScannedDocument,
    WorkspaceScanWorker,
};

pub(super) enum WorkspaceScanCandidate {
    Open,
    Scanned(DiskScanOutcome),
}

/// A scan window bounds undrained outputs as well as running workers.
/// Completed task results can retain complete analyses until collected.
pub(super) struct WorkspaceScanTaskWindow<T> {
    pub(in super::super) tasks: crate::rt::JoinSet<T>,
    capacity: usize,
}

impl<T: Send + 'static> WorkspaceScanTaskWindow<T> {
    pub(in super::super) fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            tasks: crate::rt::JoinSet::new(),
            capacity,
        }
    }

    pub(super) fn has_capacity(&self) -> bool {
        self.tasks.len() < self.capacity
    }

    pub(in super::super) fn spawn<F>(&mut self, task: F)
    where
        F: std::future::Future<Output = T> + Send + 'static,
    {
        assert!(
            self.has_capacity(),
            "scan window must drain before refilling"
        );
        self.tasks.spawn(task);
    }

    pub(in super::super) async fn join_next(&mut self) -> Option<Result<T, crate::rt::JoinError>> {
        self.tasks.join_next().await
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod workspace_scan_task_window_controls {
    use super::WorkspaceScanTaskWindow;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct RetainedOutput {
        value: usize,
        live: Arc<AtomicUsize>,
        dropped: Arc<tokio::sync::Notify>,
    }

    impl RetainedOutput {
        fn new(value: usize, live: &Arc<AtomicUsize>, dropped: &Arc<tokio::sync::Notify>) -> Self {
            live.fetch_add(1, Ordering::SeqCst);
            Self {
                value,
                live: Arc::clone(live),
                dropped: Arc::clone(dropped),
            }
        }
    }

    impl Drop for RetainedOutput {
        fn drop(&mut self) {
            self.live.fetch_sub(1, Ordering::SeqCst);
            self.dropped.notify_one();
        }
    }

    #[tokio::test]
    async fn completed_scan_outputs_are_drained_before_refilling_the_window() {
        let live = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(tokio::sync::Notify::new());
        let mut window = WorkspaceScanTaskWindow::new(2);
        let mut pending = 0..9;
        let mut observed = Vec::new();
        loop {
            while window.has_capacity() {
                let Some(value) = pending.next() else { break };
                let output = RetainedOutput::new(value, &live, &dropped);
                window.spawn(async move { output });
                assert!(live.load(Ordering::SeqCst) <= 2);
            }
            tokio::task::yield_now().await;
            // Even completed results occupy the same bounded window.
            assert!(live.load(Ordering::SeqCst) <= 2);
            let Some(result) = window.join_next().await else {
                break;
            };
            let output = result.unwrap();
            observed.push(output.value);
            drop(output);
        }
        observed.sort_unstable();
        assert_eq!(observed, (0..9).collect::<Vec<_>>());
        assert_eq!(live.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn dropped_scan_window_releases_pending_and_completed_outputs() {
        let live = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(tokio::sync::Notify::new());
        let mut window = WorkspaceScanTaskWindow::new(2);
        let complete = RetainedOutput::new(1, &live, &dropped);
        window.spawn(async move { complete });
        let pending = RetainedOutput::new(2, &live, &dropped);
        window.spawn(async move {
            std::future::pending::<()>().await;
            pending
        });
        tokio::task::yield_now().await;
        assert!(!window.has_capacity());
        assert_eq!(live.load(Ordering::SeqCst), 2);
        drop(window);
        while live.load(Ordering::SeqCst) != 0 {
            dropped.notified().await;
        }
        assert_eq!(live.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn scan_window_preserves_worker_failure_and_other_outputs() {
        let mut window = WorkspaceScanTaskWindow::new(2);
        window.spawn(async { panic!("actual worker failure") });
        window.spawn(async { 7usize });
        let mut failures = 0;
        let mut values = Vec::new();
        while let Some(result) = window.join_next().await {
            match result {
                Ok(value) => values.push(value),
                Err(error) => {
                    assert!(error.is_panic());
                    failures += 1;
                }
            }
        }
        assert_eq!(values, vec![7]);
        assert_eq!(failures, 1);
    }
}

impl Backend {
    pub(super) fn enqueue_workspace_scan_candidate(
        tasks: &mut WorkspaceScanTaskWindow<WorkspaceScanCandidate>,
        worker: WorkspaceScanWorker,
    ) {
        let WorkspaceScanWorker {
            permit,
            open,
            uri,
            driver,
            store,
            path,
            scan_analyser_inputs_epoch,
            scan_class_factory_generation,
            document_dialect_overrides,
            folder_dialects,
            default_dialect,
        } = worker;
        tasks.spawn(async move {
            let _permit = permit;

            crate::rt::spawn_blocking(move || {
                if open.contains(&uri) {
                    return WorkspaceScanCandidate::Open;
                }
                if let Err(error) = driver.path_admission.check_for(&uri, store.as_ref()) {
                    return WorkspaceScanCandidate::Scanned(DiskScanOutcome::Unavailable(
                        error,
                    ));
                }
                // Analysis form, matching `read_document` / `scan_disk_file`.
                // Shared decoder, for the same reason `scan_disk_file`
                // uses it: an ill-formed byte must not silently remove the
                // file from the workspace index.
                let (raw, _) = match store.read_source_as(&path, driver.analyser.script_encoding) {
                    Ok(source) => source,
                    Err(cause)
                        if Self::source_read_is_absent(store.as_ref(), &path, &cause) =>
                    {
                        return match driver.path_admission.check_for(&uri, store.as_ref()) {
                            Ok(()) => WorkspaceScanCandidate::Scanned(DiskScanOutcome::Absent(
                                Arc::clone(&driver.path_admission),
                            )),
                            Err(error) => WorkspaceScanCandidate::Scanned(
                                DiskScanOutcome::Unavailable(error),
                            ),
                        };
                    }
                    Err(cause) => {
                        return WorkspaceScanCandidate::Scanned(DiskScanOutcome::Unavailable(
                            DocumentSourceUnavailable::Read {
                                uri,
                                cause: Arc::new(cause),
                            },
                        ));
                    }
                };
                let text = tcl_lexer::normalise_lone_cr(&raw).into_owned();
                if driver.analyser_inputs_epoch != scan_analyser_inputs_epoch
                    || driver.class_factory_generation != scan_class_factory_generation
                {
                    return WorkspaceScanCandidate::Scanned(DiskScanOutcome::Unavailable(
                        DocumentSourceUnavailable::ScanWorker {
                            cause: "workspace scan inputs changed before row analysis"
                                .to_owned(),
                        },
                    ));
                }
                let document_override = document_dialect_overrides
                    .get(uri.as_str())
                    .map(String::as_str);
                let dialect = Self::dialect_for_closed_sync(
                    &uri,
                    &text,
                    document_override,
                    &folder_dialects,
                    &default_dialect,
                );
                workspace_scan_trace!("candidate.analysis.begin path={path:?} bytes={} dialect={dialect}", text.len());
                match driver.analyse(&text, &dialect, None) {
                    Ok(seed) => {
                        workspace_scan_trace!("candidate.analysis.end path={path:?} commands={} procedures={} classes={}", seed.analysis.command_invocations.len(), seed.analysis.all_procs.len(), seed.analysis.all_classes.len());
                        WorkspaceScanCandidate::Scanned(DiskScanOutcome::Available(
                            Box::new(ScannedDocument {
                                uri,
                                text,
                                dialect,
                                expected_analyser_inputs_epoch: Some(scan_analyser_inputs_epoch),
                                expected_class_factory_generation: Some(
                                    scan_class_factory_generation,
                                ),
                                seed,
                            }),
                        ))
                    },
                    Err(error) => {
                        WorkspaceScanCandidate::Scanned(DiskScanOutcome::Unavailable(error))
                    }
                }
            })
            .await
            .unwrap_or_else(|error| Self::unavailable_scan_worker(&error))
        });
    }

    pub(super) fn append_workspace_scan_candidate(
        item: WorkspaceScanCandidate,
        batch: &mut Vec<ScannedDocument>,
        coverage_complete: &mut bool,
    ) -> bool {
        match item {
            WorkspaceScanCandidate::Open
            | WorkspaceScanCandidate::Scanned(DiskScanOutcome::Absent(_)) => return false,
            WorkspaceScanCandidate::Scanned(DiskScanOutcome::Unavailable(_error)) => {
                *coverage_complete = false;
                return false;
            }
            WorkspaceScanCandidate::Scanned(DiskScanOutcome::Available(item)) => {
                if item.seed.analysis.analysis_context_unavailable.is_some() {
                    // Retain the typed row, but do not let an analysis that
                    // performed no declaration walk certify absence.
                    *coverage_complete = false;
                }
                batch.push(*item);
            }
        }
        true
    }

    pub(super) async fn publish_workspace_scan_batch(
        &self,
        rows: Vec<ScannedDocument>,
        coverage_complete: &mut bool,
    ) -> DocumentSourceResult<PublishedWorkspaceScanBatch> {
        let scanned_count = rows.len();
        let batch = self.current_scanned_rows(rows).await?;
        *coverage_complete &= batch.rows.len() == scanned_count;
        let (published, currentness) = self
            .publish_scanned_disk_results(
                &batch,
                &[],
                DiskRemovalPolicy::ClosedOnly,
                "merge_workspace_scan_results",
            )
            .await?;
        *coverage_complete &= currentness != DiskPublicationOutcome::Unavailable;
        let published = published.into_iter().collect::<HashSet<_>>();
        let published_rows = batch
            .rows
            .iter()
            .filter(|row| published.contains(&row.0))
            .cloned()
            .collect::<Vec<_>>();
        Ok(PublishedWorkspaceScanBatch {
            batch,
            published,
            rows: published_rows,
        })
    }

    pub(super) fn unavailable_scan_worker(cause: &impl ToString) -> WorkspaceScanCandidate {
        WorkspaceScanCandidate::Scanned(DiskScanOutcome::Unavailable(
            DocumentSourceUnavailable::ScanWorker {
                cause: cause.to_string(),
            },
        ))
    }
}
