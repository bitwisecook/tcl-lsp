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

//! Command substitutions nested inside a statement's words.
//!
//! A `[cmd …]` written as a whole statement is an IR statement the shimmer
//! detectors walk. The same `[cmd …]` written inside another command's word
//! is not: `Statement::Call` keeps its arguments as flat text, so
//! `puts [lindex $x 0]` is one opaque call and the `lindex` inside it is
//! invisible. The runtime draws no such distinction — it evaluates the
//! substitution either way — so the gap costs twice over:
//!
//! 1. the conversion at the nested site is never reported; and
//! 2. worse, it never reaches the commit state, so *every later read of that
//!    variable* is judged against a stale representation. Unlifted,
//!    `set x [llength $l]` then `puts [lindex $x 0]` then `incr x` reports
//!    nothing at all, where the same code with `lindex $x 0` on its own line
//!    reports both halves.
//!
//! Builds on [`crate::word_expr`], which owns splitting one word into its
//! substitution components; this module is the statement-level view of that —
//! which *commands* a statement's words run, and in what order.
//!
//! ## Why this reads `word_exprs` and not the argument text
//!
//! Tcl substitutes `[…]` in bare and `"…"`-quoted words but **not** in braced
//! ones, and `Statement::Call::args` cannot tell them apart: `puts [lindex $x 0]`
//! and `puts {[lindex $x 0]}` both arrive as the single argument text
//! `[lindex $x 0]`. Lifting from that text would report a command Tcl never
//! runs. [`CommandTokens::word_exprs`](crate::ir::CommandTokens::word_exprs) is
//! the structured per-word syntax the segmenter already derived — it models the
//! braced word as [`WordExpr::BracedLiteral`] and the substituted one as
//! [`WordExpr::CommandSubstitution`] — so reading it is both correct and free of
//! any re-lexing.
//!
//! ## Order
//!
//! Substitutions are yielded innermost-first, then left to right — Tcl's own
//! evaluation order, so the commit state moves exactly as the runtime converts.
//! In `foo [bar [baz $x]]` the reads of `baz` land before `bar`'s.

use tcl_lexer::Span;

use tcl_registry::model::DocumentCommandSurface;

use crate::ir::{CommandTokens, Provenance, SourceSite, WordExpr, WordPart};

/// Original lexical values used by single-command source emitters.
/// These retain raw escapes and grouping; they grant no command binding.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub struct SingleCommandSourcePart {
    pub text: String,
    pub braced: bool,
    pub expand: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub enum SingleCommandSourcePurpose {
    Script,
    CommandSubstitution,
}

/// Project one complete command through the shared configured word owner.
/// Missing geometry, malformed tails and multiple commands remain unavailable.
#[doc(hidden)]
#[must_use]
pub fn single_command_source_parts(
    text: &str,
    config: tcl_lexer::LexerConfig,
    purpose: SingleCommandSourcePurpose,
) -> Option<Vec<SingleCommandSourcePart>> {
    let image = tcl_lexer::SourceImage::document(text);
    let end = u32::try_from(text.len()).ok()?;
    let region = if purpose == SingleCommandSourcePurpose::CommandSubstitution
        && text.starts_with('[')
        && text.ends_with(']')
    {
        Span::new(1, end.checked_sub(1)?)
    } else {
        Span::new(0, end)
    };
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        image,
        region,
        config,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() > 1 {
        return None;
    }
    let Some(command) = plan.commands.first() else {
        return Some(Vec::new());
    };
    command
        .words
        .iter()
        .map(|word| {
            let span = word.content_span().ok()?;
            let text = std::str::from_utf8(word.image().bytes().get(span.as_range())?)
                .ok()?
                .to_owned();
            Some(SingleCommandSourcePart {
                text,
                braced: word.group().kind == tcl_lexer::WordKind::Braced,
                expand: word.group().expand,
            })
        })
        .collect()
}

/// How deep a nest of `[cmd [cmd …]]` this walks before giving up. Tcl's own
/// parser caps nesting far above anything hand-written; this only has to stop
/// a generated pathological input from recursing without bound.
const MAX_SUBSTITUTION_DEPTH: u32 = 8;

/// One `[cmd …]` substitution lifted out of a word, shaped like the direct
/// invocation it behaves as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftedCall {
    /// The substitution's command word, as spelled. Dispatch identity belongs
    /// to the retained source proof in `tokens`, rather than this spelling.
    pub command: String,
    /// Retained structured command words and their exact dispatch proof, when queried.
    pub tokens: Option<CommandTokens>,
    /// The substitution's argument words, index-aligned with `arg_spans`.
    pub args: Vec<String>,
    /// Absolute source span of each argument word.
    pub arg_spans: Vec<Span>,
    /// The structured syntax of each argument word, index-aligned with
    /// [`Self::args`] — so a consumer can tell a braced literal from a word
    /// that substitutes without re-deciding it from the text.
    ///
    /// Empty when [`nested_command_words`] declined the substitution, or when
    /// its word count disagreed with the argument split; a consumer that reads
    /// this must keep working without it.
    pub arg_words: Vec<WordExpr>,
    /// The substitution's own words as [`nested_command_words`] recovers
    /// them, the command word at `0`; `None` when it declined.
    pub words: Option<CommandTokens>,
    /// Absolute source span of the whole `[…]`.
    pub span: Span,
}

#[derive(Default)]
struct LiftedCallCollection {
    calls: Vec<LiftedCall>,
    incomplete: bool,
    original_image: Option<tcl_lexer::SourceImage>,
}

impl LiftedCallCollection {
    fn for_original_parent(tokens: &CommandTokens, config: tcl_lexer::LexerConfig) -> Self {
        let original_image = (|| {
            let binding = tokens.source_binding.as_ref()?;
            (binding.original_argument_lexer_config(tokens)? == config).then_some(())?;
            let origin = binding.original_argument_source(tokens)?;
            origin.original_lexical_command(
                tokens.words(),
                tokens.argv.first()?.start(),
                config,
            )?;
            Some(origin.source_image().clone())
        })();
        Self {
            original_image,
            ..Self::default()
        }
    }
}

