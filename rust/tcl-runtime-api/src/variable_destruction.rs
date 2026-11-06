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

//! Physical variable teardown phases shared by native adapters and analysis.

/// A reached phase of an originally captured variable destruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableDestructionPhase {
    /// The original root lookup and root registrations have been detached.
    /// Captured array members remain accessible through existing direct aliases.
    RootLookupDetached,
    /// One captured member has become undefined before its frozen callbacks.
    /// Other members retain their own contents and active read/write guards.
    MemberRetired,
    /// Every captured member and its callbacks has completed teardown.
    Complete,
}

/// Native ordering of contents teardown and variable callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableDestructionProtocol {
    /// Retire one cell's contents before invoking its captured unset callbacks.
    CellContentsThenCallbacks,
    /// Detach the root lookup and root traces, invoke captured root callbacks,
    /// then retire each old member immediately before that member's callbacks.
    /// Member order is not a portable guarantee. Recreated names cannot redirect
    /// these captured receivers or callback registrations.
    ArrayLookupThenRootCallbacksThenMembers,
}

/// Selected C `ArrayUnset` member lookup after the original array callback.
/// The whole-root operation separately relooks up its original name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeArrayUnsetMemberLookup {
    /// C84/85 obtain each member through the original array-name operand.
    OriginalName,
    /// C86+ retain the located physical array for each member removal.
    SelectedArray,
}
