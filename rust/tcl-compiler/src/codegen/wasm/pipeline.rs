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

use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{CommandRegistry, IntrinsicId, SemanticOperationId};

use crate::analyses::{ConstValue, LatticeValue};
use crate::backend_registry::{
    BackendDeclineReason, BackendPlanKind, BackendRegistry, BackendSelection, BackendSelector,
    SelectionFacts, SelectionInput, SelectionRegion, SelectorDecision, SelectorPriority,
    SelectorRequest,
};
use crate::common_aot_plan::{
    ClosedProgramCoverageDecision, ClosedProgramStatementEvidence, CommonAotEnvironment,
    CommonAotProofPlan, DirectActualValue, DirectCallSiteId, DirectProcBodyDecision,
    DirectProcDecision, MaterialisableSlotDecision, SemanticCallArgument, SemanticCallDecision,
};
use crate::compilation_unit::CompilationUnit;
use crate::executable_ir::{
    ExecutableFunction, ExecutableInstruction, InvocationResolution, SourceCompatibilityDecline,
};
use crate::mixed_region_plan::{GuardedSelectionEvidence, InvocationSelection, RegionPlan};
use crate::native_integer_proof::{
    NativeAddDecision, NativeAddExecution, NativeAddResult, NativeIntegerPolicy,
    NativeIntegerProof, prove_native_integer_adds,
};
use crate::native_lowering::NativeTierReport;
use crate::semantic_analysis::{ExecutableAnalysisAvailability, MixedRegionPlanAvailability};
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::target_contract::{
    LegalisationRequirements, TargetCapabilities, TargetContract, TargetFamily,
};
use crate::types::TypeShape;

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
    semantic_optimisations: SemanticOptimisationConfig,
    common_aot_environment: CommonAotEnvironment,
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
            semantic_optimisations: SemanticOptimisationConfig::new(),
            common_aot_environment: CommonAotEnvironment::Hosted,
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

    /// State that the compiled module owns the complete interpreter lifetime.
    ///
    /// This is an explicit proof premise for native-only storage, rather than
    /// a general WASM target preference. Hosted modules remain conservative.
    #[must_use]
    pub const fn for_sealed_program(mut self) -> Self {
        self.common_aot_environment = CommonAotEnvironment::SealedProgram;
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

    /// The native tier: lower every function through NLIR
    /// (`crate::native_lowering`) with representation inference,
    /// trace-barrier elision, and cell demotion, and emit it natively.
    ///
    /// Shorthand for the four passes it enables, and the same set the
    /// `native-tier` group in
    /// [`PASS_GROUPS`](crate::semantic_optimisation::PASS_GROUPS) names —
    /// `native_tier_group_matches_the_wasm_option` keeps the two in step.
    ///
    /// The tier harness's native plan uses it unconditionally. `tcl compwasm`
    /// and the Explorer reach it through `--optimise` / their pass toggles;
    /// **both default to no optimisation at all**, so an unflagged
    /// `tcl compwasm` and the Explorer's `wasm` view show the generic
    /// lowering.
    #[must_use]
    pub const fn native_tier(self) -> Self {
        self.with_semantic_optimisation(SemanticOptimisationPassId::NativeLowering)
            .with_semantic_optimisation(SemanticOptimisationPassId::RepresentationInference)
            .with_semantic_optimisation(SemanticOptimisationPassId::TraceBarrierElision)
            .with_semantic_optimisation(SemanticOptimisationPassId::CellDemotion)
    }

    /// Return a copy that explicitly enables one semantic/AOT optimisation pass.
    ///
    /// This affects only optional specialisation. Generic argv invocation and
    /// general structured lowering remain available when the configuration is
    /// empty.
    #[must_use]
    pub const fn with_semantic_optimisation(mut self, pass: SemanticOptimisationPassId) -> Self {
        self.semantic_optimisations.enable(pass);
        self
    }

    /// Return a copy with the complete semantic/AOT optimisation configuration.
    ///
    /// This lets a target-neutral compilation policy choose its complete pass
    /// set before selecting a backend. [`Self::with_semantic_optimisation`]
    /// remains the convenience for a narrowly scoped opt-in.
    #[must_use]
    pub const fn with_semantic_optimisations(
        mut self,
        semantic_optimisations: SemanticOptimisationConfig,
    ) -> Self {
        self.semantic_optimisations = semantic_optimisations;
        self
    }

    /// Target-neutral semantic/AOT optimisation configuration for this build.
    #[must_use]
    pub const fn semantic_optimisations(self) -> SemanticOptimisationConfig {
        self.semantic_optimisations
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
            && self
                .semantic_optimisations
                .is_enabled(SemanticOptimisationPassId::LegacyAnalysisSpecialisation)
    }

    /// Whether the native tier's lowering may replace general emission.
    pub(super) const fn native_tier_enabled(self) -> bool {
        matches!(self.plan_policy, WasmPlanPolicy::SemanticFirst)
            && self
                .semantic_optimisations
                .is_enabled(SemanticOptimisationPassId::NativeLowering)
    }

    pub(super) const fn common_aot_environment(self) -> CommonAotEnvironment {
        self.common_aot_environment
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
    /// No resolved environment was carried for executable analysis.
    ContextUnavailable,
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
            Self::ContextUnavailable => "context-unavailable",
            Self::Source(_) => "source-shape-declined",
            Self::SourceUnavailable => "source-unavailable",
        }
    }

    /// Stable precise reason within the availability class.
    #[must_use]
    pub const fn detail_kind(&self) -> &'static str {
        match self {
            Self::ContextUnavailable => "context-unavailable",
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
    /// A sealed-program proof selected native i64 direct addition and a
    /// registry-proved boxed boundary materialisation.
    NativeI64Add {
        /// Immutable common-proof selection consumed by the emitter.
        native: WasmNativeI64AddSelection,
        /// Target-neutral region evidence retained for Explorer consumers.
        region_plan: MixedRegionPlanAvailability,
    },
    /// Executable IR selected generic prebuilt-argv invocation.
    GenericInvoke {
        /// Registry-owned semantic operation retained by the selected plan.
        operation: SemanticOperationId,
        /// Target-neutral evidence for every executable semantic region.
        region_plan: MixedRegionPlanAvailability,
    },
    /// General structured lowering ran in the same emitter.
    General {
        /// Typed reason the narrower semantic input mode was not selected.
        semantic_decline: WasmSemanticDecline,
        /// Target-neutral region evidence retained even when WASM selection declines.
        region_plan: MixedRegionPlanAvailability,
    },
}

