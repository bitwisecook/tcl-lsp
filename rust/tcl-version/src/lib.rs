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
//! ```ignore
//! #[command(version = tcl_version::VERSION)]
//! struct Cli { /* … */ }
//! ```

/// The resolved release version, without a leading `v` (e.g. `2.1.5`).
///
/// A build from a working tree between releases reports `git describe` output
/// (`2.1.5-3-gabc1234`, `-dirty` when the tree is modified), so a development
/// binary never claims to be a release.
pub const VERSION: &str = env!("TCL_LSP_RESOLVED_VERSION");
