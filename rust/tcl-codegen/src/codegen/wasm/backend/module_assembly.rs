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

//! Assembling the module: native-tier lowering, the function table install, the runtime chunk, the special modes and the `_start` entry.

use super::super::encoding::{leb128_signed, leb128_unsigned};
use super::super::ir::{
    GlobalInit, ValType, WasmData, WasmFunction, WasmGlobal, WasmInstruction, WasmModule, WasmOp,
};
use super::super::native_emit;
use super::super::native_emit::{ConstantPool, NativeImports};
use super::super::pipeline::WasmNativeI64AddSelection;
use super::function_facts::{ProcedurePlan, function_facts, procedure_plan, span_key};
use super::imports::{
    add_codegen_import, add_general_imports, add_guarded_intrinsic_imports,
    add_native_i64_add_imports, add_semantic_imports,
};
use super::{
    BLOCK_VOID, EmitterImports, FunctionFacts, FunctionMode, WasmEmissionMode, WasmEmitter,
    owned_function_names,
};
use crate::codegen::emit::Emit;
use crate::codegen::structured;
use std::collections::{BTreeMap, HashMap, HashSet};
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::ir::Module;
use tcl_compiler::native_lowering::{
    FunctionDecline, FunctionReport, LoweringInput, NativeBinding, NativeTierReport,
    ProcEntryDecline, lower_function,
};
use tcl_compiler::semantic_optimisation::SemanticOptimisationConfig;
use tcl_registry::CommandRegistry;
use tcl_runtime_api::codegen_abi::CodegenAbiImportId;

/// The native tier's inputs, present only when the pipeline selected it.
#[derive(Clone, Copy)]
pub(super) struct NativeTier<'a> {
    pub(super) unit: &'a CompilationUnit,
    pub(super) registry: &'a CommandRegistry,
    pub(super) config: SemanticOptimisationConfig,
}

impl NativeTier<'_> {
    /// Lower one function unit through NLIR, or the typed reason it stays on
    /// the legacy structured path.
    fn lower(
        &self,
        function: &FunctionUnit,
        top_level: bool,
        line_origin: u32,
    ) -> Result<
        (
            tcl_compiler::native_lowering::ir::NativeFunction,
            FunctionReport,
        ),
        FunctionDecline,
    > {
        let facts = &function.semantic_facts;
        let executable = facts
            .executable()
            .function()
            .ok_or(FunctionDecline::NoExecutableFunction)?;
        let hints = std::collections::BTreeMap::new();
        let input = LoweringInput {
            registry: self.registry,
            context: facts.context(),
            function: executable,
            source: &self.unit.source,
            module: &self.unit.ir_module,
            mutations: &self.unit.command_mutations,
            config: self.config,
            escape: None,
            top_level,
            line_origin,
            entry_assumption: facts.dispatch_entry_assumption(),
            type_hints: &hints,
        };
        lower_function(&input)
    }
}

/// One procedure body's native lowering, held until the module's table window
/// is known: `::top` cannot be emitted before the binding set is decided, and
/// a body is only worth lowering once.
enum LoweredProc {
    /// The body lowered; `decline` says why it may nonetheless not be bound
    /// as the procedure's entry.
    Lowered {
        function: tcl_compiler::native_lowering::ir::NativeFunction,
        report: FunctionReport,
        decline: Option<ProcEntryDecline>,
    },
    /// The tier declined the body outright.
    Declined(FunctionDecline),
}

