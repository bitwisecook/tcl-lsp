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

/// A transient numeric value for math-function dispatch. Integer-preserving
/// functions keep the arbitrary-precision `B` rung; floating-point functions
/// widen through [`BigIntOps::to_f64`](crate::number_tower::BigIntOps::to_f64).
/// Each consumer converts its own value to and from this shared shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumValue<B> {
    /// Integer.
    Int(i64),
    /// Arbitrary-precision integer.
    Big(B),
    /// IEEE-754 double.
    Float(f64),
}

/// Numeric shape for callers that have no arbitrary-precision backend.
pub type Num = NumValue<NoBig>;

/// Uninhabited backend used by [`Num`]; a `NumValue<NoBig>` can never contain
/// the [`NumValue::Big`] variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NoBig {}

impl super::super::number_tower::BigIntOps for NoBig {
    const HAS_BIGNUM_RUNG: bool = false;
    fn from_i64(_: i64) -> Self {
        unreachable!("NoBig has no value; promote through `promote_i64`")
    }
    fn to_i64(&self) -> Option<i64> {
        match *self {}
    }
    fn to_i64_wrapping(&self) -> i64 {
        match *self {}
    }
    fn is_zero(&self) -> bool {
        match *self {}
    }
    fn is_negative(&self) -> bool {
        match *self {}
    }
    fn add(&self, _: &Self) -> Self {
        match *self {}
    }
    fn sub(&self, _: &Self) -> Self {
        match *self {}
    }
    fn mul(&self, _: &Self) -> Self {
        match *self {}
    }
    fn div_floor(&self, _: &Self) -> Self {
        match *self {}
    }
    fn mod_floor(&self, _: &Self) -> Self {
        match *self {}
    }
    fn neg(&self) -> Self {
        match *self {}
    }
    fn pow_u32(&self, _: u32) -> Self {
        match *self {}
    }
    fn shl(&self, _: u32) -> Self {
        match *self {}
    }
    fn shr(&self, _: usize) -> Self {
        match *self {}
    }
    fn bitand(&self, _: &Self) -> Self {
        match *self {}
    }
    fn bitor(&self, _: &Self) -> Self {
        match *self {}
    }
    fn bitxor(&self, _: &Self) -> Self {
        match *self {}
    }
    fn bit_len(&self) -> u64 {
        match *self {}
    }
    fn to_f64(&self) -> f64 {
        match *self {}
    }
}

impl<B> NumValue<B> {
    /// As an `f64` (widening an integer).
    #[must_use]
    pub fn as_f64(&self) -> f64
    where
        B: super::super::number_tower::BigIntOps,
    {
        match self {
            NumValue::Int(i) => *i as f64,
            NumValue::Big(b) => b.to_f64(),
            NumValue::Float(f) => *f,
        }
    }
    fn is_truthy(&self) -> bool
    where
        B: super::super::number_tower::BigIntOps,
    {
        match self {
            NumValue::Int(i) => *i != 0,
            NumValue::Big(b) => !b.is_zero(),
            NumValue::Float(f) => *f != 0.0,
        }
    }
}

/// Whether `int()` keeps arbitrary precision or narrows to the signed 64-bit
/// window — the one **release-dependent** math-function semantic.
///
/// Tcl 9.0 binds `int` and `entier` to the same unbounded `ExprIntFunc`
/// (`tclBasic.c`), while 8.4-8.6 keep `int`'s windowing: `tclsh8.6.16` says
/// `int(1e20)` is `7766279631452241920` where `tclsh9.0.4` says
/// `100000000000000000000`, and likewise `int(2**64+1)` is `1` versus
/// `18446744073709551617`. `wide()` windows in every release, `entier()` is
/// unbounded in every release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntWidth {
    /// `int()` narrows to the 64-bit window (Tcl 8.4-8.6).
    Windowed,
    /// `int()` keeps arbitrary precision, exactly like `entier()` (Tcl 9.0+).
    Unbounded,
    /// The caller has not resolved a release. `int()` then answers only where
    /// the two releases agree (an operand that fits a wide) and abstains
    /// otherwise, so a const-folder can never bake in the wrong release's
    /// answer.
    Unresolved,
}

impl IntWidth {
    /// The `int()` width the given core release uses.
    #[must_use]
    pub fn for_tcl_version(version: tcl_dialect::TclVersion) -> Self {
        if version >= tcl_dialect::TclVersion::V9_0 {
            Self::Unbounded
        } else {
            Self::Windowed
        }
    }
}

/// Why a math-function dispatch could not produce a value — C's error surface
/// for `expr`'s function calls, carried so both engines report the same
/// message *and* `-errorcode` instead of collapsing every refusal into one
/// generic domain error.
///
/// The message/code pairs are tclsh 8.6.16 and 9.0.4 output, read with
/// `catch {expr {...}} m o; list $m [dict get $o -errorcode]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathFuncError {
    /// `name` is not a built-in `expr` function in any release.
    UnknownFunction,
    /// The call has the wrong number of arguments for the function.
    WrongArgCount,
    /// An infinity reached an integer conversion (`entier`/`int`/`wide`/
    /// `round`/`isqrt`): `integer value too large to represent`,
    /// `-errorcode ARITH IOVERFLOW {integer value too large to represent}`.
    IntegerOverflow,
    /// A NaN reached a function that needs a real number:
    /// `floating point value is Not a Number`,
    /// `-errorcode TCL VALUE DOUBLE NAN`.
    NotANumber,
    /// A genuine domain error (`sqrt(-1)`, `fmod(x, 0)`, `log(-1)`):
    /// `domain error: argument not in valid range`, `-errorcode ARITH DOMAIN
    /// {domain error: argument not in valid range}`.
    Domain,
    /// `isqrt` of a negative operand — C keeps its own message
    /// (`square root of negative argument`) with the ordinary domain
    /// `-errorcode`.
    NegativeSqrt,
    /// Tcl 8.4: a result past the double range (`floating-point value too
    /// large to represent`, `-errorcode ARITH OVERFLOW …`).
    FloatOverflow,
    /// Tcl 8.4: a zero result where the exact value is not zero
    /// (`floating-point value too small to represent`, `-errorcode ARITH
    /// UNDERFLOW …`).
    FloatUnderflow,
    /// **Not a Tcl error.** There is no answer *this caller* can carry: its
    /// backend has no arbitrary-precision rung ([`Num`], the const-folder's
    /// shape) or it did not resolve a release and `int()`'s width is
    /// release-dependent. A const-folder must abstain — leave the call to run
    /// at run time — never turn this into an error.
    Abstain,
}

