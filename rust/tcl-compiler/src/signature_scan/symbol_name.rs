// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Immutable source cards for Registry `SymbolDef` roles. A named source card
//! does not publish a command, select a live symbol or grant an editable span.

use super::original_name::SourceOriginalNameOccurrence;
use super::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use crate::analyser::types::DefinedSymbol;
use crate::command_binding::CommandAllocationSite;
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_registry::{CommandRegistry, RegistrySemanticKey, SymbolDef};

/// A genuine selected source definer and its independently produced name.
/// Reporting fields remain advisory; source and roles retain their own owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceSymbolDeclaration {
    head: SourceOriginalNameOccurrence,
    input: SignatureSourceNameInput,
    descriptor: SymbolDef,
    namespace: Option<SignatureNamespaceScope>,
    conditional_metadata: Option<crate::registry_invocation::OriginalConditionalRegistryMetadata>,
    metadata: DefinedSymbol,
    registry: RegistrySemanticKey,
}

impl OriginalSourceSymbolDeclaration {
    pub(crate) fn new(
        head: &SourceOriginalNameOccurrence,
        input: &SignatureSourceNameInput,
        descriptor: SymbolDef,
        namespace: Option<&SignatureNamespaceScope>,
        conditional_metadata: Option<
            &crate::registry_invocation::OriginalConditionalRegistryMetadata,
        >,
        metadata: &DefinedSymbol,
        registry: &CommandRegistry,
    ) -> Option<Self> {
        if metadata.original_name_input.as_ref() != Some(input)
            || matches!(input, SignatureSourceNameInput::OriginalVariableRoot(_))
            || metadata.kind != descriptor.kind
        {
            return None;
        }
        let head_key = head.name_input();
        if let Some(key) = input.original_word_key()
            && (key.source_image() != head_key.source_image()
                || key.lexer_config() != head_key.lexer_config())
        {
            return None;
        }
        Some(Self {
            head: head.clone(),
            input: input.clone(),
            descriptor,
            namespace: namespace.cloned(),
            conditional_metadata: conditional_metadata.cloned(),
            metadata: metadata.clone(),
            registry: registry.snapshot().semantic_key(),
        })
    }

    /// Actual complete source definer occurrence, independent of execution.
    #[must_use]
    pub fn site(&self) -> &CommandAllocationSite {
        self.head.site()
    }
    /// Original name value; readonly produced values remain distinct from words.
    #[must_use]
    pub fn name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Independently retained source namespace geometry for this definer.
    /// Missing geometry stays unknown; this does not select a live holder.
    #[must_use]
    pub fn original_namespace(&self) -> Option<&SignatureNamespaceScope> {
        self.namespace.as_ref()
    }
    /// Conditional catalogue schema retains unknown applicability separately.
    /// This remains source-card advice and grants no selected handler or cell.
    #[must_use]
    pub fn conditional_metadata(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalConditionalRegistryMetadata> {
        self.conditional_metadata.as_ref()
    }
    /// Independently selected Registry role, kind and written name ordinal.
    #[must_use]
    pub const fn descriptor(&self) -> &SymbolDef {
        &self.descriptor
    }
    /// Readonly original name operand range, without an editable grant.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.metadata.name_span
    }
    /// Immutable source-card reporting metadata, without publication identity.
    #[must_use]
    pub const fn metadata(&self) -> &DefinedSymbol {
        &self.metadata
    }
    /// Complete original image and full source grammar correspondence only.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.head.name_input().source_image() == image
            && self.head.name_input().lexer_config() == config
    }
    /// Same retained Registry semantic generation; no lookup is performed.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}
