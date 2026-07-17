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

//! `expr` math functions, registered as `tcl::mathfunc::*` (the names the
//! compiler invokes and `ExprEval::call` routes to).
// The math functions intentionally coerce between i64 and f64 (`int`/`round`/
// `entier`/… follow Tcl's expr numeric conversions, not lossless casts).
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use num_traits::{FromPrimitive, Signed, ToPrimitive};
use tcl_runtime_api::Completion;
use tcl_syntax::number::{self, Number};

use crate::command::err_with_code;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

/// Coerce `v` to a double for the number-flavoured math functions
/// (`abs`/`int`/`round`/`entier`/`isqrt`/`max`/`min`). Their non-numeric error
/// in Tcl 9 is the generic `expected number but got "…"` — not the
/// floating-point-specific wording `as_double` produces.
fn num_arg(v: &Value) -> Result<f64, String> {
    v.as_double()
        .map_err(|_| format!("expected number but got \"{}\"", v.to_str()))
}

/// Coerce `v` to a double for the classification predicates (`isnan` /
/// `isunordered` / …), which deliberately accept a literal `NaN` — inspecting
/// NaN/Inf is their purpose — even though a bare `NaN` is a domain error as an
/// ordinary operand.
/// `fpclassify floatValue` — the top-level command (`tclBasic.c`
/// `FloatClassifyObjCmd`): classify a number as zero / subnormal / normal /
/// infinite / nan. An integer coerces to its double value first (so `0` is
/// `zero`, any other integer `normal`); a non-number errors.
fn cmd_fpclassify(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [v] = args else {
        return err("wrong # args: should be \"fpclassify floatValue\"");
    };
    match num_or_nan(v) {
        Ok(d) => ok(Value::string(match d.classify() {
            std::num::FpCategory::Nan => "nan",
            std::num::FpCategory::Infinite => "infinite",
            std::num::FpCategory::Zero => "zero",
            std::num::FpCategory::Subnormal => "subnormal",
            std::num::FpCategory::Normal => "normal",
        })),
        Err(m) => err(m),
    }
}

fn num_or_nan(v: &Value) -> Result<f64, String> {
    if let Ok(d) = v.as_double() {
        return Ok(d);
    }
    match number::parse_whole(v.to_str().trim()) {
        Some(Number::Nan { .. }) => Ok(f64::NAN),
        // A bignum coerces to its nearest double (overflowing to ±Inf past the
        // double range), matching C's `Tcl_GetDoubleFromObj` — the same
        // `TclBignumToDouble` rounding the expr tower uses.
        Some(Number::Big { .. }) => {
            Ok(crate::expr::value_as_bigint(v).map_or(f64::NAN, |b| crate::expr::big_to_f64(&b)))
        }
        _ => Err(format!("expected number but got \"{}\"", v.to_str())),
    }
}

/// Coerce the argument of an integer-producing conversion (`int`/`wide`/
/// `entier`/`round`) to a *finite* double, with C's special-value errors:
/// NaN — spelled or typed — is "floating point value is Not a Number" and
/// ±Inf is "integer value too large to represent" (tclsh 8.6/9.0); a
/// non-number keeps the generic expected-number message. Callers handle
/// integer arguments first, so a bignum never reaches here.
fn finite_num_arg(v: &Value) -> Result<f64, String> {
    let f = num_or_nan(v)?;
    if f.is_nan() {
        return Err("floating point value is Not a Number".to_string());
    }
    if f.is_infinite() {
        return Err("integer value too large to represent".to_string());
    }
    Ok(f)
}

/// A finite double truncated toward zero, as an exact integer — `from_f64` on
/// a finite integral double is lossless, so `int(1e300)`-style conversions
/// see the full 10^300-scale value C does (the `None` fallback is unreachable
/// for the finite values [`finite_num_arg`] admits).
fn exact_trunc(f: f64) -> num_bigint::BigInt {
    num_bigint::BigInt::from_f64(f.trunc()).unwrap_or_default()
}

