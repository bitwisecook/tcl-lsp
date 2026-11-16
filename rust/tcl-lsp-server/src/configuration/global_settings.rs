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

//! Applying a pulled `tclLsp` section onto the session's global state.

use tower_lsp_server::ls_types::MessageType;

use crate::Backend;
use crate::dialects::{DEFAULT_SESSION_DIALECT, is_known_dialect_name, selectable_dialect_names};
use crate::documents::DocumentSourceResult;

#[cfg(test)]
use super::PolicyLayers;
use super::folder_settings::signature_help_disabled_commands;
use super::setting_parsers::{
    BigipVersionSetting, parse_bigip_version, parse_declared_targets, parse_package_provides,
    settings_environment_kind_enabled, settings_non_ascii_mode,
};

impl Backend {
    /// Apply the *content* of a pulled `tclLsp` config section (`cfg`) onto the
    /// session's global state: feature toggles, the `xcDiagnostics` section
    /// flag, the optimiser switch / profile / per-code overrides, the formatter
    /// line length, the default dialect, the W108 non-ASCII mode, and the
    /// disabled-diagnostic set — then mirror the analyser knobs onto the salsa
    /// config input. A non-object `cfg` is a no-op.
    ///
    /// Split out of [`Self::pull_and_apply_config`] (which fetches `cfg` from
    /// the client and then handles per-folder configs) so the apply logic is
    /// unit-testable without a live editor client.
    // A long but flat sequence of independent `tclLsp.*` knob applications.
    #[cfg(test)]
    pub(in super::super) async fn apply_global_config(
        &self,
        cfg: &serde_json::Value,
    ) -> DocumentSourceResult<()> {
        *self.policy_layers.lock().await = PolicyLayers {
            editor: cfg.clone(),
            ..PolicyLayers::default()
        };
        self.apply_global_config_with_signature_fallback(cfg, cfg)
            .await?;

        Ok(())
    }

    /// Apply the primary root's merged configuration while keeping signature
    /// suppression's folder fallback at the user/editor layers. Project INI
    /// settings belong to their own root and must not leak into siblings whose
    /// merged folder configuration omits `signatureHelp`.
    pub(in super::super) async fn apply_global_config_with_signature_fallback(
        &self,
        cfg: &serde_json::Value,
        signature_fallback_cfg: &serde_json::Value,
    ) -> DocumentSourceResult<()> {
        if !cfg.is_object() {
            return Ok(());
        }
        let analyser_inputs_guard = self.analyser_inputs_gate.write().await;
        if let Some(features) = cfg.get("features").and_then(serde_json::Value::as_object) {
            self.feature_toggles.lock().await.apply(features);
        }
        let mut rescan_workspace = self.apply_global_library_paths(cfg).await;
        rescan_workspace |= self.apply_workspace_scan_budget(cfg).await;
        self.apply_global_toggles(cfg, signature_fallback_cfg).await;
        let dialect_warning = self.apply_global_formatting(cfg).await;
        self.apply_global_analyser_knobs(cfg).await;
        // Mirror the applied analyser knobs onto the salsa config input so the
        // query graph recomputes against the latest settings.
        self.sync_db_config().await;
        // Everything above is an input to `diag_inputs`, so no diagnostics run
        // scheduled before this point may keep its cached copy — including one
        // an edit schedules between here and the reload's own reschedule.
        self.invalidate_diag_inputs();
        drop(analyser_inputs_guard);
        self.warn_configured_dialect(dialect_warning).await;
        let _: () = if rescan_workspace {
            self.scan_workspace_folders().await?;
        };
        Ok(())
    }