impl WasmCodegenPlan {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::NativeI64Add { .. } => "native-i64-add",
            Self::GenericInvoke { .. } => "generic-invoke",
            Self::General { .. } => "general",
        }
    }

    /// Typed reason the narrow semantic input mode declined.
    #[must_use]
    pub const fn semantic_decline(&self) -> Option<&WasmSemanticDecline> {
        match self {
            Self::NativeI64Add { .. } | Self::GenericInvoke { .. } => None,
            Self::General {
                semantic_decline, ..
            } => Some(semantic_decline),
        }
    }

    /// Stable operation category selected by common semantic facts.
    #[must_use]
    pub const fn operation_kind(&self) -> Option<&'static str> {
        match self {
            Self::NativeI64Add { native, .. } => Some(native.boundary_operation.kind_str()),
            Self::GenericInvoke {
                operation: SemanticOperationId::Invoke,
                ..
            } => Some("invoke"),
            Self::GenericInvoke {
                operation: SemanticOperationId::Intrinsic(_),
                ..
            } => Some("intrinsic"),
            Self::GenericInvoke {
                operation: SemanticOperationId::StructuredLowering(_),
                ..
            } => Some("structured-lowering"),
            Self::General { .. } => None,
        }
    }

    /// Target-neutral selection and decline evidence for each stable region.
    #[must_use]
    pub const fn region_plan(&self) -> &MixedRegionPlanAvailability {
        match self {
            Self::NativeI64Add { region_plan, .. }
            | Self::GenericInvoke { region_plan, .. }
            | Self::General { region_plan, .. } => region_plan,
        }
    }
}

/// Exhaustively checked input to the native i64 addition emitter.
///
/// Constructed only from common proof objects. The backend receives values and
/// an already-resolved direct function identity, never Tcl command spellings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmNativeI64AddSelection {
    /// Direct procedure identity selected by the common binding proof.
    pub callee: crate::common_aot_plan::ProcedureIdentity,
    /// Exact first operand from covered constant and range proofs.
    pub left: i64,
    /// Exact second operand from covered constant and range proofs.
    pub right: i64,
    /// Registry-owned semantic operation at the boxed result boundary.
    pub boundary_operation: SemanticOperationId,
    /// The common direct-call proof discharged Tcl frame materialisation.
    pub frame_elided: bool,
    /// Exact number of closed top-level statements consumed by this selection.
    pub closed_program_statements: u32,
}

