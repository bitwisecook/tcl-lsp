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

//! `tclvm` initialises the selected script library as `Tcl_Main` does: with
//! `TCL_LIBRARY` set it sources that library's `init.tcl`, so `package
//! require` finds the packages the library bundles through their
//! `pkgIndex.tcl`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tcl_dialect::TclVersion;

const CAUGHT_REQUIRE: &str = "puts [catch {package require msgcat 1.6} m]; puts \"<$m>\"; \
                              puts \"<[package provide msgcat]>\"\n";
const ARGV_REQUIRE: &str = "if {[llength $argv]} { package require msgcat 1.6 }\n\
                            puts \"after <[package provide msgcat]>\"\n";

struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tclvm-script-library-{}-{test}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("creates the scratch directory");
        Self(dir)
    }

    fn write(&self, name: &str, source: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().expect("a parent")).expect("creates the parent");
        fs::write(&path, source).expect("writes the file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tclvm(library: Option<&Path>, script: &Path, argv: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tclvm"));
    match library {
        Some(library) => command.env("TCL_LIBRARY", library),
        None => command.env_remove("TCL_LIBRARY"),
    };
    command.arg(script).args(argv).output().expect("run tclvm")
}

fn outcome(output: &Output) -> (String, String, Option<i32>) {
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code(),
    )
}

/// A library whose `init.tcl` sets `auto_path` and a `package unknown` that
/// reads each `pkgIndex.tcl` beneath it — the shape of the real one, small
/// enough that nothing but this mechanism can supply the package.
fn fixture_library(scratch: &Scratch) -> PathBuf {
    scratch.write(
        "lib/tcl/init.tcl",
        "set ::auto_path [list $::tcl_library [file dirname $::tcl_library]]\n\
         proc ::fixtureUnknown {name args} {\n\
             foreach root $::auto_path {\n\
                 foreach index [glob -nocomplain -directory $root -join * pkgIndex.tcl] {\n\
                     set dir [file dirname $index]\n\
                     source $index\n\
                 }\n\
             }\n\
         }\n\
         package unknown ::fixtureUnknown\n",
    );
    scratch.write(
        "lib/tcl/msgcat/pkgIndex.tcl",
        "package ifneeded msgcat 1.6.1 [list source [file join $dir msgcat.tcl]]\n",
    );
    scratch.write(
        "lib/tcl/msgcat/msgcat.tcl",
        "package provide msgcat 1.6.1\n",
    );
    scratch.0.join("lib/tcl")
}

#[test]
fn a_bundled_package_is_found_through_the_library_init() {
    let scratch = Scratch::new("fixture");
    let library = fixture_library(&scratch);
    let caught = scratch.write("caught.tcl", CAUGHT_REQUIRE);
    let argv = scratch.write("argv.tcl", ARGV_REQUIRE);

    assert_eq!(
        outcome(&tclvm(Some(&library), &caught, &[])),
        ("0\n<1.6.1>\n<1.6.1>\n".into(), String::new(), Some(0))
    );
    assert_eq!(
        outcome(&tclvm(Some(&library), &argv, &["x"])),
        ("after <1.6.1>\n".into(), String::new(), Some(0))
    );
}

#[test]
fn without_a_library_the_failing_require_is_caught_and_reported() {
    let scratch = Scratch::new("none");
    let caught = scratch.write("caught.tcl", CAUGHT_REQUIRE);
    let argv = scratch.write("argv.tcl", ARGV_REQUIRE);

    assert_eq!(
        outcome(&tclvm(None, &caught, &[])),
        (
            "1\n<can't find package msgcat 1.6>\n<>\n".into(),
            String::new(),
            Some(0)
        )
    );
    let (stdout, stderr, code) = outcome(&tclvm(None, &argv, &["x"]));
    assert_eq!((stdout.as_str(), code), ("", Some(1)));
    assert!(
        stderr.starts_with("can't find package msgcat 1.6\n    while executing\n"),
        "{stderr}"
    );
}

#[test]
fn an_unusable_library_is_reported_as_tcl_main_reports_it() {
    let scratch = Scratch::new("broken");
    scratch.write("lib/init.tcl", "error {init boom}\n");
    let script = scratch.write("script.tcl", "puts ran\n");

    assert_eq!(
        outcome(&tclvm(Some(&scratch.0.join("lib")), &script, &[])),
        (
            "ran\n".into(),
            "application-specific initialization failed: init boom\n".into(),
            Some(0)
        )
    );
}

/// The pinned Tcl 9.0.4 library — the one the development environment
/// exports as `TCL_LIBRARY` — gives what `tclsh9.1` prints for both scripts.
#[test]
fn the_pinned_tcl_9_library_provides_its_msgcat() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Some(tree) = tcl_test_support::locate_source_tree(&repo_root, TclVersion::V9_0, None)
        .expect("Tcl 9.0 source tree discovery")
    else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    let library = tree.library_dir();
    let scratch = Scratch::new("pinned");
    let caught = scratch.write("caught.tcl", CAUGHT_REQUIRE);
    let argv = scratch.write("argv.tcl", ARGV_REQUIRE);

    assert_eq!(
        outcome(&tclvm(Some(&library), &caught, &[])),
        ("0\n<1.7.1>\n<1.7.1>\n".into(), String::new(), Some(0))
    );
    assert_eq!(
        outcome(&tclvm(Some(&library), &argv, &["x"])),
        ("after <1.7.1>\n".into(), String::new(), Some(0))
    );
}

/// msgcat's `namespace eval` body builds its tables with `dict create {*}{…}`.
/// A compiled body whose command has no native recipe for the expansion runs
/// that command by plain dispatch, as C invokes a command its compiler
/// declines, rather than refusing the script: each line is what tclsh 8.6.18,
/// 9.0.4 and 9.1.0 print.
#[test]
fn an_expanded_word_in_a_compiled_body_runs_as_tclsh_runs_it() {
    let scratch = Scratch::new("expanded");
    let script = scratch.write(
        "expanded.tcl",
        "set x {a b}\n\
         puts [if 1 { list {*}{a b} }]\n\
         puts [if 1 { dict create {*}$x }]\n\
         if 1 { puts [dict create {*}{a b}] }\n\
         proc p {} { if 1 { set v {}; append v {*}{a b} } }\n\
         puts [p]\n\
         namespace eval m { variable W [dict create {*}{c d}] }\n\
         puts $m::W\n\
         puts [catch { if 1 { dict create {*}{a} } } m]$m\n",
    );
    for release in ["8.6", "9.0", "9.1"] {
        let output = Command::new(env!("CARGO_BIN_EXE_tclvm"))
            .env_remove("TCL_LIBRARY")
            .args(["--tcl-version", release])
            .arg(&script)
            .output()
            .expect("run tclvm");
        assert_eq!(
            outcome(&output),
            (
                "a b\na b\na b\nab\nc d\n1wrong # args: should be \"dict create ?key value ...?\"\n"
                    .into(),
                String::new(),
                Some(0)
            ),
            "{release}"
        );
    }
}
