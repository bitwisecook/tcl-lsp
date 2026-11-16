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
    original_procedure_has_execution_trace, trace_set_requires_original_owner,
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
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{
    CommandRegistry, InvocationFacts, InvocationResolutionUnresolved, InvocationWord,
    InvocationWordKind, InvocationWords,
};

use crate::ir::{CommandTokens, Statement, WordExpr, WordPart};
use crate::segmenter::SegmentedCommand;
use tcl_syntax::word_rules::WordValueRules;

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
    let mut segments = crate::segmenter::segment_commands_with_offset_and_config(
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

/// Structured semantics of one exact source invocation for analysis.
///
/// Conditional declarations describe their own frame only. This projection
/// grants no native compilation admission, executed activation, successful
/// store, caller identity or outward effect.
pub(crate) struct LogicalStructuredInvocation {
    invocation: ResolvedStatementInvocation,
    source_roles: Option<StructuredSourceRoles>,
    source_purpose: crate::command_binding::OriginalInvocationSourcePurpose,
}

/// Distinct source purposes retain their own registry-issued role owners.
/// The Native case projection needs an actual closed compiler observation;
/// neither purpose grants a handler, entered arm or completed operation.
enum StructuredSourceRoles {
    Logical(tcl_registry::resolved_invocation::AuthoredLogicalSourceRoleProjection),
    NativeCase(tcl_registry::case_bodies::CaseBodyOperands),
    AuthoredSimulationCase(tcl_registry::case_bodies::CaseBodyOperands),
}

impl StructuredSourceRoles {
    fn selection_unknown(&self) -> bool {
        match self {
            Self::Logical(roles) => roles.selection_unknown(),
            Self::NativeCase(roles) | Self::AuthoredSimulationCase(roles) => {
                roles.selection_unknown
            }
        }
    }

    fn argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        match self {
            Self::Logical(roles) => roles.argument_roles().to_vec(),
            Self::NativeCase(roles) | Self::AuthoredSimulationCase(roles) => roles
                .bodies
                .iter()
                .map(|body| (body.argument, tcl_registry::ArgRole::Body))
                .collect(),
        }
    }
}

impl LogicalStructuredInvocation {
    pub(crate) fn is_authored_simulation(&self) -> bool {
        self.source_purpose
            == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    }

    pub(crate) fn canonical_command(&self) -> &str {
        &self.invocation.facts.canonical_command
    }

    pub(crate) fn descriptor_key(&self) -> tcl_registry::CanonicalCommandDescriptorKey<'_> {
        self.invocation.facts.descriptor_key()
    }

    /// Borrow selected source descriptors under the genuine supplied generation.
    /// Original direct-word correspondence remains part of this issuer.
    pub(crate) fn with_metadata_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        realm: tcl_dialect::model::InvocationRealm,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        self.invocation
            .with_metadata_schema(registry, context, realm, apply)
    }

    pub(crate) fn lowering_hook(&self) -> Option<tcl_registry::hooks::LoweringHookId> {
        self.invocation.facts.lowering_hook
    }

    pub(crate) fn written_argument(&self, effective: usize) -> Option<usize> {
        self.invocation.effective.written_argument(effective)
    }

    pub(crate) fn written_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.source_roles.as_ref().map_or_else(
            || self.invocation.written_argument_roles(),
            |roles| {
                roles
                    .argument_roles()
                    .iter()
                    .filter_map(|&(argument, role)| {
                        self.invocation
                            .effective
                            .written_argument(argument)
                            .map(|written| (written, role))
                    })
                    .collect()
            },
        )
    }

    /// Conditional source body positions from the exact retained Logical
    /// descriptor. These roles grant no handler, entered frame or Normal path.
    pub(crate) fn conditional_logical_body_indices(&self) -> Option<Vec<usize>> {
        if self.source_purpose != crate::command_binding::OriginalInvocationSourcePurpose::Logical {
            return None;
        }
        let StructuredSourceRoles::Logical(roles) = self.source_roles.as_ref()? else {
            return None;
        };
        if roles.selection_unknown()
            || roles.case_body_operands().is_some()
            || self.invocation.facts.operation
                != tcl_registry::SemanticOperationId::StructuredLowering(
                    tcl_registry::hooks::LoweringHookId::If,
                )
            || self.invocation.facts.body_kind != tcl_registry::BodyKind::Plain
            || self.invocation.facts.body_interpreter != tcl_registry::BodyInterpreter::Current
            || self.invocation.facts.frame_effect.is_some()
            || self.invocation.facts.body_execution.is_some()
            || self
                .invocation
                .facts
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY)
        {
            return None;
        }
        Some(
            self.written_roles()
                .into_iter()
                .filter_map(|(index, role)| (role == tcl_registry::ArgRole::Body).then_some(index))
                .collect(),
        )
    }

    /// Literal name at one genuine written variable-role operand. Captured
    /// prefix words and ambiguous effective correspondence have no occurrence.
    pub(crate) fn written_variable_name_operand(&self, written: usize) -> Option<String> {
        self.written_roles()
            .iter()
            .any(|&(argument, role)| {
                argument == written
                    && matches!(
                        role,
                        tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                    )
            })
            .then_some(())?;
        let mut arguments = (0..self.invocation.arguments.len()).filter(|&argument| {
            self.invocation.effective.written_argument(argument) == Some(written)
        });
        let argument = arguments.next()?;
        if arguments.next().is_some() {
            return None;
        }
        self.invocation.argument_literal(argument)
    }

    /// Literal payload under the retained grammar; direct written slots were
    /// checked by the source issuer. This grants no evaluated Native operand.
    pub(crate) fn argument_literal(&self, argument: usize) -> Option<String> {
        self.invocation.argument_literal(argument)
    }

    pub(crate) fn loop_bindings_conditional(&self) -> bool {
        self.invocation.facts.repeated_args.iter().any(|layout| {
            layout.conditional_binding && layout.role == tcl_registry::ArgRole::LoopVarList
        })
    }

    pub(crate) fn traits(&self) -> tcl_registry::Traits {
        self.invocation.facts.traits
    }

    pub(crate) fn body_scope<'a>(
        &self,
        registry: &CommandRegistry,
        context: impl Into<InvocationMetadataContext<'a>>,
    ) -> Option<&'static tcl_registry::scoped::ScopedCommandEnv> {
        let context = context.into();
        if !context.matches_registry(registry) {
            return None;
        }
        context
            .context()
            .resolve_spec(registry, self.canonical_command())?
            .body_scope
    }

    /// Abstract routing inside this original conditional body. This cannot
    /// certify an actual completion, handler effect or native instruction.
    pub(crate) fn conditional_completion_route(
        &self,
        registry: &CommandRegistry,
    ) -> tcl_registry::completion_route::InvocationCompletionRoute {
        self.invocation.completion_route(registry)
    }

    pub(crate) fn error_context(&self) -> Option<tcl_registry::InlineBodyErrorContext> {
        self.invocation.facts.operation.inline_body_error_context()
    }
}

/// Resolve logical structure through the selected handler and original words.
/// Native compilation and runtime execution consume their separate receipts.
pub(crate) fn logical_structured_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    bindings: Option<&crate::command_binding::SourceCommandBindings>,
) -> Option<LogicalStructuredInvocation> {
    let context = body_assistance_context(registry, tokens)?;
    logical_structured_invocation_with_metadata_context(registry, context.into(), tokens, bindings)
}

fn trace_original_structure(stage: &'static str, tokens: &CommandTokens, passed: bool) {
    host_structure_observer::flag(stage, passed);
    let _ = tokens;
}

fn original_structured_source_selection(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: Option<&crate::command_binding::SourceCommandBindings>,
) -> Option<ResolvedStatementInvocation> {
    let selected =
        resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens);
    trace_original_structure("resolved_handler", tokens, selected.is_some());
    let selected = selected.or_else(|| {
        let selected = bindings.and_then(|bindings| {
            owned_body_layout_invocation(registry, context, tokens, bindings)
                .map(|owned| owned.invocation)
        });
        trace_original_structure("owned_body_layout", tokens, selected.is_some());
        selected
    });
    let selected = selected.or_else(|| {
        let selected = original_declared_structured_invocation(registry, context, tokens);
        trace_original_structure("original_declaration", tokens, selected.is_some());
        selected
    });
    let selected = selected.or_else(|| {
        let selected = original_inline_control_structured_invocation(registry, context, tokens);
        trace_original_structure("original_inline_control", tokens, selected.is_some());
        selected
    })?;
    Some(selected)
}

/// A word a simulated lookup reads without evaluating anything: a literal
/// (bare, braced, or quoted or escaped text without substitutions), or one
/// whole scalar variable reference.
fn simulation_static_word(word: &WordExpr, config: tcl_lexer::LexerConfig) -> bool {
    match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => true,
        WordExpr::Template {
            parts,
            rejected: None,
            ..
        } => parts
            .iter()
            .all(|part| matches!(part, WordPart::Text { .. })),
        WordExpr::Variable { .. } => {
            word.sole_variable_substitution()
                .is_some_and(|(spelling, _)| {
                    tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, config)
                        .is_some()
                })
        }
        _ => false,
    }
}

fn authored_simulation_structure_is_supported(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    selected: &ResolvedStatementInvocation,
    source_roles: Option<&StructuredSourceRoles>,
) -> Option<()> {
    let binding = tokens.source_binding.as_ref()?;
    // A reached point whose registered handler is proved in its own closed
    // world needs no simulated declaration lookup; only bodies need modelling.
    let reached = binding.reached_in_closed_world()
        && binding
            .proved_handler_target()
            .is_some_and(|target| target.registry_backed);
    let advice = binding
        .declaration_operand_layout_advice(tokens)
        .or_else(|| binding.original_compilation_lookup_advice(tokens));
    let realm = match &advice {
        Some(advice) => {
            if !advice.closed_authored_simulation_source_lookup(tokens, context, registry) {
                return None;
            }
            advice.realm()
        }
        None if reached => binding.invocation_realm()?,
        None => return None,
    };
    let config = context.source_analysis_input()?.lexer_config();
    let roles = source_roles.map_or_else(
        || selected.written_argument_roles(),
        StructuredSourceRoles::argument_roles,
    );
    let bodiless = roles.iter().all(|(_, role)| {
        !matches!(
            role,
            tcl_registry::ArgRole::Body
                | tcl_registry::ArgRole::OpaqueScript
                | tcl_registry::ArgRole::LoopVarList
                | tcl_registry::ArgRole::ParamList
                | tcl_registry::ArgRole::StaticVarList
        )
    });
    // Simulated words must be static. A reached bodiless command's values
    // were evaluated at its point; only its head and role words must be.
    // The returned value is a value like any other operand, not a name.
    let simulated = |index: usize| {
        !(reached && bodiless)
            || index == 0
            || roles.iter().any(|(argument, role)| {
                argument + 1 == index && *role != tcl_registry::ArgRole::Result
            })
    };
    if tokens
        .words()
        .iter()
        .enumerate()
        .any(|(index, word)| simulated(index) && !simulation_static_word(word, config))
    {
        return None;
    }
    selected.with_metadata_schema(registry, context, realm, |schema| {
        authored_simulation_schema_is_supported(schema, tokens, &roles, bodiless)
    })?;
    Some(())
}

/// The schema half of [`authored_simulation_structure_is_supported`]: which
/// lowering structures the authored walk models, given the call's static words.
fn authored_simulation_schema_is_supported(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    tokens: &CommandTokens,
    roles: &[(usize, tcl_registry::ArgRole)],
    bodiless: bool,
) -> Option<()> {
    use tcl_registry::hooks::LoweringHookId;
    // A class or object definition, like a procedure, stores its body for
    // later dispatch; its static words above are all that is modelled.
    if schema
        .authored_source_definition_body_grammar()
        .is_some_and(|grammar| grammar.family.manufactures_runtime_commands())
    {
        return Some(());
    }
    let hook = schema.facts().lowering_hook;
    // A definition stores its body for later calls in the procedure's own
    // frame, so only a fully static name, parameter list and body is modelled.
    let all_literal = tokens.words().iter().all(|word| {
        matches!(
            word,
            WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
        )
    });
    if hook == Some(LoweringHookId::Proc) {
        return (schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
            && all_literal)
            .then_some(());
    }
    // A static `namespace eval` runs its body now, in the named namespace's
    // frame; the walk models that frame for each command it contains.
    if hook == Some(LoweringHookId::NamespaceEval) {
        return (schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
            && all_literal
            && roles.iter().all(|&(argument, role)| {
                role != tcl_registry::ArgRole::Body
                    || schema.authored_source_script_timing_at(argument)
                        == Some(tcl_registry::ScriptTiming::SameInvocation)
            }))
        .then_some(());
    }
    let static_body_evaluator =
        hook.is_some_and(|hook| static_body_evaluator_is_supported(schema, hook, roles));
    if !static_body_evaluator
        && (!(bodiless
            || matches!(
                hook,
                Some(
                    LoweringHookId::If
                        | LoweringHookId::Switch
                        | LoweringHookId::While
                        | LoweringHookId::For
                        | LoweringHookId::Foreach
                        | LoweringHookId::Lmap
                        | LoweringHookId::Catch
                        | LoweringHookId::Try
                )
            ))
            || schema.semantics.frame_effect.is_some()
            || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
            || schema.semantics.body_interpreter != tcl_registry::BodyInterpreter::Current)
    {
        return None;
    }
    for &(argument, role) in roles {
        if role == tcl_registry::ArgRole::Body
            && schema.authored_source_script_timing_at(argument)?
                != tcl_registry::ScriptTiming::SameInvocation
        {
            return None;
        }
    }
    Some(())
}

/// A selected builtin that runs its body word exactly as written: `eval` in
/// the current frame, `uplevel` in the frame its level word names, and the
/// loops `dict for`/`dict map`, `array for` and `foreachLine`. The lowerer
/// still declines a body or level word it cannot read statically.
fn static_body_evaluator_is_supported(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    hook: tcl_registry::hooks::LoweringHookId,
    roles: &[(usize, tcl_registry::ArgRole)],
) -> bool {
    use tcl_registry::frame_effect::FrameArgLayout;
    use tcl_registry::hooks::LoweringHookId;
    let semantics = &schema.semantics;
    let layout = semantics.frame_effect.map(|effect| effect.layout);
    let plain = semantics.body_kind == tcl_registry::BodyKind::Plain;
    let loop_body = roles
        .iter()
        .any(|&(_, role)| role == tcl_registry::ArgRole::LoopVarList);
    semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
        && match hook {
            LoweringHookId::Eval => plain && layout == Some(FrameArgLayout::ScriptInCurrentFrame),
            LoweringHookId::Uplevel => layout == Some(FrameArgLayout::ScriptInSelectedFrame),
            LoweringHookId::Dict | LoweringHookId::ArrayFor => {
                plain && layout.is_none() && loop_body
            }
            LoweringHookId::ForeachLine => plain && layout.is_none(),
            _ => false,
        }
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

fn original_structured_operands_are_supported(
    registry: &CommandRegistry,
    selected: &ResolvedStatementInvocation,
    source_roles: Option<&StructuredSourceRoles>,
    tokens: &CommandTokens,
) -> Option<()> {
    let result = original_structured_operands_are_supported_untraced(
        registry,
        selected,
        source_roles,
        tokens,
    );
    #[cfg(debug_assertions)]
    trace_jim_structure_fields("operands", selected, source_roles);
    crate::registry_invocation::source_structure::trace_jim_source_option(
        "jim.body.operands_supported",
        result,
    )
}

fn original_structured_operands_are_supported_untraced(
    registry: &CommandRegistry,
    selected: &ResolvedStatementInvocation,
    source_roles: Option<&StructuredSourceRoles>,
    tokens: &CommandTokens,
) -> Option<()> {
    // A possible case body is readonly advice. Unreadable option selection
    // retains generic IR instead of borrowing a definitive Switch topology.
    if source_roles.is_some_and(StructuredSourceRoles::selection_unknown) {
        #[cfg(test)]
        trace_original_structure("source_roles", tokens, false);
        host_structure_observer::flag("operands.source_roles", false);
        return None;
    }
    // Source-shape lowering requires each operand to retain its original slot.
    // Alias prefixes, selector rewriting and expanded elements use generic IR
    // until a lowerer explicitly accepts their composed operand projection.
    if (!selected.facts.arg_roles_complete && source_roles.is_none())
        || selected.facts.arity_accepts_frozen_arguments() != Some(true)
        || selected.effective.words.len() != tokens.words().len()
        || !selected
            .effective
            .origins
            .iter()
            .enumerate()
            .skip(1)
            .all(|(index, origin)| *origin == InvocationWordOrigin::Written(index))
    {
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS").is_some() {
            trace_original_structure(
                "roles_complete",
                tokens,
                selected.facts.arg_roles_complete || source_roles.is_some(),
            );
            trace_original_structure(
                "arity",
                tokens,
                selected.facts.arity_accepts_frozen_arguments() == Some(true),
            );
            trace_original_structure(
                "word_count",
                tokens,
                selected.effective.words.len() == tokens.words().len(),
            );
            trace_original_structure(
                "written_origins",
                tokens,
                selected
                    .effective
                    .origins
                    .iter()
                    .enumerate()
                    .skip(1)
                    .all(|(index, origin)| *origin == InvocationWordOrigin::Written(index)),
            );
        }
        host_structure_observer::flag("operands.shape", false);
        return None;
    }
    if selected.facts.successful_handler
        == Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::ExpressionArguments)
        && tokens
            .source_binding
            .as_ref()?
            .conditional_expression_evaluation(registry, tokens)
            .is_none_or(|expression| !expression.semantic_lookup_closed())
    {
        #[cfg(test)]
        trace_original_structure("expression_lookup", tokens, false);
        host_structure_observer::flag("operands.expression_lookup", false);
        return None;
    }
    #[cfg(test)]
    trace_original_structure("complete", tokens, true);
    Some(())
}

fn traced_original_source_purpose(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<crate::command_binding::OriginalInvocationSourcePurpose> {
    source_structure::trace_jim_source_option(
        "jim.body.purpose",
        host_structure_observer::option(
            "source.purpose",
            host_structure_observer::option("source.binding", tokens.source_binding.as_ref())?
                .original_source_advice_purpose(tokens, context, registry),
        ),
    )
    .inspect(|purpose| {
        use crate::command_binding::OriginalInvocationSourcePurpose as Purpose;
        for (stage, expected) in [
            ("purpose.execution", Purpose::Execution),
            ("purpose.logical", Purpose::Logical),
            ("purpose.standalone", Purpose::Standalone),
            ("purpose.authored_simulation", Purpose::AuthoredSimulation),
        ] {
            host_structure_observer::flag(stage, *purpose == expected);
        }
    })
}

/// Select source structure using the caller's complete immutable context.
/// Missing or foreign context refuses; this supplies no executed body/frame.
pub(crate) fn logical_structured_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: Option<&crate::command_binding::SourceCommandBindings>,
) -> Option<LogicalStructuredInvocation> {
    let source_purpose = traced_original_source_purpose(registry, context, tokens)?;
    let logical_source =
        source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) {
        trace_original_structure("registry", tokens, false);
        return None;
    }
    let context = if logical_source {
        source_structure::trace_jim_source_option(
            "jim.body.metadata",
            tokens
                .source_binding
                .as_ref()?
                .original_invocation_metadata_for_input(
                    tokens,
                    host_structure_observer::option(
                        "metadata.input",
                        context.source_analysis_input(),
                    )?,
                    registry,
                ),
        )?
    } else {
        context
    };
    if bindings.is_some_and(|bindings| bindings.original_arguments_rejected_before_handler(tokens))
    {
        trace_original_structure("argument_entry", tokens, false);
        return None;
    }
    let selected = original_structured_source_selection(registry, context, tokens, bindings)?;
    let source_roles = if logical_source {
        let binding = tokens.source_binding.as_ref()?;
        let realm = binding.original_source_realm_for_input(
            tokens,
            host_structure_observer::option("metadata.input", context.source_analysis_input())?,
            registry,
        )?;
        selected
            .with_metadata_schema(registry, context, realm, |schema| {
                schema.authored_logical_source_role_projection()
            })
            .map(StructuredSourceRoles::Logical)
    } else if !selected.facts.arg_roles_complete {
        // Case syntax retains its independently selected source purpose.
        // Native lookup requires its original compiler observation; authored
        // simulation retains its source namespace without a physical frame.
        let binding = tokens.source_binding.as_ref()?;
        let authored_simulation = source_purpose
            == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation;
        let lookup = if authored_simulation {
            binding
                .declaration_operand_layout_advice(tokens)
                .or_else(|| binding.original_compilation_lookup_advice(tokens))?
        } else {
            binding.original_compilation_lookup_advice(tokens)?
        };
        if if authored_simulation {
            !lookup.closed_authored_simulation_source_lookup(tokens, context, registry)
        } else {
            lookup.has_unclosed_source_coverage()
        } {
            return None;
        }
        match selected.authored_case_body_selection.as_ref()? {
            AuthoredSourceCaseBodySelection::Selected(roles)
                if !roles.selection_unknown
                    && roles
                        .bodies
                        .iter()
                        .all(|body| selected.argument_literal(body.argument).is_some()) =>
            {
                Some(if authored_simulation {
                    StructuredSourceRoles::AuthoredSimulationCase(roles.clone())
                } else {
                    StructuredSourceRoles::NativeCase(roles.clone())
                })
            }
            _ => None,
        }
    } else {
        None
    };
    if source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    {
        authored_simulation_structure_is_supported(
            registry,
            context,
            tokens,
            &selected,
            source_roles.as_ref(),
        )?;
    }
    original_structured_operands_are_supported(registry, &selected, source_roles.as_ref(), tokens)?;
    Some(LogicalStructuredInvocation {
        invocation: selected,
        source_roles,
        source_purpose,
    })
}

/// Conditional structure of one unchanged original source invocation.
/// The shared issuer requires closed original lookup, complete roles, accepted
/// arity and direct written operand correspondence. It can describe an
/// unentered declaration body; it grants no Normal completion, native compiler
/// admission, physical frame or executed store.
#[must_use]
pub fn original_structured_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    logical_structured_invocation(registry, tokens, None).map(|selected| selected.invocation)
}

/// Select unchanged original source structure under the supplied metadata.
/// Complete source consumers validate their input with
/// [`InvocationMetadataContext::for_source_input`]. Missing ownership refuses;
/// direct operand correspondence grants no entered frame or Normal completion.
#[must_use]
pub fn original_structured_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    logical_structured_invocation_with_metadata_context(registry, context, tokens, None)
        .map(|selected| selected.invocation)
}

/// Same-owner quiet scalar-store source coverage, not registry transition facts.
/// Only the original full-input/point/operand and quiet-write kernel below can
/// issue this receipt. It grants no Native, physical frame or Normal capability.
pub struct OriginalQuietScalarStoreSourceCoverage<'a> {
    selected: ResolvedStatementInvocation,
    analysis: &'a crate::analyser::AnalysisResult,
}

impl OriginalQuietScalarStoreSourceCoverage<'_> {
    /// The exact invocation whose source obligations the issuer checked.
    #[must_use]
    pub const fn invocation(&self) -> &ResolvedStatementInvocation {
        &self.selected
    }

    /// A receipt cannot discharge obligations for another analysis owner.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        std::ptr::eq(self.analysis, analysis)
    }
}

/// Coverage of one actual completed quiet scalar store in authored source.
/// Original point, operands, receiver and handler completion stay independent;
/// no compiler visit or descriptor callback absence supplies their evidence.
#[must_use]
pub fn original_authored_quiet_store_for_source_coverage<'a>(
    source: &str,
    analysis: &'a crate::analyser::AnalysisResult,
    tokens: &CommandTokens,
) -> Option<OriginalQuietScalarStoreSourceCoverage<'a>> {
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let image = tcl_lexer::SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let realm = analysis.retained_command_realm()?;
    if !realm.matches_resolved_analysis_input(input) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let offset = tokens.argv.first()?.start();
    let site = binding.invocation_site()?;
    let origin = realm.source_bindings_ref().source_origin()?;
    if site.offset != offset || !std::sync::Arc::ptr_eq(&site.source, origin) {
        return None;
    }
    let expected = realm.retained_original_tokens_for_command(&image, config, offset, tokens)?;
    if expected.source_binding.as_ref() != Some(binding)
        || !binding.original_invocation_completes_normally(tokens)
    {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    let context = InvocationMetadataContext::for_source_input(
        registry,
        input,
        config,
        analysis.resolved_profile(),
    )?;
    if binding.original_source_advice_purpose(tokens, context, registry)?
        != crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    {
        return None;
    }
    let selected = original_declared_structured_invocation(registry, context, tokens)?;
    original_structured_operands_are_supported(registry, &selected, None, tokens)?;
    let native = original_native_compiler_words(&image, tokens.words(), offset, config)?;
    realm
        .source_bindings_ref()
        .original_quiet_scalar_store_for_source_coverage(&native, &selected.facts, registry)
        .then_some(OriginalQuietScalarStoreSourceCoverage { selected, analysis })
}

/// Same-owner coverage of one known source-load transition. The completed
/// source survey remains conditional; physical leases belong to the host.
pub struct OriginalConditionalSourceLoadCoverage<'a> {
    selected: ResolvedStatementInvocation,
    analysis: &'a crate::analyser::AnalysisResult,
    outcome: &'a crate::command_binding::OriginalSourceConditionalOutcome,
    load: source_structure::OriginalProjectSourceLoad,
}
impl OriginalConditionalSourceLoadCoverage<'_> {
    /// Only this selected invocation may consume the source-load coverage.
    #[must_use]
    pub const fn invocation(&self) -> &ResolvedStatementInvocation {
        &self.selected
    }
    /// Never attach this proof to another current analysis allocation.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        std::ptr::eq(self.analysis, analysis) && self.outcome.matches_prefix_analysis(analysis)
    }
    /// Exact original path ordinal of the registered load transition.
    #[must_use]
    pub const fn path_argument(&self) -> usize {
        self.load.path_argument()
    }
}

/// Rejoin the exact current static load with its compiler-issued finite survey.
/// This grants no physical loading, Native invocation or operation Normal fact.
#[must_use]
pub fn original_conditional_source_load_for_coverage<'a>(
    source: &str,
    analysis: &'a crate::analyser::AnalysisResult,
    tokens: &CommandTokens,
    survey: &crate::command_binding::OriginalSingleSourceCutSurvey<'a>,
) -> Option<OriginalConditionalSourceLoadCoverage<'a>> {
    let input = analysis.resolved_input.as_ref()?;
    let image = tcl_lexer::SourceImage::document(source);
    let config = input.lexer_config();
    let realm = analysis.retained_command_realm()?;
    if !analysis.matches_original_source_image(&image, config)
        || !realm.matches_resolved_analysis_input(input)
    {
        return None;
    }
    let offset = tokens.argv.first()?.start();
    let binding = tokens.source_binding.as_ref()?;
    let expected = realm.retained_original_tokens_for_command(&image, config, offset, tokens)?;
    if expected.source_binding.as_ref() != Some(binding) {
        return None;
    }
    let load = source_structure::original_project_source_load_at(source, analysis, offset)?;
    load.literal_path()?;
    if !survey.matches_load(analysis, &load) {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    let context = InvocationMetadataContext::for_source_input(
        registry,
        input,
        config,
        analysis.resolved_profile(),
    )?;
    let selected = original_declared_structured_invocation(registry, context, tokens)?;
    original_structured_operands_are_supported(registry, &selected, None, tokens)?;
    let native = original_native_compiler_words(&image, tokens.words(), offset, config)?;
    let name = binding.original_head_name_input(tokens)?;
    let captured = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        &native,
        name.policy().string_protocol(),
    )
    .ok()?;
    if native.len() != 2 || (0..native.len()).any(|ordinal| captured.literal(ordinal).is_none()) {
        return None;
    }
    Some(OriginalConditionalSourceLoadCoverage {
        selected,
        analysis,
        outcome: survey.outcome(),
        load,
    })
}

/// Exact conditional result-completion source coverage, without execution,
/// Native admission, completed effects or Normal operation completion.
/// Every executable argument component remains an independent source survey.
pub struct OriginalAuthoredResultCompletionSourceCoverage<'a> {
    selected: ResolvedStatementInvocation,
    analysis: &'a crate::analyser::AnalysisResult,
    regions: Vec<tcl_lexer::Span>,
}

impl OriginalAuthoredResultCompletionSourceCoverage<'_> {
    /// Return the selected statement invocation retained by this source-only coverage.
    #[must_use]
    pub const fn invocation(&self) -> &ResolvedStatementInvocation {
        &self.selected
    }

    /// Check whether this coverage borrows the same analysis allocation.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        std::ptr::eq(self.analysis, analysis)
    }

    /// All original executable command arguments, never Result text as script.
    #[must_use]
    pub fn argument_regions(&self) -> &[tcl_lexer::Span] {
        &self.regions
    }
}

fn registered_source_completion_resolvers(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> bool {
    use tcl_registry::state_transition::LogicalSourceStateTransitionResolver;
    use tcl_registry::world_effect::SourceWorldEffectResolver;
    let effects = schema.semantics.world_effects;
    let transitions = schema.semantics.state_transitions;
    effects.is_declared()
        && transitions.is_declared()
        && [effects.command, effects.subcommand, effects.form]
            .into_iter()
            .flatten()
            .all(|descriptor| {
                matches!(
                    descriptor.source_resolver,
                    Some(SourceWorldEffectResolver::Effects(_))
                )
            })
        && [
            transitions.command,
            transitions.subcommand,
            transitions.form,
        ]
        .into_iter()
        .flatten()
        .all(|descriptor| {
            matches!(
                descriptor.logical_source_resolver,
                Some(LogicalSourceStateTransitionResolver::CompletionState(_))
            )
        })
}

fn registered_result_completion_schema_is_supported(
    selected: &ResolvedStatementInvocation,
    input: &crate::analyser::ResolvedAnalysisInput,
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> bool {
    use tcl_registry::world_effect::{CallbackKinds, Reentrancy, WorldStateDomain};
    let mut dialect = tcl_registry::InvocationDialect::of_profile(input.unit_profile());
    dialect.lexer_grammar = input.lexer_config().grammar_over(dialect.lexer_grammar);
    dialect.word_values =
        tcl_syntax::word_rules::WordValueRules::from_config(&input.lexer_config());
    // Proof comes only from registered source resolvers, never a
    // Native resolver fallback or the metadata family's Native label.
    if !registered_source_completion_resolvers(schema) {
        return false;
    }
    let facts = schema.facts();
    let (roles, complete) = schema.authored_source_argument_roles();
    let (effects, transitions) = schema.authored_source_effects_and_state_transitions(dialect);

    facts.descriptor_key() == selected.facts.descriptor_key()
        && schema.semantics.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Return,
            )
        && schema.semantics.argument_offset == 0
        && facts.native_result
            == Some(tcl_registry::native_result::NativeResultContract::ReturnResult)
        && complete
        && match schema.words.arguments().exact_argv_len() {
            Some(0) => roles.is_empty(),
            Some(1) => roles.as_slice() == [(0, tcl_registry::ArgRole::Result)],
            _ => false,
        }
        && schema
            .words
            .arguments()
            .exact_argv_len()
            .is_some_and(|count| count <= 1)
        && facts.arity_accepts_frozen_arguments() == Some(true)
        && schema.semantics.body_kind == tcl_registry::BodyKind::Plain
        && schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
        && schema.semantics.body_execution.is_none()
        && schema.semantics.frame_effect.is_none()
        && !facts.traits.intersects(
            tcl_registry::FRAME_REACH_TRAITS
                | tcl_registry::Traits::INTROSPECTS_BY_NAME
                | tcl_registry::Traits::TARGETS_VARIABLE_BY_NAME
                | tcl_registry::Traits::REFLECTS_COMMAND_NAMES
                | tcl_registry::Traits::CREATES_SCOPE_ALIAS
                | tcl_registry::Traits::CREATES_DYNAMIC_BARRIER
                | tcl_registry::Traits::DEFERS_BODY
                | tcl_registry::Traits::PERFORMS_SUBSTITUTION,
        )
        && schema.semantics.world_effects.is_declared()
        && schema.semantics.state_transitions.is_declared()
        && effects.callback().kinds == CallbackKinds::NONE
        && effects.callback().reentrancy == Reentrancy::Never
        && effects.accesses().iter().all(|access| {
            matches!(
                access.domain,
                WorldStateDomain::InterpreterResult
                    | WorldStateDomain::CompletionState
                    | WorldStateDomain::LegacyExternal(
                        tcl_registry::side_effects::SideEffectTarget::EventControl
                    )
            )
        })
        && transitions.facts().is_empty()
        && transitions.command_resolution_impact() == tcl_registry::CommandResolutionImpact::None
}

fn authored_result_completion_effects_are_supported(
    selected: &ResolvedStatementInvocation,
    input: &crate::analyser::ResolvedAnalysisInput,
    context: InvocationMetadataContext<'_>,
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> bool {
    RegistryInvocationShape::from_facts(&selected.facts, selected.effective.clone())
        .with_source_schema(registry, context, tokens, |schema| {
            Some(registered_result_completion_schema_is_supported(
                selected, input, schema,
            ))
        })
        .is_some_and(|closed| closed)
}

/// Source-only result completion under the actual current authored schema.
/// Arguments retain their independent conditional closure and nested commands;
/// the operation's abrupt completion is never converted to Normal.
#[must_use]
pub fn original_authored_result_completion_for_source_coverage<'a>(
    source: &str,
    analysis: &'a crate::analyser::AnalysisResult,
    tokens: &CommandTokens,
) -> Option<OriginalAuthoredResultCompletionSourceCoverage<'a>> {
    let input = analysis.resolved_input.as_ref()?;
    let config = input.lexer_config();
    let image = tcl_lexer::SourceImage::document(source);
    if !analysis.matches_original_source_image(&image, config) {
        return None;
    }
    let realm = analysis.retained_command_realm()?;
    if !realm.matches_resolved_analysis_input(input) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let offset = tokens.argv.first()?.start();
    let site = binding.invocation_site()?;
    let origin = realm.source_bindings_ref().source_origin()?;
    if site.offset != offset || !std::sync::Arc::ptr_eq(&site.source, origin) {
        return None;
    }
    let expected = realm.retained_original_tokens_for_command(&image, config, offset, tokens)?;
    if expected.source_binding.as_ref() != Some(binding) {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    let context = InvocationMetadataContext::for_source_input(
        registry,
        input,
        config,
        analysis.resolved_profile(),
    )?;
    if binding.original_source_advice_purpose(tokens, context, registry)?
        != crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    {
        return None;
    }
    let selected = original_declared_structured_invocation(registry, context, tokens)?;
    original_structured_operands_are_supported(registry, &selected, None, tokens)?;
    if selected.effective.origins.first() != Some(&InvocationWordOrigin::ResolvedHead)
        || selected
            .effective
            .origins
            .iter()
            .enumerate()
            .skip(1)
            .any(|(index, origin)| *origin != InvocationWordOrigin::Written(index))
    {
        return None;
    }
    let native = original_native_compiler_words(&image, tokens.words(), offset, config)?;
    if !realm
        .source_bindings_ref()
        .original_conditional_arguments_for_source_coverage(&native, registry)
        || !authored_result_completion_effects_are_supported(
            &selected, input, context, registry, tokens,
        )
    {
        return None;
    }
    let mut regions = std::collections::BTreeSet::new();
    for word in &native {
        for part in word.executable_parts().all_parts() {
            match part.part {
                tcl_lexer::ExecutablePart::Command { body } => {
                    regions.insert((body.start(), body.end()));
                }
                tcl_lexer::ExecutablePart::ParseError(_)
                | tcl_lexer::ExecutablePart::Expression { .. } => return None,
                _ => {}
            }
        }
    }
    Some(OriginalAuthoredResultCompletionSourceCoverage {
        selected,
        analysis,
        regions: regions
            .into_iter()
            .map(|(a, b)| tcl_lexer::Span::new(a, b))
            .collect(),
    })
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
    command: crate::segmenter::SegmentedCommand,
}
impl<'a> OriginalSourceCutProcedureDeclaration<'a> {
    /// Same original selected owner, never an equal-text replacement analysis.
    #[must_use]
    pub fn is_for_analysis(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        std::ptr::eq(self.analysis, analysis)
            && self
                .head_correspondence
                .matches_selected_source(self.body.source_words())
    }
    /// Source-only facts for the independent command-link coverage checks.
    #[must_use]
    pub const fn invocation(&self) -> &ResolvedStatementInvocation {
        &self.selected
    }
    /// Retain this same declaration and completed cut for conditional body syntax.
    /// No alternate header/body or execution entry can replace a refused owner.
    #[must_use]
    pub fn into_body_layout(
        self,
        survey: &crate::command_binding::OriginalSingleSourceCutSurvey<'_>,
    ) -> Option<OriginalSourceCutBodyLayout<'a>> {
        OriginalSourceCutBodyLayout::capture(self, survey)
    }

    /// Exact complete original deferred body, still requiring its own survey.
    #[must_use]
    pub const fn body(&self) -> &OriginalSourceScriptBody {
        &self.body
    }
}