/// Original syntax regions that may contribute memo-summary dependencies.
/// The caller still owns current Module/target selection and every residual.
pub(crate) struct OriginalSourceDependencyRegions {
    pub(crate) substitutions: Vec<Span>,
    pub(crate) scripts: Vec<Span>,
    pub(crate) unknown: bool,
}

/// Original complete words and selected source roles distinguish evaluated
/// syntax from bracket text in data. No invocation or frame is admitted.
pub(crate) fn original_source_dependency_regions(
    tokens: &CommandTokens,
    words: &[tcl_lexer::NativeWord],
    registry: &tcl_registry::CommandRegistry,
    profile: Option<&tcl_dialect::DialectProfile>,
) -> Option<OriginalSourceDependencyRegions> {
    if tokens.synthetic.is_some() || !tokens.original_words_match_argv_text() {
        return None;
    }
    let first = words.first()?;
    let image = first.image();
    let config = first.config();
    let original = crate::registry_invocation::original_native_compiler_words(
        image,
        tokens.words(),
        first.span().start(),
        config,
    )?;
    (original == words).then_some(())?;
    let mut out = OriginalSourceDependencyRegions {
        substitutions: Vec::new(),
        scripts: Vec::new(),
        unknown: false,
    };
    let mut expressions = Vec::new();
    for word in words {
        dependency_parts(word.executable_parts(), &mut expressions, &mut out);
    }
    let selected = crate::registry_invocation::original_source_name_roles(registry, tokens);
    if words.iter().any(|word| word.group().expand)
        || selected
            .as_ref()
            .is_some_and(|selected| !selected.direct_written_arguments())
    {
        out.unknown = true;
        return Some(out);
    }
    if let Some(selected) = selected.as_ref() {
        out.unknown |= !selected.roles_complete()
            && words
                .iter()
                .skip(1)
                .any(|word| word.bytes().contains(&b'['));
        if selected
            .facts()
            .traits
            .contains(tcl_registry::Traits::PERFORMS_SUBSTITUTION)
        {
            out.unknown = true;
        }
        for &(argument, role) in selected.roles() {
            if !role.carries_script() && role != tcl_registry::ArgRole::CommandPrefix {
                continue;
            }
            let word = words.get(argument.checked_add(1)?)?;
            let content = word.content_span().ok()?;
            let original = image.bytes().get(content.as_range())?;
            let unchanged = tcl_syntax::word_rules::original_static_word_unicode_value(word)
                .is_some_and(|value| value.as_slice() == original);
            if !unchanged || word.group().expand {
                out.unknown = true;
                continue;
            }
            if !original.contains(&b'[') {
                continue;
            }
            out.unknown |= selected.has_unknown_residual();
            match role {
                tcl_registry::ArgRole::Expr => expressions.push(content),
                tcl_registry::ArgRole::Body
                    if selected.facts().body_kind == tcl_registry::BodyKind::Plain
                        && selected.facts().body_interpreter
                            == tcl_registry::BodyInterpreter::Current =>
                {
                    out.scripts.push(content);
                }
                _ => out.unknown = true,
            }
        }
    } else {
        out.unknown |= words
            .iter()
            .skip(1)
            .any(|word| word.bytes().contains(&b'['));
    }
    collect_original_expression_dependency_regions(image, config, profile, expressions, &mut out)?;
    Some(out)
}

fn collect_original_expression_dependency_regions(
    image: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
    profile: Option<&tcl_dialect::DialectProfile>,
    mut expressions: Vec<Span>,
    out: &mut OriginalSourceDependencyRegions,
) -> Option<()> {
    expressions.retain(|span| {
        image
            .bytes()
            .get(span.as_range())
            .is_some_and(|bytes| bytes.contains(&b'['))
    });
    let mut depth = 0;
    while !expressions.is_empty() {
        if crate::depth_guard::MAX_BRACKET_TEXT_DEPTH.exceeded(depth) {
            out.unknown = true;
            break;
        }
        let Some(profile) = profile else {
            out.unknown = true;
            break;
        };
        let mut nested = Vec::new();
        for span in expressions {
            let expression = image.bytes().get(span.as_range())?;
            let (tokens, unknown) = tcl_lexer::tokenise_expr_bytes_checked_with_expression_grammar(
                expression,
                &config.grammar_over(profile.grammar),
                profile.expr_grammar_base,
                profile.f5_core_expr_grammar(),
            );
            out.unknown |= unknown;
            let Some(terms) = tcl_lexer::expression_terms(expression, &tokens, config) else {
                out.unknown = true;
                continue;
            };
            for term in terms {
                let relative = match term.kind {
                    tcl_lexer::ExprTermKind::Variable
                    | tcl_lexer::ExprTermKind::IndexedVariable
                    | tcl_lexer::ExprTermKind::Command => term.source,
                    tcl_lexer::ExprTermKind::EscapedString => term.value,
                    tcl_lexer::ExprTermKind::Number
                    | tcl_lexer::ExprTermKind::Boolean
                    | tcl_lexer::ExprTermKind::String => continue,
                };
                let part = Span::new(
                    span.start().checked_add(relative.start)?,
                    span.start().checked_add(relative.end)?,
                );
                let arena = tcl_lexer::ExecutablePartArena::decompose(
                    image.clone(),
                    part,
                    tcl_lexer::word_parts::SubstFlags::default(),
                    config,
                )
                .ok()?;
                dependency_parts(&arena, &mut nested, out);
            }
        }
        expressions = nested;
        depth += 1;
    }
    Some(())
}

