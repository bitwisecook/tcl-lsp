//! Math-category builtins.
//!
//! Covers the trig / hyperbolic wrappers, the advanced C-math family
//! (`sqrt`, `pow`, `log*`, `exp*`, gamma, …), the IEEE helpers
//! (`copysign`, `frexp`, `modf`, `remainder`, …) and the Bessel functions.
//!
//! `floor`, `ceil`, and `abs` live in the parent module and are NOT
//! re-registered here.
//!
//! Float results render with a trailing `.0` and integers without, so each
//! builtin tracks the int-vs-float result type precisely. For byte-identical
//! results we mostly use Rust's `std::f64` methods (which bind to the same
//! system libm `math` calls), fall back to the pure-Rust `libm`
//! crate for the few C functions `std` lacks (`expm1`, `log1p`, `ldexp`,
//! `remainder`, `fmod`, `frexp`, `modf`), and implement two families exactly
//! so they stay byte-identical: the Bessel functions (an Abramowitz & Stegun
//! polynomial approximation) and gamma / lgamma (the Lanczos
//! approximation for gamma / lgamma — these do not use the platform libm).

// This module reproduces the C `math` numerics bit-for-bit, which means
// transcribing the exact float literals and float-equality checks:
// - `approx_constant` / `unreadable_literal` / `excessive_precision`: the
//   Bessel polynomials use truncated constants (e.g. `0.636619772` ≈ 2/π and
//   long coefficient literals) copied verbatim; "tidying" them
//   would change the result bits.
// - `float_cmp`: integral / tie / zero tests (`x == x.floor()`, `… == 0.5`,
//   `x == 0.0`) are exact by design — they belong to the Lanczos gamma /
//   lgamma approximation and the round-half-even path.
#![allow(
    clippy::approx_constant,
    clippy::unreadable_literal,
    clippy::excessive_precision,
    clippy::float_cmp
)]

use crate::builtins::{BuiltinSpec, as_int, as_number, plain};
use crate::errors::QueryError;
use crate::jsonfmt;
use crate::value::Value;

// The `math` surface mostly delegates to the platform C library (glibc
// on the reference host). Rust's `std::f64` transcendental methods bind to
// the same system libm, so they reproduce the reference results bit-for-bit —
// whereas the pure-Rust `libm` crate diverges by ~1 ULP on some functions
// (`exp`, `cosh`, …). We therefore prefer `std` methods, and use the `libm`
// crate only for the C functions `std` lacks where its result already agrees
// with glibc bit-for-bit on the relevant domain: `expm1`, `log1p`, `ldexp`,
// `remainder`, `fmod`, plus the exact bit-manipulation `frexp` / `modf`. The
// `gamma`/`lgamma` family is NOT libm-backed: it uses the Lanczos
// approximation reproduced below so results match bit-for-bit.
// (`unsafe_code = "forbid"` rules out binding the system libm via
// `extern "C"`, which is why these choices matter for byte parity.)

