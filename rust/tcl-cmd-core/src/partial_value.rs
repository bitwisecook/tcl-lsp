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

//! Partial integer and correlated finite-value operations shared by analysis domains.
//! Unknown bounds and unreached inputs remain distinct; exact command evaluation
//! retains its own value representation and completion evidence.

/// A bound is an `i64`, or `None` for an infinity (sign given by position).
pub type Bound = Option<i64>;

/// A closed integer interval `[lo, hi]`; `None` bound = ±infinity.
///
/// `lo is None` → -inf, `hi is None` → +inf.  The empty/unreached value is
/// `BOTTOM` (lo > hi sentinel, detected via [`Interval::is_bottom`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    /// Lower bound (`None` = -inf).
    pub lo: Bound,
    /// Upper bound (`None` = +inf).
    pub hi: Bound,
}

/// The unbounded interval `[-inf, +inf]`.
pub const TOP: Interval = Interval { lo: None, hi: None };
/// The canonical empty interval (`lo > hi`).
pub const BOTTOM: Interval = Interval {
    lo: Some(1),
    hi: Some(0),
};

/// The single-point interval `[n, n]`.
#[must_use]
pub fn constant(n: i64) -> Interval {
    Interval {
        lo: Some(n),
        hi: Some(n),
    }
}

impl Interval {
    /// True when both bounds are infinite.
    #[must_use]
    pub fn is_top(self) -> bool {
        self.lo.is_none() && self.hi.is_none()
    }

    /// True when the interval is the empty `lo > hi` sentinel.
    #[must_use]
    pub fn is_bottom(self) -> bool {
        matches!((self.lo, self.hi), (Some(l), Some(h)) if l > h)
    }
}

/// Least upper bound: the smallest interval containing both.
#[must_use]
pub fn join(a: Interval, b: Interval) -> Interval {
    if a.is_bottom() {
        return b;
    }
    if b.is_bottom() {
        return a;
    }
    // lo = min(lo) treating None as -inf; hi = max(hi) treating None as +inf.
    let lo = match (a.lo, b.lo) {
        (Some(x), Some(y)) => Some(x.min(y)),
        _ => None,
    };
    let hi = match (a.hi, b.hi) {
        (Some(x), Some(y)) => Some(x.max(y)),
        _ => None,
    };
    Interval { lo, hi }
}

/// Standard interval widening: any bound that moved outward jumps to ±inf.
/// Guarantees termination of the loop fixpoint.
#[must_use]
pub fn widen(old: Interval, new: Interval) -> Interval {
    if old.is_bottom() {
        return new;
    }
    if new.is_bottom() {
        return old;
    }
    let lo = match (old.lo, new.lo) {
        (Some(o), Some(n)) if n >= o => Some(o),
        _ => None,
    };
    let hi = match (old.hi, new.hi) {
        (Some(o), Some(n)) if n <= o => Some(o),
        _ => None,
    };
    Interval { lo, hi }
}

/// Sound integer addition; overflow leaves the corresponding bound unknown.
#[must_use]
pub fn add(a: Interval, b: Interval) -> Interval {
    if a.is_bottom() || b.is_bottom() {
        return BOTTOM;
    }
    let lo = match (a.lo, b.lo) {
        (Some(x), Some(y)) => x.checked_add(y),
        _ => None,
    };
    let hi = match (a.hi, b.hi) {
        (Some(x), Some(y)) => x.checked_add(y),
        _ => None,
    };
    Interval { lo, hi }
}

/// Sound integer subtraction; overflow leaves the corresponding bound unknown.
#[must_use]
pub fn sub(a: Interval, b: Interval) -> Interval {
    if a.is_bottom() || b.is_bottom() {
        return BOTTOM;
    }
    // a - b = [a.lo - b.hi, a.hi - b.lo]
    let lo = match (a.lo, b.hi) {
        (Some(x), Some(y)) => x.checked_sub(y),
        _ => None,
    };
    let hi = match (a.hi, b.lo) {
        (Some(x), Some(y)) => x.checked_sub(y),
        _ => None,
    };
    Interval { lo, hi }
}

