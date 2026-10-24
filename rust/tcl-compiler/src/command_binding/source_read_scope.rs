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

pub(super) fn trace_rbs_refusal(offset: u32, stage: &'static str) {
    if cfg!(debug_assertions) && std::env::var_os("TCL_LSP_TRACE_RBS_SOURCE_OWNER").is_some() {
        eprintln!("RBS_SOURCE_READ offset={offset} stage={stage} passed=false");
    }
}

pub(super) fn trace_rbs_required<T>(
    value: Option<T>,
    offset: u32,
    stage: &'static str,
) -> Option<T> {
    if value.is_none() {
        trace_rbs_refusal(offset, stage);
    }
    value
}

fn trace_rbs_condition(value: bool, offset: u32, stage: &'static str) -> bool {
    if !value {
        trace_rbs_refusal(offset, stage);
    }
    value
}

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
            trace_rbs_refusal(site.offset, "scope.synthetic-or-barrier");
            return None;
        }
        let advice = if let Some(advice) = self.tape.body_catalogue(site.offset) {
            advice
        } else {
            match self.tape.lookup(site.offset) {
                OriginalSourceTransitionAdviceLookup::Advice(advice) => advice,
                OriginalSourceTransitionAdviceLookup::Refused
                | OriginalSourceTransitionAdviceLookup::Unrepresented => {
                    trace_rbs_refusal(site.offset, "scope.catalogue-or-advice");
                    return None;
                }
            }
        };
        if advice.site() != site
            || !advice.matches_source(site.source.source_image(), config)
            || !advice.matches_context(&self.context)
        {
            trace_rbs_refusal(site.offset, "scope.advice-current");
            return None;
        }
        let parent_matches = if let Some(parent) = advice.original_catalogue_source_body() {
            trace_rbs_condition(
                parent.content_span() == body,
                site.offset,
                "scope.catalogue-parent-span",
            ) && parent.matches_context(&self.context)
                && parent.matches_source(site.source.source_image(), config)
        } else {
            let parent = trace_rbs_required(
                advice
                    .original_procedure_source_body()
                    .or_else(|| advice.logical_source_body()),
                site.offset,
                "scope.parent-owner",
            )?;
            trace_rbs_condition(
                parent.content_span() == body,
                site.offset,
                "scope.procedure-parent-span",
            ) && parent.matches_context(&self.context)
                && parent.matches_source(site.source.source_image(), config)
        };
        if !parent_matches {
            trace_rbs_refusal(site.offset, "scope.parent-body");
            return None;
        }
        let original = trace_rbs_required(
            crate::registry_invocation::original_native_compiler_words(
                site.source.source_image(),
                tokens.words(),
                site.offset,
                config,
            ),
            site.offset,
            "scope.original-words",
        )?;
        if original.as_slice() != advice.original_words() {
            trace_rbs_refusal(site.offset, "scope.original-vector");
            return None;
        }
        if let Some(binding) = &tokens.source_binding {
            if binding.lookup_state.is_some() {
                trace_rbs_required(
                    binding
                        .original_invocation_metadata_at_point(tokens, self.context.commands())
                        .ok(),
                    site.offset,
                    "scope.point-metadata",
                )?;
            }
            if !source_targets_match_advice(&self.context, binding, advice) {
                trace_rbs_refusal(site.offset, "scope.selected-targets");
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