/// Canonical code-generation artifact and durable selection evidence.
#[derive(Debug, Clone)]
pub struct WasmCompilation {
    /// Generated module consumed by encoding, rendering, linking, and bundling.
    pub module: WasmModule,
    /// Selected semantic or general plan.
    pub plan: WasmCodegenPlan,
    /// The native tier's per-function lowering and elision record.
    pub native: NativeTierReport,
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
    let region_plan = unit
        .top_level
        .semantic_facts
        .mixed_plan_with_optimisations(options.semantic_optimisations());
    if let Some(native) = select_native_i64_add_plan(unit, registry, options) {
        let (module, report) = backend::emit_wasm(
            unit,
            registry,
            options,
            WasmEmissionMode::NativeI64Add(&native),
        );
        return WasmCompilation {
            module,
            plan: WasmCodegenPlan::NativeI64Add {
                native,
                region_plan,
            },
            native: report,
        };
    }
    let (semantic_plan, evidence) = match select_semantic_plan(unit, options) {
        Ok(plan) => match validate_plan_layout(&plan, options.data_base) {
            Ok(()) => {
                let operation = plan.operation;
                (
                    Some(plan),
                    WasmCodegenPlan::GenericInvoke {
                        operation,
                        region_plan: region_plan.clone(),
                    },
                )
            }
            Err(decline) => (
                None,
                WasmCodegenPlan::General {
                    semantic_decline: WasmSemanticDecline::PlanLayout(decline),
                    region_plan: region_plan.clone(),
                },
            ),
        },
        Err(semantic_decline) => (
            None,
            WasmCodegenPlan::General {
                semantic_decline,
                region_plan: region_plan.clone(),
            },
        ),
    };
    let mode = semantic_plan
        .as_ref()
        .map_or(WasmEmissionMode::General, |plan| {
            guarded_intrinsic_evidence(&region_plan, plan)
                .map_or(WasmEmissionMode::SemanticInvoke(plan), |evidence| {
                    WasmEmissionMode::GuardedIntrinsic { plan, evidence }
                })
        });
    let (module, native) = backend::emit_wasm(unit, registry, options, mode);
    WasmCompilation {
        module,
        plan: evidence,
        native,
    }
}

fn select_native_i64_add_plan(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    options: WasmCompileOptions,
) -> Option<WasmNativeI64AddSelection> {
    let config = options.semantic_optimisations();
    let required = [
        SemanticOptimisationPassId::DirectProc,
        SemanticOptimisationPassId::MaterialisableSlot,
        SemanticOptimisationPassId::FrameElision,
        SemanticOptimisationPassId::NativeInteger,
        SemanticOptimisationPassId::SemanticOperationSpecialisation,
    ];
    if !matches!(options.plan_policy, WasmPlanPolicy::SemanticFirst)
        || options.is_standalone()
        || options.common_aot_environment() != CommonAotEnvironment::SealedProgram
        || required.into_iter().any(|pass| !config.is_enabled(pass))
    {
        return None;
    }

    let common = CommonAotProofPlan::build(
        unit,
        registry,
        unit.top_level.semantic_facts.context(),
        config,
        options.common_aot_environment(),
    );
    if !common.coverage_declines().is_empty() {
        return None;
    }
    common.direct_calls().find_map(|(site, decision)| {
        let DirectProcDecision::Selected(direct) = decision else {
            return None;
        };
        let DirectProcBodyDecision::Selected(_) = &direct.body else {
            return None;
        };
        let (covered_left, covered_right, closed_program_statements) =
            selected_closed_native_coverage(&common, site, direct)?;
        if !direct.frame_elidable
            || !direct.frame_escape_private
            || direct.formals.len() != 2
            || direct.actual_values.len() != 2
            || !direct.actual_values.iter().all(|actual| {
                matches!(actual, DirectActualValue::Ssa(value) if selected_materialisable_identity(&common, value))
            })
        {
            return None;
        }

        let boundary_operation = selected_native_boundary(&common, site)?;
        let NativeIntegerProof::Analysed(decisions) = prove_native_integer_adds(
            unit,
            &direct.callee.qualified_name,
            registry,
            config,
            NativeIntegerPolicy::default(),
            &common,
        ) else {
            return None;
        };
        let NativeAddDecision::Proven(native) = decisions.as_slice().first()? else {
            return None;
        };
        if decisions.len() != 1
            || native.site.result != NativeAddResult::FunctionReturn
            || native.execution != NativeAddExecution::OverflowImpossible
            || native.composition.direct_calls.as_slice() != [site.clone()]
            || !native.composition.requires_frame_plan
            || !native.composition.requires_internal_operation_guard_or_sealed_policy
            || !selected_materialisable_int(
                &common,
                &direct.callee.qualified_name,
                native.left.value,
            )
            || !selected_materialisable_int(
                &common,
                &direct.callee.qualified_name,
                native.right.value,
            )
        {
            return None;
        }
        let left = exact_i64(native.left.range)?;
        let right = exact_i64(native.right.range)?;
        if (left, right) != (covered_left, covered_right) {
            return None;
        }
        Some(WasmNativeI64AddSelection {
            callee: direct.callee.clone(),
            left,
            right,
            boundary_operation,
            frame_elided: true,
            closed_program_statements,
        })
    })
}