fn dependency_parts(
    arena: &tcl_lexer::ExecutablePartArena,
    expressions: &mut Vec<Span>,
    out: &mut OriginalSourceDependencyRegions,
) {
    for part in arena.all_parts() {
        match part.part {
            tcl_lexer::ExecutablePart::Command { body } => out.substitutions.push(body),
            tcl_lexer::ExecutablePart::Expression { expression } => expressions.push(expression),
            tcl_lexer::ExecutablePart::ParseError(_) => out.unknown = true,
            tcl_lexer::ExecutablePart::Text(_) | tcl_lexer::ExecutablePart::Variable { .. } => {}
        }
    }
}

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
    words: Vec<tcl_lexer::NativeWord>,
    registry: tcl_registry::RegistrySnapshot,
    dialect: Option<tcl_registry::InvocationDialect>,
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
enum OriginalLiftedCallContext<'a> {
    Metadata(crate::registry_invocation::InvocationMetadataContext<'a>),
    LogicalInput(&'a crate::analyser::ResolvedAnalysisInput),
}

impl<'a> OriginalLiftedCallContext<'a> {
    fn input(self) -> Option<&'a crate::analyser::ResolvedAnalysisInput> {
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

fn original_call_context(
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

/// Every command substitution nested in `tokens`' words, innermost-first.
///
/// Returns an empty vector for a statement whose words hold no substitution —
/// the overwhelming majority — without allocating beyond the empty `Vec`.
#[must_use]
pub fn lifted_calls(
    tokens: Option<&CommandTokens>,
    config: tcl_lexer::LexerConfig,
) -> Vec<LiftedCall> {
    lift(tokens, config, None).calls
}

/// A reached expression evaluator inside an original written operand.
/// This carries no dispatch or normal-completion authority for its parent.
pub(crate) struct EnteredExpressionEvaluation {
    expression: crate::expr_ast::ExprNode,
    expression_base: u32,
    span: Span,
    numbers: tcl_dialect::NumberSyntax,
    grammar: tcl_dialect::LexerGrammar,
}

impl EnteredExpressionEvaluation {
    pub(crate) fn expression(&self) -> &crate::expr_ast::ExprNode {
        &self.expression
    }

    pub(crate) fn expression_base(&self) -> u32 {
        self.expression_base
    }

    pub(crate) fn span(&self) -> Span {
        self.span
    }

    pub(crate) fn numbers(&self) -> tcl_dialect::NumberSyntax {
        self.numbers
    }

    pub(crate) fn grammar(&self) -> tcl_dialect::LexerGrammar {
        self.grammar
    }
}

/// Recover entered child evaluation independently of the enclosing handler.
/// Only the child's exact source binding supplies its implementation and syntax.
pub(crate) fn entered_expression_evaluations(
    tokens: Option<&CommandTokens>,
    config: tcl_lexer::LexerConfig,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<EnteredExpressionEvaluation> {
    lifted_calls(tokens, config)
        .into_iter()
        .filter_map(|call| entered_expression_evaluation(&call.tokens?, registry))
        .collect()
}

fn entered_expression_evaluation(
    tokens: &CommandTokens,
    registry: &tcl_registry::CommandRegistry,
) -> Option<EnteredExpressionEvaluation> {
    let binding = tokens.source_binding.as_ref()?;
    if binding.runtime_reachability() != crate::command_binding::SourceRuntimeReachability::Reached
    {
        return None;
    }
    let site = binding.invocation_site()?;
    let invocation =
        crate::registry_invocation::resolved_handler_invocation(registry, None, tokens)?;
    if invocation.facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
        || invocation.facts.argument_offset != 0
        || tokens.words().len() != 2
    {
        return None;
    }
    let word = tokens.words().get(1)?;
    let WordExpr::BracedLiteral { text, source } = word else {
        return None;
    };
    if source.provenance != Provenance::Source {
        return None;
    }
    let head = tokens.words().first()?.source();
    if head.provenance != Provenance::Source || head.span.start() != site.offset {
        return None;
    }
    let base = crate::lowering_hooks::word_content_base(
        *tokens.argv.get(1)?,
        tokens.single_token_word.get(1).copied()?,
        text,
    )?;
    let length = u32::try_from(text.len()).ok()?;
    let end = base.checked_add(length)?;
    if site
        .source
        .source_image()
        .bytes()
        .get(base as usize..end as usize)
        != Some(text.as_bytes())
    {
        return None;
    }
    let dialect = binding.variable_context.invocation_dialect?;
    let parser = dialect.expression_parse_context(None);
    Some(EnteredExpressionEvaluation {
        expression: crate::expr_parser::parse_expr_with_syntax_context(text, &parser),
        expression_base: base,
        span: Span::new(head.span.start(), word.source().span.end()),
        numbers: dialect.numbers,
        grammar: parser.lexer_grammar,
    })
}

/// [`lifted_calls_with_surface`] over one word rather than a whole command.
///
/// A CFG terminator keeps its value as a single [`WordExpr`] —
/// `Terminator::Return::value_word` — not as a `CommandTokens`, and the
/// commands in it run exactly as they would in a statement.
#[must_use]
pub fn lifted_calls_in_word(
    word: Option<&WordExpr>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
) -> Vec<LiftedCall> {
    let mut out = LiftedCallCollection::default();
    if let Some(word) = word {
        collect_word(word, config, Some(surface), 0, &mut out);
    }
    out.calls
}

/// [`lifted_calls`], plus the substitutions a brace-quoted **expression** word
/// runs.
///
/// A braced word is inert to the command parser, but not to `expr`: it
/// re-parses that text as an expression and a `[…]` in it is a command the
/// statement really runs. Without a command surface there is no way to tell
/// `puts {[f]}` (literal) from `expr {[f]}` (a call), so the plain
/// [`lifted_calls`] has to take the narrow view; a caller that holds one gets
/// the whole set.
///
/// The *surface*, not the bare catalogue: a document's own
/// `# tcl-lsp: stub myexpr {value:expr}` declares an expression word exactly
/// as a shipped command does, and lowering honours that role, so a call
/// `myexpr {[id 9]}` is a call site like any other.
///
/// `return [expr {[fact [expr {$n - 1}] [expr {$n * $acc}]]}]` is the case
/// that matters. Its recursive call was invisible to the caller-evidence scan,
/// so two visible `fact 5 1` / `fact 3 1` sites read as the *complete* caller
/// set and O100 specialised `acc` to `1`: tclsh 9.0.4 prints `120` then `6`,
/// the optimised program printed `1` then `1`, and with a single call site
/// O112 deleted the base case outright and the program no longer terminated
/// (#2118).
#[must_use]
pub fn lifted_calls_with_surface(
    tokens: Option<&CommandTokens>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
) -> Vec<LiftedCall> {
    lift(tokens, config, Some(surface)).calls
}

/// Checked lexical substitutions in retained original source, independently
/// of whether this declaration was entered. Each child retains its own proof;
/// this inventory grants no reached operand or command dispatch authority.
pub(crate) fn original_lifted_calls_with_surface(
    tokens: &CommandTokens,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
) -> Vec<LiftedCall> {
    let Some(site) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.invocation_site())
    else {
        return Vec::new();
    };
    if crate::registry_invocation::native_compiler_replay_source(tokens, site).is_none() {
        return Vec::new();
    }
    let config = tokens.native_lexer_config(config);
    let mut out = LiftedCallCollection::default();
    collect_command_words(&tokens.word_exprs, config, Some(surface), 0, &mut out);
    for call in &mut out.calls {
        if let Some(nested) = &mut call.tokens {
            nested.inherit_nested_bindings(tokens);
        }
    }
    out.calls
}

fn lift(
    tokens: Option<&CommandTokens>,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
) -> LiftedCallCollection {
    let mut out = LiftedCallCollection::default();
    let Some(tokens) = tokens else {
        return out;
    };
    if !tokens.evaluates_words() {
        return out;
    }
    let config = tokens.native_lexer_config(config);
    out = LiftedCallCollection::for_original_parent(tokens, config);
    collect_command_words(&tokens.word_exprs, config, surface, 0, &mut out);
    for lifted in &mut out.calls {
        if let Some(nested) = &mut lifted.tokens {
            nested.inherit_nested_bindings(tokens);
        }
    }
    out
}

/// Push every substitution one command's words evaluate: the `[…]` each
/// substituting word carries, then — when the surface says so — the `[…]`
/// inside a brace-quoted word the head evaluates as an expression.
fn collect_command_words(
    words: &[WordExpr],
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    for word in words {
        collect_word(word, config, surface, depth, out);
    }
    let Some(surface) = surface else {
        return;
    };
    // This lexical traversal asks only for assistance roles. Actual nested
    // dispatch proofs are attached to the lifted token carriers afterwards.
    let Some(WordExpr::Literal { text: head, .. }) = words.first() else {
        return;
    };
    let args: Vec<String> = words.iter().skip(1).map(WordExpr::legacy_text).collect();
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    for index in crate::ir_helpers::in_frame_expression_arg_indices(head, &arg_refs, surface) {
        // An unbraced expression word already substituted at the command
        // level, so the loop above has seen its `[…]`; descending again would
        // report the same call twice.
        let Some(WordExpr::BracedLiteral { text, source }) = words.get(index + 1) else {
            continue;
        };
        // The word's site covers the `{…}` that quoted it, so its content
        // starts one byte in. A word whose site is not direct source carries
        // no offset worth anchoring to.
        let base = (source.provenance == Provenance::Source)
            .then(|| source.span.start().saturating_add(1));
        collect_surface_text(text, base, config, surface, depth, out);
    }
}

/// Push every `[…]` an evaluated **source surface** runs.
///
/// A surface is text Tcl substitutes but which reached the IR without words of
/// its own: an expression operand, an `incr` amount, a `return` value. It is
/// not a script, so it is not segmented — the lexer's own `Cmd` tokens are
/// exactly its bracket substitutions, spans included, and each is handed to
/// the same [`push_substitution`] a word-level `[…]` takes.
///
/// `base` is the absolute offset of `text`'s first byte when the surface is a
/// verbatim source slice. Without one the recovered spans are relative to the
/// surface, so a consumer that reports positions must supply it.
#[must_use]
pub fn lifted_calls_in_text(
    text: &str,
    base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
) -> Vec<LiftedCall> {
    let mut out = LiftedCallCollection::default();
    collect_surface_text(text, base, config, surface, 0, &mut out);
    out.calls
}

fn collect_surface_text(
    text: &str,
    base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    collect_surface_text_with_surface(text, base, config, Some(surface), depth, out);
}

fn collect_surface_text_with_surface(
    text: &str,
    base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        out.incomplete = true;
        return;
    }
    let Ok(tokens) = tcl_lexer::Lexer::with_config(text, config.nested()).tokenise_all() else {
        out.incomplete = true;
        return;
    };
    let provenance = if base.is_some() {
        Provenance::Source
    } else {
        Provenance::Opaque
    };
    let base = base.unwrap_or(0);
    for token in tokens
        .iter()
        .filter(|token| token.kind == tcl_lexer::TokenType::Cmd)
    {
        let start = token.span.start() as usize;
        // A `Cmd` token's span runs to the last byte *inside* the brackets;
        // `push_substitution` parses a whole `[…]`, so the closing bracket is
        // taken back. Without one the `[` never closed, which is a parse error
        // Tcl raises rather than a command it runs.
        let end = token.span.end() as usize;
        let close = if text.as_bytes().get(end) == Some(&b']') {
            end + 1
        } else {
            end
        };
        let Some(spelling) = text.get(start..close).filter(|s| s.ends_with(']')) else {
            continue;
        };
        let site = SourceSite {
            span: Span::new(
                base.saturating_add(token.span.start()),
                base.saturating_add(u32::try_from(close).unwrap_or(u32::MAX)),
            ),
            provenance: provenance.clone(),
        };
        push_substitution(spelling, &site, config, surface, depth + 1, out);
    }
}

/// Every command substitution an expression evaluates.
///
/// A fused `AssignExpr` or `ExprEval` statement, and a CFG `Branch`
/// terminator, keep a parsed expression instead of words — `set r [expr {[f]}]`
/// and `if {[f] > 5} …` reach the IR with no `CommandTokens` at all, so a
/// walk that reads only words sees neither the `f` they run nor the caller
/// evidence it carries (#2118).
///
/// `expr_base` is the absolute offset of the expression text's first byte,
/// as [`crate::ir::IfClause::condition_base`] records it; see
/// [`lifted_calls_in_text`] for what its absence costs.
#[must_use]
pub fn lifted_calls_in_expr(
    expr: &crate::expr_ast::ExprNode,
    expr_base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
) -> Vec<LiftedCall> {
    let mut out = LiftedCallCollection::default();
    collect_expr_node(expr, expr_base, config, surface, 0, &mut out);
    out.calls
}

fn collect_expr_node(
    expr: &crate::expr_ast::ExprNode,
    expr_base: Option<u32>,
    config: tcl_lexer::LexerConfig,
    surface: &DocumentCommandSurface<'_>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    use crate::expr_ast::ExprNode;

    if depth > MAX_SUBSTITUTION_DEPTH {
        out.incomplete = true;
        return;
    }
    let mut descend = |node| collect_expr_node(node, expr_base, config, surface, depth + 1, out);
    match expr {
        ExprNode::Command { text, start, .. } => {
            let site = match expr_base {
                Some(base) => SourceSite::source(Span::new(
                    base.saturating_add(*start),
                    base.saturating_add(*start)
                        .saturating_add(u32::try_from(text.len()).unwrap_or(0)),
                )),
                None => SourceSite::opaque(Span::new(0, 0)),
            };
            push_substitution(text, &site, config, Some(surface), depth, out);
        }
        ExprNode::Binary { left, right, .. } => {
            descend(left);
            descend(right);
        }
        ExprNode::Unary { operand, .. } => descend(operand),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            descend(condition);
            descend(true_branch);
            descend(false_branch);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                descend(arg);
            }
        }
        // A word already reduced to its value: braced, its brackets are data
        // and never run; unbraced, whatever substitution it still owes really
        // is executed. The same split [`crate::ir_helpers`] makes for the
        // variable-effect walk.
        ExprNode::CompiledWord { text, braced } => {
            if !*braced {
                collect_surface_text(text, None, config, surface, depth + 1, out);
            }
        }
        // The parse gave up; its text is still evaluated, so it is read as a
        // surface rather than treated as running nothing.
        ExprNode::Raw { text } => {
            collect_surface_text(text, expr_base, config, surface, depth + 1, out);
        }
        // `"…"` substitutes inside an expression, `{…}` does not, and this
        // variant carries the source text *including* its delimiters — so the
        // delimiter is what decides. Reading every string as inert missed the
        // call in `set r [expr {"[id 9]"}]`: tclsh 8.6.18 prints `7 9` and
        // `--profile standard` folded `id`'s body to `return 7`, printing
        // `7 7` (#2118, found in review).
        ExprNode::String { text, start, .. } => {
            if let Some(inner) = quoted_operand_body(text) {
                let base = expr_base.map(|base| base.saturating_add(*start).saturating_add(1));
                collect_surface_text(inner, base, config, surface, depth + 1, out);
            }
        }
        ExprNode::Literal { .. } | ExprNode::Var { .. } => {}
    }
}

