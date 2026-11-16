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

//! An alias transition the original command world covers at one invocation.

use super::super::{CommandAllocationSite, ModuleCommandBindings};
use super::{OriginalCommandPublication, OriginalCommandPublicationKind};
use crate::signature_scan::scope::SignatureSourceNameInput;
use tcl_registry::{CommandBindingTransition, TransitionSubject};

/// Complete modeled alias binding effect for an original caller scan. This
/// carries source inventory coverage only, never handler entry or completion.
pub(crate) struct OriginalCoveredAliasTransition {
    _publication: OriginalCommandPublication,
    _target: super::super::CommandAllocation,
}

impl super::super::SourceInvocationBinding {
    fn original_covered_alias_metadata(
        &self,
        tokens: &crate::ir::CommandTokens,
        module: &crate::ir::Module,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<()> {
        self.original_invocation_metadata_for_input(tokens, input, registry)?;
        let metadata = self.original_invocation_metadata_for_module(tokens, module, registry)?;
        if metadata.source_analysis_input() != Some(input)
            || self.unknown
            || self.may_be_absent
            || !self.original_argument_completion.is_normal()
        {
            return None;
        }
        Some(())
    }

    /// Join an original alias transition to its exact retained post-transfer
    /// publication and closed source procedure target. An independent source
    /// interpretation, final world or name label cannot replace this owner.
    pub(crate) fn original_covered_alias_transition(
        &self,
        tokens: &crate::ir::CommandTokens,
        module: &crate::ir::Module,
        input: &crate::analyser::ResolvedAnalysisInput,
        registry: &tcl_registry::CommandRegistry,
        transition: &CommandBindingTransition,
    ) -> Option<OriginalCoveredAliasTransition> {
        let CommandBindingTransition::Alias {
            source_interpreter,
            alias,
            target_interpreter,
            target,
            arguments,
            target_lookup,
        } = transition
        else {
            return None;
        };
        self.original_covered_alias_metadata(tokens, module, input, registry)?;
        let retained = module.retained_source_bindings.as_deref()?;
        retained.owns_original_tokens(tokens).then_some(())?;
        let positioned = retained.positioned_for_module(module, registry)?;
        let site = self.invocation_site()?;
        let before = &self.lookup_state.as_ref()?.state;
        if before.has_opaque_domain() || before.source_step_observed() {
            return None;
        }
        let operands = self.original_variable_operands.as_ref()?;
        let operand = |subject: &TransitionSubject| {
            operands.input(subject.argument_index()?, &before.source_variables)
        };
        if !operand(source_interpreter)?.bytes().is_empty()
            || !operand(target_interpreter)?.bytes().is_empty()
        {
            return None;
        }
        let alias_input = operand(alias)?;
        let target_input = operand(target)?;
        let prefix = arguments.iter().map(operand).collect::<Option<Vec<_>>>()?;
        let mut selected = None;
        let mut continuations = positioned
            .points
            .iter()
            .filter(|point| point.continuation_of.as_ref() == Some(site));
        let first = continuations.next()?;
        for point in std::iter::once(first).chain(continuations) {
            let state = &point.state;
            if point.declaration_preview
                || point.dispatch
                || point.namespace_key != self.lookup_namespace_key
                || state.variable_frame != self.variable_frame
                || state.current_source_origin.as_ref() != Some(&site.source)
                || state.baseline != before.baseline
                || state.has_opaque_domain()
                || state.source_step_observed()
                || state.source_execution_observed(None)
            {
                return None;
            }
            let mut rows = state
                .original_command_world
                .publications
                .iter()
                .filter(|row| {
                    row.declaration_site() == site
                        && row.kind() == OriginalCommandPublicationKind::Alias
                        && row.name_input() == alias_input
                });
            let publication = rows.next()?;
            rows.next().is_none().then_some(())?;
            let original = publication.alias_target()?;
            if original.name_input() != target_input
                || original.lookup() != *target_lookup
                || original.arguments().iter().collect::<Vec<_>>() != prefix
                || publication.implementation_allocation.as_ref()?.site != *site
            {
                return None;
            }
            let covered = original_covered_alias_target(
                state,
                before,
                publication,
                alias_input,
                module,
                retained,
                site,
            )?;
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &covered)
            {
                return None;
            }
            selected = Some(covered);
        }
        let (publication, target) = selected?;
        Some(OriginalCoveredAliasTransition {
            _publication: publication,
            _target: target,
        })
    }
}

fn original_covered_alias_target(
    state: &ModuleCommandBindings,
    before: &ModuleCommandBindings,
    publication: &OriginalCommandPublication,
    alias_input: &SignatureSourceNameInput,
    module: &crate::ir::Module,
    retained: &super::super::RetainedSourceModuleBindings,
    site: &CommandAllocationSite,
) -> Option<(OriginalCommandPublication, super::super::CommandAllocation)> {
    let root = state.source_root_namespace_key()?;
    let key = state.original_command_key_for_slot(publication.slot(), publication.policy())?;
    let bindings = state.original_bindings_for_key(&key)?;
    let super::super::MayBinding::Target(alias) = bindings.first()? else {
        return None;
    };
    if bindings.len() != 1 {
        return None;
    }
    let previous = before.original_bindings_for_key(&key)?;
    if previous != std::collections::BTreeSet::from([super::super::MayBinding::Missing]) {
        return None;
    }
    if alias.kind != super::super::BindingKind::Alias
        || !publication.has_implementation_allocation(alias.implementation_allocation.as_ref()?)
    {
        return None;
    }
    let selection = state.original_targets_for_input(
        alias_input,
        &root,
        super::super::CommandTargetLookup::NamedSlots,
    )?;
    if selection.unknown || selection.may_be_absent || selection.targets.is_empty() {
        return None;
    }
    let first_target = selection.targets.first()?;
    let allocation = first_target.implementation_allocation.as_ref()?;
    if allocation.site.source != site.source
        || selection.targets.iter().any(|target| {
            target.kind != super::super::BindingKind::Proc
                || target.registry_backed
                || !target.terminal
                || target.implementation_allocation.as_ref() != Some(allocation)
                || target.command != first_target.command
                || target.prepended != first_target.prepended
                || selection
                    .original_prefixes
                    .get(target)
                    .and_then(Option::as_ref)
                    .is_none()
        })
    {
        return None;
    }
    let (id, procedure) = module.source_procedure_for_allocation(allocation)?;
    if !retained.matches_original_procedure_summary(id, procedure)
        || procedure.span.start() != allocation.site.offset
    {
        return None;
    }
    Some((publication.clone(), allocation.clone()))
}