pub(super) fn registrations() -> Vec<(&'static str, BuiltinSpec)> {
    vec![
        // Rounding / sign.
        plain("round", "math", 1, Some(1), false, bi_round),
        plain("fabs", "math", 1, Some(1), false, bi_fabs),
        plain("trunc", "math", 1, Some(1), false, bi_trunc),
        plain("rint", "math", 1, Some(1), false, bi_rint),
        plain("nearbyint", "math", 1, Some(1), false, bi_nearbyint),
        // Powers / roots / logs.
        plain("sqrt", "math", 1, Some(1), false, bi_sqrt),
        plain("cbrt", "math", 1, Some(1), false, bi_cbrt),
        plain("pow", "math", 2, Some(2), false, bi_pow),
        plain("exp", "math", 1, Some(1), false, bi_exp),
        plain("exp2", "math", 1, Some(1), false, bi_exp2),
        plain("exp10", "math", 1, Some(1), false, bi_exp10),
        plain("pow10", "math", 1, Some(1), false, bi_exp10),
        plain("expm1", "math", 1, Some(1), false, bi_expm1),
        plain("log", "math", 1, Some(1), false, bi_log),
        plain("log10", "math", 1, Some(1), false, bi_log10),
        plain("log2", "math", 1, Some(1), false, bi_log2),
        plain("log1p", "math", 1, Some(1), false, bi_log1p),
        plain("logb", "math", 1, Some(1), false, bi_logb),
        plain("significand", "math", 1, Some(1), false, bi_significand),
        // Constants / classification.
        plain("nan", "math", 0, Some(0), false, bi_nan),
        plain("infinite", "math", 0, Some(0), false, bi_infinite),
        plain("isnan", "math", 1, Some(1), false, bi_isnan),
        plain("isinfinite", "math", 1, Some(1), false, bi_isinfinite),
        plain("isnormal", "math", 1, Some(1), false, bi_isnormal),
        // Trig / hyperbolic (thin wrappers, domain errors propagate).
        plain("sin", "math", 1, Some(1), false, bi_sin),
        plain("cos", "math", 1, Some(1), false, bi_cos),
        plain("tan", "math", 1, Some(1), false, bi_tan),
        plain("asin", "math", 1, Some(1), false, bi_asin),
        plain("acos", "math", 1, Some(1), false, bi_acos),
        plain("atan", "math", 1, Some(1), false, bi_atan),
        plain("atan2", "math", 2, Some(2), false, bi_atan2),
        plain("sinh", "math", 1, Some(1), false, bi_sinh),
        plain("cosh", "math", 1, Some(1), false, bi_cosh),
        plain("tanh", "math", 1, Some(1), false, bi_tanh),
        plain("asinh", "math", 1, Some(1), false, bi_asinh),
        plain("acosh", "math", 1, Some(1), false, bi_acosh),
        plain("atanh", "math", 1, Some(1), false, bi_atanh),
        // IEEE helpers.
        plain("copysign", "math", 2, Some(2), false, bi_copysign),
        plain("fdim", "math", 2, Some(2), false, bi_fdim),
        plain("frexp", "math", 1, Some(1), false, bi_frexp),
        plain("ldexp", "math", 2, Some(2), false, bi_ldexp),
        plain("modf", "math", 1, Some(1), false, bi_modf),
        plain("hypot", "math", 2, Some(2), false, bi_hypot),
        plain("fma", "math", 3, Some(3), false, bi_fma),
        plain("fmax", "math", 2, Some(2), false, bi_fmax),
        plain("fmin", "math", 2, Some(2), false, bi_fmin),
        plain("fmod", "math", 2, Some(2), false, bi_fmod),
        plain("remainder", "math", 2, Some(2), false, bi_remainder),
        plain("drem", "math", 2, Some(2), false, bi_remainder),
        // Gamma family.
        plain("gamma", "math", 1, Some(1), false, bi_gamma),
        plain("tgamma", "math", 1, Some(1), false, bi_gamma),
        plain("lgamma", "math", 1, Some(1), false, bi_lgamma),
        plain("lgamma_r", "math", 1, Some(1), false, bi_lgamma_r),
        // Bessel.
        plain("j0", "math", 1, Some(1), false, bi_j0),
        plain("j1", "math", 1, Some(1), false, bi_j1),
        plain("y0", "math", 1, Some(1), false, bi_y0),
        plain("y1", "math", 1, Some(1), false, bi_y1),
        plain("jn", "math", 2, Some(2), false, bi_jn),
        plain("yn", "math", 2, Some(2), false, bi_yn),
    ]
}

// Helpers

/// Coerce an `as_number` result to `f64`.
fn num_f64(v: &Value) -> f64 {
    match v {
        Value::Int(i) => *i as f64,
        Value::Float(f) => *f,
        _ => unreachable!("as_number returns Int or Float"),
    }
}

/// Repr-style rendering of an int-or-float number value, used inside
/// domain-error messages that interpolate `{n!r}`.
fn num_repr(v: &Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Float(f) => jsonfmt::py_float_repr(*f),
        _ => unreachable!("as_number returns Int or Float"),
    }
}

/// `str(n)` of a number value, used by the Bessel domain
/// errors which interpolate `{val}` rather than `{val!r}`.
fn num_str(v: &Value) -> String {
    // For ints and floats `str` and `repr` agree on the shapes we hit here.
    num_repr(v)
}

// Rounding / sign

fn bi_round(args: &[Value]) -> Result<Value, QueryError> {
    // C `round` semantics (ties away from zero) — jq parity, not banker's.
    let n = num_f64(&as_number(&args[0], "round", 1)?);
    let r = if n >= 0.0 {
        (n + 0.5).floor()
    } else {
        -((-n + 0.5).floor())
    };
    Ok(Value::Int(r as i64))
}

fn bi_fabs(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Float(
        num_f64(&as_number(&args[0], "fabs", 1)?).abs(),
    ))
}

fn bi_trunc(args: &[Value]) -> Result<Value, QueryError> {
    // `trunc` returns an int.
    Ok(Value::Int(
        num_f64(&as_number(&args[0], "trunc", 1)?).trunc() as i64,
    ))
}

fn bi_rint(args: &[Value]) -> Result<Value, QueryError> {
    // `int(round(x))` — banker's rounding, then int.
    Ok(Value::Int(
        py_round_half_even(num_f64(&as_number(&args[0], "rint", 1)?)) as i64,
    ))
}

