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

//! Typed static outcomes of a registry-declared command-prefix expression.
//!
//! The registry owns the vocabulary. Consumers attach source spans and prove a
//! concrete expression has one of these outcomes by following
//! [`crate::Traits::BUILDS_COMMAND_PREFIX`] and
//! [`crate::Traits::WRAPS_COMMAND_PREFIX`].

/// The dispatch model represented by a statically recoverable command prefix.
///
/// This is deliberately not a boolean: current-object callbacks have two
/// different visibility models, and wrappers must preserve that distinction.
/// Future class systems can extend the closed vocabulary without each
/// consumer inventing its own callback classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandPrefixTarget {
    /// A normal static command head such as `[list ::pkg::callback arg]`.
    DirectCommandHead,
    /// `[list [self] method ...]`, dispatched externally through an object command.
    CurrentObjectExternalMethod,
    /// `[list my method ...]`, dispatched through the current object's method frame.
    CurrentObjectInternalMethod,
}
