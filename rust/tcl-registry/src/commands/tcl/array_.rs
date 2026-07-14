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

//! `array` — manipulate array variables.

use crate::hooks::{CodegenHookId, InlineCodegenHookId};
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "array option arrayName ?arg arg ...?",
}];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "anymore",
        arity: Arity::exact(2),
        detail: "Returns 1 if there are any more elements left to be processed in an array search, 0 if all elements have already been returned.",
        synopsis: "array anymore arrayName searchId",
        return_type: Some(TclType::Boolean),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "default",
        arity: Arity::at_least(2),
        detail: "Manages the default value of the array.",
        synopsis: "array default subcommand arrayName args...",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        dialects: Some(DialectSet::TCL90_PLUS),
        arg_values: &[(
            0,
            &[
                ArgValue {
                    value: "exists",
                    detail: "Returns a boolean indicating whether a default value has been set for the array.",
                },
                ArgValue {
                    value: "get",
                    detail: "Returns the current default value for the array.",
                },
                ArgValue {
                    value: "set",
                    detail: "Sets the default value for the array to value.",
                },
                ArgValue {
                    value: "unset",
                    detail: "Removes the default value for the array.",
                },
            ],
        )],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "donesearch",
        arity: Arity::exact(2),
        detail: "Terminates an array search and destroys all the state associated with that search.",
        synopsis: "array donesearch arrayName searchId",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Returns 1 if arrayName is an array variable, 0 if there is no variable by that name or if it is a scalar variable.",
        synopsis: "array exists arrayName",
        return_type: Some(TclType::Boolean),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "for",
        arity: Arity::exact(3),
        detail: "Iterates over array entries. The first argument is a two-element list of variable names for the key and value of each entry.",
        synopsis: "array for {keyVariable valueVariable} arrayName body",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarRead), (2, ArgRole::Body)],
        lowering_hook: Some(crate::hooks::LoweringHookId::ArrayFor),
        loop_list_header: true,
        dialects: Some(DialectSet::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::new(1, 2),
        detail: "Returns a list containing pairs of elements.",
        synopsis: "array get arrayName ?pattern?",
        return_type: Some(TclType::List),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::new(1, 3),
        detail: "Returns a list containing the names of all of the elements in the array that match pattern.",
        synopsis: "array names arrayName ?mode? ?pattern?",
        return_type: Some(TclType::List),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nextelement",
        arity: Arity::exact(2),
        detail: "Returns the name of the next element in arrayName, or an empty string if all elements have already been returned in this search.",
        synopsis: "array nextelement arrayName searchId",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::exact(2),
        detail: "Sets the values of one or more elements in arrayName.",
        synopsis: "array set arrayName list",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarWrite)],
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        arity: Arity::exact(1),
        detail: "Returns a decimal string giving the number of elements in the array.",
        synopsis: "array size arrayName",
        return_type: Some(TclType::Int),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "startsearch",
        arity: Arity::exact(1),
        detail: "Initializes an element-by-element search through the array given by arrayName.",
        synopsis: "array startsearch arrayName",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "statistics",
        arity: Arity::exact(1),
        detail: "Returns statistics about the distribution of data within the hashtable that represents the array.",
        synopsis: "array statistics arrayName",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unset",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::new(1, 2),
        detail: "Unsets all of the elements in the array that match pattern.",
        synopsis: "array unset arrayName ?pattern?",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarWrite)],
        mutator: true,
        // `Tcl_ArrayObjCmd` (tclVar.c, `ArrayUnsetCmd`) destroys matching
        // elements — or the whole array in the pattern-less form — via the
        // same `TclObjUnsetVar2` machinery as `unset`; a bare
        // `catch {array unset a …}` is the documented fire-and-forget
        // idiom the W302 suppression keys off.
        destructive: true,
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `array`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "array",
        // `FIRE_AND_FORGET_TEARDOWN` (on the subform below where noted): the `unset` subform destroys elements or
        // the whole array (`ArrayUnsetCmd`, tclVar.c) — see the
        // `destructive` flag on the `unset` subcommand.
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::WHOLE_ARRAY_ARG
           ,
        arity: Arity::at_least(1),
        assigns_variable_at: Some(1),
        inferred_storage_type: Some(StorageType::Array),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Manipulate array variables",
            synopsis: &["array option arrayName ?arg arg ...?"],
            snippet: "This command performs one of several operations on the variable given by arrayName.",
            source: "Tcl man page array.n",
            examples: "",
            return_value: "",
        }),
        codegen_hook: Some(CodegenHookId::Array),
        inline_codegen_hook: Some(InlineCodegenHookId::Array),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
