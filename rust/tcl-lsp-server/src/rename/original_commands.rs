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

//! Renames of commands and members planned from their retained declarations.

use super::{
    AnalysisResult, Arc, Backend, DocumentSourceResult, DocumentState, HashMap, Position,
    PrepareRenameResponse, TextEdit, Uri, WorkspaceEdit, core_definition, core_workspace_index,
    implementation_inventory, jsonrpc, lift_lsp_range, line_col_to_byte_offset,
    original_command_rename_refusal_error, original_member_rename_refusal_error,
};

pub(in super::super) type OriginalDocumentSnapshot = (String, Arc<str>, Arc<AnalysisResult>);

impl Backend {
    async fn original_rename_documents_are_current(
        &self,
        documents: &[OriginalDocumentSnapshot],
        epoch: u64,
    ) -> DocumentSourceResult<bool> {
        for (owner, source, analysis) in documents {
            let Ok(uri) = owner.parse::<Uri>() else {
                return Ok(false);
            };
            let Some(document) = self.read_document(&uri).await? else {
                return Ok(false);
            };
            if document.text.as_ref() != source.as_ref() {
                return Ok(false);
            }
            let current = self
                .analysis_for(&uri, document.text, document.dialect)
                .await?;
            let context =
                core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(analysis);
            if context.is_none()
                || context
                    != core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(
                        &current,
                    )
            {
                return Ok(false);
            }
        }
        Ok(self.diag_inputs_epoch() == epoch)
    }

