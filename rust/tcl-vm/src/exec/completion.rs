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

//! Settling a completion: catch ranges, error logging, unwinding activations and procedure frames, and return information.

use super::{
    ActivationErrorLog, CaughtState, ChildCompletion, EachLoopFold, Frame, SubstFold, TailcallReq,
};
use crate::interp::Vm;
use crate::value::Value;
use tcl_bytecode::{ErrorRegion, ErrorRegionFrame, ErrorStackContext, Instruction, Op};
use tcl_runtime_api::{Code, Completion};

/// The command C's `TclLogCommandInfo` names for `instruction`, as bytes, its
/// line, and the commands a directly evaluated substitution logs after it
/// (upstream `Instruction::error_info_text`/`error_info_enclosing`). The
/// compiler's `ErrorInfoCommand` keeps the trailing whitespace before the
/// command's terminator and names the command inside a substitution; without
/// one the replayable statement text is the command.
pub(crate) fn logged_command(instruction: &Instruction) -> LoggedCommand {
    instruction.error_info_command.as_ref().map_or_else(
        || {
            (
                instruction.source_cmd_text.bytes().to_vec(),
                instruction.source_line,
                Vec::new(),
            )
        },
        |command| {
            (
                command.text.as_bytes().to_vec(),
                command.line,
                command.enclosing.clone(),
            )
        },
    )
}

/// The TIP 348 `INNER` context of an invoke that failed with `words` (C's
/// `TclGetInnerContext`): the instruction under its release's name and its
/// words; an `error`/`throw` C compiles to `returnImm` names that; a command
/// the release evaluates directly is its text.
pub(super) fn invoke_inner_context(
    vm: &Vm,
    instruction: &Instruction,
    words: &[Value],
    text: &[u8],
) -> Value {
    if instruction.evaluation == tcl_runtime_api::CommandEvaluation::Direct {
        return Value::from_native_string_bytes(text.to_vec());
    }
    if let Some(compiled) = compiled_error_context(words, text) {
        return compiled;
    }
    let mnemonic = if vm.runtime_version().emits_wide_instructions_only() {
        instruction.op.wide_mnemonic()
    } else {
        instruction.op.mnemonic()
    };
    let mut context = Vec::with_capacity(words.len() + 1);
    context.push(Value::string(mnemonic));
    context.extend(words.iter().cloned());
    Value::list(context)
}

/// The TIP 348 `INNER` context of a compiled instruction that failed after
/// consuming `operands` (C's `TclGetInnerContext`): its name in the release's
/// instruction set and those operands, as `{mult abc 2}`.
pub(super) fn instruction_inner_context(
    vm: &Vm,
    instruction: &Instruction,
    operands: &[Value],
) -> Value {
    let mnemonic = if vm.runtime_version().emits_wide_instructions_only() {
        instruction.op.wide_mnemonic()
    } else {
        instruction.op.mnemonic()
    };
    let mut context = Vec::with_capacity(operands.len() + 1);
    context.push(Value::string(mnemonic));
    let taken = inner_operand_count(instruction.op);
    if taken > 0 && operands.len() == taken {
        context.extend(operands.iter().cloned());
    }
    Value::list(context)
}

/// How many stack operands C's `TclGetInnerContext` lists after an
/// instruction's name; any other failing instruction is named alone.
/// tclsh 8.6.18/9.0.4/9.1.0: `INNER {strindex abc x}`, `INNER listIndexImm`.
const fn inner_operand_count(op: Op) -> usize {
    match op {
        Op::STR_LEN
        | Op::LNOT
        | Op::NOT
        | Op::BITNOT
        | Op::UMINUS
        | Op::UPLUS
        | Op::TRY_CVT_TO_NUMERIC
        | Op::EXPAND_STKTOP
        | Op::EXPR_STK
        | Op::RETURN_STK => 1,
        Op::LIST_IN
        | Op::LIST_NOT_IN
        | Op::STR_EQ
        | Op::STR_NEQ
        | Op::STR_CMP
        | Op::STR_INDEX
        | Op::STR_MATCH
        | Op::REGEXP
        | Op::EQ
        | Op::NEQ
        | Op::LT
        | Op::GT
        | Op::LE
        | Op::GE
        | Op::MOD
        | Op::LSHIFT
        | Op::RSHIFT
        | Op::BITOR
        | Op::BITXOR
        | Op::BITAND
        | Op::EXPON
        | Op::ADD
        | Op::SUB
        | Op::DIV
        | Op::MULT
        | Op::SYNTAX
        | Op::RETURN_IMM => 2,
        _ => 0,
    }
}

/// The `INNER` context of an `error` or `throw` C compiles to `returnImm`
/// (`TclCompileErrorCmd`, `TclCompileThrowCmd`): one whose command word is
/// literally the command, with the message and the options it builds.
fn compiled_error_context(words: &[Value], text: &[u8]) -> Option<Value> {
    let head = words.first()?.string_bytes();
    let first = text
        .split(u8::is_ascii_whitespace)
        .find(|word| !word.is_empty())?;
    if first != &*head {
        return None;
    }
    let (message, options) = match (&*head, words) {
        (b"error", [_, message]) => (message, Vec::new()),
        (b"error", [_, message, info]) => {
            (message, vec![Value::string("-errorinfo"), info.clone()])
        }
        (b"error", [_, message, info, code]) => (
            message,
            vec![
                Value::string("-errorinfo"),
                info.clone(),
                Value::string("-errorcode"),
                code.clone(),
            ],
        ),
        (b"throw", [_, code, message]) => {
            (message, vec![Value::string("-errorcode"), code.clone()])
        }
        _ => return None,
    };
    Some(Value::list(vec![
        Value::string("returnImm"),
        message.clone(),
        Value::list(options),
    ]))
}