/// Every procedure a lowered function defines through the native definition
/// shape, which is the exact set that can read a table entry.
fn collect_defined_procs(
    function: &tcl_compiler::native_lowering::ir::NativeFunction,
    out: &mut HashSet<tcl_compiler::ir::ProcedureId>,
) {
    fn walk(
        ops: &[tcl_compiler::native_lowering::ir::NativeOp],
        out: &mut HashSet<tcl_compiler::ir::ProcedureId>,
    ) {
        use tcl_compiler::native_lowering::ir::NativeOp;
        for op in ops {
            match op {
                NativeOp::DefineProc { entry_key, .. } => {
                    out.insert(*entry_key);
                }
                NativeOp::IfElse {
                    then_ops, else_ops, ..
                } => {
                    walk(then_ops, out);
                    walk(else_ops, out);
                }
                _ => {}
            }
        }
    }
    for block in &function.blocks {
        for statement in &block.statements {
            walk(&statement.ops, out);
        }
    }
}

/// `::top`'s table-install prologue: grow the runtime's shared function table
/// by this module's entry count, keep the base in the module's `$table_base`
/// global, and write one `ref.func` per bound procedure.
///
/// Guarded on the global, so a `::top` that runs twice — a host calling it
/// after `_start` already did — installs once and keeps the same indices. A
/// `table.grow` answering `-1` is a runtime linked without
/// `--growable-table`: the module traps at its entry point with a clear
/// backtrace rather than dispatching to whatever sits at slot 0.
fn table_install(
    bound: &BTreeMap<tcl_compiler::ir::ProcedureId, u32>,
    indices: &HashMap<tcl_compiler::ir::ProcedureId, u32>,
    base_global: u32,
) -> Vec<WasmInstruction> {
    if bound.is_empty() {
        return Vec::new();
    }
    let global =
        |op: WasmOp| WasmInstruction::with_operands(op, leb128_unsigned(u64::from(base_global)));
    let constant =
        |value: i64| WasmInstruction::with_operands(WasmOp::I32Const, leb128_signed(value));
    let mut ops = vec![
        global(WasmOp::GlobalGet),
        constant(0),
        WasmInstruction::new(WasmOp::I32LtS),
        WasmInstruction::with_operands(WasmOp::If, vec![BLOCK_VOID]),
        WasmInstruction::ref_null_func(),
        constant(i64::try_from(bound.len()).unwrap_or(i64::MAX)),
        WasmInstruction::table_grow(0),
        global(WasmOp::GlobalSet),
        global(WasmOp::GlobalGet),
        constant(0),
        WasmInstruction::new(WasmOp::I32LtS),
        WasmInstruction::with_operands(WasmOp::If, vec![BLOCK_VOID]),
        WasmInstruction::new(WasmOp::Unreachable),
        WasmInstruction::new(WasmOp::End),
    ];
    for (name, slot) in bound {
        let Some(index) = indices.get(name).copied() else {
            continue;
        };
        ops.push(global(WasmOp::GlobalGet));
        ops.push(constant(i64::from(*slot)));
        ops.push(WasmInstruction::new(WasmOp::I32Add));
        ops.push(WasmInstruction::ref_func(index));
        ops.push(WasmInstruction::table_set(0));
    }
    ops.push(WasmInstruction::new(WasmOp::End));
    ops
}

