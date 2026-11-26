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

//! Folding an assigned value word: literals, variable references, pure command substitutions and quoted or braced `expr`.

use super::*;

#[test]
fn evaluate_def_assign_value_plain_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "hello", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::String("hello".into()))
    );
}

#[test]
fn evaluate_def_assign_value_integer_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "42", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(42))
    );
}

#[test]
fn evaluate_def_assign_value_resolves_var_ref() {
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "y", "$x", 1);
    let x = ssa.intern_var("x");
    stmt.uses.insert(x, 1);
    let mut values = HashMap::new();
    values.insert((x, 1), LatticeValue::Const(ConstValue::Int(7)));
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &values,
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Const(ConstValue::Int(7))
    );
}

#[test]
fn evaluate_def_assign_value_folds_list_cmd() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[list a b c]", 1);
    let result = evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default());
    match result {
        LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s.as_str(), "a b c"),
        other => panic!("expected Const(String), got {other:?}"),
    }
}

#[test]
fn evaluate_def_assign_value_folds_llength_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[llength {a b c d}]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(4))
    );
}

#[test]
fn evaluate_def_assign_value_folds_string_length() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(5))
    );
}

#[test]
fn string_length_fold_counts_in_the_selected_dialects_character_model() {
    // U+1D11E written in a UTF-8 source file: tclsh 9.0 and 9.1 count one
    // scalar; 8.6 counts 4 under `encoding system iso8859-1` and 2 under
    // utf-8, so an 8.x fold declines, as does no selected release.
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "n", "[string length \"\u{1D11E}\"]", 1);
    let fold = |dialect: Option<&'static tcl_dialect::DialectProfile>| {
        evaluate_pristine(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::for_profile(Some(false), dialect),
        )
    };
    assert_eq!(
        fold(Some(
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile()
        )),
        LatticeValue::Const(ConstValue::Int(1))
    );
    assert_eq!(
        fold(Some(
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
        )),
        LatticeValue::Overdefined
    );
    assert_eq!(
        fold(None),
        LatticeValue::Overdefined,
        "no selected release leaves a supplementary width ambiguous"
    );

    // A string both models count identically still folds with no selected
    // release — declining those would drop an ordinary optimisation.
    let mut ssa = bare_ssa();
    let ascii = assign_value_stmt(&mut ssa, "n", "[string length \"hello\"]", 1);
    assert_eq!(
        evaluate_pristine(
            &ascii,
            &HashMap::new(),
            &ssa,
            FoldPolicy::for_profile(Some(false), None)
        ),
        LatticeValue::Const(ConstValue::Int(5))
    );
}

#[test]
fn evaluate_def_assign_value_folds_expr_cmd_subst() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[expr {1 + 2}]", 1);
    assert_eq!(
        evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(3))
    );
}

#[test]
fn evaluate_def_assign_value_folds_format_literal() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "s", "[format \"%d-%d\" 1 2]", 1);
    match evaluate_pristine(&stmt, &HashMap::new(), &ssa, FoldPolicy::default()) {
        LatticeValue::Const(ConstValue::String(s)) => assert_eq!(s.as_str(), "1-2"),
        other => panic!("expected Const(String), got {other:?}"),
    }
}

