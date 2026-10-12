// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target-neutral semantic analysis facts owned by one function unit.
//!
//! This is an ownership and availability boundary, not an optimiser or a
//! backend plan.  The existing scalar SSA remains owned by
//! [`crate::compilation_unit::FunctionUnit`], and the existing optional
//! memory SSA remains opt-in.  This bundle adds the facts that can be built
//! faithfully from the current narrow executable-IR compatibility layer:
//! structured registry invocation outcomes, completion and effect inputs, and
//! executable world-state SSA.  Source shapes outside that compatibility layer
//! retain a typed decline instead of receiving guessed facts.

use tcl_registry::model::semantic::SemanticContext;
use tcl_registry::{CommandRegistry, EffectFootprint};

use crate::completion::CompletionObligations;
use crate::dispatch_proof::DispatchEntryAssumption;
use crate::executable_ir::{
    EvaluatedRegionCompletion, ExecutableFunction, ExecutableFunctionId,
    ExecutableMetadataBuildDecline, GenericInvoke, InvocationResolution, LoweredOperation,
    OpaqueRegion, SourceCompatibilityDecline, StructuredRegion,
    build_linear_executable_ir_with_metadata_context,
};
use crate::ir::Script;
use crate::mixed_region_plan::{MixedPlanBuildError, MixedRegionPlan};
use crate::registry_invocation::InvocationMetadataContext;
use crate::semantic_optimisation::{SemanticOptimisationConfig, SemanticOptimisationPassId};
use crate::world_state_ssa::{
    ExecutableWorldStateSsa, WorldStateSsaDecline, build_executable_world_state_ssa,
};

/// Unforgeable authority carried by evidence produced inside common analysis.
///
/// Backend-facing proof constructors accept this token but cannot construct it:
/// the private field is owned by this module.  Future common passes should
/// create proof-bearing plans through focused methods here after establishing
/// their semantic obligations; merely being a backend is never authority to
/// assert that an obligation is absent or satisfied.
#[derive(Debug)]
pub struct CommonAnalysisProvenance {
    _private: (),
}

#[cfg(test)]
pub(crate) const fn test_common_analysis_provenance() -> CommonAnalysisProvenance {
    CommonAnalysisProvenance { _private: () }
}

/// Target-neutral semantic facts attached to one
/// [`crate::compilation_unit::FunctionUnit`].
///
/// Source analyses retain their actual metadata owner independently of the
/// dispatch entry contract. Explicit standalone callers may supply their own
/// [`SemanticContext`]. A function with no retained source IR records a typed
/// unavailable state; unsupported source records its exact decline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticAnalysisBundle {
    context: Option<SemanticContext>,
    metadata_context: Option<RetainedSemanticMetadataContext>,
    executable: ExecutableAnalysisAvailability,
    entry_assumption: DispatchEntryAssumption,
}

/// Retain a source driver's shared context without rebuilding its profile.
/// Explicitly borrowed metadata callers retain a complete availability view.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RetainedSemanticMetadataContext {
    Source(crate::analyser::ResolvedAnalysisInput),
    Borrowed(std::sync::Arc<tcl_registry::model::ResolvedContext>),
}
impl RetainedSemanticMetadataContext {
    fn from_supplied(context: InvocationMetadataContext<'_>) -> Self {
        context.source_analysis_input().map_or_else(
            || Self::Borrowed(std::sync::Arc::new(context.context().clone())),
            |input| Self::Source(input.clone()),
        )
    }
    fn context(&self) -> &tcl_registry::model::ResolvedContext {
        match self {
            Self::Source(input) => input.borrowed_context_registry().context(),
            Self::Borrowed(context) => context,
        }
    }
}

impl SemanticAnalysisBundle {
    /// Build facts from source-faithful IR under the explicitly resolved
    /// semantic context and dispatch entry contract.
    #[must_use]
    pub fn build(
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        script: &Script,
        entry_assumption: DispatchEntryAssumption,
    ) -> Self {
        let mut bundle = Self::build_with_metadata_context(
            registry,
            context.map(Into::into),
            script,
            entry_assumption,
        );
        bundle.context = context;
        bundle.metadata_context = None;
        bundle
    }

