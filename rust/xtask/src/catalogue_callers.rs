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

//! `catalogue-callers` — holds the dialect catalogue's enumerations to an
//! allowlist.
//!
//! `DialectProfile::all()` and `KNOWN_DIALECTS` list the lexer's grammar rows
//! and the editors' identity key. A list of names shown to a user comes from
//! the environment registry (read at run time, or projected by a generator
//! over the compiled registry), never from the catalogue
//! (`docs/design/contracts/environment-selection.md`, § *The rule*).
//!
//! Every file under `rust/` that spells either enumeration in code needs an
//! entry in [`ALLOWED`] with a one-line reason. An entry whose file no
//! longer spells one fails too, so the list only shrinks: whoever removes
//! the last caller from a file removes its entry in the same change.
//!
//! Comment lines are not call sites. This file is exempt because it names
//! the enumerations as data.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// The spellings of the two catalogue enumerations.
const NEEDLES: &[&str] = &["DialectProfile::all(", "KNOWN_DIALECTS"];

/// This file names the needles as data.
const SELF_PATH: &str = "rust/xtask/src/catalogue_callers.rs";

/// Files allowed to enumerate the catalogue, each with why. Sorted by path.
const ALLOWED: &[(&str, &str)] = &[
    (
        "rust/tcl-cli-support/src/environment.rs",
        "tests: the CLI's environment seam agrees with every catalogue profile",
    ),
    (
        "rust/tcl-cli-support/src/input.rs",
        "the CLI's unknown-dialect message lists the catalogue names",
    ),
    (
        "rust/tcl-cli/src/cli.rs",
        "the `--dialect` values are built from the catalogue",
    ),
    (
        "rust/tcl-cli/src/commands/registry.rs",
        "`registry-dump --all-dialects` snapshots the plain-Tcl profiles",
    ),
    (
        "rust/tcl-dialect/src/grammar.rs",
        "tests: the grammar table agrees with every catalogue row",
    ),
    (
        "rust/tcl-dialect/src/lib.rs",
        "re-exports `KNOWN_DIALECTS` from the catalogue",
    ),
    (
        "rust/tcl-dialect/src/model/environment.rs",
        "tests: the compiled environments agree with the catalogue",
    ),
    (
        "rust/tcl-dialect/src/model/point.rs",
        "tests: a dialect point agrees with every catalogue row",
    ),
    (
        "rust/tcl-dialect/src/profile.rs",
        "the catalogue itself: `KNOWN_DIALECTS`, `all()` and their invariant tests",
    ),
    (
        "rust/tcl-explorer/src/environment.rs",
        "tests: the explorer's environment seam agrees with every catalogue profile",
    ),
    (
        "rust/tcl-lsp-server/src/lib.rs",
        "`listDialects`, `getEffectiveConfig` labels, the unknown-dialect error and the pack pre-warm",
    ),
    (
        "rust/tcl-mcp/src/environment.rs",
        "tests: the MCP environment seam agrees with every catalogue profile",
    ),
    (
        "rust/tcl-mcp/src/main.rs",
        "the server blurb names the catalogued dialect families",
    ),
    (
        "rust/tcl-mcp/src/tools.rs",
        "the `dialect_schema` enum and the tool-schema tests",
    ),
    (
        "rust/tcl-registry/examples/dialect_surface.rs",
        "the availability differential walks every catalogue profile",
    ),
    (
        "rust/tcl-registry/src/cache.rs",
        "tests: cache pruning across every catalogue profile",
    ),
    (
        "rust/tcl-registry/src/dialects.rs",
        "the extension and filename detection maps read the catalogue; re-exports `KNOWN_DIALECTS`",
    ),
    (
        "rust/tcl-registry/src/lib.rs",
        "re-exports `KNOWN_DIALECTS`",
    ),
    (
        "rust/tcl-registry/src/model/assembly.rs",
        "tests: context assembly agrees with every catalogue profile",
    ),
    (
        "rust/tcl-registry/src/model/context.rs",
        "tests: resolved contexts agree with every catalogue profile",
    ),
    (
        "rust/tcl-registry/src/model/ingress.rs",
        "tests: the ingress agrees with every catalogue profile",
    ),
    (
        "rust/tcl-registry/src/model/semantic.rs",
        "tests: the semantic handle agrees with every catalogue profile",
    ),
    (
        "rust/tcl-registry/tests/detect_dialect.rs",
        "tests: detection agrees with the catalogue",
    ),
    (
        "rust/tcl-registry/tests/dialect_profile.rs",
        "tests: catalogue profile invariants",
    ),
    (
        "rust/tcl-registry/tests/registry_commands.rs",
        "tests: the registry's dialect roster matches `KNOWN_DIALECTS`",
    ),
    (
        "rust/tcl-registry/tests/registry_sweep.rs",
        "tests: the registry sweep walks every catalogue profile",
    ),
    (
        "rust/tcl-registry/tests/spec_pack_provenance.rs",
        "tests: pack provenance across every catalogue profile",
    ),
    (
        "rust/tcl-spec-studio/src/environment.rs",
        "tests: the studio's environment seam agrees with every catalogue profile",
    ),
    (
        "rust/tcl-spec-studio/src/lib.rs",
        "`browsable_dialects`, the studio's dialect picker",
    ),
    (
        "rust/tcl-spectcl/src/environment.rs",
        "tests: the pack loader's environment seam agrees with every catalogue profile",
    ),
    (
        "rust/tcl-spectcl/src/pack.rs",
        "`could_collide` asks every profile's context whether two packages coexist",
    ),
    (
        "rust/xtask/src/callback_inventory.rs",
        "the callback inventory is computed per catalogue profile",
    ),
    (
        "rust/xtask/src/editor_extensions.rs",
        "the editor language table is built from the catalogue",
    ),
    (
        "rust/xtask/src/gen_ai.rs",
        "the AI prompt manifest lists the catalogue's dialects",
    ),
    (
        "rust/xtask/src/gen_editor_dialects.rs",
        "the editor dialect lists are projected from the catalogue",
    ),
];

