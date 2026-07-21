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

//! `proc` iRules command.
//!
//! Structurally identical to Tcl's `proc` — same arity, same
//! argument roles. Carrying the same `arg_roles` here means when
//! the iRules dialect is loaded into a shared registry, body-role
//! lookups (folding, document symbols, …) keep finding the body at
//! index 2 instead of falling off because the iRules override
//! shadows the Tcl spec with empty roles.
use crate::hooks::LoweringHookId;
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "proc",
        traits: Traits::DEFINES_PROCEDURE.union(Traits::NEVER_INLINE_BODY),
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(3),
        arg_roles: &[
            (0, ArgRole::Name),
            (1, ArgRole::ParamList),
            (2, ArgRole::Body),
        ],
        lowering_hook: Some(LoweringHookId::Proc),
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "Define an iRule proc.",
            synopsis: &["proc NAME ARGUMENT_N_DEFAULT PROC_SCRIPT"],
            snippet: "Define an iRule proc which is called by iRule command call.\n\nThe syntax is same as basic TCL proc command.",
            source: "https://clouddocs.f5.com/api/irules/proc.html",
            examples: "when CLIENT_DATA {\n    call logme \"Coming to CLIENT_DATA\"\n}",
            return_value: "Returns the value in the return command, if any, in the proc script.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "proc NAME ARGUMENT_N_DEFAULT PROC_SCRIPT",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ProcDefinition,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Global,
            dialects: None,
        }],
        analyser_hook: Some(crate::hooks::AnalyserHookId::Proc),
        command_table_effect: Some(crate::command_table::CommandTableEffect::DefinesProcedure),
        ..CommandSpec::DEFAULT
    }
}
