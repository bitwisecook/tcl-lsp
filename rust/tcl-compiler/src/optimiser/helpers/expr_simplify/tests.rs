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

use super::*;

/// A rewrite's fold inputs with no module mutations and no dialect.
fn folds() -> BuiltinFoldInputs<'static> {
    static TRUSTED: std::sync::LazyLock<crate::command_binding::ModuleCommandMutations> =
        std::sync::LazyLock::new(crate::command_binding::ModuleCommandMutations::default);
    BuiltinFoldInputs {
        source_metadata_input: None,
        registry: tcl_registry::default_registry(),
        mutations: &TRUSTED,
        dialect: None,
        defining_class: None,
        registry_engine: false,
        trust: crate::sccp::FoldTrust::WholeModule,
        proven_pure_parameters: false,
    }
}

// try_fold_expr

#[test]
fn fold_integer_arithmetic() {
    assert_eq!(try_fold_expr("1 + 2", folds()).as_deref(), Some("3"));
    assert_eq!(try_fold_expr("10 * 5", folds()).as_deref(), Some("50"));
    assert_eq!(try_fold_expr("100 / 4", folds()).as_deref(), Some("25"));
}

#[test]
fn fold_comparison_to_bool_literal() {
    assert_eq!(try_fold_expr("1 < 2", folds()).as_deref(), Some("1"));
    assert_eq!(try_fold_expr("3 == 3", folds()).as_deref(), Some("1"));
    assert_eq!(try_fold_expr("5 > 10", folds()).as_deref(), Some("0"));
}

#[test]
fn fold_returns_none_for_var_expressions() {
    assert!(try_fold_expr("$x + 1", folds()).is_none());
    assert!(try_fold_expr("[cmd]", folds()).is_none());
}

#[test]
fn fold_returns_none_when_already_literal() {
    // "42" folds to "42" — no change, None.
    assert!(try_fold_expr("42", folds()).is_none());
}

#[test]
fn fold_empty_expression() {
    assert!(try_fold_expr("", folds()).is_none());
    assert!(try_fold_expr("   ", folds()).is_none());
}

/// `simplify_node_once`,
/// `collect_add_terms`, `collect_mul_terms` and `expr_has_command_subst`
/// each recurse once per `ExprNode` level
/// with no depth cap before this fix. The Pratt parser caps *its* output
/// at 256 levels, but a tree built directly is unbounded and empirically
/// overflowed the native stack (SIGABRT) in the low thousands of levels
/// on a 2 MiB thread (`cargo test`'s default). 3000 is comfortably past
/// that crash range and past `MAX_EXPR_NODE_DEPTH` (256); the assertion is
/// that each walker returns at all.
#[test]
fn deeply_nested_expr_simplify_walks_survive() {
    use crate::expr_ast::UnaryOp;

    fn var_x() -> ExprNode {
        ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        }
    }

    // A 3000-deep nested-unary `!` tree — drives `simplify_node_once`
    // (its `reduce_unary`/`reassociate_node` helpers never render or eval
    // this Unary shape, so this isolates the walker's own recursion) and
    // the two boolean predicates.
    let mut unary = var_x();
    for _ in 0..3000 {
        unary = ExprNode::Unary {
            op: UnaryOp::Not,
            operand: Box::new(unary),
        };
    }
    let _ = simplify_node_once(&unary, false, None, 0);
    assert!(expr_has_command_subst(&ExprNode::Command {
        text: "[x]".into(),
        start: 0,
        end: 3,
    }));
    // Deep tree with no command subst / no mathfunc → the walkers descend
    // fully (capped) and answer `false` for realistic input; the point is
    // they do not overflow.
    let _ = expr_has_command_subst(&unary);

    // A 3000-deep left-nested `+` chain drives `collect_add_terms`.
    let mut add = var_x();
    for _ in 0..3000 {
        add = ExprNode::Binary {
            op: BinOp::Add,
            left: Box::new(add),
            right: Box::new(make_int_literal(1)),
        };
    }
    let mut terms = Vec::new();
    let _ = collect_add_terms(&add, &mut terms, 0);

    // A 3000-deep left-nested `*` chain drives `collect_mul_terms`.
    let mut mul = var_x();
    for _ in 0..3000 {
        mul = ExprNode::Binary {
            op: BinOp::Mul,
            left: Box::new(mul),
            right: Box::new(make_int_literal(2)),
        };
    }
    let mut mterms = Vec::new();
    let _ = collect_mul_terms(&mul, &mut mterms, 0);
}

// try_unwrap_expr_in_expr

#[test]
fn unwrap_braced_body() {
    assert_eq!(
        try_unwrap_expr_in_expr("[expr {$x + 1}]").as_deref(),
        Some("$x + 1"),
    );
}

