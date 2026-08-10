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

//! `oo::class` — the class of all classes; the `TclOO` metaclass.
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

/// The three class-manufacturing methods `class.n` documents for
/// `oo::class` — non-exhaustive completion/hover data only, never
/// `closed_value_args`: `oo::class` is itself an ordinary `TclOO`
/// object, so it also answers to every method `oo::object` defines
/// (`destroy`, …) and to any method grafted on afterwards (e.g. via
/// `oo::objdefine`), neither of which belongs in a fixed enum here.
const CLASS_METHOD_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "create",
        detail: "Create a new instance named name, passing the optional definition script to oo::define. Returns the fully qualified object name.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "new",
        detail: "Create a new instance with an automatically generated unique name. Not exported on oo::class itself — new classes should be made with create.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "createWithNamespace",
        detail: "Non-exported: like create, but the instance's internal namespace is explicitly named nsName instead of being auto-generated.",
        ..ArgValue::DEFAULT
    },
];

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
    let Some(body) = args
        .first()
        .and_then(|word| crate::definer::TCLOO_GRAMMAR.manufacturer(word))
        .and_then(|method| method.definition_body_at)
    else {
        return Vec::new();
    };
    (usize::from(body) < args.len())
        .then_some(vec![(body, ArgRole::Body)])
        .unwrap_or_default()
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
                "oo::class new ?definition?",
                "oo::class createWithNamespace name nsName ?definition?",
            ],
            snippet: "Classes are objects that manufacture other objects according to a pattern stored in the factory object (the class): create makes an instance with an explicit name, new makes one with an automatically generated unique name. oo::class is the class of all classes — every class, including oo::class itself, is an instance of oo::class, and oo::class is a subclass of oo::object, so every class is also an object. oo::class hides its own new method on itself, so a brand-new class should always be made with create; a class made that way still exports a normal, working new of its own for creating instances. The constructor takes a single optional argument, passed on to oo::define together with the new class's name, so a class body can be supplied directly at creation time. There is no explicit destructor: destroying a class also destroys all its subclasses, instances, and any objects it has been mixed into. The non-exported createWithNamespace method behaves like create but additionally lets the caller name the instance's internal namespace explicitly.",
            source: "Tcl man page class.n",
            examples: "oo::class create fruit {\n    method eat {} {\n        puts \"yummy!\"\n    }\n}\noo::class create banana {\n    superclass fruit\n    constructor {} {\n        my variable peeled\n        set peeled 0\n    }\n    method peel {} {\n        my variable peeled\n        set peeled 1\n        puts \"skin now off\"\n    }\n    method edible? {} {\n        my variable peeled\n        return $peeled\n    }\n    method eat {} {\n        if {![my edible?]} {\n            my peel\n        }\n        next\n    }\n}\nset b [banana new]\n$b eat    ;# prints \"skin now off\" and \"yummy!\"\nfruit destroy\n$b eat    ;# error \"unknown command\"",
            return_value: "The fully qualified name of the newly created class for create, new, and createWithNamespace; otherwise whatever the invoked method returns.",
        }),
        forms: FORMS,
        arg_values: &[(0, CLASS_METHOD_VALUES)],
        side_effects: SIDE_EFFECTS,
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        manufacturer_methods: crate::definer::TCLOO_ROOT_CLASS_MANUFACTURERS,
        ..CommandSpec::DEFAULT
    }
}