/// Host admission executes the whole source before any compiled effects,
/// exports or procedure-table installation can bypass its compiler entry.
pub(super) fn codegen_runtime_chunk(
    chunk: Option<&tcl_names::origin::ExecutedScriptSource>,
    data_base: i64,
    standalone: bool,
    init: bool,
) -> (WasmModule, NativeTierReport) {
    let mut wasm = WasmModule::new();
    let imports = add_general_imports(&mut wasm, false);
    let bootstrap = standalone.then(|| {
        let create = add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeCreateInterp);
        let set_current =
            add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeSetCurrentInterp);
        let library =
            init.then(|| add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeInitLibrary));
        (create, set_current, library)
    });
    let top_index = u32::try_from(wasm.imports.len()).expect("import count fits u32");
    let mut emitter = WasmEmitter {
        imports: EmitterImports::General(imports),
        body: Vec::new(),
        data: Vec::new(),
        data_offset: data_base,
        ctrl_depth: 0,
        invocation_abort: None,
        loops: Vec::new(),
        code_local: 0,
        frame_local: 1,
        mode: FunctionMode::Top,
        local_slots: HashMap::new(),
        proc_indices: HashMap::new(),
        procedure_arity: HashMap::new(),
        direct_procs: HashSet::new(),
        facts: FunctionFacts::default(),
        retained_i64: None,
        retained_i64_import: None,
    };
    if let Some(chunk) = chunk {
        wasm.source_function_ids.insert(
            top_index,
            tcl_compiler::native_lowering::NativeFunctionId::TopLevel,
        );
        emitter.emit_command_image(&chunk.text);
    } else {
        emitter.refuse_native_compilation_admission();
    }
    wasm.functions
        .push(emitter.finish_function("::top", "host-compilation-entry", None));
    wasm.data_segments = emitter.data;
    if let Some((create, set_current, library)) = bootstrap {
        wasm.functions
            .push(start_function(create, set_current, library, top_index));
    }
    let mut report = NativeTierReport::default();
    report.record(
        tcl_compiler::native_lowering::NativeFunctionId::TopLevel,
        "::top",
        FunctionReport::declined(FunctionDecline::NativeCompilationAdmissionRequired),
    );
    (wasm, report)
}

