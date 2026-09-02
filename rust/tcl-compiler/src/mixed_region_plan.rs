// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target-neutral, mixed per-region execution planning.
//!
//! A [`MixedRegionPlan`] is keyed by the executable semantic IR's stable
//! [`NodeId`] values. It records an exact generic slow path for every command
//! invocation, plus guarded direct and intrinsic candidate outcomes. Lowered
//! operations and opaque compatibility regions remain first-class entries, so
//! one function can mix all of these shapes without selecting a whole-function
//! backend mode.
//!
//! The current builder is deliberately conservative. [`InvocationFacts`]
//! describe registry semantics, not a live interpreter binding or procedure
//! identity, so the builder selects generic invocation over the already-built
//! argv and records typed guarded-candidate declines. A later common analysis
//! may replace those decisions only after producing the corresponding proof.

use std::collections::{BTreeMap, btree_map::Entry};

use tcl_dialect::TclVersion;
use tcl_registry::{
    CompletionCode, CompletionCodeDomain, CompletionPayloadObligations, DispatchDependencies,
    IntrinsicId, InvocationFacts, SemanticOperationId,
};

use crate::backend_registry::{
    BackendPlanKind, SelectionFacts, SelectionInput, SelectionProofFailure, SelectionRegion,
    guard_domains_for_dispatch, specialisation_proof_failures,
};
use crate::executable_ir::{
    CompletionId, ExecutableArgvId, ExecutableFunction, ExecutableFunctionId,
    ExecutableInstruction, ExecutableIrValidationError, InvocationResolution,
};
use crate::ir::NodeId;
use crate::representation_plan::{GuardCondition, GuardIdentity, GuardedPlan, GuardedPlanError};
use crate::semantic_analysis::CommonAnalysisProvenance;

/// A target-neutral execution plan containing independently planned regions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixedRegionPlan {
    function: ExecutableFunctionId,
    regions: BTreeMap<NodeId, RegionPlan>,
}

impl MixedRegionPlan {
    /// Build the most precise plan justified by the current executable IR.
    ///
    /// Generic invocations reuse their existing argv. Static registry
    /// resolution is retained as candidate evidence, but never promoted to a
    /// guarded or direct selection without a live common-analysis proof.
    pub fn build(function: &ExecutableFunction) -> Result<Self, MixedPlanBuildError> {
        function
            .validate()
            .map_err(MixedPlanBuildError::InvalidExecutableIr)?;

        let mut regions = BTreeMap::new();
        for instruction in function
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
        {
            let (node, plan) = match instruction {
                ExecutableInstruction::Invoke(invoke) => (
                    invoke.node.clone(),
                    RegionPlan::Invocation(Box::new(InvocationRegionPlan::from_invoke(invoke))),
                ),
                ExecutableInstruction::ExecuteLowered(operation) => (
                    operation.node.clone(),
                    RegionPlan::Lowered(LoweredRegionPlan {
                        completion: operation.completion,
                        operation: operation.semantic_operation(),
                    }),
                ),
                ExecutableInstruction::ExecuteOpaqueRegion(region) => (
                    region.node.clone(),
                    RegionPlan::Opaque(OpaqueRegionPlan {
                        completion: region.completion,
                        operation: region.semantic_operation(),
                    }),
                ),
                ExecutableInstruction::CompleteStructuredRegion(region) => (
                    region.node.clone(),
                    RegionPlan::Structured(StructuredRegionPlan {
                        completion: region.completion,
                        operation: region.semantic_operation(),
                        kind: region.kind,
                    }),
                ),
                ExecutableInstruction::EvaluateWord { .. }
                | ExecutableInstruction::ExpandWord { .. }
                | ExecutableInstruction::BuildArgv { .. }
                | ExecutableInstruction::EvaluateExpr { .. }
                | ExecutableInstruction::MatchPattern { .. }
                | ExecutableInstruction::IterateLists { .. }
                | ExecutableInstruction::JoinCompletion { .. }
                | ExecutableInstruction::WriteCompletionCell { .. } => continue,
            };
            match regions.entry(node) {
                Entry::Vacant(entry) => {
                    entry.insert(plan);
                }
                Entry::Occupied(entry) => {
                    return Err(MixedPlanBuildError::DuplicateRegionNode(
                        entry.key().clone(),
                    ));
                }
            }
        }

        let plan = Self {
            function: function.id,
            regions,
        };
        plan.validate_against(function)
            .map_err(MixedPlanBuildError::InvalidPlan)?;
        Ok(plan)
    }

    /// The executable function that owns every region identity in this plan.
    #[must_use]
    pub const fn function(&self) -> ExecutableFunctionId {
        self.function
    }

    /// Return the plan for one stable source-semantic node.
    #[must_use]
    pub fn region(&self, node: &NodeId) -> Option<&RegionPlan> {
        self.regions.get(node)
    }

    /// Iterate regions in stable structural-node order.
    pub fn regions(&self) -> impl Iterator<Item = (&NodeId, &RegionPlan)> {
        self.regions.iter()
    }

    /// Refine resolved boxed intrinsics using common proof obligations.
    ///
    /// This method is invoked only by semantic analysis after explicit pass
    /// enablement. It does not alter executable IR or emit target code: it
    /// replaces a generic decision with proof-bearing guarded evidence, while
    /// retaining the exact prebuilt-argv fallback.
    pub(crate) fn select_guarded_boxed_intrinsics(
        &self,
        function: &ExecutableFunction,
        runtime_version: Option<TclVersion>,
        provenance: &CommonAnalysisProvenance,
    ) -> Result<Self, MixedPlanValidationError> {
        let mut refined = self.clone();
        for instruction in function
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
        {
            let ExecutableInstruction::Invoke(invoke) = instruction else {
                continue;
            };
            let Some(RegionPlan::Invocation(region)) = refined.regions.get_mut(&invoke.node) else {
                continue;
            };
            let InvocationResolution::Resolved(facts) = &invoke.resolution else {
                continue;
            };
            refine_guarded_boxed_intrinsic(region, facts, runtime_version, provenance);
        }
        refined.validate_against(function)?;
        Ok(refined)
    }

