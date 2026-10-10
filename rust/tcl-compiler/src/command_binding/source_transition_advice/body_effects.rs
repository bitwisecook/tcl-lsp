// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original immediate-body mutation coverage for conditional source advice.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceInvocationContext, AdviceNamingPolicy,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceCommandTransitionObligation, capture_transition, original_words, registry_words,
};
use std::sync::Arc;
use tcl_lexer::{ExecutablePart, NativeScriptCommandWords, NativeWord, Span};
use tcl_registry::world_effect::{EffectAccessMode, WorldStateDomain};

const MAX_DEPTH: usize = 64;
const MAX_COMMANDS: usize = 2048;

pub(super) struct BodyEffectProjection {
    pub(super) effects_complete: bool,
    pub(super) inventory: OriginalSourceTransitionAdviceTape,
}

struct BodyWalk<'context, 'source> {
    context: &'context AdviceInvocationContext<'source>,
    visited: usize,
    inventory: Option<OriginalSourceTransitionAdviceTape>,
    source_body: Option<Arc<crate::registry_invocation::OriginalSourceScriptBody>>,
}

impl AdviceInvocationContext<'_> {
    /// Reuse the original executable arena before selecting source head advice.
    /// Known child transitions are processed; unresolved variable/observer
    /// effects stay separate from the post-operand execution lookup.
    pub(super) fn inspect_operand_effects(
        &self,
        graph: &mut AdviceGraph,
        native: &[NativeWord],
    ) -> Option<bool> {
        let substitutions = native.iter().any(|word| {
            word.executable_parts()
                .all_parts()
                .any(|part| !matches!(part.part, ExecutablePart::Text(_)))
        });
        if substitutions {
            graph.record_uncertainty(native);
            BodyWalk {
                context: self,
                visited: 0,
                inventory: None,
                source_body: None,
            }
            .substitutions(graph, native, 0)?;
        }
        Some(substitutions)
    }

    /// Inspect real original regions before carrying unresolved source advice.
    /// Every possible branch may withdraw an earlier alias; a body never
    /// installs its speculative new declarations into the enclosing graph.
    pub(super) fn apply_immediate_body_effects(
        &self,
        graph: &mut AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> BodyEffectProjection {
        let mut walk = BodyWalk {
            context: self,
            visited: 0,
            inventory: (matches!(self.policy, AdviceNamingPolicy::Logical(_))
                && schema.semantics.body_kind == tcl_registry::BodyKind::Plain)
                .then(OriginalSourceTransitionAdviceTape::default),
            source_body: None,
        };
        let expressions_complete = walk.expressions(graph, advice, schema, 0).is_some();
        if !expressions_complete && walk.inventory.is_some() {
            // The conditional source question survives unavailable effects;
            // execution state is still widened by the caller independently.
            graph.record_uncertainty(advice.original_words());
        }
        let regions_complete = (expressions_complete || walk.inventory.is_some())
            && walk.regions(graph, advice, schema, 0).is_some();
        BodyEffectProjection {
            effects_complete: expressions_complete && regions_complete,
            inventory: if regions_complete {
                walk.inventory.unwrap_or_default()
            } else {
                OriginalSourceTransitionAdviceTape::default()
            },
        }
    }
}

