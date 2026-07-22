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

//! `lpop` — get and remove an element from a list variable (Tcl 9.0+, TIP 523).
//
// VERIFIED: Tcl 9.0 and 9.1 TclCmd manual trees carry byte-identical
// lpop(n) text — the same SYNOPSIS/DESCRIPTION/EXAMPLES/SEE ALSO/KEYWORDS
// in both (only the version-banner breadcrumb differs, "9.0.4" vs
// "9.1b0"), so nothing about `lpop` changed between the two releases.
// 8.4, 8.5, and 8.6 (the 8.6 tree checked via both the `.htm` and
// `.html` extensions) each resolve to the site's standard "URL Not
// Found" page rather than a real command page — `lpop` genuinely does
// not exist before 9.0. No dialect directory (irules/, expect/,
// eda_*/, iapps/, tk/, itcl/) references `lpop`, and none of those
// dialects model a base Tcl version of 9.0 or later, so the plain
// `TCL90_PLUS` gate below already excludes every one of them.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lpop varName ?index ...?",
    dialects: None,
}];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// Command spec for `lpop`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lpop",
        // `lpop` reads the list variable's current value, removes one
        // (possibly deeply nested) element, and writes the shortened list
        // back — a read-before-write of `varName`, like `lappend` /
        // `append` / `incr` (`READS_BEFORE_WRITE`), and `varName` is
        // itself a variable *name* argument, like `set` / `lset` / `ledit`
        // (`FIRST_ARG_VARNAME` — this also makes `rmw_first_arg_variable`
        // recognise `lpop` as an RMW target the minifier must not rename
        // out from under it). Unlike `ledit` (whose manpage explicitly
        // rules out sublist indices, so it deliberately stays off the
        // hash-bucket trait — see `commands/tcl/ledit.rs`), `lpop`
        // descends into nested sublists exactly like `lset`: the manpage
        // states that additional index arguments "address an element
        // within a sublist designated by the previous indexing
        // operation... similar to lindex and lset", so it shares `lset`'s
        // `FRAME_HASH_BUILTIN` classification rather than `ledit`'s
        // flat-name one. `lpop varName {i} {j}` is also a real `HEAD NAME
        // BRACED BRACED` four-token invocation shape (a two-level index
        // descent), so it needs `NOT_PROC_FACTORY` for the same reason
        // `ledit`/`lindex` do. `const` and `ledit` — the other Tcl-9.0-native
        // C-implemented list/variable commands — both carry
        // `BYTE_COMPILED`; `lpop` is the same kind of real core builtin
        // (it has its own dedicated manual page, unlike the Tcl-coded
        // `foreachLine`/`readFile`/`writeFile` library procs — see
        // `commands/tcl/foreachline.rs`), so it carries `BYTE_COMPILED` too.
        traits: Traits::READS_BEFORE_WRITE
            | Traits::FIRST_ARG_VARNAME
            | Traits::FRAME_HASH_BUILTIN
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: Some(TclType::List),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        return_type: Some(TclType::String),
        // `lpop` returns the *removed element* but leaves the variable holding
        // the shortened *list*.  Type the write as the list it always becomes,
        // not the popped element's return type (issue #867).
        var_write_typing: VarWriteTyping::Fixed(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Get and remove an element from a list variable, in place.",
            synopsis: &["lpop varName ?index ...?"],
            snippet: "lpop operates on a list stored in the variable varName, which must already exist. It accepts one or more index arguments; if none are given, index defaults to end, so a bare lpop varName removes and returns the last element. With a single index, lpop removes the addressed element from the list and returns it, leaving the shortened list stored back in varName. index accepts the same syntax as string index — a plain integer, end, or an arithmetic end-relative form such as end-1 — and it is an error for index to be negative or greater than or equal to the number of elements in the list. When more than one index is given, each one after the first addresses an element within the sublist selected by the previous index, letting lpop remove an element from an arbitrarily nested list the same way lindex and lset descend into one.",
            source: "Tcl lpop(n)",
            examples: "set letters {a b c d e}\nlpop letters      ;# returns e; letters becomes \"a b c d\"\nlpop letters 0     ;# returns a; letters becomes \"b c d\"\n\nset matrix {{1 2 3} {4 5 6} {7 8 9}}\nlpop matrix 1 2     ;# returns 6; matrix becomes \"{1 2 3} {4 5} {7 8 9}\"",
            return_value: "The element that was removed from the list.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
