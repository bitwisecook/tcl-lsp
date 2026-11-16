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

//! Binary operators over folded numbers: exact comparison, division, modulo, power, bitwise and the iRules string operators.

use super::TclValue;
use crate::expr_ast::BinOp;

// Binary operators

pub(super) fn apply_binary(op: BinOp, a: TclValue, b: TclValue) -> Option<TclValue> {
    match op {
        // Arithmetic.
        BinOp::Add => Some(arith(&a, &b, i64::checked_add, |x, y| x + y, |x, y| x + y)?),
        BinOp::Sub => Some(arith(&a, &b, i64::checked_sub, |x, y| x - y, |x, y| x - y)?),
        BinOp::Mul => Some(arith(&a, &b, i64::checked_mul, |x, y| x * y, |x, y| x * y)?),
        BinOp::Div => tcl_div(&a, &b),
        BinOp::Mod => tcl_mod(&a, &b),
        BinOp::Pow => tcl_pow(&a, &b),

        // Shifts and bitwise — integer only (wide or bignum; exact).
        BinOp::LShift => match (&a, &b) {
            (TclValue::Int(0), TclValue::Int(y)) if *y >= 0 => Some(TclValue::Int(0)),
            // The shift count is capped so a folded literal stays small
            // (`1 << 100000` is a real Tcl value but not one worth
            // materialising into source text — decline past the cap).
            (TclValue::Int(_) | TclValue::Big(_), TclValue::Int(y)) if (0..=256).contains(y) => {
                let x = a.to_bigint()?;
                Some(TclValue::from_big(x << u32::try_from(*y).ok()?))
            }
            _ => None,
        },
        BinOp::RShift => match (&a, &b) {
            (TclValue::Int(x), TclValue::Int(y)) if *y >= 0 => {
                // At y == 64 the value has been fully shifted out, so an
                // arithmetic right shift yields the sign bit replicated
                // (0 for non-negative, -1 for negative). Executing `x >> 64`
                // here would panic on shift-overflow in debug builds and mask
                // to `x >> 0` in release. The boundary is therefore `>= 64`,
                // not `> 64` (i64 has 64 bits).
                if *y >= 64 {
                    Some(TclValue::Int(if *x >= 0 { 0 } else { -1 }))
                } else {
                    Some(TclValue::Int(x >> y))
                }
            }
            (TclValue::Big(x), TclValue::Int(y)) if *y >= 0 => {
                use num_traits::Signed;
                // A count past the operand's width collapses to the sign.
                match usize::try_from(*y) {
                    Ok(count) if count <= x.bits() as usize => {
                        Some(TclValue::from_big(x.clone() >> count))
                    }
                    _ => Some(TclValue::Int(if x.is_negative() { -1 } else { 0 })),
                }
            }
            _ => None,
        },
        BinOp::BitAnd => bitwise(&a, &b, |x, y| x & y, |x, y| x & y),
        BinOp::BitOr => bitwise(&a, &b, |x, y| x | y, |x, y| x | y),
        BinOp::BitXor => bitwise(&a, &b, |x, y| x ^ y, |x, y| x ^ y),

        // Numeric comparison — always returns Int(0) or Int(1). A NaN operand
        // is unordered: unequal to everything, ordered against nothing.
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            use std::cmp::Ordering;
            use tcl_syntax::expr::NumericCompare;
            let outcome = numeric_cmp(a, b)?;
            let result = match (op, outcome) {
                (BinOp::Ne, NumericCompare::Unordered) => true,
                (_, NumericCompare::Unordered) => false,
                (BinOp::Eq, NumericCompare::Ordered(o)) => o == Ordering::Equal,
                (BinOp::Ne, NumericCompare::Ordered(o)) => o != Ordering::Equal,
                (BinOp::Lt, NumericCompare::Ordered(o)) => o == Ordering::Less,
                (BinOp::Le, NumericCompare::Ordered(o)) => o != Ordering::Greater,
                (BinOp::Gt, NumericCompare::Ordered(o)) => o == Ordering::Greater,
                (_, NumericCompare::Ordered(o)) => o != Ordering::Less,
            };
            Some(TclValue::Int(i64::from(result)))
        }

        // String-comparison ops (eq/ne/lt/le/gt/ge) are routed through
        // `apply_string_compare` from `eval_binary` before they reach here
        // (they need string, not numeric, operands), as are the
        // short-circuit and iRules string ops.
        BinOp::StrEq
        | BinOp::StrNe
        | BinOp::StrLt
        | BinOp::StrLe
        | BinOp::StrGt
        | BinOp::StrGe
        | BinOp::And
        | BinOp::Or
        | BinOp::WordAnd
        | BinOp::WordOr
        | BinOp::Contains
        | BinOp::StartsWith
        | BinOp::EndsWith
        | BinOp::StrEquals
        | BinOp::Matches
        | BinOp::MatchesGlob
        | BinOp::MatchesRegex
        | BinOp::In
        | BinOp::Ni => None,
    }
}

