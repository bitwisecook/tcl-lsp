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

//! Variable renames whose edits are conditional on the client's confirmation.

use super::file_rename::FileRenameInputSnapshot;
use super::{
    Arc, Backend, ControlFlow, DocumentPathAdmission, DocumentState, FreshAnalysisSeed,
    InitializeParams, Position, PrepareRenameResponse, Range, TextEdit, Uri, WorkspaceEdit,
    WorkspaceRenameCaller, WorkspaceSourcePlanToken, core_rename, jsonrpc, lift_lsp_range,
};

impl Backend {
    pub(in super::super) async fn conditional_variable_prepare_tier(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        pos: Position,
        conditional_uri_generation: u64,
    ) -> jsonrpc::Result<ControlFlow<Option<PrepareRenameResponse>>> {
        if !self
            .conditional_variable_rename_supported
            .load(std::sync::atomic::Ordering::Relaxed)
            || doc.version.is_none()
        {
            return Ok(ControlFlow::Continue(()));
        }
        let source_plan = self.capture_workspace_source_plan().await;
        let Some(inputs) = self.file_rename_input_snapshot().await else {
            return Ok(ControlFlow::Break(None));
        };
        let seed = self
            .fresh_analysis_seed(uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let text = doc.text.clone();
        let worker_analysis = seed.analysis.clone();
        let prepared = crate::rt::spawn_blocking(move || {
            core_rename::prepare_conditional_variable_rename(
                &text,
                pos.line,
                pos.character,
                &worker_analysis,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("conditional prepare worker panicked: {err}").into(),
            data: None,
        })?;
        if let Some(prepared) = prepared {
            if !self.workspace_source_plan_is_current(source_plan).await {
                return Ok(ControlFlow::Break(None));
            }
            let response = PrepareRenameResponse::RangeWithPlaceholder {
                range: lift_lsp_range(prepared.range),
                placeholder: prepared.placeholder,
            };
            return Ok(ControlFlow::Break(
                self.publish_conditional_if_current(
                    uri,
                    doc,
                    &seed,
                    &inputs,
                    conditional_uri_generation,
                    response,
                )
                .await,
            ));
        }
        Ok(ControlFlow::Continue(()))
    }

    pub(super) async fn conditional_variable_rename_tier(
        &self,
        caller: WorkspaceRenameCaller<'_>,
        doc: &DocumentState,
        new_name: &str,
        source_plan: WorkspaceSourcePlanToken,
        conditional_uri_generation: u64,
    ) -> jsonrpc::Result<ControlFlow<Option<WorkspaceEdit>>> {
        let WorkspaceRenameCaller {
            uri,
            analysis,
            position: pos,
            ..
        } = caller;
        if self
            .conditional_variable_rename_supported
            .load(std::sync::atomic::Ordering::Relaxed)
            && analysis.resolved_input.as_ref().is_some_and(
                tcl_compiler::analyser::ResolvedAnalysisInput::has_logical_source_name_context,
            )
            && let std::ops::ControlFlow::Break(Some(selected)) =
                tcl_lsp_core::variable_symbol::select(&doc.text, analysis, pos.line, pos.character)
            && !analysis.original_variable_rename_is_complete(selected.symbol())
        {
            let Some(version) = doc.version else {
                return Ok(ControlFlow::Break(None));
            };
            let Some(inputs) = self.file_rename_input_snapshot().await else {
                return Ok(ControlFlow::Break(None));
            };
            let seed = self
                .fresh_analysis_seed(uri, doc.text.clone(), doc.dialect.clone())
                .await?;
            let text = doc.text.clone();
            let worker_analysis = seed.analysis.clone();
            let worker_name = new_name.to_owned();
            let proposed = crate::rt::spawn_blocking(move || {
                core_rename::conditional_variable_rename_with_diagnosis(
                    &text,
                    pos.line,
                    pos.character,
                    &worker_name,
                    &worker_analysis,
                )
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("conditional rename worker panicked: {err}").into(),
                data: None,
            })?;
            let proposal = match proposed {
                Ok(proposal) if !proposal.edits.is_empty() => proposal,
                _ => return Ok(ControlFlow::Break(None)),
            };
            if !self.workspace_source_plan_is_current(source_plan).await {
                return Ok(ControlFlow::Break(None));
            }
            let raw_source = doc.raw_text.as_deref().unwrap_or(doc.text.as_ref());
            let Some(edit) = coalesce_conditional_variable_edits(
                raw_source,
                &doc.text,
                &proposal.generated_source,
                &doc.line_index,
                &proposal.edits,
            ) else {
                return Ok(ControlFlow::Break(None));
            };
            let Some(response) =
                annotated_conditional_variable_edit(uri.clone(), version, vec![edit])
            else {
                return Ok(ControlFlow::Break(None));
            };
            Ok(ControlFlow::Break(
                self.publish_conditional_if_current(
                    uri,
                    doc,
                    &seed,
                    &inputs,
                    conditional_uri_generation,
                    response,
                )
                .await,
            ))
        } else {
            Ok(ControlFlow::Continue(()))
        }
    }
}

pub(in super::super) fn client_supports_conditional_rename(params: &InitializeParams) -> bool {
    let Some(edit) = params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|w| w.workspace_edit.as_ref())
    else {
        return false;
    };
    edit.document_changes == Some(true)
        && edit.change_annotation_support.is_some()
        && params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|d| d.rename.as_ref())
            .is_some_and(|rename| rename.honors_change_annotations == Some(true))
}

