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

//! v3 — α-renaming for inlined IR bodies.
//!
//! When a callee's body is
//! spliced into a caller's IR, the callee's parameter and local names
//! must be rewritten so they don't collide with the caller's locals.
//! This module walks every [`Statement`] shape and produces a clone with
//! the rename map applied to:
//!
//! * `name` fields on every assignment / [`Statement::Incr`] /
//!   [`Statement::Call`] `defs` / `reads` / [`Statement::Foreach`]
//!   iterator var lists / [`Statement::Catch`] / [`Statement::Try`]
//!   exception bindings;
//! * every embedded `$name` / `${name}` substitution in string-valued
//!   fields (`AssignValue.value`, `Call.args`, `Return.value`,
//!   `Switch.subject`, …);
//! * every [`ExprNode::Var`] node in embedded expression ASTs
//!   (`AssignExpr.expr`, `If` / `For` / `While` conditions,
//!   `Return.expr`, `ExprEval.expr`).
//!
//! The rename map applies only to names listed as keys. Any name not in
//! the map (a global `::ns::var`, a namespace variable, a name that's
//! neither parameter nor local of the callee) passes through verbatim.
//!
//! The IR is owned rather than structurally shared, so this walker always
//! rebuilds every sub-tree instead of skipping untouched ones; the rebuilt
//! body is value-identical to the input.

use std::collections::HashMap;

use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::ExprNode;
use crate::ir::{IfClause, Script, Statement, SwitchArm, TryHandler};

/// Return a new [`Script`] with `rename` applied everywhere reachable.
#[must_use]
pub(super) fn rewrite_script(
    script: &Script,
    names: &HashMap<String, String>,
    config: tcl_lexer::LexerConfig,
) -> Script {
    rewrite_script_in(script, &Rename { names, config })
}

struct Rename<'a> {
    names: &'a HashMap<String, String>,
    config: tcl_lexer::LexerConfig,
}

impl std::ops::Deref for Rename<'_> {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        self.names
    }
}

fn rewrite_script_in(script: &Script, rename: &Rename<'_>) -> Script {
    if rename.is_empty() {
        return script.clone();
    }
    Script {
        namespace_context: script.namespace_context.clone(),
        executed_source: script.executed_source.clone(),
        implicit_math_invocations: script.implicit_math_invocations.clone(),
        expression_preparations: script.expression_preparations.clone(),
        statements: script
            .statements
            .iter()
            .map(|s| rewrite_stmt(s, rename))
            .collect(),
        command_binding_sites: script.command_binding_sites.clone(),
        procedure_binding_requirements: script.procedure_binding_requirements.clone(),
        native_compilation_failure: script.native_compilation_failure.clone(),
        native_compilation_admission: script.native_compilation_admission.clone(),
    }
}

/// Apply `rename` to a variable-name field, handling array-element
/// references. For `arr(idx)` shapes the array base carries the binding
/// identity and the `(idx)` suffix is a substituted context whose own `$var`
/// references are α-renamed too (see [`rewrite_array_index_tail`]).
fn rename_var_name(name: &str, rename: &Rename<'_>) -> String {
    rename_name(name, true, rename)
}

fn rename_name(name: &str, substitutes_index: bool, rename: &Rename<'_>) -> String {
    if let Some((base, _)) = tcl_syntax::naming::split_element_ref(name) {
        let tail = &name[base.len()..];
        // Both the array base *and* any `$var` inside the index need the
        // rename: `set arr($idx) …` substitutes `$idx`, so an inlined body's
        // index reference must map to the renamed inline local, not capture the
        // caller's variable. The base is renamed only when it
        // is a tracked local; the tail is rewritten regardless (its `$var`
        // might be a local even when the base is a caller global).
        let renamed_base = rename.get(base).map_or(base, String::as_str);
        if substitutes_index {
            format!("{renamed_base}{}", rewrite_array_index_tail(tail, rename))
        } else {
            format!("{renamed_base}{tail}")
        }
    } else {
        rename.get(name).cloned().unwrap_or_else(|| name.to_owned())
    }
}

