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

//! What a spliced body reads: the names its own frame binds, and nothing else.
//!
//! A procedure's body runs in a frame of its own, so a name it reads is a
//! parameter or a local it set before the read, and any other name is unset.
//! Spliced into a caller the body shares the caller's frame. [`super::rename`]
//! gives the names the body binds a slot of its own and leaves every other name as
//! written, so a read of a name the body never binds answers the caller's variable
//! of that spelling, and a name only some paths bind raises, on the others, an error
//! that names the slot and not the variable. [`reads_only_bound_names`] is the gate:
//! a body that may read a name before it is bound stays a call.
//!
//! The walk is over the structure the IR keeps. A name is bound at a point when
//! every path to the point has bound it; the paths of an `if` or a `switch` join
//! by intersection, a path that ends in `return` joins nothing, and a loop body, a
//! `catch` body and a `try` handler bind nothing the code after them may count on.
//! What the walk cannot follow — an `uplevel` body, which runs in another frame —
//! is declined, as is a read it cannot place.
//!
//! The rename reaches a variable the expression tree names and a word's
//! substitutions. It does not reach one inside an operand the tree keeps as text
//! — a command substitution, a quoted string, an unparsed remainder — so a body
//! with a `$name` in one of those is declined too: its read would be the caller's.
//! Nor can it tell, in a braced word of a command a word substitutes, a `$name`
//! the command evaluates (`[expr {$x}]`) from text it hands on
//! (`[string length {$x}]`), and a body with the second would be spliced with the
//! first's rewriting.
//!
//! It reaches no variable a command is handed by name either. `[set y]`,
//! `[incr y]` and `[info exists y]` carry no `$` to rewrite, and spliced they
//! would address the caller's `y` and not the slot the body's own `y` became, so a
//! body that substitutes a command which is not known to work on its values alone
//! — one that takes a variable's name, runs a script, or is a procedure, which can
//! reach the frame it was called from — is declined as well.

use std::collections::HashSet;

use tcl_lexer::{Lexer, LexerConfig, SourceMap, TokenType};
use tcl_registry::{CommandRegistry, Traits};

use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{ExprNode, render_expr};
use crate::ir::{Procedure, Script, Statement};
use crate::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
use crate::var_refs::{variable_name_role_words, vars_in_word};

/// Whether every variable `proc`'s body reads is bound when it is read.
pub(super) fn reads_only_bound_names(proc: &Procedure, registry: &CommandRegistry) -> bool {
    let frame = Frame { registry };
    let mut bound: HashSet<String> = proc
        .params
        .iter()
        .map(|name| base(name).to_owned())
        .collect();
    frame.script(&proc.body, &mut bound, 0).is_some()
}

fn falls(exit: Exit) -> bool {
    exit == Exit::Falls
}

/// How control leaves a script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exit {
    /// Off the end, to the command after it.
    Falls,
    /// Out of the procedure.
    Returns,
}

struct Frame<'a> {
    registry: &'a CommandRegistry,
}

/// The variable a name addresses: an array element is its array.
fn base(name: &str) -> &str {
    name.split_once('(').map_or(name, |(array, _)| array)
}

/// A name that is not a local of the frame — one a namespace qualifies — is not
/// the frame's to bind or to read.
fn is_local(name: &str) -> bool {
    !name.is_empty() && !name.contains("::")
}

fn bind(bound: &mut HashSet<String>, name: &str) {
    if is_local(base(name)) {
        bound.insert(base(name).to_owned());
    }
}

/// Where the paths of a branching command meet: a path that returned is no
/// path, and a name is bound after the command when every path that reaches it
/// has bound it.
fn join(paths: Vec<HashSet<String>>, bound: &mut HashSet<String>) -> Exit {
    let mut paths = paths.into_iter();
    let Some(mut meet) = paths.next() else {
        return Exit::Returns;
    };
    for path in paths {
        meet.retain(|name| path.contains(name));
    }
    *bound = meet;
    Exit::Falls
}

