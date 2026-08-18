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

//! Generate the editors' registered file-extension and language lists from
//! their two sources of truth: the [`tcl_dialect::DialectProfile`] catalog
//! (each profile's `display_name` / `editor_language_id` /
//! `file_extensions`) and the bundled `SpecTcl` packs' `file_extension` rows
//! (`specs/*.tclspec`).  The core Tcl source extensions come from
//! [`tcl_registry::dialects::TCL_SOURCE_EXTENSIONS`], minus the ones a
//! dialect owns.
//!
//! Projections:
//! - VS Code `package.json` `contributes.languages` (one language per
//!   profile with an `editor_language_id`, carrying its extensions) and
//!   `contributes.grammars` (a `source.tcl` grammar row for any language
//!   that lacks one).
//! - VS Code `src/languageIds.ts` `TCL_LANGUAGE_IDS` and
//!   `src/extension.ts` `LANGUAGE_ID_DIALECTS` (marked blocks).
//! - `JetBrains` `plugin.xml`: the `Tcl` and `iRule` fileType
//!   `extensions="…"` attributes.
//! - Sublime `Tcl.sublime-syntax` `file_extensions` and Zed
//!   `config.toml` `path_suffixes` (single-syntax editors get the full
//!   union).
//!
//! Run `cargo xtask gen-editor-extensions`; `--check` makes the committed
//! projections a drift gate.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use tcl_dialect::DialectProfile;

use crate::util::repo_root;

const VSCODE_PACKAGE: &str = "editors/vscode/package.json";
const VSCODE_RUNTIME: &str = "editors/vscode/src/extension.ts";
const VSCODE_LANGUAGE_IDS: &str = "editors/vscode/src/languageIds.ts";
const JETBRAINS_PLUGIN: &str = "editors/jetbrains/src/main/resources/META-INF/plugin.xml";
const JETBRAINS_FILETYPE: &str =
    "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/TclFileType.kt";
const SUBLIME_SYNTAX: &str = "editors/sublime-text/Tcl.sublime-syntax";
const ZED_CONFIG: &str = "editors/zed/languages/tcl/config.toml";

/// `.apl` (the iApp presentation language) has an editor language of its own
/// (`tcl-apl`, hand-maintained: it is an iApp *sublanguage*, not a dialect
/// profile), so the generated `tcl` language must not also claim it.
const HAND_MAINTAINED_EXTENSIONS: &[&str] = &["apl"];

/// Language entries the generator preserves verbatim rather than rebuilding
/// from a profile: sublanguages with no catalog profile behind them.
const HAND_MAINTAINED_LANGUAGES: &[&str] = &["tcl-apl"];

/// Everything the editors register for one language id.
struct Language {
    id: String,
    /// Menu labels, most human first (`["F5 iRules", "irule"]`).
    aliases: Vec<String>,
    /// Lower-case extensions without dots.
    extensions: Vec<String>,
    /// The canonical dialect the language pins, if any (`None` for plain
    /// `tcl`, whose dialect is detected).
    dialect: Option<String>,
}

/// The assembled model: every language the editors register, in stable
/// order — `tcl` first, then the catalog profiles in catalog order.
fn languages() -> Result<Vec<Language>> {
    let root = repo_root();
    let set = tcl_spectcl::bundled::load_from(&root.join("specs"));

    // extension → owning language id, catalog first (its invariant tests
    // guarantee one owner per extension), packs second.
    let language_for_profile = |name: &str| -> Option<&'static str> {
        DialectProfile::all()
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.editor_language_id)
    };

    let mut langs: Vec<Language> = Vec::new();
    langs.push(Language {
        id: "tcl".to_owned(),
        aliases: vec!["Tcl".to_owned(), "tcl".to_owned()],
        extensions: Vec::new(),
        dialect: None,
    });
    for profile in DialectProfile::all() {
        let Some(id) = profile.editor_language_id else {
            continue;
        };
        // The compact menu alias VS Code already used (`synopsys`,
        // `irule`): the language id minus its `tcl-` prefix, when that
        // differs from the display name's own spelling.
        let mut aliases = vec![profile.display_name.to_owned()];
        if let Some(short) = id.strip_prefix("tcl-") {
            aliases.push(short.to_owned());
        }
        langs.push(Language {
            id: id.to_owned(),
            aliases,
            extensions: profile
                .file_extensions
                .iter()
                .map(|row| row.extension.to_owned())
                .collect(),
            dialect: Some(profile.name.to_owned()),
        });
    }

    // Pack-declared extensions land on the language of the dialect their
    // row routes to; rows with no `-dialect`, or whose dialect has no
    // dedicated language, ride plain `tcl`.
    let owned: Vec<String> = langs.iter().flat_map(|l| l.extensions.clone()).collect();
    for pack in &set.packs {
        for row in &pack.file_extensions {
            if owned.contains(&row.extension) {
                continue;
            }
            let target = row
                .dialect
                .and_then(language_for_profile)
                .unwrap_or("tcl")
                .to_owned();
            let lang = langs
                .iter_mut()
                .find(|l| l.id == target)
                .ok_or_else(|| anyhow!("pack {}: no language {target}", pack.name))?;
            if !lang.extensions.contains(&row.extension) {
                lang.extensions.push(row.extension.clone());
            }
        }
    }

    // The core Tcl source extensions that no dialect or pack owns are the
    // plain-`tcl` language's registration list.
    let owned: Vec<String> = langs.iter().flat_map(|l| l.extensions.clone()).collect();
    for ext in tcl_registry::dialects::TCL_SOURCE_EXTENSIONS {
        if owned.iter().any(|o| o == ext) || HAND_MAINTAINED_EXTENSIONS.contains(ext) {
            continue;
        }
        langs[0].extensions.push((*ext).to_owned());
    }

    Ok(langs)
}

