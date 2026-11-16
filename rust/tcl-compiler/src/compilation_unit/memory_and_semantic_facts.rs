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

//! Memory SSA and semantic facts attached on demand, for one function and for the whole unit.

use super::{CompilationUnit, FunctionUnit, RetainedSemanticDepth};
use crate::semantic_analysis::SemanticAnalysisBundle;
use tcl_registry::CommandRegistry;
use tcl_registry::model::semantic::SemanticContext;

impl FunctionUnit {
    /// Populate memory-SSA on demand in `context`. Returns `self` for
    /// chaining.
    #[must_use]
    pub fn with_memory_ssa(
        self,
        registry: &tcl_registry::CommandRegistry,
        context: Option<SemanticContext>,
    ) -> Self {
        self.with_memory_ssa_with_metadata_context(registry, context.map(Into::into))
    }

    /// Populate memory facts using the exact supplied metadata generation.
    /// Missing or foreign actual input cannot borrow a catalogue profile.
    #[must_use]
    pub fn with_memory_ssa_with_metadata_context(
        mut self,
        registry: &tcl_registry::CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> Self {
        self.memory_ssa = Some(self.memory_ssa_for_metadata(registry, context));
        self
    }

    fn memory_ssa_for_metadata(
        &self,
        registry: &CommandRegistry,
        context: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    ) -> crate::memory_ssa::MemorySsaFunction {
        let fallback;
        let points = if let Some(points) = &self.ssa.point_contexts {
            points
        } else {
            fallback = crate::variable_bindings::build_point_resolve_contexts(
                &self.cfg, &self.name, registry,
            );
            &fallback
        };
        crate::memory_ssa::build_memory_ssa_with_cfg_with_metadata_context(
            &self.cfg, &self.ssa, points, registry, context,
        )
    }

    /// Attach semantic facts for a compilation unit's top-level script.
    ///
    /// This explicit standalone context models the top-level script in a fresh interpreter
    /// whose dispatch state is the registry baseline; every other unit kind
    /// runs after arbitrary interposed history and must use
    /// [`Self::with_semantic_analysis`] with an unknown-world entry contract.
    #[must_use]
    pub fn with_top_level_semantic_analysis(
        self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        script: &crate::ir::Script,
    ) -> Self {
        self.with_semantic_analysis(
            registry,
            context,
            Some(script),
            crate::dispatch_proof::DispatchEntryAssumption::PristineRegistryWorld,
        )
    }

    /// Attach standalone source facts under an explicitly supplied default
    /// context and dispatch contract. Current unit consumers use
    /// [`Self::with_retained_semantic_analysis`] for their actual input.
    ///
    /// A missing source script remains an explicit unavailable state. A
    /// structured source shape outside the executable compatibility subset is
    /// retained as a typed decline, never converted into guessed facts.
    #[must_use]
    pub fn with_semantic_analysis(
        mut self,
        registry: &CommandRegistry,
        context: Option<SemanticContext>,
        script: Option<&crate::ir::Script>,
        entry_assumption: crate::dispatch_proof::DispatchEntryAssumption,
    ) -> Self {
        self.semantic_facts = script.map_or_else(
            || SemanticAnalysisBundle::unavailable(context),
            |script| {
                SemanticAnalysisBundle::build_for_interactive_analysis(
                    registry,
                    context,
                    script,
                    entry_assumption,
                )
            },
        );
        self
    }

    /// Attach current source facts using this unit's retained input and grammar.
    /// Missing, foreign or differently parsed input stays unavailable. This
    /// metadata selection supplies no Native command entry or dispatch proof.
    #[must_use]
    pub fn with_retained_semantic_analysis(
        mut self,
        registry: &CommandRegistry,
        script: Option<&crate::ir::Script>,
        entry_assumption: crate::dispatch_proof::DispatchEntryAssumption,
    ) -> Self {
        self.semantic_facts = self.retained_semantic_facts(
            registry,
            script,
            entry_assumption,
            RetainedSemanticDepth::Interactive,
        );
        self
    }

    fn retained_semantic_facts(
        &self,
        registry: &CommandRegistry,
        script: Option<&crate::ir::Script>,
        entry_assumption: crate::dispatch_proof::DispatchEntryAssumption,
        depth: RetainedSemanticDepth,
    ) -> SemanticAnalysisBundle {
        Self::semantic_facts_with_metadata(
            registry,
            self.invocation_metadata_context(registry),
            script,
            entry_assumption,
            depth,
        )
    }

    fn semantic_facts_with_metadata(
        registry: &CommandRegistry,
        metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
        script: Option<&crate::ir::Script>,
        entry_assumption: crate::dispatch_proof::DispatchEntryAssumption,
        depth: RetainedSemanticDepth,
    ) -> SemanticAnalysisBundle {
        script.map_or_else(
            || SemanticAnalysisBundle::unavailable_with_metadata_context(registry, metadata),
            |script| match depth {
                RetainedSemanticDepth::Interactive => {
                    SemanticAnalysisBundle::build_for_interactive_analysis_with_metadata_context(
                        registry,
                        metadata,
                        script,
                        entry_assumption,
                    )
                }
                RetainedSemanticDepth::Full => SemanticAnalysisBundle::build_with_metadata_context(
                    registry,
                    metadata,
                    script,
                    entry_assumption,
                ),
            },
        )
    }
}

impl CompilationUnit {
    /// Populate memory facts from retained module and function inputs.
    /// The compatibility context argument is ignored; it cannot replace an
    /// absent or stale actual owner. Actual consumers use
    /// [`Self::with_retained_memory_ssa`].
    #[must_use]
    pub fn with_memory_ssa(
        self,
        registry: &tcl_registry::CommandRegistry,
        _context: Option<SemanticContext>,
    ) -> Self {
        self.with_retained_memory_ssa(registry)
    }