impl Frame<'_> {
    /// Walk `script` with `bound` the names bound on entry — on exit, when control
    /// falls off its end. `None` when it reads a name that may not be bound, or
    /// when the walk cannot follow it.
    fn script(&self, script: &Script, bound: &mut HashSet<String>, depth: u32) -> Option<Exit> {
        if super::MAX_INLINING_WALK_DEPTH.exceeded(depth) {
            return None;
        }
        for statement in &script.statements {
            if self.statement(statement, bound, depth)? == Exit::Returns {
                return Some(Exit::Returns);
            }
        }
        Some(Exit::Falls)
    }

    /// `script` run on a copy of `bound`: a body whose bindings the code after it
    /// may not rely on.
    fn detached(&self, script: &Script, bound: &HashSet<String>, depth: u32) -> Option<Exit> {
        self.script(script, &mut bound.clone(), depth + 1)
    }

    /// `script` run on a copy of `bound`, which joins `paths` when control falls
    /// off its end: one path of a branching command.
    fn path(
        &self,
        script: &Script,
        bound: &HashSet<String>,
        depth: u32,
        paths: &mut Vec<HashSet<String>>,
    ) -> Option<()> {
        let mut path = bound.clone();
        if falls(self.script(script, &mut path, depth + 1)?) {
            paths.push(path);
        }
        Some(())
    }

    fn statement(
        &self,
        statement: &Statement,
        bound: &mut HashSet<String>,
        depth: u32,
    ) -> Option<Exit> {
        match statement {
            Statement::Return {
                value,
                expr,
                braced,
                ..
            } => {
                if let Some(value) = value.as_ref().filter(|_| !braced) {
                    self.word(value, bound)?;
                }
                if let Some(expr) = expr {
                    self.expression(expr, bound)?;
                }
                Some(Exit::Returns)
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                let mut paths = Vec::new();
                for clause in clauses {
                    self.expression(&clause.condition, bound)?;
                    self.path(&clause.body, bound, depth, &mut paths)?;
                }
                match else_body {
                    Some(body) => self.path(body, bound, depth, &mut paths)?,
                    None => paths.push(bound.clone()),
                }
                Some(join(paths, bound))
            }
            Statement::Switch {
                subject,
                subject_braced,
                arms,
                default_body,
                ..
            } => {
                if !subject_braced {
                    self.word(subject, bound)?;
                }
                let mut paths = Vec::new();
                for body in arms.iter().filter_map(|arm| arm.body.as_ref()) {
                    self.path(body, bound, depth, &mut paths)?;
                }
                match default_body {
                    Some(body) => self.path(body, bound, depth, &mut paths)?,
                    None => paths.push(bound.clone()),
                }
                Some(join(paths, bound))
            }
            Statement::Block { body, .. } => self.script(body, bound, depth + 1),
            Statement::For { .. } | Statement::While { .. } | Statement::Foreach { .. } => {
                self.looping(statement, bound, depth)
            }
            Statement::Catch { .. } | Statement::Try { .. } => {
                self.guarded(statement, bound, depth)
            }
            // An `uplevel` body runs in another frame, and a barrier is a command
            // the IR did not read: neither says what it binds.
            Statement::UpFrame { .. } | Statement::Barrier { .. } => None,
            Statement::AssignConst { .. }
            | Statement::AssignValue { .. }
            | Statement::AssignExpr { .. }
            | Statement::Incr { .. }
            | Statement::ExprEval { .. }
            | Statement::Call { .. } => self.command(statement, bound).map(|()| Exit::Falls),
        }
    }

    /// A statement that reads some words and binds some names, and whose control
    /// falls through.
    fn command(&self, statement: &Statement, bound: &mut HashSet<String>) -> Option<()> {
        match statement {
            Statement::AssignConst { name, .. } => bind(bound, name),
            Statement::AssignValue { name, value, .. } => {
                self.word(value, bound)?;
                bind(bound, name);
            }
            Statement::AssignExpr {
                name,
                expr,
                fallback_value,
                ..
            } => {
                self.expression(expr, bound)?;
                self.word(fallback_value, bound)?;
                bind(bound, name);
            }
            Statement::Incr {
                name,
                amount,
                amount_braced,
                ..
            } => {
                read(name, bound)?;
                if let Some(amount) = amount.as_ref().filter(|_| !amount_braced) {
                    self.word(amount, bound)?;
                }
            }
            Statement::ExprEval { expr, .. } => self.expression(expr, bound)?,
            Statement::Call {
                args,
                tokens,
                defs,
                reads,
                reads_own_defs,
                safe_on_uninit,
                ..
            } => {
                for (index, arg) in args.iter().enumerate() {
                    let literal = tokens
                        .as_ref()
                        .is_some_and(|tokens| tokens.arg_is_braced_literal(index));
                    if !literal {
                        self.word(arg, bound)?;
                    }
                }
                for name in reads {
                    read(name, bound)?;
                }
                if *reads_own_defs && !*safe_on_uninit {
                    for name in defs {
                        read(name, bound)?;
                    }
                }
                for name in defs {
                    bind(bound, name);
                }
            }
            _ => unreachable!("`statement` sends `command` only the statements it reads"),
        }
        Some(())
    }

    /// A loop: its condition and its list words are read where it starts, and its
    /// body runs on a copy, for it may run no time at all.
    fn looping(
        &self,
        statement: &Statement,
        bound: &mut HashSet<String>,
        depth: u32,
    ) -> Option<Exit> {
        match statement {
            Statement::For {
                init,
                condition,
                next,
                body,
                ..
            } => {
                if self.script(init, bound, depth + 1)? == Exit::Returns {
                    return Some(Exit::Returns);
                }
                self.expression(condition, bound)?;
                let mut paths = Vec::new();
                self.path(body, bound, depth, &mut paths)?;
                if let Some(mut path) = paths.pop() {
                    self.script(next, &mut path, depth + 1)?;
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression(condition, bound)?;
                self.detached(body, bound, depth)?;
            }
            Statement::Foreach {
                iterators, body, ..
            } => {
                let mut inner = bound.clone();
                for iterator in iterators {
                    if !iterator.list_braced {
                        self.word(&iterator.list_arg, bound)?;
                    }
                    for name in &iterator.vars {
                        bind(&mut inner, name);
                    }
                }
                self.script(body, &mut inner, depth + 1)?;
            }
            _ => unreachable!("`statement` sends `looping` only the loops"),
        }
        Some(Exit::Falls)
    }

    /// A body that may stop part of the way: nothing it binds is counted on after
    /// it, and what it names its result and its handlers' variables bind.
    fn guarded(
        &self,
        statement: &Statement,
        bound: &mut HashSet<String>,
        depth: u32,
    ) -> Option<Exit> {
        match statement {
            Statement::Catch {
                body,
                result_var,
                options_var,
                ..
            } => {
                self.detached(body, bound, depth)?;
                for name in result_var.iter().chain(options_var) {
                    bind(bound, name);
                }
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                self.detached(body, bound, depth)?;
                for handler in handlers {
                    let mut inner = bound.clone();
                    for name in handler.var_name.iter().chain(&handler.options_var) {
                        bind(&mut inner, name);
                    }
                    self.script(&handler.body, &mut inner, depth + 1)?;
                }
                if let Some(body) = finally_body {
                    self.detached(body, bound, depth)?;
                }
            }
            _ => unreachable!("`statement` sends `guarded` only `catch` and `try`"),
        }
        Some(Exit::Falls)
    }

    /// Every name `text`, read as a word, substitutes.
    fn word(&self, text: &str, bound: &HashSet<String>) -> Option<()> {
        if braced_reference_in_substitution(text)
            || substitutes_a_command_beyond_values(text, self.registry, 0)
        {
            return None;
        }
        self.names(text, bound)
    }

    fn names(&self, text: &str, bound: &HashSet<String>) -> Option<()> {
        vars_in_word(text, self.registry)
            .iter()
            .try_for_each(|name| read(name, bound))
    }

    /// Every name an expression reads, as its text spells them: a variable, a
    /// quoted operand and a command it substitutes alike.
    fn expression(&self, expr: &ExprNode, bound: &HashSet<String>) -> Option<()> {
        if text_operand_substitutes(expr, 0) {
            return None;
        }
        let text = render_expr(expr);
        if substitutes_a_command_beyond_values(&text, self.registry, 0) {
            return None;
        }
        self.names(&text, bound)
    }
}

