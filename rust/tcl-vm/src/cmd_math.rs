//! `expr` math functions, registered as `tcl::mathfunc::*` (the names the
//! compiler invokes and `ExprEval::call` routes to).
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use tcl_runtime_api::Completion;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("tcl::mathfunc::abs", m_abs);
    vm.register("tcl::mathfunc::int", m_int);
    vm.register("tcl::mathfunc::wide", m_wide);
    vm.register("tcl::mathfunc::double", m_double);
    vm.register("tcl::mathfunc::round", m_round);
    vm.register("tcl::mathfunc::sqrt", m_sqrt);
    vm.register("tcl::mathfunc::floor", m_floor);
    vm.register("tcl::mathfunc::ceil", m_ceil);
    vm.register("tcl::mathfunc::pow", m_pow);
    vm.register("tcl::mathfunc::bool", m_bool);
    vm.register("tcl::mathfunc::max", m_max);
    vm.register("tcl::mathfunc::min", m_min);
}

fn one<'a>(args: &'a [Value], name: &str) -> Result<&'a Value, Completion<Value>> {
    match args {
        [x] => Ok(x),
        _ => Err(err(format!(
            "too many/few args to math function \"{name}\""
        ))),
    }
}

fn m_abs(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "abs") {
        Ok(v) => v,
        Err(c) => return c,
    };
    if let Ok(n) = x.as_int() {
        return ok(Value::int(n.wrapping_abs()));
    }
    match x.as_double() {
        Ok(f) => ok(Value::double(f.abs())),
        Err(e) => err(e.message),
    }
}

fn m_int(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "int") {
        Ok(v) => v,
        Err(c) => return c,
    };
    match x.as_double() {
        Ok(f) => ok(Value::int(f.trunc() as i64)),
        Err(e) => err(e.message),
    }
}

fn m_wide(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // `wide()` truncates to a 64-bit integer — the same width as our `int`.
    let x = match one(args, "wide") {
        Ok(v) => v,
        Err(c) => return c,
    };
    if let Ok(n) = x.as_int() {
        return ok(Value::int(n));
    }
    match x.as_double() {
        Ok(f) => ok(Value::int(f.trunc() as i64)),
        Err(e) => err(e.message),
    }
}

fn m_double(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "double") {
        Ok(v) => v,
        Err(c) => return c,
    };
    match x.as_double() {
        Ok(f) => ok(Value::double(f)),
        Err(e) => err(e.message),
    }
}

fn m_round(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let x = match one(args, "round") {
        Ok(v) => v,
        Err(c) => return c,
    };
    if let Ok(n) = x.as_int() {
        return ok(Value::int(n));
    }
    match x.as_double() {
        Ok(f) => ok(Value::int(f.round() as i64)),
        Err(e) => err(e.message),
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

fn m_sqrt(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    dbl_fn(args, "sqrt", f64::sqrt)
}
fn m_floor(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    dbl_fn(args, "floor", f64::floor)
}
fn m_ceil(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    dbl_fn(args, "ceil", f64::ceil)
}

fn m_pow(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let [b, e] = args else {
        return err("too many/few args to math function \"pow\"");
    };
    match (b.as_double(), e.as_double()) {
        (Ok(bb), Ok(ee)) => ok(Value::double(bb.powf(ee))),
        (Err(er), _) | (_, Err(er)) => err(er.message),
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

/// `max`/`min` over their (numeric) arguments. Integer result when all args are
/// integers, else double.
fn min_max(args: &[Value], name: &str, want_max: bool) -> Completion<Value> {
    if args.is_empty() {
        return err(format!("too few args to math function \"{name}\""));
    }
    let mut all_int = true;
    let mut nums = Vec::with_capacity(args.len());
    for a in args {
        match a.as_double() {
            Ok(d) => {
                if a.as_int().is_err() {
                    all_int = false;
                }
                nums.push(d);
            }
            Err(e) => return err(e.message),
        }
    }
    let best = nums.iter().copied().fold(nums[0], |acc, d| {
        if (want_max && d > acc) || (!want_max && d < acc) {
            d
        } else {
            acc
        }
    });
    if all_int {
        ok(Value::int(best as i64))
    } else {
        ok(Value::double(best))
    }
}

fn m_max(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    min_max(args, "max", true)
}
fn m_min(_vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    min_max(args, "min", false)
}
