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

//! `tcl-pkg-model` — the `tclpkg` data model.
//!
//! The manifest loader, the lockfile, the version type and the error type
//! of the package manager (`tcl-pkg`), and the one derivation of how far a
//! package sits from the workspace root ([`tier::dependency_tier`]).
//!
//! They live in a crate of their own because the `SpecTcl` pack loader reads
//! them: what a pack a package ships may declare depends on the package's
//! place in the lockfile's graph, and the loader cannot reasonably depend on
//! the package manager for it. `tcl-pkg` carries the network client, the
//! archive readers and the process sandbox, none of which builds for the
//! browser hosts that load packs. One implementation, two consumers;
//! `tcl-pkg` re-exports the modules, so `tcl_pkg::manifest::load_manifest`
//! still resolves.

pub mod errors;
pub mod json;
pub mod lockfile;
pub mod manifest;
pub mod tier;
pub mod version;

pub use errors::{Category, TclPkgError};
pub use version::{Version, VersionError, max_version, parse_version};
