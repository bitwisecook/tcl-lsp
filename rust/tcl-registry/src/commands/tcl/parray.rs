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

//! `parray` — print an array's contents to standard output.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// (tcl-lang.org's TclCmd/library.html, `.htm` for the 8.6 tree — `parray`
// has no manual page of its own; it is documented inside library.n
// alongside auto_load/auto_execok/tcl_findLibrary/etc.): the 8.4 SYNOPSIS
// is `parray arrayName` with no pattern argument at all, and the 8.4 body
// text only documents printing every element ("Prints on standard output
// the names and values of all the elements in the array arrayName"). Tcl
// 8.5 widens the SYNOPSIS to `parray arrayName ?pattern?` and the body
// text to "...or just the names that match pattern (using the matching
// rules of string match) and their values if pattern is given" —
// unchanged through 8.6, 9.0, and 9.1. Tcl 8.6 additionally appends a
// "For example, to print the contents of the tcl_platform array, do:
// parray tcl_platform" paragraph (documentation polish only, no behaviour
// change). Tcl 9.0 is the first tree whose body text adds the explicit
// sentence "The result of this command is the empty string." (kept in
// 9.1 unchanged); the actual return value was already always the empty
// string in every version — the library implementation's last statement
// is an unconditional `foreach` loop with no `return` — so that fact is
// stated as current, version-independent truth in the hover text below
// rather than gated to 9.0+.
//
// Excluded from f5-irules, like every *other* library.n sibling documented on
// the same manpage (auto_execok, auto_import, auto_load, auto_mkindex,
// auto_mkindex_old, auto_qualify, auto_reset, tcl_findLibrary): `parray`'s own
// command-level surface below is `Some(SpecSurface::ALL_TCL)`, which names
// only Tcl and so is simply absent under iRules — there is no disable list. It
// stays reachable in every real-Tcl-hosting dialect (whose mask carries a
// release row `ALL_TCL` intersects), and no irules/iapps/tk/expect/eda/itcl
// override file registers a variant of its own. `parray` is also a Tcl-level
// library proc — shipped as its own `library/parray.tcl` (confirmed
// internally: both `runtime/rust/src/embedded_stdlib.rs`'s seed file list and
// `xtask/src/command_backing.rs`'s `STDLIB` table name it separately from
// `init.tcl`) — not a `Tcl_CreateObjCommand`-registered builtin, so it carries
// no `CmdInfo` row and is absent from the exact C Tcl safe-interpreter
// hidden-command table `Traits::SAFE_INTERP_HIDDEN` documents; that trait does
// not apply here, matching `auto_execok`'s identical reasoning.
const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "parray arrayName ?pattern?",
        surface: Some(SpecSurface::TCL85_PLUS),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "parray arrayName",
        surface: Some(SpecSurface::TCL84),
        ..FormSpec::DEFAULT
    },
];

/// Command spec for `parray`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "parray",
        traits: Traits::WHOLE_ARRAY_ARG | Traits::OVERRIDABLE_LIBRARY_PROC,
        surface: Some(SpecSurface::ALL_TCL),
        // Real 8.5+ ceiling (`arrayName ?pattern?`); Tcl 8.4's SYNOPSIS is
        // `parray arrayName` alone (see the FORMS comment above), so a
        // narrow gap remains for arity diagnostics against code
        // specifically targeting Tcl 8.4 that calls `parray` with a
        // pattern argument — `Arity` has no per-dialect axis of its own
        // (the same documented trade-off `lassign`/`linsert` make).
        arity: Arity::new(1, 2),
        // Position 1 (`pattern`) mirrors `chan names ?pattern?`'s
        // identically-shaped optional glob filter: `ArgRole::Pattern` +
        // `PatternType::Glob`, matching the manpage's own "using the
        // matching rules of string match" wording.
        arg_roles: &[(0, ArgRole::VarRead), (1, ArgRole::Pattern)],
        pattern_type: Some(PatternType::Glob),
        return_type: Some(TclType::String),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                writes: true,
                ..SideEffect::DEFAULT
            },
            // Reads the array VARIABLE.
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: true,
                ..SideEffect::DEFAULT
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Print an array's names and values to standard output.",
            synopsis: &["parray arrayName ?pattern?", "parray arrayName"],
            snippet: "Prints every name/value pair in arrayName to stdout, one pair per line as `arrayName(elementName) = value`, with the name field padded so every `=` lines up in a column; names are visited in sorted order. arrayName must already be an array accessible to the caller — local or global — since parray reaches it via upvar. Since Tcl 8.5, an optional trailing pattern restricts the printed elements to those whose names match pattern under string match's glob rules; Tcl 8.4 has no pattern argument and always prints every element. Output always goes to stdout; there is no option to target a different channel. parray is a standard-library Tcl procedure, autoloaded like foreachLine or the auto_* helpers, and a script may freely redefine it.",
            source: "Tcl library(n)",
            examples: "parray tcl_platform             ;# prints every tcl_platform element\nparray tcl_platform os*         ;# Tcl 8.5+: prints only names matching the glob os* (e.g. os, osVersion)",
            return_value: "The empty string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
