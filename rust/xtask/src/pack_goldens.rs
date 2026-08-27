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

//! `pack-goldens` — regenerate (or verify) the checked-in golden snapshots
//! of every shipped `.tclspec`.
//!
//! The gate itself is `rust/tcl-spectcl/tests/golden_packs.rs`, which runs on
//! every `cargo test`; this verb is how a deliberate change to what a pack
//! means gets written down. Both sides render through
//! [`tcl_spectcl::golden::render`], so the file the verb writes and the
//! string the gate compares cannot drift.
//!
//! It replaced the two-loader byte-identity gate when the CST loader was
//! deleted: with only one loader there is no second reading to compare
//! against, so the comparison is against a reading from a previous build —
//! which is the direction regressions actually travel.

use std::fmt::Write as _;
use std::process::ExitCode;

use tcl_spectcl::golden;

pub fn run(check: bool) -> ExitCode {
    let root = crate::util::repo_root();
    let packs = golden::shipped_packs(&root);
    if packs.len() < 24 {
        eprintln!(
            "pack-goldens: only {} shipped packs found under {:?} — the scan has gone blind",
            packs.len(),
            golden::PACK_DIRS
        );
        return ExitCode::FAILURE;
    }

    let mut stale = String::new();
    let mut written = 0_usize;
    let mut expected: Vec<String> = Vec::with_capacity(packs.len());
    for pack_path in &packs {
        let Ok(source) = std::fs::read_to_string(pack_path) else {
            eprintln!("pack-goldens: cannot read {}", pack_path.display());
            return ExitCode::FAILURE;
        };
        let rendered = golden::render(&tcl_spectcl::evaluate_pack(&source));
        let golden_path = golden::golden_path(&root, pack_path);
        if let Some(name) = golden_path.file_name() {
            expected.push(name.to_string_lossy().into_owned());
        }
        let current = std::fs::read_to_string(&golden_path).unwrap_or_default();
        if current == rendered {
            continue;
        }
        if check {
            let _ = writeln!(
                stale,
                "  {}",
                golden_path.strip_prefix(&root).unwrap_or(&golden_path).display()
            );
            continue;
        }
        if let Some(parent) = golden_path.parent()
            && let Err(err) = std::fs::create_dir_all(parent)
        {
            eprintln!("pack-goldens: {}: {err}", parent.display());
            return ExitCode::FAILURE;
        }
        if let Err(err) = std::fs::write(&golden_path, &rendered) {
            eprintln!("pack-goldens: {}: {err}", golden_path.display());
            return ExitCode::FAILURE;
        }
        written += 1;
    }

    // A golden with no pack behind it is a pack that was deleted or renamed
    // without its snapshot; the gate would never look at it again.
    let dir = root.join("rust/tcl-spectcl/tests/golden");
    let mut orphans = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".snap") && !expected.contains(&name) {
                if check {
                    orphans.push(name);
                } else if std::fs::remove_file(entry.path()).is_ok() {
                    written += 1;
                }
            }
        }
    }

    if check {
        if stale.is_empty() && orphans.is_empty() {
            println!(
                "pack-goldens: OK ({} shipped packs match their golden snapshots)",
                packs.len()
            );
            return ExitCode::SUCCESS;
        }
        if !orphans.is_empty() {
            let _ = writeln!(stale, "  orphaned goldens: {}", orphans.join(", "));
        }
        eprintln!(
            "pack-goldens: {} shipped pack(s) no longer load to their checked-in \
             snapshot. If the change is intended, run `cargo xtask pack-goldens` and \
             review the diff — that diff *is* the record of what a shipped pack now \
             means:\n{stale}",
            stale.lines().count()
        );
        return ExitCode::FAILURE;
    }

    println!("pack-goldens: {written} snapshot(s) rewritten, {} pack(s) scanned", packs.len());
    ExitCode::SUCCESS
}
