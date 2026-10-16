// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! An error that ends a script file, as `tclsh` reports it: the whole
//! `errorInfo` trace on stderr, ending with where in the file it occurred, and
//! exit status 1. The expectations are tclsh 8.4.20 through 9.1.0's, the
//! script's path written `PATH`.

use std::path::{Path, PathBuf};
use std::process::Command;

const RELEASES: [&str; 5] = ["8.4", "8.5", "8.6", "9.0", "9.1"];

/// A directory of its own for one test's scripts.
fn scratch(test: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("tclvm-script-errors-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creates the scratch directory");
    dir
}

/// Run the script at `path` under `release`: its stdout, its stderr with the
/// path written `PATH`, and whether it exited 1.
fn run(path: &Path, release: &str) -> (String, String, bool) {
    let output = Command::new(env!("CARGO_BIN_EXE_tclvm"))
        .args(["--tcl-version", release])
        .arg(path)
        .env("LC_ALL", "C.UTF-8")
        .output()
        .expect("run tclvm");
    let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    let path = path.to_str().expect("a UTF-8 path");
    (
        text(&output.stdout),
        text(&output.stderr).replace(path, "PATH"),
        output.status.code() == Some(1),
    )
}

fn write(dir: &Path, name: &str, source: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, source).expect("writes the script");
    path
}