/// Shared implementation for hosted, linked, and standalone packaging.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn codegen(
    module: &Module,
    source: &str,
    data_base: i64,
    standalone: bool,
    init: bool,
    analysis: Option<(&CompilationUnit, &CommandRegistry)>,
    native: Option<NativeTier<'_>>,
    mode: WasmEmissionMode<'_>,
) -> (WasmModule, NativeTierReport) {
    if let WasmEmissionMode::RuntimeChunk(chunk) = mode {
        return codegen_runtime_chunk(chunk, data_base, standalone, init);
    }
    let mut wasm = WasmModule::new();
    let mut report = NativeTierReport {
        enabled: native.is_some(),
        functions: std::collections::BTreeMap::new(),
    };
    if emit_special_mode(&mut wasm, data_base, mode) {
        return (
            owned_function_names::finish_owned_function_names(wasm),
            report,
        );
    }
    let imports = add_general_imports(&mut wasm, analysis.is_some());
    let retained_i64 = match mode {
        WasmEmissionMode::RetainedNativeI64Add(publication) => Some(publication.clone()),
        _ => None,
    };
    let retained_i64_import = retained_i64
        .as_ref()
        .map(|_| add_codegen_import(&mut wasm, CodegenAbiImportId::ProcDefineGuardedI64AddArgv));
    let native_imports: Option<NativeImports> =
        native.map(|_| native_emit::add_native_imports(&mut wasm, &mut add_codegen_import));

    // Standalone: the interp-bootstrap imports `_start` drives. Added after the
    // ABI imports so the ABI indices in `Imports` are unchanged.
    let bootstrap = standalone.then(|| {
        let create = add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeCreateInterp);
        let set_current =
            add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeSetCurrentInterp);
        let init_library =
            init.then(|| add_codegen_import(&mut wasm, CodegenAbiImportId::RuntimeInitLibrary));
        (create, set_current, init_library)
    });

    // `::top` is the first *defined* function, so its call index is the import
    // count (imports occupy the low indices). Capture it before emitting bodies.
    let top_idx = u32::try_from(wasm.imports.len()).expect("import count fits in u32");

    let ProcedurePlan {
        procs,
        indices: proc_indices,
        arity: procedure_arity,
        direct: mut direct_procs,
    } = procedure_plan(module, analysis, top_idx);
    let mut top_facts = analysis.map_or_else(FunctionFacts::default, |(unit, registry)| {
        function_facts(
            &unit.top_level,
            module,
            registry,
            &unit.command_mutations,
            true,
        )
    });

    if let Some(candidate) = &retained_i64 {
        direct_procs.clear();
        top_facts.direct_assignments.clear();
        top_facts
            .leaf_invocations
            .insert(span_key(candidate.span), candidate.word_plan.clone());
    }
    let mut emitter = WasmEmitter {
        imports: EmitterImports::General(imports),
        body: Vec::new(),
        data: Vec::new(),
        data_offset: data_base,
        ctrl_depth: 0,
        invocation_abort: None,
        loops: Vec::new(),
        code_local: 0,
        frame_local: 1,
        mode: FunctionMode::Top,
        local_slots: HashMap::new(),
        proc_indices,
        procedure_arity,
        direct_procs,
        facts: top_facts,
        retained_i64,
        retained_i64_import,
    };
    // Every procedure body is lowered *before* `::top` is emitted: which of
    // them lowered decides the module's window in the runtime's shared
    // function table, and `::top` both grows that window and binds each entry
    // to its definition, so it cannot be emitted without knowing.
    let lowered_top = native.map(|tier| tier.lower(&tier.unit.top_level, true, 0));
    let lowered_procs: Vec<Option<LoweredProc>> = procs
        .iter()
        .map(|(id, proc)| {
            let tier = native?;
            let unit = tier.unit.function_for_body(Some(*id))?;
            Some(match tier.lower(unit, false, proc.body_offset) {
                Ok((function, report)) => LoweredProc::Lowered {
                    decline: native_emit::proc_entry_decline(&function),
                    function,
                    report,
                },
                Err(reason) => LoweredProc::Declined(reason),
            })
        })
        .collect();
    // Only a definition statement the lowering actually took natively can
    // carry an entry, and the table window must hold exactly those: a slot
    // nothing binds would import the runtime's table for nothing.
    let mut defined: HashSet<tcl_compiler::ir::ProcedureId> = HashSet::new();
    if let Some(Ok((function, _))) = &lowered_top {
        collect_defined_procs(function, &mut defined);
    }
    for lowered in &lowered_procs {
        if let Some(LoweredProc::Lowered { function, .. }) = lowered {
            collect_defined_procs(function, &mut defined);
        }
    }
    // A binding is only reachable through `::top`'s install sequence, so a
    // module whose entry point stayed on the legacy path installs nothing and
    // every definition it emits is source-only.
    let bound: BTreeMap<tcl_compiler::ir::ProcedureId, u32> = if matches!(lowered_top, Some(Ok(_)))
    {
        procs
            .iter()
            .zip(&lowered_procs)
            .filter(|((id, _), lowered)| {
                matches!(lowered, Some(LoweredProc::Lowered { decline: None, .. }))
                    && defined.contains(id)
            })
            .enumerate()
            .map(|(slot, ((id, _), _))| (*id, u32::try_from(slot).unwrap_or(u32::MAX)))
            .collect()
    } else {
        BTreeMap::new()
    };
    // The table import is opt-in per module: a runtime linked without
    // `--export-table` cannot satisfy it at all, so a module that installs
    // nothing must not ask for it.
    let table_base_global = u32::try_from(wasm.globals.len()).unwrap_or(u32::MAX);
    if !bound.is_empty() {
        wasm.import_table = true;
        wasm.globals.push(WasmGlobal {
            name: "$table_base".to_owned(),
            mutable: true,
            init: GlobalInit::I32(-1),
        });
        wasm.elem_declared = bound
            .keys()
            .filter_map(|name| emitter.proc_indices.get(name).copied())
            .collect();
        wasm.elem_declared.sort_unstable();
    }
    let table = native_emit::EntryTable {
        slots: &bound,
        base_global: table_base_global,
    };

    // The top-level script: the native tier when it lowers, else the legacy
    // structured walk with the typed reason recorded.
    match (lowered_top, native_imports) {
        (Some(Ok((function, function_report))), Some(native_imports)) => {
            report.record(
                tcl_compiler::native_lowering::NativeFunctionId::TopLevel,
                "::top",
                function_report,
            );
            let mut top = native_emit::emit_function(
                "::top",
                "native-top",
                &function,
                native_imports,
                table,
                ConstantPool {
                    data: &mut emitter.data,
                    offset: &mut emitter.data_offset,
                },
            );
            let install = table_install(&bound, &emitter.proc_indices, table_base_global);
            top.body.splice(0..0, install);
            wasm.source_function_ids.insert(
                top_idx,
                tcl_compiler::native_lowering::NativeFunctionId::TopLevel,
            );
            wasm.functions.push(top);
        }
        (lowered, _) => {
            if let Some(Err(reason)) = lowered {
                report.record(
                    tcl_compiler::native_lowering::NativeFunctionId::TopLevel,
                    "::top",
                    FunctionReport::declined(reason),
                );
            }
            structured::walk(
                &mut emitter,
                &module.top_level,
                source,
                &module.source,
                module.lexer_config,
            );
            let top = emitter.finish_function("::top", "top", None);
            wasm.source_function_ids.insert(
                top_idx,
                tcl_compiler::native_lowering::NativeFunctionId::TopLevel,
            );
            wasm.functions.push(top);
        }
    }

    // Each user-defined proc body becomes its own WASM function, driven through
    // the same structured walk (its body is already lowered IR with absolute
    // source spans). Namespace-scoped procs are created at run time inside
    // `namespace eval`, not at load, so they are skipped — mirroring the bytecode
    // backend (`codegen/emitter/mod.rs`). Emitted in qualified-name order so the
    // module bytes are deterministic (`procedures` is a hash map).
    for ((id, proc), lowered) in procs.iter().zip(lowered_procs) {
        match (lowered, native_imports) {
            (
                Some(LoweredProc::Lowered {
                    function,
                    report: mut function_report,
                    decline,
                }),
                Some(native_imports),
            ) => {
                function_report.binding = match (decline, bound.contains_key(id)) {
                    (None, true) => NativeBinding::BoundNatively,
                    (Some(reason), _) => NativeBinding::SourceOnly(reason),
                    (None, false) => NativeBinding::NotApplicable,
                };
                report.record(
                    tcl_compiler::native_lowering::NativeFunctionId::Procedure(*id),
                    &proc.qualified_name,
                    function_report,
                );
                let func = native_emit::emit_function(
                    &proc.qualified_name,
                    "native-proc",
                    &function,
                    native_imports,
                    table,
                    ConstantPool {
                        data: &mut emitter.data,
                        offset: &mut emitter.data_offset,
                    },
                );
                wasm.source_function_ids.insert(
                    emitter.proc_indices[id],
                    tcl_compiler::native_lowering::NativeFunctionId::Procedure(*id),
                );
                wasm.functions.push(func);
                continue;
            }
            (Some(LoweredProc::Declined(reason)), _) => {
                report.record(
                    tcl_compiler::native_lowering::NativeFunctionId::Procedure(*id),
                    &proc.qualified_name,
                    FunctionReport::declined(reason),
                );
            }
            _ => {}
        }
        let direct = emitter.direct_procs.contains(id);
        emitter.mode = if direct {
            FunctionMode::DirectProc
        } else {
            FunctionMode::FallbackProc
        };
        emitter.local_slots = if direct {
            proc.params
                .iter()
                .enumerate()
                .map(|(slot, name)| (name.clone(), u32::try_from(slot).unwrap_or(u32::MAX)))
                .collect()
        } else {
            HashMap::new()
        };
        emitter.code_local = if direct {
            u64::try_from(proc.params.len()).unwrap_or(u64::MAX)
        } else {
            0
        };
        emitter.frame_local = emitter.code_local.saturating_add(1);
        emitter.facts = analysis
            .and_then(|(unit, registry)| {
                unit.function_for_body(Some(*id))
                    .map(|fu| function_facts(fu, module, registry, &unit.command_mutations, false))
            })
            .unwrap_or_default();
        if direct {
            emitter.emit_proc_prelude(proc);
        }
        structured::walk(
            &mut emitter,
            &proc.body,
            source,
            &module.source,
            module.lexer_config,
        );
        let func = emitter.finish_function(&proc.qualified_name, "proc", Some(proc));
        wasm.source_function_ids.insert(
            emitter.proc_indices[id],
            tcl_compiler::native_lowering::NativeFunctionId::Procedure(*id),
        );
        wasm.functions.push(func);
    }

    wasm.data_segments = emitter.data;

    if let Some((create, set_current, init_library)) = bootstrap {
        wasm.functions
            .push(start_function(create, set_current, init_library, top_idx));
    }

    (
        owned_function_names::finish_owned_function_names(wasm),
        report,
    )
}

