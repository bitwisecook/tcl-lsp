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

//! E004: malformed `if` shapes, the well-formed ones it leaves alone and its fixes.

use super::*;

// -- E004 malformed-if emitter
//
// Fires ``Severity::Error`` when an ``if`` invocation's structural
// shape doesn't match
// ``if COND BODY ?elseif COND BODY ...? ?else BODY?``.  Detection
// reads the registry's walk of `if`'s clause grammar through
// `CommandRegistry::clause_shape_defect` — the grammar itself is not
// reimplemented here (see `emit_e004_clause_shape_diagnostic`).
// Every case is cross-checked against tclsh 8.6 and Tcl 9.0.4's
// `TclNRIfObjCmd` source; see the truth table in
// `tcl-registry/src/commands/tcl/if_.rs`'s own tests for the
// grammar-level cases this integration layer doesn't repeat.

fn e004_diags(src: &str) -> Vec<crate::analyser::types::Diagnostic> {
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    r.diagnostics
        .into_iter()
        .filter(|d| d.code == DiagCode::E004)
        .collect()
}

fn span_text(src: &str, span: tcl_lexer::Span) -> &str {
    &src[span.start() as usize..span.end() as usize]
}

// -- TP: genuinely malformed shapes, with the precise message and a
// tight span (not the whole `if` statement).

#[test]
fn tp_bare_if_names_the_invoked_command_in_the_message() {
    // ``if`` alone — no expression at all.  Message and span both
    // name "if" itself (real Tcl: `no expression after "if"
    // argument`), and this must be the *only* diagnostic for the
    // line — no redundant generic arity error alongside it (see
    // `no_duplicate_e002_alongside_e004`).
    let src = "if\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No expression after \"if\" argument");
    assert_eq!(span_text(src, e004[0].span), "if");
}

#[test]
fn tp_condition_without_body_names_the_condition_text() {
    // ``if {1}`` — real Tcl: `no script following "1" argument`.
    let src = "if {1}\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No script following \"1\" argument");
    assert_eq!(span_text(src, e004[0].span), "{1}");
}

#[test]
fn tp_then_keyword_without_body_names_then() {
    // ``if {1} then`` — real Tcl: `no script following "then"
    // argument`.
    let src = "if {1} then\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No script following \"then\" argument");
    assert_eq!(span_text(src, e004[0].span), "then");
}

#[test]
fn tp_bare_else_without_body_names_else() {
    // ``if {1} { a } else`` — real Tcl: `no script following
    // "else" argument`.
    let src = "if {1} { a } else\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No script following \"else\" argument");
    assert_eq!(span_text(src, e004[0].span), "else");
}

#[test]
fn tp_elseif_without_expr_names_elseif() {
    // ``if {1} { a } elseif`` — real Tcl: `no expression after
    // "elseif" argument`.
    let src = "if {1} { a } elseif\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No expression after \"elseif\" argument");
    assert_eq!(span_text(src, e004[0].span), "elseif");
}

#[test]
fn tp_elseif_condition_without_body_names_the_condition_text() {
    // ``if {1} { a } elseif {2}`` — real Tcl: `no script
    // following "2" argument`.
    let src = "if {1} { a } elseif {2}\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].message, "No script following \"2\" argument");
    assert_eq!(span_text(src, e004[0].span), "{2}");
}

#[test]
fn tp_extra_words_after_explicit_else_anchors_only_the_extra_words() {
    // ``if {1} { a } else { b } extra`` — the span covers just
    // "extra", not the whole statement.
    let src = "if {1} { a } else { b } extra\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(
        e004[0].message,
        "Extra words after \"else\" clause in \"if\" command"
    );
    assert_eq!(span_text(src, e004[0].span), "extra");
    assert!(matches!(e004[0].severity, crate::analyser::Severity::Error));
}

#[test]
fn tp_extra_words_after_implicit_else_anchors_only_the_extra_word() {
    // ``if {1} { a } { b } { c }`` — implicit else (no ``else``
    // keyword); "c" is the first extra word.
    let src = "if {1} { a } { b } { c }\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(span_text(src, e004[0].span), "{ c }");
}

#[test]
fn tp_qualified_double_colon_if_is_checked_too() {
    // ``::if`` names the same global command as ``if`` — registry
    // name resolution strips the leading ``::`` for every command,
    // so the dispatch-generic hook lookup picks this up without any
    // `if`-specific handling in the compiler.
    let src = "::if {1} { a } { b } { c }\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
}

// -- FP fixed: a leading `else`/`elseif` is a well-formed `if` whose
// condition happens to be that bareword — not a malformed `if`.
// Verified against tclsh 8.6: both raise `invalid bareword "else"` /
// `"elseif"` at *expression*-evaluation time, never a `wrong # args`
// structural error.

#[test]
fn fp_leading_else_is_not_malformed() {
    let r_diags = e004_diags("if else { x }\n");
    assert_eq!(r_diags.len(), 0, "got {r_diags:?}");
}

#[test]
fn fp_leading_elseif_is_not_malformed() {
    let r_diags = e004_diags("if elseif { x }\n");
    assert_eq!(r_diags.len(), 0, "got {r_diags:?}");
}

