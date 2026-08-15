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

use num_bigint::BigInt;
use num_traits::{FromPrimitive, Signed, ToPrimitive};
use tcl_runtime_api::Completion;
use tcl_syntax::expr::mathfunc::{NumValue, dispatch_with_backend};
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

fn exact_trunc(f: f64) -> BigInt {
    BigInt::from_f64(f.trunc()).unwrap_or_default()
}

fn wide_window(b: &BigInt) -> i64 {
    let low = b & &BigInt::from(u64::MAX);
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

fn shared_math(name: &str, args: &[Value]) -> Completion<Value> {
    let Some(spec) = tcl_syntax::expr::mathfunc::spec(name) else {
        return err(format!("invalid command name \"tcl::mathfunc::{name}\""));
    };
    let n = args.len();
    let min = usize::from(spec.arity.min);
    if n < min || spec.arity.max.is_some_and(|m| n > usize::from(m)) {
        return err(format!(
            "{} for math function \"{name}\"",
            if n < min {
                "not enough arguments"
            } else {
                "too many arguments"
            }
        ));
    }
    let nums: Result<Vec<NumValue<BigInt>>, Completion<Value>> = args
        .iter()
        .map(|v| {
            if let Ok(i) = v.as_int() {
                return Ok(NumValue::Int(i));
            }
            if let Some(b) = crate::expr::value_as_bigint(v) {
                return Ok(NumValue::Big(b));
            }
            if let Ok(f) = v.as_double() {
                return Ok(NumValue::Float(f));
            }
            if matches!(
                number::parse_whole(v.to_str().trim()),
                Some(Number::Nan { .. })
            ) {
                return Ok(NumValue::Float(f64::NAN));
            }
            let message = if tcl_syntax::expr::mathfunc::expects_floating_operand_error(name) {
                format!("expected floating-point number but got \"{}\"", v.to_str())
            } else {
                format!("expected number but got \"{}\"", v.to_str())
            };
            Err(err(message))
        })
        .collect();
    let nums = match nums {
        Ok(nums) => nums,
        Err(e) => return e,
    };
    match dispatch_with_backend(name, &nums) {
        Some(NumValue::Int(i)) => ok(Value::int(i)),
        Some(NumValue::Big(b)) => ok(crate::expr::big_value(&b)),
        Some(NumValue::Float(f)) => ok(Value::double(f)),
        None => domain_err(),
    }
}

macro_rules! shared_wrapper {
    ($fn_name:ident, $math_name:literal) => {
        fn $fn_name(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
            shared_math($math_name, args)
        }
    };
}

shared_wrapper!(shared_sqrt, "sqrt");
shared_wrapper!(shared_max, "max");
shared_wrapper!(shared_min, "min");
shared_wrapper!(shared_floor, "floor");
shared_wrapper!(shared_ceil, "ceil");
shared_wrapper!(shared_pow, "pow");
shared_wrapper!(shared_sin, "sin");
shared_wrapper!(shared_cos, "cos");
shared_wrapper!(shared_tan, "tan");
shared_wrapper!(shared_asin, "asin");
shared_wrapper!(shared_acos, "acos");
shared_wrapper!(shared_atan, "atan");
shared_wrapper!(shared_sinh, "sinh");
shared_wrapper!(shared_cosh, "cosh");
shared_wrapper!(shared_tanh, "tanh");
shared_wrapper!(shared_exp, "exp");
shared_wrapper!(shared_log, "log");
shared_wrapper!(shared_log10, "log10");
shared_wrapper!(shared_atan2, "atan2");
shared_wrapper!(shared_hypot, "hypot");
shared_wrapper!(shared_fmod, "fmod");
shared_wrapper!(shared_isfinite, "isfinite");
shared_wrapper!(shared_isinf, "isinf");
shared_wrapper!(shared_isnan, "isnan");
shared_wrapper!(shared_isnormal, "isnormal");
shared_wrapper!(shared_issubnormal, "issubnormal");
shared_wrapper!(shared_isunordered, "isunordered");

pub(crate) fn register(vm: &mut Vm) {
    // `::tcl::mathfunc` is a real namespace in C Tcl, so a user
    // `proc tcl::mathfunc::square {x} {…}` (TIP 232's custom-function
    // mechanism) must find it existing — declare it alongside the builtin
    // registrations (which only create flat command-table keys).
    vm.declare_namespace("tcl::mathfunc");
    vm.register("tcl::mathfunc::abs", m_abs);
    // These conversions retain the VM's exact finite-double-to-bignum path;
    // the shared value seam deliberately has no float-to-arbitrary-integer
    // operation, so routing them through it would turn `int(1e300)` and
    // `round(1e300)` into a spurious domain error.
    vm.register("tcl::mathfunc::int", m_int);
    vm.register("tcl::mathfunc::wide", m_wide);
    vm.register("tcl::mathfunc::entier", m_entier);
    vm.register("tcl::mathfunc::double", m_double);
    vm.register("tcl::mathfunc::round", m_round);
    vm.register("tcl::mathfunc::sqrt", shared_sqrt);
    vm.register("tcl::mathfunc::isqrt", m_isqrt);
    vm.register("tcl::mathfunc::floor", shared_floor);
    vm.register("tcl::mathfunc::ceil", shared_ceil);
    vm.register("tcl::mathfunc::pow", shared_pow);
    vm.register("tcl::mathfunc::bool", m_bool);
    vm.register("tcl::mathfunc::max", shared_max);
    vm.register("tcl::mathfunc::min", shared_min);
    vm.register("tcl::mathfunc::srand", m_srand);
    vm.register("tcl::mathfunc::rand", m_rand);
    // Trigonometric / transcendental — single `double` argument.
    vm.register("tcl::mathfunc::sin", shared_sin);
    vm.register("tcl::mathfunc::cos", shared_cos);
    vm.register("tcl::mathfunc::tan", shared_tan);
    vm.register("tcl::mathfunc::asin", shared_asin);
    vm.register("tcl::mathfunc::acos", shared_acos);
    vm.register("tcl::mathfunc::atan", shared_atan);
    vm.register("tcl::mathfunc::sinh", shared_sinh);
    vm.register("tcl::mathfunc::cosh", shared_cosh);
    vm.register("tcl::mathfunc::tanh", shared_tanh);
    vm.register("tcl::mathfunc::exp", shared_exp);
    vm.register("tcl::mathfunc::log", shared_log);
    vm.register("tcl::mathfunc::log10", shared_log10);
    // Two `double` arguments.
    vm.register("tcl::mathfunc::atan2", shared_atan2);
    vm.register("tcl::mathfunc::hypot", shared_hypot);
    vm.register("tcl::mathfunc::fmod", shared_fmod);
    // Classification predicates — one `double`, returns a boolean (never a
    // domain error, since inspecting a NaN/Inf is their purpose).
    vm.register("tcl::mathfunc::isfinite", shared_isfinite);
    vm.register("tcl::mathfunc::isinf", shared_isinf);
    vm.register("tcl::mathfunc::isnan", shared_isnan);
    vm.register("tcl::mathfunc::isnormal", shared_isnormal);
    vm.register("tcl::mathfunc::issubnormal", shared_issubnormal);
    vm.register("tcl::mathfunc::isunordered", shared_isunordered);
    // `fpclassify` is a top-level command, not a math function.
    vm.register("fpclassify", cmd_fpclassify);
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
    if let Ok(n) = x.as_int() {
        return ok(crate::expr::int_value(i128::from(n).abs()));
    }
    if let Some(b) = x.as_i128()
        && b != i128::MIN
    {
        return ok(crate::expr::int_value(b.abs()));
    }
    if let Some(b) = crate::expr::value_as_bigint(x) {
        return ok(crate::expr::big_value(&b.abs()));
    }
    match num_or_nan(x) {
        Ok(f) if f.is_nan() => err("floating point value is Not a Number".to_string()),
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
/// (`int(9007199254740993)` must stay `…993`).
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
        Ok(f) if f.is_nan() => err("floating point value is Not a Number"),
        Ok(f) => ok(Value::double(f)),
        Err(e) => {
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

/// VM-facing `isqrt` keeps Tcl's specialised negative-argument diagnostic;
/// the exact positive bignum root is the same shared tower operation used by
/// the other backends.
fn m_isqrt(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "isqrt") {
        Ok(v) => v,
        Err(c) => return c,
    };
    if let Ok(n) = x.as_int() {
        if n < 0 {
            return err_with_code("square root of negative argument", DOMAIN_CODE);
        }
        return ok(Value::int(isqrt_i64(n)));
    }
    if let Some(b) = crate::expr::value_as_bigint(x) {
        if b.is_negative() {
            return err_with_code("square root of negative argument", DOMAIN_CODE);
        }
        return ok(crate::expr::big_value(&num_integer::Roots::sqrt(&b)));
    }
    let f = match num_arg(x) {
        Ok(f) => f,
        Err(m) => return err(m),
    };
    if f < 0.0 {
        return err_with_code("square root of negative argument", DOMAIN_CODE);
    }
    ok(Value::int(isqrt_i64(f.trunc() as i64)))
}

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
