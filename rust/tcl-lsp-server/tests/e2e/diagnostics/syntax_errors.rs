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

//! Malformed `if` (E004) and the stray-closer and recovery diagnostics (E100 to E103): what each names and where it anchors.

use super::*;

#[test]
fn e004_bare_if_names_the_invoked_command_and_anchors_on_it() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if\n");
    let e004 = with_code(&diags, "E004");
    assert_eq!(e004.len(), 1, "got {diags:?}");
    assert_eq!(message(&e004[0]), "No expression after \"if\" argument");
    assert_eq!(diag_range(&e004[0]), ((0, 0), (0, 2)));
}

#[test]
fn e004_condition_without_body_anchors_on_the_condition_word() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1}\n");
    let e004 = with_code(&diags, "E004");
    assert_eq!(e004.len(), 1, "got {diags:?}");
    assert_eq!(message(&e004[0]), "No script following \"1\" argument");
    assert_eq!(diag_range(&e004[0]), ((0, 3), (0, 6)));
}

#[test]
fn e004_extra_words_anchors_only_the_extra_word_not_the_whole_statement() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1} {a} {b} {c}\n");
    let e004 = with_code(&diags, "E004");
    assert_eq!(e004.len(), 1, "got {diags:?}");
    assert_eq!(
        message(&e004[0]),
        "Extra words after \"else\" clause in \"if\" command"
    );
    // Just "{c}" (columns 15..18) — not the whole `if ... {c}` statement.
    assert_eq!(diag_range(&e004[0]), ((0, 15), (0, 18)));
}

#[test]
fn e004_leading_else_bareword_condition_is_not_flagged() {
    // `if else {a}` — "else" is a well-formed (if ill-typed) condition,
    // not a malformed `if`; see the FP fix in
    // `tcl-compiler`'s `analyser::state::tests::fp_leading_else_is_not_malformed`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if else {a}\n");
    assert!(with_code(&diags, "E004").is_empty(), "got {diags:?}");
}

#[test]
fn e004_qualified_double_colon_if_is_checked() {
    // `::if` names the same global command as `if` — the E004 dispatch
    // is generic on the resolved spec's hook, not on the literal
    // `cmd_name == "if"` text, so registry `::`-stripping picks this up
    // for free.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "::if {1} {a} {b} {c}\n");
    assert_eq!(with_code(&diags, "E004").len(), 1, "got {diags:?}");
}

#[test]
fn e004_well_formed_elseif_chain_has_no_e004() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "if {$a} {\n  puts a\n} elseif {$b} {\n  puts b\n} else {\n  puts c\n}\n",
    );
    assert!(with_code(&diags, "E004").is_empty(), "got {diags:?}");
}

#[test]
fn e004_no_duplicate_e002_for_the_same_malformed_if() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "if {1}\n");
    assert_eq!(with_code(&diags, "E004").len(), 1, "got {diags:?}");
    assert!(
        with_code(&diags, "E002").is_empty(),
        "E004 must not carry a redundant generic E002 alongside it: {diags:?}"
    );
}

// E100 / E102 stray-closer diagnostics.
//
// A bare `]` / `}` has no special meaning to Tcl outside `[...]` / `{...}`,
// so these are "probably a typo" heuristics, not hard parse errors. The
// range must be tight around the offending character: it must not exclude
// the stray character itself, must not be a fix-less diagnostic spanning
// the whole command instead of just the character, and a repair must not
// corrupt an unrelated "Unknown command" diagnostic elsewhere in the file.

fn range_of(diags: &[Value], code: &str) -> (i64, i64, i64, i64) {
    let d = diags
        .iter()
        .find(|d| code_str(d).as_deref() == Some(code))
        .unwrap_or_else(|| panic!("no {code} in {diags:?}"));
    let r = &d["range"];
    (
        r["start"]["line"].as_i64().unwrap(),
        r["start"]["character"].as_i64().unwrap(),
        r["end"]["line"].as_i64().unwrap(),
        r["end"]["character"].as_i64().unwrap(),
    )
}