fn selected_materialisable_int(
    common: &CommonAotProofPlan,
    function: &str,
    value: crate::ssa::ValueKey,
) -> bool {
    common.materialisable_slots().any(|(identity, decision)| {
        identity.function == function
            && (identity.symbol, identity.version) == value
            && matches!(
                decision,
                MaterialisableSlotDecision::Selected(evidence) if evidence.shape == TypeShape::Int
            )
    })
}

fn selected_materialisable_identity(
    common: &CommonAotProofPlan,
    value: &crate::common_aot_plan::SsaValueIdentity,
) -> bool {
    common.materialisable_slots().any(|(identity, decision)| {
        identity == value
            && matches!(
                decision,
                MaterialisableSlotDecision::Selected(evidence) if evidence.shape == TypeShape::Int
            )
    })
}

fn selected_closed_native_coverage(
    common: &CommonAotProofPlan,
    direct_site: &DirectCallSiteId,
    direct: &crate::common_aot_plan::DirectProcEvidence,
) -> Option<(i64, i64, u32)> {
    let coverage = match common.closed_program_coverage() {
        ClosedProgramCoverageDecision::Selected(coverage) => coverage,
        ClosedProgramCoverageDecision::Declined(_) => return None,
    };
    let [
        ClosedProgramStatementEvidence::DirectProcedureDefinition { procedure, .. },
        ClosedProgramStatementEvidence::DirectActualConstant {
            value: first,
            constant: left,
            operation: left_operation,
            ..
        },
        ClosedProgramStatementEvidence::DirectActualConstant {
            value: second,
            constant: right,
            operation: right_operation,
            ..
        },
        ClosedProgramStatementEvidence::SemanticBoundary { call, operation },
    ] = coverage.statements.as_slice()
    else {
        return None;
    };
    if !(procedure == &direct.callee
        && matches!(
            direct.actual_values.as_slice(),
            [DirectActualValue::Ssa(left), DirectActualValue::Ssa(right)] if left == first && right == second
        )
        && call.function == direct_site.function
        && call.block == direct_site.block
        && call.statement_index == direct_site.statement_index
        && call.nested_argument.is_none()
        && *operation == SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite)
        && *left_operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set)
        && *right_operation == SemanticOperationId::StructuredLowering(LoweringHookId::Set))
    {
        return None;
    }
    let statement_count = u32::try_from(coverage.statements.len()).ok()?;
    Some((lattice_i64(left)?, lattice_i64(right)?, statement_count))
}

fn lattice_i64(value: &LatticeValue) -> Option<i64> {
    match value {
        LatticeValue::Const(ConstValue::Int(value)) => Some(*value),
        LatticeValue::Unknown
        | LatticeValue::Const(ConstValue::Float(_) | ConstValue::Bool(_) | ConstValue::String(_))
        | LatticeValue::ConstSet(_)
        | LatticeValue::Overdefined => None,
    }
}

fn selected_native_boundary(
    common: &CommonAotProofPlan,
    direct_site: &DirectCallSiteId,
) -> Option<SemanticOperationId> {
    common.semantic_calls().find_map(|(_, decision)| {
        let SemanticCallDecision::Selected(evidence) = decision else {
            return None;
        };
        (evidence.operation == SemanticOperationId::Intrinsic(IntrinsicId::ChannelWrite)
            && matches!(
                evidence.arguments.as_slice(),
                [SemanticCallArgument::NestedDirect { outer_argument: 0, call }] if call == direct_site
            ))
        .then_some(evidence.operation)
    })
}

fn exact_i64(interval: crate::intervals::Interval) -> Option<i64> {
    match (interval.lo, interval.hi) {
        (Some(value), Some(same)) if value == same => Some(value),
        _ => None,
    }
}

