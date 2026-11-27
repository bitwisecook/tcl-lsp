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

//! Target-neutral resolution of one Tcl command invocation.
//!
//! [`ResolvedInvocation`] is the common compiler-facing projection of the
//! registry's command, subcommand, and form metadata. It deliberately carries
//! Tcl semantics and source-word facts, but not a backend code-generation
//! hook: target selection happens after common analysis has established that a
//! semantic operation is legal.

use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::body_kind::{BodyInterpreter, BodyKind};
#[cfg(test)]
use crate::command_table::CommandTableEffect;
use crate::completion::CompletionDescriptor;
use crate::dispatch_stability::ResolvedDispatchDependencies;
use crate::forms::CommandForm;
use crate::frame_effect::FrameEffectSpec;
use crate::hooks::{CodegenHookId, LoweringHookId};
use crate::hover::OptionSpec;
#[cfg(test)]
use crate::intrinsic::IntrinsicId;
#[cfg(test)]
use crate::invocation_words::InvocationWord;
use crate::invocation_words::InvocationWords;
use crate::literal_validation::{LiteralArgumentValidation, LiteralArgumentValidator};
use crate::option_effect::{OptionEffectScope, OptionEffects};
use crate::representation::RepresentationEffect;
use crate::result_stability::ResultStability;
use crate::semantic_operation::SemanticOperationId;
use crate::side_effects::SideEffect;
use crate::spec::{
    ArgRoleCountResolver, ArgRoleLayoutResolver, ArgRoleResolver, ArgRoleResolverInput,
    CommandSpec, SubCommand,
};
use crate::state_transition::ResolvedStateTransitions;
use crate::traits::Traits;
use crate::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
use crate::world_effect::ResolvedWorldEffects;
use tcl_dialect::model::{SpecSurface, SurfaceQuery};

mod argument_count;
mod argument_roles;
mod case_clauses;
mod channel_facts;
mod effects_and_transitions;
mod invocation_facts;
mod invocation_options;
mod operation_resolution;
mod source_definitions;
mod source_operands;
mod source_options;
mod source_scripts;
mod subcommand_resolution;
#[cfg(test)]
mod variable_receiver_tests;

pub use argument_count::{
    AuthoredSourceArity, InvocationArgumentCount, count_invocation_argument_layout,
    count_invocation_arguments, count_invocation_argv,
};
pub use argument_roles::AuthoredLogicalSourceRoleProjection;
pub use case_clauses::AuthoredSourceCaseBody;
pub use effects_and_transitions::ConditionalNoLevelAliasLayout;
pub use invocation_facts::{InvocationFacts, VariableReceiverOperandForm};
pub(crate) use invocation_options::invocation_options;
use invocation_options::selected_nested_options;
pub use invocation_options::{InvocationOptionOccurrence, InvocationOptions};
use operation_resolution::resolve_invocation_semantics;
pub(crate) use operation_resolution::{descriptor_operation, resolved_operation};
pub use source_definitions::{
    AuthoredSourceCommandPublication, AuthoredSourceCommandPublicationKind,
    AuthoredSourceJimClassLists, AuthoredSourceProcedureArguments,
    AuthoredSourceProcedureStaticList, NamedObjectFactory,
};
pub use source_operands::{
    AuthoredSourceAppendArguments, AuthoredSourceExpressionArguments, AuthoredSourceLambdaCall,
    AuthoredSourceScopeAliasArgument,
};
pub use source_options::{
    AuthoredSourceCredentialArgument, AuthoredSourceCredentialPosition,
    AuthoredSourceDestructivePathArguments, AuthoredSourceOption, AuthoredSourceOptionArguments,
    AuthoredSourceOptionBoundary, AuthoredSourceOptionRelationships, AuthoredSourceOptionScan,
    AuthoredSourceTeardownArguments,
};
pub use source_scripts::{AuthoredSourceSelfDispatch, SourceExecutableValueCapability};
pub use subcommand_resolution::{
    InvocationResolutionUnresolved, OwnedSubcommandResolution, ResolvedSubcommand,
    StructuredInvocationResolution, SubcommandResolution, SubcommandResolutionKind,
};

/// The semantic portion of an invocation form.
///
/// This is intentionally a projection rather than a borrowed
/// [`CommandForm`].  `CommandForm` still carries legacy backend hook fields;
/// exposing it from the common API would let a consumer accidentally couple
/// semantic resolution to a target emitter.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedForm<'r> {
    /// Registry-stable form name, such as `"implicit"` or `"flat_path"`.
    pub name: &'static str,
    /// The form's arity after the command head or subcommand word.
    pub arity: Arity,
    /// Form-local static argument roles.
    pub arg_roles: &'r [(u8, ArgRole)],
    /// Form-local options.  Shared options remain in
    /// [`InvocationOptions::base`].
    pub options: &'r [OptionSpec],
}

/// Availability selected once at semantic invocation ingress.
#[derive(Debug, Clone, Copy, Default)]
pub struct InvocationAvailability<'r> {
    /// Actual requested core/package surface, independent of operand syntax.
    pub query: Option<SurfaceQuery<'r>>,
    /// Explicit registry/profile floor of the owning package, never inferred from its name.
    pub package_version: Option<&'r str>,
}

