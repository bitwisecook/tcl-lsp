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

use super::*;
use tcl_compiler::executable_ir::{CompletionId, ExecutableFunctionId};
use tcl_compiler::native_lowering::ir::{NativeBlock, NativeStatement, NativeValue};
use tcl_compiler::native_lowering::representation::Representation;
use tcl_registry::native_boolean_truth::{
    NativeBooleanExpressionResultProduction as Production, NativeBooleanTruthPurpose as Purpose,
};

fn boolean_transport_function(completion: CompletionId) -> NativeFunction {
    NativeFunction {
        native_compilation_admission: None,
        values: vec![
            NativeValue {
                ty: NativeType::Obj,
                rep: Representation::Boxed(None),
            },
            NativeValue {
                ty: NativeType::Bool,
                rep: Representation::NativeBool,
            },
            NativeValue {
                ty: NativeType::Bool,
                rep: Representation::NativeBool,
            },
            NativeValue {
                ty: NativeType::Bool,
                rep: Representation::NativeBool,
            },
            NativeValue {
                ty: NativeType::Bool,
                rep: Representation::NativeBool,
            },
        ],
        blocks: vec![NativeBlock {
            id: NativeBlockId(0),
            statements: vec![NativeStatement {
                completion,
                node: None,
                site: None,
                ops: vec![
                    NativeOp::ConstStr {
                        dst: NativeValueId(0),
                        text: "opaque".into(),
                    },
                    NativeOp::UnboxBool {
                        dst: NativeValueId(1),
                        src: NativeValueId(0),
                        purpose: Purpose::LogicalAndInstruction,
                    },
                    NativeOp::BooleanExpressionResult {
                        dst: NativeValueId(2),
                        src: NativeValueId(0),
                        production: Production::InlineExpression,
                    },
                    NativeOp::BooleanExpressionResult {
                        dst: NativeValueId(3),
                        src: NativeValueId(0),
                        production: Production::PublicExpressionApi,
                    },
                    NativeOp::ExprBoolEval {
                        dst: NativeValueId(4),
                        text: "[original_callback]".into(),
                    },
                ],
            }],
            terminator: NativeTerminator::Return(completion),
        }],
        entry: NativeBlockId(0),
        completion_count: 1,
        max_argc: 0,
        protocol: EntryProtocol::Script,
    }
}

fn assert_combined_boolean_transport(
    emitted: &WasmFunction,
    imports: &NativeImports,
    declared: &[CodegenAbiImportId],
) {
    let combined: Vec<_> = emitted
        .body
        .iter()
        .enumerate()
        .filter(|(_, instruction)| {
            instruction.op == WasmOp::Call
                && instruction.operands == leb128_unsigned(u64::from(imports.expr_bool_eval))
        })
        .map(|(index, _)| index)
        .collect();
    assert_eq!(combined.len(), 1);
    let index = combined[0];
    let after = &emitted.body[index + 1..index + 9];
    assert_eq!(after[0].op, WasmOp::Call);
    assert_eq!(
        after[0].operands,
        leb128_unsigned(u64::from(imports.host_refusal_pending))
    );
    assert_eq!(after[1].op, WasmOp::If);
    assert_eq!(after[2].op, WasmOp::Br);
    assert_eq!(after[3].op, WasmOp::End);
    assert_eq!(after[4].op, WasmOp::Drop);
    assert_eq!(after[5].op, WasmOp::LocalGet);
    assert_eq!(
        after[6].op,
        WasmOp::I32Load,
        "the original Guest completion is inspected only after first Host"
    );
    assert_eq!(after[7].op, WasmOp::If);
    assert_eq!(
        declared[imports.expr_bool_eval as usize],
        CodegenAbiImportId::ExprBoolEval
    );
}

#[test]
fn original_boolean_stage_imports_check_first_host_cause_before_guest_status_or_out_slots() {
    // naming.numeric.original-primitive-boolean-vs-expression-truth
    // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
    // Typed software transport only. The original provider rows attest
    // reached getter/result effects separately from emitted instructions.
    let mut wasm = WasmModule::new();
    let mut declared = Vec::new();
    let imports = add_native_imports(&mut wasm, &mut |_, id| {
        let index = u32::try_from(declared.len()).unwrap();
        declared.push(id);
        index
    });
    let completion = CompletionId::new(ExecutableFunctionId::new(0), 0);
    let function = boolean_transport_function(completion);
    let slots = BTreeMap::new();
    let mut data = Vec::new();
    let mut offset = 0;
    let emitted = emit_function(
        "::top",
        "top",
        &function,
        imports,
        EntryTable {
            slots: &slots,
            base_global: 0,
        },
        ConstantPool {
            data: &mut data,
            offset: &mut offset,
        },
    );
    let reached: Vec<_> = emitted
        .body
        .iter()
        .enumerate()
        .filter_map(|(index, instruction)| {
            if instruction.op != WasmOp::Call {
                return None;
            }
            [
                imports.value_get_bool_for_purpose,
                imports.value_get_expression_bool,
            ]
            .into_iter()
            .find(|candidate| instruction.operands == leb128_unsigned(u64::from(*candidate)))
            .map(|import| (index, declared[import as usize]))
        })
        .collect();
    assert_eq!(
        reached.iter().map(|(_, id)| *id).collect::<Vec<_>>(),
        [
            CodegenAbiImportId::ValueGetBoolForPurpose,
            CodegenAbiImportId::ValueGetExpressionBool,
            CodegenAbiImportId::ValueGetExpressionBool,
        ]
    );
    for ((index, _), tag) in reached.into_iter().zip([6, 0, 1]) {
        assert_eq!(emitted.body[index - 4].op, WasmOp::I32Const);
        assert_eq!(emitted.body[index - 4].operands, leb128_signed(tag));
        let after = &emitted.body[index + 1..index + 7];
        assert_eq!(after[0].op, WasmOp::Call);
        assert_eq!(
            after[0].operands,
            leb128_unsigned(u64::from(imports.host_refusal_pending))
        );
        assert_eq!(after[1].op, WasmOp::If);
        assert_eq!(after[2].op, WasmOp::Br);
        assert_eq!(after[3].op, WasmOp::End);
        assert_eq!(
            after[4].op,
            WasmOp::If,
            "Guest status is read after first Host cause"
        );
        assert!(
            !after
                .iter()
                .any(|instruction| instruction.op == WasmOp::I32Load)
        );
    }
    assert_combined_boolean_transport(&emitted, &imports, &declared);
}
