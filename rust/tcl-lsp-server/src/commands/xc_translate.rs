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

//! Translation of iRules to F5 Distributed Cloud constructs.

use super::{Arc, Backend, document_transform, jsonrpc};

impl Backend {
    /// Handle `tcl-lsp.xcTranslate`: statically translate an iRule to F5
    /// Distributed Cloud constructs and report the result.
    ///
    /// Arguments are `[source, output_format?]`, where `output_format` is
    /// `"terraform"` | `"json"` | `"console"` | `"both"` (default). The
    /// `"console"` rendering is one pasteable F5 XC Console document per
    /// object, in `console_objects`; `"both"` stays terraform plus JSON API.
    /// The payload is
    /// `f5-xc`'s own reporting shape — the same one the `xc_translate` MCP
    /// tool returns — so a client renders either source identically:
    /// `terraform` HCL and `json_api` documents, `coverage_pct` with the
    /// per-status counts, and the status-tagged `items` list.
    ///
    /// Blank source is `null`, not an `{error}` object: there is nothing to
    /// translate and the editors already refuse to send an empty buffer.
    pub(in super::super) async fn xc_translate_command(
        &self,
        args: &[serde_json::Value],
    ) -> Option<serde_json::Value> {
        let source = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if source.trim().is_empty() {
            return None;
        }
        let output_format =
            f5_xc::OutputFormat::from_arg(args.get(1).and_then(serde_json::Value::as_str));
        // Translation lowers the whole document to IR — pure CPU work that
        // belongs off the LSP event loop.  A panic in the walk is contained
        // and surfaced as the `{error}` object the clients already render.
        let registry = self.registry_for_dialect("f5-irules").await;
        let value = crate::rt::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let result = f5_xc::translate_irule_with_registry(&source, &registry);
                f5_xc::translation_payload(
                    &result,
                    f5_xc::DEFAULT_NAMESPACE,
                    f5_xc::DEFAULT_LB_NAME,
                    output_format,
                )
            }))
        })
        .await;
        match value {
            Ok(Ok(payload)) => Some(payload),
            _ => Some(serde_json::json!({
                "error": "the XC translator failed to analyse the source",
            })),
        }
    }

    /// Handle `tcl-lsp.xcTranslateDocument`: translate an open iRules
    /// document under its own analysis.
    ///
    /// Arguments are `[{uri, version, source}, output_format?]`. The snapshot
    /// must still be the server's copy of the document; the translation then
    /// walks the document's retained analysis, registry and a unit built from
    /// that analysis ([`f5_xc::translate_irule_for_document`]), and is
    /// published only while the same version and source are current. The
    /// payload is [`Self::xc_translate_command`]'s, plus `source` naming the
    /// document, version and dialect it describes.
    ///
    /// Anything else answers `{unavailable: reason}` — `request`, `notOpen`,
    /// `stale`, `notIrules` or `analysis` — never a standalone reparse: the
    /// source-only `tcl-lsp.xcTranslate` remains the explicit standalone
    /// purpose for text that is not such a document.
    pub(in super::super) async fn xc_translate_document_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let unavailable = |reason: &str| Ok(Some(serde_json::json!({ "unavailable": reason })));
        let Some(document_transform::TransformRequest::Snapshot {
            uri,
            version,
            source,
        }) = document_transform::TransformRequest::parse(args.first())
        else {
            return unavailable("request");
        };
        let output_format =
            f5_xc::OutputFormat::from_arg(args.get(1).and_then(serde_json::Value::as_str));
        let Some(doc) = self.read_local_document(&uri).await? else {
            return unavailable("notOpen");
        };
        if doc.version != Some(version) || doc.raw() != source.as_str() {
            return unavailable("stale");
        }
        if !tcl_lsp_core::profile_for_dialect(&doc.dialect).is_irules() {
            return unavailable("notIrules");
        }
        let Ok(analysis) = self
            .analysis_for(&uri, Arc::clone(&doc.text), doc.dialect.clone())
            .await
        else {
            return unavailable("analysis");
        };
        let text = Arc::clone(&doc.text);
        let value = crate::rt::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let registry = analysis.resolved_registry()?;
                let unit = tcl_lsp_db::compilation_unit_uncached_from_analysis(
                    &text, registry, &analysis, None,
                )?;
                let result = f5_xc::translate_irule_for_document(&text, &unit, &analysis)?;
                Some(f5_xc::translation_payload(
                    &result,
                    f5_xc::DEFAULT_NAMESPACE,
                    f5_xc::DEFAULT_LB_NAME,
                    output_format,
                ))
            }))
        })
        .await;
        let mut payload = match value {
            Ok(Ok(Some(payload))) => payload,
            Ok(Ok(None)) => return unavailable("analysis"),
            _ => {
                return Ok(Some(serde_json::json!({
                    "error": "the XC translator failed to analyse the source",
                })));
            }
        };
        let Some(current) = self.read_local_document(&uri).await? else {
            return unavailable("notOpen");
        };
        if current.version != doc.version
            || current.raw() != doc.raw()
            || current.dialect != doc.dialect
        {
            return unavailable("stale");
        }
        if let Some(object) = payload.as_object_mut() {
            object.insert(
                "source".to_owned(),
                serde_json::json!({
                    "purpose": "document",
                    "uri": uri.as_str(),
                    "version": version,
                    "dialect": doc.dialect,
                }),
            );
        }
        Ok(Some(payload))
    }
}
