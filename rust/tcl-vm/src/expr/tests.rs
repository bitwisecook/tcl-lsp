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

#[test]
fn original_numeric_comparison_keeps_retired_header_refusal_before_lexical_fallback() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Real header retirement and executable adapter; no external provider outcome.
    let mut vm = crate::native_fixture::core(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile(),
    );
    let original = Value::int(17);
    let retired = original.native_lifetime_lease();
    drop(original);
    assert!(!retired.value().native_object_is_live());
    let right = Value::string("17");
    let before = right.native_object_snapshot();
    let expected = tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
        "retired native object header",
    );
    let error = compare_in(vm.numeric_context(), BinOp::Eq, retired.value(), &right).unwrap_err();
    assert_eq!(error.native_access_refusal(), Some(expected));
    assert_eq!(right.native_object_snapshot(), before);
    // The checked expression path reports the same refusal as the
    // interpreter's retained first cause.
    let mut ops = ExprEval::new(&mut vm);
    let error = ops
        .compare_numeric_checked(retired.value(), &right)
        .unwrap_err();
    assert!(matches!(
        error,
        TclError::Host(crate::error::TclHostFailure::Execution(
            tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(refusal)
        )) if refusal == expected
    ));
    assert_eq!(right.native_object_snapshot(), before);
    assert_eq!(retired.value().native_object_reference_count(), 0);
}