/// Rebuild `contributes.languages` and `contributes.grammars`: generated
/// entries from the model, hand-maintained entries (`tcl-apl`) preserved
/// verbatim in their original positions at the tail.
fn render_vscode_package(original: &str, langs: &[Language]) -> Result<String> {
    let mut root: Value = serde_json::from_str(original).context("parsing VS Code package.json")?;

    let existing = root["contributes"]["languages"]
        .as_array()
        .context("contributes.languages must be an array")?
        .clone();
    let configuration = existing
        .first()
        .and_then(|l| l["configuration"].as_str())
        .unwrap_or("./language-configuration.json")
        .to_owned();

    let mut out: Vec<Value> = Vec::new();
    for lang in langs {
        let mut entry = serde_json::Map::new();
        entry.insert("id".to_owned(), Value::String(lang.id.clone()));
        entry.insert(
            "aliases".to_owned(),
            Value::Array(lang.aliases.iter().cloned().map(Value::String).collect()),
        );
        if !lang.extensions.is_empty() {
            entry.insert(
                "extensions".to_owned(),
                Value::Array(
                    lang.extensions
                        .iter()
                        .map(|e| Value::String(format!(".{e}")))
                        .collect(),
                ),
            );
        }
        entry.insert(
            "configuration".to_owned(),
            Value::String(configuration.clone()),
        );
        out.push(Value::Object(entry));
    }
    for entry in &existing {
        let id = entry["id"].as_str().unwrap_or_default();
        if HAND_MAINTAINED_LANGUAGES.contains(&id) {
            out.push(entry.clone());
        } else if !langs.iter().any(|l| l.id == id) {
            bail!(
                "contributes.languages entry {id:?} is neither generated from the \
                 dialect catalog nor listed in HAND_MAINTAINED_LANGUAGES — add it \
                 to a profile (editor_language_id) or to the hand-maintained list"
            );
        }
    }
    root["contributes"]["languages"] = Value::Array(out);

    // Grammars: keep every existing row (some languages carry their own
    // scope — `source.tcl-apl`, `source.tcl-bigip`); add a `source.tcl` row
    // for any language that has none.
    let grammars = root["contributes"]["grammars"]
        .as_array()
        .context("contributes.grammars must be an array")?
        .clone();
    let tcl_grammar = grammars
        .iter()
        .find(|g| g["language"] == "tcl")
        .context("grammar for language `tcl` missing")?
        .clone();
    let mut out = grammars;
    let all_ids: Vec<&str> = langs
        .iter()
        .map(|l| l.id.as_str())
        .chain(HAND_MAINTAINED_LANGUAGES.iter().copied())
        .collect();
    for id in &all_ids {
        if !out.iter().any(|g| g["language"] == *id) {
            let mut row = tcl_grammar.clone();
            row["language"] = Value::String((*id).to_owned());
            out.push(row);
        }
    }
    root["contributes"]["grammars"] = Value::Array(out);

    // Every registered language needs the per-language editor defaults the
    // existing languages carry — sticky scroll follows the LSP folding
    // provider, not the outline (`[tcl]` set the pattern). Adding a
    // language without this block regresses it to outlineModel, which the
    // extension's stickyScroll suite pins per language id.
    let defaults = root["contributes"]["configurationDefaults"]
        .as_object_mut()
        .context("contributes.configurationDefaults must be an object")?;
    for id in &all_ids {
        let key = format!("[{id}]");
        let entry = defaults
            .entry(key)
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if entry.get("editor.stickyScroll.defaultModel").is_none() {
            entry["editor.stickyScroll.defaultModel"] =
                Value::String("foldingProviderModel".into());
        }
    }

    let mut rendered =
        serde_json::to_string_pretty(&root).context("serialising VS Code package.json")?;
    rendered.push('\n');
    Ok(rendered)
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
    let end_line_start = text[..end_tag_start].rfind('\n').map_or(0, |n| n + 1);
    Ok(format!(
        "{}{}{}",
        &text[..body_start],
        body,
        &text[end_line_start..]
    ))
}

