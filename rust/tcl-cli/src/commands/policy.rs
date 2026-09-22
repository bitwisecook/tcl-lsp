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

//! The configuration layers a `tcl` verb resolves each input document's
//! diagnostic policy under (`docs/design/compiler/diagnostic-policy.md`
//! § Configuration): the user's global `config.ini`, the verb's own flags as
//! the invocation layer in the editor layer's slot, and the input file's own
//! project `.tcl-lsp.ini` — per document, not per process, so one run can
//! span two projects and resolve each under its own layer.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use tcl_lsp_core::config_ini;
use tcl_lsp_core::diagnostic_policy::{PolicyBuilder, PolicyLayer};

/// The layers one verb invocation resolves every document under.
pub struct ConfigLayers {
    global: Value,
    invocation: Value,
    /// Project layers by root, read once per root.
    projects: RefCell<HashMap<PathBuf, Value>>,
}

impl ConfigLayers {
    /// The global `config.ini` plus `invocation` — the verb's `--disable` /
    /// `--enable` (and `--profile`) flags as one settings layer, from
    /// [`invocation_layer`].
    #[must_use]
    pub fn new(invocation: Value) -> Self {
        Self {
            global: config_ini::global_layer(),
            invocation,
            projects: RefCell::new(HashMap::new()),
        }
    }

    /// A builder with the layers for the document at `path` applied, lowest
    /// first: the global file, the flags in the editor layer's slot, then
    /// the document's project file when it sits under one. A document with
    /// no path (`--source`, stdin) has no project layer at all; the
    /// process's working directory is never a substitute.
    #[must_use]
    pub fn builder_for(&self, path: Option<&Path>) -> PolicyBuilder {
        let mut builder = PolicyBuilder::new()
            .layer(PolicyLayer::Global, &self.global)
            .layer(PolicyLayer::Invocation, &self.invocation);
        if let Some(project) = path.and_then(|path| self.project_layer_for(path)) {
            builder = builder.layer(PolicyLayer::Project, &project);
        }
        builder
    }

    /// The project layer for the file at `path`, read once per root.
    fn project_layer_for(&self, path: &Path) -> Option<Value> {
        let root = config_ini::project_root_for(path)?;
        let mut projects = self.projects.borrow_mut();
        if let Some(layer) = projects.get(&root) {
            return Some(layer.clone());
        }
        let layer = config_ini::project_layer_at(&root)?;
        projects.insert(root, layer.clone());
        Some(layer)
    }
}

/// Whether every path resolves under one project layer (or none) — the
/// first half of the test `tcl opt` makes before folding several inputs into
/// one text.
#[must_use]
pub fn share_one_project<'a>(paths: impl IntoIterator<Item = Option<&'a Path>>) -> bool {
    let roots: HashSet<Option<PathBuf>> = paths
        .into_iter()
        .map(|path| path.and_then(config_ini::project_root_for))
        .collect();
    roots.len() <= 1
}

/// The `--disable` / `--enable` flags (comma-separated, upper-cased,
/// repeatable) as one settings layer over `section` — `diagnostics` for the
/// diagnostic verbs, `optimiser` for `tcl opt`. The flags keep their
/// tri-state meaning: a later `--enable` turns a code back on, which is what
/// makes `--enable W242` reach a default-off code.
#[must_use]
pub fn invocation_layer(disable: &[String], enable: &[String], section: &str) -> Value {
    let mut codes = Map::new();
    for (raws, on) in [(disable, false), (enable, true)] {
        for raw in raws {
            for code in raw.split(',') {
                let code = code.trim();
                if !code.is_empty() {
                    codes.insert(code.to_ascii_uppercase(), Value::Bool(on));
                }
            }
        }
    }
    let mut layer = Map::new();
    layer.insert(section.to_owned(), Value::Object(codes));
    Value::Object(layer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::compiler_checks::DiagCode;
    use tcl_lsp_core::diagnostic_policy::Reason;

    #[test]
    fn the_flags_are_one_layer_with_enable_winning_last() {
        let layer = invocation_layer(
            &["w100,W210".to_owned(), "s100".to_owned()],
            &["W210".to_owned(), "W242".to_owned()],
            "diagnostics",
        );
        assert_eq!(
            layer,
            serde_json::json!({ "diagnostics": {
                "W100": false, "W210": true, "S100": false, "W242": true
            } })
        );
        let policy = PolicyBuilder::new()
            .layer(PolicyLayer::Invocation, &layer)
            .build();
        assert_eq!(
            policy.code_reason(DiagCode::W100),
            Some(Reason::Disabled(PolicyLayer::Invocation))
        );
        assert_eq!(policy.code_reason(DiagCode::W210), None);
        assert_eq!(
            policy.code_reason(DiagCode::W242),
            None,
            "a flag reaches a default-off code"
        );
    }

    #[test]
    fn a_pathless_document_has_no_project_layer() {
        let layers = ConfigLayers {
            global: serde_json::json!({}),
            invocation: serde_json::json!({}),
            projects: RefCell::new(HashMap::new()),
        };
        let policy = layers.builder_for(None).build();
        assert_eq!(policy.code_reason(DiagCode::W242), Some(Reason::DefaultOff));
        assert!(share_one_project([None, None]));
    }
}
