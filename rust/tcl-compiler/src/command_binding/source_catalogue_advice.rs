// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional catalogue coordinates retain their own source and uncertainty.

use super::{CommandAllocationSite, DeclaredCommandCandidate, SourceInvocationBinding};
use crate::ir::CommandTokens;
use crate::signature_scan::original_name::SourceOriginalNameOccurrence;
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use tcl_core_types::ByteCommandSlot;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};

/// Unclosed applicability of a conditional Registry source schema.
/// These obligations never become a selected runtime handler or variable cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginalCatalogueSourceObligation {
    /// Current command-table contents are not closed at the original point.
    UnknownCurrentLookup,
    /// A retained deferred source body has no observed future entry or lookup.
    UnknownFutureEntry,
    /// An independent receiver-body declaration supplies conditional source roles.
    ConditionalReceiverSource,
    /// The independently retained source scope lacks an executing holder join.
    UnknownExecutingNamespace,
    /// The source coordinate path is conditional on unavailable lookup state.
    UnknownLookupCoordinates,
    /// A candidate holder has no independently retained current incarnation.
    UnretainedHolder(ByteCommandSlot),
    /// An earlier cell can stop lookup before this nominal schema candidate.
    EarlierBindingAlternative(ByteCommandSlot),
    /// The queried cell has unknown or non-Registry alternatives.
    SelectedBindingAlternative(ByteCommandSlot),
}

/// A source-owned catalogue candidate, not a lookup result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalCatalogueSourceCandidate {
    head: SourceOriginalNameOccurrence,
    original: Vec<NativeWord>,
    namespace: SignatureNamespaceScope,
    candidate: DeclaredCommandCandidate,
    paths: Vec<Vec<ByteCommandSlot>>,
    obligations: Vec<OriginalCatalogueSourceObligation>,
    observations: Vec<super::declaration_layout::DeclarationLayoutObservation>,
    registry: RegistrySemanticKey,
}
impl OriginalCatalogueSourceCandidate {
    pub(crate) fn with_unknown_future_entry(mut self) -> Self {
        if !self
            .obligations
            .contains(&OriginalCatalogueSourceObligation::UnknownFutureEntry)
        {
            self.obligations
                .push(OriginalCatalogueSourceObligation::UnknownFutureEntry);
        }
        self
    }
    /// The genuine complete original source head.
    #[must_use]
    pub const fn original_head(&self) -> &SourceOriginalNameOccurrence {
        &self.head
    }
    /// Complete original vector under the head's source and grammar.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Independently selected source namespace, without a live holder grant.
    #[must_use]
    pub const fn original_namespace(&self) -> &SignatureNamespaceScope {
        &self.namespace
    }
    /// Exact consuming source position, without a reached invocation grant.
    #[must_use]
    pub fn site(&self) -> &CommandAllocationSite {
        self.head.site()
    }
    /// Fixed authored catalogue descriptor selected by counted slot geometry.
    #[must_use]
    pub const fn candidate(&self) -> &DeclaredCommandCandidate {
        &self.candidate
    }
    /// Conditional ordered byte paths; uncertainty is retained separately.
    #[must_use]
    pub fn paths(&self) -> &[Vec<ByteCommandSlot>] {
        &self.paths
    }
    /// Every retained unknown or alternative applicability obligation.
    #[must_use]
    pub fn obligations(&self) -> &[OriginalCatalogueSourceObligation] {
        &self.obligations
    }
    /// Complete source/channel and every lexer configuration axis agree.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.head.name_input().source_image() == image
            && self.head.name_input().lexer_config() == config
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == config)
    }
    /// Unanimous original source grammar from the same retained observations.
    /// This does not substitute for a physical compiler or handler policy.
    #[must_use]
    pub fn original_source_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        let first = self.observations.first()?.snapshot.state.baseline.dialect?;
        self.observations
            .iter()
            .all(|row| row.snapshot.state.baseline.dialect == Some(first))
            .then_some(first)
    }

    /// Same independently retained command-store semantics.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}