/// Integer-only bitwise operator over the wide/bignum tiers (`num-bigint`'s
/// negative-operand semantics are two's-complement, matching Tcl).
pub(super) fn bitwise<F, B>(a: &TclValue, b: &TclValue, int_op: F, big_op: B) -> Option<TclValue>
where
    F: FnOnce(i64, i64) -> i64,
    B: FnOnce(&num_bigint::BigInt, &num_bigint::BigInt) -> num_bigint::BigInt,
{
    match (a, b) {
        (TclValue::Int(x), TclValue::Int(y)) => Some(TclValue::Int(int_op(*x, *y))),
        (TclValue::Big(_) | TclValue::Int(_), TclValue::Big(_) | TclValue::Int(_)) => {
            Some(TclValue::from_big(big_op(&a.to_bigint()?, &b.to_bigint()?)))
        }
        _ => None,
    }
}

/// Whether a value's `f64` view is exact — always true for wides and floats
/// (the wide→double rounding is C's own conversion), and true for a bignum
/// only when the conversion round-trips. An inexact bignum→double declines
/// the fold rather than depend on rounding-parity with `TclBignumToDouble`.
fn big_to_f64_exact(v: &TclValue) -> bool {
    match v {
        TclValue::Big(b) => num_traits::FromPrimitive::from_f64(v.as_f64())
            .is_some_and(|back: num_bigint::BigInt| back == *b),
        TclValue::Int(_) | TclValue::Float(_) => true,
    }
}

pub(super) fn arith<F, B, G>(
    a: &TclValue,
    b: &TclValue,
    int_op: F,
    big_op: B,
    float_op: G,
) -> Option<TclValue>
where
    F: FnOnce(i64, i64) -> Option<i64>,
    B: FnOnce(&num_bigint::BigInt, &num_bigint::BigInt) -> num_bigint::BigInt,
    G: FnOnce(f64, f64) -> f64,
{
    match (a, b) {
        // Wide fast path; overflow promotes to the exact bignum tier
        // (never wraps, never declines — C Tcl's seamless promotion).
        (TclValue::Int(x), TclValue::Int(y)) => Some(match int_op(*x, *y) {
            Some(r) => TclValue::Int(r),
            None => TclValue::from_big(big_op(
                &num_bigint::BigInt::from(*x),
                &num_bigint::BigInt::from(*y),
            )),
        }),
        // Any float operand contaminates to double arithmetic — including a
        // bignum operand, with C's same double-conversion rounding. Two
        // divergence guards, pinned against tclsh:
        // - a NaN *result* is C's "domain error" (`Inf - Inf`, `Inf * 0`),
        //   and a NaN *operand* is "can't use non-numeric floating-point
        //   value" — both decline (the NaN result covers both);
        // - a bignum whose double conversion is inexact declines rather
        //   than bet on rounding parity with C's `TclBignumToDouble`.
        (TclValue::Float(_), _) | (_, TclValue::Float(_)) => {
            if !big_to_f64_exact(a) || !big_to_f64_exact(b) {
                return None;
            }
            let r = float_op(a.as_f64(), b.as_f64());
            if r.is_nan() {
                return None;
            }
            Some(TclValue::Float(r))
        }
        // At least one bignum, no float: exact arbitrary precision.
        _ => Some(TclValue::from_big(big_op(&a.to_bigint()?, &b.to_bigint()?))),
    }
}

