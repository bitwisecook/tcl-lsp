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

//! Generate the published code tables under `docs/generated/` from the
//! `DiagCode` catalogue in `tcl-core-types`.
//!
//! `DiagCode::ALL` carries each code's section/category, one-line description,
//! and default-on flag (its [`DocRow`]), so the three tables are a pure
//! projection of the enum:
//!
//! - `diagnostic_tables.md` — combined diagnostics + optimisations (the
//!   README-includable view);
//! - `diagnostic_codes.md` — diagnostics only;
//! - `optimisation_codes.md` — optimisations only.
//!
//! Run `cargo xtask diag-tables` to (re)write them. `--check` instead verifies
//! the committed files match what the enum would produce, exiting non-zero on
//! drift — the same invariant the `committed_tables_match_generated` test pins
//! into the `cargo test` gate.

use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use tcl_core_types::{DiagCode, DiagSection, DocRow, OptCategory};

use crate::util::repo_root;

/// The trailing profile-legend paragraph shared by the combined and
/// optimisation-only tables.
const PROFILES_FOOTER: &str =
    "**Profiles:** `off` disables all passes. `readability`, `standard`, and `full` enable
progressively more passes (single-pass). `aggressive` = `full` with multi-pass
to fixpoint (up to 5 iterations). The default editor profile is `readability`;
explicit actions (CLI, chat, MCP) default to `full`.
";

/// Diagnostics, grouped by section (in `DiagSection` declaration order) then by
/// code spelling.
fn diagnostic_rows() -> Vec<(DiagCode, DiagSection, bool, &'static str)> {
    let mut rows: Vec<_> = DiagCode::ALL
        .iter()
        .copied()
        .filter_map(|c| match c.doc_row() {
            DocRow::Diagnostic {
                section,
                default_on,
                description,
                internal: _,
                reserved: _,
            } => Some((c, section, default_on, description)),
            DocRow::Optimisation { .. } => None,
        })
        .collect();
    rows.sort_by_key(|(c, section, _, _)| (*section, c.as_str()));
    rows
}

/// Optimisations, ordered by code spelling.
fn optimisation_rows() -> Vec<(DiagCode, OptCategory, &'static str)> {
    let mut rows: Vec<_> = DiagCode::ALL
        .iter()
        .copied()
        .filter_map(|c| match c.doc_row() {
            DocRow::Optimisation {
                category,
                description,
            } => Some((c, category, description)),
            DocRow::Diagnostic { .. } => None,
        })
        .collect();
    rows.sort_by_key(|(c, _, _)| c.as_str());
    rows
}

/// The `### Diagnostic Codes` table, ending with the last row's newline.
fn diagnostic_table() -> String {
    let mut out = String::from(
        "### Diagnostic Codes\n\n\
         | Code | Section | Description | Default |\n\
         |------|---------|-------------|---------|\n",
    );
    for (code, section, default_on, description) in diagnostic_rows() {
        let default = if default_on { "✓" } else { "✗" };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            code.as_str(),
            section.as_str(),
            description,
            default
        );
    }
    out
}

/// The `### Optimisation Codes` table, ending with the last row's newline. The
/// `readability` / `standard` columns are derived from the category's profile
/// membership; `full` enables every pass.
fn optimisation_table() -> String {
    let mut out = String::from(
        "### Optimisation Codes\n\n\
         | Code | Category | Description | readability | standard | full |\n\
         |------|----------|-------------|:-----------:|:--------:|:----:|\n",
    );
    for (code, category, description) in optimisation_rows() {
        let readability = if category.in_readability_profile() {
            "✓"
        } else {
            ""
        };
        let standard = if category.in_standard_profile() {
            "✓"
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | ✓ |",
            code.as_str(),
            category.as_str(),
            description,
            readability,
            standard
        );
    }
    out
}

/// `(repo-relative path, generated content)` for each table.
fn tables() -> Vec<(&'static str, String)> {
    let diag = diagnostic_table();
    let opt = optimisation_table();
    vec![
        (
            "docs/generated/diagnostic_tables.md",
            format!("{diag}\n{opt}\n{PROFILES_FOOTER}"),
        ),
        ("docs/generated/diagnostic_codes.md", diag.clone()),
        (
            "docs/generated/optimisation_codes.md",
            format!("{opt}\n{PROFILES_FOOTER}"),
        ),
    ]
}

/// Write (or, with `check`, verify) the generated tables.
pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let mut drift = Vec::new();
    for (rel, content) in tables() {
        let path = root.join(rel);
        if check {
            let current =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            if current != content {
                drift.push(rel);
            }
        } else {
            fs::write(&path, &content).with_context(|| format!("writing {}", path.display()))?;
            eprintln!("wrote {rel}");
        }
    }

    if check && !drift.is_empty() {
        eprintln!(
            "{} generated code table(s) are stale — run `cargo xtask diag-tables`:",
            drift.len()
        );
        for rel in &drift {
            eprintln!("  - {rel}");
        }
        return Ok(ExitCode::from(1));
    }
    if check {
        eprintln!("OK: all generated code tables are in sync with DiagCode::ALL.");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drift guard: the committed tables must equal what `DiagCode::ALL`
    /// renders. A code added/removed/re-described without regenerating fails
    /// here, on the `cargo test` gate.
    #[test]
    fn committed_tables_match_generated() {
        let root = repo_root();
        for (rel, content) in tables() {
            let path = root.join(rel);
            let current = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            assert_eq!(
                current, content,
                "{rel} is stale — run `cargo xtask diag-tables`"
            );
        }
    }
}
