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

//! Flattening `for`, `while` and `foreach` into header, body and step blocks.

use super::super::CfgBuilder;
use crate::cfg::{LoopNode, Terminator};
use crate::expr_ast::ExprNode;
use crate::ir::Statement;
use crate::ir_helpers::expr_has_command;
use tcl_lexer::Span;

/// Per-word token metadata for the synthetic loop-header call
/// [`CfgBuilder::lower_foreach`] builds.
///
/// The selected input words keep their physical read sites and argument-time
/// snapshots. Synthetic iteration bindings do not re-resolve a written command
/// or evaluate its original arguments again.
fn foreach_binding_statement(statement: &Statement) -> Statement {
    let Statement::Foreach {
        span,
        iterators,
        is_lmap,
        is_dict_iteration,
        is_array_iteration,
        raw_tokens,
        ..
    } = statement
    else {
        unreachable!("iteration binding requires Foreach");
    };
    // Collect all iteration variable names.  The ``defs``
    // vector is a flattened concatenation of every iterator
    // group's vars; ``foreach_groups`` records the size of
    // each group so the codegen can reconstruct the original
    // ``var-list`` ↔ ``list-arg`` pairing.
    let all_vars: Vec<String> = iterators.iter().flat_map(|it| it.vars.clone()).collect();
    let group_sizes: Vec<usize> = iterators.iter().map(|it| it.vars.len()).collect();
    let list_args: Vec<String> = iterators.iter().map(|it| it.list_arg.clone()).collect();

    let fe_cmd = match (*is_dict_iteration, *is_lmap) {
        (true, false) => "dict for",
        (true, true) => "dict map",
        (false, true) => "lmap",
        (false, false) => "foreach",
    };

    let header_tokens = foreach_header_tokens(
        fe_cmd,
        *span,
        iterators,
        &list_args,
        raw_tokens.as_ref(),
        *is_dict_iteration || *is_array_iteration,
        if *is_array_iteration {
            None
        } else if *is_dict_iteration {
            Some(tcl_registry::TclType::Dict)
        } else {
            Some(tcl_registry::TclType::List)
        },
    );

    // Synthetic def node for iteration variables (placed at the header for
    // the normal shape, or at the top of the body when rotated).
    Statement::Call {
        span: *span,
        command: fe_cmd.into(),
        canonical_command: None,
        args: list_args,
        defs: all_vars,
        reads: vec![],
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: Some(header_tokens),
        foreach_groups: Some(group_sizes),
    }
}

/// Exact typed list-iteration header, independently of its reporting head.
/// The caller must retain the current CFG edges and source-owner correspondence.
#[doc(hidden)]
#[must_use]
pub fn matches_foreach_header(
    original: &Statement,
    statement: &Statement,
    terminator: &Terminator,
) -> bool {
    if !matches!(
        original,
        Statement::Foreach {
            is_dict_iteration: false,
            is_array_iteration: false,
            ..
        }
    ) {
        return false;
    }
    if !matches!(
        terminator,
        Terminator::Branch {
            condition,
            span,
            condition_base: None,
            ..
        } if condition == &foreach_condition() && *span == Some(original.span())
    ) {
        return false;
    }
    let expected = foreach_binding_statement(original);
    let mut actual = statement.clone();
    let (
        Statement::Call {
            command,
            tokens: Some(tokens),
            ..
        },
        Statement::Call {
            command: expected_command,
            tokens: Some(expected_tokens),
            ..
        },
    ) = (&mut actual, &expected)
    else {
        return false;
    };
    let (Some(text), Some(word)) = (tokens.argv_texts.first_mut(), tokens.word_exprs.first_mut())
    else {
        return false;
    };
    // The synthetic head is a report label; every operand and owner stays exact.
    command.clone_from(expected_command);
    text.clone_from(&expected_tokens.argv_texts[0]);
    word.clone_from(&expected_tokens.word_exprs[0]);
    actual == expected
}

fn foreach_condition() -> ExprNode {
    ExprNode::Raw {
        text: "<foreach_has_next>".into(),
    }
}

