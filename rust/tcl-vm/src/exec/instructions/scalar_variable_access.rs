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

//! Scalar and array-element loads, stores and increments, by stack name and by local slot.

use super::super::completion::logged_command;
use super::super::{Frame, Tick};
use super::operands::{imm_at, imm0, pop};
use crate::interp::Vm;
use crate::value::Value;
use tcl_bytecode::{Instruction, Op};
use tcl_runtime_api::Code;

impl Vm {
    /// The scalar and array-element loads, stores and increments of
    /// [`self.tick_variable_instruction`]: the instructions recursion through
    /// a variable trace most often passes, in a frame of their own.
    #[inline(never)]
    pub(super) fn tick_scalar_variable_instruction(
        &mut self,
        f: &mut Frame,
        instr: &Instruction,
    ) -> Tick {
        match instr.op {
            Op::LOAD_STK
            | Op::LOAD_SCALAR_STK
            | Op::STORE_STK
            | Op::STORE_SCALAR_STK
            | Op::INCR_STK_IMM
            | Op::INCR_SCALAR_STK_IMM
            | Op::INCR_STK
            | Op::INCR_SCALAR_STK => self.tick_stack_scalar_variable_instruction(f, instr),
            Op::LOAD_SCALAR1
            | Op::LOAD_SCALAR4
            | Op::STORE_SCALAR1
            | Op::STORE_SCALAR4
            | Op::INCR_SCALAR1
            | Op::INCR_SCALAR1_IMM
            | Op::INCR_SCALAR4
            | Op::INCR_SCALAR4_IMM => self.tick_local_scalar_variable_instruction(f, instr),
            Op::INCR_ARRAY_STK_IMM
            | Op::INCR_ARRAY_STK
            | Op::LOAD_ARRAY_STK
            | Op::STORE_ARRAY_STK => self.tick_stack_array_variable_instruction(f, instr),
            Op::INCR_ARRAY1
            | Op::INCR_ARRAY1_IMM
            | Op::INCR_ARRAY4
            | Op::INCR_ARRAY4_IMM
            | Op::LOAD_ARRAY1
            | Op::LOAD_ARRAY4
            | Op::STORE_ARRAY1
            | Op::STORE_ARRAY4 => self.tick_local_array_variable_instruction(f, instr),
            _ => Tick::Return(
                self.refuse_host_command("variable instruction dispatch is inconsistent".into()),
            ),
        }
    }

