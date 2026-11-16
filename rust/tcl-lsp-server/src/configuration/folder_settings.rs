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

//! One workspace folder's overrides: their shape, their parse and their application.

use super::setting_parsers::{
    BigipVersionSetting, parse_bigip_version, parse_declared_targets, parse_package_provides,
    settings_non_ascii_mode,
};
use super::{
    Backend, FolderConfig, FolderGenericPatterns, HashMap, IlxPluginSpec, Path, PolicyLayers, Uri,
    core_ilx, is_known_dialect_name,
};

impl Backend {
    /// Store the per-folder editor configs and refresh the per-folder salsa
    /// `AnalyserConfig` handles.  A handle is created only for a folder one of
    /// whose *resolved* analyser inputs — the sorted skip, the non-ASCII mode,
    /// the extra commands, the generic variable patterns, the package
    /// provides, the BIG-IP version or the targets — differs from the
    /// session's; every other folder shares [`crate::BackendState::db_config`] and its
    /// memo, so a document under it is analysed once per revision for
    /// diagnostics and symbols alike.  Existing handles are reused across
    /// re-pulls so the salsa store does not accumulate dead config inputs.
    ///
    /// A folder that stops overriding anything — or leaves the workspace — has
    /// its handle *retired* into [`crate::BackendState::folder_db_config_tombstones`] with
    /// its payload cleared, not dropped, and a folder that starts overriding
    /// again revives the retired handle.  Salsa never frees an input, so
    /// dropping one leaks it: `.tcl-lsp.ini` re-pulls the whole layered config
    /// on every save, which would otherwise allocate a fresh config input per
    /// save for a folder toggling an override.
    pub(in super::super) async fn apply_folder_configs(&self, parsed: Vec<(Uri, FolderConfig)>) {
        use salsa::Setter as _;
        let analyser_inputs_guard = self.analyser_inputs_gate.write().await;
        {
            let global_disabled = self.sorted_global_disabled_diagnostics().await;
            let global_mode = *self.non_ascii_mode.lock().await;
            // Per-folder `extraCommands` / `genericVariablePatterns` override the
            // process-global value when set; otherwise the folder inherits it.
            let global_extra = self.extra_commands.lock().await.clone();
            let global_generic = self.generic_variable_patterns.lock().await.clone();
            let global_provides = self.package_provides.lock().await.clone();
            let global_bigip = self.bigip_version.lock().await.clone();
            let global_targets = self.declared_targets.lock().await.clone();
            let encoding = self.declared_script_encoding();
            // Packs are a workspace fact, never a per-folder one, so every
            // folder handle carries the same key the global config does.
            let pack_key = self.spec_packs().await.key;
            let mut db = self.db.lock().await;
            let mut handles = self.folder_db_configs.lock().await;
            let mut tombstones = self.folder_db_config_tombstones.lock().await;
            // Every live handle starts out claimed-back; whatever is still here
            // when the loop ends belongs to a folder that no longer needs one and
            // is retired below.
            let mut reclaimable: HashMap<Uri, tcl_lsp_db::AnalyserConfig> =
                handles.drain(..).collect();
            let mut next: Vec<(Uri, tcl_lsp_db::AnalyserConfig)> = Vec::new();
            for (folder, fc) in &parsed {
                let mut disabled: Vec<String> = match &fc.disabled_diagnostics {
                    Some(d) => d.iter().cloned().collect(),
                    None => global_disabled.clone(),
                };
                disabled.sort();
                let mode = fc.non_ascii_mode.unwrap_or(global_mode);
                let extra = fc
                    .extra_commands
                    .clone()
                    .unwrap_or_else(|| global_extra.clone());
                let generic = match &fc.generic_variable_patterns {
                    FolderGenericPatterns::Inherit => global_generic.clone(),
                    FolderGenericPatterns::BuiltinDefaults => None,
                    FolderGenericPatterns::Replace(list) => Some(list.clone()),
                };
                let provides = fc
                    .package_provides
                    .clone()
                    .unwrap_or_else(|| global_provides.clone());
                let bigip = match &fc.bigip_version {
                    BigipVersionSetting::Present(v) => v.clone(),
                    BigipVersionSetting::Absent => global_bigip.clone(),
                };
                let targets = fc
                    .declared_targets
                    .clone()
                    .unwrap_or_else(|| global_targets.clone());
                // A handle is only needed when a resolved input differs from
                // the session's: a folder that configures the session's own
                // values — every configured folder carries its own skip — reads
                // the session's handle and shares its memo.
                if disabled == global_disabled
                    && mode == global_mode
                    && extra == global_extra
                    && generic == global_generic
                    && provides == global_provides
                    && bigip == global_bigip
                    && targets == global_targets
                {
                    continue;
                }
                // The folder's own live handle first, then its retired one, and
                // only then a fresh input.
                let handle = if let Some(handle) = reclaimable
                    .remove(folder)
                    .or_else(|| tombstones.remove(folder))
                {
                    handle.set_disabled_diagnostics(&mut *db).to(disabled);
                    handle.set_non_ascii_mode(&mut *db).to(mode);
                    handle.set_extra_commands(&mut *db).to(extra);
                    handle.set_generic_variable_patterns(&mut *db).to(generic);
                    handle.set_spec_pack_key(&mut *db).to(pack_key);
                    handle.set_package_provides(&mut *db).to(provides);
                    handle.set_bigip_version(&mut *db).to(bigip);
                    handle.set_targets(&mut *db).to(targets);
                    handle
                } else {
                    tcl_lsp_db::AnalyserConfig::new(
                        &*db, disabled, mode, extra, generic, bigip, pack_key, targets, provides,
                    )
                };
                handle.set_script_encoding(&mut *db).to(encoding);
                next.push((folder.clone(), handle));
            }
            for (folder, handle) in reclaimable {
                // Release the payload before retiring, so a folder that toggles
                // an override off does not keep its old code lists (and the
                // memos keyed on them) alive until it is toggled back on.
                Self::clear_retired_folder_configuration(&mut db, handle);
                tombstones.insert(folder, handle);
            }
            *handles = next;
            let files = self.db_files.lock().await;
            let members = self.db_project_members.lock().await;
            let mut project = self.db_project.lock().await;
            let global_config = *self.db_config.lock().await;
            Self::sync_db_project_with_configurations(
                &mut db,
                &files,
                &members,
                &mut project,
                global_config,
                &handles,
            );
        }
        *self.folder_configs.lock().await = parsed;
        // The folder chain is what every per-URI knob in `diag_inputs` resolves
        // through, so a replaced chain retires the scheduler's cached copies for
        // the same reason `apply_global_config` does.
        self.invalidate_diag_inputs();
        drop(analyser_inputs_guard);
    }

