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

//! Dialect resolution for open and closed documents: overrides, folder defaults, language ids and in-source hints.

use super::Backend;
use super::configuration::{
    FolderConfig, normalize_config_payload, settings_environment_kind_enabled,
    settings_non_ascii_mode, workspace_trust_in,
};
use super::documents::{DocumentPathAdmission, DocumentSourceResult};
use super::workspace::uri_under_folder;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tcl_lsp_core::{bigip as core_bigip, config_ini};
use tokio::sync::Mutex;
use tower_lsp_server::ls_types::{InitializeParams, Range, Uri};

/// The iRules dialect key.  A BIG-IP config's `ltm rule { … }` bodies are iRules
/// code, so they are tokenised against this registry rather than the config's
/// own `f5-bigip` one.
pub(super) const IRULES_DIALECT: &str = "f5-irules";

/// The iApps dialect key — an APL presentation's embedded `[ … ]` Tcl.
pub(super) const IAPPS_DIALECT: &str = "f5-iapps";

/// The debounced, detached diagnostics worker for one document.
///
/// A free function rather than an inline `tokio::spawn` body so it can
/// **respawn itself for a peer document**: refreshing cross-file call-site
/// evidence here can invalidate another file's diagnostics, and that file's
/// own worker has to be (re)started to republish them.
/// See [`Backend::session_dialect`].
pub(super) async fn session_dialect_from(
    session_override: &Mutex<Option<String>>,
    default_dialect: &Mutex<String>,
) -> String {
    if let Some(over) = session_override.lock().await.clone() {
        return over;
    }
    default_dialect.lock().await.clone()
}

/// See [`Backend::folder_dialect_overrides`].
pub(super) async fn merged_folder_dialect_overrides(
    folder_dialects: &Mutex<Vec<(Uri, String)>>,
    folder_configs: &Mutex<Vec<(Uri, FolderConfig)>>,
) -> Vec<(Uri, String)> {
    let mut merged = folder_dialects.lock().await.clone();
    let from_config: Vec<(Uri, String)> = {
        let configs = folder_configs.lock().await;
        configs
            .iter()
            .filter_map(|(folder, fc)| fc.dialect.clone().map(|d| (folder.clone(), d)))
            .collect()
    };
    for (folder, dialect) in from_config {
        if merged.iter().any(|(u, _)| u.as_str() == folder.as_str()) {
            continue;
        }
        merged.push((folder, dialect));
    }
    merged
}

/// A known editor language ID after its ingress spelling has been resolved
/// through the one environment seam.
///
/// `tk` is an environment like any other, so the ingress has one arm — the
/// resolved environment's
/// [`unit_profile`](tcl_registry::model::DocumentEnvironment::unit_profile),
/// which is its catalogue profile for a catalogue environment and the typed
/// additive Tk profile for `tk`. The newtype keeps the language-id table from
/// leaking a raw dialect string into server routing.
#[derive(Clone, Copy)]
pub(super) struct LanguageDialect(&'static tcl_dialect::DialectProfile);

impl LanguageDialect {
    pub(super) fn name(self) -> &'static str {
        self.0.name
    }

    fn is_explicit_non_tcl(self) -> bool {
        !self.0.name.starts_with("tcl")
    }
}