fn foreach_header_tokens(
    fe_cmd: &str,
    span: Span,
    iterators: &[crate::ir::ForeachIterator],
    list_args: &[String],
    original: Option<&crate::ir::CommandTokens>,
    compound: bool,
    input_representation: Option<tcl_registry::TclType>,
) -> crate::ir::CommandTokens {
    let mut argv_kinds = vec![tcl_lexer::TokenType::Esc];
    argv_kinds.extend(iterators.iter().map(|it| {
        if it.list_braced {
            tcl_lexer::TokenType::Str
        } else {
            tcl_lexer::TokenType::Esc
        }
    }));
    let mut argv_texts = vec![fe_cmd.to_owned()];
    argv_texts.extend(list_args.iter().cloned());
    let mut tokens = crate::ir::CommandTokens::from_lossy_parts(
        vec![span; iterators.len() + 1],
        argv_texts,
        argv_kinds,
        vec![true; iterators.len() + 1],
        Vec::new(),
        None,
    );
    if let Some(original) = original {
        tokens.source_binding.clone_from(&original.source_binding);
        tokens.nested_bindings.clone_from(&original.nested_bindings);
        tokens
            .variable_accesses
            .clone_from(&original.variable_accesses);
        if let Some(effective) = crate::registry_invocation::effective_command_words(original) {
            let (first, stride) = if compound { (3, 1) } else { (2, 2) };
            for index in 0..iterators.len() {
                if let Some(word) = effective.words.get(first + index * stride) {
                    tokens.argv[index + 1] = word.source().span;
                    tokens.word_exprs[index + 1] = word.clone();
                }
            }
        }
    }
    tokens.synthetic = Some(crate::ir::SyntheticMarker::IterationBindings(
        input_representation,
    ));
    tokens
}

/// A foldable always-true literal condition (`1`) for a rotated loop's
/// synthetic entry-guard branch.  Span-less offsets (`0`) keep it distinct
/// from any real source condition; SCCP folds it so the zero-iteration
/// guard→exit edge is pruned.
fn literal_true_expr() -> ExprNode {
    ExprNode::Literal {
        text: "1".into(),
        start: 0,
        end: 0,
    }
}

/// The always-false counterpart of [`literal_true_expr`], guarding a loop
/// body that provably never runs.
fn literal_false_expr() -> ExprNode {
    ExprNode::Literal {
        text: "0".into(),
        start: 0,
        end: 0,
    }
}

