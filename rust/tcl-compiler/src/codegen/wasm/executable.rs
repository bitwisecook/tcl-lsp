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

//! WASM lowering for the executable IR's ordinary prebuilt-argv invocation.
//!
//! This deliberately handles only one validated linear invocation whose unique
//! `OK` spine contains literal values. Every word/build stage must retain an
//! abrupt `ReturnCompletion` side edge; the plan records why direct literal
//! materialisation makes that edge unreachable for this target. It never
//! reinterprets `original_words`, reparses command source, or turns a dynamic
//! word into a source string. The generated function has the portable completion transport signature
//! `(result i32 i32 i32)`: Tcl code, then independently owned result and
//! return-options handles. Its caller releases the latter two references.

use super::encoding::{leb128_signed, leb128_unsigned};
use super::ir::{ValType, WasmData, WasmFunction, WasmInstruction, WasmModule, WasmOp};
use crate::executable_ir::{
    ArgvEntry, CompletionId, ExecutableBlockId, ExecutableFunction, ExecutableInstruction,
    ExecutableTerminator, ExecutableValueId, GenericInvoke, InvocationResolution,
};
use crate::ir::WordExpr;
use tcl_registry::SemanticOperationId;
use tcl_runtime_api::codegen_abi::{
    CodegenAbiImportId, CodegenAbiValueType, WASM32_CODEGEN_DATA_END, WASM32_CODEGEN_DATA_START,
    WASM32_COMPLETION_ALIGN, WASM32_COMPLETION_CODE_OFFSET, WASM32_COMPLETION_OPTIONS_OFFSET,
    WASM32_COMPLETION_RESULT_OFFSET, WASM32_COMPLETION_SIZE, WASM32_POINTER_BYTES,
};

/// The lowerable, immutable content of one generic executable-IR invocation.
///
/// This is intentionally separate from emission. A backend selector can retain
/// the plan while deciding target legality, and the emitter only consumes the
/// already-proven argv literals and generic semantic operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmGenericInvokePlan {
    /// Stable exported name for this isolated generated function.
    pub function_name: String,
    /// The registry-selected semantic operation at the invocation site.
    ///
    /// The backend plan is still generic argv invocation: a structural or
    /// intrinsic operation is not lost merely because this target has not
    /// selected its specialised lowering.
    pub operation: SemanticOperationId,
    /// Tcl values for argv in the exact executable-IR evaluation order.
    pub argv_literals: Vec<String>,
    /// The completion produced by the generic invocation.
    pub completion: CompletionId,
    /// Explicit target proofs that make the staged `OK` spine straight-line.
    pub stage_proofs: Vec<WasmStageProof>,
}

/// A target proof used to erase a staged completion side edge without ignoring it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmStageProof {
    /// A literal or braced Tcl word is copied directly, so word evaluation has no Tcl completion.
    LiteralMaterialisation {
        /// Completion whose non-`OK` side exit is proven unreachable.
        completion: CompletionId,
    },
    /// The frame allocator either supplies storage or traps; it cannot yield a Tcl completion.
    FrameAssembly {
        /// Completion whose non-`OK` side exit is proven unreachable.
        completion: CompletionId,
    },
}

/// The shared-memory frame layout emitted for one [`WasmGenericInvokePlan`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WasmCallFrameLayout {
    /// Offset of the `argc` consecutive wasm32 object-handle slots.
    pub argv_offset: i32,
    /// Offset of the runtime-owned `TclCompletionAbi` output triple.
    pub completion_offset: i32,
    /// Exact allocation size passed to the runtime frame allocator.
    pub bytes: i32,
    /// Exact alignment passed to the runtime frame allocator.
    pub align: i32,
}

impl WasmCallFrameLayout {
    fn for_argc(argc: usize) -> Result<Self, WasmExecutableInvokeDecline> {
        let argc = i32::try_from(argc).map_err(|_| WasmExecutableInvokeDecline::TooManyWords)?;
        let argv_bytes = argc
            .checked_mul(WASM32_POINTER_BYTES)
            .ok_or(WasmExecutableInvokeDecline::FrameTooLarge)?;
        let bytes = argv_bytes
            .checked_add(WASM32_COMPLETION_SIZE)
            .ok_or(WasmExecutableInvokeDecline::FrameTooLarge)?;
        Ok(Self {
            argv_offset: 0,
            completion_offset: argv_bytes,
            bytes,
            align: WASM32_COMPLETION_ALIGN,
        })
    }
}

