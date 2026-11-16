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

//! Readers of individual settings and payload shapes shared by the global and per-folder parses.

use std::path::PathBuf;

use tcl_compiler::analyser::NonAsciiMode;
use tcl_lsp_core::config_ini;

use crate::vfs;

/// Lift the analyser's diagnostic records into the LSP wire
/// shape.  Shared by both the push-based `publish_diagnostics`
/// path (via `publish_analyser_diagnostics`) and the pull-
/// based `textDocument/diagnostic` handler.
/// Resolve a byte `span` to an LSP `Range` through `line_index`.
/// Shared by every `lift_*_diagnostics` helper so the offset →
/// (line, character) mapping is identical for analyser, compiler-
/// check, and optimiser diagnostics.
/// The startup optimisation profile: the `TCL_LSP_OPTIMISER_PROFILE` env
/// override (used by the fp/ground-truth battery harness to request the
/// `full` profile), else the editor default
/// ([`tcl_compiler::optimiser::profiles::DEFAULT_EDITOR_PROFILE`]).
/// A `tclLsp.optimiser.profile` config pull overrides it at runtime.
pub(in super::super) fn default_optimiser_profile()
-> tcl_compiler::optimiser::profiles::OptimisationProfile {
    std::env::var("TCL_LSP_OPTIMISER_PROFILE").map_or(
        tcl_compiler::optimiser::profiles::DEFAULT_EDITOR_PROFILE,
        |s| tcl_compiler::optimiser::profiles::OptimisationProfile::parse(&s),
    )
}

/// Map a `tclLsp.style.nonAscii` setting string to a [`NonAsciiMode`].
/// An unknown / absent value resolves to [`NonAsciiMode::Default`] (the
/// per-dialect auto behaviour).
pub(in super::super) fn parse_non_ascii_mode(s: &str) -> NonAsciiMode {
    match s {
        "off" => NonAsciiMode::Off,
        "strict" => NonAsciiMode::Strict,
        "confusables" => NonAsciiMode::Confusables,
        "common" => NonAsciiMode::Common,
        _ => NonAsciiMode::Default,
    }
}

/// Read an INI config file at `path` (if present/readable) into the
/// editor-shape `tclLsp` settings JSON for its `layer`. Missing or unreadable
/// files yield an empty object, so a layer simply contributes nothing.
pub(in super::super) fn read_ini_layer(
    store: &dyn vfs::SourceStore,
    path: Option<PathBuf>,
    layer: config_ini::Layer,
) -> serde_json::Value {
    path.and_then(|p| store.read_to_string(&p).ok())
        .map_or_else(
            || serde_json::json!({}),
            |content| config_ini::settings_from_ini(&content, layer),
        )
}

/// Normalise a pulled configuration payload into the unwrapped `tclLsp`-content
/// object [`Backend::apply_global_config`] expects, accepting three shapes:
///
/// 1. **Nested** — `{ "tclLsp": { "optimiser": { "O109": false } } }`.
/// 2. **Flat dotted** — `{ "tclLsp.optimiser.O109": false }` (or without the
///    `tclLsp.` prefix), folded into nested objects.
/// 3. **Unwrapped** — `{ "optimiser": { "O109": false }, "dialect": "tcl8.6" }`
///    (e.g. a `JetBrains` pull-model response with no `tclLsp` prefix).
///
/// The shapes compose: a nested `tclLsp` object is merged with any flat dotted
/// keys and any unwrapped top-level sections.
/// Collapse the retired `features.inlayHints` boolean into `inlayTypeHints`
/// in-place, for one settings layer. `inlayHints` is the backward-compatible
/// alias for `inlayTypeHints`; an explicit `inlayTypeHints` in the *same* layer
/// still wins (`or_insert` keeps it). Done per-layer before [`merge_settings`]
/// so cross-layer precedence stays correct — see the call site in
/// [`Backend::pull_and_apply_config`].
pub(in super::super) fn collapse_inlay_alias(settings: &mut serde_json::Value) {
    if let Some(features) = settings
        .get_mut("features")
        .and_then(serde_json::Value::as_object_mut)
        && let Some(alias) = features.remove("inlayHints")
    {
        features.entry("inlayTypeHints").or_insert(alias);
    }
}

pub(in super::super) fn normalize_config_payload(payload: &serde_json::Value) -> serde_json::Value {
    use serde_json::{Map, Value};
    let mut out = Map::new();
    let Some(obj) = payload.as_object() else {
        return Value::Object(out);
    };
    // 1. A nested `tclLsp` object contributes its keys directly.
    if let Some(nested) = obj.get("tclLsp").and_then(Value::as_object) {
        for (k, v) in nested {
            out.insert(k.clone(), v.clone());
        }
    }
    for (key, value) in obj {
        if key == "tclLsp" {
            continue;
        }
        // 2. Flat dotted keys (`tclLsp.a.b` or `a.b`) → nested objects.
        let path = key.strip_prefix("tclLsp.").unwrap_or(key);
        if path.contains('.') {
            let segments: Vec<&str> = path.split('.').collect();
            let mut cursor = &mut out;
            for seg in &segments[..segments.len() - 1] {
                let slot = cursor
                    .entry((*seg).to_owned())
                    .or_insert_with(|| Value::Object(Map::new()));
                // A colliding non-object at this segment — a mixed-shape payload
                // that supplied both a scalar/object prefix and a dotted child
                // (e.g. `tclLsp.features: false` alongside
                // `tclLsp.features.semanticTokens: true`) — is replaced by an
                // object so the deeper key can be inserted (last-writer-wins),
                // rather than panicking on the descent.
                if !slot.is_object() {
                    *slot = Value::Object(Map::new());
                }
                cursor = slot
                    .as_object_mut()
                    .expect("slot was just ensured to be an object");
            }
            cursor.insert(segments[segments.len() - 1].to_owned(), value.clone());
        } else if key.starts_with("tclLsp.") {
            // `tclLsp.dialect` with no further nesting → top-level key.
            out.insert(path.to_owned(), value.clone());
        } else {
            // 3. Unwrapped top-level section/scalar.
            out.insert(key.clone(), value.clone());
        }
    }
    Value::Object(out)
}

