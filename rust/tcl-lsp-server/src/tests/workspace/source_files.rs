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

//! Source extensions, globs and the directory walk.

use super::*;

#[test]
fn uri_under_folder_handles_trailing_slash_and_boundary() {
    assert!(uri_under_folder(
        "file:///ws/app/file.tcl",
        "file:///ws/app/",
    ));
    assert!(uri_under_folder(
        "file:///ws/app/file.tcl",
        "file:///ws/app",
    ));
    // The folder itself counts as a match.
    assert!(uri_under_folder("file:///ws/app", "file:///ws/app/",));
    // A sibling folder does not.
    assert!(!uri_under_folder(
        "file:///ws/app2/file.tcl",
        "file:///ws/app",
    ));
    assert!(!uri_under_folder(
        "file:///ws/app2/file.tcl",
        "file:///ws/app/",
    ));
}

#[test]
fn is_tcl_source_matches_the_full_tcl_family_extension_set() {
    for ext in [
        "tcl", "tk", "itcl", "tm", "irul", "irule", "iapp", "iappimpl", "impl", "exp", "apl",
        "test",
    ] {
        assert!(
            is_tcl_source(Path::new(&format!("/a/b.{ext}"))),
            "expected .{ext} to be a Tcl-family source",
        );
    }
    assert!(!is_tcl_source(Path::new("/a/b.txt")));
    assert!(!is_tcl_source(Path::new("/a/b")));
}

#[test]
fn is_tcl_source_recognises_tcltest_files() {
    // TP — the standard `tcltest` extension (`test/argparse.test`, and
    // every `.test` file throughout tcllib's own test suite). Omitted from
    // the background workspace scan's allowlist, a proc's call sites
    // living in an un-opened `.test` file are invisible to cross-document
    // find-references / rename-safety.
    assert!(is_tcl_source(Path::new("/ws/test/argparse.test")));
}

/// The extension test must case-fold, like every other extension check in
/// the server (`dialect_from_extension`, `is_bigip_conf_uri`, the APL
/// source check).  An `UPPER.TCL` opened fine but was invisible to the
/// workspace scan, the watched-file filter, and the rename filter — so its
/// procs never reached cross-document navigation until it was opened, and a
/// rename never rewrote the `source` lines pointing at it.
#[test]
fn is_tcl_source_ignores_extension_case() {
    for name in [
        "/ws/UPPER.TCL",
        "/ws/Mixed.Tcl",
        "/ws/rule.IRULE",
        "/ws/suite.TEST",
        "/ws/mod.Tm",
    ] {
        assert!(
            is_tcl_source(Path::new(name)),
            "expected {name} to be a Tcl-family source",
        );
    }
    // NEGATIVE control: case-folding must not widen the set itself.
    assert!(!is_tcl_source(Path::new("/ws/notes.TXT")));
}

/// The file-watcher registration and the `willRename` / `didRename` filter
/// must name exactly the extensions the workspace scan indexes.  Listing
/// fewer leaves a `.test` / `.iapp` / `.exp` file the scan indexed going
/// stale the moment it changes on disk, and keeping its old `source`
/// references through a rename.
#[test]
fn watcher_and_rename_globs_cover_every_indexed_extension() {
    let glob = tcl_source_glob();
    for ext in TCL_SOURCE_EXTENSIONS {
        assert!(
            glob.contains(&format!("{ext},")) || glob.contains(&format!("{ext}}}")),
            "watcher glob {glob} omits .{ext}",
        );
    }
    assert_eq!(
        rename_file_operation_options().filters[0].pattern.glob,
        glob,
        "the rename filter must use the same one list",
    );
    // NEGATIVE control: an extension the scan does not index is absent.
    assert!(!glob.contains("txt"), "{glob}");
}

/// `workspace/didChangeWatchedFiles` carries no `ignoreCase`
/// option and `VS Code` matches watcher globs case-**sensitively** on Linux,
/// so the registration folds case per character instead.
#[test]
fn watch_glob_matches_every_casing_of_every_indexed_extension() {
    let glob = tcl_source_watch_glob();
    for ext in TCL_SOURCE_EXTENSIONS {
        let folded: String = ext.chars().fold(String::new(), |mut acc, c| {
            use std::fmt::Write as _;
            let _ = write!(acc, "[{}{}]", c, c.to_ascii_uppercase());
            acc
        });
        assert!(
            glob.contains(&folded),
            "watch glob {glob} omits a case-folded .{ext} ({folded})",
        );
        // The single-cased spelling must be gone — that is the bug.
        assert!(
            !glob.contains(&format!("{{{ext},")) && !glob.contains(&format!(",{ext},")),
            "watch glob {glob} still carries a single-cased .{ext}",
        );
    }
    assert!(!glob.contains("txt"), "{glob}");
    // Exactly one alternative per indexed extension — no broad `**/*`
    // fallback smuggled in, which would trade correctness for noise.
    assert_eq!(
        glob.matches(',').count() + 1,
        TCL_SOURCE_EXTENSIONS.len(),
        "{glob}",
    );
}

