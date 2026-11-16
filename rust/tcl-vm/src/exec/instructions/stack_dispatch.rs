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

//! The stack instructions: literal pushes, stack shuffling, string concatenation and command-start markers.

use super::super::{Frame, ScriptNamespace, Tick};
use super::operands::{imm0, label_to_idx, label0};
use crate::interp::{Vm, err};
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::{FunctionAsm, Instruction, Op};

impl Vm {
    /// The stack arms of [`Self::tick_instruction`]: literal pushes, `pop`/`dup`/`over`, string concatenation and command-start markers.
    #[inline(never)]
    pub(super) fn tick_stack_instruction(
        &mut self,
        f: &mut Frame,
        asm: &Rc<FunctionAsm>,
        instr: &Instruction,
    ) -> Tick {
        // Portable operation failures retain byte diagnostics and host refusal tags.
        macro_rules! try_core {
            ($operation:expr) => {
                match $operation {
                    Ok(value) => value,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                }
            };
        }

        match instr.op {
            Op::PUSH1 | Op::PUSH4 => return self.tick_push_literal_instruction(f, asm, instr),
            Op::POP => {
                // Native POP discards TOS; only DONE or a command publisher
                // installs it as the interpreter result.
                drop(f.stack.pop());
                match self.with_native_interp_result(Value::native_lifetime_lease) {
                    Ok(result) => f.last_result = Some(result),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(
                            self,
                            error.into(),
                        ));
                    }
                }
            }
            Op::DUP => {
                if let Some(v) = f.stack.last().cloned() {
                    f.stack.push(v);
                }
            }
            Op::OVER => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                if let Some(i) = f.stack.len().checked_sub(n + 1) {
                    let v = f.stack[i].clone();
                    f.stack.push(v);
                }
            }
            Op::STR_CONCAT1 => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < n {
                    return Tick::Return(err("strcat: stack underflow"));
                }
                let parts = f.stack.split_off(f.stack.len() - n);
                if let Some(protocol) = self.actual_native_string_protocol()
                    && protocol.tcl_version().is_some()
                {
                    let value = try_core!(tcl_cmd_core::native_cat::concatenate_compiled(
                        &crate::value::VmAppendObjects,
                        protocol,
                        &parts
                    ));
                    f.stack.push(value);
                } else {
                    let mut bytes = Vec::new();
                    for part in &parts {
                        bytes.extend_from_slice(&part.string_bytes());
                    }
                    f.stack.push(Value::from_string_bytes(bytes));
                }
            }
            Op::START_CMD => return self.tick_start_command_instruction(f, asm, instr),
            Op::NOP => {}
            _ => unreachable!("tick_stack_instruction handles only stack opcodes"),
        }

        Tick::Continue
    }

    /// The `START_CMD` arm of [`Self::tick_stack_instruction`]: the command-start marker and its replay boundary.
    #[inline(never)]
    fn tick_start_command_instruction(
        &mut self,
        f: &mut Frame,
        asm: &Rc<FunctionAsm>,
        instr: &Instruction,
    ) -> Tick {
        match instr.op {
            Op::START_CMD => {
                let current_epoch = self.trace_deopt_epoch();
                // Compiler-only markers have no exact owning Tcl command and
                // therefore are not safe acknowledgement/replay points. Leave
                // the frame stale until a real source boundary is reached.
                if f.command_epoch != current_epoch && !instr.source_cmd_text.is_empty() {
                    if self.active_native_bindings_match(f, asm) && !self.step_trace_active() {
                        f.command_epoch = current_epoch;
                    } else {
                        let Some(target) =
                            label0(instr).and_then(|label| label_to_idx(asm, &f.off2idx, label))
                        else {
                            return Tick::Return(err(
                                "stale startCommand has no valid continuation",
                            ));
                        };
                        if self
                            .resolve_compiled_namespace_context(&instr.source_namespace_context())
                            .is_none()
                        {
                            return Tick::Return(err(
                                "native replay namespace context is unavailable",
                            ));
                        }
                        let child = match self.compile_plain_function_cached_bytes(
                            tcl_runtime_api::ScriptCompileTargetBytes {
                                source: &instr.source_cmd_text,
                                namespace: &instr.source_command_namespace,
                            },
                        ) {
                            Ok(asm) => asm,
                            Err(error) => {
                                return Tick::Return(crate::command::completion_from_tcl_error(
                                    self, error,
                                ));
                            }
                        };
                        // The transparent child produces this command's result
                        // on the parent stack; resume at the original
                        // startCommand continuation, skipping stale opcodes.
                        f.pc = target;
                        return Tick::PushScript {
                            script: Box::new(
                                self.compiled_unit(child, instr.source_command_namespace.clone()),
                            ),
                            label: None,
                            cleanup_proc: None,
                            fatal_tail: None,
                            namespace: ScriptNamespace::CommandBoundary(
                                instr.source_namespace_context(),
                            ),
                        };
                    }
                }
            }
            _ => unreachable!("tick_start_command_instruction handles only its own opcode"),
        }

        Tick::Continue
    }

    /// The `PUSH1`/`PUSH4` arm of [`Self::tick_stack_instruction`]: literal pushes.
    #[inline(never)]
    fn tick_push_literal_instruction(
        &mut self,
        f: &mut Frame,
        asm: &Rc<FunctionAsm>,
        instr: &Instruction,
    ) -> Tick {
        let lits = asm.literals.entries();

        match instr.op {
            Op::PUSH1 | Op::PUSH4 => {
                let raw = usize::try_from(imm0(instr))
                    .ok()
                    .and_then(|idx| lits.get(idx))
                    .cloned()
                    .unwrap_or_default();
                // A verbatim (braced / constant) literal suppresses all word
                // substitution — it is pushed exactly as the codegen interned
                // it (the codegen sets this flag for braced words / `proc`
                // bodies / constant assignments).
                if instr.push_verbatim {
                    let value = if let Some(direct) = f.direct_source_operands.as_deref().copied() {
                        let Some(literal) = usize::try_from(imm0(instr))
                            .ok()
                            .and_then(|index| lits.get(index))
                        else {
                            return Tick::Return(self.refuse_host_command(
                                "native DIRECT source operand index is unavailable".into(),
                            ));
                        };
                        direct.value(literal)
                    } else {
                        let pool = match &f.literal_pool {
                            Ok(pool) => pool,
                            Err(error) => {
                                return Tick::Return(self.refuse_host_command(error.to_string()));
                            }
                        };
                        let Some(value) = usize::try_from(imm0(instr))
                            .ok()
                            .and_then(|index| pool.value(index))
                        else {
                            return Tick::Return(self.refuse_host_command(
                                "native literal object-array index is unavailable".into(),
                            ));
                        };
                        value
                    };
                    if let Some(line) = instr.source_value_line {
                        let base = f.source_location.as_ref();
                        value.retain_source_location(
                            tcl_runtime_api::script_source_location::ScriptSourceLocation {
                                file: base
                                    .map_or("", |location| location.file.as_str())
                                    .to_string(),
                                line: base
                                    .map_or(1, |location| location.line)
                                    .saturating_add(line.saturating_sub(1)),
                            },
                        );
                    }
                    f.stack.push(value);
                } else {
                    match crate::subst::subst_word_bytes_as(raw.bytes(), self, instr.evaluation) {
                        // A stack slot owns its reference: a later command may
                        // reset the interpreter result this word came from.
                        Ok(v) => f.stack.push(v.into_native_reference()),
                        // A `break`/`continue`/`return` carried out of a `[…]`
                        // substitution propagates with its own code (an enclosing
                        // loop's exception range / a proc boundary handles it).
                        Err(e) => {
                            return Tick::Return(crate::command::completion_from_tcl_error(
                                self, e,
                            ));
                        }
                    }
                }
            }
            _ => unreachable!("tick_push_literal_instruction handles only its own opcode"),
        }

        Tick::Continue
    }
}
