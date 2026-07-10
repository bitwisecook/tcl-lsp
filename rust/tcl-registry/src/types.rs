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

//! Tcl internal representation types.
//!
//! Tcl values are always strings but may cache a typed internal
//! representation. This enum models the set of known intreps used
//! throughout the registry, compiler, and analyser.

/// Known Tcl internal representation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TclType {
    /// Pure string (no cached intrep).
    String,
    /// Integer.
    Int,
    /// Double-precision float.
    Double,
    /// Boolean.
    Boolean,
    /// Tcl list.
    List,
    /// Tcl dict.
    Dict,
    /// Byte array.
    ByteArray,
    /// Abstract join of `Int` and `Double`.
    Numeric,
    /// `TclOO` object instance.
    Object,
    /// I/O channel handle.
    Channel,
}

/// How a command types the variable(s) it *writes* as a side effect — its
/// [`ArgRole::VarWrite`](crate::arg_role::ArgRole::VarWrite) / IR `defs`
/// targets — as distinct from the value the command *returns*
/// ([`CommandSpec::return_type`](crate::CommandSpec::return_type)).
///
/// A variable a command writes does not always receive the command's return
/// value.  `append` / `lappend` store exactly what they return, so the return
/// type describes both.  But a destructuring command returns one thing while
/// writing another: `lassign` returns the *leftover* list yet writes list
/// *elements*; `scan` / `regexp` / `binary scan` return a match/convert
/// *count* yet write parsed pieces; `gets chan line` returns the character
/// count yet writes the *line*.  Broadcasting the return type onto those
/// targets is the S100 / W126 false-positive source (issue #867): a `lassign`
/// target wrongly typed `List`, a `regexp` capture wrongly typed `Int`.
///
/// The compiler's type-inference pass reads this per command / subcommand so
/// it never keys on the command name — the distinction lives in the registry
/// as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VarWriteTyping {
    /// The written variable's new value *is* the command's return value, so
    /// type it from [`CommandSpec::return_type`](crate::CommandSpec::return_type).
    /// The default — matches `append`, `lappend`, `ledit`, `lset`, `dict set`,
    /// and every writer whose stored value is its result.
    #[default]
    ReturnValue,
    /// The written variable receives a fixed intrep, independent of the
    /// command's return value.  `gets chan line` stores a text `String` line
    /// (while returning the character count); `lpop listVar` leaves a `List`
    /// (while returning the popped element).
    Fixed(TclType),
    /// The written variables receive destructured elements / parsed pieces
    /// whose static intrep is unknown and unrelated to the return value —
    /// `lassign` (list elements of any type), `scan` / `binary scan`
    /// (format-dependent conversions), `regexp` / `regsub` (matched
    /// substrings).  Each target widens to *overdefined* so no downstream
    /// type check reads a bogus intrep.
    Destructured,
}
