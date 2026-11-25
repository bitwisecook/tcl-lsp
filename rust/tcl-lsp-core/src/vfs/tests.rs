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

use super::{
    MemoryStore, NativeStore, SourceStore as _, WorkspaceSourceInventoryCompleteness,
    WorkspaceSourceInventoryObservation,
};
use std::path::Path;

#[test]
fn a_stored_file_reads_back() {
    let store = MemoryStore::new();
    store.upsert("/w/a.tcl", b"puts hi\n".to_vec());
    assert_eq!(
        store.read_to_string(Path::new("/w/a.tcl")).expect("stored"),
        "puts hi\n"
    );
    assert!(store.is_file(Path::new("/w/a.tcl")));
    assert!(!store.is_dir(Path::new("/w/a.tcl")));
}

#[test]
fn a_host_inventory_receipt_is_bound_to_the_current_store_revision() {
    let store = MemoryStore::new();
    let root = Path::new("/workspace");
    store.upsert("/workspace/a.tcl", b"puts a\n".to_vec());
    store.set_workspace_source_inventory(
        root,
        r#"{"globs":["**/*.tcl"],"maxFiles":8}"#,
        WorkspaceSourceInventoryCompleteness::Complete,
    );
    let first = store.workspace_source_inventory(root);
    assert!(first.admits_complete_inventory());

    store.upsert("/workspace/b.tcl", b"puts b\n".to_vec());
    assert!(matches!(
        store.workspace_source_inventory(root),
        WorkspaceSourceInventoryObservation::Stale { .. }
    ));
    store.set_workspace_source_inventory(
        root,
        r#"{"globs":["**/*.tcl"],"maxFiles":8}"#,
        WorkspaceSourceInventoryCompleteness::Incomplete,
    );
    assert!(
        !store
            .workspace_source_inventory(root)
            .admits_complete_inventory()
    );
}