    fn original_command_workspace_edit(
        documents: &[OriginalDocumentSnapshot],
        intents: Vec<tcl_lsp_core::rename::WorkspaceTextEdit>,
    ) -> Result<
        Option<WorkspaceEdit>,
        tcl_lsp_core::original_command_rename::OriginalCommandRenameRefusal,
    > {
        use tcl_lsp_core::original_command_rename::OriginalCommandRenameRefusal;
        let mut changes = HashMap::<Uri, Vec<TextEdit>>::new();
        for intent in intents {
            let Some((_, source, _)) = documents.iter().find(|(owner, _, _)| owner == &intent.uri)
            else {
                return Err(OriginalCommandRenameRefusal::StaleSource);
            };
            let Ok(target) = intent.uri.parse::<Uri>() else {
                return Err(OriginalCommandRenameRefusal::StaleSource);
            };
            changes.entry(target).or_default().push(TextEdit {
                range: lift_lsp_range(core_definition::span_to_range(
                    source,
                    &tcl_lexer::LineIndex::new(source),
                    intent.span,
                )),
                new_text: intent.new_text,
            });
        }
        Ok((!changes.is_empty()).then_some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }))
    }

    pub(in super::super) async fn original_command_rename_tier(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
        new_name: &str,
    ) -> DocumentSourceResult<std::ops::ControlFlow<jsonrpc::Result<Option<WorkspaceEdit>>>> {
        use std::ops::ControlFlow;
        use tcl_lsp_core::original_command_rename::{self, OriginalCommandRenameRefusal};
        if analysis.allows_lexical_declaration_advice() {
            return Ok(ControlFlow::Continue(()));
        }
        let member =
            tcl_lsp_core::method_symbol::select(&doc.text, analysis, pos.line, pos.character)
                .is_break();
        let stale = || {
            if member {
                original_member_rename_refusal_error(
                    tcl_lsp_core::original_member_rename::OriginalMemberRenameRefusal::StaleSource,
                )
            } else {
                original_command_rename_refusal_error(OriginalCommandRenameRefusal::StaleSource)
            }
        };
        let epoch = self.diag_inputs_epoch();
        let documents = self.original_declaration_documents(uri).await?;
        let context =
            core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(analysis);
        if context.is_none()
            || !documents.iter().any(|(owner, source, retained)| {
                owner == uri.as_str()
                    && source.as_ref() == doc.text.as_ref()
                    && core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(
                        retained,
                    ) == context
            })
        {
            return Ok(ControlFlow::Break(Err(stale())));
        }
        let owners = implementation_inventory(&documents);
        let Some(offset) = line_col_to_byte_offset(&doc.text, pos.line, pos.character)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return Ok(ControlFlow::Break(Ok(None)));
        };
        let selected = if member {
            self.original_member_identity_at(uri, &doc.text, analysis, (pos, offset), &documents)
                .await?
        } else {
            original_command_rename::select_in_documents(&owners, uri.as_str(), offset)
        };
        let ControlFlow::Break(Some(identity)) = selected else {
            return Ok(ControlFlow::Break(Ok(None)));
        };
        drop(original_command_rename::take_missing_rename_world_trace());
        drop(owners);
        let (documents, source_worlds) =
            Box::pin(self.command_source_documents(member, documents, identity.uri())).await?;
        let owners = implementation_inventory(&documents);
        let planned = if member {
            tcl_lsp_core::original_member_rename::original_member_rename_edits(
                &owners, &identity, new_name,
            )
            .map_err(original_member_rename_refusal_error)
        } else {
            self.original_command_plan_with_source_views(
                &owners,
                &identity,
                new_name,
                (uri.as_str(), offset),
                source_worlds.as_ref(),
            )
            .await?
            .ok_or_else(stale)
            .and_then(|planned| planned.map_err(original_command_rename_refusal_error))
        };
        let intents = match planned {
            Ok(intents) => intents,
            Err(refusal) => {
                return Ok(ControlFlow::Break(Err(refusal)));
            }
        };
        if let Some(capture) = source_worlds.as_ref()
            && !Box::pin(self.original_command_source_worlds_are_current(capture)).await?
        {
            return Ok(ControlFlow::Break(Err(stale())));
        }
        if !self
            .original_rename_documents_are_current(&documents, epoch)
            .await?
        {
            return Ok(ControlFlow::Break(Err(stale())));
        }
        Ok(ControlFlow::Break(
            Self::original_command_workspace_edit(&documents, intents).map_err(|_| stale()),
        ))
    }

    fn original_command_prepare_plan(
        member: bool,
        owners: &[tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'_>],
        identity: &tcl_lsp_core::original_declaration::OriginalDeclarationIdentity,
        uri: &Uri,
        offset: u32,
        conditional_worlds: &[tcl_lsp_core::original_command_rename::OriginalConditionalCommandWorld<'_>],
    ) -> Option<tcl_lexer::Span> {
        use tcl_lsp_core::original_command_rename;
        if member {
            tcl_lsp_core::original_member_rename::original_member_prepare_span_in(
                owners,
                identity,
                uri.as_str(),
                offset,
            )
            .ok()
        } else {
            original_command_rename::original_command_prepare_span_in_with_conditional_worlds(
                owners,
                identity,
                uri.as_str(),
                offset,
                conditional_worlds,
            )
            .ok()
        }
    }

    /// A consumer's member call selects the declaration its loaded class
    /// retains in the owner's document; the owner's analysis supplies the
    /// identity, never the consumer's reporting names.
    async fn original_member_identity_at(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        (pos, offset): (Position, u32),
        documents: &[OriginalDocumentSnapshot],
    ) -> DocumentSourceResult<
        std::ops::ControlFlow<
            Option<tcl_lsp_core::original_declaration::OriginalDeclarationIdentity>,
        >,
    > {
        use std::ops::ControlFlow;
        let local = tcl_lsp_core::original_declaration::select_at_offset(
            uri.as_str(),
            source,
            analysis,
            offset,
        );
        if !matches!(local, ControlFlow::Break(None)) {
            return Ok(local);
        }
        let ControlFlow::Break(Some(candidate)) = self
            .original_method_candidate_at(uri, source, analysis, pos)
            .await?
        else {
            return Ok(ControlFlow::Break(None));
        };
        if candidate.uri() == uri.as_str() {
            return Ok(ControlFlow::Break(None));
        }
        Ok(ControlFlow::Break(
            documents
                .iter()
                .find(|(owner, _, _)| owner == candidate.uri())
                .and_then(|(owner, owner_source, owner_analysis)| {
                    tcl_lsp_core::original_declaration::OriginalDeclarationIdentity::for_member_candidate(
                        owner,
                        owner_source,
                        owner_analysis,
                        &candidate,
                    )
                }),
        ))
    }

    pub(in super::super) async fn original_command_prepare_tier(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<std::ops::ControlFlow<Option<PrepareRenameResponse>>> {
        use std::ops::ControlFlow;
        use tcl_lsp_core::original_command_rename;
        if analysis.allows_lexical_declaration_advice()
            || Self::original_namespace_cursor_retained(&doc.text, analysis, pos)
            || tcl_lsp_core::variable_symbol::select(&doc.text, analysis, pos.line, pos.character)
                .is_break()
        {
            return Ok(ControlFlow::Continue(()));
        }
        let member =
            tcl_lsp_core::method_symbol::select(&doc.text, analysis, pos.line, pos.character)
                .is_break();
        let epoch = self.diag_inputs_epoch();
        let documents = self.original_declaration_documents(uri).await?;
        let context =
            core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(analysis);
        if context.is_none()
            || !documents.iter().any(|(owner, source, retained)| {
                owner == uri.as_str()
                    && source.as_ref() == doc.text.as_ref()
                    && core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(
                        retained,
                    ) == context
            })
        {
            return Ok(ControlFlow::Break(None));
        }
        let owners = implementation_inventory(&documents);
        let Some(offset) = line_col_to_byte_offset(&doc.text, pos.line, pos.character)
            .and_then(|offset| u32::try_from(offset).ok())
        else {
            return Ok(ControlFlow::Break(None));
        };
        let selected = if member {
            self.original_member_identity_at(uri, &doc.text, analysis, (pos, offset), &documents)
                .await?
        } else {
            original_command_rename::select_in_documents(&owners, uri.as_str(), offset)
        };
        let ControlFlow::Break(Some(identity)) = selected else {
            return Ok(ControlFlow::Break(None));
        };
        drop(owners);
        let (documents, source_worlds) =
            Box::pin(self.command_source_documents(member, documents, identity.uri())).await?;
        let owners = implementation_inventory(&documents);
        let planned = if member {
            Self::original_command_prepare_plan(true, &owners, &identity, uri, offset, &[])
        } else {
            Box::pin(self.original_command_prepare_with_source_views(
                &owners,
                &identity,
                uri.as_str(),
                offset,
                source_worlds.as_ref(),
            ))
            .await?
        };
        let Some(span) = planned else {
            return Ok(ControlFlow::Break(None));
        };
        if let Some(capture) = source_worlds.as_ref()
            && !Box::pin(self.original_command_source_worlds_are_current(capture)).await?
        {
            return Ok(ControlFlow::Break(None));
        }
        if !self
            .original_rename_documents_are_current(&documents, epoch)
            .await?
        {
            return Ok(ControlFlow::Break(None));
        }
        let Some(placeholder) = doc.text.get(span.as_range()) else {
            return Ok(ControlFlow::Break(None));
        };
        Ok(ControlFlow::Break(Some(
            PrepareRenameResponse::RangeWithPlaceholder {
                range: lift_lsp_range(core_definition::span_to_range(
                    &doc.text,
                    &doc.line_index,
                    span,
                )),
                placeholder: placeholder.to_owned(),
            },
        )))
    }

    /// Expression identifiers and command words have independent edit owners.
    /// A selected procedure cannot be partly renamed across the workspace.
    pub(in super::super) async fn original_procedure_expression_coverage_is_editable(
        &self,
        uri: &Uri,
        source: &str,
        analysis: &AnalysisResult,
        pos: Position,
    ) -> DocumentSourceResult<bool> {
        if analysis.allows_lexical_declaration_advice() {
            return Ok(true);
        }
        let epoch = self
            .diag_inputs_epoch
            .load(std::sync::atomic::Ordering::Acquire);
        let documents = self.original_declaration_documents(uri).await?;
        let Some(current_context) =
            core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(analysis)
        else {
            return Ok(false);
        };
        if current_context.image() != &tcl_lexer::SourceImage::document(source)
            || !documents.iter().any(|(owner, text, retained)| {
                owner == uri.as_str()
                    && text.as_ref() == source
                    && core_workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(
                        retained,
                    )
                    .as_ref()
                        == Some(&current_context)
            })
        {
            return Ok(false);
        }
        let owners: Vec<_> = documents
            .iter()
            .map(|(uri, source, analysis)| {
                tcl_lsp_core::original_call_hierarchy::OriginalCallDocument {
                    uri,
                    source,
                    analysis,
                }
            })
            .collect();
        let items = {
            let index = self.workspace_index.read().await;
            tcl_lsp_core::original_call_hierarchy::prepare(
                uri.as_str(),
                pos.line,
                pos.character,
                &owners,
                Some(&index),
            )
        };
        if epoch
            != self
                .diag_inputs_epoch
                .load(std::sync::atomic::Ordering::Acquire)
        {
            return Ok(false);
        }
        let [item] = items.as_slice() else {
            return Ok(true);
        };
        let Some(identity) = item.identity.as_ref() else {
            return Ok(false);
        };
        if identity.role() != tcl_lsp_core::original_declaration::OriginalDeclarationRole::Procedure
        {
            return Ok(true);
        }
        let Some((declaration_uri, declaration_source, declaration_analysis)) = documents
            .iter()
            .find(|(owner, _, _)| owner == identity.uri())
        else {
            return Ok(false);
        };
        if !identity.is_current(declaration_uri, declaration_source, declaration_analysis) {
            return Ok(false);
        }
        let Some(declaration) = identity.procedure_metadata(declaration_analysis) else {
            return Ok(false);
        };
        Ok(documents
            .iter()
            .all(|(_, consumer_source, consumer_analysis)| {
                tcl_lsp_core::math_function_symbol::procedure_references_in(
                    consumer_source,
                    consumer_analysis,
                    declaration_source,
                    declaration_analysis,
                    declaration,
                    true,
                )
                .is_some_and(|references| references.is_empty())
            }))
    }
}
