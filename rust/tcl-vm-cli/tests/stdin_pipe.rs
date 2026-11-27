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

//! `stdin` fed through a pipe, and read-only command pipelines, read as
//! `tclsh` reads them. Each case runs under `tclvm` for every release and
//! against the pinned expectation, and under each reference `tclsh` found
//! ([`tcl_test_support::witness_tclsh`]) as a live differential.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use tcl_dialect::TclVersion;

/// Run `program` with `args`, `input` piped to its stdin, in `dir`; its
/// stdout and stderr with every `CHILDSTATUS` / `CHILDKILLED` pid written
/// `PID`. An inherited `TCL_LIBRARY` names one release's library, which every
/// other release fails to initialise from, so each interpreter finds its own.
fn run(program: &Path, args: &[&str], dir: &Path, input: &[u8]) -> (String, String) {
    let mut child = Command::new(program)
        .env_remove("TCL_LIBRARY")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the interpreter");
    let mut stdin = child.stdin.take().expect("piped stdin");
    stdin.write_all(input).expect("write stdin");
    drop(stdin);
    let output = child.wait_with_output().expect("the interpreter exits");
    let mask = |bytes: &[u8]| {
        let text = String::from_utf8_lossy(bytes);
        let mut masked = text.into_owned();
        for tag in ["CHILDSTATUS ", "CHILDKILLED "] {
            let mut out = String::new();
            let mut rest = masked.as_str();
            while let Some(at) = rest.find(tag) {
                let (head, tail) = rest.split_at(at + tag.len());
                out.push_str(head);
                out.push_str("PID");
                rest = tail.trim_start_matches(|c: char| c.is_ascii_digit());
            }
            out.push_str(rest);
            masked = out;
        }
        masked
    };
    (mask(&output.stdout), mask(&output.stderr))
}

/// Check `script`, given as a file (`as_file`) or as stdin itself, with
/// `input` on stdin after it, under `tclvm` and every reference `tclsh`.
fn check(name: &str, script: &str, as_file: bool, input: &[u8], expected: &str) {
    let dir = std::env::temp_dir().join(format!("tclvm-stdin-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create the script directory");
    std::fs::write(dir.join("script.tcl"), script).expect("write the script");
    let stdin = if as_file {
        input.to_vec()
    } else {
        [script.as_bytes(), input].concat()
    };
    let file_arg: &[&str] = if as_file { &["script.tcl"] } else { &[] };
    for version in TclVersion::ALL {
        let release = version.version_string();
        let args = [&["--tcl-version", release][..], file_arg].concat();
        let vm = run(Path::new(env!("CARGO_BIN_EXE_tclvm")), &args, &dir, &stdin);
        assert_eq!(
            vm,
            (expected.to_owned(), String::new()),
            "tclvm {release}: {name}"
        );
        if let Some(tclsh) = tcl_test_support::witness_tclsh(version) {
            let reference = run(&tclsh.path, file_arg, &dir, &stdin);
            assert_eq!(vm, reference, "tclvm against tclsh{release}: {name}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn gets_reads_a_piped_line_into_a_variable() {
    check(
        "gets-var",
        "puts [gets stdin line]; puts $line\n",
        true,
        b"hello\n",
        "5\nhello\n",
    );
}

#[test]
fn eof_is_set_by_the_read_that_meets_the_end() {
    check(
        "gets-eof",
        "puts [eof stdin]\n\
         puts [gets stdin]/[eof stdin]\n\
         puts [gets stdin]/[eof stdin]\n\
         puts [gets stdin x]/$x/[eof stdin]\n",
        true,
        b"a\r\nb",
        "0\na/0\nb/1\n-1//1\n",
    );
}

#[test]
fn read_takes_a_count_then_the_rest() {
    check(
        "read",
        "puts [read stdin 3]/[eof stdin]\n\
         puts [read -nonewline stdin]/[eof stdin]\n\
         puts [read stdin]/[eof stdin]\n",
        true,
        b"abcdef\nghi\n",
        "abc/0\ndef\nghi/1\n/1\n",
    );
}

#[test]
fn a_script_read_from_stdin_reads_the_lines_after_its_command() {
    check(
        "script-on-stdin",
        "gets stdin x\n",
        false,
        b"hello there\nputs <$x>\n",
        "<hello there>\n",
    );
}

#[test]
fn a_read_only_pipeline_reads_its_command_output() {
    check(
        "pipeline",
        "set f [open {|printf {a\\nb\\n}}]\n\
         while {[gets $f line] >= 0} { puts $line/[eof $f] }\n\
         puts [eof $f]; close $f\n\
         set f [open {|printf abc}]\n\
         puts [read $f 2]/[eof $f]/[read $f]/[eof $f]; close $f\n\
         puts [catch {close [open {|sh -c {exit 3}}]} m]/$m/$::errorCode\n\
         puts [catch {close [open {|sh -c {echo oops >&2}}]} m]/$m\n\
         puts [catch {close [open {|sh -c {kill -KILL $$}}]} m]/$m/$::errorCode\n\
         puts [catch {open {|tclvm_no_such_command}} m]/$m\n",
        true,
        b"",
        "a/0\nb/0\n1\nab/0/c/1\n\
         1/child process exited abnormally/CHILDSTATUS PID 3\n\
         1/oops\n\
         1/child killed: kill signal/CHILDKILLED PID SIGKILL {kill signal}\n\
         1/couldn't execute \"tclvm_no_such_command\": no such file or directory\n",
    );
}