/// Return common guarded-intrinsic evidence only when it describes the same
/// single semantic operation selected for the prebuilt-argv emitter. The
/// region plan has already validated that its slow path is the executable
/// invocation's exact argv/completion pair; the backend merely consumes it.
fn guarded_intrinsic_evidence<'a>(
    regions: &'a MixedRegionPlanAvailability,
    plan: &WasmGenericInvokePlan,
) -> Option<&'a GuardedSelectionEvidence> {
    regions.plan()?.regions().find_map(|(node, region)| {
        let RegionPlan::Invocation(invocation) = region else {
            return None;
        };
        let InvocationSelection::GuardedIntrinsic(evidence) = invocation.selection() else {
            return None;
        };
        (node == &plan.node
            && evidence.operation() == plan.operation
            && evidence.guarded_plan().slow().argv() == plan.argv
            && evidence.guarded_plan().slow().completion() == plan.completion)
            .then_some(evidence)
    })
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
            ExecutableAnalysisAvailability::ContextUnavailable => {
                WasmExecutableAvailabilityDecline::ContextUnavailable
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
            | ExecutableInstruction::ExecuteOpaqueRegion(_)
            | ExecutableInstruction::EvaluateExpr { .. }
            | ExecutableInstruction::MatchPattern { .. }
            | ExecutableInstruction::IterateLists { .. }
            | ExecutableInstruction::JoinCompletion { .. }
            | ExecutableInstruction::WriteCompletionCell { .. }
            | ExecutableInstruction::CompleteStructuredRegion(_) => None,
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
            | ExecutableInstruction::ExecuteOpaqueRegion(_)
            | ExecutableInstruction::EvaluateExpr { .. }
            | ExecutableInstruction::MatchPattern { .. }
            | ExecutableInstruction::IterateLists { .. }
            | ExecutableInstruction::JoinCompletion { .. }
            | ExecutableInstruction::WriteCompletionCell { .. }
            | ExecutableInstruction::CompleteStructuredRegion(_) => None,
        })
        .unwrap_or_else(SelectionFacts::unavailable)
}

#[cfg(test)]
mod tests {
    use super::super::{ValType, WasmOp};
    use super::*;
    use tcl_runtime_api::codegen_abi::CodegenAbiImportId;

    fn unit(source: &str, registry: &CommandRegistry) -> CompilationUnit {
        CompilationUnit::build_for_dialect(source, registry, false, "tcl8.6")
    }

    /// Count the `call` sites reaching one runtime import.
    fn calls_to(module: &WasmModule, import: CodegenAbiImportId) -> usize {
        let Some(index) = module
            .imports
            .iter()
            .position(|candidate| candidate.name == import.descriptor().name)
        else {
            return 0;
        };
        module
            .functions
            .iter()
            .flat_map(|function| &function.body)
            .filter(|instruction| {
                instruction.op == WasmOp::Call
                    && super::super::encoding::decode_leb128_unsigned(&instruction.operands)
                        == index as u64
            })
            .count()
    }

    #[test]
    fn the_native_tier_emits_the_t0_program_without_source_or_expression_rungs() {
        use crate::native_lowering::FunctionStatus;
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for_dialect(
            "set x 10\nset y 3\nset z [expr {$x * $y + 7}]\nincr z -1\nputs $z\n",
            &registry,
            false,
            "tcl9.0",
        );
        let output = compile_wasm(
            &unit,
            &registry,
            WasmCompileOptions::standalone(true).native_tier(),
        );
        assert!(output.native.enabled);
        assert_eq!(
            output.native.function("::top").map(|report| &report.status),
            Some(&FunctionStatus::Lowered)
        );
        assert_eq!(calls_to(&output.module, CodegenAbiImportId::EvalCode), 0);
        assert_eq!(calls_to(&output.module, CodegenAbiImportId::ExprBool), 0);
        assert_eq!(calls_to(&output.module, CodegenAbiImportId::InvokeArgv), 0);
        assert_eq!(calls_to(&output.module, CodegenAbiImportId::Puts), 1);
        assert_eq!(
            calls_to(&output.module, CodegenAbiImportId::ActivationEnter),
            1
        );
        let top = output
            .module
            .functions
            .iter()
            .find(|function| function.name == "::top")
            .expect("native top");
        assert_eq!(top.kind, "native-top");
        let native_ops = top
            .body
            .iter()
            .filter(|instruction| {
                matches!(
                    instruction.op,
                    WasmOp::I64Add | WasmOp::I64Mul | WasmOp::I64Sub
                )
            })
            .count();
        assert!(native_ops >= 3, "{native_ops} native i64 operations");
        // The binary encodes and every function still validates its shape.
        let mut module = output.module;
        assert_eq!(&module.to_bytes()[..4], b"\0asm");
    }

