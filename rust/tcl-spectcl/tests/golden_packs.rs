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

//! **The golden-snapshot gate** — a loader change cannot silently alter what
//! a shipped pack means.
//!
//! Every `.tclspec` the repository ships loads and is rendered by
//! [`tcl_spectcl::golden::render`]; the rendering must equal the snapshot
//! checked in beside this file. A deliberate change is recorded by running
//! `cargo xtask pack-goldens`, which writes the same rendering through the
//! same function — so the diff a reviewer reads *is* the change in meaning.
//!
//! This replaced the two-loader byte-identity gate when the CST loader was
//! deleted. `tcl_spectcl::golden`'s module documentation says precisely what
//! that trade gains and gives up; the short version is that the two-loader
//! gate compared two readings of one build (blind to a bug both shared) and
//! this one compares against a reading from an *earlier* build (the direction
//! regressions travel), while `eval_loader.rs`'s fast-path gate keeps a
//! same-build duality over the same 24 packs.

use std::path::{Path, PathBuf};

use tcl_spectcl::golden;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

/// The first differing line of two renderings, and — when the line names a
/// command — the full before/after of that command's exhaustive rendering,
/// which is the thing the golden holds only a digest of.
fn explain(golden: &str, fresh: &str, pack: &tcl_spectcl::Pack) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for (index, (was, now)) in golden.lines().zip(fresh.lines()).enumerate() {
        if was == now {
            continue;
        }
        let _ = writeln!(out, "line {}:\n  golden: {was}\n  now   : {now}", index + 1);
        if let Some(name) = now.strip_prefix("command ").and_then(|r| r.split(' ').next())
            && let Some(rendering) = golden::spec_rendering(pack, name)
        {
            let _ = writeln!(
                out,
                "  `{name}` now renders as:\n    {rendering}\n  \
                 (the golden holds a digest of this; run `cargo xtask pack-goldens` \
                 to record the new value)"
            );
        }
        return out;
    }
    let _ = writeln!(
        out,
        "line counts differ: golden {} vs now {}",
        golden.lines().count(),
        fresh.lines().count()
    );
    out
}

#[test]
fn every_shipped_pack_still_loads_to_its_golden_snapshot() {
    let root = repo_root();
    let packs = golden::shipped_packs(&root);
    assert!(
        packs.len() >= 24,
        "the inventory must cover the shipped packs; found {packs:?}"
    );

    let mut commands = 0_usize;
    let mut notices = 0_usize;
    for path in &packs {
        let source = std::fs::read_to_string(path).expect("readable pack");
        let pack = tcl_spectcl::evaluate_pack(&source);
        let fresh = golden::render(&pack);
        let golden_path = golden::golden_path(&root, path);
        let recorded = std::fs::read_to_string(&golden_path).unwrap_or_else(|err| {
            panic!(
                "{}: no golden snapshot at {} ({err}) — run `cargo xtask pack-goldens`",
                path.display(),
                golden_path.display()
            )
        });
        assert!(
            recorded == fresh,
            "{} no longer loads to its golden snapshot.\n{}",
            path.display(),
            explain(&recorded, &fresh, &pack)
        );
        commands += pack.commands.len();
        notices += pack.notices.len();
    }

    println!(
        "golden gate: {} packs, {commands} commands, {notices} notices held to their \
         checked-in snapshots",
        packs.len()
    );
    assert!(commands >= 800, "only {commands} commands covered");
}

/// A golden with no pack behind it would never be looked at again — a pack
/// renamed or deleted without its snapshot. The regeneration verb removes
/// them; this is the gate that notices.
#[test]
fn no_golden_snapshot_is_orphaned() {
    let root = repo_root();
    let expected: Vec<String> = golden::shipped_packs(&root)
        .iter()
        .filter_map(|pack| {
            golden::golden_path(&root, pack)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    let dir = root.join("rust/tcl-spectcl/tests/golden");
    let mut orphans: Vec<String> = std::fs::read_dir(&dir)
        .expect("the golden directory")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".snap") && !expected.contains(name))
        .collect();
    orphans.sort();
    assert!(
        orphans.is_empty(),
        "golden snapshots with no shipped pack behind them: {orphans:?} — \
         run `cargo xtask pack-goldens`"
    );
}
