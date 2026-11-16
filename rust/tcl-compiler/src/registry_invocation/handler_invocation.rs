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

//! Invocations of a selected handler: callbacks, declared assistance and completion routes.

use super::ResolvedStatementInvocation;
use super::body_candidates::body_assistance_context;
use super::effective_words::{
    effective_words_for_target, frozen_argument_values, frozen_argument_words,
};
use super::metadata_context::InvocationMetadataContext;
use super::structured_invocation::original_declared_structured_invocation;
use super::word_resolution::{
    RegistryInvocationResolution, resolve_effective_tokens_with_metadata_context_and_case,
};
use crate::ir::{CommandTokens, WordExpr};
use tcl_registry::CommandRegistry;
use tcl_registry::model::semantic::SemanticContext;

/// Possible effect regions on the selected handler's normal route. No
/// completion, compiler, purity or unknown-residual obligation is withdrawn.
pub(crate) fn possible_normal_handler_effect_regions_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<(
    crate::side_effects::EffectRegion,
    crate::side_effects::EffectRegion,
)> {
    let invocation = resolved_handler_invocation_with_metadata_context(registry, context, tokens)?;
    Some(crate::side_effects::normal_handler_effect_regions(
        &invocation.facts.effects,
    ))
}

/// Exact original argv layout of the independently proved handler. This
/// borrows its retained semantic context and implementation dependencies, but
/// does not require an expression preparation, successful execution or CPP.
/// Consumers close any value/evaluation obligations through their own owner.
pub(crate) fn original_selected_handler_layout_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let context = body_assistance_context(registry, tokens)?;
    resolved_handler_invocation(registry, Some(context), tokens)
}

/// Selected callback grammar from an actual handler or its authentic
/// declaration observation. This supplies no successful handler or CPP entry.
pub(crate) fn original_callback_invocation(
    registry: &CommandRegistry,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    let context = body_assistance_context(registry, tokens)?;
    original_callback_invocation_with_metadata_context(registry, context.into(), tokens)
}

pub(crate) fn original_callback_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    if !context.matches_registry(registry) {
        return None;
    }
    if tokens.synthetic.is_some() {
        return None;
    }
    resolved_handler_invocation_with_metadata_context(registry, Some(context), tokens)
        .inspect(|invocation| {
            trace_callback_selection("handler", context, tokens, invocation);
        })
        .or_else(|| {
            original_declared_structured_invocation(registry, context, tokens).inspect(
                |invocation| {
                    trace_callback_selection("declaration", context, tokens, invocation);
                },
            )
        })
}

fn trace_callback_selection(
    route: &str,
    context: InvocationMetadataContext<'_>,
    tokens: &CommandTokens,
    invocation: &ResolvedStatementInvocation,
) {
    if tcl_registry::trace_deferred_writes_any() {
        eprintln!(
            "TCL_LSP_TRACE_METADATA_CALLBACK route={route} source_offset={:?} written_head={:?} canonical={:?} context={:p}",
            tokens
                .words()
                .first()
                .map(|word| word.source().span.start()),
            tokens.argv_texts.first(),
            invocation.facts.canonical_command,
            context.context(),
        );
    }
}

pub(crate) fn resolved_handler_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    resolved_handler_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

pub(crate) fn resolved_handler_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<ResolvedStatementInvocation> {
    use tcl_registry::native_compilation::NormalHandlerImplementationLookup;
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let target = binding
        .proved_handler_target()
        .filter(|target| target.registry_backed)?;
    let effective = effective_words_for_target(tokens, target)?;
    let dialect = binding.variable_context.invocation_dialect.or_else(|| {
        registry
            .profile()
            .map(tcl_registry::InvocationDialect::of_profile)
    });
    let (resolution, authored_case_body_selection) =
        resolve_effective_tokens_with_metadata_context_and_case(
            registry, context, tokens, &effective, dialect,
        )
        .ok()?;
    let RegistryInvocationResolution::Resolved(facts) = resolution else {
        return None;
    };
    let invocation = ResolvedStatementInvocation {
        dialect,
        arguments: effective.argument_presentations(tokens.argv_texts.get(1..)?)?,
        evaluated_arguments: frozen_argument_values(tokens, &effective),
        evaluated_words: frozen_argument_words(tokens, &effective),
        effective,
        facts,
        authored_case_body_selection: Some(authored_case_body_selection),
    };
    match invocation
        .facts
        .normal_handler_implementation_lookup(dialect)
    {
        NormalHandlerImplementationLookup::NoneRequired => {}
        NormalHandlerImplementationLookup::Required(lookup) => {
            if !binding.proves_native_implementation_lookup(&lookup) {
                return None;
            }
        }
        NormalHandlerImplementationLookup::RequiredPath(paths) => {
            let closed = invocation.with_argument_words(|words| {
                binding.proves_native_handler_path(
                    paths,
                    words.arguments(),
                    invocation.facts.argument_offset,
                )
            });
            if !closed {
                return None;
            }
        }
        NormalHandlerImplementationLookup::Unknown => return None,
    }
    Some(invocation)
}

