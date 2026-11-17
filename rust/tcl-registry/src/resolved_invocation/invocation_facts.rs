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

//! `InvocationFacts`: the owned summary of one resolved invocation, its queries and how it is built on each completion edge.

use super::source_definitions::NamedObjectFactory;
use super::subcommand_resolution::{OwnedSubcommandResolution, SubcommandResolution};
use super::{ResolvedInvocation, SourceCallbackRetention};
use crate::arg_role::ArgRole;
use crate::arity::Arity;
use crate::body_kind::{BodyInterpreter, BodyKind};
use crate::completion::CompletionDescriptor;
use crate::dispatch_stability::DispatchDependencies;
use crate::frame_effect::FrameEffectSpec;
use crate::hooks::LoweringHookId;
use crate::intrinsic::IntrinsicId;
use crate::representation::RepresentationEffect;
use crate::result_stability::ResultStability;
use crate::semantic_operation::SemanticOperationId;
use crate::state_transition::{StateTransitionKnowledge, StateTransitions};
use crate::traits::Traits;
use crate::types::{ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
use crate::world_effect::{EffectFootprint, TransitionEffectCoverages};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SelectedArgumentTypeHints {
    table: &'static [(u8, crate::hooks::ArgTypeHint)],
    first_positional: usize,
}

impl SelectedArgumentTypeHints {
    pub(super) fn at(self, index: usize) -> Option<&'static crate::hooks::ArgTypeHint> {
        let relative = u8::try_from(index.checked_sub(self.first_positional)?).ok()?;
        self.table
            .iter()
            .find(|(position, _)| *position == relative)
            .map(|(_, hint)| hint)
    }
}

