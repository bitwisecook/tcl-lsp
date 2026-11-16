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

//! The setting values in force for one document: folder override, else the global value.

use super::setting_parsers::BigipVersionSetting;
use super::{
    Backend, FolderGenericPatterns, HashSet, NonAsciiMode, PolicyLayers, ResourceAnalyserInputs,
    Uri, core_formatting, longest_folder_match,
};

impl Backend {
    /// Resolve the analyser-affecting settings for `uri`: a per-folder editor
    /// config (longest-prefix match) overrides the process-global fields
    /// field-by-field; unset folder fields inherit the global value.  Returns
    /// `(disabled_diagnostics, non_ascii_mode)` — the analyser's
    /// production-time skip and its W108 mode.  In a single-root workspace
    /// (no per-folder configs) this is exactly the global state.
    ///
    /// The presentation decisions — which codes show, at what severity, under
    /// which optimiser profile — are not resolved here: they are the
    /// document's [`core_policy::Policy`], built from
    /// [`Self::resolved_policy_layers`].
    pub(in super::super) async fn resolved_analysis_settings(
        &self,
        uri: &Uri,
    ) -> (HashSet<String>, NonAsciiMode) {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).cloned()
        };
        let disabled = match folder.as_ref().and_then(|f| f.disabled_diagnostics.clone()) {
            Some(d) => d,
            None => self.disabled_diagnostics.lock().await.clone(),
        };
        let non_ascii_mode = match folder.as_ref().and_then(|f| f.non_ascii_mode) {
            Some(m) => m,
            None => *self.non_ascii_mode.lock().await,
        };
        (disabled, non_ascii_mode)
    }

    /// The configuration layers `uri`'s document resolves its policy from:
    /// the longest-matching workspace folder's own three layers when the
    /// folder has been configured, else the session's.
    ///
    /// Longest-prefix matching over workspace folders is the LSP concept the
    /// other surfaces have no counterpart for, which is why this resolution
    /// lives here and the layering itself lives in
    /// [`core_policy::PolicyBuilder`].
    pub(in super::super) async fn resolved_policy_layers(&self, uri: &Uri) -> PolicyLayers {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).and_then(|f| f.policy_layers.clone())
        };
        match folder {
            Some(layers) => layers,
            None => self.policy_layers.lock().await.clone(),
        }
    }

    /// Resolve the formatter line length for `uri` (per-folder override, else
    /// the global `tclLsp.formatting.lineLength`).
    pub(in super::super) async fn resolved_line_length(&self, uri: &Uri) -> u32 {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).and_then(|f| f.line_length)
        };
        match folder {
            Some(len) => len,
            None => *self.line_length.lock().await,
        }
    }

    /// The resolved `tclLsp.extraCommands` for `uri`: a folder override wins,
    /// else the process-global set.
    pub(in super::super) async fn resolved_extra_commands(&self, uri: &Uri) -> Vec<String> {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).and_then(|f| f.extra_commands.clone())
        };
        match folder {
            Some(v) => v,
            None => self.extra_commands.lock().await.clone(),
        }
    }

    /// Every `"scope": "resource"` analyser input, resolved for `uri` — a
    /// folder override wins over the process-global value, field by field.
    ///
    /// `None` for `uri` answers with the global values alone, for the paths
    /// that analyse without a document in hand (the workspace scan, which
    /// reaches files across every folder, and the disk scanner's batch).
    /// The session's declared script encoding (`tclLsp.sourceEncoding`).
    pub(in super::super) fn declared_script_encoding(
        &self,
    ) -> tcl_registry::value_transfer::ScriptEncoding {
        *self
            .script_encoding
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(in super::super) async fn resource_analyser_inputs(
        &self,
        uri: Option<&Uri>,
    ) -> ResourceAnalyserInputs {
        let folder = match uri {
            Some(uri) => {
                let configs = self.folder_configs.lock().await;
                longest_folder_match(&configs, uri).cloned()
            }
            None => None,
        };
        ResourceAnalyserInputs {
            package_provides: match folder.as_ref().and_then(|f| f.package_provides.clone()) {
                Some(v) => v,
                None => self.package_provides.lock().await.clone(),
            },
            bigip_version: match folder.as_ref().map(|f| f.bigip_version.clone()) {
                Some(BigipVersionSetting::Present(v)) => v,
                _ => self.bigip_version.lock().await.clone(),
            },
            declared_targets: match folder.as_ref().and_then(|f| f.declared_targets.clone()) {
                Some(v) => v,
                None => self.declared_targets.lock().await.clone(),
            },
            script_encoding: self.declared_script_encoding(),
        }
    }

    /// The resolved built-in signature-help exclusion list for `uri`: a
    /// folder override wins, else the process-global list.
    pub(in super::super) async fn resolved_signature_help_disabled_commands(
        &self,
        uri: &Uri,
    ) -> Vec<String> {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri)
                .and_then(|f| f.signature_help_disabled_commands.clone())
        };
        match folder {
            Some(v) => v,
            None => self.signature_help_disabled_commands.lock().await.clone(),
        }
    }

    pub(in super::super) async fn signature_disabled_for(&self, uri: Option<&Uri>) -> Vec<String> {
        match uri {
            Some(uri) => self.resolved_signature_help_disabled_commands(uri).await,
            None => self.signature_help_disabled_commands.lock().await.clone(),
        }
    }

    /// The resolved `tclLsp.diagnostics.genericVariablePatterns` for `uri`: a
    /// folder override wins, else the process-global value.
    pub(in super::super) async fn resolved_generic_variable_patterns(
        &self,
        uri: &Uri,
    ) -> Option<Vec<String>> {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).map(|f| f.generic_variable_patterns.clone())
        };
        match folder {
            // `Replace`/`BuiltinDefaults` are the folder's own override; only
            // `Inherit` (or no matching folder) falls back to the global value.
            Some(FolderGenericPatterns::Replace(list)) => Some(list),
            Some(FolderGenericPatterns::BuiltinDefaults) => None,
            Some(FolderGenericPatterns::Inherit) | None => {
                self.generic_variable_patterns.lock().await.clone()
            }
        }
    }

    /// The resolved `tclLsp.formatting` settings object for `uri`: a folder
    /// override wins, else the process-global section (or `Null`).
    pub(in super::super) async fn resolved_formatting(&self, uri: &Uri) -> serde_json::Value {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).and_then(|f| f.formatting.clone())
        };
        match folder {
            Some(v) => v,
            None => self.formatting_settings.lock().await.clone(),
        }
    }

    /// The resolved `tclLsp.formatting.docstringStyle` setting for `uri`
    /// — a folder override wins, else the process-global value,
    /// same precedence as [`Self::resolved_formatting`] (whose object this
    /// reads the key from). Falls back to
    /// [`core_formatting::DocstringStyle::None`] — the documented default —
    /// when the key is absent or not a string.
    pub(in super::super) async fn resolved_docstring_style(
        &self,
        uri: &Uri,
    ) -> core_formatting::DocstringStyle {
        self.resolved_formatting(uri)
            .await
            .get("docstringStyle")
            .and_then(serde_json::Value::as_str)
            .map_or(core_formatting::DocstringStyle::None, |s| {
                core_formatting::DocstringStyle::parse(s)
            })
    }

    /// The line ending any server-composed edit for `uri` should insert: the
    /// resolved `tclLsp.formatting.lineEnding`, whose `auto` default means
    /// "whatever `source` already uses" ([`core_formatting::detect_line_ending`]).
    ///
    /// Pass the document's **raw** text ([`DocumentState::raw`]) — the sniff
    /// has to see the real terminators, not their analysis form.
    pub(in super::super) async fn resolved_edit_line_ending(
        &self,
        uri: &Uri,
        source: &str,
    ) -> String {
        let configured = self
            .resolved_formatting(uri)
            .await
            .get("lineEnding")
            .and_then(serde_json::Value::as_str)
            .map(str::to_ascii_lowercase);
        match configured.as_deref() {
            Some("crlf") => "\r\n".to_owned(),
            Some("cr") => "\r".to_owned(),
            Some("lf") => "\n".to_owned(),
            // `auto` (the default) and anything unrecognised: follow the file.
            _ => core_formatting::detect_line_ending(source).to_owned(),
        }
    }

    /// The resolved W111 source-style line length (`tclLsp.style.lineLength`)
    /// for `uri`: a folder override wins, else the process-global value.
    /// Distinct from [`Self::resolved_line_length`] (the formatter width).
    pub(in super::super) async fn resolved_style_line_length(&self, uri: &Uri) -> u32 {
        let folder = {
            let configs = self.folder_configs.lock().await;
            longest_folder_match(&configs, uri).and_then(|f| f.style_line_length)
        };
        match folder {
            Some(len) => len,
            None => *self.style_line_length.lock().await,
        }
    }
}
