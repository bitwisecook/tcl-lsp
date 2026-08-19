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

//! `f5-query-builtins-doc` — the F5 query DSL builtin-reference coverage gate
//! (issue #1404 item 1).
//!
//! `docs/references/f5_query/builtins.md` is the canonical per-function
//! reference for every builtin `f5 query` exposes. It used to claim to be
//! *generated* from a Python registry that no longer exists; the header has
//! since been corrected to say it is hand-maintained, but nothing verified
//! that by hand meant *current* rather than merely *plausible*.
//!
//! A full content generator is not on the table: [`tcl_bigip_query::builtins`]
//! deliberately carries only dispatch metadata (name / category / arity /
//! flags) — see that module's `format_catalogue` doc comment — not the doc
//! prose (summary, signatures, worked examples) this reference page exists to
//! hold. Inventing a doc-prose field on `BuiltinSpec` for ~270 builtins is a
//! separate, much larger undertaking than a coverage gate; what this check
//! *can* verify cheaply and permanently is **coverage**: every builtin the
//! registry actually dispatches has at least one heading in the doc, and the
//! doc names no builtin the registry has since dropped or renamed. That is
//! exactly the failure mode a hand-maintained reference actually suffers —
//! a new builtin lands without its doc entry, or a retired one leaves a
//! stale, misleading one behind — and it is the same shape of gate as
//! `command-backing` for the WASM runtime and `audit-option-dialects --check`
//! for `OptionSpec` dialect gates.
//!
//! # What is (and isn't) checked
//!
//! Checked: the *set* of builtin names the registry dispatches equals the set
//! of names given a `` `name` `` heading in the doc. Most entries are one
//! `### \`name\`` heading each; a handful of tightly-related builtins share
//! one heading (`### \`files\` / \`file\` / \`glob\` / \`grep\``), so every
//! backtick-quoted token on a `###` heading line counts as documenting that
//! name — not just the first.
//!
//! Not checked: prose accuracy, per-argument signatures, example correctness,
//! or which `##` category section a heading lives under. Those need a human
//! reader; this gate exists so that need is never hidden behind a false
//! "the reference is up to date" impression.

use std::collections::BTreeSet;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};

use crate::util::repo_root;

const DOC_PATH: &str = "docs/references/f5_query/builtins.md";

/// Every builtin name the query DSL actually dispatches, straight from the
/// registry — the same list `all_specs()` builds `--help-builtins` from.
fn registry_names() -> BTreeSet<&'static str> {
    tcl_bigip_query::builtins::all_specs()
        .into_iter()
        .map(|spec| spec.name)
        .collect()
}

/// Every builtin name given a `### ...` heading in `builtins.md`.
///
/// A heading line may name more than one builtin
/// (`` ### `files` / `file` / `glob` / `grep` ``) — every backtick-quoted
/// token on the line counts, not just the first.
fn doc_heading_names(doc: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in doc.lines() {
        let Some(rest) = line.strip_prefix("### ") else {
            continue;
        };
        // Splitting on the backtick delimiter alternates
        // [outside, inside, outside, inside, ...] starting with "outside" —
        // every odd-indexed piece is one backtick-quoted token. Pairing
        // opening/closing backticks by re-scanning for the *next* backtick
        // after each match (the naive approach) double-counts: the closing
        // backtick of one name is itself a valid "opening" backtick for the
        // next search, so it would also capture the plain-text separator
        // (`` " / " ``) between names as a bogus extra "name".
        for (i, token) in rest.split('`').enumerate() {
            if i % 2 == 1 {
                names.insert(token.to_owned());
            }
        }
    }
    names
}