/// A logged command's text, line and enclosing commands ([`logged_command`]).
pub(crate) type LoggedCommand = (Vec<u8>, u32, Vec<(String, u32)>);

impl Vm {
    /// Settle a frame's exceptional completion (`Tick::Return`): a live catch
    /// range in the faulting frame absorbs it first (C's catchStack; it defers
    /// internally when an inline loop is nested inside the range); then a
    /// `break`/`continue` *returned by a command* (`if {…} $z`, `eval break`)
    /// inside an inline loop body jumps to the loop's exit/continue point
    /// rather than unwinding out of the function (the inline `JUMP` only
    /// covers a *literal* break/continue — this mirrors C Tcl's loop exception
    /// ranges); anything still unhandled unwinds. `Some(done)` ends the drive.
    pub(super) fn settle_completion(
        &mut self,
        acts: &mut Vec<Frame>,
        c: Completion<Value>,
    ) -> Option<Completion<Value>> {
        let c = match self.publish_and_log_completion(acts, c) {
            Ok(completion) => completion,
            Err(completion) => return Some(completion),
        };
        let c = match self
            .absorb_catch_range(acts.last_mut().expect("activation stack is non-empty"), c)
        {
            Ok(()) => return None,
            Err(c) => c,
        };
        if matches!(c.code, Code::Break | Code::Continue)
            && Self::catch_loop_completion(acts.last_mut().expect("still non-empty"), c.code)
        {
            return None;
        }
        self.unwind(acts, c)
    }

    pub(super) fn publish_and_log_completion(
        &mut self,
        acts: &mut [Frame],
        c: Completion<Value>,
    ) -> Result<Completion<Value>, Completion<Value>> {
        let c = match self.publish_native_interp_completion(c) {
            Ok(completion) => completion,
            Err(error) => {
                return Err(crate::command::completion_from_tcl_error(self, error));
            }
        };
        if c.code == Code::Error {
            self.observe_native_error_result(&c.result);
        }
        if c.code == Code::Error && self.uses_jim_error_stack() {
            self.refresh_jim_evaluation_frames(acts);
            self.capture_jim_error_stack();
            if let Some(frame) = acts.last_mut() {
                frame.jim_evaluation.invocation = Value::empty();
            }
        }
        if c.code == Code::Error
            && let Some((text, line, context, enclosing)) = acts.last().and_then(|frame| {
                let index = frame.pc.saturating_sub(1);
                frame.asm.instructions.get(index).map(|instruction| {
                    let (logged, line, enclosing) = logged_command(instruction);
                    let context = match instruction.error_stack_context.as_ref() {
                        Some(ErrorStackContext::CommandResult { head, .. }) => {
                            Value::list(vec![Value::string(head.as_str()), c.result.clone()])
                        }
                        None if instruction.evaluation
                            != tcl_runtime_api::CommandEvaluation::Direct =>
                        {
                            instruction_inner_context(
                                self,
                                instruction,
                                frame.failed_operands_at(index).unwrap_or_default(),
                            )
                        }
                        Some(ErrorStackContext::ReturnImmediate { .. }) | None => {
                            Value::from_native_string_bytes(
                                instruction.source_cmd_text.bytes().to_vec(),
                            )
                        }
                    };
                    let text = match instruction.error_stack_context.as_ref() {
                        Some(ErrorStackContext::CommandResult {
                            error_info_command, ..
                        }) => error_info_command.clone(),
                        Some(ErrorStackContext::ReturnImmediate { error_info_command })
                            if !error_info_command.is_empty() =>
                        {
                            error_info_command.clone()
                        }
                        Some(ErrorStackContext::ReturnImmediate { .. }) | None => logged,
                    };
                    (text, line, context, enclosing)
                })
            })
        {
            let message = c.result.string_bytes();
            self.log_command_with_enclosing(&text, context, &enclosing, &message, line);
        }
        Ok(c)
    }

    /// A compiled `catch` evaluating its script word inside its catch range:
    /// C logs the `catch` command as an error leaves the script, before
    /// catching it.
    pub(super) fn log_compiled_catch(&mut self, acts: &[Frame], c: &Completion<Value>) {
        if c.code == Code::Error
            && let Some((text, line, enclosing)) = acts.last().and_then(|frame| {
                frame
                    .asm
                    .instructions
                    .get(frame.pc.saturating_sub(1))
                    .map(logged_command)
            })
        {
            let message = c.result.string_bytes();
            self.clear_error_logged();
            self.log_command_with_enclosing(
                &text,
                Value::from_native_string_bytes(text.clone()),
                &enclosing,
                &message,
                line,
            );
        }
    }

    /// Log an instruction's command and, when that logs it, the commands
    /// enclosing it ([`Self::log_enclosing_commands`]). An instruction outside
    /// any command (a condition word's runtime substitution) names none: C
    /// never logs an empty command.
    pub(crate) fn log_command_with_enclosing(
        &mut self,
        text: &[u8],
        context: Value,
        enclosing: &[(String, u32)],
        message: &[u8],
        line: u32,
    ) {
        if self.command_error_is_logged() || text.is_empty() {
            return;
        }
        self.log_command_info_with_context(text, context, message, line);
        self.log_enclosing_commands(enclosing, text, message);
    }

    /// Log the commands enclosing a directly evaluated substitution, as each
    /// of C's nested `TclEvalEx` calls logs its own as the error leaves it,
    /// after `logged`, the command just logged.
    pub(crate) fn log_enclosing_commands(
        &mut self,
        enclosing: &[(String, u32)],
        logged: &[u8],
        message: &[u8],
    ) {
        let mut previous = logged;
        for (text, line) in enclosing {
            if text.as_bytes() != previous {
                self.clear_error_logged();
                self.log_command_info(text.as_bytes(), message, *line);
            }
            previous = text.as_bytes();
        }
    }

