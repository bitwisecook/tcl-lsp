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

//! Integer arithmetic with Tcl rounding and overflow checks, and the dynamic binary, compare and unary operations.

use super::super::encoding::leb128_unsigned;
use super::super::ir::{WasmInstruction, WasmOp};
use super::{
    Emitter, FRAME_COMPLETION, FRAME_SCRATCH_I32, FRAME_SCRATCH_I64, LOCAL_FRAME,
    LOCAL_SCRATCH_F64_A, LOCAL_SCRATCH_F64_B, LOCAL_SCRATCH_I32_B, LOCAL_SCRATCH_I64_A,
    LOCAL_SCRATCH_I64_B, LOCAL_SCRATCH_I64_C, LOCAL_SCRATCH_OBJ_A, Label,
};
use tcl_compiler::native_lowering::ir::{CmpOp, CompareKind, IntOp, NativeType, NativeValueId};
use tcl_compiler::native_lowering::representation::int_op;
use tcl_runtime_api::codegen_abi::{
    WASM32_COMPLETION_CODE_OFFSET, WASM32_COMPLETION_OPTIONS_OFFSET,
    WASM32_COMPLETION_RESULT_OFFSET,
};
use tcl_syntax::expr::{BinOp, UnaryOp};

impl Emitter<'_, '_> {
    // Integer arithmetic.

