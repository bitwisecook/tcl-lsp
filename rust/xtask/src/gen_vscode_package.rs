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

//! Generate the `tclLsp.*` configuration sections of `editors/vscode/package.json`
//! from the `DiagCode` catalogue + the formatter settings catalogue — the
//! native successor to the `package.json` half of `scripts/codegen/editor_settings.py`.
//!
//! The manifest is parsed, its generated `tclLsp.*` sections are rebuilt, and
//! it is re-serialised with stock `serde_json::to_string_pretty` — which is
//! byte-identical to the committed 2-space-indented manifest, so the
//! hand-written sections (`General`, `Features`, `Runtime Validation`, `AI`, …)
//! round-trip untouched and only the `Formatting — *` / `Diagnostics — *` /
//! `Optimiser` sections change.
//!
//! The `workspaceContains:` activation glob is generated the same way, from
//! `tcl_registry::dialects::TCL_SOURCE_EXTENSIONS` — the same list the server
//! indexes and watches — so the manifest cannot drift from the extension set
//! again (issue #1242).

use std::process::ExitCode;

use anyhow::{Context, Result};
use serde_json::{Map, Value, json};
use tcl_core_types::{DiagCode, DocRow};

use crate::util::repo_root;

const PACKAGE_JSON: &str = "editors/vscode/package.json";

/// The section grouping (key, title) in table order — mirrors
/// `shared/codes.py::SECTIONS`; the three `irules*` keys share the iRules title.
const SECTIONS: &[(&str, &str)] = &[
    ("error", "Diagnostics — Errors"),
    ("warning", "Diagnostics — Style & Best Practice"),
    ("variable", "Diagnostics — Variables"),
    ("security", "Diagnostics — Security"),
    ("hint", "Diagnostics — Hints"),
    ("shimmer", "Diagnostics — Shimmer"),
    ("taint", "Diagnostics — Taint"),
    ("irules", "Diagnostics — iRules"),
    ("irules_security", "Diagnostics — iRules"),
    ("irules_variable", "Diagnostics — iRules"),
    ("bigip", "Diagnostics — BIG-IP Configuration"),
    ("tclpkg", "Diagnostics — Package Manager"),
];

/// A user-configurable diagnostic row: `(code, default_on, description)`.
type DiagRow = (&'static str, bool, &'static str);

/// A formatter setting: `field_name` (`snake_case`), VS Code default value,
/// json-type, description, `enum` values, `enum` descriptions, minimum, maximum.
struct FmtSetting {
    field: &'static str,
    default: Value,
    json_type: &'static str,
    description: &'static str,
    enum_values: &'static [&'static str],
    enum_descriptions: &'static [&'static str],
    minimum: Option<i64>,
    maximum: Option<i64>,
}

impl FmtSetting {
    /// Baseline for struct-update construction: no enum choices and no numeric
    /// bounds, so each catalogue entry states only the fields that differ.
    const DEFAULT: Self = FmtSetting {
        field: "",
        default: Value::Null,
        json_type: "",
        description: "",
        enum_values: &[],
        enum_descriptions: &[],
        minimum: None,
        maximum: None,
    };
}

/// `snake_case` → `camelCase` for VS Code setting keys.
fn snake_to_camel(name: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in name.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// The scope for a generated `tclLsp.*` key (multi-folder workspaces reject
/// `window`-scoped settings in folder settings).
fn scope_for(key: &str) -> Option<&'static str> {
    const PREFIXES: &[(&str, &str)] = &[
        ("tclLsp.formatting.", "language-overridable"),
        ("tclLsp.style.", "language-overridable"),
        ("tclLsp.diagnosticSeverity.", "resource"),
        ("tclLsp.diagnostics.", "resource"),
        ("tclLsp.optimiser.", "resource"),
        ("tclLsp.shimmer.", "resource"),
    ];
    PREFIXES
        .iter()
        .find(|(p, _)| key.starts_with(p))
        .map(|(_, s)| *s)
}

