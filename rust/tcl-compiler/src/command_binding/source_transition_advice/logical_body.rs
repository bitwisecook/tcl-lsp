// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deferred Logical source headers retain their original parent body/schema.

use super::body_effects::{
    OriginalOperandEffect, boxed_body_advice, boxed_body_resolution, boxed_region_words,
    original_operand_effects,
};
use super::{
    AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    AdvicePublicationPurpose, OriginalSourceCommandTransitionAdvice,
    OriginalSourceTransitionAdviceTape, SourceAdviceNameInput, SourceCommandTransitionObligation,
    capture_transition, original_words, registry_words,
};
use crate::command_binding::source_declared_command::{
    OriginalDeclaredLogicalBodyContext, OriginalDeclaredSourceBodyFrame,
};
use crate::registry_invocation::OriginalSourceScriptBody;
use std::sync::Arc;
use tcl_lexer::{NativeScriptCommandWords, NativeWord, Span};
use tcl_registry::{CommandBindingDefinitionKind, CommandBindingTransition, Traits};

/// Authentic readonly procedure header, formals and deferred body context.
/// The source card supplies no command allocation, publication or body entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalLogicalProcedureBody {
    scope: Arc<OriginalDeclaredLogicalBodyContext>,
    site: crate::command_binding::CommandAllocationSite,
    summary_coordinate: tcl_core_types::ByteCommandSlot,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    parameters_text: String,
    formal_original: NativeWord,
    formal_topology: Option<Arc<crate::command_binding::formal_topology::OriginalFormalTopology>>,
    earlier_unknown_mutations: bool,
    module_entry: Option<crate::command_binding::SourceAnalysisEntry>,
    parameter_source_prefixes: Arc<super::source_parameter_template::ParameterSourcePrefixCache>,
}
impl OriginalLogicalProcedureBody {
    /// Conditional readonly source recipe at an authentic own-body command.
    /// Cache contents are excluded from this body's semantic equality.
    pub(crate) fn parameter_source_prefix(
        &self,
        original: &[NativeWord],
    ) -> Option<&super::OriginalLogicalParameterSourcePrefix> {
        self.parameter_source_prefixes.get(original)
    }

    pub(crate) fn retain_module_entry(
        mut self,
        entry: &crate::command_binding::SourceAnalysisEntry,
    ) -> Self {
        self.module_entry = Some(entry.clone());
        self
    }
    pub(crate) fn scope(&self) -> &Arc<OriginalDeclaredLogicalBodyContext> {
        &self.scope
    }
    pub(crate) fn site(&self) -> &crate::command_binding::CommandAllocationSite {
        &self.site
    }
    /// Conditional header name geometry; never an installed command slot.
    pub(crate) fn summary_coordinate(&self) -> &tcl_core_types::ByteCommandSlot {
        &self.summary_coordinate
    }
    pub(crate) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        &self.parameters
    }
    pub(crate) fn parameters_text(&self) -> &str {
        &self.parameters_text
    }
    pub(crate) fn formal_original(&self) -> &NativeWord {
        &self.formal_original
    }
    pub(crate) fn formal_topology(
        &self,
    ) -> Option<&crate::command_binding::formal_topology::OriginalFormalTopology> {
        self.formal_topology.as_deref()
    }
    pub(crate) fn has_earlier_unknown_mutations(&self) -> bool {
        self.earlier_unknown_mutations
    }
    pub(crate) fn name(&self) -> Option<&str> {
        std::str::from_utf8(self.scope.scope_name_input()?.bytes()).ok()
    }
    pub(crate) fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> bool {
        self.summary_coordinate.namespace == *self.scope.namespace_recipe()
            && self.scope.scope_name_input().is_some_and(|name| {
                self.summary_coordinate.simple.as_bytes()
                    == tcl_syntax::naming::written_command_tail(name.bytes())
            })
            && self.site.source.source_image() == image
            && self.scope.matches_input(input)
            && self
                .scope
                .scope_body()
                .matches_source(image, input.lexer_config())
            && self
                .scope
                .scope_body()
                .matches_context(input.borrowed_context_registry())
            && self.scope.frame_recipe() == OriginalDeclaredSourceBodyFrame::Procedure
    }
    pub(crate) fn matches_procedure(
        &self,
        module: &crate::ir::Module,
        procedure: &crate::ir::Procedure,
    ) -> bool {
        let Some(input) = module.source_metadata_input.as_ref() else {
            return false;
        };
        self.module_entry.as_ref() == Some(&module.source_entry)
            && module.source_entry.native_entry.is_none()
            && module.source_entry.hosted_execution_context.is_none()
            && !module.source_entry.unknown_entry
            && module.native_lexer_config() == input.lexer_config()
            && self.matches_source(&module.source, input)
            && procedure.declaration_site.as_ref() == Some(&self.site)
            && procedure.body_kind == crate::ir::ProcedureBodyKind::OriginalDeclaration
            && self.name() == Some(procedure.name.as_str())
            && procedure.params_raw == self.parameters_text
            && procedure
                .params
                .iter()
                .map(String::as_str)
                .eq(self.parameters.iter().map(|p| p.name.as_str()))
            && procedure.body.namespace_context.as_deref()
                == tcl_syntax::naming::checked_namespace_path_key(self.scope.namespace_recipe())
                    .map(crate::command_binding::SourceNamespaceKey::authored)
                    .as_ref()
            && procedure.body.executed_source.is_none()
            && procedure.body_offset == self.scope.scope_body().content_span().start()
            && procedure.body_source.as_deref()
                == module
                    .source
                    .try_text()
                    .ok()
                    .and_then(|s| s.get(self.scope.scope_body().content_span().as_range()))
    }
}

const MAX_DEPTH: usize = 64;
const MAX_COMMANDS: usize = 2048;

impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn logical_variable_frame_at_offset(
        &self,
        input: &crate::analyser::ResolvedAnalysisInput,
        image: &tcl_lexer::SourceImage,
        offset: u32,
    ) -> Option<crate::command_binding::SourceOriginalVariableFrame> {
        let length = self
            .logical_variable_scopes
            .keys()
            .filter(|(start, end)| *start <= offset && offset <= *end)
            .map(|(start, end)| end - start)
            .min()?;
        let mut selected = self
            .logical_variable_scopes
            .iter()
            .filter(|((start, end), _)| {
                *start <= offset && offset <= *end && end - start == length
            });
        let ((start, end), scope) = selected.next()?;
        let scope = scope.as_ref()?;
        if selected.any(|(_, other)| other.as_ref() != Some(scope))
            || !scope.matches_input(input)
            || scope.body().content_span() != Span::new(*start, *end)
            || !scope.body().matches_source(image, input.lexer_config())
            || !matches!(
                scope.frame_recipe(),
                OriginalDeclaredSourceBodyFrame::Procedure
                    | OriginalDeclaredSourceBodyFrame::Lambda
            )
        {
            return None;
        }
        if scope.frame_recipe() == OriginalDeclaredSourceBodyFrame::Lambda {
            return crate::command_binding::SourceOriginalVariableFrame::from_logical_lambda_source(
                Arc::clone(scope.lambda_source()?), input,
            ).filter(|frame| frame.source_region_cursor_is_direct(image, scope.body(), offset));
        }
        let mut cards = self
            .logical_procedure_bodies
            .values()
            .flatten()
            .filter(|card| {
                std::ptr::eq(card.scope().scope_body(), scope.scope_body())
                    && card.scope().scope_name_input().is_some()
                    && card.scope().scope_name_input() == scope.scope_name_input()
                    && card.scope().namespace_recipe() == scope.namespace_recipe()
                    && card.scope().frame_recipe() == scope.frame_recipe()
                    && card.matches_source(image, input)
            });
        let card = cards.next()?;
        cards.all(|other| other == card).then_some(())?;
        crate::command_binding::SourceOriginalVariableFrame::from_logical_procedure_source(
            card, input,
        )
        .filter(|frame| frame.source_region_cursor_is_direct(image, scope.body(), offset))
    }
}