/// Independently owned availability axes of an actual source instance method.
/// This typed transport preserves the selected provider and receiver purposes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InstanceMethodAvailability<'w> {
    pub(crate) provider: InvocationAvailability<'w>,
    pub(crate) receiver: InvocationAvailability<'w>,
}

/// Target-neutral semantic and effect descriptors for an invocation.
///
/// Every field is derived from command-registry data.  In particular this
/// structure has no `TclVM`, `WASM`, `BPF`, or other backend code-generation
/// hook.
/// `lowering_hook` is the existing common front-end structural-lowering
/// descriptor; it is retained so the current compiler can adopt this API
/// without changing behaviour.
#[derive(Debug, Clone, Copy)]
pub struct InvocationSemantics<'r, 'w> {
    script_metadata: crate::selected_script_timing::SelectedScriptMetadata<'r>,
    /// Selected named-object factory metadata. A candidate class does not
    /// establish a live object allocation or method implementation.
    pub named_object_factory: Option<NamedObjectFactory>,
    /// Registry-selected static semantic operation identity.
    ///
    /// This is not a live command identity; runtime command binding and trace
    /// state are established by later common analyses.
    pub operation: SemanticOperationId,
    /// Effective codegen descriptor under the same selected form, member and
    /// availability as this view. This is metadata, not native entry proof.
    pub codegen_hook: Option<CodegenHookId>,
    /// Effective registry-selected analyser semantic hook.
    pub analyser_hook: Option<crate::hooks::AnalyserHookId>,
    /// Effective target-neutral completion contract.
    ///
    /// The resolver selects a matching form first, then a resolved subcommand,
    /// then the command descriptor, falling back to the conservative generic
    /// invoke contract only when the registry supplied none of them: read it
    /// with [`crate::knowledge::Knowledge::or_conservative`].
    pub completion: crate::knowledge::Knowledge<CompletionDescriptor>,
    /// Registry-declared dependency contract for the invocation's result.
    pub result_stability: crate::knowledge::Knowledge<ResultStability>,
    /// Authored native result dependency, requiring actual implementation proof.
    pub native_result: Option<crate::native_result::NativeResultContract>,
    /// Effective Tcl value-representation effect.
    pub representation_effect: RepresentationEffect,
    /// Selected operand representation contracts, before argv layout projection.
    pub arg_types: &'static [(u8, crate::hooks::ArgTypeHint)],
    /// Additive command and subcommand behaviour traits.
    pub traits: Traits,
    /// Selected command's authored getter source colour, before argc selection.
    pub taint_source: Option<crate::taint::TaintColour>,
    /// Selected successful-result transform, before frozen operand validation.
    pub taint_transform: Option<crate::taint::SelectedTaintTransform>,
    /// Whether the selected command/subcommand/form mutates its receiver or
    /// other command-specific state.
    pub mutator: bool,
    /// Effective arity after command/subcommand/form resolution.
    pub arity: Arity,
    /// Number of leading post-head words before `arg_roles` starts.
    ///
    /// It is one for a subcommand form and zero otherwise.
    pub argument_offset: usize,
    /// Effective static argument-role declaration.
    pub arg_roles: &'r [(u8, ArgRole)],
    /// Dynamic argument-role resolver when the effective descriptor uses one.
    ///
    /// A matched form has only static roles and therefore supplies `None`.
    pub arg_role_resolver: Option<ArgRoleResolver>,
    /// Exact-cardinality resolver, independent of operand contents.
    pub arg_role_count_resolver: Option<ArgRoleCountResolver>,
    /// Native structured layout projection, retaining unknown operand values.
    pub arg_role_layout_resolver: Option<ArgRoleLayoutResolver>,
    /// Authored roles that remain possible when computed operands prevent
    /// selecting the resolver's exact positions. This never assigns a role
    /// to a particular argument or establishes a physical mutation.
    pub arg_role_resolver_roles: &'static [ArgRole],
    /// Structural repeating argument roles, independent of operand values.
    pub repeated_args: &'r [crate::RepeatedArgLayout],
    /// The effective shared clause grammar; forms retain their own static roles.
    pub clause_grammar: Option<&'r crate::clause_grammar::ClauseGrammarSpec>,
    /// Effective command/subcommand/form option descriptors.
    pub options: InvocationOptions<'r, 'w>,
    /// Where the options' effects come from — the families, reservation,
    /// prefix policy and inherited release gate of the selected option table
    /// (read by [`ResolvedInvocation::option_effects`]).
    pub option_scope: OptionEffectScope<'r>,
    /// Result Tcl internal-representation type, when declared.
    pub return_type: Option<TclType>,
    /// Authored native procedure definition grammar.
    pub procedure_definition: Option<crate::native_procedure::NativeProcedureDefinitionSpec>,
    /// Selected native compiler descriptor; syntax and compiler entry remain required.
    pub native_compilation: Option<crate::native_compilation::NativeCompilationSpec>,
    /// Successful handler transfer without compiler or opcode proof.
    pub successful_handler: Option<crate::native_compilation::SuccessfulHandlerSpec>,
    /// Selected receiver naming grammar. Argument indices are descriptor-local;
    /// `argument_offset` supplies the effective post-head coordinate. This does
    /// not prove a successful store or a normal read.
    pub variable_receivers: Option<&'static [(u8, VariableReceiverOperandForm)]>,
    /// Selected native byte-array transformation contract.
    pub byte_array_effect: crate::ByteArrayEffect,
    /// Selected payload getter/sink operand layout.
    pub byte_array_payload: Option<crate::BytePayloadSpec>,
    /// Dialects in which this invocation safely initialises an unset target.
    pub safe_on_uninit: Option<&'static [SpecSurface]>,
    /// How the invocation types variables it writes.
    pub var_write_typing: VarWriteTyping,
    /// Result-to-container-element relationship, when declared.
    pub return_elements: Option<ReturnElements>,
    /// In-place container-element evolution, when declared.
    pub var_elements_effect: Option<VarElementsEffect>,
    /// Frame-crossing argument grammar, when declared by the command.
    pub frame_effect: Option<FrameEffectSpec>,
    /// Whether body arguments run as ordinary scripts or structural bodies.
    pub body_kind: BodyKind,
    /// Registry-authored immediate script execution grammar.
    pub body_execution: Option<crate::body_execution::BodyExecutionSpec>,
    /// Which interpreter owns evaluated body arguments.
    pub body_interpreter: BodyInterpreter,
    /// Typed common front-end structural-lowering descriptor.
    ///
    /// This is not a target code-generation hook.  It is the existing
    /// registry-to-common-IR dispatch key and will ultimately become the
    /// semantic operation selected by the registry.
    pub lowering_hook: Option<LoweringHookId>,
    /// Structured side effects and how completely they are known.  A
    /// resolved subcommand that states any knowledge takes precedence over
    /// the command-level declaration, and a form's over both.
    pub side_effects: crate::knowledge::Knowledge<&'static [SideEffect]>,
    /// Effective declared mutable-world descriptor.
    ///
    /// Command, resolved-subcommand, and form descriptors in composition
    /// order.  Call [`ResolvedInvocation::effects`] to resolve this
    /// cheap descriptor chain and bridge existing effect metadata into the
    /// owned representation common compiler passes consume.
    pub world_effects: ResolvedWorldEffects,
    /// Effective command-binding and variable-cell transition descriptors.
    ///
    /// This remains borrowed until [`ResolvedInvocation::state_transitions`]
    /// materialises owned facts for a common consumer.
    pub state_transitions: ResolvedStateTransitions,
    /// Required live-interpreter stability proofs for this resolution.
    ///
    /// This descriptor chain is independent of backend guard encodings. An
    /// unstamped command resolves to the conservative Tcl dependency set.
    pub dispatch_dependencies: ResolvedDispatchDependencies,
    /// Registry-selected relationship/content validator for literal arguments.
    pub literal_argument_validator: Option<LiteralArgumentValidator>,
    /// The value-transfer declaration state, resolved with the selected
    /// subcommand and form: the innermost explicit declaration or
    /// abstention, or the specialisation derived from a descriptor stating
    /// the same operation (`docs/design/compiler/value-transfers.md`). The
    /// value axis is a projection of this resolution, never a second
    /// resolver.
    pub value: crate::value_transfer::ResolvedSemantics,
}