    /// If the instruction that just produced a `break`/`continue` completion is
    /// inside an inline loop body (per `FunctionAsm::loop_targets`), redirect the
    /// frame's `pc` to the loop's break / continue target and return `true`;
    /// otherwise return `false` (the completion keeps unwinding). The stack is at
    /// a statement boundary here (the failing command consumed its args and
    /// pushed no result), matching what the loop-end / header expects.
    fn catch_loop_completion(f: &mut Frame, code: Code) -> bool {
        let idx = f.pc.saturating_sub(1);
        let Some(&(brk, cont)) = f.asm.loop_targets.get(&idx) else {
            return false;
        };
        let target = if code == Code::Break { brk } else { cont };
        let Some(off) = target else {
            return false; // no target (e.g. a for-step continue) → propagate
        };
        let Some(&tidx) = f.off2idx.get(&off) else {
            return false;
        };
        f.pc = tidx;
        true
    }

    /// As an error unwinds out of activation `act`, synthesise the `errorInfo`
    /// body frames for every inlined command body (`FunctionAsm::error_regions`)
    /// covering the failing instruction — the compiled analogue of C's
    /// per-command `CmdFrame`. Innermost region first (a deeper region has the
    /// larger start): each appends its `("LABEL" body line N)` frame — the
    /// failing command's line made body-relative — then its enclosing command's
    /// `invoked from within "…"` frame, whose line then drives the next-outer
    /// frame (an enclosing region, or this activation's proc frame).
    fn apply_error_regions(&mut self, act: &Frame) {
        if act.asm.error_regions.is_empty() {
            return;
        }
        // The failing instruction's source span — a region covers it when the
        // span lies within the region's (the enclosing command's). Containment,
        // not an index range, so an interleaved non-body instruction is excluded
        // and layout reordering is irrelevant.
        let Some(span) = act
            .asm
            .instructions
            .get(act.pc.saturating_sub(1))
            .and_then(|i| i.source_span)
        else {
            return;
        };
        let (s, e) = (span.start(), span.end());
        let mut covering: Vec<&ErrorRegion> = act
            .asm
            .error_regions
            .iter()
            .filter(|r| r.start <= s && e <= r.end)
            .collect();
        // Innermost first: a deeper body has the larger start (ties broken by the
        // smaller end). Each appends its body frame then its enclosing command's
        // `invoked from within` frame, whose line drives the next-outer frame.
        covering.sort_by(|a, b| b.start.cmp(&a.start).then(a.end.cmp(&b.end)));
        for r in covering {
            let body_line = self.error_line().saturating_sub(r.line_base).max(1);
            match &r.frame {
                ErrorRegionFrame::Body(label) => self.append_body_frame_line(label, body_line),
                ErrorRegionFrame::Line(text) => {
                    self.append_frame_text(&format!("{text} line {body_line}"));
                }
                ErrorRegionFrame::Fixed(text) => self.append_frame_text(text),
                ErrorRegionFrame::None => self.clear_error_logged(),
            }
            self.log_command_info_only(&r.cmd_text, "", r.cmd_line);
        }
    }

    /// Unwind one or more activations with completion `c`. Returns `Some` when
    /// the whole `run` is finished, `None` when a parent activation resumed.
    /// Offer an exceptional completion to `f`'s innermost live catch range
    /// (C's `TclGetExceptionRangeForPc(catchOnly)` at the faulting pc).
    /// `Ok(())` means the range absorbed it: the operand/foreach/expand stacks
    /// are trimmed to the range's depths, the digested completion is recorded
    /// for the epilogue's `PUSH_RESULT`/`PUSH_RETURN_CODE`/`PUSH_RETURN_OPTS`,
    /// and `pc` moves to the handler. `Err(c)` gives the completion back:
    /// no live range, an uncatchable `exit`, or a `break`/`continue` whose
    /// inline loop is nested *inside* the range (the loop's redirect wins, as
    /// C picks the smallest enclosing exception range).
    pub(super) fn absorb_catch_range(
        &mut self,
        f: &mut Frame,
        c: Completion<Value>,
    ) -> Result<(), Completion<Value>> {
        if self.exit_pending() {
            return Err(c);
        }
        // Innermost range not already in its handler (an exception inside a
        // handler belongs to the *enclosing* range, per C's pc containment).
        let fault = f.pc.wrapping_sub(1);
        let Some(pos) = f
            .catch_ranges
            .iter()
            .rposition(|r| !r.in_handler && fault >= r.start_idx && fault < r.end_idx)
        else {
            return Err(c);
        };
        if matches!(c.code, Code::Break | Code::Continue) {
            // The faulting instruction sits in an inline loop body iff
            // `loop_targets` has it; the catch range is *inner* to that loop
            // iff its `BEGIN_CATCH4` sits in the same body (same target
            // pair). Otherwise the loop is the smaller range — defer to
            // `catch_loop_completion`.
            let fault = f.pc.wrapping_sub(1);
            if let Some(pair) = f.asm.loop_targets.get(&fault) {
                let begin = f.catch_ranges[pos].begin_idx;
                if f.asm.loop_targets.get(&begin) != Some(pair) {
                    return Err(c);
                }
            }
        }
        // Ranges nested above the firing one belong to abandoned constructs —
        // their `END_CATCH`s are skipped by the handler jump — so drop them;
        // the firing range itself stays (armed) for its own `END_CATCH`.
        f.catch_ranges.truncate(pos + 1);
        let range = f.catch_ranges.last_mut().expect("pos is in bounds");
        range.in_handler = true;
        let (target_idx, stack_len, foreach_len, expand_len) = (
            range.target_idx,
            range.stack_len,
            range.foreach_len,
            range.expand_len,
        );
        f.stack.truncate(stack_len);
        f.foreach_stack.truncate(foreach_len);
        f.expand_markers.truncate(expand_len);
        f.pc = target_idx;
        let options = self
            .digest_catch_options(&c)
            .map_err(|error| crate::command::completion_from_tcl_error(self, error))?;
        let caught = CaughtState {
            result: c.result,
            code: c.code,
            options,
        };
        if let Some(range) = f.catch_ranges.last_mut() {
            range.caught = Some(caught);
        }
        Ok(())
    }