    /// `tclLsp.libraryPaths` — the editor layer of the package database's
    /// `auto_path` (the user's picked installation / hand-entered paths).
    /// A change rebuilds the database so the new paths take effect immediately.
    async fn apply_global_library_paths(&self, cfg: &serde_json::Value) -> bool {
        let mut rescan_workspace = false;
        if let Some(paths) = cfg
            .get("libraryPaths")
            .and_then(serde_json::Value::as_array)
        {
            let libs: Vec<String> = paths
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            let changed = {
                let mut guard = self.editor_library_paths.lock().await;
                if *guard == libs {
                    false
                } else {
                    *guard = libs;
                    true
                }
            };
            if changed {
                rescan_workspace = true;
            }
        }
        // `tclLsp.packages.preferLatest` — the interpreter's starting
        // `package prefer` mode. See the field doc for why this is a setting
        // and not read off the server's own environment.
        if let Some(flag) = cfg
            .get("packages")
            .and_then(|p| p.get("preferLatest"))
            .and_then(serde_json::Value::as_bool)
        {
            *self.package_prefer_latest_default.lock().await = flag;
        }
        // `tclLsp.packages.provides` — what a `package require` additionally
        // loads. See the field doc: a binary extension's own
        // `Tcl_PkgRequire` / `Tk_InitStubs` is invisible to every scan, so it
        // is declared.
        if let Some(map) = cfg
            .get("packages")
            .and_then(|p| p.get("provides"))
            .and_then(serde_json::Value::as_object)
        {
            let provides = parse_package_provides(map);
            let changed = {
                let mut guard = self.package_provides.lock().await;
                if *guard == provides {
                    false
                } else {
                    *guard = provides;
                    true
                }
            };
            // The workspace scan bakes these edges into every closed file's
            // index entry, so a change to them has to re-scan — the same
            // treatment `libraryPaths` gets above.
            if changed {
                rescan_workspace = true;
            }
        }
        // `tclLsp.sourceEncoding` — closed files are decoded with it, so a
        // change re-reads every one of them. An unknown name is ignored.
        if let Some(encoding) = cfg
            .get("sourceEncoding")
            .and_then(serde_json::Value::as_str)
            .and_then(tcl_registry::value_transfer::ScriptEncoding::parse)
        {
            let mut guard = self
                .script_encoding
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if *guard != encoding {
                *guard = encoding;
                rescan_workspace = true;
            }
        }
        rescan_workspace
    }

    /// `tclLsp.workspaceScan.maxFiles` — how many on-disk files the workspace
    /// scan indexes (issue #2021).  Returns whether the value moved, so the
    /// caller re-runs the scan for it exactly as it does for a `libraryPaths`
    /// change: the cap decides which files are in the index, so a raised cap
    /// only reaches the user once the tree is walked again.
    ///
    /// A value below 1 is ignored rather than obeyed — `0` would mean "index
    /// nothing", which no user asking for a file budget means, and the editor
    /// schemas declare `minimum: 1`.  The rescan is suppressed until the first
    /// scan has completed: before that, `initialized` is about to walk the
    /// tree anyway and rescanning here would just walk a large workspace
    /// twice.
    async fn apply_workspace_scan_budget(&self, cfg: &serde_json::Value) -> bool {
        let Some(max) = cfg
            .get("workspaceScan")
            .and_then(|w| w.get("maxFiles"))
            .and_then(serde_json::Value::as_u64)
            .filter(|&max| max > 0)
        else {
            return false;
        };
        let max = usize::try_from(max).unwrap_or(usize::MAX);
        let changed = {
            let mut guard = self.workspace_scan_max_files.lock().await;
            let changed = *guard != max;
            *guard = max;
            changed
        };
        changed && self.workspace_scan_ready.is_complete()
    }

    /// The interpreter's **starting** `package prefer` mode — the base
    /// `package_prefer_at` latches up from.
    pub(in super::super) async fn default_package_prefer(
        &self,
    ) -> tcl_lsp_core::package_resolver::PackagePrefer {
        if *self.package_prefer_latest_default.lock().await {
            tcl_lsp_core::package_resolver::PackagePrefer::Latest
        } else {
            tcl_lsp_core::package_resolver::PackagePrefer::Stable
        }
    }

