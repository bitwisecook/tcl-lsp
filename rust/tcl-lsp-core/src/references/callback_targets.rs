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

//! Method words captured into callbacks: stored selectors, command prefixes and list-built self dispatch.

use super::dispatch_regions::{
    MAX_DISPATCH_SCAN_DEPTH, RetainedDispatchContext, nested_dispatch_regions,
    retained_dispatch_context, strip_outer_braces,
};
use super::member_references::collect_member_bodies;
use super::next_dispatch::span_contains_offset;
use super::self_dispatch_scan::{MyMethodScan, scan_method_sites_by_kind};
use super::strip_var_decoration;
use tcl_compiler::analyser::AnalysisResult;

/// Readonly stored selectors under a genuine original method-body issuer.
/// Source roles retain conditional declaration advice, independently of capture
/// results, a live receiver, callback entry and replacement permission.
pub(crate) fn authored_callback_reference_spans(
    source: &str,
    analysis: &AnalysisResult,
    target: &crate::method_symbol::OriginalMethodCandidate,
) -> Vec<tcl_lexer::Span> {
    use tcl_registry::definer::DefinitionMemberNamePurpose;

    let Some(current) = retained_dispatch_context(source, analysis) else {
        return Vec::new();
    };
    let Some(class) = target.declaring_class_in(source, analysis) else {
        return Vec::new();
    };
    let method = target.method();
    if method.name_purpose() != DefinitionMemberNamePurpose::TclOoMethod {
        return Vec::new();
    }
    let name = method.original_name_input();
    let mut declarations = class
        .metadata()
        .original_members
        .declarations()
        .filter(|row| {
            row.side() == method.side()
                && row.original_name_input().policy() == name.policy()
                && row.original_name_input().bytes() == name.bytes()
        });
    if declarations.next() != Some(method) || declarations.next().is_some() {
        return Vec::new();
    }
    let ctx = MyMethodScan {
        source,
        analysis,
        config: current.config,
        dialect: current.dialect,
        method: "",
        skip: None,
        external_callback_allowed: false,
        callbacks_only: true,
        stored_callbacks: &[],
    };
    let mut spans = Vec::new();
    crate::executable_regions::visit_analysis_executable_commands(
        source,
        analysis,
        &mut |command, _, _| {
            if let Some(span) = authored_stored_callback_selector(
                ctx,
                &current.context,
                class.metadata(),
                method,
                command,
            ) {
                spans.push(span);
            }
            false
        },
    );
    spans.sort_unstable_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    spans
}

fn authored_stored_callback_selector(
    ctx: MyMethodScan<'_>,
    context: &tcl_registry::model::ContextRegistry,
    class: &tcl_compiler::analyser::ClassDef,
    method: &tcl_compiler::analyser::types::OriginalSourceMethodMetadata,
    command: &tcl_source::segment::SegmentedCommand,
) -> Option<tcl_lexer::Span> {
    use tcl_compiler::registry_invocation::{InvocationWordOrigin, source_structure};
    use tcl_names::word::SignatureSourceNameKey;

    let prefix =
        source_structure::source_authored_command_prefix(ctx.source, ctx.analysis, command)?;
    if !prefix.matches_source_context(
        &tcl_lexer::SourceImage::document(ctx.source),
        ctx.config,
        context,
    ) || prefix.with_source_schema(context, |schema| {
        schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
    }) != Some(true)
    {
        return None;
    }
    let owner = authored_callback_body_owner(class, method, prefix.definition_member_body()?)?;
    let word = prefix.original_argument(0)?;
    let InvocationWordOrigin::Written(written) = prefix.argument_origin(0)? else {
        return None;
    };
    if written == 0 || prefix.producer().original_words().get(written) != Some(word) {
        return None;
    }
    let name = method.original_name_input();
    let key = SignatureSourceNameKey::from_original_native_word(
        word,
        tcl_syntax::word_rules::WordValueRules::from_config(&ctx.config),
        name.policy(),
    )?;
    if key.bytes() != name.bytes()
        || !method
            .name_purpose()
            .admits(key.policy().recipe(), key.bytes())
    {
        return None;
    }
    let span = bare_original_prefix_span(word)?;
    let body = owner.metadata().body_span;
    let stored = stored_callback_targets(ctx, body);
    let body_ctx = MyMethodScan {
        stored_callbacks: &stored,
        ..ctx
    };
    let (start, end) = strip_outer_braces(ctx.source, body);
    let candidate = PrefixTargetAtSpan {
        kind: tcl_registry::CommandPrefixTarget::CurrentObjectInternalMethod,
        span,
    };
    (stored.iter().any(|stored| stored.target == candidate)
        && stored_callback_is_consumed(body_ctx, start, end, candidate, 0))
    .then_some(span)
}

