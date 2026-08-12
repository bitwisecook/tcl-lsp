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

//! Immutable target-plan selection keyed by common semantic operations.
//!
//! The [`BackendRegistry`] is deliberately above target emission and below the
//! common semantic compiler.  The command registry resolves Tcl spellings to
//! [`SemanticOperationId`] values; this registry selects a complete immutable
//! backend plan for that operation.  A selector only receives immutable input
//! and returns a plan or a typed decline, so it cannot partially mutate an
//! emitter before another selector takes the fallback path.
//!
//! The fixed ladder is direct/native, runtime intrinsic, generic invocation
//! over a prebuilt argv, and `eval` only for explicitly dynamic scripts or
//! opaque regions.  GPU/FPGA host continuation is deliberately outside the
//! ordinary invocation ladder: it is valid only at an explicit offload-region
//! boundary.

use std::collections::{BTreeMap, btree_map::Entry};

use tcl_registry::{
    CompletionDescriptor, DispatchDependencies, DispatchDependencyDomain, EffectFootprint,
    InvocationFacts, SemanticOperationId, StateTransitionKnowledge,
};
use tcl_runtime_api::guard::{GuardDomain, GuardDomains};

use crate::representation_plan::{
    BoundaryPlan, GuardCondition, GuardedPlan, OwnershipLedger, SuspensionPlan,
};
use crate::target_contract::{
    LegalisationRequirements, RequirementFailure, RuntimeService, TargetContract, TargetFamily,
};

/// The semantic shape of the region requesting a backend plan.
///
/// This is a common-IR property, not a command-name classification.  In
/// particular, [`Self::PrebuiltArgvInvocation`] means word substitution has
/// already constructed argv once; a generic fallback must reuse that argv
/// rather than re-evaluating Tcl words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionRegion {
    /// A resolved command invocation with a prebuilt argv.
    PrebuiltArgvInvocation,
    /// A dynamically generated script requiring Tcl evaluation semantics.
    DynamicScript,
    /// An opaque script region whose Tcl semantics require evaluation.
    OpaqueRegion,
    /// An explicit host/device offload boundary for an enclosing region.
    OffloadBoundary,
}

/// Whether a common analysis proved a backend-selection obligation.
///
/// `Unavailable` is deliberately distinct from `Unsatisfied`: the former says
/// analysis did not establish an answer, while the latter is a known failed
/// obligation. Neither is permission to select a direct or intrinsic plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofStatus {
    /// The relevant common analysis did not provide a proof.
    Unavailable,
    /// Analysis proved that this invocation has no such obligation.
    NotRequired,
    /// A supplied plan was validated successfully.
    Satisfied,
    /// A supplied plan was checked and found not to meet its obligation.
    Unsatisfied,
}

impl ProofStatus {
    const fn permits_specialisation(self) -> bool {
        matches!(self, Self::NotRequired | Self::Satisfied)
    }
}

/// A referenced common plan and the result of validating its obligation.
#[derive(Debug)]
pub struct PlanEvidence<'a, T: ?Sized> {
    plan: Option<&'a T>,
    status: ProofStatus,
}

impl<T: ?Sized> Copy for PlanEvidence<'_, T> {}

impl<T: ?Sized> Clone for PlanEvidence<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: ?Sized> PlanEvidence<'a, T> {
    const fn unavailable() -> Self {
        Self {
            plan: None,
            status: ProofStatus::Unavailable,
        }
    }

    const fn not_required() -> Self {
        Self {
            plan: None,
            status: ProofStatus::NotRequired,
        }
    }

    const fn checked(plan: &'a T, satisfied: bool) -> Self {
        Self {
            plan: Some(plan),
            status: if satisfied {
                ProofStatus::Satisfied
            } else {
                ProofStatus::Unsatisfied
            },
        }
    }

    /// The common plan that was checked, when one was supplied.
    #[must_use]
    pub const fn plan(&self) -> Option<&'a T> {
        self.plan
    }

    /// Whether the proof is unavailable, unnecessary, satisfied, or failed.
    #[must_use]
    pub const fn status(&self) -> ProofStatus {
        self.status
    }
}

/// Type-erased view of a common completion-ownership plan.
///
/// Resource identities remain private to common IR. A backend selector needs
/// only the proof that the ledger balances, not a target-specific handle.
pub trait OwnershipSelectionPlan: std::fmt::Debug + Sync {
    /// Whether every owned value in the plan is released exactly once.
    fn is_satisfied(&self) -> bool;
}

impl<R> OwnershipSelectionPlan for OwnershipLedger<R>
where
    R: Ord + std::fmt::Debug + Sync,
{
    fn is_satisfied(&self) -> bool {
        self.validate_released().is_ok()
    }
}

/// Type-erased view of a validated common guard plan.
pub trait GuardSelectionPlan: std::fmt::Debug + Sync {
    /// Offline runtime guard request protecting the plan's fast path.
    fn guard(&self) -> &GuardCondition;

    /// Whether the runtime guard request covers at least one domain.
    fn is_satisfied(&self) -> bool;
}

/// Proof from common analysis that dispatch dependencies cannot change.
///
/// Constructing this token does not inspect a target or choose an encoding.
/// Its producer must be the common binding/namespace/trace/interpreter
/// analysis that established the closed-world assumption for these domains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedWorldDispatchProof {
    dependencies: DispatchDependencies,
    policy: ClosedWorldDispatchPolicy,
}

/// Common-analysis policy under which dispatch domains were sealed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedWorldDispatchPolicy {
    /// The whole compiled program and every reachable interpreter are sealed.
    WholeProgram,
    /// One interpreter is sealed against binding, trace, and policy mutation.
    SealedInterpreter,
}

impl ClosedWorldDispatchProof {
    /// Domains covered by this closed-world proof.
    #[must_use]
    pub const fn dependencies(&self) -> &DispatchDependencies {
        &self.dependencies
    }

    /// Sealed-environment policy used to establish the proof.
    #[must_use]
    pub const fn policy(&self) -> ClosedWorldDispatchPolicy {
        self.policy
    }
}

/// Evidence that the registry-resolved runtime dispatch remains valid.
#[derive(Debug, Clone, Copy)]
pub struct DispatchStabilityEvidence<'a> {
    status: ProofStatus,
    guards: Option<&'a (dyn GuardSelectionPlan + 'a)>,
    closed_world: Option<&'a ClosedWorldDispatchProof>,
}