/// Rewrite the `(idx)` array-index tail of an *unbraced* variable reference.
/// The index is a substituted context (`arr($idx)`), so any `$var` / `${var}`
/// inside it is α-renamed via the value-string rewriter rather than copied
/// verbatim. A braced `${arr(idx)}` name is *not* a
/// substituted context and must not be routed here.
fn rewrite_array_index_tail(tail: &str, rename: &Rename<'_>) -> String {
    if tail.is_empty() {
        return String::new();
    }
    rewrite_value_string(tail, rename)
}

/// Rename a binding-position local: a renamed name maps through, an
/// untracked name passes through unchanged.
fn rename_local(name: &str, rename: &Rename<'_>) -> String {
    rename_name(name, false, rename)
}

fn rewrite_stmt(stmt: &Statement, rename: &Rename<'_>) -> Statement {
    match stmt {
        Statement::AssignConst { .. }
        | Statement::AssignValue { .. }
        | Statement::AssignExpr { .. }
        | Statement::Incr { .. }
        | Statement::ExprEval { .. } => rewrite_assign_like(stmt, rename),
        Statement::Call { .. } | Statement::Return { .. } => rewrite_call_like(stmt, rename),
        Statement::Block { .. } | Statement::UpFrame { .. } => rewrite_block_like(stmt, rename),
        Statement::If { .. } | Statement::For { .. } | Statement::While { .. } => {
            rewrite_control_flow(stmt, rename)
        }
        Statement::Foreach { .. } | Statement::Catch { .. } | Statement::Try { .. } => {
            rewrite_binding_scope(stmt, rename)
        }
        Statement::Switch { .. } => rewrite_switch(stmt, rename),
        // Barrier never appears in a v3-eligible body (it isn't
        // splice-eligible, so `_v3_eligible` rejects the proc). Pass
        // through unchanged for completeness.
        Statement::Barrier { .. } | Statement::NativeCall { .. } => stmt.clone(),
    }
}

/// Assignment-shaped statements: rename the LHS name and rewrite the RHS
/// value / expression.
fn rewrite_assign_like(stmt: &Statement, rename: &Rename<'_>) -> Statement {
    match stmt {
        Statement::AssignConst {
            span,
            name,
            name_braced,
            value,
            value_span,
        } => Statement::AssignConst {
            span: *span,
            name: rename_name(name, !name_braced, rename),
            name_braced: *name_braced,
            value: value.clone(),
            value_span: *value_span,
        },
        Statement::AssignValue {
            span,
            name,
            name_braced,
            value,
            value_needs_backsubst,
            tokens,
        } => Statement::AssignValue {
            span: *span,
            name: rename_name(name, !name_braced, rename),
            name_braced: *name_braced,
            value: rewrite_value_string(value, rename),
            value_needs_backsubst: *value_needs_backsubst,
            tokens: tokens.clone(),
        },
        Statement::AssignExpr {
            span,
            name,
            name_braced,
            expr,
            command_binding,
            fallback_value,
            ..
        } => Statement::AssignExpr {
            span: *span,
            name: rename_name(name, !name_braced, rename),
            name_braced: *name_braced,
            expr: rewrite_expr(expr, rename),
            command_binding: command_binding.clone(),
            fallback_value: rewrite_value_string(fallback_value, rename),
            // The rewrite renames variables in-place, so leaf offsets no
            // longer index the (renamed) text — drop the source anchor.
            expr_base: None,
        },
        Statement::Incr {
            span,
            name,
            name_braced,
            amount,
            amount_braced,
            safe_on_uninit,
        } => Statement::Incr {
            span: *span,
            name: rename_name(name, !name_braced, rename),
            name_braced: *name_braced,
            // A braced amount substitutes nothing, so it names no variable
            // a rename could reach.
            amount: amount.as_ref().map(|a| {
                if *amount_braced {
                    a.clone()
                } else {
                    rewrite_value_string(a, rename)
                }
            }),
            amount_braced: *amount_braced,
            safe_on_uninit: *safe_on_uninit,
        },
        Statement::ExprEval {
            span,
            command_binding,
            expr,
            ..
        } => Statement::ExprEval {
            span: *span,
            command_binding: command_binding.clone(),
            expr: rewrite_expr(expr, rename),
            expr_base: None,
        },
        _ => unreachable!("rewrite_assign_like dispatched a non-assign statement"),
    }
}

