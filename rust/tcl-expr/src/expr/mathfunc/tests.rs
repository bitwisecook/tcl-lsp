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
use tcl_numeric::number_tower::BigIntOps;

/// Every built-in has a class of its own, and the classes are what the
/// interpreters return (tclsh 8.4 to 9.1: `int(2.5)` is 2, `ceil(3.14)`
/// is 4.0, `abs(-2)` is 2 and `abs(-2.5)` is 2.5, `isfinite(1.0)` is 1).
#[test]
fn every_function_has_a_result_class() {
    for spec in all() {
        assert_ne!(
            spec.result_class,
            MathResultClass::Any,
            "{} has no class",
            spec.name
        );
        assert_eq!(result_class(spec.name), spec.result_class, "{}", spec.name);
    }
    for (name, class) in [
        ("int", MathResultClass::Int),
        ("entier", MathResultClass::Int),
        ("ceil", MathResultClass::Float),
        ("trunc", MathResultClass::Float),
        ("rand", MathResultClass::Float),
        ("abs", MathResultClass::Numeric),
        ("max", MathResultClass::Numeric),
        ("bool", MathResultClass::Bool),
        ("isfinite", MathResultClass::Bool),
        ("isunordered", MathResultClass::Bool),
        ("signbit", MathResultClass::Bool),
    ] {
        assert_eq!(result_class(name), class, "{name}");
    }
    assert_eq!(result_class("nope"), MathResultClass::Any);
    assert_eq!(
        result_class("ABS"),
        MathResultClass::Any,
        "lookup is verbatim"
    );
}

/// Tcl 8.4 checks each result as tclsh 8.4.20 does; 8.5 on keep it.
#[test]
fn release_range_checks() {
    use tcl_dialect::TclVersion::{V8_4, V8_6};
    let call = |name: &str, args: &[f64], release| {
        let args: Vec<Num> = args.iter().map(|&f| Num::Float(f)).collect();
        try_dispatch_for_release(name, &args, release)
    };
    let refused = [
        ("exp", vec![-1000.0], MathFuncError::FloatUnderflow),
        ("exp", vec![710.0], MathFuncError::FloatOverflow),
        ("pow", vec![10.0, -400.0], MathFuncError::FloatUnderflow),
        ("pow", vec![0.0, -1.0], MathFuncError::FloatOverflow),
        ("atan2", vec![1e-300, 1e300], MathFuncError::FloatUnderflow),
        ("log", vec![0.0], MathFuncError::FloatOverflow),
        ("cosh", vec![1000.0], MathFuncError::FloatOverflow),
        ("sqrt", vec![-1.0], MathFuncError::Domain),
        ("int", vec![9.3e18], MathFuncError::IntegerOverflow),
        ("wide", vec![-9.3e18], MathFuncError::IntegerOverflow),
        ("round", vec![9.3e18], MathFuncError::IntegerOverflow),
    ];
    for (name, args, error) in refused {
        assert_eq!(call(name, &args, V8_4), Err(error), "{name}{args:?}");
    }
    let kept = [
        ("exp", vec![-745.0]),
        ("pow", vec![0.0, 0.0]),
        ("atan2", vec![0.0, 0.0]),
        ("fmod", vec![0.0, 1.0]),
        ("tanh", vec![1000.0]),
        ("round", vec![9.2e18]),
    ];
    for (name, args) in kept {
        assert!(call(name, &args, V8_4).is_ok(), "{name}{args:?}");
    }
    assert_eq!(call("exp", &[-1000.0], V8_6), Ok(Num::Float(0.0)));
    assert_eq!(call("cosh", &[1000.0], V8_6), Ok(Num::Float(f64::INFINITY)));
}

#[test]
fn float_functions() {
    assert_eq!(dispatch("sqrt", &[Num::Int(4)]), Some(Num::Float(2.0)));
    assert!(dispatch("sqrt", &[Num::Int(-1)]).is_none()); // domain error
    assert_eq!(dispatch("sqrt", &[Num::Float(f64::NAN)]), None);
    assert_eq!(
        dispatch("atan2", &[Num::Float(f64::NAN), Num::Int(1)]),
        None
    );
    assert_eq!(
        dispatch("pow", &[Num::Int(2), Num::Int(10)]),
        Some(Num::Float(1024.0))
    );
}

