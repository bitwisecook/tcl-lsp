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

//! Executing bytecode instructions: the step prologue, the opcode dispatch, variable access, and the operand and classification helpers.

use super::activation::pair_foreach;
use super::{BytecodeEntry, Frame, OriginalExecutionOwner, ScriptNamespace, Tick};
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::interp::{Vm, err};
use crate::value::Value;
use std::collections::HashMap;
use std::ops::ControlFlow;
use std::rc::Rc;
use tcl_bytecode::{FunctionAsm, Instruction, Op};
use tcl_runtime_api::{Code, Completion};

mod comparison_dispatch;
mod native_operations;
mod opcode_dispatch;
mod operands;
mod scalar_variable_access;
mod stack_dispatch;
mod variable_access;

#[cfg(test)]
pub(super) use operands::cmp;
#[cfg(test)]
pub(super) use operands::{bin, char_find, imm_index};
pub(super) use operands::{imm_at, imm0, pop, take_words};

pub(super) type FrameIndexes = (
    Rc<rustc_hash::FxHashMap<i32, usize>>,
    Rc<HashMap<usize, usize>>,
);

/// The offset and `foreach` pairing indexes of shared, immutable assembly,
/// built once per live function rather than once per activation.
pub(super) fn frame_indexes(asm: &Rc<FunctionAsm>) -> FrameIndexes {
    thread_local! {
        static INDEXES: std::cell::RefCell<Vec<(std::rc::Weak<FunctionAsm>, FrameIndexes)>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }
    INDEXES.with_borrow_mut(|cache| {
        if let Some((_, (offsets, pairs))) = cache.iter().rev().find(|(seen, _)| {
            seen.strong_count() != 0 && std::ptr::eq(seen.as_ptr(), Rc::as_ptr(asm))
        }) {
            return (Rc::clone(offsets), Rc::clone(pairs));
        }
        cache.retain(|(seen, _)| seen.strong_count() != 0);
        if cache.len() >= 64 {
            cache.remove(0);
        }
        let offsets = Rc::new(build_off2idx(asm));
        let pairs = Rc::new(pair_foreach(asm));
        cache.push((Rc::downgrade(asm), (Rc::clone(&offsets), Rc::clone(&pairs))));
        (offsets, pairs)
    })
}

fn build_off2idx(asm: &FunctionAsm) -> rustc_hash::FxHashMap<i32, usize> {
    asm.instructions
        .iter()
        .enumerate()
        .map(|(i, instr)| (instr.offset, i))
        .collect()
}

/// The instructions [`Vm::tick_invoke`] executes: those that run a command
/// (and so may recurse natively into a `TclOO` method) while their frame is live.
fn is_invoke_op(op: Op) -> bool {
    matches!(
        op,
        Op::INVOKE_STK1
            | Op::INVOKE_STK4
            | Op::INVOKE_EXPANDED
            | Op::INVOKE_REPLACE
            | Op::TCLOO_NEXT
            | Op::TCLOO_NEXT4
            | Op::TCLOO_NEXT_CLASS
            | Op::TCLOO_NEXT_CLASS4
            | Op::TCLOO_NEXT_LIST
            | Op::TCLOO_NEXT_CLASS_LIST
    )
}

/// Whether the command starting at instruction `start` is invoked by name (an
/// `INVOKE_*` of its own source span), so dispatch charges it to the
/// `commands` limit, rather than compiled inline.
fn command_is_dispatched(asm: &FunctionAsm, start: usize) -> bool {
    let Some(span) = asm
        .instructions
        .get(start)
        .and_then(|instr| instr.source_span)
    else {
        return true;
    };
    asm.instructions[start..]
        .iter()
        .take_while(|instr| {
            instr
                .source_span
                .is_none_or(|inner| span.start() <= inner.start() && inner.end() <= span.end())
        })
        .any(|instr| is_invoke_op(instr.op) && instr.source_span == Some(span))
}

