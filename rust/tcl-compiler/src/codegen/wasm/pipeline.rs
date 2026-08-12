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

//! The one public Tcl-to-WebAssembly compilation pipeline.
//!
//! Common executable IR and [`BackendRegistry`] selection choose an input mode
//! for one emitter. A typed semantic decline selects general structured
//! lowering inside that emitter; it never selects another implementation.

use std::ops::{Deref, DerefMut};

use tcl_registry::{CommandRegistry, SemanticOperationId};

use crate::backend_registry::{
    BackendDeclineReason, BackendPlanKind, BackendRegistry, BackendSelection, BackendSelector,
    SelectionFacts, SelectionInput, SelectionRegion, SelectorDecision, SelectorPriority,
    SelectorRequest,
};
use crate::compilation_unit::CompilationUnit;
use crate::executable_ir::{
    ExecutableFunction, ExecutableInstruction, InvocationResolution, SourceCompatibilityDecline,
};
use crate::semantic_analysis::ExecutableAnalysisAvailability;
use crate::target_contract::{
    LegalisationRequirements, TargetCapabilities, TargetContract, TargetFamily,
};

use super::semantic_plan::{
    WasmExecutableInvokeDecline, WasmGenericInvokePlan, plan_wasm_generic_invoke_named,
    validate_plan_layout,
};
use super::{RESERVED_DATA_BASE, WasmModule, backend};
use backend::WasmEmissionMode;

/// Packaging and semantic-plan policy for [`compile_wasm`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WasmCompileOptions {
    pub(super) data_base: i64,
    packaging: WasmPackaging,
    plan_policy: WasmPlanPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WasmPackaging {
    Hosted,
    Standalone { initialise_library: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WasmPlanPolicy {
    SemanticFirst,
    EvalOnlyTestHost,
}

impl WasmCompileOptions {
    /// A host-loaded module using the runtime ABI's reserved data window.
    #[must_use]
    pub const fn hosted() -> Self {
        Self {
            data_base: RESERVED_DATA_BASE,
            packaging: WasmPackaging::Hosted,
            plan_policy: WasmPlanPolicy::SemanticFirst,
        }
    }

    /// A module relocated into the data window reserved by `runtime/rust`.
    #[must_use]
    pub const fn runtime_linked() -> Self {
        Self::hosted()
    }

    /// A relocated WASI command that creates an interpreter and runs `::top`.
    ///
    /// `initialise_library` additionally loads the embedded standard library
    /// before entering the compiled program. Standalone bootstrap synthesis is
    /// not yet represented in executable IR, so this packaging shape records a
    /// typed semantic decline on the general plan.
    #[must_use]
    pub const fn standalone(initialise_library: bool) -> Self {
        Self {
            packaging: WasmPackaging::Standalone { initialise_library },
            ..Self::runtime_linked()
        }
    }

    /// Relocate the immutable data pool to a caller-selected address.
    ///
    /// Production modules should keep the runtime-reserved default. This
    /// option exists for ABI boundary tests and evaluation-only hosts.
    #[must_use]
    pub const fn with_data_base(mut self, data_base: i64) -> Self {
        self.data_base = data_base;
        self
    }

    /// Use the source-evaluation ABI expected by an isolated test host.
    ///
    /// The differential test host currently implements the source-evaluation
    /// ABI only. This option does not choose another public backend: the sole
    /// pipeline records [`WasmSemanticDecline::SemanticPlansDisabled`] and
    /// disables analysis specialisations that require more host
    /// imports.
    #[must_use]
    pub const fn for_eval_only_test_host(mut self) -> Self {
        self.plan_policy = WasmPlanPolicy::EvalOnlyTestHost;
        self
    }

    pub(super) const fn is_standalone(self) -> bool {
        matches!(self.packaging, WasmPackaging::Standalone { .. })
    }

    pub(super) const fn initialise_library(self) -> bool {
        match self.packaging {
            WasmPackaging::Hosted => false,
            WasmPackaging::Standalone { initialise_library } => initialise_library,
        }
    }

    pub(super) const fn analysis_specialisations(self) -> bool {
        matches!(self.plan_policy, WasmPlanPolicy::SemanticFirst)
    }
}

impl Default for WasmCompileOptions {
    fn default() -> Self {
        Self::hosted()
    }
}

/// Stable packaging shapes that common executable IR cannot yet synthesise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmPackagingConstraint {
    /// WASI `_start`, interpreter creation, and optional library initialisation.
    StandaloneBootstrap,
}

impl WasmPackagingConstraint {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StandaloneBootstrap => "standalone-bootstrap",
        }
    }
}

