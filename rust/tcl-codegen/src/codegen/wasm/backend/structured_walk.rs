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

//! Completion dispatch, condition expressions and the `Emit` implementation the structured walk drives.

use super::super::ir::WasmOp;
use super::{
    CONDITION_COMPLETION_OFFSET, CONDITION_FRAME_BYTES, CONDITION_SOURCE_OFFSET,
    CONDITION_TRUTH_OFFSET, LoopFrame, TCL_BREAK, TCL_CONTINUE, WasmEmitter,
};
use crate::codegen::emit::Emit;
use tcl_compiler::ir::Statement;
use tcl_runtime_api::codegen_abi::{
    WASM32_COMPLETION_ALIGN, WASM32_COMPLETION_CODE_OFFSET, WASM32_POINTER_BYTES,
};

impl WasmEmitter {
    /// Call the generic ABI over the exact argv and completion frame already
    /// owned by this semantic emission.
    pub(super) fn emit_generic_argv_invoke(
        &mut self,
        argc: usize,
        frame_local: u64,
        completion_local: u64,
    ) {
        self.local_get(frame_local);
        self.push_i32(i64::try_from(argc).expect("validated argc"));
        self.local_get(completion_local);
        self.call(self.semantic_imports().invoke_argv);
        self.push(WasmOp::Drop);
        self.emit_semantic_refusal_check(argc, true);
    }

    /// Honour the completion code a leaf command's [`tcl_eval_code`] left on the
    /// stack — the AOT realisation of "stop the script on the
    /// first non-`OK` command" loop (`eval_script_mode`), so abrupt completion
    /// propagates through compiled `if`/`while`/`for` instead of being swallowed.
    ///
    /// Inside a loop, `break` (3) / `continue` (4) re-enter that loop's structural
    /// scopes (identical to a literal `break`/`continue`, so a *dynamic* one — a
    /// called command that completes `break` — behaves the same). Any other
    /// non-`OK` code (error, return, a `return -code N`, or a break/continue with
    /// no enclosing loop) unwinds the function with `return`. `OK` (0) falls
    /// through to the next statement.
    pub(super) fn emit_completion_dispatch(&mut self) {
        // Stash the code; the dispatch reads it up to three times.
        self.local_set(self.code_local);
        self.dispatch_stashed_code();
    }

    /// [`Self::emit_completion_dispatch`] for a code already in `code_local`.
    pub(super) fn dispatch_stashed_code(&mut self) {
        let enclosing = self
            .loops
            .last()
            .map(|frame| (frame.break_block, frame.continue_block));
        self.dispatch_stashed_code_in_loop(enclosing);
    }

    fn dispatch_stashed_code_in_loop(&mut self, enclosing: Option<(u32, u32)>) {
        if let Some((break_block, continue_block)) = enclosing {
            self.emit_code_eq_branch(TCL_BREAK, break_block);
            self.emit_code_eq_branch(TCL_CONTINUE, continue_block);
        }
        self.local_get(self.code_local);
        self.open_frame(WasmOp::If);
        // Only a DirectProc has the frame emitted by emit_proc_prelude.
        self.emit_general_refusal_return(self.general_imports());
        self.close_frame();
    }