impl Backend {
    /// Resolve one dirty open document against its **current** identity.
    ///
    /// Both the dialect resolver and live publication await other stores, so
    /// an edit or a second configuration change can overtake either phase. The
    /// per-document generation makes that visible: a mismatched snapshot is
    /// retried, and resolving generation N cannot clear generation N+1.
    pub(super) async fn resolve_dirty_open_document_dialect(
        &self,
        uri: &Uri,
        operation: &'static str,
    ) -> DocumentSourceResult<()> {
        loop {
            let admission = self.document_path_admission(uri).await?;
            let (language_id, old_dialect, text, revision, generation) = {
                let docs = self.documents.lock(operation).await;
                let Some(doc) = docs.get(uri) else {
                    return Ok(());
                };
                if !doc.dialect_resolution_pending() {
                    return Ok(());
                }
                (
                    doc.language_id.clone(),
                    doc.dialect.clone(),
                    doc.text.clone(),
                    doc.revision,
                    doc.dialect_resolution_generation,
                )
            };
            let new_dialect = self.dialect_for_open(uri, &language_id, &text).await;
            let changed = {
                let mut docs = self.documents.lock(operation).await;
                let Some(doc) = docs.get_mut(uri) else {
                    return Ok(());
                };
                if doc.revision != revision
                    || doc.text.as_ref() != text.as_ref()
                    || doc.dialect != old_dialect
                    || doc.dialect_resolution_generation != generation
                {
                    continue;
                }
                DocumentPathAdmission::with_current(
                    &[admission.as_ref()],
                    self.store.as_ref(),
                    || {
                        doc.resolved_dialect_generation = generation;
                        if new_dialect == old_dialect {
                            None
                        } else {
                            doc.dialect.clone_from(&new_dialect);
                            doc.bump_analysis_generation();
                            Some((doc.text.clone(), doc.revision))
                        }
                    },
                )?
            };
            let Some(changed) = changed else {
                return Ok(());
            };
            self.invalidate_live_publication(std::iter::once(uri));
            self.notify_environment_kind(&new_dialect);
            if self
                .commit_live_dialect(operation, uri, &changed.0, &new_dialect, changed.1)
                .await?
            {
                self.reschedule_diagnostics(uri.clone(), new_dialect).await;
                return Ok(());
            }
            // A newer edit that overtook publication inherited the resolved
            // dialect above. A newer configuration change advances the dirty
            // generation and this loop resolves it; a close exits next round.
        }
    }

    /// Re-resolve one open document's dialect after a change that can only
    /// affect it (a per-document override). Marking just this document keeps a
    /// pinned buffer from disturbing every other one, which is the whole point
    /// of the per-document tier.
    pub(super) async fn reresolve_open_document_dialect(
        &self,
        uri: &Uri,
    ) -> DocumentSourceResult<()> {
        let admission = self.document_path_admission(uri).await?;
        {
            let mut docs = self.documents.lock("reresolve_open_document_dialect").await;
            let Some(doc) = docs.get_mut(uri) else {
                return Ok(());
            };
            DocumentPathAdmission::with_current(
                &[admission.as_ref()],
                self.store.as_ref(),
                || {
                    doc.mark_dialect_resolution_dirty();
                },
            )?;
        }
        self.resolve_dirty_open_document_dialect(uri, "reresolve_open_document_dialect")
            .await?;

        Ok(())
    }

    /// Re-resolve every open document's dialect after a session/folder-default
    /// change. Mark all documents first so a concurrent edit carries the work
    /// forward; each resolver then retries until it consumes the exact current
    /// generation or the document closes.
    pub(super) async fn reresolve_open_document_dialects(&self) -> DocumentSourceResult<()> {
        let uris = {
            let mut docs = self
                .documents
                .lock("reresolve_open_document_dialects")
                .await;
            // Authenticate the complete write set before marking its first row.
            let admissions = docs
                .iter()
                .map(|(uri, doc)| {
                    let admission = match &doc.path_admission {
                        Some(retained) => retained.as_ref().clone(),
                        None => DocumentPathAdmission::capture(uri, self.store.as_ref())?,
                    };
                    admission.check_uri(uri)?;
                    Ok(admission)
                })
                .collect::<DocumentSourceResult<Vec<_>>>()?;
            let retained = admissions.iter().collect::<Vec<_>>();
            DocumentPathAdmission::with_current(&retained, self.store.as_ref(), || {
                for doc in docs.values_mut() {
                    doc.mark_dialect_resolution_dirty();
                }
            })?;
            docs.keys().cloned().collect::<Vec<Uri>>()
        };
        let _: () = for uri in uris {
            self.resolve_dirty_open_document_dialect(&uri, "reresolve_open_document_dialects")
                .await?;
        };
        Ok(())
    }

