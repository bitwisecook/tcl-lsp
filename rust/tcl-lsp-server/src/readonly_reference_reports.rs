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

//! Reporting-only source reference locations and their independent coverage.

use super::{Backend, DocumentPathAdmission, DocumentSourceResult, LibraryPublicationSource};
use std::ops::ControlFlow;
use std::sync::Arc;
use tcl_compiler::analyser::{AnalysisResult, ResolvedAnalysisInput};
use tcl_lsp_core::original_declaration::{OriginalDeclarationIdentity, ReadonlyReferenceCoverage};
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};

struct OriginalReadonlyReferenceDocument {
    uri: Uri,
    source: Arc<str>,
    analysis: Arc<AnalysisResult>,
    file: tcl_lsp_db::SourceFile,
    config: tcl_lsp_db::AnalyserConfig,
    configuration: tcl_lsp_db::DiagnosticConfigurationSnapshot,
    input: Arc<ResolvedAnalysisInput>,
    external_call_sites: Option<Arc<tcl_compiler::unit_scope::CallSiteEvidence>>,
    admission: Arc<DocumentPathAdmission>,
    publication: LibraryPublicationSource,
    database_revision: salsa::Revision,
}

pub(super) struct ReadonlySourceDocument {
    uri: Uri,
    source: Arc<str>,
    file: tcl_lsp_db::SourceFile,
    config: tcl_lsp_db::AnalyserConfig,
    /// Configuration values read under the same database lock as `input`.
    configuration: tcl_lsp_db::DiagnosticConfigurationSnapshot,
    input: Arc<ResolvedAnalysisInput>,
    external_call_sites: Option<Arc<tcl_compiler::unit_scope::CallSiteEvidence>>,
    admission: Arc<DocumentPathAdmission>,
    publication: LibraryPublicationSource,
    database_revision: salsa::Revision,
}

/// One request's checked analysis for an actual workspace-symbol URI. A
/// missing database `SourceFile` is not missing source or missing declarations.
pub(super) struct WorkspaceSymbolSourceAnalysis {
    kind: WorkspaceSymbolAnalysisKind,
}

/// Coarse token syntax retains the same genuine fresh-source currency owner
/// as readonly reports; it never represents enriched semantic completion.
pub(super) struct CoarseSemanticTokenSource {
    analysis: WorkspaceSymbolSourceAnalysis,
    inputs_epoch: u64,
    factory_generation: u64,
}

impl CoarseSemanticTokenSource {
    fn from_fresh_analysis(analysis: WorkspaceSymbolSourceAnalysis) -> Option<Self> {
        let WorkspaceSymbolAnalysisKind::Fresh(document) = &analysis.kind else {
            return None;
        };
        if !matches!(
            document.purpose,
            FreshSourceAnalysisPurpose::SemanticStructure
        ) {
            return None;
        }
        let inputs_epoch = document.seed.analyser_inputs_epoch;
        let factory_generation = document.seed.class_factory_generation;
        Some(Self {
            analysis,
            inputs_epoch,
            factory_generation,
        })
    }

    /// Actual configured structure-only analysis; no enriched completion grant.
    pub(super) fn analysis(&self) -> &Arc<AnalysisResult> {
        match &self.analysis.kind {
            WorkspaceSymbolAnalysisKind::Fresh(document) => &document.seed.analysis,
            WorkspaceSymbolAnalysisKind::Database(document) => &document.analysis,
        }
    }
}

#[derive(Clone, Copy)]
enum FreshSourceAnalysisPurpose {
    ReadonlyComplete,
    SemanticStructure,
    ConditionalSourceRegion,
}

/// Currency for an ordinary source action; Core still owns action admissibility.
pub(super) struct OriginalCodeActionSource {
    uri: Uri,
    document: super::DocumentState,
    analysis: WorkspaceSymbolSourceAnalysis,
    inputs_epoch: u64,
    factory_generation: u64,
    index_generation: u64,
    exports: Arc<tcl_lsp_core::workspace_index::NamespaceExportSnapshot>,
}

impl OriginalCodeActionSource {
    pub(super) fn analysis(&self) -> Arc<AnalysisResult> {
        match &self.analysis.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => Arc::clone(&document.analysis),
            WorkspaceSymbolAnalysisKind::Fresh(document) => Arc::clone(&document.seed.analysis),
        }
    }

    pub(super) fn contains_range(&self, range: tower_lsp_server::ls_types::Range) -> bool {
        let exact = |position: tower_lsp_server::ls_types::Position| {
            let offset = self.document.line_index.offset_at_utf16(
                position.line,
                tcl_lexer::Utf16Col::new(position.character),
                &self.document.text,
            );
            let actual = self
                .document
                .line_index
                .position_at_utf16(offset, &self.document.text);
            actual.line == position.line && actual.character.get() == position.character
        };
        range.start <= range.end && exact(range.start) && exact(range.end)
    }

    pub(super) fn exports(&self) -> Arc<tcl_lsp_core::workspace_index::NamespaceExportSnapshot> {
        Arc::clone(&self.exports)
    }
}

type ReadonlySavedSources<'a> = std::borrow::Cow<'a, [(&'a DocumentPathAdmission, &'a [u8])]>;

struct ReadonlyPublicationInputs<'a> {
    admissions: std::borrow::Cow<'a, [&'a DocumentPathAdmission]>,
    saved_sources: Option<ReadonlySavedSources<'a>>,
}

pub(super) type WorkspaceSymbolRow = (
    tower_lsp_server::ls_types::SymbolInformation,
    Uri,
    super::DocumentState,
    Option<u64>,
);

enum WorkspaceSymbolAnalysisKind {
    Database(Box<OriginalReadonlyReferenceDocument>),
    Fresh(Box<WorkspaceSymbolFreshInput>),
}

struct WorkspaceSymbolFreshInput {
    purpose: FreshSourceAnalysisPurpose,
    uri: Uri,
    snapshot: super::DocumentState,
    was_open: bool,
    seed: super::FreshAnalysisSeed,
    input: Arc<ResolvedAnalysisInput>,
    config: tcl_lsp_db::AnalyserConfig,
    configuration: tcl_lsp_db::DiagnosticConfigurationSnapshot,
    database_revision: salsa::Revision,
    uri_generation: u64,
}

/// References into the actual held final publication transaction. This is
/// source/input currency only; it supplies no source semantics or absence.
struct ReadonlySourceState<'a> {
    open: &'a std::collections::HashMap<Uri, super::DocumentState>,
    db: &'a tcl_lsp_db::TclDatabase,
    files: &'a std::collections::HashMap<Uri, tcl_lsp_db::SourceFile>,
    published: &'a std::collections::HashMap<Uri, super::PublishedSourcePath>,
    uri_generations: &'a std::collections::HashMap<Uri, u64>,
}

struct WorkspaceSymbolPublication<'a> {
    analyses: &'a std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
    rows: &'a [WorkspaceSymbolRow],
    views: &'a [Option<source_views::WorkspaceSymbolSourceView>],
    epochs: (u64, u64),
    diagnostic: &'a super::WorkspaceSymbolDiagnostic,
    index_moved: std::sync::atomic::AtomicBool,
}

impl WorkspaceSymbolPublication<'_> {
    fn current_rows(
        &self,
        state: &ReadonlySourceState<'_>,
        index: &tcl_lsp_core::workspace_index::WorkspaceIndex,
        view_guards: &source_views::WorkspaceSymbolViewGuards<'_>,
    ) -> Vec<usize> {
        let owners = self
            .rows
            .iter()
            .map(|(_, uri, _, _)| uri.as_str())
            .collect();
        let current_inputs = Backend::workspace_symbol_current_inputs(
            &owners,
            self.analyses,
            state,
            self.epochs,
            self.diagnostic,
        );
        let mut current = Vec::with_capacity(self.rows.len());
        for (ordinal, (_, uri, snapshot, revision)) in self.rows.iter().enumerate() {
            if !current_inputs.contains(uri.as_str())
                || self
                    .analyses
                    .get(uri.as_str())
                    .and_then(Option::as_ref)
                    .is_none_or(|analysis| {
                        !WorkspaceSymbolSourceAnalysis::matches_snapshot(analysis, uri, snapshot)
                    })
            {
                continue;
            }
            if !workspace_symbol_index_snapshot_is_current(
                uri,
                snapshot,
                *revision,
                state.open.get(uri),
                index,
            ) {
                self.diagnostic.record("lease.index_snapshot_refused", &[]);
                self.index_moved
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                continue;
            }
            if self
                .views
                .get(ordinal)
                .and_then(Option::as_ref)
                .is_some_and(|view| {
                    !view_guards.matches(view, &self.rows[ordinal], state, index, self.epochs)
                })
            {
                self.diagnostic.record("lease.source_view_refused", &[]);
                continue;
            }
            current.push(ordinal);
        }
        current
    }

    fn source_inputs<'a>(
        &'a self,
        admissions: &[&'a DocumentPathAdmission],
        current: &[usize],
    ) -> ReadonlyPublicationInputs<'a> {
        let mut admissions = admissions.to_vec();
        let mut sources = super::DocumentState::saved_source_inputs(
            current.iter().map(|&ordinal| &self.rows[ordinal].2),
        );
        // Subtractive deduplication only: independent retained receipts stay
        // distinct even when their URI, path or byte values happen to agree.
        let mut selected_admissions = admissions
            .iter()
            .map(|admission| std::ptr::from_ref(*admission))
            .collect::<std::collections::HashSet<_>>();
        let mut selected_images = sources
            .iter()
            .map(|(admission, bytes)| (std::ptr::from_ref(*admission), bytes.as_ptr(), bytes.len()))
            .collect::<std::collections::HashSet<_>>();
        for view in current
            .iter()
            .filter_map(|&ordinal| self.views.get(ordinal)?.as_ref())
        {
            for admission in view.admissions() {
                if selected_admissions.insert(std::ptr::from_ref(admission)) {
                    admissions.push(admission);
                }
            }
            for (admission, bytes) in view.saved_sources() {
                if selected_images.insert((
                    std::ptr::from_ref(admission),
                    bytes.as_ptr(),
                    bytes.len(),
                )) {
                    sources.push((admission, bytes));
                }
            }
        }
        ReadonlyPublicationInputs {
            admissions: std::borrow::Cow::Owned(admissions),
            saved_sources: (!sources.is_empty()).then_some(std::borrow::Cow::Owned(sources)),
        }
    }
}

pub(super) struct OriginalReadonlyReferenceReport {
    pub(super) locations: Vec<Location>,
    pub(super) coverage: ReadonlyReferenceCoverage,
}

/// Whether a coverage claims a complete source occurrence census.
pub(super) const fn is_source_census(coverage: ReadonlyReferenceCoverage) -> bool {
    matches!(
        coverage,
        ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus
    )
}

/// Total captures of one readonly answer, the first included.
///
/// Bounded so a recapture loop can never spin: a further attempt runs only
/// after the final transaction found a lock, epoch, generation or captured
/// snapshot moved, and each attempt repeats the whole capture. Exhausting it
/// is safe rather than merely unlikely: the answer is withheld as
/// unavailable, never presented from a stale capture, and the client's next
/// request captures again.
pub(super) const READONLY_CURRENCY_ATTEMPTS: usize = 3;

/// What a symbol publication does with a row whose document was indexed
/// again after the row was captured.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MovedIndexRows {
    /// Refuse the attempt so the caller captures again.
    Recapture,
    /// Publish the rows that are still current.
    Omit,
}

/// Outcome of one final readonly transaction over captured sources.
pub(super) enum ReadonlyTransaction<T> {
    /// The captured sources were current and the answer was published.
    Published(T),
    /// A lock, epoch, generation or captured snapshot moved since capture.
    CurrencyRefused,
    /// The sources were current but the answer itself could not be issued.
    PublishRefused,
}

impl<T> ReadonlyTransaction<T> {
    pub(super) fn published(self) -> Option<T> {
        match self {
            Self::Published(value) => Some(value),
            Self::CurrencyRefused | Self::PublishRefused => None,
        }
    }

    /// Only a currency refusal asks for a fresh capture.
    pub(super) fn into_attempt(self) -> ControlFlow<Option<T>> {
        match self {
            Self::Published(value) => ControlFlow::Break(Some(value)),
            Self::PublishRefused => ControlFlow::Break(None),
            Self::CurrencyRefused => ControlFlow::Continue(()),
        }
    }

    /// A refused coarse token issue keeps its content-modified answer.
    pub(super) fn into_coarse_token_attempt(
        self,
    ) -> ControlFlow<tower_lsp_server::jsonrpc::Result<T>> {
        match self {
            Self::Published(value) => ControlFlow::Break(Ok(value)),
            Self::PublishRefused => {
                ControlFlow::Break(Err(super::coarse_semantic_source_changed()))
            }
            Self::CurrencyRefused => ControlFlow::Continue(()),
        }
    }
}