/// Jim's `pow` of two integers is an integer power (jimsh 0.84: `pow(2,3)`
/// is `8`, `pow(2,-1)` is `0`); every Tcl release computes C `pow`.
#[test]
fn dispatch_under_declines_jims_integer_pow() {
    use tcl_dialect::NumberSyntax;
    let ints = [Num::Int(2), Num::Int(3)];
    for jim in [NumberSyntax::Jim, NumberSyntax::Jim080] {
        assert_eq!(dispatch_under("pow", &ints, jim), None);
        assert_eq!(
            dispatch_under("pow", &[Num::Float(2.0), Num::Int(3)], jim),
            Some(Num::Float(8.0))
        );
        assert_eq!(
            dispatch_under("hypot", &[Num::Int(3), Num::Int(4)], jim),
            Some(Num::Float(5.0))
        );
    }
    for tcl in [
        NumberSyntax::Tcl84,
        NumberSyntax::Tcl85,
        NumberSyntax::Tcl90,
    ] {
        assert_eq!(dispatch_under("pow", &ints, tcl), Some(Num::Float(8.0)));
    }
}

#[test]
fn min_max_preserves_int_width() {
    assert_eq!(
        dispatch("max", &[Num::Int(1), Num::Int(9), Num::Int(3)]),
        Some(Num::Int(9))
    );
    assert_eq!(
        dispatch("min", &[Num::Int(5), Num::Float(2.5)]),
        Some(Num::Float(2.5))
    );
    // A mixed-type call whose winner is
    // the `Int` operand must return that `Int` unchanged, not a
    // re-widened `Float` — real Tcl preserves the winning argument's
    // own type (`expr {min(3, 5.5)}` is `3`, not `3.0`; confirmed
    // tclsh8.6/9.0). The case above (`min(5, 2.5)`) doesn't catch this:
    // its winner is already the `Float` operand, so widening the
    // non-winning side is invisible there.
    assert_eq!(
        dispatch("min", &[Num::Int(3), Num::Float(5.5)]),
        Some(Num::Int(3)),
        "min(3, 5.5): the int operand wins and must stay an Int, not become 3.0"
    );
    assert_eq!(
        dispatch("max", &[Num::Int(9), Num::Float(2.5)]),
        Some(Num::Int(9)),
        "max(9, 2.5): the int operand wins and must stay an Int, not become 9.0"
    );
    assert_eq!(dispatch("max", &[Num::Int(5), Num::Float(f64::NAN)]), None);
    assert_eq!(dispatch("min", &[Num::Float(f64::NAN), Num::Int(5)]), None);
}

#[test]
fn type_conversions() {
    assert_eq!(dispatch("abs", &[Num::Int(-7)]), Some(Num::Int(7)));
    assert_eq!(dispatch("int", &[Num::Float(3.9)]), Some(Num::Int(3)));
    assert_eq!(dispatch("round", &[Num::Float(2.5)]), Some(Num::Int(3))); // ties away from 0
    assert_eq!(dispatch("round", &[Num::Float(-2.5)]), Some(Num::Int(-3)));
    assert_eq!(
        dispatch("int", &[Num::Float(I64_MIN_F64)]),
        Some(Num::Int(i64::MIN))
    );
    // `Num` is `NumValue<NoBig>` — the backend-less shape a const-folder
    // uses. A double outside the wide range has no representation there,
    // so the conversion *abstains* (the folder leaves the call to run at
    // run time) rather than folding a lossy or wrong value. An adopter
    // with a real bignum rung gets the exact answer instead — see
    // `bignum_backed_conversions_are_exact`.
    assert_eq!(dispatch("int", &[Num::Float(1.0e20)]), None);
    assert_eq!(dispatch("entier", &[Num::Float(1.0e20)]), None);
    assert_eq!(dispatch("wide", &[Num::Float(1.0e20)]), None);
    assert_eq!(dispatch("round", &[Num::Float(1.0e20)]), None);
    assert_eq!(dispatch("abs", &[Num::Float(f64::NAN)]), None);
    assert_eq!(dispatch("double", &[Num::Float(f64::NAN)]), None);
    assert_eq!(dispatch("bool", &[Num::Float(f64::NAN)]), None);
    assert_eq!(dispatch("ceil", &[Num::Float(f64::NAN)]), None);
    assert_eq!(dispatch("double", &[Num::Int(5)]), Some(Num::Float(5.0)));
    assert_eq!(
        dispatch("isnan", &[Num::Float(f64::NAN)]),
        Some(Num::Int(1))
    );
    assert_eq!(
        dispatch("isfinite", &[Num::Float(f64::NAN)]),
        Some(Num::Int(0))
    );
    assert_eq!(
        dispatch("isinf", &[Num::Float(f64::NAN)]),
        Some(Num::Int(0))
    );
}

/// The wide-range edges every integer/double conversion in the table has
/// to survive: the most negative wide (whose `abs` and negation promote),
/// the first doubles past each wide bound, and the non-finite operands.
fn edge_operands<B: BigIntOps>() -> Vec<NumValue<B>> {
    vec![
        NumValue::Int(i64::MIN),
        NumValue::Int(i64::MIN + 1),
        NumValue::Int(-1),
        NumValue::Int(0),
        NumValue::Int(1),
        NumValue::Int(i64::MAX),
        NumValue::Float(I64_MIN_F64),
        NumValue::Float(I64_MAX_PLUS_ONE_F64),
        NumValue::Float(1.0e19),
        NumValue::Float(1.0e300),
        NumValue::Float(-1.0e300),
        NumValue::Float(f64::INFINITY),
        NumValue::Float(f64::NAN),
        NumValue::Float(-0.0),
        NumValue::Float(5e-324),
    ]
}

