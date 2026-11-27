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

//! Shared compiler adapter from semantic source words to registry facts.
//!
//! The registry must only receive a Tcl word's evaluated value when the
//! semantic IR proves it literal. This module centralises that conversion for
//! every common compiler pass: executable IR, memory SSA, and later shared
//! analyses all resolve exactly the same structured [`WordExpr`] facts under
//! an explicitly supplied [`SemanticContext`] — the resolved environment the
//! document is assisted under (redesign §11.2 D1).

pub(crate) mod host_structure_observer;
#[doc(hidden)]
pub use host_structure_observer::{HostStructureTraceGuard, begin_host_structure_trace};

mod original_procedure_call;
#[cfg(test)]
pub(crate) use original_procedure_call::OriginalSourceProcedureCall;
#[doc(hidden)]
pub use original_procedure_call::{
    OriginalProcedureMayDependencies, original_authored_body_procedure_calls_for_module,
    original_authored_procedure_calls_for_module, original_logical_procedure_calls_for_module,
    original_logical_procedure_may_dependencies_for_module, original_procedure_formal_count_shape,
    original_procedure_may_dependencies_for_word_for_module, original_procedure_parameters,
    original_procedure_scalar_bindings, original_source_procedure_calls_for_word_for_module,
    original_source_procedure_ids_for_word_for_module,
};
mod authored_body_scope;
pub(crate) use authored_body_scope::{
    OriginalAuthoredFormalAliasObservation, OriginalAuthoredSourceBodyScope,
    original_authored_parameter_source_command_for_module,
};
pub(crate) use source_scoped_body::original_written_script_body_span;
mod authored_source_bodies;
pub(crate) use authored_source_bodies::original_authored_procedure_body;
pub use authored_source_bodies::{AuthoredProcedureFormalTopology, OriginalAuthoredProcedureBody};
mod source_definition;
pub(crate) use source_definition::{
    original_logical_source_definition_layout,
    original_source_definition_layout_from_selected_words,
};
mod source_receiver_names;
pub(crate) use source_receiver_names::{
    SourceReceiverNameOperand, original_source_receiver_name_operand,
};
mod original_word_data;
#[doc(hidden)]
pub use original_word_data::{
    OriginalSourceArgument, original_argument_command_effect, original_command_words_for_module,
    original_literal_argument_data, original_literal_argument_data_for_module,
    original_lowered_expression_syntax, original_source_argument_for_module,
    original_source_argument_for_script,
};
mod source_reference_candidates;
pub(crate) use source_reference_candidates::original_body_reference_candidates;
mod source_operand_evaluation;
pub(crate) use source_operand_evaluation::{
    OriginalOperandEvaluationUpdate, original_operand_evaluation_update,
};
mod source_name_roles;
pub(crate) use source_name_roles::{
    OriginalVariableNameOperand, SourceNameRoleInventory, original_assignment_name_operand,
    original_authored_substitution_name_roles, original_name_role_values,
    original_source_metadata_name_roles, original_source_name_roles,
    standalone_written_source_name_roles,
};
mod authored_source_structure;
pub(crate) use authored_source_structure::{
    AuthoredSourceTaintInvocation, AuthoredSourceVariableAccess,
    original_authored_script_scalar_variable, original_authored_source_structure,
};
mod execution_trace;
mod metadata_context;
pub(crate) use execution_trace::{
    original_execution_trace_keys, original_invocation_has_execution_trace,
    original_procedure_has_execution_trace, source_fold_has_execution_trace,
    source_procedure_fold_has_execution_trace,
};
pub(crate) use metadata_context::InvocationMetadataSelection;
pub(crate) use metadata_context::retained_module_metadata_context;
pub use metadata_context::{
    InvocationMetadataContext, InvocationMetadataInput, OwnedInvocationMetadataContext,
};
mod symbol_advice;
pub(crate) use symbol_advice::{
    OriginalSymbolDeclarationAdvice, original_symbol_declaration_advice,
    vendor_symbol_declaration_advice,
};
mod conditional_vendor_metadata;
pub(crate) use conditional_vendor_metadata::original_vendor_occurrence_registry_metadata;
pub use conditional_vendor_metadata::{
    OriginalConditionalVendorRegistryMetadata, original_conditional_vendor_registry_metadata,
};
mod conditional_metadata;
pub use conditional_metadata::{
    OriginalConditionalRegistryMetadata, original_conditional_registry_metadata,
};
/// Original Logical formal binding identity and bounded alpha source coverage.
pub mod logical_formals;
mod source_availability;
pub use source_availability::{
    CommandSourceAvailabilityDomain, OriginalSourceCommandAvailability,
    original_source_command_availability,
};
mod source_publication;
pub use source_publication::{
    OriginalSourceCommandPublication, OriginalSourceHandleClassAdvice,
    OriginalSourceHandleConstruction,
};
mod source_scoped_body;
pub(crate) use source_scoped_body::OriginalSourceDefinitionMemberBodyCorrespondence;
/// Sealed readonly original argv and Registry source-role structure.
pub mod source_structure;
pub use source_scoped_body::{
    OpaqueScriptAuthoringRegion, OriginalDeclaredSourceScriptBody,
    OriginalLogicalScopedEnvironment, OriginalSourceDefinitionMemberDisposition,
    OriginalSourceDefinitionMemberReference, OriginalSourceDefinitionMemberRegion,
    OriginalSourceDefinitionMemberScriptBody, OriginalSourceScopedBody, OriginalSourceScriptBody,
    OriginalSourceScriptPurpose,
};
pub(crate) use source_scoped_body::{
    OriginalSourceScriptBodyOrigin, declared_source_script_bodies_for,
    original_source_scoped_bodies, original_written_script_body_is_empty,
};
pub(crate) mod source_schema_queries;
mod source_taint;
mod source_taint_arguments;
pub use source_taint::HostedSourceTaintContext;
pub(crate) use source_taint::{
    NativeSourceTaintInvocation, OriginalSourceReadSites, OriginalSourceTaintReadOperands,
    actual_taint_script_arguments, hosted_source_taint_invocation, native_source_taint_invocation,
    original_compiled_list_head, original_conditional_list_result_head,
    original_expression_taint_read_operands, original_hosted_source_registry_metadata,
    original_source_taint_read_operands,
};
mod vendor_advice;
mod vendor_source_barriers;
pub use vendor_advice::{VendorRegistryInvocationShape, vendor_registry_invocation_shape};
pub use vendor_source_barriers::{NominalAliasSourceOperandOrigin, VendorSourceCatalogueBarriers};