/// Reborrow declaration syntax from the actual child or issued suffix owner.
/// The ordinary implicit source-purpose and execution consumers are unchanged.
#[must_use]
pub fn original_source_cut_procedure_declaration_for_coverage<'a>(
    source: &str,
    analysis: &'a crate::analyser::AnalysisResult,
    command: &crate::segmenter::SegmentedCommand,
    survey: &crate::command_binding::OriginalSingleSourceCutSurvey<'_>,
) -> Option<OriginalSourceCutProcedureDeclaration<'a>> {
    if source_cut_refuses(
        "survey-or-partial",
        !survey.owns_declaration_command(analysis, command) || command.is_partial,
    ) {
        return None;
    }
    let words = source_cut_stage(
        "registry-words",
        source_structure::source_registry_words(source, analysis, command),
    )?;
    let input = source_cut_stage("resolved-input", analysis.resolved_input.as_ref())?;
    let context = input.borrowed_context_registry();
    let realm = source_cut_stage("retained-realm", analysis.retained_command_realm())?;
    let image = source_cut_stage("source-image", realm.original_source_image())?;
    if source_cut_refuses(
        "image-words-context",
        source_cut_image_words_refused(&words, image, source, input, context),
    ) {
        return None;
    }
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        input.lexer_config(),
        command,
    );
    realm.stamp_original_tokens(&mut tokens);
    if source_cut_refuses(
        "static-four-words",
        tokens.words().len() != 4
            || tokens.words().iter().any(|word| {
                !matches!(
                    word,
                    WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
                )
            }),
    ) {
        return None;
    }
    let native = source_cut_stage(
        "native-words",
        original_native_compiler_words(
            image,
            tokens.words(),
            source_cut_stage("head-argv", command.argv.first())?
                .span
                .start(),
            input.lexer_config(),
        ),
    )?;
    if source_cut_refuses(
        "native-word-correspondence",
        words.origins().len() != native.len()
            || source_cut_stage("head-source", words.head_source())?.word() != native.first()
            || (0..3)
                .any(|argument| words.original_argument_word(argument) != native.get(argument + 1)),
    ) {
        return None;
    }
    let head_correspondence = source_cut_head(
        source,
        (input.lexer_config(), input, realm),
        context,
        &tokens,
        &words,
    )?;
    let facts = source_cut_stage(
        "schema-facts",
        words.with_source_schema(context, source_schema_queries::facts),
    )?;
    let selected = source_cut_stage(
        "selection",
        source_cut_declaration_selection(&tokens, &words, facts),
    )?;
    let mut bodies = source_cut_stage(
        "script-bodies",
        words.source_script_bodies_for_syntax_coverage(context),
    )?;
    if source_cut_refuses("single-body-count", bodies.len() != 1) {
        return None;
    }
    let body = source_cut_stage("one-body", bodies.pop())?;
    let original = source_cut_stage("body-word", words.original_argument_word(2))?;
    if source_cut_refuses(
        "body-source-context",
        source_cut_body_refused(original, &body, image, input, context),
    ) {
        return None;
    }
    source_cut_stage("body-commands", body.original_commands(context))?;
    Some(OriginalSourceCutProcedureDeclaration {
        analysis,
        selected,
        body,
        head_correspondence,
        header: tokens,
        command: command.clone(),
    })
}

fn source_cut_body_refused(
    original: &tcl_lexer::NativeWord,
    body: &OriginalSourceScriptBody,
    image: &tcl_lexer::SourceImage,
    input: &crate::analyser::ResolvedAnalysisInput,
    context: &tcl_registry::model::ContextRegistry,
) -> bool {
    original.group().kind != tcl_lexer::WordKind::Braced
        || original.group().expand
        || body.original_container() != original
        || !body.matches_source(image, input.lexer_config())
        || !body.matches_context(context)
}

/// A value-only declaration-word join, never source command selection.
/// The existing opaque head pair retains its original purposes; command and
/// body selection still come only from the independently selected right side.
struct SourceCutDeclarationWordCorrespondence {
    heads: source_structure::OriginalSourceHeadCorrespondence,
}
impl SourceCutDeclarationWordCorrespondence {
    fn matches_selected_source(&self, words: &source_structure::OriginalRegistryWords) -> bool {
        self.heads.matches_selected_source(words)
    }
    #[cfg(test)]
    fn left(&self) -> &source_structure::OriginalRegistryWords {
        self.heads.left()
    }
    #[cfg(test)]
    fn right(&self) -> &source_structure::OriginalRegistryWords {
        self.heads.right()
    }
}

enum SourceCutStoredHead {
    Written,
    Resolved(Box<SourceCutDeclarationWordCorrespondence>),
}
impl SourceCutStoredHead {
    fn matches_selected_source(&self, words: &source_structure::OriginalRegistryWords) -> bool {
        match self {
            Self::Written => true,
            Self::Resolved(head) => head.matches_selected_source(words),
        }
    }
}

fn source_cut_head(
    source: &str,
    current: (
        tcl_lexer::LexerConfig,
        &crate::analyser::ResolvedAnalysisInput,
        &crate::realm::CommandBindingRealm,
    ),
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
    words: &source_structure::OriginalRegistryWords,
) -> Option<SourceCutStoredHead> {
    let (config, input, realm) = current;
    match words.origins().first()? {
        InvocationWordOrigin::Written(0) => Some(SourceCutStoredHead::Written),
        InvocationWordOrigin::ResolvedHead => Some(SourceCutStoredHead::Resolved(Box::new(
            source_cut_resolved_head_correspondence(
                source, config, input, realm, context, tokens, words,
            )?,
        ))),
        _ => None,
    }
}

fn source_cut_image_words_refused(
    words: &source_structure::OriginalRegistryWords,
    image: &tcl_lexer::SourceImage,
    source: &str,
    input: &crate::analyser::ResolvedAnalysisInput,
    context: &tcl_registry::model::ContextRegistry,
) -> bool {
    image.channel() != tcl_lexer::SourceChannel::Document
        || image.bytes() != source.as_bytes()
        || !words.matches_source(image, input.lexer_config())
        || words.context() != Some(context.context())
        || !source_cut_declaration_origins(words)
}

pub(crate) fn source_cut_declaration_origins(
    words: &source_structure::OriginalRegistryWords,
) -> bool {
    matches!(
        words.origins().first(),
        Some(InvocationWordOrigin::Written(0) | InvocationWordOrigin::ResolvedHead)
    ) && words
        .origins()
        .iter()
        .enumerate()
        .skip(1)
        .all(|(index, origin)| *origin == InvocationWordOrigin::Written(index))
}

fn source_cut_resolved_head_correspondence(
    source: &str,
    config: tcl_lexer::LexerConfig,
    input: &crate::analyser::ResolvedAnalysisInput,
    realm: &crate::realm::CommandBindingRealm,
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
    words: &source_structure::OriginalRegistryWords,
) -> Option<SourceCutDeclarationWordCorrespondence> {
    let image = source_cut_stage("pair.source-image", realm.original_source_image())?;
    let offset = source_cut_stage("pair.argv", tokens.argv.first())?.start();
    let expected = source_cut_stage(
        "pair.expected-tokens",
        realm.retained_original_tokens_for_command(image, config, offset, tokens),
    )?;
    if source_cut_refuses(
        "pair.binding",
        expected.source_binding != tokens.source_binding,
    ) {
        return None;
    }
    let values = source_cut_stage(
        "pair.primary-declaration-values",
        realm.original_source_procedure_declaration_values(input, context, tokens),
    )?;
    values
        .head_correspondence(source, config, input, words)
        .map(|heads| SourceCutDeclarationWordCorrespondence { heads })
}

pub(crate) fn source_cut_stage<T>(stage: &'static str, value: Option<T>) -> Option<T> {
    source_cut_refuses(stage, value.is_none());
    value
}

pub(crate) fn source_cut_refuses(stage: &'static str, refused: bool) -> bool {
    if refused && source_cut_trace_admitted() {
        eprintln!("SOURCE_CUT_DECLARATION first_refusal={stage}");
    }
    refused
}

/// Observe an already-computed source-load predicate with the existing shared quota.
pub(crate) fn load_trace_flag(stage: &'static str, offset: Option<u32>, value: bool) -> bool {
    if source_cut_trace_admitted() {
        eprintln!("SOURCE_LOAD_ISSUER stage={stage} offset={offset:?} value={value}");
    }
    value
}

/// Preserve the original source-load result and its exact ownership.
pub(crate) fn load_trace_option<T>(
    stage: &'static str,
    offset: Option<u32>,
    value: Option<T>,
) -> Option<T> {
    if source_cut_trace_admitted() {
        eprintln!(
            "SOURCE_LOAD_ISSUER stage={stage} offset={offset:?} present={}",
            value.is_some()
        );
    }
    value
}

pub(crate) fn source_cut_trace_admitted() -> bool {
    static ROWS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    option_env!("TCL_LSP_TRACE_COARSE_SOURCE") == Some("1")
        && ROWS
            .try_update(
                std::sync::atomic::Ordering::Relaxed,
                std::sync::atomic::Ordering::Relaxed,
                |rows| (rows < 64).then_some(rows + 1),
            )
            .is_ok()
}

fn source_cut_declaration_selection(
    tokens: &CommandTokens,
    words: &source_structure::OriginalRegistryWords,
    facts: InvocationFacts,
) -> Option<ResolvedStatementInvocation> {
    if facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Proc,
        )
        || facts.body_kind != tcl_registry::BodyKind::Structural
        || !facts.traits.contains(
            tcl_registry::Traits::DEFERS_BODY | tcl_registry::Traits::BODY_RUNS_IN_OWN_FRAME,
        )
        || !facts.arg_roles_complete
        || facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    let effective = compose_original_effective_words(tokens, words.command(), &[])?;
    let selected = ResolvedStatementInvocation {
        dialect: words.dialect(),
        facts: Box::new(facts),
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        effective,
        evaluated_arguments: words
            .arguments()
            .iter()
            .map(|word| match word {
                EffectiveInvocationWord::Literal(value) => Some(value.clone()),
                EffectiveInvocationWord::ByteLiteral(value) => {
                    std::str::from_utf8(value).ok().map(str::to_owned)
                }
                _ => None,
            })
            .collect(),
        evaluated_words: words.arguments().to_vec(),
        authored_case_body_selection: None,
    };
    (selected.written_argument_roles()
        == vec![
            (0, tcl_registry::ArgRole::Name),
            (1, tcl_registry::ArgRole::ParamList),
            (2, tcl_registry::ArgRole::Body),
        ])
    .then_some(selected)
}

/// Original deferred procedure declaration grammar for source coverage only.
/// This retains the exact authored source lookup and direct static operands;
/// it grants neither body entry nor Native frame or Normal completion.
#[must_use]
pub fn original_authored_procedure_declaration_for_source_coverage(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    if tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        != crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
        || tokens.words().iter().enumerate().any(|(ordinal, word)| {
            !(matches!(
                word,
                WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
            ) || ordinal == 1
                && tokens.source_binding.as_ref().is_some_and(|binding| {
                    binding.original_completed_static_declaration_name(tokens)
                }))
        })
    {
        return None;
    }
    let selected = original_declared_structured_invocation(registry, context, tokens)?;
    original_structured_operands_are_supported(registry, &selected, None, tokens)?;
    if selected.facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Proc,
        )
        || selected.facts.body_kind != tcl_registry::BodyKind::Structural
        || !selected.facts.traits.contains(
            tcl_registry::Traits::DEFERS_BODY | tcl_registry::Traits::BODY_RUNS_IN_OWN_FRAME,
        )
        || selected.written_argument_roles()
            != vec![
                (0, tcl_registry::ArgRole::Name),
                (1, tcl_registry::ArgRole::ParamList),
                (2, tcl_registry::ArgRole::Body),
            ]
    {
        return None;
    }
    Some(selected)
}

/// Original declaration grammar, without a completed handler or CPP receipt.
fn original_declared_structured_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let context = context.into();
    let source_purpose = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?;
    let logical_source =
        source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) {
        trace_original_structure("declaration_registry", tokens, false);
        return None;
    }
    let Some(binding) = tokens.source_binding.as_ref() else {
        trace_original_structure("declaration_binding", tokens, false);
        return None;
    };
    let context = if logical_source {
        binding.original_invocation_metadata_for_input(
            tokens,
            host_structure_observer::option("metadata.input", context.source_analysis_input())?,
            registry,
        )?
    } else {
        context
    };
    if let Some(input) = context.source_analysis_input()
        && host_structure_observer::option(
            "declaration.lexer_config",
            binding.original_lexer_config_for_tokens(tokens),
        )?
        .normalized()
            != input.lexer_config().normalized()
    {
        trace_original_structure("declaration_config", tokens, false);
        return None;
    }
    let advice = if logical_source {
        binding.original_conditional_operand_layout_advice(tokens, registry)
    } else if source_purpose
        == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    {
        binding
            .declaration_operand_layout_advice(tokens)
            .or_else(|| binding.original_compilation_lookup_advice(tokens))
    } else {
        binding
            .declaration_operand_layout_advice(tokens)
            .or_else(|| {
                binding
                    .original_compilation_lookup_advice(tokens)
                    .filter(|advice| !advice.has_unclosed_source_coverage())
            })
    };
    trace_original_structure("declaration_advice", tokens, advice.is_some());
    let advice = advice?;
    // This is conditional source structure. A positively retained Logical
    // command world needs no physical variable frame; Native lookup keeps its
    // independent frame gate and cannot borrow that source-only closure.
    let closed = if logical_source {
        advice.closed_logical_source_lookup()
    } else if source_purpose
        == crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    {
        advice.closed_authored_simulation_source_lookup(tokens, context, registry)
    } else {
        advice.closed_lookup()
    };
    trace_original_structure("declaration_closed", tokens, closed);
    if !closed {
        return None;
    }
    let selected = resolve_original_declared_layout(registry, context, tokens, &advice);
    trace_original_structure("declaration_facts", tokens, selected.is_some());
    selected
}

/// Conditional source structure retained by a genuine Inline Control compiler.
/// This purpose does not repair an unavailable runtime declaration or handler.
fn original_inline_control_structured_invocation(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let source_purpose = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?;
    let logical_source =
        source_purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) || logical_source {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_inline_control_metadata_for_input(
        tokens,
        host_structure_observer::option("metadata.input", context.source_analysis_input())?,
        registry,
    )?;
    let preparation = binding.original_inline_control_compilation(tokens)?;
    let advice = binding.original_compilation_lookup_advice(tokens)?;
    if !advice.closed_lookup()
        || advice.has_opaque_handler_alternatives()
        || advice.namespace() != &preparation.dependency().namespace_key
        || !advice
            .targets()
            .iter()
            .all(|target| target == &preparation.dependency().target)
    {
        return None;
    }
    resolve_original_declared_layout(registry, metadata, tokens, &advice)
}

/// Conditional option topology of the retained original invocation. Captured
/// operands keep their effective ordinals, with no handler or Normal grant.
pub(crate) fn conditional_option_arguments_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<(
    ResolvedStatementInvocation,
    tcl_registry::AuthoredSourceOptionArguments,
)> {
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) || tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    if let Some(input) = context.source_analysis_input()
        && binding
            .original_lexer_config_for_tokens(tokens)?
            .normalized()
            != input.lexer_config().normalized()
    {
        return None;
    }
    let invocation =
        resolved_tokens_invocation_with_metadata_context(registry, Some(context), tokens)
            .or_else(|| {
                resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)
            })
            .or_else(|| original_declared_structured_invocation(registry, context, tokens))?;
    let realm = if logical_source {
        binding.original_source_realm_for_input(
            tokens,
            context.source_analysis_input()?,
            registry,
        )?
    } else {
        binding.invocation_realm().or_else(|| {
            binding
                .declaration_operand_layout_advice(tokens)
                .map(|advice| advice.realm())
        })?
    };
    let options = invocation.with_metadata_schema(registry, context, realm, |schema| {
        schema.authored_source_possible_option_arguments()
    })?;
    Some((invocation, options))
}

/// Conditional operation recipe for the exact supplied Logical source model.
/// Original alias captures and source words remain composed by the shared
/// declaration owner. This grants no Native handler, Normal completion,
/// compiler entry, variable allocation or executable erasure permission.
#[doc(hidden)]
pub fn original_logical_operation_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    original_source_model_operation_invocation_with_metadata_context(
        registry,
        context,
        tokens,
        crate::command_binding::OriginalInvocationSourcePurpose::Logical,
    )
}

/// The original operation invocation of a source read under `purpose`, a
/// software source model (Logical, or a release's authored simulation). It
/// answers source structure only: no Native entry, dispatch or completion.
#[doc(hidden)]
#[must_use]
pub fn original_source_model_operation_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    purpose: crate::command_binding::OriginalInvocationSourcePurpose,
) -> Option<ResolvedStatementInvocation> {
    if !matches!(
        purpose,
        crate::command_binding::OriginalInvocationSourcePurpose::Logical
            | crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation
    ) {
        return None;
    }
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        == purpose;
    if !context.matches_registry(registry) || !logical_source || tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let context = binding.original_invocation_metadata_for_input(
        tokens,
        context.source_analysis_input()?,
        registry,
    )?;
    if !logical_source {
        return None;
    }
    resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)
        .or_else(|| original_declared_structured_invocation(registry, context, tokens))
}

/// Conditional completion of the exact original Native source schema.
/// This supplies analysis topology only, without a reached handler or Normal result.
pub(crate) fn original_native_source_conditional_completion(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<tcl_registry::completion_route::InvocationCompletionRoute> {
    let input = context.source_analysis_input()?;
    if input.has_logical_source_name_context()
        || input.has_hosted_source_name_context()
        || !context.matches_registry(registry)
        || tokens.synthetic.is_some()
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let context = binding.original_invocation_metadata_for_input(tokens, input, registry)?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    if config.normalized() != input.lexer_config().normalized() {
        return None;
    }
    if binding.has_original_native_source_command() {
        let words = binding.original_native_source_words(tokens, input, registry)?;
        let dialect = words.dialect()?;
        let arguments = words
            .arguments()
            .iter()
            .map(|word| {
                word.literal_bytes()
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    .map(tcl_registry::InvocationWord::Literal)
            })
            .collect::<Option<Vec<_>>>()?;
        return words.with_source_schema(input.borrowed_context_registry(), |schema| {
            (schema.facts().arity_accepts_frozen_arguments() == Some(true)).then_some(())?;
            registry.invocation_completion_route(
                schema.descriptor_key().label(),
                tcl_registry::InvocationArguments::structured(&arguments).with_dialect(dialect),
                Some(context.context().authoring_query()),
            )
        })?;
    }
    let advice = binding
        .declaration_operand_layout_advice(tokens)
        .or_else(|| binding.original_compilation_lookup_advice(tokens))?;
    if advice.has_unclosed_source_coverage() {
        return None;
    }
    let selected = resolve_original_source_layout(registry, Some(context), tokens, &advice)?;
    selected.dialect?;
    if selected.facts.arity_accepts_frozen_arguments() != Some(true)
        || selected
            .evaluated_words
            .iter()
            .any(|word| word.literal_bytes().is_none())
    {
        return None;
    }
    Some(selected.completion_route(registry))
}

/// Conditional variable destinations and aliases of the exact Logical source.
/// This is a readonly MAY footprint, without Native entry or completed stores.
pub(crate) fn original_logical_variable_footprint_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<(
    tcl_registry::VariableWriteProjection,
    Vec<tcl_registry::VariableCellAliasTransition>,
)> {
    if context.matches_registry(registry)
        && let Some(words) = tokens
            .source_binding
            .as_ref()?
            .original_logical_source_words(tokens, context.source_analysis_input()?, registry)
    {
        return words.with_source_schema(
            context.source_analysis_input()?.borrowed_context_registry(),
            |schema| {
                Some((
                    CommandRegistry::variable_write_projection_for_selected_source(schema),
                    schema.authored_logical_source_variable_aliases()?,
                ))
            },
        )?;
    }
    let binding = tokens.source_binding.as_ref()?;
    let context = binding.original_invocation_metadata_for_input(
        tokens,
        context.source_analysis_input()?,
        registry,
    )?;
    let selected =
        original_logical_operation_invocation_with_metadata_context(registry, context, tokens)?;
    let realm = binding.original_source_realm_for_input(
        tokens,
        context.source_analysis_input()?,
        registry,
    )?;
    selected.with_metadata_schema(registry, context, realm, |schema| {
        let aliases = schema.authored_logical_source_variable_aliases()?;
        Some((
            CommandRegistry::variable_write_projection_for_selected_source(schema),
            aliases,
        ))
    })
}

/// Purity of the selected original substitution implementation. Native
/// dispatch requires a normal handler and closed execution alternatives;
/// the separate Logical branch retains its exact source operation recipe.
/// Neither branch proves operand evaluation, no-raise, observers or erasure.
pub(crate) fn original_substitution_purity_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<bool> {
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    if !context.matches_registry(registry) || tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let input = context.source_analysis_input()?;
    if config.normalized() != input.lexer_config().normalized() {
        return None;
    }
    let invocation = if logical_source {
        original_logical_operation_invocation_with_metadata_context(registry, context, tokens)?
    } else {
        if binding.execution_is_unknown() || binding.execution_may_be_absent() {
            return None;
        }
        resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)?
    };
    if invocation.facts.arity_accepts_frozen_arguments() != Some(true)
        || matches!(
            invocation.facts.subcommand,
            tcl_registry::OwnedSubcommandResolution::Unknown { .. }
                | tcl_registry::OwnedSubcommandResolution::Ambiguous { .. }
                | tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
        )
    {
        return None;
    }
    Some(!invocation.facts.mutator && invocation.facts.evaluates_purely())
}

/// The source-role consumer chooses its purpose-specific closure before this
/// shared full-vector/schema resolution. This helper supplies no completion.
fn resolve_original_declared_layout<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<ResolvedStatementInvocation> {
    resolve_original_source_layout(registry, Some(context.into()), tokens, advice)
}

fn resolve_original_source_layout(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<ResolvedStatementInvocation> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    let dialect = advice.dialect();
    let mut unanimous = None;
    for target in advice.targets() {
        if !target.registry_backed {
            return None;
        }
        if !target.prepended.is_empty()
            && context.is_some_and(
                crate::registry_invocation::InvocationMetadataContext::permits_logical_source_names,
            )
        {
            let context = context?;
            let purpose = tokens
                .source_binding
                .as_ref()?
                .original_source_advice_purpose(tokens, context, registry)?;
            if purpose == crate::command_binding::OriginalInvocationSourcePurpose::Logical {
                original_logical_prefix_for_tokens(
                    tokens,
                    target,
                    context.source_analysis_input()?,
                )?;
            }
        }
        let effective =
            compose_original_effective_words(tokens, &target.command, &target.prepended)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let (resolution, authored_case_body_selection) =
            resolve_registry_words_in_realm_with_metadata_context_and_case(
                registry,
                context,
                &words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?;
        let RegistryInvocationResolution::Resolved(facts) = resolution else {
            return None;
        };
        let evaluated_words = values.get(1..)?.to_vec();
        let selected = ResolvedStatementInvocation {
            dialect: Some(dialect),
            facts,
            arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
            evaluated_arguments: evaluated_words
                .iter()
                .map(|word| match word {
                    EffectiveInvocationWord::Literal(value) => Some(value.clone()),
                    _ => None,
                })
                .collect(),
            evaluated_words,
            effective,
            authored_case_body_selection: Some(authored_case_body_selection),
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return None;
        }
        unanimous = Some(selected);
    }
    unanimous
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

impl AdmittedNativeCompilerInvocation {
    /// Inert logical member selectors needed by a shared backend hook.
    /// Original operands follow in their original order and retain their source.
    #[must_use]
    pub fn hook_operand_prefix(&self) -> &'static [&'static str] {
        self.hook_operand_prefix
    }
    /// Actual original compiler registration, with its slot/token identity.
    #[must_use]
    pub fn target(&self) -> &crate::command_binding::SourceCommandTarget {
        &self.target
    }
    /// Actual registration name, independent of logical ensemble metadata.
    #[must_use]
    pub fn canonical_registration_name(&self) -> &str {
        &self.target.command
    }
    /// Original effective words and their written/prefix origins.
    #[must_use]
    pub fn effective_words(&self) -> &EffectiveCommandWords {
        &self.invocation.effective
    }
    /// Number of original operands before the selected native member layout.
    #[must_use]
    pub fn argument_offset(&self) -> usize {
        self.invocation.facts.argument_offset
    }
    /// Registry-authored native compiler grammar.
    #[must_use]
    pub fn native_compilation(
        &self,
    ) -> Option<tcl_registry::native_compilation::NativeCompilationSpec> {
        self.invocation.facts.native_compilation
    }
    /// Compiler-owned error annotation for an inline body operand.
    #[must_use]
    pub fn compiler_operand_error_context(&self) -> Option<tcl_registry::InlineBodyErrorContext> {
        self.invocation.facts.operation.inline_body_error_context()
    }
    /// Logical registry command used only for compiler metadata selection.
    #[must_use]
    pub fn canonical_logical_command(&self) -> &str {
        &self.invocation.facts.canonical_command
    }

    /// Exact metadata key retained by the already admitted original compiler.
    /// The key supplies no additional handler, frame, completion or admission.
    #[must_use]
    pub fn descriptor_key(&self) -> tcl_registry::CanonicalCommandDescriptorKey<'_> {
        self.invocation.facts.descriptor_key()
    }
    /// Compiler metadata from the already admitted original descriptor.
    /// This does not establish successful-handler effects.
    #[must_use]
    pub fn traits(&self) -> tcl_registry::Traits {
        self.invocation.facts.traits
    }
    /// Statement emitter metadata; admission remains an independent premise.
    #[must_use]
    pub fn codegen_hook(&self) -> Option<tcl_registry::hooks::CodegenHookId> {
        self.hooks.codegen
    }
    /// Value emitter metadata; admission remains an independent premise.
    #[must_use]
    pub fn inline_codegen_hook(&self) -> Option<tcl_registry::hooks::InlineCodegenHookId> {
        self.hooks.inline
    }
    /// Structured lowering metadata; no successful-handler facts accompany it.
    #[must_use]
    pub fn lowering_hook(&self) -> Option<tcl_registry::hooks::LoweringHookId> {
        self.hooks.lowering
    }
    /// Decode original source words under their retained logical value rules,
    /// then query compiler recipes under the independent physical engine.
    pub fn with_argument_words<T>(&self, apply: impl FnOnce(InvocationWords<'_>) -> T) -> T {
        let dialect = self.invocation.dialect;
        let effective = self
            .invocation
            .effective
            .words
            .iter()
            .map(|word| {
                dialect.map_or(EffectiveInvocationWord::Dynamic, |dialect| {
                    effective_invocation_word(
                        word,
                        dialect.lexer_grammar.escapes,
                        dialect.word_values,
                    )
                })
            })
            .collect::<Vec<_>>();
        let values = effective
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect::<Vec<_>>();
        apply(
            InvocationWords::structured(values[0], &values[1..])
                .with_dialect(self.compiler_dialect),
        )
    }
    /// Exact compiler environment of an original selected body operand.
    #[must_use]
    pub fn body_context(
        &self,
        enclosing: tcl_registry::native_compilation::NativeCompilationContext,
        operand: usize,
    ) -> Option<tcl_registry::native_compilation::NativeCompilationContext> {
        let spec = self.native_compilation()?;
        let word = self.effective_words().words.get(operand.checked_add(1)?)?;
        Some(self.with_argument_words(|words| {
            spec.body_context_for_invocation_operand(
                enclosing,
                self.selection,
                native_compilation_word_shape(word),
                words.arguments(),
                operand,
            )
        }))
    }
}

/// Resolve only an admitted original compiler target, even if its later guard
/// or runtime handler is uncertain. This carrier cannot license normal transfer.
#[must_use]
pub fn admitted_native_compiler_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<AdmittedNativeCompilerInvocation> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let target = binding
        .admitted_inline_invocation()
        .map(|proof| &proof.target)
        .or_else(|| {
            binding
                .admitted_named_invocation()?
                .dependencies
                .first()
                .map(|dependency| &dependency.target)
        })?
        .clone();
    if !target.registry_backed {
        return None;
    }
    let effective = compose_original_effective_words(tokens, &target.command, &target.prepended)?;
    let dialect = binding.native_compiler_dialect()?;
    let word_dialect = binding.compiler_word_dialect()?;
    let values = effective
        .words
        .iter()
        .map(|word| {
            effective_invocation_word(
                word,
                word_dialect.lexer_grammar.escapes,
                word_dialect.word_values,
            )
        })
        .collect::<Vec<_>>();
    let words = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect::<Vec<_>>();
    let RegistryInvocationResolution::Resolved(facts) = resolve_registry_words_in_realm(
        registry,
        context,
        &words,
        Some(dialect),
        binding.compiler_invocation_realm()?,
    )
    .ok()?
    else {
        return None;
    };
    let hooks = registry.native_compiler_hooks(
        InvocationWords::structured(words[0], &words[1..]).with_dialect(dialect),
    )?;
    let hook_operand_prefix = registry
        .native_registration_operand_prefix(&target.command, dialect)
        .unwrap_or(&[]);
    Some(AdmittedNativeCompilerInvocation {
        compiler_dialect: dialect,
        invocation: ResolvedStatementInvocation {
            dialect: Some(word_dialect),
            arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
            evaluated_arguments: frozen_argument_values(tokens, &effective),
            evaluated_words: frozen_argument_words(tokens, &effective),
            effective,
            facts,
            authored_case_body_selection: None,
        },
        hooks,
        target,
        selection: binding.native_compilation_admission_selection(),
        hook_operand_prefix,
    })
}

