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

//! Source files that are test code by declaration.
//!
//! A module declared `#[cfg(test)] mod name;` lives in its own file, and so
//! does every module that file declares. The gates treat such a file as the
//! test code of the file that declares it, exactly as they treat an inline
//! `#[cfg(test)] mod name { ... }`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::OnceLock;

/// Child file to the file that declares it, for every test-only module file.
fn declared() -> &'static BTreeMap<String, String> {
    static DECLARED: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    DECLARED.get_or_init(|| scan(&super::repo_root()))
}

/// The file that declares `rel` as a test-only module, if one does.
pub(crate) fn test_module_parent(rel: &str) -> Option<&'static str> {
    declared().get(rel).map(String::as_str)
}

/// Whether `rel` is a test-only module file.
pub(crate) fn is_test_module_file(rel: &str) -> bool {
    declared().contains_key(rel)
}

/// The production file `rel` belongs to: itself, or the file whose test-only
/// modules lead to it.
pub(crate) fn owning_source(rel: &str) -> &str {
    let mut current = rel;
    while let Some(parent) = test_module_parent(current) {
        current = parent;
    }
    current
}

/// The test-only module files declared, directly or not, by `rel`.
pub(crate) fn test_module_files_of(rel: &str) -> Vec<&'static str> {
    declared()
        .keys()
        .map(String::as_str)
        .filter(|child| *child != rel && owning_source(child) == rel)
        .collect()
}

fn scan(root: &Path) -> BTreeMap<String, String> {
    let mut sources = Vec::new();
    for tree in ["rust", "runtime/rust"] {
        collect(root, tree, &mut sources);
    }
    let mut declared = BTreeMap::new();
    let mut pending: Vec<(String, bool)> = sources.into_iter().map(|rel| (rel, false)).collect();
    while let Some((rel, test_only)) = pending.pop() {
        let Ok(text) = std::fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        for name in module_declarations(&text, test_only) {
            let Some(child) = module_file(root, &rel, &name) else {
                continue;
            };
            if declared.insert(child.clone(), rel.clone()).is_none() {
                pending.push((child, true));
            }
        }
    }
    declared
}

fn collect(root: &Path, relative: &str, found: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(root.join(relative)) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let child = format!("{relative}/{name}");
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            if name != "target" && !name.starts_with('.') {
                collect(root, &child, found);
            }
        } else if Path::new(&name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        {
            found.push(child);
        }
    }
}

/// Names of `mod name;` declarations: those under a test gate
/// (`#[cfg(test)]` or `#[cfg(feature = "test-support")]`), or every
/// one when the declaring file is itself test-only.
fn module_declarations(text: &str, every: bool) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut names = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some(name) = declared_module(line) else {
            continue;
        };
        let guarded = lines[..index]
            .iter()
            .rev()
            .take_while(|above| above.starts_with("#[") || above.starts_with("///"))
            .any(|above| super::is_test_gate(above) || above.starts_with("#[cfg(all(test,"));
        if every || guarded {
            names.push(name.to_owned());
        }
    }
    names
}

fn declared_module(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("pub ").unwrap_or(line);
    let rest = if rest.starts_with("pub(") {
        rest.split_once(") ")?.1
    } else {
        rest
    };
    let name = rest.strip_prefix("mod ")?.strip_suffix(';')?;
    name.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
        .then_some(name)
}

pub(crate) fn module_file(root: &Path, parent: &str, name: &str) -> Option<String> {
    let (directory, file) = parent.rsplit_once('/')?;
    let stem = file.strip_suffix(".rs")?;
    let nested = format!("{directory}/{stem}");
    let bases = if matches!(file, "mod.rs" | "lib.rs" | "main.rs") {
        [directory.to_owned(), nested]
    } else {
        [nested, directory.to_owned()]
    };
    bases
        .iter()
        .flat_map(|base| [format!("{base}/{name}.rs"), format!("{base}/{name}/mod.rs")])
        .find(|candidate| root.join(candidate).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declarations_under_cfg_test_are_found_and_plain_ones_are_not() {
        let text = "mod plain;\n#[cfg(test)]\nmod tests;\n#[cfg(test)]\n#[allow(dead_code)]\npub(crate) mod fixtures;\n";
        assert_eq!(module_declarations(text, false), ["tests", "fixtures"]);
        assert_eq!(
            module_declarations(text, true),
            ["plain", "tests", "fixtures"]
        );
        assert_eq!(declared_module("mod inline {"), None);
        let narrowed = "#[cfg(all(test, not(target_family = \"wasm\")))]\nmod controls;\n";
        assert_eq!(module_declarations(narrowed, false), ["controls"]);
    }

    #[test]
    fn a_hoisted_test_module_belongs_to_its_parent() {
        let child = "rust/xtask/src/util/test_sources/fixture_only.rs";
        assert!(!is_test_module_file(child));
        assert_eq!(owning_source(child), child);
        let parent = "rust/tcl-lsp-core/src/semantic_tokens.rs";
        let tests = "rust/tcl-lsp-core/src/semantic_tokens/tests.rs";
        assert_eq!(test_module_parent(tests), Some(parent));
        assert!(test_module_files_of(parent).contains(&tests));
    }
}
