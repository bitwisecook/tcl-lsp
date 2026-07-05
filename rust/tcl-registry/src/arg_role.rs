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

//! Argument roles — what role each argument plays in a command.

/// What role an argument plays in a command invocation.
///
/// Used by the compiler, analyser, and LSP features to understand
/// which arguments are scripts to recurse into, which are variable
/// names, which are expressions, etc. Consumers query roles via the
/// registry — never by matching on command names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ArgRole {
    /// Tcl script body — recursively analysed.
    Body,
    /// Expression (`expr` sub-language).
    Expr,
    /// Variable name written by the command (`set`, `incr`, `lassign`).
    VarWrite,
    /// Variable name read without modification (`info exists`, `array get`).
    VarRead,
    /// Loop variable-binding list evaluated once before the body
    /// (`dict for {k v} …`, `dict map {k v} …`).
    LoopVarList,
    /// Procedure parameter list.
    ParamList,
    /// Symbolic name (proc name, namespace name).
    Name,
    /// Pattern or regex.
    Pattern,
    /// Switch/flag option.
    Option,
    /// Generic value argument.
    Value,
    /// The subcommand word (e.g. `"length"` in `string length`).
    Subcommand,
    /// The `--` option terminator.
    OptionTerminator,
    /// Channel identifier (`stdout`, `stdin`, channel ID).
    Channel,
    /// List/string index expression.
    Index,
    /// A structural keyword word — `if`'s `then`/`elseif`/`else`,
    /// `try`'s `on`/`trap`/`finally`. These sit at argument positions
    /// (not the command-name slot), so the semantic-token layer marks
    /// them with this role to highlight them as keywords rather than
    /// strings. Adding `Keyword` to a position that previously had no
    /// role is inert for every other role consumer — they filter by the
    /// roles they care about.
    Keyword,
}