/// Typed common-IR availability reasons retained by the canonical pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmExecutableAvailabilityDecline {
    /// No single registry dialect could be selected for executable analysis.
    DialectUnavailable,
    /// The source compatibility bridge declined with its precise reason.
    Source(SourceCompatibilityDecline),
    /// The compilation unit did not retain a source script for this function.
    SourceUnavailable,
}

impl WasmExecutableAvailabilityDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::DialectUnavailable => "dialect-unavailable",
            Self::Source(_) => "source-shape-declined",
            Self::SourceUnavailable => "source-unavailable",
        }
    }

    /// Stable precise reason within the availability class.
    #[must_use]
    pub const fn detail_kind(&self) -> &'static str {
        match self {
            Self::DialectUnavailable => "dialect-unavailable",
            Self::Source(SourceCompatibilityDecline::EmptyScript) => "empty-script",
            Self::Source(SourceCompatibilityDecline::UnsupportedStatement { .. }) => {
                "unsupported-statement"
            }
            Self::Source(SourceCompatibilityDecline::MissingCommandTokens { .. }) => {
                "missing-command-tokens"
            }
            Self::Source(SourceCompatibilityDecline::InconsistentCommandTokens { .. }) => {
                "inconsistent-command-tokens"
            }
            Self::Source(SourceCompatibilityDecline::MissingCommandHead { .. }) => {
                "missing-command-head"
            }
            Self::Source(SourceCompatibilityDecline::IncompleteRegistryResolution { .. }) => {
                "incomplete-registry-resolution"
            }
            Self::SourceUnavailable => "source-unavailable",
        }
    }
}

/// Why semantic prebuilt-argv selection declined to general lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmSemanticDecline {
    /// The caller's isolated host implements only the general evaluation ABI.
    SemanticPlansDisabled,
    /// The requested package shape is not yet expressed by executable IR.
    Packaging(WasmPackagingConstraint),
    /// Common executable IR was unavailable with a retained typed reason.
    ExecutableUnavailable(WasmExecutableAvailabilityDecline),
    /// The WASM backend registry declined every generic invocation selector.
    BackendSelection(BackendDeclineReason<WasmExecutableInvokeDecline>),
    /// The selected immutable plan is not legal at the requested module layout.
    PlanLayout(WasmExecutableInvokeDecline),
    /// Construction of the fixed WASM selector registry failed.
    SelectorRegistration,
}

impl WasmSemanticDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SemanticPlansDisabled => "semantic-plans-disabled",
            Self::Packaging(_) => "packaging-constraint",
            Self::ExecutableUnavailable(_) => "executable-ir-unavailable",
            Self::BackendSelection(_) => "backend-selection-declined",
            Self::PlanLayout(_) => "semantic-plan-layout-declined",
            Self::SelectorRegistration => "selector-registration-failed",
        }
    }

    /// Stable precise reason inside the semantic-decline class.
    #[must_use]
    pub const fn detail_kind(&self) -> &'static str {
        match self {
            Self::SemanticPlansDisabled => "eval-only-test-host",
            Self::Packaging(constraint) => constraint.as_str(),
            Self::ExecutableUnavailable(decline) => decline.detail_kind(),
            Self::BackendSelection(BackendDeclineReason::MissingOperation(_)) => {
                "missing-operation-selector"
            }
            Self::BackendSelection(BackendDeclineReason::NoViablePlan { .. }) => {
                "no-viable-semantic-plan"
            }
            Self::PlanLayout(decline) => decline.as_str(),
            Self::SelectorRegistration => "duplicate-selector",
        }
    }
}

/// The immutable plan chosen by the canonical code-generation pipeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmCodegenPlan {
    /// Executable IR selected generic prebuilt-argv invocation.
    GenericInvoke {
        /// Registry-owned semantic operation retained by the selected plan.
        operation: SemanticOperationId,
    },
    /// General structured lowering ran in the same emitter.
    General {
        /// Typed reason the narrower semantic input mode was not selected.
        semantic_decline: WasmSemanticDecline,
    },
}