mod source_remote_first_operand;
pub use source_remote_first_operand::{
    OriginalSourceRemoteFirstOperandAssociation, OriginalSourceRemoteResultAssociation,
    RemoteResultSourceObligation, source_remote_first_operand_associations,
};
mod hosted_structure;
pub(crate) use hosted_structure::{
    SourceStructuredInvocation, original_hosted_scalar_store_facts,
    original_hosted_structured_invocation, original_logical_source_structured_invocation,
    original_native_source_structured_invocation,
};
mod logical_frame;
pub(crate) use logical_frame::{
    OriginalLogicalFrameInvocation, original_logical_body_geometry_for_module,
    original_logical_frame_invocation_for_module,
    original_logical_frame_invocation_with_metadata_context,
};
mod variable_write_advice;
pub(crate) use variable_write_advice::{
    logical_source_variable_write_advice, original_variable_write_advice,
    vendor_variable_write_advice,
};

mod declaration_assistance;
pub use declaration_assistance::{
    DeclarationArgument, DeclarationVariableAliasPurpose, OriginalDeclarationAssistance,
};
pub(crate) use declaration_assistance::{
    OriginalLogicalAliasTemplateSource, OriginalSourceVariableAliasOperands,
    original_declaration_assistance,
};

mod declaration_flow;
pub(crate) use declaration_flow::{
    DeclarationInvocationFlow, DeclarationLifecycleInvocation, DeclarationSchemaOrigin,
    declaration_dictionary_scope_advice, declaration_invocation_flow,
    declaration_invocation_flow_in_frame, declaration_lifecycle_invocation,
    declaration_original_words_flow, declaration_source_flow_corresponds,
    declaration_source_read_flow, declared_argument_words,
    source_advice_has_no_declared_lookup_mutation,
};

mod interval_advice;
pub(crate) use interval_advice::declaration_increment_advice;

mod native_compilation_source;
pub use native_compilation_source::original_native_compiler_words;
pub(crate) use native_compilation_source::{
    OriginalNativeCompilerInvocation, OriginalNativeCompilerPreparation,
    OriginalSourceCommandProjection, original_native_compilation,
};

mod store_advice;
pub(crate) use store_advice::{
    conditional_declared_overwrite_advice, conditional_overwritten_local_store_advice,
    conditional_unread_local_store_advice, overwritten_local_store_advice,
};

use tcl_dialect::EscapeSyntax;
#[cfg(test)]
use tcl_registry::InvocationWordKind;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{CommandRegistry, InvocationFacts, InvocationWord, InvocationWords};

use crate::ir::CommandTokens;
use tcl_source::words::WordExpr;
use tcl_source::words::WordPart;
use tcl_syntax::word_rules::WordValueRules;

mod body_assistance;
mod body_candidates;
mod body_topology;
mod effective_words;
mod expression_operand_advice;
mod handler_invocation;
mod invocation_assistance;
mod logical_parameter_bodies;
mod native_compilation;
mod normal_representation;
mod normal_transfer;
mod source_coverage;
mod source_model_operations;
mod structured_invocation;
mod variable_transitions;
mod word_resolution;

