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

/// Reference-interpreter regressions (tclsh 8.6/9.0 verified): exact
/// bignum↔double comparison folds, the `**` base collapses for bignum
/// exponents, and NaN branch conditions declining.
#[test]
fn adversarial_fold_regressions() {
    // C compares a bignum and a double EXACTLY — never through the
    // bignum's rounded double view.
    assert_eq!(
        eval_str("18446744073709551617 == 1.8446744073709552e19"),
        Some(TclValue::Int(0)),
        "exact compare: 2^64+1 != its rounded double"
    );
    assert_eq!(
        eval_str("18446744073709551617 > 1.8446744073709552e19"),
        Some(TclValue::Int(1))
    );
    assert_eq!(eval_str("10**308 == 1e308"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("(2**1024) == inf"), Some(TclValue::Int(0)));
    // 0/±1 base collapses precede the negative-bignum-exponent rule.
    assert_eq!(
        eval_str("0**(-(2**64))"),
        None,
        "0 ** -big is a runtime domain error — never fold"
    );
    assert_eq!(eval_str("1**(-(2**64))"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("(-1)**(-(2**64))"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("(-1)**(-(2**64)-1)"), Some(TclValue::Int(-1)));
}

use super::*;
use crate::expr_parser::parse_expr;

/// tclsh 8.4.20–9.1.0: `ParseBoolean` matches a word untrimmed
/// (`if {"yes "} {}` is `expected boolean value but got "yes "`); a
/// number still skips its surrounding whitespace.
#[test]
fn an_integer_operand_reads_a_boolean_word_untrimmed() {
    let read = |text| parse_integer_operand_with_policy(text, FoldPolicy::default());
    assert_eq!(read("yes"), Some(TclValue::Int(1)));
    assert_eq!(read("yes "), None);
    assert_eq!(read(" true"), None);
    assert_eq!(read(" 7 "), Some(TclValue::Int(7)));
}

#[test]
fn original_boolean_value_advice_cannot_attest_a_retained_native_truth_stage() {
    // naming.numeric.original-primitive-boolean-vs-expression-truth
    // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
    use tcl_syntax::expr::ExprOps;
    use tcl_syntax::native_boolean_truth::NativeBooleanTruthPurpose as Purpose;
    let env = Env::new();
    let mut executable = make_fold_ops(
        &env,
        FoldPolicy::default(),
        None,
        None,
        FoldNativeInputs::objects(None),
        false,
    );
    let mut advice = make_fold_ops(
        &env,
        FoldPolicy::default(),
        None,
        None,
        FoldNativeInputs::objects(None),
        true,
    );
    let retained = FoldValue::RetainedNativeObject {
        value: Box::new(FoldValue::Int(4_294_967_296)),
        proof: None,
        integer_contents: None,
        reference: "$original".into(),
        start: Some(0),
    };
    assert!(
        advice
            .to_bool_for_purpose(&retained, Purpose::LogicalAnd)
            .unwrap()
    );
    assert!(
        advice
            .logical_right_truth(&retained, &FoldValue::Int(1), true)
            .unwrap()
    );
    assert!(
        advice
            .coercions
            .borrow()
            .iter()
            .any(|obligation| obligation.kind == NativeCoercionKind::Boolean)
    );
    assert!(
        executable
            .to_bool_for_purpose(&retained, Purpose::LogicalAnd)
            .is_err()
    );
    assert!(
        executable
            .logical_right_truth(&retained, &FoldValue::Int(1), true)
            .is_err()
    );
    assert!(
        executable
            .logical_right_truth(&FoldValue::Int(1), &retained, true)
            .is_err()
    );
    assert!(
        executable
            .to_bool_for_purpose(&FoldValue::Int(1), Purpose::ConditionalJump)
            .unwrap()
    );
    assert!(
        executable
            .to_bool_for_purpose(&FoldValue::Int(1), Purpose::ExpressionApiResult)
            .is_err()
    );
}

// evaluate_expr_with_constants

#[test]
fn evaluate_expr_integer() {
    let mut env = Env::new();
    env.insert("x".into(), EnvValue::Int(5));
    assert_eq!(value_86_env("$x + 3", &env), Some(TclValue::Int(8)));
}

#[test]
fn evaluate_expr_integer_valued_float() {
    assert_eq!(
        evaluate_expr_with_constants(
            &parse_expr("6.0 / 2", None),
            &Env::new(),
            FoldPolicy::default()
        ),
        Some(3)
    );
}

#[test]
fn evaluate_expr_fractional_float_none() {
    assert_eq!(
        evaluate_expr_with_constants(&parse_expr("1.5", None), &Env::new(), FoldPolicy::default()),
        None
    );
}

/// A string binding is decoded as Tcl reads a literal: a boolean word is
/// 1 or 0, an integer's text is the integer.
#[test]
fn evaluate_expr_decodes_a_string_binding() {
    let mut env = Env::new();
    env.insert("flag".into(), EnvValue::Str("true".into()));
    env.insert("n".into(), EnvValue::Str("42".into()));
    let value = |text: &str| value_86_env(text, &env);
    assert_eq!(value("$flag"), Some(TclValue::Int(1)));
    assert_eq!(value("$n + 1"), Some(TclValue::Int(43)));
}

#[test]
fn saturating_f64_to_i64_in_range_and_saturates() {
    // In-range integral floats convert exactly.
    assert_eq!(saturating_f64_to_i64(0.0), 0);
    assert_eq!(saturating_f64_to_i64(42.0), 42);
    assert_eq!(saturating_f64_to_i64(-42.0), -42);
    // `-0.0` normalises to 0 (tclsh `int(-0.0)` == 0), not "-0".
    assert_eq!(saturating_f64_to_i64(-0.0), 0);
    // Out-of-range magnitudes saturate by sign, as an `as i64` cast
    // does.
    assert_eq!(saturating_f64_to_i64(1e30), i64::MAX);
    assert_eq!(saturating_f64_to_i64(-1e30), i64::MIN);
}

/// The half-line an ordered comparison with an integer proves, on each
/// edge: the false edge reads the operator's inverse (`$x < 5` failing
/// puts an integer `x` at 5 or above), and a bound at the 64-bit edge
/// saturates, one value wide, rather than wrapping — `$x < MIN` puts `x`
/// at `MIN` or below on its true edge, `$x > MAX` at `MAX` or above.
#[test]
fn an_ordered_comparison_states_the_half_line_on_each_edge() {
    let range = |facts: &[PlaceFact]| match facts {
        [
            PlaceFact {
                fact: FactView::Domain(DomainFact::Range { lo, hi }),
                ..
            },
        ] => Some((*lo, *hi)),
        _ => None,
    };
    let policy = FoldPolicy::default();
    let five = ordering_facts(BinOp::Lt, "x", "5", policy);
    assert_eq!(range(&five.on_true), Some((None, Some(4))));
    assert_eq!(range(&five.on_false), Some((Some(5), None)));
    let at_most = ordering_facts(BinOp::Le, "x", "5", policy);
    assert_eq!(range(&at_most.on_false), Some((Some(6), None)));
    let min = ordering_facts(BinOp::Lt, "x", &i64::MIN.to_string(), policy);
    assert_eq!(range(&min.on_true), Some((None, Some(i64::MIN))));
    assert_eq!(range(&min.on_false), Some((Some(i64::MIN), None)));
    let max = ordering_facts(BinOp::Gt, "x", &i64::MAX.to_string(), policy);
    assert_eq!(range(&max.on_true), Some((Some(i64::MAX), None)));
    assert_eq!(range(&max.on_false), Some((None, Some(i64::MAX))));
    for word in ["true", "false", "2.5", "abc", "18446744073709551616"] {
        let none = ordering_facts(BinOp::Lt, "x", word, policy);
        assert_eq!(none, EdgeFacts::default(), "{word}");
    }
}

fn eval_str(expr: &str) -> Option<TclValue> {
    let env = Env::new();
    eval_tcl_expr(&parse_expr(expr, None), &env)
}

fn eval_str_env(expr: &str, env: &Env) -> Option<TclValue> {
    eval_tcl_expr(&parse_expr(expr, None), env)
}

/// Fold under Tcl 8.6. A comparison of strings is decided by the
/// release's character model, so it has no release-blind answer.
fn eval_86(expr: &str) -> Option<TclValue> {
    eval_release(expr, "tcl8.6")
}

fn eval_release(expr: &str, release: &str) -> Option<TclValue> {
    eval_tcl_expr_in_dialect(
        &parse_expr(expr, Some(release)),
        &Env::new(),
        tcl_registry::model::ingress::resolve_environment(release).analyser_profile(),
    )
}

/// The value an expression has over `env` under `policy`. A fold that
/// replaces the expression is refused while a variable read leaves a
/// conversion behind; the value is what these tests are about.
fn value_over(expr: &ExprNode, env: &Env, policy: FoldPolicy) -> Option<TclValue> {
    analyse_tcl_expr_with_resolved_math_bindings(expr, env, policy, &|_, _| None, None)
        .map(|evaluation| evaluation.value)
}

/// [`value_over`] under Tcl 8.6.
fn value_86_env(expr: &str, env: &Env) -> Option<TclValue> {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    value_over(
        &parse_expr(expr, Some("tcl8.6")),
        env,
        FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile)),
    )
}

#[test]
fn string_ordering_uses_selected_native_units_and_jim_equality_uses_bytes() {
    for (profile, ordered) in [
        ("tcl8.4", Some(0)),
        ("tcl8.5", Some(0)),
        ("tcl8.6", Some(0)),
        ("tcl9.0", Some(1)),
        ("tcl9.1", Some(1)),
        ("jim", None),
    ] {
        let supplied = tcl_registry::model::ingress::static_context_for(profile);
        let registry = supplied.commands();
        let selected = registry.profile().unwrap();
        let policy = FoldPolicy::for_retained_entry(
            registry,
            Some(tcl_registry::InvocationDialect::of_profile(selected)),
            &tcl_lexer::LexerConfig::from_grammar(selected.grammar),
        );
        for (expression, expected) in [
            ("\"\u{10000}\" > \"\u{e000}\"", ordered),
            ("\"\u{10000}\" eq \"\u{10000}\"", Some(1)),
            ("\"\u{10000}\" eq \"\u{e000}\"", Some(0)),
        ] {
            let tree = tcl_syntax::expr::parser::parse_expr_with_syntax_context(
                expression,
                &policy.preparation_context().unwrap(),
            );
            assert_eq!(
                eval_tcl_expr_with_policy(&tree, &Env::new(), policy),
                expected.map(TclValue::Int),
                "{profile}: {expression}"
            );
        }
    }
}

/// Parse + evaluate using the iRules dialect, which enables
/// `contains`/`starts_with`/`ends_with`/`equals`/`matches_glob`/
/// `matches_regex`/`in`/`ni` word operators. Must use the
/// dialect-threading evaluator, not the bare [`eval_tcl_expr`] — the
/// word operators parse under any dialect gate, but only actually
/// *fold* when [`FoldOps::f5_predicates`] is set, which only
/// [`eval_tcl_expr_in_dialect`] does.
fn eval_irules(expr: &str) -> Option<TclValue> {
    let env = Env::new();
    eval_tcl_expr_in_dialect(
        &parse_expr(expr, Some("f5-irules")),
        &env,
        tcl_dialect::DialectProfile::irules(),
    )
}

#[test]
fn math_functions_fold_only_from_their_introducing_release() {
    let env = Env::new();
    let fold = |expr: &str, dialect: &str| {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        // This test selects mathematical implementation grammar explicitly;
        // it supplies no executable command-table proof.
        eval_tcl_expr_with_policy(
            &parse_expr(expr, Some(dialect)),
            &env,
            FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile))
                .with_intrinsic_math(),
        )
    };
    // `min`/`max` are 8.5+: fold from 8.5, decline under 8.4.
    assert_eq!(fold("min(3, 1, 2)", "tcl8.6"), Some(TclValue::Int(1)));
    assert_eq!(fold("min(3, 1, 2)", "tcl8.4"), None);
    // The `is*` classification family is 9.0+.
    assert_eq!(fold("isinf(1.0)", "tcl9.0"), Some(TclValue::Int(0)));
    assert_eq!(fold("isinf(1.0)", "tcl8.6"), None);
    // An 8.4-era function folds everywhere.
    assert_eq!(fold("abs(-5)", "tcl8.4"), Some(TclValue::Int(5)));
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        assert_eq!(
            eval_tcl_expr_in_dialect(&parse_expr("abs(-5)", Some(dialect)), &env, profile),
            None,
            "a profile alone cannot establish a reached native function"
        );
    }
}