/// Numeric comparison outcome for two folded numbers, exact across the whole
/// wide range (C Tcl's `TclCompareTwoNumbers` — a both-as-`f64` comparison
/// merges distinct wides above 2⁵³), with NaN as `Unordered` (Tcl's "`!=` is
/// true, every other comparison false" rule).
///
/// `None` declines the fold: a double equal to exactly 2⁶³ compared against a
/// wide that rounds onto it hits undefined behaviour in C Tcl's double→wide
/// cast, and real interpreters answer platform-dependently (x86-64 tclsh 8.6
/// says the wide is *greater*; the saturating ARM64 conversion says *equal*;
/// the exact answer is *less*). No fold can match every runtime, so none is
/// made.
pub(super) fn numeric_cmp(a: TclValue, b: TclValue) -> Option<tcl_syntax::expr::NumericCompare> {
    use tcl_syntax::expr::NumericCompare;
    Some(match (a, b) {
        (TclValue::Int(x), TclValue::Int(y)) => NumericCompare::Ordered(x.cmp(&y)),
        (TclValue::Float(x), TclValue::Float(y)) => NumericCompare::from_partial(x.partial_cmp(&y)),
        (TclValue::Int(x), TclValue::Float(y)) => int_vs_double(x, y)?,
        (TclValue::Float(x), TclValue::Int(y)) => match int_vs_double(y, x)? {
            NumericCompare::Ordered(ord) => NumericCompare::Ordered(ord.reverse()),
            NumericCompare::Unordered => NumericCompare::Unordered,
        },
        // Integer-vs-integer with a bignum side is exact at any width; a
        // bignum is canonical (beyond i64), so against a wide only the sign
        // matters and against a double the bignum's double view is what C
        // compares (`mp_int` → double, same rounding).
        (TclValue::Big(x), TclValue::Big(y)) => NumericCompare::Ordered(x.cmp(&y)),
        (TclValue::Big(x), TclValue::Int(y)) => {
            NumericCompare::Ordered(x.cmp(&num_bigint::BigInt::from(y)))
        }
        (TclValue::Int(x), TclValue::Big(y)) => {
            NumericCompare::Ordered(num_bigint::BigInt::from(x).cmp(&y))
        }
        (TclValue::Big(x), TclValue::Float(y)) => big_vs_double(&x, y),
        (TclValue::Float(x), TclValue::Big(y)) => match big_vs_double(&y, x) {
            NumericCompare::Ordered(o) => NumericCompare::Ordered(o.reverse()),
            NumericCompare::Unordered => NumericCompare::Unordered,
        },
    })
}