const WIDTHS: [IntWidth; 3] = [
    IntWidth::Windowed,
    IntWidth::Unbounded,
    IntWidth::Unresolved,
];

/// `abs(-9223372036854775808)` is `2^63`, past the wide rung. A backend
/// with no bignum rung (`Num`, the const-folder's shape) must decline —
/// it once reached `NoBig::from_i64` and aborted `tcl opt` / `tcl diag`
/// (#2401). tclsh 8.5 to 9.1 answer `9223372036854775808`, 8.4 raises
/// `ARITH IOVERFLOW`, and jimsh 0.84 wraps to `-9223372036854775808`, so
/// no single wide answer is right either.
#[test]
fn abs_of_the_most_negative_wide_declines_without_a_bignum_rung() {
    assert_eq!(dispatch("abs", &[Num::Int(i64::MIN)]), None);
    const { assert!(!NoBig::HAS_BIGNUM_RUNG) };
    assert!(NoBig::promote_i64(i64::MIN).is_none());
    assert!(NoBig::from_f64_trunc(1.0e300).is_none());
    for width in WIDTHS {
        assert_eq!(
            try_dispatch_with_backend_int_width::<NoBig>("abs", &[Num::Int(i64::MIN)], width),
            Err(MathFuncError::Abstain),
            "{width:?}"
        );
    }
    assert_eq!(
        dispatch("abs", &[Num::Int(i64::MIN + 1)]),
        Some(Num::Int(i64::MAX))
    );
    assert_eq!(
        dispatch("abs", &[Num::Float(I64_MIN_F64)]),
        Some(Num::Float(-I64_MIN_F64))
    );
}

/// No call over the wide edges reaches the unreachable `NoBig::from_i64`:
/// every function, every arity, every width.
#[test]
fn a_backend_without_a_bignum_rung_never_panics() {
    let operands = edge_operands::<NoBig>();
    for &name in ALL_NAMES {
        for width in WIDTHS {
            for a in &operands {
                let _ = try_dispatch_with_backend_int_width(name, &[*a], width);
                for b in &operands {
                    let _ = try_dispatch_with_backend_int_width(name, &[*a, *b], width);
                    for c in &operands {
                        let _ = try_dispatch_with_backend_int_width(name, &[*a, *b, *c], width);
                    }
                }
            }
        }
    }
}

/// The no-bignum backend only ever declines: whenever `Num` answers, a real
/// bignum backend gives the identical wide or double, and a bignum answer
/// (or a conversion whose exact intermediate is one) is declined, never
/// approximated.
#[cfg(feature = "num-bigint")]
#[test]
fn a_backend_without_a_bignum_rung_agrees_wherever_it_answers() {
    use num_bigint::BigInt;
    let narrow = edge_operands::<NoBig>();
    let wide = edge_operands::<BigInt>();
    let agree = |name: &str, width, n: &[Num], w: &[NumValue<BigInt>]| {
        let want = dispatch_with_backend_int_width::<BigInt>(name, w, width);
        let got = dispatch_with_backend_int_width::<NoBig>(name, n, width);
        match (want, got) {
            (Some(NumValue::Int(a)), Some(NumValue::Int(b))) => assert_eq!(a, b, "{name}"),
            (Some(NumValue::Float(a)), Some(NumValue::Float(b))) => {
                assert_eq!(a.to_bits(), b.to_bits(), "{name}");
            }
            (_, None) => {}
            (want, got) => panic!("{name}{n:?} {width:?}: {want:?} vs {got:?}"),
        }
    };
    for &name in ALL_NAMES {
        for width in WIDTHS {
            for (i, a) in narrow.iter().enumerate() {
                agree(name, width, &[*a], &[wide[i].clone()]);
                for (j, b) in narrow.iter().enumerate() {
                    agree(name, width, &[*a, *b], &[wide[i].clone(), wide[j].clone()]);
                }
            }
        }
    }
}

