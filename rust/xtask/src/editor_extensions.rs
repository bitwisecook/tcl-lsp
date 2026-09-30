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
//! the compiled environment registry
//! ([`tcl_dialect::model::EnvironmentRegistry`]): each selectable
//! environment's `display_name`, editor identity, `file_extensions`,
//! `filenames` and `shebang_words`, plus the bundled `SpecTcl` packs'
//! `file_extension` rows (`specs/*.tclspec`). The core Tcl source extensions
//! come from [`tcl_registry::dialects::TCL_SOURCE_EXTENSIONS`], minus the
//! ones an environment owns.
//!
//! One language is not an environment: `tcl-apl`, the iApp presentation
//! language, is kept as an explicit extra ([`EXTRA_LANGUAGES`]) whose
//! dialect the language-id table names.
//!
//! Projections:
//! - VS Code `package.json` `contributes.languages` (one language per
//!   environment with an editor identity, carrying its extensions, its
//!   whole-basename `filenames` and a `firstLine` pattern built from its
//!   shebang words), `contributes.grammars` (a `source.tcl` grammar row for
//!   any language that lacks one), `contributes.semanticTokenScopes` (one
//!   block per language), the `onLanguage:` half of `activationEvents`, the
//!   per-language `configurationDefaults`, and the language-id pattern every
//!   `editorLangId =~` menu clause tests.
//! - VS Code `src/languageIds.ts` `TCL_LANGUAGE_IDS`,
//!   `LANGUAGE_ID_DIALECTS`, `EXTENSION_LANGUAGE_IDS` and
//!   `FILENAME_LANGUAGE_IDS` (marked blocks).
//! - `JetBrains` `plugin.xml`: the `Tcl` and `iRule` fileType
//!   `extensions="…"` attributes, `TclFileType.SUPPORTED_EXTENSIONS`, the
//!   `TextMate` bundle manifest that binds them all to the `source.tcl`
//!   grammar, and the iRule language id the pack-association reconciler
//!   compares against.
//! - Sublime's minimal `LSP-Tcl` helper suffix bridge.
//! - Zed's `languages/*/config.toml` `path_suffixes` and `first_line_pattern`:
//!   the plain `tcl` language takes the union of every extension and the
//!   shebang words of every environment without a language directory of its
//!   own; each other directory carries exactly what its one environment owns.
//!
//! Run `cargo xtask gen-editor-extensions`; `--check` makes the committed
//! projections a drift gate.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use regex::Regex;
use serde_json::{Map, Value};
use tcl_dialect::model::{
    EditorLanguageIdentityId, EnvironmentDefinition, EnvironmentRegistry, Family,
    LENIENT_ENVIRONMENT_ID,
};

use crate::util::{replace_marked_block, repo_root};

const VSCODE_PACKAGE: &str = "editors/vscode/package.json";
const VSCODE_LANGUAGE_IDS: &str = "editors/vscode/src/languageIds.ts";
const JETBRAINS_PLUGIN: &str = "editors/jetbrains/src/main/resources/META-INF/plugin.xml";
const JETBRAINS_FILETYPE: &str =
    "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/TclFileType.kt";
const JETBRAINS_TEXTMATE: &str = "editors/jetbrains/src/main/resources/textmate/package.json";
const JETBRAINS_RECONCILER: &str =
    "editors/jetbrains/src/main/kotlin/com/tcllsp/jetbrains/packs/PackAssociationReconciler.kt";
const SUBLIME_PLUGIN: &str = "editors/sublime-text/plugin.py";
const ZED_LANGUAGES_DIR: &str = "editors/zed/languages";

/// The environment `JetBrains` gives a file type of its own; every other
/// extension rides its plain `Tcl` file type and the server routes the
/// dialect.
const JETBRAINS_IRULE_ENVIRONMENT: &str = "f5-irules";

