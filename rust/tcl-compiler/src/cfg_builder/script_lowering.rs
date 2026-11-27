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

//! Lowering a script statement by statement: plain statements, loop jumps, `return`, split points and structured-command boundaries.

use super::split_points::{NextFailure, SplitPoint, is_literal_assignment};
use super::{CfgBuilder, MAX_LOWER_DEPTH};
use crate::cfg::Terminator;
use crate::ir::{CommandBindingSite, Script, Statement};
use tcl_lexer::Span;

impl CfgBuilder<'_> {
    /// Consume a proved native break/continue in an analysis loop. Completion
    /// comes from the shared frozen invocation projection; missing proof or
    /// dynamic completion cannot establish an edge. Execution graphs retain
    /// the invocation and its normal runtime-replay continuation.
    pub(super) fn lower_loop_jump(&mut self, current: &str, stmt: &Statement) -> bool {
        use tcl_registry::completion_route::InvocationCompletionRoute as Route;
        // Execution retains the invocation's normal continuation for runtime
        // binding replay. The emitter handles a native jump in its selected
        // lexical loop; a replacement command can return normally instead.
        if self.plain_command_dispatch || !self.faithful_exceptions {
            return false;
        }
        let Statement::Call { span, .. } = stmt else {
            return false;
        };
        let is_break = match self.command_classes.completion_route(stmt) {
            Some(Route::Tcl(tcl_registry::CompletionCode::Break)) => true,
            Some(Route::Tcl(tcl_registry::CompletionCode::Continue)) => false,
            _ => return false,
        };
        let Some((brk, cont)) = self.loop_stack.last().cloned() else {
            return false;
        };
        let Some(target_name) = (if is_break { Some(brk) } else { cont }) else {
            return false;
        };
        let target = self.bid(&target_name);
        self.push_statement(current, stmt.clone());
        self.set_terminator(
            current,
            Terminator::Goto {
                target,
                span: Some(*span),
            },
        );
        true
    }

    /// Whether `stmt` ends the block it is lowered into and leaves from the
    /// state before it: a `return`, a `break` or `continue` a loop takes, or a
    /// command that terminates the block (`error`, `throw`, `exit`, and in an
    /// analysis build `tailcall`), each with words that substitute nothing, so
    /// it stores nothing and cannot fail before it completes with its code. A
    /// split region keeps such a statement in the block before it: the block's
    /// exit is the state the statement leaves from, and the block's statements
    /// are the run whose exact completion a handler is matched against
    /// (`set z 0; error boom` completes with an error whichever command raises).
    pub(super) fn leaves_from_the_state_before(&self, stmt: &Statement) -> bool {
        let literal = |word: &tcl_source::words::WordExpr| {
            matches!(
                word,
                tcl_source::words::WordExpr::Literal { .. }
                    | tcl_source::words::WordExpr::BracedLiteral { .. }
            )
        };
        match stmt {
            Statement::Return {
                value,
                value_word,
                braced,
                ..
            } => value.is_none() || *braced || value_word.as_ref().is_some_and(literal),
            Statement::Call { tokens, .. } => {
                if self.plain_command_dispatch
                    || !tokens
                        .as_ref()
                        .is_some_and(|tokens| tokens.word_exprs.iter().all(literal))
                {
                    return false;
                }
                match self.command_classes.completion_route(stmt) {
                    Some(tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::CompletionCode::Break,
                    )) => !self.loop_stack.is_empty(),
                    Some(tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::CompletionCode::Continue,
                    )) => self
                        .loop_stack
                        .last()
                        .is_some_and(|(_, cont)| cont.is_some()),
                    Some(route) => !route.normal_possible(),
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// The point a split ends at `ended`, where `next` is the statement after
    /// it: a statement of `ended` may leave after its own stores, unless each
    /// is a literal assignment, which raises before it stores if it raises at
    /// all — a block with no statement joins paths that may — and `next` may
    /// fail before it stores ([`NextFailure::of`]).
    fn split_point(&self, ended: String, next: &Statement) -> SplitPoint {
        let after_stores = self.blocks.get(&ended).is_none_or(|block| {
            block.statements.is_empty() || !block.statements.iter().all(is_literal_assignment)
        });
        SplitPoint {
            block: ended,
            after_stores,
            next: NextFailure::of(next),
        }
    }

    /// Push a non-control-flow statement into `current` (after upvar
    /// invalidation), promoting `error` / `throw` / `exit` (and, in analysis
    /// builds, `tailcall`) to a `Return` terminator so any following statements
    /// become dead code (mirrors the `TERMINATES_BLOCK` registry trait).
    pub(super) fn push_plain_statement(&mut self, current: &str, stmt: &Statement) {
        if self.bpf_closed_effects.is_some() {
            // Every call was certified as an actual closed BPF core operation.
            // It is emitted as DSL data, never dispatched through Tcl.
            self.push_statement(current, stmt.clone());
            return;
        }
        for s in self.in_barrier_block(current, |builder| {
            builder.apply_upvar_invalidation(stmt.clone())
        }) {
            self.push_statement(current, s);
        }
        // A runtime binding check can replay a specialised command through a
        // replacement that returns normally. Codegen must retain that normal
        // continuation rather than manufacture a Tcl return after the invoke.
        if self.plain_command_dispatch || !self.faithful_exceptions {
            return;
        }
        if let Statement::Call { span, .. } | Statement::Barrier { span, .. } = stmt
            && self.block_mut(current).terminator.is_none()
        {
            let route = self.command_classes.completion_route(stmt).or_else(|| {
                let tokens = stmt.tokens()?;
                self.command_classes
                    .conditional_source_completion_route(tokens)
                    .or_else(|| {
                        self.command_classes
                            .logical_structured_invocation(tokens)
                            .map(|invocation| {
                                invocation.conditional_completion_route(self.registry)
                            })
                    })
            });
            let Some(route) = route else {
                return;
            };
            if !route.normal_possible() {
                if !matches!(
                    route,
                    tcl_registry::completion_route::InvocationCompletionRoute::ProcessExit
                ) && let Some(blocks) = self.throw_blocks.as_mut()
                {
                    blocks.push(current.to_owned());
                }
                self.set_terminator(
                    current,
                    Terminator::Complete {
                        route,
                        span: Some(*span),
                    },
                );
            }
        }
    }

    pub(super) fn lower_script(&mut self, script: &Script, block_name: &str) -> Option<String> {
        for proof in &script.expression_preparations {
            if !self.expression_preparations.contains(proof) {
                self.expression_preparations.push(proof.clone());
            }
        }
        for proof in &script.implicit_math_invocations {
            if !self.implicit_math_invocations.contains(proof) {
                self.implicit_math_invocations.push(proof.clone());
            }
        }
        let previous_source =
            std::mem::replace(&mut self.current_source, script.executed_source.clone());
        if self.faithful_exceptions && script.native_compilation_failure.is_some() {
            // Compilation happens before this chunk's first source effect.
            // Analysis must not retain fictitious stores or calls preceding the
            // rejected syntax. Execution retains source instructions separately
            // for admission-time dependency validation and native presentation.
            self.set_terminator(
                block_name,
                Terminator::Complete {
                    route: tcl_registry::completion_route::InvocationCompletionRoute::Tcl(
                        tcl_registry::completion::CompletionCode::Error,
                    ),
                    span: None,
                },
            );
            if let Some(blocks) = self.throw_blocks.as_mut() {
                blocks.push(block_name.to_owned());
            }
            self.current_source = previous_source;
            return Some(block_name.to_owned());
        }
        self.command_binding_sites
            .extend(script.command_binding_sites.iter().cloned());
        self.procedure_binding_requirements
            .extend(script.procedure_binding_requirements.iter().cloned());
        self.depth += 1;
        if MAX_LOWER_DEPTH.exceeded(self.depth) {
            self.depth -= 1;
            self.current_source = previous_source;
            return None;
        }

        let result = self.lower_script_inner(script, block_name);
        self.current_source = previous_source;
        self.depth -= 1;
        result
    }

    /// Retain the original return-with-options invocation and project its
    /// established abrupt route without evaluating a second return.
    fn lower_return_options_barrier(&mut self, stmt: &Statement, span: Span, current: &str) {
        let Some(route) = self.command_classes.completion_route(stmt) else {
            return;
        };
        self.push_plain_statement(current, stmt);
        self.set_terminator(
            current,
            Terminator::Complete {
                route,
                span: Some(span),
            },
        );
    }

    fn lower_return_statement(&mut self, stmt: &Statement, current: &str) {
        let Statement::Return {
            span,
            tokens,
            value,
            value_word,
            expr,
            expr_base,
            command_binding,
            braced,
        } = stmt
        else {
            unreachable!();
        };

        // Every substitution in the returned value runs before the frame
        // unwinds. Materialise its analysis effects before installing the
        // terminator.
        self.push_embedded_control_effects(
            stmt,
            current,
            "return value invokes an opaque embedded command",
        );
        if let Some(binding) = command_binding {
            self.command_binding_sites.push(CommandBindingSite {
                span: *span,
                binding: binding.clone(),
                known_namespaces: None,
                variable_frame: None,
                variable_context: None,
                existing_namespace_cells: None,
                source_tokens: None,
                shape: tcl_registry::release_compilation::InvocationShape::counted(usize::from(
                    value.is_some() || expr.is_some(),
                )),
            });
        }
        // No value, a braced one, or a literal word: nothing can raise before
        // the `return` itself completes with `TCL_RETURN`.
        if value.is_none()
            || *braced
            || matches!(
                value_word,
                Some(
                    tcl_source::words::WordExpr::Literal { .. }
                        | tcl_source::words::WordExpr::BracedLiteral { .. }
                )
            )
        {
            self.plain_return_blocks.insert(current.to_owned());
        }
        self.set_terminator(
            current,
            Terminator::Return {
                value: value.clone(),
                tokens: tokens.clone().map(Box::new),
                value_word: value_word.clone(),
                span: Some(*span),
                expr: expr.clone(),
                expr_base: *expr_base,
                braced: *braced,
            },
        );
    }

    fn lower_script_statement(&mut self, stmt: &Statement, current: &str) -> Option<String> {
        self.mark_command_boundary(current, stmt);
        self.record_body_error_sites(stmt);
        if self.faithful_exceptions
            && !self.plain_command_dispatch
            && let Some(region) = Self::evaluated_body_region(stmt)
        {
            return Some(self.lower_evaluated_body(stmt, region, current));
        }
        match stmt {
            Statement::If { .. } => Some(self.lower_if(stmt, current)),
            Statement::For { .. } => self.lower_for_or_frozen(stmt, current),
            Statement::While { .. } => Some(self.lower_while_or_frozen(stmt, current)),
            Statement::Foreach { .. } => Some(self.lower_foreach_dispatch(stmt, current)),
            Statement::Catch { .. } => Some(self.lower_catch_dispatch(stmt, current)),
            Statement::Try { .. } => Some(self.lower_try_dispatch(stmt, current)),
            Statement::Switch { .. } => Some(self.lower_switch(stmt, current)),
            Statement::Return { .. } => {
                self.lower_return_statement(stmt, current);
                Some(current.to_owned())
            }
            // Inline block: flatten the body's statements into the current
            // control-flow stream so SSA / codegen see them as plain inline
            // statements. Preserve any registry-described Tcl error context
            // with the original command span.
            Statement::Block {
                body,
                span,
                error_context,
                ..
            } => {
                if !self.faithful_exceptions
                    && crate::registry_invocation::block_requires_runtime_script(stmt)
                {
                    let boundary =
                        crate::registry_invocation::original_block_runtime_invocation(stmt)
                            .unwrap_or_else(|| stmt.clone());
                    self.push_plain_statement(current, &boundary);
                    return Some(current.to_owned());
                }
                if let Some(context) = error_context {
                    self.inline_body_error_sites
                        .push(crate::cfg::InlineBodyErrorSite {
                            span: *span,
                            context: crate::cfg::BodyErrorContext::Registry(*context),
                            region: *span,
                            compiled: crate::cfg::CompiledAt::NEVER,
                        });
                }
                let owns_inline_region = self
                    .command_boundary_sites
                    .get(current)
                    .is_some_and(|site| site.span == *span);
                if !self.faithful_exceptions && owns_inline_region {
                    let owner = self.command_boundary_sites.get(current).cloned();
                    let body_block = self.new_block("inline_block_body");
                    let continuation = self.new_block("inline_block_end");
                    self.ensure_goto(current, &body_block, Some(*span));
                    self.command_boundary_continuations
                        .insert(current.to_owned(), continuation.clone());
                    if let Some(tail) = self.lower_body(owner.as_ref(), body, &body_block) {
                        self.ensure_goto(&tail, &continuation, Some(*span));
                    }
                    Some(continuation)
                } else {
                    self.lower_script(body, current)
                }
            }
            // `return -options …` / `return {*}…args` lower to a
            // Statement::Barrier, but still unconditionally exit the proc
            // in analysis builds.
            Statement::Barrier { span, .. }
                if self.faithful_exceptions
                    && stmt.is_return_barrier()
                    && self
                        .command_classes
                        .completion_route(stmt)
                        .is_some_and(|route| !route.normal_possible()) =>
            {
                self.lower_return_options_barrier(stmt, *span, current);
                Some(current.to_owned())
            }
            other => {
                self.push_plain_statement(current, other);
                Some(current.to_owned())
            }
        }
    }

    /// Associate `block` with the explicit structured-command site retained by
    /// lowering for `stmt`. Statements without such a site are deliberately not
    /// inferred from their source text.
    fn mark_command_boundary(&mut self, block: &str, stmt: &Statement) {
        if let Some(site) = self
            .command_binding_sites
            .iter()
            .rev()
            .find(|site| site.span == stmt.span())
            .cloned()
        {
            self.command_boundary_sites.insert(block.to_owned(), site);
        }
    }

    /// Lower `body`, a script the structured command `owner` evaluates, into
    /// `entry`, recording the blocks that run it.
    pub(super) fn lower_body(
        &mut self,
        owner: Option<&CommandBindingSite>,
        body: &Script,
        entry: &str,
    ) -> Option<String> {
        let block_count = |builder: &Self| {
            u32::try_from(builder.block_ids.len()).expect("CFG block count fits in u32")
        };
        let nested_from = block_count(self);
        let tail = self.lower_script(body, entry);
        if let Some(site) = owner {
            self.body_scripts.push(crate::cfg::BodyScript {
                owner: site.clone(),
                entry: self.bid(entry),
                nested: nested_from..block_count(self),
            });
        }
        tail
    }

    /// Propagate an established owner to a helper block which emits the same
    /// structured command's synthetic runtime boundary.
    pub(super) fn copy_command_boundary(&mut self, owner: &str, block: &str) {
        if let Some(site) = self.command_boundary_sites.get(owner).cloned() {
            self.command_boundary_sites.insert(block.to_owned(), site);
        }
    }

    /// Retain an established source owner for native condition reads only.
    pub(super) fn copy_condition_binding(&mut self, owner: &str, condition: &str) {
        let site = self.command_boundary_sites.get(owner).cloned();
        self.retain_condition_binding(site.as_ref(), condition);
    }

    pub(super) fn retain_condition_binding(
        &mut self,
        site: Option<&CommandBindingSite>,
        condition: &str,
    ) {
        if let Some(site) = site {
            self.condition_binding_sites
                .insert(condition.to_owned(), site.clone());
        }
    }

    fn lower_script_inner(&mut self, script: &Script, block_name: &str) -> Option<String> {
        let mut current = block_name.to_owned();
        // The block the *main* (reachable) path ends in once it hits an
        // unconditional terminator — everything after is dead code captured
        // in orphan blocks, and the script does not fall through to its
        // caller — with the number of throw points recorded by then.
        let mut main_terminal: Option<(String, usize)> = None;
        // A `catch` body's request is for this script alone, not for those it
        // holds; a `try` body's holds for every script inside it.
        let split = match self.split_region {
            Some(region) if region.nested() => Some(region),
            _ => self.split_region.take(),
        };

        for (index, stmt) in script.statements.iter().enumerate() {
            if let Some(region) = split
                && index > 0
                && self.block_mut(&current).terminator.is_none()
                && !(region.selective() && self.leaves_from_the_state_before(stmt))
            {
                let next = self.new_block(region.step());
                self.copy_command_boundary(block_name, &next);
                self.ensure_goto(&current, &next, Some(stmt.span()));
                self.split_parent.insert(next.clone(), current.clone());
                let ended = std::mem::replace(&mut current, next);
                let point = self.split_point(ended, stmt);
                if !region.selective() || point.may_leave() {
                    self.split_points.push(point);
                }
            }
            // If the current block is already terminated, subsequent
            // statements are dead code.  Route them into a fresh orphan
            // block with no incoming edge (rather than dropping them) so
            // SCCP marks it unreachable and O107 can flag the dead code.
            if self.block_mut(&current).terminator.is_some() {
                if main_terminal.is_none() {
                    let throws = self.throw_blocks.as_ref().map_or(0, Vec::len);
                    main_terminal = Some((current.clone(), throws));
                }
                current = self.new_block("unreachable");
            }

            // `break` / `continue` inside a loop body lower to a CFG edge to
            // the loop's exit / continue target, so the loop-exit block stays
            // reachable (a `while 1 { … break }` post-loop block is live).
            if self.lower_loop_jump(&current, stmt) {
                continue;
            }
            current = self.lower_script_statement(stmt, &current)?;
        }

        // Always return the block control finally rests in, even when it is
        // terminated by a straight-line `return`/`error`.
        // `build_function` then appends a synthetic (unreachable) `exit` block
        // via `ensure_goto` (a no-op on the already-terminated block).
        // Termination is tracked separately in
        // `last_terminal_block` so `lower_try`
        // can source an on-error edge from a body that ended without an explicit
        // `error`/`throw`.  Nested control-flow lowerings still signal "no
        // continuation" by returning `None` (propagated through this loop's
        // `?` / explicit-`None` arms).
        // The script leaves where its main path ends, not in the dead code
        // after it, and the dead code holds no throw point: a `try` handler is
        // thrown to from there, so in `try {return r3; set y 1} on return {}
        // {…}` the handler's source is the block of `return r3`, never the
        // orphan block of `set y 1`, which no path reaches.
        self.last_terminal_block = match main_terminal {
            Some((block, throws)) => {
                if let Some(blocks) = self.throw_blocks.as_mut() {
                    blocks.truncate(throws);
                }
                Some(block)
            }
            None => self
                .block_mut(&current)
                .terminator
                .is_some()
                .then(|| current.clone()),
        };
        Some(current)
    }
}