impl DispatchStabilityEvidence<'_> {
    const fn unavailable() -> Self {
        Self {
            status: ProofStatus::Unavailable,
            guards: None,
            closed_world: None,
        }
    }

    /// Whether dispatch stability is unavailable, satisfied, or disproven.
    #[must_use]
    pub const fn status(&self) -> ProofStatus {
        self.status
    }

    /// Guarded fast/slow plan supplying a runtime guard request, when used.
    #[must_use]
    pub const fn guards(&self) -> Option<&dyn GuardSelectionPlan> {
        self.guards
    }

    /// Closed-world dispatch proof, when used instead of runtime guards.
    #[must_use]
    pub const fn closed_world(&self) -> Option<&ClosedWorldDispatchProof> {
        self.closed_world
    }
}

impl<Fast, Slow> GuardSelectionPlan for GuardedPlan<Fast, Slow>
where
    Fast: std::fmt::Debug + Sync,
    Slow: std::fmt::Debug + Sync,
{
    fn guard(&self) -> &GuardCondition {
        self.guard()
    }

    fn is_satisfied(&self) -> bool {
        !self.guard().domains().is_empty()
    }
}

fn guard_domain(dependency: DispatchDependencyDomain) -> GuardDomain {
    match dependency {
        DispatchDependencyDomain::CommandBinding => GuardDomain::CommandEnvironment,
        DispatchDependencyDomain::NamespaceLookup => GuardDomain::Namespace,
        DispatchDependencyDomain::CommandTraces => GuardDomain::CommandTrace,
        DispatchDependencyDomain::InterpreterPolicy => GuardDomain::Interpreter,
        DispatchDependencyDomain::ObjectDispatch => GuardDomain::ObjectDispatch,
        DispatchDependencyDomain::UnknownHandling => GuardDomain::UnknownHandling,
    }
}

/// Canonical runtime guard domains required by registry dispatch dependencies.
///
/// Common guarded planners and backend proof validation share this mapping so
/// binding, namespace, trace, and interpreter invalidation cannot acquire
/// subtly different guard sets in individual optimisation passes.
#[must_use]
pub fn guard_domains_for_dispatch(dependencies: DispatchDependencies) -> GuardDomains {
    dependencies
        .iter()
        .fold(GuardDomains::EMPTY, |domains, dependency| {
            domains.with(guard_domain(dependency))
        })
}

fn guards_cover_dispatch(
    dependencies: DispatchDependencies,
    guarded_domains: GuardDomains,
) -> bool {
    dependencies
        .iter()
        .all(|dependency| guarded_domains.contains(guard_domain(dependency)))
}

fn closed_world_covers_dispatch(
    available: DispatchDependencies,
    required: DispatchDependencies,
) -> bool {
    required
        .iter()
        .all(|dependency| available.contains(dependency))
}

/// Immutable common facts and proof obligations visible to backend selectors.
///
/// The registry invocation is the single source for command semantics,
/// completion, effects, transitions, and argument roles. Representation plans
/// come from common legalisation passes. This interface exposes both without
/// selecting an encoding or authorising an optimisation.
#[derive(Debug, Clone, Copy)]
pub struct SelectionFacts<'a> {
    invocation: Option<&'a InvocationFacts>,
    dispatch: DispatchStabilityEvidence<'a>,
    boundary: PlanEvidence<'a, BoundaryPlan>,
    ownership: PlanEvidence<'a, dyn OwnershipSelectionPlan + 'a>,
    suspension: PlanEvidence<'a, SuspensionPlan>,
}

impl<'a> SelectionFacts<'a> {
    /// Construct a conservative fact bundle with no semantic proofs.
    #[must_use]
    pub const fn unavailable() -> Self {
        Self {
            invocation: None,
            dispatch: DispatchStabilityEvidence::unavailable(),
            boundary: PlanEvidence::unavailable(),
            ownership: PlanEvidence::unavailable(),
            suspension: PlanEvidence::unavailable(),
        }
    }

    /// Start a proof bundle from the registry's complete invocation snapshot.
    #[must_use]
    pub const fn from_invocation(invocation: &'a InvocationFacts) -> Self {
        Self {
            invocation: Some(invocation),
            dispatch: DispatchStabilityEvidence::unavailable(),
            boundary: PlanEvidence::unavailable(),
            ownership: PlanEvidence::unavailable(),
            suspension: PlanEvidence::unavailable(),
        }
    }

    /// Mark every representation obligation as proven unnecessary.
    ///
    /// This is appropriate only when common analyses established that no
    /// runtime boundary, owned completion lifetime, or suspension crosses the
    /// selected operation. Dispatch stability remains a separate mandatory
    /// proof and is never waived by this method.
    #[must_use]
    pub const fn with_no_representation_obligations(
        mut self,
        _provenance: &crate::semantic_analysis::CommonAnalysisProvenance,
    ) -> Self {
        self.boundary = PlanEvidence::not_required();
        self.ownership = PlanEvidence::not_required();
        self.suspension = PlanEvidence::not_required();
        self
    }

    /// Attach and validate a variable materialisation boundary plan.
    #[must_use]
    pub fn with_boundary_plan(
        mut self,
        plan: &'a BoundaryPlan,
        _provenance: &crate::semantic_analysis::CommonAnalysisProvenance,
    ) -> Self {
        self.boundary = PlanEvidence::checked(plan, plan.validate().is_ok());
        self
    }