/// Fold an exact integer into the signed 64-bit window (modulo 2^64, bit-cast
/// to `i64`) — what `int()`/`wide()` do to an out-of-range value. The
/// integer-*string* case lives in `Value::as_wide`; this is the same fold for
/// values arriving as a bignum (a truncated double).
fn wide_window(b: &num_bigint::BigInt) -> i64 {
    // num-bigint's `&` is two's-complement, so the mask keeps the low 64 bits
    // with the wrap C computes (`-1 & mask` is 2^64-1, folding back to `-1`).
    let low = b & &num_bigint::BigInt::from(u64::MAX);
    low.to_u64().map_or(0, u64::cast_signed)
}

/// The message and `errorCode` C raises (`tclExecute.c`, errno `EDOM`) when a
/// math function's argument is out of range — `sqrt(-1)`, `acos(2)`, `fmod(x,0)`,
/// … . `isqrt` of a negative reuses the same code with its own message.
const DOMAIN_MSG: &str = "domain error: argument not in valid range";
const DOMAIN_CODE: &str = "ARITH DOMAIN {domain error: argument not in valid range}";

fn domain_err() -> Completion<Value> {
    err_with_code(DOMAIN_MSG, DOMAIN_CODE)
}

pub(crate) fn register(vm: &mut Vm) {
    // `::tcl::mathfunc` is a real namespace in C Tcl, so a user
    // `proc tcl::mathfunc::square {x} {…}` (TIP 232's custom-function
    // mechanism) must find it existing — declare it alongside the builtin
    // registrations (which only create flat command-table keys).
    vm.declare_namespace("tcl::mathfunc");
    vm.register("tcl::mathfunc::abs", m_abs);
    vm.register("tcl::mathfunc::int", m_int);
    vm.register("tcl::mathfunc::wide", m_wide);
    vm.register("tcl::mathfunc::entier", m_entier);
    vm.register("tcl::mathfunc::double", m_double);
    vm.register("tcl::mathfunc::round", m_round);
    vm.register("tcl::mathfunc::sqrt", m_sqrt);
    vm.register("tcl::mathfunc::isqrt", m_isqrt);
    vm.register("tcl::mathfunc::floor", m_floor);
    vm.register("tcl::mathfunc::ceil", m_ceil);
    vm.register("tcl::mathfunc::pow", m_pow);
    vm.register("tcl::mathfunc::bool", m_bool);
    vm.register("tcl::mathfunc::max", m_max);
    vm.register("tcl::mathfunc::min", m_min);
    vm.register("tcl::mathfunc::srand", m_srand);
    vm.register("tcl::mathfunc::rand", m_rand);
    // Trigonometric / transcendental — single `double` argument.
    vm.register("tcl::mathfunc::sin", |_, a| dom_fn(a, "sin", f64::sin));
    vm.register("tcl::mathfunc::cos", |_, a| dom_fn(a, "cos", f64::cos));
    vm.register("tcl::mathfunc::tan", |_, a| dom_fn(a, "tan", f64::tan));
    vm.register("tcl::mathfunc::asin", |_, a| dom_fn(a, "asin", f64::asin));
    vm.register("tcl::mathfunc::acos", |_, a| dom_fn(a, "acos", f64::acos));
    vm.register("tcl::mathfunc::atan", |_, a| dom_fn(a, "atan", f64::atan));
    vm.register("tcl::mathfunc::sinh", |_, a| dom_fn(a, "sinh", f64::sinh));
    vm.register("tcl::mathfunc::cosh", |_, a| dom_fn(a, "cosh", f64::cosh));
    vm.register("tcl::mathfunc::tanh", |_, a| dom_fn(a, "tanh", f64::tanh));
    vm.register("tcl::mathfunc::exp", |_, a| dom_fn(a, "exp", f64::exp));
    vm.register("tcl::mathfunc::log", |_, a| dom_fn(a, "log", f64::ln));
    vm.register("tcl::mathfunc::log10", |_, a| {
        dom_fn(a, "log10", f64::log10)
    });
    // Two `double` arguments.
    vm.register("tcl::mathfunc::atan2", |_, a| {
        dom_fn2(a, "atan2", f64::atan2)
    });
    vm.register("tcl::mathfunc::hypot", |_, a| {
        dom_fn2(a, "hypot", f64::hypot)
    });
    vm.register("tcl::mathfunc::fmod", |_, a| {
        dom_fn2(a, "fmod", |x, y| x % y)
    });
    // Classification predicates — one `double`, returns a boolean (never a
    // domain error, since inspecting a NaN/Inf is their purpose).
    vm.register("tcl::mathfunc::isfinite", |_, a| {
        pred_fn(a, "isfinite", f64::is_finite)
    });
    vm.register("tcl::mathfunc::isinf", |_, a| {
        pred_fn(a, "isinf", f64::is_infinite)
    });
    vm.register("tcl::mathfunc::isnan", |_, a| {
        pred_fn(a, "isnan", f64::is_nan)
    });
    vm.register("tcl::mathfunc::isnormal", |_, a| {
        pred_fn(a, "isnormal", f64::is_normal)
    });
    vm.register("tcl::mathfunc::issubnormal", |_, a| {
        pred_fn(a, "issubnormal", |x| {
            x.classify() == std::num::FpCategory::Subnormal
        })
    });
    vm.register("tcl::mathfunc::isunordered", |_, a| {
        pred_fn2(a, "isunordered", |x, y| x.is_nan() || y.is_nan())
    });
    // `fpclassify` is a top-level command, not a math function.
    vm.register("fpclassify", cmd_fpclassify);
}

