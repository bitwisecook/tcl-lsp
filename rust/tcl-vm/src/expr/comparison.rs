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

//! Comparison operators: numeric comparison across the whole tower with the string comparison fallback.

use super::NumericContext;
use super::arithmetic::value_as_bigint;
use crate::error::TclError;
use crate::value::Value;
use num_bigint::BigInt;
use num_traits::FromPrimitive;
use std::cmp::Ordering;
use tcl_syntax::expr::{BinOp, NumericCompare};
use tcl_syntax::number;
use tcl_syntax::number::Number;

/// Exact bignum-vs-double comparison for integers past `i128`: compare integer
/// parts as bignums, then let a non-zero fraction break a tie. NaN is
/// unordered.
fn cmp_bigint_double(w: &BigInt, d: f64) -> NumericCompare {
    use NumericCompare::{Ordered, Unordered};
    if d.is_nan() {
        return Unordered;
    }
    if d == f64::INFINITY {
        return Ordered(Ordering::Less);
    }
    if d == f64::NEG_INFINITY {
        return Ordered(Ordering::Greater);
    }
    let truncated = d.trunc();
    // `from_f64` on a finite integral double is exact; `None` is unreachable
    // for the finite values that get here, but degrade to "unordered" (which
    // callers treat conservatively) rather than panic.
    let Some(int_part) = BigInt::from_f64(truncated) else {
        return Unordered;
    };
    Ordered(match w.cmp(&int_part) {
        Ordering::Equal => {
            let fraction = d - truncated;
            if fraction > 0.0 {
                Ordering::Less
            } else if fraction < 0.0 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        }
        unequal => unequal,
    })
}

/// A comparison operand classified across the full numeric tower. Distinct
/// from [`Num`] (the arithmetic coercion) because comparison must keep
/// integers past `i128` exact and must classify NaN as a *number* — both of
/// which the arithmetic path deliberately rejects.
pub(super) enum CmpNum {
    /// Any integer that fits `i128` (wides included).
    Int(i128),
    /// An integer past `i128`, exact.
    Big(BigInt),
    /// A double — possibly NaN.
    Dbl(f64),
}

/// Classify a comparison operand, or `None` for a non-numeric value.
fn cmp_num_of(v: &Value) -> Option<CmpNum> {
    if let Ok(n) = v.as_int() {
        return Some(CmpNum::Int(i128::from(n)));
    }
    if let Some(b) = v.as_i128() {
        return Some(CmpNum::Int(b));
    }
    // Typed doubles (including a typed NaN) and ordinary numeric strings.
    if let Ok(d) = v.as_double() {
        return Some(CmpNum::Dbl(d));
    }
    // `as_double` rejects integers past `i128` and the NaN spellings; both
    // are numeric for comparison purposes (Tcl's NaN rule).
    match number::parse_whole(v.try_to_str().ok()?.trim()) {
        Some(Number::Nan { .. }) => Some(CmpNum::Dbl(f64::NAN)),
        Some(Number::Big { .. }) => value_as_bigint(v).map(CmpNum::Big),
        _ => None,
    }
}

/// The one numeric-comparison path over *values*: `None` when either operand is
/// non-numeric (the caller falls back to a string comparison), otherwise the
/// exact outcome across the whole tower — wide/`i128` exactly, integer-vs-double
/// exactly, integers past `i128` as bignums, NaN unordered.
/// Shared by [`compare`] and the [`ExprEval`] adapter so the two cannot drift.
pub(super) fn compare_values_numeric(a: &Value, b: &Value) -> Option<NumericCompare> {
    use NumericCompare::{Ordered, Unordered};
    let reversed = |outcome: NumericCompare| match outcome {
        Ordered(ord) => Ordered(ord.reverse()),
        Unordered => Unordered,
    };
    Some(match (cmp_num_of(a)?, cmp_num_of(b)?) {
        (CmpNum::Int(p), CmpNum::Int(q)) => Ordered(p.cmp(&q)),
        (CmpNum::Int(p), CmpNum::Dbl(d)) => {
            NumericCompare::from_partial(number::compare_int_double(p, d))
        }
        (CmpNum::Dbl(d), CmpNum::Int(q)) => reversed(NumericCompare::from_partial(
            number::compare_int_double(q, d),
        )),
        (CmpNum::Dbl(p), CmpNum::Dbl(q)) => NumericCompare::from_partial(p.partial_cmp(&q)),
        (CmpNum::Big(p), CmpNum::Big(q)) => Ordered(p.cmp(&q)),
        (CmpNum::Big(p), CmpNum::Int(q)) => Ordered(p.cmp(&BigInt::from(q))),
        (CmpNum::Int(p), CmpNum::Big(q)) => Ordered(BigInt::from(p).cmp(&q)),
        (CmpNum::Big(p), CmpNum::Dbl(d)) => cmp_bigint_double(&p, d),
        (CmpNum::Dbl(d), CmpNum::Big(q)) => reversed(cmp_bigint_double(&q, d)),
    })
}

