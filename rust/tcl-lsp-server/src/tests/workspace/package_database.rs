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

//! The package resolver and the `auto_path` documents contribute.

use super::*;

#[test]
fn effective_auto_path_layers_editor_project_and_discovery() {
    // editor libraryPaths → global → project .tcl-lsp.ini → discovered →
    // TCLLIBPATH, deduped and in priority order.
    let ws = TmpWs::new("ap");
    ws.write(".tcl-lsp.ini", "[project]\nlibraryPaths = /proj/lib\n");
    let editor = vec!["/editor/lib".to_owned()];
    let discovered = vec![core_tcl_install::TclInstallation {
        version: "8.6".to_owned(),
        tcl_library: PathBuf::from("/sys/lib/tcl8.6"),
        auto_path: vec![PathBuf::from("/sys/lib"), PathBuf::from("/sys/lib/tcl8.6")],
    }];
    let ap = effective_auto_path(
        &vfs::NativeStore,
        std::slice::from_ref(&ws.0),
        &editor,
        &discovered,
    );
    assert!(ap.contains(&PathBuf::from("/editor/lib")));
    assert!(ap.contains(&PathBuf::from("/proj/lib")));
    assert!(ap.contains(&PathBuf::from("/sys/lib")));
    let pos = |p: &str| ap.iter().position(|x| x == &PathBuf::from(p)).unwrap();
    assert!(pos("/editor/lib") < pos("/proj/lib"));
    assert!(pos("/proj/lib") < pos("/sys/lib"));
}

#[test]
fn build_package_resolver_picks_up_a_discovered_installation_package() {
    // A discovered installation whose `auto_path` holds a package's
    // pkgIndex.tcl is scanned, so the database can resolve it — the
    // mechanism that lets W120 refinement see system-installed Tk.
    let sys = TmpWs::new("sys");
    sys.write(
        "lib/tk8.6/pkgIndex.tcl",
        "package ifneeded Tk 8.6 [list load [file join $dir libtk8.6.so] Tk]\n",
    );
    // The package has no source file (a C extension loaded via `load`); the
    // fallback lists the dir's *.tcl — none here, so give it a marker .tcl.
    sys.write("lib/tk8.6/tk.tcl", "# tk\n");
    let lib_root = sys.0.join("lib");
    let discovered = vec![core_tcl_install::TclInstallation {
        version: "8.6".to_owned(),
        tcl_library: lib_root.join("tcl8.6"),
        auto_path: vec![lib_root.clone()],
    }];
    let resolver = build_package_resolver(&vfs::NativeStore, &[], &[], &discovered, 100);
    assert!(
        resolver.provides("Tk"),
        "discovered install's Tk package should be in the database"
    );
}

/// TP: a file's own `lappend auto_path [file dirname
/// [file dirname [info script]]]` must feed the package database, so the
/// repo-local package it points at resolves even when the workspace root
/// is the `examples/` subfolder that does **not** enclose it.
///
/// tclsh 9.0.4 / 8.6.16 oracle on this exact layout: running
/// `examples/user.tcl` by absolute path prints `loaded:foo.png` — the
/// `package require` resolves deterministically through the computed
/// `auto_path` entry.
#[tokio::test]
async fn document_auto_path_mutation_feeds_the_package_database() {
    let ws = TmpWs::new("autopath");
    ws.write(
        "reporoot/pkgIndex.tcl",
        "package ifneeded mypix 1.0 [list source [file join $dir mypix.tcl]]\n",
    );
    ws.write(
        "reporoot/mypix.tcl",
        "package provide mypix 1.0\nproc ::mypix::readImage {path} { return \"loaded:$path\" }\n",
    );
    ws.write(
        "reporoot/examples/user.tcl",
        "lappend auto_path [file dirname [file dirname [info script]]]\n\
             package require mypix\n\
             puts [::mypix::readImage foo.png]\n",
    );

    let backend = test_backend();
    // Root is `examples/` only: the package directory is the *parent*, so
    // the recursive tree scan can never reach it.
    let narrow_root = Uri::from_file_path(ws.0.join("reporoot/examples")).unwrap();
    *backend.workspace_folders.lock().await = vec![narrow_root];
    backend.scan_workspace_folders().await.unwrap();
    assert!(
        backend.package_resolver.read().await.provides("mypix"),
        "the file's own auto_path mutation must put the package directory \
             on the resolver's search path",
    );

    // NEGATIVE control: the identical workspace with the `auto_path` line
    // removed leaves the package unknown — proving the resolution above
    // comes from the mutation, not from the root scan.
    let bare = TmpWs::new("autopath-bare");
    bare.write(
        "reporoot/pkgIndex.tcl",
        "package ifneeded mypix2 1.0 [list source [file join $dir mypix.tcl]]\n",
    );
    bare.write("reporoot/mypix.tcl", "package provide mypix2 1.0\n");
    bare.write(
        "reporoot/examples/user.tcl",
        "package require mypix2\nputs hi\n",
    );
    let backend2 = test_backend();
    let bare_root = Uri::from_file_path(bare.0.join("reporoot/examples")).unwrap();
    *backend2.workspace_folders.lock().await = vec![bare_root];
    backend2.scan_workspace_folders().await.unwrap();
    assert!(
        !backend2.package_resolver.read().await.provides("mypix2"),
        "without an auto_path mutation a package outside the root must stay \
             unknown — the fix must not widen the search path on its own",
    );
}