/// Build one property object, prepending `scope` as the first key.
fn scoped_prop(key: &str, mut schema: Map<String, Value>) -> Value {
    if let Some(scope) = scope_for(key) {
        let mut with_scope = Map::new();
        with_scope.insert("scope".to_owned(), Value::String(scope.to_owned()));
        with_scope.append(&mut schema);
        Value::Object(with_scope)
    } else {
        Value::Object(schema)
    }
}

/// Render a formatter setting's VS Code schema (key order: type, default,
/// markdownDescription, [enum], [enumDescriptions], [minimum], [maximum],
/// order).
fn fmt_schema(s: &FmtSetting, order: usize) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("type".to_owned(), json!(s.json_type));
    m.insert("default".to_owned(), s.default.clone());
    m.insert("markdownDescription".to_owned(), json!(s.description));
    if !s.enum_values.is_empty() {
        m.insert("enum".to_owned(), json!(s.enum_values));
    }
    if !s.enum_descriptions.is_empty() {
        m.insert("enumDescriptions".to_owned(), json!(s.enum_descriptions));
    }
    if let Some(min) = s.minimum {
        m.insert("minimum".to_owned(), json!(min));
    }
    if let Some(max) = s.maximum {
        m.insert("maximum".to_owned(), json!(max));
    }
    m.insert("order".to_owned(), json!(order));
    m
}

/// The 6 formatter sections. `order` starts at 2 (after `General`/`Features`).
fn formatter_sections() -> Vec<Value> {
    let sections = formatter_catalogue();
    sections
        .iter()
        .enumerate()
        .map(|(idx, (title, settings))| {
            let mut props = Map::new();
            for (i, s) in settings.iter().enumerate() {
                let key = format!("tclLsp.formatting.{}", snake_to_camel(s.field));
                props.insert(key.clone(), scoped_prop(&key, fmt_schema(s, i)));
            }
            json!({ "title": title, "order": 2 + idx, "properties": Value::Object(props) })
        })
        .collect()
}