impl MathFuncError {
    /// C's verbatim message, or `""` for the two refusals that have none of
    /// their own ([`Self::UnknownFunction`] / [`Self::WrongArgCount`] are
    /// worded by the caller, which knows the invoked spelling, and
    /// [`Self::Abstain`] is not an error at all).
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            MathFuncError::IntegerOverflow => super::errors::IOVERFLOW_MESSAGE,
            MathFuncError::NotANumber => super::errors::NAN_MESSAGE,
            MathFuncError::Domain => super::errors::DOMAIN_MESSAGE,
            MathFuncError::NegativeSqrt => "square root of negative argument",
            MathFuncError::FloatOverflow => super::errors::FLOAT_OVERFLOW_MESSAGE,
            MathFuncError::FloatUnderflow => super::errors::FLOAT_UNDERFLOW_MESSAGE,
            _ => "",
        }
    }

    /// C's verbatim `-errorcode`, or `""` when the caller supplies it.
    /// `isqrt`'s negative-operand error reuses the ordinary domain code
    /// (tclsh verified), even though its message differs.
    #[must_use]
    pub fn error_code(self) -> &'static str {
        match self {
            MathFuncError::IntegerOverflow => super::errors::IOVERFLOW_CODE,
            MathFuncError::NotANumber => super::errors::NAN_CODE,
            MathFuncError::Domain | MathFuncError::NegativeSqrt => super::errors::DOMAIN_CODE,
            MathFuncError::FloatOverflow => super::errors::FLOAT_OVERFLOW_CODE,
            MathFuncError::FloatUnderflow => super::errors::FLOAT_UNDERFLOW_CODE,
            _ => "",
        }
    }
}

/// Whether `name` is one of the classification predicates that *inspect* a
/// NaN or infinity rather than computing with it — the functions for which a
/// NaN operand is the point, not an error (`isnan(NaN)` is `1`).
fn inspects_non_finite(name: &str) -> bool {
    matches!(
        name,
        "isnan" | "isinf" | "isfinite" | "isnormal" | "issubnormal" | "isunordered" | "signbit"
    )
}

/// The integer conversions: an infinity is `ARITH IOVERFLOW` here, while
/// `abs(Inf)` / `double(Inf)` / `floor(Inf)` all answer `Inf` (tclsh
/// verified).
fn converts_to_integer(name: &str) -> bool {
    matches!(name, "entier" | "int" | "wide" | "round" | "isqrt")
}

/// How one of the three integer conversions treats an operand that is
/// **already** an integer object.
///
/// C's `ExprEntierFunc` hands such an operand straight back, so the result
/// keeps the operand's own string representation
/// (`::tcl::mathfunc::entier 0x10` is `0x10`, not `16`, on tclsh 8.5-9.1).
/// A conversion that narrows has to build a new integer and therefore cannot
/// preserve it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerConversion {
    /// Hand the operand back as the same object, keeping its string rep.
    Preserve,
    /// Narrow to C's low-64-bit window, building a new integer.
    Window,
}

/// What `name` does to an already-integral operand under `int_width`, or
/// `None` when `name` is not an integer conversion that can preserve its
/// operand, or the release is unresolved.
///
/// This is the single owner of the policy: `wide` windows in every release,
/// `entier` is unbounded in every release, and `int` follows the release's
/// [`IntWidth`] axis — 8.4-8.6 window it, 9.0+ bind it to the same unbounded
/// conversion as `entier`. Under [`IntWidth::Unresolved`] the caller has not
/// pinned a release, and `int` would differ between them, so it abstains
/// rather than letting a consumer bake in one release's answer — the same
/// rule [`abstains`] applies to the value table.
///
/// `round` and `isqrt` convert to an integer but always compute a new value,
/// so they are not preservation candidates and answer `None`.
#[must_use]
pub fn integer_conversion(name: &str, int_width: IntWidth) -> Option<IntegerConversion> {
    match name {
        "wide" => Some(IntegerConversion::Window),
        "entier" => Some(IntegerConversion::Preserve),
        "int" => match int_width {
            IntWidth::Windowed => Some(IntegerConversion::Window),
            IntWidth::Unbounded => Some(IntegerConversion::Preserve),
            IntWidth::Unresolved => None,
        },
        _ => None,
    }
}

