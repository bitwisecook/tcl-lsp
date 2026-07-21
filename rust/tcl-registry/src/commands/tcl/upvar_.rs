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

//! `upvar` — create link to variable in a different stack frame.

use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "upvar ?level? otherVar myVar ?otherVar myVar ...?",
    dialects: None,
}];

/// Command spec for `upvar`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "upvar",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::CREATES_SCOPE_ALIAS
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::FRAME_HASH_BUILTIN,
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Create link to variable in a different stack frame",
            synopsis: &["upvar ?level? otherVar myVar ?otherVar myVar ...?"],
            snippet: "This command arranges for one or more local variables in the current procedure to refer to variables in an enclosing procedure call or to global variables.",
            source: "Tcl man page upvar.n",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Upvar),
        codegen_hook: Some(CodegenHookId::Upvar),
        forms: FORMS,
        xc_translatable: Some(false),
        analyser_hook: Some(crate::hooks::AnalyserHookId::Upvar),
        ..CommandSpec::DEFAULT
    }
}
