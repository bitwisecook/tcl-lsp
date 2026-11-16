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

//! The source worlds an original command plan is prepared in, captured over the settled views.

use super::super::{Arc, Backend, DocumentSourceResult, original_readonly_analysis_matches_input};
use super::OriginalCommandSourceWorldCapture;

impl OriginalCommandSourceWorldCapture {
    /// Select the actual still-leased prefix producer for independent collision
    /// participants. The selected declaration retains its original owner.
    fn document_owners(
        &self,
        documents: &[super::super::super::OriginalDocumentSnapshot],
        selected_uri: &str,
    ) -> Option<Vec<super::super::super::OriginalDocumentSnapshot>> {
        documents.iter().map(|document| {
            if document.0 == selected_uri || document.2.original_completed_command_world().is_some() {
                return Some(document.clone());
            }
            let mut scopes = self.settlement.scopes.iter().filter(|scope|
                scope.root_uri.as_str() == document.0.as_str() && scope.completion.is_some());
            let Some(scope) = scopes.next() else { return Some(document.clone()) };
            if scopes.next().is_some() || !scope.complete || scope.conditional_obligations {
                return None;
            }
            let completion = scope.completion.as_ref()?;
            let prefix = &completion.prefix.draft;
            let analysis = &prefix.producer.seed.analysis;
            let context = tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(&document.2)?;
            if scope.root != Some(completion.prefix.id)
                || prefix.inherited.is_some() || prefix.region.is_some()
                || prefix.physical.uri.as_str() != document.0.as_str()
                || prefix.physical.source.as_ref() != document.1.as_ref()
                || prefix.producer.seed.path_admission.as_ref() != prefix.physical.admission.as_ref()
                || !prefix.matches_producer()
                || !original_readonly_analysis_matches_input(&document.1, &document.2, &prefix.producer.input)
                || tcl_lsp_core::workspace_index::WorkspaceDiagnosticSourceContext::for_analysis(analysis) != Some(context)
                || !completion.outcome.matches_prefix_analysis(analysis)
            {
                return None;
            }
            Some((document.0.clone(), Arc::clone(&document.1), Arc::clone(analysis)))
        }).collect()
    }

    pub(crate) fn worlds(
        &self,
    ) -> Vec<tcl_lsp_core::original_command_rename::OriginalConditionalCommandWorld<'_>> {
        self.settlement
            .scopes
            .iter()
            .filter_map(|scope| {
                let completion = scope.completion.as_ref()?;
                Some(
                    tcl_lsp_core::original_command_rename::OriginalConditionalCommandWorld {
                        uri: scope.root_uri.as_str(),
                        outcome: &completion.outcome,
                    },
                )
            })
            .collect()
    }
}

impl Backend {
    pub(crate) async fn command_source_documents(
        &self,
        member: bool,
        documents: Vec<super::super::super::OriginalDocumentSnapshot>,
        selected_uri: &str,
    ) -> DocumentSourceResult<(
        Vec<super::super::super::OriginalDocumentSnapshot>,
        Option<OriginalCommandSourceWorldCapture>,
    )> {
        let capture = {
            let owners = super::super::super::implementation_inventory(&documents);
            self.capture_original_command_source_worlds_if_needed(member, &owners)
                .await?
        };
        let Some(capture) = capture else {
            return Ok((documents, None));
        };
        let Some(selected_documents) = capture.document_owners(&documents, selected_uri) else {
            // Keep the original unknown-world refusal; no alternate owner is
            // installed when a current full-context/physical join is missing.
            return Ok((documents, None));
        };
        Ok((selected_documents, Some(capture)))
    }

