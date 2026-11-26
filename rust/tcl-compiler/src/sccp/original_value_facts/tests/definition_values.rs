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

//! The value one definition takes: constants, expressions, `incr` and variable references under the selected grammar.

use super::*;

/// Whitespace inside a Tcl word is data, so the lattice keeps the exact
/// spelling (#2052).
///
/// Trimming corrupted every consumer: `set p { again}` reached the lattice
/// as `again`, so `append s $p` was rewritten to `append s again` and the
/// program printed `helloagain` where tclsh 9.0.4 prints `hello again`.
/// `string length $p` folded to 5 against the true 6.
#[test]
fn a_literal_keeps_its_surrounding_whitespace() {
    assert_eq!(
        parse_literal_value(" again"),
        ConstValue::String(" again".into()),
    );
    assert_eq!(
        parse_literal_value("trailing "),
        ConstValue::String("trailing ".into()),
    );
    // An integer with whitespace around it is not the integer: rendering
    // it back as `5` would drop the space just as surely.
    assert_eq!(parse_literal_value(" 5"), ConstValue::String(" 5".into()),);
    // The bare integer still folds.
    assert_eq!(parse_literal_value("5"), ConstValue::Int(5));
    assert_eq!(parse_literal_value("-17"), ConstValue::Int(-17));
}

#[test]
fn evaluate_def_assign_const_produces_int_or_string() {
    let mut ssa = bare_ssa();
    let s_int = assign_const_stmt(&mut ssa, "x", "42", 1);
    assert_eq!(
        evaluate_fixture_def(
            &s_int,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(42))
    );
    let s_str = assign_const_stmt(&mut ssa, "x", "hello", 1);
    assert_eq!(
        evaluate_fixture_def(
            &s_str,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::String("hello".into()))
    );
}

#[test]
fn evaluate_def_assign_expr_retains_numeric_precision_and_conversion() {
    let unit = cu("set a 2; set x [expr {$a + 3}]");
    let facts = unit.top_level.semantic_values().unwrap();
    let analysis = facts
        .expression_iter()
        .find_map(|(_, analysis)| {
            (analysis.evaluation.value == TclValue::Int(5)).then_some(analysis)
        })
        .expect("the actual prepared expression computes 5");
    assert_ne!(
        analysis.evaluation.coercions,
        [] as [crate::tcl_expr_eval::NativeCoercionObligation; 0]
    );
    assert!(
        !unit
            .top_level
            .sccp
            .values
            .values()
            .any(|value| { *value == LatticeValue::Const(ConstValue::Int(5)) }),
        "a contents value alone cannot erase its retained-object conversion"
    );
}

/// SCCP folds the iRules word operators when — and only when — the
/// policy says the dialect has them.
///
/// Evaluating through a dialect-blind entry point (a bare
/// `octal: Option<bool>`) leaves `FoldOps::is_irules` `false`, so every
/// word operator silently declines while the `eq` control on the same
/// shape folds, because plain Tcl shares it.
#[test]
fn evaluate_def_folds_irules_word_operator_only_under_an_irules_policy() {
    let mut ssa = bare_ssa();
    let subject = ssa.intern_var("s");
    let out = ssa.intern_var("hit");
    let mut uses = HashMap::new();
    uses.insert(subject, 1);
    let mut defs = HashMap::new();
    defs.insert(out, 1);

    let expr = ExprNode::Binary {
        op: crate::expr_ast::BinOp::Contains,
        left: Box::new(ExprNode::Var {
            text: "$s".into(),
            name: "s".into(),
            start: 0,
            end: 2,
        }),
        right: Box::new(ExprNode::String {
            text: "cd".into(),
            start: 3,
            end: 7,
        }),
    };
    let stmt_ssa = SsaStatement {
        statement: Statement::AssignExpr {
            span: Span::new(0, 0),
            name: "hit".into(),
            name_braced: false,
            expr,
            command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
            expr_base: None,
            fallback_value: "[expr {$s contains \"cd\"}]".into(),
        },
        uses,
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    };
    let mut values = HashMap::new();
    values.insert(
        (subject, 1),
        LatticeValue::Const(ConstValue::String("abcde".into())),
    );

    let irules = FoldPolicy::for_profile(Some(true), Some(tcl_dialect::DialectProfile::irules()));
    assert_eq!(
        evaluate_fixture_def(
            &stmt_ssa,
            &values,
            &ssa,
            irules,
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(1)),
        "`$s contains \"cd\"` with $s = abcde must fold to 1 under an iRules policy"
    );
    // TN: without the dialect fact the fold is declined, not guessed.
    assert_eq!(
        evaluate_fixture_def(
            &stmt_ssa,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined,
        "a dialect-blind policy must decline the word-operator fold"
    );
}

// evaluate_fixture_def for Incr.

fn incr_stmt(
    ssa: &mut SsaFunction,
    name: &str,
    amount: Option<&str>,
    old_ver: u32,
    new_ver: u32,
) -> SsaStatement {
    let sym = ssa.intern_var(name);
    let mut uses = HashMap::new();
    uses.insert(sym, old_ver);
    let mut defs = HashMap::new();
    defs.insert(sym, new_ver);
    SsaStatement {
        statement: Statement::Incr {
            span: Span::new(0, 0),
            name: name.into(),
            name_braced: false,
            amount: amount.map(String::from),
            amount_braced: false,
            safe_on_uninit: false,
        },
        uses,
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

#[test]
fn evaluate_def_incr_default_amount() {
    // x@1 = Const(Int(5)); `incr x` → x@2 = Const(Int(6)).
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(5)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(6))
    );
}

#[test]
fn evaluate_def_incr_integer_literal_amount() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("10"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(3)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(13))
    );
}