/// Exact bignum-vs-double comparison — C converts the double to a bignum
/// (`Tcl_InitBignumFromDouble`) and compares exactly, so
/// `18446744073709551617 == 1.8446744073709552e19` is FALSE even though the
/// bignum's rounded double view equals the float. NaN is unordered; ±Inf
/// orders past every integer; a finite double compares by exact integer
/// part with the fraction as tiebreak.
fn big_vs_double(x: &num_bigint::BigInt, d: f64) -> tcl_syntax::expr::NumericCompare {
    use std::cmp::Ordering;
    use tcl_syntax::expr::NumericCompare;
    if d.is_nan() {
        return NumericCompare::Unordered;
    }
    if d.is_infinite() {
        return NumericCompare::Ordered(if d > 0.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let trunc = <num_bigint::BigInt as num_traits::FromPrimitive>::from_f64(d.trunc())
        .expect("finite double truncation is an exact integer");
    match x.cmp(&trunc) {
        Ordering::Equal => {
            let frac = d.fract();
            NumericCompare::Ordered(if frac > 0.0 {
                Ordering::Less
            } else if frac < 0.0 {
                Ordering::Greater
            } else {
                Ordering::Equal
            })
        }
        other => NumericCompare::Ordered(other),
    }
}

/// Exact wide-vs-double comparison, declining (`None`) in the C-UB sliver
/// documented on [`numeric_cmp`].
fn int_vs_double(w: i64, d: f64) -> Option<tcl_syntax::expr::NumericCompare> {
    use tcl_syntax::expr::NumericCompare;
    // 2⁶³ as a double. A wide in (2⁶³−1024, 2⁶³) rounds onto it, which is the
    // window where C Tcl's `(Tcl_WideInt) d2` conversion is undefined.
    const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;
    if d == TWO_POW_63 && w as f64 == TWO_POW_63 {
        return None;
    }
    Some(
        tcl_syntax::number::compare_int_double(i128::from(w), d)
            .map_or(NumericCompare::Unordered, NumericCompare::Ordered),
    )
}

fn tcl_div(a: &TclValue, b: &TclValue) -> Option<TclValue> {
    use num_integer::Integer;
    match (a, b) {
        (TclValue::Int(_), TclValue::Int(0)) => None,
        (TclValue::Int(x), TclValue::Int(y)) => {
            // Floor division: r and y must have the same sign,
            // otherwise subtract 1 from the truncated quotient.
            // `i64::MIN / -1` overflows a wide — promote to the bignum tier
            // like every other integer overflow.
            match x.checked_div(*y) {
                Some(q) => {
                    let r = x.checked_rem(*y)?;
                    if r != 0 && (r.signum() != y.signum()) {
                        Some(TclValue::Int(q.checked_sub(1)?))
                    } else {
                        Some(TclValue::Int(q))
                    }
                }
                None => Some(TclValue::from_big(
                    num_bigint::BigInt::from(*x).div_floor(&num_bigint::BigInt::from(*y)),
                )),
            }
        }
        // Exact floor division on the bignum tier — the shared tower
        // semantics (`tcl_syntax::number_tower`).
        (TclValue::Big(_) | TclValue::Int(_), TclValue::Big(_) | TclValue::Int(_)) => {
            let (x, y) = (a.to_bigint()?, b.to_bigint()?);
            tcl_syntax::number_tower::int_div(&x, &y).map(TclValue::from_big)
        }
        _ => {
            // A non-zero numerator over a zero divisor is Inf/-Inf, a real
            // (foldable) Tcl value — only 0.0/0.0 is a domain error
            // (tclsh: `expr {5.0/0.0}` -> Inf, `expr {0.0/0.0}` errors).
            // IEEE-754 division naturally produces NaN for exactly that
            // case, so decline on NaN rather than blanket-declining
            // whenever the divisor is zero — the same pattern `tcl_pow`'s
            // float path already uses below.
            let r = a.as_f64() / b.as_f64();
            if r.is_nan() {
                return None;
            }
            Some(TclValue::Float(r))
        }
    }
}

fn tcl_mod(a: &TclValue, b: &TclValue) -> Option<TclValue> {
    match (a, b) {
        (TclValue::Int(_), TclValue::Int(0)) => None,
        (TclValue::Int(x), TclValue::Int(y)) => {
            // Sign follows divisor. `i64::MIN % -1` overflows the checked
            // rem — the true result is 0.
            // `i64::MIN % -1` overflows the checked rem — the true result is 0.
            let r = x.checked_rem(*y).unwrap_or_default();
            if r != 0 && (r.signum() != y.signum()) {
                Some(TclValue::Int(r.checked_add(*y)?))
            } else {
                Some(TclValue::Int(r))
            }
        }
        // Floor modulus on the bignum tier — the shared tower semantics.
        (TclValue::Big(_) | TclValue::Int(_), TclValue::Big(_) | TclValue::Int(_)) => {
            let (x, y) = (a.to_bigint()?, b.to_bigint()?);
            tcl_syntax::number_tower::int_mod(&x, &y).map(TclValue::from_big)
        }
        _ => None, // Tcl 9.0 rejects floats for `%`.
    }
}

fn tcl_pow(a: &TclValue, b: &TclValue) -> Option<TclValue> {
    if matches!(a, TclValue::Float(_)) || matches!(b, TclValue::Float(_)) {
        let fa = a.as_f64();
        let fb = b.as_f64();
        if fa == 0.0 && fb < 0.0 {
            return None;
        }
        if fa < 0.0 && (!fb.is_finite() || fb.fract() != 0.0) {
            return None;
        }
        let r = fa.powf(fb);
        if r.is_nan() {
            return None;
        }
        return Some(TclValue::Float(r));
    }
    // Integer path — the shared tower semantics (`INST_EXPON`'s collapses,
    // the 2^28 exponent ceiling, exact bignum powers). The exponent must be
    // a wide: a beyond-wide exponent with |base| > 1 is C's "exponent too
    // large" runtime error, and with a negative bignum exponent the result
    // collapses to 0 (base magnitude ≥ 2 by bignum canonicality).
    let y = match b {
        TclValue::Int(y) => *y,
        TclValue::Big(yy) => {
            use num_traits::{One, Signed, Zero};
            // The 0 / ±1 base collapses hold for ANY exponent magnitude and
            // precede the negative-exponent rule: `0 ** -big` is C's
            // domain error (decline the fold so it surfaces), `1 ** ±big`
            // is 1, `(-1) ** ±big` is ±1 by exponent parity. Only then
            // does |base| ≥ 2 collapse a negative exponent to 0.
            let xb = a.to_bigint()?;
            if xb.is_zero() {
                return None;
            }
            if xb.is_one() {
                return Some(TclValue::Int(1));
            }
            if (-&xb).is_one() {
                let even = (yy % 2u8).is_zero();
                return Some(TclValue::Int(if even { 1 } else { -1 }));
            }
            return if yy.is_negative() {
                Some(TclValue::Int(0))
            } else {
                None
            };
        }
        TclValue::Float(_) => return None,
    };
    let x = a.to_bigint()?;
    tcl_syntax::number_tower::int_pow(&x, y).map(TclValue::from_big)
}

// iRules string ops

/// Split a Tcl list string into its decoded element values.
///
/// A thin wrapper over the word-value owner's tolerant split
/// ([`tcl_syntax::word_rules::WordValueRules::split_list_tolerant`]):
/// membership (`in` / `ni`) compares against decoded element values, and
/// element *counts* (`llength` folds) need the full grammar so an escaped
/// separator like `a\ b` counts as one element.  Tolerant of a malformed tail
/// so folding a partial list still yields a best-effort element list rather
/// than nothing.
///
/// `rules` are the document dialect's word-value rules, so a `JimTcl`
/// document's list divides the way its own runtime divides it.
pub(crate) fn split_tcl_list(
    text: &str,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> Vec<String> {
    rules
        .split_list_tolerant(text)
        .into_iter()
        .map(std::borrow::Cow::into_owned)
        .collect()
}

/// Apply an iRules string operator to two rendered string operands.
///
/// Reached only through [`FoldOps::binary_other`], which the shared
/// [`tcl_syntax::expr::eval`] tree-walk calls for the operators its own
/// `match` does not handle.  That match already covers `equals`
/// ([`BinOp::StrEquals`], via `compare_string` alongside `eq`) and `in` /
/// `ni` (via `in_list`), so those three never arrive here — the arms that
/// re-implemented them were unreachable, and re-implementing `in`/`ni` with
/// a private list split risked disagreeing with the shared `in_list`
/// semantics if either drifted.
pub(super) fn apply_irules_string_op(
    provider: tcl_syntax::expr::operators::AuthoredF5StringPredicateProvider,
    op: BinOp,
    left: &str,
    right: &str,
) -> Option<TclValue> {
    let predicate = provider.predicate(op)?;
    Some(TclValue::Int(i64::from(predicate.evaluate(left, right))))
}