/// Where a Zed language directory's extensions come from.
#[derive(Clone, Copy)]
pub enum ZedSource {
    /// Every extension any language registers, and the shebang words of every
    /// environment without a directory of its own: the plain `Tcl` language,
    /// where a file with no language of its own lands.
    Union,
    /// The extensions and shebang words of one environment.
    Environment(&'static str),
    /// The extensions of the extra language with this editor id.
    Extra(&'static str),
}

/// One `editors/zed/languages/<dir>` directory: the language id Zed sends the
/// server for it and where its registrations come from. The set of
/// directories on disk must equal this table; the language's display name and
/// grammar are read from the directory's own `config.toml`.
#[derive(Clone, Copy)]
pub struct ZedLanguage {
    pub dir: &'static str,
    pub language_id: &'static str,
    pub source: ZedSource,
}

/// Every Zed language directory, in the order `extension.toml` lists them.
///
/// The `tmsh` directory registers the extensions of `f5-tmsh` but sends the
/// `tcl-bigip` language id: it is backed by the BIG-IP configuration grammar,
/// so the server must analyse its buffers as BIG-IP configuration.
pub const ZED_LANGUAGES: &[ZedLanguage] = &[
    ZedLanguage {
        dir: "tcl",
        language_id: "tcl",
        source: ZedSource::Union,
    },
    ZedLanguage {
        dir: "expect",
        language_id: "tcl-expect",
        source: ZedSource::Environment("expect"),
    },
    ZedLanguage {
        dir: "iapps",
        language_id: "tcl-iapp",
        source: ZedSource::Environment("f5-iapps"),
    },
    ZedLanguage {
        dir: "irules",
        language_id: "tcl-irule",
        source: ZedSource::Environment("f5-irules"),
    },
    ZedLanguage {
        dir: "tmsh",
        language_id: "tcl-bigip",
        source: ZedSource::Environment("f5-tmsh"),
    },
    ZedLanguage {
        dir: "apl",
        language_id: "tcl-apl",
        source: ZedSource::Extra("tcl-apl"),
    },
];

/// Which set of `TextMate` scopes a language maps its semantic token types to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ScopeShape {
    /// The token types every Tcl-family language shares.
    Tcl,
    /// The shared set plus the BIG-IP `object` token: plain Tcl, which hosts
    /// F5 rules, and every environment on an F5 core.
    TclWithObjects,
    /// BIG-IP configuration, which adds a token type per object kind. An
    /// environment with no Tcl core is the configuration surface.
    Bigip,
    /// The iApp presentation language's own token types.
    Apl,
}

/// The shared token types that precede the BIG-IP object token.
const HEAD_TOKEN_SCOPES: &[(&str, &str)] = &[
    ("regexp", "string.regexp.tcl"),
    ("escape", "constant.character.escape.tcl"),
    ("number", "constant.numeric.tcl"),
    ("event", "entity.name.tag.event.tcl"),
];

/// The BIG-IP `object` token.
const OBJECT_TOKEN_SCOPE: (&str, &str) = ("object", "entity.name.type.bigip");

/// The token types BIG-IP configuration adds, one per object kind.
const BIGIP_TOKEN_SCOPES: &[(&str, &str)] = &[
    ("fqdn", "string.unquoted.hostname.bigip"),
    ("ipAddress", "constant.numeric.ip-address.bigip"),
    ("port", "constant.numeric.port.bigip"),
    ("routeDomain", "entity.name.type.route-domain.bigip"),
    ("partition", "entity.name.namespace.partition.bigip"),
    ("username", "variable.other.username.bigip"),
    ("encrypted", "string.other.encrypted.bigip"),
    ("pool", "entity.name.type.pool.bigip"),
    ("monitor", "entity.name.type.monitor.bigip"),
    ("profile", "entity.name.type.profile.bigip"),
    ("vlan", "entity.name.type.vlan.bigip"),
    ("bigipInterface", "entity.name.type.interface.bigip"),
];

/// The shared token types for the sub-languages inside a Tcl string
/// (regular expressions, `binary`, `format` and `clock` specifiers).
const TAIL_TOKEN_SCOPES: &[(&str, &str)] = &[
    ("regexpGroup", "keyword.operator.regexp.group.tcl"),
    ("regexpCharClass", "constant.other.regexp.charclass.tcl"),
    ("regexpQuantifier", "keyword.operator.regexp.quantifier.tcl"),
    ("regexpAnchor", "keyword.operator.regexp.anchor.tcl"),
    ("regexpEscape", "constant.character.escape.regexp.tcl"),
    ("regexpBackref", "constant.other.regexp.backref.tcl"),
    (
        "regexpAlternation",
        "keyword.operator.regexp.alternation.tcl",
    ),
    ("binarySpec", "keyword.operator.binary.specifier.tcl"),
    ("binaryCount", "constant.numeric.binary.count.tcl"),
    ("binaryFlag", "keyword.operator.binary.flag.tcl"),
    ("formatPercent", "keyword.operator.format.percent.tcl"),
    ("formatSpec", "keyword.operator.format.specifier.tcl"),
    ("formatFlag", "keyword.operator.format.flag.tcl"),
    ("formatWidth", "constant.numeric.format.width.tcl"),
    ("clockPercent", "keyword.operator.clock.percent.tcl"),
    ("clockSpec", "keyword.operator.clock.specifier.tcl"),
    ("clockModifier", "keyword.operator.clock.modifier.tcl"),
];

/// The iApp presentation language's token types.
const APL_TOKEN_SCOPES: &[(&str, &str)] = &[
    ("escape", "constant.character.escape.apl"),
    ("number", "constant.numeric.apl"),
    ("aplSection", "keyword.control.apl"),
    ("aplFieldType", "keyword.other.apl"),
    ("aplAttribute", "entity.other.attribute-name.apl"),
    ("aplSectionName", "entity.name.section.apl"),
    ("aplFieldName", "variable.other.field.apl"),
    ("aplDefine", "keyword.other.define.apl"),
    ("aplDefineName", "entity.name.function.apl"),
    ("aplDirective", "keyword.control.directive.apl"),
    ("aplOptional", "keyword.control.optional.apl"),
    ("aplValidator", "support.constant.validator.apl"),
];

fn token_scopes(shape: ScopeShape) -> Vec<(&'static str, &'static str)> {
    let mut scopes = Vec::new();
    match shape {
        ScopeShape::Tcl => {
            scopes.extend_from_slice(HEAD_TOKEN_SCOPES);
            scopes.extend_from_slice(TAIL_TOKEN_SCOPES);
        }
        ScopeShape::TclWithObjects => {
            scopes.push(OBJECT_TOKEN_SCOPE);
            scopes.extend_from_slice(HEAD_TOKEN_SCOPES);
            scopes.extend_from_slice(TAIL_TOKEN_SCOPES);
        }
        ScopeShape::Bigip => {
            scopes.extend_from_slice(HEAD_TOKEN_SCOPES);
            scopes.push(OBJECT_TOKEN_SCOPE);
            scopes.extend_from_slice(BIGIP_TOKEN_SCOPES);
            scopes.extend_from_slice(TAIL_TOKEN_SCOPES);
        }
        ScopeShape::Apl => scopes.extend_from_slice(APL_TOKEN_SCOPES),
    }
    scopes
}

/// The shape an environment's language takes: BIG-IP configuration has no Tcl
/// core, and an F5 core carries the BIG-IP object token.
fn scope_shape(environment: &EnvironmentDefinition) -> ScopeShape {
    match environment.core.map(|core| core.family) {
        None => ScopeShape::Bigip,
        Some(Family::F5Tcl | Family::F5Irules) => ScopeShape::TclWithObjects,
        Some(_) => ScopeShape::Tcl,
    }
}

/// An editor language that is not an environment.
///
/// `tcl-apl` is the iApp presentation language: the `.apl` files and the
/// `presentation` file of an iApp template. It has its own editor language
/// (grammar, language configuration and token scopes) but no dialect of its
/// own: the language-id table sends it to the environment
/// [`EditorLanguageIdentityId::SELECTING`] names.
struct ExtraLanguage {
    id: &'static str,
    aliases: &'static [&'static str],
    extensions: &'static [&'static str],
    filenames: &'static [&'static str],
    configuration: &'static str,
    scopes: ScopeShape,
}

const EXTRA_LANGUAGES: &[ExtraLanguage] = &[ExtraLanguage {
    id: "tcl-apl",
    aliases: &["iApp APL", "apl", "presentation"],
    extensions: &["apl"],
    filenames: &["presentation"],
    configuration: "./apl-language-configuration.json",
    scopes: ScopeShape::Apl,
}];

