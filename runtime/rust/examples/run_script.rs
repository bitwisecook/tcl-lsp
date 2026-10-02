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

//! Minimal script runner (dev tool) — evaluate a Tcl script through the runtime
//! interpreter and print its result.
//!
//! Usage: `cargo run --example run_script -- path/to/script.tcl`
//!        `echo 'puts [expr {2+2}]' | cargo run --example run_script`
//!        `cargo run --example run_script -- --tcl-version 8.6 script.tcl`
//!
//! This is the end-to-end "execute a simple script" path: parse → eval loop →
//! builtins (`set`/`expr`/`if`/`while`/`for`/`foreach`/`proc`/`puts`/…). `puts`
//! writes to stdout directly; the script's final result is printed last,
//! **unless** `--quiet` is given, which matches `tclsh script.tcl`'s actual
//! non-interactive contract (only `puts` reaches stdout; the last command's
//! result is silently discarded, not echoed). `--quiet` exists for tooling
//! that diffs this runner's stdout against `tclsh`/`tclvm` (see
//! `rust/tcl-fuzz`) — a raw stdout comparison would otherwise see a spurious
//! divergence on any script whose last command leaves a non-empty result.
//!
//! # The command surface is the engine's, not this file's
//!
//! The interpreter is built with [`Interp::new`], which runs
//! `builtins::install` and therefore registers **every** command module the
//! engine has — `if`/`while`/`for`/`foreach`/`switch` from `cmd_control`,
//! `catch`/`error`/`try` from `cmd_error`, and the rest. A differential sheet
//! written in ordinary Tcl runs through here unmodified; nothing has to be
//! rewritten `if`-free.
//!
//! Do not add a hand-rolled registration list to this example. A private
//! subset here silently narrows every campaign that uses this harness while
//! looking perfectly healthy, and `run_script` is the documented way to
//! exercise this engine (the fuzzer's taxonomy names it). The contract is
//! pinned by `tests/run_script_builtin_surface.rs`.
//!
//! One conditional gap remains, and it is a *build* property rather than a
//! harness one: `expr` and the `::tcl::mathfunc`/`::tcl::mathop` ensembles are
//! `have_tommath`-gated, so a build whose `build.rs` could not find the
//! libtommath source (it warns, it does not fail) has no numeric tower. Build
//! with `TCL_TOMMATH_DIR` set — `make runtime-rust-test` does — or expect
//! every expression to fail.

use std::io::{self, Read, Write};

use tcl_runtime::interp::{Code, Interp};
use tcl_runtime::{CompletionCode, ScriptCompletion};

/// The recursive tree-walking interpreter uses native stack per Tcl call level,
/// so honouring the 1000-deep `interp recursionlimit` (a *catchable* error)
/// needs more than the default 8 MiB main-thread stack — otherwise deep
/// recursion overflows the native stack and aborts before the limit fires.
/// `tclsh` likewise runs on a large stack. 512 MiB is virtual (only touched
/// pages are committed), comfortably covering the limit.
const EVAL_STACK_BYTES: usize = 512 * 1024 * 1024;

fn main() {
    let code = std::thread::Builder::new()
        .stack_size(EVAL_STACK_BYTES)
        .spawn(run)
        .expect("spawn eval thread")
        .join()
        .expect("eval thread panicked");
    std::process::exit(code);
}

/// Write one byte-valued result as a host line without interpreting it as
/// UTF-8. The process adapter owns the prefix/newline framing; the runtime API
/// owns the exact completion bytes.
fn write_result_line(output: &mut impl Write, prefix: &[u8], result: &[u8]) -> io::Result<()> {
    output.write_all(prefix)?;
    output.write_all(result)?;
    output.write_all(b"\n")
}

/// Apply the documented dev-runner echo/error policy to a byte completion.
fn report_completion(completion: &ScriptCompletion, quiet: bool) -> io::Result<i32> {
    report_completion_to(
        completion,
        quiet,
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    )
}