#[test]
fn measured_bare_matches_tree_evaluation_requires_independent_authored_policy() {
    use tcl_native_model::LogicalExpressionParseProvider;
    let mut vm = Vm::new();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    assert!(vm.set_native_engine_profile(crate::environment::profile_for_dialect("tcl9.0")));
    assert!(vm.eval_expr("\"abcd\" matches \"a*\"").is_err());
    assert!(vm.set_logical_expression_parse_provider(
        LogicalExpressionParseProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_quote_provider(
        tcl_native_model::LogicalExpressionQuoteProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_source_word_provider(
        tcl_native_model::LogicalSourceWordProvider::Tcl84CoreSimulation
    ));
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core,
    ));
    for row in include_str!("../../../tcl-syntax/tests/data/f5-matches-21.1.0.1-0.0.26.tsv")
        .lines()
        .take(6)
    {
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        let result = vm.eval_expr(fields[0]);
        if fields[1] == "0" {
            assert_eq!(
                result.unwrap().to_str().as_ref(),
                fields[2],
                "{}",
                fields[0]
            );
        } else {
            assert_eq!(
                result.unwrap_err().message_bytes().unwrap().as_ref(),
                fields[2].as_bytes(),
                "{}",
                fields[0]
            );
        }
    }
    for (expression, expected) in [
        (r#""foobar" contains "oob""#, "1"),
        (r#""foobar" starts_with "foo""#, "1"),
        (r#""foobar" ends_with "foo""#, "0"),
        (r#""foobar" matches_glob "f?o*""#, "1"),
    ] {
        assert_eq!(
            vm.eval_expr(expression).unwrap().to_str().as_ref(),
            expected
        );
    }
    assert!(vm.eval_expr(r#""foobar" matches_regex "o+b""#).is_err());
    for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(name));
        for row in
            include_str!("../../../tcl-syntax/tests/data/f5-matches-21.1.0.1-0.0.26.tsv").lines()
        {
            assert!(
                vm.eval_expr(tcl_test_support::fixture::tsv_fields(row).next().unwrap())
                    .is_err(),
                "{name}: {row}"
            );
        }
    }
}

use super::*;
use crate::command::Command;
use crate::interp::{Vm, ok};
use tcl_runtime_api::Completion;

#[test]
fn c84_reached_expression_matches_seven_original_cache_controls() {
    use tcl_syntax::scalar_getter::{NativeScalarCache as Cache, NativeScalarGetterKind as Getter};
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    let class = |value: &Value| match value.native_scalar_cache() {
        Some(Cache::Tcl84Long(_)) => "int",
        Some(Cache::Number(Number::Int(_))) => "wideInt",
        Some(Cache::Number(Number::Double(_))) => "double",
        None => "none",
        _ => panic!("original scalar class"),
    };
    let words = [b"2".as_slice(), b"2.0", b"2", b"2", b"2", b"0x10", b"010"];
    let mut compared = 0;
    for row in include_str!(
        "../../../tcl-syntax/tests/data/native_numeric_operand_conversions/expression84-reached.tsv"
    )
    .lines()
    {
        let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
        let mode = fields[0].parse::<usize>().unwrap();
        let original = match mode {
            2 => Value::double(2.0),
            4 => Value::int(2),
            _ => Value::new_native_string_bytes(words[mode]),
        };
        if mode < 2 {
            original
                .native_scalar_getter(dialect, Getter::Double)
                .unwrap();
        }
        if mode == 3 {
            original
                .native_scalar_getter(dialect, Getter::Wide)
                .unwrap();
        }
        assert_eq!(class(&original), fields[1], "native before mode {mode}");
        assert_eq!(
            usize::from(original.resident_string_bytes().is_some()).to_string(),
            fields[2]
        );
        let one = Value::from_native_scalar_cache(Cache::Tcl84Long(1), None, dialect).unwrap();
        let result = arith_in(dialect, BinOp::Add, &original, &one).unwrap();
        assert_eq!(fields[3], "0");
        assert_eq!(class(&original), fields[4], "native reached mode {mode}");
        assert_eq!(
            usize::from(original.resident_string_bytes().is_some()).to_string(),
            fields[5]
        );
        assert_eq!(class(&result), fields[6], "native result mode {mode}");
        assert_eq!(
            usize::from(result.resident_string_bytes().is_some()).to_string(),
            fields[7]
        );
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.4"));
        let normalized = normalize_numeric_instruction_for_vm(&mut vm, &result).unwrap();
        assert_eq!(
            class(&normalized),
            fields[6],
            "native normalized result mode {mode}"
        );
        assert_eq!(
            normalized
                .native_string_bytes(dialect.native_string_protocol().unwrap())
                .unwrap()
                .as_ref(),
            fields[8].as_bytes()
        );
        compared += 1;
    }
    assert_eq!(compared, 7);
}

fn arith(op: BinOp, a: &Value, b: &Value) -> Result<Value, TclError> {
    arith_in(
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0),
        op,
        a,
        b,
    )
}

fn arithmetic_profiles() -> Vec<&'static tcl_dialect::DialectProfile> {
    let mut profiles: Vec<_> = tcl_dialect::TclVersion::ALL
        .iter()
        .map(|version| tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap())
        .collect();
    profiles.push(tcl_registry::model::ingress::resolve_environment("jim").analyser_profile());
    profiles
}

#[test]
fn original_expression_bytes_keep_leaf_values_and_cached_source() {
    for profile in arithmetic_profiles() {
        let mut vm = crate::native_fixture::core(profile);
        vm.set_var_bytes(b"\xff", Value::from_native_string_bytes(&b"7"[..]))
            .expect("original byte variable name");
        for (source, expected) in [
            (&b"\"\xff\0tail\""[..], &b"\xff\0tail"[..]),
            (&b"{\xff\0tail}"[..], &b"\xff\0tail"[..]),
            (&b"${\xff}"[..], &b"7"[..]),
            (&b"\"${\xff}\""[..], &b"7"[..]),
            (&b"0 && \"${\xffmissing}\""[..], &b"0"[..]),
        ] {
            let object = Value::from_native_string_bytes(source);
            for _ in 0..2 {
                let node = vm
                    .prepare_expression_value(&object)
                    .expect("native original expression source");
                let mut ops = ExprEval::new(&mut vm);
                let value = tcl_syntax::expr::eval(&node, &mut ops)
                    .expect("native byte expression evaluation");
                assert_eq!(
                    value.string_bytes().as_ref(),
                    expected,
                    "{}",
                    profile.display_name
                );
                assert_eq!(object.string_bytes().as_ref(), source);
            }
        }
    }
}

#[test]
fn operand_failures_keep_logical_simulation_separate_from_the_host_engine() {
    let mut vm = Vm::new();
    vm.set_dialect_profile(tcl_dialect::DialectProfile::irules());
    let host = crate::environment::profile_for_dialect("tcl9.0");
    assert!(vm.set_native_engine_profile(host));
    let invalid = Value::from_string_bytes(b"bad\xff\0tail".as_slice());
    let refused = arith_in(vm.numeric_context(), BinOp::Add, &invalid, &Value::int(1))
        .expect_err("logical numeric provider is required");
    assert!(refused.is_host());
    assert!(vm.set_logical_numeric_provider(
        tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation::Tcl84Core,
    ));
    let context = vm.numeric_context();
    let error = arith_in(context, BinOp::Add, &invalid, &Value::int(1))
        .expect_err("authored logical operand rejection");
    assert_eq!(
        error.message_bytes().unwrap().as_ref(),
        b"can't use non-numeric string as operand of \"+\""
    );
    assert_eq!(
        error.error_code_bytes().unwrap().as_deref(),
        Some(b"NONE".as_slice())
    );
    assert!(invalid.integer_representation().is_none());
    let boolean = boolean_operand_error(
        context,
        authored_boolean(context, &invalid).expect_err("full logical boolean spelling"),
    );
    assert_eq!(
        boolean.message_bytes().unwrap().as_ref(),
        b"expected boolean value but got \"bad\xff\0tail\""
    );
    assert_eq!(
        boolean.error_code_bytes().unwrap().as_deref(),
        Some(b"NONE".as_slice())
    );
    let float = unary_in(context, UnaryOp::BitNot, &Value::double(1.5))
        .expect_err("integer-only logical operator");
    assert_eq!(
        float.message_bytes().unwrap().as_ref(),
        b"can't use floating-point value as operand of \"~\""
    );
    assert_eq!(
        float.error_code_bytes().unwrap().as_deref(),
        Some(b"NONE".as_slice())
    );
    let older_native =
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_5);
    for error in [
        arith_in(
            older_native,
            BinOp::BitAnd,
            &Value::double(1.5),
            &Value::int(1),
        )
        .expect_err("selected older native integer-only binary error"),
        unary_in(older_native, UnaryOp::BitNot, &Value::double(1.5))
            .expect_err("selected older native integer-only unary error"),
    ] {
        assert!(
            error
                .message_bytes()
                .unwrap()
                .starts_with(b"can't use floating-point value as operand of")
        );
        assert_eq!(
            error.error_code_bytes().unwrap().as_deref(),
            Some(b"ARITH DOMAIN {floating-point value}".as_slice())
        );
    }
    let native = tcl_native_model::InvocationDialect::of_profile(host);
    let error = arith_in(native, BinOp::Add, &Value::string("bad"), &Value::int(1))
        .expect_err("actual host operand rejection");
    assert_eq!(
        error.message_bytes().unwrap().as_ref(),
        b"cannot use non-numeric string \"bad\" as left operand of \"+\""
    );
    assert_eq!(
        error.error_code_bytes().unwrap().as_deref(),
        Some(b"ARITH DOMAIN {non-numeric string}".as_slice())
    );
}

#[test]
fn reached_operand_error_codes_follow_the_actual_expression_release() {
    for version in [tcl_dialect::TclVersion::V8_4, tcl_dialect::TclVersion::V8_5] {
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        let value = Value::string("invalid");
        let arithmetic =
            arith_in(dialect, BinOp::Add, &value, &Value::int(1)).expect_err("non-numeric operand");
        let mut original = crate::native_fixture::core(crate::environment::profile_for_dialect(
            version.dialect_profile_name(),
        ));
        let boolean = boolean_for_vm(
            &mut original,
            &value,
            tcl_native_model::native_boolean_truth::NativeBooleanTruthPurpose::ConditionalJump,
        )
        .expect_err("non-boolean operand");
        if version == tcl_dialect::TclVersion::V8_4 {
            assert_eq!(
                arithmetic.error_code_bytes().unwrap().as_deref(),
                Some(b"NONE".as_slice())
            );
            assert_eq!(
                boolean.error_code_bytes().unwrap().as_deref(),
                Some(b"NONE".as_slice())
            );
        } else {
            assert_eq!(
                arithmetic.error_code_bytes().unwrap().as_deref(),
                Some(b"ARITH DOMAIN {non-numeric string}".as_slice())
            );
            assert_eq!(
                boolean
                    .error_code_bytes_in(&mut original)
                    .unwrap()
                    .as_deref(),
                Some(b"TCL VALUE NUMBER".as_slice())
            );
        }
    }
}

/// Fixed expectations come from C8.4–9.1 and Jim. Each runs through
/// bytecode arithmetic and through the dynamic expression parser.
#[test]
fn checked_equality_ordering_and_mathop_preserve_distinct_native_object_protocols() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
    crate::native_fixture::install_distribution(&mut vm);
    let dialect = vm.native_invocation_dialect();
    let raw = Value::from_string_bytes([0xff]);
    let unicode = Value::string("ÿ");
    assert!(!compare_in(dialect, BinOp::StrEq, &raw, &unicode).unwrap());
    assert!(compare_in(dialect, BinOp::Eq, &raw, &unicode).unwrap());
    let cached = Value::native_jim_string(&[0xc3, b'A', 0xc3, 0xa9], 2);
    let recounted = Value::from_string_bytes([0xc3, b'A', 0xc3, 0xa9]);
    assert!(compare_in(dialect, BinOp::StrEq, &cached, &recounted).unwrap());
    assert!(!compare_in(dialect, BinOp::Eq, &cached, &recounted).unwrap());
    let mut ops = ExprEval::new(&mut vm);
    let equal = tcl_cmd_core::mathop::eval(&mut ops, "eq", vec![cached.clone(), recounted.clone()])
        .unwrap_or_else(|_| panic!("known byte equality succeeds"));
    assert!(equal.as_bool().unwrap());
    let ordered = tcl_cmd_core::mathop::eval(&mut ops, "lt", vec![cached, recounted])
        .unwrap_or_else(|_| panic!("known cached ordering succeeds"));
    assert!(ordered.as_bool().unwrap());
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    let error = tcl_cmd_core::mathop::eval(&mut ExprEval::new(&mut vm), "lt", vec![raw, unicode])
        .unwrap_err();
    let tcl_cmd_core::mathop::MathopError::Op(error) = error else {
        panic!("checked comparison error")
    };
    assert!(error.is_host());
    assert!(
        compare_string_units(None, &Value::string("a"), &Value::string("b"))
            .unwrap_err()
            .native_access_refusal()
            .is_some()
    );
}