/// A one-`double` classification predicate (`isnan`/`isinf`/…): coerce the
/// argument and return its boolean as `0`/`1`, with no domain check.
fn pred_fn(args: &[Value], name: &str, f: impl Fn(f64) -> bool) -> Completion<Value> {
    let x = match one(args, name) {
        Ok(v) => v,
        Err(c) => return c,
    };
    match num_or_nan(x) {
        Ok(d) => ok(Value::bool(f(d))),
        Err(m) => err(m),
    }
}

/// A two-`double` predicate (`isunordered`).
fn pred_fn2(args: &[Value], name: &str, f: impl Fn(f64, f64) -> bool) -> Completion<Value> {
    let [lhs, rhs] = args else {
        let which = if args.len() < 2 {
            "not enough arguments"
        } else {
            "too many arguments"
        };
        return err(format!("{which} for math function \"{name}\""));
    };
    match (num_or_nan(lhs), num_or_nan(rhs)) {
        (Ok(x), Ok(y)) => ok(Value::bool(f(x, y))),
        (Err(m), _) | (_, Err(m)) => err(m),
    }
}

fn one<'a>(args: &'a [Value], name: &str) -> Result<&'a Value, Completion<Value>> {
    match args {
        [x] => Ok(x),
        _ => Err(err(format!(
            "{} for math function \"{name}\"",
            if args.is_empty() {
                "not enough arguments"
            } else {
                "too many arguments"
            }
        ))),
    }
}

fn m_abs(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "abs") {
        Ok(v) => v,
        Err(c) => return c,
    };
    // Compute the magnitude in `i128` so the most-negative wide doesn't wrap:
    // `abs(-9223372036854775808)` is `2^63`, which has no `i64` but is the
    // bignum tclsh returns (rendered as a decimal string by `int_value`).
    if let Ok(n) = x.as_int() {
        return ok(crate::expr::int_value(i128::from(n).abs()));
    }
    if let Some(b) = x.as_i128() {
        return ok(crate::expr::int_value(b.saturating_abs()));
    }
    // An integer past `i128` stays exact too: tclsh's `abs(-(2**150))` is the
    // full 2^150, never a rounded double.
    if let Some(b) = crate::expr::value_as_bigint(x) {
        return ok(crate::expr::big_value(&b.abs()));
    }
    match num_arg(x) {
        Ok(f) => ok(Value::double(f.abs())),
        Err(m) => err(m),
    }
}

