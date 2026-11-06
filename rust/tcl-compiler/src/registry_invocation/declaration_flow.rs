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

//! Conditional source layout and completion for declaration diagnostics.

use super::{
    CommandRegistry, CommandTokens, EffectiveCommandWords, EffectiveInvocationWord, InvocationWord,
    InvocationWords, RegistryInvocationResolution, effective_invocation_word,
    effective_words_for_target, frozen_argument_words,
    resolve_registry_words_in_realm_with_metadata_context,
};
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::script_body_flow::ScriptBodyFlow;
use tcl_registry::{ArgRole, SemanticOperationId, Traits};

/// No executable facts escape this original-layout projection. All fields
/// describe the condition that the original native handler and local frame
/// are used; runtime dispatch and object callbacks remain independently open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclarationInvocationFlow {
    pub flow: ScriptBodyFlow,
    pub completion: tcl_registry::completion_route::InvocationCompletionRoute,
    pub effective: EffectiveCommandWords,
    pub quoted_operands: Vec<tcl_registry::body_execution::BodyOperand>,
    pub writes: Vec<String>,
    pub possible_output_writes: Vec<String>,
    pub reads: Vec<String>,
    pub name_queries: Vec<String>,
    pub loop_bindings: Vec<String>,
    pub literal_stores: Vec<(String, String)>,
    pub literal_store_operand: Option<usize>,
    pub increments: Vec<(String, String)>,
    pub first_list_iteration: Option<bool>,
    pub list_loop: bool,
    pub accumulator_writes: Vec<String>,
    pub removes: Vec<String>,
    /// Genuine written post-head operands of conditional destructions.
    /// Diagnostic spellings cannot reconstruct their native cell addresses.
    pub removal_arguments: Vec<usize>,
    pub aliases: Vec<String>,
    pub caller_frame_aliases: Vec<String>,
    pub caller_alias_operands: Vec<crate::ir::WordExpr>,
    /// Exact conditional schema, retained only by the representation graph.
    pub representation_words: Option<super::source_structure::OriginalRegistryWords>,
    /// Original point has the shared quiet scalar-context receipt.
    pub representation_quiet: bool,
    /// No known scalar observer or alias; unknown state remains conditional.
    pub representation_no_known_counterevidence: bool,
    pub effects: DeclarationEffectKnowledge,
}

