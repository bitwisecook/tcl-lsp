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

//! Where command substitutions sit: the commands inside an expression tree and the value and expression surfaces of one statement whose `[…]` run while it is evaluated.

use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Statement};
use tcl_registry::CommandRegistry;
use tcl_source::limits::MAX_EXPR_NODE_DEPTH;

/// Return `true` if *expr* contains at least one command
/// substitution (`[cmd ...]`). Used by CFG lowering to decide
/// whether a branch condition needs a synthetic `<cond>` placeholder
/// for emission-time startCommand wrapping.
#[must_use]
pub fn expr_has_command(expr: &ExprNode) -> bool {
    // Public entry: the top of an expression tree is nesting depth 0; the
    // recursion cap lives in [`expr_has_command_at`].
    expr_has_command_at(expr, 0)
}

fn expr_has_command_at(expr: &ExprNode, depth: u32) -> bool {
    // Native-stack safety net: past the cap, assume "yes, has a
    // command substitution" — the conservative direction, since callers use
    // this to decide whether a branch condition needs a synthetic `<cond>`
    // placeholder / startCommand wrapping; a false `true` only adds a
    // harmless wrapper, never drops a needed one.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    match expr {
        ExprNode::Command { .. } => true,
        ExprNode::Binary { left, right, .. } => {
            expr_has_command_at(left, depth + 1) || expr_has_command_at(right, depth + 1)
        }
        ExprNode::Unary { operand, .. } => expr_has_command_at(operand, depth + 1),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expr_has_command_at(condition, depth + 1)
                || expr_has_command_at(true_branch, depth + 1)
                || expr_has_command_at(false_branch, depth + 1)
        }
        ExprNode::Call { args, .. } => args.iter().any(|a| expr_has_command_at(a, depth + 1)),
        // A `"…"` operand substitutes, so a `[cmd]` in it runs: O126 dropped
        // `set y [expr {"[incr x]"}]` as an unused store, losing the `incr`
        // tclsh 8.6.18 performs (#2227, found in review).
        ExprNode::String { text, .. } => crate::word_subst::quoted_operand_body(text).is_some(),
        ExprNode::Literal { .. }
        | ExprNode::CompiledWord { .. }
        | ExprNode::Var { .. }
        | ExprNode::Raw { .. } => false,
    }
}

/// Recursively collect `ExprNode::Command` text from an expression tree.
///
/// Respects short-circuit evaluation: for `&&`/`||`, the RHS is
/// always included (conservative — we do not attempt compile-time
/// constant evaluation).
pub(crate) fn collect_expr_commands(expr: &ExprNode, out: &mut Vec<String>) {
    // Entry point: the top of an expression tree is nesting depth 0; the
    // recursion cap lives in [`collect_expr_commands_at`].
    collect_expr_commands_at(expr, out, 0);
}

/// Tcl source fragments owned by `stmt` whose `[...]` substitutions execute
/// while the statement is evaluated, plus whether a bounded expression walk
/// had to give up.
///
/// This is the shared statement-shape inventory for effect consumers.  It is
/// intentionally conservative: where structured IR does not retain enough
/// word quoting to prove a fragment inert, the fragment is included.  A false
/// positive only widens an effect summary; omitting an executed substitution
/// can make propagation carry a stale value across an arbitrary command.
/// Callers still lex the returned fragments with the active document's
/// [`LexerConfig`] before interpreting command words.
pub(crate) struct EvaluatedCommandSubstitutionSurfaces<'a> {
    /// Source fragments which may contain executed command substitutions.
    pub texts: Vec<&'a str>,
    /// The expression walk exceeded its shared recursion limit.
    pub opaque: bool,
    /// At least one collected surface is conditionally evaluated by Tcl
    /// short-circuit/ternary control flow or by a later `elseif` clause.
    pub conditional: bool,
}

fn push_command_substitution_text<'a>(text: &'a str, out: &mut Vec<&'a str>) {
    if text.contains('[') {
        out.push(text);
    }
}