/// Separate original compiler registration requirements from Logical metadata.
#[derive(Clone, Copy)]
pub(crate) enum SelectedLoweringBindingIdentity<'a> {
    Native(&'a AdmittedNativeCompilerInvocation),
    Logical(tcl_registry::CanonicalCommandDescriptorKey<'a>),
    HostedSchema(tcl_registry::CanonicalCommandDescriptorKey<'a>),
    NativeSourceSchema(tcl_registry::CanonicalCommandDescriptorKey<'a>),
    LogicalSourceSchema(tcl_registry::CanonicalCommandDescriptorKey<'a>),
    AuthoredSourceSchema(tcl_registry::CanonicalCommandDescriptorKey<'a>),
    AuthoredSimulationSourceSchema(tcl_registry::CanonicalCommandDescriptorKey<'a>),
}

impl<'a> SelectedLoweringBindingIdentity<'a> {
    pub(crate) fn descriptor_key(self) -> tcl_registry::CanonicalCommandDescriptorKey<'a> {
        match self {
            Self::Native(invocation) => invocation.descriptor_key(),
            Self::Logical(key)
            | Self::HostedSchema(key)
            | Self::NativeSourceSchema(key)
            | Self::LogicalSourceSchema(key)
            | Self::AuthoredSimulationSourceSchema(key)
            | Self::AuthoredSourceSchema(key) => key,
        }
    }

    pub(crate) fn binding_identity(self) -> &'a str {
        let name = match self {
            Self::Native(invocation) => invocation.canonical_registration_name(),
            Self::Logical(key)
            | Self::HostedSchema(key)
            | Self::NativeSourceSchema(key)
            | Self::LogicalSourceSchema(key)
            | Self::AuthoredSimulationSourceSchema(key)
            | Self::AuthoredSourceSchema(key) => key.label(),
        };
        tcl_syntax::naming::unroot_rooted_key(name).unwrap_or(name)
    }
}

/// A reached handler's normal variable transfer, independent of compiler-hook
/// selection. This carrier cannot license an opcode, body evaluation or command
/// completion: its registry facts remain private to the variable transfer owner.
#[derive(Debug)]
pub struct NormalTransferInvocation {
    invocation: ResolvedStatementInvocation,
    compilation: tcl_registry::native_compilation::NativeCompilationSelection,
    alias_frame: tcl_registry::VariableAliasFrame,
    captured_outputs: bool,
    original_operands: Option<std::sync::Arc<crate::variable_bindings::OriginalVariableInvocation>>,
}

/// Element evolution of one proved normal, unobserved container store.
pub struct NormalContainerElementWrite<'a> {
    /// Authored element evolution on the normal store continuation.
    pub effect: tcl_registry::VarElementsEffect,
    /// Actual literal variable operand bound to the proved unobserved cell.
    pub target: String,
    /// Effective argv offset introduced by a selected subcommand.
    pub argument_offset: usize,
    /// Retained operand presentation for source-aware value typing. This is
    /// not evidence of evaluated key bytes.
    pub arguments: Vec<String>,
    frozen_dictionary_keys: Option<Vec<String>>,
    /// Effective source words for representation-aware value typing.
    pub words: &'a [WordExpr],
}

impl NormalContainerElementWrite<'_> {
    /// Actual dictionary update key path frozen after argument evaluation.
    /// Unknown evaluated keys cannot borrow their source presentation.
    #[must_use]
    pub fn dictionary_keys(&self) -> Option<&[String]> {
        (self.effect == tcl_registry::VarElementsEffect::SetsDictValue)
            .then_some(self.frozen_dictionary_keys.as_deref())
            .flatten()
    }
}

impl NormalTransferInvocation {
    /// Alias declarations active in the proved frame on the normal continuation.
    /// Native no-op declarations and unproved activation policies are excluded.
    /// This exposes no compiler selection, body execution, or completion facts.
    pub fn variable_alias_transitions(
        &self,
    ) -> impl Iterator<Item = &tcl_registry::state_transition::VariableCellAliasTransition> {
        self.invocation
            .facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
            .filter_map(|fact| match &fact.transition {
                tcl_registry::state_transition::StateTransition::VariableCellAlias(alias)
                    if !self.captured_outputs
                        && self.invocation.dialect.and_then(|dialect| {
                            alias
                                .destination
                                .is_active_in_frame(self.alias_frame, dialect)
                        }) == Some(true) =>
                {
                    Some(alias)
                }
                _ => None,
            })
    }

    /// Decode an alias frame selector under this invocation's retained policies.
    /// Missing selector values or execution dialects retain unknown selection.
    #[must_use]
    pub fn variable_alias_frame_level(
        &self,
        frame: &tcl_registry::CallerFrameSelection,
    ) -> Option<tcl_registry::FrameLevel> {
        match frame {
            tcl_registry::CallerFrameSelection::DefaultCaller => {
                Some(tcl_registry::FrameLevel::DEFAULT)
            }
            tcl_registry::CallerFrameSelection::Explicit(level) => {
                tcl_registry::FrameLevel::parse_for_dialect(
                    level.literal()?,
                    self.invocation.dialect?,
                )
            }
        }
    }

    /// Variable trace-table transitions on the reached normal registration path.
    /// Captured output boundaries cannot register or remove an observer.
    pub fn variable_trace_transitions(
        &self,
    ) -> impl Iterator<Item = &tcl_registry::TraceTransition> {
        self.invocation
            .facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
            .filter_map(|fact| match &fact.transition {
                tcl_registry::StateTransition::Trace(trace)
                    if !self.captured_outputs
                        && matches!(
                            trace,
                            tcl_registry::TraceTransition::Add {
                                target: tcl_registry::TraceTarget::Variable(_),
                                ..
                            } | tcl_registry::TraceTransition::Remove {
                                target: tcl_registry::TraceTarget::Variable(_),
                                ..
                            }
                        ) =>
                {
                    Some(trace)
                }
                _ => None,
            })
    }

    /// Select container value effects only after proving the actual store cell.
    /// This projection carries no compiler, callback-body or completion licence.
    #[must_use]
    pub fn container_element_write(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<NormalContainerElementWrite<'_>> {
        let effect = self.invocation.facts.var_elements_effect?;
        let places = self.mutation_places(state, registry);
        let [place] = places.as_slice() else {
            return None;
        };
        if place.observed || place.dynamic || place.kind == crate::place::PlaceKind::Unknown {
            return None;
        }
        let argument_offset = self.invocation.facts.argument_offset;
        Some(NormalContainerElementWrite {
            effect,
            target: self.invocation.argument_literal(argument_offset)?,
            argument_offset,
            arguments: self
                .invocation
                .arguments
                .iter()
                .cloned()
                .collect::<Option<Vec<_>>>()?,
            frozen_dictionary_keys: (argument_offset + 1
                ..self.invocation.arguments.len().checked_sub(1)?)
                .map(|index| self.invocation.argument_literal(index))
                .collect(),
            words: self.invocation.effective.words.get(1..)?,
        })
    }

    /// Select variable addresses at the phase authored by the reached handler.
    #[must_use]
    pub fn variable_binding_phase(
        &self,
    ) -> tcl_registry::native_compilation::VariableOperandBindingPhase {
        self.invocation.facts.successful_handler.map_or(
            tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
            tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
        )
    }

    /// Authored output-address lookup protocol, without exposing compiler facts.
    #[must_use]
    pub fn variable_output_lookup(
        &self,
    ) -> Option<tcl_registry::native_compilation::VariableOutputLookup> {
        self.invocation
            .facts
            .successful_handler
            .map(tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_output_lookup)
    }

    /// Variable output indices in their observable native write order.
    /// Missing ordering evidence remains unknown rather than choosing an order.
    #[must_use]
    pub fn variable_output_arguments(&self) -> Option<Vec<usize>> {
        use tcl_registry::native_compilation::VariableOutputLookup;
        let writes = || {
            self.variable_roles()
                .into_iter()
                .filter_map(|(index, role)| {
                    (role == tcl_registry::ArgRole::VarWrite).then_some(index)
                })
                .collect::<Vec<_>>()
        };
        match self.variable_output_lookup() {
            Some(VariableOutputLookup::CatchSelected) => self.with_arguments(|arguments| {
                let dialect = self.invocation.dialect?;
                let tcl_registry::catch_invocation::CatchInvocationSelection::Valid(capture) =
                    tcl_registry::catch_invocation::select_catch_invocation(arguments, dialect)
                else {
                    return None;
                };
                Some(
                    capture
                        .output_order(dialect, self.compilation)
                        .indices(capture)?
                        .into_iter()
                        .flatten()
                        .collect(),
                )
            }),
            Some(VariableOutputLookup::Sequential | VariableOutputLookup::SingleTarget) => {
                Some(writes())
            }
            Some(VariableOutputLookup::BodyProtocol) => None,
            None => {
                let writes = writes();
                (writes.len() <= 1).then_some(writes)
            }
        }
    }

    /// Locate a setter's retained value operand on the normal handler path.
    /// An operand dependency cannot select a compiler operation or opcode.
    #[must_use]
    pub fn stored_value_argument(&self) -> Option<usize> {
        (self.invocation.facts.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            )
            && self.argument_count() == 2)
            .then_some(1)
    }

    /// Selected normal numeric store protocol, without a concrete value or
    /// permission to omit its operand conversion or observer schedule.
    #[must_use]
    pub fn numeric_store_production(
        &self,
    ) -> Option<tcl_registry::native_result::NativeNumericStoreProduction> {
        let contract = self.invocation.facts.native_result?;
        self.with_arguments(|arguments| {
            contract
                .normal_numeric_store_production(arguments, self.invocation.facts.argument_offset)
        })
    }

    /// Variable-access traits; these do not select script bodies or compile hooks.
    #[must_use]
    pub fn variable_traits(&self) -> tcl_registry::Traits {
        self.invocation.facts.traits.intersection(
            tcl_registry::Traits::READS_BEFORE_WRITE
                | tcl_registry::Traits::DESTROYS_VARIABLE
                | tcl_registry::Traits::CONDITIONAL_VARIABLE_WRITE
                | tcl_registry::Traits::WHOLE_ARRAY_ARG,
        )
    }

    /// Locate an original value word without reconstructing frozen argv.
    #[must_use]
    pub fn written_argument(&self, argument: usize) -> Option<usize> {
        self.invocation.effective.written_argument(argument)
    }

    /// Number of effective arguments on this reached handler continuation.
    #[must_use]
    pub fn argument_count(&self) -> usize {
        self.invocation.arguments.len()
    }

    /// Named contents reads on this handler phase, independently of alias registration.
    #[must_use]
    pub fn read_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<crate::place::Place> {
        if self.captured_outputs
            || self.variable_binding_phase()
                == tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
        {
            return Vec::new();
        }
        self.with_arguments(|arguments| {
            if state.execution_name_policy.is_some() {
                return self.original_operands.as_deref().map_or_else(
                    || vec![crate::place::unknown_top()],
                    |operands| crate::variable_bindings::source_variable_read_places_with_original_operands(
                        &self.invocation.facts, arguments, state, registry, operands,
                    ),
                );
            }
            crate::variable_bindings::source_variable_read_places(
                &self.invocation.facts, arguments, state, registry,
            )
        })
    }

    /// Bound contents changes, including destruction; no runtime dispatch licence.
    #[must_use]
    pub fn mutation_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<crate::place::Place> {
        if self.captured_outputs {
            return self.definition_places(state, registry);
        }
        self.with_arguments(|arguments| {
            if state.execution_name_policy.is_some() {
                return self.original_operands.as_deref().map_or_else(
                    || vec![crate::place::unknown_top()],
                    |operands| crate::variable_bindings::source_variable_write_places_with_output_order_and_original_operands(
                        &self.invocation.facts, arguments, state, registry,
                        self.variable_output_arguments().as_deref(), operands,
                    ),
                );
            }
            crate::variable_bindings::source_variable_write_places_with_output_order(
                &self.invocation.facts, arguments, state, registry,
                self.variable_output_arguments().as_deref(),
            )
        })
    }

    /// Bound SSA value definitions on the successful continuation.
    #[must_use]
    pub fn definition_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<crate::place::Place> {
        if self.variable_binding_phase()
            == tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
        {
            return Vec::new();
        }
        self.definition_operands(state, registry)
            .into_iter()
            .map(|(_, place)| place)
            .collect()
    }

    /// Original written output arguments paired with their selected normal
    /// definition places. Inserted and expanded operands have no written slot.
    /// Missing address/order evidence remains unknown in the returned place.
    #[must_use]
    pub fn written_definition_places(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<(usize, crate::place::Place)> {
        if self.variable_binding_phase()
            == tcl_registry::native_compilation::VariableOperandBindingPhase::BodyProtocol
        {
            return Vec::new();
        }
        self.definition_operands(state, registry)
            .into_iter()
            .filter_map(|(effective, place)| Some((self.written_argument(effective)?, place)))
            .collect()
    }

    fn definition_operands(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Vec<(usize, crate::place::Place)> {
        if state.execution_name_policy.is_some() {
            let Some(operands) = self
                .original_operands
                .as_deref()
                .filter(|operands| operands.argument_count() == self.argument_count())
            else {
                return Vec::new();
            };
            return self.with_arguments(|arguments| {
                crate::variable_bindings::source_variable_definitions_with_original_operands(
                    &self.invocation.facts,
                    arguments,
                    state,
                    registry,
                    self.variable_output_arguments().as_deref(),
                    operands,
                )
            });
        }
        crate::place_bridge::resolved_invocation_variable_definitions(
            &self.invocation,
            state,
            registry,
            self.variable_output_arguments().as_deref(),
        )
    }

    /// Apply only the variable owner's normal continuation effects.
    pub fn transfer_variables(
        &self,
        state: &mut crate::var_resolve::ResolveContext,
        statement: &crate::ir::Statement,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) {
        if self.captured_outputs {
            let outputs = self.definition_places(state, registry);
            crate::variable_bindings::transfer_captured_variable_outputs(
                state,
                &outputs,
                statement.span().start(),
            );
            return;
        }
        self.with_arguments(|arguments| {
            crate::variable_bindings::transfer_resolved_invocation(
                state,
                statement,
                tokens,
                &self.invocation.facts,
                arguments,
                registry,
                (
                    self.variable_output_arguments().as_deref(),
                    self.original_operands.as_deref(),
                ),
            );
        });
    }

    /// A passthrough store value only when the selected cell has no write
    /// observer and its physical store is bounded. This says nothing about
    /// executing the invocation or eliminating its possible compiler error.
    ///
    /// `state` must describe the actual address-selection phase returned by
    /// `variable_binding_phase`, including all preceding argv/body effects.
    /// A pre-argv or freshly reconstructed context cannot establish this store.
    #[must_use]
    pub fn stored_value_operand(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<(&str, &WordExpr)> {
        let word = self.stored_value_word(state, registry)?;
        Some((self.invocation.arguments.get(1)?.as_deref()?, word))
    }

    /// Original stored word after proving an unobserved bounded normal store.
    /// Unknown evaluated bytes retain their structured source and read identity.
    #[must_use]
    pub fn stored_value_word(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<&WordExpr> {
        self.stored_value_argument()?;
        let places = self.mutation_places(state, registry);
        let [place] = places.as_slice() else {
            return None;
        };
        if place.observed || place.dynamic || place.kind == crate::place::PlaceKind::Unknown {
            return None;
        }
        self.invocation.effective.words.get(2)
    }

    /// The value frozen before later argv substitutions, after proving the
    /// same observer-free physical store as `stored_value_operand`. Its actual
    /// address-selection context precondition applies here as well.
    #[must_use]
    pub fn stored_value_literal(
        &self,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<String> {
        self.stored_value_operand(state, registry)?;
        self.invocation.argument_literal(1)
    }

    /// Named variable operands, limited to storage access roles.
    #[must_use]
    pub fn variable_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.invocation
            .facts
            .arg_roles
            .iter()
            .filter_map(|&(index, role)| {
                matches!(
                    role,
                    tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                )
                .then_some((
                    self.invocation.facts.argument_offset + usize::from(index),
                    role,
                ))
            })
            .collect()
    }

    /// Select one named storage operand from this retained normal invocation.
    /// Original execution uses its exact byte/compiler descriptor; the label
    /// returned by `argument_literal` is never an address producer.
    pub(crate) fn variable_operand_place(
        &self,
        argument: usize,
        state: &crate::var_resolve::ResolveContext,
        registry: &CommandRegistry,
    ) -> Option<crate::place::Place> {
        let role = self
            .variable_roles()
            .into_iter()
            .find_map(|(index, role)| (index == argument).then_some(role))?;
        self.with_arguments(|arguments| {
            if role == tcl_registry::ArgRole::VarWrite {
                return if state.execution_name_policy.is_some() {
                    self.original_operands.as_deref().map(|operands| {
                        crate::variable_bindings::variable_output_operand_access_with_original_operands(
                            &self.invocation.facts, arguments, argument, state, registry, operands,
                        )
                    })
                } else {
                    Some(crate::variable_bindings::variable_output_operand_access(
                        &self.invocation.facts, arguments, argument, state, registry,
                    ))
                };
            }
            if state.execution_name_policy.is_some() {
                return self.original_operands.as_deref()
                    .filter(|operands| operands.argument_count() == arguments.len())
                    .map(|operands| operands.access(argument, state, registry,
                        tcl_registry::TraceOperation::Read,
                        self.variable_traits().contains(tcl_registry::Traits::WHOLE_ARRAY_ARG)));
            }
            Some(crate::variable_bindings::variable_operand_access(
                arguments, argument, state, registry, tcl_registry::TraceOperation::Read,
                self.variable_traits().contains(tcl_registry::Traits::WHOLE_ARRAY_ARG),
            ))
        })
    }

    /// One variable operand's frozen name shape. A known array root remains
    /// bounded even when its element index is computed; source spelling and
    /// mathematical contents facts cannot replace this actual argv projection.
    #[must_use]
    pub fn variable_operand_word(&self, index: usize) -> Option<EffectiveInvocationWord> {
        self.variable_roles()
            .iter()
            .any(|&(variable_index, _)| variable_index == index)
            .then(|| self.invocation.argument_word(index))
    }

    /// A frozen variable/value operand on this normal transfer boundary.
    #[must_use]
    pub fn argument_literal(&self, index: usize) -> Option<String> {
        self.invocation.argument_literal(index)
    }

    /// Source names receiving contents on the successful continuation.
    #[must_use]
    pub fn definition_names(&self) -> Vec<String> {
        self.variable_roles()
            .into_iter()
            .filter_map(|(index, role)| {
                if role != tcl_registry::ArgRole::VarWrite {
                    return None;
                }
                let name = self.argument_literal(index)?;
                crate::variable_bindings::contents_write_operand(
                    &self.invocation.facts,
                    Some(&name),
                )
                .then_some(name)
            })
            .collect()
    }

    fn with_arguments<T>(
        &self,
        apply: impl FnOnce(tcl_registry::InvocationArguments<'_>) -> T,
    ) -> T {
        let values: Vec<_> = (0..self.argument_count())
            .map(|index| self.invocation.argument_word(index))
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let mut arguments = tcl_registry::InvocationArguments::structured(&words);
        if let Some(dialect) = self.invocation.dialect {
            arguments = arguments.with_dialect(dialect);
        }
        apply(arguments)
    }
}

/// Resolve a normal handler transfer. Strict executable resolution remains the
/// first choice. An unknown compiler protocol additionally needs an exact stable
/// handler and an authored normal-effect equivalence for its frozen argv/frame.
#[must_use]
pub fn normal_transfer_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalTransferInvocation> {
    normal_transfer_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Resolve retained normal transfer metadata under the complete supplied availability generation.
/// Existing original identity, argument and handler premises remain required.
#[must_use]
pub fn normal_transfer_invocation_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<NormalTransferInvocation> {
    normal_transfer_invocation_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

/// Select successful transfer metadata with the complete supplied source owner.
/// Missing or foreign input cannot reconstruct a standalone catalogue context.
#[must_use]
pub fn normal_transfer_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalTransferInvocation> {
    if tokens.synthetic == Some(crate::ir::SyntheticMarker::CapturedCatchOutputs) {
        let mut original = tokens.clone();
        original.synthetic = None;
        let mut normal =
            normal_transfer_invocation_with_metadata_context(registry, context, &original)?;
        normal.captured_outputs = true;
        return Some(normal);
    }
    if let Some(invocation) =
        resolved_tokens_invocation_with_metadata_context(registry, context, tokens)
    {
        return Some(NormalTransferInvocation {
            invocation,
            compilation: normal_output_compilation(tokens),
            alias_frame: normal_alias_frame(tokens),
            captured_outputs: false,
            original_operands: tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.original_variable_operands_for_tokens(tokens))
                .cloned(),
        });
    }
    let binding = tokens.source_binding.as_ref()?;
    let invocation = resolved_handler_invocation_with_metadata_context(registry, context, tokens)?;
    let normal = NormalTransferInvocation {
        invocation,
        compilation: normal_output_compilation(tokens),
        alias_frame: normal_alias_frame(tokens),
        captured_outputs: false,
        original_operands: binding
            .original_variable_operands_for_tokens(tokens)
            .cloned(),
    };
    normal.with_arguments(|arguments| {
        normal
            .invocation
            .facts
            .successful_handler_effects(arguments, binding.variable_context.alias_frame())
    })?;
    Some(normal)
}

/// Namespace import/export directives of one converged handler and frozen argv.
/// This metadata supplies neither a physical mutation nor an execution,
/// compiler, or successful-import proof.
#[derive(Debug)]
pub struct NamespaceDirectiveFootprint {
    transitions: Vec<tcl_registry::NamespaceTransition>,
}

impl NamespaceDirectiveFootprint {
    /// Authored directives, retaining unknown operand values and namespace axes.
    pub fn transitions(&self) -> impl Iterator<Item = &tcl_registry::NamespaceTransition> {
        self.transitions.iter()
    }
}

/// Recover directive metadata independently of native opcode eligibility.
/// A replaced or uncertain handler cannot donate its written command grammar.
#[must_use]
pub fn namespace_directive_footprint(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NamespaceDirectiveFootprint> {
    namespace_directive_footprint_for_invocation(&resolved_handler_invocation(
        registry, context, tokens,
    )?)
}

/// Namespace directive candidates under the actual supplied metadata owner.
/// Missing or foreign availability refuses; this grants no namespace effect.
pub(crate) fn namespace_directive_footprint_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NamespaceDirectiveFootprint> {
    let context = context.filter(|context| context.matches_registry(registry))?;
    namespace_directive_footprint_for_invocation(
        &resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)?,
    )
}

fn namespace_directive_footprint_for_invocation(
    invocation: &ResolvedStatementInvocation,
) -> Option<NamespaceDirectiveFootprint> {
    if invocation.facts.arity_accepts_frozen_arguments() != Some(true) {
        return None;
    }
    let transitions = invocation
        .facts
        .state_transitions
        .declared()?
        .facts()
        .iter()
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::Namespace(
                transition @ (tcl_registry::NamespaceTransition::Import { .. }
                | tcl_registry::NamespaceTransition::Export { .. }),
            ) => Some(transition.clone()),
            _ => None,
        })
        .collect();
    Some(NamespaceDirectiveFootprint { transitions })
}

/// Possible alias declarations of retained handler candidates. This footprint
/// records exposure hazards, never a successful declaration or physical store.
#[derive(Debug)]
pub struct PossibleVariableAliasTransitions {
    aliases: Vec<tcl_registry::state_transition::VariableCellAliasTransition>,
    unknown_residual: bool,
}

impl PossibleVariableAliasTransitions {
    /// Candidate declarations whose activation policy does not rule them out.
    pub fn aliases(
        &self,
    ) -> impl Iterator<Item = &tcl_registry::state_transition::VariableCellAliasTransition> {
        self.aliases.iter()
    }

    /// Additional runtime handlers or operand layouts remain unenumerated.
    #[must_use]
    pub const fn unknown_residual(&self) -> bool {
        self.unknown_residual
    }
}

/// Project possible outward alias exposure from exact source candidate tokens.
/// Unknown receiver namespaces retain their lookup residual; catalogue names
/// cannot create candidates, and compiler selection grants no declaration edge.
#[must_use]
pub fn possible_variable_alias_transitions(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableAliasTransitions> {
    possible_variable_alias_transitions_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Candidate alias declarations under complete supplied source availability.
/// This grants exposure advice, never a completed link or physical cell.
#[must_use]
pub fn possible_variable_alias_transitions_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<PossibleVariableAliasTransitions> {
    possible_variable_alias_transitions_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn possible_variable_alias_transitions_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableAliasTransitions> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let dialect = binding.variable_context.invocation_dialect;
    let mut footprint = PossibleVariableAliasTransitions {
        aliases: Vec::new(),
        unknown_residual: binding.execution_is_unknown() || binding.execution_may_be_absent(),
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            footprint.unknown_residual = true;
            continue;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let values = frozen_argument_words(tokens, &effective);
        let mut words = vec![InvocationWord::Literal(&target.command)];
        words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                context,
                &words,
                dialect,
                binding.invocation_realm().unwrap_or_default(),
            )
        else {
            footprint.unknown_residual = true;
            continue;
        };
        for fact in facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
        {
            let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
                continue;
            };
            if alias
                .destination
                .is_active_in_frame_with_policy(binding.variable_context.alias_frame(), dialect)
                != Some(false)
                && !footprint.aliases.contains(alias)
            {
                footprint.aliases.push(alias.clone());
            }
        }
    }
    Some(footprint)
}

/// Possible variable-observer registrations from retained handler candidates.
/// This is a May exposure footprint, never a completed trace-table mutation.
#[derive(Debug)]
pub struct PossibleVariableTraceTransitions {
    traces: Vec<tcl_registry::TraceTransition>,
    unknown_residual: bool,
    metadata_unavailable: bool,
}

impl PossibleVariableTraceTransitions {
    /// Candidate observer transitions with their frozen variable operands.
    pub fn transitions(&self) -> impl Iterator<Item = &tcl_registry::TraceTransition> {
        self.traces.iter()
    }

    /// Additional handlers or operand layouts remain unenumerated.
    #[must_use]
    pub const fn unknown_residual(&self) -> bool {
        self.unknown_residual
    }

    /// An original candidate has no descriptor in the supplied generation.
    /// The observer inventory cannot be closed from a catalogue fallback.
    #[must_use]
    pub const fn metadata_unavailable(&self) -> bool {
        self.metadata_unavailable
    }
}

/// Retain observer hazards even when receiver lookup cannot prove one handler.
/// Only original candidate identities can supply this metadata.
#[must_use]
pub fn possible_variable_trace_transitions(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableTraceTransitions> {
    possible_variable_trace_transitions_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// May observer transitions under the complete supplied availability generation.
/// This does not establish a completed registration or physical observer table.
#[must_use]
pub fn possible_variable_trace_transitions_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<PossibleVariableTraceTransitions> {
    possible_variable_trace_transitions_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn possible_variable_trace_transitions_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableTraceTransitions> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let mut footprint = PossibleVariableTraceTransitions {
        traces: Vec::new(),
        unknown_residual: binding.execution_is_unknown() || binding.execution_may_be_absent(),
        metadata_unavailable: false,
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            footprint.unknown_residual = true;
            continue;
        }
        let realm = binding.invocation_realm().unwrap_or_default();
        if context.is_some_and(|context| {
            context
                .context()
                .resolve_spec_in_realm(registry, &target.command, realm)
                .is_none()
        }) {
            footprint.metadata_unavailable = true;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let values = frozen_argument_words(tokens, &effective);
        let mut words = vec![InvocationWord::Literal(&target.command)];
        words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                context,
                &words,
                binding.variable_context.invocation_dialect,
                binding.invocation_realm().unwrap_or_default(),
            )
        else {
            footprint.unknown_residual = true;
            continue;
        };
        for fact in facts
            .state_transitions
            .declared()
            .into_iter()
            .flat_map(tcl_registry::StateTransitions::facts)
        {
            let tcl_registry::StateTransition::Trace(trace) = &fact.transition else {
                continue;
            };
            if matches!(
                trace,
                tcl_registry::TraceTransition::Add {
                    target: tcl_registry::TraceTarget::Variable(_),
                    ..
                } | tcl_registry::TraceTransition::Remove {
                    target: tcl_registry::TraceTarget::Variable(_),
                    ..
                }
            ) && !footprint.traces.contains(trace)
            {
                footprint.traces.push(trace.clone());
            }
        }
    }
    Some(footprint)
}

/// Possible name accesses of retained execution candidates.
/// These are hazard dependencies only: neither a physical store nor normal
/// continuation, value, compiler eligibility or purity is established.
#[derive(Debug)]
pub struct PossibleVariableNameOperands {
    candidates: Vec<VariableNameOperands>,
    unknown_residual: bool,
}

impl PossibleVariableNameOperands {
    /// Frozen candidate operands in their actual effective argument positions.
    pub fn operands(
        &self,
    ) -> impl Iterator<Item = (tcl_registry::ArgRole, &EffectiveInvocationWord)> {
        self.candidates
            .iter()
            .flat_map(VariableNameOperands::operands)
    }

    /// Possible contents mutations, excluding proved binding-only operands of
    /// each candidate. This grants no physical write or normal completion.
    pub fn contents_write_operands(&self) -> impl Iterator<Item = &EffectiveInvocationWord> {
        self.candidates
            .iter()
            .flat_map(VariableNameOperands::contents_write_operands)
    }

    /// Possible operand lookups with their authored handler phase. The phase
    /// chooses a context for a hazard query; it cannot establish a store.
    pub fn phased_operands(
        &self,
    ) -> impl Iterator<
        Item = (
            tcl_registry::ArgRole,
            &EffectiveInvocationWord,
            tcl_registry::native_compilation::VariableOperandBindingPhase,
            bool,
        ),
    > {
        self.candidates.iter().flat_map(|candidate| {
            candidate
                .operands()
                .map(|(role, word)| (role, word, candidate.binding_phase, candidate.destroys))
        })
    }

    /// Possible role classes whose exact operand layout remains unresolved.
    pub fn unresolved_roles(&self) -> impl Iterator<Item = tcl_registry::ArgRole> + '_ {
        self.candidates
            .iter()
            .flat_map(|candidate| candidate.unresolved_roles.iter().copied())
    }

    /// Unresolved role classes paired with that candidate's destruction effect.
    /// This preserves both write and destruction hazards when selected handlers
    /// disagree; it grants no operand position or reached mutation proof.
    pub fn phased_unresolved_roles(
        &self,
    ) -> impl Iterator<Item = (tcl_registry::ArgRole, bool)> + '_ {
        self.candidates.iter().flat_map(|candidate| {
            candidate
                .unresolved_roles
                .iter()
                .map(|role| (*role, candidate.destroys))
        })
    }

    /// At least one retained candidate may destroy its selected variable.
    #[must_use]
    pub fn destroys(&self) -> bool {
        self.candidates.iter().any(VariableNameOperands::destroys)
    }

    /// Additional handlers or accesses cannot be enumerated.
    #[must_use]
    pub const fn unknown_residual(&self) -> bool {
        self.unknown_residual
    }
}

/// Union May name-access hazards from authoritative candidate identities.
/// Written command spelling and catalogue declarations cannot donate candidates.
#[must_use]
pub fn possible_variable_name_operands(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableNameOperands> {
    possible_variable_name_operands_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Resolve retained possible variable name operands under the complete supplied availability generation.
/// Existing original identity, argument and handler premises remain required.
#[must_use]
pub fn possible_variable_name_operands_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<PossibleVariableNameOperands> {
    possible_variable_name_operands_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn possible_variable_name_operands_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleVariableNameOperands> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }

    if !matches!(
        tokens.synthetic,
        None | Some(
            crate::ir::SyntheticMarker::EvaluatedWrapper
                | crate::ir::SyntheticMarker::CapturedCatchOutputs
        )
    ) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let dialect = binding.variable_context.invocation_dialect;
    let mut footprint = PossibleVariableNameOperands {
        candidates: Vec::new(),
        unknown_residual: binding.execution_is_unknown() || binding.execution_may_be_absent(),
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        let candidate = target
            .registry_backed
            .then(|| {
                let effective = effective_words_for_target(tokens, target)?;
                let values = frozen_argument_words(tokens, &effective);
                let expansion_at = values.iter().position(|word| {
                    matches!(
                        word,
                        EffectiveInvocationWord::Expanded
                            | EffectiveInvocationWord::KnownExpansion(_) | EffectiveInvocationWord::KnownByteExpansion(_)
                    )
                });
                let mut words = vec![InvocationWord::Literal(&target.command)];
                words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
                let Ok(RegistryInvocationResolution::Resolved(facts)) =
                    resolve_registry_words_in_realm_with_metadata_context(
                        registry,
                        context,
                        &words,
                        dialect,
                        binding.invocation_realm().unwrap_or_default(),
                    )
                else {
                    return None;
                };
                if facts.argument_offset > values.len()
                    || facts.arity_accepts_frozen_arguments() == Some(false)
                {
                    return None;
                }
                Some(VariableNameOperands {
                    binding_phase: facts.successful_handler.map_or(
                        tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
                        tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
                    ),
                    operands: facts
                        .arg_roles
                        .iter()
                        .filter_map(|&(index, role)| {
                            matches!(
                                role,
                                tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
                            )
                            .then(|| {
                                let argument = facts.argument_offset + usize::from(index);
                                possible_name_operand_word(&values, expansion_at, argument)
                                    .map(|word| (role, word))
                            })
                            .flatten()
                        })
                        .collect(),
                    unresolved_roles: if facts.arg_roles_complete {
                        &[]
                    } else {
                        facts.arg_role_resolver_roles
                    },
                    binding_only_names: binding_only_operand_names(&facts),
                    destroys: facts
                        .traits
                        .contains(tcl_registry::Traits::DESTROYS_VARIABLE),
                })
            })
            .flatten();
        if let Some(candidate) = candidate {
            footprint.candidates.push(candidate);
        } else {
            footprint.unknown_residual = true;
        }
    }
    Some(footprint)
}

fn possible_name_operand_word(
    values: &[EffectiveInvocationWord],
    expansion_at: Option<usize>,
    argument: usize,
) -> Option<EffectiveInvocationWord> {
    // Expansion can move or supply later argv values. Earlier captured name
    // values remain fixed; neither branch grants a normal physical store.
    if expansion_at.is_some_and(|start| argument >= start) {
        Some(EffectiveInvocationWord::Dynamic)
    } else {
        values.get(argument).cloned()
    }
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

impl VariableNameOperands {
    /// Frozen evaluated name shapes, without a physical-store licence.
    pub fn operands(
        &self,
    ) -> impl Iterator<Item = (tcl_registry::ArgRole, &EffectiveInvocationWord)> {
        self.operands.iter().map(|(role, word)| (*role, word))
    }

    /// Candidate value/presence mutations, separately from alias or observer
    /// registration roles. Unknown names remain possible mutations.
    pub fn contents_write_operands(&self) -> impl Iterator<Item = &EffectiveInvocationWord> {
        self.operands.iter().filter_map(|(role, word)| {
            if *role != tcl_registry::ArgRole::VarWrite
                || matches!(word, EffectiveInvocationWord::Literal(name) if self.binding_only_names.contains(name))
            {
                None
            } else {
                Some(word)
            }
        })
    }

    /// Role classes whose exact argument positions remain unresolved.
    /// These denote possible accesses; they do not create definitions.
    #[must_use]
    pub const fn unresolved_roles(&self) -> &'static [tcl_registry::ArgRole] {
        self.unresolved_roles
    }

    /// The authored handler destroys the selected variable rather than stores it.
    #[must_use]
    pub const fn destroys(&self) -> bool {
        self.destroys
    }
}

fn binding_only_operand_names(facts: &InvocationFacts) -> std::collections::HashSet<String> {
    facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::VariableCellAlias(alias) => alias.local.literal(),
            tcl_registry::StateTransition::Trace(
                tcl_registry::TraceTransition::Add {
                    target: tcl_registry::TraceTarget::Variable(target),
                    ..
                }
                | tcl_registry::TraceTransition::Remove {
                    target: tcl_registry::TraceTarget::Variable(target),
                    ..
                },
            ) => target.literal(),
            _ => None,
        })
        .filter(|name| !crate::variable_bindings::contents_write_operand(facts, Some(name)))
        .map(str::to_owned)
        .collect()
}

/// Name-uncertainty projection independent of successful physical transfer.
#[must_use]
pub fn variable_name_operands(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<VariableNameOperands> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let invocation = resolved_handler_invocation(registry, context, tokens)?;
    if invocation
        .effective
        .words
        .iter()
        .any(|word| matches!(word, WordExpr::Expand { .. }))
    {
        return None;
    }
    let count = invocation.effective.words.len().checked_sub(1)?;
    if invocation.facts.argument_offset > count
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    let operands = invocation
        .facts
        .arg_roles
        .iter()
        .filter(|&&(_, role)| {
            matches!(
                role,
                tcl_registry::ArgRole::VarRead | tcl_registry::ArgRole::VarWrite
            )
        })
        .map(|&(index, role)| {
            (
                role,
                invocation.argument_word(invocation.facts.argument_offset + usize::from(index)),
            )
        })
        .collect();
    Some(VariableNameOperands {
        binding_phase: invocation.facts.successful_handler.map_or(
            tcl_registry::native_compilation::VariableOperandBindingPhase::AfterArguments,
            tcl_registry::native_compilation::SuccessfulHandlerSpec::variable_binding_phase,
        ),
        operands,
        unresolved_roles: if invocation.facts.arg_roles_complete {
            &[]
        } else {
            invocation.facts.arg_role_resolver_roles
        },
        binding_only_names: binding_only_operand_names(&invocation.facts),
        destroys: invocation
            .facts
            .traits
            .contains(tcl_registry::Traits::DESTROYS_VARIABLE),
    })
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

/// Value retained by a successful selected variable assignment.
/// This representation projection supplies neither an opcode nor a write proof.
#[derive(Debug)]
pub struct NormalValueAssignment {
    /// Evaluated literal destination name.
    pub name: String,
    /// Original value word, retaining its positioned substitutions.
    pub value: WordExpr,
}

/// Successful native index access, without compiler or completion permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalIndexAccessKind {
    /// Read one list element.
    ListRead,
    /// Write one list element, subject to the selected append-slot policy.
    ListWrite,
    /// Read one native string character.
    StringRead,
}

/// Original index-operation operands under a conditional native-handler
/// interpretation. This supplies no normal completion, current object class,
/// physical read version or executable operation.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ConditionalIndexAccessAdvice {
    pub(crate) kind: NormalIndexAccessKind,
    pub(crate) container: String,
    pub(crate) container_literal: Option<String>,
    pub(crate) index: String,
    pub(crate) container_word: WordExpr,
    pub(crate) index_word: WordExpr,
    pub(crate) list_set_bounds: Option<tcl_dialect::ListSetBounds>,
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

pub(crate) fn conditional_index_access_advice_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<ConditionalIndexAccessAdvice> {
    use NormalIndexAccessKind as Kind;
    use tcl_registry::{IntrinsicId, SemanticOperationId};
    let advice = tokens
        .source_binding
        .as_ref()?
        .declaration_operand_layout_advice(tokens)?;
    let dialect = advice.dialect();
    let mut unanimous = None;
    for target in advice.targets() {
        let effective =
            compose_original_effective_words(tokens, &target.command, &target.prepended)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                context,
                &words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?
        else {
            return None;
        };
        let offset = facts.argument_offset;
        let operands = effective.words.get(offset + 1..)?;
        let kind = match facts.operation {
            SemanticOperationId::Intrinsic(IntrinsicId::ListIndex) if operands.len() == 2 => {
                Kind::ListRead
            }
            SemanticOperationId::Intrinsic(IntrinsicId::ListSet) if operands.len() == 3 => {
                Kind::ListWrite
            }
            SemanticOperationId::Intrinsic(IntrinsicId::StringIndex) if operands.len() == 2 => {
                Kind::StringRead
            }
            _ => return None,
        };
        let literal = match values.get(offset + 1)? {
            EffectiveInvocationWord::Literal(value) => Some(value.clone()),
            _ => None,
        };
        let selected = ConditionalIndexAccessAdvice {
            kind,
            container: if kind == Kind::ListWrite {
                literal.clone()?
            } else {
                operands[0].legacy_text().clone()
            },
            container_literal: (kind != Kind::ListWrite).then_some(literal).flatten(),
            index: operands[1].sole_variable_substitution().map_or_else(
                || operands[1].legacy_text(),
                |(spelling, _)| spelling.to_owned(),
            ),
            container_word: operands[0].clone(),
            index_word: operands[1].clone(),
            list_set_bounds: dialect.list_set_bounds(),
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return None;
        }
        unanimous = Some(selected);
    }
    unanimous
}

/// Frozen operands of a successful selected index handler.
#[derive(Debug)]
pub struct NormalIndexAccess {
    /// Authored container operation.
    pub kind: NormalIndexAccessKind,
    /// Written container or variable-name presentation.
    pub container: String,
    /// Container value frozen before later argument effects, if proved.
    pub container_literal: Option<String>,
    /// Written index presentation.
    pub index: String,
    /// Index value frozen before later argument effects, if proved.
    pub index_literal: Option<String>,
    /// Selected native grammar for interpreting index values.
    pub index_syntax: Option<tcl_dialect::IndexSyntax>,
    /// Original effective container word, retaining its exact read site.
    pub container_word: WordExpr,
    /// Original effective index word, retaining its exact read site.
    pub index_word: WordExpr,
    /// Native list write boundary, independent of the editing catalogue.
    pub list_set_bounds: Option<tcl_dialect::ListSetBounds>,
}

#[derive(Debug)]
enum NormalRepresentationProof {
    Handler(Box<ResolvedStatementInvocation>),
    // Normal-edge operand layout only. Unknown positional arity cannot donate
    // the representation, type, security or effect contracts of Handler.
    PatternLayout(Box<ResolvedStatementInvocation>),
    Iteration {
        expected: Option<tcl_registry::TclType>,
        words: EffectiveCommandWords,
        label: String,
    },
}

impl NormalRepresentationInvocation {
    /// Frozen name and nominal class of a selected normal naming factory.
    /// This candidate grants no command allocation, physical object receipt,
    /// method implementation, completion or executable transformation.
    #[must_use]
    pub fn naming_factory_candidate(&self) -> Option<(String, &'static str)> {
        let call = self.handler()?;
        let factory = call.facts.named_object_factory?;
        let argument = call.facts.argument_offset.checked_add(factory.argument())?;
        Some((call.argument_literal(argument)?, factory.class_name()))
    }

    /// Nominal class of a successful selected callable-result factory.
    /// No written name, created command token or method implementation is proved.
    #[must_use]
    pub fn callable_result_class_candidate(&self) -> Option<&'static str> {
        let call = self.handler()?;
        (call.facts.return_type == Some(tcl_registry::TclType::Object))
            .then_some(call.facts.named_object_factory?.class_name())
    }

    /// Literal list result and its first original written operand. The selected
    /// normal handler's result contract owns the construction; public spelling,
    /// captured alias prefixes and substituted operands cannot create an anchor.
    #[must_use]
    pub fn plain_literal_list_result(&self) -> Option<(String, tcl_lexer::Span)> {
        let call = self.handler()?;
        let contract = call.facts.native_result?;
        let tcl_registry::native_result::NativeResultSelection::ListArguments { from, len } = call
            .with_argument_words(|words| {
                contract.select(words.arguments(), call.facts.argument_offset)
            })
        else {
            return None;
        };
        if len == 0 {
            return None;
        }
        let mut values = Vec::with_capacity(len);
        let mut first = None;
        for index in from..from.checked_add(len)? {
            let word_index = index.checked_add(1)?;
            if !matches!(
                call.effective.origins.get(word_index)?,
                InvocationWordOrigin::Written(_)
            ) {
                return None;
            }
            let WordExpr::Literal { text, source } = call.effective.words.get(word_index)? else {
                return None;
            };
            if text.is_empty()
                || text.contains(['{', '}', '"', '\\', '$', '[', ']'])
                || text.chars().any(char::is_whitespace)
            {
                return None;
            }
            if call.argument_literal(index).as_deref() != Some(text) {
                return None;
            }
            let end = source
                .span
                .start()
                .checked_add(u32::try_from(text.len()).ok()?)?;
            first.get_or_insert(tcl_lexer::Span::new(source.span.start(), end));
            values.push(text.as_str());
        }
        Some((values.join(" "), first?))
    }

    /// Dictionary read operands under the selected normal handler. These retain
    /// original value reads and frozen keys without selecting executable code.
    #[must_use]
    pub fn dictionary_lookup(&self) -> Option<(WordExpr, Vec<Option<String>>)> {
        let call = self.handler()?;
        if call.facts.operation
            != tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::DictGet)
        {
            return None;
        }
        let input = call.facts.argument_offset;
        let keys = (input + 1..call.arguments.len())
            .map(|index| call.argument_literal(index))
            .collect();
        Some((call.effective.words.get(input + 1)?.clone(), keys))
    }

    /// Original operands constructing a successful dictionary value, with
    /// duplicate-key order preserved and no inferred representation licence.
    #[must_use]
    pub fn dictionary_constructor_words(&self) -> Option<Vec<WordExpr>> {
        let call = self.handler()?;
        let contract = call.facts.native_result?;
        let selection = call.with_argument_words(|words| {
            contract.select(words.arguments(), call.facts.argument_offset)
        });
        let tcl_registry::native_result::NativeResultSelection::DictionaryArguments { from, len } =
            selection
        else {
            return None;
        };
        Some(call.effective.words.get(from + 1..from + len + 1)?.to_vec())
    }

    /// Ordered original operands of a selected successful List constructor.
    /// Their source spans stay separate; this grants no object representation proof.
    #[must_use]
    pub fn list_constructor_words(&self) -> Option<Vec<WordExpr>> {
        let call = self.handler()?;
        let contract = call.facts.native_result?;
        let selection = call.with_argument_words(|words| {
            contract.select(words.arguments(), call.facts.argument_offset)
        });
        let tcl_registry::native_result::NativeResultSelection::ListArguments { from, len } =
            selection
        else {
            return None;
        };
        let end = from.checked_add(len)?.checked_add(1)?;
        if !(from + 1..end).all(|index| {
            matches!(
                call.effective.origins.get(index),
                Some(InvocationWordOrigin::Written(_))
            )
        }) {
            return None;
        }
        Some(call.effective.words.get(from + 1..end)?.to_vec())
    }

    fn handler(&self) -> Option<&ResolvedStatementInvocation> {
        match &self.proof {
            NormalRepresentationProof::Handler(invocation) => Some(invocation),
            NormalRepresentationProof::PatternLayout(_)
            | NormalRepresentationProof::Iteration { .. } => None,
        }
    }

    fn pattern_handler(&self) -> Option<&ResolvedStatementInvocation> {
        match &self.proof {
            NormalRepresentationProof::Handler(call)
            | NormalRepresentationProof::PatternLayout(call) => Some(call),
            NormalRepresentationProof::Iteration { .. } => None,
        }
    }

    /// Rendered-result hints of the selected normal handler only. Conditional
    /// layouts and synthetic iteration contracts do not prove these properties.
    pub(crate) fn rendered_value_traits(&self) -> Option<tcl_registry::Traits> {
        Some(self.handler()?.facts.traits.intersection(
            tcl_registry::Traits::IS_UNESCAPE
                | tcl_registry::Traits::RETURNS_PATH
                | tcl_registry::Traits::UNNORMALISED_HTTP_GETTER,
        ))
    }

    /// Successful assignment value for tracking representations through generic
    /// calls. Physical definitions still come from the shared SSA place owner.
    #[must_use]
    pub fn value_assignment(&self) -> Option<NormalValueAssignment> {
        let invocation = self.handler()?;
        if invocation.facts.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Set,
            )
            || invocation.arguments.len() != 2
        {
            return None;
        }
        Some(NormalValueAssignment {
            name: invocation.argument_literal(0)?,
            value: invocation.effective.words.get(2)?.clone(),
        })
    }

    /// The actual selected handler key, without borrowing diagnostic spelling.
    pub(crate) fn descriptor_key(&self) -> Option<tcl_registry::CanonicalCommandDescriptorKey<'_>> {
        Some(self.handler()?.facts.descriptor_key())
    }

    /// Whether the selected normal handler uses no caller frame access.
    /// This escape-analysis property does not select a compiler operation,
    /// donate an ABI implementation or prove an invocation's completion.
    #[must_use]
    pub fn is_frameless_runtime(&self) -> bool {
        self.handler().is_some_and(|invocation| {
            invocation
                .facts
                .traits
                .contains(tcl_registry::Traits::FRAMELESS_RUNTIME)
        })
    }

    /// Normal index operands for diagnostic bounds analysis.
    /// This does not prove that the access succeeds or licence removing it.
    #[must_use]
    pub fn index_access(&self) -> Option<NormalIndexAccess> {
        use NormalIndexAccessKind as Kind;
        use tcl_registry::{IntrinsicId, SemanticOperationId};
        let invocation = self.handler()?;
        let offset = invocation.facts.argument_offset;
        let arguments = invocation.arguments.get(offset..)?;
        let kind = match invocation.facts.operation {
            SemanticOperationId::Intrinsic(IntrinsicId::ListIndex) if arguments.len() == 2 => {
                Kind::ListRead
            }
            SemanticOperationId::Intrinsic(IntrinsicId::ListSet) if arguments.len() == 3 => {
                Kind::ListWrite
            }
            SemanticOperationId::Intrinsic(IntrinsicId::StringIndex) if arguments.len() == 2 => {
                Kind::StringRead
            }
            _ => return None,
        };
        Some(NormalIndexAccess {
            kind,
            container: if kind == Kind::ListWrite {
                invocation.argument_literal(offset)?
            } else {
                arguments[0].clone()?
            },
            container_literal: (kind != Kind::ListWrite)
                .then(|| invocation.argument_literal(offset))
                .flatten(),
            index: arguments[1].clone()?,
            index_literal: invocation.argument_literal(offset + 1),
            index_syntax: invocation
                .dialect
                .and_then(tcl_registry::InvocationDialect::index_syntax),
            container_word: invocation.effective.words.get(offset + 1)?.clone(),
            index_word: invocation.effective.words.get(offset + 2)?.clone(),
            list_set_bounds: invocation
                .dialect
                .and_then(tcl_registry::InvocationDialect::list_set_bounds),
        })
    }

    /// Evaluated argument count, including retained alias prefixes.
    #[must_use]
    pub fn argument_count(&self) -> usize {
        self.effective_words().words.len().saturating_sub(1)
    }

    /// Selected subcommand operand offset.
    #[must_use]
    pub fn argument_offset(&self) -> usize {
        self.handler().map_or(0, |call| call.facts.argument_offset)
    }

    /// Original words and retained alias origins, for positioned value reads.
    #[must_use]
    pub fn effective_words(&self) -> &EffectiveCommandWords {
        match &self.proof {
            NormalRepresentationProof::Handler(invocation)
            | NormalRepresentationProof::PatternLayout(invocation) => &invocation.effective,
            NormalRepresentationProof::Iteration { words, .. } => words,
        }
    }

    /// Canonical diagnostic identity; this is not an execution licence.
    #[must_use]
    pub fn diagnostic_command(&self) -> &str {
        match &self.proof {
            NormalRepresentationProof::Handler(invocation)
            | NormalRepresentationProof::PatternLayout(invocation) => {
                &invocation.facts.canonical_command
            }
            NormalRepresentationProof::Iteration { label, .. } => label,
        }
    }

    /// Diagnostic label including the selected canonical subcommand.
    #[must_use]
    pub fn diagnostic_label(&self) -> String {
        let mut label = self.diagnostic_command().to_owned();
        if let Some(invocation) = self.handler()
            && let tcl_registry::OwnedSubcommandResolution::Exact { canonical_name, .. }
            | tcl_registry::OwnedSubcommandResolution::UniquePrefix { canonical_name, .. } =
                &invocation.facts.subcommand
        {
            label.push(' ');
            label.push_str(canonical_name);
        }
        label
    }

    /// Original written pattern operand selected by the normal handler grammar.
    /// Captured alias-prefix patterns have no source argument position.
    #[must_use]
    pub fn pattern_source_argument_index(&self, registry: &CommandRegistry) -> Option<usize> {
        let call = self.pattern_handler()?;
        let query = call
            .dialect
            .and_then(tcl_registry::InvocationDialect::authoring_query);
        if registry
            .get_for_surface(&call.facts.canonical_command, query)?
            .pattern_type
            != Some(tcl_registry::patterns::PatternType::Regex)
        {
            return None;
        }
        if let Some(index) = self.native_pattern_source_argument {
            return Some(index);
        }
        let alternatives = self
            .pattern_operand_alternatives
            .iter()
            .map(|(argument, values)| tcl_registry::RoleOperandAlternatives {
                argument: *argument,
                values: tcl_registry::RoleOperandValues::Closed(values),
            })
            .collect::<Vec<_>>();
        let index = call.with_argument_words(|words| {
            let roles = registry.arg_role_assignments_consensus(
                &call.facts.canonical_command,
                words.arguments(),
                &alternatives,
                &[tcl_registry::ArgRole::Pattern],
            )?;
            match roles.as_slice() {
                [(index, tcl_registry::ArgRole::Pattern)] => Some(*index),
                _ => None,
            }
        })?;
        match call.effective.origins.get(index.checked_add(1)?)? {
            InvocationWordOrigin::Written(index) => index.checked_sub(1),
            _ => None,
        }
    }

    /// Written operands that may carry a regex on an admitted runtime layout.
    /// This hazard-only projection gives no guaranteed role, result type,
    /// variable definition, native operation or execution proof.
    #[must_use]
    pub fn possible_pattern_source_argument_indices(
        &self,
        registry: &CommandRegistry,
    ) -> Option<Vec<usize>> {
        if let Some(index) = self.pattern_source_argument_index(registry) {
            return Some(vec![index]);
        }
        let call = self.pattern_handler()?;
        if let Some(index) = self.native_pattern_source_argument {
            return Some(vec![index]);
        }
        let indices = call.with_argument_words(|words| {
            registry.possible_pattern_argument_indices_words(
                &call.facts.canonical_command,
                words.arguments(),
            )
        })?;
        Some(
            indices
                .into_iter()
                .filter_map(
                    |index| match call.effective.origins.get(index.checked_add(1)?)? {
                        InvocationWordOrigin::Written(index) => index.checked_sub(1),
                        _ => None,
                    },
                )
                .collect(),
        )
    }

    /// Authored normal representation mutation/coercion contract.
    #[must_use]
    pub fn representation_effect(&self) -> tcl_registry::RepresentationEffect {
        self.handler()
            .map_or(tcl_registry::RepresentationEffect::None, |call| {
                call.facts.representation_effect
            })
    }

    /// Normal conversion policy for independently proved ordinary containers.
    /// A semantic type, unknown object or abstract list cannot supply that proof.
    /// The policy applies to operand objects, never the command's return value.
    #[must_use]
    pub fn successful_ordinary_container_coercion(
        &self,
    ) -> Option<tcl_registry::representation::OrdinaryContainerCoercion> {
        let call = self.handler()?;
        if call.facts.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        call.with_argument_words(|words| {
            call.facts
                .representation_effect
                .successful_ordinary_container_coercion(
                    words.arguments(),
                    call.facts.argument_offset,
                )
        })
    }

    /// Successful ordinary list-input conversions under native foreach layout.
    /// Before success, possible conversions cannot commit a Must representation.
    #[must_use]
    pub fn successful_ordinary_container_coercions(
        &self,
    ) -> Option<Vec<tcl_registry::representation::OrdinaryContainerCoercion>> {
        let call = self.handler()?;
        if call.facts.arity_accepts_frozen_arguments() != Some(true) {
            return None;
        }
        call.with_argument_words(|words| {
            call.facts
                .representation_effect
                .successful_ordinary_container_coercions(
                    words.arguments(),
                    call.facts.argument_offset,
                )
        })
    }

    /// Authored normal byte-array operand conversion contract.
    #[must_use]
    pub fn byte_array_effect(&self) -> tcl_registry::ByteArrayEffect {
        self.handler()
            .map_or(tcl_registry::ByteArrayEffect::None, |call| {
                call.facts.byte_array_effect
            })
    }

    /// Encoder data object selected by the normal handler's operand protocol.
    /// Encoding names and option values cannot supply data provenance.
    #[must_use]
    pub fn encoded_value_word(&self) -> Option<WordExpr> {
        let call = self.handler()?;
        let offset = call.facts.argument_offset;
        let count = call.arguments.len().checked_sub(offset)?;
        let data = call.facts.byte_array_effect.encoded_value_argument(count)?;
        call.effective
            .words
            .get(offset.checked_add(data)?.checked_add(1)?)
            .cloned()
    }

    /// Declared representation of the successful selected result, when known.
    /// This supplies no result bytes, native opcode or non-normal guarantee.
    #[must_use]
    pub fn result_representation_type(&self) -> Option<tcl_registry::TclType> {
        self.handler()?.facts.return_type
    }

    /// Authored element representation of the successful selected result.
    /// Input element types still require positioned reads of the retained words.
    #[must_use]
    pub fn result_elements(&self) -> Option<tcl_registry::ReturnElements> {
        self.handler()?.facts.return_elements
    }

    /// Whether a successful selected form produces a byte-array object.
    #[must_use]
    pub fn returns_byte_array(&self) -> bool {
        self.handler()
            .is_some_and(|call| call.facts.return_type == Some(tcl_registry::TclType::ByteArray))
    }

    /// Selected byte payload layout, with unknown forms excluded.
    #[must_use]
    pub fn byte_array_payload(&self) -> Option<tcl_registry::BytePayloadSpec> {
        self.handler()?.facts.byte_array_payload
    }

    /// Whether normal mutation reads the old value before writing it.
    #[must_use]
    pub fn reads_before_write(&self) -> bool {
        self.handler().is_some_and(|call| {
            call.facts
                .traits
                .contains(tcl_registry::Traits::READS_BEFORE_WRITE)
        })
    }

    /// Argument roles only for distinguishing inert words from value reads.
    /// These do not establish script entry or execution topology.
    #[must_use]
    pub fn operand_roles(&self) -> &[(u8, tcl_registry::ArgRole)] {
        self.handler()
            .map_or(&[], |call| call.facts.arg_roles.as_slice())
    }

    /// Original operands selected as deferred scripts by the actual normal
    /// handler and its frozen layout. This proves metadata only: no script
    /// entry, callback dispatch or editable source is supplied by these indices.
    /// Unknown timing coverage returns `None`. Even `Some([])` cannot prove
    /// callback absence: operands captured in an alias prefix have no writable
    /// source index in this invocation and are deliberately omitted.
    #[must_use]
    pub fn deferred_script_source_argument_indices(&self) -> Option<Vec<usize>> {
        let call = self.handler()?;
        Some(
            call.facts
                .deferred_script_argument_indices()?
                .iter()
                .filter_map(|&argument| call.effective.written_argument(argument))
                .collect(),
        )
    }

    /// Frozen evaluated argument knowledge, without inventing unknown bytes.
    #[must_use]
    pub fn argument_word(&self, index: usize) -> EffectiveInvocationWord {
        self.handler()
            .map_or(EffectiveInvocationWord::Dynamic, |call| {
                call.argument_word(index)
            })
    }

    /// Known evaluated value, including an alias prefix's captured value.
    #[must_use]
    pub fn argument_literal(&self, index: usize) -> Option<String> {
        self.handler()?.argument_literal(index)
    }

    /// Registry-owned positional representation hint selected with frozen argv.
    /// Genuine synthetic iterators carry their producer's input representation.
    #[must_use]
    pub fn argument_type_hint(
        &self,
        index: usize,
    ) -> Option<&'static tcl_registry::hooks::ArgTypeHint> {
        use tcl_registry::{TclType, hooks::ArgTypeHint};
        const LIST: ArgTypeHint = ArgTypeHint {
            expected: Some(TclType::List),
            shimmers: true,
            transparent_from: &[],
        };
        const DICT: ArgTypeHint = ArgTypeHint {
            expected: Some(TclType::Dict),
            shimmers: true,
            transparent_from: &[],
        };
        match &self.proof {
            NormalRepresentationProof::Handler(call) => call.facts.argument_type_hint(index),
            NormalRepresentationProof::Iteration { expected, .. }
                if index < self.argument_count() =>
            {
                match expected {
                    Some(TclType::List) => Some(&LIST),
                    Some(TclType::Dict) => Some(&DICT),
                    _ => None,
                }
            }
            NormalRepresentationProof::PatternLayout(_)
            | NormalRepresentationProof::Iteration { .. } => None,
        }
    }

    /// Conditional normal cache disposition for the selected stock Length
    /// operand. The original current stock class must be authenticated by the
    /// caller; semantic types and result types cannot supply that evidence.
    /// This is conversion-cost metadata, not success, value or erasure proof.
    #[must_use]
    pub fn stock_length_operand_cache(
        &self,
        index: usize,
        class: tcl_registry::native_stock_list::NativeStockListInputClass,
        bytes: Option<&[u8]>,
    ) -> Option<tcl_registry::native_stock_list::NativeStockListCacheDisposition> {
        let call = self.handler()?;
        call.with_argument_words(|words| {
            let protocol = call.facts.stock_list_length_protocol(words.arguments())?;
            if protocol.argument() != index {
                return None;
            }
            protocol.normal_cache_disposition(class, bytes)
        })
    }

    /// Normal expression read projection under the caller's retained full grammar.
    pub(crate) fn source_expression_with_syntax_context(
        &self,
        parser: tcl_syntax::expr::parser::ExprParseContext,
        span: tcl_lexer::Span,
        parent: Option<crate::command_binding::CommandAllocationSite>,
    ) -> Option<crate::word_subst::LiftedSourceExpression> {
        crate::word_subst::source_expression_from_invocation_with_syntax_context(
            self.handler()?,
            parser,
            span,
            parent,
        )
    }
}

