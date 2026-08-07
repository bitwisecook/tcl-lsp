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

//! `workflow-sync` — keep the installed GitHub Actions workflows byte-identical
//! to their canonical sources.
//!
//! Two workflows are maintained under `rust/bigip-report-gen/python/deploy/`
//! and *installed* into `.github/workflows/` by copying. The split exists
//! because writing a file under `.github/workflows/` needs the GitHub
//! `workflow` OAuth scope, which automation tokens have not always carried —
//! so the canonical copy lives somewhere any token can edit, and a human with
//! the scope installs it.
//!
//! A copied artefact with no gate is a drift generator, and this pair has
//! drifted twice: an earlier pass reconciled a stale comment in both files,
//! and the two later diverged again — `.github/workflows/` gained
//! `fetch-depth: 0`, the release-version stamping step and an explicit stable
//! toolchain, while the canonical copies gained a self-description neither
//! installed copy had. Each side held a fix the other lacked, so *either*
//! direction of the documented `cp` would have silently reverted real work:
//! reinstalling from canon would have dropped the version stamping and
//! restored the `0.1.0` placeholder bug, and at that point the Pages deploy
//! would also have lost the spec studio entirely.
//!
//! This gate makes that impossible to do quietly. `--check` fails on any
//! difference; without it, the canonical copy is reinstalled.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

/// Where the canonical copies live, relative to the repo root.
const CANONICAL_DIR: &str = "rust/bigip-report-gen/python/deploy";

/// Where the installed copies live, relative to the repo root.
const INSTALLED_DIR: &str = ".github/workflows";

/// The workflows maintained as canonical-source + installed-copy pairs.
///
/// Only these are paired: every other file under `.github/workflows/` is
/// edited in place and has no canonical counterpart.
const PAIRED: &[&str] = &["github-pages.yml", "report-pyz.yml"];

/// Verify (or restore) byte-identity between each canonical workflow and its
/// installed copy.
pub fn run(check: bool) -> anyhow::Result<ExitCode> {
    let root = crate::util::repo_root();
    let mut drifted: Vec<(String, String)> = Vec::new();
    let mut restored: Vec<String> = Vec::new();

    for name in PAIRED {
        let canonical: PathBuf = root.join(CANONICAL_DIR).join(name);
        let installed: PathBuf = root.join(INSTALLED_DIR).join(name);

        let want = std::fs::read_to_string(&canonical).map_err(|e| {
            anyhow::anyhow!(
                "canonical workflow {} is unreadable: {e}",
                canonical.display()
            )
        })?;
        // A missing installed copy is drift too — the workflow is not active.
        let have = std::fs::read_to_string(&installed).unwrap_or_default();

        if want == have {
            continue;
        }
        if check {
            drifted.push(((*name).to_owned(), describe(&want, &have)));
        } else {
            std::fs::write(&installed, &want)?;
            restored.push((*name).to_owned());
        }
    }

    if !drifted.is_empty() {
        let mut msg = String::from(
            "workflow-sync: installed workflow(s) differ from their canonical source\n",
        );
        for (name, detail) in &drifted {
            let _ = write!(
                msg,
                "\n  {name}\n    canonical: {CANONICAL_DIR}/{name}\n    installed: {INSTALLED_DIR}/{name}\n    {detail}\n"
            );
        }
        msg.push_str(
            "\nBoth copies must match. Decide which side is correct — the installed\n\
             copy may hold a fix the canonical one never received — fold that into\n\
             the canonical file, then run `cargo xtask workflow-sync` to reinstall.\n",
        );
        eprintln!("{msg}");
        return Ok(ExitCode::FAILURE);
    }

    if restored.is_empty() {
        println!("workflow-sync: {} workflow pair(s) in sync", PAIRED.len());
    } else {
        for name in &restored {
            println!(
                "workflow-sync: reinstalled {INSTALLED_DIR}/{name} from {CANONICAL_DIR}/{name}"
            );
        }
        println!(
            "workflow-sync: committing a change under {INSTALLED_DIR}/ needs the GitHub `workflow` scope"
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// A one-line summary of how two versions differ, enough to tell a missing
/// file from a stale one without printing a whole diff.
fn describe(want: &str, have: &str) -> String {
    if have.is_empty() {
        return "installed copy is missing or empty".to_owned();
    }
    let (want_lines, have_lines) = (want.lines().count(), have.lines().count());
    let first = want
        .lines()
        .zip(have.lines())
        .position(|(a, b)| a != b)
        .map_or_else(
            || "content matches to the shorter file's end".to_owned(),
            |i| format!("first difference at line {}", i + 1),
        );
    format!("{first} (canonical {want_lines} lines, installed {have_lines})")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pair list must name files that actually exist on both sides —
    /// a typo here would make the gate silently vacuous.
    #[test]
    fn every_paired_workflow_exists_in_both_locations() {
        let root = crate::util::repo_root();
        for name in PAIRED {
            let canonical = root.join(CANONICAL_DIR).join(name);
            let installed = root.join(INSTALLED_DIR).join(name);
            assert!(
                canonical.is_file(),
                "canonical workflow missing: {}",
                canonical.display()
            );
            assert!(
                installed.is_file(),
                "installed workflow missing: {}",
                installed.display()
            );
        }
    }

    /// The committed tree must already satisfy the gate.
    #[test]
    fn the_committed_workflows_are_in_sync() {
        let root = crate::util::repo_root();
        for name in PAIRED {
            let canonical = std::fs::read_to_string(root.join(CANONICAL_DIR).join(name)).unwrap();
            let installed = std::fs::read_to_string(root.join(INSTALLED_DIR).join(name)).unwrap();
            assert_eq!(
                canonical, installed,
                "{name} has drifted — run `cargo xtask workflow-sync`"
            );
        }
    }

    /// Every canonical workflow points a reader at the sync gate, so someone
    /// editing the installed copy directly finds out how it is maintained.
    #[test]
    fn each_canonical_workflow_documents_the_pairing() {
        let root = crate::util::repo_root();
        for name in PAIRED {
            let text = std::fs::read_to_string(root.join(CANONICAL_DIR).join(name)).unwrap();
            assert!(
                text.contains("The canonical copy of this workflow lives at"),
                "{name} lacks the canonical-location note"
            );
            assert!(
                text.contains("workflow-sync"),
                "{name} does not mention the sync gate"
            );
        }
    }

    #[test]
    fn describe_flags_a_missing_installed_copy() {
        assert_eq!(describe("a\nb\n", ""), "installed copy is missing or empty");
    }

    #[test]
    fn describe_locates_the_first_differing_line() {
        let d = describe("a\nb\nc\n", "a\nX\nc\n");
        assert!(d.starts_with("first difference at line 2"), "{d}");
    }
}