    /// Build full semantic facts with the exact supplied metadata owner.
    /// Missing or foreign availability remains unavailable; source metadata
    /// supplies no entered Native command, frame or compiler admission.
    #[must_use]
    pub fn build_with_metadata_context(
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        script: &Script,
        entry_assumption: DispatchEntryAssumption,
    ) -> Self {
        let context = context.filter(|context| context.matches_registry(registry));
        if context.is_none() {
            return Self::from_executable(
                None,
                ExecutableAnalysisAvailability::ContextUnavailable,
                entry_assumption,
            );
        }
        let executable = match build_linear_executable_ir_with_metadata_context(
            registry,
            context,
            ExecutableFunctionId::new(0),
            script,
        ) {
            Ok(function) => match build_executable_world_state_ssa(&function) {
                Ok(world_state_ssa) => {
                    ExecutableAnalysisAvailability::Available(ExecutableSemanticFacts {
                        function,
                        world_state_ssa,
                    })
                }
                Err(decline) => {
                    ExecutableAnalysisAvailability::WorldStateDeclined { function, decline }
                }
            },
            Err(ExecutableMetadataBuildDecline::Source(decline)) => {
                ExecutableAnalysisAvailability::SourceDeclined(decline)
            }
            Err(ExecutableMetadataBuildDecline::ContextUnavailable) => {
                ExecutableAnalysisAvailability::ContextUnavailable
            }
        };
        let mut bundle = Self::from_executable(None, executable, entry_assumption);
        bundle.metadata_context = context.map(RetainedSemanticMetadataContext::from_supplied);
        bundle
    }

    /// Build the executable invocation facts used by interactive GVN, and
    /// materialise world-state SSA only when an invocation can actually enter
    /// GVN's reusable-value domain.
    ///
    /// The full [`Self::build`] path remains available to backends and deep
    /// analyses that consume world-state versions directly. Ordinary LSP
    /// indexing must not pay that graph cost merely to discover that every
    /// invocation fails GVN's closed-world eligibility predicate.
    #[must_use]
    pub fn build_for_interactive_analysis(
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        script: &Script,
        entry_assumption: DispatchEntryAssumption,
    ) -> Self {
        let mut bundle = Self::build_for_interactive_analysis_with_metadata_context(
            registry,
            context.map(Into::into),
            script,
            entry_assumption,
        );
        bundle.context = context;
        bundle.metadata_context = None;
        bundle
    }

    /// Interactive facts under the supplied source availability, with no
    /// profile-context fallback. Retained source grammar stays independent.
    #[must_use]
    pub fn build_for_interactive_analysis_with_metadata_context(
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
        script: &Script,
        entry_assumption: DispatchEntryAssumption,
    ) -> Self {
        let context = context.filter(|context| context.matches_registry(registry));
        if context.is_none() {
            return Self::from_executable(
                None,
                ExecutableAnalysisAvailability::ContextUnavailable,
                entry_assumption,
            );
        }
        // A unit whose dispatch entry contract is `UnknownWorld` starts at the
        // contents lattice's top element, and widening is absorbing, so no
        // site proof it could produce would ever succeed. Building its world
        // graph would materialise a structure whose only interactive consumer
        // is that proof. The deep [`Self::build`] path is unaffected: code
        // generation and auditing consume the graph directly.
        let proof_can_succeed = entry_assumption != DispatchEntryAssumption::UnknownWorld;
        let executable = match build_linear_executable_ir_with_metadata_context(
            registry,
            context,
            ExecutableFunctionId::new(0),
            script,
        ) {
            Ok(function) if proof_can_succeed && interactive_gvn_needs_world_state(&function) => {
                match build_executable_world_state_ssa(&function) {
                    Ok(world_state_ssa) => {
                        ExecutableAnalysisAvailability::Available(ExecutableSemanticFacts {
                            function,
                            world_state_ssa,
                        })
                    }
                    Err(decline) => {
                        ExecutableAnalysisAvailability::WorldStateDeclined { function, decline }
                    }
                }
            }
            Ok(function) => ExecutableAnalysisAvailability::WorldStateNotRequired { function },
            Err(ExecutableMetadataBuildDecline::Source(decline)) => {
                ExecutableAnalysisAvailability::SourceDeclined(decline)
            }
            Err(ExecutableMetadataBuildDecline::ContextUnavailable) => {
                ExecutableAnalysisAvailability::ContextUnavailable
            }
        };
        let mut bundle = Self::from_executable(None, executable, entry_assumption);
        bundle.metadata_context = context.map(RetainedSemanticMetadataContext::from_supplied);
        bundle
    }