/// Select a representation contract for a reached normal handler.
/// Compiler uncertainty never becomes an opcode or body-execution proof here.
///
/// ```no_run
/// use tcl_compiler::{ir::CommandTokens, registry_invocation::normal_representation_invocation};
/// use tcl_registry::CommandRegistry;
/// fn successful_result(registry: &CommandRegistry, tokens: &CommandTokens) {
///     if let Some(normal) = normal_representation_invocation(registry, None, tokens) {
///         let result_representation = normal.result_representation_type();
///         // This hint supplies neither result bytes nor an opcode.
///         let _ = result_representation;
///     }
/// }
/// ```
#[must_use]
pub fn normal_representation_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalRepresentationInvocation> {
    normal_representation_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Successful representation metadata under the actual availability generation.
/// This retains all normal-handler and operand requirements of the typed owner;
/// availability does not create a successful handler or a native invocation.
#[must_use]
pub fn normal_representation_invocation_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<NormalRepresentationInvocation> {
    normal_representation_invocation_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

/// Successful representation metadata with the caller's complete supplied
/// availability and source-policy context. Selection remains independent of
/// successful completion, original value identity and Native execution entry.
#[must_use]
pub fn normal_representation_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalRepresentationInvocation> {
    if let Some(crate::ir::SyntheticMarker::IterationBindings(expected)) = tokens.synthetic {
        // Minted only by the typed iterator lowering. This selects input value
        // conversion, never an invocation of the synthetic diagnostic label.
        return Some(NormalRepresentationInvocation {
            proof: NormalRepresentationProof::Iteration {
                expected,
                words: EffectiveCommandWords {
                    words: tokens.words().to_vec(),
                    origins: (0..tokens.words().len())
                        .map(InvocationWordOrigin::Written)
                        .collect(),
                    binding_prefix: Vec::new(),
                },
                label: tokens.argv_texts.first()?.clone(),
            },
            native_pattern_source_argument: None,
            pattern_operand_alternatives: Vec::new(),
        });
    }
    let invocation = resolved_tokens_invocation_with_metadata_context(registry, context, tokens)
        .or_else(|| resolved_handler_invocation_with_metadata_context(registry, context, tokens))?;
    // The public ensemble's identity does not select an unavailable or
    // indeterminate member. Parent facts retain that uncertainty for other
    // purposes; they cannot supply a successful value-handler contract here.
    if matches!(
        invocation.facts.subcommand,
        tcl_registry::OwnedSubcommandResolution::Unknown { .. }
            | tcl_registry::OwnedSubcommandResolution::Ambiguous { .. }
            | tcl_registry::OwnedSubcommandResolution::Indeterminate { .. }
    ) {
        return None;
    }
    // Representation contracts require accepted arity. A conditional Pattern
    // projection may retain unknown positional arity under a separate carrier.
    // Expansion still cannot fabricate ordinary operand positions.
    if invocation
        .effective
        .words
        .iter()
        .any(|word| matches!(word, WordExpr::Expand { .. }))
    {
        return None;
    }
    if invocation.facts.argument_offset > invocation.arguments.len() {
        return None;
    }
    let accepted = invocation.facts.arity_accepts_frozen_arguments();
    if accepted == Some(false)
        || (accepted.is_none() && !has_regex_pattern_contract(&invocation, registry))
    {
        return None;
    }
    let native_pattern_source_argument = compiled_pattern_source_argument(tokens);
    let pattern_operand_alternatives = pattern_operand_alternatives(tokens, &invocation, registry);
    let invocation = Box::new(invocation);
    Some(NormalRepresentationInvocation {
        native_pattern_source_argument,
        pattern_operand_alternatives,
        proof: if accepted == Some(true) {
            NormalRepresentationProof::Handler(invocation)
        } else {
            NormalRepresentationProof::PatternLayout(invocation)
        },
    })
}

fn has_regex_pattern_contract(
    invocation: &ResolvedStatementInvocation,
    registry: &CommandRegistry,
) -> bool {
    let query = invocation
        .dialect
        .and_then(tcl_registry::InvocationDialect::authoring_query);
    registry
        .get_for_surface(&invocation.facts.canonical_command, query)
        .and_then(|spec| spec.pattern_type)
        == Some(tcl_registry::patterns::PatternType::Regex)
}

// Values come from each original read boundary, before later argv callbacks.
// They are used only to compare operand layouts, never to freeze an argv value.
fn pattern_operand_alternatives(
    tokens: &CommandTokens,
    invocation: &ResolvedStatementInvocation,
    registry: &CommandRegistry,
) -> Vec<(usize, Vec<String>)> {
    if !has_regex_pattern_contract(invocation, registry) {
        return Vec::new();
    }
    invocation
        .effective
        .words
        .iter()
        .skip(1)
        .enumerate()
        .filter(|(index, _)| invocation.argument_literal(*index).is_none())
        .filter_map(|(index, word)| {
            let (_, source) = word.sole_variable_substitution()?;
            let access = crate::command_binding::SourceVariableAccess::find_at_site(
                &tokens.variable_accesses,
                source,
            )?;
            closed_read_literals(access, registry).map(|values| (index, values))
        })
        .collect()
}

fn closed_read_literals(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &CommandRegistry,
) -> Option<Vec<String>> {
    if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed
        || access.context_alternatives().is_empty()
    {
        return None;
    }
    let mut values = std::collections::BTreeSet::new();
    for context in access.context_alternatives() {
        let crate::literal_contents::LiteralContentsAlternatives::Closed(contents) =
            context.substitution_literal_alternatives(&access.original_spelling, registry)
        else {
            return None;
        };
        values.extend(contents.values().iter().cloned());
        if values.len() > 8 || values.iter().map(String::len).sum::<usize>() > 8192 {
            return None;
        }
    }
    Some(values.into_iter().collect())
}

fn compiled_pattern_source_argument(tokens: &CommandTokens) -> Option<usize> {
    let binding = tokens.source_binding.as_ref()?;
    let proof = binding.native_operand_layout()?;
    // Layout and normal handler identities have independent authority. An
    // argv mutation or conflicting registration cannot borrow this layout.
    if binding.proved_handler_target()? != proof.target()
        || proof.compilation_site().offset != tokens.words().first()?.source().span.start()
    {
        return None;
    }
    let tcl_registry::native_compilation::NativeCompilationOperandLayout::Pattern {
        argument_index,
        ..
    } = proof.layout();
    let effective = compose_original_effective_words(
        tokens,
        &proof.target().command,
        &proof.target().prepended,
    )?;
    match effective.origins.get(argument_index.checked_add(1)?)? {
        InvocationWordOrigin::Written(index) => index.checked_sub(1),
        _ => None,
    }
}

/// Successful result security properties of the exact selected normal handler.
/// No compiler operation, completion or catalogue candidate is exposed.
#[derive(Debug)]
pub struct NormalTaintInvocation {
    invocation: Box<ResolvedStatementInvocation>,
}

impl NormalTaintInvocation {
    /// Authored source colour after actual frozen argc selection. Safety bits
    /// require this selected implementation and never come from its spelling.
    #[must_use]
    pub fn source_colour(&self) -> Option<tcl_registry::taint::TaintColour> {
        self.invocation.facts.taint_source
    }
    /// Declared bounded successful numeric result, without an execution licence.
    #[must_use]
    pub fn is_sanitiser(&self) -> bool {
        matches!(
            self.invocation.facts.return_type,
            Some(tcl_registry::TclType::Int | tcl_registry::TclType::Boolean)
        )
    }

    /// Authored successful-result transform proven from frozen evaluated argv.
    #[must_use]
    pub fn transform_colour(&self) -> Option<tcl_registry::taint::TaintColour> {
        self.invocation.facts.taint_transform
    }
}

/// Resolve security metadata through the same accepted normal handler proof as
/// representation queries, retaining private worker and vendor prerequisites.
#[must_use]
pub fn normal_taint_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalTaintInvocation> {
    let normal = normal_representation_invocation(registry, context, tokens)?;
    let NormalRepresentationProof::Handler(invocation) = normal.proof else {
        return None;
    };
    Some(NormalTaintInvocation { invocation })
}

/// Select normal result metadata through the supplied generation and original
/// handler proof. Missing or foreign availability never reconstructs a label.
pub(crate) fn normal_taint_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalTaintInvocation> {
    let context = context.filter(|context| context.matches_registry(registry))?;
    let normal =
        normal_representation_invocation_with_metadata_context(registry, Some(context), tokens)?;
    let NormalRepresentationProof::Handler(invocation) = normal.proof else {
        return None;
    };
    Some(NormalTaintInvocation { invocation })
}

/// Statement convenience projection retaining the original token proof.
#[must_use]
pub fn normal_statement_representation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    statement: &crate::ir::Statement,
) -> Option<NormalRepresentationInvocation> {
    normal_statement_representation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        statement,
    )
}

pub(crate) fn normal_statement_representation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    statement: &crate::ir::Statement,
) -> Option<NormalRepresentationInvocation> {
    normal_representation_invocation_with_metadata_context(registry, context, statement.tokens()?)
}

/// Possible effect regions on the selected handler's normal route. No
/// completion, compiler, purity or unknown-residual obligation is withdrawn.
pub(crate) fn possible_normal_handler_effect_regions_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<(
    crate::side_effects::EffectRegion,
    crate::side_effects::EffectRegion,
)> {
    let invocation = resolved_handler_invocation_with_metadata_context(registry, context, tokens)?;
    Some(crate::side_effects::normal_handler_effect_regions(
        &invocation.facts.effects,
    ))
}

/// Exact original argv layout of the independently proved handler. This
/// borrows its retained semantic context and implementation dependencies, but
/// does not require an expression preparation, successful execution or CPP.
/// Consumers close any value/evaluation obligations through their own owner.
pub(crate) fn original_selected_handler_layout_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let context = body_assistance_context(registry, tokens)?;
    resolved_handler_invocation(registry, Some(context), tokens)
}

/// Selected callback grammar from an actual handler or its authentic
/// declaration observation. This supplies no successful handler or CPP entry.
pub(crate) fn original_callback_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let context = body_assistance_context(registry, tokens)?;
    original_callback_invocation_with_metadata_context(registry, context.into(), tokens)
}

pub(crate) fn original_callback_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    if !context.matches_registry(registry) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)
        .inspect(|invocation| {
            trace_callback_selection("handler", context, tokens, invocation);
        })
        .or_else(|| {
            original_declared_structured_invocation(registry, context, tokens).inspect(
                |invocation| {
                    trace_callback_selection("declaration", context, tokens, invocation);
                },
            )
        })
}

fn trace_callback_selection(
    route: &str,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    invocation: &ResolvedStatementInvocation,
) {
    if tcl_registry::trace_deferred_writes_any() {
        eprintln!(
            "TCL_LSP_TRACE_METADATA_CALLBACK route={route} source_offset={:?} written_head={:?} canonical={:?} context={:p}",
            tokens
                .words()
                .first()
                .map(|word| word.source().span.start()),
            tokens.argv_texts.first(),
            invocation.facts.canonical_command,
            context.context(),
        );
    }
}

pub(crate) fn resolved_handler_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    resolved_handler_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

pub(crate) fn resolved_handler_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    use tcl_registry::native_compilation::NormalHandlerImplementationLookup;
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let target = binding
        .proved_handler_target()
        .filter(|target| target.registry_backed)?;
    let effective = effective_words_for_target(tokens, target)?;
    let dialect = binding.variable_context.invocation_dialect.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    let (resolution, authored_case_body_selection) =
        resolve_effective_tokens_with_metadata_context_and_case(
            registry, context, tokens, &effective, dialect,
        )
        .ok()?;
    let RegistryInvocationResolution::Resolved(facts) = resolution else {
        return None;
    };
    let invocation = ResolvedStatementInvocation {
        dialect,
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
        facts,
        authored_case_body_selection: Some(authored_case_body_selection),
    };
    match invocation
        .facts
        .normal_handler_implementation_lookup(dialect)
    {
        NormalHandlerImplementationLookup::NoneRequired => {}
        NormalHandlerImplementationLookup::Required(lookup) => {
            if !binding.proves_native_implementation_lookup(&lookup) {
                return None;
            }
        }
        NormalHandlerImplementationLookup::RequiredPath(paths) => {
            let closed = invocation.with_argument_words(|words| {
                binding.proves_native_handler_path(
                    paths,
                    words.arguments(),
                    invocation.facts.argument_offset,
                )
            });
            if !closed {
                return None;
            }
        }
        NormalHandlerImplementationLookup::Unknown => return None,
    }
    Some(invocation)
}

fn normal_alias_frame(tokens: &CommandTokens) -> tcl_registry::VariableAliasFrame {
    tokens
        .source_binding
        .as_ref()
        .map_or(tcl_registry::VariableAliasFrame::Unknown, |binding| {
            binding.variable_context.alias_frame()
        })
}