    /// Validate this immutable plan against the executable function it plans.
    ///
    /// In particular, an invocation slow path must name the exact argv and
    /// completion produced by the source invocation. Since
    /// [`PrebuiltArgvSlowPath`] has no source-word payload, a validated plan has
    /// no representation capable of replaying substitutions.
    pub fn validate_against(
        &self,
        function: &ExecutableFunction,
    ) -> Result<(), MixedPlanValidationError> {
        function
            .validate()
            .map_err(MixedPlanValidationError::InvalidExecutableIr)?;
        if self.function != function.id {
            return Err(MixedPlanValidationError::FunctionMismatch {
                expected: function.id,
                actual: self.function,
            });
        }

        let mut expected = BTreeMap::new();
        for instruction in function
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
        {
            let node = match instruction {
                ExecutableInstruction::Invoke(invoke) => &invoke.node,
                ExecutableInstruction::ExecuteLowered(operation) => &operation.node,
                ExecutableInstruction::ExecuteOpaqueRegion(region) => &region.node,
                ExecutableInstruction::CompleteStructuredRegion(region) => &region.node,
                ExecutableInstruction::EvaluateWord { .. }
                | ExecutableInstruction::ExpandWord { .. }
                | ExecutableInstruction::BuildArgv { .. }
                | ExecutableInstruction::EvaluateExpr { .. }
                | ExecutableInstruction::MatchPattern { .. }
                | ExecutableInstruction::IterateLists { .. }
                | ExecutableInstruction::JoinCompletion { .. }
                | ExecutableInstruction::WriteCompletionCell { .. } => continue,
            };
            if expected.insert(node.clone(), instruction).is_some() {
                return Err(MixedPlanValidationError::DuplicateExecutableNode(
                    node.clone(),
                ));
            }
        }

        for (node, instruction) in &expected {
            let region = self
                .regions
                .get(node)
                .ok_or_else(|| MixedPlanValidationError::MissingRegion(node.clone()))?;
            validate_region(node, region, instruction)?;
        }
        if let Some(node) = self
            .regions
            .keys()
            .find(|node| !expected.contains_key(*node))
        {
            return Err(MixedPlanValidationError::UnexpectedRegion(node.clone()));
        }
        Ok(())
    }
}

/// One planned semantic region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionPlan {
    /// A command invocation with one exact generic slow path.
    Invocation(Box<InvocationRegionPlan>),
    /// A registry-described structural operation already lowered in common IR.
    Lowered(LoweredRegionPlan),
    /// A conservative compatibility boundary whose interior is not projected.
    Opaque(OpaqueRegionPlan),
    /// A structured control region projected into executable edges.
    Structured(StructuredRegionPlan),
}

impl RegionPlan {
    /// Completion triple produced by this entire region.
    #[must_use]
    pub const fn completion(&self) -> CompletionId {
        match self {
            Self::Invocation(plan) => plan.completion,
            Self::Lowered(plan) => plan.completion,
            Self::Opaque(plan) => plan.completion,
            Self::Structured(plan) => plan.completion,
        }
    }

    /// Stable target-neutral class selected for this region.
    #[must_use]
    pub const fn selected_kind(&self) -> RegionSelectionKind {
        match self {
            Self::Invocation(plan) => match &plan.selection {
                InvocationSelection::GenericPrebuiltArgv => {
                    RegionSelectionKind::GenericPrebuiltArgv
                }
                InvocationSelection::GuardedIntrinsic(_) => RegionSelectionKind::GuardedIntrinsic,
                InvocationSelection::GuardedDirect(_) => RegionSelectionKind::GuardedDirect,
            },
            Self::Lowered(_) => RegionSelectionKind::Lowered,
            Self::Opaque(_) => RegionSelectionKind::Opaque,
            Self::Structured(_) => RegionSelectionKind::Structured,
        }
    }

    /// Registry semantic operation retained for this region, when known.
    #[must_use]
    pub const fn operation(&self) -> Option<SemanticOperationId> {
        match self {
            Self::Invocation(plan) => plan.operation,
            Self::Lowered(plan) => Some(plan.operation),
            Self::Opaque(plan) => plan.operation,
            Self::Structured(plan) => Some(plan.operation),
        }
    }
}

/// Stable selected execution class for one mixed-plan region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionSelectionKind {
    /// Generic runtime dispatch consumes the prebuilt argv.
    GenericPrebuiltArgv,
    /// A registry intrinsic was selected under runtime guards.
    GuardedIntrinsic,
    /// A direct implementation was selected under runtime guards.
    GuardedDirect,
    /// A registry-described structural operation remains lowered.
    Lowered,
    /// A conservative opaque compatibility boundary remains in place.
    Opaque,
    /// Structured control projected into executable edges.
    Structured,
}

impl RegionSelectionKind {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GenericPrebuiltArgv => "generic-prebuilt-argv",
            Self::GuardedIntrinsic => "guarded-intrinsic",
            Self::GuardedDirect => "guarded-direct",
            Self::Lowered => "lowered",
            Self::Opaque => "opaque",
            Self::Structured => "structured",
        }
    }
}

/// A generic fallback that consumes an argv already evaluated by common IR.
///
/// This type intentionally carries neither [`crate::ir::WordExpr`] nor source
/// text. A slow path cannot use it to repeat substitutions, expansion, traces,
/// or other effects from word evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrebuiltArgvSlowPath {
    argv: ExecutableArgvId,
    completion: CompletionId,
}

impl PrebuiltArgvSlowPath {
    /// The exact argv constructed before dispatch selection and guards.
    #[must_use]
    pub const fn argv(&self) -> ExecutableArgvId {
        self.argv
    }

    /// The exact completion triple produced by invoking this argv.
    #[must_use]
    pub const fn completion(&self) -> CompletionId {
        self.completion
    }
}