#[test]
fn tcl84_expr_uses_the_registry_selected_fixed_math_table() {
    fn replacement(_vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        ok(Value::int(99))
    }

    let interpreter =
        |name| crate::native_fixture::core(crate::environment::profile_for_dialect(name));
    let mut vm = interpreter("tcl8.4");
    // Replace the normal command-table entry after construction. The
    // preserved fixed table is intentionally not changed: Tcl 8.4's expr
    // dispatch did not use a Tcl command name at all.
    vm.register_command("tcl::mathfunc::sqrt", Command::Builtin(replacement));

    // TP: a fixed-table builtin remains executable.
    assert_eq!(vm.eval_expr("sqrt(4)").unwrap().to_str().as_ref(), "2.0");
    // FN: an absent fixed-table entry has C Tcl's distinct diagnostic.
    assert_eq!(
        vm.eval_expr("user_supplied(1)")
            .unwrap_err()
            .message_unicode()
            .expect("Unicode fixture error")
            .to_string(),
        "unknown math function \"user_supplied\""
    );
    // Host registrations remain visible independently of absent stock commands.
    let replaced = vm.invoke_command("::tcl::mathfunc::sqrt", &[Value::int(4)]);
    assert_eq!(replaced.code, Code::Ok);
    assert_eq!(replaced.result.to_str().as_ref(), "99");
    // The default stock wrapper is absent before TIP 232.
    let mut stock = interpreter("tcl8.4");
    assert_eq!(
        stock
            .invoke_command("::tcl::mathfunc::sqrt", &[Value::int(4)])
            .code,
        Code::Error
    );

    let mut vm = interpreter("tcl8.5");
    vm.register_command("tcl::mathfunc::sqrt", Command::Builtin(replacement));
    // FP guard: the same replacement becomes the open command-table target
    // once TIP 232 is enabled, rather than retaining the 8.4 shortcut.
    assert_eq!(vm.eval_expr("sqrt(4)").unwrap().to_str().as_ref(), "99");
}

