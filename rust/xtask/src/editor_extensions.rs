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

//! Drift gate: every file extension the bundled packs declare
//! (`file_extension upf -dialect …` in `specs/*.tclspec`) must be registered
//! in each shipped editor integration, so a pack's language actually opens
//! out of the box. The packs are the source of truth; the manifests are
//! hand-maintained and this gate is what keeps them honest — the same split
//! `docs/design/spec-dsl-examples/README.md` documents for the statement.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};

/// The editor manifests that must carry every pack-declared extension, with
/// the spelling each uses.
struct Manifest {
    path: &'static str,
    /// Render the extension the way this manifest spells it.
    spell: fn(&str) -> String,
}

const MANIFESTS: &[Manifest] = &[
    Manifest {
        path: "editors/vscode/package.json",
        spell: |ext| format!("\".{ext}\""),
    },
    Manifest {
        path: "editors/jetbrains/src/main/resources/META-INF/plugin.xml",
        // The `extensions="tcl;tk;…"` attribute — matched as a `;`/quote
        // delimited token below rather than a plain substring.
        spell: |ext| ext.to_owned(),
    },
    Manifest {
        path: "editors/sublime-text/Tcl.sublime-syntax",
        spell: |ext| format!("- {ext}"),
    },
    Manifest {
        path: "editors/zed/languages/tcl/config.toml",
        spell: |ext| format!("\"{ext}\""),
    },
];

/// Whether the `JetBrains` `extensions="…"` attribute carries `ext` as a whole
/// `;`-separated token.
fn jetbrains_has(text: &str, ext: &str) -> bool {
    text.lines()
        .filter_map(|line| {
            let start = line.find("extensions=\"")? + "extensions=\"".len();
            let end = line[start..].find('"')? + start;
            Some(&line[start..end])
        })
        .any(|attr| attr.split(';').any(|token| token == ext))
}

pub fn run() -> Result<std::process::ExitCode> {
    let root = crate::util::repo_root();
    let specs = root.join("specs");
    let set = tcl_spectcl::bundled::load_from(&specs);

    let mut declared: Vec<(String, String)> = Vec::new();
    for pack in &set.packs {
        for row in &pack.file_extensions {
            declared.push((pack.name.clone(), row.extension.clone()));
        }
    }
    if declared.is_empty() {
        println!("editor-extensions: no pack declares a file_extension; nothing to check");
        return Ok(std::process::ExitCode::SUCCESS);
    }

    let mut missing: Vec<String> = Vec::new();
    for manifest in MANIFESTS {
        let path: PathBuf = root.join(manifest.path);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        for (pack, ext) in &declared {
            let present = if manifest.path.ends_with("plugin.xml") {
                jetbrains_has(&text, ext)
            } else {
                text.contains(&(manifest.spell)(ext))
            };
            if !present {
                missing.push(format!(
                    "{}: `.{ext}` (declared by pack `{pack}`) is not registered",
                    manifest.path
                ));
            }
        }
    }

    if missing.is_empty() {
        println!(
            "editor-extensions: {} declared extension(s) registered in all {} manifests",
            declared.len(),
            MANIFESTS.len()
        );
        return Ok(std::process::ExitCode::SUCCESS);
    }
    for line in &missing {
        eprintln!("editor-extensions: {line}");
    }
    bail!(
        "{} manifest entr{} missing — register the extension(s) above, or drop \
         the `file_extension` row from the pack",
        missing.len(),
        if missing.len() == 1 {
            "y is"
        } else {
            "ies are"
        }
    );
}