fn authored_callback_body_owner<'a>(
    class: &'a tcl_compiler::analyser::ClassDef,
    method: &tcl_compiler::analyser::types::OriginalSourceMethodMetadata,
    issuer: &tcl_compiler::registry_invocation::OriginalSourceDefinitionMemberScriptBody,
) -> Option<&'a tcl_compiler::analyser::types::OriginalSourceMethodMetadata> {
    use tcl_compiler::analyser::types::MemberSide;
    use tcl_registry::definer::{
        CallableRole, DefinitionMemberNamePurpose, MemberEffect, MemberReceiver,
    };

    let row = issuer.member_row()?;
    if !matches!(
        row.effect,
        MemberEffect::Callable {
            role: CallableRole::Method,
            ..
        }
    ) {
        return None;
    }
    let side = match row.receiver {
        MemberReceiver::Instance => MemberSide::Instance,
        MemberReceiver::TypeObject => MemberSide::ClassObject,
        MemberReceiver::Both => return None,
    };
    if side != method.side() {
        return None;
    }
    let mut owners = class.original_members.declarations().filter(|owner| {
        owner.side() == side
            && owner.name_purpose() == DefinitionMemberNamePurpose::TclOoMethod
            && owner.body_word() == Some(issuer.original_container())
    });
    let owner = owners.next()?;
    owners.next().is_none().then_some(owner)
}