/// A precise reason the literal-only generic WASM transport declined a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmExecutableInvokeDecline {
    /// The executable IR failed its own dominance, ownership, or CFG checks.
    InvalidExecutableIr,
    /// The function does not have the bounded single-invocation staged CFG shape.
    UnsupportedControlFlow,
    /// An instruction other than literal evaluation, argv construction, and invoke occurred.
    UnsupportedInstruction,
    /// A word needs Tcl evaluation rather than literal materialisation.
    NonLiteralWord {
        /// Executable value whose source word requires runtime evaluation.
        value: ExecutableValueId,
    },
    /// `{*}` expansion requires Tcl list processing and is deliberately not source-replayed.
    ArgumentExpansion,
    /// The argv construction does not exactly use the preceding literal values.
    InconsistentArgv,
    /// The generic invocation does not consume that argv construction.
    InconsistentInvocation,
    /// The return terminator does not forward the invocation's full completion.
    InconsistentCompletionReturn,
    /// A literal cannot be represented by the wasm32 string-length ABI.
    LiteralTooLong,
    /// The argv exceeds the Tcl wasm32 ABI's signed argument count.
    TooManyWords,
    /// The transient shared-memory frame would exceed wasm32 addressing.
    FrameTooLarge,
    /// The caller selected an invalid data-region base.
    InvalidDataBase,
    /// The literal constant pool would escape the runtime-reserved data window.
    ConstantPoolOutOfBounds,
    /// The requested generated export name is empty.
    EmptyFunctionName,
}

impl WasmExecutableInvokeDecline {
    /// Stable code-generation evidence spelling for Explorer and API clients.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidExecutableIr => "invalid-executable-ir",
            Self::UnsupportedControlFlow => "unsupported-control-flow",
            Self::UnsupportedInstruction => "unsupported-instruction",
            Self::NonLiteralWord { .. } => "non-literal-word",
            Self::ArgumentExpansion => "argument-expansion",
            Self::InconsistentArgv => "inconsistent-argv",
            Self::InconsistentInvocation => "inconsistent-invocation",
            Self::InconsistentCompletionReturn => "inconsistent-completion-return",
            Self::LiteralTooLong => "literal-too-long",
            Self::TooManyWords => "too-many-words",
            Self::FrameTooLarge => "frame-too-large",
            Self::InvalidDataBase => "invalid-data-base",
            Self::ConstantPoolOutOfBounds => "constant-pool-out-of-bounds",
            Self::EmptyFunctionName => "empty-function-name",
        }
    }
}

/// Build a generic invocation plan with a caller-controlled stable export name.
pub fn plan_wasm_generic_invoke_named(
    function: &ExecutableFunction,
    function_name: String,
) -> Result<WasmGenericInvokePlan, WasmExecutableInvokeDecline> {
    if function_name.is_empty() {
        return Err(WasmExecutableInvokeDecline::EmptyFunctionName);
    }
    function
        .validate()
        .map_err(|_| WasmExecutableInvokeDecline::InvalidExecutableIr)?;
    let stages = staged_ok_spine(function)?;
    let Some(build_index) = stages
        .iter()
        .position(|stage| matches!(stage.instruction, ExecutableInstruction::BuildArgv { .. }))
    else {
        return Err(WasmExecutableInvokeDecline::UnsupportedInstruction);
    };
    let invoke_index = build_index + 1;
    if invoke_index >= stages.len() || invoke_index + 1 != stages.len() {
        return Err(WasmExecutableInvokeDecline::UnsupportedInstruction);
    }
    let evaluations = &stages[..build_index];
    if evaluations.iter().any(|stage| {
        !matches!(
            stage.instruction,
            ExecutableInstruction::EvaluateWord { .. }
        )
    }) {
        return Err(WasmExecutableInvokeDecline::UnsupportedInstruction);
    }
    let literals = evaluations
        .iter()
        .map(|stage| literal_value(stage.instruction))
        .collect::<Result<Vec<_>, _>>()?;
    let ExecutableInstruction::BuildArgv { argv, entries, .. } = stages[build_index].instruction
    else {
        return Err(WasmExecutableInvokeDecline::UnsupportedInstruction);
    };
    if entries.len() != literals.len()
        || !entries
            .iter()
            .zip(evaluations)
            .all(|(entry, evaluation)| matches_value(entry, evaluation.instruction))
    {
        return Err(WasmExecutableInvokeDecline::InconsistentArgv);
    }
    let ExecutableInstruction::Invoke(invoke) = stages[invoke_index].instruction else {
        return Err(WasmExecutableInvokeDecline::UnsupportedInstruction);
    };
    if invoke.argv != *argv {
        return Err(WasmExecutableInvokeDecline::InconsistentInvocation);
    }
    for literal in &literals {
        if i32::try_from(literal.len()).is_err() {
            return Err(WasmExecutableInvokeDecline::LiteralTooLong);
        }
    }
    WasmCallFrameLayout::for_argc(literals.len())?;
    let mut stage_proofs = Vec::with_capacity(build_index + 1);
    for stage in stages.iter().take(build_index + 1) {
        let proof = match stage.instruction {
            ExecutableInstruction::EvaluateWord { completion, .. } => {
                WasmStageProof::LiteralMaterialisation {
                    completion: *completion,
                }
            }
            ExecutableInstruction::BuildArgv { completion, .. } => WasmStageProof::FrameAssembly {
                completion: *completion,
            },
            ExecutableInstruction::ExpandWord { .. }
            | ExecutableInstruction::Invoke(_)
            | ExecutableInstruction::ExecuteLowered(_)
            | ExecutableInstruction::ExecuteOpaqueRegion(_) => {
                return Err(WasmExecutableInvokeDecline::UnsupportedInstruction);
            }
        };
        stage_proofs.push(proof);
    }
    Ok(WasmGenericInvokePlan {
        function_name,
        operation: invoke_operation(invoke),
        argv_literals: literals,
        completion: invoke.completion,
        stage_proofs,
    })
}