fn bi_nearbyint(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Int(
        py_round_half_even(num_f64(&as_number(&args[0], "nearbyint", 1)?)) as i64,
    ))
}

/// Round `x` to the nearest int — ties to even.
fn py_round_half_even(x: f64) -> f64 {
    let r = x.round(); // ties away from zero
    if (x - x.trunc()).abs() == 0.5 {
        // Exact tie: round to even.
        let lower = x.floor();
        if (lower as i64) % 2 == 0 {
            lower
        } else {
            x.ceil()
        }
    } else {
        r
    }
}

// Powers / roots / logs

fn bi_sqrt(args: &[Value]) -> Result<Value, QueryError> {
    let v = as_number(&args[0], "sqrt", 1)?;
    let n = num_f64(&v);
    if n < 0.0 {
        return Err(QueryError::builtin(format!(
            "sqrt: negative input {}",
            num_repr(&v)
        )));
    }
    Ok(Value::Float(n.sqrt()))
}

fn bi_cbrt(args: &[Value]) -> Result<Value, QueryError> {
    // Uses `n ** (1/3)` (not C cbrt), negating for negative inputs.
    let n = num_f64(&as_number(&args[0], "cbrt", 1)?);
    let r = if n < 0.0 {
        -(-n).powf(1.0 / 3.0)
    } else {
        n.powf(1.0 / 3.0)
    };
    Ok(Value::Float(r))
}

fn bi_pow(args: &[Value]) -> Result<Value, QueryError> {
    let b = num_f64(&as_number(&args[0], "pow", 1)?);
    let e = num_f64(&as_number(&args[1], "pow", 2)?);
    Ok(Value::Float(b.powf(e)))
}

fn bi_exp(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Float(num_f64(&as_number(&args[0], "exp", 1)?).exp()))
}

fn bi_exp2(args: &[Value]) -> Result<Value, QueryError> {
    // Computes `pow(2.0, value)`.
    Ok(Value::Float(
        2.0f64.powf(num_f64(&as_number(&args[0], "exp2", 1)?)),
    ))
}

fn bi_exp10(args: &[Value]) -> Result<Value, QueryError> {
    // Computes `pow(10.0, value)` — shared by `exp10` and `pow10`.
    Ok(Value::Float(
        10.0f64.powf(num_f64(&as_number(&args[0], "exp10", 1)?)),
    ))
}

fn bi_expm1(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Float(libm::expm1(num_f64(&as_number(
        &args[0], "expm1", 1,
    )?))))
}

fn bi_log(args: &[Value]) -> Result<Value, QueryError> {
    let v = as_number(&args[0], "log", 1)?;
    let n = num_f64(&v);
    if n <= 0.0 {
        return Err(QueryError::builtin(format!(
            "log: non-positive input {}",
            num_repr(&v)
        )));
    }
    Ok(Value::Float(n.ln()))
}

fn bi_log10(args: &[Value]) -> Result<Value, QueryError> {
    let v = as_number(&args[0], "log10", 1)?;
    let n = num_f64(&v);
    if n <= 0.0 {
        return Err(QueryError::builtin(format!(
            "log10: non-positive input {}",
            num_repr(&v)
        )));
    }
    Ok(Value::Float(n.log10()))
}

fn bi_log2(args: &[Value]) -> Result<Value, QueryError> {
    let v = as_number(&args[0], "log2", 1)?;
    let n = num_f64(&v);
    if n <= 0.0 {
        return Err(QueryError::builtin(format!(
            "log2: non-positive input {}",
            num_repr(&v)
        )));
    }
    Ok(Value::Float(n.log2()))
}

fn bi_log1p(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Float(libm::log1p(num_f64(&as_number(
        &args[0], "log1p", 1,
    )?))))
}

fn bi_logb(args: &[Value]) -> Result<Value, QueryError> {
    // -inf for 0, +inf for inf, else float(floor(log2(|n|))).
    let n = num_f64(&as_number(&args[0], "logb", 1)?);
    if n == 0.0 {
        return Ok(Value::Float(f64::NEG_INFINITY));
    }
    if n.is_infinite() {
        return Ok(Value::Float(f64::INFINITY));
    }
    Ok(Value::Float(n.abs().log2().floor()))
}

fn bi_significand(args: &[Value]) -> Result<Value, QueryError> {
    // Returns the input unchanged for zero, infinities, and NaN.
    let n = num_f64(&as_number(&args[0], "significand", 1)?);
    if n == 0.0 || n.is_infinite() || n.is_nan() {
        return Ok(Value::Float(n));
    }
    let e = n.abs().log2().floor();
    Ok(Value::Float(n / 2.0f64.powf(e)))
}

// Constants / classification

fn bi_nan(_args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Float(f64::NAN))
}

fn bi_infinite(_args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Float(f64::INFINITY))
}