    /// Read per-folder dialect
    /// overrides from `params.initialization_options`.
    ///
    /// Expected shape:
    /// `{ "folderDialects": { "file:///path": "f5-irules",
    ///                        "file:///other": "tcl9.0" } }`.
    ///
    /// Unknown dialect names and unparseable folder URLs are
    /// dropped silently rather than failing the entire
    /// initialise — the server keeps starting up with whatever
    /// valid entries it could pull from the editor.
    ///
    /// `workspaceTrust: "trusted" | "untrusted"` is the editor's Workspace
    /// Trust state ([`workspace_trust_in`]). Read here, before `initialized`
    /// starts the first pack load, so an untrusted workspace's hook bodies
    /// never run even once; a client that sends nothing is trusted.
    pub(super) async fn apply_initialization_options(&self, params: &InitializeParams) {
        let Some(opts) = &params.initialization_options else {
            return;
        };
        if let Some(trust) = workspace_trust_in(opts) {
            let _inputs = self.analyser_inputs_gate.write().await;
            *self.workspace_trust.lock().await = trust;
        }
        // `folderDialects` map (per-folder dialect overrides).
        if let Some(entries) = opts
            .as_object()
            .and_then(|m| m.get("folderDialects"))
            .and_then(serde_json::Value::as_object)
        {
            let mut parsed: Vec<(Uri, String)> = Vec::new();
            for (folder_url, dialect_val) in entries {
                let Ok(url) = Uri::from_str(folder_url) else {
                    continue;
                };
                let Some(dialect) = dialect_val.as_str() else {
                    continue;
                };
                if !is_known_dialect_name(dialect) {
                    continue;
                }
                parsed.push((url, dialect.to_owned()));
            }
            *self.folder_dialects.lock().await = parsed;
        }
        // Diagnostic settings (`tclLsp.style.nonAscii` /
        // `tclLsp.diagnostics.<CODE>`), accepted in either the nested or
        // flat-dotted shape.
        if let Some(mode) = settings_non_ascii_mode(opts) {
            *self.non_ascii_mode.lock().await = mode;
        }
        // The same payload is the editor layer of the session's policy until
        // the first `workspace/configuration` pull replaces it, and the
        // analyser's skip is the production skip of the layers it merged into.
        let skip = {
            let mut layers = self.policy_layers.lock().await;
            layers.editor =
                config_ini::merge_settings(&layers.editor, &normalize_config_payload(opts));
            layers.production_skip()
        };
        *self.disabled_diagnostics.lock().await = skip;
        if let Some(flag) = settings_environment_kind_enabled(opts) {
            self.environment_notice.set_enabled(flag);
        }
    }

    /// Resolve the dialect string a freshly opened document should
    /// be tagged with.
    ///
    /// Resolution order:
    ///
    /// 0. A deliberate per-document override installed by
    ///    `tcl-lsp.setDocumentDialectOverride`
    ///    ([`crate::BackendState::document_dialect_overrides`]) — a host naming this exact
    ///    URI outranks every inference below, including the in-source
    ///    directive.
    /// 1. The LSP ``languageId`` field — when it names a known
    ///    dialect (``"tcl-irule"`` / ``"f5-irules"`` / ``"tcl90"`` /
    ///    ``"tcl9.0"`` / etc.), use it directly.
    /// 2. Detection over the document itself — for the bare ``"tcl"``
    ///    id every editor sends for a `.tcl` buffer, the shared
    ///    [`tcl_registry::dialects::detect_dialect`] (directive,
    ///    shebang, version guard, content signatures, then extension).
    /// 3. The per-folder override map ([`Self::folder_dialect_overrides`] —
    ///    `initializationOptions.folderDialects` merged over each folder's
    ///    pulled `tclLsp.dialect`) — when the document URI sits under one of
    ///    the configured folder URLs, use the deepest-matching folder's
    ///    dialect.
    /// 4. The session-wide ``default_dialect`` fallback.
    pub(super) async fn dialect_for_open(
        &self,
        uri: &Uri,
        language_id: &str,
        text: &str,
    ) -> String {
        let document_override = self.document_dialect_override(uri).await;
        let folder_dialects = self.folder_dialect_overrides().await;
        let default_dialect = self.session_dialect().await;
        Self::dialect_for_open_sync(
            uri,
            language_id,
            text,
            document_override.as_deref(),
            &folder_dialects,
            &default_dialect,
        )
    }