/// Apply a comparison operator, returning the boolean result. Numeric when both
/// operands look numeric (or always-string for the `STR_*` variants), else a
/// string comparison — Tcl's `==`/`<`… rule.
pub fn compare(op: BinOp, a: &Value, b: &Value) -> Result<bool, TclError> {
    // This standalone helper has Unicode-scalar string semantics. Native
    // execution passes its retained model through compare_in instead.
    let numeric = if string_comparison(op) {
        None
    } else {
        compare_values_numeric(a, b)
    };
    compare_with_numeric(
        op,
        a,
        b,
        numeric,
        Some(tcl_dialect::StringCharacterModel::UnicodeScalars),
    )
}

pub(super) fn string_comparison(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::StrEq | BinOp::StrNe | BinOp::StrLt | BinOp::StrLe | BinOp::StrGt | BinOp::StrGe
    )
}

#[cfg(test)]
pub(super) fn compare_string_units(
    model: Option<tcl_dialect::StringCharacterModel>,
    a: &Value,
    b: &Value,
) -> Result<Ordering, TclError> {
    compare_string_units_using(model, a, b, None)
}

pub(super) fn compare_string_units_using(
    model: Option<tcl_dialect::StringCharacterModel>,
    a: &Value,
    b: &Value,
    context: Option<NumericContext<'_>>,
) -> Result<Ordering, TclError> {
    let model =
        model.ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::CharacterModelUnavailable)?;
    let (left_bytes, right_bytes, counts) = if let Some(context) = context {
        let left = context.string_for_value(a)?;
        let right = context.string_for_value(b)?;
        context.current_for_value(a)?;
        let left_count = a.native_character_count(model, None)?;
        context.current_for_value(a)?;
        context.current_for_value(b)?;
        let right_count = b.native_character_count(model, None)?;
        context.current_for_value(b)?;
        (left, right, (left_count, right_count))
    } else {
        let counts = (
            a.native_character_count(model, None)?,
            b.native_character_count(model, None)?,
        );
        (a.string_bytes(), b.string_bytes(), counts)
    };
    let left = tcl_syntax::raw_string::RawString::from_bytes(left_bytes);
    let right = tcl_syntax::raw_string::RawString::from_bytes(right_bytes);
    left.compare_character_units(model, &right, counts)
        .map_err(Into::into)
}

fn equal_string_bytes(
    model: Option<tcl_dialect::StringCharacterModel>,
    a: &Value,
    b: &Value,
) -> Result<bool, TclError> {
    model.ok_or(tcl_syntax::raw_string::NativeValueAccessRefusal::CharacterModelUnavailable)?;
    Ok(a.string_bytes() == b.string_bytes())
}

fn compare_with_numeric(
    op: BinOp,
    a: &Value,
    b: &Value,
    numeric: Option<NumericCompare>,
    model: Option<tcl_dialect::StringCharacterModel>,
) -> Result<bool, TclError> {
    compare_with_numeric_using(op, a, b, numeric, model, None)
}

pub(super) fn compare_with_numeric_using(
    op: BinOp,
    a: &Value,
    b: &Value,
    numeric: Option<NumericCompare>,
    model: Option<tcl_dialect::StringCharacterModel>,
    context: Option<NumericContext<'_>>,
) -> Result<bool, TclError> {
    use BinOp::{Eq, Ge, Gt, Le, Lt, Ne, StrEq, StrGe, StrGt, StrLe, StrLt, StrNe};
    use NumericCompare::{Ordered, Unordered};
    if matches!(op, StrEq | StrNe) {
        let equal = if let Some(context) = context {
            model.ok_or(
                tcl_syntax::raw_string::NativeValueAccessRefusal::CharacterModelUnavailable,
            )?;
            context.string_for_value(a)? == context.string_for_value(b)?
        } else {
            equal_string_bytes(model, a, b)?
        };
        return Ok(equal ^ (op == StrNe));
    }
    let outcome = match (string_comparison(op), numeric) {
        (false, Some(numeric)) => numeric,
        _ => Ordered(compare_string_units_using(model, a, b, context)?),
    };
    Ok(match (op, outcome) {
        (Ne | StrNe, Unordered) => true,
        (_, Unordered) => false,
        (Eq | StrEq, Ordered(ord)) => ord.is_eq(),
        (Ne | StrNe, Ordered(ord)) => ord.is_ne(),
        (Lt | StrLt, Ordered(ord)) => ord.is_lt(),
        (Le | StrLe, Ordered(ord)) => ord.is_le(),
        (Gt | StrGt, Ordered(ord)) => ord.is_gt(),
        (Ge | StrGe, Ordered(ord)) => ord.is_ge(),
        _ => return Err(TclError::new("unsupported comparison operator")),
    })
}