/// The body of a `"…"` expression operand that may run a command, or `None`
/// for a `{…}` one or one with no `[`.
///
/// The quoted/braced rule is [`tcl_syntax::expr::quoted_string_body`]'s:
/// tclsh 8.6.18 and 9.0.4 both print `2` then `2` for
/// `set x 1; puts [expr {"[incr x]"}]; puts $x`, and `1` then `1` for the
/// braced `{[incr x]}`.
pub(crate) fn quoted_operand_body(text: &str) -> Option<&str> {
    tcl_syntax::expr::quoted_string_body(text).filter(|inner| inner.contains('['))
}

/// Walk one word, pushing every substitution it evaluates.
fn collect_word(
    word: &WordExpr,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        out.incomplete = true;
        return;
    }
    match word {
        // The whole word is `[cmd …]`.
        WordExpr::CommandSubstitution { spelling, source } => {
            push_substitution(spelling, source, config, surface, depth, out);
        }
        // A compound word — `"[cmd …]"`, `a[cmd …]b`, `$v[cmd …]` — whose
        // parts evaluate left to right.
        WordExpr::Template { parts, .. } => {
            for part in parts {
                match part {
                    WordPart::CommandSubstitution { spelling, source } => {
                        push_substitution(spelling, source, config, surface, depth, out);
                    }
                    WordPart::Opaque { .. } => out.incomplete = true,
                    WordPart::Variable { spelling, source } => {
                        collect_variable_index(spelling, source, config, surface, depth, out);
                    }
                    WordPart::Text { .. } => {}
                }
            }
        }
        // `{*}[cmd …]` still evaluates the substitution before expanding it.
        WordExpr::Expand { word, .. } => collect_word(word, config, surface, depth, out),
        // A braced word's `[…]` is literal to the command parser — reading
        // `word_exprs` rather than the argument text is the whole point of
        // knowing that. Whether the command it is an argument *to* re-parses
        // it as an expression is `collect_command_words`' question, not this
        // one. Variable indices have their own configured component owner.
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => {}
        WordExpr::Variable { spelling, source } => {
            collect_variable_index(spelling, source, config, surface, depth, out);
        }
        WordExpr::Opaque { .. } => out.incomplete = true,
    }
}