    /// The completion absorbed by the innermost live catch range, which is the
    /// one whose epilogue is running (the reads precede the paired `END_CATCH`).
    /// `None` when no range fired — the fall-through path, an OK completion.
    pub(super) fn innermost_caught(f: &Frame) -> Option<&CaughtState> {
        f.catch_ranges.last().and_then(|r| r.caught.as_ref())
    }

    /// Add the error frames owned by one completed activation before its proc
    /// boundary is unwound. Keeping this here gives compiled `eval`/`uplevel`
    /// bodies the same single error-context path as every other activation.
    fn apply_activation_error_context(&mut self, act: &mut Frame, acts: &[Frame], message: &[u8]) {
        self.apply_error_regions(act);
        let Some(label) = act.body_label else {
            // A script word the parent's code evaluates (`INST_EVAL_STK`:
            // `catch $s`) is a level of its own: as the error leaves it C
            // logs the command holding that instruction. So does a control
            // command the release runs (`if 1 $s`, `while 1 $s`), once its
            // own frame is added.
            let control_logs = act.error_log == ActivationErrorLog::InvokingCommand
                || act
                    .control
                    .as_ref()
                    .is_some_and(|control| control.logs_invoking_command());
            if let Some((cmd, line, enclosing)) = acts.last().and_then(|parent| {
                parent
                    .asm
                    .instructions
                    .get(parent.pc.saturating_sub(1))
                    .filter(|instruction| control_logs || instruction.op == Op::EVAL_STK)
                    .map(logged_command)
            }) {
                self.clear_error_logged();
                self.log_command_with_enclosing(
                    &cmd,
                    Value::from_native_string_bytes(cmd.clone()),
                    &enclosing,
                    message,
                    line,
                );
            }
            return;
        };
        self.append_body_frame(label);
        if let Some((words, line)) = &act.body_invocation {
            let original = Value::invocation_list_view(words);
            let Some(protocol) = self
                .actual_native_invocation_dialect()
                .native_error_log_protocol()
            else {
                let _ =
                    self.refuse_host_command("tailcall error-frame protocol unavailable".into());
                return;
            };
            let text = match original.native_string_bytes(protocol.string_protocol()) {
                Ok(text) => text,
                Err(error) => {
                    let _ = self.refuse_host_command(error.to_string());
                    return;
                }
            };
            self.log_command_info_with_context(&text, original, "", *line);
            // The entered replacement and the original procedure call are
            // separate Tcl command frames, even though tailcall removed the
            // procedure activation between them.
            self.clear_error_logged();
        }
        // Tcl's uplevel callback appends the body context, then restores the
        // variable frame before its caller logs the enclosing command. The
        // child log owns the UP entry; the enclosing log sees the caller again.
        self.restore_execution_frame(act.selected_frame_restore.take());
        if let Some((cmd, line, enclosing)) = acts.last().and_then(|parent| {
            parent
                .asm
                .instructions
                .get(parent.pc.saturating_sub(1))
                .map(logged_command)
        }) {
            self.log_command_with_enclosing(
                &cmd,
                Value::from_native_string_bytes(cmd.clone()),
                &enclosing,
                b"",
                line,
            );
        }
    }

    fn settle_activation_parse_tail(&mut self, act: &mut Frame, c: &mut Completion<Value>) {
        if c.code == Code::Ok
            && let Some(tail) = act.fatal_tail.take()
        {
            *c = self.raise_fatal_tail(tail);
        }
    }

    fn unwind_activation_state(
        &mut self,
        act: &mut Frame,
        acts: &[Frame],
        c: &mut Completion<Value>,
    ) -> Option<TailcallReq> {
        if let Some(exit) = act
            .oo_call_exit
            .as_ref()
            .or(act.procedure_call_exit.as_ref())
            .cloned()
        {
            let reached = exit.in_original(self, |vm, original| {
                if let Err(error) = exit.check_running(vm, original) {
                    crate::interp::oo_cleanup::retain_error(vm, &error);
                    exit.retire(vm, original);
                    return None;
                }
                vm.unwind_activation_state_in_original(act, acts, c, Some((&exit, original)))
            });
            return match reached {
                Ok(tailcall) => tailcall,
                Err(error) => {
                    crate::interp::oo_cleanup::retain_error(self, &error);
                    let replacement = crate::command::completion_from_cmd_error(self, error);
                    let prior = std::mem::replace(c, replacement);
                    drop(prior);
                    None
                }
            };
        }
        self.unwind_activation_state_in_original(act, acts, c, None)
    }

