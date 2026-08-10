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

use tcl_registry::dialects::DialectSet;
use tcl_registry::{CommandRegistry, EffectFootprint};

use crate::completion::CompletionObligations;
use crate::executable_ir::{
    ExecutableFunction, ExecutableFunctionId, GenericInvoke, InvocationResolution,
    LoweredOperation, OpaqueRegion, SourceCompatibilityDecline, build_linear_executable_ir,
};
use crate::ir::Script;
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
}

impl SemanticAnalysisBundle {
    /// Build facts from source-faithful IR under the explicitly selected
    /// registry dialect.
    #[must_use]
    pub fn build(registry: &CommandRegistry, dialect: DialectSet, script: &Script) -> Self {
        if dialect.canonical_name().is_none() {
            return Self {
                dialect,
                executable: ExecutableAnalysisAvailability::DialectUnavailable { dialect },
            };
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
        Self {
            dialect,
            executable,
        }
    }

    /// Build an explicit unavailable bundle for a function build that did not
    /// retain a source script.
    #[must_use]
    pub fn unavailable(dialect: DialectSet) -> Self {
        Self {
            dialect,
            executable: if dialect.canonical_name().is_some() {
                ExecutableAnalysisAvailability::SourceUnavailable
            } else {
                ExecutableAnalysisAvailability::DialectUnavailable { dialect }
            },
        }
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
            Self::WorldStateDeclined { function, .. } => Some(function),
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