/// `Call` / `Return`: rewrite argument value strings and (for calls)
/// `defs` / `reads` variable names.
fn rewrite_call_like(stmt: &Statement, rename: &Rename<'_>) -> Statement {
    match stmt {
        Statement::Call {
            span,
            command,
            canonical_command,
            args,
            defs,
            reads,
            reads_own_defs,
            safe_on_uninit,
            tokens,
            foreach_groups,
        } => Statement::Call {
            span: *span,
            command: command.clone(),
            canonical_command: canonical_command.clone(),
            // A braced word substitutes nothing, so a `$v` inside it is literal
            // text and survives unrewritten.
            args: args
                .iter()
                .enumerate()
                .map(|(index, arg)| {
                    let literal = tokens
                        .as_ref()
                        .is_some_and(|tokens| tokens.arg_is_braced_literal(index));
                    if literal {
                        arg.clone()
                    } else {
                        rewrite_value_string(arg, rename)
                    }
                })
                .collect(),
            defs: defs.iter().map(|d| rename_var_name(d, rename)).collect(),
            reads: reads.iter().map(|r| rename_var_name(r, rename)).collect(),
            reads_own_defs: *reads_own_defs,
            safe_on_uninit: *safe_on_uninit,
            tokens: tokens.clone(),
            foreach_groups: foreach_groups.clone(),
        },
        Statement::Return {
            span,
            value,
            expr,
            command_binding,
            braced,
            ..
        } => Statement::Return {
            tokens: None,
            span: *span,
            value: value.as_ref().map(|v| {
                if *braced {
                    v.clone()
                } else {
                    rewrite_value_string(v, rename)
                }
            }),
            value_word: None,
            expr: expr.as_ref().map(|e| rewrite_expr(e, rename)),
            expr_base: None,
            command_binding: command_binding.clone(),
            braced: *braced,
        },
        _ => unreachable!("rewrite_call_like dispatched a non-call statement"),
    }
}

/// `Block` / `UpFrame`: rewrite the nested body script.
fn rewrite_block_like(stmt: &Statement, rename: &Rename<'_>) -> Statement {
    match stmt {
        Statement::Block {
            span,
            body,
            namespace,
            tokens,
            error_context,
        } => Statement::Block {
            span: *span,
            body: rewrite_script_in(body, rename),
            namespace: namespace.clone(),
            tokens: tokens.clone(),
            error_context: *error_context,
        },
        Statement::UpFrame {
            span,
            frame_shift,
            absolute,
            body,
            tokens,
        } => Statement::UpFrame {
            span: *span,
            frame_shift: *frame_shift,
            absolute: *absolute,
            body: rewrite_script_in(body, rename),
            tokens: tokens.clone(),
        },
        _ => unreachable!("rewrite_block_like dispatched a non-block statement"),
    }
}