/// These boundaries are established below the VM by the shared expression
/// numeral scanner. The parse success and error shape are direct tclsh
/// 8.6.17 / 9.0.3 oracle rows; if a radix run or completed NaN payload
/// fuses with its suffix, `eval_expr` reaches a different AST or misses
/// C's bareword. The VM's comparison coercion is intentionally not the
/// lexeme-boundary assertion here.
#[test]
fn shared_expr_numeral_boundaries_reach_the_vm() {
    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl8.6"));
    assert!(vm.eval_expr("0b1ne 1").is_ok());
    assert!(vm.eval_expr("0xfne 1").is_ok());
    assert!(vm.eval_expr("0xffin {255}").is_ok());

    let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    assert!(vm.eval_expr("0xfge 15").is_ok());
    assert!(vm.eval_expr("0d9lt 10").is_ok());
    for source in ["0 + 1_eq", "0 + 12x"] {
        assert!(
            vm.eval_expr(source)
                .unwrap_err()
                .message_unicode()
                .expect("Unicode fixture error")
                .to_string()
                .contains("invalid bareword"),
            "{source}"
        );
    }
    assert!(
        vm.eval_expr("NaN(1)x")
            .unwrap_err()
            .message_unicode()
            .expect("Unicode fixture error")
            .to_string()
            .starts_with("invalid bareword \"x\"")
    );
}

