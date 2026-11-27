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

//! `TclOO`: the `oo::class` / `oo::define` member table, built-in object methods, manufacturers and grammars.

use super::builtin_methods::{
    BuiltinMethodReceiver, BuiltinObjectMethod, BuiltinObjectMethodOperation,
};
use super::command_installation::ManufacturerMethod;
use super::member_effect::{
    DeclaredMemberVisibility, MemberEffect, MemberReceiver, MemberVisibility, RelationSlot,
    StateScope,
};
use super::slots::SlotOp;
use super::{
    BODY0_ROLES, CONSTRUCTOR, CTOR_ROLES, DESTRUCTOR, DefinerFamily, DefinitionBodyGrammar,
    FORWARD_ROLES, INIT_AT_DEFINITION, INSTANCE_METHOD, METHOD_ROLES, MemberOptionValue,
    MemberRefKind, MemberRetraction, MemberSpec, OptionalMemberArgument, TYPE_METHOD, WrapperShift,
    declaring, relation, state,
};
use crate::arg_role::ArgRole;
use tcl_dialect::model::SpecSurface;

/// Tcl 9's `method NAME ?EXPORT-FLAG? PARAMS BODY` declaration flags.
///
/// These are intentionally a closed registry vocabulary rather than arbitrary
/// `-word` parsing: Tcl rejects every other spelling at this position.
pub(super) const TCLOO_METHOD_VISIBILITY_OPTIONS: &[MemberOptionValue] = &[
    MemberOptionValue {
        value: "-export",
        role: ArgRole::Option,
        surface: Some(TCL90_MEMBERS),
        declared_visibility: Some(DeclaredMemberVisibility::Public),
    },
    MemberOptionValue {
        value: "-private",
        role: ArgRole::Option,
        surface: Some(TCL90_MEMBERS),
        declared_visibility: Some(DeclaredMemberVisibility::Private),
    },
    MemberOptionValue {
        value: "-unexport",
        role: ArgRole::Option,
        surface: Some(TCL90_MEMBERS),
        declared_visibility: Some(DeclaredMemberVisibility::Unexported),
    },
];

pub(super) const TCLOO_METHOD_VISIBILITY_ARGUMENT: OptionalMemberArgument =
    OptionalMemberArgument {
        // The method name is fixed; the optional flag appears before params/body.
        position: 1,
        values: TCLOO_METHOD_VISIBILITY_OPTIONS,
    };

/// Tcl 9's `definitionnamespace ?-class|-instance? NAMESPACE` selector.
pub(super) const TCLOO_DEFINITION_NAMESPACE_FACETS: &[MemberOptionValue] = &[
    MemberOptionValue {
        value: "-class",
        role: ArgRole::Option,
        surface: Some(TCL90_MEMBERS),
        declared_visibility: None,
    },
    MemberOptionValue {
        value: "-instance",
        role: ArgRole::Option,
        surface: Some(TCL90_MEMBERS),
        declared_visibility: None,
    },
];

pub(super) const TCLOO_DEFINITION_NAMESPACE_ARGUMENT: OptionalMemberArgument =
    OptionalMemberArgument {
        // The facet is optional; the namespace is always the final fixed tail.
        position: 0,
        values: TCLOO_DEFINITION_NAMESPACE_FACETS,
    };

pub(super) const DEFINITION_NAMESPACE_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::NamespaceName)];

/// The `TclOO` definition members Tcl 9.0 added (TIP 478 `classmethod` /
/// `initialise` / `initialize` / `private`, TIP 524 `definitionnamespace`).
/// None of them exists in the 8.6 grammar — confirmed live: each of the
/// three call shapes (a member inside `oo::class create`'s body, inside an
/// `oo::define` block, and the single-command `oo::define Cls classmethod …`
/// form) fails on tclsh8.6 with `invalid command name "<member>"` and
/// succeeds on tclsh9.0.
pub(super) const TCL90_MEMBERS: &[SpecSurface] = SpecSurface::TCL90_PLUS;