/// `If` / `For` / `While`: rewrite conditions, sub-scripts, and bodies.
fn rewrite_control_flow(stmt: &Statement, rename: &Rename<'_>) -> Statement {
    match stmt {
        Statement::If {
            span,
            clauses,
            else_body,
            else_span,
        } => Statement::If {
            span: *span,
            clauses: clauses
                .iter()
                .map(|c| IfClause {
                    condition: rewrite_expr(&c.condition, rename),
                    condition_span: c.condition_span,
                    condition_base: None,
                    body: rewrite_script_in(&c.body, rename),
                    body_span: c.body_span,
                })
                .collect(),
            else_body: else_body.as_ref().map(|b| rewrite_script_in(b, rename)),
            else_span: *else_span,
        },
        Statement::For {
            span,
            init,
            init_span,
            condition,
            condition_span,
            next,
            next_span,
            body,
            body_span,
            raw_args,
            raw_tokens,
            ..
        } => Statement::For {
            span: *span,
            init: rewrite_script_in(init, rename),
            init_span: *init_span,
            condition: rewrite_expr(condition, rename),
            condition_span: *condition_span,
            condition_base: None,
            next: rewrite_script_in(next, rename),
            next_span: *next_span,
            body: rewrite_script_in(body, rename),
            body_span: *body_span,
            raw_args: raw_args.clone(),
            raw_tokens: raw_tokens.clone(),
        },
        Statement::While {
            span,
            condition,
            condition_span,
            body,
            body_span,
            raw_args,
            raw_tokens,
            ..
        } => Statement::While {
            span: *span,
            condition: rewrite_expr(condition, rename),
            condition_span: *condition_span,
            condition_base: None,
            body: rewrite_script_in(body, rename),
            body_span: *body_span,
            raw_args: raw_args.clone(),
            raw_tokens: raw_tokens.clone(),
        },
        _ => unreachable!("rewrite_control_flow dispatched a non-control statement"),
    }
}

/// `Foreach` / `Catch` / `Try`: rewrite bodies plus the binding-position
/// locals each introduces (loop vars, result/options vars, handler vars).
fn rewrite_binding_scope(stmt: &Statement, rename: &Rename<'_>) -> Statement {
    match stmt {
        Statement::Foreach {
            span,
            iterators,
            body,
            body_span,
            is_lmap,
            raw_args,
            is_dict_iteration,
            is_array_iteration,
            raw_tokens,
        } => Statement::Foreach {
            span: *span,
            iterators: iterators
                .iter()
                .map(|it| crate::ir::ForeachIterator {
                    vars: it.vars.iter().map(|v| rename_local(v, rename)).collect(),
                    // A brace-quoted value word substitutes nothing, so a
                    // `$v` inside it is literal text and must survive
                    // inlining unrewritten.
                    list_arg: if it.list_braced {
                        it.list_arg.clone()
                    } else {
                        rewrite_value_string(&it.list_arg, rename)
                    },
                    list_braced: it.list_braced,
                })
                .collect(),
            body: rewrite_script_in(body, rename),
            body_span: *body_span,
            is_lmap: *is_lmap,
            raw_args: raw_args.clone(),
            is_dict_iteration: *is_dict_iteration,
            is_array_iteration: *is_array_iteration,
            raw_tokens: raw_tokens.clone(),
        },
        Statement::Catch {
            span,
            body,
            body_span,
            result_var,
            options_var,
            raw_args,
            tokens,
        } => Statement::Catch {
            span: *span,
            body: rewrite_script_in(body, rename),
            body_span: *body_span,
            result_var: result_var.as_ref().map(|v| rename_local(v, rename)),
            options_var: options_var.as_ref().map(|v| rename_local(v, rename)),
            raw_args: raw_args.clone(),
            tokens: tokens.clone(),
        },
        Statement::Try {
            span,
            body,
            body_span,
            handlers,
            finally_body,
            finally_span,
            raw_args,
        } => Statement::Try {
            span: *span,
            body: rewrite_script_in(body, rename),
            body_span: *body_span,
            handlers: handlers
                .iter()
                .map(|h| TryHandler {
                    kind: h.kind,
                    match_arg: h.match_arg.clone(),
                    trap_pattern: h.trap_pattern.clone(),
                    var_name: h.var_name.as_ref().map(|v| rename_local(v, rename)),
                    options_var: h.options_var.as_ref().map(|v| rename_local(v, rename)),
                    body: rewrite_script_in(&h.body, rename),
                    body_span: h.body_span,
                    fallthrough: h.fallthrough,
                })
                .collect(),
            finally_body: finally_body.as_ref().map(|b| rewrite_script_in(b, rename)),
            finally_span: *finally_span,
            raw_args: raw_args.clone(),
        },
        _ => unreachable!("rewrite_binding_scope dispatched an unexpected statement"),
    }
}

