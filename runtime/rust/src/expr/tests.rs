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
fn original_unary_boolean_classifier_keeps_primitive_guest_acceptance_separate() {
    // Source/API: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Pinned Jim source JimExprOpNumUnary reaches its public Boolean getter;
    // these adapter controls assert no new original execution observation.
    let interp = crate::interp::Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    let dialect = interp.native_invocation_dialect();
    for (word, expected) in [
        (
            b"true".as_slice(),
            b"can't use non-numeric string as operand of \"+\"".as_slice(),
        ),
        (
            b"invalid".as_slice(),
            b"expected boolean but got \"invalid\"".as_slice(),
        ),
    ] {
        let original = Owned::fresh(obj::new_string_bytes(word));
        let error = selected_unary_error(
            ArithError::NonNumeric,
            UnaryOp::Pos,
            original.as_ptr(),
            dialect,
            None,
            None,
        );
        assert_eq!(error.msg, expected);
        assert!(error.native_access_refusal.is_none());
        assert!(error.native_execution_refusal.is_none());
    }
}

#[test]
fn original_unary_classifier_preserves_retirement_first_host_and_prior_result() {
    // Source/API: naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut interp = crate::interp::Interp::with_native_core(
        crate::interp::default_host(),
        tcl_registry::model::ingress::resolve_environment("jim").unit_profile(),
        tcl_registry::special_vars::NativeBootstrapInputs::default(),
    )
    .unwrap();
    interp.set_result_bytes(b"PRIOR\0\xff");
    let prior = interp.get_obj_result();
    let original = Owned::fresh(obj::new_string_bytes(b"true"));
    let pointer = original.as_ptr();
    let lifetime = obj::NativeObjectLifetime::retain(pointer);
    drop(original);
    assert!(!obj::allocation_is_live(pointer));
    let error = selected_unary_error(
        ArithError::NonNumeric,
        UnaryOp::Neg,
        pointer,
        interp.native_invocation_dialect(),
        None,
        None,
    );
    let cause = tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
        tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
            "retired native object",
        ),
    );
    assert_eq!(error.native_execution_refusal, Some(cause.clone()));
    assert!(
        error
            .retained_command_error()
            .unwrap()
            .native_execution_refusal()
            == Some(&cause)
    );
    interp.report_expr_error(error);
    assert_eq!(interp.native_execution_refusal(), Some(cause));
    assert_eq!(interp.get_obj_result(), prior);
    assert_eq!(interp.result_bytes(), b"PRIOR\0\xff");
    drop(lifetime);
}

use super::*;
use tcl_syntax::expr::parser::parse_expr;

/// A mock context: a `$var` table; `[cmd]` is unsupported in these tests.
struct MockCtx(std::collections::HashMap<String, i64>);
impl ExprCtx for MockCtx {
    fn invocation_dialect(&self) -> tcl_native_model::InvocationDialect {
        tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_0)
    }
    fn read_var(&mut self, name: &str) -> Result<Owned, ExprError> {
        self.0
            .get(name)
            .map(|&v| Owned::fresh(obj::new_wide_int_obj(v)))
            .ok_or_else(|| ExprError::msg(b"can't read var"))
    }
    fn eval_command(&mut self, _script: &str) -> Result<Owned, ExprError> {
        Err(ExprError::msg(b"no commands"))
    }
    fn call_function(&mut self, name: &str, args: &[Owned]) -> Result<Owned, ExprError> {
        // No command table in the mock — use the shared built-in dispatch.
        dispatch_shared(name, args)
    }
}

struct NativeCtx(tcl_native_model::InvocationDialect);
impl ExprCtx for NativeCtx {
    fn invocation_dialect(&self) -> tcl_native_model::InvocationDialect {
        self.0
    }
    fn read_var(&mut self, _: &str) -> Result<Owned, ExprError> {
        Err(ExprError::msg(b"unexpected variable read"))
    }
    fn eval_command(&mut self, _: &str) -> Result<Owned, ExprError> {
        Err(ExprError::msg(b"unexpected command"))
    }
    fn call_function(&mut self, _: &str, _: &[Owned]) -> Result<Owned, ExprError> {
        Err(ExprError::msg(b"unexpected function"))
    }
}

