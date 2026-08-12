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

use tcl_dialect::DialectProfile;
use tcl_registry::dialects::DialectSet;
use tcl_registry::{CommandRegistry, EffectFootprint};

use crate::completion::CompletionObligations;
use crate::executable_ir::{
    ExecutableFunction, ExecutableFunctionId, GenericInvoke, InvocationResolution,
    LoweredOperation, OpaqueRegion, SourceCompatibilityDecline, build_linear_executable_ir,
};
use crate::ir::Script;
use crate::mixed_region_plan::{MixedPlanBuildError, MixedRegionPlan};
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
/// The dialect is an explicit registry dialect bit rather than an inferred
/// target choice.  A function with no retained source IR records a typed
/// unavailable state; a source script the linear compatibility builder cannot
/// represent records its exact decline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticAnalysisBundle {
    dialect: DialectSet,
    executable: ExecutableAnalysisAvailability,
    mixed_plan: MixedRegionPlanAvailability,
}

impl SemanticAnalysisBundle {
    /// Build facts from source-faithful IR under the explicitly selected
    /// registry dialect.
    #[must_use]
    pub fn build(registry: &CommandRegistry, dialect: DialectSet, script: &Script) -> Self {
        if dialect.canonical_name().is_none() {
            return Self::from_executable(
                dialect,
                ExecutableAnalysisAvailability::DialectUnavailable { dialect },
            );
        }
        let executable = match build_linear_executable_ir(
            registry,
            dialect,
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
            Err(decline) => ExecutableAnalysisAvailability::SourceDeclined(decline),
        };
        Self::from_executable(dialect, executable)
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
    pub(crate) fn build_for_interactive_analysis(
        registry: &CommandRegistry,
        dialect: DialectSet,
        script: &Script,
    ) -> Self {
        if dialect.canonical_name().is_none() {
            return Self::from_executable(
                dialect,
                ExecutableAnalysisAvailability::DialectUnavailable { dialect },
            );
        }
        let executable = match build_linear_executable_ir(
            registry,
            dialect,
            ExecutableFunctionId::new(0),
            script,
        ) {
            Ok(function) if interactive_gvn_needs_world_state(&function) => {
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
            Err(decline) => ExecutableAnalysisAvailability::SourceDeclined(decline),
        };
        Self::from_executable(dialect, executable)
    }

    /// Build an explicit unavailable bundle for a function build that did not
    /// retain a source script.
    #[must_use]
    pub fn unavailable(dialect: DialectSet) -> Self {
        Self::from_executable(
            dialect,
            if dialect.canonical_name().is_some() {
                ExecutableAnalysisAvailability::SourceUnavailable
            } else {
                ExecutableAnalysisAvailability::DialectUnavailable { dialect }
            },
        )
    }

    /// The registry dialect used for every invocation resolution in this
    /// bundle.
    #[must_use]
    pub const fn dialect(&self) -> DialectSet {
        self.dialect
    }

    /// The executable-IR availability, including every typed decline.
    #[must_use]
    pub const fn executable(&self) -> &ExecutableAnalysisAvailability {
        &self.executable
    }

    /// Retained mixed per-region plan, or why no faithful plan was available.
    #[must_use]
    pub const fn mixed_plan(&self) -> &MixedRegionPlanAvailability {
        &self.mixed_plan
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
        if !config.is_enabled(SemanticOptimisationPassId::GuardedIntrinsic) {
            return self.mixed_plan.clone();
        }
        let (MixedRegionPlanAvailability::Available(plan), Some(function)) =
            (&self.mixed_plan, self.executable.function())
        else {
            return self.mixed_plan.clone();
        };
        let provenance = CommonAnalysisProvenance { _private: () };
        let runtime_version = self
            .dialect
            .canonical_name()
            .and_then(DialectProfile::find)
            .and_then(|profile| profile.runtime_base);
        match plan.select_guarded_boxed_intrinsics(function, runtime_version, &provenance) {
            Ok(plan) => MixedRegionPlanAvailability::Available(plan),
            Err(error) => {
                MixedRegionPlanAvailability::Declined(MixedPlanBuildError::InvalidPlan(error))
            }
        }
    }

    fn from_executable(dialect: DialectSet, executable: ExecutableAnalysisAvailability) -> Self {
        let mixed_plan = match executable.function() {
            Some(function) => match MixedRegionPlan::build(function) {
                Ok(plan) => MixedRegionPlanAvailability::Available(plan),
                Err(decline) => MixedRegionPlanAvailability::Declined(decline),
            },
            None => MixedRegionPlanAvailability::ExecutableUnavailable,
        };
        Self {
            dialect,
            executable,
            mixed_plan,
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
    /// The attachment had no one explicit dialect profile. Empty and
    /// combinator masks are deliberately not treated as a union-resolution
    /// policy for a per-document sidecar.
    DialectUnavailable {
        /// The caller-supplied dialect mask that could not select one profile.
        dialect: DialectSet,
    },
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
            Self::DialectUnavailable { .. } | Self::SourceDeclined(_) | Self::SourceUnavailable => {
                None
            }
        }
    }

    /// Return world-state SSA when its common renamer completed.
    #[must_use]
    pub const fn world_state_ssa(&self) -> Option<&ExecutableWorldStateSsa> {
        match self {
            Self::Available(facts) => Some(&facts.world_state_ssa),
            Self::WorldStateDeclined { .. }
            | Self::WorldStateNotRequired { .. }
            | Self::DialectUnavailable { .. }
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
                        | crate::executable_ir::ExecutableInstruction::ExecuteOpaqueRegion(_) => {
                            None
                        }
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
    fn guarded_intrinsic_selection_requires_explicit_enablement() {
        let registry = CommandRegistry::build_default();
        let module = lower_to_ir("string length value", &registry);
        let bundle = SemanticAnalysisBundle::build(&registry, DialectSet::TCL90, &module.top_level);
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

        let tcl8_bundle =
            SemanticAnalysisBundle::build(&registry, DialectSet::TCL86, &module.top_level);
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