/// Resolve a method operand only through its actual receiver entry.
/// Stored prefix syntax alone cannot grant a captured receiver allocation.
pub(crate) fn list_built_self_method_target_at_cursor(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<(String, bool)> {
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let selected = crate::receiver_identity::method_at_cursor(analysis, source, cursor_offset)?;
    let side = if selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class {
        tcl_compiler::analyser::types::MemberSide::ClassObject
    } else {
        tcl_compiler::analyser::types::MemberSide::Instance
    };
    let (current, member) =
        analysis
            .class_hierarchy()
            .declared_member(&selected.class.qualified_name, word, side)?;
    // The shared compatibility owner must retain the same declaration and side.
    (selected.editable_selector().is_some()
        && member == selected.method
        && current.name_span == selected.class.name_span)
        .then(|| {
            (
                selected.class.qualified_name.clone(),
                selected.receiver == tcl_compiler::command_binding::SourceMethodReceiver::Class,
            )
        })
}

/// A retained receiver at the cursor, independently of lexical class scopes.
/// Unexecuted callback prefixes need their own capture receipt.
#[must_use]
pub fn callback_prefix_method_receiver_at_cursor(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    word: &str,
    cursor_offset: u32,
) -> Option<(String, bool, bool)> {
    let (class, is_classmethod) =
        list_built_self_method_target_at_cursor(source, dialect, analysis, word, cursor_offset)?;
    Some((class, is_classmethod, true))
}

/// A syntactically possible captured selector without an actual entry receipt.
/// This is a whole-rename hazard only, never a reference or replacement span.
pub(crate) fn unproved_callback_method_selector(
    source: &str,
    _dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    method: &str,
) -> Option<tcl_lexer::Span> {
    for class in analysis
        .all_classes
        .values()
        .chain(analysis.superseded_classes.values().flatten())
    {
        for body in collect_member_bodies(class) {
            for span in
                scan_method_sites_by_kind(source, analysis, &[body], method, None, true, true)
            {
                if command_prefix_target_at_cursor(source, analysis, body, span.start()).is_some()
                    && crate::receiver_identity::method_at_cursor(analysis, source, span.start())
                        .is_none()
                {
                    return Some(span);
                }
            }
        }
    }
    None
}

#[derive(Clone)]
pub(super) struct StoredCallbackTarget {
    variable: String,
    assignment_end: u32,
    pub(super) target: PrefixTargetAtSpan,
}

/// Recover only the one-hop, same-frame constant form of a stored callback.
///
/// This deliberately treats every write to a variable as a candidate write:
/// if there is more than one write, or the sole write is dynamic, the variable
/// is omitted.  That makes reassignment, branch joins, parameters, and other
/// incomplete SSA cases abstain without teaching this LSP scanner a command
/// name or inventing a target.
pub(super) fn stored_callback_targets(
    ctx: MyMethodScan<'_>,
    body_span: tcl_lexer::Span,
) -> Vec<StoredCallbackTarget> {
    use std::collections::HashMap;

    let (start, end) = strip_outer_braces(ctx.source, body_span);
    if start >= end || end > ctx.source.len() {
        return Vec::new();
    }
    let mut writes: HashMap<String, (u32, Option<PrefixTargetAtSpan>)> = HashMap::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let has_scope_alias =
        collect_stored_callback_writes(ctx, start, end, 0, &mut writes, &mut counts);
    if has_scope_alias {
        return Vec::new();
    }
    writes
        .into_iter()
        .filter_map(|(name, (assignment_end, target))| {
            if counts.get(&name) != Some(&1) {
                return None;
            }
            target.map(|target| StoredCallbackTarget {
                variable: name,
                assignment_end,
                target,
            })
        })
        .collect()
}

/// Collect callback assignments from every registry-declared same-frame
/// region. The callback consumer already descends through these regions; the
/// write inventory must cover the identical control-flow surface or a nested
/// reassignment could leave a stale outer callback looking unique.
fn collect_stored_callback_writes(
    ctx: MyMethodScan<'_>,
    start: usize,
    end: usize,
    depth: u32,
    writes: &mut std::collections::HashMap<String, (u32, Option<PrefixTargetAtSpan>)>,
    counts: &mut std::collections::HashMap<String, usize>,
) -> bool {
    use tcl_compiler::registry_invocation::{InvocationWordOrigin, source_structure};
    use tcl_lexer::TokenType;
    use tcl_source::segment::segment_commands_with_offset_and_config;

    if MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) || end > ctx.source.len() {
        return true;
    }
    if start >= end {
        return false;
    }
    let Some(input) = ctx.analysis.resolved_input.as_ref() else {
        return true;
    };
    let context = input.context_registry();
    let Ok(offset) = u32::try_from(start) else {
        return true;
    };
    let commands = segment_commands_with_offset_and_config(
        &ctx.source[start..end],
        offset,
        ctx.config.nested(),
    );
    let mut has_scope_alias = false;
    for cmd in commands {
        let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, &cmd)
        else {
            has_scope_alias = true;
            continue;
        };
        let Some((aliases, setter, variables)) = stored_callback_schema(&words, &context) else {
            has_scope_alias = true;
            continue;
        };
        has_scope_alias |= aliases;
        for (ordinal, variable) in variables {
            let Some(var) = variable else {
                has_scope_alias = true;
                continue;
            };
            if tcl_syntax::naming::is_qualified(var.as_bytes())
                || tcl_syntax::naming::split_element_ref(&var).is_some()
            {
                continue;
            }
            *counts.entry(var.clone()).or_default() += 1;
            // Selected Set describes adjacent effective operands. Captured names
            // retain their value but never borrow a caller's value-token index.
            let value_target = setter
                .then(|| {
                    let origin = words.origins().get(ordinal.checked_add(2)?)?;
                    let InvocationWordOrigin::Written(written) = origin else {
                        return None;
                    };
                    let &token = cmd.argv.get(*written)?;
                    (token.kind == TokenType::Cmd
                        && cmd.single_token_word.get(*written) == Some(&true)
                        && positions_are_literal_through(&cmd, *written))
                    .then(|| command_prefix_targets_from_word(ctx, &token, 0))
                    .and_then(|targets| (targets.len() == 1).then(|| targets[0]))
                })
                .flatten();
            writes.insert(var, (cmd.span.end(), value_target));
        }
        for (nested_start, nested_end) in
            nested_dispatch_regions(ctx.source, ctx.analysis, ctx.dialect, &cmd)
        {
            has_scope_alias |= collect_stored_callback_writes(
                ctx,
                nested_start,
                nested_end,
                depth + 1,
                writes,
                counts,
            );
        }
    }
    has_scope_alias
}

