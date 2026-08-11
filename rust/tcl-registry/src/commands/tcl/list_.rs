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
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "list",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        const_fold: Some(crate::const_fold::fold_list),
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::PRODUCES_CANONICAL_LIST
            | Traits::EXPANSION_ESCAPE_SAFE
            | Traits::BUILDS_COMMAND_PREFIX,
        arity: Arity::any(),
        return_type: Some(TclType::List),
        return_elements: Some(ReturnElements::ListOfArgs { from: 0 }),
        hover: Some(HoverSnippet {
            summary: "Create a list",
            synopsis: &["list ?arg arg ...?"],
            snippet: "Every argument becomes exactly one element of the result, with braces or backslashes inserted wherever needed so that lindex can recover the original argument unchanged and eval can run the result as a command — the first element as the command name and the rest as its arguments. This is what distinguishes list from concat: concat removes one level of list structure before joining its arguments, while list always treats each argument as a single opaque element, even one containing embedded whitespace or list-special characters such as braces. Called with no arguments, list returns the empty string.",
            source: "Tcl list(n)",
            examples: "list a b c\nlist a b \"c d e  \" \"  f {g h}\"\nlist",
            return_value: "A list containing one element per argument, in order, or the empty string when no arguments are given.",
        }),
        inline_codegen_hook: Some(InlineCodegenHookId::List),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