pub(in super::super) fn annotated_conditional_variable_edit(
    uri: Uri,
    version: i32,
    edits: Vec<TextEdit>,
) -> Option<WorkspaceEdit> {
    if edits.is_empty() {
        return None;
    }
    let id = "conditional-variable-source-alpha".to_owned();
    let changes = tower_lsp_server::ls_types::TextDocumentEdit {
        text_document: tower_lsp_server::ls_types::OptionalVersionedTextDocumentIdentifier {
            uri,
            version: Some(version),
        },
        edits: edits
            .into_iter()
            .map(|text_edit| {
                tower_lsp_server::ls_types::OneOf::Right(
                    tower_lsp_server::ls_types::AnnotatedTextEdit {
                        text_edit,
                        annotation_id: id.clone(),
                    },
                )
            })
            .collect(),
    };
    let mut annotations = std::collections::HashMap::new();
    annotations.insert(id, tower_lsp_server::ls_types::ChangeAnnotation {
        label: "Conditional rename in this document".to_owned(),
        needs_confirmation: Some(true),
        description: Some("Uses authored command roles in this document. Other files, runtime bindings, callbacks and traces are not verified.".to_owned()),
    });
    Some(WorkspaceEdit {
        changes: None,
        document_changes: Some(tower_lsp_server::ls_types::DocumentChanges::Edits(vec![
            changes,
        ])),
        change_annotations: Some(annotations),
    })
}

impl Backend {
    pub(in super::super) async fn publish_conditional_if_current<T: Clone>(
        &self,
        uri: &Uri,
        document: &DocumentState,
        seed: &FreshAnalysisSeed,
        inputs: &FileRenameInputSnapshot,
        uri_generation: u64,
        response: T,
    ) -> Option<T> {
        if seed.analyser_inputs_epoch != inputs.analyser_inputs_epoch
            || seed.class_factory_generation != inputs.class_factory_generation
        {
            return None;
        }
        let site = "conditional_variable_source_currency";
        loop {
            let guards = match self.try_file_rename_publication_guards(inputs, site) {
                Ok(Some(guards)) => guards,
                Ok(None) => return None,
                Err(wait) => {
                    self.wait_for_file_rename_publication(wait, site).await;
                    continue;
                }
            };
            let current = guards.documents.get(uri)?;
            let current = current.clone().normalised_for_analysis();
            if current.backing_file_deleted
                || current.version != document.version
                || current.revision != document.revision
                || current.text != document.text
                || current.raw_text != document.raw_text
                || current.dialect != document.dialect
            {
                return None;
            }
            let original_admission = document.path_admission.as_ref()?;
            let live_admission = current.path_admission.as_ref()?;
            if !Arc::ptr_eq(original_admission, live_admission)
                || !Arc::ptr_eq(original_admission, &seed.path_admission)
            {
                return None;
            }
            original_admission.check_uri(uri).ok()?;
            let generations = self
                .live_publication_uri_generations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if generations.get(uri).copied().unwrap_or_default() != uri_generation {
                return None;
            }
            return DocumentPathAdmission::with_current(
                &[original_admission.as_ref(), seed.path_admission.as_ref()],
                self.store.as_ref(),
                || response.clone(),
            )
            .ok();
        }
    }
}

/// Preserve all intervening original source in one versioned text edit.
pub(in super::super) fn coalesce_conditional_variable_edits(
    source: &str,
    analysis_source: &str,
    certified_after: &str,
    index: &tcl_lexer::LineIndex,
    edits: &[core_rename::TextEdit],
) -> Option<TextEdit> {
    if tcl_lexer::normalise_lone_cr(source).as_ref() != analysis_source {
        return None;
    }
    let mut ranges = Vec::new();
    for edit in edits {
        let range = edit.range;
        let start = index.offset_at_utf16(
            range.start_line,
            tcl_lexer::Utf16Col::new(range.start_character),
            source,
        );
        let end = index.offset_at_utf16(
            range.end_line,
            tcl_lexer::Utf16Col::new(range.end_character),
            source,
        );
        let actual_start = index.position_at_utf16(start, source);
        let actual_end = index.position_at_utf16(end, source);
        if (actual_start.line, actual_start.character.get())
            != (range.start_line, range.start_character)
            || (actual_end.line, actual_end.character.get())
                != (range.end_line, range.end_character)
            || start >= end
            || source.get(start as usize..end as usize).is_none()
        {
            return None;
        }
        ranges.push((start, end, edit));
    }
    ranges.sort_by_key(|(start, end, _)| (*start, *end));
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return None;
    }
    let first = ranges.first()?;
    let last = ranges.last()?;
    let mut replacement = source.get(first.0 as usize..last.1 as usize)?.to_owned();
    for (start, end, edit) in ranges.iter().rev() {
        let begin = usize::try_from(start.checked_sub(first.0)?).ok()?;
        let finish = usize::try_from(end.checked_sub(first.0)?).ok()?;
        replacement.replace_range(begin..finish, &edit.new_text);
    }
    let mut expected = analysis_source.to_owned();
    for (start, end, edit) in ranges.iter().rev() {
        expected.get(*start as usize..*end as usize)?;
        expected.replace_range(*start as usize..*end as usize, &edit.new_text);
    }
    let mut materialised = source.to_owned();
    materialised.replace_range(first.0 as usize..last.1 as usize, &replacement);
    if expected != certified_after
        || tcl_lexer::normalise_lone_cr(&materialised).as_ref() != certified_after
    {
        return None;
    }
    Some(TextEdit {
        range: Range::new(
            Position::new(first.2.range.start_line, first.2.range.start_character),
            Position::new(last.2.range.end_line, last.2.range.end_character),
        ),
        new_text: replacement,
    })
}
