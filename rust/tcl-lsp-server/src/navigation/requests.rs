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

//! The navigation request handlers: definitions, references, hierarchies, hover and signature help.

use crate::{
    Arc, Backend, CallHierarchyIncomingCall, CallHierarchyIncomingCallsParams, CallHierarchyItem,
    CallHierarchyOutgoingCall, CallHierarchyOutgoingCallsParams, CallHierarchyPrepareParams,
    ControlFlow, DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams,
    GotoDeclarationParams, GotoDeclarationResponse, GotoDefinitionParams, GotoDefinitionResponse,
    GotoImplementationParams, GotoImplementationResponse, GotoTypeDefinitionParams,
    GotoTypeDefinitionResponse, Hover, HoverParams, LinkedEditingRangeParams, LinkedEditingRanges,
    Location, OriginalIndexedSourceLocation, Range, ReferenceParams, SelectionRange,
    SelectionRangeParams, SignatureHelp, SignatureHelpParams, TypeHierarchyItem,
    TypeHierarchyPrepareParams, TypeHierarchySubtypesParams, TypeHierarchySupertypesParams, Uri,
    core_declaration, core_definition, core_hover, core_implementation, core_linked_editing_range,
    core_references, core_selection_range, core_sig, core_type_definition, core_type_hierarchy,
    dedup_locations, implementation_inventory, jsonrpc, lift_hover, lift_lsp_range,
    lift_signature_help, materialise_selection_range, position_is_command_head,
    position_within_line, type_definition_locations,
};
use std::str::FromStr;