fn normal_output_compilation(
    tokens: &CommandTokens,
) -> tcl_registry::native_compilation::NativeCompilationSelection {
    tokens.source_binding.as_ref().map_or(
        tcl_registry::native_compilation::NativeCompilationSelection::Unknown,
        crate::command_binding::SourceInvocationBinding::native_compilation_selection,
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

/// Document-declared argument roles applicable at a retained lookup site.
/// These are advisory facts: they cannot establish stores, effects, completion,
/// purity or executable dispatch and are deliberately separate from runtime facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredInvocationAssistance<'a> {
    /// Authored declaration with its original trust provenance.
    pub declaration: &'a tcl_registry::model::DeclaredCommand,
    /// Declared role positions in the written argument sequence.
    pub roles: Vec<(usize, tcl_registry::ArgRole)>,
}

/// Query declaration assistance through its owner after the shared lookup
/// kernel establishes slot applicability. Mutation or argument expansion
/// cannot recover an applicable contract by written-head matching.
#[must_use]
pub fn resolved_declared_assistance<'a>(
    surface: &tcl_registry::model::DocumentCommandSurface<'a>,
    tokens: &CommandTokens,
) -> Option<DeclaredInvocationAssistance<'a>> {
    let candidate = tokens.source_binding.as_ref()?.declared_command.as_ref()?;
    let declaration = surface.declared_command(&candidate.name)?;
    if tokens
        .words()
        .iter()
        .any(|word| matches!(word, WordExpr::Expand { .. } | WordExpr::Opaque { .. }))
    {
        return None;
    }
    let supplied = tokens.words().len().checked_sub(1)?;
    let mut roles = Vec::new();
    for argument in &declaration.arguments {
        for index in declaration.arg_indices_for_role(argument.role, supplied) {
            if !roles.contains(&(index, argument.role)) {
                roles.push((index, argument.role));
            }
        }
    }
    roles.sort_by_key(|(index, _)| *index);
    Some(DeclaredInvocationAssistance { declaration, roles })
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

/// Compose effective execution words exactly once from the shared point binding.
/// Native compiled selection can differ from the later live lookup target.
/// An explicitly uncertain binding never falls back to source spelling.
#[must_use]
pub fn effective_command_words(tokens: &CommandTokens) -> Option<EffectiveCommandWords> {
    let Some(binding) = &tokens.source_binding else {
        return Some(EffectiveCommandWords {
            words: tokens.words().to_vec(),
            origins: (0..tokens.words().len())
                .map(InvocationWordOrigin::Written)
                .collect(),
            binding_prefix: Vec::new(),
        });
    };
    effective_words_for_target(tokens, binding.proved_execution_target()?)
}

/// Retain original Tcl word shape independently from its evaluated value.
#[must_use]
pub fn native_compilation_word_shape(
    word: &WordExpr,
) -> tcl_registry::native_compilation::NativeCompilationWordShape {
    use tcl_registry::native_compilation::NativeCompilationWordShape as Shape;
    match word {
        WordExpr::BracedLiteral { .. } => Shape::BracedLiteral,
        WordExpr::Literal { .. } => Shape::Literal,
        WordExpr::Expand { .. } => Shape::Expanded,
        WordExpr::Opaque { .. } => Shape::Opaque,
        WordExpr::Template { parts, .. }
            if matches!(parts.as_slice(), [WordPart::Text { text, .. }]
                if native_simple_text_component(text)) =>
        {
            // A quoted SIMPLE_WORD retains its literal compiler token. A
            // decoded backslash run is a Parts word even when its value is
            // known, so it cannot acquire that compiler capability.
            Shape::QuotedLiteral
        }
        WordExpr::Template { parts, .. }
            if !parts.is_empty()
                && parts
                    .iter()
                    .all(|part| matches!(part, WordPart::Text { .. })) =>
        {
            Shape::BackslashLiteral
        }
        _ => Shape::Substituted,
    }
}

fn native_simple_text_component(text: &str) -> bool {
    use tcl_lexer::word_parts::{ExecutablePart, ExecutablePartArena, ExecutableText, SubstFlags};
    let Ok(length) = u32::try_from(text.len()) else {
        return false;
    };
    let Ok(arena) = ExecutablePartArena::decompose(
        tcl_lexer::SourceImage::native(text.as_bytes()),
        tcl_lexer::Span::new(0, length),
        SubstFlags::default(),
        // dialect-drift-ok: `text` is a Text part already split under the
        // document's grammar (no substitution); only its backslash runs are
        // asked about, which every grammar decodes alike
        tcl_lexer::LexerConfig::default(),
    ) else {
        return false;
    };
    matches!(arena.list(arena.root()), [part] if matches!(part.part, ExecutablePart::Text(ExecutableText::Original)))
}

/// Query original source compiler grammar. This establishes syntax capability
/// only; callers still need actual token selection and runtime entry validation.
#[must_use]
pub fn native_compilation_syntax(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    dialect: tcl_registry::InvocationDialect,
    context: tcl_registry::native_compilation::NativeCompilationContext,
) -> Option<(
    InvocationFacts,
    tcl_registry::native_compilation::NativeCompilationSelection,
)> {
    if tokens
        .words()
        .first()
        .and_then(|word| native_compilation_word_shape(word).compiler_head(Some(dialect)))
        != Some(true)
    {
        return None;
    }
    let values: Vec<_> = tokens
        .words()
        .iter()
        .map(|word| {
            effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
        })
        .collect();
    let words: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let (head, arguments) = words.split_first()?;
    let invocation = InvocationWords::structured(*head, arguments).with_dialect(dialect);
    let resolution = registry.resolve_structured_invocation(invocation, dialect.authoring_query());
    let resolved = resolution.resolved()?;
    let native = resolved.semantics.native_compilation?;
    let shapes: Vec<_> = tokens
        .words()
        .iter()
        .skip(1)
        .map(native_compilation_word_shape)
        .collect();
    let facts = resolved.facts();
    let original = tokens.source_binding.as_ref().and_then(|binding| {
        let (image, original_words, offset) = binding.original_compiler_source(tokens)?;
        original_native_compilation(
            native,
            OriginalNativeCompilerInvocation {
                image,
                words: original_words,
                offset,
                config: binding.original_lexer_config_for_tokens(tokens)?,
                source_protocol: binding.compiler_source_protocol(),
                compiler_dialect: binding
                    .native_compiler_dialect()
                    .filter(|original| original.has_same_execution_policy(dialect)),
                context,
                operand_from: native.original_operand_from_for_facts(&facts)?,
            },
        )
    });
    let selection = original.map_or_else(
        || {
            if (native.namespace_binding_kind().is_some()
                || native.grammar
                    == tcl_registry::native_compilation::NativeCompilationGrammar::Switch)
                && tokens.source_binding.is_some()
            {
                tcl_registry::native_compilation::NativeCompilationSelection::Unknown
            } else {
                native.select_for_facts(invocation, &shapes, &facts, Some(dialect), context)
            }
        },
        |(selection, _)| selection,
    );
    Some((facts, selection))
}

/// Runtime validation boundary of a selected native operation. Ordinary
/// executable specialisations retain live before-arguments validation.
///
/// Handler convergence alone does not establish that such an operation exists.
/// Use [`proved_native_admitted_inline_operation`] before emitting a guarded opcode.
#[must_use]
pub fn command_binding_guard(tokens: &CommandTokens) -> tcl_runtime_api::CommandBindingGuard {
    use tcl_runtime_api::CommandBindingGuard;
    let Some(binding) = &tokens.source_binding else {
        return CommandBindingGuard::BeforeArguments;
    };
    binding
        .admitted_inline_invocation()
        .map_or(CommandBindingGuard::BeforeArguments, |proof| {
            native_command_binding_guard(proof.guard)
        })
}

/// Exact native operation selected at this original invocation's compilation.
/// A known successful handler or a generic late call cannot license an opcode.
#[must_use]
pub fn proved_native_inline_operation(
    tokens: &CommandTokens,
) -> Option<(
    tcl_registry::SemanticOperationId,
    tcl_registry::native_compilation::NativeCompilationGuard,
)> {
    match tokens
        .source_binding
        .as_ref()?
        .native_compilation_selection()
    {
        tcl_registry::native_compilation::NativeCompilationSelection::Inline {
            operation,
            guard,
        } => Some((operation, guard)),
        _ => None,
    }
}

/// Opcode recipe retained at chunk admission, before runtime callback effects.
/// This licenses guarded emission and original-source replay only. It cannot
/// establish a reached handler, normal effects, or permission to erase work.
#[must_use]
pub fn proved_native_admitted_inline_operation(
    tokens: &CommandTokens,
) -> Option<(
    tcl_registry::SemanticOperationId,
    tcl_registry::native_compilation::NativeCompilationGuard,
)> {
    match tokens
        .source_binding
        .as_ref()?
        .native_compilation_admission_selection()
    {
        tcl_registry::native_compilation::NativeCompilationSelection::Inline {
            operation,
            guard,
        } => Some((operation, guard)),
        _ => None,
    }
}

/// Whether a folded same-frame script must retain its runtime invocation.
/// A logical body region does not supply an actual native expansion recipe.
#[must_use]
pub fn block_requires_runtime_script(statement: &Statement) -> bool {
    let Statement::Block {
        error_context: Some(context),
        tokens,
        ..
    } = statement
    else {
        return false;
    };
    tokens
        .as_ref()
        .and_then(proved_native_admitted_inline_operation)
        .is_none_or(|(operation, _)| operation.inline_body_error_context() != Some(*context))
}

/// Restore a folded script's complete original invocation without flattening
/// its body into the caller's local table, literal pool or exception ranges.
/// Missing original words cannot be reconstructed from the advisory body.
#[must_use]
pub fn original_block_runtime_invocation(statement: &Statement) -> Option<Statement> {
    let Statement::Block {
        span,
        tokens: Some(tokens),
        ..
    } = statement
    else {
        return None;
    };
    if tokens.synthetic.is_some() || !tokens.words_align_with_argv_text() {
        return None;
    }
    let (command, arguments) = tokens.argv_texts.split_first()?;
    Some(Statement::Barrier {
        span: *span,
        reason: "separate same-frame script compilation".to_owned(),
        command: command.clone(),
        canonical_command: None,
        args: arguments.to_vec(),
        tokens: Some(tokens.clone()),
    })
}

/// Compiler environment of one proved inline script operand. This consumes
/// the same frozen argv and authored native range descriptor as source preflight;
/// a spelling or normal-handler envelope cannot supply an exception range.
/// `None` means that retained invocation, operand, or compiler metadata is
/// insufficient; it does not authorise inheriting a default child context.
#[must_use]
pub fn proved_native_inline_body_context(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    enclosing: tcl_registry::native_compilation::NativeCompilationContext,
    operand: usize,
) -> Option<tcl_registry::native_compilation::NativeCompilationContext> {
    proved_native_admitted_inline_operation(tokens)?;
    admitted_native_compiler_invocation(registry, None, tokens)?.body_context(enclosing, operand)
}

/// Retain a site's existing opcode guard only when its source selected Inline.
/// Source-less compatibility sites keep their explicitly authored requirement;
/// this projection does not supply missing opcode or source evidence.
#[must_use]
pub fn native_site_binding_requirement(
    site: &crate::ir::CommandBindingSite,
) -> Option<&tcl_runtime_api::CommandBindingIdentity> {
    site.source_tokens
        .as_deref()
        .is_none_or(|tokens| {
            tokens.source_binding.is_none()
                || proved_native_admitted_inline_operation(tokens).is_some()
        })
        .then_some(&site.binding)
}

/// Original invocation and premises of an already proved native operation.
/// This is a guard/replay plan, not a captured command token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeOperationSelectionPlan {
    /// Exact native operation whose selection the range preserves.
    pub operation: tcl_registry::SemanticOperationId,
    /// Exact source-instance owner; equal bytes and spans cannot substitute.
    pub compilation_site: crate::command_binding::CommandAllocationSite,
    /// Original command and auxiliary implementation lookup premises.
    pub requirements: Vec<tcl_runtime_api::CommandBindingIdentity>,
    /// Actual raw compiler registration selected independently of the handler.
    pub compiler_prerequisite: Option<
        std::sync::Arc<tcl_runtime_api::native_compilation::NativeCommandCompilerPrerequisite>,
    >,
    /// Selection phase, separate from the requirements' validation phases.
    pub guard: tcl_runtime_api::CommandBindingGuard,
    /// Exact original written command for replay before any argument executes.
    pub source: String,
    /// That command's coordinates in its retained source instance.
    pub span: tcl_lexer::Span,
    /// Original rooted command lookup namespace.
    pub namespace: String,
    /// Exact owner used by replay; display spelling cannot reconstruct it.
    pub namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
}

/// A proved operation lacks the original carrier needed for safe replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOperationSelectionDecline {
    /// The singleton selected-operation witness is unavailable.
    MissingOperationWitness,
    /// Original source words cannot be recovered in their retained instance.
    MissingOriginalSource,
    /// Original coordinates cannot represent the complete command.
    MissingSourceCoordinates,
    /// The selected command has no proved static lookup value.
    MissingStaticHead,
}

/// Carry exact original namespace identity across compiler/runtime consumers.
#[doc(hidden)]
#[must_use]
pub fn compiled_namespace_context(
    tokens: &CommandTokens,
) -> Option<tcl_runtime_api::CompiledNamespaceContext> {
    tokens
        .source_binding
        .as_ref()?
        .lookup_namespace_key
        .to_compiled_context()
}

/// Preserve an inline operation's selection across its original arguments.
/// Procedure-header operations use their independent actual-header plan.
/// `Ok(None)` identifies an invocation without an applicable inline plan.
///
/// # Errors
/// Returns the missing carrier when a proved operation cannot retain exact
/// replay. Executable consumers must preserve a provider obligation.
pub fn native_operation_selection_plan(
    tokens: &CommandTokens,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> Result<Option<NativeOperationSelectionPlan>, NativeOperationSelectionDecline> {
    use NativeOperationSelectionDecline as Decline;
    let Some((operation, guard)) = proved_native_admitted_inline_operation(tokens) else {
        return Ok(None);
    };
    if operation
        == tcl_registry::SemanticOperationId::Intrinsic(tcl_registry::IntrinsicId::ProcedureNoOp)
    {
        return Ok(None);
    }
    let binding = tokens
        .source_binding
        .as_ref()
        .ok_or(Decline::MissingOperationWitness)?;
    let selected = binding
        .admitted_inline_invocation()
        .ok_or(Decline::MissingOperationWitness)?;
    let source = native_compiler_replay_source(tokens, &selected.compilation_site)
        .ok_or(Decline::MissingOriginalSource)?;
    let length = u32::try_from(source.len()).map_err(|_| Decline::MissingSourceCoordinates)?;
    let end = selected
        .compilation_site
        .offset
        .checked_add(length)
        .ok_or(Decline::MissingSourceCoordinates)?;
    let guard = native_command_binding_guard(guard);
    let mut requirements = if selected.compiler_prerequisite.is_some() {
        Vec::new()
    } else {
        vec![
            tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
                &binding.lookup_namespace,
                static_command_word(tokens, escapes, word_rules)
                    .ok_or(Decline::MissingStaticHead)?,
                tcl_syntax::naming::unroot_rooted_key(&selected.target.command)
                    .unwrap_or(&selected.target.command),
            )
            .with_namespace_context(compiled_namespace_context(tokens))
            .with_guard(guard),
        ]
    };
    requirements.extend(
        selected
            .lookup_dependencies
            .iter()
            .filter_map(native_implementation_dependency),
    );
    Ok(Some(NativeOperationSelectionPlan {
        operation,
        compilation_site: selected.compilation_site.clone(),
        compiler_prerequisite: selected.compiler_prerequisite.clone(),
        requirements,
        guard,
        source,
        span: tcl_lexer::Span::new(selected.compilation_site.offset, end),
        namespace: binding.lookup_namespace.clone(),
        namespace_context: compiled_namespace_context(tokens),
    }))
}

/// Decode a statically known original command word under the retained grammar.
///
/// This returns a lookup value only. It does not prove a handler, compiler hook,
/// or validation boundary; callers must retain those independent proofs. A
/// dynamic or expanded original word never becomes a static lookup here.
#[must_use]
pub fn static_command_word(
    tokens: &CommandTokens,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> Option<String> {
    match effective_invocation_word(tokens.words().first()?, escapes, word_rules) {
        EffectiveInvocationWord::Literal(value) => Some(value),
        EffectiveInvocationWord::ByteLiteral(_)
        | EffectiveInvocationWord::ArrayElementName { .. }
        | EffectiveInvocationWord::Dynamic
        | EffectiveInvocationWord::Expanded
        | EffectiveInvocationWord::KnownExpansion(_)
        | EffectiveInvocationWord::KnownByteExpansion(_)
        | EffectiveInvocationWord::Opaque => None,
    }
}

/// Project the registry's native compiler validation boundary to the runtime ABI.
/// This shared conversion carries no additional command identity evidence.
#[must_use]
pub const fn native_command_binding_guard(
    guard: tcl_registry::native_compilation::NativeCompilationGuard,
) -> tcl_runtime_api::CommandBindingGuard {
    match guard {
        tcl_registry::native_compilation::NativeCompilationGuard::ChunkEntry => {
            tcl_runtime_api::CommandBindingGuard::ChunkEntry
        }
        tcl_registry::native_compilation::NativeCompilationGuard::BeforeArguments => {
            tcl_runtime_api::CommandBindingGuard::BeforeArguments
        }
    }
}

/// Preserve every auxiliary implementation lookup selected by the exact
/// native invocation proof. These are runtime guard requirements, not source
/// command boundaries or a replacement for the entered command's identity.
#[must_use]
pub fn native_implementation_dependencies(
    tokens: &CommandTokens,
) -> Vec<tcl_runtime_api::CommandBindingIdentity> {
    let Some(binding) = tokens.source_binding.as_ref() else {
        return Vec::new();
    };
    if let Some(named) = binding.admitted_named_invocation() {
        return named
            .dependencies
            .iter()
            .filter_map(native_implementation_dependency)
            .collect();
    }
    binding
        .admitted_inline_invocation()
        .into_iter()
        .flat_map(|proof| &proof.lookup_dependencies)
        .filter_map(native_implementation_dependency)
        .collect()
}

/// One shared conversion for selected-operation and compiler-failure lookup
/// dependencies. The registry owns timing; source proof owns lookup identity.
#[must_use]
pub fn native_implementation_dependency(
    dependency: &crate::command_binding::SourceNativeCompilationDependency,
) -> Option<tcl_runtime_api::CommandBindingIdentity> {
    if dependency.compiler_prerequisite.is_some() {
        return None;
    }
    Some(
        tcl_runtime_api::CommandBindingIdentity::in_rooted_namespace(
            &dependency.namespace,
            &dependency.head,
            tcl_syntax::naming::unroot_rooted_key(&dependency.target.command)
                .unwrap_or(&dependency.target.command),
        )
        .with_namespace_context(dependency.namespace_key.to_compiled_context())
        .with_guard(native_command_binding_guard(dependency.guard)),
    )
}

/// Preserve the actual ordinary or ensemble compiler registration as issued.
#[must_use]
pub fn native_compiler_dependency(
    dependency: &crate::command_binding::SourceNativeCompilationDependency,
) -> Option<tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite> {
    dependency.compiler_prerequisite.as_ref().map(|required| {
        tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite::from_command_registration(
            std::sync::Arc::clone(required),
        )
    })
}

/// Independent registration and handler axes consumed by compilation.
#[doc(hidden)]
pub enum NativeCompilationDependency {
    Compiler(tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite),
    Implementation(tcl_runtime_api::CommandBindingIdentity),
}

#[doc(hidden)]
#[must_use]
pub fn native_compilation_dependency(
    dependency: &crate::command_binding::SourceNativeCompilationDependency,
) -> NativeCompilationDependency {
    if let Some(required) = native_compiler_dependency(dependency) {
        NativeCompilationDependency::Compiler(required)
    } else {
        NativeCompilationDependency::Implementation(
            native_implementation_dependency(dependency).expect("implementation dependency"),
        )
    }
}

fn original_logical_prefix_for_tokens<'a>(
    tokens: &CommandTokens,
    target: &'a crate::command_binding::SourceCommandTarget,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<&'a [crate::signature_scan::scope::SignatureSourceNameInput]> {
    let binding = tokens.source_binding.as_ref()?;
    target.original_logical_prepended_name_inputs(
        input,
        binding.invocation_site()?.source.source_image(),
        binding.original_lexer_config_for_tokens(tokens)?,
    )
}

/// Compose a single candidate using the same ordering as proved dispatch.
pub(crate) fn effective_words_for_target(
    tokens: &CommandTokens,
    target: &crate::command_binding::SourceCommandTarget,
) -> Option<EffectiveCommandWords> {
    if !target.prepended.is_empty()
        && let Some(input) = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_logical_prefix_source_input())
    {
        original_logical_prefix_for_tokens(tokens, target, input)?;
    }
    let mut effective = compose_effective_words(tokens, &target.command, &target.prepended)?;
    if let Some(named) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.named_invocation())
    {
        discard_named_selection_operands(&mut effective, named.arguments_from);
    }
    Some(effective)
}

fn source_effective_words_for_target(
    tokens: &CommandTokens,
    target: &crate::command_binding::SourceCommandTarget,
) -> Option<EffectiveCommandWords> {
    if !target.prepended.is_empty()
        && let Some(input) = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.original_logical_prefix_source_input())
    {
        original_logical_prefix_for_tokens(tokens, target, input)?;
    }
    // Source descriptors retain their selectors; private handler argv does not.
    compose_effective_words(tokens, &target.command, &target.prepended)
}

/// Replay prerequisites for an actually selected empty procedure compiler.
/// Retains opaque callable identity rather than borrowing a registry identity.
pub struct NativeProcedureNoOpPlan {
    /// Actual header compiler and allocation selected before argv.
    pub prerequisite: tcl_runtime_api::native_compilation::NativeProcedureHeaderPrerequisite,
    /// Exact written command in its own source instance.
    pub source: String,
    /// Original command lookup namespace, separate from its slot's namespace.
    pub namespace: String,
    /// Exact original command lookup owner.
    pub namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
}

/// A complete execution/replay plan; unknown source or header evidence declines.
#[must_use]
pub fn native_procedure_noop_plan(tokens: &CommandTokens) -> Option<NativeProcedureNoOpPlan> {
    let binding = tokens.source_binding.as_ref()?;
    if !matches!(
        binding.native_compilation_admission_selection(),
        tcl_registry::native_compilation::NativeCompilationSelection::Inline {
            operation: tcl_registry::SemanticOperationId::Intrinsic(
                tcl_registry::IntrinsicId::ProcedureNoOp
            ),
            ..
        }
    ) {
        return None;
    }
    let selected = binding.admitted_inline_invocation()?;
    let prerequisite = selected.procedure_header_prerequisite.as_deref()?.clone();
    if tokens.words().len() != tokens.argv_texts.len()
        || tokens
            .words()
            .iter()
            .any(|word| matches!(word, WordExpr::Expand { .. } | WordExpr::Opaque { .. }))
    {
        return None;
    }
    let source = native_compiler_replay_source(tokens, &selected.compilation_site)?;
    Some(NativeProcedureNoOpPlan {
        prerequisite,
        source,
        namespace: binding.lookup_namespace.clone(),
        namespace_context: compiled_namespace_context(tokens),
    })
}

/// Recover one exact original written command for compiler-selection replay.
/// This does not grant implementation identity or native compilation eligibility.
#[must_use]
pub fn native_compiler_replay_source(
    tokens: &CommandTokens,
    site: &crate::command_binding::CommandAllocationSite,
) -> Option<String> {
    if tokens.words().len() != tokens.argv_texts.len() {
        return None;
    }
    let text = site.source.try_text().ok()?;
    let first = tokens.words().first()?.original_written_span(text)?;
    let last = tokens.words().last()?.original_written_span(text)?;
    if first.start() != site.offset || first.start() > last.end() {
        return None;
    }
    text.get(first.start() as usize..last.end() as usize)
        .map(str::to_owned)
}

/// Compiler-selected private name with original argument words. This authorizes
/// late dispatch only, including an unknown or absent late handler; it never
/// licenses the original handler's inline opcodes or semantic effects.
#[must_use]
pub fn native_named_command_words(tokens: &CommandTokens) -> Option<EffectiveCommandWords> {
    let named = tokens
        .source_binding
        .as_ref()?
        .admitted_named_invocation()?;
    let mut effective = compose_original_effective_words(tokens, named.captured_name(), &[])?;
    discard_named_selection_operands(&mut effective, named.arguments_from);
    Some(effective)
}

fn discard_named_selection_operands(effective: &mut EffectiveCommandWords, arguments_from: usize) {
    let mut index = 0;
    effective.words.retain(|_| {
        let retain = !matches!(effective.origins[index], InvocationWordOrigin::Written(written) if written <= arguments_from);
        index += 1;
        retain
    });
    effective.origins.retain(|origin| !matches!(origin, InvocationWordOrigin::Written(written) if *written <= arguments_from));
}

fn compose_effective_words(
    tokens: &CommandTokens,
    command: &str,
    prepended: &[EffectiveInvocationWord],
) -> Option<EffectiveCommandWords> {
    let original = compose_original_effective_words(tokens, command, prepended)?;
    let Some(frozen) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.frozen_written_words())
    else {
        return Some(original);
    };
    if frozen.len() != tokens.words().len() {
        return None;
    }
    let mut effective = EffectiveCommandWords {
        words: Vec::new(),
        origins: Vec::new(),
        binding_prefix: original.binding_prefix.clone(),
    };
    for (word, origin) in original.words.into_iter().zip(original.origins) {
        if !matches!(origin, InvocationWordOrigin::Written(_)) {
            effective.words.push(word);
            effective.origins.push(origin);
        }
    }
    let mut needs_head = true;
    for (written, (word, frozen)) in tokens.words().iter().zip(frozen).enumerate() {
        if let Some(count) = frozen.expansion_len() {
            if count == 0 {
                continue;
            }
            append_expansion_elements(
                &mut effective,
                word,
                written,
                frozen,
                usize::from(needs_head),
            );
        } else if matches!(frozen, EffectiveInvocationWord::Expanded) {
            return None;
        } else if !needs_head {
            effective.words.push(word.clone());
            effective
                .origins
                .push(InvocationWordOrigin::Written(written));
        }
        needs_head = false;
    }
    if needs_head {
        return None;
    }
    Some(effective)
}

fn append_expansion_elements(
    effective: &mut EffectiveCommandWords,
    word: &WordExpr,
    written: usize,
    elements: &EffectiveInvocationWord,
    from: usize,
) {
    for element in from..elements.expansion_len().unwrap_or(0) {
        let text = elements
            .expansion_element(element)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .unwrap_or_default();
        effective.words.push(WordExpr::Opaque {
            text: text.to_owned(),
            source: crate::ir::SourceSite::opaque(word.source().span),
            reason: crate::ir::WordOpacity::LossySnapshot,
        });
        effective
            .origins
            .push(InvocationWordOrigin::ExpandedElement { written, element });
    }
}

pub(crate) fn compose_original_effective_words(
    tokens: &CommandTokens,
    command: &str,
    prepended: &[EffectiveInvocationWord],
) -> Option<EffectiveCommandWords> {
    let span = tokens.words().first()?.source().span;
    let mut words = vec![WordExpr::Literal {
        text: command.to_owned(),
        source: crate::ir::SourceSite::opaque(span),
    }];
    let mut origins = vec![InvocationWordOrigin::ResolvedHead];
    for (index, value) in prepended.iter().enumerate() {
        words.push(match value {
            EffectiveInvocationWord::Literal(text) => WordExpr::Literal {
                text: text.clone(),
                source: crate::ir::SourceSite::opaque(span),
            },
            _ => WordExpr::Opaque {
                text: String::new(),
                source: crate::ir::SourceSite::opaque(span),
                reason: crate::ir::WordOpacity::LossySnapshot,
            },
        });
        origins.push(InvocationWordOrigin::BindingPrefix(index));
    }
    for (index, word) in tokens.words().iter().enumerate().skip(1) {
        words.push(word.clone());
        origins.push(InvocationWordOrigin::Written(index));
    }
    Some(EffectiveCommandWords {
        words,
        origins,
        binding_prefix: prepended.to_vec(),
    })
}

/// Values retained before later argument substitutions change interpreter state.
/// Expansion and recovery words cannot acquire a fixed argument cardinality.
fn frozen_argument_values(
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
) -> Vec<Option<String>> {
    effective
        .words
        .iter()
        .zip(&effective.origins)
        .skip(1)
        .map(|(word, origin)| {
            if let InvocationWordOrigin::ExpandedElement { written, element } = origin {
                let values = tokens
                    .source_binding
                    .as_ref()?
                    .frozen_written_words()?
                    .get(*written)?;
                return values
                    .expansion_element(*element)
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    .map(str::to_owned);
            }
            if matches!(word, WordExpr::Expand { .. } | WordExpr::Opaque { .. }) {
                return None;
            }
            match origin {
                InvocationWordOrigin::BindingPrefix(_) => match word {
                    WordExpr::Literal { text, .. } => Some(text.clone()),
                    _ => None,
                },
                InvocationWordOrigin::Written(index) => tokens
                    .source_binding
                    .as_ref()?
                    .evaluated_argument_values
                    .get(index.checked_sub(1)?)
                    .cloned()
                    .flatten(),
                InvocationWordOrigin::ExpandedElement { .. }
                | InvocationWordOrigin::ResolvedHead => None,
            }
        })
        .collect()
}

/// Readonly evaluated arguments aligned with the supplied effective origins.
/// Captured alias operands and retained native expansion children preserve
/// their exact bytes; no value is re-evaluated and no source geometry, handler
/// execution or completion proof is issued by this projection.
#[must_use]
pub fn frozen_argument_words(
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
) -> Vec<EffectiveInvocationWord> {
    effective
        .words
        .iter()
        .zip(&effective.origins)
        .skip(1)
        .map(|(word, origin)| match origin {
            InvocationWordOrigin::Written(index) => tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.evaluated_argument_words.get(index - 1))
                .cloned()
                .unwrap_or_else(|| {
                    if let Some(dialect) = tokens
                        .source_binding
                        .as_ref()
                        .and_then(|binding| binding.variable_context.invocation_dialect)
                    {
                        let value = effective_invocation_word(
                            word,
                            dialect.lexer_grammar.escapes,
                            dialect.word_values,
                        );
                        return if value == EffectiveInvocationWord::Dynamic {
                            tokens
                                .source_binding
                                .as_ref()
                                .and_then(|binding| {
                                    binding.original_compound_array_argument(
                                        tokens,
                                        *index,
                                        dialect.lexer_grammar.escapes,
                                        dialect.word_values,
                                    )
                                })
                                .unwrap_or(value)
                        } else {
                            value
                        };
                    }
                    match invocation_word(word) {
                        InvocationWord::Literal(text) => {
                            EffectiveInvocationWord::Literal(text.to_owned())
                        }
                        InvocationWord::KnownBytes(value) => {
                            EffectiveInvocationWord::from_bytes(value)
                        }
                        InvocationWord::Expanded => EffectiveInvocationWord::Expanded,
                        InvocationWord::Opaque => EffectiveInvocationWord::Opaque,
                        _ => EffectiveInvocationWord::Dynamic,
                    }
                }),
            InvocationWordOrigin::ExpandedElement { written, element } => tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.frozen_written_words())
                .and_then(|words| words.get(*written))
                .and_then(|word| word.expansion_element(*element))
                .map_or(
                    EffectiveInvocationWord::Dynamic,
                    EffectiveInvocationWord::from_bytes,
                ),
            InvocationWordOrigin::BindingPrefix(index) => effective
                .binding_prefix
                .get(*index)
                .cloned()
                .unwrap_or(EffectiveInvocationWord::Dynamic),
            InvocationWordOrigin::ResolvedHead => EffectiveInvocationWord::Dynamic,
        })
        .collect()
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
    /// Alias-composed source words and original argument indices.
    pub effective: EffectiveCommandWords,
}

impl RegistryInvocationShape {
    /// Possible authored body evaluation within this shape. This establishes
    /// no entered frame, callback execution or closed invocation alternatives.
    pub(crate) fn may_execute_parameter_body(&self) -> bool {
        self.possible_traits
            .contains(tcl_registry::Traits::PERFORMS_SUBSTITUTION)
            || self.roles.iter().any(|(_, role)| {
                matches!(
                    role,
                    tcl_registry::ArgRole::Body
                        | tcl_registry::ArgRole::LambdaLiteral
                        | tcl_registry::ArgRole::CommandPrefix
                )
            })
    }

    fn from_hosted_source(
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
    ) -> Option<Self> {
        let metadata = original_hosted_source_registry_metadata(registry, context, tokens)?;
        if metadata
            .authored_barriers()
            .has_nominal_command_name_hazard()
        {
            return None;
        }
        let effective = compose_original_effective_words(tokens, metadata.shape().command(), &[])?;
        let mut shape = metadata
            .shape()
            .with_source_schema(registry, context, |schema| {
                Some(Self::from_facts(&schema.facts(), effective))
            })?;
        shape.hosted_source = Some(std::sync::Arc::new(metadata));
        Some(shape)
    }

    fn from_facts(facts: &InvocationFacts, effective: EffectiveCommandWords) -> Self {
        Self {
            hosted_source: None,
            command: facts.canonical_command.clone(),
            operation: facts.operation,
            data_collection: facts.data_collection,
            side_switch_target: facts.side_switch_target,
            option_keywords: facts.option_keywords.clone(),
            irules_source_flow_action: facts.irules_source_flow_action,
            source_may_commit_response: facts.source_may_commit_response,
            roles: facts.arg_roles.clone(),
            argument_offset: facts.argument_offset,
            roles_complete: facts.arg_roles_complete,
            possible_traits: facts.traits,
            nominal_return_type: facts.return_type,
            effective,
        }
    }