/// `name` read where `bound` is bound: a name the frame does not own is not the
/// frame's to refuse.
fn read(name: &str, bound: &HashSet<String>) -> Option<()> {
    let name = base(name);
    (!is_local(name) || bound.contains(name)).then_some(())
}

/// Whether `expr` has an operand the tree keeps as text that substitutes a
/// variable: the rename leaves it as written.
fn text_operand_substitutes(expr: &ExprNode, depth: u32) -> bool {
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    let next = depth + 1;
    match expr {
        ExprNode::Command { text, .. } | ExprNode::Raw { text } => text.contains('$'),
        ExprNode::String { text, .. } => text.starts_with('"') && text.contains('$'),
        ExprNode::Binary { left, right, .. } => {
            text_operand_substitutes(left, next) || text_operand_substitutes(right, next)
        }
        ExprNode::Unary { operand, .. } => text_operand_substitutes(operand, next),
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => {
            text_operand_substitutes(condition, next)
                || text_operand_substitutes(true_branch, next)
                || text_operand_substitutes(false_branch, next)
        }
        ExprNode::Call { args, .. } => args.iter().any(|arg| text_operand_substitutes(arg, next)),
        ExprNode::Literal { .. } | ExprNode::Var { .. } | ExprNode::CompiledWord { .. } => false,
    }
}

