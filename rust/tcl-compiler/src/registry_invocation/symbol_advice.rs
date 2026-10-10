// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Positioned Registry symbol-name advice, separate from execution and stores.

use super::{
    CommandRegistry, CommandTokens, InvocationWordOrigin, RegistryInvocationShape,
    catalogue_invocation_assistance,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalSymbolDeclarationAdvice {
    pub(crate) command: String,
    pub(crate) symbol: tcl_registry::SymbolDef,
    pub(crate) traits: tcl_registry::Traits,
    pub(crate) conditional_metadata: Option<super::OriginalConditionalRegistryMetadata>,
}

/// Original direct written operands under unanimous Registry grammar, or an
/// independently applicable catalogue declaration. This supplies only source
/// cards; a package need not have loaded, and no reached handler is inferred.
/// Alias-inserted and expanded positions stay unavailable to this direct form.
pub(crate) fn original_symbol_declaration_advice<'a>(
    registry: &CommandRegistry,
    context: impl Into<super::InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<OriginalSymbolDeclarationAdvice> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    if let Some(catalogue) = catalogue_invocation_assistance(registry, context, tokens) {
        let selected = for_shape(registry, context, &catalogue.shape);
        trace_symbol_advice(
            "catalogue",
            tokens,
            Some(&catalogue.shape.command),
            selected.is_some(),
        );
        return selected;
    }
    let Some(assistance) = super::original_registry_invocation_assistance_with_metadata_context(
        registry,
        Some(context),
        tokens,
    ) else {
        trace_symbol_advice("no-assistance", tokens, None, false);
        return None;
    };
    if assistance.unknown_residual || assistance.may_be_absent {
        trace_symbol_advice(
            if assistance.unknown_residual {
                "unknown-residual"
            } else {
                "may-absent"
            },
            tokens,
            None,
            false,
        );
        return None;
    }
    let mut agreed = None;
    for candidate in &assistance.candidates {
        let Some(selected) = for_shape(registry, context, candidate) else {
            trace_symbol_advice("candidate-shape", tokens, Some(&candidate.command), false);
            return None;
        };
        if agreed.as_ref().is_some_and(|prior| prior != &selected) {
            return None;
        }
        agreed = Some(selected);
    }
    trace_symbol_advice("candidate-consensus", tokens, None, agreed.is_some());
    agreed
}

/// Vendor source cards consume their own original word/context producer.
/// Registry availability supplies authored metadata only, without native
/// lookup, shadow closure, Normal or source table publication.
pub(crate) fn vendor_symbol_declaration_advice(
    context: &tcl_registry::model::ContextRegistry,
    tokens: &CommandTokens,
    head: &crate::signature_scan::vendor_name::VendorSourceNameInput,
    original: &[tcl_lexer::NativeWord],
) -> Option<OriginalSymbolDeclarationAdvice> {
    let metadata =
        super::original_conditional_vendor_registry_metadata(context, tokens, head, original)?;
    let shape = metadata.shape();
    let registry = context.commands();
    let symbol =
        *registry.defines_symbol(shape.command(), Some(shape.context().authoring_query()))?;
    if shape
        .original_words()
        .iter()
        .any(|word| word.group().expand)
    {
        return None;
    }
    let supplied = shape.original_words().len().checked_sub(1)?;
    if usize::from(symbol.name_arg) >= supplied
        || symbol
            .requires_arg
            .is_some_and(|argument| usize::from(argument) >= supplied)
    {
        return None;
    }
    Some(OriginalSymbolDeclarationAdvice {
        command: shape.command().to_owned(),
        symbol,
        traits: shape.possible_traits(),
        conditional_metadata: None,
    })
}

fn trace_symbol_advice(stage: &str, tokens: &CommandTokens, command: Option<&str>, admitted: bool) {
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_SYMBOL_ADVICE").is_some() {
        eprintln!(
            "ORIGINAL_SYMBOL_ADVICE stage={stage} offset={:?} words={} command={command:?} admitted={admitted} catalogue={} binding={}",
            tokens
                .words()
                .first()
                .map(|word| word.source().span.start()),
            tokens.words().len(),
            tokens
                .source_binding
                .as_ref()
                .is_some_and(|binding| binding.catalogue_command.is_some()),
            tokens.source_binding.is_some()
        );
    }
    let _ = (stage, tokens, command, admitted);
}

fn for_shape(
    registry: &CommandRegistry,
    context: super::InvocationMetadataContext<'_>,
    shape: &RegistryInvocationShape,
) -> Option<OriginalSymbolDeclarationAdvice> {
    if shape.effective.words.len() != shape.effective.origins.len()
        || !shape.effective.binding_prefix.is_empty()
        || !shape
            .effective
            .origins
            .iter()
            .enumerate()
            .skip(1)
            .all(|(ordinal, origin)| *origin == InvocationWordOrigin::Written(ordinal))
    {
        return None;
    }
    let symbol =
        *registry.defines_symbol(&shape.command, Some(context.context().authoring_query()))?;
    let supplied = shape.effective.words.len().checked_sub(1)?;
    if usize::from(symbol.name_arg) >= supplied
        || symbol
            .requires_arg
            .is_some_and(|argument| usize::from(argument) >= supplied)
    {
        return None;
    }
    Some(OriginalSymbolDeclarationAdvice {
        command: shape.command.clone(),
        symbol,
        traits: shape.possible_traits,
        conditional_metadata: None,
    })
}