/// Every `*.rs` file under `dir`, skipping build output and hidden
/// directories.
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name != "target" && !name.starts_with('.') {
                collect_rs_files(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The line numbers in `text` that spell a needle in code. Comment lines
/// and the comment tail of a code line do not count.
fn call_sites(text: &str) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                return false;
            }
            let code = trimmed.split("//").next().unwrap_or(trimmed);
            NEEDLES.iter().any(|needle| code.contains(needle))
        })
        .map(|(index, _)| index + 1)
        .collect()
}

/// Every file with call sites, keyed by repo-relative path.
fn scan(root: &Path) -> BTreeMap<String, Vec<usize>> {
    let mut files = Vec::new();
    collect_rs_files(&root.join("rust"), &mut files);
    let mut found = BTreeMap::new();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if rel == SELF_PATH {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let sites = call_sites(&text);
        if !sites.is_empty() {
            found.insert(rel, sites);
        }
    }
    found
}

/// Files that spell an enumeration without an entry, and entries whose file
/// no longer does.
fn problems(found: &BTreeMap<String, Vec<usize>>) -> (Vec<String>, Vec<&'static str>) {
    let unlisted = found
        .iter()
        .filter(|(rel, _)| !ALLOWED.iter().any(|(path, _)| path == rel))
        .map(|(rel, sites)| {
            let lines: Vec<String> = sites.iter().map(ToString::to_string).collect();
            format!("{rel}:{}", lines.join(","))
        })
        .collect();
    let stale = ALLOWED
        .iter()
        .filter(|(path, _)| !found.contains_key(*path))
        .map(|(path, _)| *path)
        .collect();
    (unlisted, stale)
}

/// Scan the workspace; exit non-zero listing every unlisted caller and every
/// stale entry. `check` is accepted for CLI symmetry with the other gates —
/// the gate never rewrites anything, so both modes verify.
pub fn run(_check: bool) -> ExitCode {
    let found = scan(&crate::util::repo_root());
    let (unlisted, stale) = problems(&found);
    if unlisted.is_empty() && stale.is_empty() {
        println!(
            "catalogue-callers: OK ({} file(s) enumerate the dialect catalogue, all on the allowlist)",
            found.len()
        );
        return ExitCode::SUCCESS;
    }
    let mut report = String::new();
    if !unlisted.is_empty() {
        let _ = writeln!(
            report,
            "{} file(s) enumerate the dialect catalogue without an entry in ALLOWED \
             (rust/xtask/src/catalogue_callers.rs). A list of names shown to a user reads \
             the environment registry, or a generator over it:",
            unlisted.len()
        );
        for site in &unlisted {
            let _ = writeln!(report, "  {site}");
        }
    }
    if !stale.is_empty() {
        let _ = writeln!(
            report,
            "{} ALLOWED entr(y/ies) name a file that no longer enumerates the catalogue; \
             remove them:",
            stale.len()
        );
        for path in &stale {
            let _ = writeln!(report, "  {path}");
        }
    }
    eprintln!("catalogue-callers: FAILED\n{report}");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_allowlist_is_sorted_unique_and_explained() {
        let paths: Vec<&str> = ALLOWED.iter().map(|(path, _)| *path).collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            paths, sorted,
            "ALLOWED is sorted by path, one entry per file"
        );
        for (path, reason) in ALLOWED {
            assert!(!reason.is_empty() && !reason.contains('\n'), "{path}");
        }
    }

    #[test]
    fn comments_are_not_call_sites() {
        let text = "\
/// `DialectProfile::all()` is the identity key.
// for profile in DialectProfile::all() {
let names = 1; // KNOWN_DIALECTS
for profile in DialectProfile::all() {
    use_it(KNOWN_DIALECTS);
";
        assert_eq!(call_sites(text), [4, 5]);
    }

    #[test]
    fn an_unlisted_caller_and_a_stale_entry_are_both_reported() {
        let mut found = BTreeMap::new();
        found.insert("rust/elsewhere/src/lib.rs".to_owned(), vec![3, 9]);
        let (unlisted, stale) = problems(&found);
        assert_eq!(unlisted, ["rust/elsewhere/src/lib.rs:3,9"]);
        assert_eq!(stale.len(), ALLOWED.len(), "nothing on the list was found");
    }
}
