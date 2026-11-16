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

//! The effective configuration and its INI export.

use std::str::FromStr;

use tcl_compiler::analyser::NonAsciiMode;
use tcl_lsp_core::formatting as core_formatting;
use tower_lsp_server::jsonrpc;
use tower_lsp_server::ls_types::Uri;

use crate::Backend;
use crate::configuration::deepest_folder_containing;
use crate::dialects::folder_dialect_for;

impl Backend {
    /// `tclLsp.formatting.docstringStyle` (#1314) as `getEffectiveConfig`
    /// reports it — the same per-URI fold `resolved_docstring_style` performs,
    /// except that a folder-only query (no readable document) has no `Uri` to
    /// resolve *through*, so it reads the process-global settings object
    /// directly, matching the `dialect` fallback. This is also the settle
    /// signal a test polls after
    /// `Lsp::with_config({"formatting": {"docstringStyle": …}})`.
    async fn reported_docstring_style(&self, uri: Option<&Uri>) -> &'static str {
        let style = match uri {
            Some(uri) => self.resolved_docstring_style(uri).await,
            None => self
                .formatting_settings
                .lock()
                .await
                .get("docstringStyle")
                .and_then(serde_json::Value::as_str)
                .map_or(core_formatting::DocstringStyle::None, |s| {
                    core_formatting::DocstringStyle::parse(s)
                }),
        };
        match style {
            core_formatting::DocstringStyle::Preceding => "preceding",
            core_formatting::DocstringStyle::Body => "body",
            core_formatting::DocstringStyle::None => "none",
        }
    }

    /// Handle `tcl-lsp.getEffectiveConfig`: the resolved per-document config —
    /// active dialect, the resolved `features` toggle map, the optimiser
    /// switch, line length, and analyser settings.  Tests poll this command
    /// after a `tclLsp.features.X = false` config change to confirm the server
    /// has applied the toggle without sleeping on wall-clock time.
    ///
    /// The argument may name a *folder* rather than a document — the multi-root
    /// suite polls each workspace folder's URI to watch the per-folder pull
    /// settle — so every resolved value goes through the same folder chain a
    /// document under that folder would use.
    pub(in super::super) async fn get_effective_config_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let uri_str = args
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let parsed_uri = Uri::from_str(uri_str).ok();
        let folder_overrides = self.folder_dialect_overrides().await;
        let known_folders = self.workspace_folder_urls().await;
        let folder_uri = parsed_uri
            .as_ref()
            .and_then(|uri| deepest_folder_containing(uri, &known_folders));
        // Resolve the dialect via the open document when the URI names one.  A
        // folder (or any other non-document) URI is not readable as a document,
        // so it resolves through the per-folder override chain before falling
        // back to the session default — never `null`, so a polling client
        // always sees a concrete value.
        let dialect = match &parsed_uri {
            Some(uri) => match self.read_document(uri).await? {
                Some(doc) => doc.dialect,
                None => match folder_dialect_for(uri, &folder_overrides) {
                    Some(d) => d,
                    None => self.session_dialect().await,
                },
            },
            None => self.session_dialect().await,
        };
        // The resolved environment's labels, so a status bar or picker can
        // render the dialect without keeping its own name table: every
        // environment carries them, `tk`, `jim` and the lenient `tcl`
        // included. `null` only for a name that names no environment (an
        // unrecognised configured value).
        let resolved = tcl_registry::model::resolve_known_environment(&dialect);
        let environment = resolved.as_ref().map(|resolved| &*resolved.definition);
        // Whether a `tclLsp.dialect` was actually configured for this URI — by
        // the folder it sits under, or session-wide — as opposed to the
        // built-in fallback that a never-configured session reports.  A
        // deliberate session override counts: it is exactly "someone chose this
        // dialect", and a caller tracing a surprising dialect must be able to
        // see it.
        let iruleslx_plugins = self.reported_ilx_plugin_roots().await;
        let session_override = self.session_dialect_override.lock().await.clone();
        let dialect_explicitly_set = parsed_uri
            .as_ref()
            .is_some_and(|uri| folder_dialect_for(uri, &folder_overrides).is_some())
            || session_override.is_some()
            || *self.default_dialect_explicit.lock().await;
        let extra_commands = match &parsed_uri {
            Some(uri) => self.resolved_extra_commands(uri).await,
            None => self.extra_commands.lock().await.clone(),
        };
        let disabled_signatures = self.signature_disabled_for(parsed_uri.as_ref()).await;
        // Resolved through the same folder chain every provider gate uses, so a
        // client polling this command observes the fact the provider will act
        // on rather than the process-global one.  If the two differed, a
        // folder-scoped toggle would be invisible here and a client's toggle
        // barrier would wait on the wrong fact.
        let features = self
            .resolved_feature_toggles(parsed_uri.as_ref())
            .await
            .resolved_map();
        let optimiser_enabled = *self.optimiser_enabled.lock().await;
        // The optimiser *profile* and the editor `libraryPaths` are as much a
        // part of "what config is in effect" as the master switch, and a caller
        // tracing a surprising diagnostic needs them.  They are also the settle
        // signal the e2e harness polls to know a pulled config has been
        // *applied*: `initialized` pulls the config concurrently with any
        // `didOpen` the client already queued, so without an observable
        // post-apply signal a test cannot know whether its document was
        // analysed before or after its config landed.
        let optimiser_profile = self.optimiser_profile.lock().await.name();
        let library_paths = self.editor_library_paths.lock().await.clone();
        // Session-wide, like `library_paths`: one scan serves every folder, so
        // there is no per-URI chain to resolve it through.  Reported so a user
        // tracing "why is this file not in the index?" can see the budget the
        // scan actually ran under (issue #2021) — and so a test can wait for a
        // pushed cap to have landed before asserting on the rescan.
        let workspace_scan_max_files = *self.workspace_scan_max_files.lock().await;
        let (spec_packs, spec_packs_loaded, pack_file_extensions) = self.spec_pack_report().await;
        let spec_pack_publication = self.spec_pack_configuration_report().await;
        let line_length = *self.line_length.lock().await;
        let docstring_style_str = self.reported_docstring_style(parsed_uri.as_ref()).await;
        // Report the *per-folder* analyser settings (the same resolvers the
        // feature/diagnostics paths use), not the process-global ones: in a
        // multi-root workspace a folder may override the disabled-codes set /
        // non-ASCII mode, and this "trace where a setting comes from" tool must
        // reflect what actually applies to `uri_str`.  A URI naming no
        // overriding folder (or a single-root workspace) falls back to the
        // global `db_config`, matching the previous behaviour exactly.  The
        // disabled set is the analyser's skip plus the fact codes the layers
        // turn off ([`PolicyLayers::reported_disabled`]).
        let (disabled_sorted, mode) = if let Some(uri) = &parsed_uri {
            let layers = self.resolved_policy_layers(uri).await;
            let config = self.resolved_db_config(uri).await;
            let db = self.db.lock().await;
            (
                layers.reported_disabled(config.disabled_diagnostics(&*db)),
                config.non_ascii_mode(&*db),
            )
        } else {
            let layers = self.policy_layers.lock().await.clone();
            let skip = self.disabled_diagnostics.lock().await.clone();
            (
                layers.reported_disabled(&skip),
                *self.non_ascii_mode.lock().await,
            )
        };
        Ok(Some(serde_json::json!({
            "uri": uri_str,
            "folder_uri": folder_uri,
            "dialect": dialect,
            "dialect_id": environment.map(|environment| environment.id.as_str()),
            "dialect_display_name": environment.map(|environment| environment.display_name.as_ref()),
            "dialect_short_name": environment.map(|environment| environment.short_name.as_ref()),
            // `language` or `packages`: the group a picker files the dialect
            // under.
            "dialect_kind": environment.map(|environment| environment.kind.word()),
            "dialect_description": environment.map(tcl_dialect::model::EnvironmentDefinition::description),
            // Where the environment came from: `built-in`, `bundled-pack`,
            // `user-pack` or `workspace-pack`.
            "dialect_provenance": environment.map(|environment| environment.provenance.word()),
            "dialect_explicitly_set": dialect_explicitly_set,
            // The deliberate session override, when one is in force — `null`
            // otherwise. Observable so a caller (and the e2e suite) can tell a
            // temporary chat-command override from a configured dialect.
            "session_dialect_override": session_override,
            "extra_commands": extra_commands,
            "signature_help_disabled_commands": disabled_signatures,
            "known_folder_uris": known_folders
                .iter()
                .map(|f| f.as_str().to_owned())
                .collect::<Vec<String>>(),
            "features": features,
            "optimiser_enabled": optimiser_enabled,
            "optimiser_profile": optimiser_profile,
            "library_paths": library_paths,
            "workspace_scan_max_files": workspace_scan_max_files,
            // Session-wide: whether the tool-environment explanation may be
            // shown (`tclLsp.notifications.environmentKind`).
            "notifications_environment_kind": self.environment_notice.enabled(),
            "spec_packs": spec_packs,
            "spec_packs_loaded": spec_packs_loaded,
            "spec_pack_publication": spec_pack_publication,
            "pack_file_extensions": pack_file_extensions,
            "line_length": line_length,
            "docstring_style": docstring_style_str,
            "non_ascii_mode": non_ascii_mode_str(mode),
            "disabled_diagnostics": disabled_sorted,
            // The declared iRulesLX plugin associations, resolved to
            // absolute paths — session-wide, so a caller can see what is
            // configured without naming a document inside the folder.
            "iruleslx_plugins": iruleslx_plugins,
        })))
    }

    /// Handle `tcl-lsp.exportConfig`: write the resolved settings to the
    /// user config file (`$XDG_CONFIG_HOME/tcl-lsp/config.ini`, else
    /// `~/.config/tcl-lsp/config.ini`) and return its path.  Only the resolved
    /// feature / optimiser / style / disabled-diagnostic state is written.
    pub(in super::super) async fn export_config_command(&self) -> serde_json::Value {
        let path = config_ini_path();
        let contents = self.render_config_ini().await;
        let result = path.parent().map_or_else(
            || Err(std::io::Error::other("no config directory")),
            |parent| self.store.create_dir_all(parent),
        );
        match result.and_then(|()| self.store.write(&path, contents.as_bytes())) {
            // The rendered text comes back with the path. On a host whose
            // store is in memory there is no file for the user to open, so the
            // contents are the only useful half of the answer; natively it is
            // an extra field a client may ignore.
            Ok(()) => serde_json::json!({
                "success": true,
                "path": path.to_string_lossy(),
                "contents": contents,
            }),
            Err(err) => serde_json::json!({
                "success": false,
                "error": err.to_string(),
            }),
        }
    }

    /// Render the resolved config as an INI document (the format the server
    /// reads from `config.ini`): `[features]`, `[optimiser]`, `[style]`, and
    /// the disabled `[diagnostics]` codes.
    pub(in super::super) async fn render_config_ini(&self) -> String {
        use std::fmt::Write as _;
        let features = self.feature_toggles.lock().await.resolved_map();
        let optimiser_enabled = *self.optimiser_enabled.lock().await;
        let line_length = *self.line_length.lock().await;
        let disabled = self.policy_layers.lock().await.disabled_codes();
        let exclude = self.diagnostics_exclude.lock().await.clone();

        let mut out = String::from("# tcl-lsp configuration (exported)\n\n[features]\n");
        let mut feature_keys: Vec<&String> = features.keys().collect();
        feature_keys.sort();
        for key in feature_keys {
            let enabled = features.get(key).and_then(serde_json::Value::as_bool) == Some(true);
            let _ = writeln!(out, "{key} = {enabled}");
        }
        let _ = write!(out, "\n[optimiser]\nenabled = {optimiser_enabled}\n");
        let _ = write!(out, "\n[style]\nlineLength = {line_length}\n");
        if !disabled.is_empty() || !exclude.is_empty() {
            out.push_str("\n[diagnostics]\n");
            for code in disabled {
                let _ = writeln!(out, "{code} = false");
            }
            // The exclude globs round-trip as the continuation list the
            // parser reads — never a comma list, which would split
            // a brace alternation apart.
            if !exclude.is_empty() {
                out.push_str("exclude =\n");
                for pattern in exclude {
                    let _ = writeln!(out, "    {pattern}");
                }
            }
        }
        out
    }
}

/// Resolve the user config-file path for `tcl-lsp.exportConfig`:
/// `$XDG_CONFIG_HOME/tcl-lsp/config.ini`, falling back to
/// `$HOME/.config/tcl-lsp/config.ini`.  Both env vars are honoured on every
/// platform so tests can isolate the server's config via `XDG_CONFIG_HOME`.
pub(in super::super) fn config_ini_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| std::path::PathBuf::from(".config"));
    base.join("tcl-lsp").join("config.ini")
}

/// Render a [`NonAsciiMode`] for `getEffectiveConfig`.  The unset
/// [`NonAsciiMode::Default`] reports as JSON `null` (the per-dialect
/// auto behaviour); every explicit
/// mode reports its setting string.
pub(in super::super) fn non_ascii_mode_str(mode: NonAsciiMode) -> serde_json::Value {
    let label = match mode {
        NonAsciiMode::Default => return serde_json::Value::Null,
        NonAsciiMode::Off => "off",
        NonAsciiMode::Strict => "strict",
        NonAsciiMode::Confusables => "confusables",
        NonAsciiMode::Common => "common",
    };
    serde_json::Value::String(label.to_owned())
}