impl CfgBuilder<'_> {
    pub(in super::super) fn lower_for(
        &mut self,
        stmt: &Statement,
        block_name: &str,
    ) -> Option<String> {
        let Statement::For {
            init,
            init_span,
            condition,
            condition_span,
            condition_base,
            next,
            next_span,
            body,
            body_span,
            ..
        } = stmt
        else {
            unreachable!("lower_for called with non-For");
        };

        let condition_owner = self.command_boundary_sites.get(block_name).cloned();

        // Placeholder for empty init clause.
        if init.statements.is_empty() {
            self.push_empty_clause(block_name, *init_span);
        }
        let init_tail = self.lower_script(init, block_name)?;
        // Lowering init can replace the entry block's current command owner.
        // The synthetic boundary still belongs to the original for command.
        if let Some(site) = condition_owner.as_ref() {
            self.command_boundary_sites
                .insert(init_tail.clone(), site.clone());
        }

        let header = self.new_block("for_header");
        let body_block = self.new_block("for_body");
        let step_block = self.new_block("for_step");
        let owner = self.command_boundary_sites.get(&init_tail).cloned();
        let end_block = self.new_block("for_end");

        self.ensure_goto(&init_tail, &header, Some(*init_span));
        self.retain_condition_binding(condition_owner.as_ref(), &header);

        // A `for` condition is re-evaluated every iteration exactly as a
        // `while` condition is, and until now contributed neither defs nor
        // reads — so `proc p {} {set k 0; for {set i 0} {[incr k] < 3} {} {puts $k}}`
        // had `set k 0` deleted as dead and the literal `0` forwarded into the
        // body: tclsh 9.0.4 prints `1` then `2`, the optimised program printed
        // `0` then `0` (#2132).
        if expr_has_command(condition) {
            self.push_condition_effects(condition, *condition_span, &header);
        }

        let body_id = self.bid(&body_block);
        let end_id = self.bid(&end_block);
        self.condition_owners.insert(
            *condition_span,
            crate::cfg::ConditionOwner {
                command: stmt.span(),
                skips_constant_dead_script: matches!(stmt, Statement::While { .. }),
            },
        );
        self.set_terminator(
            &header,
            Terminator::Branch {
                condition: condition.clone(),
                true_target: body_id,
                false_target: end_id,
                span: Some(*condition_span),
                condition_base: *condition_base,
            },
        );

        // `break` exits to `end_block`; `continue` runs the step at `step_block`.
        self.loop_stack
            .push((end_block.clone(), step_block.clone()));
        let body_tail = self.lower_body(owner.as_ref(), body, &body_block);
        self.loop_stack.pop();
        if let Some(tail) = body_tail {
            self.ensure_goto(&tail, &step_block, Some(*body_span));
        }

        // Placeholder for empty next clause.
        if next.statements.is_empty() {
            self.push_empty_clause(&step_block, *next_span);
        }
        // Analysis builds rotate a `for` whose condition is statically true on
        // entry: the step re-checks the condition
        // (back-edge) instead of looping to the header, and the header is demoted
        // to a synthetic always-true entry guard (span `None`, so the optimiser's
        // constant-branch source rewriter never folds the loop's source
        // condition). SCCP then prunes the zero-iteration header→end edge, and
        // the dead-edge phi filter ignores the version-0 operand it carried,
        // so a body-assigned variable read after the loop is no longer a false
        // read-before-set. `break`/`continue` stay real edges (partial-def exits
        // remain sound); `loop_nodes` + the init exit versions are unchanged, so
        // the solver enumerates the loop from the same state.
        let rotate = self.faithful_exceptions && self.for_runs_at_least_once(stmt);
        let step_tail = self.lower_body(owner.as_ref(), next, &step_block);
        if let Some(step_tail) = step_tail {
            if rotate {
                self.condition_binding_sites.remove(&header);
                self.retain_condition_binding(condition_owner.as_ref(), &step_tail);
                self.set_terminator(
                    &header,
                    Terminator::Branch {
                        condition: literal_true_expr(),
                        true_target: body_id,
                        false_target: end_id,
                        span: None,
                        condition_base: None,
                    },
                );
                self.set_terminator(
                    &step_tail,
                    Terminator::Branch {
                        condition: condition.clone(),
                        true_target: body_id,
                        false_target: end_id,
                        span: Some(*condition_span),
                        condition_base: *condition_base,
                    },
                );
            } else {
                self.ensure_goto(&step_tail, &header, Some(*next_span));
            }
        }

        self.retain_for_loop_node(stmt, block_name, &init_tail, &end_block);

        Some(end_block)
    }

    fn retain_for_loop_node(
        &mut self,
        stmt: &Statement,
        block_name: &str,
        init_tail: &str,
        end_block: &str,
    ) {
        let entry_block = self.bid(block_name);
        let start = self.bid(init_tail);
        self.loop_nodes.insert(
            end_block.to_owned(),
            LoopNode {
                executed_source: self.current_source.clone(),
                entry_block,
                start,
                span: stmt.span(),
                statement: stmt.clone(),
            },
        );
    }

    /// Record the loop `stmt`, which starts in `block_name` and leaves to
    /// `end_block`, for the solver's enumeration ([`LoopNode`]).
    fn record_loop(&mut self, stmt: &Statement, block_name: &str, end_block: &str) {
        let entry_block = self.bid(block_name);
        self.loop_nodes.insert(
            end_block.to_owned(),
            LoopNode {
                executed_source: self.current_source.clone(),
                entry_block,
                start: entry_block,
                span: stmt.span(),
                statement: stmt.clone(),
            },
        );
    }

    // while

    /// Flatten `Statement::While` into header → body → header loop.
    pub(in super::super) fn lower_while(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::While {
            condition,
            condition_span,
            condition_base,
            body,
            body_span,
            ..
        } = stmt
        else {
            unreachable!("lower_while called with non-While");
        };

        let header = self.new_block("while_header");
        let body_block = self.new_block("while_body");
        let end_block = self.new_block("while_end");
        let owner = self.command_boundary_sites.get(block_name).cloned();

        self.ensure_goto(block_name, &header, Some(*condition_span));
        self.copy_condition_binding(block_name, &header);

        // A `catch`/`regexp`/`scan` substitution — or a call to a known
        // upvar / global-writing user proc — in the loop condition writes
        // result variables each iteration; record
        // them as defs in the header so a read in the body is not flagged
        // read-before-set (W210).
        if expr_has_command(condition) {
            // As `lower_if`: the loop condition's substitutions write result
            // variables each iteration, and read the ones they
            // read-modify-write.
            self.push_condition_effects(condition, *condition_span, &header);
        }
        let body_id = self.bid(&body_block);
        let end_id = self.bid(&end_block);
        self.condition_owners.insert(
            *condition_span,
            crate::cfg::ConditionOwner {
                command: stmt.span(),
                skips_constant_dead_script: matches!(stmt, Statement::While { .. }),
            },
        );
        self.set_terminator(
            &header,
            Terminator::Branch {
                condition: condition.clone(),
                true_target: body_id,
                false_target: end_id,
                span: Some(*condition_span),
                condition_base: *condition_base,
            },
        );

        // `break` exits to `end_block`; `continue` re-tests at `header`.
        self.loop_stack.push((end_block.clone(), header.clone()));
        let body_tail = self.lower_body(owner.as_ref(), body, &body_block);
        self.loop_stack.pop();
        if let Some(tail) = body_tail {
            self.ensure_goto(&tail, &header, Some(*body_span));
        }

        self.record_loop(stmt, block_name, &end_block);
        end_block
    }

    // foreach / lmap

    fn push_iteration_arguments(
        &mut self,
        span: Span,
        raw_tokens: Option<&crate::ir::CommandTokens>,
        block_name: &str,
    ) {
        if self.faithful_exceptions
            && let Some(original) = raw_tokens
        {
            let mut arguments = original.clone();
            arguments.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments);
            self.push_statement(
                block_name,
                Statement::Call {
                    span,
                    command: original.argv_texts.first().cloned().unwrap_or_default(),
                    canonical_command: None,
                    args: original.argv_texts.iter().skip(1).cloned().collect(),
                    defs: Vec::new(),
                    reads: Vec::new(),
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: Some(arguments),
                    foreach_groups: None,
                },
            );
        }
    }

    /// Flatten `Statement::Foreach` into a header → body → header loop
    /// with a synthetic variable-definition node at the header.
    ///
    /// The body recursion is the only work in this frame; the loop scaffolding
    /// lives in leaves so a deep nest costs a pointer per level.
    pub(in super::super) fn lower_foreach(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::Foreach { body, .. } = stmt else {
            unreachable!("lower_foreach called with non-Foreach");
        };
        let frame = self.open_foreach_loop(stmt, block_name);
        self.loop_stack
            .push((frame.end_block.clone(), frame.continue_target().to_owned()));
        let body_tail = self.lower_body(frame.owner.as_ref(), body, &frame.body_block);
        self.loop_stack.pop();
        self.close_foreach_loop(stmt, block_name, frame, body_tail)
    }

    #[inline(never)]
    fn open_foreach_loop(&mut self, stmt: &Statement, block_name: &str) -> Box<ForeachLoopFrame> {
        let Statement::Foreach {
            span, raw_tokens, ..
        } = stmt
        else {
            unreachable!("open_foreach_loop called with non-Foreach");
        };

        let command_owner = self.command_boundary_sites.get(block_name).cloned();
        self.push_iteration_arguments(*span, raw_tokens.as_ref(), block_name);

        let header = self.new_block("foreach_header");
        let body_block = self.new_block("foreach_body");
        let end_block = self.new_block("foreach_end");
        // The iterator condition does not evaluate the original foreach argv.
        // Its input dependencies belong to the pre-loop argument boundary.
        // The runtime iterator-entry marker nevertheless replays that command.
        let owner = command_owner.clone();
        if let Some(site) = command_owner {
            self.command_boundary_sites.insert(header.clone(), site);
        }

        self.ensure_goto(block_name, &header, Some(*span));

        let var_def = foreach_binding_statement(stmt);
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&self.config);

        // Analysis builds rotate a provably-non-empty foreach so the
        // 0-iteration skip is a *separate*, statically-true entry-guard edge
        // (SCCP prunes it; the dead-edge phi filter then ignores the
        // version-0 operand it carried). The var-def + body run at least once
        // before the back-edge re-check, so a body-assigned variable (or a loop
        // variable) read after the loop is no longer a false read-before-set,
        // while SCCP values stay intact (no synthetic def). `break`/`continue`
        // stay real edges, so partial-def exits remain sound.
        let body_id = self.bid(&body_block);
        let end_id = self.bid(&end_block);
        if self.faithful_exceptions && crate::cfg_builder::foreach_runs_at_least_once(stmt, rules) {
            let latch_block = self.new_block("foreach_latch");
            // Entry guard: the list is a non-empty literal, so the body always
            // runs at least once. A statically-true condition SCCP folds, so the
            // entry→end (zero-iteration) edge is dead. `span = None` keeps the
            // optimiser's constant-branch source rewriter off this synthetic guard.
            self.set_terminator(
                &header,
                Terminator::Branch {
                    condition: literal_true_expr(),
                    true_target: body_id,
                    false_target: end_id,
                    span: None,
                    condition_base: None,
                },
            );
            // The iteration variables are (re)bound at the top of every body
            // execution, so a post-loop read of a loop variable also resolves.
            // `continue` re-checks via the latch; `break` exits the loop.
            self.push_statement(&body_block, var_def);
            return Box::new(ForeachLoopFrame {
                owner,
                header,
                body_block,
                end_block,
                latch: Some(latch_block),
            });
        }

        // Every iterator list is an empty literal: the body never runs and no
        // loop variable is written. A statically-false guard lets SCCP prune
        // the body, whose commands the source walk never reached.
        let never_runs =
            self.faithful_exceptions && crate::cfg_builder::foreach_never_runs(stmt, rules);
        if never_runs {
            self.push_statement(&body_block, var_def);
        } else {
            self.push_statement(&header, var_def);
        }

        self.set_terminator(
            &header,
            if never_runs {
                Terminator::Branch {
                    condition: literal_false_expr(),
                    true_target: body_id,
                    false_target: end_id,
                    span: None,
                    condition_base: None,
                }
            } else {
                // Opaque condition: non-deterministic branch.
                Terminator::Branch {
                    condition: foreach_condition(),
                    true_target: body_id,
                    false_target: end_id,
                    span: Some(*span),
                    condition_base: None,
                }
            },
        );

        // `break` exits to `end_block`; `continue` advances at `header`.
        Box::new(ForeachLoopFrame {
            owner,
            header,
            body_block,
            end_block,
            latch: None,
        })
    }

    #[inline(never)]
    fn close_foreach_loop(
        &mut self,
        stmt: &Statement,
        block_name: &str,
        frame: Box<ForeachLoopFrame>,
        body_tail: Option<String>,
    ) -> String {
        let Statement::Foreach {
            span, body_span, ..
        } = stmt
        else {
            unreachable!("close_foreach_loop called with non-Foreach");
        };
        if let Some(tail) = body_tail {
            self.ensure_goto(&tail, frame.continue_target(), Some(*body_span));
        }
        let ForeachLoopFrame {
            body_block,
            end_block,
            latch,
            ..
        } = *frame;
        if let Some(latch_block) = latch {
            // Back-edge re-check: another element → body, else → exit.
            let body_id = self.bid(&body_block);
            let end_id = self.bid(&end_block);
            self.set_terminator(
                &latch_block,
                Terminator::Branch {
                    condition: foreach_condition(),
                    true_target: body_id,
                    false_target: end_id,
                    span: Some(*span),
                    condition_base: None,
                },
            );
            return end_block;
        }
        self.record_loop(stmt, block_name, &end_block);
        end_block
    }
}

/// The blocks a `foreach` lowering opens before its body and closes after it.
struct ForeachLoopFrame {
    owner: Option<crate::ir::CommandBindingSite>,
    header: String,
    body_block: String,
    end_block: String,
    latch: Option<String>,
}

impl ForeachLoopFrame {
    pub(super) fn continue_target(&self) -> &str {
        self.latch.as_deref().unwrap_or(&self.header)
    }
}
