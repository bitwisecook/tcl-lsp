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

//! Registration of the host imports each emission mode calls.

use super::super::ir::{ValType, WasmModule};
use super::{
    AotImports, ArgvImports, ConditionImports, GuardedIntrinsicImports, Imports,
    NativeI64AddImports, SemanticImports,
};
use tcl_runtime_api::codegen_abi::{CodegenAbiImportId, CodegenAbiValueType};

const fn abi_value_type(value: CodegenAbiValueType) -> ValType {
    match value {
        CodegenAbiValueType::I32 => ValType::I32,
        CodegenAbiValueType::I64 => ValType::I64,
        CodegenAbiValueType::F64 => ValType::F64,
    }
}

pub(super) fn add_codegen_import(module: &mut WasmModule, import: CodegenAbiImportId) -> u32 {
    let descriptor = import.descriptor();
    let parameters = descriptor
        .parameters
        .iter()
        .copied()
        .map(abi_value_type)
        .collect::<Vec<_>>();
    let results = descriptor
        .results
        .iter()
        .copied()
        .map(abi_value_type)
        .collect::<Vec<_>>();
    u32::try_from(module.add_import(descriptor.module, descriptor.name, &parameters, &results))
        .expect("WASM import index fits u32")
}

pub(super) fn general_import_id(imports: Imports, index: u32) -> Option<CodegenAbiImportId> {
    let mut entries = vec![
        (
            imports.host_refusal_pending,
            CodegenAbiImportId::HostRefusalPending,
        ),
        (imports.obj_new_string, CodegenAbiImportId::ObjectNewString),
        (imports.eval_code, CodegenAbiImportId::EvalCode),
        (
            imports.condition.frame_alloc,
            CodegenAbiImportId::CallFrameAlloc,
        ),
        (
            imports.condition.frame_free,
            CodegenAbiImportId::CallFrameFree,
        ),
        (
            imports.condition.string_owned,
            CodegenAbiImportId::NewOwnedString,
        ),
        (
            imports.condition.expr_bool_eval,
            CodegenAbiImportId::ExprBoolEval,
        ),
        (
            imports.condition.completion_release,
            CodegenAbiImportId::CompletionRelease,
        ),
        (
            imports.condition.object_release,
            CodegenAbiImportId::ObjectRelease,
        ),
    ];
    if let Some(aot) = imports.aot {
        entries.push((aot.value_new_string, CodegenAbiImportId::ValueNewString));
        entries.push((aot.frame_push, CodegenAbiImportId::FramePush));
        entries.push((aot.frame_pop, CodegenAbiImportId::FramePop));
        entries.push((aot.local_bind, CodegenAbiImportId::LocalBind));
        entries.push((aot.local_set, CodegenAbiImportId::LocalSet));
        entries.push((aot.local_get, CodegenAbiImportId::LocalGet));
        entries.push((aot.var_set, CodegenAbiImportId::VarSet));
        entries.push((aot.var_get, CodegenAbiImportId::VarGet));
        entries.push((aot.expr_add, CodegenAbiImportId::ExprAdd));
        entries.push((aot.puts, CodegenAbiImportId::Puts));
        entries.push((aot.argv.frame_alloc, CodegenAbiImportId::CallFrameAlloc));
        entries.push((aot.argv.frame_free, CodegenAbiImportId::CallFrameFree));
        entries.push((aot.argv.string_owned, CodegenAbiImportId::NewOwnedString));
        entries.push((aot.argv.invoke_argv, CodegenAbiImportId::InvokeArgv));
        entries.push((aot.argv.object_release, CodegenAbiImportId::ObjectRelease));
        entries.push((aot.argv.var_get, CodegenAbiImportId::VarGet));
        entries.push((aot.argv.var_get_element, CodegenAbiImportId::VarGetElement));
        entries.push((aot.argv.word_concat, CodegenAbiImportId::WordConcat));
    }
    entries
        .into_iter()
        .find_map(|(candidate, id)| (candidate == index).then_some(id))
}

pub(super) fn add_semantic_imports(wasm: &mut WasmModule) -> SemanticImports {
    SemanticImports {
        host_refusal_pending: add_codegen_import(wasm, CodegenAbiImportId::HostRefusalPending),
        frame_alloc: add_codegen_import(wasm, CodegenAbiImportId::CallFrameAlloc),
        frame_free: add_codegen_import(wasm, CodegenAbiImportId::CallFrameFree),
        string_owned: add_codegen_import(wasm, CodegenAbiImportId::NewOwnedString),
        invoke_argv: add_codegen_import(wasm, CodegenAbiImportId::InvokeArgv),
        completion_release: add_codegen_import(wasm, CodegenAbiImportId::CompletionRelease),
        object_retain: add_codegen_import(wasm, CodegenAbiImportId::ObjectRetain),
        object_release: add_codegen_import(wasm, CodegenAbiImportId::ObjectRelease),
    }
}

