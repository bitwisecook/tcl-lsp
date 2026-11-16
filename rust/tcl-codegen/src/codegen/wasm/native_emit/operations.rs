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

//! Statements and their operations: result materialisation, completions, error sites and `emit_op`.

use super::super::encoding::leb128_unsigned;
use super::super::ir::{WasmInstruction, WasmOp};
use super::arithmetic::compare_op;
use super::function_plan::{ResultSource, can_fail, result_source};
use super::{
    Emitter, FRAME_COMPLETION, FRAME_SCRATCH_I32, FRAME_SCRATCH_I64, LOCAL_FRAME,
    LOCAL_SCRATCH_I32, LOCAL_SCRATCH_OBJ_A, Label, PARAM_OUT, PLAIN_RETURN_LEVEL, TCL_ERROR,
};
use tcl_compiler::native_lowering::cells::CellPlace;
use tcl_compiler::native_lowering::ir::{
    DoubleOp, IfElseResult, NativeOp, NativeStatement, NativeType, NativeValueId,
};
use tcl_core_types::Code as CompletionCode;
use tcl_runtime_api::codegen_abi::{
    WASM32_COMPLETION_CODE_OFFSET, WASM32_COMPLETION_OPTIONS_OFFSET,
    WASM32_COMPLETION_RESULT_OFFSET,
};

impl Emitter<'_, '_> {
    // Statements.

    pub(super) fn emit_statement(&mut self, statement: &NativeStatement) {
        let code = self.code_local(statement.completion);
        // A proc entry hands the completions a `Return` reads to the runtime,
        // so those — and only those — have to carry the statement's Tcl
        // result. Every other completion is read by a completion switch,
        // which looks at the code alone.
        let escapes = self.proc_entry() && self.returned.contains(&statement.completion);
        let materialise = escapes.then(|| result_source(statement)).flatten();
        self.i32(0);
        self.set(code);
        if escapes {
            // An edge that abandons the statement must report the runtime's
            // own result — the error message the failing operation set — not
            // the one a previous execution of this same statement left here.
            self.i32(0);
            self.set_owned(code + 1);
        }
        self.open(WasmOp::Block, Label::Abort);
        for op in &statement.ops {
            self.emit_op(op, statement.completion);
        }
        // Reached only when every operation ran: an abandoned statement
        // branched past this to the end of the block.
        if let Some(source) = materialise {
            self.materialise_result(statement.completion, source);
        }
        self.close();
        self.log_error_site(statement);
    }

    /// Put the Tcl result of the statement's command in the completion, for
    /// the operations whose runtime call does not leave it anywhere the
    /// proc-entry epilogue can find it.
    ///
    /// `set foo bar` answers with `bar` and `incr n` with the new value, but
    /// `tcl_codegen_var_set`/`tcl_codegen_var_incr` set no interpreter result:
    /// without this a body ending in one would answer with whatever the
    /// runtime's result happened to be. Only a proc entry needs it — a script
    /// entry's completion result is never read.
    pub(super) fn materialise_result(
        &mut self,
        completion: tcl_compiler::executable_ir::CompletionId,
        source: ResultSource,
    ) {
        let code = self.code_local(completion);
        match source {
            // Already where the epilogue looks.
            ResultSource::Completion => {}
            ResultSource::Empty => {
                self.text_pair("");
                self.call(self.imports.new_owned_string);
                self.set(code + 1);
            }
            ResultSource::Value(value) => {
                self.push_retained(value);
                self.set(code + 1);
            }
        }
    }

    /// Log the statement's own `errorInfo` frame when it completed with an
    /// error, because a compiled statement reaches no eval loop that would.
    ///
    /// The runtime owns the `already_logged` protocol, so the innermost
    /// statement of a nest logs and the rest are no-ops — the same dedup C
    /// applies within one bytecode frame. Without this a compiled body's
    /// error carries neither a `while executing "<text>"` frame nor the TIP
    /// 348 `CALL` entry, and `errorLine` never advances.
    fn log_error_site(&mut self, statement: &NativeStatement) {
        let Some(site) = statement.site.as_ref() else {
            return;
        };
        if !can_fail(&statement.ops) {
            return;
        }
        let (line, text) = (i64::from(site.line), site.text.clone());
        let code = self.code_local(statement.completion);
        self.get(code);
        self.i32(TCL_ERROR);
        self.push(WasmOp::I32Eq);
        self.open(WasmOp::If, Label::Plain);
        self.i32(line);
        self.text_pair(&text);
        self.call(self.imports.log_command);
        self.close();
    }