#[test]
fn unwrap_quoted_body() {
    assert_eq!(
        try_unwrap_expr_in_expr(r#"[expr "$x + 1"]"#).as_deref(),
        Some("$x + 1"),
    );
}

#[test]
fn unwrap_bare_body() {
    assert_eq!(try_unwrap_expr_in_expr("[expr $x]").as_deref(), Some("$x"),);
}

#[test]
fn unwrap_rejects_non_expr_wrapping() {
    assert!(try_unwrap_expr_in_expr("[foo bar]").is_none());
    assert!(try_unwrap_expr_in_expr("$x + 1").is_none());
    assert!(try_unwrap_expr_in_expr("[express]").is_none());
}

#[test]
fn unwrap_rejects_empty_body() {
    assert!(try_unwrap_expr_in_expr("[expr {}]").is_none());
}

// substitute_expr_constants

fn consts(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn scalar_constant_source_substitution_keeps_exact_literal_names_and_array_refusal() {
    // naming.expression.scalar-reference-constant-source-projection
    // docs/design/analysis/name-resolution-proofs/expression-scalar-reference-constant-source-projection.md
    // Source/API emitter controls. Native public scalar-value observations
    // are separate; these constants establish no runtime object/read proof.
    let constants = consts(&[
        ("scalar(open", "11"),
        ("scalar(open)tail", "12"),
        ("scalar", "99"),
        ("$cash", "13"),
        ("cash", "98"),
        ("é(open", "14"),
        ("é", "97"),
        ("arr", "96"),
    ]);
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
        let invocation = tcl_native_model::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(dialect)).unwrap(),
        );
        let context = invocation.expression_parse_context(None);
        for (source, name, expected) in [
            ("${scalar(open}", "scalar(open", "11"),
            ("${scalar(open)tail}", "scalar(open)tail", "12"),
            ("${$cash}", "$cash", "13"),
            ("${é(open}", "é(open", "14"),
        ] {
            let out = substitute_expr_constants_in_context(source, &constants, &context);
            assert!(out.changed, "{dialect} {source}");
            assert_eq!(out.text, expected);
            assert_eq!(out.substituted, HashSet::from([name.to_owned()]));
            let execution = substitute_expr_constants_for_execution(
                source,
                &constants,
                tcl_dialect::DialectProfile::find(dialect),
                &crate::tcl_expr_eval::NativeOperandProofs::new(),
            );
            assert!(!execution.changed);
            assert_eq!(
                execution.text, source,
                "value constants grant no execution erasure"
            );
        }
        for source in ["$arr(key)", "${arr(key)}", "$arr($key)", "${missing"] {
            let out = substitute_expr_constants_in_context(source, &constants, &context);
            assert!(!out.changed, "{dialect} {source}");
            assert_eq!(out.text, source);
            assert!(out.substituted.is_empty());
        }
    }
}

#[test]
fn scalar_constant_context_uses_jim_variable_grammar_without_catalogue_profile() {
    // naming.expression.scalar-reference-constant-source-projection
    // docs/design/analysis/name-resolution-proofs/expression-scalar-reference-constant-source-projection.md
    assert!(tcl_dialect::DialectProfile::find("jimtcl").is_none());
    let jim = tcl_native_model::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::of_dialect_name(Some("jimtcl")).unwrap(),
    )
    .expression_parse_context(None);
    let c = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
        .expression_parse_context(None);
    let constants = consts(&[("é", "14"), ("x", "15")]);
    let jim_out = substitute_expr_constants_in_context("$é + $x", &constants, &jim);
    assert_eq!(jim_out.text, "14 + 15");
    assert_eq!(
        jim_out.substituted,
        HashSet::from(["é".to_owned(), "x".to_owned()])
    );
    let c_out = substitute_expr_constants_in_context("$é + $x", &constants, &c);
    assert_eq!(c_out.text, "$é + 15");
    assert_eq!(c_out.substituted, HashSet::from(["x".to_owned()]));
}

#[test]
fn scalar_constant_reference_selection_uses_actual_brace_close_grammar() {
    // naming.expression.scalar-reference-constant-source-projection
    // docs/design/analysis/name-resolution-proofs/expression-scalar-reference-constant-source-projection.md
    let constants = consts(&[("a{b", "15"), ("a{b}c", "16"), ("a", "99")]);
    let first = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let nested = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    assert_eq!(
        substitute_expr_constants("${a{b}", &constants, Some(first)).text,
        "15"
    );
    assert_eq!(
        substitute_expr_constants("${a{b}", &constants, Some(nested)).text,
        "${a{b}"
    );
    assert_eq!(
        substitute_expr_constants("${a{b}c}", &constants, Some(nested)).text,
        "16"
    );
    let mixed = substitute_expr_constants("${a{b}c}", &constants, Some(first));
    assert_eq!(mixed.text, "15c}");
    assert_eq!(mixed.substituted, HashSet::from(["a{b".to_owned()]));
}

