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

//! Warnings for a block's branch condition and the command substitutions nested in it.

use super::super::{
    BlockId, CfgFunction, ExprNode, HashMap, MAX_EXPR_NODE_DEPTH, SinkCall, SourceWarningRead,
    Span, Symbol, TaintScan, TaintWarning, Terminator, all_argument_candidates,
    parse_command_substitution_with_config,
};
use super::BranchTaintCtx;
use super::classification::{classify_sink, classify_source_sink};
use super::numeric_coercion::{
    CoercionOperand, CoercionTaint, emit_expr_coercion_warnings, expression_read_taint,
};
use super::sink_emission::emit_sink_warnings;
use super::source_reads::SourceWarningMetadata;

/// Emit taint warnings for a block's `if`/`while`/`for` branch condition:
/// T100 for a numeric-coercion-risk operand directly in the condition
/// expression, plus the full sink family for any nested `[cmd …]`
/// command substitution the condition contains (see
/// [`emit_branch_condition_nested_command_warnings`]).
///
/// Branch conditions live on `Terminator::Branch`, not in
/// `ssa_block.statements` — `find_taint_warnings`'s per-statement scan
/// never reaches them, so without this dedicated pass a condition like
/// `if {$tainted + 1 > 5} { … }` raised no T100 at all, even though it is
/// evaluated exactly like any other braced `expr`. Native conditions retain
/// their parser base and per-reference read carriers; each occurrence resolves
/// independently at the terminator. Only carrierless compatibility SSA uses
/// represented exit versions. Missing native evidence never invents version0.
pub(in super::super) fn emit_branch_condition_warnings<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    bn: BlockId,
    ctx: &BranchTaintCtx<'_, S>,
    warnings: &mut Vec<TaintWarning>,
) {
    let Some(block) = cfg.blocks.get(&bn) else {
        return;
    };
    let Some(Terminator::Branch {
        condition,
        condition_base,
        span,
        ..
    }) = &block.terminator
    else {
        return;
    };
    let Some(fallback_span) = span else {
        return;
    };
    let source = crate::ssa::SsaSourceView::at_terminator(ctx.ssa, bn);
    let resolve = |hit: &CoercionOperand| {
        if ctx.ssa.point_contexts.is_some() {
            return expression_read_taint(
                source,
                hit.variable.as_ref()?,
                *condition_base,
                ctx.registry,
                ctx.taints,
            );
        }
        let sym = source.symbol(&hit.name)?;
        ctx.ssa_block
            .exit_versions
            .get(&sym)
            .copied()
            .map(|version| CoercionTaint::represented(sym, version, ctx.taints))
    };
    emit_expr_coercion_warnings(
        condition,
        *condition_base,
        *fallback_span,
        resolve,
        warnings,
        (
            ctx.source.config,
            ctx.source.config.grammar_over(
                ctx.dialect
                    .map_or_else(tcl_dialect::LexerGrammar::default, |profile| {
                        profile.grammar
                    }),
            ),
        ),
    );
    let Some(source_tokens) = cfg.source_input_tokens_at(bn, usize::MAX) else {
        if ctx.source.allows_nominal_metadata() {
            emit_branch_condition_nested_command_warnings_standalone(
                condition,
                ctx,
                *fallback_span,
                warnings,
            );
        }
        return;
    };
    emit_branch_condition_nested_command_warnings(
        bn,
        condition,
        *condition_base,
        source_tokens,
        ctx,
        *fallback_span,
        warnings,
    );
}

/// Emit sink warnings for command substitutions nested inside a branch
/// condition. Source-backed analysis joins original expression spans to the
/// retained branch invocation, exact dispatch, selected Registry descriptor,
/// and point-specific SSA reads. Missing evidence is unavailable; detached
/// standalone callers keep their explicit compatibility path.
struct BranchExpressionSource<'a> {
    base: u32,
    pub(in super::super) tokens: &'a crate::ir::CommandTokens,
    pub(in super::super) config: tcl_lexer::LexerConfig,
    evaluation: &'a crate::command_binding::SourceConditionalExpressionEvaluation,
}

