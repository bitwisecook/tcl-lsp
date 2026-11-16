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

//! Editor and project settings: the pull, the layers, per-folder overrides and the values a document resolves.

use std::collections::HashSet;

use tcl_compiler::analyser::{Analyser, NonAsciiMode};
use tcl_lsp_core::diagnostic_policy as core_policy;
use tower_lsp_server::ls_types::Uri;

use crate::workspace::uri_under_folder;

mod feature_toggles;
mod folder_settings;
mod formatter_settings;
mod global_settings;
mod reload;
mod requests;
mod resolved_settings;
mod setting_parsers;

pub(super) use feature_toggles::FeatureToggles;
#[cfg(test)]
pub(super) use folder_settings::parse_folder_config;
pub(super) use folder_settings::resolve_ilx_specs;
pub(super) use formatter_settings::{apply_formatting_object, formatter_config_from};
#[cfg(test)]
pub(super) use reload::CONFIG_RELOAD_DEBOUNCE;
pub(super) use reload::ConfigReloadSlot;
#[cfg(test)]
pub(super) use setting_parsers::collapse_inlay_alias;
#[cfg(test)]
pub(super) use setting_parsers::read_ini_layer;
pub(super) use setting_parsers::{
    BigipVersionSetting, default_optimiser_profile, normalize_config_payload,
    settings_environment_kind_enabled, settings_non_ascii_mode, workspace_trust_in,
};
#[cfg(test)]
pub(super) use setting_parsers::{parse_bigip_version, parse_non_ascii_mode};

/// The three configuration layers one scope resolves its diagnostic policy
/// from, lowest first: the user's global `config.ini`, the editor's
/// `workspace/configuration` payload, and the project's `.tcl-lsp.ini`.
///
/// Kept unmerged, because only the unmerged layers can name the layer that
/// decided a code (`docs/design/compiler/diagnostic-policy.md`
/// § Configuration). The session holds one set; a configured workspace
/// folder holds its own ([`FolderConfig::policy_layers`]), and longest-prefix
/// matching over folders — the one LSP-specific part of the resolution —
/// picks between them ([`Backend::resolved_policy_layers`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct PolicyLayers {
    pub(super) global: serde_json::Value,
    pub(super) editor: serde_json::Value,
    pub(super) project: serde_json::Value,
}

impl PolicyLayers {
    /// A builder with the three layers applied, lowest first.
    pub(super) fn builder(&self) -> core_policy::PolicyBuilder {
        core_policy::PolicyBuilder::new()
            .layer(core_policy::PolicyLayer::Global, &self.global)
            .layer(core_policy::PolicyLayer::Editor, &self.editor)
            .layer(core_policy::PolicyLayer::Project, &self.project)
    }

    /// The analyser's production-time skip under these layers — the codes
    /// the per-code decision turns off, which it need not compute
    /// ([`core_policy::Policy::production_skip`]). Every write of the
    /// session's [`crate::BackendState::disabled_diagnostics`] and of a configured
    /// folder's [`FolderConfig::disabled_diagnostics`] is this, so the skip
    /// and the policy come from the same layers and the report can explain
    /// every gap.
    pub(super) fn production_skip(&self) -> HashSet<String> {
        self.builder()
            .build()
            .production_skip()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// [`Self::production_skip`] sorted, as the salsa `AnalyserConfig` input
    /// holds it: a stable order, so an unchanged set never reads as an edit.
    pub(super) fn sorted_production_skip(&self) -> Vec<String> {
        let mut skip: Vec<String> = self.production_skip().into_iter().collect();
        skip.sort();
        skip
    }

    /// Every code these layers turn off, sorted — what the configuration
    /// disables ([`core_policy::Policy::disabled_codes`]), a fact code the
    /// analyser still computes included. What the INI export writes; the
    /// analyser's skip is [`Self::production_skip`].
    pub(super) fn disabled_codes(&self) -> Vec<String> {
        let mut codes: Vec<String> = self
            .builder()
            .build()
            .disabled_codes()
            .iter()
            .map(ToString::to_string)
            .collect();
        codes.sort();
        codes
    }

    /// The disabled set `getEffectiveConfig` reports, sorted: `skip`, the
    /// analyser's skip in force, plus the fact codes these layers turn off,
    /// which no skip carries ([`core_policy::FACT_CODES`]). Once a
    /// configuration re-pull has settled it is [`Self::disabled_codes`].
    ///
    /// The skip is read rather than re-derived because a re-pull writes it
    /// after the layers, inside the analyser-inputs gate: a client that waits
    /// for a code to leave the list knows the analyser computes it again. A
    /// fact code needs no such wait — the analyser computes it whatever the
    /// layers say, and only the policy, which reads the layers, decides it.
    pub(super) fn reported_disabled<'a>(
        &self,
        skip: impl IntoIterator<Item = &'a String>,
    ) -> Vec<String> {
        let mut codes: std::collections::BTreeSet<String> = skip.into_iter().cloned().collect();
        codes.extend(
            self.builder()
                .build()
                .disabled_codes()
                .into_iter()
                .filter(|code| core_policy::FACT_CODES.contains(code))
                .map(|code| code.to_string()),
        );
        codes.into_iter().collect()
    }
}

