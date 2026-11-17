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

//! `tick_instruction`: the match over every non-invoking opcode.

use super::super::{CatchRange, ExpressionReq, Frame, ScriptNamespace, Tick};
use super::operands::{
    bin, char_find, checked_index_value, encoded_string_index, get_at, ilen, imm_at, imm_index,
    imm0, irule, jump_target, label_to_idx, land_lor, native_numeric_type, pop, slice, un,
};
use crate::expr;
use crate::interp::native_operation_currency::NativeOperationScope;
use crate::interp::{Vm, err};
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::{FunctionAsm, Instruction, Op, Operand};
use tcl_runtime_api::{Code, Completion};
use tcl_syntax::expr::{BinOp, UnaryOp};

impl Vm {
    /// Execute one non-invoking instruction (`f.pc` is already past it).
    #[inline(never)]
    #[allow(clippy::too_many_lines)] // One match over every opcode; the VM's central dispatch is clearer whole.
    pub(in super::super) fn tick_instruction(
        &mut self,
        f: &mut Frame,
        asm: &Rc<FunctionAsm>,
        instr: &Instruction,
    ) -> Tick {
        // Unwrap a fallible step, unwinding the instruction on an error. The
        // value form lets a store arm push what the variable reads back.
        macro_rules! try_op {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(c) => return Tick::Return(c),
                }
            };
        }

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
            // Stack.
            Op::PUSH1
            | Op::PUSH4
            | Op::POP
            | Op::DUP
            | Op::OVER
            | Op::STR_CONCAT1
            | Op::START_CMD
            | Op::NOP => return self.tick_stack_instruction(f, asm, instr),

            // A `BEGIN_CATCH4` carrying an out-of-band handler label opens a
            // live catch range (C `INST_BEGIN_CATCH4` pushing the catchStack);
            // without one it is a *decorative* range — C-faithful reference
            // bytecode whose construct the VM protects via its activation
            // stack instead (`dict for`/`dict map`/`try` epilogues) — and
            // stays inert. The 4-byte operand keeps C's meaning (range index)
            // and is not consulted.
            Op::BEGIN_CATCH4 => {
                if let Some(idx) = instr
                    .catch_target
                    .as_deref()
                    .and_then(|label| label_to_idx(asm, &f.off2idx, label))
                {
                    f.catch_ranges.push(CatchRange {
                        target_idx: idx,
                        start_idx: instr
                            .catch_start
                            .as_deref()
                            .and_then(|label| label_to_idx(asm, &f.off2idx, label))
                            .unwrap_or(f.pc),
                        end_idx: instr
                            .catch_end
                            .as_deref()
                            .and_then(|label| label_to_idx(asm, &f.off2idx, label))
                            .unwrap_or(idx),
                        stack_len: f.stack.len(),
                        foreach_len: f.foreach_stack.len(),
                        expand_len: f.expand_markers.len(),
                        begin_idx: f.pc - 1,
                        in_handler: false,
                        caught: None,
                    });
                }
            }
            // `END_CATCH` closes the innermost range. A decorative
            // `BEGIN_CATCH4` pushed nothing, so its paired `END_CATCH` finds
            // the stack empty and stays a no-op.
            Op::END_CATCH => {
                if f.catch_ranges.pop().is_some() {
                    // The protected completion's options have already been read
                    // by PUSH_RETURN_OPTS. A live END_CATCH completes `catch`
                    // itself, whose successful integer result has ordinary
                    // options. Decorative try/dict cleanup ranges preserve the
                    // completion they are forwarding.
                    f.last_options = Value::empty();
                    // INST_END_CATCH resets the result: a later failure in the
                    // same command starts a fresh errorInfo.
                    self.reset_native_error_result();
                    let _ = self.take_error_info();
                }
            }

            // Lists (inline opcodes)
            Op::LIST => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < n {
                    return Tick::Return(err("list: stack underflow"));
                }
                let items = f.stack.split_off(f.stack.len() - n);
                let value = if let Some(protocol) = self.object_protocol() {
                    Value::native_list_constructor(items, protocol)
                } else {
                    Value::list(items)
                };
                f.stack.push(value);
            }
            Op::LIST_LENGTH if instr.native_switch_version.is_some() => {
                let original = pop(f);
                let result = try_core!(self.execute_native_scalar_length(
                    &original,
                    tcl_registry::native_scalar_compilation::NativeScalarOperation::ListLength,
                    instr.native_switch_version.unwrap(),
                ));
                drop(original);
                f.stack.push(result);
            }
            Op::LIST_LENGTH => {
                let l = pop(f);
                match self.object_list(&l) {
                    Ok(items) => f.stack.push(Value::int(ilen(items.len()))),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                }
            }
            // `[lindex $list $i]` — C `INST_LIST_INDEX` (`tclExecute.c:4696-4768`).
            // The integer fast path is only a fast path: when the index object
            // does not parse as one, C falls through to
            // `TclLindexList(interp, valuePtr, value2Ptr)` (line 4754), which
            // treats the index word as an index *list* — so a list-valued index
            // drills nested sublists (`lindex {{a b} {c d}} {1 0}` → `c`), an
            // empty index list is the whole list, and a spec that is neither
            // (`foo`) is `bad index "foo": …` (`objResultPtr == NULL` →
            // `goto gotError`, 4758-4761). That is exactly the runtime `lindex`
            // command's core, so both paths single-source it.
            Op::LIST_INDEX => {
                let idx = pop(f);
                let l = pop(f);
                match self.original_list_index(&l, std::slice::from_ref(&idx), true) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(self, e));
                    }
                }
            }
            Op::LIST_INDEX_IMM => {
                let list = pop(f);
                if let Some(coordinate) = instr.native_list_index {
                    match tcl_cmd_core::native_list_index::immediate(self, &list, coordinate) {
                        Ok(value) => f.stack.push(value),
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self, error,
                            ));
                        }
                    }
                } else {
                    let items = match self.object_list(&list) {
                        Ok(items) => items,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_tcl_error(
                                self, error,
                            ));
                        }
                    };
                    let index = imm_index(imm0(instr), items.len());
                    f.stack.push(get_at(&items, index));
                }
            }
            Op::LIST_RANGE_IMM => {
                let l = pop(f);
                if let Some(range) = instr.native_list_range {
                    let Some(protocol) = self.object_protocol() else {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "native compiled List range issuer",
                            )
                            .into(),
                        ));
                    };
                    let discard = asm.instructions.get(f.pc).is_some_and(|next| {
                        next.op == Op::POP
                            && !next.selects_native_compiler()
                            && next.native_operation_selections.is_empty()
                            && !next.source_command_boundary.is_start()
                            && next.completion_option_scope.is_none()
                    });
                    if discard {
                        match l.native_list_range_validate(protocol) {
                            Ok(()) => {
                                f.pc += 1;
                            }
                            Err(error) => {
                                return Tick::Return(crate::command::completion_from_cmd_error(
                                    self,
                                    error.into(),
                                ));
                            }
                        }
                    } else {
                        match l.native_list_range(range, protocol) {
                            Ok(value) => f.stack.push(value),
                            Err(error) => {
                                return Tick::Return(crate::command::completion_from_cmd_error(
                                    self,
                                    error.into(),
                                ));
                            }
                        }
                    }
                    return Tick::Continue;
                }
                if imm0(instr) == 0 && imm_at(instr, 1) == tcl_bytecode::INDEX_END {
                    if let Some(protocol) = self.object_protocol().filter(|protocol| {
                        protocol
                            .tcl_version()
                            .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
                    }) {
                        match l.native_full_list_range(protocol) {
                            Ok(value) => f.stack.push(value),
                            Err(error) => {
                                return Tick::Return(crate::command::completion_from_cmd_error(
                                    self,
                                    error.into(),
                                ));
                            }
                        }
                    } else {
                        let items = try_op!(self.object_list(&l).map_err(|error| {
                            crate::command::completion_from_tcl_error(self, error)
                        }));
                        f.stack.push(slice(
                            &items,
                            0,
                            isize::try_from(items.len()).unwrap_or(isize::MAX) - 1,
                        ));
                    }
                } else {
                    let items = match self.object_list(&l) {
                        Ok(i) => i,
                        Err(e) => {
                            return Tick::Return(crate::command::completion_from_tcl_error(
                                self, e,
                            ));
                        }
                    };
                    let lo = imm_index(imm0(instr), items.len()).max(0);
                    let hi = imm_index(imm_at(instr, 1), items.len());
                    f.stack.push(slice(&items, lo, hi));
                }
            }
            Op::LIST_IN | Op::LIST_NOT_IN => {
                let list = pop(f);
                let needle = pop(f);
                let items = match self.object_list(&list) {
                    Ok(i) => i,
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                };
                let n = needle.to_str();
                let found = items.iter().any(|v| *v.to_str() == *n);
                let r = if instr.op == Op::LIST_IN {
                    found
                } else {
                    !found
                };
                f.stack.push(Value::bool(r));
            }

            // Concat (stack form): Tcl-concat the top N values.
            Op::CONCAT_STK => {
                let n = usize::try_from(imm0(instr)).unwrap_or(0);
                let take = f.stack.len().saturating_sub(n);
                let vals: Vec<Value> = f.stack.split_off(take);
                match tcl_cmd_core::list::concat_selected(self, &vals) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            Op::LIST_CONCAT => {
                let source = pop(f);
                let target = pop(f);
                let protocol = match crate::cmd_dict::VmDictionaryObjects::selected(self) {
                    Ok(objects) => objects.string_protocol(),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                };
                let result = target.native_list_concatenate(&source, protocol);
                match result {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                }
            }
            // Inline `lreplace`/`linsert` (C Tcl `INST_LREPLACE4`). Operands:
            // [argc, mode] where mode 1 = lreplace, 2 = linsert. Stack holds
            // `argc` values in push order: list, index arg(s), then elements.
            Op::LREPLACE4 | Op::LREPLACE if instr.native_list_insertion.is_some() => {
                let count = match usize::try_from(imm0(instr)) {
                    Ok(count) if count >= 2 && count <= f.stack.len() && imm_at(instr, 1) == 2 => {
                        count
                    }
                    _ => {
                        return Tick::Return(
                            self.refuse_host_command("original List insertion stack layout".into()),
                        );
                    }
                };
                let operands = f.stack.split_off(f.stack.len() - count);
                match self.execute_native_list_insertion(
                    operands,
                    instr.native_list_insertion.unwrap(),
                    instr.op,
                ) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            // C9.1 renamed INST_LREPLACE4; only the lreplace mode has no
            // separately selected insertion recipe.
            Op::LREPLACE if imm_at(instr, 1) != 1 => {
                return Tick::Return(
                    self.refuse_host_command("original List insertion recipe unavailable".into()),
                );
            }
            Op::LREPLACE4 | Op::LREPLACE => {
                let argc = usize::try_from(imm0(instr)).unwrap_or(0);
                let mode = imm_at(instr, 1);
                let take = f.stack.len().saturating_sub(argc);
                let vals: Vec<Value> = f.stack.split_off(take);
                let result = match vals.split_first() {
                    None => Vec::new(),
                    Some((list_v, _)) => {
                        let items = match self.object_list(list_v) {
                            Ok(l) => (*l).clone(),
                            Err(e) => {
                                return Tick::Return(crate::command::completion_from_tcl_error(
                                    self, e,
                                ));
                            }
                        };
                        let n = items.len();
                        let nlen = isize::try_from(n).unwrap_or(isize::MAX);
                        if mode == 1 {
                            // lreplace: list first last ?elem ...?
                            let first = match vals.get(1) {
                                Some(v) => match checked_index_value(self, v, n) {
                                    Ok(i) => i.max(0),
                                    Err(c) => return Tick::Return(c),
                                },
                                None => 0,
                            };
                            let last = match vals.get(2) {
                                Some(v) => match checked_index_value(self, v, n) {
                                    Ok(i) => i,
                                    Err(c) => return Tick::Return(c),
                                },
                                None => -1,
                            };
                            let new_elems = vals.get(3..).unwrap_or(&[]);
                            let fu = usize::try_from(first).unwrap_or(0).min(n);
                            let mut r = items[..fu].to_vec();
                            r.extend_from_slice(new_elems);
                            if last >= first {
                                let last = last.min(nlen - 1);
                                let lu = usize::try_from(last + 1).unwrap_or(n).min(n);
                                r.extend_from_slice(&items[lu..]);
                            } else {
                                r.extend_from_slice(&items[fu..]);
                            }
                            r
                        } else {
                            // linsert: list index ?elem ...? ("end" → after last).
                            let idx = match vals.get(1) {
                                Some(v) => match checked_index_value(self, v, n + 1) {
                                    Ok(i) => i,
                                    Err(c) => return Tick::Return(c),
                                },
                                None => 0,
                            };
                            let idx = usize::try_from(idx.max(0)).unwrap_or(0).min(n);
                            let new_elems = vals.get(2..).unwrap_or(&[]);
                            let mut r = items[..idx].to_vec();
                            r.extend_from_slice(new_elems);
                            r.extend_from_slice(&items[idx..]);
                            r
                        }
                    }
                };
                f.stack.push(Value::list(result));
            }

            // Arithmetic / bitwise / shift.
            Op::ADD => try_op!(bin(self, f, BinOp::Add)),
            Op::SUB => try_op!(bin(self, f, BinOp::Sub)),
            Op::MULT => try_op!(bin(self, f, BinOp::Mul)),
            Op::DIV => try_op!(bin(self, f, BinOp::Div)),
            Op::MOD => try_op!(bin(self, f, BinOp::Mod)),
            Op::EXPON => try_op!(bin(self, f, BinOp::Pow)),
            Op::LSHIFT => try_op!(bin(self, f, BinOp::LShift)),
            Op::RSHIFT => try_op!(bin(self, f, BinOp::RShift)),
            Op::BITAND => try_op!(bin(self, f, BinOp::BitAnd)),
            Op::BITOR => try_op!(bin(self, f, BinOp::BitOr)),
            Op::BITXOR => try_op!(bin(self, f, BinOp::BitXor)),
            Op::LAND => try_op!(land_lor(self, f, true)),
            Op::LOR => try_op!(land_lor(self, f, false)),

            // Comparisons.
            Op::EQ
            | Op::NEQ
            | Op::LT
            | Op::GT
            | Op::LE
            | Op::GE
            | Op::ERROR_PREFIX_EQ
            | Op::STR_EQ
            | Op::STR_NEQ
            | Op::STR_LT
            | Op::STR_GT
            | Op::STR_LE
            | Op::STR_GE
            | Op::STR_CMP => return self.tick_comparison_instruction(f, asm, instr),

            // iRules dialect operators.
            // The F5 word operators (`contains`/`starts_with`/`ends_with`/
            // `equals`/`matches`/`matches_glob`/`matches_regex`/`and`/`or`/
            // `not`), which
            // `Op::from_binop`/`from_unaryop` emit for an iRules expression.
            // They have no C Tcl counterpart; the semantics live in `expr` next
            // to the standard operators, and reuse the same glob matcher, ARE
            // engine and string comparison the equivalent commands do. Operands
            // arrive left-then-right (subject below, needle/pattern on top).
            Op::IRULE_CONTAINS => try_op!(irule(self, f, BinOp::Contains)),
            Op::IRULE_STARTS_WITH => try_op!(irule(self, f, BinOp::StartsWith)),
            Op::IRULE_ENDS_WITH => try_op!(irule(self, f, BinOp::EndsWith)),
            Op::IRULE_EQUALS => try_op!(irule(self, f, BinOp::StrEquals)),
            Op::IRULE_MATCHES_GLOB => try_op!(irule(self, f, BinOp::MatchesGlob)),
            Op::IRULE_MATCHES_REGEX => try_op!(irule(self, f, BinOp::MatchesRegex)),
            Op::IRULE_MATCHES => try_op!(irule(self, f, BinOp::Matches)),
            Op::IRULE_WORD_AND => try_op!(irule(self, f, BinOp::WordAnd)),
            Op::IRULE_WORD_OR => try_op!(irule(self, f, BinOp::WordOr)),
            Op::IRULE_WORD_NOT => {
                try_op!(un(self, f, UnaryOp::WordNot));
            }

            // String ops (inline; char-based, mirroring the reference VM)
            Op::STR_LEN if instr.native_switch_version.is_some() => {
                let original = pop(f);
                let result = try_core!(self.execute_native_scalar_length(
                    &original,
                    tcl_registry::native_scalar_compilation::NativeScalarOperation::StringLength,
                    instr.native_switch_version.unwrap(),
                ));
                drop(original);
                f.stack.push(result);
            }
            Op::STR_LEN => {
                let value = pop(f);
                match tcl_cmd_core::string::length(self, &value) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            // C `INST_STR_INDEX` (`tclExecute.c:5336-5380`): the index goes
            // through `TclGetIntForIndexM` and a *syntactically* bad spec is
            // `goto gotError` (`bad index "foo": …`) — only the separate
            // `index < 0 || index >= slength` test yields the empty string. The
            // old `resolve_index(...).and_then(...)` chain collapsed both into
            // the empty string, so a garbage index looked like a miss.
            Op::STR_INDEX
                if instr.native_switch_version.is_some()
                    || instr.native_string_index_purpose.is_some() =>
            {
                let index = pop(f);
                let subject = pop(f);
                let result = try_core!(match (
                    instr.native_switch_version,
                    instr.native_string_index_purpose,
                ) {
                    (
                        Some(version),
                        Some(tcl_bytecode::NativeStringIndexPurpose::RegisteredInstruction),
                    ) => tcl_cmd_core::string::registered_index_instruction(
                        self, &subject, &index, version
                    ),
                    (
                        Some(version),
                        Some(tcl_bytecode::NativeStringIndexPurpose::ReachedNamespaceUnicode),
                    ) => tcl_cmd_core::string::compiled_index(self, &subject, &index, version),
                    _ => Err(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native String index instruction purpose",
                    )
                    .into()),
                });
                f.stack.push(result);
            }
            Op::STR_INDEX => {
                let index = pop(f);
                let value = pop(f);
                let numbers = self.runtime_version().number_syntax();
                let result = try_core!(tcl_cmd_core::string::index(self, &value, &index, numbers));
                f.stack.push(result);
            }
            Op::STR_RANGE if instr.native_switch_version.is_some() => {
                let last = pop(f);
                let first = pop(f);
                let subject = pop(f);
                let result = try_core!(tcl_cmd_core::string::compiled_range(
                    self,
                    &subject,
                    &first,
                    &last,
                    instr.native_switch_version.unwrap()
                ));
                f.stack.push(result);
            }
            Op::STR_RANGE => {
                let last = pop(f);
                let first = pop(f);
                let value = pop(f);
                let numbers = self.runtime_version().number_syntax();
                let result = try_core!(tcl_cmd_core::string::range(
                    self, &value, &first, &last, numbers
                ));
                f.stack.push(result);
            }
            Op::STR_RANGE_IMM if instr.native_switch_version.is_some() => {
                let value = pop(f);
                let coordinates = tcl_syntax::native_compiled_index::NativeCompiledListRange {
                    first: tcl_syntax::native_compiled_index::NativeCompiledListIndex::from_encoded(
                        imm0(instr),
                    ),
                    last: tcl_syntax::native_compiled_index::NativeCompiledListIndex::from_encoded(
                        imm_at(instr, 1),
                    ),
                };
                let result = try_core!(tcl_cmd_core::string::compiled_immediate_range(
                    self,
                    &value,
                    coordinates,
                    instr.native_switch_version.unwrap()
                ));
                f.stack.push(match result {
                    tcl_cmd_core::string::NativeCompiledRangeResult::Borrowed(value)
                    | tcl_cmd_core::string::NativeCompiledRangeResult::Created(value) => value,
                });
            }
            Op::STR_RANGE_IMM => {
                let value = pop(f);
                let first = encoded_string_index(imm0(instr));
                let last = encoded_string_index(imm_at(instr, 1));
                let numbers = self.runtime_version().number_syntax();
                let result = try_core!(tcl_cmd_core::string::range(
                    self, &value, &first, &last, numbers
                ));
                f.stack.push(result);
            }
            Op::STR_FIND | Op::STR_RFIND if instr.native_switch_version.is_some() => {
                let subject = pop(f);
                let needle = pop(f);
                let version = instr.native_switch_version.unwrap();
                let index = try_core!(tcl_cmd_core::string::compiled_find(
                    self,
                    &needle,
                    &subject,
                    version,
                    instr.op == Op::STR_RFIND
                ));
                let result = Value::int(index);
                try_core!(result.set_native_unshared_integer(index, version));
                f.stack.push(result);
            }
            Op::STR_FIND => {
                let s = pop(f).to_str();
                let needle = pop(f).to_str();
                f.stack.push(Value::int(char_find(&s, &needle, false)));
            }
            Op::STR_RFIND => {
                let s = pop(f).to_str();
                let needle = pop(f).to_str();
                f.stack.push(Value::int(char_find(&s, &needle, true)));
            }
            Op::STR_MATCH if instr.native_switch_version.is_some() => {
                let subject = pop(f);
                let pattern = pop(f);
                let matched = try_core!(tcl_cmd_core::switch::compiled_glob(
                    self,
                    &pattern,
                    &subject,
                    instr.native_switch_version.unwrap(),
                    imm0(instr) != 0
                ));
                drop(subject);
                let result = try_core!(self.native_compiled_match_result(
                    pattern,
                    matched,
                    instr.native_switch_version.unwrap(),
                    tcl_registry::native_string_compilation::NativeStringMatchOperation::Glob {
                        nocase: imm0(instr) != 0
                    }
                ));
                f.stack.push(result);
            }
            Op::STR_MATCH => {
                let s = pop(f).to_str();
                let pat = pop(f).to_str();
                let nocase = imm0(instr) != 0;
                f.stack
                    .push(Value::bool(tcl_syntax::glob::string_case_match(
                        &pat, &s, nocase,
                    )));
            }
            // C `INST_STR_UPPER`/`INST_STR_LOWER`/`INST_STR_TITLE`
            // (`tclExecute.c:5284-5334`) call `Tcl_UtfToUpper`/`ToLower`/`ToTitle`,
            // which map **one code point to one code point** through Tcl's own
            // tables (`Tcl_UniCharToUpper` &co, `tclUtf.c:1777-1858`) and so never
            // change the character count. Rust's `str::to_uppercase()` implements
            // *full* Unicode mapping and expands (`ß` → `SS`, `İ` → `i` + U+0307),
            // which changed `string length` of the result; the shared
            // `simple_*` helpers restrict it to the 1:1 mapping and are the same
            // ones the `string toupper`/`tolower`/`totitle` commands use.
            Op::STR_LOWER if instr.native_switch_version.is_some() => {
                let original = pop(f);
                let scope = try_core!(NativeOperationScope::enter(self));
                let dialect = self.native_invocation_dialect();
                let version = instr.native_switch_version.unwrap();
                let bytes = try_core!(tcl_syntax::value::ValueOps::native_string_bytes(
                    self, &original
                ));
                try_core!(scope.check(self));
                let lowered = tcl_syntax::native_glob::lower_c_string_bytes(version, &bytes);
                let value = Value::new_native_string_bytes(lowered);
                if original.native_object_is_shared() {
                    let materialization = dialect.native_string_materialization(None).ok_or(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "compiled string lower issuer",
                        ),
                    );
                    let materialization = try_core!(materialization);
                    try_core!(value.retain_native_string_representation(materialization));
                    f.stack.push(value);
                } else {
                    let reached = {
                        let ops = try_core!(crate::interp::native_numeric_ops::OriginalNumericOps::in_original_scope(self, &scope));
                        let reached = original
                            .adopt_native_object_representation_with_string_mutation_checked(
                                &value,
                                dialect,
                                tcl_core_types::ResidentStringMutation::Replace,
                                &mut || {
                                    ops.current_for_value(&original)?;
                                    ops.current_for_value(&value)
                                },
                            );
                        if let Err(error) = &reached {
                            ops.retain_failure(error);
                        }
                        if let Err(first) = ops.current() {
                            drop(reached);
                            let _ = ops.current();
                            Err(first)
                        } else {
                            reached
                        }
                    };
                    try_core!(reached);
                    f.stack.push(original);
                }
            }
            Op::STR_UPPER | Op::STR_LOWER => {
                let s = pop(f).to_str();
                let map = if instr.op == Op::STR_UPPER {
                    tcl_cmd_core::string::simple_upper
                } else {
                    tcl_cmd_core::string::simple_lower
                };
                f.stack
                    .push(Value::string(s.chars().map(map).collect::<String>()));
            }
            Op::STR_TITLE => {
                let s = pop(f).to_str();
                let mut out = String::with_capacity(s.len());
                for (i, c) in s.chars().enumerate() {
                    out.push(if i == 0 {
                        tcl_cmd_core::string::simple_title(c)
                    } else {
                        tcl_cmd_core::string::simple_title_rest(c)
                    });
                }
                f.stack.push(Value::string(out));
            }
            Op::STR_REVERSE => {
                let value = pop(f);
                let result = try_core!(tcl_cmd_core::string::reverse(self, &value));
                f.stack.push(result);
            }
            // `strmap` is the one-pair, always-case-sensitive `string map` C
            // compiles a two-element literal charMap to (`INST_STR_MAP`). The
            // stack is `from to string` — the subject on top — and the mapping
            // itself is the `string map` command's (`cmd_string::map_apply`).
            Op::STR_MAP => {
                let s = pop(f).to_str();
                let to = pop(f).to_str().to_string();
                let from = pop(f).to_str().to_string();
                let limit = self.value_size_limit_value();
                let Some(out) = crate::cmd_string::map_apply(&[(from, to)], &s, false, limit)
                else {
                    return Tick::Return(self.exceed_limit_inner(crate::LimitKind::ValueSize));
                };
                let mapped = self.new_host_text(&out);
                f.stack.push(mapped);
            }
            Op::STR_REPEAT => {
                let count = pop(f);
                let value = pop(f);
                let count = try_core!(tcl_cmd_core::string::prepare_repeat_count(self, &count));
                let bytes = value.string_bytes();
                let wanted = u64::try_from(bytes.len())
                    .unwrap_or(u64::MAX)
                    .saturating_mul(u64::try_from(count.max(0)).unwrap_or(u64::MAX));
                if let Some(refusal) = self.charge_allocation(wanted) {
                    return Tick::Return(refusal);
                }
                let result =
                    try_core!(tcl_cmd_core::string::repeat_with_count(self, &value, count));
                f.stack.push(result);
            }
            Op::STR_TRIM | Op::STR_TRIM_LEFT | Op::STR_TRIM_RIGHT
                if instr.native_switch_version.is_some() =>
            {
                let characters = pop(f);
                let subject = pop(f);
                let result = try_core!(tcl_cmd_core::string::compiled_trim(
                    self,
                    &subject,
                    &characters,
                    instr.native_switch_version.unwrap(),
                    instr.op != Op::STR_TRIM_RIGHT,
                    instr.op != Op::STR_TRIM_LEFT
                ));
                f.stack.push(result);
            }
            // `INST_STR_TRIM` and its one-sided forms leave the value itself
            // where nothing is trimmed.
            Op::STR_TRIM | Op::STR_TRIM_LEFT | Op::STR_TRIM_RIGHT => {
                let chars = pop(f).to_str();
                let value = pop(f);
                let s = value.to_str();
                let set: Vec<char> = chars.chars().collect();
                let pred = |c: char| set.contains(&c);
                let out = match instr.op {
                    Op::STR_TRIM => s.trim_matches(pred),
                    Op::STR_TRIM_LEFT => s.trim_start_matches(pred),
                    _ => s.trim_end_matches(pred),
                };
                if out.len() == s.len() {
                    f.stack.push(value);
                } else {
                    f.stack.push(Value::string(out));
                }
            }

            // Numeric/boolean coercion checks (expr result validation)
            Op::TRY_CVT_TO_NUMERIC => {
                // Canonical normalisation (C Tcl `INST_TRY_CVT_TO_NUMERIC`): a
                // numeric result's string rep is regenerated from the number
                // (`expr {1e3}` → `1000.0`, not `1e3`), a non-numeric value
                // (`expr {1 ? "big" : "x"}`) passes through, and a bare `NaN` is
                // the domain error.
                let v = pop(f);
                match crate::expr::normalize_numeric_instruction_for_vm(self, &v) {
                    Ok(nv) => f
                        .stack
                        .push(nv.with_native_double_format(self.native_invocation_dialect())),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                }
            }
            // C `INST_TRY_CVT_TO_BOOLEAN` (`tclExecute.c:6404-6414`; table
            // `tclCompile.c:616` — `{"tryCvtToBoolean", 1, +1, 0, …}`, "Try
            // converting stktop to boolean if possible. **No errors.** Stack:
            // … value => … value isStrictBool"): the value is *left in place*
            // and a 0/1 "is a valid boolean" flag is pushed on top, and the
            // instruction never raises. Popping/re-pushing the value (and
            // erroring on a non-boolean) desynchronised the operand stack from
            // our own `string is boolean` codegen
            // (`tcl-codegen/src/codegen/cmd_subst.rs`), which emits C's
            // `tryCvtToBoolean; jumpTrue L; push ""; streq; jump end; L: pop;
            // push "1"` sequence and so needs both the flag and the value.
            // The flag is C's *strict* test — `TclSetBooleanFromAny` /
            // `ParseBoolean` (`tclObj.c:2100-2280`), hence the table's
            // `isStrictBool` — which accepts only `0`, `1` and the unique
            // case-insensitive prefixes of `yes`/`no`/`true`/`false`/`on`/`off`,
            // *not* `Value::as_bool`'s `Tcl_GetBooleanFromObj` leniency (any
            // non-zero number). So `string is boolean 2` is 0 while `if {2}`
            // still runs, and the opcode agrees with the `string is boolean`
            // command (`tcl_cmd_core::string_is`, same acceptor).
            Op::TRY_CVT_TO_BOOLEAN => {
                let is_bool = f.stack.last().is_some_and(|v| {
                    tcl_syntax::boolean::parse_boolean_strict(&v.to_str()).is_some()
                });
                f.stack.push(Value::bool(is_bool));
            }

            // Unary.
            Op::UMINUS => try_op!(un(self, f, UnaryOp::Neg)),
            Op::UPLUS => try_op!(un(self, f, UnaryOp::Pos)),
            Op::BITNOT => try_op!(un(self, f, UnaryOp::BitNot)),
            Op::NOT | Op::LNOT => try_op!(un(self, f, UnaryOp::Not)),

            // Control flow.
            Op::JUMP1 | Op::JUMP4 => {
                if let Some(idx) = jump_target(asm, &f.off2idx, instr) {
                    f.pc = idx;
                }
            }
            Op::JUMP_TRUE1 | Op::JUMP_TRUE4 => {
                let c = pop(f);
                match expr::boolean_for_vm(
                    self,
                    &c,
                    tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose::ConditionalJump,
                ) {
                    Ok(true) => {
                        if let Some(idx) = jump_target(asm, &f.off2idx, instr) {
                            f.pc = idx;
                        }
                    }
                    Ok(false) => {}
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                }
            }
            Op::JUMP_FALSE1 | Op::JUMP_FALSE4 => {
                let c = pop(f);
                match expr::boolean_for_vm(
                    self,
                    &c,
                    tcl_registry::native_boolean_truth::NativeBooleanTruthPurpose::ConditionalJump,
                ) {
                    Ok(false) => {
                        if let Some(idx) = jump_target(asm, &f.off2idx, instr) {
                            f.pc = idx;
                        }
                    }
                    Ok(true) => {}
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                }
            }
            Op::JUMP_TABLE if instr.native_switch_version.is_some() => {
                let original = pop(f);
                let label = if let Some(table) = &instr.native_switch_integers {
                    let integer = try_core!(tcl_syntax::value::ValueOps::as_int(self, &original));
                    table.get(&integer)
                } else if let Some(table) = &instr.native_switch_bytes {
                    let bytes = try_core!(tcl_syntax::value::ValueOps::native_string_bytes(
                        self, &original
                    ));
                    table.get(tcl_core_types::c_string_extent(&bytes))
                } else {
                    unreachable!("authenticated native switch table");
                };
                if let Some(label) = label
                    && let Some(index) = label_to_idx(asm, &f.off2idx, label)
                {
                    f.pc = index;
                }
            }
            Op::JUMP_TABLE => {
                let key = pop(f).to_str();
                if let Some(jt) = &instr.jump_table
                    && let Some(label) = jt.get(&*key)
                    && let Some(idx) = label_to_idx(asm, &f.off2idx, label)
                {
                    f.pc = idx;
                }
            }

            // Break / continue.
            Op::BREAK => {
                return Tick::Return(Completion::new(Code::Break, Value::empty(), Value::empty()));
            }
            Op::CONTINUE => {
                return Tick::Return(Completion::new(
                    Code::Continue,
                    Value::empty(),
                    Value::empty(),
                ));
            }

            // Return.
            // `SYNTAX` shares `RETURN_IMM`'s arm in C too (`tclExecute.c:2287`
            // falls through the two labels): both carry `(code, level)`
            // immediates and read `OBJ_AT_TOS` as the return-options dict,
            // `OBJ_UNDER_TOS` as the result — `CompileReturnInternal` pushes the
            // options *last*. The inline `if {…}`/`expr` codegen emits `syntax`
            // to raise a compile-time expression error at runtime.
            //
            // `TclProcessReturn` (`tclResult.c:777-785`) decides the completion:
            // `level != 0` raises TCL_RETURN carrying `-code`/`-level` for the
            // proc-boundary countdown, while `level == 0` makes the completion
            // *be* `code` — so `(0, 0)` is not a return at all, it falls through
            // to the next instruction with the result left on the stack
            // (`INST_RETURN_IMM`'s `NEXT_INST_F(9, 1, 0)` pops only the dict).
            //
            // The immediates are the authoritative merged `-code`/`-level`:
            // `TclMergeReturnOptions` consumes both keys out of the literal
            // dict, which is why they ride the operands. Appending them after
            // `-options` lets them win, exactly as C's out-params do.
            Op::RETURN_IMM | Op::SYNTAX => {
                // C's compiler guarantees both stack slots. Ours only pushes the
                // options literal outside a proc — inside one the plain return is
                // expected to fold to `done` — so an unfolded proc-body
                // `returnImm` arrives with the result alone; treat that as empty
                // options rather than eating whatever sits underneath.
                let (result, options) = if f.stack.len() >= 2 {
                    let opts = pop(f);
                    let res = pop(f);
                    (res, opts)
                } else {
                    (pop(f), Value::empty())
                };
                let application = if instr.op == Op::SYNTAX {
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Syntax
                } else {
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate
                };
                let native_context = self
                    .native_invocation_dialect()
                    .native_return_options_application(application)
                    .and_then(tcl_registry::native_return_options::NativeReturnOptionsApplicationProtocol::inner_context_name)
                    .map(|name| {
                        (
                            name,
                            result.native_lifetime_lease(),
                            options.native_lifetime_lease(),
                        )
                    });
                let c = if self
                    .native_invocation_dialect()
                    .native_return_options_application(application)
                    .is_some()
                {
                    crate::native_return_merge::process(
                        self,
                        application,
                        imm0(instr),
                        i64::from(imm_at(instr, 1)),
                        &options,
                        result,
                    )
                } else {
                    crate::command::apply_return_options(
                        self,
                        &[
                            Value::string("-options"),
                            options,
                            Value::string("-code"),
                            Value::int(i64::from(imm0(instr))),
                            Value::string("-level"),
                            Value::int(i64::from(imm_at(instr, 1))),
                            result,
                        ],
                    )
                };
                // `INST_SYNTAX` clears `ERR_ALREADY_LOGGED` after the result
                // is published, whether or not the release keeps an inner
                // context (8.5 has none), so the failing command still logs.
                if c.code == Code::Error
                    && self
                        .native_invocation_dialect()
                        .native_return_options_application(application)
                        .is_some_and(tcl_registry::native_return_options::NativeReturnOptionsApplicationProtocol::clears_logged_after_result)
                {
                    self.forget_error_logged();
                }
                if c.code == Code::Error
                    && let Some((name, result, options)) = native_context
                {
                    self.capture_original_return_instruction_context(
                        application,
                        name,
                        &[result.into_value(), options.into_value()],
                    );
                }
                if let Err(c) = self.deliver_sync(f, c) {
                    return Tick::Return(c);
                }
            }
            // `returnStk` is C-ordered — result on top, the return-options dict
            // under it — and applies the dict exactly as `return -options $opts
            // $result` would: `-code`/`-level` drive the completion (level 0 ⇒
            // the code takes effect immediately, level > 0 ⇒ `TCL_RETURN`
            // counted down at proc boundaries), and a malformed dict overrides
            // the result with its own error (C `INST_RETURN_STK` →
            // `Tcl_SetReturnOptions`). An outcome of TCL_OK (`-level 0 -code
            // ok`) pushes the result and *continues* — C only routes non-OK
            // codes to `processExceptionReturn`.
            Op::RETURN_STK => {
                let result = pop(f);
                let opts = pop(f);
                let c = if self
                    .native_invocation_dialect()
                    .native_return_options_application(
                        tcl_registry::native_return_options::NativeReturnOptionsApplication::Stack,
                    )
                    .is_some()
                {
                    crate::native_return_merge::process_stack(self, &opts, result)
                } else {
                    crate::command::apply_return_options(
                        self,
                        &[Value::string("-options"), opts, result],
                    )
                };
                if let Err(c) = self.deliver_sync(f, c) {
                    return Tick::Return(c);
                }
            }

            // Command dispatch / expr.
            Op::CALL_FUNC1 => {
                use tcl_runtime_api::native_compilation::NativeMathFunctionResolution;

                let Some(binding) = instr.native_fixed_math_call.as_ref() else {
                    return Tick::Return(self.refuse_host_command(
                        "fixed math-function compilation metadata is unavailable".into(),
                    ));
                };
                let retained =
                    asm.native_math_table_prerequisite
                        .as_ref()
                        .is_some_and(|required| {
                            matches!(required.table.lookup_bytes(binding.name.as_bytes()),
                        NativeMathFunctionResolution::Present(actual) if actual == binding)
                        });
                let [Operand::Imm(argc)] = instr.operands.as_slice() else {
                    return Tick::Return(self.refuse_host_command(
                        "fixed math-function argument metadata is unavailable".into(),
                    ));
                };
                let Some(argc) = usize::try_from(*argc).ok().filter(|argc| *argc <= 255) else {
                    return Tick::Return(self.refuse_host_command(
                        "fixed math-function argument metadata is unavailable".into(),
                    ));
                };
                if !retained || binding.arity != Some(argc) {
                    return Tick::Return(self.refuse_host_command(
                        "fixed math-function compilation metadata is inconsistent".into(),
                    ));
                }
                if f.stack.len() < argc {
                    return Tick::Return(err("fixed math-function call: stack underflow"));
                }
                let Some(selected) = f
                    .fixed_math_calls
                    .as_ref()
                    .and_then(|calls| f.pc.checked_sub(1).and_then(|index| calls.get(&index)))
                    .filter(|selected| &selected.binding == binding)
                    .cloned()
                else {
                    return Tick::Return(self.refuse_host_command(
                        "original fixed math-function handler is unavailable".into(),
                    ));
                };
                let arguments = f.stack.split_off(f.stack.len() - argc);
                let completion = self.invoke_native_fixed_math_call(&selected, &arguments);
                if let Err(completion) = self.deliver_sync(f, completion) {
                    return Tick::Return(completion);
                }
            }
            // {*} argument expansion.
            // `EXPAND_START` records the current stack depth: every word pushed
            // after it belongs to the command being built. `EXPAND_STKTOP`
            // expands the top word (a list) in place. `INVOKE_EXPANDED` recovers
            // the post-expansion argument count from the marker and invokes.
            Op::EXPAND_START => {
                f.expand_markers.push(f.stack.len());
            }
            Op::EXPAND_STKTOP => {
                let Some(list_v) = f.stack.pop() else {
                    return Tick::Return(err("expand: stack underflow"));
                };
                let elements = self
                    .object_protocol()
                    .ok_or(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "original expansion List conversion",
                    ))
                    .and_then(|protocol| self.native_object_list_elements_in(&list_v, protocol));
                match elements {
                    Ok(items) => f.stack.extend(items.iter().cloned()),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            e.into(),
                        ));
                    }
                }
            }
            // `expandDrop` abandons the innermost expansion: pop its marker and
            // truncate the stack back to the depth `EXPAND_START` recorded (C
            // `INST_EXPAND_DROP` pops `CURR_DEPTH - marker` operands).
            Op::EXPAND_DROP => {
                if let Some(marker) = f.expand_markers.pop()
                    && marker <= f.stack.len()
                {
                    f.stack.truncate(marker);
                }
            }
            // Ensemble-rewrite invoke (C Tcl `INST_INVOKE_REPLACE`): operands are
            // `(objc, opnd)` — `objc` words sit on the stack with the resolved
            // implementation word on top. The first `opnd` original words (e.g.
            // `string equal`) are replaced by the popped implementation
            // (`::tcl::string::equal`); the effective command is
            // `impl + words[opnd..]`. The same selected prefix rewrite supplies
            // native wrong-argument presentation without recompiling argv.
            // C `Tcl_ExprObj` compiles the source with `CompileExprTree`,
            // whose root ends in `INST_TRY_CVT_TO_NUMERIC`.
            Op::EXPR_STK => {
                let source = pop(f);
                return match self.prepare_original_expression_value(&source) {
                    Ok(node) => Tick::PushExpression {
                        req: Box::new(ExpressionReq {
                            state: tcl_syntax::expr::ExprEvalState::new(node),
                            awaiting_array: None,
                            normalize: true,
                            jim_objects: source.native_jim_expression_objects(),
                            restore_primary: source.retain_expression_primary(),
                        }),
                        placeholder: Box::new(self.current_placeholder_unit()),
                    },
                    Err(error) => {
                        Tick::Return(crate::command::completion_from_tcl_error(self, error))
                    }
                };
            }
            // `dict get $d k1 ?k2 …?` — operand [Imm(N)]; stack holds the dict
            // then the N keys. Leaves the looked-up value on the stack.
            Op::DICT_GET => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < nkeys + 1 {
                    return Tick::Return(err("dictGet: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let dictionary = pop(f);
                match tcl_cmd_core::dict::get(self, &dictionary, &keys) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            // `dict getdef $d k1 ?k2 …? default` — operand [Imm(N)]; stack holds
            // the dict, the N keys, then the default (C `INST_DICT_GET_DEF`). A
            // key missing at any depth yields the default; a *malformed* dict
            // still errors (C walks the path with `DICT_PATH_EXISTS`, which
            // distinguishes "absent" from "not a dictionary").
            Op::DICT_GET_DEF => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < nkeys + 2 {
                    return Tick::Return(err("dictGetDef: stack underflow"));
                }
                let default = pop(f);
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let dictionary = pop(f);
                match tcl_cmd_core::dict::getdef(self, &dictionary, &keys, &default) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            // `dict exists $d k1 ?k2 …?` — operand [Imm(N)]; stack holds the dict
            // then the N keys. Leaves a boolean on the stack.
            Op::DICT_EXISTS => {
                let nkeys = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < nkeys + 1 {
                    return Tick::Return(err("dictExists: stack underflow"));
                }
                let keys = f.stack.split_off(f.stack.len() - nkeys);
                let dictionary = pop(f);
                match tcl_cmd_core::dict::exists(self, &dictionary, &keys) {
                    Ok(value) => f.stack.push(value),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }

            // Reverse the top `N` stack elements in place (C Tcl `INST_REVERSE`;
            // operand = N). Used to reorder operands an inline emitter pushed in
            // the convenient order.
            Op::REVERSE | Op::SWAP => {
                let n = if instr.op == Op::SWAP {
                    2
                } else {
                    usize::try_from(imm0(instr)).unwrap_or(0)
                };
                let len = f.stack.len();
                if n > len {
                    return Tick::Return(err("reverse: stack underflow"));
                }
                f.stack[len - n..].reverse();
            }
            // `[lindex $list i1 i2 …]` with ≥ 2 indices — C
            // `INST_LIST_INDEX_MULTI` (`tclExecute.c:4833-4858`); operand =
            // list + index count. The list is deepest, then the indices. C
            // hands the whole run to `TclLindexFlat`, where each index is *one*
            // spec (never re-split as an index list), an out-of-range index
            // yields the empty string, and a malformed one returns `NULL` →
            // `goto gotError` (`bad index "foo": …`).
            Op::LINDEX_MULTI => {
                let opnd = usize::try_from(imm0(instr)).unwrap_or(0);
                if opnd == 0 || f.stack.len() < opnd {
                    return Tick::Return(err("lindexMulti: stack underflow"));
                }
                let items = f.stack.split_off(f.stack.len() - opnd);
                let Some((list, idxs)) = items.split_first() else {
                    return Tick::Return(err("lindexMulti: stack underflow"));
                };
                match self.original_list_index(list, idxs, false) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(self, e));
                    }
                }
            }
            // `string replace $s $first $last $new` — C Tcl `INST_STR_REPLACE`.
            // Stack holds the string, the first index, the last index, then the
            // replacement (top). Character-indexed, `end`/`end±N` aware.
            Op::STR_REPLACE => {
                let newstr = pop(f);
                let last = pop(f);
                let first = pop(f);
                let string = pop(f).to_str().to_string();
                let chars: Vec<char> = string.chars().collect();
                let len = chars.len();

                let from = match checked_index_value(self, &first, len) {
                    Ok(index) => index,
                    Err(error) => return Tick::Return(error),
                };
                let to = match checked_index_value(self, &last, len) {
                    Ok(index) => index,
                    Err(error) => return Tick::Return(error),
                };
                let slen = isize::try_from(len).unwrap_or(isize::MAX) - 1;
                if to < 0 || from > slen || to < from {
                    f.stack.push(Value::string(string));
                } else {
                    let from = usize::try_from(from.max(0)).unwrap_or(0);
                    let to = usize::try_from(to.min(slen)).unwrap_or(0);
                    if from == 0 && usize::try_from(slen).unwrap_or(0) == to {
                        f.stack.push(newstr);
                    } else {
                        let mut out: String = chars[..from].iter().collect();
                        out.push_str(&newstr.to_str());
                        out.extend(&chars[to + 1..]);
                        f.stack.push(Value::string(out));
                    }
                }
            }
            // `lset var ?index? value` (single index, or a `{i j …}` index
            // list) — C Tcl `INST_LSET_LIST`. Stack holds the index list, the
            // new value, then the list (top); leaves the rebuilt list.
            Op::LSET_LIST => {
                let list = pop(f);
                let value = pop(f);
                let index_list = pop(f);
                let release = self.runtime_version();
                match tcl_cmd_core::list::lset(
                    self,
                    &list,
                    std::slice::from_ref(&index_list),
                    value,
                    release,
                ) {
                    Ok(r) => f.stack.push(r),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(self, e));
                    }
                }
            }
            // `lset var i1 i2 ?…? value` (≥ 2 flat indices) — C Tcl
            // `INST_LSET_FLAT`; operand = index-count + 2. Stack holds the
            // indices (deepest first), the value, then the list (top).
            Op::LSET_FLAT => {
                let num_indices = usize::try_from(imm0(instr) - 2).unwrap_or(0);
                let list = pop(f);
                let value = pop(f);
                if f.stack.len() < num_indices {
                    return Tick::Return(err("lsetFlat: stack underflow"));
                }
                let path = f.stack.split_off(f.stack.len() - num_indices);
                let release = self.runtime_version();
                match tcl_cmd_core::list::lset(self, &list, &path, value, release) {
                    Ok(r) => f.stack.push(r),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(self, e));
                    }
                }
            }
            // `[regexp $pat $str]` in value position — operand [Imm(cflags)];
            // stack holds the pattern then the string (string on top, matching
            // C Tcl's `INST_REGEXP`: valuePtr = OBJ_AT_TOS, value2Ptr =
            // OBJ_UNDER_TOS). `cflags` carries the regexp compile flags; only the
            // `TCL_REG_NOCASE` bit (010 octal = 8) is meaningful here, since the
            // codegen routes `-nocase` glob-equivalents through `STR_MATCH` and
            // emits `TCL_REG_ADVANCED` (3) for the plain form. Leaves the match
            // boolean on the stack.
            Op::REGEXP if instr.native_switch_version.is_some() => {
                let subject = pop(f);
                let pattern = pop(f);
                let version = instr.native_switch_version.unwrap();
                let mut flags = tcl_cmd_core::regex::RegexFlags::for_release(version);
                flags.nocase = imm0(instr) & 8 != 0;
                let matched = try_core!(
                    tcl_cmd_core::regex::compiled_match_original::<
                        Vm,
                        crate::cmd_regexp::CrateEngine,
                    >(self, &pattern, &subject, flags, version)
                    .map_err(tcl_cmd_core::regex::RegexError::into_cmd_error)
                );
                f.stack.push(Value::int(i64::from(matched)));
            }
            Op::REGEXP => {
                const TCL_REG_NOCASE: i32 = 0o10;
                let nocase = imm0(instr) & TCL_REG_NOCASE != 0;
                let subject = pop(f);
                let pattern = pop(f);
                if self
                    .actual_native_invocation_dialect()
                    .native_jim_regex_protocol()
                    .is_some()
                {
                    match crate::cmd_regexp::invoke_jim_regexp(self, &pattern, &subject, nocase) {
                        Ok(matched) => f.stack.push(Value::bool(matched)),
                        Err(completion) => return Tick::Return(completion),
                    }
                    return Tick::Continue;
                }
                let s = subject.to_str();
                let pat = pattern.to_str();
                let version = self.runtime_version();
                match crate::cmd_regexp::regexp_matches(&pat, &s, nocase, version) {
                    Ok(m) => f.stack.push(Value::bool(m)),
                    // The command's error, `TclRegError`'s code included.
                    Err(detail) => {
                        return Tick::Return(crate::cmd_regexp::regex_completion(
                            self,
                            tcl_cmd_core::regex::compile_error(version, detail.as_bytes()),
                        ));
                    }
                }
            }
            // `[string is CLASS $str]` per-character class test — operand
            // [Imm(class-id)]; pops the string, pushes a boolean. Mirrors C Tcl's
            // `INST_STR_CLASS`: the empty string matches, otherwise every
            // character must satisfy the class comparator. Shares the classifier
            // with the `string is` command.
            Op::STR_CLASS if instr.native_switch_version.is_some() => {
                if imm0(instr) != 2 {
                    return Tick::Return(
                        self.refuse_host_command(
                            "unmodeled native string class instruction".into(),
                        ),
                    );
                }
                let original = pop(f);
                let result = try_core!(self.execute_native_ascii_predicate(
                    &original,
                    instr.native_switch_version.unwrap(),
                ));
                drop(original);
                f.stack.push(result);
            }
            Op::STR_CLASS => {
                let class_id = u8::try_from(imm0(instr)).unwrap_or(u8::MAX);
                let Some(class) = tcl_bytecode::str_class_name(class_id) else {
                    return Tick::Return(err(format!("strclass: bad class id {class_id}")));
                };
                let s = pop(f).to_str();
                let numbers = self.runtime_version().number_syntax();
                let (member, _fail) =
                    tcl_cmd_core::string_is::class_check(class, &s, false, numbers);
                f.stack.push(Value::bool(member));
            }
            // `numericType` — pops a value, pushes its numeric-tower code (C Tcl
            // `INST_NUM_TYPE` / `GetNumberFromObj`): 0 = not a number,
            // `TCL_NUMBER_INT` = 2, `TCL_NUMBER_BIG` = 3, `TCL_NUMBER_DOUBLE` = 4,
            // `TCL_NUMBER_NAN` = 5. The `string is integer/double` inline forms
            // test this against `<= 3` (integer) or `!= 0` (double/any numeric).
            Op::IS_EMPTY => {
                let empty = pop(f).native_is_empty();
                let result = self
                    .native_c_execution_boolean(empty, tcl_dialect::TclVersion::V9_1)
                    .unwrap_or_else(|_| Value::int(i64::from(empty)));
                f.stack.push(result);
            }
            Op::NUMERIC_TYPE => {
                let version = self.runtime_version();
                let v = pop(f);
                f.stack
                    .push(Value::int(native_numeric_type(&v.to_str(), version)));
            }

            Op::TAILCALL | Op::TAILCALL4 => {
                if !self.frame_owns_local_variables(self.current_level()) {
                    return Tick::Return(crate::command::err_with_code(
                        "tailcall can only be called from a proc, lambda or method",
                        "TCL TAILCALL ILLEGAL",
                    ));
                }
                let count = usize::try_from(imm0(instr)).unwrap_or(0);
                if f.stack.len() < count || count == 0 {
                    return Tick::Return(err("tailcall: stack underflow"));
                }
                let mut words = f.stack.split_off(f.stack.len() - count);
                if instr.op == Op::TAILCALL {
                    words[0] = match tcl_cmd_core::namespace::current_original(self) {
                        Ok(value) => value,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self, error,
                            ));
                        }
                    };
                }
                let Some(protocol) = self.object_protocol() else {
                    return Tick::Return(
                        self.refuse_host_command("native tailcall List issuer".into()),
                    );
                };
                let list = Value::native_list_constructor(words, protocol);
                return self.native_tailcall_tick(list);
            }
            Op::TAILCALL_LIST => {
                let list = pop(f);
                return self.native_tailcall_tick(list);
            }

            // Termination.
            Op::DONE => return self.finish_frame_result(f),

            // The compiled catch epilogue's reads (C `INST_PUSH_RESULT`/
            // `INST_PUSH_RETURN_CODE`/`INST_PUSH_RETURN_OPTS`): after a range
            // absorbed a completion they report *it*; on the fall-through (no
            // catch fired) path they report the current result / an ok options
            // dict, as C's untouched interp state would.
            Op::PUSH_RESULT => {
                let v = match Self::innermost_caught(f) {
                    Some(caught) => caught.result.clone(),
                    None => try_core!(self.frame_result_transport(f)),
                };
                f.stack.push(v.into_native_reference());
            }
            Op::PUSH_RETURN_CODE => {
                let code = Self::innermost_caught(f).map_or(0, |c| c.code.as_int());
                f.stack.push(Value::int(code));
            }
            Op::PUSH_RETURN_OPTS => {
                let result = try_core!(self.frame_result_transport(f));
                let opts = match Self::innermost_caught(f) {
                    Some(caught) => caught.options.clone(),
                    None => match self.completion_options_snapshot(&Completion::new(
                        Code::Ok,
                        result,
                        f.last_options.clone(),
                    )) {
                        Ok(options) => options,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_tcl_error(
                                self, error,
                            ));
                        }
                    },
                };
                f.stack.push(opts);
            }
            // Pop a (non-ok) return code and branch into the 5-slot `jump1`
            // table the compiler lays out right after this instruction: target
            // byte = `2*code − 1` past the opcode, error (1) first; anything
            // outside `error..continue` lands on the fifth slot (C
            // `INST_RETURN_CODE_BRANCH`, which panics on TCL_OK / non-integers
            // — the VM errors instead of aborting).
            Op::RETURN_CODE_BRANCH => {
                let original = pop(f);
                let code = match self.original_return_code_for_branch(&original) {
                    Ok(Some(code)) => code,
                    Ok(None) => {
                        return Tick::Return(err("returnCodeBranch: TOS not a return code"));
                    }
                    Err(refusal) => return Tick::Return(refusal),
                };
                if code == 0 {
                    return Tick::Return(err("returnCodeBranch: TOS is TCL_OK"));
                }
                let code = if (1..=4).contains(&code) { code } else { 5 };
                let target_off = instr.offset + 2 * i32::try_from(code).unwrap_or(5) - 1;
                match f.off2idx.get(&target_off) {
                    Some(&idx) => f.pc = idx,
                    None => {
                        return Tick::Return(err("returnCodeBranch: no jump table"));
                    }
                }
            }
            // Evaluate a popped script string and push its result — value-position
            // multi-command substitution `[a; b]`. A non-OK
            // completion unwinds like any other command error.
            Op::UPLEVEL => {
                let script = pop(f);
                let level = pop(f);
                let completion = crate::command::compiled_uplevel(self, &level, script);
                match self.settle_native_dispatch(f, completion) {
                    Ok(Some(tick)) => return tick,
                    Ok(None) => {}
                    Err(completion) => return Tick::Return(completion),
                }
            }
            Op::EVAL_STK => {
                // Run the script on the *explicit* stack (a transparent script
                // frame) rather than a nested drive, so a `yield` inside it stays
                // yieldable. Its result/`break`/`continue`/error
                // is delivered to this frame by `unwind` exactly as the old inline
                // push/`Tick::Return` did.
                let original = pop(f);
                let source = match self.native_script_source_image(&original) {
                    Ok(source) => source,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(
                            self, error,
                        ));
                    }
                };
                match self.compile_script_cached_bytes(&source) {
                    Ok(script) => {
                        return Tick::PushScript {
                            script: Box::new(script),
                            label: None,
                            cleanup_proc: None,
                            fatal_tail: None,
                            namespace: ScriptNamespace::Inherit,
                        };
                    }
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_tcl_error(self, e));
                    }
                }
            }

            // Introspection (C Tcl's "general introspector" instructions)
            // Each routes through the same core its command form uses, so the
            // compiled and dispatched paths cannot drift.
            Op::CURRENT_NAMESPACE => match tcl_cmd_core::namespace::current_original(self) {
                Ok(value) => f.stack.push(value),
                Err(error) => {
                    return Tick::Return(crate::command::completion_from_cmd_error(self, error));
                }
            },
            Op::INFO_LEVEL_NUM => match self.native_info_level(None) {
                Ok(v) => f.stack.push(v),
                Err(e) => return Tick::Return(crate::command::completion_from_cmd_error(self, e)),
            },
            Op::INFO_LEVEL_ARGS => {
                let n = pop(f);
                match self.native_info_level(Some(&n)) {
                    Ok(v) => f.stack.push(v),
                    Err(e) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(self, e));
                    }
                }
            }
            // Both instructions reach the same original-object getter and name
            // reporting owner as their namespace command forms.
            Op::RESOLVE_CMD => {
                let original = pop(f);
                match self.native_namespace_command_name(&original, false) {
                    Ok(bytes) => f
                        .stack
                        .push(bytes.map_or_else(Value::empty, Value::from_native_string_bytes)),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            Op::ORIGIN_CMD => {
                let original = pop(f);
                match self.native_namespace_command_name(&original, true) {
                    Ok(Some(bytes)) => match self.native_namespace_origin_result(&bytes) {
                        Ok(result) => f.stack.push(result),
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self,
                                error.into(),
                            ));
                        }
                    },
                    Ok(None) => {
                        return Tick::Return(self.native_namespace_origin_failure(&original));
                    }
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self, error,
                        ));
                    }
                }
            }
            // `clockRead <which>` reads the same host clock `clock clicks` /
            // `clock seconds` do (`cmd_clock`), so the opcode and the command
            // agree. C: 0 = clicks (wide clicks — microseconds here, as
            // `clock clicks` returns), 1 = µs, 2 = ms, 3 = s.
            Op::CLOCK_READ => {
                let clock = self.host_rc();
                let clock = clock.clock();
                let wide = match imm0(instr) {
                    2 => i64::try_from(clock.now_millis()).unwrap_or(i64::MAX),
                    3 => clock.now_secs(),
                    // 0 (clicks) and 1 (microseconds) read the same counter.
                    _ => i64::try_from(clock.now_micros()).unwrap_or(i64::MAX),
                };
                f.stack.push(Value::int(wide));
            }

            // -- coroutines (C `INST_YIELD` / `INST_YIELD_TO_INVOKE` /
            // `INST_COROUTINE_NAME`) --
            // The compiled `yield`/`yieldto` record their suspend request through
            // the very core the builtins use (`cmd_coro::request_*`, boundary
            // checks included) and then drain it into the `Tick::Suspend` the
            // builtin path gets from `dispatch_words`. C pops the yielded value
            // and pushes the resume value in its place (`pc++; cleanup = 1;
            // TEBC_YIELD()`), which is exactly what popping here plus `resume`'s
            // delivered-value push does.
            Op::YIELD => {
                let value = pop(f);
                if let Err(c) = crate::cmd_coro::request_yield(self, value) {
                    return Tick::Return(c);
                }
                if let Some(req) = self.coro.pending.take() {
                    return Tick::Suspend(req);
                }
            }
            // TOS is the captured namespace followed by relay command argv.
            // The namespace was evaluated before the original operands.
            Op::YIELD_TO_INVOKE => {
                let original = pop(f);
                if let Err(c) = crate::cmd_coro::request_yieldto_original(self, original) {
                    return Tick::Return(c);
                }
                if let Some(req) = self.coro.pending.take() {
                    return Tick::Suspend(req);
                }
            }
            Op::CORO_NAME => {
                f.stack.push(crate::cmd_coro::current_coroutine(self));
            }

            // TclOO (C's "start of TclOO support instructions" block)
            // Each routes through the core its command form uses: `self object`,
            // `info object class`/`namespace`/`isa object`, `next`, `nextto`. The
            // context checks are the opcodes' own, since C's compiled forms report
            // "X may only be called from inside a method" where the command forms
            // report an unresolvable command name.
            Op::TCLOO_SELF => match crate::cmd_oo::current_object_name(self) {
                Some(name) => f.stack.push(name),
                None => {
                    return Tick::Return(crate::cmd_oo::native_context::helper_context_error(
                        self, b"self",
                    ));
                }
            },
            Op::TCLOO_IS_OBJECT => {
                let name = pop(f);
                match crate::cmd_oo::is_object(self, &name) {
                    Ok(present) => match self.native_oo_predicate_result(present) {
                        Ok(value) => f.stack.push(value),
                        Err(error) => {
                            return Tick::Return(self.refuse_host_command(error.to_string()));
                        }
                    },
                    Err(completion) => return Tick::Return(completion),
                }
            }
            Op::TCLOO_CLASS => {
                let name = pop(f);
                match crate::cmd_oo::object_key(self, &name) {
                    Ok(key) => {
                        let v = crate::cmd_oo::object_class_name(self, key);
                        f.stack.push(v);
                    }
                    Err(c) => return Tick::Return(c),
                }
            }
            Op::TCLOO_NS => {
                let name = pop(f);
                match crate::cmd_oo::object_key(self, &name) {
                    Ok(key) => match crate::cmd_oo::object_namespace_name(self, key) {
                        Ok(value) => f.stack.push(value),
                        Err(completion) => return Tick::Return(completion),
                    },
                    Err(c) => return Tick::Return(c),
                }
            }
            Op::TCLOO_ID => {
                let dialect = self.actual_native_invocation_dialect();
                if dialect.family() != Some(tcl_dialect::model::Family::Tcl)
                    || dialect.tcl_version != Some(tcl_dialect::TclVersion::V9_1)
                {
                    return Tick::Return(
                        self.refuse_host_command("native TclOO creation-id instruction".to_owned()),
                    );
                }
                let name = pop(f);
                match crate::cmd_oo::object_key(self, &name)
                    .and_then(|key| crate::cmd_oo::object_creation_id(self, key))
                {
                    Ok(value) => f.stack.push(value),
                    Err(completion) => return Tick::Return(completion),
                }
            }
            Op::INVOKE_STK1
            | Op::INVOKE_STK4
            | Op::INVOKE_EXPANDED
            | Op::INVOKE_REPLACE
            | Op::TCLOO_NEXT
            | Op::TCLOO_NEXT4
            | Op::TCLOO_NEXT_CLASS
            | Op::TCLOO_NEXT_CLASS4
            | Op::TCLOO_NEXT_LIST
            | Op::TCLOO_NEXT_CLASS_LIST => return self.tick_invoke(f, asm, instr),
            // Routed before this match by `tick_in_original_scope`; kept so the
            // match stays exhaustive.
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
            | Op::DICT_LAPPEND => return self.tick_variable_instruction(f, asm, instr),
        }

        Tick::Continue
    }
}
