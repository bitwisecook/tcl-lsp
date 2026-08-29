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

//! Generate the shared diagnostic category catalogue from `DiagCode`, package
//! identical copies for AI prompts and MCP, and render the prompt/skill files.
//!
//! The diagnostic → AI-category classification is `ai_category` override
//! (per-code) else the section's default category; the small metadata tables
//! (overrides, section→category, conversion labels, prefix rules) are carried
//! here — they are AI-generator-specific, not core-registry data. Emitted with
//! stock `serde_json` 2-space pretty-printing.

use std::fmt::Write as _;
use std::process::ExitCode;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use tcl_core_types::{DiagCode, DocRow, OptCategory};

use crate::util::repo_root;

const DIAGNOSTICS_JSON: &str = "ai/shared/diagnostics.json";
/// The native MCP binary packages the same catalogue inside its crate. It is
/// generated from the identical content so category membership cannot drift.
const MCP_DIAGNOSTICS_JSON: &str = "rust/tcl-mcp/diagnostics.json";
/// Which prompt fragment covers which dialect. Hand-maintained (a fragment's
/// audience is an editorial choice), gated below against the profile catalogue.
const PROMPT_MANIFEST: &str = "ai/prompts/manifest.json";
/// The VS Code chat surfaces' dialect table, projected from the profile
/// catalogue so `/help` names every dialect the server accepts.
const VSCODE_DIALECT_CATALOG: &str = "editors/vscode/src/chat/dialectCatalog.ts";

/// AI prompt/skill templates: `(template, output)` relative paths.
const AI_TEMPLATES: &[(&str, &str)] = &[
    ("ai/prompts/tcl_system.md.j2", "ai/prompts/tcl_system.md"),
    (
        "ai/prompts/irules_system.md.j2",
        "ai/prompts/irules_system.md",
    ),
    (
        "ai/prompts/tcl_system.md.j2",
        "ai/claude/skills/_prompts/tcl_system.md",
    ),
    (
        "ai/prompts/irules_system.md.j2",
        "ai/claude/skills/_prompts/irules_system.md",
    ),
    (
        "ai/claude/skills/tcl-optimise/SKILL.md.j2",
        "ai/claude/skills/tcl-optimise/SKILL.md",
    ),
    (
        "ai/claude/skills/irule-optimise/SKILL.md.j2",
        "ai/claude/skills/irule-optimise/SKILL.md",
    ),
];

/// AI categories in output order: `(key, label)`.
const CATEGORY_DEFS: &[(&str, &str)] = &[
    ("error", "Errors"),
    ("security", "Security"),
    ("taint", "Taint Analysis"),
    ("thread_safety", "Thread Safety"),
    ("control_flow", "Control Flow"),
    ("performance", "Performance"),
    ("style", "Style & Best Practice"),
    ("optimiser", "Optimiser"),
    ("irules", "iRules"),
    ("tk", "Tk GUI"),
];

/// Default AI category per diagnostic section (unmapped sections → `style`).
fn section_to_category(section: &str) -> &'static str {
    match section {
        "error" => "error",
        "security" | "irules_security" => "security",
        "shimmer" => "performance",
        "taint" => "taint",
        "irules" | "bigip" => "irules",
        "irules_variable" => "thread_safety",
        // warning / variable / hint / tclpkg / tk → style (tk is overridden below).
        _ => "style",
    }
}

/// Per-code AI-category overrides.
fn ai_category_override(code: &str) -> Option<&'static str> {
    Some(match code {
        "W100" => "security",
        "W230" | "W231" | "W232" | "W233" | "W240" | "W241" | "W242" | "IRULE1005"
        | "IRULE1006" | "IRULE1007" | "IRULE1008" | "IRULE1201" | "IRULE1202" => "control_flow",
        "W302" | "W304" | "IRULE2001" => "style",
        "IRULE2101" | "IRULE5001" => "performance",
        _ => return None,
    })
}

/// The convertible diagnostics in section-then-code order: `(code, label)`.
/// Mirrors `misc::conversion_for`.
const CONVERSION_MAP: &[(&str, &str)] = &[
    ("W100", "Unbraced expr -> braced expr"),
    ("W104", "String concat for lists -> lappend"),
    ("W110", "== / != for strings -> eq / ne"),
    ("W304", "Missing -- option terminator -> add --"),
    ("IRULE2001", "Deprecated matchclass -> class match"),
    ("IRULE5001", "Ungated log in hot event -> add debug gating"),
];