/// Original variable components retain their own substituted index lists.
/// Only the checked original parent supplies this arena's image and channel.
/// Lexical children retain separate source-binding and dispatch requirements.
fn collect_variable_index(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    use tcl_lexer::ExecutablePart;
    if !variable_index_may_run_command(spelling, config) {
        return;
    }
    let Ok(length) = u32::try_from(spelling.len()) else {
        out.incomplete = true;
        return;
    };
    if source.provenance != Provenance::Source || source.span.len() != length {
        out.incomplete = true;
        return;
    }
    let Some(image) = out.original_image.clone() else {
        out.incomplete = true;
        return;
    };
    if image.bytes().get(source.span.as_range()) != Some(spelling.as_bytes()) {
        out.incomplete = true;
        return;
    }
    let extent = source.span;
    let Ok(arena) = tcl_lexer::ExecutablePartArena::decompose(
        image.clone(),
        extent,
        tcl_lexer::word_parts::SubstFlags::default(),
        config,
    ) else {
        out.incomplete = true;
        return;
    };
    let [root] = arena.list(arena.root()) else {
        out.incomplete = true;
        return;
    };
    if root.span != extent || !matches!(root.part, ExecutablePart::Variable { .. }) {
        out.incomplete = true;
        return;
    }
    let mut pending = vec![arena.list(arena.root()).iter()];
    while let Some(parts) = pending.last_mut() {
        let Some(part) = parts.next() else {
            pending.pop();
            continue;
        };
        match part.part {
            ExecutablePart::Variable {
                index: Some(index), ..
            } => pending.push(arena.list(index).iter()),
            ExecutablePart::Command { .. } => {
                let Some(raw) = image
                    .bytes()
                    .get(part.span.as_range())
                    .and_then(|bytes| std::str::from_utf8(bytes).ok())
                else {
                    out.incomplete = true;
                    return;
                };
                push_substitution(
                    raw,
                    &SourceSite::source(part.span),
                    config,
                    surface,
                    depth,
                    out,
                );
            }
            ExecutablePart::Expression { .. } | ExecutablePart::ParseError(_) => {
                out.incomplete = true;
            }
            ExecutablePart::Variable { index: None, .. } | ExecutablePart::Text(_) => {}
        }
    }
}

