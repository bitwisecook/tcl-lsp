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

//! `tcl-pkg` — the `tclpkg` package manager.
//!
//! Manifest loader, MVS resolver, lockfile I/O, content-addressable store,
//! source fetchers, registry client, virtual
//! environments, and Dockerfile generation. The `tcl pkg` / `tcl venv` /
//! `tcl docker` CLI verb groups in `tcl-cli` drive these modules; behaviour and
//! on-disk formats are stable and canonical.

pub mod cas;
pub mod docker;
pub mod errors;
pub mod exec;
pub mod fetchers;
pub mod hooks;
pub mod installer;
pub mod json;
pub mod lockfile;
pub mod manifest;
pub mod policy;
pub mod registry;
pub mod resolver;
pub mod ui;
pub mod venv;
pub mod version;

pub use errors::{Category, TclPkgError};
pub use version::{Version, VersionError, max_version, parse_version};

/// The per-user directory conventions, re-exported from the `tcl-userdirs`
/// leaf crate.
///
/// They lived here first and every `tcl_pkg::cache_dir()` call site still
/// resolves; the bodies moved out when the LSP server's SpecTcl pack loader
/// needed the same answers and could not reasonably depend on the package
/// manager to get them. One implementation, two consumers.
pub use tcl_userdirs::{cache_dir, config_dir, state_dir, system_config_dir, venv_pool_dir};