/// Everything the editors register for one language id.
#[derive(Clone)]
pub struct Language {
    pub id: String,
    /// Menu labels, most human first (`["F5 iRules", "irule"]`).
    pub aliases: Vec<String>,
    /// Lower-case extensions without dots.
    pub extensions: Vec<String>,
    /// Whole basenames the language claims by name rather than by extension
    /// (`bigip.conf`), from the environment's `filenames` axis.
    pub filenames: Vec<String>,
    /// Whether the basenames are also contributed case-folded.
    fold_filename_case: bool,
    /// A language configuration of its own, when it is not the shared one.
    configuration: Option<String>,
    /// The canonical environment the language pins, if any (`None` for plain
    /// `tcl`, whose dialect is detected).
    pub dialect: Option<String>,
    /// Interpreter words a shebang line names to select the language.
    pub shebang_words: Vec<String>,
    /// Whether this is one of the [`EXTRA_LANGUAGES`]: a language that selects
    /// an environment without being its editor identity.
    pub is_extra: bool,
    scopes: ScopeShape,
}

fn strings<T: AsRef<str>>(items: impl IntoIterator<Item = T>) -> Vec<String> {
    items
        .into_iter()
        .map(|item| item.as_ref().to_owned())
        .collect()
}

fn language_of(environment: &EnvironmentDefinition, id: &str) -> Language {
    // The compact menu alias VS Code already used (`synopsys`, `irule`,
    // `jim`): the language id minus its `tcl-` prefix, when it has one.
    let mut aliases = vec![environment.display_name.to_string()];
    if let Some(short) = id.strip_prefix("tcl-") {
        aliases.push(short.to_owned());
    }
    Language {
        id: id.to_owned(),
        aliases,
        extensions: strings(
            environment
                .server_detection
                .file_extensions
                .iter()
                .map(|claim| claim.extension.as_ref()),
        ),
        filenames: strings(&environment.server_detection.filenames),
        fold_filename_case: true,
        configuration: None,
        dialect: Some(environment.id.to_string()),
        shebang_words: strings(&environment.server_detection.shebang_words),
        is_extra: false,
        scopes: scope_shape(environment),
    }
}

fn extra_language(extra: &ExtraLanguage) -> Result<Language> {
    let dialect = EditorLanguageIdentityId::selected_environment(extra.id)
        .with_context(|| format!("{} is not a selecting language id", extra.id))?;
    Ok(Language {
        id: extra.id.to_owned(),
        aliases: strings(extra.aliases),
        extensions: strings(extra.extensions),
        filenames: strings(extra.filenames),
        fold_filename_case: false,
        configuration: Some(extra.configuration.to_owned()),
        dialect: Some(dialect.to_owned()),
        shebang_words: Vec::new(),
        is_extra: true,
        scopes: extra.scopes,
    })
}

/// The assembled model: every language the editors register, in stable
/// order — plain `tcl` first, then each environment with an editor identity
/// in selectable order, then the extra languages.
pub fn languages() -> Result<Vec<Language>> {
    let root = repo_root();
    let set = tcl_spectcl::bundled::load_from(&root.join("specs"));
    let registry = EnvironmentRegistry::compiled();
    let lenient = registry
        .resolve(LENIENT_ENVIRONMENT_ID)
        .context("the compiled registry has no plain `tcl` environment")?;

    let language_for_environment = |name: &str| -> Option<&'static str> {
        registry
            .resolve(name)
            .and_then(|environment| environment.editor_identity)
            .map(EditorLanguageIdentityId::as_str)
    };

    let mut langs: Vec<Language> = Vec::new();
    langs.push(Language {
        id: "tcl".to_owned(),
        aliases: vec!["Tcl".to_owned(), "tcl".to_owned()],
        extensions: strings(
            lenient
                .server_detection
                .file_extensions
                .iter()
                .map(|claim| claim.extension.as_ref()),
        ),
        filenames: Vec::new(),
        fold_filename_case: true,
        configuration: None,
        dialect: None,
        shebang_words: strings(&lenient.server_detection.shebang_words),
        is_extra: false,
        scopes: ScopeShape::TclWithObjects,
    });
    for environment in EnvironmentRegistry::compiled_selectable() {
        let Some(identity) = environment.editor_identity else {
            continue;
        };
        langs.push(language_of(environment, identity.as_str()));
    }
    for extra in EXTRA_LANGUAGES {
        langs.push(extra_language(extra)?);
    }

    // Pack-declared extensions land on the language of the environment their
    // row routes to; rows with no `-dialect`, or whose environment has no
    // dedicated language, ride plain `tcl`. An `environment` block's own
    // `file_extension` claims route the same way, to the language of the
    // environment that declares them — the pack-declared environments' door
    // into the generated editor manifests.
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
                .and_then(language_for_environment)
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

    // The core Tcl source extensions that no environment or pack owns are the
    // plain-`tcl` language's registration list.
    let owned: Vec<String> = langs.iter().flat_map(|l| l.extensions.clone()).collect();
    for ext in tcl_registry::dialects::TCL_SOURCE_EXTENSIONS {
        if owned.iter().any(|o| o == ext) {
            continue;
        }
        langs[0].extensions.push((*ext).to_owned());
    }

    Ok(langs)
}

/// `^#!.*\bjimsh\b`: a first line whose interpreter is one of `words`.
fn first_line_pattern(words: &[String]) -> Option<String> {
    let escaped: Vec<String> = words.iter().map(|word| regex::escape(word)).collect();
    match escaped.as_slice() {
        [] => None,
        [word] => Some(format!(r"^#!.*\b{word}\b")),
        many => Some(format!(r"^#!.*\b(?:{})\b", many.join("|"))),
    }
}

/// One `contributes.languages` entry: the two file-recognition axes plus the
/// shared language configuration.
///
/// The basename axis is contributed **twice**, on purpose. `filenames` is an
/// exact, case-*sensitive* match on a case-sensitive filesystem, while the
/// registry and the server deliberately compare basenames case-insensitively
/// — so on `filenames` alone a `BIGIP.CONF` would match nothing, open as
/// plaintext, and never even activate the extension, leaving the client's
/// own case-insensitive lookup unreachable.
///
/// `filenamePatterns` folds case per character rather than
/// listing variants: `bigip.conf` has 2^9 casings, and the `[bB]` class
/// matches all of them exactly with no extra matches. It is the same trick the
/// `workspaceContains` activation glob uses for the same reason,
/// from the same registry helper, so the two can never disagree.
///
/// `filenames` stays beside it because it is the axis VS Code shows in
/// "Configure File Association" and the one older clients understand; the
/// pattern is the superset that makes the promise true.
///
/// `firstLine` names the interpreter words a shebang line selects the
/// language by, so a script with no extension still opens in its language.
fn contributed_language(lang: &Language, configuration: &str) -> Value {
    let mut entry = Map::new();
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
        if lang.fold_filename_case {
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
    }
    if let Some(pattern) = first_line_pattern(&lang.shebang_words) {
        entry.insert("firstLine".to_owned(), Value::String(pattern));
    }
    entry.insert(
        "configuration".to_owned(),
        Value::String(
            lang.configuration
                .clone()
                .unwrap_or_else(|| configuration.to_owned()),
        ),
    );
    Value::Object(entry)
}

