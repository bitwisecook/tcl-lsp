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

//! `oo::configurable` — metaclass for configurable classes and objects.
use super::oo_class::{CLASS_FACTORY_SUBCOMMANDS, oo_class_arg_roles};
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "oo::configurable method ?arg ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::configurable",
        // `oo::configurable create Name { … }` is a four-token `HEAD NAME
        // BRACED BRACED` call — the same shape `oo::class` and
        // `oo::abstract` also match, which is why it carries
        // `NOT_PROC_FACTORY` too (see the rationale on `oo_abstract.rs`).
        traits: Traits::IS_OO_METACLASS
            | Traits::CONFIGURES_BY_PROPERTY
            | Traits::LANGUAGE_KEYWORD
            | Traits::DEFINES_PROCEDURE
            | Traits::NOT_PROC_FACTORY,
        // Verified absent (soft-404 "URL Not Found" page, HTTP 200) from
        // the tcl-lang.org manpage tree for 8.4, 8.5, and 8.6; present with
        // byte-identical NAME/SYNOPSIS/DESCRIPTION/CONFIGURE
        // METHOD/PROPERTY DEFINITIONS/EXAMPLES text in both the 9.0.4 and
        // 9.1b0 configurable.n pages (only the version banner differs) —
        // a Tcl 9.0 addition, unchanged in 9.1.
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(oo_class_arg_roles),
        return_type: Some(TclType::String),
        subcommands: CLASS_FACTORY_SUBCOMMANDS,
        allow_unknown_subcommands: true,
        // Bodies of `oo::configurable create / new / createWithNamespace`
        // run in a TclOO definition context (not the caller's frame),
        // exactly like `oo::class`.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "metaclass for configurable classes and objects",
            synopsis: &[
                "oo::configurable method ?arg ...?",
                "oo::configurable create name ?definition?",
            ],
            snippet: "Installs a property definition command (used inside oo::define, oo::objdefine, or a class body) and a configure method on instances, modelled after fconfigure/chan configure. Called with no arguments, configure returns an alphabetically sorted dict of every readable/read-write property and its value; with one argument it reads that property; with an even count of arguments it sets each pair in the order given (property names may be given as an unambiguous prefix in either case). If a set fails partway through, the pairs already applied stay applied and the rest are skipped. property name ?-get script? ?-kind readable|writable|readwrite? ?-set script? declares a property: -kind defaults to readwrite, and the default getter/setter just read/write an instance variable of the same name. Write-only properties never appear in a bare configure listing. A subclass must itself be created via oo::configurable (or a further subclass) to declare additional properties of its own.",
            source: "Tcl man page configurable.n",
            examples: "oo::configurable create Point {\n    property x y\n    constructor args {\n        my configure -x 0 -y 0 {*}$args\n    }\n    variable x y\n}\nset pt [Point new -x 27]\n$pt configure -y 42\nputs [$pt configure -x]      ;# 27\nputs [$pt configure]         ;# -x 27 -y 42",
            return_value: "The fully qualified name of the newly created class for create/new/createWithNamespace; otherwise whatever the invoked oo::class-inherited method returns.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        // The configurable-specific grammar: the same TclOO definition body,
        // plus the `configure` method this metaclass generates for the
        // class's `property` members (`property_accessor_methods`).
        definition_body: Some(&crate::definer::TCLOO_CONFIGURABLE_GRAMMAR),
        manufacturer_methods: crate::definer::TCLOO_DERIVED_METACLASS_MANUFACTURERS,
        ..CommandSpec::DEFAULT
    }
}