/// Emit an already-planned generic invocation at a caller-owned immutable data
/// base inside the shared wasm32 code-generation data window. Only literal
/// bytes are data segments; argv and completion scratch are dynamically
/// allocated through the runtime call-frame ABI.
pub fn emit_wasm_generic_invoke_at(
    plan: &WasmGenericInvokePlan,
    data_base: i64,
) -> Result<WasmModule, WasmExecutableInvokeDecline> {
    if !(WASM32_CODEGEN_DATA_START..WASM32_CODEGEN_DATA_END).contains(&data_base) {
        return Err(WasmExecutableInvokeDecline::InvalidDataBase);
    }
    let layout = WasmCallFrameLayout::for_argc(plan.argv_literals.len())?;
    let literals = data_segments(&plan.argv_literals, data_base)?;
    let mut wasm = WasmModule::new();
    wasm.memory_pages = required_pages(data_base, &literals)?;
    let imports = RuntimeImports::add_to(&mut wasm);
    wasm.functions
        .push(emit_function(plan, layout, &literals, imports));
    wasm.data_segments = literals;
    Ok(wasm)
}

struct StagedInstruction<'a> {
    instruction: &'a ExecutableInstruction,
}

fn staged_ok_spine(
    function: &ExecutableFunction,
) -> Result<Vec<StagedInstruction<'_>>, WasmExecutableInvokeDecline> {
    let mut current = function.entry;
    let mut stages = Vec::new();
    let mut visited = std::collections::BTreeSet::new();
    loop {
        if !visited.insert(current) {
            return Err(WasmExecutableInvokeDecline::UnsupportedControlFlow);
        }
        let block = function
            .blocks
            .get(current.index())
            .ok_or(WasmExecutableInvokeDecline::UnsupportedControlFlow)?;
        if block.id != current || block.instructions.len() != 1 {
            return Err(WasmExecutableInvokeDecline::UnsupportedControlFlow);
        }
        let instruction = &block.instructions[0];
        let completion = instruction_completion(instruction);
        stages.push(StagedInstruction { instruction });
        match instruction {
            ExecutableInstruction::Invoke(_) => {
                if block.terminator != Some(ExecutableTerminator::ReturnCompletion(completion)) {
                    return Err(WasmExecutableInvokeDecline::InconsistentCompletionReturn);
                }
                break;
            }
            ExecutableInstruction::EvaluateWord { .. }
            | ExecutableInstruction::BuildArgv { .. } => {
                current = require_ok_side_exit(function, block, completion, &mut visited)?;
            }
            ExecutableInstruction::ExpandWord { .. } => {
                return Err(WasmExecutableInvokeDecline::ArgumentExpansion);
            }
            ExecutableInstruction::ExecuteLowered(_)
            | ExecutableInstruction::ExecuteOpaqueRegion(_) => {
                return Err(WasmExecutableInvokeDecline::UnsupportedInstruction);
            }
        }
    }
    if visited.len() != function.blocks.len() {
        return Err(WasmExecutableInvokeDecline::UnsupportedControlFlow);
    }
    Ok(stages)
}