    async fn sorted_global_disabled_diagnostics(&self) -> Vec<String> {
        let mut global_disabled: Vec<String> = self
            .disabled_diagnostics
            .lock()
            .await
            .iter()
            .cloned()
            .collect();
        global_disabled.sort();
        global_disabled
    }

    fn clear_retired_folder_configuration(
        db: &mut tcl_lsp_db::TclDatabase,
        handle: tcl_lsp_db::AnalyserConfig,
    ) {
        use salsa::Setter as _;
        handle.set_disabled_diagnostics(db).to(Vec::new());
        handle.set_extra_commands(db).to(Vec::new());
        handle.set_generic_variable_patterns(db).to(None);
        handle.set_package_provides(db).to(Vec::new());
        handle.set_targets(db).to(Vec::new());
        handle.set_bigip_version(db).to(None);
        handle.set_targets(db).to(Vec::new());
    }
}

/// Parse one folder's resolved `tclLsp` config object into a [`FolderConfig`]
/// of overrides.  Keys absent from the object stay `None` / empty so the
/// resolver inherits the process-global value.  Returns `None` when `cfg` is
/// not a JSON object (the folder pull returned nothing usable).  Mirrors the
/// key handling of [`Backend::pull_and_apply_config`]'s global pull.
/// Parse the formatter line length, the whole `formatting` section, and the
/// `style.lineLength` (W111) threshold of a folder config.
fn parse_folder_formatting(
    obj: &serde_json::Map<String, serde_json::Value>,
    fc: &mut FolderConfig,
) {
    if let Some(len) = obj
        .get("formatting")
        .and_then(|f| f.get("lineLength"))
        .or_else(|| obj.get("formatting").and_then(|f| f.get("maxLineLength")))
        .or_else(|| obj.get("lineLength"))
        .and_then(serde_json::Value::as_u64)
    {
        fc.line_length = Some(u32::try_from(len).unwrap_or(80));
    }
    if let Some(formatting) = obj.get("formatting") {
        fc.formatting = Some(formatting.clone());
    }
    if let Some(len) = obj
        .get("style")
        .and_then(|s| s.get("lineLength"))
        .and_then(serde_json::Value::as_u64)
        .filter(|&len| len > 0)
    {
        fc.style_line_length = Some(u32::try_from(len).unwrap_or(120));
    }
}

