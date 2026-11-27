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

//! Tokens for one command: its head and the walk over its argument words.

use super::argument_override::ArgOverride;
use super::argument_tokens::{classify_arg_token, emit_arg_token};
use super::command_roles::{
    declared_class_for_body, deferred_role_arg_starts, next_original_definition_grammar,
    unrecorded_command_spec, unrecorded_head_kind,
};
use super::emission::{Entry, push_token};
use super::legend::{MOD_DEFAULT_LIBRARY, TokenKind};
use super::original;
use super::script_walk::{ScriptCtx, emit_static_command_head};
use rustc_hash::{FxHashMap, FxHashSet};
use tcl_lexer::{Span, Token, TokenType};

pub(super) struct CommandTokenSources<'a> {
    pub(super) original: &'a Option<crate::original_invocation::OriginalRegistryWords>,
    pub(super) produced: &'a Option<crate::original_invocation::OriginalRegistryWords>,
    pub(super) declared: &'a Option<tcl_compiler::command_binding::OriginalDeclaredCommandWords>,
    pub(super) member_bodies: &'a Option<
        Vec<tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody>,
    >,
    pub(super) original_bodies: &'a [tcl_compiler::registry_invocation::OriginalSourceScriptBody],
    pub(super) member_parameter_operands: &'a [crate::original_invocation::OriginalOperandSource],
    pub(super) overrides: &'a FxHashMap<u32, ArgOverride>,
    pub(super) deferred_role_starts: &'a FxHashSet<u32>,
    pub(super) expanded_roles: &'a [(Span, TokenKind, u32)],
    pub(super) computed_head: bool,
    pub(super) static_head_extent: Span,
}

pub(super) fn emit_selected_command_head(
    ctx: ScriptCtx<'_>,
    seg: &tcl_source::segment::SegmentedCommand,
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    member_head: Option<(&tcl_lexer::NativeWord, Span)>,
    entries: &mut Vec<Entry>,
) {
    let full_source = ctx.full_source;
    let head_tok = seg.argv[0];
    if member_head.is_none() && document_member(ctx, seg).is_some() {
        push_token(
            ctx.line_index,
            full_source,
            head_tok,
            TokenKind::Keyword,
            0,
            entries,
        );
    } else if ctx.lexical && original.is_none() && member_head.is_none() {
        emit_static_command_head(ctx, seg, entries);
    } else if let Some((_, span)) = member_head {
        push_token(
            ctx.line_index,
            full_source,
            Token { span, ..head_tok },
            TokenKind::Keyword,
            0,
            entries,
        );
    } else if ctx.original_roles.head_names_class(ctx, seg) {
        push_token(
            ctx.line_index,
            full_source,
            head_tok,
            TokenKind::Class,
            0,
            entries,
        );
    } else if let Some(words) = original {
        original::registry_head(ctx, seg, words, entries);
    } else {
        push_written_head(ctx, seg, entries);
    }
}

/// A written bareword head with no selected invocation. Its qualifier reads
/// as a namespace and its tail as the command, split where the naming owner
/// ends the qualifier, as a selected head is shown.
fn push_written_head(
    ctx: ScriptCtx<'_>,
    seg: &tcl_source::segment::SegmentedCommand,
    entries: &mut Vec<Entry>,
) {
    let head = seg.argv[0];
    let text = &seg.texts[0];
    let (kind, mods) = unrecorded_head_kind(ctx, seg);
    let tail = tcl_syntax::naming::written_command_tail(text.as_bytes());
    let split = text
        .len()
        .checked_sub(tail.len())
        .filter(|boundary| *boundary != 0 && !tail.is_empty())
        .filter(|_| {
            // A backslash sequence can end the name early (a NUL ends a C
            // command name), so only a plain spelling is split.
            head.kind == TokenType::Esc
                && seg.single_token_word.first() == Some(&true)
                && !text.contains('\\')
                && ctx.full_source.get(head.span.as_range()) == Some(text.as_str())
        })
        .and_then(|boundary| u32::try_from(boundary).ok())
        .and_then(|boundary| head.span.start().checked_add(boundary));
    let Some(split) = split else {
        push_token(ctx.line_index, ctx.full_source, head, kind, mods, entries);
        return;
    };
    let known = unrecorded_command_spec(ctx, seg).is_some();
    push_token(
        ctx.line_index,
        ctx.full_source,
        Token {
            span: Span::new(head.span.start(), split),
            ..head
        },
        TokenKind::Namespace,
        if known { MOD_DEFAULT_LIBRARY } else { 0 },
        entries,
    );
    push_token(
        ctx.line_index,
        ctx.full_source,
        Token {
            span: Span::new(split, head.span.end()),
            ..head
        },
        kind,
        mods,
        entries,
    );
}

