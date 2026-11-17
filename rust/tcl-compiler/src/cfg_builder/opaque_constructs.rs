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

//! Constructs kept as one opaque statement instead of blocks: frozen loops, opaque `foreach`, `catch` and `try`, and the effects of the scripts they keep inside.

use super::CfgBuilder;
use super::flow_facts::{Completion, flow_facts_stmt_with_classes};
use crate::expr_ast::ExprNode;
use crate::ir::{CommandTokens, Script, Statement};
use std::collections::BTreeSet;
use tcl_lexer::{Span, TokenType};
use tcl_registry::Traits;

/// Choose the [`CommandTokens`] for a "frozen" `while`/`for` runtime call.
///
/// The frozen-loop barrier hands the source words (the condition expression
/// text, the body script text) to the runtime `while` / `for` builtin, which
/// re-evaluates the condition and body on each iteration. Each word must be
/// pushed *as written*: a braced `{cond}` / `{body}` word verbatim
/// (substitution suppressed), a `$body` word interpolated. The codegen's
/// [`emit_call`] decides this from `argv_kinds[i] == Str && single_token_word`,
/// so the barrier needs the real per-word kinds.
///
/// `source` is the loop's recorded [`Statement::While::raw_tokens`] — the exact
/// segmenter token metadata. When present (always, for lowered loops) it is
/// used directly; the all-verbatim fallback ([`all_str_tokens`]) only covers
/// synthetically-constructed loops that carry no token metadata, matching the
/// braced-word idiom they are built from. Without this the words are pushed as
/// interpolated literals and the condition's command substitution is evaluated
/// *once* at the call site, freezing the loop (`while {[gets $f line] >= 0}`
/// happens to work, but a bare `while {[string length $x]}` would spin forever).
fn frozen_loop_tokens(cmd: &str, args: &[String], source: Option<&CommandTokens>) -> CommandTokens {
    source.cloned().unwrap_or_else(|| all_str_tokens(cmd, args))
}

/// Return `tokens` with the word at `idx` removed from every per-word vector.
///
/// Used to derive a `dict for`/`map` barrier's tokens from the source
/// `dict for {vars} $d {body}` tokens by dropping the `for`/`map` subcommand
/// word (index 1), so the remaining words line up with the barrier's
/// `::tcl::dict::for {vars} $d {body}` argv.
pub(super) fn drop_word(tokens: &CommandTokens, idx: usize) -> CommandTokens {
    let mut out = tokens.clone();
    if idx < out.argv.len() {
        out.argv.remove(idx);
    }
    if idx < out.argv_texts.len() {
        out.argv_texts.remove(idx);
    }
    if idx < out.word_exprs.len() {
        out.word_exprs.remove(idx);
    }
    if idx < out.argv_kinds.len() {
        out.argv_kinds.remove(idx);
    }
    if idx < out.single_token_word.len() {
        out.single_token_word.remove(idx);
    }
    if let Some(expand) = out.expand_word.as_mut()
        && idx < expand.len()
    {
        expand.remove(idx);
    }
    debug_assert!(out.words_align_with_argv_text());
    out
}

/// Synthesise [`CommandTokens`] marking every word as a single brace-string
/// token — the verbatim fallback for a frozen loop with no recorded source
/// tokens. See [`frozen_loop_tokens`].
fn all_str_tokens(cmd: &str, args: &[String]) -> CommandTokens {
    let word_count = 1 + args.len();
    let mut argv_texts = Vec::with_capacity(word_count);
    argv_texts.push(cmd.to_owned());
    argv_texts.extend(args.iter().cloned());
    CommandTokens::from_lossy_parts(
        vec![Span::new(0, 0); word_count],
        argv_texts,
        vec![TokenType::Str; word_count],
        vec![true; word_count],
        Vec::new(),
        None,
    )
}

