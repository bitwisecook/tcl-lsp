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

//! `lassign` — assign list elements to variables.
use crate::hooks::CodegenHookId;
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Variable,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lassign list ?varName ...?",
}];

/// D4-F2: `lassign list ?varName ...?` accepts variable-name args from index 1
/// onward to the end of the call.  Resolve `VarWrite` dynamically so calls with
/// arbitrarily many vars don't false-fire W210 on the unmodelled tail.
fn lassign_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    (1..args.len())
        .filter_map(|i| u8::try_from(i).ok().map(|i| (i, ArgRole::VarWrite)))
        .collect()
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lassign",
        traits: Traits::FRAMELESS_RUNTIME | Traits::FRAME_HASH_BUILTIN,
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        // `lassign` writes list *elements* to its targets — of any intrep —
        // while returning the *leftover* list.  The elements are not the
        // return value, so they must not be typed `List` (issue #867).
        var_write_typing: VarWriteTyping::ElementsOf { container_arg: 0 },
        hover: Some(HoverSnippet {
            summary: "Assign list elements to variables",
            synopsis: &["lassign list ?varName ...?"],
            snippet: "This command treats the value list as a list and assigns successive elements from that list to the variables given by the varName arguments in order.",
            source: "Tcl man page lassign.n",
            examples: "",
            return_value: "",
        }),
        codegen_hook: Some(CodegenHookId::Lassign),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        arg_role_resolver: Some(lassign_arg_roles),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        ..CommandSpec::DEFAULT
    }
}