    /// The deliberate per-document dialect in force for `uri`, if a host
    /// installed one with `tcl-lsp.setDocumentDialectOverride`.
    async fn document_dialect_override(&self, uri: &Uri) -> Option<String> {
        self.document_dialect_overrides
            .lock()
            .await
            .get(uri.as_str())
            .cloned()
    }

    /// One snapshot of every per-document override, for a caller resolving
    /// many URIs off-lock (the watched-files batch reindex).
    pub(super) async fn document_dialect_override_snapshot(&self) -> Arc<HashMap<String, String>> {
        Arc::new(self.document_dialect_overrides.lock().await.clone())
    }

    /// The session-wide fallback dialect: the deliberate
    /// [`crate::BackendState::session_dialect_override`] when one is in force, otherwise
    /// the configured [`crate::BackendState::default_dialect`].
    ///
    /// The one read point for that tier, so a configuration pull rewriting
    /// `default_dialect` cannot silently revert an override.
    pub(super) async fn session_dialect(&self) -> String {
        session_dialect_from(&self.session_dialect_override, &self.default_dialect).await
    }

    /// Every per-folder dialect override in effect, as the
    /// `(folder-uri, dialect)` list [`folder_dialect_for`] resolves a document
    /// against.
    ///
    /// Two sources can set one and they are merged here so every dialect
    /// resolution — open document, closed on-disk file, batched watched-file
    /// reindex, the `getEffectiveConfig` command — sees the same mapping:
    ///
    /// 1. `initializationOptions.folderDialects` (`folder_dialects`) — a
    ///    host-supplied mapping fixed at `initialize` and never re-pulled.  It
    ///    wins for the folder URIs it names: a host that states the mapping
    ///    outright is making a deliberate choice the editor's settings files
    ///    should not silently override.
    /// 2. The folder's pulled `tclLsp.dialect` (`folder_configs`) — what VS
    ///    Code resolves for a `.vscode/settings.json` at the folder scope.
    ///    Applies to every folder the map above does not name.
    ///
    /// Across *different* folders precedence is unchanged: `folder_dialect_for`
    /// still picks the longest matching folder prefix, so a nested folder
    /// shadows its parent whichever source each of them came from.
    pub(super) async fn folder_dialect_overrides(&self) -> Vec<(Uri, String)> {
        merged_folder_dialect_overrides(&self.folder_dialects, &self.folder_configs).await
    }

