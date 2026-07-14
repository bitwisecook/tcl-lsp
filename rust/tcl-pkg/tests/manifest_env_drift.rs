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

//! Drift gate: the registry's `tclpkg.tcl` whole-file scoped environment
//! (`TCLPKG_MANIFEST_ENV`) must describe exactly the directive set this
//! crate's manifest evaluator accepts — the two live in different crates
//! (command surface is registry data; evaluation is `tcl-pkg`), so a
//! directive added to one without the other fails here, not in an editor.

use std::collections::BTreeSet;

#[test]
fn registry_manifest_env_matches_evaluator_directives() {
    let evaluator: BTreeSet<&str> = tcl_pkg::manifest::directive_names()
        .iter()
        .copied()
        .collect();
    let registry: BTreeSet<&str> = tcl_registry::scoped::TCLPKG_MANIFEST_ENV
        .commands
        .iter()
        .map(|c| c.name)
        .collect();
    assert_eq!(
        registry, evaluator,
        "tcl-registry TCLPKG_MANIFEST_ENV and tcl-pkg manifest DIRECTIVES have drifted"
    );
}

/// The registry file-scope hook resolves the manifest by basename, from any
/// directory, and never for other names.
#[test]
fn file_scope_env_keys_on_the_manifest_basename() {
    assert!(tcl_registry::scoped::file_scope_env("/proj/tclpkg.tcl").is_some());
    assert!(tcl_registry::scoped::file_scope_env("tclpkg.tcl").is_some());
    assert!(tcl_registry::scoped::file_scope_env(r"C:\proj\tclpkg.tcl").is_some());
    assert!(tcl_registry::scoped::file_scope_env("/proj/other.tcl").is_none());
    assert!(tcl_registry::scoped::file_scope_env("/proj/pkgIndex.tcl").is_none());
}
