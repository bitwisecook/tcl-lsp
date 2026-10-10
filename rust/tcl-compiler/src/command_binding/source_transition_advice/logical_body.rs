// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deferred Logical source headers retain their original parent body/schema.

use super::body_effects::{OriginalOperandEffect, original_operand_effects};
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

const MAX_DEPTH: usize = 64;
const MAX_COMMANDS: usize = 2048;

struct LogicalBodyWalk<'context, 'source> {
    context: &'context AdviceInvocationContext<'source>,
    visited: usize,
}

pub(super) struct LogicalNamespaceScope {
    name: super::OriginalLogicalSourceNameInput,
    namespace: tcl_core_types::ByteNamespacePath,
    receipt: Arc<super::OriginalSourceCommandTransition>,
}
impl LogicalNamespaceScope {
    pub(super) fn namespace(&self) -> &tcl_core_types::ByteNamespacePath {
        &self.namespace
    }
}

impl AdviceInvocationContext<'_> {
    pub(super) fn logical_namespace_scope(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<LogicalNamespaceScope> {
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
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
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
        if !matches!(self.policy, AdviceNamingPolicy::Logical(_)) {
            return;
        }
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
        if schema.semantics.traits.contains(Traits::DEFERS_BODY) {
            self.procedure(tape, graph, invocation, schema, parent, depth)
        } else {
            self.namespace(tape, graph, invocation, schema, parent, depth)
        }
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
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        if depth > MAX_DEPTH
            || !schema.semantics.traits.contains(Traits::DEFERS_BODY)
            || schema.facts().arity_accepts_frozen_arguments() != Some(true)
        {
            return None;
        }
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
        let mut advice = self.context.schema_advice(invocation, graph, schema)?;
        advice.logical_body = parent.map(|parent| Arc::clone(parent.body_arc()));
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
        let mut branch = graph.clone();
        branch.apply(&receipt)?;
        branch.logical_namespace = Some(key.namespace);
        branch.handles.clear();
        branch.class_handles.clear();
        self.script(tape, &mut branch, &scope, depth.checked_add(1)?)
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
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        if depth > MAX_DEPTH {
            return None;
        }
        let selected = self
            .context
            .logical_namespace_scope(graph, invocation, schema)?;
        let mut advice = self.context.schema_advice(invocation, graph, schema)?;
        advice.logical_body = parent.map(|parent| Arc::clone(parent.body_arc()));
        let bodies = self.bodies(&advice)?;
        if bodies.len() != 1 {
            return None;
        }
        let AdviceNamingPolicy::Logical(input) = self.context.policy else {
            return None;
        };
        let scope = Arc::new(OriginalDeclaredLogicalBodyContext::capture(
            Arc::new(bodies.into_iter().next()?),
            selected.name,
            selected.namespace.clone(),
            OriginalDeclaredSourceBodyFrame::Namespace,
            input,
            parent.cloned(),
        )?);
        let mut branch = graph.clone();
        branch.apply(&selected.receipt)?;
        branch.logical_namespace = Some(selected.namespace);
        self.script(tape, &mut branch, &scope, depth.checked_add(1)?)
    }

    fn bodies(
        &self,
        advice: &OriginalSourceCommandTransitionAdvice,
    ) -> Option<Vec<OriginalSourceScriptBody>> {
        let source = self.context.origin.source_image().try_text().ok()?;
        let words =
            crate::registry_invocation::source_structure::source_transition_words_from_advice(
                source,
                self.context.config,
                self.context.context,
                advice.clone(),
            )?;
        let bodies = words.source_script_bodies(self.context.context);
        if bodies.iter().any(|body| {
            let container = body.original_container();
            let content = container.content_span().ok();
            !body.matches_source(self.context.origin.source_image(), self.context.config)
                || !body.matches_context(self.context.context)
                || content.and_then(|span| container.image().bytes().get(span.as_range()))
                    != tcl_syntax::word_rules::original_static_word_ascii_presentation(container)
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
        let native = &command.words;
        tape.represented.insert(native.first()?.span().start());
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
            graph.widen(native);
            return Some(());
        };
        let original_selection = class_handle
            .is_some()
            .then(|| graph.resolve(&head))
            .flatten();
        let operand_barrier =
            self.retain_deferred_operand_header(tape, graph, native, &head, (scope, depth))?;
        let Some((selected, mut arguments, lineage)) =
            original_selection.or_else(|| graph.resolve(&head))
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
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.context.commands(),
                Some(self.context.context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&selected),
                    &values,
                )
                .with_dialect(self.context.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
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
        let class = self.context.source_class_declaration_in_body(
            graph,
            invocation,
            schema,
            (
                scope.body_arc(),
                SourceCommandTransitionObligation::DeferredLogicalBodyApplicability,
            ),
        );
        self.context
            .apply_schema_transition(graph, invocation, selected, schema)?;
        if operand_barrier {
            graph.widen(native);
        }
        self.context
            .install_registered_results(tape, graph, factory, handle);
        AdviceInvocationContext::install_source_class_handle(graph, class_handle);
        AdviceInvocationContext::install_source_class(tape, graph, class);
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
        let mut advice = self.context.schema_advice(invocation, graph, schema)?;
        advice.logical_body = Some(Arc::clone(scope.body_arc()));
        advice
            .obligations
            .push(SourceCommandTransitionObligation::DeferredLogicalBodyApplicability);
        self.context
            .retain_authored_command_prefix(tape, invocation, graph, schema, Some(&advice));
        tape.advice.insert(advice.site.offset, advice.clone());
        self.context
            .retain_source_callback_targets(tape, invocation, graph, schema);
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
        if schema.semantics.traits.contains(Traits::DEFERS_BODY)
            || schema.semantics.traits.contains(Traits::DECLARES_NAMESPACE)
        {
            // Each changing source scope retains its own selected name/body.
            // Unmodelled workers never inherit a procedure namespace/frame.
            let mut retained = OriginalSourceTransitionAdviceTape::default();
            if self
                .parent_scope(&mut retained, graph, invocation, schema, Some(scope), depth)
                .is_some()
            {
                tape.extend_inventory(retained);
            }
            return Some(());
        }
        if schema.semantics.body_kind != tcl_registry::BodyKind::Plain {
            return Some(());
        }
        for body in self.bodies(advice)? {
            let mut branch = graph.clone();
            let child = Arc::new(scope.with_body(Arc::new(body))?);
            self.script(tape, &mut branch, &child, depth.checked_add(1)?)?;
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
    fn logical_deferred_source_body_retains_nested_schema_and_parent_geometry() {
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
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
        // naming.minifier.complete-logical-metadata
        // docs/design/analysis/name-resolution-proofs/minifier-complete-logical-metadata.md
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
        // naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
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
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
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
        let commands =
            std::sync::Arc::new(tcl_registry::command_registry::CommandRegistry::build_default());
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
}