/// An owned, target-neutral projection of a resolved registry invocation.
///
/// This is the hand-off shape for executable IR and shared optimisation
/// passes.  It contains facts selected by the registry, not a claim that the
/// command's runtime binding, namespace lookup, aliases, traces, or `unknown`
/// handling have been proven stable.  It deliberately excludes every backend
/// and lowering hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationFacts {
    source_callback_retention: SourceCallbackRetention,
    deferred_script_arguments: Option<Vec<usize>>,
    script_lookup_arguments: Option<Vec<(usize, crate::ScriptLookupScope)>>,
    command_prefix_arguments: Option<Vec<(usize, crate::AppendedArity)>>,
    argument_type_hints: Option<SelectedArgumentTypeHints>,
    /// Selected normal naming-factory contract, independent of object identity.
    pub named_object_factory: Option<NamedObjectFactory>,
    /// Canonical registry command identity used to select these facts.
    pub canonical_command: String,
    /// The complete owned subcommand-resolution outcome.
    pub subcommand: OwnedSubcommandResolution,
    /// Registry form identity, when a form was selected from a determinate
    /// argv shape.
    pub form: Option<String>,
    /// Target-neutral semantic operation selected by the registry.
    pub operation: SemanticOperationId,
    /// Selected logical structural hook, independently of native admission.
    pub lowering_hook: Option<LoweringHookId>,
    /// Effective registry-selected analyser semantic hook.
    pub analyser_hook: Option<crate::hooks::AnalyserHookId>,
    /// Readonly collection lifecycle metadata from the actual selected command
    /// descriptor. This supplies no reached operation or completed effect.
    pub data_collection: Option<crate::events::DataCollectionOperation>,
    /// Readonly execution-side metadata from that same descriptor; body entry
    /// and original source correspondence remain separate requirements.
    pub side_switch_target: Option<crate::side_effects::SideSwitchTarget>,
    /// Available selected option vocabulary, including its actual abbreviation
    /// policy. This metadata establishes no accepted operands or handler.
    pub option_keywords: crate::abbrev::KeywordTable<'static>,
    /// Conditional flow action from the actual selected source grammar.
    pub irules_source_flow_action: Option<crate::events::IrulesSourceFlowAction>,
    /// Possible response commitment under this descriptor's actual context.
    pub source_may_commit_response: bool,
    /// Effective Tcl completion descriptor.
    pub completion: CompletionDescriptor,
    /// Registry-declared dependency contract for the invocation's result.
    pub result_stability: ResultStability,
    /// Authored native result dependency, requiring actual implementation proof.
    pub native_result: Option<crate::native_result::NativeResultContract>,
    /// Effective Tcl value-representation effect.
    pub representation_effect: RepresentationEffect,
    /// Native coercion phase projected from the selected form and actual argv.
    /// This can affect shared objects without writing their variable cells.
    pub operand_representation_coercions: crate::representation::RepresentationCoercionSelection,
    /// Fully resolved mutable-world footprint.
    ///
    /// Writes covered by [`Self::transition_effect_coverage`] have already
    /// been removed here, while independent reads, clobbers, callbacks, and
    /// other domains remain present.
    pub effects: EffectFootprint,
    /// Complete registry transition facts, or a typed wildcard obligation for
    /// an unstamped generic Tcl invocation.
    pub state_transitions: StateTransitionKnowledge,
    /// Registry contract identifying legacy or explicit writes whose
    /// completion-edge transition facts are authoritative.
    pub transition_effect_coverage: TransitionEffectCoverages,
    /// Mutable Tcl domains that must be proven stable before specialising this
    /// statically resolved invocation.
    pub dispatch_dependencies: DispatchDependencies,
    /// Effective command and subcommand traits.
    pub traits: Traits,
    /// Authored successful result source colour. Actual implementation proof
    /// remains required; this is not catalogue-based source classification.
    pub taint_source: Option<crate::taint::TaintColour>,
    /// Successful-result transform validated against actual frozen operands.
    /// Handler provenance remains an independent consumer requirement.
    pub taint_transform: Option<crate::taint::TaintColour>,
    /// Whether the selected invocation form mutates command-specific state.
    pub mutator: bool,
    /// Effective arity after command, subcommand, and form selection.
    pub arity: Arity,
    /// Count under the authored option/positional grammar of the frozen argv.
    /// Unknown option layout retains `None`, rather than a guessed argc.
    pub arity_argument_count: Option<u16>,
    /// Actual frozen post-head argv cardinality, independent of count axis.
    pub frozen_argument_count: Option<usize>,
    /// Number of leading post-head words before static argument roles start.
    pub argument_offset: usize,
    /// Effective argument-role declarations for this invocation.
    ///
    /// A registry resolver is evaluated here when every argument is literal.
    /// Otherwise this retains the descriptor's conservative static roles and
    /// [`Self::arg_roles_complete`] is `false`.
    pub arg_roles: Vec<(u8, ArgRole)>,
    /// Repeated layouts retaining conditional-binding policy for consumers.
    pub repeated_args: Vec<crate::RepeatedArgLayout>,
    /// Whether [`Self::arg_roles`] is the complete role assignment.
    ///
    /// `false` means registry metadata has an argument-role resolver whose
    /// result depends on literal invocation arguments. Consumers must retain
    /// the dynamic role obligation rather than treating this static slice as a
    /// complete answer.
    pub arg_roles_complete: bool,
    /// Closed capability inventory for unresolved value-dependent role positions.
    /// Meaningful as uncertainty only while `arg_roles_complete` is false.
    pub arg_role_resolver_roles: &'static [ArgRole],
    /// Declared result internal-representation type, when available.
    pub return_type: Option<TclType>,
    /// Native procedure argv selection, preserving invalid/unknown outcomes.
    pub procedure_definition: Option<crate::native_procedure::NativeProcedureDefinitionSelection>,
    /// Native compilation capability independent of runtime target selection.
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
    /// How written variables receive types from this invocation.
    pub var_write_typing: VarWriteTyping,
    /// Result-to-container-element relationship, when declared.
    pub return_elements: Option<ReturnElements>,
    /// In-place container-element evolution, when declared.
    pub var_elements_effect: Option<VarElementsEffect>,
    /// Whether body words run in the caller frame or a structural scope.
    pub body_kind: BodyKind,
    /// Registry-authored immediate script execution grammar.
    pub body_execution: Option<crate::body_execution::BodyExecutionSpec>,
    /// Which interpreter owns evaluated body arguments.
    pub body_interpreter: BodyInterpreter,
    /// Frame-crossing argument grammar, when declared.
    pub frame_effect: Option<FrameEffectSpec>,
}

