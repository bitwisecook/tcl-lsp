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

//! Reference reports for an original declaration: the source occurrences, their coverage and the locations lifted from them.

use super::super::{Backend, DocumentSourceResult};
use super::{
    OriginalReadonlyReferenceDocument, OriginalReadonlyReferenceReport, ReadonlyTransaction,
    retry_readonly_attempt, single_document_coverage,
};
use std::ops::ControlFlow;
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::original_declaration::{OriginalDeclarationIdentity, ReadonlyReferenceCoverage};
use tower_lsp_server::ls_types::{Location, Position, Range, Uri};

impl Backend {
    pub(super) async fn finish_readonly_reference_report(
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

    /// Source-only occurrences join the original declaration in its real URI.
    /// All participants retain their actual `SourceFile` and full checked input;
    /// this report cannot be consumed as call edges or editable occurrences.
    pub(in super::super) async fn readonly_original_declaration_reference_report(
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
        super::super::dedup_span_targets(&mut targets);
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

    pub(in super::super) async fn readonly_original_method_reference_locations_at(
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
