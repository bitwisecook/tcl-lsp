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
//! # The command surface is the engine's, not this file's (issue #1589)
//!
//! The interpreter is built with [`Interp::new`], which runs
//! `builtins::install` and therefore registers **every** command module the
//! engine has — `if`/`while`/`for`/`foreach`/`switch` from `cmd_control`,
//! `catch`/`error`/`try` from `cmd_error`, and the rest. A differential sheet
//! written in ordinary Tcl runs through here unmodified; nothing has to be
//! rewritten `if`-free, which is the gap #1589 reported.
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

use std::io::Read;

use tcl_runtime::interp::{Code, Interp};

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
            // emulates (issue #1328). Without it the runtime keeps its 9.0
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
        eprintln!(
            "init error: {}",
            String::from_utf8_lossy(&interp.result_bytes())
        );
        return 1;
    }
    // Optionally pre-load tcltest and source the backend-constraint overlay so
    // tests the running backend cannot support are skipped. Loading tcltest
    // here makes the test file's own `package require tcltest` a no-op.
    let overlay = std::env::var("TCL_BACKEND_CONSTRAINTS").unwrap_or_default();
    if init && !overlay.is_empty() {
        let pre = format!(
            "package require tcltest\nnamespace import -force ::tcltest::*\nsource {overlay}\n"
        );
        if interp.eval_str(pre.as_bytes()) == Code::Error {
            eprintln!(
                "backend-constraint overlay error: {}",
                String::from_utf8_lossy(&interp.result_bytes())
            );
            return 1;
        }
    }
    let code = match &path {
        Some(p) => interp.eval_sourced(&src, p.as_bytes()),
        None => interp.eval_str(&src),
    };
    let result = interp.result_bytes();
    if code == Code::Error {
        eprintln!("error: {}", String::from_utf8_lossy(&result));
        return 1;
    }
    // Print the script's final result (if any), like an interactive evaluation
    // — unless `--quiet` asked for `tclsh script.tcl`'s actual non-interactive
    // contract instead (stdout carries only `puts` output).
    if !quiet && !result.is_empty() {
        println!("{}", String::from_utf8_lossy(&result));
    }
    0
}