/// Dispatch with an **error channel**: the same table as
/// [`dispatch_with_backend_int_width`], but each refusal keeps the class C
/// reports it as, so a runtime can stamp the right message and `-errorcode`,
/// and a const-folder can tell "this would raise" from "I cannot
/// represent the answer" ([`MathFuncError::Abstain`]).
///
/// A const-folder must abstain on **every** `Err`, never fold one into a
/// value: the erroring call may sit in a branch that never runs.
///
/// # Errors
/// See [`MathFuncError`] for the classes and their C message/`-errorcode`.
pub fn try_dispatch_with_backend_int_width<B: super::super::number_tower::BigIntOps>(
    name: &str,
    args: &[NumValue<B>],
    int_width: IntWidth,
) -> Result<NumValue<B>, MathFuncError> {
    let Some(spec) = spec(name) else {
        return Err(MathFuncError::UnknownFunction);
    };
    let n = args.len();
    if n < usize::from(spec.arity.min) || spec.arity.max.is_some_and(|m| n > usize::from(m)) {
        return Err(MathFuncError::WrongArgCount);
    }
    if !inspects_non_finite(name) {
        // C converts each operand before computing, and the two non-finite
        // refusals it raises there outrank any later domain error.
        for arg in args {
            if let NumValue::Float(f) = arg {
                if f.is_nan() {
                    return Err(MathFuncError::NotANumber);
                }
                if f.is_infinite() && converts_to_integer(name) {
                    return Err(MathFuncError::IntegerOverflow);
                }
            }
        }
    }
    if name == "isqrt" && is_negative(&args[0]) {
        return Err(MathFuncError::NegativeSqrt);
    }
    dispatch_with_backend_int_width(name, args, int_width).ok_or_else(|| {
        // A finite operand the backend cannot represent exactly (no bignum
        // rung), or `int()` with an unresolved release, is an abstention —
        // everything else that declines is a real domain error.
        if abstains(name, args, int_width) {
            MathFuncError::Abstain
        } else {
            MathFuncError::Domain
        }
    })
}

/// [`try_dispatch_with_backend_int_width`] under `release`: its `int()` width,
/// and the checks Tcl 8.4 makes after each call. 8.4 refuses a double past the
/// integer range in `int`, `wide` and `round` (`ExprIntFunc`, `ExprWideFunc`,
/// `ExprRoundFunc`), and checks `errno` and the result of every other function
/// (`ExprUnaryFunc`, `ExprBinaryFunc`).
///
/// # Errors
/// As [`try_dispatch_with_backend_int_width`], and under 8.4
/// [`MathFuncError::IntegerOverflow`], [`MathFuncError::FloatOverflow`],
/// [`MathFuncError::FloatUnderflow`] or [`MathFuncError::Domain`].
pub fn try_dispatch_for_release<B: super::super::number_tower::BigIntOps>(
    name: &str,
    args: &[NumValue<B>],
    release: tcl_dialect::TclVersion,
) -> Result<NumValue<B>, MathFuncError> {
    let checks = super::errors::checks_float_range(release);
    if checks && matches!(args, [NumValue::Float(x)] if integer_conversion_overflows_84(name, *x)) {
        return Err(MathFuncError::IntegerOverflow);
    }
    let result =
        try_dispatch_with_backend_int_width(name, args, IntWidth::for_tcl_version(release))?;
    match result {
        NumValue::Float(value) if checks => {
            let operands: Vec<f64> = args.iter().map(NumValue::as_f64).collect();
            float_function_error(name, &operands, value).map_or(Ok(result), Err)
        }
        _ => Ok(result),
    }
}

/// What 8.4 makes of `value`, the double `name` computed from `args`
/// (`TclExprFloatError` after the `errno` check): a NaN is a domain error, an
/// infinity too large, and a zero where the exact value is not zero too small.
/// The C library reports `ERANGE` only for a result that overflows or
/// underflows to zero, so a subnormal one passes (tclsh 8.4.20: `exp(-745)` is
/// `4.94065645841e-324`, `exp(-746)` is too small).
fn float_function_error(name: &str, args: &[f64], value: f64) -> Option<MathFuncError> {
    if value.is_nan() {
        Some(MathFuncError::Domain)
    } else if value.is_infinite() {
        Some(MathFuncError::FloatOverflow)
    } else if value == 0.0 && exact_value_is_non_zero(name, args) {
        Some(MathFuncError::FloatUnderflow)
    } else {
        None
    }
}

/// Whether the exact value of `name` over the finite `args` is non-zero, so a
/// zero result is an underflow rather than the answer.
fn exact_value_is_non_zero(name: &str, args: &[f64]) -> bool {
    match (name, args) {
        ("exp", _) => true,
        ("pow" | "atan2", [first, _]) => *first != 0.0,
        ("sin" | "tan" | "asin" | "atan" | "sinh" | "tanh", [x]) => *x != 0.0,
        _ => false,
    }
}

/// Whether 8.4 refuses the double `x` in the integer conversion `name`: `int`
/// and `wide` beyond ±2^63, `round` once rounded to ±2^63 or beyond (tclsh
/// 8.4.20: `int(9.3e18)` and `round(9223372036854775807.0)` are `integer value
/// too large to represent`, `int(9223372036854775807.0)` is not).
fn integer_conversion_overflows_84(name: &str, x: f64) -> bool {
    const LIMIT: f64 = 9_223_372_036_854_775_808.0;
    match name {
        "int" | "wide" => x.abs() > LIMIT,
        "round" => x.round().abs() >= LIMIT,
        _ => false,
    }
}

/// Whether `v` is negative (`isqrt`'s own refusal, which C words specially).
fn is_negative<B: super::super::number_tower::BigIntOps>(v: &NumValue<B>) -> bool {
    match v {
        NumValue::Int(i) => *i < 0,
        NumValue::Big(b) => b.is_negative(),
        NumValue::Float(f) => *f < 0.0,
    }
}

/// Whether a `None` from the value table means "no answer this caller can
/// carry" rather than "C would raise a domain error": an exact conversion
/// (or `abs(i64::MIN)`) whose result needs a bignum rung the backend lacks,
/// or `int()` under an unresolved release.
fn abstains<B: super::super::number_tower::BigIntOps>(
    name: &str,
    args: &[NumValue<B>],
    int_width: IntWidth,
) -> bool {
    if name == "abs" {
        return !B::HAS_BIGNUM_RUNG && matches!(args[0], NumValue::Int(i64::MIN));
    }
    if !converts_to_integer(name) {
        return false;
    }
    let beyond_wide = match &args[0] {
        NumValue::Float(f) => f.is_finite() && finite_trunc_to_i64(*f).is_none(),
        NumValue::Big(b) => b.to_i64().is_none(),
        NumValue::Int(_) => false,
    };
    beyond_wide && (!B::HAS_BIGNUM_RUNG || (name == "int" && int_width == IntWidth::Unresolved))
}

