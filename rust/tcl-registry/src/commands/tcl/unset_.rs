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

//! `unset` — delete variables.

use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "unset ?-nocomplain? ?--? ?name name name ...?",
}];

/// Command spec for `unset`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unset",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::DESTROYS_VARIABLE
            | Traits::FIRST_ARG_VARNAME,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        return_type: Some(TclType::String),
        options: const { &[
            OptionSpec {
                name: "-nocomplain",
                value: OptionValue::flag(),
                detail: "Suppress errors for non-existent variables.",
                dialects: None,
                aliases: &[],
                min_version: None,
            },
            OptionSpec {
                name: "--",
                value: OptionValue::flag(),
                detail: "End of options.",
                dialects: None,
                aliases: &[],
                min_version: None,
            },
        ] },
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Delete variables",
            synopsis: &["unset ?-nocomplain? ?--? ?name name name ...?"],
            snippet: "This command removes one or more variables.",
            source: "Tcl man page unset.n",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Unset),
        codegen_hook: Some(CodegenHookId::Unset),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