/// Read the doc and diff it against the registry: `(undocumented, stale)`.
///
/// `undocumented` is every registered builtin with no doc heading;
/// `stale` is every doc heading name the registry no longer dispatches
/// (a retired or renamed builtin left behind).
fn diff(doc: &str) -> (Vec<String>, Vec<String>) {
    let registry = registry_names();
    let documented = doc_heading_names(doc);

    let undocumented: Vec<String> = registry
        .iter()
        .filter(|name| !documented.contains(**name))
        .map(|name| (*name).to_owned())
        .collect();
    let stale: Vec<String> = documented
        .iter()
        .filter(|name| !registry.contains(name.as_str()))
        .cloned()
        .collect();
    (undocumented, stale)
}

/// Verify `docs/references/f5_query/builtins.md` documents exactly the
/// builtins the registry dispatches — no more, no fewer.
///
/// `check` is accepted for command-line symmetry with the other drift gates
/// (`number-drift`, `resolution-drift`): there is no generated form to write,
/// so this always verifies regardless of the flag.
pub fn run(check: bool) -> Result<ExitCode> {
    let _ = check; // symmetry only — see the doc comment above
    let root = repo_root();
    let path = root.join(DOC_PATH);
    let doc = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;

    let (undocumented, stale) = diff(&doc);
    if undocumented.is_empty() && stale.is_empty() {
        eprintln!(
            "OK: {DOC_PATH} documents exactly the {} builtins the registry dispatches.",
            registry_names().len()
        );
        return Ok(ExitCode::SUCCESS);
    }

    if !undocumented.is_empty() {
        eprintln!(
            "{} builtin(s) registered in tcl-bigip-query have no heading in {DOC_PATH}:",
            undocumented.len()
        );
        for name in &undocumented {
            eprintln!("  + {name}");
        }
    }
    if !stale.is_empty() {
        eprintln!(
            "{} heading(s) in {DOC_PATH} name a builtin the registry no longer dispatches \
             (retired or renamed — fix the doc, or fix the registry name):",
            stale.len()
        );
        for name in &stale {
            eprintln!("  - {name}");
        }
    }
    Ok(ExitCode::from(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_extraction_takes_every_name_on_a_combined_heading() {
        let doc = "### `files` / `file` / `glob` / `grep`\n\nsome prose\n";
        let names = doc_heading_names(doc);
        assert_eq!(
            names,
            BTreeSet::from([
                "files".to_owned(),
                "file".to_owned(),
                "glob".to_owned(),
                "grep".to_owned(),
            ])
        );
    }

    #[test]
    fn heading_extraction_ignores_non_heading_backticks() {
        let doc = "Some prose mentioning `select` inline.\n\n### `map`\n\nmore `noise` here\n";
        let names = doc_heading_names(doc);
        assert_eq!(names, BTreeSet::from(["map".to_owned()]));
    }

    #[test]
    fn diff_flags_an_undocumented_registry_name() {
        let doc = "### `select`\n";
        let (undocumented, stale) = diff(doc);
        assert!(undocumented.contains(&"map".to_owned()), "{undocumented:?}");
        assert!(stale.is_empty());
    }

    #[test]
    fn diff_flags_a_stale_doc_heading() {
        let doc = "### `this_builtin_was_retired_years_ago`\n";
        let (_undocumented, stale) = diff(doc);
        assert!(
            stale.contains(&"this_builtin_was_retired_years_ago".to_owned()),
            "{stale:?}"
        );
    }

    /// The gate this module exists to provide: the committed reference page
    /// must document exactly the registry's current builtin set. A failure
    /// here means `docs/references/f5_query/builtins.md` needs a hand edit —
    /// add the missing heading(s), or drop the stale one(s) — same as a
    /// `cargo xtask f5-query-builtins-doc --check` failure would report.
    #[test]
    fn committed_reference_doc_matches_the_registry() {
        let root = repo_root();
        let path = root.join(DOC_PATH);
        let doc = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        let (undocumented, stale) = diff(&doc);
        assert!(
            undocumented.is_empty(),
            "builtins registered but undocumented in {DOC_PATH}: {undocumented:?}"
        );
        assert!(
            stale.is_empty(),
            "headings in {DOC_PATH} for builtins the registry no longer has: {stale:?}"
        );
    }
}