    /// Pure core of [`Self::dialect_for_open`] (and so of
    /// [`Self::dialect_for_closed`]): every input the resolution needs reduced
    /// to plain arguments, so a caller that already holds a `folder_dialects` /
    /// `default_dialect` snapshot — a batched watched-file reindex resolving
    /// many URIs at once — can resolve a dialect without a per-file
    /// `&self` lock round trip. `dialect_for_open` is the only other caller;
    /// keeping the logic in one place is what makes the two paths agree.
    ///
    /// Resolution order:
    ///
    /// 0. A deliberate per-document override installed by
    ///    `tcl-lsp.setDocumentDialectOverride`
    ///    ([`crate::BackendState::document_dialect_overrides`]) — a host naming this exact
    ///    URI outranks every inference below, including the in-source
    ///    directive.
    /// 1. The LSP ``languageId`` field — when it names a known
    ///    dialect (``"tcl-irule"`` / ``"f5-irules"`` / ``"tcl90"`` /
    ///    ``"tcl9.0"`` / etc.), use it directly.
    /// 2. Detection over the document itself — for the bare ``"tcl"``
    ///    id every editor sends for a `.tcl` buffer, the shared
    ///    [`tcl_registry::dialects::detect_dialect`] (directive,
    ///    shebang, version guard, content signatures, then extension).
    /// 3. The per-folder override map ([`Self::folder_dialect_overrides`] —
    ///    `initializationOptions.folderDialects` merged over each folder's
    ///    pulled `tclLsp.dialect`) — when the document URI sits under one of
    ///    the configured folder URLs, use the deepest-matching folder's
    ///    dialect.
    /// 4. The session-wide ``default_dialect`` fallback.
    pub(super) fn dialect_for_open_sync(
        uri: &Uri,
        language_id: &str,
        text: &str,
        document_override: Option<&str>,
        folder_dialects: &[(Uri, String)],
        default_dialect: &str,
    ) -> String {
        // A host that named this URI outright has already decided; nothing
        // inferred from the id, the basename or the text may overrule it.
        if let Some(dialect) = document_override {
            return dialect.to_owned();
        }
        // An explicit BIG-IP language id (`tcl-bigip`, advertised by the VS
        // Code extension, or the canonical `f5-bigip`) selects the BIG-IP
        // config dialect even when the basename is not a canonical
        // `bigip*.conf` name — e.g. a user who manually picks the BIG-IP
        // language mode on a differently-named config file. Mirrors the
        // BIG-IP routing the `is_bigip_conf_uri` basename branch below feeds.
        if matches!(language_id, "tcl-bigip" | "f5-bigip") {
            return "f5-bigip".to_owned();
        }
        let lang_dialect = Self::dialect_from_language_id(language_id);
        // A canonical BIG-IP basename (``bigip.conf``, ``bigip_base.conf``,
        // …) routes to ``f5-bigip`` ahead of a *generic* Tcl ``languageId``:
        // only an explicit
        // non-Tcl dialect id (``f5-irules`` / ``f5-iapps`` / ``expect`` /
        // EDA / ``tk``) wins over the basename. This is what lets the test
        // harness open ``bigip.conf`` with ``languageId: "tcl"`` and still
        // get the BIG-IP path (parse / outline + general-Tcl-diagnostic
        // suppression).
        let explicit_non_tcl = lang_dialect.is_some_and(LanguageDialect::is_explicit_non_tcl);
        if !explicit_non_tcl && core_bigip::is_bigip_conf_uri(uri.as_str()) {
            return "f5-bigip".to_owned();
        }
        // In-source dialect hints — a `# tcl-dialect:` directive, a
        // `#!…tclshX.Y` shebang, or a `package require Tcl X.Y` line — are
        // authoritative for a *generically* opened Tcl buffer.  VS Code always
        // sends `languageId: "tcl"` for `.tcl` files (both `"tcl"` and the
        // explicit `"tcl8.6"` id map to `tcl8.6` above), so an in-file
        // directive is the only way a user can pin 8.4 / 8.5 / 9.x, and it must
        // win over the generic `tcl8.6` default — otherwise completion offers
        // 8.6+-only commands like `try` in an 8.4/8.5 file.  An explicit
        // versioned or non-Tcl `languageId` still takes precedence (it is a
        // deliberate editor choice), so only the bare `"tcl"` id defers here.
        // Delegates to the shared `detect_dialect` (directive > shebang >
        // `package require Tcl` > content signatures > file extension), the
        // same detector `tcl diag` resolves a document with — so the editor
        // and the CLI report the same set for the same file. The
        // content-signature tier is what recognises an iRule saved as
        // `.tcl` (or opened by an editor with no iRules language mode) from
        // its `when EVENT {` handlers; the extension tier never fires for a
        // plain `.tcl` name, so the folder override and session default below
        // still decide an ordinary Tcl buffer. An empty `default`
        // is the "nothing detected" sentinel: it keeps that deferral intact.
        if language_id == "tcl" {
            // The directive / shebang / version-guard tiers are line-oriented,
            // so an old-Mac buffer must be normalised first or the whole file
            // reads as line 0 and a `# tcl-lsp: dialect=…` header on line 2
            // never registers.  A no-op for LF / CRLF text.
            let text = tcl_lexer::normalise_lone_cr(text);
            let detected = tcl_registry::dialects::detect_dialect(&text, Some(uri.as_str()), "");
            if !detected.is_empty() {
                return detected.to_owned();
            }
        }
        // A *versioned* or non-Tcl language id (`tcl84`/`tcl8.4`,
        // `tcl90`/`tcl9.0`,
        // `f5-irules`, …) is a deliberate, specific choice and wins over the
        // per-folder and session (config-file) dialect below. The bare `"tcl"`
        // id is different: editors send it for *every* `.tcl` file, so it names
        // no specific version and must NOT be treated as authoritative — it has
        // to defer to the folder override and the session `default_dialect`
        // (which the `dialect =` key of `config.ini` / `.tcl-lsp.ini` sets).
        // Without this deferral a config-file dialect would never take effect
        // for a normally-opened `.tcl` buffer; the session default is itself
        // `tcl8.6` unless configured, so an unconfigured file still resolves
        // to `tcl8.6`.
        if language_id != "tcl"
            && let Some(d) = lang_dialect
        {
            return d.name().to_owned();
        }
        if let Some(d) = folder_dialect_for(uri, folder_dialects) {
            return d;
        }
        default_dialect.to_owned()
    }