impl Backend {
    pub(crate) async fn serve_goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoDefinitionResponse>> {
        if !self
            .feature_enabled(
                "definition",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        if let std::ops::ControlFlow::Break(locations) = self
            .original_source_view_navigation_route(&uri, pos, None)
            .await?
        {
            return Ok(locations
                .filter(|items| !items.is_empty())
                .map(GotoDefinitionResponse::Array));
        }
        let locations = Box::pin(self.compute_definition(&uri, pos)).await?;
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDefinitionResponse::Array(locations)))
    }

    pub(crate) async fn serve_goto_declaration(
        &self,
        params: GotoDeclarationParams,
    ) -> jsonrpc::Result<Option<GotoDeclarationResponse>> {
        if !self
            .feature_enabled(
                "declaration",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // For a `$var`, go-to-declaration resolves the
        // visible `global` / `variable` / `upvar` scoping statement that
        // declares the name; for any other symbol it defers to plain
        // go-to-definition (handled inside `core_declaration`).
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let text = doc.text.clone();
        let dialect = doc.dialect.clone();
        let ranges = crate::rt::spawn_blocking(move || {
            core_declaration::declaration(
                &text,
                pos.line,
                pos.character,
                tcl_lsp_core::profile_for_dialect(&dialect),
                &analysis,
                &registry,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("declaration worker panicked: {err}").into(),
            data: None,
        })?;
        let locations: Vec<Location> = if ranges.is_empty() {
            // No in-document declaration / definition — try the
            // cross-document index (a sibling-file proc / class), the
            // same fallback go-to-definition uses.
            Box::pin(self.compute_definition(&uri, pos)).await?
        } else {
            ranges
                .into_iter()
                .map(|r| Location {
                    uri: uri.clone(),
                    range: lift_lsp_range(r),
                })
                .collect()
        };
        if locations.is_empty() {
            return Ok(None);
        }
        Ok(Some(GotoDeclarationResponse::Array(locations)))
    }

    pub(crate) async fn serve_goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> jsonrpc::Result<Option<GotoTypeDefinitionResponse>> {
        if !self
            .feature_enabled(
                "typeDefinition",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // Type-definition jumps to the class that types the symbol (a `$obj`
        // instance's class, or a method's owning class), never the plain
        // definition site.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let pos = position_within_line(&doc.text, params.text_document_position_params.position);
        let source_epoch = self.diag_inputs_epoch();
        let caller_generation = self.live_publication_uri_generation(&uri);
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            match self
                .original_method_candidate_at(&uri, &doc.text, &analysis, pos)
                .await?
            {
                ControlFlow::Break(Some(method)) => {
                    let Ok(owner) = method.uri().parse::<Uri>() else {
                        return Ok(None);
                    };
                    let target_generation = self.live_publication_uri_generation(&owner);
                    let Some(target) = self.read_document(&owner).await? else {
                        return Ok(None);
                    };
                    let target_analysis = self
                        .analysis_for(&owner, target.text.clone(), target.dialect.clone())
                        .await?;
                    let Some(class) = method.declaring_class_in(&target.text, &target_analysis)
                    else {
                        return Ok(None);
                    };
                    let Some(identity) =
                        tcl_lsp_core::original_declaration::OriginalDeclarationIdentity::for_class(
                            owner.as_str(),
                            &target.text,
                            &target_analysis,
                            class,
                        )
                    else {
                        return Ok(None);
                    };
                    let Some(context) = tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(&target_analysis) else { return Ok(None); };
                    let Some(location) =
                        OriginalIndexedSourceLocation::from_declaration(&context, &identity)
                    else {
                        return Ok(None);
                    };
                    if location
                        .validated_span(&target.text, &target_analysis)
                        .is_none()
                    {
                        return Ok(None);
                    }
                    let locations = self
                        .resolve_current_original_class_type_location(
                            (uri.clone(), doc.clone(), caller_generation),
                            (owner, target, target_analysis, target_generation),
                            location,
                            source_epoch,
                        )
                        .await?;
                    return Ok(Some(GotoTypeDefinitionResponse::Array(locations)));
                }
                ControlFlow::Break(None) => return Ok(None),
                ControlFlow::Continue(()) => {}
            }
        }
        let text = doc.text.clone();
        let ranges = crate::rt::spawn_blocking(move || {
            core_type_definition::type_definition(&text, pos.line, pos.character, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("type-definition worker panicked: {err}").into(),
            data: None,
        })?;
        if ranges.is_empty() {
            return Ok(None);
        }
        let locations = type_definition_locations(&uri, ranges);
        Ok(Some(GotoTypeDefinitionResponse::Array(locations)))
    }

    pub(crate) async fn serve_goto_implementation(
        &self,
        params: GotoImplementationParams,
    ) -> jsonrpc::Result<Option<GotoImplementationResponse>> {
        if !self
            .feature_enabled(
                "implementation",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // Implementation queries retain exact source owners across the
        // complete current workspace declaration inventory.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            use std::ops::ControlFlow;
            let class = core_type_hierarchy::prepare_for_document(
                uri.as_str(),
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
            )
            .into_iter()
            .next()
            .and_then(|item| item.original_declaration);
            let method = if class.is_some() {
                None
            } else {
                match self
                    .original_method_candidate_at(&uri, &doc.text, &analysis, pos)
                    .await?
                {
                    ControlFlow::Break(Some(method)) => Some(method),
                    ControlFlow::Break(None) | ControlFlow::Continue(()) => return Ok(None),
                }
            };
            let documents = self.original_declaration_documents(&uri).await?;
            let targets = crate::rt::spawn_blocking(move || {
                let inventory = implementation_inventory(&documents);
                if let Some(class) = class {
                    core_type_hierarchy::related_from_inventory(&class, &inventory, true)
                        .into_iter()
                        .filter_map(|item| Some((item.original_declaration?, item.selection_range)))
                        .collect::<Vec<_>>()
                } else if let Some(method) = method {
                    core_implementation::method_implementations_in_inventory(&method, &inventory)
                        .into_iter()
                        .map(|target| (target.identity, target.range))
                        .collect()
                } else {
                    Vec::new()
                }
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("implementation worker panicked: {err}").into(),
                data: None,
            })?;
            let mut locations = Vec::new();
            for (identity, range) in targets {
                let Ok(target_uri) = Uri::from_str(identity.uri()) else {
                    continue;
                };
                let Some(target_doc) = self.read_document(&target_uri).await? else {
                    continue;
                };
                let target_analysis = self
                    .analysis_for(&target_uri, target_doc.text.clone(), target_doc.dialect)
                    .await?;
                if identity.is_current(target_uri.as_str(), &target_doc.text, &target_analysis) {
                    locations.push(Location {
                        uri: target_uri,
                        range: lift_lsp_range(range),
                    });
                }
            }
            return Ok(
                (!locations.is_empty()).then_some(GotoImplementationResponse::Array(locations))
            );
        }
        let locations = self.readonly_logical_implementation(&uri, pos).await?;
        Ok(locations
            .filter(|locations| !locations.is_empty())
            .map(GotoImplementationResponse::Array))
    }

    pub(crate) async fn serve_references(
        &self,
        params: ReferenceParams,
    ) -> jsonrpc::Result<Option<Vec<Location>>> {
        if !self
            .feature_enabled(
                "references",
                &params.text_document_position.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri.clone();
        let pos = params.text_document_position.position;
        let include_decl = params.context.include_declaration;
        let mut settled = None;
        if let std::ops::ControlFlow::Break(locations) = self
            .original_source_view_navigation_route(&uri, pos, Some(include_decl))
            .await?
        {
            match self.original_source_reference_supplement(&uri).await? {
                Some(sourced) => settled = Some((locations.unwrap_or_default(), sourced)),
                // A view that lists something is the answer. One that lists
                // nothing leaves the ordinary listing to answer.
                None if locations.as_ref().is_some_and(|found| !found.is_empty()) => {
                    return Ok(locations);
                }
                None => {}
            }
        }
        let ordinary = self.ordinary_references(&uri, pos, include_decl).await?;
        let Some((mut locations, sourced)) = settled else {
            return Ok(ordinary);
        };
        locations.extend(
            ordinary
                .into_iter()
                .flatten()
                .filter(|location| !sourced.contains(&location.uri)),
        );
        dedup_locations(&mut locations);
        Ok((!locations.is_empty()).then_some(locations))
    }

    pub(crate) async fn serve_document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> jsonrpc::Result<Option<Vec<DocumentHighlight>>> {
        if !self
            .feature_enabled(
                "documentHighlight",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        // Highlighting is find-references narrowed to one document, so it
        // takes the same whole-program export view —
        // otherwise a `-force`-shadowed call highlights as an occurrence of a
        // definition go-to-definition refuses to open.
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let entries = crate::rt::spawn_blocking(move || {
            // The kinded entry
            // point tags variable defining spans as `Write` and
            // their reads as `Read`; command-invocation heads
            // stay `Text`.
            core_references::document_highlights_in_program(
                &doc.text,
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
                pos.line,
                pos.character,
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
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("document_highlight worker panicked: {err}").into(),
            data: None,
        })?;
        if entries.is_empty() {
            return Ok(None);
        }
        let highlights = entries
            .into_iter()
            .map(|(r, kind)| DocumentHighlight {
                range: lift_lsp_range(r),
                kind: Some(match kind {
                    core_references::HighlightKind::Read => DocumentHighlightKind::READ,
                    core_references::HighlightKind::Write => DocumentHighlightKind::WRITE,
                    core_references::HighlightKind::Text => DocumentHighlightKind::TEXT,
                }),
            })
            .collect();
        Ok(Some(highlights))
    }

    pub(crate) async fn serve_prepare_call_hierarchy(
        &self,
        params: CallHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyItem>>> {
        if !self
            .feature_enabled(
                "callHierarchy",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        if let std::ops::ControlFlow::Break(items) = self
            .original_source_view_call_prepare_route(&uri, pos)
            .await?
        {
            return Ok(items.filter(|items| !items.is_empty()));
        }
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            let documents = self.original_declaration_documents(&uri).await?;
            let workspace = self.workspace_index.read().await;
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
            let items = tcl_lsp_core::original_call_hierarchy::prepare(
                uri.as_str(),
                pos.line,
                pos.character,
                &inventory,
                Some(&workspace),
            );
            drop(workspace);
            let mut lifted = Vec::new();
            for item in items {
                if let Some(item) = self.lift_original_call_item(item).await? {
                    lifted.push(item);
                }
            }
            return Ok((!lifted.is_empty()).then_some(lifted));
        }
        let items = self.readonly_logical_call_prepare(&uri, pos).await?;
        Ok(items.filter(|items| !items.is_empty()))
    }

    pub(crate) async fn serve_incoming_calls(
        &self,
        params: CallHierarchyIncomingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyIncomingCall>>> {
        if !self
            .feature_enabled("callHierarchy", &params.item.uri)
            .await
        {
            return Ok(None);
        }
        let item = params.item;
        if item
            .data
            .as_ref()
            .and_then(|data| data.get("originalDeclaration"))
            .and_then(|handle| handle.get("schema"))
            .and_then(serde_json::Value::as_u64)
            == Some(5)
        {
            let edges = self.walk_original_source_view_calls(&item, true).await?;
            return Ok(Some(
                edges
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(from, from_ranges)| CallHierarchyIncomingCall { from, from_ranges })
                    .collect(),
            ));
        }
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
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
            let workspace = self.workspace_index.read().await;
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
            let edges = tcl_lsp_core::original_call_hierarchy::incoming(
                &identity,
                &inventory,
                Some(&workspace),
            );
            drop(workspace);
            let mut lifted = Vec::new();
            for (_, edge) in edges {
                if let Some(item) = self.lift_original_call_item(edge.from).await? {
                    lifted.push(CallHierarchyIncomingCall {
                        from: item,
                        from_ranges: edge.from_ranges.into_iter().map(lift_lsp_range).collect(),
                    });
                }
            }
            return Ok(Some(lifted));
        }
        let edges = self.readonly_logical_call_walk(&item, true).await?;
        Ok(edges.map(|edges| {
            edges
                .into_iter()
                .map(|(from, from_ranges)| CallHierarchyIncomingCall { from, from_ranges })
                .collect()
        }))
    }

    pub(crate) async fn serve_outgoing_calls(
        &self,
        params: CallHierarchyOutgoingCallsParams,
    ) -> jsonrpc::Result<Option<Vec<CallHierarchyOutgoingCall>>> {
        if !self
            .feature_enabled("callHierarchy", &params.item.uri)
            .await
        {
            return Ok(None);
        }
        let item = params.item;
        if item
            .data
            .as_ref()
            .and_then(|data| data.get("originalDeclaration"))
            .and_then(|handle| handle.get("schema"))
            .and_then(serde_json::Value::as_u64)
            == Some(5)
        {
            let edges = self.walk_original_source_view_calls(&item, false).await?;
            return Ok(Some(
                edges
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(to, from_ranges)| CallHierarchyOutgoingCall { to, from_ranges })
                    .collect(),
            ));
        }
        let uri = item.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
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
            let workspace = self.workspace_index.read().await;
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
            let edges = tcl_lsp_core::original_call_hierarchy::outgoing(
                &identity,
                &inventory,
                Some(&workspace),
            );
            drop(workspace);
            let mut lifted = Vec::new();
            for (_, edge) in edges {
                if let Some(item) = self.lift_original_call_item(edge.to).await? {
                    lifted.push(CallHierarchyOutgoingCall {
                        to: item,
                        from_ranges: edge.from_ranges.into_iter().map(lift_lsp_range).collect(),
                    });
                }
            }
            return Ok(Some(lifted));
        }
        let edges = self.readonly_logical_call_walk(&item, false).await?;
        Ok(edges.map(|edges| {
            edges
                .into_iter()
                .map(|(to, from_ranges)| CallHierarchyOutgoingCall { to, from_ranges })
                .collect()
        }))
    }

    pub(crate) async fn serve_prepare_type_hierarchy(
        &self,
        params: TypeHierarchyPrepareParams,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let pos = params.text_document_position_params.position;
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if analysis.allows_lexical_declaration_advice() {
            let lifted = self.readonly_logical_hierarchy_prepare(&uri, pos).await?;
            return Ok((!lifted.is_empty()).then_some(lifted));
        }
        let owner = uri.to_string();
        let items = crate::rt::spawn_blocking(move || {
            core_type_hierarchy::prepare_for_document(
                &owner,
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("type_hierarchy worker panicked: {err}").into(),
            data: None,
        })?;
        if items.is_empty() {
            return Ok(None);
        }
        let lifted = self.lift_original_type_hierarchy_items(items).await?;
        Ok((!lifted.is_empty()).then_some(lifted))
    }

    pub(crate) async fn serve_supertypes(
        &self,
        params: TypeHierarchySupertypesParams,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
        self.type_hierarchy_walk(params.item, false).await
    }

    pub(crate) async fn serve_subtypes(
        &self,
        params: TypeHierarchySubtypesParams,
    ) -> jsonrpc::Result<Option<Vec<TypeHierarchyItem>>> {
        self.type_hierarchy_walk(params.item, true).await
    }

    pub(crate) async fn serve_linked_editing_range(
        &self,
        params: LinkedEditingRangeParams,
    ) -> jsonrpc::Result<Option<LinkedEditingRanges>> {
        if !self
            .feature_enabled(
                "linkedEditingRange",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        // `S-linked-editing-range-rich`: when the cursor sits
        // on a proc declaration's name or inside the proc's
        // body, return every range that should be edited in
        // lock-step (the declaration plus any self-call sites
        // inside the body).  Editors then paint these as
        // linked-edit chips so renaming one updates the others
        // as the user types.
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let pos = params.text_document_position_params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let result = crate::rt::spawn_blocking(move || {
            core_linked_editing_range::linked_editing_ranges(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("linked_editing_range worker panicked: {err}").into(),
            data: None,
        })?;
        let Some(linked) = result else {
            return Ok(None);
        };
        Ok(Some(LinkedEditingRanges {
            ranges: linked.ranges.into_iter().map(lift_lsp_range).collect(),
            word_pattern: Some(linked.word_pattern),
        }))
    }

    pub(crate) async fn serve_selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> jsonrpc::Result<Option<Vec<SelectionRange>>> {
        if !self
            .feature_enabled("selectionRange", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let positions = params.positions;
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let result = crate::rt::spawn_blocking(move || {
            positions
                .into_iter()
                .map(|pos| {
                    let chain = core_selection_range::selection_range_for_dialect(
                        &doc.text,
                        pos.line,
                        pos.character,
                        Some(&analysis),
                        tcl_lsp_core::profile_for_dialect(&doc.dialect),
                    );
                    // The LSP spec requires `result[i]` to answer
                    // `positions[i]`, so every position must yield a range.
                    // When no chain is found (e.g. the cursor sits on empty
                    // space), fall back to a degenerate range at the cursor
                    // itself rather than dropping the entry, which would
                    // misalign the client's cursor-to-range pairing.
                    materialise_selection_range(&chain).unwrap_or(SelectionRange {
                        range: Range {
                            start: pos,
                            end: pos,
                        },
                        parent: None,
                    })
                })
                .collect::<Vec<SelectionRange>>()
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("selection_range worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(Some(result))
    }

    pub(crate) async fn serve_signature_help(
        &self,
        params: SignatureHelpParams,
    ) -> jsonrpc::Result<Option<SignatureHelp>> {
        if !self
            .feature_enabled(
                "signatureHelp",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let pos = position_within_line(&doc.text, params.text_document_position_params.position);
        let disabled_commands = self.resolved_signature_help_disabled_commands(&uri).await;
        // Per-dialect cached registry — same source the
        // completion handler uses.  Threading it through lets
        // `S-signature-help-rich` surface signatures for
        // built-in commands (e.g. `puts`, `lsearch`) without
        // requiring a user proc with the same name.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        // Same whole-program view as hover: the signature rendered is the
        // reached proc's parameter list, which a `-force` shadow changes.
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let result = crate::rt::spawn_blocking(move || {
            core_sig::signature_help_in_program_with_options(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                core_definition::CallResolution {
                    registry: Some(&registry),
                    program: Some(core_definition::ProgramExports {
                        uri: &uri_key,
                        oracle: exports.as_ref(),
                    }),
                },
                core_sig::SignatureHelpOptions {
                    disabled_builtin_commands: &disabled_commands,
                },
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("signature_help worker panicked: {err}").into(),
            data: None,
        })?;
        Ok(result.map(lift_signature_help))
    }

    pub(crate) async fn serve_hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        if !self
            .feature_enabled(
                "hover",
                &params.text_document_position_params.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let source_epoch = self.diag_inputs_epoch();
        let source_generation = self.live_publication_uri_generation(&uri);
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let pos = position_within_line(&doc.text, params.text_document_position_params.position);
        // Run the provider over the memoised `file_analysis` query (no separate
        // hover result cache — walking the cached analysis is cheap, and the
        // query graph already avoids re-analysing).  Worker-offload via
        // `spawn_blocking` keeps the LSP event loop responsive.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if let std::ops::ControlFlow::Break(Some(hover)) = self
            .original_source_view_hover_route_for_snapshot(&uri, &doc.text, &analysis, pos)
            .await?
        {
            return Ok(Some(lift_hover(hover)));
        }

        // `profile_for_dialect`, not `by_name`: this is a dialect ingress, and
        // `by_name` sinks the additive set-only spelling `tk` to the plain
        // fallback. That is not a repair — hover answers a `tk` document
        // correctly either way, because the registry beside this profile is
        // keyed by the dialect *name* and the plain-Tcl fallback is permissive
        // enough to resolve its rows. Resolving here forecloses the divergence
        // instead: the profile and the registry now agree about what `tk` is,
        // so a future gate that does consult the profile cannot silently
        // disagree with the command surface. See
        // `hover_on_a_tk_document_resolves_a_tk_command`.
        let hover_profile = tcl_lsp_core::profile_for_dialect(&doc.dialect);
        // The cross-document fallback must only fire on a command head, the
        // same gate `compute_definition` applies — otherwise an argument word
        // that happens to share a sibling proc's name would pop up that proc's
        // signature.  Computed here, while `analysis` is still in scope.
        let on_command_head = position_is_command_head(&doc.text, pos, &analysis, &doc.line_index);
        if let ControlFlow::Break(hover) = self
            .position_hover_before_method(&uri, &doc, &analysis, pos)
            .await?
        {
            return Ok(hover);
        }
        if let Some(hover) = self
            .conditional_foreign_factory_source_hover(
                &uri,
                &doc,
                source_epoch,
                source_generation,
                pos,
                &analysis,
            )
            .await
        {
            return Ok(Some(lift_hover(hover)));
        }
        // A member route with no card still lets the document's own hover
        // answer from its declared classes; only the workspace fallback,
        // which knows nothing of the member, is withheld.
        let member_route_declined = match Box::pin(self.original_method_hover_for_snapshot(
            (uri.clone(), doc.clone(), source_generation),
            &analysis,
            pos,
            source_epoch,
        ))
        .await?
        {
            std::ops::ControlFlow::Break(Some(hover)) => return Ok(Some(lift_hover(hover))),
            std::ops::ControlFlow::Break(None) => true,
            std::ops::ControlFlow::Continue(()) => false,
        };
        let text = doc.text.clone();
        let analysis_worker = Arc::clone(&analysis);
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let result = crate::rt::spawn_blocking(move || {
            core_hover::hover_in_program(
                &text,
                pos.line,
                pos.character,
                &analysis_worker,
                Some(&registry),
                hover_profile,
                Some(core_definition::ProgramExports {
                    uri: &uri_key,
                    oracle: exports.as_ref(),
                }),
            )
        })
        .await
        .map_err(|err| Self::hover_worker_error(&err))?;
        if let Some(hover) = result {
            return Ok(Some(lift_hover(hover)));
        }
        if member_route_declined {
            return Ok(None);
        }
        self.hover_workspace_fallback(&uri, &doc, &analysis, pos, on_command_head)
            .await
    }
}
