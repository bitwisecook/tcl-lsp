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

//! W003: `expr` operators the active dialect does not have, their tight spans and portable rewrites.

use super::*;

#[test]
fn w003_fires_on_string_compare_in_tcl84() {
    // `lt` / `le` / `gt` / `ge` are Tcl 9.0+ (TIP 461); on
    // Tcl 8.4 / 8.5 / 8.6 they should produce W003.
    let mut a = Analyser::new();
    let result = a.analyse("if {$x lt $y} { puts hi }", "tcl8.4");
    let w003: Vec<&Diagnostic> = result
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W003)
        .collect();
    assert!(
        !w003.is_empty(),
        "expected W003 on tcl8.4 'lt' operator, got {:?}",
        result.diagnostics
    );
    assert!(w003[0].message.contains("'lt'"));
}

#[test]
fn w003_silent_on_string_compare_in_tcl9() {
    let mut a = Analyser::new();
    let result = a.analyse("if {$x lt $y} { puts hi }", "tcl9.0");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W003),
        "W003 should not fire on tcl9.0, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w003_fires_on_in_operator_in_tcl84() {
    // `in` / `ni` are Tcl 8.5+ (TIP 201).
    let mut a = Analyser::new();
    let result = a.analyse("if {$x in {a b c}} { puts hi }", "tcl8.4");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W003),
        "expected W003 on tcl8.4 'in' operator, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w003_fires_on_exponentiation_operator_in_tcl84() {
    // `**` (exponentiation) is Tcl 8.5+ (TIP 123) — a symbolic operator a
    // word-shaped gated set misses entirely.
    let has_w003 = |src: &str, d: &str| {
        Analyser::new()
            .analyse(src, d)
            .diagnostics
            .iter()
            .any(|x| x.code == DiagCode::W003)
    };
    assert!(has_w003("expr {2 ** 3}", "tcl8.4"));
    // Available from 8.5 onward — silent.
    assert!(!has_w003("expr {2 ** 3}", "tcl8.6"));
    assert!(!has_w003("expr {2 ** 3}", "tcl9.0"));
    // Message cites the operator and its TIP.
    let d = Analyser::new().analyse("expr {2 ** 3}", "tcl8.4");
    assert!(
        d.diagnostics.iter().any(|x| x.code == DiagCode::W003
            && x.message.contains("'**'")
            && x.message.contains("TIP 123")),
        "message must cite `**` + TIP 123; got {:?}",
        d.diagnostics,
    );
}

#[test]
fn w003_fires_on_tab_separated_operator() {
    // The prefilter must tolerate any
    // whitespace, not just literal spaces.  `if {$x\tlt\t$y}` is
    // valid Tcl 8.4 syntax that the expr parser handles — the
    // analyser must not skip it because we only checked for
    // space-delimited operators.
    let mut a = Analyser::new();
    let result = a.analyse("if {$x\tlt\t$y} { puts hi }", "tcl8.4");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W003),
        "W003 must fire on tab-separated 'lt', got {:?}",
        result.diagnostics
    );
}

#[test]
fn w003_fires_on_newline_separated_operator() {
    // Same shape with a newline boundary — also valid Tcl.
    let mut a = Analyser::new();
    let result = a.analyse("if {$x\nin\n{a b c}} { puts hi }", "tcl8.4");
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W003),
        "W003 must fire on newline-separated 'in', got {:?}",
        result.diagnostics
    );
}

#[test]
fn contains_gated_word_handles_boundaries() {
    // No false positives on identifiers that contain the keyword.
    assert!(!contains_gated_word("$alt"));
    assert!(!contains_gated_word("$align"));
    assert!(!contains_gated_word("inner"));
    assert!(!contains_gated_word("$gem"));
    // Real matches at word boundaries.
    assert!(contains_gated_word("$x lt $y"));
    assert!(contains_gated_word("$x\tlt\t$y"));
    assert!(contains_gated_word("($x)lt($y)"));
    assert!(contains_gated_word("lt $y"));
    assert!(contains_gated_word("$x lt"));
}

