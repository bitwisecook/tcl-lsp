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

/// Maximum number of values in a `CONSTSET` before widening to `OVERDEFINED`.
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
#[derive(Debug, Clone, PartialEq)]
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

/// A constant value in the SCCP lattice.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    /// Integer constant.
    Int(i64),
    /// Floating-point constant.
    Float(f64),
    /// Boolean constant.
    Bool(bool),
    /// String constant.
    String(String),
}

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
            LatticeValue::constant(ConstValue::Int(42)).kind(),
            LatticeKind::Const
        );
        assert_eq!(
            LatticeValue::constset(vec![ConstValue::Int(1), ConstValue::Int(2)]).kind(),
            LatticeKind::ConstSet
        );
    }

    #[test]
    fn constset_auto_widens() {
        let big: Vec<ConstValue> = (0..50).map(ConstValue::Int).collect();
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
