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
use crate::hooks::{CodegenHookId, InlineCodegenHookId};
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "linsert list index ?element element ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "linsert",
        traits: Traits::FRAMELESS_RUNTIME | Traits::BYTE_COMPILED | Traits::PURE,
        arity: Arity::at_least(2),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Insert elements into a list",
            synopsis: &[
                "linsert list index ?element element ...?",
                "linsert list index ?element ...?",
            ],
            snippet: "This command produces a new list from list by inserting all of the element arguments just before the index'th element of list.",
            source: "Tcl man page linsert.n",
            examples: "",
            return_value: "",
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
        ..CommandSpec::DEFAULT
    }
}
