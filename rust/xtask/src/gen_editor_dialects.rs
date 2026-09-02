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

//! Generate editor-facing selectable dialect lists from
//! [`tcl_dialect::DialectProfile::all`].
//!
//! A dialect is selectable exactly when it has a canonical profile.  There is
//! deliberately no "CLI-only" or editor-local filter: the server accepts every
//! catalog profile in `tclLsp.dialect`, including `bpf`, `f5-tmsh`, and
//! `f5-bigip`.  The presentation labels below are deliberately exhaustive so
//! adding a profile makes this generator fail until its human-facing name is
//! chosen, rather than silently dropping the new selectable value.
//!
//! Run `cargo xtask gen-editor-dialects`; `--check` makes the committed VS
//! Code, `JetBrains`, and Sublime projections a drift gate (issue #1394).

use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use tcl_dialect::DialectProfile;

use crate::util::repo_root;

const VSCODE_PACKAGE: &str = "editors/vscode/package.json";
const VSCODE_RUNTIME: &str = "editors/vscode/src/extension.ts";
const VSCODE_EXPLORER: &str = "editors/vscode/src/compilerExplorerHtml.ts";
const JETBRAINS_SETTINGS: &str =
    "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettings.kt";
const SUBLIME_PLUGIN: &str = "editors/sublime-text/plugin.py";
const SUBLIME_README: &str = "editors/sublime-text/README.md";
const SUBLIME_PACKAGE: &str = "editors/sublime-text/sublime-package.json";

#[derive(Clone, Copy)]
struct EditorDialect {
    name: &'static str,
    label: &'static str,
    short_label: &'static str,
}

/// The profile catalogue owns membership, canonical IDs, *and* the
/// human-facing spellings: `display_name` for menus, `short_name` for tight
/// UI like the compiler-explorer dropdown.  Both are mandatory struct
/// fields, so adding a profile can't ship without its labels.
fn editor_dialect(profile: &DialectProfile) -> EditorDialect {
    EditorDialect {
        name: profile.name,
        label: profile.display_name,
        short_label: profile.short_name,
    }
}

fn dialects() -> Vec<EditorDialect> {
    DialectProfile::all().iter().map(editor_dialect).collect()
}

fn replace_marked_block(text: &str, begin: &str, end: &str, body: &str) -> Result<String> {
    let start = text
        .find(begin)
        .with_context(|| format!("missing {begin:?}"))?;
    let body_start = text[start..]
        .find('\n')
        .map(|n| start + n + 1)
        .ok_or_else(|| anyhow!("{begin:?} must end in a newline"))?;
    let end_tag_start = text[body_start..]
        .find(end)
        .map(|n| body_start + n)
        .with_context(|| format!("missing {end:?} after {begin:?}"))?;
    // Preserve the end marker's indentation. `find(end)` starts at the marker
    // itself, not its line's leading spaces; replacing from there would leave
    // a generated marker at column zero in an otherwise indented Kotlin/Python
    // block.
    let end_line_start = text[..end_tag_start].rfind('\n').map_or(0, |n| n + 1);
    Ok(format!(
        "{}{}{}",
        &text[..body_start],
        body,
        &text[end_line_start..]
    ))
}

fn render_vscode(original: &str, ds: &[EditorDialect]) -> Result<String> {
    let mut root: Value = serde_json::from_str(original).context("parsing VS Code package.json")?;
    let configs = root["contributes"]["configuration"]
        .as_array_mut()
        .context("contributes.configuration must be an array")?;
    let general = configs
        .iter_mut()
        .find(|section| section["title"] == "General")
        .context("General configuration section missing")?;
    let dialect = general["properties"]["tclLsp.dialect"]
        .as_object_mut()
        .context("tclLsp.dialect schema missing")?;
    dialect.insert(
        "enum".to_owned(),
        Value::Array(
            ds.iter()
                .map(|d| Value::String(d.name.to_owned()))
                .collect(),
        ),
    );
    dialect.insert(
        "enumDescriptions".to_owned(),
        Value::Array(
            ds.iter()
                .map(|d| Value::String(format!("{} dialect", d.label)))
                .collect(),
        ),
    );

    let ai = configs
        .iter_mut()
        .find(|section| section["title"] == "AI")
        .context("AI configuration section missing")?;
    let enum_values = std::iter::once(Value::String("*".to_owned()))
        .chain(ds.iter().map(|d| Value::String(d.name.to_owned())))
        .collect();
    ai["properties"]["tclLsp.ai.extraPrompts"]["items"]["properties"]["dialects"]["items"]["enum"] =
        Value::Array(enum_values);

    let mut rendered =
        serde_json::to_string_pretty(&root).context("serialising VS Code package.json")?;
    rendered.push('\n');
    Ok(rendered)
}