    /// Query conditional source descriptors at this same retained original point.
    /// The selected schema supplies syntax and warning metadata only, never a
    /// live handler, effects, completed reads or Native compilation authority.
    pub(crate) fn with_source_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if let Some(original) = &self.hosted_source {
            if Self::from_hosted_source(registry, context, tokens).as_ref() != Some(self) {
                return None;
            }
            return original
                .shape()
                .with_source_schema(registry, context, apply);
        }
        self.with_source_schema_alternatives(registry, context, tokens, true, apply)
    }

    /// Readonly values under the shape's retained source purpose. Hosted
    /// literals use their own protocol rather than an evaluated Native argv.
    pub(crate) fn source_argument_words(
        &self,
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        tokens: &CommandTokens,
    ) -> Option<Vec<EffectiveInvocationWord>> {
        let Some(_) = &self.hosted_source else {
            return Some(frozen_argument_words(tokens, &self.effective));
        };
        let context = context?;
        let current = Self::from_hosted_source(registry, context, tokens)?;
        if &current != self {
            return None;
        }
        let input = context.source_analysis_input()?;
        let words = source_structure::original_vendor_registry_words(
            input.borrowed_context_registry(),
            tokens,
            current.hosted_source.as_deref()?.clone(),
        )?;
        Some(words.arguments().to_vec())
    }

    /// Logical candidate unions retain possible schemas alongside unknown
    /// handlers. Other source purposes require closed unanimous alternatives.
    fn with_source_candidate_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if !context.permits_logical_source_names()
            || tokens.source_binding.as_ref().and_then(|binding| {
                binding.original_source_advice_purpose(tokens, context, registry)
            }) != Some(crate::command_binding::OriginalInvocationSourcePurpose::Logical)
        {
            return self.with_source_schema(registry, context, tokens, apply);
        }
        if self.effective.words.is_empty()
            || self.effective.words.len() != self.effective.origins.len()
            || self.effective.origins.iter().any(|origin| {
                matches!(origin, InvocationWordOrigin::BindingPrefix(index)
                    if *index >= self.effective.binding_prefix.len())
            })
        {
            return None;
        }
        self.with_source_schema_alternatives(registry, context, tokens, false, apply)
    }

    /// One possible candidate's own schema, for conditional advice that keeps
    /// lookup closure as an open obligation: an unknown residual or possible
    /// absence does not withdraw a candidate that is itself selected.
    pub(crate) fn with_possible_source_schema<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        if self.hosted_source.is_some() {
            return self.with_source_schema(registry, context, tokens, apply);
        }
        // A hosted rule body has no native operand layout; its own hosted
        // catalogue entry describes the same command's schema.
        if context
            .source_analysis_input()
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_hosted_source_name_context)
            && tokens.source_binding.as_ref().is_some_and(|binding| {
                binding
                    .original_conditional_operand_layout_advice(tokens, registry)
                    .is_none()
            })
        {
            let hosted = Self::from_hosted_source(registry, context, tokens)?;
            if hosted.command != self.command {
                return None;
            }
            return hosted
                .hosted_source
                .as_deref()?
                .shape()
                .with_source_schema(registry, context, apply);
        }
        if self.effective.words.is_empty()
            || self.effective.words.len() != self.effective.origins.len()
            || self.effective.origins.iter().any(|origin| {
                matches!(origin, InvocationWordOrigin::BindingPrefix(index)
                    if *index >= self.effective.binding_prefix.len())
            })
        {
            return None;
        }
        self.with_source_schema_alternatives(registry, context, tokens, false, apply)
    }

    fn with_source_schema_alternatives<T>(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
        require_unanimous: bool,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        let binding = tokens.source_binding.as_ref()?;
        let point = binding
            .original_source_metadata_for_advice(tokens, registry)
            .ok()??;
        context.source_analysis_input()?;
        point.source_analysis_input()?;
        if !context.matches_registry(registry)
            || point.context() != context.context()
            || point.source_analysis_input() != context.source_analysis_input()
        {
            return None;
        }
        let assistance = original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(point),
            tokens,
        )?;
        if require_unanimous {
            assistance.unanimous_command_words()?;
        }
        if !assistance.candidates.contains(self) {
            return None;
        }
        let original = binding.original_conditional_operand_layout_advice(tokens, registry)?;
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        let dialect = tcl_registry::InvocationDialect {
            lexer_grammar: config.grammar_over(original.dialect().lexer_grammar),
            word_values: WordValueRules::from_config(&config),
            ..original.dialect()
        };
        let values: Vec<_> = self
            .effective
            .words
            .iter()
            .zip(&self.effective.origins)
            .map(|(word, origin)| match origin {
                InvocationWordOrigin::BindingPrefix(index) => {
                    self.effective.binding_prefix.get(*index).cloned()
                }
                _ => Some(effective_invocation_word(
                    word,
                    config.escapes,
                    dialect.word_values,
                )),
            })
            .collect::<Option<_>>()?;
        let words: Vec<_> = self
            .effective
            .words
            .iter()
            .zip(&values)
            .map(|(word, value)| {
                invocation_word_with_source(word, value, dialect.lexer_grammar.escapes)
            })
            .collect();
        let selected =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                registry,
                Some(point.context()),
                InvocationWords::structured(*words.first()?, &words[1..]).with_dialect(dialect),
                original.realm(),
            );
        let selected = selected.resolved()?;
        (selected.facts().canonical_command == self.command).then_some(())?;
        apply(&selected)
    }

    /// Literal names this original candidate may link into a method frame.
    /// Exact frozen prefix/argument values stay data; this inventory grants no
    /// Native cell, handler, completed store, observer absence or erasure proof.
    #[must_use]
    pub fn possible_method_instance_variable_names(
        &self,
        registry: &CommandRegistry,
        context: InvocationMetadataContext<'_>,
        tokens: &CommandTokens,
    ) -> Option<Vec<String>> {
        let current = original_registry_invocation_assistance_with_metadata_context(
            registry,
            Some(context),
            tokens,
        )?;
        if current.unknown_residual || current.may_be_absent || !current.candidates.contains(self) {
            return None;
        }
        if self.operation
            != tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Variable,
            )
        {
            return Some(Vec::new());
        }
        if !self.roles_complete {
            return None;
        }
        let words = frozen_argument_words(tokens, &self.effective);
        self.roles
            .iter()
            .filter(|(_, role)| *role == tcl_registry::ArgRole::VarWrite)
            .map(|(argument, _)| {
                let index = self.argument_offset.checked_add(usize::from(*argument))?;
                match words.get(index)? {
                    EffectiveInvocationWord::Literal(value) => Some(value.clone()),
                    EffectiveInvocationWord::ByteLiteral(value) => {
                        std::str::from_utf8(value).ok().map(str::to_owned)
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Original written operands mentioned by this possible candidate.
    /// These roles grant assistance only, not reads, stores or body execution.
    #[must_use]
    pub fn written_argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        mapped_written_argument_roles(&self.effective, self.argument_offset, &self.roles)
    }
}

fn mapped_written_argument_roles(
    effective: &EffectiveCommandWords,
    argument_offset: usize,
    roles: &[(u8, tcl_registry::ArgRole)],
) -> Vec<(usize, tcl_registry::ArgRole)> {
    let mut mapped = Vec::new();
    for &(index, role) in roles {
        if let Some(effective_index) = argument_offset.checked_add(usize::from(index))
            && let Some(written) = effective.written_argument(effective_index)
            && !mapped.contains(&(written, role))
        {
            mapped.push((written, role));
        }
    }
    mapped
}

/// Advisory envelope retaining every uncertainty of shared command lookup.
/// Consumers may explain candidates but must not infer stores or optimise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryInvocationAssistance {
    /// Registry candidates for which nominal metadata is available.
    pub candidates: Vec<RegistryInvocationShape>,
    /// At least one possible implementation has no registry shape.
    pub unknown_residual: bool,
    /// Lookup can fail at runtime.
    pub may_be_absent: bool,
}

/// Command words shared by every closed advisory lookup alternative.
/// This projection grants no execution, role, mutation or completion facts.
#[derive(Debug, Clone, Copy)]
pub struct AdvisoryCommandWords<'a> {
    command: &'a str,
    effective: &'a EffectiveCommandWords,
}

impl AdvisoryCommandWords<'_> {
    /// Registry command identity shared by the advisory alternatives.
    #[must_use]
    pub fn command(&self) -> &str {
        self.command
    }

    /// Original words, alias prefix and origins shared by all alternatives.
    #[must_use]
    pub fn effective(&self) -> &EffectiveCommandWords {
        self.effective
    }
}

impl RegistryInvocationAssistance {
    /// Project unanimous advisory words without selecting an implementation.
    /// Any absent, unknown or disagreeing lookup alternative withdraws the view.
    #[must_use]
    pub fn unanimous_command_words(&self) -> Option<AdvisoryCommandWords<'_>> {
        if self.unknown_residual || self.may_be_absent {
            return None;
        }
        let first = self.candidates.first()?;
        if first.effective.words.is_empty()
            || first.effective.words.len() != first.effective.origins.len()
            || first.effective.origins.iter().any(|origin| {
                matches!(origin, InvocationWordOrigin::BindingPrefix(index)
                    if *index >= first.effective.binding_prefix.len())
            })
        {
            return None;
        }
        self.candidates
            .iter()
            .all(|candidate| {
                candidate.command == first.command && candidate.effective == first.effective
            })
            .then_some(AdvisoryCommandWords {
                command: &first.command,
                effective: &first.effective,
            })
    }
}

/// A possible value-store shape for assistance. It cannot establish a write,
/// a variable's runtime type, or a constant-propagation fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvisoryValueAssignment {
    /// Literal destination mentioned by the candidate operation.
    pub name: String,
    /// Authored or alias-inserted value word.
    pub value: WordExpr,
}

/// Project possible scalar value stores from the shared May envelope. A
/// document procedure named `set` contributes no stock assignment contract.
#[must_use]
pub fn advisory_value_assignments(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    advisory_value_assignments_with_metadata_context(registry, None, tokens)
}

/// Possible scalar stores under the complete supplied availability generation.
/// The May envelope supplies no actual write, type or constant value.
#[must_use]
pub fn advisory_value_assignments_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    advisory_value_assignments_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn advisory_value_assignments_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    let Some(assistance) =
        registry_invocation_assistance_with_metadata_context(registry, context, tokens)
    else {
        return Vec::new();
    };
    let dialect = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .or_else(|| {
            context.map_or_else(
                || {
                    registry
                        .profile()
                        .map(tcl_registry::InvocationDialect::of_profile)
                },
                |context| {
                    context
                        .context()
                        .environment
                        .point()
                        .map(tcl_registry::InvocationDialect::of_point)
                },
            )
        });
    let Some(dialect) = dialect else {
        return Vec::new();
    };
    assistance
        .candidates
        .into_iter()
        .filter_map(|shape| {
            if shape.operation
                != tcl_registry::SemanticOperationId::StructuredLowering(
                    tcl_registry::hooks::LoweringHookId::Set,
                )
                || shape.effective.words.len() != 3
            {
                return None;
            }
            Some(AdvisoryValueAssignment {
                name: shape.effective.argument_literal(
                    0,
                    dialect.lexer_grammar.escapes,
                    dialect.word_values,
                )?,
                value: shape.effective.words.get(2)?.clone(),
            })
        })
        .collect()
}

/// Possible original value stores under the complete selected source schema.
/// No handler, completed write, native cell or value-version fact follows.
pub(crate) fn original_advisory_value_assignments_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Vec<AdvisoryValueAssignment> {
    let Some(context) = context.filter(|context| context.permits_logical_source_names()) else {
        return Vec::new();
    };
    let Some(assistance) = original_registry_invocation_assistance_with_metadata_context(
        registry,
        Some(context),
        tokens,
    ) else {
        return Vec::new();
    };
    assistance
        .candidates
        .into_iter()
        .filter_map(|candidate| {
            candidate.with_source_candidate_schema(registry, context, tokens, |schema| {
                (schema.facts().operation
                    == tcl_registry::SemanticOperationId::StructuredLowering(
                        tcl_registry::hooks::LoweringHookId::Set,
                    )
                    && candidate.effective.words.len() == 3)
                    .then_some(())?;
                Some(AdvisoryValueAssignment {
                    name: schema
                        .words
                        .arguments()
                        .literal_at(schema.semantics.argument_offset)?
                        .to_owned(),
                    value: candidate.effective.words.get(2)?.clone(),
                })
            })
        })
        .collect()
}

/// Conditional object factory metadata at one original source point.
/// Unknown receiver contents, handler and completion alternatives remain open.
#[derive(Clone, Debug)]
pub(crate) struct AdvisoryObjectFactory {
    pub(crate) publication: Option<(String, &'static str)>,
    pub(crate) result_class: Option<&'static str>,
}

/// Original selected factory source schemas for advisory object-class unions.
/// A named publication requires the descriptor's explicit source layout; an
/// unsupported optional/control layout supplies no name. Callable result class
/// metadata remains independent of that named publication. Neither is a store,
/// allocation, physical read, runtime class, completed handler or dispatch proof.
pub(crate) fn advisory_object_factories_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Vec<AdvisoryObjectFactory> {
    let Some(assistance) = original_registry_invocation_assistance_with_metadata_context(
        registry,
        Some(context),
        tokens,
    ) else {
        return Vec::new();
    };
    assistance
        .candidates
        .into_iter()
        .filter_map(|candidate| {
            candidate.with_source_candidate_schema(registry, context, tokens, |schema| {
                (schema.facts().arity_accepts_frozen_arguments() == Some(true)).then_some(())?;
                let publication =
                    schema
                        .authored_source_command_publication()
                        .and_then(|publication| {
                            let tcl_registry::AuthoredSourceCommandPublicationKind::Instance {
                                class_name,
                            } = publication.kind
                            else {
                                return None;
                            };
                            Some((
                                schema
                                    .words
                                    .arguments()
                                    .literal_at(publication.argument)?
                                    .to_owned(),
                                class_name,
                            ))
                        });
                let result_class = schema.authored_source_callable_factory_class();
                (publication.is_some() || result_class.is_some()).then_some(AdvisoryObjectFactory {
                    publication,
                    result_class,
                })
            })
        })
        .collect()
}

/// Possible original operands of a default return operation. These are
/// result candidates only: no completion, returned type or reached caller is
/// established. Option-bearing forms require their own return-options recipe.
#[must_use]
pub fn advisory_return_values(registry: &CommandRegistry, tokens: &CommandTokens) -> Vec<WordExpr> {
    advisory_return_values_with_metadata_context(registry, None, tokens)
}

/// Possible return operands selected under the retained availability context.
/// These source candidates provide no completion or returned runtime type.
#[must_use]
pub fn advisory_return_values_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Vec<WordExpr> {
    advisory_return_values_with_metadata_context(context.commands(), Some(context.into()), tokens)
}

pub(crate) fn advisory_return_values_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Vec<WordExpr> {
    let mut values = Vec::new();
    for shape in registry_invocation_assistance_with_metadata_context(registry, context, tokens)
        .into_iter()
        .flat_map(|assistance| assistance.candidates)
    {
        if shape.operation
            == tcl_registry::SemanticOperationId::StructuredLowering(
                tcl_registry::hooks::LoweringHookId::Return,
            )
            && shape.effective.words.len() == 2
            && let Some(value) = shape.effective.words.get(1)
            && !values.contains(value)
        {
            values.push(value.clone());
        }
    }
    values
}

/// Nominal catalogue assistance for an unchanged logical lookup slot. The
/// provider need not be loaded; this cannot establish actual result types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueInvocationAssistance {
    /// Slot applicability established by the source lookup owner.
    pub candidate: crate::command_binding::DeclaredCommandCandidate,
    /// Available nominal metadata, selected under the document's dialect.
    pub shape: RegistryInvocationShape,
}

/// Obtain catalogue assistance separately from execution proofs. A command
/// mutation, document definition or unknown lookup invalidates applicability;
/// catalogue availability alone never restores the live implementation.
#[must_use]
pub fn catalogue_invocation_assistance<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<CatalogueInvocationAssistance> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let candidate = binding.catalogue_command.as_ref()?;
    context.context().resolve_spec(registry, &candidate.name)?;
    let effective = compose_effective_words(tokens, &candidate.name, &[])?;
    let RegistryInvocationResolution::Resolved(facts) =
        resolve_effective_tokens_with_metadata_context(
            registry,
            Some(context),
            tokens,
            &effective,
            binding.variable_context.invocation_dialect,
        )
        .ok()?
    else {
        return None;
    };
    Some(CatalogueInvocationAssistance {
        candidate: candidate.clone(),
        shape: RegistryInvocationShape::from_facts(&facts, effective),
    })
}

/// Query possible registry shapes without upgrading a May binding into Must.
/// Document implementations retain an unknown residual and cannot inherit a
/// stock command contract from their written name.
#[must_use]
pub fn registry_invocation_assistance(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    registry_invocation_assistance_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Query possible Registry shapes under the retained availability generation.
/// The original binding supplies candidates and unresolved alternatives; this
/// context neither selects a candidate nor supplies missing execution proof.
#[must_use]
pub fn registry_invocation_assistance_in_context(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    registry_invocation_assistance_with_metadata_context(
        context.commands(),
        Some(context.into()),
        tokens,
    )
}

pub(crate) fn registry_invocation_assistance_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    registry_invocation_assistance_with_metadata_context_using(
        registry,
        context,
        tokens,
        effective_words_for_target,
    )
}

fn registry_invocation_assistance_with_metadata_context_using(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
    compose: fn(
        &CommandTokens,
        &crate::command_binding::SourceCommandTarget,
    ) -> Option<EffectiveCommandWords>,
) -> Option<RegistryInvocationAssistance> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let dialect = binding.variable_context.invocation_dialect.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    let mut assistance = RegistryInvocationAssistance {
        candidates: Vec::new(),
        unknown_residual: binding.execution_is_unknown(),
        may_be_absent: binding.execution_may_be_absent(),
    };
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            assistance.unknown_residual = true;
            continue;
        }
        let effective = compose(tokens, target)?;
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry, context, tokens, &effective, dialect,
            )
        else {
            assistance.unknown_residual = true;
            continue;
        };
        assistance
            .candidates
            .push(RegistryInvocationShape::from_facts(&facts, effective));
    }
    Some(assistance)
}

/// Readonly Registry metadata at an unchanged original command site.
/// A unanimous retained execution envelope is preferred. Otherwise the genuine
/// original declaration layout may supply conditional command words before
/// operand evaluation, including an uncalled procedure body. Runtime lookup
/// uncertainty, Normal completion, stores and compiler admission are separate;
/// this projection changes none of their receipts.
#[must_use]
pub fn original_registry_invocation_assistance(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    original_registry_invocation_assistance_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Read conditional original Registry syntax with the supplied metadata owner.
/// Source consumers retain complete input and reject missing ownership before
/// calling; this readonly advice supplies no entered invocation or edit grant.
#[must_use]
pub fn original_registry_invocation_assistance_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    // Implementation contract: naming.source.original-registry-header-advice
    // docs/design/analysis/name-resolution-proofs/original-registry-header-advice.md
    tokens
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(tokens)?;
    let reached = registry_invocation_assistance_with_metadata_context_using(
        registry,
        context,
        tokens,
        source_effective_words_for_target,
    );
    if reached
        .as_ref()
        .is_some_and(|advice| advice.unanimous_command_words().is_some())
    {
        return reached;
    }
    if let Some(context) = context
        && context
            .source_analysis_input()
            .is_some_and(crate::analyser::ResolvedAnalysisInput::has_hosted_source_name_context)
    {
        let mut reached = reached?;
        // In the fresh F5 world the original command table answers first;
        // the hosted catalogue is advice only where that lookup has nothing.
        if !reached.candidates.is_empty()
            && tokens.source_binding.as_ref().is_some_and(
                crate::command_binding::SourceInvocationBinding::in_hosted_rule_loader_world,
            )
        {
            return Some(reached);
        }
        let Some(candidate) =
            RegistryInvocationShape::from_hosted_source(registry, context, tokens)
        else {
            return Some(reached);
        };
        if !reached.candidates.contains(&candidate) {
            reached.candidates.push(candidate);
        }
        return Some(reached);
    }
    let context = context
        .or_else(|| body_assistance_context(registry, tokens).map(InvocationMetadataContext::from));
    let declared = context
        .and_then(|context| original_declared_structured_invocation(registry, context, tokens));
    let Some(declared) = declared else {
        return reached;
    };
    Some(RegistryInvocationAssistance {
        candidates: vec![RegistryInvocationShape::from_facts(
            &declared.facts,
            declared.effective,
        )],
        unknown_residual: false,
        may_be_absent: false,
    })
}

/// Possible Registry grammar in the unchanged literal prefix of an open quoted
/// procedure body. The shared source owner retains a distinct incomplete header
/// observation; this creates no complete body, installed definition, formal
/// activation, executable invocation, Normal or native compiler capability.
#[must_use]
pub fn original_incomplete_body_registry_invocation_assistance(
    registry: &CommandRegistry,
    realm: &crate::realm::CommandBindingRealm,
    tokens: &CommandTokens,
) -> Option<RegistryInvocationAssistance> {
    // Implementation contract: naming.source.incomplete-body-header-metadata
    // docs/design/analysis/name-resolution-proofs/incomplete-body-header-metadata.md
    let advice = realm
        .source_bindings_ref()
        .incomplete_body_operand_layout_advice(tokens, registry)?;
    let dialect = advice.dialect();
    let mut candidates = Vec::new();
    for target in advice.targets() {
        let effective = source_effective_words_for_target(tokens, target)?;
        let values: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm(registry, None, &words, Some(dialect), advice.realm())
                .ok()?
        else {
            return None;
        };
        candidates.push(RegistryInvocationShape::from_facts(&facts, effective));
    }
    (!candidates.is_empty()).then_some(RegistryInvocationAssistance {
        candidates,
        unknown_residual: false,
        may_be_absent: false,
    })
}

/// Source role assistance from retained lookup candidates and applicable
/// declarations. Original operand mapping is owned here; captured alias
/// prefixes and expanded values cannot manufacture editable source indices.
/// This union preserves lookup uncertainty and grants no execution, read,
/// store, completion or optimisation proof.
#[must_use]
pub fn invocation_argument_role_assistance<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
) -> Vec<(usize, tcl_registry::ArgRole)> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return Vec::new();
    }
    let mut roles = Vec::new();
    if let Some(assistance) =
        registry_invocation_assistance_with_metadata_context(registry, Some(context), tokens)
    {
        for candidate in assistance.candidates {
            roles.extend(candidate.written_argument_roles());
        }
    }
    if let Some(catalogue) = catalogue_invocation_assistance(registry, context, tokens) {
        roles.extend(catalogue.shape.written_argument_roles());
    }
    if let Some(declared) = resolved_declared_assistance(surface, tokens) {
        roles.extend(declared.roles);
    }
    let mut unique = Vec::new();
    for role in roles {
        if !unique.contains(&role) {
            unique.push(role);
        }
    }
    unique.sort_by_key(|(index, _)| *index);
    unique
}

/// Definite syntactic roles shared by every actual retained implementation.
/// An opaque, absent, incomplete or unaccepted alternative withdraws the
/// entire projection. Catalogue and declared navigation do not establish a
/// grammar obligation. This grants neither execution nor successful effects.
#[must_use]
pub fn invocation_argument_role_consensus<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Vec<(usize, tcl_registry::ArgRole)> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return Vec::new();
    }
    let Some(binding) = tokens.source_binding.as_ref() else {
        return Vec::new();
    };
    if tokens.synthetic.is_some()
        || binding.execution_is_unknown()
        || binding.execution_may_be_absent()
    {
        return Vec::new();
    }
    let dialect = binding.variable_context.invocation_dialect;
    let mut consensus: Option<Vec<(usize, tcl_registry::ArgRole)>> = None;
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            return Vec::new();
        }
        let Some(effective) = effective_words_for_target(tokens, target) else {
            return Vec::new();
        };
        let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry,
                Some(context),
                tokens,
                &effective,
                dialect,
            )
        else {
            return Vec::new();
        };
        if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
            return Vec::new();
        }
        let roles =
            mapped_written_argument_roles(&effective, facts.argument_offset, &facts.arg_roles);
        if let Some(shared) = &mut consensus {
            shared.retain(|role| roles.contains(role));
        } else {
            consensus = Some(roles);
        }
    }
    consensus.unwrap_or_default()
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

impl ParameterRoleAdvice {
    pub(crate) fn is_closed(&self) -> bool {
        self.closed
    }

    pub(crate) fn candidates(&self) -> &[RegistryInvocationShape] {
        &self.candidates
    }

    pub(crate) fn written_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        self.candidates
            .iter()
            .flat_map(RegistryInvocationShape::written_argument_roles)
            .collect()
    }

    pub(crate) fn may_execute_body(&self) -> bool {
        self.candidates
            .iter()
            .any(RegistryInvocationShape::may_execute_parameter_body)
    }
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
impl BodyParameterSourceInvocation {
    fn matches_parameters(&self, parameters: &[&str]) -> bool {
        self.entry.matches_parameters(parameters)
    }
}

/// Borrowed conditional source parameter layout from its genuine retained
/// body/prefix owner. Ordinary invocation facts retain their own proof purpose.
/// This facade supplies no closed advice, Native handler or Normal operation.
pub(crate) struct ConditionalParameterSourceInvocation<'a> {
    invocation: &'a ResolvedStatementInvocation,
    shape: &'a RegistryInvocationShape,
    pub(crate) facts: &'a tcl_registry::InvocationFacts,
}

impl<'a> ConditionalParameterSourceInvocation<'a> {
    pub(crate) fn written_argument_roles(&self) -> Vec<(usize, tcl_registry::ArgRole)> {
        mapped_written_argument_roles(
            &self.invocation.effective,
            self.facts.argument_offset,
            &self.shape.roles,
        )
    }

    /// The original data-only invocation for the existing body-owner-checked
    /// alias/template readers. Its ordinary facts are unchanged.
    pub(crate) fn retained_invocation(&self) -> &'a ResolvedStatementInvocation {
        self.invocation
    }
}

struct ConditionalBodyInvocation {
    entry: std::sync::Arc<crate::command_binding::SourceConditionalBodyEntry>,
    invocation: ResolvedStatementInvocation,
}

pub(crate) enum ConditionalParameterSourceAliasLayout {
    Unavailable,
    Available((tcl_registry::FrameLevel, usize)),
}

impl ConditionalParameterSourceAliasLayout {
    pub(crate) fn into_option(self) -> Option<(tcl_registry::FrameLevel, usize)> {
        match self {
            Self::Unavailable => None,
            Self::Available(layout) => Some(layout),
        }
    }
}

impl InvocationBodyAssistance {
    /// Conditional roles from the authentic enclosing Logical lambda source.
    /// This projection never populates ordinary parameter advice.
    pub(crate) fn conditional_logical_lambda_roles(
        &self,
    ) -> Option<Vec<(usize, tcl_registry::ArgRole)>> {
        let owned = self.logical_lambda_roles.as_ref()?;
        let source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            owned.words.source()
        else {
            return None;
        };
        (owned
            .lambda
            .matches_source(owned.lambda.lambda().image(), &owned.input)
            && owned.scope.matches_input(&owned.input)
            && advice.logical_source_input() == Some(&owned.input)
            && owned.invocation.effective == owned.shape.effective)
            .then(|| owned.shape.written_argument_roles())
    }

    pub(crate) fn conditional_logical_lambda_shape(&self) -> Option<&RegistryInvocationShape> {
        let owned = self.logical_lambda_roles.as_ref()?;
        let source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            owned.words.source()
        else {
            return None;
        };
        (owned
            .lambda
            .matches_source(owned.lambda.lambda().image(), &owned.input)
            && owned.scope.matches_input(&owned.input)
            && advice.logical_source_input() == Some(&owned.input)
            && owned.invocation.effective == owned.shape.effective)
            .then_some(&owned.shape)
    }

    /// Retain the selected nested lambda under the same source owner and
    /// ancestry as the current logical body.
    pub(crate) fn logical_trait_caller_lambda_owner(
        &self,
        argument: usize,
    ) -> Option<LogicalLambdaSourceOwner> {
        let (input, words, parent_scope) = if let Some(owned) = &self.logical_trait_invocation {
            (
                &owned.input,
                &owned.words,
                std::sync::Arc::clone(owned.body.scope()),
            )
        } else {
            let owned = self.logical_lambda_roles.as_ref()?;
            (
                &owned.input,
                &owned.words,
                std::sync::Arc::clone(&owned.scope),
            )
        };
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            advice,
        ) = words.source()
        else {
            return None;
        };
        let policy = crate::command_binding::source_input_dialect(input).authored_name_policy()?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let lambda =
            std::sync::Arc::new(crate::command_binding::OriginalLogicalLambdaBody::capture(
                input,
                words.clone(),
                advice.original_words(),
                purpose,
            )?);
        let selected_word = words.original_argument_word(argument)?;
        if lambda.lambda() != selected_word {
            return None;
        }
        let scope = std::sync::Arc::new(
            crate::command_binding::OriginalDeclaredLogicalBodyContext::capture_lambda(
                lambda.clone(),
                input,
                Some(parent_scope),
            )?,
        );
        lambda.frame_source(input)?;
        Some(LogicalLambdaSourceOwner { lambda, scope })
    }

    fn retain_caller_template_source(&mut self, tokens: &CommandTokens) {
        self.caller_template_source = self.caller_name_template_invocation().map(|invocation| {
            Box::new(CallerTemplateSource {
                tokens: tokens.clone(),
                invocation: invocation.clone(),
            })
        });
    }

    /// Literal data from this retained invocation's original Written operand
    /// or actual captured prefix value. No name, frame or execution follows.
    pub(crate) fn caller_template_literal_data(
        &self,
        selected: &ResolvedStatementInvocation,
        argument: usize,
    ) -> Option<String> {
        (self
            .caller_name_template_invocation()
            .is_some_and(|actual| std::ptr::eq(actual, selected))
            || self.conditional_parameter_source().is_some_and(|owned| {
                std::ptr::eq(std::ptr::from_ref(&owned.invocation), selected)
            })
            || self.body_parameter_source.as_ref().is_some_and(|owned| {
                std::ptr::eq(std::ptr::from_ref(&owned.invocation), selected)
            }))
        .then_some(())?;
        if let Some(owned) = &self.body_parameter_source
            && std::ptr::eq(std::ptr::from_ref(&owned.invocation), selected)
        {
            let InvocationWordOrigin::Written(written) =
                selected.effective.origins.get(argument.checked_add(1)?)?
            else {
                return None;
            };
            let word = owned.words.get(*written)?;
            let value = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
            return (!value.contains(&0))
                .then(|| String::from_utf8(value).ok())
                .flatten();
        }
        if let Some(owned) = &self.logical_trait_invocation
            && std::ptr::eq(std::ptr::from_ref(&owned.invocation), selected)
        {
            let word = owned.words.original_argument_word(argument)?;
            let value = tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
            return (!value.contains(&0))
                .then(|| String::from_utf8(value).ok())
                .flatten();
        }
        let source = self.caller_template_source.as_deref()?;
        (&source.invocation == selected).then_some(())?;
        match selected.effective.origins.get(argument.checked_add(1)?)? {
            InvocationWordOrigin::Written(written) => {
                original_literal_argument_data(&source.tokens, written.checked_sub(1)?)
            }
            InvocationWordOrigin::BindingPrefix(_) => {
                let value = selected.evaluated_words.get(argument)?.literal_bytes()?;
                (!value.contains(&0)).then_some(())?;
                Some(std::str::from_utf8(value).ok()?.to_owned())
            }
            InvocationWordOrigin::ResolvedHead | InvocationWordOrigin::ExpandedElement { .. } => {
                None
            }
        }
    }

    pub(crate) fn parameter_role_advice(&self) -> &ParameterRoleAdvice {
        &self.parameter_advice
    }

    /// The own-body caller-name layout is a symbolic callee template. It
    /// identifies no caller cell and authorises no successful alias or store.
    pub(crate) fn caller_name_template_invocation(&self) -> Option<&ResolvedStatementInvocation> {
        self.definite_invocation
            .as_ref()
            .or_else(|| {
                self.conditional_invocation
                    .as_ref()
                    .map(|conditional| &conditional.invocation)
            })
            .or_else(|| {
                self.receiver_trait_invocation
                    .as_ref()
                    .map(|owned| &owned.invocation)
            })
            .or_else(|| {
                self.logical_trait_invocation
                    .as_ref()
                    .filter(|owned| owned.source_parameter_shape.is_none())
                    .map(|owned| &owned.invocation)
            })
    }

    fn conditional_parameter_source(&self) -> Option<&LogicalTraitInvocation> {
        let owned = self.logical_trait_invocation.as_ref()?;
        owned.source_parameter_shape.as_ref()?;
        let source_structure::OriginalRegistrySource::SourceTransitions(advice) =
            owned.words.source()
        else {
            return None;
        };
        let prefix = owned
            .body
            .parameter_source_prefix(advice.original_words())?;
        prefix
            .matches_body(&owned.body, &owned.input)
            .then_some(())?;
        (prefix.current() == Some(advice.as_ref())).then_some(owned)
    }

    /// Exact conditional own-formal source shape. Original observer/lookup
    /// applicability stays retained on the body prefix, independently of the
    /// closed execution candidate advice and successful caller operations.
    pub(crate) fn conditional_parameter_source_shape(
        &self,
        parameters: &[&str],
    ) -> Option<&RegistryInvocationShape> {
        if self.logical_trait_invocation.is_none() {
            let owned = self.body_parameter_source.as_ref()?;
            return owned.matches_parameters(parameters).then_some(&owned.shape);
        }
        let owned = self.conditional_parameter_source()?;
        owned
            .body
            .parameters()
            .iter()
            .map(|formal| formal.name.as_str())
            .eq(parameters.iter().copied())
            .then_some(())?;
        owned.source_parameter_shape.as_ref()
    }

    /// Source-only symbolic operand layout. This is never returned by the
    /// generic parameter/caller-template invocation or definite accessors.
    pub(crate) fn conditional_parameter_source_invocation(
        &self,
        parameters: &[&str],
    ) -> Option<ConditionalParameterSourceInvocation<'_>> {
        let shape = self.conditional_parameter_source_shape(parameters)?;
        let invocation = if self.logical_trait_invocation.is_some() {
            &self.conditional_parameter_source()?.invocation
        } else {
            &self.body_parameter_source.as_ref()?.invocation
        };
        Some(ConditionalParameterSourceInvocation {
            facts: &invocation.facts,
            invocation,
            shape,
        })
    }

    /// Conditional source alias layout from the same retained Registry source
    /// schema. None means this is not that purpose; Unavailable preserves a
    /// missing layout without falling through to Native success rules.
    pub(crate) fn conditional_parameter_source_alias_layout(
        &self,
        selected: &ResolvedStatementInvocation,
    ) -> Option<ConditionalParameterSourceAliasLayout> {
        let owned = self.conditional_parameter_source()?;
        std::ptr::eq(std::ptr::from_ref(&owned.invocation), selected).then_some(())?;
        Some(
            owned
                .words
                .with_source_schema(owned.input.borrowed_context_registry(), |schema| {
                    let aliases = schema.authored_logical_source_variable_aliases()?;
                    if let Some(first) = aliases
                        .iter()
                        .filter_map(|alias| {
                            matches!(
                                alias.target,
                                tcl_registry::VariableAliasTarget::Namespace { .. }
                            )
                            .then_some(alias.words.target)
                        })
                        .min()
                    {
                        return Some((tcl_registry::FrameLevel::Dynamic, first));
                    }
                    (schema.semantics.frame_effect?.layout
                        == tcl_registry::FrameArgLayout::AliasPairs)
                        .then_some(())?;
                    match schema.authored_logical_source_frame_argument_layout()? {
                        tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                            level,
                            level_word_len,
                        } => Some((level, level_word_len)),
                        tcl_registry::frame_effect::FrameArgumentResolution::Unknown
                        | tcl_registry::frame_effect::FrameArgumentResolution::Invalid => None,
                    }
                })
                .flatten()
                .map_or(
                    ConditionalParameterSourceAliasLayout::Unavailable,
                    ConditionalParameterSourceAliasLayout::Available,
                ),
        )
    }

    /// The actual original descriptor decides whether this script operand
    /// inherits the caller source context. A deferred/fresh-frame body cannot
    /// donate its locals to the enclosing procedure's parameter template.
    pub(crate) fn logical_trait_caller_body(
        &self,
        argument: usize,
    ) -> Option<OriginalSourceScriptBody> {
        let (words, input) = if let Some(owned) = &self.logical_trait_invocation {
            (&owned.words, &owned.input)
        } else {
            let owned = self.logical_lambda_roles.as_ref()?;
            (&owned.words, &owned.input)
        };
        let word = words.original_argument_word(argument)?;
        let context = input.borrowed_context_registry();
        let mut bodies = words
            .source_script_bodies(context)
            .into_iter()
            .filter(|body| body.original_container() == word);
        let body = bodies.next()?;
        (bodies.next().is_none() && body.inherits_immediate_caller_source_context(context))
            .then_some(body)
    }

    /// Exact source lambda body selected by this same Logical invocation.
    /// This retains only authored list geometry; it does not establish an
    /// entered lambda frame or parameter binding.
    pub(crate) fn logical_trait_caller_lambda(
        &self,
        argument: usize,
    ) -> Option<OriginalSourceScriptBody> {
        let (words, input) = if let Some(owned) = &self.logical_trait_invocation {
            (&owned.words, &owned.input)
        } else {
            let owned = self.logical_lambda_roles.as_ref()?;
            (&owned.words, &owned.input)
        };
        let word = words.original_argument_word(argument)?;
        let context = input.borrowed_context_registry();
        let body = words.original_lambda_body_geometry(context)?;
        (body.original_container() == word
            && body.matches_source(word.image(), input.lexer_config()))
        .then_some(body)
    }

    /// Symbolic local-copy and alias traits use the declaration's original
    /// formal recipe. Symbolic caller-name roles may use it too; actual caller
    /// cells and effects require entered-frame proof and `definite_invocation`.
    pub(crate) fn parameter_invocation(
        &self,
        parameters: &[&str],
    ) -> Option<&ResolvedStatementInvocation> {
        self.definite_invocation
            .as_ref()
            .or_else(|| {
                let conditional = self.conditional_invocation.as_ref()?;
                conditional
                    .entry
                    .matches_parameters(parameters)
                    .then_some(&conditional.invocation)
            })
            .or_else(|| {
                let owned = self.receiver_trait_invocation.as_ref()?;
                owned
                    .entry
                    .matches_trait_parameters(parameters)
                    .then_some(&owned.invocation)
            })
            .or_else(|| {
                let owned = self.logical_trait_invocation.as_ref()?;
                if owned.source_parameter_shape.is_some() {
                    return None;
                }
                let formal_names = owned
                    .body
                    .parameters()
                    .iter()
                    .map(|formal| formal.name.as_str())
                    .collect::<Vec<_>>();
                (formal_names == parameters).then_some(&owned.invocation)
            })
    }
}