#[test]
fn c84_reached_expression_matches_seven_original_cache_controls() {
    use tcl_syntax::scalar_getter::{NativeScalarCache as Cache, NativeScalarGetterKind as Getter};
    let dialect = tcl_native_model::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_4);
    let class = |value| match obj::native_scalar_cache(value).unwrap() {
        Some(Cache::Tcl84Long(_)) => "int",
        Some(Cache::Number(tcl_syntax::number::Number::Int(_))) => "wideInt",
        Some(Cache::Number(tcl_syntax::number::Number::Double(_))) => "double",
        None => "none",
        _ => panic!("original scalar class"),
    };
    let words = [b"2".as_slice(), b"2.0", b"2", b"2", b"2", b"0x10", b"010"];
    let mut compared = 0;
    for row in include_str!("../../../../rust/tcl-syntax/tests/data/native_numeric_operand_conversions/expression84-reached.tsv").lines() {
            let fields = tcl_test_support::fixture::tsv_fields(row).collect::<Vec<_>>();
            let mode = fields[0].parse::<usize>().unwrap();
            let original = Owned::fresh(match mode {
                2 => obj::new_double_obj(2.0),
                4 => obj::new_wide_int_obj(2),
                _ => obj::new_string_bytes(words[mode]),
            });
            if mode < 2 { crate::typed_value::native_scalar_getter(original.ptr(), dialect, Getter::Double).unwrap(); }
            if mode == 3 { crate::typed_value::native_scalar_getter(original.ptr(), dialect, Getter::Wide).unwrap(); }
            assert_eq!(class(original.ptr()), fields[1], "native before mode {mode}");
            assert_eq!(usize::from(obj::has_string_rep(original.ptr())).to_string(), fields[2]);
            let one = native_integer_result(dialect, 1, &[]).unwrap();
            let mut context = NativeCtx(dialect);
            let mut ops = TowerOps { ctx: &mut context, jim: None, safe: false };
            let result = ops.arith(BinOp::Add, original.clone(), one).unwrap();
            assert_eq!(fields[3], "0");
            assert_eq!(class(original.ptr()), fields[4], "native reached mode {mode}");
            assert_eq!(usize::from(obj::has_string_rep(original.ptr())).to_string(), fields[5]);
            assert_eq!(class(result.ptr()), fields[6], "native result mode {mode}");
            assert_eq!(usize::from(obj::has_string_rep(result.ptr())).to_string(), fields[7]);
            assert_eq!(obj::bytes_of(result.ptr()), fields[8].as_bytes());
            compared += 1;
        }
    assert_eq!(compared, 7);
}

#[test]
fn checked_comparison_separates_jim_bytes_counts_and_storage_refusal() {
    crate::counters::reset();
    {
        let profile = crate::environment::profile_for_dialect("jim");
        let mut context = NativeCtx(tcl_native_model::InvocationDialect::of_profile(profile));
        let mut ops = TowerOps {
            ctx: &mut context,
            jim: None,
            safe: false,
        };
        let raw = Owned::fresh(obj::new_string_bytes(&[0xff]));
        let utf8 = Owned::fresh(obj::new_string_bytes(&[0xc3, 0xbf]));
        assert!(!ops.equal_string(&raw, &utf8).unwrap());
        assert_eq!(ops.compare_string(&raw, &utf8).unwrap(), Ordering::Equal);
        let list = Owned::fresh(crate::list::new_list_obj(&[utf8.ptr()]));
        assert!(!ops.in_list(&raw, &list).unwrap());
        assert!(ops.in_list(&utf8, &list).unwrap());
        let cached = Owned::fresh(obj::new_string_bytes(b"ab"));
        obj::retain_jim_string_count(cached.ptr(), 5);
        let error = ops.compare_string(&cached, &cached).unwrap_err();
        assert!(matches!(
            error.native_access_refusal,
            Some(tcl_syntax::raw_string::NativeValueAccessRefusal::StringAccess(_))
        ));
        assert!(error.msg.is_empty() && error.code.is_none());
    }
    assert_eq!(crate::counters::finalize(), 0, "comparison ownership leak");
}

