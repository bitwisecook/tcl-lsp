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

//! `coroutine` — create a coroutine.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "coroutine name command ?arg...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroutine",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Name)],
        // `coroutine NAME cmd ?arg …?` creates the command NAME
        // (`TclNRCoroutineObjCmd`, `tclBasic.c`) — later calls to a literal
        // NAME resolve to the coroutine, not an unknown command.
        defines_command_at: Some(0),
        // Arg 1 is the command the coroutine runs, invoked with the trailing
        // args appended (a variable count) — a command prefix, so the proc it
        // names is seen by references / go-to-definition / rename.
        command_prefixes: &[(1, AppendedArity::Unknown)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Create and produce values from coroutines",
            synopsis: &[
                "coroutine name command ?arg...?",
                "coroutine name command ?arg ...?",
            ],
            snippet: "The coroutine command creates a new coroutine context (with associated command) named name and executes that context by calling command, passing in the other remaining arguments without further interpretation.",
            source: "Tcl man page coroutine.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
