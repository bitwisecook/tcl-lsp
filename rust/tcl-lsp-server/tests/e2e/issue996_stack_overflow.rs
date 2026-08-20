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
//! `analyse_body` recursion is correctly bounded by `MAX_BODY_DEPTH` (256 at
//! the time), but 256 real Rust stack frames of that recursive chain need
//! more stack
//! than Tokio's default 2 MiB worker-thread stack provides — the thread the
//! native LSP server actually runs analysis on via `tokio::spawn`. `ulimit -s
//! 2048` against the *unfixed* binary reproduces a crash at nesting depth
//! 130-140, an exact match for the reported range; `tcl-lsp-server/src/
//! main.rs` now builds its Tokio runtime with a 64 MiB `thread_stack_size`,
//! which eliminates it (verified: the same pathological input survives at
//! every depth up to and well past the analyser's own cap).
//!
//! Issue #1654 later closed the other half: a big stack made the *shipped*
//! entry points safe, but the cap itself still did not fit the 2 MiB any
//! ordinary caller gets, and the lowering walk — fatter per level than
//! `analyse_body`, and never in this suite's frame — aborted on ~400 nested
//! `foreach` bodies. The braced-body caps are now derived from that budget
//! in `tcl_compiler::depth_guard`, so they trip before the stack runs out
//! rather than long after; the 64 MiB here remains as headroom.
//!
//! This suite drives the real, packaged native server (not the analyser
//! library function directly — a unit test calling `Analyser::analyse` runs
//! on `cargo test`'s own worker thread, which has the *same* undersized
//! default stack, so it isn't a faithful stand-in for "does the shipped
//! server survive this"). A process abort here shows up as the reader
//! thread hitting EOF, so a regression fails as an `await_diagnostics`
//! timeout — loud, not a silent pass.
//!
//! The investigation this issue triggered found the same bug class in the
//! formatter and minifier (`docs/design/compiler/
//! recursive-descent-depth-limits.md`), both LSP-reachable (`textDocument/
//! formatting`, the `tcl-lsp.minifyDocument` workspace command) — covered
//! here too, driving the same real server process.

use serde_json::{Value, json};

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

/// Depth well past the analyser's own `MAX_BODY_DEPTH` cap — the
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

/// The formatter (`tcl_lsp_core::formatting::engine::format_body`) recurses
/// once per nested body, independently of the analyser, and is now
/// depth-capped (`MAX_FORMAT_DEPTH` = 128) — reachable over the wire via
/// `textDocument/formatting`. 500 is comfortably past that cap; the request
/// must return (formatted, with the excess nesting left untouched past the
/// cap) rather than hang or crash the server.
#[test]
fn formatting_survives_deep_nesting() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("issue996-format-depth500.tcl");
    lsp.open_ready(&uri, &nested_if_source(500));
    let edits = lsp.formatting(&uri, 4, true);
    assert!(!edits.is_null(), "expected formatting edits, got null");

    // Same "the process is still alive" proof the analyser tests use.
    let other_uri = unique_uri("issue996-format-followup.tcl");
    lsp.open_document(&other_uri, "puts hi\n");
    lsp.await_diagnostics(&other_uri);
}

/// The minifier (`tcl_lsp_core::minify::minify_body`) has the same
/// recursion shape as the formatter and is now capped the same way
/// (`MAX_MINIFY_DEPTH` = 128) — reachable over the wire via the
/// `tcl-lsp.minifyDocument` workspace command. 500 is comfortably past that
/// cap; the request must return a minified document rather than hang or
/// crash the server.
#[test]
fn minify_survives_deep_nesting() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("issue996-minify-depth500.tcl");
    lsp.open_ready(&uri, &nested_if_source(500));
    let result = lsp.execute_command("tcl-lsp.minifyDocument", json!([uri, false, false, false]));
    assert!(!result.is_null(), "expected a minify result, got null");
    let minified = result.get("source").and_then(Value::as_str).unwrap_or("");
    assert!(!minified.is_empty(), "expected non-empty minified source");

    let other_uri = unique_uri("issue996-minify-followup.tcl");
    lsp.open_document(&other_uri, "puts hi\n");
    lsp.await_diagnostics(&other_uri);
}

/// `tcl_irules::walker` (the object-reference walker semantic tokens use to
/// highlight `pool`/`node`/`class` names) has its own, independent
/// recursion — `walk`/`recurse_token`, capped at `MAX_WALK_DEPTH` (128) —
/// reachable over the wire via `textDocument/semanticTokens/full` on an
/// iRules document. 300 nested `[…]` command substitutions is comfortably
/// past that cap; the request must return (highlighting whatever it found
/// before the cap stopped collection) rather than hang or crash the server.
#[test]
fn irules_semantic_tokens_survive_deep_command_substitution() {
    let mut lsp = Lsp::irules();
    let uri = unique_uri("issue996-irules-depth300.tcl");
    let depth = 300;
    let mut source = "when HTTP_REQUEST {\n  set x ".to_owned();
    for _ in 0..depth {
        source.push('[');
    }
    source.push_str("pool /Common/p");
    for _ in 0..depth {
        source.push(']');
    }
    source.push_str("\n}\n");
    lsp.open_document_lang(&uri, &source, "tcl-irule", 1);
    let tokens = lsp.semantic_tokens(&uri);
    assert!(
        !tokens.is_null(),
        "expected a semantic tokens result, got null"
    );

    let other_uri = unique_uri("issue996-irules-followup.tcl");
    lsp.open_document(&other_uri, "puts hi\n");
    lsp.await_diagnostics(&other_uri);
}