#[test]
fn fp_else_in_elseif_condition_slot_is_not_malformed() {
    // ``if {1} { a } elseif else { b }`` — "else" sits in the
    // *elseif's* condition slot, never keyword-matched there
    // either (verified against tclsh 8.6: runs body "a", not a
    // wrong-#args error).
    let r_diags = e004_diags("if {1} { a } elseif else { b }\n");
    assert_eq!(r_diags.len(), 0, "got {r_diags:?}");
}

// -- TN: well-formed shapes never flagged.

#[test]
fn tn_single_clause_if() {
    assert_eq!(
        e004_diags("if {1} { a }\n"),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn tn_if_else() {
    assert_eq!(
        e004_diags("if {1} { a } else { b }\n"),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn tn_if_elseif_else_chain() {
    assert_eq!(
        e004_diags("if {$a} { x } elseif {$b} { y } else { z }\n"),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn tn_if_with_then_keyword() {
    assert_eq!(
        e004_diags("if {1} then { a }\n"),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn tn_implicit_else_single_body() {
    // ``if {1} { a } { b }`` — one implicit-else body, no keyword.
    assert_eq!(
        e004_diags("if {1} { a } { b }\n"),
        [] as [crate::analyser::types::Diagnostic; 0]
    );
}

#[test]
fn tn_inside_tcloo_method_body() {
    // `if` structural checking is generic body-walking — it must
    // fire the same way inside a TclOO method body as at top level.
    let src = "oo::class create C {\n  method m {} {\n    if {1} { a } { b } { c }\n  }\n}\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
}

// Original move lineage retains the selected clause grammar.
#[test]
fn renamed_if_keeps_its_original_clause_shape() {
    // naming.diagnostic.original-control-advice
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-control-advice.md
    let src = "rename if myif\nmyif {1} { a } { b } { c }\n";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
}

// -- Redundant-diagnostic fix: `if`'s registry `arity` floor no
// longer produces a second, generic E002 alongside the precise
// E004 for the same defect.

#[test]
fn no_duplicate_e002_alongside_e004() {
    for src in ["if\n", "if {1}\n"] {
        let mut a = Analyser::new();
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::E002),
            "src={src:?} got {:?}",
            r.diagnostics
        );
        assert!(
            r.diagnostics.iter().any(|d| d.code == DiagCode::E004),
            "src={src:?} got {:?}",
            r.diagnostics
        );
    }
}

// -- Code fixes.

fn apply_fix(src: &str, fix: &crate::analyser::types::CodeFix) -> String {
    let mut fixed = src.to_string();
    fixed.replace_range(
        fix.span.start() as usize..fix.span.end() as usize,
        &fix.new_text,
    );
    fixed
}

#[test]
fn fix_merges_extra_words_into_the_final_body() {
    let src = "if {1} { a } { b } { c }";
    let e004 = e004_diags(src);
    assert_eq!(e004.len(), 1, "got {e004:?}");
    assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
    let fixed = apply_fix(src, &e004[0].fixes[0]);
    assert_eq!(fixed, "if {1} { a } {{ b } { c }}");
    // The fixed source must itself be shape-well-formed.
    assert_eq!(e004_diags(&fixed).len(), 0, "fixed={fixed:?}");
}

#[test]
fn fix_merges_extra_words_after_explicit_else() {
    let src = "if {1} { a } else { b } extra";
    let e004 = e004_diags(src);
    assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
    let fixed = apply_fix(src, &e004[0].fixes[0]);
    assert_eq!(fixed, "if {1} { a } else {{ b } extra}");
    assert_eq!(e004_diags(&fixed).len(), 0, "fixed={fixed:?}");
}

#[test]
fn fix_removes_dangling_elseif_clause() {
    let src = "if {1} { a } elseif";
    let e004 = e004_diags(src);
    assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
    assert_eq!(e004[0].fixes[0].new_text, "");
    let fixed = apply_fix(src, &e004[0].fixes[0]);
    assert_eq!(fixed, "if {1} { a } ");
    assert_eq!(e004_diags(&fixed).len(), 0, "fixed={fixed:?}");
}

#[test]
fn fix_removes_dangling_else_with_no_body() {
    let src = "if {1} { a } else";
    let e004 = e004_diags(src);
    assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
    let fixed = apply_fix(src, &e004[0].fixes[0]);
    assert_eq!(fixed, "if {1} { a } ");
    assert_eq!(e004_diags(&fixed).len(), 0, "fixed={fixed:?}");
}

#[test]
fn fix_removes_dangling_elseif_condition_without_body() {
    let src = "if {1} { a } elseif {2}";
    let e004 = e004_diags(src);
    assert_eq!(e004[0].fixes.len(), 1, "got {:?}", e004[0].fixes);
    let fixed = apply_fix(src, &e004[0].fixes[0]);
    assert_eq!(fixed, "if {1} { a } ");
    assert_eq!(e004_diags(&fixed).len(), 0, "fixed={fixed:?}");
}

#[test]
fn no_fix_offered_when_the_first_clause_never_completed() {
    // ``if`` / ``if {1}`` — there is no well-formed prefix to fall
    // back to, so no fix is offered (never a guessed body).
    for src in ["if", "if {1}", "if {1} then"] {
        let e004 = e004_diags(src);
        assert_eq!(e004.len(), 1, "src={src:?}");
        assert_eq!(
            e004[0].fixes.len(),
            0,
            "src={src:?} got {:?}",
            e004[0].fixes
        );
    }
}