fn bi_isnan(args: &[Value]) -> Result<Value, QueryError> {
    // `false` for non-number / bool / int input; only floats can be NaN.
    Ok(Value::Bool(
        matches!(&args[0], Value::Float(f) if f.is_nan()),
    ))
}

fn bi_isinfinite(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Bool(
        matches!(&args[0], Value::Float(f) if f.is_infinite()),
    ))
}

fn bi_isnormal(args: &[Value]) -> Result<Value, QueryError> {
    let result = match &args[0] {
        Value::Bool(_) => false,
        Value::Int(i) => *i != 0,
        Value::Float(f) => {
            // Finite, non-zero, non-subnormal: |f| >= smallest normal.
            f.is_finite() && *f != 0.0 && f.abs() >= 2.2250738585072014e-308
        }
        _ => false,
    };
    Ok(Value::Bool(result))
}

// Trig / hyperbolic (thin wrappers — domain errors surface "math domain error")

/// Domain-checking wrapper for the `math` builtins: when the result is NaN
/// but the input was finite, this is a `ValueError: math domain error`.
fn domain_checked(input: f64, result: f64) -> Result<Value, QueryError> {
    if result.is_nan() && !input.is_nan() {
        return Err(QueryError::builtin("math domain error"));
    }
    Ok(Value::Float(result))
}

fn bi_sin(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "sin", 1)?);
    domain_checked(n, n.sin())
}

fn bi_cos(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "cos", 1)?);
    domain_checked(n, n.cos())
}

fn bi_tan(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "tan", 1)?);
    domain_checked(n, n.tan())
}

fn bi_asin(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "asin", 1)?);
    domain_checked(n, n.asin())
}

fn bi_acos(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "acos", 1)?);
    domain_checked(n, n.acos())
}

fn bi_atan(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "atan", 1)?);
    domain_checked(n, n.atan())
}

fn bi_atan2(args: &[Value]) -> Result<Value, QueryError> {
    let y = num_f64(&as_number(&args[0], "atan2", 1)?);
    let x = num_f64(&as_number(&args[1], "atan2", 2)?);
    Ok(Value::Float(y.atan2(x)))
}

fn bi_sinh(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "sinh", 1)?);
    domain_checked(n, n.sinh())
}

fn bi_cosh(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "cosh", 1)?);
    domain_checked(n, n.cosh())
}

fn bi_tanh(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "tanh", 1)?);
    domain_checked(n, n.tanh())
}

fn bi_asinh(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "asinh", 1)?);
    domain_checked(n, n.asinh())
}

fn bi_acosh(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "acosh", 1)?);
    domain_checked(n, n.acosh())
}

fn bi_atanh(args: &[Value]) -> Result<Value, QueryError> {
    let n = num_f64(&as_number(&args[0], "atanh", 1)?);
    domain_checked(n, n.atanh())
}

// IEEE helpers

fn bi_copysign(args: &[Value]) -> Result<Value, QueryError> {
    let x = num_f64(&as_number(&args[0], "copysign", 1)?);
    let y = num_f64(&as_number(&args[1], "copysign", 2)?);
    Ok(Value::Float(x.copysign(y)))
}

fn bi_fdim(args: &[Value]) -> Result<Value, QueryError> {
    // Computes `max(float(x - y), 0.0)`.
    let x = num_f64(&as_number(&args[0], "fdim", 1)?);
    let y = num_f64(&as_number(&args[1], "fdim", 2)?);
    let d = x - y;
    Ok(Value::Float(if d > 0.0 { d } else { 0.0 }))
}

fn bi_frexp(args: &[Value]) -> Result<Value, QueryError> {
    // Returns [mantissa: float, exponent: int].
    let n = num_f64(&as_number(&args[0], "frexp", 1)?);
    let (m, e) = libm::frexp(n);
    Ok(Value::List(vec![Value::Float(m), Value::Int(i64::from(e))]))
}

fn bi_ldexp(args: &[Value]) -> Result<Value, QueryError> {
    let m = num_f64(&as_number(&args[0], "ldexp", 1)?);
    let e = as_int(&args[1], "ldexp", 2)?;
    // Saturate the exponent to i32 range as C ldexp does for out-of-range.
    let ec = e.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    Ok(Value::Float(libm::ldexp(m, ec)))
}

fn bi_modf(args: &[Value]) -> Result<Value, QueryError> {
    // `modf` returns (frac, int) — both floats; libm::modf
    // returns the same `(frac, int)` order. Pure bit manipulation, so this
    // matches the platform C library exactly.
    let n = num_f64(&as_number(&args[0], "modf", 1)?);
    let (frac, integ) = libm::modf(n);
    Ok(Value::List(vec![Value::Float(frac), Value::Float(integ)]))
}

