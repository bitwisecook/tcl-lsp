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

//! The proof that a straight-line body only calls empty source-class factories, which keeps caller types unclobbered.

use super::TypePropagationMetadata;
use crate::cfg::{Function as CfgFunction, Terminator};
use crate::ir::Statement;
use std::collections::HashSet;
use tcl_registry::CommandRegistry;

/// Account for an empty declaration through its actual original source schema.
/// The descriptive roster checks inclusion only; it cannot select a factory.
pub(super) fn empty_source_class(
    metadata: TypePropagationMetadata<'_>,
    tokens: &crate::ir::CommandTokens,
) -> Option<String> {
    let context = metadata.context?;
    let selected =
        crate::registry_invocation::original_structured_invocation_with_metadata_context(
            metadata.registry,
            context,
            tokens,
        )?;
    let spec = context
        .context()
        .resolve_spec(metadata.registry, &selected.facts.canonical_command)?;
    if !spec.traits.contains(tcl_registry::Traits::IS_OO_METACLASS)
        || !spec
            .definition_body?
            .profile
            .has_native_definition_workers()
    {
        return None;
    }
    let selector =
        crate::registry_invocation::invocation_word(selected.effective.words.get(1)?).literal()?;
    let method = CommandRegistry::manufacturer_method_for_descriptor(spec, selector)?;
    let body_index = usize::from(method.definition_body_at?);
    let name_index = usize::from(method.names_instance_at?);
    let name_word = selected.effective.written_argument(name_index)?;
    let body_word = selected.effective.written_argument(body_index)?;
    let words = tokens.words();
    let class = crate::registry_invocation::invocation_word(words.get(name_word + 1)?).literal()?;
    let body = crate::registry_invocation::invocation_word(words.get(body_word + 1)?).literal()?;
    (body.trim().is_empty() && selected.effective.words.len() == body_index + 2)
        .then(|| crate::naming::normalise_qualified_name(class))
}

pub(super) fn caller_safe_factory_invocation(
    metadata: TypePropagationMetadata<'_>,
    tokens: &crate::ir::CommandTokens,
    _head: &str,
    _args: &[&str],
) -> bool {
    let registry = metadata.registry;
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(registry))
    else {
        return false;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return false;
    };
    let Some(original_config) = binding.original_lexer_config_for_tokens(tokens) else {
        return false;
    };
    if original_config.nested().normalized() != metadata.config.nested().normalized() {
        return false;
    }
    if let Some(input) = context.source_analysis_input() {
        if metadata.config.normalized() != input.lexer_config().normalized()
            || binding
                .original_invocation_metadata_for_input(tokens, input, registry)
                .is_none()
        {
            return false;
        }
    } else if !context.is_standalone()
        || binding
            .original_invocation_metadata_at_point(tokens, registry)
            .is_err()
    {
        return false;
    }
    if let Some(shape) = binding.original_default_manufacture_shape(tokens, registry) {
        return shape.is_argument_free_anonymous_shape();
    }
    // An actual class target lacking the default owner cannot be repaired
    // through its reporting name or an ordinary TclOO default grammar.
    if binding.proved_target().is_some_and(|target| {
        target.kind == tcl_names::command::BindingKind::Class || !target.registry_backed
    }) {
        return false;
    }
    // Metadata ownership retains unknown lookup as a separate axis. Select
    // the actual original invocation before reading its Registry facts; a
    // written reporting head cannot repair a missing or clobbered binding.
    let Some(selected) =
        crate::registry_invocation::original_structured_invocation_with_metadata_context(
            registry, context, tokens,
        )
    else {
        return false;
    };
    !selected.facts.traits.intersects(
        tcl_registry::Traits::EVALUATES_CODE
            | tcl_registry::Traits::CREATES_BARRIER
            | tcl_registry::Traits::CREATES_DYNAMIC_BARRIER,
    ) && !selected.facts.state_transitions.is_declared()
}

/// Prove the narrow straight-line case of empty source class factories.
/// Any other opaque invocation, binding transition, class body, or control
/// edge withdraws the proof for the whole function.
pub(super) fn has_only_caller_safe_factories<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
) -> bool {
    if cfg.name != "::top"
        || known_classes.is_empty()
        || !cfg.exception_edges.is_empty()
        || cfg.blocks.iter().any(|(id, block)| {
            *id != cfg.entry && (!block.statements.is_empty() || block.terminator.is_some())
        })
        || !matches!(
            cfg.blocks[&cfg.entry].terminator,
            None | Some(Terminator::Goto { .. })
        )
    {
        return false;
    }
    let mut classes = HashSet::new();
    for stmt in &cfg.blocks[&cfg.entry].statements {
        if let Some(marker) = stmt.synthetic_marker() {
            if !matches!(
                marker,
                crate::ir::SyntheticMarker::UnseenCall
                    | crate::ir::SyntheticMarker::GlobalFrameScript
            ) || !cfg.blocks[&cfg.entry]
                .statements
                .iter()
                .any(|host| host.synthetic_marker().is_none() && host.span() == stmt.span())
            {
                return false;
            }
            continue;
        }
        match stmt {
            Statement::Call {
                command,
                args,
                tokens,
                ..
            }
            | Statement::Barrier {
                command,
                args,
                tokens,
                ..
            } => {
                let Some(tokens) = tokens else {
                    return false;
                };
                let words = tokens.words();
                if words
                    .first()
                    .and_then(|word| crate::registry_invocation::invocation_word(word).literal())
                    .is_none()
                {
                    return false;
                }
                let empty_class = empty_source_class(metadata, tokens);
                if let Some(class) = empty_class {
                    if !known_classes.contains(&class) || !classes.insert(class) {
                        return false;
                    }
                    continue;
                }
                if !caller_safe_factory_invocation(
                    metadata,
                    tokens,
                    command,
                    &args.iter().map(String::as_str).collect::<Vec<_>>(),
                ) {
                    return false;
                }
            }
            Statement::AssignConst { .. } | Statement::AssignValue { .. } => {}
            _ => return false,
        }
        if !substitutions_have_only_caller_safe_factories(stmt, metadata) {
            return false;
        }
    }
    !classes.is_empty()
}

/// Check authentic nested operands without replacing their source owner.
pub(super) fn substitutions_have_only_caller_safe_factories(
    stmt: &Statement,
    metadata: TypePropagationMetadata<'_>,
) -> bool {
    let Some(tokens) = stmt.tokens() else {
        // A typed statement without substitutions needs no nested factory
        // query. Recovered commands without original tokens cannot supply one.
        let inventory = crate::ir_helpers::evaluated_command_substitutions_with_metadata_context(
            stmt,
            metadata.registry,
            metadata.context,
            metadata.config,
        );
        return !inventory.opaque && inventory.all_commands().next().is_none();
    };
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(metadata.registry))
    else {
        return false;
    };
    let calls = if context.source_analysis_input().is_some() {
        crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            tokens,
            metadata.config,
            metadata.registry,
            context,
        )
    } else if context.is_standalone() {
        crate::word_subst::checked_lifted_calls(tokens, metadata.config)
    } else {
        None
    };
    let Some(calls) = calls else {
        return false;
    };
    calls.into_iter().all(|call| {
        let Some(tokens) = call.tokens.as_ref() else {
            return false;
        };
        caller_safe_factory_invocation(
            metadata,
            tokens,
            &call.command,
            &call.args.iter().map(String::as_str).collect::<Vec<_>>(),
        )
    })
}