#[cfg(test)]
use body_candidates::body_assistance_context;
#[cfg(test)]
use body_candidates::original_body_parameter_source_invocation;
pub(crate) use body_candidates::{invocation_body_assistance, segmented_body_assistance};
pub use body_topology::{PossibleBodyInvocation, possible_body_invocation};
pub(crate) use body_topology::{
    conditional_body_topology_advice_with_metadata_context,
    possible_body_invocation_with_metadata_context,
};
use effective_words::{
    compose_effective_words, frozen_argument_values, original_logical_prefix_for_tokens,
};
pub(crate) use effective_words::{compose_original_effective_words, effective_words_for_target};
pub use effective_words::{effective_command_words, frozen_argument_words};
#[cfg(test)]
pub(crate) use expression_operand_advice::OriginalExpressionOperandAdvice;
pub(crate) use expression_operand_advice::{
    original_expression_operand_advice, original_expression_operand_advice_for_word,
    original_literal_expression_context_advice,
};
pub use handler_invocation::{
    DeclaredInvocationAssistance, NormalUserProcedureInvocation, normal_handler_completion_route,
    normal_user_procedure_invocation, resolved_declared_assistance,
};
pub(crate) use handler_invocation::{
    normal_user_procedure_invocation_with_metadata_context, original_callback_invocation,
    original_callback_invocation_with_metadata_context,
    original_selected_handler_layout_invocation,
    possible_normal_handler_effect_regions_with_metadata_context, resolved_handler_invocation,
    resolved_handler_invocation_with_metadata_context,
};
use invocation_assistance::mapped_written_argument_roles;
pub use invocation_assistance::{
    AdvisoryCommandWords, AdvisoryValueAssignment, CatalogueInvocationAssistance,
    RegistryInvocationAssistance, advisory_return_values, advisory_return_values_in_context,
    advisory_value_assignments, advisory_value_assignments_in_context,
    catalogue_invocation_assistance, invocation_argument_role_assistance,
    invocation_argument_role_consensus, original_incomplete_body_registry_invocation_assistance,
    original_registry_invocation_assistance,
    original_registry_invocation_assistance_with_metadata_context, registry_invocation_assistance,
    registry_invocation_assistance_in_context,
};
pub(crate) use invocation_assistance::{
    AdvisoryObjectFactory, advisory_object_factories_with_metadata_context,
    advisory_return_values_with_metadata_context, advisory_value_assignments_with_metadata_context,
    original_advisory_value_assignments_with_metadata_context,
    registry_invocation_assistance_with_metadata_context,
};
pub(crate) use logical_parameter_bodies::{
    logical_lambda_parameter_body_assistance, logical_parameter_body_assistance,
    unpositioned_body_assistance,
};
pub(crate) use native_compilation::SelectedLoweringBindingIdentity;
pub use native_compilation::{
    NativeCompilationDependency, NativeOperationSelectionDecline, NativeOperationSelectionPlan,
    NativeProcedureNoOpPlan, admitted_native_compiler_invocation, block_requires_runtime_script,
    command_binding_guard, compiled_namespace_context, native_command_binding_guard,
    native_compilation_dependency, native_compilation_syntax, native_compilation_word_shape,
    native_compiler_dependency, native_compiler_replay_source, native_implementation_dependencies,
    native_implementation_dependency, native_named_command_words, native_operation_selection_plan,
    native_procedure_noop_plan, native_site_binding_requirement, original_block_runtime_invocation,
    proved_native_admitted_inline_operation, proved_native_inline_body_context,
    proved_native_inline_operation, static_command_word,
};
#[cfg(test)]
pub(crate) use normal_representation::ConditionalIndexAccessAdvice;
use normal_representation::NormalRepresentationProof;
pub use normal_representation::{
    NormalIndexAccess, NormalIndexAccessKind, NormalTaintInvocation, NormalValueAssignment,
    normal_representation_invocation, normal_representation_invocation_in_context,
    normal_representation_invocation_with_metadata_context, normal_statement_representation,
    normal_taint_invocation,
};
pub(crate) use normal_representation::{
    conditional_index_access_advice_with_metadata_context,
    normal_statement_representation_with_metadata_context,
    normal_taint_invocation_with_metadata_context,
};
pub use normal_transfer::{
    NormalContainerElementWrite, NormalTransferInvocation, normal_transfer_invocation,
    normal_transfer_invocation_in_context, normal_transfer_invocation_with_metadata_context,
};
#[cfg(test)]
use source_coverage::source_cut_resolved_head_correspondence;
pub use source_coverage::{
    OriginalQuietScalarStoreSourceCoverage,
    original_authored_procedure_declaration_for_source_coverage,
    original_authored_quiet_store_for_source_coverage,
    original_authored_result_completion_for_source_coverage,
    original_conditional_source_load_for_coverage,
    original_source_cut_procedure_declaration_for_coverage,
};
use source_coverage::{SourceCutStoredHead, registered_result_completion_schema_is_supported};
pub(crate) use source_coverage::{
    load_trace_flag, load_trace_option, source_cut_declaration_origins, source_cut_refuses,
    source_cut_stage, source_cut_trace_admitted,
};
pub(crate) use source_model_operations::{
    conditional_option_arguments_with_metadata_context,
    original_logical_variable_footprint_with_metadata_context,
    original_native_source_conditional_completion,
    original_substitution_purity_with_metadata_context,
};
pub use source_model_operations::{
    original_logical_operation_invocation_with_metadata_context,
    original_source_model_operation_invocation_with_metadata_context,
};
use source_model_operations::{resolve_original_declared_layout, resolve_original_source_layout};
#[cfg(any(test, debug_assertions))]
use structured_invocation::StructuredSourceRoles;
#[cfg(test)]
use structured_invocation::original_declared_structured_invocation;
pub(crate) use structured_invocation::{
    LogicalStructuredInvocation, logical_structured_invocation,
    logical_structured_invocation_with_metadata_context,
};
pub use structured_invocation::{
    original_structured_invocation, original_structured_invocation_with_metadata_context,
};
use structured_invocation::{
    original_structured_operands_are_supported, static_body_evaluator_is_supported,
};
pub use variable_transitions::{
    NamespaceDirectiveFootprint, PossibleVariableNameOperands, PossibleVariableTraceTransitions,
    namespace_directive_footprint, possible_variable_alias_transitions,
    possible_variable_alias_transitions_in_context, possible_variable_name_operands,
    possible_variable_name_operands_in_context, possible_variable_trace_transitions,
    possible_variable_trace_transitions_in_context, variable_name_operands,
};
pub(crate) use variable_transitions::{
    namespace_directive_footprint_with_metadata_context,
    possible_variable_alias_transitions_with_metadata_context,
    possible_variable_name_operands_with_metadata_context,
    possible_variable_trace_transitions_with_metadata_context,
};
pub use word_resolution::{
    CompiledLocalNameWord, EffectiveInvocationWord, OwnedInvocationResolutionUnresolved,
    RegistryInvocationDecline, RegistryInvocationResolution, compiled_local_name_value,
    compiled_local_name_word, effective_command_arguments, effective_invocation_word,
    invocation_word, invocation_word_with_source, resolve_command_tokens,
    resolve_command_tokens_with_metadata_context, resolve_word_exprs,
    resolve_word_exprs_with_dialect, segmented_command_arguments,
};
pub(crate) use word_resolution::{
    original_array_element_word, resolve_registry_words_in_realm,
    resolve_registry_words_in_realm_with_metadata_context,
};
use word_resolution::{
    resolve_command_tokens_with_metadata_context_and_case,
    resolve_effective_tokens_with_metadata_context,
    resolve_registry_words_in_realm_with_metadata_context_and_case,
};