impl WasmCodegenPlan {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::GenericInvoke { .. } => "generic-invoke",
            Self::General { .. } => "general",
        }
    }

    /// Typed reason the narrow semantic input mode declined.
    #[must_use]
    pub const fn semantic_decline(&self) -> Option<&WasmSemanticDecline> {
        match self {
            Self::GenericInvoke { .. } => None,
            Self::General { semantic_decline } => Some(semantic_decline),
        }
    }

    /// Stable operation category selected by common semantic facts.
    #[must_use]
    pub const fn operation_kind(&self) -> Option<&'static str> {
        match self {
            Self::GenericInvoke {
                operation: SemanticOperationId::Invoke,
            } => Some("invoke"),
            Self::GenericInvoke {
                operation: SemanticOperationId::Intrinsic(_),
            } => Some("intrinsic"),
            Self::GenericInvoke {
                operation: SemanticOperationId::StructuredLowering(_),
            } => Some("structured-lowering"),
            Self::General { .. } => None,
        }
    }
}

/// Canonical code-generation artifact and durable selection evidence.
#[derive(Debug, Clone)]
pub struct WasmCompilation {
    /// Generated module consumed by encoding, rendering, linking, and bundling.
    pub module: WasmModule,
    /// Selected semantic or general plan.
    pub plan: WasmCodegenPlan,
}

impl Deref for WasmCompilation {
    type Target = WasmModule;

    fn deref(&self) -> &Self::Target {
        &self.module
    }
}

impl DerefMut for WasmCompilation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.module
    }
}

impl WasmCompilation {
    /// Consume the selection record and return its generated module.
    #[must_use]
    pub fn into_module(self) -> WasmModule {
        self.module
    }
}

#[derive(Debug)]
struct GenericSelectionContext<'a> {
    function: &'a ExecutableFunction,
}

#[derive(Debug, Clone, Copy)]
struct GenericInvokeSelector;

impl<'a>
    BackendSelector<WasmGenericInvokePlan, GenericSelectionContext<'a>, WasmExecutableInvokeDecline>
    for GenericInvokeSelector
{
    fn select(
        &self,
        request: &SelectorRequest<'_, GenericSelectionContext<'a>>,
    ) -> SelectorDecision<WasmGenericInvokePlan, WasmExecutableInvokeDecline> {
        match plan_wasm_generic_invoke_named(request.context().function, "::top".to_owned()) {
            Ok(plan) => SelectorDecision::Selected(plan),
            Err(decline) => SelectorDecision::Declined(decline),
        }
    }
}

/// Compile one fully analysed source unit to WebAssembly.
///
/// This is the sole production code-generation entry point. It first consumes
/// the executable semantic facts already attached to the compilation unit and
/// selects a plan through [`BackendRegistry`]. The result then enters the sole
/// module emitter exactly once, either as a selected semantic invocation or as
/// general structured lowering carrying a typed semantic decline.
#[must_use]
pub fn compile_wasm(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    options: WasmCompileOptions,
) -> WasmCompilation {
    let (semantic_plan, evidence) = match select_semantic_plan(unit, options) {
        Ok(plan) => match validate_plan_layout(&plan, options.data_base) {
            Ok(()) => {
                let operation = plan.operation;
                (Some(plan), WasmCodegenPlan::GenericInvoke { operation })
            }
            Err(decline) => (
                None,
                WasmCodegenPlan::General {
                    semantic_decline: WasmSemanticDecline::PlanLayout(decline),
                },
            ),
        },
        Err(semantic_decline) => (None, WasmCodegenPlan::General { semantic_decline }),
    };
    let mode = semantic_plan
        .as_ref()
        .map_or(WasmEmissionMode::General, WasmEmissionMode::SemanticInvoke);
    WasmCompilation {
        module: backend::emit_wasm(unit, registry, options, mode),
        plan: evidence,
    }
}

