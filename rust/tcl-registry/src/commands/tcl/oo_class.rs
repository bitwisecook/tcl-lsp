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
    synopsis: "oo::class method ?arg ...?",
    dialects: None,
}];

/// Resolve the body argument index for the metaclass shapes:
///
/// * `oo::class create Name body` → body at index 2.
/// * `oo::class new body` → body at index 1.
/// * `oo::class createWithNamespace Name ::ns body` → body at index 3.
///
/// Shared by every `IS_OO_METACLASS` command (`oo::class`,
/// `oo::configurable`, `oo::abstract`, `oo::singleton`) — they all
/// manufacture classes with the same `create` / `new` /
/// `createWithNamespace` shapes, so their definition-body argument
/// lives at the same index.
pub(crate) fn oo_class_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let n = args.len();
    if n < 2 {
        return Vec::new();
    }
    match args[0] {
        "create" if n >= 3 => vec![(2, ArgRole::Body)],
        "new" if n >= 2 => vec![(1, ArgRole::Body)],
        "createWithNamespace" if n >= 4 => vec![(3, ArgRole::Body)],
        _ => Vec::new(),
    }
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::class",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::IS_OO_METACLASS
            | Traits::LANGUAGE_KEYWORD
            | Traits::DEFINES_PROCEDURE,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(oo_class_arg_roles),
        return_type: Some(TclType::String),
        // Bodies of `oo::class create / new / createWithNamespace`
        // run in a TclOO definition context (not the caller's frame).
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "class of all classes",
            synopsis: &[
                "oo::class method ?arg ...?",
                "oo::class create name ?definition?",
            ],
            snippet: "Classes are objects that can manufacture other objects according to a pattern stored in the factory object (the class).",
            source: "Tcl man page class.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        ..CommandSpec::DEFAULT
    }
}
