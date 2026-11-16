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

//! Complete, bounded substitution inventories over original source: argument words, authored body scopes and selected expression roles.

use super::dependency_regions::{
    OriginalSourceDependencyRegions, collect_original_expression_dependency_regions,
};
use super::{
    LiftedCall, LiftedCallCollection, MAX_SUBSTITUTION_DEPTH, collect_command_words,
    collect_surface_text_with_surface, lift, push_substitution,
};
use crate::ir::{CommandTokens, Provenance, SourceSite, WordExpr};
use tcl_lexer::Span;

/// Complete original substitution inventory for effect consumers. A truncated
/// or unsupported word walk must not establish absence of name effects.
#[doc(hidden)]
#[must_use]
pub fn checked_lifted_calls(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<LiftedCall>> {
    let collected = lift(Some(tokens), config, None);
    (!collected.incomplete).then_some(collected.calls)
}

/// Complete substitutions of the written argv alone. This source walk does
/// not enter the enclosing command's braced expression or script operands.
/// Parent configuration stays unchanged; each extracted child uses its nested
/// parser purpose in the existing collector.
pub(crate) fn checked_argument_lifted_calls(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<LiftedCall>> {
    let mut collected = LiftedCallCollection::for_original_parent(tokens, config);
    collect_command_words(tokens.words(), config, None, 0, &mut collected);
    if collected.incomplete {
        return None;
    }
    for call in &mut collected.calls {
        let child = call.tokens.as_mut()?;
        child.inherit_nested_bindings(tokens);
        call.words = Some(child.clone());
    }
    Some(collected.calls)
}

/// Source-only original argv inventory under the same retained input. Every
/// child matches its original vector/config/site; this supplies no dispatch.
pub(crate) fn checked_original_argument_lifted_calls(
    tokens: &CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
) -> Option<Vec<LiftedCall>> {
    let input = metadata.source_analysis_input()?;
    metadata.matches_registry(registry).then_some(())?;
    let binding = tokens.source_binding.as_ref()?;
    binding.original_argument_metadata_for_input(tokens, input, registry)?;
    let config = binding.original_argument_lexer_config(tokens)?;
    let calls = checked_argument_lifted_calls(tokens, config)?;
    for call in &calls {
        let child = call.tokens.as_ref()?;
        child
            .source_binding
            .as_ref()?
            .original_argument_metadata_for_input(child, input, registry)?;
    }
    Some(calls)
}

/// One authentic lexical/Expr substitution in an immutable Authored body scope.
/// The private receipt supplies source candidates, without a point or handler.
pub(crate) struct OriginalAuthoredSourceSubstitutionWords {
    pub(super) words: Vec<tcl_lexer::NativeWord>,
    registry: tcl_registry::RegistrySnapshot,
    pub(super) dialect: Option<tcl_registry::InvocationDialect>,
    realm: tcl_dialect::model::InvocationRealm,
}

impl OriginalAuthoredSourceSubstitutionWords {
    pub(crate) fn original_words(&self) -> &[tcl_lexer::NativeWord] {
        &self.words
    }
    pub(crate) fn with_source_schema<T>(
        &self,
        registry: &tcl_registry::CommandRegistry,
        apply: impl FnOnce(&tcl_registry::ResolvedInvocation<'_, '_>) -> Option<T>,
    ) -> Option<T> {
        (registry.snapshot().semantic_key() == self.registry.semantic_key()).then_some(())?;
        if self.words.iter().any(|word| word.group().expand) {
            return None;
        }
        let values = self
            .words
            .iter()
            .map(tcl_syntax::word_rules::original_static_word_unicode_value)
            .collect::<Vec<_>>();
        let word = |at: usize| {
            if self.words[at].group().expand {
                return tcl_registry::InvocationWord::Expanded;
            }
            values[at]
                .as_deref()
                .map_or(tcl_registry::InvocationWord::Dynamic, |bytes| {
                    crate::registry_invocation::source_structure::source_schema_word(
                        tcl_registry::InvocationWord::KnownBytes(bytes),
                    )
                })
        };
        let arguments = (1..self.words.len()).map(word).collect::<Vec<_>>();
        let mut words = tcl_registry::InvocationWords::structured(word(0), &arguments);
        if let Some(dialect) = self.dialect {
            words = words.with_dialect(dialect);
        }
        let selected =
            tcl_registry::model::assembly::resolve_structured_invocation_in_resolved_context(
                self.registry.registry(),
                None,
                words,
                self.realm,
            );
        apply(&selected.resolved()?)
    }
}

/// Bounded original lexical and selected Expr substitutions in the held source scope.
/// Source execution uncertainty remains independent even for an empty inventory.
pub(crate) fn checked_authored_source_substitution_words(
    tokens: &CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    scope: &crate::registry_invocation::OriginalAuthoredSourceBodyScope,
) -> Option<Vec<OriginalAuthoredSourceSubstitutionWords>> {
    let binding = tokens.source_binding.as_ref()?;
    (binding.original_authored_source_body_scope_for_name_roles(tokens, registry) == Some(scope))
        .then_some(())?;
    let original = scope.original_words(tokens, registry)?;
    let image = original.first()?.image().clone();
    let config = scope.config();
    let mut pending = vec![(tokens.clone(), original, 0)];
    let mut seen = std::collections::HashSet::new();
    let mut discovered =
        std::collections::HashMap::from([(pending[0].1[0].span().start(), pending[0].1.clone())]);
    let mut result = Vec::new();
    while let Some((parent, original, depth)) = pending.pop() {
        if depth > MAX_SUBSTITUTION_DEPTH {
            return None;
        }
        let offset = original.first()?.span().start();
        if !seen.insert(offset) {
            continue;
        }
        let mut collected = LiftedCallCollection::default();
        collect_command_words(parent.words(), config, None, 0, &mut collected);
        let has_braced_commands = parent
            .words()
            .iter()
            .any(|word| matches!(word, WordExpr::BracedLiteral { text, .. } if text.contains('[')));
        if has_braced_commands {
            let receipt = OriginalAuthoredSourceSubstitutionWords {
                words: original,
                registry: scope.retained_registry_snapshot().clone(),
                dialect: scope.original_word_dialect(),
                realm: scope.source_realm(),
            };
            let (roles, traits) = receipt.with_source_schema(registry, |schema| {
                let (roles, complete) = schema.authored_source_argument_roles();
                complete.then(|| {
                    (
                        roles
                            .into_iter()
                            .map(|(ordinal, role)| {
                                (schema.facts().argument_offset + usize::from(ordinal), role)
                            })
                            .collect::<Vec<_>>(),
                        schema.semantics.traits,
                    )
                })
            })?;
            collect_original_braced_expression_calls_in_image(
                &parent,
                config,
                &image,
                &roles,
                traits,
                OriginalExpressionInventoryPurpose::AuthoredCandidates,
                &mut collected,
            )?;
        }
        if collected.incomplete {
            return None;
        }
        for call in collected.calls {
            let child = call.tokens?;
            if child.source_binding.is_some()
                || child.synthetic.is_some()
                || !child.original_words_match_argv_text()
            {
                return None;
            }
            let offset = child.words().first()?.source().span.start();
            let words = crate::registry_invocation::original_native_compiler_words(
                &image,
                child.words(),
                offset,
                config,
            )?;
            if let Some(previous) = discovered.get(&offset) {
                if previous != &words {
                    return None;
                }
            } else {
                discovered.insert(offset, words.clone());
                result.push(OriginalAuthoredSourceSubstitutionWords {
                    words: words.clone(),
                    registry: scope.retained_registry_snapshot().clone(),
                    dialect: scope.original_word_dialect(),
                    realm: scope.source_realm(),
                });
                pending.push((child, words, depth + 1));
            }
        }
    }
    Some(result)
}

#[derive(Clone, Copy)]
pub(super) enum OriginalLiftedCallContext<'a> {
    Metadata(crate::registry_invocation::InvocationMetadataContext<'a>),
    LogicalInput(&'a crate::analyser::ResolvedAnalysisInput),
}

impl<'a> OriginalLiftedCallContext<'a> {
    pub(super) fn input(self) -> Option<&'a crate::analyser::ResolvedAnalysisInput> {
        match self {
            Self::Metadata(metadata) => metadata.source_analysis_input(),
            Self::LogicalInput(input) => Some(input),
        }
    }
}

/// Lexical and Expr substitutions under the actual readonly Logical word owner.
/// A supplied stronger owner or unavailable child stays a terminal refusal.
pub(crate) fn checked_original_lifted_calls_with_logical_source_input(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<Vec<LiftedCall>> {
    input.has_logical_source_name_context().then_some(())?;
    checked_original_source_lifted_calls(
        tokens,
        config,
        registry,
        OriginalLiftedCallContext::LogicalInput(input),
    )
}

/// Complete original nested-call geometry under the retained source input.
/// Selected expression roles extend lexical substitutions; every child keeps
/// its original sidecar. Missing source/config/roles or incomplete recovery
/// withdraws the inventory, independently of handler or body entry.
pub(crate) fn checked_original_lifted_calls_with_metadata_context(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
) -> Option<Vec<LiftedCall>> {
    checked_original_source_lifted_calls(
        tokens,
        config,
        registry,
        OriginalLiftedCallContext::Metadata(metadata),
    )
}

fn checked_original_source_lifted_calls(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
    context: OriginalLiftedCallContext<'_>,
) -> Option<Vec<LiftedCall>> {
    let mut pending = vec![(tokens.clone(), 0)];
    let mut seen = std::collections::HashSet::new();
    let mut calls: Vec<LiftedCall> = Vec::new();
    while let Some((parent, depth)) = pending.pop() {
        if depth > MAX_SUBSTITUTION_DEPTH {
            return None;
        }
        let offset = original_call_context(&parent, config, registry, context)?;
        if !seen.insert(offset) {
            continue;
        }
        let mut collected = LiftedCallCollection::for_original_parent(&parent, config);
        collect_command_words(&parent.word_exprs, config, None, 0, &mut collected);
        collect_original_expression_calls(&parent, config, registry, context, &mut collected)?;
        if collected.incomplete {
            return None;
        }
        for mut call in collected.calls {
            let child = call.tokens.as_mut()?;
            child.inherit_nested_bindings(&parent);
            call.words = Some(child.clone());
            if let Some(previous) = calls.iter().find(|previous| previous.span == call.span) {
                if previous != &call {
                    return None;
                }
            } else {
                pending.push((child.clone(), depth + 1));
                calls.push(call);
            }
        }
    }
    calls.sort_by_key(|call| (call.span.end(), std::cmp::Reverse(call.span.start())));
    Some(calls)
}

pub(super) fn original_call_context(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
    context: OriginalLiftedCallContext<'_>,
) -> Option<u32> {
    let input = context.input()?;
    if config.nested().normalized() != input.lexer_config().nested().normalized()
        || tokens.synthetic.is_some()
    {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    match context {
        OriginalLiftedCallContext::Metadata(metadata) => {
            metadata.matches_registry(registry).then_some(())?;
        }
        OriginalLiftedCallContext::LogicalInput(input) => {
            if binding.original_logical_source_input_for_name_roles(tokens, registry) != Some(input)
            {
                return None;
            }
            let site = binding.original_logical_source_site_for_name_roles(tokens, registry)?;
            site.source.original_lexical_command(
                tokens.words(),
                site.offset,
                input.lexer_config(),
            )?;
            return Some(site.offset);
        }
    }
    if binding.has_original_native_source_command() {
        return binding.original_native_source_syntax_offset(tokens, input, registry);
    }
    let original_config = binding.original_lexer_config_for_tokens(tokens)?;
    if original_config.nested().normalized() != config.nested().normalized() {
        return None;
    }
    let site = binding.invocation_site()?;
    site.source
        .original_lexical_command(tokens.words(), site.offset, original_config)?;
    Some(site.offset)
}

fn collect_native_expression_calls(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
    context: OriginalLiftedCallContext<'_>,
    collected: &mut LiftedCallCollection,
    binding: &crate::command_binding::SourceInvocationBinding,
) -> Option<()> {
    let held = binding.original_native_source_input_for_name_roles(tokens, registry)?;
    let input = context.input()?;
    if input != held && input != &held.for_nested_source() {
        return None;
    }
    let words = binding.original_native_source_words(tokens, held, registry)?;
    let crate::registry_invocation::source_structure::OriginalRegistrySource::SourceTransitions(
        advice,
    ) = words.source()
    else {
        return None;
    };
    let (roles, traits) =
        words.with_source_schema(held.borrowed_context_registry(), |schema| {
            let facts = schema.facts();
            if !facts.arg_roles_complete
                || words.origins().iter().enumerate().any(|(ordinal, origin)| {
                    *origin != crate::registry_invocation::InvocationWordOrigin::Written(ordinal)
                })
                || advice
                    .original_words()
                    .iter()
                    .any(|word| word.group().expand)
            {
                return None;
            }
            Some((
                facts
                    .arg_roles
                    .into_iter()
                    .map(|(ordinal, role)| (facts.argument_offset + usize::from(ordinal), role))
                    .collect::<Vec<_>>(),
                schema.semantics.traits,
            ))
        })??;
    collect_original_braced_expression_calls(
        tokens,
        config,
        advice.site(),
        &roles,
        traits,
        context.input()?.analyser_profile(),
        collected,
    )
}

type OriginalExpressionRoles = (Vec<(usize, tcl_registry::ArgRole)>, tcl_registry::Traits);

fn logical_expression_roles(
    words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
    input: &crate::analyser::ResolvedAnalysisInput,
) -> Option<OriginalExpressionRoles> {
    words.with_source_schema(input.borrowed_context_registry(), |schema| {
        let projection = schema.authored_logical_source_role_projection()?;
        if projection.selection_unknown()
            || projection.argument_roles().iter().any(|(argument, role)| {
                *role == tcl_registry::ArgRole::Expr
                    && !matches!(
                        words
                            .origins()
                            .get(argument.checked_add(1).unwrap_or(usize::MAX)),
                        Some(crate::registry_invocation::InvocationWordOrigin::Written(_))
                    )
            })
            || (schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
                && words.origins().iter().any(|origin| {
                    matches!(
                        origin,
                        crate::registry_invocation::InvocationWordOrigin::BindingPrefix(_)
                    )
                }))
        {
            return None;
        }
        Some((words.written_argument_roles(), schema.semantics.traits))
    })?
}

fn selected_expression_roles(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    logical_source: bool,
    tokens: &CommandTokens,
    metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
    registry: &tcl_registry::CommandRegistry,
) -> Option<OriginalExpressionRoles> {
    let roles = if logical_source {
        let realm = tokens
            .source_binding
            .as_ref()?
            .original_source_realm_for_input(tokens, metadata.source_analysis_input()?, registry)?;
        let projection = invocation.with_metadata_schema(registry, metadata, realm, |schema| {
            schema.authored_logical_source_role_projection()
        })?;
        if projection.selection_unknown() {
            return None;
        }
        // Captured Expr words belong to their original producer, rather
        // than this caller's written braced-word inventory.
        if projection.argument_roles().iter().any(|(argument, role)| {
            *role == tcl_registry::ArgRole::Expr
                && invocation.effective.written_argument(*argument).is_none()
        }) {
            return None;
        }
        projection
            .argument_roles()
            .iter()
            .filter_map(|(argument, role)| {
                invocation
                    .effective
                    .written_argument(*argument)
                    .map(|written| (written, *role))
            })
            .collect()
    } else {
        // Native inventory keeps its separate complete-role permission.
        if !invocation.facts.arg_roles_complete {
            return None;
        }
        invocation.written_argument_roles()
    };
    if !invocation.effective.binding_prefix.is_empty()
        && (invocation
            .facts
            .traits
            .contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS)
            || (!logical_source
                && invocation
                    .facts
                    .arg_roles
                    .iter()
                    .any(|(_, role)| *role == tcl_registry::ArgRole::Expr)))
    {
        return None;
    }
    Some((roles, invocation.facts.traits))
}

fn collect_original_expression_calls(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
    context: OriginalLiftedCallContext<'_>,
    collected: &mut LiftedCallCollection,
) -> Option<()> {
    let has_braced_commands = tokens
        .words()
        .iter()
        .any(|word| matches!(word, WordExpr::BracedLiteral { text, .. } if text.contains('[')));
    // The enclosing and nested original words are authenticated by the
    // caller. Handler roles are needed only to enter a written braced Expr.
    if !has_braced_commands {
        return Some(());
    }
    let binding = tokens.source_binding.as_ref()?;
    if binding.has_original_native_source_command() {
        return collect_native_expression_calls(
            tokens, config, registry, context, collected, binding,
        );
    }
    let site = match context {
        OriginalLiftedCallContext::Metadata(_) => binding.invocation_site()?,
        OriginalLiftedCallContext::LogicalInput(_) => {
            binding.original_logical_source_site_for_name_roles(tokens, registry)?
        }
    };
    if let Some(input) = context.input()
        && let Some(words) = tokens
            .source_binding
            .as_ref()?
            .original_logical_source_words(tokens, input, registry)
    {
        let (roles, traits) = logical_expression_roles(&words, input)?;
        return collect_original_braced_expression_calls(
            tokens,
            config,
            site,
            &roles,
            traits,
            input.analyser_profile(),
            collected,
        );
    }
    let OriginalLiftedCallContext::Metadata(metadata) = context else {
        return None;
    };
    let logical_source = tokens
        .source_binding
        .as_ref()?
        .original_source_advice_purpose(tokens, metadata, registry)?
        == crate::command_binding::OriginalInvocationSourcePurpose::Logical;
    let invocation = if logical_source {
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            registry, metadata, tokens,
        )
    } else {
        crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
            registry,
            Some(metadata),
            tokens,
        )
        .or_else(|| {
            crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                registry,
                Some(metadata),
                tokens,
            )
        })
    };
    let (roles, traits) = if let Some(invocation) = invocation {
        selected_expression_roles(&invocation, logical_source, tokens, metadata, registry)?
    } else if let Some(invocation) = (!logical_source)
        .then(|| {
            crate::registry_invocation::logical_structured_invocation_with_metadata_context(
                registry, metadata, tokens, None,
            )
        })
        .flatten()
    {
        (invocation.written_roles(), invocation.traits())
    } else {
        if tokens
            .source_binding
            .as_ref()?
            .execution_targets()
            .any(|target| target.registry_backed)
        {
            return None;
        }
        return (!has_braced_commands).then_some(());
    };
    collect_original_braced_expression_calls(
        tokens,
        config,
        site,
        &roles,
        traits,
        metadata.source_analysis_input()?.analyser_profile(),
        collected,
    )
}

