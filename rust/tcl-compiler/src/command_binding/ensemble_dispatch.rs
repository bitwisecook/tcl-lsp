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

//! A literal ensemble's call dispatches like an interpreter alias: the
//! subcommand word selects a command prefix from the table frozen when the
//! ensemble was created, and the remaining words follow that prefix.

use super::{
    AllocationIncarnation, Arc, BTreeMap, BTreeSet, BindingKind, CommandAllocation, MayBinding,
    ModuleCommandBindings, SourceCommandBindings, SourceCommandKey, SourceCommandTarget,
    SourceExecutionContext, SourceNamespaceKey, SourceOutcomes, boxed_source_branch,
    source_binding,
};
use crate::registry_invocation::EffectiveInvocationWord;
use tcl_registry::completion::CompletionCode;
use tcl_registry::completion_route::InvocationCompletionRoute as Route;
use tcl_registry::{EnsembleDispatchTable, EnsembleDispatchTarget};

/// Frozen tables by the allocation of the command each ensemble created;
/// `None` once its configuration may have changed.
pub(super) type EnsembleDispatches =
    BTreeMap<CommandAllocation, Option<Arc<FrozenEnsembleDispatch>>>;

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) struct FrozenEnsembleDispatch {
    namespace: SourceNamespaceKey,
    table: EnsembleDispatchTable,
}