#[test]
fn quoted_expr_with_string_var_does_not_fold() {
    // `set r [expr "$a == $b"]` with a="alpha", b="beta": Tcl substitutes
    // the values textually before parsing, so `expr "alpha == beta"`
    // errors (`invalid bareword`). The fold must bail rather than treat
    // the strings as operands and return 0.
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "r", "[expr \"$a == $b\"]", 1);
    let a = ssa.intern_var("a");
    let b = ssa.intern_var("b");
    stmt.uses.insert(a, 1);
    stmt.uses.insert(b, 1);
    let mut values = HashMap::new();
    values.insert(
        (a, 1),
        LatticeValue::Const(ConstValue::String("alpha".into())),
    );
    values.insert(
        (b, 1),
        LatticeValue::Const(ConstValue::String("beta".into())),
    );
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
fn quoted_expr_with_numeric_var_retains_known_contents() {
    let unit = cu(r#"set a 3; set b 4; set r [expr "$a + $b"]"#);
    let facts = unit.top_level.semantic_values().unwrap();
    let known_seven = facts
        .contents_iter()
        .any(|(_, value)| *value == LatticeValue::Const(ConstValue::Int(7)));
    if !known_seven {
        eprintln!(
            "quoted expr statement kinds={:?} failure={} admission={}",
            unit.ir_module
                .top_level
                .statements
                .iter()
                .map(std::mem::discriminant)
                .collect::<Vec<_>>(),
            unit.ir_module
                .top_level
                .native_compilation_failure
                .is_some(),
            unit.ir_module
                .top_level
                .native_compilation_admission
                .is_some()
        );
        let show = |binding: &crate::command_binding::SourceInvocationBinding| {
            format!(
                "targets={:?} unknown={} absent={} residual={:?} handler={:?} bindings={} traces={} argv={:?} compiled={:?}",
                binding
                    .targets
                    .iter()
                    .map(|target| (&target.command, target.registry_backed))
                    .collect::<Vec<_>>(),
                binding.unknown,
                binding.may_be_absent,
                binding.compiled_execution_residual,
                binding
                    .native_handler_envelope
                    .as_ref()
                    .map(|target| &target.command),
                binding.variable_context.dynamic_bindings,
                binding.variable_context.dynamic_traces,
                binding.evaluated_argument_values,
                binding
                    .compiled_candidates
                    .iter()
                    .map(|proof| (proof.operation, proof.certainty))
                    .collect::<Vec<_>>()
            )
        };
        for site in unit.ir_module.top_level.command_binding_sites.iter() {
            if let Some(tokens) = &site.source_tokens {
                eprintln!(
                    "quoted expr point {:?} argv={:?} proof={:?}",
                    site.span,
                    tokens.argv_texts,
                    tokens.source_binding.as_ref().map(show)
                );
                for (offset, binding) in tokens.nested_bindings.iter() {
                    eprintln!("quoted expr nested {offset} {}", show(binding));
                }
            }
        }
        for proof in &unit.top_level.cfg.expression_preparations {
            eprintln!(
                "quoted expr preparation site={} base={} bytes={:?} axes={:?}",
                proof.invocation.offset,
                proof.source.base(),
                proof.witness.source(),
                proof.witness.context()
            );
        }
    }
    assert!(
        known_seven,
        "textual substitution computes native contents 7: {facts:#?}"
    );
    assert!(
        !unit
            .top_level
            .sccp
            .values
            .values()
            .any(|value| { *value == LatticeValue::Const(ConstValue::Int(7)) }),
        "unproved preparation/producer effects do not license erasure"
    );
}

#[test]
fn materialised_condition_values_do_not_license_branch_erasure() {
    let unit = cu("set n 1; if $n {set result YES} else {set result NO}");
    let function = &unit.top_level;
    let known = function.semantic_values().unwrap();
    assert!(
        known
            .result
            .constant_branches
            .iter()
            .any(|branch| { branch.condition == "$n" && branch.value })
    );
    assert_eq!(function.sccp.constant_branches, [] as [ConstantBranch; 0]);
    let analysis = known
        .expression_iter()
        .find_map(|(point, analysis)| {
            matches!(point, ExpressionEvaluationPoint::Branch { .. }).then_some(analysis)
        })
        .expect("the reached condition retains its actual preparation");
    assert_eq!(analysis.required_expression_preparations.len(), 1);
    assert_eq!(
        analysis.required_expression_preparations[0]
            .witness
            .source(),
        "1"
    );
}

#[test]
fn braced_expr_with_string_var_retains_numeric_comparison_obligations() {
    let unit = cu("set a alpha; set b beta; set r [expr {$a == $b}]");
    let facts = unit.top_level.semantic_values().unwrap();
    let analysis = facts
        .expression_iter()
        .find_map(|(_, analysis)| {
            (analysis.evaluation.value == TclValue::Int(0)).then_some(analysis)
        })
        .expect("the actual prepared string comparison computes false");
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
            .any(|value| { *value == LatticeValue::Const(ConstValue::Int(0)) }),
        "numeric comparison may change shared string representations"
    );
}

#[test]
fn evaluate_def_assign_value_unknown_cmd_widens() {
    let mut ssa = bare_ssa();
    let stmt = assign_value_stmt(&mut ssa, "x", "[nonexistent_fold args]", 1);
    assert_eq!(
        evaluate_fixture_def(
            &stmt,
            &HashMap::new(),
            &ssa,
            FoldPolicy::default(),
            tcl_lexer::LexerConfig::default()
        ),
        LatticeValue::Overdefined
    );
}

#[test]
fn evaluate_def_assign_value_llength_via_lattice_var() {
    let mut ssa = bare_ssa();
    let mut stmt = assign_value_stmt(&mut ssa, "n", "[llength $lst]", 1);
    let lst = ssa.intern_var("lst");
    stmt.uses.insert(lst, 1);
    let mut values = HashMap::new();
    values.insert(
        (lst, 1),
        LatticeValue::Const(ConstValue::String("a b c".into())),
    );
    assert_eq!(
        evaluate_pristine(&stmt, &values, &ssa, FoldPolicy::default()),
        LatticeValue::Const(ConstValue::Int(3))
    );
}

#[test]
fn split_head_basic() {
    assert_eq!(split_head("cmd arg1 arg2"), ("cmd", Some("arg1 arg2")));
    assert_eq!(split_head("  cmd"), ("cmd", None));
    assert_eq!(split_head(""), ("", None));
}

#[test]
fn strip_one_level_braces_and_quotes() {
    assert_eq!(strip_one_level("{abc}"), "abc");
    assert_eq!(strip_one_level("\"abc\""), "abc");
    assert_eq!(strip_one_level("bare"), "bare");
    assert_eq!(strip_one_level("{}"), "");
}

#[test]
fn parse_literal_value_prefers_int() {
    assert_eq!(parse_literal_value("42"), ConstValue::Int(42));
    assert_eq!(parse_literal_value("-5"), ConstValue::Int(-5));
    assert_eq!(parse_literal_value("0"), ConstValue::Int(0));
    assert_eq!(
        parse_literal_value("hello"),
        ConstValue::String("hello".into())
    );
    // Leading-zero and non-canonical integer forms do not round-trip, so
    // they stay strings (lets SCCP apply the per-dialect leading-zero rule).
    assert_eq!(parse_literal_value("08"), ConstValue::String("08".into()));
    assert_eq!(parse_literal_value("010"), ConstValue::String("010".into()));
    assert_eq!(parse_literal_value("+5"), ConstValue::String("+5".into()));
    assert_eq!(parse_literal_value("-0"), ConstValue::String("-0".into()));
}
