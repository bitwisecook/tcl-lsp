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

//! `::tcl::unsupported::grapheme` — extended grapheme cluster operations.
//!
//! New in Tcl 9.1.0 (`generic/tclGrapheme.c`, `tests/grapheme.test`); absent
//! from 9.1b0 and every earlier release. An ensemble over
//! `tclGraphemeImplMap`, registered `CMD_IS_SAFE` in `tclBasic.c`, with no
//! manual page. Arities and wrong-# args texts are tclsh 9.1.0's. Both
//! spellings are registered, as for `corotype`.
use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "::tcl::unsupported::grapheme subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

const VAR_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

const fn pure_sub(
    name: &'static str,
    arity: Arity,
    synopsis: &'static str,
    detail: &'static str,
    return_type: TclType,
) -> SubCommand {
    SubCommand {
        name,
        arity,
        detail,
        synopsis,
        pure: true,
        return_type: Some(return_type),
        ..SubCommand::DEFAULT
    }
}

// `next`/`prev` read `indexVar` as a character index (unset reads as 0 for
// `next`), then write the index past/before the returned cluster.
const fn step_sub(name: &'static str, synopsis: &'static str, detail: &'static str) -> SubCommand {
    SubCommand {
        name,
        arity: Arity::exact(2),
        detail,
        synopsis,
        arg_roles: &[(1, ArgRole::VarWrite)],
        mutator: true,
        safe_on_uninit: Some(SpecSurface::TCL91),
        side_effects: VAR_EFFECTS,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    }
}

static SUBCOMMANDS: [SubCommand; 8] = [
    pure_sub(
        "index",
        Arity::exact(2),
        "grapheme index string grIndex",
        "Return the grapheme cluster at grapheme index grIndex (an index expression such as end-1).",
        TclType::String,
    ),
    pure_sub(
        "length",
        Arity::exact(1),
        "grapheme length string",
        "Return the number of grapheme clusters in string.",
        TclType::Int,
    ),
    step_sub(
        "next",
        "grapheme next string indexVar",
        "Return the grapheme cluster starting at character index $indexVar (0 when unset) and store the character index just past it; at the end of string return an empty string.",
    ),
    pure_sub(
        "offset",
        Arity::exact(2),
        "grapheme offset string grIndex",
        "Return the character index where grapheme cluster grIndex starts, or -1 when it is out of range.",
        TclType::Int,
    ),
    step_sub(
        "prev",
        "grapheme prev string indexVar",
        "Return the grapheme cluster ending just before character index $indexVar and store its starting character index; at the start of string return an empty string.",
    ),
    pure_sub(
        "range",
        Arity::exact(3),
        "grapheme range string grFirst grLast",
        "Return the substring covering grapheme clusters grFirst through grLast.",
        TclType::String,
    ),
    pure_sub(
        "reverse",
        Arity::exact(1),
        "grapheme reverse string",
        "Return string with its grapheme clusters in reverse order, keeping each cluster intact.",
        TclType::String,
    ),
    pure_sub(
        "split",
        Arity::exact(1),
        "grapheme split string",
        "Return a list of the grapheme clusters in string.",
        TclType::List,
    ),
];

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        surface: Some(SpecSurface::TCL91),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        forms: FORMS,
        hover: Some(HoverSnippet {
            summary: "Operate on a string's extended grapheme clusters (Tcl 9.1.0).",
            synopsis: &[
                "::tcl::unsupported::grapheme index string grIndex",
                "::tcl::unsupported::grapheme length string",
                "::tcl::unsupported::grapheme next string indexVar",
                "::tcl::unsupported::grapheme offset string grIndex",
                "::tcl::unsupported::grapheme prev string indexVar",
                "::tcl::unsupported::grapheme range string grFirst grLast",
                "::tcl::unsupported::grapheme reverse string",
                "::tcl::unsupported::grapheme split string",
            ],
            snippet: "New in Tcl 9.1.0 and undocumented: it lives in ::tcl::unsupported and has no manual page. Where the string command counts characters (code points), grapheme counts user-perceived characters — Unicode extended grapheme clusters — so a base letter with combining marks, or a flag's regional-indicator pair, is one unit. index, offset and range take grapheme indices (including end-relative forms); next and prev instead walk a character-index cursor held in a variable, returning one cluster per call and an empty string once they run off either end. It is registered as safe, so it is available in safe interpreters.",
            source: "Tcl ::tcl::unsupported::grapheme (undocumented; generic/tclGrapheme.c)",
            examples: "namespace path ::tcl::unsupported\nset s \"e\\u0301x\\U1F1EB\\U1F1F7\"\nputs [string length $s]      ;# 5\nputs [grapheme length $s]    ;# 3\nputs [grapheme offset $s 2]  ;# 3\n\nset i 0\nwhile {$i < [string length $s]} {\n    lappend clusters [grapheme next $s i]\n}",
            return_value: "Depends on the subcommand: a cluster or substring, a count, a character index (-1 when out of range), or a list of clusters.",
        }),
        ..CommandSpec::DEFAULT
    }
}

/// Command spec for the namespace-relative form `tcl::unsupported::grapheme`.
pub fn spec() -> CommandSpec {
    make_spec("tcl::unsupported::grapheme")
}

/// Command spec for the fully-qualified form `::tcl::unsupported::grapheme`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::unsupported::grapheme")
}
