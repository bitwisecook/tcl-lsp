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

//! Object-method targets for readonly navigation and hover.

use std::ops::ControlFlow;
#[cfg(test)]
use std::str::FromStr;

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::hover::Hover as CoreHover;
use tcl_lsp_core::original_indexed_location::OriginalIndexedSourceLocation;
use tcl_lsp_core::{hover as core_hover, workspace_index as core_workspace_index};
#[cfg(test)]
use tower_lsp_server::ls_types::Range;
use tower_lsp_server::ls_types::{Location, Position, Uri};

use crate::documents::{DocumentPathAdmission, DocumentSourceResult};
use crate::{Backend, DocumentState};

#[cfg(test)]
use super::dedup_locations;
use super::{ReadonlyDeclarationLocation, ReadonlyDeclarationTarget};

impl Backend {
    /// Shared typed OO selection for readonly editor consumers. An exact
    /// retained operand with unavailable ownership is terminal; declaration
    /// candidates do not issue native dispatch or source editing authority.
    pub(in super::super) async fn original_method_candidate_at(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        position: Position,
    ) -> DocumentSourceResult<
        std::ops::ControlFlow<Option<tcl_lsp_core::method_symbol::OriginalMethodCandidate>>,
    > {
        use std::ops::ControlFlow;
        let query = match tcl_lsp_core::method_symbol::select(
            source,
            analysis,
            position.line,
            position.character,
        ) {
            ControlFlow::Continue(()) => return Ok(ControlFlow::Continue(())),
            ControlFlow::Break(None) => return Ok(ControlFlow::Break(None)),
            ControlFlow::Break(Some(query)) => query,
        };
        let _rehoming = self.rehomed_index_guard().await?;
        let index = self.workspace_index.read().await;
        Ok(tcl_lsp_core::method_symbol::candidate(
            &index,
            uri.as_str(),
            &query,
        ))
    }