struct LogicalBodyWalk<'context, 'source> {
    context: &'context AdviceInvocationContext<'source>,
    visited: usize,
}

struct PreparedLogicalBodyBranch {
    graph: AdviceGraph,
    scope: Arc<OriginalDeclaredLogicalBodyContext>,
}

pub(super) struct LogicalNamespaceScope {
    name: super::OriginalLogicalSourceNameInput,
    namespace: tcl_core_types::ByteNamespacePath,
    receipt: Arc<super::OriginalSourceCommandTransition>,
}
impl LogicalNamespaceScope {
    pub(super) fn body_scope(
        &self,
        body: Arc<OriginalSourceScriptBody>,
        input: &crate::analyser::ResolvedAnalysisInput,
        parent: Option<Arc<OriginalDeclaredLogicalBodyContext>>,
    ) -> Option<Arc<OriginalDeclaredLogicalBodyContext>> {
        OriginalDeclaredLogicalBodyContext::capture(
            body,
            self.name.clone(),
            self.namespace.clone(),
            OriginalDeclaredSourceBodyFrame::Namespace,
            input,
            parent,
        )
        .map(Arc::new)
    }
}

impl AdviceInvocationContext<'_> {
    pub(super) fn logical_namespace_scope(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<LogicalNamespaceScope> {
        if !matches!(self.policy, AdviceNamingPolicy::Logical(_))
            || schema.facts().arity_accepts_frozen_arguments() != Some(true)
            || !schema.semantics.traits.contains(Traits::DECLARES_NAMESPACE)
            || schema.semantics.body_kind != tcl_registry::BodyKind::Structural
            || schema.semantics.body_interpreter.resolve_with(|ordinal| {
                std::str::from_utf8(invocation.arguments.get(ordinal)?.value.as_deref()?).ok()
            }) != tcl_registry::world_effect::InterpreterScope::Current
        {
            return None;
        }
        let transitions = schema.state_transitions();
        let mut names = transitions
            .facts()
            .iter()
            .filter_map(|fact| match &fact.transition {
                tcl_registry::StateTransition::Namespace(
                    tcl_registry::NamespaceTransition::Ensure {
                        namespace: tcl_registry::NamespaceTransitionTarget::Named(name),
                    },
                ) => Some(name),
                _ => None,
            });
        let name = names.next()?;
        if names.next().is_some() {
            return None;
        }
        let receipt = capture_transition(
            self.origin,
            invocation.native,
            schema.canonical_command,
            invocation.arguments,
            transitions.clone(),
            invocation.lineage,
        )?;
        let name = receipt.input(name)?.logical_input()?.clone();
        let namespace = tcl_syntax::naming::authored_source_namespace_path(
            graph.logical_namespace.as_ref()?,
            name.bytes(),
        )?;
        Some(LogicalNamespaceScope {
            name,
            namespace,
            receipt,
        })
    }

    /// An own conditional inventory never publishes child mutations to the
    /// enclosing graph or issues an entered procedure/frame receipt.
    pub(super) fn retain_logical_procedure_body(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        if !matches!(self.policy, AdviceNamingPolicy::Logical(_)) {
            return;
        }
        self.retain_logical_lambda_formatting_phase(tape, graph, invocation, schema, None);
        let mut retained = OriginalSourceTransitionAdviceTape::default();
        let mut walk = LogicalBodyWalk {
            context: self,
            visited: 0,
        };
        if walk
            .parent_scope(&mut retained, graph, invocation, schema, None, 0)
            .is_some()
        {
            tape.extend_inventory(retained);
        } else {
            if (schema.authored_source_lambda_call().is_some()
                || schema.authored_source_procedure_arguments().is_some()
                || schema.semantics.traits.contains(Traits::DECLARES_NAMESPACE))
                && let Some(head) = invocation.native.first()
            {
                tape.logical_procedure_catalogue_refusals.insert(
                    super::LogicalProcedureCatalogueRefusal::SelectedScopeUnavailable(
                        head.span().start(),
                    ),
                );
            }
            self.withdraw_missing_publication_phase(schema);
        }
    }
}

impl LogicalBodyWalk<'_, '_> {
    fn parent_scope(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        parent: Option<&Arc<OriginalDeclaredLogicalBodyContext>>,
        depth: usize,
    ) -> Option<()> {
        if schema.authored_source_lambda_call().is_some() {
            self.lambda(tape, graph, invocation, schema, parent, depth)
        } else if schema.authored_source_procedure_arguments().is_some() {
            self.procedure(tape, graph, invocation, schema, parent, depth)
        } else {
            self.namespace(tape, graph, invocation, schema, parent, depth)
        }
    }

