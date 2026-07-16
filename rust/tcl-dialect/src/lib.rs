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

//! Foundational dialect vocabulary for the whole Tcl toolchain.
//!
//! This crate is the single owner of what a *dialect* means:
//!
//! - [`DialectSet`] — the compact membership bitflags commands and options
//!   are tagged with (the availability atom).
//! - [`TclVersion`] — the ordered Tcl release enum behaviour/const-fold
//!   semantics key off.
//! - [`LexerGrammar`] / [`BracedVarStyle`] — the dialect-derived lexing
//!   grammar knobs.
//! - [`DialectProfile`] — one interned profile per canonical dialect,
//!   resolved once at ingest and threaded everywhere.
//! - [`LibraryPin`] / [`LibraryVersion`] / [`VersionKey`] — the
//!   versioned-library axis (§7.1, D5): which packages a profile ships
//!   ambiently and at what version floor.
//!
//! It deliberately depends on nothing (beyond `bitflags`) so every layer —
//! `tcl-lexer` and `tcl-syntax` *below* the registry included — can consume
//! the same source of truth without a dependency cycle. Dialect *detection*
//! heuristics (which need the lexer) stay in `tcl-registry::dialects`.
//!
//! See `docs/design/dialect-profile-model.md` for the full model.

#![deny(missing_docs)]

mod dialect_set;
mod grammar;
mod library;
mod profile;
mod version;

pub use dialect_set::{DialectSet, KNOWN_DIALECTS, available_dialects};
pub use grammar::{BracedVarStyle, LexerGrammar};
pub use library::{LibraryPin, LibraryVersion, LibraryVersionOverrides, VersionKey};
pub use profile::DialectProfile;
pub use version::{TclVersion, Ternary};

/// Crate version string.
///
/// ```
/// assert!(!tcl_dialect::VERSION.is_empty());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