pub(super) fn emit_branch_condition_nested_command_warnings<S: std::hash::BuildHasher>(
    bn: BlockId,
    condition: &ExprNode,
    condition_base: Option<u32>,
    source_tokens: &crate::ir::CommandTokens,
    ctx: &BranchTaintCtx<'_, S>,
    fallback_span: Span,
    warnings: &mut Vec<TaintWarning>,
) {
    if ctx.source.allows_nominal_metadata() {
        emit_branch_condition_nested_command_warnings_standalone(
            condition,
            ctx,
            fallback_span,
            warnings,
        );
        return;
    }
    let Some(base) = condition_base else {
        return;
    };
    let Some(actual_config) = source_tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.original_lexer_config_for_tokens(source_tokens))
        .filter(|config| *config == ctx.source.config)
    else {
        return;
    };
    let Some(binding) = source_tokens.source_binding.as_ref() else {
        return;
    };
    let Some(invocation_site) = binding.invocation_site() else {
        return;
    };
    let Some(evaluation) = binding.conditional_expression_evaluation(ctx.registry, source_tokens)
    else {
        return;
    };
    let expression_source = evaluation.source();
    let Some(expression_start) = usize::try_from(base).ok() else {
        return;
    };
    let Some(expression_end) = expression_start.checked_add(expression_source.text.bytes().len())
    else {
        return;
    };
    if !evaluation.semantic_lookup_closed()
        || evaluation.tree() != condition
        || expression_source.base() != base
        || !std::sync::Arc::ptr_eq(&expression_source.origin, &invocation_site.source)
        || expression_source
            .origin
            .source_image()
            .bytes()
            .get(expression_start..expression_end)
            != Some(expression_source.text.bytes())
    {
        return;
    }
    let source = BranchExpressionSource {
        base,
        tokens: source_tokens,
        config: actual_config,
        evaluation: &evaluation,
    };
    let mut commands = Vec::new();
    collect_branch_expression_commands(evaluation.tree(), &mut commands, 0);
    for command in commands {
        emit_one_branch_condition_command(command, &source, bn, ctx, warnings);
    }
}

fn emit_one_branch_condition_command<S: std::hash::BuildHasher>(
    command_node: (String, u32, u32),
    source_owner: &BranchExpressionSource<'_>,
    bn: BlockId,
    ctx: &BranchTaintCtx<'_, S>,
    warnings: &mut Vec<TaintWarning>,
) {
    let (text, relative_start, relative_end) = command_node;
    let Some(start) = source_owner.base.checked_add(relative_start) else {
        return;
    };
    let Some(end) = source_owner.base.checked_add(relative_end) else {
        return;
    };
    let Some(nested) = retained_branch_command_tokens(&text, start, end, source_owner) else {
        return;
    };
    let Some(metadata) = ctx.source.metadata_context() else {
        return;
    };
    let Some(selected) =
        crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
            ctx.registry,
            Some(metadata),
            &nested,
        )
    else {
        return;
    };
    let command = selected.facts.canonical_command.clone();
    let Some(written_args) = nested.argv_texts.get(1..) else {
        return;
    };
    let Some(args) = selected.effective.argument_spellings(written_args) else {
        return;
    };
    let source = crate::ssa::SsaSourceView::at_terminator(ctx.ssa, bn);
    let Some(uses) =
        branch_source_read_versions(&nested, source_owner.tokens, Span::new(start, end), source)
    else {
        return;
    };
    let Some(metadata_context) = ctx.source.metadata_context() else {
        return;
    };
    let Some(source_operands) = crate::registry_invocation::original_expression_taint_read_operands(
        ctx.registry,
        metadata_context,
        source_owner.tokens,
        &nested,
        source_owner.evaluation,
        &text,
        Span::new(start, end),
    ) else {
        return;
    };
    let source_metadata = SourceWarningMetadata::Original(Box::new(source_operands));
    let source_read = SourceWarningRead {
        metadata: &source_metadata,
        point: source,
    };
    let call = SinkCall {
        source: ctx.source,
        command: &command,
        args: &args,
        registry: ctx.registry,
        tokens: Some(&nested),
        effective: Some(&selected.effective),
        source_read: Some(source_read),
        argument_offset: 0,
    };
    let Some((code, sink_label)) = classify_source_sink(&call, ctx.dialect) else {
        return;
    };
    let env = TaintScan {
        uses: &uses,
        taints: ctx.taints,
        ssa: ctx.ssa,
        quoted_uses: None,
        source_read: Some(source_read),
        source: ctx.source,
        withdrawn_writes: None,
    };
    emit_sink_warnings(
        &env,
        Span::new(start, end),
        code,
        &sink_label,
        &call,
        warnings,
    );
}

fn retained_branch_command_tokens(
    text: &str,
    start: u32,
    end: u32,
    owner: &BranchExpressionSource<'_>,
) -> Option<crate::ir::CommandTokens> {
    if text.len() != end.saturating_sub(start) as usize
        || owner
            .evaluation
            .source()
            .origin
            .source_image()
            .bytes()
            .get(start as usize..end as usize)
            != Some(text.as_bytes())
        || owner.tokens.source_binding.as_ref().is_none_or(|binding| {
            binding.invocation_site().is_none_or(|site| {
                site.source
                    .source_image()
                    .bytes()
                    .get(start as usize..end as usize)
                    != Some(text.as_bytes())
            })
        })
    {
        return None;
    }
    let source_site = tcl_source::words::SourceSite::source(Span::new(start, end));
    let mut nested =
        crate::word_subst::nested_command_words(text, &source_site, owner.config).ok()?;
    nested.inherit_nested_bindings(owner.tokens);
    let binding = nested.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    (std::sync::Arc::ptr_eq(&site.source, &owner.evaluation.source().origin)
        && site.offset >= start
        && site.offset < end
        && binding.invocation_realm()
            == Some(tcl_dialect::model::InvocationRealm::InterpreterRuntime))
    .then_some(nested)
}

