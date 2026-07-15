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

//! `return` — return from the current procedure or script.

use crate::hooks::{InlineCodegenHookId, LoweringHookId};
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "return ?-code code? ?-level level? ?result?",
}];

/// Command spec for `return`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "return",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::TERMINATES_BLOCK
            | Traits::NEEDS_START_CMD,
        arity: Arity::any(),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Return from the current procedure/script with optional control-code metadata.",
            synopsis: &["return ?-code code? ?-level level? ?result?"],
            snippet: "Advanced forms can emulate `break`, `continue`, or custom return codes.",
            source: "Tcl return(1)",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Return),
        inline_codegen_hook: Some(InlineCodegenHookId::Return),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