/// `Switch`: rewrite the subject value and each arm / default body.
fn rewrite_switch(stmt: &Statement, rename: &Rename<'_>) -> Statement {
    let Statement::Switch {
        span,
        subject,
        subject_braced,
        subject_span,
        arms,
        default_body,
        default_span,
        mode,
        nocase,
        raw_args,
        raw_arg_braced,
        raw_arg_quoted,
        command,
        patterns_braced,
    } = stmt
    else {
        unreachable!("rewrite_switch dispatched a non-switch statement");
    };
    Statement::Switch {
        subject_braced: *subject_braced,
        span: *span,
        // A braced subject's value is literal — a `$x` inside it is data, not
        // a read — so alpha-renaming must leave it alone, exactly as the arm
        // patterns below are cloned rather than rewritten. The rewrite was
        // harmless while the subject was substituted at run time regardless of
        // its braces: renaming it and then reading the renamed variable
        // happened to give an answer. Now that a braced subject is the literal
        // it always was, rewriting it compares `$__inline_…__x` against the
        // pattern `$x` and takes the wrong arm.
        subject: if *subject_braced {
            subject.clone()
        } else {
            rewrite_value_string(subject, rename)
        },
        subject_span: *subject_span,
        arms: arms
            .iter()
            .map(|a| SwitchArm {
                pattern: a.pattern.clone(),
                pattern_braced: a.pattern_braced,
                pattern_span: a.pattern_span,
                body: a.body.as_ref().map(|b| rewrite_script_in(b, rename)),
                body_span: a.body_span,
                fallthrough: a.fallthrough,
            })
            .collect(),
        default_body: default_body.as_ref().map(|b| rewrite_script_in(b, rename)),
        default_span: *default_span,
        mode: *mode,
        nocase: *nocase,
        raw_args: raw_args.clone(),
        raw_arg_braced: raw_arg_braced.clone(),
        raw_arg_quoted: raw_arg_quoted.clone(),
        command: command.clone(),
        patterns_braced: *patterns_braced,
    }
}

/// Rewrite `$name` / `${name}` substitutions in `text`. Array-element
/// references retain their original index extent and recursively rename only
/// its enabled substitutions. Braced combined names keep literal index bytes.
/// The backslash-protection rule also applies: `\$x` is a literal `$`, not
/// a substitution, so its name is never renamed.
fn rewrite_value_string(text: &str, rename: &Rename<'_>) -> String {
    use tcl_lexer::word_parts::{SubstFlags, WordPart, decompose_spanned_checked, scan_var_ref};

    if text.is_empty() || rename.is_empty() {
        return text.to_owned();
    }
    let source = text.as_bytes();
    let Ok(parts) = decompose_spanned_checked(source, SubstFlags::default(), rename.config) else {
        return text.to_owned();
    };
    let mut output = String::with_capacity(text.len());
    for part in parts {
        let original = &text[part.start..part.end];
        match part.part {
            WordPart::Variable(_) => {
                let Ok(Some(reference)) = scan_var_ref(original.as_bytes(), 0, rename.config)
                else {
                    output.push_str(original);
                    continue;
                };
                let Some(root) = tcl_syntax::naming::variable_reference_root_bytes(
                    original.as_bytes(),
                    rename.config,
                )
                .ok()
                .flatten() else {
                    output.push_str(original);
                    continue;
                };
                let Ok(root_text) = std::str::from_utf8(root) else {
                    output.push_str(original);
                    continue;
                };
                // The shared scanner borrows its literal name from this exact
                // component. The root's bytes retain their original edit extent.
                let Some(name_range) = reference.name_range_in(original.as_bytes()) else {
                    output.push_str(original);
                    continue;
                };
                let name_at = name_range.start;
                output.push_str(&original[..name_at]);
                output.push_str(rename.get(root_text).map_or(root_text, String::as_str));
                let tail_at = name_at + root.len();
                if let Some(index) = reference.index_range_in(original.as_bytes()) {
                    output.push_str(&original[tail_at..index.start]);
                    output.push_str(&rewrite_value_string(&original[index.clone()], rename));
                    output.push_str(&original[index.end..]);
                } else {
                    // A braced combined name is literal: its element key is
                    // never recursively substituted by this rewrite.
                    output.push_str(&original[tail_at..]);
                }
            }
            WordPart::Command(_) | WordPart::Expression(_) => {
                // The frame/source eligibility owner has independently proved
                // every traversed command and every literal expression operand.
                output.push_str(&original[..1]);
                output.push_str(&rewrite_value_string(
                    &original[1..original.len() - 1],
                    rename,
                ));
                output.push_str(&original[original.len() - 1..]);
            }
            WordPart::Text(_) | WordPart::ParseError(_) => output.push_str(original),
        }
    }
    output
}