fn branch_source_read_versions(
    nested: &crate::ir::CommandTokens,
    parent_tokens: &crate::ir::CommandTokens,
    span: Span,
    source: crate::ssa::SsaSourceView<'_>,
) -> Option<HashMap<Symbol, u32>> {
    let binding = nested.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let parent_binding = parent_tokens.source_binding.as_ref()?;
    let parent_site = parent_binding.invocation_site()?;
    if !std::sync::Arc::ptr_eq(&site.source, &parent_site.source) {
        return None;
    }
    let actual_reads = parent_tokens
        .variable_accesses
        .iter()
        .filter(|access| {
            access.owner.is_expression_evaluation_of(parent_site)
                && access.source.span.start() >= span.start()
                && access.source.span.end() <= span.end()
        })
        .cloned()
        .collect::<Vec<_>>();
    let projected_reads = nested
        .variable_accesses
        .iter()
        .filter(|access| {
            access.source.span.start() >= span.start() && access.source.span.end() <= span.end()
        })
        .collect::<Vec<_>>();
    if actual_reads.len() != projected_reads.len()
        || actual_reads.iter().any(|actual| {
            !projected_reads.iter().any(|projected| {
                projected.source == actual.source
                    && projected.original_spelling == actual.original_spelling
                    && projected.owner == actual.owner
            })
        })
    {
        return None;
    }
    let mut uses = HashMap::new();
    for access in actual_reads {
        if access.source.span.start() < span.start() || access.source.span.end() > span.end() {
            return None;
        }
        let read = source.read_reference(&access.source, &access.original_spelling)?;
        let version = read.version?;
        if uses
            .get(&read.symbol)
            .is_some_and(|previous| *previous != version)
        {
            return None;
        }
        uses.insert(read.symbol, version);
    }
    Some(uses)
}

fn collect_branch_expression_commands(
    node: &ExprNode,
    out: &mut Vec<(String, u32, u32)>,
    depth: u32,
) {
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match node {
        ExprNode::Command { text, start, end } => out.push((text.clone(), *start, *end)),
        ExprNode::Binary { left, right, .. } => {
            collect_branch_expression_commands(left, out, depth + 1);
            collect_branch_expression_commands(right, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => {
            collect_branch_expression_commands(operand, out, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_branch_expression_commands(condition, out, depth + 1);
            collect_branch_expression_commands(true_branch, out, depth + 1);
            collect_branch_expression_commands(false_branch, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_branch_expression_commands(arg, out, depth + 1);
            }
        }
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. } => {}
    }
}

/// Explicitly detached compatibility callers retain the historical scalar
/// behavior. This branch is unreachable for source-backed functions, whose
/// missing carrier never falls through to nominal metadata or exit version 0.
fn emit_branch_condition_nested_command_warnings_standalone<S: std::hash::BuildHasher>(
    condition: &ExprNode,
    ctx: &BranchTaintCtx<'_, S>,
    fallback_span: Span,
    warnings: &mut Vec<TaintWarning>,
) {
    for text in condition.command_texts() {
        let Some((command, call_args)) =
            parse_command_substitution_with_config(&text, ctx.source.config)
        else {
            continue;
        };
        let Some((code, sink_label)) =
            classify_sink(ctx.registry, &command, &call_args, ctx.dialect)
        else {
            continue;
        };
        let config = ctx.source.config;
        let mut uses = HashMap::new();
        for name in all_argument_candidates(&text, config, ctx.ssa) {
            if let Some(sym) = ctx.ssa.var_symbol(&name) {
                let version = ctx.ssa_block.exit_versions.get(&sym).copied().unwrap_or(0);
                uses.insert(sym, version);
            }
        }
        let env = TaintScan {
            uses: &uses,
            taints: ctx.taints,
            ssa: ctx.ssa,
            quoted_uses: None,
            source_read: None,
            source: ctx.source,
            withdrawn_writes: None,
        };
        let sink_call = SinkCall {
            source: ctx.source,
            command: &command,
            args: &call_args,
            registry: ctx.registry,
            tokens: None,
            effective: None,
            source_read: None,
            argument_offset: 0,
        };
        emit_sink_warnings(&env, fallback_span, code, &sink_label, &sink_call, warnings);
    }
}