pub(super) fn collect_expr_command_surface_refs<'a>(
    expr: &'a ExprNode,
    out: &mut Vec<&'a str>,
    opaque: &mut bool,
    conditional: &mut bool,
    depth: u32,
) {
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        *opaque = true;
        return;
    }
    let push_text = |text: &'a str, out: &mut Vec<&'a str>| {
        if text.contains('[') {
            out.push(text);
        }
    };
    match expr {
        ExprNode::Command { text, .. } => push_text(text, out),
        ExprNode::Raw { text } => {
            let before = out.len();
            push_text(text, out);
            if out.len() != before {
                // A raw expression has no retained evaluation tree, so the
                // collector cannot prove its substitutions unconditional.
                *conditional = true;
            }
        }
        ExprNode::Binary { op, left, right } => {
            collect_expr_command_surface_refs(left, out, opaque, conditional, depth + 1);
            let before_right = out.len();
            collect_expr_command_surface_refs(right, out, opaque, conditional, depth + 1);
            if matches!(
                op,
                crate::expr_ast::BinOp::And
                    | crate::expr_ast::BinOp::Or
                    | crate::expr_ast::BinOp::WordAnd
                    | crate::expr_ast::BinOp::WordOr
            ) && out.len() != before_right
            {
                *conditional = true;
            }
        }
        ExprNode::Unary { operand, .. } => {
            collect_expr_command_surface_refs(operand, out, opaque, conditional, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_expr_command_surface_refs(condition, out, opaque, conditional, depth + 1);
            let before_arms = out.len();
            collect_expr_command_surface_refs(true_branch, out, opaque, conditional, depth + 1);
            collect_expr_command_surface_refs(false_branch, out, opaque, conditional, depth + 1);
            if out.len() != before_arms {
                *conditional = true;
            }
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_expr_command_surface_refs(arg, out, opaque, conditional, depth + 1);
            }
        }
        // A word already reduced to its value. Braced, its brackets are data
        // and provably never run, so it is inert — the one case this may skip
        // without guessing. Unbraced, whatever substitution the word still owes
        // *is* executed at run time, and omitting an executed substitution is
        // the failure this inventory exists to avoid, so its text is collected
        // like any other surface.
        ExprNode::CompiledWord { text, braced } => {
            if !*braced {
                push_text(text, out);
            }
        }
        // `"…"` substitutes inside an expression and `{…}` does not, and the
        // variant carries its delimiters, so the delimiter decides. Reading
        // every string as inert hid the write in
        // `set y [expr {"[incr x]" + 0}]`: tclsh 8.6.18 prints `2` then `2`,
        // and O102 forwarded the stale `1` across it exactly as it did for a
        // braced operand before #2141 (#2118, found in review).
        ExprNode::String { text, .. } => {
            if let Some(inner) = crate::word_subst::quoted_operand_body(text) {
                out.push(inner);
            }
        }
        ExprNode::Literal { .. } | ExprNode::Var { .. } => {}
    }
}

fn collect_control_command_substitution_surfaces<'a>(
    stmt: &'a Statement,
    texts: &mut Vec<&'a str>,
    opaque: &mut bool,
    conditional: &mut bool,
) {
    match stmt {
        Statement::If { clauses, .. } => {
            for (index, clause) in clauses.iter().enumerate() {
                let before = texts.len();
                collect_expr_command_surface_refs(&clause.condition, texts, opaque, conditional, 0);
                if index != 0 && texts.len() != before {
                    *conditional = true;
                }
            }
        }
        Statement::For { condition, .. } | Statement::While { condition, .. } => {
            collect_expr_command_surface_refs(condition, texts, opaque, conditional, 0);
        }
        Statement::Foreach { iterators, .. } => {
            for iterator in iterators.iter().filter(|iterator| !iterator.list_braced) {
                push_command_substitution_text(&iterator.list_arg, texts);
            }
        }
        Statement::Catch {
            raw_args, tokens, ..
        } => {
            for arg in unbraced_words(raw_args, tokens.as_ref()) {
                push_command_substitution_text(arg, texts);
            }
        }
        Statement::Try { raw_args, .. } => {
            // `Try` does not retain per-word quoting. Include every raw word;
            // literal-body substitutions are harmless over-approximation and
            // the recursively lowered bodies supply the exact coverage.
            for arg in raw_args {
                push_command_substitution_text(arg, texts);
            }
        }
        Statement::Switch {
            subject,
            subject_braced,
            arms,
            ..
        } => {
            if !subject_braced {
                push_command_substitution_text(subject, texts);
            }
            for arm in arms.iter().filter(|arm| !arm.pattern_braced) {
                push_command_substitution_text(&arm.pattern, texts);
            }
        }
        _ => {}
    }
}

