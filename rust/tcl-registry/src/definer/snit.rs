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

//! snit: the `snit::type` / `snit::widget` member tables, built-in methods and grammars.

use super::builtin_methods::{BuiltinMethodReceiver, BuiltinObjectMethod};
use super::command_installation::{BareWordConstructionSourceHint, ManufacturerMethod};
use super::member_effect::{
    CallableRole, MemberEffect, MemberReceiver, MemberVisibility, StateScope,
};
use super::{
    BODY0_ROLES, CONSTRUCTOR, CTOR_ROLES, DESTRUCTOR, DefinerFamily, DefinitionBodyGrammar,
    INIT_AT_DEFINITION, INSTANCE_METHOD, METHOD_ROLES, MemberBodyCommand, MemberSpec, TYPE_METHOD,
    TYPE_PROCEDURE, VAR0_ROLES, callable, state,
};
use crate::arg_role::ArgRole;

/// snit's single class-command manufacturer — snit(n), "The Type Command":
/// `$type create name ?option value…?`.  There is no `new`: a snit instance
/// is always named, either explicitly or through the `%AUTO%` substitution
/// in the name itself.
pub(super) const SNIT_MANUFACTURERS: &[ManufacturerMethod] = &[ManufacturerMethod {
    keyword: "create",
    visibility: MemberVisibility::Exported,
    names_instance_at: Some(1),
    definition_body_at: None,
    constructor_args_from: 2,
}];

// snit — `snit::type` / `snit::widget` / `snit::widgetadaptor` bodies.

/// `onconfigure -option valueVar BODY` (snit 1.x) — the value var + body.
pub(super) const ONCONFIGURE_ROLES: &[(u8, ArgRole)] =
    &[(1, ArgRole::VarWrite), (2, ArgRole::Body)];

/// `oncget -option BODY` (snit 1.x) — the body.
pub(super) const ONCGET_ROLES: &[(u8, ArgRole)] = &[(1, ArgRole::Body)];

pub(super) const SNIT_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES, INSTANCE_METHOD),
    MemberSpec::flat("typemethod", METHOD_ROLES, TYPE_METHOD),
    // A type-private `proc NAME ARGS BODY` — same shape as a method, but a
    // procedure in the type's namespace rather than a dispatched member.
    MemberSpec::flat("proc", METHOD_ROLES, TYPE_PROCEDURE),
    MemberSpec::flat("constructor", CTOR_ROLES, CONSTRUCTOR),
    MemberSpec::flat("destructor", BODY0_ROLES, DESTRUCTOR),
    MemberSpec::flat("typeconstructor", BODY0_ROLES, INIT_AT_DEFINITION),
    // `onconfigure -option valueVar BODY` / `oncget -option BODY`: the
    // option's write and read handlers, named by the option word rather than a
    // `Name` slot.
    MemberSpec::flat(
        "onconfigure",
        ONCONFIGURE_ROLES,
        callable(
            MemberReceiver::Instance,
            CallableRole::Mutator,
            (None, None, Some(2)),
        ),
    ),
    MemberSpec::flat(
        "oncget",
        ONCGET_ROLES,
        callable(
            MemberReceiver::Instance,
            CallableRole::Accessor,
            (None, None, Some(1)),
        ),
    ),
    MemberSpec::flat("variable", VAR0_ROLES, state(StateScope::PerInstance)),
    MemberSpec::flat("typevariable", VAR0_ROLES, state(StateScope::PerType)),
    MemberSpec::flat("component", VAR0_ROLES, state(StateScope::PerInstance)),
    MemberSpec::flat("typecomponent", VAR0_ROLES, state(StateScope::PerType)),
    // Name-reference / option-declaration members — recognised keywords with
    // nothing to recurse or declare.
    MemberSpec::keyword_only("option", state(StateScope::Option)),
    MemberSpec::keyword_only("delegate", MemberEffect::Configuration),
    MemberSpec::keyword_only("expose", MemberEffect::Configuration),
];

/// The methods every snit **instance** answers to without its type body
/// declaring them.
///
/// From snit(n), "The Instance Command": every instance supports `configure`,
/// `configurelist`, `cget`, `destroy` and `info` in addition to its declared
/// methods.  snit generates an ordinary Tcl dispatcher proc rather than using
/// `TclOO`'s export machinery, so it has no unexported tier — every one is
/// reachable through the instance command.  `create` belongs to the *type*
/// command, so it is marked [`BuiltinMethodReceiver::ClassObject`]; snit has
/// no `new` at all.
pub(super) const SNIT_BUILTIN_OBJECT_METHODS: &[BuiltinObjectMethod] = &[
    BuiltinObjectMethod {
        name: "configure",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "set one or more of the instance's options",
    },
    BuiltinObjectMethod {
        name: "configurelist",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "set options from an option/value list",
    },
    BuiltinObjectMethod {
        name: "cget",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "retrieve the value of one of the instance's options",
    },
    BuiltinObjectMethod {
        name: "destroy",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "destroy the instance",
    },
    BuiltinObjectMethod {
        name: "info",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "introspect the instance's type, options and components",
    },
    BuiltinObjectMethod {
        name: "create",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::ClassObject,
        operation: None,
        detail: "construct an instance with the given name",
    },
];

/// The commands snit injects into every member body — see
/// [`DefinitionBodyGrammar::member_body_commands`].
///
/// VERIFIED against tcllib snit(n): `install` is available in any snit
/// instance method / constructor and, in its `using` form, "creates the
/// component using the specified command, and assigns the result to the
/// component variable".  The bare `install NAME $widget` form binds a
/// component whose class is only known at run time, so no
/// [`MemberBodyCommand::binds_handle`] fires for it — abstention, not a
/// guess.
pub(super) const SNIT_MEMBER_BODY_COMMANDS: &[MemberBodyCommand] = &[MemberBodyCommand {
    name: "install",
    detail: "install a snit component into its component variable",
    binds_handle: Some(crate::handle_binding::SNIT_INSTALL_BINDS_HANDLE),
}];