/// Origin of a word in the post-binding argument vector. Inserted alias
/// arguments have values but have no editable range in the invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationWordOrigin {
    /// The canonical implementation head, derived from dispatch proof.
    ResolvedHead,
    /// An argument retained by an interpreter alias.
    BindingPrefix(usize),
    /// A word in the original invocation, including its head at index zero.
    Written(usize),
    /// One native list element of an original expanded word; no editable range.
    ExpandedElement {
        /// Original word index, including the head.
        written: usize,
        /// Element index in the retained native list.
        element: usize,
    },
}

/// One shared projection of proved dispatch and alias argument composition.
/// Document procedures retain identity here; registry selection separately
/// requires registry-backed implementation provenance.
/// The original token snapshot remains the runtime evaluation sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveCommandWords {
    /// Effective command head followed by effective arguments.
    pub words: Vec<WordExpr>,
    /// One origin for every effective word.
    pub origins: Vec<InvocationWordOrigin>,
    /// Frozen alias values, including unknown values with known argv cardinality.
    pub binding_prefix: Vec<EffectiveInvocationWord>,
}

/// Scope a structured projection of original contiguous written argument words.
/// Missing fragments, recovery, or changed cardinality leave no layout proof.
/// This lexical view supplies no handler identity or frozen evaluated value.
pub(crate) fn with_source_argument_words<T>(
    source: &str,
    arg_tokens: &[tcl_lexer::Token],
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
    apply: impl FnOnce(tcl_registry::InvocationArguments<'_>) -> T,
) -> Option<T> {
    let first = tcl_lexer::word_span_at(source, arg_tokens.first()?.span);
    let last = tcl_lexer::word_span_at(source, arg_tokens.last()?.span);
    let text = source.get(first.start() as usize..last.end() as usize)?;
    let mut segments = tcl_source::segment::segment_commands_with_offset_and_config(
        text,
        first.start(),
        config.nested(),
    )
    .into_iter();
    let segment = segments.next()?;
    if segments.next().is_some() || segment.argv.len() != arg_tokens.len() || segment.is_partial {
        return None;
    }
    let tokens = crate::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(source),
        config,
        &segment,
    );
    let values: Vec<_> = tokens
        .word_exprs
        .iter()
        .map(|word| {
            crate::registry_invocation::effective_invocation_word(
                word,
                config.escapes,
                dialect.word_values,
            )
        })
        .collect();
    let words: Vec<_> = values
        .iter()
        .zip(&tokens.word_exprs)
        .map(|(effective, source)| {
            crate::registry_invocation::invocation_word_with_source(
                source,
                effective,
                config.escapes,
            )
        })
        .collect();
    Some(apply(
        tcl_registry::InvocationArguments::structured(&words).with_dialect(dialect),
    ))
}

/// Case-clause layout selected from one actual registry invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoredSourceCaseBodySelection {
    /// The actual selected command descriptor has no case-list grammar.
    NotCase,
    /// A case-list descriptor was selected, but its authored layout is unknown.
    Unavailable,
    /// Layout projected by the actual selected descriptor and original argv.
    Selected(tcl_registry::case_bodies::CaseBodyOperands),
}

/// Registry facts and arguments selected from one retained source-state proof.
/// Consumers use this projection together so role indices cannot drift from
/// alias composition or the original evaluation sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStatementInvocation {
    /// Interpreter policies retained independently of catalogue availability.
    pub dialect: Option<tcl_registry::InvocationDialect>,
    /// Live implementation's registry facts.
    pub facts: Box<InvocationFacts>,
    /// Effective post-head presentation slots. Written words retain their
    /// authored spellings; captured values without retained bytes remain None.
    /// Semantic consumers use frozen argument values and shapes instead.
    pub arguments: Vec<Option<String>>,
    /// Effective values and their written or inserted origins.
    pub effective: EffectiveCommandWords,
    /// Effective argument values frozen by the source owner at evaluation time.
    /// This never replaces the original words or their editable provenance.
    pub evaluated_arguments: Vec<Option<String>>,
    /// Frozen argv shape, including bounded array roots with unknown indices.
    pub evaluated_words: Vec<EffectiveInvocationWord>,
    /// Case-list layout retained by the same actual registry selection.
    pub authored_case_body_selection: Option<AuthoredSourceCaseBodySelection>,
}

#[cfg(debug_assertions)]
pub(crate) fn trace_jim_body_dispatch(
    site: u32,
    logical: Option<&LogicalStructuredInvocation>,
    bodies: &[usize],
    arguments: &[tcl_lexer::Token],
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_JIM_SOURCE_OWNER").is_some() {
        let arguments = arguments
            .iter()
            .map(|token| (token.kind, token.span))
            .collect::<Vec<_>>();
        eprintln!(
            "JIM_SOURCE_OWNER stage=jim.body.dispatch_result offset={site} logical={} bodies={bodies:?} arguments={arguments:?}",
            logical.is_some(),
        );
        if let Some(shape) = logical {
            trace_jim_structure_fields("dispatch", &shape.invocation, shape.source_roles.as_ref());
        }
    }
}