impl ModuleCommandBindings {
    /// Freeze the table of the ensemble command just installed at `key`. A
    /// repeated allocation cannot tell its executions' tables apart.
    pub(super) fn freeze_ensemble_dispatch(
        &mut self,
        key: &SourceCommandKey,
        declaration: u32,
        namespace: &SourceNamespaceKey,
        table: Option<&EnsembleDispatchTable>,
    ) {
        let allocations = self
            .binding_alternatives(key)
            .into_iter()
            .filter_map(|binding| match binding {
                MayBinding::Target(target)
                    if target.kind == BindingKind::Command
                        && target.implementation_generation == declaration =>
                {
                    target.implementation_allocation
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        for allocation in allocations {
            let frozen = table
                .filter(|_| allocation.incarnation != AllocationIncarnation::RepeatedFresh)
                .map(|table| {
                    Arc::new(FrozenEnsembleDispatch {
                        namespace: namespace.clone(),
                        table: table.clone(),
                    })
                });
            Arc::make_mut(&mut self.ensemble_dispatches).insert(allocation, frozen);
        }
    }

    /// A reconfiguration, or code this lattice cannot see, may have changed
    /// any ensemble's table.
    pub(super) fn withdraw_ensemble_dispatches(&mut self) {
        if self.ensemble_dispatches.values().any(Option::is_some) {
            for frozen in Arc::make_mut(&mut self.ensemble_dispatches).values_mut() {
                *frozen = None;
            }
        }
    }

    /// A table survives a join only where both paths agree on it.
    pub(super) fn join_ensemble_dispatches(&mut self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.ensemble_dispatches, &other.ensemble_dispatches) {
            return false;
        }
        let mut changed = false;
        for (allocation, incoming) in other.ensemble_dispatches.iter() {
            let joined = match self.ensemble_dispatches.get(allocation) {
                Some(current) if current == incoming || current.is_none() => continue,
                Some(_) => None,
                None => incoming.clone(),
            };
            Arc::make_mut(&mut self.ensemble_dispatches).insert(allocation.clone(), joined);
            changed = true;
        }
        changed
    }

    fn frozen_ensemble_dispatch(
        &self,
        target: &SourceCommandTarget,
    ) -> Option<Arc<FrozenEnsembleDispatch>> {
        if target.kind != BindingKind::Command || target.registry_backed {
            return None;
        }
        self.ensemble_dispatches
            .get(target.implementation_allocation.as_ref()?)?
            .clone()
    }
}

impl SourceCommandBindings {
    /// Walk a call of a literal ensemble as the call of its selected target.
    /// `None` leaves the call unknown: a computed subcommand word, a miss
    /// answered by an `-unknown` handler, or a table that may have changed.
    pub(super) fn walk_ensemble_dispatch(
        &mut self,
        site: u32,
        original_words: Option<&[crate::ir::WordExpr]>,
        effective: &[EffectiveInvocationWord],
        state: &mut ModuleCommandBindings,
        target: &SourceCommandTarget,
        context: SourceExecutionContext<'_>,
    ) -> Option<SourceOutcomes> {
        let frozen = state.frozen_ensemble_dispatch(target)?;
        if matches!(
            effective.first(),
            None | Some(
                EffectiveInvocationWord::Expanded
                    | EffectiveInvocationWord::KnownExpansion(_)
                    | EffectiveInvocationWord::KnownByteExpansion(_)
            )
        ) {
            return None;
        }
        let selected = match effective.get(1) {
            None => None,
            Some(EffectiveInvocationWord::Literal(word)) => frozen.table.select(word),
            Some(_) => return None,
        };
        let Some(selected) = selected else {
            if frozen.table.unknown_handler && effective.len() > 1 {
                return None;
            }
            // `wrong # args` or `unknown or ambiguous subcommand`.
            return Some(SourceOutcomes::invocation(
                state,
                Route::Tcl(CompletionCode::Error),
            ));
        };
        let (head, prefix) = match selected {
            EnsembleDispatchTarget::Qualified(words) => {
                let (head, rest) = words.split_first()?;
                let head = if head.starts_with("::") {
                    head.clone()
                } else {
                    tcl_syntax::naming::rooted_member_key(&frozen.namespace.display()?, head)
                };
                (head, rest)
            }
            EnsembleDispatchTarget::Relative(name) => (name.clone(), &[][..]),
        };
        let binding = source_binding(state, &head, &frozen.namespace);
        let incoming = boxed_source_branch(state);
        let mut outcomes = SourceOutcomes::default();
        if binding.unknown {
            let mut residual = incoming.clone();
            residual.mark_opaque_binding_mutation();
            outcomes.join(&SourceOutcomes::invocation(&residual, Route::Unknown));
        }
        if binding.may_be_absent {
            outcomes.add_abrupt(Route::UnknownAbrupt, &incoming);
        }
        let context = if binding.unknown || binding.may_be_absent || binding.targets.len() != 1 {
            context.conditional()
        } else {
            context
        };
        // The subcommand word is consumed: the dispatched call keeps the
        // ensemble's head and the written words after the subcommand, so each
        // argument retains its own source provenance.
        let projected = original_words.and_then(|words| {
            super::named_arguments::ProjectedNamedArguments::capture(
                words, effective, &incoming, &context, 1,
            )
            .ok()
        });
        let projected_context = projected
            .as_ref()
            .map(|projected| projected.context(&context));
        for selected in &binding.targets {
            let mut branch = incoming.clone();
            if selected.registry_backed {
                // A registry command reads its written operands, which the
                // ensemble's own words are not.
                branch.mark_opaque_binding_mutation();
                outcomes.join(&SourceOutcomes::invocation(&branch, Route::Unknown));
                continue;
            }
            let mut selected = selected.clone();
            selected.prepended.extend(
                prefix
                    .iter()
                    .map(|word| EffectiveInvocationWord::Literal(word.clone())),
            );
            outcomes.join(&match (&projected, &projected_context) {
                (Some(projected), Some(projected_context)) => self.walk_document_target(
                    site,
                    Some(&projected.words),
                    &projected.effective,
                    &mut branch,
                    &selected,
                    **projected_context,
                ),
                _ => self.walk_document_target(
                    site,
                    None,
                    &effective[1..],
                    &mut branch,
                    &selected,
                    context,
                ),
            });
        }
        outcomes.publish(state);
        Some(outcomes)
    }
}
