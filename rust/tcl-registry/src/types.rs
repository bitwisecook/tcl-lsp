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
    /// `scan` / `binary scan` (format-dependent conversions), `regexp` /
    /// `regsub` (matched substrings).  Each target widens to *overdefined*
    /// so no downstream type check reads a bogus intrep.
    Destructured,
    /// The written variables receive the container argument's elements
    /// **positionally**: target `i` takes element `i` of the container at
    /// `container_arg` (0-based, after the command / subcommand word).
    /// `lassign $l a b` types `a` / `b` from `$l`'s tracked per-position
    /// element shapes when known, and widens each target to *overdefined*
    /// otherwise (the pre-element-tracking `Destructured` behaviour).
    /// `foreach` / `lmap` element variables share this semantic; their
    /// group→container mapping rides the lowered statement's `foreach_groups`
    /// metadata rather than a single argument index.
    ElementsOf {
        /// 0-based index of the container argument.
        container_arg: u8,
    },
}

/// A per-call refinement of [`crate::CommandSpec::return_type`] — the
/// "per-form return typing" the specs of `regexp`, `regsub`, `scan`, and
/// `pid` each defer to.
///
/// `return_type` is one fact per command, which is right only for a command
/// whose result shape never moves. Several core commands hand back a
/// different *kind* of value depending on how they were called: `regexp`
/// counts matches but `regexp -inline` returns the matched substrings as a
/// list, `regsub` returns a replacement count until its `varName` is omitted
/// and it returns the substituted string instead. Typing every call by the
/// command's usual result makes the compiler confidently wrong about the
/// others — issue #1720, where iterating a `regexp -all -inline` result drew
/// a shimmer warning saying the list "has int intrep".
///
/// Forms are tried in declaration order and the first match wins, so declare
/// the more specific shape first (`lsearch` declares `-all` ahead of
/// `-inline`, because `-all -inline` is a list of elements while a bare
/// `-inline` is one element).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReturnForm {
    /// The call returns `then` when `switch` appears among its leading
    /// switches. The switch is matched through the command's own option
    /// table, so a legal abbreviation counts (`lsearch -al` is `-all`) and a
    /// word past the `--` terminator does not.
    WhenSwitch {
        /// Canonical spelling of the switch, as declared in the command's
        /// option table.
        switch: &'static str,
        /// The type the call produces, or `None` when the result's intrep is
        /// whatever the caller already put there — `lsearch -inline` lifts an
        /// element straight out of the source list.
        then: Option<TclType>,
    },
    /// The call returns `then` when it supplies exactly `count` positional
    /// arguments after its leading switches — the optional-trailing-argument
    /// split (`regsub`'s `varName`, `pid`'s `fileId`, `scan`'s inline form).
    WhenPositionals {
        /// Number of positional words, switches and their values excluded.
        count: u8,
        /// The type the call produces, or `None` when it can't be typed.
        then: Option<TclType>,
    },
}

impl ReturnForm {
    /// What this form says the call returns — `None` when the form matches
    /// but the result's intrep still can't be named.
    #[must_use]
    pub const fn then(self) -> Option<TclType> {
        match self {
            Self::WhenSwitch { then, .. } | Self::WhenPositionals { then, .. } => then,
        }
    }
}

/// How a command's *result value* relates to container element structure —
/// the registry fact behind per-element type inference
/// (`docs/design/compiler/type-tracking.md`, P3). Read by the compiler's
/// type-propagation pass; never keyed on command names in the compiler.
///
/// Faithful to the runtime: container elements are shared `Tcl_Obj`s, so a
/// builder's computed argument keeps its intrep inside the container, and a
/// retrieval hands back the same object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReturnElements {
    /// The result is a list of exactly the argument words from `from`
    /// onward, one element per word in order (`list ?value …?`).
    ListOfArgs {
        /// 0-based index of the first element word.
        from: u8,
    },
    /// The result is a dict built from alternating key/value words from
    /// `from` onward (`dict create ?key value …?`); element facts describe
    /// the **values**.
    DictOfPairs {
        /// 0-based index of the first key word.
        from: u8,
    },
    /// The result is one element of the container argument — `lindex $l $i`
    /// (single-index form) / `dict get $d $k` (single-key form). Multi-level
    /// paths are outside this fact; the compiler applies it only to the
    /// single-step call shape.
    ElementOf {
        /// 0-based index of the container argument.
        container_arg: u8,
    },
    /// The result is a sub-list of the container argument (`lrange $l a b`):
    /// a list whose every element is bounded by the source's uniform element
    /// shape.
    SubListOf {
        /// 0-based index of the container argument.
        container_arg: u8,
    },
}

/// How a command evolves the container *elements* of the variable it writes
/// in place — the registry fact that generalises the old object-only
/// element-class harvesting to every element shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarElementsEffect {
    /// `lappend var ?value …?` — appends each value word (from `values_from`
    /// onward) as one new element of the variable's list.
    AppendsListElements {
        /// 0-based index of the first appended value word.
        values_from: u8,
    },
    /// `dict set var ?key …? value` — stores the final word as one value of
    /// the variable's dict. Only the single-key form carries the leaf's
    /// shape; a nested path (`dict set d outer inner v`) stores a *dict*
    /// under the first key (tclsh 8.6/9.0: `dict get $d outer` is a dict),
    /// so the consumer records a value-shape of `Dict` with unknown
    /// structure for multi-key writes.
    SetsDictValue,
    /// `dict append var key ?value …?` — the stored value is a string
    /// *concatenation*, so value **intreps do not survive** (tclsh 8.6:
    /// string for both a missing and an existing key; 9.0 keeps the
    /// argument's intrep only on the missing-key fast path). What does
    /// survive is an object's *dispatch identity* — the objref text — so
    /// only object-class facts flow into the value bound (the
    /// collection-of-objects pattern, issue #797); every other shape
    /// contributes nothing.
    ExtendsDictValuesByName {
        /// 0-based index of the first appended value word.
        values_from: u8,
    },
    /// `dict lappend var key ?value …?` — the key's value becomes a *list*
    /// (tclsh: `dict get` after `dict lappend` has list intrep; a prior
    /// scalar value becomes element 0). The list wrapper is the fact; the
    /// element shapes are not tracked (the prior value's shape is part of
    /// the elements and is not statically known).
    ListifiesDictValue,
}
