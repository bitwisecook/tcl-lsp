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

use tcl_lexer::LexerConfig;
#[cfg(test)]
use tcl_lexer::{Lexer, SourceMap, TokenType};
use tcl_registry::{CommandRegistry, Traits};

use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{ExprNode, render_expr};
use crate::ir::{CommandTokens, Module, Procedure, Script, Statement, WordExpr, WordPart};
#[cfg(test)]
use crate::segmenter::{SegmentedCommand, segment_commands_with_offset_and_config};
#[cfg(test)]
use crate::var_refs::variable_name_role_words;
use crate::var_refs::{VarReferenceScanner, VarScanOptions};

/// Whether every variable `proc`'s body reads is bound when it is read.
pub(super) fn reads_only_bound_names(proc: &Procedure, source: SourceContext<'_>) -> bool {
    let frame = Frame { source };
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

/// Complete source metadata, distinct from runtime frame or splice admission.
#[derive(Clone, Copy)]
pub(super) struct SourceContext<'a> {
    pub(super) registry: &'a CommandRegistry,
    pub(super) metadata: crate::registry_invocation::InvocationMetadataContext<'a>,
    pub(super) config: LexerConfig,
    image: &'a tcl_lexer::SourceImage,
}

impl<'a> SourceContext<'a> {
    pub(super) fn for_module(module: &'a Module, registry: &'a CommandRegistry) -> Option<Self> {
        if !module
            .retained_source_bindings
            .as_ref()?
            .matches_module(module, registry)
        {
            return None;
        }
        let metadata =
            crate::registry_invocation::InvocationMetadataContext::for_module(registry, module)?;
        metadata.permits_logical_source_names().then_some(Self {
            registry,
            metadata,
            config: module.lexer_config,
            image: &module.source,
        })
    }

    pub(super) fn invocation(
        self,
        tokens: &CommandTokens,
    ) -> Option<crate::registry_invocation::ResolvedStatementInvocation> {
        self.accepts_tokens(tokens)?;
        crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
            self.registry,
            self.metadata,
            tokens,
        )
        .filter(|invocation| {
            invocation.facts.arg_roles_complete
                && invocation.facts.arity_accepts_frozen_arguments() == Some(true)
        })
    }
    fn accepts_tokens(self, tokens: &CommandTokens) -> Option<()> {
        let binding = tokens.source_binding.as_ref()?;
        (binding.logical_source_name_advice_input() == self.metadata.source_analysis_input()
            && binding.source_origin()?.source_image() == self.image
            && binding
                .original_lexer_config_for_tokens(tokens)?
                .normalized()
                == self.config.normalized())
        .then_some(())
    }
}

struct Frame<'a> {
    source: SourceContext<'a>,
}

/// The variable a name addresses: an array element is its array.
fn base(name: &str) -> &str {
    tcl_syntax::naming::split_element_ref(name).map_or(name, |(array, _)| array)
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
            let tokens = script.retained_source_tokens_for_statement(statement)?;
            self.source_tokens(tokens, bound)?;
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
            Statement::UpFrame { .. }
            | Statement::Barrier { .. }
            | Statement::NativeCall { .. } => None,
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

    /// Original children retain their own point-specific source lookup. The
    /// registry never selects a substitution from its written head here.
    fn source_tokens(&self, tokens: &CommandTokens, bound: &HashSet<String>) -> Option<()> {
        self.source.invocation(tokens)?;
        for word in tokens.words() {
            self.original_word_reads(word, bound)?;
        }
        let calls = crate::word_subst::checked_lifted_calls(tokens, self.source.config)?;
        for call in calls {
            let child = call.tokens.as_ref()?;
            let invocation = self.source.invocation(child)?;
            if !invocation
                .facts
                .traits
                .intersects(Traits::PURE | Traits::PURE_EVALUATION)
                || invocation.facts.arg_roles.iter().any(|(_, role)| {
                    role.names_variable()
                        || matches!(
                            role,
                            tcl_registry::ArgRole::Body | tcl_registry::ArgRole::LambdaLiteral
                        )
                })
            {
                return None;
            }
            let roles = invocation.written_argument_roles();
            for (index, word) in child.words().iter().skip(1).enumerate() {
                if let WordExpr::BracedLiteral { text, .. } = word {
                    let expression_role = roles
                        .iter()
                        .any(|(at, role)| *at == index && *role == tcl_registry::ArgRole::Expr);
                    if expression_role {
                        let input = self.source.metadata.source_analysis_input()?;
                        let mut parser = tcl_syntax::expr::parser::ExprParseContext::for_profile(
                            input.unit_profile(),
                        );
                        parser.lexer_grammar =
                            self.source.config.grammar_over(parser.lexer_grammar);
                        let expression =
                            crate::expr_parser::parse_expr_with_syntax_context(text, &parser);
                        self.expression(&expression, bound)?;
                    } else if text.contains('$') {
                        // The recursive substitution rewrite cannot alter literal
                        // data into a reference belonging to the caller's frame.
                        return None;
                    }
                }
            }
        }
        Some(())
    }

    fn original_word_reads(&self, word: &WordExpr, bound: &HashSet<String>) -> Option<()> {
        match word {
            WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => Some(()),
            WordExpr::Variable { spelling, .. }
            | WordExpr::CommandSubstitution { spelling, .. } => self.names(spelling, bound),
            WordExpr::Template {
                parts,
                rejected: None,
                ..
            } => parts.iter().try_for_each(|part| match part {
                WordPart::Text { .. } => Some(()),
                WordPart::Variable { spelling, .. }
                | WordPart::CommandSubstitution { spelling, .. } => self.names(spelling, bound),
                WordPart::Opaque { .. } => None,
            }),
            WordExpr::Template {
                rejected: Some(_), ..
            }
            | WordExpr::Expand { .. }
            | WordExpr::Opaque { .. } => None,
        }
    }

    /// Every lexical reference is read under this exact original grammar.
    /// Variable-name command roles are handled by selected child metadata.
    fn word(&self, text: &str, bound: &HashSet<String>) -> Option<()> {
        self.names(text, bound)
    }

    fn names(&self, text: &str, bound: &HashSet<String>) -> Option<()> {
        let parts = tcl_lexer::word_parts::decompose_spanned_checked(
            text.as_bytes(),
            tcl_lexer::word_parts::SubstFlags::default(),
            self.source.config,
        )
        .ok()?;
        if parts
            .iter()
            .any(|part| matches!(part.part, tcl_lexer::word_parts::WordPart::ParseError(_)))
        {
            return None;
        }
        VarReferenceScanner::with_config(VarScanOptions::default(), self.source.config)
            .scan_word(text, self.source.registry)
            .iter()
            .try_for_each(|name| read(name, bound))
    }

    fn expression(&self, expr: &ExprNode, bound: &HashSet<String>) -> Option<()> {
        if text_operand_substitutes(expr, 0) {
            return None;
        }
        self.names(&render_expr(expr), bound)
    }
}