    fn lambda(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        parent: Option<&Arc<OriginalDeclaredLogicalBodyContext>>,
        depth: usize,
    ) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        let AdviceNamingPolicy::Logical(input) = self.context.policy else {
            return None;
        };
        let mut advice = boxed_body_advice(self.context, invocation, graph, schema)?;
        advice.logical_body = parent.map(|parent| Arc::clone(parent.body_arc()));
        advice.logical_scope = parent.cloned();
        let words = boxed_region_words(self.context, &advice)?;
        let policy = self.context.dialect.authored_name_policy()?;
        let purpose =
            crate::signature_scan::original_name::OriginalSourceNamePurpose::for_analysis_input(
                input, None, policy,
            );
        let lambda = Arc::new(crate::command_binding::OriginalLogicalLambdaBody::capture(
            input,
            *words,
            invocation.native,
            purpose,
        )?);
        let scope = Arc::new(OriginalDeclaredLogicalBodyContext::capture_lambda(
            lambda,
            input,
            parent.cloned(),
        )?);
        let mut branch = graph.clone();
        branch.logical_namespace = Some(scope.namespace_recipe().clone());
        branch.handles.clear();
        branch.class_handles.clear();
        self.script(tape, &mut branch, &scope, depth.checked_add(1)?)
    }

    fn procedure(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        parent: Option<&Arc<OriginalDeclaredLogicalBodyContext>>,
        depth: usize,
    ) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        let Some(mut prepared) = self.prepare_procedure(tape, graph, invocation, schema, parent)
        else {
            // This selected declaration attempt exists even when no complete
            // header card can be issued. Catalogue readers must retain refusal.
            super::source_class::merge_receipt(
                &mut tape.logical_procedure_bodies,
                invocation.native.first()?.span().start(),
                None,
            );
            return None;
        };
        self.script(
            tape,
            &mut prepared.graph,
            &prepared.scope,
            depth.checked_add(1)?,
        )?;
        // Only this successful actual walk can populate the derived source
        // prefix sidecar. Missing/partial walks cannot mint positive receipts.
        if let Some(card) = tape
            .logical_procedure_bodies
            .get(&invocation.native.first()?.span().start())
            .and_then(Option::as_ref)
            .cloned()
        {
            let prefixes = super::source_parameter_template::capture_prefixes(
                tape,
                &card,
                self.context.context,
            );
            if let Some(prefixes) = prefixes {
                card.parameter_source_prefixes.retain(prefixes);
            }
        }
        Some(())
    }

    #[inline(never)]
    fn prepare_procedure(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        parent: Option<&Arc<OriginalDeclaredLogicalBodyContext>>,
    ) -> Option<Box<PreparedLogicalBodyBranch>> {
        let positions = schema.authored_source_procedure_arguments()?;
        let transitions = schema.state_transitions();
        let mut definitions =
            transitions
                .command_bindings()
                .filter_map(|transition| match transition {
                    CommandBindingTransition::Define {
                        name,
                        kind: CommandBindingDefinitionKind::Procedure,
                    } => Some(name),
                    _ => None,
                });
        let name = definitions.next()?;
        if definitions.next().is_some() {
            return None;
        }
        let receipt = capture_transition(
            self.context.origin,
            invocation.native,
            schema.canonical_command,
            invocation.arguments,
            transitions.clone(),
            invocation.lineage,
        )?;
        let name_input = receipt.input(name)?.logical_input()?.clone();
        let key = graph.publication_key(name_input.bytes(), AdvicePublicationPurpose::Define)?;
        let mut advice = boxed_body_advice(self.context, invocation, graph, schema)?;
        advice.logical_body = parent.map(|parent| Arc::clone(parent.body_arc()));
        advice.logical_scope = parent.cloned();
        if schema.semantics.body_interpreter.resolve_with(|ordinal| {
            std::str::from_utf8(advice.arguments().get(ordinal)?.value.as_deref()?).ok()
        }) != tcl_registry::world_effect::InterpreterScope::Current
        {
            return None;
        }
        let bodies = self.bodies(&advice)?;
        if bodies.len() != 1 {
            return None;
        }
        let AdviceNamingPolicy::Logical(input) = self.context.policy else {
            return None;
        };
        let body = Arc::new(bodies.into_iter().next()?);
        let scope = Arc::new(OriginalDeclaredLogicalBodyContext::capture(
            body,
            name_input,
            key.namespace.clone(),
            OriginalDeclaredSourceBodyFrame::Procedure,
            input,
            parent.cloned(),
        )?);
        let formal = invocation.arguments.get(positions.parameters)?;
        let parameters_text = std::str::from_utf8(formal.value.as_deref()?).ok()?;
        if tcl_syntax::word_rules::original_static_word_unicode_value(&formal.original).as_deref()
            != Some(parameters_text.as_bytes())
        {
            return None;
        }
        let parameters = crate::command_binding::logical_definition::static_formals(
            parameters_text,
            self.context.config.list_parse,
        )?;
        let formal_topology = self.formal_topology(&formal.original, input);
        let card = Arc::new(OriginalLogicalProcedureBody {
            scope: Arc::clone(&scope),
            site: crate::command_binding::CommandAllocationSite {
                source: Arc::clone(self.context.origin),
                offset: invocation.native.first()?.span().start(),
            },
            summary_coordinate: key.clone(),
            parameters,
            parameters_text: parameters_text.to_owned(),
            formal_original: formal.original.clone(),
            formal_topology,
            earlier_unknown_mutations: !graph.uncertain_operations.is_empty(),
            module_entry: None,
            parameter_source_prefixes: Arc::default(),
        });
        super::source_class::merge_receipt(
            &mut tape.logical_procedure_bodies,
            card.site.offset,
            Some(card),
        );
        let branch = Self::prepare_procedure_branch(graph, &receipt, key.namespace)?;
        Some(Box::new(PreparedLogicalBodyBranch {
            graph: branch,
            scope,
        }))
    }

    fn prepare_procedure_branch(
        graph: &AdviceGraph,
        receipt: &Arc<super::OriginalSourceCommandTransition>,
        namespace: tcl_core_types::ByteNamespacePath,
    ) -> Option<AdviceGraph> {
        let mut branch = graph.clone();
        branch.apply(receipt)?;
        branch.logical_namespace = Some(namespace);
        branch.handles.clear();
        branch.class_handles.clear();
        Some(branch)
    }

    fn formal_topology(
        &self,
        formal: &NativeWord,
        input: &crate::analyser::ResolvedAnalysisInput,
    ) -> Option<Arc<crate::command_binding::formal_topology::OriginalFormalTopology>> {
        let dialect = crate::command_binding::source_analysis_entry::source_input_dialect(input);
        dialect.authored_name_policy().and_then(|policy| {
            crate::signature_scan::scope::SignatureSourceNameKey::from_original_logical_source_word(
                formal,
                tcl_syntax::word_rules::WordValueRules::from_config(&self.context.config),
                policy,
            )
            .and_then(|key| {
                crate::command_binding::formal_topology::OriginalFormalTopology::from_original_key(
                    key, dialect,
                )
            })
            .map(Arc::new)
        })
    }

    fn namespace(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        parent: Option<&Arc<OriginalDeclaredLogicalBodyContext>>,
        depth: usize,
    ) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        let mut prepared = self.prepare_namespace(graph, invocation, schema, parent)?;
        self.script(
            tape,
            &mut prepared.graph,
            &prepared.scope,
            depth.checked_add(1)?,
        )
    }

    #[inline(never)]
    fn prepare_namespace(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        parent: Option<&Arc<OriginalDeclaredLogicalBodyContext>>,
    ) -> Option<Box<PreparedLogicalBodyBranch>> {
        let selected = self
            .context
            .logical_namespace_scope(graph, invocation, schema)?;
        let mut advice = boxed_body_advice(self.context, invocation, graph, schema)?;
        advice.logical_body = parent.map(|parent| Arc::clone(parent.body_arc()));
        advice.logical_scope = parent.cloned();
        let bodies = self.bodies(&advice)?;
        if bodies.len() != 1 {
            return None;
        }
        let AdviceNamingPolicy::Logical(input) = self.context.policy else {
            return None;
        };
        let scope =
            selected.body_scope(Arc::new(bodies.into_iter().next()?), input, parent.cloned())?;
        let mut branch = graph.clone();
        branch.apply(&selected.receipt)?;
        branch.logical_namespace = Some(selected.namespace);
        Some(Box::new(PreparedLogicalBodyBranch {
            graph: branch,
            scope,
        }))
    }

    #[inline(never)]
    fn bodies(
        &self,
        advice: &OriginalSourceCommandTransitionAdvice,
    ) -> Option<Vec<OriginalSourceScriptBody>> {
        let words = boxed_region_words(self.context, advice)?;
        let bodies = if matches!(
            self.context.query,
            Some(super::SourceAdviceQuery::Publication(_))
        ) {
            words.source_script_bodies_for_mutation_coverage(self.context.context)?
        } else {
            let bodies = words.source_script_bodies(self.context.context);
            if bodies.is_empty() {
                words
                    .normal_source_case_bodies(self.context.context)
                    .and_then(|cases| cases.bodies().map(<[_]>::to_vec))
                    .unwrap_or_default()
            } else {
                bodies
            }
        };
        if bodies.iter().any(|body| {
            let container = body.original_container();
            let content = container.content_span().ok();
            !body.matches_source(self.context.origin.source_image(), self.context.config)
                || !body.matches_context(self.context.context)
                || content.and_then(|span| container.image().bytes().get(span.as_range()))
                    != tcl_syntax::word_rules::original_static_word_source_bytes(container)
                        .as_deref()
        }) {
            return None;
        }
        Some(bodies)
    }

    fn script(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        scope: &Arc<OriginalDeclaredLogicalBodyContext>,
        depth: usize,
    ) -> Option<()> {
        self.script_at(tape, graph, scope, scope.body().content_span(), depth)
    }

    fn script_at(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        scope: &Arc<OriginalDeclaredLogicalBodyContext>,
        span: Span,
        depth: usize,
    ) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        let body = scope.body().content_span();
        super::source_class::merge_receipt(
            &mut tape.logical_variable_scopes,
            (body.start(), body.end()),
            Some(Arc::clone(scope)),
        );
        let plan = tcl_lexer::native_script_words_in(
            self.context.origin.source_image().clone(),
            span,
            self.context.config,
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        for command in plan.commands {
            self.visited = self.visited.checked_add(1)?;
            if self.visited > MAX_COMMANDS {
                return None;
            }
            self.command(tape, graph, &command, scope, depth)?;
        }
        Some(())
    }

    fn command(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        command: &NativeScriptCommandWords,
        scope: &Arc<OriginalDeclaredLogicalBodyContext>,
        depth: usize,
    ) -> Option<()> {
        self.context.observe_publication_proposal(graph);
        tape.retain_arity_prefix(graph, &command.words);
        let retained = self.command_inner(tape, graph, command, scope, depth);
        self.context.observe_publication_proposal(graph);
        retained
    }

    fn command_inner(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        command: &NativeScriptCommandWords,
        scope: &Arc<OriginalDeclaredLogicalBodyContext>,
        depth: usize,
    ) -> Option<()> {
        let native = &command.words;
        tape.represented.insert(native.first()?.span().start());
        tape.retain_logical_summary_scope(native, scope);
        tape.retain_logical_navigation(graph, native, scope);
        let Some(written) = original_words(native, self.context.policy) else {
            graph.widen(native);
            return Some(());
        };
        if self.context.retain_original_body_constructor_calls(
            tape,
            graph,
            native,
            (
                scope.body_arc(),
                SourceCommandTransitionObligation::DeferredLogicalBodyApplicability,
            ),
        ) {
            return Some(());
        }
        if self
            .context
            .retain_registered_invocation(tape, graph, native, &written)
        {
            self.context.retain_registered_body_owner(
                tape,
                native.first()?.span().start(),
                scope.body_arc(),
            );
            self.context
                .finish_registered_invocation(graph, native, &written);
            return Some(());
        }
        let handle = self
            .context
            .before_operand_handle_binding(graph, native, &written);
        let class_handle = self.context.retain_source_class_handle_body(
            self.context
                .before_operand_class_handle(tape, graph, native, &written),
            scope.body_arc(),
            SourceCommandTransitionObligation::DeferredLogicalBodyApplicability,
        );
        let Some(head) = written
            .first()
            .and_then(|word| word.input.as_ref())
            .cloned()
        else {
            self.inspect_deferred_operands(tape, graph, native, (scope, depth))?;
            tape.registry_barriers
                .insert(native.first()?.span().start());
            graph.widen(native);
            return Some(());
        };
        let original_selection = class_handle
            .is_some()
            .then(|| graph.resolve(&head))
            .flatten();
        let operand_barrier =
            self.retain_deferred_operand_header(tape, graph, native, &head, (scope, depth))?;
        OriginalSourceTransitionAdviceTape::retain_logical_procedure_call(tape, graph, native);
        let Some((selected, mut arguments, lineage)) = original_selection
            .or_else(|| graph.resolve(&head))
            .or_else(|| graph.readonly_logical_source_candidate(&head, native, scope))
        else {
            graph.widen(native);
            return Some(());
        };
        for (ordinal, argument) in arguments.iter_mut().enumerate() {
            argument.origin =
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.into_iter().skip(1));
        let values = registry_words(&arguments);
        let resolution = boxed_body_resolution(
            self.context.context,
            self.context.dialect,
            &selected,
            &values,
        );
        let Some(schema) = resolution.resolved() else {
            graph.widen(native);
            return Some(());
        };
        let invocation = AdviceInvocation {
            native,
            head: &head,
            arguments: &arguments,
            lineage: &lineage,
        };
        self.context.retain_arity_debt(graph, invocation, &schema)?;
        self.retain_original_deferred_invocation(tape, graph, invocation, &schema, (scope, depth))?;
        self.apply_deferred_invocation_effects(
            tape,
            graph,
            invocation,
            &selected,
            &schema,
            (scope, handle, class_handle, operand_barrier),
        )?;
        Some(())
    }

    fn retain_deferred_operand_header(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        native: &[NativeWord],
        head: &SourceAdviceNameInput,
        parent: (&Arc<OriginalDeclaredLogicalBodyContext>, usize),
    ) -> Option<bool> {
        let (scope, _) = parent;
        let operand_barrier = self.inspect_deferred_operands(tape, graph, native, parent)?;
        if graph.blocks_registry_source(head) {
            tape.registry_barriers
                .insert(native.first()?.span().start());
        }
        if let Some(selection) = graph
            .declared_source_selection(self.context.origin, head, native)
            .and_then(|selection| selection.with_logical_body(Arc::clone(scope)))
        {
            tape.retain_declared(selection);
        }
        Some(operand_barrier)
    }

    fn inspect_deferred_operands(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        native: &[NativeWord],
        parent: (&Arc<OriginalDeclaredLogicalBodyContext>, usize),
    ) -> Option<bool> {
        let (scope, depth) = parent;
        let effects = original_operand_effects(native);
        let operand_barrier = !effects.is_empty();
        if operand_barrier {
            graph.record_uncertainty(native);
        }
        for effect in effects {
            match effect {
                OriginalOperandEffect::Command(body) => {
                    self.script_at(tape, graph, scope, body, depth.checked_add(1)?)?;
                }
                OriginalOperandEffect::Unknown => graph.record_uncertainty(native),
            }
        }
        Some(operand_barrier)
    }

    fn apply_deferred_invocation_effects(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        invocation: AdviceInvocation<'_>,
        selected: &str,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        owner: (
            &Arc<OriginalDeclaredLogicalBodyContext>,
            Option<Arc<super::OriginalSourceRegisteredHandleBinding>>,
            Option<super::source_instance::PendingSourceClassHandle>,
            bool,
        ),
    ) -> Option<()> {
        let (scope, handle, class_handle, operand_barrier) = owner;
        let native = invocation.native;
        let factory = self.context.registered_factory(graph, invocation, schema);
        let authored_jim = self.context.authored_jim_class_source(
            graph,
            invocation,
            schema,
            Some(scope.body_arc()),
        );
        let class = self.context.source_class_declaration_in_body(
            tape,
            graph,
            invocation,
            schema,
            (
                scope.body_arc(),
                SourceCommandTransitionObligation::DeferredLogicalBodyApplicability,
                Some(scope),
            ),
        );
        let (roles, complete) = self.context.authored_roles(schema);
        let immediate_body = roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
            && !schema.semantics.traits.contains(Traits::DEFERS_BODY)
            && schema.authored_source_procedure_arguments().is_none();
        if !complete || immediate_body {
            self.context.apply_schema_transition(
                graph,
                invocation,
                selected,
                schema,
                Some(scope),
            )?;
        } else {
            self.context
                .apply_source_body_lookup_effects(graph, selected, invocation, schema);
        }
        if operand_barrier {
            graph.widen(native);
        }
        self.context
            .install_registered_results(tape, graph, factory, handle);
        AdviceInvocationContext::install_source_class_handle(graph, class_handle);
        AdviceInvocationContext::install_source_class(tape, graph, class);
        AdviceInvocationContext::install_authored_jim_class(tape, graph, authored_jim);
        Some(())
    }

    fn retain_original_deferred_invocation(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        parent: (&Arc<OriginalDeclaredLogicalBodyContext>, usize),
    ) -> Option<()> {
        let (scope, depth) = parent;
        let mut advice = boxed_body_advice(self.context, invocation, graph, schema)?;
        advice.logical_body = Some(Arc::clone(scope.body_arc()));
        advice.logical_scope = Some(Arc::clone(scope));
        advice
            .obligations
            .push(SourceCommandTransitionObligation::DeferredLogicalBodyApplicability);
        self.context
            .retain_authored_command_prefix(tape, invocation, graph, schema, Some(&advice));
        Self::retain_advice(tape, &advice);
        self.context
            .retain_logical_math_functions(tape, graph, &advice, schema);
        self.context
            .retain_source_callback_targets(tape, invocation, graph, schema);
        self.context.retain_logical_callback_candidates(
            tape,
            invocation,
            graph,
            schema,
            super::logical_navigation::LogicalCallbackScope::Declared(scope),
        );
        self.context.retain_produced_command_prefixes(
            tape,
            invocation,
            graph,
            schema,
            Some(&advice),
        );
        self.children(tape, graph, invocation, (&advice, schema), scope, depth)?;
        Some(())
    }

    #[inline(never)]
    fn boxed_inventory() -> Box<OriginalSourceTransitionAdviceTape> {
        Box::new(OriginalSourceTransitionAdviceTape::default())
    }

    #[inline(never)]
    fn retain_inventory(
        tape: &mut OriginalSourceTransitionAdviceTape,
        retained: Box<OriginalSourceTransitionAdviceTape>,
    ) {
        tape.extend_inventory(*retained);
    }

    #[inline(never)]
    fn retain_advice(
        tape: &mut OriginalSourceTransitionAdviceTape,
        advice: &OriginalSourceCommandTransitionAdvice,
    ) {
        tape.advice.insert(advice.site.offset, advice.clone());
    }

    #[inline(never)]
    fn prepare_child_body(
        graph: &AdviceGraph,
        scope: &Arc<OriginalDeclaredLogicalBodyContext>,
        body: OriginalSourceScriptBody,
        frame_level: Option<tcl_registry::FrameLevel>,
    ) -> Option<Box<PreparedLogicalBodyBranch>> {
        let body = Arc::new(body);
        let child = if frame_level.is_some_and(tcl_registry::FrameLevel::is_global_frame) {
            scope.with_global_body(body)?
        } else {
            scope.with_body(body)?
        };
        let mut branch = graph.clone();
        branch.logical_namespace = Some(child.namespace_recipe().clone());
        Some(Box::new(PreparedLogicalBodyBranch {
            graph: branch,
            scope: Arc::new(child),
        }))
    }

    fn children(
        &mut self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        selected: (
            &OriginalSourceCommandTransitionAdvice,
            &tcl_registry::ResolvedInvocation<'_, '_>,
        ),
        scope: &Arc<OriginalDeclaredLogicalBodyContext>,
        depth: usize,
    ) -> Option<()> {
        let (advice, schema) = selected;
        self.context.retain_logical_lambda_formatting_phase(
            tape,
            graph,
            invocation,
            schema,
            Some(scope),
        );
        if schema.authored_source_lambda_call().is_some()
            || schema.authored_source_procedure_arguments().is_some()
            || schema.semantics.traits.contains(Traits::DECLARES_NAMESPACE)
        {
            // Each changing source scope retains its own selected name/body.
            // Unmodelled workers never inherit a procedure namespace/frame.
            let mut retained = Self::boxed_inventory();
            if self
                .parent_scope(&mut retained, graph, invocation, schema, Some(scope), depth)
                .is_some()
            {
                Self::retain_inventory(tape, retained);
            } else {
                if let Some(head) = invocation.native.first() {
                    tape.logical_procedure_catalogue_refusals.insert(
                        super::LogicalProcedureCatalogueRefusal::SelectedScopeUnavailable(
                            head.span().start(),
                        ),
                    );
                }
                self.context.withdraw_missing_publication_phase(schema);
            }
            return Some(());
        }
        let selected_frame = schema.semantics.frame_effect.is_some_and(|frame| {
            frame.layout == tcl_registry::FrameArgLayout::ScriptInSelectedFrame
        });
        if schema.semantics.body_kind != tcl_registry::BodyKind::Plain && !selected_frame {
            self.context.withdraw_missing_publication_phase(schema);
            return Some(());
        }
        let frame_level = if selected_frame {
            let Some(tcl_registry::frame_effect::FrameArgumentResolution::Valid { level, .. }) =
                schema.authored_logical_source_frame_argument_layout()
            else {
                self.context.withdraw_missing_publication_phase(schema);
                return Some(());
            };
            if !level.is_current_frame() && !level.is_global_frame() {
                self.context.withdraw_missing_publication_phase(schema);
                return Some(());
            }
            Some(level)
        } else {
            None
        };
        if matches!(
            self.context.query,
            Some(super::SourceAdviceQuery::Publication(_))
        ) && schema
            .authored_logical_source_argument_roles()
            .0
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Expr)
        {
            self.context.withdraw_missing_publication_phase(schema);
        }
        for body in self.bodies(advice)? {
            let mut prepared = Self::prepare_child_body(graph, scope, body, frame_level)?;
            self.script(
                tape,
                &mut prepared.graph,
                &prepared.scope,
                depth.checked_add(1)?,
            )?;
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_registry_words,
    };

    fn analyse_source(source: &str) -> crate::analyser::AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::context_for_profile(profile),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    fn words_at(
        source: &str,
        analysis: &crate::analyser::AnalysisResult,
        fragment: &str,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        let offset = source.find(fragment)?;
        let config = analysis.body_lexer_config?;
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            fragment,
            u32::try_from(offset).ok()?,
            config,
        )
        .remove(0);
        source_registry_words(source, analysis, &segment)
    }

    #[test]
    fn logical_cursor_regions_keep_selected_same_frame_bodies_and_inert_data_distinct() {
        for source in [
            "proc p {} {while 1 {set nested 1; }}",
            "proc p {} {if 1 {set nested 1; }}",
            "apply {{} {while 1 {set nested 1; }}}",
            "apply {{} {if 1 {set nested 1; }}}",
        ] {
            let analysis = analyse_source(source);
            let image = tcl_lexer::SourceImage::document(source);
            let config = analysis.body_lexer_config.unwrap();
            let offset = u32::try_from(source.find("nested").unwrap()).unwrap();
            let frame = analysis
                .original_variable_frame_in_source(&image, config, offset)
                .expect(source);
            assert!(frame.source().text.try_text().unwrap().contains("nested"));
            let mut changed = config;
            changed.strict_quoting = !changed.strict_quoting;
            assert!(
                analysis
                    .original_variable_frame_in_source(&image, changed, offset)
                    .is_none()
            );
        }
        for source in [
            "proc p {} {set data {inert}}",
            "apply {{} {set data {inert}}}",
            "proc p {} {namespace eval N {set inert 1}}",
            "apply {{} {namespace eval N {set inert 1}}}",
            "proc p {} {apply {{required} {set inert 1}}}",
        ] {
            let analysis = analyse_source(source);
            let image = tcl_lexer::SourceImage::document(source);
            let offset = u32::try_from(source.find("inert").unwrap()).unwrap();
            assert!(
                analysis
                    .original_variable_frame_in_source(
                        &image,
                        analysis.body_lexer_config.unwrap(),
                        offset
                    )
                    .is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn logical_lambda_children_use_their_own_scope_and_keep_outer_unknown_effects() {
        for source in [
            "apply {{} {set local 1; puts $local}}",
            "namespace eval Caller {apply {{} {set local 1; puts $local}}}",
        ] {
            let analysis = analyse_source(source);
            let input = analysis.resolved_input.as_ref().unwrap();
            let words = words_at(source, &analysis, "set local 1")
                .expect("genuine explicit lambda child inventory");
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("Logical source owner");
            };
            let scope = advice
                .logical_source_scope()
                .expect("actual distinct lambda source scope");
            assert_eq!(
                scope.frame_recipe(),
                crate::command_binding::OriginalDeclaredSourceBodyFrame::Lambda
            );
            assert!(scope.scope_name_input().is_none());
            assert!(scope.namespace_recipe().is_root());
            assert!(scope.lambda_source().is_some());
            assert!(scope.matches_input(input));
            assert!(advice.original_head().native_input().is_none());
            let mut foreign_config = input.lexer_config();
            foreign_config.strict_quoting = !foreign_config.strict_quoting;
            let foreign = crate::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                input.context_registry(),
                foreign_config,
            );
            assert!(!scope.matches_input(&foreign));
            let image = tcl_lexer::SourceImage::document(source);
            let config = input.lexer_config();
            let write = u32::try_from(source.find("local 1").unwrap()).unwrap();
            let read = u32::try_from(source.find("$local").unwrap() + 1).unwrap();
            let write = analysis
                .original_variable_symbol_in_source(&image, config, write)
                .expect("source-only lambda local declaration");
            let read = analysis
                .original_variable_symbol_in_source(&image, config, read)
                .expect("source-only lambda local reference");
            assert_eq!(write.symbol(), read.symbol());
            assert!(matches!(
                read.symbol().slot(),
                crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::Local { .. }
            ));
            assert!(!analysis.original_variable_rename_is_complete(read.symbol()));
        }
        for (source, child, later) in [
            (
                "apply {{} {rename ::set {}}}; set after 1",
                "rename ::set {}",
                "set after 1",
            ),
            (
                "namespace eval N {apply {{} {namespace eval X {proc introduced {} {}}}}; set after 1}",
                "namespace eval X {proc introduced {} {}}",
                "set after 1",
            ),
        ] {
            let analysis = analyse_source(source);
            assert!(words_at(source, &analysis, child).is_some(), "{source}");
            if let Some(later) = words_at(source, &analysis, later) {
                let OriginalRegistrySource::SourceTransitions(advice) = later.source() else {
                    panic!("original Logical later call");
                };
                assert!(
                    advice.obligations().contains(
                        &super::SourceCommandTransitionObligation::UnknownEarlierMutation
                    ) || !advice.uncertain_operations.is_empty(),
                    "lambda inventory cannot close outer effects: {source}"
                );
            }
        }
        for source in [
            "apply {{argument} {set local 1}}",
            r"apply {{{argument D\u0045FAULT}} {set local 1}}",
            "proc apply args {}; apply {{} {set local 1}}",
        ] {
            let analysis = analyse_source(source);
            assert!(
                words_at(source, &analysis, "set local 1").is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn logical_selected_frame_children_keep_root_current_and_unknown_source_owners_distinct() {
        for (source, fragment, expected_level) in [
            (
                "namespace eval N {proc outer {} {uplevel #0 {proc inner {} {set x INNER; puts $x}}}}\n",
                "proc inner {} {set x INNER; puts $x}",
                tcl_registry::FrameLevel::Absolute(0),
            ),
            (
                "namespace eval N {proc outer {} {uplevel 0 {proc inner {} {set x INNER; puts $x}}}}\n",
                "proc inner {} {set x INNER; puts $x}",
                tcl_registry::FrameLevel::Relative(0),
            ),
        ] {
            let analysis = analyse_source(source);
            let input = analysis.resolved_input.as_ref().unwrap();
            let words = words_at(source, &analysis, fragment).expect(fragment);
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("selected original source frame body")
            };
            assert_eq!(advice.logical_source_input(), Some(input));
            let parent = advice.logical_source_body().unwrap();
            assert_eq!(
                parent
                    .source_words()
                    .with_source_schema(input.borrowed_context_registry(), |schema| {
                        schema.authored_logical_source_frame_argument_layout()
                    }),
                Some(Some(
                    tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                        level_word_len: 1,
                        level: expected_level,
                    }
                ))
            );
            assert!(
                !words
                    .source_script_bodies(input.borrowed_context_registry())
                    .is_empty()
            );
            assert!(words_at(source, &analysis, "puts $x").is_some());
            assert!(!words.operands_preserve_source_lookup());
            assert!(advice.original_head().native_input().is_none());
            assert!(words_at(&format!("{source} "), &analysis, fragment).is_none());
            let mut changed = analysis.clone();
            changed.body_lexer_config = Some(tcl_lexer::LexerConfig {
                strict_quoting: !input.lexer_config().strict_quoting,
                ..input.lexer_config()
            });
            assert!(words_at(source, &changed, fragment).is_none());
        }
    }

    #[test]
    fn logical_sequential_selected_frame_bodies_keep_exact_source_owners() {
        let source = "set x ROOT\nnamespace eval N {proc p {} {set x LOCAL; uplevel #0 {puts $x}; uplevel 0 {puts $x}}}\n";
        let analysis = analyse_source(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        for (fragment, level) in [
            ("uplevel #0 {puts $x}", b"#0".as_slice()),
            ("uplevel 0 {puts $x}", b"0".as_slice()),
        ] {
            let words = words_at(source, &analysis, fragment).unwrap();
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("actual original selected-frame advice")
            };
            assert_eq!(advice.logical_source_input(), Some(input));
            assert!(advice.original_head().native_input().is_none());
            let offset = source.find(fragment).unwrap() + fragment.find("puts").unwrap();
            let tail = &source[offset..];
            let segment = crate::segmenter::segment_commands_with_offset_and_config(
                &tail[..tail.find('}').unwrap()],
                u32::try_from(offset).unwrap(),
                input.lexer_config(),
            )
            .remove(0);
            let child = source_registry_words(source, &analysis, &segment).unwrap();
            let OriginalRegistrySource::SourceTransitions(child_advice) = child.source() else {
                panic!("actual original child advice")
            };
            let owner = child_advice.logical_source_body().unwrap();
            assert_eq!(
                owner.original_container().image().bytes(),
                source.as_bytes()
            );
            assert_eq!(
                owner.source_words().arguments()[0].literal_bytes(),
                Some(level)
            );
            assert!(words_at(&format!("{source} "), &analysis, fragment).is_none());
        }
    }

    #[test]
    fn logical_selected_frame_mutations_still_withdraw_following_source_advice() {
        for body in [
            "uplevel #0 {rename ::uplevel {}}",
            "uplevel 0 {rename ::uplevel {}}",
            "uplevel #0 {unknown missing}",
            "uplevel $level {puts $x}",
            "uplevel 1 {puts $x}",
        ] {
            let source = format!(
                "namespace eval N {{proc p {{}} {{set x LOCAL; {body}; uplevel 0 {{puts $x}}}}}}\n"
            );
            let analysis = analyse_source(&source);
            assert!(
                words_at(&source, &analysis, "uplevel 0 {puts $x}").is_none(),
                "{body}"
            );
        }
    }

    #[test]
    fn logical_empty_transition_recipe_keeps_residual_naming_writes_uncovered() {
        let source = "namespace eval N {proc p {} {unknown missing; uplevel #0 {puts $x}}}";
        let analysis = analyse_source(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        let words = words_at(source, &analysis, "unknown missing").unwrap();
        assert_eq!(
            words.with_source_schema(input.borrowed_context_registry(), |schema| {
                assert!(schema.state_transitions().facts().is_empty());
                schema.effects().accesses().iter().any(|access| {
                    access.mode != tcl_registry::world_effect::EffectAccessMode::Read
                        && matches!(
                            access.domain,
                            tcl_registry::world_effect::WorldStateDomain::CommandBindings
                                | tcl_registry::world_effect::WorldStateDomain::NamespaceLookup
                        )
                })
            }),
            Some(true)
        );
        assert!(words_at(source, &analysis, "uplevel #0 {puts $x}").is_none());
    }

    #[test]
    fn logical_deferred_source_lookup_keeps_scalar_effects_and_binding_barriers_separate() {
        for (source, fragment) in [
            (
                "set x ROOT\nnamespace eval N {proc p {} {set x LOCAL; uplevel #0 {puts $x}}}\n",
                "uplevel #0 {puts $x}",
            ),
            (
                "set x ROOT\nnamespace eval N {proc p {} {set x LOCAL; uplevel 0 {puts $x}}}\n",
                "uplevel 0 {puts $x}",
            ),
        ] {
            let analysis = analyse_source(source);
            let input = analysis.resolved_input.as_ref().unwrap();
            let words = words_at(source, &analysis, fragment).expect(fragment);
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("original conditional Logical body owner")
            };
            assert_eq!(advice.logical_source_input(), Some(input));
            assert!(advice.original_head().native_input().is_none());
            assert!(advice.obligations().contains(
                &super::SourceCommandTransitionObligation::DeferredLogicalBodyApplicability
            ));
            assert!(advice.logical_source_body().unwrap().matches_source(
                &tcl_lexer::SourceImage::document(source),
                input.lexer_config(),
            ));
            assert_eq!(
                words.with_source_schema(input.borrowed_context_registry(), |schema| schema
                    .authored_logical_source_argument_roles()
                    .1),
                Some(true)
            );
            assert!(
                !words
                    .source_script_bodies(input.borrowed_context_registry())
                    .is_empty()
            );
            assert!(!words.operands_preserve_source_lookup());
            assert!(words_at(&format!("{source} "), &analysis, fragment).is_none());
            let mut changed = analysis.clone();
            changed.body_lexer_config = Some(tcl_lexer::LexerConfig {
                strict_quoting: !input.lexer_config().strict_quoting,
                ..input.lexer_config()
            });
            assert!(words_at(source, &changed, fragment).is_none());
        }
        for source in [
            "namespace eval N {proc uplevel args {}; proc p {} {set x LOCAL; uplevel #0 {puts $x}}}",
            "namespace eval N {proc p {} {set x LOCAL; rename ::uplevel {}; uplevel #0 {puts $x}}}",
            "namespace eval N {proc p {} {set x LOCAL; unknown; uplevel #0 {puts $x}}}",
            "namespace eval N {proc p {} {set x LOCAL; unknown missing; uplevel #0 {puts $x}}}",
            "namespace eval N {proc p {} {set x LOCAL; command_without_catalogue; uplevel #0 {puts $x}}}",
        ] {
            let analysis = analyse_source(source);
            assert!(
                words_at(source, &analysis, "uplevel #0 {puts $x}").is_none(),
                "{source}"
            );
        }
    }

    #[test]
    fn logical_deferred_source_body_retains_nested_schema_and_parent_geometry() {
        let source = "proc p {v} {foreach x $v {if {$x} {puts $x}}}\n";
        let analysis = analyse_source(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        for (fragment, complete_roles) in [
            ("foreach x $v {if {$x} {puts $x}}", true),
            ("if {$x} {puts $x}", true),
            ("puts $x", false),
        ] {
            let words = words_at(source, &analysis, fragment).expect(fragment);
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("missing genuine Logical deferred body owner");
            };
            assert_eq!(advice.logical_source_input(), Some(input));
            assert!(advice.original_head().native_input().is_none());
            assert!(advice.obligations().contains(
                &super::SourceCommandTransitionObligation::DeferredLogicalBodyApplicability
            ));
            let body = advice.logical_source_body().unwrap();
            assert!(body.matches_source(
                &tcl_lexer::SourceImage::document(source),
                input.lexer_config()
            ));
            assert!(body.matches_context(&input.context_registry()));
            assert!(
                advice
                    .original_words()
                    .iter()
                    .all(|word| word.span().start() >= body.content_span().start()
                        && word.span().end() <= body.content_span().end())
            );
            assert!(!words.operands_preserve_source_lookup());
            // The dynamic puts operand cannot select its Channel role.
            // Complete control-body roles remain independently required.
            assert_eq!(words.roles().is_some(), complete_roles, "{fragment}");
            assert_eq!(
                words.with_source_schema(&input.context_registry(), |schema| schema
                    .authored_source_argument_roles()
                    .1),
                Some(complete_roles)
            );
        }
    }

    #[test]
    fn logical_deferred_source_body_keeps_mutations_and_cooked_geometry_separate() {
        let source = "proc p {} {proc puts {args} {}; puts inner}\nputs outer\n";
        let analysis = analyse_source(source);
        assert!(words_at(source, &analysis, "puts inner").is_none());
        assert!(words_at(source, &analysis, "puts outer").is_some());
        let cooked = r#"proc p {} "puts\u0020inner""#;
        let cooked_analysis = analyse_source(cooked);
        let parent = words_at(cooked, &cooked_analysis, cooked).unwrap();
        assert!(
            parent
                .source_script_bodies(
                    &cooked_analysis
                        .resolved_input
                        .as_ref()
                        .unwrap()
                        .context_registry()
                )
                .is_empty()
        );
        assert!(words_at(cooked, &cooked_analysis, r"puts\u0020inner").is_none());
        for source in [
            "proc puts {args} {}; proc p {} {puts inner}",
            "proc A::p {} {puts inner}",
        ] {
            let analysis = analyse_source(source);
            assert!(
                words_at(source, &analysis, "puts inner").is_none(),
                "{source}"
            );
        }
    }
    #[test]
    fn logical_deferred_substitutions_keep_original_body_and_evaluation_order() {
        let source = "proc p {} {return [join [string index abc 99]]}\n";
        let analysis = analyse_source(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        for fragment in ["join [string index abc 99]", "string index abc 99"] {
            let words = words_at(source, &analysis, fragment).expect(fragment);
            let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                panic!("missing deferred original substitution body");
            };
            assert_eq!(advice.logical_source_input(), Some(input));
            assert!(advice.original_head().native_input().is_none());
            assert!(advice.obligations().contains(
                &super::SourceCommandTransitionObligation::DeferredLogicalBodyApplicability
            ));
            let body = advice.logical_source_body().unwrap();
            assert!(body.matches_source(
                &tcl_lexer::SourceImage::document(source),
                input.lexer_config()
            ));
            assert_eq!(
                source.get(body.content_span().as_range()),
                Some("return [join [string index abc 99]]")
            );
            assert!(!words.operands_preserve_source_lookup());
        }
        let changed = "proc p {} {return [list [rename string {}] [string index abc 99]]}\n";
        let changed_analysis = analyse_source(changed);
        assert!(words_at(changed, &changed_analysis, "rename string {}").is_some());
        assert!(words_at(changed, &changed_analysis, "string index abc 99").is_none());
    }

    #[test]
    fn logical_deferred_substitutions_withdraw_missing_and_changed_owners() {
        let source = "proc p {} {return [string index abc 99]}\n";
        let baseline = analyse_source(source);
        assert!(words_at(source, &baseline, "string index abc 99").is_some());
        assert!(words_at(&format!("{source} "), &baseline, "string index abc 99").is_none());
        let mut changed = baseline.clone();
        changed.body_lexer_config = Some(tcl_lexer::LexerConfig {
            strict_quoting: !baseline.body_lexer_config.unwrap().strict_quoting,
            ..baseline.body_lexer_config.unwrap()
        });
        assert!(words_at(source, &changed, "string index abc 99").is_none());
        let mut missing = baseline.clone();
        missing.resolved_input = None;
        assert!(words_at(source, &missing, "string index abc 99").is_none());
        let mut foreign = baseline;
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let commands = std::sync::Arc::new(tcl_registry::CommandRegistry::build_default());
        let context = std::sync::Arc::new(
            tcl_registry::model::ingress::context_for_profile(profile).with_command_store(commands),
        );
        foreign.resolved_input = Some(crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        ));
        assert!(words_at(source, &foreign, "string index abc 99").is_none());
    }
    #[test]
    fn logical_unicode_body_children_keep_exact_original_source_geometry() {
        let source = "proc π {} {return [string index café 99]}\n";
        let baseline = analyse_source(source);
        let words = words_at(source, &baseline, "string index café 99").unwrap();
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("missing genuine Unicode deferred body");
        };
        assert_eq!(
            advice.logical_source_input(),
            baseline.resolved_input.as_ref()
        );
        assert!(advice.original_head().native_input().is_none());
        assert_eq!(
            source.get(
                advice
                    .logical_source_body()
                    .unwrap()
                    .content_span()
                    .as_range()
            ),
            Some("return [string index café 99]")
        );
        let cooked = r#"proc p {} "puts\u0020café""#;
        let cooked_analysis = analyse_source(cooked);
        assert!(words_at(cooked, &cooked_analysis, r"puts\u0020café").is_none());
        let mut changed = baseline.clone();
        changed.body_lexer_config = Some(tcl_lexer::LexerConfig {
            strict_quoting: !baseline.body_lexer_config.unwrap().strict_quoting,
            ..baseline.body_lexer_config.unwrap()
        });
        assert!(words_at(source, &changed, "string index café 99").is_none());
        assert!(words_at(&format!("{source} "), &baseline, "string index café 99").is_none());
        let native = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        if let Some(words) = words_at(source, &native, "string index café 99")
            && let OriginalRegistrySource::SourceTransitions(advice) = words.source()
        {
            assert!(advice.logical_source_input().is_none());
            assert!(!advice.obligations().contains(
                &super::SourceCommandTransitionObligation::DeferredLogicalBodyApplicability
            ));
        }
    }

    #[test]
    fn logical_computed_heads_keep_children_and_refuse_parent_dispatch() {
        for source in ["[list puts] VALUE", "proc p {} {[list puts] VALUE}"] {
            let analysis = analyse_source(source);
            let child = words_at(source, &analysis, "list puts").unwrap();
            assert_eq!(child.command(), "list");
            let OriginalRegistrySource::SourceTransitions(advice) = child.source() else {
                panic!("missing original computed-head child point");
            };
            assert_eq!(
                advice.logical_source_input(),
                analysis.resolved_input.as_ref()
            );
            assert_eq!(
                advice.logical_source_body().is_some(),
                source.starts_with("proc")
            );
            assert!(words_at(source, &analysis, "[list puts] VALUE").is_none());
        }
        for source in [
            "[list puts] [rename string {}] [string index café 99]",
            "proc p {} {[list puts] [rename string {}] [string index café 99]}",
        ] {
            let analysis = analyse_source(source);
            assert!(words_at(source, &analysis, "rename string {}").is_some());
            assert!(words_at(source, &analysis, "string index café 99").is_none());
        }
        let source = "proc p {} {[list puts] VALUE}";
        let mut missing = analyse_source(source);
        missing.resolved_input = None;
        assert!(words_at(source, &missing, "list puts").is_none());
    }
    #[test]
    fn uncertain_logical_parent_keeps_possible_body_syntax_without_caller_or_handler_closure() {
        let source = "namespace eval N {proc p {} {uplevel 1 {puts $relative}; uplevel 1 {puts $::Target::x}}}";
        let analysis = analyse_source(source);
        let words = words_at(source, &analysis, "uplevel 1 {puts $::Target::x}").unwrap();
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("original conditional source advice")
        };
        assert!(!advice.uncertain_operations().is_empty());
        assert_eq!(
            advice.logical_source_input(),
            analysis.resolved_input.as_ref()
        );
        let scope = advice.logical_source_scope().unwrap();
        assert!(scope.owns_words(advice.original_words()));
        assert_eq!(
            words.roles(),
            Some([(1, tcl_registry::ArgRole::Body)].as_slice())
        );
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let bodies = words.source_script_bodies(&context);
        assert_eq!(bodies.len(), 1);
        assert_eq!(
            &source[bodies[0].content_span().as_range()],
            "puts $::Target::x"
        );
        assert!(
            words_at(source, &analysis, "puts $::Target::x").is_none(),
            "unknown caller never obtains a child lookup namespace"
        );
        let altered = format!("{source} ");
        assert!(words_at(&altered, &analysis, "uplevel 1 {puts $::Target::x}").is_none());
        for source in [
            "namespace eval N {}; rename ::if ::N::routed; namespace eval N {proc p {} {routed 1 {puts $::Target::x}}}",
            "namespace eval N {}; rename ::if ::N::routed; namespace eval N {namespace export routed}; namespace eval Use {namespace import ::N::routed; proc p {} {routed 1 {puts $::Target::x}}}",
        ] {
            let analysis = analyse_source(source);
            let selected = words_at(source, &analysis, "routed 1 {puts $::Target::x}").unwrap();
            assert_eq!(selected.command(), "if");
            let OriginalRegistrySource::SourceTransitions(advice) = selected.source() else {
                panic!("authentic selected moved/imported descriptor")
            };
            assert!(!advice.lineage().is_empty());
        }
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut registry = tcl_registry::CommandRegistry::build_default();
        registry.insert(tcl_registry::CommandSpec {
            name: "N::uplevel",
            arity: tcl_registry::Arity::exact(2),
            arg_roles: &[(0, tcl_registry::ArgRole::Body)],
            ..tcl_registry::CommandSpec::DEFAULT
        });
        let context = std::sync::Arc::new(
            tcl_registry::model::ingress::context_for_profile(profile)
                .with_command_store(registry.snapshot().shared_registry()),
        );
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context,
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let source = "namespace eval N {proc p {} {unknown; uplevel 1 {puts $::Target::x}}}";
        let analysis = crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, "presentation");
        let words = words_at(source, &analysis, "uplevel 1 {puts $::Target::x}").unwrap();
        assert_eq!(words.command(), "N::uplevel");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert_eq!(
            words.roles(),
            Some([(0, tcl_registry::ArgRole::Body)].as_slice())
        );
        let root_body = words.operands()[1].as_ref().unwrap().word().unwrap();
        assert!(
            words
                .source_script_bodies(&context)
                .iter()
                .all(|body| body.original_container() != root_body),
            "local descriptor never borrows the root uplevel Body operand"
        );
        for source in [
            "namespace eval N {proc p {} {uplevel 1 {}; proc uplevel args {}; uplevel 1 {puts $::Target::x}}}",
            "namespace eval N {proc p {} {uplevel 1 {}; unregistered 1 {puts $::Target::x}}}",
            "namespace eval N {proc p {} {uplevel 1 {}; $head 1 {puts $::Target::x}}}",
        ] {
            let analysis = analyse_source(source);
            let fragment = if source.contains("unregistered 1") {
                "unregistered 1 {puts $::Target::x}"
            } else if source.contains("$head 1") {
                "$head 1 {puts $::Target::x}"
            } else {
                "uplevel 1 {puts $::Target::x}"
            };
            assert!(words_at(source, &analysis, fragment).is_none(), "{source}");
        }
    }

    #[test]
    fn logical_parameter_source_prefix_cache_warmth_is_not_body_identity() {
        let source = "proc reader {name} {upvar 1 $name local; set local 1}";
        let analysis = analyse_source(source);
        let input = analysis.resolved_input.as_ref().unwrap();
        let realm = analysis.retained_command_realm().unwrap();
        let cards = realm.original_logical_procedure_body_cards(input).unwrap();
        let [body] = cards.as_slice() else {
            panic!("one actual body");
        };
        let offset = u32::try_from(source.find("upvar").unwrap()).unwrap();
        let words = words_at(
            source,
            &analysis,
            &source[offset as usize..source.len() - 1],
        )
        .unwrap();
        let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
            panic!("source advice");
        };
        assert!(
            body.parameter_source_prefix(advice.original_words())
                .is_some()
        );
        let mut cold = body.as_ref().clone();
        cold.parameter_source_prefixes = std::sync::Arc::default();
        assert_eq!(body.as_ref(), &cold);
        assert!(
            cold.parameter_source_prefix(advice.original_words())
                .is_none()
        );
        assert!(
            body.parameter_source_prefix(advice.original_words())
                .is_some()
        );
    }
}