/// Document-declared argument roles applicable at a retained lookup site.
/// These are advisory facts: they cannot establish stores, effects, completion,
/// purity or executable dispatch and are deliberately separate from runtime facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredInvocationAssistance<'a> {
    /// Authored declaration with its original trust provenance.
    pub declaration: &'a tcl_registry::model::DeclaredCommand,
    /// Declared role positions in the written argument sequence.
    pub roles: Vec<(usize, tcl_registry::ArgRole)>,
}

/// Query declaration assistance through its owner after the shared lookup
/// kernel establishes slot applicability. Mutation or argument expansion
/// cannot recover an applicable contract by written-head matching.
#[must_use]
pub fn resolved_declared_assistance<'a>(
    surface: &tcl_registry::model::DocumentCommandSurface<'a>,
    tokens: &CommandTokens,
) -> Option<DeclaredInvocationAssistance<'a>> {
    let candidate = tokens.source_binding.as_ref()?.declared_command.as_ref()?;
    let declaration = surface.declared_command(&candidate.name)?;
    if tokens
        .words()
        .iter()
        .any(|word| matches!(word, WordExpr::Expand { .. } | WordExpr::Opaque { .. }))
    {
        return None;
    }
    let supplied = tokens.words().len().checked_sub(1)?;
    let mut roles = Vec::new();
    for argument in &declaration.arguments {
        for index in declaration.arg_indices_for_role(argument.role, supplied) {
            if !roles.contains(&(index, argument.role)) {
                roles.push((index, argument.role));
            }
        }
    }
    roles.sort_by_key(|(index, _)| *index);
    Some(DeclaredInvocationAssistance { declaration, roles })
}

/// Completion of the selected normal handler for diagnostic flow only.
/// Native compiler uncertainty and its independent pre-entry failure remain in
/// the execution carrier; this route cannot license CFG pruning or an opcode.
#[must_use]
pub fn normal_handler_completion_route(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> tcl_registry::completion_route::InvocationCompletionRoute {
    resolved_handler_invocation(registry, context, tokens).map_or(
        tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
        |invocation| invocation.completion_route(registry),
    )
}

/// Procedure caller operands of a converged handler, independently of its
/// compiler protocol. This carries no effects of the called procedure.
pub struct NormalUserProcedureInvocation {
    /// Frozen procedure name at the handler's selected operand.
    pub target: String,
    /// Frozen actual values passed to the procedure, preserving unknown slots.
    pub arguments: Vec<Option<String>>,
    /// Actual runtime dispatch uncertainty, separate from compiler uncertainty.
    pub unknown_runtime: bool,
}

/// Project only an authored converged procedure-caller contract. Catalogue
/// spelling and an unknown actual command binding cannot provide this proof.
#[must_use]
pub fn normal_user_procedure_invocation(
    registry: &CommandRegistry,
    context: Option<SemanticContext>,
    tokens: &CommandTokens,
) -> Option<NormalUserProcedureInvocation> {
    normal_user_procedure_invocation_with_metadata_context(
        registry,
        context.map(InvocationMetadataContext::from),
        tokens,
    )
}

pub(crate) fn normal_user_procedure_invocation_with_metadata_context(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<NormalUserProcedureInvocation> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let invocation = resolved_handler_invocation_with_metadata_context(registry, context, tokens)?;
    let layout = invocation.with_argument_words(|words| {
        invocation
            .facts
            .successful_handler_user_procedure_call(words.arguments())
    })?;
    Some(NormalUserProcedureInvocation {
        target: invocation.argument_literal(layout.target_at)?,
        arguments: (layout.arguments_from..invocation.arguments.len())
            .map(|index| invocation.argument_literal(index))
            .collect(),
        unknown_runtime: binding.unknown || binding.may_be_absent,
    })
}
