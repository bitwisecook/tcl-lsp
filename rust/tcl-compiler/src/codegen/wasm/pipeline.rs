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

//! Public compiler pipeline for the deliberately narrow WASM argv transport.
//!
//! This is not the old source-replaying tree-walker backend.  It first builds
//! [`ExecutableFunction`](crate::executable_ir::ExecutableFunction), retains
//! the registry's semantic operation and selection facts, chooses the generic
//! argv rung through a backend registry, and only then emits the selected
//! immutable plan.  The present emitter intentionally declines anything other
//! than one literal-safe staged invocation; callers must use a different
//! backend for a multi-command or dynamic program.

use tcl_registry::{CommandRegistry, SemanticOperationId, dialects::DialectSet};

use crate::backend_registry::{
    BackendDeclineReason, BackendPlanKind, BackendRegistry, BackendSelection, BackendSelector,
    SelectionFacts, SelectionInput, SelectionRegion, SelectorDecision, SelectorPriority,
    SelectorRequest,
};
use crate::executable_ir::{
    ExecutableFunction, ExecutableFunctionId, ExecutableInstruction, InvocationResolution,
    SourceCompatibilityDecline, build_linear_executable_ir,
};
use crate::ir::Script;
use crate::target_contract::{
    LegalisationRequirements, TargetCapabilities, TargetContract, TargetFamily,
};

use super::{
    RESERVED_DATA_BASE, WasmExecutableInvokeDecline, WasmGenericInvokePlan, WasmModule,
    emit_wasm_generic_invoke_at, plan_wasm_generic_invoke_named,
};

/// Input identity and layout choices for the literal-safe generic WASM API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralSafeWasmOptions {
    /// Identity assigned to the executable semantic function.
    pub function: ExecutableFunctionId,
    /// Stable export name requested for the generated WASM function.
    pub export_name: String,
    /// First byte of the runtime-reserved immutable data region.
    pub data_base: i64,
}

impl LiteralSafeWasmOptions {
    /// Create explicit options for one isolated generated function.
    #[must_use]
    pub fn new(function: ExecutableFunctionId, export_name: impl Into<String>) -> Self {
        Self {
            function,
            export_name: export_name.into(),
            data_base: RESERVED_DATA_BASE,
        }
    }

    /// Relocate the generated immutable data segments within the runtime's
    /// reserved region.  The emitter validates the final wasm32 address.
    #[must_use]
    pub const fn with_data_base(mut self, data_base: i64) -> Self {
        self.data_base = data_base;
        self
    }
}

/// The checked common-IR and target artifact produced by this narrow path.
#[derive(Debug, Clone)]
pub struct LiteralSafeWasmOutput {
    /// Validated executable semantic function used to build the target plan.
    pub executable: ExecutableFunction,
    /// Registry-owned semantic operation retained by the selected plan.
    pub operation: SemanticOperationId,
    /// The backend-registry ladder rung that authorised emission.
    pub plan_kind: BackendPlanKind,
    /// Generated module with generic argv invocation imports and no eval import.
    pub module: WasmModule,
}

/// A typed refusal from the literal-safe generic invocation pipeline.
#[derive(Debug, thiserror::Error)]
pub enum LiteralSafeWasmDecline {
    /// The source compatibility bridge cannot form executable semantic IR.
    #[error("executable semantic IR declined the source shape: {0:?}")]
    Source(SourceCompatibilityDecline),
    /// The literal-only target planner declined a word or CFG shape.
    #[error("literal-safe WASM lowering declined the executable shape: {0:?}")]
    Executable(WasmExecutableInvokeDecline),
    /// The per-WASM backend registry found no legal generic argv plan.
    #[error("WASM backend selection declined generic argv invocation: {0:?}")]
    Backend(BackendDeclineReason<WasmExecutableInvokeDecline>),
    /// The fixed generic selector was unexpectedly registered twice.
    #[error("WASM generic argv selector registration failed")]
    DuplicateSelector,
    /// A non-generic plan was returned by the fixed literal-safe registry.
    #[error("WASM backend selection returned an unexpected plan kind: {0:?}")]
    UnexpectedPlan(BackendPlanKind),
}

/// Compile one flat, literal-safe script through common executable IR.
///
/// `dialect` is intentionally explicit: command resolution must never infer a
/// profile from source text at this layer.  The source compatibility builder,
/// the invocation facts supplied to selection, and the emitted plan therefore
/// all describe exactly the same dialect.
pub fn compile_literal_safe_wasm(
    registry: &CommandRegistry,
    dialect: DialectSet,
    script: &Script,
    options: LiteralSafeWasmOptions,
) -> Result<LiteralSafeWasmOutput, LiteralSafeWasmDecline> {
    let LiteralSafeWasmOptions {
        function,
        export_name,
        data_base,
    } = options;
    let executable = build_linear_executable_ir(registry, dialect, function, script)
        .map_err(LiteralSafeWasmDecline::Source)?;
    let operation = selection_operation(&executable);
    let context = LiteralSafeWasmSelectionContext {
        function: &executable,
        export_name: &export_name,
    };
    let selection = literal_safe_wasm_registry()
        .map_err(|_| LiteralSafeWasmDecline::DuplicateSelector)?
        .select_with_context(
            SelectionInput::with_facts(
                operation,
                SelectionRegion::PrebuiltArgvInvocation,
                selection_facts(&executable),
            ),
            &context,
        );
    let selected = match selection {
        BackendSelection::Selected(selected) => selected,
        BackendSelection::Declined(reason) => return Err(LiteralSafeWasmDecline::Backend(reason)),
    };
    if selected.kind() != BackendPlanKind::GenericInvoke {
        return Err(LiteralSafeWasmDecline::UnexpectedPlan(selected.kind()));
    }
    let module = emit_wasm_generic_invoke_at(&selected.into_plan(), data_base)
        .map_err(LiteralSafeWasmDecline::Executable)?;
    Ok(LiteralSafeWasmOutput {
        executable,
        operation,
        plan_kind: BackendPlanKind::GenericInvoke,
        module,
    })
}