/// Enumerate every command-substitution-bearing value/expression surface of
/// one IR statement.
///
/// Nested *script bodies* are deliberately excluded: callers already recurse
/// through [`nested_bodies`] with the correct execution namespace.  This
/// helper covers the statement's own evaluated words and expression nodes.
#[must_use]
pub(crate) fn evaluated_command_substitution_surfaces<'a>(
    stmt: &'a Statement,
    registry: &CommandRegistry,
) -> EvaluatedCommandSubstitutionSurfaces<'a> {
    let mut texts = Vec::new();
    let mut opaque = false;
    let mut conditional = false;
    match stmt {
        Statement::NativeCall { .. } => opaque = true,
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            collect_expr_command_surface_refs(expr, &mut texts, &mut opaque, &mut conditional, 0);
        }
        Statement::AssignValue { value, .. } => push_command_substitution_text(value, &mut texts),
        Statement::Incr { amount, .. } => {
            if let Some(amount) = amount {
                push_command_substitution_text(amount, &mut texts);
            }
        }
        Statement::Call {
            command,
            canonical_command,
            args,
            tokens,
            ..
        }
        | Statement::Barrier {
            command,
            canonical_command,
            args,
            tokens,
            ..
        } => {
            for arg in invoked_word_surfaces(
                canonical_command.as_deref().unwrap_or(command.as_str()),
                args,
                tokens.as_ref(),
                registry,
            ) {
                push_command_substitution_text(arg, &mut texts);
            }
        }
        Statement::Return {
            value,
            expr,
            braced,
            ..
        } => {
            if !braced && let Some(value) = value {
                push_command_substitution_text(value, &mut texts);
            }
            if let Some(expr) = expr {
                collect_expr_command_surface_refs(
                    expr,
                    &mut texts,
                    &mut opaque,
                    &mut conditional,
                    0,
                );
            }
        }
        _ => collect_control_command_substitution_surfaces(
            stmt,
            &mut texts,
            &mut opaque,
            &mut conditional,
        ),
    }
    EvaluatedCommandSubstitutionSurfaces {
        texts,
        opaque,
        conditional,
    }
}

fn collect_expr_commands_at(expr: &ExprNode, out: &mut Vec<String>, depth: u32) {
    // Native-stack safety net: walks the `ExprNode` tree, one
    // native frame per level. Past the cap, stop descending — a collector
    // that returns the command texts gathered so far is the safe fallback
    // (substitutions buried deeper than the cap are not collected; never a
    // crash).
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return;
    }
    match expr {
        ExprNode::Command { text, .. } => {
            out.push(text.clone());
        }
        ExprNode::Binary { left, right, .. } => {
            collect_expr_commands_at(left, out, depth + 1);
            collect_expr_commands_at(right, out, depth + 1);
        }
        ExprNode::Unary { operand, .. } => {
            collect_expr_commands_at(operand, out, depth + 1);
        }
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            collect_expr_commands_at(condition, out, depth + 1);
            collect_expr_commands_at(true_branch, out, depth + 1);
            collect_expr_commands_at(false_branch, out, depth + 1);
        }
        ExprNode::Call { args, .. } => {
            for arg in args {
                collect_expr_commands_at(arg, out, depth + 1);
            }
        }
        _ => {}
    }
}

/// The words of a statement that keeps its own raw argv, minus the
/// brace-quoted ones: Tcl substitutes nothing inside braces, so a `[…]`
/// there is text the statement does not run.
pub(super) fn unbraced_words<'a>(
    args: &'a [String],
    tokens: Option<&CommandTokens>,
) -> impl Iterator<Item = &'a str> {
    let braced: Vec<bool> = (0..args.len())
        .map(|idx| tokens.is_some_and(|tokens| tokens.arg_is_braced_literal(idx)))
        .collect();
    args.iter()
        .enumerate()
        .filter(move |(idx, _)| !braced[*idx])
        .map(|(_, arg)| arg.as_str())
}

/// The argument words of an invocation that are this statement's own
/// substitution surface.
///
/// A *structural* body — one the registry says runs in a separate definition
/// or dispatch context rather than in this frame — is excluded. The module
/// doc already says bodies are [`nested_bodies`]' job; the invocation arm of
/// [`evaluated_command_substitution_surfaces`] simply never applied it.
///
/// `proc q {} {…}` is the case that matters. Its body is lowered into a
/// procedure of its own, so reading it here attributed the body's
/// substitutions to the enclosing `proc` statement: once those carried reads
/// as well as writes, `puts [incr m]` inside the body reported `W210 'm' is
/// read before it is set` against the caller's frame, on a program tclsh
/// 9.0.4 runs cleanly (it prints `2`).
///
/// Braced words are not argument substitution surfaces, even when they
/// contain a caller-frame body. Entered body and expression reads retain their
/// separate evaluation owners; nested scripts are walked at that execution
/// boundary rather than attributed to this command's argv.
fn invoked_word_surfaces<'a>(
    lookup: &str,
    args: &'a [String],
    tokens: Option<&CommandTokens>,
    registry: &CommandRegistry,
) -> impl Iterator<Item = &'a str> {
    let structural = crate::ssa::structural_body_indices(lookup, args, tokens, registry);
    args.iter()
        .enumerate()
        .filter(move |(idx, _)| {
            tokens.is_none_or(CommandTokens::evaluates_words)
                && !structural.contains(idx)
                && !tokens.is_some_and(|tokens| tokens.arg_is_braced_literal(*idx))
        })
        .map(|(_, arg)| arg.as_str())
}
