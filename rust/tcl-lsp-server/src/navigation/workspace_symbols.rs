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

//! `workspace/symbol`: indexed hits and open documents the index does not hold yet.

use super::{
    Arc, Backend, Client, ControlFlow, CoreWorkspaceSymbolKind, DocumentPublication,
    DocumentSourceResult, DocumentSourceUnavailable, DocumentState, FromStr, HashMap, HashSet,
    Location, MessageType, Path, Position, Range, SymbolInformation, SymbolKind, Uri,
    WorkspaceSymbolResponse, core_workspace_index, core_workspace_symbols, new_workspace_index,
    readonly_reference_reports,
};

#[derive(Clone, Default)]
pub(in super::super) struct WorkspaceSymbolDiagnostic(Option<Arc<std::sync::Mutex<Vec<String>>>>);

impl WorkspaceSymbolDiagnostic {
    pub(in super::super) fn new() -> Self {
        if matches!(option_env!("TCL_LSP_TRACE_WORKSPACE_SYMBOLS"), Some("1")) {
            Self(Some(Arc::new(std::sync::Mutex::new(Vec::new()))))
        } else {
            Self::default()
        }
    }

    pub(in super::super) fn enabled(&self) -> bool {
        self.0.is_some()
    }

    pub(in super::super) fn record(&self, class: &'static str, counts: &[usize]) {
        let Some(events) = &self.0 else { return };
        let mut events = events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if events.len() < 32 {
            events.push(format!("{class}:{counts:?}"));
        }
    }

    pub(in super::super) fn check(&self, class: &'static str, admitted: bool) -> bool {
        if !admitted {
            self.record(class, &[]);
        }
        admitted
    }

    fn record_transaction<T>(
        &self,
        result: &DocumentSourceResult<readonly_reference_reports::ReadonlyTransaction<T>>,
        rows: usize,
    ) {
        self.record(
            match result {
                Ok(readonly_reference_reports::ReadonlyTransaction::Published(_)) => {
                    "lease.published"
                }
                Ok(readonly_reference_reports::ReadonlyTransaction::CurrencyRefused) => {
                    "lease.currency_refused"
                }
                Ok(readonly_reference_reports::ReadonlyTransaction::PublishRefused) => {
                    "lease.publish_refused"
                }
                Err(_) => "lease.error",
            },
            &[rows],
        );
    }

    pub(in super::super) async fn emit(&self, client: &Client, verdict: &'static str) {
        static ISSUED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let Some(events) = &self.0 else { return };
        let message = {
            let events = events
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            format!(
                "[workspace-symbol-trace] verdict={verdict} events={}",
                events.join(";")
            )
        };
        if ISSUED.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 32 {
            client.log_message(MessageType::LOG, message).await;
        }
    }
}

/// Symbols captured from one workspace-index snapshot, together with the exact
/// open-document snapshots their byte spans were analysed against. Keeping the
/// snapshots beside both fallback and indexed hits avoids mapping an old span
/// through newer live text after fallback analysis has awaited.
pub(in super::super) struct WorkspaceSymbolCandidates {
    pub(in super::super) fallback_hits: Vec<core_workspace_symbols::IndexedWorkspaceSymbol>,
    pub(in super::super) indexed_hits: Vec<core_workspace_symbols::IndexedWorkspaceSymbol>,
    indexed_revisions: HashMap<String, u64>,
    pub(in super::super) sources: HashMap<String, Option<DocumentState>>,
    pub(in super::super) analyses:
        HashMap<String, Option<readonly_reference_reports::WorkspaceSymbolSourceAnalysis>>,
}

struct WorkspaceSymbolRows {
    pub(in super::super) rows: Vec<readonly_reference_reports::WorkspaceSymbolRow>,
    pub(in super::super) analyses:
        HashMap<String, Option<readonly_reference_reports::WorkspaceSymbolSourceAnalysis>>,
    pub(in super::super) views:
        Vec<Option<readonly_reference_reports::source_views::WorkspaceSymbolSourceView>>,
}

