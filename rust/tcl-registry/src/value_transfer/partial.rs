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

//! Projections of canonical registry facts into the shared partial-value kernels.
//! These bounds are conditional value facts, never completion or native-object proof.

use tcl_cmd_core::partial_value::{self, BOTTOM, Interval, TOP};

use super::{DomainFact, ExactValue, FactView, NumericValue, ValueShape};

fn integer(value: &ExactValue) -> Interval {
    match value.numeric {
        Some(NumericValue::Int(value)) => partial_value::constant(value),
        Some(NumericValue::Bool(value)) => partial_value::constant(i64::from(value)),
        _ => TOP,
    }
}

/// Integer bounds supplied by a canonical fact, retaining unreached inputs.
#[must_use]
pub fn integer_bounds(fact: &FactView) -> Interval {
    match fact {
        FactView::Pending => BOTTOM,
        FactView::Exact(value, _) => integer(value),
        FactView::Finite(values, _) => values.iter().fold(BOTTOM, |bounds, value| {
            partial_value::join(bounds, integer(value))
        }),
        FactView::Domain(DomainFact::Range { lo, hi }) => Interval { lo: *lo, hi: *hi },
        _ => TOP,
    }
}

/// Length bounds supplied by a list shape. Exact values still pass through
/// the concrete list kernel so malformed input retains its guest error.
#[must_use]
pub fn list_length_bounds(fact: &FactView) -> Interval {
    match fact {
        FactView::Pending => BOTTOM,
        FactView::Domain(DomainFact::Type {
            shape: Some(ValueShape::ListLength(length)),
            ..
        }) => partial_value::list_length(Some(*length)),
        _ => partial_value::list_length(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_transfer::{DeclineReason, ExactValue};

    #[test]
    fn projections_preserve_pending_and_partial_list_shape() {
        assert_eq!(integer_bounds(&FactView::Pending), BOTTOM);
        assert_eq!(
            integer_bounds(&FactView::Top(DeclineReason::Unsupported)),
            TOP
        );
        let shape = FactView::Domain(DomainFact::Type {
            intrep: Some(crate::TclType::List),
            shape: Some(ValueShape::ListLength(3)),
        });
        assert_eq!(list_length_bounds(&shape), partial_value::constant(3));
        assert_eq!(
            list_length_bounds(&FactView::Top(DeclineReason::Unsupported)),
            Interval {
                lo: Some(0),
                hi: None
            }
        );
        assert_eq!(
            integer_bounds(&FactView::Finite(
                vec![ExactValue::int(-2), ExactValue::int(7)],
                None
            )),
            Interval {
                lo: Some(-2),
                hi: Some(7)
            }
        );
        assert_eq!(
            integer_bounds(&FactView::Exact(ExactValue::from_literal("08"), None)),
            TOP
        );
    }
}