impl InvocationSemantics<'_, '_> {
    /// Resolver input contract; it does not imply cardinality is known.
    #[must_use]
    pub fn arg_role_resolver_input(&self) -> ArgRoleResolverInput {
        if [
            self.arg_role_resolver.is_some(),
            self.arg_role_count_resolver.is_some(),
            self.arg_role_layout_resolver.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count()
            > 1
        {
            ArgRoleResolverInput::ConflictingResolvers
        } else if self.arg_role_layout_resolver.is_some() {
            ArgRoleResolverInput::StructuredLayout
        } else if self.arg_role_count_resolver.is_some() {
            ArgRoleResolverInput::Cardinality
        } else {
            ArgRoleResolverInput::LiteralValues
        }
    }
}

/// A command invocation resolved to target-neutral registry semantics.
///
/// The structure retains caller word facts while exposing canonical names and
/// a backend-independent semantic projection. It neither assumes
/// that the command's runtime binding is immutable nor proves a native
/// specialisation is safe; later command-environment and trace analyses supply
/// those dynamic proofs.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedInvocation<'r, 'w> {
    /// Original command-head and post-head source-word facts.
    pub words: InvocationWords<'w>,
    /// Registry canonical command name.
    pub canonical_command: &'static str,
    /// Typed subcommand-resolution outcome, preserving the source spelling.
    pub subcommand: SubcommandResolution<'w>,
    /// Matched command or subcommand form, projected without backend hooks.
    pub form: Option<ResolvedForm<'r>>,
    /// Effective target-neutral semantic and effect descriptors.
    pub semantics: InvocationSemantics<'r, 'w>,
    /// The surface query the invocation was resolved under — `None` for a
    /// dialect-blind resolution. Every derived query answers under it.
    pub dialect: Option<SurfaceQuery<'w>>,
    /// The descriptors the registry selected — the derived queries' own
    /// inputs, beside the effective [`Self::semantics`].
    pub(crate) selected: SelectedDescriptors<'r>,
    // Actual inherited source provider; legacy uniform-floor issuers retain None.
    instance_method_provider: Option<(&'r CommandSpec, InvocationAvailability<'w>)>,
}

