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

//! Setting and listing dialects.

use super::{Backend, FromStr, Uri, is_known_dialect_name, jsonrpc, unknown_dialect_error};

impl Backend {
    /// Handle `tcl-lsp.setDialect`: switch the session default dialect and
    /// re-resolve every open document under it (so buffers that fell back to
    /// the default re-analyse immediately).  Returns `{success, dialect}`, or
    /// `{success: false, error}` for an unknown dialect.  Drives the `VS Code`
    /// `tclLsp.selectDialect` command.
    pub(in super::super) async fn set_dialect_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(dialect) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(Some(serde_json::json!({
                "success": false,
                "error": "setDialect requires a dialect-name argument",
            })));
        };
        if !is_known_dialect_name(dialect) {
            return Ok(Some(serde_json::json!({
                "success": false,
                "error": unknown_dialect_error(dialect),
            })));
        }
        {
            let _analyser_inputs_guard = self.analyser_inputs_gate.write().await;
            let mut default_dialect = self.default_dialect.lock().await;
            if *default_dialect != dialect {
                dialect.clone_into(&mut default_dialect);
                self.invalidate_diag_inputs();
            }
            *self.default_dialect_explicit.lock().await = true;
        }
        self.reresolve_open_document_dialects().await?;
        Ok(Some(
            serde_json::json!({ "success": true, "dialect": dialect }),
        ))
    }

    /// Handle `tcl-lsp.setSessionDialectOverride`: install (or, with a `null` /
    /// absent argument, clear) the deliberate session dialect override, then
    /// re-resolve every open document under it.
    ///
    /// Distinct from `tcl-lsp.setDialect`, which rewrites the *configured*
    /// session default: an override lives in its own slot
    /// ([`crate::BackendState::session_dialect_override`]) that no `workspace/configuration`
    /// pull touches, so it lasts until the caller clears it rather than until
    /// the next time anything happens to settings.  That is the
    /// contract the chat commands need — run this buffer under `f5-irules`,
    /// then put it back — and the reason they must not push a
    /// `didChangeConfiguration` to get it.
    ///
    /// Returns `{success, dialect}` (`dialect: null` when cleared), or
    /// `{success: false, error}` for an unknown dialect.
    pub(in super::super) async fn set_session_dialect_override_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let requested = args.first().and_then(serde_json::Value::as_str);
        if let Some(dialect) = requested
            && !is_known_dialect_name(dialect)
        {
            return Ok(Some(serde_json::json!({
                "success": false,
                "error": unknown_dialect_error(dialect),
            })));
        }
        {
            let _analyser_inputs_guard = self.analyser_inputs_gate.write().await;
            let mut override_dialect = self.session_dialect_override.lock().await;
            let requested = requested.map(str::to_owned);
            if *override_dialect != requested {
                *override_dialect = requested;
                self.invalidate_diag_inputs();
            }
        }
        self.reresolve_open_document_dialects().await?;
        Ok(Some(
            serde_json::json!({ "success": true, "dialect": requested }),
        ))
    }

    /// Handle `tcl-lsp.setDocumentDialectOverride`: pin one document to a
    /// dialect, or (with a `null` / absent second argument) release it.
    ///
    /// The per-document counterpart of
    /// [`Self::set_session_dialect_override_command`], and the seam a host
    /// needs when *one* buffer must differ from the session. Both existing
    /// dialect commands are session-global, so a host using either to re-tag a
    /// single buffer re-tags every other open buffer with it. The Spec
    /// Studio's dialect selector is one caller: its sample surface is always
    /// materialised as `test.tcl`, so without this the server resolves it as
    /// generic Tcl and a pack whose commands only exist in, say, `f5-irules`
    /// shows no highlighting, completion or hover in the very buffer the studio
    /// exists to give feedback on.
    ///
    /// Arguments are `[uri, dialect]`; `dialect` absent or `null` clears.
    /// Returns `{success, uri, dialect}` (`dialect: null` when cleared), or
    /// `{success: false, error}` for a missing URI or an unknown dialect.
    pub(in super::super) async fn set_document_dialect_override_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_arg) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(Some(serde_json::json!({
                "success": false,
                "error": "tcl-lsp.setDocumentDialectOverride needs a document URI as its first argument",
            })));
        };
        let Ok(uri) = Uri::from_str(uri_arg) else {
            return Ok(Some(serde_json::json!({
                "success": false,
                "error": format!("not a valid document URI: {uri_arg}"),
            })));
        };
        let requested = args.get(1).and_then(serde_json::Value::as_str);
        if let Some(dialect) = requested
            && !is_known_dialect_name(dialect)
        {
            return Ok(Some(serde_json::json!({
                "success": false,
                "error": unknown_dialect_error(dialect),
            })));
        }
        {
            let _analyser_inputs_guard = self.analyser_inputs_gate.write().await;
            let mut overrides = self.document_dialect_overrides.lock().await;
            let current = overrides.get(uri.as_str()).map(String::as_str);
            let changed = match requested {
                Some(dialect) if current != Some(dialect) => {
                    overrides.insert(uri.as_str().to_owned(), dialect.to_owned());
                    true
                }
                Some(_) => false,
                None => overrides.remove(uri.as_str()).is_some(),
            };
            if changed {
                self.invalidate_diag_inputs();
            }
        }
        self.reresolve_open_document_dialect(&uri).await?;
        Ok(Some(serde_json::json!({
            "success": true,
            "uri": uri.as_str(),
            "dialect": requested,
        })))
    }

    /// Handle `tcl-lsp.listDialects`: every selectable environment as
    /// presentation data, in selectable order (languages first, then Tcl
    /// releases with packages, canonical id ascending within each). Each entry
    /// carries the canonical `name` (the spelling `tcl-lsp.setDialect` and
    /// `tclLsp.dialect` take), the full and compact labels, the `kind`
    /// (`language` or `packages`) a picker groups by, the derived one-line
    /// `description`, the `aliases` a directive also accepts, the dedicated
    /// editor language id (`null` where the environment has none) and the file
    /// `extensions` it owns, each with its human-facing name.
    ///
    /// Read from the live registry, so an environment a pack declares appears
    /// without a rebuild. The `VS Code` extension also gets the compiled list
    /// projected into its manifest at build time (`cargo xtask
    /// gen-editor-dialects`); every other editor asks for it here, so a
    /// dialect picker or status bar never has to hardcode one.
    pub(in super::super) fn list_dialects_command() -> serde_json::Value {
        serde_json::Value::Array(
            tcl_registry::model::selectable_environments()
                .iter()
                .map(|environment| {
                    serde_json::json!({
                        "name": environment.id.as_str(),
                        "display_name": environment.display_name.as_ref(),
                        "short_name": environment.short_name.as_ref(),
                        "kind": environment.kind.word(),
                        "description": environment.description(),
                        "aliases": environment
                            .aliases
                            .iter()
                            .map(AsRef::as_ref)
                            .collect::<Vec<&str>>(),
                        "editor_language_id": environment
                            .editor_identity
                            .map(tcl_dialect::model::EditorLanguageIdentityId::as_str),
                        "extensions": environment
                            .server_detection
                            .file_extensions
                            .iter()
                            .map(|claim| serde_json::json!({
                                "extension": claim.extension.as_ref(),
                                "display_name": claim.display_name.as_ref(),
                            }))
                            .collect::<Vec<serde_json::Value>>(),
                    })
                })
                .collect::<Vec<serde_json::Value>>(),
        )
    }
}
