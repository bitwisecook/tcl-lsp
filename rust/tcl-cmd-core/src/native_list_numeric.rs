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

//! Shared original numeric pattern/key owner, with an explicit independent neutral parser.
use crate::{
    CmdError,
    index::{self, ListNumber, ListNumericKind, NativeIndexOps},
    native_option_index::OptionCommand,
    sort,
};
use core::cmp::Ordering;

/// Returned number; neutral integer parsing retains its own existing i128 extent.
#[derive(Debug, Clone, Copy)]
pub enum NumericKey {
    Integer(i128),
    Real(f64),
}

/// Read the same original header through its selected public command primitive.
/// No String getter precedes a real primitive, including an already-typed value.
///
/// # Errors
/// Original Guest/full Host cause, or the explicit neutral lexical diagnostic.
pub fn number<O: NativeIndexOps>(
    ops: &mut O,
    original: &O::Value,
    command: OptionCommand,
    kind: ListNumericKind,
) -> Result<NumericKey, CmdError> {
    if let Some(value) = index::original_list_number(ops, original, command, kind)? {
        return Ok(match value {
            ListNumber::Integer(v) => NumericKey::Integer(i128::from(v)),
            ListNumber::Real(v) => NumericKey::Real(v),
        });
    }
    let bytes = index::checked_operand_bytes(ops, original)?;
    let parsed = match kind {
        ListNumericKind::Integer => sort::parse_wide(&bytes).map(NumericKey::Integer),
        ListNumericKind::Real => sort::parse_real(&bytes).map(NumericKey::Real),
    };
    parsed.ok_or_else(|| {
        let mut message = if kind == ListNumericKind::Integer {
            b"expected integer but got \"".to_vec()
        } else {
            b"expected floating-point number but got \"".to_vec()
        };
        message.extend_from_slice(&bytes);
        message.push(b'"');
        CmdError::new_bytes(message)
    })
}

/// Actual search compares equality first, then less, then greater. Unordered
/// accepted C84 Doubles do not become false Equal through `partial_cmp`.
#[must_use]
pub fn search_order(left: NumericKey, right: NumericKey) -> Ordering {
    match (left, right) {
        (NumericKey::Integer(a), NumericKey::Integer(b)) => a.cmp(&b),
        (NumericKey::Real(a), NumericKey::Real(b)) => {
            a.partial_cmp(&b).unwrap_or(Ordering::Greater)
        }
        _ => unreachable!("one selected search primitive"),
    }
}

/// Source-selected numeric comparator. Jim's signed subtraction is supported
/// only where it is defined; overflow does not acquire a guessed wrapping ABI.
///
/// # Errors
/// Unavailable Jim subtraction extent instead of a heuristic native outcome.
pub fn sort_order(left: NumericKey, right: NumericKey, jim: bool) -> Result<Ordering, CmdError> {
    Ok(match (left, right) {
        (NumericKey::Integer(a), NumericKey::Integer(b)) if jim => {
            let a = i64::try_from(a).map_err(|_| unavailable())?;
            let b = i64::try_from(b).map_err(|_| unavailable())?;
            a.checked_sub(b).ok_or_else(unavailable)?.cmp(&0)
        }
        (NumericKey::Integer(a), NumericKey::Integer(b)) => a.cmp(&b),
        (NumericKey::Real(a), NumericKey::Real(b)) if jim => {
            a.partial_cmp(&b).unwrap_or(Ordering::Less)
        }
        (NumericKey::Real(a), NumericKey::Real(b)) => {
            if a > b {
                Ordering::Greater
            } else if b > a {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        }
        _ => unreachable!("one selected sort primitive"),
    })
}
fn unavailable() -> CmdError {
    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
        "defined original Jim signed sort subtraction",
    )
    .into()
}

#[cfg(test)]
mod bounded_comparison_controls {
    use super::*;
    #[test]
    fn source_selected_unordered_and_defined_subtraction_boundaries_are_explicit() {
        // NaN is independently measured in645/647 finite public results; signed
        // extreme subtraction is a source/definedness boundary, not a measured
        // native extreme outcome or an invented wrapping comparator ABI.
        assert_eq!(
            search_order(NumericKey::Real(f64::NAN), NumericKey::Real(1.0)),
            Ordering::Greater
        );
        assert_eq!(
            sort_order(NumericKey::Real(f64::NAN), NumericKey::Real(1.0), false).unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            sort_order(NumericKey::Real(f64::NAN), NumericKey::Real(1.0), true).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            sort_order(
                NumericKey::Integer(i128::from(i64::MAX)),
                NumericKey::Integer(0),
                true
            )
            .unwrap(),
            Ordering::Greater
        );
        assert!(
            sort_order(
                NumericKey::Integer(i128::from(i64::MAX)),
                NumericKey::Integer(-1),
                true
            )
            .is_err()
        );
        assert!(
            sort_order(
                NumericKey::Integer(i128::from(i64::MIN)),
                NumericKey::Integer(1),
                true
            )
            .is_err()
        );
        assert_eq!(
            sort_order(
                NumericKey::Integer(i128::from(i64::MIN)),
                NumericKey::Integer(i128::from(i64::MAX)),
                false
            )
            .unwrap(),
            Ordering::Less
        );
    }
}
