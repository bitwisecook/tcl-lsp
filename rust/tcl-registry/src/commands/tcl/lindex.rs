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

//! `lindex` — retrieve an element from a list.
use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lindex list ?index ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lindex",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        const_fold: Some(crate::const_fold::fold_lindex),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        return_elements: Some(ReturnElements::ElementOf { container_arg: 0 }),
        hover: Some(HoverSnippet {
            summary: "Retrieve an element (or nested sublist) from a list by index.",
            synopsis: &["lindex list ?index ...?"],
            snippet: "Treats list as a Tcl list and returns the index'th element (0 refers to the first element). With no index, or a single empty-string index, the return value is list itself, unchanged. index accepts a non-negative or negative integer, end, end-integer, or — since Tcl 8.5 — the fuller index-arithmetic syntax of `string index`, such as end+1 or $n+2 (each index takes at most one +/- offset — end-1+1 is not valid syntax); Tcl 8.4 supports only a bare integer or the fixed end-integer form. An index that is negative or greater than or equal to the list's length returns an empty string rather than raising an error. Extraction follows the same brace/quote/backslash rules as the Tcl parser, but performs no variable or command substitution, so an element that looks like $x or [foo] is returned literally. Additional index arguments are applied in turn to the result of the previous step, descending into nested sublists — lindex $a 1 2 is equivalent to lindex [lindex $a 1] 2 — and may be given as separate words or grouped into a single list argument, e.g. lindex $a {1 2}.",
            source: "Tcl lindex(n)",
            examples: "set colors {red green blue yellow}\nlindex $colors 0      ;# red\nlindex $colors end    ;# yellow\nlindex $colors end-1  ;# blue\nlindex $colors        ;# red green blue yellow -- no index returns the list unchanged\n\nset matrix {{1 2 3} {4 5 6}}\nlindex $matrix 1 2    ;# 6\nlindex $matrix {1 2}  ;# 6 -- indices grouped into a single list argument",
            return_value: "The selected element, or nested sublist when multiple indices are given; the list itself, unchanged, when no index is given; or an empty string when an index is out of range.",
        }),
        inline_codegen_hook: Some(InlineCodegenHookId::Lindex),
        forms: FORMS,
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
        ..CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
    }
}