/// One `contributes.semanticTokenScopes` block per language: the token types
/// the server emits, mapped to the `TextMate` scopes a theme colours.
fn semantic_token_scopes(langs: &[Language]) -> Value {
    Value::Array(
        langs
            .iter()
            .map(|lang| {
                let scopes: Map<String, Value> = token_scopes(lang.scopes)
                    .into_iter()
                    .map(|(token, scope)| {
                        (
                            token.to_owned(),
                            Value::Array(vec![Value::String(scope.to_owned())]),
                        )
                    })
                    .collect();
                let mut block = Map::new();
                block.insert("language".to_owned(), Value::String(lang.id.clone()));
                block.insert("scopes".to_owned(), Value::Object(scopes));
                Value::Object(block)
            })
            .collect(),
    )
}

/// The regex every `editorLangId =~` menu clause tests a language id against:
/// one anchored prefix per group of ids — `tcl` covers `tcl-irule`, `tcl84`
/// and `tclspec`; `sslictcl` covers itself.
fn language_id_pattern(langs: &[Language]) -> Result<String> {
    let mut ids: Vec<&str> = langs.iter().map(|l| l.id.as_str()).collect();
    if let Some(bad) = ids
        .iter()
        .find(|id| !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
    {
        bail!("language id {bad:?} is not a plain identifier a when-clause regex can hold");
    }
    ids.sort_by_key(|id| (id.len(), *id));
    let mut roots: Vec<&str> = Vec::new();
    for id in ids {
        if !roots.iter().any(|root| id.starts_with(root)) {
            roots.push(id);
        }
    }
    roots.sort_unstable();
    Ok(match roots.as_slice() {
        [root] => format!("^{root}"),
        many => format!("^(?:{})", many.join("|")),
    })
}

fn when_clause_regex() -> Regex {
    Regex::new(r"editorLangId =~ /([^/]*)/").expect("the when-clause pattern is a valid regex")
}

/// Rewrite the language-id regex of every `editorLangId =~` clause under a
/// `when` key.
fn rewrite_when_clauses(value: &mut Value, clause: &Regex, pattern: &str) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                match child {
                    Value::String(text) if key == "when" => {
                        *text = clause
                            .replace_all(text, |_: &regex::Captures| {
                                format!("editorLangId =~ /{pattern}/")
                            })
                            .into_owned();
                    }
                    _ => rewrite_when_clauses(child, clause, pattern),
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                rewrite_when_clauses(item, clause, pattern);
            }
        }
        _ => {}
    }
}

fn collect_when_clauses<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                match child {
                    Value::String(text) if key == "when" => out.push(text),
                    _ => collect_when_clauses(child, out),
                }
            }
        }
        Value::Array(items) => items
            .iter()
            .for_each(|item| collect_when_clauses(item, out)),
        _ => {}
    }
}

/// Every `editorLangId =~ /re/` clause in the manifest must match every
/// language the manifest contributes; otherwise a menu entry meant for every
/// Tcl-family file is missing from a language whose id the regex misses.
fn check_when_clauses(manifest: &Value, langs: &[Language]) -> Result<()> {
    let mut clauses = Vec::new();
    collect_when_clauses(manifest, &mut clauses);
    let clause = when_clause_regex();
    let mut seen = BTreeSet::new();
    for text in clauses {
        for captures in clause.captures_iter(text) {
            let pattern = &captures[1];
            if !seen.insert(pattern.to_owned()) {
                continue;
            }
            let re = Regex::new(pattern)
                .with_context(|| format!("the when-clause regex /{pattern}/ does not compile"))?;
            let missed: Vec<&str> = langs
                .iter()
                .map(|l| l.id.as_str())
                .filter(|id| !re.is_match(id))
                .collect();
            if !missed.is_empty() {
                bail!(
                    "{VSCODE_PACKAGE}: `editorLangId =~ /{pattern}/` matches none of {missed:?}, \
                     so a menu entry for every Tcl-family file is missing from those languages"
                );
            }
        }
    }
    Ok(())
}

fn render_vscode_package(original: &str, langs: &[Language]) -> Result<String> {
    let mut root: Value = serde_json::from_str(original).context("parsing VS Code package.json")?;

    let configuration = root["contributes"]["languages"]
        .as_array()
        .context("contributes.languages must be an array")?
        .first()
        .and_then(|l| l["configuration"].as_str())
        .unwrap_or("./language-configuration.json")
        .to_owned();

    root["contributes"]["languages"] = Value::Array(
        langs
            .iter()
            .map(|lang| contributed_language(lang, &configuration))
            .collect(),
    );

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
    let all_ids: Vec<&str> = langs.iter().map(|l| l.id.as_str()).collect();
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
    // extension's stickyScroll suite pins per language id. Keys that are not a
    // language block (`editor.semanticTokenColorCustomizations`) keep their
    // place ahead of the language blocks.
    let defaults = root["contributes"]["configurationDefaults"]
        .as_object_mut()
        .context("contributes.configurationDefaults must be an object")?;
    let previous = std::mem::take(defaults);
    let language_keys: Vec<String> = all_ids.iter().map(|id| format!("[{id}]")).collect();
    for (key, value) in &previous {
        if !language_keys.contains(key) {
            defaults.insert(key.clone(), value.clone());
        }
    }
    for key in language_keys {
        let mut entry = previous
            .get(&key)
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        if entry.get("editor.stickyScroll.defaultModel").is_none() {
            entry["editor.stickyScroll.defaultModel"] =
                Value::String("foldingProviderModel".into());
        }
        defaults.insert(key, entry);
    }

    set_on_language_events(&mut root, &all_ids)?;
    root["contributes"]["semanticTokenScopes"] = semantic_token_scopes(langs);
    rewrite_when_clauses(
        &mut root,
        &when_clause_regex(),
        &language_id_pattern(langs)?,
    );

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
/// A hand-written list can silently drop languages: `onLanguage:`
/// is the only activation path an opened file takes — `workspaceContains:`
/// covers the workspace-*scan* path and, for `.tmsh`, does not even list the
/// extension. A lone `.tmsh` or `.tclspec` file with no `onLanguage:` entry
/// activates nothing at all, which is exactly the failure the drift gate
/// catches.
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

fn render_language_ids(original: &str, langs: &[Language]) -> Result<String> {
    let mut rows = String::new();
    for lang in langs {
        let _ = writeln!(rows, "  \"{}\",", lang.id);
    }
    let body = format!("export const TCL_LANGUAGE_IDS = new Set([\n{rows}]);\n");
    replace_marked_block(
        original,
        "// @generated:language-ids:begin",
        "// @generated:language-ids:end",
        &body,
    )
}

/// The language ids that select an environment: every environment's editor
/// identity, then the spellings a client may send that are not an
/// environment's identity ([`EditorLanguageIdentityId::SELECTING`]).
fn language_id_dialects(langs: &[Language]) -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = langs
        .iter()
        .filter_map(|lang| {
            let dialect = lang.dialect.as_ref()?;
            let selecting = EditorLanguageIdentityId::selected_environment(&lang.id).is_some();
            (!selecting).then(|| (lang.id.clone(), dialect.clone()))
        })
        .collect();
    rows.extend(
        EditorLanguageIdentityId::SELECTING
            .iter()
            .map(|&(id, environment)| (id.to_owned(), environment.to_owned())),
    );
    rows
}

