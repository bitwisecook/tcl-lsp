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

//! Where a `catch` or `try` body splits into blocks, and how the statement after a split may fail.

use crate::ir::Statement;

/// How a protected region's scripts split their statements into blocks in an
/// analysis build, so that every point at which the region can fail ends a
/// block an exception edge leaves from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum SplitRegion {
    /// A flattened `catch` body: each of its straight-line statements, every
    /// block a split ends a point the end block is thrown to from.
    Catch,
    /// A `try` body: each statement of it and of every script it holds, save
    /// a nested `try` or `catch` body, which is a region of its own.
    Try,
}

impl SplitRegion {
    /// The prefix the blocks a split begins are named with.
    pub(in super::super) const fn step(self) -> &'static str {
        match self {
            Self::Catch => "catch_step",
            Self::Try => "try_step",
        }
    }

    /// Whether the scripts the region's statements hold — an `if` arm, a loop
    /// body, a handler of a nested `try` — split too. A flattened `catch` body
    /// is straight-line statements and holds none.
    pub(in super::super) const fn nested(self) -> bool {
        matches!(self, Self::Try)
    }

    /// Whether a statement that leaves from the state before it stays in the
    /// block before it ([`CfgBuilder::leaves_from_the_state_before`]), and a
    /// block a split ends is a point a throw leaves from only where one may
    /// ([`CfgBuilder::a_throw_may_leave_from`]): a `try`'s handlers select by
    /// completion, so an edge one cannot take is a path that never runs.
    pub(in super::super) const fn selective(self) -> bool {
        matches!(self, Self::Try)
    }
}

/// A point a throw may leave a protected region from: the exit of a block a
/// split ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in super::super) struct SplitPoint {
    /// The block whose exit the point is.
    pub(in super::super) block: String,
    /// Whether a statement of the block may leave after its own stores, with
    /// any code.
    pub(in super::super) after_stores: bool,
    /// How the statement after the block fails before it stores.
    pub(in super::super) next: NextFailure,
}

impl SplitPoint {
    /// A point anything may leave from: a body's tail, a throw point, or a
    /// point of a nested body that hands its ways out to the region around it.
    pub(in super::super) fn any(block: String) -> Self {
        Self {
            block,
            after_stores: true,
            next: NextFailure::Any,
        }
    }

    /// Whether a throw may leave from the point at all.
    pub(in super::super) fn may_leave(&self) -> bool {
        self.after_stores || self.next != NextFailure::Intercepted
    }

    /// Whether the statement after the point may fail before it stores with
    /// a code a target takes when `takes` says so.
    pub(in super::super) fn next_fails_for(
        &self,
        takes: impl Fn(tcl_core_types::Code) -> bool,
    ) -> bool {
        match self.next {
            NextFailure::Intercepted => false,
            NextFailure::Error => takes(tcl_core_types::Code::Error),
            NextFailure::Any => true,
        }
    }
}

/// How a statement fails before it stores, for the state before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) enum NextFailure {
    /// It runs a clause of its own first — a `catch`, which takes every
    /// completion of its script, or a `try` with a `finally`, whose clause is
    /// the way out of it — so nothing leaves from the state before it.
    Intercepted,
    /// It raises an error if it fails at all: a literal assignment.
    Error,
    /// It may complete with any code.
    Any,
}

impl NextFailure {
    /// How `stmt` fails before it stores.
    pub(in super::super) fn of(stmt: &Statement) -> Self {
        if matches!(
            stmt,
            Statement::Catch { .. }
                | Statement::Try {
                    finally_body: Some(_),
                    ..
                }
        ) {
            Self::Intercepted
        } else if is_literal_assignment(stmt) {
            Self::Error
        } else {
            Self::Any
        }
    }
}

/// Whether `stmt` assigns a literal to a name that substitutes nothing: it
/// completes normally or raises `TCL_ERROR` (the name is an array, a write
/// trace fails), never another code, and it raises before it stores.
pub(in super::super) fn is_literal_assignment(stmt: &Statement) -> bool {
    matches!(stmt, Statement::AssignConst { name, name_braced, .. }
        if *name_braced || !name.contains('['))
}
