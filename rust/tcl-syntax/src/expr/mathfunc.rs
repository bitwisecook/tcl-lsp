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
        | "isqrt" | "isinf" | "isnan" | "isfinite" | "isnormal" | "issubnormal" => {
            type_conv(name, args)
        }
        "isunordered" => is_unordered(args),
        _ => None,
    }
}

/// Whether math function `name`'s operand accepts Tcl boolean words
/// (`true`/`yes`/`on`/`false`/…) in addition to the numeric grammar.
///
/// Only `bool` does — it calls `Tcl_GetBooleanFromObj`. Every other expr
/// function reads its operand with `Tcl_GetDoubleFromObj` /
/// `Tcl_GetWideIntFromObj`, which reject boolean words (`expr {abs(true)}`
/// is an error, not `1`). A const-folder must therefore parse the operands
/// of every function except `bool` *strictly* — without the boolean coercion
/// `Tcl_GetBoolean` would apply — so it never folds an error into a value.
#[must_use]
pub fn accepts_boolean_operand(name: &str) -> bool {
    name == "bool"
}

/// The Tcl core release an `expr` math function first appeared in.
///
/// `expr` functions gate by the *expr grammar* base version — the same axis
/// the relational operators (`in`/`lt`/…) do — so a vendor shell running on an
/// 8.5 core has the 8.5 set even though its dialect tag isn't a plain Tcl
/// version.  The variants are ordered oldest-first, so a caller checks
/// availability with `added_in(name) <= base_version`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MathFuncSince {
    /// The fixed 8.4 C function table (`tclExecute.c`).
    Tcl84,
    /// TIP 232 introduced the `::tcl::mathfunc` command scheme and added
    /// `bool` / `entier` / `isqrt` / `min` / `max`.
    Tcl85,
    /// TIP 521 added the floating-point classification functions
    /// (`isinf` / `isnan` / `isnormal` / `issubnormal` / `isfinite` /
    /// `isunordered`).
    Tcl90,
    /// TIP 745 added the C99 batch (`acosh` / `cbrt` / `fma` / `log2` / …).
    Tcl91,
}

/// The release a math function named `name` first became available in, or
/// `None` when `name` is not a built-in `expr` function in any release.
///
/// This is the single source of truth for *which* names are `expr` functions
/// and *when* each appeared, shared by the const-folder, the runtime, and the
/// dialect-availability diagnostic.  Names are matched verbatim (mathfunc
/// lookup is case-sensitive).
#[must_use]
pub fn added_in(name: &str) -> Option<MathFuncSince> {
    let since = match name {
        // The 8.4 fixed C table (`wide` landed in 8.4.0).
        "abs" | "acos" | "asin" | "atan" | "atan2" | "ceil" | "cos" | "cosh" | "double" | "exp"
        | "floor" | "fmod" | "hypot" | "int" | "log" | "log10" | "pow" | "rand" | "round"
        | "sin" | "sinh" | "sqrt" | "srand" | "tan" | "tanh" | "wide" => MathFuncSince::Tcl84,
        // TIP 232 (8.5).
        "bool" | "entier" | "isqrt" | "max" | "min" => MathFuncSince::Tcl85,
        // TIP 521 (9.0).
        "isfinite" | "isinf" | "isnan" | "isnormal" | "issubnormal" | "isunordered" => {
            MathFuncSince::Tcl90
        }
        // TIP 745 (9.1) C99 batch; the multi-value C99 functions land as the
        // `divmod` / `frexp` / `modf` / `remquo` *commands* instead.
        "acosh" | "asinh" | "atanh" | "cbrt" | "copysign" | "dim" | "erf" | "erfc" | "exp2"
        | "expm1" | "fma" | "gamma" | "ldexp" | "lgamma" | "log1p" | "log2" | "logb"
        | "nextafter" | "remainder" | "signbit" | "trunc" => MathFuncSince::Tcl91,
        _ => return None,
    };
    Some(since)
}

/// `isunordered(x, y)` — 1 if either operand is NaN (they cannot be ordered),
/// else 0 (C's `ExprIsUnorderedFunc`). Integers convert to finite doubles.
fn is_unordered(vals: &[Num]) -> Option<Num> {
    if vals.len() != 2 {
        return None;
    }
    let nan = |v: Num| matches!(v, Num::Float(f) if f.is_nan());
    Some(Num::Int(i64::from(nan(vals[0]) || nan(vals[1]))))
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
        // `fpclassify`-based predicates (C's `DoubleObjIsClass`): an integer
        // operand converts to a finite double first.
        "isnormal" => Some(Num::Int(i64::from(
            v.as_f64().classify() == core::num::FpCategory::Normal,
        ))),
        "issubnormal" => Some(Num::Int(i64::from(
            v.as_f64().classify() == core::num::FpCategory::Subnormal,
        ))),
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
}
