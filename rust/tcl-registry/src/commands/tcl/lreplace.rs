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

//! `lreplace` — replace elements in a list.
use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lreplace list first last ?element element ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lreplace",
        traits: Traits::FRAMELESS_RUNTIME | Traits::BYTE_COMPILED | Traits::PURE,
        arity: Arity::at_least(3),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Replace elements in a list with new elements",
            synopsis: &[
                "lreplace list first last ?element element ...?",
                "lreplace list first last ?element ...?",
            ],
            snippet: "lreplace returns a new list formed by replacing zero or more elements of list with the element arguments.",
            source: "Tcl man page lreplace.n",
            examples: "",
            return_value: "",
        }),
        inline_codegen_hook: Some(InlineCodegenHookId::Lreplace),
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
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                    transparent_from: &[],
                },
            ),
        ],
        ..CommandSpec::DEFAULT
    }
}