    /// Build an explicit unavailable bundle for a function build that did not
    /// retain a source script.
    #[must_use]
    pub fn unavailable(context: Option<SemanticContext>) -> Self {
        Self::from_executable(
            context,
            if context.is_some() {
                ExecutableAnalysisAvailability::SourceUnavailable
            } else {
                ExecutableAnalysisAvailability::ContextUnavailable
            },
            DispatchEntryAssumption::UnknownWorld,
        )
    }

    /// Record missing source independently of supplied metadata availability.
    /// A valid retained owner remains visible without creating invocation,
    /// frame or execution facts; missing or foreign input stays unavailable.
    #[must_use]
    pub fn unavailable_with_metadata_context(
        registry: &CommandRegistry,
        context: Option<InvocationMetadataContext<'_>>,
    ) -> Self {
        let context = context.filter(|context| context.matches_registry(registry));
        let executable = if context.is_some() {
            ExecutableAnalysisAvailability::SourceUnavailable
        } else {
            ExecutableAnalysisAvailability::ContextUnavailable
        };
        let mut bundle =
            Self::from_executable(None, executable, DispatchEntryAssumption::UnknownWorld);
        bundle.metadata_context = context.map(RetainedSemanticMetadataContext::from_supplied);
        bundle
    }

    /// The explicit standalone context, when this bundle used that ingress.
    /// Supplied source analyses expose their actual [`Self::metadata_context`]
    /// instead of constructing a default environment handle.
    #[must_use]
    pub const fn context(&self) -> Option<SemanticContext> {
        self.context
    }

    /// Exact retained availability used to select this bundle's invocation
    /// metadata. This is independent of source grammar and Native execution.
    #[must_use]
    pub fn metadata_context(&self) -> Option<&tcl_registry::model::ResolvedContext> {
        self.metadata_context
            .as_ref()
            .map(RetainedSemanticMetadataContext::context)
            .or_else(|| self.context.map(SemanticContext::context))
    }

    /// The executable-IR availability, including every typed decline.
    #[must_use]
    pub const fn executable(&self) -> &ExecutableAnalysisAvailability {
        &self.executable
    }

    /// The mixed per-region plan for this bundle, or why no faithful plan was
    /// available.
    ///
    /// Built on demand rather than retained.  Interactive analysis constructs a
    /// bundle for every procedure on every keystroke and never reads this plan,
    /// whereas its consumers — the WASM pipeline and Explorer — ask for it once
    /// per compile.  Retaining it would put a whole-function plan build and its
    /// validation on the per-keystroke path for no consumer.
    #[must_use]
    pub fn mixed_plan(&self) -> MixedRegionPlanAvailability {
        match self.executable.function() {
            Some(function) => match MixedRegionPlan::build(function) {
                Ok(plan) => MixedRegionPlanAvailability::Available(plan),
                Err(decline) => MixedRegionPlanAvailability::Declined(decline),
            },
            None => MixedRegionPlanAvailability::ExecutableUnavailable,
        }
    }