#[cfg(debug_assertions)]
fn trace_jim_structure_fields(
    stage: &str,
    selected: &ResolvedStatementInvocation,
    roles: Option<&StructuredSourceRoles>,
) {
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_JIM_SOURCE_OWNER").is_some() {
        let logical_roles = match roles {
            Some(StructuredSourceRoles::Logical(roles)) => Some(roles),
            _ => None,
        };
        eprintln!(
            "JIM_SOURCE_OWNER stage=jim.body.structure_fields point={stage} operation={:?} hook={:?} kind={:?} interpreter={:?} frame_effect={} body_execution={} traits={:?} logical_roles={logical_roles:?}",
            selected.facts.operation,
            selected.facts.lowering_hook,
            selected.facts.body_kind,
            selected.facts.body_interpreter,
            selected.facts.frame_effect.is_some(),
            selected.facts.body_execution.is_some(),
            selected.facts.traits,
        );
    }
}

/// Same-owner coverage of one known source-load transition. The completed
/// source survey remains conditional; physical leases belong to the host.
pub struct OriginalConditionalSourceLoadCoverage<'a> {
    selected: ResolvedStatementInvocation,
    analysis: &'a crate::analyser::AnalysisResult,
    outcome: &'a crate::command_binding::OriginalSourceConditionalOutcome,
    load: source_structure::OriginalProjectSourceLoad,
}

/// Exact conditional result-completion source coverage, without execution,
/// Native admission, completed effects or Normal operation completion.
/// Every executable argument component remains an independent source survey.
pub struct OriginalAuthoredResultCompletionSourceCoverage<'a> {
    selected: ResolvedStatementInvocation,
    analysis: &'a crate::analyser::AnalysisResult,
    regions: Vec<tcl_lexer::Span>,
}

mod source_cut_body_layout;
pub use source_cut_body_layout::{
    OriginalSourceCutBodyLayout, OriginalSourceCutBodyReference,
    OriginalSourceCutBodyResultCoverage,
};

/// Deferred declaration geometry under one exact completed source cut.
/// The original child or suffix owns its Registry source schema; this grants
/// no inherited invocation purpose, body entry, Native frame or Normal result.
pub struct OriginalSourceCutProcedureDeclaration<'a> {
    analysis: &'a crate::analyser::AnalysisResult,
    selected: ResolvedStatementInvocation,
    body: OriginalSourceScriptBody,
    head_correspondence: SourceCutStoredHead,
    header: CommandTokens,
    command: tcl_source::segment::SegmentedCommand,
}

/// A value-only declaration-word join, never source command selection.
/// The existing opaque head pair retains its original purposes; command and
/// body selection still come only from the independently selected right side.
struct SourceCutDeclarationWordCorrespondence {
    heads: source_structure::OriginalSourceHeadCorrespondence,
}

/// Resolve a retained invocation through the shared point owner. Synthetic
/// analysis boundaries contribute their typed effects rather than invoking a
/// command, and explicitly unproved dispatch never uses a written-head query.
#[must_use]
pub fn resolved_statement_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    statement: &crate::ir::Statement,
) -> Option<ResolvedStatementInvocation> {
    resolved_statement_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        statement,
    )
}

pub(crate) fn resolved_statement_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    statement: &crate::ir::Statement,
) -> Option<ResolvedStatementInvocation> {
    resolved_tokens_invocation_with_metadata_context(registry, context, statement.tokens()?)
}

/// Resolve an original statement under the retained availability generation.
/// Synthetic boundaries and statements without genuine invocation tokens do
/// not acquire a command operation through this metadata context.
#[must_use]
pub fn resolved_statement_invocation_in_context(
    context: &tcl_registry::model::ContextRegistry,
    statement: &crate::ir::Statement,
) -> Option<ResolvedStatementInvocation> {
    resolved_tokens_invocation_in_context(context, statement.tokens()?)
}

/// Resolve a complete retained invocation, including a nested substitution,
/// without reconstructing a statement or changing its source provenance.
#[must_use]
pub fn resolved_tokens_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    resolved_tokens_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Resolve the retained invocation under the supplied availability generation.
/// Command identity, native implementation and argument completion remain
/// independently required; this context supplies none of their missing proof.
#[must_use]
pub fn resolved_tokens_invocation_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    resolved_tokens_invocation_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

/// Select retained invocation facts under the actual supplied metadata owner.
/// Independent command, native handler and argument receipts remain required.
#[must_use]
pub fn resolved_tokens_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let (resolution, authored_case_body_selection) =
        resolve_command_tokens_with_metadata_context_and_case(registry, context, tokens).ok()?;
    let RegistryInvocationResolution::Resolved(facts) = resolution else {
        return None;
    };
    let effective = effective_command_words(tokens)?;
    let arguments = effective.argument_presentations(tokens.argv_texts.get(1..)?)?;
    let dialect = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .or_else(|| {
            context
                .and_then(|context| context.context().environment.point())
                .map(tcl_registry::InvocationDialect::of_point)
        })
        .or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        });
    Some(ResolvedStatementInvocation {
        dialect,
        facts,
        arguments,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
        authored_case_body_selection: Some(authored_case_body_selection),
    })
}

/// Compiler metadata of an admitted recipe; no normal effects are exposed.
#[derive(Debug)]
pub struct AdmittedNativeCompilerInvocation {
    invocation: ResolvedStatementInvocation,
    compiler_dialect: tcl_registry::InvocationDialect,
    hooks: tcl_registry::registry::NativeCompilerHooks,
    target: crate::command_binding::SourceCommandTarget,
    selection: tcl_registry::native_compilation::NativeCompilationSelection,
    hook_operand_prefix: &'static [&'static str],
}

/// Possible alias declarations of retained handler candidates. This footprint
/// records exposure hazards, never a successful declaration or physical store.
#[derive(Debug)]
pub struct PossibleVariableAliasTransitions {
    aliases: Vec<tcl_registry::state_transition::VariableCellAliasTransition>,
    unknown_residual: bool,
}