#[test]
fn evaluate_def_incr_negative_literal_amount() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("-2"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(10)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(8))
    );
}

#[test]
fn evaluate_def_incr_var_ref_amount() {
    // `incr x $y` where $y resolves to 4.
    let mut ssa = bare_ssa();
    let mut stmt = incr_stmt(&mut ssa, "x", Some("$y"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let y = ssa.intern_var("y");
    stmt.uses.insert(y, 1);
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(6)));
    values.insert((y, 1), LatticeValue::Const(ConstValue::Int(4)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(10))
    );
}

#[test]
fn evaluate_def_incr_unknown_base_propagates_unknown() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
    let values = HashMap::new();
    // No entry for x@1 → base is Unknown → result Unknown.
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Unknown
    );
}

#[test]
fn evaluate_def_incr_overdefined_base_widens() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", None, 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Overdefined);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined
    );
}

#[test]
fn evaluate_def_incr_non_integer_amount_widens() {
    let mut ssa = bare_ssa();
    let stmt = incr_stmt(&mut ssa, "x", Some("2.5"), 1, 2);
    let x = ssa.var_symbol("x").unwrap();
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(1)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined
    );
}

fn assert_profile_reference_shapes(
    profile: &str,
    config: tcl_lexer::LexerConfig,
    rules: tcl_syntax::word_rules::WordValueRules,
    uses: &HashMap<Symbol, crate::ssa::Version>,
    values: &HashMap<ValueKey, LatticeValue>,
    ssa: &SsaFunction,
) {
    assert_eq!(
        resolve_simple_var_ref("$café", uses, values, ssa, Some(config)).is_some(),
        profile == "jim"
    );
    assert_eq!(
        resolve_simple_var_ref("${a{b}c}", uses, values, ssa, Some(config)).is_some(),
        matches!(profile, "tcl9.0" | "tcl9.1")
    );
    assert_eq!(
        resolve_simple_var_ref("$a", &HashMap::new(), values, ssa, Some(config)),
        None
    );
    assert_eq!(
        resolve_foreach_list_via_lattice("$a", &HashMap::new(), values, ssa, rules, Some(config)),
        None
    );
}

#[test]
fn original_value_reference_consumers_require_grammar_and_scalar_shape() {
    // Original inputs: experiments/name-handling/oracles/whole-variable-reference-shapes-20261014.
    let mut ssa = bare_ssa();
    let mut uses = HashMap::new();
    let mut values = HashMap::new();
    for name in ["a", "a:b", "café", "", "scalar(open", "arr(key)", "a{b}c"] {
        let symbol = ssa.intern_var(name);
        uses.insert(symbol, 1);
        values.insert(
            (symbol, 1),
            LatticeValue::Const(ConstValue::String("one two".into())),
        );
    }
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let config = tcl_lexer::LexerConfig::for_dialect(profile);
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
        for text in ["$a", "${}", "${scalar(open}"] {
            assert!(
                resolve_simple_var_ref(text, &uses, &values, &ssa, Some(config)).is_some(),
                "{profile}: {text}"
            );
            assert_eq!(
                resolve_foreach_list_via_lattice(text, &uses, &values, &ssa, rules, Some(config)),
                Some(vec!["one".into(), "two".into()]),
                "{profile}: {text}"
            );
            assert_eq!(
                resolve_const_string(
                    text,
                    &uses,
                    &values,
                    &ssa,
                    FoldPolicy::default(),
                    Some(config)
                ),
                Some("one two".into()),
                "{profile}: {text}"
            );
            assert_eq!(
                resolve_simple_var_ref(text, &uses, &values, &ssa, None),
                None
            );
            assert_eq!(
                resolve_foreach_list_via_lattice(text, &uses, &values, &ssa, rules, None),
                None
            );
            assert_eq!(
                resolve_const_string(text, &uses, &values, &ssa, FoldPolicy::default(), None),
                None
            );
        }
        for text in ["$a:b", "$arr(key)", "${arr(key)}", "$a$b"] {
            assert_eq!(
                resolve_simple_var_ref(text, &uses, &values, &ssa, Some(config)),
                None,
                "{profile}: {text}"
            );
            assert_eq!(
                resolve_foreach_list_via_lattice(text, &uses, &values, &ssa, rules, Some(config)),
                None,
                "{profile}: {text}"
            );
            assert_eq!(
                resolve_const_string(
                    text,
                    &uses,
                    &values,
                    &ssa,
                    FoldPolicy::default(),
                    Some(config)
                ),
                None,
                "{profile}: {text}"
            );
        }
        assert_profile_reference_shapes(profile, config, rules, &uses, &values, &ssa);
    }
}

#[test]
fn resolve_simple_var_ref_accepts_bare_and_braced() {
    let mut ssa = bare_ssa();
    let x = ssa.intern_var("x");
    let mut uses = HashMap::new();
    uses.insert(x, 1);
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(7)));
    assert_eq!(
        resolve_simple_var_ref(
            "$x",
            &uses,
            &values,
            &ssa,
            Some(tcl_lexer::LexerConfig::default())
        ),
        Some(LatticeValue::Const(ConstValue::Int(7)))
    );
    assert_eq!(
        resolve_simple_var_ref(
            "${x}",
            &uses,
            &values,
            &ssa,
            Some(tcl_lexer::LexerConfig::default())
        ),
        Some(LatticeValue::Const(ConstValue::Int(7)))
    );
    assert_eq!(
        resolve_simple_var_ref(
            "$y",
            &uses,
            &values,
            &ssa,
            Some(tcl_lexer::LexerConfig::default())
        ),
        None
    );
    assert_eq!(
        resolve_simple_var_ref(
            "plain",
            &uses,
            &values,
            &ssa,
            Some(tcl_lexer::LexerConfig::default())
        ),
        None
    );
}