    /// Return mixed-region evidence after explicitly enabled common semantic
    /// optimisation passes have run.
    ///
    /// The retained bundle remains the conservative, pass-disabled baseline.
    /// Provenance for guard and representation proofs is created only inside
    /// this semantic-analysis boundary.
    #[must_use]
    pub fn mixed_plan_with_optimisations(
        &self,
        config: SemanticOptimisationConfig,
    ) -> MixedRegionPlanAvailability {
        let baseline = self.mixed_plan();
        if !config.is_enabled(SemanticOptimisationPassId::GuardedIntrinsic) {
            return baseline;
        }
        let (function, plan) = match (self.executable.function(), baseline) {
            (Some(function), MixedRegionPlanAvailability::Available(plan)) => (function, plan),
            (_, baseline) => return baseline,
        };
        let provenance = CommonAnalysisProvenance { _private: () };
        let runtime_version = self
            .metadata_context()
            .or_else(|| self.context.map(SemanticContext::context))
            .and_then(|context| context.environment.point())
            .and_then(tcl_dialect::model::DialectPoint::tcl_version);
        match plan.select_guarded_boxed_intrinsics(function, runtime_version, &provenance) {
            Ok(plan) => MixedRegionPlanAvailability::Available(plan),
            Err(error) => {
                MixedRegionPlanAvailability::Declined(MixedPlanBuildError::InvalidPlan(error))
            }
        }
    }

    /// The dispatch entry contract this unit's world proofs are made under.
    #[must_use]
    pub const fn dispatch_entry_assumption(&self) -> DispatchEntryAssumption {
        self.entry_assumption
    }

    const fn from_executable(
        context: Option<SemanticContext>,
        executable: ExecutableAnalysisAvailability,
        entry_assumption: DispatchEntryAssumption,
    ) -> Self {
        Self {
            context,
            metadata_context: None,
            executable,
            entry_assumption,
        }
    }
}

/// Availability of the target-neutral mixed per-region execution plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixedRegionPlanAvailability {
    /// Every executable region has retained selection and decline evidence.
    Available(MixedRegionPlan),
    /// No executable function was available; its precise reason remains in
    /// [`ExecutableAnalysisAvailability`].
    ExecutableUnavailable,
    /// Executable IR existed, but mixed planning declined without guessing.
    Declined(MixedPlanBuildError),
}

impl MixedRegionPlanAvailability {
    /// Return the retained plan when construction succeeded.
    #[must_use]
    pub const fn plan(&self) -> Option<&MixedRegionPlan> {
        match self {
            Self::Available(plan) => Some(plan),
            Self::ExecutableUnavailable | Self::Declined(_) => None,
        }
    }
}

/// Availability of the target-neutral executable semantic facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutableAnalysisAvailability {
    /// The attachment carried no resolved environment. A per-document sidecar
    /// deliberately never invents one; the state is an absent
    /// [`SemanticContext`].
    ContextUnavailable,
    /// Executable IR and world-state SSA were both constructed and validated.
    Available(ExecutableSemanticFacts),
    /// Executable IR was constructed, but the world-state renamer declined it.
    ///
    /// Invocation, completion, and effect inputs remain available through the
    /// retained executable function; only the derived world SSA is absent.
    WorldStateDeclined {
        /// The validated executable semantic function.
        function: ExecutableFunction,
        /// Why the common world-state SSA builder declined it.
        decline: WorldStateSsaDecline,
    },
    /// Executable invocation facts were retained, but no invocation could
    /// enter interactive GVN's reusable-value domain, so no world-state graph
    /// was requested.
    WorldStateNotRequired {
        /// The validated executable semantic function.
        function: ExecutableFunction,
    },
    /// The source IR was outside the deliberately narrow executable subset.
    SourceDeclined(SourceCompatibilityDecline),
    /// No source script was supplied by this function-build entry point.
    SourceUnavailable,
}

impl ExecutableAnalysisAvailability {
    /// Return the executable IR when it was built, even if only the
    /// world-state derivation declined.
    #[must_use]
    pub const fn function(&self) -> Option<&ExecutableFunction> {
        match self {
            Self::Available(facts) => Some(&facts.function),
            Self::WorldStateDeclined { function, .. }
            | Self::WorldStateNotRequired { function } => Some(function),
            Self::ContextUnavailable | Self::SourceDeclined(_) | Self::SourceUnavailable => None,
        }
    }