struct OpenWorkspaceSymbolSnapshot {
    unindexed: Vec<(Uri, DocumentState)>,
    pub(in super::super) indexed_hits: Vec<core_workspace_symbols::IndexedWorkspaceSymbol>,
    indexed_revisions: HashMap<String, u64>,
    pub(in super::super) sources: HashMap<String, Option<DocumentState>>,
    closed_revisions: HashMap<String, u64>,
}

impl Backend {
    /// Load a closed document solely for workspace-symbol span mapping.
    ///
    /// No dialect/configuration lookup is needed: only the decoded analysis
    /// bytes and their line index are consumed. Keeping that unrelated work
    /// out also makes the index-revision revalidation below cover one bounded
    /// source read rather than an arbitrary configuration wait.
    pub(in super::super) async fn closed_workspace_symbol_source(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<Option<DocumentState>> {
        let admission = self.document_path_admission(uri).await?;
        let Some(path) = admission.path().map(Path::to_path_buf) else {
            return Ok(None);
        };
        let store = Arc::clone(&self.store);
        let selected_path = path.clone();
        let bytes = match crate::rt::spawn_blocking(move || store.read(&path)).await {
            Ok(Ok(source)) => source,
            Ok(Err(cause))
                if Self::source_read_is_absent(self.store.as_ref(), &selected_path, &cause) =>
            {
                admission.check_for(uri, self.store.as_ref())?;
                return Ok(None);
            }
            Ok(Err(cause)) => {
                return Err(DocumentSourceUnavailable::Read {
                    uri: uri.clone(),
                    cause: Arc::new(cause),
                });
            }
            Err(_) => return Err(DocumentSourceUnavailable::Worker { uri: uri.clone() }),
        };
        let (text, decode_report) =
            tcl_lsp_core::source_decode::decode_source_as(&bytes, self.declared_script_encoding());
        admission.check_for(uri, self.store.as_ref())?;
        let mut document = DocumentState::new(text, String::new()).with_path_admission(admission);
        document.decode_report = Some(decode_report);
        document.saved_source_bytes = Some(bytes.into());
        Ok(Some(document.normalised_for_analysis()))
    }

    /// The `workspace/symbol` matches contributed by **open** documents the
    /// workspace index does not currently hold.
    ///
    /// `did_open` drops the opened document's index entry — its on-disk
    /// records stop being authoritative the moment a live buffer exists — and
    /// the debounced diagnostics publish is what re-adds it.  Between the two
    /// the file is invisible to the picker, which is exactly the moment a user
    /// (or a test) searches for something they just opened.  A new
    /// untitled buffer has the same gap until its first publish.
    ///
    /// Bounded by the open-document set and empty in steady state. Each
    /// captured source retains one independently checked analysis/input owner
    /// for this request. A publication-pending open retains its fresh seed
    /// because Salsa may still memoise the pre-open disk source.
    ///
    /// A **cancelled** memo read must not be read as "this document has no
    /// symbols".  [`Self::cached_analysis`] answers `None` for two unrelated
    /// facts — "there is no salsa input for this URI" and "the read was
    /// cancelled" — and acting on both by silently dropping the document is
    /// wrong.  The second is not a fact about the document at
    /// all, and the write that causes it is the very publish this window
    /// exists because of: `did_open` schedules the debounced diagnostics run,
    /// that run's `sync_workspace_call_site_evidence` writes
    /// `SourceFile::external_call_sites` for the file just opened, and a
    /// salsa write unwinds every in-flight read.  Lose that race and the
    /// picker would intermittently answer *nothing* for the file the user had
    /// just opened.  So a URI that **has** an input falls back to analysis of its
    /// own buffer, which no concurrent write can cancel. Before live Salsa
    /// publication it retains the actual [`FreshAnalysisSeed`]. Afterwards
    /// the existing readonly source/input capture reissues supplied ownership;
    /// unavailable ownership cannot be read as semantic absence.
    ///
    /// An orphan contributes nothing, deliberately, even after a later edit
    /// restores the Salsa handle its local providers need. Resurrecting it
    /// from `documents` here would re-ghost every proc in the picker under the
    /// dead URI. A pending non-orphan is different: a brand-new or untitled
    /// buffer may simply not have reached its first Salsa publication, so its
    /// captured live text remains authoritative even without an input handle.
    #[cfg(test)]
    pub(in super::super) async fn open_but_unindexed_symbols(
        &self,
        query: &str,
        limit: usize,
    ) -> DocumentSourceResult<WorkspaceSymbolCandidates> {
        self.open_but_unindexed_symbols_diagnostic(
            query,
            limit,
            &WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    async fn open_but_unindexed_symbols_diagnostic(
        &self,
        query: &str,
        limit: usize,
        diagnostic: &WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<WorkspaceSymbolCandidates> {
        const MAX_CLOSED_SOURCE_SNAPSHOT_ATTEMPTS: usize = 3;
        for attempt in 0..MAX_CLOSED_SOURCE_SNAPSHOT_ATTEMPTS {
            let OpenWorkspaceSymbolSnapshot {
                unindexed,
                mut indexed_hits,
                mut indexed_revisions,
                mut sources,
                mut closed_revisions,
            } = {
                // `documents` → `workspace_index`, the lock order `did_open`
                // establishes.
                let docs = self.documents.lock("open_but_unindexed_symbols").await;
                let index = self.workspace_index.read().await;
                Self::capture_open_workspace_symbol_snapshot(&docs, &index, query, limit)
            };

            let mut pending = new_workspace_index();
            let mut fallback_sources = HashMap::new();
            let mut analyses = HashMap::new();
            for (uri, doc) in unindexed {
                // A published document with no input is a deliberately retired
                // orphan (see the doc comment), but a pending brand-new/untitled
                // buffer has not created its first Salsa handle yet and is still
                // authoritative for its own live symbols.
                if doc.publication != DocumentPublication::Pending
                    && !self.db_files.lock().await.contains_key(&uri)
                {
                    continue;
                }
                // Retain the checked source/input owner alongside fallback
                // symbols. Pending buffers keep their genuine fresh seed;
                // published database inputs retain the existing readonly owner.
                let Some(source_owner) = self
                    .capture_workspace_symbol_analysis_diagnostic(&uri, &doc, diagnostic)
                    .await?
                else {
                    continue;
                };
                pending.add_document(
                    uri.as_str(),
                    readonly_reference_reports::WorkspaceSymbolSourceAnalysis::analysis(
                        &source_owner,
                    ),
                );
                analyses.insert(uri.as_str().to_owned(), Some(source_owner));
                fallback_sources
                    .insert(uri.as_str().to_owned(), Some(doc.normalised_for_analysis()));
            }
            let fallback_hits = pending.symbols_matching(query, limit);
            diagnostic.record(
                "candidates",
                &[
                    fallback_hits.len(),
                    indexed_hits.len(),
                    sources.len(),
                    closed_revisions.len(),
                ],
            );

            // Fallback hits have priority in the final response. Do not read
            // closed sources for indexed candidates that cannot fit after
            // them: broad queries can otherwise turn one pending open buffer
            // into hundreds of serial disk reads that are immediately thrown
            // away by the caller.
            indexed_hits.truncate(limit.saturating_sub(fallback_hits.len()));
            let selected_indexed_uris: HashSet<String> =
                indexed_hits.iter().map(|hit| hit.uri.clone()).collect();
            sources.retain(|uri, _| selected_indexed_uris.contains(uri));
            closed_revisions.retain(|uri, _| selected_indexed_uris.contains(uri));
            indexed_revisions.retain(|uri, _| selected_indexed_uris.contains(uri));

            for uri_s in closed_revisions.keys() {
                let loaded = match Uri::from_str(uri_s) {
                    Ok(uri) => self.closed_workspace_symbol_source(&uri).await?,
                    Err(_) => None,
                };
                sources.insert(uri_s.clone(), loaded);
            }
            // A watcher may reindex a closed file while its bytes are being
            // read. Retry only when one of the documents whose hits we
            // captured changed; unrelated index churn must not starve the
            // picker. After the small bound, omit unstable closed documents
            // rather than ever map a stale byte span through mismatched text.
            let unstable: HashSet<String> = {
                let index = self.workspace_index.read().await;
                closed_revisions
                    .iter()
                    .filter(|(uri, revision)| index.document_revision(uri) != Some(**revision))
                    .map(|(uri, _)| uri.clone())
                    .collect()
            };
            if !unstable.is_empty() && attempt + 1 < MAX_CLOSED_SOURCE_SNAPSHOT_ATTEMPTS {
                continue;
            }
            if !unstable.is_empty() {
                indexed_hits.retain(|hit| !unstable.contains(&hit.uri));
                for uri in unstable {
                    sources.remove(&uri);
                }
            }
            sources.extend(fallback_sources);
            return Ok(WorkspaceSymbolCandidates {
                fallback_hits,
                indexed_hits,
                indexed_revisions,
                sources,
                analyses,
            });
        }
        unreachable!("the bounded source-snapshot loop always returns")
    }

    fn capture_open_workspace_symbol_snapshot(
        docs: &HashMap<Uri, DocumentState>,
        index: &core_workspace_index::WorkspaceIndex,
        query: &str,
        limit: usize,
    ) -> OpenWorkspaceSymbolSnapshot {
        let unindexed: Vec<(Uri, DocumentState)> = docs
            .iter()
            .filter(|(uri, doc)| {
                !doc.backing_file_deleted && !index.contains_document(uri.as_str())
            })
            .map(|(uri, doc)| (uri.clone(), doc.clone()))
            .collect();
        // Snapshot ordinary index hits under the same read lock that
        // classified fallback URIs. Fresh analysis below may await
        // long enough for one of those URIs to publish; consulting
        // the index a second time would mix two revisions.
        let indexed_hits = index.symbols_matching_excluding(
            query,
            limit,
            docs.iter()
                .filter_map(|(uri, doc)| doc.backing_file_deleted.then_some(uri.as_str())),
        );
        // An indexed hit names the bytes current at this index
        // snapshot. Open sources can be captured atomically under the
        // documents lock; closed sources need the per-document index
        // revision below to validate their out-of-lock disk read.
        let sources: HashMap<String, Option<DocumentState>> = indexed_hits
            .iter()
            .filter_map(|hit| {
                let uri = Uri::from_str(&hit.uri).ok()?;
                docs.get(&uri)
                    .cloned()
                    .map(|doc| (hit.uri.clone(), Some(doc.normalised_for_analysis())))
            })
            .collect();
        let indexed_revisions: HashMap<String, u64> = indexed_hits
            .iter()
            .filter_map(|hit| {
                index
                    .document_revision(&hit.uri)
                    .map(|revision| (hit.uri.clone(), revision))
            })
            .collect();
        let closed_revisions: HashMap<String, u64> = indexed_revisions
            .iter()
            .filter(|(uri, _)| !sources.contains_key(*uri))
            .map(|(uri, revision)| (uri.clone(), *revision))
            .collect();
        OpenWorkspaceSymbolSnapshot {
            unindexed,
            indexed_hits,
            indexed_revisions,
            sources,
            closed_revisions,
        }
    }
}

impl Backend {
    #[cfg(test)]
    pub(in super::super) async fn workspace_symbol_attempt(
        &self,
        query: &str,
    ) -> DocumentSourceResult<ControlFlow<Option<WorkspaceSymbolResponse>>> {
        self.workspace_symbol_attempt_diagnostic(
            query,
            &WorkspaceSymbolDiagnostic::default(),
            readonly_reference_reports::MovedIndexRows::Omit,
        )
        .await
    }

    pub(in super::super) async fn workspace_symbol_attempt_diagnostic(
        &self,
        query: &str,
        diagnostic: &WorkspaceSymbolDiagnostic,
        moved: readonly_reference_reports::MovedIndexRows,
    ) -> DocumentSourceResult<ControlFlow<Option<WorkspaceSymbolResponse>>> {
        let limit = core_workspace_symbols::MAX_WORKSPACE_SYMBOL_RESULTS;
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        // Open buffers the index does not currently hold, filled in from their
        // own analysis.  `did_open` drops a document's index entry (its
        // on-disk records stop being authoritative the moment a buffer exists)
        // and the debounced publish is what puts it back, so for that window
        // every symbol in the file the user *just opened* — the file most
        // likely to be searched for — would be missing from the picker.
        // Bounded by the open-document set, empty in steady state, and the
        // analysis read here is the memoised one the pending publish is
        // already computing.
        let candidates = self
            .open_but_unindexed_symbols_diagnostic(query, limit, diagnostic)
            .await?;
        let WorkspaceSymbolRows {
            rows: all,
            analyses,
            views: symbol_views,
        } = match self
            .map_workspace_symbol_candidates(candidates, limit, diagnostic)
            .await?
        {
            readonly_reference_reports::source_views::OriginalSourceViewAttempt::Ready(rows) => {
                rows
            }
            readonly_reference_reports::source_views::OriginalSourceViewAttempt::RetryCapture => {
                return Ok(ControlFlow::Continue(()));
            }
            readonly_reference_reports::source_views::OriginalSourceViewAttempt::Unavailable => {
                return Ok(ControlFlow::Break(None));
            }
        };
        // Reuse each actual checked URI input without weakening its final
        // source/configuration/provider/index currency. No await separates the
        // final joint owner checks from response projection.
        let result = self
            .workspace_symbol_response_transaction_with_views(
                &analyses,
                &all,
                &symbol_views,
                (inputs_epoch, factory_generation),
                diagnostic,
                moved,
            )
            .await;
        diagnostic.record_transaction(&result, all.len());
        result.map(readonly_reference_reports::ReadonlyTransaction::into_attempt)
    }

    async fn map_workspace_symbol_candidates(
        &self,
        candidates: WorkspaceSymbolCandidates,
        limit: usize,
        diagnostic: &WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<
        readonly_reference_reports::source_views::OriginalSourceViewAttempt<WorkspaceSymbolRows>,
    > {
        let WorkspaceSymbolCandidates {
            fallback_hits: mut hits,
            indexed_hits,
            indexed_revisions,
            mut sources,
            mut analyses,
        } = candidates;
        if hits.len() < limit {
            let remaining = limit - hits.len();
            hits.extend(indexed_hits.into_iter().take(remaining));
        }
        if hits.is_empty() {
            diagnostic.record("attempt.no_hits", &[]);
            return Ok(
                readonly_reference_reports::source_views::OriginalSourceViewAttempt::Unavailable,
            );
        }
        // A name span becomes an LSP range only against a checked source
        // snapshot for its own URI. The index retains immutable validation
        // context, not a mutable current buffer. `hits` arrives grouped by
        // URI, so each document is resolved once — from the open buffer when
        // there is one, else from disk.
        let mut all: Vec<(SymbolInformation, Uri, DocumentState, Option<u64>)> =
            Vec::with_capacity(hits.len());
        let mut symbol_views = Vec::with_capacity(hits.len());
        let mut view_cache =
            readonly_reference_reports::source_views::WorkspaceSymbolSourceViewCache::default();
        for hit in hits {
            self.capture_workspace_symbol_sources(
                &hit.uri,
                &mut sources,
                &mut analyses,
                diagnostic,
            )
            .await?;
            let (Some(Some(doc)), Ok(uri)) = (sources.get(&hit.uri), Uri::from_str(&hit.uri))
            else {
                diagnostic.record("attempt.no_source_snapshot", &[]);
                continue;
            };
            let Some(Some(current)) = analyses.get(&hit.uri) else {
                diagnostic.record("attempt.no_source_analysis", &[]);
                continue;
            };
            let view = match self.workspace_symbol_hit_source_view(
                &hit, doc, current, &indexed_revisions, &mut view_cache, diagnostic,
            ) {
                readonly_reference_reports::source_views::OriginalSourceViewAttempt::Ready(view) => view,
                readonly_reference_reports::source_views::OriginalSourceViewAttempt::RetryCapture => return Ok(readonly_reference_reports::source_views::OriginalSourceViewAttempt::RetryCapture),
                readonly_reference_reports::source_views::OriginalSourceViewAttempt::Unavailable => continue,
            };
            let symbol = Self::workspace_symbol_information(
                hit.name,
                hit.kind,
                hit.container_name,
                hit.name_span,
                &uri,
                doc,
            );
            symbol_views.push(view);
            all.push((
                symbol,
                uri.clone(),
                (*doc).clone(),
                indexed_revisions.get(&hit.uri).copied(),
            ));
        }
        if all.is_empty() {
            diagnostic.record("attempt.no_mapped_rows", &[]);
            return Ok(
                readonly_reference_reports::source_views::OriginalSourceViewAttempt::Unavailable,
            );
        }

        Ok(
            readonly_reference_reports::source_views::OriginalSourceViewAttempt::Ready(
                WorkspaceSymbolRows {
                    rows: all,
                    analyses,
                    views: symbol_views,
                },
            ),
        )
    }

    async fn capture_workspace_symbol_sources(
        &self,
        owner: &str,
        sources: &mut HashMap<String, Option<DocumentState>>,
        analyses: &mut HashMap<
            String,
            Option<readonly_reference_reports::WorkspaceSymbolSourceAnalysis>,
        >,
        diagnostic: &WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<()> {
        if !sources.contains_key(owner) {
            let loaded = match Uri::from_str(owner) {
                Ok(uri) => self.read_document(&uri).await?,
                Err(_) => None,
            };
            sources.insert(owner.to_owned(), loaded);
        }
        if !analyses.contains_key(owner)
            && let (Some(Some(doc)), Ok(uri)) = (sources.get(owner), Uri::from_str(owner))
        {
            let current = self
                .capture_workspace_symbol_analysis_diagnostic(&uri, doc, diagnostic)
                .await?;
            analyses.insert(owner.to_owned(), current);
        }
        Ok(())
    }

    pub(in super::super) fn workspace_symbol_hit_source_view(
        &self,
        hit: &core_workspace_symbols::IndexedWorkspaceSymbol,
        doc: &DocumentState,
        current: &readonly_reference_reports::WorkspaceSymbolSourceAnalysis,
        indexed_revisions: &HashMap<String, u64>,
        view_cache: &mut readonly_reference_reports::source_views::WorkspaceSymbolSourceViewCache,
        diagnostic: &WorkspaceSymbolDiagnostic,
    ) -> readonly_reference_reports::source_views::OriginalSourceViewAttempt<
        Option<readonly_reference_reports::source_views::WorkspaceSymbolSourceView>,
    > {
        use readonly_reference_reports::source_views::OriginalSourceViewAttempt;
        // Keep the existing exact source-fact route when it already owns the
        // captured physical analysis. No view is required or substituted there.
        if Self::workspace_symbol_hit_is_current(hit, doc, current, indexed_revisions) {
            return OriginalSourceViewAttempt::Ready(None);
        }
        if hit.original_location.is_some() && indexed_revisions.contains_key(&hit.uri) {
            match self.capture_workspace_symbol_source_view(hit, doc, view_cache) {
                OriginalSourceViewAttempt::Ready(Some(view)) => {
                    return OriginalSourceViewAttempt::Ready(Some(view));
                }
                OriginalSourceViewAttempt::Ready(None) => {}
                OriginalSourceViewAttempt::RetryCapture => {
                    return OriginalSourceViewAttempt::RetryCapture;
                }
                OriginalSourceViewAttempt::Unavailable => {
                    diagnostic.record("attempt.source_view_unavailable", &[]);
                    return OriginalSourceViewAttempt::Unavailable;
                }
            }
        }
        diagnostic.record(
            "attempt.hit_owner_refused",
            &[
                usize::from(hit.original_location.is_some()),
                usize::from(hit.logical_source.is_some()),
            ],
        );
        OriginalSourceViewAttempt::Unavailable
    }

    pub(in super::super) fn workspace_symbol_information(
        name: String,
        kind: CoreWorkspaceSymbolKind,
        container_name: Option<String>,
        name_span: tcl_lexer::Span,
        uri: &Uri,
        doc: &DocumentState,
    ) -> SymbolInformation {
        let start = doc
            .line_index
            .position_at_utf16(name_span.start(), &doc.text);
        let end = doc.line_index.position_at_utf16(name_span.end(), &doc.text);
        #[allow(deprecated)]
        SymbolInformation {
            name,
            kind: Self::indexed_workspace_symbol_kind(kind),
            tags: None,
            deprecated: None,
            location: Location {
                uri: uri.clone(),
                range: Range {
                    start: Position {
                        line: start.line,
                        character: start.character.get(),
                    },
                    end: Position {
                        line: end.line,
                        character: end.character.get(),
                    },
                },
            },
            container_name,
        }
    }

    pub(in super::super) fn workspace_symbol_hit_is_current(
        hit: &core_workspace_symbols::IndexedWorkspaceSymbol,
        doc: &DocumentState,
        current: &readonly_reference_reports::WorkspaceSymbolSourceAnalysis,
        indexed_revisions: &HashMap<String, u64>,
    ) -> bool {
        if let Some(owner) = hit.original_location.as_ref() {
            if owner.uri() != hit.uri
                || owner.validated_span(
                    &doc.text,
                    readonly_reference_reports::WorkspaceSymbolSourceAnalysis::analysis(current),
                ) != Some(hit.name_span)
            {
                return false;
            }
        } else {
            let Some(owner) = hit.logical_source.as_ref() else {
                // No latest-source span inverse: a logical report row
                // without its own retained source view is unavailable.
                return false;
            };
            if owner.uri() != hit.uri
                || owner.span() != hit.name_span
                || !owner.matches_source(&hit.uri, &doc.text)
                || indexed_revisions
                    .get(&hit.uri)
                    .is_some_and(|revision| owner.document_revision() != *revision)
            {
                return false;
            }
        }
        true
    }

    fn indexed_workspace_symbol_kind(kind: CoreWorkspaceSymbolKind) -> SymbolKind {
        match kind {
            // A tcltest case has no dedicated LSP kind; it lists as
            // a function alongside real functions.
            CoreWorkspaceSymbolKind::Function | CoreWorkspaceSymbolKind::Test => {
                SymbolKind::FUNCTION
            }
            CoreWorkspaceSymbolKind::Class => SymbolKind::CLASS,
            CoreWorkspaceSymbolKind::Method => SymbolKind::METHOD,
            CoreWorkspaceSymbolKind::Property => SymbolKind::PROPERTY,
            CoreWorkspaceSymbolKind::Constructor => SymbolKind::CONSTRUCTOR,
            CoreWorkspaceSymbolKind::Constant => SymbolKind::CONSTANT,
            CoreWorkspaceSymbolKind::Operator => SymbolKind::OPERATOR,
            CoreWorkspaceSymbolKind::Event => SymbolKind::EVENT,
        }
    }
}
