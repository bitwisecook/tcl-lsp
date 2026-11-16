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

//! Minify, optimise and unminify.

use super::{Arc, Backend, core_minify, core_report, document_transform, jsonrpc};

impl Backend {
    /// Handle the `tcl-lsp.minifyDocument` workspace command.
    ///
    /// Arguments (positional, matching the editor extensions):
    /// `[uri-or-source-snapshot, compact: bool, aggressive: bool, isolated: bool]`.
    /// A snapshot `{uri, version, source}` also receives an original result
    /// receipt for post-dialog validation. Legacy URI-only results are unbound.
    /// Returns a JSON object with `source`, `originalLength`,
    /// `minifiedLength`, and — for the compact / aggressive tiers —
    /// `symbolMap` (and `optimisationsApplied` for aggressive).
    pub(in super::super) async fn minify_document_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(request) = document_transform::TransformRequest::parse(args.first()) else {
            return Ok(None);
        };
        let uri = request.uri().clone();
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let source_receipt = self.capture_transform_source(&request, &doc).await?;
        let compact = args
            .get(1)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let aggressive = args
            .get(2)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let isolated = args
            .get(3)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        // Keep the same source/configuration/Registry analysis used by the
        // document's other consumers. Presentation dialect labels cannot
        // reconstruct its naming policy or command store.
        let text = doc.text.clone();
        let analysis = self
            .analysis_for(&uri, Arc::clone(&doc.text), doc.dialect.clone())
            .await?;
        let worker = document_transform::TransformWorkerSource::for_document(
            &doc.text,
            &analysis,
            source_receipt.as_deref(),
        );
        let value = crate::rt::spawn_blocking(move || {
            let tier = if aggressive {
                core_minify::MinifyTier::Aggressive
            } else if compact {
                core_minify::MinifyTier::Compact
            } else {
                core_minify::MinifyTier::Default
            };
            let result = core_minify::minify_with_analysis(
                &text,
                &analysis,
                core_minify::MinifyOptions {
                    tier,
                    isolated,
                    ..core_minify::MinifyOptions::default()
                },
            );
            let refusals: Vec<_> = result
                .refusals
                .iter()
                .map(|refusal| {
                    serde_json::json!({ "code": refusal.code(), "reason": refusal.reason() })
                })
                .collect();
            let mut value = serde_json::json!({
                "source": result.source,
                "originalLength": result.original_length,
                "minifiedLength": result.minified_length(),
                "refusals": refusals,
            });
            if compact || aggressive {
                value["symbolMap"] = serde_json::json!(result.symbol_map.format());
            }
            if aggressive {
                value["optimisationsApplied"] = serde_json::json!(result.optimisations_applied);
            }
            value
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("minify worker panicked: {err}").into(),
            data: None,
        })?;
        self.finish_document_transform(source_receipt, worker.into_iter().collect(), value)
            .await
    }

    /// Handle the `tcl-lsp.optimiseDocument` workspace command.
    ///
    /// Arguments: `[uri-or-source-snapshot, profile?]`. Snapshot requests retain
    /// an original result receipt; URI-only results remain legacy and unbound.
    /// Runs the optimiser over the document
    /// for the profile in force's passes (multi-pass for `aggressive`,
    /// single-pass otherwise), applies the rewrites the document's policy
    /// shows, and returns `{source, optimisations}` — the optimised text
    /// plus the list of applied optimisation suggestions.
    ///
    /// This command is the batch form of the optimiser code actions, so it
    /// owes the user the policy the published O-code diagnostics apply
    /// (#2119): the `tclLsp.optimiser.enabled` master switch, the profile's
    /// category set with the per-code overrides, the inline suppression
    /// directives, and a group's edits all or none (#2149) — one shared
    /// rewrite loop ([`core_report::optimise_under_policy`]) under the
    /// folder's layers.
    ///
    /// The `profile` argument, when it names an `OptimisationProfile` — the
    /// vocabulary `tcl opt --profile` parses — is the profile in force, over
    /// the editor's setting and the project and global files
    /// ([`core_policy::PolicyBuilder::requested_profile`]): `optimiseDocument
    /// uri "full"` asks for the full profile, and answering it with a
    /// configured one (`readability` by default, where constant folding is
    /// opt-in) would refuse what it asked for. An unrecognised or absent name
    /// names none, so the configured profile decides. Either way the switch
    /// and the per-code overrides are the layers', and the pass count is the
    /// profile in force's, as it is for `tcl opt` and the MCP `optimize`
    /// tool.
    pub(in super::super) async fn optimise_document_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        use tcl_compiler::optimiser::profiles::OptimisationProfile;

        let Some(request) = document_transform::TransformRequest::parse(args.first()) else {
            return Ok(None);
        };
        let uri = request.uri().clone();
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let source_receipt = self.capture_transform_source(&request, &doc).await?;
        let requested = args
            .get(1)
            .and_then(serde_json::Value::as_str)
            .and_then(|name| {
                OptimisationProfile::ALL
                    .into_iter()
                    .find(|profile| profile.name() == name)
            });
        let analysis_text = tcl_lexer::normalise_lone_cr(&doc.text);
        let analysis = self
            .analysis_for(&uri, Arc::from(analysis_text.as_ref()), doc.dialect.clone())
            .await?;
        let registry = analysis
            .resolved_registry()
            .map(|registry| registry.snapshot().shared_registry());
        let layers = self.resolved_policy_layers(&uri).await;
        let text = analysis_text.into_owned();
        let dialect = doc.dialect.clone();
        let worker = document_transform::TransformWorkerSource::for_document(
            &doc.text,
            &analysis,
            source_receipt.as_deref(),
        );
        let value = crate::rt::spawn_blocking(move || {
            // A command the user invoked reads no whole-document gate:
            // `features.diagnostics` turns off the published squiggles, not
            // the rewrite asked for — `tcl opt`'s rule.
            let policy = layers
                .builder()
                .reporting(true)
                .requested_profile(requested)
                .dialect(tcl_lsp_core::profile_for_dialect(&dialect))
                .build();
            let passes = policy.optimiser.profile.max_iterations();
            let optimised = registry.as_ref().map_or_else(
                || core_report::OptimisedSource {
                    text: text.clone(),
                    applied: Vec::new(),
                    iterations: 0,
                },
                |registry| {
                    core_report::optimise_under_policy_from_analysis(
                        &text, registry, &analysis, passes, &policy,
                    )
                },
            );
            let (source, opts) = (optimised.text, optimised.applied);
            let line_index = tcl_lexer::LineIndex::new_lsp(&text);
            let items: Vec<serde_json::Value> = opts
                .iter()
                .map(|o| {
                    let start = line_index.position_at_utf16(o.span.start(), &text);
                    let end = line_index.position_at_utf16(o.span.end(), &text);
                    serde_json::json!({
                        "code": o.code.as_str(),
                        "message": o.message,
                        "startLine": start.line,
                        "startCharacter": start.character.get(),
                        "endLine": end.line,
                        "endCharacter": end.character.get(),
                        "replacement": o.replacement,
                        "group": o.group,
                        "hintOnly": o.hint_only,
                    })
                })
                .collect();
            serde_json::json!({
                "source": source,
                "optimisations": items,
            })
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("optimise worker panicked: {err}").into(),
            data: None,
        })?;
        self.finish_document_transform(source_receipt, worker.into_iter().collect(), value)
            .await
    }

    /// Handle the `tcl-lsp.unminifyError` workspace command.
    ///
    /// Arguments: `[errorMessage, symbolMapText, minifiedSource?,
    /// originalSource?]`.  Translates compacted names in the error
    /// message back to originals via the symbol map and returns
    /// `{originalError, translatedError, changed}`.  Source-
    /// correlated line remapping (the `minified` / `original`
    /// arguments) is not implemented.
    pub(in super::super) fn unminify_error_command(
        args: &[serde_json::Value],
    ) -> Option<serde_json::Value> {
        let error_message = args.first().and_then(serde_json::Value::as_str)?;
        let symbol_map_text = args
            .get(1)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let symbol_map = core_minify::SymbolMap::parse(symbol_map_text);
        let mut translated = core_minify::unminify_error(error_message, &symbol_map);
        // When both the minified and original sources are supplied,
        // remap minified line references to approximate original lines.
        let minified = args.get(2).and_then(serde_json::Value::as_str);
        let original = args.get(3).and_then(serde_json::Value::as_str);
        if let (Some(minified), Some(original)) = (minified, original)
            && !minified.is_empty()
            && !original.is_empty()
        {
            translated = core_minify::remap_line_references(&translated, minified, original);
        }
        Some(serde_json::json!({
            "originalError": error_message,
            "translatedError": translated,
            "changed": translated != error_message,
        }))
    }
}