/// Dispatch a math function by (lowercased) `name` over already-evaluated
/// numeric `args`. Returns `None` for an unknown function, a wrong argument
/// count, or a domain error (matching the const-folder "give up" / runtime
/// "fall through" contract). `rand`/`srand` are the caller's responsibility
/// ([`super::rand`] owns their generator).
///
/// Release-agnostic: `int()` abstains where 8.6 and 9.0 disagree
/// ([`IntWidth::Unresolved`]).
#[must_use]
pub fn dispatch(name: &str, args: &[Num]) -> Option<Num> {
    dispatch_with_backend(name, args)
}

/// [`dispatch`] as an implementation reading numbers under `numbers`
/// computes `name`, or `None` where this table does not model its answer.
///
/// Jim's `pow` is its `**` operator (`JimExprOpBin`, `jim.c`), an integer
/// power when both operands are integers
/// ([`NumberSyntax::pow_of_integers_is_integer`]): jimsh 0.84 prints `8` for
/// `pow(2,3)` and `0` for `pow(2,-1)`, where tclsh 8.4.20 to 9.1.0 print `8.0`
/// and `0.5`. That case declines; a double operand computes alike.
///
/// [`NumberSyntax::pow_of_integers_is_integer`]: tcl_dialect::NumberSyntax::pow_of_integers_is_integer
#[must_use]
pub fn dispatch_under(name: &str, args: &[Num], numbers: tcl_dialect::NumberSyntax) -> Option<Num> {
    let integer_operands = args.iter().all(|arg| matches!(arg, Num::Int(_)));
    if name == "pow" && integer_operands && numbers.pow_of_integers_is_integer() {
        return None;
    }
    dispatch(name, args)
}

/// Dispatch through the same math-function table while preserving the
/// caller's arbitrary-precision integer backend, release-agnostic
/// ([`IntWidth::Unresolved`]).
#[must_use]
pub fn dispatch_with_backend<B: super::super::number_tower::BigIntOps>(
    name: &str,
    args: &[NumValue<B>],
) -> Option<NumValue<B>> {
    dispatch_with_backend_int_width(name, args, IntWidth::Unresolved)
}

/// Dispatch with the caller's backend **and** its resolved release, so
/// `int()` answers with that release's width.
#[must_use]
pub fn dispatch_with_backend_int_width<B: super::super::number_tower::BigIntOps>(
    name: &str,
    args: &[NumValue<B>],
    int_width: IntWidth,
) -> Option<NumValue<B>> {
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
            type_conv(name, args, int_width)
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

/// The message and `-errorcode` `release` gives an argument of the math
/// function `name` that is no number: 8.4's `ExprUnaryFunc` and kin one
/// message and no code, from 8.5 the reader's own words with `TCL VALUE
/// NUMBER`.
#[must_use]
pub fn non_numeric_argument_error(
    name: &str,
    value: &str,
    release: tcl_dialect::TclVersion,
) -> (String, &'static str) {
    if release == tcl_dialect::TclVersion::V8_4 {
        return (
            "argument to math function didn't have numeric value".to_owned(),
            "NONE",
        );
    }
    let expected = if expects_floating_operand_error(name) {
        "floating-point number"
    } else {
        "number"
    };
    (
        format!("expected {expected} but got \"{value}\""),
        "TCL VALUE NUMBER",
    )
}

/// Whether a failed operand parse uses Tcl's floating-point error wording.
/// This belongs beside the shared math-function table so runtimes do not
/// re-derive the operand family from command names.
#[must_use]
pub fn expects_floating_operand_error(name: &str) -> bool {
    matches!(
        name,
        "sqrt"
            | "floor"
            | "ceil"
            | "pow"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "sinh"
            | "cosh"
            | "tanh"
            | "exp"
            | "log"
            | "log10"
            | "atan2"
            | "hypot"
            | "fmod"
    )
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

/// What a math function returns: the one table every consumer that types
/// an `expr` function call reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathResultClass {
    /// An integer: `int`, `round`, `wide`, `entier`, `isqrt`.
    Int,
    /// A double: `double`, `ceil` and `floor` (`expr {ceil(3.14)}` is
    /// `4.0`), and every transcendental function.
    Float,
    /// A number whose representation follows the operands: the result is
    /// one of them (`abs`, `max`, `min`), so its type is their join.
    Numeric,
    /// `0` or `1`: `bool` and the classification predicates.
    Bool,
    /// Whatever the function returns. No built-in has this class; it is the
    /// answer for a name the table does not describe, such as a
    /// `::tcl::mathfunc` command the program defines itself.
    Any,
}

/// Static facts about one `expr` math function — the string-keyed
/// counterpart to `operators::OperatorSpec` (math functions are open and
/// overridable via `::tcl::mathfunc::*`, TIP 232, so there's no closed enum
/// to attach metadata to). This is the fact table `mathfunc_generated.rs`
/// (layer 2) reads for hover/completion; it carries no behaviour of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathFuncSpec {
    /// Function name, matched verbatim (mathfunc lookup is case-sensitive).
    pub name: &'static str,
    /// The release this function first appeared in.
    pub since: MathFuncSince,
    /// Argument-count contract.
    pub arity: super::operators::CommandArity,
    /// Whether the operand accepts Tcl boolean words (`true`/`yes`/…) — see
    /// [`accepts_boolean_operand`].
    pub accepts_boolean_operand: bool,
    /// What the function returns.
    pub result_class: MathResultClass,
    /// A one-line human summary for hover text.
    pub summary: &'static str,
}