    fn tick_stack_scalar_variable_instruction(
        &mut self,
        f: &mut Frame,
        instr: &Instruction,
    ) -> Tick {
        // Like `try_op!`, but logs this instruction's command-source frame to
        // `errorInfo` before unwinding — for compiled command-boundary ops
        // (`set`/`incr` → STORE_*/INCR_*) so a failing compiled command
        // contributes its `while executing`/`invoked from within "<cmd>"`
        // frame just as the INVOKE path and reference Tcl's bytecode engine do
        // (set-2.4: a write-trace rejection's `errorInfo` must reach the
        // triggering `set x 1`).
        macro_rules! try_cmd {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(c) => {
                        // C logs only an error; a `TCL_RETURN` carrying `-code
                        // error` is raised at the procedure boundary, which
                        // logs the call instead.
                        if c.code == Code::Error {
                            let (cmd_text, line, enclosing) = logged_command(instr);
                            let msg = c.result.string_bytes();
                            self.log_command_with_enclosing(
                                &cmd_text,
                                Value::from_native_string_bytes(cmd_text.clone()),
                                &enclosing,
                                &msg,
                                line,
                            );
                        }
                        return Tick::Return(c);
                    }
                }
            };
        }

        match instr.op {
            // Variables (stack form, by name)
            // The `*ScalarStk` opcodes share their C `TEBCresume` case with the
            // general `*Stk` form (the compiler emits them when the name is known
            // to carry no `(index)` part), so they are the same arm here.
            Op::LOAD_STK | Op::LOAD_SCALAR_STK => {
                let name = variable_operand_bytes!(self, pop(f));
                let value = match self.direct_read(self.direct_named_scalar(&name)) {
                    Some(value) => value,
                    None => variable_try_op!(self.read_variable_result_bytes(&name, None)),
                };
                f.stack.push(value);
            }
            Op::STORE_STK | Op::STORE_SCALAR_STK => {
                let value = pop(f);
                let name = variable_operand_bytes!(self, pop(f));
                let stored = match self.direct_store(self.direct_named_scalar(&name)) {
                    Some(id) => self.direct_write(id, value),
                    None => try_cmd!(self.store_var_result_bytes(&name, value)),
                };
                f.stack.push(stored);
            }
            Op::INCR_STK_IMM | Op::INCR_SCALAR_STK_IMM => {
                let name = variable_operand_bytes!(self, pop(f));
                let amount = Value::int(i64::from(imm0(instr)));
                let direct = self.direct_store(self.direct_named_scalar(&name));
                let updated = match self.direct_increment(direct, &amount) {
                    Some(updated) => updated,
                    None => variable_try_op!(self.increment_captured_bytes(&name, None, &amount)),
                };
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::INCR_STK | Op::INCR_SCALAR_STK => {
                let amount = pop(f);
                let name = variable_operand_bytes!(self, pop(f));
                let direct = self.direct_store(self.direct_named_scalar(&name));
                let updated = match self.direct_increment(direct, &amount) {
                    Some(updated) => updated,
                    None => variable_try_op!(self.increment_captured_bytes(&name, None, &amount)),
                };
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            _ => {
                return Tick::Return(
                    self.refuse_host_command(
                        "variable instruction dispatch is inconsistent".into(),
                    ),
                );
            }
        }

        Tick::Continue
    }

    fn tick_local_scalar_variable_instruction(
        &mut self,
        f: &mut Frame,
        instr: &Instruction,
    ) -> Tick {
        match instr.op {
            Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4 => {
                f.stack.push(variable_try_logged!(
                    self,
                    instr,
                    self.read_compiled_variable_result(
                        usize::try_from(imm0(instr)).expect("local slot index"),
                        None
                    )
                ));
            }
            Op::STORE_SCALAR1 | Op::STORE_SCALAR4 => {
                let value = pop(f);
                let stored = variable_try_logged!(
                    self,
                    instr,
                    self.store_compiled_variable_result(
                        usize::try_from(imm0(instr)).expect("local slot index"),
                        None,
                        value
                    )
                );
                f.stack.push(stored);
            }
            Op::INCR_SCALAR1 | Op::INCR_SCALAR4 => {
                let amount = pop(f);
                let updated = variable_try_logged!(
                    self,
                    instr,
                    self.increment_compiled_variable(
                        usize::try_from(imm0(instr)).expect("local slot index"),
                        None,
                        &amount
                    )
                );
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::INCR_SCALAR1_IMM | Op::INCR_SCALAR4_IMM => {
                let amount = Value::int(i64::from(imm_at(instr, 1)));
                let updated = variable_try_logged!(
                    self,
                    instr,
                    self.increment_compiled_variable(
                        usize::try_from(imm0(instr)).expect("local slot index"),
                        None,
                        &amount
                    )
                );
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            _ => {
                return Tick::Return(
                    self.refuse_host_command(
                        "variable instruction dispatch is inconsistent".into(),
                    ),
                );
            }
        }

        Tick::Continue
    }

    fn tick_stack_array_variable_instruction(
        &mut self,
        f: &mut Frame,
        instr: &Instruction,
    ) -> Tick {
        match instr.op {
            Op::INCR_ARRAY_STK_IMM => {
                let key = variable_operand_bytes!(self, pop(f));
                let name = variable_operand_bytes!(self, pop(f));
                let amount = Value::int(i64::from(imm0(instr)));
                let updated = variable_try_logged!(
                    self,
                    instr,
                    self.increment_captured_bytes(&name, Some(&key), &amount)
                );
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::INCR_ARRAY_STK => {
                let amount = pop(f);
                let key = variable_operand_bytes!(self, pop(f));
                let name = variable_operand_bytes!(self, pop(f));
                let updated = variable_try_logged!(
                    self,
                    instr,
                    self.increment_captured_bytes(&name, Some(&key), &amount)
                );
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::LOAD_ARRAY_STK => {
                let key = variable_operand_bytes!(self, pop(f));
                let name = variable_operand_bytes!(self, pop(f));
                f.stack.push(variable_try_logged!(
                    self,
                    instr,
                    self.read_variable_result_bytes(&name, Some(&key))
                ));
            }
            Op::STORE_ARRAY_STK => {
                let value = pop(f);
                let key = variable_operand_bytes!(self, pop(f));
                let name = variable_operand_bytes!(self, pop(f));
                f.stack.push(variable_try_logged!(
                    self,
                    instr,
                    self.store_elem_result_bytes(&name, &key, value)
                ));
            }
            _ => {
                return Tick::Return(
                    self.refuse_host_command(
                        "variable instruction dispatch is inconsistent".into(),
                    ),
                );
            }
        }

        Tick::Continue
    }

    fn tick_local_array_variable_instruction(
        &mut self,
        f: &mut Frame,
        instr: &Instruction,
    ) -> Tick {
        match instr.op {
            Op::INCR_ARRAY1 | Op::INCR_ARRAY4 => {
                let amount = pop(f);
                let key = variable_operand_bytes!(self, pop(f));
                let updated = variable_try_logged!(
                    self,
                    instr,
                    self.increment_compiled_variable(
                        usize::try_from(imm0(instr)).expect("local slot index"),
                        Some(&key),
                        &amount
                    )
                );
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::INCR_ARRAY1_IMM | Op::INCR_ARRAY4_IMM => {
                let key = variable_operand_bytes!(self, pop(f));
                let amount = Value::int(i64::from(imm_at(instr, 1)));
                let updated = variable_try_logged!(
                    self,
                    instr,
                    self.increment_compiled_variable(
                        usize::try_from(imm0(instr)).expect("local slot index"),
                        Some(&key),
                        &amount
                    )
                );
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::LOAD_ARRAY1 | Op::LOAD_ARRAY4 => {
                let key = variable_operand_bytes!(self, pop(f));
                f.stack.push(variable_try_logged!(
                    self,
                    instr,
                    self.read_compiled_variable_result(
                        usize::try_from(imm0(instr)).expect("local slot index"),
                        Some(&key)
                    )
                ));
            }
            Op::STORE_ARRAY1 | Op::STORE_ARRAY4 => {
                let value = pop(f);
                let key = variable_operand_bytes!(self, pop(f));
                f.stack.push(variable_try_logged!(
                    self,
                    instr,
                    self.store_compiled_variable_result(
                        usize::try_from(imm0(instr)).expect("local slot index"),
                        Some(&key),
                        value
                    )
                ));
            }
            _ => {
                return Tick::Return(
                    self.refuse_host_command(
                        "variable instruction dispatch is inconsistent".into(),
                    ),
                );
            }
        }

        Tick::Continue
    }
}