/// The command (or class) and subcommand (or instance method) descriptors an
/// invocation resolved to.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SelectedDescriptors<'r> {
    /// The command's descriptor — the class command's for an instance call.
    pub(crate) spec: &'r CommandSpec,
    /// The resolved subcommand, or the instance method.
    pub(crate) sub: Option<&'r SubCommand>,
    /// Whether `sub` is an instance method reached through an object, whose
    /// class command's own tables do not apply.
    pub(crate) instance: bool,
    /// Actual form selected by the original full argv, never its display name.
    pub(crate) form: Option<&'r CommandForm>,
    /// Actual numeric grammar used while selecting default/form syntax.
    pub(crate) numbers: Option<tcl_dialect::NumberSyntax>,
}

/// Registry descriptors already selected for readonly source assistance.
/// These references retain their selected availability and argv context;
/// they cannot establish installed handlers, execution or rewrite permission.
#[derive(Debug, Clone, Copy)]
pub struct AuthoredSourceDescriptors<'r> {
    /// The selected root command descriptor.
    pub command: &'r crate::CommandSpec,
    /// The selected subcommand descriptor, absent for unresolved selectors.
    pub subcommand: Option<&'r crate::SubCommand>,
}

/// Selector diagnosis from the same selected command and availability context.
/// This readonly result grants neither entered dispatch nor rewrite authority.
#[derive(Debug, Clone)]
pub enum AuthoredSourceSubcommandDiagnostic {
    /// A literal selector absent from the admitted table and all declared rows.
    Unknown {
        /// Canonical admitted spellings for suggestions.
        candidates: Vec<&'static str>,
    },
    /// A literal selector abbreviates several admitted rows.
    Ambiguous {
        /// Matching canonical spellings from the common keyword owner.
        candidates: Vec<&'static str>,
    },
    /// A declared row is excluded by the actual context.
    Disabled {
        /// Canonical spelling from the unfiltered descriptor table.
        canonical: &'static str,
        /// Explicit or inherited availability of that row.
        surface: Option<&'static [SpecSurface]>,
    },
}

/// Conditional source capability to retain a callback under selected metadata.
/// This proves no native handler, completed registration or callback entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCallbackRetention {
    /// This selected descriptor and source layout cannot retain a callback.
    Absent,
    /// A script or command-prefix callback may be retained.
    MayRetain,
    /// Callback-bearing options, selection or prefix layout remain unproved.
    Unproven,
}

