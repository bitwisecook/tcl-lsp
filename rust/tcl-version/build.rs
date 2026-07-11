// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Resolve the version every user-facing binary reports, in this order:
//!
//! 1. `TCL_LSP_VERSION` — set by CI from the tag, and by the Makefile from
//!    `git describe`. Authoritative when present.
//! 2. `git describe --tags --always --dirty` — a working-tree build gets
//!    `2.1.5-3-gabc1234` so a dev binary never claims to be a release.
//! 3. `CARGO_PKG_VERSION` — the manifest's `0.1.0`. Only reached when the
//!    source tree has no tags and no env override (a vendored crate, say).
//!
//! Releases are tag-only: no version literal is bumped in the tree, so the tag
//! is the single source of truth. See `scripts/release/tag.sh`.

use std::process::Command;

fn main() {
    // A change to either input must re-run this script, or the binary keeps a
    // stale version baked in from a previous build.
    println!("cargo:rerun-if-env-changed=TCL_LSP_VERSION");
    for p in [".git/HEAD", ".git/refs/tags", ".git/packed-refs"] {
        println!("cargo:rerun-if-changed=../../{p}");
    }

    let version = env_version()
        .or_else(git_describe)
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());

    println!("cargo:rustc-env=TCL_LSP_RESOLVED_VERSION={version}");
}

/// `TCL_LSP_VERSION`, with a leading `v` stripped so `v2.1.5` and `2.1.5` are
/// equivalent — CI passes `github.ref_name`, which carries the `v`.
fn env_version() -> Option<String> {
    let raw = std::env::var("TCL_LSP_VERSION").ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(strip_v(trimmed).to_owned())
}

fn git_describe() -> Option<String> {
    let out = Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    Some(strip_v(s).to_owned())
}

fn strip_v(s: &str) -> &str {
    s.strip_prefix('v').unwrap_or(s)
}
