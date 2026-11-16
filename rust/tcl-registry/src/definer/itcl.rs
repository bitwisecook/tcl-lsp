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

//! [incr Tcl]: the `itcl::class` member table, built-in methods and grammar.

use super::builtin_methods::{BuiltinMethodReceiver, BuiltinObjectMethod};
use super::member_effect::{DeclaredMemberVisibility, MemberVisibility, RelationSlot, StateScope};
use super::{
    BODY0_ROLES, CONSTRUCTOR, CTOR_ROLES, DESTRUCTOR, DefinerFamily, DefinitionBodyGrammar,
    INSTANCE_METHOD, METHOD_ROLES, MemberRefKind, MemberSpec, TYPE_METHOD, declaring, relation,
    state,
};
use crate::arg_role::ArgRole;

// [incr Tcl] — `itcl::class Name { … }` (and the bare `class` alias) bodies.
//
// The access modifiers `public` / `protected` / `private` are prefix wrappers
// (`public method foo {args} {body}`, `private variable x`) — `MemberKind::
// Wrapper`, handled generically like TclOO's `self`.  `inherit` lists base
// classes (multiple inheritance).  `variable` declares an instance variable
// (optionally with an init value + config body); `common` a class/static one.

/// itcl `variable NAME ?init? ?configbody?` — the declared name plus the
/// optional trailing config body (the script run when the public variable is
/// modified via `configure`); the init value between them is left to the
/// default classifier.
pub(super) const ITCL_VAR_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite), (2, ArgRole::Body)];

/// itcl `common NAME ?init?` — a class/static variable; the declared name only
/// (no config body).
pub(super) const ITCL_COMMON_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::VarWrite)];

pub(super) const ITCL_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES, INSTANCE_METHOD),
    // A class-scoped `proc NAME ARGS BODY` — same shape as a method.
    MemberSpec::flat("proc", METHOD_ROLES, TYPE_METHOD),
    MemberSpec::flat("constructor", CTOR_ROLES, CONSTRUCTOR),
    MemberSpec::flat("destructor", BODY0_ROLES, DESTRUCTOR),
    MemberSpec::flat("variable", ITCL_VAR_ROLES, state(StateScope::PerInstance)),
    MemberSpec::flat("common", ITCL_COMMON_ROLES, state(StateScope::PerType)),
    // Base-class list (multiple inheritance) — each argument names a base
    // class, a first-class command reference exactly like TclOO's `superclass`,
    // so navigation reaches the base class across files.
    MemberSpec::all_refs(
        "inherit",
        MemberRefKind::Class,
        relation(RelationSlot::Superclass),
    ),
    // Access modifiers: prefix wrappers around an inner member keyword. A
    // `protected` member is reachable from the class and its heirs but never
    // dispatched from outside — the unexported tier.
    MemberSpec::wrapper("public", declaring(DeclaredMemberVisibility::Public)),
    MemberSpec::wrapper("protected", declaring(DeclaredMemberVisibility::Unexported)),
    MemberSpec::wrapper("private", declaring(DeclaredMemberVisibility::Private)),
];

/// The methods every [incr Tcl] **object** answers to without its class body
/// declaring them.
///
/// From the itcl(n) / itclObject documentation: every object supports
/// `configure`, `cget` and `isa`, plus the `info` ensemble extended with the
/// class-introspection subcommands.  Like snit, itcl dispatches through its
/// own generated machinery with no `my`-only tier, so all are exported.
/// Object destruction is `itcl::delete object NAME`, a command rather than a
/// method, so no `destroy` is listed.
pub(super) const ITCL_BUILTIN_OBJECT_METHODS: &[BuiltinObjectMethod] = &[
    BuiltinObjectMethod {
        name: "configure",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "set one or more of the object's public variables",
    },
    BuiltinObjectMethod {
        name: "cget",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "retrieve the value of one of the object's public variables",
    },
    BuiltinObjectMethod {
        name: "isa",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "test whether the object belongs to the named class",
    },
    BuiltinObjectMethod {
        name: "info",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "introspect the object's class, variables and methods",
    },
];

/// The definition-body grammar for [incr Tcl] `itcl::class` / bare `class`.
/// Member bodies run in the object's context with the instance/common
/// variables and `this` in scope.
pub const ITCL_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::Itcl,
    members: ITCL_MEMBERS,
    implicit_vars: &["this"],
    implicit_receiver_vars: &["this"],
    member_body_namespace_path: &[],
    // [incr Tcl] class commands expose `::itcl::class` introspection rather
    // than built-in typemethods on the class command itself.
    builtin_type_methods: &[],
    builtin_object_methods: ITCL_BUILTIN_OBJECT_METHODS,
    builtin_terminating_methods: &[],
    member_body_commands: &[],
    // itcl constructs through `ClassName objName` *at the class command* —
    // but only via the documented `ClassName objName ?args?` form, which the
    // handle scan reaches through `creates_instance_at`, not through this
    // snit-specific bare-word shorthand.
    bare_word_construction: false,
    bare_word_construction_hint: None,
    bare_word_construction_source_hint: None,
    dynamic_method_dispatch: false,
    // Reached through `creates_instance_at` (see above), not a named
    // manufacturer method on the class command.
    manufacturers: &[],
    // [incr Tcl] has no user-writable unrecognised-method fallback member.
    unknown_dispatch_method: None,
    // itcl's `configure` / `cget` are family built-ins (every instance has
    // them), so they live in `builtin_object_methods`.
    property_accessor_methods: &[],
};
