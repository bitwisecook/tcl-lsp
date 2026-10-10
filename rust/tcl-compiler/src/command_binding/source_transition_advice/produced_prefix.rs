// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original list-built command-prefix syntax retains its actual source producer.

use super::{
    AdviceGraph, AdviceInvocation, AdviceInvocationContext, OriginalSourceCommandTransition,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceAdviceNameInput, SourceAdviceWord, SourceCommandTransitionObligation, original_words,
    registry_words,
};
use std::sync::Arc;
use tcl_lexer::{ExecutablePart, NativeWord, SourceImage};
use tcl_registry::{ArgRole, InvocationWord, InvocationWords, Traits};

/// Conditional source syntax of a command prefix built by one original
/// invocation in an independently selected deferred operand. No value object,
/// dispatch point, future lookup, frame, Normal or edit authority is issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceProducedCommandPrefix {
    parent: Arc<OriginalSourceCommandTransitionAdvice>,
    producer: Arc<OriginalSourceCommandTransitionAdvice>,
    head: SourceAdviceNameInput,
    command: String,
    arguments: Vec<SourceAdviceWord>,
    roles: Option<Vec<(usize, ArgRole)>>,
    lineage: Vec<Arc<OriginalSourceCommandTransition>>,
    obligations: Vec<SourceCommandTransitionObligation>,
}
impl OriginalSourceProducedCommandPrefix {
    /// Exact parent schema owns the deferred original command-substitution word.
    #[must_use]
    pub fn parent(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.parent
    }
    /// Complete original builder invocation, independently of its target words.
    #[must_use]
    pub fn producer(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.producer
    }
    /// Genuine builder operand that supplies the future command head.
    #[must_use]
    pub const fn original_head(&self) -> &SourceAdviceNameInput {
        &self.head
    }
    /// Selected conditional target descriptor; this is not a reached lookup.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    /// Exact target aliases/moves keep their own prior original operands.
    #[must_use]
    pub fn lineage(&self) -> &[Arc<OriginalSourceCommandTransition>] {
        &self.lineage
    }
    /// Parent, builder and future invocation applicability remain explicit.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Full source, configuration and actual context agree at both producers.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: tcl_lexer::LexerConfig,
        context: &tcl_registry::model::ContextRegistry,
    ) -> bool {
        self.parent.matches_source(image, config)
            && self.parent.matches_context(context)
            && self.producer.matches_source(image, config)
            && self.producer.matches_context(context)
            && self.arguments.iter().all(|argument| {
                argument.original.image() == image && argument.original.config() == config
            })
    }
    pub(crate) fn arguments(&self) -> &[SourceAdviceWord] {
        &self.arguments
    }
    pub(crate) fn roles(&self) -> Option<&[(usize, ArgRole)]> {
        self.roles.as_deref()
    }
}

impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn produced_prefix(
        &self,
        offset: u32,
    ) -> Option<&OriginalSourceProducedCommandPrefix> {
        self.produced_prefixes.get(&offset)?.as_ref()
    }

    pub(super) fn merge_produced_prefix(
        &mut self,
        offset: u32,
        prefix: Option<OriginalSourceProducedCommandPrefix>,
    ) {
        self.produced_prefixes
            .entry(offset)
            .and_modify(|retained| {
                if retained != &prefix {
                    *retained = None;
                }
            })
            .or_insert(prefix);
    }
}

/// Whole original substitution container and its single original command.
/// Expansion markers retain source geometry only, without an evaluated argc.
pub(crate) struct OriginalSingleCommandSubstitution {
    operand: NativeWord,
    command: tcl_lexer::NativeScriptCommandWords,
}
impl OriginalSingleCommandSubstitution {
    pub(crate) const fn original_operand(&self) -> &NativeWord {
        &self.operand
    }
    pub(crate) const fn command(&self) -> &tcl_lexer::NativeScriptCommandWords {
        &self.command
    }
}

/// One whole original command substitution and its complete single command.
/// The returned vector retains only grammar/geometry; no builder, target,
/// evaluated value, callback, unexpanded argv or body purpose is selected here.
pub(crate) fn original_single_command_substitution_words(
    operand: &NativeWord,
) -> Option<OriginalSingleCommandSubstitution> {
    // naming.source.original-produced-command-prefix
    // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
    let arena = operand.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let ExecutablePart::Command { body } = part.part else {
        return None;
    };
    let mut plan =
        tcl_lexer::native_script_words_in(operand.image().clone(), body, operand.config()).ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
        return None;
    }
    Some(OriginalSingleCommandSubstitution {
        operand: operand.clone(),
        command: plan.commands.pop()?,
    })
}

