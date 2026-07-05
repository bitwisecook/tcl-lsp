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

//! Argument count constraints.

/// Argument count range for a command or subcommand.
///
/// `min` and `max` are counts of arguments *after* the command name
/// (and after the subcommand word, for subcommands).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Arity {
    /// Minimum number of arguments.
    pub min: u16,
    /// Maximum number of arguments (`u16::MAX` = unlimited).
    pub max: u16,
}

impl Arity {
    /// Unlimited upper bound sentinel.
    pub const UNLIMITED: u16 = u16::MAX;

    /// Create an arity with explicit min and max.
    #[must_use]
    pub const fn new(min: u16, max: u16) -> Self {
        Self { min, max }
    }

    /// Exactly `n` arguments required.
    #[must_use]
    pub const fn exact(n: u16) -> Self {
        Self { min: n, max: n }
    }

    /// At least `min` arguments, no upper bound.
    #[must_use]
    pub const fn at_least(min: u16) -> Self {
        Self {
            min,
            max: Self::UNLIMITED,
        }
    }

    /// Zero or more arguments (no constraint).
    #[must_use]
    pub const fn any() -> Self {
        Self {
            min: 0,
            max: Self::UNLIMITED,
        }
    }

    /// Whether `n` arguments satisfy this constraint.
    #[must_use]
    pub const fn accepts(self, n: u16) -> bool {
        n >= self.min && n <= self.max
    }

    /// Whether the upper bound is unlimited.
    #[must_use]
    pub const fn is_unlimited(self) -> bool {
        self.max == Self::UNLIMITED
    }
}

impl Default for Arity {
    fn default() -> Self {
        Self::any()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_accepts_only_n() {
        let a = Arity::exact(4);
        assert!(!a.accepts(3));
        assert!(a.accepts(4));
        assert!(!a.accepts(5));
    }

    #[test]
    fn at_least_accepts_min_and_above() {
        let a = Arity::at_least(2);
        assert!(!a.accepts(1));
        assert!(a.accepts(2));
        assert!(a.accepts(100));
        assert!(a.is_unlimited());
    }

    #[test]
    fn range_accepts_within() {
        let a = Arity::new(1, 3);
        assert!(!a.accepts(0));
        assert!(a.accepts(1));
        assert!(a.accepts(3));
        assert!(!a.accepts(4));
        assert!(!a.is_unlimited());
    }

    #[test]
    fn any_accepts_everything() {
        let a = Arity::any();
        assert!(a.accepts(0));
        assert!(a.accepts(u16::MAX));
    }
}
