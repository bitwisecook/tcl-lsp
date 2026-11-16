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
fn actual_scan_bound_cannot_donate_unvisited_nested_manifest_subtrees_to_parent() {
    let store = MemoryStore::new();
    store.upsert("/project/tclpkg.tcl", b"package require x\n".to_vec());
    store.upsert("/project/visible.tclspec", b"speclib visible 1 {}".to_vec());
    // Fixed finite fixture just beyond the actual production scan budget.
    // Ordinary observed directories remain usable; the unvisited frontier
    // cannot masquerade as complete absence of a nested manifest.
    for index in 0..=MANIFEST_SCAN_DIR_CAP {
        store.upsert(format!("/project/d{index:04}/marker"), Vec::<u8>::new());
    }
    let tail = format!("/project/d{MANIFEST_SCAN_DIR_CAP:04}");
    store.upsert(format!("{tail}/tclpkg.tcl"), b"spec {packs".to_vec());
    store.upsert(
        format!("{tail}/hidden.tclspec"),
        b"speclib hidden 1 {}".to_vec(),
    );
    let mut selected = BTreeSet::new();
    let mut inputs = ManifestSelectionInputs::default();
    collect_beside_manifests_observed(&store, Path::new("/project"), &mut selected, &mut inputs);
    assert_eq!(selected.len(), 1);
    assert_eq!(
        selected.first().unwrap().path,
        Path::new("/project/visible.tclspec")
    );
    assert!(
        inputs
            .unavailable_subtrees
            .iter()
            .any(|path| Path::new(&tail).starts_with(path))
    );
    assert!(
        inputs
            .issues
            .iter()
            .any(|(_, issue)| issue.contains("directory bound"))
    );
    let mut loaded = crate::pack::LoadedSourceInputs::default();
    loaded.retain_manifest_selection(&store, &inputs);
    assert!(loaded.selected(&[]).is_none());
}
