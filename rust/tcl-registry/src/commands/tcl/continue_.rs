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

//! `continue` — skip to the next iteration of a loop.

use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "continue",
}];

/// Command spec for `continue`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "continue",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CONTINUES_LOOP
            | Traits::TRANSFERS_CONTROL
            | Traits::NEEDS_START_CMD,
        arity: Arity::exact(0),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Skip to the next iteration of a loop",
            synopsis: &["continue"],
            snippet: "This command is typically invoked inside the body of a looping command such as for or foreach or while.",
            source: "Tcl man page continue.n",
            examples: "",
            return_value: "",
        }),
        inline_codegen_hook: Some(InlineCodegenHookId::Continue),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
