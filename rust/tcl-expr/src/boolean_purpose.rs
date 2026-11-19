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

//! Selected conversion site for expression and command truth.

/// Actual reached conversion site, independent of mathematical value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NativeBooleanTruthPurpose {
    /// The operand of a reached logical-not instruction.
    LogicalNot = 0,
    /// A reached expression conditional jump, including a ternary condition.
    ConditionalJump = 1,
    /// The short-circuit branch of a conjunction expression.
    LogicalAnd = 2,
    /// The short-circuit branch of a disjunction expression.
    LogicalOr = 3,
    /// Boolean extraction after the command's expression result producer.
    CommandCondition = 4,
    /// Boolean extraction after the public expression API's result producer.
    ExpressionApiResult = 5,
    /// C8.4's reached non-short-circuit logical-and instruction.
    LogicalAndInstruction = 6,
    /// C8.4's reached non-short-circuit logical-or instruction.
    LogicalOrInstruction = 7,
}

impl NativeBooleanTruthPurpose {
    /// Decode a counted ABI tag without selecting a default conversion.
    #[must_use]
    pub const fn from_abi(tag: i32) -> Option<Self> {
        match tag {
            0 => Some(Self::LogicalNot),
            1 => Some(Self::ConditionalJump),
            2 => Some(Self::LogicalAnd),
            3 => Some(Self::LogicalOr),
            4 => Some(Self::CommandCondition),
            5 => Some(Self::ExpressionApiResult),
            6 => Some(Self::LogicalAndInstruction),
            7 => Some(Self::LogicalOrInstruction),
            _ => None,
        }
    }

    /// Whether the caller supplies an independently evaluated expression result.
    #[must_use]
    pub const fn requires_expression_result(self) -> bool {
        matches!(self, Self::CommandCondition | Self::ExpressionApiResult)
    }
}
