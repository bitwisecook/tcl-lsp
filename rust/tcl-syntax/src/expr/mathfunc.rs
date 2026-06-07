//! `expr` math functions (`sin`, `max`, `int`, …) — the **single** shared
//! implementation both the compiler's const-folder and the runtime evaluate
//! through (each maps its value type to/from [`Num`]). The semantics follow
//! C Tcl 9.0 (`tclBasic.c`'s `::tcl::mathfunc::*`); `rand`/`srand` are
//! non-deterministic and handled by the caller (not here).
//!
//! These functions are *also* overridable commands in `::tcl::mathfunc::*`; once
//! the runtime has namespaces, a user-defined `::tcl::mathfunc::foo` is resolved
//! through the command table first and only then does the caller fall back to
//! this built-in dispatch — see the command-binding contract (A3).

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]

/// A transient numeric value for math-function dispatch — `i64` or `f64` (the
/// double rung is where the float functions compute). Each consumer converts its
/// own value (`TclValue` / a `Tcl_Obj` read off the tower) to/from this.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Num {
    /// Integer.
    Int(i64),
    /// IEEE-754 double.
    Float(f64),
}

impl Num {
    /// As an `f64` (widening an integer).
    #[must_use]
    pub fn as_f64(self) -> f64 {
        match self {
            Num::Int(i) => i as f64,
            Num::Float(f) => f,
        }
    }
    fn is_truthy(self) -> bool {
        match self {
            Num::Int(i) => i != 0,
            Num::Float(f) => f != 0.0,
        }
    }
}

/// Dispatch a math function by (lowercased) `name` over already-evaluated
/// numeric `args`. Returns `None` for an unknown function, a wrong argument
/// count, or a domain error (matching the const-folder "give up" / runtime
/// "fall through" contract). `rand`/`srand` are the caller's responsibility.
#[must_use]
pub fn dispatch(name: &str, args: &[Num]) -> Option<Num> {
    match name {
        "min" | "max" => min_max(name, args),
        "sqrt" | "exp" | "log" | "log10" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan"
        | "sinh" | "cosh" | "tanh" => unary_float(name, args),
        "atan2" | "hypot" | "fmod" | "pow" => binary_float(name, args),
        "abs" | "int" | "entier" | "wide" | "double" | "bool" | "round" | "ceil" | "floor"
        | "isqrt" | "isinf" | "isnan" | "isfinite" => type_conv(name, args),
        _ => None,
    }
}

// `i64::MAX as f64` rounds up to 2^63, so the positive bound is
// exclusive. The negative 2^63 value is exactly representable and fits.
const I64_MIN_F64: f64 = -9_223_372_036_854_775_808.0;
const I64_MAX_PLUS_ONE_F64: f64 = 9_223_372_036_854_775_808.0;

fn finite_trunc_to_i64(f: f64) -> Option<i64> {
    if !f.is_finite() {
        return None;
    }
    let truncated = f.trunc();
    if !(I64_MIN_F64..I64_MAX_PLUS_ONE_F64).contains(&truncated) {
        return None;
    }
    Some(truncated as i64)
}

fn finite_round_to_i64(f: f64) -> Option<i64> {
    if !f.is_finite() {
        return None;
    }
    let rounded = if f >= 0.0 {
        (f + 0.5).floor()
    } else {
        (f - 0.5).ceil()
    };
    finite_trunc_to_i64(rounded)
}

fn has_nan(vals: &[Num]) -> bool {
    vals.iter()
        .any(|v| matches!(v, Num::Float(f) if f.is_nan()))
}