#[test]
fn substitute_numeric_var_verbatim() {
    let c = consts(&[("x", "42")]);
    let out = substitute_expr_constants("$x + 1", &c, None);
    assert_eq!(out.text, "42 + 1");
    assert!(out.changed);
    assert!(out.substituted.contains("x"));
}

#[test]
fn substitute_string_var_is_quoted() {
    let c = consts(&[("s", "hello")]);
    let out = substitute_expr_constants("$s", &c, None);
    assert_eq!(out.text, "\"hello\"");
    assert!(out.changed);
}

#[test]
fn substitute_unknown_var_is_untouched() {
    let c = consts(&[("x", "1")]);
    let out = substitute_expr_constants("$y + 1", &c, None);
    assert!(!out.changed);
    assert!(out.substituted.is_empty());
}

#[test]
fn substitute_braced_var_reference() {
    let c = consts(&[("x", "7")]);
    let out = substitute_expr_constants("${x} * 2", &c, None);
    assert_eq!(out.text, "7 * 2");
    assert!(out.changed);
}

#[test]
fn substitute_escapes_backslash_and_quote() {
    let c = consts(&[("s", "he said \"hi\\there\"")]);
    let out = substitute_expr_constants("$s", &c, None);
    assert_eq!(out.text, r#""he said \"hi\\there\"""#);
}

// expr_has_command_subst

#[test]
fn command_subst_detection() {
    let expr = parse_expr_for_profile("[foo] + 1", None);
    assert!(expr_has_command_subst(&expr));
    let expr = parse_expr_for_profile("$x + 1", None);
    assert!(!expr_has_command_subst(&expr));
    let expr = parse_expr_for_profile("1 + 2", None);
    assert!(!expr_has_command_subst(&expr));
}

// try_strength_reduce_expr

#[test]
fn strength_reduce_identity_mul_one() {
    let (out, changed) = try_strength_reduce_expr("$x * 1");
    assert!(changed);
    assert_eq!(out.trim(), "$x");
}

#[test]
fn strength_reduce_identity_add_zero() {
    let (out, changed) = try_strength_reduce_expr("$x + 0");
    assert!(changed);
    assert_eq!(out.trim(), "$x");
}

#[test]
fn strength_reduce_pow2_mod() {
    let (out, changed) = try_strength_reduce_expr("$x % 8");
    assert!(changed);
    assert_eq!(out.trim(), "$x & 7");
}

#[test]
fn strength_reduce_pow_two_to_mul() {
    let (out, changed) = try_strength_reduce_expr("$x ** 2");
    assert!(changed);
    // Rendered form is "$x * $x" (exact spacing per render_expr).
    assert!(
        out.contains("$x") && out.contains('*'),
        "unexpected render: {out}",
    );
}

#[test]
fn strength_reduce_noop_leaves_unchanged() {
    let (out, changed) = try_strength_reduce_expr("$x + 5");
    assert!(!changed);
    assert_eq!(out, "$x + 5");
}

#[test]
fn arith_identity_numeric_guard() {
    // No type context (`None`) → unguarded behaviour: drop.
    for e in [
        "$x * 0", "$x + 0", "$x * 1", "$x - 0", "$x / 1", "$x % 1", "$x << 0",
    ] {
        let (_, changed) = instcombine_expr(e, false);
        assert!(changed, "None ctx must still simplify {e:?}");
    }
    // With a type context that does NOT prove `$x` numeric → keep the
    // operand (dropping it would hide a coercion error).
    let empty = OperandTypes::default();
    for e in ["$x * 0", "$x + 0", "$x * 1", "$x % 1", "$x << 0"] {
        let (out, changed) = instcombine_expr_typed(e, false, Some(&empty), None);
        assert!(!changed, "non-numeric `$x` must keep {e:?}, got {out:?}");
    }
    // With `$x` proven numeric (but not integer) → identities that KEEP the
    // operand fire (`$x + 0` → `$x`), but the integer-result annihilator
    // (`$x * 0` → `0`) does NOT (a double `$x` yields `0.0`).
    let numeric = OperandTypes::numeric_only(&["x"]);
    let (out, changed) = instcombine_expr_typed("$x + 0", false, Some(&numeric), None);
    assert!(
        changed && out.trim() == "$x",
        "numeric `$x + 0` → $x, got {out:?}"
    );
    let (out, changed) = instcombine_expr_typed("$x * 0", false, Some(&numeric), None);
    assert!(
        !changed,
        "numeric-but-not-integer `$x * 0` must NOT fold to 0, got {out:?}"
    );
    // With `$x` proven integer → the integer-result / integer-only rewrites
    // fire.
    let integer = OperandTypes::integer(&["x"]);
    let (out, changed) = instcombine_expr_typed("$x * 0", false, Some(&integer), None);
    assert!(
        changed && out.trim() == "0",
        "integer `$x * 0` → 0, got {out:?}"
    );
}

#[test]
fn integer_only_ops_need_integer_not_just_numeric() {
    // `<<`/`>>`/`&`/`|`/`^`/`%`/`**0`/`*0`/`$x-$x` are wrong
    // for a Double-typed operand — they either yield a double where an
    // integer literal is produced, or are integer-only ops Tcl rejects on a
    // float. A numeric-but-not-integer context must decline them all.
    let double = OperandTypes::numeric_only(&["x"]);
    for e in [
        "$x << 0", "$x >> 0", "$x & 0", "$x | 0", "$x ^ 0", "$x % 1", "$x ** 0", "$x * 0",
        "$x - $x", "$x ^ $x",
    ] {
        let (out, changed) = instcombine_expr_typed(e, false, Some(&double), None);
        assert!(
            !changed,
            "double `$x`: {e:?} must not be rewritten, got {out:?}"
        );
    }
    // An integer context fires them (spot-check a representative set).
    let integer = OperandTypes::integer(&["x"]);
    for (e, want) in [
        ("$x << 0", "$x"),
        ("$x & 0", "0"),
        ("$x % 1", "0"),
        ("$x ** 0", "1"),
        ("$x - $x", "0"),
    ] {
        let (out, changed) = instcombine_expr_typed(e, false, Some(&integer), None);
        assert!(
            changed && out.trim() == want,
            "integer `$x`: {e:?} → {want:?}, got {out:?}"
        );
    }
}

/// `!($x < 1)` → `$x >= 1` is wrong when `$x` may be NaN
/// (`expr {!(NaN < 1)}` is 1, `expr {NaN >= 1}` is 0), so the four ordered
/// comparisons invert only on operands proved NaN-free.
#[test]
fn ordered_comparison_inversion_needs_non_nan_operands() {
    // No proof available — with no type context at all, and with a context
    // that knows nothing about `$x`.
    let empty = OperandTypes::default();
    for e in ["!($x < 1)", "!($x <= 1)", "!($x > 1)", "!($x >= 1)"] {
        let (out, changed) = instcombine_expr(e, false);
        assert!(!changed, "untyped `$x`: {e:?} must not invert, got {out:?}");
        let (out, changed) = instcombine_expr_typed(e, false, Some(&empty), None);
        assert!(
            !changed,
            "unproven `$x`: {e:?} must not invert, got {out:?}"
        );
    }
    // A provably-integer `$x` cannot be NaN, so the inversion fires.
    let integer = OperandTypes::integer(&["x"]);
    for (e, want) in [
        ("!($x < 1)", "$x >= 1"),
        ("!($x <= 1)", "$x > 1"),
        ("!($x > 1)", "$x <= 1"),
        ("!($x >= 1)", "$x < 1"),
    ] {
        let (out, changed) = instcombine_expr_typed(e, false, Some(&integer), None);
        assert!(
            changed && out.trim() == want,
            "integer `$x`: {e:?} → {want:?}, got {out:?}"
        );
    }
    // A `Double`-typed operand is numeric but may still be NaN.
    let double = OperandTypes::numeric_only(&["x"]);
    let (out, changed) = instcombine_expr_typed("!($x < 1)", false, Some(&double), None);
    assert!(!changed, "double `$x`: must not invert, got {out:?}");
    // A NaN *literal* is no proof either, in any spelling.
    for e in ["!(NaN < 1)", "!(nan < 1)", "!(1 < NaN)"] {
        let (out, changed) = instcombine_expr_typed(e, false, Some(&empty), None);
        assert!(!changed, "{e:?} must not invert, got {out:?}");
    }
    // Two ordinary numeric literals are proof enough.
    let (out, changed) = instcombine_expr_typed("!(2 < 1)", false, Some(&empty), None);
    assert!(changed && out.trim() == "2 >= 1", "got {out:?}");
}

/// `==`/`!=` are exact complements even for NaN, and the string / membership
/// operators never compare numerically — those rows keep inverting with no
/// type facts at all.
#[test]
fn equality_and_string_comparison_inversion_stays_unconditional() {
    let empty = OperandTypes::default();
    for (e, want) in [
        ("!($x == $y)", "$x != $y"),
        ("!($x != $y)", "$x == $y"),
        ("!($x eq $y)", "$x ne $y"),
        ("!($x ne $y)", "$x eq $y"),
        ("!($x lt $y)", "$x ge $y"),
        ("!($x in $y)", "$x ni $y"),
        ("!($x ni $y)", "$x in $y"),
    ] {
        let (out, changed) = instcombine_expr_typed(e, false, Some(&empty), None);
        assert!(
            changed && out.trim() == want,
            "{e:?} → {want:?}, got {out:?}"
        );
    }
}

/// `$x == $x` is 0 and `$x != $x` is 1 for a NaN `$x`, so the
/// reflexive folds need the same non-NaN proof. `$x < $x` / `$x > $x` are
/// false for every value including NaN, and `eq`/`ne` have no NaN notion, so
/// those keep folding unconditionally.
#[test]
fn reflexive_self_comparison_folds_need_non_nan() {
    let empty = OperandTypes::default();
    for e in ["$x == $x", "$x != $x", "$x <= $x", "$x >= $x"] {
        let (out, changed) = instcombine_expr(e, false);
        assert!(!changed, "untyped `$x`: {e:?} must not fold, got {out:?}");
        let (out, changed) = instcombine_expr_typed(e, false, Some(&empty), None);
        assert!(!changed, "unproven `$x`: {e:?} must not fold, got {out:?}");
    }
    // Always-false and string-identity rows fold with no proof.
    for (e, want) in [
        ("$x < $x", "0"),
        ("$x > $x", "0"),
        ("$x eq $x", "1"),
        ("$x ne $x", "0"),
    ] {
        let (out, changed) = instcombine_expr_typed(e, false, Some(&empty), None);
        assert!(
            changed && out.trim() == want,
            "{e:?} → {want:?}, got {out:?}"
        );
    }
    // A provably-integer `$x` unlocks the reflexive rows.
    let integer = OperandTypes::integer(&["x"]);
    for (e, want) in [
        ("$x == $x", "1"),
        ("$x != $x", "0"),
        ("$x <= $x", "1"),
        ("$x >= $x", "1"),
    ] {
        let (out, changed) = instcombine_expr_typed(e, false, Some(&integer), None);
        assert!(
            changed && out.trim() == want,
            "integer `$x`: {e:?} → {want:?}, got {out:?}"
        );
    }
    // A `Double` is numeric but not NaN-free.
    let double = OperandTypes::numeric_only(&["x"]);
    let (out, changed) = instcombine_expr_typed("$x == $x", false, Some(&double), None);
    assert!(!changed, "double `$x == $x` must not fold, got {out:?}");
}

/// The NaN gate's proof sources, direct.
#[test]
fn node_cannot_be_nan_proof_sources() {
    let integer = OperandTypes::integer(&["i"]);
    let double = OperandTypes::numeric_only(&["d"]);
    let node = |src: &str| parse_expr_for_profile(src, None);
    assert!(node_cannot_be_nan(&node("1"), Some(&integer)));
    assert!(node_cannot_be_nan(&node("1.5"), Some(&integer)));
    assert!(node_cannot_be_nan(&node("\"abc\""), Some(&integer)));
    assert!(node_cannot_be_nan(&node("$i"), Some(&integer)));
    assert!(!node_cannot_be_nan(&node("$d"), Some(&double)));
    assert!(!node_cannot_be_nan(&node("$i"), None));
    assert!(!node_cannot_be_nan(&node("NaN"), Some(&integer)));
    assert!(!node_cannot_be_nan(&node("[f]"), Some(&integer)));
    assert!(is_nan_string_in_any_release("NaN"));
    assert!(is_nan_string_in_any_release("-nan"));
    assert!(!is_nan_string_in_any_release("42"));
    assert!(!is_nan_string_in_any_release("banana"));
}

#[test]
fn numeric_string_literal_is_provably_numeric() {
    // An SCCP-inlined numeric constant arrives as a string literal.
    assert!(is_numeric_string_in_every_release("42"));
    assert!(is_numeric_string_in_every_release("\"3.5\""));
    assert!(!is_numeric_string_in_every_release("\"abc\""));
    assert!(!is_numeric_string_in_every_release(""));
}

/// The gate reads through the shared Tcl grammar, not Rust's — `0x1a` is a
/// number here where `str::parse::<i64>/<f64>` rejects it outright.
///
/// It asks the *provable* question, though, so only spellings every release
/// agrees on qualify. `0x1a` is hex in all of them; `0o17`/`0b101` arrive in
/// 8.5 and `1_000` in 9.0, so none of those three is provably a number
/// without a target in hand — and this predicate gates rewrites that need a
/// genuine numeric operand. The permissive question has its own predicate,
/// [`is_numeric_string_in_any_release`], asserted below.
#[test]
fn is_numeric_string_recognises_non_decimal_tcl_numbers() {
    // Universal in every release — and rejected by Rust's own parsers.
    for text in ["0x1a", "\"0x1a\""] {
        assert!(
            is_numeric_string_in_every_release(text),
            "{text:?} should be provably numeric"
        );
    }
    // Real Tcl numbers, but only from 8.5 / 9.0 — not provable without a
    // target, and each *is* numeric under the release that has it.
    for (text, first) in [
        ("0o17", tcl_syntax::number::NumberSyntax::Tcl85),
        ("0b101", tcl_syntax::number::NumberSyntax::Tcl85),
        ("1_000", tcl_syntax::number::NumberSyntax::Tcl90),
    ] {
        assert!(
            !is_numeric_string_in_every_release(text),
            "{text:?} is not provable across releases"
        );
        assert!(
            is_numeric_string_under(text, Some(first)),
            "{text:?} is numeric under {first:?}"
        );
        assert!(
            is_numeric_string_in_any_release(text),
            "{text:?} is numeric on some release, so it blocks a promotion"
        );
    }
    // TN control: still rejects genuine non-numbers, both ways.
    for text in ["hello", "0xzz"] {
        assert!(!is_numeric_string_in_every_release(text), "{text:?}");
        assert!(!is_numeric_string_in_any_release(text), "{text:?}");
    }
}

#[test]
fn is_numeric_literal_agrees_with_is_numeric_string() {
    // Bare numeric emission uses the shared numeral grammar.
    assert!(is_numeric_literal("0x1a"));
    assert!(!is_numeric_literal("hello"));
}

#[test]
fn hex_string_literal_is_not_wrongly_promoted_to_streq() {
    // FP guard: `"0x1a" == $y` must NOT promote to `eq` (O120) — 0x1a
    // is a genuine Tcl number (tclsh: `expr {"0x1a" == 26}` -> 1, a
    // numeric compare), so treating it as "provably non-numeric" would
    // silently turn a numeric comparison into a string comparison.
    // `is_numeric_string_in_every_release("0x1a")` must therefore be
    // true, or `node_provably_non_numeric` returns true for this literal
    // and the eq/ne promotion fires.
    let (out, changed) = try_eq_ne_string_compare_simplify_expr("\"0x1a\" == $y", None);
    assert!(
        !changed,
        "hex literal must not be promoted to string eq, got {out:?}"
    );
}

// try_strlen_simplify_expr

#[test]
fn strlen_zero_equal_becomes_eq_empty() {
    let (out, changed) = try_strlen_simplify_expr("[string length $s] == 0", None);
    assert!(changed);
    assert!(out.contains("eq") && out.contains("\"\""));
}

#[test]
fn strlen_nonzero_literal_not_rewritten() {
    let (out, changed) = try_strlen_simplify_expr("[string length $s] == 1", None);
    assert!(!changed);
    assert_eq!(out, "[string length $s] == 1");
}

// try_eq_ne_string_compare_simplify_expr

#[test]
fn streq_promotion_with_quoted_literal() {
    let (out, changed) = try_eq_ne_string_compare_simplify_expr("$x == \"foo\"", None);
    assert!(changed);
    assert!(out.contains("eq"));
}

#[test]
fn streq_promotion_with_numeric_literal_noop() {
    // `$x == 5` is a legitimate numeric compare — don't
    // promote to `eq`.
    let (out, changed) = try_eq_ne_string_compare_simplify_expr("$x == 5", None);
    assert!(!changed);
    assert_eq!(out, "$x == 5");
}

#[test]
fn streq_promotion_with_numeric_string_literal_is_unsound_noop() {
    // `$x == "1"` must stay numeric. `"1"` parses as a
    // number, so Tcl runs the numeric compare; promoting to `eq`
    // would flip the result when `$x` is numeric (e.g. `1.0`).
    // `"1"`/`"3.5"` are numeric; `"yes"` is a Tcl boolean word that
    // `==` still treats as number-ish — none may promote.
    for input in ["$x == \"1\"", "$x != \"3.5\"", "$x == \"yes\""] {
        let (out, changed) = try_eq_ne_string_compare_simplify_expr(input, None);
        assert!(!changed, "{input:?} should not promote to eq/ne");
        assert_eq!(out, input);
    }
}

#[test]
fn streq_promotion_with_nonnumeric_string_literal() {
    // At least one operand is a provably non-numeric string literal,
    // so the string-compare path is guaranteed — promotion is sound.
    for input in ["$x == \"foo\"", "\"a\" != $y", "$x == \"\""] {
        let (_out, changed) = try_eq_ne_string_compare_simplify_expr(input, None);
        assert!(changed, "{input:?} should promote to eq/ne");
    }
}

// instcombine_expr (composite)

#[test]
fn o110_logical_identity_boolifies_outside_bool_context() {
    // `$x && 1` at expression-value position must normalise to `0`/`1`,
    // not the operand value — so it becomes `!!$x`, never bare `$x`.
    let (out, changed) = instcombine_expr("$x && 1", false);
    assert!(changed);
    assert_eq!(out.trim(), "!!$x");
    let (out, changed) = instcombine_expr("$x || 0", false);
    assert!(changed);
    assert_eq!(out.trim(), "!!$x");
}

#[test]
fn o110_logical_identity_drops_in_bool_context() {
    // In a boolean context the truthiness is all that is consumed, so
    // `$x && 1` collapses to bare `$x`.
    let (out, changed) = instcombine_expr("$x && 1", true);
    assert!(changed);
    assert_eq!(out.trim(), "$x");
}

#[test]
fn o110_logical_identity_keeps_boolean_operand_bare() {
    // `($a < $b) && 1` — the operand is already a `0`/`1` comparison,
    // so it stays bare even outside a boolean context.
    let (out, changed) = instcombine_expr("$a < $b && 1", false);
    assert!(changed);
    assert_eq!(out.trim(), "$a < $b");
}

#[test]
fn o110_logical_absorbing_still_folds() {
    // The absorbing cases must keep folding to a constant.
    assert_eq!(instcombine_expr("$x && 0", false).0.trim(), "0");
    assert_eq!(instcombine_expr("$x || 1", false).0.trim(), "1");
}

#[test]
fn instcombine_composes_multiple_rewrites() {
    // `$x * 1 + 0` should simplify to `$x` via two passes:
    // `$x * 1` → `$x`, then `$x + 0` → `$x`.
    let (out, changed) = instcombine_expr("$x * 1 + 0", false);
    assert!(changed);
    assert_eq!(out.trim(), "$x");
}

#[test]
fn instcombine_raw_expression_left_alone() {
    let (out, changed) = instcombine_expr("$x + $y", false);
    assert!(!changed);
    assert_eq!(out, "$x + $y");
}

#[test]
fn o110_reassociates_constant_chains() {
    // Additive and multiplicative constant reassociation (O110), over
    // terms the type lattice proves integer.
    let integer = OperandTypes::integer(&["a", "b"]);
    for (input, want) in [
        ("$a + 1 + 2", "$a + 3"),
        ("$a * 2 * 3", "$a * 6"),
        ("$a + 3 - 1", "$a + 2"),
        ("$a - 1 - 2", "$a - 3"),
        ("5 + $a - 2", "$a + 3"),
        ("$a + $b + 1 + 2", "$a + $b + 3"),
        // `$a + $b + 1 - 1` still folds the constant: two terms survive, so
        // `$a + $b` keeps a `+` that coerces both operands.
        ("$a + $b + 1 - 1", "$a + $b"),
    ] {
        let (out, changed) = instcombine_expr_typed(input, false, Some(&integer), None);
        assert!(changed, "expected a rewrite for {input:?}");
        assert_eq!(out.trim(), want, "for {input:?}");
    }
}

/// Regrouping consumes the type proof: over a term the lattice proves
/// integer the chain regroups, and over an unproven term it is left as
/// written, because a double's rounding is order-dependent — `set x
/// 10000000000000000.0; expr {$x + 1 + 2}` prints `10000000000000002.0`
/// under tclsh 8.5 to 9.1, and `expr {$x + 3}` prints
/// `10000000000000004.0` (8.4 prints `1e+16` for both at its default
/// precision). A closed constant subtree still folds.
#[test]
fn reassociation_refuses_an_unproven_float_term() {
    let integer = OperandTypes::integer(&["i"]);
    let (out, changed) = instcombine_expr_typed("$i + 1 + 2", false, Some(&integer), None);
    assert!(changed);
    assert_eq!(out.trim(), "$i + 3");
    for (context, label) in [
        (OperandTypes::default(), "untyped"),
        (OperandTypes::numeric_only(&["x"]), "double"),
    ] {
        for input in ["$x + 1 + 2", "$x * 3 * 5"] {
            assert!(
                reassociate_node(&parse_expr_for_profile(input, None), &context).is_none(),
                "{label}: {input}"
            );
            let (out, changed) = instcombine_expr_typed(input, false, Some(&context), None);
            assert!(!changed, "{label}: {input} became {out}");
        }
    }
    let (out, changed) = instcombine_expr("2 + 3 + $x", false);
    assert!(changed);
    assert_eq!(out.trim(), "5 + $x");
}

#[test]
fn o110_additive_lone_term_cancel_to_zero_abstains() {
    // `$a + 1 - 1` cancels to a lone bare `$a`, dropping the
    // numeric-coercion error `$a + 1 - 1` raises for a non-numeric `$a`.
    // Mirror the multiplicative `$a * 1` guard and abstain (the AST pass
    // has no numeric proof).
    assert!(
        reassociate_node(
            &parse_expr_for_profile("$a + 1 - 1", None),
            &OperandTypes::integer(&["a"])
        )
        .is_none(),
        "lone additive term cancelling to zero must not fold to a bare term",
    );
    let (out, changed) = instcombine_expr("$a + 1 - 1", false);
    assert!(!changed, "`$a + 1 - 1` must be left unchanged, got {out:?}");
}

#[test]
fn o110_skips_pure_reorder_and_unsafe_drops() {
    // No additive chain to flatten → the reassociation does not fire
    // (a bare `1 + $a` reorder is left to other passes / suppressed).
    assert!(
        reassociate_node(
            &parse_expr_for_profile("1 + $a", None),
            &OperandTypes::integer(&["a"])
        )
        .is_none(),
        "reassociation must not fire on a non-chain reorder",
    );
    // `* 0` annihilation across a chain needs a numeric proof this
    // AST-level pass cannot make, so the reassociation abstains (the
    // result, if any, comes from the separate identity pass, not here).
    let parsed = parse_expr_for_profile("$a * 0 * 3", None);
    assert!(
        reassociate_node(&parsed, &OperandTypes::integer(&["a"])).is_none(),
        "reassociation must not annihilate `* 0` without a numeric proof",
    );
}

#[test]
fn instcombine_whitespace_only_rerender_is_not_a_change() {
    // A re-render differing from the input
    // only in whitespace (`$x<0` → `$x < 0`) is canonical-spacing
    // noise, not an O110 finding. The guard strips all whitespace
    // before comparing, so the spacing-only re-render reports no
    // change.
    let (out, changed) = instcombine_expr("$x<0", false);
    assert!(
        !changed,
        "whitespace-only re-render must not count as changed (got {out:?})",
    );
    // The narrow no-op case (input already canonical) also reports
    // no change — unchanged from before the guard.
    let (_, unchanged) = instcombine_expr("$x + $y", false);
    assert!(!unchanged);
    // A real structural change (identity fold) still fires.
    let (reduced, structural) = instcombine_expr("$x * 1", false);
    assert!(
        structural,
        "identity fold `$x * 1` → `$x` must still register as changed",
    );
    assert_eq!(reduced.trim(), "$x");
}

#[test]
fn strip_ws_removes_all_whitespace() {
    assert_eq!(strip_ws("$x < 0"), "$x<0");
    assert_eq!(strip_ws("  a  b\tc\n"), "abc");
    assert_eq!(strip_ws("nospace"), "nospace");
}

#[test]
fn mathematical_string_projection_keeps_literal_substitutions_and_braces_inert() {
    let value = "$literal [danger] {unterminated";
    let constants = consts(&[("s", value)]);
    let result = substitute_expr_constants("$s", &constants, None);
    assert!(result.changed);
    assert!(result.substituted.contains("s"));
    assert!(result.text.contains("\\$literal"));
    assert!(result.text.contains("\\[danger\\]"));
    assert!(result.text.contains("\\{unterminated"));
    let context = tcl_syntax::expr::parser::ExprParseContext::for_profile(
        tcl_dialect::DialectProfile::plain_tcl(),
    );
    let original = tcl_syntax::expr::OriginalSourceExpression::capture(
        &result.text,
        &context,
        tcl_lexer::LexerConfig::from_grammar(context.lexer_grammar),
    )
    .unwrap();
    assert_eq!(original.source(), result.text);
    let substitutions =
        tcl_syntax::expr::checked_source_expression_substitutions(&result.text, &context).unwrap();
    assert_eq!(substitutions.variables, [] as [tcl_lexer::Span; 0]);
    assert_eq!(substitutions.commands, [] as [tcl_lexer::Span; 0]);
}

#[test]
fn reassociation_uses_the_actual_original_reference_type_key() {
    let reference = "${a}";
    let keys = [reference.to_owned()].into_iter().collect::<HashSet<_>>();
    let types = OperandTypes {
        numeric: keys.clone(),
        integer: keys,
        ..OperandTypes::default()
    };
    let source = "${a} + 1 + 2";
    let (rewritten, changed) = instcombine_expr_typed(source, false, Some(&types), None);
    assert!(changed);
    assert_eq!(rewritten.trim(), "${a} + 3");
    let (other, changed) = instcombine_expr_typed("$a + 1 + 2", false, Some(&types), None);
    assert!(!changed);
    assert_eq!(other, "$a + 1 + 2");
}