/// Selection for one invocation after considering guarded candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvocationSelection {
    /// Invoke through the generic runtime path with the region's prebuilt argv.
    GenericPrebuiltArgv,
    /// Execute a registry intrinsic under common dispatch-stability evidence.
    GuardedIntrinsic(GuardedSelectionEvidence),
    /// Call a proved procedure or other direct implementation under guards.
    GuardedDirect(GuardedSelectionEvidence),
}

/// Common proof summary attached to a selected guarded candidate.
///
/// Fields are private because static registry facts cannot construct this
/// evidence. A future common proof pass will provide a provenance-gated
/// constructor when live identity and guard-domain analysis is available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardedSelectionEvidence {
    operation: SemanticOperationId,
    dispatch_dependencies: DispatchDependencies,
    runtime_version: TclVersion,
    guarded_plan: GuardedPlan<SemanticOperationId, PrebuiltArgvSlowPath>,
}

impl GuardedSelectionEvidence {
    /// Registry semantic operation whose live identity was proven.
    #[must_use]
    pub const fn operation(&self) -> SemanticOperationId {
        self.operation
    }

    /// Mutable dispatch domains covered by the common proof or guard plan.
    #[must_use]
    pub const fn dispatch_dependencies(&self) -> DispatchDependencies {
        self.dispatch_dependencies
    }

    /// Runtime Tcl release whose registry semantics the identity attests.
    #[must_use]
    pub const fn runtime_version(&self) -> TclVersion {
        self.runtime_version
    }

    /// Complete guarded fast/slow plan authorised by common analysis.
    #[must_use]
    pub const fn guarded_plan(&self) -> &GuardedPlan<SemanticOperationId, PrebuiltArgvSlowPath> {
        &self.guarded_plan
    }
}

/// Guarded specialisation class considered for an invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuardedCandidateKind {
    /// A registry-described target-neutral intrinsic.
    Intrinsic,
    /// A direct call to a proved procedure or other implementation identity.
    Direct,
}

impl GuardedCandidateKind {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intrinsic => "guarded-intrinsic",
            Self::Direct => "guarded-direct",
        }
    }
}

/// Result of considering one guarded specialisation class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardedCandidateEvidence {
    kind: GuardedCandidateKind,
    decision: GuardedCandidateDecision,
}

impl GuardedCandidateEvidence {
    /// Candidate class whose decision this evidence records.
    #[must_use]
    pub const fn kind(&self) -> GuardedCandidateKind {
        self.kind
    }

    /// Typed selection or decline produced by common planning.
    #[must_use]
    pub const fn decision(&self) -> &GuardedCandidateDecision {
        &self.decision
    }
}

/// Selection result for one guarded candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardedCandidateDecision {
    /// Common analysis proved this candidate and selected it.
    Selected(GuardedSelectionEvidence),
    /// Common analysis did not authorise this candidate.
    Declined(GuardedCandidateDecline),
}

/// Typed reason a guarded direct or intrinsic candidate was not selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardedCandidateDecline {
    /// The command head did not yield registry-owned invocation facts.
    InvocationUnresolved,
    /// Registry facts selected another semantic-operation class.
    OperationIsNotIntrinsic {
        /// Operation actually selected by the registry.
        operation: SemanticOperationId,
    },
    /// Executable invocation facts do not identify a direct callable target.
    DirectCalleeIdentityUnavailable,
    /// The semantic optimisation pass was not explicitly enabled.
    PassDisabled,
    /// This first boxed-intrinsic slice does not yet claim this intrinsic.
    UnsupportedIntrinsic {
        /// Registry-owned intrinsic identity that remains on the generic path.
        intrinsic: IntrinsicId,
    },
    /// The intrinsic does not have the exact produced completion contract
    /// implemented by this boxed fast path.
    CompletionContractUnsupported,
    /// The selected dialect has no concrete Tcl runtime semantics to attest.
    RuntimeSemanticsUnavailable,
    /// Shared backend-registry legality obligations were not satisfied.
    ProofObligations(Vec<SelectionProofFailure>),
    /// Common analysis could not form a non-empty runtime guard request.
    GuardPlanInvalid(GuardedPlanError),
    /// Static registry resolution has no live dispatch-stability proof.
    DispatchStabilityUnavailable {
        /// Mutable domains that a future proof or guard must cover.
        dependencies: DispatchDependencies,
    },
}

impl GuardedCandidateDecline {
    /// Stable Explorer/API spelling.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::InvocationUnresolved => "invocation-unresolved",
            Self::OperationIsNotIntrinsic { .. } => "operation-is-not-intrinsic",
            Self::DirectCalleeIdentityUnavailable => "direct-callee-identity-unavailable",
            Self::PassDisabled => "pass-disabled",
            Self::UnsupportedIntrinsic { .. } => "unsupported-intrinsic",
            Self::CompletionContractUnsupported => "completion-contract-unsupported",
            Self::RuntimeSemanticsUnavailable => "runtime-semantics-unavailable",
            Self::ProofObligations(_) => "proof-obligations-unsatisfied",
            Self::GuardPlanInvalid(_) => "guard-plan-invalid",
            Self::DispatchStabilityUnavailable { .. } => "dispatch-stability-unavailable",
        }
    }
}

/// Target-neutral plan for one generic or guarded invocation region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationRegionPlan {
    completion: CompletionId,
    operation: Option<SemanticOperationId>,
    slow_path: PrebuiltArgvSlowPath,
    selection: InvocationSelection,
    candidates: [GuardedCandidateEvidence; 2],
}

impl InvocationRegionPlan {
    fn from_invoke(invoke: &crate::executable_ir::GenericInvoke) -> Self {
        let (operation, candidates) = match &invoke.resolution {
            InvocationResolution::Resolved(facts) => {
                (Some(facts.operation), candidates_from_facts(facts))
            }
            InvocationResolution::Unresolved(_) => (None, unresolved_candidates()),
        };
        Self {
            completion: invoke.completion,
            operation,
            slow_path: PrebuiltArgvSlowPath {
                argv: invoke.argv,
                completion: invoke.completion,
            },
            selection: InvocationSelection::GenericPrebuiltArgv,
            candidates,
        }
    }