    /// Enclose the complete source survey in the existing physical and
    /// interpretation publication leases. No borrowed view escapes rendering.
    pub(crate) async fn original_command_plan_with_source_views(
        &self,
        documents: &[tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'_>],
        identity: &tcl_lsp_core::original_declaration::OriginalDeclarationIdentity,
        new_tail: &str,
        cursor: (&str, u32),
        capture: Option<&OriginalCommandSourceWorldCapture>,
    ) -> DocumentSourceResult<
        Option<
            Result<
                Vec<tcl_lsp_core::rename::WorkspaceTextEdit>,
                tcl_lsp_core::original_command_rename::OriginalCommandRenameRefusal,
            >,
        >,
    > {
        let Some(capture) = capture else {
            return Ok(Some(tcl_lsp_core::original_command_rename::original_command_rename_edits_with_conditional_worlds(
                documents, identity, new_tail, &[],
            )));
        };
        if !self
            .original_command_source_worlds_are_current(capture)
            .await?
        {
            return Ok(None);
        }
        let mut ids = Vec::new();
        for scope in &capture.settlement.scopes {
            for id in &scope.ids {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        let worlds = capture.worlds();
        self.with_selected_original_source_views(&ids, Some(capture.settlement.revision), |views| {
            let (publications, ancestors) = Self::original_source_view_labels(views)?;
            let inventory = Self::original_source_view_inventory(views, &publications, &ancestors);
            Some(tcl_lsp_core::original_command_rename::original_command_rename_edits_with_selected_source_view(
                documents, identity, new_tail, cursor.0, cursor.1, &worlds, &inventory,
            ))
        }).await
    }

    /// Select and consume the inherited declaration only inside the same
    /// indexed source-view revision used by the subsequent rename planner.
    pub(crate) async fn original_command_prepare_with_source_views(
        &self,
        documents: &[tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'_>],
        identity: &tcl_lsp_core::original_declaration::OriginalDeclarationIdentity,
        cursor_uri: &str,
        cursor: u32,
        capture: Option<&OriginalCommandSourceWorldCapture>,
    ) -> DocumentSourceResult<Option<tcl_lexer::Span>> {
        let Some(capture) = capture else {
            return Ok(
                tcl_lsp_core::original_command_rename::original_command_prepare_span_in(
                    documents, identity, cursor_uri, cursor,
                )
                .ok(),
            );
        };
        if !self
            .original_command_source_worlds_are_current(capture)
            .await?
        {
            return Ok(None);
        }
        let mut ids = Vec::new();
        for scope in &capture.settlement.scopes {
            for id in &scope.ids {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        let worlds = capture.worlds();
        self.with_selected_original_source_views(&ids, Some(capture.settlement.revision), |views| {
            let (publications, ancestors) = Self::original_source_view_labels(views)?;
            let inventory = Self::original_source_view_inventory(views, &publications, &ancestors);
            tcl_lsp_core::original_command_rename::original_command_prepare_span_in_with_selected_source_view(
                documents, identity, cursor_uri, cursor, &worlds, &inventory,
            ).ok()
        }).await
    }

    pub(crate) async fn capture_original_command_source_worlds_if_needed(
        &self,
        member: bool,
        owners: &[tcl_lsp_core::original_declaration::OriginalDeclarationDocument<'_>],
    ) -> DocumentSourceResult<Option<OriginalCommandSourceWorldCapture>> {
        if !member
            && owners
                .iter()
                .any(|owner| owner.analysis.original_completed_command_world().is_none())
        {
            self.capture_original_command_source_worlds().await
        } else {
            Ok(None)
        }
    }

    pub(crate) async fn capture_original_command_source_worlds(
        &self,
    ) -> DocumentSourceResult<Option<OriginalCommandSourceWorldCapture>> {
        let Some(settlement) = self.refresh_original_source_view_graph().await? else {
            return Ok(None);
        };
        let capture = OriginalCommandSourceWorldCapture { settlement };
        Ok(self
            .original_command_source_worlds_are_current(&capture)
            .await?
            .then_some(capture))
    }

    pub(crate) async fn original_command_source_worlds_are_current(
        &self,
        capture: &OriginalCommandSourceWorldCapture,
    ) -> DocumentSourceResult<bool> {
        if !self.original_source_view_settlement_is_indexed(&capture.settlement) {
            return Ok(false);
        }
        for scope in &capture.settlement.scopes {
            let Some(completion) = &scope.completion else {
                continue;
            };
            let current = self
                .with_selected_original_source_views(
                    &scope.ids,
                    Some(capture.settlement.revision),
                    |views| {
                        (views.iter().any(|view| {
                            view.id == completion.prefix.id
                                && view.draft.same_live_producer(&completion.prefix.draft)
                        }) && completion.dependencies.iter().all(|dependency| {
                            views.iter().any(|view| {
                                view.id == dependency.id
                                    && view.draft.same_live_producer(&dependency.draft)
                            })
                        }) && completion.outcome.matches_prefix_analysis(
                            &completion.prefix.draft.producer.seed.analysis,
                        ))
                        .then_some(())
                    },
                )
                .await?;
            if current.is_none() {
                return Ok(false);
            }
        }
        Ok(self.original_source_view_settlement_is_indexed(&capture.settlement))
    }
}
