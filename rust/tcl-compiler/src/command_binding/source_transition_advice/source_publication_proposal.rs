// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conditional proposals for new source publications retain their whole source timeline.

use super::{
    AdviceCell, AdviceGraph, AdviceNamingPolicy, AdvicePublicationPurpose,
    OriginalSourceCataloguePurpose, OriginalSourceTransitionSeed, SourceAdviceQuery,
    original_words, registry_words,
};
use crate::analyser::ResolvedAnalysisInput;
use std::cell::Cell;
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath};
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_registry::{
    CommandBindingDefinitionKind, CommandBindingTransition, model::ContextRegistry,
};

/// A conditional proposal for a new root source procedure. This retains source
/// collision and constructor premises, without proving a physical vacant slot,
/// executing the template or permitting an edit. Callers own diagnostic and edit
/// applicability independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourcePublicationProposal {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    template: SourceImage,
    call: SourceImage,
    publication: ByteCommandSlot,
}

impl OriginalSourcePublicationProposal {
    /// Exact source/configuration and independently selected authoring context.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, input: &ResolvedAnalysisInput) -> bool {
        &self.image == image && &self.input == input
    }

    /// Conditional source coordinate for the proposed definition, never a live slot.
    #[must_use]
    pub fn source_publication(&self) -> &ByteCommandSlot {
        &self.publication
    }

    /// The proposed source, distinct from the retained original document.
    #[must_use]
    pub fn template(&self) -> &SourceImage {
        &self.template
    }

    /// The proposed call syntax, without a reached argv or handler.
    #[must_use]
    pub fn call(&self) -> &SourceImage {
        &self.call
    }
}

#[derive(Debug)]
struct GeneratedCommandRequirement {
    slot: ByteCommandSlot,
    descriptor: String,
}

#[derive(Clone, Copy)]
pub(super) struct PublicationProposalQuery<'a> {
    publication: &'a ByteCommandSlot,
    commands: &'a [GeneratedCommandRequirement],
    admitted: &'a Cell<bool>,
}

impl PublicationProposalQuery<'_> {
    pub(super) fn withdraw(self) {
        self.admitted.set(false);
    }

    pub(super) fn observe(self, graph: &AdviceGraph) {
        if graph.cells.contains_key(self.publication)
            || !graph.uncertain_operations.is_empty()
            || graph.unknown_paths.contains(&ByteNamespacePath::root())
            || !self.commands.iter().all(|requirement| {
                matches!(graph.cells.get(&requirement.slot),
                    Some(AdviceCell::Descriptor(descriptor, lineage))
                    if descriptor == &requirement.descriptor && lineage.is_empty())
            })
        {
            self.admitted.set(false);
        }
    }
}

fn original_plan(
    image: &SourceImage,
    span: Span,
    config: LexerConfig,
) -> Option<Vec<Vec<NativeWord>>> {
    let plan = tcl_lexer::native_script_words_in(image.clone(), span, config).ok()?;
    plan.fatal_tail.is_none().then(|| {
        plan.commands
            .into_iter()
            .map(|command| command.words)
            .collect()
    })
}