pub(super) const TCLOO_MEMBERS: &[MemberSpec] = &[
    MemberSpec::flat("method", METHOD_ROLES, INSTANCE_METHOD)
        .optional_argument(TCLOO_METHOD_VISIBILITY_ARGUMENT),
    MemberSpec::flat("classmethod", METHOD_ROLES, TYPE_METHOD).with_surface(TCL90_MEMBERS),
    MemberSpec::flat("constructor", CTOR_ROLES, CONSTRUCTOR),
    MemberSpec::flat("destructor", BODY0_ROLES, DESTRUCTOR),
    MemberSpec::flat("initialise", BODY0_ROLES, INIT_AT_DEFINITION).with_surface(TCL90_MEMBERS),
    MemberSpec::flat("initialize", BODY0_ROLES, INIT_AT_DEFINITION).with_surface(TCL90_MEMBERS),
    // `private` is a prefix wrapper (`private method m {} {…}`, `private
    // variable x`) *and* a bare definition-script block (`private { … }`).
    // The wrapped member keeps its side and is declared private.
    MemberSpec::wrapper_or_body("private", declaring(DeclaredMemberVisibility::Private))
        .with_surface(TCL90_MEMBERS),
    // `variable a b c` inside a class body declares every name.  A slot:
    // `-append` default like `filter`, but deduplicating (tclsh 9.0.4:
    // `variable a ; variable a b` → `a b`).
    MemberSpec::all_vars("variable", state(StateScope::PerInstance))
        .slot_spec(SlotOp::Append, true),
    // Reference-only members: they declare nothing and recurse nothing, but
    // their arguments *name* an entity defined elsewhere — a class or a method
    // — so they are references, not free strings.  All three are slots;
    // the defaults are pinned against C Tcl (`slots[]` in
    // 9.0.4's tclOODefineCmds.c, the `--default-operation` forwards in
    // 8.6.16's tclOO.c — identical): `superclass` / `mixin` replace,
    // `filter` appends.
    MemberSpec::all_refs(
        "superclass",
        MemberRefKind::Class,
        relation(RelationSlot::Superclass),
    )
    .slot_spec(SlotOp::Set, false),
    MemberSpec::all_refs("mixin", MemberRefKind::Class, relation(RelationSlot::Mixin))
        .slot_spec(SlotOp::Set, false),
    MemberSpec::all_refs(
        "filter",
        MemberRefKind::Method,
        relation(RelationSlot::Filter),
    )
    .slot_spec(SlotOp::Append, false),
    MemberSpec::all_refs("export", MemberRefKind::Method, MemberEffect::Visibility)
        .visibility(MemberVisibility::Exported),
    MemberSpec::all_refs("unexport", MemberRefKind::Method, MemberEffect::Visibility)
        .visibility(MemberVisibility::Unexported),
    MemberSpec::all_refs(
        "deletemethod",
        MemberRefKind::Method,
        MemberEffect::Retraction,
    )
    .retracting(MemberRetraction::EveryArgument),
    // `forward NAME cmd ?arg…?` declares NAME as a method; the word after it
    // (`cmd`) is the delegated command's name — a first-class command
    // reference the walker records so navigation reaches it, exactly like the
    // command a `superclass`/`mixin` names.  Any baked arguments after it are
    // ordinary values.
    MemberSpec::flat(
        "forward",
        FORWARD_ROLES,
        MemberEffect::Forward {
            name_slot: 0,
            prefix_slot: 1,
        },
    ),
    // `renamemethod FROM TO` — both name methods.
    // Both words name methods; the FROM word is retracted (and the TO word is
    // a member this walker does not record), so the whole call retracts.
    MemberSpec::all_refs(
        "renamemethod",
        MemberRefKind::Method,
        MemberEffect::Retraction,
    )
    .retracting(MemberRetraction::FirstArgument),
    MemberSpec::flat(
        "definitionnamespace",
        DEFINITION_NAMESPACE_ROLES,
        MemberEffect::Configuration,
    )
    .optional_argument(TCLOO_DEFINITION_NAMESPACE_ARGUMENT)
    .with_surface(TCL90_MEMBERS),
    // Structurally irregular — a nested-member wrapper (`self method …`) and a
    // flag-keyed body form (`property … -get/-set …`); their body indices come
    // from the walker's `MemberKind`-driven handling, not a hardcoded name.
    // `self` moves the wrapped member to the class object.
    MemberSpec::wrapper_or_body(
        "self",
        WrapperShift {
            receiver: Some(MemberReceiver::TypeObject),
            visibility: None,
        },
    ),
    // `property` (and its configurable-class accessor machinery) is a 9.0
    // addition; the 8.6 `TclOO` definition grammar has no such member.
    MemberSpec::flag_keyed("property", MemberEffect::Configuration).with_surface(TCL90_MEMBERS),
];

