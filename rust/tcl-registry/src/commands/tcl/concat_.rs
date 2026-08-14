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

//! `concat` — concatenate lists.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "concat ?arg arg ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "concat",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        byte_array_effect: ByteArrayEffect::Coerces,
        const_fold: Some(crate::const_fold::fold_concat),
        codegen_hook: Some(CodegenHookId::Concat),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::PURE
            | Traits::PRODUCES_CANONICAL_LIST
            | Traits::EXPANSION_ESCAPE_SAFE
            | Traits::BYTE_COMPILED,
        arity: Arity::any(),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Join lists together",
            synopsis: &["concat ?arg arg ...?"],
            snippet: "Joins every argument together with a single space, after trimming leading and trailing white-space from each one. If all of the arguments are lists, this has the same effect as concatenating them into a single list. Arguments that are empty after trimming are dropped entirely rather than leaving a stray separating space (documented from Tcl 8.6 on; the 8.4/8.5 manual pages describe only the trim-and-join rule). Whitespace in the interior of an argument is left untouched, so multiple internal spaces survive the join. Because concat splices raw argument text rather than list structure, unbalanced braces or quotes spanning an argument boundary can produce a malformed result; for guaranteed list-safe concatenation use `list {*}listA {*}listB` instead (the official recommendation as of the Tcl 9.0 manual page).",
            source: "Tcl concat(n)",
            examples: "concat a b {c d e} {f {g h}}\nconcat \" a b {c   \" d \"  e} f\"\nconcat \"a   b   c\" { d e f }\nlist {*}\"a   b   c\" {*}{ d e f }",
            return_value: "The concatenated string, or the empty string when no arguments are given.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
