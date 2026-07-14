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

//! `for` — C-style loop with init, test, and next scripts.

use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::Unknown,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "for start test next body",
}];

/// Command spec for `for`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "for",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::CONTROL_FLOW
            | Traits::LANGUAGE_KEYWORD
            | Traits::HAS_BOOLEAN_COND
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY,
        arity: Arity::exact(4),
        arg_roles: &[
            (0, ArgRole::Body),
            (1, ArgRole::Expr),
            (2, ArgRole::Body),
            (3, ArgRole::Body),
        ],
        lowering_hook: Some(crate::hooks::LoweringHookId::For),
        return_type: Some(TclType::String),
        arg_types: &[(
            1,
            ArgTypeHint {
                expected: Some(TclType::Boolean),
                shimmers: true,
                transparent_from: &[],
            },
        )],
        hover: Some(HoverSnippet {
            summary: "C-style loop with init, test, and next scripts.",
            synopsis: &["for start test next body"],
            snippet: "`start` runs once; loop continues while `test` is true; `next` runs after each body pass.",
            source: "Tcl for(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        analyser_hook: Some(crate::hooks::AnalyserHookId::For),
        ..CommandSpec::DEFAULT
    }
}
