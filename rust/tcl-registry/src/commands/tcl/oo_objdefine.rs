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

//! `oo::objdefine` — define per-object members.
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
    synopsis: "oo::objdefine object defScript",
    dialects: None,
}];

use super::oo_define::oo_define_arg_roles;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::objdefine",
        traits: Traits::NOT_PROC_FACTORY | Traits::LANGUAGE_KEYWORD | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        // `oo::objdefine` has the same body-shape rules as
        // `oo::define`; share the resolver.
        arg_role_resolver: Some(oo_define_arg_roles),
        return_type: Some(TclType::String),
        // Same structural-body rule as `oo::define` — bodies
        // run in a per-object definition context, not the caller's
        // frame.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "define and configure classes and objects",
            synopsis: &[
                "oo::objdefine object defScript",
                "oo::objdefine object subcommand arg ?arg ...?",
                "oo::objdefine objectName ?definition?",
            ],
            snippet: "The oo::define command is used to control the configuration of classes, and the oo::objdefine command is used to control the configuration of objects (including classes as instance objects), with the configuration being applied to the entity named in the class or the object argument.",
            source: "Tcl man page define.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        analyser_hook: Some(crate::hooks::AnalyserHookId::OoObjdefine),
        ..CommandSpec::DEFAULT
    }
}
