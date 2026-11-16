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

//! The semantic-token request handlers.

use crate::{
    Arc, Backend, ConvergenceGuard, LspSemanticTokens, RangeConvergenceInputs, RangeSettleOutcome,
    SemanticTokensDeltaParams, SemanticTokensFullDeltaResult, SemanticTokensParams,
    SemanticTokensRangeParams, SemanticTokensRangeResult, SemanticTokensResult,
    coarse_semantic_source_changed, core_semantic_range, jsonrpc, lift_semantic_token_data,
    log_range_convergence_settled, range_token_data, readonly_reference_reports,
};

impl Backend {
    pub(crate) async fn serve_semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        if !self
            .feature_enabled("semanticTokens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let target = &uri;
        readonly_reference_reports::retry_readonly_attempt(
            Err(coarse_semantic_source_changed()),
            move || self.semantic_tokens_full_attempt(target),
        )
        .await?
    }

    pub(crate) async fn serve_semantic_tokens_full_delta(
        &self,
        params: SemanticTokensDeltaParams,
    ) -> jsonrpc::Result<Option<SemanticTokensFullDeltaResult>> {
        if !self
            .feature_enabled("semanticTokens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let (target, request) = (&uri, &params);
        readonly_reference_reports::retry_readonly_attempt(
            Err(coarse_semantic_source_changed()),
            move || self.semantic_tokens_full_delta_attempt(target, request),
        )
        .await?
    }

    pub(crate) async fn serve_semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> jsonrpc::Result<Option<SemanticTokensRangeResult>> {
        // Gate on the same `semanticTokens` toggle as `full` / `full_delta`, so
        // disabling semantic tokens also silences viewport (range) requests —
        // otherwise range-request clients keep rendering highlights the user
        // turned off (issue 174).
        if !self
            .feature_enabled("semanticTokens", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let Some(doc) = self
            .read_document_for_semantic_tokens(&params.text_document.uri)
            .await?
        else {
            return Ok(None);
        };
        // `S-semantic-tokens-rich`: filter tokens to those that
        // start inside the requested range.  Editors call this
        // when only the visible viewport needs colouring.
        let core_range = core_semantic_range(params.range);
        if let Some(tokens) = self
            .non_tcl_range_tokens(&params.text_document.uri, &doc, core_range)
            .await
        {
            // Settle here too: an APL / BIG-IP viewport never touches the Tcl
            // compilation unit, so there is no enriched tier to converge to —
            // and an observer waiting on the marker must not be left waiting for
            // a decision that will never be made.
            log_range_convergence_settled(
                &self.client,
                params.text_document.uri.as_str(),
                false,
                RangeSettleOutcome::NoAnalysis,
            )
            .await;
            return Ok(Some(SemanticTokensRangeResult::Tokens(LspSemanticTokens {
                result_id: None,
                data: lift_semantic_token_data(&tokens.data),
            })));
        }
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let uri = &params.text_document.uri;
        // Race the memoised unit + analysis against the fast-path budget; on
        // timeout `pending` carries the still-running reads to the convergence
        // continuation below (see `race_range_enriched_reads`).
        let (cached_cu, cached_analysis, pending, had_analysis_handles) =
            self.race_range_enriched_reads(uri).await;
        let cached_analysis = if had_analysis_handles {
            cached_analysis
        } else {
            Some(
                self.fresh_analysis_for(uri, Arc::clone(&doc.text), doc.dialect.clone())
                    .await?,
            )
        };
        // Distinguishes the two no-continuation cases for the settled marker
        // below: the enriched reads landed inside the budget (so this response
        // *is* the enriched viewport), versus the document having no analysis
        // handles at all.
        // Both inputs are required for the complete enriched tier. A Salsa
        // write can cancel one read after its sibling completed; that partial
        // result is useful for this response but remains retryable rather than
        // being mislabelled final.
        let served_enriched = cached_cu.is_some() && cached_analysis.is_some();
        // Pure-CPU tokenisation on a worker so a parser panic is contained
        // as a JSON-RPC error.  The text goes in behind an `Arc` so the
        // convergence continuation below shares this buffer instead of taking a
        // second full-document copy.
        let text: Arc<str> = Arc::clone(&doc.text);
        let dialect = doc.dialect.clone();
        let serve_text = Arc::clone(&text);
        let serve_dialect = dialect.clone();
        let serve_registry = Arc::clone(&registry);
        let core_data = crate::rt::spawn_blocking(move || {
            range_token_data(
                had_analysis_handles,
                cached_analysis.as_deref(),
                &serve_text,
                core_range,
                &serve_dialect,
                &serve_registry,
                cached_cu.as_deref(),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("semantic_tokens_range worker panicked: {err}").into(),
            data: None,
        })?;

        // Convergence: if the coarse tier was served because the
        // enriched reads overran the budget, detach a continuation that awaits
        // them and refreshes once the enriched viewport differs.
        if let Some(pending) = pending {
            // At most one continuation per document: a client scrolling
            // a cold file issues a viewport request per frame, and each detached
            // continuation holds the document and queues a blocking recompute
            // only to ask for the same workspace-wide refresh the pending one
            // will already ask for.
            if let Some(guard) = ConvergenceGuard::claim(&self.semantic_tokens_convergence, uri) {
                self.spawn_range_convergence(
                    RangeConvergenceInputs {
                        uri: uri.as_str().to_owned(),
                        served: core_data.clone(),
                        registry,
                        text,
                        dialect,
                        range: core_range,
                    },
                    pending,
                    guard,
                );
            } else {
                // Settle here: the in-flight continuation settles under its own
                // request's marker, so without this one an observer waiting on
                // *this* request's decision would wait for a line that never
                // comes.
                log_range_convergence_settled(
                    &self.client,
                    uri.as_str(),
                    false,
                    RangeSettleOutcome::Coalesced,
                )
                .await;
            }
        } else {
            // No continuation to detach, so settle here instead: this request's
            // convergence decision is already final and an observer waiting on
            // the marker must not be left waiting for a continuation that will
            // never run. Which of the two no-continuation cases it was is
            // recorded in `outcome`.
            let (refreshed, outcome) =
                self.inline_range_settlement(served_enriched, had_analysis_handles);
            log_range_convergence_settled(&self.client, uri.as_str(), refreshed, outcome).await;
        }

        Ok(Some(SemanticTokensRangeResult::Tokens(LspSemanticTokens {
            result_id: None,
            data: lift_semantic_token_data(&core_data),
        })))
    }
}