fn bi_hypot(args: &[Value]) -> Result<Value, QueryError> {
    let x = num_f64(&as_number(&args[0], "hypot", 1)?);
    let y = num_f64(&as_number(&args[1], "hypot", 2)?);
    Ok(Value::Float(x.hypot(y)))
}

fn bi_fma(args: &[Value]) -> Result<Value, QueryError> {
    // Computes the naive expression float(x)*float(y) + float(z).
    let x = num_f64(&as_number(&args[0], "fma", 1)?);
    let y = num_f64(&as_number(&args[1], "fma", 2)?);
    let z = num_f64(&as_number(&args[2], "fma", 3)?);
    Ok(Value::Float(x * y + z))
}

fn bi_fmax(args: &[Value]) -> Result<Value, QueryError> {
    // `max(x, y)` preserves the winning operand's int/float type and
    // its tie behaviour (returns the first when equal).
    py_max_min(&args[0], &args[1], true)
}

fn bi_fmin(args: &[Value]) -> Result<Value, QueryError> {
    py_max_min(&args[0], &args[1], false)
}

/// `max`/`min` over two numbers: returns the original value
/// (preserving int vs float), with `min`/`max` keeping the first operand on
/// ties (`x` is not replaced unless `y` is strictly better).
fn py_max_min(a: &Value, b: &Value, want_max: bool) -> Result<Value, QueryError> {
    let av = as_number(a, if want_max { "fmax" } else { "fmin" }, 1)?;
    let bv = as_number(b, if want_max { "fmax" } else { "fmin" }, 2)?;
    let af = num_f64(&av);
    let bf = num_f64(&bv);
    // `max(x, y)` returns y only when y > x; otherwise x.
    let pick_b = if want_max { bf > af } else { bf < af };
    Ok(if pick_b { bv } else { av })
}

fn bi_fmod(args: &[Value]) -> Result<Value, QueryError> {
    let x = num_f64(&as_number(&args[0], "fmod", 1)?);
    let y = num_f64(&as_number(&args[1], "fmod", 2)?);
    Ok(Value::Float(libm::fmod(x, y)))
}

fn bi_remainder(args: &[Value]) -> Result<Value, QueryError> {
    // Shared by `remainder` and `drem`.
    let x = num_f64(&as_number(&args[0], "remainder", 1)?);
    let y = num_f64(&as_number(&args[1], "remainder", 2)?);
    Ok(Value::Float(libm::remainder(x, y)))
}

// Gamma family

fn bi_gamma(args: &[Value]) -> Result<Value, QueryError> {
    // Shared by `gamma` and `tgamma`. The C domain error surfaces
    // as `BuiltinError("gamma: math domain error")`.
    let n = num_f64(&as_number(&args[0], "gamma", 1)?);
    if gamma_is_domain_error(n) {
        return Err(QueryError::builtin("gamma: math domain error"));
    }
    Ok(Value::Float(m_tgamma(n)))
}

/// `math.gamma` raises `ValueError` for non-positive integers and for
/// negative infinity (a domain error there).
fn gamma_is_domain_error(n: f64) -> bool {
    if n.is_nan() {
        return false;
    }
    if n == f64::NEG_INFINITY {
        return true;
    }
    n <= 0.0 && n.fract() == 0.0
}

fn bi_lgamma(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Float(m_lgamma(num_f64(&as_number(
        &args[0], "lgamma", 1,
    )?))))
}

fn bi_lgamma_r(args: &[Value]) -> Result<Value, QueryError> {
    // Returns [lgamma, sign]; sign is +1 when gamma(n) >= 0 (or on the
    // gamma-domain-error fallback), else -1.
    let n = num_f64(&as_number(&args[0], "lgamma_r", 1)?);
    let lg = m_lgamma(n);
    // Sign falls back to +1 when `gamma(n)` raises (the domain-error
    // case); otherwise the sign of `gamma(n)`.
    let sign: i64 = if !gamma_is_domain_error(n) && m_tgamma(n) < 0.0 {
        -1
    } else {
        1
    };
    Ok(Value::List(vec![Value::Float(lg), Value::Int(sign)]))
}

// Lanczos approximation for gamma / lgamma. `math.gamma` / `lgamma` do
// NOT use the platform libm — they use this Lanczos approximation — so we
// compute it directly for byte-identical results. The `errno`/`EDOM`
// signalling is handled by the callers (`gamma_is_domain_error`); these
// helpers just compute the value.

const LANCZOS_N: usize = 13;
const LANCZOS_G: f64 = 6.024_680_040_776_729_5;
const LANCZOS_G_MINUS_HALF: f64 = 5.524_680_040_776_729_5;
const NGAMMA_INTEGRAL: usize = 23;
const LOGPI: f64 = 1.144_729_885_849_400_2;

