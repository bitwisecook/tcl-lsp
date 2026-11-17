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

//! The taint of a command substitution: sources, sanitisers, transforms, boolean results and the argument walk.

use super::definition_taint::{interproc_call_taint, positional_argument_taint};
use super::registry_colours::{reg_colour, source_colour, transform_colour};
use super::word_taint::word_taint;
use super::{TaintColour, TaintCtx, TaintLattice, is_sanitiser};
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::ExprNode;
use crate::ir::CommandTokens;
use crate::ssa::{Symbol, ValueKey};
use std::collections::HashMap;
use tcl_lexer::Span;
use tcl_registry::CommandRegistry;

pub(super) fn expr_yields_boolean(expr: &ExprNode) -> bool {
    match expr {
        ExprNode::Binary { op, .. } => op.yields_boolean(),
        ExprNode::Unary { op, .. } => op.yields_boolean(),
        _ => false,
    }
}

/// `[expr {$a eq $b}]` yields only `0`/`1`, whatever its operands held.
fn expression_invocation_yields_boolean(
    invocation: Option<&crate::registry_invocation::ResolvedStatementInvocation>,
    ctx: TaintCtx<'_>,
) -> bool {
    invocation
        .and_then(|invocation| {
            crate::word_subst::source_expression_from_invocation(
                invocation,
                ctx.dialect,
                tcl_lexer::Span::new(0, 0),
                None,
            )
        })
        .is_some_and(|lifted| expr_yields_boolean(&lifted.expression))
}

/// Join the taint contributed by every command substitution embedded in an
/// expression AST (`[cmd …]` retained as [`ExprNode::Command`], or `[cmd …]`
/// inside a quoted `ExprNode::String`).
///
/// Variable references are intentionally *not* re-scanned here — they are
/// already covered by the `AssignExpr` statement's SSA `uses` join — so this
/// adds only the command-substitution taint that variable joining misses.
pub(super) fn expr_command_taint<S: std::hash::BuildHasher>(
    node: &ExprNode,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    base: Option<u32>,
    depth: u32,
) -> TaintLattice {
    // Native-stack safety net: this walks the `ExprNode`
    // operator tree, one native frame per level. Past the cap, assume
    // tainted — the conservative direction, so a command substitution nested
    // deeper than we can walk can't launder taint into a false negative.
    // (Calls into `word_taint` below re-enter that walker at bracket-depth 0:
    // a leaf node's raw text is a separate, independent recursion axis.)
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return TaintLattice::tainted();
    }
    match node {
        // A bracketed command substitution is classified exactly as a word; an
        // unparseable `Raw` fallback is treated the same way, scanning its text
        // for `[cmd]` substitutions.
        ExprNode::Command { text, start, .. } => {
            let ctx = TaintCtx {
                word_offset: base.and_then(|base| base.checked_add(*start)),
                ..ctx
            };
            word_taint(text, uses, taints, ctx)
        }
        ExprNode::Raw { text } => word_taint(text, uses, taints, ctx),
        // A quoted string undergoes substitution, so an embedded `[cmd]`
        // counts; a braced `{…}` string is literal (no substitution) and is
        // skipped to avoid a false positive.
        ExprNode::String { text, .. } => {
            if tcl_syntax::expr::quoted_string_body(text).is_some() {
                word_taint(text, uses, taints, ctx)
            } else {
                TaintLattice::clean()
            }
        }
        // A word already reduced to its value. A braced one is literal, so
        // nothing can flow into it; otherwise whatever substitution is still
        // live is what a taint can arrive through, which is the same question
        // `word_taint` answers for a quoted operand.
        ExprNode::CompiledWord { text, braced } => {
            if *braced {
                TaintLattice::clean()
            } else {
                word_taint(text, uses, taints, ctx)
            }
        }
        ExprNode::Binary { left, right, .. } => {
            expr_command_taint(left, uses, taints, ctx, base, depth + 1).join(expr_command_taint(
                right,
                uses,
                taints,
                ctx,
                base,
                depth + 1,
            ))
        }
        ExprNode::Unary { operand, .. } => {
            expr_command_taint(operand, uses, taints, ctx, base, depth + 1)
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
            ..
        } => expr_command_taint(condition, uses, taints, ctx, base, depth + 1)
            .join(expr_command_taint(
                true_branch,
                uses,
                taints,
                ctx,
                base,
                depth + 1,
            ))
            .join(expr_command_taint(
                false_branch,
                uses,
                taints,
                ctx,
                base,
                depth + 1,
            )),
        ExprNode::Call { args, .. } => args.iter().fold(TaintLattice::clean(), |acc, a| {
            acc.join(expr_command_taint(a, uses, taints, ctx, base, depth + 1))
        }),
        // Literals and variable references carry no command substitution
        // (variables are covered by the statement's `uses` join).
        ExprNode::Literal { .. } | ExprNode::Var { .. } => TaintLattice::clean(),
    }
}

