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
        let advice = if let Some(advice) = self.tape.body_catalogue(site.offset) {
            advice
        } else {
            match self.tape.lookup(site.offset) {
                OriginalSourceTransitionAdviceLookup::Advice(advice) => advice,
                OriginalSourceTransitionAdviceLookup::Refused
                | OriginalSourceTransitionAdviceLookup::Unrepresented => return None,
            }
        };
        if advice.site() != site
            || !advice.matches_source(site.source.source_image(), config)
            || !advice.matches_context(&self.context)
        {
            return None;
        }
        let parent_matches = if let Some(parent) = advice.original_catalogue_source_body() {
            parent.content_span() == body
                && parent.matches_context(&self.context)
                && parent.matches_source(site.source.source_image(), config)
        } else {
            let parent = advice
                .original_procedure_source_body()
                .or_else(|| advice.logical_source_body())?;
            parent.content_span() == body
                && parent.matches_context(&self.context)
                && parent.matches_source(site.source.source_image(), config)
        };
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
        if !prefix.is_empty() {
            let Some(inputs) = target.original_prepended_name_inputs() else {
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
                    word.input
                        .as_ref()
                        .and_then(crate::command_binding::SourceAdviceNameInput::native_input)
                        != Some(input)
                })
            {
                return false;
            }
        }
    }
    true
}
