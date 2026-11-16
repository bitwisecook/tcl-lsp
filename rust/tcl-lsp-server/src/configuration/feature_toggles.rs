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

//! The `tclLsp.features.*` toggles and their per-folder resolution.

use super::{Backend, HashMap, Uri, longest_folder_match};

/// Resolved `tclLsp.features.*` toggle state.
///
/// Stores only the keys an editor has explicitly set; every other
/// feature resolves to enabled.  This follows
/// "absent → default-on" semantics and the config-pull restore
/// contract (a pulled config only *sets* the keys it carries).
#[derive(Debug, Default, Clone)]
pub(in super::super) struct FeatureToggles {
    pub(in super::super) set: HashMap<String, bool>,
}

impl FeatureToggles {
    /// camelCase feature keys reported by `getEffectiveConfig`.
    pub(in super::super) const KEYS: &'static [&'static str] = &[
        "hover",
        "completion",
        "diagnostics",
        "semanticTokens",
        "codeActions",
        "definition",
        "references",
        "documentSymbols",
        "folding",
        "rename",
        "signatureHelp",
        "workspaceSymbols",
        // Inlay hints split into two independently-toggled families:
        // inferred-type hints and parameter-name hints.  Both
        // default **off** (see `DEFAULT_OFF`).  The legacy `inlayHints`
        // key is accepted on input as an alias for `inlayTypeHints`
        // (see `apply`).
        "inlayTypeHints",
        "inlayParameterHints",
        "callHierarchy",
        "documentLinks",
        "selectionRange",
        "documentHighlight",
        "codeLens",
        "workspaceFileOps",
        "implementation",
        "typeDefinition",
        "declaration",
        "linkedEditingRange",
        // Opt-in XC100-301 translatability diagnostics for
        // `f5-irules` documents (default **off**).
        "xcDiagnostics",
        // Opt-in cross-file resolution: cross-file W120/W123 suppression
        // and cross-file E002/E003 arity, for *every* dialect (default
        // **off** — see `Backend::cross_file_resolution_enabled`).
        // Deliberately separate from `xcDiagnostics`, which gates only the
        // f5-irules-specific XC100-301 translatability lints, so a plain
        // Tcl project can opt into cross-file analysis without also opting
        // into an unrelated F5 migration feature.
        "crossFileResolution",
    ];

    /// camelCase feature keys that default **off** (opt-in) rather than
    /// on.  `getEffectiveConfig` (`resolved_map`) and `is_enabled` must
    /// report these as disabled until an editor sets them, so the
    /// reported state matches the inlay handler's default-off gate
    /// (otherwise an exported `config.ini` would claim the hints are on
    /// and re-importing it would enable them).
    pub(in super::super) const DEFAULT_OFF: &'static [&'static str] = &[
        "inlayTypeHints",
        "inlayParameterHints",
        "xcDiagnostics",
        "crossFileResolution",
    ];

    /// The default fallback for `feature` when no editor has set it:
    /// `false` for the opt-in [`Self::DEFAULT_OFF`] keys, `true`
    /// otherwise.
    fn default_enabled(feature: &str) -> bool {
        !Self::DEFAULT_OFF.contains(&feature)
    }

    /// Whether `feature` is enabled — explicitly-set value, else the
    /// per-feature default ([`Self::default_enabled`]).
    pub(in super::super) fn is_enabled(&self, feature: &str) -> bool {
        self.set
            .get(feature)
            .copied()
            .unwrap_or_else(|| Self::default_enabled(feature))
    }

    /// Like [`Self::is_enabled`] but with a default-**off** fallback, for
    /// opt-in features (e.g. `willSaveWaitUntil`) that stay disabled
    /// unless an editor turns them on.
    pub(in super::super) fn is_enabled_default_off(&self, feature: &str) -> bool {
        self.set.get(feature).copied().unwrap_or(false)
    }

    /// Merge an editor-supplied `features` object, setting only the
    /// keys it carries (absent keys keep their last-applied value).
    ///
    /// The legacy `inlayHints` key is a backward-compatible alias for
    /// `inlayTypeHints`: an explicit opt-in through it enables the
    /// inferred-variable-type hints, while the verbose parameter-name hints
    /// stay off.  Applied first so an
    /// explicit new `inlayTypeHints` in the same object always wins when a
    /// config carries both.
    pub(in super::super) fn apply(
        &mut self,
        features: &serde_json::Map<String, serde_json::Value>,
    ) {
        if let Some(flag) = features
            .get("inlayHints")
            .and_then(serde_json::Value::as_bool)
        {
            self.set.insert("inlayTypeHints".to_owned(), flag);
        }
        for (key, value) in features {
            if key == "inlayHints" {
                continue;
            }
            if let Some(flag) = value.as_bool() {
                self.set.insert(key.clone(), flag);
            }
        }
    }

    /// Record a single feature flag explicitly — for toggles that live in a
    /// dedicated config *section* (e.g. `tclLsp.xcDiagnostics.enabled`)
    /// rather than the flat `features` object the editor sends to
    /// [`Self::apply`].
    pub(super) fn set_flag(&mut self, feature: &str, value: bool) {
        self.set.insert(feature.to_owned(), value);
    }

    /// `self` with every key `overlay` sets explicitly taking precedence —
    /// the folder-over-global resolution
    /// [`Backend::resolved_feature_toggles`] performs.  A key the overlay does
    /// not carry keeps this set's value, so a folder that mentions one feature
    /// does not silently reset the rest to their defaults.
    pub(in super::super) fn overlaid_with(&self, overlay: &Self) -> Self {
        let mut merged = self.clone();
        for (key, &flag) in &overlay.set {
            merged.set.insert(key.clone(), flag);
        }
        merged
    }

    /// The full resolved `{feature: bool}` map for `getEffectiveConfig`.
    pub(in super::super) fn resolved_map(&self) -> serde_json::Map<String, serde_json::Value> {
        Self::KEYS
            .iter()
            .map(|&k| (k.to_owned(), serde_json::Value::Bool(self.is_enabled(k))))
            .collect()
    }
}