#[cfg(test)]
thread_local! {
    /// Unrelated Salsa input writes to land before the next final transactions.
    static UNRELATED_WRITES_BEFORE_FINAL: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// Run one whole capture, compute and final transaction, recapturing only
/// after a currency refusal. Exhausted attempts return `unavailable`.
pub(super) async fn retry_readonly_attempt<T, E, Attempt>(
    unavailable: T,
    mut attempt: impl FnMut() -> Attempt,
) -> Result<T, E>
where
    Attempt: std::future::Future<Output = Result<ControlFlow<T>, E>>,
{
    for _ in 0..READONLY_CURRENCY_ATTEMPTS {
        if let ControlFlow::Break(answer) = attempt().await? {
            return Ok(answer);
        }
    }
    Ok(unavailable)
}

/// A source occurrence census covers its own document only: any other
/// retained document, or an unavailable inventory, may hold references it
/// never inspected.
pub(super) const fn single_document_coverage(
    coverage: ReadonlyReferenceCoverage,
    retained_documents: usize,
) -> ReadonlyReferenceCoverage {
    match coverage {
        ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus if retained_documents != 1 => {
            ReadonlyReferenceCoverage::Partial
        }
        coverage => coverage,
    }
}

pub(super) const fn coverage_tag(coverage: ReadonlyReferenceCoverage) -> &'static str {
    match coverage {
        ReadonlyReferenceCoverage::ExistingInventory => "existing-inventory",
        ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus => "complete-source-census",
        ReadonlyReferenceCoverage::Partial => "partial",
        ReadonlyReferenceCoverage::Unavailable => "unavailable",
    }
}

impl OriginalReadonlyReferenceReport {
    pub(super) fn unavailable() -> Self {
        Self {
            locations: Vec::new(),
            coverage: ReadonlyReferenceCoverage::Unavailable,
        }
    }

    pub(super) fn lens_command(
        self,
        uri: &Uri,
        position: Position,
    ) -> tower_lsp_server::ls_types::Command {
        let title = tcl_lsp_core::code_lens::readonly_reference_count_title(
            self.locations.len(),
            self.coverage,
        );
        // A complete count of zero is an answer, and its empty peek a valid
        // target; an incomplete one with nothing found is not.
        let clickable = match self.coverage {
            ReadonlyReferenceCoverage::ExistingInventory
            | ReadonlyReferenceCoverage::CompleteSourceOccurrenceCensus => true,
            ReadonlyReferenceCoverage::Partial => !self.locations.is_empty(),
            ReadonlyReferenceCoverage::Unavailable => false,
        };
        tower_lsp_server::ls_types::Command {
            title,
            command: if clickable {
                "tcl-lsp.showReferences".to_owned()
            } else {
                String::new()
            },
            arguments: clickable.then(|| {
                vec![
                    serde_json::Value::String(uri.to_string()),
                    serde_json::to_value(position).unwrap_or(serde_json::Value::Null),
                    serde_json::to_value(self.locations).unwrap_or(serde_json::Value::Null),
                ]
            }),
        }
    }
}

// Actual host stamps are kept session-side, beside the CALL receipt in the
// existing bounded registry. Source equality cannot recreate these epochs.
#[derive(Clone, PartialEq, Eq)]
struct LogicalCallPublicationStamp {
    index_generation: u64,
    inputs_epoch: u64,
    factory_generation: u64,
    documents: Vec<LogicalCallDocumentStamp>,
}
#[derive(Clone, PartialEq, Eq)]
struct LogicalCallDocumentStamp {
    uri: Uri,
    source: Arc<str>,
    file: tcl_lsp_db::SourceFile,
    config: tcl_lsp_db::AnalyserConfig,
    input: Arc<ResolvedAnalysisInput>,
    admission: Arc<DocumentPathAdmission>,
    database_revision: salsa::Revision,
    open_revision: Option<u64>,
    uri_generation: u64,
    document_revision: u64,
    version: Option<i32>,
    raw_text: Option<Arc<str>>,
    dialect: String,
    dialect_resolution_generation: u64,
    resolved_dialect_generation: u64,
    index_revision: u64,
    index_context: tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext,
}
impl std::fmt::Debug for LogicalCallDocumentStamp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LogicalCallDocumentStamp")
            .field("uri", &self.uri)
            .field("database_revision", &self.database_revision)
            .field("document_revision", &self.document_revision)
            .field("index_revision", &self.index_revision)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for LogicalCallPublicationStamp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LogicalCallPublicationStamp")
            .field("index_generation", &self.index_generation)
            .field("inputs_epoch", &self.inputs_epoch)
            .field("factory_generation", &self.factory_generation)
            .field("document_count", &self.documents.len())
            .finish_non_exhaustive()
    }
}