#[test]
fn a_missing_file_is_not_found_rather_than_a_panic() {
    let store = MemoryStore::new();
    let err = store.read(Path::new("/w/gone.tcl")).expect_err("missing");
    assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn an_ill_formed_byte_still_decodes_through_read_source() {
    let store = MemoryStore::new();
    store.upsert("/w/latin.tcl", vec![b'p', 0xFF, b'\n']);
    let (text, report) = store
        .read_source(Path::new("/w/latin.tcl"))
        .expect("stored");
    assert!(text.contains('p'));
    assert!(!report.is_faithful());
}

#[test]
fn a_uri_alias_finds_the_file_again() {
    let store = MemoryStore::new();
    store.upsert_uri("file:///w/a.tcl", "/w/a.tcl", b"set x 1\n".to_vec());
    assert_eq!(
        store.path_for_uri("file:///w/a.tcl").expect("alias"),
        Path::new("/w/a.tcl")
    );
    assert!(store.remove_uri("file:///w/a.tcl"));
    assert!(!store.exists(Path::new("/w/a.tcl")));
}

#[test]
fn explicit_empty_directories_do_not_supply_files_or_census_authority() {
    let store = MemoryStore::new();
    let root = Path::new("/empty");
    assert_eq!(
        store.read_dir(root).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    store.create_dir_all(root).unwrap();
    assert!(store.is_dir(root));
    assert!(!store.is_file(root));
    assert_eq!(
        store.read_dir(root).unwrap(),
        [] as [crate::vfs::DirEntry; 0]
    );
    assert_eq!(store.len(), 0);
    assert!(matches!(
        store.workspace_source_inventory(root),
        WorkspaceSourceInventoryObservation::Missing { .. }
    ));
    store.set_workspace_source_inventory(
        root,
        "empty-source-selection",
        WorkspaceSourceInventoryCompleteness::Complete,
    );
    store.create_dir_all(root).unwrap();
    assert!(
        store
            .workspace_source_inventory(root)
            .admits_complete_inventory()
    );
    store.create_dir_all(Path::new("/empty/nested")).unwrap();
    assert!(matches!(
        store.workspace_source_inventory(root),
        WorkspaceSourceInventoryObservation::Stale { .. }
    ));
    assert_eq!(
        store.read_dir(Path::new("/empty/nested")).unwrap(),
        [] as [crate::vfs::DirEntry; 0]
    );
}

#[test]
fn explicit_and_implicit_directories_share_the_actual_listing() {
    let store = MemoryStore::new();
    store.create_dir_all(Path::new("/w/empty/nested")).unwrap();
    store.upsert("/w/code/a.tcl", b"source".to_vec());
    let mut entries = store.read_dir(Path::new("/w")).unwrap();
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    assert_eq!(
        entries
            .iter()
            .map(|entry| (entry.path.as_path(), entry.is_dir, entry.is_file))
            .collect::<Vec<_>>(),
        vec![
            (Path::new("/w/code"), true, false),
            (Path::new("/w/empty"), true, false)
        ]
    );
    assert!(store.is_dir(Path::new("/w/empty/nested")));
    assert!(store.remove(Path::new("/w/code/a.tcl")));
    assert!(!store.exists(Path::new("/w/code")));
    assert!(store.is_dir(Path::new("/w/empty")));
}

#[test]
fn directory_file_conflicts_leave_namespace_and_census_unchanged() {
    let store = MemoryStore::new();
    store.upsert("/w/file", b"source".to_vec());
    store.set_workspace_source_inventory(
        "/w",
        "one-file",
        WorkspaceSourceInventoryCompleteness::Complete,
    );
    assert_eq!(
        store
            .create_dir_all(Path::new("/w/file"))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        store
            .create_dir_all(Path::new("/w/file/nested"))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotADirectory
    );
    assert_eq!(
        store.read_dir(Path::new("/w/file")).unwrap_err().kind(),
        std::io::ErrorKind::NotADirectory
    );
    assert!(!store.exists(Path::new("/w/file/nested")));
    assert_eq!(store.read(Path::new("/w/file")).unwrap(), b"source");
    assert!(
        store
            .workspace_source_inventory(Path::new("/w"))
            .admits_complete_inventory()
    );
}

#[test]
fn reverse_file_conflicts_preserve_aliases_bytes_and_census() {
    let store = MemoryStore::new();
    store.create_dir_all(Path::new("/w/d/nested")).unwrap();
    store.upsert_uri("file:///original", "/w/original", b"ORIGINAL".to_vec());
    store.upsert("/w/file", b"FILE".to_vec());
    let selected = store.document_uri_path("file:///original").unwrap();
    store.set_workspace_source_inventory(
        "/w",
        "original-files",
        WorkspaceSourceInventoryCompleteness::Complete,
    );
    for path in ["/w/d", "/w", "/w/file/child"] {
        assert!(
            store.try_upsert(path, b"REFUSED".to_vec()).is_err(),
            "{path}"
        );
        assert!(store.write(Path::new(path), b"REFUSED").is_err(), "{path}");
        assert!(
            store
                .try_upsert_uri_aliases(
                    &["file:///original", "file:///new"],
                    path,
                    b"REFUSED".to_vec()
                )
                .is_err(),
            "{path}"
        );
    }
    store.upsert("/w/d", b"REFUSED".to_vec());
    store.upsert_uri("file:///new", "/w/d", b"REFUSED".to_vec());
    store.validate_document_uri_path(&selected).unwrap();
    assert!(store.path_for_uri("file:///new").is_none());
    assert_eq!(store.read(Path::new("/w/original")).unwrap(), b"ORIGINAL");
    assert_eq!(store.read(Path::new("/w/file")).unwrap(), b"FILE");
    assert!(store.is_dir(Path::new("/w/d")));
    assert!(store.is_dir(Path::new("/w/d/nested")));
    assert!(!store.exists(Path::new("/w/file/child")));
    assert!(
        store
            .workspace_source_inventory(Path::new("/w"))
            .admits_complete_inventory()
    );
}

#[test]
fn directories_are_implied_by_the_files_under_them() {
    let store = MemoryStore::new();
    store.upsert("/w/lib/a.tcl", b"".to_vec());
    store.upsert("/w/b.tcl", b"".to_vec());
    assert!(store.is_dir(Path::new("/w")));
    let mut names: Vec<_> = store
        .read_dir(Path::new("/w"))
        .expect("listing")
        .into_iter()
        .map(|e| (e.path, e.is_dir))
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            (Path::new("/w/b.tcl").to_path_buf(), false),
            (Path::new("/w/lib").to_path_buf(), true),
        ]
    );
}

#[test]
fn a_write_is_readable_again() {
    let store = MemoryStore::new();
    store
        .write(Path::new("/cfg/config.ini"), b"[features]\n")
        .expect("write");
    assert_eq!(
        store
            .read_to_string(Path::new("/cfg/config.ini"))
            .expect("written"),
        "[features]\n"
    );
}

#[test]
fn the_native_store_is_std_fs() {
    let dir = std::env::temp_dir().join("tcl-lsp-vfs-native-store");
    let store = NativeStore;
    store.create_dir_all(&dir).expect("mkdir");
    let file = dir.join("probe.tcl");
    store.write(&file, b"proc p {} {}\n").expect("write");
    assert_eq!(store.read_to_string(&file).expect("read"), "proc p {} {}\n");
    assert!(store.is_dir(&dir));
    assert!(
        store
            .read_dir(&dir)
            .expect("listing")
            .iter()
            .any(|e| e.path == file)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