// Braced names and escaped index brackets are separate from executable children.
fn variable_index_may_run_command(spelling: &str, config: tcl_lexer::LexerConfig) -> bool {
    if !spelling.starts_with('$') {
        return true;
    }
    match tcl_lexer::word_parts::scan_var_ref(spelling.as_bytes(), 0, config) {
        Ok(Some(reference)) if reference.next == spelling.len() => {
            reference.index.is_some_and(|index| index.contains(&b'['))
        }
        _ => true,
    }
}

/// Parse one `[…]` and push it, deepest first, together with any substitution
/// nested in its own arguments.
fn push_substitution(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
    surface: Option<&DocumentCommandSurface<'_>>,
    depth: u32,
    out: &mut LiftedCallCollection,
) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        out.incomplete = true;
        return;
    }
    let Some((command, args_with_spans)) =
        crate::value_shapes::parse_command_substitution_with_spans_and_config(spelling, config)
    else {
        out.incomplete = true;
        return;
    };
    let base = source.span.start();
    let leading = u32::try_from(spelling.len() - spelling.trim_start().len()).unwrap_or(0);
    let span = Span::new(
        base + leading,
        base + leading + u32::try_from(spelling.trim().len()).unwrap_or(0),
    );

    // The substitution's own words, recovered by the canonical segmenter. Its
    // structure is what decides which of them evaluate a further substitution
    // — `[list "[a]"]` and `[list b[c]d]` run one, `[list {[a]}]` does not —
    // and no test over the flat argument text can tell those apart.
    let nested = nested_command_words(spelling, source, config).ok();
    out.incomplete |= nested.is_none();
    if let Some(tokens) = nested.as_ref() {
        collect_command_words(&tokens.word_exprs, config, surface, depth + 1, out);
    }
    // Index-aligned with `args` (which start at word 1) or empty: the two
    // recoveries split words under the same `LexerConfig`, so a disagreement
    // means one of them declined the shape and the structure is not safe to
    // pair up positionally.
    let arg_words = nested
        .as_ref()
        .and_then(|tokens| tokens.word_exprs.get(1..))
        .filter(|words| words.len() == args_with_spans.len())
        .map_or_default(<[WordExpr]>::to_vec);

    let mut args = Vec::with_capacity(args_with_spans.len());
    let mut arg_spans = Vec::with_capacity(args_with_spans.len());
    for (text, rel) in &args_with_spans {
        args.push(text.clone());
        arg_spans.push(Span::new(base + rel.start(), base + rel.end()));
    }
    out.calls.push(LiftedCall {
        command,
        tokens: nested.clone(),
        args,
        arg_spans,
        arg_words,
        words: nested,
        span,
    });
}

/// Why the words of a `[…]` command substitution could not be recovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NestedWordsDecline {
    /// The spelling is not one complete, non-empty `[…]` command.
    Unmodelled,
    /// It segmented, but carried no words.
    NoWords,
}

