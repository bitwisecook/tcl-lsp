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

//! `TclOO` object.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "oo::object method ?arg ...?",
}];

/// The universal object-method surface every `TclOO` object (class or
/// instance) exposes.  `destroy` is the registry-declared destructive
/// method the command-binding lattice consults: `NAME destroy` deletes
/// the `NAME` command, so later dispatches fall through to `unknown`.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "destroy",
        arity: Arity::exact(0),
        detail: "Delete the object (a class's instances are deleted with it) and its command.",
        synopsis: "obj destroy",
        destructive: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "new",
        arity: Arity::at_least(0),
        detail: "Create a new instance with an auto-generated name.",
        synopsis: "class new ?arg ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::at_least(1),
        detail: "Create a new instance with the given command name.",
        synopsis: "class create name ?arg ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::object",
        traits: Traits::IS_OO_METACLASS,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        subcommands: SUBCOMMANDS,
        // Every user method also dispatches through an object command, so
        // unknown words must stay valid.
        allow_unknown_subcommands: true,
        hover: Some(HoverSnippet {
            summary: "root class of the class hierarchy",
            synopsis: &["oo::object method ?arg ...?", "oo::object"],
            snippet: "The oo::object class is the root class of the object hierarchy; every object is an instance of this class.",
            source: "Tcl man page object.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