fn render_language_ids(original: &str, langs: &[Language]) -> Result<String> {
    let mut rows = String::new();
    for lang in langs {
        let _ = writeln!(rows, "  \"{}\",", lang.id);
    }
    for id in HAND_MAINTAINED_LANGUAGES {
        let _ = writeln!(rows, "  \"{id}\",");
    }
    let body = format!("export const TCL_LANGUAGE_IDS = new Set([\n{rows}]);\n");
    replace_marked_block(
        original,
        "// @generated:language-ids:begin",
        "// @generated:language-ids:end",
        &body,
    )
}

fn render_vscode_runtime(original: &str, langs: &[Language]) -> Result<String> {
    let mut rows = String::new();
    for lang in langs {
        let Some(dialect) = &lang.dialect else {
            continue;
        };
        // Prettier key style: ids with punctuation stay quoted.
        let key = if lang.id.chars().all(|c| c.is_ascii_alphanumeric()) {
            lang.id.clone()
        } else {
            format!("\"{}\"", lang.id)
        };
        let _ = writeln!(rows, "  {key}: \"{dialect}\",");
    }
    // `tcl-apl` is the APL (iApp presentation language) editor id — an iApp
    // sublanguage, so it analyses as `f5-iapps`.
    let _ = writeln!(rows, "  \"tcl-apl\": \"f5-iapps\",");
    let body = format!("const LANGUAGE_ID_DIALECTS: Record<string, string> = {{\n{rows}}};\n");
    replace_marked_block(
        original,
        "// @generated:language-id-dialects:begin",
        "// @generated:language-id-dialects:end",
        &body,
    )
}

/// Rewrite one `<fileType name="…" …extensions="…"/>` element's extensions
/// attribute, leaving the rest of the element untouched.
fn set_jetbrains_filetype_extensions(text: &str, name: &str, extensions: &str) -> Result<String> {
    let tag = format!("<fileType name=\"{name}\"");
    let start = text
        .find(&tag)
        .with_context(|| format!("missing {tag:?}"))?;
    let end = text[start..]
        .find("/>")
        .map(|n| start + n)
        .with_context(|| format!("unterminated {tag:?}"))?;
    let attr = "extensions=\"";
    let attr_start = text[start..end]
        .find(attr)
        .map(|n| start + n + attr.len())
        .with_context(|| format!("{tag:?} has no extensions attribute"))?;
    let attr_end = text[attr_start..]
        .find('"')
        .map(|n| attr_start + n)
        .context("unterminated extensions attribute")?;
    Ok(format!(
        "{}{}{}",
        &text[..attr_start],
        extensions,
        &text[attr_end..]
    ))
}

fn render_jetbrains(original: &str, langs: &[Language]) -> Result<String> {
    // JetBrains keeps two fileTypes: `iRule` (its own icon/type) and `Tcl`
    // (everything else — JetBrains routes dialects server-side).
    let irule: Vec<String> = langs
        .iter()
        .filter(|l| l.dialect.as_deref() == Some("f5-irules"))
        .flat_map(|l| l.extensions.clone())
        .collect();
    let mut main: Vec<String> = langs
        .iter()
        .filter(|l| l.dialect.as_deref() != Some("f5-irules"))
        .flat_map(|l| l.extensions.clone())
        .collect();
    main.extend(HAND_MAINTAINED_EXTENSIONS.iter().map(|e| (*e).to_owned()));
    let text = set_jetbrains_filetype_extensions(original, "Tcl", &main.join(";"))?;
    set_jetbrains_filetype_extensions(&text, "iRule", &irule.join(";"))
}