    /// Completion triple produced by either the selected path or its fallback.
    #[must_use]
    pub const fn completion(&self) -> CompletionId {
        self.completion
    }

    /// Registry semantic operation selected statically, when resolution succeeded.
    #[must_use]
    pub const fn operation(&self) -> Option<SemanticOperationId> {
        self.operation
    }

    /// Exact generic fallback shared by guarded and non-guarded selections.
    #[must_use]
    pub const fn slow_path(&self) -> &PrebuiltArgvSlowPath {
        &self.slow_path
    }

    /// Execution class currently selected for this invocation.
    #[must_use]
    pub const fn selection(&self) -> &InvocationSelection {
        &self.selection
    }

    /// Guarded candidates in the common direct-then-intrinsic ladder order.
    #[must_use]
    pub const fn candidates(&self) -> &[GuardedCandidateEvidence; 2] {
        &self.candidates
    }
}

/// Exact plan for a common structural lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoweredRegionPlan {
    completion: CompletionId,
    operation: SemanticOperationId,
}

impl LoweredRegionPlan {
    /// Completion produced by this lowered operation.
    #[must_use]
    pub const fn completion(&self) -> CompletionId {
        self.completion
    }

    /// Registry-owned target-neutral operation identity.
    #[must_use]
    pub const fn operation(&self) -> SemanticOperationId {
        self.operation
    }
}

/// Exact plan for one structured control region with executable edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuredRegionPlan {
    completion: CompletionId,
    operation: SemanticOperationId,
    kind: crate::executable_ir::StructuredRegionKind,
}

impl StructuredRegionPlan {
    /// Completion produced by the region as a whole.
    #[must_use]
    pub const fn completion(&self) -> CompletionId {
        self.completion
    }

    /// Registry-owned structural identity of the region.
    #[must_use]
    pub const fn operation(&self) -> SemanticOperationId {
        self.operation
    }

    /// The executable shape the region was projected into.
    #[must_use]
    pub const fn kind(&self) -> crate::executable_ir::StructuredRegionKind {
        self.kind
    }
}

/// Exact plan for one opaque common-IR compatibility boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpaqueRegionPlan {
    completion: CompletionId,
    operation: Option<SemanticOperationId>,
}

impl OpaqueRegionPlan {
    /// Completion produced by executing the whole opaque region.
    #[must_use]
    pub const fn completion(&self) -> CompletionId {
        self.completion
    }

    /// Retained structural identity, if the earlier lowering preserved it.
    #[must_use]
    pub const fn operation(&self) -> Option<SemanticOperationId> {
        self.operation
    }
}

fn candidates_from_facts(facts: &InvocationFacts) -> [GuardedCandidateEvidence; 2] {
    let direct = GuardedCandidateEvidence {
        kind: GuardedCandidateKind::Direct,
        decision: GuardedCandidateDecision::Declined(
            GuardedCandidateDecline::DirectCalleeIdentityUnavailable,
        ),
    };
    let intrinsic = GuardedCandidateEvidence {
        kind: GuardedCandidateKind::Intrinsic,
        decision: match facts.operation {
            SemanticOperationId::Intrinsic(_) => {
                GuardedCandidateDecision::Declined(GuardedCandidateDecline::PassDisabled)
            }
            operation => GuardedCandidateDecision::Declined(
                GuardedCandidateDecline::OperationIsNotIntrinsic { operation },
            ),
        },
    };
    [direct, intrinsic]
}

fn refine_guarded_boxed_intrinsic(
    region: &mut InvocationRegionPlan,
    facts: &InvocationFacts,
    runtime_version: Option<TclVersion>,
    provenance: &CommonAnalysisProvenance,
) {
    let SemanticOperationId::Intrinsic(intrinsic) = facts.operation else {
        return;
    };
    let intrinsic_candidate = &mut region.candidates[1];
    debug_assert_eq!(intrinsic_candidate.kind, GuardedCandidateKind::Intrinsic);

    // The first slice is deliberately narrow. `StringLength` consumes and
    // returns boxed Tcl values through the runtime intrinsic contract; no
    // native representation, frame, or suspension state crosses this region.
    if intrinsic != IntrinsicId::StringLength {
        intrinsic_candidate.decision =
            GuardedCandidateDecision::Declined(GuardedCandidateDecline::UnsupportedIntrinsic {
                intrinsic,
            });
        return;
    }
    if !completion_is_exact_produced_ok(facts) {
        intrinsic_candidate.decision = GuardedCandidateDecision::Declined(
            GuardedCandidateDecline::CompletionContractUnsupported,
        );
        return;
    }
    let Some(runtime_version) = runtime_version else {
        intrinsic_candidate.decision = GuardedCandidateDecision::Declined(
            GuardedCandidateDecline::RuntimeSemanticsUnavailable,
        );
        return;
    };

    let domains = guard_domains_for_dispatch(facts.dispatch_dependencies);
    let guard = GuardCondition::from_analysis(
        GuardIdentity::registry_intrinsic_with_semantics(
            intrinsic.stable_id(),
            intrinsic.guard_semantics_key(runtime_version),
        ),
        domains,
        provenance,
    );
    let guarded_plan =
        match GuardedPlan::from_analysis(guard, facts.operation, region.slow_path, provenance) {
            Ok(plan) => plan,
            Err(error) => {
                intrinsic_candidate.decision = GuardedCandidateDecision::Declined(
                    GuardedCandidateDecline::GuardPlanInvalid(error),
                );
                return;
            }
        };

    let selection_facts = SelectionFacts::from_invocation(facts)
        .with_no_representation_obligations(provenance)
        .with_guard_plan(&guarded_plan);
    let input = SelectionInput::with_facts(
        facts.operation,
        SelectionRegion::PrebuiltArgvInvocation,
        selection_facts,
    );
    let failures = specialisation_proof_failures(BackendPlanKind::RuntimeIntrinsic, &input);
    if !failures.is_empty() {
        intrinsic_candidate.decision =
            GuardedCandidateDecision::Declined(GuardedCandidateDecline::ProofObligations(failures));
        return;
    }

    let evidence = GuardedSelectionEvidence {
        operation: facts.operation,
        dispatch_dependencies: facts.dispatch_dependencies,
        runtime_version,
        guarded_plan,
    };
    intrinsic_candidate.decision = GuardedCandidateDecision::Selected(evidence.clone());
    region.selection = InvocationSelection::GuardedIntrinsic(evidence);
}