pub(super) fn add_guarded_intrinsic_imports(wasm: &mut WasmModule) -> GuardedIntrinsicImports {
    GuardedIntrinsicImports {
        semantic: add_semantic_imports(wasm),
        guard_prepare: add_codegen_import(wasm, CodegenAbiImportId::GuardPrepare),
        guard_check: add_codegen_import(wasm, CodegenAbiImportId::GuardCheck),
        guard_release: add_codegen_import(wasm, CodegenAbiImportId::GuardRelease),
        invoke_intrinsic_argv: add_codegen_import(wasm, CodegenAbiImportId::InvokeIntrinsicArgv),
    }
}

pub(super) fn add_native_i64_add_imports(wasm: &mut WasmModule) -> NativeI64AddImports {
    NativeI64AddImports {
        host_refusal_pending: add_codegen_import(wasm, CodegenAbiImportId::HostRefusalPending),
        value_new_wide_int: add_codegen_import(wasm, CodegenAbiImportId::ValueNewWideInt),
        puts: add_codegen_import(wasm, CodegenAbiImportId::Puts),
    }
}

pub(super) fn add_general_imports(wasm: &mut WasmModule, analysis: bool) -> Imports {
    let mut imports = Imports {
        host_refusal_pending: add_codegen_import(wasm, CodegenAbiImportId::HostRefusalPending),
        obj_new_string: add_codegen_import(wasm, CodegenAbiImportId::ObjectNewString),
        eval_code: add_codegen_import(wasm, CodegenAbiImportId::EvalCode),
        condition: ConditionImports {
            frame_alloc: add_codegen_import(wasm, CodegenAbiImportId::CallFrameAlloc),
            frame_free: add_codegen_import(wasm, CodegenAbiImportId::CallFrameFree),
            string_owned: add_codegen_import(wasm, CodegenAbiImportId::NewOwnedString),
            expr_bool_eval: add_codegen_import(wasm, CodegenAbiImportId::ExprBoolEval),
            completion_release: add_codegen_import(wasm, CodegenAbiImportId::CompletionRelease),
            object_release: add_codegen_import(wasm, CodegenAbiImportId::ObjectRelease),
        },
        aot: None,
    };
    if analysis {
        imports.aot = Some(add_aot_imports(wasm));
    }
    imports
}

/// The prebuilt-argv surface the general tier's normal leaf-command path uses.
///
/// `var_get` is shared with the direct scalar-read specialisation; everything
/// else is the shared code-generation ABI's own transport surface.
fn add_argv_imports(wasm: &mut WasmModule, var_get: u32) -> ArgvImports {
    ArgvImports {
        frame_alloc: add_codegen_import(wasm, CodegenAbiImportId::CallFrameAlloc),
        frame_free: add_codegen_import(wasm, CodegenAbiImportId::CallFrameFree),
        string_owned: add_codegen_import(wasm, CodegenAbiImportId::NewOwnedString),
        invoke_argv: add_codegen_import(wasm, CodegenAbiImportId::InvokeArgv),
        object_release: add_codegen_import(wasm, CodegenAbiImportId::ObjectRelease),
        var_get,
        var_get_element: add_codegen_import(wasm, CodegenAbiImportId::VarGetElement),
        word_concat: add_codegen_import(wasm, CodegenAbiImportId::WordConcat),
    }
}

fn add_aot_imports(wasm: &mut WasmModule) -> AotImports {
    let mut imports = AotImports {
        value_new_string: add_codegen_import(wasm, CodegenAbiImportId::ValueNewString),
        frame_push: add_codegen_import(wasm, CodegenAbiImportId::FramePush),
        frame_pop: add_codegen_import(wasm, CodegenAbiImportId::FramePop),
        local_bind: add_codegen_import(wasm, CodegenAbiImportId::LocalBind),
        local_set: add_codegen_import(wasm, CodegenAbiImportId::LocalSet),
        local_get: add_codegen_import(wasm, CodegenAbiImportId::LocalGet),
        var_set: add_codegen_import(wasm, CodegenAbiImportId::VarSet),
        var_get: add_codegen_import(wasm, CodegenAbiImportId::VarGet),
        expr_add: add_codegen_import(wasm, CodegenAbiImportId::ExprAdd),
        puts: add_codegen_import(wasm, CodegenAbiImportId::Puts),
        argv: ArgvImports::default(),
    };
    imports.argv = add_argv_imports(wasm, imports.var_get);
    imports
}