/// `round()` is half away from zero on the *exact* operand — C's `modf`
/// form, not `floor(d + 0.5)`, which rounds twice and answers `1` for
/// `0.499_999_999_999_999_94` and `4_503_599_627_370_498` for `2**52 + 1`. Rows
/// measured on tclsh 8.6.16 and 9.0.4.
#[test]
fn round_is_half_away_from_zero_on_the_exact_value() {
    for (operand, want) in [
        (0.499_999_999_999_999_94, 0),
        (-0.499_999_999_999_999_94, 0),
        (0.500_000_000_000_000_1, 1),
        (0.5, 1),
        (1.5, 2),
        (2.5, 3),
        (-2.5, -3),
        (4_503_599_627_370_497.0, 4_503_599_627_370_497),
        (-4_503_599_627_370_497.0, -4_503_599_627_370_497),
        (2_251_799_813_685_248.5, 2_251_799_813_685_249),
        (-2_251_799_813_685_248.5, -2_251_799_813_685_249),
    ] {
        assert_eq!(
            dispatch("round", &[Num::Float(operand)]),
            Some(Num::Int(want)),
            "round({operand:?})"
        );
    }
}

#[cfg(feature = "num-bigint")]
#[test]
fn bignum_mathfunc_rung_is_exact() {
    use num_bigint::BigInt;
    let two_200: BigInt = BigInt::from(1u8) << 200;
    let two_100: BigInt = BigInt::from(1u8) << 100;
    let nums = |n: BigInt| NumValue::<BigInt>::Big(n);
    assert_eq!(
        dispatch_with_backend("abs", &[NumValue::<BigInt>::Int(i64::MIN)]),
        Some(nums(BigInt::from(1u8) << 63))
    );
    assert_eq!(
        dispatch_with_backend("round", &[nums(two_200.clone())]),
        Some(nums(two_200.clone()))
    );
    assert_eq!(
        dispatch_with_backend("isqrt", &[nums(two_200.clone())]),
        Some(nums(two_100))
    );
    assert_eq!(
        dispatch_with_backend("max", &[nums(BigInt::from(7)), NumValue::<BigInt>::Int(9)]),
        Some(NumValue::<BigInt>::Int(9))
    );
    assert_eq!(
        dispatch_with_backend(
            "min",
            &[nums(BigInt::from(-7)), NumValue::<BigInt>::Int(-9)]
        ),
        Some(NumValue::<BigInt>::Int(-9))
    );
    for n in 0..=256i64 {
        let expected = (n as f64).sqrt() as i64;
        assert_eq!(
            dispatch("isqrt", &[Num::Int(n)]),
            Some(Num::Int(expected)),
            "isqrt({n})"
        );
    }
}

#[test]
fn isqrt_accepts_a_float_operand() {
    // Tcl accepts a float operand to `isqrt`, truncating it toward
    // zero first (confirmed tclsh8.6/9.0):
    //   expr {isqrt(9.0)}      -> 3
    //   expr {isqrt(9.5)}      -> 3   (truncates to 9, same as isqrt(9))
    //   expr {isqrt(15.9999)}  -> 3   (truncates to 15, not 16)
    //   expr {isqrt(16.0)}     -> 4
    //   expr {isqrt(-1.0)}     -> domain error, same as isqrt(-1)
    assert_eq!(dispatch("isqrt", &[Num::Float(9.0)]), Some(Num::Int(3)));
    assert_eq!(dispatch("isqrt", &[Num::Float(9.5)]), Some(Num::Int(3)));
    assert_eq!(dispatch("isqrt", &[Num::Float(15.9999)]), Some(Num::Int(3)));
    assert_eq!(dispatch("isqrt", &[Num::Float(16.0)]), Some(Num::Int(4)));
    assert_eq!(dispatch("isqrt", &[Num::Float(0.5)]), Some(Num::Int(0)));
    assert_eq!(dispatch("isqrt", &[Num::Float(-1.0)]), None);
    assert_eq!(dispatch("isqrt", &[Num::Float(f64::INFINITY)]), None);
    assert_eq!(dispatch("isqrt", &[Num::Float(f64::NAN)]), None);
}

#[test]
fn fp_classification() {
    // isnormal: integers widen to finite normal doubles; zero/NaN are not.
    assert_eq!(dispatch("isnormal", &[Num::Float(1.0)]), Some(Num::Int(1)));
    assert_eq!(dispatch("isnormal", &[Num::Int(7)]), Some(Num::Int(1)));
    assert_eq!(dispatch("isnormal", &[Num::Float(0.0)]), Some(Num::Int(0)));
    assert_eq!(
        dispatch("isnormal", &[Num::Float(f64::NAN)]),
        Some(Num::Int(0))
    );
    assert_eq!(
        dispatch("isnormal", &[Num::Float(f64::MIN_POSITIVE / 2.0)]),
        Some(Num::Int(0))
    );
    // issubnormal: the smallest denormal is subnormal; 1.0 is not.
    assert_eq!(
        dispatch("issubnormal", &[Num::Float(f64::from_bits(1))]),
        Some(Num::Int(1))
    );
    assert_eq!(
        dispatch("issubnormal", &[Num::Float(1.0)]),
        Some(Num::Int(0))
    );
    // isunordered: 1 iff either operand is NaN.
    assert_eq!(
        dispatch("isunordered", &[Num::Float(f64::NAN), Num::Int(1)]),
        Some(Num::Int(1))
    );
    assert_eq!(
        dispatch("isunordered", &[Num::Int(1), Num::Float(2.0)]),
        Some(Num::Int(0))
    );
    assert_eq!(dispatch("isunordered", &[Num::Int(1)]), None); // wrong arity
}