/// The result class of math function `name`: its [`MathFuncSpec`]'s, or
/// [`MathResultClass::Any`] for a name no release defines.
#[must_use]
pub fn result_class(name: &str) -> MathResultClass {
    spec(name).map_or(MathResultClass::Any, |spec| spec.result_class)
}

/// The table [`MathFuncSpec::result_class`] is filled from, keyed like
/// [`added_in`]. Verified against tclsh 8.4 to 9.1: `ceil(3.14)` is `4.0`,
/// `trunc(2.5)` is `2.0`, and `isfinite`, `isnormal`, `issubnormal` and
/// `isunordered` answer `0` or `1` like `isnan` and `isinf`.
fn class_of(name: &str) -> MathResultClass {
    match name {
        "int" | "round" | "wide" | "entier" | "isqrt" => MathResultClass::Int,
        "abs" | "max" | "min" => MathResultClass::Numeric,
        "bool" | "isfinite" | "isinf" | "isnan" | "isnormal" | "issubnormal" | "isunordered"
        | "signbit" => MathResultClass::Bool,
        _ if added_in(name).is_some() => MathResultClass::Float,
        _ => MathResultClass::Any,
    }
}

/// Static metadata for math function `name`, or `None` when `name` isn't a
/// built-in `expr` function in any release — the single source layer 2
/// (`mathfunc_generated.rs`) reads to build hover/completion `CommandSpec`s.
/// Split into one helper per release (mirroring [`added_in`]'s own grouping)
/// purely to stay under clippy's function-length lint.
#[must_use]
pub fn spec(name: &str) -> Option<MathFuncSpec> {
    spec_tcl84(name)
        .or_else(|| spec_tcl85(name))
        .or_else(|| spec_tcl90(name))
        .or_else(|| spec_tcl91(name))
}

/// All built-in `expr` math functions, in a fixed (declaration) order —
/// the completeness anchor for consumers that need to enumerate every
/// function (layer 2's generator, completion, editor-generator sweeps).
#[must_use]
pub fn all() -> Vec<MathFuncSpec> {
    ALL_NAMES
        .iter()
        .map(|&n| spec(n).expect("name is in spec()'s own tables"))
        .collect()
}

/// Every name [`spec`] recognises, in declaration order — kept in one place
/// so [`all`] can't drift from the four `spec_tcl8x`/`spec_tcl9x` helpers.
const ALL_NAMES: &[&str] = &[
    "abs",
    "acos",
    "asin",
    "atan",
    "atan2",
    "ceil",
    "cos",
    "cosh",
    "double",
    "exp",
    "floor",
    "fmod",
    "hypot",
    "int",
    "log",
    "log10",
    "pow",
    "rand",
    "round",
    "sin",
    "sinh",
    "sqrt",
    "srand",
    "tan",
    "tanh",
    "wide",
    "bool",
    "entier",
    "isqrt",
    "max",
    "min",
    "isfinite",
    "isinf",
    "isnan",
    "isnormal",
    "issubnormal",
    "isunordered",
    "acosh",
    "asinh",
    "atanh",
    "cbrt",
    "copysign",
    "dim",
    "erf",
    "erfc",
    "exp2",
    "expm1",
    "fma",
    "gamma",
    "ldexp",
    "lgamma",
    "log1p",
    "log2",
    "logb",
    "nextafter",
    "remainder",
    "signbit",
    "trunc",
];

/// The 8.4 fixed C function table (`tclExecute.c`).
fn spec_tcl84(name: &str) -> Option<MathFuncSpec> {
    use super::operators::CommandArity as Arity;
    // Each arm's first element is the `'static` spelling itself — `name`
    // (the parameter) borrows from the caller, so the `MathFuncSpec.name`
    // field (which must outlive `'static`) has to come from the match arm's
    // own literal, not from `name` directly.
    let (name, arity, summary) = match name {
        "abs" => ("abs", Arity::exact(1), "absolute value"),
        "acos" => ("acos", Arity::exact(1), "arc cosine"),
        "asin" => ("asin", Arity::exact(1), "arc sine"),
        "atan" => ("atan", Arity::exact(1), "arc tangent"),
        "atan2" => ("atan2", Arity::exact(2), "arc tangent of y/x"),
        "ceil" => ("ceil", Arity::exact(1), "ceiling (round up)"),
        "cos" => ("cos", Arity::exact(1), "cosine"),
        "cosh" => ("cosh", Arity::exact(1), "hyperbolic cosine"),
        "double" => ("double", Arity::exact(1), "convert to floating point"),
        "exp" => ("exp", Arity::exact(1), "exponential (e^x)"),
        "floor" => ("floor", Arity::exact(1), "floor (round down)"),
        "fmod" => ("fmod", Arity::exact(2), "floating-point remainder"),
        "hypot" => ("hypot", Arity::exact(2), "hypotenuse (sqrt(x*x + y*y))"),
        "int" => ("int", Arity::exact(1), "convert to integer (truncating)"),
        "log" => ("log", Arity::exact(1), "natural logarithm"),
        "log10" => ("log10", Arity::exact(1), "base-10 logarithm"),
        "pow" => ("pow", Arity::exact(2), "exponentiation (x^y)"),
        "rand" => ("rand", Arity::exact(0), "pseudo-random number in [0, 1)"),
        "round" => ("round", Arity::exact(1), "round to nearest integer"),
        "sin" => ("sin", Arity::exact(1), "sine"),
        "sinh" => ("sinh", Arity::exact(1), "hyperbolic sine"),
        "sqrt" => ("sqrt", Arity::exact(1), "square root"),
        "srand" => ("srand", Arity::exact(1), "seed the random number generator"),
        "tan" => ("tan", Arity::exact(1), "tangent"),
        "tanh" => ("tanh", Arity::exact(1), "hyperbolic tangent"),
        "wide" => (
            "wide",
            Arity::exact(1),
            "convert to a wide (64-bit) integer",
        ),
        _ => return None,
    };
    Some(MathFuncSpec {
        name,
        since: MathFuncSince::Tcl84,
        arity,
        accepts_boolean_operand: false,
        result_class: class_of(name),
        summary,
    })
}