impl BodyWalk<'_, '_> {
    fn regions(
        &mut self,
        graph: &mut AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
    ) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        let source = self.context.origin.source_image().try_text().ok()?;
        let words =
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                source,
                self.context.config,
                self.context.context,
                advice.clone(),
            )?;
        let interpreter = schema.semantics.body_interpreter.resolve_with(|ordinal| {
            std::str::from_utf8(words.arguments().get(ordinal)?.literal_bytes()?).ok()
        });
        if interpreter != tcl_registry::world_effect::InterpreterScope::Current {
            return None;
        }
        let regions = words.source_script_bodies_for_mutation_coverage(self.context.context)?;
        if regions.is_empty() {
            return None;
        }
        let original = graph.clone();
        for region in regions {
            if !region.matches_source(self.context.origin.source_image(), self.context.config)
                || !region.matches_context(self.context.context)
            {
                return None;
            }
            let span = region.content_span();
            // A changing Logical namespace uses the same original scope
            // recipe as body assistance; no Native frame or execution follows.
            let mut branch = original.clone();
            if span.start() != span.end()
                && schema.semantics.body_kind != tcl_registry::BodyKind::Plain
            {
                let selected = self.context.logical_namespace_scope(
                    &original,
                    AdviceInvocation {
                        native: advice.original_words(),
                        head: advice.original_head(),
                        arguments: advice.arguments(),
                        lineage: advice.lineage(),
                    },
                    schema,
                )?;
                branch.logical_namespace = Some(selected.namespace().clone());
            }
            let source_body = (self.inventory.is_some()
                && schema.semantics.body_kind == tcl_registry::BodyKind::Plain)
                .then(|| Arc::new(region));
            let parent = std::mem::replace(&mut self.source_body, source_body);
            let walked = self.script(&mut branch, span, depth);
            self.source_body = parent;
            walked?;
            Self::retain_possible_effects(graph, &original, branch, advice.original_words());
        }
        Some(())
    }

    fn retain_possible_effects(
        graph: &mut AdviceGraph,
        original: &AdviceGraph,
        branch: AdviceGraph,
        operation: &[NativeWord],
    ) {
        for (key, cell) in &original.cells {
            if branch.cells.get(key) != Some(cell) {
                // Possible original source mutations block an earlier schema;
                // they do not prove that a delete or replacement succeeded.
                if graph.native_baseline
                    && matches!(cell, AdviceCell::Descriptor(_, lineage) if lineage.is_empty())
                {
                    // The authentic entry schema is a source possibility only;
                    // a conditional branch does not prove its replacement ran.
                    graph.record_uncertainty(operation);
                } else {
                    graph.cells.insert(key.clone(), AdviceCell::Shadowed);
                }
            }
        }
        for operation in branch.uncertain_operations {
            if !graph.uncertain_operations.contains(&operation) {
                graph.uncertain_operations.push(operation);
            }
        }
    }

    fn expressions(
        &mut self,
        graph: &mut AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
    ) -> Option<()> {
        let (roles, complete) = schema.authored_source_argument_roles();
        if !complete {
            return None;
        }
        for (ordinal, role) in roles {
            if role != tcl_registry::ArgRole::Expr {
                continue;
            }
            let ordinal = schema
                .facts()
                .argument_offset
                .checked_add(usize::from(ordinal))?;
            let argument = advice.arguments().get(ordinal)?;
            if !matches!(
                argument.origin,
                crate::registry_invocation::InvocationWordOrigin::Written(_)
            ) {
                return None;
            }
            let expression_span = argument.original.content_span().ok()?;
            let source = argument
                .original
                .image()
                .try_text()
                .ok()?
                .get(expression_span.as_range())?;
            // Cooked values and incomplete expressions cannot lend their
            // original positions to the checked expression/script bridge.
            if argument.value.as_deref() != Some(source.as_bytes()) {
                return None;
            }
            let mut context = self
                .context
                .dialect
                .expression_parse_context(self.context.context.commands().profile());
            context.lexer_grammar = self.context.config.grammar_over(context.lexer_grammar);
            let substitutions =
                tcl_syntax::expr::checked_expression_substitutions(source, &context)?;
            if !substitutions.variables.is_empty() || !substitutions.commands.is_empty() {
                graph.record_uncertainty(advice.original_words());
            }
            let original = graph.clone();
            for command in substitutions.commands {
                // The shared expression owner supplies whole brackets. Lazy
                // branches are lexical possibilities, never entered scripts.
                let start = expression_span
                    .start()
                    .checked_add(command.start())?
                    .checked_add(1)?;
                let end = expression_span
                    .start()
                    .checked_add(command.end())?
                    .checked_sub(1)?;
                let mut branch = original.clone();
                self.script(&mut branch, Span::new(start, end), depth.checked_add(1)?)?;
                Self::retain_possible_effects(graph, &original, branch, advice.original_words());
            }
        }
        Some(())
    }

    fn script(&mut self, graph: &mut AdviceGraph, span: Span, depth: usize) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
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
            self.command(graph, &command, depth)?;
        }
        Some(())
    }

    fn substitutions(
        &mut self,
        graph: &mut AdviceGraph,
        native: &[NativeWord],
        depth: usize,
    ) -> Option<()> {
        let mut covered = Vec::<Span>::new();
        for word in native {
            for part in word.executable_parts().all_parts() {
                match part.part {
                    ExecutablePart::Text(_) => {}
                    ExecutablePart::Command { body, .. } => {
                        if covered
                            .iter()
                            .any(|outer| outer.start() <= body.start() && body.end() <= outer.end())
                        {
                            continue;
                        }
                        self.script(graph, body, depth.checked_add(1)?)?;
                        covered.push(body);
                    }
                    _ => graph.record_uncertainty(native),
                }
            }
        }
        Some(())
    }

    fn command(
        &mut self,
        graph: &mut AdviceGraph,
        native: &NativeScriptCommandWords,
        depth: usize,
    ) -> Option<()> {
        self.mark_child_header(native)?;
        if let (Some(tape), Some(body)) = (&mut self.inventory, &self.source_body)
            && self.context.retain_original_body_constructor_calls(
                tape,
                graph,
                &native.words,
                (
                    body,
                    SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
                ),
            )
        {
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
        let Some(head) = written
            .first()
            .and_then(|word| word.input.as_ref())
            .cloned()
        else {
            graph.record_uncertainty(&native.words);
            return Some(());
        };
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
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.context.commands(),
                Some(self.context.context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&command),
                    &words,
                )
                .with_dialect(self.context.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
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
        self.retain_child_schema(graph, invocation, &schema)?;
        self.apply_original_body_class_effects(graph, &command, invocation, &schema, class_handle);
        let immediate = schema
            .authored_source_argument_roles()
            .0
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::Body)
            && !schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DEFERS_BODY);
        let advice = self.context.schema_advice(invocation, graph, &schema)?;
        if self.expressions(graph, &advice, &schema, depth).is_none() {
            graph.widen(&native.words);
        }
        if immediate
            && self
                .regions(graph, &advice, &schema, depth.checked_add(1)?)
                .is_none()
        {
            graph.widen(&native.words);
        }
        Some(())
    }
    fn original_body_class_handle(
        &self,
        graph: &AdviceGraph,
        native: &[NativeWord],
    ) -> Option<super::source_instance::PendingSourceClassHandle> {
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
        class_handle: Option<super::source_instance::PendingSourceClassHandle>,
    ) {
        let class = self
            .inventory
            .as_ref()
            .and(self.source_body.as_ref())
            .and_then(|body| {
                self.context.source_class_declaration_in_body(
                    graph,
                    invocation,
                    schema,
                    (
                        body,
                        SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability,
                    ),
                )
            });
        self.apply_command_effects(graph, command, invocation, schema);
        AdviceInvocationContext::install_source_class_handle(graph, class_handle);
        if let Some(tape) = &mut self.inventory {
            AdviceInvocationContext::install_source_class(tape, graph, class);
        }
    }
    fn retain_child_schema(
        &mut self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<()> {
        let (Some(tape), Some(body)) = (&mut self.inventory, &self.source_body) else {
            return Some(());
        };
        self.context
            .retain_source_callback_targets(tape, invocation, graph, schema);
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        if graph.blocks_registry_source(invocation.head) {
            tape.registry_barriers
                .insert(invocation.native.first()?.span().start());
            return Some(());
        }
        let mut advice = self.context.schema_advice(invocation, graph, schema)?;
        advice.logical_body = Some(Arc::clone(body));
        advice
            .obligations
            .push(SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability);
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

    fn mark_child_header(&mut self, native: &NativeScriptCommandWords) -> Option<()> {
        if let Some(tape) = &mut self.inventory
            && self.source_body.is_some()
        {
            tape.represented
                .insert(native.words.first()?.span().start());
        }
        Some(())
    }

    fn apply_command_effects(
        &self,
        graph: &mut AdviceGraph,
        command: &str,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) {
        let transitions = schema.state_transitions();
        let receipt = capture_transition(
            self.context.origin,
            invocation.native,
            command,
            invocation.arguments,
            transitions.clone(),
            invocation.lineage,
        );
        if receipt
            .as_ref()
            .is_some_and(|receipt| graph.apply(receipt).is_none())
            || (receipt.is_none() && !transitions.facts().is_empty())
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandBindings)
            || transitions.widens(tcl_registry::StateTransitionDomain::CommandResolution)
        {
            graph.widen(invocation.native);
        }
        let effects = schema.effect_footprint();
        // Transition effect coverage removes an already represented trace
        // write from the generic footprint. Source observer state still
        // requires its own owner; a covered registration is not a quiet read.
        let changes_observers = transitions
            .facts()
            .iter()
            .any(|fact| matches!(fact.transition, tcl_registry::StateTransition::Trace(_)))
            || [
                tcl_registry::StateTransitionDomain::VariableTraces,
                tcl_registry::StateTransitionDomain::CommandTraces,
                tcl_registry::StateTransitionDomain::ExecutionTraces,
            ]
            .into_iter()
            .any(|domain| transitions.widens(domain));
        if changes_observers
            || effects.requires_world_barrier()
            || effects.accesses().iter().any(|access| {
                access.mode != EffectAccessMode::Read
                    && matches!(
                        access.domain,
                        WorldStateDomain::VariableTraces
                            | WorldStateDomain::CommandTraces
                            | WorldStateDomain::ExecutionTraces
                    )
            })
        {
            // A trace registration can change future callback state without
            // synchronously reentering this invocation. Its exact producer
            // remains unresolved until a separate observer-state owner closes
            // that state. Neither it nor a synchronous callback footprint
            // permits execution or semantic rewriting from source advice.
            graph.record_uncertainty(invocation.native);
        }
        if effects.accesses().iter().any(|access| {
            access.mode != EffectAccessMode::Read
                && matches!(
                    access.domain,
                    WorldStateDomain::CommandBindings | WorldStateDomain::NamespaceLookup
                )
        }) && receipt.is_none()
        {
            graph.widen(invocation.native);
        }
    }
}