fn completion_is_exact_produced_ok(facts: &InvocationFacts) -> bool {
    matches!(
        facts.completion.codes,
        CompletionCodeDomain::Exact(codes) if codes == [CompletionCode::Ok]
    ) && facts.completion.payloads == CompletionPayloadObligations::PRODUCED
}

fn unresolved_candidates() -> [GuardedCandidateEvidence; 2] {
    [
        GuardedCandidateEvidence {
            kind: GuardedCandidateKind::Direct,
            decision: GuardedCandidateDecision::Declined(
                GuardedCandidateDecline::InvocationUnresolved,
            ),
        },
        GuardedCandidateEvidence {
            kind: GuardedCandidateKind::Intrinsic,
            decision: GuardedCandidateDecision::Declined(
                GuardedCandidateDecline::InvocationUnresolved,
            ),
        },
    ]
}

fn validate_region(
    node: &NodeId,
    region: &RegionPlan,
    instruction: &ExecutableInstruction,
) -> Result<(), MixedPlanValidationError> {
    match (region, instruction) {
        (RegionPlan::Invocation(plan), ExecutableInstruction::Invoke(invoke)) => {
            validate_invocation(node, plan, invoke)
        }
        (RegionPlan::Lowered(plan), ExecutableInstruction::ExecuteLowered(operation)) => {
            if plan.completion != operation.completion {
                return Err(completion_mismatch(
                    node,
                    operation.completion,
                    plan.completion,
                ));
            }
            let expected = operation.semantic_operation();
            if plan.operation != expected {
                return Err(MixedPlanValidationError::OperationMismatch {
                    node: node.clone(),
                    expected: Some(expected),
                    actual: Some(plan.operation),
                });
            }
            Ok(())
        }
        (RegionPlan::Opaque(plan), ExecutableInstruction::ExecuteOpaqueRegion(region)) => {
            if plan.completion != region.completion {
                return Err(completion_mismatch(
                    node,
                    region.completion,
                    plan.completion,
                ));
            }
            let expected = region.semantic_operation();
            if plan.operation != expected {
                return Err(MixedPlanValidationError::OperationMismatch {
                    node: node.clone(),
                    expected,
                    actual: plan.operation,
                });
            }
            Ok(())
        }
        (RegionPlan::Structured(plan), ExecutableInstruction::CompleteStructuredRegion(region)) => {
            if plan.completion != region.completion {
                return Err(completion_mismatch(
                    node,
                    region.completion,
                    plan.completion,
                ));
            }
            let expected = region.semantic_operation();
            if plan.operation != expected || plan.kind != region.kind {
                return Err(MixedPlanValidationError::OperationMismatch {
                    node: node.clone(),
                    expected: Some(expected),
                    actual: Some(plan.operation),
                });
            }
            Ok(())
        }
        _ => Err(MixedPlanValidationError::RegionKindMismatch(node.clone())),
    }
}

fn validate_invocation(
    node: &NodeId,
    plan: &InvocationRegionPlan,
    invoke: &crate::executable_ir::GenericInvoke,
) -> Result<(), MixedPlanValidationError> {
    if plan.completion != invoke.completion {
        return Err(completion_mismatch(
            node,
            invoke.completion,
            plan.completion,
        ));
    }
    if plan.slow_path.argv != invoke.argv {
        return Err(MixedPlanValidationError::SlowPathArgvMismatch {
            node: node.clone(),
            expected: invoke.argv,
            actual: plan.slow_path.argv,
        });
    }
    if plan.slow_path.completion != invoke.completion {
        return Err(MixedPlanValidationError::SlowPathCompletionMismatch {
            node: node.clone(),
            expected: invoke.completion,
            actual: plan.slow_path.completion,
        });
    }

    let expected_operation = match &invoke.resolution {
        InvocationResolution::Resolved(facts) => Some(facts.operation),
        InvocationResolution::Unresolved(_) => None,
    };
    if plan.operation != expected_operation {
        return Err(MixedPlanValidationError::OperationMismatch {
            node: node.clone(),
            expected: expected_operation,
            actual: plan.operation,
        });
    }

    let intrinsic = candidate(plan, node, GuardedCandidateKind::Intrinsic)?;
    let direct = candidate(plan, node, GuardedCandidateKind::Direct)?;
    match &plan.selection {
        InvocationSelection::GenericPrebuiltArgv => {
            if matches!(intrinsic.decision, GuardedCandidateDecision::Selected(_))
                || matches!(direct.decision, GuardedCandidateDecision::Selected(_))
            {
                return Err(MixedPlanValidationError::SelectedCandidateOnGenericPath(
                    node.clone(),
                ));
            }
        }
        InvocationSelection::GuardedIntrinsic(evidence) => {
            validate_selected_candidate(node, intrinsic, evidence)?;
        }
        InvocationSelection::GuardedDirect(evidence) => {
            validate_selected_candidate(node, direct, evidence)?;
        }
    }

    if let InvocationResolution::Resolved(facts) = &invoke.resolution {
        for candidate in [intrinsic, direct] {
            if let GuardedCandidateDecision::Selected(evidence) = &candidate.decision {
                if evidence.operation != facts.operation {
                    return Err(MixedPlanValidationError::CandidateOperationMismatch {
                        node: node.clone(),
                        kind: candidate.kind,
                        expected: facts.operation,
                        actual: evidence.operation,
                    });
                }
                if evidence.dispatch_dependencies != facts.dispatch_dependencies {
                    return Err(MixedPlanValidationError::CandidateDependenciesMismatch {
                        node: node.clone(),
                        kind: candidate.kind,
                    });
                }
                if candidate.kind == GuardedCandidateKind::Intrinsic
                    && !matches!(facts.operation, SemanticOperationId::Intrinsic(_))
                {
                    return Err(MixedPlanValidationError::IntrinsicOperationRequired(
                        node.clone(),
                    ));
                }
                if candidate.kind == GuardedCandidateKind::Intrinsic {
                    validate_intrinsic_guarded_plan(node, evidence, facts, &plan.slow_path)?;
                }
            }
        }
    } else if matches!(intrinsic.decision, GuardedCandidateDecision::Selected(_))
        || matches!(direct.decision, GuardedCandidateDecision::Selected(_))
    {
        return Err(MixedPlanValidationError::GuardedUnresolvedInvocation(
            node.clone(),
        ));
    }
    Ok(())
}

