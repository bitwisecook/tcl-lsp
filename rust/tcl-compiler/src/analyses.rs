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

//! Core analysis result types.
//!
//! This module defines the output structures produced by dataflow
//! analyses (SCCP, liveness, dead-store detection, type inference,
//! taint analysis). The analysis *algorithms* live in their own modules
//! (`sccp`, `type_infer`, `taint`, …); this module provides only the data
//! types that downstream consumers (diagnostics, codegen, optimiser) read.

use std::fmt;

use tcl_registry::value_transfer::{ExactValue, NumericValue};

/// Maximum number of values in a [`LatticeValue::ConstSet`] before widening to
/// [`LatticeValue::Overdefined`].
pub const MAX_CONSTSET_SIZE: usize = 32;

// SCCP constant propagation lattice

/// Kind of a constant-propagation lattice element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LatticeKind {
    /// Bottom — no information yet.
    Unknown,
    /// A single known constant value.
    Const,
    /// A small set of possible constant values.
    ConstSet,
    /// Top — value varies too much to track.
    Overdefined,
}

/// A Tcl value in the SCCP constant-propagation lattice.
///
/// Progression: `Unknown → Const(v) → ConstSet({v1, v2, …}) → Overdefined`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LatticeValue {
    /// No information.
    Unknown,
    /// A single known constant.
    Const(ConstValue),
    /// A small set of possible values (up to [`MAX_CONSTSET_SIZE`]).
    ConstSet(Vec<ConstValue>),
    /// Too many values to track.
    Overdefined,
}

/// A constant value in the SCCP lattice. Doubles compare by their bits,
/// since signed zero and NaN payloads are distinct lattice members.
#[derive(Clone)]
pub enum ConstValue {
    /// Numeric or boolean value whose text is materialised by the consumer.
    Number(NumericValue),
    /// String constant.
    String(LatticeText),
}

impl ConstValue {
    /// An integer constant.
    #[must_use]
    pub const fn int(value: i64) -> Self {
        Self::Number(NumericValue::Int(value))
    }

    /// A floating-point constant.
    #[must_use]
    pub const fn float(value: f64) -> Self {
        Self::Number(NumericValue::Float(value))
    }

    /// A boolean constant.
    #[must_use]
    pub const fn boolean(value: bool) -> Self {
        Self::Number(NumericValue::Bool(value))
    }
}

impl fmt::Debug for ConstValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(value) => fmt::Debug::fmt(value, formatter),
            Self::String(value) => formatter.debug_tuple("String").field(value).finish(),
        }
    }
}

impl PartialEq for ConstValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(NumericValue::Float(left)), Self::Number(NumericValue::Float(right))) => {
                left.to_bits() == right.to_bits()
            }
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for ConstValue {}

/// Exact UTF-8 text in the contents lattice. Native representation and
/// release-dependent numeric readings require separate evidence at a read.
#[derive(Clone)]
pub struct LatticeText(ExactValue<String>);

impl LatticeText {
    /// The string's exact UTF-8 contents.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0.bytes
    }

    pub(crate) fn into_exact(self) -> ExactValue<String> {
        self.0
    }
}

impl From<String> for LatticeText {
    fn from(text: String) -> Self {
        Self(ExactValue::utf8(text))
    }
}

impl From<&str> for LatticeText {
    fn from(text: &str) -> Self {
        Self::from(text.to_owned())
    }
}

impl fmt::Debug for LatticeText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), formatter)
    }
}

impl PartialEq for LatticeText {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for LatticeText {}

impl LatticeValue {
    /// The bottom element.
    #[must_use]
    pub fn unknown() -> Self {
        Self::Unknown
    }

    /// The top element.
    #[must_use]
    pub fn overdefined() -> Self {
        Self::Overdefined
    }

    /// A single constant value.
    #[must_use]
    pub fn constant(value: ConstValue) -> Self {
        Self::Const(value)
    }

    /// A set of constant values (auto-widens if too large).
    #[must_use]
    pub fn constset(values: Vec<ConstValue>) -> Self {
        if values.len() > MAX_CONSTSET_SIZE {
            Self::Overdefined
        } else {
            Self::ConstSet(values)
        }
    }

    /// The lattice kind of this value.
    #[must_use]
    pub fn kind(&self) -> LatticeKind {
        match self {
            Self::Unknown => LatticeKind::Unknown,
            Self::Const(_) => LatticeKind::Const,
            Self::ConstSet(_) => LatticeKind::ConstSet,
            Self::Overdefined => LatticeKind::Overdefined,
        }
    }
}

// Analysis diagnostic types

/// A branch whose condition was determined by SCCP.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantBranch {
    /// CFG block name containing the branch.
    pub block: String,
    /// Condition expression text.
    pub condition: String,
    /// Constant-evaluated condition result.
    pub value: bool,
    /// Target block when condition is true.
    pub taken_target: String,
    /// Target block when condition is false.
    pub not_taken_target: String,
}

/// A dead store: a variable definition that is never read.
#[derive(Debug, Clone, PartialEq)]
pub struct DeadStore {
    /// CFG block name.
    pub block: String,
    /// Statement index within the block.
    pub statement_index: usize,
    /// Variable name.
    pub variable: String,
    /// SSA version of the dead definition.
    pub version: u32,
}

/// A variable read before it was set.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadBeforeSet {
    /// CFG block name.
    pub block: String,
    /// Statement index within the block.
    pub statement_index: usize,
    /// Variable name.
    pub variable: String,
}

/// An unused variable (defined but never read anywhere).
#[derive(Debug, Clone, PartialEq)]
pub struct UnusedVariable {
    /// CFG block name of the definition.
    pub block: String,
    /// Statement index within the block.
    pub statement_index: usize,
    /// Variable name.
    pub variable: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lattice_value_kind() {
        assert_eq!(LatticeValue::unknown().kind(), LatticeKind::Unknown);
        assert_eq!(LatticeValue::overdefined().kind(), LatticeKind::Overdefined);
        assert_eq!(
            LatticeValue::constant(ConstValue::int(42)).kind(),
            LatticeKind::Const
        );
        assert_eq!(
            LatticeValue::constset(vec![ConstValue::int(1), ConstValue::int(2)]).kind(),
            LatticeKind::ConstSet
        );
    }

    #[test]
    fn constset_auto_widens() {
        let big: Vec<ConstValue> = (0..50).map(ConstValue::int).collect();
        assert_eq!(LatticeValue::constset(big).kind(), LatticeKind::Overdefined);
    }

    #[test]
    fn dead_store_construction() {
        let ds = DeadStore {
            block: "entry".into(),
            statement_index: 0,
            variable: "x".into(),
            version: 1,
        };
        assert_eq!(ds.variable, "x");
    }

    #[test]
    fn constant_branch_construction() {
        let cb = ConstantBranch {
            block: "if_dispatch_1".into(),
            condition: "$x == 1".into(),
            value: true,
            taken_target: "if_then_1".into(),
            not_taken_target: "if_else_1".into(),
        };
        assert!(cb.value);
    }
}