/// The structured words of a `[…]` command substitution.
///
/// The word snapshot keeps a command substitution as one opaque spelling, so
/// its inner words are recovered by running the canonical segmenter over the
/// recorded lexical extent — the same segmentation the outer command's words
/// came from, not a bespoke parser. Anything other than exactly one complete
/// command declines.
///
/// The one owner for this: the native and WASM lowerings plan a nested
/// invocation from the same words this module lifts for analysis, and a second
/// recovery that split a word differently would let the two tiers disagree
/// about what a substitution runs.
///
/// # Errors
///
/// Returns [`NestedWordsDecline`] when the spelling is not a single complete
/// `[…]` command, or when it carries no words at all.
pub fn nested_command_words(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
) -> Result<CommandTokens, NestedWordsDecline> {
    // A pure function of its inputs that many consumers ask of the same word
    // in turn: re-segmenting a long substitution for each made it cost its
    // length per consumer (#2422).
    type Recent = (
        String,
        SourceSite,
        tcl_lexer::LexerConfig,
        Result<CommandTokens, NestedWordsDecline>,
    );
    const RECENT_WORDS: usize = 8;
    thread_local! {
        static RECENT: std::cell::RefCell<std::collections::VecDeque<Recent>> =
            const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
    }
    let hit = RECENT.with_borrow_mut(|recent| {
        let at = recent.iter().position(|(text, site, at_config, _)| {
            *at_config == config && site == source && text == spelling
        })?;
        let entry = recent.remove(at)?;
        let answer = entry.3.clone();
        recent.push_front(entry);
        Some(answer)
    });
    if let Some(answer) = hit {
        return answer;
    }
    let answer = nested_command_words_uncached(spelling, source, config);
    RECENT.with_borrow_mut(|recent| {
        recent.truncate(RECENT_WORDS - 1);
        recent.push_front((spelling.to_owned(), source.clone(), config, answer.clone()));
    });
    answer
}

fn nested_command_words_uncached(
    spelling: &str,
    source: &SourceSite,
    config: tcl_lexer::LexerConfig,
) -> Result<CommandTokens, NestedWordsDecline> {
    // Word spellings arrive exact, but a substitution recovered from argument
    // text can carry the whitespace that separated it; the offset the trim
    // drops is added back so spans stay anchored where the word really sits.
    let leading = u32::try_from(spelling.len() - spelling.trim_start().len()).unwrap_or(0);
    let inner = spelling
        .trim()
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
        .ok_or(NestedWordsDecline::Unmodelled)?;
    if inner.trim().is_empty() {
        return Err(NestedWordsDecline::Unmodelled);
    }
    let base = if source.provenance == Provenance::Source {
        source
            .span
            .start()
            .saturating_add(leading)
            .saturating_add(1)
    } else {
        0
    };
    let config = config.nested();
    let segments = crate::segmenter::segment_commands_with_offset_and_config(inner, base, config);
    let [segment] = segments.as_slice() else {
        return Err(NestedWordsDecline::Unmodelled);
    };
    if segment.is_partial {
        return Err(NestedWordsDecline::Unmodelled);
    }
    // The nested script is a sub-lex: its own text, segmented at the document
    // offset it sits at, so the word model reads `inner` and still reports
    // document spans (`SourceMap::with_base` is that sub-lexing contract).
    let sm = tcl_lexer::SourceMap::new(inner).with_base(base, 0, 0);
    let mut tokens = CommandTokens::from_segmented(&sm, config, segment);
    if source.provenance != Provenance::Source {
        for word in &mut tokens.word_exprs {
            retain_nested_provenance(word, &source.provenance);
        }
    }
    if tokens.word_exprs.is_empty() {
        return Err(NestedWordsDecline::NoWords);
    }
    Ok(tokens)
}

fn retain_nested_provenance(word: &mut WordExpr, provenance: &Provenance) {
    let source = match word {
        WordExpr::Template { parts, source, .. } => {
            for part in parts {
                let site = match part {
                    WordPart::Text { source, .. }
                    | WordPart::Variable { source, .. }
                    | WordPart::CommandSubstitution { source, .. }
                    | WordPart::Opaque { source, .. } => source,
                };
                site.provenance.clone_from(provenance);
            }
            source
        }
        WordExpr::Expand { source, word } => {
            retain_nested_provenance(word, provenance);
            source
        }
        WordExpr::Literal { source, .. }
        | WordExpr::BracedLiteral { source, .. }
        | WordExpr::Variable { source, .. }
        | WordExpr::CommandSubstitution { source, .. }
        | WordExpr::Opaque { source, .. } => source,
    };
    source.provenance.clone_from(provenance);
}

/// Recover the structured words for a value word that consists solely of one
/// command substitution.
///
/// A bare `[cmd …]` and a quoted `"[cmd …]"` have different outer syntax but
/// both evaluate the same one command substitution.  Values with literal text,
/// variables, expansion, or a braced word deliberately decline: no caller may
/// infer a command's word form from their flattened value text.
#[must_use]
pub fn whole_word_command_tokens(
    word: &WordExpr,
    config: tcl_lexer::LexerConfig,
) -> Option<CommandTokens> {
    match word {
        WordExpr::CommandSubstitution { spelling, source } => {
            nested_command_words(spelling, source, config).ok()
        }
        WordExpr::Template { parts, .. } => {
            let mut substitution = None;
            for part in parts {
                match part {
                    WordPart::Text { text, .. } if text.is_empty() => {}
                    WordPart::CommandSubstitution { spelling, source }
                        if substitution.is_none() =>
                    {
                        substitution = Some((spelling, source));
                    }
                    _ => return None,
                }
            }
            let (spelling, source) = substitution?;
            nested_command_words(spelling, source, config).ok()
        }
        _ => None,
    }
}

/// A proved nested native expression with its verbatim source anchor.
#[derive(Debug, Clone)]
pub struct LiftedSourceExpression {
    /// Parsed expression selected by the shared invocation owner.
    pub expression: crate::expr_ast::ExprNode,
    /// Extent of the enclosing substitution, for diagnostics.
    pub span: Span,
    /// First byte of the original expression text; transformed text has no anchor.
    pub expression_base: Option<u32>,
    /// Exact concatenated expression bytes and piecewise original read sites.
    pub executed_source: Option<crate::command_binding::ExecutedExpressionSource>,
}