    /// Return world-state SSA when its common renamer completed.
    #[must_use]
    pub const fn world_state_ssa(&self) -> Option<&ExecutableWorldStateSsa> {
        match self {
            Self::Available(facts) => Some(&facts.world_state_ssa),
            Self::WorldStateDeclined { .. }
            | Self::WorldStateNotRequired { .. }
            | Self::ContextUnavailable
            | Self::SourceDeclined(_)
            | Self::SourceUnavailable => None,
        }
    }

    /// Iterate generic invocations with their structured registry outcomes.
    ///
    /// A resolved outcome owns [`tcl_registry::InvocationFacts`]; an
    /// unresolved outcome preserves why no descriptor could safely be chosen.
    pub fn invocations(&self) -> impl Iterator<Item = &GenericInvoke> {
        self.function().into_iter().flat_map(|function| {
            function.blocks.iter().flat_map(|block| {
                block
                    .instructions
                    .iter()
                    .filter_map(|instruction| match instruction {
                        crate::executable_ir::ExecutableInstruction::Invoke(invoke) => Some(invoke),
                        crate::executable_ir::ExecutableInstruction::EvaluateWord { .. }
                        | crate::executable_ir::ExecutableInstruction::ExpandWord { .. }
                        | crate::executable_ir::ExecutableInstruction::BuildArgv { .. }
                        | crate::executable_ir::ExecutableInstruction::ExecuteLowered(_)
                        | crate::executable_ir::ExecutableInstruction::ExecuteOpaqueRegion(_)
                        | crate::executable_ir::ExecutableInstruction::EvaluateExpr { .. }
                        | crate::executable_ir::ExecutableInstruction::MatchPattern { .. }
                        | crate::executable_ir::ExecutableInstruction::IterateLists { .. }
                        | crate::executable_ir::ExecutableInstruction::JoinCompletion { .. }
                        | crate::executable_ir::ExecutableInstruction::WriteCompletionCell {
                            ..
                        }
                        | crate::executable_ir::ExecutableInstruction::CompleteStructuredRegion(
                            _,
                        )
                        | crate::executable_ir::ExecutableInstruction::CompleteEvaluatedRegion(_) => None,
                    })
            })
        })
    }

    /// Iterate already-lowered operations that retain a registry-owned
    /// structural descriptor but deliberately carry no forged command identity.
    pub fn lowered_operations(&self) -> impl Iterator<Item = &LoweredOperation> {
        self.function().into_iter().flat_map(|function| {
            function.blocks.iter().flat_map(|block| {
                block.instructions.iter().filter_map(|instruction| {
                    if let crate::executable_ir::ExecutableInstruction::ExecuteLowered(operation) =
                        instruction
                    {
                        Some(operation)
                    } else {
                        None
                    }
                })
            })
        })
    }

    /// Iterate structured compatibility regions that remain executable world
    /// barriers instead of declining the whole containing function.
    pub fn opaque_regions(&self) -> impl Iterator<Item = &OpaqueRegion> {
        self.function().into_iter().flat_map(|function| {
            function.blocks.iter().flat_map(|block| {
                block.instructions.iter().filter_map(|instruction| {
                    if let crate::executable_ir::ExecutableInstruction::ExecuteOpaqueRegion(
                        region,
                    ) = instruction
                    {
                        Some(region)
                    } else {
                        None
                    }
                })
            })
        })
    }

    /// Iterate structured control regions that now carry real executable
    /// edges instead of a conservative opaque barrier.
    pub fn structured_regions(&self) -> impl Iterator<Item = &StructuredRegion> {
        self.function().into_iter().flat_map(|function| {
            function.blocks.iter().flat_map(|block| {
                block.instructions.iter().filter_map(|instruction| {
                    if let crate::executable_ir::ExecutableInstruction::CompleteStructuredRegion(
                        region,
                    ) = instruction
                    {
                        Some(region)
                    } else {
                        None
                    }
                })
            })
        })
    }

