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

//! The value-representation lattice (plan §3.4) and the interval proofs that
//! let native integer arithmetic drop its overflow edge.
//!
//! A representation says what the emitter may assume about a value without
//! testing it at run time. `NativeInt` carries the closed interval the value
//! is proven to lie in, so a binary operation whose corner results all fit
//! `i64` needs no overflow check; a result the intervals cannot bound is
//! *not* native — it is a dynamic operation whose fast path is checked and
//! whose slow edge is the runtime's bignum path. Nothing here ever wraps.
//!
//! `NativeDouble` records whether the value is known finite, because IEEE
//! arithmetic on non-finite operands can produce `NaN`, and Tcl's treatment
//! of a `NaN` result is the runtime's to decide, not the emitter's.

use tcl_syntax::expr::BinOp;

use super::ir::{CmpOp, DoubleOp, IntOp};
use crate::intervals::Interval;
use crate::types::TypeShape;

/// The representation-lattice element of one NLIR value.
#[derive(Debug, Clone, PartialEq)]
pub enum Representation {
    /// A native `i64` proven to lie in the closed interval.
    NativeInt(Interval),
    /// A native `f64`; `finite` records that it is neither infinite nor `NaN`.
    NativeDouble {
        /// Whether the value is proven finite.
        finite: bool,
    },
    /// A native truth value.
    NativeBool,
    /// A boxed Tcl object, with the type shape the front end inferred for it
    /// when one is known. The shape is a hint for which native fast path to
    /// try first; it is never a proof.
    Boxed(Option<TypeShape>),
    /// Nothing is known.
    Unknown,
}

impl Representation {
    /// The exact-constant interval representation.
    #[must_use]
    pub const fn exact_int(value: i64) -> Self {
        Self::NativeInt(Interval {
            lo: Some(value),
            hi: Some(value),
        })
    }

    /// A native integer with no proven bound.
    #[must_use]
    pub const fn any_int() -> Self {
        Self::NativeInt(Interval { lo: None, hi: None })
    }

    /// Stable Explorer spelling of the lattice element's kind.
    #[must_use]
    pub const fn kind_str(&self) -> &'static str {
        match self {
            Self::NativeInt(_) => "native-int",
            Self::NativeDouble { .. } => "native-double",
            Self::NativeBool => "native-bool",
            Self::Boxed(_) => "boxed",
            Self::Unknown => "unknown",
        }
    }

    /// The interval when this is a native integer.
    #[must_use]
    pub const fn interval(&self) -> Option<Interval> {
        match self {
            Self::NativeInt(interval) => Some(*interval),
            _ => None,
        }
    }

    /// Whether the value is a native number (integer or double).
    #[must_use]
    pub const fn is_native_numeric(&self) -> bool {
        matches!(self, Self::NativeInt(_) | Self::NativeDouble { .. })
    }
}

/// The native operator a Tcl binary operator maps to on `i64` operands, when
/// it has one.
#[must_use]
pub const fn int_op(op: BinOp) -> Option<IntOp> {
    match op {
        BinOp::Add => Some(IntOp::Add),
        BinOp::Sub => Some(IntOp::Sub),
        BinOp::Mul => Some(IntOp::Mul),
        BinOp::Div => Some(IntOp::Div),
        BinOp::Mod => Some(IntOp::Mod),
        BinOp::BitAnd => Some(IntOp::And),
        BinOp::BitOr => Some(IntOp::Or),
        BinOp::BitXor => Some(IntOp::Xor),
        BinOp::LShift => Some(IntOp::Shl),
        BinOp::RShift => Some(IntOp::Shr),
        _ => None,
    }
}

/// The native operator a Tcl binary operator maps to on `f64` operands.
#[must_use]
pub const fn double_op(op: BinOp) -> Option<DoubleOp> {
    match op {
        BinOp::Add => Some(DoubleOp::Add),
        BinOp::Sub => Some(DoubleOp::Sub),
        BinOp::Mul => Some(DoubleOp::Mul),
        BinOp::Div => Some(DoubleOp::Div),
        _ => None,
    }
}

/// Whether a native `f64` operation on two proven-finite operands cannot
/// produce `NaN`.
///
/// C Tcl raises `ARITH DOMAIN` ("domain error: argument not in valid range")
/// whenever a double operation yields `NaN`, while an infinite result is an
/// ordinary value (`1.0/0.0` is `Inf`, `0.0/0.0` errors). Finite operands can
/// only reach `NaN` through `0.0/0.0`, and the double lattice carries no value
/// interval to prove a divisor non-zero, so division takes the runtime
/// operator — the same conservative edge `proven_int_result` gives integer
/// division by an unproven divisor.
#[must_use]
pub const fn double_result_defined(op: DoubleOp) -> bool {
    match op {
        DoubleOp::Add | DoubleOp::Sub | DoubleOp::Mul => true,
        DoubleOp::Div => false,
    }
}

