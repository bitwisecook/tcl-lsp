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

//! One or two cases for each basic code, locked to the published output and its anchor: W100, W302, W128, W212, W126, W101, W216 and T101.

use super::*;

#[test]
fn unbraced_expr_is_w100() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "if $a {puts x}\n"), "W100"));
}

#[test]
fn tk_diagnostics_do_not_offer_tailcall() {
    // The memoised diagnostics path receives Tk as a set-only dialect. It must
    // retain the typed profile rather than use the unknown-dialect optimiser
    // default, which would incorrectly offer Tcl 8.6's O121 tailcall rewrite.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tk");
    let diagnostics = lsp.open_ready_lang(&uri, "proc recurse {} { recurse }\n", "tk");
    assert!(!has_code(&diagnostics, "O121"), "{diagnostics:?}");
}

#[test]
fn catch_without_result_is_w302() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert!(has_code(&lsp.open_ready(&uri, "catch {error e}\n"), "W302"));
}

#[test]
fn clean_file_has_no_diagnostics() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    assert_eq!(
        lsp.open_ready(&uri, "set x [clock seconds]\nputs $x\n"),
        [] as [serde_json::Value; 0]
    );
}

#[test]
fn renamed_away_command_is_w128() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc a {} {return 1}\na\nrename a b\na\n");
    assert!(has_code(&diags, "W128"));
}

#[test]
fn unbraced_expr_inside_catch_body_is_w100() {
    // The analyser must recurse into `catch { ... }` bodies, so the unbraced
    // `expr` inside is a W100 just as at the top level (catch-body-walk fix).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "catch { set y [expr $a + $b] } msg\n");
    assert!(has_code(&diags, "W100"));
    assert_eq!(on_line(&diags, "W100"), BTreeSet::from([0]));
}

#[test]
fn unbraced_expr_inside_tcltest_body_is_w100() {
    // `tcltest::test` evaluates its body as Tcl, so diagnostics inside it are
    // real.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "package require tcltest\ntcltest::test t {d} { set y [expr $a + $b] } {}\n";
    assert!(has_code(&lsp.open_ready(&uri, src), "W100"));
}

#[test]
fn while_true_with_throw_or_tailcall_is_not_infinite() {
    // W241 must not fire when the body leaves the loop via `throw`/`tailcall`.
    let mut lsp = Lsp::tcl();
    for src in ["while 1 {throw MYERR boom}\n", "while 1 {tailcall next}\n"] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, src);
        assert!(!has_code(&diags, "W241"), "{src:?} → {diags:?}");
    }
    // Control: a body with no exit is still flagged.
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "while 1 {incr n}\n");
    assert!(has_code(&diags, "W241"), "got {diags:?}");
}

#[test]
fn w212_covers_registry_name_positions_end_to_end() {
    // `vwait $x` and `catch {…} $res` are name/value confusions the old
    // hardcoded list missed; the registry roles now supply them.
    let mut lsp = Lsp::tcl();
    for src in [
        "proc p {} { vwait $x }\n",
        "proc p {} { catch {error e} $res }\n",
    ] {
        let uri = unique_uri("tcl");
        let diags = lsp.open_ready(&uri, src);
        assert!(has_code(&diags, "W212"), "{src:?} → {diags:?}");
    }
}

#[test]
fn w126_anchors_at_the_channel_argument() {
    // `puts notachan hello` — the non-channel literal `notachan` (cols 5..13)
    // is the problem, not the whole command.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "puts notachan hello\n");
    let w126 = with_code(&diags, "W126");
    assert_eq!(w126.len(), 1, "got {diags:?}");
    assert_eq!(diag_range(&w126[0]), ((0, 5), (0, 13)), "got {diags:?}");
}

#[test]
fn w101_anchors_at_the_substituted_argument() {
    // `eval "safeprefix" $x` — the hazard is `$x` (col 18..20), not the safe
    // literal prefix. The squiggle must point at the substituted word.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "eval \"safeprefix\" $x\n");
    let w101 = with_code(&diags, "W101");
    assert_eq!(w101.len(), 1, "got {diags:?}");
    assert_eq!(diag_range(&w101[0]), ((0, 18), (0, 20)), "got {diags:?}");
}

#[test]
fn w216_upvar_local_indirect_array_is_silent() {
    // `upvar 1 remote ${arr}(x)` — the local-name slot is the legitimate
    // indirect-array idiom, so W216 must not fire (the two name-position lists
    // had drifted; W216's omitted `upvar`).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "proc p {arr} { upvar 1 remote ${arr}(x) }\n");
    assert!(!has_code(&diags, "W216"), "got {diags:?}");
    // Control: the same shape in a value position IS a broken read.
    let uri2 = unique_uri("tcl");
    let diags2 = lsp.open_ready(&uri2, "proc p {arr} { puts ${arr}(x) }\n");
    assert!(has_code(&diags2, "W216"), "got {diags2:?}");
}

// T101 (tainted data into `puts`) end-to-end: the diagnostic must highlight
// only the tainted argument word, not the whole `puts $x` statement.

#[test]
fn puts_tainted_data_has_tight_argument_span() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "set x [gets stdin]\nputs $x\n";
    let diags = lsp.open_ready(&uri, src);
    let t101: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("T101"))
        .collect();
    assert_eq!(t101.len(), 1, "expected exactly one T101: {diags:?}");
    let range = &t101[0]["range"];
    // Line 1 (0-based) is `puts $x`; `$x` starts at character 5, ends at 7 —
    // not the whole 7-character statement starting at character 0.
    assert_eq!(range["start"]["line"], 1);
    assert_eq!(
        range["start"]["character"], 5,
        "span must start at `$x`, not the `puts` command word: {diags:?}"
    );
    assert_eq!(range["end"]["line"], 1);
    assert_eq!(
        range["end"]["character"], 7,
        "span must cover only `$x`: {diags:?}"
    );
}
