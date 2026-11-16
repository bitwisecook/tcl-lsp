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
#[cfg(test)]
use tcl_lsp_core::original_declaration::OriginalDeclarationIdentity;
use tcl_lsp_core::original_declaration::ReadonlyReferenceCoverage;
#[cfg(test)]
use tower_lsp_server::ls_types::Range;
use tower_lsp_server::ls_types::{Location, Position, Uri};

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

mod capture;
mod coarse_semantic_source;
mod code_action_source;
mod declaration_references;
mod logical_calls;
mod logical_hierarchy;
mod namespace_advice;
mod publication_transactions;
mod snapshot_currency;
mod workspace_symbols;

#[cfg(test)]
use logical_calls::LogicalCallPublicationStamp;
#[cfg(test)]
use logical_calls::logical_call_inventory;
use snapshot_currency::original_readonly_analysis_matches_input;

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