impl LogicalCallPublicationStamp {
    fn capture(
        documents: &[OriginalReadonlyReferenceDocument],
        workspace: &tcl_lsp_core::workspace_index::WorkspaceIndex,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> Option<Self> {
        let mut stamps = documents
            .iter()
            .map(|document| {
                let index_context = workspace.diagnostic_source_context(document.uri.as_str())?;
                let expected =
                    tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(
                        &document.analysis,
                    )?;
                if index_context != &expected {
                    return None;
                }
                Some(LogicalCallDocumentStamp {
                    uri: document.uri.clone(),
                    source: document.source.clone(),
                    file: document.file,
                    config: document.config,
                    input: document.input.clone(),
                    admission: document.admission.clone(),
                    database_revision: document.database_revision,
                    open_revision: document.publication.open_revision,
                    uri_generation: document.publication.uri_generation,
                    document_revision: document.publication.document.revision,
                    version: document.publication.document.version,
                    raw_text: document.publication.document.raw_text.clone(),
                    dialect: document.publication.document.dialect.clone(),
                    dialect_resolution_generation: document
                        .publication
                        .document
                        .dialect_resolution_generation,
                    resolved_dialect_generation: document
                        .publication
                        .document
                        .resolved_dialect_generation,
                    index_revision: workspace.document_revision(document.uri.as_str())?,
                    index_context: index_context.clone(),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        stamps.sort_by(|a, b| a.uri.as_str().cmp(b.uri.as_str()));
        Some(Self {
            index_generation: workspace.generation(),
            inputs_epoch,
            factory_generation,
            documents: stamps,
        })
    }
}

struct LogicalCallProjection {
    items: Vec<tower_lsp_server::ls_types::CallHierarchyItem>,
    edges: Vec<(tower_lsp_server::ls_types::CallHierarchyItem, Vec<Range>)>,
}

fn logical_call_inventory(
    documents: &[OriginalReadonlyReferenceDocument],
) -> Vec<tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'_>> {
    documents
        .iter()
        .map(
            |document| tcl_lsp_core::original_declaration::OriginalDeclarationDocument {
                uri: document.uri.as_str(),
                source: &document.source,
                analysis: &document.analysis,
            },
        )
        .collect()
}

fn lift_logical_call_item(
    selected: &tcl_lsp_core::call_hierarchy::LogicalCallHierarchyItem,
    registry: &mut tcl_lsp_core::original_declaration::OriginalDeclarationRegistry,
    publication: &tcl_lsp_core::original_declaration::LogicalCallHostPublication,
) -> Option<tower_lsp_server::ls_types::CallHierarchyItem> {
    let identity = selected.identity();
    let handle = registry.issue_logical_call(identity.clone(), publication.clone())?;
    let item = selected.presentation();
    Some(tower_lsp_server::ls_types::CallHierarchyItem {
        name: item.name.clone(),
        kind: tower_lsp_server::ls_types::SymbolKind::FUNCTION,
        tags: None,
        detail: item.detail.clone(),
        uri: identity.uri().parse().ok()?,
        range: super::lift_lsp_range(item.range),
        selection_range: super::lift_lsp_range(item.selection_range),
        data: Some(serde_json::json!({ "logicalCallHierarchy": handle })),
    })
}

impl Backend {
    pub(super) async fn capture_readonly_source_document(
        &self,
        owner: &str,
        source: Arc<str>,
    ) -> DocumentSourceResult<Option<ReadonlySourceDocument>> {
        self.capture_readonly_source_document_diagnostic(
            owner,
            source,
            &super::WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    pub(super) async fn capture_readonly_source_document_diagnostic(
        &self,
        owner: &str,
        source: Arc<str>,
        diagnostic: &super::WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<Option<ReadonlySourceDocument>> {
        let Ok(uri) = owner.parse::<Uri>() else {
            diagnostic.record("source.uri_parse", &[]);
            return Ok(None);
        };
        let Some(document) = self.read_document(&uri).await? else {
            diagnostic.record("source.read_absent", &[]);
            return Ok(None);
        };
        if document.text != source {
            diagnostic.record("source.text_changed", &[]);
            return Ok(None);
        }
        let config = self.resolved_db_config(&uri).await;
        let Some(publication) = self.library_publication_source(&uri, document).await else {
            diagnostic.record("source.publication_unavailable", &[]);
            return Ok(None);
        };
        let (file, configuration, input, external_call_sites, admission, database_revision) = {
            let db = self.db.lock().await;
            let files = self.db_files.lock().await;
            let Some(&file) = files.get(&uri) else {
                diagnostic.record("source.no_db_file", &[]);
                return Ok(None);
            };
            let published = self
                .source_path_admissions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(retained) = published.get(&uri) else {
                diagnostic.record("source.no_retained_path", &[]);
                return Ok(None);
            };
            if retained.file != file
                || file.path(&*db).as_deref() != retained.admission.analyser_path()
                || file.text(&*db).as_str() != source.as_ref()
                || file.dialect(&*db) != &publication.document.dialect
                || publication
                    .document
                    .path_admission
                    .as_deref()
                    .is_some_and(|admission| admission != retained.admission.as_ref())
            {
                diagnostic.record("source.retained_publication_mismatch", &[]);
                return Ok(None);
            }
            let Ok(input) = tcl_lsp_db::document_analysis_input(&*db, file, config) else {
                diagnostic.record("source.analysis_input_unavailable", &[]);
                return Ok(None);
            };
            (
                file,
                config.diagnostic_report_configuration(&*db),
                input,
                file.external_call_sites(&*db).clone(),
                retained.admission.clone(),
                tcl_lsp_db::database_revision(&db),
            )
        };
        Ok(Some(ReadonlySourceDocument {
            uri,
            source,
            file,
            config,
            configuration,
            input,
            external_call_sites,
            admission,
            publication,
            database_revision,
        }))
    }

    async fn capture_readonly_reference_document(
        &self,
        owner: &str,
        source: Arc<str>,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceDocument>> {
        self.capture_readonly_reference_document_diagnostic(
            owner,
            source,
            &super::WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    async fn capture_readonly_reference_document_diagnostic(
        &self,
        owner: &str,
        source: Arc<str>,
        diagnostic: &super::WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceDocument>> {
        let Some(retained) = self
            .capture_readonly_source_document_diagnostic(owner, source, diagnostic)
            .await?
        else {
            diagnostic.record("metadata.no_source_owner", &[]);
            return Ok(None);
        };
        let ReadonlySourceDocument {
            uri,
            source,
            file,
            config,
            configuration,
            input,
            external_call_sites,
            admission,
            publication,
            database_revision,
        } = retained;
        // Reissue this report's metadata from the genuine retained provider;
        // a later equal-path publication cannot relabel an old analysis.
        let mut driver = self.fresh_analysis_driver(admission.clone()).await;
        driver.analyser = driver.analyser.with_resolved_input(input.as_ref().clone());
        let producer_source = source.clone();
        let dialect = publication.document.dialect.clone();
        let original_uri = uri.clone();
        let producer_input = Arc::clone(&input);
        let producer_evidence = external_call_sites.clone();
        let seed = crate::rt::spawn_blocking(move || {
            driver.analyse_readonly_document(
                &producer_source,
                &dialect,
                &producer_input,
                producer_evidence.as_deref(),
            )
        })
        .await
        .map_err(|_| super::DocumentSourceUnavailable::Worker { uri: original_uri })??;
        if seed.path_admission.as_ref() != admission.as_ref() {
            diagnostic.record("metadata.path_admission_changed", &[]);
            return Ok(None);
        }
        let current = OriginalReadonlyReferenceDocument {
            uri,
            source,
            analysis: seed.analysis,
            file,
            config,
            configuration,
            input,
            external_call_sites,
            admission,
            publication,
            database_revision,
        };
        let accepted = current.matches_original_input();
        if !accepted && diagnostic.enabled() {
            let analysis = &current.analysis;
            let input = &current.input;
            let config = input.lexer_config();
            diagnostic.record(
                "metadata_match",
                &[
                    usize::from(analysis.resolved_input.as_ref() == Some(input.as_ref())),
                    usize::from(analysis.analysis_context_unavailable.is_none()),
                    usize::from(analysis.body_lexer_config == Some(config)),
                    usize::from(analysis.matches_original_source_image(
                        &tcl_lexer::SourceImage::document(&current.source),
                        config,
                    )),
                    usize::from(
                        analysis
                            .retained_command_realm()
                            .is_some_and(|realm| realm.matches_resolved_analysis_input(input)),
                    ),
                ],
            );
        }
        Ok(accepted.then_some(current))
    }

    /// Preserve the source snapshot's own semantic input. Published database
    /// documents use the existing genuine readonly owner; Pending and genuine
    /// no-handle sources retain their actual fresh driver/seed instead.
    pub(super) async fn capture_workspace_symbol_analysis(
        &self,
        uri: &Uri,
        snapshot: &super::DocumentState,
    ) -> DocumentSourceResult<Option<WorkspaceSymbolSourceAnalysis>> {
        self.capture_workspace_symbol_analysis_diagnostic(
            uri,
            snapshot,
            &super::WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    pub(super) async fn capture_workspace_symbol_analysis_diagnostic(
        &self,
        uri: &Uri,
        snapshot: &super::DocumentState,
        diagnostic: &super::WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<Option<WorkspaceSymbolSourceAnalysis>> {
        let snapshot = if snapshot.dialect.is_empty() {
            let Some(current) = self.read_document(uri).await? else {
                diagnostic.record("capture.closed_read_absent", &[]);
                return Ok(None);
            };
            if current.clone().normalised_for_analysis().text != snapshot.text {
                diagnostic.record("capture.closed_text_changed", &[]);
                return Ok(None);
            }
            current.normalised_for_analysis()
        } else {
            snapshot.clone().normalised_for_analysis()
        };
        if snapshot.publication == super::DocumentPublication::Indexed
            && self.db_files.lock().await.contains_key(uri)
        {
            let Some(document) = self
                .capture_readonly_reference_document_diagnostic(
                    uri.as_str(),
                    Arc::clone(&snapshot.text),
                    diagnostic,
                )
                .await?
            else {
                diagnostic.record("capture.no_metadata_owner", &[]);
                return Ok(None);
            };
            let accepted = document.publication.document.raw() == snapshot.raw()
                && document.publication.document.dialect == snapshot.dialect;
            if !accepted {
                diagnostic.record("symbol_capture_document_snapshot", &[]);
            }
            return Ok(accepted.then_some(WorkspaceSymbolSourceAnalysis {
                kind: WorkspaceSymbolAnalysisKind::Database(Box::new(document)),
            }));
        }

        self.capture_fresh_source_analysis_diagnostic(
            uri,
            snapshot,
            FreshSourceAnalysisPurpose::ReadonlyComplete,
            diagnostic,
        )
        .await
    }

    // One genuine fresh-source capture/matcher for complete readonly analysis
    // and the independently selected coarse syntax-only worker purpose.
    async fn capture_fresh_source_analysis(
        &self,
        uri: &Uri,
        snapshot: super::DocumentState,
        purpose: FreshSourceAnalysisPurpose,
    ) -> DocumentSourceResult<Option<WorkspaceSymbolSourceAnalysis>> {
        self.capture_fresh_source_analysis_diagnostic(
            uri,
            snapshot,
            purpose,
            &super::WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    #[allow(clippy::too_many_lines)] // one report, step by step
    async fn capture_fresh_source_analysis_diagnostic(
        &self,
        uri: &Uri,
        snapshot: super::DocumentState,
        purpose: FreshSourceAnalysisPurpose,
        diagnostic: &super::WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<Option<WorkspaceSymbolSourceAnalysis>> {
        let config = self.resolved_db_config(uri).await;
        let (configuration, database_revision) = {
            let db = self.db.lock().await;
            (
                config.diagnostic_report_configuration(&*db),
                tcl_lsp_db::database_revision(&db),
            )
        };
        let was_open = self
            .documents
            .lock("workspace_symbol_source_capture")
            .await
            .contains_key(uri);
        if !was_open && matches!(purpose, FreshSourceAnalysisPurpose::SemanticStructure) {
            // A path receipt authenticates the selected provider address, not
            // unchanged closed-file bytes. This new publication purpose needs
            // the actual retained open-document source owner.
            diagnostic.record("fresh.structure_without_open", &[]);
            return Ok(None);
        }
        let uri_generation = self.live_publication_uri_generation(uri);
        let seed = match purpose {
            FreshSourceAnalysisPurpose::ConditionalSourceRegion => return Ok(None),
            FreshSourceAnalysisPurpose::ReadonlyComplete => {
                self.fresh_analysis_seed(uri, Arc::clone(&snapshot.text), snapshot.dialect.clone())
                    .await?
            }
            FreshSourceAnalysisPurpose::SemanticStructure => {
                let Some(admission) = snapshot.path_admission.clone() else {
                    diagnostic.record("fresh.structure_without_path", &[]);
                    return Ok(None);
                };

                admission.check_for(uri, self.store.as_ref())?;
                let sidecar_revision = self
                    .fresh_analysis_cache
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .revision();
                let mut driver = self
                    .fresh_analysis_driver(admission)
                    .await
                    .into_source_shell();
                // Every capture attempt of every coarse token request used to
                // re-analyse the same text; key it exactly as the seed cache.
                let key = super::fresh_analysis_cache::FreshAnalysisKey {
                    text: Arc::clone(&snapshot.text),
                    dialect: snapshot.dialect.clone(),
                    path_admission: Arc::clone(&driver.path_admission),
                    analyser_inputs_epoch: driver.analyser_inputs_epoch,
                    class_factory_generation: driver.class_factory_generation,
                    inputs: driver.inputs.clone(),
                    resolved_input: driver
                        .analyser
                        .prepare_analysis_input(&snapshot.dialect)
                        .ok(),
                };
                let cached = key.resolved_input.as_ref().and_then(|_| {
                    self.fresh_analysis_cache
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .get_structure(uri, &key)
                });
                if let Some(seed) = cached {
                    seed
                } else {
                    let text = Arc::clone(&snapshot.text);
                    let dialect = snapshot.dialect.clone();
                    let original_uri = uri.clone();
                    let seed =
                        crate::rt::spawn_blocking(move || driver.analyse(&text, &dialect, None))
                            .await
                            .map_err(|_| super::DocumentSourceUnavailable::Worker {
                                uri: original_uri,
                            })??;
                    let mut cache = self
                        .fresh_analysis_cache
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if key.resolved_input.is_some() && cache.is_current(&sidecar_revision) {
                        cache.insert_structure(uri, key, seed.clone());
                    }
                    seed
                }
            }
        };
        let Some(input) = seed.analysis.resolved_input.clone().map(Arc::new) else {
            diagnostic.record("fresh.no_resolved_input", &[]);
            return Ok(None);
        };
        let matches_input =
            original_readonly_analysis_matches_input(&snapshot.text, &seed.analysis, &input);
        if !matches_input {
            if diagnostic.enabled() {
                diagnostic.record(
                    "fresh_match",
                    &[
                        usize::from(seed.analysis.resolved_input.as_ref() == Some(input.as_ref())),
                        usize::from(seed.analysis.analysis_context_unavailable.is_none()),
                        usize::from(seed.analysis.body_lexer_config == Some(input.lexer_config())),
                        usize::from(seed.analysis.matches_original_source_image(
                            &tcl_lexer::SourceImage::document(&snapshot.text),
                            input.lexer_config(),
                        )),
                        usize::from(
                            seed.analysis
                                .retained_command_realm()
                                .is_some_and(|realm| realm.matches_resolved_analysis_input(&input)),
                        ),
                    ],
                );
            }
            diagnostic.record("fresh.source_input_mismatch", &[]);
            return Ok(None);
        }
        Ok(Some(WorkspaceSymbolSourceAnalysis {
            kind: WorkspaceSymbolAnalysisKind::Fresh(Box::new(WorkspaceSymbolFreshInput {
                purpose,
                uri: uri.clone(),
                snapshot,
                was_open,
                seed,
                input,
                config,
                configuration,
                database_revision,
                uri_generation,
            })),
        }))
    }

    /// Issue only from the real current open source and configured fresh worker.
    pub(super) async fn capture_coarse_semantic_source(
        &self,
        uri: &Uri,
        snapshot: &super::DocumentState,
    ) -> DocumentSourceResult<Option<CoarseSemanticTokenSource>> {
        if snapshot.dialect_resolution_generation != snapshot.resolved_dialect_generation {
            // A current buffer with unresolved resource dialect is not a
            // current configured analysis input, even when bytes are unchanged.
            return Ok(None);
        }
        let Some(analysis) = self
            .capture_fresh_source_analysis(
                uri,
                snapshot.clone().normalised_for_analysis(),
                FreshSourceAnalysisPurpose::SemanticStructure,
            )
            .await?
        else {
            return Ok(None);
        };

        let Some(source) = CoarseSemanticTokenSource::from_fresh_analysis(analysis) else {
            return Ok(None);
        };
        Ok(Some(source))
    }

    /// Same bounded readonly transaction, including original provider lease,
    /// encloses both coarse token cache mutation and response projection.
    #[cfg(test)]
    pub(super) async fn with_current_coarse_semantic_source<T>(
        &self,
        source: &CoarseSemanticTokenSource,
        publish: impl FnMut() -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        self.coarse_semantic_source_transaction(source, publish)
            .await
            .map(ReadonlyTransaction::published)
    }

    pub(super) async fn coarse_semantic_source_transaction<T>(
        &self,
        source: &CoarseSemanticTokenSource,
        mut publish: impl FnMut() -> Option<T>,
    ) -> DocumentSourceResult<ReadonlyTransaction<T>> {
        let admissions = [source.analysis.admission()];
        self.readonly_source_transaction(
            source.inputs_epoch,
            source.factory_generation,
            &admissions,
            None,
            |state| {
                WorkspaceSymbolSourceAnalysis::matches_current_state(
                    &source.analysis,
                    state,
                    source.inputs_epoch,
                    source.factory_generation,
                )
                .then_some(())
            },
            |_, ()| publish(),
        )
        .await
    }

    /// Retain a fresh source only after resolving its current dialect generation.
    /// Old tokens are discarded; the replacement analysis and final publication
    /// must authenticate the newly captured generations independently.
    pub(super) async fn coarse_source_recompute_snapshot(
        &self,
        source: &CoarseSemanticTokenSource,
    ) -> DocumentSourceResult<Option<super::DocumentState>> {
        let WorkspaceSymbolAnalysisKind::Fresh(document) = &source.analysis.kind else {
            return Ok(None);
        };
        self.resolve_dirty_open_document_dialect(&document.uri, "coarse_recompute_resolve")
            .await?;
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let admissions = [source.analysis.admission()];
        self.with_current_readonly_source_state(
            epoch,
            generation,
            &admissions,
            None,
            |state| {
                let current = state.open.get(&document.uri)?;
                if epoch == source.inputs_epoch
                    && generation == source.factory_generation
                    && document.open_snapshot_matches(current)
                {
                    return None;
                }
                (document.was_open
                    && document.recompute_snapshot_matches(current)
                    && document.uri_generation_matches(state)
                    && document.matches_input_state(
                        state,
                        source.inputs_epoch,
                        source.factory_generation,
                    ))
                .then(|| current.clone())
            },
            |_, current| Some(current.clone()),
        )
        .await
    }

    pub(super) async fn capture_code_action_source(
        &self,
        uri: &Uri,
        snapshot: &super::DocumentState,
    ) -> DocumentSourceResult<Option<OriginalCodeActionSource>> {
        if snapshot.version.is_none() {
            return Ok(None);
        }
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let Some(analysis) = self
            .capture_workspace_symbol_analysis(uri, snapshot)
            .await?
        else {
            return Ok(None);
        };
        let (exports, index_generation) = {
            let index = self.workspace_index.read().await;
            (index.export_snapshot(), index.generation())
        };
        Ok(Some(OriginalCodeActionSource {
            uri: uri.clone(),
            document: snapshot.clone(),
            analysis,
            inputs_epoch,
            factory_generation,
            index_generation,
            exports,
        }))
    }

    #[cfg(test)]
    pub(super) async fn finish_code_action_response(
        &self,
        source: &OriginalCodeActionSource,
        actions: Vec<tcl_lsp_core::code_actions::CodeAction>,
        only: Option<&Vec<tower_lsp_server::ls_types::CodeActionKind>>,
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::CodeActionOrCommand>>> {
        self.code_action_response_transaction(source, actions, only)
            .await
            .map(ReadonlyTransaction::published)
    }

    pub(super) async fn code_action_response_transaction(
        &self,
        source: &OriginalCodeActionSource,
        actions: Vec<tcl_lsp_core::code_actions::CodeAction>,
        only: Option<&Vec<tower_lsp_server::ls_types::CodeActionKind>>,
    ) -> DocumentSourceResult<
        ReadonlyTransaction<Vec<tower_lsp_server::ls_types::CodeActionOrCommand>>,
    > {
        let mut admissions = vec![source.analysis.admission()];
        if let Some(admission) = source.document.path_admission.as_deref() {
            if admission.uri() != &source.uri {
                return Ok(ReadonlyTransaction::PublishRefused);
            }
            admissions.push(admission);
        } else if self.store.workspace_source_inventory_provider()
            == super::vfs::WorkspaceSourceInventoryProvider::HostReported
            && source.uri.scheme().as_str() == "file"
        {
            return Ok(ReadonlyTransaction::PublishRefused);
        }
        // FnMut is called once by the shared transaction. Do not clone action
        // plans or mint a second source receipt on a refused publication.
        let mut actions = Some(actions);
        self.readonly_source_transaction(
            source.inputs_epoch,
            source.factory_generation,
            &admissions,
            None,
            |state| {
                let current = state.open.get(&source.uri)?;
                (code_action_document_matches(&source.document, current)
                    && source
                        .analysis
                        .matches_snapshot(&source.uri, &source.document)
                    && source.analysis.matches_current_state(
                        state,
                        source.inputs_epoch,
                        source.factory_generation,
                    ))
                .then_some(())
            },
            |_, ()| {
                let index = self.workspace_index.try_read().ok()?;
                if index.generation() != source.index_generation {
                    return None;
                }
                Some(super::lift_versioned_code_actions(
                    actions.take()?,
                    &source.uri,
                    source.document.version?,
                    source.document.raw(),
                    only,
                ))
            },
        )
        .await
    }

    /// Project once while the request's captured snapshot remains valid: the
    /// analyser-input epoch, factory generation, configuration and provider
    /// receipts it was analysed under, and each document revision it captured.
    /// A document whose client version advanced after capture keeps its
    /// captured answer (#1854); one whose captured revision was replaced in
    /// place, closed or deleted is withdrawn, and so is a closed document
    /// whose disk read or index revision changed. This retains possible
    /// symbol advice, not a complete workspace absence census.
    #[cfg(test)]
    pub(super) async fn finish_workspace_symbol_response(
        &self,
        analyses: &std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
        rows: &[WorkspaceSymbolRow],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<Option<tower_lsp_server::ls_types::WorkspaceSymbolResponse>> {
        self.workspace_symbol_response_transaction(analyses, rows, inputs_epoch, factory_generation)
            .await
            .map(ReadonlyTransaction::published)
    }

    #[cfg(test)]
    pub(super) async fn workspace_symbol_response_transaction(
        &self,
        analyses: &std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
        rows: &[WorkspaceSymbolRow],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<
        ReadonlyTransaction<tower_lsp_server::ls_types::WorkspaceSymbolResponse>,
    > {
        self.workspace_symbol_response_transaction_diagnostic(
            analyses,
            rows,
            inputs_epoch,
            factory_generation,
            &super::WorkspaceSymbolDiagnostic::default(),
        )
        .await
    }

    #[cfg(test)]
    pub(super) async fn workspace_symbol_response_transaction_diagnostic(
        &self,
        analyses: &std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
        rows: &[WorkspaceSymbolRow],
        inputs_epoch: u64,
        factory_generation: u64,
        diagnostic: &super::WorkspaceSymbolDiagnostic,
    ) -> DocumentSourceResult<
        ReadonlyTransaction<tower_lsp_server::ls_types::WorkspaceSymbolResponse>,
    > {
        self.workspace_symbol_response_transaction_with_views(
            analyses,
            rows,
            &[],
            (inputs_epoch, factory_generation),
            diagnostic,
            MovedIndexRows::Omit,
        )
        .await
    }

    pub(super) async fn workspace_symbol_response_transaction_with_views(
        &self,
        analyses: &std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
        rows: &[WorkspaceSymbolRow],
        views: &[Option<source_views::WorkspaceSymbolSourceView>],
        epochs: (u64, u64),
        diagnostic: &super::WorkspaceSymbolDiagnostic,
        moved: MovedIndexRows,
    ) -> DocumentSourceResult<
        ReadonlyTransaction<tower_lsp_server::ls_types::WorkspaceSymbolResponse>,
    > {
        if rows.is_empty() {
            diagnostic.record("lease.publish_guard.1", &[]);
            return Ok(ReadonlyTransaction::PublishRefused);
        }
        let owners = rows
            .iter()
            .map(|(_, uri, _, _)| uri.as_str())
            .collect::<std::collections::HashSet<_>>();
        let Some(selected) = owners
            .iter()
            .map(|uri| analyses.get(*uri)?.as_ref())
            .collect::<Option<Vec<_>>>()
        else {
            diagnostic.record("lease.publish_guard.2", &[]);
            return Ok(ReadonlyTransaction::PublishRefused);
        };
        if !rows.iter().any(|(_, uri, snapshot, _)| {
            analyses
                .get(uri.as_str())
                .and_then(Option::as_ref)
                .is_some_and(|analysis| {
                    WorkspaceSymbolSourceAnalysis::matches_snapshot(analysis, uri, snapshot)
                })
        }) {
            diagnostic.record("lease.publish_guard.3", &[]);
            return Ok(ReadonlyTransaction::PublishRefused);
        }
        let Some(admissions) = self.workspace_symbol_source_admissions(&selected, rows, diagnostic)
        else {
            return Ok(ReadonlyTransaction::PublishRefused);
        };
        if !views.is_empty() && views.len() != rows.len() {
            return Ok(ReadonlyTransaction::PublishRefused);
        }
        let publication = WorkspaceSymbolPublication {
            analyses,
            rows,
            views,
            epochs,
            diagnostic,
            index_moved: std::sync::atomic::AtomicBool::new(false),
        };
        self.readonly_source_transaction_with_inputs(
            epochs,
            |(_, _, current): &(_, _, Vec<usize>)| publication.source_inputs(&admissions, current),
            diagnostic,
            |state| {
                // Keep view/table ownership before the existing index guard,
                // through one combined provider lease and response projection.
                let view_guards = self.workspace_symbol_view_guards(views)?;
                let Ok(index) = self.workspace_index.try_read() else {
                    diagnostic.record("lease.index_busy", &[]);
                    return None;
                };
                let current = publication.current_rows(state, &index, &view_guards);
                if moved == MovedIndexRows::Recapture
                    && publication
                        .index_moved
                        .load(std::sync::atomic::Ordering::Relaxed)
                {
                    return None;
                }
                if current.is_empty() {
                    diagnostic.record("lease.no_current_rows", &[]);
                }
                (!current.is_empty()).then_some((view_guards, index, current))
            },
            |_, (_, _, current)| {
                Some(tower_lsp_server::ls_types::WorkspaceSymbolResponse::Flat(
                    current
                        .iter()
                        .map(|&ordinal| rows[ordinal].0.clone())
                        .collect(),
                ))
            },
        )
        .await
    }

    fn workspace_symbol_current_inputs<'a>(
        owners: &std::collections::HashSet<&'a str>,
        analyses: &std::collections::HashMap<String, Option<WorkspaceSymbolSourceAnalysis>>,
        state: &ReadonlySourceState<'_>,
        epochs: (u64, u64),
        diagnostic: &super::WorkspaceSymbolDiagnostic,
    ) -> std::collections::HashSet<&'a str> {
        let (inputs_epoch, factory_generation) = epochs;
        let current_inputs = owners
            .iter()
            .copied()
            .filter(|uri| {
                analyses
                    .get(*uri)
                    .and_then(Option::as_ref)
                    .is_some_and(|analysis| {
                        WorkspaceSymbolSourceAnalysis::matches_request_snapshot(
                            analysis,
                            state,
                            inputs_epoch,
                            factory_generation,
                            diagnostic,
                        )
                    })
            })
            .collect::<std::collections::HashSet<_>>();
        diagnostic.record(
            "lease.inputs_current",
            &[current_inputs.len(), owners.len()],
        );
        current_inputs
    }

    fn workspace_symbol_source_admissions<'a>(
        &self,
        selected: &[&'a WorkspaceSymbolSourceAnalysis],
        rows: &'a [WorkspaceSymbolRow],
        diagnostic: &super::WorkspaceSymbolDiagnostic,
    ) -> Option<Vec<&'a DocumentPathAdmission>> {
        let mut admissions = selected
            .iter()
            .map(|analysis| WorkspaceSymbolSourceAnalysis::admission(analysis))
            .collect::<Vec<_>>();
        // The analysis seed cannot renew an independently captured source-read
        // mapping. Retain both actual receipts through the same final lease;
        // equal URI/path/text never substitutes for the original allocation.
        for (_, uri, snapshot, _) in rows {
            if let Some(admission) = snapshot.path_admission.as_deref() {
                if admission.uri() != uri {
                    diagnostic.record("lease.publish_guard.4", &[]);
                    return None;
                }
                admissions.push(admission);
            } else if self.store.workspace_source_inventory_provider()
                == super::vfs::WorkspaceSourceInventoryProvider::HostReported
                && uri.scheme().as_str() == "file"
            {
                // Preserve the original host-file source-receipt refusal.
                // Native and pathless/in-memory purposes remain independent.
                diagnostic.record("lease.publish_guard.5", &[]);
                return None;
            }
        }
        Some(admissions)
    }

    async fn finish_readonly_reference_report(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        targets: Vec<(String, tcl_lexer::Span)>,
        coverage: ReadonlyReferenceCoverage,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<ReadonlyTransaction<OriginalReadonlyReferenceReport>> {
        self.readonly_documents_transaction(documents, inputs_epoch, factory_generation, || {
            lift_readonly_reference_targets(&targets, documents).map(|locations| {
                OriginalReadonlyReferenceReport {
                    locations,
                    coverage,
                }
            })
        })
        .await
    }

    pub(super) async fn publish_factory_source_if_current<T>(
        &self,
        documents: &[ReadonlySourceDocument],
        inputs_epoch: u64,
        factory_generation: u64,
        revision: salsa::Revision,
        caller: (&Uri, &super::DocumentState, u64),
        value: T,
    ) -> DocumentSourceResult<Option<T>> {
        let (caller_uri, caller, caller_generation) = caller;
        let selected = documents
            .iter()
            .find(|document| &document.uri == caller_uri);
        let Some(selected) = selected else {
            return Ok(None);
        };
        let selected_caller = &selected.publication.document;
        if selected_caller.text != caller.text
            || selected_caller.raw() != caller.raw()
            || selected_caller.dialect != caller.dialect
            || selected_caller.revision != caller.revision
            || selected_caller.path_admission != caller.path_admission
            || selected_caller.saved_source_bytes.is_some() != caller.saved_source_bytes.is_some()
            || selected.publication.open_revision.is_none() != caller.saved_source_bytes.is_some()
        {
            return Ok(None);
        }
        let mut admissions = Vec::new();
        let mut saved_documents = Vec::new();
        if let Some(admission) = caller.path_admission.as_deref() {
            if admission.uri() != caller_uri {
                return Ok(None);
            }
            admissions.push(admission);
        }
        if caller.saved_source_bytes.is_some() {
            if caller.path_admission.is_none() {
                return Ok(None);
            }
            saved_documents.push(caller);
        }
        for document in documents {
            admissions.push(document.admission.as_ref());
            let source = &document.publication.document;
            if let Some(admission) = source.path_admission.as_deref() {
                if admission.uri() != &document.uri {
                    return Ok(None);
                }
                admissions.push(admission);
            }
            if document.publication.open_revision.is_none() {
                if source.path_admission.is_none() || source.saved_source_bytes.is_none() {
                    return Ok(None);
                }
                saved_documents.push(source);
            }
        }
        let saved_sources = super::DocumentState::saved_source_inputs(saved_documents);
        let mut value = Some(value);
        self.readonly_source_transaction(
            inputs_epoch,
            factory_generation,
            &admissions,
            (!saved_sources.is_empty()).then_some(saved_sources.as_slice()),
            |state| {
                (tcl_lsp_db::database_revision(state.db) == revision
                    && state
                        .uri_generations
                        .get(caller_uri)
                        .copied()
                        .unwrap_or_default()
                        == caller_generation
                    && state.open.get(caller_uri).is_none_or(|current| {
                        current.revision == caller.revision
                            && current.raw() == caller.raw()
                            && current.dialect == caller.dialect
                    })
                    && documents.iter().all(|document| {
                        document.matches_current_snapshot(
                            state.uri_generations,
                            state.open,
                            state.db,
                            state.files,
                            state.published,
                        )
                    }))
                .then_some(())
            },
            |_, ()| value.take(),
        )
        .await
        .map(ReadonlyTransaction::published)
    }

    async fn readonly_documents_transaction<T>(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        inputs_epoch: u64,
        factory_generation: u64,
        mut publish: impl FnMut() -> Option<T>,
    ) -> DocumentSourceResult<ReadonlyTransaction<T>> {
        let admissions = documents
            .iter()
            .map(|document| document.admission.as_ref())
            .collect::<Vec<_>>();
        self.readonly_source_transaction(
            inputs_epoch,
            factory_generation,
            &admissions,
            None,
            |state| {
                documents
                    .iter()
                    .all(|document| {
                        document.matches_current_snapshot(
                            state.uri_generations,
                            state.open,
                            state.db,
                            state.files,
                            state.published,
                        )
                    })
                    .then_some(())
            },
            |_, ()| publish(),
        )
        .await
    }

    async fn with_current_readonly_source_state<T, Checked>(
        &self,
        inputs_epoch: u64,
        factory_generation: u64,
        admissions: &[&DocumentPathAdmission],
        saved_sources: Option<&[(&DocumentPathAdmission, &[u8])]>,
        check: impl FnMut(&ReadonlySourceState<'_>) -> Option<Checked>,
        publish: impl FnMut(&ReadonlySourceState<'_>, &Checked) -> Option<T>,
    ) -> DocumentSourceResult<Option<T>> {
        self.readonly_source_transaction(
            inputs_epoch,
            factory_generation,
            admissions,
            saved_sources,
            check,
            publish,
        )
        .await
        .map(ReadonlyTransaction::published)
    }

    /// Rewrite one retained file's cross-file evidence with its own value, as
    /// the diagnostics worker does: the revision moves, captured values do not.
    #[cfg(test)]
    async fn write_unrelated_input_before_final(&self) {
        use salsa::Setter as _;
        if UNRELATED_WRITES_BEFORE_FINAL.get() == 0 {
            return;
        }
        UNRELATED_WRITES_BEFORE_FINAL.set(UNRELATED_WRITES_BEFORE_FINAL.get() - 1);
        let mut db = self.db.lock().await;
        let files = self.db_files.lock().await;
        let file = *files.values().next().expect("a retained source file");
        let evidence = file.external_call_sites(&*db).clone();
        file.set_external_call_sites(&mut *db).to(evidence);
    }

    async fn readonly_source_transaction<T, Checked>(
        &self,
        inputs_epoch: u64,
        factory_generation: u64,
        admissions: &[&DocumentPathAdmission],
        saved_sources: Option<&[(&DocumentPathAdmission, &[u8])]>,
        check: impl FnMut(&ReadonlySourceState<'_>) -> Option<Checked>,
        publish: impl FnMut(&ReadonlySourceState<'_>, &Checked) -> Option<T>,
    ) -> DocumentSourceResult<ReadonlyTransaction<T>> {
        self.readonly_source_transaction_diagnostic(
            (inputs_epoch, factory_generation),
            admissions,
            move |_| saved_sources.map(std::borrow::Cow::Borrowed),
            &super::WorkspaceSymbolDiagnostic::default(),
            check,
            publish,
        )
        .await
    }

    async fn readonly_source_transaction_diagnostic<'saved, T, Checked>(
        &self,
        epochs: (u64, u64),
        admissions: &'saved [&'saved DocumentPathAdmission],
        mut saved_sources: impl FnMut(&Checked) -> Option<ReadonlySavedSources<'saved>>,
        diagnostic: &super::WorkspaceSymbolDiagnostic,
        check: impl FnMut(&ReadonlySourceState<'_>) -> Option<Checked>,
        publish: impl FnMut(&ReadonlySourceState<'_>, &Checked) -> Option<T>,
    ) -> DocumentSourceResult<ReadonlyTransaction<T>> {
        self.readonly_source_transaction_with_inputs(
            epochs,
            |checked| ReadonlyPublicationInputs {
                admissions: std::borrow::Cow::Borrowed(admissions),
                saved_sources: saved_sources(checked),
            },
            diagnostic,
            check,
            publish,
        )
        .await
    }

    async fn readonly_source_transaction_with_inputs<'saved, T, Checked>(
        &self,
        epochs: (u64, u64),
        mut select_inputs: impl FnMut(&Checked) -> ReadonlyPublicationInputs<'saved>,
        diagnostic: &super::WorkspaceSymbolDiagnostic,
        mut check: impl FnMut(&ReadonlySourceState<'_>) -> Option<Checked>,
        mut publish: impl FnMut(&ReadonlySourceState<'_>, &Checked) -> Option<T>,
    ) -> DocumentSourceResult<ReadonlyTransaction<T>> {
        let (inputs_epoch, factory_generation) = epochs;
        #[cfg(test)]
        self.write_unrelated_input_before_final().await;
        let _inputs = self.analyser_inputs_gate.read().await;
        let generation = self.class_factory_generation.read().await;
        let open = self
            .documents
            .lock("readonly_reference_final_snapshot")
            .await;
        let Ok(db) = self.db.try_lock() else {
            diagnostic.record("lease.db_busy", &[]);
            return Ok(ReadonlyTransaction::CurrencyRefused);
        };
        let Ok(files) = self.db_files.try_lock() else {
            diagnostic.record("lease.files_busy", &[]);
            return Ok(ReadonlyTransaction::CurrencyRefused);
        };
        let published = self
            .source_path_admissions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let uri_generations = self
            .live_publication_uri_generations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !(self.diag_inputs_epoch() == inputs_epoch && *generation == factory_generation) {
            diagnostic.record("lease.epoch_or_factory", &[]);
            return Ok(ReadonlyTransaction::CurrencyRefused);
        }
        let state = ReadonlySourceState {
            open: &open,
            db: &db,
            files: &files,
            published: &published,
            uri_generations: &uri_generations,
        };
        let Some(checked) = check(&state) else {
            diagnostic.record("lease.source_or_index_state", &[]);
            return Ok(ReadonlyTransaction::CurrencyRefused);
        };
        let selected = select_inputs(&checked);
        let admissions = selected.admissions.as_ref();
        let issued = if let Some(sources) = selected.saved_sources.as_deref() {
            DocumentPathAdmission::with_current_sources(
                admissions,
                sources,
                self.store.as_ref(),
                || publish(&state, &checked),
            )
        } else {
            DocumentPathAdmission::with_current(admissions, self.store.as_ref(), || {
                publish(&state, &checked)
            })
        }?;
        Ok(issued.map_or(
            ReadonlyTransaction::PublishRefused,
            ReadonlyTransaction::Published,
        ))
    }

    #[cfg(test)]
    pub(super) async fn publish_checked_library_analyses(
        &self,
        analysed: &[(Uri, super::DocumentState, WorkspaceSymbolSourceAnalysis)],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<bool> {
        self.library_analyses_transaction(analysed, inputs_epoch, factory_generation)
            .await
            .map(|transaction| transaction.published().is_some())
    }

    /// Capture each library file and publish the whole set, recapturing only
    /// after a currency refusal. `None` withholds every captured analysis.
    pub(super) async fn capture_and_publish_library_files(
        &self,
        files: &[std::path::PathBuf],
    ) -> DocumentSourceResult<Option<Vec<(Uri, super::DocumentState, WorkspaceSymbolSourceAnalysis)>>>
    {
        retry_readonly_attempt(None, move || {
            self.capture_and_publish_library_attempt(files)
        })
        .await
    }

    async fn capture_and_publish_library_attempt(
        &self,
        files: &[std::path::PathBuf],
    ) -> DocumentSourceResult<
        ControlFlow<Option<Vec<(Uri, super::DocumentState, WorkspaceSymbolSourceAnalysis)>>>,
    > {
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let mut analysed = Vec::new();
        for path in files {
            let Some(target_uri) = super::canonical_file_uri(path) else {
                continue;
            };
            let Some(snapshot) = self.read_document(&target_uri).await? else {
                continue;
            };
            let Some(receipt) = self
                .capture_workspace_symbol_analysis(&target_uri, &snapshot)
                .await?
            else {
                continue;
            };
            analysed.push((target_uri, snapshot, receipt));
        }
        if analysed.is_empty() {
            return Ok(ControlFlow::Break(Some(analysed)));
        }
        Ok(self
            .library_analyses_transaction(&analysed, inputs_epoch, factory_generation)
            .await?
            .into_attempt()
            .map_break(|published| published.map(|()| analysed)))
    }

    async fn library_analyses_transaction(
        &self,
        analysed: &[(Uri, super::DocumentState, WorkspaceSymbolSourceAnalysis)],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<ReadonlyTransaction<()>> {
        let mut saved = Vec::new();
        let mut admissions = Vec::new();
        for (uri, snapshot, analysis) in analysed {
            if snapshot.path_admission.is_none()
                && self.store.workspace_source_inventory_provider()
                    == super::vfs::WorkspaceSourceInventoryProvider::HostReported
                && uri.scheme().as_str() == "file"
            {
                return Ok(ReadonlyTransaction::PublishRefused);
            }
            for admission in
                std::iter::once(analysis.admission()).chain(snapshot.path_admission.as_deref())
            {
                if admission.uri() != uri {
                    return Ok(ReadonlyTransaction::PublishRefused);
                }
                admissions.push(admission);
                if let Some(path) = admission.path() {
                    admission.check_current(self.store.as_ref())?;
                    let Ok(bytes) = self.store.read(path) else {
                        return Ok(ReadonlyTransaction::PublishRefused);
                    };
                    let (text, _) = tcl_lsp_core::source_decode::decode_source_as(
                        &bytes,
                        self.declared_script_encoding(),
                    );
                    if tcl_lexer::normalise_lone_cr(&text).as_ref() != snapshot.text.as_ref() {
                        return Ok(ReadonlyTransaction::PublishRefused);
                    }
                    saved.push((admission, bytes));
                }
            }
        }
        let sources = saved
            .iter()
            .map(|(admission, bytes)| (*admission, bytes.as_slice()))
            .collect::<Vec<_>>();
        self.readonly_source_transaction(
            inputs_epoch,
            factory_generation,
            &admissions,
            Some(&sources),
            |state| {
                analysed
                    .iter()
                    .all(|(uri, snapshot, analysis)| {
                        analysis.matches_snapshot(uri, snapshot)
                            && analysis.matches_current_state(
                                state,
                                inputs_epoch,
                                factory_generation,
                            )
                    })
                    .then_some(())
            },
            |_, ()| {
                let _rehoming = self.rehoming_gate.try_lock().ok()?;
                let mut index = self.workspace_index.try_write().ok()?;
                let mut seeds = self.rehomed_source_seeds.try_lock().ok()?;
                let mut autoloaded = self.autoloaded_library_uris.try_lock().ok()?;
                for (uri, _, analysis) in analysed {
                    index.replace_document(uri.as_str(), analysis.analysis());
                    seeds.remove(uri.as_str());
                    autoloaded.insert(uri.as_str().to_owned());
                }
                Some(())
            },
        )
        .await
    }

    /// Project namespace relationships only under their original input/index
    /// and publication owners. Missing is unavailable, never namespace absence.
    pub(super) async fn finish_conditional_namespace_advice(
        &self,
        advice: &tcl_lsp_core::workspace_index::ConditionalNamespaceImportAdvice,
    ) -> DocumentSourceResult<Option<Vec<String>>> {
        retry_readonly_attempt(None, move || {
            self.finish_conditional_namespace_advice_attempt(advice)
        })
        .await
    }

    async fn finish_conditional_namespace_advice_attempt(
        &self,
        advice: &tcl_lsp_core::workspace_index::ConditionalNamespaceImportAdvice,
    ) -> DocumentSourceResult<ControlFlow<Option<Vec<String>>>> {
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let mut documents = Vec::new();
        for source in advice.documents() {
            let Some(text) = source.source() else {
                return Ok(ControlFlow::Break(None));
            };
            let Some(document) = self
                .capture_readonly_source_document(source.uri(), Arc::from(text))
                .await?
            else {
                return Ok(ControlFlow::Break(None));
            };
            if !source.matches_input(document.input.as_ref()) {
                return Ok(ControlFlow::Break(None));
            }
            documents.push(document);
        }
        let _inputs = self.analyser_inputs_gate.read().await;
        let generation = self.class_factory_generation.read().await;
        let open = self.documents.lock("namespace_advice_final_snapshot").await;
        let index = self.workspace_index.read().await;
        let Ok(db) = self.db.try_lock() else {
            return Ok(ControlFlow::Continue(()));
        };
        let Ok(files) = self.db_files.try_lock() else {
            return Ok(ControlFlow::Continue(()));
        };
        let published = self
            .source_path_admissions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let uri_generations = self
            .live_publication_uri_generations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.diag_inputs_epoch() != inputs_epoch
            || *generation != factory_generation
            || !advice.matches_index(&index)
            || documents.iter().any(|document| {
                !document.matches_current_snapshot(&uri_generations, &open, &db, &files, &published)
            })
        {
            return Ok(ControlFlow::Continue(()));
        }
        let admissions = documents
            .iter()
            .map(|document| document.admission.as_ref())
            .collect::<Vec<_>>();
        DocumentPathAdmission::with_current(&admissions, self.store.as_ref(), || {
            Some(
                advice
                    .compatibility_report()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            )
        })
        .map(ControlFlow::Break)
    }

    async fn logical_call_documents(
        &self,
        selected: &Uri,
    ) -> DocumentSourceResult<Option<Vec<OriginalReadonlyReferenceDocument>>> {
        let documents = self.original_declaration_documents(selected).await?;
        let mut retained = Vec::with_capacity(documents.len());
        for (uri, source, _) in documents {
            let Some(document) = self
                .capture_readonly_reference_document(&uri, source)
                .await?
            else {
                return Ok(None);
            };
            retained.push(document);
        }
        if !retained.iter().any(|document| document.uri == *selected) {
            return Ok(None);
        }
        Ok(Some(retained))
    }

    pub(super) async fn readonly_logical_lenses(
        &self,
        selected: &Uri,
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::CodeLens>>> {
        retry_readonly_attempt(None, move || self.readonly_logical_lenses_attempt(selected)).await
    }

    async fn readonly_logical_lenses_attempt(
        &self,
        selected: &Uri,
    ) -> DocumentSourceResult<ControlFlow<Option<Vec<tower_lsp_server::ls_types::CodeLens>>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(documents) = self.logical_call_documents(selected).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &documents,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(lenses) = tcl_lsp_core::code_lens::logical_source_lenses(
            selected.as_str(),
            &logical_call_inventory(&documents),
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let mut registry = self.original_declaration_registry.lock().await;
        Ok(self.readonly_documents_transaction(&documents, epoch, generation, || {
            let workspace = self.workspace_index.try_read().ok()?;
            if LogicalCallPublicationStamp::capture(&documents, &workspace, epoch, generation).as_ref() != Some(&stamp) {
                return None;
            }
            let inventory = logical_call_inventory(&documents);
            let publication = tcl_lsp_core::original_declaration::LogicalCallHostPublication::retain(stamp.clone());
            lenses.iter().map(|lens| {
                lens.identity().is_current_in(&inventory).then_some(())?;
                let handle = registry.issue_logical_lens(lens.identity().clone(), publication.clone())?;
                Some(tower_lsp_server::ls_types::CodeLens {
                    range: super::lift_lsp_range(lens.range()), command: None,
                    data: Some(serde_json::json!({ "logicalSourceLens": handle, "uri": selected.as_str() })),
                })
            }).collect::<Option<Vec<_>>>()
        }).await?.into_attempt())
    }

    pub(super) async fn readonly_logical_lens_resolve(
        &self,
        selected: &Uri,
        handle: tcl_lsp_core::original_declaration::OriginalDeclarationHandle,
    ) -> DocumentSourceResult<Option<tower_lsp_server::ls_types::CodeLens>> {
        retry_readonly_attempt(None, move || {
            self.readonly_logical_lens_resolve_attempt(selected, handle)
        })
        .await
    }

    async fn readonly_logical_lens_resolve_attempt(
        &self,
        selected: &Uri,
        handle: tcl_lsp_core::original_declaration::OriginalDeclarationHandle,
    ) -> DocumentSourceResult<ControlFlow<Option<tower_lsp_server::ls_types::CodeLens>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(documents) = self.logical_call_documents(selected).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &documents,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let publication =
            tcl_lsp_core::original_declaration::LogicalCallHostPublication::retain(stamp.clone());
        let Some(identity) = self
            .original_declaration_registry
            .lock()
            .await
            .resolve_logical_lens(
                handle,
                selected.as_str(),
                &logical_call_inventory(&documents),
                &publication,
            )
        else {
            return Ok(ControlFlow::Break(None));
        };
        let uri = selected.clone();
        let (documents, selection, report) = crate::rt::spawn_blocking(move || {
            let inventory = logical_call_inventory(&documents);
            let selection =
                tcl_lsp_core::code_lens::logical_source_lenses(uri.as_str(), &inventory).and_then(
                    |lenses| lenses.into_iter().find(|lens| lens.identity() == &identity),
                );
            let report =
                tcl_lsp_core::code_lens::logical_lens_reference_advice(&identity, &inventory);
            (documents, selection, report)
        })
        .await
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let (Some(selection), Some(report)) = (selection, report) else {
            return Ok(ControlFlow::Break(None));
        };
        Ok(self
            .readonly_documents_transaction(&documents, epoch, generation, || {
                let workspace = self.workspace_index.try_read().ok()?;
                if LogicalCallPublicationStamp::capture(&documents, &workspace, epoch, generation)
                    .as_ref()
                    != Some(&stamp)
                    || !report.is_current(&logical_call_inventory(&documents))
                {
                    return None;
                }
                let locations = report
                    .possible_source_targets()
                    .iter()
                    .map(|(owner, range)| {
                        documents
                            .iter()
                            .find(|document| document.uri.as_str() == owner)
                            .map(|document| Location {
                                uri: document.uri.clone(),
                                range: super::lift_lsp_range(*range),
                            })
                    })
                    .collect::<Option<Vec<_>>>()?;
                let range = super::lift_lsp_range(selection.range());
                let command = OriginalReadonlyReferenceReport {
                    locations,
                    coverage: report.coverage(),
                }
                .lens_command(selected, range.start);
                Some(tower_lsp_server::ls_types::CodeLens {
                    range,
                    command: Some(command),
                    data: Some(serde_json::json!({
                        "logicalSourceLens": handle, "uri": selected.as_str(),
                        "sourceReferenceCoverage": coverage_tag(report.coverage()),
                    })),
                })
            })
            .await?
            .into_attempt())
    }

    pub(super) async fn readonly_logical_call_prepare(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::CallHierarchyItem>>> {
        retry_readonly_attempt(None, move || {
            self.readonly_logical_call_prepare_attempt(selected, position)
        })
        .await
    }

    async fn readonly_logical_call_prepare_attempt(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<ControlFlow<Option<Vec<tower_lsp_server::ls_types::CallHierarchyItem>>>>
    {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(retained) = self.logical_call_documents(selected).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let uri = selected.clone();
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &retained,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let (retained, advice) = crate::rt::spawn_blocking(move || {
            let inventory = logical_call_inventory(&retained);
            let advice = tcl_lsp_core::call_hierarchy::prepare_logical_from_inventory(
                uri.as_str(),
                position.line,
                position.character,
                &inventory,
            );
            (retained, advice)
        })
        .await
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(ControlFlow::Break(None));
        };
        Ok(self
            .finish_logical_call_advice(&retained, &advice, &stamp)
            .await?
            .into_attempt()
            .map_break(|projection| projection.map(|projection| projection.items)))
    }

    pub(super) async fn readonly_logical_call_walk(
        &self,
        item: &tower_lsp_server::ls_types::CallHierarchyItem,
        incoming: bool,
    ) -> DocumentSourceResult<
        Option<Vec<(tower_lsp_server::ls_types::CallHierarchyItem, Vec<Range>)>>,
    > {
        retry_readonly_attempt(None, move || {
            self.readonly_logical_call_walk_attempt(item, incoming)
        })
        .await
    }

    async fn readonly_logical_call_walk_attempt(
        &self,
        item: &tower_lsp_server::ls_types::CallHierarchyItem,
        incoming: bool,
    ) -> DocumentSourceResult<
        ControlFlow<Option<Vec<(tower_lsp_server::ls_types::CallHierarchyItem, Vec<Range>)>>>,
    > {
        let Some(handle) = item
            .data
            .as_ref()
            .and_then(|data| data.get("logicalCallHierarchy"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
        else {
            return Ok(ControlFlow::Break(None));
        };
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(retained) = self.logical_call_documents(&item.uri).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(stamp) = LogicalCallPublicationStamp::capture(
            &retained,
            &*self.workspace_index.read().await,
            epoch,
            generation,
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let publication =
            tcl_lsp_core::original_declaration::LogicalCallHostPublication::retain(stamp.clone());
        let Some(identity) = self
            .original_declaration_registry
            .lock()
            .await
            .resolve_logical_call(
                handle,
                item.uri.as_str(),
                &logical_call_inventory(&retained),
                &publication,
            )
        else {
            return Ok(ControlFlow::Break(None));
        };
        let (retained, advice) = crate::rt::spawn_blocking(move || {
            let inventory = logical_call_inventory(&retained);
            let advice = if incoming {
                tcl_lsp_core::call_hierarchy::logical_incoming_from_inventory(&identity, &inventory)
            } else {
                tcl_lsp_core::call_hierarchy::logical_outgoing_from_inventory(&identity, &inventory)
            };
            (retained, advice)
        })
        .await
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: item.uri.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(ControlFlow::Break(None));
        };
        Ok(self
            .finish_logical_call_advice(&retained, &advice, &stamp)
            .await?
            .into_attempt()
            .map_break(|projection| projection.map(|projection| projection.edges)))
    }

    async fn finish_logical_call_advice(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        advice: &tcl_lsp_core::call_hierarchy::LogicalCallAdvice,
        stamp: &LogicalCallPublicationStamp,
    ) -> DocumentSourceResult<ReadonlyTransaction<LogicalCallProjection>> {
        let mut registry = self.original_declaration_registry.lock().await;
        self.readonly_documents_transaction(
            documents,
            stamp.inputs_epoch,
            stamp.factory_generation,
            || {
                // Nonblocking acquisition inside the existing final lease preserves
                // its lock ordering and holds the actual indexed revision through
                // issuance. Missing/busy index is unavailable, never fresh by bytes.
                let workspace = self.workspace_index.try_read().ok()?;
                if LogicalCallPublicationStamp::capture(
                    documents,
                    &workspace,
                    stamp.inputs_epoch,
                    stamp.factory_generation,
                )
                .as_ref()
                    != Some(stamp)
                {
                    return None;
                }
                let publication =
                    tcl_lsp_core::original_declaration::LogicalCallHostPublication::retain(
                        stamp.clone(),
                    );
                if !advice.is_current(&logical_call_inventory(documents)) {
                    return None;
                }
                // Retain typed debt and every independent original source receipt
                // through this lease. Standard LSP lists project possible positives;
                // neither an empty list nor an item grants call-graph absence.
                let items = advice
                    .possible_source_items()
                    .iter()
                    .map(|item| lift_logical_call_item(item, &mut registry, &publication))
                    .collect::<Option<Vec<_>>>()?;
                let edges = advice
                    .possible_source_edges()
                    .iter()
                    .map(|edge| {
                        Some((
                            lift_logical_call_item(edge.item(), &mut registry, &publication)?,
                            edge.possible_source_ranges()
                                .iter()
                                .copied()
                                .map(super::lift_lsp_range)
                                .collect(),
                        ))
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(LogicalCallProjection { items, edges })
            },
        )
        .await
    }

    async fn finish_logical_hierarchy_projection<'a>(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        projection: impl Fn() -> &'a [tcl_lsp_core::type_hierarchy::TypeHierarchyItem],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<ReadonlyTransaction<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>>>
    {
        let mut registry = self.original_declaration_registry.lock().await;
        self.readonly_documents_transaction(documents, inputs_epoch, factory_generation, || {
            projection()
                .iter()
                .map(|item| {
                    let identity = item.logical_declaration.as_ref()?;
                    let document = documents
                        .iter()
                        .find(|document| document.uri.as_str() == identity.uri())?;
                    if !identity.is_current(identity.uri(), &document.source, &document.analysis) {
                        return None;
                    }
                    let handle = registry.issue_logical_class(identity.clone())?;
                    Some(tower_lsp_server::ls_types::TypeHierarchyItem {
                        name: item.name.clone(),
                        kind: tower_lsp_server::ls_types::SymbolKind::CLASS,
                        tags: None,
                        detail: item.detail.clone(),
                        uri: document.uri.clone(),
                        range: super::lift_lsp_range(item.range),
                        selection_range: super::lift_lsp_range(item.selection_range),
                        data: Some(serde_json::json!({ "logicalHierarchy": handle })),
                    })
                })
                .collect::<Option<Vec<_>>>()
        })
        .await
    }

    async fn finish_logical_hierarchy_advice(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        advice: &tcl_lsp_core::type_hierarchy::LogicalHierarchyAdvice,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<ReadonlyTransaction<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>>>
    {
        self.finish_logical_hierarchy_projection(
            documents,
            || advice.possible_source_items(),
            inputs_epoch,
            factory_generation,
        )
        .await
    }

    #[cfg(test)]
    async fn finish_logical_hierarchy_items(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        items: &[tcl_lsp_core::type_hierarchy::TypeHierarchyItem],
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>> {
        self.finish_logical_hierarchy_projection(
            documents,
            || items,
            inputs_epoch,
            factory_generation,
        )
        .await
        .map(|transaction| transaction.published().unwrap_or_default())
    }

    pub(super) async fn readonly_logical_hierarchy_prepare(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>> {
        retry_readonly_attempt(Vec::new(), move || {
            self.readonly_logical_hierarchy_prepare_attempt(selected, position)
        })
        .await
    }

    async fn readonly_logical_hierarchy_prepare_attempt(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<ControlFlow<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(current) = self.read_document(selected).await? else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let Some(document) = self
            .capture_readonly_reference_document(selected.as_str(), current.text)
            .await?
        else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let (document, advice) = crate::rt::spawn_blocking(move || {
            let advice = tcl_lsp_core::type_hierarchy::prepare_logical_advice_for_document(
                document.uri.as_str(),
                &document.source,
                position.line,
                position.character,
                &document.analysis,
            );
            (document, advice)
        })
        .await
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        Ok(self
            .finish_logical_hierarchy_advice(&[document], &advice, epoch, generation)
            .await?
            .into_attempt()
            .map_break(Option::unwrap_or_default))
    }

    pub(super) async fn readonly_logical_hierarchy_walk(
        &self,
        item: &tower_lsp_server::ls_types::TypeHierarchyItem,
        subtypes: bool,
    ) -> DocumentSourceResult<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>> {
        retry_readonly_attempt(Vec::new(), move || {
            self.readonly_logical_hierarchy_walk_attempt(item, subtypes)
        })
        .await
    }

    async fn readonly_logical_hierarchy_walk_attempt(
        &self,
        item: &tower_lsp_server::ls_types::TypeHierarchyItem,
        subtypes: bool,
    ) -> DocumentSourceResult<ControlFlow<Vec<tower_lsp_server::ls_types::TypeHierarchyItem>>> {
        let Some(handle) = item
            .data
            .as_ref()
            .and_then(|data| data.get("logicalHierarchy"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
        else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let documents = self.original_declaration_documents(&item.uri).await?;
        let mut retained = Vec::with_capacity(documents.len());
        for (uri, source, _) in documents {
            let Some(document) = self
                .capture_readonly_reference_document(&uri, source)
                .await?
            else {
                return Ok(ControlFlow::Break(Vec::new()));
            };
            retained.push(document);
        }
        let Some(owner) = retained.iter().find(|document| document.uri == item.uri) else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let Some(identity) = self
            .original_declaration_registry
            .lock()
            .await
            .resolve_logical_class(handle, item.uri.as_str(), &owner.source, &owner.analysis)
        else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let (retained, advice) = crate::rt::spawn_blocking(move || {
            let inventory = retained
                .iter()
                .map(
                    |document| tcl_lsp_core::type_hierarchy::OriginalHierarchyDocument {
                        uri: document.uri.as_str(),
                        source: &document.source,
                        analysis: &document.analysis,
                    },
                )
                .collect::<Vec<_>>();
            let advice = tcl_lsp_core::type_hierarchy::related_logical_from_inventory(
                &identity, &inventory, subtypes,
            );
            (retained, advice)
        })
        .await
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: item.uri.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        Ok(self
            .finish_logical_hierarchy_advice(&retained, &advice, epoch, generation)
            .await?
            .into_attempt()
            .map_break(Option::unwrap_or_default))
    }

    pub(super) async fn readonly_logical_implementation(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<Option<Vec<Location>>> {
        retry_readonly_attempt(None, move || {
            self.readonly_logical_implementation_attempt(selected, position)
        })
        .await
    }

    async fn readonly_logical_implementation_attempt(
        &self,
        selected: &Uri,
        position: Position,
    ) -> DocumentSourceResult<ControlFlow<Option<Vec<Location>>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = *self.class_factory_generation.read().await;
        let Some(current) = self.read_document(selected).await? else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(document) = self
            .capture_readonly_reference_document(selected.as_str(), current.text)
            .await?
        else {
            return Ok(ControlFlow::Break(None));
        };
        let (document, advice) = crate::rt::spawn_blocking(move || {
            let advice = tcl_lsp_core::implementation::logical_implementation_advice(
                document.uri.as_str(),
                &document.source,
                position.line,
                position.character,
                &document.analysis,
            );
            (document, advice)
        })
        .await
        .map_err(|_| super::DocumentSourceUnavailable::Worker {
            uri: selected.clone(),
        })?;
        let Some(advice) = advice else {
            return Ok(ControlFlow::Break(None));
        };
        Ok(self
            .finish_logical_implementation_advice(&[document], &advice, epoch, generation)
            .await?
            .into_attempt())
    }

    async fn finish_logical_implementation_advice(
        &self,
        documents: &[OriginalReadonlyReferenceDocument],
        advice: &tcl_lsp_core::implementation::LogicalImplementationAdvice,
        epoch: u64,
        generation: u64,
    ) -> DocumentSourceResult<ReadonlyTransaction<Vec<Location>>> {
        self.readonly_documents_transaction(documents, epoch, generation, || {
            let owner = documents.first()?;
            if !advice.is_current(owner.uri.as_str(), &owner.source, &owner.analysis) {
                return None;
            }
            // Standard Locations carry possible positives only; the typed
            // source result and its debt stay held through this final lease.
            Some(
                advice
                    .possible_source_ranges()
                    .iter()
                    .map(|range| Location {
                        uri: owner.uri.clone(),
                        range: super::lift_lsp_range(*range),
                    })
                    .collect(),
            )
        })
        .await
    }

    /// Source-only occurrences join the original declaration in its real URI.
    /// All participants retain their actual `SourceFile` and full checked input;
    /// this report cannot be consumed as call edges or editable occurrences.
    pub(super) async fn readonly_original_declaration_reference_report(
        &self,
        identity: &OriginalDeclarationIdentity,
        selected: &Uri,
        include_declaration: bool,
    ) -> DocumentSourceResult<Option<OriginalReadonlyReferenceReport>> {
        retry_readonly_attempt(None, move || {
            self.readonly_original_declaration_reference_attempt(
                identity,
                selected,
                include_declaration,
            )
        })
        .await
    }

    async fn readonly_original_declaration_reference_attempt(
        &self,
        identity: &OriginalDeclarationIdentity,
        selected: &Uri,
        include_declaration: bool,
    ) -> DocumentSourceResult<ControlFlow<Option<OriginalReadonlyReferenceReport>>> {
        let inputs_epoch = self.diag_inputs_epoch();
        let factory_generation = *self.class_factory_generation.read().await;
        let documents = self.original_declaration_documents(selected).await?;
        let mut retained = Vec::with_capacity(documents.len());
        for (owner, source, _) in documents {
            let Some(document) = self
                .capture_readonly_reference_document(&owner, source)
                .await?
            else {
                return Ok(ControlFlow::Break(None));
            };
            retained.push(document);
        }
        let Some(owner) = retained
            .iter()
            .find(|document| document.uri.as_str() == identity.uri())
        else {
            return Ok(ControlFlow::Break(None));
        };
        if !identity.is_current(identity.uri(), &owner.source, &owner.analysis) {
            return Ok(ControlFlow::Break(None));
        }
        if !original_declaration_owner_is_unique(identity, &retained) {
            return Ok(ControlFlow::Break(None));
        }
        let Some(report) = tcl_lsp_core::original_declaration::readonly_reference_report(
            identity,
            &owner.source,
            &owner.analysis,
            true,
        ) else {
            return Ok(ControlFlow::Break(None));
        };
        let inventory = retained
            .iter()
            .map(
                |document| tcl_lsp_core::original_call_hierarchy::OriginalCallDocument {
                    uri: document.uri.as_str(),
                    source: &document.source,
                    analysis: &document.analysis,
                },
            )
            .collect::<Vec<_>>();
        let mut targets = {
            let workspace = self.workspace_index.read().await;
            tcl_lsp_core::original_call_hierarchy::references(
                identity,
                &inventory,
                Some(&workspace),
            )
        };
        targets.extend(
            report
                .spans()
                .iter()
                .map(|span| (identity.uri().to_owned(), *span)),
        );
        if include_declaration {
            targets.push((identity.uri().to_owned(), identity.span()));
        }
        super::dedup_span_targets(&mut targets);
        let coverage = single_document_coverage(report.coverage(), retained.len());
        Ok(self
            .finish_readonly_reference_report(
                &retained,
                targets,
                coverage,
                inputs_epoch,
                factory_generation,
            )
            .await?
            .into_attempt())
    }

    pub(super) async fn readonly_original_method_reference_locations_at(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        position: Position,
        include_declaration: bool,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Vec<Location>>> {
        use std::ops::ControlFlow;
        let target = match self
            .original_method_candidate_at(uri, source, analysis, position)
            .await?
        {
            ControlFlow::Continue(()) => return Ok(ControlFlow::Continue(())),
            ControlFlow::Break(None) => {
                return Ok(ControlFlow::Break(Vec::new()));
            }
            ControlFlow::Break(Some(target)) => target,
        };
        let Some(owner) = target.uri().parse::<Uri>().ok() else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let Some(document) = self.read_document(&owner).await? else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        let current = self
            .analysis_for(&owner, document.text.clone(), document.dialect)
            .await?;
        let Some(identity) = OriginalDeclarationIdentity::for_member_candidate(
            owner.as_str(),
            &document.text,
            &current,
            &target,
        ) else {
            return Ok(ControlFlow::Break(Vec::new()));
        };
        Ok(ControlFlow::Break(
            self.readonly_original_declaration_reference_report(
                &identity,
                uri,
                include_declaration,
            )
            .await?
            .map_or_else(Vec::new, |report| report.locations),
        ))
    }
}

fn lift_readonly_reference_targets(
    targets: &[(String, tcl_lexer::Span)],
    documents: &[OriginalReadonlyReferenceDocument],
) -> Option<Vec<Location>> {
    targets
        .iter()
        .map(|(uri, span)| {
            let document = documents
                .iter()
                .find(|document| document.uri.as_str() == uri.as_str())?;
            if span.start() > span.end() || span.end() as usize > document.source.len() {
                return None;
            }
            let index = tcl_lexer::LineIndex::new_lsp(&document.source);
            let start = index.position_at_utf16(span.start(), &document.source);
            let end = index.position_at_utf16(span.end(), &document.source);
            Some(Location {
                uri: document.uri.clone(),
                range: Range {
                    start: Position::new(start.line, start.character.get()),
                    end: Position::new(end.line, end.character.get()),
                },
            })
        })
        .collect()
}

fn original_declaration_owner_is_unique(
    identity: &OriginalDeclarationIdentity,
    documents: &[OriginalReadonlyReferenceDocument],
) -> bool {
    documents
        .iter()
        .filter(|document| {
            tcl_lsp_core::original_declaration::declarations(
                document.uri.as_str(),
                &document.source,
                &document.analysis,
            )
            .is_some_and(|rows| {
                rows.iter().any(|row| {
                    row.role() == identity.role()
                        && row.site() == identity.site()
                        && row.image() == identity.image()
                        && row.config() == identity.config()
                })
            })
        })
        .count()
        == 1
}

impl OriginalReadonlyReferenceDocument {
    fn matches_current_snapshot(
        &self,
        uri_generations: &std::collections::HashMap<Uri, u64>,
        open: &std::collections::HashMap<Uri, super::DocumentState>,
        db: &tcl_lsp_db::TclDatabase,
        files: &std::collections::HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &std::collections::HashMap<Uri, super::PublishedSourcePath>,
    ) -> bool {
        self.matches_original_input()
            && self.source_currency().matches_current_snapshot(
                uri_generations,
                open,
                db,
                files,
                published,
            )
    }

    fn source_currency(&self) -> ReadonlySourceDocument {
        ReadonlySourceDocument {
            uri: self.uri.clone(),
            source: self.source.clone(),
            file: self.file,
            config: self.config,
            configuration: self.configuration.clone(),
            input: self.input.clone(),
            external_call_sites: self.external_call_sites.clone(),
            admission: self.admission.clone(),
            publication: LibraryPublicationSource {
                uri: self.publication.uri.clone(),
                document: self.publication.document.clone(),
                open_revision: self.publication.open_revision,
                uri_generation: self.publication.uri_generation,
            },
            database_revision: self.database_revision,
        }
    }

    fn matches_original_input(&self) -> bool {
        original_readonly_analysis_matches_input(&self.source, &self.analysis, &self.input)
    }

    /// Workspace-symbol snapshot validity (#1854). The Salsa handle, provider
    /// receipt, path, configuration values and analysis input must be the
    /// captured ones; the database revision need not, so an unrelated write
    /// or a later edit of this buffer keeps the captured answer. An unchanged
    /// buffer, or a closed file, must still be the exact captured publication.
    fn matches_request_snapshot(
        &self,
        state: &ReadonlySourceState<'_>,
        diagnostic: &super::WorkspaceSymbolDiagnostic,
    ) -> bool {
        let current = state.open.get(&self.uri);
        let advanced = self.publication.open_revision.is_some()
            && current
                .is_some_and(|current| open_document_advanced(&self.publication.document, current));
        let publication_matches = if advanced {
            current.is_some_and(|current| !current.backing_file_deleted)
        } else {
            self.publication.matches_open_documents(state.open)
                && state
                    .uri_generations
                    .get(&self.uri)
                    .copied()
                    .unwrap_or_default()
                    == self.publication.uri_generation
                && self.file.text(state.db).as_str() == self.source.as_ref()
                && self.file.dialect(state.db) == &self.publication.document.dialect
        };
        diagnostic.check("lease.db.publication", publication_matches)
            && diagnostic.check("lease.db.original_input", self.matches_original_input())
            && diagnostic.check(
                "lease.db.file",
                state.files.get(&self.uri) == Some(&self.file),
            )
            && diagnostic.check(
                "lease.db.path_receipt",
                state.published.get(&self.uri).is_some_and(|retained| {
                    retained.file == self.file
                        && retained.admission.as_ref() == self.admission.as_ref()
                }),
            )
            && diagnostic.check(
                "lease.db.path",
                self.file.path(state.db).as_deref() == self.admission.analyser_path(),
            )
            && diagnostic.check(
                "lease.db.configuration",
                self.config.diagnostic_report_configuration(state.db) == self.configuration,
            )
            && diagnostic.check(
                "lease.db.analysis_input",
                { tcl_lsp_db::document_analysis_input(state.db, self.file, self.config) }
                    .is_ok_and(|input| input.as_ref() == self.input.as_ref()),
            )
    }
}

/// Whether `live` is a later client version of the open buffer `captured`
/// named, without its local revision going backwards. Such a request answers
/// from its captured revision (#1854); equal or regressed versions must still
/// be that exact revision. A close and reopen at a higher version is accepted
/// when the captured revision was 0, since reopening restarts at revision 0:
/// the captured text and its own analysis still answer together, and the
/// provider receipt is checked independently. A closed capture has no client
/// version, so it never advances: a changed disk read or index revision for
/// it withdraws the row deliberately.
fn open_document_advanced(captured: &super::DocumentState, live: &super::DocumentState) -> bool {
    matches!(
        (captured.version, live.version),
        (Some(captured_version), Some(live_version)) if live_version > captured_version
    ) && live.revision >= captured.revision
}

fn workspace_symbol_index_snapshot_is_current(
    uri: &Uri,
    snapshot: &super::DocumentState,
    indexed_revision: Option<u64>,
    live: Option<&super::DocumentState>,
    index: &tcl_lsp_core::workspace_index::WorkspaceIndex,
) -> bool {
    let Some(advanced) = workspace_symbol_snapshot_advanced(snapshot, live) else {
        return false;
    };
    // An unchanged or closed source keeps its captured index revision.
    advanced
        || match indexed_revision {
            Some(revision) => index.document_revision(uri.as_str()) == Some(revision),
            None => !index.contains_document(uri.as_str()),
        }
}

fn workspace_symbol_snapshot_advanced(
    snapshot: &super::DocumentState,
    live: Option<&super::DocumentState>,
) -> Option<bool> {
    Some(match live {
        Some(live) if live.backing_file_deleted => return None,
        // The buffer moved on after capture; its index entry
        // may follow it. The captured revision still answers.
        Some(live) if open_document_advanced(snapshot, live) => true,
        Some(live) => {
            if live.revision != snapshot.revision
                || live.version != snapshot.version
                || live.raw() != snapshot.raw()
                || live.dialect != snapshot.dialect
                || live.path_admission != snapshot.path_admission
            {
                return None;
            }
            false
        }
        None if snapshot.version.is_some() => return None,
        None => false,
    })
}

fn code_action_document_matches(
    expected: &super::DocumentState,
    current: &super::DocumentState,
) -> bool {
    expected.version.is_some()
        && current.version == expected.version
        && !current.backing_file_deleted
        && current.revision == expected.revision
        && current.raw() == expected.raw()
        && current.clone().normalised_for_analysis().text
            == expected.clone().normalised_for_analysis().text
        && current.dialect == expected.dialect
        && current.language_id == expected.language_id
        && current.path_admission == expected.path_admission
        && current.publication == expected.publication
        && current.dialect_resolution_generation == expected.dialect_resolution_generation
        && current.resolved_dialect_generation == expected.resolved_dialect_generation
}

fn original_readonly_analysis_matches_input(
    source: &str,
    analysis: &AnalysisResult,
    input: &ResolvedAnalysisInput,
) -> bool {
    let config = input.lexer_config();
    analysis.resolved_input.as_ref() == Some(input)
        && analysis.analysis_context_unavailable.is_none()
        && analysis.body_lexer_config == Some(config)
        && analysis.matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        && analysis
            .retained_command_realm()
            .is_some_and(|realm| realm.matches_resolved_analysis_input(input))
}

impl WorkspaceSymbolSourceAnalysis {
    pub(super) fn analysis(&self) -> &AnalysisResult {
        match &self.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => &document.analysis,
            WorkspaceSymbolAnalysisKind::Fresh(document) => &document.seed.analysis,
        }
    }

    fn matches_snapshot(&self, uri: &Uri, snapshot: &super::DocumentState) -> bool {
        match &self.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => {
                &document.uri == uri
                    && document.source == snapshot.text
                    && document.publication.document.raw() == snapshot.raw()
            }
            WorkspaceSymbolAnalysisKind::Fresh(document) => {
                &document.uri == uri
                    && document.snapshot.text == snapshot.text
                    && document.snapshot.raw() == snapshot.raw()
            }
        }
    }

    fn admission(&self) -> &DocumentPathAdmission {
        match &self.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => &document.admission,
            WorkspaceSymbolAnalysisKind::Fresh(document) => &document.seed.path_admission,
        }
    }

    fn matches_current_state(
        &self,
        state: &ReadonlySourceState<'_>,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> bool {
        match &self.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => document.matches_current_snapshot(
                state.uri_generations,
                state.open,
                state.db,
                state.files,
                state.published,
            ),
            WorkspaceSymbolAnalysisKind::Fresh(document) => {
                let open_matches = match state.open.get(&document.uri) {
                    Some(current) if document.was_open => document.open_snapshot_matches(current),
                    None => !document.was_open,
                    Some(_) => false,
                };
                open_matches
                    && document.uri_generation_matches(state)
                    // A fresh syntax worker retains no Salsa query result.
                    && (matches!(document.purpose, FreshSourceAnalysisPurpose::SemanticStructure)
                        || tcl_lsp_db::database_revision(state.db) == document.database_revision)
                    && document.matches_input_state(state, inputs_epoch, factory_generation)
            }
        }
    }

    /// Validity of this request's captured snapshot (#1854). Unlike
    /// [`Self::matches_current_state`], a buffer whose client version advanced
    /// after capture keeps its captured analysis, and unrelated database writes
    /// do not withdraw it. A replaced, closed or deleted captured revision, and
    /// any configuration, factory, epoch, provider or input change, still do.
    fn matches_request_snapshot(
        &self,
        state: &ReadonlySourceState<'_>,
        inputs_epoch: u64,
        factory_generation: u64,
        diagnostic: &super::WorkspaceSymbolDiagnostic,
    ) -> bool {
        match &self.kind {
            WorkspaceSymbolAnalysisKind::Database(document) => {
                document.matches_request_snapshot(state, diagnostic)
            }
            WorkspaceSymbolAnalysisKind::Fresh(document) => {
                let current = state.open.get(&document.uri);
                let advanced = document.was_open
                    && matches!(
                        document.purpose,
                        FreshSourceAnalysisPurpose::ReadonlyComplete
                    )
                    && current
                        .is_some_and(|current| open_document_advanced(&document.snapshot, current));
                let open_matches = match current {
                    Some(current) if advanced => !current.backing_file_deleted,
                    Some(current) if document.was_open => document.open_snapshot_matches(current),
                    None => !document.was_open,
                    Some(_) => false,
                };
                diagnostic.check("lease.fresh.open_snapshot", open_matches)
                    && diagnostic.check(
                        "lease.fresh.uri_generation",
                        advanced || document.uri_generation_matches(state),
                    )
                    && diagnostic.check(
                        "lease.fresh.inputs",
                        document.matches_input_state(state, inputs_epoch, factory_generation),
                    )
            }
        }
    }
}

impl WorkspaceSymbolFreshInput {
    /// Eligibility for discarding old data and recapturing, never publication.
    fn recompute_snapshot_matches(&self, current: &super::DocumentState) -> bool {
        matches!(self.purpose, FreshSourceAnalysisPurpose::SemanticStructure)
            && current.backing_file_deleted == self.snapshot.backing_file_deleted
            && current.revision == self.snapshot.revision
            && current.version == self.snapshot.version
            && current.raw() == self.snapshot.raw()
            && current.dialect == self.snapshot.dialect
            && (current.publication == self.snapshot.publication
                || (self.snapshot.publication == super::DocumentPublication::Salsa
                    && current.publication == super::DocumentPublication::Indexed))
            && current.path_admission == self.snapshot.path_admission
            && !current.dialect_resolution_pending()
    }

    fn open_snapshot_matches(&self, current: &super::DocumentState) -> bool {
        (match self.purpose {
            FreshSourceAnalysisPurpose::ReadonlyComplete
            | FreshSourceAnalysisPurpose::ConditionalSourceRegion => !current.backing_file_deleted,
            FreshSourceAnalysisPurpose::SemanticStructure => {
                current.backing_file_deleted == self.snapshot.backing_file_deleted
            }
        }) && current.revision == self.snapshot.revision
            && current.version == self.snapshot.version
            && current.raw() == self.snapshot.raw()
            && current.dialect == self.snapshot.dialect
            && (current.publication == self.snapshot.publication
                || (matches!(self.purpose, FreshSourceAnalysisPurpose::SemanticStructure)
                    && self.snapshot.publication == super::DocumentPublication::Salsa
                    && current.publication == super::DocumentPublication::Indexed))
            && current.path_admission == self.snapshot.path_admission
            && current.dialect_resolution_generation == self.snapshot.dialect_resolution_generation
            && current.resolved_dialect_generation == self.snapshot.resolved_dialect_generation
    }

    fn uri_generation_matches(&self, state: &ReadonlySourceState<'_>) -> bool {
        self.uri_generation
            == state
                .uri_generations
                .get(&self.uri)
                .copied()
                .unwrap_or_default()
    }

    /// The analysis still describes its own captured source under the
    /// request's analyser inputs, configuration and provider receipt.
    fn matches_input_state(
        &self,
        state: &ReadonlySourceState<'_>,
        inputs_epoch: u64,
        factory_generation: u64,
    ) -> bool {
        self.seed.analyser_inputs_epoch == inputs_epoch
            && self.seed.class_factory_generation == factory_generation
            && self.config.diagnostic_report_configuration(state.db) == self.configuration
            && self.seed.path_admission.uri() == &self.uri
            && original_readonly_analysis_matches_input(
                &self.snapshot.text,
                &self.seed.analysis,
                &self.input,
            )
    }
}

impl ReadonlySourceDocument {
    fn matches_current_snapshot(
        &self,
        uri_generations: &std::collections::HashMap<Uri, u64>,
        open: &std::collections::HashMap<Uri, super::DocumentState>,
        db: &tcl_lsp_db::TclDatabase,
        files: &std::collections::HashMap<Uri, tcl_lsp_db::SourceFile>,
        published: &std::collections::HashMap<Uri, super::PublishedSourcePath>,
    ) -> bool {
        self.publication.matches_open_documents(open)
            && uri_generations.get(&self.uri).copied().unwrap_or_default()
                == self.publication.uri_generation
            && tcl_lsp_db::database_revision(db) == self.database_revision
            && files.get(&self.uri) == Some(&self.file)
            && published.get(&self.uri).is_some_and(|retained| {
                retained.file == self.file && retained.admission.as_ref() == self.admission.as_ref()
            })
            && self.file.path(db).as_deref() == self.admission.analyser_path()
            && self.file.text(db).as_str() == self.source.as_ref()
            && self.file.external_call_sites(db) == &self.external_call_sites
            && self.file.dialect(db) == &self.publication.document.dialect
            && tcl_lsp_db::document_analysis_input(db, self.file, self.config)
                .is_ok_and(|input| input.as_ref() == self.input.as_ref())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod logical_hierarchy_tests;

#[cfg(test)]
mod conditional_navigation_publication_tests;

#[cfg(test)]
mod logical_call_wire_tests;

#[cfg(test)]
mod logical_call_provider_generation_tests;

#[cfg(test)]
mod logical_lens_wire_tests;

#[cfg(test)]
mod logical_call_backend_session_tests;

#[cfg(test)]
mod workspace_symbol_input_lease_tests;

#[cfg(test)]
mod workspace_symbol_original_admission_tests;

#[cfg(test)]
mod code_action_source_version_tests;

#[cfg(test)]
mod coarse_semantic_source_tests;

// Source-only retained interpretation owners and publication leases.
pub(crate) mod source_views;

#[cfg(test)]
mod factory_source_custody_tests;

#[cfg(test)]
mod saved_source_allocation_tests;