fn select_semantic_plan(
    unit: &CompilationUnit,
    options: WasmCompileOptions,
) -> Result<WasmGenericInvokePlan, WasmSemanticDecline> {
    if matches!(options.plan_policy, WasmPlanPolicy::EvalOnlyTestHost) {
        return Err(WasmSemanticDecline::SemanticPlansDisabled);
    }
    if options.is_standalone() {
        return Err(WasmSemanticDecline::Packaging(
            WasmPackagingConstraint::StandaloneBootstrap,
        ));
    }
    let availability = unit.top_level.semantic_facts.executable();
    let function = availability.function().ok_or_else(|| {
        WasmSemanticDecline::ExecutableUnavailable(match availability {
            ExecutableAnalysisAvailability::DialectUnavailable { .. } => {
                WasmExecutableAvailabilityDecline::DialectUnavailable
            }
            ExecutableAnalysisAvailability::SourceDeclined(decline) => {
                WasmExecutableAvailabilityDecline::Source(decline.clone())
            }
            ExecutableAnalysisAvailability::SourceUnavailable => {
                WasmExecutableAvailabilityDecline::SourceUnavailable
            }
            ExecutableAnalysisAvailability::Available(_)
            | ExecutableAnalysisAvailability::WorldStateDeclined { .. }
            | ExecutableAnalysisAvailability::WorldStateNotRequired { .. } => {
                unreachable!("function() covers executable availability")
            }
        })
    })?;
    let mut selector_registry = BackendRegistry::new(TargetContract::new(
        TargetFamily::Wasm,
        TargetCapabilities::wasm(),
    ));
    selector_registry
        .register(
            SemanticOperationId::Invoke,
            BackendPlanKind::GenericInvoke,
            SelectorPriority::DEFAULT,
            LegalisationRequirements::new(),
            GenericInvokeSelector,
        )
        .map_err(|_| WasmSemanticDecline::SelectorRegistration)?;
    let operation = selection_operation(function);
    let context = GenericSelectionContext { function };
    match selector_registry.select_with_context(
        SelectionInput::with_facts(
            operation,
            SelectionRegion::PrebuiltArgvInvocation,
            selection_facts(function),
        ),
        &context,
    ) {
        BackendSelection::Selected(selected) => Ok(selected.into_plan()),
        BackendSelection::Declined(decline) => Err(WasmSemanticDecline::BackendSelection(decline)),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(source: &str, registry: &CommandRegistry) -> CompilationUnit {
        CompilationUnit::build_for_dialect(source, registry, false, "tcl8.6")
    }

    #[test]
    fn hosted_literal_invocation_selects_executable_generic_argv_plan() {
        let registry = CommandRegistry::build_default();
        let mut output = compile_wasm(
            &unit("string length hello", &registry),
            &registry,
            WasmCompileOptions::hosted(),
        );

        assert!(matches!(output.plan, WasmCodegenPlan::GenericInvoke { .. }));
        let wat = output.to_wat();
        assert!(wat.contains("tcl_invoke_argv"), "{wat}");
        assert!(!wat.contains("tcl_eval"), "{wat}");
        assert!(
            output
                .data_segments
                .iter()
                .all(|segment| segment.offset >= RESERVED_DATA_BASE)
        );
    }

    #[test]
    fn broad_source_records_typed_semantic_decline_on_general_plan() {
        let registry = CommandRegistry::build_default();
        let mut output = compile_wasm(
            &unit("string length $value", &registry),
            &registry,
            WasmCompileOptions::hosted(),
        );

        let WasmCodegenPlan::General {
            semantic_decline:
                WasmSemanticDecline::BackendSelection(BackendDeclineReason::NoViablePlan {
                    attempts,
                    ..
                }),
        } = &output.plan
        else {
            panic!("expected typed backend decline, got {:?}", output.plan);
        };
        assert!(!attempts.is_empty());
        assert!(output.to_wat().contains("tcl_eval_code"));
    }

    #[test]
    fn standalone_records_packaging_decline_on_the_same_emitter() {
        let registry = CommandRegistry::build_default();
        let output = compile_wasm(
            &unit("puts hello", &registry),
            &registry,
            WasmCompileOptions::standalone(true),
        );

        assert_eq!(
            output.plan,
            WasmCodegenPlan::General {
                semantic_decline: WasmSemanticDecline::Packaging(
                    WasmPackagingConstraint::StandaloneBootstrap
                )
            }
        );
        assert!(
            output
                .functions
                .iter()
                .any(|function| function.name == "_start")
        );
    }

    #[test]
    fn invalid_semantic_layout_declines_before_the_single_emitter_runs() {
        let registry = CommandRegistry::build_default();
        let mut output = compile_wasm(
            &unit("string length hello", &registry),
            &registry,
            WasmCompileOptions::hosted().with_data_base(0),
        );

        assert_eq!(
            output.plan,
            WasmCodegenPlan::General {
                semantic_decline: WasmSemanticDecline::PlanLayout(
                    WasmExecutableInvokeDecline::InvalidDataBase
                )
            }
        );
        assert!(output.to_wat().contains("tcl_eval_code"));
    }
}