/// TIP 232 (8.5): the `::tcl::mathfunc` command scheme, plus `bool` /
/// `entier` / `isqrt` / `min` / `max`.
fn spec_tcl85(name: &str) -> Option<MathFuncSpec> {
    use super::operators::CommandArity as Arity;
    let (name, arity, summary) = match name {
        "bool" => ("bool", Arity::exact(1), "convert to boolean"),
        "entier" => (
            "entier",
            Arity::exact(1),
            "convert to an arbitrary-precision integer",
        ),
        "isqrt" => ("isqrt", Arity::exact(1), "integer square root"),
        "max" => ("max", Arity::at_least(1), "largest of the arguments"),
        "min" => ("min", Arity::at_least(1), "smallest of the arguments"),
        _ => return None,
    };
    Some(MathFuncSpec {
        name,
        since: MathFuncSince::Tcl85,
        arity,
        accepts_boolean_operand: name == "bool",
        result_class: class_of(name),
        summary,
    })
}

/// TIP 521 (9.0): floating-point classification.
fn spec_tcl90(name: &str) -> Option<MathFuncSpec> {
    use super::operators::CommandArity as Arity;
    let (name, summary) = match name {
        "isfinite" => (
            "isfinite",
            "true if the value is finite (not infinite or NaN)",
        ),
        "isinf" => (
            "isinf",
            "true if the value is positive or negative infinity",
        ),
        "isnan" => ("isnan", "true if the value is NaN (not a number)"),
        "isnormal" => (
            "isnormal",
            "true if the value is a normal floating-point number",
        ),
        "issubnormal" => (
            "issubnormal",
            "true if the value is a subnormal (denormal) number",
        ),
        "isunordered" => (
            "isunordered",
            "true if either argument is NaN (they cannot be ordered)",
        ),
        _ => return None,
    };
    let arity = if name == "isunordered" {
        Arity::exact(2)
    } else {
        Arity::exact(1)
    };
    Some(MathFuncSpec {
        name,
        since: MathFuncSince::Tcl90,
        arity,
        accepts_boolean_operand: false,
        result_class: class_of(name),
        summary,
    })
}

/// TIP 745 (9.1): the C99 math function batch.
fn spec_tcl91(name: &str) -> Option<MathFuncSpec> {
    use super::operators::CommandArity as Arity;
    let (name, arity, summary) = match name {
        "acosh" => ("acosh", Arity::exact(1), "inverse hyperbolic cosine"),
        "asinh" => ("asinh", Arity::exact(1), "inverse hyperbolic sine"),
        "atanh" => ("atanh", Arity::exact(1), "inverse hyperbolic tangent"),
        "cbrt" => ("cbrt", Arity::exact(1), "cube root"),
        "copysign" => (
            "copysign",
            Arity::exact(2),
            "magnitude of x with the sign of y",
        ),
        "dim" => (
            "dim",
            Arity::exact(2),
            "positive difference (max(x - y, 0))",
        ),
        "erf" => ("erf", Arity::exact(1), "error function"),
        "erfc" => ("erfc", Arity::exact(1), "complementary error function"),
        "exp2" => ("exp2", Arity::exact(1), "base-2 exponential (2^x)"),
        "expm1" => ("expm1", Arity::exact(1), "exp(x) - 1, accurate for small x"),
        "fma" => (
            "fma",
            Arity::exact(3),
            "fused multiply-add (x*y + z, one rounding)",
        ),
        "gamma" => ("gamma", Arity::exact(1), "gamma function"),
        "ldexp" => ("ldexp", Arity::exact(2), "x * 2^exp"),
        "lgamma" => (
            "lgamma",
            Arity::exact(1),
            "natural log of the absolute value of gamma(x)",
        ),
        "log1p" => ("log1p", Arity::exact(1), "log(1 + x), accurate for small x"),
        "log2" => ("log2", Arity::exact(1), "base-2 logarithm"),
        "logb" => ("logb", Arity::exact(1), "unbiased base-2 exponent"),
        "nextafter" => (
            "nextafter",
            Arity::exact(2),
            "next representable value after x, toward y",
        ),
        "remainder" => ("remainder", Arity::exact(2), "IEEE remainder of x/y"),
        "signbit" => (
            "signbit",
            Arity::exact(1),
            "true if the sign bit of x is set",
        ),
        "trunc" => ("trunc", Arity::exact(1), "truncate toward zero"),
        _ => return None,
    };
    Some(MathFuncSpec {
        name,
        since: MathFuncSince::Tcl91,
        arity,
        accepts_boolean_operand: false,
        result_class: class_of(name),
        summary,
    })
}

