// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target planning for a prebuilt-argv executable-IR invocation.
//!
//! This module validates and retains immutable semantic input for the sole
//! WASM emitter. It does not create a [`super::WasmModule`] or serialise any
//! target instructions.

use crate::executable_ir::{
    ArgvEntry, CompletionId, ExecutableArgvId, ExecutableBlockId, ExecutableFunction,
    ExecutableInstruction, ExecutableTerminator, ExecutableValueId, GenericInvoke,
    InvocationResolution,
};
use crate::ir::{NodeId, WordExpr};
use tcl_registry::SemanticOperationId;
use tcl_runtime_api::codegen_abi::{
    WASM32_CODEGEN_DATA_END, WASM32_CODEGEN_DATA_START, WASM32_COMPLETION_SIZE,
    WASM32_POINTER_BYTES,
};

/// Immutable semantic input for one prebuilt-argv invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WasmGenericInvokePlan {
    pub(super) function_name: String,
    /// Stable invocation site selected from the executable semantic IR.
    pub(super) node: NodeId,
    pub(super) operation: SemanticOperationId,
    /// The exact prebuilt argv consumed by the selected invocation.
    pub(super) argv: ExecutableArgvId,
    pub(super) argv_literals: Vec<String>,
    pub(super) completion: CompletionId,
    pub(super) stage_proofs: Vec<WasmStageProof>,
}

/// Proof that a staged executable-IR completion side edge is unreachable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WasmStageProof {
    LiteralMaterialisation { completion: CompletionId },
    FrameAssembly { completion: CompletionId },
}

/// A precise reason the prebuilt-argv semantic plan declined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmExecutableInvokeDecline {
    /// Executable IR failed its dominance, ownership, or CFG checks.
    InvalidExecutableIr,
    /// The function does not have the bounded single-invocation shape.
    UnsupportedControlFlow,
    /// An instruction is outside literal evaluation, argv construction, and invocation.
    UnsupportedInstruction,
    /// A word needs Tcl evaluation rather than literal materialisation.
    NonLiteralWord {
        /// Executable value whose word is dynamic.
        value: ExecutableValueId,
    },
    /// `{*}` expansion requires runtime Tcl list processing.
    ArgumentExpansion,
    /// Argv construction does not use the preceding values exactly.
    InconsistentArgv,
    /// The invocation does not consume the planned argv.
    InconsistentInvocation,
    /// The return terminator does not forward the invocation completion.
    InconsistentCompletionReturn,
    /// A literal does not fit the wasm32 string-length ABI.
    LiteralTooLong,
    /// The argv count does not fit the wasm32 ABI.
    TooManyWords,
    /// The transient call frame does not fit wasm32 addressing.
    FrameTooLarge,
    /// The caller selected a base outside the runtime-reserved window.
    InvalidDataBase,
    /// The literal pool would escape the runtime-reserved window.
    ConstantPoolOutOfBounds,
    /// The requested export name is empty.
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

/// Validate executable IR and retain the literal argv in evaluation order.
pub(super) fn plan_wasm_generic_invoke_named(
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
    validate_frame(literals.len())?;
    if literals
        .iter()
        .any(|literal| i32::try_from(literal.len()).is_err())
    {
        return Err(WasmExecutableInvokeDecline::LiteralTooLong);
    }
    let mut stage_proofs = Vec::with_capacity(build_index + 1);
    for stage in stages.iter().take(build_index + 1) {
        stage_proofs.push(match stage.instruction {
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
            | ExecutableInstruction::ExecuteOpaqueRegion(_)
            | ExecutableInstruction::EvaluateExpr { .. }
            | ExecutableInstruction::MatchPattern { .. }
            | ExecutableInstruction::IterateLists { .. }
            | ExecutableInstruction::JoinCompletion { .. }
            | ExecutableInstruction::WriteCompletionCell { .. }
            | ExecutableInstruction::CompleteStructuredRegion(_) => {
                return Err(WasmExecutableInvokeDecline::UnsupportedInstruction);
            }
        });
    }
    Ok(WasmGenericInvokePlan {
        function_name,
        node: invoke.node.clone(),
        operation: invoke_operation(invoke),
        argv: invoke.argv,
        argv_literals: literals,
        completion: invoke.completion,
        stage_proofs,
    })
}

/// Validate layout before the pipeline selects semantic emission.
pub(super) fn validate_plan_layout(
    plan: &WasmGenericInvokePlan,
    data_base: i64,
) -> Result<(), WasmExecutableInvokeDecline> {
    if !(WASM32_CODEGEN_DATA_START..WASM32_CODEGEN_DATA_END).contains(&data_base) {
        return Err(WasmExecutableInvokeDecline::InvalidDataBase);
    }
    validate_frame(plan.argv_literals.len())?;
    let mut offset = data_base;
    for literal in &plan.argv_literals {
        let length = i64::try_from(literal.len())
            .map_err(|_| WasmExecutableInvokeDecline::LiteralTooLong)?;
        offset = offset
            .checked_add(length)
            .ok_or(WasmExecutableInvokeDecline::ConstantPoolOutOfBounds)?;
        if offset > WASM32_CODEGEN_DATA_END {
            return Err(WasmExecutableInvokeDecline::ConstantPoolOutOfBounds);
        }
    }
    Ok(())
}

fn validate_frame(argc: usize) -> Result<(), WasmExecutableInvokeDecline> {
    let argc = i32::try_from(argc).map_err(|_| WasmExecutableInvokeDecline::TooManyWords)?;
    argc.checked_mul(WASM32_POINTER_BYTES)
        .and_then(|bytes| bytes.checked_add(WASM32_COMPLETION_SIZE))
        .ok_or(WasmExecutableInvokeDecline::FrameTooLarge)?;
    Ok(())
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
            | ExecutableInstruction::ExecuteOpaqueRegion(_)
            | ExecutableInstruction::EvaluateExpr { .. }
            | ExecutableInstruction::MatchPattern { .. }
            | ExecutableInstruction::IterateLists { .. }
            | ExecutableInstruction::JoinCompletion { .. }
            | ExecutableInstruction::WriteCompletionCell { .. }
            | ExecutableInstruction::CompleteStructuredRegion(_) => {
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
        | ExecutableInstruction::BuildArgv { completion, .. }
        | ExecutableInstruction::EvaluateExpr { completion, .. }
        | ExecutableInstruction::MatchPattern { completion, .. }
        | ExecutableInstruction::JoinCompletion { completion, .. }
        | ExecutableInstruction::WriteCompletionCell { completion, .. }
        | ExecutableInstruction::IterateLists { completion, .. } => *completion,
        ExecutableInstruction::Invoke(invoke) => invoke.completion,
        ExecutableInstruction::ExecuteLowered(operation) => operation.completion,
        ExecutableInstruction::ExecuteOpaqueRegion(region) => region.completion,
        ExecutableInstruction::CompleteStructuredRegion(region) => region.completion,
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