#[test]
fn unknown_and_arity() {
    assert_eq!(dispatch("frobnicate", &[Num::Int(1)]), None);
    assert_eq!(dispatch("sqrt", &[Num::Int(1), Num::Int(2)]), None); // wrong arity
}

/// `added_in()` and `dispatch()` must agree on exactly which names are
/// implemented: `added_in()` claiming Tcl 9.1 support for a function
/// `dispatch()` does not implement at all is exactly the drift this
/// guards against. For every name `added_in()` recognises
/// (except `rand`/`srand`, the caller's responsibility), `dispatch()`
/// must produce a real value for at least one in-domain argument list —
/// not just "some arity returns `None`", which a merely-missing arm
/// would also satisfy.
#[test]
fn added_in_and_dispatch_agree() {
    const ALL_NAMES: &[&str] = &[
        "abs",
        "acos",
        "acosh",
        "asin",
        "asinh",
        "atan",
        "atan2",
        "atanh",
        "bool",
        "cbrt",
        "ceil",
        "copysign",
        "cos",
        "cosh",
        "dim",
        "double",
        "entier",
        "erf",
        "erfc",
        "exp",
        "exp2",
        "expm1",
        "floor",
        "fma",
        "fmod",
        "gamma",
        "hypot",
        "int",
        "isfinite",
        "isinf",
        "isnan",
        "isnormal",
        "isqrt",
        "issubnormal",
        "isunordered",
        "ldexp",
        "lgamma",
        "log",
        "log10",
        "log1p",
        "log2",
        "logb",
        "max",
        "min",
        "nextafter",
        "pow",
        "rand",
        "remainder",
        "round",
        "signbit",
        "sin",
        "sinh",
        "sqrt",
        "srand",
        "tan",
        "tanh",
        "trunc",
        "wide",
    ];
    for &name in ALL_NAMES {
        if name == "rand" || name == "srand" {
            continue;
        }
        assert!(
            added_in(name).is_some(),
            "{name}: missing from added_in() — test's own name list is stale"
        );
        let in_domain_args = in_domain_args_for(name);
        assert!(
            dispatch(name, &in_domain_args).is_some(),
            "{name}: added_in() claims support but dispatch({name}, {in_domain_args:?}) is None"
        );
    }
}

/// A safely in-domain argument list for `name`, sized to its real arity
/// — used only by the agreement test above.
fn in_domain_args_for(name: &str) -> Vec<Num> {
    match name {
        "atan2" | "hypot" | "fmod" | "pow" | "copysign" | "dim" | "nextafter" | "isunordered" => {
            vec![Num::Float(1.5), Num::Float(2.5)]
        }
        "remainder" => vec![Num::Float(5.0), Num::Float(3.0)],
        "ldexp" => vec![Num::Float(1.5), Num::Int(4)],
        "fma" => vec![Num::Float(2.0), Num::Float(3.0), Num::Float(4.0)],
        "min" | "max" => vec![Num::Int(1), Num::Int(2)],
        "acosh" | "log" | "log10" | "log2" | "logb" => vec![Num::Float(2.0)],
        "atanh" | "asin" | "acos" => vec![Num::Float(0.5)],
        "log1p" => vec![Num::Float(1.0)],
        "gamma" | "lgamma" => vec![Num::Float(5.0)],
        "isqrt" => vec![Num::Int(9)],
        _ => vec![Num::Float(1.5)],
    }
}