impl<'r, 'w> ResolvedInvocation<'r, 'w> {
    /// Exact selected Registry label, independently of the original written head.
    #[must_use]
    pub fn descriptor_key(&self) -> crate::CanonicalCommandDescriptorKey<'static> {
        crate::CanonicalCommandDescriptorKey::from_registry_label(self.canonical_command)
    }

    /// Completion shape under this exact selected descriptor and frozen source
    /// argument dialect. This supplies no Native handler, successful completion,
    /// entered frame or result value. Missing argument grammar stays unknown.
    #[must_use]
    pub fn authored_source_invocation_completion(&self) -> crate::registry::InvocationCompletion {
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return crate::registry::InvocationCompletion::Unknown;
        }
        let Some(dialect) = self.words.arguments().dialect() else {
            return crate::registry::InvocationCompletion::Unknown;
        };
        crate::registry::invocation_completion_for_selected(
            self,
            tcl_syntax::number::Numbers::Target(dialect.numbers),
            crate::completion_route::ReturnInvocationGrammar::for_dialect(dialect),
        )
    }

    pub(crate) fn new(
        words: InvocationWords<'w>,
        spec: &'r CommandSpec,
        sub: Option<&'r SubCommand>,
        form: Option<&'r CommandForm>,
        subcommand: SubcommandResolution<'w>,
        availability: InvocationAvailability<'w>,
        numbers: Option<tcl_dialect::NumberSyntax>,
    ) -> Self {
        let mut semantics = resolve_invocation_semantics(
            spec,
            sub,
            form,
            true,
            words.arguments().dialect(),
            availability.query.as_ref(),
        );
        semantics.options = invocation_options(spec, sub, form, availability);
        semantics.options = selected_nested_options(semantics.options, spec, sub, words);
        if let Some(nested) = sub.and_then(|sub| {
            sub.nested_native_compilation(words.arguments().slice_from(semantics.argument_offset))
        }) {
            semantics.native_compilation = Some(nested);
        }
        if sub.is_none()
            && form.is_none_or(|form| form.return_type.is_none())
            && let Some(hook) = spec.return_type_hook
        {
            semantics.return_type =
                crate::return_type::resolve_arguments(hook, words.arguments(), semantics.options);
        }
        let dialect = availability.query;
        Self {
            words,
            canonical_command: spec.name,
            subcommand,
            form: form.map(|form| ResolvedForm {
                name: form.name,
                arity: form.arity,
                arg_roles: form.arg_roles,
                options: form.options,
            }),
            semantics,
            dialect,
            selected: SelectedDescriptors {
                spec,
                sub,
                instance: false,
                form,
                numbers,
            },
            instance_method_provider: None,
        }
    }

    pub(crate) fn new_instance(
        words: InvocationWords<'w>,
        class_spec: &'r CommandSpec,
        method: &'r SubCommand,
        form: Option<&'r CommandForm>,
        subcommand: SubcommandResolution<'w>,
        availability: InvocationAvailability<'w>,
    ) -> Self {
        let mut semantics = resolve_invocation_semantics(
            class_spec,
            Some(method),
            form,
            false,
            words.arguments().dialect(),
            availability.query.as_ref(),
        );
        semantics.options = invocation_options(class_spec, Some(method), form, availability);
        semantics.options =
            selected_nested_options(semantics.options, class_spec, Some(method), words);
        // A selected setter may configure the owning instance's option table.
        // Explicit method tables keep precedence; query forms carry no setter
        // trait and therefore cannot inherit executable constructor options.
        if semantics
            .traits
            .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
            && semantics.options.base.is_empty()
        {
            semantics.options.base = class_spec.options;
        }
        let dialect = availability.query;
        Self {
            words,
            canonical_command: class_spec.name,
            subcommand,
            form: form.map(|form| ResolvedForm {
                name: form.name,
                arity: form.arity,
                arg_roles: form.arg_roles,
                options: form.options,
            }),
            semantics,
            dialect,
            selected: SelectedDescriptors {
                spec: class_spec,
                sub: Some(method),
                instance: true,
                form,
                numbers: None,
            },
            instance_method_provider: None,
        }
    }

    /// Separate actual receiver identity from its selected inherited source provider.
    pub(crate) fn new_instance_with_method_provider(
        words: InvocationWords<'w>,
        class_spec: &'r CommandSpec,
        provider: &'r CommandSpec,
        method: &'r SubCommand,
        form: Option<&'r CommandForm>,
        subcommand: SubcommandResolution<'w>,
        availability: InstanceMethodAvailability<'w>,
    ) -> Result<Self, crate::registry::InstanceMethodContextUnavailable> {
        let InstanceMethodAvailability {
            provider: availability,
            receiver: receiver_availability,
        } = availability;
        let mut selected =
            Self::new_instance(words, class_spec, method, form, subcommand, availability);
        let mut options = invocation_options(provider, Some(method), form, availability);
        options = selected_nested_options(options, provider, Some(method), words);
        if selected
            .semantics
            .traits
            .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
            && options.base.is_empty()
            && !class_spec.options.is_empty()
        {
            // Receiver options retain their independent receiver package. The
            // current option card has one version axis for base and form; a
            // mixed card with unequal floors stays unavailable, not absent.
            if !options.form.is_empty()
                && availability.package_version != receiver_availability.package_version
            {
                return Err(crate::registry::InstanceMethodContextUnavailable::ReceiverOptionAxes);
            }
            options.base = class_spec.options;
            options.parent_surface = class_spec.surface;
            options.availability = receiver_availability;
        }
        selected.semantics.options = options;
        selected.semantics.option_scope.parent_surface = method.surface.or(provider.surface);
        selected.instance_method_provider = Some((provider, availability));
        Ok(selected)
    }

    /// Actual inherited provider and its own retained package/surface axis.
    /// The factory remains `authored_source_descriptors().command`. None from
    /// a legacy uniform-floor issuer supplies no provider or method-absence fact.
    #[must_use]
    pub const fn authored_source_instance_method_provider(
        &self,
    ) -> Option<(&'r CommandSpec, InvocationAvailability<'w>)> {
        self.instance_method_provider
    }

    /// The option-effect answer for this call
    /// (`docs/design/compiler/registry-consumer-contracts.md` § *Options with
    /// semantic effects*): the generic walk over the selected option table —
    /// the subcommand's own, when one was resolved — with the options
    /// available at the invocation's [`Self::dialect`].
    /// [`OptionEffects::option_end`] is a post-head argument index, like
    /// every other index this resolution answers.
    #[must_use]
    pub fn option_effects(&self) -> OptionEffects {
        self.option_effects_for_arguments(self.words.arguments())
    }

    fn option_effects_for_arguments(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> OptionEffects {
        let scope = self.semantics.option_scope;
        let options = self.semantics.options.available().collect::<Vec<_>>();
        let offset = self.semantics.argument_offset;
        let mut effects = crate::option_effect::option_effects_over(
            &options,
            scope.families,
            arguments.slice_from(offset),
            scope.reserved_trailing_words,
            self.dialect,
            scope.prefix_matching,
        );
        effects.option_end += offset;
        effects
    }

    /// Interpret proposed advisory operands under this already selected
    /// descriptor's option grammar and complete availability. This pure
    /// projection neither reselects a command nor creates original argv,
    /// accepted handler arguments or successful evaluation.
    #[must_use]
    pub fn authored_source_substitution_proposal(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::substitution::SubstitutionKinds> {
        if !self
            .semantics
            .traits
            .contains(Traits::PERFORMS_SUBSTITUTION)
        {
            return None;
        }
        let count = arguments.exact_argv_len()?;
        let effects = self.option_effects_for_arguments(arguments);
        (effects.complete
            && effects.option_end + self.semantics.option_scope.reserved_trailing_words >= count)
            .then(|| effects.substitution_kinds())
    }

    /// Which substitutions this call performs over its own argument text, or
    /// `None` when the command performs none — the projection of
    /// [`Self::option_effects`] onto
    /// [`crate::substitution::SubstitutionKinds`], with the rule
    /// [`crate::CommandSpec::substitutions_performed`] states: an unreadable
    /// call, or one whose option run stops before the reserved operands,
    /// performs every kind.
    #[must_use]
    pub fn substitutions_performed(&self) -> Option<crate::substitution::SubstitutionKinds> {
        if !self
            .semantics
            .traits
            .contains(Traits::PERFORMS_SUBSTITUTION)
        {
            return None;
        }
        let effects = self.option_effects();
        let reaches_operands = self.words.arguments().exact_argv_len().is_some_and(|len| {
            effects.option_end + self.semantics.option_scope.reserved_trailing_words >= len
        });
        Some(if reaches_operands {
            effects.substitution_kinds()
        } else {
            crate::substitution::SubstitutionKinds::ALL
        })
    }

    /// Every word's literal value, a computed word standing in as an inert
    /// empty placeholder — or `None` when an expansion makes the argv shape
    /// unknown. The spelling projection the registry's position-only readers
    /// take; never read a placeholder as a value.
    fn placeholder_spellings(&self) -> Option<Vec<&'w str>> {
        let arguments = self.words.arguments();
        arguments.has_exact_argv_len().then(|| {
            (0..arguments.len())
                .map(|index| arguments.literal_at(index).unwrap_or(""))
                .collect()
        })
    }

    /// The release the invocation's [`Self::dialect`] names on the Tcl
    /// ladder, for a rule whose grammar is a release's numerals.
    fn tcl_version(&self) -> Option<tcl_dialect::TclVersion> {
        match self.dialect?.core.nearest() {
            Some((tcl_dialect::model::Family::Tcl, Some(release))) => {
                tcl_dialect::TclVersion::from_version_string(release)
            }
            _ => None,
        }
    }

    /// The selected command's options available at [`Self::dialect`].
    fn spec_options(&self) -> Vec<&'static OptionSpec> {
        self.selected.spec.option_specs(self.dialect)
    }

    /// Whether the words prove the layout a resolver-derived role depends on
    /// — no expansion, a literal subcommand word, and an option run whose
    /// every word is literal while an option could stand: the registry's
    /// source-layout proof over this resolution's own selection.
    fn layout_is_proven(&self) -> bool {
        let SelectedDescriptors { spec, sub, .. } = self.selected;
        let arguments = self.words.arguments();
        if !spec.subcommands.is_empty()
            && !arguments.is_empty()
            && arguments.literal_at(0).is_none()
        {
            return false;
        }
        crate::registry::layout_is_proven_in(
            spec,
            sub,
            arguments,
            || self.spec_options(),
            |sub| crate::registry::sub_options_at(spec, sub, self.dialect),
        )
    }

    /// The type this call's result is represented as —
    /// `CommandSpec::return_type_for_call` re-keyed on the resolution: the
    /// selected subcommand's (or instance method's) declared type for a
    /// command with subcommands, else the command's return-type hook over the
    /// words, else the command's declared type.
    ///
    /// `None` when the type is unknown for this call — a subcommand the call
    /// does not select, a hook that cannot tell (a computed word where a
    /// switch could stand), or a `{*}` expansion under a hook.
    #[must_use]
    pub fn return_type(&self) -> Option<TclType> {
        self.semantics.return_type
    }

    /// Validate registry-declared relationships between literal arguments.
    ///
    /// An absent descriptor is a conservative abstention: it is never treated
    /// as proof that arbitrary command arguments are valid.
    #[must_use]
    pub fn validate_literal_arguments(&self) -> Option<LiteralArgumentValidation> {
        self.semantics
            .literal_argument_validator
            .map(|validator| validator(self.words.arguments()))
    }

    fn uses_authored_jim_core_layout(&self) -> bool {
        self.semantics.procedure_definition
            == Some(crate::native_procedure::NativeProcedureDefinitionSpec::Core)
            && self.semantics.argument_offset == 0
            && self.semantics.frame_effect.is_none()
            && self
                .semantics
                .traits
                .contains(Traits::DEFINES_PROCEDURE | Traits::DEFERS_BODY)
            && self
                .words
                .arguments()
                .dialect()
                .and_then(tcl_native_model::InvocationDialect::parameter_grammar)
                == Some(tcl_dialect::ParameterGrammar::Jim)
    }

    /// Conservative registration capability independent of execution coverage.
    /// Body-valued options are outside the older prefix-retention projection;
    /// their absence therefore needs its own complete selected-layout proof.
    fn selected_source_callback_retention(&self) -> SourceCallbackRetention {
        if matches!(
            self.subcommand,
            SubcommandResolution::Indeterminate { .. }
                | SubcommandResolution::Unknown { .. }
                | SubcommandResolution::Ambiguous { .. }
        ) {
            return SourceCallbackRetention::Unproven;
        }
        if self
            .semantics
            .options
            .available()
            .any(|option| option.value_role() == Some(ArgRole::Body))
        {
            return SourceCallbackRetention::Unproven;
        }
        match self.authored_source_retains_callback() {
            Some(false) => SourceCallbackRetention::Absent,
            Some(true) => SourceCallbackRetention::MayRetain,
            None => SourceCallbackRetention::Unproven,
        }
    }

    /// Priority grammar of the command selected by this original source schema.
    #[must_use]
    pub fn authored_event_handler_priority(&self) -> Option<crate::events::EventHandlerPriority> {
        self.semantics
            .script_metadata
            .command
            .event_handler_priority
    }

    fn authored_source_option_selector_path(&self) -> Option<Vec<&'static str>> {
        if !matches!(
            self.subcommand,
            SubcommandResolution::NotApplicable
                | SubcommandResolution::Exact(_)
                | SubcommandResolution::UniquePrefix(_)
        ) {
            return None;
        }
        let selected = self.semantics.script_metadata;
        let options = self.semantics.options;
        let mut subcommands = selected
            .subcommand
            .map(|sub| sub.name)
            .into_iter()
            .collect::<Vec<_>>();
        if let Some(sub) = selected
            .subcommand
            .filter(|sub| !sub.sub_subcommands.is_empty())
        {
            let word = self.words.arguments().literal_at(1)?;
            let nested = sub.resolve_sub_subcommand_gated(
                word,
                options.availability.query,
                options.availability.package_version,
            )?;
            subcommands.push(nested.name);
        }
        Some(subcommands)
    }

    /// Formatter presentation for an actual authored Body operand. The
    /// independently selected subcommand owns its local presentation table;
    /// unknown role/cardinality remains absent. This is a style preference,
    /// not script entry, runtime equivalence or rewrite permission.
    #[must_use]
    pub fn authored_source_argument_presentation(
        &self,
        argument: usize,
    ) -> Option<crate::presentation::ArgPresentation> {
        let count = self.words.arguments().exact_argv_len()?;
        if argument >= count {
            return None;
        }
        let local = argument.checked_sub(self.semantics.argument_offset)?;
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete
            || !roles
                .iter()
                .any(|&(index, role)| usize::from(index) == local && role == ArgRole::Body)
        {
            return None;
        }
        let selected = self.semantics.script_metadata;
        let table = selected
            .subcommand
            .map_or(selected.command.arg_presentation, |subcommand| {
                subcommand.arg_presentation
            });
        Some(
            table
                .iter()
                .find(|&&(index, _)| usize::from(index) == local)
                .map_or_else(
                    crate::presentation::ArgPresentation::default,
                    |&(_, presentation)| presentation,
                ),
        )
    }

    /// Diagnose an original literal selector with the already selected table.
    /// Default forms, factories, dynamic selectors and open vocabularies decline.
    /// Only an excluded declared row uses the unfiltered table, to explain its
    /// availability; accepted and ambiguous prefixes use the actual vocabulary.
    #[must_use]
    pub fn authored_source_subcommand_diagnostic(
        &self,
    ) -> Option<AuthoredSourceSubcommandDiagnostic> {
        let command = self.semantics.script_metadata.command;
        if command.allow_unknown_subcommands {
            return None;
        }
        let (SubcommandResolution::Unknown { spelling }
        | SubcommandResolution::Ambiguous { spelling }) = self.subcommand
        else {
            return None;
        };
        let table = command.subcommand_table(
            self.semantics.options.availability.query,
            self.semantics.options.availability.package_version,
            None,
        );
        if let crate::abbrev::KeywordMatch::Ambiguous(candidates) = table.resolve(spelling) {
            return Some(AuthoredSourceSubcommandDiagnostic::Ambiguous { candidates });
        }
        if let crate::abbrev::KeywordMatch::Unique(canonical) =
            command.resolve_subcommand_word(spelling, None, None, None)
        {
            let sub = command.subcommand(canonical)?;
            return Some(AuthoredSourceSubcommandDiagnostic::Disabled {
                canonical,
                surface: sub.surface.or(command.surface),
            });
        }
        Some(AuthoredSourceSubcommandDiagnostic::Unknown {
            candidates: table.names().collect(),
        })
    }

    /// Lexical context advice from the same selected descriptor and argv.
    /// No execution frame or native return capability is supplied here.
    #[must_use]
    pub fn authored_source_context_gate(&self, in_event_body: bool) -> Option<&'static str> {
        (self.semantics.script_metadata.command.context_gate?)(
            self.words.arguments(),
            in_event_body,
        )
    }

    /// Names of a consecutive optional trailing role run in this exact
    /// selected descriptor. Dynamic layouts decline rather than invent values.
    /// Each returned name is synopsis metadata, not a variable allocation.
    #[must_use]
    pub fn authored_source_optional_trailing_names(&self, role: ArgRole) -> Vec<&'static str> {
        let arguments = self.words.arguments();
        let Some(count) = arguments.exact_argv_len() else {
            return Vec::new();
        };
        if self.semantics.argument_offset != 0 || self.semantics.arg_role_layout_resolver.is_some()
        {
            return Vec::new();
        }
        let roles = if let Some(resolve) = self.semantics.arg_role_count_resolver {
            resolve(count)
        } else if let Some(resolve) = self.semantics.arg_role_resolver {
            let Some(values) = arguments.literal_values() else {
                return Vec::new();
            };
            resolve(&values)
        } else {
            self.semantics.arg_roles.to_vec()
        };
        let ceiling = usize::from(self.semantics.arity.max);
        let run = (count..ceiling)
            .take_while(|index| {
                roles.iter().any(|&(position, selected)| {
                    usize::from(position) == *index && selected == role
                })
            })
            .count();
        self.semantics
            .script_metadata
            .command
            .optional_trailing_arg_names(self.semantics.options.availability.query, None)
            .into_iter()
            .take(run)
            .collect()
    }

    /// Borrow the same selected descriptors for readonly lifecycle and syntax
    /// assistance. Canonical reporting names must not be looked up afresh.
    #[must_use]
    pub const fn authored_source_descriptors(&self) -> AuthoredSourceDescriptors<'r> {
        AuthoredSourceDescriptors {
            command: self.semantics.script_metadata.command,
            subcommand: self.semantics.script_metadata.subcommand,
        }
    }

    /// Retain actual descriptor/form selection after one proposed literal
    /// replacement. This is source selection correspondence only, never
    /// operand equivalence, handler success or edit permission. Instance
    /// dispatch keeps its independent receiver/provider admission.
    #[must_use]
    pub fn source_literal_replacement_keeps_selection(&self, argument: usize, value: &str) -> bool {
        self.source_literal_replacements_keep_selection(&[(argument, value)])
    }

    /// Joint source selection correspondence for literal replacement candidates.
    /// Individually stable replacements must not jointly select another form.
    /// This remains metadata correspondence, not an operand edit grant.
    #[must_use]
    pub fn source_literal_replacements_keep_selection(
        &self,
        replacements: &[(usize, &str)],
    ) -> bool {
        if self.selected.instance
            || replacements
                .iter()
                .enumerate()
                .any(|(position, (argument, _))| {
                    self.words.arguments().literal_at(*argument).is_none()
                        || replacements[..position]
                            .iter()
                            .any(|(earlier, _)| earlier == argument)
                })
        {
            return false;
        }
        let Some(count) = self.words.arguments().exact_argv_len() else {
            return false;
        };
        let Some(arguments) = (0..count)
            .map(|index| {
                replacements
                    .iter()
                    .find(|(argument, _)| *argument == index)
                    .map_or_else(
                        || self.words.arguments().get(index),
                        |(_, value)| Some(crate::InvocationWord::Literal(value)),
                    )
            })
            .collect::<Option<Vec<_>>>()
        else {
            return false;
        };
        let mut words = InvocationWords::structured(self.words.head(), &arguments);
        if let Some(dialect) = self.words.arguments().dialect() {
            words = words.with_dialect(dialect);
        }
        let proposed = crate::registry::resolve_source_descriptor_invocation(
            self.selected.spec,
            words,
            self.semantics.options.availability,
            self.selected.numbers,
        );
        std::ptr::eq(self.selected.spec, proposed.selected.spec)
            && match (self.selected.sub, proposed.selected.sub) {
                (Some(left), Some(right)) => std::ptr::eq(left, right),
                (None, None) => true,
                _ => false,
            }
            && match (self.selected.form, proposed.selected.form) {
                (Some(left), Some(right)) => std::ptr::eq(left, right),
                (None, None) => true,
                _ => false,
            }
            && self.semantics.argument_offset == proposed.semantics.argument_offset
            && std::ptr::eq(self.semantics.options.base, proposed.semantics.options.base)
            && std::ptr::eq(self.semantics.options.form, proposed.semantics.options.form)
            && self.semantics.options.positional_prefix_words
                == proposed.semantics.options.positional_prefix_words
            && self.semantics.options.reserved_trailing_words
                == proposed.semantics.options.reserved_trailing_words
    }

    /// A procedure definition's dialect-selected arity, else the declared arity.
    fn selected_arity(&self) -> Arity {
        self.semantics
            .procedure_definition
            .map_or(self.semantics.arity, |descriptor| {
                descriptor.arity(self.words.arguments().dialect())
            })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_source_presentation_tests;

#[cfg(test)]
mod logical_binding_result_tests;

#[cfg(test)]
mod logical_frame_source_role_tests;

#[cfg(test)]
mod scope_alias_metadata_tests;

#[cfg(test)]
mod logical_source_alias_projection_tests;

#[cfg(test)]
mod literal_role_dependency_controls;

#[cfg(test)]
mod selected_effect_hint_controls;
