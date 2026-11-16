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

//! One leaf statement: the typed attempt, the prebuilt-argv invocation, its word plans and the adoption of its completion.

use super::super::ir::WasmOp;
use super::super::leaf_invoke::{WasmInvokeNode, WasmLeafInvokePlan, WasmWordPlan};
use super::function_facts::span_key;
use super::{ArgvImports, FunctionMode, TCL_ERROR, WasmEmitter, completion_offset, slot_offset};
use tcl_compiler::ir::Statement;
use tcl_lexer::Span;
use tcl_runtime_api::codegen_abi::{
    WASM32_COMPLETION_CODE_OFFSET, WASM32_COMPLETION_OPTIONS_OFFSET,
    WASM32_COMPLETION_RESULT_OFFSET,
};

impl WasmEmitter {
    /// Run one emission alternative, rolling the body back if it declines.
    ///
    /// A declining `try_*` emitter is not necessarily a no-op: it may append
    /// part of its sequence before reaching a word it cannot prove. Anything
    /// that falls through to another alternative must therefore undo the
    /// partial work first, or the abandoned prefix stays in the body with its
    /// operands stranded on the operand stack.
    pub(super) fn attempt(&mut self, emit: impl FnOnce(&mut Self) -> bool) -> bool {
        let body_len = self.body.len();
        let data_len = self.data.len();
        let data_offset = self.data_offset;
        if emit(self) {
            true
        } else {
            self.body.truncate(body_len);
            self.data.truncate(data_len);
            self.data_offset = data_offset;
            false
        }
    }

    pub(super) fn try_emit_typed_statement(&mut self, statement: &Statement) -> bool {
        let Some(aot) = self.general_imports().aot else {
            return false;
        };
        match statement {
            Statement::AssignConst { span, .. } | Statement::AssignValue { span, .. } => {
                let Some((name, value)) =
                    self.facts.direct_assignments.get(&span_key(*span)).cloned()
                else {
                    return false;
                };
                if self.mode == FunctionMode::DirectProc {
                    let Some(slot) = self.local_slots.get(&name).copied() else {
                        return false;
                    };
                    self.push_i32(i64::from(slot));
                    if !self.box_value(&value) {
                        return false;
                    }
                    self.call(aot.local_set);
                } else if self.mode == FunctionMode::Top {
                    self.push_text_pair(&name);
                    if !self.box_value(&value) {
                        return false;
                    }
                    self.call(aot.var_set);
                } else {
                    return false;
                }
                self.emit_completion_dispatch();
                true
            }
            Statement::Return {
                expr: Some(expr), ..
            } if self.mode == FunctionMode::DirectProc => {
                if !self.emit_expr_value(expr) {
                    return false;
                }
                self.call(aot.frame_pop);
                self.push(WasmOp::Return);
                true
            }
            Statement::Call { span, .. } => self.try_emit_leaf_invocation(*span),
            _ => false,
        }
    }

    /// The general tier's normal leaf-command path: evaluate every word into a
    /// transient call frame, dispatch the complete argv through the runtime's
    /// ordinary command resolution, and release everything before the
    /// completion is allowed to branch anywhere.
    ///
    /// The whole statement sits inside one `block`. Word evaluation branches to
    /// its end on an abrupt completion, so there is exactly one cleanup path:
    /// every object slot was nulled up front and is released null-safely, then
    /// the frame is freed. Only after that does the completion dispatch run and
    /// possibly `br`/`return` out of the statement, so no exit path can leak.
    fn try_emit_leaf_invocation(&mut self, span: Span) -> bool {
        // A direct procedure returns a value, so the completion dispatch's bare
        // `return` is not well typed there. Its body is a single proven
        // expression return, so no leaf call reaches this in practice; the
        // guard keeps that a property of the code rather than of the caller.
        if self.mode == FunctionMode::DirectProc {
            return false;
        }
        let Some(aot) = self.general_imports().aot else {
            return false;
        };
        let Some(plan) = self.facts.leaf_invocations.get(&span_key(span)).cloned() else {
            return false;
        };
        let argv = aot.argv;

        self.push_i32(i64::from(plan.frame_bytes));
        self.push_i32(i64::from(plan.frame_align));
        self.call(argv.frame_alloc);
        self.local_set(self.frame_local);
        for slot in 0..plan.object_slots {
            self.local_get(self.frame_local);
            self.push_i32(0);
            self.store_i32(slot_offset(slot));
        }
        self.push_i32(0);
        self.local_set(self.code_local);

        let abort = self.open_frame(WasmOp::Block);
        self.invocation_abort = Some(abort);
        let candidate = self
            .retained_i64
            .as_ref()
            .filter(|candidate| candidate.span == span && self.mode == FunctionMode::Top)
            .cloned();
        self.emit_invoke_node(&plan.root, &plan, argv, abort, true, candidate.as_ref());
        self.invocation_abort = None;
        self.close_frame();

        for slot in 0..plan.object_slots {
            self.local_get(self.frame_local);
            self.load_i32(slot_offset(slot));
            self.call(argv.object_release);
        }
        self.local_get(self.frame_local);
        self.call(argv.frame_free);
        self.push(WasmOp::Drop);

        let imports = self.general_imports();
        self.raw_call(imports.host_refusal_pending);
        self.open_frame(WasmOp::If);
        self.emit_general_refusal_return(imports);
        self.close_frame();
        self.dispatch_stashed_code();
        true
    }