type StoredCallbackSchema = (bool, bool, Vec<(usize, Option<String>)>);

fn stored_callback_schema(
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
) -> Option<StoredCallbackSchema> {
    use tcl_registry::hooks::LoweringHookId;
    use tcl_registry::{ArgRole, Traits};
    words
        .with_source_schema(context, |schema| {
            let roles = words.roles()?;
            let variables = roles
                .iter()
                .filter(|(_, role)| *role == ArgRole::VarWrite)
                .map(|&(ordinal, _)| {
                    (
                        ordinal,
                        schema
                            .words
                            .arguments()
                            .get(ordinal)
                            .and_then(tcl_registry::InvocationWord::literal)
                            .map(str::to_owned),
                    )
                })
                .collect::<Vec<_>>();
            Some((
                schema
                    .semantics
                    .traits
                    .contains(Traits::CREATES_SCOPE_ALIAS)
                    || matches!(
                        schema.semantics.lowering_hook,
                        Some(
                            LoweringHookId::Global
                                | LoweringHookId::Variable
                                | LoweringHookId::Upvar
                        )
                    ),
                schema.semantics.lowering_hook == Some(LoweringHookId::Set),
                variables,
            ))
        })
        .flatten()
}

/// Matching method words captured into this command's executable callback.
///
/// A command-prefix builder can be wrapped by another registry-declared
/// prefix wrapper, so this recognises both direct `[list [self] METHOD]` /
/// `[list my METHOD]` forms and values such as `[namespace code [list my
/// METHOD]]`.  The traversal is trait-driven: it follows only
/// `BUILDS_COMMAND_PREFIX` and `WRAPS_COMMAND_PREFIX`, never a command name.
///
/// The enclosing resolved registry spec must mark the argument as either a
/// complete executable [`tcl_registry::ArgRole::Body`] or a first-class
/// [`tcl_registry::ArgRole::CommandPrefix`], and the inner resolved spec must carry
/// `BUILDS_COMMAND_PREFIX`; neither a callback consumer nor its builder is
/// named here. Inert data such as `set x [list [self] METHOD]` therefore
/// remains data.
pub(super) fn list_built_self_reference_spans(
    ctx: MyMethodScan<'_>,
    cmd: &tcl_source::segment::SegmentedCommand,
) -> Vec<tcl_lexer::Span> {
    callback_targets_from_command(ctx, cmd)
        .into_iter()
        .filter_map(|target| match target {
            PrefixTargetAtSpan {
                kind: tcl_registry::CommandPrefixTarget::CurrentObjectInternalMethod,
                span,
            } if source_span_text(ctx.source, span) == ctx.method => Some(span),
            PrefixTargetAtSpan {
                kind: tcl_registry::CommandPrefixTarget::CurrentObjectExternalMethod,
                span,
            } if ctx.external_callback_allowed
                && source_span_text(ctx.source, span) == ctx.method =>
            {
                Some(span)
            }
            PrefixTargetAtSpan { .. } => None,
        })
        .collect()
}

