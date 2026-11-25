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

#[test]
fn analysis_preserves_jim_selected_operand_result_bytes_and_sharing() {
    let jim = tcl_registry::InvocationDialect::of_point(
        tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
    );
    let policy = FoldPolicy::default().with_invocation_dialect(jim);
    let env = Env::from([("x".into(), EnvValue::Str("003".into()))]);
    let original = crate::expr_parser::parse_expr("1?$x:0", None);
    let result =
        analyse_tcl_expr_with_resolved_math_bindings(&original, &env, policy, &|_, _| None, None)
            .unwrap();
    assert_eq!(result.value, TclValue::Int(3));
    assert_eq!(
        result.coercions,
        [] as [crate::tcl_expr_eval::NativeCoercionObligation; 0]
    );
    assert_eq!(
        result.result_dependency,
        Some(NativeExpressionResultDependency::SelectedOperand {
            reference: "$x".into(),
            start: Some(2),
            existing_value: Some(tcl_registry::value_transfer::ExactValue::utf8("003")),
        },)
    );
    assert!(!result.native_value_effects_are_proved());
    assert_eq!(
        eval_tcl_expr_with_resolved_math_bindings(&original, &env, policy, &|_, _| None,),
        None
    );
}

#[test]
fn analysis_retains_values_and_only_reached_coercion_obligations() {
    let policy = FoldPolicy::default().with_invocation_dialect(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    let env = Env::from([("x".into(), EnvValue::Int(3))]);
    let calls = |function: &str, _| {
        Some(NativeMathFunctionTarget {
            function: function.to_owned(),
            prepended: Vec::new(),
        })
    };
    for (source, kind, start) in [
        ("$x+1", NativeCoercionKind::Number, 0),
        ("!!$x", NativeCoercionKind::Boolean, 2),
        ("$x==3", NativeCoercionKind::NumericComparison, 0),
        ("abs($x)", NativeCoercionKind::Number, 4),
        ("$x", NativeCoercionKind::ResultNormalization, 0),
    ] {
        let original = crate::expr_parser::parse_expr(source, None);
        let result =
            analyse_tcl_expr_with_resolved_math_bindings(&original, &env, policy, &calls, None)
                .unwrap();
        assert_eq!(
            result.coercions,
            vec![NativeCoercionObligation {
                reference: "$x".into(),
                start: Some(start),
                kind,
            }],
            "{source}"
        );
        assert!(!result.coercions_are_proved());
    }
    let original = crate::expr_parser::parse_expr("$x+1", None);
    assert_eq!(
        analyse_tcl_expr_with_resolved_math_bindings(&original, &env, policy, &calls, None,)
            .unwrap()
            .value,
        TclValue::Int(4)
    );
    let skipped = crate::expr_parser::parse_expr("1?abs(-3):$x+1", None);
    let result =
        analyse_tcl_expr_with_resolved_math_bindings(&skipped, &env, policy, &calls, None).unwrap();
    assert_eq!(result.value, TclValue::Int(3));
    assert!(result.coercions_are_proved());
    // Mathematical constants cannot turn the original retained read into a
    // fresh object simply because its value happens to be known.
    assert_eq!(
        eval_tcl_expr_with_resolved_math_bindings(&original, &env, policy, &calls,),
        None
    );
}

#[test]
fn known_string_bytes_do_not_erase_an_unproved_operand_materialisation() {
    let policy = FoldPolicy::default().with_invocation_dialect(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    let original = crate::expr_parser::parse_expr("$x eq \"hello\"", None);
    let environment = Env::from([("x".into(), EnvValue::Str("hello".into()))]);
    let analysis = analyse_tcl_expr_with_resolved_math_bindings(
        &original,
        &environment,
        policy,
        &|_, _| None,
        None,
    )
    .unwrap();
    assert_eq!(analysis.value, TclValue::Int(1));
    assert_eq!(
        analysis.coercions,
        vec![NativeCoercionObligation {
            reference: "$x".into(),
            start: Some(0),
            kind: NativeCoercionKind::String,
        }]
    );
    assert!(!analysis.native_value_effects_are_proved());
    assert_eq!(
        eval_tcl_expr_with_resolved_math_bindings(&original, &environment, policy, &|_, _| {
            None
        },),
        None
    );
}

#[test]
fn retained_value_constants_do_not_prove_required_native_coercions() {
    let policy = FoldPolicy::default().with_invocation_dialect(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    let env = Env::from([("x".into(), EnvValue::Int(3))]);
    for source in ["$x", "$x+0", "!!$x", "$x==3", "abs($x)"] {
        let expression = crate::expr_parser::parse_expr(source, None);
        assert_eq!(
            eval_tcl_expr_with_resolved_math_bindings(&expression, &env, policy, &|function, _| {
                Some(NativeMathFunctionTarget {
                    function: function.to_owned(),
                    prepended: Vec::new(),
                })
            },),
            None,
            "a value constant cannot erase a retained-object conversion: {source}",
        );
    }
    let lazy = crate::expr_parser::parse_expr("1?abs(-3):$x+0", None);
    assert_eq!(
        eval_tcl_expr_with_resolved_math_bindings(&lazy, &env, policy, &|function, _| {
            Some(NativeMathFunctionTarget {
                function: function.to_owned(),
                prepended: Vec::new(),
            })
        }),
        Some(TclValue::Int(3)),
    );
}

#[test]
fn already_native_object_proof_licenses_only_matching_numeric_reads() {
    let expression = crate::expr_parser::parse_expr("abs($x)", None);
    let policy = FoldPolicy::default().with_invocation_dialect(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    let env = Env::from([("x".into(), EnvValue::Int(-3))]);
    let mut operands = NativeOperandProofs::from([(
        "x".into(),
        RetainedNativeOperandProof {
            dialect: tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
            identity: NativeOperandObjectIdentity::Runtime {
                interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                    owner: 12,
                    interpreter: 2,
                },
                object: 7,
                generation: 4,
            },
            value: TclValue::Int(-3),
            existing_string: None,
        },
    )]);
    let calls = |function: &str, _| {
        Some(NativeMathFunctionTarget {
            function: function.to_owned(),
            prepended: Vec::new(),
        })
    };
    assert_eq!(
        eval_tcl_expr_with_proved_operands(&expression, &env, policy, &calls, &operands),
        Some(TclValue::Int(3))
    );
    operands.get_mut("x").unwrap().value = TclValue::Int(8);
    assert_eq!(
        eval_tcl_expr_with_proved_operands(&expression, &env, policy, &calls, &operands),
        None
    );
}

#[test]
fn resolved_native_alias_uses_its_terminal_handler_and_frozen_prefix() {
    let expression = crate::expr_parser::parse_expr("chosen()", None);
    let policy = FoldPolicy::default().with_invocation_dialect(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    let folded = eval_tcl_expr_with_resolved_math_bindings(
        &expression,
        &Env::new(),
        policy,
        &|function, start| {
            (function == "chosen" && start == 0).then(|| NativeMathFunctionTarget {
                function: "abs".to_owned(),
                prepended: vec!["-3".to_owned()],
            })
        },
    );
    assert_eq!(folded, Some(TclValue::Int(3)));
    assert_eq!(
        eval_tcl_expr_with_resolved_math_bindings(&expression, &Env::new(), policy, &|_, _| {
            None
        }),
        None
    );
}

#[test]
fn execution_folding_requires_the_reached_native_implementation() {
    let expression = crate::expr_parser::parse_expr("abs(-3)", None);
    let policy = FoldPolicy::default().with_invocation_dialect(
        tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
    );
    assert_eq!(
        eval_tcl_expr_with_policy(&expression, &Env::new(), policy),
        None
    );
    assert_eq!(
        eval_tcl_expr_with_math_bindings(&expression, &Env::new(), policy, &|function, start| {
            function == "abs" && start == 0
        }),
        Some(TclValue::Int(3)),
    );
    assert_eq!(
        eval_tcl_expr_with_math_bindings(&expression, &Env::new(), policy, &|_, _| false),
        None,
    );
}

#[test]
fn unproved_unreached_function_does_not_suppress_a_valid_lazy_fold() {
    let expression = crate::expr_parser::parse_expr("1 ? abs(-3) : abs(-7)", None);
    let reached = std::cell::RefCell::new(Vec::new());
    let folded = eval_tcl_expr_with_math_bindings(
        &expression,
        &Env::new(),
        FoldPolicy::default().with_invocation_dialect(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        ),
        &|function, start| {
            reached.borrow_mut().push((function.to_owned(), start));
            function == "abs" && start == 4
        },
    );
    assert_eq!(folded, Some(TclValue::Int(3)));
    assert_eq!(*reached.borrow(), [("abs".to_owned(), 4)]);
}