#[test]
fn w003_silent_on_in_operator_in_tcl85() {
    let mut a = Analyser::new();
    let result = a.analyse("if {$x in {a b c}} { puts hi }", "tcl8.5");
    assert!(
        !result.diagnostics.iter().any(|d| d.code == DiagCode::W003),
        "W003 should not fire on tcl8.5, got {:?}",
        result.diagnostics
    );
}

#[test]
fn w003_tight_span_covers_only_the_operator_in_a_braced_if() {
    // The whole condition is `$x lt $y` (11 chars); the diagnostic must
    // highlight just the 2-byte `lt`, not the condition or the `if`.
    let hits = w003_hits("if {$x lt $y} { puts hi }", "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "lt");
}

#[test]
fn w003_tight_span_covers_only_the_operator_in_bare_expr() {
    let hits = w003_hits("expr {2 in {1 2 3}}", "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "in");
}

#[test]
fn w003_distinct_operators_each_get_their_own_tight_span() {
    // Two *different* gated operators in one expression must not collapse
    // onto one coarse diagnostic covering the whole condition; each gets its
    // own diagnostic at its own span.
    let hits = w003_hits("if {$a lt $b && $c in $d} { puts hi }", "tcl8.4");
    let mut texts: Vec<&str> = hits.iter().map(|(t, _)| t.as_str()).collect();
    texts.sort_unstable();
    assert_eq!(texts, vec!["in", "lt"], "{hits:?}");
    // The two spans must not overlap.
    assert_ne!(hits[0].1.span, hits[1].1.span);
}

#[test]
fn w003_repeated_same_operator_gets_a_diagnostic_per_occurrence() {
    let hits = w003_hits("if {$a in $b && $c in $d} { puts hi }", "tcl8.4");
    assert_eq!(hits.len(), 2, "{hits:?}");
    assert_eq!(hits[0].0, "in");
    assert_eq!(hits[1].0, "in");
    // Same text, but must anchor at two different source positions.
    assert_ne!(hits[0].1.span.start(), hits[1].1.span.start());
}

#[test]
fn w003_message_cites_the_relevant_tip() {
    let hits = w003_hits("expr {2 in {1 2 3}}", "tcl8.4");
    assert_eq!(hits.len(), 1);
    assert!(
        hits[0].1.message.contains("TIP 201"),
        "{}",
        hits[0].1.message
    );
    let hits = w003_hits("if {$x lt $y} { puts hi }", "tcl8.4");
    assert_eq!(hits.len(), 1);
    assert!(
        hits[0].1.message.contains("TIP 461"),
        "{}",
        hits[0].1.message
    );
}

#[test]
fn w003_fires_on_all_six_gated_operators_pre_availability() {
    // `ni`/`le`/`gt`/`ge` were only ever exercised indirectly before
    // (only `lt` and `in` had a dedicated test); cover the full set.
    for (src, op) in [
        ("if {$x ni {a b c}} { puts hi }", "ni"),
        ("if {$x le $y} { puts hi }", "le"),
        ("if {$x gt $y} { puts hi }", "gt"),
        ("if {$x ge $y} { puts hi }", "ge"),
    ] {
        let hits = w003_hits(src, "tcl8.4");
        assert_eq!(hits.len(), 1, "{src}: {hits:?}");
        assert_eq!(hits[0].0, op, "{src}");
    }
}

#[test]
fn w003_silent_on_ni_le_gt_ge_when_dialect_supports_them() {
    // `ni` only needs 8.5+; `le`/`gt`/`ge` need 9.0+.
    assert_eq!(
        w003_hits("if {$x ni {a b c}} { puts hi }", "tcl8.5"),
        [] as [(std::string::String, crate::analyser::types::Diagnostic); 0]
    );
    for src in [
        "if {$x le $y} { puts hi }",
        "if {$x gt $y} { puts hi }",
        "if {$x ge $y} { puts hi }",
    ] {
        assert_eq!(w003_hits(src, "tcl9.0").len(), 0, "{src}");
        assert!(w003_hits(src, "tcl8.5").len() == 1, "{src}");
    }
}

#[test]
pub(in super::super) fn w003_fires_on_unbraced_multiword_expr_at_a_tight_span() {
    // `expr` is the only EXPR-role command that accepts an unbraced,
    // multi-word expression; the gated keyword is its own Tcl word, so
    // this exercises the separate `emit_w003_dialect_invalid_expr_words`
    // path rather than the single-argument one above.
    let hits = w003_hits("expr $a in $b", "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "in");
    // No fix is offered for this shape (see doc comment on the emitter).
    assert_eq!(hits[0].1.fixes, [] as [crate::irules_checks::CodeFix; 0]);
}

#[test]
fn w003_offers_lsearch_fix_for_in() {
    let hits = w003_hits("expr {2 in {1 2 3}}", "tcl8.4");
    assert_eq!(hits.len(), 1);
    let fixes = &hits[0].1.fixes;
    assert_eq!(fixes.len(), 1, "{fixes:?}");
    assert_eq!(fixes[0].new_text, "([lsearch -exact {1 2 3} 2] >= 0)");
}

#[test]
fn w003_offers_lsearch_fix_for_ni() {
    let hits = w003_hits("expr {2 ni {1 2 3}}", "tcl8.4");
    assert_eq!(hits.len(), 1);
    let fixes = &hits[0].1.fixes;
    assert_eq!(fixes.len(), 1, "{fixes:?}");
    assert_eq!(fixes[0].new_text, "([lsearch -exact {1 2 3} 2] < 0)");
}

#[test]
fn w003_offers_string_compare_fix_for_string_relational_ops() {
    for (src, expect) in [
        ("if {$x lt $y} { puts hi }", "([string compare $x $y] < 0)"),
        ("if {$x le $y} { puts hi }", "([string compare $x $y] <= 0)"),
        ("if {$x gt $y} { puts hi }", "([string compare $x $y] > 0)"),
        ("if {$x ge $y} { puts hi }", "([string compare $x $y] >= 0)"),
    ] {
        let hits = w003_hits(src, "tcl8.4");
        assert_eq!(hits.len(), 1, "{src}");
        let fixes = &hits[0].1.fixes;
        assert_eq!(fixes.len(), 1, "{src}: {fixes:?}");
        assert_eq!(fixes[0].new_text, expect, "{src}");
    }
}

#[test]
fn w003_fix_span_replaces_exactly_the_gated_application() {
    let src = "expr {2 in {1 2 3}}";
    let hits = w003_hits(src, "tcl8.4");
    let fix = &hits[0].1.fixes[0];
    assert_eq!(
        &src[fix.span.start() as usize..fix.span.end() as usize],
        "2 in {1 2 3}"
    );
}

#[test]
fn w003_no_fix_when_operator_nested_in_a_larger_expression() {
    // Only `in` is gated here (`&&` is fine everywhere); it is the sole
    // W003 occurrence, but it is not the *whole* condition — rewriting
    // just the `in` sub-expression while leaving `&& $c` around it is
    // exactly the "nested" shape the fix deliberately declines.
    let hits = w003_hits("if {$a in $b && $c} { puts hi }", "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].1.fixes, [] as [crate::irules_checks::CodeFix; 0]);
}

#[test]
fn w003_no_fix_when_more_than_one_occurrence() {
    let hits = w003_hits("if {$a lt $b && $c in $d} { puts hi }", "tcl8.4");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].1.fixes, [] as [crate::irules_checks::CodeFix; 0]);
    assert_eq!(hits[1].1.fixes, [] as [crate::irules_checks::CodeFix; 0]);
}

#[test]
fn w003_no_fix_when_operand_is_a_call() {
    // `max($a, $b)` contains an unprotected space after the comma —
    // splicing its rendered text bare into `lsearch`'s argument list
    // would silently mis-word-split, so `is_simple_operand` excludes
    // `Call` and no fix is offered even though this is the sole,
    // top-level occurrence.
    let hits = w003_hits("expr {max($a, $b) in $list}", "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].1.fixes, [] as [crate::irules_checks::CodeFix; 0]);
}

#[test]
fn w003_silent_on_variable_named_like_a_gated_operator() {
    // `$in` is a variable reference, not the `in` operator — the
    // lexical prefilter alone can't tell the two apart, but the real
    // expr parse must.
    assert_eq!(
        w003_hits("if {$in} { puts hi }", "tcl8.4"),
        [] as [(std::string::String, crate::analyser::types::Diagnostic); 0]
    );
    assert_eq!(
        w003_hits("if {$ni && $lt} { puts hi }", "tcl8.4"),
        [] as [(std::string::String, crate::analyser::types::Diagnostic); 0]
    );
}

#[test]
fn w003_silent_on_array_element_named_like_a_gated_operator() {
    assert_eq!(
        w003_hits("if {$arr(in)} { puts hi }", "tcl8.4"),
        [] as [(std::string::String, crate::analyser::types::Diagnostic); 0]
    );
}

#[test]
fn w003_silent_on_quoted_string_literal_operator_word() {
    // `"in"` is a quoted string literal, not the bareword operator.
    let hits = w003_hits(r#"if {"in" eq $x} { puts hi }"#, "tcl8.4");
    assert_eq!(hits.len(), 0, "{hits:?}");
}

#[test]
fn w003_silent_on_malformed_expression() {
    // `lt` with no right-hand operand doesn't parse — `parse_expr`
    // falls back to `Raw`, so W003 must stay silent rather than guess.
    assert_eq!(
        w003_hits("if {$x lt} { puts hi }", "tcl8.4"),
        [] as [(std::string::String, crate::analyser::types::Diagnostic); 0]
    );
}

#[test]
fn w003_dialect_is_file_wide_regardless_of_namespace_or_proc_nesting() {
    // The active dialect is resolved once per file/analysis, not
    // per-scope — a gated operator inside a deeply nested proc body
    // must still be flagged.
    let src = "namespace eval ::foo {\n  proc bar {} {\n    if {$x in $y} { return 1 }\n  }\n}\n";
    let hits = w003_hits(src, "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "in");
}

#[test]
fn w003_fires_inside_a_tcl_oo_method_body() {
    // 8.4 has no TclOO, so the gated operator is 9.0's `lt` under 8.6:
    // tclsh 8.6 calling the method fails `invalid bareword "lt"`.
    let src = "oo::class create Foo {\n  method bar {} {\n    if {$x lt $y} { return 1 }\n  }\n}\n";
    let hits = w003_hits(src, "tcl8.6");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert_eq!(hits[0].0, "lt");
}

#[test]
fn w003_fires_inside_a_sub_interpreter_eval_body() {
    // `interp eval`'s script argument is a registry BODY role, so it is
    // walked like any other nested script. Static analysis can't know
    // the sub-interpreter's own Tcl version, so it reasonably applies
    // the enclosing file's dialect uniformly.
    // tclsh 8.4.20: with the interpreter created, the body raises `syntax
    // error in expression "$x in $y"`; unset, `$safeInterp` raises first.
    let src = "set safeInterp [interp create]\ninterp eval $safeInterp {\n  if {$x in $y} { return 1 }\n}\n";
    let hits = w003_hits(src, "tcl8.4");
    assert_eq!(hits.len(), 1, "{hits:?}");
}

#[test]
fn w003_suppressed_by_an_earlier_proc_shadowing_if() {
    // A user `proc if {...} {...}` defined *before* the call site
    // resolves at the call site instead of the builtin `::if` — Tcl's
    // own name resolution, mirrored by W002's existing shadow rule and
    // now shared by the EXPR-role dispatch (W100/W110/W003/W114 all at
    // once, via `dispatch_expr_arguments`'s shadow guard).
    let src = "proc if {c b} { return 1 }\nif {$x in $y} { puts hi }\n";
    assert_eq!(
        w003_hits(src, "tcl8.4").len(),
        0,
        "shadowed 'if' must suppress W003"
    );
}

#[test]
fn w003_not_suppressed_by_a_later_proc_shadowing_if() {
    // The shadowing proc is defined *after* this call site, so it
    // cannot have been in effect when Tcl resolved this particular
    // call — W003 must still fire here.
    let src = "if {$x in $y} { puts hi }\nproc if {c b} { return 1 }\n";
    assert_eq!(w003_hits(src, "tcl8.4").len(), 1);
}

#[test]
fn w003_correctly_gates_eda_vendor_dialects_by_documented_base_version() {
    // These vendor dialects are documented as running on top of a real
    // Tcl 8.5+ core (`docs/design/compiler/dialects-events.md`), so
    // `in`/`ni` (TIP 201, 8.5+) must NOT be flagged for them:
    // `DialectProfile::expr_grammar_base` decides, not a blanket
    // `SpecSurface::TCL85_PLUS` check. (`f5-iapps` is deliberately NOT here:
    // it rides
    // the `f5-tcl` trunk, a fork of Tcl at 8.4.6, and its measured expr
    // surface fails every 8.5 discriminator —
    // bigip-irule-parser-measurements.md §4a.)
    for dialect in [
        "xilinx-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
        "synopsys-eda-tcl",
        "expect",
    ] {
        assert_eq!(
            w003_hits("expr {2 in {1 2 3}}", dialect).len(),
            0,
            "{dialect} should support TIP 201 'in'"
        );
    }
    // None of them reach Tcl 9.0, so the string-relational operators
    // are still correctly flagged.
    for dialect in [
        "f5-iapps",
        "xilinx-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
        "synopsys-eda-tcl",
        "expect",
    ] {
        assert_eq!(
            w003_hits("if {$x lt $y} { puts hi }", dialect).len(),
            1,
            "{dialect} should still gate TIP 461 'lt'"
        );
    }
    // Cadence Innovus/Genus run an 8.4-safe core (owner decision), so —
    // like `f5-irules` — TIP 201 `in`/`ni` (8.5+) is *also* out of
    // grammar and must be flagged, not just TIP 461's `lt`/`le`/`gt`/`ge`.
    assert_eq!(
        w003_hits("expr {2 in {1 2 3}}", "cadence-eda-tcl").len(),
        1,
        "cadence-eda-tcl runs an 8.4 core — TIP 201 'in' must gate"
    );
    // So does the F5 trunk: iApps ride the fork of Tcl at 8.4.6
    // (measured, bigip-irule-parser-measurements.md §4a).
    assert_eq!(
        w003_hits("expr {2 in {1 2 3}}", "f5-iapps").len(),
        1,
        "f5-iapps rides the 8.4.6 trunk — TIP 201 'in' must gate"
    );
    assert_eq!(
        w003_hits("if {$x lt $y} { puts hi }", "cadence-eda-tcl").len(),
        1,
        "cadence-eda-tcl should still gate TIP 461 'lt'"
    );
}

#[test]
fn w003_f5_irules_stays_gated_on_its_tcl_8_4_runtime() {
    // iRules advertises an 8.6-shaped command *signature* but its
    // runtime `expr` evaluator is a genuine embedded Tcl 8.4.6 — both
    // TIPs must be flagged, unlike the 8.5-base vendor dialects above.
    assert_eq!(w003_hits("expr {2 in {1 2 3}}", "f5-irules").len(), 1);
    assert_eq!(w003_hits("if {$x lt $y} { puts hi }", "f5-irules").len(), 1);
}

#[test]
fn w003_f5_tmsh_gates_both_tip_201_and_tip_461() {
    // `f5-tmsh` rides the `f5-tcl` trunk — a fork of Tcl at 8.4.6
    // (measured, bigip-irule-parser-measurements.md §4a: a tmsh
    // `cli script` fails every 8.5 discriminator) — so TIP 201 `in`/`ni`
    // (8.5+) is out of grammar there exactly as TIP 461's
    // `lt`/`le`/`gt`/`ge` always were.
    assert_eq!(w003_hits("expr {2 in {1 2 3}}", "f5-tmsh").len(), 1);
    assert_eq!(w003_hits("if {$x lt $y} { puts hi }", "f5-tmsh").len(), 1);
}