/// Resolve registry-declared callback argument positions and extract their
/// typed static prefix target.  This is intentionally independent of a
/// particular method name so cursor navigation and call hierarchy can use the
/// exact same source classification as references/rename/code-lens.
fn callback_targets_from_command(
    ctx: MyMethodScan<'_>,
    cmd: &tcl_source::segment::SegmentedCommand,
) -> Vec<PrefixTargetAtSpan> {
    use tcl_compiler::registry_invocation::{InvocationWordOrigin, source_structure};
    use tcl_lexer::TokenType;

    let Some(input) = ctx.analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let context = input.context_registry();
    let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, cmd) else {
        return Vec::new();
    };
    // A prefix wrapper (`namespace code`) builds a value and runs nothing; it
    // is followed from the sink that consumes its result, never as a sink.
    if words.with_source_schema(&context, |schema| {
        schema
            .semantics
            .traits
            .contains(tcl_registry::Traits::WRAPS_COMMAND_PREFIX)
    }) != Some(false)
    {
        return Vec::new();
    }
    let Some(positions) = words
        .with_source_schema(&context, |schema| {
            if ctx.analysis.allows_retained_logical_declaration_advice() {
                schema.authored_logical_source_plain_script_arguments()
            } else {
                schema.authored_source_plain_script_arguments()
            }
        })
        .flatten()
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ordinal in positions {
        let Some(InvocationWordOrigin::Written(written)) = words.origins().get(ordinal + 1) else {
            continue;
        };
        let Some(&body_tok) = cmd.argv.get(*written) else {
            continue;
        };
        if !positions_are_literal_through(cmd, *written) {
            continue;
        }
        if body_tok.kind == TokenType::Var && cmd.single_token_word.get(*written) == Some(&true) {
            let original_words = tcl_compiler::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(ctx.source),
                ctx.config,
                cmd,
            );
            let Some(variable) = original_words
                .word_exprs
                .get(*written)
                .and_then(tcl_source::words::WordExpr::sole_variable_substitution)
                .and_then(|(spelling, _)| strip_var_decoration(spelling, Some(ctx.config)))
            else {
                continue;
            };
            let matches: Vec<_> = ctx
                .stored_callbacks
                .iter()
                .filter(|stored| {
                    stored.variable == variable && stored.assignment_end < cmd.span.start()
                })
                .collect();
            if matches.len() == 1 {
                out.push(matches[0].target);
            }
            continue;
        }
        // Only a genuine sole written substitution is renameable here. A
        // captured operand retains its producer, not this call's source span.
        if body_tok.kind == TokenType::Cmd && cmd.single_token_word.get(*written) == Some(&true) {
            out.extend(command_prefix_targets_from_word(ctx, &body_tok, 0));
        }
    }
    out
}