fn instruction_completion(instruction: &ExecutableInstruction) -> CompletionId {
    match instruction {
        ExecutableInstruction::EvaluateWord { completion, .. }
        | ExecutableInstruction::ExpandWord { completion, .. }
        | ExecutableInstruction::BuildArgv { completion, .. } => *completion,
        ExecutableInstruction::Invoke(invoke) => invoke.completion,
        ExecutableInstruction::ExecuteLowered(operation) => operation.completion,
        ExecutableInstruction::ExecuteOpaqueRegion(region) => region.completion,
    }
}

fn require_ok_side_exit(
    function: &ExecutableFunction,
    block: &crate::executable_ir::ExecutableBlock,
    completion: CompletionId,
    visited: &mut std::collections::BTreeSet<ExecutableBlockId>,
) -> Result<ExecutableBlockId, WasmExecutableInvokeDecline> {
    let Some(ExecutableTerminator::CompletionSwitch {
        completion: switched,
        cases,
        default,
    }) = &block.terminator
    else {
        return Err(WasmExecutableInvokeDecline::UnsupportedControlFlow);
    };
    if *switched != completion || cases.len() != 1 || !cases[0].code.is_ok() {
        return Err(WasmExecutableInvokeDecline::UnsupportedControlFlow);
    }
    let failure = function
        .blocks
        .get(default.index())
        .ok_or(WasmExecutableInvokeDecline::UnsupportedControlFlow)?;
    if failure.id != *default
        || !failure.instructions.is_empty()
        || failure.terminator != Some(ExecutableTerminator::ReturnCompletion(completion))
        || !visited.insert(*default)
    {
        return Err(WasmExecutableInvokeDecline::UnsupportedControlFlow);
    }
    Ok(cases[0].target)
}

fn literal_value(
    instruction: &ExecutableInstruction,
) -> Result<String, WasmExecutableInvokeDecline> {
    let ExecutableInstruction::EvaluateWord { value, word, .. } = instruction else {
        return Err(WasmExecutableInvokeDecline::UnsupportedInstruction);
    };
    match word {
        WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } => Ok(text.clone()),
        WordExpr::Expand { .. } => Err(WasmExecutableInvokeDecline::ArgumentExpansion),
        WordExpr::Variable { .. }
        | WordExpr::CommandSubstitution { .. }
        | WordExpr::Template { .. }
        | WordExpr::Opaque { .. } => {
            Err(WasmExecutableInvokeDecline::NonLiteralWord { value: *value })
        }
    }
}

fn matches_value(entry: &ArgvEntry, instruction: &ExecutableInstruction) -> bool {
    matches!(
        (entry, instruction),
        (ArgvEntry::Value(entry_value), ExecutableInstruction::EvaluateWord { value, .. })
            if entry_value == value
    )
}

fn invoke_operation(invoke: &GenericInvoke) -> SemanticOperationId {
    match &invoke.resolution {
        InvocationResolution::Resolved(facts) => facts.operation,
        InvocationResolution::Unresolved(_) => SemanticOperationId::Invoke,
    }
}

fn data_segments(
    words: &[String],
    data_base: i64,
) -> Result<Vec<WasmData>, WasmExecutableInvokeDecline> {
    let mut offset = data_base;
    let mut data = Vec::with_capacity(words.len());
    for word in words {
        let length =
            i64::try_from(word.len()).map_err(|_| WasmExecutableInvokeDecline::LiteralTooLong)?;
        let next = offset
            .checked_add(length)
            .ok_or(WasmExecutableInvokeDecline::ConstantPoolOutOfBounds)?;
        if next > WASM32_CODEGEN_DATA_END {
            return Err(WasmExecutableInvokeDecline::ConstantPoolOutOfBounds);
        }
        data.push(WasmData {
            offset,
            data: word.as_bytes().to_vec(),
        });
        offset = next;
    }
    Ok(data)
}

fn required_pages(data_base: i64, data: &[WasmData]) -> Result<u64, WasmExecutableInvokeDecline> {
    let end = data.iter().try_fold(data_base, |largest, segment| {
        let segment_end = segment
            .offset
            .checked_add(i64::try_from(segment.data.len()).ok()?)?;
        Some(largest.max(segment_end))
    });
    let Some(end) = end else {
        return Err(WasmExecutableInvokeDecline::InvalidDataBase);
    };
    let pages = end
        .checked_add(65_535)
        .ok_or(WasmExecutableInvokeDecline::InvalidDataBase)?
        / 65_536;
    u64::try_from(pages.max(1)).map_err(|_| WasmExecutableInvokeDecline::InvalidDataBase)
}