#[test]
fn int_div_mod_floored() {
    // Tcl: -7 / 2 == -4, -7 % 2 == 1
    assert_eq!(
        arith(BinOp::Div, &Value::int(-7), &Value::int(2))
            .unwrap()
            .as_int()
            .unwrap(),
        -4
    );
    assert_eq!(
        arith(BinOp::Mod, &Value::int(-7), &Value::int(2))
            .unwrap()
            .as_int()
            .unwrap(),
        1
    );
}

/// The arbitrary-precision integer tower: operations that
/// overflow `i128` (or whose operands already exceed it) stay exact and match
/// tclsh 9.0.4, instead of wrapping or degrading to a lossy `double`.
#[test]
fn bignum_tower_stays_exact() {
    let big = |op: BinOp, a: &str, b: &str| {
        arith(op, &Value::string(a), &Value::string(b))
            .unwrap()
            .to_str()
            .to_string()
    };
    // (2^64)^2 = 2^128 — a product past i128 promotes to an exact bignum.
    assert_eq!(
        big(BinOp::Mul, "18446744073709551616", "18446744073709551616"),
        "340282366920938463463374607431768211456"
    );
    // Pow, floor div/mod and shifts stay exact.
    assert_eq!(
        arith(BinOp::Pow, &Value::int(2), &Value::int(200))
            .unwrap()
            .to_str()
            .to_string(),
        "1606938044258990275541962092341162602522202993782792835301376"
    );
    assert_eq!(
        big(BinOp::Div, "1000000000000000000000000000000", "3"),
        "333333333333333333333333333333"
    );
    assert_eq!(big(BinOp::Mod, "1000000000000000000000000000000", "7"), "1");
    assert_eq!(
        arith(BinOp::LShift, &Value::int(1), &Value::int(100))
            .unwrap()
            .to_str()
            .to_string(),
        "1267650600228229401496703205376"
    );
    // i64/i128-overflowing sums promote instead of wrapping.
    assert_eq!(
        arith(BinOp::Add, &Value::int(i64::MAX), &Value::int(1))
            .unwrap()
            .to_str()
            .to_string(),
        "9223372036854775808"
    );
    // The shared-tower oracle rows (tclsh 8.6/9.0): floor div/mod at and
    // beyond the wide boundary, the shift width-collapse, and `1 << 63`
    // crossing into bignum.
    assert_eq!(big(BinOp::Mod, "18446744073709551616", "7"), "2");
    assert_eq!(
        big(BinOp::Div, "18446744073709551616", "-3"),
        "-6148914691236517206"
    );
    assert_eq!(big(BinOp::RShift, "18446744073709551616", "64"), "1");
    assert_eq!(big(BinOp::RShift, "-18446744073709551616", "200"), "-1");
    assert_eq!(
        arith(BinOp::LShift, &Value::int(1), &Value::int(63))
            .unwrap()
            .to_str()
            .to_string(),
        "9223372036854775808"
    );
    // Exact bignum comparison — an `f64` would collapse `2^100` and `2^100+1`.
    let (a, b) = (
        "1267650600228229401496703205376",
        "1267650600228229401496703205377",
    );
    assert!(compare(BinOp::Lt, &Value::string(a), &Value::string(b)).unwrap());
    assert!(!compare(BinOp::Eq, &Value::string(a), &Value::string(b)).unwrap());
    // Unary negate / bit-not of a bignum stay exact; `0 ** -n` errors like C.
    assert_eq!(
        unary(UnaryOp::Neg, &Value::string(a))
            .unwrap()
            .to_str()
            .to_string(),
        format!("-{a}")
    );
    assert_eq!(
        unary(UnaryOp::BitNot, &Value::string(a))
            .unwrap()
            .to_str()
            .to_string(),
        format!("-{b}")
    );
    assert_eq!(
        unary(UnaryOp::BitNot, &Value::string("18446744073709551616"))
            .unwrap()
            .to_str()
            .to_string(),
        "-18446744073709551617"
    );
    assert!(arith(BinOp::Pow, &Value::int(0), &Value::int(-1)).is_err());
}

