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
//! namespace, so the `string` of a pack's `proc vlen`, defined in the global
//! namespace, is the global one. Spliced into a caller in another namespace the
//! same spelling resolves from that namespace first, and a caller that defines a
//! `string` of its own would answer in its place. [`root`] spells each such word
//! from the global namespace, where the definition's own lookup ends, so the
//! splice means what the call it replaced meant wherever it stands.
//!
//! A definition in a namespace of its own looks a name up there first and in the
//! global namespace after it, and no spelling says that: `::string` skips a
//! `vendor::string` the package defines when it runs, and `string` is the
//! caller's own lookup. The command it would have to be held to is not the
//! compile's to see, and the binding the VM holds a lowered statement to names a
//! builtin and never a procedure. So such a body is spliced only where the caller
//! looks names up as the definition did, or when it names no command by a word
//! the IR keeps ([`names_a_command`]).
//!
//! Only the command words the IR keeps as calls can be spelled. A command a word
//! substitutes — `[join $l ,]`, or one inside an expression — is text to the IR,
//! so a body with one stays a call where it would need the spelling. A statement
//! lowering consumed — `set`, `incr`, `expr`, an `if` — carries a binding
//! recorded in the definition's own namespace, which the VM holds to the live
//! command, and so is already the definition's.

use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Script, Statement};
use tcl_lexer::LexerConfig;

/// `statements` with every command word not already absolute spelled from the
/// global namespace, to the depth the inliner reads a body to; `None` when the
/// body runs a command by a name the IR keeps as text.
pub(super) fn root(mut statements: Vec<Statement>, config: LexerConfig) -> Option<Vec<Statement>> {
    if statements
        .iter()
        .any(|statement| runs_a_command_by_name(statement, false, config, 0))
    {
        return None;
    }
    for statement in &mut statements {
        rooted(statement, 0);
    }
    Some(statements)
}

/// Whether `statements`, the body of a definition, name a command by a word
/// the definition's own namespace resolves: a call head that is not absolute,
/// or a command a word or an expression substitutes.
pub(super) fn names_a_command(statements: &[Statement], config: LexerConfig) -> bool {
    statements
        .iter()
        .any(|statement| runs_a_command_by_name(statement, true, config, 0))
}

fn substitutes(text: &str, config: LexerConfig) -> bool {
    use tcl_lexer::word_parts::{SubstFlags, WordPart, decompose_spanned_checked};
    let Ok(parts) = decompose_spanned_checked(text.as_bytes(), SubstFlags::default(), config)
    else {
        return true;
    };
    let mut pending = parts.into_iter().map(|part| part.part).collect::<Vec<_>>();
    while let Some(part) = pending.pop() {
        match part {
            WordPart::Command(_) | WordPart::Expression(_) | WordPart::ParseError(_) => {
                return true;
            }
            WordPart::Variable(variable) => pending.extend(variable.index.into_iter().flatten()),
            WordPart::Text(_) => {}
        }
    }
    false
}

/// Whether an operand of `expr` is a command substitution, or text that may
/// hold one.
fn expression_substitutes(expr: &ExprNode, config: LexerConfig, depth: u32) -> bool {
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    let next = depth + 1;
    match expr {
        ExprNode::Command { .. } => true,
        ExprNode::Raw { text } => substitutes(text, config),
        ExprNode::String { text, .. } => text.starts_with('"') && substitutes(text, config),
        ExprNode::Binary { left, right, .. } => {
            expression_substitutes(left, config, next)
                || expression_substitutes(right, config, next)
        }
        ExprNode::Unary { operand, .. } => expression_substitutes(operand, config, next),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            expression_substitutes(condition, config, next)
                || expression_substitutes(true_branch, config, next)
                || expression_substitutes(false_branch, config, next)
        }
        ExprNode::Call { args, .. } => args
            .iter()
            .any(|arg| expression_substitutes(arg, config, next)),
        ExprNode::Var { text, .. } => substitutes(text, config),
        ExprNode::Literal { .. } | ExprNode::CompiledWord { .. } => false,
    }
}

/// Whether a word of a call that is not braced substitutes a command.
fn call_substitutes(args: &[String], tokens: Option<&CommandTokens>, config: LexerConfig) -> bool {
    args.iter().enumerate().any(|(index, arg)| {
        !tokens.is_some_and(|tokens| tokens.arg_is_braced_literal(index))
            && substitutes(arg, config)
    })
}

fn script_substitutes(script: &Script, heads: bool, config: LexerConfig, depth: u32) -> bool {
    super::MAX_INLINING_WALK_DEPTH.exceeded(depth)
        || script
            .statements
            .iter()
            .any(|statement| runs_a_command_by_name(statement, heads, config, depth))
}