    /// Iterate wrapper completions whose scripts already have executable edges.
    ///
    /// These are metadata for the residual wrapper, not a second invocation of
    /// its original source. Consumers must not replay the wrapper after its phases.
    ///
    /// ```
    /// use tcl_compiler::semantic_analysis::ExecutableAnalysisAvailability;
    /// let unavailable = ExecutableAnalysisAvailability::SourceUnavailable;
    /// assert_eq!(unavailable.evaluated_regions().count(), 0);
    /// ```
    pub fn evaluated_regions(&self) -> impl Iterator<Item = &EvaluatedRegionCompletion> {
        self.function().into_iter().flat_map(|function| {
            function.blocks.iter().flat_map(|block| {
                block.instructions.iter().filter_map(|instruction| {
                    if let crate::executable_ir::ExecutableInstruction::CompleteEvaluatedRegion(
                        region,
                    ) = instruction
                    {
                        Some(region)
                    } else {
                        None
                    }
                })
            })
        })
    }

    /// Iterate completion inputs for generic invocation sites.
    ///
    /// Unresolved heads are deliberately conservative, not assumed to have a
    /// successful or effect-free completion contract.
    pub fn completion_inputs(&self) -> impl Iterator<Item = CompletionObligations> + '_ {
        self.invocations()
            .map(|invoke| match &invoke.resolution {
                InvocationResolution::Resolved(facts) => {
                    CompletionObligations::from_descriptor(facts.completion)
                }
                InvocationResolution::Unresolved(_) => CompletionObligations::conservative(),
            })
            .chain(
                self.lowered_operations()
                    .map(|_| CompletionObligations::conservative()),
            )
            .chain(
                self.opaque_regions()
                    .map(|_| CompletionObligations::conservative()),
            )
            .chain(
                self.evaluated_regions()
                    .map(|_| CompletionObligations::conservative()),
            )
    }

    /// Iterate effect inputs without fabricating a closed footprint for an
    /// unresolved invocation.
    pub fn effect_inputs(&self) -> impl Iterator<Item = InvocationEffectInput<'_>> {
        self.invocations()
            .map(|invoke| match &invoke.resolution {
                InvocationResolution::Resolved(facts) => {
                    InvocationEffectInput::Resolved(facts.world_state_effects())
                }
                InvocationResolution::Unresolved(_) => InvocationEffectInput::ConservativeUnknown,
            })
            .chain(
                self.lowered_operations()
                    .map(|_| InvocationEffectInput::ConservativeUnknown),
            )
            .chain(
                self.opaque_regions()
                    .map(|_| InvocationEffectInput::ConservativeUnknown),
            )
            .chain(
                self.evaluated_regions()
                    .map(|_| InvocationEffectInput::ConservativeUnknown),
            )
    }
}

fn interactive_gvn_needs_world_state(function: &ExecutableFunction) -> bool {
    function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| {
            let crate::executable_ir::ExecutableInstruction::Invoke(invoke) = instruction else {
                return false;
            };
            matches!(
                &invoke.resolution,
                InvocationResolution::Resolved(facts)
                    if crate::gvn::resolved_invocation_is_gvn_candidate(facts)
                        || crate::gvn::resolved_invocation_is_versioned_world_gvn_candidate(facts)
            )
        })
    })
}

/// Executable semantic facts whose common derivations all succeeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableSemanticFacts {
    /// The source-faithful executable semantic IR.
    pub function: ExecutableFunction,
    /// CFG-aware SSA over registry-owned interpreter-world state.
    pub world_state_ssa: ExecutableWorldStateSsa,
}

