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
//!   profile with an `editor_language_id`, carrying its extensions and its
//!   whole-basename `filenames`), `contributes.grammars` (a `source.tcl`
//!   grammar row for any language that lacks one), and the
//!   `onLanguage:` half of `activationEvents`.
//! - VS Code `src/languageIds.ts` `TCL_LANGUAGE_IDS`,
//!   `src/extension.ts` `LANGUAGE_ID_DIALECTS`, and
//!   `src/languageIds.ts` `EXTENSION_LANGUAGE_IDS` (marked blocks).
//! - `JetBrains` `plugin.xml`: the `Tcl` and `iRule` fileType
//!   `extensions="…"` attributes.
//! - Sublime `Tcl.sublime-syntax` `file_extensions` and Zed
//!   `languages/tcl/config.toml` `path_suffixes` (single-syntax editors get
//!   the full union).
//! - The **per-dialect** editor surfaces, which used to be hand-maintained
//!   and had drifted (issue #1625): Sublime's `iRule` / `Expect` / `iApp` /
//!   `BIG-IP` syntaxes and Zed's secondary `languages/*/config.toml`, each
//!   carrying exactly the extensions its one dialect owns.
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

/// The per-dialect editor surfaces: one file, one canonical dialect whose
/// extensions it registers.
///
/// Every one of these was hand-maintained, and three of the five had drifted
/// from the catalog by the time issue #1625 audited them — Sublime's `iRule`
/// syntax was missing `irules` and its `Expect` syntax `expect`, so those
/// files opened under the umbrella `Tcl` syntax with the wrong scope, and
/// Zed's secondary configs were correct only by luck, gated by nothing.
///
/// Sublime's eight EDA / Tcl-version syntaxes are deliberately absent: they
/// declare no `file_extensions` at all today, and giving them one would make
/// a *new* ambiguous claim against the umbrella `Tcl.sublime-syntax` (which
/// lists the whole union) rather than fix a drifted one. That is a design
/// decision about Sublime's syntax-selection order, not catalog drift, so it
/// stays out of the generator until it is taken.
const DIALECT_SURFACES: &[(&str, &str, Surface)] = &[
    (
        "editors/sublime-text/iRule.sublime-syntax",
        "f5-irules",
        Surface::SublimeSyntax,
    ),
    (
        "editors/sublime-text/Expect.sublime-syntax",
        "expect",
        Surface::SublimeSyntax,
    ),
    (
        "editors/sublime-text/iApp.sublime-syntax",
        "f5-iapps",
        Surface::SublimeSyntax,
    ),
    (
        "editors/sublime-text/BIG-IP.sublime-syntax",
        "f5-bigip",
        Surface::SublimeSyntax,
    ),
    (
        "editors/zed/languages/irules/config.toml",
        "f5-irules",
        Surface::ZedConfig,
    ),
    (
        "editors/zed/languages/expect/config.toml",
        "expect",
        Surface::ZedConfig,
    ),
    (
        "editors/zed/languages/iapps/config.toml",
        "f5-iapps",
        Surface::ZedConfig,
    ),
    (
        "editors/zed/languages/tmsh/config.toml",
        "f5-tmsh",
        Surface::ZedConfig,
    ),
];

