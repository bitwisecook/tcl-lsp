// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The version every user-facing binary reports.
//!
//! Resolved at compile time by `build.rs` from `TCL_LSP_VERSION`, else
//! `git describe`, else the manifest version. The workspace manifest carries
//! `0.1.0` and is never bumped: releases are tag-only, so the annotated tag is
//! the single source of truth.
//!
//! The commit is always appended as semver build metadata (`+g<hash>`), because
//! `git describe` alone would omit it from precisely the builds whose provenance
//! matters most: it only appends `-<n>-g<hash>` when the build is *not* on a tag,
//! so a release would carry no hash at all. CI compounds this by setting
//! `TCL_LSP_VERSION` from the tag, which bypasses `git describe` entirely.
//!
//! ```text
//! at a tag              2.1.8+g4c5a8f86
//! 3 commits past a tag  2.1.8-3+g4c5a8f86
//! modified working tree  2.1.8+g4c5a8f86.dirty
//! ```
//!
//! ```ignore
//! #[command(version = tcl_version::VERSION)]
//! struct Cli { /* … */ }
//! ```

/// The resolved release version, without a leading `v` (e.g. `2.1.8+g4c5a8f86`).
///
/// Always carries the commit as semver build metadata (`+g<hash>`), and
/// `.dirty` when built from a modified tree — so a development binary can never
/// pass itself off as the release it was cut from. Build metadata is ignored for
/// semver precedence, so this remains a legal version string.
pub const VERSION: &str = env!("TCL_LSP_RESOLVED_VERSION");

/// The short commit hash the binary was built from (e.g. `4c5a8f86`), or empty
/// when built outside a git checkout with no `GIT_HASH` override.
///
/// Already embedded in [`VERSION`]; exposed separately for callers that want to
/// show it in its own field rather than parse it back out.
pub const COMMIT: &str = env!("TCL_LSP_RESOLVED_COMMIT");

// Compile the build-script support into this crate only for its regression
// tests. `build.rs` includes the same source directly, keeping the resolver's
// tested implementation and the one Cargo executes identical.
#[cfg(test)]
mod git_inputs;
