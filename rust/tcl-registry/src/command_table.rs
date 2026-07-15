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

//! Command-table mutation descriptors.
//!
//! A handful of commands mutate the interpreter's *command table*
//! itself — they define, move, or alias command names.  Consumers that
//! model command-name bindings (the flow-sensitive lattice in
//! `tcl_compiler::command_binding`, the lowerer's alias table, the
//! analyser's rename / alias records) need to know *which* calls do
//! this; before this descriptor existed each of them matched
//! `proc` / `rename` / `interp alias` by name.  The registry now
//! declares the effect on the spec (or subcommand), and the consumers
//! dispatch on it — the shape destructuring (argument layout, dynamic
//! operands) stays with the consumers.

/// How a command mutates the interpreter's command table.
///
/// Stamped on [`crate::CommandSpec::command_table_effect`] (or the
/// [`crate::SubCommand`] twin for a subcommand-shaped mutator such as
/// `interp alias`) and resolved via
/// [`crate::CommandRegistry::command_table_effect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandTableEffect {
    /// The call binds its first argument as a procedure — `proc name
    /// params body` (`Tcl_ProcObjCmd`, `generic/tclProc.c`).  Narrower
    /// than [`crate::Traits::DEFINES_PROCEDURE`], which also marks the
    /// `TclOO` metaclasses whose *name* argument sits behind a
    /// `create` / `new` subcommand word — a binding-lattice consumer
    /// reading argument 0 as the defined name must only see the
    /// `proc`-shaped form.
    DefinesProcedure,
    /// The call moves (or, with an empty target, deletes) a command —
    /// `rename oldName newName` (`Tcl_RenameObjCmd`,
    /// `generic/tclCmdMZ.c`, dispatching to `TclRenameCommand` in
    /// `generic/tclBasic.c`).
    RenamesCommands,
    /// The call creates (or, in the shorter forms, queries / deletes)
    /// a command alias — `interp alias` (`AliasCreate`,
    /// `generic/tclInterp.c`).  Stamped on `interp`'s `alias`
    /// subcommand.
    CreatesAliases,
}
