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

//! W105 unbraced bodies and W110 string comparison in expressions.

use super::*;

// -- W105 unbraced-body emitter

#[test]
fn w105_braces_the_complete_original_quoted_body() {
    use crate::analyser::{FixSafety, Severity};
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for (body, severity, replacement) in [
            (
                r#""set x \$""#,
                Severity::Warning,
                Some(("{set x $}", FixSafety::SemanticsEquivalent)),
            ),
            (
                r#""set x $who""#,
                Severity::Error,
                Some(("{set x $who}", FixSafety::BehaviourHardening)),
            ),
            (r#""set x }""#, Severity::Warning, None),
        ] {
            let source = format!("if {{1}} {body}");
            let result = Analyser::new().analyse(&source, dialect);
            let diagnostics: Vec<_> = result
                .diagnostics
                .iter()
                .filter(|d| d.code == DiagCode::W105)
                .collect();
            let [diagnostic] = diagnostics.as_slice() else {
                panic!("{dialect} {body}: {diagnostics:?}");
            };
            assert_eq!(&source[diagnostic.span.as_range()], body);
            assert_eq!(diagnostic.severity, severity);
            let fixes: Vec<_> = diagnostic
                .fixes
                .iter()
                .map(|fix| {
                    assert_eq!(fix.span, diagnostic.span);
                    (fix.new_text.as_str(), fix.safety)
                })
                .collect();
            assert_eq!(
                fixes,
                replacement.into_iter().collect::<Vec<_>>(),
                "{dialect} {body}"
            );
        }
    }
}

#[test]
fn analyse_skips_w105_for_braced_if_body() {
    // Braced ``{ ... }`` body — no W105.
    let mut a = Analyser::new();
    let r = a.analyse("if {$cond} { puts $x }\n", "tcl");
    assert!(!r.diagnostics.iter().any(|d| d.code == DiagCode::W105));
}

#[test]
fn analyse_skips_w105_for_command_substitution_body() {
    // A whole-word `[…]` command-substitution body is the safe
    // list-building idiom — `check_unbraced_body` exempts `Cmd`
    // tokens, so no W105.
    let mut a = Analyser::new();
    let r = a.analyse("eval [list set y $x]\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W105),
        "{:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_skips_w105_for_single_var_body() {
    // FP-STY-14: a body that is a *single bare variable* (`$body`,
    // `$cmd`, `$script`) is a script-valued reference — the variable
    // already holds the script — not an inline code block.  Bracing it
    // (`{$body}`) would turn the reference into the literal text, so the
    // W105 quick-fix is wrong and must not fire: `while {$cond} $body`
    // → no W105, as for `eval $cmd`, `proc $n $a $body`, `uplevel $script`.
    let mut a = Analyser::new();
    for src in [
        "while {$cond} $body\n",
        "eval $cmd\n",
        "after 0 $coroName\n",
        "proc $fakeName $arglist $body\n",
        "foreach name $nameList $body\n",
        "uplevel $script\n",
    ] {
        let r = a.analyse(src, "tcl");
        assert!(
            !r.diagnostics.iter().any(|d| d.code == DiagCode::W105),
            "W105 should be silent on single-var body {src:?}, got {:?}",
            r.diagnostics,
        );
    }
}

#[test]
fn analyse_emits_w105_for_quoted_interpolated_body() {
    // TP control: a *quoted* body with interpolation (`eval "do $script"`)
    // really is an inline script woven from substitutions — it can and
    // should be braced, so W105 still fires at ERROR severity.
    let mut a = Analyser::new();
    let r = a.analyse("eval \"do $script\"\n", "tcl");
    let w105: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W105)
        .collect();
    assert!(!w105.is_empty(), "expected W105, got {:?}", r.diagnostics);
    assert!(matches!(w105[0].severity, crate::analyser::Severity::Error));
}

// -- W110 string-compare-in-expr emitter

#[test]
fn expression_hints_require_the_complete_original_expression() {
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        for (space, comparison) in [(" ", true), ("\u{a0}", false)] {
            let source = format!("expr {{{space}\"abc\" == \"def\"}}");
            let result = Analyser::new().analyse(&source, dialect);
            assert_eq!(
                result.diagnostics.iter().any(|d| d.code == DiagCode::W110),
                comparison,
                "{dialect}: {source}"
            );
        }
    }
}