    /// Whether `dialect` denotes an F5 BIG-IP config document — the
    /// signal every BIG-IP-specific branch (analysis suppression,
    /// config outline) keys on.
    pub(super) fn is_bigip_dialect(dialect: &str) -> bool {
        // The resolved environment's authoring point, not a second name
        // validator: sweep-pinned to the profile's own point for every
        // catalogue environment, and the lenient sink (every unknown name)
        // carries no vendor package, exactly as `find` answering `None` did.
        tcl_lsp_core::document_context_for_dialect(dialect)
            .authoring_query()
            .carries("bigip")
    }

    /// Offer the tool-environment explanation for a document that resolves to
    /// `dialect`, if that is a tool environment (see [`environment_notice`]).
    ///
    /// Only decides whether there is anything to offer, then hands the rest to
    /// its own task: the message awaits the client, and neither the document
    /// map nor an edit turn may be held across that.
    pub(super) fn notify_environment_kind(&self, dialect: &str) {
        let Some(resolved) = tcl_registry::model::resolve_known_environment(dialect) else {
            return;
        };
        if !self.environment_notice.wants(&resolved.definition) {
            return;
        }
        crate::rt::spawn(
            Arc::clone(&self.environment_notice)
                .offer(self.client.clone(), Arc::clone(&resolved.definition)),
        );
    }

    /// Look up the per-folder dialect override for `uri`,
    /// preferring the deepest (longest-prefix) match so nested
    /// folders shadow their parents.
    #[cfg(test)]
    pub(super) async fn resolve_folder_dialect(&self, uri: &Uri) -> Option<String> {
        folder_dialect_for(uri, &self.folder_dialect_overrides().await)
    }

    /// Map an LSP ``languageId`` string to a dialect name accepted
    /// by the dialect catalogue / the providers' ``dialect`` arg.
    ///
    /// Recognises the editor-extension language ids
    /// (``tcl-irule`` → ``f5-irules``, etc.) plus the canonical
    /// dialect names used by the MCP bridge / direct callers
    /// (``f5-irules``, ``tcl9.0``, …).  Returns ``None`` when the
    /// language id does not name a known dialect — the caller falls
    /// back to the session default.
    pub(super) fn dialect_from_language_id(language_id: &str) -> Option<LanguageDialect> {
        // Every editor sends the bare `tcl` id for a plain `.tcl` buffer; it
        // names no version, and the default environment is the fallback the
        // rest of the resolution chain is written against. (The `tcl`
        // *environment* is the lenient whole-ladder one, which is a different
        // answer.) It is the detection trigger, not an environment selection.
        let language_id = if language_id == "tcl" {
            tcl_dialect::model::DEFAULT_ENVIRONMENT_ID
        } else {
            language_id
        };
        // This is an editor-ID ingress boundary, and it goes through the one
        // language-id resolver: a canonical id, a contributed identity
        // (`tcl90`, `tcl-synopsys`; the version-pinned ones are undotted
        // because a language id containing a `.` cannot carry a
        // `configurationDefaults` override) or a selecting spelling
        // (`tcl-apl`) answers with its environment, so a new environment is
        // resolvable here the day it is declared. An alias is not a language
        // id, and an id that names no environment answers `None` so the
        // caller falls back to the session default.
        tcl_registry::model::resolve_language_id(language_id)
            .map(|environment| LanguageDialect(environment.unit_profile()))
    }