/// A folder's `tclLsp.diagnostics.genericVariablePatterns` override (IRULE4002).
///
/// A 3-state resolution, distinguishing "the folder said nothing" from "the
/// folder explicitly asked for the built-in defaults" from "the folder supplied
/// its own list".
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum FolderGenericPatterns {
    /// The folder did not set the value: inherit the process-global
    /// `genericVariablePatterns`.
    #[default]
    Inherit,
    /// The folder set the value to the built-in default patterns: the analyser
    /// uses its built-in generic-name set.
    BuiltinDefaults,
    /// The folder supplied its own list: replace the built-in patterns with it.
    Replace(Vec<String>),
}

/// One workspace folder's editor configuration overrides.
///
/// Each field is `None` (or empty) when the folder's pulled `tclLsp` config
/// did not set it, in which case the resolver falls back to the process-global
/// value.  In a multi-root workspace each root can
/// carry its own diagnostics, optimiser, formatting, dialect, and feature
/// settings.  The other source of a per-folder dialect —
/// The analyser inputs `VS Code` declares `"scope": "resource"` — settings whose
/// value belongs to the document rather than the session.
///
/// Bundled rather than passed as three more positional arguments: wired one at
/// a time, each reaches only a subset of the analyser construction sites, and a
/// resource-scoped input missing from a site is silently off for every analysis
/// that does not go through salsa. A single value that every site applies makes
/// the *next* resource-scoped input a one-line change here instead of an audit.
///
/// Resolve with [`Backend::resource_analyser_inputs`], which answers per
/// document: a folder override wins, else the process-global value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ResourceAnalyserInputs {
    /// `tclLsp.packages.provides` / `[packages.provides]` — what a package's
    /// loader additionally brings up.
    pub(super) package_provides: Vec<(String, Vec<String>)>,
    /// `tclLsp.bigipVersion` — the keyed library-version axis; `None` is the
    /// oldest-supported default.
    pub(super) bigip_version: Option<String>,
    /// `tclLsp.targets` — declared version-target ranges as
    /// `(provider, range)`. Empty leaves range mode off.
    pub(super) declared_targets: Vec<(String, String)>,
    /// `tclLsp.sourceEncoding` — session-wide, carried with the rest so every
    /// analyser a document is built with reads its text the same way.
    pub(super) script_encoding: tcl_registry::value_transfer::ScriptEncoding,
}

impl ResourceAnalyserInputs {
    /// Read every input off a salsa [`AnalyserConfig`](tcl_lsp_db::AnalyserConfig)
    /// handle.
    ///
    /// The handle a document resolves to is already folder-scoped
    /// (`Backend::resolved_db_config`), so a branch that has one needs no
    /// further resolution — and reading all three here keeps it from carrying
    /// some and dropping others.
    pub(super) fn from_db_config(
        config: tcl_lsp_db::AnalyserConfig,
        db: &dyn salsa::Database,
    ) -> Self {
        Self {
            package_provides: config.package_provides(db).clone(),
            bigip_version: config.bigip_version(db).clone(),
            declared_targets: config.targets(db).clone(),
            script_encoding: config.script_encoding(db),
        }
    }

    /// Hand every input to `analyser`. The one place a new resource-scoped
    /// input has to be wired for all construction sites to carry it.
    pub(super) fn apply(self, analyser: Analyser) -> Analyser {
        analyser
            .with_package_provides(self.package_provides)
            .with_bigip_version(self.bigip_version)
            .with_declared_targets(self.declared_targets)
            .with_script_encoding(self.script_encoding)
    }
}