fn render_vscode_runtime(original: &str, ds: &[EditorDialect]) -> Result<String> {
    let mut rows = String::new();
    for d in ds {
        // Match Prettier's TypeScript object-key style: simple identifiers are
        // bare, while dialect IDs containing punctuation remain quoted.
        let key = if d.name.chars().enumerate().all(|(i, c)| {
            c == '_' || c.is_ascii_alphanumeric() && (i > 0 || c.is_ascii_alphabetic())
        }) {
            d.name.to_owned()
        } else {
            format!("\"{}\"", d.name)
        };
        let _ = writeln!(rows, "  {key}: \"{}\",", d.label);
    }
    let body = format!("const DIALECT_LABELS: Record<string, string> = {{\n{rows}}};\n");
    replace_marked_block(
        original,
        "// @generated:dialect-labels:begin",
        "// @generated:dialect-labels:end",
        &body,
    )
}

fn render_jetbrains(original: &str, ds: &[EditorDialect]) -> Result<String> {
    let mut rows = String::new();
    for d in ds {
        let _ = writeln!(rows, "            \"{}\" to \"{}\",", d.name, d.label);
    }
    let body = format!("        val DIALECT_OPTIONS = listOf(\n{rows}        )\n");
    replace_marked_block(
        original,
        "// @generated:dialect-options:begin",
        "// @generated:dialect-options:end",
        &body,
    )
}

fn render_sublime_plugin(original: &str, ds: &[EditorDialect]) -> Result<String> {
    let mut rows = String::new();
    for d in ds {
        let default = if d.name == "tcl8.6" { " (default)" } else { "" };
        let _ = writeln!(rows, "    (\"{}\", \"{}{}\"),", d.name, d.label, default);
    }
    replace_marked_block(
        original,
        "# @generated:dialects:begin",
        "# @generated:dialects:end",
        &rows,
    )
}

fn render_sublime_readme(original: &str, ds: &[EditorDialect]) -> Result<String> {
    let mut rows = String::new();
    for d in ds {
        let default = if d.name == "tcl8.6" { " (default)" } else { "" };
        let _ = writeln!(rows, "| `{}` | {}{} |", d.name, d.label, default);
    }
    replace_marked_block(
        original,
        "<!-- @generated:dialects:begin -->",
        "<!-- @generated:dialects:end -->",
        &rows,
    )
}

/// Replace one JSON string array without reserialising the whole Sublime
/// schema.  The schema intentionally keeps a few compact one-item arrays, so
/// a full `serde_json` round trip would create unrelated formatting drift.
fn render_sublime_package(original: &str, ds: &[EditorDialect]) -> Result<String> {
    let dialect_key = "\"dialect\": {";
    let dialect_start = original
        .find(dialect_key)
        .with_context(|| format!("missing {dialect_key:?}"))?;
    let key = "\"enum\": [";
    let key_start = dialect_start
        + original[dialect_start..]
            .find(key)
            .with_context(|| format!("missing {key:?} after {dialect_key:?}"))?;
    let values_start = key_start + key.len();
    let end = original[values_start..]
        .find(']')
        .map(|offset| values_start + offset)
        .with_context(|| format!("missing closing array for {key:?}"))?;
    let key_line_start = original[..key_start].rfind('\n').map_or(0, |n| n + 1);
    let key_indent = &original[key_line_start..key_start];
    let item_indent = format!("{key_indent}    ");
    let mut values = String::new();
    for (index, d) in ds.iter().enumerate() {
        let encoded = serde_json::to_string(d.name).context("serialising dialect name")?;
        let comma = if index + 1 == ds.len() { "" } else { "," };
        let _ = writeln!(values, "{item_indent}{encoded}{comma}");
    }
    let replacement = format!("\n{values}{key_indent}]");
    Ok(format!(
        "{}{replacement}{}",
        &original[..values_start],
        &original[end + 1..]
    ))
}

/// The compiler-explorer toolbar dropdown: every catalog profile, labelled
/// with the profile's compact `short_name` (the toolbar has no room for the
/// full display names), `tcl8.6` pre-selected as the explorer's default.
fn render_compiler_explorer(original: &str, ds: &[EditorDialect]) -> Result<String> {
    let mut rows = String::new();
    for d in ds {
        let selected = if d.name == "tcl8.6" { " selected" } else { "" };
        let _ = writeln!(
            rows,
            "      <option value=\"{}\"{selected}>{}</option>",
            d.name, d.short_label
        );
    }
    replace_marked_block(
        original,
        "<!-- @generated:dialect-options:begin",
        "<!-- @generated:dialect-options:end -->",
        &rows,
    )
}

