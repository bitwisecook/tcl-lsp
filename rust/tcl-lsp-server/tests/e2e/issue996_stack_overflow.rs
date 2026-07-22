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

//! End-to-end coverage for issue #996: `Analyser::analyse()` crashed the
//! whole process with an uncatchable stack overflow (SIGABRT) on Tcl source
//! nested 100-150 levels deep.
//!
//! Root cause (confirmed empirically, not just inferred): the analyser's
//! `analyse_body` recursion is correctly bounded by `MAX_BODY_DEPTH` (256),
//! but 256 real Rust stack frames of that recursive chain need more stack
//! than Tokio's default 2 MiB worker-thread stack provides — the thread the
//! native LSP server actually runs analysis on via `tokio::spawn`. `ulimit -s
//! 2048` against the *unfixed* binary reproduces a crash at nesting depth
//! 130-140, an exact match for the reported range; `tcl-lsp-server/src/
//! main.rs` now builds its Tokio runtime with a 64 MiB `thread_stack_size`,
//! which eliminates it (verified: the same pathological input survives at
//! every depth up to and well past the analyser's own cap).
//!
//! This suite drives the real, packaged native server (not the analyser
//! library function directly — a unit test calling `Analyser::analyse` runs
//! on `cargo test`'s own worker thread, which has the *same* undersized
//! default stack, so it isn't a faithful stand-in for "does the shipped
//! server survive this"). A process abort here shows up as the reader
//! thread hitting EOF, so a regression fails as an `await_diagnostics`
//! timeout — loud, not a silent pass.

use crate::common::{Lsp, unique_uri};

/// `if {1} { if {1} { ... } }`, `depth` levels deep, wrapped in a `proc`
/// body. Each level is its own line so a stray parse error is easy to spot
/// from the diagnostic's line number if this ever fails.
fn nested_if_source(depth: usize) -> String {
    let mut source = String::from("proc deepnest {} {\n");
    for _ in 0..depth {
        source.push_str("if {1} {\n");
    }
    for _ in 0..depth {
        source.push_str("}\n");
    }
    source.push_str("}\n");
    source
}

/// The exact reported crash range (100-150 levels): the server must survive
/// and answer with diagnostics, not abort.
#[test]
fn nested_if_at_reported_crash_depth_survives_and_returns_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("issue996-depth140.tcl");
    let source = nested_if_source(140);
    lsp.open_document(&uri, &source);
    let diags = lsp.await_diagnostics(&uri);
    assert!(
        !diags.is_empty(),
        "expected diagnostics for 140-level nested if, got none"
    );

    // Prove the *same server process* is still alive and responsive to
    // unrelated work afterwards, not just that one lucky notification beat
    // a delayed abort — a crash tears down every in-flight task, but the
    // reader thread only notices on its next read, so a single diagnostics
    // publish arriving is not on its own proof of survival.
    let other_uri = unique_uri("issue996-followup.tcl");
    lsp.open_document(&other_uri, "puts hi\n");
    lsp.await_diagnostics(&other_uri);
}

/// Depth well past the analyser's own `MAX_BODY_DEPTH` cap (256) — the
/// adversarial/`DoS` shape the issue calls out (deeply-nested generated /
/// minified Tcl). Must still survive and respond within the harness's
/// default timeout, not hang or abort.
#[test]
fn nested_if_far_past_analyser_cap_survives() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("issue996-depth2000.tcl");
    let source = nested_if_source(2000);
    lsp.open_document(&uri, &source);
    let diags = lsp.await_diagnostics(&uri);
    assert!(
        diags.iter().any(|d| d["code"] == "E207"),
        "expected an E207 (nesting depth exceeds analysis limit) diagnostic, got {diags:?}"
    );
}

/// Mixed control-flow keywords (`if`/`while`/`foreach`/`try`/`catch`), not
/// just one repeated — the issue explicitly calls out "any mix" as part of
/// the repro shape.
#[test]
fn mixed_control_flow_nesting_survives() {
    let kinds = [
        "if {1} {",
        "while {1} {",
        "foreach x {1} {",
        "try {",
        "catch {",
    ];
    let depth = 150;
    let mut source = String::from("proc deepmix {} {\n");
    for i in 0..depth {
        source.push_str(kinds[i % kinds.len()]);
        source.push('\n');
    }
    for _ in 0..depth {
        source.push_str("}\n");
    }
    source.push_str("}\n");

    let mut lsp = Lsp::tcl();
    let uri = unique_uri("issue996-mixed.tcl");
    lsp.open_document(&uri, &source);
    // Just needs to come back at all within the default timeout — an abort
    // (no diagnostics ever) or a hang (timeout) both fail this the same way
    // `await_diagnostics` already fails any other missing-publish case.
    lsp.await_diagnostics(&uri);
}