    fn unwind_activation_state_in_original(
        &mut self,
        act: &mut Frame,
        acts: &[Frame],
        c: &mut Completion<Value>,
        oo_exit: Option<(
            &crate::interp::oo_cleanup::OriginalOoCallExit,
            &crate::interp::oo_cleanup::OriginalOoCleanup,
        )>,
    ) -> Option<TailcallReq> {
        self.settle_activation_parse_tail(act, c);
        if let Some((exit, original)) = oo_exit
            && let Err(error) = exit.check_running(self, original)
        {
            crate::interp::oo_cleanup::retain_error(self, &error);
            exit.retire(self, original);
            return None;
        }
        // An error unwinding through an inlined command body (`eval {…}`)
        // adds the body frames the uncompiled command would, before this
        // activation's own proc frame (innermost first) — the compiled
        // analogue of C's `CmdFrame` trace.
        if c.code == Code::Error {
            let message = c.result.string_bytes();
            self.apply_activation_error_context(act, acts, &message);
        }
        if let Some((exit, original)) = oo_exit
            && let Err(error) = exit.check_running(self, original)
        {
            crate::interp::oo_cleanup::retain_error(self, &error);
            exit.retire(self, original);
            return None;
        }
        let tailcall = act.is_proc.then(|| self.take_frame_tailcall()).flatten();
        if act.is_proc {
            self.unwind_proc_frame(act, acts, c, oo_exit);
        } else if let Some(previous) = act.replay_namespace_restore.take() {
            self.leave_replay_namespace(previous);
        }
        self.restore_execution_frame(act.selected_frame_restore.take());
        tailcall
    }

    fn settle_unwound_activation(
        &mut self,
        act: &mut Frame,
        acts: &mut Vec<Frame>,
        mut c: Completion<Value>,
    ) -> Result<(Completion<Value>, Option<TailcallReq>), ()> {
        let original: Vec<_> = act
            .exec_leave
            .iter()
            .flat_map(|context| context.original_scopes.iter().cloned())
            .collect();
        let tailcall = self.unwind_activation_state(act, acts, &mut c);
        for scope in &original {
            let _ = scope.check(self);
        }
        if self.execution_refusal.is_some() {
            return Ok((c, tailcall));
        }
        // `catch` and `try` still have command-owned completion work below:
        // catch absorbs its body's completion, while try may advance through
        // several handler/finally phase activations. Their execution-trace
        // leave contexts therefore settle only after that work is complete.
        // Every other activation settles here, preserving the established
        // proc-boundary and apply-cleanup ordering.
        let defer_exec_leave = act.catch.is_some()
            || act.package.is_some()
            || act.try_ctx.is_some()
            || act.control.is_some()
            || act.expression.is_some();
        if !defer_exec_leave && !act.exec_leave.is_empty() {
            c = self.settle_exec_leaves(&mut act.exec_leave, c);
        }
        // `apply`'s temporary lambda proc is torn down here, once its script
        // activation completes — on every completion code, matching a
        // nested-drive `cmd_apply`'s unconditional `vm.take_command` after
        // `eval_source` returns.
        if let Some(name) = act.cleanup_proc.take() {
            self.take_command_unchecked(&name);
        }
        if let Some(registration) = act.lambda_registration.take() {
            self.retire_original_lambda_registration(&registration);
        }
        // A `catch` activation absorbs the body's completion of *any* code:
        // its epilogue binds the result / options variables and yields the
        // status code as an integer, delivered to the parent as this `catch`
        // command's result (`exit` stays uncatchable — `finish_catch` passes
        // it straight through).
        for scope in &original {
            let _ = scope.check(self);
        }
        if self.execution_refusal.is_some() {
            return Ok((c, tailcall));
        }
        c = match self.settle_control_activation(act, acts, c) {
            Ok(completion) => completion,
            Err(()) => return Err(()),
        };
        // Settle a traced catch only after its body completion has been
        // absorbed into catch's successful integer result. A traced try
        // reaches here only for its final delivered phase; intermediate
        // phases transferred these contexts above. A leave-trace failure
        // now escapes the completed control command instead of being
        // caught or handled by that same command.
        if defer_exec_leave && !act.exec_leave.is_empty() {
            c = self.settle_exec_leaves(&mut act.exec_leave, c);
        }
        Ok((c, tailcall))
    }

    pub(super) fn unwind(
        &mut self,
        acts: &mut Vec<Frame>,
        c: Completion<Value>,
    ) -> Option<Completion<Value>> {
        let original: Vec<_> = acts
            .iter()
            .flat_map(|activation| activation.exec_leave.iter())
            .chain(self.pending_exec_leave.iter())
            .flat_map(|context| context.original_scopes.iter().cloned())
            .collect();
        let target = acts.last().and_then(|frame| frame.original_target.clone());
        if let Some(target) = &target {
            let _ = target.check_current(self);
        }
        let reached = self.unwind_in_original_scope(acts, c);
        if self.execution_refusal.is_some() {
            drop(reached);
            return Some(self.abort_refused_activations_using(acts, target.as_ref()));
        }
        for scope in &original {
            if self.original_owner_refusal(scope).is_some() {
                drop(reached);
                let completion = self.abort_refused_activations_using(acts, target.as_ref());
                let _ = scope.check(self);
                return Some(completion);
            }
        }
        reached
    }

