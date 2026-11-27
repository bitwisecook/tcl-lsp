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

//! How the native `TclOO` definition workers treat a selected member: lookup, receiver layout, deferred setters, inheritance, constructors and lifecycle bodies.

use super::member_effect::DeclaredMemberVisibility;
use super::{DefinitionBodyGrammar, DefinitionSpecialMemberKind, MemberKind, MemberSpec};
use crate::arg_role::ArgRole;
use crate::special_vars::BootstrapDialect;
use tcl_dialect::TclVersion;
use tcl_dialect::model::{SurfaceQuery, surface_admits};

/// Effect of a definition member on a class's default construction path.
/// Unknown effects cannot justify a bounded constructor execution summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberConstructionEffect {
    /// Declares an instance method whose body is deferred until invocation.
    DeferredInstanceMethod,
    /// The member may alter or execute a construction hook or class dispatch.
    Unknown,
}

/// Native class-method lookup contribution of an original definition member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeInheritedClassKind {
    /// The superclass follows methods declared by the receiver class.
    Superclass,
    /// The mixin precedes methods declared by the receiver class.
    Mixin,
}

/// Native definition receiver whose method table a declaration modifies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionReceiver {
    /// Methods on manufactured instances.
    Instance,
    /// Methods on the class object's own object dispatch table.
    Class,
}

/// Audited deferred method layout in original definition words.
#[derive(Debug, Clone, Copy)]
pub struct ReceiverMethodLayout {
    /// Instance or class-object table selected by the native definition handler.
    pub receiver: DefinitionReceiver,
    /// Definition worker table, independent of the resulting receiver table.
    pub worker_receiver: DefinitionReceiver,
    /// Native classmethod body on the class factory's generated delegate.
    /// This describes ownership only; the caller must retain the actual
    /// factory allocation and current delegate dispatch dependencies.
    pub class_delegate: bool,
    /// Original word index containing the selected method declaration keyword.
    pub member_word: usize,
    /// Exact admitted member descriptor, without executing its body.
    pub member: &'static MemberSpec,
}

/// Effect of an admitted instance declaration on the class object's native
/// manufacture dispatch. This is independent of constructor callback effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberManufactureDispatchEffect {
    /// The native declaration changes instances; original class manufacture
    /// remains selected. Constructor bodies may still execute arbitrary code.
    PreservesNativeFactory,
    /// Class dispatch, metaclass configuration or immediate callbacks may change.
    Unknown,
}

/// Effect of the original native constructor/destructor body operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeLifecycleBodyDisposition {
    /// An empty counted body removes the native method record. No formal
    /// parser or method activation is selected by that declaration.
    Removed,
    /// A nonempty body installs the original native method source.
    Retained,
}

/// Applicability of a selected definition member's source formal grammar.
/// This grants no handler, installed callable, entered frame or completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormalValidationApplicability {
    /// The authored declaration requires formal-list validation.
    Required,
    /// A removed lifecycle declaration does not select its formal parser.
    SkippedRemovedLifecycle,
}

/// Selected intrinsic deferred-method setter. The caller must independently
/// retain a live definition target and unchanged worker, validate the complete
/// formal list under `recipe`, and retain the evaluated body value. The setter
/// stores that value without entering it. Replacing a known source procedure
/// releases only that procedure record; unknown method implementations cannot
/// borrow this closed transfer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeDeferredMethodSetter {
    pub(super) recipe: tcl_syntax::naming::NativeNameProtocol,
    pub(super) name: usize,
    pub(super) parameters: usize,
    pub(super) body: usize,
}

impl NativeDeferredMethodSetter {
    /// Independently selected native name/formal protocol.
    #[must_use]
    pub const fn recipe(self) -> tcl_syntax::naming::NativeNameProtocol {
        self.recipe
    }
    /// Complete post-worker name operand ordinal.
    #[must_use]
    pub const fn name_argument(self) -> usize {
        self.name
    }
    /// Complete post-worker formal-list operand ordinal.
    #[must_use]
    pub const fn parameters_argument(self) -> usize {
        self.parameters
    }
    /// Complete post-worker body value ordinal; its contents do not execute.
    #[must_use]
    pub const fn body_argument(self) -> usize {
        self.body
    }
    /// Exact argc required by this unwrapped setter form.
    #[must_use]
    pub const fn argument_count(self) -> usize {
        3
    }
}

/// Native allocation boundary around an original constructor activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeConstructorBoundary {
    /// `TclOO` restores the saved state on success; every failed constructor
    /// deletes the allocated object and invokes its destruction callbacks.
    TclOo,
}