#[test]
fn e100_range_is_tight_around_bracket_without_a_fix() {
    // `set x blah]` — no known command / arity overflow, so no fix is
    // available; the highlighted range must be just the `]` (char 10..11),
    // not the whole command from `set`.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x blah]\n");
    assert_eq!(range_of(&diags, "E100"), (0, 10, 0, 11));
}

#[test]
fn e100_range_includes_the_bracket_with_a_fix() {
    // `puts string]` — `string` is a known command right before the `]`;
    // the range must still run through (and include) the `]` itself.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "puts string]\n");
    let (l0, c0, l1, c1) = range_of(&diags, "E100");
    assert_eq!((l0, l1), (0, 0));
    assert_eq!(c1, 12, "range end must include the ']' at char 11");
    assert!(c0 < c1);
}

#[test]
fn e102_range_is_tight_around_embedded_brace() {
    // A `}` embedded in a bareword (not the whole token) must still be
    // flagged, with a range covering only the `}` character.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set x foo}bar\n");
    assert_eq!(range_of(&diags, "E102"), (0, 9, 0, 10));
}

#[test]
fn e100_repair_does_not_corrupt_unrelated_command_name() {
    // A stray `]` after a call to an already-declared user proc must not
    // be "repaired" into a virtual command-substitution token with a
    // byte-offset bug — that would corrupt the recorded invocation and
    // fire a phantom "Unknown command" (W123) on a garbled substring of
    // the proc name, with no E100 fix to explain it either.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc myHelper {a b} {return $a}\nset y myHelper arg1 arg2]\n",
    );
    assert!(has_code(&diags, "E100"));
    assert!(
        !has_code(&diags, "W123"),
        "no phantom unknown-command diagnostic expected: {:?}",
        diags
            .iter()
            .map(|d| (code_str(d), message(d).to_string()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn e100_escaped_bracket_under_arity_overflow_is_silent() {
    // A genuinely escaped trailing `]` (`\]`) combined with an arity
    // overflow on the enclosing command must not fire E100 at all, and
    // must not trigger a repair that corrupts anything downstream.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(&uri, "set y bar baz\\]\n");
    assert!(!has_code(&diags, "E100"), "{:?}", codes(&diags));
    assert!(!has_code(&diags, "W123"), "{:?}", codes(&diags));
}

#[test]
fn e101_recovery_does_not_swallow_a_call_to_a_known_proc() {
    // Regression: only registry builtins were excluded from looking
    // like an orphaned switch case, so a genuine call to an
    // already-declared user proc with a single braced argument right
    // after the case list — `renderReport { prose text }` — was
    // swallowed as an extra case, corrupting the switch's argv and
    // running the braced prose through command analysis as if it were
    // Tcl (a phantom "Unknown command" on ordinary text).
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc renderReport {body} {\n    puts $body\n}\n\nproc foo {x} {\n    switch $x\n    a {\n        return 1\n    }\n    renderReport {\n        Some unrelated braced-argument call, not a switch case.\n    }\n}\n",
    );
    assert!(has_code(&diags, "E101"), "{:?}", codes(&diags));
    assert!(
        !has_code(&diags, "W123"),
        "the renderReport call must not be parsed as switch-case body text: {:?}",
        diags
            .iter()
            .map(|d| (code_str(d), message(d).to_string()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn e103_abstains_when_missing_brace_swallows_more_than_one_statement() {
    // The "stolen close brace" heuristic must not fire on whichever `}` is
    // LAST in the swallowed text when that text spans more than one
    // top-level statement (here a sibling `proc` swallowed along with the
    // `if` that actually stole the brace) — picking that brace would parse
    // clean but silently nest the sibling proc inside the unclosed one
    // instead of closing it where the missing brace belongs, a structural
    // corruption, not just an imprecise diagnostic. Pure brace-counting
    // can't safely pick a location once more than one statement is
    // swallowed, so this must fall back to the generic (fix-less) E200
    // instead of guessing wrong.
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let diags = lsp.open_ready(
        &uri,
        "proc foo {} {\n    if {1} {\n        puts hi\n    }\nproc bar {} {\n    return 1\n}\n",
    );
    assert!(!has_code(&diags, "E103"), "{:?}", codes(&diags));
    assert!(has_code(&diags, "E200"), "{:?}", codes(&diags));
}