    /// Attach and validate a completion ownership ledger.
    #[must_use]
    pub fn with_ownership_plan<R>(
        mut self,
        plan: &'a OwnershipLedger<R>,
        _provenance: &crate::semantic_analysis::CommonAnalysisProvenance,
    ) -> Self
    where
        R: Ord + std::fmt::Debug + Sync + 'a,
    {
        let plan: &'a (dyn OwnershipSelectionPlan + 'a) = plan;
        self.ownership = PlanEvidence::checked(plan, plan.is_satisfied());
        self
    }

    /// Attach and validate a suspension preservation plan.
    #[must_use]
    pub fn with_suspension_plan(
        mut self,
        plan: &'a SuspensionPlan,
        _provenance: &crate::semantic_analysis::CommonAnalysisProvenance,
    ) -> Self {
        self.suspension = PlanEvidence::checked(plan, plan.validate().is_ok());
        self
    }

    /// Attach and validate a guarded fast/slow common plan.
    #[must_use]
    pub fn with_guard_plan<Fast, Slow>(mut self, plan: &'a GuardedPlan<Fast, Slow>) -> Self
    where
        Fast: std::fmt::Debug + Sync + 'a,
        Slow: std::fmt::Debug + Sync + 'a,
    {
        let plan: &'a (dyn GuardSelectionPlan + 'a) = plan;
        let satisfied = self.invocation.is_some_and(|invocation| {
            plan.is_satisfied()
                && guards_cover_dispatch(invocation.dispatch_dependencies, plan.guard().domains())
        });
        self.dispatch = DispatchStabilityEvidence {
            status: if satisfied {
                ProofStatus::Satisfied
            } else {
                ProofStatus::Unsatisfied
            },
            guards: Some(plan),
            closed_world: None,
        };
        self
    }

    /// Attach a common-analysis proof that dispatch cannot be invalidated.
    #[must_use]
    pub fn with_closed_world_dispatch(mut self, proof: &'a ClosedWorldDispatchProof) -> Self {
        let satisfied = self.invocation.is_some_and(|invocation| {
            closed_world_covers_dispatch(*proof.dependencies(), invocation.dispatch_dependencies)
        });
        self.dispatch = DispatchStabilityEvidence {
            status: if satisfied {
                ProofStatus::Satisfied
            } else {
                ProofStatus::Unsatisfied
            },
            guards: None,
            closed_world: Some(proof),
        };
        self
    }

    /// Registry-owned invocation facts, if resolution supplied them.
    #[must_use]
    pub const fn invocation(&self) -> Option<&'a InvocationFacts> {
        self.invocation
    }

    /// Registry-owned completion contract, if invocation facts are available.
    #[must_use]
    pub fn completion(&self) -> Option<CompletionDescriptor> {
        self.invocation.map(|facts| facts.completion)
    }

    /// Fully resolved world effects, if invocation facts are available.
    #[must_use]
    pub fn effects(&self) -> Option<&'a EffectFootprint> {
        self.invocation.map(|facts| &facts.effects)
    }

    /// Closed transition facts or typed unknown knowledge from the registry.
    #[must_use]
    pub fn state_transitions(&self) -> Option<&'a StateTransitionKnowledge> {
        self.invocation.map(|facts| &facts.state_transitions)
    }

    /// Dispatch-stability proof kept separate from command transition facts.
    #[must_use]
    pub const fn dispatch(&self) -> &DispatchStabilityEvidence<'a> {
        &self.dispatch
    }

    /// Materialisation-boundary evidence.
    #[must_use]
    pub const fn boundary(&self) -> &PlanEvidence<'a, BoundaryPlan> {
        &self.boundary
    }

    /// Completion-ownership evidence.
    #[must_use]
    pub const fn ownership(&self) -> &PlanEvidence<'a, dyn OwnershipSelectionPlan + 'a> {
        &self.ownership
    }

    /// Suspension-preservation evidence.
    #[must_use]
    pub const fn suspension(&self) -> &PlanEvidence<'a, SuspensionPlan> {
        &self.suspension
    }
}

/// Immutable input to a backend-plan selection.
#[derive(Debug, Clone, Copy)]
pub struct SelectionInput<'a> {
    operation: SemanticOperationId,
    region: SelectionRegion,
    facts: SelectionFacts<'a>,
}

impl<'a> SelectionInput<'a> {
    /// Construct a conservative input when common facts are unavailable.
    ///
    /// Direct and intrinsic plans will decline this compatibility path. A
    /// prebuilt-argv generic invocation remains eligible when the target
    /// supplies its runtime service.
    #[must_use]
    pub const fn new(operation: SemanticOperationId, region: SelectionRegion) -> Self {
        Self {
            operation,
            region,
            facts: SelectionFacts::unavailable(),
        }
    }

    /// Construct an input with registry and common-pass proof obligations.
    #[must_use]
    pub const fn with_facts(
        operation: SemanticOperationId,
        region: SelectionRegion,
        facts: SelectionFacts<'a>,
    ) -> Self {
        Self {
            operation,
            region,
            facts,
        }
    }

    /// The registry-resolved semantic operation being selected.
    #[must_use]
    pub const fn operation(&self) -> SemanticOperationId {
        self.operation
    }

    /// The semantic region that constrains fallback legality.
    #[must_use]
    pub const fn region(&self) -> SelectionRegion {
        self.region
    }

    /// Common semantic facts and validated representation obligations.
    #[must_use]
    pub const fn facts(&self) -> &SelectionFacts<'a> {
        &self.facts
    }
}

/// One target-plan class in the fixed selection ladder.
///
/// Declaration order is selection order.  It is intentionally stable so that
/// a target's diagnostics and output remain deterministic when selectors have
/// equal explicit priorities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BackendPlanKind {
    /// A direct or native target-family operation.
    Direct,
    /// A call through a target runtime intrinsic.
    RuntimeIntrinsic,
    /// Tcl generic invocation reusing a prebuilt argv.
    GenericInvoke,
    /// Tcl `eval`, only for explicitly dynamic or opaque script regions.
    Eval,
    /// Transfer an explicit GPU/FPGA offload region to its host continuation.
    HostContinuation,
}

impl BackendPlanKind {
    fn required_runtime_service(self) -> Option<RuntimeService> {
        match self {
            Self::Direct => None,
            Self::RuntimeIntrinsic => Some(RuntimeService::Intrinsics),
            Self::GenericInvoke => Some(RuntimeService::GenericInvoke),
            Self::Eval => Some(RuntimeService::Eval),
            Self::HostContinuation => Some(RuntimeService::HostContinuation),
        }
    }
}

/// Deterministic preference within one [`BackendPlanKind`].
///
/// Smaller values are attempted first. The enclosing plan kind still wins:
/// every direct selector is considered before every intrinsic selector,
/// regardless of this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectorPriority(u16);

impl SelectorPriority {
    /// The default preference within one plan kind.
    pub const DEFAULT: Self = Self(0);

    /// Construct an explicit preference within one plan kind.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// The stable numeric representation for diagnostics and cache keys.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// An immutable selector request.
///
/// Selectors can inspect target capability and resource declarations, but they
/// cannot access an emitter or mutable compilation state.  The registry owns
/// construction of this request after it has checked the selector's declared
/// target-independent requirements.
#[derive(Debug, Clone, Copy)]
pub struct SelectorRequest<'a, Context = ()> {
    input: SelectionInput<'a>,
    target: &'a TargetContract<SemanticOperationId>,
    context: &'a Context,
}

impl<'a, Context> SelectorRequest<'a, Context> {
    /// The semantic operation being selected.
    #[must_use]
    pub const fn operation(&self) -> SemanticOperationId {
        self.input.operation()
    }

    /// The semantic region requesting the plan.
    #[must_use]
    pub const fn region(&self) -> SelectionRegion {
        self.input.region()
    }

    /// The immutable target contract for the compiler/backend pair.
    #[must_use]
    pub const fn target(&self) -> &'a TargetContract<SemanticOperationId> {
        self.target
    }

    /// Common semantic facts and proof obligations for this selection.
    #[must_use]
    pub const fn facts(&self) -> &SelectionFacts<'a> {
        self.input.facts()
    }

    /// Backend-specific immutable selection context supplied by the caller.
    ///
    /// The common registry never interprets this payload. It lets a backend
    /// plan from executable IR, target layout choices, or another immutable
    /// backend-owned representation only after common ladder legality checks.
    #[must_use]
    pub const fn context(&self) -> &Context {
        self.context
    }
}