    /// Native integer arithmetic on locals `a`, `b` into `d`, Tcl rounding.
    pub(super) fn int_binary(&mut self, op: IntOp, a: u64, b: u64, d: u64) {
        match op {
            IntOp::Add
            | IntOp::Sub
            | IntOp::Mul
            | IntOp::And
            | IntOp::Or
            | IntOp::Xor
            | IntOp::Shl
            | IntOp::Shr => {
                self.get(a);
                self.get(b);
                self.push(match op {
                    IntOp::Add => WasmOp::I64Add,
                    IntOp::Sub => WasmOp::I64Sub,
                    IntOp::Mul => WasmOp::I64Mul,
                    IntOp::And => WasmOp::I64And,
                    IntOp::Or => WasmOp::I64Or,
                    IntOp::Xor => WasmOp::I64Xor,
                    IntOp::Shl => WasmOp::I64Shl,
                    _ => WasmOp::I64ShrS,
                });
                self.set(d);
            }
            IntOp::Div => {
                // q = a / b; if (a % b != 0) && ((a % b < 0) != (b < 0)) { q -= 1 }
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64RemS);
                self.set(LOCAL_SCRATCH_I64_C);
                self.floor_adjust_condition(b);
                self.set(LOCAL_SCRATCH_I32_B);
                // The checked path uses scratch c as d: preserve the
                // adjustment before replacing the remainder with the quotient.
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64DivS);
                self.set(d);
                self.get(LOCAL_SCRATCH_I32_B);
                self.open(WasmOp::If, Label::Plain);
                self.get(d);
                self.i64(1);
                self.push(WasmOp::I64Sub);
                self.set(d);
                self.close();
            }
            IntOp::Mod => {
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64RemS);
                self.tee(LOCAL_SCRATCH_I64_C);
                self.set(d);
                self.floor_adjust_condition(b);
                self.open(WasmOp::If, Label::Plain);
                self.get(d);
                self.get(b);
                self.push(WasmOp::I64Add);
                self.set(d);
                self.close();
            }
        }
    }

    /// Leaves `(r != 0) && ((r < 0) != (b < 0))` on the stack, with the
    /// remainder in scratch `c`.
    fn floor_adjust_condition(&mut self, b: u64) {
        self.get(LOCAL_SCRATCH_I64_C);
        self.i64(0);
        self.push(WasmOp::I64Ne);
        self.get(LOCAL_SCRATCH_I64_C);
        self.i64(0);
        self.push(WasmOp::I64LtS);
        self.get(b);
        self.i64(0);
        self.push(WasmOp::I64LtS);
        self.push(WasmOp::I32Xor);
        self.push(WasmOp::I32And);
    }

    /// Emit the checked native integer operation on scratch `a`/`b` into
    /// scratch `c`, branching to `slow` when the result leaves `i64` or a
    /// Tcl precondition fails.
    #[allow(clippy::too_many_lines)]
    fn checked_int_op(&mut self, op: IntOp, slow: Label) {
        let (a, b, c) = (
            LOCAL_SCRATCH_I64_A,
            LOCAL_SCRATCH_I64_B,
            LOCAL_SCRATCH_I64_C,
        );
        match op {
            IntOp::Add => {
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64Add);
                self.set(c);
                // overflow iff ((a ^ r) & (b ^ r)) < 0
                self.get(a);
                self.get(c);
                self.push(WasmOp::I64Xor);
                self.get(b);
                self.get(c);
                self.push(WasmOp::I64Xor);
                self.push(WasmOp::I64And);
                self.i64(0);
                self.push(WasmOp::I64LtS);
                self.br_if(slow);
            }
            IntOp::Sub => {
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64Sub);
                self.set(c);
                // overflow iff ((a ^ b) & (a ^ r)) < 0
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64Xor);
                self.get(a);
                self.get(c);
                self.push(WasmOp::I64Xor);
                self.push(WasmOp::I64And);
                self.i64(0);
                self.push(WasmOp::I64LtS);
                self.br_if(slow);
            }
            IntOp::Mul => {
                // The two products that overflow through division itself.
                self.get(a);
                self.i64(-1);
                self.push(WasmOp::I64Eq);
                self.get(b);
                self.i64(i64::MIN);
                self.push(WasmOp::I64Eq);
                self.push(WasmOp::I32And);
                self.br_if(slow);
                self.get(b);
                self.i64(-1);
                self.push(WasmOp::I64Eq);
                self.get(a);
                self.i64(i64::MIN);
                self.push(WasmOp::I64Eq);
                self.push(WasmOp::I32And);
                self.br_if(slow);
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64Mul);
                self.set(c);
                // overflow iff a != 0 && r / a != b
                self.get(a);
                self.i64(0);
                self.push(WasmOp::I64Ne);
                self.open(WasmOp::If, Label::Plain);
                self.get(c);
                self.get(a);
                self.push(WasmOp::I64DivS);
                self.get(b);
                self.push(WasmOp::I64Ne);
                self.open(WasmOp::If, Label::Plain);
                self.br(slow);
                self.close();
                self.close();
            }
            IntOp::Div | IntOp::Mod => {
                self.get(b);
                self.push(WasmOp::I64Eqz);
                self.br_if(slow);
                if op == IntOp::Div {
                    self.get(a);
                    self.i64(i64::MIN);
                    self.push(WasmOp::I64Eq);
                    self.get(b);
                    self.i64(-1);
                    self.push(WasmOp::I64Eq);
                    self.push(WasmOp::I32And);
                    self.br_if(slow);
                } else {
                    // rem_s by -1 is defined (zero); nothing to check.
                }
                self.int_binary(op, a, b, c);
            }
            IntOp::And | IntOp::Or | IntOp::Xor => {
                self.int_binary(op, a, b, c);
            }
            IntOp::Shl => {
                self.get(b);
                self.i64(0);
                self.push(WasmOp::I64LtS);
                self.br_if(slow);
                self.get(b);
                self.i64(62);
                self.push(WasmOp::I64GtS);
                self.br_if(slow);
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64Shl);
                self.set(c);
                self.get(c);
                self.get(b);
                self.push(WasmOp::I64ShrS);
                self.get(a);
                self.push(WasmOp::I64Ne);
                self.br_if(slow);
            }
            IntOp::Shr => {
                self.get(b);
                self.i64(0);
                self.push(WasmOp::I64LtS);
                self.br_if(slow);
                self.get(b);
                self.i64(63);
                self.push(WasmOp::I64GtS);
                self.open(WasmOp::If, Label::Plain);
                self.get(a);
                self.i64(63);
                self.push(WasmOp::I64ShrS);
                self.set(c);
                self.else_();
                self.get(a);
                self.get(b);
                self.push(WasmOp::I64ShrS);
                self.set(c);
                self.close();
            }
        }
    }

    /// `dst = lhs op rhs` over operands of any representation: integer fast
    /// path, double fast path, then the runtime operator.
    pub(super) fn dynamic_binary(
        &mut self,
        dst: NativeValueId,
        op: BinOp,
        lhs: NativeValueId,
        rhs: NativeValueId,
        completion: tcl_compiler::executable_ir::CompletionId,
    ) {
        let done = Label::Plain;
        self.open(WasmOp::Block, done);
        let done_depth = self.labels.len() - 1;
        self.open(WasmOp::Block, Label::Plain); // slow
        let slow_depth = self.labels.len() - 1;
        if let Some(iop) = int_op(op) {
            self.open(WasmOp::Block, Label::Plain); // not int
            let notint_depth = self.labels.len() - 1;
            self.load_as_int_at(lhs, LOCAL_SCRATCH_I64_A, notint_depth);
            self.load_as_int_at(rhs, LOCAL_SCRATCH_I64_B, notint_depth);
            self.checked_int_op_at(iop, slow_depth);
            self.get(LOCAL_SCRATCH_I64_C);
            self.call(self.imports.value_new_wide_int);
            self.set_value_owned(dst);
            self.br_depth(done_depth);
            self.close();
            if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div) {
                self.load_as_double_at(lhs, LOCAL_SCRATCH_F64_A, slow_depth);
                self.load_as_double_at(rhs, LOCAL_SCRATCH_F64_B, slow_depth);
                self.get(LOCAL_SCRATCH_F64_A);
                self.get(LOCAL_SCRATCH_F64_B);
                self.push(match op {
                    BinOp::Add => WasmOp::F64Add,
                    BinOp::Sub => WasmOp::F64Sub,
                    BinOp::Mul => WasmOp::F64Mul,
                    _ => WasmOp::F64Div,
                });
                self.call(self.imports.value_new_double);
                self.set_value_owned(dst);
                self.br_depth(done_depth);
            }
        }
        self.close(); // slow
        self.text_pair(op.spec().spelling);
        self.argv(&[lhs, rhs]);
        self.i32(2);
        self.frame_offset(FRAME_COMPLETION);
        self.call(self.imports.mathop);
        self.push(WasmOp::Drop);
        self.take_frame_result(dst, completion);
        self.close(); // done
    }

    pub(super) fn dynamic_compare(
        &mut self,
        dst: NativeValueId,
        op: BinOp,
        lhs: NativeValueId,
        rhs: NativeValueId,
        completion: tcl_compiler::executable_ir::CompletionId,
    ) {
        self.open(WasmOp::Block, Label::Plain); // done
        let done_depth = self.labels.len() - 1;
        self.open(WasmOp::Block, Label::Plain); // slow
        let slow_depth = self.labels.len() - 1;
        if let Some(cmp) = tcl_compiler::native_lowering::representation::cmp_op(op) {
            self.open(WasmOp::Block, Label::Plain); // not int
            let notint_depth = self.labels.len() - 1;
            self.load_as_int_at(lhs, LOCAL_SCRATCH_I64_A, notint_depth);
            self.load_as_int_at(rhs, LOCAL_SCRATCH_I64_B, notint_depth);
            self.get(LOCAL_SCRATCH_I64_A);
            self.get(LOCAL_SCRATCH_I64_B);
            self.push(compare_op(cmp, CompareKind::I64));
            self.set(self.local_of(dst));
            self.br_depth(done_depth);
            self.close();
            self.load_as_double_at(lhs, LOCAL_SCRATCH_F64_A, slow_depth);
            self.load_as_double_at(rhs, LOCAL_SCRATCH_F64_B, slow_depth);
            self.get(LOCAL_SCRATCH_F64_A);
            self.get(LOCAL_SCRATCH_F64_B);
            self.push(compare_op(cmp, CompareKind::F64));
            self.set(self.local_of(dst));
            self.br_depth(done_depth);
        }
        self.close(); // slow
        self.text_pair(op.spec().spelling);
        self.argv(&[lhs, rhs]);
        self.i32(2);
        self.frame_offset(FRAME_COMPLETION);
        self.call(self.imports.mathop);
        self.push(WasmOp::Drop);
        // The runtime comparison returns canonical boxed 0/1. This
        // primitive read consumes that result contract; it is not a reached
        // source-operand truth or expression-result-normalisation stage.
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.open(WasmOp::If, Label::Plain);
        self.fail_with_frame_completion(completion);
        self.close();
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.set_owned(LOCAL_SCRATCH_OBJ_A);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.call(self.imports.obj_release);
        self.get(LOCAL_SCRATCH_OBJ_A);
        self.frame_offset(FRAME_SCRATCH_I32);
        self.call(self.imports.value_get_bool);
        self.open(WasmOp::If, Label::Plain);
        self.fail(completion);
        self.close();
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_SCRATCH_I32);
        self.set(self.local_of(dst));
        self.release_scratch();
        self.close(); // done
    }

    pub(super) fn dynamic_unary(
        &mut self,
        dst: NativeValueId,
        op: UnaryOp,
        src: NativeValueId,
        completion: tcl_compiler::executable_ir::CompletionId,
    ) {
        self.open(WasmOp::Block, Label::Plain); // done
        let done_depth = self.labels.len() - 1;
        self.open(WasmOp::Block, Label::Plain); // slow
        let slow_depth = self.labels.len() - 1;
        self.open(WasmOp::Block, Label::Plain); // not int
        let notint_depth = self.labels.len() - 1;
        self.load_as_int_at(src, LOCAL_SCRATCH_I64_A, notint_depth);
        match op {
            UnaryOp::Neg => {
                self.get(LOCAL_SCRATCH_I64_A);
                self.i64(i64::MIN);
                self.push(WasmOp::I64Eq);
                self.br_if_depth(slow_depth);
                self.i64(0);
                self.get(LOCAL_SCRATCH_I64_A);
                self.push(WasmOp::I64Sub);
            }
            UnaryOp::Pos => self.get(LOCAL_SCRATCH_I64_A),
            UnaryOp::BitNot => {
                self.get(LOCAL_SCRATCH_I64_A);
                self.i64(-1);
                self.push(WasmOp::I64Xor);
            }
            UnaryOp::Not | UnaryOp::WordNot => self.br_depth(slow_depth),
        }
        if !matches!(op, UnaryOp::Not | UnaryOp::WordNot) {
            self.call(self.imports.value_new_wide_int);
            self.set_value_owned(dst);
            self.br_depth(done_depth);
        }
        self.close(); // not int
        if matches!(op, UnaryOp::Neg | UnaryOp::Pos) {
            self.load_as_double_at(src, LOCAL_SCRATCH_F64_A, slow_depth);
            self.get(LOCAL_SCRATCH_F64_A);
            if op == UnaryOp::Neg {
                self.push(WasmOp::F64Neg);
            }
            self.call(self.imports.value_new_double);
            self.set_value_owned(dst);
            self.br_depth(done_depth);
        }
        self.close(); // slow
        self.text_pair(op.as_str());
        self.argv(&[src]);
        self.i32(1);
        self.frame_offset(FRAME_COMPLETION);
        self.call(self.imports.mathop);
        self.push(WasmOp::Drop);
        self.take_frame_result(dst, completion);
        self.close(); // done
    }

    // Depth-addressed branches for the anonymous blocks of the dynamic ops.

    fn br_depth(&mut self, label_index: usize) {
        let depth = u64::try_from(self.labels.len() - 1 - label_index).unwrap_or(u64::MAX);
        self.body.push(WasmInstruction::with_operands(
            WasmOp::Br,
            leb128_unsigned(depth),
        ));
    }

    fn br_if_depth(&mut self, label_index: usize) {
        let depth = u64::try_from(self.labels.len() - 1 - label_index).unwrap_or(u64::MAX);
        self.body.push(WasmInstruction::with_operands(
            WasmOp::BrIf,
            leb128_unsigned(depth),
        ));
    }

    fn load_as_int_at(&mut self, value: NativeValueId, target: u64, slow_index: usize) {
        match self.ty(value) {
            NativeType::F64 => self.br_depth(slow_index),
            NativeType::Obj => {
                self.get(self.local_of(value));
                self.frame_offset(FRAME_SCRATCH_I64);
                self.call(self.imports.value_try_wide_int);
                self.push(WasmOp::I32Eqz);
                self.br_if_depth(slow_index);
                self.get(LOCAL_FRAME);
                self.load_i64(FRAME_SCRATCH_I64);
                self.set(target);
            }
            NativeType::I64 => {
                self.get(self.local_of(value));
                self.set(target);
            }
            NativeType::Bool => {
                self.get(self.local_of(value));
                self.push(WasmOp::I64ExtendI32U);
                self.set(target);
            }
        }
    }

    fn load_as_double_at(&mut self, value: NativeValueId, target: u64, slow_index: usize) {
        match self.ty(value) {
            NativeType::Obj => {
                self.get(self.local_of(value));
                self.frame_offset(FRAME_SCRATCH_I64);
                self.call(self.imports.value_try_double);
                self.push(WasmOp::I32Eqz);
                self.br_if_depth(slow_index);
                self.get(LOCAL_FRAME);
                self.load_f64(FRAME_SCRATCH_I64);
                self.set(target);
            }
            NativeType::I64 | NativeType::Bool | NativeType::F64 => {
                self.get(self.local_of(value));
                if self.ty(value) == NativeType::Bool {
                    self.push(WasmOp::I64ExtendI32U);
                }
                if self.ty(value) != NativeType::F64 {
                    self.push(WasmOp::F64ConvertI64S);
                }
                self.set(target);
            }
        }
        self.get(target);
        self.get(target);
        self.push(WasmOp::F64Ne);
        self.br_if_depth(slow_index);
    }

    fn checked_int_op_at(&mut self, op: IntOp, slow_index: usize) {
        // `checked_int_op` branches through a `Label`; the dynamic ops
        // address their slow block by index, so give it a unique label.
        let marker = Label::Block(usize::MAX - slow_index);
        self.labels[slow_index] = marker;
        self.checked_int_op(op, marker);
        self.labels[slow_index] = Label::Plain;
    }
}

pub(super) const fn compare_op(op: CmpOp, kind: CompareKind) -> WasmOp {
    match (kind, op) {
        (CompareKind::I64, CmpOp::Eq) => WasmOp::I64Eq,
        (CompareKind::I64, CmpOp::Ne) => WasmOp::I64Ne,
        (CompareKind::I64, CmpOp::Lt) => WasmOp::I64LtS,
        (CompareKind::I64, CmpOp::Le) => WasmOp::I64LeS,
        (CompareKind::I64, CmpOp::Gt) => WasmOp::I64GtS,
        (CompareKind::I64, CmpOp::Ge) => WasmOp::I64GeS,
        (CompareKind::F64, CmpOp::Eq) => WasmOp::F64Eq,
        (CompareKind::F64, CmpOp::Ne) => WasmOp::F64Ne,
        (CompareKind::F64, CmpOp::Lt) => WasmOp::F64Lt,
        (CompareKind::F64, CmpOp::Le) => WasmOp::F64Le,
        (CompareKind::F64, CmpOp::Gt) => WasmOp::F64Gt,
        (CompareKind::F64, CmpOp::Ge) => WasmOp::F64Ge,
    }
}
