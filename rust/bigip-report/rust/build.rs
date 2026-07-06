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

//! Stamp the short git commit hash into `GIT_HASH` so the generated report can
//! show which build produced it (alongside the crate version). Falls back to
//! `"unknown"` when git metadata is unavailable (e.g. a source tarball build).

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    // rust/bigip-report/rust -> rust/bigip-report -> rust -> repo root.
    let repo_root = manifest
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| manifest.clone());

    // A CI / packaging pipeline that builds outside a git checkout (an unpacked
    // crate/source tarball) sets `GIT_HASH` to inject the commit it built from.
    println!("cargo:rerun-if-env-changed=GIT_HASH");

    // Re-run whenever the checked-out commit changes so the stamp stays
    // current. We deliberately track the commit *pointer* — HEAD, the branch ref
    // it resolves to (which is what a commit rewrites), and packed-refs — rather
    // than trying to watch working-tree state: an unstaged edit touches none of
    // these files, so a build script that keyed a "-dirty" flag off them could
    // not be re-run reliably. The stamp is therefore the committed short hash,
    // which is exactly what a shipped report needs.
    watch_commit_refs(&repo_root.join(".git"));

    println!("cargo:rustc-env=GIT_HASH={}", resolve_git_hash(&repo_root));
}

/// The git hash to stamp: an explicit `GIT_HASH` override (set by CI when
/// building outside a checkout) wins; otherwise ask git; otherwise `"unknown"`.
fn resolve_git_hash(repo_root: &Path) -> String {
    if let Ok(hash) = std::env::var("GIT_HASH") {
        let hash = hash.trim();
        if !hash.is_empty() {
            return hash.to_owned();
        }
    }
    git_hash(repo_root)
}

/// Emit `rerun-if-changed` for the files git rewrites when a new commit lands:
/// `HEAD`, the loose ref HEAD points at, and `packed-refs`.
fn watch_commit_refs(git_dir: &Path) {
    let head = git_dir.join("HEAD");
    if !head.exists() {
        return; // not a git checkout (e.g. an unpacked source tarball)
    }
    println!("cargo:rerun-if-changed={}", head.display());
    if let Ok(contents) = std::fs::read_to_string(&head)
        && let Some(reference) = contents.strip_prefix("ref:").map(str::trim)
    {
        let ref_path = git_dir.join(reference);
        if ref_path.exists() {
            println!("cargo:rerun-if-changed={}", ref_path.display());
        }
    }
    let packed = git_dir.join("packed-refs");
    if packed.exists() {
        println!("cargo:rerun-if-changed={}", packed.display());
    }
}

/// `git rev-parse --short HEAD`. `"unknown"` when git is unavailable or errors.
fn git_hash(repo_root: &Path) -> String {
    Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}