/// Bounded integer multiplication; overflow or unbounded inputs give top.
#[must_use]
pub fn mul(a: Interval, b: Interval) -> Interval {
    if a.is_bottom() || b.is_bottom() {
        return BOTTOM;
    }
    // Only attempt when fully bounded; otherwise TOP (sound).
    let (Some(alo), Some(ahi), Some(blo), Some(bhi)) = (a.lo, a.hi, b.lo, b.hi) else {
        return TOP;
    };
    let corners = [
        alo.checked_mul(blo),
        alo.checked_mul(bhi),
        ahi.checked_mul(blo),
        ahi.checked_mul(bhi),
    ];
    // Any overflow → fall back to TOP (sound).
    if corners.iter().any(Option::is_none) {
        return TOP;
    }
    let vals: Vec<i64> = corners.into_iter().flatten().collect();
    Interval {
        lo: vals.iter().copied().min(),
        hi: vals.iter().copied().max(),
    }
}

/// Integer negation, including unknown bounds when the wide range overflows.
#[must_use]
pub fn negate(a: Interval) -> Interval {
    if a.is_bottom() {
        return BOTTOM;
    }
    Interval {
        lo: a.hi.and_then(i64::checked_neg),
        hi: a.lo.and_then(i64::checked_neg),
    }
}

/// Greatest lower bound: the largest interval contained in both.  Here `None`
/// is the *identity* (-inf for a lower bound, +inf for an upper), the opposite
/// of [`join`] where it is absorbing.
#[must_use]
pub fn intersect(a: Interval, b: Interval) -> Interval {
    if a.is_bottom() || b.is_bottom() {
        return BOTTOM;
    }
    let lo = match (a.lo, b.lo) {
        (None, x) | (x, None) => x,
        (Some(x), Some(y)) => Some(x.max(y)),
    };
    let hi = match (a.hi, b.hi) {
        (None, x) | (x, None) => x,
        (Some(x), Some(y)) => Some(x.min(y)),
    };
    Interval { lo, hi }
}

/// Interpret an integer binary operation over partial bounds.
/// Unsupported operations keep no range information.
#[must_use]
pub fn binary(op: tcl_syntax::expr::ast::BinOp, left: Interval, right: Interval) -> Interval {
    use tcl_syntax::expr::ast::BinOp;
    if left.is_bottom() || right.is_bottom() {
        return BOTTOM;
    }
    match op {
        BinOp::Add => add(left, right),
        BinOp::Sub => sub(left, right),
        BinOp::Mul => mul(left, right),
        BinOp::Eq
        | BinOp::Ne
        | BinOp::Lt
        | BinOp::Le
        | BinOp::Gt
        | BinOp::Ge
        | BinOp::And
        | BinOp::Or => Interval {
            lo: Some(0),
            hi: Some(1),
        },
        _ => TOP,
    }
}

/// Interpret integer unary operations without coercing an unknown value.
#[must_use]
pub fn unary(op: tcl_syntax::expr::ast::UnaryOp, value: Interval) -> Interval {
    use tcl_syntax::expr::ast::UnaryOp;
    if value.is_bottom() {
        return BOTTOM;
    }
    match op {
        UnaryOp::Neg => negate(value),
        UnaryOp::Pos => value,
        _ => TOP,
    }
}

/// Conditional bounds of a successful list length, without requiring its elements.
#[must_use]
pub fn list_length(length: Option<usize>) -> Interval {
    length.and_then(|length| i64::try_from(length).ok()).map_or(
        Interval {
            lo: Some(0),
            hi: None,
        },
        constant,
    )
}

/// An input restriction of bounded finite evaluation, before any evaluator runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiniteLimit {
    /// More than one independently varying finite input was supplied.
    IndependentInputs,
    /// A finite input has no identity with which to correlate repeated uses.
    MissingIdentity,
    /// The one admitted set exceeds the work bound.
    TooManyMembers,
}