fn m_int(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    int_window(args, "int")
}

fn m_wide(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    int_window(args, "wide")
}

/// `int(x)` / `wide(x)` — one conversion since 8.6 (`int` is no longer the
/// narrower "long"): an integer of any magnitude is taken modulo 2^64,
/// two's-complement folded (`int(2**64 + 1)` is `1`, `int(-(2**64 + 1))` is
/// `-1`); a double truncates toward zero first, then wraps the same way
/// (`int(1e300)` is `0` — 10^300 divides by 2^64). The integer path is exact —
/// never round-tripped through `f64`, which would lose precision above 2^53
/// (`int(9007199254740993)` must stay `…993`, `RUST_ISSUE_096`).
fn int_window(args: &[Value], name: &str) -> Completion<Value> {
    let x = match one(args, name) {
        Ok(v) => v,
        Err(c) => return c,
    };
    // `as_wide` is exactly this window for integer inputs of any size.
    if let Ok(n) = x.as_wide() {
        return ok(Value::int(n));
    }
    match finite_num_arg(x) {
        Ok(f) => ok(Value::int(wide_window(&exact_trunc(f)))),
        Err(m) => err(m),
    }
}

fn m_double(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "double") {
        Ok(v) => v,
        Err(c) => return c,
    };
    match x.as_double() {
        // A typed NaN is not a usable number here, same as the spelled one
        // below (tclsh: "floating point value is Not a Number").
        Ok(f) if f.is_nan() => err("floating point value is Not a Number"),
        Ok(f) => ok(Value::double(f)),
        Err(e) => {
            // `as_double` declines integers past its range and NaN spellings;
            // both are still numbers to `double()`: a bignum converts with
            // C's `TclBignumToDouble` rounding (`double(2**200)` is
            // `1.6069380442589903e+60`), NaN is the domain-style error.
            if let Some(b) = crate::expr::value_as_bigint(x) {
                return ok(Value::double(crate::expr::big_to_f64(&b)));
            }
            if matches!(
                number::parse_whole(x.to_str().trim()),
                Some(Number::Nan { .. })
            ) {
                return err("floating point value is Not a Number");
            }
            err(e.message)
        }
    }
}

fn m_round(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "round") {
        Ok(v) => v,
        Err(c) => return c,
    };
    // Integers of any magnitude pass through exactly (tclsh: `round(2**200)`
    // is 2^200 itself).
    if let Ok(n) = x.as_int() {
        return ok(Value::int(n));
    }
    if let Some(b) = crate::expr::value_as_bigint(x) {
        return ok(crate::expr::big_value(&b));
    }
    // Round half away from zero, then keep the result *exact*: tclsh's
    // `round(1e300)` is the full 309-digit integer, not a saturated wide.
    match finite_num_arg(x) {
        Ok(f) => ok(crate::expr::big_value(
            &num_bigint::BigInt::from_f64(f.round()).unwrap_or_default(),
        )),
        Err(m) => err(m),
    }
}

fn dbl_fn(args: &[Value], name: &str, f: impl Fn(f64) -> f64) -> Completion<Value> {
    let x = match one(args, name) {
        Ok(v) => v,
        Err(c) => return c,
    };
    match x.as_double() {
        Ok(d) => ok(Value::double(f(d))),
        Err(e) => err(e.message),
    }
}