    /// Abandon the statement with an error completion.
    pub(super) fn fail(&mut self, completion: tcl_compiler::executable_ir::CompletionId) {
        let code = self.code_local(completion);
        self.i32(1);
        self.set(code);
        self.release_scratch();
        self.br(Label::Abort);
    }

    /// Abandon the statement with the completion triple the frame holds.
    pub(super) fn fail_with_frame_completion(
        &mut self,
        completion: tcl_compiler::executable_ir::CompletionId,
    ) {
        self.adopt_frame_completion(completion);
        self.release_scratch();
        self.br(Label::Abort);
    }

    /// Move the frame's completion triple into the statement's completion
    /// locals.
    fn adopt_frame_completion(&mut self, completion: tcl_compiler::executable_ir::CompletionId) {
        let code = self.code_local(completion);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.set(code);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.set_owned(code + 1);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.set_owned(code + 2);
    }

    /// Hand the completion triple to the caller through `out` and give up
    /// ownership of it: the runtime takes the reference on each non-null
    /// pointer, so the locals are nulled and the epilogue's release loop
    /// skips them.
    ///
    /// A null result is not an omission: it means the runtime's own current
    /// result is the body's answer, which is what an evaluated source rung
    /// and every error edge leave behind.
    pub(super) fn write_completion_out(
        &mut self,
        completion: tcl_compiler::executable_ir::CompletionId,
    ) {
        let code = self.code_local(completion);
        self.param(PARAM_OUT);
        self.get(code);
        self.store_i32(i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.param(PARAM_OUT);
        self.get(code + 1);
        self.store_i32(i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.i32(0);
        self.set(code + 1);
        self.param(PARAM_OUT);
        self.get(code + 2);
        self.store_i32(i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.i32(0);
        self.set(code + 2);
    }

    /// After a completion-writing intrinsic: fail on a non-OK code, else move
    /// the result into `dst` and release the options.
    pub(super) fn take_frame_result(
        &mut self,
        dst: NativeValueId,
        completion: tcl_compiler::executable_ir::CompletionId,
    ) {
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.open(WasmOp::If, Label::Plain);
        self.fail_with_frame_completion(completion);
        self.close();
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.set_value_owned(dst);
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.call(self.imports.obj_release);
        self.release_scratch();
    }

    /// Both stage imports retain their own first-Host-cause check through
    /// `call`; a Guest failure abandons this statement before any later stage.
    fn boolean_conversion(
        &mut self,
        dst: NativeValueId,
        src: NativeValueId,
        tag: i32,
        import: u32,
        completion: tcl_compiler::executable_ir::CompletionId,
    ) {
        self.get(self.local_of(src));
        self.i32(i64::from(tag));
        self.frame_offset(FRAME_SCRATCH_I32);
        self.call(import);
        self.open(WasmOp::If, Label::Plain);
        self.fail(completion);
        self.close();
        self.get(LOCAL_FRAME);
        self.load_i32(FRAME_SCRATCH_I32);
        self.set(self.local_of(dst));
    }

    #[allow(clippy::too_many_lines)]
    fn emit_op(&mut self, op: &NativeOp, completion: tcl_compiler::executable_ir::CompletionId) {
        match op {
            NativeOp::ConstInt { dst, value } => {
                self.i64(*value);
                self.set(self.local_of(*dst));
            }
            NativeOp::ConstDouble { dst, value } => {
                self.f64(*value);
                self.set(self.local_of(*dst));
            }
            NativeOp::ConstBool { dst, value } => {
                self.i32(i64::from(*value));
                self.set(self.local_of(*dst));
            }
            NativeOp::ConstStr { dst, text } => {
                self.text_pair(text);
                self.call(self.imports.new_owned_string);
                self.set_value_owned(*dst);
            }
            NativeOp::Box { dst, src } => {
                self.box_value(*src);
                self.set_value_owned(*dst);
            }
            NativeOp::Unbox { dst, src, target } => {
                self.get(self.local_of(*src));
                let (import, offset) = match target {
                    NativeType::I64 => (self.imports.value_get_wide_int, FRAME_SCRATCH_I64),
                    NativeType::F64 => (self.imports.value_get_double, FRAME_SCRATCH_I64),
                    NativeType::Bool => {
                        unreachable!("Boolean operands carry their reached purpose")
                    }
                    NativeType::Obj => (self.imports.obj_retain, 0),
                };
                if *target == NativeType::Obj {
                    self.call(import);
                    self.set_value_owned(*dst);
                    return;
                }
                self.frame_offset(offset);
                self.call(import);
                self.open(WasmOp::If, Label::Plain);
                self.fail(completion);
                self.close();
                self.get(LOCAL_FRAME);
                match target {
                    NativeType::I64 => self.load_i64(offset),
                    NativeType::F64 => self.load_f64(offset),
                    _ => self.load_i32(offset),
                }
                self.set(self.local_of(*dst));
            }
            NativeOp::UnboxBool { dst, src, purpose } => {
                self.boolean_conversion(
                    *dst,
                    *src,
                    *purpose as i32,
                    self.imports.value_get_bool_for_purpose,
                    completion,
                );
            }
            NativeOp::BooleanExpressionResult {
                dst,
                src,
                production,
            } => {
                self.boolean_conversion(
                    *dst,
                    *src,
                    *production as i32,
                    self.imports.value_get_expression_bool,
                    completion,
                );
            }
            NativeOp::Truth { dst, src } => {
                self.get(self.local_of(*src));
                match self.ty(*src) {
                    NativeType::I64 => {
                        self.push(WasmOp::I64Eqz);
                        self.push(WasmOp::I32Eqz);
                    }
                    NativeType::F64 => {
                        self.f64(0.0);
                        self.push(WasmOp::F64Ne);
                    }
                    NativeType::Bool | NativeType::Obj => {}
                }
                self.set(self.local_of(*dst));
            }
            NativeOp::IntToDouble { dst, src } => {
                self.get(self.local_of(*src));
                self.push(WasmOp::F64ConvertI64S);
                self.set(self.local_of(*dst));
            }
            NativeOp::BoolToInt { dst, src } => {
                self.get(self.local_of(*src));
                self.push(WasmOp::I64ExtendI32U);
                self.set(self.local_of(*dst));
            }
            NativeOp::IntBinary { dst, op, lhs, rhs } => {
                let (a, b) = (self.local_of(*lhs), self.local_of(*rhs));
                let d = self.local_of(*dst);
                self.int_binary(*op, a, b, d);
            }
            NativeOp::IntNeg { dst, src } => {
                self.i64(0);
                self.get(self.local_of(*src));
                self.push(WasmOp::I64Sub);
                self.set(self.local_of(*dst));
            }
            NativeOp::IntBitNot { dst, src } => {
                self.get(self.local_of(*src));
                self.i64(-1);
                self.push(WasmOp::I64Xor);
                self.set(self.local_of(*dst));
            }
            NativeOp::DoubleBinary { dst, op, lhs, rhs } => {
                self.get(self.local_of(*lhs));
                self.get(self.local_of(*rhs));
                self.push(match op {
                    DoubleOp::Add => WasmOp::F64Add,
                    DoubleOp::Sub => WasmOp::F64Sub,
                    DoubleOp::Mul => WasmOp::F64Mul,
                    DoubleOp::Div => WasmOp::F64Div,
                });
                self.set(self.local_of(*dst));
            }
            NativeOp::DoubleNeg { dst, src } => {
                self.get(self.local_of(*src));
                self.push(WasmOp::F64Neg);
                self.set(self.local_of(*dst));
            }
            NativeOp::Compare {
                dst,
                op,
                kind,
                lhs,
                rhs,
            } => {
                self.get(self.local_of(*lhs));
                self.get(self.local_of(*rhs));
                self.push(compare_op(*op, *kind));
                self.set(self.local_of(*dst));
            }
            NativeOp::NotBool { dst, src } => {
                self.get(self.local_of(*src));
                self.push(WasmOp::I32Eqz);
                self.set(self.local_of(*dst));
            }
            NativeOp::DynamicBinary {
                dst, op, lhs, rhs, ..
            } => self.dynamic_binary(*dst, *op, *lhs, *rhs, completion),
            NativeOp::DynamicCompare {
                dst, op, lhs, rhs, ..
            } => self.dynamic_compare(*dst, *op, *lhs, *rhs, completion),
            NativeOp::DynamicUnary { dst, op, src } => {
                self.dynamic_unary(*dst, *op, *src, completion);
            }
            NativeOp::MathOp { dst, op, args } => {
                self.text_pair(op);
                self.argv(args);
                self.i32(i64::try_from(args.len()).unwrap_or(0));
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.mathop);
                self.push(WasmOp::Drop);
                self.take_frame_result(*dst, completion);
            }
            NativeOp::MathFunc { dst, name, args } => {
                self.text_pair(name);
                self.argv(args);
                self.i32(i64::try_from(args.len()).unwrap_or(0));
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.mathfunc);
                self.push(WasmOp::Drop);
                self.take_frame_result(*dst, completion);
            }
            NativeOp::ExprEval { dst, text } => {
                self.text_pair(text);
                self.call(self.imports.new_owned_string);
                self.set_owned(LOCAL_SCRATCH_OBJ_A);
                self.scratch_owned.push(LOCAL_SCRATCH_OBJ_A);
                self.get(LOCAL_SCRATCH_OBJ_A);
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.expr_eval);
                self.push(WasmOp::Drop);
                self.take_frame_result(*dst, completion);
            }
            NativeOp::ExprBoolEval { dst, text } => {
                self.text_pair(text);
                self.call(self.imports.new_owned_string);
                self.set_owned(LOCAL_SCRATCH_OBJ_A);
                self.scratch_owned.push(LOCAL_SCRATCH_OBJ_A);
                self.get(LOCAL_SCRATCH_OBJ_A);
                self.frame_offset(FRAME_COMPLETION);
                self.frame_offset(FRAME_SCRATCH_I32);
                self.call(self.imports.expr_bool_eval);
                self.push(WasmOp::Drop);
                self.get(LOCAL_FRAME);
                self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_CODE_OFFSET));
                self.open(WasmOp::If, Label::Plain);
                self.fail_with_frame_completion(completion);
                self.close();
                self.get(LOCAL_FRAME);
                self.load_i32(FRAME_SCRATCH_I32);
                self.set(self.local_of(*dst));
                self.get(LOCAL_FRAME);
                self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_RESULT_OFFSET));
                self.call(self.imports.obj_release);
                self.get(LOCAL_FRAME);
                self.load_i32(FRAME_COMPLETION + i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
                self.call(self.imports.obj_release);
                self.release_scratch();
            }
            NativeOp::IfElse {
                condition,
                then_ops,
                else_ops,
                result,
            } => {
                self.get(self.local_of(*condition));
                self.open(WasmOp::If, Label::Plain);
                for op in then_ops {
                    self.emit_op(op, completion);
                }
                if let Some(IfElseResult { dst, then_src, .. }) = result {
                    self.copy_value(*then_src, *dst);
                }
                self.else_();
                for op in else_ops {
                    self.emit_op(op, completion);
                }
                if let Some(IfElseResult { dst, else_src, .. }) = result {
                    self.copy_value(*else_src, *dst);
                }
                self.close();
            }
            NativeOp::CellRead { dst, place, .. } => {
                self.cell_name_args(place);
                match place {
                    CellPlace::Named { .. } => self.call(self.imports.var_get),
                    CellPlace::Element { .. } => self.call(self.imports.var_get_element),
                }
                self.tee(LOCAL_SCRATCH_I32);
                self.push(WasmOp::I32Eqz);
                self.open(WasmOp::If, Label::Plain);
                self.fail(completion);
                self.close();
                self.get(LOCAL_SCRATCH_I32);
                self.set_value_owned(*dst);
            }
            NativeOp::CellWrite { place, src, .. } => {
                self.cell_name_args(place);
                self.push_retained(*src);
                match place {
                    CellPlace::Named { .. } => self.call(self.imports.var_set),
                    CellPlace::Element { .. } => self.call(self.imports.var_set_element),
                }
                self.fail_on_code(completion);
            }
            NativeOp::CellIncr {
                dst,
                place,
                delta,
                cached,
                ..
            } => {
                if let Some(candidate) = cached {
                    self.try_cached_increment(place, *delta, candidate);
                } else {
                    self.text_pair(&place.spelling());
                    self.push_boxed(*delta);
                    self.call(self.imports.var_incr);
                }
                self.tee(LOCAL_SCRATCH_I32);
                self.push(WasmOp::I32Eqz);
                self.open(WasmOp::If, Label::Plain);
                self.fail(completion);
                self.close();
                self.get(LOCAL_SCRATCH_I32);
                self.set_value_owned(*dst);
                self.release_scratch();
            }
            NativeOp::CellAppend {
                place,
                values,
                list,
                ..
            } => {
                self.text_pair(&place.spelling());
                self.argv(values);
                self.i32(i64::try_from(values.len()).unwrap_or(0));
                self.i32(i64::from(*list));
                self.call(self.imports.var_update);
                self.fail_on_code(completion);
            }
            NativeOp::Concat { dst, parts } => {
                self.argv(parts);
                self.i32(i64::try_from(parts.len()).unwrap_or(0));
                self.call(self.imports.word_concat);
                self.tee(LOCAL_SCRATCH_I32);
                self.push(WasmOp::I32Eqz);
                self.open(WasmOp::If, Label::Plain);
                self.fail(completion);
                self.close();
                self.get(LOCAL_SCRATCH_I32);
                self.set_value_owned(*dst);
                self.release_scratch();
            }
            NativeOp::Puts { src } => {
                self.push_retained(*src);
                self.call(self.imports.puts);
                self.set(self.code_local(completion));
            }
            NativeOp::Invoke { argv } => {
                self.argv(argv);
                self.i32(i64::try_from(argv.len()).unwrap_or(0));
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.invoke_argv);
                self.push(WasmOp::Drop);
                self.adopt_frame_completion(completion);
                self.release_scratch();
            }
            NativeOp::NestedInvoke { dst, argv } => {
                self.argv(argv);
                self.i32(i64::try_from(argv.len()).unwrap_or(0));
                self.frame_offset(FRAME_COMPLETION);
                self.call(self.imports.invoke_argv);
                self.push(WasmOp::Drop);
                self.take_frame_result(*dst, completion);
            }
            NativeOp::Complete { code, result } => {
                let code_local = self.code_local(completion);
                // A compiled `return` *is* the `return` command, so it records
                // the pending return state that command records for its plain
                // form. Without it the enclosing procedure's return boundary
                // (`Interp::settle_return`) consumes whatever an earlier
                // `return -level N` left behind, and the call propagates code
                // 2 — or a stale requested code — instead of its value. Only
                // `return` reaches this arm: every option-carrying form keeps
                // the generic invocation, which records the state itself, and
                // `settle_return` ignores every completion code but `Return`.
                if *code == CompletionCode::Return {
                    self.i32(PLAIN_RETURN_LEVEL);
                    self.i32(CompletionCode::Ok.as_int());
                    self.call(self.imports.return_state);
                }
                self.i32(code.as_int());
                self.set(code_local);
                if let Some(result) = result {
                    self.push_retained(*result);
                    self.set_owned(code_local + 1);
                }
            }
            NativeOp::DefineProc {
                entry_key,
                argv,
                dispatch,
            } => {
                self.argv(argv);
                self.i32(i64::try_from(argv.len()).unwrap_or(0));
                self.push_entry_index(entry_key);
                let import = match dispatch {
                    tcl_compiler::native_lowering::ir::DefinitionDispatch::Proved => {
                        self.imports.proc_define_original_argv
                    }
                    tcl_compiler::native_lowering::ir::DefinitionDispatch::RuntimeGuarded(
                        identity,
                    ) => {
                        self.i64(i64::from_le_bytes(identity.value().to_le_bytes()));
                        self.imports.proc_define_guarded_argv
                    }
                };
                self.frame_offset(FRAME_COMPLETION);
                self.call(import);
                self.push(WasmOp::Drop);
                self.adopt_frame_completion(completion);
                self.release_scratch();
            }
            NativeOp::EvalSource { text, .. } => {
                self.text_pair(text);
                self.call(self.imports.obj_new_string);
                self.call(self.imports.eval_code);
                self.set(self.code_local(completion));
            }
        }
    }

    /// The completion code an adopting runtime call left on the stack: keep
    /// it as the statement's code and abandon the statement when non-zero.
    fn fail_on_code(&mut self, completion: tcl_compiler::executable_ir::CompletionId) {
        let code = self.code_local(completion);
        self.tee(code);
        self.open(WasmOp::If, Label::Plain);
        self.release_scratch();
        self.br(Label::Abort);
        self.close();
        self.release_scratch();
    }

    /// The `entry` argument of a proc definition: the module's own table
    /// index for a body it compiled and installed, or `0` — the ABI's
    /// "source body only" — for one it did not.
    fn push_entry_index(&mut self, id: &tcl_compiler::ir::ProcedureId) {
        match self.table.slot(id) {
            Some(slot) => {
                self.body.push(WasmInstruction::with_operands(
                    WasmOp::GlobalGet,
                    leb128_unsigned(u64::from(self.table.base_global)),
                ));
                self.i32(i64::from(slot));
                self.push(WasmOp::I32Add);
            }
            None => self.i32(0),
        }
    }

    fn cell_name_args(&mut self, place: &CellPlace) {
        match place {
            CellPlace::Named { name } => self.text_pair(name),
            CellPlace::Element { name, key } => {
                self.text_pair(name);
                self.text_pair(key);
            }
        }
    }

    /// `dst = src` for values of one machine type, retaining a boxed value.
    fn copy_value(&mut self, src: NativeValueId, dst: NativeValueId) {
        if self.ty(dst) == NativeType::Obj {
            self.push_retained(src);
            self.set_value_owned(dst);
        } else {
            self.get(self.local_of(src));
            self.set(self.local_of(dst));
        }
    }
}