#[derive(Clone, Copy)]
struct RuntimeImports {
    frame_alloc: u32,
    frame_free: u32,
    string_owned: u32,
    invoke_argv: u32,
    completion_release: u32,
    object_retain: u32,
    object_release: u32,
}

impl RuntimeImports {
    fn add_to(module: &mut WasmModule) -> Self {
        let add = |module: &mut WasmModule, import: CodegenAbiImportId| {
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
            u32::try_from(module.add_import(
                descriptor.module,
                descriptor.name,
                &parameters,
                &results,
            ))
            .expect("WASM import index fits u32")
        };
        Self {
            frame_alloc: add(module, CodegenAbiImportId::CallFrameAlloc),
            frame_free: add(module, CodegenAbiImportId::CallFrameFree),
            string_owned: add(module, CodegenAbiImportId::NewOwnedString),
            invoke_argv: add(module, CodegenAbiImportId::InvokeArgv),
            completion_release: add(module, CodegenAbiImportId::CompletionRelease),
            object_retain: add(module, CodegenAbiImportId::ObjectRetain),
            object_release: add(module, CodegenAbiImportId::ObjectRelease),
        }
    }
}

const fn abi_value_type(value: CodegenAbiValueType) -> ValType {
    match value {
        CodegenAbiValueType::I32 => ValType::I32,
    }
}

fn emit_function(
    plan: &WasmGenericInvokePlan,
    layout: WasmCallFrameLayout,
    literals: &[WasmData],
    imports: RuntimeImports,
) -> WasmFunction {
    const FRAME_LOCAL: u64 = 0;
    const COMPLETION_LOCAL: u64 = 1;
    const CODE_LOCAL: u64 = 2;
    const RESULT_LOCAL: u64 = 3;
    const OPTIONS_LOCAL: u64 = 4;
    let word_local = |index: usize| u64::try_from(5 + index).expect("word local index fits u64");

    let mut body = Vec::new();
    push_i32(&mut body, layout.bytes);
    push_i32(&mut body, layout.align);
    call(&mut body, imports.frame_alloc);
    local_set(&mut body, FRAME_LOCAL);

    for (index, literal) in literals.iter().enumerate() {
        push_i32(
            &mut body,
            i32::try_from(literal.offset).expect("validated data offset"),
        );
        push_i32(
            &mut body,
            i32::try_from(literal.data.len()).expect("validated literal length"),
        );
        call(&mut body, imports.string_owned);
        local_set(&mut body, word_local(index));
        local_get(&mut body, FRAME_LOCAL);
        local_get(&mut body, word_local(index));
        store_i32(
            &mut body,
            i64::try_from(index * 4).expect("argv offset fits i64"),
        );
    }

    local_get(&mut body, FRAME_LOCAL);
    push_i32(
        &mut body,
        i32::try_from(plan.argv_literals.len()).expect("validated argc"),
    );
    local_get(&mut body, FRAME_LOCAL);
    push_i32(&mut body, layout.completion_offset);
    body.push(WasmInstruction::new(WasmOp::I32Add));
    local_tee(&mut body, COMPLETION_LOCAL);
    call(&mut body, imports.invoke_argv);
    body.push(WasmInstruction::new(WasmOp::Drop));

    local_get(&mut body, COMPLETION_LOCAL);
    load_i32(&mut body, i64::from(WASM32_COMPLETION_CODE_OFFSET));
    local_set(&mut body, CODE_LOCAL);
    local_get(&mut body, COMPLETION_LOCAL);
    load_i32(&mut body, i64::from(WASM32_COMPLETION_RESULT_OFFSET));
    call(&mut body, imports.object_retain);
    local_set(&mut body, RESULT_LOCAL);
    local_get(&mut body, COMPLETION_LOCAL);
    load_i32(&mut body, i64::from(WASM32_COMPLETION_OPTIONS_OFFSET));
    call(&mut body, imports.object_retain);
    local_set(&mut body, OPTIONS_LOCAL);
    call_with_local(&mut body, imports.completion_release, COMPLETION_LOCAL);

    for index in 0..literals.len() {
        call_with_local(&mut body, imports.object_release, word_local(index));
    }
    local_get(&mut body, FRAME_LOCAL);
    call(&mut body, imports.frame_free);
    body.push(WasmInstruction::new(WasmOp::Drop));

    local_get(&mut body, CODE_LOCAL);
    local_get(&mut body, RESULT_LOCAL);
    local_get(&mut body, OPTIONS_LOCAL);
    body.push(WasmInstruction::new(WasmOp::Return));

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
        body,
        local_names,
        exported: true,
        source_range: None,
        kind: "executable-generic-invoke".to_string(),
    }
}

