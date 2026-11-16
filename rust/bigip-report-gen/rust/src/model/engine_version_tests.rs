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

/// The report header renders `query engine v{{ engine_version }}`, and read
/// `0.1.0` — the never-bumped workspace manifest version — for every release.
///
/// The guard is "comes from `tcl-version`", not "looks like a tag": CI checks
/// out with `fetch-depth: 1` and no tags, so no tag is reachable there and the
/// resolved version legitimately falls back to the manifest base (`0.1.0`,
/// plus the commit). Asserting a tag-shaped string would fail on every PR.
#[test]
fn engine_version_comes_from_tcl_version_not_cargo_pkg_version() {
    assert_eq!(
        super::ENGINE_VERSION,
        tcl_version::VERSION,
        "ENGINE_VERSION must be sourced from tcl-version (which resolves the \
             release tag), not from CARGO_PKG_VERSION (the workspace manifest's \
             permanent 0.1.0 placeholder)"
    );
}

/// Whatever the base resolves to, the commit must always be present — that is
/// the whole point of stamping it separately from `git describe`.
#[test]
fn engine_version_carries_the_commit() {
    let v = super::ENGINE_VERSION;
    assert!(
        v.contains("+g") || tcl_version::COMMIT.is_empty(),
        "expected the commit as build metadata, got {v:?}"
    );
}
