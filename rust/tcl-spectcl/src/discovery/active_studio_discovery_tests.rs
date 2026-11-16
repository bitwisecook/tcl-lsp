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
fn checked_studio_discovery_requires_explicit_paths_and_preserves_ordinary_tiers() {
    let store = MemoryStore::new();
    let active = PathBuf::from("/ws/.tcl-lsp/.spec-studio/a/active.tclspec");
    let retired = PathBuf::from("/ws/.tcl-lsp/.spec-studio/b/active.tclspec");
    for path in [&active, &retired] {
        store.upsert(path, b"speclib studio 1 {}".to_vec());
    }
    store.upsert(
        "/ws/.tcl-lsp/ordinary.tclspec",
        b"speclib ordinary 1 {}".to_vec(),
    );
    store.upsert("/user/user.tclspec", b"speclib user 1 {}".to_vec());
    store.upsert("/bundled/bundled.tclspec", b"speclib bundled 1 {}".to_vec());
    let options = DiscoveryOptions {
        workspace_roots: vec![PathBuf::from("/ws")],
        configured: vec![retired.clone()],
        user_dir: Some(PathBuf::from("/user")),
        bundled_dir: Some(PathBuf::from("/bundled")),
        ..DiscoveryOptions::default()
    };
    let closed = discover_with_active_studio_in(&store, &options, &[]);
    assert_eq!(
        closed.iter().map(|file| file.tier).collect::<Vec<_>>(),
        vec![Tier::Workspace, Tier::User, Tier::Bundled]
    );
    let opened = discover_with_active_studio_in(&store, &options, std::slice::from_ref(&active));
    assert_eq!(opened[0].tier, Tier::StudioOverride);
    assert_eq!(opened[0].path, active);
    assert!(!opened.iter().any(|file| file.path == retired));
    assert_eq!(opened[1..], closed);
    assert!(!is_workspace_studio_source(
        Path::new("/ws/.tcl-lsp/.spec-studio/a/../outside.tclspec"),
        &options.workspace_roots
    ));
    assert!(!is_workspace_studio_source(
        Path::new("/other/.tcl-lsp/.spec-studio/a/active.tclspec"),
        &options.workspace_roots
    ));
}
