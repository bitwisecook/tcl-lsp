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

//! `TclOO` class.
use super::oo_class::oo_class_arg_roles;
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "oo::configurable method ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::configurable",
        traits: Traits::IS_OO_METACLASS | Traits::LANGUAGE_KEYWORD | Traits::DEFINES_PROCEDURE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(oo_class_arg_roles),
        return_type: Some(TclType::String),
        // Bodies of `oo::configurable create / new / createWithNamespace`
        // run in a TclOO definition context (not the caller's frame),
        // exactly like `oo::class`.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "class that supports configurable properties",
            synopsis: &[
                "oo::configurable method ?arg ...?",
                "oo::configurable create name ?definition?",
            ],
            snippet: "The oo::configurable command creates a class that automatically supports the property definition command and a configure method for getting and setting property values on instances.",
            source: "Tcl man page configurable.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        ..CommandSpec::DEFAULT
    }
}