/// The old folder stays within the target's tower, as the route does:
/// a beyond-wide integer or an infinity, as an operand, a result or in
/// the middle, folds nothing under an 8.4 runtime or a profile naming
/// no release. tclsh 8.4: `1 << 70` is 0, `9223372036854775807 + 1`
/// wraps, `1e308 * 10` and `1e309` raise `floating-point value too large
/// to represent`, and `18446744073709551616 > 0` raises `integer value
/// too large to represent`; 8.5 to 9.1 answer
/// 1180591620717411303424, 9223372036854775808, `Inf`, `Inf` and 1.
#[test]
fn the_old_folder_stays_within_the_targets_tower() {
    let env = Env::new();
    let fold = |expr: &str, dialect: &str| {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        eval_tcl_expr_with_policy(
            &parse_expr(expr, Some(dialect)),
            &env,
            // The `abs` row is about the tower, not about which command
            // `abs` reaches, so the intrinsic table answers it.
            FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile))
                .with_intrinsic_math(),
        )
    };
    // The fixed-width model shifts a C integer, where a count at or past
    // the width is undefined: tclsh 8.4.20 here prints 0 for
    // `puts [expr {1 << 70}]`, a masked build 64, so the fold declines. A
    // profile naming no release has no model.
    assert_eq!(fold("5 >> 64", "tcl8.4"), None);
    assert_eq!(fold("1 << 62", "tcl8.4"), Some(TclValue::Int(1 << 62)));
    for expr in ["1 << 70", "(1 << 70) == 0", "-(1 << 70)"] {
        for dialect in ["tcl8.4", "f5-irules"] {
            assert_eq!(fold(expr, dialect), None, "{dialect}: {expr}");
        }
        assert_eq!(fold(expr, "f5-bigip"), None, "f5-bigip: {expr}");
        assert!(fold(expr, "tcl8.6").is_some(), "tcl8.6: {expr}");
    }
    for expr in [
        "9223372036854775807 + 1",
        "1e308 * 10",
        "1e308 * 10 > 0",
        "1e309 > 0",
        "18446744073709551616 > 0",
        "abs(1e308 * 10)",
        // A double spelling outside the range raises `ARITH OVERFLOW` /
        // `UNDERFLOW` under tclsh 8.4 (`4.9e-324` as "too large").
        "4.9e-324",
        "1e-400",
        "1e-400 * 1",
        // `i64::MIN % -1` is 0 from 8.5 and a `SIGFPE` that kills tclsh 8.4
        // and jimsh.
        "(-9223372036854775807 - 1) % -1",
    ] {
        for dialect in ["tcl8.4", "f5-irules", "f5-bigip"] {
            assert_eq!(fold(expr, dialect), None, "{dialect}: {expr}");
        }
        assert!(fold(expr, "tcl8.6").is_some(), "tcl8.6: {expr}");
    }
    assert_eq!(fold("1 << 62", "tcl8.4"), Some(TclValue::Int(1 << 62)));
    assert_eq!(fold("1e308 * 1", "tcl8.4"), Some(TclValue::Float(1e308)));
    assert_eq!(
        fold("2.2250738585072014e-308", "tcl8.4"),
        Some(TclValue::Float(f64::MIN_POSITIVE))
    );
    assert_eq!(fold("0e999", "tcl8.4"), Some(TclValue::Float(0.0)));
}

/// #2401: `abs(-9223372036854775808)` is `2^63`, which the folder's
/// wide/double math dispatch cannot carry. It declined by panicking
/// (`NoBig::from_i64`) under every dialect; it must decline quietly. The
/// releases disagree on the answer anyway: tclsh 8.5 to 9.1 give
/// `9223372036854775808`, 8.4 raises `ARITH IOVERFLOW`, and jimsh 0.84
/// wraps to `-9223372036854775808`.
/// jimsh 0.84's `pow` is its `**` operator: `pow(2,3)` is `8` and
/// `pow(2,-1)` is `0`, where tclsh 8.4.20 to 9.1.0 give `8.0` and `0.5`
/// (`mathfunc::dispatch_under`). A fold under Jim never gives Tcl's
/// double answer, and the Tcl table declines Jim's integer power.
#[test]
fn jims_integer_pow_never_folds_to_tcls_double() {
    let env = Env::new();
    let profile = tcl_registry::model::ingress::resolve_environment("jim").analyser_profile();
    let fold = |expr: &str| {
        eval_tcl_expr_with_policy(
            &parse_expr(expr, Some("jim")),
            &env,
            FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile)),
        )
    };
    assert!(matches!(fold("pow(2, 3)"), None | Some(TclValue::Int(8))));
    assert!(matches!(fold("pow(2, -1)"), None | Some(TclValue::Int(0))));
    let jim_grammar = FoldPolicy {
        numbers: Some(NumberSyntax::Jim),
        ..FoldPolicy::from_octal(Some(false))
    }
    .with_intrinsic_math();
    assert_eq!(
        eval_tcl_expr_with_policy(&parse_expr("pow(2, 3)", None), &env, jim_grammar),
        None
    );
}