/// Writer-parameterised core of [`report_completion`], kept separate from the
/// process handles so the byte contract is testable without descriptor
/// redirection.
fn report_completion_to(
    completion: &ScriptCompletion,
    quiet: bool,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> io::Result<i32> {
    if completion.code == CompletionCode::Error {
        return write_result_line(stderr, b"error: ", &completion.result).map(|()| 1);
    }
    if !quiet && !completion.result.is_empty() {
        write_result_line(stdout, b"", &completion.result)?;
    }
    Ok(0)
}

/// Evaluate the script (or stdin) and return the process exit code. Runs on the
/// large-stack worker thread so the interp — an `Rc` handle, hence single-thread
/// — is created and used entirely here.
fn run() -> i32 {
    let mut args: Vec<String> = std::env::args().collect();
    // `--init` bootstraps the standard library (TCL_LIBRARY → source init.tcl)
    // before evaluating, like a real `tclsh`. `--quiet` suppresses the final
    // non-empty-result echo (see the module doc comment). Both are recognised
    // in either order, ahead of the path/stdin argument.
    let mut init = false;
    let mut quiet = false;
    let mut version = None;
    loop {
        match args.get(1).map(String::as_str) {
            Some("--init") => {
                init = true;
                args.remove(1);
            }
            Some("--quiet") => {
                quiet = true;
                args.remove(1);
            }
            // `--tcl-version X.Y` pins the Tcl release the interpreter
            // emulates. Without it the runtime keeps its 9.0
            // default. The differential fuzzer passes it so a `runtime-rust`
            // ↔ `tclsh` pair can be run *version-matched* against an 8.6
            // `tclsh`, instead of recording every deliberate 8.6-vs-9.0
            // semantic difference as a divergence.
            Some("--tcl-version") => {
                args.remove(1);
                let Some(raw) = args.get(1).cloned() else {
                    eprintln!("run_script: --tcl-version needs a value (e.g. 8.6)");
                    return 2;
                };
                args.remove(1);
                match tcl_dialect::TclVersion::from_package_version(&raw) {
                    Some(v) => version = Some(v),
                    None => {
                        eprintln!(
                            "run_script: unknown --tcl-version {raw:?} (want 8.4, 8.5, 8.6, 9.0 or 9.1)"
                        );
                        return 2;
                    }
                }
            }
            _ => break,
        }
    }
    // A file path is *sourced* (like `tclsh script.tcl`: `info script` is set and
    // `info frame` reports `type source`); stdin is evaluated as a script.
    let path = args.get(1).cloned();
    let src = if let Some(path) = &path {
        match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("run_script: cannot read {path}: {e}");
                return 2;
            }
        }
    } else {
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf).expect("read stdin");
        buf
    };

    let mut interp = Interp::new();
    // Before `init_library`, so the release the stdlib sees matches the one
    // the script will run under.
    if let Some(v) = version {
        interp.set_runtime_version(v);
    }
    if init && interp.init_library() == Code::Error {
        return write_result_line(
            &mut std::io::stderr().lock(),
            b"init error: ",
            &interp.result_bytes(),
        )
        .map_or_else(
            |error| {
                eprintln!("run_script: cannot write completion: {error}");
                2
            },
            |()| 1,
        );
    }
    // Optionally pre-load tcltest and source the backend-constraint overlay so
    // tests the running backend cannot support are skipped. Loading tcltest
    // here makes the test file's own `package require tcltest` a no-op.
    let overlay = std::env::var("TCL_BACKEND_CONSTRAINTS").unwrap_or_default();
    if init && !overlay.is_empty() {
        let pre = format!(
            "package require tcltest\nnamespace import -force ::tcltest::*\nsource {overlay}\n"
        );
        let completion = interp.eval_completion(pre.as_bytes());
        if completion.code == CompletionCode::Error {
            return write_result_line(
                &mut std::io::stderr().lock(),
                b"backend-constraint overlay error: ",
                &completion.result,
            )
            .map_or_else(
                |error| {
                    eprintln!("run_script: cannot write completion: {error}");
                    2
                },
                |()| 1,
            );
        }
    }
    let completion = match &path {
        Some(p) => interp.eval_sourced_completion(&src, p.as_bytes()),
        None => interp.eval_completion(&src),
    };
    // Print the script's final result (if any), like an interactive evaluation
    // — unless `--quiet` asked for `tclsh script.tcl`'s actual non-interactive
    // contract instead (stdout carries only `puts` output).
    report_completion(&completion, quiet).unwrap_or_else(|error| {
        eprintln!("run_script: cannot write completion: {error}");
        2
    })
}

#[cfg(test)]
mod tests {
    use super::{report_completion_to, CompletionCode, ScriptCompletion};

    #[test]
    fn result_and_error_lines_preserve_non_utf8_bytes() {
        const VALUE: &[u8] = &[0xff, 0x00, b'A', 0x80];

        let success = ScriptCompletion::new(CompletionCode::Ok, VALUE.to_vec(), Vec::new());
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            report_completion_to(&success, false, &mut stdout, &mut stderr).unwrap(),
            0
        );
        assert_eq!(stdout, [VALUE, b"\n"].concat());
        assert!(stderr.is_empty());

        let failure = ScriptCompletion::new(CompletionCode::Error, VALUE.to_vec(), Vec::new());
        stdout.clear();
        assert_eq!(
            report_completion_to(&failure, false, &mut stdout, &mut stderr).unwrap(),
            1
        );
        assert!(stdout.is_empty());
        assert_eq!(stderr, [b"error: ".as_slice(), VALUE, b"\n"].concat());
    }
}
