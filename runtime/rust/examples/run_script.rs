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
//!
//! This is the end-to-end "execute a simple script" path: parse → eval loop →
//! builtins (`set`/`expr`/`if`/`while`/`for`/`foreach`/`proc`/`puts`/…). `puts`
//! writes to stdout directly; the script's final result is printed last.

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
    // before evaluating, like a real `tclsh`.
    let init = args.get(1).map(String::as_str) == Some("--init");
    if init {
        args.remove(1);
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
    // Print the script's final result (if any), like an interactive evaluation.
    if !result.is_empty() {
        println!("{}", String::from_utf8_lossy(&result));
    }
    0
}