/// Selected runtime variable-name receiver grammar, independently of lexical
/// roots, compiler locals, namespace alias declarations and successful access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableReceiverOperandForm {
    /// A single argv object may contain a combined root and element spelling.
    Combined,
    /// A trace variable subject uses its independently selected name ingress
    /// before separating a root and element. This grants no trace operation.
    TraceSubject,
}

impl VariableReceiverOperandForm {
    /// Purpose-selected original operand geometry. Registration, observer
    /// lifetime and successful access remain independent of this projection.
    #[must_use]
    pub fn input_form(
        self,
        protocol: tcl_syntax::naming::NativeNameProtocol,
        original: &[u8],
    ) -> Option<tcl_syntax::naming::NativeVariableInputForm<'_>> {
        use tcl_syntax::naming::NativeVariableInputForm;
        let selected = match self {
            Self::Combined => original,
            Self::TraceSubject => protocol
                .trace_query_input(original)
                .ok()?
                .borrowed_selected()?,
        };
        Some(NativeVariableInputForm::Combined(selected))
    }
}

fn selected_variable_receiver_operand_form(
    argument: usize,
    argument_offset: usize,
    roles: &[(u8, ArgRole)],
    traits: Traits,
    receivers: Option<&[(u8, VariableReceiverOperandForm)]>,
    handler: Option<crate::native_compilation::SuccessfulHandlerSpec>,
    compiler: Option<crate::native_compilation::NativeCompilationSpec>,
) -> Option<VariableReceiverOperandForm> {
    use crate::native_compilation::{
        NativeCompilationGrammar as Grammar, SuccessfulHandlerSpec as Handler,
    };
    if traits.contains(Traits::CREATES_SCOPE_ALIAS)
        || !roles.iter().any(|&(index, role)| {
            argument_offset.checked_add(usize::from(index)) == Some(argument)
                && matches!(role, ArgRole::VarRead | ArgRole::VarWrite)
        })
    {
        return None;
    }
    if let Some(receivers) = receivers {
        return receivers.iter().find_map(|&(index, form)| {
            (argument_offset.checked_add(usize::from(index)) == Some(argument)).then_some(form)
        });
    }
    let handler = matches!(
        handler,
        Some(
            Handler::VariableOperands
                | Handler::ConditionalVariableOperands(_)
                | Handler::InitialiseEmptyVariable
                | Handler::CatchOutputs
                | Handler::DictionaryScope
        )
    );
    let compiler = compiler.is_some_and(|spec| {
        matches!(
            spec.grammar,
            Grammar::VariableLoadStore
                | Grammar::Increment
                | Grammar::VariableAppend(_)
                | Grammar::InfoExists
                | Grammar::ListAssignment
                | Grammar::Catch
                | Grammar::Try
        )
    });
    (handler || compiler).then_some(VariableReceiverOperandForm::Combined)
}

impl InvocationFacts {
    /// Whether the selected invocation is declared free of side effects.
    #[must_use]
    pub const fn is_pure(&self) -> bool {
        self.traits.contains(Traits::PURE)
    }

    /// Whether the invocation is pure and a candidate for common
    /// subexpression elimination.
    #[must_use]
    pub const fn is_cse_pure(&self) -> bool {
        self.traits
            .contains(Traits::PURE.union(Traits::CSE_CANDIDATE))
    }

    /// Whether evaluating the invocation changes no state: a pure command, or
    /// one whose evaluation is pure (a braced `expr`).
    #[must_use]
    pub const fn evaluates_purely(&self) -> bool {
        self.traits
            .intersects(Traits::PURE.union(Traits::PURE_EVALUATION))
    }