/// The member-body commands a snit **widget** definer injects: `install`, plus
/// the hull installer only a widget has.
///
/// VERIFIED against tcllib snit(n): `installhull` exists for `snit::widget` /
/// `snit::widgetadaptor`, whose instances have a hull; a plain `snit::type`
/// has none, so it is deliberately absent from [`SNIT_MEMBER_BODY_COMMANDS`].
pub(super) const SNIT_WIDGET_MEMBER_BODY_COMMANDS: &[MemberBodyCommand] = &[
    MemberBodyCommand {
        name: "install",
        detail: "install a snit component into its component variable",
        binds_handle: Some(crate::handle_binding::SNIT_INSTALL_BINDS_HANDLE),
    },
    MemberBodyCommand {
        name: "installhull",
        detail: "install the widget's hull component",
        binds_handle: Some(crate::handle_binding::SNIT_INSTALLHULL_BINDS_HANDLE),
    },
];

/// The two snit instance-name shapes that remain recognisable when a
/// low-level consumer has only a set of class names, not each class's family:
/// `%AUTO%` asks snit to generate a unique suffix, and a leading dot is the
/// conventional Tk widget path. Other plain names are valid too, but require
/// the exact class grammar to distinguish them from a method word.
pub(super) fn snit_bare_word_construction_hint(word: &str) -> bool {
    word == "%AUTO%" || word.starts_with('.')
}

/// The definition-body grammar for a plain snit `type`.  `implicit_vars` is
/// the set snit injects into *every* member body; the widget definers carry
/// [`SNIT_WIDGET_GRAMMAR`], whose `implicit_vars` add the `win` / `hull`
/// pair a widget injects on top (they are not implicit in a plain
/// `snit::type`), so which definers inject them is registry data, not a
/// consumer's name-suffix check.
pub const SNIT_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Snit,
    profile: super::SNIT_PROFILE,
    members: SNIT_MEMBERS,
    implicit_vars: &["self", "selfns", "type", "options"],
    implicit_receiver_vars: &["self"],
    member_body_namespace_path: &[],
    // snit(n): "Every snit type has the following type methods: create,
    // info, destroy."  `create` is left out — see the field's doc comment.
    builtin_type_methods: &["info", "destroy"],
    builtin_object_methods: SNIT_BUILTIN_OBJECT_METHODS,
    builtin_terminating_methods: &[],
    member_body_commands: SNIT_MEMBER_BODY_COMMANDS,
    // snit(n), "The Type Command": `$type name ?args?` with a non-typemethod
    // first word is `$type create name ?args?`.
    bare_word_construction: true,
    bare_word_construction_hint: Some(snit_bare_word_construction_hint),
    bare_word_construction_source_hint: Some(BareWordConstructionSourceHint {
        values: &["%AUTO%"],
        prefixes: &["."],
    }),
    dynamic_method_dispatch: true,
    manufacturers: SNIT_MANUFACTURERS,
    // snit dispatches an unrecognised *method* through `delegate method *`
    // when the type declares one, which is a declared delegation target
    // rather than a member body this analysis can read; there is no snit
    // counterpart of TclOO's `unknown`.
    unknown_dispatch_method: None,
    // snit's `configure` / `cget` are on every instance of the family, not
    // generated from property declarations, so they live in
    // `builtin_object_methods`.
    property_accessor_methods: &[],
};

/// The definition-body grammar for snit `widget` / `widgetadaptor`: the same
/// member set as [`SNIT_GRAMMAR`], plus the widget-only implicit instance
/// variables `win` (the widget's window path) and `hull` (the hull
/// component).
pub const SNIT_WIDGET_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Snit,
    profile: super::SNIT_PROFILE,
    members: SNIT_MEMBERS,
    implicit_vars: &["self", "selfns", "type", "options", "win", "hull"],
    implicit_receiver_vars: &["self"],
    // Same as SNIT_GRAMMAR: snit resolves member-body barewords through its
    // own generated type namespace, not an implicit helper path (the
    // `::oo::Helpers` fact is TclOO-only).
    member_body_namespace_path: &[],
    // snit(n): "Every snit type has the following type methods: create,
    // info, destroy."  `create` is left out — see the field's doc comment.
    builtin_type_methods: &["info", "destroy"],
    builtin_object_methods: SNIT_BUILTIN_OBJECT_METHODS,
    builtin_terminating_methods: &[],
    member_body_commands: SNIT_WIDGET_MEMBER_BODY_COMMANDS,
    // snit(n), "The Type Command": `$type name ?args?` with a non-typemethod
    // first word is `$type create name ?args?`.
    bare_word_construction: true,
    bare_word_construction_hint: Some(snit_bare_word_construction_hint),
    bare_word_construction_source_hint: Some(BareWordConstructionSourceHint {
        values: &["%AUTO%"],
        prefixes: &["."],
    }),
    dynamic_method_dispatch: true,
    manufacturers: SNIT_MANUFACTURERS,
    // snit dispatches an unrecognised *method* through `delegate method *`
    // when the type declares one, which is a declared delegation target
    // rather than a member body this analysis can read; there is no snit
    // counterpart of TclOO's `unknown`.
    unknown_dispatch_method: None,
    // snit's `configure` / `cget` are on every instance of the family, not
    // generated from property declarations, so they live in
    // `builtin_object_methods`.
    property_accessor_methods: &[],
};