/// Whether every word of `cmd` up to and including `upto` is written out
/// rather than `{*}`-expanded.
///
/// A consumer that reads word *N* is relying on the registry's role indices,
/// and those describe words. `{*}` splices a value's elements into the
/// argument list, so one expansion at or before *N* makes every later index
/// unreliable — not just the expanded word itself. Checking the whole prefix
/// is what makes `[namespace code {*}[list my tick]]` abstain: the expansion
/// is at the wrapper's body position, and after it `namespace code` has two
/// arguments and errors rather than dispatching anything.
///
/// `expand_word` is `None` for the overwhelming majority of commands — no word
/// uses expansion — and a per-word flag list otherwise.
fn positions_are_literal_through(cmd: &tcl_source::segment::SegmentedCommand, upto: usize) -> bool {
    cmd.expand_word
        .as_ref()
        .is_none_or(|flags| flags.iter().take(upto + 1).all(|expanded| !expanded))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct PrefixTargetAtSpan {
    pub(super) kind: tcl_registry::CommandPrefixTarget,
    pub(super) span: tcl_lexer::Span,
}

/// Map an effective builder operand to its genuine whole written source token.
fn written_prefix_operand(
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    command: &tcl_source::segment::SegmentedCommand,
    ordinal: usize,
) -> Option<tcl_lexer::Token> {
    use tcl_compiler::registry_invocation::InvocationWordOrigin;
    let InvocationWordOrigin::Written(written) = words.origins().get(ordinal.checked_add(1)?)?
    else {
        return None;
    };
    (command.single_token_word.get(*written) == Some(&true)
        && positions_are_literal_through(command, *written))
    .then_some(())?;
    command.argv.get(*written).copied()
}

fn bare_original_prefix_span(word: &tcl_lexer::NativeWord) -> Option<tcl_lexer::Span> {
    let [token] = word.tokens() else {
        return None;
    };
    (token.kind == tcl_lexer::TokenType::Esc && !token.in_quote && !word.group().expand)
        .then_some(token.span)
}

/// Extract static method words from one sole command substitution used as a
/// callback value.  The recursion deliberately requires each hop to be a
/// single substitution, so concatenated and dynamically-built values abstain.
fn command_prefix_targets_from_word(
    ctx: MyMethodScan<'_>,
    word_tok: &tcl_lexer::Token,
    depth: u32,
) -> Vec<PrefixTargetAtSpan> {
    use tcl_compiler::registry_invocation::source_structure;
    use tcl_lexer::TokenType;
    use tcl_registry::Traits;
    use tcl_source::segment::segment_commands_with_offset_and_config;

    if MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) || word_tok.kind != TokenType::Cmd {
        return Vec::new();
    }
    let Some(input) = ctx.analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    let context = input.context_registry();
    let regions =
        crate::executable_regions::command_substitution_regions(ctx.source, ctx.config, *word_tok);
    let [(inner_start, inner_end)] = regions.as_slice() else {
        return Vec::new();
    };
    let Ok(offset) = u32::try_from(*inner_start) else {
        return Vec::new();
    };
    let built = segment_commands_with_offset_and_config(
        &ctx.source[*inner_start..*inner_end],
        offset,
        ctx.config.nested(),
    );
    let [builder] = built.as_slice() else {
        return Vec::new();
    };
    let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, builder)
    else {
        return Vec::new();
    };
    let Some((builds, wraps)) = words.with_source_schema(&context, |schema| {
        (
            schema
                .semantics
                .traits
                .contains(Traits::BUILDS_COMMAND_PREFIX)
                && schema.semantics.native_result
                    == Some(
                        tcl_registry::native_result::NativeResultContract::ListArguments {
                            from: 0,
                        },
                    )
                && schema.semantics.argument_offset == 0
                && schema.facts().arity_accepts_frozen_arguments() == Some(true),
            schema
                .semantics
                .traits
                .contains(Traits::WRAPS_COMMAND_PREFIX),
        )
    }) else {
        return Vec::new();
    };
    if builds {
        return built_command_prefix_targets(ctx, builder, &words, &context);
    }
    if !wraps {
        return Vec::new();
    }
    let Some(positions) = words
        .with_source_schema(&context, |schema| {
            if ctx.analysis.allows_retained_logical_declaration_advice() {
                schema.authored_logical_source_plain_script_arguments()
            } else {
                schema.authored_source_plain_script_arguments()
            }
        })
        .flatten()
    else {
        return Vec::new();
    };
    positions
        .into_iter()
        .filter_map(|ordinal| written_prefix_operand(&words, builder, ordinal))
        .flat_map(|body| command_prefix_targets_from_word(ctx, &body, depth + 1))
        .collect()
}

