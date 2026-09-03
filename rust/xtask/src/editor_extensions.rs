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
const HELIX_README: &str = "editors/helix/README.md";
const INSTALL_EDITORS: &str = "INSTALL-editors.md";

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
    // dedicated language, ride plain `tcl`. An `environment` block's own
    // `file_extension` claims route the same way, to the language of the
    // environment that declares them — the pack-declared environments'
    // door into the generated editor manifests (D17).
    let owned: Vec<String> = langs.iter().flat_map(|l| l.extensions.clone()).collect();
    for pack in &set.packs {
        let mut claims: Vec<(String, Option<&str>)> = pack
            .file_extensions
            .iter()
            .map(|row| (row.extension.clone(), row.dialect))
            .collect();
        for environment in &pack.environments {
            claims.extend(
                environment
                    .file_extensions
                    .iter()
                    .map(|claim| (claim.extension.to_string(), Some(environment.id.as_str()))),
            );
        }
        for (extension, dialect) in claims {
            if owned.contains(&extension) {
                continue;
            }
            let target = dialect
                .and_then(language_for_profile)
                .unwrap_or("tcl")
                .to_owned();
            let lang = langs
                .iter_mut()
                .find(|l| l.id == target)
                .ok_or_else(|| anyhow!("pack {}: no language {target}", pack.name))?;
            if !lang.extensions.contains(&extension) {
                lang.extensions.push(extension);
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
/// One `contributes.languages` entry: the two file-recognition axes plus the
/// shared language configuration.
///
/// The basename axis is contributed **twice**, on purpose. `filenames` is an
/// exact, case-*sensitive* match on a case-sensitive filesystem, while the
/// catalogue and the server deliberately compare basenames case-insensitively
/// — so a `BIGIP.CONF` matched nothing, opened as plaintext, and never even
/// activated the extension, leaving the client's own case-insensitive lookup
/// unreachable (issue #1625, review finding P2-2).
///
/// `filenamePatterns` is the fix, folding case per character rather than by
/// listing variants: `bigip.conf` has 2^9 casings, and the `[bB]` class
/// matches all of them exactly with no extra matches. It is the same trick the
/// `workspaceContains` activation glob uses for the same reason (issue #1215),
/// from the same registry helper, so the two can never disagree.
///
/// `filenames` stays beside it because it is the axis VS Code shows in
/// "Configure File Association" and the one older clients understand; the
/// pattern is the superset that makes the promise true.
fn contributed_language(lang: &Language, configuration: &str) -> Value {
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
    if !lang.filenames.is_empty() {
        entry.insert(
            "filenames".to_owned(),
            Value::Array(lang.filenames.iter().cloned().map(Value::String).collect()),
        );
        entry.insert(
            "filenamePatterns".to_owned(),
            Value::Array(
                lang.filenames
                    .iter()
                    .map(|name| Value::String(tcl_registry::dialects::fold_case_in_glob(name)))
                    .collect(),
            ),
        );
    }
    entry.insert(
        "configuration".to_owned(),
        Value::String(configuration.to_owned()),
    );
    Value::Object(entry)
}

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

    let mut out: Vec<Value> = langs
        .iter()
        .map(|lang| contributed_language(lang, &configuration))
        .collect();
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

/// An object key spelled the way Prettier's default `quoteProps: "as-needed"`
/// spells it: bare when it is a plain identifier, quoted otherwise.
///
/// The generated TypeScript is checked by the same `prettier --check` the rest
/// of the extension is, so a generator that always quotes produces a file the
/// formatter immediately rewrites — and then the drift gate and the format
/// gate disagree forever.
fn prettier_key(key: &str) -> String {
    let identifier = !key.is_empty()
        && !key.starts_with(|c: char| c.is_ascii_digit())
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if identifier {
        key.to_owned()
    } else {
        format!("\"{key}\"")
    }
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
            let _ = writeln!(name_rows, "  {}: \"{id}\",", prettier_key(&name));
        }
    }
    let text = replace_marked_block(
        original,
        "// @generated:extension-language-ids:begin",
        "// @generated:extension-language-ids:end",
        &format!(
            "export const EXTENSION_LANGUAGE_IDS: Record<string, string> = {{\n{ext_rows}}};\n"
        ),
    )?;
    replace_marked_block(
        &text,
        "// @generated:filename-language-ids:begin",
        "// @generated:filename-language-ids:end",
        &format!(
            "export const FILENAME_LANGUAGE_IDS: Record<string, string> = {{\n{name_rows}}};\n"
        ),
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

/// Helix has no extension of its own: its support is a block of `languages.toml`
/// users copy out of the README, one `[[language]]` entry per dialect. So the
/// README *is* the configuration surface, and a stale `file-types` line there
/// is a real routing bug rather than a documentation nit — it was missing
/// `scf` and `test` (issue #1625).
///
/// Every `[[language]]` block whose `name` is a catalog dialect (or plain
/// `tcl`) has its `file-types` rewritten from the catalog; a dialect that owns
/// extensions and has **no** block is a hard error, so adding a profile can
/// never silently leave Helix behind.
fn render_helix_readme(original: &str, langs: &[Language]) -> Result<String> {
    let mut out = original.to_owned();
    for lang in langs {
        let dialect = lang.dialect.as_deref().unwrap_or("tcl");
        let mut extensions = lang.extensions.clone();
        // The iApp presentation language has no Helix entry of its own (nor a
        // dialect profile); its files analyse as `f5-iapps`, so that is the
        // block they ride — the arrangement `render_jetbrains` uses for the
        // same sublanguage.
        if dialect == "f5-iapps" {
            extensions.extend(HAND_MAINTAINED_EXTENSIONS.iter().map(|e| (*e).to_owned()));
        }
        let anchor = format!("\nname = \"{dialect}\"\n");
        let Some(start) = out.find(&anchor) else {
            if extensions.is_empty() {
                continue;
            }
            bail!(
                "editors/helix/README.md has no `[[language]]` block for {dialect:?}, \
                 which owns {extensions:?} — add one beside the others"
            );
        };
        if extensions.is_empty() {
            continue;
        }
        let key = "\nfile-types = [";
        let line_start = out[start..]
            .find(key)
            .map(|n| start + n + 1)
            .with_context(|| format!("the {dialect:?} Helix block has no file-types"))?;
        let line_end = out[line_start..]
            .find('\n')
            .map(|n| line_start + n)
            .with_context(|| format!("the {dialect:?} Helix file-types line is unterminated"))?;
        let quoted: Vec<String> = extensions.iter().map(|e| format!("\"{e}\"")).collect();
        out.replace_range(
            line_start..line_end,
            &format!("file-types = [{}]", quoted.join(", ")),
        );
    }
    Ok(out)
}

/// The generic-client extension lists in the installation guide — Vim/Neovim
/// `au BufRead`, coc-settings' `fileExtensions`, and the Lua `file_patterns`.
///
/// Four identical stale nine-item lists, hand-maintained (issue #1625). They
/// are configuration users paste, so a missing entry is a client that never
/// attaches, not a documentation nit.
///
/// These name [`tcl_registry::dialects::TCL_SOURCE_EXTENSIONS`] rather than
/// the full registered union, and deliberately: each attaches **one** filetype
/// or language to everything it lists, which is the "project source we index"
/// question, not the "which dialect owns this suffix" one. That is also what
/// keeps the vendor suffixes that collide with foreign files (`.do`,
/// `.globals`, `.sdc`) out of a blanket `set filetype=tcl`.
fn render_install_editors(original: &str, _langs: &[Language]) -> Result<String> {
    let extensions = tcl_registry::dialects::TCL_SOURCE_EXTENSIONS;
    let mut out = original.to_owned();
    let renders: [(&str, &str, String); 3] = [
        (
            "au BufRead,BufNewFile ",
            " set filetype=tcl",
            extensions
                .iter()
                .map(|e| format!("*.{e}"))
                .collect::<Vec<_>>()
                .join(","),
        ),
        (
            "      \"fileExtensions\": [",
            "],",
            extensions
                .iter()
                .map(|e| format!("\".{e}\""))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        (
            "  file_patterns = { ",
            " },",
            extensions
                .iter()
                .map(|e| format!("\"%.{e}$\""))
                .collect::<Vec<_>>()
                .join(", "),
        ),
    ];
    for (prefix, suffix, body) in &renders {
        let mut at = 0;
        let mut rewrote = false;
        while let Some(found) = out[at..].find(prefix) {
            let list_start = at + found + prefix.len();
            let Some(list_end) = out[list_start..]
                .find(suffix)
                .map(|n| list_start + n)
                .filter(|end| !out[list_start..*end].contains('\n'))
            else {
                at = list_start;
                continue;
            };
            out.replace_range(list_start..list_end, body);
            at = list_start + body.len();
            rewrote = true;
        }
        if !rewrote {
            bail!("{INSTALL_EDITORS} has no `{prefix}…{suffix}` extension list to generate");
        }
    }
    Ok(out)
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

/// Every language the manifest contributes must also have an `onLanguage:`
/// activation event, and nothing else may — or opening a file of that language
/// activates nothing (16 of 19 were named, hand-written: issue #1625).
fn verify_every_language_activates(root: &std::path::Path, langs: &[Language]) -> Result<()> {
    let manifest: Value = serde_json::from_str(&fs::read_to_string(root.join(VSCODE_PACKAGE))?)
        .context("parsing VS Code package.json")?;
    let events: Vec<&str> = manifest["activationEvents"]
        .as_array()
        .context("activationEvents must be an array")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let contributed: Vec<&str> = manifest["contributes"]["languages"]
        .as_array()
        .context("contributes.languages must be an array")?
        .iter()
        .filter_map(|l| l["id"].as_str())
        .collect();
    for id in &contributed {
        let event = format!("{ON_LANGUAGE_PREFIX}{id}");
        if !events.contains(&event.as_str()) {
            bail!(
                "{VSCODE_PACKAGE} contributes language {id:?} but has no {event:?} \
                 activation event — a file of that language would activate nothing"
            );
        }
    }
    // And nothing the other way round: an `onLanguage:` for a language we do
    // not contribute activates us on somebody else's files.
    for event in &events {
        let Some(id) = event.strip_prefix(ON_LANGUAGE_PREFIX) else {
            continue;
        };
        if !contributed.contains(&id) {
            bail!("{VSCODE_PACKAGE} activates on language {id:?}, which it does not contribute");
        }
    }
    if contributed.len() != langs.len() + HAND_MAINTAINED_LANGUAGES.len() {
        bail!(
            "{VSCODE_PACKAGE} contributes {} languages; the catalog model has {}",
            contributed.len(),
            langs.len() + HAND_MAINTAINED_LANGUAGES.len()
        );
    }
    Ok(())
}

/// Every per-dialect editor surface on disk must be one this generator owns.
///
/// The inverse of the drift check, and the half it cannot do: a surface that
/// quietly drops out of [`DIALECT_SURFACES`] goes back to being
/// hand-maintained and nothing ever notices — exactly the state issue #1625
/// found Zed's secondary configs and Sublime's `iRule` / `Expect` syntaxes in.
fn verify_every_per_dialect_surface_is_generated(root: &std::path::Path) -> Result<()> {
    let generated: Vec<&str> = DIALECT_SURFACES.iter().map(|(rel, _, _)| *rel).collect();

    // Zed: every secondary language directory beside `tcl/`.
    let zed = root.join("editors/zed/languages");
    for entry in fs::read_dir(&zed).with_context(|| format!("reading {}", zed.display()))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        let rel = format!("editors/zed/languages/{name}/config.toml");
        // `apl` is the iApp presentation sublanguage: no dialect profile, so
        // no catalog row to project — the one hand-maintained Zed config.
        if !root.join(&rel).is_file() || rel == ZED_CONFIG || name == "apl" {
            continue;
        }
        if !generated.contains(&rel.as_str()) {
            bail!(
                "{rel} registers extensions but is not in DIALECT_SURFACES — \
                 add it (with its dialect) or record the exemption there"
            );
        }
    }

    // Sublime: every syntax that declares a `file_extensions:` block.
    let sublime = root.join("editors/sublime-text");
    for entry in fs::read_dir(&sublime).with_context(|| format!("reading {}", sublime.display()))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if !name.ends_with(".sublime-syntax") {
            continue;
        }
        let rel = format!("editors/sublime-text/{name}");
        if rel == SUBLIME_SYNTAX || name == "APL.sublime-syntax" {
            continue;
        }
        // The EDA / Tcl-version syntaxes claim nothing; see the note on
        // DIALECT_SURFACES for why that stays a decision, not drift.
        if !fs::read_to_string(root.join(&rel))?.contains("\nfile_extensions:\n") {
            continue;
        }
        if !generated.contains(&rel.as_str()) {
            bail!(
                "{rel} declares file_extensions but is not in DIALECT_SURFACES — \
                 add it (with its dialect) or record the exemption there"
            );
        }
    }
    Ok(())
}

type Render = Box<dyn Fn(&str, &[Language]) -> Result<String>>;

/// Every file this generator owns, paired with the render that rebuilds it.
///
/// Extracted from [`run`] so a test can assert on the *set* of targets. The
/// drift gate cannot: deleting a target leaves its committed file matching
/// itself, so the projection silently reverts to hand-maintained — which is
/// exactly the state issue #1625 found six surfaces in.
fn render_targets() -> Vec<(&'static str, Render)> {
    let mut renders: Vec<(&str, Render)> = vec![
        (VSCODE_PACKAGE, Box::new(render_vscode_package)),
        (VSCODE_LANGUAGE_IDS, Box::new(render_language_ids)),
        (VSCODE_LANGUAGE_IDS, Box::new(render_extension_language_ids)),
        (VSCODE_RUNTIME, Box::new(render_vscode_runtime)),
        (JETBRAINS_PLUGIN, Box::new(render_jetbrains)),
        (JETBRAINS_FILETYPE, Box::new(render_jetbrains_kotlin)),
        (SUBLIME_SYNTAX, Box::new(render_sublime)),
        (ZED_CONFIG, Box::new(render_zed)),
        (HELIX_README, Box::new(render_helix_readme)),
        (INSTALL_EDITORS, Box::new(render_install_editors)),
    ];
    for (rel, dialect, surface) in DIALECT_SURFACES {
        renders.push((
            rel,
            Box::new(move |original: &str, langs: &[Language]| {
                render_dialect_surface(original, langs, dialect, *surface)
            }),
        ));
    }
    renders
}

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let langs = languages()?;
    let renders = render_targets();

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

    // The drift gate compares each render against the file it owns, which
    // catches a *stale* projection but not a *missing* one. These two assert
    // the structural facts the projections exist to guarantee, against the
    // committed tree, in both modes.
    verify_every_language_activates(&root, &langs)?;
    verify_every_per_dialect_surface_is_generated(&root)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Each of these takes the **committed** surface, breaks it the way it was
    /// found broken in issue #1625, and asserts the render repairs it.
    ///
    /// That is deliberately not what `--check` proves. `--check` compares a
    /// render against the file it owns, so deleting the render leaves the
    /// committed file trivially matching itself and the gate stays green —
    /// the projection is gone and nothing says so. Breaking the input first is
    /// what makes the render itself the thing under test.
    fn committed(rel: &str) -> String {
        fs::read_to_string(repo_root().join(rel)).expect("committed surface")
    }

    #[test]
    fn the_render_restores_a_missing_on_language_activation() {
        let original = committed(VSCODE_PACKAGE);
        // The exact shape the hand-written list was in: a contributed
        // language with no activation event, so a lone `.tmsh` file activated
        // nothing at all.
        let broken = original.replace("    \"onLanguage:tcl-tmsh\",\n", "");
        assert_ne!(
            broken, original,
            "the manifest must carry the event to drop"
        );

        let rendered = render_vscode_package(&broken, &languages().unwrap()).unwrap();
        assert!(
            rendered.contains("\"onLanguage:tcl-tmsh\""),
            "the render must restore the dropped activation event"
        );
        // And it is the *catalog* that decides, not the input: an event for a
        // language we no longer contribute is dropped rather than preserved.
        let stray = original.replace(
            "    \"onLanguage:tcl\",\n",
            "    \"onLanguage:tcl\",\n    \"onLanguage:tcl-nonesuch\",\n",
        );
        let rendered = render_vscode_package(&stray, &languages().unwrap()).unwrap();
        assert!(
            !rendered.contains("tcl-nonesuch"),
            "the render must drop an activation event for a language we do not contribute"
        );
    }

    #[test]
    fn the_render_restores_a_missing_per_dialect_extension() {
        let langs = languages().unwrap();
        for (rel, dialect, surface) in DIALECT_SURFACES {
            let original = committed(rel);
            let lang = langs
                .iter()
                .find(|l| l.dialect.as_deref() == Some(*dialect))
                .expect("a language for every surface's dialect");
            let last = lang.extensions.last().expect("extensions");

            // Drop the dialect's last extension the way `iRule.sublime-syntax`
            // was missing `irules` and `Expect.sublime-syntax` was missing
            // `expect` — registered by the catalog, absent from the surface.
            // A surface whose dialect owns exactly one extension has nothing
            // to drop without emptying the list (which the renders reject on
            // its own account), so its single entry is corrupted instead.
            let broken = if lang.extensions.len() > 1 {
                match surface {
                    Surface::SublimeSyntax => original.replace(&format!("  - {last}\n"), ""),
                    Surface::ZedConfig => original.replace(&format!(", \"{last}\"]"), "]"),
                }
            } else {
                match surface {
                    Surface::SublimeSyntax => {
                        original.replace(&format!("  - {last}\n"), "  - zzbogus\n")
                    }
                    Surface::ZedConfig => {
                        original.replace(&format!("[\"{last}\"]"), "[\"zzbogus\"]")
                    }
                }
            };
            assert_ne!(broken, original, "{rel}: nothing was broken");

            let rendered = render_dialect_surface(&broken, &langs, dialect, *surface).unwrap();
            assert_eq!(rendered, original, "{rel}: the render must restore .{last}");
        }
    }

    #[test]
    fn the_render_restores_a_missing_helix_file_type() {
        let original = committed(HELIX_README);
        // Helix's README *is* the configuration users copy out, so a stale
        // `file-types` there is a routing bug: it was missing `test`.
        let broken = original.replace(
            "file-types = [\"tcl\", \"tk\", \"itcl\", \"tm\", \"test\"]",
            "file-types = [\"tcl\", \"tk\", \"itcl\", \"tm\"]",
        );
        assert_ne!(broken, original, "the README must carry the entry to drop");
        assert_eq!(
            render_helix_readme(&broken, &languages().unwrap()).unwrap(),
            original,
            "the render must restore the dropped Helix file type"
        );
    }

    /// The generator's *coverage* — which files it owns at all.
    ///
    /// The drift gate is blind here: a target dropped from the list leaves its
    /// committed file matching itself, so a projection can silently revert to
    /// hand-maintained without anything failing. Naming the roster explicitly
    /// is what makes that deletion a test failure, and the list is short
    /// enough that a reviewer can check it against the surfaces that exist.
    #[test]
    fn every_generated_surface_has_a_render_target() {
        let mut covered: Vec<&str> = render_targets().iter().map(|(rel, _)| *rel).collect();
        covered.sort_unstable();
        covered.dedup();

        let mut expected: Vec<&str> = vec![
            VSCODE_PACKAGE,
            VSCODE_LANGUAGE_IDS,
            VSCODE_RUNTIME,
            JETBRAINS_PLUGIN,
            JETBRAINS_FILETYPE,
            SUBLIME_SYNTAX,
            ZED_CONFIG,
            HELIX_README,
            INSTALL_EDITORS,
        ];
        expected.extend(DIALECT_SURFACES.iter().map(|(rel, _, _)| *rel));
        expected.sort_unstable();
        expected.dedup();

        assert_eq!(
            covered, expected,
            "a generated surface lost (or gained) its render target"
        );
        // And every one of them is a file that actually exists, so a renamed
        // surface fails here rather than at the next regeneration.
        for rel in &covered {
            assert!(
                repo_root().join(rel).is_file(),
                "{rel} is a render target but not a file"
            );
        }
    }

    /// The structural gate over the committed tree, as a test rather than only
    /// a CLI check — this is what catches a per-dialect surface dropping out
    /// of `DIALECT_SURFACES` and quietly going back to hand-maintained.
    #[test]
    fn every_per_dialect_surface_on_disk_is_generated() {
        verify_every_per_dialect_surface_is_generated(&repo_root())
            .expect("every per-dialect editor surface must be one the generator owns");
    }

    /// Review finding P2-2: the contributed basename axis has to match any
    /// casing, or a `BIGIP.CONF` opens as plaintext on a case-sensitive
    /// filesystem and never even activates the extension.
    #[test]
    fn contributed_filenames_carry_case_folded_patterns() {
        // Asserted on the **render**, not the committed bytes. Reading the
        // committed manifest would pass even with the projection deleted —
        // the same blindness the drift gate has, and the reason the other
        // render tests here break their input first.
        let original = committed(VSCODE_PACKAGE);
        let stripped: Value = {
            let mut manifest: Value = serde_json::from_str(&original).expect("manifest parses");
            for lang in manifest["contributes"]["languages"]
                .as_array_mut()
                .expect("languages")
            {
                if let Some(obj) = lang.as_object_mut() {
                    obj.remove("filenamePatterns");
                }
            }
            manifest
        };
        let broken = serde_json::to_string_pretty(&stripped).expect("serialise") + "\n";
        assert_ne!(
            broken, original,
            "the manifest must carry patterns to strip"
        );

        let rendered = render_vscode_package(&broken, &languages().unwrap()).unwrap();
        let manifest: Value = serde_json::from_str(&rendered).expect("rendered parses");
        let bigip = manifest["contributes"]["languages"]
            .as_array()
            .expect("languages")
            .iter()
            .find(|l| l["id"] == "tcl-bigip")
            .expect("tcl-bigip is contributed");
        let patterns: Vec<&str> = bigip["filenamePatterns"]
            .as_array()
            .expect("tcl-bigip must contribute filenamePatterns")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(
            patterns.contains(&"[bB][iI][gG][iI][pP].[cC][oO][nN][fF]"),
            "the BIG-IP basenames must be contributed case-folded; got {patterns:?}"
        );
        // Every plain `filenames` entry has a folded pattern beside it.
        for name in bigip["filenames"].as_array().expect("filenames") {
            let folded =
                tcl_registry::dialects::fold_case_in_glob(name.as_str().unwrap_or_default());
            assert!(
                patterns.contains(&folded.as_str()),
                "{name} has no case-folded pattern"
            );
        }
    }

    /// A profile that owns extensions and has no Helix block is a hard error,
    /// so adding one can never silently leave Helix behind — that is how
    /// `f5-bigip` came to have no entry at all.
    #[test]
    fn a_dialect_with_no_helix_block_is_an_error() {
        let original = committed(HELIX_README);
        let without_bigip = original.replace("name = \"f5-bigip\"", "name = \"f5-nonesuch\"");
        assert_ne!(without_bigip, original);
        let err = render_helix_readme(&without_bigip, &languages().unwrap())
            .expect_err("a missing Helix block must fail the generator");
        assert!(format!("{err}").contains("f5-bigip"), "{err}");
    }
}
