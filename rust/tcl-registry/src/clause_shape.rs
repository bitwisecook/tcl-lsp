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
/// Returns the first structural issue, or `None` for any shape the
/// command accepts.
pub type ClauseShapeChecker = fn(args: crate::InvocationArguments<'_>) -> Option<ClauseShapeIssue>;

/// An optional source proposal selected by the clause grammar itself.
/// Positions address complete original arguments after the command head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseShapeRepair {
    /// Merge the recognised final body and all trailing words into one body.
    MergeTrailingWords {
        /// Index of the recognised final body, before the first extra word.
        body: usize,
    },
    /// Remove a trailing incomplete clause from its actual opening keyword.
    RemoveTrailingClause {
        /// Index of the actual opening keyword of the incomplete clause.
        keyword: usize,
    },
}

/// The first clause defect and an independently authored proposal anchor.
/// A diagnostic-only hook carries no proposal merely because its error has
/// the same presentation as an issue produced by a different clause grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClauseShapeIssue {
    error: ClauseShapeError,
    repair: Option<ClauseShapeRepair>,
}

impl ClauseShapeIssue {
    /// Report a defect without borrowing a different grammar's edit anchors.
    #[must_use]
    pub const fn diagnostic(error: ClauseShapeError) -> Self {
        Self {
            error,
            repair: None,
        }
    }

    pub(crate) const fn with_repair(
        error: ClauseShapeError,
        repair: Option<ClauseShapeRepair>,
    ) -> Self {
        Self { error, repair }
    }

    /// The structural defect, independent of diagnostic presentation.
    #[must_use]
    pub const fn error(self) -> ClauseShapeError {
        self.error
    }

    /// A proposal anchor authored by the same selected clause grammar.
    #[must_use]
    pub const fn repair(self) -> Option<ClauseShapeRepair> {
        self.repair
    }
}