pub(super) fn retained_taint_substitution(word: &str, ctx: TaintCtx<'_>) -> Option<CommandTokens> {
    let parent = ctx.source_tokens?;
    let mut nested = if let Some(source_word) = ctx.source_word {
        crate::word_subst::whole_word_command_tokens(source_word, ctx.lexer_config())?
    } else {
        let offset = ctx.word_offset?;
        let span = Span::new(offset, offset.checked_add(u32::try_from(word.len()).ok()?)?);
        crate::word_subst::nested_command_words(
            word,
            &crate::ir::SourceSite::source(span),
            ctx.lexer_config(),
        )
        .ok()?
    };
    nested.inherit_nested_bindings(parent);
    Some(nested)
}

fn retained_normal_taint(
    tokens: &CommandTokens,
    ctx: TaintCtx<'_>,
) -> Option<crate::registry_invocation::NormalTaintInvocation> {
    if ctx.source.allows_nominal_metadata() && ctx.source.metadata.is_none() {
        return crate::registry_invocation::normal_taint_invocation(ctx.registry, None, tokens);
    }
    crate::registry_invocation::normal_taint_invocation_with_metadata_context(
        ctx.registry,
        ctx.source.metadata,
        tokens,
    )
}

fn normal_result_taint(
    normal: &crate::registry_invocation::NormalTaintInvocation,
) -> Option<TaintLattice> {
    if let Some(colour) = normal.source_colour() {
        return Some(TaintLattice::from_colours(reg_colour(colour)));
    }
    normal.is_sanitiser().then(TaintLattice::clean)
}

fn hosted_substitution_is_taint_source(
    metadata: &crate::registry_invocation::OriginalConditionalVendorRegistryMetadata,
    effective: &crate::registry_invocation::EffectiveCommandWords,
    args: &[String],
    registry: &CommandRegistry,
) -> bool {
    // Conditional source getters cannot issue sanitiser or shape safety.
    // May-taint remains independent of Native dispatch and effects.
    let args: Vec<_> = effective.argument_presentations(args).unwrap_or_default();
    let known: Option<Vec<_>> = args.iter().map(|value| value.as_deref()).collect();
    metadata.shape().possible_traits().intersects(
        tcl_registry::Traits::TAINT_SOURCE
            | tcl_registry::Traits::UNNORMALISED_HTTP_GETTER
            | tcl_registry::Traits::TAINT_SOURCE_ZERO_ARGS,
    ) || known.as_deref().is_some_and(|args| {
        tcl_registry::taint::is_taint_source(registry, metadata.shape().command(), args, None)
    })
}

fn substitution_transform_colour(
    normal: Option<&crate::registry_invocation::NormalTaintInvocation>,
    ctx: TaintCtx<'_>,
    (cmd, hosted): (&str, bool),
    arg_refs: &[&str],
) -> Option<TaintColour> {
    normal
        .and_then(crate::registry_invocation::NormalTaintInvocation::transform_colour)
        .map(reg_colour)
        .or_else(|| {
            (hosted || (ctx.source.allows_nominal_metadata() && ctx.source_tokens.is_none()))
                .then(|| transform_colour(ctx.registry, cmd, arg_refs))
                .flatten()
        })
}