/// The instructions [`Vm::tick_variable_instruction`] runs: variable access,
/// which can run guest code (a variable trace) before the instruction ends.
fn is_variable_op(op: Op) -> bool {
    matches!(
        op,
        Op::LOAD_STK
            | Op::LOAD_SCALAR_STK
            | Op::STORE_STK
            | Op::STORE_SCALAR_STK
            | Op::INCR_STK_IMM
            | Op::INCR_SCALAR_STK_IMM
            | Op::INCR_STK
            | Op::INCR_SCALAR_STK
            | Op::LOAD_SCALAR1
            | Op::LOAD_SCALAR4
            | Op::STORE_SCALAR1
            | Op::STORE_SCALAR4
            | Op::INCR_SCALAR1
            | Op::INCR_SCALAR1_IMM
            | Op::INCR_SCALAR4
            | Op::INCR_SCALAR4_IMM
            | Op::INCR_ARRAY_STK_IMM
            | Op::INCR_ARRAY_STK
            | Op::INCR_ARRAY1
            | Op::INCR_ARRAY1_IMM
            | Op::INCR_ARRAY4
            | Op::INCR_ARRAY4_IMM
            | Op::EXIST_SCALAR
            | Op::EXIST_STK
            | Op::EXIST_ARRAY
            | Op::EXIST_ARRAY_STK
            | Op::LOAD_ARRAY_STK
            | Op::STORE_ARRAY_STK
            | Op::LOAD_ARRAY1
            | Op::LOAD_ARRAY4
            | Op::STORE_ARRAY1
            | Op::STORE_ARRAY4
            | Op::ARRAY_EXISTS_IMM
            | Op::ARRAY_EXISTS_STK
            | Op::ARRAY_MAKE_IMM
            | Op::ARRAY_MAKE_STK
            | Op::UNSET_STK
            | Op::UNSET_SCALAR
            | Op::UNSET_ARRAY
            | Op::UNSET_ARRAY_STK
            | Op::APPEND_SCALAR1
            | Op::APPEND_SCALAR4
            | Op::APPEND_ARRAY1
            | Op::APPEND_ARRAY4
            | Op::LAPPEND_SCALAR1
            | Op::LAPPEND_SCALAR4
            | Op::LAPPEND_ARRAY1
            | Op::LAPPEND_ARRAY4
            | Op::APPEND_STK
            | Op::LAPPEND_STK
            | Op::APPEND_ARRAY_STK
            | Op::LAPPEND_ARRAY_STK
            | Op::LAPPEND_LIST
            | Op::LAPPEND_LIST_STK
            | Op::LAPPEND_LIST_ARRAY
            | Op::LAPPEND_LIST_ARRAY_STK
            | Op::CONST_IMM
            | Op::CONST_STK
            | Op::NSUPVAR
            | Op::UPVAR
            | Op::VARIABLE
            | Op::FOREACH_START
            | Op::FOREACH_STEP
            | Op::LMAP_COLLECT
            | Op::FOREACH_END
            | Op::DICT_FIRST
            | Op::DICT_NEXT
            | Op::DICT_UPDATE_START
            | Op::DICT_UPDATE_END
            | Op::DICT_EXPAND
            | Op::DICT_RECOMBINE_IMM
            | Op::DICT_RECOMBINE_STK
            | Op::VERIFY_DICT
            | Op::DICT_PUT
            | Op::DICT_SET
            | Op::DICT_UNSET
            | Op::DICT_INCR_IMM
            | Op::DICT_APPEND
            | Op::DICT_LAPPEND
    )
}

/// The instructions [`Vm::tick_scalar_variable_instruction`] runs.
fn is_scalar_variable_op(op: Op) -> bool {
    matches!(
        op,
        Op::LOAD_STK
            | Op::LOAD_SCALAR_STK
            | Op::STORE_STK
            | Op::STORE_SCALAR_STK
            | Op::INCR_STK_IMM
            | Op::INCR_SCALAR_STK_IMM
            | Op::INCR_STK
            | Op::INCR_SCALAR_STK
            | Op::LOAD_SCALAR1
            | Op::LOAD_SCALAR4
            | Op::STORE_SCALAR1
            | Op::STORE_SCALAR4
            | Op::INCR_SCALAR1
            | Op::INCR_SCALAR1_IMM
            | Op::INCR_SCALAR4
            | Op::INCR_SCALAR4_IMM
            | Op::INCR_ARRAY_STK_IMM
            | Op::INCR_ARRAY_STK
            | Op::INCR_ARRAY1
            | Op::INCR_ARRAY1_IMM
            | Op::INCR_ARRAY4
            | Op::INCR_ARRAY4_IMM
            | Op::LOAD_ARRAY_STK
            | Op::STORE_ARRAY_STK
            | Op::LOAD_ARRAY1
            | Op::LOAD_ARRAY4
            | Op::STORE_ARRAY1
            | Op::STORE_ARRAY4
    )
}

