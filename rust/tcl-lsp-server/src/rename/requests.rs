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

//! The rename and prepare-rename request handlers.

use crate::{
    Backend, ControlFlow, PrepareRenameResponse, RenameParams, TextDocumentPositionParams,
    WorkspaceEdit, core_definition, core_rename, jsonrpc, lift_lsp_range,
    readonly_reference_reports,
};

impl Backend {
    pub(crate) async fn serve_prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> jsonrpc::Result<Option<PrepareRenameResponse>> {
        let conditional_uri = params.text_document.uri.clone();
        let conditional_uri_generation = self.live_publication_uri_generation(&conditional_uri);
        if !self
            .feature_enabled("rename", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let pos = params.position;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if let std::ops::ControlFlow::Break(prepared) = self
            .original_command_prepare_tier(&uri, &doc, &analysis, pos)
            .await?
        {
            return Ok(prepared);
        }
        let text = doc.text.clone();
        let analysis_for_worker = analysis.clone();
        // A rename started from a `-force`-shadowed call must offer the
        // definition that call reaches, not the local one the import deleted —
        // the same view definition uses.
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let result = crate::rt::spawn_blocking(move || {
            core_rename::prepare_rename_in_program(
                &text,
                pos.line,
                pos.character,
                &analysis_for_worker,
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
            message: format!("prepare_rename worker panicked: {err}").into(),
            data: None,
        })?;
        if let Some(p) = result {
            return Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                range: lift_lsp_range(p.range),
                placeholder: p.placeholder,
            }));
        }
        if let ControlFlow::Break(prepared) = self
            .conditional_variable_prepare_tier(&uri, &doc, pos, conditional_uri_generation)
            .await?
        {
            return Ok(prepared);
        }
        if Self::original_namespace_cursor_retained(&doc.text, &analysis, pos)
            || tcl_lsp_core::variable_symbol::select(&doc.text, &analysis, pos.line, pos.character)
                .is_break()
            || tcl_lsp_core::method_symbol::select(&doc.text, &analysis, pos.line, pos.character)
                .is_break()
        {
            return Ok(None);
        }
        // Consumer-document fall-through: the local analysis has no
        // declaration to anchor prepare on, but the cursor symbol may still
        // resolve through the workspace index (a sibling document or an
        // autoloaded library defines it) — exactly the case the rename
        // handler's workspace-resolved branch serves.  VS Code gates every
        // rename behind prepare, so refusing here would make that branch
        // unreachable from the editor.  Accept with the call-site word's own
        // range when the workspace resolves it.
        let Some(word_p) = core_rename::word_prepare_at(&doc.text, pos.line, pos.character) else {
            return Ok(None);
        };
        if !self
            .resolve_workspace_symbols_for_binding(&uri, &doc.text, &analysis, pos)
            .await?
            .is_empty()
        {
            return Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                range: lift_lsp_range(word_p.range),
                placeholder: word_p.placeholder,
            }));
        }
        Ok(None)
    }

    pub(crate) async fn serve_rename(
        &self,
        params: RenameParams,
    ) -> jsonrpc::Result<Option<WorkspaceEdit>> {
        // A refusal computed against a workspace snapshot that moved during
        // the request says nothing about the current workspace: plan again.
        for attempt in 0..readonly_reference_reports::READONLY_CURRENCY_ATTEMPTS {
            if attempt != 0 {
                let _ = crate::rt::timeout(
                    readonly_reference_reports::source_views::SOURCE_VIEW_CONTENTION_BACKOFF,
                    self.live_publication_advanced.notified(),
                )
                .await;
            }
            let mut plan = None;
            let result = self.rename_attempt(params.clone(), &mut plan).await?;
            let Some(plan) = plan else {
                return Ok(result);
            };
            if result.is_some() || self.workspace_source_plan_is_current(plan).await {
                return Ok(result);
            }
        }
        Ok(None)
    }
}
