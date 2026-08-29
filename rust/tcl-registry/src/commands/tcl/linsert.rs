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

//! `linsert` — insert elements into a list.
//
// Manpage comparison across Tcl 8.4, 8.5, 8.6, 9.0, and 9.1
// (tcl-lang.org's TclCmd/linsert.html, `.htm` for the 8.6 tree): 8.4 and
// 8.5 require at least one `element` in the synopsis ("list index
// element ?element element ...?"), while 8.6 loosens this so `element`
// is fully optional ("list index ?element element ...?") — a bare
// `linsert list index` becomes a legal no-op that returns an unmodified
// copy of `list`. Unchanged through 9.0 and 9.1, and already how
// `tcl-vm/src/cmd_list.rs::cmd_linsert` matches its arguments (`[list,
// index, elems @ ..]`, `elems` possibly empty) regardless of dialect.
// The 8.6+ manpage also adds a paragraph spelling out that a
// start-relative `index` positions the *first* inserted element while
// an end-relative one positions the *last*; nothing indicates the
// underlying placement itself changed pre-8.6, only that the wording
// became more explicit, so that description is folded into the hover
// snippet as plain, version-independent prose rather than a dialect
// gate. Not disabled or overridden in any modelled dialect — `linsert`
// carries the `IRULES` bit explicitly (its `dialects` group is
// `ALL_TCL.union(IRULES)`, so it resolves under the bare `IRULES` mask),
// and no irules/iapps/tk/expect/eda/itcl override file exists — so the
// command-level group is correct as-is.
use crate::hooks::{CodegenHookId, InlineCodegenHookId};
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};
use tcl_dialect::surface;
use tcl_dialect::model::Family;

const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "linsert list index ?element element ...?",
        surface: Some(SpecSurface::TCL86_PLUS),
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "linsert list index element ?element element ...?",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.4", Some("8.6"))])]),
        ..FormSpec::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "linsert",
        surface: Some(surface![SpecSurface::core_in(Family::Tcl, &[("8.4", Some("9.2"))]), SpecSurface::core(Family::F5Irules)]),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(2),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Insert elements into a list",
            synopsis: &[
                "linsert list index ?element element ...?",
                "linsert list index element ?element element ...?",
            ],
            snippet: "Produces a new list from list by inserting the element arguments as separate new elements just before the index'th element; list itself is left unmodified. If index is less than or equal to zero, the new elements go at the beginning of the list; if index is end, or is greater than or equal to the number of elements in list, they are appended to the end. index takes the same forms as string index: a plain integer, end, or end-integer/end+integer arithmetic. When index counts from the start, the first inserted element ends up at that index in the result; when index counts from the end (e.g. end-1), the last inserted element ends up at that index instead. Before Tcl 8.6, at least one element argument was required; Tcl 8.6 and later also accept linsert list index with no element arguments at all, which is a no-op that returns an unmodified copy of list.",
            source: "Tcl linsert(n)",
            examples: "set oldList {the fox jumps over the dog}\nset midList [linsert $oldList 1 quick]\nset newList [linsert $midList end-1 lazy]",
            return_value: "A new list consisting of every element of list, with the element arguments inserted before index.",
        }),
        codegen_hook: Some(CodegenHookId::Linsert),
        inline_codegen_hook: Some(InlineCodegenHookId::Linsert),
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
