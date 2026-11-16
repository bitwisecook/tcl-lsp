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

//! The comparison instructions: numeric and string comparisons and the error-prefix test.

use super::super::{Frame, Tick};
use super::operands::{cmp, imm0, pop};
use crate::interp::Vm;
use crate::value::Value;
use std::rc::Rc;
use tcl_bytecode::{FunctionAsm, Instruction, Op};
use tcl_syntax::expr::BinOp;

impl Vm {
    /// The comparison arms of [`Self::tick_instruction`]: numeric and string comparisons, string compare and the error-prefix test.
    #[inline(never)]
    pub(super) fn tick_comparison_instruction(
        &mut self,
        f: &mut Frame,
        _asm: &Rc<FunctionAsm>,
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
            Op::EQ => try_op!(cmp(self, f, BinOp::Eq)),
            Op::NEQ => try_op!(cmp(self, f, BinOp::Ne)),
            Op::LT => try_op!(cmp(self, f, BinOp::Lt)),
            Op::GT => try_op!(cmp(self, f, BinOp::Gt)),
            Op::LE => try_op!(cmp(self, f, BinOp::Le)),
            Op::GE => try_op!(cmp(self, f, BinOp::Ge)),
            Op::ERROR_PREFIX_EQ => return self.tick_error_prefix_comparison(f, instr),
            Op::STR_EQ if instr.native_switch_version.is_some() => {
                let right = pop(f);
                let left = pop(f);
                let matched = try_core!(tcl_cmd_core::switch::compiled_equal(
                    self,
                    &left,
                    &right,
                    instr.native_switch_version.unwrap()
                ));
                drop(right);
                let result = try_core!(self.native_compiled_match_result(
                    left,
                    matched,
                    instr.native_switch_version.unwrap(),
                    tcl_registry::native_string_compilation::NativeStringMatchOperation::Equal
                ));
                f.stack.push(result);
            }
            Op::STR_EQ => try_op!(cmp(self, f, BinOp::StrEq)),
            Op::STR_NEQ => try_op!(cmp(self, f, BinOp::StrNe)),
            Op::STR_LT => try_op!(cmp(self, f, BinOp::StrLt)),
            Op::STR_GT => try_op!(cmp(self, f, BinOp::StrGt)),
            Op::STR_LE => try_op!(cmp(self, f, BinOp::StrLe)),
            Op::STR_GE => try_op!(cmp(self, f, BinOp::StrGe)),
            Op::STR_CMP => {
                let b = pop(f);
                let a = pop(f);
                let ord = (*a.to_str()).cmp(&b.to_str());
                f.stack.push(Value::int(match ord {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }));
            }
            _ => unreachable!("tick_comparison_instruction handles only comparison opcodes"),
        }

        Tick::Continue
    }

    /// The `ERROR_PREFIX_EQ` arm of [`Self::tick_comparison_instruction`]: the native error-prefix list comparison.
    #[inline(never)]
    fn tick_error_prefix_comparison(&mut self, f: &mut Frame, instr: &Instruction) -> Tick {
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
            Op::ERROR_PREFIX_EQ => {
                let prefix = pop(f);
                let original = pop(f);
                let Some(protocol) = self.object_protocol().filter(|protocol| {
                    protocol
                        .tcl_version()
                        .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_1)
                }) else {
                    return Tick::Return(self.refuse_host_command(
                        "native error-prefix comparison recipe is unavailable".into(),
                    ));
                };
                let left = try_core!(self.native_object_list_elements_in(&original, protocol));
                let right = try_core!(self.native_object_list_elements_in(&prefix, protocol));
                let length = usize::try_from(imm0(instr)).expect("native error-prefix length");
                let mut matched = true;
                for index in 0..length {
                    matched = match (left.get(index), right.get(index)) {
                        (Some(left), Some(right)) => {
                            try_core!(tcl_cmd_core::switch::compiled_equal(
                                self,
                                left,
                                right,
                                tcl_dialect::TclVersion::V9_1
                            ))
                        }
                        (Some(value), None) | (None, Some(value)) => {
                            use tcl_syntax::native_object::{
                                NativeObjectStringEmptiness, native_c_string_emptiness,
                            };
                            let snapshot = try_core!(
                                tcl_syntax::value::ValueOps::native_object_snapshot(self, value)
                            );
                            match native_c_string_emptiness(
                                tcl_dialect::TclVersion::V9_1,
                                &snapshot,
                            ) {
                                Ok(NativeObjectStringEmptiness::Empty) => true,
                                Ok(NativeObjectStringEmptiness::Nonempty) => false,
                                Ok(NativeObjectStringEmptiness::Unknown) => {
                                    try_core!(value.native_string_bytes(protocol).map_err(
                                    tcl_syntax::raw_string::NativeStringAccessError::Unavailable
                                ))
                                    .is_empty()
                                }
                                Err(_) => {
                                    return Tick::Return(
                                        self.refuse_host_command(
                                            "native error-prefix emptiness recipe is unavailable"
                                                .into(),
                                        ),
                                    );
                                }
                            }
                        }
                        (None, None) => true,
                    };
                    if !matched {
                        break;
                    }
                }
                f.stack.push(Value::int(i64::from(matched)));
            }
            _ => unreachable!("tick_error_prefix_comparison handles only its own opcode"),
        }

        Tick::Continue
    }
}