fn emit_special_mode(wasm: &mut WasmModule, data_base: i64, mode: WasmEmissionMode<'_>) -> bool {
    match mode {
        WasmEmissionMode::NativeI64Add(plan) => {
            emit_native_i64_add(wasm, data_base, plan);
        }
        WasmEmissionMode::SemanticInvoke(plan) => {
            let imports = add_semantic_imports(wasm);
            let mut emitter = WasmEmitter::for_semantic_invoke(imports, data_base);
            wasm.source_function_ids.insert(
                u32::try_from(wasm.imports.len()).expect("import count fits u32"),
                tcl_compiler::native_lowering::NativeFunctionId::TopLevel,
            );
            wasm.functions.push(emitter.finish_semantic_invoke(plan));
            wasm.memory_pages = required_pages(data_base, &emitter.data);
            wasm.data_segments = emitter.data;
        }
        WasmEmissionMode::GuardedIntrinsic { plan, evidence } => {
            let imports = add_guarded_intrinsic_imports(wasm);
            let mut emitter = WasmEmitter::for_guarded_intrinsic_invoke(imports, data_base);
            wasm.source_function_ids.insert(
                u32::try_from(wasm.imports.len()).expect("import count fits u32"),
                tcl_compiler::native_lowering::NativeFunctionId::TopLevel,
            );
            wasm.functions
                .push(emitter.finish_guarded_intrinsic_invoke(plan, evidence));
            wasm.memory_pages = required_pages(data_base, &emitter.data);
            wasm.data_segments = emitter.data;
        }
        WasmEmissionMode::General
        | WasmEmissionMode::RuntimeChunk(_)
        | WasmEmissionMode::RetainedNativeI64Add(_) => return false,
    }
    true
}

