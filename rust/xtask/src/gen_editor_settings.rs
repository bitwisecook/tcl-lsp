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

//! Generate the VS Code diagnostic catalogue (`diagnosticCatalog.ts`) from the
//! `DiagCode` catalogue — the native successor to the TypeScript half of
//! `scripts/codegen/editor_settings.py`.
//!
//! The catalogue is a pure projection of [`tcl_core_types::DiagCode`]:
//!
//! - `DIAGNOSTICS` — every **user-configurable** diagnostic (i.e. not
//!   [`DiagCode::is_internal`]), grouped by section then code;
//! - `OPTIMISATIONS` — every optimisation code, by code;
//! - `SECTION_TITLES` / `SECTION_ORDER` — the VS Code settings grouping.
//!
//! Run `cargo xtask gen-editor-settings` to (re)write it; `--check` verifies the
//! committed file matches, exiting non-zero on drift.
//!
//! String literals use stock `serde_json::to_string` (standard JSON escaping,
//! raw UTF-8); `description` fields wrap onto a second line past a 100-column
//! width for readable diffs.

use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use tcl_core_types::{DiagCode, DocRow};

use crate::util::repo_root;

const CATALOG_PATH: &str = "editors/vscode/src/generated/diagnosticCatalog.ts";

/// The VS Code section grouping — `(section key, display title)` in table
/// order. Mirrors `shared/codes.py::SECTIONS`; the three `irules*` keys share
/// one title, which collapses in `SECTION_TITLES`. Deliberately omits the `tk`
/// section (no user-configurable Tk diagnostics — the `TK###` codes are all
/// internal).
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

/// The sort rank of a section key (its index in [`SECTIONS`]); unknown keys
/// sort last.
fn section_rank(key: &str) -> usize {
    SECTIONS.iter().position(|(k, _)| *k == key).unwrap_or(999)
}

/// A TypeScript/JSON string literal (double-quoted, standard JSON escaping, raw
/// UTF-8). `serde_json::to_string` never fails for a `&str`.
fn ts_string(value: &str) -> String {
    serde_json::to_string(value).expect("serialise string")
}

/// The `description:` field, wrapped onto a second (6-space-indented) line when
/// the single-line form would exceed a 100-column width (kept for readable
/// diffs on long descriptions).
fn ts_description_field(value: &str) -> String {
    let literal = ts_string(value);
    let one_line = format!("    description: {literal},");
    if one_line.chars().count() <= 100 {
        one_line
    } else {
        format!("    description:\n      {literal},")
    }
}

/// Render the full `diagnosticCatalog.ts`.
fn catalog() -> String {
    // User-configurable diagnostics, grouped by section then code.
    let mut diags: Vec<(&'static str, &'static str, bool, &'static str)> = DiagCode::ALL
        .iter()
        .filter_map(|c| match c.doc_row() {
            DocRow::Diagnostic {
                section,
                default_on,
                internal: false,
                reserved: false,
                description,
                tag: _,
            } => Some((c.as_str(), section.as_str(), default_on, description)),
            _ => None,
        })
        .collect();
    diags.sort_by_key(|(code, section, _, _)| (section_rank(section), *code));

    // Optimisations, by code (always default-on).
    let mut opts: Vec<(&'static str, &'static str)> = DiagCode::ALL
        .iter()
        .filter_map(|c| match c.doc_row() {
            DocRow::Optimisation { description, .. } => Some((c.as_str(), description)),
            DocRow::Diagnostic { .. } => None,
        })
        .collect();
    opts.sort_by_key(|(code, _)| *code);

    let mut out = crate::util::license_banner("//");
    out.push('\n');
    out.push_str(
        "// GENERATED by `cargo xtask gen-editor-settings` — do not edit.\n\
         // Source of truth: `tcl-core-types` `DiagCode` catalogue.\n\
         \n\
         export interface DiagnosticDef {\n  code: string;\n  section: string;\n  \
         description: string;\n  defaultEnabled: boolean;\n}\n\
         \n\
         export interface OptimisationDef {\n  code: string;\n  description: string;\n  \
         defaultEnabled: boolean;\n}\n\
         \n\
         export const DIAGNOSTICS: DiagnosticDef[] = [\n",
    );
    for (code, section, default_on, description) in &diags {
        let _ = write!(
            out,
            "  {{\n    code: \"{code}\",\n    section: \"{section}\",\n{}\n    defaultEnabled: {default_on},\n  }},\n",
            ts_description_field(description),
        );
    }
    out.push_str("];\n\nexport const OPTIMISATIONS: OptimisationDef[] = [\n");
    for (code, description) in &opts {
        let _ = write!(
            out,
            "  {{\n    code: \"{code}\",\n{}\n    defaultEnabled: true,\n  }},\n",
            ts_description_field(description),
        );
    }
    out.push_str("];\n\nexport const SECTION_TITLES: Record<string, string> = {\n");
    // Deduplicate section titles (the three `irules*` keys share one title):
    // first key wins, preserving declaration order.
    let mut seen: Vec<&str> = Vec::new();
    let mut ordered_keys: Vec<&str> = Vec::new();
    for (key, title) in SECTIONS {
        if !seen.contains(title) {
            seen.push(title);
            ordered_keys.push(key);
            let _ = writeln!(out, "  {key}: {},", ts_string(title));
        }
    }
    out.push_str("};\n\nexport const SECTION_ORDER: string[] = [\n");
    for key in &ordered_keys {
        let _ = writeln!(out, "  \"{key}\",");
    }
    out.push_str("];\n");
    out
}

/// Write (or, with `check`, verify) `diagnosticCatalog.ts`.
pub fn run(check: bool) -> Result<ExitCode> {
    let path = repo_root().join(CATALOG_PATH);
    let content = catalog();
    if check {
        let current =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        if current != content {
            eprintln!(
                "{CATALOG_PATH} is stale — run `cargo xtask gen-editor-settings`. \
                 The committed catalogue does not match the DiagCode catalogue."
            );
            return Ok(ExitCode::from(1));
        }
        eprintln!("OK: {CATALOG_PATH} is in sync with the DiagCode catalogue.");
    } else {
        fs::write(&path, &content).with_context(|| format!("writing {}", path.display()))?;
        eprintln!("wrote {CATALOG_PATH}");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ts_string_is_raw_utf8_json() {
        // Standard JSON string escaping, raw (non-`\uXXXX`) non-ASCII.
        assert_eq!(ts_string("a — b"), "\"a — b\"");
        assert_eq!(ts_string("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(
            ts_string("Diagnostics — Style & Best Practice"),
            "\"Diagnostics — Style & Best Practice\""
        );
    }

    #[test]
    fn description_wraps_past_100_columns() {
        let short = ts_description_field("short");
        assert_eq!(short, "    description: \"short\",");
        let long = "x".repeat(120);
        let wrapped = ts_description_field(&long);
        assert!(wrapped.starts_with("    description:\n      \""));
    }

    /// Drift guard: the committed catalogue must equal what `DiagCode` renders.
    #[test]
    fn committed_catalog_matches_generated() {
        let path = repo_root().join(CATALOG_PATH);
        let current =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        assert_eq!(
            current,
            catalog(),
            "{CATALOG_PATH} is stale — run `cargo xtask gen-editor-settings`"
        );
    }
}