/// How a per-dialect surface spells its extension list.
#[derive(Clone, Copy)]
enum Surface {
    /// A `file_extensions:` YAML block terminated by a blank line.
    SublimeSyntax,
    /// A `path_suffixes = […]` TOML array.
    ZedConfig,
}

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
    /// Whole basenames the language claims by name rather than by extension
    /// (`bigip.conf`), from the catalog's `filenames` axis.
    filenames: Vec<String>,
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
        filenames: Vec::new(),
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
            filenames: profile.filenames.iter().map(|n| (*n).to_owned()).collect(),
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
        // The whole-basename axis. VS Code matches `filenames` exactly and
        // case-sensitively, so the catalog's lower-case names are what a real
        // `bigip.conf` carries. Before the catalog grew this axis, `tcl-bigip`
        // contributed none and a `bigip.conf` never associated at all, even
        // though the server has always routed it (issue #1625).
        if !lang.filenames.is_empty() {
            entry.insert(
                "filenames".to_owned(),
                Value::Array(lang.filenames.iter().cloned().map(Value::String).collect()),
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

    set_on_language_events(&mut root, &all_ids)?;

    let mut rendered =
        serde_json::to_string_pretty(&root).context("serialising VS Code package.json")?;
    rendered.push('\n');
    Ok(rendered)
}

/// Rewrite the `onLanguage:` half of `activationEvents` to name exactly the
/// languages the manifest contributes, leaving every other event
/// (`onChatParticipant:`, the generated `workspaceContains:` glob that
/// `gen-vscode-package` owns) untouched and in place.
///
/// Hand-written, this list carried 16 of the 19 contributed languages: a lone
/// `.tmsh` or `.tclspec` file activated nothing at all, because `onLanguage:`
/// is the only activation path an opened file takes — `workspaceContains:`
/// covers the workspace-*scan* path and, for `.tmsh`, does not even list the
/// extension (issue #1625). A language contributed but never named here is
/// exactly the failure the drift gate now catches.
fn set_on_language_events(manifest: &mut Value, all_ids: &[&str]) -> Result<()> {
    let events = manifest
        .get_mut("activationEvents")
        .and_then(Value::as_array_mut)
        .context("activationEvents must be an array")?;
    // Splice the generated block in where the first `onLanguage:` entry sat,
    // so the manifest's ordering (languages, then chat participants, then the
    // workspace glob) survives a regeneration.
    let at = events
        .iter()
        .position(|e| {
            e.as_str()
                .is_some_and(|s| s.starts_with(ON_LANGUAGE_PREFIX))
        })
        .unwrap_or(0);
    events.retain(|e| {
        !e.as_str()
            .is_some_and(|s| s.starts_with(ON_LANGUAGE_PREFIX))
    });
    let generated: Vec<Value> = all_ids
        .iter()
        .map(|id| Value::String(format!("{ON_LANGUAGE_PREFIX}{id}")))
        .collect();
    let at = at.min(events.len());
    events.splice(at..at, generated);
    Ok(())
}

const ON_LANGUAGE_PREFIX: &str = "onLanguage:";

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

/// The `.ext` → language-id and basename → language-id maps the extension's
/// runtime resolves a file with no (or a lost) association through.
fn render_extension_language_ids(original: &str, langs: &[Language]) -> Result<String> {
    let mut ext_rows = String::new();
    let mut name_rows = String::new();
    for (id, extensions, filenames) in owned_paths(langs) {
        for ext in extensions {
            let _ = writeln!(ext_rows, "  \".{ext}\": \"{id}\",");
        }
        for name in filenames {
            let _ = writeln!(name_rows, "  \"{name}\": \"{id}\",");
        }
    }
    let text = replace_marked_block(
        original,
        "// @generated:extension-language-ids:begin",
        "// @generated:extension-language-ids:end",
        &format!("export const EXTENSION_LANGUAGE_IDS: Record<string, string> = {{\n{ext_rows}}};\n"),
    )?;
    replace_marked_block(
        &text,
        "// @generated:filename-language-ids:begin",
        "// @generated:filename-language-ids:end",
        &format!("export const FILENAME_LANGUAGE_IDS: Record<string, string> = {{\n{name_rows}}};\n"),
    )
}

/// Every `(language id, extensions, filenames)` triple the editors register,
/// including the hand-maintained sublanguages the catalog has no profile for.
fn owned_paths(langs: &[Language]) -> Vec<(String, Vec<String>, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>, Vec<String>)> = langs
        .iter()
        .map(|l| (l.id.clone(), l.extensions.clone(), l.filenames.clone()))
        .collect();
    // `tcl-apl` is the iApp presentation language: an iApp *sublanguage* with
    // no dialect profile of its own, so its `.apl` extension and its
    // `presentation` basename are hand-maintained here rather than projected.
    out.push((
        "tcl-apl".to_owned(),
        HAND_MAINTAINED_EXTENSIONS
            .iter()
            .map(|e| (*e).to_owned())
            .collect(),
        vec!["presentation".to_owned()],
    ));
    out
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

/// Rewrite a Sublime syntax's `file_extensions:` block — a YAML list of
/// `  - ext` items, ending at the first line that is not one (a blank line in
/// the umbrella `Tcl` syntax, end-of-file in the per-dialect ones, which
/// carry nothing after their list).
fn set_sublime_extensions(original: &str, extensions: &[String]) -> Result<String> {
    let start = original
        .find("file_extensions:\n")
        .context("missing file_extensions block")?;
    let list_start = start + "file_extensions:\n".len();
    let mut list_end = list_start;
    while list_end < original.len() {
        let line_end = original[list_end..]
            .find('\n')
            .map_or(original.len(), |n| list_end + n + 1);
        if !original[list_end..line_end].starts_with("  - ") {
            break;
        }
        list_end = line_end;
    }
    if list_end == list_start {
        bail!("file_extensions block lists nothing");
    }
    let mut rows = String::new();
    for ext in extensions {
        let _ = writeln!(rows, "  - {ext}");
    }
    Ok(format!(
        "{}{}{}",
        &original[..list_start],
        rows,
        &original[list_end..]
    ))
}

/// Rewrite a Zed `config.toml`'s `path_suffixes = […]` array.
fn set_zed_suffixes(original: &str, extensions: &[String]) -> Result<String> {
    let start = original
        .find("path_suffixes = [")
        .context("missing path_suffixes")?;
    let end = original[start..]
        .find(']')
        .map(|n| start + n + 1)
        .context("unterminated path_suffixes")?;
    let quoted: Vec<String> = extensions.iter().map(|e| format!("\"{e}\"")).collect();
    Ok(format!(
        "{}path_suffixes = [{}]{}",
        &original[..start],
        quoted.join(", "),
        &original[end..]
    ))
}

fn render_sublime(original: &str, langs: &[Language]) -> Result<String> {
    set_sublime_extensions(original, &all_extensions(langs))
}

fn render_zed(original: &str, langs: &[Language]) -> Result<String> {
    set_zed_suffixes(original, &all_extensions(langs))
}

/// One per-dialect surface: exactly the extensions its dialect owns, in
/// catalog order.
fn render_dialect_surface(
    original: &str,
    langs: &[Language],
    dialect: &str,
    surface: Surface,
) -> Result<String> {
    let lang = langs
        .iter()
        .find(|l| l.dialect.as_deref() == Some(dialect))
        .ok_or_else(|| anyhow!("no editor language for dialect {dialect}"))?;
    if lang.extensions.is_empty() {
        bail!("dialect {dialect} owns no extensions to register");
    }
    match surface {
        Surface::SublimeSyntax => set_sublime_extensions(original, &lang.extensions),
        Surface::ZedConfig => set_zed_suffixes(original, &lang.extensions),
    }
}

type Render = Box<dyn Fn(&str, &[Language]) -> Result<String>>;

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let langs = languages()?;

    let mut renders: Vec<(&str, Render)> = vec![
        (VSCODE_PACKAGE, Box::new(render_vscode_package)),
        (VSCODE_LANGUAGE_IDS, Box::new(render_language_ids)),
        (VSCODE_LANGUAGE_IDS, Box::new(render_extension_language_ids)),
        (VSCODE_RUNTIME, Box::new(render_vscode_runtime)),
        (JETBRAINS_PLUGIN, Box::new(render_jetbrains)),
        (JETBRAINS_FILETYPE, Box::new(render_jetbrains_kotlin)),
        (SUBLIME_SYNTAX, Box::new(render_sublime)),
        (ZED_CONFIG, Box::new(render_zed)),
    ];
    for (rel, dialect, surface) in DIALECT_SURFACES {
        renders.push((
            rel,
            Box::new(move |original: &str, langs: &[Language]| {
                render_dialect_surface(original, langs, dialect, *surface)
            }),
        ));
    }

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
            if !drifted.contains(&rel) {
                drifted.push(rel);
            }
        } else {
            fs::write(&path, rendered).with_context(|| format!("writing {}", path.display()))?;
            println!("gen-editor-extensions: wrote {rel}");
        }
    }

    // Belt and braces: the model itself must be one-owner-per-extension
    // (the catalog's invariant tests cover the profiles; packs could still
    // collide with each other here).
    let mut owners: BTreeMap<String, String> = BTreeMap::new();
    let mut named: BTreeMap<String, String> = BTreeMap::new();
    for lang in &langs {
        for ext in &lang.extensions {
            if let Some(prior) = owners.insert(ext.clone(), lang.id.clone()) {
                bail!(
                    "extension {ext:?} registered by both {prior:?} and {:?}",
                    lang.id
                );
            }
        }
        // The basename axis is a function too — an editor cannot open one
        // file under two languages.
        for name in &lang.filenames {
            if let Some(prior) = named.insert(name.clone(), lang.id.clone()) {
                bail!(
                    "filename {name:?} registered by both {prior:?} and {:?}",
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
