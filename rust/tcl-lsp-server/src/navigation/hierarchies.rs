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

//! Type-hierarchy walks and cross-document call hierarchy.

#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use std::str::FromStr;
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use tcl_compiler::analyser::AnalysisResult;
use tcl_lsp_core::type_hierarchy as core_type_hierarchy;
#[cfg(test)]
use tcl_lsp_core::workspace_index as core_workspace_index;
#[cfg(test)]
use tcl_lsp_core::{call_hierarchy as core_call_hierarchy, definition as core_definition};
use tower_lsp_server::jsonrpc;
#[cfg(test)]
use tower_lsp_server::ls_types::{CallHierarchyItem, CallHierarchyOutgoingCall, Position, Range};
use tower_lsp_server::ls_types::{SymbolKind, TypeHierarchyItem, Uri};

use crate::documents::DocumentSourceResult;
use crate::protocol::lift_lsp_range;

use crate::Backend;

#[cfg(test)]
type CrossDocumentSourceRows = Vec<(Uri, Arc<str>, String)>;

impl Backend {
    /// Cross-file super/subtype targets for the type-hierarchy walk, resolved
    /// against the workspace class index.  Each entry is
    /// `(document-uri, qualified-name, name-span)` for a class defined in some
    /// (possibly other) document.
    ///
    /// For **subtypes**, the index resolves every class declaring `class_name`
    /// as a direct owner-aware superclass/mixin.  For **supertypes**, the
    /// written super/mixin names are read from the definition matching
    /// `selected_uri` (the class under the cursor), so a homonymous `::C` in
    /// an unrelated file can't contribute its own parents and the result is
    /// deterministic; the union over every homonym is used only as a fallback
    /// when the selected document isn't indexed.
    /// Shared supertype (`subtypes = false`) / subtype (`subtypes = true`)
    /// walk for one type-hierarchy item, resolving against the item's
    /// document.  Returns an empty list (not an error) when the document or
    /// class cannot be resolved, so the editor's hierarchy view degrades
    /// cleanly.
    pub(in super::super) async fn type_hierarchy_walk(
        &self,
        item: TypeHierarchyItem,
        subtypes: bool,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(Some(Vec::new()));
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            let Some(identity) = self
                .resolve_original_declaration_data(item.data.as_ref(), &uri, &doc.text, &analysis)
                .await
            else {
                return Ok(Some(Vec::new()));
            };
            let documents = self.original_declaration_documents(&uri).await?;
            let items = crate::rt::spawn_blocking(move || {
                let inventory = documents
                    .iter()
                    .map(
                        |(uri, source, analysis)| core_type_hierarchy::OriginalHierarchyDocument {
                            uri,
                            source,
                            analysis,
                        },
                    )
                    .collect::<Vec<_>>();
                core_type_hierarchy::related_from_inventory(&identity, &inventory, subtypes)
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("type_hierarchy worker panicked: {err}").into(),
                data: None,
            })?;
            return Ok(Some(self.lift_original_type_hierarchy_items(items).await?));
        }
        Ok(Some(
            self.readonly_logical_hierarchy_walk(&item, subtypes)
                .await?,
        ))
    }

    pub(in super::super) async fn lift_original_type_hierarchy_items(
        &self,
        items: Vec<core_type_hierarchy::TypeHierarchyItem>,
    ) -> DocumentSourceResult<Vec<TypeHierarchyItem>> {
        let mut lifted = Vec::new();
        for item in items {
            let Some(identity) = item.original_declaration.as_ref() else {
                continue;
            };
            let Ok(uri) = identity.uri().parse::<Uri>() else {
                continue;
            };
            let Some(doc) = self.read_document(&uri).await? else {
                continue;
            };
            let analysis = self
                .analysis_for(&uri, doc.text.clone(), doc.dialect)
                .await?;
            let Some(data) = self
                .retain_original_declaration_data(&uri, &doc.text, &analysis, Some(identity))
                .await
            else {
                continue;
            };
            lifted.push(TypeHierarchyItem {
                name: item.name,
                kind: SymbolKind::CLASS,
                tags: None,
                detail: item.detail,
                uri,
                range: lift_lsp_range(item.range),
                selection_range: lift_lsp_range(item.selection_range),
                data: Some(data),
            });
        }
        Ok(lifted)
    }

    /// Conditional incoming source calls in sibling documents. The actual
    /// selected declaration supplies procedure/member kind, original name and
    /// member side; reporting labels cannot reconstruct this target.
    #[cfg(test)]
    pub(in super::super) async fn cross_document_incoming_calls(
        &self,
        current_uri: &Uri,
        target: &core_call_hierarchy::LogicalCallTarget<'_>,
    ) -> DocumentSourceResult<Vec<(Uri, core_call_hierarchy::IncomingCall)>> {
        // Collect (uri, source, dialect) for every document *other
        // than* the current one: first the open buffers, then the
        // indexed-but-unopened files the folder scan discovered
        // (read from disk).  Without the latter, callers living in
        // files the editor never opened would be missing from the
        // incoming-call results even though they are indexed for
        // every other cross-document feature.
        let (_open_uris, docs) = self.collect_cross_document_sources(current_uri).await?;
        let mut out = Vec::new();
        for (doc_uri, source, dialect) in docs {
            let analysis = self
                .analysis_for(&doc_uri, Arc::clone(&source), dialect)
                .await?;
            match target.declaration() {
                core_call_hierarchy::LogicalCallDeclaration::Member {
                    definition: method,
                    side,
                    ..
                } => {
                    let receivers: Vec<(String, bool)> = {
                        let index = self.workspace_index.read().await;
                        analysis
                            .all_classes
                            .keys()
                            .filter_map(|receiver| {
                                let chain = |access| match side {
                                    tcl_compiler::analyser::MemberSide::Instance => {
                                        index.method_dispatch_chain(receiver, &method.name, access)
                                    }
                                    tcl_compiler::analyser::MemberSide::ClassObject => index
                                        .class_method_dispatch_chain(
                                            receiver,
                                            &method.name,
                                            access,
                                        ),
                                };
                                let internal = chain(core_workspace_index::MethodAccess::Internal)
                                    .first()
                                    .is_some_and(|provider| {
                                        index.logical_member_provider_matches_target(
                                            current_uri.as_str(),
                                            provider,
                                            target,
                                        )
                                    });
                                internal.then(|| {
                                    let external =
                                        chain(core_workspace_index::MethodAccess::External)
                                            .first()
                                            .is_some_and(|provider| {
                                                index.logical_member_provider_matches_target(
                                                    current_uri.as_str(),
                                                    provider,
                                                    target,
                                                )
                                            });
                                    (receiver.clone(), external)
                                })
                            })
                            .collect()
                    };
                    for (receiver, external) in receivers {
                        out.extend(
                            core_call_hierarchy::incoming_calls_for_logical_member_in_class(
                                &source, &analysis, &receiver, target, external,
                            )
                            .into_iter()
                            .map(|call| (doc_uri.clone(), call)),
                        );
                    }
                }
                core_call_hierarchy::LogicalCallDeclaration::Procedure {
                    qualified,
                    definition,
                } => {
                    out.extend(
                        core_call_hierarchy::incoming_calls_for_target(
                            &source,
                            &analysis,
                            &definition.name,
                            qualified,
                            None,
                        )
                        .into_iter()
                        .map(|call| (doc_uri.clone(), call)),
                    );
                }
            }
        }
        Ok(out)
    }

    #[cfg(test)]
    async fn collect_cross_document_sources(
        &self,
        current_uri: &Uri,
    ) -> DocumentSourceResult<(HashSet<Uri>, CrossDocumentSourceRows)> {
        let mut open_uris: HashSet<Uri> = HashSet::new();
        let mut docs: Vec<(Uri, Arc<str>, String)> = {
            let store = self.documents.lock("cross_document_incoming_calls").await;
            store
                .iter()
                .filter(|(u, _)| *u != current_uri)
                .map(|(u, d)| {
                    open_uris.insert(u.clone());
                    (u.clone(), d.text.clone(), d.dialect.clone())
                })
                .collect()
        };
        let indexed = self.workspace_index.read().await.document_uris();
        for uri_str in indexed {
            let Ok(uri) = Uri::from_str(&uri_str) else {
                continue;
            };
            if uri == *current_uri || open_uris.contains(&uri) {
                continue;
            }
            if let Some(doc) = self.read_document(&uri).await? {
                docs.push((uri, doc.text, doc.dialect));
            }
        }
        Ok((open_uris, docs))
    }

    /// Cross-document outgoing calls from the queried proc /
    /// method: the call sites inside its body whose callee is
    /// defined in a *sibling* document.  The local
    /// [`core_call_hierarchy::outgoing_calls`] pass only resolves
    /// callees in the current document; this resolves the
    /// remaining heads against the workspace index, builds a `to`
    /// item pointing at the sibling definition (URI + name span →
    /// range against that document's source), and keeps the call
    /// ranges from the *current* document.
    #[cfg(test)]
    pub(in super::super) async fn cross_document_outgoing_calls(
        &self,
        current_uri: &Uri,
        source: &str,
        dialect: &'static tcl_dialect::DialectProfile,
        item: &core_call_hierarchy::CallHierarchyItem,
        analysis: &AnalysisResult,
    ) -> DocumentSourceResult<Vec<CallHierarchyOutgoingCall>> {
        let unresolved = {
            let source = source.to_owned();
            let item = item.clone();
            let analysis = analysis.clone();
            let exports = self.export_snapshot().await;
            let uri_key = current_uri.as_str().to_owned();
            crate::rt::spawn_blocking(move || {
                core_call_hierarchy::unresolved_outgoing_calls_in_program(
                    &source,
                    dialect,
                    &item,
                    &analysis,
                    core_definition::CallResolution::document_only().in_program(
                        core_definition::ProgramExports {
                            uri: &uri_key,
                            oracle: exports.as_ref(),
                        },
                    ),
                )
            })
            .await
            .unwrap_or_default()
        };
        let mut out: Vec<CallHierarchyOutgoingCall> = Vec::new();
        for occurrence in unresolved
            .into_iter()
            .flat_map(|call| call.logical_occurrences)
        {
            let Some(request) = occurrence.candidates else {
                continue;
            };
            let selected = {
                let index = self.workspace_index.read().await;
                let candidates = index.logical_command_candidates(&request, current_uri.as_str());
                let [candidate] = candidates.as_slice() else {
                    continue;
                };
                candidate.clone()
            };
            let target_image = selected.clone();
            let (target_uri, qualified, name_span, detail) = (
                selected.uri().to_owned(),
                selected.qualified_name().to_owned(),
                selected.name_span(),
                selected.detail().map(str::to_owned),
            );
            // Convert the sibling definition's name span to a range
            // against that document's source.
            let Ok(parsed) = Uri::from_str(&target_uri) else {
                continue;
            };
            let Some(target_doc) = self.read_document(&parsed).await? else {
                continue;
            };
            if !target_image.matches_source(&target_doc.text)
                || !self
                    .workspace_index
                    .read()
                    .await
                    .logical_command_candidate_is_current(&target_image)
            {
                continue;
            }
            let line_index = target_doc.line_index.clone();
            let start = line_index.position_at_utf16(name_span.start(), &target_doc.text);
            let end = line_index.position_at_utf16(name_span.end(), &target_doc.text);
            let name_range = Range {
                start: Position {
                    line: start.line,
                    character: start.character.get(),
                },
                end: Position {
                    line: end.line,
                    character: end.character.get(),
                },
            };
            let outgoing = CallHierarchyOutgoingCall {
                to: CallHierarchyItem {
                    name: qualified,
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    detail,
                    uri: parsed,
                    range: name_range,
                    selection_range: name_range,
                    data: None,
                },
                from_ranges: vec![lift_lsp_range(occurrence.range)],
            };
            if let Some(previous) = out.iter_mut().find(|previous| {
                previous.to.uri == outgoing.to.uri
                    && previous.to.selection_range == outgoing.to.selection_range
            }) {
                previous.from_ranges.extend(outgoing.from_ranges);
            } else {
                out.push(outgoing);
            }
        }
        Ok(out)
    }
}