impl SourceInvocationBinding {
    /// Original declaration observations supply their own policy and namespace.
    /// A missing reached lookup cannot be repaired from a reporting label.
    pub(crate) fn original_catalogue_source_candidate_from_tokens(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<OriginalCatalogueSourceCandidate> {
        let rows = super::declaration_layout::original_source_catalogue_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let first = rows.clone().next()?;
        let policy = first
            .snapshot
            .state
            .baseline
            .execution_name_policy
            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)?;
        let namespace = first
            .snapshot
            .state
            .original_command_world
            .retained_scope_for_source_advice(&first.namespace, policy);
        let namespace = namespace?;
        if rows.clone().any(|row| {
            row.config != first.config
                || row
                    .snapshot
                    .state
                    .baseline
                    .execution_name_policy
                    .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                    != Some(policy)
                || row
                    .snapshot
                    .state
                    .original_command_world
                    .retained_scope_for_source_advice(&row.namespace, policy)
                    .as_ref()
                    != Some(&namespace)
        }) {
            return None;
        }
        let site = self.invocation_site()?;
        let original = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            first.config,
        )?;
        let key = crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
            original.first()?,
            tcl_syntax::word_rules::WordValueRules::from_config(&first.config),
            policy,
        )?;
        let head = SourceOriginalNameOccurrence::new(site, key)?;
        self.original_catalogue_source_candidate(tokens, &head, Some(&namespace), registry)
    }

    pub(crate) fn original_catalogue_source_candidate(
        &self,
        tokens: &CommandTokens,
        head: &SourceOriginalNameOccurrence,
        namespace: Option<&SignatureNamespaceScope>,
        registry: &CommandRegistry,
    ) -> Option<OriginalCatalogueSourceCandidate> {
        if tokens.source_binding.as_ref() != Some(self)
            || self.invocation_site() != Some(head.site())
        {
            return None;
        }
        let key = head.name_input();
        let input = SignatureSourceNameInput::OriginalWord(key.clone());
        let namespace = namespace.cloned().or_else(|| {
            let root = SignatureNamespaceScope::root(Some(key.policy()));
            let selected = key
                .policy()
                .recipe()
                .command_lookup_input(root.context()?, key.bytes())
                .ok()?;
            (selected.qualification() == tcl_syntax::naming::NativeNameQualification::Absolute)
                .then_some(root)
        })?;
        namespace.context_for_policy(key.policy())?;
        let original = crate::registry_invocation::original_native_compiler_words(
            key.source_image(),
            tokens.words(),
            head.site().offset,
            key.lexer_config(),
        )?;
        if original.first() != Some(key.original_word())
            || original.iter().any(|word| word.group().expand)
        {
            return None;
        }
        let rows = super::declaration_layout::original_source_catalogue_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let registry_key = registry.snapshot().semantic_key();
        let mut candidate = None;
        let mut paths = None;
        let mut obligations = Vec::new();
        let mut observations = Vec::new();
        for row in rows {
            if row.issuer
                == super::declaration_layout::DeclarationLayoutIssuer::ConditionalReceiverSource
                && !obligations
                    .contains(&OriginalCatalogueSourceObligation::ConditionalReceiverSource)
            {
                obligations.push(OriginalCatalogueSourceObligation::ConditionalReceiverSource);
            }
            if row.words.as_ref() != tokens.words()
                || row.config != key.lexer_config()
                || row.entry.source().origin.as_ref() != head.site().source.as_ref()
                || row.snapshot.state.baseline.registry_snapshot.as_ref() != Some(&registry_key)
                || row
                    .snapshot
                    .state
                    .baseline
                    .execution_name_policy
                    .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                    != Some(key.policy())
            {
                return None;
            }
            let (selected, selected_paths, residual) = row
                .snapshot
                .state
                .original_catalogue_source_candidate(&row.namespace, &namespace, &input)?;
            if candidate.as_ref().is_some_and(|prior| prior != &selected)
                || paths.as_ref().is_some_and(|prior| prior != &selected_paths)
            {
                return None;
            }
            candidate = Some(selected);
            paths = Some(selected_paths);
            for item in residual {
                if !obligations.contains(&item) {
                    obligations.push(item);
                }
            }
            observations.push(row.clone());
        }
        Some(OriginalCatalogueSourceCandidate {
            head: head.clone(),
            original,
            namespace,
            candidate: candidate?,
            paths: paths?,
            obligations,
            observations,
            registry: registry_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::OriginalCatalogueSourceObligation;
    use crate::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
    use crate::registry_invocation::source_structure::{
        OriginalRegistrySource, source_registry_words,
    };

    const SOURCE: &str = include_str!(
        "../../../tcl-registry/tests/data/source_contract_vectors/patterns/composed-callback-prefix/probes/positive.tcl"
    );

    fn analyse(source: &str, release: &str) -> AnalysisResult {
        let environment = tcl_registry::model::ingress::resolve_environment(release);
        let profile = environment.analyser_profile();
        let input = ResolvedAnalysisInput::new(
            profile,
            profile,
            environment.default_context_registry(),
            tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
        );
        Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name)
    }

    fn callback(source: &str, analysis: &AnalysisResult) -> crate::segmenter::SegmentedCommand {
        let offset = source.find("after idle $cb").unwrap();
        crate::segmenter::segment_commands_with_offset_and_config(
            &source[offset..offset + "after idle $cb".len()],
            u32::try_from(offset).unwrap(),
            analysis.resolved_input.as_ref().unwrap().lexer_config(),
        )
        .remove(0)
    }

    #[test]
    fn called_receiver_catalogue_uses_one_conditional_source_owner() {
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let analysis = analyse(SOURCE, release);
            let command = callback(SOURCE, &analysis);
            let words = source_registry_words(SOURCE, &analysis, &command).expect(release);
            let OriginalRegistrySource::Conditional(metadata) = words.source() else {
                panic!("{release}: the declared receiver needs conditional source authority");
            };
            assert_eq!(metadata.command(), "after", "{release}");
            assert!(
                metadata
                    .obligations()
                    .contains(&OriginalCatalogueSourceObligation::ConditionalReceiverSource)
            );
            let input = analysis.resolved_input.as_ref().unwrap();
            let realm = analysis.retained_command_realm().unwrap();
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(SOURCE),
                input.lexer_config(),
                &command,
            );
            realm.stamp_original_tokens(&mut tokens);
            let supplied = crate::registry_invocation::original_conditional_registry_metadata(
                &input.context_registry(),
                &tokens,
                metadata.original_head(),
                Some(metadata.original_namespace()),
            )
            .expect(release);
            assert_eq!(&supplied, metadata.as_ref());

            let binding = tokens.source_binding.as_ref().unwrap();
            assert!(binding.proved_target().is_none(), "{release}");
            assert!(
                binding.declaration_operand_layout_advice(&tokens).is_none(),
                "{release}"
            );
            assert!(
                super::super::declaration_layout::original_declaration_layouts(
                    binding.declaration_layout_observations.as_deref().unwrap(),
                )
                .is_none(),
                "{release}: source rows cannot supply execution or read advice"
            );
            assert!(!metadata.matches_source(
                &tcl_lexer::SourceImage::native(SOURCE.as_bytes()),
                input.lexer_config(),
            ));
        }
    }

    #[test]
    fn called_receiver_catalogue_keeps_source_barriers_and_currency() {
        for release in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            for prefix in ["proc after args {}\n", "rename ::after {}\n"] {
                let source = format!("{prefix}{SOURCE}");
                let analysis = analyse(&source, release);
                let command = callback(&source, &analysis);
                assert!(
                    source_registry_words(&source, &analysis, &command).is_none(),
                    "{release}/{prefix}"
                );
            }
            let analysis = analyse(SOURCE, release);
            let command = callback(SOURCE, &analysis);
            let changed = format!("{SOURCE}# changed whole source\n");
            assert!(source_registry_words(&changed, &analysis, &command).is_none());
            let mut foreign = analysis.clone();
            foreign.body_lexer_config.as_mut().unwrap().expand_syntax ^= true;
            assert!(source_registry_words(SOURCE, &foreign, &command).is_none());
        }
    }
}
