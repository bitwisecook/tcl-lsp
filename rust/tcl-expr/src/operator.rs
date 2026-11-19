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

//! Operator syntax, precedence, and the source text channels.

use std::fmt;

/// Owned expression leaves; native execution keeps byte payloads while
/// Unicode analysis uses the default `String` specialization.
pub trait ExprText: Clone {
    /// The exact original leaf bytes.
    fn bytes(&self) -> &[u8];
    /// Own a slice from the same source channel. Unicode consumers supply
    /// only verified Unicode token boundaries.
    fn from_source_bytes(bytes: &[u8]) -> Self;
    /// A checked textual projection for ASCII grammar and Unicode advice.
    fn try_text(&self) -> Option<&str> {
        std::str::from_utf8(self.bytes()).ok()
    }
}

impl ExprText for String {
    fn bytes(&self) -> &[u8] {
        self.as_bytes()
    }
    fn from_source_bytes(bytes: &[u8]) -> Self {
        String::from_utf8(bytes.to_vec()).expect("Unicode expression source boundary")
    }
}

impl ExprText for Vec<u8> {
    fn bytes(&self) -> &[u8] {
        self
    }
    fn from_source_bytes(bytes: &[u8]) -> Self {
        bytes.to_vec()
    }
}

/// Character offset within expression source text.
pub type ExprOffset = u32;

// Binary operators

/// Binary operators in Tcl expressions.
///
/// Covers standard Tcl arithmetic, comparison, logical, bitwise, and
/// string operators, plus iRules-specific extensions (`contains`,
/// `starts_with`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    // Arithmetic
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `%`
    Mod,
    /// `**`
    Pow,

    // Shift
    /// `<<`
    LShift,
    /// `>>`
    RShift,

    // Bitwise
    /// `&`
    BitAnd,
    /// `|`
    BitOr,
    /// `^`
    BitXor,

    // Logical
    /// `&&`
    And,
    /// `||`
    Or,

    // Numeric comparison
    /// `==`
    Eq,
    /// `!=`
    Ne,
    /// `<`
    Lt,
    /// `<=`
    Le,
    /// `>`
    Gt,
    /// `>=`
    Ge,

    // String comparison
    /// `eq`
    StrEq,
    /// `ne`
    StrNe,
    /// `lt`
    StrLt,
    /// `le`
    StrLe,
    /// `gt`
    StrGt,
    /// `ge`
    StrGe,

    // List membership
    /// `in`
    In,
    /// `ni`
    Ni,

    // iRules word-based logical operators
    /// `and`
    WordAnd,
    /// `or`
    WordOr,

    // iRules string comparison operators
    /// `contains`
    Contains,
    /// `starts_with`
    StartsWith,
    /// `ends_with`
    EndsWith,
    /// `equals`
    StrEquals,
    /// `matches`
    Matches,
    /// `matches_glob`
    MatchesGlob,
    /// `matches_regex`
    MatchesRegex,
}

impl BinOp {
    /// Whether the operator always produces a boolean `0`/`1`: comparisons,
    /// membership and the logical connectives.
    #[must_use]
    pub const fn yields_boolean(self) -> bool {
        matches!(
            self,
            Self::And
                | Self::Or
                | Self::WordAnd
                | Self::WordOr
                | Self::Eq
                | Self::Ne
                | Self::Lt
                | Self::Le
                | Self::Gt
                | Self::Ge
                | Self::StrEq
                | Self::StrNe
                | Self::StrLt
                | Self::StrLe
                | Self::StrGt
                | Self::StrGe
                | Self::In
                | Self::Ni
                | Self::Contains
                | Self::StartsWith
                | Self::EndsWith
                | Self::StrEquals
                | Self::Matches
                | Self::MatchesGlob
                | Self::MatchesRegex
        )
    }

    /// Return the source-text representation of this operator.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Mod => "%",
            Self::Pow => "**",
            Self::LShift => "<<",
            Self::RShift => ">>",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::And => "&&",
            Self::Or => "||",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::StrEq => "eq",
            Self::StrNe => "ne",
            Self::StrLt => "lt",
            Self::StrLe => "le",
            Self::StrGt => "gt",
            Self::StrGe => "ge",
            Self::In => "in",
            Self::Ni => "ni",
            Self::WordAnd => "and",
            Self::WordOr => "or",
            Self::Contains => "contains",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
            Self::StrEquals => "equals",
            Self::Matches => "matches",
            Self::MatchesGlob => "matches_glob",
            Self::MatchesRegex => "matches_regex",
        }
    }

    /// Operator precedence (higher = tighter binding).
    ///
    /// Used by [`render_expr`] to
    /// determine when parentheses are needed.
    #[must_use]
    pub const fn precedence(self) -> u8 {
        match self {
            Self::Or | Self::WordOr => 1,
            Self::And | Self::WordAnd => 2,
            Self::BitOr => 3,
            Self::BitXor => 4,
            Self::BitAnd => 5,
            Self::Eq
            | Self::Ne
            | Self::StrEq
            | Self::StrNe
            | Self::In
            | Self::Ni
            | Self::Contains
            | Self::StartsWith
            | Self::EndsWith
            | Self::StrEquals
            | Self::Matches
            | Self::MatchesGlob
            | Self::MatchesRegex => 6,
            Self::Lt
            | Self::Le
            | Self::Gt
            | Self::Ge
            | Self::StrLt
            | Self::StrLe
            | Self::StrGt
            | Self::StrGe => 7,
            Self::LShift | Self::RShift => 8,
            Self::Add | Self::Sub => 9,
            Self::Mul | Self::Div | Self::Mod => 10,
            Self::Pow => 11,
        }
    }

    /// Whether this operator is right-associative.
    #[must_use]
    pub const fn is_right_assoc(self) -> bool {
        matches!(self, Self::Pow)
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// Unary operators

/// Unary operators in Tcl expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    /// `-` (arithmetic negation)
    Neg,
    /// `+` (arithmetic identity)
    Pos,
    /// `~` (bitwise complement)
    BitNot,
    /// `!` (logical NOT)
    Not,
    /// `not` (iRules word-based NOT)
    WordNot,
}

impl UnaryOp {
    /// Whether the operator always produces a boolean `0`/`1` (logical NOT).
    #[must_use]
    pub const fn yields_boolean(self) -> bool {
        matches!(self, Self::Not | Self::WordNot)
    }

    /// Return the source-text representation of this operator.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Neg => "-",
            Self::Pos => "+",
            Self::BitNot => "~",
            Self::Not => "!",
            Self::WordNot => "not",
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
