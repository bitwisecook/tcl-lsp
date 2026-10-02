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

//! Generate editor-facing selectable dialect lists from the compiled
//! environment registry ([`EnvironmentRegistry::compiled_selectable`]).
//!
//! A dialect is selectable exactly when the registry lists it as a choice:
//! there is no editor-local filter, so `bpf`, `f5-tmsh`, `jim` and `tk` appear
//! wherever the server accepts them in `tclLsp.dialect`. Order is the
//! registry's: languages first, then Tcl releases with packages, canonical id
//! ascending within each group. A label is the environment's `display_name`
//! (`short_name` where the toolbar is tight) and a description is
//! [`EnvironmentDefinition::description`].
//!
//! The starting dialect every manifest and settings file names is
//! [`DEFAULT_ENVIRONMENT_ID`], the constant the server's own default is meant
//! to follow.
//!
//! Run `cargo xtask gen-editor-dialects`; `--check` makes the committed VS
//! Code, `JetBrains`, and Sublime projections a drift gate.

use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Map, Value};
use tcl_dialect::model::{DEFAULT_ENVIRONMENT_ID, EnvironmentDefinition, EnvironmentRegistry};

use crate::util::{replace_marked_block, repo_root};

const VSCODE_PACKAGE: &str = "editors/vscode/package.json";
const VSCODE_EXPLORER: &str = "editors/vscode/src/compilerExplorerHtml.ts";
const JETBRAINS_SETTINGS: &str =
    "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/settings/TclLspSettings.kt";
const SUBLIME_PACKAGE: &str = "editors/sublime-text/sublime-package.json";
const SUBLIME_SETTINGS: &str = "editors/sublime-text/LSP-Tcl.sublime-settings";

type Environment = Arc<EnvironmentDefinition>;

fn dialects() -> &'static [Environment] {
    EnvironmentRegistry::compiled_selectable()
}

fn names(ds: &[Environment]) -> Vec<Value> {
    ds.iter()
        .map(|d| Value::String(d.id.as_str().to_owned()))
        .collect()
}

fn descriptions(ds: &[Environment]) -> Vec<Value> {
    ds.iter().map(|d| Value::String(d.description())).collect()
}

fn labels(ds: &[Environment]) -> Vec<Value> {
    ds.iter()
        .map(|d| Value::String(d.display_name.to_string()))
        .collect()
}

/// Rebuild the `tclLsp.dialect` schema object with its enumeration keys
/// (`enum`, `enumItemLabels`, `enumDescriptions`) in that order at the place
/// `enum` already sat, and its default set.
fn set_dialect_schema(schema: &mut Map<String, Value>, ds: &[Environment]) -> Result<()> {
    if !schema.contains_key("enum") {
        bail!("tclLsp.dialect schema has no enum to regenerate");
    }
    let mut rebuilt = Map::new();
    for (key, value) in std::mem::take(schema) {
        match key.as_str() {
            "enumItemLabels" | "enumDescriptions" => {}
            "enum" => {
                rebuilt.insert(key, Value::Array(names(ds)));
                rebuilt.insert("enumItemLabels".to_owned(), Value::Array(labels(ds)));
                rebuilt.insert(
                    "enumDescriptions".to_owned(),
                    Value::Array(descriptions(ds)),
                );
            }
            "default" => {
                rebuilt.insert(key, Value::String(DEFAULT_ENVIRONMENT_ID.to_owned()));
            }
            _ => {
                rebuilt.insert(key, value);
            }
        }
    }
    *schema = rebuilt;
    Ok(())
}

fn render_vscode(original: &str, ds: &[Environment]) -> Result<String> {
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
    set_dialect_schema(dialect, ds)?;

    let ai = configs
        .iter_mut()
        .find(|section| section["title"] == "AI")
        .context("AI configuration section missing")?;
    let enum_values = std::iter::once(Value::String("*".to_owned()))
        .chain(names(ds))
        .collect();
    ai["properties"]["tclLsp.ai.extraPrompts"]["items"]["properties"]["dialects"]["items"]["enum"] =
        Value::Array(enum_values);

    let mut rendered =
        serde_json::to_string_pretty(&root).context("serialising VS Code package.json")?;
    rendered.push('\n');
    Ok(rendered)
}