fn generated_requirements(
    template: &SourceImage,
    config: LexerConfig,
    graph: &AdviceGraph,
    context: &ContextRegistry,
) -> Option<(ByteCommandSlot, Vec<GeneratedCommandRequirement>)> {
    // This purpose currently owns one unqualified root procedure and its flat
    // original body. Namespace constructors and nested script phases require
    // their own source correspondence before they can extend this inventory.
    let roots = original_plan(
        template,
        Span::new(0, u32::try_from(template.len()).ok()?),
        config,
    )?;
    let [root] = roots.as_slice() else {
        return None;
    };
    let mut pending = vec![(root.clone(), true)];
    let mut publication = None;
    let mut requirements = Vec::new();
    while let Some((native, declaration)) = pending.pop() {
        let written = original_words(&native, &graph.policy)?;
        let head = written.first()?.input.as_ref()?;
        let (descriptor, prefix, lineage) = graph.resolve(head)?;
        if !prefix.is_empty() || !lineage.is_empty() {
            return None;
        }
        let arguments = registry_words(written.get(1..)?);
        let resolution =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                context.commands(),
                Some(context.context()),
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal(&descriptor),
                    &arguments,
                )
                .with_dialect(graph.dialect),
                tcl_dialect::model::InvocationRealm::RuleLoader,
            );
        let schema = resolution.resolved()?;
        schema
            .facts()
            .arity_accepts_frozen_arguments()
            .filter(|accepts| *accepts)?;
        let (roles, complete) = schema.authored_logical_source_argument_roles();
        complete.then_some(())?;
        requirements.push(GeneratedCommandRequirement {
            slot: graph.lookup_head_key(head.bytes())?,
            descriptor: descriptor.clone(),
        });
        if declaration {
            let layout = schema.authored_source_procedure_arguments()?;
            if schema
                .semantics
                .body_interpreter
                .resolve(schema.words.arguments())
                != tcl_registry::world_effect::InterpreterScope::Current
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
            let name_argument = definitions.next()?.argument_index()?;
            (name_argument == layout.name).then_some(())?;
            let name = written.get(name_argument.checked_add(1)?)?.input.as_ref()?;
            definitions.next().is_none().then_some(())?;
            let slot = graph.publication_key(name.bytes(), AdvicePublicationPurpose::Define)?;
            (slot.namespace == ByteNamespacePath::root()).then_some(())?;
            publication = Some(slot);
            let mut bodies = roles
                .iter()
                .filter(|(_, role)| *role == tcl_registry::ArgRole::Body);
            let &(ordinal, _) = bodies.next()?;
            bodies.next().is_none().then_some(())?;
            let argument = schema
                .semantics
                .argument_offset
                .checked_add(usize::from(ordinal))?;
            (argument == layout.body).then_some(())?;
            let word = native.get(argument.checked_add(1)?)?;
            let body =
                crate::registry_invocation::source_scoped_body::original_written_script_body_span(
                    word,
                )?;
            pending.extend(
                original_plan(template, body, config)?
                    .into_iter()
                    .map(|words| (words, false)),
            );
        } else if roles.iter().any(|(_, role)| role.has_script_timing())
            || native.iter().any(|word| {
                word.executable_parts().all_parts().any(|part| {
                    matches!(
                        part.part,
                        tcl_lexer::ExecutablePart::Command { .. }
                            | tcl_lexer::ExecutablePart::Expression { .. }
                            | tcl_lexer::ExecutablePart::ParseError(_)
                    )
                })
            })
        {
            return None;
        }
    }
    Some((publication?, requirements))
}

fn generated_call_matches(
    call: &SourceImage,
    config: LexerConfig,
    publication: &ByteCommandSlot,
) -> Option<()> {
    let roots = original_plan(call, Span::new(0, u32::try_from(call.len()).ok()?), config)?;
    let [words] = roots.as_slice() else {
        return None;
    };
    let [word] = words.as_slice() else {
        return None;
    };
    let arena = word.executable_parts();
    let [part] = arena.list(arena.root()) else {
        return None;
    };
    let tcl_lexer::ExecutablePart::Command { body } = part.part else {
        return None;
    };
    let children = original_plan(call, body, config)?;
    let [words] = children.as_slice() else {
        return None;
    };
    let [head, _argument] = words.as_slice() else {
        return None;
    };
    let bytes = tcl_syntax::word_rules::original_static_word_unicode_value(head)?;
    (tcl_syntax::naming::authored_source_command_slot(&ByteNamespacePath::root(), &bytes).as_ref()
        == Some(publication))
    .then_some(())
}

impl OriginalSourceTransitionSeed {
    pub(crate) fn original_logical_publication_proposal(
        &self,
        input: &ResolvedAnalysisInput,
        template: &str,
        call: &str,
    ) -> Option<OriginalSourcePublicationProposal> {
        if !input.has_logical_source_name_context()
            || self.logical_input.as_ref() != Some(input)
            || self.baseline.logical_source_input.as_ref() != Some(input)
            || self.baseline.metadata_context.source_analysis_input() != Some(input)
            || self.baseline.native_entry.is_some()
            || self.baseline.hosted_execution_context.is_some()
            || self.baseline.vendor_source_input.is_some()
            || self.baseline.unknown_entry
        {
            return None;
        }
        let context = input.borrowed_context_registry();
        let prepared = self
            .prepare_source_advice_tape(context, OriginalSourceCataloguePurpose::CompleteProgram)?;
        let graph = self.initial_source_advice_graph(context, &prepared.policy)?;
        matches!(prepared.policy, AdviceNamingPolicy::Logical(_)).then_some(())?;
        let template = SourceImage::document(template);
        let call = SourceImage::document(call);
        let (publication, commands) =
            generated_requirements(&template, self.config, &graph, context)?;
        generated_call_matches(&call, self.config, &publication)?;
        let admitted = Cell::new(true);
        let query = PublicationProposalQuery {
            publication: &publication,
            commands: &commands,
            admitted: &admitted,
        };
        self.prepare_for_query(
            context,
            OriginalSourceCataloguePurpose::CompleteProgram,
            Some(SourceAdviceQuery::Publication(query)),
        )?;
        admitted.get().then(|| OriginalSourcePublicationProposal {
            image: self.origin.source_image().clone(),
            input: input.clone(),
            template,
            call,
            publication,
        })
    }
}

