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

//! `list` — create a Tcl list.
use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "list ?arg arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "list",
        const_fold: Some(crate::const_fold::fold_list),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::PRODUCES_CANONICAL_LIST,
        arity: Arity::any(),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Create a list",
            synopsis: &["list ?arg arg ...?", "list ?arg ...?"],
            snippet: "This command returns a list comprised of all the args, or an empty string if no args are specified.",
            source: "Tcl man page list.n",
            examples: "",
            return_value: "",
        }),
        inline_codegen_hook: Some(InlineCodegenHookId::List),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