/// Heap-owned only during this original invocation's preparation. Different
/// May and exact-actual dialect inputs retain independent selected facts.
struct PreparedBodyCandidate {
    effective: EffectiveCommandWords,
    possible_facts: Box<InvocationFacts>,
    exact_facts: Option<Box<InvocationFacts>>,
    same_dialect: bool,
    selected_execution: bool,
}

impl PreparedBodyCandidate {
    fn exact_facts(&self) -> Option<&InvocationFacts> {
        if self.same_dialect {
            Some(&self.possible_facts)
        } else {
            self.exact_facts.as_deref()
        }
    }
}

struct PreparedBodyCandidates {
    candidates: Vec<PreparedBodyCandidate>,
    unknown_residual: bool,
    may_be_absent: bool,
}

#[inline(never)]
fn prepare_body_candidates<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<Box<PreparedBodyCandidates>> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let actual = binding.variable_context.invocation_dialect;
    let possible = actual.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    let mut prepared = Box::new(PreparedBodyCandidates {
        candidates: Vec::new(),
        unknown_residual: binding.execution_is_unknown(),
        may_be_absent: binding.execution_may_be_absent(),
    });
    for target in binding
        .execution_targets()
        .chain(binding.proved_handler_target())
    {
        if !target.registry_backed {
            prepared.unknown_residual = true;
            continue;
        }
        let effective = effective_words_for_target(tokens, target)?;
        let Ok(RegistryInvocationResolution::Resolved(possible_facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry,
                Some(context),
                tokens,
                &effective,
                possible,
            )
        else {
            prepared.unknown_residual = true;
            continue;
        };
        let same_dialect = actual.is_some() && actual == possible;
        let exact_facts = if same_dialect || actual.is_none() {
            None
        } else if let Ok(RegistryInvocationResolution::Resolved(facts)) =
            resolve_effective_tokens_with_metadata_context(
                registry,
                Some(context),
                tokens,
                &effective,
                actual,
            )
        {
            Some(facts)
        } else {
            None
        };
        prepared.candidates.push(PreparedBodyCandidate {
            effective,
            possible_facts,
            exact_facts,
            same_dialect,
            selected_execution: binding.proved_execution_target() == Some(target),
        });
    }
    Some(prepared)
}

#[inline(never)]
pub(crate) fn invocation_body_assistance<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
) -> Box<InvocationBodyAssistance> {
    let context = context.into();
    body_assistance_with_entry(registry, context, surface, tokens, None)
}

#[inline(never)]
fn body_assistance_with_entry<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
    entry: Option<std::sync::Arc<crate::command_binding::SourceConditionalBodyEntry>>,
) -> Box<InvocationBodyAssistance> {
    let context = context.into();
    let prepared = prepare_body_candidates(registry, context, tokens);
    let mut result = Box::new(InvocationBodyAssistance {
        caller_template_source: None,
        possible_roles: Vec::new(),
        definite_roles: Vec::new(),
        possible_traits: tcl_registry::Traits::empty(),
        definite_traits: tcl_registry::Traits::empty(),
        definite_scope: None,
        possible_definition_grammars: Vec::new(),
        possible_case_lists: Vec::new(),
        possible_operations: Vec::new(),
        definite_invocation: None,
        conditional_invocation: None,
        receiver_trait_invocation: None,
        logical_trait_invocation: None,
        logical_lambda_roles: None,
        body_parameter_source: None,
        declared_roles: Vec::new(),
        unknown_residual: prepared.as_ref().is_none_or(|view| view.unknown_residual),
        may_be_absent: prepared.as_ref().is_none_or(|view| view.may_be_absent),
        parameter_advice: ParameterRoleAdvice::default(),
    });
    result.parameter_advice.closed = !result.unknown_residual && !result.may_be_absent;
    publish_possible_body_context(
        registry,
        context,
        surface,
        tokens,
        prepared.as_deref(),
        &mut result,
    );
    if result.unknown_residual || result.may_be_absent || tokens.synthetic.is_some() {
        return result;
    }
    let prepared = prepared.expect("closed assistance has prepared candidates");
    publish_unanimous_body_context(registry, context, &prepared, &mut result);
    if let Some(entry) = entry {
        retain_conditional_body_invocation(
            entry,
            registry,
            context,
            tokens,
            &prepared,
            &mut result,
        );
    }
    result.definite_invocation = prepared
        .candidates
        .iter()
        .find(|candidate| candidate.selected_execution)
        .filter(|candidate| {
            candidate.exact_facts().is_some_and(|facts| {
                facts.arg_roles_complete && facts.arity_accepts_frozen_arguments() == Some(true)
            })
        })
        .and_then(|candidate| prepared_body_invocation(registry, context, tokens, candidate));
    result.retain_caller_template_source(tokens);
    result
}

fn publish_possible_body_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    tokens: &CommandTokens,
    prepared: Option<&PreparedBodyCandidates>,
    result: &mut InvocationBodyAssistance,
) {
    if !context.matches_registry(registry) {
        return;
    }
    if let Some(prepared) = prepared {
        for candidate in &prepared.candidates {
            let facts = &candidate.possible_facts;
            let roles = mapped_written_argument_roles(
                &candidate.effective,
                facts.argument_offset,
                &facts.arg_roles,
            );
            result
                .parameter_advice
                .candidates
                .push(RegistryInvocationShape::from_facts(
                    facts,
                    candidate.effective.clone(),
                ));
            result.possible_roles.extend(roles);
            result.possible_traits |= facts.traits;
            collect_body_navigation(
                registry,
                context,
                tokens,
                (
                    &facts.canonical_command,
                    facts.operation,
                    &candidate.effective,
                ),
                result,
            );
        }
    }
    if let Some(catalogue) = catalogue_invocation_assistance(registry, context, tokens) {
        result
            .possible_roles
            .extend(catalogue.shape.written_argument_roles());
        result.possible_traits |= catalogue.shape.possible_traits;
        collect_body_navigation(
            registry,
            context,
            tokens,
            (
                &catalogue.shape.command,
                catalogue.shape.operation,
                &catalogue.shape.effective,
            ),
            result,
        );
    }
    if let Some(declared) = resolved_declared_assistance(surface, tokens) {
        result.declared_roles.clone_from(&declared.roles);
        result.possible_roles.extend(declared.roles);
    }
    let mut unique = Vec::new();
    for role in std::mem::take(&mut result.possible_roles) {
        if !unique.contains(&role) {
            unique.push(role);
        }
    }
    unique.sort_by_key(|(index, _)| *index);
    result.possible_roles = unique;
}

fn publish_unanimous_body_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    prepared: &PreparedBodyCandidates,
    result: &mut InvocationBodyAssistance,
) {
    let mut roles: Option<Vec<(usize, tcl_registry::ArgRole)>> = None;
    let mut traits: Option<tcl_registry::Traits> = None;
    let mut scope: Option<Option<&'static tcl_registry::scoped::ScopedCommandEnv>> = None;
    for candidate in &prepared.candidates {
        let Some(facts) = candidate.exact_facts().filter(|facts| {
            facts.arg_roles_complete && facts.arity_accepts_frozen_arguments() == Some(true)
        }) else {
            return;
        };
        let mapped = mapped_written_argument_roles(
            &candidate.effective,
            facts.argument_offset,
            &facts.arg_roles,
        );
        if let Some(shared) = &mut roles {
            shared.retain(|role| mapped.contains(role));
        } else {
            roles = Some(mapped);
        }
        traits = Some(traits.map_or(facts.traits, |previous| previous.intersection(facts.traits)));
        let selected_scope = context
            .context()
            .resolve_spec(registry, &facts.canonical_command)
            .and_then(|spec| spec.body_scope);
        scope = Some(match scope {
            None => selected_scope,
            Some(Some(previous))
                if selected_scope.is_some_and(|next| std::ptr::eq(previous, next)) =>
            {
                Some(previous)
            }
            _ => None,
        });
    }
    result.definite_roles = roles.unwrap_or_default();
    result.definite_traits = traits.unwrap_or_default();
    result.definite_scope = scope.flatten();
}

fn prepared_body_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    candidate: &PreparedBodyCandidate,
) -> Option<ResolvedStatementInvocation> {
    let context = context.into();
    let effective = candidate.effective.clone();
    let dialect = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .or_else(|| {
            context
                .context()
                .environment
                .point()
                .map(tcl_registry::InvocationDialect::of_point)
        })
        .or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        });
    Some(ResolvedStatementInvocation {
        dialect,
        facts: Box::new(candidate.exact_facts()?.clone()),
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
        authored_case_body_selection: None,
    })
}

fn collect_body_navigation(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    candidate: (
        &str,
        tcl_registry::SemanticOperationId,
        &EffectiveCommandWords,
    ),
    result: &mut InvocationBodyAssistance,
) {
    let (command, operation, effective) = candidate;
    let operation = (operation, effective.clone());
    if !result.possible_operations.contains(&operation) {
        result.possible_operations.push(operation);
    }
    if let Some(grammar) = context
        .context()
        .resolve_spec(registry, command)
        .and_then(|spec| spec.definition_body)
        && !result
            .possible_definition_grammars
            .iter()
            .any(|previous| std::ptr::eq(*previous, grammar))
    {
        result.possible_definition_grammars.push(grammar);
    }
    let Some(arguments) = effective.argument_spellings(&tokens.argv_texts[1..]) else {
        return;
    };
    let arguments: Vec<_> = arguments.iter().map(String::as_str).collect();
    // CaseInvocation indices are already relative to the complete post-head
    // argv supplied below, unlike selected member ArgRole indices.
    if let Some((spec, invocation)) = registry.case_invocation(
        command,
        &arguments,
        Some(context.context().authoring_query()),
    ) && let Some(index) = invocation
        .clause_list_index
        .and_then(|index| effective.written_argument(index))
    {
        result.possible_case_lists.push((spec, index));
    }
}

/// Use the retained execution point before the catalogue's separate authoring
/// view. The version-to-environment label is owned by the typed Tcl release;
/// an unknown point cannot manufacture a runtime or a default context.
fn body_assistance_context(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<SemanticContext> {
    let authoring = registry.profile().map(SemanticContext::for_profile);
    if let Some(packaged) =
        authoring.filter(|context| !context.context().authoring_query().packages.is_empty())
    {
        // Package placements are authoring availability, independently of the
        // actual engine passed to native facts and the retained invocation realm.
        return Some(packaged);
    }
    tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.variable_context.invocation_dialect)
        .and_then(tcl_registry::InvocationDialect::execution_point)
        .and_then(|point| {
            point
                .tcl_version()
                .map(|version| SemanticContext::for_environment(version.dialect_profile_name()))
                .or_else(|| {
                    (point.family() == tcl_dialect::model::Family::Jim)
                        .then(|| SemanticContext::for_environment(point.family().name()))
                })
        })
        .or(authoring)
}

/// Reconstruct only original words in this source buffer, then attach the
/// caller's retained source inventory. The affine base is supplied by the
/// source owner; this helper never analyses a detached body as a fresh entry.
pub(crate) fn segmented_body_assistance(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    bindings: &crate::command_binding::SourceCommandBindings,
    source: &str,
    config: tcl_lexer::LexerConfig,
    command: &crate::segmenter::SegmentedCommand,
    base: u32,
    metadata: Option<&tcl_registry::model::ContextRegistry>,
) -> Option<Box<InvocationBodyAssistance>> {
    let registry = surface.commands();
    let original = match bindings.source_origin()?.kind() {
        crate::command_binding::SourceOriginKind::InheritedPoint { source: text, .. }
        | crate::command_binding::SourceOriginKind::Authored(text)
        | crate::command_binding::SourceOriginKind::Loaded { source: text, .. }
        | crate::command_binding::SourceOriginKind::Derived { source: text, .. }
        | crate::command_binding::SourceOriginKind::ConditionalField { source: text, .. } => text,
    };
    let start = usize::try_from(base).ok()?;
    if original.get(start..start.checked_add(source.len())?) != Some(source.as_bytes()) {
        return None;
    }
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, command);
    crate::lattice_rebase::rebase_command_tokens(&mut tokens, i64::from(base));
    bindings.stamp_original_tokens(&mut tokens);
    bindings.attach_declared_body_assistance(&mut tokens, surface);
    let context = match metadata {
        Some(context) => InvocationMetadataContext::from(context),
        None => body_assistance_context(registry, &tokens)?.into(),
    };
    if !context.matches_registry(registry) {
        return None;
    }
    let mut view = body_assistance_with_entry(registry, context, surface, &tokens, None);
    if let Some(owned) = owned_body_layout_invocation(registry, context, &tokens, bindings) {
        publish_conditional_parameter_advice(owned, &mut view);
    } else if let Some(owned) =
        receiver_source_trait_invocation(registry, context, &tokens, bindings)
    {
        view.parameter_advice = ParameterRoleAdvice {
            candidates: vec![RegistryInvocationShape::from_facts(
                &owned.invocation.facts,
                owned.invocation.effective.clone(),
            )],
            closed: true,
        };
        view.receiver_trait_invocation = Some(owned);
    } else {
        view.body_parameter_source =
            original_body_parameter_source_invocation(registry, context, &tokens, bindings, config);
    }
    view.retain_caller_template_source(&tokens);
    Some(view)
}

/// Parameter usage from an authentic original body/declaration row. The
/// dedicated source facade cannot supply generic closed or executable facts.
fn original_body_parameter_source_invocation(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
    config: tcl_lexer::LexerConfig,
) -> Option<BodyParameterSourceInvocation> {
    context.matches_registry(registry).then_some(())?;
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    (bindings.source_origin() == Some(&site.source)).then_some(())?;
    let entry = binding.original_body_parameter_source_scope(tokens)?;
    let layout = entry.layout();
    let metadata = layout
        .original_declaration_metadata_context(registry, tokens)
        .ok()??;
    if metadata.context() != context.context()
        || context
            .source_analysis_input()
            .is_some_and(|input| metadata.source_analysis_input() != Some(input))
    {
        return None;
    }
    let words = entry.original_words(tokens, config, registry)?;
    if words.iter().any(|word| word.group().expand) {
        return None;
    }
    let invocation = resolve_original_declared_layout(registry, metadata, tokens, layout)?;
    if !invocation.facts.arg_roles_complete
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
        || invocation.effective.words.len() != words.len()
        || invocation.effective.origins.len() != words.len()
        || invocation.effective.origins.first() != Some(&InvocationWordOrigin::ResolvedHead)
        || invocation
            .effective
            .origins
            .iter()
            .enumerate()
            .skip(1)
            .any(|(ordinal, origin)| *origin != InvocationWordOrigin::Written(ordinal))
    {
        return None;
    }
    let shape =
        RegistryInvocationShape::from_facts(&invocation.facts, invocation.effective.clone());
    Some(BodyParameterSourceInvocation {
        entry,
        words,
        shape,
        invocation,
    })
}

/// Conditional traits from the complete original receiver declaration. Its
/// source namespace does not claim the future receiver's private namespace.
fn receiver_source_trait_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<ReceiverTraitInvocation> {
    // naming.tcloo.original-declared-receiver-caller-traits
    // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let entry = bindings.declared_receiver_body_entry_at(&site.source, site.offset)?;
    native_compiler_replay_source(tokens, site)?;
    let advice = binding.declaration_operand_layout_advice(tokens)?;
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_CALLER_FRAME").is_some() {
        eprintln!(
            "ORIGINAL_RECEIVER_TRAITS offset={} closed={} original_closed={} opaque={} targets={}",
            site.offset,
            advice.closed_receiver_trait_lookup(&entry),
            advice.closed_lookup(),
            advice.has_opaque_handler_alternatives(),
            advice.targets().len()
        );
    }
    if !advice.closed_receiver_trait_lookup(&entry) {
        return None;
    }
    let invocation = resolve_original_declared_layout(registry, context, tokens, &advice)?;
    if !invocation.facts.arg_roles_complete
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    Some(ReceiverTraitInvocation { entry, invocation })
}

/// One declaration's conditional handler layout. Compiled execution residuals
/// remain independent; they cannot erase a separately proved logical handler.
fn owned_body_layout_invocation<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<ConditionalBodyInvocation> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    let site = tokens.source_binding.as_ref()?.invocation_site()?;
    let entry = bindings.conditional_body_entry_at(&site.source, site.offset)?;
    native_compiler_replay_source(tokens, site)?;
    let mut conditional = tokens.clone();
    conditional.source_binding =
        Some(bindings.conditional_invocation_at_source(&entry, site.offset));
    if !entry.owns_invocation(conditional.source_binding.as_ref()?) {
        return None;
    }
    let invocation =
        resolved_handler_invocation_with_metadata_context(registry, Some(context), &conditional)
            .or_else(|| {
                let prepared = prepare_body_candidates(registry, context, &conditional)?;
                if prepared.unknown_residual || prepared.may_be_absent {
                    return None;
                }
                let mut unanimous = None;
                for candidate in &prepared.candidates {
                    let facts = candidate.exact_facts()?;
                    if !facts.arg_roles_complete
                        || facts.arity_accepts_frozen_arguments() != Some(true)
                    {
                        return None;
                    }
                    let selected =
                        prepared_body_invocation(registry, context, &conditional, candidate)?;
                    if unanimous
                        .as_ref()
                        .is_some_and(|previous| previous != &selected)
                    {
                        return None;
                    }
                    unanimous = Some(selected);
                }
                unanimous
            })?;
    if !invocation.facts.arg_roles_complete
        || invocation.facts.arity_accepts_frozen_arguments() != Some(true)
    {
        return None;
    }
    Some(ConditionalBodyInvocation { entry, invocation })
}

fn publish_conditional_parameter_advice(
    owned: ConditionalBodyInvocation,
    view: &mut InvocationBodyAssistance,
) {
    view.parameter_advice = ParameterRoleAdvice {
        candidates: vec![RegistryInvocationShape::from_facts(
            &owned.invocation.facts,
            owned.invocation.effective.clone(),
        )],
        closed: true,
    };
    view.conditional_invocation = Some(owned);
}

/// Build a purpose-restricted layout only from converged original candidates.
/// The declaration recipe is independent of entered-script frame observations.
fn retain_conditional_body_invocation(
    entry: std::sync::Arc<crate::command_binding::SourceConditionalBodyEntry>,
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    prepared: &PreparedBodyCandidates,
    view: &mut InvocationBodyAssistance,
) {
    let Some(site) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.invocation_site())
    else {
        return;
    };
    if !entry.owns_source(&site.source, site.offset)
        || !tokens
            .source_binding
            .as_ref()
            .is_some_and(|binding| entry.owns_invocation(binding))
    {
        return;
    }
    let mut unanimous = None;
    for candidate in &prepared.candidates {
        let Some(facts) = candidate.exact_facts() else {
            return;
        };
        if !facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true) {
            return;
        }
        let Some(selected) = prepared_body_invocation(registry, context, tokens, candidate) else {
            return;
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return;
        }
        unanimous = Some(selected);
    }
    if let Some(invocation) = unanimous {
        publish_conditional_parameter_advice(ConditionalBodyInvocation { entry, invocation }, view);
    }
}

struct LogicalParameterSourceFacts {
    facts: InvocationFacts,
    source_roles: Option<(Vec<(u8, tcl_registry::ArgRole)>, bool)>,
    named_only_roles: bool,
}

fn logical_parameter_source_facts(
    words: &source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
    conditional_source: bool,
    named_only: bool,
) -> Option<LogicalParameterSourceFacts> {
    let (facts, source_roles) = words.with_source_schema(context, |schema| {
        (
            schema.facts().clone(),
            conditional_source.then(|| schema.authored_logical_source_argument_roles()),
        )
    })?;
    let named_only_roles = named_only && facts.arg_roles_complete;
    if facts.arity_accepts_frozen_arguments() != Some(true)
        || (!named_only_roles && !source_roles.as_ref().is_some_and(|(_, complete)| *complete))
    {
        return None;
    }
    Some(LogicalParameterSourceFacts {
        facts,
        source_roles,
        named_only_roles,
    })
}

fn logical_parameter_origins_are_unwritten(
    words: &source_structure::OriginalRegistryWords,
) -> bool {
    words
        .origins()
        .iter()
        .enumerate()
        .any(|(ordinal, origin)| *origin != InvocationWordOrigin::Written(ordinal))
}

/// Offset-free syntax assistance joins the retained lookup worlds. It has no
/// original invocation point and therefore never returns definite grammar,
/// entered scope or state-bearing alias/value-copy evidence.
pub(crate) fn logical_parameter_body_assistance(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    realm: &crate::realm::CommandBindingRealm,
    body: &std::sync::Arc<crate::command_binding::OriginalLogicalProcedureBody>,
    source: &str,
    command: &crate::segmenter::SegmentedCommand,
    base: u32,
) -> Option<Box<InvocationBodyAssistance>> {
    let input = realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()?;
    let config = input.lexer_config();
    let image = realm.original_source_image()?;
    let context = input.borrowed_context_registry();
    if !realm.matches_resolved_analysis_input(input)
        || !body.matches_source(image, input)
        || body.has_earlier_unknown_mutations()
        || !body
            .scope()
            .scope_body()
            .matches_original_content(source, base, config)
        || context.commands().snapshot().semantic_key()
            != surface.commands().snapshot().semantic_key()
    {
        return None;
    }
    let segment = command.clone().shifted_by(base);
    let full_source = image.try_text().ok()?;
    let words =
        source_structure::logical_source_registry_words(full_source, realm, input, &segment)?;
    let source_structure::OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
        return None;
    };
    let scope = advice.logical_source_scope()?;
    let conditional_source = body
        .parameter_source_prefix(advice.original_words())
        .is_some_and(|prefix| {
            prefix.matches_body(body, input) && prefix.current() == Some(advice.as_ref())
        });
    let named_only = advice.has_named_only_logical_summary_descriptor();
    if (!named_only && !conditional_source)
        || scope.frame_recipe()
            != crate::command_binding::OriginalDeclaredSourceBodyFrame::Procedure
        || scope.scope_body() != body.scope().scope_body()
    {
        return None;
    }
    let LogicalParameterSourceFacts {
        facts,
        source_roles,
        named_only_roles,
    } = logical_parameter_source_facts(&words, context, conditional_source, named_only)?;
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::from_image(image), config, &segment);
    realm.stamp_original_tokens(&mut tokens);
    // This named-only source descriptor has no alias or expanded prefix. Its
    // canonical Registry label selects facts; original Written operands retain
    // their own geometry and unknown values.
    if logical_parameter_origins_are_unwritten(&words) {
        return None;
    }
    let effective = compose_original_effective_words(&tokens, words.command(), &[])?;
    let evaluated_words = words.arguments().to_vec();
    let invocation = ResolvedStatementInvocation {
        dialect: words.dialect(),
        facts: Box::new(facts),
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: evaluated_words
            .iter()
            .map(|word| {
                word.literal_bytes()
                    .and_then(|value| String::from_utf8(value.to_vec()).ok())
            })
            .collect(),
        evaluated_words,
        effective,
        authored_case_body_selection: None,
    };
    let mut view = unpositioned_body_assistance(
        surface,
        realm.source_bindings_ref(),
        source,
        config,
        command,
        Some(context),
    )?;
    let mut shape =
        RegistryInvocationShape::from_facts(&invocation.facts, invocation.effective.clone());
    let source_parameter_shape = if named_only_roles {
        view.parameter_advice = ParameterRoleAdvice {
            candidates: vec![shape],
            closed: true,
        };
        None
    } else {
        // Only this separate shape consumes the source-role consensus. Keep
        // ordinary InvocationFacts roles/completeness and all generic closed
        // advice unchanged, including unavailable execution frame grammar.
        let (roles, complete) = source_roles?;
        shape.roles = roles;
        shape.roles_complete = complete;
        view.parameter_advice = ParameterRoleAdvice::default();
        Some(shape)
    };
    view.logical_trait_invocation = Some(LogicalTraitInvocation {
        source_parameter_shape,
        input: input.clone(),
        body: std::sync::Arc::clone(body),
        words,
        invocation,
    });
    view.logical_lambda_roles = None;
    // Other execution, Native conditional-entry and definite fields retain
    // their independent meaning; only this authenticated source template is added.
    Some(view)
}

/// Source-only argument roles for one actual command inside a selected
/// Logical lambda. The separate role facet never enters ordinary invocation
/// advice or establishes lambda entry.
// naming.compiler.logical-lambda-parameter-trait-source-roles
// docs/design/analysis/name-resolution-proofs/logical-lambda-parameter-trait-source-roles.md
pub(crate) fn logical_lambda_parameter_body_assistance(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    realm: &crate::realm::CommandBindingRealm,
    owner: &LogicalLambdaSourceOwner,
    selected_child_body: Option<&OriginalSourceScriptBody>,
    source: &str,
    command: &crate::segmenter::SegmentedCommand,
    base: u32,
) -> Option<Box<InvocationBodyAssistance>> {
    let input = realm
        .source_bindings_ref()
        .original_logical_source_name_advice_input()?;
    let image = realm.original_source_image()?;
    let context = input.borrowed_context_registry();
    let lambda = &owner.lambda;
    let scope = &owner.scope;
    let body = lambda.body();
    let source_scope = selected_child_body.unwrap_or(body);
    let segment = command.clone().shifted_by(base);
    if !realm.matches_resolved_analysis_input(input)
        || !lambda.matches_source(image, input)
        || !scope.matches_input(input)
        || !scope.lambda_source().is_some_and(|owner| owner == lambda)
        || !source_scope.matches_original_content(source, base, input.lexer_config())
        || !source_scope.matches_source(image, input.lexer_config())
        || !source_scope.matches_context(context)
        || source_scope.content_span().start() < body.content_span().start()
        || source_scope.content_span().end() > body.content_span().end()
        || segment.span.start() < source_scope.content_span().start()
        || segment.span.end() > source_scope.content_span().end()
        || context.commands().snapshot().semantic_key()
            != surface.commands().snapshot().semantic_key()
    {
        return None;
    }
    let full_source = image.try_text().ok()?;
    let words =
        source_structure::logical_source_registry_words(full_source, realm, input, &segment)?;
    let (facts, roles) = words.with_source_schema(context, |schema| {
        (
            schema.facts().clone(),
            schema.authored_logical_source_argument_roles(),
        )
    })?;
    let (roles, complete) = roles;
    if !complete
        || facts.arity_accepts_frozen_arguments() != Some(true)
        || words
            .origins()
            .iter()
            .enumerate()
            .any(|(ordinal, origin)| *origin != InvocationWordOrigin::Written(ordinal))
    {
        return None;
    }
    let mut tokens = CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(image),
        input.lexer_config(),
        &segment,
    );
    realm.stamp_original_tokens(&mut tokens);
    let effective = compose_original_effective_words(&tokens, words.command(), &[])?;
    let evaluated_words = words.arguments().to_vec();
    let invocation = ResolvedStatementInvocation {
        dialect: words.dialect(),
        facts: Box::new(facts),
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: evaluated_words
            .iter()
            .map(|word| {
                word.literal_bytes()
                    .and_then(|value| String::from_utf8(value.to_vec()).ok())
            })
            .collect(),
        evaluated_words,
        effective,
        authored_case_body_selection: None,
    };
    let mut shape =
        RegistryInvocationShape::from_facts(&invocation.facts, invocation.effective.clone());
    shape.roles = roles;
    shape.roles_complete = true;
    let mut view = unpositioned_body_assistance(
        surface,
        realm.source_bindings_ref(),
        source,
        input.lexer_config(),
        command,
        Some(context),
    )?;
    view.logical_lambda_roles = Some(ConditionalLogicalLambdaRoles {
        lambda: std::sync::Arc::clone(lambda),
        scope: std::sync::Arc::clone(scope),
        input: input.clone(),
        words,
        invocation,
        shape,
    });
    Some(view)
}

pub(crate) fn unpositioned_body_assistance(
    surface: &tcl_registry::model::DocumentCommandSurface<'_>,
    bindings: &crate::command_binding::SourceCommandBindings,
    source: &str,
    config: tcl_lexer::LexerConfig,
    command: &crate::segmenter::SegmentedCommand,
    metadata: Option<&tcl_registry::model::ContextRegistry>,
) -> Option<Box<InvocationBodyAssistance>> {
    let registry = surface.commands();
    let mut tokens =
        CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, command);
    tokens.source_binding = Some(bindings.invocation_unpositioned(command.name()));
    bindings.attach_declared_body_assistance(&mut tokens, surface);
    let context = match metadata {
        Some(context) => Some(InvocationMetadataContext::from(context)),
        None => body_assistance_context(registry, &tokens).map(InvocationMetadataContext::from),
    };
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return None;
    }
    let Some(context) = context else {
        let declared = resolved_declared_assistance(surface, &tokens)?;
        return Some(Box::new(InvocationBodyAssistance {
            caller_template_source: None,
            possible_roles: declared.roles.clone(),
            declared_roles: declared.roles,
            definite_roles: Vec::new(),
            possible_traits: tcl_registry::Traits::empty(),
            definite_traits: tcl_registry::Traits::empty(),
            definite_scope: None,
            possible_definition_grammars: Vec::new(),
            possible_case_lists: Vec::new(),
            possible_operations: Vec::new(),
            definite_invocation: None,
            conditional_invocation: None,
            receiver_trait_invocation: None,
            logical_trait_invocation: None,
            logical_lambda_roles: None,
            body_parameter_source: None,
            unknown_residual: true,
            may_be_absent: true,
            parameter_advice: ParameterRoleAdvice::default(),
        }));
    };
    let mut view = invocation_body_assistance(registry, context, surface, &tokens);
    view.definite_roles.clear();
    view.definite_traits = tcl_registry::Traits::empty();
    view.definite_scope = None;
    view.definite_invocation = None;
    view.unknown_residual = true;
    view.may_be_absent = true;
    Some(view)
}

/// Owned source-aware word fact for callers that must decode a static Tcl
/// word before lending it to the registry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EffectiveInvocationWord {
    /// A Tcl value proven static after applying the active escape syntax.
    Literal(String),
    /// A decoded native string whose exact bytes are not Unicode.
    /// This remains one argv operand and grants no text, name or object proof.
    ByteLiteral(std::sync::Arc<[u8]>),
    /// One argv word produced by substitution.
    Dynamic,
    /// A computed variable name whose evaluated root and array form are proved.
    ArrayElementName {
        /// Root before the first opening parenthesis; the element remains unknown.
        root: String,
    },
    /// An expansion with an unknown resulting argv length.
    Expanded,
    /// Native list elements retained at a written expansion evaluation.
    /// This grants cardinality and bytes, never object-representation proof.
    KnownExpansion(Vec<String>),
    /// Native list elements whose payloads cannot all be represented as Unicode.
    /// Each element remains one argv operand, with no object or source-word grant.
    KnownByteExpansion(Vec<std::sync::Arc<[u8]>>),
    /// A compatibility/recovery word whose source meaning is unavailable.
    Opaque,
}

/// Whether a source word may name a compiled local-variable slot.
///
/// This is deliberately stricter than [`EffectiveInvocationWord`]: a bare
/// word with backslash processing has a static *value*, but C Tcl still emits
/// the stack form for it and does not intern that value in the LVT.  Braced
/// and quoted/plain text words whose spelling needs no backslash processing
/// retain the direct-name form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledLocalNameWord {
    /// The source form is eligible for a direct local-name opcode.
    Direct,
    /// The word must be evaluated or decoded before it can name a variable.
    Stack,
}

/// Classify one source word for bytecode local-name selection.
///
/// The word-parts owner has already retained substitution and raw backslash
/// facts in [`WordExpr`].  Consumers must ask this adapter rather than infer
/// source provenance from compatibility argv text.  In particular, `{p\\x}`
/// is direct (the backslash is part of the name), while bare `p\\x` is stack
/// (the backslash is decoded before lookup).
#[must_use]
pub fn compiled_local_name_word(word: &WordExpr) -> CompiledLocalNameWord {
    match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => CompiledLocalNameWord::Direct,
        WordExpr::Template { parts, .. }
            if parts.iter().all(
                |part| matches!(part, WordPart::Text { text, .. } if !text.contains('\\')),
            ) =>
        {
            // A quoted literal reaches this arm: grouping quotes are outside
            // its text parts, while a quoted backslash remains in one.
            CompiledLocalNameWord::Direct
        }
        WordExpr::Variable { .. }
        | WordExpr::CommandSubstitution { .. }
        | WordExpr::Template { .. }
        | WordExpr::Expand { .. }
        | WordExpr::Opaque { .. } => CompiledLocalNameWord::Stack,
    }
}

/// Return the evaluated name for a source word eligible for a compiled local
/// opcode. The same source owner supplies both the eligibility decision and
/// the literal value so brace grouping cannot reach an opcode as data.
#[must_use]
pub fn compiled_local_name_value(
    word: &WordExpr,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> Option<String> {
    if compiled_local_name_word(word) != CompiledLocalNameWord::Direct {
        return None;
    }
    match effective_invocation_word(word, escapes, word_rules) {
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

impl EffectiveInvocationWord {
    /// Cardinality retained by a proved native list expansion.
    #[must_use]
    pub fn expansion_len(&self) -> Option<usize> {
        match self {
            Self::KnownExpansion(values) => Some(values.len()),
            Self::KnownByteExpansion(values) => Some(values.len()),
            _ => None,
        }
    }

    /// Exact native bytes of one proved expansion child.
    #[must_use]
    pub fn expansion_element(&self, ordinal: usize) -> Option<&[u8]> {
        match self {
            Self::KnownExpansion(values) => values.get(ordinal).map(String::as_bytes),
            Self::KnownByteExpansion(values) => values.get(ordinal).map(AsRef::as_ref),
            _ => None,
        }
    }

    /// Retain decoded native bytes with a checked Unicode projection.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        std::str::from_utf8(bytes).map_or_else(
            |_| Self::ByteLiteral(std::sync::Arc::from(bytes)),
            |text| Self::Literal(text.to_owned()),
        )
    }

    /// Exact payload when this word has a known native string value.
    #[must_use]
    pub fn literal_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Literal(text) => Some(text.as_bytes()),
            Self::ByteLiteral(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// Lend this owned fact to the registry's allocation-free vocabulary.
    #[must_use]
    pub fn as_registry_word(&self) -> InvocationWord<'_> {
        match self {
            Self::Literal(value) => InvocationWord::Literal(value),
            Self::ByteLiteral(value) => InvocationWord::KnownBytes(value),
            Self::Dynamic => InvocationWord::Dynamic,
            Self::ArrayElementName { root } => InvocationWord::ArrayElementName { root },
            Self::Expanded | Self::KnownExpansion(_) | Self::KnownByteExpansion(_) => {
                InvocationWord::Expanded
            }
            Self::Opaque => InvocationWord::Opaque,
        }
    }
}

/// An owned, target-neutral result from registry invocation resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryInvocationResolution {
    /// The registry selected complete common facts for the invocation.
    Resolved(Box<InvocationFacts>),
    /// The command head could not safely select a registry descriptor.
    Unresolved(OwnedInvocationResolutionUnresolved),
}