/// `isunordered(x, y)` — 1 if either operand is NaN (they cannot be ordered),
/// else 0 (C's `ExprIsUnorderedFunc`). Integers convert to finite doubles.
fn is_unordered<B>(vals: &[NumValue<B>]) -> Option<NumValue<B>> {
    if vals.len() != 2 {
        return None;
    }
    let nan = |v: &NumValue<B>| matches!(v, NumValue::Float(f) if f.is_nan());
    Some(NumValue::Int(i64::from(nan(&vals[0]) || nan(&vals[1]))))
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

/// The exact integer value of a finite double, truncated toward zero: an
/// `Int` when it fits a wide, otherwise the backend's arbitrary-precision
/// rung (C's `Tcl_InitBignumFromDouble`). `None` for a non-finite operand, or
/// for a backend with no bignum rung — [`Num`], the const-folder's shape,
/// which then abstains rather than folding a lossy answer.
fn trunc_exact<B: super::super::number_tower::BigIntOps>(f: f64) -> Option<NumValue<B>> {
    if let Some(i) = finite_trunc_to_i64(f) {
        return Some(NumValue::Int(i));
    }
    B::from_f64_trunc(f).map(NumValue::Big)
}

/// C's `wide()` window: the low 64 bits of an integer, two's-complement
/// folded (`wide(2**64 + 1)` is `1`). A value already on the wide rung is its
/// own window.
fn wide_window<B: super::super::number_tower::BigIntOps>(v: NumValue<B>) -> NumValue<B> {
    match v {
        NumValue::Big(b) => NumValue::Int(b.to_i64_wrapping()),
        other => other,
    }
}

/// `round()`'s exact conversion: half away from zero on the *exact* operand,
/// then the same exact truncation — tclsh's `round(1e300)` is the full
/// 301-digit integer in every release.
///
/// C's `ExprRoundFunc` splits the operand with `modf` and steps the integer
/// part by one when the fraction reaches one half in magnitude;
/// [`f64::round`] is that operation, computed exactly. It is **not**
/// `floor(d + 0.5)` / `ceil(d - 0.5)`, which rounds twice and so disagrees
/// wherever `d ± 0.5` itself rounds: `0.49999999999999994 + 0.5` is exactly
/// `1.0` in binary64 and `4503599627370497.0 + 0.5` ties to even, so that
/// spelling answered `1` and `4503599627370498` where tclsh 8.6.16/9.0.4
/// answer `0` and `4503599627370497`.
fn round_exact<B: super::super::number_tower::BigIntOps>(f: f64) -> Option<NumValue<B>> {
    if !f.is_finite() {
        return None;
    }
    trunc_exact(f.round())
}

/// The exact integer square root of a non-negative `i`, `isqrt`'s core.
///
/// Float `sqrt` is only an estimate: a near-2^62 operand rounds up in the
/// i64→f64 conversion and lands the estimate one too high vs C's exact
/// `mp_sqrt` (oracle: `isqrt(4611686018427387903)` is `2147483647`, not
/// `2^31`) — correct the float estimate to the exact integer square root.
fn exact_isqrt(i: i64) -> i64 {
    let mut r = (i as f64).sqrt() as i64;
    while r > 0 && r.checked_mul(r).is_none_or(|sq| sq > i) {
        r -= 1;
    }
    while (r + 1).checked_mul(r + 1).is_some_and(|sq| sq <= i) {
        r += 1;
    }
    r
}

fn has_nan<B>(vals: &[NumValue<B>]) -> bool {
    vals.iter()
        .any(|v| matches!(v, NumValue::Float(f) if f.is_nan()))
}

fn min_max<B: super::super::number_tower::BigIntOps>(
    name: &str,
    vals: &[NumValue<B>],
) -> Option<NumValue<B>> {
    if vals.is_empty() || has_nan(vals) {
        return None;
    }
    if vals
        .iter()
        .all(|v| matches!(v, NumValue::Int(_) | NumValue::Big(_)))
    {
        let mut best = vals[0].clone();
        for value in &vals[1..] {
            let better = match (&best, value) {
                (NumValue::Int(a), NumValue::Int(b)) => {
                    (name == "min" && b < a) || (name == "max" && b > a)
                }
                (NumValue::Big(a), NumValue::Big(b)) => {
                    (name == "min" && b < a) || (name == "max" && b > a)
                }
                (NumValue::Int(a), NumValue::Big(b)) => {
                    let a = B::from_i64(*a);
                    (name == "min" && b < &a) || (name == "max" && b > &a)
                }
                (NumValue::Big(a), NumValue::Int(b)) => {
                    let b = B::from_i64(*b);
                    (name == "min" && &b < a) || (name == "max" && &b > a)
                }
                _ => false,
            };
            if better {
                best = value.clone();
            }
        }
        Some(best)
    } else {
        // At least one argument is a `Float`, but the *winner* need not be —
        // real Tcl returns the winning argument's own value, preserving its
        // type (`expr {min(3, 5.5)}` is `3`, an int, not `3.0`): compare
        // numerically via `as_f64()`, but keep `best` as the original `Num`
        // rather than re-widening it, so an `Int` winner stays an `Int`.
        let mut best = vals[0].clone();
        for v in &vals[1..] {
            if (name == "min" && v.as_f64() < best.as_f64())
                || (name == "max" && v.as_f64() > best.as_f64())
            {
                best = v.clone();
            }
        }
        Some(best)
    }
}

fn unary_float<B: super::super::number_tower::BigIntOps>(
    name: &str,
    vals: &[NumValue<B>],
) -> Option<NumValue<B>> {
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
        Some(NumValue::Float(r))
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

fn binary_float<B: super::super::number_tower::BigIntOps>(
    name: &str,
    vals: &[NumValue<B>],
) -> Option<NumValue<B>> {
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
        Some(NumValue::Float(r))
    }
}

/// `ldexp(x, exp)` (TIP 745) — unlike every other binary math function, C's
/// `ldexp` takes a genuine `int` exponent (Tcl's `ExprBinaryDIFunc` reads it
/// with `Tcl_GetIntFromObj`, which rejects a double operand outright), so
/// this doesn't fit the `binary_float` `fn(f64, f64) -> f64` shape.
fn ldexp_fn<B: super::super::number_tower::BigIntOps>(vals: &[NumValue<B>]) -> Option<NumValue<B>> {
    if vals.len() != 2 {
        return None;
    }
    let NumValue::Int(exp) = vals[1] else {
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
        Some(NumValue::Float(r))
    }
}

/// `fma(x, y, z)` (TIP 745) — the one ternary `expr` math function, computed
/// via the portable `libm` fused multiply-add so results stay identical
/// across every backend (native, WASM, eBPF) regardless of hardware FMA
/// support.
fn fma_fn<B: super::super::number_tower::BigIntOps>(vals: &[NumValue<B>]) -> Option<NumValue<B>> {
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
        Some(NumValue::Float(r))
    }
}

