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

//! The script `tclvm` is given, and the commands piped to it, run as
//! `Tcl_Main` runs them: with no command active, so a completion leaving one
//! is settled as C's `TclEvalEx` settles it there, and an error is reported
//! as tclsh reports it.
//!
//! Every expectation is tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0's
//! stdout, stderr and exit status for the same script.

use std::io::Write as _;
use std::process::{Command, Stdio};

/// Run `script` as the file `script.tcl` (`stdin == false`) or piped to
/// stdin, under `version`; its stdout, stderr and exit status.
fn run(version: &str, script: &str, stdin: bool) -> (String, String, i32) {
    let dir = std::env::temp_dir().join(format!(
        "tclvm-top-level-{}-{}",
        std::process::id(),
        std::thread::current()
            .name()
            .unwrap_or("main")
            .replace("::", "-")
    ));
    std::fs::create_dir_all(&dir).expect("create the script directory");
    std::fs::write(dir.join("script.tcl"), script).expect("write the script");
    let mut command = Command::new(env!("CARGO_BIN_EXE_tclvm"));
    command.args(["--tcl-version", version]).current_dir(&dir);
    if !stdin {
        command.arg("script.tcl");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run tclvm");
    let mut input = child.stdin.take().expect("piped stdin");
    if stdin {
        input.write_all(script.as_bytes()).expect("write stdin");
    }
    drop(input);
    let output = child.wait_with_output().expect("tclvm exits");
    let _ = std::fs::remove_dir_all(&dir);
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

fn assert_releases(releases: &[&str], script: &str, stdin: bool, expected: (&str, &str, i32)) {
    for version in releases {
        let (stdout, stderr, status) = run(version, script, stdin);
        assert_eq!(
            (stdout.as_str(), stderr.as_str(), status),
            expected,
            "{version}"
        );
    }
}

/// An error leaving the script `tclsh` is given reports its errorInfo, with
/// the file frame, and exits 1.
#[test]
fn file_error_reports_its_error_info() {
    let script = r"proc p {} {
  error deep
}
puts a
p
puts b
";
    assert_releases(
        &["8.4", "8.5", "8.6", "9.0", "9.1"],
        script,
        false,
        (
            r"a
",
            r#"deep
    while executing
"error deep"
    (procedure "p" line 2)
    invoked from within
"p"
    (file "script.tcl" line 5)
"#,
            1,
        ),
    );
}

/// A `return -code error` reaching the file is settled at top level and
/// logs the `return` command.
#[test]
fn file_return_code_error_logs_the_return() {
    let script = r"puts a
return -code error -errorcode {X Y} boom
puts b
";
    assert_releases(
        &["8.4", "8.5", "8.6", "9.0", "9.1"],
        script,
        false,
        (
            r"a
",
            r#"boom
    while executing
"return -code error -errorcode {X Y} boom"
    (file "script.tcl" line 2)
"#,
            1,
        ),
    );
}

/// A `break` reaching the top level is `invoked "break" outside of a loop`.
#[test]
fn file_break_is_an_error() {
    let script = r"puts a
break
puts b
";
    assert_releases(
        &["8.4", "8.5", "8.6", "9.0", "9.1"],
        script,
        false,
        (
            r"a
",
            r#"invoked "break" outside of a loop
    while executing
"break"
    (file "script.tcl" line 2)
"#,
            1,
        ),
    );
}

/// A `continue` reaching the top level is an error too.
#[test]
fn file_continue_is_an_error() {
    let script = r"continue
";
    assert_releases(
        &["8.4", "8.5", "8.6", "9.0", "9.1"],
        script,
        false,
        (
            "",
            r#"invoked "continue" outside of a loop
    while executing
"continue"
    (file "script.tcl" line 1)
"#,
            1,
        ),
    );
}

/// A `return` with a level left after the top level is `command returned
/// bad code: 2`.
#[test]
fn file_return_level_two_is_a_bad_code() {
    let script = r"puts a
return -level 2 hi
puts b
";
    assert_releases(
        &["8.5", "8.6", "9.0", "9.1"],
        script,
        false,
        (
            r"a
",
            r#"command returned bad code: 2
    while executing
"return -level 2 hi"
    (file "script.tcl" line 2)
"#,
            1,
        ),
    );
}

/// A plain `return` ends the file without an error.
#[test]
fn file_return_ends_the_file() {
    let script = r"puts a
return val
puts b
";
    assert_releases(
        &["8.4", "8.5", "8.6", "9.0", "9.1"],
        script,
        false,
        (
            r"a
", "", 0,
        ),
    );
}

/// The script's own commands are evaluated directly, so each enclosing
/// substitution logs its command as the error leaves it.
#[test]
fn file_substitution_errors_log_each_enclosing_command() {
    let script = r"proc e {} {error x}
set y [string repeat [e] 2]
";
    assert_releases(
        &["8.4", "8.5", "8.6", "9.0", "9.1"],
        script,
        false,
        (
            "",
            r#"x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"
    invoked from within
"string repeat [e] 2"
    invoked from within
"set y [string repeat [e] 2]"
    (file "script.tcl" line 2)
"#,
            1,
        ),
    );
}

/// A substitution of several commands is logged by its statement too, on
/// the statement's line.
#[test]
fn file_multi_command_substitution_logs_the_statement() {
    let script = r"proc e {} {error x}
puts a

set y [set a 1; e]
";
    assert_releases(
        &["8.4", "8.5", "8.6", "9.0", "9.1"],
        script,
        false,
        (
            r"a
",
            r#"x
    while executing
"error x"
    (procedure "e" line 1)
    invoked from within
"e"
    invoked from within
"set y [set a 1; e]"
    (file "script.tcl" line 4)
"#,
            1,
        ),
    );
}

/// A coroutine a top-level statement creates completes as a top-level
/// command does on every resume (C's `auxNumLevels`): its `return` is
/// settled. One created inside another command, `eval` included, is not.
#[test]
fn coroutine_created_at_top_level_settles_as_top_level() {
    let script = r"proc body {} { yield; return -code return x }
proc mk {name} { coroutine $name body }
coroutine c1 body
mk c2
eval {coroutine c3 body}
proc res {name} { list [catch {$name} m o] $m [dict get $o -code] [dict get $o -level] }
puts [list [res c1] [res c2] [res c3]]
";
    assert_releases(
        &["8.6", "9.0", "9.1"],
        script,
        false,
        (
            r"{0 x 0 0} {2 x 0 1} {2 x 0 1}
",
            "",
            0,
        ),
    );
}

/// Piped stdin is read and evaluated a command at a time: an error prints
/// its message, publishes `errorInfo`/`errorCode` and evaluation goes on.
#[test]
fn stdin_is_read_a_command_at_a_time() {
    let script = r"puts a
break
puts b
return -code error x
puts c
error y
puts d
proc p {} {error z}
p
puts $errorInfo
puts $errorCode
";
    assert_releases(
        &["8.4", "8.5", "8.6", "9.0", "9.1"],
        script,
        true,
        (
            r#"a
b
c
d
z
    while executing
"error z"
    (procedure "p" line 1)
    invoked from within
"p"
NONE
"#,
            r#"invoked "break" outside of a loop
x
y
z
"#,
            0,
        ),
    );
}
