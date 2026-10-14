// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original source owners for procedure-effect summary selection.

use crate::ir::{CommandTokens, Module, ProcedureId};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tcl_registry::CommandRegistry;

/// The actual Module issuing summary coordinates and original call receipts.
/// Unavailable ownership cannot select a procedure. Raw memo identity retains
/// this owner; certified registry-only templates independently omit it.
#[derive(Debug, Clone, Default)]
pub struct OriginalProcedureSummaryOwners(Option<Arc<Module>>);
impl PartialEq for OriginalProcedureSummaryOwners {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(left), Some(right)) => Arc::ptr_eq(left, right),
            (None, None) => true,
            _ => false,
        }
    }
}
impl Eq for OriginalProcedureSummaryOwners {}
impl Hash for OriginalProcedureSummaryOwners {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ref().map(Arc::as_ptr).hash(state);
    }
}
impl OriginalProcedureSummaryOwners {
    /// Retain the unchanged source Module and its real issued body coordinates.
    #[must_use]
    pub fn for_module(module: &Module, registry: &CommandRegistry) -> Self {
        Self(
            module
                .retained_source_bindings
                .as_ref()
                .filter(|owner| owner.matches_module(module, registry))
                .map(|_| Arc::new(module.clone())),
        )
    }

    pub(super) fn module(&self, registry: &CommandRegistry) -> Option<&Module> {
        let module = self.0.as_deref()?;
        module
            .retained_source_bindings
            .as_ref()?
            .matches_module(module, registry)
            .then_some(module)
    }

    pub(super) fn calls(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<
        Vec<(
            ProcedureId,
            crate::registry_invocation::EffectiveCommandWords,
        )>,
    > {
        let binding = tokens.source_binding.as_ref()?;
        let module = self.module(registry);
        let metadata = match module {
            Some(module) => {
                binding.original_invocation_metadata_for_module(tokens, module, registry)?
            }
            None => binding
                .original_source_metadata_for_advice(tokens, registry)
                .ok()??,
        };
        if binding.original_invocation_source_purpose(tokens, metadata, registry)?
            == crate::command_binding::OriginalInvocationSourcePurpose::Logical
        {
            if crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                registry, metadata, tokens).is_some() { return Some(Vec::new()); }
            return crate::registry_invocation::original_logical_procedure_calls_for_module(
                tokens, module?, registry,
            )?
            .into_iter()
            .map(|call| Some((call.id(), call.effective_arguments().clone())))
            .collect();
        }
        let head = original_head(tokens)?;
        let lookup = binding.lookup_command_word(&head);
        if lookup.unknown || lookup.may_be_absent || lookup.targets.is_empty() {
            return None;
        }
        lookup
            .targets
            .iter()
            .filter(|target| !target.registry_backed)
            .map(|target| {
                let (id, _) = module?.source_procedure_for_target(target)?;
                Some((
                    id,
                    crate::registry_invocation::effective_words_for_target(tokens, target)?,
                ))
            })
            .collect()
    }

    pub(super) fn children(
        &self,
        tokens: &CommandTokens,
        registry: &CommandRegistry,
    ) -> Option<Vec<CommandTokens>> {
        let binding = tokens.source_binding.as_ref()?;
        let metadata = match self.module(registry) {
            Some(module) => {
                crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?
            }
            None => binding
                .original_source_metadata_for_advice(tokens, registry)
                .ok()??,
        };
        let config = binding.original_lexer_config_for_tokens(tokens)?;
        crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            tokens, config, registry, metadata,
        )?
        .into_iter()
        .map(|call| call.tokens)
        .collect()
    }
}

pub(super) fn original_head(tokens: &CommandTokens) -> Option<String> {
    let binding = tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let config = binding.original_lexer_config_for_tokens(tokens)?;
    let words = crate::registry_invocation::original_native_compiler_words(
        site.source.source_image(),
        tokens.words(),
        site.offset,
        config,
    )?;
    String::from_utf8(tcl_syntax::word_rules::original_static_word_source_bytes(
        words.first()?,
    )?)
    .ok()
}