/// Which environment a language id implies, for the status bar before the
/// server has answered.
fn render_language_id_dialects(original: &str, langs: &[Language]) -> Result<String> {
    let mut rows = String::new();
    for (id, dialect) in language_id_dialects(langs) {
        let _ = writeln!(rows, "  {}: \"{dialect}\",", prettier_key(&id));
    }
    let body =
        format!("export const LANGUAGE_ID_DIALECTS: Record<string, string> = {{\n{rows}}};\n");
    replace_marked_block(
        original,
        "// @generated:language-id-dialects:begin",
        "// @generated:language-id-dialects:end",
        &body,
    )
}

/// The `.ext` → language-id and basename → language-id maps the extension's
/// runtime resolves a file with no (or a lost) association through.
fn render_extension_language_ids(original: &str, langs: &[Language]) -> Result<String> {
    let mut ext_rows = String::new();
    let mut name_rows = String::new();
    for lang in langs {
        for ext in &lang.extensions {
            let _ = writeln!(ext_rows, "  \".{ext}\": \"{}\",", lang.id);
        }
        for name in &lang.filenames {
            let _ = writeln!(name_rows, "  {}: \"{}\",", prettier_key(name), lang.id);
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

fn is_irule(lang: &Language) -> bool {
    lang.dialect.as_deref() == Some(JETBRAINS_IRULE_ENVIRONMENT)
}

fn render_jetbrains(original: &str, langs: &[Language]) -> Result<String> {
    // JetBrains keeps two fileTypes: `iRule` (its own icon/type) and `Tcl`
    // (everything else — JetBrains routes dialects server-side).
    let irule: Vec<String> = langs
        .iter()
        .filter(|l| is_irule(l))
        .flat_map(|l| l.extensions.clone())
        .collect();
    let main: Vec<String> = langs
        .iter()
        .filter(|l| !is_irule(l))
        .flat_map(|l| l.extensions.clone())
        .collect();
    let text = set_jetbrains_filetype_extensions(original, "Tcl", &main.join(";"))?;
    set_jetbrains_filetype_extensions(&text, "iRule", &irule.join(";"))
}

/// Every registered extension, for the single-syntax editors.
pub fn all_extensions(langs: &[Language]) -> Vec<String> {
    langs.iter().flat_map(|l| l.extensions.clone()).collect()
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

/// The language id the pack-association reconciler maps to the iRule file
/// type: the editor identity of the environment `JetBrains` types on its own.
fn render_jetbrains_reconciler(original: &str, langs: &[Language]) -> Result<String> {
    let irule = langs
        .iter()
        .find(|l| is_irule(l))
        .with_context(|| format!("no language for {JETBRAINS_IRULE_ENVIRONMENT}"))?;
    let body = format!(
        "    private const val IRULE_LANGUAGE_ID = \"{}\"\n",
        irule.id
    );
    replace_marked_block(
        original,
        "// @generated:irule-language-id:begin",
        "// @generated:irule-language-id:end",
        &body,
    )
}

/// The `JetBrains` plugin's `TextMate` bundle manifest claims the same union
/// again, in the one place the *grammar* is bound to file extensions.
///
/// `TextMateService` resolves a grammar by file name, so an extension missing
/// here opens as a Tcl file that no grammar matches — a plain-text editor with
/// a language server attached, which is what shipped before the bundle was
/// registered at all.
fn render_jetbrains_textmate(original: &str, langs: &[Language]) -> Result<String> {
    let mut root: Value =
        serde_json::from_str(original).context("parsing the JetBrains TextMate package.json")?;
    let extensions: Vec<Value> = all_extensions(langs)
        .into_iter()
        .map(|ext| Value::String(format!(".{ext}")))
        .collect();
    let languages = root["contributes"]["languages"]
        .as_array_mut()
        .context("contributes.languages must be an array")?;
    let [language] = languages.as_mut_slice() else {
        bail!(
            "{JETBRAINS_TEXTMATE} must contribute exactly one language — the plugin \
             binds every extension to the single `source.tcl` grammar"
        );
    };
    language["extensions"] = Value::Array(extensions);
    Ok(format!("{}\n", serde_json::to_string_pretty(&root)?))
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

/// Set a Zed `config.toml`'s `first_line_pattern`, inserting the key after
/// `path_suffixes` when the file has none, or dropping it when there is no
/// pattern to state.
fn set_zed_first_line(original: &str, pattern: Option<&str>) -> Result<String> {
    let key = "first_line_pattern = ";
    if let Some(start) = original.find(key) {
        let end = original[start..]
            .find('\n')
            .map_or(original.len(), |n| start + n + 1);
        let replacement = pattern.map_or_else(String::new, |p| format!("{key}\"{p}\"\n"));
        return Ok(format!(
            "{}{replacement}{}",
            &original[..start],
            &original[end..]
        ));
    }
    let Some(pattern) = pattern else {
        return Ok(original.to_owned());
    };
    let suffixes = original
        .find("path_suffixes = [")
        .context("missing path_suffixes")?;
    let line_end = original[suffixes..]
        .find('\n')
        .map(|n| suffixes + n + 1)
        .context("path_suffixes must end in a newline")?;
    Ok(format!(
        "{}{key}\"{pattern}\"\n{}",
        &original[..line_end],
        &original[line_end..]
    ))
}

/// The first-line pattern a Zed language uses: the interpreter names,
/// without a version suffix (`tclsh8.6` is covered by `tclsh` because the
/// pattern is not anchored after the word).
fn zed_first_line_pattern(words: &[String]) -> Option<String> {
    let stems: BTreeSet<&str> = words
        .iter()
        .map(|word| word.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.'))
        .collect();
    if stems.is_empty() {
        return None;
    }
    let alternatives: Vec<String> = stems.into_iter().map(regex::escape).collect();
    Some(format!("^#!.*(?:{})", alternatives.join("|")))
}

/// The shebang words of every environment that has no language directory of
/// its own: the ones the plain Zed `Tcl` language has to recognise.
fn zed_union_shebang_words() -> Vec<String> {
    let owned: Vec<&str> = ZED_LANGUAGES
        .iter()
        .filter_map(|zed| match zed.source {
            ZedSource::Environment(id) => Some(id),
            ZedSource::Union | ZedSource::Extra(_) => None,
        })
        .collect();
    EnvironmentRegistry::compiled()
        .definitions()
        .iter()
        .filter(|environment| !owned.contains(&environment.id.as_str()))
        .flat_map(|environment| strings(&environment.server_detection.shebang_words))
        .collect()
}

fn render_sublime_plugin(original: &str, langs: &[Language]) -> Result<String> {
    let mut rows = String::new();
    for extension in all_extensions(langs) {
        let _ = writeln!(rows, "    \"{extension}\",");
    }
    replace_marked_block(
        original,
        "# @generated:file-extensions:begin",
        "# @generated:file-extensions:end",
        &rows,
    )
}

/// One Zed language directory's `config.toml`: the extensions and shebang
/// words its source owns.
fn render_zed_language(original: &str, langs: &[Language], zed: &ZedLanguage) -> Result<String> {
    let (extensions, words) = match zed.source {
        ZedSource::Union => (all_extensions(langs), zed_union_shebang_words()),
        ZedSource::Environment(id) => {
            let lang = language_of_environment(langs, id)
                .ok_or_else(|| anyhow!("no editor language for environment {id}"))?;
            (lang.extensions.clone(), lang.shebang_words.clone())
        }
        ZedSource::Extra(id) => {
            let lang = langs
                .iter()
                .find(|l| l.id == id)
                .ok_or_else(|| anyhow!("no editor language {id}"))?;
            (lang.extensions.clone(), Vec::new())
        }
    };
    if extensions.is_empty() {
        bail!("{} owns no extensions to register", zed.dir);
    }
    let text = set_zed_suffixes(original, &extensions)?;
    set_zed_first_line(&text, zed_first_line_pattern(&words).as_deref())
}

/// The language an environment's editor identity names: the one its
/// documents open under. Not the extra languages, which select an
/// environment without being its identity.
pub fn language_of_environment<'a>(
    langs: &'a [Language],
    environment: &str,
) -> Option<&'a Language> {
    let identity = EnvironmentRegistry::compiled()
        .resolve(environment)?
        .editor_identity?;
    langs
        .iter()
        .find(|l| !l.is_extra && l.id == identity.as_str())
}

/// The `config.toml` of each Zed language directory, repo-relative.
pub fn zed_config_path(zed: &ZedLanguage) -> String {
    format!("{ZED_LANGUAGES_DIR}/{}/config.toml", zed.dir)
}

/// Every language the manifest contributes must also have an `onLanguage:`
/// activation event, and nothing else may — or opening a file of that language
/// activates nothing.
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
    if contributed.len() != langs.len() {
        bail!(
            "{VSCODE_PACKAGE} contributes {} languages; the registry model has {}",
            contributed.len(),
            langs.len()
        );
    }
    check_when_clauses(&manifest, langs)
}

/// Every Zed language directory on disk must be one this generator owns, and
/// every one it owns must exist and name a language id an editor contributes.
///
/// The inverse of the drift check, and the half it cannot do: a directory
/// that quietly drops out of [`ZED_LANGUAGES`] has nothing generating its
/// registrations, and nothing notices.
fn verify_every_zed_directory_is_wired(root: &std::path::Path) -> Result<()> {
    let zed = root.join(ZED_LANGUAGES_DIR);
    for entry in fs::read_dir(&zed).with_context(|| format!("reading {}", zed.display()))? {
        let entry = entry?;
        if !entry.path().join("config.toml").is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !ZED_LANGUAGES.iter().any(|z| z.dir == name) {
            bail!(
                "{ZED_LANGUAGES_DIR}/{name} is a Zed language directory that ZED_LANGUAGES does \
                 not name — add it with the language id it sends and the environment it owns"
            );
        }
    }
    for entry in ZED_LANGUAGES {
        if !root.join(zed_config_path(entry)).is_file() {
            bail!(
                "ZED_LANGUAGES names {}, which has no config.toml",
                entry.dir
            );
        }
        if EditorLanguageIdentityId::new(entry.language_id).is_none() {
            bail!(
                "{}: {:?} is not a language id an editor contributes",
                entry.dir,
                entry.language_id
            );
        }
        if let ZedSource::Environment(id) = entry.source
            && tcl_registry::model::resolve_known_environment(id).is_none()
        {
            bail!("{}: {id:?} is not an environment", entry.dir);
        }
    }
    Ok(())
}

type Render = Box<dyn Fn(&str, &[Language]) -> Result<String>>;

/// Every file this generator owns, paired with the render that rebuilds it.
///
/// Extracted from [`run`] so a test can assert on the *set* of targets. The
/// drift gate cannot: deleting a target leaves its committed file matching
/// itself, so the projection silently stops being generated.
fn render_targets() -> Vec<(String, Render)> {
    let mut renders: Vec<(String, Render)> = vec![
        (VSCODE_PACKAGE.to_owned(), Box::new(render_vscode_package)),
        (
            VSCODE_LANGUAGE_IDS.to_owned(),
            Box::new(render_language_ids),
        ),
        (
            VSCODE_LANGUAGE_IDS.to_owned(),
            Box::new(render_extension_language_ids),
        ),
        (
            VSCODE_LANGUAGE_IDS.to_owned(),
            Box::new(render_language_id_dialects),
        ),
        (JETBRAINS_PLUGIN.to_owned(), Box::new(render_jetbrains)),
        (
            JETBRAINS_FILETYPE.to_owned(),
            Box::new(render_jetbrains_kotlin),
        ),
        (
            JETBRAINS_TEXTMATE.to_owned(),
            Box::new(render_jetbrains_textmate),
        ),
        (
            JETBRAINS_RECONCILER.to_owned(),
            Box::new(render_jetbrains_reconciler),
        ),
        (SUBLIME_PLUGIN.to_owned(), Box::new(render_sublime_plugin)),
    ];
    for zed in ZED_LANGUAGES {
        renders.push((
            zed_config_path(zed),
            Box::new(move |original: &str, langs: &[Language]| {
                render_zed_language(original, langs, zed)
            }),
        ));
    }
    renders
}

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let langs = languages()?;
    let renders = render_targets();

    let mut drifted: Vec<String> = Vec::new();
    for (rel, render) in renders {
        let path = root.join(&rel);
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
    // (the registry's invariant tests cover the environments; two packs could
    // collide with each other here).
    let mut owners: BTreeMap<String, String> = BTreeMap::new();
    let mut named: BTreeMap<String, String> = BTreeMap::new();
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for lang in &langs {
        if !ids.insert(&lang.id) {
            bail!("language id {:?} is registered twice", lang.id);
        }
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
    // catches a *stale* projection but not a *missing* one. These
    // assert the structural facts the projections exist to guarantee, against
    // the committed tree, in both modes.
    verify_every_language_activates(&root, &langs)?;
    verify_every_zed_directory_is_wired(&root)?;

    if check && !drifted.is_empty() {
        for rel in &drifted {
            eprintln!("gen-editor-extensions: {rel} is out of sync");
        }
        bail!("run `cargo xtask gen-editor-extensions` and commit the result");
    }
    println!(
        "gen-editor-extensions: {} languages, {} extensions{}",
        langs.len(),
        owners.len(),
        if check { " — in sync" } else { "" }
    );
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each of these takes the **committed** surface, breaks it in a
    /// realistic way, and asserts the render repairs it.
    ///
    /// That is deliberately not what `--check` proves. `--check` compares a
    /// render against the file it owns, so deleting the render leaves the
    /// committed file trivially matching itself and the gate stays green —
    /// the projection is gone and nothing says so. Breaking the input first is
    /// what makes the render itself the thing under test.
    fn committed(rel: &str) -> String {
        fs::read_to_string(repo_root().join(rel)).expect("committed surface")
    }

    fn manifest_language(rendered: &str, id: &str) -> Value {
        let manifest: Value = serde_json::from_str(rendered).expect("manifest parses");
        manifest["contributes"]["languages"]
            .as_array()
            .expect("languages")
            .iter()
            .find(|l| l["id"] == id)
            .unwrap_or_else(|| panic!("{id} is contributed"))
            .clone()
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
        // And it is the *registry* that decides, not the input: an event for a
        // language we do not contribute is dropped rather than preserved.
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
    fn the_render_restores_a_missing_zed_extension() {
        let langs = languages().unwrap();
        for zed in ZED_LANGUAGES {
            let rel = zed_config_path(zed);
            let original = committed(&rel);
            let last = original
                .lines()
                .find_map(|line| line.strip_prefix("path_suffixes = ["))
                .and_then(|rest| rest.strip_suffix(']'))
                .and_then(|list| list.rsplit(", ").next())
                .expect("path_suffixes")
                .to_owned();

            // Drop the language's last extension — registered by the
            // registry, absent from the surface. A directory that owns
            // exactly one extension has nothing to drop without emptying the
            // list, so its single entry is corrupted instead.
            let broken = if original.contains(&format!(", {last}]")) {
                original.replace(&format!(", {last}]"), "]")
            } else {
                original.replace(&format!("[{last}]"), "[\"zzbogus\"]")
            };
            assert_ne!(broken, original, "{rel}: nothing was broken");

            let rendered = render_zed_language(&broken, &langs, zed).unwrap();
            assert_eq!(rendered, original, "{rel}: the render must restore {last}");
        }
    }

    #[test]
    fn jim_is_contributed_with_a_shebang_pattern_and_no_extension() {
        let langs = languages().unwrap();
        let rendered = render_vscode_package(&committed(VSCODE_PACKAGE), &langs).unwrap();
        let jim = manifest_language(&rendered, "tcl-jim");
        assert_eq!(jim["aliases"], serde_json::json!(["Jim Tcl", "jim"]));
        assert!(jim.get("extensions").is_none(), "Jim owns no extension");
        assert_eq!(jim["firstLine"], r"^#!.*\bjimsh\b");
        assert!(
            language_id_dialects(&langs).contains(&("tcl-jim".to_owned(), "jim".to_owned())),
            "the Jim language id selects the jim environment"
        );
    }

    #[test]
    fn the_selecting_spellings_join_the_identities_in_the_language_id_table() {
        let rows = language_id_dialects(&languages().unwrap());
        for &(id, environment) in EditorLanguageIdentityId::SELECTING {
            assert!(
                rows.contains(&(id.to_owned(), environment.to_owned())),
                "{id}"
            );
        }
        assert_eq!(
            rows.iter().filter(|(id, _)| id == "tcl-apl").count(),
            1,
            "a selecting spelling appears once"
        );
    }

    #[test]
    fn every_language_with_shebang_words_carries_a_first_line_pattern() {
        let langs = languages().unwrap();
        let rendered = render_vscode_package(&committed(VSCODE_PACKAGE), &langs).unwrap();
        for lang in &langs {
            let contributed = manifest_language(&rendered, &lang.id);
            assert_eq!(
                contributed.get("firstLine").is_some(),
                !lang.shebang_words.is_empty(),
                "{}",
                lang.id
            );
        }
        let expect = manifest_language(&rendered, "tcl-expect");
        assert_eq!(expect["firstLine"], r"^#!.*\bexpect\b");
        let tcl86 = manifest_language(&rendered, "tcl86");
        assert_eq!(tcl86["firstLine"], r"^#!.*\b(?:tclsh8\.6|wish8\.6)\b");
    }

    #[test]
    fn every_language_has_a_semantic_token_block() {
        let langs = languages().unwrap();
        let broken = {
            let mut manifest: Value = serde_json::from_str(&committed(VSCODE_PACKAGE)).unwrap();
            manifest["contributes"]["semanticTokenScopes"] = Value::Array(Vec::new());
            serde_json::to_string_pretty(&manifest).unwrap() + "\n"
        };
        let rendered: Value =
            serde_json::from_str(&render_vscode_package(&broken, &langs).unwrap()).unwrap();
        let blocks = rendered["contributes"]["semanticTokenScopes"]
            .as_array()
            .unwrap();
        let block_languages: Vec<&str> = blocks
            .iter()
            .map(|b| b["language"].as_str().unwrap())
            .collect();
        let ids: Vec<&str> = langs.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(block_languages, ids);
        let scopes_of = |id: &str| -> Vec<String> {
            blocks.iter().find(|b| b["language"] == id).unwrap()["scopes"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect()
        };
        for id in [
            "tcl-tmsh",
            "tcl-microchip",
            "tclspec",
            "sslictcl",
            "tcl-jim",
        ] {
            assert!(scopes_of(id).contains(&"regexp".to_owned()), "{id}");
        }
        assert!(scopes_of("tcl-tmsh").contains(&"object".to_owned()));
        assert!(!scopes_of("tcl-jim").contains(&"object".to_owned()));
        assert!(scopes_of("tcl-bigip").contains(&"vlan".to_owned()));
        assert!(scopes_of("tcl-apl").contains(&"aplSection".to_owned()));
    }

    #[test]
    fn a_menu_clause_that_misses_a_language_is_refused_and_repaired() {
        let langs = languages().unwrap();
        let original = committed(VSCODE_PACKAGE);
        let pattern = language_id_pattern(&langs).unwrap();
        let clause = format!("editorLangId =~ /{pattern}/");
        assert!(
            original.contains(&clause),
            "the manifest carries the clause"
        );

        // The clause the manifest carried before: a bare `tcl` prefix, which
        // `sslictcl` does not begin with.
        let narrow = original.replace(&clause, "editorLangId =~ /^tcl/");
        let narrow_manifest: Value = serde_json::from_str(&narrow).unwrap();
        let err = check_when_clauses(&narrow_manifest, &langs)
            .expect_err("a clause missing a language id must fail");
        assert!(format!("{err}").contains("sslictcl"), "{err}");

        assert_eq!(
            render_vscode_package(&narrow, &langs).unwrap(),
            original,
            "the render regenerates every clause"
        );
        let manifest: Value = serde_json::from_str(&original).unwrap();
        check_when_clauses(&manifest, &langs).expect("the committed clauses cover every language");
    }

    #[test]
    fn the_language_id_pattern_groups_ids_by_prefix() {
        let langs = languages().unwrap();
        assert_eq!(language_id_pattern(&langs).unwrap(), "^(?:sslictcl|tcl)");
    }

    #[test]
    fn a_zed_directory_absent_from_the_table_is_an_error() {
        // A directory the table does not name.
        let dir =
            std::env::temp_dir().join(format!("tcl-xtask-zed-{}-{}", std::process::id(), line!()));
        let languages = dir.join(ZED_LANGUAGES_DIR);
        fs::create_dir_all(languages.join("nonesuch")).unwrap();
        fs::write(languages.join("nonesuch/config.toml"), "name = \"X\"\n").unwrap();
        let err =
            verify_every_zed_directory_is_wired(&dir).expect_err("an unnamed directory must fail");
        assert!(format!("{err}").contains("nonesuch"), "{err}");
        fs::remove_dir_all(dir).unwrap();
    }

    /// The generator's *coverage* — which files it owns at all.
    ///
    /// The drift gate is blind here: a target dropped from the list leaves its
    /// committed file matching itself, so a projection can silently stop being
    /// generated without anything failing. Naming the roster explicitly is
    /// what makes that deletion a test failure, and the list is short enough
    /// that a reviewer can check it against the surfaces that exist.
    #[test]
    fn every_generated_surface_has_a_render_target() {
        let mut covered: Vec<String> = render_targets().into_iter().map(|(rel, _)| rel).collect();
        covered.sort_unstable();
        covered.dedup();

        let mut expected: Vec<String> = [
            VSCODE_PACKAGE,
            VSCODE_LANGUAGE_IDS,
            JETBRAINS_PLUGIN,
            JETBRAINS_FILETYPE,
            JETBRAINS_TEXTMATE,
            JETBRAINS_RECONCILER,
            SUBLIME_PLUGIN,
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        expected.extend(ZED_LANGUAGES.iter().map(zed_config_path));
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
    /// a CLI check — this is what catches a Zed directory dropping out of
    /// [`ZED_LANGUAGES`] and being left with nothing that generates it.
    #[test]
    fn every_zed_directory_on_disk_is_generated() {
        verify_every_zed_directory_is_wired(&repo_root())
            .expect("every Zed language directory must be one the generator owns");
    }

    /// The contributed basename axis has to match any casing, or a
    /// `BIGIP.CONF` opens as plaintext on a case-sensitive filesystem and never
    /// even activates the extension.
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
        let bigip = manifest_language(&rendered, "tcl-bigip");
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

    #[test]
    fn the_zed_tcl_language_recognises_every_shebang_word_without_its_own_directory() {
        let langs = languages().unwrap();
        let tcl = ZED_LANGUAGES.iter().find(|z| z.dir == "tcl").unwrap();
        let rendered = render_zed_language(&committed(&zed_config_path(tcl)), &langs, tcl).unwrap();
        assert!(
            rendered.contains("first_line_pattern = \"^#!.*(?:jimsh|tclsh|wish)\""),
            "{rendered}"
        );
        let expect = ZED_LANGUAGES.iter().find(|z| z.dir == "expect").unwrap();
        let rendered =
            render_zed_language(&committed(&zed_config_path(expect)), &langs, expect).unwrap();
        assert!(
            rendered.contains("first_line_pattern = \"^#!.*(?:expect)\""),
            "{rendered}"
        );
    }

    #[test]
    fn the_irule_language_id_is_read_from_the_registry() {
        let langs = languages().unwrap();
        let original = committed(JETBRAINS_RECONCILER);
        let broken = original.replace(
            "private const val IRULE_LANGUAGE_ID = \"tcl-irule\"",
            "private const val IRULE_LANGUAGE_ID = \"tcl-stale\"",
        );
        assert_ne!(broken, original);
        assert_eq!(
            render_jetbrains_reconciler(&broken, &langs).unwrap(),
            original
        );
    }
}