/// Parse the `tclLsp.diagnostics` object keys of one folder's merged config
/// into `fc` — the ones that are not per-code booleans (those are the
/// policy's per-code decisions, read by [`PolicyLayers`]):
///
/// - `genericVariablePatterns` — a present array replaces the patterns
///   (`Replace`); an explicit `null` requests the analyser's built-in
///   defaults (`BuiltinDefaults`); an absent key inherits the global value
///   (`Inherit`, the default).
/// - `exclude` — glob patterns naming files that produce no
///   diagnostics at all. `Some` (possibly empty) whenever the merged config
///   carries the key, so a folder list replaces the global one rather than
///   unioning with it.
fn parse_folder_diagnostics(
    obj: &serde_json::Map<String, serde_json::Value>,
    fc: &mut FolderConfig,
) {
    let Some(diagnostics) = obj
        .get("diagnostics")
        .and_then(serde_json::Value::as_object)
    else {
        return;
    };
    if let Some(value) = diagnostics.get("genericVariablePatterns") {
        if let Some(patterns) = value.as_array() {
            fc.generic_variable_patterns = FolderGenericPatterns::Replace(
                patterns
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect(),
            );
        } else if value.is_null() {
            fc.generic_variable_patterns = FolderGenericPatterns::BuiltinDefaults;
        }
    }
    if let Some(patterns) = diagnostics
        .get("exclude")
        .and_then(serde_json::Value::as_array)
    {
        fc.diagnostics_exclude = Some(
            patterns
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        );
    }
}

fn normalise_disabled_signature_commands(values: &[serde_json::Value]) -> Vec<String> {
    let mut commands: Vec<String> = values
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .map(tcl_lsp_core::normalise_qualified_command_name)
        .collect();
    commands.sort();
    commands.dedup();
    commands
}

pub(in super::super) fn signature_help_disabled_commands(
    cfg: &serde_json::Value,
) -> Option<Vec<String>> {
    cfg.get("signatureHelp")
        .and_then(|section| section.get("disabledCommands"))
        .and_then(serde_json::Value::as_array)
        .map(|commands| normalise_disabled_signature_commands(commands))
}

pub(in super::super) fn parse_folder_config(cfg: &serde_json::Value) -> Option<FolderConfig> {
    let obj = cfg.as_object()?;
    let mut fc = FolderConfig::default();
    // `tclLsp.dialect` scoped to this folder.  Validated with the
    // same predicate the `initializationOptions.folderDialects` path uses, so an
    // unknown name is dropped (the folder inherits the session default) rather
    // than pinning documents to a dialect no provider can resolve.
    if let Some(dialect) = obj
        .get("dialect")
        .and_then(serde_json::Value::as_str)
        .filter(|d| is_known_dialect_name(d))
    {
        fc.dialect = Some(dialect.to_owned());
    }
    if let Some(features) = obj.get("features").and_then(serde_json::Value::as_object) {
        fc.feature_toggles.apply(features);
    }
    // `xcDiagnostics.enabled` is a config section, not a `features.*` key
    // (see `pull_and_apply_config`) — map it onto the toggle for the folder.
    if let Some(flag) = obj
        .get("xcDiagnostics")
        .and_then(|x| x.get("enabled"))
        .and_then(serde_json::Value::as_bool)
    {
        fc.feature_toggles.set_flag("xcDiagnostics", flag);
    }
    parse_folder_formatting(obj, &mut fc);
    // `tclLsp.extraCommands` per-folder override.
    if let Some(cmds) = obj
        .get("extraCommands")
        .and_then(serde_json::Value::as_array)
    {
        fc.extra_commands = Some(
            cmds.iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        );
    }
    // The `"scope": "resource"` analyser inputs, each parsed through the same
    // normaliser the global path uses so the two cannot disagree.
    if let Some(map) = obj
        .get("packages")
        .and_then(|p| p.get("provides"))
        .and_then(serde_json::Value::as_object)
    {
        fc.package_provides = Some(parse_package_provides(map));
    }
    // `Present(None)` — an explicit empty string — clears the pin for this
    // folder rather than inheriting the global one, so it is a real override.
    fc.bigip_version = parse_bigip_version(cfg);
    if let Some(map) = obj.get("targets").and_then(serde_json::Value::as_object) {
        fc.declared_targets = Some(parse_declared_targets(map));
    }
    fc.signature_help_disabled_commands = signature_help_disabled_commands(cfg);
    parse_folder_diagnostics(obj, &mut fc);
    // `tclLsp.libraryPaths` per-folder override.
    if let Some(paths) = obj
        .get("libraryPaths")
        .and_then(serde_json::Value::as_array)
    {
        fc.library_paths = Some(
            paths
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        );
    }
    // `tclLsp.specPacks` per-folder override. Trimmed and emptied-dropped
    // exactly as the session-scope parse does, so the two paths cannot
    // disagree about what an entry is.
    if let Some(paths) = obj.get("specPacks").and_then(serde_json::Value::as_array) {
        fc.spec_packs = Some(
            paths
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        );
    }
    // `.tcl-lsp.ini [project] entryPoints` per-folder value.
    if let Some(points) = obj.get("entryPoints").and_then(serde_json::Value::as_array) {
        fc.entry_points = Some(
            points
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
        );
    }
    fc.iruleslx = parse_ilx_plugins(obj);
    // The non-ASCII helper expects the value wrapped under `tclLsp`; the
    // per-folder pull hands us the section content directly.
    let wrapped = serde_json::json!({ "tclLsp": cfg });
    fc.non_ascii_mode = settings_non_ascii_mode(&wrapped);
    // The merged payload as a lone editor layer — the test seam's skip. The
    // configuration pull overwrites it with the production skip of the
    // folder's own three layers, which also become its policy.
    fc.disabled_diagnostics = Some(
        PolicyLayers {
            editor: cfg.clone(),
            ..PolicyLayers::default()
        }
        .production_skip(),
    );
    Some(fc)
}

