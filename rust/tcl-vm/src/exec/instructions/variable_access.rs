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

//! The variable instructions that can run traces: existence, array shape, unset, append, constants, links, loop variables and dictionary variables.

use super::super::{DictIterState, ForeachState, Frame, Tick};
use super::operands::{imm_at, imm0, pop};
use crate::interp::{Vm, err};
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::{FunctionAsm, Instruction, Op};

impl Vm {
    /// The variable-access arms of [`Self::tick_instruction`]. A variable
    /// trace (and an `upvar`/`dict`/`foreach` write that fires one) runs its
    /// callback while the instruction is still executing, so recursion
    /// through a trace keeps this frame, not the whole opcode match, on the
    /// native stack.
    #[inline(never)]
    pub(super) fn tick_variable_instruction(
        &mut self,
        f: &mut Frame,
        asm: &Rc<FunctionAsm>,
        instr: &Instruction,
    ) -> Tick {
        match instr.op {
            Op::EXIST_SCALAR | Op::EXIST_STK | Op::EXIST_ARRAY | Op::EXIST_ARRAY_STK => {
                self.tick_variable_existence(f, instr)
            }
            Op::ARRAY_EXISTS_IMM
            | Op::ARRAY_EXISTS_STK
            | Op::ARRAY_MAKE_IMM
            | Op::ARRAY_MAKE_STK => self.tick_variable_array_shape(f, instr),
            Op::UNSET_STK | Op::UNSET_SCALAR | Op::UNSET_ARRAY | Op::UNSET_ARRAY_STK => {
                self.tick_variable_unset(f, instr)
            }
            Op::APPEND_SCALAR1
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
            | Op::LAPPEND_ARRAY_STK => self.tick_variable_append_value(f, instr),
            Op::LAPPEND_LIST
            | Op::LAPPEND_LIST_STK
            | Op::LAPPEND_LIST_ARRAY
            | Op::LAPPEND_LIST_ARRAY_STK => self.tick_variable_append_list(f, instr),
            Op::CONST_IMM | Op::CONST_STK => self.tick_variable_constant(f, instr),
            Op::NSUPVAR | Op::UPVAR | Op::VARIABLE => self.tick_variable_link(f, instr),
            Op::FOREACH_START => self.tick_variable_loop_start(f, instr),
            Op::FOREACH_STEP | Op::LMAP_COLLECT => self.tick_variable_loop_step(f, instr),
            Op::FOREACH_END => self.tick_variable_loop_end(f, instr),
            Op::DICT_FIRST | Op::DICT_NEXT => self.tick_variable_dictionary_iteration(f, instr),
            Op::DICT_UPDATE_START | Op::DICT_UPDATE_END => {
                self.tick_variable_dictionary_update(f, instr)
            }
            Op::DICT_EXPAND | Op::DICT_RECOMBINE_IMM | Op::DICT_RECOMBINE_STK | Op::VERIFY_DICT => {
                self.tick_variable_dictionary_scope(f, asm, instr)
            }
            Op::DICT_PUT => self.tick_variable_dictionary_put(f, instr),
            Op::DICT_SET
            | Op::DICT_UNSET
            | Op::DICT_INCR_IMM
            | Op::DICT_APPEND
            | Op::DICT_LAPPEND => self.tick_variable_dictionary_write(f, asm, instr),
            _ => Tick::Return(
                self.refuse_host_command("variable instruction dispatch is inconsistent".into()),
            ),
        }
    }