const LANCZOS_NUM_COEFFS: [f64; LANCZOS_N] = [
    23_531_376_880.410_76,
    42_919_803_642.649_1,
    35_711_959_237.355_667,
    17_921_034_426.037_21,
    6_039_542_586.352_028,
    1_439_720_407.311_721_7,
    248_874_557.862_054_15,
    31_426_415.585_400_194,
    2_876_370.628_935_372_4,
    186_056.265_395_223_5,
    8_071.672_002_365_816,
    210.824_277_751_579_35,
    2.506_628_274_631_000_3,
];
const LANCZOS_DEN_COEFFS: [f64; LANCZOS_N] = [
    0.0,
    39_916_800.0,
    120_543_840.0,
    150_917_976.0,
    105_258_076.0,
    45_995_730.0,
    13_339_535.0,
    2_637_558.0,
    357_423.0,
    32_670.0,
    1_925.0,
    66.0,
    1.0,
];
const GAMMA_INTEGRAL: [f64; NGAMMA_INTEGRAL] = [
    1.0,
    1.0,
    2.0,
    6.0,
    24.0,
    120.0,
    720.0,
    5_040.0,
    40_320.0,
    362_880.0,
    3_628_800.0,
    39_916_800.0,
    479_001_600.0,
    6_227_020_800.0,
    87_178_291_200.0,
    1_307_674_368_000.0,
    20_922_789_888_000.0,
    355_687_428_096_000.0,
    6_402_373_705_728_000.0,
    121_645_100_408_832_000.0,
    2_432_902_008_176_640_000.0,
    51_090_942_171_709_440_000.0,
    1_124_000_727_777_607_680_000.0,
];

fn lanczos_sum(x: f64) -> f64 {
    let mut num = 0.0;
    let mut den = 0.0;
    if x < 5.0 {
        let mut i = LANCZOS_N;
        while i > 0 {
            i -= 1;
            num = num * x + LANCZOS_NUM_COEFFS[i];
            den = den * x + LANCZOS_DEN_COEFFS[i];
        }
    } else {
        for i in 0..LANCZOS_N {
            num = num / x + LANCZOS_NUM_COEFFS[i];
            den = den / x + LANCZOS_DEN_COEFFS[i];
        }
    }
    num / den
}

fn m_sinpi(x: f64) -> f64 {
    use std::f64::consts::PI;
    let y = x.abs() % 2.0;
    let n = (2.0 * y).round() as i64;
    let r = match n {
        0 => (PI * y).sin(),
        1 => (PI * (y - 0.5)).cos(),
        2 => (PI * (1.0 - y)).sin(),
        3 => -(PI * (y - 1.5)).cos(),
        4 => (PI * (y - 2.0)).sin(),
        _ => f64::NAN,
    };
    1.0f64.copysign(x) * r
}

fn m_tgamma(x: f64) -> f64 {
    use std::f64::consts::PI;
    if x.is_nan() || (x.is_infinite() && x > 0.0) {
        return x;
    }
    if x.is_infinite() {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::INFINITY.copysign(x);
    }
    if x == x.floor() {
        if x < 0.0 {
            return f64::NAN;
        }
        if x <= NGAMMA_INTEGRAL as f64 {
            return GAMMA_INTEGRAL[(x as i64 - 1) as usize];
        }
    }
    let absx = x.abs();
    if absx < 1e-20 {
        return 1.0 / x;
    }
    if absx > 200.0 {
        return if x < 0.0 {
            0.0 / m_sinpi(x)
        } else {
            f64::INFINITY
        };
    }
    let y = absx + LANCZOS_G_MINUS_HALF;
    let z = if absx > LANCZOS_G_MINUS_HALF {
        let q = y - absx;
        q - LANCZOS_G_MINUS_HALF
    } else {
        let q = y - LANCZOS_G_MINUS_HALF;
        q - absx
    };
    let z = z * LANCZOS_G / y;
    let mut r;
    if x < 0.0 {
        r = -PI / m_sinpi(absx) / absx * y.exp() / lanczos_sum(absx);
        r -= z * r;
        if absx < 140.0 {
            r /= y.powf(absx - 0.5);
        } else {
            let sqrtpow = y.powf(absx / 2.0 - 0.25);
            r /= sqrtpow;
            r /= sqrtpow;
        }
    } else {
        r = lanczos_sum(absx) / y.exp();
        r += z * r;
        if absx < 140.0 {
            r *= y.powf(absx - 0.5);
        } else {
            let sqrtpow = y.powf(absx / 2.0 - 0.25);
            r *= sqrtpow;
            r *= sqrtpow;
        }
    }
    r
}

