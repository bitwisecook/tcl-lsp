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

//! Inline `try`: `on error` handlers with their bindings and `-during` merge, and `finally`.

use super::super::cmd_subst::parse_script_parts_with_config;
use super::super::{CodegenCtx, Op, Operand, bytecode_imm};
use super::{InlineBodyEmitter, TryHandlerBindings, is_inline_local_scalar_name};
use tcl_compiler::cfg::Function as CfgFunction;
use tcl_compiler::ir::Statement;
use tcl_registry::TryClauseKind;
use tcl_registry::completion::CompletionCode;
use tcl_registry::hooks::{InlineCodegenHookId, LoweringHookId};

impl CodegenCtx<'_> {
    /// Resolve the one `try body on error variableList handler` shape this
    /// inline emitter implements.
    ///
    /// The registry owns the typed clause grammar and Tcl's shared list codec
    /// owns `variableList`. This consumer only adds its code-generation proof:
    /// body and handler must be closed braced script values, and bound names
    /// must fit procedure-local scalar bytecodes. Anything else declines to
    /// the generic, yield-aware runtime `try` implementation.
    pub(super) fn try_on_error_inline_bindings(
        &self,
        command: &str,
        args: &[(String, bool)],
    ) -> Option<TryHandlerBindings> {
        let arg_refs: Vec<&str> = args.iter().map(|(arg, _)| arg.as_str()).collect();
        let invocation = self.registry.try_control_invocation(
            command,
            &arg_refs,
            self.invocation_surface_query(),
        )?;
        let [clause] = invocation.clauses.as_slice() else {
            return None;
        };
        if !matches!(clause.kind, TryClauseKind::On(selector) if selector.matches_route(
            tcl_registry::completion_route::InvocationCompletionRoute::Tcl(CompletionCode::Error)
        ) == Some(true))
            || clause.fallthrough
            || !args[invocation.body_index].1
            || !args[clause.body_index].1
        {
            return None;
        }
        let variable_list_index = clause.variable_list_index?;
        if tcl_syntax::naming::word_is_dynamic(
            &args[variable_list_index].0,
            args[variable_list_index].1,
        ) || (!args[variable_list_index].1 && args[variable_list_index].0.contains('\\'))
        {
            return None;
        }

        let names = self
            .word_rules
            .split_list(&args[variable_list_index].0)
            .ok()?;
        let result_var = names.first().map(ToString::to_string);
        let options_var = names.get(1).map(ToString::to_string);
        // Empty receivers have engine-specific output semantics and stay generic.
        if result_var
            .iter()
            .chain(options_var.iter())
            .any(|name| !is_inline_local_scalar_name(name))
        {
            return None;
        }

        Some(TryHandlerBindings {
            body_index: invocation.body_index,
            handler_body_index: clause.body_index,
            result_var,
            options_var,
        })
    }

    /// Emit the selected original Try statement using the same exception and
    /// body-entry owners as a Try nested in a protected command substitution.
    pub(in super::super) fn emit_selected_try_statement(
        &mut self,
        command: &str,
        values: &[String],
        tokens: Option<&tcl_compiler::ir::CommandTokens>,
    ) -> bool {
        if self.native_entry.is_some() || !self.compiles_locals() {
            return false;
        }
        // CFG Try calls can omit their local token field while retaining the
        // original invocation tokens in emit_stmt's source scope.
        let original = self.invocation_tokens.clone();
        let source_tokens = tokens.or(original.as_deref());
        if source_tokens
            .and_then(|tokens| tokens.expand_word.as_ref())
            .is_some_and(|words| words.iter().any(|expanded| *expanded))
        {
            return false;
        }
        let args = values
            .iter()
            .enumerate()
            .map(|(index, argument)| {
                let braced = source_tokens.is_some_and(|tokens| {
                    tokens.argv_kinds.get(index + 1) == Some(&tcl_lexer::TokenType::Str)
                        && tokens
                            .single_token_word
                            .get(index + 1)
                            .copied()
                            .unwrap_or(false)
                });
                (argument.clone(), braced)
            })
            .collect::<Vec<_>>();
        if self.inline_cmd_subst_hook_candidate(command, &args) != Some(InlineCodegenHookId::Try)
            || self.try_on_error_inline_bindings(command, &args).is_none()
        {
            return false;
        }
        // Retain the exact command binding only after the source-form guard.
        if self.inline_cmd_subst_hook(command, &args) != Some(InlineCodegenHookId::Try) {
            return false;
        }
        let exit = self.fresh_label("try_statement_end");
        self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(exit.clone()), Operand::Imm(1)],
            "",
        );
        self.cmd_index += 1;
        self.emit_try_on_error_inline(command, &args, &exit);
        self.place_label(&exit);
        self.emit(Op::POP, vec![]);
        self.seen_generic_invoke = true;
        true
    }

    // Inline try/on error compilation.

    /// Emit the `-during` merge sequence: load saved opts, prepend
    /// the new error opts, dict-set `-during` key, store back to
    /// temps slot.  Extracted from
    /// [`Self::emit_try_on_error_inline`].
    fn emit_try_during_merge(&mut self, temp_opts_slot: i32) {
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit(Op::POP, vec![]);
        self.push_lit("-during");
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        self.emit_comment(
            Op::DICT_SET,
            vec![Operand::Imm(1), Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
    }

    /// Emit the no-match (code != 1) re-raise path: pop the
    /// dispatch flag, reload saved opts + result, and `RETURN_STK`.
    fn emit_try_no_match_reraise(&mut self, temp_opts_slot: i32, temp_result_slot: i32) {
        self.emit(Op::POP, vec![]);
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit_comment(
            Op::LOAD_SCALAR1,
            vec![Operand::Imm(temp_result_slot)],
            &format!("temp var {temp_result_slot}"),
        );
        self.emit(Op::RETURN_STK, vec![]);
    }

    /// Begin one inline exception range and advance the shared range index.
    fn begin_inline_catch_range(&mut self) -> usize {
        let begin = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;
        begin
    }

    /// Bind the caught error result/options to the handler's declared local
    /// variables. The typed try-clause parser owns which names are present;
    /// this helper owns their bytecode transfer from the shared temp slots.
    fn emit_try_error_bindings(
        &mut self,
        bindings: &TryHandlerBindings,
        result_slot: Option<i32>,
        options_slot: Option<i32>,
        temp_result_slot: i32,
        temp_opts_slot: i32,
    ) {
        if let (Some(name), Some(slot)) = (&bindings.result_var, result_slot) {
            self.emit_comment(
                Op::LOAD_SCALAR1,
                vec![Operand::Imm(temp_result_slot)],
                &format!("temp var {temp_result_slot}"),
            );
            self.emit_comment(
                Op::STORE_SCALAR1,
                vec![Operand::Imm(slot)],
                &format!("var \"{name}\""),
            );
            self.emit(Op::POP, vec![]);
        }
        if let (Some(name), Some(slot)) = (&bindings.options_var, options_slot) {
            self.emit_comment(
                Op::LOAD_SCALAR1,
                vec![Operand::Imm(temp_opts_slot)],
                &format!("temp var {temp_opts_slot}"),
            );
            self.emit_comment(
                Op::STORE_SCALAR1,
                vec![Operand::Imm(slot)],
                &format!("var \"{name}\""),
            );
            self.emit(Op::POP, vec![]);
        }
    }

    /// Emit inline `try { body } on error {var} { handler }` bytecodes.
    pub fn emit_try_on_error_inline(
        &mut self,
        command: &str,
        args: &[(String, bool)],
        normal_exit: &str,
    ) {
        let bindings = self
            .try_on_error_inline_bindings(command, args)
            .expect("caller proved the typed literal try/on-error shape");
        let try_body_text = &args[bindings.body_index].0;
        let handler_body_text = &args[bindings.handler_body_index].0;

        // Allocate LVT slots in tclsh order
        let msg_slot = bindings.result_var.as_ref().and_then(|name| {
            self.command_variable_slot(name.as_bytes())
                .map(bytecode_imm)
        });
        let handler_opts_slot = bindings.options_var.as_ref().and_then(|name| {
            self.command_variable_slot(name.as_bytes())
                .map(bytecode_imm)
        });
        let temp_result_name = format!("#temp{}", self.catch_depth);
        let temp_opts_name = format!("#temp{}", self.catch_depth + 1);
        let temp_result_slot = bytecode_imm(self.lvt.intern_synthetic(&temp_result_name));
        let temp_opts_slot = bytecode_imm(self.lvt.intern_synthetic(&temp_opts_name));

        let initial_depth = self.catch_depth;

        // Try body exception range
        let try_begin_idx = self.begin_inline_catch_range();

        // Compile the complete try body in this activation so a yield freezes
        // this phase's live exception range.
        self.with_inline_body_operand(try_body_text, bindings.body_index, |ctx| {
            ctx.emit_segmented_inline_body(try_body_text, InlineBodyEmitter::Catch);
        });

        // Normal exit from try body
        self.emit(Op::END_CATCH, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // Exception handler for try body. The out-of-band target is the VM's
        // analogue of C Tcl's exception-range catch offset.
        let try_exception = self.fresh_label("try_on_body_exception");
        self.place_label(&try_exception);
        self.instructions[try_begin_idx].catch_target = Some(try_exception);
        self.emit(Op::PUSH_RETURN_CODE, vec![]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // Store return opts and result to temps
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(temp_opts_slot)],
            &format!("temp var {temp_opts_slot}"),
        );
        self.emit(Op::POP, vec![]);
        self.emit_comment(
            Op::STORE_SCALAR1,
            vec![Operand::Imm(temp_result_slot)],
            &format!("temp var {temp_result_slot}"),
        );
        self.emit(Op::POP, vec![]);

        // Return code dispatch: check if code == 1 (TCL_ERROR)
        self.emit(Op::DUP, vec![]);
        self.push_lit("1");
        self.emit(Op::EQ, vec![]);
        let no_match = self.fresh_label("try_on_nomatch");
        self.emit_comment(
            Op::JUMP_FALSE4,
            vec![Operand::Label(no_match.clone())],
            "try_on",
        );

        // Matched error handler (code == 1)
        self.emit(Op::POP, vec![]);
        self.emit_try_error_bindings(
            &bindings,
            msg_slot,
            handler_opts_slot,
            temp_result_slot,
            temp_opts_slot,
        );

        // Handler body exception range
        let handler_begin_idx = self.begin_inline_catch_range();

        // Compile handler body
        self.with_inline_body_operand(handler_body_text, bindings.handler_body_index, |ctx| {
            ctx.emit_try_handler_body(handler_body_text);
        });

        // Normal exit from handler body
        self.emit(Op::END_CATCH, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // Exception handler for handler body.
        let handler_exception = self.fresh_label("try_on_handler_exception");
        self.place_label(&handler_exception);
        self.instructions[handler_begin_idx].catch_target = Some(handler_exception);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RETURN_CODE, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // Check if handler exception code == 1 for -during merge
        self.push_lit("1");
        self.emit(Op::EQ, vec![]);
        let shared_cleanup = self.fresh_label("try_on_cleanup");
        self.emit(
            Op::JUMP_FALSE1,
            vec![Operand::Label(shared_cleanup.clone())],
        );

        // -during merge
        self.emit_try_during_merge(temp_opts_slot);

        // Shared cleanup
        self.place_label(&shared_cleanup);
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        self.emit(Op::RETURN_STK, vec![]);
        self.emit_comment(
            Op::JUMP4,
            vec![Operand::Label(normal_exit.to_owned())],
            "try_on",
        );

        // No match (code != 1): re-raise
        self.place_label(&no_match);
        self.emit_try_no_match_reraise(temp_opts_slot, temp_result_slot);

        // Restore catch depth
        self.catch_depth = initial_depth;
    }

    /// Compile a complete try/on handler body inline with `startCommand` at
    /// each command boundary.
    pub fn emit_try_handler_body(&mut self, body_text: &str) {
        self.emit_segmented_inline_body(body_text, InlineBodyEmitter::TryHandler);
    }

    /// Compile one command of a try/on handler body.
    pub(super) fn emit_try_handler_command(&mut self, body_text: &str) {
        let Some(parts) = parse_script_parts_with_config(body_text, self.lexer_config()) else {
            self.emit_script_eval(body_text);
            self.seen_generic_invoke = true;
            return;
        };
        if parts.is_empty() {
            self.push_lit("");
            return;
        }

        let cmd = &parts[0].0;
        let cmd_args = &parts[1..];

        let sc_label = self.fresh_label("handler_body_end");
        self.emit_comment(
            Op::START_CMD,
            vec![Operand::Label(sc_label.clone()), Operand::Imm(1)],
            "",
        );
        self.cmd_index += 1;

        if super::super::emitter::bytecoded::try_value_bytecoded(self, cmd, cmd_args) {
            self.place_label(&sc_label);
            return;
        }

        let arg_refs: Vec<&str> = cmd_args.iter().map(|(arg, _)| arg.as_str()).collect();
        let inline_lowering = self.inline_lowering_hook(cmd, &arg_refs);
        match inline_lowering {
            // The registry's typed lowering hook proves that this is the
            // two-argument `set` shape, but its first word is still a Tcl
            // variable-name word. The specialised STORE_SCALAR1 path cannot
            // perform Tcl's dynamic name substitution (e.g. `set $v value`),
            // so decline it unless the shared scalar-name shape proves the
            // target is a procedure-local name.
            Some((LoweringHookId::Set, binding))
                if cmd_args.len() == 2
                    && !tcl_syntax::naming::word_is_dynamic(&cmd_args[0].0, cmd_args[0].1)
                    && is_inline_local_scalar_name(&cmd_args[0].0) =>
            {
                self.require_command_binding(&binding);
                self.emit_native_argument_word(1, &cmd_args[1].0, cmd_args[1].1);
                self.store_var(&cmd_args[0].0);
            }
            _ => {
                self.emit_generic_cmd_subst(cmd, cmd_args);
            }
        }

        self.place_label(&sc_label);
    }

    // Inline try/finally compilation.

    /// Emit inline `try { body } finally { cleanup }` bytecodes.
    pub fn emit_try_finally_inline(
        &mut self,
        cfg: &CfgFunction,
        try_body_name: &str,
        try_finally_name: &str,
    ) {
        let body_blk = cfg
            .block_by_name(try_body_name)
            .expect("try body block present");
        let finally_blk = cfg
            .block_by_name(try_finally_name)
            .expect("try finally block present");

        // try body
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        for stmt in &body_blk.statements {
            self.emit_try_body_stmt(stmt);
        }

        // Transition: jump over pushResult, converge at pushReturnOpts + endCatch
        let conv = self.fresh_label("try_conv");
        self.emit(Op::JUMP1, vec![Operand::Label(conv.clone())]);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.place_label(&conv);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // finally body
        self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        for stmt in &finally_blk.statements {
            self.emit_try_finally_stmt(stmt);
        }

        // Normal exit from finally
        self.emit(Op::END_CATCH, vec![]);
        self.emit(Op::POP, vec![]);
        let normal_exit = self.fresh_label("try_normal");
        self.emit(Op::JUMP1, vec![Operand::Label(normal_exit.clone())]);

        // Exception handler for the finally body
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        self.emit(Op::PUSH_RETURN_CODE, vec![]);
        self.emit(Op::END_CATCH, vec![]);

        // Check return code == 1 for -during merge
        self.push_lit("1");
        self.emit(Op::EQ, vec![]);
        let shared_cleanup = self.fresh_label("try_cleanup");
        self.emit(
            Op::JUMP_FALSE1,
            vec![Operand::Label(shared_cleanup.clone())],
        );

        // -during option merging
        self.push_lit("-during");
        self.emit(Op::OVER, vec![Operand::Imm(3)]);
        self.emit(Op::LIST, vec![Operand::Imm(2)]);
        self.emit(Op::LIST_CONCAT, vec![]);

        // Shared cleanup
        self.place_label(&shared_cleanup);
        self.emit(Op::REVERSE, vec![Operand::Imm(4)]);
        self.emit(Op::POP, vec![]);
        self.emit(Op::POP, vec![]);
        let return_target = self.fresh_label("try_return");
        self.emit(Op::JUMP1, vec![Operand::Label(return_target.clone())]);

        // Normal exit: swap result and return opts
        self.place_label(&normal_exit);
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);

        // Return / re-raise
        self.place_label(&return_target);
        self.emit(Op::RETURN_STK, vec![]);

        // Restore catch depth
        self.catch_depth -= 2;
    }

    /// Emit a statement using the same selected native hook and original-word
    /// protocol inside the try body's protected compiler context.
    pub fn emit_try_body_stmt(&mut self, stmt: &Statement) {
        self.emit_stmt_with_start_cmd(stmt, None, None);
        // Remove trailing pop — result stays on stack
        if self.instructions.last().is_some_and(|i| i.op == Op::POP) {
            self.instructions.pop();
        }
    }

    /// Emit a statement in finally-body context.
    pub fn emit_try_finally_stmt(&mut self, stmt: &Statement) {
        self.emit_stmt_with_start_cmd(stmt, None, None);
        // Remove trailing pop — result stays on stack
        if self.instructions.last().is_some_and(|i| i.op == Op::POP) {
            self.instructions.pop();
        }
    }
}