/// Select the one correlated finite input, if any. Repeated uses of an
/// identity share a member; independent inputs are never multiplied together.
///
/// # Errors
/// Returns the finite-evaluation restriction which prevents enumeration.
pub fn finite_input<I: Copy + PartialEq, E>(
    inputs: &[(Option<I>, Vec<E>)],
    cap: usize,
) -> Result<Option<(I, &[E])>, FiniteLimit> {
    let Some((identity, members)) = inputs.first() else {
        return Ok(None);
    };
    if inputs
        .iter()
        .skip(1)
        .any(|(other, _)| identity.is_none() || other != identity)
    {
        return Err(FiniteLimit::IndependentInputs);
    }
    let identity = identity.ok_or(FiniteLimit::MissingIdentity)?;
    if members.len() > cap {
        return Err(FiniteLimit::TooManyMembers);
    }
    Ok(Some((identity, members)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_integer_boundaries_remain_sound() {
        assert_eq!(negate(constant(i64::MIN)).hi, None);
        assert_eq!(negate(constant(i64::MIN)).lo, None);
        assert_eq!(add(constant(i64::MAX), constant(1)).hi, None);
        assert_eq!(mul(constant(i64::MAX), constant(2)), TOP);
        assert_eq!(
            binary(tcl_syntax::expr::ast::BinOp::Eq, BOTTOM, TOP),
            BOTTOM
        );
    }

    #[test]
    fn partial_arithmetic_contains_fixed_tcl_execution_witnesses() {
        use tcl_syntax::expr::ast::BinOp;
        let cases = [
            (
                "-2 + 3",
                BinOp::Add,
                Interval {
                    lo: Some(-2),
                    hi: Some(4),
                },
                Interval {
                    lo: Some(3),
                    hi: Some(8),
                },
            ),
            (
                "4 - 8",
                BinOp::Sub,
                Interval {
                    lo: Some(-2),
                    hi: Some(4),
                },
                Interval {
                    lo: Some(3),
                    hi: Some(8),
                },
            ),
            (
                "-2 * 8",
                BinOp::Mul,
                Interval {
                    lo: Some(-2),
                    hi: Some(4),
                },
                Interval {
                    lo: Some(3),
                    hi: Some(8),
                },
            ),
            (
                "4 < 3",
                BinOp::Lt,
                Interval {
                    lo: Some(-2),
                    hi: Some(4),
                },
                Interval {
                    lo: Some(3),
                    hi: Some(8),
                },
            ),
        ];
        for release in tcl_dialect::TclVersion::ALL {
            let Some(interpreter) = tcl_test_support::witness_tclsh(release) else {
                continue;
            };
            for (expression, operation, left, right) in cases {
                let script = format!("puts [expr {{{expression}}}]\n");
                let output = tcl_test_support::run_script(&interpreter.path, script.as_bytes())
                    .unwrap()
                    .strict_text()
                    .unwrap();
                let actual: i64 = output.trim().parse().unwrap();
                let bounds = binary(operation, left, right);
                assert!(!bounds.is_bottom());
                assert!(
                    bounds.lo.is_none_or(|lower| lower <= actual),
                    "{release:?}: {expression}: {bounds:?} excludes {actual}"
                );
                assert!(
                    bounds.hi.is_none_or(|upper| upper >= actual),
                    "{release:?}: {expression}: {bounds:?} excludes {actual}"
                );
            }
        }
    }

    #[test]
    fn finite_inputs_keep_repeated_values_correlated() {
        let repeated = [(Some(7), vec![1, 2]), (Some(7), vec![1, 2])];
        let (identity, members) = finite_input(&repeated, 8).unwrap().unwrap();
        assert_eq!(identity, 7);
        assert_eq!(
            members
                .iter()
                .map(|value| value * value)
                .collect::<Vec<_>>(),
            [1, 4]
        );
        assert_eq!(
            finite_input(&[(Some(7), vec![1]), (Some(8), vec![2])], 8),
            Err(FiniteLimit::IndependentInputs)
        );
        assert_eq!(
            finite_input(&[(None::<u8>, vec![1])], 8),
            Err(FiniteLimit::MissingIdentity)
        );
        assert_eq!(
            finite_input(&[(Some(7), vec![1, 2])], 1),
            Err(FiniteLimit::TooManyMembers)
        );
    }
}