#[test]
fn expression_refusal_bypasses_guest_capture_and_finally() {
    crate::counters::reset();
    {
        let mut interp = crate::interp::Interp::new();
        interp.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        interp
            .var_set(b"a", obj::new_string_bytes(&[0xff]))
            .unwrap();
        let code = interp.eval_str(b"set b okay; set prior 1; try {catch {expr {$a lt $b}} captured; set recovered 1} finally {set final 1}; set after 1");
        assert_eq!(code, crate::interp::Code::Error);
        assert!(matches!(
            interp.native_access_refusal(),
            Some(tcl_syntax::raw_string::NativeValueAccessRefusal::Unicode(_))
        ));
        // The refusal stops variable lookup, so observe owned storage.
        assert_eq!(
            crate::test_storage::global_scalar(&interp, b"prior").map(obj::bytes_of),
            Some(b"1".to_vec())
        );
        for name in [b"captured".as_slice(), b"recovered", b"final", b"after"] {
            assert!(
                crate::test_storage::global_scalar(&interp, name).is_none(),
                "unexpected guest write: {name:?}"
            );
        }
    }
    assert_eq!(crate::counters::finalize(), 0, "refusal ownership leak");
}

#[test]
fn selected_numeric_input_preserves_original_bytes_and_caches_the_actual_object() {
    crate::counters::reset();
    {
        let jim = tcl_native_model::InvocationDialect::of_profile(
            crate::environment::profile_for_dialect("jim"),
        );
        let c = tcl_native_model::InvocationDialect::of_profile(
            crate::environment::profile_for_dialect("tcl8.6"),
        );
        let raw = Owned::fresh(obj::new_string_bytes(&[b'1', 0, 0xff]));
        assert_eq!(
            crate::typed_value::scalar_number(raw.ptr(), c, true).unwrap(),
            None
        );
        assert_eq!(
            crate::typed_value::scalar_number(raw.ptr(), jim, true).unwrap(),
            Some(tcl_syntax::number::Number::Int(1))
        );
        assert!(core::ptr::eq(
            obj::obj_type_ptr(raw.ptr()),
            &obj::TCL_INT_TYPE
        ));
        assert_eq!(obj::bytes_of(raw.ptr()), [b'1', 0, 0xff]);
        let bad = Owned::fresh(obj::new_string_bytes(&[0xff, 0, b'1']));
        assert_eq!(
            crate::typed_value::scalar_number(bad.ptr(), jim, false).unwrap(),
            None
        );
        assert_eq!(obj::bytes_of(bad.ptr()), [0xff, 0, b'1']);
    }
    assert_eq!(
        crate::counters::finalize(),
        0,
        "numeric input ownership leak"
    );
}

#[test]
fn jim_explicit_double_getter_keeps_exact_integer_and_lazy_string_cache() {
    use tcl_syntax::value::ValueOps;
    crate::counters::reset();
    {
        let mut interp = crate::interp::Interp::new();
        interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
        for integer in [7, i64::MAX] {
            let original = Owned::fresh(obj::new_wide_int_obj(integer));
            let alias = original.clone();
            assert!(!obj::has_string_rep(original.ptr()));
            assert_eq!(interp.as_double(&alias.ptr()).unwrap(), integer as f64);
            assert!(core::ptr::eq(
                obj::obj_type_ptr(original.ptr()),
                &obj::JIM_COERCED_DOUBLE_TYPE
            ));
            assert_eq!(obj::has_string_rep(original.ptr()), integer == i64::MAX);
            assert_eq!(interp.as_int(&original.ptr()).unwrap(), integer);
            assert!(core::ptr::eq(
                obj::obj_type_ptr(alias.ptr()),
                &obj::TCL_INT_TYPE
            ));
            assert_eq!(obj::bytes_of(alias.ptr()), integer.to_string().as_bytes());
        }
        interp.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        let c = Owned::fresh(obj::new_wide_int_obj(7));
        assert_eq!(interp.as_double(&c.ptr()).unwrap(), 7.0);
        assert!(core::ptr::eq(
            obj::obj_type_ptr(c.ptr()),
            &obj::TCL_INT_TYPE
        ));
        assert!(!obj::has_string_rep(c.ptr()));
    }
    assert_eq!(
        crate::counters::finalize(),
        0,
        "coerced getter ownership leak"
    );
}

