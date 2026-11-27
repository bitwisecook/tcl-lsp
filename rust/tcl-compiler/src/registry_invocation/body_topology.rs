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

//! Possible body invocations and the conditional body topology advice of a handler.

use super::body_candidates::{
    owned_body_layout_invocation, prepare_body_candidates, prepared_body_invocation,
};
use super::effective_words::{
    compose_original_effective_words, effective_words_for_target, frozen_argument_values,
    frozen_argument_words,
};
use super::metadata_context::InvocationMetadataContext;
use super::native_compilation::native_compiler_replay_source;
use super::word_resolution::{
    EffectiveInvocationWord, RegistryInvocationResolution, effective_invocation_word,
    resolve_effective_tokens_with_metadata_context,
    resolve_registry_words_in_realm_with_metadata_context,
};
use super::{EffectiveCommandWords, ResolvedStatementInvocation};
use crate::ir::CommandTokens;
use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{CommandRegistry, InvocationWords};

/// Purpose-limited possible body inventory of a converged handler. It exposes
/// no registry effects, purity, completion or executable opcode permission.
pub struct PossibleBodyInvocation {
    /// Logical source grammar of this selected layout, independently of runtime dispatch.
    pub dialect: Option<tcl_native_model::InvocationDialect>,
    /// Registry-authored topology of possible script entry.
    pub topology: tcl_registry::native_compilation::PossibleBodyTopology,
    /// Frozen native expression operands, preserving unknown evaluated values.
    pub condition_values: Vec<(usize, Option<String>)>,
    /// Original/alias operand correspondence; original words still evaluate once.
    pub effective: EffectiveCommandWords,
}

/// Query candidate bodies independently of native compiler selection. Only the
/// shared source kernel's converged handler and exact frozen argv/frame contract
/// can supply this inventory; written spellings do not provide fallback proof.
#[must_use]
pub fn possible_body_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<PossibleBodyInvocation> {
    possible_body_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

/// Candidate handler-body topology under the supplied complete availability.
/// The original handler and frame owner remain independently required.
pub(crate) fn possible_body_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<PossibleBodyInvocation> {
    if context.is_some_and(|context| !context.matches_registry(registry))
        || tokens.synthetic.is_some()
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let target = binding
        .proved_handler_target()
        .filter(|target| target.registry_backed)?;
    let effective = effective_words_for_target(tokens, target)?;
    let dialect = binding.variable_context.invocation_dialect;
    let RegistryInvocationResolution::Resolved(facts) =
        resolve_effective_tokens_with_metadata_context(
            registry, context, tokens, &effective, dialect,
        )
        .ok()?
    else {
        return None;
    };
    let invocation = ResolvedStatementInvocation {
        dialect,
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
        facts,
        authored_case_body_selection: None,
    };
    let topology = invocation.with_argument_words(|words| {
        invocation
            .facts
            .possible_handler_body_flow(
                registry,
                words.arguments(),
                binding.variable_context.alias_frame(),
            )?
            .topology()
    })?;
    let condition_values = match &topology {
        tcl_registry::native_compilation::PossibleBodyTopology::Conditional(branches) => branches
            .iter()
            .filter_map(|(condition, _)| *condition)
            .map(|argument| (argument, invocation.argument_literal(argument)))
            .collect(),
        _ => Vec::new(),
    };
    Some(PossibleBodyInvocation {
        dialect: invocation.dialect,
        topology,
        condition_values,
        effective: invocation.effective,
    })
}

/// Conditional topology from an original declaration's owned invocation
/// layout. This is lexical diagnostic advice only; it grants neither entered
/// phases nor successful dispatch and retains every runtime residual.
pub(crate) fn conditional_body_topology_advice_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<PossibleBodyInvocation> {
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) {
        return None;
    }
    let context = if logical_source {
        tokens
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_input(
                tokens,
                context.source_analysis_input()?,
                registry,
            )?
    } else {
        context
    };
    conditional_runtime_body_topology_advice(registry, context, tokens, bindings).or_else(|| {
        original_compilation_body_topology_advice(registry, context, tokens).or_else(|| {
            original_layout_body_topology_advice(
                registry,
                context,
                tokens,
                &bindings.declaration_operand_layout_advice(tokens)?,
            )
        })
    })
}