/// `initializationOptions.folderDialects` — lives in
/// [`crate::BackendState::folder_dialects`]; [`Backend::folder_dialect_overrides`] merges
/// the two.
#[derive(Clone, Default)]
pub(super) struct FolderConfig {
    /// `tclLsp.dialect` scoped to this folder.  `None` when the
    /// folder's pulled config names no dialect (or names an unknown one, which
    /// is dropped by [`is_known_dialect_name`] exactly as the
    /// `initializationOptions.folderDialects` path drops it), in which case the
    /// document falls back to the session `default_dialect`.
    pub(super) dialect: Option<String>,
    pub(super) feature_toggles: FeatureToggles,
    pub(super) disabled_diagnostics: Option<HashSet<String>>,
    /// The folder's own configuration layers — the global `config.ini`, the
    /// folder-scoped editor settings and the folder's `.tcl-lsp.ini` — that
    /// its documents' policies are built from. `None` for a folder parsed
    /// from a merged payload alone (a test seam), which resolves under the
    /// session's layers.
    pub(super) policy_layers: Option<PolicyLayers>,
    pub(super) non_ascii_mode: Option<NonAsciiMode>,
    pub(super) line_length: Option<u32>,
    /// `tclLsp.formatting` section override for the formatter; `None` inherits
    /// the global formatting settings.
    pub(super) formatting: Option<serde_json::Value>,
    /// `tclLsp.style.lineLength` override (W111 threshold); `None` inherits the
    /// global value.
    pub(super) style_line_length: Option<u32>,
    /// `tclLsp.extraCommands` override; `None` inherits the global set.
    pub(super) extra_commands: Option<Vec<String>>,
    /// `tclLsp.packages.provides` / `[packages.provides]` override; `None`
    /// inherits the global map. The setting is `"scope": "resource"` and a
    /// `.tcl-lsp.ini` belongs to its own project, so a secondary root's
    /// declarations must not be answered with the primary root's.
    pub(super) package_provides: Option<Vec<(String, Vec<String>)>>,
    /// `tclLsp.bigipVersion` override; `Absent` inherits the global pin. Also
    /// `"scope": "resource"` — one root may target a different BIG-IP release
    /// from its sibling.
    pub(super) bigip_version: BigipVersionSetting,
    /// `tclLsp.targets` override; `None` inherits the global ranges. Also
    /// `"scope": "resource"`, and a `.tcl-lsp.ini` states the project's own
    /// supported range.
    pub(super) declared_targets: Option<Vec<(String, String)>>,
    /// `tclLsp.signatureHelp.disabledCommands` override; `None` inherits the
    /// global list. `Some([])` explicitly enables every built-in signature.
    pub(super) signature_help_disabled_commands: Option<Vec<String>>,
    /// `tclLsp.diagnostics.genericVariablePatterns` override. See
    /// [`FolderGenericPatterns`]: `Inherit` falls back to the global value,
    /// `BuiltinDefaults` selects the analyser's built-in set, and `Replace`
    /// supplies the folder's own list.
    pub(super) generic_variable_patterns: FolderGenericPatterns,
    /// `tclLsp.libraryPaths` override; `None` inherits the global set.
    pub(super) library_paths: Option<Vec<String>>,
    /// `tclLsp.specPacks` scoped to this folder; `None` inherits the session
    /// value. Kept per folder rather than folded into the session list
    /// because a **relative** entry here means "under this folder" — the
    /// client answered the pull for this folder's `scopeUri`. Resolving it
    /// against every root would invent packs the user never configured, and
    /// dropping it would lose a secondary folder's packs entirely in a
    /// multi-root workspace.
    pub(super) spec_packs: Option<Vec<String>>,
    /// `.tcl-lsp.ini [project] entryPoints` — the project's "main" files (paths
    /// relative to the folder root). When set, the W120 workspace refinement
    /// treats these entries' `package require`s as available across the folder
    /// and disables the automatic `source`-graph inheritance. `None` / empty
    /// leaves auto-detection on.
    pub(super) entry_points: Option<Vec<String>>,
    /// `tclLsp.diagnostics.exclude` override — glob patterns naming
    /// files that produce no diagnostics at all; `None` inherits the global
    /// list.  `Some` (possibly empty) whenever the folder's merged config
    /// carries the key, so a folder list replaces the global one rather
    /// than unioning with it.
    pub(super) diagnostics_exclude: Option<Vec<String>>,
    /// `tclLsp.iruleslx` / `.tcl-lsp.ini [iruleslx.plugins]` + `[iruleslx.rules]`
    /// — the declared iRulesLX plugin associations. Held per folder
    /// because the paths are folder-relative, exactly as `entry_points` is.
    pub(super) iruleslx: Vec<IlxPluginSpec>,
}

/// One `[iruleslx.plugins]` entry, with any `[iruleslx.rules]` directories
/// declared for the same plugin, as written in the configuration.
///
/// Paths stay unresolved here: a relative path means "under this folder", and
/// the folder root is only known where the config is read
/// ([`Backend::ilx_plugin_roots`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct IlxPluginSpec {
    /// The `PLUGIN` word of `ILX::init`.
    pub(super) plugin: String,
    /// The workspace directory holding `extensions/`.
    pub(super) workspace: String,
    /// Extra directories holding callers of this plugin.
    pub(super) extra_rule_dirs: Vec<String>,
}

/// The deepest workspace folder containing `uri` — the folder itself when a
/// folder URI was passed. That is the scope whose overrides every per-folder
/// value resolved through, so `tcl-lsp.getEffectiveConfig` reports it beside
/// them.
pub(super) fn deepest_folder_containing(uri: &Uri, folders: &[Uri]) -> Option<String> {
    folders
        .iter()
        .filter(|folder| uri_under_folder(uri.as_str(), folder.as_str()))
        .max_by_key(|folder| folder.as_str().len())
        .map(|folder| folder.as_str().to_owned())
}

/// Pick the value associated with the **longest** folder URI that `uri` sits
/// under.
pub(super) fn longest_folder_match<'a, T>(entries: &'a [(Uri, T)], uri: &Uri) -> Option<&'a T> {
    let mut best: Option<&'a (Uri, T)> = None;
    for entry in entries {
        if uri_under_folder(uri.as_str(), entry.0.as_str())
            && best.is_none_or(|b| entry.0.as_str().len() > b.0.as_str().len())
        {
            best = Some(entry);
        }
    }
    best.map(|(_, value)| value)
}
