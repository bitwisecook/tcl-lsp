// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional source write-name metadata, independent of executed stores.

use super::declaration_assistance::DeclarationVariableReceiver;
use super::{
    CommandRegistry, CommandTokens, RegistryInvocationResolution, RegistryInvocationShape,
    catalogue_invocation_assistance,
};

/// Actual hosted Registry roles remain readonly source advice. Expansion
/// withdraws direct written ordinals; alias/destruction roles stay separate.
pub(crate) fn vendor_variable_write_advice(
    shape: &super::VendorRegistryInvocationShape,
) -> Option<Vec<DeclarationVariableReceiver>> {
    if !shape.roles_complete()
        || shape
            .original_words()
            .iter()
            .any(|word| word.group().expand)
    {
        return None;
    }
    if shape.possible_traits().intersects(
        tcl_registry::Traits::CREATES_SCOPE_ALIAS | tcl_registry::Traits::DESTROYS_VARIABLE,
    ) {
        return Some(Vec::new());
    }
    let mut result = Vec::new();
    for &(index, role) in shape.roles() {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        let argument = shape.argument_offset().checked_add(usize::from(index))?;
        shape.original_words().get(argument.checked_add(1)?)?;
        let Some(form) = shape.possible_variable_receiver_operand_form(argument) else {
            continue;
        };
        if form != tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined {
            continue;
        }
        let selected = DeclarationVariableReceiver { argument, form };
        if !result.contains(&selected) {
            result.push(selected);
        }
    }
    Some(result)
}

/// Retained candidates supply only conditional source cards. Unknown execution
/// remains unknown; unanimous known shapes cannot close its residual. A
/// captured/expanded argument cannot manufacture a written name position.
pub(crate) fn original_variable_write_advice<'a>(
    registry: &CommandRegistry,
    context: impl Into<super::InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<Vec<DeclarationVariableReceiver>> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }
    if let Some(catalogue) = catalogue_invocation_assistance(registry, context, tokens) {
        return write_arguments(registry, context, tokens, &catalogue.shape);
    }
    let assistance = super::registry_invocation_assistance_with_metadata_context(
        registry,
        Some(context),
        tokens,
    )?;
    let mut agreed = None;
    for candidate in &assistance.candidates {
        let selected = write_arguments(registry, context, tokens, candidate)?;
        if agreed
            .as_ref()
            .is_some_and(|previous| previous != &selected)
        {
            return None;
        }
        agreed = Some(selected);
    }
    agreed
}

fn write_arguments(
    registry: &CommandRegistry,
    context: super::InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    shape: &RegistryInvocationShape,
) -> Option<Vec<DeclarationVariableReceiver>> {
    let binding = tokens.source_binding.as_ref()?;
    let RegistryInvocationResolution::Resolved(facts) =
        super::resolve_effective_tokens_with_metadata_context(
            registry,
            Some(context),
            tokens,
            &shape.effective,
            binding.variable_context.invocation_dialect,
        )
        .ok()?
    else {
        return None;
    };
    if !facts.arg_roles_complete {
        return None;
    }
    if facts.traits.intersects(
        tcl_registry::Traits::CREATES_SCOPE_ALIAS | tcl_registry::Traits::DESTROYS_VARIABLE,
    ) {
        return Some(Vec::new());
    }
    let mut result = Vec::new();
    for &(index, role) in &facts.arg_roles {
        if role != tcl_registry::ArgRole::VarWrite {
            continue;
        }
        let effective = facts.argument_offset.checked_add(usize::from(index))?;
        let Some(form) = facts.variable_receiver_operand_form(effective) else {
            continue;
        };
        let Some(argument) = shape.effective.written_argument(effective) else {
            continue;
        };
        if form != tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined {
            continue;
        }
        let selected = DeclarationVariableReceiver { argument, form };
        if !result.contains(&selected) {
            result.push(selected);
        }
    }
    Some(result)
}