/// A written member of the document's own grammar (a `.sslictcl` or `SpecTcl`
/// document), at the root or inside a member that opens a member grammar,
/// where no selected definition-member inventory covers the command.
pub(super) fn document_member(
    ctx: ScriptCtx<'_>,
    seg: &tcl_source::segment::SegmentedCommand,
) -> Option<&'static tcl_registry::definer::MemberSpec> {
    if ctx.original_definition_members.is_some()
        || seg.single_token_word.first() != Some(&true)
        || seg.argv.first()?.kind != TokenType::Esc
    {
        return None;
    }
    let grammar = ctx.oo_grammar?;
    if !ctx.written_members && ctx.registry.document_grammar()?.family != grammar.family {
        return None;
    }
    grammar.source_member_in(
        seg.texts.first()?.as_bytes(),
        Some(ctx.context.authoring_query()),
    )
}

pub(super) fn deferred_produced_command_words(
    ctx: ScriptCtx<'_>,
    seg: &tcl_source::segment::SegmentedCommand,
    deferred_role: bool,
) -> Option<crate::original_invocation::OriginalRegistryWords> {
    let full_source = ctx.full_source;
    deferred_role
        .then(|| {
            let analysis = ctx.analysis?;
            crate::original_invocation::source_produced_command_prefix_words(
                full_source,
                analysis,
                seg,
            )
        })
        .flatten()
}

pub(super) fn command_body_context<'a>(
    ctx: ScriptCtx<'a>,
    seg: &tcl_source::segment::SegmentedCommand,
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    head_text: &str,
    arg_texts: &[&str],
) -> ScriptCtx<'a> {
    let full_source = ctx.full_source;
    // Receiver advice keeps its independent grammar. Member vocabulary
    // travels in the original body receipts below.
    let outer_grammar = original.and_then(|words| {
        words
            .with_source_schema(ctx.generation, |schema| {
                schema.authored_source_definition_body_grammar()
            })
            .flatten()
    });
    // A command with no retained invocation opens the definition grammar its
    // written words select, so the member words inside read as members.
    let written_grammar = (!ctx.lexical && original.is_none())
        .then(|| {
            let spec = unrecorded_command_spec(ctx, seg)?;
            ctx.registry
                .resolve_invocation(spec.name, arg_texts, None)?
                .authored_source_definition_body_grammar()
        })
        .flatten();
    let written_members = written_grammar.is_some() || (ctx.written_members && original.is_none());
    let next_oo = if ctx.lexical || written_members {
        next_original_definition_grammar(
            ctx,
            head_text,
            arg_texts,
            outer_grammar.or(written_grammar),
        )
    } else {
        // A document grammar's member opens its own member grammar on every
        // profile; a class-system body keeps its selected source owner.
        ctx.oo_grammar.and_then(|current| {
            ctx.registry
                .authored_document_member_grammar(current, head_text)
        })
    };
    // The class whose body the recursion enters: a `oo::class create NAME`
    // (and the property-/instantiation-metaclasses) names it at argv[2], an
    // `oo::define NAME { … }` at argv[1].  Slice it from the source so it
    // outlives the walk; otherwise inherit the enclosing class (so a
    // `method …` body keeps its class).  Lets `my method …` in the body
    // resolve against that class.
    let next_class = if ctx.lexical {
        original
            .and_then(|words| original::definition_class_name(ctx, seg, words))
            .or(ctx.enclosing_class)
    } else {
        declared_class_for_body(ctx, seg).or(ctx.enclosing_class)
    };
    // The scoped command environment the recursion into THIS command's body
    // should carry: a command whose spec declares a `body_scope` switches it
    // on (`report::defstyle`'s style script); otherwise it persists so the
    // scoped commands stay resolvable inside nested control-flow bodies and
    // `[…]` substitutions within the style script.
    let next_scoped = ctx
        .analysis
        .and_then(|analysis| {
            analysis.original_scoped_body_in_source(
                &tcl_lexer::SourceImage::document(full_source),
                ctx.config,
                seg.span.start(),
            )
        })
        .map(tcl_compiler::registry_invocation::OriginalSourceScopedBody::environment)
        .or(ctx.scoped_env);
    ScriptCtx {
        oo_grammar: next_oo,
        written_members: written_members && next_oo.is_some(),
        enclosing_class: next_class,
        scoped_env: next_scoped,
        ..ctx
    }
}