/// A selector's typed refusal to create a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorDeclineReason {
    /// The selector does not implement this operation's semantic shape.
    NotApplicable,
    /// The selector requires facts that common analysis has not established.
    MissingSemanticProof,
    /// The selector's target-specific cost model rejected this otherwise legal plan.
    CostModelRejected,
}

/// A missing or negative common proof that forbids specialisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionProofFailure {
    /// No registry-owned invocation snapshot was supplied.
    InvocationFactsUnavailable,
    /// The supplied facts describe a different semantic operation.
    OperationMismatch,
    /// The registry did not declare a closed state-transition description.
    UnknownStateTransitions,
    /// Callback or trace re-entry requires a common world-state barrier.
    WorldBarrier,
    /// Registry argument roles still require argument-dependent resolution.
    IncompleteArgumentRoles,
    /// Materialisation-boundary evidence is missing or failed.
    Boundary(ProofStatus),
    /// Completion ownership evidence is missing or failed.
    Ownership(ProofStatus),
    /// Suspension preservation evidence is missing or failed.
    Suspension(ProofStatus),
    /// Command-dispatch stability evidence is missing or failed.
    DispatchStability(ProofStatus),
}

impl SelectionProofFailure {
    /// Stable Explorer/API spelling for one failed common proof obligation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvocationFactsUnavailable => "invocation-facts-unavailable",
            Self::OperationMismatch => "operation-mismatch",
            Self::UnknownStateTransitions => "unknown-state-transitions",
            Self::WorldBarrier => "world-barrier",
            Self::IncompleteArgumentRoles => "incomplete-argument-roles",
            Self::Boundary(_) => "boundary",
            Self::Ownership(_) => "ownership",
            Self::Suspension(_) => "suspension",
            Self::DispatchStability(_) => "dispatch-stability",
        }
    }
}

/// The result of one selector building an immutable plan payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorDecision<Plan, Decline = SelectorDeclineReason> {
    /// A complete immutable plan was selected; emission may begin afterwards.
    Selected(Plan),
    /// This selector created no plan and made no emitter-visible change.
    Declined(Decline),
}

/// Target-family plan construction for one semantic operation.
///
/// This deliberately has no emitter parameter and only an immutable receiver.
/// Implementations must also be shareable because one backend registry may be
/// queried by parallel compilation workers. Emitters consume
/// [`SelectedBackendPlan`] after selection has completed.
pub trait BackendSelector<Plan, Context = (), Decline = SelectorDeclineReason>:
    Send + Sync
{
    /// Build one complete immutable plan, or decline without emitting anything.
    fn select(&self, request: &SelectorRequest<'_, Context>) -> SelectorDecision<Plan, Decline>;
}

/// Why a registered selector cannot be used for a particular selection input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorAttemptFailure<Decline = SelectorDeclineReason> {
    /// Direct/native and intrinsic selection requires a resolved operation or
    /// an explicit offload-boundary operation, never a dynamic script.
    DirectOrIntrinsicRequiresResolvedOperation,
    /// Generic invocation requires an argv that common IR already constructed.
    GenericInvokeRequiresPrebuiltArgv,
    /// `eval` is legal only for explicitly dynamic scripts or opaque regions.
    EvalRequiresDynamicOrOpaqueRegion,
    /// Host continuation is legal only at an explicit offload boundary.
    HostContinuationRequiresOffloadBoundary,
    /// Host continuation is reserved for GPU and FPGA target families.
    HostContinuationRequiresDeviceTarget,
    /// Common analyses did not prove target-independent specialisation legal.
    SemanticProofs(Vec<SelectionProofFailure>),
    /// The target lacks one or more declared feature, service, or resource needs.
    TargetRequirements(Vec<RequirementFailure>),
    /// The selector itself declined without producing a plan.
    Selector(Decline),
}

/// One failed selector attempt, in deterministic selection order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorAttempt<Decline = SelectorDeclineReason> {
    /// Target-plan class attempted.
    pub kind: BackendPlanKind,
    /// Deterministic preference within that class.
    pub priority: SelectorPriority,
    /// Why the selector did not provide an immutable plan.
    pub failure: SelectorAttemptFailure<Decline>,
}

/// The typed reason a backend-plan selection could not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendDeclineReason<Decline = SelectorDeclineReason> {
    /// No selector was registered for the semantic operation or generic fallback.
    MissingOperation(SemanticOperationId),
    /// Every eligible selector declined or failed target/region checks.
    NoViablePlan {
        /// Registry-resolved operation that needed a plan.
        operation: SemanticOperationId,
        /// Attempts in fixed ladder then priority order.
        attempts: Vec<SelectorAttempt<Decline>>,
    },
}

/// The final outcome of selecting a backend plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendSelection<Plan, Decline = SelectorDeclineReason> {
    /// A complete plan was selected before any emission began.
    Selected(SelectedBackendPlan<Plan>),
    /// No plan was emitted; the typed reason lists every attempted fallback.
    Declined(BackendDeclineReason<Decline>),
}

/// A complete immutable target plan selected before emission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedBackendPlan<Plan> {
    operation: SemanticOperationId,
    kind: BackendPlanKind,
    plan: Plan,
}

impl<Plan> SelectedBackendPlan<Plan> {
    /// The semantic operation for which this plan was selected.
    #[must_use]
    pub const fn operation(&self) -> SemanticOperationId {
        self.operation
    }

    /// The selected target-plan class.
    #[must_use]
    pub const fn kind(&self) -> BackendPlanKind {
        self.kind
    }

    /// Borrow the immutable plan payload before handing it to an emitter.
    #[must_use]
    pub const fn plan(&self) -> &Plan {
        &self.plan
    }

    /// Consume the selection and hand its immutable payload to an emitter.
    #[must_use]
    pub fn into_plan(self) -> Plan {
        self.plan
    }
}

/// A duplicate selector registration for the same operation, plan class, and priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateSelector {
    /// The semantic operation whose selector slot is already occupied.
    pub operation: SemanticOperationId,
    /// The duplicated target-plan class.
    pub kind: BackendPlanKind,
    /// The duplicated deterministic preference.
    pub priority: SelectorPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SelectorOrder {
    kind: BackendPlanKind,
    priority: SelectorPriority,
}

struct SelectorEntry<Plan, Context, Decline> {
    requirements: LegalisationRequirements,
    selector: Box<dyn BackendSelector<Plan, Context, Decline> + Send + Sync>,
}

