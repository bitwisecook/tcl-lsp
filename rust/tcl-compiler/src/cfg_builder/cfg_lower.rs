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

//! Per-construct CFG lowering methods.
//!
//! Each method flattens one structured IR construct (`If`, `For`,
//! `While`, `Foreach`, `Switch`, `Catch`, `Try`) into basic blocks
//! with terminators, returning the name of the "continuation" block
//! (or `None` if control doesn't fall through).

use tcl_lexer::{LexerConfig, Span};

use crate::cfg::{LoopNode, Terminator};
use crate::expr_ast::{BinOp, ExprNode};
use crate::ir::Statement;
use crate::ir_helpers::expr_has_command;
use crate::lowering::structured::parse_switch_options;
use crate::value_transfer::recorded_word_value;
use tcl_dialect::TclVersion;
use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::TargetSemantics;
use tcl_registry::value_transfer::completion::{HandlerChain, HandlerLink};

use super::CfgBuilder;

/// The enclosing loop's escape targets for an opaque `switch` whose arms may
/// `break` / `continue`. Bundled to keep [`CfgBuilder::wire_opaque_switch_jumps`]
/// within the argument limit.
struct SwitchEscape<'a> {
    can_break: bool,
    can_continue: bool,
    break_target: &'a str,
    continue_target: &'a str,
}

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
pub(super) fn subject_may_scan_as_option(
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

/// Whether `subject` is one whole `${…}` / `$name` variable reference under
/// **either** release's close rule.
///
/// # This gate must accept under either rule — do not "simplify" it
///
/// The `${…}` close rule is release-dependent and CFG lowering
/// has no dialect in hand: it runs before the target release reaches codegen.
/// Every narrower gate is wrong, and each has its own distinguishing program.
///
/// Declining is not free. It sends the subject down the `String` path, where
/// the whole word is re-substituted as ordinary text — and for a name carrying
/// a backslash that is *not* the same operation as loading the variable, because
/// the escape is processed instead of staying literal. So `Raw` is not merely an
/// optimisation over `String`; it is the only arm that preserves the name. That
/// makes an accepting gate the safe direction and abstention the risky one,
/// which is the opposite of the usual intuition about optimisation gates.
///
/// **Pinning to [`BracedVarStyle::Tcl9Nesting`]** (the `default()`) loses a
/// subject that is whole under the 8.x rule only:
///
/// ```tcl
/// set "a\\" K
/// switch -- ${a\} { K {puts hit} default {puts miss} }
/// ```
///
/// `${a\}` closes at the first `}` under `FirstClose`, naming `a\`; under
/// `Tcl9Nesting` the `\}` is inert so the name never closes. Real tclsh 8.6
/// prints `hit`. Pinned to the 9.x rule this gate declines, the subject is
/// re-substituted as text, the `\}` is processed, and the load becomes
/// `can't read "a"` — an error, not a wrong branch. (At 9.x the same spelling
/// is unterminated, so the script is a parse error and never reaches the gate.)
///
/// **Pinning to [`BracedVarStyle::FirstClose`], or requiring both rules to
/// agree** (an earlier revision of this fix did the latter, believing
/// abstention was safe) loses the mirror-image subject:
///
/// ```tcl
/// set {a\}b} K
/// switch -- ${a\}b} { K {puts hit} default {puts miss} }
/// ```
///
/// Real tclsh: `can't read "a\"` at 8.6, `hit` at 9.0. Either narrowing yields
/// `can't read "a"` at **both** releases — wrong twice over.
///
/// Accepting under either rule is what covers both, and it is sound because the
/// gate only chooses a *representation*: dialect-aware codegen re-decides the
/// reference under the real target style, and a spelling that is whole under
/// only one rule is a parse error under the other, so it can never reach
/// codegen wearing the wrong release's answer. Pinned by
/// `compiled_interpolated_and_switch_paths_follow_the_emulated_release` and
/// `switch_subject_whole_under_the_8x_rule_only_follows_the_emulated_release`.
fn is_whole_var_ref(subject: &str) -> bool {
    [
        tcl_dialect::BracedVarStyle::Tcl9Nesting,
        tcl_dialect::BracedVarStyle::FirstClose,
    ]
    .into_iter()
    .any(|style| crate::codegen::values::parse_simple_var_ref(subject, style).is_some())
}

impl CfgBuilder<'_> {
    // if

    /// Flatten `Statement::If` into cascaded branch blocks.
    pub(super) fn lower_if(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::If {
            span,
            clauses,
            else_body,
            else_span,
            ..
        } = stmt
        else {
            unreachable!("lower_if called with non-If");
        };

        let end_block = self.new_block("if_end");
        let mut dispatch = block_name.to_owned();

        for clause in clauses {
            // When the branch condition contains a command substitution,
            // append a synthetic `<cond>` Statement::Call so the emitter
            // can wrap the ExprNode::Command with its own startCommand
            // boundary.
            if expr_has_command(&clause.condition) {
                // A `catch`/`regexp`/`scan` substitution — or a call to a
                // known upvar / global-writing user proc — in the condition
                // writes result variables; record them as defs so a read in
                // the guarded body is not flagged read-before-set (W210), and
                // its reads so the store feeding an `[incr n]` there is not
                // taken for a dead one.
                self.push_condition_effects(&clause.condition, *span, &dispatch);
            }
            let then_block = self.new_block("if_then");
            let next_dispatch = self.new_block("if_next");
            self.copy_command_boundary(block_name, &dispatch);

            let true_target = self.bid(&then_block);
            let false_target = self.bid(&next_dispatch);
            self.set_terminator(
                &dispatch,
                Terminator::Branch {
                    condition: clause.condition.clone(),
                    true_target,
                    false_target,
                    span: Some(clause.condition_span),
                    condition_base: clause.condition_base,
                },
            );

            if let Some(tail) = self.lower_script(&clause.body, &then_block) {
                self.ensure_goto(&tail, &end_block, Some(clause.body_span));
            }

            dispatch = next_dispatch;
        }

        if let Some(eb) = else_body {
            if let Some(tail) = self.lower_script(eb, &dispatch) {
                self.ensure_goto(&tail, &end_block, *else_span);
            }
        } else {
            self.ensure_goto(&dispatch, &end_block, Some(*span));
        }

        end_block
    }

    // for

    /// Flatten `Statement::For` into init → header → body → step → header loop.
    /// Push the placeholder an empty `for` init / next clause takes, so the
    /// clause keeps its place in the instruction stream (codegen emits the
    /// three `nop`s tclsh's bytecode has there).
    ///
    /// The statement is a *synthetic marker*: the `command` spelling is only a
    /// label for the disassembly, and codegen recognises it by the typed
    /// [`crate::ir::SyntheticMarker`] on its tokens — `<empty_clause>` is a
    /// legal Tcl command name a script may define and call.
    fn push_empty_clause(&mut self, block: &str, span: Span) {
        self.push_statement(
            block,
            Statement::Call {
                span,
                command: "<empty_clause>".into(),
                canonical_command: None,
                args: vec![],
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::EmptyClause,
                )),
                foreach_groups: None,
            },
        );
    }

    pub(super) fn lower_for(&mut self, stmt: &Statement, block_name: &str) -> Option<String> {
        let Statement::For {
            span,
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
        let body_tail = self.lower_script(body, &body_block);
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
        let step_tail = self.lower_script(next, &step_block);
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

        let entry_block = self.bid(block_name);
        let start = self.bid(&init_tail);
        self.loop_nodes.insert(
            end_block.clone(),
            LoopNode {
                executed_source: self.current_source.clone(),
                entry_block,
                start,
                span: *span,
                statement: stmt.clone(),
            },
        );

        Some(end_block)
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
    pub(super) fn lower_while(&mut self, stmt: &Statement, block_name: &str) -> String {
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
        let body_tail = self.lower_script(body, &body_block);
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
    pub(super) fn lower_foreach(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::Foreach {
            span,
            body,
            body_span,
            raw_tokens,
            ..
        } = stmt
        else {
            unreachable!("lower_foreach called with non-Foreach");
        };

        let command_owner = self.command_boundary_sites.get(block_name).cloned();
        self.push_iteration_arguments(*span, raw_tokens.as_ref(), block_name);

        let header = self.new_block("foreach_header");
        let body_block = self.new_block("foreach_body");
        let end_block = self.new_block("foreach_end");
        // The iterator condition does not evaluate the original foreach argv.
        // Its input dependencies belong to the pre-loop argument boundary.
        // The runtime iterator-entry marker nevertheless replays that command.
        if let Some(site) = command_owner {
            self.command_boundary_sites.insert(header.clone(), site);
        }

        self.ensure_goto(block_name, &header, Some(*span));

        let var_def = foreach_binding_statement(stmt);

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
        if self.faithful_exceptions
            && crate::cfg_builder::foreach_runs_at_least_once(
                stmt,
                tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
            )
        {
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
            self.push_statement(&body_block, var_def);
            // `continue` re-checks via the latch; `break` exits the loop.
            self.loop_stack
                .push((end_block.clone(), latch_block.clone()));
            let body_tail = self.lower_script(body, &body_block);
            self.loop_stack.pop();
            if let Some(tail) = body_tail {
                self.ensure_goto(&tail, &latch_block, Some(*body_span));
            }
            // Back-edge re-check: another element → body, else → exit.
            self.set_terminator(
                &latch_block,
                Terminator::Branch {
                    condition: ExprNode::Raw {
                        text: "<foreach_has_next>".into(),
                    },
                    true_target: body_id,
                    false_target: end_id,
                    span: Some(*span),
                    condition_base: None,
                },
            );
            return end_block;
        }

        self.push_statement(&header, var_def);

        // Opaque condition: non-deterministic branch.
        self.set_terminator(
            &header,
            Terminator::Branch {
                condition: ExprNode::Raw {
                    text: "<foreach_has_next>".into(),
                },
                true_target: body_id,
                false_target: end_id,
                span: Some(*span),
                condition_base: None,
            },
        );

        // `break` exits to `end_block`; `continue` advances at `header`.
        self.loop_stack.push((end_block.clone(), header.clone()));
        let body_tail = self.lower_script(body, &body_block);
        self.loop_stack.pop();
        if let Some(tail) = body_tail {
            self.ensure_goto(&tail, &header, Some(*body_span));
        }

        self.record_loop(stmt, block_name, &end_block);
        end_block
    }

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
        for effect in self.opaque_arm_effects(stmt) {
            self.block_mut(block_name).statements.push(effect);
        }
        if !self.faithful_exceptions {
            return block_name.to_owned();
        }
        let completion =
            crate::cfg_builder::flow_facts_stmt_with_classes(stmt, &self.command_classes).1;
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
        if let Some((break_target, continue_target)) = self.loop_stack.last().cloned() {
            let (can_break, can_continue) = crate::cfg_builder::switch_escaping_jumps_with_classes(
                stmt,
                0,
                &self.command_classes,
            );
            if can_break || can_continue {
                let escape = SwitchEscape {
                    can_break,
                    can_continue,
                    break_target: &break_target,
                    continue_target: &continue_target,
                };
                return self.wire_opaque_switch_jumps(stmt, block_name, completion, &escape);
            }
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

    pub(super) fn lower_switch(&mut self, stmt: &Statement, block_name: &str) -> String {
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
        if !super::switch_is_flattened(stmt, self.registry, &self.config) {
            return self.lower_opaque_switch(stmt, block_name);
        }

        let end_block = self.new_block("switch_end");
        let default_block = self.new_block("switch_default");

        let body_name = "switch_arm_body";
        let body_targets: Vec<String> = arms.iter().map(|_| self.new_block(body_name)).collect();

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
                && let Some(tail) = self.lower_script(body, &body_targets[i])
            {
                self.ensure_goto(&tail, &end_block, arm.body_span);
            }
        }

        // Default body.
        if let Some(db) = default_body {
            if let Some(tail) = self.lower_script(db, &default_block) {
                self.ensure_goto(&tail, &end_block, *default_span);
            }
        } else {
            self.ensure_goto(&default_block, &end_block, Some(*span));
        }

        end_block
    }

    // try

    /// Record the analysis-only exception edges into a `try` handler block:
    ///
    /// * `on ok` runs only after the body completes normally → source = body
    ///   tail (the body's exit versions).
    /// * a body with *no* normal fall-through (it unconditionally throws /
    ///   returns) reaches the handler only from its explicit throw points (at
    ///   their throw-time versions), falling back to the terminal block when the
    ///   body terminated without an explicit `error`/`throw`.
    /// * a body that *can* fall through may complete abnormally at any point, so
    ///   a body-set var is only *maybe* defined → merge the pre-`try` state
    ///   (version-0) with the body-exit state.
    fn push_try_handler_exception_edges(
        &mut self,
        (chain, group): (&HandlerChain, &[usize]),
        (handler_block, block_name, body_block): (&str, &str, &str),
        body_tail: Option<&str>,
        (body_throw_blocks, split_points): (&[String], &[super::SplitPoint]),
        (body_terminal, first): (Option<&str>, Option<&Statement>),
    ) {
        if !self.faithful_exceptions || group.is_empty() {
            return;
        }
        // `group` is every handler whose match runs this block's body: a `-`
        // handler's own block is empty, so the body it shares is reached only
        // through the edges of the handler that owns it. It runs after the
        // body completes normally alone when every member selects `ok` — the
        // registry's completion-code parse, so `0` is `ok` too. `try` declares
        // no default clause: `ok` is a value of the pattern word, not "no
        // handler matched".
        let is_on_ok = group
            .iter()
            .all(|&member| chain.takes(member, tcl_core_types::Code::Ok));
        if is_on_ok {
            if let Some(tail) = body_tail {
                self.exception_edges
                    .push((tail.to_owned(), handler_block.to_owned()));
            }
        } else if body_tail.is_none() {
            let mut throw_sources: Vec<String> = Vec::new();
            for tb in body_throw_blocks {
                if !throw_sources.contains(tb) {
                    throw_sources.push(tb.clone());
                }
            }
            if throw_sources.is_empty()
                && let Some(terminal) = body_terminal
            {
                throw_sources.push(terminal.to_owned());
            }
            // A source whose completion the registry knows exactly reaches
            // only a handler that selects that code. Wiring
            // `try {break} on error {} {}`, `on continue`, or
            // `try {return early} on error {} {}` to the handler made the
            // `try` look as if it could complete normally (found in review).
            // A selector or completion the registry cannot decode keeps the
            // edge, and so does a match by any handler of a `-` group:
            // `try {error boom} on error {} - on ok {} {set x 1}` runs
            // `set x 1`, which tclsh 8.6.18 and 9.0.4 confirm (found in
            // review).
            throw_sources.retain(|src| {
                group
                    .iter()
                    .any(|&member| !self.handler_misses_completion(chain, member, src))
            });
            for src in throw_sources {
                self.exception_edges.push((src, handler_block.to_owned()));
            }
            // A body that never rests still starts somewhere: a first command
            // other than a literal assignment may fail before it stores, and
            // the handler then sees the state before the body, over the region
            // entry the solver opens there, as for a body that rests below —
            // unless the body's first block completes with a code the registry
            // knows exactly and no member takes, as the throw sources above
            // are filtered. A literal assignment needs no entry: it raises
            // only where its own place holds an array, whose scalar value
            // nothing reads, and leaves every other place as the point after
            // it does. Without the entry the handler took what the first block
            // left: `try {lassign {x y} a b; error boom} on error {} {}` gave it
            // `b` as `lassign` wrote it where `a` may be an array, and `set b
            // old` before it went as dead.
            let entry = (block_name.to_owned(), handler_block.to_owned());
            let first_may_fail = first.is_some_and(|first| {
                !self.leaves_from_the_state_before(first)
                    && super::NextFailure::of(first) == super::NextFailure::Any
            });
            let reached = group
                .iter()
                .any(|&member| !self.handler_misses_completion(chain, member, body_block));
            if first_may_fail && reached && !self.exception_edges.contains(&entry) {
                self.exception_edges.push(entry);
                self.region_entries.push((
                    block_name.to_owned(),
                    handler_block.to_owned(),
                    body_block.to_owned(),
                ));
            }
            self.push_split_failure_edges(handler_block, split_points, |code| {
                group.iter().any(|&member| !chain.misses(member, code))
            });
        } else {
            self.exception_edges
                .push((block_name.to_owned(), handler_block.to_owned()));
            self.region_entries.push((
                block_name.to_owned(),
                handler_block.to_owned(),
                body_block.to_owned(),
            ));
            if let Some(tail) = body_tail
                && tail != block_name
            {
                self.exception_edges
                    .push((tail.to_owned(), handler_block.to_owned()));
            }
            // A resting tail does not mean the explicit throws went unseen:
            // one inside a nested `if`, or a `finally` that only ever resumes
            // unwinding, raises with its own block's defs live. Sourcing only
            // the pre-`try` block and the tail read `x` as unset after
            // `try { if {$c} {set x 1; error b} else {set x 2; error c} }
            // on error {} {}` and called its stores dead.
            let mut seen: Vec<&String> = Vec::new();
            for tb in body_throw_blocks {
                if tb != block_name && body_tail != Some(tb.as_str()) && !seen.contains(&tb) {
                    seen.push(tb);
                    self.exception_edges
                        .push((tb.clone(), handler_block.to_owned()));
                }
            }
            self.push_split_failure_edges(handler_block, split_points, |code| {
                group.iter().any(|&member| !chain.misses(member, code))
            });
        }
    }

    /// Record an exception edge into `target` from each point a throw may
    /// leave the body from for it, where the edge is not there already. The
    /// state at the point is the one a statement before it completes with when
    /// it leaves abnormally after its own stores, with any code, and the one
    /// the next statement fails with when it fails before it stores — an error
    /// alone from a literal assignment, for which `takes` says whether the
    /// target may take it. A codegen build records none.
    fn push_split_failure_edges(
        &mut self,
        target: &str,
        points: &[super::SplitPoint],
        takes: impl Fn(tcl_core_types::Code) -> bool,
    ) {
        if !self.faithful_exceptions {
            return;
        }
        for point in points {
            if !point.after_stores && !point.next_fails_for(&takes) {
                continue;
            }
            let edge = (point.block.clone(), target.to_owned());
            if !self.exception_edges.contains(&edge) {
                self.exception_edges.push(edge);
            }
        }
    }

    /// Record the ways a failure of the body reaches the `finally` clause when
    /// no handler takes it: from each point a throw may leave the body from,
    /// and from the block before the body, an edge that opens where the body's
    /// first command can fail before it stores (a region entry). A command
    /// may fail with any code and no handler takes every code, so a handler
    /// stands in for a point only where the failure is an error a handler
    /// certainly takes ([`HandlerChain::first_taking`]). A first statement
    /// that completes with its code from the state before it — `error boom`,
    /// `exit 7` — leaves by its own block, which the clause is wired from
    /// already, and one that runs a clause of its own first leaves through it.
    /// Without these edges the clause ran over the state the body ends in
    /// alone: `try {set x 2; foo; set x 3} finally {puts $x}` printed `3`
    /// once optimised where tclsh prints `2` when `foo` raises.
    fn push_finally_failure_edges(
        &mut self,
        (end_block, chain): (&str, &HandlerChain),
        (block_name, body_block, first): (&str, &str, Option<&Statement>),
        points: &[super::SplitPoint],
    ) {
        if !self.faithful_exceptions {
            return;
        }
        let escapes = |code| chain.first_taking(code).is_none();
        let entry = (block_name.to_owned(), end_block.to_owned());
        let first_fails = first.is_some_and(|first| {
            !self.leaves_from_the_state_before(first)
                && match super::NextFailure::of(first) {
                    super::NextFailure::Intercepted => false,
                    super::NextFailure::Error => escapes(tcl_core_types::Code::Error),
                    super::NextFailure::Any => true,
                }
        });
        if first_fails && !self.exception_edges.contains(&entry) {
            self.exception_edges.push(entry);
            self.region_entries.push((
                block_name.to_owned(),
                end_block.to_owned(),
                body_block.to_owned(),
            ));
        }
        self.push_split_failure_edges(end_block, points, escapes);
    }

    /// Hand the ways the body of a `try` with no `finally` can leave to the
    /// region around it: a completion none of its handlers takes leaves the
    /// `try` from where the body left, so each of the body's points is a way
    /// out of the enclosing body too. A `try` with a `finally` runs it first,
    /// and the clause's own blocks are the enclosing region's.
    fn escape_to_the_region_around(
        &mut self,
        points: &[super::SplitPoint],
        throw_blocks: &[String],
        ends: [Option<&str>; 2],
    ) {
        if !self.split_region.is_some_and(super::SplitRegion::nested) {
            return;
        }
        let ways_out = throw_blocks
            .iter()
            .map(String::as_str)
            .chain(ends.into_iter().flatten())
            .map(|block| super::SplitPoint::any(block.to_owned()));
        for point in points.iter().cloned().chain(ways_out) {
            if !self
                .split_points
                .iter()
                .any(|known| known.block == point.block)
            {
                self.split_points.push(point);
            }
        }
    }

    /// The exact completion code with which *every* path through `block`
    /// leaves, when it is known: the [`terminal code`](Self::terminal_code)
    /// of a block with nothing else in it. An earlier statement may complete
    /// first — `set y $x; return ok` raises when `x` is unset, which an
    /// `on error` handler catches — so a block with one proves no single code
    /// (found in review).
    ///
    /// Nor may anything before the block: `try {if {$c} {}; return ok}` puts
    /// the `return` alone in `if_end`, but `$c` may raise first, and a failure
    /// in an earlier block has no edge of its own — the terminal block carries
    /// it (found in review). So only `entry`, the construct's own first block,
    /// qualifies — with the steps an analysis build splits the construct's
    /// first statements into after it, whose statements are read as one run
    /// ([`Self::entry_run_statements`]).
    fn block_completion_code(&self, block: &str, entry: &str) -> Option<tcl_core_types::Code> {
        let statements = self.entry_run_statements(block, entry)?;
        if self.plain_return_blocks.contains(block) {
            return statements
                .is_empty()
                .then(|| self.terminal_code(block))
                .flatten();
        }
        let code = self.terminal_code(block)?;
        let [before @ .., _] = statements.as_slice() else {
            return None;
        };
        // A literal assignment completes normally or raises `TCL_ERROR` (the
        // name is an array, a write trace fails) — never another code — so it
        // cannot change an error's code: `set z 0; error boom` is caught by
        // `on error` whichever raises (found in review). A command
        // substitution in an unbraced name could complete with any code; a
        // braced name (`set {[} 0`) substitutes nothing.
        let only_errors_before = before.iter().all(|stmt| {
            matches!(stmt, Statement::AssignConst { name, name_braced, .. }
                if *name_braced || !name.contains('['))
        });
        (before.is_empty() || (code == tcl_core_types::Code::Error && only_errors_before))
            .then_some(code)
    }

    /// The statements of `block` and of the blocks before it on the straight
    /// run of split steps that begins at `entry`, in order: the construct's
    /// first statements, which an analysis build gives a block each. `None`
    /// when `block` is not on that run — a compound statement ends it, so in
    /// `if {$c} {}; return ok` the `return` is past a block `$c` may raise in.
    fn entry_run_statements(&self, block: &str, entry: &str) -> Option<Vec<&Statement>> {
        let mut run = vec![block];
        let mut at = block;
        while at != entry {
            at = self.split_parent.get(at)?;
            run.push(at);
        }
        let mut statements = Vec::new();
        for name in run.iter().rev() {
            statements.extend(self.blocks.get(*name)?.statements.iter());
        }
        Some(statements)
    }

    /// The completion code of whatever ended `block`, when it is known: a
    /// plain `return`, or a last statement the registry decodes (see
    /// [`super::exact_statement_completion`]) that is what ended the block — a
    /// `break` / `continue` behind its `Goto`, or a non-`ok` code behind a
    /// `Return`. A block ended some other way (an opaque `switch`, or a
    /// `finally` clause resuming what it interrupted) has no single code.
    /// Earlier statements in the block are not considered.
    fn terminal_code(&self, block: &str) -> Option<tcl_core_types::Code> {
        use tcl_core_types::Code;
        if self.plain_return_blocks.contains(block) {
            return Some(Code::Return);
        }
        let mutable = self.blocks.get(block)?;
        if let Some(Terminator::Complete { route, .. }) = &mutable.terminator {
            return route.immediate_code();
        }
        let stmt = mutable.statements.last()?;
        let resolve = self.embedded_head_resolver();
        let tcl_registry::registry::ExactInvocationCompletion::Tcl(code) =
            self.command_classes.exact_completion(stmt, &resolve)?
        else {
            return None;
        };
        match (&mutable.terminator, code) {
            (Some(Terminator::Goto { .. }), Code::Break | Code::Continue) => Some(code),
            (Some(Terminator::Return { .. }), code)
                if !matches!(code, Code::Ok | Code::Break | Code::Continue) =>
            {
                Some(code)
            }
            _ => None,
        }
    }

    /// The completion code named by the handler under its retained source
    /// numeral grammar. Catalogue availability does not choose this grammar;
    /// unavailable input permits only selectors on which all grammars agree.
    fn handler_code(&self, handler: &crate::ir::TryHandler) -> Option<tcl_core_types::Code> {
        crate::executable_ir::try_handler_code_in(
            handler,
            self.command_classes.source_numbers(self.config),
        )
    }

    /// The handlers of a `try` as the registry reads them
    /// ([`HandlerChain`]): each selector decoded with the target's own numeral
    /// grammar, so `on 010` is code 8 in Tcl 8.x and 10 in 9.0 (decoding it as
    /// 9.0 dropped a live 8.x handler — found in review). Which handler a
    /// completion reaches, which can never run and which scripts a `-` handler
    /// shares are the chain's answers; nothing here reads the list again.
    fn handler_chain(&self, handlers: &[crate::ir::TryHandler]) -> HandlerChain {
        HandlerChain::new(
            handlers.iter().map(|handler| HandlerLink {
                matches: handler.kind,
                selector: &handler.match_arg,
                falls_through: handler.fallthrough,
            }),
            self.command_classes.source_numbers(self.config),
        )
    }

    /// Whether `block` ends in a statement whose completion code handler
    /// `member` is known not to select: the block's code known exactly
    /// ([`Self::block_completion_code`]), and the handler's selector naming
    /// another ([`HandlerChain::misses`]). Either one unknown answers `false`,
    /// keeping the edge.
    fn handler_misses_completion(&self, chain: &HandlerChain, member: usize, block: &str) -> bool {
        self.try_entry
            .as_deref()
            .and_then(|entry| self.block_completion_code(block, entry))
            .is_some_and(|code| chain.misses(member, code))
    }

    /// Record the analysis-only edges that keep a `finally` clause reachable
    /// from every way the `try` body or one of its handlers can leave.
    ///
    /// `lower_try` gave `end_block` a predecessor from a body that falls through
    /// normally, or from a handler's throw edge. Nothing connected a body exit
    /// that is neither — a `return`, `error`, `throw`, or a `break`/`continue`
    /// out of the body — so a `finally` reached only that way read as dead, and
    /// O107 emptied it:
    ///
    /// ```tcl
    /// set g 0
    /// proc p {} { global g; try {error boom} finally {set g 1} }
    /// catch {p}; puts $g
    /// ```
    ///
    /// printed `0` where tclsh 8.6.18 and 9.0.4 print `1` (#2142). The exits
    /// are read off the body's own blocks rather than its resting tail: a body
    /// whose every branch leaves — `if {$c} {return ok} else {error boom}` —
    /// still ends in a resting `if_end` block, so "has a tail" is not "can fall
    /// through", and gating on it left that `finally` dead too.
    ///
    /// Not added without a `finally`: there the tail really is unreachable on
    /// these paths, because the exception resumes unwinding past it.
    ///
    /// A `break` or `continue` is different: the clause runs and then the jump
    /// goes on to its loop target. The jump is retargeted at `end_block` and
    /// the saved targets are returned for [`Self::lower_try`] to resume from
    /// the clause's last block. Adding an edge alongside the jump left a
    /// path into the loop that skipped the clause, and
    /// `while {$first || $x} { try {set first 0; continue} finally {set x 0} }`
    /// reported `x` read before it is set (found in review). The same holds
    /// for a jump a nested `try` has already resumed after its own clause.
    ///
    /// What it does cost, when the body or a handler can *also* complete
    /// normally, is that the clause's normal exit into `try_after_finally` is
    /// shared by the exit paths, where Tcl in fact keeps unwinding or jumps.
    /// Modelling that exactly needs the clause body lowered once per way in;
    /// the merge only adds paths, and over-approximating the *other* way — a
    /// `finally` clause that is never entered — is what corrupted the program
    /// above. When nothing can complete normally there is no merge:
    /// [`Self::lower_try_finally`] ends the clause as an exit.
    fn push_finally_exit_edges(
        &mut self,
        end_block: &str,
        post_body: &str,
        body_block: &str,
        first_body_id: usize,
        chain: &HandlerChain,
        handler_blocks: &[String],
    ) -> (Vec<String>, bool) {
        if !self.faithful_exceptions {
            return (Vec::new(), false);
        }
        let end_id = self.bid(end_block);
        let body_block_id = self.bid(body_block);
        // The body's blocks *and* the handlers': both are lowered after
        // `first_body_id`, and a handler that leaves — `on error {} {return
        // handled}` — is as much an exit as the body's own `return`.
        let in_body = |id: crate::cfg::BlockId| {
            id == body_block_id || usize::try_from(id.0).is_ok_and(|i| i >= first_body_id)
        };
        // Normal completion already reaches `try_end`, directly or through
        // `try_ok`; a jump there is not an exit to wire a second time.
        let completion = [end_id, self.bid(post_body)];
        let leaves = |t: crate::cfg::BlockId| !in_body(t) && !completion.contains(&t);
        // A `return` a nested `catch` or `try … finally` inside this body
        // already intercepts is not an exit of *this* body: the inner one was
        // lowered first and recorded its own edge, and control reaches this
        // `finally` only after the inner clause has run — through the inner
        // construct's normal flow. Wiring it here too would skip the inner
        // `finally`, and `try { try {return ok} finally {set x 1} }
        // finally {puts $x}` read `x` as possibly unset (found in review). A
        // jump needs no such care: rerouting it only replaces an edge that
        // skipped this clause with one through it.
        let intercepted = self.totally_intercepted(&in_body, true);
        let mut sources: Vec<String> = Vec::new();
        let mut jumps: Vec<String> = Vec::new();
        let resolve_head = self.embedded_head_resolver();
        for (name, _) in self.block_ids.iter().filter(|(_, id)| in_body(**id)) {
            let Some(block) = self.blocks.get(name.as_str()) else {
                continue;
            };
            // A nested clause that resumes unwinding once done is an exit of
            // this body too, beside its fall-through.
            if self.unwinding_tails.contains(name) && !intercepted.contains(name.as_str()) {
                sources.push(name.clone());
            }
            match &block.terminator {
                // A process exit runs no `finally` (found in review) — only
                // when nothing can raise before it: it is the only statement
                // of the construct's first block, the body's or a handler's.
                // An earlier statement, or an earlier block (`if {$c} {};
                // exit 0` evaluates `$c` first), may raise an error, and an
                // error does run the clause. So may a handler's binding of its
                // result or options variable: a write trace, or an `upvar` to
                // an array, rejects it before the body runs (found in review).
                // The shared source context owns exact process completion.
                Some(
                    crate::cfg::Terminator::Return { .. } | crate::cfg::Terminator::Complete { .. },
                ) => {
                    if !intercepted.contains(name.as_str())
                        && !self.caught_by_handler(name, body_block, chain, handler_blocks)
                        && !((name == body_block || handler_blocks.contains(name))
                            && matches!(block.statements.as_slice(), [only]
                                if self.command_classes.exits_process(only, &resolve_head)))
                    {
                        sources.push(name.clone());
                    }
                }
                Some(crate::cfg::Terminator::Goto { target, .. }) if leaves(*target) => {
                    jumps.push(name.clone());
                }
                Some(crate::cfg::Terminator::Branch {
                    true_target,
                    false_target,
                    ..
                }) if leaves(*true_target) || leaves(*false_target) => {
                    jumps.push(name.clone());
                }
                _ => {}
            }
        }
        drop(resolve_head);
        let names: rustc_hash::FxHashMap<crate::cfg::BlockId, String> = self
            .block_ids
            .iter()
            .map(|(name, id)| (*id, name.clone()))
            .collect();
        let mut targets: Vec<String> = Vec::new();
        let mut resume = |t: &mut crate::cfg::BlockId| {
            if leaves(*t) {
                targets.push(names[t].clone());
                *t = end_id;
            }
        };
        jumps.sort();
        for name in &jumps {
            match &mut self.block_mut(name).terminator {
                Some(crate::cfg::Terminator::Goto { target, .. }) => resume(target),
                Some(crate::cfg::Terminator::Branch {
                    true_target,
                    false_target,
                    ..
                }) => {
                    resume(true_target);
                    resume(false_target);
                }
                _ => {}
            }
        }
        // A jump a nested `try … finally` resumes after its own clause is
        // still a jump out of this body, and must pass this clause too.
        let (nested, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.finally_jump_edges)
            .into_iter()
            .partition(|(from, to)| {
                self.block_ids.get(from).is_some_and(|id| in_body(*id))
                    && self.block_ids.get(to).is_some_and(|id| leaves(*id))
            });
        self.finally_jump_edges = kept;
        let unwinds = !sources.is_empty();
        for edge in nested {
            self.exception_edges.retain(|e| *e != edge);
            sources.push(edge.0);
            targets.push(edge.1);
        }
        sources.sort();
        sources.dedup();
        for src in sources {
            self.exception_edges.push((src, end_block.to_owned()));
        }
        targets.sort();
        targets.dedup();
        (targets, unwinds)
    }

    /// Lower the body of the `try` `stmt` from `body_block` and send a body that
    /// falls through on to `post_body`, returning what its handlers and its
    /// `finally` clause are wired from. With no `finally`, the body's points
    /// are ways out of the region around it too.
    fn lower_try_body(&mut self, stmt: &Statement, body_block: &str, post_body: &str) -> TryBody {
        let Statement::Try {
            body,
            body_span,
            finally_body,
            ..
        } = stmt
        else {
            unreachable!("lower_try_body called with non-Try");
        };
        // Install a fresh throw-block list around the body so on-error edges
        // are sourced from each explicit `error`/`throw` point (where the
        // body's prior defs are live), not the pre-`try` block. Restore the
        // outer list afterwards so a nested `try`'s throws aren't attributed
        // to this handler.
        let outer_throw_blocks = self.throw_blocks.take();
        self.throw_blocks = Some(Vec::new());
        // Any command of the body, at any depth, may fail, and what the body
        // has stored when it does is what a handler or the `finally` clause
        // runs over: in an analysis build each statement of the body and of
        // the scripts it holds ends a block an exception edge leaves from.
        let outer_region = std::mem::replace(
            &mut self.split_region,
            self.faithful_exceptions.then_some(super::SplitRegion::Try),
        );
        let outer_split_points = std::mem::take(&mut self.split_points);
        let raw_body_tail = self.lower_script(body, body_block);
        // Capture the body's terminating block *before* the handler bodies are
        // lowered below (each overwrites `last_terminal_block`).  Used to source
        // an on-error edge from a body that ended without an explicit
        // `error`/`throw` (a bare `return`).
        let body_terminal = self.last_terminal_block.take();
        let split_points = std::mem::replace(&mut self.split_points, outer_split_points);
        self.split_region = outer_region;
        let body_throw_blocks = self.throw_blocks.take().unwrap_or_default();
        self.throw_blocks = outer_throw_blocks;
        // `lower_script` now always returns the resting block, so distinguish
        // *normal fall-through* from a
        // terminated body via `body_terminal` (set iff the body did not fall
        // through — the former `body_tail.is_none()` signal). A terminated body
        // must not edge to `post_body`, and the handler's on-error edge must be
        // sourced from the throw block(s), not the pre-`try` block.
        let body_tail = if body_terminal.is_none() {
            raw_body_tail
        } else {
            None
        };
        if let Some(tail) = &body_tail {
            self.ensure_goto(tail, post_body, Some(*body_span));
        }
        if finally_body.is_none() {
            self.escape_to_the_region_around(
                &split_points,
                &body_throw_blocks,
                [body_terminal.as_deref(), body_tail.as_deref()],
            );
        }
        TryBody {
            body_tail,
            body_terminal,
            body_throw_blocks,
            split_points,
        }
    }

    /// Flatten `Statement::Try` into body → handlers → finally → end CFG.
    pub(super) fn lower_try(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::Try {
            span,
            body,
            handlers,
            finally_body,
            finally_span,
            ..
        } = stmt
        else {
            unreachable!("lower_try called with non-Try");
        };

        let body_block = self.new_block("try_body");
        let end_block = self.new_block("try_end");
        self.copy_command_boundary(block_name, &body_block);

        self.ensure_goto(block_name, &body_block, Some(*span));

        // Where does control go after body succeeds?
        let post_body = if handlers.is_empty() {
            end_block.clone()
        } else {
            self.new_block("try_ok")
        };

        let first_body_id = self.block_ids.len();
        let TryBody {
            body_tail,
            body_terminal,
            body_throw_blocks,
            split_points,
        } = self.lower_try_body(stmt, &body_block, &post_body);

        // A `-` (fallthrough) handler shares the next non-`-` handler's body.
        // Tcl binds the *matching* handler's variables when running that shared
        // body in the byte-compiled form, but the *target* handler's variables
        // in the interpreted form (the two diverge).
        // Statically we can't know which handler matched, so the shared body is
        // analysed with the whole group's variables treated as defined: the
        // precise over-approximation that avoids a read-before-set false
        // positive under either binding rule.
        let mut pending_fallthrough_defs: Vec<String> = Vec::new();
        let first_handler_id = self.block_ids.len();
        let mut handler_blocks: Vec<String> = Vec::new();
        let outer_entry = self.try_entry.replace(body_block.clone());
        let chain = self.handler_chain(handlers);

        // Each handler reachable from body failure.
        for (index, handler) in handlers.iter().enumerate() {
            let handler_block = self.new_block("try_handler");
            handler_blocks.push(handler_block.clone());
            self.ensure_goto(block_name, &handler_block, Some(*span));

            // Record throw edges into the handler (analysis builds only):
            // `block_name` already gotos `try_body`.
            let live_group = chain.live_group(index);
            self.push_try_handler_exception_edges(
                (&chain, &live_group),
                (&handler_block, block_name, &body_block),
                body_tail.as_deref(),
                (&body_throw_blocks, &split_points),
                (body_terminal.as_deref(), body.statements.first()),
            );

            let var_defs = handler_var_defs(handler, &mut pending_fallthrough_defs);
            self.push_handler_var_defs(&handler_block, var_defs, *span);

            if let Some(tail) = self.lower_script(&handler.body, &handler_block) {
                self.ensure_goto(&tail, &end_block, Some(handler.body_span));
            }
        }

        self.try_entry = outer_entry;
        // A split body's last step, where it leaves in a straight line, is the
        // block a handler may catch whole.
        let run_end = body_terminal.clone().unwrap_or_else(|| body_block.clone());
        if self.caught_by_handler(&run_end, &body_block, &chain, &handler_blocks) {
            self.handler_caught.insert(run_end);
        }
        // Success path reaches end.
        if !handlers.is_empty() {
            self.ensure_goto(&post_body, &end_block, Some(*span));
        }
        self.route_caught_loop_jumps(
            &chain,
            &handler_blocks,
            &body_block,
            first_body_id,
            first_handler_id,
        );

        // Finally block.
        let Some(fb) = finally_body else {
            return end_block;
        };
        self.total_interceptors.insert(end_block.clone());
        // Read before the exit edges below add paths into `end_block`.
        let completes_normally = self.try_completes_normally(
            block_name,
            &end_block,
            &post_body,
            &body_block,
            first_body_id,
        );
        let (jump_targets, unwinds) = self.push_finally_exit_edges(
            &end_block,
            &post_body,
            &body_block,
            first_body_id,
            &chain,
            &handler_blocks,
        );
        self.push_finally_failure_edges(
            (&end_block, &chain),
            (block_name, &body_block, body.statements.first()),
            &split_points,
        );
        self.finish_try_finally(
            fb,
            finally_span.or(Some(*span)),
            &end_block,
            completes_normally.then_some(unwinds),
            jump_targets,
        )
    }

    /// Lower a `try`'s `finally` clause and resume the loop jumps routed
    /// through it, returning the block the whole statement rests in.
    ///
    /// A saved jump resumes from the clause's own last block, not from
    /// `after_finally`: the statements after the `try` are appended there,
    /// and a `break` does not run them (found in review). A clause that
    /// cannot complete normally resumes nothing.
    ///
    /// `falls_through` is `None` when nothing completes the `try` normally,
    /// else whether an unwinding exit (a `return`, an error) also enters the
    /// clause. Then the clause's fall-through `Goto` stands only for normal
    /// completion: the `return` resumes past the statements after the `try`,
    /// so the last block is recorded as an unwinding tail and a throw point
    /// for the constructs around it. Without that, `try { try {if {$c}
    /// {return}} finally {}; set x 1 } finally {puts $x}` ran `set x 1` on
    /// the `return` path too, and O102 forwarded `1` into a read tclsh 8.6.18
    /// and 9.0.4 fail on (found in review).
    fn finish_try_finally(
        &mut self,
        body: &crate::ir::Script,
        fin_span: Option<tcl_lexer::Span>,
        end_block: &str,
        falls_through: Option<bool>,
        jump_targets: Vec<String>,
    ) -> String {
        let (after_finally, finally_tail) = self.lower_try_finally(
            body,
            fin_span,
            end_block,
            falls_through.is_some() || !self.faithful_exceptions,
        );
        if let Some(tail) = finally_tail {
            if falls_through == Some(true) {
                self.unwinding_tails.insert(tail.clone());
                if let Some(blocks) = self.throw_blocks.as_mut() {
                    blocks.push(tail.clone());
                }
            }
            for target in jump_targets {
                let edge = (tail.clone(), target);
                self.exception_edges.push(edge.clone());
                self.finally_jump_edges.push(edge);
            }
        }
        after_finally
    }

    /// Send a `break` / `continue` out of the body into the handler that
    /// catches it, instead of to its loop target.
    ///
    /// Tcl runs the first handler whose selector matches the completion, so a
    /// jump an `on break` handler selects never reaches the loop: in
    /// `while 1 { try {break} on break {} {set x 1} finally {}; break }` the
    /// handler binds `x` before the loop is left. Keeping the jump's own edge
    /// let the read after the loop see `x` unset, and the `finally` routing
    /// resumed it too (found in review). Only a jump out of the body (not one
    /// inside a nested loop, and not one in a handler) is caught; and a
    /// handler met first whose selector the registry cannot decode might catch
    /// it instead, so then the jump keeps its edge. A `-` handler hands the
    /// jump to the body it shares.
    fn route_caught_loop_jumps(
        &mut self,
        chain: &HandlerChain,
        handler_blocks: &[String],
        body_block: &str,
        first_body_id: usize,
        first_handler_id: usize,
    ) {
        if !self.faithful_exceptions || chain.is_empty() {
            return;
        }
        let body_block_id = self.bid(body_block);
        let in_try = |id: crate::cfg::BlockId| {
            id == body_block_id || usize::try_from(id.0).is_ok_and(|i| i >= first_body_id)
        };
        let in_body = |id: crate::cfg::BlockId| {
            id == body_block_id
                || usize::try_from(id.0).is_ok_and(|i| i >= first_body_id && i < first_handler_id)
        };
        // A jump a nested `catch` (or a nested `try … finally`) swallows never
        // reaches this `try`'s handlers: `try {catch {break}; return}
        // on break {} {…}` runs no handler (found in review).
        let intercepted = self.totally_intercepted(&in_body, false);
        let mut retargets: Vec<(String, String)> = Vec::new();
        for (name, id) in &self.block_ids {
            if !in_body(*id) || intercepted.contains(name.as_str()) {
                continue;
            }
            let Some(Terminator::Goto { target, .. }) = self
                .blocks
                .get(name.as_str())
                .and_then(|b| b.terminator.as_ref())
            else {
                continue;
            };
            if in_try(*target) {
                continue;
            }
            // The `Goto` stands for the jump alone; an earlier statement's
            // failure has its own handler edges.
            let Some(jump) = self.terminal_code(name) else {
                continue;
            };
            if !matches!(
                jump,
                tcl_core_types::Code::Break | tcl_core_types::Code::Continue
            ) {
                continue;
            }
            if let Some(first) = chain.first_taking(jump) {
                retargets.push((name.clone(), handler_blocks[chain.owner(first)].clone()));
            }
        }
        retargets.sort();
        for (name, handler_block) in retargets {
            let target = self.bid(&handler_block);
            if let Some(Terminator::Goto { target: t, .. }) = &mut self.block_mut(&name).terminator
            {
                *t = target;
            }
        }
    }

    /// Whether one of this `try`'s handlers catches every completion `block`
    /// can leave with: the block's exact completion code is known, and it has
    /// an edge into a handler whose decoded selector is that code. Control
    /// then reaches the `finally` through the handler, and a direct edge would
    /// add a path where the error escaped uncaught — which made
    /// `try {error boom} on error {} {set f 2} finally {set f 1}` read the
    /// handler's dead store as live. A block whose completion is not exact
    /// (`return $x` may raise while substituting) keeps its own exit, and so
    /// does one whose only matching handler is a `trap`: its `-errorcode`
    /// prefix may not match, and `try {error boom} trap {NOT MATCHING} {}
    /// {exit 0} finally {…}` runs the clause (found in review).
    fn caught_by_handler(
        &self,
        block: &str,
        body_block: &str,
        chain: &HandlerChain,
        handler_blocks: &[String],
    ) -> bool {
        let Some(code) = self.block_completion_code(block, body_block) else {
            return false;
        };
        // A `-` handler's match runs its owner's block.
        (0..chain.len()).any(|index| {
            chain.takes(index, code)
                && self
                    .exception_edges
                    .iter()
                    .any(|(from, to)| from == block && *to == handler_blocks[chain.owner(index)])
        })
    }

    /// The blocks whose every completion a construct nested inside this one
    /// intercepts: sources of an exception edge into a
    /// [`total interceptor`](Self::total_interceptors) that `inside` contains.
    ///
    /// A `try` handler does not count. It selects only some completion codes,
    /// and a block whose completion may be one it does not select must keep
    /// its own exit: in `try {return $x} on error {} {exit 0} finally {set g
    /// 1}` the substitution's error reaches the handler but the `return` still
    /// runs the clause, and counting the handler edge as interception let O107
    /// empty it (found in review).
    ///
    /// With `with_handler_catches`, also the blocks a nested `try`'s
    /// unconditional handler catches whole ([`Self::handler_caught`]): an
    /// outer `finally` scan must not route those past the inner handler and
    /// clause. Loop-jump routing leaves them out — a nested `try` has already
    /// sent its own caught jumps into its handler.
    pub(super) fn totally_intercepted(
        &self,
        inside: &dyn Fn(crate::cfg::BlockId) -> bool,
        with_handler_catches: bool,
    ) -> std::collections::HashSet<&str> {
        let caught = self
            .handler_caught
            .iter()
            .filter(|_| with_handler_catches)
            .filter(|block| self.block_ids.get(*block).is_some_and(|id| inside(*id)))
            .map(String::as_str);
        self.exception_edges
            .iter()
            .filter(|(_, to)| {
                self.total_interceptors.contains(to)
                    && self.block_ids.get(to).is_some_and(|id| inside(*id))
            })
            .map(|(from, _)| from.as_str())
            .chain(caught)
            .collect()
    }

    /// Whether any path through a `try`'s body or handlers reaches `end_block`
    /// by completing normally.
    ///
    /// Asked of the graph rather than of the resting tails, because a body
    /// that cannot fall through still leaves one: an inner `try` that never
    /// completes returns its unreachable `try_after_finally`, and an `if`
    /// whose every branch leaves its `if_end`. Walks the construct's own
    /// blocks from the pre-`try` block, whose exception edges lead into the
    /// handlers; anything outside the construct is not a way to `end_block`.
    fn try_completes_normally(
        &self,
        block_name: &str,
        end_block: &str,
        post_body: &str,
        body_block: &str,
        first_body_id: usize,
    ) -> bool {
        let end_id = self.bid(end_block);
        // With handlers, a body that falls through reaches `try_end` by way of
        // `try_ok`, which is allocated before the body; reaching it is normal
        // completion. Missing it made `try {set x 1} on error {} {return
        // early} finally {}` look as if it never completed, and O107 deleted
        // the code after it (found in review).
        let ok_id = self.bid(post_body);
        let body_block_id = self.bid(body_block);
        let in_body = |id: crate::cfg::BlockId| {
            id == body_block_id || usize::try_from(id.0).is_ok_and(|i| i >= first_body_id)
        };
        let names: rustc_hash::FxHashMap<crate::cfg::BlockId, &str> = self
            .block_ids
            .iter()
            .map(|(name, id)| (*id, name.as_str()))
            .collect();
        let mut seen: rustc_hash::FxHashSet<&str> = rustc_hash::FxHashSet::default();
        let mut work = vec![block_name];
        while let Some(name) = work.pop() {
            if !seen.insert(name) {
                continue;
            }
            let terminator_succs = self
                .blocks
                .get(name)
                .and_then(|b| b.terminator.as_ref())
                .map(crate::cfg::Terminator::successors)
                .unwrap_or_default();
            if terminator_succs.contains(&end_id) || terminator_succs.contains(&ok_id) {
                return true;
            }
            let exception_succs = self
                .exception_edges
                .iter()
                .filter(|(from, _)| from == name)
                .filter_map(|(_, to)| self.block_ids.get(to).copied());
            for id in terminator_succs.into_iter().chain(exception_succs) {
                if in_body(id)
                    && let Some(next) = names.get(&id)
                {
                    work.push(next);
                }
            }
        }
        false
    }

    /// Bind a handler's `on`/`trap` variables at the top of its block, as the
    /// synthetic definition the ordinary walks read.
    fn push_handler_var_defs(
        &mut self,
        handler_block: &str,
        var_defs: Vec<String>,
        span: tcl_lexer::Span,
    ) {
        if var_defs.is_empty() {
            return;
        }
        self.push_statement(
            handler_block,
            Statement::Call {
                span,
                command: "try".into(),
                canonical_command: None,
                args: vec![],
                defs: var_defs,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            },
        );
    }

    /// Lower a `finally` clause after the `try`'s end block, returning the
    /// resting block the whole statement leaves behind and the clause's own
    /// last block, if it completes normally.
    ///
    /// Only a `try` whose body or some handler can complete normally falls
    /// through the clause into the statements after it. When none can, every
    /// path into the clause is an exit, which resumes unwinding or a saved
    /// jump once the clause is done, so the clause ends as an `error` does and
    /// the code after the `try` is unreachable: `while 1 { try {break}
    /// finally {}; set x 1 }` never runs `set x 1` (found in review).
    fn lower_try_finally(
        &mut self,
        body: &crate::ir::Script,
        fin_span: Option<tcl_lexer::Span>,
        end_block: &str,
        falls_through: bool,
    ) -> (String, Option<String>) {
        let finally_block = self.new_block("try_finally");
        self.ensure_goto(end_block, &finally_block, fin_span);
        let after_finally = self.new_block("try_after_finally");
        let tail = self.lower_script(body, &finally_block);
        // A clause that itself leaves — `finally {break}` — keeps its own
        // transfer, which overrides whatever completion was pending: Tcl runs
        // the code after the loop in `while 1 { try {return} finally {break} }`.
        // Such a tail neither falls through nor resumes a saved jump
        // (found in review).
        let tail = tail.filter(|_| self.last_terminal_block.take().is_none());
        if let Some(tail) = &tail {
            if falls_through {
                self.ensure_goto(tail, &after_finally, fin_span);
            } else {
                self.set_terminator(tail, crate::cfg::Terminator::Complete {
                    route: tcl_registry::completion_route::InvocationCompletionRoute::UnknownAbrupt,
                    span: fin_span,
                });
                // The clause resumes unwinding, so an enclosing handler catches
                // what it raises with the clause's defs live, as for `error`.
                if let Some(blocks) = self.throw_blocks.as_mut() {
                    blocks.push(tail.clone());
                }
            }
        }
        (after_finally, tail)
    }

    /// Flatten `Statement::Catch` into body → end CFG, the analogue of
    /// [`Self::lower_try`] for the simpler construct.
    ///
    /// `catch` has no handler clauses, no fallthrough groups and no handler
    /// variable binding, so the shape is just body → end with exception edges
    /// from the body's throw points. What it buys is what the opaque form
    /// costs: with the body as real blocks the ordinary emitters compile it,
    /// so its variables reach the LVT and the optimiser can see inside — C
    /// allocates `q` in `catch {set q $x}` as a compiled local, and before
    /// this the whole body was one `invokeStk` (#2207, and the missing slot
    /// behind #2173).
    ///
    /// `result_var` / `options_var` are defined on *both* paths — normal
    /// completion stores the body's result, an error stores the message — so
    /// unlike a `try` handler's variables they are defined at the end block,
    /// which both paths reach, rather than on the error path alone.
    pub(super) fn lower_catch(&mut self, stmt: &Statement, block_name: &str) -> String {
        let Statement::Catch {
            span,
            body,
            body_span,
            ..
        } = stmt
        else {
            unreachable!("lower_catch called with non-Catch");
        };

        let body_block = self.new_block("catch_body");
        let end_block = self.new_block("catch_end");
        self.copy_command_boundary(block_name, &body_block);
        self.command_boundary_continuations
            .insert(body_block.clone(), end_block.clone());
        self.ensure_goto(block_name, &body_block, Some(*span));

        // Same throw-block bookkeeping as `lower_try`: install a fresh list
        // around the body so the exception edges are sourced from each
        // explicit `error`/`throw` point — where the body's prior defs are
        // live — and not from the pre-`catch` block. Restoring the outer list
        // keeps a nested `catch`'s throws attributed to its own region.
        let outer_throw_blocks = self.throw_blocks.take();
        self.throw_blocks = Some(Vec::new());
        // Any command of the body may fail, and what the body has stored when
        // it does is the handler's state, so in an analysis build each
        // statement ends a block an exception edge leaves from.
        let outer_region = std::mem::replace(
            &mut self.split_region,
            self.faithful_exceptions
                .then_some(super::SplitRegion::Catch),
        );
        let outer_split_points = std::mem::take(&mut self.split_points);
        let (raw_body_tail, body_terminal) = if self.faithful_exceptions {
            // The same captured-completion owner handles package lifecycle
            // phases and catch bodies. Captured return/break/continue must
            // reach this merge, rather than remain procedure/loop exits.
            self.lower_region_phase(body, &body_block, Some(&end_block), Some(&end_block));
            (None, None)
        } else {
            (
                self.lower_script(body, &body_block),
                self.last_terminal_block.take(),
            )
        };
        let split_points = std::mem::replace(&mut self.split_points, outer_split_points);
        self.split_region = outer_region;
        let body_throw_blocks = self.throw_blocks.take().unwrap_or_default();
        self.throw_blocks = outer_throw_blocks;

        // A body that did not fall through (a bare `return`, an `error`) must
        // not edge to the end block as normal completion; `catch` still
        // resumes there, but by catching, which is an exception edge.
        let body_tail = if body_terminal.is_none() {
            raw_body_tail
        } else {
            None
        };
        if let Some(tail) = &body_tail {
            self.ensure_goto(tail, &end_block, Some(*body_span));
        }

        // Every way the body can fail reaches the end block, because `catch`
        // catches everything. Analysis-only edges (SSA phi predecessors and
        // SCCP reachability), exactly as `push_try_handler_exception_edges`
        // records them for a handler.
        //
        // The edge from the *pre-catch* block is the one that matters for
        // soundness, and it is not optional: the body can fail at its very
        // first command, so the state before the `catch` reaches the end
        // untouched by anything the body writes. Without it
        // `set cmd safe; catch {set cmd risky}; $cmd` joins to `risky`
        // alone — a single may-target that would license a destructive
        // rename, where the truth is `safe` or `risky`. `lower_try` gets
        // this from `ensure_goto(block_name, &handler_block, …)`; a `catch`
        // has no handler block to edge to, so it is recorded here.
        let mut throw_sources: Vec<String> = vec![block_name.to_owned()];
        throw_sources.extend(split_points.into_iter().map(|point| point.block));
        for tb in &body_throw_blocks {
            if !throw_sources.contains(tb) {
                throw_sources.push(tb.clone());
            }
        }
        if let Some(terminal) = &body_terminal
            && !throw_sources.contains(terminal)
        {
            throw_sources.push(terminal.clone());
        }
        if let Some(tail) = &body_tail
            && !throw_sources.contains(tail)
        {
            throw_sources.push(tail.clone());
        }
        for src in throw_sources {
            self.exception_edges.push((src, end_block.clone()));
        }
        if self.faithful_exceptions {
            self.region_entries.push((
                block_name.to_owned(),
                end_block.clone(),
                body_block.clone(),
            ));
        }
        self.total_interceptors.insert(end_block.clone());

        self.end_flattened_catch(stmt, (block_name, &end_block));

        end_block
    }

    /// The statement that ends a flattened `catch` region: it defines the
    /// result and options variables however the body ended, so they belong at
    /// the merge rather than on one path, and the script's last command's
    /// stores are observed there, since `catch` stores that command's value in
    /// the result variable — a store nothing else reads is not a dead one, and
    /// deleting it changes what the result variable holds.
    ///
    /// The statement has no words, so the code generator skips it; the
    /// analysis build keeps the `catch` as written beside it, for the solver to
    /// evaluate over the state before the body.
    fn end_flattened_catch(&mut self, stmt: &Statement, (block_name, end_block): (&str, &str)) {
        let Statement::Catch {
            span,
            body,
            result_var,
            options_var,
            raw_args,
            tokens,
            ..
        } = stmt
        else {
            unreachable!("end_flattened_catch called with non-Catch");
        };
        if result_var.is_some()
            && let Some(last) = body.statements.last()
        {
            self.alias_observed_vars
                .extend(crate::ssa::defs_of_with_registry(last, Some(self.registry)));
        }
        let mut defs = Vec::new();
        if let Some(rv) = result_var {
            defs.push(rv.clone());
        }
        if let Some(ov) = options_var {
            defs.push(ov.clone());
        }
        if defs.is_empty() {
            return;
        }
        if self.faithful_exceptions {
            self.catch_ends.push((
                block_name.to_owned(),
                end_block.to_owned(),
                Statement::Call {
                    span: *span,
                    command: "catch".into(),
                    canonical_command: None,
                    args: raw_args.clone(),
                    defs: defs.clone(),
                    reads: vec![],
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: tokens.clone(),
                    foreach_groups: None,
                },
            ));
        }
        let mut output_tokens = tokens.clone().unwrap_or_else(|| {
            crate::ir::CommandTokens::marker(crate::ir::SyntheticMarker::CapturedCatchOutputs)
        });
        output_tokens.synthetic = Some(crate::ir::SyntheticMarker::CapturedCatchOutputs);
        let command = output_tokens
            .argv_texts
            .first()
            .cloned()
            .unwrap_or_default();
        self.push_statement(
            end_block,
            Statement::Call {
                span: *span,
                command,
                canonical_command: None,
                args: raw_args.clone(),
                defs,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(output_tokens),
                foreach_groups: None,
            },
        );
    }
}

/// What lowering a `try` body leaves for its handlers and its `finally`
/// clause to be wired from.
struct TryBody {
    /// The block the body rests in, when it falls through.
    body_tail: Option<String>,
    /// The block the body ended in, when it did not fall through.
    body_terminal: Option<String>,
    /// The blocks an explicit `error` or `throw` of the body ends.
    body_throw_blocks: Vec<String>,
    /// The points a throw may leave the body from, in an analysis build.
    split_points: Vec<super::SplitPoint>,
}

/// The names a `try` handler binds at the top of its block.
///
/// A `-` (fallthrough) handler has an empty body of its own and carries its
/// names on to the shared body; the target of a `-` chain may run with any
/// group member's names bound, so it defines them all.
fn handler_var_defs(
    handler: &crate::ir::TryHandler,
    pending_fallthrough_defs: &mut Vec<String>,
) -> Vec<String> {
    let own_defs: Vec<String> = handler
        .var_name
        .iter()
        .chain(&handler.options_var)
        .cloned()
        .collect();
    if handler.fallthrough {
        pending_fallthrough_defs.extend(own_defs.iter().cloned());
        return own_defs;
    }
    let mut defs = std::mem::take(pending_fallthrough_defs);
    for d in own_defs {
        if !defs.contains(&d) {
            defs.push(d);
        }
    }
    defs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg_builder::{
        CfgCommandClasses, build_cfg_function as build_cfg_function_for_registry,
    };
    use crate::ir::{ForeachIterator, Script, SwitchArm, SwitchMode, TryHandler};
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;

    fn build_cfg_function(name: &str, script: &Script, inline_loops: bool) -> crate::cfg::Function {
        build_cfg_function_for_registry(
            name,
            script,
            inline_loops,
            &CommandRegistry::build_default(),
            false,
        )
    }

    #[test]
    fn original_try_handler_codes_keep_source_grammar_over_catalogue_availability() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Source-clause selection only; no native completion was executed.
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let registry = context.commands();
        let source = "try {} on 8 {} {} on 010 {} {}";
        for (numbers, expected, shadowed) in [
            (
                tcl_dialect::NumberSyntax::Tcl85,
                tcl_core_types::Code::Other(8),
                true,
            ),
            (
                tcl_dialect::NumberSyntax::Tcl90,
                tcl_core_types::Code::Other(10),
                false,
            ),
        ] {
            let mut profile = tcl_dialect::DialectProfile::plain_tcl().clone();
            profile.grammar.numbers = numbers;
            let profile = profile.intern();
            let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
            let input = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                std::sync::Arc::clone(&context),
                config,
            );
            let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
                .with_resolved_analysis_input(input.clone());
            let module = lowerer.lower(source).clone();
            let Statement::Try { handlers, .. } = &module.top_level.statements[0] else {
                panic!("genuine source try clauses: {:?}", module.top_level);
            };
            assert_eq!(handlers.len(), 2);
            let mut builder = CfgBuilder::new(false, registry).with_lexer_config(config);
            builder.command_classes = CfgCommandClasses::from_source_input(registry, Some(&input));
            assert_eq!(builder.handler_code(&handlers[1]), Some(expected));
            assert_eq!(
                builder.handler_shadowed(&handlers[..1], &handlers[1]),
                shadowed
            );
            builder.command_classes = CfgCommandClasses::from_source_input(registry, None);
            assert_eq!(builder.handler_code(&handlers[1]), None);
            assert!(!builder.handler_shadowed(&handlers[..1], &handlers[1]));
            assert_eq!(
                builder.handler_code(&handlers[0]),
                Some(tcl_core_types::Code::Other(8))
            );
            let foreign = crate::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                tcl_registry::model::ingress::resolve_environment("tcl8.4")
                    .default_context_registry(),
                config,
            );
            builder.command_classes =
                CfgCommandClasses::from_source_input(registry, Some(&foreign));
            assert_eq!(builder.handler_code(&handlers[1]), None);
            builder.command_classes = CfgCommandClasses::from_source_input(registry, Some(&input));
            let changed = tcl_lexer::LexerConfig {
                expand_syntax: !config.expand_syntax,
                ..config
            };
            builder = builder.with_lexer_config(changed);
            assert_eq!(builder.handler_code(&handlers[1]), None);
        }
    }

    #[test]
    fn for_loop_creates_header_body_step() {
        let script = Script::from_statements(vec![Statement::For {
            span: Span::new(0, 40),
            init: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(5, 12),
                name: "i".into(),
                name_braced: false,
                value: "0".into(),
                value_span: None,
            }]),
            init_span: Span::new(4, 13),
            condition: ExprNode::Binary {
                op: BinOp::Lt,
                left: Box::new(ExprNode::Var {
                    text: "$i".into(),
                    name: "i".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Literal {
                    text: "10".into(),
                    start: 5,
                    end: 7,
                }),
            },
            condition_span: Span::new(14, 22),
            next: Script::from_statements(vec![Statement::Incr {
                span: Span::new(24, 30),
                name: "i".into(),
                name_braced: false,
                amount: None,
                amount_braced: false,
                safe_on_uninit: false,
            }]),
            next_span: Span::new(23, 31),
            body: Script::new(),
            body_span: Span::new(32, 34),
            raw_args: vec![],
            raw_tokens: None,
            condition_base: None,
        }]);

        let func = build_cfg_function("::test", &script, true);
        // Should have blocks for: entry, for_header, for_body, for_step, for_end, exit
        assert!(func.blocks.len() >= 5, "got {} blocks", func.blocks.len());
        // A for_header block should have a Branch terminator.
        let header = func
            .blocks
            .values()
            .find(|b| b.name.starts_with("for_header"))
            .expect("should have for_header block");
        assert!(matches!(header.terminator, Some(Terminator::Branch { .. })));
    }

    #[test]
    fn while_loop_creates_header_body() {
        let script = Script::from_statements(vec![Statement::While {
            span: Span::new(0, 20),
            condition: ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            condition_span: Span::new(6, 7),
            body: Script::new(),
            body_span: Span::new(9, 11),
            raw_args: vec![],
            raw_tokens: None,
            condition_base: None,
        }]);

        let func = build_cfg_function("::test", &script, true);
        let header = func
            .blocks
            .values()
            .find(|b| b.name.starts_with("while_header"))
            .expect("should have while_header");
        assert!(matches!(header.terminator, Some(Terminator::Branch { .. })));
    }

    #[test]
    fn foreach_creates_loop() {
        let script = Script::from_statements(vec![Statement::Foreach {
            span: Span::new(0, 30),
            iterators: vec![ForeachIterator {
                vars: vec!["x".into()],
                list_arg: "$list".into(),
                list_braced: false,
            }],
            body: Script::new(),
            body_span: Span::new(20, 25),
            is_lmap: false,
            raw_args: vec![],
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        }]);

        let func = build_cfg_function("::test", &script, true);
        let header = func
            .blocks
            .values()
            .find(|b| b.name.starts_with("foreach_header"))
            .expect("should have foreach_header");
        // Header should have a synthetic Call for iteration vars.
        assert!(header.statements.iter().any(|s| matches!(
            s,
            Statement::Call { command, .. } if command == "foreach"
        )));
    }

    #[test]
    fn switch_exact_creates_branches() {
        let script = Script::from_statements(vec![Statement::Switch {
            subject_braced: false,
            raw_arg_braced: Vec::new(),
            raw_arg_quoted: Vec::new(),
            command: "switch".into(),
            span: Span::new(0, 50),
            subject: "$x".into(),
            subject_span: Span::new(7, 9),
            arms: vec![
                SwitchArm {
                    pattern: "a".into(),
                    pattern_braced: true,
                    pattern_span: Span::new(11, 12),
                    body: Some(Script::new()),
                    body_span: Some(Span::new(13, 15)),
                    fallthrough: false,
                },
                SwitchArm {
                    pattern: "b".into(),
                    pattern_braced: true,
                    pattern_span: Span::new(16, 17),
                    body: Some(Script::new()),
                    body_span: Some(Span::new(18, 20)),
                    fallthrough: false,
                },
            ],
            default_body: None,
            default_span: None,
            mode: SwitchMode::Exact,
            nocase: false,
            raw_args: vec![],
            patterns_braced: true,
        }]);

        let func = build_cfg_function("::test", &script, true);
        // Entry should have a Branch (first arm test).
        let entry = &func.blocks[&func.entry];
        assert!(matches!(entry.terminator, Some(Terminator::Branch { .. })));
    }

    /// Build a one-arm switch (body reads `$y`) in the given mode.
    fn glob_regexp_switch(mode: SwitchMode) -> Script {
        Script::from_statements(vec![Statement::Switch {
            subject_braced: false,
            raw_arg_braced: Vec::new(),
            raw_arg_quoted: Vec::new(),
            command: "switch".into(),
            span: Span::new(0, 40),
            subject: "$x".into(),
            subject_span: Span::new(7, 9),
            arms: vec![SwitchArm {
                pattern: "a*".into(),
                pattern_braced: true,
                pattern_span: Span::new(11, 13),
                body: Some(Script::from_statements(vec![Statement::Call {
                    span: Span::new(15, 22),
                    command: "puts".into(),
                    canonical_command: None,
                    args: vec!["$y".into()],
                    defs: vec![],
                    reads: vec!["y".into()],
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: None,
                    foreach_groups: None,
                }])),
                body_span: Some(Span::new(14, 23)),
                fallthrough: false,
            }],
            default_body: None,
            default_span: None,
            mode,
            nocase: false,
            raw_args: vec!["$x".into(), "a*".into(), "{puts $y}".into()],
            patterns_braced: true,
        }])
    }

    #[test]
    fn switch_glob_stays_opaque() {
        // Glob/regexp switches are kept opaque (a single `Statement::Switch` in
        // the block, no expanded arm blocks / branches). Codegen emits a generic
        // `switch` invoke for them.
        let func = build_cfg_function("::test", &glob_regexp_switch(SwitchMode::Glob), true);
        let entry = &func.blocks[&func.entry];
        assert_eq!(entry.statements.len(), 1, "opaque switch is one statement");
        assert!(
            matches!(entry.statements[0], Statement::Switch { .. }),
            "glob switch should stay an opaque Statement::Switch"
        );
        // The block falls through (no branch dispatch); `build_function` adds a
        // goto to the synthetic trailing exit block.
        assert!(
            matches!(entry.terminator, Some(Terminator::Goto { .. })),
            "opaque switch block falls through via a goto, not a branch dispatch"
        );
        // Only the entry + the synthetic trailing exit block exist (no arm
        // blocks — codegen emits a generic invoke from the statement).
        assert_eq!(func.blocks.len(), 2);
    }

    #[test]
    fn switch_regexp_stays_opaque() {
        let func = build_cfg_function("::test", &glob_regexp_switch(SwitchMode::Regexp), true);
        let entry = &func.blocks[&func.entry];
        assert!(matches!(entry.statements[0], Statement::Switch { .. }));
        assert!(matches!(entry.terminator, Some(Terminator::Goto { .. })));
        // Exact switches without fall-through keep the real expanded jump table.
        let exact = build_cfg_function("::test", &glob_regexp_switch(SwitchMode::Exact), true);
        assert!(
            matches!(
                exact.blocks[&exact.entry].terminator,
                Some(Terminator::Branch { .. })
            ),
            "exact non-fall-through switch still expands to a branch dispatch"
        );
    }

    /// The operands of the first dispatch branch of the `switch` in `source`,
    /// lowered and built under `dialect`'s own registry; `None` where the
    /// statement stayed one opaque statement.
    fn dispatch_operands(source: &str, dialect: &str) -> Option<(ExprNode, ExprNode)> {
        use tcl_registry::model::ingress::{resolve_environment, static_context_for};
        let registry = static_context_for(dialect).commands();
        let profile = resolve_environment(dialect).analyser_profile();
        let module = crate::lowering::lower_to_ir_with_dialect(
            source,
            registry,
            LexerConfig::for_profile(registry.profile()),
            Some(profile),
        );
        let func =
            build_cfg_function_for_registry("::test", &module.top_level, false, registry, false);
        match &func.blocks[&func.entry].terminator {
            Some(Terminator::Branch {
                condition:
                    ExprNode::Binary {
                        op: BinOp::StrEq,
                        left,
                        right,
                    },
                ..
            }) => Some(((**left).clone(), (**right).clone())),
            _ => None,
        }
    }

    /// The chain compares the values of a `switch`'s words, carried braced so
    /// nothing reads them again: a bare or quoted word is its escapes decoded,
    /// a braced one its content with the continuation collapsed, and an escaped
    /// `$` is data. A word that substitutes stays its spelling, which no
    /// evaluator folds, and a word whose value is its spelling keeps the
    /// operand it always had.
    #[test]
    fn the_chain_compares_the_values_of_a_switch_words() {
        let value = |text: &str| ExprNode::CompiledWord {
            text: text.into(),
            braced: true,
        };
        for (source, subject, pattern) in [
            (r#"switch a\nb {"a\nb" {puts hit}}"#, "a\nb", "a\nb"),
            (r#"switch "a\tb" a\tb {puts hit}"#, "a\tb", "a\tb"),
            (r#"switch {a\b} {"a\\b" {puts hit}}"#, r"a\b", r"a\b"),
            ("switch {a\\\nb} {{a b} {puts hit}}", "a b", "a b"),
            (r"switch a\$b {a\$b {puts hit}}", "a$b", "a$b"),
        ] {
            let (left, right) = dispatch_operands(source, "tcl8.6").expect("a dispatch chain");
            assert_eq!((left, right), (value(subject), value(pattern)), "{source}");
        }
        let (left, right) =
            dispatch_operands("switch a${x} {abc {puts hit}}", "tcl8.6").expect("a dispatch chain");
        assert_eq!(
            left,
            ExprNode::CompiledWord {
                text: "a${x}".into(),
                braced: false
            }
        );
        assert_eq!(right, value("abc"));
        let (left, _) =
            dispatch_operands("switch abc {abc {puts hit}}", "tcl8.6").expect("a dispatch chain");
        assert_eq!(
            left,
            ExprNode::CompiledWord {
                text: "abc".into(),
                braced: false
            }
        );
    }

    /// Before 8.5 `switch` reads every leading word that starts with `-` as an
    /// option, however many words follow, so a subject whose value may start
    /// that way — a variable, or a literal whose escape decodes to `-` — is not
    /// flattened under a release that may be before 8.5, unless `--` ended the
    /// run; any other subject is, and so is every subject from 8.5. From 8.5
    /// the scan stops with two words left, which a pattern and its body fill,
    /// so with the arms as words the subject is inside it on every release. A
    /// profile that names no release is read as one that may be 8.4.
    #[test]
    fn a_subject_a_release_may_read_as_an_option_stays_one_statement() {
        for (dialect, chained) in [
            ("tcl8.4", false),
            ("f5-irules", false),
            ("tk", false),
            ("tcl8.5", true),
            ("tcl8.6", true),
            ("tcl9.0", true),
        ] {
            for source in [
                "switch $x {a {puts A}}",
                "switch -exact $x {a {puts A}}",
                r"switch \x2dglob {a {puts A}}",
            ] {
                assert_eq!(
                    dispatch_operands(source, dialect).is_some(),
                    chained,
                    "{dialect}: {source}"
                );
            }
        }
        for dialect in ["tcl8.4", "f5-irules", "tk", "tcl8.5", "tcl8.6", "tcl9.0"] {
            for source in [
                "switch $x a {puts A} b {puts B}",
                "switch -exact $x a {puts A}",
                r"switch \x2dglob a {puts A} b {puts B}",
            ] {
                assert!(
                    dispatch_operands(source, dialect).is_none(),
                    "{dialect}: {source}"
                );
            }
            for source in [
                "switch -- $x a {puts A} b {puts B}",
                "switch abc a {puts A} b {puts B}",
                "switch [gets stdin] a {puts A} b {puts B}",
            ] {
                assert!(
                    dispatch_operands(source, dialect).is_some(),
                    "{dialect}: {source}"
                );
            }
        }
        for dialect in ["tcl8.4", "f5-irules", "tk", "tcl8.6"] {
            for source in [
                "switch -- $x {a {puts A}}",
                "switch -exact -- $x {a {puts A}}",
                "switch abc {a {puts A}}",
                "switch [gets stdin] {a {puts A}}",
                "switch {$x} {a {puts A}}",
            ] {
                assert!(
                    dispatch_operands(source, dialect).is_some(),
                    "{dialect}: {source}"
                );
            }
        }
    }

    /// A registry with no profile declares no target at all, so the rule reads
    /// no release there and the statement is flattened.
    #[test]
    fn a_registry_with_no_profile_reads_no_release_for_the_option_scan() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir("switch $x {a {puts A}}", &registry);
        let func =
            build_cfg_function_for_registry("::test", &module.top_level, false, &registry, false);
        assert!(matches!(
            func.blocks[&func.entry].terminator,
            Some(Terminator::Branch { .. })
        ));
    }

    #[test]
    fn try_finally_creates_finally_block() {
        let script = Script::from_statements(vec![Statement::Try {
            span: Span::new(0, 40),
            body: Script::new(),
            body_span: Span::new(4, 6),
            handlers: vec![],
            finally_body: Some(Script::from_statements(vec![Statement::Call {
                span: Span::new(20, 35),
                command: "cleanup".into(),
                canonical_command: None,
                args: vec![],
                defs: vec![],
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            }])),
            finally_span: Some(Span::new(18, 36)),
            raw_args: vec![],
        }]);

        let func = build_cfg_function("::test", &script, true);
        // Should have a try_finally block.
        assert!(
            func.blocks
                .values()
                .any(|b| b.name.starts_with("try_finally")),
            "should have try_finally block"
        );
    }

    #[test]
    fn try_with_handler() {
        let script = Script::from_statements(vec![Statement::Try {
            span: Span::new(0, 60),
            body: Script::new(),
            body_span: Span::new(4, 6),
            handlers: vec![TryHandler {
                kind: crate::ir::HandlerMatch::CompletionCode,
                match_arg: "error".into(),
                trap_pattern: None,
                var_name: Some("e".into()),
                options_var: None,
                body: Script::new(),
                body_span: Span::new(30, 35),
                fallthrough: false,
            }],
            finally_body: None,
            finally_span: None,
            raw_args: vec![],
        }]);

        let func = build_cfg_function("::test", &script, true);
        assert!(
            func.blocks
                .values()
                .any(|b| b.name.starts_with("try_handler")),
            "should have try_handler block"
        );
    }
}