    /// Resolve the dialect for a **closed** on-disk file the way
    /// [`Self::dialect_for_open`] resolves a freshly opened one, but without an
    /// editor `language_id`: synthesise one from the file extension
    /// ([`tcl_registry::dialect_from_extension`], falling back to the bare
    /// `"tcl"` id that triggers in-source directive detection) so the basename
    /// (BIG-IP), `# tcl-dialect:` / shebang / `package require Tcl` hint,
    /// per-folder override, and session default all still apply.
    pub(super) async fn dialect_for_closed(&self, uri: &Uri, text: &str) -> String {
        let language_id = tcl_registry::dialect_from_extension(uri.as_str()).unwrap_or("tcl");
        self.dialect_for_open(uri, language_id, text).await
    }

    /// Pure core of [`Self::dialect_for_closed`] — see
    /// [`Self::dialect_for_open_sync`]. Lets the watched-files batch reindex
    /// resolve every changed file's dialect from one snapshot instead
    /// of a `&self` lock round trip per file.
    pub(super) fn dialect_for_closed_sync(
        uri: &Uri,
        text: &str,
        document_override: Option<&str>,
        folder_dialects: &[(Uri, String)],
        default_dialect: &str,
    ) -> String {
        let language_id = tcl_registry::dialect_from_extension(uri.as_str()).unwrap_or("tcl");
        Self::dialect_for_open_sync(
            uri,
            language_id,
            text,
            document_override,
            folder_dialects,
            default_dialect,
        )
    }
}

/// The session dialect before any `tclLsp.dialect` is configured, and the one a
/// configured value that names no environment falls back to.
pub(super) const DEFAULT_SESSION_DIALECT: &str = tcl_dialect::model::DEFAULT_ENVIRONMENT_ID;

