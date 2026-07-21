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

//! `tailcall` — replace the current procedure with another command.

use crate::hooks::CodegenHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tailcall command ?arg ...?",
    dialects: None,
}];

/// Command spec for `tailcall`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tailcall",
        traits: Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::REPLACES_FRAME
            | Traits::TRANSFERS_CONTROL,
        dialects: Some(DialectSet::TCL86_PLUS),
        // Tcl 9.0: ``tailcall`` with no args clears any scheduled
        // tailcall; with args it replaces it.  Real arity is 0..∞.
        arity: Arity::any(),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Replace the current procedure with another command",
            synopsis: &["tailcall command ?arg ...?"],
            snippet: "The tailcall command replaces the currently executing procedure, lambda application, or method with another command.",
            source: "Tcl man page tailcall.n",
            examples: "",
            return_value: "",
        }),
        codegen_hook: Some(CodegenHookId::Tailcall),
        // `tailcall command ?arg …?` — arg 0 is the command to become, invoked
        // with the trailing args appended (a variable count).  Declaring it a
        // command prefix makes references / go-to-definition / rename see the
        // proc named in a `tailcall` the same as a direct call.
        command_prefixes: &[(0, AppendedArity::Unknown)],
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
