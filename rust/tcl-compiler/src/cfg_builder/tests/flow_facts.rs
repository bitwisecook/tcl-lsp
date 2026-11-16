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

//! `escaping_loop_jumps` over un-lowered scripts: `try` propagation, early stop and deep nesting.

use super::*;

// --- escaping_loop_jumps: try propagation and
// dead-code early-stop
//
// These exercise the helper directly: their end-to-end W210 effect is
// masked by a separate, pre-existing `while 1` exit-reachability behaviour
// (Rust treats the post-loop block of an infinite loop as reachable), so we
// assert the `(can_break, can_continue)` result the wiring keys off.

#[test]
fn escaping_loop_jumps_follows_try_but_not_catch() {
    // `try { break }` (no matching handler) propagates the break to the
    // enclosing loop; `catch { break }` absorbs it (yields code 3). Confirmed
    // against tclsh 9.0.3.
    assert_eq!(
        escaping_loop_jumps(&lower_module("try { break }").top_level, 0),
        (true, false),
        "try propagates break",
    );
    assert_eq!(
        escaping_loop_jumps(&lower_module("try { continue } finally { }").top_level, 0),
        (false, true),
        "try (with finally) propagates continue",
    );
    assert_eq!(
        escaping_loop_jumps(
            &lower_module("try { puts x } on error {} { break }").top_level,
            0
        ),
        (true, false),
        "a break in a try handler body propagates",
    );
    assert_eq!(
        escaping_loop_jumps(&lower_module("catch { break }").top_level, 0),
        (false, false),
        "catch absorbs break (does not propagate)",
    );
}

#[test]
fn escaping_loop_jumps_stops_after_non_completing_stmt() {
    // A `break`/`continue` after a statement that cannot complete normally
    // is dead code and must not be collected as an escaping jump (else it
    // forges a spurious loop-exit edge).
    assert_eq!(
        escaping_loop_jumps(&lower_module("error bad\nbreak").top_level, 0),
        (false, false),
        "dead break after error is not collected",
    );
    assert_eq!(
        escaping_loop_jumps(&lower_module("return\ncontinue").top_level, 0),
        (false, false),
        "dead continue after return is not collected",
    );
    // A live jump before dead code IS collected (and stops the scan there).
    assert_eq!(
        escaping_loop_jumps(&lower_module("break\nset x 1").top_level, 0),
        (true, false),
        "live break is collected",
    );
}

/// `escaping_loop_jumps` and the
/// mutually-recursive `switch_escaping_jumps` recurse once per nested
/// `if`/`switch`/`Block`/`UpFrame`/`try` body, with no depth cap of
/// their own before this fix. Transitively bounded to
/// `MAX_LOWER_DEPTH` (256) by the lowering pass today, so this is
/// defence-in-depth / consistency with every other full-tree walker in
/// this crate, not a currently-reproducible crash. 1000 levels of
/// source nesting is comfortably past this new cap; the assertion is
/// that `escaping_loop_jumps` returns at all, not what it returns.
/// Spawns its own big-stack thread since the lexer/CST/segmenter
/// stages upstream of the lowering cap still walk the full
/// un-truncated source nesting before that cap trims it — same
/// rationale as
/// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
#[test]
fn deeply_nested_if_survives_escaping_loop_jumps() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = String::new();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("break\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let _ = escaping_loop_jumps(&lower_module(&src).top_level, 0);
        })
        .unwrap()
        .join()
        .unwrap();
}
