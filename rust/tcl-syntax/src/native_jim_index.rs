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

//! Jim084's integer/index encoding and original expression operand selection.
//! The caller evaluates selected original objects through its actual safe
//! `GetWideExpr` owner; this module does not parse or evaluate expressions.
use crate::value::ValueError;

/// Cached Jim index integer. This is distinct from C's static option Index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JimIndex(pub i32);
impl JimIndex {
    /// Encode a reached wide expression using `SetIndexFromAny`'s native rules.
    pub fn from_expression(value: i64, end_relative: bool) -> Result<Self, ValueError> {
        let encoded = if end_relative {
            if value > 0 {
                i64::from(i32::MAX)
            } else {
                value.wrapping_sub(1)
            }
        } else if value < 0 {
            -i64::from(i32::MAX)
        } else {
            value
        };
        if !end_relative && encoded > i64::from(i32::MAX) {
            return Err(ValueError::NotIntegerBytes(Vec::new()));
        }
        Ok(Self(crate::number::native_int32_low_bits(encoded)))
    }
    /// `Jim_GetIndex`'s existing Int primary bypass clamps without shimmering.
    #[must_use]
    pub fn from_integer(value: i64) -> i32 {
        if value < 0 {
            -i32::MAX
        } else {
            i32::try_from(value).unwrap_or(i32::MAX)
        }
    }
    /// Native updater text; no expression or name is reconstructed.
    #[must_use]
    pub fn string_bytes(self) -> Vec<u8> {
        if self.0 == -1 {
            b"end".to_vec()
        } else if self.0 >= 0 || self.0 == -i32::MAX {
            self.0.to_string().into_bytes()
        } else {
            format!("end{}", self.0 + 1).into_bytes()
        }
    }
}
/// The exact operand selected by `SetIndexFromAny` for safe `GetWideExpr`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JimIndexExpression<'a> {
    /// Evaluate the SAME original index object, preserving its counted source.
    Original,
    /// Evaluate a fresh counted String containing this original `CString` suffix.
    EndSuffix(&'a [u8]),
}

/// Index geometry and actual expression completion retain separate ownership.
#[derive(Debug)]
pub enum JimIndexEvaluationError<E> {
    Index(ValueError),
    Expression(E),
}

/// Select the original safe-expression operand and encode its actual wide value.
/// `end` bypasses evaluation; ordinary expressions use the SAME original object,
/// while `end+...` and `end-...` use a genuine fresh suffix object.
pub fn evaluate_index<E>(
    original: &[u8],
    mut evaluate: impl FnMut(JimIndexExpression<'_>) -> Result<i64, E>,
) -> Result<JimIndex, JimIndexEvaluationError<E>> {
    let cstring = tcl_core_types::c_string_extent(original);
    let (expression, relative) = if let Some(rest) = cstring.strip_prefix(b"end") {
        if rest.is_empty() {
            return Ok(JimIndex(-1));
        }
        if !matches!(rest.first(), Some(b'+' | b'-')) {
            return Err(JimIndexEvaluationError::Index(ValueError::NotIntegerBytes(
                original.to_vec(),
            )));
        }
        (JimIndexExpression::EndSuffix(rest), true)
    } else {
        (JimIndexExpression::Original, false)
    };
    let value = evaluate(expression).map_err(JimIndexEvaluationError::Expression)?;
    JimIndex::from_expression(value, relative).map_err(JimIndexEvaluationError::Index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_index_expression_selection_preserves_counted_and_cstring_sources() {
        let mut visited = 0;
        assert_eq!(
            evaluate_index(b"2*3\0tail", |input| {
                assert_eq!(input, JimIndexExpression::Original);
                visited += 1;
                Ok::<_, ()>(6)
            })
            .unwrap(),
            JimIndex(6)
        );
        assert_eq!(visited, 1);
        assert_eq!(
            evaluate_index(b"end-2*3\0tail", |input| {
                assert_eq!(input, JimIndexExpression::EndSuffix(b"-2*3"));
                Ok::<_, ()>(-6)
            })
            .unwrap(),
            JimIndex(-7)
        );
        assert_eq!(
            evaluate_index(b"end\0tail", |_| -> Result<i64, ()> {
                panic!("native end does not enter an expression")
            })
            .unwrap(),
            JimIndex(-1)
        );
        assert!(matches!(
            evaluate_index(b"endx", |_| -> Result<i64, ()> {
                panic!("invalid original suffix does not enter an expression")
            }),
            Err(JimIndexEvaluationError::Index(_))
        ));
        assert!(matches!(
            evaluate_index(b"[callback]", |_| Err::<i64, _>("actual refusal")),
            Err(JimIndexEvaluationError::Expression("actual refusal"))
        ));
    }
}
