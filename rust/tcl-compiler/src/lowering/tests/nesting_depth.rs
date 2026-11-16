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

//! Nesting past the lowering depth cap becomes a barrier instead of a crash.

use super::*;

/// `if {1} { if {1} { ... } }`, `depth` levels deep.
fn nested_if_source(depth: u32) -> String {
    let mut source = String::new();
    for _ in 0..depth {
        source.push_str("if {1} {\n");
    }
    for _ in 0..depth {
        source.push_str("}\n");
    }
    source
}

/// Depth-first search for a [`Statement::Barrier`] whose `reason`
/// contains `needle`, anywhere in `script`'s tree (into `If` clauses and
/// `else` bodies — the only nesting `nested_if_source` produces).
fn contains_barrier_reason(script: &Script, needle: &str) -> bool {
    script.statements.iter().any(|stmt| match stmt {
        Statement::Barrier { reason, .. } => reason.contains(needle),
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses
                .iter()
                .any(|c| contains_barrier_reason(&c.body, needle))
                || else_body
                    .as_ref()
                    .is_some_and(|e| contains_barrier_reason(e, needle))
        }
        _ => false,
    })
}

/// `lower_script` / `lower_body` recurse one Rust frame group per `if`
/// nesting level, so they need the same depth cap as `analyse_body`
/// (`tcl_compiler::analyser::commands::MAX_BODY_DEPTH`) and
/// `cfg_builder::lower_script` (`MAX_LOWER_DEPTH`). A document whose IR
/// lowering runs unguarded defeats the CFG builder's own guard
/// downstream — the crash happens one stage earlier. Lowering 2000
/// levels must neither hang nor overflow the stack, and must record the
/// cap trip as a `Statement::Barrier` (unknown effects) rather than
/// silently truncating to an empty, falsely-dead-looking script.
#[test]
fn deeply_nested_if_past_max_lower_depth_barriers_not_crashes() {
    let source = nested_if_source(2000);
    // `cargo test` runs each test on a small default-stack thread — see
    // the identical helper's doc comment in `analyser::commands::tests`.
    let m = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || lower_to_ir(&source, &reg()))
        .expect("spawn big-stack test thread")
        .join()
        .expect("lower_to_ir on big-stack thread panicked");
    assert!(
        contains_barrier_reason(&m.top_level, "nesting depth exceeds analysis limit"),
        "expected a depth-cap Barrier somewhere in the lowered tree"
    );
}

/// Depth of the deepest chain of nested `If` statements in `script`.
fn if_chain_depth(script: &Script) -> u32 {
    script
        .statements
        .iter()
        .map(|stmt| match stmt {
            Statement::If { clauses, .. } => {
                1 + clauses.first().map_or(0, |c| if_chain_depth(&c.body))
            }
            _ => 0,
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn shallow_nested_if_lowers_with_no_barrier() {
    // False-positive guard: ordinary, hand-written nesting must lower
    // as real `If` statements all the way down, never barriered.
    let source = nested_if_source(10);
    let m = lower_to_ir(&source, &reg());
    assert!(
        !contains_barrier_reason(&m.top_level, "nesting depth exceeds analysis limit"),
        "shallow nesting must not trip the depth cap"
    );
    // 10 real nested `If`s, not a single opaque barrier standing in for
    // all of them.
    assert_eq!(if_chain_depth(&m.top_level), 10);
}