/// TP end to end: with the package directory outside
/// the workspace root, go-to-definition on a command the required package
/// provides resolves through the package tier
/// (`ensure_required_packages_indexed`) seeded by the file's own
/// `auto_path` mutation.
#[tokio::test]
async fn definition_resolves_through_a_document_auto_path_package() {
    let ws = TmpWs::new("autopath-def");
    ws.write(
        "reporoot/pkgIndex.tcl",
        "package ifneeded mypix 1.0 [list apply {dir {source [file join $dir mypix.tcl]}} $dir]\n",
    );
    ws.write(
        "reporoot/mypix.tcl",
        "package provide mypix 1.0\nproc ::mypix::readImage {path} { return \"loaded:$path\" }\n",
    );
    let user_src = "lappend auto_path [file dirname [file dirname [info script]]]\n\
                        package require mypix\n\
                        puts [::mypix::readImage foo.png]\n";
    ws.write("reporoot/examples/user.tcl", user_src);

    let backend = test_backend();
    let narrow_root = Uri::from_file_path(ws.0.join("reporoot/examples")).unwrap();
    *backend.workspace_folders.lock().await = vec![narrow_root];
    backend.scan_workspace_folders().await.unwrap();

    let user_uri = Uri::from_file_path(ws.0.join("reporoot/examples/user.tcl")).unwrap();
    register(&backend, &user_uri, user_src).await;
    let locs = Box::pin(backend.compute_definition(
        &user_uri,
        Position {
            line: 2,
            character: 16,
        },
    ))
    .await
    .unwrap();
    assert_eq!(
        locs.len(),
        1,
        "the package's proc must resolve through the auto_path-seeded \
             package tier, got: {locs:?}",
    );
    assert!(
        locs[0].uri.as_str().ends_with("mypix.tcl"),
        "unexpected target: {:?}",
        locs[0].uri,
    );
}

/// The depth a real project writes: **three** nested `file dirname`s,
/// reaching two directories above the analysed file.  The other tests are
/// one level shallower, and the evaluator is recursive, so this is about
/// the layout rather than the arithmetic: a
/// deeply-nested `examples/subdir/user.tcl` whose package lives at the
/// repo root, with the workspace root scoped to the leaf directory.
///
/// Oracle (tclsh 9.0.4 and 8.6.16, this exact layout): running
/// `examples/demos/user.tcl` prints `loaded:foo.png`.
#[tokio::test]
async fn a_triple_nested_dirname_auto_path_feeds_the_package_database() {
    let ws = TmpWs::new("autopath-triple");
    ws.write(
        "reporoot/pkgIndex.tcl",
        "package ifneeded mypix 1.0 [list source [file join $dir mypix.tcl]]\n",
    );
    ws.write(
        "reporoot/mypix.tcl",
        "package provide mypix 1.0\nproc ::mypix::readImage {path} { return \"loaded:$path\" }\n",
    );
    ws.write(
        "reporoot/examples/demos/user.tcl",
        "lappend auto_path [file dirname [file dirname [file dirname [info script]]]]\n\
             package require mypix\n\
             puts [::mypix::readImage foo.png]\n",
    );

    let backend = test_backend();
    // Root is the leaf `demos/` directory — two levels below the package.
    let narrow_root = Uri::from_file_path(ws.0.join("reporoot/examples/demos")).unwrap();
    *backend.workspace_folders.lock().await = vec![narrow_root];
    backend.scan_workspace_folders().await.unwrap();
    assert!(
        backend.package_resolver.read().await.provides("mypix"),
        "a triple-nested `[file dirname …]` auto_path entry must reach the \
             repo-root package directory",
    );
}

