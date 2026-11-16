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

//! The code action and code lens request handlers.

use crate::{
    Backend, CodeActionOrCommand, CodeActionParams, CodeLens, CodeLensParams, Position, Uri,
    core_code_lens, jsonrpc, lift_lsp_range, readonly_reference_reports,
};

impl Backend {
    pub(crate) async fn serve_code_lens(
        &self,
        params: CodeLensParams,
    ) -> jsonrpc::Result<Option<Vec<CodeLens>>> {
        if !self
            .feature_enabled("codeLens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        if !analysis.allows_lexical_declaration_advice() {
            let workspace = self.workspace_index.read().await;
            let lenses = core_code_lens::code_lenses(
                &doc.text,
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
                Some(&analysis),
                Some(&workspace),
                uri.as_str(),
            );
            drop(workspace);
            // The unresolved tag follows the same single-document census
            // scope as the resolved report.
            let retained_documents = if lenses.iter().any(|lens| {
                lens.reference_coverage
                    .is_some_and(readonly_reference_reports::is_source_census)
            }) {
                // An unavailable inventory degrades the tag, never the request.
                self.original_declaration_documents(&uri)
                    .await
                    .map_or(0, |documents| documents.len())
            } else {
                1
            };
            let mut lifted = Vec::new();
            for lens in lenses {
                let Some(mut data) = self
                    .retain_original_declaration_data(
                        &uri,
                        &doc.text,
                        &analysis,
                        lens.identity.as_ref(),
                    )
                    .await
                else {
                    continue;
                };
                data["uri"] = serde_json::Value::String(uri.to_string());
                // Presentation only: resolve reads the declaration handle.
                data["qname"] = serde_json::Value::String(lens.qname);
                if let Some(coverage) = lens.reference_coverage {
                    data["sourceReferenceCoverage"] =
                        serde_json::json!(readonly_reference_reports::coverage_tag(
                            readonly_reference_reports::single_document_coverage(
                                coverage,
                                retained_documents,
                            )
                        ));
                }
                lifted.push(CodeLens {
                    range: lift_lsp_range(lens.range),
                    data: Some(data),
                    command: None,
                });
            }
            return Ok((!lifted.is_empty()).then_some(lifted));
        }
        Ok(self
            .readonly_logical_lenses(&uri)
            .await?
            .filter(|lenses| !lenses.is_empty()))
    }

    /// Resolve a current readonly source lens by its retained opaque identity.
    /// Client labels and old ranges cannot select a declaration. Native and
    /// Logical schemas remain separate; partial positive counts never close a
    /// reference census, and a stale or unavailable selection stays inert.
    pub(crate) async fn serve_code_lens_resolve(
        &self,
        lens: CodeLens,
    ) -> jsonrpc::Result<CodeLens> {
        if lens
            .data
            .as_ref()
            .is_some_and(|data| data.get("originalDeclaration").is_some())
        {
            let mut lens = lens;
            lens.command = None;
            let Some(uri) = lens
                .data
                .as_ref()
                .and_then(|data| data.get("uri"))
                .and_then(serde_json::Value::as_str)
                .and_then(|uri| uri.parse::<Uri>().ok())
            else {
                return Ok(lens);
            };
            let Some(doc) = self.read_document(&uri).await? else {
                return Ok(lens);
            };
            let analysis = self
                .analysis_for(&uri, doc.text.clone(), doc.dialect)
                .await?;
            let Some(identity) = self
                .resolve_original_declaration_data(lens.data.as_ref(), &uri, &doc.text, &analysis)
                .await
            else {
                return Ok(lens);
            };
            let index = tcl_lexer::LineIndex::new_lsp(&doc.text);
            let selected = index.position_at_utf16(identity.span().start(), &doc.text);
            let position = Position {
                line: selected.line,
                character: selected.character.get(),
            };
            let report = self
                .readonly_original_declaration_reference_report(&identity, &uri, false)
                .await?
                .unwrap_or_else(
                    readonly_reference_reports::OriginalReadonlyReferenceReport::unavailable,
                );
            if let Some(data) = lens.data.as_mut() {
                data["sourceReferenceCoverage"] =
                    serde_json::json!(readonly_reference_reports::coverage_tag(report.coverage));
            }
            lens.command = Some(report.lens_command(&uri, position));
            return Ok(lens);
        }
        let mut lens = lens;
        lens.command = None;
        let Some((uri, handle)) = lens.data.as_ref().and_then(|data| {
            let uri = data.get("uri")?.as_str()?.parse::<Uri>().ok()?;
            let handle = serde_json::from_value(data.get("logicalSourceLens")?.clone()).ok()?;
            Some((uri, handle))
        }) else {
            // Old qname/range payloads and unknown handles cannot reissue a
            // source declaration or borrow the current document's authority.
            return Ok(lens);
        };
        Ok(self
            .readonly_logical_lens_resolve(&uri, handle)
            .await?
            .unwrap_or(lens))
    }

    pub(crate) async fn serve_code_action(
        &self,
        params: CodeActionParams,
    ) -> jsonrpc::Result<Option<Vec<CodeActionOrCommand>>> {
        if !self
            .feature_enabled("codeActions", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let (target, request) = (&uri, &params);
        readonly_reference_reports::retry_readonly_attempt(None, move || {
            self.code_action_attempt(target, request)
        })
        .await
    }
}
