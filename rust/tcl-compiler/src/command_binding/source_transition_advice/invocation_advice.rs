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

//! Advice for one invocation: schema transitions, retained inventories and original operands.

use super::transition_capture::{
    capture_transition, original_words, registry_words, trace_jim_alias_guard,
    trace_jim_alias_option,
};
use super::{
    AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceAdviceNameInput, SourceAdviceQuery, SourceAdviceWord, SourceCommandTransitionObligation,
    logical_navigation, source_class, source_procedure,
};
use std::sync::Arc;
use tcl_lexer::NativeWord;
use tcl_registry::StateTransition;

impl AdviceInvocationContext<'_> {
    pub(super) fn apply_schema_transition(
        &self,
        graph: &mut AdviceGraph,
        invocation: AdviceInvocation<'_>,
        command: &str,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        logical_scope: Option<
            &Arc<super::super::source_declared_command::OriginalDeclaredLogicalBodyContext>,
        >,
    ) -> Option<OriginalSourceTransitionAdviceTape> {
        let AdviceInvocation {
            native,
            arguments,
            lineage,
            ..
        } = invocation;
        let mut inventory = OriginalSourceTransitionAdviceTape::default();
        let transitions = schema.state_transitions();
        let ensemble_transition = transitions.facts().iter().any(|fact| {
            matches!(
                &fact.transition,
                StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensemble { .. })
            )
        });
        if ensemble_transition {
            inventory
                .logical_ensemble_transition_receipts
                .insert(native.first()?.span().start(), None);
        }
        let receipt = trace_jim_alias_option(
            command,
            "jim.alias.receipt",
            capture_transition(
                self.origin,
                native,
                command,
                arguments,
                transitions.clone(),
                lineage,
            ),
        );
        if let Some(receipt) = &receipt {
            if ensemble_transition {
                inventory.logical_ensemble_transition_receipts.insert(
                    receipt.original_words().first()?.span().start(),
                    Some(Arc::clone(receipt)),
                );
            }
            let applied =
                trace_jim_alias_guard(command, "jim.alias.applied", graph.apply(receipt).is_some());
            if !applied {
                graph.widen(native);
            }
        }
        let declared = schema.semantics.state_transitions.is_declared();
        let immediate_body = (self
            .authored_roles(schema)
            .0
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
            || self.normal_case_source_layout(schema).is_some())
            && !schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY)
            && schema.authored_source_procedure_arguments().is_none();
        if (receipt.is_none() && !transitions.facts().is_empty())
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandBindings)
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandResolution)
        {
            graph.widen(native);
        } else if immediate_body {
            let advice = self.schema_advice(invocation, graph, schema)?;
            let projected =
                self.apply_immediate_body_effects(graph, &advice, schema, logical_scope);
            if !projected.effects_complete {
                graph.widen(native);
            }
            inventory.extend_inventory(projected.inventory);
        } else if !declared
            && !self.normal_source_variable_effects_closed(invocation, schema)
            && !Self::apply_jim_stock_source_effects(graph, invocation, schema)
        {
            graph.widen(native);
        }
        if schema.semantics.procedure_definition.is_none()
            && schema.authored_source_procedure_arguments().is_some()
        {
            Self::apply_source_lookup_residual_effects(graph, invocation, schema);
        }
        Some(inventory)
    }

    pub(super) fn retain_source_inventory(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        procedure: Option<&source_procedure::OriginalSourceProcedureBody>,
        callback_scope: logical_navigation::LogicalCallbackScope<'_>,
    ) -> Option<()> {
        self.retain_requested_class_relation(invocation, graph, schema);
        self.retain_schema_advice(tape, invocation, graph, schema, procedure.is_some())?;
        self.retain_native_expression_syntax_inventory(tape, graph, invocation, schema);
        self.retain_original_expression_catalogue(tape, graph, invocation, schema);
        self.retain_c_source_lifecycle_inventory(tape, graph, invocation, schema, None, 0);
        self.retain_authored_command_prefix(tape, invocation, graph, schema, None);
        self.retain_definition_member_prefix_inventory(tape, graph, invocation, schema);
        if procedure.is_none() {
            self.retain_original_body_catalogue(tape, graph, invocation, schema);
        }
        self.retain_source_callback_targets(tape, invocation, graph, schema);
        self.retain_logical_callback_candidates(tape, invocation, graph, schema, callback_scope);
        self.retain_logical_name_references(tape, invocation, graph, schema);
        self.retain_produced_command_prefixes(tape, invocation, graph, schema, None);
        self.retain_logical_procedure_body(tape, invocation, graph, schema);
        Some(())
    }

    fn retain_schema_advice(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        procedure_present: bool,
    ) -> Option<()> {
        let retained = graph.native_baseline
            || !invocation.lineage.is_empty()
            || graph.procedure_body.is_some()
            || matches!(self.policy, AdviceNamingPolicy::Logical(_));
        if schema.semantics.procedure_definition.is_some()
            && crate::registry_invocation::source_cut_trace_admitted()
        {
            let image = self.origin.source_image();
            eprintln!(
                "SOURCE_CUT_VALUE_ISSUER origin={:p} data={:p} bytes={} offset={:?} procedure={} native={} lineage={} body={} logical={} retained={retained}",
                Arc::as_ptr(self.origin),
                image.bytes().as_ptr(),
                image.len(),
                invocation.native.first().map(|word| word.span().start()),
                procedure_present,
                graph.native_baseline,
                !invocation.lineage.is_empty(),
                graph.procedure_body.is_some(),
                matches!(self.policy, AdviceNamingPolicy::Logical(_)),
            );
        }
        if retained {
            let advice = self.schema_advice(invocation, graph, schema)?;
            tape.advice.insert(advice.site.offset, advice);
        }
        Some(())
    }

    pub(super) fn authored_roles(
        &self,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> (Vec<(u8, tcl_registry::ArgRole)>, bool) {
        match self.policy {
            AdviceNamingPolicy::Logical(_) => schema.authored_logical_source_argument_roles(),
            AdviceNamingPolicy::Native(_) => schema.authored_source_argument_roles(),
        }
    }

    pub(super) fn retain_arity_debt(
        &self,
        graph: &mut AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<()> {
        if schema.facts().arity_accepts_frozen_arguments() == Some(false) {
            let mut debt = self.schema_advice(invocation, graph, schema)?;
            if !debt
                .obligations
                .contains(&SourceCommandTransitionObligation::UnacceptedSourceArity)
            {
                debt.obligations
                    .push(SourceCommandTransitionObligation::UnacceptedSourceArity);
            }
            if !graph.arity_debts.iter().any(|old| old.as_ref() == &debt) {
                graph.arity_debts.push(Arc::new(debt));
            }
        }
        Some(())
    }

    pub(super) fn schema_advice(
        &self,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<OriginalSourceCommandTransitionAdvice> {
        let AdviceInvocation {
            native,
            head,
            arguments,
            lineage,
        } = invocation;
        let facts = schema.facts();
        let (roles, complete) = self.authored_roles(schema);
        let roles = complete
            .then(|| {
                roles
                    .into_iter()
                    .map(|(ordinal, role)| {
                        facts
                            .argument_offset
                            .checked_add(usize::from(ordinal))
                            .map(|ordinal| (ordinal, role))
                    })
                    .collect::<Option<Vec<_>>>()
            })
            .flatten();
        let mut obligations = vec![SourceCommandTransitionObligation::UnavailableActualLookup];
        if graph.native_baseline {
            obligations.push(SourceCommandTransitionObligation::NativeBaselineSourceApplicability);
        }
        if !lineage.is_empty() {
            obligations.push(SourceCommandTransitionObligation::WrittenTransitionApplicability);
        }
        if graph.procedure_body.is_some() {
            obligations.push(SourceCommandTransitionObligation::OriginalProcedureBodyApplicability);
        }
        if graph.future_procedure_source {
            obligations.push(
                SourceCommandTransitionObligation::FutureOriginalProcedureSourceApplicability,
            );
        }
        if matches!(self.policy, AdviceNamingPolicy::Logical(_)) {
            obligations.push(SourceCommandTransitionObligation::ExplicitLogicalSourceApplicability);
        }
        if graph.c_source_namespace.is_some() && !graph.native_baseline {
            obligations.push(SourceCommandTransitionObligation::ConditionalCNamespaceApplicability);
        }
        let c_source_namespace = (!graph.native_baseline)
            .then(|| graph.c_source_lookup_namespace().cloned())
            .flatten();
        let c_source_path = c_source_namespace.as_ref().and_then(|namespace| {
            (!graph.unknown_paths.contains(namespace)).then(|| {
                graph
                    .namespace_paths
                    .get(namespace)
                    .cloned()
                    .unwrap_or_default()
            })
        });
        if c_source_path.as_ref().is_some_and(|path| {
            path.iter()
                .any(|namespace| !graph.namespaces.contains(namespace))
        }) {
            obligations.push(
                SourceCommandTransitionObligation::UnretainedCSourcePathProviderApplicability,
            );
        }
        if !graph.uncertain_operations.is_empty() {
            obligations.push(SourceCommandTransitionObligation::UnknownEarlierMutation);
        }
        if !graph.arity_debts.is_empty() {
            obligations.push(SourceCommandTransitionObligation::UnacceptedSourceArity);
        }
        Some(OriginalSourceCommandTransitionAdvice {
            site: tcl_names::command::CommandAllocationSite {
                source: Arc::clone(self.origin),
                offset: native.first()?.span().start(),
            },
            original: Arc::from(native),
            head: head.clone(),
            arguments: arguments.to_vec(),
            command: facts.canonical_command.clone(),
            context: self.context.context().clone(),
            registry: self.context.commands().snapshot().semantic_key(),
            config: self.config,
            dialect: self.dialect,
            roles,
            lineage: lineage.to_vec(),
            obligations,
            uncertain_operations: graph.uncertain_operations.clone(),
            declaration: None,
            logical_body: None,
            logical_scope: None,
            c_source_body: None,
            c_source_namespace,
            c_source_path,
            procedure_body: graph.procedure_body.clone(),
            member_catalogue_body: None,
            catalogue_body: None,
            logical_input: match self.policy {
                AdviceNamingPolicy::Logical(input) => Some(input.clone()),
                AdviceNamingPolicy::Native(_) => None,
            },
        })
    }
}

impl AdviceInvocationContext<'_> {
    fn retain_source_header_relations(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        native: &[NativeWord],
        head: &SourceAdviceNameInput,
    ) -> bool {
        if let Some(declared) = graph.declared_source_selection(self.origin, head, native) {
            tape.retain_declared(declared);
        }
        if let Some(body) = self.interpreter_handle_body(graph, head, native) {
            tape.interpreter_bodies
                .insert(body.invocation_offset(), body);
            // The child can call its parent; no later parent map follows.
            graph.widen(native);
            return true;
        }
        false
    }

    pub(super) fn observe_publication_proposal(&self, graph: &AdviceGraph) {
        if let Some(SourceAdviceQuery::Publication(query)) = self.query {
            query.observe(graph);
        }
    }

    pub(super) fn withdraw_missing_publication_phase(
        &self,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let Some(SourceAdviceQuery::Publication(query)) = self.query else {
            return;
        };
        let (roles, complete) = self.authored_roles(schema);
        let relevant = !complete
            || roles
                .iter()
                .any(|(_, role)| *role == tcl_registry::ArgRole::Expr)
            || schema
                .authored_source_script_arguments()
                .is_none_or(|arguments| !arguments.is_empty());
        if relevant {
            query.withdraw();
        }
    }

    fn partial_expression_geometry(
        &self,
        tail: tcl_lexer::NativeScriptWordCut,
    ) -> Option<(
        tcl_lexer::NativeScriptWordCut,
        tcl_lexer::SourceSegmentationPurpose,
    )> {
        let image = self.origin.source_image();
        if tail.cut.kind != tcl_lexer::ParseIssueKind::MissingBrace
            || image.bytes().get(tail.cut.term as usize) != Some(&b'{')
        {
            return None;
        }
        if tail.cut.offset as usize == image.bytes().len() {
            Some((tail, tcl_lexer::SourceSegmentationPurpose::WholeInput))
        } else {
            let bracket = tail.cut.offset;
            if image.bytes().get(bracket as usize) != Some(&b'[') {
                return None;
            }
            let outer = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(tail.command_start, bracket),
                self.config,
            )
            .ok()?;
            let [parent] = outer.commands.as_slice() else {
                return None;
            };
            let last = parent.words.last()?;
            if outer.fatal_tail.is_some()
                || !image
                    .bytes()
                    .get(last.span().end() as usize..bracket as usize)?
                    .iter()
                    .all(u8::is_ascii_whitespace)
                || last.span().end() == bracket
                || parent.words.iter().any(|word| {
                    word.group().expand
                        || tcl_syntax::word_rules::original_static_word_unicode_value(word)
                            .is_none()
                })
            {
                return None;
            }
            let child = tcl_lexer::native_script_words_in_with_purpose(
                image.clone(),
                tcl_lexer::Span::new(bracket + 1, image.bytes().len().try_into().ok()?),
                self.config,
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
            )
            .ok()?;
            if !child.commands.is_empty() {
                return None;
            }
            let child_tail = child.fatal_tail?;
            if child_tail.cut.kind != tcl_lexer::ParseIssueKind::MissingBrace
                || child_tail.cut.offset as usize != image.bytes().len()
                || child_tail.cut.term != tail.cut.term
            {
                return None;
            }
            Some((
                child_tail,
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
            ))
        }
    }

    pub(super) fn partial_expression_span(
        &self,
        graph: &AdviceGraph,
        tail: tcl_lexer::NativeScriptWordCut,
    ) -> Option<tcl_lexer::Span> {
        if !graph.uncertain_operations.is_empty() || !graph.arity_debts.is_empty() {
            return None;
        }
        let image = self.origin.source_image();
        let (tail, purpose) = self.partial_expression_geometry(tail)?;
        let prefix = tcl_lexer::native_script_words_in_with_purpose(
            image.clone(),
            tcl_lexer::Span::new(tail.command_start, tail.cut.term),
            self.config,
            purpose,
        )
        .ok()?;
        let [command] = prefix.commands.as_slice() else {
            return None;
        };
        let [word] = command.words.as_slice() else {
            return None;
        };
        if prefix.fatal_tail.is_some()
            || !image
                .bytes()
                .get(word.span().end() as usize..tail.cut.term as usize)?
                .iter()
                .all(u8::is_ascii_whitespace)
        {
            return None;
        }
        let written = original_words(&command.words, self.policy)?;
        let head = written.first()?.input.as_ref()?;
        if graph.blocks_registry_source(head) {
            return None;
        }
        let (name, arguments, lineage) = graph.resolve(head)?;
        if !arguments.is_empty() || !lineage.is_empty() {
            return None;
        }
        let args = [tcl_registry::InvocationWord::Dynamic];
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.commands(),
                Some(self.context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&name),
                    &args,
                )
                .with_dialect(self.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved()?;
        if schema.facts().argument_offset != 0 {
            return None;
        }
        let (roles, complete) = self.authored_roles(&schema);
        (complete
            && roles.iter().filter(|(index, _)| *index == 0).count() == 1
            && roles.contains(&(0, tcl_registry::ArgRole::Expr)))
        .then(|| tcl_lexer::Span::new(tail.cut.term + 1, tail.cut.offset))
    }

    pub(super) fn retain_original_invocation(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
        callback_scope: logical_navigation::LogicalCallbackScope<'_>,
    ) -> Option<()> {
        self.begin_normal_source_invocation();
        self.observe_publication_proposal(graph);
        tape.retain_arity_prefix(graph, &invocation.words);
        let retained =
            self.retain_original_invocation_inner(graph, tape, invocation, callback_scope);
        self.observe_publication_proposal(graph);
        retained
    }

    fn retain_original_invocation_prefix(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
        written: &[SourceAdviceWord],
    ) -> bool {
        self.retain_native_procedure_may_command(tape, graph, &invocation.words, written);
        self.retain_original_operand_constructor_calls(tape, graph, &invocation.words, None);
        self.retain_original_source_receiver(tape, graph, &invocation.words, written)
    }

    fn retain_unresolved_invocation_operands(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
    ) -> Option<()> {
        if matches!(self.policy, AdviceNamingPolicy::Logical(_)) {
            // The original children execute before an unknown parent head;
            // retaining them does not select that head from result data.
            let _ = self.inspect_operand_effects(graph, &invocation.words, tape);
            tape.registry_barriers
                .insert(invocation.words.first()?.span().start());
        }
        graph.widen(&invocation.words);
        Some(())
    }

    fn append_original_invocation_operands(
        arguments: &mut Vec<SourceAdviceWord>,
        written: Vec<SourceAdviceWord>,
    ) {
        for (ordinal, argument) in arguments.iter_mut().enumerate() {
            argument.origin =
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.into_iter().skip(1));
    }

    fn retain_invocation_barrier(
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
        original_barrier: bool,
    ) -> Option<()> {
        if original_barrier {
            tape.registry_barriers
                .insert(invocation.words.first()?.span().start());
        }
        Some(())
    }

    fn retain_original_invocation_inner(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: &tcl_lexer::NativeScriptCommandWords,
        callback_scope: logical_navigation::LogicalCallbackScope<'_>,
    ) -> Option<()> {
        tape.represented
            .insert(invocation.words.first()?.span().start());
        let Some(written) = original_words(&invocation.words, self.policy) else {
            graph.widen(&invocation.words);
            return Some(());
        };
        if self.retain_original_invocation_prefix(graph, tape, invocation, &written) {
            return Some(());
        }
        let handle = self.before_operand_handle_binding(graph, &invocation.words, &written);
        let class_handle =
            self.before_operand_class_handle(tape, graph, &invocation.words, &written);
        let Some(head) = written
            .first()
            .and_then(|head| head.input.as_ref())
            .cloned()
        else {
            self.retain_unresolved_invocation_operands(graph, tape, invocation)?;
            return Some(());
        };
        let original_selection = (graph.native_baseline || class_handle.is_some())
            .then(|| graph.resolve(&head))
            .flatten();
        let original_barrier = graph.blocks_registry_source(&head);
        let Some(operand_barrier) = self.inspect_operand_effects(graph, &invocation.words, tape)
        else {
            graph.widen(&invocation.words);
            return Some(());
        };
        Self::retain_invocation_barrier(tape, invocation, original_barrier)?;
        OriginalSourceTransitionAdviceTape::retain_logical_procedure_call(
            tape,
            graph,
            &invocation.words,
        );
        if self.retain_source_header_relations(graph, tape, &invocation.words, &head) {
            return Some(());
        }
        let selected = original_selection.or_else(|| graph.resolve(&head));
        let Some((command, mut arguments, lineage)) = selected else {
            graph.widen(&invocation.words);
            return Some(());
        };
        // Prefix ordinals belong to the composed effective prefix. Each
        // retained word/input still owns its original transition producer.
        Self::append_original_invocation_operands(&mut arguments, written);
        let words = registry_words(&arguments);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.commands(),
                Some(self.context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&command),
                    &words,
                )
                .with_dialect(self.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let Some(schema) = resolution.resolved() else {
            graph.widen(&invocation.words);
            return Some(());
        };
        let selected = AdviceInvocation {
            native: &invocation.words,
            head: &head,
            arguments: &arguments,
            lineage: &lineage,
        };
        let mut procedure = None;
        self.retain_original_invocation_inventory(
            graph,
            tape,
            selected,
            &schema,
            callback_scope,
            &mut procedure,
        )?;
        self.retain_interpreter_paths(tape, selected, graph, &schema);
        let factory = self.registered_factory(graph, selected, &schema);
        let authored_jim = self.authored_jim_class_source(graph, selected, &schema, None);
        let class = self.retain_source_class_declaration(tape, graph, selected, &schema);
        self.retain_source_configured_class(tape, graph, selected, &schema);
        self.retain_original_class_operand_references(
            tape,
            graph,
            class.as_ref(),
            procedure.as_ref(),
        );
        let body_inventory =
            self.apply_schema_transition(graph, selected, &command, &schema, None)?;
        tape.extend_inventory(body_inventory);
        if operand_barrier {
            // The current receipt is only a pre-operand source possibility.
            // Unknown operand callbacks cannot preserve the later graph.
            graph.widen(&invocation.words);
        }
        self.install_registered_results(tape, graph, factory, handle);
        Self::install_source_class_handle(graph, class_handle);
        self.retain_source_class_adoptions(tape, graph, class.as_ref());
        Self::install_source_class(tape, graph, class);
        self.retain_authored_jim_procedure_adoptions(tape, graph, authored_jim.as_ref());
        Self::install_authored_jim_class(tape, graph, authored_jim);
        source_procedure::install_original_procedure(graph, procedure);
        Some(())
    }
    fn retain_original_invocation_inventory(
        &self,
        graph: &mut AdviceGraph,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        callback_scope: logical_navigation::LogicalCallbackScope<'_>,
        procedure: &mut Option<Arc<source_procedure::OriginalSourceProcedureBody>>,
    ) -> Option<()> {
        self.retain_arity_debt(graph, invocation, schema)?;
        self.observe_source_point(graph, invocation.native, schema);
        self.observe_normal_source_operation(graph, schema);
        *procedure = self.source_procedure_declaration(graph, invocation, schema);
        if let Some(shape) = schema.authored_source_procedure_arguments() {
            source_class::merge_receipt(
                &mut tape.conditional_procedure_declarations,
                invocation.native.first()?.span().start(),
                procedure.clone().map(|procedure| (procedure, shape)),
            );
        }
        self.retain_source_inventory(
            tape,
            invocation,
            graph,
            schema,
            procedure.as_deref(),
            callback_scope,
        )?;
        Some(())
    }
}