#[cfg(test)]
mod logical_body_inventory_tests {
    use crate::command_binding::SourceCommandTransitionObligation;
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_registry_words,
    };

    fn analyse(source: &str) -> crate::analyser::AnalysisResult {
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        crate::analyser::Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    #[test]
    fn logical_immediate_body_schema_keeps_original_geometry_and_unknown_effects_separate() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        for source in [
            "switch $subject one {set local 1} default {set other 2}",
            "if $condition {set local 1}",
            "foreach item $values {set local 1}",
        ] {
            let analysis = analyse(source);
            let input = analysis.resolved_input.as_ref().unwrap();
            let config = input.lexer_config();
            let context = input.context_registry();
            let root = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
                .remove(0);
            let parent = source_registry_words(source, &analysis, &root).unwrap();
            let bodies = parent.source_script_bodies(&context);
            assert!(!bodies.is_empty(), "{source}");
            for body in bodies {
                let span = body.content_span();
                let child = crate::segmenter::segment_commands_with_offset_and_config(
                    &source[span.as_range()],
                    span.start(),
                    config,
                )
                .remove(0);
                let words = source_registry_words(source, &analysis, &child).expect(source);
                let OriginalRegistrySource::SourceTransitions(advice) = words.source() else {
                    panic!("missing own conditional Logical body: {source}");
                };
                assert_eq!(advice.logical_source_input(), Some(input));
                assert_eq!(advice.logical_source_body(), Some(&body));
                assert_eq!(
                    words.roles(),
                    Some([(0, tcl_registry::ArgRole::VarWrite)].as_slice())
                );
                assert!(advice.obligations().contains(
                    &SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability
                ));
                assert!(
                    advice
                        .obligations()
                        .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
                );
                assert!(!words.operands_preserve_source_lookup());
                assert!(
                    !advice
                        .matches_source(&tcl_lexer::SourceImage::native(source.as_bytes()), config)
                );
            }
        }
    }

    #[test]
    fn logical_immediate_body_schema_keeps_child_mutations_and_foreign_owners_terminal() {
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = "switch $subject one {rename set gone; set blocked 1}";
        let analysis = analyse(source);
        let config = analysis.resolved_input.as_ref().unwrap().lexer_config();
        let offset = u32::try_from(source.find("set blocked").unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            &source[usize::try_from(offset).unwrap()..source.len() - 1],
            offset,
            config,
        )
        .remove(0);
        assert!(source_registry_words(source, &analysis, &segment).is_none());
        let source = "if $condition {set local 1}";
        let mut analysis = analyse(source);
        let offset = u32::try_from(source.find("set local").unwrap()).unwrap();
        let segment = crate::segmenter::segment_commands_with_offset_and_config(
            &source[usize::try_from(offset).unwrap()..source.len() - 1],
            offset,
            config,
        )
        .remove(0);
        assert!(source_registry_words(source, &analysis, &segment).is_some());
        assert!(source_registry_words(&format!("{source} "), &analysis, &segment).is_none());
        analysis.body_lexer_config = Some(tcl_lexer::LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        });
        assert!(source_registry_words(source, &analysis, &segment).is_none());
        let native = crate::analyser::Analyser::new().analyse(source, "tcl9.1");
        if let Some(words) = source_registry_words(source, &native, &segment)
            && let OriginalRegistrySource::SourceTransitions(advice) = words.source()
        {
            assert!(advice.logical_source_input().is_none());
            assert!(
                !advice.obligations().contains(
                    &SourceCommandTransitionObligation::ConditionalLogicalBodyApplicability
                )
            );
        }
    }
}
