// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-read schemas from the shared original body/transition catalogue.

use super::source_transition_advice::{
    OriginalSourceCataloguePurpose, OriginalSourceTransitionAdviceLookup,
    OriginalSourceTransitionAdviceTape, OriginalSourceTransitionSeed,
};
use super::{CommandAllocationSite, OriginalSourceCommandTransitionAdvice};
use std::sync::{Arc, Mutex};
use tcl_registry::model::ContextRegistry;

type CatalogueMemoEntry = (
    OriginalSourceTransitionSeed,
    Option<Arc<SourceReadCatalogue>>,
);
#[derive(Debug, Default)]
pub(super) struct SourceReadCatalogueMemo(Mutex<Vec<CatalogueMemoEntry>>);
impl SourceReadCatalogueMemo {
    pub(super) fn get_or_prepare(
        &self,
        seed: &OriginalSourceTransitionSeed,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Arc<SourceReadCatalogue>> {
        seed.matches_registry(registry).then_some(())?;
        let find = |entries: &[CatalogueMemoEntry]| {
            entries
                .iter()
                .find(|(owned, _)| owned == seed)
                .map(|(_, value)| value.clone())
        };
        if let Some(value) = find(
            &self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        ) {
            return value;
        }
        let prepared = SourceReadCatalogue::prepare(seed, registry).map(Arc::new);
        let mut entries = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(value) = find(&entries) {
            return value;
        }
        if entries.len() < 4 {
            entries.push((seed.clone(), prepared.clone()));
        }
        prepared
    }
}

#[derive(Debug)]
pub(super) struct SourceReadCatalogue {
    context: Arc<ContextRegistry>,
    tape: OriginalSourceTransitionAdviceTape,
}
impl SourceReadCatalogue {
    pub(super) fn prepare(
        seed: &OriginalSourceTransitionSeed,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Self> {
        let context = seed.source_context(registry)?;
        let tape = seed.prepare_for(&context, OriginalSourceCataloguePurpose::CompletePrefix)?;
        Some(Self { context, tape })
    }

    /// A retained Logical body receipt owns its declared frame; the separate
    /// syntax catalogue deliberately carries no such interpretation. Prefer
    /// that existing receipt only when both original vectors and inputs agree.
    fn read_advice(&self, offset: u32) -> Option<&OriginalSourceCommandTransitionAdvice> {
        let current = match self.tape.lookup(offset) {
            OriginalSourceTransitionAdviceLookup::Advice(advice) => Some(advice),
            OriginalSourceTransitionAdviceLookup::Refused
            | OriginalSourceTransitionAdviceLookup::Unrepresented => None,
        };
        let catalogue = self.tape.body_catalogue(offset);
        if let Some(owned) = current.filter(|advice| advice.logical_source_scope().is_some()) {
            let input = owned.logical_source_input()?;
            if !input.has_logical_source_name_context()
                || catalogue.is_some_and(|syntax| {
                    syntax.site() != owned.site()
                        || syntax.original_words() != owned.original_words()
                        || syntax.logical_source_input() != Some(input)
                })
            {
                return None;
            }
            return Some(owned);
        }
        catalogue.or(current)
    }

    /// A conditional source schema inside its genuine declared body. The body
    /// and exact argv do not supply a reached lookup, frame or current store.
    pub(super) fn scope(
        &self,
        site: &CommandAllocationSite,
        tokens: &crate::ir::CommandTokens,
        body: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SourceReadScope> {
        if tokens.synthetic.is_some() || self.tape.blocks_registry_source(site.offset) {
            return None;
        }
        let advice = self.read_advice(site.offset)?;
        if advice.site() != site
            || !advice.matches_source(site.source.source_image(), config)
            || !advice.matches_context(&self.context)
        {
            return None;
        }
        let parent_matches = matches_read_body_ancestry(advice, body, &self.context, config);
        if !parent_matches {
            return None;
        }
        let original = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            config,
        )?;
        if original.as_slice() != advice.original_words() {
            return None;
        }
        if let Some(binding) = &tokens.source_binding {
            if binding.lookup_state.is_some() {
                binding
                    .original_invocation_metadata_at_point(tokens, self.context.commands())
                    .ok()?;
            }
            if !source_targets_match_advice(&self.context, binding, advice) {
                return None;
            }
        }
        Some(SourceReadScope {
            context: Arc::clone(&self.context),
            advice: advice.clone(),
            config,
        })
    }
}

fn inherits_read_body_context(
    current: &crate::registry_invocation::OriginalSourceScriptBody,
    context: &ContextRegistry,
) -> bool {
    let words = current.source_words();
    if words.with_source_schema(context, |schema| {
        schema.semantics.body_kind == tcl_registry::BodyKind::Plain
            && schema.semantics.body_interpreter == tcl_registry::BodyInterpreter::Current
            && schema.semantics.frame_effect.is_none()
            && !schema
                .semantics
                .traits
                .intersects(tcl_registry::FRAME_REACH_TRAITS | tcl_registry::Traits::DEFERS_BODY)
    }) != Some(true)
    {
        return false;
    }
    if current.inherits_immediate_caller_source_context(context)
        && words.source_script_bodies(context).contains(current)
    {
        return true;
    }
    words
        .normal_source_case_bodies(context)
        .is_some_and(|cases| {
            cases
                .bodies()
                .is_some_and(|bodies| bodies.contains(current))
        })
}

/// Original script ancestry for this conditional read question only. Every
/// intermediate body retains its selected schema and its immediate parent;
/// lexical containment alone cannot cross a namespace or frame boundary.
fn matches_read_body_ancestry(
    mut advice: &OriginalSourceCommandTransitionAdvice,
    body: tcl_lexer::Span,
    context: &ContextRegistry,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let image = advice.site().source.source_image();
    loop {
        let current = if let Some(parent) = advice.original_catalogue_source_body() {
            if !parent.matches_context(context) || !parent.matches_source(image, config) {
                return false;
            }
            if parent.content_span() == body {
                return true;
            }
            let super::source_transition_advice::OriginalSourceCatalogueBody::Script(current) =
                parent
            else {
                return false;
            };
            current.as_ref()
        } else {
            let Some(parent) = advice
                .original_procedure_source_body()
                .or_else(|| advice.logical_source_body())
            else {
                return false;
            };
            if !parent.matches_context(context) || !parent.matches_source(image, config) {
                return false;
            }
            if parent.content_span() == body {
                return true;
            }
            parent
        };
        if !inherits_read_body_context(current, context) {
            return false;
        }
        let words = current.source_words();
        let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
            parent,
        ) = words.source()
        else {
            return false;
        };
        if !parent.matches_context(context) || !parent.matches_source(image, config) {
            return false;
        }
        let next = parent
            .original_catalogue_source_body()
            .map(super::source_transition_advice::OriginalSourceCatalogueBody::content_span)
            .or_else(|| {
                parent
                    .original_procedure_source_body()
                    .or_else(|| parent.logical_source_body())
                    .map(crate::registry_invocation::OriginalSourceScriptBody::content_span)
            });
        // The private parent receipts establish ancestry. Strictly decreasing
        // offsets only bound this walk without a new depth threshold.
        if next.is_none_or(|next| next.start() >= current.content_span().start()) {
            return false;
        }
        advice = parent;
    }
}

/// An original source-reading purpose, distinct from invocation/compiler entry.
pub(crate) struct SourceReadScope {
    context: Arc<ContextRegistry>,
    advice: OriginalSourceCommandTransitionAdvice,
    config: tcl_lexer::LexerConfig,
}
impl SourceReadScope {
    pub(crate) fn context(&self) -> &ContextRegistry {
        &self.context
    }
    pub(crate) fn advice(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.advice
    }
    pub(crate) const fn config(&self) -> tcl_lexer::LexerConfig {
        self.config
    }
}

fn logical_prefix_inputs_match(
    target: &super::SourceCommandTarget,
    source: &crate::analyser::ResolvedAnalysisInput,
    advice: &OriginalSourceCommandTransitionAdvice,
    prefix_len: usize,
) -> bool {
    let Some(inputs) = target.original_logical_prepended_source_inputs(
        source,
        advice.site().source.source_image(),
        source.lexer_config(),
    ) else {
        return false;
    };
    let captured = advice.arguments().iter().filter(|word| {
        matches!(
            word.origin,
            crate::registry_invocation::InvocationWordOrigin::BindingPrefix(_)
        )
    });
    if inputs.len() != prefix_len
        || inputs
            .iter()
            .zip(captured)
            .any(|(input, word)| match word.input.as_ref() {
                Some(crate::command_binding::SourceAdviceNameInput::Logical(original)) => {
                    original != input
                }
                _ => true,
            })
    {
        return false;
    }
    true
}

/// Conditional original source advice cannot override an actually selected target.
pub(crate) fn source_targets_match_advice(
    context: &ContextRegistry,
    binding: &crate::command_binding::SourceInvocationBinding,
    advice: &OriginalSourceCommandTransitionAdvice,
) -> bool {
    if binding.lookup_state.is_some() && !binding.unknown && binding.targets.is_empty() {
        return false;
    }
    let prefix = advice
        .arguments()
        .iter()
        .filter(|word| {
            matches!(
                word.origin,
                crate::registry_invocation::InvocationWordOrigin::BindingPrefix(_)
            )
        })
        .map(|word| word.value.as_deref())
        .collect::<Option<Vec<_>>>();
    let Some(prefix) = prefix else {
        return false;
    };
    let Some(descriptor) = context
        .context()
        .resolve_spec(context.commands(), advice.command())
    else {
        return false;
    };
    for target in &binding.targets {
        if !target.registry_backed
            || !context
                .context()
                .resolve_spec(context.commands(), &target.command)
                .is_some_and(|selected| std::ptr::eq(selected, descriptor))
            || target
                .prepended
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::literal_bytes)
                .collect::<Option<Vec<_>>>()
                .as_deref()
                != Some(prefix.as_slice())
        {
            return false;
        }
        if !prefix.is_empty()
            && let Some(source) = advice.logical_source_input()
        {
            if !logical_prefix_inputs_match(target, source, advice, prefix.len()) {
                return false;
            }
            continue;
        }
        if !prefix.is_empty() {
            let inputs = if let Some(input) = advice.logical_source_input() {
                target.original_logical_prepended_name_inputs(
                    input,
                    advice.site().source.source_image(),
                    input.lexer_config(),
                )
            } else {
                target.original_prepended_name_inputs()
            };
            let Some(inputs) = inputs else {
                return false;
            };
            let captured = advice.arguments().iter().filter(|word| {
                matches!(
                    word.origin,
                    crate::registry_invocation::InvocationWordOrigin::BindingPrefix(_)
                )
            });
            if inputs.len() != prefix.len()
                || inputs.iter().zip(captured).any(|(input, word)| {
                    let Some(captured) = word.input.as_ref() else {
                        return true;
                    };
                    match captured {
                        crate::command_binding::SourceAdviceNameInput::Native(original) => {
                            original != input
                        }
                        crate::command_binding::SourceAdviceNameInput::Logical(original) => {
                            advice.logical_source_input().is_none_or(|source| {
                                [source.analyser_profile(), source.unit_profile()]
                                    .into_iter()
                                    .any(|profile| {
                                        tcl_registry::InvocationDialect::of_profile(profile)
                                            .authored_name_policy()
                                            != Some(input.policy())
                                    })
                            }) || input.policy().authority()
                                != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
                                || input.original_word_key().is_none_or(|key| {
                                    key.original_word() != original.original_word()
                                        || key.bytes() != original.bytes()
                                })
                        }
                    }
                })
            {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod read_body_ancestry_tests {
    use super::{SourceReadCatalogue, matches_read_body_ancestry};
    use crate::command_binding::{OriginalSourceCommandTransitionAdvice, SourceCommandBindings};
    use tcl_lexer::{LexerConfig, Span};

    fn catalogue(source: &str) -> (SourceReadCatalogue, LexerConfig) {
        let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let config = LexerConfig::for_profile(context.commands().profile());
        let bindings = SourceCommandBindings::analyse(source, config, context.commands());
        let seed = bindings.original_source_transition_seed().unwrap();
        (
            SourceReadCatalogue::prepare(&seed, context.commands()).unwrap(),
            config,
        )
    }

    fn advice(
        catalogue: &SourceReadCatalogue,
        offset: u32,
    ) -> &OriginalSourceCommandTransitionAdvice {
        catalogue.tape.body_catalogue(offset).or_else(|| {
            if let crate::command_binding::source_transition_advice::OriginalSourceTransitionAdviceLookup::Advice(advice) = catalogue.tape.lookup(offset) {
                Some(advice)
            } else {
                None
            }
        }).unwrap()
    }

    #[test]
    fn nested_plain_and_normal_case_bodies_retain_exact_procedure_ancestry() {
        let source = "proc p {x} {foreach i {1 2 3} {switch -glob $x {a* {break} default {break}}}; puts $missing}";
        let (catalogue, config) = catalogue(source);
        let root = Span::new(
            u32::try_from(source.find("foreach").unwrap()).unwrap(),
            u32::try_from(source.len() - 1).unwrap(),
        );
        for head in ["switch", "break"] {
            let child = advice(
                &catalogue,
                u32::try_from(source.find(head).unwrap()).unwrap(),
            );
            let immediate = child.original_catalogue_source_body().unwrap();
            assert_ne!(immediate.content_span(), root);
            assert!(matches_read_body_ancestry(
                child,
                root,
                &catalogue.context,
                config
            ));
            assert!(!matches_read_body_ancestry(
                child,
                Span::new(root.start() + 1, root.end()),
                &catalogue.context,
                config
            ));
            let foreign = tcl_registry::model::ingress::static_context_for("tcl8.5");
            assert!(!matches_read_body_ancestry(child, root, foreign, config));
        }
    }

    #[test]
    fn original_namespace_and_frame_boundaries_do_not_inherit_read_scope() {
        for nested in [
            "namespace eval n {puts $missing}",
            "uplevel 0 {puts $missing}",
        ] {
            let source = format!("proc p {{}} {{{nested}}}");
            let (catalogue, config) = catalogue(&source);
            let root = Span::new(
                u32::try_from(source.find(nested).unwrap()).unwrap(),
                u32::try_from(source.len() - 1).unwrap(),
            );
            let child = advice(
                &catalogue,
                u32::try_from(source.find("puts").unwrap()).unwrap(),
            );
            assert_ne!(
                child
                    .original_catalogue_source_body()
                    .unwrap()
                    .content_span(),
                root
            );
            assert!(!matches_read_body_ancestry(
                child,
                root,
                &catalogue.context,
                config
            ));
        }
    }

    #[test]
    fn exact_original_parent_read_scope_remains_available() {
        let source = "proc p {} {puts $missing}";
        let (catalogue, config) = catalogue(source);
        let root = Span::new(
            u32::try_from(source.find("puts").unwrap()).unwrap(),
            u32::try_from(source.len() - 1).unwrap(),
        );
        let child = advice(&catalogue, root.start());
        assert!(matches_read_body_ancestry(
            child,
            root,
            &catalogue.context,
            config
        ));
    }
}