fn candidate<'a>(
    plan: &'a InvocationRegionPlan,
    node: &NodeId,
    kind: GuardedCandidateKind,
) -> Result<&'a GuardedCandidateEvidence, MixedPlanValidationError> {
    let mut matches = plan
        .candidates
        .iter()
        .filter(|candidate| candidate.kind == kind);
    let Some(candidate) = matches.next() else {
        return Err(MixedPlanValidationError::MissingCandidate {
            node: node.clone(),
            kind,
        });
    };
    if matches.next().is_some() {
        return Err(MixedPlanValidationError::DuplicateCandidate {
            node: node.clone(),
            kind,
        });
    }
    Ok(candidate)
}

fn validate_selected_candidate(
    node: &NodeId,
    candidate: &GuardedCandidateEvidence,
    selected: &GuardedSelectionEvidence,
) -> Result<(), MixedPlanValidationError> {
    match &candidate.decision {
        GuardedCandidateDecision::Selected(evidence) if evidence == selected => Ok(()),
        GuardedCandidateDecision::Selected(_) | GuardedCandidateDecision::Declined(_) => {
            Err(MixedPlanValidationError::SelectionEvidenceMismatch {
                node: node.clone(),
                kind: candidate.kind,
            })
        }
    }
}

fn validate_intrinsic_guarded_plan(
    node: &NodeId,
    evidence: &GuardedSelectionEvidence,
    facts: &InvocationFacts,
    slow_path: &PrebuiltArgvSlowPath,
) -> Result<(), MixedPlanValidationError> {
    let SemanticOperationId::Intrinsic(intrinsic) = facts.operation else {
        return Err(MixedPlanValidationError::IntrinsicOperationRequired(
            node.clone(),
        ));
    };
    if evidence.guarded_plan.fast() != &facts.operation
        || evidence.guarded_plan.guard().expected_identity()
            != GuardIdentity::registry_intrinsic_with_semantics(
                intrinsic.stable_id(),
                intrinsic.guard_semantics_key(evidence.runtime_version),
            )
    {
        return Err(MixedPlanValidationError::GuardIdentityMismatch(
            node.clone(),
        ));
    }
    if evidence.guarded_plan.guard().domains()
        != guard_domains_for_dispatch(facts.dispatch_dependencies)
    {
        return Err(MixedPlanValidationError::GuardDomainsMismatch(node.clone()));
    }
    if evidence.guarded_plan.slow() != slow_path {
        return Err(MixedPlanValidationError::GuardedSlowPathMismatch(
            node.clone(),
        ));
    }
    Ok(())
}

fn completion_mismatch(
    node: &NodeId,
    expected: CompletionId,
    actual: CompletionId,
) -> MixedPlanValidationError {
    MixedPlanValidationError::CompletionMismatch {
        node: node.clone(),
        expected,
        actual,
    }
}

/// Failure while constructing a mixed plan from executable IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixedPlanBuildError {
    /// The source executable IR failed its own validation.
    InvalidExecutableIr(ExecutableIrValidationError),
    /// Two executable regions claimed the same function-local semantic node.
    DuplicateRegionNode(NodeId),
    /// The newly built plan failed its internal cross-check.
    InvalidPlan(MixedPlanValidationError),
}

impl MixedPlanBuildError {
    /// Stable Explorer/API spelling for this construction decline.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidExecutableIr(_) => "invalid-executable-ir",
            Self::DuplicateRegionNode(_) => "duplicate-region-node",
            Self::InvalidPlan(_) => "invalid-mixed-plan",
        }
    }
}