/// Whether a word of `statement`, or of a body inside it, substitutes a command,
/// or, with `heads`, a call has a head that is not absolute. A braced word is
/// literal, and a nested expression the statement consumed natively
/// (`return [expr {…}]`) is read from its tree and not its text.
fn runs_a_command_by_name(
    statement: &Statement,
    heads: bool,
    config: LexerConfig,
    depth: u32,
) -> bool {
    let next = depth + 1;
    match statement {
        Statement::AssignConst { .. } => false,
        Statement::AssignValue { value, .. } => substitutes(value, config),
        Statement::AssignExpr { expr, .. } | Statement::ExprEval { expr, .. } => {
            expression_substitutes(expr, config, 0)
        }
        Statement::Incr {
            amount,
            amount_braced,
            ..
        } => {
            !amount_braced
                && amount
                    .as_deref()
                    .is_some_and(|amount| substitutes(amount, config))
        }
        Statement::Call {
            command,
            args,
            tokens,
            ..
        } => {
            (heads && !command.starts_with("::")) || call_substitutes(args, tokens.as_ref(), config)
        }
        Statement::Return {
            value,
            expr,
            braced,
            ..
        } => match expr {
            Some(expr) => expression_substitutes(expr, config, 0),
            None => {
                !braced
                    && value
                        .as_deref()
                        .is_some_and(|value| substitutes(value, config))
            }
        },
        Statement::Block { body, .. } | Statement::Catch { body, .. } => {
            script_substitutes(body, heads, config, next)
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            clauses.iter().any(|clause| {
                expression_substitutes(&clause.condition, config, 0)
                    || script_substitutes(&clause.body, heads, config, next)
            }) || else_body
                .as_ref()
                .is_some_and(|body| script_substitutes(body, heads, config, next))
        }
        Statement::For {
            init,
            condition,
            next: step,
            body,
            ..
        } => {
            expression_substitutes(condition, config, 0)
                || script_substitutes(init, heads, config, next)
                || script_substitutes(step, heads, config, next)
                || script_substitutes(body, heads, config, next)
        }
        Statement::While {
            condition, body, ..
        } => {
            expression_substitutes(condition, config, 0)
                || script_substitutes(body, heads, config, next)
        }
        Statement::Foreach {
            iterators, body, ..
        } => {
            iterators
                .iter()
                .any(|iterator| !iterator.list_braced && substitutes(&iterator.list_arg, config))
                || script_substitutes(body, heads, config, next)
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            script_substitutes(body, heads, config, next)
                || handlers
                    .iter()
                    .any(|handler| script_substitutes(&handler.body, heads, config, next))
                || finally_body
                    .as_ref()
                    .is_some_and(|body| script_substitutes(body, heads, config, next))
        }
        Statement::Switch {
            subject,
            subject_braced,
            arms,
            default_body,
            ..
        } => {
            (!subject_braced && substitutes(subject, config))
                || arms.iter().any(|arm| {
                    (!arm.pattern_braced && substitutes(&arm.pattern, config))
                        || arm
                            .body
                            .as_ref()
                            .is_some_and(|body| script_substitutes(body, heads, config, next))
                })
                || default_body
                    .as_ref()
                    .is_some_and(|body| script_substitutes(body, heads, config, next))
        }
        // Neither is read by the inliner's eligibility; a body with one is not spelled.
        Statement::UpFrame { .. } | Statement::Barrier { .. } | Statement::NativeCall { .. } => {
            true
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_head_namespace_scan_keeps_selected_word_and_index_grammar() {
        // naming.inlining.original-frame-source-context
        // docs/design/analysis/name-resolution-proofs/inlining-original-frame-source-context.md
        // Source head-rewrite boundary, independent of native dispatch/frame.
        let old = LexerConfig::for_dialect("tcl8.6");
        let new = LexerConfig::for_dialect("tcl9.0");
        assert!(!substitutes(r"literal\[data", old));
        assert!(!substitutes("${array([literal])}", old));
        assert!(substitutes("$array([selected])", old));
        assert!(substitutes("$array($other([selected]))", new));
        assert!(!substitutes("${café}", old));
        assert!(!substitutes("${a{b}", old));
        assert!(substitutes("${a{b}", new));
        let mut jim = old;
        jim.var_syntax = tcl_dialect::VarSyntax::Jim;
        assert!(substitutes("$(1 + 2)", jim));
        assert!(!substitutes("$(1 + 2)", old));
    }
}