/// Per-backend registry of immutable selectors keyed by semantic operation.
///
/// Each compiler/backend pair owns one instance.  It deliberately reads the
/// shared [`TargetContract`] rather than mutating its operation registrations:
/// contracts model target capability, while this registry owns target-specific
/// plan payload construction and selection priority.
pub struct BackendRegistry<Plan, Context = (), Decline = SelectorDeclineReason> {
    target: TargetContract<SemanticOperationId>,
    selectors: BTreeMap<
        SemanticOperationId,
        BTreeMap<SelectorOrder, SelectorEntry<Plan, Context, Decline>>,
    >,
}

impl<Plan, Context, Decline> BackendRegistry<Plan, Context, Decline> {
    /// Create an empty selector registry for one compiler/backend target contract.
    #[must_use]
    pub fn new(target: TargetContract<SemanticOperationId>) -> Self {
        Self {
            target,
            selectors: BTreeMap::new(),
        }
    }

    /// The immutable target contract used for every selection.
    #[must_use]
    pub const fn target(&self) -> &TargetContract<SemanticOperationId> {
        &self.target
    }

    /// Register one plan builder for a semantic operation.
    ///
    /// Selectors are ordered first by [`BackendPlanKind`] and then by
    /// [`SelectorPriority`].  A duplicate operation/kind/priority slot is
    /// rejected so registration order can never affect a tie.
    pub fn register<S>(
        &mut self,
        operation: SemanticOperationId,
        kind: BackendPlanKind,
        priority: SelectorPriority,
        requirements: LegalisationRequirements,
        selector: S,
    ) -> Result<(), DuplicateSelector>
    where
        S: BackendSelector<Plan, Context, Decline> + 'static,
    {
        let order = SelectorOrder { kind, priority };
        let entries = self.selectors.entry(operation).or_default();
        match entries.entry(order) {
            Entry::Vacant(slot) => {
                slot.insert(SelectorEntry {
                    requirements,
                    selector: Box::new(selector),
                });
                Ok(())
            }
            Entry::Occupied(_) => Err(DuplicateSelector {
                operation,
                kind,
                priority,
            }),
        }
    }

    /// Select a complete immutable plan using backend-specific immutable
    /// context, or return every typed decline.
    #[must_use]
    pub fn select_with_context<'a>(
        &self,
        input: SelectionInput<'a>,
        context: &'a Context,
    ) -> BackendSelection<Plan, Decline> {
        let mut candidates = self.candidates(input.operation());
        if candidates.is_empty() {
            return BackendSelection::Declined(BackendDeclineReason::MissingOperation(
                input.operation(),
            ));
        }

        candidates.sort_by_key(|candidate| (candidate.order, candidate.origin));
        let mut attempts = Vec::new();
        for candidate in candidates {
            if let Some(failure) = self.region_failure(candidate.order.kind, input.region()) {
                attempts.push(candidate.attempt(failure));
                continue;
            }

            let semantic_failures = specialisation_proof_failures(candidate.order.kind, &input);
            if !semantic_failures.is_empty() {
                attempts.push(
                    candidate.attempt(SelectorAttemptFailure::SemanticProofs(semantic_failures)),
                );
                continue;
            }

            let unmet =
                self.unmet_requirements(candidate.order.kind, &candidate.entry.requirements);
            if !unmet.is_empty() {
                attempts.push(candidate.attempt(SelectorAttemptFailure::TargetRequirements(unmet)));
                continue;
            }

            let request = SelectorRequest {
                input,
                target: &self.target,
                context,
            };
            match candidate.entry.selector.select(&request) {
                SelectorDecision::Selected(plan) => {
                    return BackendSelection::Selected(SelectedBackendPlan {
                        operation: input.operation(),
                        kind: candidate.order.kind,
                        plan,
                    });
                }
                SelectorDecision::Declined(reason) => {
                    attempts.push(candidate.attempt(SelectorAttemptFailure::Selector(reason)));
                }
            }
        }

        BackendSelection::Declined(BackendDeclineReason::NoViablePlan {
            operation: input.operation(),
            attempts,
        })
    }

    fn candidates(
        &self,
        operation: SemanticOperationId,
    ) -> Vec<Candidate<'_, Plan, Context, Decline>> {
        let mut candidates = Vec::new();
        if let Some(entries) = self.selectors.get(&operation) {
            candidates.extend(entries.iter().map(|(order, entry)| Candidate {
                order: *order,
                origin: CandidateOrigin::ExactOperation,
                entry,
            }));
        }

        if operation != SemanticOperationId::Invoke
            && let Some(entries) = self.selectors.get(&SemanticOperationId::Invoke)
        {
            candidates.extend(
                entries
                    .iter()
                    .filter(|(order, _)| {
                        matches!(
                            order.kind,
                            BackendPlanKind::GenericInvoke
                                | BackendPlanKind::Eval
                                | BackendPlanKind::HostContinuation
                        )
                    })
                    .map(|(order, entry)| Candidate {
                        order: *order,
                        origin: CandidateOrigin::GenericInvokeFallback,
                        entry,
                    }),
            );
        }
        candidates
    }

    fn region_failure(
        &self,
        kind: BackendPlanKind,
        region: SelectionRegion,
    ) -> Option<SelectorAttemptFailure<Decline>> {
        match kind {
            BackendPlanKind::Direct | BackendPlanKind::RuntimeIntrinsic => (!matches!(
                region,
                SelectionRegion::PrebuiltArgvInvocation | SelectionRegion::OffloadBoundary
            ))
            .then_some(SelectorAttemptFailure::DirectOrIntrinsicRequiresResolvedOperation),
            BackendPlanKind::GenericInvoke => (region != SelectionRegion::PrebuiltArgvInvocation)
                .then_some(SelectorAttemptFailure::GenericInvokeRequiresPrebuiltArgv),
            BackendPlanKind::Eval => (!matches!(
                region,
                SelectionRegion::DynamicScript | SelectionRegion::OpaqueRegion
            ))
            .then_some(SelectorAttemptFailure::EvalRequiresDynamicOrOpaqueRegion),
            BackendPlanKind::HostContinuation => {
                if region != SelectionRegion::OffloadBoundary {
                    Some(SelectorAttemptFailure::HostContinuationRequiresOffloadBoundary)
                } else if !matches!(self.target.family(), TargetFamily::Gpu | TargetFamily::Fpga) {
                    Some(SelectorAttemptFailure::HostContinuationRequiresDeviceTarget)
                } else {
                    None
                }
            }
        }
    }

    fn unmet_requirements(
        &self,
        kind: BackendPlanKind,
        requirements: &LegalisationRequirements,
    ) -> Vec<RequirementFailure> {
        let mut unmet = requirements.unmet_by(self.target.capabilities());
        if let Some(service) = kind.required_runtime_service()
            && !self
                .target
                .capabilities()
                .runtime()
                .services()
                .contains(&service)
            && !unmet.contains(&RequirementFailure::Runtime(service))
        {
            unmet.push(RequirementFailure::Runtime(service));
        }
        unmet
    }
}

