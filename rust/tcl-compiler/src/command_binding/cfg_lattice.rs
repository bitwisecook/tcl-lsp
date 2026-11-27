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

//! The flow-sensitive CFG view of the command table.

use super::CommandToken;
use super::cfg_lookup_context::{CfgLookupContexts, CfgPointContext};
use super::module_bindings::{
    BindingInvocationMetadata, RetainedBindingRoots, apply_embedded_transitions_in,
    apply_may_invocation_transitions,
};
use super::{
    Binding, BindingBaseline, CommandBinding, InvocationBindingContext, MayBinding,
    ModuleCommandBindings, ResolvedCommandTarget,
};
use crate::cfg::{BlockId, Function as CfgFunction};
use crate::ir::Statement;
use crate::naming::normalise_qualified_name as nqn;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_names::command::BindingKind;
use tcl_registry::CommandRegistry;

pub(super) fn unavailable_cfg_entry(registry: &CommandRegistry) -> ModuleCommandBindings {
    let mut unknown = ModuleCommandBindings::initial(registry);
    let baseline = Arc::make_mut(&mut unknown.baseline);
    baseline.source_read_config = crate::var_resolve::SourceReadConfiguration::Unavailable;
    baseline.refresh_fingerprint();
    Arc::make_mut(&mut unknown.source_variables).source_read_configuration =
        crate::var_resolve::SourceReadConfiguration::Unavailable;
    unknown.mark_opaque_binding_mutation();
    unknown.loader_handler_unknown = true;
    unknown
}

pub(super) fn cfg_entry_state(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
) -> ModuleCommandBindings {
    let mut retained: Option<Arc<BindingBaseline>> = None;
    let registry_key = registry.snapshot().semantic_key();
    for statement in cfg.blocks.values().flat_map(|block| &block.statements) {
        if !statement.is_executable_invocation() {
            continue;
        }
        let tokens = crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
            &cfg.command_binding_sites,
            statement,
        );
        let Some(tokens) = tokens else {
            if statement.tokens().is_some()
                || cfg
                    .command_binding_sites
                    .iter()
                    .any(|site| site.span == statement.span() && site.source_tokens.is_some())
            {
                return unavailable_cfg_entry(registry);
            }
            continue;
        };
        let Some(binding) = tokens.source_binding.as_ref() else {
            continue;
        };
        let baseline = binding
            .lookup_state
            .as_ref()
            .map(|snapshot| &snapshot.state.baseline);
        let Some(baseline) = baseline.filter(|baseline| {
            baseline.registry_snapshot.as_ref() == Some(&registry_key)
                && retained.as_ref().is_none_or(|old| old == *baseline)
        }) else {
            return unavailable_cfg_entry(registry);
        };
        retained = Some(Arc::clone(baseline));
    }
    retained.map_or_else(
        || {
            if cfg.statement_sources.values().any(Option::is_some)
                || cfg.executed_source.is_some()
                || cfg.namespace_context.is_some()
                || !matches!(
                    &cfg.metadata_context,
                    crate::cfg_builder::CfgMetadataContext::Standalone
                )
            {
                unavailable_cfg_entry(registry)
            } else {
                ModuleCommandBindings::initial(registry)
            }
        },
        |baseline| ModuleCommandBindings::initial_entry_from_baseline(registry, baseline),
    )
}

fn transfer_cfg_statement(
    stmt: &Statement,
    state: &mut ModuleCommandBindings,
    registry: &CommandRegistry,
    point: Option<&CfgPointContext>,
    metadata: BindingInvocationMetadata<'_>,
) {
    if !stmt.is_executable_invocation() {
        return;
    }
    let Some(point) = point else {
        state.mark_opaque_binding_mutation();
        return;
    };
    let namespace = &point.namespace;
    apply_embedded_transitions_in(
        stmt,
        registry,
        state,
        namespace,
        false,
        (metadata, point.config),
    );
    if !matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. }) {
        return;
    }
    let Some(command_namespace) = namespace.for_head_context(stmt.canonical_command_or_source())
    else {
        state.mark_opaque_binding_mutation();
        return;
    };
    let mut observed = None;
    let mut retained = RetainedBindingRoots::default();
    let mut context = InvocationBindingContext {
        retained_roots: &mut retained,
        source_order_mode: false,
        metadata,
    };
    apply_may_invocation_transitions(
        stmt,
        registry,
        state,
        &mut observed,
        command_namespace.as_ref(),
        namespace,
        &mut context,
    );
}

