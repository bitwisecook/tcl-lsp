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

//! Read-only views of a document: Tk preview, diagram data and the compiler explorer.

use super::{Arc, Backend, Digest, FromStr, Sha256, Uri, core_tk_preview, jsonrpc};

fn sha256_hex(bytes: &[u8]) -> String {
    sha256_digest_hex(&Sha256::digest(bytes))
}

pub(in super::super) fn sha256_digest_hex(digest: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}")
            .expect("writing a SHA-256 digest to a String cannot fail");
    }
    encoded
}

/// The exact editor document and analysis-input generation supplied to an
/// Explorer request. The retained input carries the actual registry Arc; the
/// display dialect is descriptive and never reselects that registry.
#[derive(Clone)]
struct ExplorerDocumentInput {
    pub(in super::super) uri: Uri,
    pub(in super::super) source: String,
    document_text: Arc<str>,
    pub(in super::super) dialect: String,
    pub(in super::super) version: i32,
    pub(in super::super) revision: u64,
    pub(in super::super) inputs_epoch: u64,
    pub(in super::super) file: tcl_lsp_db::SourceFile,
    pub(in super::super) input: Arc<tcl_compiler::analyser::ResolvedAnalysisInput>,
}

impl Backend {
    /// Build the versioned, static Tk UI model for an open document.
    ///
    /// Arguments are `[ { "uri": string, "version"?: integer,
    /// "document_sha256"?: string } ]`. Source text is deliberately not
    /// accepted: preview must analyse the same versioned snapshot as
    /// diagnostics and navigation, not execute or trust an editor-supplied
    /// second copy of a workspace program. Clients whose platform LSP API does
    /// not expose its document version may send a SHA-256 of their open text;
    /// that fingerprint is compared without accepting a second source copy.
    /// The result is stamped with the snapshot version and fingerprint so a
    /// client can discard a response that lost a race with a later edit.
    pub(in super::super) async fn tk_preview_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(request) = args.first().and_then(serde_json::Value::as_object) else {
            return Ok(None);
        };
        let Some(uri_str) = request.get("uri").and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Uri::from_str(uri_str) else {
            return Ok(None);
        };
        let requested_version = match request.get("version") {
            None => None,
            Some(value) => {
                let Some(version) = value.as_i64().and_then(|value| i32::try_from(value).ok())
                else {
                    return Err(jsonrpc::Error {
                        code: jsonrpc::ErrorCode::InvalidParams,
                        message: "Tk preview version must be a 32-bit integer".into(),
                        data: None,
                    });
                };
                Some(version)
            }
        };
        let requested_sha256 = match request.get("document_sha256") {
            None => None,
            Some(value) => {
                let Some(hash) = value.as_str() else {
                    return Err(jsonrpc::Error {
                        code: jsonrpc::ErrorCode::InvalidParams,
                        message: "Tk preview document_sha256 must be a string".into(),
                        data: None,
                    });
                };
                if hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(jsonrpc::Error {
                        code: jsonrpc::ErrorCode::InvalidParams,
                        message: "Tk preview document_sha256 must be a SHA-256 hex digest".into(),
                        data: None,
                    });
                }
                Some(hash.to_ascii_lowercase())
            }
        };
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        if requested_version.is_some() && requested_version != doc.version {
            return Err(jsonrpc::Error {
                code: jsonrpc::ErrorCode::ContentModified,
                message: format!(
                    "Tk preview requested document version {:?}, but the open snapshot is {:?}",
                    requested_version, doc.version
                )
                .into(),
                data: None,
            });
        }
        // The editor fingerprints the bytes in its buffer. Analysis uses the
        // byte-length-preserving lone-CR normalisation in `doc.text`, but the
        // snapshot identity must use the client's original spelling.
        let document_sha256 = sha256_hex(doc.raw().as_bytes());
        if requested_sha256.is_some() && requested_sha256.as_deref() != Some(&document_sha256) {
            return Err(jsonrpc::Error {
                code: jsonrpc::ErrorCode::ContentModified,
                message: "Tk preview requested document text differs from the open snapshot".into(),
                data: None,
            });
        }
        let analysis = self
            .analysis_for(&uri, Arc::clone(&doc.text), doc.dialect.clone())
            .await?;
        let text = Arc::clone(&doc.text);
        let document_uri = uri_str.to_owned();
        let document_version = doc.version;
        let mut model = crate::rt::spawn_blocking(move || {
            core_tk_preview::analyse_tk_ui_with_analysis(&text, &analysis)
        })
        .await
        .map_err(|err| jsonrpc::Error {
            code: jsonrpc::ErrorCode::InternalError,
            message: format!("Tk preview worker panicked: {err}").into(),
            data: None,
        })?;
        model.document_uri = Some(document_uri);
        model.document_version = document_version;
        model.document_sha256 = Some(document_sha256);
        serde_json::to_value(model)
            .map(Some)
            .map_err(|err| jsonrpc::Error {
                code: jsonrpc::ErrorCode::InternalError,
                message: format!("Tk preview serialisation failed: {err}").into(),
                data: None,
            })
    }

    /// Handle `tcl-lsp.diagramData` through the shared structured diagram
    /// projection.  The CLI and MCP use the same `tcl-diagram` entry point;
    /// keeping the LSP on it preserves the full `{events, procedures}` shape
    /// (including flow, priority, and multiplicity) consumed by the editors.
    pub(in super::super) async fn diagram_data_command(
        &self,
        args: &[serde_json::Value],
    ) -> Option<serde_json::Value> {
        let source = args.first().and_then(serde_json::Value::as_str)?;
        let dialect = "f5-irules";
        let registry = self.registry_for_dialect(dialect).await;
        Some(tcl_diagram::diagram_data_for_dialect(
            source,
            &registry,
            tcl_lsp_core::profile_for_dialect(dialect),
        ))
    }

    /// Capture a document-bound Explorer request from the exact open
    /// document and Salsa analysis inputs. Callers without a document owner
    /// retain the standalone source-only behavior.
    async fn capture_explorer_document_input(
        &self,
        owner: &serde_json::Value,
        source: &str,
        dialect: &str,
    ) -> Option<ExplorerDocumentInput> {
        use std::str::FromStr;

        let uri = Uri::from_str(owner.get("uri")?.as_str()?).ok()?;
        let version: i32 = owner.get("version")?.as_i64()?.try_into().ok()?;
        let _inputs = self.analyser_inputs_gate.read().await;
        let inputs_epoch = self.diag_inputs_epoch();
        let document = self
            .documents
            .lock("capture_explorer_document_input")
            .await
            .get(&uri)
            .cloned()?;
        if document.dialect_resolution_pending()
            || !document.matches_open_publication(source, dialect, version)
        {
            return None;
        }
        let file = self.db_files.lock().await.get(&uri).copied()?;
        let config = self.resolved_db_config(&uri).await;
        let db = self.db.snapshot("capture_explorer_document_input").await;
        if file.text(&*db) != source || file.dialect(&*db) != dialect {
            return None;
        }
        let input = tcl_lsp_db::document_analysis_input(&*db, file, config).ok()?;
        drop(db);
        if self.diag_inputs_epoch() != inputs_epoch {
            return None;
        }
        let current = self
            .documents
            .lock("capture_explorer_document_input")
            .await
            .get(&uri)
            .is_some_and(|current| {
                current.revision == document.revision
                    && Arc::ptr_eq(&current.text, &document.text)
                    && !current.dialect_resolution_pending()
                    && current.matches_open_publication(source, dialect, version)
            });
        current.then_some(ExplorerDocumentInput {
            uri,
            source: source.to_owned(),
            document_text: Arc::clone(&document.text),
            dialect: dialect.to_owned(),
            version,
            revision: document.revision,
            inputs_epoch,
            file,
            input,
        })
    }

    /// Rejoin the retained document, Salsa handle, configuration epoch, and
    /// actual resolved-input Arc after Explorer's asynchronous CPU pass.
    async fn explorer_document_input_is_current(&self, expected: &ExplorerDocumentInput) -> bool {
        let _inputs = self.analyser_inputs_gate.read().await;
        if self.diag_inputs_epoch() != expected.inputs_epoch {
            return false;
        }
        let Some(file) = self.db_files.lock().await.get(&expected.uri).copied() else {
            return false;
        };
        if file != expected.file {
            return false;
        }
        let config = self.resolved_db_config(&expected.uri).await;
        let db = self.db.snapshot("explorer_document_input_is_current").await;
        if file.text(&*db) != &expected.source || file.dialect(&*db) != &expected.dialect {
            return false;
        }
        let Ok(input) = tcl_lsp_db::document_analysis_input(&*db, file, config) else {
            return false;
        };
        drop(db);
        if input.as_ref() != expected.input.as_ref()
            || self.diag_inputs_epoch() != expected.inputs_epoch
        {
            return false;
        }
        self.documents
            .lock("explorer_document_input_is_current")
            .await
            .get(&expected.uri)
            .is_some_and(|current| {
                current.revision == expected.revision
                    && Arc::ptr_eq(&current.text, &expected.document_text)
                    && !current.dialect_resolution_pending()
                    && current.matches_open_publication(
                        &expected.source,
                        &expected.dialect,
                        expected.version,
                    )
            })
    }

    /// Handle `tcl-lsp.compilerExplorer`: run the compiler pipeline
    /// (lexer → green tree → IR → CFG → SSA → codegen) on `args[0]` under the
    /// dialect in `args[1]` (default: the session dialect) and return the same
    /// serialised JSON the `tcl explore` CLI produces.  Empty/blank source
    /// returns `{error}` — the front-end's "nothing to compile" contract.
    pub(in super::super) async fn compiler_explorer_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let source = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if source.trim().is_empty() {
            return Ok(Some(serde_json::json!({ "error": "no source to compile" })));
        }
        let dialect = match args.get(1).and_then(serde_json::Value::as_str) {
            Some(d) => d.to_owned(),
            None => self.session_dialect().await,
        };
        let bound = match args.get(2) {
            Some(owner) => match self
                .capture_explorer_document_input(owner, &source, &dialect)
                .await
            {
                Some(input) => Some(input),
                None => {
                    return Ok(Some(serde_json::json!({
                        "error": "compiler explorer source document is stale or unavailable",
                    })));
                }
            },
            None => None,
        };
        // The pipeline is heavy pure-CPU work — run it off the LSP event loop.
        // A parser panic is contained and surfaced as an `{error}` object
        // rather than tearing down the worker.
        let input = bound.as_ref().map(|bound| Arc::clone(&bound.input));
        let value = crate::rt::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let result = match input.as_deref() {
                    Some(input) => {
                        tcl_explorer::run_pipeline_with_analysis_input(&source, &dialect, input)
                    }
                    None => tcl_explorer::run_pipeline(&source, &dialect),
                };
                tcl_explorer::serialise_result(&result)
            }))
        })
        .await;
        if let Some(bound) = &bound
            && !self.explorer_document_input_is_current(bound).await
        {
            return Ok(Some(serde_json::json!({
                "error": "compiler explorer source document changed during analysis",
            })));
        }
        match value {
            Ok(Ok(v)) => Ok(Some(v)),
            _ => Ok(Some(serde_json::json!({
                "error": "compiler explorer failed to analyse the source",
            }))),
        }
    }

    /// Handle `tcl-lsp.compilerExplorerView`: render one explorer view as text
    /// plus the source span each of its lines points at.
    ///
    /// `args` is `(source, dialect, viewId)`. The reply is
    /// `{text, lines: [null | {startLine, startColUtf16, endLine, endColUtf16,
    /// startOffset, endOffset}]}`, one `lines` entry per line of `text`.
    ///
    /// This exists so an editor host can show a projection in one of its own
    /// editor panes and still navigate from it: the pane holds `text`, and a
    /// caret on line *n* resolves through `lines[n]` back into the user's
    /// file. Rendering it here rather than in each host keeps one renderer —
    /// the same [`tcl_explorer::render`] the `tcl explore --text` CLI and the
    /// TUI use — and keeps the line map produced by the code that produced the
    /// lines, so the two cannot drift.
    pub(in super::super) async fn compiler_explorer_view_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let source = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if source.trim().is_empty() {
            return Ok(Some(serde_json::json!({ "error": "no source to compile" })));
        }
        let dialect = match args.get(1).and_then(serde_json::Value::as_str) {
            Some(d) => d.to_owned(),
            None => self.session_dialect().await,
        };
        let view = args
            .get(2)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("ir")
            .to_owned();
        let bound = match args.get(3) {
            Some(owner) => match self
                .capture_explorer_document_input(owner, &source, &dialect)
                .await
            {
                Some(input) => Some(input),
                None => {
                    return Ok(Some(serde_json::json!({
                        "error": "compiler explorer source document is stale or unavailable",
                    })));
                }
            },
            None => None,
        };
        // Same containment as `compiler_explorer_command`: heavy pure-CPU work
        // off the event loop, and a parser panic surfaced as `{error}` rather
        // than tearing down the worker.
        let input = bound.as_ref().map(|bound| Arc::clone(&bound.input));
        let value = crate::rt::spawn_blocking(move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let result = match input.as_deref() {
                    Some(input) => {
                        tcl_explorer::run_pipeline_with_analysis_input(&source, &dialect, input)
                    }
                    None => tcl_explorer::run_pipeline(&source, &dialect),
                };
                let data = tcl_explorer::serialise_result(&result);
                let rendered = tcl_explorer::render::render_view_mapped(&view, &data, false);
                serde_json::json!({
                    "view": view,
                    "text": rendered.text,
                    "lines": rendered.lines,
                })
            }))
        })
        .await;
        if let Some(bound) = &bound
            && !self.explorer_document_input_is_current(bound).await
        {
            return Ok(Some(serde_json::json!({
                "error": "compiler explorer source document changed during analysis",
            })));
        }
        match value {
            Ok(Ok(v)) => Ok(Some(v)),
            _ => Ok(Some(serde_json::json!({
                "error": "compiler explorer failed to render the view",
            }))),
        }
    }
}