    #[inline(never)]
    fn tick_variable_existence(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            Op::EXIST_SCALAR => {
                let found = variable_try_op!(self.exists_compiled_original_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None
                ));
                f.stack
                    .push(variable_try_op!(self.compiled_existence_result(found)));
            }
            Op::EXIST_STK => {
                let original = pop(f);
                let found = if self.native_c_variable_name_protocol().is_some() {
                    variable_try_op!(self.exists_original_c_parts(&original, None))
                } else {
                    let name = variable_operand_bytes!(self, original);
                    variable_try_op!(self.exists_var_traced_bytes(&name).map_err(|first| {
                        self.refuse_tcl_host_failure(crate::error::TclHostFailure::Execution(first))
                    }))
                };
                f.stack
                    .push(variable_try_op!(self.compiled_existence_result(found)));
            }
            Op::EXIST_ARRAY => {
                let index = pop(f);
                let found = variable_try_op!(self.exists_compiled_original_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&index)
                ));
                f.stack
                    .push(variable_try_op!(self.compiled_existence_result(found)));
            }
            Op::EXIST_ARRAY_STK => {
                let index = pop(f);
                let original = pop(f);
                let found = if self.native_c_variable_name_protocol().is_some() {
                    variable_try_op!(self.exists_original_c_parts(&original, Some(&index)))
                } else {
                    let index = variable_operand_bytes!(self, index);
                    let name = variable_operand_bytes!(self, original);
                    variable_try_op!(self.exists_elem_traced_bytes(&name, &index).map_err(
                        |first| self.refuse_tcl_host_failure(
                            crate::error::TclHostFailure::Execution(first)
                        )
                    ))
                };
                f.stack
                    .push(variable_try_op!(self.compiled_existence_result(found)));
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

    #[inline(never)]
    fn tick_variable_array_shape(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            Op::ARRAY_EXISTS_IMM => {
                let found = variable_try_op!(self.array_exists_compiled(
                    usize::try_from(imm0(instr)).expect("local slot index")
                ));
                let version = self.actual_native_invocation_dialect().tcl_version;
                let Some(version) = version else {
                    return Tick::Return(
                        self.refuse_host_command("native array existence issuer".into()),
                    );
                };
                let result =
                    variable_try_op!(self.native_c_execution_boolean(found, version).map_err(
                        |error| crate::command::completion_from_cmd_error(self, error.into())
                    ));
                f.stack.push(result);
            }
            Op::ARRAY_EXISTS_STK => {
                let original = pop(f);
                let found = variable_try_op!(self.array_exists_original_opcode(&original));
                let Some(version) = self.actual_native_invocation_dialect().tcl_version else {
                    return Tick::Return(
                        self.refuse_host_command("native Array constant issuer".into()),
                    );
                };
                let result =
                    variable_try_op!(self.native_c_execution_boolean(found, version).map_err(
                        |error| crate::command::completion_from_cmd_error(self, error.into())
                    ));
                f.stack.push(result);
            }
            // `array set`'s materialising half (C `INST_ARRAY_MAKE_*`): make the
            // variable an empty array when undefined, no-op when it already is
            // one, and error on a scalar or an array element.
            Op::ARRAY_MAKE_IMM => {
                variable_try_op!(self.ensure_array_compiled(
                    usize::try_from(imm0(instr)).expect("local slot index")
                ));
            }
            Op::ARRAY_MAKE_STK => {
                let original = pop(f);
                variable_try_op!(self.array_make_original_opcode(&original));
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

    #[inline(never)]
    fn tick_variable_unset(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            Op::UNSET_STK => {
                // operand = flags; a non-zero flag means "complain" (error when
                // the variable is absent). The name (possibly `a(k)`) is on TOS.
                let complain = imm0(instr) != 0;
                let name = pop(f);
                variable_try_op!(self.unset_original_named_variable(&name, complain));
            }
            Op::UNSET_SCALAR => {
                // Operands [flags, slot]; flags bit 0 ⇒ complain when absent.
                f.dict_iters.remove(&imm_at(instr, 1));
                let complain = imm0(instr) != 0;
                variable_try_op!(self.unset_compiled_variable(
                    usize::try_from(imm_at(instr, 1)).expect("local slot index"),
                    None,
                    complain
                ));
            }
            Op::UNSET_ARRAY => {
                // Operands [flags, slot]; the element key is on TOS. C
                // `INST_UNSET_ARRAY` reads the flags operand
                // (`flags = TclGetUInt1AtPtr(pc + 1) ? TCL_LEAVE_ERR_MSG : 0`,
                // `tclExecute.c:3814`) and, when it is set and the element is
                // absent, deliberately falls to `slowUnsetArray` (3837-3839,
                // 3851-3862) so the miss is reported. Ignoring the operand made
                // `unset B(nope)` silently succeed while `UNSET_SCALAR`/
                // `UNSET_STK` complained.
                let complain = imm0(instr) != 0;
                let key_original = pop(f);
                let key = variable_operand_bytes!(
                    self,
                    key_original.native_lifetime_lease().into_value()
                );
                variable_try_op!(self.unset_compiled_variable(
                    usize::try_from(imm_at(instr, 1)).expect("local slot index"),
                    Some(&key),
                    complain
                ));
            }
            Op::UNSET_ARRAY_STK => {
                // Operand = flags (non-zero ⇒ C's `TCL_LEAVE_ERR_MSG`: complain
                // when the element is absent); the key is on TOS over the array
                // name, matching C's `part2Ptr = OBJ_AT_TOS` /
                // `part1Ptr = OBJ_UNDER_TOS` (`tclExecute.c:3866-3872`).
                let complain = imm0(instr) != 0;
                let key = pop(f);
                let name = pop(f);
                if self.native_c_variable_name_protocol().is_some() {
                    // A directly evaluated statement runs the generic `unset`,
                    // whose trace names the joined spelling.
                    let direct = f
                        .script_command
                        .and_then(|start| f.asm.instructions.get(start))
                        .and_then(|start| start.script_command.as_ref())
                        .map(|command| command.span)
                        .zip(instr.source_span)
                        .is_some_and(|(command, own)| {
                            command.start() <= own.start() && own.end() <= command.end()
                        });
                    variable_try_op!(
                        self.unset_original_c_variable_parts_reporting(
                            &name, &key, complain, !direct
                        )
                    );
                } else {
                    let key_bytes =
                        variable_operand_bytes!(self, key.native_lifetime_lease().into_value());
                    let name_bytes =
                        variable_operand_bytes!(self, name.native_lifetime_lease().into_value());
                    variable_try_op!(self.unset_array_elem_checked_bytes(
                        &name_bytes,
                        &key_bytes,
                        complain
                    ));
                }
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

    #[inline(never)]
    fn tick_variable_append_value(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            // Single-value append opcodes do not fire a read trace; list
            // expansion opcodes use the retained-cell read/update protocol.
            Op::APPEND_SCALAR1 | Op::APPEND_SCALAR4 => {
                let value = pop(f);
                f.stack.push(variable_try_op!(self.append_compiled_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None,
                    &[value]
                )));
            }
            Op::APPEND_ARRAY1 | Op::APPEND_ARRAY4 => {
                let value = pop(f);
                let key = variable_operand_bytes!(self, pop(f));
                f.stack.push(variable_try_op!(self.append_compiled_variable(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&key),
                    &[value]
                )));
            }
            Op::LAPPEND_SCALAR1 | Op::LAPPEND_SCALAR4 => {
                let value = pop(f);
                f.stack.push(variable_try_op!(self.lappend_compiled_single(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None,
                    &value
                )));
            }
            Op::LAPPEND_ARRAY1 | Op::LAPPEND_ARRAY4 => {
                let value = pop(f);
                let key = variable_operand_bytes!(self, pop(f));
                f.stack.push(variable_try_op!(self.lappend_compiled_single(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&key),
                    &value
                )));
            }
            Op::APPEND_STK | Op::LAPPEND_STK => {
                let value = pop(f);
                let name = variable_operand_bytes!(self, pop(f));
                let stored = if instr.op == Op::APPEND_STK {
                    self.append_variable_bytes(&name, None, value)
                } else {
                    self.lappend_instruction_single_bytes(&name, None, &value)
                };
                f.stack.push(variable_try_op!(stored));
            }
            Op::APPEND_ARRAY_STK | Op::LAPPEND_ARRAY_STK => {
                let value = pop(f);
                let key = variable_operand_bytes!(self, pop(f));
                let name = variable_operand_bytes!(self, pop(f));
                let stored = if instr.op == Op::APPEND_ARRAY_STK {
                    self.append_variable_bytes(&name, Some(&key), value)
                } else {
                    self.lappend_instruction_single_bytes(&name, Some(&key), &value)
                };
                f.stack.push(variable_try_op!(stored));
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

    #[inline(never)]
    fn tick_variable_append_list(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            Op::LAPPEND_LIST => {
                let additions = pop(f);
                let updated = variable_try_op!(self.lappend_instruction_compiled_list(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    None,
                    &additions
                ));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::LAPPEND_LIST_STK => {
                let additions = pop(f);
                let name = variable_operand_bytes!(self, pop(f));
                let updated =
                    variable_try_op!(self.lappend_instruction_list_bytes(&name, None, &additions));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::LAPPEND_LIST_ARRAY => {
                let additions = pop(f);
                let key = variable_operand_bytes!(self, pop(f));
                let updated = variable_try_op!(self.lappend_instruction_compiled_list(
                    usize::try_from(imm0(instr)).expect("local slot index"),
                    Some(&key),
                    &additions
                ));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            Op::LAPPEND_LIST_ARRAY_STK => {
                let additions = pop(f);
                let key = variable_operand_bytes!(self, pop(f));
                let name = variable_operand_bytes!(self, pop(f));
                let updated = variable_try_op!(self.lappend_instruction_list_bytes(
                    &name,
                    Some(&key),
                    &additions
                ));
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

    #[inline(never)]
    fn tick_variable_constant(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            Op::CONST_IMM => {
                let value = pop(f);
                variable_try_op!(self.declare_compiled_constant(
                    usize::try_from(imm0(instr)).unwrap_or(usize::MAX),
                    value
                ));
            }
            Op::CONST_STK => {
                let value = pop(f);
                let name = variable_operand_bytes!(self, pop(f));
                variable_try_op!(self.declare_constant_bytes(&name, value, false));
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

    #[inline(never)]
    fn tick_variable_link(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            // -- global / upvar links. The namespace ("::") or level reference
            // is left on the stack for the next link op to reuse; a trailing
            // `pop` discards it (matching C Tcl's nsupvar/upvar codegen). --
            Op::NSUPVAR => {
                let other = pop(f);
                let Some(namespace_operand) = f.stack.last() else {
                    return Tick::Return(err("nsupvar: stack underflow"));
                };
                let namespace =
                    variable_try_core!(self, self.namespace_object_lookup(namespace_operand));
                let Some(namespace) = namespace else {
                    let namespace_bytes = variable_operand_bytes!(
                        self,
                        namespace_operand.native_lifetime_lease().into_value()
                    );
                    return Tick::Return(self.namespace_lookup_error_bytes(&namespace_bytes));
                };
                variable_try_op!(self.link_compiled_namespace_original(
                    usize::try_from(imm0(instr)).unwrap_or(usize::MAX),
                    namespace,
                    &other,
                    false,
                ));
            }
            Op::UPVAR => {
                let other = pop(f);
                let Some(level_operand) = f
                    .stack
                    .last()
                    .map(|value| value.native_lifetime_lease().into_value())
                else {
                    return Tick::Return(err("upvar: stack underflow"));
                };
                // C's INST_UPVAR reads its level with TclObjGetFrame itself:
                // a word that is no level takes the default, as for `uplevel`.
                let target = variable_try_op!(crate::command::runtime_explicit_frame_selection(
                    self,
                    &level_operand
                ));
                variable_try_op!(self.link_compiled_upvar(
                    usize::try_from(imm0(instr)).unwrap_or(usize::MAX),
                    target,
                    &other,
                ));
            }
            // `variable` (C `INST_VARIABLE`): link the local slot to the
            // namespace variable named by the popped original object. Namespace-only
            // lookup updates its selected name cache before binding the local slot.
            Op::VARIABLE => {
                let original = pop(f);
                variable_try_op!(self.link_compiled_namespace_original(
                    usize::try_from(imm0(instr)).unwrap_or(usize::MAX),
                    self.current_ns_id(),
                    &original,
                    true,
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

    #[inline(never)]
    fn tick_variable_loop_start(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            // -- foreach (C Tcl INST_FOREACH_*): aux var groups on the
            //    instruction; foreach_start jumps to step; step binds the loop
            //    vars and jumps back to the body, or falls through to end. --
            Op::FOREACH_START => {
                if let Some(auxiliary) = &instr.native_each {
                    return self.native_compiled_each_start(f, auxiliary, instr.foreach_collect);
                }
                let policy = if instr.foreach_collect {
                    tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_FORWARDED
                } else {
                    tcl_runtime_api::completion_options::ControlOptionPolicy::FRESH_SETTLED
                };
                if policy.begins_fresh() {
                    f.last_options = Value::empty();
                }
                let start_idx = f.pc - 1;
                let body_idx = f.pc;
                let groups = instr.foreach_vars.clone().unwrap_or_default();
                let n = groups.len();
                if f.stack.len() < n {
                    return Tick::Return(err("foreach: stack underflow"));
                }
                let raw = f.stack.split_off(f.stack.len() - n);
                let mut value_lists = Vec::with_capacity(n);
                let mut iter_max = 0;
                let Some(protocol) = self.object_protocol() else {
                    return Tick::Return(self.refuse_host_command(
                        "native foreach list protocol is unavailable".into(),
                    ));
                };
                for (gi, lv) in raw.iter().enumerate() {
                    let items = match self.native_object_list_elements_in(lv, protocol) {
                        Ok(items) => items,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self,
                                error.into(),
                            ));
                        }
                    };
                    let nv = groups[gi].len().max(1);
                    iter_max = iter_max.max(items.len().div_ceil(nv));
                    value_lists.push(items);
                }
                f.foreach_stack.push(ForeachState {
                    native: None,
                    _native_iterator_headers: None,
                    native_accumulator: None,
                    var_groups: groups,
                    lists: value_lists,
                    _list_roots: raw,
                    iter_num: 0,
                    iter_max,
                    body_idx,
                    collect: instr.foreach_collect,
                    accum: Vec::new(),
                });
                if let Some(&step) = f.foreach_pairs.get(&start_idx) {
                    f.pc = step;
                }
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

    #[inline(never)]
    fn tick_variable_loop_step(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            Op::FOREACH_STEP => {
                if f.foreach_stack
                    .last()
                    .is_some_and(|state| state.native.is_some())
                {
                    return self.native_compiled_each_step(f);
                }
                let (binds, body, more) = {
                    match f.foreach_stack.last_mut() {
                        None => (Vec::new(), 0usize, false),
                        Some(st) if st.iter_num >= st.iter_max => (Vec::new(), 0, false),
                        Some(st) => {
                            let it = st.iter_num;
                            let mut binds: Vec<(tcl_bytecode::CompiledVariableTarget, Value)> =
                                Vec::new();
                            for (gi, group) in st.var_groups.iter().enumerate() {
                                let nv = group.len();
                                for (j, var) in group.iter().enumerate() {
                                    let v = st.lists[gi]
                                        .get(it * nv + j)
                                        .cloned()
                                        .unwrap_or_else(Value::empty);
                                    binds.push((var.clone(), v));
                                }
                            }
                            st.iter_num += 1;
                            (binds, st.body_idx, true)
                        }
                    }
                };
                if more {
                    f.last_options = Value::empty();
                    for (name, v) in binds {
                        match name {
                            tcl_bytecode::CompiledVariableTarget::Slot(slot) => {
                                variable_try_op!(
                                    self.store_compiled_variable_result(slot, None, v)
                                );
                            }
                            tcl_bytecode::CompiledVariableTarget::Name(name) => {
                                variable_try_op!(self.set_var_bytes(name.as_bytes(), v));
                            }
                        }
                    }
                    f.pc = body;
                }
            }
            Op::LMAP_COLLECT => {
                // Append this fall-through iteration's result to the collecting
                // loop's accumulator. A `break`/`continue` redirect jumps past this
                // opcode (to `FOREACH_END` / `FOREACH_STEP`), so a skipped iteration
                // contributes nothing — matching C `lmap`.
                let v = pop(f);
                if let Some(st) = f.foreach_stack.last_mut() {
                    if let (Some(accumulator), Some(auxiliary)) =
                        (&st.native_accumulator, &st.native)
                    {
                        let protocol =
                            tcl_syntax::native_string::NativeStringProtocol::C(auxiliary.version);
                        if let Err(error) =
                            accumulator.native_list_append_prepared_elements(&[v], protocol)
                        {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self,
                                error.into(),
                            ));
                        }
                    } else {
                        st.accum.push(v);
                    }
                }
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

    #[inline(never)]
    fn tick_variable_loop_end(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            Op::FOREACH_END => {
                let st = f.foreach_stack.pop();
                // A collecting loop (`lmap`) yields `list(accum)`; a plain
                // `foreach` yields nothing (its `""` result is pushed by the
                // loop-end block).
                if let Some(st) = st {
                    if st.collect {
                        f.stack.push(
                            st.native_accumulator
                                .unwrap_or_else(|| Value::list(st.accum)),
                        );
                    } else {
                        f.last_options = Value::empty();
                    }
                }
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

    #[inline(never)]
    fn tick_variable_dictionary_iteration(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            // Compiled `dict for` / `dict map` iteration primitives. `dictFirst
            // <slot>` pops the dict (TOS), records an iterator in `<slot>`, and
            // pushes `value, key, done` (done on top). `dictNext <slot>`
            // advances and pushes the next `value, key, done`. `done` is the
            // "exhausted" flag the compiled loop tests with `jumpTrue`
            // (dictFirst: skip an empty dict) / `jumpFalse` (dictNext: loop
            // while more). On exhaustion the pushed key/value are empty
            // placeholders the loop epilogue pops.
            Op::DICT_FIRST => {
                let slot = imm0(instr);
                let dict = pop(f);
                let Some(protocol) = self.object_protocol() else {
                    return Tick::Return(self.refuse_host_command(
                        "native dictionary search protocol is unavailable".into(),
                    ));
                };
                let mut search = match dict.into_native_dictionary_search(protocol) {
                    Ok(search) => search,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                };
                #[cfg(test)]
                {
                    self.actual_dictionary_search_entries += 1;
                }
                let pair = match search.next_pair() {
                    Ok(pair) => pair,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                };
                let done = pair.is_none();
                let (k, v) = pair.unwrap_or_else(|| (Value::empty(), Value::empty()));
                f.dict_iters.insert(slot, DictIterState { search });
                f.stack.push(v);
                f.stack.push(k);
                f.stack.push(Value::bool(done));
            }
            Op::DICT_NEXT => {
                let pair = match f.dict_iters.get_mut(&imm0(instr)) {
                    Some(state) => match state.search.next_pair() {
                        Ok(pair) => pair,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self,
                                error.into(),
                            ));
                        }
                    },
                    None => None,
                };
                let done = pair.is_none();
                let (k, v) = pair.unwrap_or_else(|| (Value::empty(), Value::empty()));
                f.stack.push(v);
                f.stack.push(k);
                f.stack.push(Value::bool(done));
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

    #[inline(never)]
    fn tick_variable_dictionary_update(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            Op::DICT_UPDATE_START => {
                let slot = usize::try_from(imm0(instr)).unwrap_or(usize::MAX);
                let vars = instr.dict_vars.as_deref().unwrap_or_default();
                let Some(keys) = f.stack.last() else {
                    return Tick::Return(err("dictUpdateStart: stack underflow"));
                };
                variable_try_op!(self.start_compiled_dictionary_update(slot, keys, vars));
            }
            Op::DICT_UPDATE_END => {
                let slot = usize::try_from(imm0(instr)).unwrap_or(usize::MAX);
                let keys = pop(f);
                variable_try_op!(self.finish_compiled_dictionary_update(
                    slot,
                    &keys,
                    instr.dict_vars.as_deref().unwrap_or_default()
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

    #[inline(never)]
    fn tick_variable_dictionary_scope(
        &mut self,
        f: &mut Frame,
        asm: &Rc<FunctionAsm>,
        instr: &Instruction,
    ) -> Tick {
        let lvt = asm.lvt.entries();
        let lvt_name = |slot: i32| -> tcl_core_types::NameBytes {
            lvt.get(usize::try_from(slot).unwrap_or(usize::MAX))
                .cloned()
                .unwrap_or_default()
        };
        match instr.op {
            Op::DICT_EXPAND => {
                let path = pop(f);
                let dictionary = pop(f);
                let keys = variable_try_op!(crate::cmd_dict::expand_dictionary_scope(
                    self,
                    &dictionary,
                    &path
                ));
                f.stack.push(keys);
            }
            Op::DICT_RECOMBINE_IMM | Op::DICT_RECOMBINE_STK => {
                let state = pop(f);
                let path = pop(f);
                let (name, publication) = if instr.op == Op::DICT_RECOMBINE_STK {
                    (tcl_core_types::NameBytes::from(variable_operand_bytes!(self, pop(f)).as_ref()),
                     crate::interp::native_dictionary::DictionaryVariablePublication::RetainedLocalCell)
                } else {
                    (lvt_name(imm0(instr)), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(
                        usize::try_from(imm0(instr)).unwrap_or(usize::MAX)))
                };
                variable_try_op!(crate::cmd_dict::recombine_dictionary_scope(
                    self,
                    name.as_bytes(),
                    publication,
                    &path,
                    &state
                ));
            }
            Op::VERIFY_DICT => {
                let top = pop(f);
                match tcl_syntax::value::ValueOps::dict_pairs(self, &top) {
                    Ok(pairs) => drop(pairs),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                }
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

    #[inline(never)]
    fn tick_variable_dictionary_put(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
        match instr.op {
            // Dicts (LVT form, proc bodies)
            // `dict set var k1 ?k2 …? value` — operands [Imm(N), Imm(slot)];
            // stack holds the N keys then the value. Writes the variable and
            // leaves the new dict on the stack (the codegen POPs it).
            Op::DICT_PUT => {
                let value = pop(f);
                let key = pop(f);
                let original = pop(f);
                if !self
                    .actual_native_string_protocol()
                    .is_some_and(|protocol| {
                        protocol
                            .tcl_version()
                            .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_1)
                    })
                {
                    return Tick::Return(self.refuse_host_command(
                        "native dictionary-put recipe is unavailable".into(),
                    ));
                }
                let updated = variable_try_core!(
                    self,
                    crate::cmd_dict::put_original_member(self, &original, &key, value)
                );
                f.stack.push(updated);
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

    #[inline(never)]
    fn tick_variable_dictionary_write(
        &mut self,
        f: &mut Frame,
        asm: &Rc<FunctionAsm>,
        instr: &Instruction,
    ) -> Tick {
        let lvt = asm.lvt.entries();
        let lvt_name = |slot: i32| -> tcl_core_types::NameBytes {
            lvt.get(usize::try_from(slot).unwrap_or(usize::MAX))
                .cloned()
                .unwrap_or_default()
        };
        match instr.op {
            Op::DICT_SET => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                let name = lvt_name(imm_at(instr, 1));
                let value = pop(f);
                if f.stack.len() < nkeys || nkeys == 0 {
                    return Tick::Return(err("dictSet: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let updated = variable_try_op!(crate::cmd_dict::dictionary_path_update_bytes(self,
                    name.as_bytes(), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(usize::try_from(imm_at(instr, 1)).unwrap_or(usize::MAX)),
                    &keys, Some(value)));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            // `dict unset var k1 ?k2 …?` — operands [Imm(N), Imm(slot)]; stack
            // holds the N keys.
            Op::DICT_UNSET => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                let name = lvt_name(imm_at(instr, 1));
                if f.stack.len() < nkeys || nkeys == 0 {
                    return Tick::Return(err("dictUnset: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let updated = variable_try_op!(crate::cmd_dict::dictionary_path_update_bytes(self,
                    name.as_bytes(), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(usize::try_from(imm_at(instr, 1)).unwrap_or(usize::MAX)),
                    &keys, None));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            // `dict incr var key ?amount?` — operands [Imm(amount), Imm(slot)];
            // stack holds the key.
            Op::DICT_INCR_IMM => {
                let amount = i64::from(imm0(instr));
                let name = lvt_name(imm_at(instr, 1));
                let key = pop(f);
                let increment = Value::int(amount);
                let updated = variable_try_op!(crate::cmd_dict::dictionary_member_update_bytes(self,
                    name.as_bytes(), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(usize::try_from(imm_at(instr, 1)).unwrap_or(usize::MAX)),
                    &key, |vm, old| crate::cmd_dict::increment_dictionary_member(vm, old, &increment, true)));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            // `dict append var key value` — operand [Imm(slot)]; stack holds the
            // key then the value.
            Op::DICT_APPEND => {
                let name = lvt_name(imm0(instr));
                let value = pop(f);
                let key = pop(f);
                let updated = variable_try_op!(crate::cmd_dict::dictionary_member_update_bytes(self,
                    name.as_bytes(), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(usize::try_from(imm0(instr)).unwrap_or(usize::MAX)),
                    &key, |vm, old| crate::cmd_dict::append_compiled_member_value(vm, old, &value)));
                f.last_options = updated.options;
                f.stack.push(updated.value);
            }
            // `dict lappend var key value` — operand [Imm(slot)]; stack holds the
            // key then the value.
            Op::DICT_LAPPEND => {
                let name = lvt_name(imm0(instr));
                let value = pop(f);
                let key = pop(f);
                let updated = variable_try_op!(crate::cmd_dict::dictionary_member_update_bytes(self,
                    name.as_bytes(), crate::interp::native_dictionary::DictionaryVariablePublication::RetainedCompiledLocal(usize::try_from(imm0(instr)).unwrap_or(usize::MAX)),
                    &key, |vm, old| crate::cmd_dict::lappend_member_values(vm, old, &[value])));
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
}