fn substitution_arguments_taint<S: std::hash::BuildHasher>(
    args: &[String],
    effective: Option<&crate::registry_invocation::EffectiveCommandWords>,
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    (nested, ctx): (Option<&CommandTokens>, TaintCtx<'_>),
    depth: u32,
) -> TaintLattice {
    let mut t = TaintLattice::clean();
    for index in 0..args.len() {
        let arg_ctx = TaintCtx {
            source_tokens: nested.or(ctx.source_tokens),
            ..ctx
        };
        t = t.join(positional_argument_taint(
            index,
            args,
            effective,
            uses,
            taints,
            arg_ctx,
            depth + 1,
        ));
    }
    t
}

fn finish_substitution_taint(
    mut t: TaintLattice,
    normal: Option<&crate::registry_invocation::NormalTaintInvocation>,
    ctx: TaintCtx<'_>,
    cmd: (&str, bool),
    arg_refs: &[&str],
) -> TaintLattice {
    t = t.shape_unproven();
    // Stamp the encoder/transform colour the command adds to its result
    // (e.g. `uri::encode` → `URL_ENCODED`, `file normalize` →
    // `PATH_NORMALISED`), so a later pass through the same encoder is
    // detectable as a double-encode (T106) and a consumer asking "has
    // this value been through a normalising sanitiser" gets a yes.
    //
    // Stamped regardless of taint.  The colour describes what the
    // *command* guarantees about its result, not what its input was, so
    // gating it on `is_tainted()` would make the fact unobservable for
    // exactly the values a hygiene check like W201 asks about — a path
    // built from clean local variables and then normalised.  Nothing
    // widens as a result: every consumer that acts on a colour
    // ([`emit_double_encode_warnings`], the sink mitigation checks)
    // requires `is_tainted()` first, and a clean lattice is `join`'s
    // identity, so a clean colour never dilutes a tainted operand's
    // must-have mitigations.
    let transform = substitution_transform_colour(normal, ctx, cmd, arg_refs);
    if let Some(colour) = transform {
        t = t.with(colour);
    }
    t
}

/// The taint a handler's vendor command settles without an argument walk.
/// It has no resolved invocation; its guarded hosted row classifies the
/// result for this diagnostic alone: a sanitiser is clean and a getter
/// carries its colour. No rewrite, fold or proof reads these.
fn hosted_result_taint(
    (metadata, effective): (
        &crate::registry_invocation::OriginalConditionalVendorRegistryMetadata,
        &crate::registry_invocation::EffectiveCommandWords,
    ),
    written: &[String],
    args: &[String],
    ctx: TaintCtx<'_>,
) -> Option<TaintLattice> {
    let command = metadata.shape().command();
    let refs: Vec<&str> = written.iter().map(String::as_str).collect();
    if is_sanitiser(ctx.registry, command, &refs) {
        return Some(TaintLattice::clean());
    }
    source_colour(
        ctx.registry,
        command,
        &refs,
        ctx.dialect,
        None,
        ctx.source_position,
        None,
    )
    .or_else(|| {
        hosted_substitution_is_taint_source(metadata, effective, args, ctx.registry)
            .then(TaintLattice::tainted)
    })
}

fn reaches_only_a_catalogued_absent_head(
    nested: Option<&CommandTokens>,
    ctx: TaintCtx<'_>,
) -> bool {
    nested
        .and_then(|nested| nested.source_binding.as_ref())
        .is_some_and(|binding| binding.reaches_only_a_catalogued_absent_head(ctx.registry))
}

/// The source colour of a resolved command substitution, read with its
/// receiver word and position.
fn substitution_source_colour(
    cmd: &str,
    arg_refs: &[&str],
    nested: Option<&CommandTokens>,
    ctx: TaintCtx<'_>,
) -> Option<TaintLattice> {
    source_colour(
        ctx.registry,
        cmd,
        arg_refs,
        ctx.dialect,
        ctx.instance_classes,
        ctx.source_position,
        nested
            .and_then(|tokens| tokens.words().first())
            .map(|word| (word, ctx.source.config)),
    )
}

/// Everything a command substitution's taint needs before the recursive
/// argument walk, or its answer when no walk is needed. A leaf, so these
/// temporaries are not held on every level of nested substitution.
#[inline(never)]
fn classify_substitution_taint(
    word: &str,
    invocation: (&str, &[String]),
    ctx: TaintCtx<'_>,
) -> Result<Box<SubstitutionTaintPlan>, TaintLattice> {
    let (cmd, args) = invocation;
    if !ctx.source.allows_nominal_metadata() && ctx.source.metadata.is_none() {
        return Err(TaintLattice::tainted());
    }
    let nested = retained_taint_substitution(word, ctx);
    let normal = nested
        .as_ref()
        .and_then(|tokens| retained_normal_taint(tokens, ctx));
    if let Some(taint) = normal.as_ref().and_then(normal_result_taint) {
        return Err(taint);
    }
    let resolved = nested.as_ref().and_then(|nested| {
        crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
            ctx.registry,
            ctx.source.metadata,
            nested,
        )
    });
    if expression_invocation_yields_boolean(resolved.as_ref(), ctx) {
        return Err(TaintLattice::clean());
    }
    let hosted = nested
        .as_ref()
        .filter(|_| ctx.source.metadata.is_some())
        .and_then(|tokens| {
            crate::registry_invocation::hosted_source_taint_invocation(ctx.registry, tokens)
        });
    let effective = nested
        .as_ref()
        .and_then(crate::registry_invocation::effective_command_words);
    let effective_args = effective
        .as_ref()
        .and_then(|effective| effective.argument_spellings(args));
    let point_target = nested
        .as_ref()
        .and_then(|nested| nested.source_binding.as_ref())
        .and_then(crate::command_binding::SourceInvocationBinding::proved_target);
    let registry_allowed =
        (ctx.source.allows_nominal_metadata() && ctx.source_tokens.is_none()) || resolved.is_some();
    if let Some(hosted) = hosted.as_ref() {
        let written = effective_args.as_deref().unwrap_or(args);
        let subject = (&hosted.metadata, &hosted.effective);
        return match hosted_result_taint(subject, written, args, ctx) {
            Some(taint) => Err(taint),
            None => Ok(Box::new(SubstitutionTaintPlan {
                cmd: hosted.metadata.shape().command().to_owned(),
                hosted: Some(hosted.effective.clone()),
                effective_args,
                effective,
                normal,
                nested,
            })),
        };
    }
    if ctx.source_tokens.is_some()
        && point_target.is_none()
        && normal.is_none()
        && !reaches_only_a_catalogued_absent_head(nested.as_ref(), ctx)
    {
        // Missing word provenance or an explicit unknown target cannot
        // inherit a sanitiser/source/transform from the written name.
        return Err(TaintLattice::tainted());
    }
    let cmd = resolved
        .as_ref()
        .map(|resolved| resolved.facts.canonical_command.as_str())
        .or_else(|| point_target.map(|target| target.command.as_str()))
        .unwrap_or(cmd);
    let args = effective_args.as_deref().unwrap_or(args);
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    if ctx.source.allows_nominal_metadata()
        && ctx.source_tokens.is_none()
        && is_sanitiser(ctx.registry, cmd, &arg_refs)
    {
        return Err(TaintLattice::clean());
    }
    if registry_allowed
        && let Some(t) = substitution_source_colour(cmd, &arg_refs, nested.as_ref(), ctx)
    {
        return Err(t);
    }
    Ok(Box::new(SubstitutionTaintPlan {
        cmd: cmd.to_owned(),
        hosted: None,
        effective_args,
        effective,
        normal,
        nested,
    }))
}