fn push_i32(body: &mut Vec<WasmInstruction>, value: i32) {
    body.push(WasmInstruction::with_operands(
        WasmOp::I32Const,
        leb128_signed(i64::from(value)),
    ));
}

fn call(body: &mut Vec<WasmInstruction>, index: u32) {
    body.push(WasmInstruction::with_operands(
        WasmOp::Call,
        leb128_unsigned(u64::from(index)),
    ));
}

fn local_get(body: &mut Vec<WasmInstruction>, index: u64) {
    body.push(WasmInstruction::with_operands(
        WasmOp::LocalGet,
        leb128_unsigned(index),
    ));
}

fn local_set(body: &mut Vec<WasmInstruction>, index: u64) {
    body.push(WasmInstruction::with_operands(
        WasmOp::LocalSet,
        leb128_unsigned(index),
    ));
}

fn local_tee(body: &mut Vec<WasmInstruction>, index: u64) {
    body.push(WasmInstruction::with_operands(
        WasmOp::LocalTee,
        leb128_unsigned(index),
    ));
}

fn call_with_local(body: &mut Vec<WasmInstruction>, import: u32, local: u64) {
    local_get(body, local);
    call(body, import);
}

fn store_i32(body: &mut Vec<WasmInstruction>, offset: i64) {
    let mut operands = leb128_unsigned(2);
    operands.extend(leb128_unsigned(
        u64::try_from(offset).expect("non-negative offset"),
    ));
    body.push(WasmInstruction::with_operands(WasmOp::I32Store, operands));
}