/// Operands already past `i128` are numeric (`Num::Huge`), not the
/// non-numeric-string operand error: they reach the arbitrary-precision
/// path in `arith`/`unary` and stay exact. All rows pinned to tclsh
/// 8.6.14 (`2**200` is the 61-digit 16069…376).
#[test]
fn operands_past_i128_reach_the_bignum_path() {
    const P200: &str = "1606938044258990275541962092341162602522202993782792835301376";
    let big = |op: BinOp, a: &str, b: &str| {
        arith(op, &Value::string(a), &Value::string(b))
            .unwrap()
            .to_str()
            .to_string()
    };
    // tclsh: 2**200 + 1 (the divergence that motivated the gap fix).
    assert_eq!(
        big(BinOp::Add, P200, "1"),
        "1606938044258990275541962092341162602522202993782792835301377"
    );
    // tclsh: floor div/mod, shifts and bit ops over a beyond-i128 operand.
    assert_eq!(big(BinOp::Mod, P200, "7"), "4");
    assert_eq!(
        big(BinOp::Div, P200, "-3"),
        "-535646014752996758513987364113720867507400997927597611767126"
    );
    assert_eq!(big(BinOp::RShift, P200, "137"), "9223372036854775808");
    assert_eq!(
        big(BinOp::RShift, &format!("-{P200}"), "500"),
        "-1",
        "sign collapse past the width"
    );
    assert_eq!(big(BinOp::BitAnd, P200, "255"), "0");
    assert_eq!(
        big(BinOp::BitOr, P200, "1"),
        "1606938044258990275541962092341162602522202993782792835301377"
    );
    // Unary over a beyond-i128 operand: exact negate / complement / pass.
    assert_eq!(
        unary(UnaryOp::Neg, &Value::string(P200))
            .unwrap()
            .to_str()
            .to_string(),
        format!("-{P200}")
    );
    assert_eq!(
        unary(UnaryOp::BitNot, &Value::string(P200))
            .unwrap()
            .to_str()
            .to_string(),
        "-1606938044258990275541962092341162602522202993782792835301377"
    );
    assert_eq!(
        unary(UnaryOp::Pos, &Value::string(P200))
            .unwrap()
            .to_str()
            .to_string(),
        P200
    );
    // Error surfaces on the bignum path keep C's message text.
    let div0 = arith(BinOp::Div, &Value::string(P200), &Value::int(0)).unwrap_err();
    assert_eq!(
        div0.message_unicode()
            .expect("Unicode fixture error")
            .as_ref(),
        "divide by zero"
    );
    let mod0 = arith(BinOp::Mod, &Value::string(P200), &Value::int(0)).unwrap_err();
    assert_eq!(
        mod0.message_unicode()
            .expect("Unicode fixture error")
            .as_ref(),
        "divide by zero"
    );
}

