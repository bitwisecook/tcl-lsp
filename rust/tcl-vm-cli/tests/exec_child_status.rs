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

//! `exec` raises as `TclCleanupChildren` does: a non-zero exit is
//! `CHILDSTATUS pid status`, a signal kill `CHILDKILLED pid SIGNAME msg`, and
//! unredirected stderr output is an error unless `-ignorestderr` is given.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

use tcl_dialect::TclVersion;

/// Each vector prints `{code result errorCode}`, the pid normalised to `PID`
/// once it is checked to be a positive integer.
const SCRIPT: &str = r#"
proc t {args} {
    global errorCode
    set code [catch {eval exec $args} result]
    set ec -
    if {$code} {
        set ec $errorCode
        if {[string match CHILD* $ec]} {
            if {![regexp {^[1-9][0-9]*$} [lindex $ec 1]]} { error "bad pid in $ec" }
            set ec [lreplace $ec 1 1 PID]
        }
    }
    puts [list $code $result $ec]
}
t false
t sh -c {echo out; exit 3}
t sh -c {kill -KILL $$}
t sh -c {echo out; kill -TERM $$}
t sh -c {echo out; echo err >&2; exit 2}
t sh -c {echo out; echo err >&2}
t sh -c {echo err >&2; kill -INT $$}
t sh -c {printf 'a\n\n'}
t -keepnewline sh -c {printf 'a\n\n'}
t -- echo -x
t -bogus true
t -keepnewline
"#;

const SHARED: &str = r"1 {child process exited abnormally} {CHILDSTATUS PID 1}
1 {out
child process exited abnormally} {CHILDSTATUS PID 3}
1 {child killed: kill signal} {CHILDKILLED PID SIGKILL {kill signal}}
1 {out
child killed: software termination signal} {CHILDKILLED PID SIGTERM {software termination signal}}
1 {out
err} {CHILDSTATUS PID 2}
1 {out
err} NONE
1 err {CHILDKILLED PID SIGINT interrupt}
0 {a
} -
0 {a

} -
0 -x -
";

/// The switch-table vectors: the table, its noun and the usage text move with
/// the release. Measured on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0.
fn expected(version: TclVersion) -> String {
    let tail = match version {
        TclVersion::V8_4 => {
            r#"1 {bad switch "-bogus": must be -keepnewline or --} NONE
1 {wrong # args: should be "exec ?switches? arg ?arg ...?"} NONE
"#
        }
        TclVersion::V8_5 => {
            r#"1 {bad switch "-bogus": must be -ignorestderr, -keepnewline, or --} {TCL LOOKUP INDEX switch -bogus}
1 {wrong # args: should be "exec ?switches? arg ?arg ...?"} NONE
"#
        }
        TclVersion::V8_6 => {
            r#"1 {bad option "-bogus": must be -ignorestderr, -keepnewline, or --} {TCL LOOKUP INDEX option -bogus}
1 {wrong # args: should be "exec ?-option ...? arg ?arg ...?"} {TCL WRONGARGS}
"#
        }
        TclVersion::V9_0 | TclVersion::V9_1 => {
            r#"1 {bad option "-bogus": must be -ignorestderr, -keepnewline, -encoding, or --} {TCL LOOKUP INDEX option -bogus}
1 {wrong # args: should be "exec ?-option ...? arg ?arg ...?"} {TCL WRONGARGS}
"#
        }
    };
    format!("{SHARED}{tail}")
}

/// The reported reproducer, and `-ignorestderr` passing the child's stderr
/// through to the interpreter's own (Tcl 8.5+).
const IGNORESTDERR: &str = r"
puts [catch {exec false} m o]
puts $m
puts [lreplace [dict get $o -errorcode] 1 1 PID]
puts [list [catch {exec -ignorestderr sh -c {echo out; echo err >&2}} m] $m]
";

const IGNORESTDERR_STDOUT: &str = "1\nchild process exited abnormally\nCHILDSTATUS PID 1\n0 out\n";

struct Run {
    stdout: String,
    stderr: String,
}

fn run(command: &mut Command) -> Option<Run> {
    // Each run pins its own release, so an inherited library for one release
    // would make the others report an init failure on stderr.
    let output = command
        .env("LC_ALL", "C.UTF-8")
        .env_remove("TCL_LIBRARY")
        .output()
        .ok()?;
    Some(Run {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn tclvm(version: TclVersion, path: &Path) -> Run {
    run(Command::new(env!("CARGO_BIN_EXE_tclvm"))
        .args(["--tcl-version", version.version_string()])
        .arg(path))
    .expect("run tclvm")
}

fn scratch(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tclvm-exec-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creates the scratch directory");
    let path = dir.join(format!("{name}.tcl"));
    std::fs::write(&path, text).expect("writes the script");
    path
}

#[test]
fn exec_raises_on_abnormal_exit_signal_and_stderr_as_each_release_does() {
    let path = scratch("status", SCRIPT);
    for version in TclVersion::ALL {
        let release = version.version_string();
        assert_eq!(
            tclvm(version, &path).stdout,
            expected(version),
            "tclvm {release}"
        );
        if let Some(oracle) = tcl_test_support::witness_tclsh(version) {
            let tclsh = run(Command::new(&oracle.path).arg(&path)).expect("run tclsh");
            assert_eq!(tclsh.stdout, expected(version), "tclsh{release}");
        }
    }
    let _ = std::fs::remove_dir_all(path.parent().expect("scratch directory"));
}

#[test]
fn ignorestderr_passes_child_stderr_through() {
    let path = scratch("ignorestderr", IGNORESTDERR);
    for version in [
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let release = version.version_string();
        let vm = tclvm(version, &path);
        assert_eq!(vm.stdout, IGNORESTDERR_STDOUT, "tclvm {release}");
        assert_eq!(vm.stderr, "err\n", "tclvm {release}");
        if let Some(oracle) = tcl_test_support::witness_tclsh(version) {
            let tclsh = run(Command::new(&oracle.path).arg(&path)).expect("run tclsh");
            assert_eq!(tclsh.stdout, IGNORESTDERR_STDOUT, "tclsh{release}");
            assert_eq!(tclsh.stderr, "err\n", "tclsh{release}");
        }
    }
    let _ = std::fs::remove_dir_all(path.parent().expect("scratch directory"));
}

tcl_registry::testing::install_shipped!();