type Render = fn(&str, &[EditorDialect]) -> Result<String>;

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let ds = dialects();
    if ds.len() != DialectProfile::all().len() {
        bail!("editor dialect projection lost a DialectProfile catalog entry");
    }
    eprintln!("  {} selectable dialect profiles", ds.len());
    let targets: &[(&str, Render)] = &[
        (VSCODE_PACKAGE, render_vscode),
        (VSCODE_RUNTIME, render_vscode_runtime),
        (VSCODE_EXPLORER, render_compiler_explorer),
        (JETBRAINS_SETTINGS, render_jetbrains),
        (SUBLIME_PLUGIN, render_sublime_plugin),
        (SUBLIME_README, render_sublime_readme),
        (SUBLIME_PACKAGE, render_sublime_package),
    ];
    let mut drift = Vec::new();
    for &(rel, render) in targets {
        let path = root.join(rel);
        let original =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let rendered = render(&original, &ds).with_context(|| format!("rendering {rel}"))?;
        if check {
            if original != rendered {
                drift.push(rel);
            }
        } else if original != rendered {
            fs::write(&path, rendered).with_context(|| format!("writing {}", path.display()))?;
            eprintln!("wrote {rel}");
        }
    }
    if check && !drift.is_empty() {
        eprintln!(
            "{} editor dialect projection(s) are stale — run `cargo xtask gen-editor-dialects`:",
            drift.len()
        );
        for target in drift {
            eprintln!("  - {target}");
        }
        return Ok(ExitCode::from(1));
    }
    if check {
        eprintln!("OK: editor dialect projections match DialectProfile::all().");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_profile_has_one_editor_projection() {
        let ds = dialects();
        assert_eq!(ds.len(), DialectProfile::all().len());
        assert!(ds.iter().any(|d| d.name == "bpf"));
        assert!(ds.iter().any(|d| d.name == "spectcl"));
        assert!(ds.iter().any(|d| d.name == "sslictcl"));
    }

    #[test]
    fn committed_files_match_generated_dialect_projections() {
        let root = repo_root();
        let ds = dialects();
        for (rel, render) in [
            (VSCODE_PACKAGE, render_vscode as Render),
            (VSCODE_RUNTIME, render_vscode_runtime),
            (VSCODE_EXPLORER, render_compiler_explorer),
            (JETBRAINS_SETTINGS, render_jetbrains),
            (SUBLIME_PLUGIN, render_sublime_plugin),
            (SUBLIME_README, render_sublime_readme),
            (SUBLIME_PACKAGE, render_sublime_package),
        ] {
            let path = root.join(rel);
            let original = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            assert_eq!(render(&original, &ds).unwrap(), original, "{rel} is stale");
        }
    }

    #[test]
    fn sublime_settings_schema_is_valid_and_covers_catalogue() {
        let root = repo_root();
        let text = fs::read_to_string(root.join(SUBLIME_PACKAGE)).unwrap();
        let json: Value = serde_json::from_str(&text).expect("Sublime package schema is JSON");
        let values = &json["contributions"]["settings"][0]["schema"]["properties"]["settings"]["properties"]
            ["tclLsp"]["properties"]["dialect"]["enum"];
        assert_eq!(
            values.as_array().unwrap().len(),
            DialectProfile::all().len()
        );
        assert!(values.as_array().unwrap().iter().any(|v| v == "bpf"));
        assert!(values.as_array().unwrap().iter().any(|v| v == "spectcl"));
        assert!(values.as_array().unwrap().iter().any(|v| v == "sslictcl"));
    }

    #[test]
    fn catalog_membership_mutation_drifts_every_generated_surface() {
        let root = repo_root();
        let full = dialects();
        let mut removed = full.clone();
        removed.pop();
        let mut added = full.clone();
        added.push(EditorDialect {
            name: "future-dialect",
            label: "Future Dialect",
            short_label: "Future",
        });
        for (rel, render) in [
            (VSCODE_PACKAGE, render_vscode as Render),
            (VSCODE_RUNTIME, render_vscode_runtime),
            (VSCODE_EXPLORER, render_compiler_explorer),
            (JETBRAINS_SETTINGS, render_jetbrains),
            (SUBLIME_PLUGIN, render_sublime_plugin),
            (SUBLIME_README, render_sublime_readme),
            (SUBLIME_PACKAGE, render_sublime_package),
        ] {
            let original = fs::read_to_string(root.join(rel)).unwrap();
            assert_ne!(
                render(&original, &removed).unwrap(),
                original,
                "removing a profile must drift {rel}"
            );
            assert_ne!(
                render(&original, &added).unwrap(),
                original,
                "adding a profile must drift {rel}"
            );
        }
    }
}
