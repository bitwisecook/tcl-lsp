// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `tclvm` initialises the library `TCL_LIBRARY` selects as `Tcl_Init` does:
//! it sources that library's `init.tcl`, so `auto_path` and the
//! `package unknown` handler exist and `package require` reaches each
//! `pkgIndex.tcl`.

use std::path::{Path, PathBuf};
use std::process::Command;

use tcl_dialect::TclVersion;
use tcl_test_support::locate_source_tree;

const CAUGHT: &str =
    "puts [catch {package require PKG 1.6} m]; puts \"<$m>\"; puts \"<[package provide PKG]>\"\n";
const GUARDED: &str =
    "if {[llength $argv]} { package require PKG 1.6 }\nputs \"after <[package provide PKG]>\"\n";

fn scratch(test: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("tclvm-library-init-{}-{test}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("creates the scratch directory");
    dir
}

fn write(path: &Path, source: &str) -> PathBuf {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("creates the directory");
    std::fs::write(path, source).expect("writes the file");
    path.to_path_buf()
}

/// Run `script` under `release` with `library` as `TCL_LIBRARY` (none when
/// `None`): stdout, stderr and exit status.
fn run(
    release: &str,
    library: Option<&Path>,
    script: &Path,
    argv: &[&str],
) -> (String, String, i32) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tclvm"));
    command
        .env_remove("TCL_LIBRARY")
        .env_remove("TCLLIBPATH")
        .env("HOME", "/home/fixture")
        .args(["--tcl-version", release])
        .arg(script)
        .args(argv);
    if let Some(library) = library {
        command.env("TCL_LIBRARY", library);
    }
    let output = command.output().expect("run tclvm");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// A 9.1-shaped library: `init.tcl` builds `auto_path` with
/// `tcl::InitAutoPath` and discovers packages through `pkgIndex.tcl`.
fn fixture_library(dir: &Path) -> PathBuf {
    let library = dir.join("lib").join("tcl9.1");
    write(
        &library.join("init.tcl"),
        "tcl::InitAutoPath\n\
         proc ::fixtureUnknown {name args} {\n\
             foreach root $::auto_path {\n\
                 set dir [file join $root $name]\n\
                 set index [file join $dir pkgIndex.tcl]\n\
                 if {[file exists $index]} { source $index }\n\
             }\n\
         }\n\
         package unknown ::fixtureUnknown\n",
    );
    write(
        &library.join("fixpkg").join("pkgIndex.tcl"),
        "package ifneeded fixpkg 1.6.1 [list package provide fixpkg 1.6.1]\n",
    );
    library
}

#[test]
fn a_require_reaches_the_selected_librarys_package_index() {
    let dir = scratch("fixture");
    let library = fixture_library(&dir);
    let caught = write(&dir.join("caught.tcl"), &CAUGHT.replace("PKG", "fixpkg"));
    let guarded = write(&dir.join("guarded.tcl"), &GUARDED.replace("PKG", "fixpkg"));
    assert_eq!(
        run("9.1", Some(&library), &caught, &[]),
        ("0\n<1.6.1>\n<1.6.1>\n".into(), String::new(), 0)
    );
    assert_eq!(
        run("9.1", Some(&library), &guarded, &["x"]),
        ("after <1.6.1>\n".into(), String::new(), 0)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Without a library the require fails, and the failure is an ordinary
/// error: `catch` takes it, and an uncaught one is reported with its trace.
#[test]
fn without_a_library_a_failed_require_is_an_ordinary_error() {
    let dir = scratch("none");
    let caught = write(&dir.join("caught.tcl"), &CAUGHT.replace("PKG", "fixpkg"));
    let guarded = write(&dir.join("guarded.tcl"), &GUARDED.replace("PKG", "fixpkg"));
    assert_eq!(
        run("9.1", None, &caught, &[]),
        (
            "1\n<can't find package fixpkg 1.6>\n<>\n".into(),
            String::new(),
            0
        )
    );
    let (stdout, stderr, status) = run("9.1", None, &guarded, &["x"]);
    assert_eq!((stdout.as_str(), status), ("", 1));
    assert!(
        stderr.starts_with(
            "can't find package fixpkg 1.6\n    while executing\n\"package require fixpkg 1.6 \"\n"
        ),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `tcl::InitAutoPath` as tclsh 9.1 answers it: `TCLLIBPATH` (tilde-expanded)
/// only when `auto_path` is unset, then the library, its parent and the
/// executable's `../lib`, each added once.
#[test]
fn init_auto_path_builds_auto_path_as_tcl_91_does() {
    let dir = scratch("autopath");
    let library = fixture_library(&dir);
    let script = write(
        &dir.join("autopath.tcl"),
        "puts [catch {tcl::InitAutoPath x} m]$m\n\
         unset auto_path\n\
         set env(TCLLIBPATH) {~/foo /a/b /a/b}\n\
         puts [tcl::InitAutoPath]\n\
         puts [tcl::InitAutoPath]\n\
         set auto_path /kept\n\
         puts [tcl::InitAutoPath]\n\
         interp create -safe s\n\
         s eval {unset -nocomplain auto_path}\n\
         puts [s eval {list [tcl::InitAutoPath] [info exists auto_path]}]\n",
    );
    let lib = library.to_str().expect("a UTF-8 path");
    let parent = library
        .parent()
        .and_then(Path::to_str)
        .expect("a UTF-8 path");
    let exe_lib = Path::new(env!("CARGO_BIN_EXE_tclvm"))
        .ancestors()
        .nth(2)
        .expect("the executable's grandparent")
        .join("lib");
    let exe_lib = exe_lib.to_str().expect("a UTF-8 path");
    let built = format!("/home/fixture/foo /a/b {lib} {parent} {exe_lib}");
    assert_eq!(
        run("9.1", Some(&library), &script, &[]),
        (
            format!(
                "1wrong # args: should be \"tcl::InitAutoPath \"\n{built}\n{built}\n/kept {lib} {parent} {exe_lib}\n{{}} 1\n"
            ),
            String::new(),
            0
        )
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The real `msgcat` each release's own library ships, required the way the
/// release's tclsh requires it.
#[test]
fn msgcat_requires_from_each_releases_own_library() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = scratch("msgcat");
    let caught = write(&dir.join("caught.tcl"), &CAUGHT.replace("PKG", "msgcat"));
    let guarded = write(&dir.join("guarded.tcl"), &GUARDED.replace("PKG", "msgcat"));
    for (version, release, msgcat) in [
        (TclVersion::V8_6, "8.6", "1.6.1"),
        (TclVersion::V9_0, "9.0", "1.7.1"),
        (TclVersion::V9_1, "9.1", "1.7.1"),
    ] {
        let Some(tree) = locate_source_tree(&repo_root, version, None).expect("source tree lookup")
        else {
            eprintln!("skipping {release}: no Tcl {release} source tree available");
            continue;
        };
        let library = tree.library_dir();
        assert_eq!(
            run(release, Some(&library), &caught, &[]),
            (format!("0\n<{msgcat}>\n<{msgcat}>\n"), String::new(), 0),
            "{release}"
        );
        assert_eq!(
            run(release, Some(&library), &guarded, &["x"]),
            (format!("after <{msgcat}>\n"), String::new(), 0),
            "{release}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A command whose word expands is invoked as tclsh invokes it inside a
/// compiled body, rather than refusing the whole file: msgcat's `.dll`-only
/// `dict create {*}{…}` and its `namespace inscope … {*}$arglist` both
/// compile this way before any require can reach them.
#[test]
fn an_expanding_word_inside_a_compiled_body_runs_as_tclsh_runs_it() {
    let dir = scratch("expand");
    let cases = [
        (
            "literal.tcl",
            "set v 0\n\
             puts [catch {if {$v} { set x [dict create {*}{a b c}] }} m]<$m>\n\
             set v 1\n\
             puts [catch {if {$v} { set x [dict create {*}{a b c}] }} m]<$m>\n\
             while {$v} { puts [list {*}\"x y\"]; set v 0 }\n",
            "0<>\n1<wrong # args: should be \"dict create ?key value ...?\">\nx y\n",
        ),
        (
            "ensemble.tcl",
            "proc p {ns cmd a} { catch { set r [namespace inscope $ns $cmd {*}$a] }; return $r }\n\
             puts [p :: list {x y}]\n\
             proc q {a} { foreach _ {1} { set r [namespace eval :: list {*}$a] }; return $r }\n\
             puts [q {x y}]\n",
            "x y\nx y\n",
        ),
    ];
    for release in ["8.6", "9.0", "9.1"] {
        for (name, source, expected) in cases {
            let script = write(&dir.join(name), source);
            assert_eq!(
                run(release, None, &script, &[]),
                (expected.to_owned(), String::new(), 0),
                "{release} {name}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