    /// Emit one invocation: its words, the argv dispatch, and adoption of the
    /// completion's owned result and options into frame slots the single
    /// cleanup path releases.
    ///
    /// A nested invocation additionally propagates an abrupt completion to the
    /// statement's `abort` block; the outermost one leaves its code in
    /// `code_local` for the completion dispatch after cleanup.
    fn emit_invoke_node(
        &mut self,
        node: &WasmInvokeNode,
        plan: &WasmLeafInvokePlan,
        argv: ArgvImports,
        abort: u32,
        outermost: bool,
        candidate: Option<&super::super::native_add::retained_publication::RetainedI64Publication>,
    ) {
        for word in &node.words {
            self.emit_word_plan(word, plan, argv, abort);
        }
        let completion = plan.completion_base + completion_offset(node.completion);

        self.local_get(self.frame_local);
        self.push_i32(slot_offset(node.argv_slot));
        self.push(WasmOp::I32Add);
        self.push_i32(i64::try_from(node.words.len()).unwrap_or(i64::MAX));
        if let Some(candidate) = candidate {
            self.push_i64(i64::from_le_bytes(candidate.guard.value().to_le_bytes()));
            self.push_text_pair(
                core::str::from_utf8(&candidate.parameters).expect("original UTF-8 formal source"),
            );
            self.push_text_pair(
                core::str::from_utf8(&candidate.body).expect("original UTF-8 body source"),
            );
        }
        self.local_get(self.frame_local);
        self.push_i32(i64::from(completion));
        self.push(WasmOp::I32Add);
        self.call(if candidate.is_some() {
            self.retained_i64_import.expect("retained candidate import")
        } else {
            argv.invoke_argv
        });
        self.push(WasmOp::Drop);

        self.local_get(self.frame_local);
        self.load_i32(i64::from(completion + WASM32_COMPLETION_CODE_OFFSET));
        self.local_set(self.code_local);
        self.adopt_completion_handle(
            completion + WASM32_COMPLETION_RESULT_OFFSET,
            node.result_slot,
        );
        self.adopt_completion_handle(
            completion + WASM32_COMPLETION_OPTIONS_OFFSET,
            node.options_slot,
        );

        if !outermost {
            self.local_get(self.code_local);
            self.open_frame(WasmOp::If);
            self.br(abort);
            self.close_frame();
        }
    }

    /// Move one owned completion handle into the frame slot the statement's
    /// cleanup path releases.
    fn adopt_completion_handle(&mut self, completion_field: i32, slot: usize) {
        self.local_get(self.frame_local);
        self.local_get(self.frame_local);
        self.load_i32(i64::from(completion_field));
        self.store_i32(slot_offset(slot));
    }

    fn emit_word_plan(
        &mut self,
        word: &WasmWordPlan,
        plan: &WasmLeafInvokePlan,
        argv: ArgvImports,
        abort: u32,
    ) {
        match word {
            WasmWordPlan::Literal { slot, text } => {
                self.local_get(self.frame_local);
                self.push_text_pair(text);
                self.call(argv.string_owned);
                self.store_i32(slot_offset(*slot));
            }
            WasmWordPlan::Scalar { slot, name } => {
                self.local_get(self.frame_local);
                self.push_text_pair(name);
                self.call(argv.var_get);
                self.store_i32(slot_offset(*slot));
                self.emit_word_error_guard(*slot, abort);
            }
            WasmWordPlan::Element { slot, name, key } => {
                self.local_get(self.frame_local);
                self.push_text_pair(name);
                self.push_text_pair(key);
                self.call(argv.var_get_element);
                self.store_i32(slot_offset(*slot));
                self.emit_word_error_guard(*slot, abort);
            }
            WasmWordPlan::Invoke(node) => {
                self.emit_invoke_node(node, plan, argv, abort, false, None);
            }
            WasmWordPlan::Concat {
                slot,
                parts_slot,
                parts,
            } => {
                for part in parts {
                    self.emit_word_plan(part, plan, argv, abort);
                }
                self.local_get(self.frame_local);
                self.local_get(self.frame_local);
                self.push_i32(slot_offset(*parts_slot));
                self.push(WasmOp::I32Add);
                self.push_i32(i64::try_from(parts.len()).unwrap_or(i64::MAX));
                self.call(argv.word_concat);
                self.store_i32(slot_offset(*slot));
                self.emit_word_error_guard(*slot, abort);
            }
        }
    }

    /// A null word means the runtime reported a Tcl error while evaluating it
    /// (a missing variable, a failed read trace). Record `TCL_ERROR` and leave
    /// through the statement's single cleanup path.
    fn emit_word_error_guard(&mut self, slot: usize, abort: u32) {
        self.local_get(self.frame_local);
        self.load_i32(slot_offset(slot));
        self.push(WasmOp::I32Eqz);
        self.open_frame(WasmOp::If);
        self.push_i32(TCL_ERROR);
        self.local_set(self.code_local);
        self.br(abort);
        self.close_frame();
    }
}