fn m_lgamma(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x.is_infinite() {
        return f64::INFINITY;
    }
    if x == x.floor() && x <= 2.0 {
        if x <= 0.0 {
            return f64::INFINITY;
        }
        return 0.0;
    }
    let absx = x.abs();
    if absx < 1e-20 {
        return -absx.ln();
    }
    let mut r = lanczos_sum(absx).ln() - LANCZOS_G;
    r += (absx - 0.5) * ((absx + LANCZOS_G - 0.5).ln() - 1.0);
    if x < 0.0 {
        r = LOGPI - m_sinpi(absx).abs().ln() - absx.ln() - r;
    }
    r
}

// Bessel (Abramowitz & Stegun polynomial approximations, matching the
// reference approximation so results stay byte-identical).

fn bi_j0(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Float(bessel_j0(num_f64(&as_number(
        &args[0], "j0", 1,
    )?))))
}

fn bi_j1(args: &[Value]) -> Result<Value, QueryError> {
    Ok(Value::Float(bessel_j1(num_f64(&as_number(
        &args[0], "j1", 1,
    )?))))
}

fn bi_y0(args: &[Value]) -> Result<Value, QueryError> {
    let v = as_number(&args[0], "y0", 1)?;
    let n = num_f64(&v);
    if n <= 0.0 {
        return Err(QueryError::builtin(format!(
            "y0: argument must be positive, got {}",
            num_str(&v)
        )));
    }
    Ok(Value::Float(bessel_y0(n)))
}

fn bi_y1(args: &[Value]) -> Result<Value, QueryError> {
    let v = as_number(&args[0], "y1", 1)?;
    let n = num_f64(&v);
    if n <= 0.0 {
        return Err(QueryError::builtin(format!(
            "y1: argument must be positive, got {}",
            num_str(&v)
        )));
    }
    Ok(Value::Float(bessel_y1(n)))
}

fn bi_jn(args: &[Value]) -> Result<Value, QueryError> {
    let order = as_int(&args[0], "jn", 1)?;
    let v = as_number(&args[1], "jn", 2)?;
    Ok(Value::Float(bessel_jn(order, num_f64(&v))))
}

fn bi_yn(args: &[Value]) -> Result<Value, QueryError> {
    let order = as_int(&args[0], "yn", 1)?;
    let v = as_number(&args[1], "yn", 2)?;
    let val = num_f64(&v);
    if val <= 0.0 {
        return Err(QueryError::builtin(format!(
            "yn: argument must be positive, got {}",
            num_str(&v)
        )));
    }
    Ok(Value::Float(bessel_yn(order, val)))
}

fn bessel_jn(order: i64, val: f64) -> f64 {
    if order == 0 {
        return bessel_j0(val);
    }
    if order == 1 {
        return bessel_j1(val);
    }
    if order < 0 {
        let sign = if order & 1 != 0 { -1.0 } else { 1.0 };
        return sign * bessel_jn(-order, val);
    }
    if val == 0.0 {
        return 0.0;
    }
    let mut jm1 = bessel_j0(val);
    let mut j = bessel_j1(val);
    for k in 1..order {
        let jp1 = (2.0 * k as f64 / val) * j - jm1;
        jm1 = j;
        j = jp1;
    }
    j
}

fn bessel_yn(order: i64, val: f64) -> f64 {
    if order == 0 {
        return bessel_y0(val);
    }
    if order == 1 {
        return bessel_y1(val);
    }
    if order < 0 {
        let sign = if order & 1 != 0 { -1.0 } else { 1.0 };
        return sign * bessel_yn(-order, val);
    }
    let mut ym1 = bessel_y0(val);
    let mut y = bessel_y1(val);
    for k in 1..order {
        let yp1 = (2.0 * k as f64 / val) * y - ym1;
        ym1 = y;
        y = yp1;
    }
    y
}

fn bessel_j0(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 8.0 {
        let y = x * x;
        let num = 57568490574.0
            + y * (-13362590354.0
                + y * (651619640.7 + y * (-11214424.18 + y * (77392.33017 + y * (-184.9052456)))));
        let den = 57568490411.0
            + y * (1029532985.0 + y * (9494680.718 + y * (59272.64853 + y * (267.8532712 + y))));
        return num / den;
    }
    let z = 8.0 / ax;
    let y = z * z;
    let p0 = 1.0
        + y * (-0.1098628627e-2
            + y * (0.2734510407e-4 + y * (-0.2073370639e-5 + y * 0.2093887211e-6)));
    let q0 = -0.1562499995e-1
        + y * (0.1430488765e-3
            + y * (-0.6911147651e-5 + y * (0.7621095161e-6 + y * (-0.934935152e-7))));
    (0.636619772 / ax).sqrt()
        * ((ax - std::f64::consts::PI / 4.0).cos() * p0
            - z * (ax - std::f64::consts::PI / 4.0).sin() * q0)
}