/// One invocation's effect input for later common analyses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationEffectInput<'a> {
    /// The fully resolved registry-owned effect footprint.
    Resolved(&'a EffectFootprint),
    /// A computed or registry-unknown command head retains the generic Tcl
    /// all-world obligation; no precise footprint was manufactured.
    ConservativeUnknown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::NodeId;
    use crate::lowering::lower_to_ir;
    use crate::mixed_region_plan::{InvocationSelection, RegionPlan};

    #[test]
    fn supplied_semantic_bundle_retains_owner_and_keeps_missing_context_unavailable() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        use std::sync::Arc;
        let generation = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let actual = Arc::new(generation.with_command_store(Arc::clone(generation.commands())));
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            Arc::clone(&actual),
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        );
        let module = lower_to_ir("puts VALUE", actual.commands());
        let context = InvocationMetadataContext::for_analysis_input(actual.commands(), &input);
        let bundle = SemanticAnalysisBundle::build_for_interactive_analysis_with_metadata_context(
            actual.commands(),
            context,
            &module.top_level,
            DispatchEntryAssumption::UnknownWorld,
        );
        assert!(bundle.context().is_none());
        assert!(std::ptr::eq(
            bundle.metadata_context().unwrap(),
            actual.context()
        ));
        assert!(matches!(
            bundle.executable(),
            ExecutableAnalysisAvailability::WorldStateNotRequired { .. }
        ));
        assert_eq!(
            bundle.dispatch_entry_assumption(),
            DispatchEntryAssumption::UnknownWorld
        );
        drop(input);
        assert!(std::ptr::eq(
            bundle.metadata_context().unwrap(),
            actual.context()
        ));
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
        for context in [None, Some(InvocationMetadataContext::from(foreign))] {
            let bundle =
                SemanticAnalysisBundle::build_for_interactive_analysis_with_metadata_context(
                    actual.commands(),
                    context,
                    &module.top_level,
                    DispatchEntryAssumption::PristineRegistryWorld,
                );
            assert_eq!(
                bundle.executable(),
                &ExecutableAnalysisAvailability::ContextUnavailable
            );
            assert!(bundle.metadata_context().is_none());
            assert!(bundle.executable().invocations().next().is_none());
        }
    }

    #[test]
    fn absent_source_keeps_valid_supplied_metadata_without_invocation_facts() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        let actual =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&actual),
            tcl_lexer::LexerConfig::for_dialect("tcl8.6"),
        );
        let context = InvocationMetadataContext::for_analysis_input(actual.commands(), &input);
        let bundle =
            SemanticAnalysisBundle::unavailable_with_metadata_context(actual.commands(), context);
        assert!(matches!(
            bundle.executable(),
            ExecutableAnalysisAvailability::SourceUnavailable
        ));
        assert!(std::ptr::eq(
            bundle.metadata_context().unwrap(),
            actual.context()
        ));
        assert!(bundle.context().is_none());
        assert_eq!(
            bundle.dispatch_entry_assumption(),
            DispatchEntryAssumption::UnknownWorld
        );
        assert!(bundle.executable().invocations().next().is_none());
        assert!(bundle.executable().world_state_ssa().is_none());
        let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
        for context in [None, Some(foreign.into())] {
            let bundle = SemanticAnalysisBundle::unavailable_with_metadata_context(
                actual.commands(),
                context,
            );
            assert!(matches!(
                bundle.executable(),
                ExecutableAnalysisAvailability::ContextUnavailable
            ));
            assert!(bundle.metadata_context().is_none());
        }
    }

    #[test]
    fn evaluated_wrapper_metadata_is_not_an_ordinary_invocation() {
        let availability = ExecutableAnalysisAvailability::WorldStateNotRequired {
            function: crate::execution_region::evaluated_region_test_fixture(),
        };
        assert_eq!(availability.evaluated_regions().count(), 1);
        assert!(availability.invocations().all(|invoke| invoke.original_words.first().is_none_or(|word| !matches!(word, crate::ir::WordExpr::Literal { text, .. } if text == "tcltest::test"))));
        assert!(
            availability
                .effect_inputs()
                .any(|effect| effect == InvocationEffectInput::ConservativeUnknown)
        );
    }

    #[test]
    fn guarded_intrinsics_keep_explicit_standalone_and_supplied_protocols() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Metadata-selection control; no Native command or execution receipt is issued.
        use std::sync::Arc;
        let registry = Arc::new(CommandRegistry::build_default());
        let module = lower_to_ir("string length value", &registry);
        let node = NodeId::from_path(vec![0]);
        let config = SemanticOptimisationConfig::new()
            .with_enabled(SemanticOptimisationPassId::GuardedIntrinsic);
        for (name, version) in [
            ("tcl8.6", tcl_dialect::TclVersion::V8_6),
            ("tcl9.0", tcl_dialect::TclVersion::V9_0),
        ] {
            let explicit = SemanticContext::for_environment(name);
            let standalone = SemanticAnalysisBundle::build(
                &registry,
                Some(explicit),
                &module.top_level,
                DispatchEntryAssumption::PristineRegistryWorld,
            );
            assert_eq!(standalone.context(), Some(explicit));
            assert!(std::ptr::eq(
                standalone.metadata_context().unwrap(),
                explicit.context(),
            ));
            let actual = tcl_registry::model::ingress::static_context_for(name)
                .with_command_store(Arc::clone(&registry));
            let supplied = SemanticAnalysisBundle::build_with_metadata_context(
                &registry,
                Some(InvocationMetadataContext::from(&actual)),
                &module.top_level,
                DispatchEntryAssumption::PristineRegistryWorld,
            );
            assert!(supplied.context().is_none());
            assert_eq!(supplied.metadata_context(), Some(actual.context()));
            for bundle in [standalone, supplied] {
                let plan = bundle.mixed_plan_with_optimisations(config);
                let RegionPlan::Invocation(region) = plan.plan().unwrap().region(&node).unwrap()
                else {
                    panic!("original invocation");
                };
                let InvocationSelection::GuardedIntrinsic(evidence) = region.selection() else {
                    panic!("selected guarded intrinsic");
                };
                assert_eq!(evidence.runtime_version(), version);
            }
        }
        let missing = SemanticAnalysisBundle::build_with_metadata_context(
            &registry,
            None,
            &module.top_level,
            DispatchEntryAssumption::PristineRegistryWorld,
        );
        assert!(missing.context().is_none());
        assert!(missing.metadata_context().is_none());
        assert!(
            missing
                .mixed_plan_with_optimisations(config)
                .plan()
                .is_none()
        );
    }

    #[test]
    fn guarded_intrinsic_selection_requires_explicit_enablement() {
        let registry = CommandRegistry::build_default();
        let module = lower_to_ir("string length value", &registry);
        let bundle = SemanticAnalysisBundle::build(
            &registry,
            Some(SemanticContext::for_environment("tcl9.0")),
            &module.top_level,
            DispatchEntryAssumption::PristineRegistryWorld,
        );
        let node = NodeId::from_path(vec![0]);

        let off = bundle.mixed_plan_with_optimisations(SemanticOptimisationConfig::default());
        let RegionPlan::Invocation(off) = off.plan().unwrap().region(&node).unwrap() else {
            panic!("expected invocation");
        };
        assert_eq!(off.selection(), &InvocationSelection::GenericPrebuiltArgv);

        let on = bundle.mixed_plan_with_optimisations(
            SemanticOptimisationConfig::new()
                .with_enabled(SemanticOptimisationPassId::GuardedIntrinsic),
        );
        let RegionPlan::Invocation(on) = on.plan().unwrap().region(&node).unwrap() else {
            panic!("expected invocation");
        };
        assert!(matches!(
            on.selection(),
            InvocationSelection::GuardedIntrinsic(_)
        ));
        let InvocationSelection::GuardedIntrinsic(tcl9) = on.selection() else {
            unreachable!();
        };
        assert_eq!(tcl9.runtime_version(), tcl_dialect::TclVersion::V9_0);

        let tcl8_bundle = SemanticAnalysisBundle::build(
            &registry,
            Some(SemanticContext::for_environment("tcl8.6")),
            &module.top_level,
            DispatchEntryAssumption::PristineRegistryWorld,
        );
        let tcl8 = tcl8_bundle.mixed_plan_with_optimisations(
            SemanticOptimisationConfig::new()
                .with_enabled(SemanticOptimisationPassId::GuardedIntrinsic),
        );
        let RegionPlan::Invocation(tcl8) = tcl8.plan().unwrap().region(&node).unwrap() else {
            panic!("expected invocation");
        };
        let InvocationSelection::GuardedIntrinsic(tcl8) = tcl8.selection() else {
            panic!("expected guarded intrinsic");
        };
        assert_eq!(tcl8.runtime_version(), tcl_dialect::TclVersion::V8_6);
        assert_ne!(
            tcl8.guarded_plan().guard().expected_identity(),
            tcl9.guarded_plan().guard().expected_identity(),
        );
    }
}
