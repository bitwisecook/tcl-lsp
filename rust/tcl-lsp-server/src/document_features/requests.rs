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

//! Single-document providers: folding, symbols, completion, links, inlay hints and formatting.

use crate::uri_norm::DocumentPath as _;
use crate::{
    Arc, Backend, CompletionItem, CompletionParams, CompletionResponse, CoreLspRange,
    DocumentFormattingParams, DocumentLink, DocumentLinkParams, DocumentRangeFormattingParams,
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, FoldingRange, FoldingRangeParams,
    InlayHint, InlayHintKind, InlayHintLabel, InlayHintParams, Position, Range, TextEdit, Uri,
    core_bigip, core_completion, core_definition, core_document_links, core_formatting,
    core_inlay_hints, core_symbols, formatter_config_from, jsonrpc, lift_bigip_range,
    lift_completion_item, lift_document_symbol, lift_folding_range, lift_lsp_range,
};
use std::str::FromStr;

impl Backend {
    pub(crate) async fn serve_folding_range(
        &self,
        params: FoldingRangeParams,
    ) -> jsonrpc::Result<Option<Vec<FoldingRange>>> {
        if !self
            .feature_enabled("folding", &params.text_document.uri)
            .await
        {
            // `None`, never `Some([])` — and the same holds for every other
            // return path below.  An *empty* folding result is
            // not neutral in VS Code: its sticky-scroll model provider treats
            // the folding candidate as valid whenever the model is non-null,
            // so an authoritative empty list is accepted as a valid, terminal
            // sticky model and the chain never falls through to the
            // indentation model — sticky scroll stays permanently blank for
            // the whole session.  `None` is the only result that both lets
            // the folding UI fall back to built-in indentation folding and
            // lets sticky scroll fall through to its indentation model.
            return Ok(None);
        }
        let Some(doc) = self.read_local_document(&params.text_document.uri).await? else {
            return Ok(None);
        };
        // BIG-IP config documents fold on their stanza tree rather than the
        // Tcl brace walk (which, on non-Tcl config text, finds only comment
        // blocks and leaves every stanza unfoldable) — the folding twin of
        // the outline split in `document_symbol`.
        let bigip = Self::is_bigip_dialect(&doc.dialect);
        let ranges = if bigip {
            let text = doc.text.clone();
            // Pure-CPU tokenise/segment work; run on a worker so a parser
            // panic is contained as a JSON-RPC error rather than unwinding
            // the event loop (defence in depth).
            crate::rt::spawn_blocking(move || core_bigip::folding_ranges(&text))
                .await
                .map_err(|err| jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: format!("folding worker panicked: {err}").into(),
                    data: None,
                })?
        } else if let Some(ranges) = self.db_folding_ranges(&params.text_document.uri).await {
            // Served from the salsa query graph (memoised); avoids a fresh
            // segmenter/registry walk on every request.
            ranges
        } else {
            // Cold / cancelled fallback: compute directly so the request
            // never returns empty due to a concurrent edit (behaviour
            // preserved).
            let analysis = self
                .analysis_for(
                    &params.text_document.uri,
                    doc.text.clone(),
                    doc.dialect.clone(),
                )
                .await?;
            crate::rt::spawn_blocking(move || {
                tcl_lsp_core::folding::folding_ranges_with_analysis(&doc.text, &analysis)
            })
            .await
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("folding worker panicked: {err}").into(),
                data: None,
            })?
        };
        if ranges.is_empty() {
            return Ok(None);
        }
        Ok(Some(ranges.into_iter().map(lift_folding_range).collect()))
    }

    pub(crate) async fn serve_document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> jsonrpc::Result<Option<DocumentSymbolResponse>> {
        if !self
            .feature_enabled("documentSymbols", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let Some(doc) = self.read_local_document(&params.text_document.uri).await? else {
            return Ok(None);
        };
        // BIG-IP config documents get a `module → kind → object` outline
        // built from their stanza tree rather than the Tcl scope walk
        // (which would find nothing in non-Tcl config text). Nameless
        // singletons fall back to their kind label so no outline symbol
        // ever carries an empty `name`.
        // The two compute branches run pure-CPU on a worker so a parser
        // panic is contained as a JSON-RPC error.
        let symbols = if Self::is_bigip_dialect(&doc.dialect) {
            let text = doc.text.clone();
            crate::rt::spawn_blocking(move || core_bigip::document_symbols(&text))
                .await
                .map_err(|err| jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: format!("document_symbol worker panicked: {err}").into(),
                    data: None,
                })?
        } else {
            // Resolve the declaration-vs-script route from the same current
            // document analysis that owns its workspace registry and lexer
            // configuration. A profile label alone can miss an installed
            // declaration grammar overlay.
            let analysis = self
                .analysis_for(
                    &params.text_document.uri,
                    doc.text.clone(),
                    doc.dialect.clone(),
                )
                .await?;
            let text = doc.text.clone();
            if tcl_lsp_core::declaration_outline::is_declaration_document_from_analysis(
                &text, &analysis,
            ) {
                crate::rt::spawn_blocking(move || {
                    core_symbols::declaration_symbols_from_analysis(&text, &analysis)
                })
                .await
                .map_err(|err| jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: format!("document_symbol worker panicked: {err}").into(),
                    data: None,
                })?
            } else if let Some(symbols) = self.db_document_symbols(&params.text_document.uri).await
            {
                // Served from the salsa query graph (memoised; reuses the
                // tracked file_analysis instead of re-running projection).
                symbols
            } else {
                // The direct analysis is current and complete for this request.
                crate::rt::spawn_blocking(move || {
                    core_symbols::document_symbols_from_analysis(&text, &analysis)
                })
                .await
                .map_err(|err| jsonrpc::Error {
                    code: jsonrpc::ErrorCode::InternalError,
                    message: format!("document_symbol worker panicked: {err}").into(),
                    data: None,
                })?
            }
        };
        let lifted: Vec<DocumentSymbol> = symbols.into_iter().map(lift_document_symbol).collect();
        Ok(Some(DocumentSymbolResponse::Nested(lifted)))
    }

    pub(crate) async fn serve_completion(
        &self,
        params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        if !self
            .feature_enabled(
                "completion",
                &params.text_document_position.text_document.uri,
            )
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document_position.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let pos = params.text_document_position.position;
        // Fetch the cached per-dialect registry on the async
        // path before spawning so the worker can read built-in
        // command names alongside the user-defined procs the
        // analyser surfaces.  Threads through as
        // Completion here only knows user procs / vars, not the
        // registry's command set.
        let registry = self.registry_for_dialect(&doc.dialect).await;
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        // Share the cross-document index with the worker so it can
        // enumerate procs from sibling files.  Move a refcounted
        // handle in and take the read lock inside the blocking
        // closure: the heavy walk holds a shared read lock, letting
        // other readers proceed concurrently while only writers wait.
        let workspace_index = Arc::clone(&self.workspace_index);
        // Pure-CPU work; spawn_blocking off the LSP event loop.
        let items = crate::rt::spawn_blocking(move || {
            let workspace = workspace_index.blocking_read();
            core_completion::completions(
                &doc.text,
                pos.line,
                pos.character,
                &analysis,
                Some(&registry),
                Some(&*workspace),
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("completion worker panicked: {err}").into(),
            data: None,
        })?;
        let lifted: Vec<CompletionItem> = items
            .into_iter()
            .map(|item| lift_completion_item(item, pos.line))
            .collect();
        Ok(Some(CompletionResponse::Array(lifted)))
    }

    pub(crate) async fn serve_document_link(
        &self,
        params: DocumentLinkParams,
    ) -> jsonrpc::Result<Option<Vec<DocumentLink>>> {
        if !self
            .feature_enabled("documentLinks", &params.text_document.uri)
            .await
        {
            return Ok(None);
        }
        let uri = params.text_document.uri.clone();
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        // BIG-IP `.conf`/`.scf`: links are object references (iRule-body and
        // supported TMSH property values) resolving to the target object's
        // stanza in the same document — a `uri#L<line>` fragment anchor. This
        // is a different engine from the Tcl `source`/`package require` link
        // scanner below.
        if Self::is_bigip_dialect(&doc.dialect) {
            let text = doc.text.clone();
            let bigip_links =
                crate::rt::spawn_blocking(move || tcl_bigip::links::document_links(&text))
                    .await
                    .map_err(|err| jsonrpc::Error {
                        code: jsonrpc::ErrorCode::InternalError,
                        message: format!("bigip document_link worker panicked: {err}").into(),
                        data: None,
                    })?;
            if bigip_links.is_empty() {
                return Ok(None);
            }
            let uri_str = uri.as_str();
            let lifted = bigip_links
                .into_iter()
                .map(|l| DocumentLink {
                    range: lift_bigip_range(l.range),
                    // Resolved → a `uri#L<1-based-line>` fragment the editor
                    // navigates to; unresolved → no target (hover-only).
                    target: l
                        .target_line
                        .and_then(|line| Uri::from_str(&format!("{uri_str}#L{}", line + 1)).ok()),
                    tooltip: Some(l.tooltip),
                    data: None,
                })
                .collect();
            return Ok(Some(lifted));
        }
        // Pass the document's
        // enclosing directory as the workspace root so
        // relative `source <path>` arguments resolve.  When
        // the URI isn't a `file://` URL we leave the workspace
        // root unset and only absolute paths surface as links.
        let doc_path = uri
            .document_path()
            .and_then(|p| p.to_str().map(str::to_owned));
        let workspace_root = uri
            .document_path()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf))
            .and_then(|p| p.to_str().map(str::to_owned));
        // The document's own path is what `[info script]` answers while it is
        // being sourced, so a computed `source [file dirname [info script]]`
        // path resolves through the same evaluator the source graph uses
        // instead of being fabricated from raw text.
        let home = std::env::var("HOME").ok();
        // The document's imported constants, so a link through
        // a value an ancestor document assigns resolves exactly as the same
        // line resolves for navigation.
        let imported = {
            let index = self.workspace_index.read().await;
            index.imported_path_constants_for(uri.as_str())
        };
        // Pure-CPU segmentation on a worker so a parser panic is contained
        // as a JSON-RPC error.
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let text = doc.text.clone();
        let links = crate::rt::spawn_blocking(move || {
            core_document_links::document_links_from_analysis(
                &text,
                &analysis,
                &core_document_links::LinkContext {
                    imported_constants: Some(&imported),
                    workspace_root: workspace_root.as_deref(),
                    home: home.as_deref(),
                    script_path: doc_path.as_deref(),
                },
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("document_link worker panicked: {err}").into(),
            data: None,
        })?;
        if links.is_empty() {
            return Ok(None);
        }
        let lifted = links
            .into_iter()
            .map(|l| DocumentLink {
                range: Range {
                    start: Position {
                        line: l.start_line,
                        character: l.start_character,
                    },
                    end: Position {
                        line: l.end_line,
                        character: l.end_character,
                    },
                },
                target: Uri::from_str(&l.target).ok(),
                tooltip: l.tooltip,
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    pub(crate) async fn serve_inlay_hint(
        &self,
        params: InlayHintParams,
    ) -> jsonrpc::Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri.clone();
        // Inlay hints are opt-in (default off), with the type-hint and
        // parameter-hint families gated independently.  The provider must
        // still answer with a well-formed (empty) list — never an error or
        // `null` — when both are disabled.
        let type_hints = self.inlay_family_enabled(&uri, "inlayTypeHints").await;
        let parameter_hints = self.inlay_family_enabled(&uri, "inlayParameterHints").await;
        if !type_hints && !parameter_hints {
            return Ok(Some(Vec::new()));
        }
        let Some(doc) = self.read_document(&uri).await? else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let analysis = self
            .analysis_for(&uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let registry = self.registry_for_dialect(&doc.dialect).await;
        // Build analysis on a worker so the inlay-hints provider can surface
        // inferred-type hints (`inlayTypeHints`) and parameter-name hints at
        // user-proc call sites plus built-in command synopsis hints
        // (`inlayParameterHints`).  When the analyser surfaces an empty
        // all_procs map (no user procs in the document), the provider still
        // returns built-in hints from the registry.
        // The parameter labels name the reached proc's parameters, so the
        // hint provider needs the same whole-program export view definition
        // and hover use — a `-force` shadow changes which proc that is.
        let exports = self.export_snapshot().await;
        let uri_key = uri.as_str().to_owned();
        let hints = crate::rt::spawn_blocking(move || {
            core_inlay_hints::inlay_hints_in_program(
                &doc.text,
                tcl_lsp_core::profile_for_dialect(&doc.dialect),
                range,
                Some(&analysis),
                core_definition::CallResolution {
                    registry: Some(&registry),
                    program: Some(core_definition::ProgramExports {
                        uri: &uri_key,
                        oracle: exports.as_ref(),
                    }),
                },
                type_hints,
                parameter_hints,
            )
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("inlay_hint worker panicked: {err}").into(),
            data: None,
        })?;
        if hints.is_empty() {
            return Ok(None);
        }
        let lifted = hints
            .into_iter()
            .map(|h| InlayHint {
                position: Position {
                    line: h.position_line,
                    character: h.position_character,
                },
                label: InlayHintLabel::String(h.label),
                kind: Some(match h.kind {
                    core_inlay_hints::InlayHintKind::Type => InlayHintKind::TYPE,
                    core_inlay_hints::InlayHintKind::Parameter => InlayHintKind::PARAMETER,
                }),
                text_edits: None,
                tooltip: None,
                padding_left: h.padding_left.then_some(true),
                padding_right: None,
                data: None,
            })
            .collect();
        Ok(Some(lifted))
    }

    pub(crate) async fn serve_formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_local_document(&params.text_document.uri).await? else {
            return Ok(None);
        };
        let analysis = self
            .analysis_for(
                &params.text_document.uri,
                doc.text.clone(),
                doc.dialect.clone(),
            )
            .await?;
        // Build from the resolved `tclLsp.formatting` settings; the client's
        // `FormattingOptions.tabSize` / `insertSpaces` override indentation by
        // LSP contract.
        let formatting = self.resolved_formatting(&params.text_document.uri).await;
        let config = formatter_config_from(
            &formatting,
            &params.options,
            tcl_lsp_core::profile_for_dialect(&doc.dialect),
        );
        // Pure-CPU formatting on a worker so a parser panic is contained as
        // a JSON-RPC error.  The formatter is one of the two consumers that
        // must see the document's *real* terminators (`lineEnding: auto`
        // sniffs them, and the whole-document replace range is measured
        // against them) — see `DocumentState::raw`.
        let text = doc.raw().to_owned();
        let edits = crate::rt::spawn_blocking(move || {
            core_formatting::formatting_with_analysis(&text, &config, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("formatting worker panicked: {err}").into(),
            data: None,
        })?;
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }

    pub(crate) async fn serve_range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> jsonrpc::Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.read_local_document(&params.text_document.uri).await? else {
            return Ok(None);
        };
        let range = CoreLspRange {
            start_line: params.range.start.line,
            start_character: params.range.start.character,
            end_line: params.range.end.line,
            end_character: params.range.end.character,
        };
        let analysis = self
            .analysis_for(
                &params.text_document.uri,
                doc.text.clone(),
                doc.dialect.clone(),
            )
            .await?;
        let formatting = self.resolved_formatting(&params.text_document.uri).await;
        let config = formatter_config_from(
            &formatting,
            &params.options,
            tcl_lsp_core::profile_for_dialect(&doc.dialect),
        );
        // Pure-CPU formatting on a worker so a parser panic is contained as
        // a JSON-RPC error.
        let text = doc.raw().to_owned();
        let edits = crate::rt::spawn_blocking(move || {
            core_formatting::range_formatting_with_analysis(&text, range, &config, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("range_formatting worker panicked: {err}").into(),
            data: None,
        })?;
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(
            edits
                .into_iter()
                .map(|e| TextEdit {
                    range: lift_lsp_range(e.range),
                    new_text: e.new_text,
                })
                .collect(),
        ))
    }
}
