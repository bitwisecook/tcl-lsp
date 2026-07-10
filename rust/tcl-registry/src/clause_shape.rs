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

//! Clause-chain shape validation — for commands whose set of valid
//! argument shapes isn't a single `min..=max` [`crate::Arity`] range.
//!
//! `if`'s grammar (`expr ?then? body (elseif expr ?then? body)* (else
//! body)?`) is the first (and, today, only) consumer: a plain arity range
//! can express "at least 2 words" but not "an `elseif` must be followed by
//! an expression and a body" or "nothing may trail the final body". A
//! command with this shape of grammar sets
//! [`crate::spec::CommandSpec::clause_shape_check`] to a
//! [`ClauseShapeChecker`] that walks its own argument list and reports the
//! first defect, so the compiler's diagnostic for it (`if`'s `E004`) reads
//! registry data instead of re-parsing the grammar itself. A command
//! carrying the hook also sets [`crate::Traits::STRUCTURALLY_CHECKED_ARITY`]
//! so the generic arity floor/ceiling check steps aside — the hook owns
//! arity together with clause shape, so a malformed call gets one precise
//! diagnostic rather than a redundant generic one alongside it.

/// A structural defect in a command's clause-chain shape, reported by a
/// [`ClauseShapeChecker`].
///
/// Word indices are 0-based into the command's arguments *after* the
/// command name, matching every other by-position hook
/// (`ArgRoleResolver`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseShapeError {
    /// A clause that requires a sub-expression word never got one.
    MissingExpr {
        /// Index of the last present word that introduced the missing
        /// expression (e.g. an `elseif` keyword) — `None` when the
        /// command has no arguments at all, in which case the caller
        /// should name the command's own invoked spelling rather than a
        /// word.
        after: Option<usize>,
    },
    /// A clause that requires a body word never got one.
    MissingBody {
        /// Index of the last present word (a condition, a `then`
        /// keyword, or a terminal keyword such as `else`).
        after: usize,
    },
    /// One or more words trail the last recognised clause.
    ExtraWords {
        /// Index of the first extra word.
        first_extra: usize,
    },
}

/// Validate a command's clause-chain shape.
///
/// Called with the command's arguments (excluding the command name).
/// Returns the first structural defect, or `None` for any shape the
/// command accepts.
pub type ClauseShapeChecker = fn(args: &[&str]) -> Option<ClauseShapeError>;