/// Apply the target-independent proof obligations used by
/// [`BackendRegistry`] before a direct or runtime-intrinsic selector runs.
///
/// Common semantic planners use this same entry point when constructing
/// proof-bearing candidates, rather than maintaining a second approximation
/// of the backend registry's legality rules.
#[must_use]
pub fn specialisation_proof_failures(
    kind: BackendPlanKind,
    input: &SelectionInput<'_>,
) -> Vec<SelectionProofFailure> {
    if !matches!(
        kind,
        BackendPlanKind::Direct | BackendPlanKind::RuntimeIntrinsic
    ) || input.region() == SelectionRegion::OffloadBoundary
    {
        return Vec::new();
    }

    let facts = input.facts();
    let Some(invocation) = facts.invocation() else {
        return vec![SelectionProofFailure::InvocationFactsUnavailable];
    };
    let mut failures = Vec::new();
    if invocation.operation != input.operation() {
        failures.push(SelectionProofFailure::OperationMismatch);
    }
    if !invocation.state_transitions.is_declared() {
        failures.push(SelectionProofFailure::UnknownStateTransitions);
    }
    if invocation.effects.requires_world_barrier() {
        failures.push(SelectionProofFailure::WorldBarrier);
    }
    if !invocation.arg_roles_complete {
        failures.push(SelectionProofFailure::IncompleteArgumentRoles);
    }
    require_plan_proof(
        facts.boundary().status(),
        SelectionProofFailure::Boundary,
        &mut failures,
    );
    require_plan_proof(
        facts.ownership().status(),
        SelectionProofFailure::Ownership,
        &mut failures,
    );
    require_plan_proof(
        facts.suspension().status(),
        SelectionProofFailure::Suspension,
        &mut failures,
    );
    require_plan_proof(
        facts.dispatch().status(),
        SelectionProofFailure::DispatchStability,
        &mut failures,
    );
    failures
}

fn require_plan_proof(
    status: ProofStatus,
    failure: fn(ProofStatus) -> SelectionProofFailure,
    failures: &mut Vec<SelectionProofFailure>,
) {
    if !status.permits_specialisation() {
        failures.push(failure(status));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CandidateOrigin {
    ExactOperation,
    GenericInvokeFallback,
}

struct Candidate<'a, Plan, Context, Decline> {
    order: SelectorOrder,
    origin: CandidateOrigin,
    entry: &'a SelectorEntry<Plan, Context, Decline>,
}

impl<Plan, Context, Decline> Candidate<'_, Plan, Context, Decline> {
    fn attempt(&self, failure: SelectorAttemptFailure<Decline>) -> SelectorAttempt<Decline> {
        SelectorAttempt {
            kind: self.order.kind,
            priority: self.order.priority,
            failure,
        }
    }
}

impl<Plan> BackendRegistry<Plan> {
    /// Select a complete immutable plan for a context-free backend.
    ///
    /// Backends whose planning needs executable IR or target-layout inputs use
    /// [`Self::select_with_context`] instead.  This compatibility entry point
    /// keeps existing context-free registries honest without inventing a
    /// mutable emitter side channel.
    #[must_use]
    pub fn select(&self, input: SelectionInput<'_>) -> BackendSelection<Plan> {
        self.select_with_context(input, &())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target_contract::{
        LegalisationRequirements, TargetCapabilities, TargetContract, TargetFamily, ValueFeature,
    };
    use tcl_registry::{
        CommandRegistry, StateTransitionKnowledge, StateTransitions, dialects::DialectSet,
        hooks::LoweringHookId,
    };

    #[derive(Debug, Clone)]
    struct StaticSelector {
        decision: SelectorDecision<&'static str>,
    }

    impl BackendSelector<&'static str> for StaticSelector {
        fn select(&self, _: &SelectorRequest<'_>) -> SelectorDecision<&'static str> {
            self.decision.clone()
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ContextDecline {
        Rejected(u8),
    }

    #[derive(Debug)]
    struct ContextSelector;

    impl BackendSelector<&'static str, u8, ContextDecline> for ContextSelector {
        fn select(
            &self,
            request: &SelectorRequest<'_, u8>,
        ) -> SelectorDecision<&'static str, ContextDecline> {
            if *request.context() == 7 {
                SelectorDecision::Selected("context-plan")
            } else {
                SelectorDecision::Declined(ContextDecline::Rejected(*request.context()))
            }
        }
    }

    fn operation() -> SemanticOperationId {
        SemanticOperationId::StructuredLowering(LoweringHookId::Incr)
    }

    fn wasm_registry() -> BackendRegistry<&'static str> {
        BackendRegistry::new(TargetContract::new(
            TargetFamily::Wasm,
            TargetCapabilities::wasm(),
        ))
    }