/// The methods every `TclOO` object inherits from `oo::object` (plus the
/// three a *class* object additionally gets from `oo::class`), which no
/// class body declares.
///
/// Pinned against tclsh 9.0.4 **and** 8.6.16 — the sets are byte-identical
/// on both:
///
/// ```text
/// info class methods ::oo::object -all -private  ->  <cloned> destroy eval unknown variable varname
/// info class methods ::oo::class  -all -private  ->  <cloned> create createWithNamespace destroy
///                                                    eval new unknown variable varname
/// ```
///
/// Only `destroy` is exported: `TclOO`'s export test is
/// `Tcl_StringMatch(name, "[a-z]*")` applied at *declaration*, and the rest
/// are explicitly unexported by the core.  `<cloned>` fails the pattern on
/// its leading `<` as well.  That is why every one of them except `destroy`
/// is reachable through `my` and through nothing else — the exact reason
/// `my variable v` must never draw an unknown-method diagnostic while
/// `$obj variable v` legitimately does.
///
/// `configure` / `cget` are **not** listed: they come from `oo::configurable`
/// (Tcl 9.0+) and are declared as real class members on the classes that
/// use it, so they arrive through the ordinary member tables rather than
/// here.
pub(super) const TCLOO_BUILTIN_OBJECT_METHODS: &[BuiltinObjectMethod] = &[
    BuiltinObjectMethod {
        name: "destroy",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "destroy the object, running its destructors",
    },
    BuiltinObjectMethod {
        name: "eval",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "evaluate a script in the object's own namespace",
    },
    BuiltinObjectMethod {
        name: "unknown",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "handler invoked for an unresolved method name",
    },
    BuiltinObjectMethod {
        name: "variable",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: Some(BuiltinObjectMethodOperation::ObjectVariableLinks),
        detail: "link object-instance variables into the calling scope",
    },
    BuiltinObjectMethod {
        name: "varname",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "fully-qualified name of an object-instance variable",
    },
    BuiltinObjectMethod {
        name: "<cloned>",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::AnyObject,
        operation: None,
        detail: "copy hook invoked by oo::copy on the new object",
    },
    BuiltinObjectMethod {
        name: "new",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::ClassObject,
        operation: None,
        detail: "construct an instance with a generated name",
    },
    BuiltinObjectMethod {
        name: "create",
        visibility: MemberVisibility::Exported,
        receiver: BuiltinMethodReceiver::ClassObject,
        operation: None,
        detail: "construct an instance with the given name",
    },
    BuiltinObjectMethod {
        name: "createWithNamespace",
        visibility: MemberVisibility::Unexported,
        receiver: BuiltinMethodReceiver::ClassObject,
        operation: None,
        detail: "construct an instance in a named namespace",
    },
];

/// The definition-body grammar for every `TclOO` metaclass and the bare
/// `oo::define` / `oo::objdefine` script form.
pub static TCLOO_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    family: DefinerFamily::TclOo,
    profile: super::TCLOO_PROFILE,
    members: TCLOO_MEMBERS,
    implicit_vars: &[],
    implicit_receiver_vars: &[],
    member_body_namespace_path: TCLOO_MEMBER_BODY_NAMESPACE_PATH,
    // The class command's own built-in, non-manufacturing surface. tclsh
    // 9.0.4 and 8.6.16 agree byte for byte:
    //   oo::class create C {}
    //   info object methods ::C -all   ->  create destroy new
    // `create` / `new` are manufacturers and live in `manufacturers`; what
    // is left is `destroy`. The definition surface proper
    // (`oo::define`/`oo::objdefine`) is a separate command, not a method
    // here.
    builtin_type_methods: &["destroy"],
    builtin_object_methods: TCLOO_BUILTIN_OBJECT_METHODS,
    builtin_terminating_methods: &["unknown"],
    // TclOO injects no extra *commands* into a method body — its helpers
    // (`next` / `self` / `link` / `classvariable`) are real global commands in
    // `::oo::Helpers` with their own specs, reached through
    // `member_body_namespace_path` above; `my` lives in the receiver's own
    // namespace (`ObjectSystemProfile::receiver_namespace_commands`).
    member_body_commands: &[],
    // `Foo $win.a` is not a construction in TclOO: a class command only
    // constructs through `create` / `new` (tclsh 9.0.4: `::C x` →
    // `unknown method "x"`).
    bare_word_construction: false,
    bare_word_construction_hint: None,
    bare_word_construction_source_hint: None,
    dynamic_method_dispatch: false,
    manufacturers: TCLOO_MANUFACTURERS,
    // `object.n`: an unrecognised method name is dispatched to `unknown`.
    // This is what makes `::C x` above *reachable* rather than fatal once a
    // class (or its metaclass) declares one.
    unknown_dispatch_method: Some("unknown"),
    // Plain `oo::class` / `oo::abstract` / `oo::singleton` instances generate
    // nothing from properties — only `oo::configurable` does, and it carries
    // its own grammar below.
    property_accessor_methods: &[],
};

/// Definition grammar and manufactured-class surface of `oo::abstract`.
/// The metaclass's own factory methods are separate: each abstract class it
/// manufactures begins with private `create`, `new` and `createWithNamespace`.
/// Original class-object export operations can refine these defaults.
pub static TCLOO_ABSTRACT_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    manufacturers: &[
        ManufacturerMethod {
            visibility: MemberVisibility::Unexported,
            ..TCLOO_CREATE_MANUFACTURER
        },
        ManufacturerMethod {
            visibility: MemberVisibility::Unexported,
            ..TCLOO_NEW_MANUFACTURER
        },
        TCLOO_CREATE_WITH_NAMESPACE_MANUFACTURER,
    ],
    ..TCLOO_GRAMMAR
};