/// A single-`double` libm function with Tcl's domain guard: a non-NaN argument
/// that yields NaN (`sqrt(-1)`, `acos(2)`, `log(-1)`) is a domain error, while a
/// pole that yields ±Inf (`log(0)`) is allowed. A NaN argument passes through.
fn dom_fn(args: &[Value], name: &str, f: impl Fn(f64) -> f64) -> Completion<Value> {
    let x = match one(args, name) {
        Ok(v) => v,
        Err(c) => return c,
    };
    match x.as_double() {
        Ok(d) => {
            let r = f(d);
            if r.is_nan() && !d.is_nan() {
                return domain_err();
            }
            ok(Value::double(r))
        }
        Err(e) => err(e.message),
    }
}

/// A two-`double` libm function with the same domain guard (`atan2`, `hypot`,
/// `fmod` — where `fmod(x, 0)` is the domain error).
fn dom_fn2(args: &[Value], name: &str, f: impl Fn(f64, f64) -> f64) -> Completion<Value> {
    let [lhs, rhs] = args else {
        let which = if args.len() < 2 {
            "not enough arguments"
        } else {
            "too many arguments"
        };
        return err(format!("{which} for math function \"{name}\""));
    };
    match (lhs.as_double(), rhs.as_double()) {
        (Ok(x), Ok(y)) => {
            let r = f(x, y);
            if r.is_nan() && !x.is_nan() && !y.is_nan() {
                return domain_err();
            }
            ok(Value::double(r))
        }
        (Err(e), _) | (_, Err(e)) => err(e.message),
    }
}

fn m_sqrt(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    dom_fn(args, "sqrt", f64::sqrt)
}
fn m_floor(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    dbl_fn(args, "floor", f64::floor)
}
fn m_ceil(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    dbl_fn(args, "ceil", f64::ceil)
}

fn m_pow(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [b, e] = args else {
        let which = if args.len() < 2 {
            "not enough arguments"
        } else {
            "too many arguments"
        };
        return err(format!("{which} for math function \"pow\""));
    };
    match (b.as_double(), e.as_double()) {
        (Ok(bb), Ok(ee)) => {
            let r = bb.powf(ee);
            if r.is_nan() && !bb.is_nan() && !ee.is_nan() {
                return domain_err();
            }
            ok(Value::double(r))
        }
        (Err(er), _) | (_, Err(er)) => err(er.message),
    }
}

/// `entier(x)` — the exact integer value of `x`, truncated toward zero and
/// **unbounded** (TIP 237): an integer of any magnitude passes through; a
/// double becomes the full exact integer (`entier(1e300)` is all 309 digits),
/// with no 64-bit window — that wrap is `int`/`wide`'s ([`int_window`]).
fn m_entier(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "entier") {
        Ok(v) => v,
        Err(c) => return c,
    };
    if let Ok(n) = x.as_int() {
        return ok(Value::int(n));
    }
    if let Some(b) = crate::expr::value_as_bigint(x) {
        return ok(crate::expr::big_value(&b));
    }
    match finite_num_arg(x) {
        Ok(f) => ok(crate::expr::big_value(&exact_trunc(f))),
        Err(m) => err(m),
    }
}

/// `isqrt(n)` — the integer floor of `sqrt(n)` for a non-negative integer (a
/// non-integer argument is truncated first). A negative argument is the error
/// `square root of negative argument` (with the `ARITH DOMAIN` code).
fn m_isqrt(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "isqrt") {
        Ok(v) => v,
        Err(c) => return c,
    };
    let n = if let Ok(n) = x.as_int() {
        n
    } else {
        match num_arg(x) {
            Ok(f) => f.trunc() as i64,
            Err(m) => return err(m),
        }
    };
    if n < 0 {
        return err_with_code("square root of negative argument", DOMAIN_CODE);
    }
    ok(Value::int(isqrt_i64(n)))
}

