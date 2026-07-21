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

//! `coroprobe` — probe a suspended coroutine.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "coroprobe coroName command ?arg ...?",
    dialects: None,
}];

// `coroprobe coroName command ?arg...?` evaluates an arbitrary command *now* in
// the paused coroutine's context and returns its result — so it runs unknown
// code with unknown reads/writes, matching `eval` / `uplevel`.
static SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroprobe",
        traits: Traits::EVALUATES_CODE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(2),
        // `coroprobe coroName command ?arg ...?` — `command` (index 1) is a
        // command prefix run in the coroutine's context with runtime args
        // appended (variadic ⇒ Unknown: a reference, not arity-checked).
        command_prefixes: &[(1, AppendedArity::Unknown)],
        return_type: Some(TclType::String),
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Evaluate a command in a suspended coroutine.",
            synopsis: &["coroprobe coroName command ?arg ...?"],
            snippet: "",
            source: "Tcl coroprobe(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