#[test]
fn retained_source_and_package_paths_consume_actual_plans() {
    // naming.navigation.retained-path-source-inventory
    // docs/design/analysis/name-resolution-proofs/retained-path-source-inventory.md
    let source = "interp alias {} build {} file join /ROOT; source [build {literal$é.tcl}]";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    let target = analysis
        .source_targets
        .first()
        .expect("original source target");
    let expression =
        tcl_compiler::auto_path_eval::capture_source_target_path_expression(&analysis, target)
            .unwrap();
    let imported = tcl_compiler::auto_path_eval::FoldedPathConstants::default();
    let expected = Uri::from_file_path("/ROOT/literal$é.tcl").unwrap();
    assert_eq!(
        resolve_source_edge(
            "file:///ROOT/main.tcl",
            "reported-unrelated.tcl",
            Some(&expression),
            target.range.start(),
            &analysis.path_constant_assignments,
            &imported
        )
        .as_deref(),
        Some(expected.as_str())
    );
    assert!(
        resolve_source_edge(
            "file:///ROOT/main.tcl",
            &target.raw_path,
            None,
            target.range.start(),
            &analysis.path_constant_assignments,
            &imported
        )
        .is_none()
    );
    let other = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert!(
        resolve_source_edge(
            "file:///ROOT/main.tcl",
            &target.raw_path,
            Some(&expression),
            target.range.start(),
            &other.path_constant_assignments,
            &imported
        )
        .is_none()
    );
    let uri = Uri::from_file_path("/ROOT/main.tcl").unwrap();
    for (source, expected) in [
        (
            "set auto_path [file join /ROOT {a b}]",
            vec![PathBuf::from("/ROOT/a"), PathBuf::from("/ROOT/b")],
        ),
        (
            "lappend auto_path [file join /ROOT {a b}]",
            vec![PathBuf::from("/ROOT/a b")],
        ),
        (
            "interp alias {} build {} file join /ROOT; lappend auto_path [build {literal$é}]",
            vec![PathBuf::from("/ROOT/literal$é")],
        ),
    ] {
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(
            document_auto_path_dirs(&uri, &analysis),
            expected,
            "{source}"
        );
    }
    let source = "proc file {args} {}; lappend auto_path [file join /ROOT leaf]";
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "tcl8.6");
    assert_ne!(analysis.auto_path_entries.len(), 0);
    assert_eq!(
        document_auto_path_dirs(&uri, &analysis),
        [] as [std::path::PathBuf; 0]
    );
}