/// User-configurable diagnostics grouped by section key.
fn diags_by_section() -> Vec<(&'static str, Vec<DiagRow>)> {
    let mut out: Vec<(&'static str, Vec<DiagRow>)> =
        SECTIONS.iter().map(|(k, _)| (*k, Vec::new())).collect();
    for c in DiagCode::ALL {
        if let DocRow::Diagnostic {
            section,
            default_on,
            internal: false,
            reserved: false,
            description,
            tag: _,
        } = c.doc_row()
        {
            let key = section.as_str();
            if let Some(entry) = out.iter_mut().find(|(k, _)| *k == key) {
                entry.1.push((c.as_str(), default_on, description));
            }
        }
    }
    for (_, v) in &mut out {
        v.sort_by_key(|(code, _, _)| *code);
    }
    out
}

/// Build the diagnostic sections (grouped by shared title, with the special
/// injected non-diagnostic properties).
fn diagnostic_sections() -> Vec<Value> {
    let by_section = diags_by_section();
    let base = 2 + formatter_catalogue().len();

    // Unique titles in declaration order + their `order` (base + first index).
    let mut ordered_titles: Vec<&str> = Vec::new();
    let mut title_order: Vec<(&str, usize)> = Vec::new();
    for (idx, (_, title)) in SECTIONS.iter().enumerate() {
        if !ordered_titles.contains(title) {
            ordered_titles.push(title);
            title_order.push((title, base + idx));
        }
    }

    let mut result = Vec::new();
    for title in &ordered_titles {
        // All diagnostics whose section maps to this title, in SECTIONS order.
        let diags: Vec<(&str, bool, &str)> = SECTIONS
            .iter()
            .filter(|(_, t)| t == title)
            .flat_map(|(k, _)| {
                by_section
                    .iter()
                    .find(|(sk, _)| sk == k)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default()
            })
            .collect();
        if diags.is_empty() {
            continue;
        }

        let mut props = Map::new();
        inject_special_leading_props(title, &mut props);
        for (i, (code, default_on, description)) in diags.iter().enumerate() {
            let key = format!("tclLsp.diagnostics.{code}");
            let mut m = Map::new();
            m.insert("type".to_owned(), json!("boolean"));
            m.insert("default".to_owned(), json!(default_on));
            m.insert(
                "markdownDescription".to_owned(),
                json!(format!("**{code}:** {description}")),
            );
            m.insert("order".to_owned(), json!(i));
            props.insert(key.clone(), scoped_prop(&key, m));
            // Per-code severity override — how prominently the editor renders
            // this diagnostic. `default` keeps the analyser's emitted severity
            // (issue #941: raise a subtle hint like W211 to a warning/error).
            let sev_key = format!("tclLsp.diagnosticSeverity.{code}");
            let mut sm = Map::new();
            sm.insert("type".to_owned(), json!("string"));
            sm.insert("default".to_owned(), json!("default"));
            sm.insert(
                "enum".to_owned(),
                json!(["default", "error", "warning", "information", "hint"]),
            );
            sm.insert(
                "enumDescriptions".to_owned(),
                json!([
                    "Use the analyser's built-in severity",
                    "Report as an error",
                    "Report as a warning",
                    "Report as information",
                    "Report as a hint (faded)",
                ]),
            );
            sm.insert(
                "markdownDescription".to_owned(),
                json!(format!("Display severity for **{code}:** {description}")),
            );
            sm.insert("order".to_owned(), json!(i));
            props.insert(sev_key.clone(), scoped_prop(&sev_key, sm));
        }
        inject_special_trailing_props(title, diags.len(), &mut props);

        let order = title_order.iter().find(|(t, _)| t == title).unwrap().1;
        result.push(json!({ "title": title, "order": order, "properties": Value::Object(props) }));
    }
    result
}

/// The optimiser section.
fn optimiser_section(order: usize) -> Value {
    let mut opts: Vec<(&str, &str)> = DiagCode::ALL
        .iter()
        .filter_map(|c| match c.doc_row() {
            DocRow::Optimisation { description, .. } => Some((c.as_str(), description)),
            DocRow::Diagnostic { .. } => None,
        })
        .collect();
    opts.sort_by_key(|(code, _)| *code);

    let mut props = Map::new();
    props.insert(
        "tclLsp.optimiser.enabled".to_owned(),
        scoped_prop(
            "tclLsp.optimiser.enabled",
            json!({
                "type": "boolean",
                "default": true,
                "description": "Enable optimiser suggestions as diagnostics.",
                "order": 0,
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    );
    props.insert(
        "tclLsp.optimiser.profile".to_owned(),
        scoped_prop(
            "tclLsp.optimiser.profile",
            json!({
                "type": "string",
                "enum": ["off", "readability", "standard", "full", "aggressive"],
                "default": "readability",
                "enumDescriptions": [
                    "All optimisations disabled.",
                    "Readability improvements only — idiomatic rewrites, no code removal.",
                    "Readability + constant folding and pattern recognition.",
                    "All optimisations enabled (single pass).",
                    "All optimisations with multi-pass to fixpoint.",
                ],
                "markdownDescription": "Optimisation profile controlling which passes run as diagnostics. Individual `O1xx` toggles below override the profile when explicitly set.",
                "order": 1,
            })
            .as_object()
            .unwrap()
            .clone(),
        ),
    );
    for (i, (code, description)) in opts.iter().enumerate() {
        let key = format!("tclLsp.optimiser.{code}");
        let mut m = Map::new();
        m.insert("type".to_owned(), json!(["boolean", "null"]));
        m.insert("default".to_owned(), Value::Null);
        m.insert(
            "markdownDescription".to_owned(),
            json!(format!(
                "**{code}:** {description} (`null` = inherit from profile)"
            )),
        );
        m.insert("order".to_owned(), json!(i + 2));
        props.insert(key.clone(), scoped_prop(&key, m));
    }
    json!({ "title": "Optimiser", "order": order, "properties": Value::Object(props) })
}

/// Special properties prepended before the per-diagnostic toggles.
fn inject_special_leading_props(title: &str, props: &mut Map<String, Value>) {
    match title {
        "Diagnostics — Errors" => {
            let k = "tclLsp.diagnostics.exclude";
            props.insert(k.to_owned(), scoped_prop(k, json!({
                "type": "array",
                "items": {"type": "string"},
                "default": [],
                "markdownDescription": "Glob patterns for files that produce **no diagnostics at all** (#1556). A pattern containing `/` matches the workspace-folder-relative path (`generated/**`); one without matches the file name at any depth, gitignore-style (`*.gen.tcl`). Supports `*`, `?`, `[a-c]` classes, `**`, and `{a,b}` alternation. Every other feature (hover, navigation, formatting, indexing) keeps working on excluded files. Also configurable via the `[diagnostics] exclude` key of the layered config file (see docs/kcs/kcs-howto-exclude-files-from-diagnostics.md).",
                "order": -1,
            }).as_object().unwrap().clone()));
        }
        "Diagnostics — Style & Best Practice" => {
            let k1 = "tclLsp.style.lineLength";
            props.insert(k1.to_owned(), scoped_prop(k1, json!({
                "type": "integer",
                "default": 120,
                "minimum": 40,
                "markdownDescription": "Maximum line length for the **W111** diagnostic. Lines exceeding this limit are flagged.",
                "order": -2,
            }).as_object().unwrap().clone()));
            let k2 = "tclLsp.style.nonAscii";
            props.insert(k2.to_owned(), scoped_prop(k2, json!({
                "type": "string",
                "default": "confusables",
                "enum": ["strict", "confusables", "common", "off"],
                "enumDescriptions": [
                    "Flag every non-ASCII character (most restrictive). Default for iRules/iApps.",
                    "Flag Unicode confusables (homoglyphs per unicode.org/Public/security confusables.txt) plus copy-paste artifacts. Default for Tcl.",
                    "Allow intentional Unicode (letters, digits, symbols in any script); only flag confusables and control characters.",
                    "Disable W108 entirely.",
                ],
                "markdownDescription": "Controls how **W108** (non-ASCII character) is detected.\n\n- **strict**: flag all non-ASCII characters (default for iRules/iApps).\n- **confusables** *(default for Tcl)*: flag Unicode confusables (homoglyphs from the Unicode security spec) and copy-paste artifacts.\n- **common**: allow Unicode letters, symbols, and punctuation; flag only confusables and control characters.\n- **off**: disable W108.",
                "order": -1,
            }).as_object().unwrap().clone()));
        }
        "Diagnostics — Shimmer" => {
            let k = "tclLsp.shimmer.enabled";
            props.insert(k.to_owned(), scoped_prop(k, json!({
                "type": "boolean",
                "default": true,
                "markdownDescription": "Enable shimmer detection (internal representation changes). When disabled, all S-series diagnostics are suppressed.",
                "order": -1,
            }).as_object().unwrap().clone()));
        }
        _ => {}
    }
}

/// The trailing special property appended after the iRules toggles.
fn inject_special_trailing_props(title: &str, ndiags: usize, props: &mut Map<String, Value>) {
    if title == "Diagnostics — iRules" {
        let k = "tclLsp.diagnostics.genericVariablePatterns";
        props.insert(k.to_owned(), scoped_prop(k, json!({
            "type": "array",
            "items": {"type": "string"},
            "default": [],
            "markdownDescription": "Regex patterns for generic `static::` variable names (IRULE4002). Each pattern is matched case-insensitively against the bare name after stripping `static::`. Also configurable via the tcl-lsp config file (see docs/kcs/kcs-xdg-config.md).",
            "order": ndiags,
        }).as_object().unwrap().clone()));
    }
}

/// Whether `title` is one of the regenerated (generated) section titles.
fn is_generated_title(title: &str) -> bool {
    title == "Optimiser"
        || formatter_catalogue().iter().any(|(t, _)| *t == title)
        || SECTIONS.iter().any(|(_, t)| *t == title)
}

/// Rebuild the manifest text with the regenerated sections spliced in.
fn rebuild() -> Result<String> {
    let path = repo_root().join(PACKAGE_JSON);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut manifest: Value = serde_json::from_str(&text).context("parsing package.json")?;

    let groups = manifest
        .get_mut("contributes")
        .and_then(|c| c.get_mut("configuration"))
        .and_then(Value::as_array_mut)
        .context("contributes.configuration is not an array")?;

    // Find the contiguous run of generated sections (first..=last by title).
    let first = groups
        .iter()
        .position(|g| {
            g.get("title")
                .and_then(Value::as_str)
                .is_some_and(is_generated_title)
        })
        .context("no generated sections found in package.json")?;
    let last = groups
        .iter()
        .rposition(|g| {
            g.get("title")
                .and_then(Value::as_str)
                .is_some_and(is_generated_title)
        })
        .unwrap();

    let mut regenerated = formatter_sections();
    regenerated.extend(diagnostic_sections());
    let next_order = regenerated
        .iter()
        .filter_map(|s| s.get("order").and_then(Value::as_u64))
        .max()
        .map_or(7, |m| usize::try_from(m).unwrap_or(usize::MAX) + 1);
    regenerated.push(optimiser_section(next_order));

    groups.splice(first..=last, regenerated);

    set_workspace_contains(&mut manifest)?;

    // Stock `serde_json` 2-space pretty-printing is byte-identical to the
    // committed manifest (a `to_string_pretty` round-trip is a no-op on the
    // hand-written sections), so no custom serialiser is needed.
    Ok(serde_json::to_string_pretty(&manifest)? + "\n")
}

/// The one `workspaceContains:` activation event, naming exactly the
/// extensions the server indexes, case-folded per character.
///
/// `workspaceContains` decides whether the extension activates on a workspace
/// **scan** — without it a workspace whose only Tcl files are `.impl` / `.exp`
/// / `.apl`, or upper-cased on Linux, gets no index and no workspace symbols
/// until a file is opened (`onLanguage:` covers only the opened-file path).
fn workspace_contains_event() -> String {
    format!(
        "workspaceContains:{}",
        tcl_registry::dialects::tcl_source_glob_any_case()
    )
}

/// Replace the manifest's `workspaceContains:` activation event with the
/// generated one, leaving the hand-written `onLanguage:` /
/// `onChatParticipant:` entries in place and in order.
fn set_workspace_contains(manifest: &mut Value) -> Result<()> {
    let events = manifest
        .get_mut("activationEvents")
        .and_then(Value::as_array_mut)
        .context("activationEvents is not an array")?;
    let generated = Value::String(workspace_contains_event());
    match events.iter().position(|e| {
        e.as_str()
            .is_some_and(|s| s.starts_with("workspaceContains:"))
    }) {
        Some(idx) => events[idx] = generated,
        None => events.push(generated),
    }
    Ok(())
}

/// Write (or, with `check`, verify) the regenerated `package.json`.
pub fn run(check: bool) -> Result<ExitCode> {
    let path = repo_root().join(PACKAGE_JSON);
    let content = rebuild()?;
    if check {
        let current = std::fs::read_to_string(&path)?;
        if current != content {
            eprintln!(
                "{PACKAGE_JSON} `tclLsp.*` sections are stale — run `cargo xtask gen-vscode-package`."
            );
            return Ok(ExitCode::from(1));
        }
        eprintln!("OK: {PACKAGE_JSON} generated sections are in sync with the registries.");
    } else {
        std::fs::write(&path, &content)?;
        eprintln!("wrote {PACKAGE_JSON}");
    }
    Ok(ExitCode::SUCCESS)
}

/// The formatter settings catalogue (6 sections, 26 settings), mirroring
/// `tooling/formatter/config.py::FORMATTER_SETTINGS_CATALOGUE`. Defaults are the
/// VS Code-resolved form of `FormatterConfig::default()` (enums lowercased,
/// `"\n"` → `"lf"`).
fn formatter_catalogue() -> Vec<(&'static str, Vec<FmtSetting>)> {
    vec![
        fmt_group_indentation(),
        fmt_group_braces_and_style(),
        fmt_group_line_length(),
        fmt_group_whitespace_and_comments(),
        fmt_group_blank_lines_and_file_format(),
        fmt_group_docstrings(),
    ]
}

fn fmt_group_indentation() -> (&'static str, Vec<FmtSetting>) {
    (
        "Formatting — Indentation",
        vec![
            FmtSetting {
                field: "indent_size",
                default: json!(4),
                json_type: "integer",
                description: "Number of spaces per indentation level.",
                minimum: Some(1),
                maximum: Some(16),
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "indent_style",
                default: json!("spaces"),
                json_type: "string",
                description: "Use spaces or tabs for indentation.",
                enum_values: &["spaces", "tabs"],
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "continuation_indent",
                default: json!(4),
                json_type: "integer",
                description: "Extra indentation for continuation lines.",
                minimum: Some(1),
                maximum: Some(16),
                ..FmtSetting::DEFAULT
            },
        ],
    )
}

fn fmt_group_braces_and_style() -> (&'static str, Vec<FmtSetting>) {
    (
        "Formatting — Braces & Style",
        vec![
            FmtSetting {
                field: "brace_style",
                default: json!("k_and_r"),
                json_type: "string",
                description: "Brace placement style. K&R places the opening brace at the end of the line.",
                enum_values: &["k_and_r"],
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "space_between_braces",
                default: json!(true),
                json_type: "boolean",
                description: "Insert spaces inside single-line braces: `{ body }` instead of `{body}`.",
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "enforce_braced_variables",
                default: json!(false),
                json_type: "boolean",
                description: "Rewrite `$var` as `${var}` for consistency.",
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "enforce_braced_expr",
                default: json!(false),
                json_type: "boolean",
                description: "Rewrite unbraced `expr` arguments as braced expressions.",
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "expand_abbreviations",
                default: json!(true),
                json_type: "boolean",
                description: "Expand unique-prefix keyword abbreviations to their canonical spellings (`string le` becomes `string length`, `lsearch -noc` becomes `lsearch -nocase`). Ambiguous and unknown words are always left alone.",
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "boolean_form",
                default: json!("true/false"),
                json_type: "string",
                description: "Spelling every boolean-consumed word is normalised to. Only consumption sites the registry proves boolean are rewritten, never a value-definition site such as `set flag yes`.",
                enum_values: &["true/false", "yes/no", "on/off", "0/1", "preserve"],
                ..FmtSetting::DEFAULT
            },
        ],
    )
}

fn fmt_group_line_length() -> (&'static str, Vec<FmtSetting>) {
    (
        "Formatting — Line Length & Wrapping",
        vec![
            FmtSetting {
                field: "max_line_length",
                default: json!(120),
                json_type: "integer",
                description: "Hard limit for line length. Lines exceeding this are wrapped.",
                minimum: Some(40),
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "goal_line_length",
                default: json!(100),
                json_type: "integer",
                description: "Soft target for line length. The formatter prefers to stay within this limit.",
                minimum: Some(40),
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "expand_single_line_bodies",
                default: json!(false),
                json_type: "boolean",
                description: "Expand single-line command bodies onto multiple lines.",
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "min_body_commands_for_expansion",
                default: json!(2),
                json_type: "integer",
                description: "Minimum commands in a body before it is expanded to multiple lines.",
                minimum: Some(1),
                ..FmtSetting::DEFAULT
            },
        ],
    )
}

fn fmt_group_whitespace_and_comments() -> (&'static str, Vec<FmtSetting>) {
    (
        "Formatting — Whitespace & Comments",
        vec![
            FmtSetting {
                field: "space_after_comment_hash",
                default: json!(true),
                json_type: "boolean",
                description: "Ensure a space after `#` in comments.",
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "trim_trailing_whitespace",
                default: json!(true),
                json_type: "boolean",
                description: "Remove trailing whitespace from lines.",
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "align_comments_to_code",
                default: json!(true),
                json_type: "boolean",
                description: "Align inline comments to a consistent column.",
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "replace_semicolons_with_newlines",
                default: json!(true),
                json_type: "boolean",
                description: "Replace `;` command separators with newlines.",
                ..FmtSetting::DEFAULT
            },
        ],
    )
}

fn fmt_group_blank_lines_and_file_format() -> (&'static str, Vec<FmtSetting>) {
    (
        "Formatting — Blank Lines & File Format",
        vec![
            FmtSetting {
                field: "blank_lines_between_procs",
                default: json!(1),
                json_type: "integer",
                description: "Number of blank lines between proc definitions.",
                minimum: Some(0),
                maximum: Some(5),
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "blank_lines_between_blocks",
                default: json!(1),
                json_type: "integer",
                description: "Number of blank lines between top-level blocks.",
                minimum: Some(0),
                maximum: Some(5),
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "max_consecutive_blank_lines",
                default: json!(2),
                json_type: "integer",
                description: "Maximum number of consecutive blank lines to keep.",
                minimum: Some(1),
                maximum: Some(10),
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "line_ending",
                default: json!("auto"),
                json_type: "string",
                description: "Line ending style for formatted output, and for newlines inserted by code actions.",
                enum_values: &["auto", "lf", "crlf", "cr"],
                enum_descriptions: &[
                    "Keep the line ending the file already uses.",
                    "Line feed (\\n).",
                    "Carriage return + line feed (\\r\\n).",
                    "Carriage return (\\r).",
                ],
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "ensure_final_newline",
                default: json!(true),
                json_type: "boolean",
                description: "Ensure the file ends with a newline character.",
                ..FmtSetting::DEFAULT
            },
        ],
    )
}

fn fmt_group_docstrings() -> (&'static str, Vec<FmtSetting>) {
    (
        "Formatting — Docstrings",
        vec![
            FmtSetting {
                field: "docstring_style",
                default: json!("none"),
                json_type: "string",
                description: "Where docstrings are placed relative to `proc` definitions. Set to `none` to leave existing docstrings as-is.",
                enum_values: &["preceding", "body", "none"],
                enum_descriptions: &[
                    "Comment block above the proc statement.",
                    "Comment block at the start of the proc body.",
                    "Do not generate or reformat docstrings.",
                ],
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "docstring_tag_style",
                default: json!("doxygen"),
                json_type: "string",
                description: "Tag format used in docstrings for parameter and return documentation.",
                enum_values: &["doxygen", "plain", "none"],
                enum_descriptions: &[
                    "Doxygen-style tags: @param, @return, @brief.",
                    "Plain prose with an Arguments: section.",
                    "Leave tag format as-is.",
                ],
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "docstring_decoration",
                default: json!(false),
                json_type: "boolean",
                description: "Add decoration border lines (e.g. `# ......`) around docstrings.",
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "docstring_decoration_char",
                default: json!("."),
                json_type: "string",
                description: "Character used for docstring decoration borders.",
                enum_values: &[".", "-", "=", "*", "~"],
                ..FmtSetting::DEFAULT
            },
            FmtSetting {
                field: "docstring_decoration_width",
                default: json!(70),
                json_type: "integer",
                description: "Width of docstring decoration border lines.",
                minimum: Some(20),
                maximum: Some(120),
                ..FmtSetting::DEFAULT
            },
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    /// Drift guard: the committed manifest's `tclLsp.*` sections must equal what
    /// the registries render (and every other section round-trips untouched).
    #[test]
    fn committed_package_json_matches_generated() {
        let path = repo_root().join(PACKAGE_JSON);
        let current = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        assert_eq!(
            current,
            rebuild().expect("rebuild"),
            "{PACKAGE_JSON} generated sections are stale — run `cargo xtask gen-vscode-package`"
        );
    }

    /// The activation glob names **every** indexed extension, and does so
    /// case-insensitively — the two ways the hand-written glob had drifted
    /// (issue #1242: nine of twelve, single-cased).
    #[test]
    fn workspace_contains_event_covers_every_indexed_extension_in_any_case() {
        let event = workspace_contains_event();
        for ext in tcl_registry::dialects::TCL_SOURCE_EXTENSIONS {
            let mut folded = String::new();
            for c in ext.chars() {
                let _ = write!(
                    folded,
                    "[{}{}]",
                    c.to_ascii_lowercase(),
                    c.to_ascii_uppercase()
                );
            }
            assert!(
                event.contains(&folded),
                "activation glob omits {ext} (expected {folded}): {event}"
            );
        }
        assert!(event.starts_with("workspaceContains:**/*.{"));
    }

    #[test]
    fn snake_to_camel_works() {
        assert_eq!(snake_to_camel("indent_size"), "indentSize");
        assert_eq!(
            snake_to_camel("max_consecutive_blank_lines"),
            "maxConsecutiveBlankLines"
        );
    }
}