/// The canonical ids of the selectable environments, comma-separated, read from
/// the live registry: what a message that rejects a dialect spelling offers
/// instead.
pub(super) fn selectable_dialect_names() -> String {
    tcl_registry::model::selectable_environments()
        .iter()
        .map(|environment| environment.id.as_str().to_owned())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The rejection message for a dialect-setting command, naming every selectable
/// dialect so the caller can correct the spelling from the error alone rather
/// than having to ask for the list separately.
pub(super) fn unknown_dialect_error(dialect: &str) -> String {
    // The accepted set is `Environment::resolve`'s — canonical ids, aliases
    // and contributed editor identities — which is wider than the canonical
    // list quoted here. The list stays canonical deliberately: it is a
    // "correct your spelling to one of these" message, not the acceptance set.
    format!(
        "unknown dialect: {dialect} (valid surface: {})",
        selectable_dialect_names()
    )
}

/// Leading lines within which a directive/shebang dialect hint can appear —
/// generous margin over [`tcl_registry::dialects::DIALECT_DIRECTIVE_SCAN_LINES`]
/// (5 lines) and the shebang's line 0, so a short header comment ahead of one
/// still counts as "near the top" for [`change_could_affect_dialect_hint`].
const DIALECT_HINT_LINE_MARGIN: u32 = 20;

/// Whether an edit could change what [`Backend::dialect_for_open`]'s in-source
/// detection ([`tcl_registry::dialects::detect_dialect`]) returns for this
/// document, so `did_change` can skip the whole-document clone + rescan it
/// would otherwise pay on every keystroke of a `languageId: "tcl"` buffer.
///
/// A conservative superset, not an exact replay of the detector: `true` when
/// the edit touches the leading [`DIALECT_HINT_LINE_MARGIN`] lines (covers the
/// directive/shebang tiers with margin), or when the **whole lines** the edit
/// touches — before *and* after the splice — contain one of the substrings
/// [`tcl_registry::dialects::dialect_hint_markers`] lists (covers the
/// content-signature / version-guard tiers, wherever in the document they
/// land). A `None` range (a full-document replacement) always returns `true`
/// — there is no local edit to reason about.
///
/// Widening to line boundaries is what makes the marker test sound.
/// [`tcl_registry::dialects::detect_dialect`] scans the *whole* source for
/// those tiers (head first, then the full text on a miss), so a marker at line
/// 500 counts — and a one-character edit *inside* such a line changes what the
/// detector returns while the changed slice itself holds no marker at all.
/// Editing the `6` of `package require Tcl 8.6` is the canonical case: the
/// removed and inserted slices are both a single digit. Reading the touched
/// lines instead costs those lines, not the document, and the persisted
/// [`tcl_lexer::LineIndex`] gives their bounds without a scan.
pub(super) fn change_could_affect_dialect_hint(
    text: &str,
    range: Option<Range>,
    new_text: &str,
    index: &tcl_lexer::LineIndex,
) -> bool {
    let Some(range) = range else {
        return true;
    };
    if range.start.line < DIALECT_HINT_LINE_MARGIN {
        return true;
    }
    let len = text.len();
    let offset = |line: u32, character: u32| {
        index.offset_at_utf16(line, tcl_lexer::Utf16Col::new(character), text) as usize
    };
    let (first_line, last_line) = (
        range.start.line.min(range.end.line),
        range.start.line.max(range.end.line),
    );
    // The touched lines in full, in the pre-edit text: `[line_start(first) ..
    // line_start(last + 1))`, clamped for the last line of the document.
    let line_from = offset(first_line, 0).min(len);
    let line_to = if (last_line as usize) + 1 < index.line_count() {
        offset(last_line + 1, 0).min(len)
    } else {
        len
    };
    let touched = &text[line_from..line_to.max(line_from)];
    if contains_dialect_hint_marker(touched) {
        return true;
    }
    // The same lines as the splice leaves them: everything before the edit on
    // the first touched line, the inserted text, everything after the edit on
    // the last one. Built rather than tested piecewise so a marker straddling
    // the splice point is still seen.
    let a = offset(range.start.line, range.start.character).min(len);
    let b = offset(range.end.line, range.end.character).min(len);
    let (cut_from, cut_to) = (a.min(b), a.max(b));
    let mut spliced =
        String::with_capacity((cut_from - line_from) + new_text.len() + (line_to - cut_to));
    spliced.push_str(&text[line_from..cut_from]);
    spliced.push_str(new_text);
    spliced.push_str(&text[cut_to..line_to.max(cut_to)]);
    contains_dialect_hint_marker(&spliced)
}

/// Whether `s` contains any of [`tcl_registry::dialects::dialect_hint_markers`].
///
/// Case-insensitive to match the detector: the `tcl-dialect:` directive key is
/// compared with `eq_ignore_ascii_case` and the content-signature tiers
/// lowercase their words, so `# TCL-DIALECT: irules` is live to
/// [`tcl_registry::dialects::detect_dialect`] and must not slip past this
/// filter. The haystack is only an edit's touched lines, so the lowercase
/// copy is a few bytes, not the document.
fn contains_dialect_hint_marker(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    tcl_registry::dialects::dialect_hint_markers().any(|m| lower.contains(m))
}

/// Whether `name` is a dialect the server accepts wherever a dialect *name* is
/// configured — `initializationOptions.folderDialects`, the per-folder
/// `tclLsp.dialect` pulled by `workspace/configuration`, and the
/// `setDialect` / `setSessionDialectOverride` commands.
///
/// One `Environment::resolve` rather than a per-call-site validator, so every
/// configuration path accepts exactly the
/// declared names: canonical environment ids, their aliases, and the
/// contributed editor identities. Whether a name is accepted therefore does
/// not depend on which table it happens to sit in, and an unknown name is
/// rejected.
pub(super) fn is_known_dialect_name(name: &str) -> bool {
    tcl_registry::model::is_known_environment_name(name)
}

/// Resolve the per-folder dialect override for the given URI. The
/// longest matching folder prefix wins so a nested folder mapping
/// shadows its parent. Returns `None` when no folder covers the
/// URI, in which case the caller falls back to the session
/// default.
pub(super) fn folder_dialect_for(uri: &Uri, folders: &[(Uri, String)]) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    let target = uri.as_str();
    for (folder, dialect) in folders {
        if !uri_under_folder(target, folder.as_str()) {
            continue;
        }
        let len = folder.as_str().len();
        let take = match best {
            Some((existing, _)) => len > existing,
            None => true,
        };
        if take {
            best = Some((len, dialect.as_str()));
        }
    }
    best.map(|(_, d)| d.to_owned())
}