#[test]
fn an_uncaught_error_reports_its_trace_and_where_in_the_file() {
    let dir = scratch("trace");
    let cases = [
        (
            "proc.tcl",
            "set x 1\n\nproc p {} {\n  error boom\n}\np\n",
            "",
            "boom\n    while executing\n\"error boom\"\n    (procedure \"p\" line 2)\n    \
             invoked from within\n\"p\"\n    (file \"PATH\" line 6)\n",
        ),
        (
            "break.tcl",
            "puts a\nbreak\n",
            "a\n",
            "invoked \"break\" outside of a loop\n    while executing\n\"break\"\n    \
             (file \"PATH\" line 2)\n",
        ),
        (
            "if.tcl",
            "if {1} {\n  break\n}\n",
            "",
            "invoked \"break\" outside of a loop\n    while executing\n\"if {1} {\n  break\n}\"\n    \
             (file \"PATH\" line 1)\n",
        ),
        (
            "return.tcl",
            "puts a\nreturn -code error boom\n",
            "a\n",
            "boom\n    while executing\n\"return -code error boom\"\n    (file \"PATH\" line 2)\n",
        ),
        (
            "code.tcl",
            "proc p {} {return -code 7 x}\np\n",
            "",
            "command returned bad code: 7\n    while executing\n\"p\"\n    (file \"PATH\" line 2)\n",
        ),
    ];
    for (name, source, stdout, stderr) in cases {
        let path = write(&dir, name, source);
        for release in RELEASES {
            assert_eq!(
                run(&path, release),
                (stdout.to_owned(), stderr.to_owned(), true),
                "{release}: {source}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_sourced_script_adds_its_own_file_frame() {
    let dir = scratch("source");
    let inner = write(&dir, "inner.tcl", "proc q {} {error boom}\nq\n");
    let breaks = write(&dir, "breaks.tcl", "puts a\nbreak\n");
    let outer = write(
        &dir,
        "outer.tcl",
        &format!(
            "catch {{source {}}}\nputs $errorInfo\nset f {}\nsource $f\n",
            inner.display(),
            breaks.display()
        ),
    );
    let inner_path = inner.to_str().expect("a UTF-8 path");
    for release in RELEASES {
        let (stdout, stderr, failed) = run(&outer, release);
        assert_eq!(
            stdout,
            format!(
                "boom\n    while executing\n\"error boom\"\n    (procedure \"q\" line 1)\n    \
                 invoked from within\n\"q\"\n    (file \"{inner_path}\" line 2)\n    \
                 invoked from within\n\"source {inner_path}\"\na\n"
            ),
            "{release}"
        );
        assert_eq!(
            stderr,
            "invoked \"break\" outside of a loop\n    while executing\n\"source $f\"\n    \
             (file \"PATH\" line 4)\n",
            "{release}"
        );
        assert!(failed, "{release}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_long_path_is_cut_as_each_release_cuts_it() {
    let dir = scratch("long");
    let nested = dir.join("d".repeat(80)).join("e".repeat(80));
    std::fs::create_dir_all(&nested).expect("creates the nested directory");
    let path = write(&nested, "long.tcl", "error boom\n");
    let full = path.to_str().expect("a UTF-8 path");
    let cut = &full[..150];
    for release in RELEASES {
        let output = Command::new(env!("CARGO_BIN_EXE_tclvm"))
            .args(["--tcl-version", release])
            .arg(&path)
            .output()
            .expect("run tclvm");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let ellipsis = if release == "8.4" { "" } else { "..." };
        assert!(
            stderr.ends_with(&format!("    (file \"{cut}{ellipsis}\" line 1)\n")),
            "{release}: {stderr}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_command_that_does_not_parse_reports_where_in_the_file_it_is() {
    // 8.4 and 8.5 quote the rest of the script from the malformed command on.
    let dir = scratch("parse");
    let cases = [
        (
            "first.tcl",
            "set x {\n",
            "",
            "missing close-brace\n    while executing\n\"set x {\"\n    (file \"PATH\" line 1)\n",
        ),
        (
            "later.tcl",
            "puts a\n\nset x \"abc\n",
            "a\n",
            "missing \"\n    while executing\n\"set x \"\"\n    (file \"PATH\" line 3)\n",
        ),
    ];
    for (name, source, stdout, stderr) in cases {
        let path = write(&dir, name, source);
        for release in ["8.6", "9.0", "9.1"] {
            assert_eq!(
                run(&path, release),
                (stdout.to_owned(), stderr.to_owned(), true),
                "{release}: {source}"
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The stderr `tclsh` reports for a script on each of some releases.
type Expectation = (&'static [&'static str], &'static str);

/// Scripts whose error passes through nested commands, each with what every
/// release reports: tclsh 8.4.20 through 9.1.0's, the path written `PATH`.
const NESTED_CASES: [(&str, &str, &[Expectation]); 9] = [
    (
        "expr.tcl",
        "proc p {} {error boom}\nset w [expr {[p]}]\n",
        &[
            (
                &["8.4"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 (file \"PATH\" line 1)\n",
            ),
            (
                &["8.5", "8.6", "9.0", "9.1"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 invoked from within\n\"expr {[p]}\"\n    \
                 invoked from within\n\"set w [expr {[p]}]\"\n    \
                 (file \"PATH\" line 2)\n",
            ),
        ],
    ),
    (
        "nested.tcl",
        "proc p {} {error boom}\nset w [list y [expr {[p]}]]\n",
        &[
            (
                &["8.4"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 (file \"PATH\" line 1)\n",
            ),
            (
                &["8.5", "8.6", "9.0", "9.1"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 invoked from within\n\"expr {[p]}\"\n    \
                 invoked from within\n\"list y [expr {[p]}]\"\n    \
                 invoked from within\n\"set w [list y [expr {[p]}]]\"\n    \
                 (file \"PATH\" line 2)\n",
            ),
        ],
    ),
    (
        "if.tcl",
        "proc p {} {error boom}\nif {[p]} {}\n",
        &[
            (
                &["8.4"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 (file \"PATH\" line 1)\n",
            ),
            (
                &["8.5", "8.6", "9.0", "9.1"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 invoked from within\n\"if {[p]} {}\"\n    (file \"PATH\" line 2)\n",
            ),
        ],
    ),
    (
        "while.tcl",
        "proc p {} {error boom}\nwhile {[p]} {}\n",
        &[
            (
                &["8.4"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 (file \"PATH\" line 1)\n",
            ),
            (
                &["8.5", "8.6", "9.0", "9.1"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 invoked from within\n\"while {[p]} {}\"\n    (file \"PATH\" line 2)\n",
            ),
        ],
    ),
    (
        "eval.tcl",
        "proc p {} {error boom}\neval {set w [list y [p]]}\n",
        &[
            (
                &["8.4", "8.5"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 invoked from within\n\"list y [p]\"\n    \
                 invoked from within\n\"set w [list y [p]]\"\n    \
                 (\"eval\" body line 1)\n    \
                 invoked from within\n\"eval {set w [list y [p]]}\"\n    \
                 (file \"PATH\" line 2)\n",
            ),
            (
                &["8.6", "9.0", "9.1"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 (\"eval\" body line 1)\n    \
                 invoked from within\n\"eval {set w [list y [p]]}\"\n    \
                 (file \"PATH\" line 2)\n",
            ),
        ],
    ),
    (
        "proc.tcl",
        "proc p {} {error boom}\nproc q {} {eval {set w [expr {[p]}]}}\nq\n",
        &[
            (
                &["8.4"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 (\"eval\" body line 1)\n    (procedure \"q\" line 1)\n    \
                 invoked from within\n\"q\"\n    (file \"PATH\" line 3)\n",
            ),
            (
                &["8.5"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 invoked from within\n\"expr {[p]}\"\n    \
                 invoked from within\n\"set w [expr {[p]}]\"\n    \
                 (\"eval\" body line 1)\n    \
                 invoked from within\n\"eval {set w [expr {[p]}]}\"\n    \
                 (procedure \"q\" line 1)\n    invoked from within\n\"q\"\n    \
                 (file \"PATH\" line 3)\n",
            ),
            (
                &["8.6", "9.0", "9.1"],
                "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 (\"eval\" body line 1)\n    \
                 invoked from within\n\"eval {set w [expr {[p]}]}\"\n    \
                 (procedure \"q\" line 1)\n    invoked from within\n\"q\"\n    \
                 (file \"PATH\" line 3)\n",
            ),
        ],
    ),
    (
        "lines.tcl",
        "proc p {} {error boom}\nproc s {} {\n  set y [list a [set z 1\np]]\n}\ns\n",
        &[(
            &["8.4", "8.5", "8.6", "9.0", "9.1"],
            "boom\n    while executing\n\"error boom\"\n    \
                 (procedure \"p\" line 1)\n    invoked from within\n\"p\"\n    \
                 (procedure \"s\" line 3)\n    invoked from within\n\"s\"\n    \
                 (file \"PATH\" line 6)\n",
        )],
    ),
    (
        "fold.tcl",
        "set y [list y [expr {1/0}]]\n",
        &[(
            &["8.4", "8.5", "8.6", "9.0", "9.1"],
            "divide by zero\n    while executing\n\"expr {1/0}\"\n    \
                 invoked from within\n\"list y [expr {1/0}]\"\n    \
                 invoked from within\n\"set y [list y [expr {1/0}]]\"\n    \
                 (file \"PATH\" line 1)\n",
        )],
    ),
    (
        "foldif.tcl",
        "if {1/0} {}\n",
        &[(
            &["8.4", "8.5", "8.6", "9.0", "9.1"],
            "divide by zero\n    while executing\n\"if {1/0} {}\"\n    \
                 (file \"PATH\" line 1)\n",
        )],
    ),
];

/// A command substitution in an expression or a substituted script, and an
/// expression C folds: each command enclosing it logs as the release
/// evaluates that command. A statement the release evaluates directly logs
/// every level from 8.5; 8.4 keeps an expression's substitution error logged
/// up to a procedure boundary; 8.4 and 8.5 evaluate an `eval` body directly.
#[test]
fn a_nested_error_logs_each_level_the_release_evaluates() {
    let dir = scratch("nested");
    for (name, source, expectations) in NESTED_CASES {
        let path = write(&dir, name, source);
        for (releases, stderr) in expectations {
            for release in *releases {
                assert_eq!(
                    run(&path, release),
                    (String::new(), (*stderr).to_owned(), true),
                    "{release}: {source}"
                );
            }
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}