/// The `**` and shift guards of the shared tower, with C's message text:
/// the exponent limit is `MAX_EXPONENT` (2^28 - 1 — tclsh errors at
/// `2**268435456` rather than compute a 33 MB number), a bignum
/// exponent keeps the `0`/`±1` collapses, and a left-shift count past
/// `INT_MAX` is C's overflow error rather than an astronomic attempt.
#[test]
fn pow_and_shift_guards_match_c() {
    let errmsg = |op: BinOp, a: &Value, b: &Value| {
        arith(op, a, b)
            .unwrap_err()
            .message_unicode()
            .expect("Unicode fixture error")
            .to_string()
    };
    // tclsh: expr {2**268435456} → exponent too large (268435455 is legal).
    assert_eq!(
        errmsg(BinOp::Pow, &Value::int(2), &Value::int(268_435_456)),
        "exponent too large"
    );
    assert_eq!(
        errmsg(BinOp::Pow, &Value::int(2), &Value::int(5_000_000_000)),
        "exponent too large"
    );
    // tclsh: a beyond-i64 exponent still collapses for |base| <= 1 —
    // parity included — and errors for any other base.
    let huge = "99999999999999999999";
    let pow = |a: i64, b: &str| {
        arith(BinOp::Pow, &Value::int(a), &Value::string(b))
            .unwrap()
            .to_str()
            .to_string()
    };
    assert_eq!(pow(1, huge), "1");
    assert_eq!(pow(-1, huge), "-1"); // odd exponent
    assert_eq!(pow(-1, "99999999999999999998"), "1"); // even exponent
    assert_eq!(pow(0, huge), "0");
    assert_eq!(
        errmsg(BinOp::Pow, &Value::int(2), &Value::string(huge)),
        "exponent too large"
    );
    assert_eq!(
        errmsg(
            BinOp::Pow,
            &Value::int(0),
            &Value::string(format!("-{huge}"))
        ),
        "exponentiation of zero by negative power"
    );
    // tclsh: expr {2 << 4294967296} → integer value too large to
    // represent (count must fit C's int); negative counts keep their
    // error on both tiers.
    assert_eq!(
        errmsg(BinOp::LShift, &Value::int(2), &Value::int(4_294_967_296)),
        "integer value too large to represent"
    );
    assert_eq!(
        errmsg(
            BinOp::LShift,
            &Value::int(2),
            &Value::string("99999999999999999999")
        ),
        "integer value too large to represent"
    );
    assert_eq!(
        errmsg(
            BinOp::RShift,
            &Value::string("18446744073709551616"),
            &Value::int(-1)
        ),
        "negative shift argument"
    );
    // tclsh: a beyond-wide right-shift count collapses to the sign.
    let big = |op: BinOp, a: &str, b: &str| {
        arith(op, &Value::string(a), &Value::string(b))
            .unwrap()
            .to_str()
            .to_string()
    };
    assert_eq!(big(BinOp::RShift, "2", huge), "0");
    assert_eq!(big(BinOp::RShift, "-2", huge), "-1");
}

/// Float contagion over bignum operands: C converts the integer with
/// `TclBignumToDouble` (round-to-nearest-even over the full value) and
/// computes in `f64` — it does not refuse. Rows pinned to tclsh 8.6.14,
/// including the tie whose deciding bit sits below the top 64 (2^127 +
/// 2^74 + 1 must round *up*; a bare 64-bit truncation rounds it down).
#[test]
fn bignum_float_contagion_matches_c() {
    let s = |t: &str| Value::string(t);
    let f = |op: BinOp, a: &Value, b: &Value| arith(op, a, b).unwrap().to_str().to_string();
    // tclsh 8.5 to 9.1: expr {1.5 + 18446744073709551616} prints the
    // exact 2^64 as "1.844674407370955e+19", which re-parses as
    // 2^64 - 2048; `format_double` follows C's digit generation.
    assert_eq!(
        f(BinOp::Add, &Value::double(1.5), &s("18446744073709551616")),
        "1.844674407370955e+19"
    );
    // tclsh: expr {2**200 + 1.5} → 1.6069380442589903e+60
    let p200 = "1606938044258990275541962092341162602522202993782792835301376";
    assert_eq!(
        f(BinOp::Add, &s(p200), &Value::double(1.5)),
        "1.6069380442589903e+60"
    );
    // Round-to-nearest-even ties: 2^127+2^74 is exactly halfway (stays
    // even → 2^127), 2^127+2^74+1 is just above it (rounds up).
    assert_eq!(
        f(
            BinOp::Mul,
            &s("170141183460469250621153235194464960512"),
            &Value::double(1.0)
        ),
        "1.7014118346046923e+38"
    );
    assert_eq!(
        f(
            BinOp::Mul,
            &s("170141183460469250621153235194464960513"),
            &Value::double(1.0)
        ),
        "1.7014118346046927e+38"
    );
    // Past the double range the conversion overflows to Inf, as C's
    // `TclBignumToDouble` does (tclsh: expr {1.0 + 10**309} → Inf).
    let p1100 = arith(BinOp::Pow, &Value::int(2), &Value::int(1100)).unwrap();
    assert_eq!(f(BinOp::Add, &p1100, &Value::double(1.0)), "Inf");
    // An integer-only operator still rejects the *float* operand — the
    // bignum side is fine (tclsh 9 wording, sided).
    let e = arith(BinOp::Mod, &s(p200), &Value::double(1.5)).unwrap_err();
    assert_eq!(
        e.message_unicode().expect("Unicode fixture error").as_ref(),
        "cannot use floating-point value \"1.5\" as right operand of \"%\""
    );
    let e = arith(BinOp::Mod, &Value::double(1.5), &s(p200)).unwrap_err();
    assert_eq!(
        e.message_unicode().expect("Unicode fixture error").as_ref(),
        "cannot use floating-point value \"1.5\" as left operand of \"%\""
    );
}