impl Vm {
    fn frame_result_transport(
        &self,
        frame: &Frame,
    ) -> Result<Value, tcl_syntax::value::ValueError> {
        if let Some(result) = frame.last_result.as_ref() {
            return Ok(result.clone().into_value());
        }
        self.with_native_interp_result(|result| result.native_lifetime_lease().into_value())
    }

    fn finish_frame_result(&mut self, frame: &mut Frame) -> Tick {
        let result = frame
            .stack
            .pop()
            .map_or_else(|| self.frame_result_transport(frame), Ok);
        match result {
            Ok(result) => Tick::Return(Completion::new(
                Code::Ok,
                result,
                frame.last_options.clone(),
            )),
            Err(error) => Tick::Return(crate::command::completion_from_tcl_error(
                self,
                error.into(),
            )),
        }
    }

    /// Execute a single instruction of the top activation.
    pub(super) fn tick(&mut self, f: &mut Frame) -> Tick {
        let original = match NativeOperationScope::enter(self) {
            Ok(original) => original,
            Err(error) => {
                return Tick::Return(crate::command::completion_from_cmd_error(self, error));
            }
        };
        let reached = self.tick_in_original_scope(f);
        if let Some(first) = self.original_scope_refusal(&original) {
            drop(reached);
            let _ = original.check(self);
            f.exec_leave.push(Self::original_scope_context(original));
            return Tick::Return(first);
        }
        match &reached {
            Tick::Call { .. }
            | Tick::PushScript { .. }
            | Tick::PushCatch(_)
            | Tick::PushEval(_)
            | Tick::PushSubst { .. }
            | Tick::PushEachLoop { .. }
            | Tick::PushControl { .. }
            | Tick::PushExpression { .. }
            | Tick::PushTry { .. } => {
                if let Some(context) = &mut self.pending_exec_leave {
                    context
                        .original_scopes
                        .push(OriginalExecutionOwner::Entered(original));
                } else {
                    self.pending_exec_leave = Some(Self::original_scope_context(original));
                }
            }
            Tick::Return(_) | Tick::Suspend(_) => {
                f.exec_leave.push(Self::original_scope_context(original));
            }
            Tick::Continue | Tick::Tailcall(_) => {}
        }
        reached
    }

    #[allow(clippy::too_many_lines)] // One match over every opcode; the VM's central dispatch is clearer whole.
    fn tick_in_original_scope(&mut self, f: &mut Frame) -> Tick {
        if let Some(dispatch) = f.deferred_dispatch.take() {
            return *dispatch;
        }
        if let Some(original) = f.original_invocation.take() {
            f.dispatched_invocation = Some(original.clone());
            return self.tick_original_invocation(f, original);
        }
        // Compilation precedes every body word, even a malformed command
        // appearing after an otherwise executable store. Procedure entry has
        // already admitted this before formal binding; script entry has no
        // parameter frame and presents its compiler contexts here.
        if f.pc == 0
            && let Some(failure) = &f.asm.native_compilation_failure
        {
            return Tick::Return(self.raise_native_compilation_failure(failure, None));
        }
        if f.jim_script_entry.is_some() || f.jim_script.is_some() {
            return self.tick_native_jim_script(f);
        }
        if f.expression.is_some() {
            return self.tick_expression(f);
        }
        if f.control.is_some() {
            return self.tick_control(f);
        }
        // A subst activation is scanner-driven, not bytecode-driven: run the next
        // scan step (native literal/`$` runs, pausing at each top-level `[…]`)
        // instead of the instruction dispatch below.
        if f.subst.is_some() {
            return self.tick_subst(f);
        }
        // An each-loop activation (`foreach`/`lmap` runtime fallback) is
        // likewise driver-stepped, not bytecode-driven.
        if f.each_loop.is_some() {
            return self.tick_each_loop(f);
        }
        let asm = match self.tick_bytecode_prologue(f) {
            ControlFlow::Continue(asm) => asm,
            ControlFlow::Break(tick) => return tick,
        };
        let instr = &asm.instructions[f.pc - 1];
        // Command invocation recurses natively for a TclOO method call (see
        // `OO_DISPATCH_DEPTH_LIMIT`), so it runs in its own small frame rather
        // than in the frame of the whole opcode match.
        if is_invoke_op(instr.op) {
            return self.tick_invoke(f, &asm, instr);
        }
        // Likewise a variable access, whose trace callbacks run guest code
        // while the instruction is executing.
        if is_scalar_variable_op(instr.op) {
            return self.tick_scalar_variable_instruction(f, instr);
        }
        if is_variable_op(instr.op) {
            return self.tick_variable_instruction(f, &asm, instr);
        }
        self.tick_instruction(f, &asm, instr)
    }