fn ev(src: &str, vars: &[(&str, i64)]) -> Result<Vec<u8>, ExprError> {
    crate::counters::reset();
    let node = parse_expr(src, None);
    let mut ctx = MockCtx(vars.iter().map(|&(k, v)| (k.to_string(), v)).collect());
    let r = eval_expr(&node, &mut ctx)?;
    let out = obj::bytes_of(r.ptr());
    drop(r);
    assert_eq!(crate::counters::finalize(), 0, "leak");
    Ok(out)
}

fn ok(src: &str) -> Vec<u8> {
    ev(src, &[]).expect("eval")
}

#[test]
fn arithmetic_and_precedence() {
    assert_eq!(ok("1 + 2 * 3"), b"7");
    assert_eq!(ok("(1 + 2) * 3"), b"9");
    assert_eq!(ok("2 ** 10"), b"1024");
    assert_eq!(ok("2 ** 64"), b"18446744073709551616"); // bignum
    assert_eq!(ok("7 / 2"), b"3");
    assert_eq!(ok("-7 / 2"), b"-4"); // floor
    assert_eq!(ok("7 % 3"), b"1");
    assert_eq!(ok("1 + 2.5"), b"3.5"); // double promotion
}

#[test]
fn bitwise_and_shifts() {
    assert_eq!(ok("0xff & 0x0f"), b"15");
    assert_eq!(ok("12 | 3"), b"15");
    assert_eq!(ok("5 ^ 3"), b"6");
    assert_eq!(ok("~5"), b"-6");
    assert_eq!(ok("1 << 4"), b"16");
    assert_eq!(ok("256 >> 4"), b"16");
}

#[test]
fn comparisons_numeric_and_string() {
    assert_eq!(ok("3 < 5"), b"1");
    assert_eq!(ok("5 <= 5"), b"1");
    assert_eq!(ok("3 == 3"), b"1");
    assert_eq!(ok("3 != 4"), b"1");
    assert_eq!(ok(r#""abc" eq "abc""#), b"1");
    assert_eq!(ok(r#""abc" ne "abd""#), b"1");
}

#[test]
fn short_circuit_and_ternary() {
    assert_eq!(ok("1 && 0"), b"0");
    assert_eq!(ok("0 || 1"), b"1");
    assert_eq!(ok("!0"), b"1");
    assert_eq!(ok("1 ? 42 : 99"), b"42");
    assert_eq!(ok("0 ? 42 : 99"), b"99");
}

#[test]
fn boolean_prefixes_share_the_canonical_converter() {
    for source in [
        "true", "tru", "t", "yes", "ye", "y", "false", "f", "no", "n", "off", "of",
    ] {
        assert_eq!(ok(source), source.as_bytes(), "{source}");
    }
    assert_eq!(ok("tru ? yes : no"), b"yes");
    assert_eq!(ok("!of"), b"1");
    assert!(ev("o", &[]).is_err(), "on/off share the prefix o");
}

#[test]
fn variables_and_membership() {
    assert_eq!(ev("$x + $y", &[("x", 10), ("y", 32)]).unwrap(), b"42");
    assert_eq!(ev("$x * 2", &[("x", 21)]).unwrap(), b"42");
    assert_eq!(ok("3 in {1 2 3 4}"), b"1");
    assert_eq!(ok("9 ni {1 2 3 4}"), b"1");
}

#[test]
fn math_functions() {
    // the shared tcl_syntax::expr::mathfunc dispatch, over the tower
    assert_eq!(ok("sqrt(4)"), b"2.0");
    assert_eq!(ok("max(1, 9, 3)"), b"9");
    assert_eq!(ok("min(5, 2)"), b"2");
    assert_eq!(ok("abs(-7)"), b"7");
    assert_eq!(ok("int(3.9)"), b"3");
    assert_eq!(ok("pow(2, 10)"), b"1024.0");
    // unknown function / domain error surface as errors
    assert!(ev("frobnicate(1)", &[]).is_err());
    assert!(ev("sqrt(-1)", &[]).is_err());
}

#[test]
fn errors() {
    assert_eq!(
        ev("1 / 0", &[]),
        Err(ExprError::with_code(
            b"divide by zero",
            b"ARITH DIVZERO {divide by zero}"
        ))
    );
    assert!(ev("$missing + 1", &[]).is_err());
}
