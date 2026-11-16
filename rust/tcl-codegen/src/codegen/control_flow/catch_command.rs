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

//! Inline `catch`: the supported shapes, output stores, the body with its `return` and `error`, and the exception region.

use super::super::cmd_subst::{
    parse_script_parts_expand_with_config, parse_script_parts_with_config,
};
use super::super::{CodegenCtx, Op, Operand};
use super::{
    InlineBodyEmitter, detect_const_expr_error, is_inline_local_scalar_name, merge_return_operands,
};
use tcl_compiler::cfg::Function as CfgFunction;
use tcl_compiler::ir::Statement;
use tcl_registry::Traits;
use tcl_registry::hooks::InlineCodegenHookId;

impl CodegenCtx<'_> {
    /// Whether the literal-body `catch` specialiser can preserve the exact
    /// source-word semantics of every argument.
    ///
    /// A braced body is already the script value which `catch` will evaluate.
    /// An unbraced/quoted body must first undergo Tcl substitution and escape
    /// decoding, so it belongs to the generic, explicit-stack runtime command.
    /// Result/options variable names may be bare literals, but only the shared
    /// Tcl word classifier and the local-scalar shape acceptor may prove them.
    pub(crate) fn catch_inline_args_are_static(args: &[(String, bool)]) -> bool {
        args.first().is_some_and(|(_, braced)| *braced)
            && args.iter().skip(1).all(|(name, braced)| {
                !tcl_syntax::naming::word_is_dynamic(name, *braced)
                    && !name.contains('\\')
                    && is_inline_local_scalar_name(name)
            })
    }

    /// Query the retained native protocol before opening a catch range. A
    /// handler identity alone cannot select the observable output store order.
    pub(crate) fn catch_output_order(
        &self,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) -> tcl_registry::catch_invocation::CatchOutputOrder {
        use tcl_registry::catch_invocation::{CatchInvocation, CatchOutputOrder};
        use tcl_registry::native_compilation::NativeCompilationSelection;
        if result_var.is_none() || options_var.is_none() {
            return CatchOutputOrder::ResultThenOptions;
        }
        let binding = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.source_binding.as_ref());
        let dialect = binding
            .and_then(
                tcl_compiler::command_binding::SourceInvocationBinding::native_compiler_dialect,
            )
            .unwrap_or_else(|| {
                tcl_compiler::environment_ingress::authoring_invocation_dialect(
                    self.registry,
                    self.dialect,
                    self.lexer_config(),
                )
            });
        let selection = binding.map_or(
            NativeCompilationSelection::Unknown,
            tcl_compiler::command_binding::SourceInvocationBinding::native_compilation_admission_selection,
        );
        CatchInvocation {
            result_var_at: Some(1),
            options_var_at: Some(2),
            ..CatchInvocation::CAPTURE_TCL_PHASE
        }
        .output_order(dialect, selection)
    }

    /// This emitter handles the positional capture grammar only. In particular,
    /// Jim's filtering flags are parsed by the central grammar and stay generic.
    pub(crate) fn catch_inline_shape_is_supported(&self, args: &[(String, bool)]) -> bool {
        use tcl_registry::catch_invocation::{CatchInvocationSelection, CatchOutputOrder};
        let dialect = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| tokens.source_binding.as_ref())
            .and_then(
                tcl_compiler::command_binding::SourceInvocationBinding::native_compiler_dialect,
            )
            .unwrap_or_else(|| {
                tcl_compiler::environment_ingress::authoring_invocation_dialect(
                    self.registry,
                    self.dialect,
                    self.lexer_config(),
                )
            });
        let words: Vec<_> = args.iter().map(|(word, _)| word.as_str()).collect();
        let CatchInvocationSelection::Valid(capture) =
            tcl_registry::catch_invocation::select_catch_invocation(
                tcl_registry::InvocationArguments::literals(&words),
                dialect,
            )
        else {
            return false;
        };
        capture.script_at == 0
            && capture.ignored_codes == 0
            && capture.result_var_at == (args.len() >= 2).then_some(1)
            && capture.options_var_at == (args.len() >= 3).then_some(2)
            && Self::catch_inline_args_are_static(args)
            && self.catch_output_order(
                args.get(1).map(|(s, _)| s.as_str()),
                args.get(2).map(|(s, _)| s.as_str()),
            ) != CatchOutputOrder::Unknown
    }

    /// Consume captured result/options objects while looking up each output
    /// only at its own write. A preceding write trace may retarget the next name.
    fn emit_catch_output_stores(
        &mut self,
        result_var: Option<&str>,
        options_var: Option<&str>,
        order: tcl_registry::catch_invocation::CatchOutputOrder,
    ) {
        use tcl_registry::catch_invocation::CatchOutputOrder;
        if let Some(options) = options_var {
            match order {
                CatchOutputOrder::ResultThenOptions => {
                    self.emit(Op::REVERSE, vec![Operand::Imm(3)]);
                    if let Some(result) = result_var {
                        self.store_var(result);
                    }
                    self.emit(Op::POP, vec![]);
                    self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
                    self.store_var(options);
                    self.emit(Op::POP, vec![]);
                    return;
                }
                CatchOutputOrder::OptionsThenResult => {
                    self.store_var(options);
                    self.emit(Op::POP, vec![]);
                }
                CatchOutputOrder::Unknown => unreachable!("unproved catch output protocol"),
            }
        }
        self.emit(Op::REVERSE, vec![Operand::Imm(2)]);
        if let Some(result) = result_var {
            self.store_var(result);
        }
        self.emit(Op::POP, vec![]);
    }

    /// Emit inline `beginCatch4`/`endCatch` bytecodes for `catch`.
    ///
    /// Compiles each complete body command inline, then emits the normal/handler
    /// paths and stores result/options variables.
    /// The catch return code is left on the stack. Returns false without
    /// emitting instructions when the output protocol has not been proved.
    pub fn emit_catch_inline(
        &mut self,
        body_text: &str,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) -> bool {
        if self.catch_output_order(result_var, options_var)
            == tcl_registry::catch_invocation::CatchOutputOrder::Unknown
        {
            return false;
        }
        let previous_context = self.native_compilation;
        self.native_compilation = previous_context.with_inline_exception_range();
        self.with_inline_body_operand(body_text, 0, |ctx| {
            ctx.emit_catch_inline_inner(body_text, result_var, options_var);
        });
        self.native_compilation = previous_context;
        true
    }

    fn emit_catch_inline_inner(
        &mut self,
        body_text: &str,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) {
        // `parse_cmd_parts` has already resolved the outer argument word. The
        // value may legitimately begin and end with braced command words
        // (`catch {{set} x {1}}`), so it must not be stripped a second time.
        let body = body_text;
        let output_order = self.catch_output_order(result_var, options_var);

        // Pre-intern result_var so it gets a lower LVT slot
        if let Some(name) = result_var {
            self.command_variable_slot(name.as_bytes());
        }

        // beginCatch4 with current nesting depth. The handler label rides
        // out-of-band on the instruction (`Instruction::catch_target`, the
        // analogue of C's `ExceptionRange.catchOffset`) so the VM opens a
        // *live* range; the operand keeps C's range-index meaning and the
        // disassembly its shape. Back-patched once the handler is placed.
        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;

        // Keep every complete command in this activation so a yield freezes the
        // live catch range itself. The shared segmenter owns command boundaries;
        // `emit_catch_body` remains the one-command emitter. Discard each
        // non-final result exactly as ordinary script execution does. For a
        // fatal or incomplete tail the compiler-owned command plan keeps the
        // complete prefix executable, then raises the catchable parse error if
        // that prefix finishes normally; substitutions in the bad command do
        // not run.
        self.emit_segmented_inline_body(body, InlineBodyEmitter::Catch);

        // Normal completion: push code "0".
        self.push_lit("0");

        let handler_label = self.fresh_label("catch_handler");
        if options_var.is_some() {
            // 3-arg catch: normal path jumps to shared pushReturnOpts.
            let opts_label = self.fresh_label("catch_opts");
            self.emit(Op::JUMP1, vec![Operand::Label(opts_label.clone())]);

            // Handler entry: push caught result and return code.
            self.place_label(&handler_label);
            self.emit(Op::PUSH_RESULT, vec![]);
            self.emit(Op::PUSH_RETURN_CODE, vec![]);

            // pushReturnOpts shared by both paths.
            self.place_label(&opts_label);
            self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        } else {
            // 2-arg catch: jump over handler to endCatch.
            let end_label = self.fresh_label("catch_end");
            self.emit(Op::JUMP1, vec![Operand::Label(end_label.clone())]);

            // Handler entry: push caught result and return code.
            self.place_label(&handler_label);
            self.emit(Op::PUSH_RESULT, vec![]);
            self.emit(Op::PUSH_RETURN_CODE, vec![]);

            self.place_label(&end_label);
        }
        self.instructions[begin_idx].catch_target = Some(handler_label);

        // endCatch.
        self.catch_depth -= 1;
        self.emit(Op::END_CATCH, vec![]);

        // Stack: [result, code], or [result, code, options].
        self.emit_catch_output_stores(result_var, options_var, output_order);
        // Return code stays on the stack as value of catch.
    }

    /// Compile one catch command using selected wrapping and emitter metadata.
    pub fn emit_catch_body(&mut self, body: &str) {
        // The resolved script keeps its own configured argument-expansion phase.
        if self.recognises_expand_syntax() && body.contains("{*}") {
            let Some(expanded) = parse_script_parts_expand_with_config(body, self.lexer_config())
            else {
                self.emit_script_eval(body);
                self.seen_generic_invoke = true;
                return;
            };
            if expanded.iter().any(|(_, _, expand)| *expand) {
                self.emit_expanded_cmd_subst(&expanded);
                self.seen_generic_invoke = true;
                return;
            }
        }
        let Some(body_parts) = parse_script_parts_with_config(body, self.lexer_config()) else {
            self.emit_script_eval(body);
            self.seen_generic_invoke = true;
            return;
        };
        if body_parts.is_empty() {
            self.push_lit("");
            return;
        }

        let body_cmd = &body_parts[0].0;
        let body_args = &body_parts[1..];

        let sc_label = self.start_catch_body_command(body_cmd, body_args);

        if super::super::emitter::bytecoded::try_value_bytecoded(self, body_cmd, body_args) {
            if let Some(label) = sc_label {
                self.place_label(&label);
            }
            return;
        }

        // Hooks this catch-body dispatcher does not specialise
        // (`Incr`, and `String`, `Lrange`, `Linsert` and `Lreplace` but for a
        // literal index read apart, …, which only the value-position
        // dispatcher in `cmd_subst` emits inline) fall to the generic invoke
        // arm, as do guard failures.
        match self.inline_cmd_subst_hook(body_cmd, body_args) {
            Some(InlineCodegenHookId::Yield) if body_args.len() <= 1 => {
                self.emit_inline_yield(body_args);
            }
            Some(InlineCodegenHookId::YieldTo) => {
                self.emit_inline_yield_to(
                    body_args
                        .iter()
                        .map(|(word, braced)| (word.as_str(), *braced, false)),
                );
            }
            Some(
                InlineCodegenHookId::String
                | InlineCodegenHookId::Lrange
                | InlineCodegenHookId::Linsert
                | InlineCodegenHookId::Lreplace,
            ) if self.try_emit_compiled_index_command(body_cmd, body_args) => {}
            Some(InlineCodegenHookId::Return) => self.emit_catch_return(body_args),
            Some(InlineCodegenHookId::Error) => self.emit_catch_error(body_cmd, body, body_args),
            Some(InlineCodegenHookId::Break) => {
                self.emit(Op::BREAK, vec![]);
            }
            Some(InlineCodegenHookId::Continue) => {
                self.emit(Op::CONTINUE, vec![]);
            }
            Some(InlineCodegenHookId::Expr)
                if super::super::cmd_subst::lone_spelled_expression(body_args) =>
            {
                self.emit_catch_expression(&body_args[0].0);
            }
            Some(InlineCodegenHookId::Try) if self.compiles_locals() => {
                if self
                    .try_on_error_inline_bindings(body_cmd, body_args)
                    .is_some()
                {
                    let try_sc = self.fresh_label("catch_body_end");
                    self.emit_comment(
                        Op::START_CMD,
                        vec![Operand::Label(try_sc.clone()), Operand::Imm(1)],
                        "",
                    );
                    self.cmd_index += 1;
                    self.emit_try_on_error_inline(body_cmd, body_args, &try_sc);
                    self.place_label(&try_sc);
                    self.seen_generic_invoke = true;
                } else {
                    self.emit_generic_cmd_subst(body_cmd, body_args);
                    self.seen_generic_invoke = true;
                }
            }
            _ => {
                // One shared fixed-arity generic emitter owns word
                // substitution and specialised entered-command metadata.
                self.emit_generic_cmd_subst(body_cmd, body_args);
                self.seen_generic_invoke = true;
            }
        }

        if let Some(label) = sc_label {
            self.place_label(&label);
        }
    }

    fn start_catch_body_command(
        &mut self,
        command: &str,
        args: &[(String, bool)],
    ) -> Option<String> {
        let needs_start_cmd = self
            .selected_invocation_traits(command, args)
            .is_some_and(|traits| traits.contains(Traits::NEEDS_START_CMD));

        if needs_start_cmd {
            let label = self.fresh_label("catch_body_end");
            self.emit_comment(
                Op::START_CMD,
                vec![Operand::Label(label.clone()), Operand::Imm(1)],
                "",
            );
            self.cmd_index += 1;
            Some(label)
        } else {
            None
        }
    }

    fn emit_catch_expression(&mut self, source: &str) {
        // Parsed under the compile's dialect, as lowering parses a
        // statement-position `expr`.
        let node = self.parse_compile_expr(source);
        if let Some((message, options)) = detect_const_expr_error(&node) {
            self.push_lit(&message);
            self.push_lit(&options);
            self.emit(Op::SYNTAX, vec![Operand::Imm(1), Operand::Imm(0)]);
        } else {
            self.emit_expr(&node);
        }
    }

    /// Compile `return ?-code C? ?-level L? ?value?` inside a catch body.
    pub fn emit_catch_return(&mut self, args: &[(String, bool)]) {
        let code_names: &[(&str, i32)] = &[
            ("ok", 0),
            ("error", 1),
            ("return", 2),
            ("break", 3),
            ("continue", 4),
        ];
        let mut i = 0;
        let mut code: Option<i32> = None;
        let mut level: Option<i32> = None;

        while i < args.len() {
            let flag = &args[i].0;
            // registry-axis-ok: options — return's -code/-level options in codegen; until the return option table drives the emitter
            if flag == "-code" && i + 1 < args.len() {
                let val = &args[i + 1].0;
                code = code_names
                    .iter()
                    .find(|(n, _)| *n == val)
                    .map(|(_, c)| *c)
                    .or_else(|| {
                        self.parse_int_operand(val)
                            .and_then(|v| i32::try_from(v).ok())
                    });
                if code.is_none() {
                    break;
                }
                i += 2;
            // registry-axis-ok: options — return's -code/-level options in codegen; until the return option table drives the emitter
            } else if flag == "-level" && i + 1 < args.len() {
                level = self
                    .parse_int_operand(&args[i + 1].0)
                    .and_then(|v| i32::try_from(v).ok());
                if level.is_none() {
                    break;
                }
                i += 2;
            // registry-axis-ok: options — return's -code/-level options in codegen; until the return option table drives the emitter
            } else if flag == "--" {
                i += 1;
                break;
            } else {
                // Non-flag or unknown flag: stop option parsing
                break;
            }
        }

        let remaining = &args[i..];
        let value = if remaining.is_empty() {
            ""
        } else {
            &remaining[0].0
        };

        let (ret_code, ret_level) = merge_return_operands(code, level);

        self.emit_value(value, true);
        self.push_lit("");
        self.emit(
            Op::RETURN_IMM,
            vec![Operand::Imm(ret_code), Operand::Imm(ret_level)],
        );
    }

    /// Compile `error msg ?info? ?code?` inside a catch body.
    pub fn emit_catch_error(
        &mut self,
        command: &str,
        source_command: &str,
        args: &[(String, bool)],
    ) {
        if !self.emit_inline_error(args, source_command) {
            self.emit_generic_cmd_subst(command, args);
            self.seen_generic_invoke = true;
        }
    }

    // Inline catch compilation.

    /// Open C's `catch` exception range.
    ///
    /// Pre-interns the result and options variables so the slots land where C
    /// puts them: `catch {set q $x} m o` gives m=1, o=2 and only then q=3.
    ///
    /// Returns the `beginCatch4` index and its handler label, for
    /// [`Self::emit_catch_region_epilogue`] to place and back-patch.
    pub fn emit_catch_region_prologue(
        &mut self,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) -> (usize, String) {
        for name in [result_var, options_var].into_iter().flatten() {
            self.command_variable_slot(name.as_bytes());
        }

        let begin_idx = self.emit(
            Op::BEGIN_CATCH4,
            vec![Operand::Imm(
                i32::try_from(self.catch_depth).expect("catch_depth fits in i32"),
            )],
        );
        self.catch_depth += 1;
        (begin_idx, self.fresh_label("catch_handler"))
    }

    /// Close the range opened by [`Self::emit_catch_region_prologue`] and
    /// store `catch`'s results.
    ///
    /// Measured against `tcl::unsupported::disassemble` on 9.0.4. For
    /// `proc p {x} { catch {set q $x} m o }` C emits, after the body:
    ///
    /// ```text
    /// push1 0             # "0"      <- normal completion pushes code 0
    /// jump1 +4
    /// pushResult                     <- handler pushes result and code
    /// pushReturnCode
    /// pushReturnOpts                 <- 3-arg only, reached by both paths
    /// endCatch
    /// storeScalar1 %v2    # var "o"  <- options come off the top first
    /// pop
    /// reverse 2
    /// storeScalar1 %v1    # var "m"
    /// pop
    /// ```
    ///
    /// Both paths arrive the same depth: normal completion leaves the body's
    /// own result below the pushed code, the handler pushes result and code
    /// itself.
    pub fn emit_catch_region_epilogue(
        &mut self,
        begin_idx: usize,
        handler_label: &str,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) {
        self.push_lit("0");
        let converge = self.fresh_label("catch_converge");
        self.emit(Op::JUMP1, vec![Operand::Label(converge.clone())]);

        self.place_label(handler_label);
        self.emit(Op::PUSH_RESULT, vec![]);
        self.emit(Op::PUSH_RETURN_CODE, vec![]);

        self.place_label(&converge);
        if options_var.is_some() {
            self.emit(Op::PUSH_RETURN_OPTS, vec![]);
        }
        self.instructions[begin_idx].catch_target = Some(handler_label.to_owned());

        self.catch_depth -= 1;
        self.emit(Op::END_CATCH, vec![]);

        let output_order = self.catch_output_order(result_var, options_var);
        self.emit_catch_output_stores(result_var, options_var, output_order);

        // `catch` in statement position: its return code is the statement's
        // value, and every statement's value is popped. In final position
        // `remove_trailing_pop` takes this one back off again, which is how C
        // ends `proc p {x} {catch {…}}` with the code still on the stack.
        self.emit(Op::POP, vec![]);
    }

    /// Emit a whole `catch` region: open the range, compile the body, close it.
    ///
    /// The body is emitted here rather than by the ordinary block walk because
    /// that walk pops every statement's value, and `catch` needs the body's
    /// last result left on the stack for its result variable. The CFG builder
    /// only inlines a body that lowers to this one straight-line block
    /// (`lower_catch_dispatch`), so there is never a later block to miss.
    pub fn emit_catch_region_inline(
        &mut self,
        cfg: &CfgFunction,
        catch_body_name: &str,
        result_var: Option<&str>,
        options_var: Option<&str>,
    ) {
        let body_blk = cfg
            .block_by_name(catch_body_name)
            .expect("catch body block present");

        let (begin_idx, handler_label) = self.emit_catch_region_prologue(result_var, options_var);
        // The captured wrapper is a source command before the body's first
        // command. Its children use the ordinary command-boundary owner.
        self.cmd_index += 1;
        // Only the *last* statement's value is `catch`'s result; every earlier
        // one is discarded exactly as ordinary script execution discards it.
        // `emit_try_body_stmt` strips the trailing pop to keep a value on the
        // stack, so it is right for the final statement and wrong for the
        // others — without this `catch {set x 1; set y 2}` leaves `1` under
        // the result, and a loop around it grows the operand stack without
        // bound. C pops after each non-final command.
        let last = body_blk
            .statements
            .iter()
            .rposition(Statement::is_executable_invocation);
        for (index, stmt) in body_blk.statements.iter().enumerate() {
            if !stmt.is_executable_invocation() {
                continue;
            }
            if Some(index) == last {
                self.emit_try_body_stmt(stmt);
            } else {
                self.emit_stmt_with_start_cmd(stmt, None, None);
            }
        }
        if last.is_none() {
            self.push_lit("");
        }
        self.emit_catch_region_epilogue(begin_idx, &handler_label, result_var, options_var);
    }
}