impl Backend {
    /// The feature toggles in effect for `uri` — the process-global set with
    /// the deepest containing folder's explicitly-set keys layered on top.
    ///
    /// Every per-URI toggle question resolves through this one place, so what
    /// `getEffectiveConfig` *reports* for a document cannot drift from what the
    /// providers *do* for it.  Were the command to report the global map while
    /// the gates below consulted the folder chain first, a client's
    /// toggle-took-effect barrier would observe a different fact from the one
    /// the provider uses.
    ///
    /// `uri` is optional because the command also answers for no document at
    /// all, which resolves to the global set alone.
    pub(in super::super) async fn resolved_feature_toggles(
        &self,
        uri: Option<&Uri>,
    ) -> FeatureToggles {
        let global = self.feature_toggles.lock().await.clone();
        let Some(uri) = uri else { return global };
        let configs = self.folder_configs.lock().await;
        match longest_folder_match(&configs, uri) {
            Some(fc) => global.overlaid_with(&fc.feature_toggles),
            None => global,
        }
    }

    /// Whether the named `tclLsp.features.*` provider is enabled.
    pub(in super::super) async fn feature_enabled(&self, feature: &str, uri: &Uri) -> bool {
        self.resolved_feature_toggles(Some(uri))
            .await
            .is_enabled(feature)
    }

    /// Whether opt-in format-on-save (`tclLsp.features.willSaveWaitUntil`)
    /// is enabled for the document at `uri`.  Resolves per folder like
    /// [`Self::feature_enabled`] so a folder-scoped opt-in works in a
    /// multi-root workspace, but — unlike the default-on feature toggles — it
    /// defaults **off**, so it cannot reuse
    /// `feature_enabled` (whose absent-key fallback is `true`).
    pub(in super::super) async fn will_save_format_enabled(&self, uri: &Uri) -> bool {
        self.resolved_feature_toggles(Some(uri))
            .await
            .is_enabled_default_off("willSaveWaitUntil")
    }

    /// Whether the given opt-in inlay family is enabled for `uri`.
    ///
    /// Inlay hints are opt-in and default **off**.  The two families —
    /// `inlayTypeHints` (inferred variable types + format-string specifier
    /// labels) and `inlayParameterHints` (call-site parameter-name labels)
    /// — gate independently; the retired `inlayHints` key is normalised to
    /// `inlayTypeHints` on input (see [`FeatureToggles::apply`]).  Resolves
    /// per folder like [`Self::feature_enabled`] so a folder-scoped opt-in
    /// works in a multi-root workspace.
    pub(in super::super) async fn inlay_family_enabled(&self, uri: &Uri, family: &str) -> bool {
        self.resolved_feature_toggles(Some(uri))
            .await
            .is_enabled_default_off(family)
    }

    /// Whether the opt-in XC100-301 translatability diagnostics
    /// (`xcDiagnostics`) are enabled for `uri`.  Default **off**, resolved
    /// per folder like [`Self::inlay_family_enabled`].  Surfaced only on
    /// `f5-irules` documents (the only dialect the `f5-xc` translator runs
    /// on) — see [`Self::cross_file_resolution_enabled`] for the separate,
    /// dialect-general cross-file toggle.
    pub(in super::super) async fn xc_diagnostics_enabled(&self, uri: &Uri) -> bool {
        self.inlay_family_enabled(uri, "xcDiagnostics").await
    }

    /// Whether opt-in cross-file resolution (cross-file W120/W123
    /// suppression + cross-file E002/E003 arity) is enabled for `uri`.
    /// Default **off** (the underlying salsa `project` query walks every
    /// file in the workspace, so it's opt-in for perf), resolved per
    /// folder like [`Self::inlay_family_enabled`]. Applies to *every*
    /// dialect — deliberately independent of [`Self::xc_diagnostics_enabled`],
    /// which gates only the unrelated, f5-irules-specific XC100-301
    /// translatability lints.
    pub(in super::super) async fn cross_file_resolution_enabled(&self, uri: &Uri) -> bool {
        self.inlay_family_enabled(uri, "crossFileResolution").await
    }
}