fn load_i32(body: &mut Vec<WasmInstruction>, offset: i64) {
    let mut operands = leb128_unsigned(2);
    operands.extend(leb128_unsigned(
        u64::try_from(offset).expect("non-negative offset"),
    ));
    body.push(WasmInstruction::with_operands(WasmOp::I32Load, operands));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_default(
        function: &ExecutableFunction,
    ) -> Result<WasmGenericInvokePlan, WasmExecutableInvokeDecline> {
        plan_wasm_generic_invoke_named(
            function,
            format!("::executable_generic_invoke_{}", function.id.index()),
        )
    }
    use crate::executable_ir::{
        ExecutableArgvId, ExecutableBlock, ExecutableBlockId, ExecutableFunctionId,
        ExecutableValueId, build_linear_executable_ir,
    };
    use crate::ir::{CommandTokens, NodeId, Script, SourceSite, Statement};
    use tcl_lexer::Span;
    use tcl_registry::CommandRegistry;
    use tcl_registry::dialects::DialectSet;

    fn source() -> SourceSite {
        SourceSite::source(Span::new(0, 1))
    }

    fn literal(text: &str) -> WordExpr {
        WordExpr::Literal {
            text: text.to_owned(),
            source: source(),
        }
    }

    fn expanded_staged() -> ExecutableFunction {
        let expanded = WordExpr::Expand {
            word: Box::new(literal("x")),
            source: source(),
        };
        let words = vec![literal("puts"), expanded.clone()];
        let tokens = CommandTokens {
            argv: vec![Span::new(0, 1), Span::new(2, 3)],
            argv_texts: words.iter().map(WordExpr::legacy_text).collect(),
            word_exprs: words,
            argv_kinds: Vec::new(),
            single_token_word: Vec::new(),
            all_tokens: Vec::new(),
            expand_word: Some(vec![false, true]),
        };
        let script = Script::from_statements(vec![Statement::Call {
            span: Span::new(0, 3),
            command: "puts".to_string(),
            canonical_command: None,
            args: vec![expanded.legacy_text()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(tokens),
            foreach_groups: None,
        }]);
        build_linear_executable_ir(
            &CommandRegistry::build_default(),
            DialectSet::TCL86,
            ExecutableFunctionId::new(1),
            &script,
        )
        .expect("expanded words build a valid staged executable function")
    }

    fn structural_operation_staged() -> ExecutableFunction {
        let words = vec![literal("if"), literal("1"), literal("return")];
        let tokens = CommandTokens {
            argv: vec![Span::new(0, 1), Span::new(2, 3), Span::new(4, 5)],
            argv_texts: words.iter().map(WordExpr::legacy_text).collect(),
            word_exprs: words,
            argv_kinds: Vec::new(),
            single_token_word: Vec::new(),
            all_tokens: Vec::new(),
            expand_word: None,
        };
        let script = Script::from_statements(vec![Statement::Call {
            span: Span::new(0, 5),
            command: "if".to_string(),
            canonical_command: None,
            args: vec!["1".to_string(), "return".to_string()],
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: Some(tokens),
            foreach_groups: None,
        }]);
        build_linear_executable_ir(
            &CommandRegistry::build_default(),
            DialectSet::TCL86,
            ExecutableFunctionId::new(2),
            &script,
        )
        .expect("structural command remains a generic executable invocation")
    }

    fn staged(words: &[&str]) -> ExecutableFunction {
        let function = ExecutableFunctionId::new(0);
        let values = words
            .iter()
            .enumerate()
            .map(|(index, _)| ExecutableValueId::new(function, index))
            .collect::<Vec<_>>();
        let completion = CompletionId::new(function, words.len() + 1);
        let argv = ExecutableArgvId::new(function, 0);
        let build_completion = CompletionId::new(function, words.len());
        let mut stages = words
            .iter()
            .zip(&values)
            .enumerate()
            .map(
                |(index, (word, value))| ExecutableInstruction::EvaluateWord {
                    value: *value,
                    completion: CompletionId::new(function, index),
                    word: literal(word),
                    node: NodeId::from_path(vec![u32::try_from(index).expect("fixture index")]),
                    source: source(),
                },
            )
            .collect::<Vec<_>>();
        stages.push(ExecutableInstruction::BuildArgv {
            argv,
            completion: build_completion,
            entries: values.into_iter().map(ArgvEntry::Value).collect(),
        });
        stages.push(ExecutableInstruction::Invoke(GenericInvoke {
            completion,
            argv,
            resolution: InvocationResolution::Unresolved(
                crate::registry_invocation::OwnedInvocationResolutionUnresolved::UnknownLiteralHead {
                    spelling: words[0].to_string(),
                },
            ),
            original_words: words.iter().map(|word| literal(word)).collect(),
            node: NodeId::from_path(vec![100]),
            source: source(),
        }));
        let stage_count = stages.len();
        let mut blocks = Vec::with_capacity(stage_count * 2 - 1);
        for (index, instruction) in stages.into_iter().enumerate() {
            let id = ExecutableBlockId::new(function, index);
            let stage_completion = instruction_completion(&instruction);
            let terminator = if index + 1 == stage_count {
                ExecutableTerminator::ReturnCompletion(stage_completion)
            } else {
                ExecutableTerminator::CompletionSwitch {
                    completion: stage_completion,
                    cases: vec![crate::executable_ir::CompletionCase {
                        code: tcl_core_types::Code::Ok,
                        target: ExecutableBlockId::new(function, index + 1),
                    }],
                    default: ExecutableBlockId::new(function, stage_count + index),
                }
            };
            blocks.push(ExecutableBlock {
                id,
                instructions: vec![instruction],
                terminator: Some(terminator),
            });
        }
        for index in 0..stage_count - 1 {
            blocks.push(ExecutableBlock {
                id: ExecutableBlockId::new(function, stage_count + index),
                instructions: Vec::new(),
                terminator: Some(ExecutableTerminator::ReturnCompletion(CompletionId::new(
                    function, index,
                ))),
            });
        }
        ExecutableFunction::new(function, ExecutableBlockId::new(function, 0), blocks)
    }

    #[test]
    fn literal_generic_plan_uses_argv_transport_not_eval_source() {
        let plan = plan_default(&staged(&["string", "length", "abc"]))
            .expect("literal staged function plans");
        let module = emit_wasm_generic_invoke_at(&plan, WASM32_CODEGEN_DATA_START)
            .expect("literal staged function lowers");
        let mut module = module;
        assert_eq!(module.functions[0].name, "::executable_generic_invoke_0");
        let wat = module.to_wat();
        assert!(wat.contains("tcl_invoke_argv"));
        assert!(wat.contains("tcl_codegen_call_frame_alloc"));
        assert!(wat.contains("tcl_completion_release"));
        assert!(!wat.contains("tcl_eval_code"));
        assert!(!wat.contains("tcl_eval\""));
        let bytes = module.to_bytes();
        assert_eq!(&bytes[..4], b"\0asm");
        assert!(wat.contains("(import \"tcl\" \"memory\""));
    }

    #[test]
    fn transport_returns_full_completion_handles_for_error_or_custom_code() {
        let plan = plan_default(&staged(&["return", "-code", "73", "value"]))
            .expect("literal generic invocation plans");
        let module =
            emit_wasm_generic_invoke_at(&plan, WASM32_CODEGEN_DATA_START).expect("plan emits");
        assert_eq!(module.functions[0].results, vec![ValType::I32; 3]);
        let names = module
            .imports
            .iter()
            .map(|import| import.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"tcl_obj_retain"));
        assert!(names.contains(&"tcl_completion_release"));
    }

    #[test]
    fn dynamic_and_expanded_words_are_typed_declines() {
        let mut dynamic = staged(&["puts", "x"]);
        let ExecutableInstruction::EvaluateWord { word, .. } =
            &mut dynamic.blocks[1].instructions[0]
        else {
            panic!("fixture evaluation");
        };
        *word = WordExpr::Variable {
            spelling: "$x".to_string(),
            source: source(),
        };
        let ExecutableInstruction::Invoke(invoke) = &mut dynamic.blocks[3].instructions[0] else {
            panic!("fixture invoke");
        };
        invoke.original_words[1] = WordExpr::Variable {
            spelling: "$x".to_string(),
            source: source(),
        };
        assert!(matches!(
            plan_default(&dynamic),
            Err(WasmExecutableInvokeDecline::NonLiteralWord { .. })
        ));

        assert_eq!(
            plan_default(&expanded_staged()),
            Err(WasmExecutableInvokeDecline::ArgumentExpansion)
        );
    }

    #[test]
    fn planner_requires_explicit_ok_and_abrupt_completion_edges() {
        let mut function = staged(&["list", "x"]);
        let Some(ExecutableTerminator::CompletionSwitch { cases, default, .. }) =
            function.blocks[0].terminator.as_mut()
        else {
            panic!("fixture word stage has a completion switch");
        };
        cases.push(crate::executable_ir::CompletionCase {
            code: tcl_core_types::Code::Error,
            target: *default,
        });
        assert_eq!(
            plan_default(&function),
            Err(WasmExecutableInvokeDecline::UnsupportedControlFlow)
        );
    }

    #[test]
    fn generic_operation_is_a_registry_level_fallback() {
        assert_eq!(
            plan_default(&staged(&["arbitrary", "literal"]))
                .expect("unknown literal heads still use generic invocation")
                .operation,
            SemanticOperationId::Invoke
        );
    }

    #[test]
    fn generic_backend_plan_preserves_resolved_structural_operation() {
        let plan = plan_default(&structural_operation_staged())
            .expect("literal structural invocation can use generic argv transport");
        assert!(matches!(
            plan.operation,
            SemanticOperationId::StructuredLowering(_)
        ));
    }

    #[test]
    fn caller_can_name_the_isolated_generated_function() {
        let plan =
            plan_wasm_generic_invoke_named(&staged(&["list", "x"]), "::unit::call".to_string())
                .expect("named plan");
        assert_eq!(plan.function_name, "::unit::call");
    }

    #[test]
    fn constant_pool_stays_inside_the_shared_codegen_data_window() {
        let one_byte = plan_default(&staged(&["x"])).expect("one-byte literal plans");
        assert!(emit_wasm_generic_invoke_at(&one_byte, WASM32_CODEGEN_DATA_END - 1).is_ok());
        assert!(matches!(
            emit_wasm_generic_invoke_at(&one_byte, WASM32_CODEGEN_DATA_START - 1),
            Err(WasmExecutableInvokeDecline::InvalidDataBase)
        ));
        assert!(matches!(
            emit_wasm_generic_invoke_at(&one_byte, WASM32_CODEGEN_DATA_END),
            Err(WasmExecutableInvokeDecline::InvalidDataBase)
        ));

        let two_bytes = plan_default(&staged(&["x", "y"])).expect("two-byte pool plans");
        assert!(matches!(
            emit_wasm_generic_invoke_at(&two_bytes, WASM32_CODEGEN_DATA_END - 1),
            Err(WasmExecutableInvokeDecline::ConstantPoolOutOfBounds)
        ));

        let mut oversized = one_byte;
        let window_bytes = usize::try_from(WASM32_CODEGEN_DATA_END - WASM32_CODEGEN_DATA_START)
            .expect("reserved data-window size fits usize");
        oversized.argv_literals[0] = "x".repeat(window_bytes + 1);
        assert!(matches!(
            emit_wasm_generic_invoke_at(&oversized, WASM32_CODEGEN_DATA_START),
            Err(WasmExecutableInvokeDecline::ConstantPoolOutOfBounds)
        ));
    }
}