fn render_jetbrains(original: &str, ds: &[Environment]) -> Result<String> {
    let mut rows = String::new();
    for d in ds {
        let _ = writeln!(
            rows,
            "            \"{}\" to \"{}\",",
            d.id.as_str(),
            d.display_name
        );
    }
    let body = format!(
        "        const val DEFAULT_DIALECT = \"{DEFAULT_ENVIRONMENT_ID}\"\n        val DIALECT_OPTIONS = listOf(\n{rows}        )\n"
    );
    replace_marked_block(
        original,
        "// @generated:dialect-options:begin",
        "// @generated:dialect-options:end",
        &body,
    )
}

/// The byte range of the array value of `key`, brackets included, searching
/// `text[from..]`. The scan is string-aware, so a bracket inside a value does
/// not end the array.
fn array_range(text: &str, from: usize, key: &str) -> Option<(usize, usize)> {
    let needle = format!("\"{key}\": [");
    let open = from + text[from..].find(&needle)? + needle.len() - 1;
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, c) in text[open..].char_indices() {
        if in_string {
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some((open, open + offset + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// `[` … `]` for `values`, one per line, items indented one level (four
/// spaces) past `key_indent`.
fn render_array(values: &[Value], key_indent: &str) -> Result<String> {
    let item_indent = format!("{key_indent}    ");
    let mut out = String::from("[\n");
    for (index, value) in values.iter().enumerate() {
        let encoded = serde_json::to_string(value).context("serialising an array item")?;
        let comma = if index + 1 == values.len() { "" } else { "," };
        let _ = writeln!(out, "{item_indent}{encoded}{comma}");
    }
    let _ = write!(out, "{key_indent}]");
    Ok(out)
}

/// The indentation of the line `key` sits on.
fn indent_of(text: &str, at: usize) -> &str {
    let line_start = text[..at].rfind('\n').map_or(0, |n| n + 1);
    &text[line_start..at]
}

/// Replace one JSON string array — or insert it right after the array of
/// `insert_after` when the key is not there yet — without reserialising the
/// whole Sublime schema. The schema intentionally keeps a few compact
/// one-item arrays, so a full `serde_json` round trip would create unrelated
/// formatting drift.
fn set_string_array(
    text: &str,
    from: usize,
    key: &str,
    values: &[Value],
    insert_after: &str,
) -> Result<String> {
    if let Some((start, end)) = array_range(text, from, key) {
        let key_at = start - format!("\"{key}\": ").len();
        let rendered = render_array(values, indent_of(text, key_at))?;
        return Ok(format!("{}{rendered}{}", &text[..start], &text[end..]));
    }
    let (anchor_start, anchor_end) = array_range(text, from, insert_after)
        .with_context(|| format!("missing {insert_after:?} array after the dialect key"))?;
    let indent = indent_of(text, anchor_start - format!("\"{insert_after}\": ").len()).to_owned();
    let rendered = render_array(values, &indent)?;
    Ok(format!(
        "{},\n{indent}\"{key}\": {rendered}{}",
        &text[..anchor_end],
        &text[anchor_end..]
    ))
}

/// Replace the string value of the first `"key": "…"` at or after `from`.
fn set_string_value(text: &str, from: usize, key: &str, value: &str) -> Result<String> {
    let needle = format!("\"{key}\": \"");
    let start = from
        + text[from..]
            .find(&needle)
            .with_context(|| format!("missing {needle:?}"))?
        + needle.len();
    let end = start
        + text[start..]
            .find('"')
            .with_context(|| format!("unterminated value for {key:?}"))?;
    Ok(format!("{}{value}{}", &text[..start], &text[end..]))
}

fn render_sublime_package(original: &str, ds: &[Environment]) -> Result<String> {
    let dialect_key = "\"dialect\": {";
    let dialect_start = original
        .find(dialect_key)
        .with_context(|| format!("missing {dialect_key:?}"))?;
    let text = set_string_value(original, dialect_start, "default", DEFAULT_ENVIRONMENT_ID)?;
    let text = set_string_array(&text, dialect_start, "enum", &names(ds), "enum")?;
    set_string_array(
        &text,
        dialect_start,
        "enumDescriptions",
        &descriptions(ds),
        "enum",
    )
}

/// The `LSP-Tcl` settings file's starting dialect; its `dialect` line is the
/// only key here the registry decides.
fn render_sublime_settings(original: &str, _ds: &[Environment]) -> Result<String> {
    set_string_value(original, 0, "dialect", DEFAULT_ENVIRONMENT_ID)
}

/// The compiler-explorer toolbar dropdown: every selectable environment,
/// labelled with its compact `short_name` (the toolbar has no room for the
/// full display names), the default dialect pre-selected.
fn render_compiler_explorer(original: &str, ds: &[Environment]) -> Result<String> {
    let mut rows = String::new();
    for d in ds {
        let selected = if d.id.as_str() == DEFAULT_ENVIRONMENT_ID {
            " selected"
        } else {
            ""
        };
        let _ = writeln!(
            rows,
            "      <option value=\"{}\"{selected}>{}</option>",
            d.id.as_str(),
            d.short_name
        );
    }
    replace_marked_block(
        original,
        "<!-- @generated:dialect-options:begin",
        "<!-- @generated:dialect-options:end -->",
        &rows,
    )
}

type Render = fn(&str, &[Environment]) -> Result<String>;

/// Every target whose content follows the selectable set's membership.
const MEMBERSHIP_TARGETS: &[(&str, Render)] = &[
    (VSCODE_PACKAGE, render_vscode),
    (VSCODE_EXPLORER, render_compiler_explorer),
    (JETBRAINS_SETTINGS, render_jetbrains),
    (SUBLIME_PACKAGE, render_sublime_package),
];

/// Every generated target.
fn targets() -> Vec<(&'static str, Render)> {
    let mut all = MEMBERSHIP_TARGETS.to_vec();
    all.push((SUBLIME_SETTINGS, render_sublime_settings));
    all
}

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let ds = dialects();
    if !ds.iter().any(|d| d.id.as_str() == DEFAULT_ENVIRONMENT_ID) {
        return Err(anyhow!(
            "the default environment {DEFAULT_ENVIRONMENT_ID:?} is not selectable"
        ));
    }
    eprintln!("  {} selectable environments", ds.len());
    let mut drift = Vec::new();
    for (rel, render) in targets() {
        let path = root.join(rel);
        let original =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let rendered = render(&original, ds).with_context(|| format!("rendering {rel}"))?;
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
        eprintln!("OK: editor dialect projections match the selectable environments.");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_dialect::model::{EnvironmentId, EnvironmentKind};

    fn committed(rel: &str) -> String {
        fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("reading {rel}: {e}"))
    }

    #[test]
    fn every_selectable_environment_reaches_the_editors() {
        let ds = dialects();
        for id in ["bpf", "jim", "tk", "spectcl", "sslictcl", "f5-tmsh"] {
            assert!(ds.iter().any(|d| d.id.as_str() == id), "{id}");
        }
        assert!(
            ds.iter().all(|d| d.id.as_str() != "tcl"),
            "the lenient sink is a fallback, not a choice"
        );
    }

    #[test]
    fn committed_files_match_generated_dialect_projections() {
        let ds = dialects();
        for (rel, render) in targets() {
            let original = committed(rel);
            assert_eq!(render(&original, ds).unwrap(), original, "{rel} is stale");
        }
    }

    #[test]
    fn the_vscode_dialect_enum_is_labelled_and_described_from_the_registry() {
        let manifest: Value = serde_json::from_str(&committed(VSCODE_PACKAGE)).unwrap();
        let general = manifest["contributes"]["configuration"]
            .as_array()
            .unwrap()
            .iter()
            .find(|section| section["title"] == "General")
            .unwrap();
        let schema = &general["properties"]["tclLsp.dialect"];
        let ds = dialects();
        assert_eq!(schema["enum"], Value::Array(names(ds)));
        assert_eq!(schema["enumItemLabels"], Value::Array(labels(ds)));
        assert_eq!(schema["enumDescriptions"], Value::Array(descriptions(ds)));
        assert_eq!(schema["default"], DEFAULT_ENVIRONMENT_ID);
        let kinds: Vec<EnvironmentKind> = ds.iter().map(|d| d.kind).collect();
        assert!(
            kinds.windows(2).all(|pair| pair[0] <= pair[1]),
            "languages are listed before tool shells"
        );
    }

    #[test]
    fn a_tool_shell_reads_as_a_tcl_release_with_packages() {
        let vivado = dialects()
            .iter()
            .find(|d| d.id.as_str() == "xilinx-eda-tcl")
            .unwrap();
        assert_eq!(
            vivado.description(),
            "Xilinx Vivado — Tcl 8.5 + vivado, sdc, upf"
        );
    }

    #[test]
    fn sublime_settings_schema_is_valid_and_covers_the_registry() {
        let json: Value = serde_json::from_str(&committed(SUBLIME_PACKAGE))
            .expect("Sublime package schema is JSON");
        let dialect = &json["contributions"]["settings"][0]["schema"]["properties"]["settings"]["properties"]
            ["tclLsp"]["properties"]["dialect"];
        let ds = dialects();
        assert_eq!(dialect["enum"], Value::Array(names(ds)));
        assert_eq!(dialect["enumDescriptions"], Value::Array(descriptions(ds)));
        assert_eq!(dialect["default"], DEFAULT_ENVIRONMENT_ID);
    }

    #[test]
    fn every_default_names_the_default_environment() {
        let ds = dialects();
        let settings = committed(SUBLIME_SETTINGS);
        assert!(settings.contains(&format!("\"dialect\": \"{DEFAULT_ENVIRONMENT_ID}\"")));
        let stale = settings.replace(
            &format!("\"dialect\": \"{DEFAULT_ENVIRONMENT_ID}\""),
            "\"dialect\": \"tcl9.0\"",
        );
        assert_eq!(
            render_sublime_settings(&stale, ds).unwrap(),
            settings,
            "the render restores the default"
        );
        assert!(committed(JETBRAINS_SETTINGS).contains(&format!(
            "const val DEFAULT_DIALECT = \"{DEFAULT_ENVIRONMENT_ID}\""
        )));
    }

    #[test]
    fn membership_mutation_drifts_every_generated_surface() {
        let full = dialects().to_vec();
        let mut removed = full.clone();
        removed.pop();
        let mut added = full;
        let mut future = (*added[0]).clone();
        future.id = EnvironmentId::new("future-dialect");
        added.push(Arc::new(future));
        for &(rel, render) in MEMBERSHIP_TARGETS {
            let original = committed(rel);
            assert_ne!(
                render(&original, &removed).unwrap(),
                original,
                "removing an environment must drift {rel}"
            );
            assert_ne!(
                render(&original, &added).unwrap(),
                original,
                "adding an environment must drift {rel}"
            );
        }
    }

    #[test]
    fn the_sublime_enum_render_survives_a_missing_description_array() {
        let original = committed(SUBLIME_PACKAGE);
        let (start, end) = array_range(&original, 0, "enumDescriptions").unwrap();
        let key_at = start - "\"enumDescriptions\": ".len();
        let line_start = original[..key_at].rfind('\n').unwrap();
        let broken = format!("{}{}", &original[..line_start - 1], &original[end..]);
        assert!(!broken.contains("enumDescriptions"));
        assert_eq!(
            render_sublime_package(&broken, dialects()).unwrap(),
            original
        );
    }
}
