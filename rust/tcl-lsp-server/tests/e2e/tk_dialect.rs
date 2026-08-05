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

use std::fmt::Write as _;

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

// -- Tk activation gating, end to end (issue #1188) ----------------------
//
// The gate decides whether a document abandons per-body memoisation for a
// whole-file re-analysis on *every keystroke*, so it must be exactly right in
// both directions: a false positive is a large, permanent latency cost, a false
// negative silently drops the TK diagnostics.

/// The TK diagnostic codes published for `src`.
fn tk_codes(lsp: &mut Lsp, uri: &str, src: &str) -> Vec<String> {
    lsp.open_ready(uri, src)
        .iter()
        .filter_map(|d| d.get("code").and_then(serde_json::Value::as_str))
        .filter(|c| c.starts_with("TK"))
        .map(str::to_owned)
        .collect()
}

/// **FP** — a document that merely *mentions* `package`, `require`, and `Tk`
/// (in a comment, in a string, and via `package require Tcl`) publishes no TK
/// diagnostics, even though it is full of Tk-shaped commands.
#[test]
fn words_mentioning_tk_do_not_activate_tk_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let codes = tk_codes(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\n\
         package require Tcl 8.6\n\
         set doc \"Tcl/Tk script text executable\"\n\
         # a stray mention: package require Tk\n\
         frame .outer.inner\n\
         pack .top.a\n\
         grid .top.b\n",
    );
    assert!(
        codes.is_empty(),
        "no TK diagnostic may fire without a real `package require Tk`: {codes:?}"
    );
}

/// **FP** — `package provide Tk` declares this file *is* part of Tk; it does
/// not load Tk into the interpreter, so it must not activate either.
#[test]
fn package_provide_tk_does_not_activate_tk_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let codes = tk_codes(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\npackage provide Tk 8.6\nframe .outer.inner\n",
    );
    assert!(codes.is_empty(), "{codes:?}");
}

/// **TP** — a real `package require Tk` still publishes the checks that need
/// whole-file state: TK1002's parent existence, and a TK1001 geometry conflict
/// spanning a proc body, which an isolated body cannot see.
#[test]
fn real_package_require_tk_still_publishes_whole_file_tk_checks() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let codes = tk_codes(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\n\
         package require Tk\n\
         frame .top\n\
         frame .missing.child\n\
         proc build {} {\n\
         pack .top.a\n\
         grid .top.b\n\
         }\n",
    );
    assert!(
        codes.iter().any(|c| c == "TK1002"),
        "the non-existent parent must still be reported: {codes:?}"
    );
    assert!(
        codes.iter().any(|c| c == "TK1001"),
        "a pack/grid conflict inside a proc body must still flush: {codes:?}"
    );
}

/// **FN** — a `package require Tk` reached only inside a procedure body is
/// still a load, and must activate: the case the post-body half of the gate
/// exists for.
#[test]
fn package_require_tk_inside_a_body_still_activates() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let codes = tk_codes(
        &mut lsp,
        &uri,
        "# tcl-dialect: tcl8.6\n\
         proc setup {} { package require Tk }\n\
         setup\n\
         frame .missing.child\n",
    );
    assert!(codes.iter().any(|c| c == "TK1002"), "{codes:?}");
}

/// **FP, at scale** — a large generated document with no `package require Tk`
/// publishes no TK diagnostics and keeps answering requests: the `filetypes.tcl`
/// shape that used to be forced onto a whole-file re-analysis, per keystroke,
/// by the word `Tk` appearing in generated data.
#[test]
fn large_generated_non_tk_document_stays_clean_and_responsive() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let mut src = String::from("# tcl-dialect: tcl8.6\npackage require Tcl 8.6\n");
    src.push_str("proc classify {kind} {\n");
    for i in 0..2000 {
        let _ = writeln!(
            src,
            "    if {{$kind eq \"k{i}\"}} {{ return \"Tcl/Tk script text executable\" }}"
        );
    }
    src.push_str("    return unknown\n}\n");
    // A full debug-build analysis of a 2000-branch proc is legitimately tens
    // of seconds on a loaded two-core CI runner: pass the large-document
    // backstop `open_ready_timeout` exists for instead of betting
    // DEFAULT_TIMEOUT covers it (its doc comment is about exactly this test
    // shape).  Load scaling still applies on top.
    let codes: Vec<String> = lsp
        .open_ready_timeout(&uri, &src, std::time::Duration::from_secs(180))
        .iter()
        .filter_map(|d| d.get("code").and_then(serde_json::Value::as_str))
        .filter(|c| c.starts_with("TK"))
        .map(str::to_owned)
        .collect();
    assert!(codes.is_empty(), "{codes:?}");
    // Still serving: a completion resolves rather than timing out.
    let labels = completion_labels(&lsp.completion(&uri, 1, 7));
    assert!(
        !labels.is_empty(),
        "server stopped answering after the open"
    );
}