/// Walk an [`ExprNode`] tree and return a clone with `rename` applied to
/// every [`ExprNode::Var`] name.
fn rewrite_expr(node: &ExprNode, rename: &Rename<'_>) -> ExprNode {
    // Public entry: the top of an expression tree is nesting depth 0; the
    // recursion cap lives in [`rewrite_expr_at`].
    rewrite_expr_at(node, rename, 0)
}

fn rewrite_expr_at(node: &ExprNode, rename: &Rename<'_>, depth: u32) -> ExprNode {
    // Native-stack safety net: this both walks the input tree
    // and constructs a renamed clone, one native frame per level. Past the
    // cap, pass the node through *unchanged* (a full `clone`) — this stops
    // transforming deeper vars but preserves the tree's structure intact
    // (never truncates or panics). Only reachable past 256 levels of
    // expression nesting, which the Pratt parser never produces.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return node.clone();
    }
    match node {
        ExprNode::Var {
            text,
            name,
            start,
            end,
        } => ExprNode::Var {
            text: rewrite_value_string(text, rename),
            name: rename.get(name).cloned().unwrap_or_else(|| name.clone()),
            start: *start,
            end: *end,
        },
        ExprNode::Binary { op, left, right } => ExprNode::Binary {
            op: *op,
            left: Box::new(rewrite_expr_at(left, rename, depth + 1)),
            right: Box::new(rewrite_expr_at(right, rename, depth + 1)),
        },
        ExprNode::Unary { op, operand } => ExprNode::Unary {
            op: *op,
            operand: Box::new(rewrite_expr_at(operand, rename, depth + 1)),
        },
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => ExprNode::Ternary {
            condition: Box::new(rewrite_expr_at(condition, rename, depth + 1)),
            true_branch: Box::new(rewrite_expr_at(true_branch, rename, depth + 1)),
            false_branch: Box::new(rewrite_expr_at(false_branch, rename, depth + 1)),
        },
        // A compiled word carries a *value*; a `$x` inside it is either data
        // (braced) or resolved by word substitution, never an expression
        // variable this rename owns.
        ExprNode::CompiledWord { text, braced } => ExprNode::CompiledWord {
            text: text.clone(),
            braced: *braced,
        },
        ExprNode::Call {
            function,
            args,
            start,
            end,
        } => ExprNode::Call {
            function: function.clone(),
            args: args
                .iter()
                .map(|a| rewrite_expr_at(a, rename, depth + 1))
                .collect(),
            start: *start,
            end: *end,
        },
        // Literal / String / Command / Raw carry no variable references
        // the rename map touches (Command is opaque cmd-subst, gated at
        // call-site eligibility).
        ExprNode::Literal { .. }
        | ExprNode::String { .. }
        | ExprNode::Command { .. }
        | ExprNode::Raw { .. } => node.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewrite_value_string(text: &str, names: &HashMap<String, String>) -> String {
        super::rewrite_value_string(
            text,
            &Rename {
                names,
                config: tcl_lexer::LexerConfig::default(),
            },
        )
    }

    fn rewrite_expr(node: &ExprNode, names: &HashMap<String, String>) -> ExprNode {
        super::rewrite_expr(
            node,
            &Rename {
                names,
                config: tcl_lexer::LexerConfig::default(),
            },
        )
    }

    fn rename_var_name(text: &str, names: &HashMap<String, String>) -> String {
        super::rename_var_name(
            text,
            &Rename {
                names,
                config: tcl_lexer::LexerConfig::default(),
            },
        )
    }

    fn rn(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    /// `rewrite_expr` recurses once per `ExprNode` level — walking the input
    /// *and* constructing a renamed clone — so it needs a depth cap. A tree
    /// built directly is unbounded (the Pratt parser caps its own output at
    /// 256) and uncapped overflows the native stack (SIGABRT) in the low
    /// thousands of levels on a 2 MiB thread. 3000 is past that crash range and past
    /// `MAX_EXPR_NODE_DEPTH` (256); the assertion is that it returns a tree
    /// (past the cap it passes the sub-tree through unchanged rather than
    /// truncating or panicking).
    #[test]
    fn deeply_nested_rewrite_expr_survives() {
        use crate::expr_ast::UnaryOp;
        let mut node = ExprNode::Var {
            text: "$x".into(),
            name: "x".into(),
            start: 0,
            end: 2,
        };
        for _ in 0..3000 {
            node = ExprNode::Unary {
                op: UnaryOp::Not,
                operand: Box::new(node),
            };
        }
        let r = rn(&[("x", "y")]);
        // Returns a full tree (never truncated) without overflowing.
        let out = rewrite_expr(&node, &r);
        assert!(matches!(out, ExprNode::Unary { .. }));
    }

    #[test]
    fn value_string_renames_dollar_name() {
        let r = rn(&[("x", "__inline_1__x")]);
        assert_eq!(rewrite_value_string("$x + 1", &r), "$__inline_1__x + 1");
    }

    #[test]
    fn value_string_renames_brace_form() {
        let r = rn(&[("x", "y")]);
        assert_eq!(rewrite_value_string("a${x}b", &r), "a${y}b");
    }

    #[test]
    fn value_string_respects_backslash_escape() {
        let r = rn(&[("x", "y")]);
        // `\$x` is a literal `$` — the `x` is not a substitution.
        assert_eq!(rewrite_value_string("\\$x", &r), "\\$x");
    }

    #[test]
    fn value_string_double_backslash_is_substitution() {
        let r = rn(&[("x", "y")]);
        // `\\$x` — the first `\` escapes the second, so `$x` IS a subst.
        assert_eq!(rewrite_value_string("\\\\$x", &r), "\\\\$y");
    }

    #[test]
    fn value_string_array_renames_base_only() {
        let r = rn(&[("arr", "z")]);
        assert_eq!(rewrite_value_string("$arr(idx)", &r), "$z(idx)");
    }

    #[test]
    fn value_string_unmapped_passes_through() {
        let r = rn(&[("x", "y")]);
        assert_eq!(rewrite_value_string("$other", &r), "$other");
    }

    #[test]
    fn value_string_renames_var_inside_array_index() {
        // `$arr($idx)` — both the base and the `$idx` inside the
        // index are α-renamed, so an inlined body reads the renamed inline
        // local, not the caller's `idx`.
        let r = rn(&[("arr", "__inline_arr"), ("idx", "__inline_idx")]);
        assert_eq!(
            rewrite_value_string("$arr($idx)", &r),
            "$__inline_arr($__inline_idx)"
        );
        // The index var is renamed even when the base is an untracked caller
        // global.
        let r2 = rn(&[("idx", "__inline_idx")]);
        assert_eq!(rewrite_value_string("$g($idx)", &r2), "$g($__inline_idx)");
    }

    #[test]
    fn assign_target_renames_var_inside_array_index() {
        // `set arr($idx) …` — the assignment target's index var must rename too.
        let r = rn(&[("arr", "__inline_arr"), ("idx", "__inline_idx")]);
        assert_eq!(
            rename_var_name("arr($idx)", &r),
            "__inline_arr($__inline_idx)"
        );
    }

    #[test]
    fn braced_array_name_index_is_literal() {
        // `${arr(idx)}` is a literal variable name — no substitution inside the
        // braces — so the index is not treated as a $var context.
        let r = rn(&[("arr", "z"), ("idx", "renamed")]);
        assert_eq!(rewrite_value_string("${arr(idx)}", &r), "${z(idx)}");
    }
    #[test]
    fn original_rename_projection_keeps_selected_reference_grammar_and_literal_names() {
        // naming.inlining.original-frame-source-context
        // docs/design/analysis/name-resolution-proofs/inlining-original-frame-source-context.md
        // Source edit projection only; native341 answers public expression results,
        // not alpha-renaming, compiled frames or runtime execution.
        let c8 = tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some(
            "tcl8.6",
        )));
        let c9 = tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some(
            "tcl9.0",
        )));
        let jim =
            tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some("jim")));
        let names = rn(&[
            ("café", "unicode"),
            ("caf", "ascii"),
            ("$literal", "sigil"),
            ("a{b", "first"),
            ("a{b}c", "nested"),
            ("scalar(open", "open"),
            ("scalar(closed)tail", "tail"),
            ("arr", "array"),
            ("idx", "index"),
        ]);
        let c8_rename = Rename {
            names: &names,
            config: c8,
        };
        let c9_rename = Rename {
            names: &names,
            config: c9,
        };
        let jim_rename = Rename {
            names: &names,
            config: jim,
        };
        assert_eq!(super::rewrite_value_string("$café", &c8_rename), "$unicode");
        assert_eq!(super::rewrite_value_string("$café", &jim_rename), "$asciié");
        assert_eq!(
            super::rewrite_value_string("${a{b}c}", &c8_rename),
            "${first}c}"
        );
        assert_eq!(
            super::rewrite_value_string("${a{b}c}", &c9_rename),
            "${nested}"
        );
        for rename in [&c8_rename, &c9_rename, &jim_rename] {
            assert_eq!(
                super::rewrite_value_string("${$literal}", rename),
                "${sigil}"
            );
            assert_eq!(
                super::rewrite_value_string("${scalar(open}", rename),
                "${open}"
            );
            assert_eq!(
                super::rewrite_value_string("${scalar(closed)tail}", rename),
                "${tail}"
            );
            assert_eq!(
                super::rewrite_value_string("$arr($idx)", rename),
                "$array($index)"
            );
            assert_eq!(
                super::rewrite_value_string("${arr($idx)}", rename),
                "${array($idx)}"
            );
            assert_eq!(
                super::rewrite_value_string(r"\${$literal}", rename),
                r"\${$literal}"
            );
            assert_eq!(super::rename_var_name("scalar(open", rename), "open");
            assert_eq!(super::rename_var_name("scalar(closed)tail", rename), "tail");
        }
    }

    #[test]
    fn original_rename_projection_keeps_literal_binding_indices_and_unrenamed_roots() {
        // naming.inlining.original-frame-source-context
        // docs/design/analysis/name-resolution-proofs/inlining-original-frame-source-context.md
        let names = rn(&[("arr", "array"), ("idx", "index")]);
        let rename = Rename {
            names: &names,
            config: tcl_lexer::LexerConfig::default(),
        };
        assert_eq!(rename_name("arr($idx)", false, &rename), "array($idx)");
        assert_eq!(rename_name("arr($idx)", true, &rename), "array($index)");
        assert_eq!(rename_local("arr($idx)", &rename), "array($idx)");
        let original = ExprNode::Var {
            text: "$global($idx)".into(),
            name: "global".into(),
            start: 0,
            end: 13,
        };
        assert!(matches!(super::rewrite_expr(&original, &rename),
            ExprNode::Var { text, name, .. } if text == "$global($index)" && name == "global"));
        let malformed = "${unterminated";
        assert_eq!(super::rewrite_value_string(malformed, &rename), malformed);
    }
}