fn conditional_runtime_body_topology_advice(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    bindings: &crate::command_binding::SourceCommandBindings,
) -> Option<PossibleBodyInvocation> {
    let binding = tokens.source_binding.as_ref()?;
    let invocation = if let Some(owned) =
        owned_body_layout_invocation(registry, context, tokens, bindings)
    {
        owned.invocation
    } else {
        // Known candidates retain lexical possibilities alongside an opaque
        // alternative. They cannot publish entered phases or close dispatch.
        let site = binding.invocation_site()?;
        native_compiler_replay_source(tokens, site)?;
        let entry = bindings.conditional_body_entry_at(&site.source, site.offset)?;
        let mut conditional = tokens.clone();
        conditional.source_binding =
            Some(bindings.conditional_invocation_at_source(&entry, site.offset));
        if !entry.owns_invocation(conditional.source_binding.as_ref()?) {
            return None;
        }
        let prepared = prepare_body_candidates(registry, context, &conditional)?;
        let mut unanimous = None;
        for candidate in &prepared.candidates {
            let selected = prepared_body_invocation(registry, context, &conditional, candidate)?;
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        if let Some(invocation) = unanimous {
            invocation
        } else {
            return original_compilation_body_topology_advice(registry, context, tokens);
        }
    };
    let topology = invocation.with_argument_words(|words| {
        invocation
            .facts
            .possible_handler_body_flow(
                registry,
                words.arguments(),
                binding.variable_context.alias_frame(),
            )?
            .topology()
    })?;
    let condition_values = match &topology {
        tcl_registry::native_compilation::PossibleBodyTopology::Conditional(branches) => branches
            .iter()
            .filter_map(|(condition, _)| *condition)
            .map(|argument| (argument, invocation.argument_literal(argument)))
            .collect(),
        _ => Vec::new(),
    };
    Some(PossibleBodyInvocation {
        dialect: invocation.dialect,
        topology,
        condition_values,
        effective: invocation.effective,
    })
}

/// Original compilation-table candidates select lexical topology only. Values
/// are decoded from unchanged source words under the captured logical grammar;
/// this never supplies runtime argument values or selected normal facts.
fn original_compilation_body_topology_advice<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
) -> Option<PossibleBodyInvocation> {
    let context = context.into();
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, context, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    if !context.matches_registry(registry) {
        return None;
    }

    let binding = tokens.source_binding.as_ref()?;
    let advice = if logical_source {
        binding.original_conditional_operand_layout_advice(tokens, registry)?
    } else {
        binding.original_compilation_lookup_advice(tokens)?
    };
    original_layout_body_topology_advice(registry, context, tokens, &advice)
}

fn original_layout_body_topology_advice<'a>(
    registry: &CommandRegistry,
    context: impl Into<InvocationMetadataContext<'a>>,
    tokens: &CommandTokens,
    advice: &crate::command_binding::OriginalCompilationLookupAdvice,
) -> Option<PossibleBodyInvocation> {
    let context = context.into();
    if !context.matches_registry(registry) {
        return None;
    }

    let dialect = advice.dialect();
    let mut unanimous = None;
    for target in advice.targets() {
        let effective =
            compose_original_effective_words(tokens, &target.command, &target.prepended)?;
        let words: Vec<_> = effective
            .words
            .iter()
            .map(|word| {
                effective_invocation_word(word, dialect.lexer_grammar.escapes, dialect.word_values)
            })
            .collect();
        let registry_words: Vec<_> = words
            .iter()
            .map(EffectiveInvocationWord::as_registry_word)
            .collect();
        let RegistryInvocationResolution::Resolved(facts) =
            resolve_registry_words_in_realm_with_metadata_context(
                registry,
                Some(context),
                &registry_words,
                Some(dialect),
                advice.realm(),
            )
            .ok()?
        else {
            return None;
        };
        let invocation =
            InvocationWords::structured(*registry_words.first()?, &registry_words[1..])
                .with_dialect(dialect);
        let frame = if advice.alias_frame() == tcl_registry::VariableAliasFrame::Unknown {
            tokens
                .source_binding
                .as_ref()?
                .declaration_alias_frame_advice(tokens)?
        } else {
            advice.alias_frame()
        };
        let topology = facts
            .possible_handler_body_flow(registry, invocation.arguments(), frame)?
            .topology()?;
        let condition_values = match &topology {
            tcl_registry::native_compilation::PossibleBodyTopology::Conditional(branches) => {
                branches
                    .iter()
                    .filter_map(|(condition, _)| *condition)
                    .map(|argument| {
                        (
                            argument,
                            words.get(argument + 1).and_then(|word| {
                                if let EffectiveInvocationWord::Literal(value) = word {
                                    Some(value.clone())
                                } else {
                                    None
                                }
                            }),
                        )
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        let selected = PossibleBodyInvocation {
            dialect: Some(dialect),
            topology,
            condition_values,
            effective,
        };
        if unanimous
            .as_ref()
            .is_some_and(|previous: &PossibleBodyInvocation| {
                previous.topology != selected.topology
                    || previous.condition_values != selected.condition_values
                    || previous.effective != selected.effective
            })
        {
            return None;
        }
        unanimous = Some(selected);
    }
    unanimous
}
