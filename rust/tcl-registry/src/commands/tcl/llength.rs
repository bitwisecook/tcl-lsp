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

//! `llength` — return the number of elements in a list.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "llength list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "llength",
        const_fold: Some(crate::const_fold::fold_llength),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        arity: Arity::exact(1),
        return_type: Some(TclType::Int),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "Count the number of elements in a list",
            synopsis: &["llength list"],
            snippet: "Treats list as a list and returns a decimal string giving the number of elements in it.",
            source: "Tcl man page llength.n",
            examples: "",
            return_value: "",
        }),
        codegen_hook: Some(CodegenHookId::Llength),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
