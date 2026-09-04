// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Resolve the version every user-facing binary reports.
//!
//! The version *base*, in order:
//!
//! 1. `TCL_LSP_VERSION` — set by CI from the tag, and by the Makefile from
//!    `git describe`. Authoritative when present.
//! 2. `git describe --tags --always --dirty` — a working-tree build gets
//!    `2.1.5-3-gabc1234` so a dev binary never claims to be a release.
//! 3. `CARGO_PKG_VERSION` — the manifest's `0.1.0`. Only reached when the
//!    source tree has no tags and no env override (a vendored crate, say).
//!
//! The *commit* is then appended as semver build metadata, resolved
//! independently of the base:
//!
//! ```text
//! at a tag              2.1.8+g4c5a8f86
//! 3 commits past a tag  2.1.8-3+g4c5a8f86
//! modified working tree 2.1.8+g4c5a8f86.dirty
//! ```
//!
//! It has to be independent, because neither source above yields a hash for a
//! release: `git describe` only appends `-<n>-g<hash>` when the build is *not*
//! on a tag, and CI's `TCL_LSP_VERSION` bypasses `git describe` altogether. So
//! the one build whose provenance matters most would otherwise carry no commit
//! at all.
//!
//! Releases are tag-only: no version literal is bumped in the tree, so the tag
//! is the single source of truth. See `scripts/release/tag.sh`.

#[path = "src/git_inputs.rs"]
mod git_inputs;

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let stamped = env_version();

    // A change to either input must re-run this script, or the binary keeps a
    // stale version baked in from a previous build.
    println!("cargo:rerun-if-env-changed=TCL_LSP_VERSION");
    println!("cargo:rerun-if-env-changed=GIT_HASH");
    for path in git_inputs::dependency_paths(Path::new(env!("CARGO_MANIFEST_DIR"))) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    if stamped.is_none() {
        // There is no complete, portable set of filesystem mtimes that models
        // `git status`: an unstaged root-level deletion has no existing child
        // to watch, and a chmod-only change alters neither size nor mtime.
        // Re-run this tiny stamp crate on every unstamped binary build instead.
        // It is intentionally absent from the xtask dependency graph, so the
        // repeated code-generation commands this repository runs stay fresh.
        let out_dir = PathBuf::from(
            std::env::var_os("OUT_DIR").expect("Cargo always supplies a build-script OUT_DIR"),
        );
        let probe = git_inputs::worktree_probe_path(&out_dir);
        println!("cargo:rerun-if-changed={}", probe.display());
    }

    let base = stamped
        .clone()
        .or_else(git_describe)
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());

    // The commit is stamped SEPARATELY rather than being read out of `git
    // describe`, because describe only appends `-<n>-g<hash>` when the build is
    // *not* sitting on a tag — so a release, the one build whose provenance you
    // most need, would carry no hash at all. Worse, CI sets `TCL_LSP_VERSION`
    // from the tag, which short-circuits describe entirely.
    //
    // Result: `2.1.8+g4c5a8f8` at a tag, `2.1.8-3+g4c5a8f8` between releases,
    // `…+g4c5a8f8.dirty` from a modified tree. Semver build metadata (`+…`) is
    // ignored for precedence, so this stays a legal version string.
    let hash = env_hash().or_else(git_hash);

    // `.dirty` means "a developer built this from a modified tree", and it must
    // not fire in CI. A workflow that ships an artefact regenerates TRACKED files
    // on the way (build-wasm.sh rewrites assets/, build-report-assets rewrites
    // dist/), so by the time the crate compiles the tree is dirty *by
    // construction* — and every published build would claim to be a dev build.
    //
    // An explicit `TCL_LSP_VERSION` is the signal that a pipeline, not a person,
    // decided what this is. Trust it and skip the check.
    let dirty = stamped.is_none() && git_is_dirty();

    // Normalise whatever `git describe` produced down to the bare
    // `<tag>[-<n>]` core: it may carry its own `-dirty` and `-g<hash>`, both of
    // which we re-add ourselves in a canonical position. Without this the string
    // says it twice (`2.1.8-dirty+g22a626eb.dirty`).
    let mut version = strip_describe_suffixes(&base);

    if let Some(h) = &hash {
        version.push_str("+g");
        version.push_str(h);
    }
    if dirty {
        // `.dirty` goes inside the build-metadata field, not after a second `+`:
        // semver allows only one `+`, with dot-separated identifiers within it.
        if hash.is_some() {
            version.push_str(".dirty");
        } else {
            version.push_str("-dirty");
        }
    }

    println!("cargo:rustc-env=TCL_LSP_RESOLVED_VERSION={version}");
    println!(
        "cargo:rustc-env=TCL_LSP_RESOLVED_COMMIT={}",
        hash.unwrap_or_default()
    );
}

/// `GIT_HASH`, for builds outside a git checkout (an sdist / vendored build),
/// mirroring the escape hatch the report generator's build script already uses.
fn env_hash() -> Option<String> {
    let raw = std::env::var("GIT_HASH").ok()?;
    let t = raw.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

fn git_hash() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_owned())
}

/// True when the working tree has uncommitted changes, so a dev build can never
/// pass itself off as the release it was cut from.
fn git_is_dirty() -> bool {
    Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .is_some_and(|o| o.status.success() && !o.stdout.is_empty())
}

/// Strip the `-dirty` and `-g<hash>` suffixes `git describe` may have appended,
/// leaving the bare `<tag>[-<n>]` core. Both are re-added in canonical form, so
/// leaving them here would duplicate them.
///
/// `2.1.7-3-g4c5a8f86-dirty` -> `2.1.7-3`, `2.1.8-dirty` -> `2.1.8`
fn strip_describe_suffixes(v: &str) -> String {
    let base = v.strip_suffix("-dirty").unwrap_or(v);
    // A trailing `-g<hex>` is describe's commit suffix. Guard against a genuine
    // pre-release segment like `-gamma`: describe's is `g` followed by hex only.
    if let Some((head, last)) = base.rsplit_once('-')
        && let Some(rest) = last.strip_prefix('g')
        && !rest.is_empty()
        && rest.chars().all(|c| c.is_ascii_hexdigit())
    {
        return head.to_owned();
    }
    base.to_owned()
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

/// `git describe --tags`, deliberately WITHOUT `--always`.
///
/// `--always` degrades to a bare commit hash when no tag is reachable, which is
/// not a version at all — and since the commit is now appended separately, it
/// produced the hash twice (`f69b196+gf69b1965`). A tag-less checkout is the norm
/// in CI, where `actions/checkout` uses `fetch-depth: 1` and fetches no tags, so
/// this is the common path, not an edge case. Returning `None` here lets the
/// caller fall through to the manifest version, giving `0.1.0+gf69b1965`: honest
/// about not knowing the release, still precise about the commit.
fn git_describe() -> Option<String> {
    let out = Command::new("git")
        .args(["describe", "--tags", "--dirty"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None; // no reachable tag
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