/// Whether every integer in `interval` converts to `f64` without rounding.
///
/// `|v| <= 2^53` is the exact-integer range of a double. Outside it the
/// conversion is lossy, and Tcl compares an integer with a double *exactly*
/// (`tclExecute.c` widens to the shared comparator rather than to `f64`):
/// `9007199254740993 == 9007199254740992.0` is false on 8.6.16 and 9.0.4,
/// and the integer compares *greater*. Promoting both sides to `f64` would
/// answer true, so a mixed comparison stays native only inside this range.
#[must_use]
pub const fn exactly_representable_as_double(interval: Interval) -> bool {
    const LIMIT: i64 = 1 << 53;
    match (interval.lo, interval.hi) {
        (Some(lo), Some(hi)) => lo >= -LIMIT && hi <= LIMIT,
        _ => false,
    }
}

/// The native comparison a Tcl numeric comparison maps to.
#[must_use]
pub const fn cmp_op(op: BinOp) -> Option<CmpOp> {
    match op {
        BinOp::Eq => Some(CmpOp::Eq),
        BinOp::Ne => Some(CmpOp::Ne),
        BinOp::Lt => Some(CmpOp::Lt),
        BinOp::Le => Some(CmpOp::Le),
        BinOp::Gt => Some(CmpOp::Gt),
        BinOp::Ge => Some(CmpOp::Ge),
        _ => None,
    }
}

/// Whether both bounds of an interval are finite.
#[must_use]
pub const fn bounded(interval: Interval) -> Option<(i64, i64)> {
    match (interval.lo, interval.hi) {
        (Some(lo), Some(hi)) if lo <= hi => Some((lo, hi)),
        _ => None,
    }
}

/// Whether `interval` cannot contain `value`.
#[must_use]
pub fn excludes(interval: Interval, value: i64) -> bool {
    match bounded(interval) {
        Some((lo, hi)) => value < lo || value > hi,
        None => match (interval.lo, interval.hi) {
            (Some(lo), None) => value < lo,
            (None, Some(hi)) => value > hi,
            _ => false,
        },
    }
}

/// Evaluate `f` at the four corners of two bounded intervals and take the
/// hull, or `None` when any corner leaves `i64`.
fn corners(lhs: Interval, rhs: Interval, f: impl Fn(i64, i64) -> Option<i64>) -> Option<Interval> {
    let (llo, lhi) = bounded(lhs)?;
    let (rlo, rhi) = bounded(rhs)?;
    let values = [f(llo, rlo)?, f(llo, rhi)?, f(lhi, rlo)?, f(lhi, rhi)?];
    let lo = values.iter().copied().min()?;
    let hi = values.iter().copied().max()?;
    Some(Interval {
        lo: Some(lo),
        hi: Some(hi),
    })
}

/// Tcl integer division: rounds toward negative infinity.
#[must_use]
pub fn floor_div(lhs: i64, rhs: i64) -> Option<i64> {
    if rhs == 0 {
        return None;
    }
    let quotient = lhs.checked_div(rhs)?;
    let remainder = lhs.checked_rem(rhs)?;
    if remainder != 0 && ((remainder < 0) != (rhs < 0)) {
        quotient.checked_sub(1)
    } else {
        Some(quotient)
    }
}

/// Tcl integer modulo: the remainder takes the divisor's sign.
#[must_use]
pub fn floor_mod(lhs: i64, rhs: i64) -> Option<i64> {
    if rhs == 0 {
        return None;
    }
    let remainder = lhs.checked_rem(rhs)?;
    if remainder != 0 && ((remainder < 0) != (rhs < 0)) {
        remainder.checked_add(rhs)
    } else {
        Some(remainder)
    }
}

/// The result interval of a native integer operation whose every possible
/// result provably fits `i64` and whose every Tcl precondition (non-zero
/// divisor, in-range shift count) provably holds; `None` when the operation
/// needs a runtime check.
#[must_use]
pub fn proven_int_result(op: IntOp, lhs: Interval, rhs: Interval) -> Option<Interval> {
    match op {
        IntOp::Add => corners(lhs, rhs, i64::checked_add),
        IntOp::Sub => corners(lhs, rhs, i64::checked_sub),
        IntOp::Mul => corners(lhs, rhs, i64::checked_mul),
        IntOp::Div => {
            if !excludes(rhs, 0) {
                return None;
            }
            corners(lhs, rhs, floor_div)
        }
        IntOp::Mod => {
            let (rlo, rhi) = bounded(rhs)?;
            if !excludes(rhs, 0) {
                return None;
            }
            // The remainder has the divisor's sign and a magnitude below it.
            if rlo > 0 {
                Some(Interval {
                    lo: Some(0),
                    hi: Some(rhi.checked_sub(1)?),
                })
            } else {
                Some(Interval {
                    lo: Some(rlo.checked_add(1)?),
                    hi: Some(0),
                })
            }
        }
        IntOp::And | IntOp::Or | IntOp::Xor => {
            // Bitwise operations never leave `i64`; the exact hull would need
            // bit reasoning, so the result is simply an unbounded native int.
            bounded(lhs)?;
            bounded(rhs)?;
            Some(Interval { lo: None, hi: None })
        }
        IntOp::Shl => {
            let (rlo, rhi) = bounded(rhs)?;
            if rlo < 0 || rhi > 62 {
                return None;
            }
            corners(lhs, rhs, |value, count| {
                let shifted = value.checked_shl(u32::try_from(count).ok()?)?;
                // A left shift is exact only when shifting back recovers it.
                (shifted >> count == value).then_some(shifted)
            })
        }
        IntOp::Shr => {
            let (rlo, rhi) = bounded(rhs)?;
            if rlo < 0 || rhi > 63 {
                return None;
            }
            corners(lhs, rhs, |value, count| {
                value.checked_shr(u32::try_from(count).ok()?)
            })
        }
    }
}