/// Extract `tclLsp.style.nonAscii` from an LSP settings payload, accepting
/// both the nested (`{"tclLsp":{"style":{"nonAscii":"…"}}}`) and the
/// flat-dotted (`{"tclLsp.style.nonAscii":"…"}`) shapes editors send.
pub(in super::super) fn settings_non_ascii_mode(
    settings: &serde_json::Value,
) -> Option<NonAsciiMode> {
    let nested = settings
        .get("tclLsp")
        .and_then(|v| v.get("style"))
        .and_then(|v| v.get("nonAscii"));
    nested
        .or_else(|| settings.get("tclLsp.style.nonAscii"))
        .and_then(serde_json::Value::as_str)
        .map(parse_non_ascii_mode)
}

/// Extract `tclLsp.notifications.environmentKind` from a settings payload,
/// accepting the nested (`{"tclLsp":{"notifications":{"environmentKind":false}}}`),
/// the unwrapped (`{"notifications":{"environmentKind":false}}`) and the
/// flat-dotted (`{"tclLsp.notifications.environmentKind":false}`) shapes.
/// `None` when the payload does not carry it, so the current value stands.
pub(in super::super) fn settings_environment_kind_enabled(
    settings: &serde_json::Value,
) -> Option<bool> {
    let nested = settings
        .get("tclLsp")
        .unwrap_or(settings)
        .get("notifications")
        .and_then(|v| v.get("environmentKind"));
    nested
        .or_else(|| settings.get("tclLsp.notifications.environmentKind"))
        .and_then(serde_json::Value::as_bool)
}

/// The editor's Workspace Trust state a client message states, when it
/// states one: a top-level `workspaceTrust` of `"trusted"` or `"untrusted"`,
/// in `initializationOptions` or in a `didChangeConfiguration` push.
///
/// Top-level only, and never read from a `tclLsp` section: VS Code's
/// configuration sync pushes the whole resolved `tclLsp` section, and a pull
/// answers it, both from every settings layer — the workspace's own
/// `.vscode/settings.json` among them — so a nested key would let an
/// untrusted workspace declare itself trusted. A client extension writes the
/// top-level key itself, from the editor's own state. Anything else is no
/// statement: the state stays what it was, trusted when nothing was ever
/// said.
pub(in super::super) fn workspace_trust_in(
    payload: &serde_json::Value,
) -> Option<tcl_dialect::model::WorkspaceTrust> {
    match payload.get("workspaceTrust")?.as_str()? {
        "trusted" => Some(tcl_dialect::model::WorkspaceTrust::Trusted),
        "untrusted" => Some(tcl_dialect::model::WorkspaceTrust::Untrusted),
        _ => None,
    }
}

/// The package names one `tclLsp.packages.provides` entry declares, accepting
/// either a JSON array (`"myExtension": ["Tk", "Img"]`) or a single string
/// (`"myExtension": "Tk"`) — the same one-or-many latitude the `.tcl-lsp.ini`
/// `[packages.provides]` comma list has. Blank names are dropped.
/// How one configuration layer answers `tclLsp.bigipVersion`.
///
/// Three states, not two: an *absent* key inherits the layer below, while an
/// explicit empty string clears the pin back to the oldest-supported default.
/// Collapsing them would leave a folder unable to say "no pin here" distinctly
/// from "whatever the workspace says".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in super::super) enum BigipVersionSetting {
    /// Not stated at this layer.
    #[default]
    Absent,
    /// Stated. `None` is the explicit empty string.
    Present(Option<String>),
}

/// Read `tclLsp.bigipVersion` from one configuration layer.
pub(in super::super) fn parse_bigip_version(cfg: &serde_json::Value) -> BigipVersionSetting {
    let Some(version) = cfg.get("bigipVersion").and_then(serde_json::Value::as_str) else {
        return BigipVersionSetting::Absent;
    };
    let version = version.trim();
    BigipVersionSetting::Present((!version.is_empty()).then(|| version.to_owned()))
}

/// `tclLsp.targets` as `(provider, range)` pairs, e.g. `("tcl", "8.5-9.0")`.
///
/// Shared by the global and per-folder parses. Sorted for the same reason as
/// [`parse_package_provides`]: it feeds a salsa input where a map-iteration
/// order change must not look like an edit.
pub(super) fn parse_declared_targets(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(String, String)> {
    let mut targets: Vec<(String, String)> = map
        .iter()
        .filter_map(|(name, range)| {
            let range = range.as_str()?.trim();
            (!range.is_empty()).then(|| (name.clone(), range.to_owned()))
        })
        .collect();
    targets.sort();
    targets
}

/// `tclLsp.packages.provides` / `[packages.provides]` as edges.
///
/// Shared by the global and per-folder parses so the two cannot disagree
/// about normalisation or ordering. Sorted because it feeds a salsa input,
/// where a map-iteration-order change must not look like an edit.
pub(super) fn parse_package_provides(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(String, Vec<String>)> {
    let mut provides: Vec<(String, Vec<String>)> = map
        .iter()
        .map(|(package, loaded)| (package.clone(), provided_package_names(loaded)))
        .filter(|(_, loaded)| !loaded.is_empty())
        .collect();
    provides.sort();
    provides
}

fn provided_package_names(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(one) => one
            .split([',', ' '])
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        serde_json::Value::Array(many) => many
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}