/// `name` read where `bound` is bound: a name the frame does not own is not the
/// frame's to refuse.
fn read(name: &str, bound: &HashSet<String>) -> Option<()> {
    let name = base(name);
    (!is_local(name) || bound.contains(name)).then_some(())
}

/// Whether `expr` contains opaque syntax or a substituting text operand the
/// expression rename cannot project onto an original variable extent.
fn text_operand_substitutes(expr: &ExprNode, depth: u32) -> bool {
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return true;
    }
    let next = depth + 1;
    match expr {
        ExprNode::Command { .. } | ExprNode::Raw { .. } => true,
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
#[cfg(test)]
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
#[cfg(test)]
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
#[cfg(test)]
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
#[cfg(test)]
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
    fn original_frame_context_keeps_selected_aliases_and_shadow_refusal() {
        // naming.inlining.original-frame-source-context
        // docs/design/analysis/name-resolution-proofs/inlining-original-frame-source-context.md
        // Conditional Logical source eligibility; no Native activation is issued.
        for source in [
            "proc p {x} {return [string length $x]}",
            "rename string ::moved_string; proc p {x} {return [::moved_string length $x]}",
            "interp alias {} sl {} string length; proc p {x} {return [sl $x]}",
            "proc p {x} {return [expr {$x + 1}]}",
            "proc p {} {set {$literal} VALUE; return ${$literal}}",
            "proc p {} {set {scalar(open} VALUE; return ${scalar(open}}",
        ] {
            let (context, module) = crate::inlining::tests::logical_module_for(source);
            let selected = SourceContext::for_module(&module, context.commands()).unwrap();
            assert!(
                reads_only_bound_names(&module.procedures["::p"], selected),
                "{source}"
            );
        }
        for source in [
            "proc string {args} {return 1}; proc p {x} {return [string length $x]}",
            "proc p {x} {return [unknown_worker $x]}",
            "proc p {x} {return [set y]}",
            "proc p {x} {return [list {$x}]}",
            "proc p {x} {return [expr {[set y]}]}",
        ] {
            let (context, module) = crate::inlining::tests::logical_module_for(source);
            let selected = SourceContext::for_module(&module, context.commands()).unwrap();
            assert!(
                !reads_only_bound_names(&module.procedures["::p"], selected),
                "{source}"
            );
        }
    }

    #[test]
    fn original_frame_context_declines_missing_foreign_and_stale_carriers() {
        // naming.inlining.original-frame-source-context
        // docs/design/analysis/name-resolution-proofs/inlining-original-frame-source-context.md
        let (context, original) =
            crate::inlining::tests::logical_module_for("proc p {x} {return $x}; p 1");
        let selected = SourceContext::for_module(&original, context.commands()).unwrap();
        assert!(reads_only_bound_names(
            &original.procedures["::p"],
            selected
        ));
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        assert!(SourceContext::for_module(&original, foreign.commands()).is_none());
        let mut missing = original.clone();
        missing.source_metadata_input = None;
        assert!(SourceContext::for_module(&missing, context.commands()).is_none());
        let mut changed_config = original.clone();
        changed_config.lexer_config.strict_quoting = !changed_config.lexer_config.strict_quoting;
        assert!(SourceContext::for_module(&changed_config, context.commands()).is_none());
        let mut stale_source = original.clone();
        stale_source.source = tcl_lexer::SourceImage::document("proc p {x} {return $missing}; p 1");
        assert!(SourceContext::for_module(&stale_source, context.commands()).is_none());
        let mut cooked = original.procedures["::p"].clone();
        cooked.body.command_binding_sites = Default::default();
        let Statement::Return { tokens, .. } = &mut cooked.body.statements[0] else {
            panic!("genuine original return fixture");
        };
        *tokens = None;
        assert!(!reads_only_bound_names(&cooked, selected));
    }

    #[test]
    fn native_dispatch_is_not_spliced_without_frame_or_head_rewrite_evidence() {
        let statement = crate::ir::native_call_for_test(b"helper $x");
        let (context, module) = crate::inlining::tests::logical_module_for("set x 1");
        let source = SourceContext::for_module(&module, context.commands()).unwrap();
        let mut bound = HashSet::from(["x".to_owned()]);
        assert!(
            Frame { source }
                .statement(&statement, &mut bound, 0)
                .is_none()
        );
        assert!(super::super::heads::names_a_command(
            std::slice::from_ref(&statement),
            source.config
        ));
        assert!(super::super::heads::root(vec![statement], source.config).is_none());
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