    fn unwind_in_original_scope(
        &mut self,
        acts: &mut Vec<Frame>,
        mut c: Completion<Value>,
    ) -> Option<Completion<Value>> {
        if self.execution_refusal.is_some() {
            return Some(self.abort_refused_activations(acts));
        }
        // Every direct exceptional hand-off must validate the activation that
        // is about to consume it before that activation is popped. Most ticks
        // are already checked by `drive_loop`, but a synchronous tailcall
        // target can change the compiler and return a non-OK completion from
        // inside `dispatch_tailcall`. Letting that completion enter the pop loop
        // first would discard the stale parent before the ordinary child-to-
        // parent boundary check could see it.
        c = self.validate_unwind_boundary(acts, c);
        loop {
            let mut act = acts.pop().expect("unwinding a non-empty stack");
            if acts.is_empty() && !c.code.is_ok() {
                // An error already logged as it left its command is not
                // logged again at the top level.
                self.pending.top_level_command = (c.code != Code::Error
                    || !self.command_error_is_logged())
                .then(|| {
                    // A directly evaluated script names the outermost command
                    // it was running (C's `TclEvalEx` at `numLevels == 0`).
                    match act
                        .script_command
                        .and_then(|start| act.asm.instructions.get(start))
                    {
                        Some(start) => Some((
                            start.source_cmd_text.bytes().to_vec(),
                            start.source_line,
                            Vec::new(),
                        )),
                        None => act
                            .asm
                            .instructions
                            .get(act.pc.saturating_sub(1))
                            .map(logged_command),
                    }
                })
                .flatten();
            }
            // The clean prefix of a partly-malformed body has now run: raise
            // the parse error C raises after it (#1603).  This happens *first*,
            // before any of the completion processing below, so the deferred
            // error is an error for all of it — error-context frames, the proc
            // boundary and leave traces each see code 1 rather than the
            // prefix's `ok`, and `-errorinfo` is built the same way it is for a
            // runtime error in the same position.  An earlier command's own
            // completion wins, exactly as in `catch`/`try`.
            act.close_native_script();
            let Ok((settled, tailcall)) = self.settle_unwound_activation(&mut act, acts, c) else {
                return None;
            };
            c = settled;
            // Crossing an activation boundary is itself a provenance
            // consumption point. In particular, a freshly compiled computed-
            // head try handler/finally must not return through a proc made stale
            // by the try body. The replacement error then follows the parent's
            // ordinary catch-range and loop settlement below.
            c = self.validate_unwind_boundary(acts, c);
            if c.code.is_ok()
                && let Some(request) = tailcall
            {
                return self.dispatch_tailcall(acts, request);
            }
            if let Some(parent) = acts.last_mut() {
                match self.fold_child_completion(parent, c) {
                    ChildCompletion::Resume => return None,
                    ChildCompletion::Unwind(completion) => {
                        c = completion;
                        continue;
                    }
                    ChildCompletion::Bytecode(completion) => c = completion,
                }
            }
            match acts.last_mut() {
                None if act.settles_as_top_level && !c.code.is_ok() => {
                    return Some(self.settle_level_zero(c));
                }
                None => return Some(c),
                Some(parent) => {
                    if c.code.is_ok() {
                        parent.jim_evaluation.invocation = Value::empty();
                        parent.last_options = c.options;
                        parent.stack.push(c.result.into_native_reference());
                        return None;
                    }
                    // A live catch range in the parent absorbs the child's
                    // exceptional completion — the child was invoked from
                    // inside the range, so this is C's "command returned
                    // non-OK inside a catch range" path at the parent's
                    // invoke instruction.
                    match self.absorb_catch_range(parent, c) {
                        Ok(()) => return None,
                        Err(back) => c = back,
                    }
                    // A child activation that leaves an unhandled
                    // `break`/`continue` delivers it to the enclosing frame's
                    // loop exactly as an inline completion would. This covers
                    // transparent script bodies, unhandled `try` phases, and a
                    // proc boundary after `return -code break|continue` has
                    // lowered the carried return to its requested completion.
                    // `catch`, `subst`, and runtime-fallback each-loops consume
                    // their own completions above before reaching this hand-off.
                    if matches!(c.code, Code::Break | Code::Continue)
                        && Self::catch_loop_completion(parent, c.code)
                    {
                        return None;
                    }
                    // Error / uncaught Break|Continue / unabsorbed Return keep unwinding.
                }
            }
        }
    }

    /// Consume the original child result through the retained parent's driver.
    /// Synchronous tailcall targets and popped child activations share this
    /// boundary; neither reconstructs a command, token or completion.
    fn fold_child_completion(
        &mut self,
        parent: &mut Frame,
        completion: Completion<Value>,
    ) -> ChildCompletion {
        parent.tailcall_invocation = None;
        parent.tailcall_owners.clear();
        if completion.code != Code::Error {
            parent.failed_operands = None;
        }
        if parent.subst.is_some() {
            return match self.fold_subst_bracket(parent, completion) {
                SubstFold::Resume => ChildCompletion::Resume,
                SubstFold::Unwind(completion) => ChildCompletion::Unwind(completion),
            };
        }
        if parent.each_loop.is_some() {
            return match Self::fold_each_loop(parent, completion) {
                EachLoopFold::Resume => ChildCompletion::Resume,
                EachLoopFold::Unwind(completion) => ChildCompletion::Unwind(completion),
            };
        }
        if parent.expression.is_some() {
            return match self.fold_expression_result(parent, completion) {
                Ok(()) => ChildCompletion::Resume,
                Err(completion) => ChildCompletion::Unwind(completion),
            };
        }
        if let Some(state) = parent.jim_script.as_mut() {
            if completion.code == Code::Ok {
                parent.jim_evaluation.invocation = Value::empty();
            }
            state.accept(self, completion);
            return ChildCompletion::Resume;
        }
        if let Some(state) = parent.control.as_mut() {
            state.accept(self, completion);
            return ChildCompletion::Resume;
        }
        ChildCompletion::Bytecode(completion)
    }

    /// Deliver a tailcall target after the issuing procedure has retired.
    pub(super) fn deliver_tailcall_completion(
        &mut self,
        acts: &mut Vec<Frame>,
        completion: Completion<Value>,
    ) -> Option<Completion<Value>> {
        let completion = self.validate_unwind_boundary(acts, completion);
        let Some(parent) = acts.last_mut() else {
            return Some(match self.publish_native_interp_completion(completion) {
                Ok(completion) => completion,
                Err(error) => crate::command::completion_from_tcl_error(self, error),
            });
        };
        match self.fold_child_completion(parent, completion) {
            ChildCompletion::Resume => None,
            ChildCompletion::Unwind(completion) => self.unwind(acts, completion),
            ChildCompletion::Bytecode(completion) if completion.code == Code::Ok => {
                parent.jim_evaluation.invocation = Value::empty();
                parent.last_options = completion.options;
                parent.stack.push(completion.result.into_native_reference());
                None
            }
            ChildCompletion::Bytecode(completion) => self.settle_completion(acts, completion),
        }
    }