    /// Withdraw every purity claim, and the frameless-runtime claim that
    /// rests on it, once the invocation is known to call back into a host.
    pub fn withdraw_purity(&mut self) {
        self.traits.remove(
            Traits::PURE
                .union(Traits::PURE_EVALUATION)
                .union(Traits::FRAMELESS_RUNTIME),
        );
    }

    /// Selected source retention capability, frozen with these same facts.
    /// Absence concerns this invocation only, never existing hooks or native
    /// implementation identity. Unproven remains a callback guard obligation.
    #[must_use]
    pub const fn source_callback_retention(&self) -> SourceCallbackRetention {
        self.source_callback_retention
    }

    /// Pure key of the exact descriptor that produced these owned facts.
    /// This supplies no command-table or source naming authority.
    #[must_use]
    pub fn descriptor_key(&self) -> crate::CanonicalCommandDescriptorKey<'_> {
        crate::CanonicalCommandDescriptorKey::from_registry_label(&self.canonical_command)
    }

    /// Original argv receiver form selected by the actual handler descriptor.
    /// A nominal variable role alone cannot select this protocol. Whole-array
    /// operations still parse a combined name; an element may be a guest error.
    #[must_use]
    pub fn variable_receiver_operand_form(
        &self,
        argument: usize,
    ) -> Option<VariableReceiverOperandForm> {
        if !self.arg_roles_complete || self.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        selected_variable_receiver_operand_form(
            argument,
            self.argument_offset,
            &self.arg_roles,
            self.traits,
            self.variable_receivers,
            self.successful_handler,
            self.native_compilation,
        )
    }

    /// Selected deferred single-script/prefix operand positions. Unknown values
    /// keep their slots; None retains unknown layout, timing or concatenation.
    /// This metadata supplies positive navigation only, never callback absence,
    /// execution, future body admission, edit completeness or erasure permission.
    #[must_use]
    pub fn deferred_script_argument_indices(&self) -> Option<&[usize]> {
        self.deferred_script_arguments.as_deref()
    }

    /// Lookup-frame purpose of one selected executable argument. The index
    /// addresses effective post-head argv, including ensemble selectors.
    /// No receiving-head site, future table, callback reach or normal entry is
    /// implied. Unknown positions and reference-only forms return `None`.
    #[must_use]
    pub fn script_lookup_scope(&self, argument_index: usize) -> Option<crate::ScriptLookupScope> {
        self.script_lookup_arguments
            .as_ref()?
            .iter()
            .find_map(|&(index, scope)| (index == argument_index).then_some(scope))
    }

    /// Appended argument count of one selected executable prefix. The ordinal
    /// addresses effective post-head argv. Reference-only, rejected and
    /// unclassified positions cannot borrow another operand's prefix purpose.
    #[must_use]
    pub fn command_prefix_arity(&self, argument_index: usize) -> Option<crate::AppendedArity> {
        self.command_prefix_arguments
            .as_ref()?
            .iter()
            .find_map(|&(index, arity)| (index == argument_index).then_some(arity))
    }

    /// Positional representation advice retained from this selected invocation.
    /// It establishes neither actual representation nor a conversion result.
    /// Unknown cardinality/options and rejected arity supply no hint.
    #[must_use]
    pub fn argument_type_hint(&self, index: usize) -> Option<&'static crate::hooks::ArgTypeHint> {
        if self.arity_accepts_frozen_arguments() != Some(true)
            || index >= self.frozen_argument_count?
        {
            return None;
        }
        self.argument_type_hints?.at(index)
    }

    /// Actual normal native numeric result shape, independently of operand
    /// contents, callback effects, completion guarantees or compiler selection.
    #[must_use]
    pub fn normal_numeric_result_production(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::native_result::NativeNumericResultProduction> {
        if self.successful_handler != Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return None;
        }
        if matches!(
            self.native_result,
            Some(crate::native_result::NativeResultContract::ScalarMath(_))
        ) {
            return self
                .normal_scalar_math_protocol(arguments)
                .map(crate::mathfunc::NativeScalarMathProtocol::result_production);
        }
        if self.operation != SemanticOperationId::Intrinsic(IntrinsicId::ListLength) {
            return None;
        }
        self.native_result?
            .normal_numeric_result_production(arguments, self.argument_offset)
    }

    /// Authored scalar math operand/result protocol from this selected native
    /// handler. Actual reached implementation and observers remain independent.
    #[must_use]
    pub fn normal_scalar_math_protocol(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::mathfunc::NativeScalarMathProtocol> {
        if self.operation != SemanticOperationId::Invoke
            || self.successful_handler
                != Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return None;
        }
        let crate::native_result::NativeResultContract::ScalarMath(operation) =
            self.native_result?
        else {
            return None;
        };
        crate::mathfunc::NativeScalarMathProtocol::for_invocation(
            operation,
            arguments,
            self.argument_offset,
        )
    }

    /// Effect-purpose refinement after proving (or failing to prove) the
    /// original scalar operand's string-access closure. This changes neither
    /// native result/completion nor compiler/implementation identity.
    pub fn refine_scalar_math_input_effects(
        &mut self,
        arguments: crate::InvocationArguments<'_>,
        input_is_closed: bool,
    ) {
        if self.normal_scalar_math_protocol(arguments).is_some() && !input_is_closed {
            self.effects
                .add_callback(crate::world_effect::CallbackEffect {
                    kinds: crate::world_effect::CallbackKinds::HOST,
                    reentrancy: crate::world_effect::Reentrancy::AnyInterpreter,
                });
            self.withdraw_purity();
        }
    }

    /// Selected normal-result provider after separate proof of the actual
    /// native handler. Operand effects and compiler admission stay independent.
    #[must_use]
    pub fn normal_list_method_provider(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::native_result::NativeListMethodProvider> {
        if self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return None;
        }
        self.native_result?
            .normal_list_method_provider(arguments, self.argument_offset)
    }

    /// Bound an authored list-length handler after independent proof that its
    /// original operand is an ordinary List/Dictionary object. This descriptor
    /// query requires actual handler identity separately and never proves OK.
    #[must_use]
    pub fn ordinary_list_length_completion(
        &self,
        arguments: crate::InvocationArguments<'_>,
        proved_argument: usize,
        representation: crate::representation::OrdinaryContainerRepresentation,
    ) -> Option<crate::completion_route::InvocationCompletionRoute> {
        if self.operation != SemanticOperationId::Intrinsic(IntrinsicId::ListLength)
            || self.successful_handler
                != Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return None;
        }
        self.representation_effect.ordinary_list_length_completion(
            arguments,
            self.argument_offset,
            proved_argument,
            representation,
        )
    }

    /// Select the native list-length object-method effect obligation. The
    /// caller must separately prove the current handler and original operand.
    /// Tcl 9's `lengthProc` returns a size, but arbitrary registered native
    /// methods can mutate the interpreter world; this is not a Tcl code bound.
    #[must_use]
    pub fn list_length_object_protocol(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<crate::representation::ListLengthObjectProtocol> {
        use crate::representation::ListLengthObjectProtocol as Protocol;
        use tcl_dialect::{
            TclVersion,
            model::{Family, Release},
        };
        if self.operation != SemanticOperationId::Intrinsic(IntrinsicId::ListLength)
            || self.successful_handler
                != Some(crate::native_compilation::SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
            || self.argument_offset.checked_add(1) != arguments.exact_argv_len()
        {
            return None;
        }
        let argument = self.argument_offset;
        let Some(dialect) = arguments.dialect() else {
            return Some(Protocol::Unknown);
        };
        Some(match (dialect.family(), dialect.tcl_version) {
            (Some(Family::Tcl), Some(version)) if version >= TclVersion::V9_0 => {
                Protocol::AbstractLength { argument }
            }
            (Some(Family::Tcl), Some(_)) => Protocol::Ordinary { argument },
            (Some(Family::Jim), _)
                if dialect
                    .core_point
                    .is_some_and(|point| point.release() == Release::JIM_0_84) =>
            {
                Protocol::Ordinary { argument }
            }
            _ => Protocol::Unknown,
        })
    }

    /// Signature validity of the argv which produced these owned facts.
    /// This does not validate option values or execution effects.
    #[must_use]
    pub fn arity_accepts_frozen_arguments(&self) -> Option<bool> {
        self.arity_argument_count
            .map(|count| self.arity.accepts(count))
    }

    /// Return the effect footprint ready for completion-aware world-state
    /// projection.
    ///
    /// This deliberately exposes no command spelling or legacy descriptor:
    /// [`Self::effects`] was filtered only through the registry-owned
    /// [`Self::transition_effect_coverage`] contract when these facts were
    /// materialised.
    #[must_use]
    pub const fn world_state_effects(&self) -> &EffectFootprint {
        &self.effects
    }

    /// Return the sole invocation argument carrying one of `roles` when the
    /// resolved command or subcommand accepts the supplied outer arity.
    ///
    /// `argument_count` and the returned index both count words after the
    /// command head, including a subcommand word when one was resolved. This
    /// keeps runtime consumers on the registry's argument-offset and
    /// resolver-first role contract instead of maintaining command-local
    /// operand tables. Multiple requested roles on the same argument still
    /// identify one operand; matching roles on different arguments are
    /// ambiguous. An incomplete dynamic role resolution, invalid arity, or
    /// missing or ambiguous role returns `None`.
    #[must_use]
    pub fn sole_argument_index_for_roles(
        &self,
        argument_count: usize,
        roles: &[ArgRole],
    ) -> Option<usize> {
        if self.frozen_argument_count != Some(argument_count)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || !self.arg_roles_complete
        {
            return None;
        }

        let mut sole = None;
        for &(index, role) in &self.arg_roles {
            if !roles.contains(&role) {
                continue;
            }
            let index = self.argument_offset + usize::from(index);
            if index >= argument_count {
                return None;
            }
            match sole {
                None => sole = Some(index),
                Some(previous) if previous == index => {}
                Some(_) => return None,
            }
        }
        sole
    }
}

impl ResolvedInvocation<'_, '_> {
    /// Possible receiver naming form from this selected authored source grammar.
    /// Complete authored roles and accepted source cardinality are required;
    /// unknown options, frame selectors and expansion retain their uncertainty.
    /// This does not validate native installation, access or successful execution.
    #[must_use]
    pub fn authored_source_variable_receiver_operand_form(
        &self,
        argument: usize,
    ) -> Option<VariableReceiverOperandForm> {
        // Implementation contract: naming.vendor.original-registry-metadata
        // docs/design/analysis/name-resolution-proofs/vendor-original-registry-metadata.md
        let (roles, complete) = self.authored_source_argument_roles();
        if !complete
            || self
                .argument_count_for_arity()
                .is_none_or(|count| !self.semantics.arity.accepts(count))
        {
            return None;
        }
        selected_variable_receiver_operand_form(
            argument,
            self.semantics.argument_offset,
            &roles,
            self.semantics.traits,
            self.semantics.variable_receivers,
            self.semantics.successful_handler,
            self.semantics.native_compilation,
        )
    }

    /// Materialise the target-neutral facts for an owned consumer such as an
    /// executable IR node.
    ///
    /// Borrowed resolution itself stays allocation-free.  This method is the
    /// explicit ownership boundary: it copies registry identities and
    /// resolves any lazy world-effect descriptors.
    #[must_use]
    pub fn facts(&self) -> InvocationFacts {
        self.facts_on_edge(false)
    }

    /// Materialise facts that hold after successful completion. Keep ordinary
    /// [`Self::facts`] for invocation, error and partial-commit paths.
    #[must_use]
    pub fn facts_after_success(&self) -> InvocationFacts {
        self.facts_on_edge(true)
    }

    fn normal_source_colour(&self) -> Option<crate::taint::TaintColour> {
        use crate::taint::{TaintColour, augment_source_colours};
        let argc = self.words.arguments().exact_argv_len();
        let source = self.semantics.taint_source;
        let traits = self.semantics.traits;
        let selected = source.is_some()
            || traits.intersects(Traits::TAINT_SOURCE | Traits::UNNORMALISED_HTTP_GETTER)
            || (traits.contains(Traits::TAINT_SOURCE_ZERO_ARGS)
                && argc == Some(self.semantics.argument_offset));
        if !selected {
            return None;
        }
        let colour = if argc == Some(0) {
            source.unwrap_or(TaintColour::TAINTED)
        } else {
            TaintColour::TAINTED
        };
        Some(augment_source_colours(colour | TaintColour::TAINTED))
    }

    /// Retain the reached representation-coercion phase independently of
    /// variable writes, result types and mathematical purity. The caller must
    /// still prove the selected native handler before consuming this contract.
    #[must_use]
    pub fn operand_representation_coercions(
        &self,
    ) -> crate::representation::RepresentationCoercionSelection {
        use crate::representation::RepresentationCoercionSelection as Coercions;
        let arguments = self.words.arguments();
        let effect = self
            .semantics
            .representation_effect
            .coercion_selection(arguments, self.semantics.argument_offset);
        if effect != Coercions::None {
            return effect;
        }
        if !self
            .semantics
            .arg_types
            .iter()
            .any(|(_, hint)| hint.shimmers)
        {
            return Coercions::None;
        }
        let Some(hints) = self.selected_argument_type_hints() else {
            return Coercions::Unknown;
        };
        let count = arguments
            .exact_argv_len()
            .expect("selected hints retain exact argc");
        let selected: Vec<_> = self
            .semantics
            .arg_types
            .iter()
            .filter(|(_, hint)| hint.shimmers)
            .filter_map(|(index, _)| hints.first_positional.checked_add(usize::from(*index)))
            .filter(|index| *index < count)
            .collect();
        if selected.is_empty() {
            Coercions::None
        } else {
            Coercions::Operands {
                arguments: selected,
            }
        }
    }

    fn selected_argument_type_hints(&self) -> Option<SelectedArgumentTypeHints> {
        if self.semantics.arg_types.is_empty()
            || matches!(
                self.subcommand,
                SubcommandResolution::Unknown { .. }
                    | SubcommandResolution::Ambiguous { .. }
                    | SubcommandResolution::Indeterminate { .. }
            )
        {
            return None;
        }
        let arguments = self.words.arguments();
        arguments.exact_argv_len()?;
        let offset = self.semantics.argument_offset;
        let skipped = self
            .semantics
            .options
            .leading_word_count(arguments.slice_from(offset))?;
        Some(SelectedArgumentTypeHints {
            table: self.semantics.arg_types,
            first_positional: offset.checked_add(skipped)?,
        })
    }

    /// The taint colour of this invocation's selected transform, if declared.
    fn selected_taint_transform(&self) -> Option<crate::taint::TaintColour> {
        self.semantics.taint_transform.and_then(|transform| {
            transform.resolve(
                self.words.arguments(),
                self.semantics.argument_offset,
                self.semantics.options,
            )
        })
    }

    /// The procedure definition this invocation's words select, if declared.
    fn selected_procedure_definition(
        &self,
    ) -> Option<crate::native_procedure::NativeProcedureDefinitionSelection> {
        self.semantics
            .procedure_definition
            .map(|descriptor| descriptor.select(self.words.arguments()))
    }

    fn facts_on_edge(&self, successful: bool) -> InvocationFacts {
        let (transitions, transition_effect_coverage) = if successful {
            self.semantics
                .state_transitions
                .resolve_after_success_with_effect_coverage(self.words.arguments())
        } else {
            self.semantics
                .state_transitions
                .resolve_with_effect_coverage(self.words.arguments())
        };
        let (arg_roles, arg_roles_complete) = self.argument_roles_on_edge(successful);
        let mut facts = InvocationFacts {
            source_callback_retention: self.selected_source_callback_retention(),
            deferred_script_arguments: self.semantics.script_metadata.deferred_arguments(
                self,
                &arg_roles,
                arg_roles_complete,
            ),
            script_lookup_arguments: self.semantics.script_metadata.lookup_arguments(
                self,
                &arg_roles,
                arg_roles_complete,
            ),
            command_prefix_arguments: self.semantics.script_metadata.prefix_arguments(
                self,
                &arg_roles,
                arg_roles_complete,
            ),
            argument_type_hints: self.selected_argument_type_hints(),
            named_object_factory: self.semantics.named_object_factory,
            canonical_command: self.canonical_command.to_owned(),
            subcommand: self.subcommand.into_owned(),
            form: self.form.map(|form| form.name.to_owned()),
            operation: self.semantics.operation,
            lowering_hook: self.semantics.lowering_hook,
            analyser_hook: self.semantics.analyser_hook,
            data_collection: self.authored_source_descriptors().command.data_collection,
            side_switch_target: self
                .authored_source_descriptors()
                .command
                .side_switch_target,
            option_keywords: self.semantics.options.keyword_table(),
            irules_source_flow_action: self.authored_source_irules_flow_action(),
            source_may_commit_response: self.authored_source_may_commit_response(),
            completion: self.semantics.completion.or_conservative(),
            result_stability: self.semantics.result_stability.stated().unwrap_or_default(),
            native_result: self.semantics.native_result,
            representation_effect: self.semantics.representation_effect,
            operand_representation_coercions: self.operand_representation_coercions(),
            effects: self.effect_footprint_with_transition_coverage(
                transitions.touches_command_bindings(),
                &transition_effect_coverage,
            ),
            state_transitions: if self.semantics.state_transitions.is_declared() {
                StateTransitionKnowledge::Declared(transitions)
            } else {
                StateTransitionKnowledge::UnknownInvocation
            },
            transition_effect_coverage,
            dispatch_dependencies: self.semantics.dispatch_dependencies.resolve(),
            traits: self.semantics.traits,
            taint_source: self.normal_source_colour(),
            taint_transform: self.selected_taint_transform(),
            mutator: self.semantics.mutator,
            arity: self.selected_arity(),
            arity_argument_count: self.argument_count_for_arity(),
            frozen_argument_count: self.words.arguments().exact_argv_len(),
            argument_offset: self.semantics.argument_offset,
            arg_roles,
            repeated_args: self.semantics.repeated_args.to_vec(),
            arg_roles_complete,
            arg_role_resolver_roles: self.semantics.arg_role_resolver_roles,
            return_type: self.semantics.return_type,
            procedure_definition: self.selected_procedure_definition(),
            native_compilation: self.semantics.native_compilation,
            successful_handler: self.semantics.successful_handler,
            variable_receivers: self.semantics.variable_receivers,
            byte_array_effect: self.semantics.byte_array_effect,
            byte_array_payload: self.semantics.byte_array_payload,
            var_write_typing: self.semantics.var_write_typing,
            return_elements: self.semantics.return_elements,
            var_elements_effect: self.semantics.var_elements_effect,
            body_kind: self.semantics.body_kind,
            body_execution: self.semantics.body_execution,
            body_interpreter: self.semantics.body_interpreter,
            frame_effect: self.semantics.frame_effect,
        };
        if successful
            && let Some(effects) = facts.successful_value_leaf_world(self.words.arguments())
        {
            facts.effects = effects;
            facts.state_transitions =
                StateTransitionKnowledge::Declared(StateTransitions::default());
        }
        facts
    }
}
