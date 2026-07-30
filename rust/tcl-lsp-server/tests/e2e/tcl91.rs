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

//! Native port of `tests/lsp_e2e/test_tcl91_e2e.py`.
//!
//! Tcl 9.1 dialect support, end-to-end over LSP. Oracle: C Tcl 9.1b0 source. The
//! dialect is pinned with a `# tcl-dialect: tcl9.1` directive (server-side source
//! detection), and behaviour is observed through completion + diagnostics.

use crate::common::helpers::*;
use crate::common::{Lsp, unique_uri};

use serde_json::Value;

use std::collections::BTreeSet;

/// The set of completion labels from a completion result.
fn labels(result: &Value) -> BTreeSet<String> {
    completion_labels(result).into_iter().collect()
}

/// The set of `code` strings carried by `diags`.
fn codes(diags: &[Value]) -> BTreeSet<String> {
    diags
        .iter()
        .map(|d| match d.get("code") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "None".to_owned(),
        })
        .collect()
}

/// Open a buffer pinned to `dialect` and complete a command-position word.
fn complete_cmd(lsp: &mut Lsp, dialect: &str, partial: &str) -> BTreeSet<String> {
    let uri = unique_uri("tcl");
    let src = format!("# tcl-dialect: {dialect}\n{partial}\n");
    lsp.open_ready(&uri, &src);
    labels(&lsp.completion(&uri, 1, u32::try_from(partial.len()).unwrap()))
}

// -- TestTcl91Completion -------------------------------------------------

#[test]
fn unicode_and_timer_offered_in_91() {
    // doc/unicode.n, doc/timer.n — both are new commands in 9.1.
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "unic").contains("unicode"));
    assert!(complete_cmd(&mut lsp, "tcl9.1", "time").contains("timer"));
}

#[test]
fn unicode_and_timer_absent_in_90() {
    let mut lsp = Lsp::tcl();
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "unic").contains("unicode"));
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "time").contains("timer"));
}

#[test]
fn math_and_lfilter_offered_in_91() {
    // doc/divmod.n, doc/lfilter.n — new commands in 9.1 (C tclBasic.c).
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "divm").contains("divmod"));
    assert!(complete_cmd(&mut lsp, "tcl9.1", "lfil").contains("lfilter"));
}

#[test]
fn math_absent_in_90() {
    let mut lsp = Lsp::tcl();
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "divm").contains("divmod"));
    assert!(!complete_cmd(&mut lsp, "tcl9.0", "lfil").contains("lfilter"));
}

#[test]
fn commands_90_still_offered_in_91() {
    // A `.1` release is additive: `lseq` (9.0) persists in 9.1.
    let mut lsp = Lsp::tcl();
    assert!(complete_cmd(&mut lsp, "tcl9.1", "lseq").contains("lseq"));
}

// `oo::Helpers::link` version-gating (issue #923, Codex review on PR
// #1020): a genuine core TclOO builtin only since 9.0 (confirmed against
// tclsh 9.0.4 — no package needed); under 8.6/8.7 it exists only via the
// Tcllib `ooutil` package (confirmed against tclsh 8.6.14 — bare `link`
// with no `package require` is `invalid command name "link"`).

#[test]
fn link_is_unconditionally_available_in_90() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "# tcl-dialect: tcl9.0\noo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n        return [foo 1]\n    }\n}\n",
    );
    assert!(
        !codes(&diags).contains("W120"),
        "core Tcl 9.0 link needs no package require ooutil: {:?}",
        codes(&diags)
    );
}

#[test]
fn link_requires_ooutil_package_require_in_86() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n        return [foo 1]\n    }\n}\n",
    );
    assert!(
        codes(&diags).contains("W120"),
        "8.6/8.7 link is Tcllib ooutil-only, not core, with no package require anywhere: {:?}",
        codes(&diags)
    );
}

#[test]
fn link_stays_silent_in_86_once_ooutil_is_required() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "package require ooutil\noo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n        return [foo 1]\n    }\n}\n",
    );
    assert!(
        !codes(&diags).contains("W120"),
        "a real `package require ooutil` makes 8.6/8.7 link known: {:?}",
        codes(&diags)
    );
}

// The whole `oo::Helpers` family is method-context-scoped (issue #1026).
//
// tclsh 9.0.4 at the top level answers `invalid command name` for every one
// of `link` / `my` / `next` / `nextto` / `self` / `classvariable`, and
// `info commands ::link` is empty — while inside a method body
// `namespace which -command link` answers `::oo::Helpers::link` (and
// `… my` answers `::oo::ObjN::my`, an object-namespace command rather than
// a helper). tclsh 8.6.14 agrees for the four members it ships.