    fn settle_unexpected_proc_completion(&mut self, act: &Frame, c: &mut Completion<Value>) {
        if matches!(c.code, Code::Break | Code::Continue) {
            let word = if c.code == Code::Break {
                "break"
            } else {
                "continue"
            };
            // Report the offending command's own line in the proc frame (C's
            // `iPtr->errorLine`), taken from the instruction that produced the
            // completion.
            if let Some(line) = act
                .asm
                .instructions
                .get(act.pc.saturating_sub(1))
                .map(|i| i.source_line)
                .filter(|&l| l != 0)
            {
                self.set_error_line(line);
            }
            let message = format!("invoked \"{word}\" outside of a loop");
            // Through the command-error path so the release's errorCode is
            // the interpreter's (`NONE` before 8.6), not only an option.
            let error = if self.runtime_version().has_unexpected_result_error_codes() {
                tcl_cmd_core::CmdError::with_error_code(message.clone(), "TCL RESULT UNEXPECTED")
            } else {
                tcl_cmd_core::CmdError::new(message.clone())
            };
            *c = crate::command::completion_from_cmd_error(self, error);
            self.seed_unexpected_completion_error(message.as_bytes());
        }
    }

    /// Unwind one proc activation `act`: on error add its `(procedure "name" line
    /// N)` errorInfo frame, pop the call-frame + namespace, apply
    /// `TclUpdateReturnInfo` (a proc boundary decrements a carried `-code`/`-level`
    /// return), and on error log the caller's `invoked from
    /// within "…"` frame. Mutates `c` in place. Split out of [`Vm::unwind`].
    fn unwind_proc_frame(
        &mut self,
        act: &Frame,
        acts: &[Frame],
        c: &mut Completion<Value>,
        oo_exit: Option<(
            &crate::interp::oo_cleanup::OriginalOoCallExit,
            &crate::interp::oo_cleanup::OriginalOoCleanup,
        )>,
    ) {
        self.release_native_procedure_execution();
        if !self.check_oo_proc_exit(oo_exit) {
            return;
        }
        // C's `InterpProcNR2` proc epilogue (`tclProc.c:1864`): a proc body that
        // reaches its boundary with a bare `break`/`continue` — i.e. a
        // `break`/`continue` or `return -level 0 -code break` that produced
        // `TCL_BREAK`/`TCL_CONTINUE` directly, *not* a `return -code break` that
        // the level-decrement below turns into break — is transformed into the
        // error `invoked "break" outside of a loop` with errorcode
        // `TCL RESULT UNEXPECTED`.  Done before the `(procedure …)` frame is
        // appended so the error is logged with the proc's trace, exactly as C's
        // `errorProc` does after the transform.
        self.settle_unexpected_proc_completion(act, c);
        if !self.check_oo_proc_exit(oo_exit) {
            return;
        }
        // The `(procedure … line N)` frame reports the body-relative line of the
        // failing command. A runtime-compiled proc body (the common case) carries
        // body-relative instruction lines with `body_base_line == 0`, so the line
        // is used as-is; a proc compiled inside a module carries absolute lines, so
        // subtract its definition line (`absolute − base + 1`). Jim keeps its
        // own stack trace and never spells the lambda or invoking word.
        if c.code == Code::Error
            && !self.uses_jim_error_stack()
            && let Some(name) = self.current_proc_name()
        {
            let base = act.asm.body_base_line;
            let n = if base == 0 {
                self.error_line().max(1)
            } else {
                self.error_line()
                    .saturating_sub(base)
                    .saturating_add(1)
                    .max(1)
            };
            // An `apply` lambda's body names its lambda (C's `MakeLambdaError`).
            // C names a procedure by the word that invoked it.
            let invoked = if act.oo_call_exit.is_none() {
                self.current_invoking_word()
            } else {
                None
            };
            match self.current_lambda_term() {
                Some(lambda) => self.append_lambda_frame(&lambda.to_str(), n),
                None => self.append_proc_frame(invoked.as_deref().unwrap_or(&name), n),
            }
        }
        if !self.check_oo_proc_exit(oo_exit) {
            return;
        }
        if let Some((exit, original)) = oo_exit {
            exit.retire(self, original);
            if let Err(error) = original.check(self) {
                crate::interp::oo_cleanup::retain_error(self, &error);
                return;
            }
        } else {
            self.pop_call_frame();
            self.pop_ns();
        }
        if !self.update_return_info(c) {
            return;
        }
        if let Some((_, original)) = oo_exit
            && let Err(error) = original.check(self)
        {
            crate::interp::oo_cleanup::retain_error(self, &error);
            return;
        }
        if c.code == Code::Error && self.uses_jim_error_stack() {
            self.refresh_jim_evaluation_frames(acts);
            self.capture_jim_error_stack();
        }
        if let Some((_, original)) = oo_exit
            && let Err(error) = original.check(self)
        {
            crate::interp::oo_cleanup::retain_error(self, &error);
            return;
        }
        if c.code == Code::Error
            && let Some(words) = acts
                .last()
                .and_then(|parent| parent.tailcall_invocation.clone())
        {
            self.log_tailcall_invocation(&words, c);
        }
        // A coroutine's body command (`coroutine c1 boom`) runs as a command
        // of the coroutine's own top level: C logs its words before the
        // resume or `coroutine` command that ran it.
        if c.code == Code::Error
            && let Some(words) = acts.last().and_then(|parent| {
                let original = parent.dispatched_invocation.as_ref()?;
                let items = original.as_list().ok()?;
                (items.len() > 1).then(|| items[1..].to_vec())
            })
        {
            self.log_tailcall_invocation(&words, c);
        }
        if c.code == Code::Error
            && let Some((cmd, line, enclosing)) = acts.last().and_then(|parent| {
                parent
                    .asm
                    .instructions
                    .get(parent.pc.saturating_sub(1))
                    .map(logged_command)
            })
        {
            self.log_command_with_enclosing(
                &cmd,
                Value::from_native_string_bytes(cmd.clone()),
                &enclosing,
                b"",
                line,
            );
        }
    }