/// Select an expression from proved invocation facts, retaining its exact
/// argument source. Dynamic concatenation cannot be projected into authored
/// expression offsets and yields no positioned expression proof.
#[must_use]
pub(crate) fn source_expression_from_invocation(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    profile: Option<&tcl_dialect::DialectProfile>,
    span: Span,
    parent: Option<crate::command_binding::CommandAllocationSite>,
) -> Option<LiftedSourceExpression> {
    source_expression_from_invocation_parsed(invocation, span, parent, |text| {
        tcl_syntax::expr::parser::parse_expr_for_profile(text, profile)
    })
}

pub(crate) fn source_expression_from_invocation_with_syntax_context(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    parser: tcl_syntax::expr::parser::ExprParseContext,
    span: Span,
    parent: Option<crate::command_binding::CommandAllocationSite>,
) -> Option<LiftedSourceExpression> {
    source_expression_from_invocation_parsed(invocation, span, parent, |text| {
        tcl_syntax::expr::parser::parse_expr_with_syntax_context(text, &parser)
    })
}

pub(crate) fn source_expression_from_invocation_parsed(
    invocation: &crate::registry_invocation::ResolvedStatementInvocation,
    span: Span,
    parent: Option<crate::command_binding::CommandAllocationSite>,
    parse: impl Fn(&str) -> crate::expr_ast::ExprNode,
) -> Option<LiftedSourceExpression> {
    if invocation.facts.operation
        != tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Expr,
        )
    {
        return None;
    }
    if let Some(parent) = parent {
        let values = (0..invocation.arguments.len())
            .map(|index| invocation.argument_literal(index))
            .collect::<Option<Vec<_>>>()?;
        let executed_source = crate::command_binding::ExecutedExpressionSource::from_arguments(
            parent,
            invocation.effective.words.get(1..)?,
            &values,
            invocation.dialect?,
        )?;
        return Some(LiftedSourceExpression {
            expression: parse(&executed_source.text),
            span,
            expression_base: None,
            executed_source: Some(executed_source),
        });
    }
    if invocation.arguments.len() != 1 {
        return None;
    }
    let word = invocation.effective.words.get(1)?;
    let (WordExpr::BracedLiteral { text, .. } | WordExpr::Literal { text, .. }) = word else {
        return None;
    };
    if invocation.argument_literal(0)?.as_str() != text {
        return None;
    }
    Some(LiftedSourceExpression {
        expression: parse(text),
        span,
        expression_base: None,
        executed_source: None,
    })
}

/// Representation expression reads with complete supplied metadata and grammar.
/// Neither input supplies execution, source positions or a native cache by itself.
pub(crate) fn representation_expression_with_metadata_context(
    tokens: &CommandTokens,
    registry: &tcl_registry::CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    parser: tcl_syntax::expr::parser::ExprParseContext,
    span: Span,
) -> Option<LiftedSourceExpression> {
    let invocation =
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            registry, context, tokens,
        )?;
    invocation.source_expression_with_syntax_context(parser, span, expression_parent(tokens))
}

/// Nested normal representation reads using the same actual parent context.
pub(crate) fn lifted_representation_expressions_with_metadata_context(
    tokens: Option<&CommandTokens>,
    registry: &tcl_registry::CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: tcl_lexer::LexerConfig,
    parser: tcl_syntax::expr::parser::ExprParseContext,
) -> Vec<LiftedSourceExpression> {
    lifted_calls(tokens, config)
        .into_iter()
        .filter_map(|lifted| {
            representation_expression_with_metadata_context(
                lifted.tokens.as_ref()?,
                registry,
                context,
                parser,
                lifted.span,
            )
        })
        .collect()
}

fn expression_parent(
    tokens: &CommandTokens,
) -> Option<crate::command_binding::CommandAllocationSite> {
    Some(crate::command_binding::CommandAllocationSite {
        source: std::sync::Arc::clone(tokens.source_binding.as_ref()?.source_origin()?),
        offset: tokens.argv.first()?.start(),
    })
}

/// Nested native expressions selected through actual retained dispatch proofs.
/// Source anchors belong to their effective argument words, including aliases;
/// unresolved commands and computed expression text provide no guessed proof.
#[must_use]
pub fn lifted_source_expressions(
    tokens: Option<&CommandTokens>,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<LiftedSourceExpression> {
    selected_lifted_expressions(tokens, registry)
}

/// Original nested expressions under the supplied full metadata and grammar.
/// Missing or foreign availability cannot borrow the registry's nominal profile.
#[must_use]
pub(crate) fn lifted_source_expressions_with_metadata_context(
    tokens: Option<&CommandTokens>,
    registry: &tcl_registry::CommandRegistry,
    context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    config: tcl_lexer::LexerConfig,
    parser: tcl_syntax::expr::parser::ExprParseContext,
) -> Vec<LiftedSourceExpression> {
    let Some(context) = context.filter(|context| context.matches_registry(registry)) else {
        return Vec::new();
    };
    lifted_calls(tokens, config)
        .into_iter()
        .filter_map(|lifted| {
            let tokens = lifted.tokens.as_ref()?;
            let invocation =
                crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
                    registry,
                    Some(context),
                    tokens,
                )?;
            source_expression_from_invocation_with_syntax_context(
                &invocation,
                parser,
                lifted.span,
                expression_parent(tokens),
            )
        })
        .collect()
}

fn selected_lifted_expressions(
    tokens: Option<&CommandTokens>,
    registry: &tcl_registry::CommandRegistry,
) -> Vec<LiftedSourceExpression> {
    let context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
    lifted_calls(
        tokens,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )
    .into_iter()
    .filter_map(|lifted| {
        let tokens = lifted.tokens.as_ref()?;
        let invocation =
            crate::registry_invocation::resolved_tokens_invocation(registry, context, tokens)?;
        source_expression_from_invocation(
            &invocation,
            registry.profile(),
            lifted.span,
            expression_parent(tokens),
        )
    })
    .collect()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod readonly_logical_substitution_tests;

#[cfg(test)]
mod native_source_lifted_inventory_owner_tests;
