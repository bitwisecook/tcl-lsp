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
    fn from_i64(_: i64) -> Self {
        unreachable!()
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
    /// A backend with no arbitrary-precision rung cannot represent a double
    /// outside the wide range, so it declines rather than panicking in the
    /// default implementation's `from_i64`. Callers (the const-folder) read
    /// the `None` as "abstain", which is exactly right: the value is a real
    /// bignum they have no way to carry.
    fn from_f64_trunc(_: f64) -> Option<Self> {
        None
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

/// Static facts about one `expr` math function — the string-keyed
/// counterpart to `operators::OperatorSpec` (math functions are open and
/// overridable via `::tcl::mathfunc::*`, TIP 232, so there's no closed enum
/// to attach metadata to). This is the fact table `mathfunc_generated.rs`
/// (layer 2) reads for hover/completion; it carries no behavior of its own.
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
    /// A one-line human summary for hover text.
    pub summary: &'static str,
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

/// `round()`'s exact conversion: half away from zero (C's
/// `floor(d + 0.5)` / `ceil(d - 0.5)`), then the same exact truncation —
/// tclsh's `round(1e300)` is the full 301-digit integer in every release.
fn round_exact<B: super::super::number_tower::BigIntOps>(f: f64) -> Option<NumValue<B>> {
    if !f.is_finite() {
        return None;
    }
    let rounded = if f >= 0.0 {
        (f + 0.5).floor()
    } else {
        (f - 0.5).ceil()
    };
    trunc_exact(rounded)
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
            NumValue::Int(i) => i.checked_abs().map_or_else(
                || Some(NumValue::Big(B::from_i64(*i).neg())),
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
        // Adversarial-review finding: a mixed-type call whose *winner* is
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
        // Adversarial-review finding: `isqrt` only matched `Num::Int`, so a
        // `Num::Float` operand fell to the catch-all `_ => None` — treated
        // as a domain error even though real Tcl accepts a float here,
        // truncating it toward zero first (confirmed tclsh8.6/9.0):
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

    /// #1382 — with a real arbitrary-precision backend, `entier`/`round`
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

    /// #1382 — `int()` is the one release-split conversion. Measured:
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
                &[two_64_1.clone()],
                IntWidth::Unbounded
            ),
            Some(two_64_1.clone())
        );
        assert_eq!(
            dispatch_with_backend_int_width::<BigInt>(
                "int",
                &[two_64_1.clone()],
                IntWidth::Windowed
            ),
            Some(N::Int(1))
        );
        assert_eq!(
            dispatch_with_backend_int_width::<BigInt>("int", &[two_64_1], IntWidth::Unresolved),
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
}