    /// Log a tailcall replacement's words as the command an error left (C
    /// runs the replacement as a command of the caller's level), then let the
    /// caller's own command log next.
    pub(super) fn log_tailcall_invocation(&mut self, words: &[Value], c: &Completion<Value>) {
        let text = Value::list(words.to_vec()).string_bytes();
        let message = c.result.string_bytes();
        self.log_command_info_with_context(&*text, Value::list(words.to_vec()), &*message, 0);
        self.clear_error_logged();
    }

    /// C's `TclUpdateReturnInfo`, run where a `return` completion reaches a
    /// procedure boundary, the end of a sourced file, or level 0: while the
    /// carried `-level` stays positive the `return` keeps unwinding; once it
    /// runs out the carried `-code` takes effect.
    /// Returns `false` where settling the options failed (`c` then holds
    /// that failure).
    pub(crate) fn update_return_info(&mut self, c: &mut Completion<Value>) -> bool {
        if c.code == Code::Return && self.uses_jim_error_stack() {
            c.code = self.jim_errors.pending_return.settle_procedure(c.code);
            c.options = match self.jim_return_options(c.code) {
                Ok(options) => options,
                Err(error) => {
                    *c = crate::command::completion_from_cmd_error(self, error);
                    return false;
                }
            };
        } else if c.code == Code::Return
            && matches!(
                c.option_origin,
                tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { .. }
            )
        {
            let tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { code, level } =
                c.option_origin
            else {
                unreachable!()
            };
            let level = level.saturating_sub(1);
            c.option_origin =
                tcl_core_types::CompletionOptionOrigin::MergedReturnOptions { code, level };
            self.settle_native_c_return_level(level);
            if level == 0 {
                c.code = Code::from_int(code);
                if c.code == Code::Error {
                    self.clear_error_logged();
                }
                // C's `TclUpdateReturnInfo` resets the carried options once the
                // level is spent: a `-code return` reaching its level is a
                // plain `return` (`-code 0 -level 1`) one level further out,
                // which the next procedure boundary absorbs.
                if c.code == Code::Return {
                    c.option_origin = tcl_core_types::CompletionOptionOrigin::MergedReturnOptions {
                        code: 0,
                        level: 1,
                    };
                    self.set_native_c_return_state(0, 1);
                }
            }
        } else if c.code == Code::Return {
            // While the carried return level stays positive the TCL_RETURN keeps
            // unwinding one proc level at a time; when it reaches 0 the carried
            // `-code` takes effect. `Code::from_int` (never a 0..=4-only map):
            // `return -code N` for a non-standard N must surface `Code::Other(N)`,
            // not collapse to `Ok` (coroutine-2.4).
            let level = match crate::command::option_integer_checked(self, &c.options, b"-level", 1)
            {
                Ok(level) => level,
                Err(error) => {
                    *c = crate::command::completion_from_tcl_error(self, error);
                    return false;
                }
            };
            if level > 1 {
                c.options = match crate::command::with_return_level(self, &c.options, level - 1) {
                    Ok(options) => options,
                    Err(error) => {
                        *c = crate::command::completion_from_tcl_error(self, error);
                        return false;
                    }
                };
                self.settle_native_c_return_level(level - 1);
            } else {
                let code = match crate::command::option_code_checked(self, &c.options, Code::Ok) {
                    Ok(code) => code,
                    Err(error) => {
                        *c = crate::command::completion_from_tcl_error(self, error);
                        return false;
                    }
                };
                // A `-code return` reaching its level is a plain `return`
                // (`-code 0 -level 1`) one level further out (C's
                // `TclUpdateReturnInfo`).
                let (spent_code, spent_level) = if code == Code::Return {
                    (Some(Value::int(0)), 1)
                } else {
                    (None, 0)
                };
                c.options = match crate::command::with_return_level(self, &c.options, spent_level) {
                    Ok(options) => options,
                    Err(error) => {
                        *c = crate::command::completion_from_tcl_error(self, error);
                        return false;
                    }
                };
                if let Some(spent_code) = spent_code {
                    c.options = match crate::command::with_return_option(
                        self, &c.options, "-code", spent_code,
                    ) {
                        Ok(options) => options,
                        Err(error) => {
                            *c = crate::command::completion_from_tcl_error(self, error);
                            return false;
                        }
                    };
                }
                c.code = code;
                self.settle_native_c_return_level(spent_level);
                if code == Code::Error {
                    // A carried `-errorinfo` suppresses the `return` command's
                    // own frame, but the call site that receives the settled
                    // error must still be appended.
                    self.clear_error_logged();
                }
            }
        }
        true
    }

    fn check_oo_proc_exit(
        &mut self,
        selected: Option<(
            &crate::interp::oo_cleanup::OriginalOoCallExit,
            &crate::interp::oo_cleanup::OriginalOoCleanup,
        )>,
    ) -> bool {
        if let Some((exit, original)) = selected
            && let Err(error) = exit.check_running(self, original)
        {
            crate::interp::oo_cleanup::retain_error(self, &error);
            exit.retire(self, original);
            return false;
        }
        true
    }
}