/// Every registered extension, for the single-syntax editors.
fn all_extensions(langs: &[Language]) -> Vec<String> {
    let mut out: Vec<String> = langs.iter().flat_map(|l| l.extensions.clone()).collect();
    out.extend(HAND_MAINTAINED_EXTENSIONS.iter().map(|e| (*e).to_owned()));
    out
}

/// The `JetBrains` plugin's Kotlin-side recognition gate mirrors the union
/// registered on its fileTypes; without this the plugin.xml registration
/// and `TclFileType.isSupported` drift apart.
fn render_jetbrains_kotlin(original: &str, langs: &[Language]) -> Result<String> {
    let mut rows = String::new();
    for ext in all_extensions(langs) {
        let _ = writeln!(rows, "            \"{ext}\",");
    }
    let body = format!("        private val SUPPORTED_EXTENSIONS = setOf(\n{rows}        )\n");
    replace_marked_block(
        original,
        "// @generated:supported-extensions:begin",
        "// @generated:supported-extensions:end",
        &body,
    )
}

fn render_sublime(original: &str, langs: &[Language]) -> Result<String> {
    let start = original
        .find("file_extensions:\n")
        .context("missing file_extensions block")?;
    let list_start = start + "file_extensions:\n".len();
    let list_end = original[list_start..]
        .find("\n\n")
        .map(|n| list_start + n + 1)
        .context("file_extensions block must end with a blank line")?;
    let mut rows = String::new();
    for ext in all_extensions(langs) {
        let _ = writeln!(rows, "  - {ext}");
    }
    Ok(format!(
        "{}{}{}",
        &original[..list_start],
        rows,
        &original[list_end..]
    ))
}

fn render_zed(original: &str, langs: &[Language]) -> Result<String> {
    let start = original
        .find("path_suffixes = [")
        .context("missing path_suffixes")?;
    let end = original[start..]
        .find(']')
        .map(|n| start + n + 1)
        .context("unterminated path_suffixes")?;
    let quoted: Vec<String> = all_extensions(langs)
        .iter()
        .map(|e| format!("\"{e}\""))
        .collect();
    Ok(format!(
        "{}path_suffixes = [{}]{}",
        &original[..start],
        quoted.join(", "),
        &original[end..]
    ))
}

type Render = fn(&str, &[Language]) -> Result<String>;

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let langs = languages()?;

    let renders: Vec<(&str, Render)> = vec![
        (VSCODE_PACKAGE, render_vscode_package),
        (VSCODE_LANGUAGE_IDS, render_language_ids),
        (VSCODE_RUNTIME, render_vscode_runtime),
        (JETBRAINS_PLUGIN, render_jetbrains),
        (JETBRAINS_FILETYPE, render_jetbrains_kotlin),
        (SUBLIME_SYNTAX, render_sublime),
        (ZED_CONFIG, render_zed),
    ];

    let mut drifted: Vec<&str> = Vec::new();
    for (rel, render) in renders {
        let path = root.join(rel);
        let original =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let rendered = render(&original, &langs).with_context(|| format!("rendering {rel}"))?;
        if rendered == original {
            continue;
        }
        if check {
            drifted.push(rel);
        } else {
            fs::write(&path, rendered).with_context(|| format!("writing {}", path.display()))?;
            println!("gen-editor-extensions: wrote {rel}");
        }
    }

    // Belt and braces: the model itself must be one-owner-per-extension
    // (the catalog's invariant tests cover the profiles; packs could still
    // collide with each other here).
    let mut owners: BTreeMap<String, String> = BTreeMap::new();
    for lang in &langs {
        for ext in &lang.extensions {
            if let Some(prior) = owners.insert(ext.clone(), lang.id.clone()) {
                bail!(
                    "extension {ext:?} registered by both {prior:?} and {:?}",
                    lang.id
                );
            }
        }
    }

    if check && !drifted.is_empty() {
        for rel in &drifted {
            eprintln!("gen-editor-extensions: {rel} is out of sync");
        }
        bail!("run `cargo xtask gen-editor-extensions` and commit the result");
    }
    println!(
        "gen-editor-extensions: {} languages, {} extensions{}",
        langs.len() + HAND_MAINTAINED_LANGUAGES.len(),
        owners.len(),
        if check { " — in sync" } else { "" }
    );
    Ok(ExitCode::SUCCESS)
}