#[test]
fn abs_of_the_most_negative_wide_declines_in_every_dialect() {
    let env = Env::new();
    for dialect in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "jim",
        "f5-irules",
        "f5-bigip",
    ] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        for expr in [
            "abs(-9223372036854775808)",
            "abs(-9223372036854775807 - 1)",
            "abs(abs(-9223372036854775808)) + 1",
            "int(abs(-9223372036854775808))",
            "max(1, abs(-0x8000000000000000))",
        ] {
            let folded = eval_tcl_expr_with_policy(
                &parse_expr(expr, Some(dialect)),
                &env,
                FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile)),
            );
            assert_eq!(folded, None, "{dialect}: {expr}");
        }
    }
    assert_eq!(eval_str("abs(-9223372036854775808)"), None);
    assert_eq!(
        eval_str("abs(-9223372036854775807)"),
        Some(TclValue::Int(i64::MAX))
    );
}

#[test]
fn literal_int() {
    assert_eq!(eval_str("42"), Some(TclValue::Int(42)));
    assert_eq!(eval_str("0"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("0x1a"), Some(TclValue::Int(26)));
    assert_eq!(eval_str("0b101"), Some(TclValue::Int(5)));
    assert_eq!(eval_str("0o17"), Some(TclValue::Int(15)));
}

#[test]
fn literal_float() {
    assert_eq!(eval_str("1.5"), Some(TclValue::Float(1.5)));
    assert_eq!(eval_str("2e3"), Some(TclValue::Float(2000.0)));
}

#[test]
fn const_fold_integers_match_tclsh() {
    // The const-folder drives O101 expr-folding, so a divergence from C Tcl
    // is a miscompile. Each (expr, value) pair is verified against
    // tclsh8.6/9.0 via `expr {<expr>}`:
    //   0x1a=26 0b101=5 0o17=15 7/2=3 7%3=1 2**10=1024 3*4+5=17
    //   (3+4)*2=14 10-3-2=5 5>3=1 5==5=1 5!=5=0 abs(-7)=7 max(3,9)=9
    //   min(3,9)=3 (1<<4)=16 (255&15)=15 (5|2)=7 (6^3)=5
    for (expr, want) in [
        ("0x1a", 26),
        ("0b101", 5),
        ("0o17", 15),
        ("7/2", 3),
        ("7%3", 1),
        ("2**10", 1024),
        ("3*4+5", 17),
        ("(3+4)*2", 14),
        ("10-3-2", 5),
        ("5>3", 1),
        ("5==5", 1),
        ("5!=5", 0),
        ("abs(-7)", 7),
        ("max(3,9)", 9),
        ("min(3,9)", 3),
        ("1<<4", 16),
        ("255&15", 15),
        ("5|2", 7),
        ("6^3", 5),
    ] {
        assert_eq!(eval_str(expr), Some(TclValue::Int(want)), "expr {{{expr}}}");
    }
}

#[test]
fn const_fold_floats_and_logicals_match_tclsh() {
    // tclsh-verified floats: 1.5+2.5=4.0, 10.0/4=2.5, 3.0*2=6.0.
    assert_eq!(eval_str("1.5+2.5"), Some(TclValue::Float(4.0)));
    assert_eq!(eval_str("10.0/4"), Some(TclValue::Float(2.5)));
    assert_eq!(eval_str("3.0*2"), Some(TclValue::Float(6.0)));
    // tclsh-verified comparisons / logicals → 0/1 integers:
    //   2.5>1.5=1, "abc" eq "abc"=1, "abc" eq "abd"=0, "a" ne "b"=1,
    //   1&&0=0, 1||0=1, !0=1.
    for (expr, want) in [
        ("2.5>1.5", 1),
        (r#""abc" eq "abc""#, 1),
        (r#""abc" eq "abd""#, 0),
        (r#""a" ne "b""#, 1),
        ("1 && 0", 0),
        ("1 || 0", 1),
        ("!0", 1),
    ] {
        assert_eq!(eval_86(expr), Some(TclValue::Int(want)), "expr {{{expr}}}");
    }
}

#[test]
fn const_fold_integer_division_floors_toward_negative_infinity() {
    // tclsh: integer `/` and `%` floor toward -inf (not truncate):
    //   -7/2 = -4 (not -3); -7%2 = 1; 7/-2 = -4; 7%-2 = -1.
    assert_eq!(eval_str("-7/2"), Some(TclValue::Int(-4)));
    assert_eq!(eval_str("-7%2"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("7/-2"), Some(TclValue::Int(-4)));
    assert_eq!(eval_str("7%-2"), Some(TclValue::Int(-1)));
}

#[test]
fn polymorphic_equality_uses_strict_numbers() {
    // `==` / `!=` compare numerically only when *both* operands are valid
    // Tcl numbers; otherwise as strings. Boolean words are NOT numbers for
    // comparison (tclsh: `expr {"true" == "1"}` → 0), so the comparison
    // must fall back to a string compare rather than coercing `true`→1.
    assert_eq!(eval_86(r#""true" == "1""#), Some(TclValue::Int(0)));
    assert_eq!(eval_86(r#""yes" == "1""#), Some(TclValue::Int(0)));
    assert_eq!(eval_86(r#""true" != "1""#), Some(TclValue::Int(1)));
    // Genuine numbers still compare numerically (incl. mixed int/float and
    // hex), and non-numeric strings string-compare.
    assert_eq!(eval_86(r#""5" == "5.0""#), Some(TclValue::Int(1)));
    assert_eq!(eval_86(r#""0x10" == "16""#), Some(TclValue::Int(1)));
    assert_eq!(eval_86(r#""foo" == "foo""#), Some(TclValue::Int(1)));
    assert_eq!(eval_86(r#""foo" == "bar""#), Some(TclValue::Int(0)));
}

#[test]
fn dialect_ambiguous_operand_declines_to_fold() {
    // A leading-zero decimal is octal in tcl8.x but decimal in tcl9.0, so
    // its numeric-vs-string comparison is dialect-dependent — the
    // const-folder declines rather than risk a wrong answer (false I230).
    assert_eq!(eval_str(r#""08" == "8""#), None);
    assert_eq!(eval_str(r#""010" == "8""#), None);
    assert_eq!(eval_str(r#""8" != "08""#), None);
    // But unambiguous numbers and non-numbers still fold.
    assert_eq!(eval_str(r#""0" == "0""#), Some(TclValue::Int(1)));
    assert_eq!(eval_str(r#""0x10" == "16""#), Some(TclValue::Int(1)));
    assert_eq!(eval_str(r#""08" == "foo""#), None); // 08 still ambiguous
}

#[test]
fn dialect_aware_leading_zero_folds_per_dialect() {
    let eval_d = |expr: &str, dialect: &str| {
        eval_tcl_expr_in_dialect(
            &parse_expr(expr, None),
            &Env::new(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        )
    };
    // tcl8.x octal: `08`/`09` are *invalid* octal → treated as strings, so
    // `"08" == "8"` is a string compare → 0. `010` is valid octal (8), so
    // `"010" == "8"` compares numerically → 1.
    for d in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules", "f5-iapps"] {
        assert_eq!(eval_d(r#""08" == "8""#, d), Some(TclValue::Int(0)), "{d}");
        assert_eq!(eval_d(r#""010" == "8""#, d), Some(TclValue::Int(1)), "{d}");
        assert_eq!(eval_d(r#""08" != "8""#, d), Some(TclValue::Int(1)), "{d}");
    }
    // tcl9.0 decimal (TIP 472): `08` → 8, `010` → 10, both numeric compares.
    assert_eq!(eval_d(r#""08" == "8""#, "tcl9.0"), Some(TclValue::Int(1)));
    assert_eq!(eval_d(r#""010" == "8""#, "tcl9.0"), Some(TclValue::Int(0)));
    assert_eq!(eval_d(r#""010" == "10""#, "tcl9.0"), Some(TclValue::Int(1)));
}

#[test]
fn dialect_aware_leading_zero_folds_in_arithmetic() {
    // TP: the octal-vs-decimal split must also apply to arithmetic /
    // unary / math-function operands, not just comparisons. Each value
    // verified against real tclsh8.6: `010 + 1` = 9 (octal 8+1),
    // `010 * 2` = 16, `-010` = -8, `abs(-010)` = 8.
    let eval_d = |expr: &str, dialect: &str| {
        eval_tcl_expr_in_dialect(
            &parse_expr(expr, None),
            &Env::new(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        )
    };
    for d in ["tcl8.4", "tcl8.5", "tcl8.6", "f5-irules", "f5-iapps"] {
        assert_eq!(eval_d("010 + 1", d), Some(TclValue::Int(9)), "{d}");
        assert_eq!(eval_d("010 * 2", d), Some(TclValue::Int(16)), "{d}");
        assert_eq!(eval_d("-010", d), Some(TclValue::Int(-8)), "{d}");
        assert_eq!(eval_d("abs(-010)", d), None, "unproved function: {d}");
        let profile = tcl_registry::model::ingress::resolve_environment(d).analyser_profile();
        assert_eq!(
            eval_tcl_expr_with_policy(
                &parse_expr("abs(-010)", Some(d)),
                &Env::new(),
                FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile))
                    .with_intrinsic_math(),
            ),
            Some(TclValue::Int(8)),
            "selected mathematical grammar: {d}"
        );
    }
    // tcl9.0 decimal (TIP 472): `010 + 1` = 11 (decimal 10+1).
    assert_eq!(eval_d("010 + 1", "tcl9.0"), Some(TclValue::Int(11)));
    assert_eq!(eval_d("-010", "tcl9.0"), Some(TclValue::Int(-10)));
}

#[test]
fn dialect_blind_arithmetic_declines_on_bare_leading_zero() {
    // FN guard (regression for the fix): when the dialect is genuinely
    // unknown (plain `eval_tcl_expr`, no octal info at all), arithmetic
    // on a bare leading-zero literal whose octal and decimal readings
    // *disagree* must decline rather than silently pick one — `010` is
    // octal 8 vs decimal 10.
    assert_eq!(eval_str("010 + 1"), None);
    assert_eq!(eval_str("-010"), None);
    assert_eq!(eval_str("abs(-010)"), None);
    // Unaffected: no leading-zero operand, arithmetic still folds.
    assert_eq!(eval_str("10 + 1"), Some(TclValue::Int(11)));
    // Unaffected: `07` is 7 under both readings (7 is a valid octal
    // digit), so a dialect-blind fold is still sound here.
    assert_eq!(eval_str("07 + 1"), Some(TclValue::Int(8)));
}

#[test]
fn invalid_octal_arithmetic_declines_under_octal_dialect() {
    // TN: `08 + 1` is a genuine Tcl *error* under the 8.x octal rule
    // (`08` is not valid octal — tclsh: "invalid bareword \"08\"") —
    // the const-folder has no error channel, so it must decline (defer
    // to the runtime, which will raise the real error) rather than
    // guess a decimal reading.
    let eval_d = |expr: &str, dialect: &str| {
        eval_tcl_expr_in_dialect(
            &parse_expr(expr, None),
            &Env::new(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        )
    };
    assert_eq!(eval_d("08 + 1", "tcl8.6"), None);
    // Decimal dialect reads `08` as 8, so this is fine there.
    assert_eq!(eval_d("08 + 1", "tcl9.0"), Some(TclValue::Int(9)));
}

#[test]
fn bpf_folds_leading_zero_as_decimal_like_its_tcl_9_runtime() {
    let eval_d = |expr: &str, dialect: &str| {
        eval_tcl_expr_in_dialect(
            &parse_expr(expr, None),
            &Env::new(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        )
    };
    // bpf embeds Tcl 9.0 (dialect-profile-model.md D7): `010` is decimal
    // 10 — tclsh9.0-verified (`expr {010 + 1}` → 11, `expr {08 + 1}` →
    // 9). The old string-prefix heuristic (!starts_with("tcl9")) wrongly
    // read bpf as octal.
    assert_eq!(eval_d("010 + 1", "bpf"), Some(TclValue::Int(11)));
    assert_eq!(eval_d("08 + 1", "bpf"), Some(TclValue::Int(9)));
    // 8.x runtimes keep the octal rule: tclsh8.6-verified
    // (`expr {010 + 1}` → 9).
    assert_eq!(eval_d("010 + 1", "tcl8.6"), Some(TclValue::Int(9)));
    assert_eq!(eval_d("010 + 1", "f5-irules"), Some(TclValue::Int(9)));
}

#[test]
fn no_runtime_profiles_abstain_from_octal_sensitive_folds() {
    let eval_d = |expr: &str, dialect: &str| {
        eval_tcl_expr_in_dialect(
            &parse_expr(expr, None),
            &Env::new(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        )
    };
    // f5-bigip has no Tcl runtime and an unknown dialect resolves to the
    // permissive fallback: leading-zero-sensitive folds abstain (§11.1)
    // rather than guessing a base, while octal-insensitive arithmetic
    // still folds.
    for d in ["f5-bigip", "no-such-dialect"] {
        assert_eq!(eval_d("010 + 1", d), None, "{d}: abstain on 010");
        assert_eq!(eval_d("1 + 1", d), Some(TclValue::Int(2)), "{d}");
    }
}

#[test]
fn leading_zero_octal_policy_follows_the_runtime_base() {
    // 8.x runtimes (plain and vendor) read leading zeros as octal.
    for d in [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
        "expect",
    ] {
        assert_eq!(
            leading_zero_is_octal(
                tcl_registry::model::ingress::resolve_environment(d).analyser_profile()
            ),
            Some(true),
            "{d}"
        );
    }
    // 9.x runtimes dropped the rule (TIP 114/472). bpf embeds Tcl 9.0,
    // so `010` is not octal there either.
    for d in ["tcl9.0", "tcl9.1", "bpf"] {
        assert_eq!(
            leading_zero_is_octal(
                tcl_registry::model::ingress::resolve_environment(d).analyser_profile()
            ),
            Some(false),
            "{d}"
        );
    }
    // No Tcl runtime / unknown dialect: abstain, never guess (§11.1).
    assert_eq!(
        leading_zero_is_octal(
            tcl_registry::model::ingress::resolve_environment("f5-bigip").analyser_profile()
        ),
        None
    );
    assert_eq!(
        leading_zero_is_octal(
            tcl_registry::model::ingress::resolve_environment("no-such-dialect").analyser_profile()
        ),
        None
    );
}

#[test]
fn literal_bool() {
    assert_eq!(parse_literal("true"), Some(TclValue::Int(1)));
    assert_eq!(parse_literal("yes"), Some(TclValue::Int(1)));
    assert_eq!(parse_literal("false"), Some(TclValue::Int(0)));
    assert_eq!(parse_literal("off"), Some(TclValue::Int(0)));
}

#[test]
fn literal_bool_unique_prefix() {
    // `Tcl_GetBoolean` accepts unique prefixes in a boolean-coercion
    // context (verified against tclsh: `expr {1 && tr}` => 1,
    // `expr {ye && of}` => 0).
    for w in ["t", "tr", "tru", "y", "ye", "on"] {
        assert_eq!(parse_literal(w), Some(TclValue::Int(1)), "{w}");
    }
    for w in ["f", "fa", "n", "no", "of", "off"] {
        assert_eq!(parse_literal(w), Some(TclValue::Int(0)), "{w}");
    }
    // Ambiguous `o` (on/off) is not a boolean.
    assert_eq!(parse_literal("o"), None);
}

#[test]
fn arithmetic_int() {
    assert_eq!(eval_str("1 + 2"), Some(TclValue::Int(3)));
    assert_eq!(eval_str("5 - 3"), Some(TclValue::Int(2)));
    assert_eq!(eval_str("4 * 5"), Some(TclValue::Int(20)));
    assert_eq!(eval_str("10 / 3"), Some(TclValue::Int(3)));
    assert_eq!(eval_str("10 % 3"), Some(TclValue::Int(1)));
}

#[test]
fn arithmetic_int_floor_division_negative() {
    // Tcl: floor toward -inf.
    assert_eq!(eval_str("-7 / 2"), Some(TclValue::Int(-4)));
    assert_eq!(eval_str("7 / -2"), Some(TclValue::Int(-4)));
    // Modulo sign follows divisor.
    assert_eq!(eval_str("-7 % 2"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("7 % -2"), Some(TclValue::Int(-1)));
}

#[test]
fn arithmetic_float_promotion() {
    assert_eq!(eval_str("1 + 2.5"), Some(TclValue::Float(3.5)));
    assert_eq!(eval_str("10.0 / 4"), Some(TclValue::Float(2.5)));
}

#[test]
fn division_by_zero() {
    assert_eq!(eval_str("1 / 0"), None);
    assert_eq!(eval_str("1 % 0"), None);
    // tclsh: `expr {0.0/0.0}` is a domain error (NaN) — declines.
    assert_eq!(eval_str("0.0 / 0.0"), None);
}

#[test]
fn float_division_by_zero_with_nonzero_numerator_folds_to_infinity() {
    // TP (fixes a previous over-conservative decline): tclsh:
    // `expr {1.0/0.0}` -> Inf, `expr {-1.0/0.0}` -> -Inf,
    // `expr {1.0/-0.0}` -> -Inf. A non-zero numerator over a zero
    // divisor is a real, foldable IEEE value, not a domain error —
    // only 0.0/0.0 itself errors.
    assert_eq!(eval_str("1.0 / 0.0"), Some(TclValue::Float(f64::INFINITY)));
    assert_eq!(
        eval_str("-1.0 / 0.0"),
        Some(TclValue::Float(f64::NEG_INFINITY))
    );
    assert_eq!(
        eval_str("1.0 / -0.0"),
        Some(TclValue::Float(f64::NEG_INFINITY))
    );
}

#[test]
fn pow_integer() {
    assert_eq!(eval_str("2 ** 10"), Some(TclValue::Int(1024)));
    assert_eq!(eval_str("2 ** 0"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("0 ** 5"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("(-1) ** 3"), Some(TclValue::Int(-1)));
    assert_eq!(eval_str("(-1) ** 4"), Some(TclValue::Int(1)));
    // |base| > 1 with negative exp → 0 (Tcl integer rules).
    assert_eq!(eval_str("2 ** -5"), Some(TclValue::Int(0)));
    // 0 ** negative → error.
    assert_eq!(eval_str("0 ** -1"), None);
}

#[test]
fn comparisons_return_0_or_1() {
    assert_eq!(eval_str("1 < 2"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("2 < 1"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("3 == 3"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("3 != 3"), Some(TclValue::Int(0)));
}

#[test]
fn mixed_int_double_comparisons_are_exact_past_2_pow_53() {
    // tclsh (8.6 and 9.0): a wide compares against a double at full
    // precision, not through a lossy both-as-f64 conversion.
    //   expr {9007199254740993 == 9007199254740992.0} → 0
    //   expr {9007199254740993 >  9007199254740992.0} → 1
    //   expr {9007199254740993 >  9007199254740993.0} → 1  (the float
    //   literal itself rounds down to …992.0)
    assert_eq!(
        eval_str("9007199254740993 == 9007199254740992.0"),
        Some(TclValue::Int(0))
    );
    assert_eq!(
        eval_str("9007199254740993 > 9007199254740992.0"),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_str("9007199254740993 > 9007199254740993.0"),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_str("20000000000000003 < 20000000000000004.0"),
        Some(TclValue::Int(1))
    );
    // Order flipped: the double on the left.
    assert_eq!(
        eval_str("9007199254740992.0 < 9007199254740993"),
        Some(TclValue::Int(1))
    );
}

#[test]
fn nan_comparisons_follow_tcl_unordered_rule() {
    // tclsh: `set x NaN; expr {$x == 1}` → 0, `!=` → 1, every ordering
    // comparison → 0 — numeric-unordered, NOT a string comparison.
    let mut env = Env::new();
    env.insert("x".to_owned(), EnvValue::Str("NaN".to_owned()));
    assert_eq!(value_86_env("$x == 1", &env), Some(TclValue::Int(0)));
    assert_eq!(value_86_env("$x != 1", &env), Some(TclValue::Int(1)));
    assert_eq!(value_86_env("$x < 1", &env), Some(TclValue::Int(0)));
    assert_eq!(value_86_env("$x <= 1", &env), Some(TclValue::Int(0)));
    assert_eq!(value_86_env("$x > 1", &env), Some(TclValue::Int(0)));
    assert_eq!(value_86_env("$x >= 1", &env), Some(TclValue::Int(0)));
    // NaN != NaN numerically… but `eq` is a string comparison, so the
    // spelling matters there instead (both verified against tclsh).
    assert_eq!(value_86_env("$x == $x", &env), Some(TclValue::Int(0)));
    assert_eq!(value_86_env("$x eq $x", &env), Some(TclValue::Int(1)));
}

#[test]
fn wide_vs_2_pow_63_double_declines_to_fold() {
    // `9223372036854775807 < 9223372036854775808.0` is answered
    // platform-dependently by C Tcl (UB in its double→wide cast: x86-64
    // says 0, a saturating conversion says 1, exact maths says 1) — the
    // folder must decline rather than bake in any one answer.
    assert_eq!(
        eval_str("9223372036854775807 < 9223372036854775808.0"),
        None
    );
    assert_eq!(
        eval_str("9223372036854775807 == 9223372036854775808.0"),
        None
    );
    // The negative mirror is well-defined (−2⁶³ is representable): fold.
    assert_eq!(
        eval_str("-9223372036854775807-1 == -9223372036854775808.0"),
        Some(TclValue::Int(1))
    );
    // And a double clearly past the boundary is well-defined: 2⁶³−1 < 9.3e18.
    assert_eq!(
        eval_str("9223372036854775807 < 9.3e18"),
        Some(TclValue::Int(1))
    );
}

#[test]
fn string_comparisons() {
    assert_eq!(eval_86("1 eq 1"), Some(TclValue::Int(1)));
    assert_eq!(eval_86("1 ne 2"), Some(TclValue::Int(1)));
}

#[test]
fn string_comparisons_fold_string_operands() {
    // `eq`/`ne`/`lt`/`gt`/`le`/`ge` compare
    // operands AS strings, so string-only operands fold
    // (an earlier numeric parse returned None for them).
    assert_eq!(eval_86("\"x\" eq \"y\""), Some(TclValue::Int(0)));
    assert_eq!(eval_86("\"x\" ne \"y\""), Some(TclValue::Int(1)));
    assert_eq!(eval_86("\"x\" eq \"x\""), Some(TclValue::Int(1)));
    // Lexical ordering on string operands.
    assert_eq!(
        eval_release("\"abc\" lt \"abd\"", "tcl9.0"),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_release("\"b\" gt \"a\"", "tcl9.0"),
        Some(TclValue::Int(1))
    );
    // `5 eq 5.0` compares the strings "5" vs "5.0" (→ 0), matching
    // C Tcl 9 — a regression guard for the numeric-looking case.
    assert_eq!(eval_86("5 eq 5.0"), Some(TclValue::Int(0)));
}

#[test]
fn short_circuit_and() {
    // Second operand is an unbound variable — short-circuit must
    // avoid evaluating it when the first operand is falsy.
    let env = Env::new();
    assert_eq!(eval_str_env("0 && $undef", &env), Some(TclValue::Int(0)));
}

#[test]
fn short_circuit_or() {
    let env = Env::new();
    assert_eq!(eval_str_env("1 || $undef", &env), Some(TclValue::Int(1)));
}

#[test]
fn ternary_selects_correct_branch() {
    assert_eq!(eval_str("1 ? 10 : 20"), Some(TclValue::Int(10)));
    assert_eq!(eval_str("0 ? 10 : 20"), Some(TclValue::Int(20)));
}

#[test]
fn unary_operators() {
    assert_eq!(eval_str("-5"), Some(TclValue::Int(-5)));
    assert_eq!(eval_str("+5"), Some(TclValue::Int(5)));
    assert_eq!(eval_str("!0"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("!1"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("~0"), Some(TclValue::Int(-1)));
}

#[test]
fn bitwise_operators() {
    assert_eq!(eval_str("0xff & 0x0f"), Some(TclValue::Int(0x0f)));
    assert_eq!(eval_str("0xf0 | 0x0f"), Some(TclValue::Int(0xff)));
    assert_eq!(eval_str("0xff ^ 0x0f"), Some(TclValue::Int(0xf0)));
}

#[test]
fn shifts() {
    assert_eq!(eval_str("1 << 4"), Some(TclValue::Int(16)));
    assert_eq!(eval_str("16 >> 2"), Some(TclValue::Int(4)));
    // Negative shift count is undefined.
    assert_eq!(eval_str("1 << -1"), None);
}

#[test]
fn right_shift_at_and_past_width() {
    // `x >> 64` must not execute a 64-bit shift (which
    // panics in debug / masks to `x >> 0` in release). At >= 64 the
    // result is the replicated sign bit.
    assert_eq!(eval_str("5 >> 64"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("5 >> 65"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("-5 >> 64"), Some(TclValue::Int(-1)));
    assert_eq!(eval_str("-5 >> 100"), Some(TclValue::Int(-1)));
    // Just below the boundary still shifts normally.
    assert_eq!(eval_str("-1 >> 63"), Some(TclValue::Int(-1)));
}

#[test]
fn variable_resolution_from_env() {
    let mut env = Env::new();
    env.insert("x".into(), EnvValue::Int(42));
    assert_eq!(eval_str_env("$x + 8", &env), Some(TclValue::Int(50)));
}

#[test]
fn unbound_variable_is_none() {
    let env = Env::new();
    assert_eq!(eval_str_env("$undef + 1", &env), None);
}

/// A command substitution an expression evaluates is the nested
/// service's to run, not the evaluator's: on its own the evaluator has no
/// service to ask and answers neither program. With the service at
/// `LocalWrites` and `x` holding 1, `[incr x] + 1` is 3 and the state
/// holds the write that makes `x` 2; under `EffectFreeOnly` the write is
/// `StatefulNested`. A command that reads the wall clock has no answer
/// under any policy, and makes no write.
#[test]
fn command_substitution_evaluates_through_the_nested_service() {
    use crate::value_transfer::evaluate_over_x;
    use tcl_registry::value_transfer::{
        DeclineReason, ExactValueOrUnavailable, LiftedAnswer, NestedPolicy, StoreOutcome,
    };
    assert_eq!(eval_str("[incr x] + 1"), None);
    assert_eq!(eval_str("[clock seconds] + 1"), None);

    let LiftedAnswer::Evaluated(outcomes) =
        evaluate_over_x("[incr x] + 1", NestedPolicy::LocalWrites)
    else {
        panic!("the nested service evaluates `[incr x] + 1` under `LocalWrites`");
    };
    let [outcome] = outcomes.as_slice() else {
        panic!("one outcome: {outcomes:?}");
    };
    let ExactValueOrUnavailable::Exact(result) = &outcome.result else {
        panic!("an exact result: {outcome:?}");
    };
    assert_eq!(result.bytes, b"3");
    let [(place, StoreOutcome::Write { value, .. })] = outcome.nested_writes.as_slice() else {
        panic!("one write: {:?}", outcome.nested_writes);
    };
    assert_eq!(
        (place.name.as_str(), value.bytes.as_slice()),
        ("x", &b"2"[..])
    );

    assert!(matches!(
        evaluate_over_x("[incr x] + 1", NestedPolicy::EffectFreeOnly),
        LiftedAnswer::Declined(DeclineReason::StatefulNested)
    ));
    for nested in [NestedPolicy::EffectFreeOnly, NestedPolicy::LocalWrites] {
        assert!(
            matches!(
                evaluate_over_x("[clock seconds] + 1", nested),
                LiftedAnswer::Declined(_)
            ),
            "{nested:?}"
        );
    }
}

#[test]
fn format_tcl_value_int_and_float() {
    assert_eq!(format_tcl_value(&TclValue::Int(42)), "42");
    assert_eq!(format_tcl_value(&TclValue::Int(-7)), "-7");
    assert_eq!(format_tcl_value(&TclValue::Float(1.5)), "1.5");
    // Integer-valued floats render with trailing .0.
    assert_eq!(format_tcl_value(&TclValue::Float(3.0)), "3.0");
}

#[test]
fn overflow_promotes_to_exact_bignum() {
    // 10 ** 100 overflows a wide: C Tcl promotes to a bignum and so does
    // the folder (`type-tracking.md`) — exactly, never wrapped.
    let want = format!("1{}", "0".repeat(100));
    assert_eq!(
        eval_str("10 ** 100").map(|v| format_tcl_value(&v)),
        Some(want)
    );
}

// matches_regex is never constant-folded.

#[test]
fn irules_matches_regex_is_not_folded() {
    // `matches_regex` is deferred to the runtime ARE engine rather
    // than folded via the Rust `regex` crate (whose syntax/semantics
    // differ from Tcl ARE), so the constant evaluator returns None
    // for every pattern — even ones the Rust engine could match.
    for expr in [
        r#""hello world" matches_regex "world""#,
        r#""hello" matches_regex "^bye""#,
        r#""abc123" matches_regex "^[a-z]+[0-9]+$""#,
        r#""apple" matches_regex "apple|orange""#,
        r#""prefix-world-suffix" matches_regex "world""#,
        r#""abc" matches_regex "(?=a)abc""#,
        r#""abc" matches_regex "[unterminated""#,
    ] {
        assert_eq!(eval_irules(expr), None, "{expr}");
    }
}

// Simple iRules string ops.

/// `FoldOps::binary_other` must only fold the iRules word operators under
/// an iRules dialect, and decline (not panic, not silently misfold)
/// everywhere else — the defence-in-depth check for
/// call sites that reach this evaluator without a dialect string
/// (`eval_tcl_expr`/`eval_tcl_expr_with_octal`).
#[test]
fn measured_bigip_bare_matches_glob_discriminators_fold_only_under_authored_policy() {
    let env = Env::new();
    for (row_index, row) in
        include_str!("../../../tcl-syntax/tests/data/f5-matches-21.1.0.1-0.0.26.tsv")
            .lines()
            .enumerate()
    {
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        let expression = fields[0];
        let expected = if fields[1] == "0" {
            Some(TclValue::Int(fields[2].parse().unwrap()))
        } else {
            None
        };
        assert_eq!(eval_irules(expression), expected, "{expression}");
        let node = parse_expr(expression, Some("f5-irules"));
        if row_index >= 6 {
            continue;
        }
        assert_eq!(eval_tcl_expr(&node, &env), None);
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            assert_eq!(
                eval_tcl_expr_in_dialect(
                    &node,
                    &env,
                    tcl_registry::model::ingress::resolve_environment(engine).analyser_profile()
                ),
                None,
                "{engine}: {expression}"
            );
        }
    }
}

#[test]
fn irules_contains_folds_under_irules_and_declines_under_plain_tcl() {
    let node_irules = parse_expr(r#""abc" contains "b""#, Some("f5-irules"));
    let env = Env::new();

    // Folds to true under the iRules dialect.
    assert_eq!(
        eval_tcl_expr_in_dialect(&node_irules, &env, tcl_dialect::DialectProfile::irules()),
        Some(TclValue::Int(1))
    );

    // The bare, dialect-less entry points decline rather than assume
    // plain Tcl — no fold, no crash.
    assert_eq!(eval_tcl_expr(&node_irules, &env), None);
    assert_eq!(eval_tcl_expr_with_octal(&node_irules, &env, None), None);

    // And explicitly asking for a plain-Tcl dialect also declines.
    assert_eq!(
        eval_tcl_expr_in_dialect(
            &node_irules,
            &env,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile()
        ),
        None
    );
}

#[test]
fn irules_contains() {
    assert_eq!(
        eval_irules(r#""hello world" contains "world""#),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_irules(r#""hello" contains "bye""#),
        Some(TclValue::Int(0))
    );
}

#[test]
fn irules_starts_with() {
    assert_eq!(
        eval_irules(r#""foobar" starts_with "foo""#),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_irules(r#""foobar" starts_with "bar""#),
        Some(TclValue::Int(0))
    );
}

#[test]
fn irules_ends_with() {
    assert_eq!(
        eval_irules(r#""foobar" ends_with "bar""#),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_irules(r#""foobar" ends_with "foo""#),
        Some(TclValue::Int(0))
    );
}

#[test]
fn irules_str_equals() {
    assert_eq!(eval_irules(r#""abc" equals "abc""#), Some(TclValue::Int(1)));
    assert_eq!(eval_irules(r#""abc" equals "xyz""#), Some(TclValue::Int(0)));
}

#[test]
fn irules_string_op_with_bound_variable() {
    let mut env = Env::new();
    env.insert("name".into(), EnvValue::Str("production".into()));
    let profile = tcl_dialect::DialectProfile::irules();
    assert_eq!(
        value_over(
            &parse_expr(r#"$name contains "prod""#, Some("f5-irules")),
            &env,
            FoldPolicy::for_profile(leading_zero_is_octal(profile), Some(profile)),
        ),
        Some(TclValue::Int(1))
    );
}

// matches_glob + in/ni.

#[test]
fn irules_matches_glob_star() {
    assert_eq!(
        eval_irules(r#""hello world" matches_glob "hello*""#),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_irules(r#""hello world" matches_glob "*world""#),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_irules(r#""hello world" matches_glob "*lo w*""#),
        Some(TclValue::Int(1))
    );
}

/// A bracket class is written braced: a quoted operand is substituted,
/// so `"a[bxy]c"` would run the command `bxy` (tclsh 8.4 to 9.1 raise
/// `invalid command name "bxy"` for `expr {"abc" eq "a[bxy]c"}`) and
/// declines here.
#[test]
fn irules_matches_glob_question_and_class() {
    assert_eq!(
        eval_irules(r#""abc" matches_glob "a?c""#),
        Some(TclValue::Int(1))
    );
    // A class is written braced: inside `"…"` the `[bxy]` is a command
    // substitution, which the folder declines (#2227).
    assert_eq!(
        eval_irules(r#""abc" matches_glob {a[bxy]c}"#),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_irules(r#""axc" matches_glob {a[bxy]c}"#),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_irules(r#""azc" matches_glob {a[bxy]c}"#),
        Some(TclValue::Int(0))
    );
    assert_eq!(eval_irules(r#""abc" matches_glob "a[bxy]c""#), None);
}

#[test]
fn irules_matches_glob_rejects_on_mismatch() {
    assert_eq!(
        eval_irules(r#""hello" matches_glob "world""#),
        Some(TclValue::Int(0))
    );
}

/// The F5 core is 8.4.6, which has no `in`/`ni` (measured; W003), so
/// the iRules fold of a membership test declines.
#[test]
fn irules_in_and_ni_fold_nothing() {
    assert_eq!(eval_irules(r#""b" in "a b c""#), None);
    assert_eq!(eval_irules(r#""d" ni "a b c""#), None);
}

#[test]
fn split_tcl_list_handles_braces_and_quotes() {
    assert_eq!(
        split_tcl_list("a b c", tcl_syntax::word_rules::WordValueRules::default()),
        vec!["a", "b", "c"]
    );
    assert_eq!(
        split_tcl_list(
            "{hello world} foo",
            tcl_syntax::word_rules::WordValueRules::default()
        ),
        vec!["hello world", "foo"]
    );
    assert_eq!(
        split_tcl_list("", tcl_syntax::word_rules::WordValueRules::default()),
        Vec::<String>::new()
    );
}

#[test]
fn matches_glob_folds_through_shared_string_match() {
    // `matches_glob` (iRules dialect) now folds through the shared
    // `tcl_syntax::glob` (one `string match` dialect); see that module for
    // the full grammar.
    assert_eq!(
        eval_irules(r#""abc" matches_glob "a*c""#),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_irules(r#""ab" matches_glob "a*c""#),
        Some(TclValue::Int(0))
    );
    assert_eq!(
        eval_irules(r#""abc" matches_glob "a?c""#),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_irules(r#""abc" matches_glob "a?d""#),
        Some(TclValue::Int(0))
    );
}

// (string-delimiter stripping now lives in the shared `tcl_syntax::expr`
// walk — `strip_delims` — and is exercised by its tests.)

// Math function dispatch.

#[test]
fn math_abs_int_and_float() {
    assert_eq!(eval_str("abs(-5)"), Some(TclValue::Int(5)));
    assert_eq!(eval_str("abs(-1.5)"), Some(TclValue::Float(1.5)));
}

#[test]
fn lshift_overflowing_a_wide_promotes_exactly() {
    // Promotes: `1 << 63` overflows a wide; Tcl promotes to
    // the bignum 9223372036854775808 and the folder now computes it
    // exactly (never the wrapped `i64::MIN`).
    assert_eq!(
        eval_str("1 << 63").map(|v| format_tcl_value(&v)),
        Some("9223372036854775808".to_owned())
    );
    assert_eq!(
        eval_str("1 << 64").map(|v| format_tcl_value(&v)),
        Some("18446744073709551616".to_owned())
    );
    assert_eq!(
        eval_str("2 << 62").map(|v| format_tcl_value(&v)),
        Some("9223372036854775808".to_owned())
    );
    // (TN / FP-guard) In-range shifts still fold to the exact wide.
    assert_eq!(eval_str("1 << 62"), Some(TclValue::Int(1i64 << 62)));
    assert_eq!(eval_str("1 << 0"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("3 << 4"), Some(TclValue::Int(48)));
    // `0 << y` never overflows, for any non-negative count.
    assert_eq!(eval_str("0 << 63"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("0 << 100"), Some(TclValue::Int(0)));
    // A negative-magnitude shift that stays in range folds normally.
    assert_eq!(eval_str("-1 << 4"), Some(TclValue::Int(-16)));
    // A count past the smallness cap declines — a folded literal of
    // thousands of digits helps nobody.
    assert_eq!(eval_str("1 << 100000"), None);
}

/// NaN in a boolean context is a C Tcl domain error ("floating point
/// value is Not a Number"), so the folder declines — it must never pick
/// a truth value (tclsh-verified; `Inf` IS truthy).
#[test]
fn nan_in_boolean_context_declines() {
    assert_eq!(eval_str("NaN ? 1 : 0"), None);
    assert_eq!(eval_str("!NaN"), None);
    assert_eq!(eval_str("NaN && 1"), None);
    assert_eq!(eval_str("Inf ? 1 : 0"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("!Inf"), Some(TclValue::Int(0)));
}

/// The float-edge reference table (tclsh 8.6.14; 9.0 agrees). Errors fold
/// to `None`; values fold to C's exact canonical text:
///
/// ```text
/// NaN + 1      => error (non-numeric operand)      -NaN / +NaN => error
/// NaN == NaN   => 0     NaN != 1.0 => 1   NaN < 1 => 0   (IEEE unordered,
///                                                          NOT an error)
/// Inf - Inf    => error (domain)   Inf * 0 => error (domain)
/// Inf + 1      => Inf   -Inf * -1 => Inf   Inf == Inf => 1
/// 5.0 / 0      => Inf   -5.0 / 0 => -Inf   0.0/0.0 => error
/// 1e309        => Inf   4.9e-324 => 5e-324 (denormal round-trip)
/// -0.0         => -0.0  0.0 == -0.0 => 1   -0.0 + 0.0 => 0.0
/// isqrt(4611686018427387903) => 2147483647 (exact, not the f64 2^31)
/// ```
#[test]
fn float_edge_oracle_table() {
    let fold = |e: &str| eval_str(e).map(|v| format_tcl_value(&v));
    // NaN in arithmetic / unary: operand errors — decline.
    assert_eq!(eval_str("NaN + 1"), None);
    assert_eq!(eval_str("-NaN"), None);
    assert_eq!(eval_str("+NaN"), None);
    // NaN in comparisons: IEEE unordered values, NOT errors.
    assert_eq!(eval_str("NaN == NaN"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("NaN != 1.0"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("NaN < 1"), Some(TclValue::Int(0)));
    // NaN results are domain errors — decline.
    assert_eq!(eval_str("Inf - Inf"), None);
    assert_eq!(eval_str("Inf * 0"), None);
    // Inf propagates as a real value.
    assert_eq!(fold("Inf + 1"), Some("Inf".into()));
    assert_eq!(fold("-Inf * -1"), Some("Inf".into()));
    assert_eq!(eval_str("Inf == Inf"), Some(TclValue::Int(1)));
    assert_eq!(
        eval_str("Inf > 9223372036854775807"),
        Some(TclValue::Int(1))
    );
    // Float division by (any) zero: ±Inf; 0.0/0.0 is the domain error.
    assert_eq!(fold("5.0 / 0"), Some("Inf".into()));
    assert_eq!(fold("-5.0 / 0"), Some("-Inf".into()));
    assert_eq!(eval_str("0.0 / 0.0"), None);
    // Overflowing literals parse to Inf; denormals round-trip.
    assert_eq!(fold("1e309"), Some("Inf".into()));
    assert_eq!(fold("4.9e-324"), Some("5e-324".into()));
    assert_eq!(fold("1e-330"), Some("0.0".into()));
    // Signed zero.
    assert_eq!(fold("-0.0"), Some("-0.0".into()));
    assert_eq!(eval_str("0.0 == -0.0"), Some(TclValue::Int(1)));
    assert_eq!(fold("-0.0 + 0.0"), Some("0.0".into()));
    // Exact integer square root at the f64-rounding edge.
    assert_eq!(
        eval_str("isqrt(4611686018427387903)"),
        Some(TclValue::Int(2_147_483_647))
    );
    assert_eq!(
        eval_str("isqrt(9223372036854775806)"),
        Some(TclValue::Int(3_037_000_499))
    );
    // int()/round() beyond a wide: int() is dialect-divergent (8.6 wraps
    // mod 2^64, 9.0 is exact) — decline; int(Inf)/int(NaN) are errors.
    assert_eq!(eval_str("int(1e300)"), None);
    assert_eq!(eval_str("int(Inf)"), None);
    assert_eq!(eval_str("int(NaN)"), None);
}

/// Values verified on tclsh 8.6.14/9.0 (see type-tracking.md):
/// exact integer arithmetic at and beyond the wide
/// boundary, floor div/mod, double contamination, and bignum demotion.
#[test]
fn bignum_oracle_corpus() {
    let fold = |e: &str| eval_str(e).map(|v| format_tcl_value(&v));
    // Exact integer arithmetic at 2^53 (f64 would merge these).
    assert_eq!(
        fold("9007199254740992 + 1"),
        Some("9007199254740993".into())
    );
    // One double operand contaminates — with f64's genuine rounding.
    assert_eq!(
        fold("9007199254740992 + 1.0"),
        Some("9007199254740992.0".into())
    );
    // Wide → bignum promotion by result magnitude.
    assert_eq!(fold("2 ** 64"), Some("18446744073709551616".into()));
    assert_eq!(
        fold("9223372036854775807 + 1"),
        Some("9223372036854775808".into())
    );
    assert_eq!(
        fold("9223372036854775807 * 2"),
        Some("18446744073709551614".into())
    );
    // Bignum arithmetic is exact, and demotes back to a wide when the
    // result fits.
    assert_eq!(
        fold("18446744073709551616 + 1"),
        Some("18446744073709551617".into())
    );
    assert_eq!(
        eval_str("18446744073709551616 - 18446744073709551616"),
        Some(TclValue::Int(0))
    );
    // Floor division / modulus, both tiers.
    assert_eq!(eval_str("7 / 2"), Some(TclValue::Int(3)));
    assert_eq!(eval_str("-7 / 2"), Some(TclValue::Int(-4)));
    assert_eq!(eval_str("-7 % 2"), Some(TclValue::Int(1)));
    assert_eq!(
        fold("-18446744073709551616 / 3"),
        Some("-6148914691236517206".into())
    );
    assert_eq!(eval_str("18446744073709551617 % 2"), Some(TclValue::Int(1)));
    // Negation off the wide edge promotes; double negation demotes back.
    assert_eq!(
        fold("0 - (-9223372036854775807 - 1)"),
        Some("9223372036854775808".into())
    );
    // Bignum comparisons are exact (distinct beyond-2^53 values).
    assert_eq!(
        eval_str("18446744073709551617 > 18446744073709551616"),
        Some(TclValue::Int(1))
    );
    assert_eq!(
        eval_str("9007199254740993 == 9007199254740992"),
        Some(TclValue::Int(0))
    );
    // Bitwise on the bignum tier (two's-complement).
    assert_eq!(
        fold("18446744073709551616 | 1"),
        Some("18446744073709551617".into())
    );
    assert_eq!(
        fold("~18446744073709551616"),
        Some("-18446744073709551617".into())
    );
}

#[test]
fn arith_rejects_boolean_words() {
    // Arithmetic/unary/mathfunc reject boolean words the
    // way Tcl's numeric context does — folding them would replace an error
    // with a value. All of these are Tcl errors, so the folder declines.
    assert_eq!(eval_str("true + 0"), None); // (TP)
    assert_eq!(eval_str("yes * 2"), None);
    assert_eq!(eval_str("off - 1"), None);
    assert_eq!(eval_str("-true"), None);
    assert_eq!(eval_str("~yes"), None);
    assert_eq!(eval_str("abs(true)"), None);
    assert_eq!(eval_str("int(no)"), None);
    // (TN / FP-guard) Genuine numbers still fold, and the boolean-accepting
    // constructs (`!`, `bool()`) keep working.
    assert_eq!(eval_str("1 + 0"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("abs(-5)"), Some(TclValue::Int(5)));
    assert_eq!(eval_str("!true"), Some(TclValue::Int(0))); // logical not takes a bool
    assert_eq!(eval_str("!0"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("bool(true)"), Some(TclValue::Int(1))); // bool() accepts words
    assert_eq!(eval_str("bool(no)"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("bool(42)"), Some(TclValue::Int(1)));
}

#[test]
fn math_int_conversion_truncates() {
    assert_eq!(eval_str("int(3.7)"), Some(TclValue::Int(3)));
    assert_eq!(eval_str("int(-3.7)"), Some(TclValue::Int(-3)));
    assert_eq!(eval_str("entier(2.9)"), Some(TclValue::Int(2)));
    assert_eq!(eval_str("wide(1)"), Some(TclValue::Int(1)));
}

#[test]
fn math_double_promotes_ints() {
    assert_eq!(eval_str("double(3)"), Some(TclValue::Float(3.0)));
}

#[test]
fn math_bool_normalises_to_01() {
    assert_eq!(eval_str("bool(42)"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("bool(0)"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("bool(0.0)"), Some(TclValue::Int(0)));
}

#[test]
fn math_round_ties_away_from_zero() {
    // Tcl round: 0.5 → 1, -0.5 → -1 (NOT banker's rounding).
    assert_eq!(eval_str("round(0.5)"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("round(-0.5)"), Some(TclValue::Int(-1)));
    assert_eq!(eval_str("round(1.5)"), Some(TclValue::Int(2)));
    assert_eq!(eval_str("round(-1.5)"), Some(TclValue::Int(-2)));
    assert_eq!(eval_str("round(2.5)"), Some(TclValue::Int(3)));
}

/// The fold is C's `modf` form, not `floor(d + 0.5)`, which rounds twice
/// and would bake `1` and `4503599627370498` into the compiled code where
/// tclsh 8.6.16/9.0.4 answer `0` and `4503599627370497`.
#[test]
fn math_round_folds_the_exact_value_not_d_plus_a_half() {
    assert_eq!(
        eval_str("round(0.49999999999999994)"),
        Some(TclValue::Int(0))
    );
    assert_eq!(
        eval_str("round(-0.49999999999999994)"),
        Some(TclValue::Int(0))
    );
    assert_eq!(
        eval_str("round(4503599627370497.0)"),
        Some(TclValue::Int(4_503_599_627_370_497))
    );
    assert_eq!(
        eval_str("round(-4503599627370497.0)"),
        Some(TclValue::Int(-4_503_599_627_370_497))
    );
    assert_eq!(
        eval_str("round(2251799813685248.5)"),
        Some(TclValue::Int(2_251_799_813_685_249))
    );
}

#[test]
fn math_ceil_and_floor_return_floats() {
    assert_eq!(eval_str("ceil(1.2)"), Some(TclValue::Float(2.0)));
    assert_eq!(eval_str("floor(1.8)"), Some(TclValue::Float(1.0)));
}

#[test]
fn math_min_max_preserve_int_width() {
    assert_eq!(eval_str("min(3, 1, 2)"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("max(3, 1, 2)"), Some(TclValue::Int(3)));
    // A mixed int/float call returns the
    // *winning* argument's own value, preserving its type — it does not
    // widen to float just because a float appeared among the operands.
    // `min(1, 2.5)` is `1` (an Int, since 1 wins), not `1.0` (confirmed
    // tclsh8.6/9.0); `max(1, 2.5)` is `2.5` (a Float, since 2.5 wins).
    assert_eq!(eval_str("min(1, 2.5)"), Some(TclValue::Int(1)));
    assert_eq!(eval_str("max(1, 2.5)"), Some(TclValue::Float(2.5)));
}

#[test]
fn math_sqrt_and_pow() {
    assert_eq!(eval_str("sqrt(16)"), Some(TclValue::Float(4.0)));
    assert_eq!(eval_str("pow(2, 10)"), Some(TclValue::Float(1024.0)));
}

#[test]
fn math_sqrt_negative_is_domain_error() {
    assert_eq!(eval_str("sqrt(-1)"), None);
}

#[test]
fn math_log_zero_and_negative_domain_error() {
    assert_eq!(eval_str("log(-1)"), None);
    // log(0) → -inf, treated as success (Tcl returns -inf too).
    let v = eval_str("log(0)");
    assert!(matches!(v, Some(TclValue::Float(f)) if f.is_infinite()));
}

#[test]
fn math_atan2_and_hypot() {
    // atan2(0, 1) = 0, hypot(3, 4) = 5
    assert_eq!(eval_str("atan2(0, 1)"), Some(TclValue::Float(0.0)));
    assert_eq!(eval_str("hypot(3, 4)"), Some(TclValue::Float(5.0)));
}

#[test]
fn math_trig_approx() {
    // sin(0) == 0.
    assert!(matches!(
        eval_str("sin(0)"),
        Some(TclValue::Float(f)) if f == 0.0
    ));
}

#[test]
fn math_classification() {
    assert_eq!(eval_str("isinf(1)"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("isnan(1.0)"), Some(TclValue::Int(0)));
    assert_eq!(eval_str("isfinite(1.0)"), Some(TclValue::Int(1)));
}

#[test]
fn math_isqrt_accepts_float_truncating_first() {
    assert_eq!(eval_str("isqrt(16)"), Some(TclValue::Int(4)));
    assert_eq!(eval_str("isqrt(17)"), Some(TclValue::Int(4)));
    // A `Float` operand must not fall to
    // `tcl_syntax::expr::mathfunc::dispatch`'s catch-all `None` (treated
    // as a domain error): real Tcl accepts one, truncating
    // toward zero first (`expr {isqrt(4.0)}` -> `2`, `isqrt(4.9)` -> `2`
    // same as `isqrt(4)`; confirmed tclsh8.6/9.0) — this const-folder
    // must fold the same value the real interpreter evaluates to, or a
    // W-series "simplify this constant expression" fix would propose a
    // wrong replacement.
    assert_eq!(eval_str("isqrt(4.0)"), Some(TclValue::Int(2)));
    assert_eq!(eval_str("isqrt(4.9)"), Some(TclValue::Int(2)));
    assert_eq!(eval_str("isqrt(-1.0)"), None); // domain error, same as isqrt(-1)
}

#[test]
fn math_rand_and_srand_always_none() {
    // Non-deterministic — callers must not constant-fold.
    assert_eq!(eval_str("rand()"), None);
    assert_eq!(eval_str("srand(42)"), None);
}

#[test]
fn math_unknown_function_is_none() {
    assert_eq!(eval_str("thereisnosuchfn(1)"), None);
}