fn built_command_prefix_targets(
    ctx: MyMethodScan<'_>,
    builder: &tcl_source::segment::SegmentedCommand,
    words: &tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<PrefixTargetAtSpan> {
    use tcl_compiler::registry_invocation::source_structure;
    use tcl_lexer::TokenType;
    use tcl_registry::Traits;
    if let Some(prefix) =
        source_structure::source_authored_command_prefix(ctx.source, ctx.analysis, builder)
        && prefix.with_source_schema(context, |schema| {
            schema
                .semantics
                .traits
                .contains(Traits::TCLOO_SELF_DISPATCH)
        }) == Some(true)
        && let Some(span) = prefix
            .original_argument(0)
            .and_then(bare_original_prefix_span)
    {
        return vec![PrefixTargetAtSpan {
            kind: tcl_registry::CommandPrefixTarget::CurrentObjectInternalMethod,
            span,
        }];
    }
    if let (Some(receiver), Some(method)) = (
        written_prefix_operand(words, builder, 0),
        written_prefix_operand(words, builder, 1),
    ) && method.kind == TokenType::Esc
        && !method.in_quote
        && original_self_receiver_word(ctx, receiver)
    {
        return vec![PrefixTargetAtSpan {
            kind: tcl_registry::CommandPrefixTarget::CurrentObjectExternalMethod,
            span: method.span,
        }];
    }
    // In a body the analysis never entered, the builder has no authored
    // prefix receipt. A literal head the current context resolves to the
    // self-dispatch command still names the current object's method, unless
    // the document may rebind that command.
    if let (Some(head), Some(method)) = (
        written_prefix_operand(words, builder, 0),
        written_prefix_operand(words, builder, 1),
    ) && head.kind == TokenType::Esc
        && !head.in_quote
        && method.kind == TokenType::Esc
        && !method.in_quote
        && let Some(spelling) = ctx.source.get(head.span.as_range())
        && context
            .context()
            .resolve_spec(context.commands(), spelling)
            .is_some_and(|spec| spec.traits.contains(Traits::TCLOO_SELF_DISPATCH))
        && context.commands().method_dispatch_keyword(spelling)
            == Some(tcl_registry::MethodDispatchKind::SelfDispatch)
        && super::unentered_callbacks::contextual_dispatch_head_is_unrebound(
            ctx.analysis,
            spelling,
            builder
                .argv
                .first()
                .map_or(head.span.start(), |word| word.span.start()),
        )
    {
        return vec![PrefixTargetAtSpan {
            kind: tcl_registry::CommandPrefixTarget::CurrentObjectInternalMethod,
            span: method.span,
        }];
    }
    // A literal list head is source syntax only. Its declaration or current
    // target, if any, is resolved independently by the navigation consumer.
    written_prefix_operand(words, builder, 0)
        .filter(|token| token.kind == TokenType::Esc && !token.in_quote)
        .map(|token| PrefixTargetAtSpan {
            kind: tcl_registry::CommandPrefixTarget::DirectCommandHead,
            span: token.span,
        })
        .into_iter()
        .collect()
}

pub(super) fn source_span_text(source: &str, span: tcl_lexer::Span) -> &str {
    let start = span.start() as usize;
    let end = span.end() as usize;
    source.get(start..end).unwrap_or_default()
}

/// Classify the deferred callback target exactly under `cursor_offset` in one
/// method body.  This reuses the same registry-role and prefix-result walker
/// as the reference scan, then descends only through registry-declared nested
/// executable regions.
fn command_prefix_target_at_cursor(
    source: &str,
    analysis: &AnalysisResult,
    body: tcl_lexer::Span,
    cursor_offset: u32,
) -> Option<PrefixTargetAtSpan> {
    let RetainedDispatchContext {
        dialect, config, ..
    } = retained_dispatch_context(source, analysis)?;
    let base_ctx = MyMethodScan {
        source,
        analysis,
        config,
        dialect,
        method: "",
        skip: None,
        external_callback_allowed: true,
        callbacks_only: true,
        stored_callbacks: &[],
    };
    let (start, end) = strip_outer_braces(source, body);
    let stored = stored_callback_targets(base_ctx, body);
    let ctx = MyMethodScan {
        stored_callbacks: &stored,
        ..base_ctx
    };
    if let Some(target) = stored.iter().find(|stored| {
        span_contains_offset(stored.target.span, cursor_offset)
            && stored_callback_is_consumed(ctx, start, end, stored.target, 0)
    }) {
        return Some(target.target);
    }
    command_prefix_target_in_region(ctx, start, end, 0, cursor_offset)
}

fn stored_callback_is_consumed(
    ctx: MyMethodScan<'_>,
    start: usize,
    end: usize,
    target: PrefixTargetAtSpan,
    depth: u32,
) -> bool {
    use tcl_source::segment::segment_commands_with_offset_and_config;
    if start >= end || end > ctx.source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return false;
    }
    let commands = segment_commands_with_offset_and_config(
        &ctx.source[start..end],
        u32::try_from(start).unwrap_or(0),
        ctx.config.nested(),
    );
    commands.iter().any(|cmd| {
        callback_targets_from_command(ctx, cmd)
            .into_iter()
            .any(|candidate| candidate == target)
            || nested_dispatch_regions(ctx.source, ctx.analysis, ctx.dialect, cmd)
                .into_iter()
                .any(|(nested_start, nested_end)| {
                    stored_callback_is_consumed(ctx, nested_start, nested_end, target, depth + 1)
                })
    })
}

