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

use num_bigint::BigInt;
use num_traits::Signed;
use tcl_runtime_api::Completion;
use tcl_syntax::expr::mathfunc::{IntWidth, NumValue, try_dispatch_with_backend_int_width};
use tcl_syntax::number::{self, Number};

use crate::command::err_with_code;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

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

/// The message and `errorCode` C raises (`tclExecute.c`, errno `EDOM`) when a
/// math function's argument is out of range — `sqrt(-1)`, `acos(2)`, `fmod(x,0)`,
/// … . `isqrt` of a negative reuses the same code with its own message.
const DOMAIN_MSG: &str = "domain error: argument not in valid range";
const DOMAIN_CODE: &str = "ARITH DOMAIN {domain error: argument not in valid range}";

fn domain_err() -> Completion<Value> {
    err_with_code(DOMAIN_MSG, DOMAIN_CODE)
}

/// A shared math-function refusal as a VM completion, carrying C's verbatim
/// message and `-errorcode` (#1581): `ARITH IOVERFLOW` for an infinity
/// reaching an integer conversion, `TCL VALUE DOUBLE NAN` for a NaN operand,
/// `ARITH DOMAIN` otherwise. `Abstain` cannot occur here — the VM's backend
/// has an arbitrary-precision rung and its release is resolved — so it falls
/// back to the generic domain error, as do the two refusals the caller words
/// itself.
fn math_func_err(e: tcl_syntax::expr::mathfunc::MathFuncError) -> Completion<Value> {
    let message = e.message();
    if message.is_empty() {
        return domain_err();
    }
    err_with_code(message, e.error_code())
}

/// A [`num_or_nan`] failure as a completion, with C's `-errorcode`: the VM
/// had the right message text for a NaN and an infinity but left `errorCode`
/// as `NONE` (#1581).
fn num_err(message: String) -> Completion<Value> {
    use tcl_syntax::expr::errors;
    if message == errors::NAN_MESSAGE {
        err_with_code(message, errors::NAN_CODE)
    } else if message == errors::IOVERFLOW_MESSAGE {
        err_with_code(message, errors::IOVERFLOW_CODE)
    } else {
        err(message)
    }
}

fn shared_math(name: &str, args: &[Value], int_width: IntWidth) -> Completion<Value> {
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
    match try_dispatch_with_backend_int_width(name, &nums, int_width) {
        Ok(NumValue::Int(i)) => ok(Value::int(i)),
        Ok(NumValue::Big(b)) => ok(crate::expr::big_value(&b)),
        Ok(NumValue::Float(f)) => ok(Value::double(f)),
        Err(e) => math_func_err(e),
    }
}

/// The one builtin behind every `tcl::mathfunc::NAME`.
///
/// The invoked word's tail selects the function, exactly as
/// `runtime/rust`'s `cmd_mathfunc::mathfunc` reads it off `argv[0]` — so a
/// single fn pointer serves all of them and the registration loop below can
/// be driven straight off the shared name table. Both the ordinary command
/// path and the 8.4 fixed-table path
/// ([`Vm::invoke_fixed_math_builtin`](crate::interp::Vm)) set the invoked
/// name before calling, and the fixed-table path sets the *canonical*
/// registry key, so the tail is the function's own spelling either way.
///
/// Most functions fall through to [`shared_math`], which drives
/// `tcl_syntax::expr::mathfunc::dispatch_with_backend` — the same shared
/// implementation `expr` itself uses. The integer conversions
/// (`int`/`wide`/`entier`/`round`/`isqrt`) are shared arms too since
/// #1382/#1795 gave the shared seam an exact float-to-bignum operation
/// (`BigIntOps::from_f64_trunc`), so `int(1e300)` and `round(1e300)` keep
/// their exact answers there.
///
/// The arms named here are the ones whose VM bodies are deliberately *not*
/// the shared ones: `abs` takes an i128 fast path before the bignum rung,
/// `double` is the VM's own value coercion, `bool` accepts a boolean *word*
/// operand (`bool(tru)` is `1`, which `shared_math`'s numeric operand
/// conversion would refuse), and `rand`/`srand` carry interpreter state.
fn m_mathfunc(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let invoked = vm.invoked_name().unwrap_or_default();
    let name = invoked.rsplit("::").next().unwrap_or(invoked).to_owned();
    match name.as_str() {
        "abs" => m_abs(args),
        "double" => m_double(args),
        "bool" => m_bool(args),
        "srand" => m_srand(vm, args),
        "rand" => m_rand(vm, args),
        name => shared_math(name, args, IntWidth::for_tcl_version(vm.runtime_version())),
    }
}

