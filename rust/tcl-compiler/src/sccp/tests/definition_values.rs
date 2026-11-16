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

//! The value one definition takes: constants, expressions and fused expressions, route explanations and variable references.

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
        ConstValue::String(" again".to_owned()),
    );
    assert_eq!(
        parse_literal_value("trailing "),
        ConstValue::String("trailing ".to_owned()),
    );
    // An integer with whitespace around it is not the integer: rendering
    // it back as `5` would drop the space just as surely.
    assert_eq!(
        parse_literal_value(" 5"),
        ConstValue::String(" 5".to_owned()),
    );
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
fn evaluate_def_assign_expr_folds_with_lattice() {
    // `set x [expr {$a + 3}]` with `a` the constant 2 settles 5.
    assert_eq!(
        settled(
            "tcl8.6",
            "proc p {} {set a 2; set x [expr {$a + 3}]; return $x}",
            "x"
        ),
        LatticeValue::Const(ConstValue::Int(5))
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

/// A fused `set r [expr {TEXT}]` under `profile`'s expression grammar.
fn fused_expr_stmt(
    ssa: &mut SsaFunction,
    text: &str,
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> SsaStatement {
    let mut defs = HashMap::new();
    defs.insert(ssa.intern_var("r"), 1);
    SsaStatement {
        statement: Statement::AssignExpr {
            span: Span::new(0, 0),
            name: "r".into(),
            name_braced: false,
            expr: crate::expr_parser::parse_expr_for_profile(text, profile),
            command_binding: Some(tcl_runtime_api::CommandBindingIdentity::new("expr", "expr")),
            expr_base: None,
            fallback_value: format!("[expr {{{text}}}]"),
        },
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    }
}

/// `expr` answers its full value, not only a number: a string result is
/// the result, and a string that reads as a number under the target's
/// grammar is that number's canonical form. Oracle, tclsh 8.4 to 9.1:
/// `expr {"x"}` is `x`, `expr {1 ? "yes" : "no"}` is `yes`, `expr
/// {"true"}` is `true`, `expr {" 5 "}` is 5, `expr {"0x10"}` is 16,
/// `expr {"1.50"}` is 1.5 and `expr {1/0}` raises; `expr {"010"}` is 8 up
/// to 8.6 and 10 from 9.0; `expr {1 << 70}` is
/// `1180591620717411303424` from 8.5 and 0 under 8.4, so a target that
/// does not widen declines it.
#[test]
fn a_fused_expression_answers_its_full_value() {
    let text = |s: &str| LatticeValue::Const(ConstValue::String(s.to_owned()));
    let int = |i: i64| LatticeValue::Const(ConstValue::Int(i));
    let under = |name: &str| {
        let profile = tcl_dialect::DialectProfile::find(name).expect(name);
        (
            Some(profile),
            FoldPolicy::for_profile(
                crate::tcl_expr_eval::leading_zero_is_octal(profile),
                Some(profile),
            ),
        )
    };
    let cases: [(&str, &str, LatticeValue); 14] = [
        ("tcl8.4", r#""x""#, text("x")),
        ("f5-irules", r#""x""#, text("x")),
        ("tcl9.0", r#"1 ? "yes" : "no""#, text("yes")),
        ("tcl8.6", r#""true""#, text("true")),
        ("tcl8.6", r#"" 5 ""#, int(5)),
        ("tcl9.0", r#""0x10""#, int(16)),
        (
            "tcl8.6",
            r#""1.50""#,
            LatticeValue::Const(ConstValue::Float(1.5)),
        ),
        ("tcl8.6", r#""010""#, int(8)),
        ("tcl9.0", r#""010""#, int(10)),
        ("tcl8.6", r#""08""#, text("08")),
        ("tcl9.0", "1/0", LatticeValue::Overdefined),
        ("tcl8.6", "2 ** 64", text("18446744073709551616")),
        ("tcl9.0", "[string length abc] * 2", int(6)),
        ("tcl9.0", "0 && [error never]", int(0)),
    ];
    for (dialect, expression, want) in cases {
        let (profile, policy) = under(dialect);
        let mut ssa = bare_ssa();
        let stmt = fused_expr_stmt(&mut ssa, expression, profile);
        assert_eq!(
            evaluate_pristine(&stmt, &HashMap::new(), &ssa, policy),
            want,
            "{dialect}: expr {{{expression}}}"
        );
    }
    // No leading-zero rule at all: the two readings of `010` disagree.
    let mut ssa = bare_ssa();
    let stmt = fused_expr_stmt(&mut ssa, r#""010""#, None);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::from_octal(None)),
        LatticeValue::Overdefined
    );
}

/// A shift past the wide boundary is the release's own answer. tclsh
/// 8.4.20 prints 0 for `puts [expr {1 << 70}]`, where the count runs off
/// its fixed-width integer; tclsh 8.5 to 9.1 print
/// 1180591620717411303424. The `f5-irules` row waits on BIG-IP check 10
/// and on handler bodies lowering with plain names.
#[test]
fn a_wide_shift_settles_what_each_release_prints() {
    let big = LatticeValue::Const(ConstValue::String("1180591620717411303424".into()));
    for (dialect, want) in [
        ("tcl8.4", LatticeValue::Const(ConstValue::Int(0))),
        ("tcl8.5", big.clone()),
        ("tcl8.6", big.clone()),
        ("tcl9.0", big.clone()),
        ("tcl9.1", big),
    ] {
        assert_eq!(
            settled(
                dialect,
                "proc p {} {set r [expr {1 << 70}]; return $r}",
                "r"
            ),
            want,
            "{dialect}"
        );
    }
}

/// The run records each statement's route and answer for the
/// Explorer, at the fixed point.
#[test]
fn sccp_records_a_route_explanation_per_statement() {
    let registry = registry();
    let cu = crate::compilation_unit::CompilationUnit::build_for(
        "proc p {} {\n    set n 1\n    incr n\n    append s x\n    set r [string range abc 0 1]\n    return $n\n}\n",
        &registry,
        false,
    );
    let fu = fixture_id_if_present(&cu.procedures, "::p")
        .and_then(|id| cu.procedures.get(&id))
        .expect("proc");
    let explained: Vec<(String, String)> = fu
        .sccp
        .explanations
        .iter()
        .map(|e| (e.command.clone(), e.answer.clone()))
        .collect();
    assert!(
        explained.contains(&("incr".to_owned(), "evaluated".to_owned())),
        "{explained:?}"
    );
    // The existence rung proves `s` unbound, and `append` creates its
    // cell in every release, so the absent cell evaluates
    // (`the_absent_cell_release_table`: `append s foo` is `foo` under
    // tclsh 8.4 to 9.1).
    assert!(
        explained.contains(&("append".to_owned(), "evaluated".to_owned())),
        "an append over an unbound cell creates it: {explained:?}"
    );
    assert!(
        fu.sccp
            .explanations
            .iter()
            .any(|e| e.command == "string" && e.route.starts_with("direct string-range")),
        "{:?}",
        fu.sccp.explanations
    );
}

#[test]
fn lattice_reference_shapes_keep_actual_grammar_and_refuse_compound_or_array_names() {
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
        for source in ["$a", "${}", "${scalar(open}"] {
            assert!(
                resolve_simple_var_ref(source, &uses, &values, &ssa, config).is_some(),
                "{profile}: {source}"
            );
            assert_eq!(
                resolve_foreach_list_via_lattice(source, &uses, &values, &ssa, rules, config),
                Some(vec!["one".into(), "two".into()]),
                "{profile}: {source}"
            );
        }
        for source in ["$a:b", "${arr(key)}", "$arr(key)", "$a$b"] {
            assert_eq!(
                resolve_simple_var_ref(source, &uses, &values, &ssa, config),
                None,
                "{profile}: {source}"
            );
            assert_eq!(
                resolve_foreach_list_via_lattice(source, &uses, &values, &ssa, rules, config),
                None,
                "{profile}: {source}"
            );
        }
        assert_eq!(
            resolve_simple_var_ref("$café", &uses, &values, &ssa, config).is_some(),
            profile == "jim"
        );
        assert_eq!(
            resolve_simple_var_ref("${a{b}c}", &uses, &values, &ssa, config).is_some(),
            matches!(profile, "tcl9.0" | "tcl9.1")
        );
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
            tcl_lexer::LexerConfig::default()
        ),
        Some(LatticeValue::Const(ConstValue::Int(7)))
    );
    assert_eq!(
        resolve_simple_var_ref(
            "${x}",
            &uses,
            &values,
            &ssa,
            tcl_lexer::LexerConfig::default()
        ),
        Some(LatticeValue::Const(ConstValue::Int(7)))
    );
    assert_eq!(
        resolve_simple_var_ref(
            "$y",
            &uses,
            &values,
            &ssa,
            tcl_lexer::LexerConfig::default()
        ),
        None
    );
    assert_eq!(
        resolve_simple_var_ref(
            "plain",
            &uses,
            &values,
            &ssa,
            tcl_lexer::LexerConfig::default()
        ),
        None
    );
}

/// The value the semantic facts hold for `name`'s last definition in
/// `::p` of `source` under `dialect`, and whether the execution lattice,
/// which rewrites read with no further check, holds a constant there.
fn value_and_constant(dialect: &str, source: &str, name: &str) -> (LatticeValue, bool) {
    let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
    let unit = crate::compilation_unit::CompilationUnit::build_for_dialect(
        source, registry, false, dialect,
    );
    let function = unit.function("::p").expect("::p is analysed");
    let symbol = function
        .ssa
        .var_symbol(name)
        .expect("the variable is an SSA symbol");
    let key = function
        .sccp
        .values
        .keys()
        .filter(|(sym, _)| *sym == symbol)
        .max_by_key(|(_, version)| *version)
        .copied()
        .expect("a definition");
    let semantic = function
        .semantic_values()
        .and_then(|facts| facts.contents(key).cloned())
        .unwrap_or(LatticeValue::Overdefined);
    (
        semantic,
        matches!(function.sccp.values.get(&key), Some(LatticeValue::Const(_))),
    )
}

fn assert_value_and_constant_rows<const N: usize>(
    rows: [(&str, &str, LatticeValue, bool, &str); N],
) {
    for (dialect, body, value, constant, printed) in rows {
        let source = format!("proc p {{}} {{{body}; return $r}}");
        assert_eq!(
            value_and_constant(dialect, &source, "r"),
            (value, constant),
            "{dialect}: {body} (tclsh prints {printed})"
        );
    }
}

/// One row per input: the value the semantic facts hold, and whether the
/// execution lattice holds a constant. A rewrite may erase an expression
/// only on an execution constant; a value alone is knowledge. The last
/// column is what tclsh prints for `proc p {} {BODY; return $r}; p` under
/// that release (8.4.20, 8.6.18, 9.0.4).
#[test]
fn a_fused_expression_has_a_value_and_maybe_an_execution_constant() {
    let int = |value: i64| LatticeValue::Const(ConstValue::Int(value));
    let text = |value: &str| LatticeValue::Const(ConstValue::String(value.to_owned()));
    let float = |value: f64| LatticeValue::Const(ConstValue::Float(value));
    let none = LatticeValue::Overdefined;
    let rows: [(&str, &str, LatticeValue, bool, &str); 16] = [
        // A comparison of two string variables is false, but the numeric
        // comparison may change the operands' representations.
        (
            "tcl8.6",
            "set a alpha; set b beta; set r [expr {$a == $b}]",
            int(0),
            false,
            "0",
        ),
        // A quoted word is substituted before the expression is parsed.
        (
            "tcl8.6",
            "set a 3; set b 4; set r [expr \"$a + $b\"]",
            int(7),
            false,
            "7",
        ),
        (
            "tcl8.6",
            "set a {1 + 1}; set r [expr \"$a * 2\"]",
            int(3),
            false,
            "3",
        ),
        (
            "tcl8.6",
            "set a {1 + 1}; set r [expr {$a * 2}]",
            none.clone(),
            false,
            "can't use non-numeric string as operand of \"*\"",
        ),
        (
            "tcl9.0",
            "set r [expr {1 ? \"yes\" : \"no\"}]",
            text("yes"),
            true,
            "yes",
        ),
        (
            "tcl8.6",
            "set r [expr {\"true\"}]",
            text("true"),
            false,
            "true",
        ),
        ("tcl8.6", "set r [expr {\" 5 \"}]", int(5), false, "5"),
        ("tcl9.0", "set r [expr {\"0x10\"}]", int(16), false, "16"),
        (
            "tcl8.6",
            "set r [expr {\"1.50\"}]",
            float(1.5),
            false,
            "1.5",
        ),
        ("tcl8.6", "set r [expr {\"010\"}]", int(8), false, "8"),
        ("tcl9.0", "set r [expr {\"010\"}]", int(10), false, "10"),
        (
            "tcl9.0",
            "set r [expr {1/0}]",
            none,
            false,
            "divide by zero",
        ),
        (
            "tcl8.6",
            "set r [expr {2 ** 64}]",
            text("18446744073709551616"),
            true,
            "18446744073709551616",
        ),
        (
            "tcl9.0",
            "set r [expr {0 && [error never]}]",
            int(0),
            false,
            "0",
        ),
        // A double from an executed operator keeps its operation under
        // 8.4, whose spelling follows `tcl_precision`.
        (
            "tcl8.4",
            "set r [expr {1 / 4.0}]",
            float(0.25),
            false,
            "0.25",
        ),
        (
            "tcl8.6",
            "set r [expr {1 / 4.0}]",
            float(0.25),
            true,
            "0.25",
        ),
    ];
    assert_value_and_constant_rows(rows);
}