/// What [`substitution_word_taint`] carries across its recursive argument walk.
struct SubstitutionTaintPlan {
    cmd: String,
    /// `cmd` is a handler's vendor command, classified from its hosted row;
    /// the effective words of that row.
    pub(super) hosted: Option<crate::registry_invocation::EffectiveCommandWords>,
    pub(super) effective_args: Option<Vec<String>>,
    pub(super) effective: Option<crate::registry_invocation::EffectiveCommandWords>,
    normal: Option<crate::registry_invocation::NormalTaintInvocation>,
    nested: Option<CommandTokens>,
}

pub(super) fn substitution_word_taint<S: std::hash::BuildHasher>(
    word: &str,
    invocation: (&str, &[String]),
    uses: &HashMap<Symbol, u32>,
    taints: &HashMap<ValueKey, TaintLattice, S>,
    ctx: TaintCtx<'_>,
    depth: u32,
) -> TaintLattice {
    let plan = match classify_substitution_taint(word, invocation, ctx) {
        Ok(plan) => plan,
        Err(taint) => return taint,
    };
    let cmd = plan.cmd.as_str();
    let args = plan.effective_args.as_deref().unwrap_or(invocation.1);
    // Interprocedural: if `cmd` resolves to a known proc with a
    // passthrough parameter, propagate the taint of the matching
    // actual.
    if let Some(t) = interproc_call_taint(
        cmd,
        args,
        plan.effective.as_ref(),
        uses,
        taints,
        TaintCtx {
            source_tokens: plan.nested.as_ref().or(ctx.source_tokens),
            ..ctx
        },
        depth + 1,
    ) {
        return t;
    }
    // Propagate from the arguments inside the command sub. `cmd` has
    // no registry classification (source / sanitiser / passthrough all
    // missed above), so its effect on argument shape is unknown —
    // strip shape-dependent mitigations (`shape_unproven`) rather than
    // optimistically assuming e.g. `PATH_PREFIXED` survives an
    // arbitrary `string range` / `lindex [split ...]` transform.
    let t = substitution_arguments_taint(
        args,
        plan.effective.as_ref(),
        uses,
        taints,
        (plan.nested.as_ref(), ctx),
        depth,
    );
    let values = plan
        .hosted
        .as_ref()
        .map(|effective| hosted_argument_spellings(effective, args));
    let arg_refs: Vec<&str> = values
        .as_deref()
        .unwrap_or(args)
        .iter()
        .map(String::as_str)
        .collect();
    finish_substitution_taint(
        t,
        plan.normal.as_ref(),
        ctx,
        (cmd, plan.hosted.is_some()),
        &arg_refs,
    )
}

/// A handler command's arguments as its transform condition reads them: the
/// written spelling, with a brace group's braces, which the raw condition
/// requires before it trusts a word's text as its value.
pub(super) fn hosted_argument_spellings(
    effective: &crate::registry_invocation::EffectiveCommandWords,
    args: &[String],
) -> Vec<String> {
    args.iter()
        .enumerate()
        .map(|(index, spelling)| match effective.words.get(index + 1) {
            Some(crate::ir::WordExpr::BracedLiteral { text, .. }) => format!("{{{text}}}"),
            _ => spelling.clone(),
        })
        .collect()
}