fn command_prefix_target_in_region(
    ctx: MyMethodScan<'_>,
    start: usize,
    end: usize,
    depth: u32,
    cursor_offset: u32,
) -> Option<PrefixTargetAtSpan> {
    use tcl_source::segment::segment_commands_with_offset_and_config;

    if start >= end || end > ctx.source.len() || MAX_DISPATCH_SCAN_DEPTH.exceeded(depth) {
        return None;
    }
    let commands = segment_commands_with_offset_and_config(
        &ctx.source[start..end],
        u32::try_from(start).unwrap_or(0),
        ctx.config.nested(),
    );
    for cmd in &commands {
        for target in callback_targets_from_command(ctx, cmd) {
            let span = target.span;
            if span_contains_offset(span, cursor_offset) {
                return Some(target);
            }
        }
        for (inner_start, inner_end) in
            nested_dispatch_regions(ctx.source, ctx.analysis, ctx.dialect, cmd)
        {
            if let Some(target) = command_prefix_target_in_region(
                ctx,
                inner_start,
                inner_end,
                depth + 1,
                cursor_offset,
            ) {
                return Some(target);
            }
        }
    }
    None
}

/// A sole original receiver substitution retains its own selected child schema.
/// Method receiver applicability supplies no entered object or callback frame.
pub(super) fn original_self_receiver_word(ctx: MyMethodScan<'_>, head: tcl_lexer::Token) -> bool {
    use tcl_compiler::registry_invocation::source_structure;
    if head.kind != tcl_lexer::TokenType::Cmd {
        return false;
    }
    let Some(input) = ctx.analysis.resolved_input.as_ref() else {
        return false;
    };
    let context = input.context_registry();
    let regions =
        crate::executable_regions::command_substitution_regions(ctx.source, ctx.config, head);
    let [(start, end)] = regions.as_slice() else {
        return false;
    };
    let Ok(offset) = u32::try_from(*start) else {
        return false;
    };
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(
        &ctx.source[*start..*end],
        offset,
        ctx.config.nested(),
    );
    let [child] = commands.as_slice() else {
        return false;
    };
    let Some(words) = source_structure::source_registry_words(ctx.source, ctx.analysis, child)
    else {
        return false;
    };
    words.with_source_schema(&context, |schema| {
        let spec = schema.authored_source_descriptors().command;
        if spec.self_receiver_words.is_empty() {
            return false;
        }
        match schema.words.arguments().exact_argv_len() {
            Some(0) => spec.arity.min == 0,
            Some(1) => schema
                .words
                .arguments()
                .get(0)
                .and_then(tcl_registry::InvocationWord::literal)
                .is_some_and(|word| spec.self_receiver_words.contains(&word)),
            _ => false,
        }
    }) == Some(true)
}
