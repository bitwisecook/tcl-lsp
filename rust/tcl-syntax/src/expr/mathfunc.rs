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
        | "sinh" | "cosh" | "tanh" | "acosh" | "asinh" | "atanh" | "cbrt" | "erf" | "erfc"
        | "exp2" | "expm1" | "gamma" | "lgamma" | "log1p" | "log2" | "logb" | "trunc" => {
            unary_float(name, args)
        }
        "atan2" | "hypot" | "fmod" | "pow" | "copysign" | "dim" | "nextafter" | "remainder" => {
            binary_float(name, args)
        }
        "abs" | "int" | "entier" | "wide" | "double" | "bool" | "round" | "ceil" | "floor"
        | "isqrt" | "isinf" | "isnan" | "isfinite" | "isnormal" | "issubnormal" | "signbit" => {
            type_conv(name, args)
        }
        "isunordered" => is_unordered(args),
        "ldexp" => ldexp_fn(args),
        "fma" => fma_fn(args),
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
        // TIP 745 (Tcl 9.1) C99 batch: `std` covers these directly.
        "acosh" => f64::acosh,
        "asinh" => f64::asinh,
        "atanh" => f64::atanh,
        "cbrt" => f64::cbrt,
        "exp2" => f64::exp2,
        "expm1" => f64::exp_m1,
        "log1p" => f64::ln_1p,
        "log2" => f64::log2,
        "trunc" => f64::trunc,
        // TIP 745: `std` has no equivalent, so these route through the
        // portable `libm` port (see the crate-level dependency note).
        "erf" => libm::erf,
        "erfc" => libm::erfc,
        "gamma" => libm::tgamma,
        "lgamma" => libm::lgamma,
        "logb" => logb_impl,
        _ => return None,
    };
    let arg = vals[0].as_f64();
    if arg.is_nan() {
        return None;
    }
    let r = f(arg);
    // A NaN result from a non-NaN argument is a domain error (e.g. `sqrt(-1)`,
    // `gamma` at a non-positive integer — `libm::tgamma` already returns NaN
    // there, matching Tcl's own `CheckDoubleResult` domain-error path).
    if r.is_nan() {
        None
    } else {
        Some(Num::Float(r))
    }
}

/// C99 `logb`: the unbiased base-2 exponent as a float. `0.0`/infinities are
/// poles (Tcl's `CheckDoubleResult` accepts the resulting `-Inf`/`+Inf` since
/// they're range, not domain, errors); `libm::ilogb` handles subnormals
/// exactly via the raw exponent bits, unlike a `log2().floor()` estimate.
fn logb_impl(x: f64) -> f64 {
    if x == 0.0 {
        f64::NEG_INFINITY
    } else if x.is_infinite() {
        f64::INFINITY
    } else if x.is_nan() {
        f64::NAN
    } else {
        f64::from(libm::ilogb(x))
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
        // TIP 745: `copysign` is a `std` method; `dim` (C `fdim`),
        // `nextafter`, and `remainder` have no `std` equivalent.
        "copysign" => f64::copysign,
        "dim" => libm::fdim,
        "nextafter" => libm::nextafter,
        "remainder" => libm::remainder,
        _ => return None,
    };
    let r = f(vals[0].as_f64(), vals[1].as_f64());
    if r.is_nan() {
        None
    } else {
        Some(Num::Float(r))
    }
}

/// `ldexp(x, exp)` (TIP 745) — unlike every other binary math function, C's
/// `ldexp` takes a genuine `int` exponent (Tcl's `ExprBinaryDIFunc` reads it
/// with `Tcl_GetIntFromObj`, which rejects a double operand outright), so
/// this doesn't fit the `binary_float` `fn(f64, f64) -> f64` shape.
fn ldexp_fn(vals: &[Num]) -> Option<Num> {
    if vals.len() != 2 {
        return None;
    }
    let Num::Int(exp) = vals[1] else {
        return None;
    };
    let m = vals[0].as_f64();
    if m.is_nan() {
        return None;
    }
    let exp_i32 = i32::try_from(exp).ok()?;
    let r = libm::ldexp(m, exp_i32);
    if r.is_nan() {
        None
    } else {
        Some(Num::Float(r))
    }
}

/// `fma(x, y, z)` (TIP 745) — the one ternary `expr` math function, computed
/// via the portable `libm` fused multiply-add so results stay identical
/// across every backend (native, WASM, eBPF) regardless of hardware FMA
/// support.
fn fma_fn(vals: &[Num]) -> Option<Num> {
    if vals.len() != 3 {
        return None;
    }
    if has_nan(vals) {
        return None;
    }
    let r = libm::fma(vals[0].as_f64(), vals[1].as_f64(), vals[2].as_f64());
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
        // TIP 745: `signbit` reads the sign bit directly (matches C's
        // `signbit()`, which is well-defined on NaN and never a domain
        // error) — an integer operand's sign stands in for the bit test
        // since Tcl's own `ExprSignBitFunc` special-cases each numeric type.
        "signbit" => Some(Num::Int(i64::from(match v {
            Num::Int(i) => i < 0,
            Num::Float(f) => f.is_sign_negative(),
        }))),
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
            Num::Int(i) if i >= 0 => {
                // Float sqrt is only an estimate: a near-2^62 operand rounds
                // up in the i64→f64 conversion and lands the estimate one too
                // high vs C's exact `mp_sqrt` (oracle:
                // `isqrt(4611686018427387903)` is 2147483647, not 2^31).
                // Correct to the exact integer square root.
                let mut r = (i as f64).sqrt() as i64;
                while r > 0 && r.checked_mul(r).is_none_or(|sq| sq > i) {
                    r -= 1;
                }
                while (r + 1).checked_mul(r + 1).is_some_and(|sq| sq <= i) {
                    r += 1;
                }
                Some(Num::Int(r))
            }
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

    /// `added_in()` and `dispatch()` must agree on exactly which names are
    /// implemented — the live drift bug this phase closes (previously
    /// `added_in()` claimed Tcl 9.1 support for 21 functions `dispatch()`
    /// didn't implement at all). For every name `added_in()` recognises
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
            "atan2" | "hypot" | "fmod" | "pow" | "copysign" | "dim" | "nextafter"
            | "isunordered" => {
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
}
