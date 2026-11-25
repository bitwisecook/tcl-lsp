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
use tcl_lsp_core::vfs::MemoryStore;

#[test]
fn unavailable_nested_manifest_cannot_borrow_parent_implicit_scan() {
    let store = MemoryStore::new();
    store.upsert(
        PathBuf::from("/project/tclpkg.tcl"),
        b"package require x\n".to_vec(),
    );
    store.upsert(
        PathBuf::from("/project/visible.tclspec"),
        b"speclib visible 1 {}".to_vec(),
    );
    store.upsert(
        PathBuf::from("/project/nested/tclpkg.tcl"),
        b"spec {packs".to_vec(),
    );
    store.upsert(
        PathBuf::from("/project/nested/hidden.tclspec"),
        b"speclib hidden 1 {}".to_vec(),
    );
    let mut found = BTreeSet::new();
    let mut inputs = ManifestSelectionInputs::default();
    collect_beside_manifests_observed(&store, Path::new("/project"), &mut found, &mut inputs);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found.first().unwrap().path,
        Path::new("/project/visible.tclspec")
    );
    assert_eq!(inputs.sources.len(), 2);
    assert_eq!(inputs.issues.len(), 1);
    let mut loaded = crate::pack::LoadedSourceInputs::default();
    loaded.retain_manifest_selection(&store, &inputs);
    assert!(loaded.selected(&[]).is_none());
}

#[test]
fn complete_no_spec_selection_retains_actual_manifest_source_dependency() {
    let store = MemoryStore::new();
    let path = PathBuf::from("/project/tclpkg.tcl");
    let source = "package require x\n";
    store.upsert(path.clone(), source.as_bytes().to_vec());
    let mut found = BTreeSet::new();
    let mut inputs = ManifestSelectionInputs::default();
    collect_beside_manifests_observed(&store, Path::new("/project"), &mut found, &mut inputs);
    assert!(found.is_empty());
    assert_eq!(inputs.issues, [] as [(std::path::PathBuf, String); 0]);
    assert_eq!(inputs.sources, vec![(path.clone(), source.to_owned())]);
    let mut loaded = crate::pack::LoadedSourceInputs::default();
    loaded.retain_manifest_selection(&store, &inputs);
    let selected = loaded.selected(&[]).unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].0.path(), path);
    assert_eq!(selected[0].1, source.as_bytes());
}
