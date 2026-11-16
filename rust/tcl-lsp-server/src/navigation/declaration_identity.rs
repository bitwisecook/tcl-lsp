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

//! The retained declaration identity call hierarchy, type hierarchy and reference lenses share.

use std::sync::Arc;

use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::{call_hierarchy as core_call_hierarchy, definition as core_definition};
use tower_lsp_server::ls_types::{CallHierarchyItem, Position, SymbolKind, Uri};
#[cfg(test)]
use tower_lsp_server::ls_types::{Location, Range};

#[cfg(test)]
use super::dedup_span_targets;
use crate::diagnostics::is_apl_source;
use crate::documents::DocumentSourceResult;
use crate::protocol::lift_lsp_range;
use crate::uri_norm::DocumentPath as _;
use crate::workspace::is_tcl_source;
use crate::{Backend, DocumentState};

impl Backend {
    pub(in super::super) async fn retain_original_declaration_data(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
        identity: Option<&tcl_lsp_core::original_declaration::OriginalDeclarationIdentity>,
    ) -> Option<serde_json::Value> {
        let identity = identity?.owned_by(uri.as_str(), source, analysis)?;
        let handle = self
            .original_declaration_registry
            .lock()
            .await
            .issue(identity)?;
        Some(serde_json::json!({ "originalDeclaration": handle }))
    }

    pub(in super::super) async fn resolve_original_declaration_data(
        &self,
        data: Option<&serde_json::Value>,
        uri: &Uri,
        source: &str,
        analysis: &tcl_compiler::analyser::AnalysisResult,
    ) -> Option<tcl_lsp_core::original_declaration::OriginalDeclarationIdentity> {
        let handle = serde_json::from_value(data?.get("originalDeclaration")?.clone()).ok()?;
        self.original_declaration_registry.lock().await.resolve(
            handle,
            uri.as_str(),
            source,
            analysis,
        )
    }

    /// Decide source participation before querying the Tcl declaration owner.
    /// File and language identity only classify the document; each admitted
    /// source retains its independently selected dialect and full currency.
    pub(in super::super) fn original_declaration_document_participates(
        uri: &Uri,
        document: &DocumentState,
    ) -> Option<bool> {
        if is_apl_source(uri, &document.language_id) {
            return Some(false);
        }
        let environment = tcl_registry::model::resolve_known_environment(&document.dialect);
        if environment
            .as_ref()
            .is_some_and(|environment| environment.definition.core.is_none())
        {
            return Some(false);
        }
        let source_file = uri.document_path().is_some_and(|path| is_tcl_source(&path));
        let source_language = tcl_registry::model::resolve_language_id(&document.language_id)
            .is_some_and(|environment| environment.definition.core.is_some());
        if !source_file && !source_language {
            return Some(false);
        }
        // A relevant source with an unknown dialect cannot be discarded:
        // it might contain an earlier occupied header or duplicate provider.
        environment.map(|environment| environment.definition.core.is_some())
    }

    pub(in super::super) async fn original_declaration_documents(
        &self,
        selected: &Uri,
    ) -> DocumentSourceResult<
        Vec<(
            String,
            Arc<str>,
            Arc<tcl_compiler::analyser::AnalysisResult>,
        )>,
    > {
        let mut uris = {
            let index = self.workspace_index.read().await;
            let mut owners = index.document_uris();
            owners.extend(
                index
                    .original_class_declarations()
                    .map(|(uri, _)| uri.to_owned()),
            );
            owners.extend(
                index
                    .original_procedure_declarations()
                    .map(|(uri, _)| uri.to_owned()),
            );
            owners
        };
        uris.extend(
            self.documents
                .lock("original_declaration_documents")
                .await
                .keys()
                .map(|uri| uri.as_str().to_owned()),
        );
        uris.push(selected.as_str().to_owned());
        uris.sort();
        uris.dedup();
        let mut documents = Vec::new();
        for owner in uris {
            let Ok(uri) = owner.parse::<Uri>() else {
                return Ok(Vec::new());
            };
            let Some(doc) = self.read_document(&uri).await? else {
                return Ok(Vec::new());
            };
            match Self::original_declaration_document_participates(&uri, &doc) {
                Some(true) => {}
                Some(false) => continue,
                None => return Ok(Vec::new()),
            }
            let analysis = self
                .analysis_for(&uri, doc.text.clone(), doc.dialect)
                .await?;
            let config = analysis.body_lexer_config;
            if !config.is_some_and(|config| {
                analysis.matches_original_source_image(
                    &tcl_lexer::SourceImage::document(&doc.text),
                    config,
                )
            }) {
                return Ok(Vec::new());
            }
            documents.push((owner, doc.text, analysis));
        }
        Ok(documents)
    }

