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

//! The module facts prepared before any CFG is built: `upvar` detection, entered procedures and the shared context bundle.

use super::summary_candidates::OriginalProcedureSummaryOwners;
use super::upvar_info::UpvarInfo;
use super::{CfgCommandClasses, CfgContext, PreparedCfgContext, frame_effects};
use crate::command_binding::ModuleCommandBindings;
use crate::ir::Module;
use crate::registry_invocation::OwnedInvocationMetadataContext as CfgMetadataContext;
use std::collections::HashMap;
use tcl_registry::CommandRegistry;

// Public API

/// Caller-frame facts retained separately for every genuine source body.
#[must_use]
pub fn detect_upvar_procs(module: &Module) -> HashMap<crate::ir::ProcedureId, UpvarInfo> {
    detect_upvar_procs_with_registry(module, module.resolved_registry())
}

/// Caller-frame facts under the Module's exact retained registry.
#[must_use]
pub fn detect_upvar_procs_with_registry(
    module: &Module,
    registry: &CommandRegistry,
) -> HashMap<crate::ir::ProcedureId, UpvarInfo> {
    detect_upvar_bodies_with_registry(module, registry)
}

pub(crate) fn detect_upvar_bodies_with_registry(
    module: &Module,
    registry: &CommandRegistry,
) -> HashMap<crate::ir::ProcedureId, UpvarInfo> {
    frame_effects::detect_caller_frame_bodies(module, registry)
}

impl PreparedCfgContext {
    /// Retain the source producer's complete availability for body CFGs.
    /// Missing input refuses metadata; it never selects standalone advice.
    #[must_use]
    pub fn from_source_input(
        context: CfgContext,
        registry: &CommandRegistry,
        input: Option<&crate::analyser::ResolvedAnalysisInput>,
    ) -> Self {
        Self::from_metadata_input(
            context,
            registry,
            crate::registry_invocation::InvocationMetadataInput::SuppliedSource(input),
        )
    }

    /// Retain the original metadata ingress purpose without converting absent
    /// supplied input into standalone catalogue compatibility.
    #[must_use]
    pub fn from_metadata_input(
        context: CfgContext,
        registry: &CommandRegistry,
        metadata: crate::registry_invocation::InvocationMetadataInput<'_>,
    ) -> Self {
        Self {
            context,
            command_classes: CfgCommandClasses::from_metadata_input(registry, metadata),
        }
    }

    /// Borrow the closed binding lattice shared by downstream CFG and codegen
    /// consumers.
    #[doc(hidden)]
    #[must_use]
    pub fn command_bindings(&self) -> &ModuleCommandBindings {
        &self.context.3
    }
}

/// Retain body-keyed caller-frame, parameter and outer-place facts alongside
/// their actual original Module and command-binding owners.
#[must_use]
pub fn prepare_cfg_context(module: &Module) -> CfgContext {
    let registry = module.resolved_registry();
    prepare_cfg_context_with_registry(module, registry)
}

/// Prepare the module-wide CFG context against the exact registry that
/// lowered the module.
#[must_use]
pub fn prepare_cfg_context_with_registry(
    module: &Module,
    registry: &CommandRegistry,
) -> CfgContext {
    prepare_cfg_context_bundle(module, registry).context
}

#[must_use]
#[doc(hidden)]
pub fn prepare_cfg_context_bundle(
    module: &Module,
    registry: &CommandRegistry,
) -> PreparedCfgContext {
    let command_bindings = ModuleCommandBindings::analyse(module, registry);
    let frame_effects::FrameEffectSummaries {
        caller: upvar_procs,
        global: global_write_procs,
    } = frame_effects::detect_frame_effect_bodies(module, registry);
    let proc_params = module
        .summary_procedure_bodies()
        .filter(|(id, procedure)| {
            module.procedure_ids.owns(**id)
                && module
                    .retained_source_bindings
                    .as_ref()
                    .is_some_and(|owner| {
                        owner.matches_module(module, registry)
                            && owner.matches_summary_body(**id, procedure)
                    })
        })
        .map(|(id, procedure)| (*id, procedure.params.clone()))
        .collect();
    let procedure_summary_owners = OriginalProcedureSummaryOwners::for_module(module, registry);
    PreparedCfgContext {
        context: (
            upvar_procs,
            proc_params,
            global_write_procs,
            command_bindings,
            procedure_summary_owners,
        ),
        command_classes: CfgCommandClasses::from_metadata_input(
            registry,
            if module
                .retained_source_bindings
                .as_ref()
                .is_some_and(|retained| retained.positioned_for_module(module, registry).is_some())
                && match &module.source_entry.metadata_context {
                    CfgMetadataContext::SuppliedSource(_) => {
                        crate::registry_invocation::InvocationMetadataContext::for_module(
                            registry, module,
                        )
                        .is_some()
                    }
                    _ => module.source_metadata_input.is_none(),
                }
            {
                crate::registry_invocation::InvocationMetadataInput::Retained(
                    &module.source_entry.metadata_context,
                )
            } else {
                crate::registry_invocation::InvocationMetadataInput::SuppliedSource(None)
            },
        ),
    }
}