#[derive(Clone, Copy)]
enum OriginalExpressionInventoryPurpose<'a> {
    Configured(&'a tcl_dialect::DialectProfile),
    // The inputless source caller retains an unconditional unknown residual.
    AuthoredCandidates,
}

fn collect_original_braced_expression_calls(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
    site: &crate::command_binding::CommandAllocationSite,
    roles: &[(usize, tcl_registry::ArgRole)],
    traits: tcl_registry::Traits,
    profile: &tcl_dialect::DialectProfile,
    collected: &mut LiftedCallCollection,
) -> Option<()> {
    collect_original_braced_expression_calls_in_image(
        tokens,
        config,
        site.source.source_image(),
        roles,
        traits,
        OriginalExpressionInventoryPurpose::Configured(profile),
        collected,
    )
}

fn collect_original_braced_expression_calls_in_image(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
    image: &tcl_lexer::SourceImage,
    roles: &[(usize, tcl_registry::ArgRole)],
    traits: tcl_registry::Traits,
    purpose: OriginalExpressionInventoryPurpose<'_>,
    collected: &mut LiftedCallCollection,
) -> Option<()> {
    let has_braced_commands = tokens
        .words()
        .iter()
        .any(|word| matches!(word, WordExpr::BracedLiteral { text, .. } if text.contains('[')));
    let concatenated = traits.contains(tcl_registry::Traits::EXPR_CONCATENATES_ARGS);
    if concatenated && has_braced_commands && tokens.words().len() != 2 {
        return None;
    }
    for (ordinal, word) in tokens.words().iter().skip(1).enumerate() {
        if !concatenated && !roles.contains(&(ordinal, tcl_registry::ArgRole::Expr)) {
            continue;
        }
        let WordExpr::BracedLiteral { text, source } = word else {
            continue;
        };
        let base = source.span.start().checked_add(1)?;
        let end = usize::try_from(base).ok()?.checked_add(text.len())?;
        if source.provenance != Provenance::Source
            || image.bytes().get(usize::try_from(base).ok()?..end) != Some(text.as_bytes())
        {
            return None;
        }
        match purpose {
            OriginalExpressionInventoryPurpose::AuthoredCandidates => {
                collect_surface_text_with_surface(text, Some(base), config, None, 0, collected);
            }
            OriginalExpressionInventoryPurpose::Configured(profile) => {
                let mut regions = OriginalSourceDependencyRegions {
                    substitutions: Vec::new(),
                    scripts: Vec::new(),
                    unknown: false,
                };
                collect_original_expression_dependency_regions(
                    image,
                    config,
                    Some(profile),
                    vec![Span::new(base, u32::try_from(end).ok()?)],
                    &mut regions,
                )?;
                if regions.unknown {
                    return None;
                }
                for body in regions.substitutions {
                    let span = Span::new(body.start().checked_sub(1)?, body.end().checked_add(1)?);
                    let spelling = image.try_text().ok()?.get(span.as_range())?;
                    push_substitution(
                        spelling,
                        &SourceSite::source(span),
                        config,
                        None,
                        0,
                        collected,
                    );
                }
            }
        }
    }
    Some(())
}