/// `tclLsp.iruleslx.plugins` / `.rules` → the folder's declared iRulesLX plugin
/// associations.
///
/// A `rules` entry for a plugin with no `plugins` entry is dropped: extra
/// caller directories are only meaningful once the plugin's own workspace is
/// known, and keeping a half-declaration would silently widen the search for a
/// plugin that still resolves by the directory-name convention — an
/// association the user did not make.
/// Resolve one folder's declared specs against that folder's root.
///
/// A relative path means "under this folder"; an absolute one is taken as
/// given. Both are lexically normalised, so a declaration written as
/// `../shared/ws` compares equal to the same directory reached from a
/// document's own ancestors — which is what lets the JavaScript end recognise
/// a declared workspace.
pub(in super::super) fn resolve_ilx_specs(
    specs: &[IlxPluginSpec],
    root: &Path,
) -> Vec<core_ilx::IlxPluginRoot> {
    specs
        .iter()
        .map(|spec| core_ilx::IlxPluginRoot {
            plugin: spec.plugin.clone(),
            workspace: tcl_lsp_core::source_graph::resolve_under(root, &spec.workspace),
            extra_rule_dirs: spec
                .extra_rule_dirs
                .iter()
                .map(|dir| tcl_lsp_core::source_graph::resolve_under(root, dir))
                .collect(),
        })
        .collect()
}

fn parse_ilx_plugins(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<IlxPluginSpec> {
    let Some(ilx) = obj.get("iruleslx").and_then(serde_json::Value::as_object) else {
        return Vec::new();
    };
    let Some(plugins) = ilx.get("plugins").and_then(serde_json::Value::as_object) else {
        return Vec::new();
    };
    let rules = ilx.get("rules").and_then(serde_json::Value::as_object);
    let mut out: Vec<IlxPluginSpec> = plugins
        .iter()
        .filter_map(|(plugin, workspace)| {
            let workspace = workspace.as_str()?.trim();
            (!plugin.is_empty() && !workspace.is_empty()).then(|| IlxPluginSpec {
                plugin: plugin.clone(),
                workspace: workspace.to_owned(),
                extra_rule_dirs: rules
                    .and_then(|r| r.get(plugin))
                    .and_then(serde_json::Value::as_array)
                    .map_or_default(|dirs| {
                        dirs.iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::trim)
                            .filter(|d| !d.is_empty())
                            .map(str::to_owned)
                            .collect()
                    }),
            })
        })
        .collect();
    // A JSON object has no guaranteed order; a stable one keeps the resolved
    // candidate list (and so an ambiguity report) reproducible.
    out.sort_by(|a, b| a.plugin.cmp(&b.plugin));
    out
}