    pub(in super::super) async fn lift_original_call_item(
        &self,
        item: core_call_hierarchy::CallHierarchyItem,
    ) -> DocumentSourceResult<Option<CallHierarchyItem>> {
        let Some(identity) = item.identity.as_ref() else {
            return Ok(None);
        };
        let Some(uri) = identity.uri().parse::<Uri>().ok() else {
            return Ok(None);
        };
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect)
            .await?;
        let Some(data) = self
            .retain_original_declaration_data(&uri, &doc.text, &analysis, Some(identity))
            .await
        else {
            return Ok(None);
        };
        let kind = match identity.role() {
            tcl_lsp_core::original_declaration::OriginalDeclarationRole::Class => SymbolKind::CLASS,
            tcl_lsp_core::original_declaration::OriginalDeclarationRole::Method(_) => {
                SymbolKind::METHOD
            }
            _ => SymbolKind::FUNCTION,
        };
        Ok(Some(CallHierarchyItem {
            name: item.name,
            kind,
            tags: None,
            detail: item.detail,
            uri,
            range: lift_lsp_range(item.range),
            selection_range: lift_lsp_range(item.selection_range),
            data: Some(data),
        }))
    }

    #[cfg(test)]
    pub(in super::super) async fn original_declaration_reference_locations(
        &self,
        identity: &tcl_lsp_core::original_declaration::OriginalDeclarationIdentity,
        selected: &Uri,
        include_declaration: bool,
    ) -> DocumentSourceResult<Vec<Location>> {
        let inputs_epoch = self.diag_inputs_epoch();
        let documents = self.original_declaration_documents(selected).await?;
        let inventory = documents
            .iter()
            .map(|(uri, source, analysis)| {
                tcl_lsp_core::original_call_hierarchy::OriginalCallDocument {
                    uri,
                    source,
                    analysis,
                }
            })
            .collect::<Vec<_>>();
        let mut targets = {
            let workspace = self.workspace_index.read().await;
            tcl_lsp_core::original_call_hierarchy::references(
                identity,
                &inventory,
                Some(&workspace),
            )
        };
        if include_declaration
            && documents
                .iter()
                .any(|(owner, source, analysis)| identity.is_current(owner, source, analysis))
        {
            targets.push((identity.uri().to_owned(), identity.span()));
        }
        dedup_span_targets(&mut targets);
        // Every span belongs to its independently current source. An await
        // can change an owner; no old extent is lifted against new text.
        for (owner, source, analysis) in &documents {
            let Ok(uri) = owner.parse::<Uri>() else {
                return Ok(Vec::new());
            };
            let Some(current) = self.read_document(&uri).await? else {
                return Ok(Vec::new());
            };
            if current.text != *source
                || !analysis.body_lexer_config.is_some_and(|config| {
                    analysis.matches_original_source_image(
                        &tcl_lexer::SourceImage::document(&current.text),
                        config,
                    )
                })
            {
                return Ok(Vec::new());
            }
        }
        if self.diag_inputs_epoch() != inputs_epoch {
            return Ok(Vec::new());
        }
        Ok(targets
            .into_iter()
            .filter_map(|(owner, span)| {
                let document = documents.iter().find(|(uri, _, _)| uri == &owner)?;
                let uri = owner.parse::<Uri>().ok()?;
                let index = tcl_lexer::LineIndex::new_lsp(&document.1);
                let start = index.position_at_utf16(span.start(), &document.1);
                let end = index.position_at_utf16(span.end(), &document.1);
                Some(Location {
                    uri,
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
                })
            })
            .collect())
    }

    /// Select the same independently owned source identity used by call
    /// hierarchy and reference lenses. A missing original lookup is terminal.
    pub(in super::super) async fn original_declaration_identity_at(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        position: Position,
    ) -> DocumentSourceResult<Option<tcl_lsp_core::original_declaration::OriginalDeclarationIdentity>>
    {
        let inputs_epoch = self.diag_inputs_epoch();
        let Some(config) = analysis.body_lexer_config else {
            return Ok(None);
        };
        if !analysis
            .matches_original_source_image(&tcl_lexer::SourceImage::document(source), config)
        {
            return Ok(None);
        }
        if core_definition::original_autoload_request_at(
            source,
            analysis,
            position.line,
            position.character,
        )
        .is_some()
            && !self
                .ensure_original_library_indexed(uri, source, analysis, position)
                .await?
        {
            return Ok(None);
        }
        let documents = self.original_declaration_documents(uri).await?;
        let Some(selected) = documents.iter().find(|(owner, _, _)| owner == uri.as_str()) else {
            return Ok(None);
        };
        if selected.1.as_ref() != source || selected.2.body_lexer_config != Some(config) {
            return Ok(None);
        }
        let inventory = documents
            .iter()
            .map(|(owner, source, analysis)| {
                tcl_lsp_core::original_call_hierarchy::OriginalCallDocument {
                    uri: owner,
                    source,
                    analysis,
                }
            })
            .collect::<Vec<_>>();
        let workspace = self.workspace_index.read().await;
        let mut selected = tcl_lsp_core::original_call_hierarchy::prepare(
            uri.as_str(),
            position.line,
            position.character,
            &inventory,
            Some(&workspace),
        )
        .into_iter();
        let Some(identity) = (match selected.next() {
            Some(__source_value) => __source_value,
            None => return Ok(None),
        })
        .identity
        else {
            return Ok(None);
        };
        Ok(
            (selected.next().is_none() && self.diag_inputs_epoch() == inputs_epoch)
                .then_some(identity),
        )
    }
}