impl crate::command_binding::SourceCommandBindings {
    pub(crate) fn original_logical_publication_proposal(
        &self,
        input: &ResolvedAnalysisInput,
        template: &str,
        call: &str,
    ) -> Option<OriginalSourcePublicationProposal> {
        self.original_source_transition_seed()?
            .original_logical_publication_proposal(input, template, call)
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
    use std::sync::Arc;

    const TEMPLATE: &str = "proc helper {value} {string length $value}";
    const CALL: &str = "[::helper $value]";

    fn analyse(source: &str) -> AnalysisResult {
        let registry =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let point =
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_79);
        let profile = tcl_dialect::DialectProfile::projected_from_point(
            "logical-publication-proposal",
            &[],
            "Logical source publication proposal",
            point,
        )
        .intern();
        let context = tcl_registry::model::ingress::context_for_profile(profile)
            .with_command_store(registry.commands().snapshot().shared_registry());
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::new(context),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    #[test]
    fn logical_publication_proposals_retain_timeline_occupants_and_constructor_selection() {
        for source in ["", "namespace eval N {proc helper {value} {return $value}}"] {
            let analysis = analyse(source);
            let proposal = crate::source_graph::original_logical_publication_proposal(
                source, &analysis, TEMPLATE, CALL,
            )
            .expect("complete conditional root source proposal");
            assert_eq!(proposal.template().bytes(), TEMPLATE.as_bytes());
            assert_eq!(proposal.call().bytes(), CALL.as_bytes());
            assert!(proposal.matches_source(
                &tcl_lexer::SourceImage::document(source),
                analysis.resolved_input.as_ref().unwrap(),
            ));
            assert!(
                crate::source_graph::original_logical_publication_proposal(
                    &format!("#{source}"),
                    &analysis,
                    TEMPLATE,
                    CALL,
                )
                .is_none()
            );
        }
        for source in [
            "proc helper {value} {return $value}",
            "interp alias {} helper {} string length",
            "proc helper {} {}; rename helper {}",
            "namespace eval N {proc ::helper {} {}; rename ::helper {}}",
            "namespace eval N {proc ::helper {} {}}; namespace delete N",
            "unknown_mutator",
            "proc string args {}",
            "rename string oldstring; rename oldstring string",
            "interp alias {} string {} list",
            "proc proc args {}",
        ] {
            let analysis = analyse(source);
            assert!(
                crate::source_graph::original_logical_publication_proposal(
                    source, &analysis, TEMPLATE, CALL,
                )
                .is_none(),
                "{source}"
            );
        }
        let analysis = analyse("");
        for (template, call) in [
            (
                "proc puts {value} {string length $value}",
                "[::puts $value]",
            ),
            (TEMPLATE, "[::other $value]"),
            (TEMPLATE, "[::helper $value] tail"),
            (TEMPLATE, "[::helper $value $other]"),
            ("proc helper {value} {string length [unknown]}", CALL),
            ("proc helper {value} {if 1 {string length $value}}", CALL),
            (
                "namespace eval N {}; proc N::helper {value} {return $value}",
                "[::N::helper $value]",
            ),
            (
                "proc helper {value} {string length $value}",
                "[::helper $value",
            ),
        ] {
            assert!(
                crate::source_graph::original_logical_publication_proposal(
                    "", &analysis, template, call,
                )
                .is_none(),
                "{template}: {call}"
            );
        }
        let mut missing = analysis;
        missing.resolved_input = None;
        assert!(
            crate::source_graph::original_logical_publication_proposal(
                "", &missing, TEMPLATE, CALL,
            )
            .is_none()
        );
    }

    #[test]
    fn logical_publication_proposals_keep_deferred_source_hazards_terminal() {
        for source in ["proc later {} {}", "proc later {} {return literal}"] {
            let analysis = analyse(source);
            assert!(
                crate::source_graph::original_logical_publication_proposal(
                    source, &analysis, TEMPLATE, CALL,
                )
                .is_some(),
                "{source}: complete retained deferred source"
            );
        }
        for source in [
            "proc later {} {proc ::helper {} {}}",
            "proc later {} {proc ::helper {} {}; rename ::helper {}}",
            "proc later {} {unknown_mutator}",
            "proc later {} $unavailable_body",
            "proc outer {} {proc later {} $unavailable_body}",
            "proc outer {} {uplevel 2 {proc ::helper {} {}}}",
            "proc outer {} {expr {[proc ::helper {} {}]}}",
        ] {
            let analysis = analyse(source);
            assert!(
                crate::source_graph::original_logical_publication_proposal(
                    source, &analysis, TEMPLATE, CALL,
                )
                .is_none(),
                "{source}"
            );
        }
    }
}