#[test]
fn tip745_c99_batch() {
    // Domain-safe spot checks (values chosen so no platform-dependent
    // ULP drift can flip a NaN/finite boundary).
    assert_eq!(dispatch("acosh", &[Num::Int(1)]), Some(Num::Float(0.0)));
    assert!(dispatch("acosh", &[Num::Float(0.5)]).is_none()); // domain: x < 1
    assert_eq!(dispatch("asinh", &[Num::Int(0)]), Some(Num::Float(0.0)));
    assert_eq!(dispatch("atanh", &[Num::Int(0)]), Some(Num::Float(0.0)));
    assert!(dispatch("atanh", &[Num::Int(2)]).is_none()); // domain: |x| > 1
    assert_eq!(dispatch("cbrt", &[Num::Int(-8)]), Some(Num::Float(-2.0)));
    assert_eq!(
        dispatch("copysign", &[Num::Float(3.0), Num::Float(-1.0)]),
        Some(Num::Float(-3.0))
    );
    assert_eq!(
        dispatch("dim", &[Num::Float(1.0), Num::Float(5.0)]),
        Some(Num::Float(0.0))
    );
    assert_eq!(
        dispatch("dim", &[Num::Float(5.0), Num::Float(1.0)]),
        Some(Num::Float(4.0))
    );
    assert_eq!(dispatch("erf", &[Num::Int(0)]), Some(Num::Float(0.0)));
    assert_eq!(dispatch("erfc", &[Num::Int(0)]), Some(Num::Float(1.0)));
    assert_eq!(dispatch("exp2", &[Num::Int(10)]), Some(Num::Float(1024.0)));
    assert_eq!(dispatch("expm1", &[Num::Int(0)]), Some(Num::Float(0.0)));
    assert_eq!(
        dispatch("fma", &[Num::Float(2.0), Num::Float(3.0), Num::Float(4.0)]),
        Some(Num::Float(10.0))
    );
    // `gamma(n) == (n-1)!` for positive integers.
    assert_eq!(dispatch("gamma", &[Num::Int(5)]), Some(Num::Float(24.0)));
    // `gamma(0)` is a pole approached from both sides (`1/x`-like): C's
    // `tgamma` returns `+Inf` with a *range* error there, which Tcl's
    // `CheckDoubleResult` silently accepts (only a NaN result, as at the
    // negative-integer poles below, is a domain error).
    assert_eq!(
        dispatch("gamma", &[Num::Int(0)]),
        Some(Num::Float(f64::INFINITY))
    );
    assert!(dispatch("gamma", &[Num::Int(-1)]).is_none()); // domain: negative-integer pole
    assert_eq!(
        dispatch("ldexp", &[Num::Float(1.5), Num::Int(4)]),
        Some(Num::Float(24.0))
    );
    assert_eq!(dispatch("ldexp", &[Num::Float(1.5), Num::Float(4.0)]), None); // exponent must be int
    assert_eq!(dispatch("lgamma", &[Num::Int(1)]), Some(Num::Float(0.0)));
    // Unlike `gamma`, `lgamma` is `+Inf` (accepted, not a domain error) at
    // *every* integer pole, including the negative ones — verified
    // against real `tclsh9.1` (`lgamma(-1)` => `Inf`, not an error).
    assert_eq!(
        dispatch("lgamma", &[Num::Int(-1)]),
        Some(Num::Float(f64::INFINITY))
    );
    assert_eq!(dispatch("log1p", &[Num::Int(0)]), Some(Num::Float(0.0)));
    assert_eq!(dispatch("log2", &[Num::Int(8)]), Some(Num::Float(3.0)));
    assert_eq!(dispatch("logb", &[Num::Int(8)]), Some(Num::Float(3.0)));
    assert_eq!(
        dispatch("logb", &[Num::Float(0.0)]),
        Some(Num::Float(f64::NEG_INFINITY))
    );
    assert_eq!(
        dispatch("logb", &[Num::Float(f64::INFINITY)]),
        Some(Num::Float(f64::INFINITY))
    );
    assert!(
        dispatch("nextafter", &[Num::Float(1.0), Num::Float(2.0)])
            .unwrap()
            .as_f64()
            > 1.0
    );
    assert_eq!(
        dispatch("remainder", &[Num::Float(5.0), Num::Float(3.0)]),
        Some(Num::Float(-1.0))
    );
    assert_eq!(dispatch("signbit", &[Num::Float(-1.0)]), Some(Num::Int(1)));
    assert_eq!(dispatch("signbit", &[Num::Float(1.0)]), Some(Num::Int(0)));
    assert_eq!(dispatch("signbit", &[Num::Int(-5)]), Some(Num::Int(1)));
    assert_eq!(dispatch("trunc", &[Num::Float(3.7)]), Some(Num::Float(3.0)));
    assert_eq!(
        dispatch("trunc", &[Num::Float(-3.7)]),
        Some(Num::Float(-3.0))
    );
    // NaN input is a domain error for every unary/binary member of the batch.
    for name in [
        "acosh", "asinh", "atanh", "cbrt", "erf", "erfc", "exp2", "expm1", "gamma", "lgamma",
        "log1p", "log2", "logb", "trunc",
    ] {
        assert_eq!(dispatch(name, &[Num::Float(f64::NAN)]), None, "{name}(NaN)");
    }
}

#[test]
fn spec_agrees_with_added_in_for_every_name() {
    for &name in ALL_NAMES {
        let s = spec(name).unwrap_or_else(|| panic!("spec({name}) is None"));
        assert_eq!(s.name, name);
        assert_eq!(
            Some(s.since),
            added_in(name),
            "{name}: spec().since disagrees with added_in()"
        );
    }
    assert_eq!(spec("not_a_real_function"), None);
    assert_eq!(all().len(), ALL_NAMES.len());
}