impl DeclarationInvocationFlow {
    /// Compare complete flow, allowing only the already checked metadata-head
    /// coordinate to differ. This does not alter either retained carrier.
    fn representation_flow_corresponds(&self, other: &Self) -> bool {
        let Self {
            effective,
            flow,
            completion,
            quoted_operands,
            writes,
            possible_output_writes,
            reads,
            name_queries,
            loop_bindings,
            literal_stores,
            literal_store_operand,
            increments,
            first_list_iteration,
            list_loop,
            accumulator_writes,
            removes,
            removal_arguments,
            aliases,
            caller_frame_aliases,
            caller_alias_operands,
            representation_words,
            representation_quiet: _,
            representation_no_known_counterevidence: _,
            effects,
        } = self;
        let Some(left_head) = effective.words.first() else {
            return false;
        };
        let Some(right_head) = other.effective.words.first() else {
            return false;
        };
        left_head.source() == right_head.source()
            && effective.words.get(1..) == other.effective.words.get(1..)
            && effective.origins == other.effective.origins
            && effective.binding_prefix == other.effective.binding_prefix
            && flow == &other.flow
            && completion == &other.completion
            && quoted_operands == &other.quoted_operands
            && writes == &other.writes
            && possible_output_writes == &other.possible_output_writes
            && reads == &other.reads
            && name_queries == &other.name_queries
            && loop_bindings == &other.loop_bindings
            && literal_stores == &other.literal_stores
            && literal_store_operand == &other.literal_store_operand
            && increments == &other.increments
            && first_list_iteration == &other.first_list_iteration
            && list_loop == &other.list_loop
            && accumulator_writes == &other.accumulator_writes
            && removes == &other.removes
            && removal_arguments == &other.removal_arguments
            && aliases == &other.aliases
            && caller_frame_aliases == &other.caller_frame_aliases
            && caller_alias_operands == &other.caller_alias_operands
            && representation_words.is_some()
            && other.representation_words.is_none()
            && !other.representation_quiet
            && !other.representation_no_known_counterevidence
            && effects.representation_effects_correspond(&other.effects)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DeclarationFrameReach {
    Local,
    OtherFrame,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum DeclarationSchemaOrigin {
    #[default]
    OriginalLayout,
    OriginalSourceCatalogue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclarationEffectKnowledge {
    pub unknown_writes: bool,
    pub unknown_reads: bool,
    /// Original metadata leaves the command lookup table unchanged. This
    /// permits retaining a later declaration layout, never entered dispatch.
    pub lookup_stable: bool,
    /// Original body catalogue fields do not establish current invocation entry.
    pub schema_origin: DeclarationSchemaOrigin,
    /// Selected intrinsic can observe or select a different call frame.
    /// Nested body effects retain their independently parsed ownership.
    frame_reach: DeclarationFrameReach,
}

impl DeclarationEffectKnowledge {
    fn representation_effects_correspond(&self, other: &Self) -> bool {
        let Self {
            unknown_writes,
            unknown_reads,
            lookup_stable,
            schema_origin,
            frame_reach,
        } = self;
        *schema_origin == DeclarationSchemaOrigin::OriginalSourceCatalogue
            && other.schema_origin == DeclarationSchemaOrigin::OriginalLayout
            && unknown_writes == &other.unknown_writes
            && unknown_reads == &other.unknown_reads
            && lookup_stable == &other.lookup_stable
            && frame_reach == &other.frame_reach
    }

    pub(crate) fn frame_reachable(&self) -> bool {
        self.frame_reach == DeclarationFrameReach::OtherFrame
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclarationLifecycleInvocation {
    pub(crate) command: String,
    pub(crate) contract: tcl_registry::body_execution::CapturedLifecycleSpec,
    pub(crate) selection: tcl_registry::body_execution::CapturedLifecycleSelection,
    pub(crate) effective: EffectiveCommandWords,
    pub(crate) arguments: Vec<Option<String>>,
    pub(crate) argument_offset: usize,
    pub(crate) dialect: tcl_registry::InvocationDialect,
}

/// Exact original declaration layout under its selected provider semantics.
/// No installed command, entered phase or native CPP is granted.
pub(crate) fn declaration_lifecycle_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<DeclarationLifecycleInvocation> {
    let advice = tokens
        .source_binding
        .as_ref()?
        .declaration_operand_layout_advice(tokens)?;
    DeclarationLifecycleInvocation::for_layout(registry, tokens, &advice)
}

impl DeclarationLifecycleInvocation {
    pub(crate) fn for_layout(
        registry: &CommandRegistry,
        tokens: &CommandTokens,
        advice: &crate::command_binding::OriginalCompilationLookupAdvice,
    ) -> Option<DeclarationLifecycleInvocation> {
        use tcl_registry::body_execution::{BodyExecutionSelection, BodyExecutionSpec};
        let metadata = advice
            .original_declaration_metadata_context(registry, tokens)
            .ok()?;
        let dialect = advice.dialect();
        let mut agreed = None;
        for target in advice.targets() {
            let effective = effective_words_for_target(tokens, target)?;
            let values = declared_argument_words(tokens, &effective, dialect);
            let mut words = vec![InvocationWord::Literal(&target.command)];
            words.extend(values.iter().map(EffectiveInvocationWord::as_registry_word));
            let RegistryInvocationResolution::Resolved(facts) =
                resolve_registry_words_in_realm_with_metadata_context(
                    registry,
                    metadata,
                    &words,
                    Some(dialect),
                    advice.realm(),
                )
                .ok()?
            else {
                return None;
            };
            let BodyExecutionSpec::CapturedLifecycle(contract) = facts.body_execution? else {
                return None;
            };
            let arguments = tcl_registry::InvocationArguments::structured(
                words.get(facts.argument_offset + 1..)?,
            )
            .with_dialect(dialect);
            if contract.has_unmodelled_pre_phase_effects(arguments) {
                return None;
            }
            let BodyExecutionSelection::CapturedLifecycle(selection) = contract.select(arguments)
            else {
                return None;
            };
            let selected = DeclarationLifecycleInvocation {
                command: target.command.clone(),
                contract: *contract,
                selection,
                effective,
                arguments: values
                    .iter()
                    .map(|word| match word {
                        EffectiveInvocationWord::Literal(value) => Some(value.clone()),
                        _ => None,
                    })
                    .collect(),
                argument_offset: facts.argument_offset,
                dialect,
            };
            if agreed.as_ref().is_some_and(|old| old != &selected) {
                return None;
            }
            agreed = Some(selected);
        }
        agreed
    }
}

pub(crate) fn declaration_invocation_flow(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<DeclarationInvocationFlow> {
    declaration_invocation_flow_in_frame(
        registry,
        tokens,
        advice,
        tcl_registry::VariableAliasFrame::Procedure,
    )
}

pub(crate) fn declaration_invocation_flow_in_frame(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
    frame: tcl_registry::VariableAliasFrame,
) -> Option<DeclarationInvocationFlow> {
    let metadata = advice
        .original_declaration_metadata_context(registry, tokens)
        .ok()?;
    let dialect = advice.dialect();
    let mut agreed = None;
    for target in advice.targets() {
        let effective = effective_words_for_target(tokens, target)?;
        let mut values = vec![effective_invocation_word(
            effective.words.first()?,
            dialect.lexer_grammar.escapes,
            dialect.word_values,
        )];
        values.extend(declared_argument_words(tokens, &effective, dialect));
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                metadata,
                &words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?
        else {
            return None;
        };
        let invocation =
            InvocationWords::structured(*words.first()?, &words[1..]).with_dialect(dialect);
        let mut selected = declaration_flow_for_original_schema(
            registry, effective, &values, &facts, invocation, dialect, frame,
        )?;
        if facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions.facts().iter().any(|fact| {
                    matches!(
                        fact.transition,
                        tcl_registry::StateTransition::VariableCellAlias(_)
                    )
                })
            })
        {
            retain_declared_caller_alias_operands(&mut selected, registry, metadata, advice)?;
        }
        if agreed
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return None;
        }
        agreed = Some(selected);
    }
    agreed
}

/// Compare two original source selections under their retained metadata, without
/// granting handler or execution identity. Every declared target must agree.
pub(crate) fn declaration_source_flow_corresponds(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
    source_flow: &DeclarationInvocationFlow,
    observed_flow: &DeclarationInvocationFlow,
    context: &tcl_registry::model::ContextRegistry,
) -> Option<()> {
    let source = source_flow.representation_words.as_ref()?;
    let crate::ir::WordExpr::Literal { text, .. } = source_flow.effective.words.first()? else {
        return None;
    };
    if text != source.command()
        || source_flow.effective.origins.first() != Some(&super::InvocationWordOrigin::ResolvedHead)
        || !source_flow.representation_flow_corresponds(observed_flow)
    {
        return None;
    }
    let metadata = advice
        .original_declaration_metadata_context(registry, tokens)
        .ok()??;
    if !std::ptr::eq(metadata.context(), context.context()) {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let image = site.source.source_image();
    let native = super::original_native_compiler_words(image, tokens.words(), site.offset, config)?;
    let head = source.head_source()?.word()?;
    if !source.matches_source(image, config)
        || Some(head) != native.first()
        || head.span() != tokens.words().first()?.source().span
        || head.group().expand
    {
        return None;
    }
    let dialect = advice.dialect();
    let mut seen = false;
    for target in advice.targets() {
        seen = true;
        let effective = effective_words_for_target(tokens, target)?;
        if effective != observed_flow.effective {
            return None;
        }
        let mut values = vec![effective_invocation_word(
            effective.words.first()?,
            dialect.lexer_grammar.escapes,
            dialect.word_values,
        )];
        values.extend(declared_argument_words(tokens, &effective, dialect));
        let words: Vec<_> = values
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let invocation =
            InvocationWords::structured(*words.first()?, &words[1..]).with_dialect(dialect);
        let resolved =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                registry,
                Some(metadata.context()),
                invocation,
                advice.realm(),
            )
            .resolved()?;
        let matches = source.with_source_schema(context, |source_schema| {
            let left = resolved.authored_source_descriptors();
            let right = source_schema.authored_source_descriptors();
            let same_subcommand = match (left.subcommand, right.subcommand) {
                (None, None) => true,
                (Some(left), Some(right)) => std::ptr::eq(left, right),
                _ => false,
            };
            std::ptr::eq(left.command, right.command)
                && same_subcommand
                && resolved.semantics.argument_offset == source_schema.semantics.argument_offset
                && tcl_registry::command_descriptor_labels_match(&target.command, source.command())
        })?;
        if !matches {
            return None;
        }
    }
    seen.then_some(())
}

/// Readonly declaration flow from the exact shared original-body catalogue.
/// The result retains conditional source effects, never execution admission.
pub(crate) fn declaration_source_read_flow(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    scope: &crate::command_binding::SourceReadScope,
    frame: tcl_registry::VariableAliasFrame,
    retain_representation_words: bool,
) -> Option<DeclarationInvocationFlow> {
    let advice = scope.advice();
    let source = advice.site().source.source_image().try_text().ok()?;
    let words = super::source_structure::source_transition_words_from_advice(
        source,
        scope.config(),
        scope.context(),
        advice.clone(),
    )?;
    declaration_original_words_flow(
        registry,
        tokens,
        words,
        scope.context(),
        frame,
        retain_representation_words,
    )
}

/// Project the same retained original Registry words without changing their
/// source variant or applicability obligations.
pub(crate) fn declaration_original_words_flow(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
    words: super::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
    frame: tcl_registry::VariableAliasFrame,
    retain_representation_words: bool,
) -> Option<DeclarationInvocationFlow> {
    let dialect = words.dialect()?;
    let mut values = vec![EffectiveInvocationWord::Literal(words.command().to_owned())];
    values.extend(words.arguments().iter().map(|word| {
        match super::source_structure::source_schema_word(word.as_registry_word()) {
            InvocationWord::Literal(value) => EffectiveInvocationWord::Literal(value.to_owned()),
            _ => word.clone(),
        }
    }));
    let prefix = values
        .iter()
        .skip(1)
        .zip(words.origins().iter().skip(1))
        .filter_map(|(value, origin)| {
            matches!(origin, super::InvocationWordOrigin::BindingPrefix(_)).then_some(value.clone())
        })
        .collect::<Vec<_>>();
    let effective = super::compose_original_effective_words(tokens, words.command(), &prefix)?;
    if effective.origins.get(1..) != words.origins().get(1..) {
        return None;
    }
    let mut selected = words.with_source_schema(context, |schema| {
        let facts = schema.facts();
        // Caller links and lifecycle phases retain their independent owners.
        if facts
            .state_transitions
            .declared()
            .is_some_and(|transitions| {
                transitions.facts().iter().any(|fact| {
                    matches!(
                        fact.transition,
                        tcl_registry::StateTransition::VariableCellAlias(_)
                    )
                })
            })
        {
            return None;
        }
        let mut selected = declaration_flow_for_original_schema(
            registry,
            effective,
            &values,
            &facts,
            schema.words,
            dialect,
            frame,
        )?;
        selected.effects.schema_origin = DeclarationSchemaOrigin::OriginalSourceCatalogue;
        (selected.flow != ScriptBodyFlow::CapturedLifecycle).then_some(selected)
    })??;
    if retain_representation_words {
        selected.representation_quiet =
            super::authored_source_structure::authored_script_scalar_context_is_quiet(tokens);
        selected.representation_no_known_counterevidence = super::authored_source_structure::authored_script_scalar_context_has_no_known_counterevidence(tokens);
        selected.representation_words = Some(words);
    }
    Some(selected)
}

fn declaration_flow_for_original_schema(
    registry: &CommandRegistry,
    effective: EffectiveCommandWords,
    values: &[EffectiveInvocationWord],
    facts: &tcl_registry::InvocationFacts,
    invocation: InvocationWords<'_>,
    dialect: tcl_registry::InvocationDialect,
    frame: tcl_registry::VariableAliasFrame,
) -> Option<DeclarationInvocationFlow> {
    let arguments = invocation.arguments();
    let flow = tcl_registry::case_bodies::script_body_flow_in_registry(registry, facts, arguments);
    if !matches!(flow, ScriptBodyFlow::CaseBodies(_))
        && (!facts.arg_roles_complete || facts.arity_accepts_frozen_arguments() != Some(true))
    {
        return None;
    }
    let completion = registry
        .native_registration_completion_route(invocation, frame)
        .or_else(|| {
            registry.invocation_completion_route_in_frame(
                &facts.canonical_command,
                arguments,
                dialect.authoring_query(),
                frame,
            )
        })?;
    let quoted_operands = declared_quoted_operands(facts, arguments, &flow)?;
    let mut selected =
        declaration_flow_projection(flow, completion, effective, quoted_operands, facts);
    retain_declared_local_values(&mut selected, facts, arguments, dialect);
    retain_declared_aliases_and_case(&mut selected, registry, facts, arguments, dialect);
    retain_declared_variable_roles(&mut selected, facts, arguments, values, dialect)?;
    retain_declared_read_footprint(&mut selected, facts);
    Some(selected)
}

fn declaration_flow_projection(
    flow: ScriptBodyFlow,
    completion: tcl_registry::completion_route::InvocationCompletionRoute,
    effective: EffectiveCommandWords,
    quoted_operands: Vec<tcl_registry::body_execution::BodyOperand>,
    facts: &tcl_registry::InvocationFacts,
) -> DeclarationInvocationFlow {
    DeclarationInvocationFlow {
        flow,
        completion,
        effective,
        quoted_operands,
        writes: Vec::new(),
        possible_output_writes: Vec::new(),
        reads: Vec::new(),
        name_queries: Vec::new(),
        loop_bindings: Vec::new(),
        literal_stores: Vec::new(),
        literal_store_operand: None,
        increments: Vec::new(),
        first_list_iteration: None,
        list_loop: false,
        accumulator_writes: Vec::new(),
        removes: Vec::new(),
        removal_arguments: Vec::new(),
        aliases: Vec::new(),
        caller_frame_aliases: Vec::new(),
        caller_alias_operands: Vec::new(),
        representation_words: None,
        representation_quiet: false,
        representation_no_known_counterevidence: false,
        effects: DeclarationEffectKnowledge {
            unknown_writes: false,
            unknown_reads: facts.traits.contains(Traits::PERFORMS_SUBSTITUTION)
                || facts.effects.callback().kinds.is_unknown(),
            lookup_stable: declared_lookup_stable(facts),
            schema_origin: DeclarationSchemaOrigin::OriginalLayout,
            frame_reach: if facts
                .traits
                .intersects(tcl_registry::traits::FRAME_REACH_TRAITS)
                || facts.frame_effect.is_some()
            {
                DeclarationFrameReach::OtherFrame
            } else {
                DeclarationFrameReach::Local
            },
        },
    }
}

/// Original dictionary mapping under the declaration's selected handler.
/// This grants warning suppression only, independently of actual entry or CPP.
pub(crate) struct DeclarationDictionaryScopeAdvice {
    pub plan: tcl_registry::dictionary_scope::DictionaryScopePlan,
    pub arguments: Vec<Option<String>>,
}

pub(crate) fn declaration_dictionary_scope_advice(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<DeclarationDictionaryScopeAdvice> {
    let binding = tokens.source_binding.as_ref()?;
    let advice = binding.declaration_operand_layout_advice(tokens)?;
    let selected = declaration_invocation_flow(registry, tokens, &advice)?;
    let ScriptBodyFlow::DictionaryScope(
        tcl_registry::dictionary_scope::DictionaryScopeSelection::Selected(plan),
    ) = selected.flow
    else {
        return None;
    };
    let arguments = declared_argument_words(tokens, &selected.effective, advice.dialect())
        .into_iter()
        .map(|word| match word {
            EffectiveInvocationWord::Literal(value) => Some(value),
            _ => None,
        })
        .collect();
    Some(DeclarationDictionaryScopeAdvice { plan, arguments })
}

fn declared_lookup_stable(facts: &tcl_registry::InvocationFacts) -> bool {
    use tcl_registry::world_effect::CallbackKinds;
    #[cfg(debug_assertions)]
    let trace = |reason: &str| {
        if std::env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some() {
            eprintln!("RBS_DECLARED_LOOKUP refusal={reason}");
        }
    };
    #[cfg(not(debug_assertions))]
    let trace = |_reason: &str| {};
    let callbacks = facts.effects.callback().kinds;
    // Variable observers remain alternatives outside this conditional native
    // declaration. Explicit script, command-prefix and host callbacks do not.
    if callbacks.is_unknown() {
        trace("callback-unknown");
        return false;
    }
    if callbacks.contains(CallbackKinds::SCRIPT) {
        trace("callback-script");
        return false;
    }
    if callbacks.contains(CallbackKinds::COMMAND_PREFIX) {
        trace("callback-command-prefix");
        return false;
    }
    if callbacks.contains(CallbackKinds::HOST) {
        trace("callback-host");
        return false;
    }
    source_advice_has_no_declared_lookup_mutation_with(facts, trace)
}

/// Source-schema mutation footprint only; callbacks keep independent obligations.
pub(crate) fn source_advice_has_no_declared_lookup_mutation(
    facts: &tcl_registry::InvocationFacts,
) -> bool {
    source_advice_has_no_declared_lookup_mutation_with(facts, |_| {})
}

fn source_advice_has_no_declared_lookup_mutation_with(
    facts: &tcl_registry::InvocationFacts,
    trace: impl Fn(&str),
) -> bool {
    use tcl_registry::world_effect::{EffectAccessMode, WorldStateDomain};
    if facts
        .state_transitions
        .declared()
        .is_some_and(tcl_registry::StateTransitions::touches_command_bindings)
    {
        trace("declared-command-bindings-transition");
        return false;
    }
    for access in facts.effects.accesses() {
        if access.mode != EffectAccessMode::Read
            && matches!(
                access.domain,
                WorldStateDomain::CommandBindings
                    | WorldStateDomain::NamespaceLookup
                    | WorldStateDomain::NamespaceUnknown
                    | WorldStateDomain::InterpreterTopology
                    | WorldStateDomain::InterpreterPolicy
                    | WorldStateDomain::ExecutionTraces
                    | WorldStateDomain::VariableTraces
            )
        {
            let reason = match access.domain {
                WorldStateDomain::CommandBindings => "write-command-bindings",
                WorldStateDomain::NamespaceLookup => "write-namespace-lookup",
                WorldStateDomain::NamespaceUnknown => "write-namespace-unknown",
                WorldStateDomain::InterpreterTopology => "write-interpreter-topology",
                WorldStateDomain::InterpreterPolicy => "write-interpreter-policy",
                WorldStateDomain::ExecutionTraces => "write-execution-traces",
                WorldStateDomain::VariableTraces => "write-variable-traces",
                _ => unreachable!("matched lookup-stability domain"),
            };
            trace(reason);
            return false;
        }
    }
    true
}

/// Original inert operands keep their value even when an earlier abrupt route
/// leaves no entered argument observation. Substituted operands still require
/// the retained evaluation receipt; this projection grants declaration advice
/// only, independently of whether this statement executes.
pub(crate) fn declared_argument_words(
    tokens: &CommandTokens,
    effective: &EffectiveCommandWords,
    dialect: tcl_registry::InvocationDialect,
) -> Vec<EffectiveInvocationWord> {
    effective
        .words
        .iter()
        .skip(1)
        .zip(frozen_argument_words(tokens, effective))
        .map(|(word, evaluated)| {
            let original =
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values);
            if matches!(
                original,
                EffectiveInvocationWord::Literal(_) | EffectiveInvocationWord::ByteLiteral(_)
            ) {
                original
            } else {
                evaluated
            }
        })
        .collect()
}

/// Native caller-link transitions name a formal without asserting that the
/// transition ran or that its caller cell was read or written.
fn retain_declared_caller_alias_operands(
    selected: &mut DeclarationInvocationFlow,
    registry: &CommandRegistry,
    metadata: Option<super::InvocationMetadataContext<'_>>,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<()> {
    use tcl_registry::state_transition::{
        CallerFrameSelection, StateTransition, TransitionSubject, VariableAliasTarget,
    };
    let dialect = advice.dialect();
    // Formal provenance belongs to the unchanged source operand. A frozen
    // actual name must not replace that indexed input in a caller template.
    // Reuse the caller's checked point metadata; only its explicit Standalone
    // owner can supply None, and a failed supplied-owner check never reaches us.
    let original = selected
        .effective
        .words
        .iter()
        .map(|word| {
            effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
        })
        .collect::<Vec<_>>();
    let words = original
        .iter()
        .map(EffectiveInvocationWord::as_registry_word)
        .collect::<Vec<_>>();
    let RegistryInvocationResolution::Resolved(facts) =
        resolve_registry_words_in_realm_with_metadata_context(
            registry,
            metadata,
            &words,
            Some(dialect),
            advice.realm(),
        )
        .ok()?
    else {
        return None;
    };
    let Some(transitions) = facts.state_transitions.declared() else {
        return Some(());
    };
    for transition in transitions.facts() {
        let StateTransition::VariableCellAlias(alias) = &transition.transition else {
            continue;
        };
        let VariableAliasTarget::CallerSelectedFrame { frame, variable } = &alias.target else {
            continue;
        };
        let caller = match frame {
            CallerFrameSelection::DefaultCaller => true,
            CallerFrameSelection::Explicit(subject) => {
                subject
                    .literal()
                    .and_then(|value| tcl_registry::FrameLevel::parse_for_dialect(value, dialect))
                    == Some(tcl_registry::FrameLevel::DEFAULT)
            }
        };
        let TransitionSubject::Unknown { argument_index, .. } = variable else {
            continue;
        };
        if caller && let Some(word) = selected.effective.words.get(argument_index + 1) {
            selected.caller_alias_operands.push(word.clone());
        }
    }
    Some(())
}

/// Inert literal operands and case patterns retain diagnostic mentions only.
/// Executed bodies, expressions and variable/parameter declarations have
/// separate owners; their dollar spellings cannot donate data uses here.
fn declared_quoted_operands(
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    flow: &ScriptBodyFlow,
) -> Option<Vec<tcl_registry::body_execution::BodyOperand>> {
    use tcl_registry::body_execution::BodyOperand;
    let mut quoted = Vec::new();
    for argument in 0..arguments.exact_argv_len()? {
        let declared = facts.arg_roles.iter().any(|&(index, role)| {
            facts.argument_offset + usize::from(index) == argument
                && (role.carries_script()
                    || matches!(
                        role,
                        ArgRole::OpaqueScript
                            | ArgRole::Expr
                            | ArgRole::VarRead
                            | ArgRole::VarWrite
                            | ArgRole::LoopVarList
                            | ArgRole::ParamList
                            | ArgRole::StaticVarList
                    ))
        });
        let case_operand = matches!(flow, ScriptBodyFlow::CaseBodies(case)
            if case.bodies.iter().chain(&case.patterns)
                .any(|operand| operand.argument == argument));
        if !declared && !case_operand {
            quoted.push(BodyOperand {
                argument,
                list_element: None,
            });
        }
    }
    if let ScriptBodyFlow::CaseBodies(case) = flow {
        quoted.extend(case.patterns.iter().copied());
    }
    Some(quoted)
}

fn retain_declared_local_values(
    selected: &mut DeclarationInvocationFlow,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    dialect: tcl_registry::InvocationDialect,
) {
    if let Some(inputs) = facts
        .representation_effect
        .ordinary_list_pair_input_arguments(arguments, facts.argument_offset)
    {
        selected.list_loop = true;
        let lengths: Option<Vec<_>> = inputs
            .iter()
            .map(|&index| {
                let value = arguments.literal_at(index)?;
                Some(!dialect.word_values.split_list(value).ok()?.is_empty())
            })
            .collect();
        selected.first_list_iteration =
            lengths.map(|lengths| lengths.into_iter().any(|nonempty| nonempty));
    }
    if facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        && arguments.exact_argv_len() == Some(facts.argument_offset + 2)
        && let Some(name) = arguments.literal_at(facts.argument_offset)
        && let Some(value) = arguments.literal_at(facts.argument_offset + 1)
    {
        selected
            .literal_stores
            .push((name.to_owned(), value.to_owned()));
        selected.literal_store_operand = Some(facts.argument_offset + 1);
    }
    if facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Incr)
        && let Some(name) = arguments.literal_at(facts.argument_offset)
    {
        let amount = if arguments.exact_argv_len() == Some(facts.argument_offset + 1) {
            Some("1")
        } else {
            arguments.literal_at(facts.argument_offset + 1)
        };
        if let Some(amount) = amount {
            selected
                .increments
                .push((name.to_owned(), amount.to_owned()));
        }
    }
}

fn retain_declared_aliases_and_case(
    selected: &mut DeclarationInvocationFlow,
    registry: &CommandRegistry,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    dialect: tcl_registry::InvocationDialect,
) {
    // Possible case operands do not predict the selected arm. A fixed
    // subject needs an independently selected matcher to certify reads in
    // a particular arm; otherwise retain an unavailable conditional path.
    if matches!(selected.flow, ScriptBodyFlow::CaseBodies(_))
        && let Some(spec) = registry.get(&facts.canonical_command)
        && let Some(case) = spec.case_list
        && let Some(literals) = arguments.literal_values()
        && case
            .invocation(
                &literals,
                &spec.options.iter().collect::<Vec<_>>(),
                dialect.authoring_query(),
            )
            .is_some_and(|layout| layout.subject_index.is_some())
    {
        selected.effects.unknown_writes = true;
    }
    if let Some(transitions) = facts.state_transitions.declared() {
        for fact in transitions.facts() {
            if let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition {
                if let Some(local) = alias.local.literal() {
                    selected.aliases.push(local.to_owned());
                    if declared_alias_targets_caller(alias, dialect) {
                        selected.caller_frame_aliases.push(local.to_owned());
                    }
                } else {
                    selected.effects.unknown_writes = true;
                }
            }
        }
    }
    if facts
        .traits
        .intersects(Traits::CREATES_SCOPE_ALIAS | Traits::ALIASES_GLOBAL)
        && selected.aliases.is_empty()
    {
        // Missing or dynamic alias destinations do not close the declared
        // local frame. Written operands cannot reconstruct a local tail.
        selected.effects.unknown_writes = true;
    }
}

fn declared_alias_targets_caller(
    alias: &tcl_registry::VariableCellAliasTransition,
    dialect: tcl_registry::InvocationDialect,
) -> bool {
    // naming.tcloo.original-declared-receiver-caller-traits
    // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-receiver-caller-traits.md
    use tcl_registry::state_transition::{CallerFrameSelection, VariableAliasTarget};
    let VariableAliasTarget::CallerSelectedFrame { frame, variable } = &alias.target else {
        return false;
    };
    // A computed target keeps the conditional alias operation unresolved,
    // even when its destination and default caller frame are authored.
    if variable.literal().is_none() {
        return false;
    }
    match frame {
        CallerFrameSelection::DefaultCaller => true,
        CallerFrameSelection::Explicit(subject) => subject
            .literal()
            .and_then(|value| tcl_registry::FrameLevel::parse_for_dialect(value, dialect))
            .is_some_and(tcl_registry::FrameLevel::is_caller_frame),
    }
}

fn retain_declared_variable_roles(
    selected: &mut DeclarationInvocationFlow,
    facts: &tcl_registry::InvocationFacts,
    arguments: tcl_registry::InvocationArguments<'_>,
    values: &[EffectiveInvocationWord],
    dialect: tcl_registry::InvocationDialect,
) -> Option<()> {
    let output = facts.successful_variable_output_commitments(arguments);
    let conditional_output = matches!(
        facts.successful_handler,
        Some(
            tcl_registry::native_compilation::SuccessfulHandlerSpec::ConditionalVariableOperands(_)
        )
    );

    for &(position, role) in &facts.arg_roles {
        let argument = facts.argument_offset + usize::from(position);
        if !matches!(
            role,
            ArgRole::VarWrite | ArgRole::VarRead | ArgRole::LoopVarList
        ) {
            continue;
        }
        let Some(EffectiveInvocationWord::Literal(name)) = values.get(argument + 1) else {
            if matches!(role, ArgRole::VarWrite | ArgRole::LoopVarList) {
                selected.effects.unknown_writes = true;
            }
            if role == ArgRole::VarRead {
                selected.effects.unknown_reads = true;
            }
            continue;
        };
        let existence_name = crate::existence_query::operand(facts)
            .is_some_and(|(_, position)| position == argument);
        if role == ArgRole::VarRead && existence_name {
            selected.name_queries.push(name.clone());
        }
        if (role == ArgRole::VarRead && !existence_name)
            || (role == ArgRole::VarWrite && facts.traits.contains(Traits::READS_BEFORE_WRITE))
        {
            selected.reads.push(
                tcl_syntax::naming::split_element_ref(name)
                    .map_or(name.as_str(), |(root, _)| root)
                    .to_owned(),
            );
        }
        if role == ArgRole::LoopVarList {
            selected.loop_bindings.extend(
                dialect
                    .word_values
                    .split_list(name)
                    .ok()?
                    .into_iter()
                    .map(std::borrow::Cow::into_owned),
            );
        } else if facts
            .traits
            .intersects(Traits::CREATES_SCOPE_ALIAS | Traits::ALIASES_GLOBAL)
        {
            // The transition owner, rather than the written target name,
            // determines qualified local tails and selected frame aliases.
        } else if facts.operation == SemanticOperationId::StructuredLowering(LoweringHookId::Unset)
        {
            selected.removes.push(name.clone());
            if let Some(written) = selected.effective.written_argument(argument) {
                selected.removal_arguments.push(written);
            }
        } else if role == ArgRole::VarWrite
            && conditional_output
            && output.as_ref().is_none_or(|outputs| {
                outputs.iter().any(|(index, commitment)| {
                    *index == argument
                        && *commitment
                            == tcl_registry::variable_output::VariableOutputCommitment::MayWrite
                })
            })
        {
            // Conditional output syntax outside the bounded matcher can
            // initialise this exact authored destination. This definition
            // belongs only to the declared diagnostic interpretation.
            selected.possible_output_writes.push(name.clone());
        } else if role == ArgRole::VarWrite
            && ((!conditional_output && output.is_none())
                || output.as_ref().is_some_and(|outputs| {
                    outputs.iter().any(|(index, commitment)| {
                        *index == argument
                            && *commitment
                                == tcl_registry::variable_output::VariableOutputCommitment::Written
                    })
                }))
        {
            selected.writes.push(name.clone());
        }
    }
    Some(())
}

fn retain_declared_read_footprint(
    selected: &mut DeclarationInvocationFlow,
    facts: &tcl_registry::InvocationFacts,
) {
    if selected.reads.is_empty()
        && selected.name_queries.is_empty()
        && facts.effects.accesses().iter().any(|access| {
            access.domain == tcl_registry::world_effect::WorldStateDomain::VariableStore
                && matches!(
                    access.mode,
                    tcl_registry::world_effect::EffectAccessMode::Read
                        | tcl_registry::world_effect::EffectAccessMode::ReadWrite
                        | tcl_registry::world_effect::EffectAccessMode::Clobber
                )
        })
    {
        selected.effects.unknown_reads = true;
    }
    if matches!(
        facts.var_elements_effect,
        Some(tcl_registry::types::VarElementsEffect::AppendsListElements { .. })
    ) {
        selected.accumulator_writes = selected.writes.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_invocation::effective_command_words;

    #[test]
    fn original_inert_declaration_values_survive_missing_entered_observations() {
        let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
        let dialect = tcl_registry::InvocationDialect::of_profile(profile);
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        for (source, expected) in [
            (
                "set {a} \"1\"",
                vec![
                    EffectiveInvocationWord::Literal("a".into()),
                    EffectiveInvocationWord::Literal("1".into()),
                ],
            ),
            (
                "set $target $value",
                vec![
                    EffectiveInvocationWord::Dynamic,
                    EffectiveInvocationWord::Dynamic,
                ],
            ),
        ] {
            let image = tcl_lexer::SourceImage::document(source);
            let segment =
                crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                    .remove(0);
            let mut tokens = CommandTokens::from_segmented(&image.source_map(), config, &segment);
            let effective = effective_command_words(&tokens).unwrap();
            let mut binding = crate::command_binding::SourceInvocationBinding::default();
            binding.evaluated_argument_words = vec![EffectiveInvocationWord::Dynamic; 2];
            tokens.source_binding = Some(binding);
            assert_eq!(
                declared_argument_words(&tokens, &effective, dialect),
                expected,
                "{source}"
            );
            assert_eq!(
                frozen_argument_words(&tokens, &effective),
                vec![EffectiveInvocationWord::Dynamic; 2]
            );
        }
    }
}