#[test]
fn analyse_emits_w110_for_string_eq_in_if_condition() {
    let mut a = Analyser::new();
    let r = a.analyse("if {$x == \"foo\"} {puts yes}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    assert!(w110[0].message.contains("eq"), "got {:?}", w110[0].message);
    assert!(matches!(w110[0].severity, crate::analyser::Severity::Hint));
}

#[test]
fn analyse_emits_w110_for_string_ne_in_if_condition() {
    let mut a = Analyser::new();
    let r = a.analyse("if {$x != \"bar\"} {puts no}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    assert!(w110[0].message.contains("ne"), "got {:?}", w110[0].message);
}

#[test]
fn w110_span_anchors_on_the_operator() {
    // Range precision: the diagnostic anchors on the `==` itself, not
    // the whole condition.
    let src = "if {$x == \"foo\"} {puts yes}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    let expected = src.find("==").unwrap();
    assert_eq!(
        (w110[0].span.start() as usize, w110[0].span.end() as usize),
        (expected, expected + 2),
        "W110 must cover exactly the operator, got {:?}",
        w110[0].span
    );
    // The fix rewrites the operator in place and leaves the braces and
    // operands as written.
    assert_eq!(w110[0].fixes.len(), 1, "fix expected: {:?}", w110[0]);
    let fix = &w110[0].fixes[0];
    assert_eq!(
        fix.span, w110[0].span,
        "the fix covers exactly the operator"
    );
    assert_eq!(fix.new_text, "eq");
}

#[test]
fn w110_span_anchors_on_the_matched_operator_not_the_first() {
    // `$a == $b` (variable compare — not flagged) precedes the
    // string compare `$c == "x"`; the anchor must be the SECOND `==`.
    let src = "if {$a == $b && $c == \"x\"} {puts yes}\n";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    let expected = src.rfind("==").unwrap();
    assert_eq!(
        (w110[0].span.start() as usize, w110[0].span.end() as usize),
        (expected, expected + 2),
        "W110 must anchor the matched (second) `==`, got {:?}",
        w110[0].span
    );
    // The fix rewrites only the proven operator; the variable compare,
    // which may be numeric, keeps its `==`.
    assert_eq!(w110[0].fixes.len(), 1, "fix expected: {:?}", w110[0]);
    let fix = &w110[0].fixes[0];
    let (s, e) = (fix.span.start() as usize, fix.span.end() as usize);
    let applied = format!("{}{}{}", &src[..s], fix.new_text, &src[e..]);
    assert_eq!(applied, "if {$a == $b && $c eq \"x\"} {puts yes}\n");
}

#[test]
fn analyse_no_w110_for_numeric_compare() {
    // ``$x == 42`` — numeric literal on the right, no string
    // operand, should not fire.
    let mut a = Analyser::new();
    let r = a.analyse("if {$x == 42} {puts yes}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w110_for_eq_operator() {
    // ``$x eq "foo"`` is the correct form — no W110.
    let mut a = Analyser::new();
    let r = a.analyse("if {$x eq \"foo\"} {puts yes}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w110_includes_eq_code_fix() {
    // Single ``==`` against a string literal — the blanket
    // rewrite should run and produce a fix containing ``eq``.
    let mut a = Analyser::new();
    let r = a.analyse("if {$x == \"foo\"} {puts yes}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    assert_eq!(w110[0].fixes.len(), 1, "got {:?}", w110[0].fixes);
    assert!(
        w110[0].fixes[0].new_text.contains("eq"),
        "got {:?}",
        w110[0].fixes[0].new_text
    );
}

#[test]
fn analyse_no_w110_for_variable_only_compare() {
    // ``$a == $b`` — both operands are variables, may hold
    // ints, no W110.
    let mut a = Analyser::new();
    let r = a.analyse("if {$a == $b} {puts yes}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_no_w110_for_numeric_string_literal() {
    // ``$x == "42"`` compares numerically when `x` is `42.0`, where `eq`
    // would not, so suggesting `eq` would change the result.
    let mut a = Analyser::new();
    let r = a.analyse("if {$x == \"42\"} {puts yes}\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w110_fires_for_boolean_string_literal() {
    // ``$x == "true"`` — `==` never reads a boolean word as a number, so
    // this compare is already a string compare.
    let mut a = Analyser::new();
    let r = a.analyse("if {$x == \"true\"} {puts yes}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
}

#[test]
fn analyse_w110_fires_through_unary_negation() {
    // ``!($x == "foo")`` — W110 walks through ExprUnary.
    let mut a = Analyser::new();
    let r = a.analyse("if {!($x == \"foo\")} {puts no}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
}

#[test]
fn analyse_w110_fix_leaves_an_unproven_compare_alone() {
    // ``$a == $b || $x == "foo"`` — only the second ``==`` is proven a
    // string compare, so the fix rewrites that one and keeps the first.
    let mut a = Analyser::new();
    let src = "if {$a == $b || $x == \"foo\"} {puts y}\n";
    let r = a.analyse(src, "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
    assert_eq!(w110[0].fixes.len(), 1, "got {:?}", w110[0].fixes);
    let fix = &w110[0].fixes[0];
    let (s, e) = (fix.span.start() as usize, fix.span.end() as usize);
    let applied = format!("{}{}{}", &src[..s], fix.new_text, &src[e..]);
    assert_eq!(applied, "if {$a == $b || $x eq \"foo\"} {puts y}\n");
}

#[test]
fn analyse_w110_fires_on_while_condition() {
    // ``while {EXPR} {body}`` — EXPR-role is at index 0.
    let mut a = Analyser::new();
    let r = a.analyse("while {$x == \"foo\"} { break }\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
}

#[test]
fn analyse_w110_fires_on_top_level_expr_command() {
    // ``expr {$x == "foo"}`` — top-level invocation of
    // ``expr`` exercises the EXPR-role dispatch on the
    // single braced arg.  (Nested ``[expr ...]`` command
    // substitutions are recorded as invocations but the
    // analyser doesn't currently re-enter them for per-
    // command checks; that's a separate concern.)
    let mut a = Analyser::new();
    let r = a.analyse("expr {$x == \"foo\"}\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
}

#[test]
fn analyse_w110_fires_on_multi_arg_expr_command() {
    // ``expr $x == "foo"`` (no braces, multiple argv slots): the args
    // are joined with spaces — the *substituted* word values, whose
    // quote delimiters Tcl's word splitting already stripped — so the
    // expression is ``$x == foo`` with ``foo`` a bareword, not an
    // ``ExprString``.  W110 (string ``==``) therefore does NOT fire on
    // the unbraced form (only W100 does); it fires on the *braced*
    // ``expr {$x == "foo"}`` where the string literal survives.
    let mut a = Analyser::new();
    let r = a.analyse("expr $x == \"foo\"\n", "tcl");
    let unbraced: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(
        unbraced.len(),
        0,
        "unbraced expr must not fire W110, got {:?}",
        r.diagnostics
    );

    let mut a2 = Analyser::new();
    let r2 = a2.analyse("set z [expr {$x == \"foo\"}]\n", "tcl");
    let braced: Vec<_> = r2
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(
        braced.len(),
        1,
        "braced expr must fire W110, got {:?}",
        r2.diagnostics
    );
}

#[test]
fn analyse_w110_no_fire_on_for_clean_condition() {
    // ``for {set i 0} {$i < 10} {incr i} {body}`` — no ``==``
    // anywhere, but ensure the EXPR-role dispatch on ``for``
    // doesn't crash and produces no W110.
    let mut a = Analyser::new();
    let r = a.analyse("for {set i 0} {$i < 10} {incr i} { break }\n", "tcl");
    assert!(
        !r.diagnostics.iter().any(|d| d.code == DiagCode::W110),
        "got {:?}",
        r.diagnostics
    );
}

#[test]
fn analyse_w110_fires_on_for_condition() {
    // ``for {set i 0} {$x == "foo"} {incr i} {body}`` —
    // the EXPR-role dispatch runs *before* the hook handlers,
    // so W110 on a ``for`` condition fires whichever walk
    // owns the bodies.
    let mut a = Analyser::new();
    let r = a.analyse("for {set i 0} {$x == \"foo\"} {incr i} { break }\n", "tcl");
    let w110: Vec<_> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == DiagCode::W110)
        .collect();
    assert_eq!(w110.len(), 1, "got {:?}", r.diagnostics);
}