/// `SpecTcl` packs are watched by the *source* glob rather than by a
/// watcher of their own, so this is the assertion that keeps them watched
/// at all: drop `tclspec` from `TCL_SOURCE_EXTENSIONS` and pack reload
/// silently stops working, with nothing else failing.
#[test]
fn the_source_watch_glob_covers_spec_packs() {
    let glob = tcl_source_watch_glob();
    assert!(
        glob.contains("[tT][cC][lL][sS][pP][eE][cC]"),
        "the source watcher must cover `.tclspec`, or a pack edit reaches \
             nothing: {glob}"
    );
    // And the classifier routes such an event to the pack reload rather
    // than to the workspace index.
    let uri: Uri = "file:///w/lib/mylib.TCLSPEC".parse().unwrap();
    assert!(is_spec_pack_file(&uri));
    let partition = partition_watched_file_changes(vec![FileEvent {
        uri,
        typ: FileChangeType::CHANGED,
    }]);
    assert!(partition.spec_pack_changed);
    assert!(
        partition.last_kind.is_empty(),
        "a pack is an analysis input, not an indexed document"
    );
}

#[test]
fn is_skipped_scan_dir_skips_vendor_and_hidden() {
    assert!(is_skipped_scan_dir(Path::new("/a/.git")));
    assert!(is_skipped_scan_dir(Path::new("/a/node_modules")));
    assert!(is_skipped_scan_dir(Path::new("/a/target")));
    assert!(is_skipped_scan_dir(Path::new("/a/tmp")));
    assert!(!is_skipped_scan_dir(Path::new("/a/src")));
    assert!(!is_skipped_scan_dir(Path::new("/a/irules")));
}

#[test]
fn collect_tcl_files_walks_recursively_and_skips_vendor() {
    let root = unique_scratch_dir("walk");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::create_dir_all(root.join("node_modules")).unwrap();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::write(root.join("top.tcl"), "set x 1\n").unwrap();
    std::fs::write(root.join("mod.tm"), "set y 2\n").unwrap();
    std::fs::write(root.join("readme.txt"), "not tcl\n").unwrap();
    std::fs::write(root.join("sub/nested.tcl"), "set z 3\n").unwrap();
    std::fs::write(root.join("node_modules/dep.tcl"), "set q 4\n").unwrap();
    std::fs::write(root.join(".git/hook.tcl"), "set w 5\n").unwrap();

    let mut out = Vec::new();
    collect_tcl_files(&vfs::NativeStore, &root, 100, &mut out);
    let mut names: Vec<String> = out
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();

    std::fs::remove_dir_all(&root).ok();

    assert_eq!(names, vec!["mod.tm", "nested.tcl", "top.tcl"]);
}

#[test]
fn collect_tcl_files_picks_up_tcltest_files() {
    // TP — a `.test` file (the standard `tcltest` extension; every mined
    // corpus, and tcllib's own test suite, use it throughout, e.g.
    // `test/argparse.test`). Invisible to the background workspace scan,
    // cross-document find-references / rename-safety silently miss call
    // sites living in an un-opened `.test` file.
    let root = unique_scratch_dir("tcltest");
    std::fs::create_dir_all(root.join("test")).unwrap();
    std::fs::write(root.join("lib.tcl"), "proc plain {} { return 1 }\n").unwrap();
    std::fs::write(root.join("test/argparse.test"), "plain\n").unwrap();

    let mut out = Vec::new();
    collect_tcl_files(&vfs::NativeStore, &root, 100, &mut out);
    let mut names: Vec<String> = out
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();

    std::fs::remove_dir_all(&root).ok();

    assert_eq!(names, vec!["argparse.test", "lib.tcl"]);
}

#[test]
fn collect_tcl_files_reads_a_memory_store_workspace() {
    let store = vfs::MemoryStore::new();
    store.upsert("/ws/main.tcl", b"".to_vec());
    store.upsert("/ws/lib/helpers.tcl", b"".to_vec());
    store.upsert("/ws/tmp/mypkg/impl.tcl", b"".to_vec());
    let mut out = Vec::new();
    collect_tcl_files(&store, Path::new("/ws"), 100, &mut out);
    let mut names: Vec<String> = out
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    names.sort();
    // `tmp/` is a skipped scan directory, exactly as on disk.
    assert_eq!(names, vec!["/ws/lib/helpers.tcl", "/ws/main.tcl"]);
}

#[test]
fn collect_tcl_files_respects_cap() {
    let root = unique_scratch_dir("cap");
    for i in 0..10 {
        std::fs::write(root.join(format!("f{i}.tcl")), "set x 1\n").unwrap();
    }
    let mut out = Vec::new();
    let complete = collect_tcl_files(&vfs::NativeStore, &root, 3, &mut out);
    let got = out.len();
    std::fs::remove_dir_all(&root).ok();
    assert_eq!(got, 3);
    assert!(!complete, "the capped scan must not authorize absence");
}