/// Possible variable-name operands of an exactly retained handler.
///
/// This footprint cannot provide variable definitions, normal store values,
/// body/control semantics or compiler eligibility. A dynamic destination retains
/// a possible name access even when successful-handler transfer cannot prove
/// its physical target. Compiler rejection may make the access unreachable;
/// that cannot establish the absence of a possible name access.
#[derive(Debug)]
pub struct VariableNameOperands {
    binding_phase: tcl_registry::native_compilation::VariableOperandBindingPhase,
    operands: Vec<(tcl_registry::ArgRole, EffectiveInvocationWord)>,
    unresolved_roles: &'static [tcl_registry::ArgRole],
    binding_only_names: std::collections::HashSet<String>,
    destroys: bool,
}

/// Representation facts on the reached handler's normal continuation.
///
/// This deliberately cannot expose native compiler, world-effect, body-flow or
/// completion facts. A converged handler may provide representation effects
/// while its compiler hook remains unknown; those effects never describe an
/// exceptional continuation. Effective words retain the original read sites.
#[derive(Debug)]
pub struct NormalRepresentationInvocation {
    proof: NormalRepresentationProof,
    native_pattern_source_argument: Option<usize>,
    pattern_operand_alternatives: Vec<(usize, Vec<String>)>,
}

/// Original declaration-owned lookup and accepted operand layout for bounds
/// diagnostics. Every native candidate must agree. Runtime residuals remain
/// and consumers need independent source-read and interval evidence.
#[cfg(test)]
pub(crate) fn conditional_index_access_advice(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<ConditionalIndexAccessAdvice> {
    conditional_index_access_advice_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

impl ResolvedStatementInvocation {
    /// Map selected argument roles to original written operands. This keeps
    /// the selected invocation's proof strength; captured prefix values and
    /// materialized expansion elements have no editable source operand.
    #[must_use]
    pub fn written_argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        mapped_written_argument_roles(
            &self.effective,
            self.facts.argument_offset,
            &self.facts.arg_roles,
        )
    }

    /// Query registry metadata with the canonical selected head and evaluated
    /// argv knowledge. Captured values keep their slots without invented bytes;
    /// the retained native dialect travels with the same argument view.
    pub fn with_argument_words<T>(
        &self,
        apply: impl FnOnce(tcl_registry::InvocationWords<'_>) -> T,
    ) -> T {
        let values: Vec<_> = (0..self.arguments.len())
            .map(|index| self.argument_word(index))
            .collect();
        let arguments: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let mut words = tcl_registry::InvocationWords::structured(
            tcl_registry::InvocationWord::Literal(&self.facts.canonical_command),
            &arguments,
        );
        if let Some(dialect) = self.dialect {
            words = words.with_dialect(dialect);
        }
        apply(words)
    }

    /// Effective evaluated argument knowledge, including bounded variable-name shape.
    /// A frozen complete value takes precedence over its original structural shape.
    #[must_use]
    pub fn argument_word(&self, argument: usize) -> EffectiveInvocationWord {
        if let Some(word) = self.evaluated_words.get(argument) {
            return word.clone();
        }
        if let Some(value) = self.argument_literal(argument) {
            return EffectiveInvocationWord::Literal(value);
        }
        match (
            self.dialect,
            argument
                .checked_add(1)
                .and_then(|index| self.effective.words.get(index)),
        ) {
            (Some(dialect), Some(word)) => {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            }
            _ => EffectiveInvocationWord::Dynamic,
        }
    }

    /// Query retained selected argv under the caller's whole metadata context.
    /// The context selects availability only; it grants no new handler proof.
    pub fn with_metadata_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        realm: tcl_dialect::model::InvocationRealm,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if !context.matches_registry(registry) {
            return None;
        }
        self.with_argument_words(|words| {
            let resolution =
                tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                    registry,
                    Some(context.context()),
                    words,
                    realm,
                );
            let selected = resolution.resolved();
            if selected.is_none()
                && tcl_registry::trace_deferred_writes_any()
            {
                eprintln!(
                    "TCL_LSP_TRACE_METADATA_SCHEMA head={:?} canonical={:?} realm={realm:?} context={:p} unresolved={:?}",
                    words.head_literal(),
                    self.facts.canonical_command,
                    context.context(),
                    resolution.unresolved(),
                );
            }
            apply(&selected?)
        })
    }

    /// Completion of this selected native implementation after the original
    /// words evaluated. Frozen values travel with their effective arguments;
    /// dynamic options and argv expansion retain the registry's unknown route.
    #[must_use]
    pub fn completion_route(
        &self,
        registry: &CommandRegistry,
    ) -> tcl_registry::completion_route::InvocationCompletionRoute {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        if self
            .effective
            .words
            .iter()
            .any(|word| matches!(word, WordExpr::Expand { .. }))
        {
            return Route::Unknown;
        }
        let values: Vec<_> = (0..self.arguments.len())
            .map(|index| self.argument_literal(index))
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(|value| {
                value.as_deref().map_or(
                    tcl_registry::InvocationWord::Dynamic,
                    tcl_registry::InvocationWord::Literal,
                )
            })
            .collect();
        let mut arguments = tcl_registry::InvocationArguments::structured(&words);
        if let Some(dialect) = self.dialect {
            arguments = arguments.with_dialect(dialect);
        }
        registry
            .invocation_completion_route(
                &self.facts.canonical_command,
                arguments,
                self.dialect
                    .and_then(tcl_registry::InvocationDialect::authoring_query)
                    .or_else(|| {
                        registry
                            .profile()
                            .map(tcl_dialect::DialectProfile::surface_query)
                    }),
            )
            .unwrap_or(Route::Unknown)
    }

    /// Decode one argument under this invocation's retained grammar. A missing
    /// grammar exposes only values that do not depend on dialect rules.
    #[must_use]
    pub fn argument_literal(&self, argument: usize) -> Option<String> {
        if let Some(Some(value)) = self.evaluated_arguments.get(argument) {
            return Some(value.clone());
        }
        if let Some(dialect) = self.dialect {
            return self.effective.argument_literal(
                argument,
                dialect.lexer_grammar.escapes,
                dialect.word_values,
            );
        }
        match self.effective.words.get(argument.checked_add(1)?)? {
            WordExpr::Literal { text, .. } => Some(text.clone()),
            WordExpr::BracedLiteral { text, .. } if !text.contains("\\\n") => Some(text.clone()),
            WordExpr::Template { parts, .. }
                if parts.iter().all(
                    |part| matches!(part, WordPart::Text { text, .. } if !text.contains('\\')),
                ) =>
            {
                Some(
                    parts
                        .iter()
                        .filter_map(|part| match part {
                            WordPart::Text { text, .. } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect(),
                )
            }
            _ => None,
        }
    }
}

impl EffectiveCommandWords {
    /// Project effective argument spellings without evaluating source words.
    ///
    /// Written arguments keep their original spelling; binding-prefix arguments
    /// use their retained literal values. A mismatched source snapshot declines
    /// rather than dropping a word and changing the invocation's arity.
    #[must_use]
    pub fn argument_spellings(&self, original_args: &[String]) -> Option<Vec<String>> {
        self.argument_presentations(original_args)?
            .into_iter()
            .collect()
    }

    /// Retain every effective argument slot without inventing captured bytes.
    #[must_use]
    pub fn argument_presentations(&self, original_args: &[String]) -> Option<Vec<Option<String>>> {
        if self.words.len() != self.origins.len() {
            return None;
        }
        self.origins
            .iter()
            .zip(&self.words)
            .skip(1)
            .map(|(origin, word)| match origin {
                InvocationWordOrigin::Written(index) => {
                    original_args.get(index.checked_sub(1)?).cloned().map(Some)
                }
                InvocationWordOrigin::BindingPrefix(index) => Some(
                    self.binding_prefix
                        .get(*index)?
                        .as_registry_word()
                        .literal()
                        .map(str::to_owned),
                ),
                InvocationWordOrigin::ExpandedElement { .. } => match word {
                    WordExpr::Opaque { text, .. } => Some(Some(text.clone())),
                    _ => None,
                },
                InvocationWordOrigin::ResolvedHead => None,
            })
            .collect()
    }

    /// Decode one effective argument only when its source representation is static.
    /// No variable substitution, script evaluation, or expansion is performed.
    #[must_use]
    pub fn argument_literal(
        &self,
        argument: usize,
        escapes: EscapeSyntax,
        rules: WordValueRules,
    ) -> Option<String> {
        match effective_invocation_word(self.words.get(argument.checked_add(1)?)?, escapes, rules) {
            EffectiveInvocationWord::Literal(value) => Some(value),
            EffectiveInvocationWord::ByteLiteral(_)
            | EffectiveInvocationWord::Dynamic
            | EffectiveInvocationWord::ArrayElementName { .. }
            | EffectiveInvocationWord::Expanded
            | EffectiveInvocationWord::KnownExpansion(_)
            | EffectiveInvocationWord::KnownByteExpansion(_)
            | EffectiveInvocationWord::Opaque => None,
        }
    }

    /// Map an effective argument to a written argument, when it has one.
    #[must_use]
    pub fn written_argument(&self, argument: usize) -> Option<usize> {
        match self.origins.get(argument + 1)? {
            InvocationWordOrigin::Written(index) => index.checked_sub(1),
            InvocationWordOrigin::ResolvedHead
            | InvocationWordOrigin::BindingPrefix(_)
            | InvocationWordOrigin::ExpandedElement { .. } => None,
        }
    }
}

/// Nominal metadata for one possible registry implementation. This type
/// deliberately has no execution effects, completion or purity proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryInvocationShape {
    hosted_source: Option<std::sync::Arc<OriginalConditionalVendorRegistryMetadata>>,
    /// Possible registry implementation identity.
    pub command: String,
    /// Nominal semantic operation, for advisory interpretation only.
    pub operation: tcl_registry::SemanticOperationId,
    /// Readonly lifecycle metadata from this candidate's selected descriptor.
    pub data_collection: Option<tcl_registry::events::DataCollectionOperation>,
    /// Possible source execution side; this supplies no entered body.
    pub side_switch_target: Option<tcl_registry::side_effects::SideSwitchTarget>,
    /// Available selected options and their actual abbreviation policy.
    pub option_keywords: tcl_registry::abbrev::KeywordTable<'static>,
    /// Possible source flow action, without a successful effect or handler.
    pub irules_source_flow_action: Option<tcl_registry::events::IrulesSourceFlowAction>,
    /// Possible response commitment from this candidate's source descriptor.
    pub source_may_commit_response: bool,
    /// Possible roles relative to the selected command or member operands.
    pub roles: Vec<(u8, tcl_registry::ArgRole)>,
    /// Selected member operands' offset in effective post-head argv, including
    /// captured alias prefixes. This is independent of compiler selector removal.
    pub argument_offset: usize,
    /// Whether the candidate's role grammar was determinate.
    pub roles_complete: bool,
    /// Traits of this candidate, never a guarantee about the actual invocation.
    pub possible_traits: tcl_registry::Traits,
    /// Declared candidate result type, never an established runtime value type.
    pub nominal_return_type: Option<tcl_registry::TclType>,
    /// Frame and timing of each `Body` role's script, keyed like [`Self::roles`].
    pub script_contexts: Vec<(u8, tcl_registry::evaluation_context::ScriptArgumentContext)>,
    /// Alias-composed source words and original argument indices.
    pub effective: EffectiveCommandWords,
}

/// Body navigation and definite grammar are separate projections of the same
/// original invocation. Possible roles never establish an entered frame,
/// successful execution, variable store or scoped command definition.
pub(crate) struct InvocationBodyAssistance {
    caller_template_source: Option<Box<CallerTemplateSource>>,
    pub possible_roles: Vec<(usize, tcl_registry::ArgRole)>,
    pub definite_roles: Vec<(usize, tcl_registry::ArgRole)>,
    pub possible_traits: tcl_registry::Traits,
    pub definite_traits: tcl_registry::Traits,
    pub definite_scope: Option<&'static tcl_registry::scoped::ScopedCommandEnv>,
    pub possible_definition_grammars: Vec<&'static tcl_registry::definer::DefinitionBodyGrammar>,
    pub possible_case_lists: Vec<(tcl_registry::spec::CaseListSpec, usize)>,
    pub possible_operations: Vec<(tcl_registry::SemanticOperationId, EffectiveCommandWords)>,
    pub definite_invocation: Option<ResolvedStatementInvocation>,
    /// Accepted own-body layout, conditional on this declaration's entry.
    /// It never establishes caller depth, actual argument values or stores.
    conditional_invocation: Option<ConditionalBodyInvocation>,
    /// Source-only receiver trait recipe. Runtime namespace and entry stay
    /// unavailable, and no definite role or execution projection uses this.
    receiver_trait_invocation: Option<ReceiverTraitInvocation>,
    logical_trait_invocation: Option<LogicalTraitInvocation>,
    /// Conditional source-only roles from one retained original Logical
    /// lambda body. Kept separate from ordinary invocation advice.
    logical_lambda_roles: Option<ConditionalLogicalLambdaRoles>,
    body_parameter_source: Option<BodyParameterSourceInvocation>,
    pub declared_roles: Vec<(usize, tcl_registry::ArgRole)>,
    pub unknown_residual: bool,
    pub may_be_absent: bool,
    /// May usage of formals by retained implementation candidates, excluding
    /// nominal catalogue assistance. This grants no copy, frame or effects.
    parameter_advice: ParameterRoleAdvice,
}

struct CallerTemplateSource {
    tokens: CommandTokens,
    invocation: ResolvedStatementInvocation,
}

#[derive(Default)]
pub(crate) struct ParameterRoleAdvice {
    candidates: Vec<RegistryInvocationShape>,
    closed: bool,
}

struct ReceiverTraitInvocation {
    entry: std::sync::Arc<crate::command_binding::SourceDeclaredReceiverBodyEntry>,
    invocation: ResolvedStatementInvocation,
}

struct LogicalTraitInvocation {
    /// Separate conditional source recipe; never enters production closed advice.
    source_parameter_shape: Option<RegistryInvocationShape>,
    input: crate::analyser::ResolvedAnalysisInput,
    body: std::sync::Arc<crate::command_binding::OriginalLogicalProcedureBody>,
    words: source_structure::OriginalRegistryWords,
    invocation: ResolvedStatementInvocation,
}

struct ConditionalLogicalLambdaRoles {
    lambda: std::sync::Arc<crate::command_binding::OriginalLogicalLambdaBody>,
    scope: std::sync::Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>,
    input: crate::analyser::ResolvedAnalysisInput,
    words: source_structure::OriginalRegistryWords,
    invocation: ResolvedStatementInvocation,
    shape: RegistryInvocationShape,
}

pub(crate) struct LogicalLambdaSourceOwner {
    pub(crate) lambda: std::sync::Arc<crate::command_binding::OriginalLogicalLambdaBody>,
    pub(crate) scope: std::sync::Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>,
}

struct BodyParameterSourceInvocation {
    entry: crate::command_binding::OriginalBodyParameterSourceScope,
    words: Vec<tcl_lexer::NativeWord>,
    shape: RegistryInvocationShape,
    invocation: ResolvedStatementInvocation,
}

/// Borrowed conditional source parameter layout from its genuine retained
/// body/prefix owner. Ordinary invocation facts retain their own proof purpose.
/// This facade supplies no closed advice, Native handler or Normal operation.
pub(crate) struct ConditionalParameterSourceInvocation<'a> {
    invocation: &'a ResolvedStatementInvocation,
    shape: &'a RegistryInvocationShape,
    pub(crate) facts: &'a tcl_registry::InvocationFacts,
}

struct ConditionalBodyInvocation {
    entry: std::sync::Arc<crate::command_binding::SourceConditionalBodyEntry>,
    invocation: ResolvedStatementInvocation,
}

pub(crate) enum ConditionalParameterSourceAliasLayout {
    Unavailable,
    Available((tcl_registry::FrameLevel, usize)),
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_argument_slice_purpose_tests;

#[cfg(test)]
mod original_logical_body_topology_input_tests;

#[cfg(test)]
#[path = "registry_invocation/original_source_purpose_tests.rs"]
mod original_source_purpose_tests;

mod source_body_names;

pub(crate) use source_body_names::original_authored_body_name_candidates;

mod authored_controls;
pub(crate) use authored_controls::{
    original_authored_control_invocation, original_authored_scalar_invocation,
};

#[cfg(test)]
mod original_body_parameter_source_tests;

#[cfg(test)]
mod authored_simulation_structure_controls;