/// The definition-body grammar for `oo::configurable` — [`TCLOO_GRAMMAR`]
/// plus the one method that metaclass generates for a class's declared
/// `property` members.
///
/// A separate constant precisely because the definition *body* grammar is
/// identical (`property` is a member of `TCLOO_MEMBERS` already): what
/// differs is the instance surface the manufactured class ends up with.
/// Attaching this to the shared grammar would have told every consumer that
/// an `oo::class` instance answers `configure`, which it does not.
pub static TCLOO_CONFIGURABLE_GRAMMAR: DefinitionBodyGrammar = DefinitionBodyGrammar {
    // `configurable.n` (9.0.4 and 9.1b0, byte-identical but for the version
    // banner): "making a `configure` method available within the instances".
    // No `cget` — see `property_accessor_methods`' own note.
    property_accessor_methods: &["configure"],
    ..TCLOO_GRAMMAR
};

/// `TclOO`'s class-command manufacturers — see
/// [`DefinitionBodyGrammar::manufacturers`].
///
/// `class.n` (9.0.4): `create name ?definition?`, `new ?definition?`,
/// `createWithNamespace name nsName ?definition?`.  Only `create` and
/// `createWithNamespace` name the instance; `new` generates the name.
pub const TCLOO_CREATE_MANUFACTURER: ManufacturerMethod = ManufacturerMethod {
    keyword: "create",
    visibility: MemberVisibility::Exported,
    names_instance_at: Some(1),
    definition_body_at: Some(2),
    constructor_args_from: 2,
};

/// `new ?definition?`: manufacture with an automatically generated name.
pub const TCLOO_NEW_MANUFACTURER: ManufacturerMethod = ManufacturerMethod {
    keyword: "new",
    visibility: MemberVisibility::Exported,
    names_instance_at: None,
    definition_body_at: Some(1),
    constructor_args_from: 1,
};

/// `new` as inherited by the root `::oo::class` object. The method exists for
/// self-dispatch and can be exported reflectively, but ordinary external
/// dispatch cannot reach it in C Tcl 8.6 or 9.0.
pub const TCLOO_ROOT_NEW_MANUFACTURER: ManufacturerMethod = ManufacturerMethod {
    visibility: MemberVisibility::Unexported,
    ..TCLOO_NEW_MANUFACTURER
};

/// `createWithNamespace name nsName ?definition?`: the unexported
/// namespace-selecting manufacturer.
pub const TCLOO_CREATE_WITH_NAMESPACE_MANUFACTURER: ManufacturerMethod = ManufacturerMethod {
    keyword: "createWithNamespace",
    visibility: MemberVisibility::Unexported,
    names_instance_at: Some(1),
    definition_body_at: Some(3),
    constructor_args_from: 3,
};

pub(super) const TCLOO_MANUFACTURERS: &[ManufacturerMethod] = &[
    TCLOO_CREATE_MANUFACTURER,
    TCLOO_NEW_MANUFACTURER,
    TCLOO_CREATE_WITH_NAMESPACE_MANUFACTURER,
];

/// Manufacturer surface present on `oo::class` itself. C Tcl 9.0.4 and 8.6
/// expose only `create`; `new` and `createWithNamespace` remain present but
/// private (`info object methods ::oo::class -all -private`).
pub const TCLOO_ROOT_CLASS_MANUFACTURERS: &[ManufacturerMethod] = &[
    TCLOO_CREATE_MANUFACTURER,
    TCLOO_ROOT_NEW_MANUFACTURER,
    TCLOO_CREATE_WITH_NAMESPACE_MANUFACTURER,
];

/// Manufacturer surface exported by the other Tcl 9 metaclass commands.
/// `createWithNamespace` remains unexported on all of them.
pub const TCLOO_DERIVED_METACLASS_MANUFACTURERS: &[ManufacturerMethod] = TCLOO_MANUFACTURERS;

/// The `namespace path` a `TclOO` member body runs with — see
/// [`DefinitionBodyGrammar::member_body_namespace_path`].
///
/// Exposed on its own so a consumer holding only the analyser's
/// "this frame resolves like a `TclOO` method body" flag (rather than the
/// whole grammar) still reads the path from registry data.
pub const TCLOO_MEMBER_BODY_NAMESPACE_PATH: &[&str] = &["::oo::Helpers"];

/// The commands Tcl creates in a fresh `TclOO` object's private namespace:
/// `info commands [info object namespace $o]::*` answers `my` on tclsh 8.6
/// and `my myclass` on 9.0 and 9.1. Both are listed, so a consumer that does
/// not separate the releases still treats `myclass` as possibly local.
pub const TCLOO_RECEIVER_NAMESPACE_COMMANDS: &[&str] = &["my", "myclass"];
