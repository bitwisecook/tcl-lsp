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

//! The source an ordinary code action is computed from, and the transaction its response is finished under.

use super::super::{Backend, DocumentSourceResult};
use super::{OriginalCodeActionSource, ReadonlyTransaction};
use tower_lsp_server::ls_types::Uri;

impl Backend {
    pub(in super::super) async fn capture_code_action_source(
        &self,
        uri: &Uri,
        snapshot: &super::super::DocumentState,
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
    pub(in super::super) async fn finish_code_action_response(
        &self,
        source: &OriginalCodeActionSource,
        actions: Vec<tcl_lsp_core::code_actions::CodeAction>,
        only: Option<&Vec<tower_lsp_server::ls_types::CodeActionKind>>,
    ) -> DocumentSourceResult<Option<Vec<tower_lsp_server::ls_types::CodeActionOrCommand>>> {
        self.code_action_response_transaction(source, actions, only)
            .await
            .map(ReadonlyTransaction::published)
    }

    pub(in super::super) async fn code_action_response_transaction(
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
            == super::super::vfs::WorkspaceSourceInventoryProvider::HostReported
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
                Some(super::super::lift_versioned_code_actions(
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
}

fn code_action_document_matches(
    expected: &super::super::DocumentState,
    current: &super::super::DocumentState,
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