/// The result interval of negating a native integer, when it fits.
#[must_use]
pub fn proven_neg_result(src: Interval) -> Option<Interval> {
    let (lo, hi) = bounded(src)?;
    Some(Interval {
        lo: Some(hi.checked_neg()?),
        hi: Some(lo.checked_neg()?),
    })
}

/// Which native fast path a dynamic operation over these representations
/// should try first.
#[must_use]
pub fn numeric_hint(lhs: &Representation, rhs: &Representation) -> super::ir::NumericHint {
    use super::ir::NumericHint;
    let doubleish = |rep: &Representation| {
        matches!(
            rep,
            Representation::NativeDouble { .. } | Representation::Boxed(Some(TypeShape::Double))
        )
    };
    if doubleish(lhs) || doubleish(rhs) {
        NumericHint::Double
    } else {
        NumericHint::Int
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iv(lo: i64, hi: i64) -> Interval {
        Interval {
            lo: Some(lo),
            hi: Some(hi),
        }
    }

    #[test]
    fn exact_constants_prove_their_arithmetic() {
        assert_eq!(
            proven_int_result(IntOp::Mul, iv(10, 10), iv(3, 3)),
            Some(iv(30, 30))
        );
        assert_eq!(
            proven_int_result(IntOp::Add, iv(30, 30), iv(7, 7)),
            Some(iv(37, 37))
        );
        assert_eq!(
            proven_int_result(IntOp::Div, iv(-7, -7), iv(2, 2)),
            Some(iv(-4, -4)),
            "Tcl division rounds toward negative infinity"
        );
        assert_eq!(
            proven_int_result(IntOp::Mod, iv(-7, 7), iv(5, 5)),
            Some(iv(0, 4))
        );
        assert_eq!(
            proven_int_result(IntOp::Mod, iv(7, 7), iv(-5, -5)),
            Some(iv(-4, 0))
        );
    }

    #[test]
    fn unprovable_results_keep_their_runtime_check() {
        assert_eq!(
            proven_int_result(IntOp::Add, Interval { lo: None, hi: None }, iv(1, 1)),
            None
        );
        assert_eq!(
            proven_int_result(IntOp::Mul, iv(i64::MAX, i64::MAX), iv(2, 2)),
            None
        );
        assert_eq!(
            proven_int_result(IntOp::Div, iv(1, 1), iv(-1, 1)),
            None,
            "a divisor interval containing zero is not proven"
        );
        assert_eq!(
            proven_int_result(IntOp::Div, iv(i64::MIN, i64::MIN), iv(-1, -1)),
            None
        );
        assert_eq!(proven_int_result(IntOp::Shl, iv(1, 1), iv(63, 63)), None);
        assert_eq!(
            proven_int_result(IntOp::Shl, iv(12, 12), iv(2, 2)),
            Some(iv(48, 48))
        );
        assert_eq!(
            proven_int_result(IntOp::Shr, iv(12, 12), iv(1, 1)),
            Some(iv(6, 6))
        );
        assert_eq!(proven_neg_result(iv(i64::MIN, 0)), None);
        assert_eq!(proven_neg_result(iv(-3, 5)), Some(iv(-5, 3)));
    }

    #[test]
    fn literal_texts_round_trip_through_their_native_constants() {
        let numbers = tcl_syntax::number::Numbers::of_dialect_name(Some("tcl9.0"));
        assert_eq!(
            numbers.parse_whole("2.5"),
            Some(tcl_syntax::number::Number::Double(2.5))
        );
        assert_eq!(tcl_syntax::number::format_double(2.5), "2.5");
        assert_eq!(
            numbers.parse_whole("10"),
            Some(tcl_syntax::number::Number::Int(10))
        );
    }

    #[test]
    fn floor_semantics_match_tcl() {
        assert_eq!(floor_div(7, 2), Some(3));
        assert_eq!(floor_div(-7, 2), Some(-4));
        assert_eq!(floor_div(7, -2), Some(-4));
        assert_eq!(floor_mod(-7, 2), Some(1));
        assert_eq!(floor_mod(7, -2), Some(-1));
        assert_eq!(floor_mod(6, 3), Some(0));
        assert_eq!(floor_div(1, 0), None);
        assert_eq!(floor_div(i64::MIN, -1), None);
    }
}
