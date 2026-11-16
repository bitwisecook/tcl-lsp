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
fn malformed_reserved_path_cannot_fall_through_as_workspace_metadata() {
    let store = MemoryStore::new();
    let malformed = PathBuf::from("/ws/.tcl-lsp/.spec-studio/panel/../retired.tclspec");
    store.upsert(&malformed, b"speclib retired 1 {}".to_vec());
    let options = DiscoveryOptions {
        workspace_roots: vec![PathBuf::from("/ws")],
        configured: vec![malformed.clone()],
        skip_user_tier: true,
        bundled_dir: Some(PathBuf::from("/no-bundled")),
        ..DiscoveryOptions::default()
    };
    assert!(!is_workspace_studio_source(
        &malformed,
        &options.workspace_roots
    ));
    assert!(discover_with_active_studio_in(&store, &options, &[]).is_empty());
}

#[cfg(unix)]
#[test]
fn native_symlink_workspace_quarantines_canonical_retired_studio_files() {
    let nonce = tcl_lsp_core::wire_session::EditorWireSession::allocate().unwrap();
    let root = std::env::temp_dir().join(format!("tcl-studio-quarantine-{nonce:?}"));
    let real = root.join("real");
    let linked = root.join("linked");
    let reserved = real.join(WORKSPACE_PACK_DIR).join(STUDIO_OVERRIDE_DIR);
    std::fs::create_dir_all(reserved.join("panel")).unwrap();
    std::fs::write(reserved.join("retired.tclspec"), "speclib retired 1 {}").unwrap();
    std::os::unix::fs::symlink(&real, &linked).unwrap();
    let options = DiscoveryOptions {
        workspace_roots: vec![linked.clone()],
        configured: vec![
            linked
                .join(WORKSPACE_PACK_DIR)
                .join(STUDIO_OVERRIDE_DIR)
                .join("panel/../retired.tclspec"),
        ],
        skip_user_tier: true,
        bundled_dir: Some(root.join("no-bundled")),
        ..DiscoveryOptions::default()
    };
    assert!(discover_with_active_studio_in(&NativeStore, &options, &[]).is_empty());
    assert!(
        discover_in(&NativeStore, &options)
            .iter()
            .any(|file| file.tier == Tier::StudioOverride)
    );
    std::fs::remove_dir_all(root).unwrap();
}