    /// Populate top-level and procedure memory facts from their matching
    /// actual source owners. Missing, foreign or stale metadata retains
    /// opaque variable-memory obligations instead of borrowing a profile.
    #[must_use]
    pub fn with_retained_memory_ssa(mut self, registry: &CommandRegistry) -> Self {
        let original_owner = self
            .optimiser_source_owner
            .clone()
            .filter(|owner| owner.matches_unit(&self) && owner.matches_registry(registry));
        self.optimiser_source_owner = None;

        let context = self
            .top_level
            .invocation_metadata_context_for_module(registry, &self.ir_module);
        self.top_level.memory_ssa = Some(self.top_level.memory_ssa_for_metadata(registry, context));
        for unit in self.procedures.values_mut() {
            let context = unit.invocation_metadata_context_for_module(registry, &self.ir_module);
            unit.memory_ssa = Some(unit.memory_ssa_for_metadata(registry, context));
        }
        if let Some(owner) = original_owner {
            self.retain_enriched_original_optimiser_owner(&owner);
        }
        self
    }

    /// Build full source sidecars from every function's actual retained input.
    /// Missing or foreign availability and stale grammar remain unavailable;
    /// no default metadata context is reconstructed for deep inspection.
    #[must_use]
    pub fn with_retained_deep_semantic_analysis(mut self, registry: &CommandRegistry) -> Self {
        use crate::dispatch_proof::DispatchEntryAssumption;
        let original_owner = self
            .optimiser_source_owner
            .clone()
            .filter(|owner| owner.matches_unit(&self) && owner.matches_registry(registry));
        self.optimiser_source_owner = None;

        self.top_level.semantic_facts = FunctionUnit::semantic_facts_with_metadata(
            registry,
            self.top_level
                .invocation_metadata_context_for_module(registry, &self.ir_module),
            Some(&self.ir_module.top_level),
            DispatchEntryAssumption::PristineRegistryWorld,
            RetainedSemanticDepth::Full,
        );
        for (name, unit) in &mut self.procedures {
            unit.semantic_facts = FunctionUnit::semantic_facts_with_metadata(
                registry,
                unit.invocation_metadata_context_for_module(registry, &self.ir_module),
                self.ir_module
                    .procedures
                    .get(name)
                    .map(|procedure| &procedure.body),
                DispatchEntryAssumption::UnknownWorld,
                RetainedSemanticDepth::Full,
            );
        }
        for (name, unit) in &mut self.methods {
            unit.semantic_facts = FunctionUnit::semantic_facts_with_metadata(
                registry,
                unit.invocation_metadata_context_for_module(registry, &self.ir_module),
                self.ir_module.methods.get(name).map(|method| &method.body),
                DispatchEntryAssumption::UnknownWorld,
                RetainedSemanticDepth::Full,
            );
        }
        for (name, unit) in self
            .body_units
            .iter_mut()
            .chain(&mut self.carried_installed_lowerings)
        {
            unit.semantic_facts = FunctionUnit::semantic_facts_with_metadata(
                registry,
                unit.invocation_metadata_context_for_module(registry, &self.ir_module),
                self.ir_module.body_lowering(name).map(|body| &body.body),
                DispatchEntryAssumption::UnknownWorld,
                RetainedSemanticDepth::Full,
            );
        }
        if let Some(owner) = original_owner {
            self.retain_enriched_original_optimiser_owner(&owner);
        }
        self
    }

    /// Rebuild full facts under an explicitly supplied standalone context.
    /// Actual source consumers use [`Self::with_retained_deep_semantic_analysis`].
    ///
    /// Ordinary interactive analysis intentionally avoids materialising a
    /// world graph when no reusable-value proof can consume it.  A caller
    /// requesting this method instead receives either the full graph or the
    /// existing typed availability/decline for every retained top-level and
    /// procedure script.
    #[must_use]
    pub fn with_deep_semantic_analysis(
        mut self,
        registry: &tcl_registry::CommandRegistry,
        context: Option<SemanticContext>,
    ) -> Self {
        use crate::dispatch_proof::DispatchEntryAssumption;
        self.optimiser_source_owner = None;

        self.top_level.semantic_facts = SemanticAnalysisBundle::build(
            registry,
            context,
            &self.ir_module.top_level,
            DispatchEntryAssumption::PristineRegistryWorld,
        );
        for (name, unit) in &mut self.procedures {
            let Some(procedure) = self.ir_module.procedures.get(name) else {
                unit.semantic_facts = SemanticAnalysisBundle::unavailable(context);
                continue;
            };
            unit.semantic_facts = SemanticAnalysisBundle::build(
                registry,
                context,
                &procedure.body,
                DispatchEntryAssumption::UnknownWorld,
            );
        }
        for (name, unit) in &mut self.methods {
            let Some(method) = self.ir_module.methods.get(name) else {
                unit.semantic_facts = SemanticAnalysisBundle::unavailable(context);
                continue;
            };
            unit.semantic_facts = SemanticAnalysisBundle::build(
                registry,
                context,
                &method.body,
                DispatchEntryAssumption::UnknownWorld,
            );
        }
        for (name, unit) in self
            .body_units
            .iter_mut()
            .chain(&mut self.carried_installed_lowerings)
        {
            let Some(body) = self.ir_module.body_lowering(name) else {
                unit.semantic_facts = SemanticAnalysisBundle::unavailable(context);
                continue;
            };
            unit.semantic_facts = SemanticAnalysisBundle::build(
                registry,
                context,
                &body.body,
                DispatchEntryAssumption::UnknownWorld,
            );
        }
        self
    }
}
