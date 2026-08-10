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
use super::oo_class::{CLASS_FACTORY_SUBCOMMANDS, oo_class_arg_roles};
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
    synopsis: "oo::abstract method ?arg ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::abstract",
        // `oo::abstract create Name { … }` is a four-token `HEAD NAME
        // BRACED BRACED` call — the same shape `oo::class` also matches,
        // which is why it carries `NOT_PROC_FACTORY` too. The
        // top-level walker dispatches it correctly via `IS_OO_METACLASS` +
        // the TclOO `definition_body` grammar (`dispatch_definer` in
        // `tcl-compiler/src/signature_scan/walker.rs`), but the *nested*
        // proc-body rescan (`scan_factory_candidates`, run from
        // `handle_proc`) never consults that dispatch — it calls
        // `maybe_record_factory_candidate` directly for any head not in
        // `skip_heads`. Without `NOT_PROC_FACTORY` a `create` call written
        // inside a `proc` body would be misfiled as a tcllib-style
        // factory-wrapper candidate.
        traits: Traits::IS_OO_METACLASS
            | Traits::LANGUAGE_KEYWORD
            | Traits::DEFINES_PROCEDURE
            | Traits::NOT_PROC_FACTORY
            | Traits::ABSTRACT_CLASS_FACTORY,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(oo_class_arg_roles),
        return_type: Some(TclType::String),
        subcommands: CLASS_FACTORY_SUBCOMMANDS,
        allow_unknown_subcommands: true,
        // Bodies of `oo::abstract create / new / createWithNamespace`
        // run in a TclOO definition context, exactly like `oo::class`.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "metaclass for abstract classes",
            synopsis: &[
                "oo::abstract method ?arg ...?",
                "oo::abstract create name ?definition?",
            ],
            snippet: "Abstract classes hold definitions to be inherited but cannot be instantiated directly: oo::abstract unexports create, createWithNamespace, and new from every class it creates, so calling one of those methods on the abstract class itself (rather than on a concrete subclass) fails with an \"unknown method\" error instead of a dedicated abstract-class message. A subclass created with oo::class create (or another concrete metaclass) regains normal instantiation. Like oo::class, oo::abstract defines no constructor or destructor of its own.",
            source: "Tcl man page abstract.n",
            examples: "oo::abstract create fruit {\n    method eat {} {\n        puts \"yummy!\"\n    }\n}\noo::class create banana {\n    superclass fruit\n    method peel {} {\n        puts \"skin now off\"\n    }\n}\nset b [banana new]\n$b peel\n$b eat\nset f [fruit new]     ;# error: unknown method \"new\"",
            return_value: "The fully qualified name of the newly created class or object for create/new/createWithNamespace; otherwise whatever the invoked oo::class-inherited method returns.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        manufacturer_methods: crate::definer::TCLOO_DERIVED_METACLASS_MANUFACTURERS,
        ..CommandSpec::DEFAULT
    }
}
