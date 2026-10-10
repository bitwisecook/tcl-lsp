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
#[cfg(debug_assertions)]
fn trace_receiver_catalogue(binding: &SourceInvocationBinding, stage: &str, available: bool) {
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE").is_some()
        && matches!(
            binding.variable_frame.layout(),
            crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
        )
    {
        eprintln!(
            "ORIGINAL_RECEIVER_CATALOGUE offset={:?} stage={stage} available={available} observations={}",
            binding.invocation_site().map(|site| site.offset),
            binding
                .declaration_layout_observations
                .as_ref()
                .map_or(0, |rows| rows.len()),
        );
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
        #[cfg(debug_assertions)]
        trace_receiver_catalogue(
            self,
            "original-rows",
            self.declaration_layout_observations
                .as_deref()
                .and_then(super::declaration_layout::original_declaration_layouts)
                .is_some(),
        );
        let rows = super::declaration_layout::original_declaration_layouts(
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
        #[cfg(debug_assertions)]
        trace_receiver_catalogue(self, "namespace-geometry", namespace.is_some());
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
        let candidate =
            self.original_catalogue_source_candidate(tokens, &head, Some(&namespace), registry);
        #[cfg(debug_assertions)]
        trace_receiver_catalogue(self, "candidate", candidate.is_some());
        candidate
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
        // Source-schema advice borrows original observations, but does not
        // require their execution frame/cell context to survive an unknown
        // command. Those stronger checks remain on the execution issuer.
        let rows = self
            .declaration_layout_observations
            .as_deref()?
            .iter()
            .filter(|row| {
                row.issuer
                    == super::declaration_layout::DeclarationLayoutIssuer::OriginalDeclaration
            });
        rows.clone().next()?;
        let registry_key = registry.snapshot().semantic_key();
        let mut candidate = None;
        let mut paths = None;
        let mut obligations = Vec::new();
        let mut observations = Vec::new();
        for row in rows {
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