pub(super) struct SelectedPrefixWords {
    pub(super) head: SourceAdviceNameInput,
    pub(super) arguments: Vec<SourceAdviceWord>,
    pub(super) command: String,
    pub(super) lineage: Vec<Arc<OriginalSourceCommandTransition>>,
}

impl AdviceInvocationContext<'_> {
    pub(super) fn retain_produced_command_prefixes(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        invocation: AdviceInvocation<'_>,
        graph: &AdviceGraph,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        retained_parent: Option<&OriginalSourceCommandTransitionAdvice>,
    ) {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        if schema.semantics.body_interpreter.resolve_with(|ordinal| {
            std::str::from_utf8(invocation.arguments.get(ordinal)?.value.as_deref()?).ok()
        }) != tcl_registry::world_effect::InterpreterScope::Current
        {
            return;
        }
        let positions = match self.policy {
            super::AdviceNamingPolicy::Logical(_) => {
                schema.authored_logical_source_plain_script_arguments()
            }
            super::AdviceNamingPolicy::Native(_) => schema.authored_source_plain_script_arguments(),
        };
        let Some(positions) = positions else {
            return;
        };
        let Some(parent) = retained_parent
            .cloned()
            .or_else(|| self.schema_advice(invocation, graph, schema))
        else {
            return;
        };
        let parent = Arc::new(parent);
        for ordinal in positions {
            let Some(argument) = invocation.arguments.get(ordinal) else {
                continue;
            };
            if !matches!(
                argument.origin,
                crate::registry_invocation::InvocationWordOrigin::Written(_)
            ) || argument.original.group().expand
                || tape.produced_prefixes.len() >= 256
            {
                continue;
            }
            if let Some(prefix) =
                self.produced_prefix(Arc::clone(&parent), &argument.original, graph)
            {
                tape.merge_produced_prefix(prefix.producer.site().offset, Some(prefix));
            }
        }
    }

    pub(super) fn selected_prefix_words(
        &self,
        native: &[NativeWord],
        graph: &AdviceGraph,
    ) -> Option<SelectedPrefixWords> {
        let mut written = original_words(native, self.policy)?;
        let head = written.first()?.input.as_ref()?.clone();
        if native.iter().any(|word| word.group().expand) {
            return None;
        }
        let (command, mut arguments, lineage) = graph.resolve(&head)?;
        for (ordinal, argument) in arguments.iter_mut().enumerate() {
            argument.origin =
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(ordinal);
        }
        arguments.extend(written.drain(1..));
        Some(SelectedPrefixWords {
            head,
            arguments,
            command,
            lineage,
        })
    }

    fn produced_prefix(
        &self,
        parent: Arc<OriginalSourceCommandTransitionAdvice>,
        operand: &NativeWord,
        graph: &AdviceGraph,
    ) -> Option<OriginalSourceProducedCommandPrefix> {
        let substitution =
            crate::command_binding::original_single_command_substitution_words(operand)?;
        if substitution.original_operand().group().expand {
            return None;
        }
        let builder = substitution.command();
        let SelectedPrefixWords {
            head,
            arguments,
            command,
            lineage,
        } = self.selected_prefix_words(&builder.words, graph)?;
        if arguments.iter().enumerate().any(|(ordinal, argument)| {
            argument.origin
                != crate::registry_invocation::InvocationWordOrigin::Written(ordinal + 1)
        }) {
            return None;
        }
        let values = registry_words(&arguments);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.commands(),
                Some(self.context.context()),
                InvocationWords::structured(InvocationWord::Literal(&command), &values)
                    .with_dialect(self.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved()?;
        if !schema
            .semantics
            .traits
            .contains(Traits::BUILDS_COMMAND_PREFIX)
            || schema.semantics.native_result
                != Some(
                    tcl_registry::native_result::NativeResultContract::ListArguments { from: 0 },
                )
            || schema.facts().arity_accepts_frozen_arguments() != Some(true)
        {
            return None;
        }
        let producer = self.schema_advice(
            AdviceInvocation {
                native: &builder.words,
                head: &head,
                arguments: &arguments,
                lineage: &lineage,
            },
            graph,
            &schema,
        )?;
        self.produced_target(parent, Arc::new(producer), builder.words.get(1..)?, graph)
    }

    fn produced_target(
        &self,
        parent: Arc<OriginalSourceCommandTransitionAdvice>,
        producer: Arc<OriginalSourceCommandTransitionAdvice>,
        native: &[NativeWord],
        graph: &AdviceGraph,
    ) -> Option<OriginalSourceProducedCommandPrefix> {
        let SelectedPrefixWords {
            head,
            arguments,
            command,
            lineage,
        } = self.selected_prefix_words(native, graph)?;
        let values = registry_words(&arguments);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.context.commands(),
                Some(self.context.context()),
                InvocationWords::structured(InvocationWord::Literal(&command), &values)
                    .with_dialect(self.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved()?;
        let (roles, complete) = self.authored_roles(&schema);
        let roles = complete
            .then(|| {
                roles
                    .into_iter()
                    .map(|(ordinal, role)| {
                        schema
                            .semantics
                            .argument_offset
                            .checked_add(usize::from(ordinal))
                            .map(|ordinal| (ordinal, role))
                    })
                    .collect::<Option<Vec<_>>>()
            })
            .flatten();
        let mut obligations = parent.obligations().to_vec();
        for obligation in producer.obligations() {
            if !obligations.contains(obligation) {
                obligations.push(obligation.clone());
            }
        }
        if !lineage.is_empty()
            && !obligations
                .contains(&SourceCommandTransitionObligation::WrittenTransitionApplicability)
        {
            obligations.push(SourceCommandTransitionObligation::WrittenTransitionApplicability);
        }
        obligations.push(SourceCommandTransitionObligation::ProducedCommandPrefixApplicability);
        Some(OriginalSourceProducedCommandPrefix {
            parent,
            producer,
            head,
            command: schema.canonical_command.to_owned(),
            arguments,
            roles,
            lineage,
            obligations,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_produced_command_prefix_words,
    };
    use std::sync::Arc;

    fn builder(source: &str, analysis: &AnalysisResult) -> crate::segmenter::SegmentedCommand {
        let config = analysis.body_lexer_config.unwrap();
        let image = tcl_lexer::SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image,
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let body = plan
            .commands
            .iter()
            .flat_map(|command| &command.words)
            .find_map(|word| {
                word.executable_parts()
                    .all_parts()
                    .find_map(|part| match part.part {
                        tcl_lexer::ExecutablePart::Command { body } => Some(body),
                        _ => None,
                    })
            })
            .unwrap();
        crate::segmenter::segment_commands_with_offset_and_config(
            &source[body.as_range()],
            body.start(),
            config,
        )
        .remove(0)
    }

    #[test]
    fn substitution_geometry_keeps_expansion_and_declines_compound_or_multiple_commands() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        let profile =
            tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        for (source, expected, expanded) in [
            ("set x [list source file.tcl]", true, false),
            ("set x {*}[list source file.tcl]", true, true),
            ("set x prefix[list source file.tcl]", false, false),
            ("set x [list source file.tcl; list other]", false, false),
        ] {
            let plan = tcl_lexer::native_script_words_in(
                tcl_lexer::SourceImage::document(source),
                tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
                config,
            )
            .unwrap();
            let operand = &plan.commands[0].words[2];
            let captured = super::original_single_command_substitution_words(operand);
            assert_eq!(captured.is_some(), expected, "{source}");
            if let Some(captured) = captured {
                assert_eq!(captured.original_operand(), operand);
                assert_eq!(captured.original_operand().group().expand, expanded);
                assert_eq!(captured.command().words[0].bytes(), b"list");
            }
        }
    }

    #[test]
    fn logical_produced_prefix_retains_consensus_roles_without_an_entered_frame() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let source = "uplevel #0 [list upvar #0 originalName localName]";
        let analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let segment = builder(source, &analysis);
        let words = source_produced_command_prefix_words(source, &analysis, &segment).unwrap();
        let OriginalRegistrySource::ProducedPrefix(prefix) = words.source() else {
            panic!("missing own list-built prefix");
        };
        assert_eq!(
            prefix.parent().logical_source_input(),
            analysis.resolved_input.as_ref()
        );
        assert_eq!(
            prefix.parent().roles(),
            Some([(1, tcl_registry::ArgRole::Body)].as_slice())
        );
        assert_eq!(
            words.roles(),
            Some([(2, tcl_registry::ArgRole::VarWrite)].as_slice())
        );
        assert_eq!(
            words.operands()[2]
                .as_ref()
                .unwrap()
                .word()
                .unwrap()
                .bytes(),
            b"localName"
        );
        assert!(!words.operands_preserve_source_lookup());
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        assert!(!prefix.matches_source_context(
            &tcl_lexer::SourceImage::native(source.as_bytes()),
            analysis.body_lexer_config.unwrap(),
            &context,
        ));
    }

    #[test]
    fn original_produced_prefix_keeps_deferred_parent_builder_and_each_word() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        let source = "after idle [list apply {{x} {puts $x}} 5]";
        for dialect in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let analysis = Analyser::new().analyse(source, dialect);
            let segment = builder(source, &analysis);
            let words =
                source_produced_command_prefix_words(source, &analysis, &segment).expect(dialect);
            let OriginalRegistrySource::ProducedPrefix(prefix) = words.source() else {
                panic!("ordinary source lookup replaced the produced owner");
            };
            assert_eq!(
                prefix.original_head().original_word().unwrap().bytes(),
                b"apply"
            );
            assert_eq!(prefix.producer().original_words().len(), 4);
            assert_eq!(prefix.parent().original_words().len(), 3);
            assert!(prefix.obligations().contains(
                &super::SourceCommandTransitionObligation::ProducedCommandPrefixApplicability
            ));
            assert_eq!(
                words.roles(),
                Some([(0, tcl_registry::ArgRole::LambdaLiteral)].as_slice())
            );
            let lambda = words.operands()[0].as_ref().unwrap().word().unwrap();
            assert_eq!(lambda.image(), &tcl_lexer::SourceImage::document(source));
            assert_eq!(lambda.bytes(), b"{{x} {puts $x}}");
            assert!(!words.operands_preserve_source_lookup());
            let context = analysis.resolved_input.as_ref().unwrap().context_registry();
            assert!(!prefix.matches_source_context(
                &tcl_lexer::SourceImage::native(source.as_bytes()),
                analysis.body_lexer_config.unwrap(),
                &context,
            ));
            let mut tape = super::OriginalSourceTransitionAdviceTape::default();
            let offset = prefix.producer().site().offset;
            tape.merge_produced_prefix(offset, Some(prefix.as_ref().clone()));
            let mut conflicting = prefix.as_ref().clone();
            conflicting
                .obligations
                .push(super::SourceCommandTransitionObligation::UnknownEarlierMutation);
            tape.merge_produced_prefix(offset, Some(conflicting));
            tape.merge_produced_prefix(offset, Some(prefix.as_ref().clone()));
            assert!(
                tape.produced_prefix(offset).is_none(),
                "conflicting ownership stays terminal"
            );
            assert!(
                source_produced_command_prefix_words(
                    &source.replace("puts", "gets"),
                    &analysis,
                    &segment
                )
                .is_none()
            );
        }
    }

    #[test]
    fn original_produced_prefix_keeps_logical_context_capture_and_terminal_refusals() {
        // naming.source.original-produced-command-prefix
        // docs/design/analysis/name-resolution-proofs/original-produced-command-prefix.md
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        let source = "interp alias {} thunk {} apply {{x} {puts $x}}; after idle [list thunk 5]";
        let analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, profile.name);
        let segment = builder(source, &analysis);
        let words = source_produced_command_prefix_words(source, &analysis, &segment)
            .expect("selected original alias prefix");
        let OriginalRegistrySource::ProducedPrefix(prefix) = words.source() else {
            panic!("missing producer");
        };
        assert_eq!(prefix.parent().logical_source_input(), Some(&input));
        assert!(!prefix.lineage().is_empty());
        assert!(
            words.operands()[0].is_none(),
            "captured lambda cannot borrow the later list source anchor"
        );
        assert!(matches!(
            words.origins()[1],
            crate::registry_invocation::InvocationWordOrigin::BindingPrefix(_)
        ));
        for source in [
            "set data [list apply {{x} {puts $x}} 5]",
            "apply [list upvar {puts ignored}]",
            "apply [list apply {{x} {puts $x}} 5]",
            "after idle [list $head {{x} {puts $x}} 5]",
            "after idle [list apply {{x} {puts $x}} 5; list noop]",
            "proc list args {}; after idle [list apply {{x} {puts $x}} 5]",
            "proc apply args {}; after idle [list apply {{x} {puts $x}} 5]",
        ] {
            let analysis = Analyser::new()
                .with_resolved_input(input.clone())
                .analyse(source, profile.name);
            assert!(
                source_produced_command_prefix_words(
                    source,
                    &analysis,
                    &builder(source, &analysis)
                )
                .is_none(),
                "{source}"
            );
        }
        let old_context =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        let old_input =
            ResolvedAnalysisInput::new(profile, profile, old_context, input.lexer_config());
        let modern = "after idle [list apply {{x} {puts $x}} 5]";
        let old = Analyser::new()
            .with_resolved_input(old_input)
            .analyse(modern, profile.name);
        assert!(
            source_produced_command_prefix_words(modern, &old, &builder(modern, &old)).is_none()
        );
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::new(context.with_command_store(context.commands().snapshot().shared_registry())),
            input.lexer_config(),
        ));
        assert!(source_produced_command_prefix_words(source, &foreign, &segment).is_none());
        let config = tcl_lexer::LexerConfig {
            expand_syntax: true,
            ..input.lexer_config()
        };
        let expanded_input = ResolvedAnalysisInput::new(profile, profile, context, config);
        let source = "after idle [list {*}$head {{x} {puts $x}} 5]";
        let expanded = Analyser::new()
            .with_resolved_input(expanded_input)
            .analyse(source, profile.name);
        assert!(
            source_produced_command_prefix_words(source, &expanded, &builder(source, &expanded))
                .is_none()
        );
    }
}
