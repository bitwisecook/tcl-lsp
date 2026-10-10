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
pub mod exec;
pub mod fetchers;
pub mod hooks;
pub mod installer;
pub mod policy;
pub mod registry;
pub mod resolver;
pub mod ui;
pub mod venv;

/// The manifest, lockfile, version and error modules, re-exported from the
/// `tcl-pkg-model` leaf crate.
///
/// The implementation lives in `tcl-pkg-model` because the `SpecTcl` pack
/// loader reads the same manifest and lockfile (a package's place in the
/// lockfile's graph decides what a pack it ships may declare) and cannot
/// reasonably depend on the package manager to do so: this crate carries the
/// network client, the archive readers and the process sandbox, none of
/// which builds for the browser hosts. One implementation, two consumers,
/// and `tcl_pkg::manifest::load_manifest` still resolves for callers that
/// expect it here.
pub use tcl_pkg_model::{errors, json, lockfile, manifest, version};

pub use errors::{Category, TclPkgError};
pub use version::{Version, VersionError, max_version, parse_version};

/// Set `lockfile`'s `generated` timestamp to now (UTC, ISO-8601, second
/// precision, `Z` suffix). A lockfile is read and compared without a clock;
/// only the installer that writes one stamps it.
pub fn stamp_lockfile(lockfile: &mut lockfile::LockFile) {
    lockfile.generated = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
}

/// The per-user directory conventions, re-exported from the `tcl-userdirs`
/// leaf crate.
///
/// The implementation lives in `tcl-userdirs` because the LSP server's
/// `SpecTcl` pack loader needs the same answers and cannot reasonably depend
/// on the package manager to get them. One implementation, two consumers,
/// and `tcl_pkg::cache_dir()` still resolves for callers that expect it here.
pub use tcl_userdirs::{cache_dir, config_dir, state_dir, system_config_dir, venv_pool_dir};