impl CommandBinding<'_> {
    fn state_at_block(&self, block: BlockId, stmt_idx: usize) -> ModuleCommandBindings {
        let mut state = self
            .block_entry
            .get(&block)
            .cloned()
            .unwrap_or_else(|| self.seed.clone());
        if let Some(blk) = self.cfg.blocks.get(&block) {
            for (index, stmt) in blk.statements.iter().enumerate().take(stmt_idx) {
                transfer_cfg_statement(
                    stmt,
                    &mut state,
                    self.registry,
                    self.lookup_contexts.point(block, index),
                    CfgLookupContexts::metadata(self.cfg, self.registry),
                );
            }
        }
        state
    }

    /// Project the exact slot's binding without inventing a second resolver.
    #[must_use]
    pub fn binding_at(&self, block: BlockId, stmt_idx: usize, command_name: &str) -> Binding {
        let state = self.state_at_block(block, stmt_idx);
        self.lookup_contexts
            .binding_at(self.cfg, &state, block, stmt_idx, command_name)
    }

    /// Whether the source name still selects its original registry command.
    #[must_use]
    pub fn is_original_builtin_at(
        &self,
        block: BlockId,
        stmt_idx: usize,
        command_name: &str,
    ) -> bool {
        self.binding_at(block, stmt_idx, command_name)
            .is_original_builtin()
    }

    /// Every source spelling changed at a represented program point.
    #[must_use]
    pub fn rebound_names(&self) -> HashSet<String> {
        let mut names = HashSet::new();
        for block in &self.ordered_blocks {
            let mut state = self
                .block_entry
                .get(block)
                .cloned()
                .unwrap_or_else(|| self.seed.clone());
            names.extend(state.changed_command_spellings());
            if let Some(blk) = self.cfg.blocks.get(block) {
                for (index, stmt) in blk.statements.iter().enumerate() {
                    transfer_cfg_statement(
                        stmt,
                        &mut state,
                        self.registry,
                        self.lookup_contexts.point(*block, index),
                        CfgLookupContexts::metadata(self.cfg, self.registry),
                    );
                    names.extend(state.changed_command_spellings());
                }
            }
        }
        names
    }

    /// Whether some executable path has an unbounded command mutation.
    #[must_use]
    pub fn has_wildcard(&self) -> bool {
        self.ordered_blocks.iter().any(|block| {
            let end = self
                .cfg
                .blocks
                .get(block)
                .map_or(0, |block| block.statements.len());
            self.state_at_block(*block, end).opaque_domain
        })
    }
}

fn cfg_seed_with_initial(
    cfg: &CfgFunction,
    registry: &CommandRegistry,
    initial: &[(String, Binding)],
) -> ModuleCommandBindings {
    let mut seed = cfg_entry_state(cfg, registry);
    for (name, binding) in initial {
        let name = nqn(name);
        let value = match binding.kind {
            BindingKind::Unknown => MayBinding::Unknown,
            BindingKind::Opaque | BindingKind::Bottom => MayBinding::Missing,
            kind => MayBinding::Target(ResolvedCommandTarget {
                command: binding.target.clone().unwrap_or_else(|| name.clone()),
                prepended: Vec::new(),
                registry_backed: kind == BindingKind::Builtin,
                kind,
                terminal: kind != BindingKind::Alias,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                implementation_generation: 0,
                token: Some(CommandToken {
                    runtime: None,
                    origin: name.clone(),
                    declaration: 0,
                    allocation: None,
                }),
                implementation_allocation: None,
            }),
        };
        if binding.kind == BindingKind::Proc {
            seed.extend_procedure_bodies([name.clone()]);
        }
        seed.install(name, value);
    }
    seed
}

/// Flow-sensitive CFG analysis using the same state and transition kernel as
/// source interpretation and module effect summaries.
#[must_use]
pub fn analyse_command_binding<'a>(
    cfg: &'a CfgFunction,
    registry: &'a CommandRegistry,
    initial: &[(String, Binding)],
) -> CommandBinding<'a> {
    let mut preds: HashMap<BlockId, Vec<BlockId>> =
        cfg.blocks.keys().map(|id| (*id, Vec::new())).collect();
    for &id in cfg.blocks.keys() {
        for succ in cfg.block_successors(id) {
            if let Some(incoming) = preds.get_mut(&succ) {
                incoming.push(id);
            }
        }
    }
    let seed = cfg_seed_with_initial(cfg, registry, initial);
    let order = cfg.reverse_postorder();
    let lookup_contexts = CfgLookupContexts::new(cfg, registry, &seed);
    let mut block_entry = HashMap::<BlockId, ModuleCommandBindings>::new();
    let mut block_exit = HashMap::<BlockId, ModuleCommandBindings>::new();
    loop {
        let mut changed = false;
        for id in &order {
            let mut predecessors = preds
                .get(id)
                .into_iter()
                .flatten()
                .filter_map(|id| block_exit.get(id));
            let mut entry = if *id == cfg.entry {
                seed.clone()
            } else if let Some(first) = predecessors.next() {
                first.clone()
            } else {
                continue;
            };
            for predecessor in predecessors {
                entry.join(predecessor);
            }
            block_entry.insert(*id, entry.clone());
            let mut exit = entry;
            if let Some(block) = cfg.blocks.get(id) {
                for (index, stmt) in block.statements.iter().enumerate() {
                    transfer_cfg_statement(
                        stmt,
                        &mut exit,
                        registry,
                        lookup_contexts.point(*id, index),
                        CfgLookupContexts::metadata(cfg, registry),
                    );
                }
            }
            if block_exit
                .get(id)
                .is_none_or(|previous| !previous.same_state(&exit))
            {
                block_exit.insert(*id, exit);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    CommandBinding {
        block_entry,
        ordered_blocks: order,
        cfg,
        registry,
        seed,
        lookup_contexts,
    }
}