fn selection_operation(function: &ExecutableFunction) -> SemanticOperationId {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match instruction {
            ExecutableInstruction::Invoke(invoke) => Some(match &invoke.resolution {
                InvocationResolution::Resolved(facts) => facts.operation,
                InvocationResolution::Unresolved(_) => SemanticOperationId::Invoke,
            }),
            ExecutableInstruction::EvaluateWord { .. }
            | ExecutableInstruction::ExpandWord { .. }
            | ExecutableInstruction::BuildArgv { .. }
            | ExecutableInstruction::ExecuteLowered(_)
            | ExecutableInstruction::ExecuteOpaqueRegion(_) => None,
        })
        .unwrap_or(SemanticOperationId::Invoke)
}

fn selection_facts(function: &ExecutableFunction) -> SelectionFacts<'_> {
    function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match instruction {
            ExecutableInstruction::Invoke(invoke) => match &invoke.resolution {
                InvocationResolution::Resolved(facts) => {
                    Some(SelectionFacts::from_invocation(facts))
                }
                InvocationResolution::Unresolved(_) => None,
            },
            ExecutableInstruction::EvaluateWord { .. }
            | ExecutableInstruction::ExpandWord { .. }
            | ExecutableInstruction::BuildArgv { .. }
            | ExecutableInstruction::ExecuteLowered(_)
            | ExecutableInstruction::ExecuteOpaqueRegion(_) => None,
        })
        .unwrap_or_else(SelectionFacts::unavailable)
}

fn literal_safe_wasm_registry<'a>() -> Result<
    BackendRegistry<
        WasmGenericInvokePlan,
        LiteralSafeWasmSelectionContext<'a>,
        WasmExecutableInvokeDecline,
    >,
    crate::backend_registry::DuplicateSelector,
> {
    let mut registry = BackendRegistry::new(TargetContract::new(
        TargetFamily::Wasm,
        TargetCapabilities::wasm(),
    ));
    registry.register(
        SemanticOperationId::Invoke,
        BackendPlanKind::GenericInvoke,
        SelectorPriority::DEFAULT,
        LegalisationRequirements::new(),
        LiteralSafeGenericSelector,
    )?;
    Ok(registry)
}

#[derive(Debug, Clone)]
struct LiteralSafeWasmSelectionContext<'a> {
    function: &'a ExecutableFunction,
    export_name: &'a str,
}

#[derive(Debug, Clone, Copy)]
struct LiteralSafeGenericSelector;

impl<'a>
    BackendSelector<
        WasmGenericInvokePlan,
        LiteralSafeWasmSelectionContext<'a>,
        WasmExecutableInvokeDecline,
    > for LiteralSafeGenericSelector
{
    fn select(
        &self,
        request: &SelectorRequest<'_, LiteralSafeWasmSelectionContext<'a>>,
    ) -> SelectorDecision<WasmGenericInvokePlan, WasmExecutableInvokeDecline> {
        let context = request.context();
        match plan_wasm_generic_invoke_named(context.function, context.export_name.to_owned()) {
            Ok(plan) => SelectorDecision::Selected(plan),
            Err(decline) => SelectorDecision::Declined(decline),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;

    #[test]
    fn literal_invocation_uses_generic_registry_rung_without_eval_import() {
        let registry = CommandRegistry::build_default();
        let module = lower_to_ir("string length hello", &registry);
        let mut output = compile_literal_safe_wasm(
            &registry,
            DialectSet::TCL86,
            &module.top_level,
            LiteralSafeWasmOptions::new(ExecutableFunctionId::new(44), "::literal"),
        )
        .expect("literal invocation compiles through common executable IR");

        assert_eq!(output.plan_kind, BackendPlanKind::GenericInvoke);
        assert_ne!(output.operation, SemanticOperationId::Invoke);
        let wat = output.module.to_wat();
        assert!(wat.contains("tcl_invoke_argv"), "{wat}");
        assert!(!wat.contains("tcl_eval"), "{wat}");
    }

    #[test]
    fn dynamic_word_declines_instead_of_replaying_source_with_eval() {
        let registry = CommandRegistry::build_default();
        let module = lower_to_ir("string length $value", &registry);
        let decline = compile_literal_safe_wasm(
            &registry,
            DialectSet::TCL86,
            &module.top_level,
            LiteralSafeWasmOptions::new(ExecutableFunctionId::new(45), "::dynamic"),
        )
        .expect_err("dynamic word is not literal-safe");
        let LiteralSafeWasmDecline::Backend(BackendDeclineReason::NoViablePlan {
            attempts, ..
        }) = decline
        else {
            panic!("dynamic word should be a typed backend-selector decline");
        };
        assert!(attempts.iter().any(|attempt| matches!(
            &attempt.failure,
            crate::backend_registry::SelectorAttemptFailure::Selector(
                WasmExecutableInvokeDecline::NonLiteralWord { .. }
            )
        )));
    }
}