#[test]
fn spec_arity_matches_dispatch_shape() {
    use super::super::operators::CommandArity;

    assert_eq!(spec("sqrt").unwrap().arity, CommandArity::exact(1));
    assert_eq!(spec("atan2").unwrap().arity, CommandArity::exact(2));
    assert_eq!(spec("fma").unwrap().arity, CommandArity::exact(3));
    assert_eq!(spec("rand").unwrap().arity, CommandArity::exact(0));
    assert_eq!(spec("max").unwrap().arity, CommandArity::at_least(1));
    assert_eq!(spec("min").unwrap().arity, CommandArity::at_least(1));
    assert_eq!(spec("isunordered").unwrap().arity, CommandArity::exact(2));
    assert_eq!(spec("ldexp").unwrap().arity, CommandArity::exact(2));
}

#[test]
fn spec_accepts_boolean_operand_matches_the_standalone_query() {
    for &name in ALL_NAMES {
        assert_eq!(
            spec(name).unwrap().accepts_boolean_operand,
            accepts_boolean_operand(name),
            "{name}"
        );
    }
}

/// With a real arbitrary-precision backend, `entier`/`round`
/// convert a double of any magnitude exactly (TIP 237), and
/// `wide` truncates then takes the low 64 bits. Every expectation is
/// tclsh 9.0.4 / 8.6.16 output (the two releases agree on all of these).
#[cfg(feature = "num-bigint")]
#[test]
fn bignum_backed_conversions_are_exact() {
    use num_bigint::BigInt;
    type N = NumValue<BigInt>;
    let big = |s: &str| BigInt::parse_bytes(s.as_bytes(), 10).expect("decimal");

    // tclsh: `entier(1e300)` is a 301-digit integer, `1e300`'s exact
    // double value — not `10^300`.
    let want_1e300 = big(
        "1000000000000000052504760255204420248704468581108159154915854115511802457988908195786371375080447864043704443832883878176942523235360430575644792184786706982848387200926575803737830233794788090059368953234970799945081119038967640880074652742780142494579258788820056842838115669472196386865459400540160",
    );
    assert_eq!(
        dispatch_with_backend::<BigInt>("entier", &[N::Float(1.0e300)]),
        Some(N::Big(want_1e300.clone()))
    );
    assert_eq!(
        dispatch_with_backend::<BigInt>("round", &[N::Float(1.0e300)]),
        Some(N::Big(want_1e300.clone()))
    );
    assert_eq!(
        dispatch_with_backend::<BigInt>("round", &[N::Float(-1.0e300)]),
        Some(N::Big(-want_1e300))
    );
    assert_eq!(
        dispatch_with_backend::<BigInt>("entier", &[N::Float(1.0e20)]),
        Some(N::Big(big("100000000000000000000")))
    );
    assert_eq!(
        dispatch_with_backend::<BigInt>("round", &[N::Float(1.0e20)]),
        Some(N::Big(big("100000000000000000000")))
    );
    // `wide(1e20)`: truncate, then the low 64 bits — both releases.
    assert_eq!(
        dispatch_with_backend::<BigInt>("wide", &[N::Float(1.0e20)]),
        Some(N::Int(7_766_279_631_452_241_920))
    );
    assert_eq!(
        dispatch_with_backend::<BigInt>("wide", &[N::Float(-1.0e20)]),
        Some(N::Int(-7_766_279_631_452_241_920))
    );
    // A value that still fits a wide stays on the wide rung.
    assert_eq!(
        dispatch_with_backend::<BigInt>("entier", &[N::Float(1.9)]),
        Some(N::Int(1))
    );
    assert_eq!(
        dispatch_with_backend::<BigInt>("entier", &[N::Float(-1.9)]),
        Some(N::Int(-1))
    );
    // Non-finite operands stay refusals (the caller's IOVERFLOW / NaN
    // errors).
    assert_eq!(
        dispatch_with_backend::<BigInt>("entier", &[N::Float(f64::INFINITY)]),
        None
    );
    assert_eq!(
        dispatch_with_backend::<BigInt>("round", &[N::Float(f64::NAN)]),
        None
    );
    // tclsh: `isqrt(1e300)` is a 151-digit integer, not the root of a
    // saturated wide.
    assert_eq!(
        dispatch_with_backend::<BigInt>("isqrt", &[N::Float(1.0e300)]),
        Some(N::Big(big(
            "1000000000000000026252380127602209779758503108492371458359424883684651414333812736380124287612629691547944630047071980611862607399628869272326975124240"
        )))
    );
}