    pub(in super::super) async fn original_method_source_target(
        &self,
        caller: &(Uri, DocumentState, u64),
        analysis: &AnalysisResult,
        position: Position,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Option<ReadonlyDeclarationTarget>>> {
        use std::ops::ControlFlow;
        let Some(input) = analysis.resolved_input.as_ref() else {
            return Ok(ControlFlow::Break(None));
        };
        let query = match tcl_lsp_core::method_symbol::select(
            &caller.1.text,
            analysis,
            position.line,
            position.character,
        ) {
            ControlFlow::Continue(()) => return Ok(ControlFlow::Continue(())),
            ControlFlow::Break(None) => return Ok(ControlFlow::Break(None)),
            ControlFlow::Break(Some(query)) => query,
        };
        let candidate = {
            let _rehoming = self.rehomed_index_guard().await?;
            let index = self.workspace_index.read().await;
            if index.source_analysis_input(caller.0.as_str()) != Some(input) {
                return Ok(ControlFlow::Break(None));
            }
            match tcl_lsp_core::method_symbol::candidate(&index, caller.0.as_str(), &query) {
                ControlFlow::Continue(()) => return Ok(ControlFlow::Continue(())),
                ControlFlow::Break(None) => return Ok(ControlFlow::Break(None)),
                ControlFlow::Break(Some(candidate)) => candidate,
            }
        };
        let Ok(uri) = candidate.uri().parse::<Uri>() else {
            return Ok(ControlFlow::Break(None));
        };
        let generation = self.live_publication_uri_generation(&uri);
        let Some(document) = self.read_document(&uri).await? else {
            return Ok(ControlFlow::Break(None));
        };
        if candidate.declaration_image() != &tcl_lexer::SourceImage::document(&document.text) {
            return Ok(ControlFlow::Break(None));
        }
        let current = self
            .analysis_for(&uri, document.text.clone(), document.dialect.clone())
            .await?;
        if current.resolved_input.as_ref() != Some(input) {
            return Ok(ControlFlow::Break(None));
        }
        let Some(identity) =
            tcl_lsp_core::original_declaration::OriginalDeclarationIdentity::for_member_candidate(
                uri.as_str(),
                &document.text,
                &current,
                &candidate,
            )
        else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(context) =
            core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(&current)
        else {
            return Ok(ControlFlow::Break(None));
        };
        let Some(location) = OriginalIndexedSourceLocation::from_declaration(&context, &identity)
        else {
            return Ok(ControlFlow::Break(None));
        };
        Ok(ControlFlow::Break(Some(ReadonlyDeclarationTarget {
            uri,
            document,
            analysis: current,
            generation,
            location: ReadonlyDeclarationLocation::OriginalMethod {
                location: Box::new(location),
                caller_uri: caller.0.as_str().to_owned(),
                input: input.clone(),
                query: Box::new(query),
                candidate: Box::new(candidate),
            },
        })))
    }

    pub(in super::super) async fn original_method_locations_for_snapshot(
        &self,
        caller: (Uri, DocumentState, u64),
        analysis: &AnalysisResult,
        position: Position,
        epoch: u64,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Vec<Location>>> {
        use std::ops::ControlFlow;
        let target = match self
            .original_method_source_target(&caller, analysis, position)
            .await?
        {
            ControlFlow::Continue(()) => return Ok(ControlFlow::Continue(())),
            ControlFlow::Break(None) => return Ok(ControlFlow::Break(Vec::new())),
            ControlFlow::Break(Some(target)) => target,
        };
        Ok(ControlFlow::Break(
            self.resolve_current_declaration_locations(caller, vec![target], epoch)
                .await?,
        ))
    }

    pub(in super::super) async fn original_pattern_hover_for_snapshot(
        &self,
        caller: (Uri, DocumentState, u64),
        analysis: &AnalysisResult,
        position: Position,
        epoch: u64,
    ) -> DocumentSourceResult<Option<CoreHover>> {
        // An authentic query rejected by the workspace or target-source join
        // stays terminal. Only the local selector's own abstention can ask
        // the independently sealed Registry operand provider.
        if !matches!(
            tcl_lsp_core::method_symbol::select(
                &caller.1.text,
                analysis,
                position.line,
                position.character,
            ),
            ControlFlow::Break(None)
        ) {
            return Ok(None);
        }
        let Some(input) = analysis.resolved_input.as_ref() else {
            return Ok(None);
        };
        let context = input.context_registry();
        let Some(source) = self.class_type_source(&caller.0, caller.1, caller.2).await else {
            return Ok(None);
        };
        let Some(saved) = self.readonly_saved_sources(&[&source]).await? else {
            return Ok(None);
        };
        let _inputs = self.analyser_inputs_gate.read().await;
        let _rehoming = self.rehoming_gate.lock().await;
        let documents = self.documents.lock("original_pattern_hover_publish").await;
        if self.diag_inputs_epoch() != epoch
            || !source.matches_open_documents(&documents)
            || self.live_publication_uri_generation(&source.uri) != source.uri_generation
        {
            return Ok(None);
        }
        let Some(admission) = source.document.path_admission.as_ref() else {
            return Ok(None);
        };
        admission.check_for(&source.uri, self.store.as_ref())?;
        let Ok(index) = self.workspace_index.try_read() else {
            return Ok(None);
        };
        if index.source_analysis_input(source.uri.as_str()) != Some(input) {
            return Ok(None);
        }
        let selected = saved
            .iter()
            .map(|saved| (saved.admission.as_ref(), saved.bytes.as_slice()))
            .collect::<Vec<_>>();
        DocumentPathAdmission::with_current_saved_sources(&selected, self.store.as_ref(), || {
            core_hover::registry_pattern_format_hover_in_workspace(
                &source.document.text,
                position.line,
                position.character,
                analysis,
                context.commands(),
                &index,
            )
        })
    }

    pub(in super::super) async fn original_method_hover_for_snapshot(
        &self,
        caller: (Uri, DocumentState, u64),
        analysis: &AnalysisResult,
        position: Position,
        epoch: u64,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Option<CoreHover>>> {
        use std::ops::ControlFlow;
        let target = match self
            .original_method_source_target(&caller, analysis, position)
            .await?
        {
            ControlFlow::Continue(()) => return Ok(ControlFlow::Continue(())),
            ControlFlow::Break(None) => {
                return Ok(ControlFlow::Break(
                    self.original_pattern_hover_for_snapshot(caller, analysis, position, epoch)
                        .await?,
                ));
            }
            ControlFlow::Break(Some(target)) => target,
        };
        let ReadonlyDeclarationLocation::OriginalMethod { candidate, .. } = &target.location else {
            return Ok(ControlFlow::Break(None));
        };
        let candidate = candidate.clone();
        let mut rendered = self
            .with_current_declaration_sources(
                caller,
                vec![target],
                epoch,
                |_, document, analysis, _| {
                    core_hover::declared_member_hover(&document.text, analysis, &candidate)
                        .or_else(|| candidate.hover())
                },
            )
            .await?;
        Ok(ControlFlow::Break(rendered.pop()))
    }

    #[cfg(test)]
    pub(in super::super) async fn original_method_locations_at(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        position: Position,
        navigation: tcl_lsp_core::method_symbol::OriginalMethodNavigation,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Vec<Location>>> {
        if navigation == tcl_lsp_core::method_symbol::OriginalMethodNavigation::Declaration {
            let epoch = self.diag_inputs_epoch();
            let generation = self.live_publication_uri_generation(uri);
            let Some(document) = self.read_document(uri).await? else {
                return Ok(std::ops::ControlFlow::Break(Vec::new()));
            };
            if document.text.as_ref() != source {
                return Ok(std::ops::ControlFlow::Break(Vec::new()));
            }
            return Box::pin(self.original_method_locations_for_snapshot(
                (uri.clone(), document, generation),
                analysis,
                position,
                epoch,
            ))
            .await;
        }
        let target = match self
            .original_method_candidate_at(uri, source, analysis, position)
            .await?
        {
            ControlFlow::Continue(()) => return Ok(ControlFlow::Continue(())),
            ControlFlow::Break(None) => return Ok(ControlFlow::Break(Vec::new())),
            ControlFlow::Break(Some(target)) => target,
        };
        let rows = match navigation {
            tcl_lsp_core::method_symbol::OriginalMethodNavigation::References {
                include_declaration,
            } => {
                let _rehoming = self.rehomed_index_guard().await?;
                self.workspace_index
                    .read()
                    .await
                    .original_method_reference_rows(&target, include_declaration)
            }
            tcl_lsp_core::method_symbol::OriginalMethodNavigation::Declaration => {
                vec![(
                    target.uri().to_owned(),
                    target.declaration_span(),
                    target.declaration_image().clone(),
                    target.declaration_config(),
                )]
            }
        };
        let mut locations = Vec::new();
        for (owner, span, image, config) in rows {
            let Ok(uri) = Uri::from_str(&owner) else {
                continue;
            };
            let Some(document) = self.read_document(&uri).await? else {
                continue;
            };
            if image != tcl_lexer::SourceImage::document(&document.text) {
                continue;
            }
            let owner_analysis = self
                .analysis_for(&uri, document.text.clone(), document.dialect.clone())
                .await?;
            if !owner_analysis.matches_original_source_image(&image, config) {
                continue;
            }
            let start = document
                .line_index
                .position_at_utf16(span.start(), &document.text);
            let end = document
                .line_index
                .position_at_utf16(span.end(), &document.text);
            locations.push(Location {
                uri,
                range: Range {
                    start: Position::new(start.line, start.character.get()),
                    end: Position::new(end.line, end.character.get()),
                },
            });
        }
        dedup_locations(&mut locations);
        Ok(ControlFlow::Break(locations))
    }

    #[cfg(test)]
    pub(in super::super) async fn original_method_hover_at(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        position: Position,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Option<CoreHover>>> {
        let epoch = self.diag_inputs_epoch();
        let generation = self.live_publication_uri_generation(uri);
        let Some(document) = self.read_document(uri).await? else {
            return Ok(std::ops::ControlFlow::Break(None));
        };
        if document.text.as_ref() != source {
            return Ok(std::ops::ControlFlow::Break(None));
        }
        Box::pin(self.original_method_hover_for_snapshot(
            (uri.clone(), document, generation),
            analysis,
            position,
            epoch,
        ))
        .await
    }
}