/// Build the `categories` array.
fn categories() -> Value {
    // Classify every user-configurable diagnostic.
    let mut by_cat: Vec<(&str, Vec<&str>)> = CATEGORY_DEFS
        .iter()
        .map(|(k, _)| (*k, Vec::new()))
        .collect();
    for c in DiagCode::ALL {
        if let DocRow::Diagnostic {
            section,
            internal: false,
            reserved: false,
            ..
        } = c.doc_row()
        {
            let cat = ai_category_override(c.as_str())
                .unwrap_or_else(|| section_to_category(section.as_str()));
            if let Some(entry) = by_cat.iter_mut().find(|(k, _)| *k == cat) {
                entry.1.push(c.as_str());
            }
        }
    }
    // `tk` is always exactly the three TK codes (they are internal, so not
    // picked up above).
    if let Some(entry) = by_cat.iter_mut().find(|(k, _)| *k == "tk") {
        entry.1 = vec!["TK1001", "TK1002", "TK1003"];
    }
    for (_, codes) in &mut by_cat {
        codes.sort_unstable();
    }

    Value::Array(
        CATEGORY_DEFS
            .iter()
            .map(|(key, label)| {
                let codes = by_cat
                    .iter()
                    .find(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                json!({ "key": key, "label": label, "codes": codes })
            })
            .collect(),
    )
}

/// Render the full `diagnostics.json`.
fn diagnostics_json() -> String {
    let conversion_map: serde_json::Map<String, Value> = CONVERSION_MAP
        .iter()
        .map(|(c, l)| ((*c).to_owned(), json!(l)))
        .collect();
    let convertible: Vec<&str> = CONVERSION_MAP.iter().map(|(c, _)| *c).collect();

    let root = json!({
        "categories": categories(),
        "convertible_codes": convertible,
        "conversion_map": Value::Object(conversion_map),
        "prefix_rules": [
            { "prefix": "E", "category": "error" },
            { "prefix": "O", "category": "optimiser" },
            { "prefix": "S", "category": "performance" },
            { "prefix": "IRULE", "category": "irules" },
            { "prefix": "TK", "category": "tk" },
        ],
        "default_category": "style",
        "review_categories": ["security", "taint", "thread_safety"],
        "irules_event_pattern": r"^\s*when\s+([A-Z][A-Z0-9_]{2,})\b",
        "irules_event_boundary_owner": "tcl-syntax::event_handler::{event_handlers,event_handlers_with_head_predicate,script_commands} parses supplied script regions; tcl-registry::events::top_level_when_handlers_with_registry_and_head_resolver resolves each top-level head at its document offset before handler acceptance; F5 iRules permits `when` only at top level, and disabled interp/rename/namespace commands never create identity facts; never scan handler text locally",
    });
    serde_json::to_string_pretty(&root).expect("serialise diagnostics.json") + "\n"
}

// AI prompt / skill templates

/// Non-internal diagnostics in `sections` (each section's codes sorted by code,
/// sections concatenated in order) as `(code, desc)` with the trailing `.`
/// stripped from the description.
fn section_list(sections: &[&str]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for &sec in sections {
        let mut rows: Vec<(&str, &str)> = DiagCode::ALL
            .iter()
            .filter_map(|c| match c.doc_row() {
                DocRow::Diagnostic {
                    section,
                    internal: false,
                    reserved: false,
                    description,
                    ..
                } if section.as_str() == sec => Some((c.as_str(), description)),
                _ => None,
            })
            .collect();
        rows.sort_unstable_by_key(|(code, _)| *code);
        out.extend(
            rows.into_iter()
                .map(|(c, d)| (c.to_owned(), d.trim_end_matches('.').to_owned())),
        );
    }
    out
}

/// The optimisations list `(code, desc)` by code.
fn optimisations_list() -> Vec<(String, String)> {
    let mut opts: Vec<(&str, &str)> = DiagCode::ALL
        .iter()
        .filter_map(|c| match c.doc_row() {
            DocRow::Optimisation { description, .. } => Some((c.as_str(), description)),
            DocRow::Diagnostic { .. } => None,
        })
        .collect();
    opts.sort_unstable_by_key(|(code, _)| *code);
    opts.into_iter()
        .map(|(c, d)| (c.to_owned(), d.trim_end_matches('.').to_owned()))
        .collect()
}

/// Comma-joined sorted optimisation codes in `cat` (the `{{ opt_* }}` vars).
fn opt_codes(cat: OptCategory) -> String {
    let mut codes: Vec<&str> = DiagCode::ALL
        .iter()
        .filter_map(|c| match c.doc_row() {
            DocRow::Optimisation { category, .. } if category == cat => Some(c.as_str()),
            _ => None,
        })
        .collect();
    codes.sort_unstable();
    codes.join(", ")
}

/// The template variables: named diagnostic lists + scalar `opt_*` strings.
struct AiContext {
    lists: Vec<(&'static str, Vec<(String, String)>)>,
    scalars: Vec<(&'static str, String)>,
}

fn ai_context() -> AiContext {
    AiContext {
        lists: vec![
            ("errors", section_list(&["error"])),
            ("style", section_list(&["warning", "variable", "hint"])),
            ("security", section_list(&["security"])),
            ("shimmer", section_list(&["shimmer"])),
            ("taint", section_list(&["taint"])),
            ("irules", section_list(&["irules", "bigip"])),
            ("irules_security", section_list(&["irules_security"])),
            ("irules_variable", section_list(&["irules_variable"])),
            ("optimisations", optimisations_list()),
        ],
        scalars: vec![
            ("opt_readability", opt_codes(OptCategory::Readability)),
            (
                "opt_constant_folding",
                opt_codes(OptCategory::ConstantFolding),
            ),
            ("opt_pattern", opt_codes(OptCategory::Pattern)),
            ("opt_dce", opt_codes(OptCategory::Dce)),
            ("opt_code_motion", opt_codes(OptCategory::CodeMotion)),
            ("opt_recursion", opt_codes(OptCategory::Recursion)),
            ("irules_vendor_surface", irules_vendor_surface()),
        ],
    }
}

/// The modelled F5 iRules command surface, summarised from the registry
/// (`ResolvedContext::vendor_command_surface` — dialect-profile model §5.4)
/// so the prompt's picture of the surface regenerates with the data instead
/// of drifting as hand-written prose: total command count plus the largest
/// `NS::` namespaces.
fn irules_vendor_surface() -> String {
    // The dialect name, the store, and the summary all resolve through the
    // one ingress seam (`crate::environment`): the environment's document
    // context answers the vendor-bit summary the retired
    // `ProfileQueries::vendor_surface` did, pinned equal to it for every
    // catalogue profile over that profile's own generation
    // (`ResolvedContext`'s `vendor_surface_matches_the_profile_query`).
    let registry = crate::environment::store_for_dialect("f5-irules");
    let Some(surface) =
        crate::environment::context_for_dialect("f5-irules").vendor_command_surface(registry)
    else {
        return String::new();
    };
    let top: Vec<String> = surface
        .namespaces
        .iter()
        .filter(|(ns, _)| !ns.is_empty())
        .take(12)
        .map(|(ns, n)| format!("{ns}:: ({n})"))
        .collect();
    format!(
        "{} modelled commands; largest namespaces: {}",
        surface.command_count,
        top.join(", ")
    )
}

/// Render one inline `{% for c in NAME %}BODY{% endfor %}` region against
/// `list`. BODY is `{{ c.code }} ({{ c.desc }})` with a
/// `{% if not loop.last %}SEP{% endif %}` separator — the only loop shape the
/// AI templates use.
fn render_loop(body: &str, list: &[(String, String)]) -> String {
    // Split the separator out of the body.
    let (item_tmpl, sep) = match body.split_once("{% if not loop.last %}") {
        Some((item, rest)) => (item, rest.strip_suffix("{% endif %}").unwrap_or(rest)),
        None => (body, ""),
    };
    let n = list.len();
    let mut out = String::new();
    for (i, (code, desc)) in list.iter().enumerate() {
        out.push_str(
            &item_tmpl
                .replace("{{ c.code }}", code)
                .replace("{{ c.desc }}", desc),
        );
        if i + 1 < n {
            out.push_str(sep);
        }
    }
    out
}

/// If the byte at `pos` in `s` is a newline, return `pos + 1` (so the caller
/// consumes it) — the `trim_blocks` rule that drops the newline right after a
/// block/comment tag.
fn trim_block_newline(s: &str, pos: usize) -> usize {
    if s.as_bytes().get(pos) == Some(&b'\n') {
        pos + 1
    } else {
        pos
    }
}

/// Render a `.j2` template against the AI context (the minimal Jinja subset the
/// AI templates use: `{# comment #}`, inline `{% for %}` loops, and
/// `{{ opt_* }}` substitution — with `trim_blocks` newline stripping).
fn render_template(template: &str, ctx: &AiContext) -> String {
    let mut out = template.to_owned();
    // Strip `{# … #}` comments (+ the trailing newline, `trim_blocks`).
    while let Some(start) = out.find("{#") {
        let close = out[start..].find("#}").expect("comment closes") + start + 2;
        let end = trim_block_newline(&out, close);
        out.replace_range(start..end, "");
    }
    // Expand each `{% for c in NAME %}…{% endfor %}` (non-nested, inline).
    while let Some(for_start) = out.find("{% for c in ") {
        let head_end = out[for_start..].find("%}").expect("for tag closes") + for_start + 2;
        let name = out[for_start + "{% for c in ".len()..head_end - 2]
            .trim()
            .to_owned();
        let end_tag = "{% endfor %}";
        let body_end = out[head_end..].find(end_tag).expect("endfor") + head_end;
        let body = out[head_end..body_end].to_owned();
        let list = ctx
            .lists
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(&[][..], |(_, l)| l.as_slice());
        let rendered = render_loop(&body, list);
        let after = trim_block_newline(&out, body_end + end_tag.len());
        out.replace_range(for_start..after, &rendered);
    }
    // Scalar `{{ opt_* }}` substitutions.
    for (name, value) in &ctx.scalars {
        out = out.replace(&format!("{{{{ {name} }}}}"), value);
    }
    out
}

// Dialect catalogue projections

/// Dialects that deliberately reach no prompt fragment, each with the reason.
/// Empty today: a new profile either joins a fragment's `dialects` array or is
/// listed here, so `manifest.json` cannot silently forget one.
const PROMPT_EXEMPT_DIALECTS: &[(&str, &str)] = &[];

/// The canonical dialect names no prompt fragment claims and no exemption
/// covers — the [`prompt_manifest_gaps`] gate's finding.
fn prompt_manifest_gaps() -> Result<Vec<&'static str>> {
    let path = repo_root().join(PROMPT_MANIFEST);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let manifest: Value =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    let covered: Vec<&str> = manifest["prompts"]
        .as_array()
        .with_context(|| format!("{PROMPT_MANIFEST} must hold a `prompts` array"))?
        .iter()
        .flat_map(|entry| entry["dialects"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect();
    Ok(tcl_dialect::DialectProfile::all()
        .iter()
        .map(|profile| profile.name)
        .filter(|name| {
            !covered.contains(name) && !PROMPT_EXEMPT_DIALECTS.iter().any(|(d, _)| d == name)
        })
        .collect())
}

/// The VS Code chat dialect table: canonical name, display label, and the file
/// extensions the profile owns.
fn vscode_dialect_catalog() -> String {
    let mut rows = String::new();
    for profile in tcl_dialect::DialectProfile::all() {
        let extensions: Vec<String> = profile
            .file_extensions
            .iter()
            .map(|ext| format!("\"{}\"", ext.extension))
            .collect();
        let fields = [
            format!("name: \"{}\"", profile.name),
            format!("label: \"{}\"", profile.display_name),
            format!("extensions: [{}]", extensions.join(", ")),
        ];
        let inline = format!("  {{ {} }},", fields.join(", "));
        // Prettier's `printWidth: 100`: emit the row already expanded when the
        // one-line form would not fit, so the committed file is formatted.
        if inline.len() <= 100 {
            let _ = writeln!(rows, "{inline}");
        } else {
            let _ = writeln!(rows, "  {{");
            for field in &fields {
                let _ = writeln!(rows, "    {field},");
            }
            let _ = writeln!(rows, "  }},");
        }
    }
    format!(
        r"{LICENSE_HEADER}
/**
 * Every dialect the server accepts, projected from
 * `tcl_dialect::DialectProfile::all()`.
 *
 * Generated by `cargo xtask gen-ai-diagnostics` — edit the profile catalogue,
 * not this file. Kept free of `vscode` imports (the `../languageIds` pattern)
 * so chat modules and their unit tests can name dialects without the language
 * client.
 */

export interface DialectEntry {{
  /** Canonical dialect name — the string the server and `set_dialect` take. */
  readonly name: string;
  /** Human-facing display name. */
  readonly label: string;
  /** File extensions this dialect owns, without the leading dot. */
  readonly extensions: readonly string[];
}}

export const DIALECT_CATALOG: readonly DialectEntry[] = [
{rows}];

/** Canonical dialect name to display label. */
export const DIALECT_LABELS: Readonly<Record<string, string>> = Object.fromEntries(
  DIALECT_CATALOG.map((dialect) => [dialect.name, dialect.label]),
);
"
    )
}

/// The AGPL header every source file in the tree carries.
const LICENSE_HEADER: &str = "\
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
";

/// Every AI artifact: `(relative-path, content)`.
fn artifacts() -> Result<Vec<(String, String)>> {
    let diagnostic_catalogue = diagnostics_json();
    let mut out = vec![
        (DIAGNOSTICS_JSON.to_owned(), diagnostic_catalogue.clone()),
        (MCP_DIAGNOSTICS_JSON.to_owned(), diagnostic_catalogue),
        (VSCODE_DIALECT_CATALOG.to_owned(), vscode_dialect_catalog()),
    ];
    let ctx = ai_context();
    for (tmpl, output) in AI_TEMPLATES {
        let tpath = repo_root().join(tmpl);
        let template = std::fs::read_to_string(&tpath)
            .with_context(|| format!("reading {}", tpath.display()))?;
        out.push(((*output).to_owned(), render_template(&template, &ctx)));
    }
    Ok(out)
}

/// Write (or, with `check`, verify) the shared catalogues and prompt/skill files.
pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let gaps = prompt_manifest_gaps()?;
    if !gaps.is_empty() {
        eprintln!(
            "{PROMPT_MANIFEST}: {} dialect(s) reach no prompt fragment — add each to a fragment's \
             `dialects` array or to PROMPT_EXEMPT_DIALECTS with its reason:",
            gaps.len()
        );
        for name in &gaps {
            eprintln!("  - {name}");
        }
        return Ok(ExitCode::from(1));
    }
    let mut drift = Vec::new();
    for (rel, content) in artifacts()? {
        let path = root.join(&rel);
        if check {
            let current = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            if current != content {
                drift.push(rel);
            }
        } else {
            std::fs::write(&path, &content)
                .with_context(|| format!("writing {}", path.display()))?;
            eprintln!("wrote {rel}");
        }
    }
    if check && !drift.is_empty() {
        eprintln!(
            "{} generated diagnostic/prompt file(s) are stale — run `cargo xtask gen-ai-diagnostics`:",
            drift.len()
        );
        for rel in &drift {
            eprintln!("  - {rel}");
        }
        return Ok(ExitCode::from(1));
    }
    if check {
        eprintln!("OK: MCP/AI diagnostic catalogues and prompt files are in sync.");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_catalogues_and_prompt_files_match_generated() {
        for (rel, content) in artifacts().expect("build AI artifacts") {
            let current = std::fs::read_to_string(repo_root().join(&rel))
                .unwrap_or_else(|e| panic!("reading {rel}: {e}"));
            assert_eq!(
                current, content,
                "{rel} is stale — run `cargo xtask gen-ai-diagnostics`"
            );
        }
    }

    #[test]
    fn claude_prompt_copies_match_canonical_prompts() {
        let artifacts = artifacts().expect("build AI artifacts");
        for (canonical_path, copy_path) in [
            (
                "ai/prompts/tcl_system.md",
                "ai/claude/skills/_prompts/tcl_system.md",
            ),
            (
                "ai/prompts/irules_system.md",
                "ai/claude/skills/_prompts/irules_system.md",
            ),
        ] {
            let canonical = artifacts
                .iter()
                .find(|(path, _)| path == canonical_path)
                .map(|(_, content)| content)
                .expect("canonical prompt artifact");
            let copy = artifacts
                .iter()
                .find(|(path, _)| path == copy_path)
                .map(|(_, content)| content)
                .expect("Claude prompt-copy artifact");
            assert_eq!(copy, canonical, "{copy_path} must match {canonical_path}");
        }
    }

    #[test]
    fn every_catalogued_dialect_reaches_a_prompt_fragment() {
        let gaps = prompt_manifest_gaps().expect("read prompt manifest");
        assert!(
            gaps.is_empty(),
            "{PROMPT_MANIFEST} covers no prompt for {gaps:?} — add them to a fragment or to \
             PROMPT_EXEMPT_DIALECTS"
        );
    }

    #[test]
    fn prompt_manifest_names_only_catalogued_dialects() {
        let path = repo_root().join(PROMPT_MANIFEST);
        let manifest: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("read"))
            .expect("parse manifest");
        for entry in manifest["prompts"].as_array().expect("prompts array") {
            for dialect in entry["dialects"].as_array().expect("dialects array") {
                let name = dialect.as_str().expect("dialect name");
                assert!(
                    tcl_registry::model::resolve_environment(name)
                        .catalogue_profile()
                        .is_some(),
                    "{PROMPT_MANIFEST}: `{name}` is not a catalogued dialect"
                );
            }
        }
    }

    #[test]
    fn render_loop_joins_with_separator() {
        let list = vec![
            ("E001".to_owned(), "a".to_owned()),
            ("E002".to_owned(), "b".to_owned()),
        ];
        let body = "{{ c.code }} ({{ c.desc }}){% if not loop.last %}, {% endif %}";
        assert_eq!(render_loop(body, &list), "E001 (a), E002 (b)");
    }
}