    #[test]
    fn the_native_tier_records_a_declined_function_and_keeps_the_legacy_walk() {
        use crate::native_lowering::{FunctionDecline, FunctionStatus};
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for_dialect(
            "foreach x {a b} {puts $x}\n",
            &registry,
            false,
            "tcl9.0",
        );
        let output = compile_wasm(&unit, &registry, WasmCompileOptions::hosted().native_tier());
        assert_eq!(
            output.native.function("::top").map(|report| &report.status),
            Some(&FunctionStatus::Declined(
                FunctionDecline::UnloweredInstruction("operand-expression")
            ))
        );
        assert!(calls_to(&output.module, CodegenAbiImportId::EvalCode) >= 1);
    }

    #[test]
    fn the_native_tier_is_off_by_default() {
        let registry = CommandRegistry::build_default();
        let unit = unit("set a 1\nputs $a\n", &registry);
        let output = compile_wasm(&unit, &registry, WasmCompileOptions::hosted());
        assert!(!output.native.enabled);
        assert!(output.native.functions.is_empty());
        assert!(!WasmCompileOptions::hosted().native_tier_enabled());
        assert!(
            WasmCompileOptions::hosted()
                .native_tier()
                .native_tier_enabled()
        );
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
    fn guarded_intrinsic_enablement_emits_runtime_guarded_argv_path() {
        let registry = CommandRegistry::build_default();
        let source = unit("string length hello", &registry);
        let mut off = compile_wasm(&source, &registry, WasmCompileOptions::hosted());
        let mut on = compile_wasm(
            &source,
            &registry,
            WasmCompileOptions::hosted()
                .with_semantic_optimisation(SemanticOptimisationPassId::GuardedIntrinsic),
        );

        let off_regions = off.plan.region_plan().plan().expect("off region plan");
        let on_regions = on.plan.region_plan().plan().expect("on region plan");
        let node = crate::ir::NodeId::from_path(vec![0]);
        assert_eq!(
            off_regions.region(&node).unwrap().selected_kind(),
            crate::mixed_region_plan::RegionSelectionKind::GenericPrebuiltArgv
        );
        assert_eq!(
            on_regions.region(&node).unwrap().selected_kind(),
            crate::mixed_region_plan::RegionSelectionKind::GuardedIntrinsic
        );
        let off_wat = off.to_wat();
        let on_wat = on.to_wat();
        assert!(off_wat.contains("tcl_invoke_argv"), "{off_wat}");
        assert!(!off_wat.contains("tcl_codegen_guard_"), "{off_wat}");
        assert!(!off_wat.contains("tcl_intrinsic_invoke_argv"), "{off_wat}");
        assert!(!off_wat.contains("tcl_value_new_wide_int"), "{off_wat}");
        for import in [
            "tcl_codegen_guard_prepare",
            "tcl_codegen_guard_check",
            "tcl_codegen_guard_release",
            "tcl_intrinsic_invoke_argv",
        ] {
            assert!(on_wat.contains(import), "missing {import}: {on_wat}");
        }
        assert!(on_wat.contains("i64.eqz"), "{on_wat}");
        assert!(on_wat.contains("tcl_invoke_argv"), "{on_wat}");
    }

    fn native_i64_options() -> WasmCompileOptions {
        [
            SemanticOptimisationPassId::DirectProc,
            SemanticOptimisationPassId::MaterialisableSlot,
            SemanticOptimisationPassId::FrameElision,
            SemanticOptimisationPassId::NativeInteger,
            SemanticOptimisationPassId::SemanticOperationSpecialisation,
        ]
        .into_iter()
        .fold(
            WasmCompileOptions::hosted().for_sealed_program(),
            WasmCompileOptions::with_semantic_optimisation,
        )
    }

    const SEALED_NATIVE_ADD: &str =
        "proc add {b c} {return [expr {$b + $c}]}\nset d 2\nset e 4\nputs [add $d $e]\n";

    #[test]
    fn sealed_common_proofs_emit_native_i64_add_at_a_boxed_boundary() {
        let registry = CommandRegistry::build_default();
        let unit =
            CompilationUnit::build_for_dialect(SEALED_NATIVE_ADD, &registry, false, "tcl9.0");
        let mut output = compile_wasm(&unit, &registry, native_i64_options());

        assert!(matches!(output.plan, WasmCodegenPlan::NativeI64Add { .. }));
        let bytes = output.to_bytes();
        assert_eq!(
            &bytes[..4],
            b"\0asm",
            "native path must use the shared encoder"
        );
        let add = output
            .functions
            .iter()
            .find(|function| function.name == "::add")
            .expect("selected native function");
        assert_eq!(add.params, vec![ValType::I64, ValType::I64]);
        assert_eq!(add.results, vec![ValType::I64]);
        assert!(
            !add.exported,
            "raw native helper must remain module-private"
        );
        let top = output
            .functions
            .iter()
            .find(|function| function.name == "::top")
            .expect("native top-level function");
        assert_eq!(top.locals, vec![ValType::I32]);
        assert!(top.body.windows(6).any(|window| {
            window.iter().map(|instruction| instruction.op).eq([
                WasmOp::LocalSet,
                WasmOp::LocalGet,
                WasmOp::If,
                WasmOp::Return,
                WasmOp::End,
                WasmOp::Return,
            ])
        }));
        let wat = output.to_wat();
        assert!(wat.contains("tcl_value_new_wide_int"), "{wat}");
        assert!(wat.contains("tcl_codegen_puts"), "{wat}");
        assert!(wat.contains("(func $::add"), "{wat}");
        assert!(!wat.contains("(export \"::add\")"), "{wat}");
        assert!(
            wat.contains("(param $left i64) (param $right i64) (result i64)"),
            "{wat}"
        );
        assert!(wat.contains("i64.add"), "{wat}");
        for absent in [
            "tcl_codegen_frame_push",
            "tcl_codegen_local_bind",
            "tcl_codegen_local_set",
            "tcl_codegen_local_get",
            "tcl_codegen_var_set",
            "tcl_codegen_var_get",
            "tcl_value_new_string",
        ] {
            assert!(!wat.contains(absent), "unexpected {absent}: {wat}");
        }
    }

    #[test]
    fn native_i64_add_requires_every_opt_in_and_a_sealed_environment() {
        let registry = CommandRegistry::build_default();
        let unit =
            CompilationUnit::build_for_dialect(SEALED_NATIVE_ADD, &registry, false, "tcl9.0");
        for pass in [
            SemanticOptimisationPassId::DirectProc,
            SemanticOptimisationPassId::MaterialisableSlot,
            SemanticOptimisationPassId::FrameElision,
            SemanticOptimisationPassId::NativeInteger,
            SemanticOptimisationPassId::SemanticOperationSpecialisation,
        ] {
            let mut config = native_i64_options().semantic_optimisations();
            config.disable(pass);
            let mut output = compile_wasm(
                &unit,
                &registry,
                WasmCompileOptions::hosted()
                    .for_sealed_program()
                    .with_semantic_optimisations(config),
            );
            assert!(
                matches!(output.plan, WasmCodegenPlan::General { .. }),
                "{pass:?}"
            );
            assert!(!output.to_wat().contains("tcl_value_new_wide_int"));
        }
        // Rebuild the exact enabled configuration under the default hosted
        // premise; a target alone cannot authorise native-only state.
        let hosted_options = [
            SemanticOptimisationPassId::DirectProc,
            SemanticOptimisationPassId::MaterialisableSlot,
            SemanticOptimisationPassId::FrameElision,
            SemanticOptimisationPassId::NativeInteger,
            SemanticOptimisationPassId::SemanticOperationSpecialisation,
        ]
        .into_iter()
        .fold(WasmCompileOptions::hosted(), |options, pass| {
            options.with_semantic_optimisation(pass)
        });
        let mut hosted = compile_wasm(&unit, &registry, hosted_options);
        assert!(matches!(hosted.plan, WasmCodegenPlan::General { .. }));
        assert!(!hosted.to_wat().contains("tcl_value_new_wide_int"));

        let mut eval_only = compile_wasm(
            &unit,
            &registry,
            native_i64_options().for_eval_only_test_host(),
        );
        assert!(matches!(eval_only.plan, WasmCodegenPlan::General { .. }));
        let eval_only_wat = eval_only.to_wat();
        assert!(!eval_only_wat.contains("tcl_value_new_wide_int"));
        assert!(!eval_only_wat.contains("tcl_codegen_puts"));
    }

    #[test]
    fn native_i64_add_declines_when_an_extra_top_level_statement_survives() {
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for_dialect(
            "puts before\nproc add {b c} {return [expr {$b + $c}]}\nset d 2\nset e 4\nputs [add $d $e]\n",
            &registry,
            false,
            "tcl9.0",
        );
        let mut output = compile_wasm(&unit, &registry, native_i64_options());
        assert!(matches!(output.plan, WasmCodegenPlan::General { .. }));
        let wat = output.to_wat();
        assert!(!wat.contains("tcl_value_new_wide_int"), "{wat}");
        assert!(wat.contains("tcl_eval_code"), "{wat}");
    }

    #[test]
    fn sealed_native_i64_add_preserves_standalone_bootstrap_decline() {
        let registry = CommandRegistry::build_default();
        let unit = unit(SEALED_NATIVE_ADD, &registry);
        let options = [
            SemanticOptimisationPassId::DirectProc,
            SemanticOptimisationPassId::MaterialisableSlot,
            SemanticOptimisationPassId::FrameElision,
            SemanticOptimisationPassId::NativeInteger,
            SemanticOptimisationPassId::SemanticOperationSpecialisation,
        ]
        .into_iter()
        .fold(
            WasmCompileOptions::standalone(false).for_sealed_program(),
            WasmCompileOptions::with_semantic_optimisation,
        );
        let output = compile_wasm(&unit, &registry, options);
        assert!(matches!(output.plan, WasmCodegenPlan::General { .. }));
        assert!(
            output
                .functions
                .iter()
                .any(|function| function.name == "_start")
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
            ..
        } = &output.plan
        else {
            panic!("expected typed backend decline, got {:?}", output.plan);
        };
        assert!(!attempts.is_empty());
        assert!(output.to_wat().contains("tcl_eval_code"));
    }

    #[test]
    fn general_wasm_plan_retains_mixed_region_evidence_without_selecting_it() {
        let registry = CommandRegistry::build_default();
        let mut output = compile_wasm(
            &unit(
                "puts one\nset value 2\nif {$enabled} {puts enabled}\nputs two",
                &registry,
            ),
            &registry,
            WasmCompileOptions::hosted(),
        );

        assert!(matches!(output.plan, WasmCodegenPlan::General { .. }));
        let MixedRegionPlanAvailability::Available(regions) = output.plan.region_plan() else {
            panic!("mixed region evidence must survive WASM selection decline");
        };
        let selected: Vec<_> = regions
            .regions()
            .map(|(node, region)| (node.path(), region.selected_kind().as_str()))
            .collect();
        assert_eq!(
            selected,
            vec![
                (&[0][..], "generic-prebuilt-argv"),
                (&[1][..], "lowered"),
                // The `if` is projected into executable edges, so it plans as a
                // structured region and the invocation in its body is a region
                // of its own nested under it.
                (&[2][..], "structured"),
                (&[2, 0, 0][..], "generic-prebuilt-argv"),
                (&[3][..], "generic-prebuilt-argv"),
            ]
        );
        assert!(output.to_wat().contains("tcl_eval_code"));
    }

    #[test]
    fn unavailable_executable_ir_retains_typed_region_plan_availability() {
        let registry = CommandRegistry::build_default();
        let output = compile_wasm(
            &unit("", &registry),
            &registry,
            WasmCompileOptions::hosted(),
        );

        assert!(matches!(
            output.plan.region_plan(),
            MixedRegionPlanAvailability::ExecutableUnavailable
        ));
        assert!(matches!(
            output.plan.semantic_decline(),
            Some(WasmSemanticDecline::ExecutableUnavailable(
                WasmExecutableAvailabilityDecline::Source(SourceCompatibilityDecline::EmptyScript)
            ))
        ));
    }

    #[test]
    fn standalone_records_packaging_decline_on_the_same_emitter() {
        let registry = CommandRegistry::build_default();
        let output = compile_wasm(
            &unit("puts hello", &registry),
            &registry,
            WasmCompileOptions::standalone(true),
        );

        assert!(matches!(
            output.plan,
            WasmCodegenPlan::General {
                semantic_decline: WasmSemanticDecline::Packaging(
                    WasmPackagingConstraint::StandaloneBootstrap
                ),
                ..
            }
        ));
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

        assert!(matches!(
            output.plan,
            WasmCodegenPlan::General {
                semantic_decline: WasmSemanticDecline::PlanLayout(
                    WasmExecutableInvokeDecline::InvalidDataBase
                ),
                ..
            }
        ));
        assert!(output.to_wat().contains("tcl_eval_code"));
    }
}