/// Whether `text` substitutes a command and holds a braced `$name`, other than as
/// the one `expr` command it is: `[expr {$x}]` evaluates its braces and
/// `[string length {$x}]` does not, and the rename must not tell them apart.
fn braced_reference_in_substitution(text: &str) -> bool {
    if !text.contains('$') || !text.contains('{') || !super::arg_has_command_subst(text) {
        return false;
    }
    let text = text.trim();
    let one_expr = text
        .strip_prefix("[expr")
        .is_some_and(|rest| rest.starts_with([' ', '\t', '{']))
        && text.ends_with(']')
        && text.matches('[').count() == 1;
    !one_expr
}

/// Whether `text` — a word, or the text of an expression — substitutes a command
/// that does more than compute from the values it is handed: one that takes a
/// variable's name, evaluates a script, or is not a command the registry knows,
/// in the substitution or in any word of it that substitutes in turn. A braced
/// word is read as well, for `expr` evaluates its braces.
fn substitutes_a_command_beyond_values(text: &str, registry: &CommandRegistry, depth: u32) -> bool {
    if super::MAX_INLINING_WALK_DEPTH.exceeded(depth) {
        return true;
    }
    if !text.contains('[') {
        return false;
    }
    let config = LexerConfig::for_profile(registry.profile())
        .nested()
        .normalized();
    let source_map = SourceMap::new(text);
    let Ok(tokens) = Lexer::with_config(text, config)
        .as_quoted_body()
        .tokenise_all()
    else {
        return true;
    };
    tokens.iter().any(|token| match token.kind {
        TokenType::Cmd => {
            segment_commands_with_offset_and_config(source_map.token_text(*token), 0, config)
                .iter()
                .any(|command| command_goes_beyond_values(command, registry, depth + 1))
        }
        TokenType::ExprSugar => {
            substitutes_a_command_beyond_values(source_map.token_text(*token), registry, depth + 1)
        }
        _ => false,
    })
}

/// Whether `command` is not one that works on its values alone, or has a word
/// that substitutes one that is not.
fn command_goes_beyond_values(
    command: &SegmentedCommand,
    registry: &CommandRegistry,
    depth: u32,
) -> bool {
    !works_on_values(command.name(), registry)
        || !variable_name_role_words(command, registry).is_empty()
        || command
            .args()
            .iter()
            .any(|word| substitutes_a_command_beyond_values(word, registry, depth + 1))
}

/// Whether `head`, a literal word, names a command the registry knows reads and
/// writes no variable by name, runs no script and does not depend on its frame:
/// one a frame can be moved from under.
fn works_on_values(head: &str, registry: &CommandRegistry) -> bool {
    !head.is_empty()
        && !head.contains(['$', '[', ']', '\\', '{', '}', '"'])
        && registry
            .get(head)
            .is_some_and(|spec| registry.is_splice_safe(head) || spec.traits.contains(Traits::PURE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn beyond_values(text: &str) -> bool {
        substitutes_a_command_beyond_values(text, &CommandRegistry::build_default(), 0)
    }

    #[test]
    fn a_command_that_works_on_its_values_is_not_beyond_them() {
        for text in [
            "[string length $x]",
            "[list $x [llength $x]]",
            "[format %s-%s $x [string length $x]]",
            "[tcl::mathfunc::abs $x]",
            "$x and ${y}, with no command in it",
        ] {
            assert!(!beyond_values(text), "{text}");
        }
    }

    #[test]
    fn a_command_that_takes_a_name_runs_a_script_or_is_unknown_is_beyond_them() {
        for text in [
            "[set y]",
            "x[incr y]z",
            "[info exists y]",
            "[namespace current]",
            "[helper $x]",
            "[$command $x]",
            "[catch {incr x}]",
            "[eval {set x}]",
            "[string is integer -failindex bad $s]",
            "[list [set x]]",
            "[expr {[set x] + 1}]",
        ] {
            assert!(beyond_values(text), "{text}");
        }
    }

    #[test]
    fn a_substitution_nested_past_the_walk_depth_is_beyond_them() {
        let registry = CommandRegistry::build_default();
        assert!(substitutes_a_command_beyond_values(
            "[list a]",
            &registry,
            u32::MAX / 2
        ));
    }
}