/// `int()` is the one release-split conversion. Measured:
/// tclsh8.6.16 `int(1e20)` is `7766279631452241920` and `int(2**64+1)` is
/// `1`; tclsh9.0.4 gives `100000000000000000000` and
/// `18446744073709551617`.
#[cfg(feature = "num-bigint")]
#[test]
fn int_follows_the_release_width() {
    use num_bigint::BigInt;
    type N = NumValue<BigInt>;
    let big = |s: &str| BigInt::parse_bytes(s.as_bytes(), 10).expect("decimal");
    let call = |w| dispatch_with_backend_int_width::<BigInt>("int", &[N::Float(1.0e20)], w);

    assert_eq!(
        call(IntWidth::Unbounded),
        Some(N::Big(big("100000000000000000000")))
    );
    assert_eq!(
        call(IntWidth::Windowed),
        Some(N::Int(7_766_279_631_452_241_920))
    );
    // Unresolved never guesses: it abstains exactly where the two
    // releases disagree, so a const-folder cannot bake in the wrong one.
    assert_eq!(call(IntWidth::Unresolved), None);

    let two_64_1 = N::Big(big("18446744073709551617"));
    assert_eq!(
        dispatch_with_backend_int_width::<BigInt>(
            "int",
            std::slice::from_ref(&two_64_1),
            IntWidth::Unbounded
        ),
        Some(two_64_1.clone())
    );
    assert_eq!(
        dispatch_with_backend_int_width::<BigInt>(
            "int",
            std::slice::from_ref(&two_64_1),
            IntWidth::Windowed
        ),
        Some(N::Int(1))
    );
    assert_eq!(
        dispatch_with_backend_int_width::<BigInt>(
            "int",
            std::slice::from_ref(&two_64_1),
            IntWidth::Unresolved
        ),
        None
    );

    // `entier` and `wide` are release-invariant, so every width agrees.
    for w in [
        IntWidth::Unbounded,
        IntWidth::Windowed,
        IntWidth::Unresolved,
    ] {
        assert_eq!(
            dispatch_with_backend_int_width::<BigInt>("entier", &[N::Float(1.0e20)], w),
            Some(N::Big(big("100000000000000000000"))),
            "entier at {w:?}"
        );
        assert_eq!(
            dispatch_with_backend_int_width::<BigInt>("wide", &[N::Float(1.0e20)], w),
            Some(N::Int(7_766_279_631_452_241_920)),
            "wide at {w:?}"
        );
    }
    assert_eq!(
        IntWidth::for_tcl_version(tcl_dialect::TclVersion::V8_6),
        IntWidth::Windowed
    );
    assert_eq!(
        IntWidth::for_tcl_version(tcl_dialect::TclVersion::V9_0),
        IntWidth::Unbounded
    );
    assert_eq!(
        IntWidth::for_tcl_version(tcl_dialect::TclVersion::V9_1),
        IntWidth::Unbounded
    );
}

/// The object-preserving integer policy has exactly one owner. Both
/// engines (`tcl-vm`'s `cmd_math` and `runtime/rust`'s `cmd_mathfunc`)
/// read [`integer_conversion`]; before this existed each carried its own
/// `wide`/`int`/`entier` name match and the two could drift.
#[test]
fn integer_conversion_owns_the_preserve_versus_window_policy() {
    use IntegerConversion::{Preserve, Window};

    // `wide` windows and `entier` preserves in every release.
    for w in [
        IntWidth::Windowed,
        IntWidth::Unbounded,
        IntWidth::Unresolved,
    ] {
        assert_eq!(integer_conversion("wide", w), Some(Window), "wide {w:?}");
        assert_eq!(
            integer_conversion("entier", w),
            Some(Preserve),
            "entier {w:?}"
        );
    }

    // `int` follows the release axis, and abstains when it is unresolved
    // rather than baking in one release's answer.
    assert_eq!(integer_conversion("int", IntWidth::Windowed), Some(Window));
    assert_eq!(
        integer_conversion("int", IntWidth::Unbounded),
        Some(Preserve)
    );
    assert_eq!(integer_conversion("int", IntWidth::Unresolved), None);

    // Tied to the releases themselves, not just to the axis enum.
    for (v, want) in [
        (tcl_dialect::TclVersion::V8_4, Window),
        (tcl_dialect::TclVersion::V8_5, Window),
        (tcl_dialect::TclVersion::V8_6, Window),
        (tcl_dialect::TclVersion::V9_0, Preserve),
        (tcl_dialect::TclVersion::V9_1, Preserve),
    ] {
        assert_eq!(
            integer_conversion("int", IntWidth::for_tcl_version(v)),
            Some(want),
            "int under {v:?}"
        );
    }

    // `round`/`isqrt` convert to an integer but always compute a new
    // value, so they are not preservation candidates; nor is anything
    // outside the three.
    for name in ["round", "isqrt", "abs", "double", "sqrt", "frobnicate"] {
        for w in [
            IntWidth::Windowed,
            IntWidth::Unbounded,
            IntWidth::Unresolved,
        ] {
            assert_eq!(integer_conversion(name, w), None, "{name} {w:?}");
        }
    }
}
