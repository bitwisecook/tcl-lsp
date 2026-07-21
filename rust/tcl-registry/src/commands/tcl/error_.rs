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

//! `error` — generate an error.

use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "error message ?info? ?code?",
}];

/// Command spec for `error`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "error",
        // `LANGUAGE_KEYWORD`, like its sibling `throw`: both raise an exception
        // and both are `TERMINATES_BLOCK`. `error` carried neither the trait nor
        // any keyword colouring, so `catch { error boom }` painted `catch` as a
        // control keyword and `error` as an ordinary library call — the two
        // halves of one construct in two different colours (issue #904).
        //
        // Every Tcl grammar that *has* a function category agrees `error` is not
        // one: Pygments lists it under `Keyword`, tree-sitter under `@keyword`,
        // Zed under `@operator`, and the TextMate bundle under `keyword.other`.
        // Only grammars with no function bucket at all put it with the builtins.
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::TERMINATES_BLOCK
            | Traits::CATCHABLE_THROW
            | Traits::NEEDS_START_CMD,
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Generate an error",
            synopsis: &["error message ?info? ?code?"],
            snippet: "Returns a TCL_ERROR code, which causes command interpretation to be unwound.",
            source: "Tcl man page error.n",
            examples: "",
            return_value: "",
        }),
        inline_codegen_hook: Some(InlineCodegenHookId::Error),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
