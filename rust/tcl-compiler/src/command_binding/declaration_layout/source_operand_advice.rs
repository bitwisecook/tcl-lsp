// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The layout a declaration's source operand is read under, standalone scripts included.

use super::super::OriginalCompilationLookupAdvice;
use super::invocation_advice::symbolic_version;
use super::{
    DeclarationSourceOperandLayoutAdvice, DeclarationSourceOperandLayoutKind,
    DeclarationSourceVariableOperandAdvice, OriginalDiagnosticFrameEntry,
    original_source_catalogue_layouts,
};
use std::sync::Arc;

impl DeclarationSourceOperandLayoutAdvice {
    #[cfg(test)]
    pub(crate) fn dialect(&self) -> tcl_registry::InvocationDialect {
        match &self.kind {
            DeclarationSourceOperandLayoutKind::Original(advice) => advice.dialect(),
            DeclarationSourceOperandLayoutKind::Configured { layout, .. } => layout.dialect(),
        }
    }

    pub(super) fn current_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        self.current_source_advice(tokens, false)
    }

    pub(super) fn current_conditional_source_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCompilationLookupAdvice> {
        match &self.kind {
            DeclarationSourceOperandLayoutKind::Configured { layout, .. }
                if !layout.source_operations.is_empty() =>
            {
                self.current_source_advice(tokens, true)
            }
            _ => self.current_advice(tokens),
        }
    }

    fn current_source_advice(
        &self,
        tokens: &crate::ir::CommandTokens,
        conditional_source: bool,
    ) -> Option<OriginalCompilationLookupAdvice> {
        let binding = tokens.source_binding.as_ref()?;
        match &self.kind {
            DeclarationSourceOperandLayoutKind::Original(original) => {
                let current = binding.declaration_operand_layout_advice(tokens)?;
                (current.snapshot == original.snapshot
                    && current.site == original.site
                    && current.words == original.words
                    && current.config == self.config
                    && current.targets == original.targets)
                    .then_some(current)
            }
            DeclarationSourceOperandLayoutKind::Configured {
                layout,
                owner,
                cache,
            } => {
                let owner = owner.upgrade()?;
                let cache = cache.upgrade()?;
                if binding
                    .original_analysis_owner
                    .as_ref()
                    .is_none_or(|actual| !Arc::ptr_eq(actual, &owner))
                    || binding.has_original_source_layout_owner()
                    || binding
                        .original_source_operand_attachment
                        .as_ref()
                        .is_none_or(|attachment| !attachment.owns(&cache, layout.site.offset))
                    || owner.config != self.config
                    || !Arc::ptr_eq(&owner.source, &layout.site.source)
                    || cache
                        .entries
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .get(&layout.site.offset)?
                        .as_ref()
                        .is_none_or(|current| !Arc::ptr_eq(current, layout))
                {
                    return None;
                }
                if conditional_source {
                    layout.conditional_source_advice(tokens)
                } else {
                    layout.metadata_advice(tokens)
                }
            }
        }
    }

    pub(super) fn current_metadata_context(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<crate::registry_invocation::InvocationMetadataSelection<'_>> {
        self.current_conditional_source_advice(tokens)?;
        match &self.kind {
            DeclarationSourceOperandLayoutKind::Original(advice) => advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()
                .map(crate::registry_invocation::InvocationMetadataSelection::from),
            DeclarationSourceOperandLayoutKind::Configured { layout, .. } => layout
                .source_metadata_context(registry, tokens)
                .ok()
                .map(crate::registry_invocation::InvocationMetadataSelection::from),
        }
    }

    /// Registry candidates under the same original source configuration.
    /// The callback receives conditional schema data, never a Normal handler.
    pub(crate) fn with_conditional_registry_facts<T: PartialEq>(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        mut project: impl FnMut(
            &tcl_registry::InvocationFacts,
            &crate::registry_invocation::EffectiveCommandWords,
            &[crate::registry_invocation::EffectiveInvocationWord],
        ) -> Option<T>,
    ) -> Option<T> {
        let advice = self.current_conditional_source_advice(tokens)?;
        let metadata = self.current_metadata_context(registry, tokens)?.context();
        let mut selected = None;
        for target in advice.targets() {
            if !target.registry_backed {
                return None;
            }
            let effective = crate::registry_invocation::compose_original_effective_words(
                tokens,
                &target.command,
                &target.prepended,
            )?;
            let values = effective
                .words
                .iter()
                .map(|word| {
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        self.config.escapes,
                        tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
                    )
                })
                .collect::<Vec<_>>();
            let words = values
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                .collect::<Vec<_>>();
            let crate::registry_invocation::RegistryInvocationResolution::Resolved(facts) =
                crate::registry_invocation::resolve_registry_words_in_realm_with_metadata_context(
                    registry,
                    metadata,
                    &words,
                    Some(advice.dialect()),
                    advice.realm(),
                )
                .ok()?
            else {
                return None;
            };
            let candidate = project(&facts, &effective, &values)?;
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &candidate)
            {
                return None;
            }
            selected = Some(candidate);
        }
        selected
    }
}

