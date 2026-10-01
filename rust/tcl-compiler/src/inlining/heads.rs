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

//! The commands a spliced body names, as its own namespace names them.
//!
//! A procedure's body resolves an unqualified command from the procedure's
//! namespace, so the `string` of a pack's `proc vlen` is the global one.
//! Spliced into a caller in another namespace the same spelling resolves from
//! that namespace first, and a caller that defines a `string` of its own would
//! answer in its place. [`root`] spells each such word from the global
//! namespace, where the definition's own lookup ends, so the splice means what
//! the call it replaced meant wherever it stands.
//!
//! Only the command words the IR keeps as calls can be spelled this way. A
//! command a word substitutes — `[join $l ,]`, or one inside an expression — is
//! text to the IR, so a body with one stays a call where it would need the
//! spelling. A statement lowering consumed — `set`, `incr`, `expr`, an `if` —
//! carries a binding recorded in the definition's own namespace, which the VM
//! holds to the live command, and so is already the definition's.

use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::ExprNode;
use crate::ir::{Script, Statement};

/// `statements` with every command word not already absolute spelled from the
/// global namespace, to the depth the inliner reads a body to; `None` when the
/// body runs a command by a name the IR keeps as text.
pub(super) fn root(mut statements: Vec<Statement>) -> Option<Vec<Statement>> {
    if statements
        .iter()
        .any(|statement| runs_a_substituted_command(statement, 0))
    {
        return None;
    }
    for statement in &mut statements {
        rooted(statement, 0);
    }
    Some(statements)
}

fn substitutes(text: &str) -> bool {
    super::arg_has_command_subst(text)
}

/// Whether an operand of `expr` is a command substitution, or text that may
/// hold one.
fn expression_substitutes(expr: &ExprNode, depth: u32) -> bool {
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    let next = depth + 1;
    match expr {
        ExprNode::Command { .. } => true,
        ExprNode::Raw { text } => text.contains('['),
        ExprNode::String { text, .. } => text.starts_with('"') && text.contains('['),
        ExprNode::Binary { left, right, .. } => {
            expression_substitutes(left, next) || expression_substitutes(right, next)
        }
        ExprNode::Unary { operand, .. } => expression_substitutes(operand, next),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expression_substitutes(condition, next)
                || expression_substitutes(true_branch, next)
                || expression_substitutes(false_branch, next)
        }
        ExprNode::Call { args, .. } => args.iter().any(|arg| expression_substitutes(arg, next)),
        ExprNode::Literal { .. } | ExprNode::Var { .. } | ExprNode::CompiledWord { .. } => false,
    }
}

fn script_substitutes(script: &Script, depth: u32) -> bool {
    super::MAX_INLINING_WALK_DEPTH.exceeded(depth)
        || script
            .statements
            .iter()
            .any(|statement| runs_a_substituted_command(statement, depth))
}

/// Whether a word of `statement`, or of a body inside it, substitutes a command.
/// A braced word is literal, and a nested expression the statement consumed
/// natively (`return [expr {…}]`) is read from its tree and not its text.
fn runs_a_substituted_command(statement: &Statement, depth: u32) -> bool {
    let next = depth + 1;
    match statement {
        Statement::AssignConst { .. } => false,
        Statement::AssignValue { value, .. } => substitutes(value),
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            expression_substitutes(expr, 0)
        }
        Statement::Incr {
            amount,
            amount_braced,
            ..
        } => !amount_braced && amount.as_deref().is_some_and(substitutes),
        Statement::Call { args, tokens, .. } => args.iter().enumerate().any(|(index, arg)| {
            !tokens
                .as_ref()
                .is_some_and(|tokens| tokens.arg_is_braced_literal(index))
                && substitutes(arg)
        }),
        Statement::Return {
            value,
            expr,
            braced,
            ..
        } => match expr {
            Some(expr) => expression_substitutes(expr, 0),
            None => !braced && value.as_deref().is_some_and(substitutes),
        },
        Statement::Block { body, .. } | Statement::Catch { body, .. } => {
            script_substitutes(body, next)
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses.iter().any(|clause| {
                expression_substitutes(&clause.condition, 0)
                    || script_substitutes(&clause.body, next)
            }) || else_body
                .as_ref()
                .is_some_and(|body| script_substitutes(body, next))
        }
        Statement::For {
            init,
            condition,
            next: step,
            body,
            ..
        } => {
            expression_substitutes(condition, 0)
                || script_substitutes(init, next)
                || script_substitutes(step, next)
                || script_substitutes(body, next)
        }
        Statement::While {
            condition, body, ..
        } => expression_substitutes(condition, 0) || script_substitutes(body, next),
        Statement::Foreach {
            iterators, body, ..
        } => {
            iterators
                .iter()
                .any(|iterator| !iterator.list_braced && substitutes(&iterator.list_arg))
                || script_substitutes(body, next)
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            script_substitutes(body, next)
                || handlers
                    .iter()
                    .any(|handler| script_substitutes(&handler.body, next))
                || finally_body
                    .as_ref()
                    .is_some_and(|body| script_substitutes(body, next))
        }
        Statement::Switch {
            subject,
            subject_braced,
            arms,
            default_body,
            ..
        } => {
            (!subject_braced && substitutes(subject))
                || arms.iter().any(|arm| {
                    (!arm.pattern_braced && substitutes(&arm.pattern))
                        || arm
                            .body
                            .as_ref()
                            .is_some_and(|body| script_substitutes(body, next))
                })
                || default_body
                    .as_ref()
                    .is_some_and(|body| script_substitutes(body, next))
        }
        // Neither is read by the inliner's eligibility; a body with one is not spelled.
        Statement::UpFrame { .. } | Statement::Barrier { .. } => true,
    }
}

fn rooted_script(script: &mut Script, depth: u32) {
    if super::MAX_INLINING_WALK_DEPTH.exceeded(depth) {
        return;
    }
    for statement in &mut script.statements {
        rooted(statement, depth);
    }
}

fn rooted(statement: &mut Statement, depth: u32) {
    let next = depth + 1;
    match statement {
        Statement::Call { command, .. } => {
            if !command.starts_with("::") {
                command.insert_str(0, "::");
            }
        }
        Statement::Block { body, .. }
        | Statement::UpFrame { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. } => rooted_script(body, next),
        Statement::If {
            clauses, else_body, ..
        } => {
            for clause in clauses {
                rooted_script(&mut clause.body, next);
            }
            if let Some(body) = else_body {
                rooted_script(body, next);
            }
        }
        Statement::For {
            init,
            next: step,
            body,
            ..
        } => {
            rooted_script(init, next);
            rooted_script(step, next);
            rooted_script(body, next);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            rooted_script(body, next);
            for handler in handlers {
                rooted_script(&mut handler.body, next);
            }
            if let Some(body) = finally_body {
                rooted_script(body, next);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for arm in arms {
                if let Some(body) = &mut arm.body {
                    rooted_script(body, next);
                }
            }
            if let Some(body) = default_body {
                rooted_script(body, next);
            }
        }
        _ => {}
    }
}