/// A typed unresolved command-head outcome retained across the registry's
/// borrowing boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedInvocationResolutionUnresolved {
    /// The command head was substituted, expanded, or opaque.
    ComputedHead {
        /// Source-word knowledge that prevented registry lookup.
        word_kind: InvocationWordKind,
    },
    /// A literal head was absent for the explicitly selected dialect profile.
    UnknownLiteralHead {
        /// Literal source spelling consulted in the registry.
        spelling: String,
    },
    /// Point-specific lookup cannot prove one live registry implementation.
    UnprovedBinding {
        /// Original command spelling, retained for presentation only.
        spelling: String,
        /// At least one represented execution path may have no command.
        may_be_absent: bool,
        /// Some execution path has no bounded target implementation.
        unknown: bool,
    },
    /// A selected semantic implementation requires an original private slot
    /// and dispatch mapping whose live identity is not established.
    UnprovedNativeImplementationLookup {
        /// Exact registry-authored lookup; catalogue presence is insufficient.
        lookup: tcl_registry::native_compilation::NativeCompilerImplementationLookup,
    },
}

impl OwnedInvocationResolutionUnresolved {
    fn from_registry(unresolved: InvocationResolutionUnresolved<'_>) -> Self {
        match unresolved {
            InvocationResolutionUnresolved::ComputedHead { word_kind } => {
                Self::ComputedHead { word_kind }
            }
            InvocationResolutionUnresolved::UnknownLiteralHead { spelling } => {
                Self::UnknownLiteralHead {
                    spelling: spelling.to_owned(),
                }
            }
        }
    }
}

/// Why no registry invocation outcome could be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryInvocationDecline {
    /// The semantic word sequence contained no command head.
    MissingCommandHead,
    /// The registry violated its structured-resolution contract by producing
    /// neither a resolved invocation nor an unresolved reason.
    IncompleteResolution,
}

/// Convert one compiler source word to the registry's value-conservative
/// invocation vocabulary.
///
/// Literal and braced-literal words expose their Tcl value. Substitutions and
/// compound templates contribute exactly one argv word but never expose their
/// source spelling as a value. Expansion keeps only its argv-shape fact, and
/// compatibility recovery remains opaque.
#[must_use]
pub fn invocation_word(word: &WordExpr) -> InvocationWord<'_> {
    match word {
        WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } => {
            InvocationWord::Literal(text)
        }
        WordExpr::Variable { .. }
        | WordExpr::CommandSubstitution { .. }
        | WordExpr::Template { .. } => InvocationWord::Dynamic,
        WordExpr::Expand { .. } => InvocationWord::Expanded,
        WordExpr::Opaque { .. } => InvocationWord::Opaque,
    }
}

/// Preserve a proved non-option prefix without exposing a substituted value.
/// Known evaluated bytes take precedence; template prefix bytes are decoded
/// under the original ingress escape grammar.
#[must_use]
pub fn invocation_word_with_source<'a>(
    source: &WordExpr,
    effective: &'a EffectiveInvocationWord,
    escapes: EscapeSyntax,
) -> InvocationWord<'a> {
    if matches!(effective, EffectiveInvocationWord::Dynamic)
        && let WordExpr::Template { parts, .. } = source
    {
        let prefix: String = parts
            .iter()
            .take_while(|part| matches!(part, WordPart::Text { .. }))
            .filter_map(|part| match part {
                WordPart::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        if tcl_syntax::backslash::decode_bytes_in(prefix.as_bytes(), escapes)
            .first()
            .is_some_and(|first| *first != b'-')
        {
            return InvocationWord::DynamicNonOption;
        }
    }
    effective.as_registry_word()
}

/// Evaluate the statically-known portion of a source word under the active
/// escape grammar. Braced words keep their backslashes except for the
/// dialect-defined line-continuation collapse.
#[must_use]
pub fn effective_invocation_word(
    word: &WordExpr,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> EffectiveInvocationWord {
    match word {
        WordExpr::Literal { text, .. } => EffectiveInvocationWord::Literal(text.clone()),
        WordExpr::BracedLiteral { text, .. } => {
            EffectiveInvocationWord::Literal(word_rules.collapse_braced_word(text).into_owned())
        }
        WordExpr::Template { parts, .. }
            if parts
                .iter()
                .all(|part| matches!(part, WordPart::Text { .. })) =>
        {
            let raw: String = parts
                .iter()
                .map(|part| match part {
                    WordPart::Text { text, .. } => text.as_str(),
                    _ => unreachable!("guard admits text parts only"),
                })
                .collect();
            EffectiveInvocationWord::from_bytes(&tcl_syntax::backslash::decode_bytes_in(
                raw.as_bytes(),
                escapes,
            ))
        }
        WordExpr::Template { .. } => EffectiveInvocationWord::Dynamic,
        WordExpr::Variable { .. } | WordExpr::CommandSubstitution { .. } => {
            EffectiveInvocationWord::Dynamic
        }
        WordExpr::Expand { .. } => EffectiveInvocationWord::Expanded,
        WordExpr::Opaque { .. } => EffectiveInvocationWord::Opaque,
    }
}

/// Partial variable-operand shape from one actual original C compiler word.
/// This retains the independently selected recipe and complete source grammar;
/// it supplies neither an evaluated whole value nor a variable handler or cell.
pub(crate) fn original_array_element_word(
    original: &tcl_lexer::NativeWord,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<EffectiveInvocationWord> {
    let tcl_syntax::naming::NativeNameProtocol::C(version) = policy.recipe() else {
        return None;
    };
    let tcl_syntax::native_variable_words::NativeVariableWordOperand::CompoundArray {
        name, ..
    } = tcl_syntax::native_variable_words::native_variable_word(
        original,
        version,
        policy.string_protocol(),
    )
    .ok()?
    else {
        return None;
    };
    Some(EffectiveInvocationWord::ArrayElementName {
        root: String::from_utf8(name).ok()?,
    })
}

/// Convert a command-token snapshot into source-aware effective argv facts.
#[must_use]
pub fn effective_command_arguments(
    tokens: &CommandTokens,
    escapes: EscapeSyntax,
    word_rules: WordValueRules,
) -> Vec<EffectiveInvocationWord> {
    tokens
        .words()
        .get(1..)
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(argument, word)| {
            let value = effective_invocation_word(word, escapes, word_rules);
            if value == EffectiveInvocationWord::Dynamic {
                tokens
                    .source_binding
                    .as_ref()
                    .and_then(|binding| {
                        binding.original_compound_array_argument(
                            tokens,
                            argument + 1,
                            escapes,
                            word_rules,
                        )
                    })
                    .unwrap_or(value)
            } else {
                value
            }
        })
        .collect()
}

/// Borrow source-aware post-head words from a segmented command.
///
/// The segmenter's compatibility text is still the literal value bridge for
/// callers that do not need escape decoding. Its lossless fragment list is
/// what prevents a `$word` / `[command]` / `{*}word` from becoming a literal
/// option or operand during a registry query, while retaining the distinct
/// safe shape of `prefix-$word`: it remains dynamic as a value but cannot
/// start a leading option.
#[must_use]
pub fn segmented_command_arguments(command: &SegmentedCommand) -> Vec<InvocationWord<'_>> {
    command
        .texts
        .iter()
        .enumerate()
        .skip(1)
        .map(|(index, text)| {
            if command
                .expand_word
                .as_ref()
                .and_then(|markers| markers.get(index))
                == Some(&true)
            {
                return InvocationWord::Expanded;
            }
            let Some(fragments) = command.word_fragments.get(index) else {
                return InvocationWord::Opaque;
            };
            if fragments.iter().any(|fragment| {
                matches!(
                    fragment.token.kind,
                    tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                )
            }) {
                // A non-empty literal prefix before the first substitution
                // proves the evaluated word cannot begin with `-`. Keep that
                // narrower source fact so option-dependent descriptors may
                // still locate its positional embedded language; it is never
                // exposed as a literal value to registry hooks.
                let literal_prefix = fragments
                    .iter()
                    .take_while(|fragment| {
                        !matches!(
                            fragment.token.kind,
                            tcl_lexer::TokenType::Var | tcl_lexer::TokenType::Cmd
                        )
                    })
                    .map(|fragment| fragment.text.as_str())
                    .collect::<String>();
                // `word_piece` retains a quoted word's opening quote in an
                // empty leading `Esc` fragment, and a leading backslash can
                // decode to `-`; neither proves a non-option value. Every
                // other direct first character is stable under Tcl word
                // substitution.
                let proves_non_option = literal_prefix
                    .chars()
                    .next()
                    .is_some_and(|first| !matches!(first, '-' | '\\' | '"'));
                if proves_non_option {
                    InvocationWord::DynamicNonOption
                } else {
                    InvocationWord::Dynamic
                }
            } else {
                InvocationWord::Literal(text)
            }
        })
        .collect()
}

/// Resolve semantic source words through the registry in `context`.
///
/// The borrowed registry resolution is materialised immediately into owned
/// facts or an owned typed unresolved outcome, making this suitable for
/// common IR and analysis state.
///
/// `context` is `None` for a caller that carries no environment (a unit
/// harness, a shape-only query); the selection is then dialect-blind, exactly
/// as the retired `None` argument behaved.
pub fn resolve_word_exprs(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    words: &[WordExpr],
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    let dialect = context
        .and_then(|context| context.context().environment.point())
        .map(tcl_registry::InvocationDialect::of_point)
        .or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        });
    resolve_word_exprs_with_dialect(registry, context, words, dialect)
}

/// Resolve source words under explicitly retained interpreter policies. This
/// keeps escape and frame grammars independent of catalogue availability.
pub fn resolve_word_exprs_with_dialect(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    words: &[WordExpr],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_word_exprs_with_dialect_and_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        words,
        dialect,
    )
}

fn resolve_word_exprs_with_dialect_and_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    words: &[WordExpr],
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    if words.is_empty() {
        return Err(RegistryInvocationDecline::MissingCommandHead);
    }
    let effective: Option<Vec<_>> = dialect.map(|dialect| {
        words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect()
    });
    let facts: Vec<_> = if let Some(effective) = &effective {
        effective
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect()
    } else {
        words.iter().map(invocation_word).collect()
    };
    resolve_registry_words_in_realm_with_metadata_context(
        registry,
        context,
        &facts,
        dialect,
        tcl_dialect::model::InvocationRealm::RuleLoader,
    )
}

pub(crate) fn resolve_registry_words_in_realm(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    facts: &[InvocationWord<'_>],
    dialect: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_registry_words_in_realm_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        facts,
        dialect,
        realm,
    )
}

pub(crate) fn resolve_registry_words_in_realm_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    facts: &[InvocationWord<'_>],
    dialect: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_registry_words_in_realm_with_metadata_context_and_case(
        registry, context, facts, dialect, realm,
    )
    .map(|(resolution, _)| resolution)
}

fn resolve_registry_words_in_realm_with_metadata_context_and_case(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    facts: &[InvocationWord<'_>],
    dialect: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
) -> Result<
    (
        RegistryInvocationResolution,
        AuthoredSourceCaseBodySelection,
    ),
    RegistryInvocationDecline,
> {
    if context.is_some_and(|context| !context.matches_registry(registry)) {
        return Err(RegistryInvocationDecline::IncompleteResolution);
    }
    let Some(head) = facts.first().copied() else {
        return Err(RegistryInvocationDecline::MissingCommandHead);
    };
    let mut invocation = InvocationWords::structured(head, &facts[1..]);
    if let Some(dialect) = dialect {
        invocation = invocation.with_dialect(dialect);
    }
    let resolution =
        tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
            registry,
            context.map(InvocationMetadataContext::context),
            invocation,
            realm,
        );
    if let Some(resolved) = resolution.resolved() {
        if tcl_registry::trace_deferred_writes_any() {
            eprintln!(
                "TCL_LSP_TRACE_METADATA_SELECTION head={:?} canonical={:?} realm={realm:?} context={:?} query={:?}",
                invocation.head_literal(),
                resolved.canonical_command,
                context.map(|context| std::ptr::from_ref(context.context())),
                resolved.dialect,
            );
        }
        let case_selection = if resolved.semantics.options.case_list.is_none() {
            AuthoredSourceCaseBodySelection::NotCase
        } else {
            dialect
                .and_then(|dialect| {
                    resolved.authored_source_case_clause_projection(dialect.word_values)
                })
                .map_or(
                    AuthoredSourceCaseBodySelection::Unavailable,
                    AuthoredSourceCaseBodySelection::Selected,
                )
        };
        return Ok((
            RegistryInvocationResolution::Resolved(Box::new(resolved.facts())),
            case_selection,
        ));
    }
    if context.is_none_or(|context| !context.is_actual())
        && let Some(facts) = registry.native_registration_invocation_facts(invocation)
    {
        return Ok((
            RegistryInvocationResolution::Resolved(Box::new(facts)),
            AuthoredSourceCaseBodySelection::Unavailable,
        ));
    }
    let Some(unresolved) = resolution.unresolved() else {
        return Err(RegistryInvocationDecline::IncompleteResolution);
    };
    Ok((
        RegistryInvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::from_registry(unresolved),
        ),
        AuthoredSourceCaseBodySelection::Unavailable,
    ))
}

/// Resolve a source command-token snapshot through the registry.
///
/// [`CommandTokens::words`] is the compiler's canonical ordered source-word
/// representation. This adapter never falls back to the lossy argv text
/// arrays, so dynamic words cannot become accidental literal command or
/// subcommand values.
pub fn resolve_command_tokens(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_command_tokens_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

#[doc(hidden)]
pub fn resolve_command_tokens_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_command_tokens_with_metadata_context_and_case(registry, context, tokens)
        .map(|(resolution, _)| resolution)
}

fn resolve_command_tokens_with_metadata_context_and_case(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Result<
    (
        RegistryInvocationResolution,
        AuthoredSourceCaseBodySelection,
    ),
    RegistryInvocationDecline,
> {
    let Some(binding) = &tokens.source_binding else {
        let dialect = context
            .and_then(|context| context.context().environment.point())
            .map(tcl_registry::InvocationDialect::of_point)
            .or_else(|| {
                registry
                    .profile()
                    .map(tcl_registry::InvocationDialect::of_profile)
            });
        let resolution = resolve_word_exprs_with_dialect_and_metadata_context(
            registry,
            context,
            tokens.words(),
            dialect,
        )?;
        let mut resolution = require_native_implementation_lookup(tokens, resolution, dialect);
        if let RegistryInvocationResolution::Resolved(facts) = &mut resolution {
            let words = (0..tokens.words().len().saturating_sub(1))
                .map(|_| InvocationWord::Dynamic)
                .collect::<Vec<_>>();
            let mut arguments = tcl_registry::InvocationArguments::structured(&words);
            if let Some(dialect) = dialect {
                arguments = arguments.with_dialect(dialect);
            }
            crate::command_binding::object_callbacks::refine_facts(facts, arguments, None);
        }
        return Ok((resolution, AuthoredSourceCaseBodySelection::Unavailable));
    };
    let Some(target) = binding
        .proved_execution_target()
        .filter(|target| target.registry_backed)
    else {
        return Ok((
            RegistryInvocationResolution::Unresolved(
                OwnedInvocationResolutionUnresolved::UnprovedBinding {
                    spelling: tokens.argv_texts.first().cloned().unwrap_or_default(),
                    may_be_absent: binding.may_be_absent,
                    unknown: binding.unknown,
                },
            ),
            AuthoredSourceCaseBodySelection::Unavailable,
        ));
    };
    // Selection above also establishes why an effective projection exists.
    let _ = target;
    let effective =
        effective_command_words(tokens).ok_or(RegistryInvocationDecline::IncompleteResolution)?;
    resolve_effective_tokens_with_metadata_context_and_case(
        registry,
        context,
        tokens,
        &effective,
        binding.variable_context.invocation_dialect.or_else(|| {
            registry
                .profile()
                .map(tcl_registry::InvocationDialect::of_profile)
        }),
    )
}

fn resolve_effective_tokens_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<RegistryInvocationResolution, RegistryInvocationDecline> {
    resolve_effective_tokens_with_metadata_context_and_case(
        registry, context, tokens, effective, dialect,
    )
    .map(|(resolution, _)| resolution)
}

fn resolve_effective_tokens_with_metadata_context_and_case(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> Result<
    (
        RegistryInvocationResolution,
        AuthoredSourceCaseBodySelection,
    ),
    RegistryInvocationDecline,
> {
    let frozen = frozen_argument_words(tokens, effective);
    let values: Vec<_> = effective
        .words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            if let Some(value) = index
                .checked_sub(1)
                .and_then(|argument| frozen.get(argument))
            {
                return value.clone();
            }
            dialect.map_or_else(
                || match invocation_word(word) {
                    InvocationWord::Literal(value) => {
                        EffectiveInvocationWord::Literal(value.to_owned())
                    }
                    InvocationWord::KnownBytes(value) => EffectiveInvocationWord::from_bytes(value),
                    InvocationWord::Expanded => EffectiveInvocationWord::Expanded,
                    InvocationWord::Opaque => EffectiveInvocationWord::Opaque,
                    _ => EffectiveInvocationWord::Dynamic,
                },
                |dialect| {
                    effective_invocation_word(
                        word,
                        dialect.lexer_grammar.escapes,
                        dialect.word_values,
                    )
                },
            )
        })
        .collect();
    let facts: Vec<_> = values
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect();
    let realm = tokens
        .source_binding
        .as_ref()
        .and_then(crate::command_binding::SourceInvocationBinding::invocation_realm)
        .unwrap_or_default();
    let (resolution, case_selection) =
        resolve_registry_words_in_realm_with_metadata_context_and_case(
            registry, context, &facts, dialect, realm,
        )?;
    let mut resolution = require_native_implementation_lookup(tokens, resolution, dialect);
    if let RegistryInvocationResolution::Resolved(resolved) = &mut resolution {
        let mut arguments = tcl_registry::InvocationArguments::structured(&facts[1..]);
        if let Some(dialect) = dialect {
            arguments = arguments.with_dialect(dialect);
        }
        crate::command_binding::object_callbacks::refine_facts(
            resolved,
            arguments,
            tokens.source_binding.as_ref(),
        );
    }
    let case_selection = if matches!(resolution, RegistryInvocationResolution::Resolved(_)) {
        case_selection
    } else {
        AuthoredSourceCaseBodySelection::Unavailable
    };
    Ok((resolution, case_selection))
}

/// Private ensemble implementation proof belongs to the semantic adapter,
/// including callers that carry source words without an interpreter snapshot.
/// Such callers may query syntax/assistance separately; they cannot borrow the
/// public ensemble's identity to specialize an unproved private implementation.
fn require_native_implementation_lookup(
    tokens: &CommandTokens,
    resolution: RegistryInvocationResolution,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> RegistryInvocationResolution {
    if tokens
        .source_binding
        .as_ref()
        .is_some_and(|binding| binding.named_invocation().is_some())
    {
        return resolution;
    }
    if let RegistryInvocationResolution::Resolved(facts) = &resolution
        && let Some(lookup) = match facts.normal_handler_implementation_lookup(dialect) {
            tcl_registry::native_compilation::NormalHandlerImplementationLookup::Required(
                lookup,
            ) => Some(lookup),
            _ => dialect.and_then(|dialect| {
                facts
                    .native_compilation
                    .and_then(|native| native.implementation_lookup(dialect))
            }),
        }
        && !tokens
            .source_binding
            .as_ref()
            .is_some_and(|binding| binding.proves_native_implementation_lookup(&lookup))
    {
        return RegistryInvocationResolution::Unresolved(
            OwnedInvocationResolutionUnresolved::UnprovedNativeImplementationLookup { lookup },
        );
    }
    resolution
}

/// Purpose-limited possible body inventory of a converged handler. It exposes
/// no registry effects, purity, completion or executable opcode permission.
pub struct PossibleBodyInvocation {
    /// Logical source grammar of this selected layout, independently of runtime dispatch.
    pub dialect: Option<tcl_registry::InvocationDialect>,
    /// Registry-authored topology of possible script entry.
    pub topology: tcl_registry::native_compilation::PossibleBodyTopology,
    /// Frozen native expression operands, preserving unknown evaluated values.
    pub condition_values: Vec<(usize, Option<String>)>,
    /// Original/alias operand correspondence; original words still evaluate once.
    pub effective: EffectiveCommandWords,
}

/// Completion of the selected normal handler for diagnostic flow only.
/// Native compiler uncertainty and its independent pre-entry failure remain in
/// the execution carrier; this route cannot license CFG pruning or an opcode.
#[must_use]
pub fn normal_handler_completion_route(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> tcl_registry::completion_route::InvocationCompletionRoute {
    resolved_handler_invocation(registry, context, tokens).map_or(
        tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
        |invocation| invocation.completion_route(registry),
    )
}

/// Procedure caller operands of a converged handler, independently of its
/// compiler protocol. This carries no effects of the called procedure.
pub struct NormalUserProcedureInvocation {
    /// Frozen procedure name at the handler's selected operand.
    pub target: String,
    /// Frozen actual values passed to the procedure, preserving unknown slots.
    pub arguments: Vec<Option<String>>,
    /// Actual runtime dispatch uncertainty, separate from compiler uncertainty.
    pub unknown_runtime: bool,
}

/// Project only an authored converged procedure-caller contract. Catalogue
/// spelling and an unknown actual command binding cannot provide this proof.
#[must_use]
pub fn normal_user_procedure_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalUserProcedureInvocation> {
    normal_user_procedure_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

pub(crate) fn normal_user_procedure_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalUserProcedureInvocation> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let invocation = resolved_handler_invocation_with_metadata_context(registry, context, tokens)?;
    let layout = invocation.with_argument_words(|words| {
        invocation
            .facts
            .successful_handler_user_procedure_call(words.arguments())
    })?;
    Some(NormalUserProcedureInvocation {
        target: invocation.argument_literal(layout.target_at)?,
        arguments: (layout.arguments_from..invocation.arguments.len())
            .map(|index| invocation.argument_literal(index))
            .collect(),
        unknown_runtime: binding.unknown || binding.may_be_absent,
    })
}

/// Query candidate bodies independently of native compiler selection. Only the
/// shared source kernel's converged handler and exact frozen argv/frame contract
/// can supply this inventory; written spellings do not provide fallback proof.
#[must_use]
pub fn possible_body_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleBodyInvocation> {
    possible_body_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Candidate handler-body topology under the supplied complete availability.
/// The original handler and frame owner remain independently required.
pub(crate) fn possible_body_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleBodyInvocation> {
    if context.is_some_and(|context| !context.matches_registry(registry))
        || tokens.synthetic.is_some()
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let target = binding
        .proved_handler_target()
        .filter(|target| target.registry_backed)?;
    let effective = effective_words_for_target(tokens, target)?;
    let dialect = binding.variable_context.invocation_dialect;
    let RegistryInvocationResolution::Resolved(facts) =
        resolve_effective_tokens_with_metadata_context(
            registry, context, tokens, &effective, dialect,
        )
        .ok()?
    else {
        return None;
    };
    let invocation = ResolvedStatementInvocation {
        dialect,
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
        facts,
        authored_case_body_selection: None,
    };
    let topology = invocation.with_argument_words(|words| {
        invocation
            .facts
            .possible_handler_body_flow(
                registry,
                words.arguments(),
                binding.variable_context.alias_frame(),
            )?
            .topology()
    })?;
    let condition_values = match &topology {
        tcl_registry::native_compilation::PossibleBodyTopology::Conditional(branches) => branches
            .iter()
            .filter_map(|(condition, _)| *condition)
            .map(|argument| (argument, invocation.argument_literal(argument)))
            .collect(),
        _ => Vec::new(),
    };
    Some(PossibleBodyInvocation {
        dialect: invocation.dialect,
        topology,
        condition_values,
        effective: invocation.effective,
    })
}

/// Conditional topology from an original declaration's owned invocation
/// layout. This is lexical diagnostic advice only; it grants neither entered
/// phases nor successful dispatch and retains every runtime residual.
pub(crate) fn conditional_body_topology_advice_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<PossibleBodyInvocation> {
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) {
        return None;
    }
    let context = if logical_source {
        tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_input(
                tokens,
                context.source_analysis_input()?,
                registry,
            )?
    } else {
        context
    };
    conditional_runtime_body_topology_advice(registry, context, tokens, bindings).or_else(|| {
        original_compilation_body_topology_advice(registry, context, tokens).or_else(|| {
            original_layout_body_topology_advice(
                registry,
                context,
                tokens,
                &bindings.declaration_operand_layout_advice(tokens)?,
            )
        })
    })
}

fn conditional_runtime_body_topology_advice(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<PossibleBodyInvocation> {
    let binding = tokens.source_binding.as_ref()?;
    let invocation = if let Some(owned) =
        owned_body_layout_invocation(registry, context, tokens, bindings)
    {
        owned.invocation
    } else {
        // Known candidates retain lexical possibilities alongside an opaque
        // alternative. They cannot publish entered phases or close dispatch.
        let site = binding.invocation_site()?;
        native_compiler_replay_source(tokens, site)?;
        let entry = bindings.conditional_body_entry_at(&site.source, site.offset)?;
        let mut conditional = tokens.clone();
        conditional.source_binding =
            Some(bindings.conditional_invocation_at_source(&entry, site.offset));
        if !entry.owns_invocation(conditional.source_binding.as_ref()?) {
            return None;
        }
        let prepared = prepare_body_candidates(registry, context, &conditional)?;
        let mut unanimous = None;
        for candidate in &prepared.candidates {
            let selected = prepared_body_invocation(registry, context, &conditional, candidate)?;
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        if let Some(invocation) = unanimous {
            invocation
        } else {
            return original_compilation_body_topology_advice(registry, context, tokens);
        }
    };
    let topology = invocation.with_argument_words(|words| {
        invocation
            .facts
            .possible_handler_body_flow(
                registry,
                words.arguments(),
                binding.variable_context.alias_frame(),
            )?
            .topology()
    })?;
    let condition_values = match &topology {
        tcl_registry::native_compilation::PossibleBodyTopology::Conditional(branches) => branches
            .iter()
            .filter_map(|(condition, _)| *condition)
            .map(|argument| (argument, invocation.argument_literal(argument)))
            .collect(),
        _ => Vec::new(),
    };
    Some(PossibleBodyInvocation {
        dialect: invocation.dialect,
        topology,
        condition_values,
        effective: invocation.effective,
    })
}

/// An unchanged original expression operand for conditional diagnostics only.
/// Neither a normal handler nor successful operand evaluation is asserted.
pub(crate) struct OriginalExpressionOperandAdvice {
    pub(crate) expression: crate::expr_ast::ExprNode,
    pub(crate) dialect: tcl_registry::InvocationDialect,
    pub(crate) evaluation: tcl_registry::conditional_expression::ConditionalExpressionEvaluation,
    pub(crate) pool: tcl_registry::conditional_expression::ConditionalExpressionPoolState,
    pub(crate) expression_base: u32,
    pub(crate) expression_text: String,
    pub(crate) lookup_closed: bool,
    pub(crate) frame: crate::var_resolve::VariableExecutionFrame,
    pub(crate) namespace_key: crate::command_binding::SourceNamespaceKey,
    pub(crate) span: tcl_lexer::Span,
}

/// Original declaration/compiler lookup candidates must agree on the expression
/// operation and the same written operand. Runtime uncertainty remains intact.
pub(crate) fn original_expression_operand_advice(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<OriginalExpressionOperandAdvice> {
    original_expression_operand_advice_selected_word(registry, tokens, None)
}

/// Conditional checked expression topology for one actual written operand.
/// The selected descriptor owns its Expression role; source geometry alone
/// supplies neither this role nor an evaluation or successful handler.
pub(crate) fn original_expression_operand_advice_for_word(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    written: usize,
) -> Option<OriginalExpressionOperandAdvice> {
    original_expression_operand_advice_selected_word(registry, tokens, Some(written))
}

/// Independently selected conditional expression grammar at one unchanged
/// original written operand. This is a parser purpose, never Normal or edits.
pub(crate) struct OriginalExpressionContextAdvice {
    pub(crate) parser: tcl_syntax::expr::parser::ExprParseContext,
    pub(crate) dialect: tcl_registry::InvocationDialect,
    pub(crate) written: usize,
    pub(crate) lookup_closed: bool,
    pool: tcl_registry::conditional_expression::ConditionalExpressionPoolState,
    frame: crate::var_resolve::VariableExecutionFrame,
    namespace_key: crate::command_binding::SourceNamespaceKey,
}

/// Literal native bytes and logical source text borrow the same original
/// candidate/role/ordinal/configuration selector; neither reselects a parser.
pub(crate) fn original_literal_expression_context_advice(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<OriginalExpressionContextAdvice> {
    original_expression_context_advice_selected_word(registry, tokens, None)
}

fn original_expression_context_advice_selected_word(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    written: Option<usize>,
) -> Option<OriginalExpressionContextAdvice> {
    let binding = tokens.source_binding.as_ref()?;
    // Availability comes from this unchanged original point. Supplied missing
    // or stale ownership cannot be replaced by a catalogue/profile context.
    let context = binding
        .original_source_metadata_for_advice(tokens, registry)
        .ok()??;
    let advice = binding.original_conditional_operand_layout_advice(tokens, registry)?;
    let logical_source = advice.original_source_purpose(context, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if advice.has_opaque_handler_alternatives() {
        return None;
    }
    let selected = resolve_original_declared_layout(registry, context, tokens, &advice)?;
    // These are the selected descriptor's conditional expression slots. They
    // do not certify successful argument evaluation or an entered handler.
    let expressions =
        selected.with_metadata_schema(registry, context, advice.realm(), |schema| {
            if logical_source {
                schema.authored_logical_source_expression_arguments()
            } else {
                schema.authored_source_expression_arguments()
            }
        })?;
    let effective_index = if let Some(written) = written {
        let mut positions = expressions.arguments.iter().filter_map(|argument| {
            let effective_index = argument.checked_add(1)?;
            matches!(selected.effective.origins.get(effective_index),
                Some(InvocationWordOrigin::Written(index)) if *index == written)
            .then_some(effective_index)
        });
        let first = positions.next()?;
        if positions.next().is_some() {
            return None;
        }
        first
    } else {
        // The whole expression command's single source operand remains a
        // distinct purpose from one condition of a structured command.
        if selected.facts.successful_handler
            != Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::ExpressionArguments)
            || selected.effective.words.len() != 2
            || expressions.arguments != [0]
        {
            return None;
        }
        1
    };
    let InvocationWordOrigin::Written(original) =
        *selected.effective.origins.get(effective_index)?
    else {
        return None;
    };
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let dialect = advice.dialect();
    let mut parser = dialect.expression_parse_context(None);
    parser.lexer_grammar = config.grammar_over(parser.lexer_grammar);
    Some(OriginalExpressionContextAdvice {
        parser,
        dialect,
        written: original,
        lookup_closed: if logical_source {
            advice.closed_logical_source_lookup()
        } else {
            advice.closed_lookup()
        },
        pool: advice.expression_pool_state(),
        frame: advice.frame().clone(),
        namespace_key: advice.namespace().clone(),
    })
}

fn original_expression_operand_advice_selected_word(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    written: Option<usize>,
) -> Option<OriginalExpressionOperandAdvice> {
    let context = original_expression_context_advice_selected_word(registry, tokens, written)?;
    let binding = tokens.source_binding.as_ref()?;
    let index = context.written;
    let (WordExpr::BracedLiteral { text, source } | WordExpr::Literal { text, source }) =
        tokens.words().get(index)?
    else {
        return None;
    };
    if source.provenance != crate::ir::Provenance::Source {
        return None;
    }
    let base = crate::lowering_hooks::word_content_base(
        *tokens.argv.get(index)?,
        *tokens.single_token_word.get(index)?,
        text,
    )?;
    let end = base.checked_add(u32::try_from(text.len()).ok()?)?;
    let site = binding.invocation_site()?;
    if site
        .source
        .source_image()
        .bytes()
        .get(base as usize..end as usize)
        != Some(text.as_bytes())
    {
        return None;
    }
    let parser = context.parser;
    let evaluation =
        tcl_registry::conditional_expression::ConditionalExpressionEvaluation::prepare(
            text, &parser,
        )?;
    let expression = evaluation.tree().clone();
    Some(OriginalExpressionOperandAdvice {
        expression,
        dialect: context.dialect,
        evaluation,
        pool: context.pool,
        expression_base: base,
        expression_text: text.clone(),
        lookup_closed: context.lookup_closed,
        frame: context.frame,
        namespace_key: context.namespace_key,
        span: tcl_lexer::Span::new(
            tokens.words().first()?.source().span.start(),
            source.span.end(),
        ),
    })
}

/// Original compilation-table candidates select lexical topology only. Values
/// are decoded from unchanged source words under the captured logical grammar;
/// this never supplies runtime argument values or selected normal facts.
fn original_compilation_body_topology_advice<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<PossibleBodyInvocation> {
    let context = context.into();
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) {
        return None;
    }

    let binding = tokens.source_binding.as_ref()?;
    let advice = if logical_source {
        binding.original_conditional_operand_layout_advice(tokens, registry)?
    } else {
        binding.original_compilation_lookup_advice(tokens)?
    };
    original_layout_body_topology_advice(registry, context, tokens, &advice)
}

fn original_layout_body_topology_advice<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<PossibleBodyInvocation> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }

    let dialect = advice.dialect();
    let mut unanimous = None;
    for target in advice.targets() {
        let effective =
            compose_original_effective_words(tokens, &target.command, &target.prepended)?;
        let words: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let registry_words: Vec<_> = words
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                Some(context),
                &registry_words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?
        else {
            return None;
        };
        let invocation =
            InvocationWords::structured(*registry_words.first()?, &registry_words[1..])
                .with_dialect(dialect);
        let frame = if advice.alias_frame() == tcl_registry::VariableAliasFrame::Unknown {
            tokens
                .source_binding
                .as_ref()?
                .declaration_alias_frame_advice(tokens)?
        } else {
            advice.alias_frame()
        };
        let topology = facts
            .possible_handler_body_flow(registry, invocation.arguments(), frame)?
            .topology()?;
        let condition_values = match &topology {
            tcl_registry::native_compilation::PossibleBodyTopology::Conditional(branches) => {
                branches
                    .iter()
                    .filter_map(|(condition, _)| *condition)
                    .map(|argument| {
                        (
                            argument,
                            words.get(argument + 1).and_then(|word| {
                                if let EffectiveInvocationWord::Literal(value) = word {
                                    Some(value.clone())
                                } else {
                                    None
                                }
                            }),
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let selected = PossibleBodyInvocation {
            dialect: Some(dialect),
            topology,
            condition_values,
            effective,
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous: &PossibleBodyInvocation| {
                previous.topology != selected.topology
                    || previous.condition_values != selected.condition_values
                    || previous.effective != selected.effective
            })
        {
            return None;
        }
        unanimous = Some(selected);
    }
    unanimous
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