impl NativeConstructorBoundary {
    /// Completion after the method's own return boundary. Allocation converts
    /// non-OK method codes into Error; process exit remains outside cleanup.
    #[must_use]
    pub fn completion(
        self,
        route: crate::completion_route::InvocationCompletionRoute,
    ) -> crate::completion_route::InvocationCompletionRoute {
        use crate::completion_route::InvocationCompletionRoute as Route;
        match self {
            Self::TclOo => match route {
                Route::Tcl(crate::CompletionCode::Ok) | Route::ProcessExit => route,
                Route::Unknown => Route::Unknown,
                Route::TclAlternatives(_) if route.normal_possible() => Route::Unknown,
                _ => Route::Tcl(crate::CompletionCode::Error),
            },
        }
    }
}

/// Native command-table dependency of a definition member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionMemberLookup {
    /// Namespace in which the definition script resolves its member head.
    pub namespace: &'static str,
    /// Canonical interpreter command implementing this member.
    pub implementation: String,
}

impl DefinitionBodyGrammar {
    /// Native `TclOO` definition-member lookup under the active release.
    /// A descriptor alone does not prove this command table slot is unmodified.
    #[must_use]
    pub fn definition_member_lookup(
        &self,
        member: &MemberSpec,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<DefinitionMemberLookup> {
        if !self.profile.has_native_definition_workers()
            || !self
                .member(member.keyword)
                .is_some_and(|admitted| std::ptr::eq(admitted, member))
            || member
                .surface
                .is_some_and(|surface| !surface_admits(surface, dialect.as_ref()))
        {
            return None;
        }
        let namespace = self.profile.native_definition_namespace?;
        Some(DefinitionMemberLookup {
            namespace,
            implementation: tcl_syntax::naming::rooted_member_key(namespace, member.keyword),
        })
    }

    /// Actual private member slot in the selected definition receiver. The
    /// class-object `self` handler enters `::oo::objdefine`; its member table
    /// is independent of the outer `::oo::define` table.
    #[must_use]
    pub fn definition_member_lookup_for_receiver(
        &self,
        member: &MemberSpec,
        receiver: DefinitionReceiver,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<DefinitionMemberLookup> {
        let mut lookup = self.definition_member_lookup(member, dialect)?;
        if receiver == DefinitionReceiver::Class {
            if self.construction_member_effect(member)
                != MemberConstructionEffect::DeferredInstanceMethod
            {
                return None;
            }
            lookup.namespace = "::oo::objdefine";
            lookup.implementation =
                tcl_syntax::naming::rooted_member_key(lookup.namespace, member.keyword);
        }
        Some(lookup)
    }

    /// Whether a selected wrapper enters the class object's definition table.
    /// This is declaration metadata only, not default manufacture preservation.
    #[must_use]
    pub fn is_class_receiver_wrapper(&self, member: &MemberSpec) -> bool {
        self.profile.has_native_definition_workers()
            && member.keyword == "self"
            && self
                .member(member.keyword)
                .is_some_and(|known| std::ptr::eq(known, member))
    }

    /// Declaration visibility selected by an admitted access wrapper. The
    /// receiver wrapper carries no visibility effect; consumers retain its
    /// independent receiver axis instead of interpreting its spelling.
    #[must_use]
    pub fn wrapper_declared_visibility(
        &self,
        member: &MemberSpec,
    ) -> Option<DeclaredMemberVisibility> {
        if member.kind != MemberKind::Wrapper
            || !self
                .member(member.keyword)
                .is_some_and(|known| std::ptr::eq(known, member))
        {
            return None;
        }
        member.wrapper_shift.and_then(|shift| shift.visibility)
    }

    /// Direct or prefix-wrapper method declaration in original written words.
    /// Block wrappers retain a separate body carrier and do not use this query.
    #[must_use]
    pub fn receiver_method_layout(
        &self,
        keyword: &str,
        args: &[&str],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ReceiverMethodLayout> {
        self.receiver_method_layout_bytes(
            keyword.as_bytes(),
            &args.iter().map(|arg| arg.as_bytes()).collect::<Vec<_>>(),
            dialect,
        )
    }

    /// Original counted receiver method arguments. Only the selected worker
    /// keyword is text; a method name need not have a Unicode presentation.
    #[must_use]
    pub fn receiver_method_layout_bytes(
        &self,
        keyword: &[u8],
        args: &[&[u8]],
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<ReceiverMethodLayout> {
        let keyword = std::str::from_utf8(keyword).ok()?;
        let outer = self.member(keyword)?;
        if self.native_classmethod_declaration(outer, dialect) {
            return Some(ReceiverMethodLayout {
                receiver: DefinitionReceiver::Class,
                worker_receiver: DefinitionReceiver::Instance,
                class_delegate: true,
                member_word: 0,
                member: outer,
            });
        }
        let (receiver, member_word, member) = if self.is_class_receiver_wrapper(outer) {
            (
                DefinitionReceiver::Class,
                1,
                self.member(std::str::from_utf8(args.first()?).ok()?)?,
            )
        } else {
            (DefinitionReceiver::Instance, 0, outer)
        };
        if self.construction_member_effect(member)
            != MemberConstructionEffect::DeferredInstanceMethod
        {
            return None;
        }
        self.definition_member_lookup_for_receiver(member, receiver, dialect)?;
        Some(ReceiverMethodLayout {
            receiver,
            worker_receiver: receiver,
            class_delegate: false,
            member_word,
            member,
        })
    }

    /// Tcl 9's original classmethod worker creates a delegate procedure and
    /// an instance forward through myclass. It is distinct from self method;
    /// declaration identity supplies no delegate allocation or execution.
    #[must_use]
    pub fn native_classmethod_declaration(
        &self,
        member: &MemberSpec,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> bool {
        self.profile.has_native_definition_workers()
            && member.keyword == "classmethod"
            && dialect.is_some_and(|query| {
                query.core.nearest().is_some_and(|point| {
                    point.0 == tcl_dialect::model::Family::Tcl
                        && point
                            .1
                            .and_then(TclVersion::from_version_string)
                            .is_some_and(|version| version >= TclVersion::V9_0)
                })
            })
            && self.definition_member_lookup(member, dialect).is_some()
    }

    /// Original native delegate-declaration worker under an explicit release.
    #[must_use]
    pub fn native_classmethod_member(
        &self,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<&'static MemberSpec> {
        let member = self.member("classmethod")?;
        self.native_classmethod_declaration(member, dialect)
            .then_some(member)
    }

    /// A deferred delegate method whose name cannot replace a default class
    /// manufacturer or the unknown-method route. This does not prove that
    /// the owning delegate or the original declaration worker is current.
    #[must_use]
    pub fn native_classmethod_preserves_default_manufacturers(
        &self,
        member: &MemberSpec,
        name: &str,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> bool {
        self.native_classmethod_declaration(member, dialect)
            && self.manufacturer(name).is_none()
            && self.unknown_dispatch_method != Some(name)
    }

    /// Registry-owned construction classification of an admitted member.
    /// Callers must prove the definition command's implementation and validate
    /// the member's argument roles, parameters and literal body separately.
    /// This describes ordinary `TclOO` instance declarations only; filters,
    /// custom superclasses/metaclasses and construction hooks remain unknown.
    /// The separate single-superclass shape query grants no construction proof.
    #[must_use]
    pub fn construction_member_effect(&self, member: &MemberSpec) -> MemberConstructionEffect {
        if self.profile.has_native_definition_workers()
            && member.keyword == "method"
            && self
                .member(member.keyword)
                .is_some_and(|admitted| std::ptr::eq(admitted, member))
        {
            MemberConstructionEffect::DeferredInstanceMethod
        } else {
            MemberConstructionEffect::Unknown
        }
    }

    /// Intrinsic ordinary deferred method setter under an exact supported
    /// engine. This receipt supplies the worker's closed storage transfer,
    /// separately from construction effects, source metadata and body flow.
    /// Option-prefixed, wrapped and generic provider declarations stay outside
    /// this initial exact three-operand form.
    #[must_use]
    pub fn native_deferred_method_setter(
        &self,
        member: &MemberSpec,
        receiver: DefinitionReceiver,
        dialect: Option<SurfaceQuery<'_>>,
    ) -> Option<NativeDeferredMethodSetter> {
        let query = dialect?;
        let (family, version) = query.core.nearest()?;
        if family != tcl_dialect::model::Family::Tcl {
            return None;
        }
        let version = TclVersion::from_version_string(version?)?;
        let layout =
            tcl_native_model::native_tcloo_method_definition::NativeTclooMethodDefinitionProtocol::select(
                tcl_native_model::InvocationDialect::for_version(version),
            )?
            .layout(3)?;
        if self.construction_member_effect(member)
            != MemberConstructionEffect::DeferredInstanceMethod
        {
            return None;
        }
        self.definition_member_lookup_for_receiver(member, receiver, Some(query))?;
        if member.kind != MemberKind::Flat
            || member.arg_roles
                != [
                    (0, ArgRole::Name),
                    (1, ArgRole::ParamList),
                    (2, ArgRole::Body),
                ]
        {
            return None;
        }
        Some(NativeDeferredMethodSetter {
            recipe: tcl_syntax::naming::NativeNameProtocol::C(version),
            name: layout.name,
            parameters: layout.parameters,
            body: layout.body,
        })
    }

    /// Pure instance-forward registration layout under an exact C release.
    /// The prefix is stored as original values and is not invoked here. Current
    /// worker, target table and release effects remain caller obligations.
    #[must_use]
    pub fn native_deferred_forward_setter(
        &self,
        member: &MemberSpec,
        dialect: tcl_native_model::InvocationDialect,
        argument_count: usize,
    ) -> Option<tcl_syntax::naming::NativeNameProtocol> {
        // naming.tcloo.original-forward-registration-prefix
        // docs/design/analysis/name-resolution-proofs/tcloo-original-forward-registration-prefix.md
        let protocol = dialect.native_name_protocol()?;
        if !self.profile.has_native_definition_workers()
            || !matches!(protocol, tcl_syntax::naming::NativeNameProtocol::C(version)
                if version >= TclVersion::V8_6)
            || !self
                .member(member.keyword)
                .is_some_and(|known| std::ptr::eq(known, member))
            || member.keyword != "forward"
            || member.kind != MemberKind::Flat
            || member.arg_roles != [(0, ArgRole::Name), (1, ArgRole::CommandName)]
            || argument_count < 2
        {
            return None;
        }
        self.definition_member_lookup(member, dialect.authoring_query())?;
        Some(protocol)
    }

    /// The bare, single-base native superclass assignment. This selects only
    /// the original declaration shape; callers must prove the slot command,
    /// base allocation, inherited constructor and dispatch dependencies.
    #[must_use]
    pub fn single_native_superclass<'a>(
        &self,
        member: &MemberSpec,
        arguments: &[&'a str],
        dialect: Option<tcl_native_model::InvocationDialect>,
    ) -> Option<&'a str> {
        let (kind, name) = self.single_native_inherited_class(member, arguments, dialect)?;
        (kind == NativeInheritedClassKind::Superclass).then_some(name)
    }

    /// One original superclass or mixin assignment under the actual `TclOO`
    /// grammar. This selects a layout, not a live base or dispatch proof.
    #[must_use]
    pub fn single_native_inherited_class<'a>(
        &self,
        member: &MemberSpec,
        arguments: &[&'a str],
        dialect: Option<tcl_native_model::InvocationDialect>,
    ) -> Option<(NativeInheritedClassKind, &'a str)> {
        let dialect = dialect?;
        let kind = match member.keyword {
            "superclass" => NativeInheritedClassKind::Superclass,
            "mixin" => NativeInheritedClassKind::Mixin,
            _ => return None,
        };
        if !self.profile.has_native_definition_workers()
            || !self
                .member(member.keyword)
                .is_some_and(|known| std::ptr::eq(known, member))
            || dialect.family() != Some(tcl_dialect::model::Family::Tcl)
            || dialect
                .tcl_version
                .is_none_or(|version| version < TclVersion::V8_6)
        {
            return None;
        }
        let [base] = arguments else { return None };
        (!base.is_empty() && !base.starts_with('-')).then_some((kind, *base))
    }

    /// An original instance constructor whose entry can be retained separately
    /// from its callback effects and the manufactured object's eventual class.
    #[must_use]
    pub fn is_instance_constructor(&self, member: &MemberSpec) -> bool {
        self.profile.has_native_definition_workers()
            && member.keyword == "constructor"
            && self
                .member(member.keyword)
                .is_some_and(|known| std::ptr::eq(known, member))
    }

    /// Original instance destructor declaration, distinct from ordinary methods.
    #[must_use]
    pub fn is_instance_destructor(&self, member: &MemberSpec) -> bool {
        self.profile.has_native_definition_workers()
            && member.keyword == "destructor"
            && self
                .member(member.keyword)
                .is_some_and(|known| std::ptr::eq(known, member))
    }

    /// Original `TclOO` lifecycle declarations remove a method for an empty
    /// counted body. This describes declaration operands, not handler identity.
    #[must_use]
    pub fn native_lifecycle_body_disposition(
        &self,
        member: &MemberSpec,
        body: &[u8],
    ) -> Option<NativeLifecycleBodyDisposition> {
        if !self.is_instance_constructor(member) && !self.is_instance_destructor(member) {
            return None;
        }
        Some(if body.is_empty() {
            NativeLifecycleBodyDisposition::Removed
        } else {
            NativeLifecycleBodyDisposition::Retained
        })
    }

    /// Formal grammar applicability for one actual selected source member.
    /// Foreign member descriptors cannot borrow this grammar. A lifecycle's
    /// missing body value remains unknown; other members retain their parser.
    #[must_use]
    pub fn source_member_formal_validation_applicability(
        &self,
        member: &MemberSpec,
        body: Option<&[u8]>,
    ) -> Option<SourceFormalValidationApplicability> {
        if !self
            .member(member.keyword)
            .is_some_and(|known| std::ptr::eq(known, member))
        {
            return None;
        }
        if let Some(kind) = self.source_special_member_kind(member) {
            self.source_special_member_formal_validation_applicability(kind, body)
        } else {
            Some(SourceFormalValidationApplicability::Required)
        }
    }

    /// Source formal grammar for an independently retained special-member kind
    /// and its selected factory grammar. The caller supplies an authentic
    /// original static body value; dynamic or unavailable bodies stay unknown.
    #[must_use]
    pub fn source_special_member_formal_validation_applicability(
        &self,
        kind: DefinitionSpecialMemberKind,
        body: Option<&[u8]>,
    ) -> Option<SourceFormalValidationApplicability> {
        // naming.tcloo.empty-lifecycle-definition
        // docs/design/analysis/name-resolution-proofs/tcloo-empty-lifecycle-definition.md
        let keyword = match kind {
            DefinitionSpecialMemberKind::Constructor => "constructor",
            DefinitionSpecialMemberKind::Destructor => "destructor",
        };
        let member = self.member(keyword)?;
        (self.source_special_member_kind(member) == Some(kind)).then_some(())?;
        if !self.is_instance_constructor(member) && !self.is_instance_destructor(member) {
            return Some(SourceFormalValidationApplicability::Required);
        }
        self.native_lifecycle_body_disposition(member, body?)
            .map(|disposition| match disposition {
                NativeLifecycleBodyDisposition::Removed => {
                    SourceFormalValidationApplicability::SkippedRemovedLifecycle
                }
                NativeLifecycleBodyDisposition::Retained => {
                    SourceFormalValidationApplicability::Required
                }
            })
    }

    /// Actual `TclOO` constructor allocation and failure-cleanup protocol.
    /// Original handler, class and method-chain selection remain caller obligations.
    #[must_use]
    pub fn native_constructor_boundary(
        &self,
        dialect: tcl_native_model::InvocationDialect,
    ) -> Option<NativeConstructorBoundary> {
        (self.profile.has_native_definition_workers()
            && dialect.native_bootstrap_protocol()?.initializes_tcl_oo())
        .then_some(NativeConstructorBoundary::TclOo)
    }

    /// Whether a declaration leaves the selected constructor-entry chain
    /// unchanged. Inheritance, mixins, filters and wrappers require independent
    /// entry proofs and cannot borrow the local constructor inventory.
    #[must_use]
    pub fn preserves_local_constructor_entry(&self, member: &MemberSpec) -> bool {
        self.profile.has_native_definition_workers()
            && self
                .member(member.keyword)
                .is_some_and(|known| std::ptr::eq(known, member))
            && (matches!(
                member.keyword,
                "constructor" | "method" | "forward" | "destructor" | "variable" | "property"
            ) || member.visibility_effect.is_some())
    }

    /// Class-result identity on successful native manufacture is independent
    /// of instance constructor effects. Callers must prove this admitted
    /// member's native token, argument grammar and instance-definition context.
    /// Class-object wrappers, class methods and immediate initialisation hooks
    /// cannot preserve this contract without a separate dispatch proof.
    #[must_use]
    pub fn manufacture_member_effect(
        &self,
        member: &MemberSpec,
    ) -> MemberManufactureDispatchEffect {
        if self.profile.has_native_definition_workers()
            && self
                .member(member.keyword)
                .is_some_and(|admitted| std::ptr::eq(admitted, member))
            && matches!(
                member.keyword,
                "method"
                    | "constructor"
                    | "destructor"
                    | "variable"
                    | "superclass"
                    | "mixin"
                    | "filter"
                    | "export"
                    | "unexport"
                    | "deletemethod"
                    | "renamemethod"
                    | "forward"
                    | "property"
            )
        {
            MemberManufactureDispatchEffect::PreservesNativeFactory
        } else {
            MemberManufactureDispatchEffect::Unknown
        }
    }
}