fn emit_native_i64_add(wasm: &mut WasmModule, data_base: i64, plan: &WasmNativeI64AddSelection) {
    let imports = add_native_i64_add_imports(wasm);
    let top_index = u32::try_from(wasm.imports.len()).expect("import count fits in u32");
    let add_index = top_index
        .checked_add(1)
        .expect("native function index fits u32");
    wasm.source_function_ids.insert(
        top_index,
        tcl_compiler::native_lowering::NativeFunctionId::TopLevel,
    );
    wasm.source_function_ids.insert(
        add_index,
        tcl_compiler::native_lowering::NativeFunctionId::Procedure(plan.callee.id),
    );
    wasm.functions.push(WasmFunction {
        name: "::top".to_owned(),
        params: Vec::new(),
        results: Vec::new(),
        locals: vec![ValType::I32],
        body: vec![
            WasmInstruction::with_operands(WasmOp::I64Const, leb128_signed(plan.left)),
            WasmInstruction::with_operands(WasmOp::I64Const, leb128_signed(plan.right)),
            WasmInstruction::with_operands(WasmOp::Call, leb128_unsigned(u64::from(add_index))),
            WasmInstruction::with_operands(
                WasmOp::Call,
                leb128_unsigned(u64::from(imports.value_new_wide_int)),
            ),
            WasmInstruction::with_operands(WasmOp::Call, leb128_unsigned(u64::from(imports.puts))),
            // Match structured lowering: a non-OK `puts` completion stops
            // the top-level script instead of being silently discarded.
            WasmInstruction::with_operands(WasmOp::LocalSet, leb128_unsigned(0)),
            WasmInstruction::with_operands(
                WasmOp::Call,
                leb128_unsigned(u64::from(imports.host_refusal_pending)),
            ),
            WasmInstruction::with_operands(WasmOp::If, vec![BLOCK_VOID]),
            WasmInstruction::new(WasmOp::Return),
            WasmInstruction::new(WasmOp::End),
            WasmInstruction::with_operands(WasmOp::LocalGet, leb128_unsigned(0)),
            WasmInstruction::with_operands(WasmOp::If, vec![BLOCK_VOID]),
            WasmInstruction::new(WasmOp::Return),
            WasmInstruction::new(WasmOp::End),
            WasmInstruction::new(WasmOp::Return),
        ],
        local_names: vec!["$completion_code".to_owned()],
        exported: true,
        source_range: None,
        kind: "native-i64-add-top".to_owned(),
    });
    wasm.functions.push(WasmFunction {
        name: plan.callee.qualified_name.clone(),
        params: vec![ValType::I64, ValType::I64],
        results: vec![ValType::I64],
        locals: Vec::new(),
        body: vec![
            WasmInstruction::with_operands(WasmOp::LocalGet, leb128_unsigned(0)),
            WasmInstruction::with_operands(WasmOp::LocalGet, leb128_unsigned(1)),
            WasmInstruction::new(WasmOp::I64Add),
            WasmInstruction::new(WasmOp::Return),
        ],
        local_names: vec!["$left".to_owned(), "$right".to_owned()],
        // The raw i64 function is an implementation detail of the closed
        // proof. Only the boxed Tcl entry boundary is externally callable.
        exported: false,
        source_range: None,
        kind: "native-i64-add-proc".to_owned(),
    });
    wasm.memory_pages = required_pages(data_base, &[]);
}