    /// Evaluate through the existing raw-evaluation/result/truth owner once,
    /// then release the source and completion before any structural edge.
    fn emit_condition_expression(&mut self, text: &str, loop_header: bool) {
        let imports = self.general_imports();
        let condition = imports.condition;
        let status_local = self.frame_local + 1;
        let truth_local = self.frame_local + 2;
        self.raw_call(imports.host_refusal_pending);
        self.open_frame(WasmOp::If);
        self.emit_general_refusal_return(imports);
        self.close_frame();
        self.push_i32(CONDITION_FRAME_BYTES);
        self.push_i32(i64::from(WASM32_COMPLETION_ALIGN));
        self.call(condition.frame_alloc);
        self.local_set(self.frame_local);
        self.local_get(self.frame_local);
        self.push(WasmOp::I32Eqz);
        self.open_frame(WasmOp::If);
        self.emit_general_refusal_return(imports);
        self.close_frame();
        for offset in (0..CONDITION_FRAME_BYTES).step_by(WASM32_POINTER_BYTES as usize) {
            self.local_get(self.frame_local);
            self.push_i32(0);
            self.store_i32(offset);
        }
        self.push_i32(0);
        self.local_set(self.code_local);
        self.push_i32(0);
        self.local_set(status_local);
        self.push_i32(0);
        self.local_set(truth_local);

        let abort = self.open_frame(WasmOp::Block);
        self.invocation_abort = Some(abort);
        self.local_get(self.frame_local);
        self.push_text_pair(text);
        self.call(condition.string_owned);
        self.store_i32(CONDITION_SOURCE_OFFSET);
        self.local_get(self.frame_local);
        self.load_i32(CONDITION_SOURCE_OFFSET);
        self.local_get(self.frame_local);
        self.push_i32(CONDITION_COMPLETION_OFFSET);
        self.push(WasmOp::I32Add);
        self.local_get(self.frame_local);
        self.push_i32(CONDITION_TRUTH_OFFSET);
        self.push(WasmOp::I32Add);
        self.call(condition.expr_bool_eval);
        self.local_set(status_local);
        self.local_get(status_local);
        self.open_frame(WasmOp::If);
        self.br(abort);
        self.close_frame();
        self.local_get(self.frame_local);
        self.load_i32(CONDITION_COMPLETION_OFFSET + i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.local_set(self.code_local);
        self.local_get(self.code_local);
        self.open_frame(WasmOp::If);
        self.br(abort);
        self.close_frame();
        self.local_get(self.frame_local);
        self.load_i32(CONDITION_TRUTH_OFFSET);
        self.local_set(truth_local);
        self.invocation_abort = None;
        self.close_frame();

        self.local_get(self.frame_local);
        self.load_i32(CONDITION_SOURCE_OFFSET);
        self.call(condition.object_release);
        self.local_get(self.frame_local);
        self.push_i32(CONDITION_COMPLETION_OFFSET);
        self.push(WasmOp::I32Add);
        self.call(condition.completion_release);
        self.local_get(self.frame_local);
        self.call(condition.frame_free);
        self.push(WasmOp::Drop);
        self.raw_call(imports.host_refusal_pending);
        self.open_frame(WasmOp::If);
        self.emit_general_refusal_return(imports);
        self.close_frame();
        self.local_get(status_local);
        self.open_frame(WasmOp::If);
        self.emit_general_refusal_return(imports);
        self.close_frame();

        // A loop test is outside its own body completion handler. Its failure
        // belongs to the caller of the whole while/for, possibly an outer loop.
        let enclosing = if loop_header {
            self.loops.iter().rev().nth(1)
        } else {
            self.loops.last()
        }
        .map(|frame| (frame.break_block, frame.continue_block));
        self.dispatch_stashed_code_in_loop(enclosing);
        self.local_get(truth_local);
    }

    /// `if (code == want) br <target>` — a guarded structural branch the
    /// completion dispatch uses for `break`/`continue`. The `if` frame is opened
    /// so the `br` depth is computed with it in place (crossing it, plus any
    /// enclosing `if`s, back to the loop scope).
    fn emit_code_eq_branch(&mut self, want: i64, target: u32) {
        self.local_get(self.code_local);
        self.push_i32(want);
        self.push(WasmOp::I32Eq);
        self.open_frame(WasmOp::If);
        self.br(target);
        self.close_frame();
    }
}

impl Emit for WasmEmitter {
    fn refuse_native_compilation_admission(&mut self) {
        self.push(WasmOp::Unreachable);
    }

    fn emit_typed_statement(&mut self, statement: &Statement, _source: &str) -> bool {
        self.attempt(|emitter| emitter.try_emit_typed_statement(statement))
    }

    fn emit_command_image(&mut self, source: &tcl_lexer::SourceImage) {
        // code = tcl_eval_code(box(text)); then honour an abrupt completion code
        // (error/return unwinds, break/continue re-enters the loop) instead of
        // swallowing it — the top-level result stays the interp's own result.
        self.box_bytes(source.bytes());
        self.call(self.general_imports().eval_code);
        self.emit_completion_dispatch();
    }

    fn begin_if(&mut self, cond_text: &str) {
        self.emit_condition_expression(cond_text, false);
        self.open_frame(WasmOp::If);
    }

    fn begin_else(&mut self) {
        // `else` stays in the same `if` frame — no depth change.
        self.push(WasmOp::Else);
    }

    fn end_if(&mut self) {
        self.close_frame();
    }

    fn begin_loop(&mut self) {
        // block (break scope) ⊃ loop (retest / back-edge). The continue scope
        // opens in `begin_loop_body`, after the guard.
        let break_block = self.open_frame(WasmOp::Block);
        let back_edge = self.open_frame(WasmOp::Loop);
        self.loops.push(LoopFrame {
            break_block,
            back_edge,
            continue_block: back_edge, // provisional; set in begin_loop_body
        });
    }

    fn loop_test(&mut self, cond_text: Option<&str>) {
        if let Some(cond) = cond_text {
            self.emit_condition_expression(cond, true);
            self.push(WasmOp::I32Eqz);
            if let Some(frame) = self.loops.last() {
                let brk = frame.break_block;
                self.br_if(brk);
            }
        }
    }

    fn begin_loop_body(&mut self) {
        let continue_block = self.open_frame(WasmOp::Block);
        if let Some(frame) = self.loops.last_mut() {
            frame.continue_block = continue_block;
        }
    }

    fn end_loop_body(&mut self) {
        // Close the continue scope: a `continue` (and the body's fall-through)
        // lands here, then runs any step and the back-edge.
        self.close_frame();
    }

    fn end_loop(&mut self) {
        if let Some(frame) = self.loops.last() {
            let back_edge = frame.back_edge;
            self.br(back_edge); // back-edge: re-test
        }
        self.close_frame(); // close loop
        self.close_frame(); // close break scope
        self.loops.pop();
    }

    fn emit_break(&mut self) {
        if let Some(frame) = self.loops.last() {
            let idx = frame.break_block;
            self.br(idx);
        }
    }

    fn emit_continue(&mut self) {
        if let Some(frame) = self.loops.last() {
            let idx = frame.continue_block;
            self.br(idx);
        }
    }

    fn emit_return(&mut self) {
        self.push(WasmOp::Return);
    }
}
