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

//! Flattening `switch`: the subject operand, the arm tests and the opaque form that keeps the original invocation.

use super::super::CfgBuilder;
use crate::cfg::Terminator;
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;
use crate::lowering::structured::parse_switch_options;
use crate::value_transfer::recorded_word_value;
use tcl_dialect::TclVersion;
use tcl_lexer::{LexerConfig, Span};
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::TargetSemantics;

/// The enclosing loop's escape targets for an opaque `switch` whose arms may
/// `break` / `continue`. Bundled to keep [`CfgBuilder::wire_opaque_switch_jumps`]
/// within the argument limit.
struct SwitchEscape<'a> {
    can_break: bool,
    can_continue: bool,
    break_target: &'a str,
    continue_target: &'a str,
}

/// A `switch` subject as an operand of the flattened exact-mode dispatch.
///
/// The subject is a *word*, never an expression: `TclCompileSwitchCmd`
/// (`tclCompCmds.c`) pushes it through `TclCompileTokens` — ordinary word
/// substitution — and compares the resulting string. `ExprNode::CompiledWord`
/// is the operand shape that says exactly that, carrying the word's value and
/// whether it was braced.
///
/// Not `ExprNode::String`, whose text is *source including delimiters* — a
/// contract a value cannot honour. A subject whose value merely looks like a
/// braced word would be read back as one and stripped, so `switch -- "{abc}"`
/// would match the arm `abc` rather than `{abc}`, and re-bracketing a
/// genuinely braced subject to escape that only makes the two spellings
/// collide on one text. Values and source need different shapes, which is what
/// this variant is.
///
/// Not `ExprNode::Raw` either: it lowers to `push` + `exprStk`, making the
/// subject an expression — `switch -- abc …` would evaluate `expr {abc}`,
/// which "works" only while `exprStk` returns an unparsable expression's own
/// source text, and mis-matches outright where the subject *is* parsable
/// (`switch -- 1+1 {2 …}` takes the `2` arm). A normalised variable reference
/// keeps the `Raw` form, whose codegen has dedicated scalar-load arms and so
/// never involves `exprStk`.
///
/// The analysis reads that `Raw` subject as the variable it names, and only
/// where the variable-name owner proves the text is exactly one reference
/// under the document's `${…}` close rule with none of `{`, `}` or `\` in
/// the name (`sccp::with_whole_variable_operands` over
/// `value_transfer::whole_variable_operand`): the dispatch then
/// decides per arm from the lattice, so a dead arm draws I231 and O107
/// removes its unreachable body. Any other `Raw` text stays undecided.
///
/// Neither form is rewritten as a branch: branch folding skips any `StrEq`
/// terminator as a switch dispatch
/// (`optimiser::branch_folding::is_switch_dispatch_cond`), and codegen's
/// `fold_const_branch` only folds a whole-condition literal, never a `Binary`.
/// So an unsubstituted subject word can never be compared as if it were its
/// own literal text.
fn switch_subject_operand(subject: &str, braced: bool, config: &LexerConfig) -> ExprNode {
    // A braced subject is a literal: its `$` and `[` are data. `ExprNode::String`
    // carries *source text including delimiters* — that is its documented
    // contract — so the braces go back on and `emit_expr_string` recognises the
    // word through the shared `whole_braced_word` owner and pushes its content
    // verbatim. Handing over the bare value instead makes it indistinguishable
    // from an unbraced word, so `switch -- {a[nosuchcmd]}` runs the command
    // where both oracles match the literal and take the default arm.
    //
    // Re-bracing is lossless here, and only here: a braced word's content is
    // brace-balanced or escaped by construction, so wrapping it always yields a
    // word the balance walk accepts. The same is not true of an arbitrary
    // value, which is why this is gated on the word really having been braced
    // rather than applied to anything that looks like it could be.
    // The whole-variable fast path is for a subject that *reads* a variable, so
    // it must not fire on a braced word whose value merely spells one:
    // `switch -- {${x}}` compares the literal text `${x}`, and taking `Raw`
    // there loaded `x` instead (or errored when it was unset).
    if !braced && is_whole_var_ref(subject) {
        return ExprNode::Raw {
            text: subject.to_owned(),
        };
    }
    word_operand(subject, braced, config)
}