/// A malformed or stale mixed-region plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixedPlanValidationError {
    /// The source executable IR failed its own validation.
    InvalidExecutableIr(ExecutableIrValidationError),
    /// The plan and source function have different identities.
    FunctionMismatch {
        /// Function supplied for validation.
        expected: ExecutableFunctionId,
        /// Function identity stored in the plan.
        actual: ExecutableFunctionId,
    },
    /// Two executable regions claimed the same stable node identity.
    DuplicateExecutableNode(NodeId),
    /// The executable function has a region absent from the plan.
    MissingRegion(NodeId),
    /// The plan contains a region absent from the executable function.
    UnexpectedRegion(NodeId),
    /// The plan's region class differs from the executable instruction class.
    RegionKindMismatch(NodeId),
    /// The plan changed the first-class Tcl completion identity.
    CompletionMismatch {
        /// Region whose identity changed.
        node: NodeId,
        /// Completion produced by executable IR.
        expected: CompletionId,
        /// Completion stored by the plan.
        actual: CompletionId,
    },
    /// A generic or guarded fallback did not reuse the invocation's argv.
    SlowPathArgvMismatch {
        /// Invocation region with the invalid fallback.
        node: NodeId,
        /// Already-evaluated argv from executable IR.
        expected: ExecutableArgvId,
        /// Argv named by the slow path.
        actual: ExecutableArgvId,
    },
    /// A generic or guarded fallback changed the invocation completion.
    SlowPathCompletionMismatch {
        /// Invocation region with the invalid fallback.
        node: NodeId,
        /// Invocation completion from executable IR.
        expected: CompletionId,
        /// Completion named by the slow path.
        actual: CompletionId,
    },
    /// A lowered or opaque plan changed its registry semantic operation.
    OperationMismatch {
        /// Region whose operation changed.
        node: NodeId,
        /// Operation retained by executable IR.
        expected: Option<SemanticOperationId>,
        /// Operation stored by the plan.
        actual: Option<SemanticOperationId>,
    },
    /// One required guarded candidate class is absent.
    MissingCandidate {
        /// Invocation missing the candidate.
        node: NodeId,
        /// Required candidate class.
        kind: GuardedCandidateKind,
    },
    /// A guarded candidate class occurs more than once.
    DuplicateCandidate {
        /// Invocation containing duplicate evidence.
        node: NodeId,
        /// Duplicated candidate class.
        kind: GuardedCandidateKind,
    },
    /// Generic invocation was selected while a candidate claimed selection.
    SelectedCandidateOnGenericPath(NodeId),
    /// The selected plan and its candidate record carry different evidence.
    SelectionEvidenceMismatch {
        /// Invocation with inconsistent evidence.
        node: NodeId,
        /// Selected candidate class.
        kind: GuardedCandidateKind,
    },
    /// Candidate evidence names a semantic operation different from the registry facts.
    CandidateOperationMismatch {
        /// Invocation with stale candidate evidence.
        node: NodeId,
        /// Candidate class.
        kind: GuardedCandidateKind,
        /// Operation in the current invocation facts.
        expected: SemanticOperationId,
        /// Operation in the candidate evidence.
        actual: SemanticOperationId,
    },
    /// Candidate guard coverage differs from the registry dependency contract.
    CandidateDependenciesMismatch {
        /// Invocation with incomplete or stale dependency evidence.
        node: NodeId,
        /// Candidate class.
        kind: GuardedCandidateKind,
    },
    /// An intrinsic candidate was selected for a non-intrinsic operation.
    IntrinsicOperationRequired(NodeId),
    /// Guarded intrinsic evidence names another stable implementation.
    GuardIdentityMismatch(NodeId),
    /// Guarded intrinsic evidence does not exactly match canonical dependency domains.
    GuardDomainsMismatch(NodeId),
    /// A guarded intrinsic does not retain the invocation's exact prebuilt fallback.
    GuardedSlowPathMismatch(NodeId),
    /// A guarded candidate was selected for an unresolved command head.
    GuardedUnresolvedInvocation(NodeId),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::model::semantic::SemanticContext;
    use tcl_registry::{CommandRegistry, StateTransitionKnowledge};

    use crate::executable_ir::{ExecutableFunctionId, build_linear_executable_ir};
    use crate::lowering::lower_to_ir;

    fn executable(source: &str, id: usize) -> ExecutableFunction {
        let registry = CommandRegistry::build_default();
        let module = lower_to_ir(source, &registry);
        build_linear_executable_ir(
            &registry,
            Some(SemanticContext::for_environment("tcl9.0")),
            ExecutableFunctionId::new(id),
            &module.top_level,
        )
        .expect("fixture must fit executable IR")
    }

    #[test]
    fn builds_multiple_invocation_lowered_and_structured_regions() {
        let function = executable(
            "puts one\nset value 2\nif {$enabled} {puts enabled}\nputs two",
            80,
        );
        let plan = MixedRegionPlan::build(&function).expect("mixed plan");
        let regions: Vec<_> = plan.regions().collect();

        // The `if` is a structured region whose body statement is a region of
        // its own, nested under the region's node.
        assert_eq!(regions.len(), 5);
        assert_eq!(regions[0].0.path(), &[0]);
        assert!(matches!(regions[0].1, RegionPlan::Invocation(_)));
        assert_eq!(regions[1].0.path(), &[1]);
        assert!(matches!(regions[1].1, RegionPlan::Lowered(_)));
        assert_eq!(regions[2].0.path(), &[2]);
        assert!(matches!(regions[2].1, RegionPlan::Structured(_)));
        assert_eq!(regions[3].0.path(), &[2, 0, 0]);
        assert!(matches!(regions[3].1, RegionPlan::Invocation(_)));
        assert_eq!(regions[4].0.path(), &[3]);
        assert!(matches!(regions[4].1, RegionPlan::Invocation(_)));

        for (_, region) in [regions[0], regions[4]] {
            let RegionPlan::Invocation(invocation) = region else {
                unreachable!();
            };
            assert_eq!(
                invocation.selection(),
                &InvocationSelection::GenericPrebuiltArgv
            );
            assert_eq!(invocation.slow_path().completion(), invocation.completion());
            assert!(invocation.candidates().iter().all(|candidate| matches!(
                candidate.decision(),
                GuardedCandidateDecision::Declined(_)
            )));
        }
        plan.validate_against(&function).expect("valid mixed plan");
    }

    #[test]
    fn unresolved_invocation_declines_both_guarded_candidates() {
        let function = executable("$command argument", 81);
        let plan = MixedRegionPlan::build(&function).expect("generic plan");
        let RegionPlan::Invocation(invocation) = plan
            .region(&NodeId::from_path(vec![0]))
            .expect("invocation region")
        else {
            panic!("expected invocation");
        };

        assert!(invocation.candidates().iter().all(|candidate| {
            candidate.decision()
                == &GuardedCandidateDecision::Declined(
                    GuardedCandidateDecline::InvocationUnresolved,
                )
        }));
    }

    #[test]
    fn guarded_boxed_intrinsic_retains_identity_domains_and_exact_slow_path() {
        let function = executable("string length value", 85);
        let baseline = MixedRegionPlan::build(&function).expect("baseline plan");
        let node = NodeId::from_path(vec![0]);
        let RegionPlan::Invocation(off) = baseline.region(&node).expect("invocation") else {
            panic!("expected invocation");
        };
        assert_eq!(off.selection(), &InvocationSelection::GenericPrebuiltArgv);
        assert!(matches!(
            off.candidates()[1].decision(),
            GuardedCandidateDecision::Declined(GuardedCandidateDecline::PassDisabled)
        ));

        let provenance = crate::semantic_analysis::test_common_analysis_provenance();
        let plan = baseline
            .select_guarded_boxed_intrinsics(
                &function,
                Some(tcl_dialect::TclVersion::V9_0),
                &provenance,
            )
            .expect("guarded plan");
        let RegionPlan::Invocation(on) = plan.region(&node).expect("invocation") else {
            panic!("expected invocation");
        };
        let InvocationSelection::GuardedIntrinsic(evidence) = on.selection() else {
            panic!("expected guarded intrinsic");
        };
        assert_eq!(
            evidence.guarded_plan().guard().expected_identity(),
            GuardIdentity::registry_intrinsic_with_semantics(
                IntrinsicId::StringLength.stable_id(),
                IntrinsicId::StringLength.guard_semantics_key(tcl_dialect::TclVersion::V9_0),
            )
        );
        assert_eq!(evidence.runtime_version(), tcl_dialect::TclVersion::V9_0);
        assert_eq!(
            evidence.guarded_plan().guard().domains(),
            guard_domains_for_dispatch(evidence.dispatch_dependencies())
        );
        assert_eq!(evidence.guarded_plan().slow(), on.slow_path());
        assert_eq!(
            evidence.guarded_plan().fast(),
            &SemanticOperationId::Intrinsic(IntrinsicId::StringLength)
        );
    }

    #[test]
    fn guarded_intrinsic_declines_when_backend_registry_proof_is_missing() {
        let mut function = executable("string length value", 86);
        let invoke = function
            .blocks
            .iter_mut()
            .flat_map(|block| block.instructions.iter_mut())
            .find_map(|instruction| match instruction {
                ExecutableInstruction::Invoke(invoke) => Some(invoke),
                _ => None,
            })
            .expect("invocation");
        let InvocationResolution::Resolved(facts) = &mut invoke.resolution else {
            panic!("resolved intrinsic");
        };
        facts.state_transitions = StateTransitionKnowledge::UnknownInvocation;

        let baseline = MixedRegionPlan::build(&function).expect("baseline plan");
        let provenance = crate::semantic_analysis::test_common_analysis_provenance();
        let plan = baseline
            .select_guarded_boxed_intrinsics(
                &function,
                Some(tcl_dialect::TclVersion::V9_0),
                &provenance,
            )
            .expect("typed decline remains valid");
        let RegionPlan::Invocation(invocation) = plan
            .region(&NodeId::from_path(vec![0]))
            .expect("invocation")
        else {
            panic!("expected invocation");
        };
        assert_eq!(
            invocation.selection(),
            &InvocationSelection::GenericPrebuiltArgv
        );
        assert!(matches!(
            invocation.candidates()[1].decision(),
            GuardedCandidateDecision::Declined(GuardedCandidateDecline::ProofObligations(
                failures
            )) if failures.contains(&SelectionProofFailure::UnknownStateTransitions)
        ));
    }

    #[test]
    fn guarded_intrinsic_declines_without_concrete_runtime_semantics() {
        let function = executable("string length value", 87);
        let baseline = MixedRegionPlan::build(&function).expect("baseline plan");
        let provenance = crate::semantic_analysis::test_common_analysis_provenance();
        let plan = baseline
            .select_guarded_boxed_intrinsics(&function, None, &provenance)
            .expect("typed decline remains valid");
        let RegionPlan::Invocation(invocation) = plan
            .region(&NodeId::from_path(vec![0]))
            .expect("invocation")
        else {
            panic!("expected invocation");
        };
        assert_eq!(
            invocation.candidates()[1].decision(),
            &GuardedCandidateDecision::Declined(
                GuardedCandidateDecline::RuntimeSemanticsUnavailable,
            )
        );
    }

    #[test]
    fn validation_rejects_a_different_slow_path_argv() {
        let function = executable("puts one\nputs two", 82);
        let mut plan = MixedRegionPlan::build(&function).expect("mixed plan");
        let node = NodeId::from_path(vec![0]);
        let RegionPlan::Invocation(invocation) = plan.regions.get_mut(&node).unwrap() else {
            panic!("expected invocation");
        };
        invocation.slow_path.argv = ExecutableArgvId::new(function.id, usize::MAX);

        assert!(matches!(
            plan.validate_against(&function),
            Err(MixedPlanValidationError::SlowPathArgvMismatch { node: actual, .. })
                if actual == node
        ));
    }

    #[test]
    fn validation_rejects_candidate_selection_on_generic_path() {
        let function = executable("string length value", 83);
        let plan = MixedRegionPlan::build(&function).expect("mixed plan");
        let provenance = crate::semantic_analysis::test_common_analysis_provenance();
        let mut plan = plan
            .select_guarded_boxed_intrinsics(
                &function,
                Some(tcl_dialect::TclVersion::V9_0),
                &provenance,
            )
            .expect("proved guarded intrinsic");
        let node = NodeId::from_path(vec![0]);
        let RegionPlan::Invocation(invocation) = plan.regions.get_mut(&node).unwrap() else {
            panic!("expected invocation");
        };
        invocation.selection = InvocationSelection::GenericPrebuiltArgv;

        assert_eq!(
            plan.validate_against(&function),
            Err(MixedPlanValidationError::SelectedCandidateOnGenericPath(
                node
            ))
        );
    }

    #[test]
    fn build_rejects_duplicate_node_keys_across_regions() {
        let mut function = executable("puts one\nputs two", 84);
        let duplicate = NodeId::from_path(vec![0]);
        let second = function
            .blocks
            .iter_mut()
            .flat_map(|block| &mut block.instructions)
            .filter_map(|instruction| match instruction {
                ExecutableInstruction::Invoke(invoke) => Some(invoke),
                _ => None,
            })
            .nth(1)
            .expect("second invocation");
        second.node = duplicate.clone();

        assert_eq!(
            MixedRegionPlan::build(&function),
            Err(MixedPlanBuildError::DuplicateRegionNode(duplicate))
        );
    }
}