pub(super) fn command_deferred_role_starts(
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    declared: Option<&tcl_compiler::command_binding::OriginalDeclaredCommandWords>,
) -> FxHashSet<u32> {
    let mut deferred_role_starts = deferred_role_arg_starts(original);
    if let Some(declared) = declared {
        deferred_role_starts.extend(
            declared
                .supplied_argument_roles()
                .unwrap_or_default()
                .into_iter()
                .filter(|(_, role)| {
                    matches!(
                        role,
                        tcl_registry::ArgRole::Body
                            | tcl_registry::ArgRole::CommandPrefix
                            | tcl_registry::ArgRole::LambdaLiteral
                    )
                })
                .filter_map(|(ordinal, _)| {
                    declared
                        .argument_word(ordinal)?
                        .tokens()
                        .first()
                        .map(|token| token.span.start())
                }),
        );
    }

    deferred_role_starts
}

pub(super) fn collect_command_tokens(
    ctx: ScriptCtx<'_>,
    body_ctx: ScriptCtx<'_>,
    seg: &tcl_source::segment::SegmentedCommand,
    sources: &CommandTokenSources<'_>,
    entries: &mut Vec<Entry>,
    depth: u32,
) {
    let CommandTokenSources {
        original,
        produced,
        declared,
        member_bodies,
        original_bodies,
        member_parameter_operands,
        overrides,
        deferred_role_starts,
        expanded_roles,
        computed_head,
        static_head_extent,
    } = *sources;
    for tok in &seg.all_tokens {
        if original::emit_expanded_role_extents(ctx, *tok, expanded_roles, entries) {
            continue;
        }
        // Skip every token that falls inside a *static* head word — not
        // just the exact head token.  Such a head is one word that
        // `emit_static_command_head` already emitted as a single command token;
        // its sub-fragments (`ns::`, `cmd` for a `ns::cmd` head) also
        // appear in `all_tokens`, and emitting those would overlap the head
        // token (invalid — LSP clients reject overlapping semantic tokens).
        //
        // A *computed* head is deliberately NOT emitted by
        // `emit_static_command_head` (see above), so its tokens must flow through
        // the argument path here: the `[…]` head token recurses into its
        // inner script and the `$var` head token reads as a variable.
        if !computed_head
            && tok.span.start() >= static_head_extent.start()
            && tok.span.end() <= static_head_extent.end()
        {
            continue;
        }
        let token_ctx = ScriptCtx {
            operand: produced
                .as_ref()
                .or(original.as_ref())
                .and_then(|words| {
                    words
                        .operands
                        .iter()
                        .filter_map(Option::as_ref)
                        .find(|operand| operand.span == tok.span)
                })
                .or_else(|| {
                    member_parameter_operands
                        .iter()
                        .find(|operand| operand.span == tok.span)
                })
                .or_else(|| ctx.original_roles.operand_at(tok.span)),
            original_words: produced.as_ref().or(original.as_ref()),
            declared_words: declared.as_ref(),
            ..ctx
        };
        let definition_parent = if let Some(member_body) =
            member_bodies.iter().flatten().find(|body| {
                body.original_container()
                    .tokens()
                    .iter()
                    .any(|part| part.span == tok.span)
            }) {
            member_body.definition_parent().cloned()
        } else if let Some(body) = original_bodies.iter().find(|body| {
            body.original_container()
                .tokens()
                .iter()
                .any(|part| part.span == tok.span)
        }) {
            body.definition_parent_for(ctx.generation, ctx.original_definition_parent)
        } else {
            None
        };
        let token_body_ctx = ScriptCtx {
            receiver_grammar: matches!(
                overrides.get(&tok.span.start()),
                Some(ArgOverride::BodyScript)
            )
            .then(|| original::receiver_grammar_for_body(ctx, seg, original.as_ref(), *tok))
            .flatten(),
            original_definition_parent: definition_parent.as_ref(),
            original_definition_members: None,
            ..body_ctx
        };
        emit_arg_token(
            token_ctx,
            token_body_ctx,
            *tok,
            overrides.get(&tok.span.start()),
            deferred_role_starts.contains(&tok.span.start()),
            entries,
            depth,
        );
    }
}

/// Emit semantic-token entries for a single non-head argument token,
/// dispatching on its [`ArgOverride`] (or falling back to default
/// classification) and recursing into braced bodies / expressions /
/// command substitutions.  Extracted from [`collect_script`] to keep that
/// function's body small.
/// When `cond` holds, classify `tok` with [`classify_arg_token`] and, if it
/// yields a kind, push a plain token.  Used as the fallback for the
/// sub-tokenising format overrides (sprintf / clock / binary / regsub) when
/// the specialised sub-lexer declined to emit anything.
pub(super) fn classify_and_push_if(
    cond: bool,
    ctx: ScriptCtx<'_>,
    tok: Token,
    entries: &mut Vec<Entry>,
) {
    if cond && let Some(kind) = classify_arg_token(tok, ctx.full_source, ctx.numbers) {
        push_token(ctx.line_index, ctx.full_source, tok, kind, 0, entries);
    }
}