/// Integer floor-sqrt, correcting the `f64` seed for rounding so the result is
/// exact even near a perfect square (the `i128` products avoid overflow at the
/// top of the `i64` range).
fn isqrt_i64(n: i64) -> i64 {
    if n < 2 {
        return n;
    }
    let nn = i128::from(n);
    let mut r = (n as f64).sqrt() as i64;
    while r > 0 && i128::from(r) * i128::from(r) > nn {
        r -= 1;
    }
    while i128::from(r + 1) * i128::from(r + 1) <= nn {
        r += 1;
    }
    r
}

fn m_bool(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "bool") {
        Ok(v) => v,
        Err(c) => return c,
    };
    match x.as_bool() {
        Ok(b) => ok(Value::bool(b)),
        Err(e) => err(e.message),
    }
}

/// `max`/`min` over their (numeric) arguments. Integer result when all args are
/// integers, else double.
fn min_max(args: &[Value], name: &str, want_max: bool) -> Completion<Value> {
    let Some((first, rest)) = args.split_first() else {
        return err(format!("not enough arguments for math function \"{name}\""));
    };
    // Compare numerically but return the *winning argument* unchanged, so the
    // result keeps the winner's own type: `max(1.5, 2)` → `2` (an integer),
    // not `2.0`. A non-numeric argument reports the integer-flavoured
    // "expected number but got …".
    let mut best = first;
    let mut best_d = match num_arg(first) {
        Ok(d) => d,
        Err(m) => return err(m),
    };
    for a in rest {
        let d = match num_arg(a) {
            Ok(d) => d,
            Err(m) => return err(m),
        };
        if (want_max && d > best_d) || (!want_max && d < best_d) {
            best = a;
            best_d = d;
        }
    }
    ok(best.clone())
}

fn m_max(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    min_max(args, "max", true)
}
fn m_min(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    min_max(args, "min", false)
}

/// `srand(seed)` — reseed the `expr rand()` generator and return its first draw.
/// C (`ExprSrandFunc`) coerces the argument to a wide integer (falling back to
/// truncating a double), installs it as the seed, then tail-calls `rand()`; so
/// `srand` is deterministic and itself yields a number in `[0, 1)`.
fn m_srand(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "srand") {
        Ok(v) => v,
        Err(c) => return c,
    };
    let seed = if let Ok(n) = x.as_wide() {
        n
    } else {
        match num_arg(x) {
            Ok(f) => f.trunc() as i64,
            Err(m) => return err(m),
        }
    };
    vm.rand_seed_set(seed);
    ok(Value::double(vm.rand_next()))
}

/// `rand()` — the next draw from the Park–Miller minimal-standard generator, a
/// `double` in `[0, 1)`. Takes no arguments.
fn m_rand(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    if !args.is_empty() {
        return err("too many arguments for math function \"rand\"");
    }
    ok(Value::double(vm.rand_next()))
}

#[cfg(test)]
mod tests {
    use super::isqrt_i64;

    #[test]
    fn isqrt_small_and_perfect() {
        assert_eq!(isqrt_i64(0), 0);
        assert_eq!(isqrt_i64(1), 1);
        assert_eq!(isqrt_i64(2), 1);
        assert_eq!(isqrt_i64(3), 1);
        assert_eq!(isqrt_i64(4), 2);
        assert_eq!(isqrt_i64(17), 4); // 4*4=16 <= 17 < 25
        assert_eq!(isqrt_i64(24), 4);
        assert_eq!(isqrt_i64(25), 5);
    }

    #[test]
    fn isqrt_large_exact_near_perfect_squares() {
        // Values where an `f64` seed can land just over/under the true root.
        assert_eq!(isqrt_i64(1_000_000_000_000_000_000), 1_000_000_000);
        let big = 3_037_000_499_i64; // floor(sqrt(i64::MAX))
        assert_eq!(isqrt_i64(big * big), big);
        assert_eq!(isqrt_i64(big * big - 1), big - 1);
        assert_eq!(isqrt_i64(i64::MAX), big);
    }
}