    /// Feature controls: per-command signature suppression, `xcDiagnostics`,
    /// and the optimiser switch and profile that `getEffectiveConfig` and the
    /// INI export report. What shows — the optimiser's codes, the `shimmer`
    /// family — is the diagnostic policy's, built from [`PolicyLayers`].
    async fn apply_global_toggles(
        &self,
        cfg: &serde_json::Value,
        signature_fallback_cfg: &serde_json::Value,
    ) {
        // This fallback contains only the user-config and editor layers. Each
        // project INI is represented by its FolderConfig, so the primary
        // project's exclusions cannot bleed into a sibling root. An absent
        // section means the list was removed from every fallback layer; reset
        // unconditionally rather than latching stale exclusions after an edit.
        *self.signature_help_disabled_commands.lock().await =
            signature_help_disabled_commands(signature_fallback_cfg).unwrap_or_default();
        // `tclLsp.xcDiagnostics.enabled` is a dedicated config section (the
        // shipped VS Code setting "XC Migration: Enabled"), not a `features.*`
        // key, so it must be mapped onto the `xcDiagnostics` feature toggle
        // here.
        if let Some(flag) = cfg
            .get("xcDiagnostics")
            .and_then(|x| x.get("enabled"))
            .and_then(serde_json::Value::as_bool)
        {
            self.feature_toggles
                .lock()
                .await
                .set_flag("xcDiagnostics", flag);
        }
        if let Some(flag) = cfg
            .get("optimiser")
            .and_then(|o| o.get("enabled"))
            .and_then(serde_json::Value::as_bool)
        {
            *self.optimiser_enabled.lock().await = flag;
        }
        // `tclLsp.notifications.environmentKind` — whether the tool-environment
        // explanation may be shown. Read when a notice is about to go out, so
        // a change applies to the next one without a restart.
        if let Some(flag) = settings_environment_kind_enabled(cfg) {
            self.environment_notice.set_enabled(flag);
        }
        if let Some(profile) = cfg
            .get("optimiser")
            .and_then(|o| o.get("profile"))
            .and_then(serde_json::Value::as_str)
        {
            *self.optimiser_profile.lock().await =
                tcl_compiler::optimiser::profiles::OptimisationProfile::parse(profile);
        }
    }

    /// The formatter / style-width / default-dialect knobs. Returns the
    /// warning for a configured dialect that names no environment, for the
    /// caller to send after it has released the analyser-inputs gate.
    pub(in super::super) async fn apply_global_formatting(
        &self,
        cfg: &serde_json::Value,
    ) -> Option<String> {
        if let Some(len) = cfg
            .get("formatting")
            .and_then(|f| f.get("lineLength"))
            .or_else(|| cfg.get("formatting").and_then(|f| f.get("maxLineLength")))
            .or_else(|| cfg.get("lineLength"))
            .and_then(serde_json::Value::as_u64)
        {
            *self.line_length.lock().await = u32::try_from(len).unwrap_or(80);
        }
        // The whole `tclLsp.formatting` section drives the formatter config
        // (indent size/style, brace/line-length, blank-line policy, …).
        if let Some(formatting) = cfg.get("formatting") {
            *self.formatting_settings.lock().await = formatting.clone();
        }
        // `tclLsp.style.lineLength` — the W111 threshold (distinct from the
        // formatter width above). A positive value replaces the default 120.
        if let Some(len) = cfg
            .get("style")
            .and_then(|s| s.get("lineLength"))
            .and_then(serde_json::Value::as_u64)
            .filter(|&len| len > 0)
        {
            *self.style_line_length.lock().await = u32::try_from(len).unwrap_or(120);
        }
        match cfg.get("dialect").and_then(serde_json::Value::as_str) {
            Some(dialect) => self.apply_configured_session_dialect(dialect).await,
            None => None,
        }
    }

    /// Set the session dialect from a configured `tclLsp.dialect`.
    ///
    /// A value that resolves to an environment is stored as written, because a
    /// sidecar stub is found by the configured spelling. One that names no
    /// environment is not stored: the session keeps the built-in default
    /// rather than resolving every document to the lenient `tcl` sink, and the
    /// returned warning names the value and the selectable dialects. A blank
    /// value is an unset setting and keeps the default without a warning, and
    /// a rejected value is reported once while it stays configured.
    pub(in super::super) async fn apply_configured_session_dialect(
        &self,
        dialect: &str,
    ) -> Option<String> {
        let mut warned = self.warned_session_dialect.lock().await;
        if is_known_dialect_name(dialect) {
            *self.default_dialect.lock().await = dialect.to_owned();
            *self.default_dialect_explicit.lock().await = true;
            *warned = None;
            return None;
        }
        DEFAULT_SESSION_DIALECT.clone_into(&mut *self.default_dialect.lock().await);
        *self.default_dialect_explicit.lock().await = false;
        if dialect.trim().is_empty() {
            *warned = None;
            return None;
        }
        if warned.as_deref() == Some(dialect) {
            return None;
        }
        *warned = Some(dialect.to_owned());
        Some(format!(
            "tclLsp.dialect `{dialect}` is not a known dialect; using the default, \
             {DEFAULT_SESSION_DIALECT}. Valid dialects: {}",
            selectable_dialect_names()
        ))
    }