/// A `switch` word, the subject or a pattern, as an operand of the flattened
/// dispatch: by its value where the statement states one
/// ([`recorded_word_value`], the decoder the selection reads its arguments
/// by), braced so nothing reads it a second time, and by its spelling where
/// the word substitutes, which the evaluators decline to fold. The recorded
/// text is a spelling, not a value: a bare or quoted `a\nb` is a letter, a
/// newline and a letter, which is what a braced arm list's element holding a
/// newline compares against. A word whose value is its spelling keeps the
/// operand it always had.
fn word_operand(text: &str, braced: bool, config: &LexerConfig) -> ExprNode {
    match recorded_word_value(text, braced, config) {
        Some(value) if braced || value != text => ExprNode::CompiledWord {
            text: value.into_owned(),
            braced: true,
        },
        _ => ExprNode::CompiledWord {
            text: text.to_owned(),
            braced,
        },
    }
}

/// Whether the subject of the exact `switch` `stmt` may be read as an option
/// by the release that runs it. Before 8.5 `switch` scans every leading word
/// that starts with `-`, however many words follow; from 8.5 the scan stops
/// with two words left, which leaves the subject outside it only where the
/// arms are one list word — with pattern and body words the subject is inside
/// it on every release. A subject whose value starts with `-` is then an
/// option — a mode, or an error — unless `--` ended the run. The registry's
/// selection reads the same rule (`tcl_registry::value_transfer::selection`)
/// and leaves such a subject to the runtime. The chain cannot state a
/// whole-variable subject's value, so that subject stays one opaque statement,
/// whose selection does; a literal one is decided by its decoded value. A
/// release the registry's profile does not declare is as if before 8.5, and a
/// registry with no profile reads no release at all.
pub(in super::super) fn subject_may_scan_as_option(
    stmt: &Statement,
    registry: &CommandRegistry,
    config: &LexerConfig,
) -> bool {
    let Some(profile) = registry.profile() else {
        return false;
    };
    let bounded = TargetSemantics::of(Some(profile))
        .release
        .is_some_and(|release| release >= TclVersion::V8_5);
    let Statement::Switch {
        subject,
        subject_braced,
        raw_args,
        patterns_braced,
        ..
    } = stmt
    else {
        return false;
    };
    let (.., ended) = parse_switch_options(raw_args);
    if (bounded && *patterns_braced) || ended {
        return false;
    }
    match recorded_word_value(subject, *subject_braced, config) {
        Some(value) => value.starts_with('-'),
        None => is_whole_var_ref(subject),
    }
}

/// Preserve braced references as Raw under either close rule so name escapes
/// are not substituted as text. Target codegen validates its actual grammar.
fn is_whole_var_ref(subject: &str) -> bool {
    [
        tcl_dialect::BracedVarStyle::Tcl9Nesting,
        tcl_dialect::BracedVarStyle::FirstClose,
    ]
    .into_iter()
    .any(|style| {
        tcl_syntax::naming::split_braced_var_ref(subject, style)
            .is_some_and(|(_, remainder)| remainder.is_empty())
    })
}