    fn gpu_registry() -> BackendRegistry<&'static str> {
        BackendRegistry::new(TargetContract::new(
            TargetFamily::Gpu,
            TargetCapabilities::gpu(),
        ))
    }

    fn fpga_registry() -> BackendRegistry<&'static str> {
        BackendRegistry::new(TargetContract::new(
            TargetFamily::Fpga,
            TargetCapabilities::fpga(),
        ))
    }

    #[test]
    fn backend_context_reaches_selector_and_preserves_its_typed_decline() {
        let mut registry: BackendRegistry<&'static str, u8, ContextDecline> = BackendRegistry::new(
            TargetContract::new(TargetFamily::Wasm, TargetCapabilities::wasm()),
        );
        registry
            .register(
                SemanticOperationId::Invoke,
                BackendPlanKind::GenericInvoke,
                SelectorPriority::DEFAULT,
                LegalisationRequirements::new(),
                ContextSelector,
            )
            .expect("unique generic selector");

        let selected = registry.select_with_context(
            SelectionInput::new(
                SemanticOperationId::Invoke,
                SelectionRegion::PrebuiltArgvInvocation,
            ),
            &7,
        );
        assert!(matches!(
            selected,
            BackendSelection::Selected(SelectedBackendPlan {
                plan: "context-plan",
                ..
            })
        ));

        let declined = registry.select_with_context(
            SelectionInput::new(
                SemanticOperationId::Invoke,
                SelectionRegion::PrebuiltArgvInvocation,
            ),
            &3,
        );
        assert!(matches!(
            declined,
            BackendSelection::Declined(BackendDeclineReason::NoViablePlan {
                attempts,
                ..
            }) if matches!(
                attempts.as_slice(),
                [SelectorAttempt {
                    failure: SelectorAttemptFailure::Selector(ContextDecline::Rejected(3)),
                    ..
                }]
            )
        ));
    }

    fn closed_invocation_facts() -> InvocationFacts {
        let registry = CommandRegistry::build_default();
        let mut facts = registry
            .resolve_invocation("incr", &["counter"], DialectSet::TCL86)
            .expect("core invocation resolves")
            .facts();
        facts.effects = EffectFootprint::default();
        facts.state_transitions = StateTransitionKnowledge::Declared(StateTransitions::default());
        facts.arg_roles_complete = true;
        facts
    }

    fn dispatch_proof(facts: &InvocationFacts) -> ClosedWorldDispatchProof {
        ClosedWorldDispatchProof {
            dependencies: facts.dispatch_dependencies,
            policy: ClosedWorldDispatchPolicy::WholeProgram,
        }
    }

    fn proven_input<'a>(
        facts: &'a InvocationFacts,
        dispatch: &'a ClosedWorldDispatchProof,
    ) -> SelectionInput<'a> {
        SelectionInput::with_facts(
            facts.operation,
            SelectionRegion::PrebuiltArgvInvocation,
            SelectionFacts::from_invocation(facts)
                .with_no_representation_obligations(
                    &crate::semantic_analysis::test_common_analysis_provenance(),
                )
                .with_closed_world_dispatch(dispatch),
        )
    }

    #[test]
    fn runtime_guard_domains_cover_registry_dispatch_dependencies() {
        let dependencies = DispatchDependencies::from_domains(&[
            DispatchDependencyDomain::CommandBinding,
            DispatchDependencyDomain::NamespaceLookup,
            DispatchDependencyDomain::CommandTraces,
            DispatchDependencyDomain::InterpreterPolicy,
            DispatchDependencyDomain::ObjectDispatch,
            DispatchDependencyDomain::UnknownHandling,
        ]);
        let complete = GuardDomains::one(GuardDomain::CommandEnvironment)
            .with(GuardDomain::Namespace)
            .with(GuardDomain::CommandTrace)
            .with(GuardDomain::Interpreter)
            .with(GuardDomain::ObjectDispatch)
            .with(GuardDomain::UnknownHandling);

        assert!(guards_cover_dispatch(dependencies, complete));
        assert!(!guards_cover_dispatch(
            dependencies,
            GuardDomains::one(GuardDomain::CommandEnvironment)
        ));
    }

    #[test]
    fn falls_back_in_fixed_ladder_order_to_generic_prebuilt_argv_invoke() {
        let mut registry = wasm_registry();
        let mut unsupported_direct = LegalisationRequirements::new();
        unsupported_direct.values.insert(ValueFeature::VectorValues);
        registry
            .register(
                operation(),
                BackendPlanKind::Direct,
                SelectorPriority::DEFAULT,
                unsupported_direct,
                StaticSelector {
                    decision: SelectorDecision::Selected("direct"),
                },
            )
            .expect("unique direct selector");
        registry
            .register(
                operation(),
                BackendPlanKind::RuntimeIntrinsic,
                SelectorPriority::DEFAULT,
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Declined(SelectorDeclineReason::NotApplicable),
                },
            )
            .expect("unique intrinsic selector");
        registry
            .register(
                SemanticOperationId::Invoke,
                BackendPlanKind::GenericInvoke,
                SelectorPriority::DEFAULT,
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Selected("generic-invoke"),
                },
            )
            .expect("unique generic selector");

        let selected = registry.select(SelectionInput::new(
            operation(),
            SelectionRegion::PrebuiltArgvInvocation,
        ));
        assert!(matches!(
            selected,
            BackendSelection::Selected(SelectedBackendPlan {
                kind: BackendPlanKind::GenericInvoke,
                plan: "generic-invoke",
                ..
            })
        ));
    }

    #[test]
    fn ordinary_unsupported_resolved_invocation_never_falls_back_to_eval() {
        let mut registry = wasm_registry();
        registry
            .register(
                SemanticOperationId::Invoke,
                BackendPlanKind::Direct,
                SelectorPriority::DEFAULT,
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Selected("must-not-select-direct"),
                },
            )
            .expect("unique direct selector");
        registry
            .register(
                SemanticOperationId::Invoke,
                BackendPlanKind::Eval,
                SelectorPriority::DEFAULT,
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Selected("eval"),
                },
            )
            .expect("unique eval selector");

        let declined = registry.select(SelectionInput::new(
            operation(),
            SelectionRegion::PrebuiltArgvInvocation,
        ));
        assert_eq!(
            declined,
            BackendSelection::Declined(BackendDeclineReason::NoViablePlan {
                operation: operation(),
                attempts: vec![SelectorAttempt {
                    kind: BackendPlanKind::Eval,
                    priority: SelectorPriority::DEFAULT,
                    failure: SelectorAttemptFailure::EvalRequiresDynamicOrOpaqueRegion,
                }],
            })
        );

        let dynamic = registry.select(SelectionInput::new(
            SemanticOperationId::Invoke,
            SelectionRegion::DynamicScript,
        ));
        assert!(matches!(
            dynamic,
            BackendSelection::Selected(SelectedBackendPlan {
                kind: BackendPlanKind::Eval,
                plan: "eval",
                ..
            })
        ));
    }

    #[test]
    fn backend_registry_with_thread_safe_selectors_is_shareable() {
        fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<BackendRegistry<&'static str>>();
    }

    #[test]
    fn declining_selector_cannot_leave_a_partial_emission_plan() {
        let mut registry = wasm_registry();
        registry
            .register(
                operation(),
                BackendPlanKind::Direct,
                SelectorPriority::DEFAULT,
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Declined(
                        SelectorDeclineReason::MissingSemanticProof,
                    ),
                },
            )
            .expect("unique declining selector");
        registry
            .register(
                operation(),
                BackendPlanKind::RuntimeIntrinsic,
                SelectorPriority::DEFAULT,
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Selected("complete-intrinsic-plan"),
                },
            )
            .expect("unique intrinsic selector");

        let facts = closed_invocation_facts();
        let dispatch = dispatch_proof(&facts);
        let selected = registry.select(proven_input(&facts, &dispatch));
        let mut emitted = Vec::new();
        if let BackendSelection::Selected(plan) = selected {
            emitted.push(plan.into_plan());
        }
        assert_eq!(emitted, vec!["complete-intrinsic-plan"]);
    }

    #[test]
    fn selector_order_uses_plan_kind_then_priority_and_rejects_duplicates() {
        let mut registry = wasm_registry();
        registry
            .register(
                operation(),
                BackendPlanKind::RuntimeIntrinsic,
                SelectorPriority::new(9),
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Selected("later-priority"),
                },
            )
            .expect("first selector registers");
        registry
            .register(
                operation(),
                BackendPlanKind::RuntimeIntrinsic,
                SelectorPriority::new(1),
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Selected("earlier-priority"),
                },
            )
            .expect("second priority is distinct");

        let duplicate = registry.register(
            operation(),
            BackendPlanKind::RuntimeIntrinsic,
            SelectorPriority::new(1),
            LegalisationRequirements::new(),
            StaticSelector {
                decision: SelectorDecision::Selected("duplicate"),
            },
        );
        assert_eq!(
            duplicate,
            Err(DuplicateSelector {
                operation: operation(),
                kind: BackendPlanKind::RuntimeIntrinsic,
                priority: SelectorPriority::new(1),
            })
        );

        let facts = closed_invocation_facts();
        let dispatch = dispatch_proof(&facts);
        let selected = registry.select(proven_input(&facts, &dispatch));
        assert!(matches!(
            selected,
            BackendSelection::Selected(SelectedBackendPlan {
                plan: "earlier-priority",
                ..
            })
        ));
    }

    #[test]
    fn host_continuation_is_only_a_gpu_or_fpga_offload_boundary_fallback() {
        for mut registry in [gpu_registry(), fpga_registry()] {
            registry
                .register(
                    SemanticOperationId::Invoke,
                    BackendPlanKind::HostContinuation,
                    SelectorPriority::DEFAULT,
                    LegalisationRequirements::new(),
                    StaticSelector {
                        decision: SelectorDecision::Selected("host-continuation"),
                    },
                )
                .expect("unique host selector");

            let in_region = registry.select(SelectionInput::new(
                operation(),
                SelectionRegion::PrebuiltArgvInvocation,
            ));
            assert!(matches!(
                in_region,
                BackendSelection::Declined(BackendDeclineReason::NoViablePlan {
                    attempts,
                    ..
                }) if attempts == vec![SelectorAttempt {
                    kind: BackendPlanKind::HostContinuation,
                    priority: SelectorPriority::DEFAULT,
                    failure: SelectorAttemptFailure::HostContinuationRequiresOffloadBoundary,
                }]
            ));

            let boundary = registry.select(SelectionInput::new(
                operation(),
                SelectionRegion::OffloadBoundary,
            ));
            assert!(matches!(
                boundary,
                BackendSelection::Selected(SelectedBackendPlan {
                    kind: BackendPlanKind::HostContinuation,
                    plan: "host-continuation",
                    ..
                })
            ));
        }
    }

    #[test]
    fn target_resource_requirements_decline_before_a_selector_can_build_a_plan() {
        use crate::target_contract::{ResourceDemand, ResourceFailure, ResourceKind};

        let mut registry = BackendRegistry::new(TargetContract::new(
            TargetFamily::Ebpf,
            TargetCapabilities::ebpf(),
        ));
        let mut requirements = LegalisationRequirements::new();
        requirements
            .resources
            .push(ResourceDemand::new(ResourceKind::StackBytes, 513));
        registry
            .register(
                operation(),
                BackendPlanKind::Direct,
                SelectorPriority::DEFAULT,
                requirements,
                StaticSelector {
                    decision: SelectorDecision::Selected("oversized-plan"),
                },
            )
            .expect("unique direct selector");

        let facts = closed_invocation_facts();
        let dispatch = dispatch_proof(&facts);
        assert_eq!(
            registry.select(proven_input(&facts, &dispatch)),
            BackendSelection::Declined(BackendDeclineReason::NoViablePlan {
                operation: operation(),
                attempts: vec![SelectorAttempt {
                    kind: BackendPlanKind::Direct,
                    priority: SelectorPriority::DEFAULT,
                    failure: SelectorAttemptFailure::TargetRequirements(vec![
                        RequirementFailure::Resource(ResourceFailure::ExceedsLimit {
                            kind: ResourceKind::StackBytes,
                            maximum: 512,
                            required: 513,
                        }),
                    ]),
                }],
            })
        );
    }

    #[test]
    fn unknown_transition_knowledge_declines_direct_but_keeps_generic_fallback() {
        let mut registry = wasm_registry();
        registry
            .register(
                operation(),
                BackendPlanKind::Direct,
                SelectorPriority::DEFAULT,
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Selected("must-not-select-direct"),
                },
            )
            .expect("unique direct selector");
        registry
            .register(
                SemanticOperationId::Invoke,
                BackendPlanKind::GenericInvoke,
                SelectorPriority::DEFAULT,
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Selected("generic-invoke"),
                },
            )
            .expect("unique generic selector");

        let mut facts = closed_invocation_facts();
        facts.state_transitions = StateTransitionKnowledge::UnknownInvocation;
        let dispatch = dispatch_proof(&facts);
        let selected = registry.select(proven_input(&facts, &dispatch));
        assert!(matches!(
            selected,
            BackendSelection::Selected(SelectedBackendPlan {
                kind: BackendPlanKind::GenericInvoke,
                plan: "generic-invoke",
                ..
            })
        ));
    }

    #[test]
    fn closed_semantics_still_require_dispatch_stability_for_direct_selection() {
        let mut registry = wasm_registry();
        registry
            .register(
                operation(),
                BackendPlanKind::Direct,
                SelectorPriority::DEFAULT,
                LegalisationRequirements::new(),
                StaticSelector {
                    decision: SelectorDecision::Selected("direct"),
                },
            )
            .expect("unique direct selector");

        let facts = closed_invocation_facts();
        let semantic_only = SelectionInput::with_facts(
            facts.operation,
            SelectionRegion::PrebuiltArgvInvocation,
            SelectionFacts::from_invocation(&facts).with_no_representation_obligations(
                &crate::semantic_analysis::test_common_analysis_provenance(),
            ),
        );
        assert!(matches!(
            registry.select(semantic_only),
            BackendSelection::Declined(BackendDeclineReason::NoViablePlan {
                attempts,
                ..
            }) if attempts == vec![SelectorAttempt {
                kind: BackendPlanKind::Direct,
                priority: SelectorPriority::DEFAULT,
                failure: SelectorAttemptFailure::SemanticProofs(vec![
                    SelectionProofFailure::DispatchStability(ProofStatus::Unavailable),
                ]),
            }]
        ));

        let dispatch = dispatch_proof(&facts);
        assert!(matches!(
            registry.select(proven_input(&facts, &dispatch)),
            BackendSelection::Selected(SelectedBackendPlan {
                kind: BackendPlanKind::Direct,
                plan: "direct",
                ..
            })
        ));
    }
}