    /// Send the warning [`Self::apply_configured_session_dialect`] returned, if
    /// there is one, on the client's log channel.
    pub(in super::super) async fn warn_configured_dialect(&self, warning: Option<String>) {
        if let Some(message) = warning {
            self.client.log_message(MessageType::WARNING, message).await;
        }
    }

    /// The analyser inputs: `extraCommands`, `genericVariablePatterns`, the
    /// non-ASCII (W108) mode, and the disabled-diagnostics set.
    async fn apply_global_analyser_knobs(&self, cfg: &serde_json::Value) {
        // `tclLsp.extraCommands` — names treated as known commands (no W123).
        if let Some(cmds) = cfg
            .get("extraCommands")
            .and_then(serde_json::Value::as_array)
        {
            let extra: Vec<String> = cmds
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            *self.extra_commands.lock().await = extra;
        }
        // `tclLsp.specPacks` — extra `.tclspec` files or directories, on top
        // of the ones discovery finds by convention (`.tcl-lsp/`, beside a
        // `tclpkg.tcl`). A relative entry is resolved against each workspace
        // folder; the reload itself is driven by `initialized` and the file
        // watcher, so this only records the paths.
        if let Some(paths) = cfg.get("specPacks").and_then(serde_json::Value::as_array) {
            let paths: Vec<String> = paths
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect();
            *self.spec_pack_paths.lock().await = paths;
        }
        // `tclLsp.bigipVersion` — the target BIG-IP release for the keyed
        // library-version axis (an empty string clears the pin back to the
        // oldest-supported default).
        if let BigipVersionSetting::Present(version) = parse_bigip_version(cfg) {
            *self.bigip_version.lock().await = version;
        }
        // `tclLsp.targets` — declared version-target ranges: an object of
        // provider → range, e.g.
        // `{ "tcl": "8.5-9.0", "Tk": "8.5-8.6" }`. Reset unconditionally so
        // removing the setting switches range mode back off.
        {
            let targets = cfg
                .get("targets")
                .and_then(serde_json::Value::as_object)
                .map_or_else(Vec::new, parse_declared_targets);
            *self.declared_targets.lock().await = targets;
        }
        // `tclLsp.diagnostics.genericVariablePatterns` — replaces the built-in
        // IRULE4002 generic-name set (an explicit empty list disables the
        // check; an absent key leaves the default).
        if let Some(patterns) = cfg
            .get("diagnostics")
            .and_then(serde_json::Value::as_object)
            .and_then(|d| d.get("genericVariablePatterns"))
            .and_then(serde_json::Value::as_array)
        {
            let patterns: Vec<String> = patterns
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
                .collect();
            *self.generic_variable_patterns.lock().await = Some(patterns);
        }
        // `tclLsp.diagnostics.exclude` — glob patterns for files that produce
        // no diagnostics at all. Reset unconditionally: the production
        // caller always passes the fully merged config, so an absent key means
        // the exclusion list really is empty now — a stale list must not keep
        // suppressing a file the user just un-excluded.
        let exclude: Vec<String> = cfg
            .get("diagnostics")
            .and_then(serde_json::Value::as_object)
            .and_then(|d| d.get("exclude"))
            .and_then(serde_json::Value::as_array)
            .map_or_default(|patterns| {
                patterns
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect()
            });
        *self.diagnostics_exclude.lock().await = exclude;
        // The pulled value is the *content* of the `tclLsp` section; the
        // `settings_non_ascii_mode` helper expects it wrapped (it looks under
        // `tclLsp`), so re-wrap before reusing it for the W108 mode.
        let wrapped = serde_json::json!({ "tclLsp": cfg.clone() });
        if let Some(mode) = settings_non_ascii_mode(&wrapped) {
            *self.non_ascii_mode.lock().await = mode;
        }
        // The analyser's skip is the production skip of the session's
        // layers, which both callers set before this runs
        // (`pull_and_apply_config_values`, and `apply_global_config` in
        // tests) — the same layers the documents' policies are built from.
        let skip = self.policy_layers.lock().await.production_skip();
        *self.disabled_diagnostics.lock().await = skip;
    }
}
