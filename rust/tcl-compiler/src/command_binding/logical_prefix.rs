// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original captured words in the independently retained Logical alias model.

use super::{
    Arc, BTreeMap, CommandAllocationSite, ModuleCommandBindings, ResolvedCommandTarget,
    SourceOriginId,
};
use crate::analyser::ResolvedAnalysisInput;
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct LogicalAliasPrefix {
    pub(super) input: ResolvedAnalysisInput,
    pub(super) site: CommandAllocationSite,
    pub(super) words: Vec<NativeWord>,
    pub(super) arguments: Option<Vec<SignatureSourceNameInput>>,
    pub(super) source_arguments: Vec<super::OriginalLogicalSourceNameInput>,
    pub(super) parent: Option<LogicalCapturedCommandPrefix>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct LogicalCapturedCommandPrefix {
    input: ResolvedAnalysisInput,
    source: Arc<SourceOriginId>,
    inputs: Option<Vec<SignatureSourceNameInput>>,
    source_inputs: Vec<super::OriginalLogicalSourceNameInput>,
    recipes: Vec<Arc<LogicalAliasPrefix>>,
}

impl LogicalCapturedCommandPrefix {
    pub(super) fn empty(state: &ModuleCommandBindings) -> Option<Self> {
        let input = state.logical_source_name_advice_input()?;
        if !input.has_logical_source_name_context()
            || state.baseline.native_entry.is_some()
            || state.baseline.unknown_entry
            || state.baseline.execution_name_policy.is_some()
            || state.baseline.hosted_execution_context.is_some()
            || state.has_opaque_domain()
            || !state.command_observers.is_quiet()
        {
            return None;
        }
        Some(Self {
            input: input.clone(),
            source: Arc::clone(state.current_source_origin.as_ref()?),
            inputs: Some(Vec::new()),
            source_inputs: Vec::new(),
            recipes: Vec::new(),
        })
    }

    pub(super) fn from_alias(
        state: &ModuleCommandBindings,
        target: &ResolvedCommandTarget,
    ) -> Option<Self> {
        let mut prefix = Self::empty(state)?;
        let receipt = state.logical_alias_prefixes.get(target)?.as_ref()?;
        if receipt.input != prefix.input
            || receipt.site.source != prefix.source
            || receipt.words.first()?.span().start() != receipt.site.offset
            || receipt.words.iter().any(|word| {
                word.image() != prefix.source.source_image()
                    || word.config() != receipt.input.lexer_config()
            })
            || receipt.source_arguments.len() != target.prepended.len()
            || receipt
                .source_arguments
                .iter()
                .zip(&target.prepended)
                .any(|(input, value)| value.literal_bytes() != Some(input.bytes()))
        {
            return None;
        }
        prefix.inputs.clone_from(&receipt.arguments);
        prefix.source_inputs.clone_from(&receipt.source_arguments);
        prefix.recipes.push(Arc::clone(receipt));
        Some(prefix)
    }

    pub(super) fn append(&mut self, other: &Self) -> Option<()> {
        if self.input != other.input || self.source != other.source {
            return None;
        }
        self.inputs = match (self.inputs.take(), other.inputs.as_ref()) {
            (Some(mut own), Some(other)) => {
                own.extend(other.iter().cloned());
                Some(own)
            }
            _ => None,
        };
        self.source_inputs
            .extend(other.source_inputs.iter().cloned());
        self.recipes.extend(other.recipes.iter().cloned());
        Some(())
    }

    pub(super) fn matches_state(&self, state: &ModuleCommandBindings) -> bool {
        Self::empty(state)
            .is_some_and(|current| self.input == current.input && self.source == current.source)
    }

    pub(super) fn inputs(&self) -> Option<&[SignatureSourceNameInput]> {
        self.inputs.as_deref()
    }

    pub(super) fn source_inputs(&self) -> &[super::OriginalLogicalSourceNameInput] {
        &self.source_inputs
    }

    pub(super) fn matches_source_words(
        &self,
        input: &ResolvedAnalysisInput,
        image: &SourceImage,
        config: LexerConfig,
    ) -> bool {
        input.has_logical_source_name_context()
            && &self.input == input
            && config == input.lexer_config()
            && self.source.source_image() == image
            && self
                .recipes
                .iter()
                .flat_map(|receipt| receipt.source_arguments.iter())
                .eq(self.source_inputs.iter())
            && self.source_inputs.iter().all(|source| {
                source.original_word().image() == image
                    && source.original_word().config() == config
                    && super::OriginalLogicalSourceNameInput::from_original_source_word(
                        source.original_word(),
                        input,
                    )
                    .as_ref()
                        == Some(source)
            })
            && self.recipes.iter().all(|receipt| {
                receipt.input == *input
                    && receipt.site.source == self.source
                    && receipt
                        .words
                        .first()
                        .is_some_and(|word| word.span().start() == receipt.site.offset)
                    && receipt
                        .words
                        .iter()
                        .all(|word| word.image() == image && word.config() == config)
                    && receipt.source_arguments.iter().all(|source| {
                        receipt.words.contains(source.original_word())
                            || receipt
                                .parent
                                .as_ref()
                                .is_some_and(|parent| parent.source_inputs.contains(source))
                    })
                    && receipt
                        .parent
                        .as_ref()
                        .is_none_or(|parent| parent.matches_source_words(input, image, config))
            })
    }

    pub(super) fn matches_source_input(
        &self,
        input: &ResolvedAnalysisInput,
        image: &SourceImage,
        config: LexerConfig,
    ) -> bool {
        let Some(inputs) = self.inputs.as_ref() else {
            return false;
        };
        if inputs.len() != self.source_inputs.len()
            || inputs.iter().zip(&self.source_inputs).any(|(key, source)| {
                key.bytes() != source.bytes()
                    || key
                        .original_word_key()
                        .is_none_or(|key| key.original_word() != source.original_word())
            })
            || !self.matches_source_words(input, image, config)
        {
            return false;
        }
        let Some(policy) = tcl_registry::InvocationDialect::of_profile(input.analyser_profile())
            .authored_name_policy()
        else {
            return false;
        };
        input.has_logical_source_name_context()
            && tcl_registry::InvocationDialect::of_profile(input.unit_profile()).authored_name_policy() == Some(policy)
            && inputs.iter().all(|input| {
                let Some(key) = input.original_word_key() else { return false; };
                let word = key.original_word();
                word.image() == image && word.config() == config
                    && crate::signature_scan::scope::SignatureSourceNameKey::from_original_logical_source_word(
                        word, tcl_syntax::word_rules::WordValueRules::from_config(&config), policy,
                    ).as_ref() == Some(key)
            })
            && &self.input == input
            && config == input.lexer_config()
            && self.source.source_image() == image
            && self.recipes.iter().all(|receipt| {
                receipt.input == *input
                    && receipt.site.source == self.source
                    && receipt
                        .words
                        .iter()
                        .all(|word| word.image() == image && word.config() == config)
                    && receipt
                        .parent
                        .as_ref()
                        .is_none_or(|parent| parent.matches_source_input(input, image, config))
            })
    }
}

#[derive(Default)]
pub(super) struct LogicalPrefixResolution {
    pub(super) targets: super::BTreeSet<ResolvedCommandTarget>,
    pub(super) prefixes: BTreeMap<ResolvedCommandTarget, Option<LogicalCapturedCommandPrefix>>,
}

impl LogicalPrefixResolution {
    pub(super) fn insert(
        &mut self,
        target: ResolvedCommandTarget,
        prefix: Option<LogicalCapturedCommandPrefix>,
    ) {
        if self.targets.contains(&target) {
            if self.prefixes.get(&target) != Some(&prefix) {
                self.prefixes.insert(target.clone(), None);
            }
        } else if prefix.is_some() {
            self.prefixes.insert(target.clone(), prefix);
        }
        self.targets.insert(target);
    }
}

impl ModuleCommandBindings {
    pub(super) fn join_logical_alias_prefixes(&mut self, other: &Self) -> bool {
        if self.logical_alias_prefixes == other.logical_alias_prefixes {
            return false;
        }
        let keys = self
            .logical_alias_prefixes
            .keys()
            .chain(other.logical_alias_prefixes.keys())
            .cloned()
            .collect::<super::BTreeSet<_>>();
        let mut joined = self.logical_alias_prefixes.as_ref().clone();
        for target in keys {
            let left = self.logical_alias_prefixes.get(&target);
            let right = other.logical_alias_prefixes.get(&target);
            let receipt = match (left, right) {
                (Some(left), Some(right)) if left == right => left.clone(),
                (Some(left), None) if !other.has_logical_alias_prototype(&target) => left.clone(),
                (None, Some(right)) if !self.has_logical_alias_prototype(&target) => right.clone(),
                _ => None,
            };
            joined.insert(target, receipt);
        }
        if joined == *self.logical_alias_prefixes {
            return false;
        }
        self.logical_alias_prefixes = Arc::new(joined);
        true
    }

    fn has_logical_alias_prototype(&self, target: &ResolvedCommandTarget) -> bool {
        self.bindings
            .iter()
            .map(|(_, bindings)| bindings)
            .chain(self.objects.values())
            .any(|bindings| bindings.contains(&super::MayBinding::Target(target.clone())))
    }
}

impl ModuleCommandBindings {
    pub(super) fn compose_logical_alias_prefix(
        &self,
        target: &ResolvedCommandTarget,
        child: Option<LogicalCapturedCommandPrefix>,
    ) -> Option<LogicalCapturedCommandPrefix> {
        let mut prefix = child?;
        prefix.append(&LogicalCapturedCommandPrefix::from_alias(self, target)?)?;
        Some(prefix)
    }
}