pub(crate) fn register(vm: &mut Vm) {
    // `::tcl::mathfunc` is a real namespace in C Tcl, so a user
    // `proc tcl::mathfunc::square {x} {…}` (TIP 232's custom-function
    // mechanism) must find it existing — declare it alongside the builtin
    // registrations (which only create flat command-table keys).
    vm.declare_namespace("tcl::mathfunc");
    // Derived from `tcl_syntax::expr::mathfunc::all()` rather than a
    // hand-typed list, exactly as `runtime/rust/src/cmd_mathfunc.rs` does
    // (ledger row B3). The hand-typed list this replaces had gone stale by
    // the whole TIP 745 (Tcl 9.1) C99 batch — 21 functions the shared
    // dispatch table already implemented but that were never registered as
    // commands, so `expr {cbrt(27)}` was an error under a 9.1 pin.
    //
    // Every name is registered at every pin; per-release availability is the
    // command surface's job, and `RuntimeExprSurface`'s math-function gate
    // already runs inside `builtin_command_visible_for_surface`, so a
    // 9.1-only function is simply invisible under an 8.6 pin.
    for spec in tcl_syntax::expr::mathfunc::all() {
        vm.register(&format!("tcl::mathfunc::{}", spec.name), m_mathfunc);
    }
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

fn m_abs(args: &[Value]) -> Completion<Value> {
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
        Ok(f) if f.is_nan() => num_err(tcl_syntax::expr::errors::NAN_MESSAGE.to_string()),
        Ok(f) => ok(Value::double(f.abs())),
        Err(m) => num_err(m),
    }
}

fn m_double(args: &[Value]) -> Completion<Value> {
    let x = match one(args, "double") {
        Ok(v) => v,
        Err(c) => return c,
    };
    match x.as_double() {
        Ok(f) if f.is_nan() => num_err(tcl_syntax::expr::errors::NAN_MESSAGE.to_string()),
        Ok(f) => ok(Value::double(f)),
        Err(e) => {
            if let Some(b) = crate::expr::value_as_bigint(x) {
                return ok(Value::double(crate::expr::big_to_f64(&b)));
            }
            if matches!(
                number::parse_whole(x.to_str().trim()),
                Some(Number::Nan { .. })
            ) {
                return num_err(tcl_syntax::expr::errors::NAN_MESSAGE.to_string());
            }
            err(e.message)
        }
    }
}

fn m_bool(args: &[Value]) -> Completion<Value> {
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
    // C reads the operand with `TclGetWideBitsFromObj`: an **integer** of any
    // width, folded to its low 64 bits (`srand(2**64+7)` seeds as `7`). A
    // double is refused, not truncated — tclsh8.6.16 says `expected integer
    // but got "1.5"` (`-errorcode TCL VALUE INTEGER`, or `TCL VALUE NUMBER`
    // for a non-number), and tclsh9.0.4 raises with an *empty* message
    // because C passes a NULL interp to the conversion there. Both engines
    // use 8.6's wording so they agree with each other (#1432); 9.0's
    // empty-message quirk is left to the error-taxonomy work (#1581).
    let Ok(seed) = x.as_wide() else {
        return err_with_code(
            format!("expected integer but got \"{}\"", x.to_str()),
            if x.as_double().is_ok() {
                "TCL VALUE INTEGER"
            } else {
                "TCL VALUE NUMBER"
            },
        );
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
