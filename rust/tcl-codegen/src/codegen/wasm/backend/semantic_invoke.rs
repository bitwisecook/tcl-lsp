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

//! The semantic invocation mode and its guarded intrinsic attempt.

use super::super::ir::{ValType, WasmFunction, WasmOp};
use super::super::semantic_plan::WasmGenericInvokePlan;
use super::{
    GUARDED_STATUS_LOCAL, GUARDED_TOKEN_LOCAL, GuardedIntrinsicImports, SEMANTIC_CODE_LOCAL,
    SEMANTIC_COMPLETION_LOCAL, SEMANTIC_FRAME_LOCAL, SEMANTIC_OPTIONS_LOCAL, SEMANTIC_RESULT_LOCAL,
    SemanticCallFrameLayout, SemanticImports, WasmEmitter, semantic_word_local,
};
use tcl_compiler::mixed_region_plan::GuardedSelectionEvidence;
use tcl_registry::SemanticOperationId;
use tcl_runtime_api::codegen_abi::{
    WASM32_COMPLETION_ALIGN, WASM32_COMPLETION_CODE_OFFSET, WASM32_COMPLETION_OPTIONS_OFFSET,
    WASM32_COMPLETION_RESULT_OFFSET,
};

impl WasmEmitter {
    /// Emit the selected semantic invocation through this emitter's shared
    /// instruction and data builders.
    pub(super) fn finish_semantic_invoke(&mut self, plan: &WasmGenericInvokePlan) -> WasmFunction {
        const FRAME_LOCAL: u64 = 0;
        const COMPLETION_LOCAL: u64 = 1;
        const CODE_LOCAL: u64 = 2;
        const RESULT_LOCAL: u64 = 3;
        const OPTIONS_LOCAL: u64 = 4;
        let word_local = |index: usize| u64::try_from(5 + index).expect("word local fits u64");
        let imports = self.semantic_imports();
        let layout = SemanticCallFrameLayout::validated(plan.argv_literals.len());
        let literals = plan
            .argv_literals
            .iter()
            .map(|literal| self.intern(literal))
            .collect::<Vec<_>>();

        self.push_i32(i64::from(layout.bytes));
        self.push_i32(i64::from(WASM32_COMPLETION_ALIGN));
        self.call(imports.frame_alloc);
        self.local_set(FRAME_LOCAL);

        for (index, (offset, length)) in literals.iter().copied().enumerate() {
            self.push_i32(offset);
            self.push_i32(length);
            self.call(imports.string_owned);
            self.local_set(word_local(index));
            self.local_get(FRAME_LOCAL);
            self.local_get(word_local(index));
            self.store_i32(i64::try_from(index * 4).expect("argv offset fits i64"));
        }

        self.local_get(FRAME_LOCAL);
        self.push_i32(i64::try_from(plan.argv_literals.len()).expect("validated argc"));
        self.local_get(FRAME_LOCAL);
        self.push_i32(i64::from(layout.completion_offset));
        self.push(WasmOp::I32Add);
        self.local_tee(COMPLETION_LOCAL);
        self.call(imports.invoke_argv);
        self.push(WasmOp::Drop);
        self.emit_semantic_refusal_check(plan.argv_literals.len(), false);

        self.local_get(COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.local_set(CODE_LOCAL);
        self.local_get(COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.call(imports.object_retain);
        self.local_set(RESULT_LOCAL);
        self.local_get(COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.call(imports.object_retain);
        self.local_set(OPTIONS_LOCAL);
        self.local_get(COMPLETION_LOCAL);
        self.call(imports.completion_release);

        for index in 0..literals.len() {
            self.local_get(word_local(index));
            self.call(imports.object_release);
        }
        self.local_get(FRAME_LOCAL);
        self.call(imports.frame_free);
        self.push(WasmOp::Drop);

        self.local_get(CODE_LOCAL);
        self.local_get(RESULT_LOCAL);
        self.local_get(OPTIONS_LOCAL);
        self.push(WasmOp::Return);

        let mut local_names = vec![
            "$frame".to_string(),
            "$completion".to_string(),
            "$code".to_string(),
            "$result".to_string(),
            "$options".to_string(),
        ];
        local_names.extend((0..literals.len()).map(|index| format!("$word{index}")));
        WasmFunction {
            name: plan.function_name.clone(),
            params: Vec::new(),
            results: vec![ValType::I32, ValType::I32, ValType::I32],
            locals: vec![ValType::I32; 5 + literals.len()],
            body: std::mem::take(&mut self.body),
            local_names,
            exported: true,
            source_range: None,
            kind: "semantic-generic-invoke".to_string(),
        }
    }

    /// Emit a guarded intrinsic attempt over the semantic plan's sole,
    /// already-materialised argv. Any runtime decline executes the exact
    /// generic argv path; no source words are re-evaluated or replayed.
    pub(super) fn finish_guarded_intrinsic_invoke(
        &mut self,
        plan: &WasmGenericInvokePlan,
        evidence: &GuardedSelectionEvidence,
    ) -> WasmFunction {
        let semantic = self.semantic_imports();
        let guarded = self.guarded_intrinsic_imports();
        assert_eq!(plan.operation, evidence.operation());
        assert_eq!(evidence.guarded_plan().fast(), &plan.operation);
        self.emit_guarded_argv_frame(plan, semantic);
        self.emit_guarded_intrinsic_dispatch(plan, evidence, guarded);
        self.emit_guarded_intrinsic_completion_return(plan, semantic);
        self.guarded_intrinsic_function(plan)
    }

    fn emit_guarded_argv_frame(&mut self, plan: &WasmGenericInvokePlan, imports: SemanticImports) {
        let layout = SemanticCallFrameLayout::validated(plan.argv_literals.len());
        let literals = plan
            .argv_literals
            .iter()
            .map(|literal| self.intern(literal))
            .collect::<Vec<_>>();
        self.push_i32(i64::from(layout.bytes));
        self.push_i32(i64::from(WASM32_COMPLETION_ALIGN));
        self.call(imports.frame_alloc);
        self.local_set(SEMANTIC_FRAME_LOCAL);
        for (index, (offset, length)) in literals.iter().copied().enumerate() {
            self.push_i32(offset);
            self.push_i32(length);
            self.call(imports.string_owned);
            self.local_set(semantic_word_local(index));
            self.local_get(SEMANTIC_FRAME_LOCAL);
            self.local_get(semantic_word_local(index));
            self.store_i32(i64::try_from(index * 4).expect("argv offset fits i64"));
        }
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.push_i32(i64::from(layout.completion_offset));
        self.push(WasmOp::I32Add);
        self.local_set(SEMANTIC_COMPLETION_LOCAL);
    }

    fn emit_guarded_intrinsic_dispatch(
        &mut self,
        plan: &WasmGenericInvokePlan,
        evidence: &GuardedSelectionEvidence,
        imports: GuardedIntrinsicImports,
    ) {
        let SemanticOperationId::Intrinsic(intrinsic) = evidence.operation() else {
            unreachable!("guarded intrinsic evidence must retain an intrinsic operation");
        };
        let guard = evidence.guarded_plan().guard();
        let identity = guard.expected_identity();
        let argc = plan.argv_literals.len();
        self.push_i32(i64::from(intrinsic.stable_id()));
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.push_i32(i64::try_from(argc).expect("validated argc"));
        self.push_i32(i64::from(identity.namespace()));
        self.push_i64(i64::try_from(identity.value()).expect("guard identity fits i64"));
        self.push_i32(i64::from(guard.domains().bits()));
        self.call(imports.guard_prepare);
        self.local_set(GUARDED_TOKEN_LOCAL);
        self.emit_semantic_refusal_check(argc, true);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.push(WasmOp::I64Eqz);
        self.open_frame(WasmOp::If);
        self.emit_generic_argv_invoke(argc, SEMANTIC_FRAME_LOCAL, SEMANTIC_COMPLETION_LOCAL);
        self.push(WasmOp::Else);
        self.emit_guarded_token_path(intrinsic.stable_id(), argc, imports);
        self.close_frame();
    }

    fn emit_guarded_token_path(
        &mut self,
        intrinsic: u32,
        argc: usize,
        imports: GuardedIntrinsicImports,
    ) {
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.push_i32(i64::from(intrinsic));
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.push_i32(i64::try_from(argc).expect("validated argc"));
        self.call(imports.guard_check);
        self.emit_semantic_refusal_check(argc, true);
        self.push(WasmOp::I32Eqz);
        self.open_frame(WasmOp::If);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.call(imports.guard_release);
        self.push_i64(0);
        self.local_set(GUARDED_TOKEN_LOCAL);
        self.emit_generic_argv_invoke(argc, SEMANTIC_FRAME_LOCAL, SEMANTIC_COMPLETION_LOCAL);
        self.push(WasmOp::Else);
        self.push_i32(i64::from(intrinsic));
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.push_i32(i64::try_from(argc).expect("validated argc"));
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.call(imports.invoke_intrinsic_argv);
        self.local_set(GUARDED_STATUS_LOCAL);
        self.emit_semantic_refusal_check(argc, true);
        self.local_get(GUARDED_STATUS_LOCAL);
        self.push(WasmOp::I32Eqz);
        self.open_frame(WasmOp::If);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.call(imports.guard_release);
        self.push_i64(0);
        self.local_set(GUARDED_TOKEN_LOCAL);
        self.push(WasmOp::Else);
        self.local_get(GUARDED_TOKEN_LOCAL);
        self.call(imports.guard_release);
        self.push_i64(0);
        self.local_set(GUARDED_TOKEN_LOCAL);
        self.emit_generic_argv_invoke(argc, SEMANTIC_FRAME_LOCAL, SEMANTIC_COMPLETION_LOCAL);
        self.close_frame();
        self.close_frame();
    }

    fn emit_guarded_intrinsic_completion_return(
        &mut self,
        plan: &WasmGenericInvokePlan,
        imports: SemanticImports,
    ) {
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_CODE_OFFSET));
        self.local_set(SEMANTIC_CODE_LOCAL);
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_RESULT_OFFSET));
        self.call(imports.object_retain);
        self.local_set(SEMANTIC_RESULT_LOCAL);
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.load_i32(i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
        self.call(imports.object_retain);
        self.local_set(SEMANTIC_OPTIONS_LOCAL);
        self.local_get(SEMANTIC_COMPLETION_LOCAL);
        self.call(imports.completion_release);
        for index in 0..plan.argv_literals.len() {
            self.local_get(semantic_word_local(index));
            self.call(imports.object_release);
        }
        self.local_get(SEMANTIC_FRAME_LOCAL);
        self.call(imports.frame_free);
        self.push(WasmOp::Drop);
        self.local_get(SEMANTIC_CODE_LOCAL);
        self.local_get(SEMANTIC_RESULT_LOCAL);
        self.local_get(SEMANTIC_OPTIONS_LOCAL);
        self.push(WasmOp::Return);
    }

    fn guarded_intrinsic_function(&mut self, plan: &WasmGenericInvokePlan) -> WasmFunction {
        let mut local_names = vec![
            "$frame".to_string(),
            "$completion".to_string(),
            "$code".to_string(),
            "$result".to_string(),
            "$options".to_string(),
            "$guard_token".to_string(),
            "$intrinsic_status".to_string(),
        ];
        local_names.extend((0..plan.argv_literals.len()).map(|index| format!("$word{index}")));
        let mut locals = vec![ValType::I32; 5];
        locals.push(ValType::I64);
        locals.push(ValType::I32);
        locals.extend(std::iter::repeat_n(ValType::I32, plan.argv_literals.len()));
        WasmFunction {
            name: plan.function_name.clone(),
            params: Vec::new(),
            results: vec![ValType::I32, ValType::I32, ValType::I32],
            locals,
            body: std::mem::take(&mut self.body),
            local_names,
            exported: true,
            source_range: None,
            kind: "semantic-guarded-intrinsic-invoke".to_string(),
        }
    }
}