/// `entier()`'s conversion — the exact, unbounded integer value of any
/// numeric operand (TIP 237). `int()`/`wide()` are this plus a window.
fn entier_of<B: super::super::number_tower::BigIntOps>(v: &NumValue<B>) -> Option<NumValue<B>> {
    match v {
        NumValue::Int(i) => Some(NumValue::Int(*i)),
        NumValue::Big(b) => Some(NumValue::Big(b.clone())),
        NumValue::Float(f) => trunc_exact(*f),
    }
}

fn type_conv<B: super::super::number_tower::BigIntOps>(
    name: &str,
    vals: &[NumValue<B>],
    int_width: IntWidth,
) -> Option<NumValue<B>> {
    if vals.len() != 1 {
        return None;
    }
    let v = &vals[0];
    match name {
        "isinf" => Some(NumValue::Int(i64::from(
            matches!(v, NumValue::Float(f) if f.is_infinite()),
        ))),
        "isnan" => Some(NumValue::Int(i64::from(
            matches!(v, NumValue::Float(f) if f.is_nan()),
        ))),
        "isfinite" => match v {
            NumValue::Int(_) | NumValue::Big(_) => Some(NumValue::Int(1)),
            NumValue::Float(f) => Some(NumValue::Int(i64::from(f.is_finite()))),
        },
        // `fpclassify`-based predicates (C's `DoubleObjIsClass`): an integer
        // operand converts to a finite double first.
        "isnormal" => Some(NumValue::Int(i64::from(
            v.as_f64().classify() == core::num::FpCategory::Normal,
        ))),
        "issubnormal" => Some(NumValue::Int(i64::from(
            v.as_f64().classify() == core::num::FpCategory::Subnormal,
        ))),
        // TIP 745: `signbit` reads the sign bit directly (matches C's
        // `signbit()`, which is well-defined on NaN and never a domain
        // error) — an integer operand's sign stands in for the bit test
        // since Tcl's own `ExprSignBitFunc` special-cases each numeric type.
        "signbit" => Some(NumValue::Int(i64::from(match v {
            NumValue::Int(i) => *i < 0,
            NumValue::Big(b) => b.is_negative(),
            NumValue::Float(f) => f.is_sign_negative(),
        }))),
        _ if matches!(v, NumValue::Float(f) if f.is_nan()) => None,
        "abs" => match v {
            // `abs(i64::MIN)` is `2^63`, past the wide rung: a backend with no
            // bignum rung declines (`None`) rather than invent a value.
            NumValue::Int(i) => i.checked_abs().map_or_else(
                || B::promote_i64(*i).map(|b| NumValue::Big(b.neg())),
                |m| Some(NumValue::Int(m)),
            ),
            NumValue::Big(b) => Some(NumValue::Big(b.abs())),
            NumValue::Float(f) => Some(NumValue::Float(f.abs())),
        },
        // TIP 237: `entier()` is unbounded in every release — a double outside
        // the wide range becomes the exact integer, not a domain error
        // (tclsh: `entier(1e300)` is all 301 digits).
        "entier" => entier_of(v),
        // `wide()` truncates then takes the low 64 bits, in every release.
        "wide" => entier_of(v).map(wide_window),
        // `int()` is the one release-split conversion: 9.0 binds it to the
        // unbounded `ExprIntFunc`, 8.4-8.6 keep the 64-bit window.
        "int" => match int_width {
            IntWidth::Unbounded => entier_of(v),
            IntWidth::Windowed => entier_of(v).map(wide_window),
            // Answer only where the releases agree; never fold a guess.
            IntWidth::Unresolved => match v {
                NumValue::Int(i) => Some(NumValue::Int(*i)),
                NumValue::Big(b) => b.to_i64().map(NumValue::Int),
                NumValue::Float(f) => finite_trunc_to_i64(*f).map(NumValue::Int),
            },
        },
        "double" => Some(NumValue::Float(v.as_f64())),
        "bool" => Some(NumValue::Int(i64::from(v.is_truthy()))),
        // `round()` is unbounded in every release too (tclsh 8.6/9.0:
        // `round(1e300)` is the full exact integer, `round(2**200)` is the
        // integer itself).
        "round" => match v {
            NumValue::Int(i) => Some(NumValue::Int(*i)),
            NumValue::Big(b) => Some(NumValue::Big(b.clone())),
            NumValue::Float(f) => round_exact(*f),
        },
        "ceil" => Some(NumValue::Float(v.as_f64().ceil())),
        "floor" => Some(NumValue::Float(v.as_f64().floor())),
        // A `Float` operand is truncated toward zero first, then the exact
        // integer square root is taken (oracle: `isqrt(9.5)` is `3`, same as
        // `isqrt(9)`; `isqrt(-1.0)` is a domain error, same as `isqrt(-1)`;
        // confirmed tclsh8.6/9.0) — real Tcl accepts a float operand here,
        // it isn't `Int`-only.
        "isqrt" => match v {
            NumValue::Int(i) if *i >= 0 => Some(NumValue::Int(exact_isqrt(*i))),
            NumValue::Big(b) => super::super::number_tower::int_sqrt(b).map(NumValue::Big),
            // A float operand truncates first, exactly like an integer one,
            // so a beyond-wide magnitude keeps its exact root rather than
            // becoming a domain error (tclsh: `isqrt(1e300)` is a 151-digit
            // integer).
            NumValue::Float(f) if *f >= 0.0 => match trunc_exact::<B>(*f)? {
                NumValue::Int(i) => Some(NumValue::Int(exact_isqrt(i))),
                NumValue::Big(b) => super::super::number_tower::int_sqrt(&b).map(NumValue::Big),
                NumValue::Float(_) => None,
            },
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::number_tower::BigIntOps;

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
    /// `0.49999999999999994` and `4503599627370498` for `2**52 + 1`. Rows
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
}
