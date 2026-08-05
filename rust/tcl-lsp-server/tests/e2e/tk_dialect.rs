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

//! Native port of `tests/lsp_e2e/test_tk_dialect_e2e.py`.
//!
//! Tk command availability, end-to-end over LSP. Tk widget/window commands
//! (`button`, `pack`, `wm`, the `ttk::` forms, …) are gated two ways, verified
//! here through completion:
//!
//! 1. **Loaded** — only offered once the file makes Tk available (a
//!    `# tcl-dialect: tk` document or a `package require Tk`). A plain `.tcl`
//!    script must not be offered Tk commands.
//! 2. **Dialect** — never offered in the restricted embedded dialects
//!    (F5 iRules / iApps), even with a stray `package require Tk`.
//!
//! Tk load-gating is a native-server feature, so the pytest suite skips unless
//! `TCL_LSP_SERVER_KIND=rust`; this native port always exercises the Rust
//! server. The Tk-availability gating is driven by `# tcl-dialect:` directives
//! in ordinary `tcl`-language documents (not a `tk` language id).

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

/// Open `src`, then return the completion labels at `(line, char)`.
fn complete(
    lsp: &mut Lsp,
    uri: &str,
    src: &str,
    line: u32,
    ch: u32,
) -> std::collections::BTreeSet<String> {
    lsp.open_ready(uri, src);
    completion_labels(&lsp.completion(uri, line, ch))
        .into_iter()
        .collect()
}

// -- TestTkLoadedGating --------------------------------------------------

#[test]
fn button_absent_in_plain_tcl() {
    // No `package require Tk` — Tk commands must not surface.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let labels = complete(&mut lsp, &uri, "# tcl-dialect: tcl8.6\nbutt\n", 1, 4);
    assert!(!labels.contains("button"), "{labels:?}");
}

#[test]
fn button_offered_after_package_require_tk() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let labels = complete(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\npackage require Tk\nbutt\n",
        2,
        4,
    );
    assert!(labels.contains("button"), "{labels:?}");
}

// -- TestTkDialectGating -------------------------------------------------

#[test]
fn button_never_offered_in_irules() {
    // Even a stray `package require Tk` cannot make Tk valid in iRules.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let labels = complete(
        &mut lsp,
        &uri,
        "# tcl-dialect: f5-irules\npackage require Tk\nbutt\n",
        2,
        4,
    );
    assert!(!labels.contains("button"), "{labels:?}");
}

#[test]
fn ttk_widget_absent_in_iapps() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let labels = complete(
        &mut lsp,
        &uri,
        "# tcl-dialect: f5-iapps\npackage require Tk\nttk::butt\n",
        2,
        9,
    );
    assert!(!labels.contains("ttk::button"), "{labels:?}");
}

/// The TK diagnostic codes published for `src`.
///
/// Duplicated from the Tk-activation-gating suite (issue #1188, its own PR)
/// so this branch stands alone; whichever lands second folds the two copies
/// into one.
fn tk_codes(lsp: &mut Lsp, uri: &str, src: &str) -> Vec<String> {
    lsp.open_ready(uri, src)
        .iter()
        .filter_map(|d| d.get("code").and_then(serde_json::Value::as_str))
        .filter(|c| c.starts_with("TK"))
        .map(str::to_owned)
        .collect()
}

// Per-interpreter Tk hierarchies, end to end (issue #1141).
//
// TK1001 / TK1002 answer questions about *runtime* Tk state, and that state
// is per-interpreter: `TkCreateMainWindow` gives every interpreter that loads
// Tk its own `TkMainInfo`, with its own widget-path table and its own `.`
// root (Tk 9.0.4 `generic/tkWindow.c`). A widget path used in a child's eval
// body and in the parent script names two unrelated windows. Published
// diagnostics must reflect that in both directions.

/// The TK diagnostic codes published for `src`, paired with their 0-based
/// start line, so a test can pin *where* a finding lands.
fn tk_codes_at(lsp: &mut Lsp, uri: &str, src: &str) -> Vec<(String, u64)> {
    lsp.open_ready(uri, src)
        .iter()
        .filter_map(|d| {
            let code = d.get("code")?.as_str()?;
            if !code.starts_with("TK") {
                return None;
            }
            let line = d.get("range")?.get("start")?.get("line")?.as_u64()?;
            Some((code.to_owned(), line))
        })
        .collect()
}

/// **FP** — the `dialog1.tcl` shape: a child interpreter with its own Tk, and
/// the same widget path used on both sides. `pack` in the child and `grid` in
/// the parent claim two different containers, so no TK1001 may be published.
#[test]
fn tk1001_not_published_across_isolated_interpreters() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let codes = tk_codes(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\n\
         package require Tk\n\
         interp create child\n\
         load {} Tk child\n\
         child eval { frame .top; pack .top.a }\n\
         frame .top\n\
         grid .top.b\n",
    );
    assert!(
        !codes.iter().any(|c| c == "TK1001"),
        "cross-interpreter geometry must not be reported as a conflict: {codes:?}"
    );
}

/// **TP** — the same conflict, wholly inside the parent script, is still
/// published: the fix narrows the check, it does not disable it.
#[test]
fn tk1001_still_published_for_a_same_interpreter_conflict() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let codes = tk_codes(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\n\
         package require Tk\n\
         interp create child\n\
         child eval { frame .other }\n\
         frame .top\n\
         pack .top.a\n\
         grid .top.b\n",
    );
    assert!(codes.iter().any(|c| c == "TK1001"), "{codes:?}");
}

/// **FN** — a conflict genuinely inside one child's body is now published;
/// before #1141 it was decided against a pool that mixed in the parent's
/// calls.
#[test]
fn tk1001_published_for_a_conflict_inside_one_child_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let codes = tk_codes(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\n\
         package require Tk\n\
         interp create child\n\
         child eval { frame .top; pack .top.a; grid .top.b }\n",
    );
    assert!(codes.iter().any(|c| c == "TK1001"), "{codes:?}");
}

/// **FN** — `.top` exists only in the child, so the parent's `.top.inner`
/// really has no parent, and the finding lands on the parent-side line.
#[test]
fn tk1002_published_when_the_parent_lives_in_another_interpreter() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let found = tk_codes_at(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\n\
         package require Tk\n\
         interp create child\n\
         child eval { frame .top }\n\
         frame .top.inner\n",
    );
    assert!(
        found.contains(&("TK1002".to_owned(), 4)),
        "expected TK1002 on the parent-side `frame .top.inner`: {found:?}"
    );
}

/// **TN** — created and used in the same interpreter, across two separate
/// evals into it (their definitions accumulate, as in C): silent.
#[test]
fn tk1002_not_published_within_one_interpreter() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let codes = tk_codes(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\n\
         package require Tk\n\
         interp create child\n\
         child eval { frame .top }\n\
         child eval { frame .top.inner }\n",
    );
    assert!(
        !codes.iter().any(|c| c == "TK1002"),
        "one interpreter's own hierarchy must vouch for itself: {codes:?}"
    );
}

/// An interpreter whose path cannot be resolved statically widens rather
/// than accusing: `$i` could name the interpreter the parent script runs in,
/// so no TK1002 is published for a parent created there.
#[test]
fn unknowable_interpreter_target_publishes_nothing() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let codes = tk_codes(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\n\
         package require Tk\n\
         frame .top\n\
         proc run {i} { interp eval $i { frame .top.inner } }\n",
    );
    assert!(codes.is_empty(), "{codes:?}");
}
