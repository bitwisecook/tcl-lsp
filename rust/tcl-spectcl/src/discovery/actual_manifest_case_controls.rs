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

use super::*;
#[test]
fn distinct_case_equivalent_providers_are_unavailable_in_either_order() {
    for paths in [
        ["/work/tclpkg.tcl", "/work/TCLPKG.TCL"],
        ["/work/TCLPKG.TCL", "/work/tclpkg.tcl"],
        ["/work/tclpkg.lock", "/work/TCLPKG.LOCK"],
    ] {
        let mut files = PackageFiles::default();
        for path in paths {
            files.observe_path(PathBuf::from(path));
        }
        assert!(files.unavailable);
    }
    let store = tcl_lsp_core::vfs::MemoryStore::new();
    store.upsert(
        PathBuf::from("/work/tclpkg.tcl"),
        b"package one\nversion 1.0\n".to_vec(),
    );
    store.upsert(
        PathBuf::from("/work/TCLPKG.TCL"),
        b"package two\nversion 1.0\n".to_vec(),
    );
    let selected = list_package_files(&store, Path::new("/work"));
    assert!(selected.unavailable);
    assert!(selected.manifest.is_none());
    assert!(selected.lockfile.is_none());
}

#[test]
fn single_original_case_path_and_repeated_same_provider_keep_identity() {
    let store = tcl_lsp_core::vfs::MemoryStore::new();
    store.upsert(
        PathBuf::from("/work/TCLPKG.TCL"),
        b"package one\nversion 1.0\n".to_vec(),
    );
    let selected = list_package_files(&store, Path::new("/work"));
    assert!(!selected.unavailable);
    assert_eq!(
        selected.manifest.as_deref(),
        Some(Path::new("/work/TCLPKG.TCL"))
    );
    let mut same = PackageFiles::default();
    same.observe_path(PathBuf::from("/work/TCLPKG.TCL"));
    same.observe_path(PathBuf::from("/work/TCLPKG.TCL"));
    assert!(!same.unavailable);
}