    /// The bytecode-instruction prologue of [`Self::tick_in_original_scope`]:
    /// command-boundary bookkeeping, stale-command replay, and the debugger seams.
    /// Returns the frame's function with `f.pc` already past the instruction to
    /// execute, or the tick that ends this step early. Kept out of line so its
    /// locals are not on the native stack while the instruction runs (a `TclOO`
    /// method call recurses natively through the INVOKE path).
    #[inline(never)]
    /// A source command starts: a pin deferred from a native command that has
    /// returned takes effect, and the interpreter's result owner is reset
    /// (frame mirrors preserve lifetime without introducing native references).
    fn begin_source_command(&mut self, f: &mut Frame) -> Option<Completion<Value>> {
        self.install_deferred_pin();
        f.last_result = None;
        self.reset_native_jim_result()
            .err()
            .map(|error| crate::command::completion_from_tcl_error(self, error.into()))
    }

    fn tick_bytecode_prologue(&mut self, f: &mut Frame) -> ControlFlow<Tick, Rc<FunctionAsm>> {
        if f.native_bytecode_entry == BytecodeEntry::Pending {
            if let Err(refusal) = self.reset_native_ensemble_rewrite(
                tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent::BytecodeEntry,
            ) {
                return ControlFlow::Break(Tick::Return(refusal));
            }
            f.native_bytecode_entry = BytecodeEntry::Entered;
        }
        // A control transfer may skip an entered command's invoke (an error,
        // catch jump, or stale-command replay). Drop only entries whose
        // continuation has been reached; enclosing nested commands have later
        // continuations and remain live.
        f.entered_commands
            .retain(|entered| entered.continuation > f.pc);
        f.entered_native_compiler_selections
            .retain(|(start, end)| *start <= f.pc && f.pc < *end);
        f.entered_native_operations
            .retain(|operation| operation.start <= f.pc && f.pc < operation.end);
        let asm = Rc::clone(&f.asm);
        if f.pc >= asm.instructions.len() {
            return ControlFlow::Break(self.finish_frame_result(f));
        }
        let instr = &asm.instructions[f.pc];
        if let Some(tick) = self.enter_native_operations(f, &asm, instr) {
            return ControlFlow::Break(tick);
        }
        if let Some(tick) = self.enter_native_compiler_selection(f, &asm, instr) {
            return ControlFlow::Break(tick);
        }
        // A release that keeps the commands running bytecode compiled (8.4)
        // checks its compile epoch only as it enters a unit: note when.
        if !self.runtime_version().revalidates_running_bytecode() && f.release_units.enters(instr) {
            let stale =
                !self.function_command_bindings_match_with_manifest(&asm, f.manifest.as_deref());
            f.release_units.enter(
                instr,
                crate::release_units::UnitEntry {
                    epoch: self.trace_deopt_epoch(),
                    stale,
                },
            );
        }
        if instr.source_command_boundary.is_start()
            && let Some(refused) = self.begin_source_command(f)
        {
            return ControlFlow::Break(Tick::Return(refused));
        }
        if instr.op != Op::START_CMD
            && instr.source_command_boundary.is_start()
            && !f
                .entered_commands
                .iter()
                .any(|entered| entered.resume <= f.pc && f.pc < entered.continuation)
            && !f
                .entered_native_compiler_selections
                .iter()
                .any(|(start, end)| *start <= f.pc && f.pc < *end)
            // A compiled operation selected before its arguments ran finishes
            // as selected, as C's runs on past `INST_START_CMD`.
            && !f
                .entered_native_operations
                .iter()
                .any(|entered| entered.start < f.pc && f.pc < entered.end)
            && f.command_epoch != self.trace_deopt_epoch()
            && !self.release_keeps_running(f, &asm, instr)
        {
            if self.active_native_bindings_match(f, &asm) && !self.step_trace_active() {
                f.command_epoch = self.trace_deopt_epoch();
            } else if self.leave_stale_splice_range(f) {
                return ControlFlow::Break(Tick::Continue);
            } else {
                return ControlFlow::Break(self.replay_stale_command(f, &asm, instr));
            }
        }
        if let Err(completion) =
            self.capture_entered_command(f, &asm, instr, f.pc.saturating_add(1))
        {
            return ControlFlow::Break(Tick::Return(completion));
        }
        if instr
            .completion_option_scope
            .is_some_and(tcl_runtime_api::completion_options::ActivationOptionScope::begins_fresh)
        {
            f.last_options = Value::empty();
        }
        if instr.script_command {
            f.script_command = Some(f.pc);
        }
        // C counts a command it compiled inline at `INST_START_CMD`, and one it
        // invokes as it dispatches it, so an armed `commands` limit also
        // bounds a loop of compiled commands that never dispatches.
        if instr.source_command_boundary.is_start()
            && self.command_limit_armed()
            && !command_is_dispatched(&asm, f.pc)
            && let Some(exceeded) = self.charge_command()
        {
            return ControlFlow::Break(Tick::Return(exceeded));
        }
        f.pc += 1;
        // Line-watch seam: keep the embedder's cell on the dispatching
        // instruction's source line (see `Vm::set_line_watch`).
        self.note_line(instr.source_line);
        // Step-debugger seam: fire once per source command, before it runs.
        #[allow(clippy::redundant_closure_for_method_calls)] // Span isn't named in this crate.
        let span_start = instr.source_span.map(|s| s.start());
        if self.debug_step(instr.source_line, span_start, &instr.source_cmd_text) {
            return ControlFlow::Break(Tick::Return(Completion::new(
                Code::Error,
                Value::string("debug: terminated"),
                Value::empty(),
            )));
        }
        ControlFlow::Continue(asm)
    }

