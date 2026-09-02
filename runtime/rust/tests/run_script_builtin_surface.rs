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

//! **Issue #1589, second half** — `examples/run_script` must present the
//! engine's *full* builtin set, so a differential sheet written in plain Tcl
//! runs through it unmodified.
//!
//! The gap #1589 reports is `if`/`catch` being unavailable through that
//! harness: a verification run had to hand-write an `if`-free equivalent of
//! its sheet, which silently narrows every campaign that assumes ordinary
//! control flow — and `run_script` is the documented way to exercise this
//! engine, named by the fuzzer's own taxonomy.
//!
//! The fix is not a second registration list in the example. `Interp::new`
//! *is* the full bootstrap (it runs `builtins::install`, which installs every
//! command module including `cmd_control`'s `if`/`while`/`for` and
//! `cmd_error`'s `catch`/`error`/`try`), so the example must construct the
//! interpreter that way and nothing else. This test pins both halves of that:
//! the constructor really does yield the control-flow surface, and the example
//! really does use the constructor rather than assembling a subset of its own.
//!
//! Pinning the example's source text is deliberate. A behavioural test of the
//! *library* cannot notice the example drifting back to a hand-rolled
//! registration list — and that drift is exactly the reported bug.

use std::path::PathBuf;

use tcl_runtime::interp::{Code, Interp};

/// Evaluate `script` on a default-constructed interp — the same interpreter
/// `examples/run_script` builds — and return the completion code plus result.
fn eval(script: &str) -> (Code, String) {
    let mut interp = Interp::new();
    let code = interp.eval_str(script.as_bytes());
    let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
    (code, result)
}

#[test]
fn the_default_interp_carries_the_control_flow_builtins() {
    // `info commands <name>` is the engine's own answer to "is this
    // registered", so this asserts registration rather than merely that some
    // fallback happened to produce a plausible result.
    for command in [
        "if", "while", "for", "foreach", "switch", "catch", "error", "try", "proc", "return",
        "uplevel", "upvar",
    ] {
        let (code, result) = eval(&format!("info commands {command}"));
        assert_eq!(code, Code::Ok, "`info commands {command}` failed: {result}");
        assert_eq!(
            result, command,
            "`{command}` is not registered on a default interp — \
             examples/run_script would reject any sheet that uses it (issue #1589)"
        );
    }
}

#[test]
fn a_plain_if_catch_sheet_runs_through_the_default_interp() {
    // The shape #1589 could not run: a differential sheet's ordinary control
    // flow, with the error surface `catch` is normally used to capture.
    let (code, result) = eval(
        "set out {}\n\
         if {[catch {error boom} message]} {\n\
             lappend out caught $message\n\
         } else {\n\
             lappend out missed\n\
         }\n\
         foreach n {1 2 3} {\n\
             if {$n == 2} { continue }\n\
             lappend out $n\n\
         }\n\
         join $out ,",
    );
    assert_eq!(code, Code::Ok, "sheet failed: {result}");
    assert_eq!(result, "caught,boom,1,3");
}

#[test]
fn the_example_bootstraps_through_the_full_builtin_constructor() {
    let example = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/run_script.rs");
    let source = std::fs::read_to_string(&example)
        .unwrap_or_else(|e| panic!("read {}: {e}", example.display()));
    assert!(
        source.contains("Interp::new()"),
        "{} must build its interpreter with `Interp::new()` — the constructor that \
         runs `builtins::install` — so the harness always presents the engine's \
         complete command surface (issue #1589)",
        example.display()
    );
    assert!(
        !source.contains("register_builtin"),
        "{} must not register commands by hand: a private list in the harness is \
         how the surface silently diverges from the engine's (issue #1589)",
        example.display()
    );
}
