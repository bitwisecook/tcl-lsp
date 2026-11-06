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

//! Structured parse issues issued where an original construct fails.
//!
//! Diagnostic text is a presentation of the kind. Completeness reads the
//! retained kind; it never recovers semantics from that presentation.

/// An issue selected by an original lexical or component parser.
/// The kind describes syntax only and grants no evaluation or native admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParseIssueKind {
    /// Jim expression substitution has no closing parenthesis.
    MissingExpressionParenthesis,
    /// A braced variable name has no selected closing brace.
    MissingVariableBrace,
    /// The Jim token lexer has an unterminated array index.
    MissingArrayIndexParenthesis,
    /// An array index contains a character rejected by the selected grammar.
    InvalidArrayIndexCharacter,
    /// The component parser has an unterminated parenthesised array index.
    MissingParenthesis,
    /// Non-separator text follows a strict word's closing quote.
    ExtraAfterQuote,
    /// A quoted word has no closing quote.
    MissingQuote,
    /// Non-separator text follows a braced word's closing brace.
    ExtraAfterBrace,
    /// A braced word has no closing brace.
    MissingBrace,
    /// A command substitution has no closing bracket.
    MissingBracket,
}

impl ParseIssueKind {
    /// Preserve the parser's existing diagnostic presentation.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::MissingExpressionParenthesis => "missing close-paren for expression substitution",
            Self::MissingVariableBrace => crate::MISSING_CLOSE_BRACE_FOR_VAR,
            Self::MissingArrayIndexParenthesis => "missing close-paren for array index",
            Self::InvalidArrayIndexCharacter => crate::INVALID_CHARACTER_IN_ARRAY_INDEX,
            Self::MissingParenthesis => crate::MISSING_PAREN,
            Self::ExtraAfterQuote => crate::EXTRA_AFTER_CLOSE_QUOTE,
            Self::MissingQuote => crate::MISSING_QUOTE,
            Self::ExtraAfterBrace => crate::EXTRA_AFTER_CLOSE_BRACE,
            Self::MissingBrace => crate::MISSING_CLOSE_BRACE,
            Self::MissingBracket => crate::MISSING_CLOSE_BRACKET,
        }
    }

    /// Whether the reached issue is a missing delimiter rather than terminal syntax.
    /// This is the C parser's incomplete projection, independently of presentation.
    #[must_use]
    pub const fn is_incomplete(self) -> bool {
        !matches!(
            self,
            Self::InvalidArrayIndexCharacter | Self::ExtraAfterQuote | Self::ExtraAfterBrace
        )
    }
}
