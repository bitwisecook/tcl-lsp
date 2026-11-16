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

//! The commands of a walked body: child schemas, class effects, catalogues and procedure calls.

use super::super::{
    AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceCommandTransitionObligation, original_words, registry_words,
};
use super::{
    BodyWalk, MAX_COMMANDS, MAX_DEPTH, OriginalOperandEffect, body_class_obligation,
    boxed_body_advice, boxed_body_resolution, boxed_region_words, note_source_point_body_walk,
    original_operand_effects,
};
#[cfg(test)]
use crate::command_binding::source_transition_advice::body_effects::trace_body_inventory;
use std::sync::Arc;
use tcl_lexer::{NativeScriptCommandWords, NativeWord, Span};

impl BodyWalk<'_, '_> {
    pub(super) fn script(
        &mut self,
        graph: &mut AdviceGraph,
        span: Span,
        depth: usize,
    ) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        let plan = self
            .context
            .origin
            .original_script_words_in(
                span,
                self.context.config,
                tcl_lexer::SourceSegmentationPurpose::NestedScript,
            )
            .ok();
        #[cfg(test)]
        trace_body_inventory(
            "script.plan",
            Some(span.start()),
            plan.is_some(),
            plan.as_ref().map_or(0, |plan| plan.commands.len()),
        );
        let plan = plan?;
        #[cfg(test)]
        trace_body_inventory(
            "script.complete",
            Some(span.start()),
            plan.fatal_tail.is_none(),
            plan.commands.len(),
        );
        if plan.fatal_tail.is_some() && self.catalogue_body.is_none() {
            return None;
        }
        for command in plan.commands {
            self.visited = self.visited.checked_add(1)?;
            if self.visited > MAX_COMMANDS {
                return None;
            }
            self.command(graph, &command, depth)?;
        }
        Some(())
    }

    pub(super) fn substitutions(
        &mut self,
        graph: &mut AdviceGraph,
        native: &[NativeWord],
        depth: usize,
    ) -> Option<()> {
        for effect in original_operand_effects(native) {
            match effect {
                OriginalOperandEffect::Command(body) => {
                    self.script(graph, body, depth.checked_add(1)?)?;
                }
                OriginalOperandEffect::Unknown => graph.record_uncertainty(native),
            }
        }
        Some(())
    }

    pub(super) fn command(
        &mut self,
        graph: &mut AdviceGraph,
        native: &NativeScriptCommandWords,
        depth: usize,
    ) -> Option<()> {
        self.context.observe_publication_proposal(graph);
        self.inventory
            .as_mut()
            .unwrap_or(&mut self.authored_prefixes)
            .retain_arity_prefix(graph, &native.words);
        let retained = self.command_inner(graph, native, depth);
        self.context.observe_publication_proposal(graph);
        retained
    }

    fn retain_native_procedure_call(
        &mut self,
        graph: &AdviceGraph,
        native: &NativeScriptCommandWords,
        written: &[super::super::SourceAdviceWord],
    ) {
        if matches!(self.context.policy, AdviceNamingPolicy::Native(_)) {
            drop(
                AdviceInvocationContext::retain_original_procedure_call_receipt(
                    self.inventory
                        .as_mut()
                        .unwrap_or(&mut self.authored_prefixes),
                    graph,
                    &native.words,
                    written,
                ),
            );
        }
    }

    fn command_inner(
        &mut self,
        graph: &mut AdviceGraph,
        native: &NativeScriptCommandWords,
        depth: usize,
    ) -> Option<()> {
        self.mark_child_header(native)?;
        self.retain_original_body_word_syntax(graph, native);
        if self.retain_logical_body_constructor_calls(graph, native) {
            return Some(());
        }
        let class_handle = self.original_body_class_handle(graph, &native.words);
        let original_selection = class_handle.as_ref().and_then(|_| {
            let written = original_words(&native.words, self.context.policy)?;
            graph.resolve(written.first()?.input.as_ref()?)
        });
        self.substitutions(graph, &native.words, depth)?;
        let Some(written) = original_words(&native.words, self.context.policy) else {
            graph.record_uncertainty(&native.words);
            return Some(());
        };
        self.retain_native_procedure_call(graph, native, &written);
        // A walk retaining Logical source-word recipes, as
        // `retain_logical_navigation` above does, records every call capture.
        // Other walks record one only where they hold a recipe at its head.
        let owns_logical_recipe = self.inventory.is_some() && self.logical_scope.is_some();
        // Syntax ownership survives unavailable handler/effect inventory.
        let tape = self
            .inventory
            .as_mut()
            .unwrap_or(&mut self.authored_prefixes);
        self.context
            .retain_native_procedure_may_command(tape, graph, &native.words, &written);
        let Some(head) = written
            .first()
            .and_then(|word| word.input.as_ref())
            .cloned()
        else {
            graph.record_uncertainty(&native.words);
            return Some(());
        };
        if owns_logical_recipe {
            OriginalSourceTransitionAdviceTape::retain_logical_procedure_call(
                tape,
                graph,
                &native.words,
            );
        } else {
            tape.retain_recipe_logical_procedure_call(graph, &native.words);
        }
        let Some((command, mut arguments, lineage)) =
            original_selection.or_else(|| graph.resolve(&head))
        else {
            graph.record_uncertainty(&native.words);
            return Some(());
        };
        for (ordinal, argument) in arguments.iter_mut().enumerate() {
            argument.origin =
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.into_iter().skip(1));
        let words = registry_words(&arguments);
        let resolution =
            boxed_body_resolution(self.context.context, self.context.dialect, &command, &words);
        let Some(schema) = resolution.resolved() else {
            graph.record_uncertainty(&native.words);
            return Some(());
        };
        let invocation = AdviceInvocation {
            native: &native.words,
            head: &head,
            arguments: &arguments,
            lineage: &lineage,
        };
        self.context.retain_arity_debt(graph, invocation, &schema)?;
        note_source_point_body_walk(
            self.context,
            invocation.native,
            self.catalogue_body.is_some() || self.member_catalogue_body.is_some(),
        );
        // Catalogue walks record syntax under conditional applicability; they
        // cannot issue an execution-point graph or veto an entered-body walk.
        if self.catalogue_body.is_none() && self.member_catalogue_body.is_none() {
            self.context
                .observe_source_point(graph, invocation.native, &schema);
        }
        self.retain_requested_body_class_relation(graph, invocation, &schema);
        self.retain_child_schema(graph, invocation, &schema)?;
        let retained_lifecycle = if let Some(tape) = &mut self.inventory {
            self.context.retain_c_source_lifecycle_inventory(
                tape,
                graph,
                invocation,
                &schema,
                self.source_body.as_ref(),
                depth.saturating_add(1),
            )
        } else {
            false
        };
        self.apply_original_body_class_effects(graph, &command, invocation, &schema, class_handle);
        if retained_lifecycle {
            // The independent SOURCE walk above owns these conditional phases.
            // Mutation coverage cannot duplicate it without its applicability,
            // and it cannot prove the provider, phase entry or phase effects.
            graph.record_uncertainty(&native.words);
            self.unavailable_lifecycle_effects = true;
            return Some(());
        }
        self.finish_original_body_command(graph, native, invocation, &schema, depth)
    }

    fn retain_logical_body_constructor_calls(
        &mut self,
        graph: &mut AdviceGraph,
        native: &NativeScriptCommandWords,
    ) -> bool {
        let (Some(tape), Some(body)) = (&mut self.inventory, &self.source_body) else {
            return false;
        };
        matches!(self.context.policy, AdviceNamingPolicy::Logical(_))
            && self.context.retain_original_body_constructor_calls(
                tape,
                graph,
                &native.words,
                (
                    body,
                    SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
                ),
            )
    }

    fn retain_original_body_word_syntax(
        &mut self,
        graph: &AdviceGraph,
        native: &NativeScriptCommandWords,
    ) {
        let syntax_body = self
            .member_catalogue_body
            .as_ref()
            .map(|body| super::super::OriginalSourceCatalogueBody::Member(Arc::clone(body)))
            .or_else(|| self.catalogue_body.clone())
            .or_else(|| {
                self.source_body
                    .as_ref()
                    .map(|body| super::super::OriginalSourceCatalogueBody::Script(Arc::clone(body)))
            });
        if let Some(body) = syntax_body {
            let tape = self
                .inventory
                .as_mut()
                .unwrap_or(&mut self.authored_prefixes);
            tape.retain_logical_body_words(self.context, &native.words, body.clone());
            tape.retain_logical_absolute_navigation(graph, &native.words, body);
        }
        if let (Some(tape), Some(scope)) = (&mut self.inventory, &self.logical_scope) {
            tape.retain_logical_summary_scope(&native.words, scope);
            tape.retain_logical_navigation(graph, &native.words, scope);
        }
    }

    fn finish_original_body_command(
        &mut self,
        graph: &mut AdviceGraph,
        native: &NativeScriptCommandWords,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
    ) -> Option<()> {
        let immediate = (self
            .context
            .authored_roles(schema)
            .0
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
            || self.context.normal_case_source_layout(schema).is_some())
            && !schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY)
            && schema.authored_source_procedure_arguments().is_none();
        let advice = boxed_body_advice(self.context, invocation, graph, schema)?;
        if self.expressions(graph, &advice, schema, depth).is_none() {
            graph.widen(&native.words);
        }
        if !immediate {
            self.member_catalogue_possible_bodies(graph, &advice, schema, depth.checked_add(1)?);
        } else if self
            .regions(graph, &advice, schema, depth.checked_add(1)?)
            .is_none()
        {
            // A `TclOO` definition script runs in a new frame in its definer's
            // namespace (`::oo::define`, `::oo::objdefine` or a configurable
            // support namespace), and the definition-member inventory owns its
            // members. Its remaining reach into this walk — `uplevel`/`upvar`
            // to the caller frame and absolute names — is that of an
            // unresolved command, so it records the same uncertainty instead
            // of withdrawing every namespace's path and export advice.
            if schema
                .authored_source_definition_body_grammar()
                .is_some_and(|grammar| {
                    grammar.family == tcl_registry::definer::DefinerFamily::TclOo
                })
            {
                graph.record_uncertainty(&native.words);
            } else {
                graph.widen(&native.words);
            }
        }
        Some(())
    }

    /// A readonly member-body catalogue also follows possible same-invocation
    /// bodies whose complete role layout is unavailable (a `switch` with a
    /// computed subject or option). The parent operation keeps its recorded
    /// uncertainty; every candidate runs on its own branch, so no candidate
    /// effect reaches the parent walk. Deferred, frame-shifting, other-
    /// interpreter and declaration bodies are never followed.
    fn member_catalogue_possible_bodies(
        &mut self,
        graph: &AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
    ) {
        if self.member_catalogue_body.is_none()
            || self.inventory.is_some()
            || self.source_body.is_some()
            || depth > MAX_DEPTH
            || schema.semantics.body_kind != tcl_registry::BodyKind::Plain
            || schema.semantics.frame_effect.is_some()
            || schema.semantics.traits.intersects(
                tcl_registry::Traits::DEFERS_BODY | tcl_registry::Traits::DEFINES_PROCEDURE,
            )
            || schema.authored_source_procedure_arguments().is_some()
            || schema.authored_source_definition_body_grammar().is_some()
        {
            return;
        }
        let Some(words) = boxed_region_words(self.context, advice) else {
            return;
        };
        let interpreter = schema.semantics.body_interpreter.resolve_with(|ordinal| {
            std::str::from_utf8(words.arguments().get(ordinal)?.literal_bytes()?).ok()
        });
        if interpreter != tcl_registry::world_effect::InterpreterScope::Current {
            return;
        }
        let context = self.context.context;
        let mut bodies = words
            .conditional_source_case_bodies(context)
            .filter(|case| {
                case.matches_source(self.context.origin.source_image(), self.context.config)
            })
            .map_or_default(|case| case.bodies().to_vec());
        bodies.extend(words.source_script_bodies_for(
            context,
            crate::registry_invocation::OriginalSourceScriptPurpose::PotentialEvaluation,
        ));
        bodies.sort_by_key(|body| (body.content_span().start(), body.content_span().end()));
        bodies.dedup_by_key(|body| body.content_span());
        for body in bodies {
            // Each candidate's own written operand must run before the
            // invocation returns; a stored callback body never qualifies.
            let same_invocation = words
                .operands()
                .iter()
                .position(|operand| {
                    operand.as_ref().and_then(
                        crate::registry_invocation::source_structure::OriginalOperandSource::word,
                    ) == Some(body.original_container())
                })
                .and_then(|argument| {
                    words
                        .with_source_schema(context, |schema| {
                            schema.authored_source_script_timing_at(argument)
                        })
                        .flatten()
                })
                == Some(tcl_registry::ScriptTiming::SameInvocation);
            if !same_invocation
                || !body.matches_source(self.context.origin.source_image(), self.context.config)
                || !body.matches_context(context)
            {
                continue;
            }
            let mut branch = graph.clone();
            branch.record_uncertainty(advice.original_words());
            // A malformed or over-deep candidate only withholds its own rows.
            let _ = self.script(&mut branch, body.content_span(), depth);
        }
    }

    fn original_body_class_handle(
        &self,
        graph: &AdviceGraph,
        native: &[NativeWord],
    ) -> Option<super::super::source_instance::PendingSourceClassHandle> {
        if !matches!(self.context.policy, AdviceNamingPolicy::Logical(_)) {
            return None;
        }
        let tape = self.inventory.as_ref()?;
        let written = original_words(native, self.context.policy)?;
        let update = self
            .context
            .before_operand_class_handle(tape, graph, native, &written);
        self.context.retain_source_class_handle_body(
            update,
            self.source_body.as_ref()?,
            SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
        )
    }

    fn apply_original_body_class_effects(
        &mut self,
        graph: &mut AdviceGraph,
        command: &str,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        class_handle: Option<super::super::source_instance::PendingSourceClassHandle>,
    ) {
        // The independent readonly catalogue owns Jim source syntax even when
        // this walk has no execution/mutation-completeness inventory.
        let jim_parent = self.source_body.as_ref().or(match &self.catalogue_body {
            Some(super::super::OriginalSourceCatalogueBody::Script(body)) => {
                // A syntax-only Logical catalogue cannot borrow the enclosing
                // graph's namespace. Its own selected body scope must agree.
                let owned = match self.context.policy {
                    AdviceNamingPolicy::Logical(input) => {
                        self.logical_scope.as_ref().is_some_and(|scope| {
                            scope.matches_input(input)
                                && scope.body_arc() == body
                                && graph.logical_namespace.as_ref()
                                    == Some(scope.namespace_recipe())
                        })
                    }
                    AdviceNamingPolicy::Native(_) => true,
                };
                owned.then_some(body)
            }
            _ => None,
        });
        let authored_jim = jim_parent.and_then(|body| {
            self.context
                .authored_jim_class_source(graph, invocation, schema, Some(body))
        });
        let class = self
            .inventory
            .as_mut()
            .zip(self.source_body.as_ref())
            .and_then(|(tape, body)| {
                let obligation = body_class_obligation(self.context, graph)?;
                self.context.source_class_declaration_in_body(
                    tape,
                    graph,
                    invocation,
                    schema,
                    (body, obligation, self.logical_scope.as_ref()),
                )
            });
        // A TclOO definer whose script only declares members publishes its
        // class and nothing else, as at the top level, where such a region is
        // skipped; its blanket footprint would mark every later command of
        // the body as following an unknown mutation.
        let residual_footprint = !self.definition_declares_members_only(graph, invocation, schema);
        self.context.apply_source_body_lookup_effects(
            graph,
            command,
            invocation,
            schema,
            residual_footprint,
        );
        AdviceInvocationContext::install_source_class_handle(graph, class_handle);
        if let Some(tape) = &mut self.inventory {
            AdviceInvocationContext::install_source_class(tape, graph, class);
        }
        AdviceInvocationContext::install_authored_jim_class(
            self.inventory
                .as_mut()
                .unwrap_or(&mut self.authored_prefixes),
            graph,
            authored_jim,
        );
    }

    /// Answer a class-relation query whose parent this body walk retains,
    /// under the body ownership its class declaration and advice carry.
    fn retain_requested_body_class_relation(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        // A catalogue walk reads syntax on a branch graph of its own and
        // owns no parent advice, so it neither answers nor refuses a query.
        if self.inventory.is_none()
            || self.source_body.is_none()
            || self.catalogue_body.is_some()
            || self.member_catalogue_body.is_some()
        {
            return;
        }
        let owner = self.source_body.as_ref().and_then(|body| {
            Some((
                body,
                body_class_obligation(self.context, graph)?,
                self.logical_scope.as_ref(),
            ))
        });
        self.context.retain_requested_class_relation_with(
            invocation,
            graph,
            schema,
            owner,
            |advice| {
                if let Some(body) = &self.source_body {
                    Self::retain_source_body(self.context.policy, advice, body);
                    if matches!(self.context.policy, AdviceNamingPolicy::Logical(_)) {
                        advice.logical_scope.clone_from(&self.logical_scope);
                    }
                }
            },
        );
    }

    fn retain_child_schema(
        &mut self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<()> {
        self.retain_original_body_catalogues(graph, invocation, schema)?;
        if let (Some(tape), None) = (&mut self.inventory, &self.source_body) {
            // Immediate original operands use their root point, not a made-up
            // body descriptor or a future frame's applicability.
            if graph.blocks_registry_source(invocation.head) {
                tape.registry_barriers
                    .insert(invocation.native.first()?.span().start());
                return Some(());
            }
            return self.context.retain_source_inventory(
                tape,
                invocation,
                graph,
                schema,
                None,
                self.logical_scope.as_ref().map_or(
                    super::super::logical_navigation::LogicalCallbackScope::Unavailable,
                    super::super::logical_navigation::LogicalCallbackScope::Declared,
                ),
            );
        }
        let retained_producer = if let Some(body) = &self.source_body {
            let mut advice = self.context.schema_advice(invocation, graph, schema)?;
            Self::retain_source_body(self.context.policy, &mut advice, body);
            if matches!(self.context.policy, AdviceNamingPolicy::Logical(_)) {
                advice.logical_scope.clone_from(&self.logical_scope);
            }
            Some(advice)
        } else {
            None
        };
        self.context.retain_authored_command_prefix(
            &mut self.authored_prefixes,
            invocation,
            graph,
            schema,
            retained_producer.as_ref(),
        );
        let (Some(tape), Some(body)) = (&mut self.inventory, &self.source_body) else {
            return Some(());
        };
        self.context
            .retain_source_callback_targets(tape, invocation, graph, schema);
        self.context.retain_logical_callback_candidates(
            tape,
            invocation,
            graph,
            schema,
            self.logical_scope.as_ref().map_or(
                super::super::logical_navigation::LogicalCallbackScope::Unavailable,
                super::super::logical_navigation::LogicalCallbackScope::Declared,
            ),
        );
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        if graph.blocks_registry_source(invocation.head) {
            tape.registry_barriers
                .insert(invocation.native.first()?.span().start());
            return Some(());
        }
        let mut advice = self.context.schema_advice(invocation, graph, schema)?;
        Self::retain_source_body(self.context.policy, &mut advice, body);
        if matches!(self.context.policy, AdviceNamingPolicy::Logical(_)) {
            advice.logical_scope.clone_from(&self.logical_scope);
        }
        if let Some(previous) = tape.advice.get(&advice.site.offset) {
            if previous != &advice {
                tape.advice.remove(&advice.site.offset);
                tape.registry_barriers.insert(advice.site.offset);
                return Some(());
            }
        } else if tape.registry_barriers.contains(&advice.site.offset) {
            return Some(());
        }
        self.context.retain_produced_command_prefixes(
            tape,
            invocation,
            graph,
            schema,
            Some(&advice),
        );
        tape.advice.insert(advice.site.offset, advice);
        Some(())
    }

    fn retain_original_body_catalogues(
        &mut self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<()> {
        // Retain possible case syntax from the actual operand walk separately
        // from an ordinary invocation or any entered-body inventory.
        if matches!(self.context.policy, AdviceNamingPolicy::Native(_))
            && self.source_body.is_none()
            && self.catalogue_body.is_none()
            && self.member_catalogue_body.is_none()
            && schema.authored_source_case_invocation().is_some()
        {
            let offset = invocation.native.first()?.span().start();
            if self.authored_prefixes.conditional_case_syntax.len() < MAX_COMMANDS {
                let receipt = (!graph.blocks_registry_source(invocation.head))
                    .then(|| self.context.schema_advice(invocation, graph, schema))
                    .flatten();
                super::super::source_class::merge_receipt(
                    &mut self.authored_prefixes.conditional_case_syntax,
                    offset,
                    receipt,
                );
            }
        }
        if let Some(body) = &self.catalogue_body {
            let offset = invocation.native.first()?.span().start();
            if self.authored_prefixes.body_catalogue.len() < MAX_COMMANDS {
                let receipt = (!graph.blocks_registry_source(invocation.head)).then(|| {
                    let mut advice = self.context.schema_advice(invocation, graph, schema)?;
                    advice.catalogue_body = Some(body.clone());
                    advice.obligations.push(SourceCommandTransitionObligation::ConditionalOriginalBodyCatalogueApplicability);
                    (advice.matches_source(self.context.origin.source_image(), self.context.config) && advice.matches_context(self.context.context)).then_some(advice)
                }).flatten();
                super::super::source_class::merge_receipt(
                    &mut self.authored_prefixes.body_catalogue,
                    offset,
                    receipt,
                );
            }
        }
        if let Some(body) = &self.member_catalogue_body {
            let offset = invocation.native.first()?.span().start();
            if self.authored_prefixes.member_catalogue.len() < 256 {
                let receipt = (|| {
                    let policy = self.context.policy.native()?;
                    let selected = policy
                        .recipe()
                        .command_lookup_projection(
                            super::super::root_context(policy),
                            invocation.head.bytes(),
                        )
                        .ok()?;
                    let contextual_self = body
                        .receiver_grammar(self.context.context)
                        .is_some_and(|grammar| grammar.family.members_are_tcloo_methods())
                        && schema
                            .semantics
                            .traits
                            .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
                        && self.context.context.commands().method_dispatch_keyword(
                            std::str::from_utf8(invocation.head.bytes()).ok()?,
                        ) == Some(tcl_registry::MethodDispatchKind::SelfDispatch);
                    if (selected.namespace_route()
                        != tcl_syntax::naming::NativeCommandNamespaceRoute::Root
                        && !contextual_self)
                        || graph.blocks_registry_source(invocation.head)
                        || !body
                            .matches_source(self.context.origin.source_image(), self.context.config)
                        || !body.matches_context(self.context.context)
                    {
                        return None;
                    }
                    let mut advice = self.context.schema_advice(invocation, graph, schema)?;
                    advice.member_catalogue_body = Some(Arc::clone(body));
                    advice.obligations.push(SourceCommandTransitionObligation::ConditionalMemberBodyCatalogueApplicability);
                    (advice.matches_source(self.context.origin.source_image(), self.context.config)
                        && advice.matches_context(self.context.context))
                    .then_some(advice)
                })();
                super::super::source_class::merge_receipt(
                    &mut self.authored_prefixes.member_catalogue,
                    offset,
                    receipt,
                );
            }
        }
        Some(())
    }

    fn retain_source_body(
        policy: &AdviceNamingPolicy,
        advice: &mut OriginalSourceCommandTransitionAdvice,
        body: &Arc<crate::registry_invocation::OriginalSourceScriptBody>,
    ) {
        match policy {
            AdviceNamingPolicy::Logical(_) => {
                advice.logical_body = Some(Arc::clone(body));
                advice
                    .obligations
                    .push(SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability);
            }
            AdviceNamingPolicy::Native(_) => {
                advice.c_source_body = Some(Arc::clone(body));
                advice
                    .obligations
                    .push(SourceCommandTransitionObligation::ConditionalCBodyApplicability);
                if matches!(body.source_words().source(),
                    crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(parent)
                    if parent.obligations().contains(&SourceCommandTransitionObligation::ConditionalCapturedLifecycleSourceApplicability))
                {
                    advice.obligations.push(SourceCommandTransitionObligation::ConditionalCapturedLifecycleSourceApplicability);
                }
            }
        }
    }

    fn mark_child_header(&mut self, native: &NativeScriptCommandWords) -> Option<()> {
        if let Some(tape) = &mut self.inventory {
            tape.represented
                .insert(native.words.first()?.span().start());
        }
        Some(())
    }
}