impl DeclarationSourceVariableOperandAdvice {
    /// Reuse the current original scalar operand receipt without converting
    /// its source spelling into a Native value, cell or invocation fact.
    pub(in super::super) fn matches_original_scalar_read(&self, name: &str) -> bool {
        self.name == name && self.source_operations.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn diagnostic_version(
        &self,
        ssa: &crate::ssa::SsaFunction,
        block: crate::cfg::BlockId,
        index: usize,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(crate::ssa::Symbol, crate::ssa::Version)> {
        let tokens = crate::ssa::SsaSourceView::at_statement(ssa, block, index).source_tokens()?;
        let current = tokens
            .source_binding
            .as_ref()?
            .declaration_source_variable_operand_advice(registry, tokens, &self.word)?;
        (current.site == self.site
            && current.name == self.name
            && current.source_operations == self.source_operations)
            .then_some(())?;
        symbolic_version(ssa, block, index, &self.name)
    }
}

fn trace_root_script_option<T>(stage: &str, value: Option<T>) -> Option<T> {
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_TAINT_CU_INGRESS").is_some() {
        eprintln!(
            "TAINT_SCRIPT_INGRESS stage={stage} present={}",
            value.is_some()
        );
    }
    #[cfg(not(debug_assertions))]
    let _ = stage;
    value
}

impl super::super::SourceInvocationBinding {
    /// Original root-script naming geometry for source analysis only. A held
    /// declaration entry does not issue an entered frame or a selected cell.
    pub(crate) fn original_standalone_script_variable_scope(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(
        crate::signature_scan::scope::SignatureNamespaceScope,
        tcl_syntax::naming::NamePolicyProtocol,
    )> {
        self.original_standalone_script_scope(tokens, registry, false)
    }

    /// A possible source schema retains original coordinates and all current
    /// lookup obligations; it cannot supply a variable identity or storage.
    pub(crate) fn original_standalone_script_catalogue_candidate(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<super::super::OriginalCatalogueSourceCandidate> {
        let (namespace, policy) = self.original_standalone_script_scope(tokens, registry, true)?;
        let site = self.invocation_site()?;
        (site.source.source_image().channel() == tcl_lexer::SourceChannel::Document)
            .then_some(())?;
        let config = self.original_lexer_config_for_tokens(tokens)?;
        let original = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            config,
        )?;
        let key = crate::signature_scan::scope::SignatureSourceNameKey::from_original_logical_source_word(
            original.first()?, tcl_syntax::word_rules::WordValueRules::from_config(&config), policy,
        )?;
        let head =
            crate::signature_scan::original_name::SourceOriginalNameOccurrence::new(site, key)?;
        self.original_catalogue_source_candidate(tokens, &head, Some(&namespace), registry)
    }

    fn original_standalone_script_scope(
        &self,
        tokens: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        catalogue_only: bool,
    ) -> Option<(
        crate::signature_scan::scope::SignatureNamespaceScope,
        tcl_syntax::naming::NamePolicyProtocol,
    )> {
        if tokens.synthetic.is_some() || tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        let owner = trace_root_script_option("root.owner", self.original_analysis_attachment())?;
        if !owner.metadata.is_standalone() {
            return None;
        }
        let rows = trace_root_script_option(
            "root.layout_rows",
            original_source_catalogue_layouts(trace_root_script_option(
                "root.layout_inventory",
                self.declaration_layout_observations.as_deref(),
            )?),
        )?;
        if rows.clone().any(|row| {
            !matches!(
                row.entry.as_ref(),
                OriginalDiagnosticFrameEntry::RootScript { .. }
            )
        }) {
            return None;
        }
        let config =
            trace_root_script_option("root.config", self.original_lexer_config_for_tokens(tokens))?;
        let retained =
            trace_root_script_option("root.registry", self.retained_registry_for_tokens(tokens))?;
        (retained.snapshot().semantic_key() == registry.snapshot().semantic_key()).then_some(())?;
        let site = trace_root_script_option("root.site", self.invocation_site())?;
        if owner.config != config || !Arc::ptr_eq(&owner.source, &site.source) {
            return None;
        }
        let mut selected = None;
        for row in rows {
            let OriginalDiagnosticFrameEntry::RootScript {
                source,
                frame,
                namespace,
            } = row.entry.as_ref()
            else {
                return None;
            };
            let state = &row.snapshot.state;
            let baseline = &state.baseline;
            if row.config != config
                || row.words.as_ref() != tokens.words()
                || !Arc::ptr_eq(&source.origin, &site.source)
                || source.base() != 0
                || source.text.bytes() != site.source.source_image().bytes()
                || !row.entry.owns_source(&site.source, site.offset)
                || frame.layout() != &crate::var_resolve::VariableExecutionFrame::Global
                || namespace != &row.namespace
                || state.current_source_origin.as_ref() != Some(&site.source)
                || !baseline.metadata_context.is_standalone()
                || baseline.native_entry.is_some()
                || baseline.logical_source_input.is_some()
                || baseline.vendor_source_input.is_some()
                || baseline.hosted_execution_context.is_some()
                || baseline.unknown_entry
                || baseline.registry_owner.as_ref()?.semantic_key()
                    != registry.snapshot().semantic_key()
            {
                return None;
            }
            let policy = trace_root_script_option(
                "root.policy",
                trace_root_script_option("root.execution_policy", baseline.execution_name_policy)?
                    .native_recipe(),
            )?;
            if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation {
                return None;
            }
            let scope = trace_root_script_option(
                "root.namespace_geometry",
                if catalogue_only {
                    state
                        .original_command_world
                        .retained_scope_for_source_advice(namespace, policy)
                } else {
                    state.original_namespace_geometry(namespace, policy)
                },
            )?;
            let current = (scope, policy);
            if selected.as_ref().is_some_and(|prior| prior != &current) {
                return None;
            }
            selected = Some(current);
        }
        trace_root_script_option("root.selected", selected)
    }

    pub(crate) fn declaration_source_operand_layout_advice(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<DeclarationSourceOperandLayoutAdvice> {
        if tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        if self.has_original_source_layout_owner() {
            let advice = self.declaration_operand_layout_advice(tokens)?;
            advice
                .original_declaration_metadata_context(registry, tokens)
                .ok()?;
            return Some(DeclarationSourceOperandLayoutAdvice {
                config: advice.config,
                kind: DeclarationSourceOperandLayoutKind::Original(advice),
            });
        }
        let owner = self.original_analysis_owner.as_ref()?;
        let offset = tokens.words().first()?.source().span.start();
        let cache = owner
            .derived_memo()
            .source_operand_layouts
            .get()?
            .upgrade()?;
        let layout = cache
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&offset)?
            .as_ref()?
            .clone();
        let advice = DeclarationSourceOperandLayoutAdvice {
            config: owner.config,
            kind: DeclarationSourceOperandLayoutKind::Configured {
                layout,
                owner: Arc::downgrade(owner),
                cache: Arc::downgrade(&cache),
            },
        };
        advice.current_metadata_context(registry, tokens)?;
        Some(advice)
    }

    pub(crate) fn declaration_source_variable_operand_advice(
        &self,
        registry: &tcl_registry::CommandRegistry,
        tokens: &crate::ir::CommandTokens,
        word: &tcl_source::words::WordExpr,
    ) -> Option<DeclarationSourceVariableOperandAdvice> {
        let source = self.declaration_source_operand_layout_advice(registry, tokens)?;
        if let DeclarationSourceOperandLayoutKind::Original(_) = &source.kind {
            let original = self.declaration_variable_operand_advice(registry, tokens, word)?;
            return Some(DeclarationSourceVariableOperandAdvice {
                site: original.invocation,
                word: original.word,
                name: original.name,
                source_operations: Arc::from([]),
            });
        }
        if word.source().provenance != tcl_source::words::Provenance::Source
            || !tokens.words().iter().any(|original| original == word)
        {
            return None;
        }
        let crate::registry_invocation::EffectiveInvocationWord::Literal(name) =
            crate::registry_invocation::effective_invocation_word(
                word,
                source.config.escapes,
                tcl_syntax::word_rules::WordValueRules::from_config(&source.config),
            )
        else {
            return None;
        };
        if tcl_syntax::naming::is_qualified(name.as_bytes())
            || tcl_syntax::naming::split_array_name_braced_for_style(
                &name,
                true,
                source.config.braced_var,
            )
            .1
            .is_some()
        {
            return None;
        }
        let DeclarationSourceOperandLayoutKind::Configured { layout, .. } = &source.kind else {
            return None;
        };
        layout
            .variable_name_is_eligible(registry, &name)
            .then_some(())?;
        Some(DeclarationSourceVariableOperandAdvice {
            site: layout.site.clone(),
            word: word.clone(),
            name,
            source_operations: Arc::clone(&layout.source_operations),
        })
    }
}