fn min_max(name: &str, vals: &[Num]) -> Option<Num> {
    if vals.is_empty() || has_nan(vals) {
        return None;
    }
    if vals.iter().all(|v| matches!(v, Num::Int(_))) {
        let ints = vals.iter().map(|v| match v {
            Num::Int(i) => *i,
            Num::Float(_) => unreachable!(),
        });
        let r = if name == "min" {
            ints.min().unwrap()
        } else {
            ints.max().unwrap()
        };
        Some(Num::Int(r))
    } else {
        let mut best = vals[0].as_f64();
        for v in &vals[1..] {
            let f = v.as_f64();
            if (name == "min" && f < best) || (name == "max" && f > best) {
                best = f;
            }
        }
        Some(Num::Float(best))
    }
}

fn unary_float(name: &str, vals: &[Num]) -> Option<Num> {
    if vals.len() != 1 {
        return None;
    }
    let f: fn(f64) -> f64 = match name {
        "sqrt" => f64::sqrt,
        "exp" => f64::exp,
        "log" => f64::ln,
        "log10" => f64::log10,
        "sin" => f64::sin,
        "cos" => f64::cos,
        "tan" => f64::tan,
        "asin" => f64::asin,
        "acos" => f64::acos,
        "atan" => f64::atan,
        "sinh" => f64::sinh,
        "cosh" => f64::cosh,
        "tanh" => f64::tanh,
        _ => return None,
    };
    let arg = vals[0].as_f64();
    if arg.is_nan() {
        return None;
    }
    let r = f(arg);
    // A NaN result from a non-NaN argument is a domain error (e.g. `sqrt(-1)`).
    if r.is_nan() {
        None
    } else {
        Some(Num::Float(r))
    }
}

fn binary_float(name: &str, vals: &[Num]) -> Option<Num> {
    if vals.len() != 2 {
        return None;
    }
    if has_nan(vals) {
        return None;
    }
    let f: fn(f64, f64) -> f64 = match name {
        "atan2" => f64::atan2,
        "hypot" => f64::hypot,
        "fmod" => |a, b| a % b,
        "pow" => f64::powf,
        _ => return None,
    };
    let r = f(vals[0].as_f64(), vals[1].as_f64());
    if r.is_nan() {
        None
    } else {
        Some(Num::Float(r))
    }
}

fn type_conv(name: &str, vals: &[Num]) -> Option<Num> {
    if vals.len() != 1 {
        return None;
    }
    let v = vals[0];
    match name {
        "isinf" => Some(Num::Int(i64::from(
            matches!(v, Num::Float(f) if f.is_infinite()),
        ))),
        "isnan" => Some(Num::Int(i64::from(
            matches!(v, Num::Float(f) if f.is_nan()),
        ))),
        "isfinite" => match v {
            Num::Int(_) => Some(Num::Int(1)),
            Num::Float(f) => Some(Num::Int(i64::from(f.is_finite()))),
        },
        _ if matches!(v, Num::Float(f) if f.is_nan()) => None,
        "abs" => match v {
            Num::Int(i) => i.checked_abs().map(Num::Int),
            Num::Float(f) => Some(Num::Float(f.abs())),
        },
        "int" | "entier" | "wide" => match v {
            Num::Int(i) => Some(Num::Int(i)),
            Num::Float(f) => finite_trunc_to_i64(f).map(Num::Int),
        },
        "double" => Some(Num::Float(v.as_f64())),
        "bool" => Some(Num::Int(i64::from(v.is_truthy()))),
        "round" => match v {
            Num::Int(i) => Some(Num::Int(i)),
            Num::Float(f) => finite_round_to_i64(f).map(Num::Int),
        },
        "ceil" => Some(Num::Float(v.as_f64().ceil())),
        "floor" => Some(Num::Float(v.as_f64().floor())),
        "isqrt" => match v {
            Num::Int(i) if i >= 0 => Some(Num::Int((i as f64).sqrt() as i64)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn unknown_and_arity() {
        assert_eq!(dispatch("frobnicate", &[Num::Int(1)]), None);
        assert_eq!(dispatch("sqrt", &[Num::Int(1), Num::Int(2)]), None); // wrong arity
    }
}