#[test]
fn mixed_promotes_to_double() {
    assert_eq!(
        &*arith(BinOp::Mul, &Value::int(2), &Value::double(1.5))
            .unwrap()
            .to_str(),
        "3.0"
    );
}

#[test]
fn compare_numeric_then_string() {
    assert!(compare(BinOp::Lt, &Value::string("9"), &Value::string("10")).unwrap());
    assert!(!compare(BinOp::Lt, &Value::string("apple"), &Value::string("Apple")).unwrap());
}

#[test]
fn compare_int_vs_double_is_exact() {
    // tclsh: expr {9007199254740993 == 9007199254740992.0} → 0 — a
    // both-as-f64 comparison would call them equal.
    let big = Value::string("9007199254740993");
    let dbl = Value::double(9_007_199_254_740_992.0);
    assert!(!compare(BinOp::Eq, &big, &dbl).unwrap());
    assert!(compare(BinOp::Gt, &big, &dbl).unwrap());
    assert!(compare(BinOp::Lt, &dbl, &big).unwrap());
    // Integers past i128 (the `CmpNum::Big` tier) against a double —
    // the bignum-vs-double arm: 2¹³⁰ vs 1.5×2¹³⁰-ish.
    let huge = Value::string("1361129467683753853853498429727072845824"); // 2^130
    assert!(compare(BinOp::Gt, &huge, &Value::double(1e39)).unwrap());
    assert!(compare(BinOp::Lt, &huge, &Value::double(1.4e39)).unwrap());
    // Equal-integer-part case in the bignum arm: a double that is exactly
    // 2¹³⁰ (powers of two are representable) compares Equal to the exact
    // decimal spelling of 2¹³⁰.
    assert!(compare(BinOp::Eq, &huge, &Value::double(2f64.powi(130))).unwrap());
}

#[test]
fn compare_nan_is_unordered() {
    // tclsh: `set x NaN` — `$x != 1` → 1; `==`/`<`/`<=`/`>`/`>=` → 0;
    // `$x == $x` → 0. Applies to string-spelled and typed NaN alike.
    for nan in [Value::string("NaN"), Value::double(f64::NAN)] {
        assert!(compare(BinOp::Ne, &nan, &Value::int(1)).unwrap());
        assert!(!compare(BinOp::Eq, &nan, &Value::int(1)).unwrap());
        assert!(!compare(BinOp::Lt, &nan, &Value::int(1)).unwrap());
        assert!(!compare(BinOp::Le, &nan, &Value::int(1)).unwrap());
        assert!(!compare(BinOp::Gt, &nan, &Value::int(1)).unwrap());
        assert!(!compare(BinOp::Ge, &nan, &Value::int(1)).unwrap());
        assert!(!compare(BinOp::Eq, &nan, &nan).unwrap());
        assert!(compare(BinOp::Ne, &nan, &nan).unwrap());
    }
    // The string comparators still see the spelling: `NaN eq NaN` → 1.
    let nan = Value::string("NaN");
    assert!(compare(BinOp::StrEq, &nan, &nan).unwrap());
}