impl CfgBuilder<'_> {
    // switch

    /// Lower a `Statement::Switch`.
    ///
    /// Glob/regexp switches, and exact switches with any fall-through arm, are
    /// kept **opaque** (a single `Statement::Switch` in the block) — see the
    /// early return below; codegen emits a generic `switch` invoke and SSA
    /// recovers the reads via `ssa::uses_of`. An exact switch without
    /// fall-through is flattened into a chain of arm-dispatch branches on a
    /// foldable `STR_EQ(subject, pattern)` so the bytecode backend can build a
    /// real jump table.
    /// Append an opaque `switch` to `block_name` and model how it leaves.
    ///
    /// An opaque (glob/regexp/fall-through) switch is kept as a single
    /// `Statement::Switch` whose arm bodies are not lowered into the CFG, so its
    /// internal control flow is otherwise invisible.  In analysis builds we
    /// recover the ways it can leave the block:
    ///
    /// * every arm exits the *procedure* → promote the block to `Return` (so the
    ///   following statements are unreachable, like a `return`);
    /// * an arm `break`s/`continue`s to an enclosing loop → wire explicit edges
    ///   from this block to that loop's break / continue target, so a loop whose
    ///   only exit is such a jump is not seen as infinite and the post-loop read
    ///   is correctly reachable (and maybe-unset);
    /// * otherwise it just falls through to the next statement.
    ///
    /// Codegen builds (`faithful_exceptions` off) leave the plain fall-through so
    /// the opaque-switch `invokeStk` bytecode / CFG shape are unchanged.  Returns
    /// the block subsequent statements continue in.
    fn lower_opaque_switch(&mut self, stmt: &Statement, block_name: &str) -> String {
        use crate::cfg_builder::Completion;
        self.push_statement(block_name, stmt.clone());
        // A command an arm runs is inside the statement, so the scans of the
        // statements the graph lowers never reach it.
        for effect in self.in_barrier_block(block_name, |builder| builder.opaque_arm_effects(stmt))
        {
            self.block_mut(block_name).statements.push(effect);
        }
        if !self.faithful_exceptions {
            return block_name.to_owned();
        }
        let completion =
            crate::cfg_builder::flow_facts_stmt_with_classes(stmt, &self.command_classes).1;
        // An arm's `break`/`continue` targets the enclosing loop, even when no
        // arm completes normally: such a switch is a loop jump, not a
        // procedure exit (tclsh: `while 1 {switch a {a {break}}}` returns).
        let loop_escape = self.loop_stack.last().cloned().and_then(|targets| {
            let (can_break, can_continue) = crate::cfg_builder::switch_escaping_jumps_with_classes(
                stmt,
                0,
                &self.command_classes,
            );
            (can_break || can_continue).then_some((targets, can_break, can_continue))
        });
        if let Some(((break_target, continue_target), can_break, can_continue)) = loop_escape {
            let escape = SwitchEscape {
                can_break,
                can_continue,
                break_target: &break_target,
                continue_target: &continue_target,
            };
            return self.wire_opaque_switch_jumps(stmt, block_name, completion, &escape);
        }
        if completion == Completion::ProcExit {
            if self.block_mut(block_name).terminator.is_none() {
                self.set_terminator(
                    block_name,
                    Terminator::Complete {
                        // The branch summary proves no normal continuation,
                        // but may combine return, error and process exit.
                        // Retain that unresolved code instead of inventing
                        // a procedure return for the original invocation.
                        route: tcl_registry::completion_route::InvocationCompletionRoute::Unknown,
                        span: Some(stmt.span()),
                    },
                );
            }
            return block_name.to_owned();
        }
        block_name.to_owned()
    }

    /// Wire an opaque switch block to its enclosing loop's jump targets.
    ///
    /// The switch can leave via the fall-through successor (only when it can
    /// still complete normally), the loop's break target, and/or the loop's
    /// continue target.  Build a non-deterministic dispatch over those targets
    /// (we can't tell which arm runs), and return the block subsequent
    /// statements continue in: the fall-through continuation when one exists,
    /// else an unreachable orphan (every path jumped, so following code is dead).
    fn wire_opaque_switch_jumps(
        &mut self,
        stmt: &Statement,
        block_name: &str,
        completion: crate::cfg_builder::Completion,
        escape: &SwitchEscape<'_>,
    ) -> String {
        use crate::cfg_builder::Completion;
        let mut targets: Vec<String> = Vec::new();
        let mut continuation: Option<String> = None;
        if completion == Completion::Normal {
            let cont = self.new_block("switch_cont");
            continuation = Some(cont.clone());
            targets.push(cont);
        }
        if escape.can_break {
            targets.push(escape.break_target.to_owned());
        }
        if escape.can_continue {
            targets.push(escape.continue_target.to_owned());
        }
        self.branch_to_any(block_name, &targets, Some(stmt.span()));
        continuation.unwrap_or_else(|| self.new_block("switch_jump_dead"))
    }

    /// Terminate `block_name` so control can reach any of `targets`.
    ///
    /// One target → a `Goto`; more → a chain of opaque `Branch`es through
    /// synthetic dispatch blocks (the choice is non-deterministic — an opaque
    /// switch hides which arm runs).
    fn branch_to_any(&mut self, block_name: &str, targets: &[String], span: Option<Span>) {
        if targets.is_empty() {
            return;
        }
        if targets.len() == 1 {
            let target = self.bid(&targets[0]);
            self.set_terminator(block_name, Terminator::Goto { target, span });
            return;
        }
        let opaque = ExprNode::Raw {
            text: "<switch_jump>".into(),
        };
        let mut current = block_name.to_owned();
        for i in 0..targets.len() - 1 {
            let false_target = if i == targets.len() - 2 {
                targets[targets.len() - 1].clone()
            } else {
                self.new_block("switch_jump")
            };
            let true_id = self.bid(&targets[i]);
            let false_id = self.bid(&false_target);
            self.set_terminator(
                &current,
                Terminator::Branch {
                    condition: opaque.clone(),
                    true_target: true_id,
                    false_target: false_id,
                    span,
                    condition_base: None,
                },
            );
            current = false_target;
        }
    }

    fn emit_original_switch_invocation(&mut self, span: tcl_lexer::Span, block_name: &str) -> bool {
        let original_tokens = (!self.faithful_exceptions && !self.plain_command_dispatch)
            .then(|| {
                self.command_binding_sites
                    .iter()
                    .rev()
                    .find(|site| site.span == span)?
                    .source_tokens
                    .as_deref()
                    .filter(|tokens| {
                        tokens.source_binding.as_ref().is_some_and(|binding| {
                            binding.original_switch_compilation(tokens).is_some()
                        })
                    })
                    .cloned()
            })
            .flatten();
        if let Some(tokens) = original_tokens {
            self.push_statement(
                block_name,
                Statement::Call {
                    span,
                    command: tokens.argv_texts[0].clone(),
                    canonical_command: None,
                    args: tokens.argv_texts[1..].to_vec(),
                    defs: vec![],
                    reads: vec![],
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: Some(tokens),
                    foreach_groups: None,
                },
            );
            return true;
        }

        false
    }

    pub(in super::super) fn lower_switch(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::Switch {
            span,
            subject,
            subject_braced,
            arms,
            default_body,
            default_span,
            ..
        } = stmt
        else {
            unreachable!("lower_switch called with non-Switch");
        };

        // Native switch preparation owns body visits, including duplicate
        // arms which are never compiled. Preserve the exact original command
        // for executable emission; the analysis CFG retains its branch facts.
        if self.emit_original_switch_invocation(*span, block_name) {
            return block_name.to_owned();
        }

        // All unbraced subject and pattern words substitute before `switch`
        // dispatches. Preserve those writes and opaque effects even when the
        // switch itself is flattened into branch terminators.
        self.push_embedded_control_effects(
            stmt,
            block_name,
            "switch word invokes an opaque embedded command",
        );

        // Glob/regexp switches, and exact switches with any fall-through arm,
        // are kept *opaque* — a single `Statement::Switch` in the current block,
        // no expanded arm blocks. Their shared-body / OR-matching topology can't
        // be expressed as structured control flow, and tclsh 9.0 invokes them
        // generically rather than compiling a jump table; codegen emits a
        // generic `switch` invoke for the opaque statement. SSA reads of the
        // subject + arm/default bodies are recovered by `ssa::uses_of`'s
        // `Statement::Switch` arm, and what they write by
        // `ssa::switch_may_defs` and the statements `opaque_arm_effects` puts
        // after the switch.
        // A `-nocase` exact switch must also stay opaque: the flattened form
        // builds a `STR_EQ`/JUMP_TABLE dispatch that is case-sensitive, so the
        // case-insensitive match has to run through the generic `switch`
        // command (the VM/runtime `cmd_switch`). So must one whose subject a
        // release's option scan may read (`switch_is_flattened`).
        if !self.flattens_switch(stmt) {
            return self.lower_opaque_switch(stmt, block_name);
        }

        let end_block = self.new_block("switch_end");
        self.branch_command_joins
            .insert(block_name.to_owned(), end_block.clone());
        let default_block = self.new_block("switch_default");

        let body_name = "switch_arm_body";
        let body_targets: Vec<String> = arms.iter().map(|_| self.new_block(body_name)).collect();
        let owner = self.command_boundary_sites.get(block_name).cloned();

        // Resolve fallthrough: fallthrough arms jump to the next
        // non-fallthrough body, or to default.
        let final_targets: Vec<String> = arms
            .iter()
            .enumerate()
            .map(|(i, arm)| {
                if arm.fallthrough {
                    let mut j = i + 1;
                    while j < arms.len() {
                        if !arms[j].fallthrough {
                            return body_targets[j].clone();
                        }
                        j += 1;
                    }
                    default_block.clone()
                } else {
                    body_targets[i].clone()
                }
            })
            .collect();

        // Chain of arm-dispatch tests.
        let mut dispatch = block_name.to_owned();
        for (i, arm) in arms.iter().enumerate() {
            let next_dispatch = self.new_block("switch_next");
            // Only exact mode reaches here — every other mode left through
            // `lower_opaque_switch` above — and it dispatches on a foldable
            // `STR_EQ(subject, pattern)` so the backend can build a jump
            // table.
            let cond = ExprNode::Binary {
                op: BinOp::StrEq,
                left: Box::new(switch_subject_operand(
                    subject,
                    *subject_braced,
                    &self.config,
                )),
                // The pattern is a *word value* too — the arm list's decoded
                // element — so it takes the same operand shape as the subject.
                // A `Literal` slot would read its text back as expression
                // source, costing a pattern that looks braced a layer: `{7}`
                // compares as `7`.
                //
                // Per arm, not per switch: a single braced arm list holds
                // literal elements, but the multi-word form is a word each,
                // where `{${x}}` is literal and a bare `$pat` substitutes.
                right: Box::new(word_operand(&arm.pattern, arm.pattern_braced, &self.config)),
            };
            let true_id = self.bid(&final_targets[i]);
            let false_id = self.bid(&next_dispatch);
            self.set_terminator(
                &dispatch,
                Terminator::Branch {
                    condition: cond,
                    true_target: true_id,
                    false_target: false_id,
                    span: arm.pattern_span.into(),
                    condition_base: None,
                },
            );
            dispatch = next_dispatch;
        }

        // Last dispatch falls through to default.
        self.ensure_goto(&dispatch, &default_block, default_span.or(Some(*span)));

        // Lower arm bodies.
        for (i, arm) in arms.iter().enumerate() {
            if arm.fallthrough || arm.body.is_none() {
                self.ensure_goto(
                    &body_targets[i],
                    &final_targets[i],
                    arm.body_span.or(Some(arm.pattern_span)),
                );
                continue;
            }
            if let Some(ref body) = arm.body
                && let Some(tail) = self.lower_body(owner.as_ref(), body, &body_targets[i])
            {
                self.ensure_goto(&tail, &end_block, arm.body_span);
            }
        }

        // Default body.
        if let Some(db) = default_body {
            if let Some(tail) = self.lower_body(owner.as_ref(), db, &default_block) {
                self.ensure_goto(&tail, &end_block, *default_span);
            }
        } else {
            self.ensure_goto(&default_block, &end_block, Some(*span));
        }

        end_block
    }
}