/// The `# tcl-dialect: tcl9.0` document the repro in issue #1026 uses.
fn scoped_family_doc(body: &str) -> String {
    format!("# tcl-dialect: tcl9.0\n{body}")
}

#[test]
fn top_level_link_draws_w123() {
    // Issue #1026's own repro: `link foo` at the top level under tcl9.0.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, &scoped_family_doc("link foo\n"));
    assert!(
        codes(&diags).contains("W123"),
        "top-level `link foo` is `invalid command name \"link\"` in real Tcl 9.0: {:?}",
        codes(&diags)
    );
}

#[test]
fn method_body_link_stays_clean() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        &scoped_family_doc(
            "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n        return [foo 1]\n    }\n}\n",
        ),
    );
    assert!(
        !codes(&diags).contains("W123"),
        "`link` resolves inside a method body: {:?}",
        codes(&diags)
    );
}

#[test]
fn top_level_family_members_all_draw_w123() {
    let mut lsp = Lsp::tcl();
    for word in ["my", "next", "nextto", "self", "classvariable"] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, &scoped_family_doc(&format!("{word} foo\n")));
        assert!(
            codes(&diags).contains("W123"),
            "top-level `{word}` is `invalid command name`: {:?}",
            codes(&diags)
        );
    }
}

#[test]
fn link_is_not_offered_in_top_level_completion() {
    // Completion follows the same scoping: offering `link` at the top level
    // would insert a call real Tcl rejects.
    let mut lsp = Lsp::tcl();
    assert!(
        !complete_cmd(&mut lsp, "tcl9.0", "lin").contains("link"),
        "no top-level completion of a method-context-only command",
    );
}

/// Byte-for-byte the VS Code fixture `editors/vscode/testFixture/
/// ooMethodContext.tcl`, so the extension test's cursor positions are
/// pinned here against the same server.
const OO_METHOD_CONTEXT_FIXTURE: &str = "# tcl-dialect: tcl9.0\nlin\noo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        lin\n    }\n}\n";

#[test]
fn link_is_offered_inside_a_method_body() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    lsp.open_ready(&uri, OO_METHOD_CONTEXT_FIXTURE);
    // Line 1 is the top-level `lin`; line 5 is the `lin` inside `method bar`.
    let top = labels(&lsp.completion(&uri, 1, 3));
    assert!(!top.contains("link"), "top level offered `link`: {top:?}");
    let inner = labels(&lsp.completion(&uri, 5, 11));
    assert!(inner.contains("link"), "expected `link` among {inner:?}");
}

#[test]
fn top_level_link_has_no_hover_but_a_method_body_one_does() {
    let mut lsp = Lsp::tcl();
    let top = unique_uri("tcl");
    lsp.open_ready(&top, &scoped_family_doc("link foo\n"));
    assert!(
        hover_text(&lsp.hover(&top, 1, 1)).is_empty(),
        "an unresolvable top-level `link` must not hover as a builtin",
    );
    let inner = unique_uri("tcl");
    lsp.open_ready(
        &inner,
        &scoped_family_doc(
            "oo::class create Widget {\n    method foo {x} { return $x }\n    method bar {} {\n        link foo\n    }\n}\n",
        ),
    );
    assert!(
        hover_text(&lsp.hover(&inner, 4, 9)).contains("link"),
        "the same word hovers inside a method body",
    );
}

#[test]
fn the_qualified_oo_helpers_spelling_resolves_at_the_top_level() {
    // `info commands ::oo::Helpers::link` answers `::oo::Helpers::link`
    // under tclsh 9.0.4 — calling it outside a method is a *runtime* error
    // ("may only be called from inside a method"), not an unknown command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, &scoped_family_doc("::oo::Helpers::link foo\n"));
    assert!(
        !codes(&diags).contains("W123"),
        "the qualified spelling is a real global command: {:?}",
        codes(&diags)
    );
}

// -- TestTcl91Operators --------------------------------------------------
// doc/expr.n — the `lt`/`le`/`gt`/`ge` string operators (TIP 461) are 9.0+.

#[test]
fn lt_operator_no_w003_in_91() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.1\nexpr {$a lt $b}\n");
    assert!(!codes(&diags).contains("W003"));
}

#[test]
fn lt_operator_flags_w003_in_86() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.6\nexpr {$a lt $b}\n");
    assert!(codes(&diags).contains("W003"));
}

#[test]
fn ge_operator_no_w003_in_91() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl9.1\nexpr {$a ge $b}\n");
    assert!(!codes(&diags).contains("W003"));
}

#[test]
fn ge_operator_flags_w003_in_86() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "# tcl-dialect: tcl8.6\nexpr {$a ge $b}\n");
    assert!(codes(&diags).contains("W003"));
}