impl CfgBuilder<'_> {
    /// What the commands the arms of the opaque `switch` `stmt` run do to this
    /// frame, beyond the writes the SSA reads off the statement itself
    /// ([`crate::ssa::switch_may_defs`]), as the statements that follow it
    /// ([`Self::opaque_script_effects`]).
    pub(super) fn opaque_arm_effects(&mut self, stmt: &Statement) -> Vec<Statement> {
        let Statement::Switch {
            arms, default_body, ..
        } = stmt
        else {
            return Vec::new();
        };
        let bodies: Vec<&Script> = arms
            .iter()
            .filter_map(|arm| arm.body.as_ref())
            .chain(default_body.as_ref())
            .collect();
        self.opaque_script_effects(
            stmt.span(),
            &bodies,
            BTreeSet::new(),
            true,
            "a command an arm of the switch runs may write any name",
        )
    }

    /// What the commands of the scripts a statement keeps inside itself do to
    /// this frame, as the statements to put beside it.
    ///
    /// The scripts stay inside the statement, so the scans of the statements the
    /// graph lowers never reach a command in one. Each is asked what the graph
    /// would have put beside it had it lowered the script
    /// ([`Self::apply_upvar_invalidation`], whose function-level facts this
    /// records too): the names a callee writes into this frame, which join
    /// `written` as the statement's may-definitions
    /// ([`crate::ir::SyntheticMarker::ArmWrites`]); a barrier for a command that
    /// may write any name (`namespace eval`, `dict with`, `eval $script`, a
    /// callee that aliases a computed name), unless the command leaves the
    /// procedure and `leaves_the_procedure` says that leaving is no resumption
    /// after the statement (an error inside a `catch` or a `try` resumes
    /// there); and a marker where code the module cannot see runs.
    fn opaque_script_effects(
        &mut self,
        span: Span,
        scripts: &[&Script],
        mut written: BTreeSet<String>,
        leaves_the_procedure: bool,
        barrier_reason: &str,
    ) -> Vec<Statement> {
        let mut barrier = false;
        for script in scripts {
            barrier |= crate::ssa::nested_writes(script, self.registry, &mut written);
        }
        let mut unseen = false;
        for script in scripts {
            crate::ir::for_each_statement(script, &mut |inner| {
                let own: &[String] = match inner {
                    Statement::Call { defs, .. } => defs,
                    _ => &[],
                };
                // A command that leaves the procedure (`exit`, `error`) writes
                // nothing the code after the statement can read.
                let exits = leaves_the_procedure
                    && flow_facts_stmt_with_classes(inner, &self.command_classes).1
                        == Completion::ProcExit;
                for produced in self.apply_upvar_invalidation(inner.clone()) {
                    match &produced {
                        _ if crate::ssa::is_unseen_call_marker(&produced) => unseen = true,
                        Statement::Barrier { .. } | Statement::UpFrame { .. } => {
                            barrier |= !exits;
                        }
                        Statement::Call { defs, .. } => {
                            written.extend(defs.iter().filter(|name| !own.contains(name)).cloned());
                        }
                        _ => {}
                    }
                }
            });
        }
        let mut effects = Vec::new();
        if !written.is_empty() {
            effects.push(Self::arm_writes_marker(span, written));
        }
        if barrier {
            effects.push(Self::caller_frame_opaque(span, barrier_reason.to_owned()));
        }
        if unseen {
            effects.push(Statement::unseen_call_marker(span));
        }
        effects
    }

    /// The statement that states the names a statement keeps its scripts in
    /// may write ([`crate::ir::SyntheticMarker::ArmWrites`]).
    fn arm_writes_marker(span: Span, names: BTreeSet<String>) -> Statement {
        Statement::Call {
            span,
            command: "<arm-writes>".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            defs: names.into_iter().collect(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::ArmWrites,
            )),
            foreach_groups: None,
        }
    }

    /// A widening *effect*, not a command to run: every tracked value is
    /// widened where it stands ([`crate::ir::SyntheticMarker::CallerFrameOpaque`]).
    pub(super) fn caller_frame_opaque(span: Span, reason: String) -> Statement {
        Statement::Barrier {
            span,
            reason,
            command: "<caller-frame-opaque>".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::CallerFrameOpaque,
            )),
        }
    }

    /// Push a "frozen" loop (`for` / `while` whose condition is a command
    /// substitution) into `current` as an opaque [`Statement::Barrier`]; the
    /// body is kept un-lowered so it is treated as an opaque effect.
    ///
    /// A `Statement::Barrier` has no `defs` field — its own
    /// [`crate::ssa::uses_of`] textually scans *both* `condition` and the
    /// un-lowered body text for `$var` reads (attributing them all to the
    /// barrier's own span), but nothing analogous computes defs for it
    /// beyond the registry's ordinary `VarWrite` role query, which `for` /
    /// `while` themselves never declare. So a `catch`/`regexp`/`scan`
    /// result var, or a known upvar/global-writing user proc's write,
    /// reached only through the frozen condition (`while {[getopt argv
    /// $opts opt arg]} { ... }`, tcllib's `cmdline::getoptions`) would be
    /// invisible to the def-use graph: the read inside
    /// the (un-lowered, but still textually-scanned) body looked
    /// read-before-set even though the condition's own command
    /// substitution completes — including the write — before the body
    /// (or even a second evaluation of the condition) ever runs. Fixed by
    /// pushing a synthetic `<cond>` `Statement::Call` carrying those defs
    /// immediately before the barrier, exactly mirroring the non-frozen
    /// `lower_if`/`lower_while` path's own `condition_out_vars` use — the
    /// barrier's textually-scanned read then resolves to this new SSA
    /// version instead of the undef origin.
    fn push_frozen_loop_barrier(
        &mut self,
        command: &str,
        condition: &ExprNode,
        raw_args: &[String],
        raw_tokens: Option<&CommandTokens>,
        span: Span,
        current: &str,
    ) {
        self.push_condition_effects(condition, span, current);
        self.push_statement(
            current,
            Statement::Barrier {
                span,
                reason: format!("frozen {command} (cmd-subst condition)"),
                command: command.to_owned(),
                canonical_command: None,
                args: raw_args.to_vec(),
                tokens: Some(frozen_loop_tokens(command, raw_args, raw_tokens)),
            },
        );
    }

    /// Lower a `for`, or freeze it as an opaque barrier when its condition is a
    /// command substitution (`for {…} [cond] {…} {…}`).
    pub(super) fn lower_for_or_frozen(
        &mut self,
        stmt: &Statement,
        current: &str,
    ) -> Option<String> {
        if let Statement::For {
            condition,
            raw_args,
            raw_tokens,
            span,
            ..
        } = stmt
            && matches!(condition, ExprNode::Command { .. })
            && !raw_args.is_empty()
        {
            self.push_frozen_loop_barrier(
                "for",
                condition,
                raw_args,
                raw_tokens.as_deref(),
                *span,
                current,
            );
            Some(current.to_owned())
        } else {
            self.lower_for(stmt, current)
        }
    }

    /// Lower a `while`, or freeze it as an opaque barrier when its condition is
    /// a command substitution (`while [cond] {…}`).
    pub(super) fn lower_while_or_frozen(&mut self, stmt: &Statement, current: &str) -> String {
        if let Statement::While {
            condition,
            raw_args,
            raw_tokens,
            span,
            ..
        } = stmt
            && matches!(condition, ExprNode::Command { .. })
            && !raw_args.is_empty()
        {
            self.push_frozen_loop_barrier(
                "while",
                condition,
                raw_args,
                raw_tokens.as_ref(),
                *span,
                current,
            );
            current.to_owned()
        } else {
            self.lower_while(stmt, current)
        }
    }

    /// List operands run before loop-variable binding or any body entry.
    /// Preserve the original global-frame and registry-barrier order here.
    fn record_foreach_operand_barriers(&mut self, stmt: &Statement, current: &str) {
        let list_effects = self.embedded_subst_extras(stmt);
        if list_effects.unseen {
            self.push_statement(current, Statement::unseen_call_marker(stmt.span()));
        }
        if list_effects.opaque_global {
            self.block_mut(current).statements.push(Statement::Barrier {
                span: stmt.span(),
                reason: "foreach list runs an unreadable script at the global frame".to_owned(),
                command: "<global-frame-script>".to_owned(),
                canonical_command: None,
                args: Vec::new(),
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::GlobalFrameScript,
                )),
            });
        }

        // Foreach/lmap list words are evaluated before the header binds any
        // iteration variables. A substitution there can reach the registry
        // fallback just like a substitution in an ordinary value or
        // condition, so widen scalar facts before the header executes.
        if self.embedded_registry_barrier(stmt) {
            self.block_mut(current)
                .statements
                .push(Self::registry_barrier_statement(
                    stmt,
                    "foreach list evaluation reaches a registry-declared evaluation barrier",
                ));
        }
    }

    /// Dispatch `Foreach` — dict for/map, opaque top-level, or inlined.
    pub(super) fn lower_foreach_dispatch(&mut self, stmt: &Statement, current: &str) -> String {
        if self.lower_foreach_opaque(stmt, current) {
            return current.to_owned();
        }
        self.lower_foreach(stmt, current)
    }

    /// Lowers the opaque `foreach` shapes and reports whether it did; a leaf so
    /// the barrier temporaries stay out of the recursive lowering frame.
    #[inline(never)]
    fn lower_foreach_opaque(&mut self, stmt: &Statement, current: &str) -> bool {
        let Statement::Foreach {
            is_dict_iteration,
            is_array_iteration,
            raw_args,
            raw_tokens,
            iterators,
            span,
            ..
        } = stmt
        else {
            unreachable!();
        };

        self.record_foreach_operand_barriers(stmt, current);

        // `array for {k v} arr body` (Tcl 9.0): the body runs in the caller's
        // frame, so the analysis CFG inlines it (shared reaching-defs / const
        // lattice, loop vars bound) while codegen barriers it to the
        // `::tcl::array::for` ensemble invoke the array hook resolves — keeping
        // the emitted bytecode byte-identical to C Tcl.
        if *is_array_iteration {
            if self.faithful_exceptions {
                return false;
            }
            self.push_statement(
                current,
                Statement::Barrier {
                    span: *span,
                    reason: "array for".into(),
                    command: "array".into(),
                    canonical_command: None,
                    args: raw_args.clone(),
                    tokens: raw_tokens.clone(),
                },
            );
            return true;
        }

        if *is_dict_iteration && !raw_args.is_empty() {
            // `dict for`/`dict map {k v} $d body`: the body runs in the caller's
            // frame, so the analysis CFG inlines it (shared reaching-defs / const
            // lattice, loop vars bound, and — crucially — the body's *own* control
            // flow lowered into real blocks, so a read nested inside an `if` /
            // `while` / `catch` body is a first-class SSA use). Codegen barriers it
            // to the `::tcl::dict::for`/`::tcl::dict::map` ensemble invoke below,
            // keeping the emitted bytecode byte-identical to C Tcl. Mirrors the
            // `array for` split above. Without the inlined body, a command-name or
            // brace-nested `$var` read the shallow barrier-word scan can't see is
            // lost, so the loop's outer reads look dead.
            if self.faithful_exceptions {
                return false;
            }
            let sub = &raw_args[0];
            let qual_cmd = crate::naming::rooted_member_key("::tcl::dict", sub);
            // The barrier re-emits `::tcl::dict::for {vars} $dict {body}` — the
            // source `dict for …` tokens with the `for`/`map` subcommand word
            // dropped — so the braced var-list / body push verbatim and the
            // `$dict` argument substitutes (else the body's command subs are
            // evaluated once, before the loop variables exist).
            let args = raw_args[1..].to_vec();
            let tokens = raw_tokens
                .as_ref()
                .map_or_else(|| all_str_tokens(&qual_cmd, &args), |t| drop_word(t, 1));
            self.push_statement(
                current,
                Statement::Barrier {
                    span: *span,
                    reason: "dict for/map".into(),
                    command: qual_cmd,
                    canonical_command: None,
                    args,
                    tokens: Some(tokens),
                },
            );
            return true;
        }

        let has_qualified_vars = iterators
            .iter()
            .any(|it| it.vars.iter().any(|v| tcl_syntax::naming::is_absolute(v)));

        // The original Each compiler owns local declarations, value-script
        // visits and the body range. Flattening it here would reserve body
        // locals before visiting the original value operands. Analysis keeps
        // its structural graph; executable emission consumes the exact source
        // once through the independently admitted compiler.
        let original_each =
            !self.faithful_exceptions
                && !self.plain_command_dispatch
                && raw_tokens.as_ref().is_some_and(|tokens| {
                    tokens.argv_texts.get(1..) == Some(raw_args.as_slice())
                        && tokens
                            .source_binding
                            .as_ref()
                            .and_then(|binding| binding.original_structured_compilation(tokens))
                            .is_some_and(|preparation| {
                                matches!(
                        preparation.recipe(),
                        tcl_registry::native_instruction_plan::NativeInstructionPlan::Each(_)
                    )
                            })
                });

        if original_each || (!self.inline_loops && !raw_args.is_empty()) || has_qualified_vars {
            self.push_opaque_loop_call(stmt, current);
            return true;
        }

        false
    }

    /// The loop as one call. Its body runs zero or more times in this frame,
    /// so what it may write the call may write, and a value from before the
    /// loop can survive it: each such name is read before it is written
    /// (#2385). Writes the analysis cannot name end at a registry barrier.
    fn push_opaque_loop_call(&mut self, stmt: &Statement, current: &str) {
        let Statement::Foreach {
            raw_args,
            raw_tokens,
            iterators,
            is_lmap,
            body,
            span,
            ..
        } = stmt
        else {
            unreachable!();
        };
        let cmd = if *is_lmap { "lmap" } else { "foreach" };
        let mut defs: Vec<String> = iterators.iter().flat_map(|it| it.vars.clone()).collect();
        let mut body_writes = std::collections::BTreeSet::new();
        let unseen_writes = crate::ssa::nested_writes(body, self.registry, &mut body_writes);
        let reads: Vec<String> = body_writes
            .into_iter()
            .filter(|name| !defs.contains(name))
            .collect();
        defs.extend(reads.iter().cloned());
        self.push_statement(
            current,
            Statement::Call {
                span: *span,
                command: cmd.into(),
                canonical_command: None,
                args: raw_args.clone(),
                defs,
                reads,
                reads_own_defs: false,
                safe_on_uninit: true,
                tokens: Some(frozen_loop_tokens(cmd, raw_args, raw_tokens.as_ref())),
                foreach_groups: None,
            },
        );
        if unseen_writes {
            self.push_statement(
                current,
                Self::registry_barrier_statement(
                    stmt,
                    "loop body writes variables the analysis cannot name",
                ),
            );
        }
    }

    /// Dispatch `Catch` — inlined into real blocks, or deferred opaque.
    ///
    /// Mirrors [`Self::lower_try_dispatch`]. Inlining is what lets the
    /// ordinary emitters compile the body, so its variables reach the LVT and
    /// the optimiser can see inside it (#2207); the opaque arm stays for the
    /// shapes that cannot be lowered faithfully.
    ///
    /// `{*}` expansion keeps the opaque path: the body word is re-parsed by
    /// the inline emitters, and `parse_cmd_parts` splits an adjacent
    /// `{*}$args` into a literal `*` and `$args`, changing the callee's argv.
    /// A qualified result/options variable keeps it too — those are not frame
    /// locals, so the store the inline form emits would address the wrong
    /// variable.
    pub(super) fn lower_catch_dispatch(&mut self, stmt: &Statement, current: &str) -> String {
        self.push_embedded_control_effects(
            stmt,
            current,
            "catch header invokes an opaque embedded command",
        );
        let Statement::Catch { body, raw_args, .. } = stmt else {
            unreachable!();
        };

        let expands = raw_args.iter().any(|arg| arg.contains("{*}"));
        // The destination words have to be written as plain scalar locals.
        // Lowering normalises `$dst` to `dst` and `a(key)` to `a`, so the
        // names alone no longer say what was written: inlining
        // `catch {…} $dst` would store into `dst` rather than into the
        // variable it names, and `catch {…} a(key)` into a scalar `a`.
        let indirect_destination = raw_args
            .iter()
            .skip(1)
            .any(|word| !is_plain_local_destination(word));

        // Semantic expansion is independent of the backend's local-table and
        // one-block emitter restrictions. Invalid/dynamic argv stays opaque;
        // a count the release's `catch` refuses never lowers to a `Catch`.
        if self.faithful_exceptions && !raw_args.is_empty() && !expands && !indirect_destination {
            return self.lower_catch(stmt, current);
        }

        if !self.is_proc_body
            || raw_args.is_empty()
            || body.statements.is_empty()
            || expands
            || indirect_destination
            || stmt
                .tokens()
                .and_then(|tokens| {
                    let context = self
                        .registry
                        .profile()
                        .map(tcl_registry::model::semantic::SemanticContext::for_profile);
                    crate::registry_invocation::normal_transfer_invocation(
                        self.registry,
                        context,
                        tokens,
                    )
                })
                .and_then(|normal| normal.variable_output_arguments())
                .is_none()
            || !self.catch_body_is_one_block(body)
        {
            self.emit_opaque_catch(stmt, current);
            return current.to_owned();
        }

        self.lower_catch(stmt, current)
    }

    /// Whether a `catch` body lowers to a single straight-line block.
    ///
    /// The inline emitter compiles the body itself, so that it can leave the
    /// last statement's value on the stack for `catch`'s result variable
    /// rather than popping it as the ordinary block walk would. That only
    /// works while the body *is* one block. A final call stays inside that
    /// block and its captured range, including its independently guarded
    /// runtime fallback. A terminating call followed by more commands, or a
    /// structured/return node, requires a complete multi-block exception range.
    ///
    /// The terminator set is the registry's [`Traits::TERMINATES_BLOCK`], not
    /// a name list here. Nested control flow needs its own blocks for the
    /// same reason and is rejected too.
    fn catch_body_is_one_block(&self, body: &Script) -> bool {
        body.statements
            .iter()
            .enumerate()
            .all(|(index, s)| match s {
                Statement::If { .. }
                | Statement::For { .. }
                | Statement::While { .. }
                | Statement::Foreach { .. }
                | Statement::Catch { .. }
                | Statement::Try { .. }
                | Statement::Switch { .. }
                | Statement::Block { .. }
                | Statement::UpFrame { .. }
                | Statement::Barrier { .. }
                | Statement::Return { .. } => false,
                Statement::Call { command, .. } => {
                    let terminates = self
                        .registry
                        .get(command)
                        .is_some_and(|spec| spec.traits.contains(Traits::TERMINATES_BLOCK));
                    !terminates || index + 1 == body.statements.len()
                }
                _ => true,
            })
    }

    /// Emit an opaque `catch` call, with the effects of its body ahead of it.
    ///
    /// The body runs inside the statement and stops at its first error, so every
    /// name it writes is a may-definition, and what its commands do to the frame
    /// ([`Self::opaque_script_effects`]) goes ahead of the call, which assigns
    /// its result and options variables once the body has finished.
    fn emit_opaque_catch(&mut self, stmt: &Statement, current: &str) {
        let Statement::Catch {
            body,
            result_var,
            options_var,
            raw_args,
            span,
            tokens,
            ..
        } = stmt
        else {
            unreachable!();
        };

        let mut written = BTreeSet::new();
        crate::ssa::nested_writes(body, self.registry, &mut written);
        for effect in self.in_barrier_block(current, |builder| {
            builder.opaque_script_effects(
                *span,
                &[body],
                written,
                false,
                "a command the catch body runs may write any name",
            )
        }) {
            self.block_mut(current).statements.push(effect);
        }
        let mut catch_defs: Vec<String> = result_var.iter().chain(options_var).cloned().collect();
        dedup_preserve_order(&mut catch_defs);
        // Preserve ``tokens`` on the synthetic ``Statement::Call``
        // so the codegen's eval-fallback can detect the braced
        // body word and re-wrap it in ``{…}`` when reconstructing
        // the script for ``tcl_eval``.  Without this,
        // ``catch {$undef} msg`` would lower to ``catch $undef
        // msg`` and the var-read trap would fire before catch
        // could intercept it.
        self.push_statement(
            current,
            Statement::Call {
                span: *span,
                command: "catch".into(),
                canonical_command: None,
                args: raw_args.clone(),
                defs: catch_defs,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: tokens.clone(),
                foreach_groups: None,
            },
        );
        self.opaque_catch_bodies.insert(*span, body.clone());
    }

    /// Dispatch `Try` — deferred opaque or inlined.
    pub(super) fn lower_try_dispatch(&mut self, stmt: &Statement, current: &str) -> String {
        // Header words substitute before either the generic invocation or
        // the inlined body begins, and may mutate this frame through a handler.
        self.push_embedded_control_effects(
            stmt,
            current,
            "try header invokes an opaque embedded command",
        );
        let Statement::Try {
            handlers,
            finally_body,
            raw_args,
            body,
            span,
            ..
        } = stmt
        else {
            unreachable!();
        };

        let defer = !self.inline_loops
            && !raw_args.is_empty()
            && (!handlers.is_empty() || finally_body.is_none());

        if defer {
            // As an opaque `catch`: what the body, the handlers and the
            // `finally` script write is a may-definition, since each stops at
            // its first error, and a handler's variables are set only when it
            // runs.
            let mut written = BTreeSet::new();
            let mut scripts: Vec<&Script> = vec![body];
            for handler in handlers {
                written.extend(handler.var_name.iter().cloned());
                written.extend(handler.options_var.iter().cloned());
                scripts.push(&handler.body);
            }
            scripts.extend(finally_body.as_ref());
            for script in &scripts {
                crate::ssa::nested_writes(script, self.registry, &mut written);
            }
            let mut try_defs: Vec<_> = written.iter().cloned().collect();
            for effect in self.in_barrier_block(current, |builder| {
                builder.opaque_script_effects(
                    *span,
                    &scripts,
                    written,
                    false,
                    "a command the try body or its handlers run may write any name",
                )
            }) {
                self.block_mut(current).statements.push(effect);
            }
            dedup_preserve_order(&mut try_defs);
            self.push_statement(
                current,
                Statement::Call {
                    span: *span,
                    command: "try".into(),
                    canonical_command: None,
                    args: raw_args.clone(),
                    defs: try_defs,
                    reads: vec![],
                    reads_own_defs: false,
                    safe_on_uninit: false,
                    tokens: None,
                    foreach_groups: None,
                },
            );
            return current.to_owned();
        }

        self.lower_try(stmt, current)
    }
}

/// Whether a `catch` destination word is a plain scalar frame local, written
/// literally.
///
/// Anything needing substitution (`$dst`, `[cmd]`), an array element
/// (`a(key)`), a qualified name (`::ns::v`) or an empty word keeps the opaque
/// path, where the runtime resolves the word itself.
fn is_plain_local_destination(word: &str) -> bool {
    !word.is_empty()
        && !tcl_syntax::naming::is_qualified(word.as_bytes())
        && !word.contains(['$', '[', ']', '(', ')', '{', '}', '\\', ' ', '\t', '\n'])
}

/// Deduplicate a `Vec` while preserving first-occurrence order.
pub(super) fn dedup_preserve_order(v: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    v.retain(|item| seen.insert(item.clone()));
}