/// A *relative* `auto_path` entry
/// (`lappend auto_path lib`) resolves against the interpreter's working
/// directory at run time, which no static analysis knows — so it must be
/// anchored on the analysed document's own directory, never on the
/// language server process's launch directory (an editor artefact).
///
/// Oracle (tclsh 8.6.16 / 9.0.4): `cd reporoot && tclsh main.tcl` loads
/// `reporoot/lib`'s package; the same file run from elsewhere would not.
/// The document's directory is the closest base an editor has, and — as
/// this test proves by construction — it is a base the *server's* own
/// working directory is not.
#[tokio::test]
async fn a_relative_auto_path_entry_anchors_on_the_document_not_the_server_cwd() {
    let ws = TmpWs::new("autopath-relative");
    ws.write(
        "reporoot/lib/pkgIndex.tcl",
        "package ifneeded relpix 1.0 [list source [file join $dir relpix.tcl]]\n",
    );
    ws.write(
        "reporoot/lib/relpix.tcl",
        "package provide relpix 1.0\nproc ::relpix::go {} { return ok }\n",
    );
    ws.write(
        "reporoot/main.tcl",
        "lappend auto_path lib\npackage require relpix\nputs [::relpix::go]\n",
    );

    let main_uri = Uri::from_file_path(ws.0.join("reporoot/main.tcl")).unwrap();
    let main_src = std::fs::read_to_string(ws.0.join("reporoot/main.tcl")).expect("read");
    let analysis = tcl_compiler::analyser::Analyser::new()
        .analyse(&main_src, "tcl8.6")
        .clone();
    assert_eq!(
        document_auto_path_dirs(&main_uri, &analysis),
        vec![ws.0.join("reporoot/lib")],
        "`lappend auto_path lib` must resolve against the document's own \
             directory; the server process's working directory is not a fact \
             about the user's program",
    );

    // And it really does make the package resolvable, with the workspace
    // root scoped to the document's directory only.
    let backend = test_backend();
    let root = Uri::from_file_path(ws.0.join("reporoot")).unwrap();
    *backend.workspace_folders.lock().await = vec![root];
    backend.scan_workspace_folders().await.unwrap();
    assert!(
        backend.package_resolver.read().await.provides("relpix"),
        "the relative auto_path entry must put `reporoot/lib` on the search path",
    );
}

/// TP: `set auto_path` assigns a **list**, so every
/// element becomes a search directory; TN: a brace-quoted element holding
/// a space stays exactly one directory.
///
/// Oracle (`tclsh8.6` 8.6.14 / `tclsh9.0` 9.0.4) on this layout: with
/// `set auto_path {…/p1 …/p2 {…/with space}}`, `llength $auto_path` is 3
/// and `package require` finds `alpha`, `beta` **and** `spaced`.
#[tokio::test]
async fn set_auto_path_puts_every_list_element_on_the_search_path() {
    let ws = TmpWs::new("autopath-set");
    for (dir, pkg) in [("p1", "alpha"), ("p2", "beta"), ("with space", "spaced")] {
        ws.write(
            &format!("reporoot/{dir}/pkgIndex.tcl"),
            &format!("package ifneeded {pkg} 1.0 [list source [file join $dir {pkg}.tcl]]\n"),
        );
        ws.write(
            &format!("reporoot/{dir}/{pkg}.tcl"),
            &format!("package provide {pkg} 1.0\nproc ::{pkg}::go {{}} {{}}\n"),
        );
    }
    let root = ws.0.join("reporoot");
    let root_s = root.to_str().unwrap();
    ws.write(
        "reporoot/examples/user.tcl",
        &format!(
            "set auto_path {{{root_s}/p1 {root_s}/p2 {{{root_s}/with space}}}}\n\
                 package require alpha\npackage require beta\npackage require spaced\n"
        ),
    );

    let backend = test_backend();
    // Root is `examples/` only, so nothing but the assignment can reach
    // the three package directories.
    let narrow_root = Uri::from_file_path(ws.0.join("reporoot/examples")).unwrap();
    *backend.workspace_folders.lock().await = vec![narrow_root];
    backend.scan_workspace_folders().await.unwrap();

    let resolver = backend.package_resolver.read().await;
    for pkg in ["alpha", "beta"] {
        assert!(
            resolver.provides(pkg),
            "`set auto_path` names {pkg}'s directory as one list element, \
                 so it must be searched — the whole right-hand side is not one \
                 space-containing path",
        );
    }
    assert!(
        resolver.provides("spaced"),
        "a brace-quoted element is ONE directory, spaces and all, and must \
             still be searched",
    );
}

/// A `PackageResolver` mutation must move its revision, and an unmutated
/// resolver must keep it — the signal the recovery memo keys on for the
/// package half of the widened set.
#[test]
fn package_resolver_revision_tracks_mutations() {
    let mut resolver = PackageResolver::new();
    let start = resolver.revision();
    assert_eq!(
        start,
        resolver.revision(),
        "a read must not move the revision"
    );
    resolver.add_tcl_index(Vec::new());
    assert_ne!(
        start,
        resolver.revision(),
        "recording an index must move the revision",
    );
    assert_ne!(
        PackageResolver::new().revision(),
        PackageResolver::new().revision(),
        "two distinct resolvers must never share a revision",
    );
}