fn required_pages(data_base: i64, data: &[WasmData]) -> u64 {
    let end = data.iter().fold(data_base, |largest, segment| {
        let segment_end = segment
            .offset
            .checked_add(i64::try_from(segment.data.len()).expect("literal length fits i64"))
            .expect("semantic plan validated constant-pool bounds");
        largest.max(segment_end)
    });
    let pages = end
        .checked_add(65_535)
        .expect("semantic plan validated page calculation")
        / 65_536;
    u64::try_from(pages.max(1)).expect("validated wasm32 page count")
}

/// The `_start` WASI-command entry:
/// `set_current_interp(create_interp()); [init_library();] ::top()`.
/// `finish_function`'s usual trailing `end` is appended here by hand since this
/// body is built directly rather than via the structured walk.
pub(super) fn start_function(
    create_interp: u32,
    set_current_interp: u32,
    init_library: Option<u32>,
    top_idx: u32,
) -> WasmFunction {
    let call =
        |idx: u32| WasmInstruction::with_operands(WasmOp::Call, leb128_unsigned(u64::from(idx)));
    // create_interp() leaves the interp ptr on the stack; set_current_interp
    // consumes it; the optional init_library() bootstraps the stdlib (its i32
    // status is discarded); ::top runs against the now-current, initialised interp.
    let mut body = vec![call(create_interp), call(set_current_interp)];
    if let Some(init) = init_library {
        body.push(call(init));
        body.push(WasmInstruction::new(WasmOp::Drop));
    }
    body.push(call(top_idx));
    body.push(WasmInstruction::new(WasmOp::End));
    WasmFunction {
        name: "_start".to_string(),
        params: Vec::new(),
        results: Vec::new(),
        locals: Vec::new(),
        body,
        local_names: Vec::new(),
        exported: true,
        source_range: None,
        kind: "start".to_string(),
    }
}
