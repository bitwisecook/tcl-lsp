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
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lindex",
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
            summary: "Retrieve an element from a list",
            synopsis: &["lindex list ?index ...?"],
            snippet: "The lindex command accepts a parameter, list, which it treats as a Tcl list.",
            source: "Tcl man page lindex.n",
            examples: "",
            return_value: "",
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
        ..CommandSpec::DEFAULT
    }
}
