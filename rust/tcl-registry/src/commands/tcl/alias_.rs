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

//! Jim command-prefix aliases.
use crate::prelude::*;

/// The measured current Jim native alias surface.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "alias",
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Direct,
        }),
        surface: Some(tcl_dialect::surface![
            tcl_dialect::model::SpecSurface::core_in(
                tcl_dialect::model::Family::Jim,
                &[("0.84", None)]
            )
        ]),
        hover: Some(HoverSnippet {
            summary: "Create a Jim command alias with a retained argument prefix.",
            synopsis: &["alias name command ?args ...?"],
            snippet: "Creates a command alias for one or more words. Invocation appends its arguments to the retained command prefix. Returns the alias name, which can be used with local.",
            source: "https://jim.tcl.tk/home/doc/trunk/jim_tcl.txt#_alias",
            examples: "alias e info exists\nif {[e var]} {puts present}",
            return_value: "Returns the new alias name.",
        }),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Name)],
        return_type: Some(TclType::String),
        traits: Traits::INSTALLS_NAMED_DEFINITION | Traits::REFLECTS_COMMAND_NAMES,
        world_effects: Some(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(crate::state_transition::command_binding::CREATES_CALLER_ALIASES),
        forms: &[FormSpec {
            synopsis: "alias newname command ?args ...?",
            ..FormSpec::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}