    /// Recompile the source command at its original boundary and resume after it.
    /// The caller has already performed entry and result-owner bookkeeping.
    fn replay_stale_command(
        &mut self,
        f: &mut Frame,
        asm: &Rc<FunctionAsm>,
        instr: &Instruction,
    ) -> Tick {
        if let Some(host) = instr.start_hosts.first() {
            return self.replay_start_host(f, asm, host);
        }
        if self
            .resolve_compiled_namespace_context(&instr.source_namespace_context())
            .is_none()
        {
            return Tick::Return(err("native replay namespace context is unavailable"));
        }
        let child = match self.compile_plain_function_cached_bytes(
            tcl_runtime_api::ScriptCompileTargetBytes {
                source: &instr.source_cmd_text,
                namespace: &instr.source_command_namespace,
            },
        ) {
            Ok(asm) => asm,
            Err(error) => {
                return Tick::Return(crate::command::completion_from_tcl_error(self, error));
            }
        };
        let mut after = f.pc + 1;
        while after < asm.instructions.len() {
            let candidate = &asm.instructions[after];
            if candidate.source_command_boundary.is_start() {
                // A command substitution has its own executable source
                // boundary inside the enclosing command. Replaying the
                // enclosing command already executes that substitution,
                // so resuming at the nested boundary would execute the
                // outer command repeatedly and eventually consume a
                // half-rebuilt operand stack. The compiler's exact spans
                // give the nesting relation without naming either
                // command; the first non-contained boundary is this
                // command's continuation.
                let nested =
                    instr
                        .source_span
                        .zip(candidate.source_span)
                        .is_some_and(|(outer, inner)| {
                            outer.start() <= inner.start() && inner.end() <= outer.end()
                        });
                if !nested {
                    break;
                }
            }
            after += 1;
        }
        // Resume on this command's trailing POP so the transparent
        // child's result becomes the parent's ordinary last result.
        f.pc = if after > f.pc + 1 && asm.instructions[after - 1].op == Op::POP {
            after - 1
        } else {
            after
        };
        Tick::PushScript {
            script: Box::new(self.compiled_unit(child, instr.source_command_namespace.clone())),
            label: None,
            cleanup_proc: None,
            fatal_tail: None,
            namespace: ScriptNamespace::CommandBoundary(instr.source_namespace_context()),
        }
    }
}