fn bessel_j1(x: f64) -> f64 {
    let ax = x.abs();
    if ax < 8.0 {
        let y = x * x;
        let num = x
            * (72362614232.0
                + y * (-7895059235.0
                    + y * (242396853.1
                        + y * (-2972611.439 + y * (15704.48260 + y * (-30.16036606))))));
        let den = 144725228442.0
            + y * (2300535178.0 + y * (18583304.74 + y * (99447.43394 + y * (376.9991397 + y))));
        return num / den;
    }
    let z = 8.0 / ax;
    let y = z * z;
    let p1 = 1.0
        + y * (0.183105e-2
            + y * (-0.3516396496e-4 + y * (0.2457520174e-5 + y * (-0.240337019e-6))));
    let q1 = 0.04687499995
        + y * (-0.2002690873e-3
            + y * (0.8449199096e-5 + y * (-0.88228987e-6 + y * 0.105787412e-6)));
    let result = (0.636619772 / ax).sqrt()
        * ((ax - 3.0 * std::f64::consts::PI / 4.0).cos() * p1
            - z * (ax - 3.0 * std::f64::consts::PI / 4.0).sin() * q1);
    if x < 0.0 { -result } else { result }
}

fn bessel_y0(x: f64) -> f64 {
    if x < 8.0 {
        let y = x * x;
        let num = -2957821389.0
            + y * (7062834065.0
                + y * (-512359803.6 + y * (10879881.29 + y * (-86327.92757 + y * 228.4622733))));
        let den = 40076544269.0
            + y * (745249964.8 + y * (7189466.438 + y * (47447.26470 + y * (226.1030244 + y))));
        return num / den + 0.636619772 * bessel_j0(x) * x.ln();
    }
    let z = 8.0 / x;
    let y = z * z;
    let p0 = 1.0
        + y * (-0.1098628627e-2
            + y * (0.2734510407e-4 + y * (-0.2073370639e-5 + y * 0.2093887211e-6)));
    let q0 = -0.1562499995e-1
        + y * (0.1430488765e-3
            + y * (-0.6911147651e-5 + y * (0.7621095161e-6 + y * (-0.934935152e-7))));
    (0.636619772 / x).sqrt()
        * ((x - std::f64::consts::PI / 4.0).sin() * p0
            + z * (x - std::f64::consts::PI / 4.0).cos() * q0)
}

fn bessel_y1(x: f64) -> f64 {
    if x < 8.0 {
        let y = x * x;
        let num = x
            * (-0.4900604943e13
                + y * (0.1275274390e13
                    + y * (-0.5153438139e11
                        + y * (0.7349264551e9 + y * (-0.4237922726e7 + y * 0.8511937935e4)))));
        let den = 0.2499580570e14
            + y * (0.4244419664e12
                + y * (0.3733650367e10
                    + y * (0.2245904002e8 + y * (0.1020426050e6 + y * (0.3549632885e3 + y)))));
        return num / den + 0.636619772 * (bessel_j1(x) * x.ln() - 1.0 / x);
    }
    let z = 8.0 / x;
    let y = z * z;
    let p1 = 1.0
        + y * (0.183105e-2
            + y * (-0.3516396496e-4 + y * (0.2457520174e-5 + y * (-0.240337019e-6))));
    let q1 = 0.04687499995
        + y * (-0.2002690873e-3
            + y * (0.8449199096e-5 + y * (-0.88228987e-6 + y * 0.105787412e-6)));
    (0.636619772 / x).sqrt()
        * ((x - 3.0 * std::f64::consts::PI / 4.0).sin() * p1
            + z * (x - 3.0 * std::f64::consts::PI / 4.0).cos() * q1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_of(n: f64) -> i64 {
        match bi_round(&[Value::Float(n)]) {
            Ok(Value::Int(i)) => i,
            other => panic!("round returned {other:?}"),
        }
    }

    fn fabs_of(n: f64) -> f64 {
        match bi_fabs(&[Value::Float(n)]) {
            Ok(Value::Float(f)) => f,
            other => panic!("fabs returned {other:?}"),
        }
    }

    #[test]
    fn round_ties_away_from_zero() {
        // Round half is C/jq ties-away-from-zero, not banker's rounding.
        assert_eq!(round_of(2.5), 3);
        assert_eq!(round_of(-2.5), -3);
        assert_eq!(round_of(0.5), 1);
        assert_eq!(round_of(-3.7), -4);
    }

    #[test]
    fn fabs_returns_float_magnitude() {
        // `fabs` always returns a float.
        assert_eq!(fabs_of(-5.0), 5.0);
        assert_eq!(fabs_of(3.14), 3.14);
    }
}
