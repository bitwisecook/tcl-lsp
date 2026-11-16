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

//! What a resolved callee does to the frame of its call: registry write roles, `upvar` summaries and outer-scope writes.

use super::{CfgBuilder, ResolvedUpvarEffects, statement_has_literal_head, summary_candidates};
use crate::ir::{CommandTokens, Statement};
use rustc_hash::FxHashSet;
use std::collections::BTreeSet;

impl CfgBuilder<'_> {
    /// Registry-owned variable writes projected in the function's actual
    /// command-resolution context. A receiver-selected relative head may be
    /// shadowed by an arbitrary object-local command and therefore widens the
    /// whole variable frame.
    pub(super) fn variable_write_projection(
        &self,
        stmt: &Statement,
    ) -> tcl_registry::VariableWriteProjection {
        let (Statement::Call { command: head, .. } | Statement::Barrier { command: head, .. }) =
            stmt
        else {
            return tcl_registry::VariableWriteProjection::default();
        };
        let Some(namespace) = self.invocation_namespace.for_head_context(head) else {
            if !statement_has_literal_head(stmt) {
                return tcl_registry::VariableWriteProjection::default();
            }
            return tcl_registry::VariableWriteProjection {
                literal_names: Vec::new(),
                read_before_write_names: Vec::new(),
                opaque_variable_frame: true,
            };
        };
        self.command_bindings
            .variable_write_projection(stmt, self.registry, namespace.as_ref())
    }

    /// Join caller-frame summaries through exact original call receipts and
    /// their frozen effective arguments.
    pub(super) fn direct_upvar_effects(&self, stmt: &Statement) -> ResolvedUpvarEffects {
        if !matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. }) {
            return ResolvedUpvarEffects::default();
        }
        self.source_upvar_effects(self.original_tokens_for_statement(stmt))
    }

    fn source_upvar_effects(&self, tokens: Option<&CommandTokens>) -> ResolvedUpvarEffects {
        let mut combined = ResolvedUpvarEffects::default();
        // A document declaration that states its own frame (or none) reaches
        // no variable of the caller's.
        if tokens.is_some_and(|tokens| self.declares_no_caller_frame(tokens)) {
            return combined;
        }
        let calls =
            tokens.and_then(|tokens| self.procedure_summary_owners.calls(tokens, self.registry));
        let Some(calls) = calls else {
            combined.has_unresolvable_target = true;
            combined.opaque_arguments = true;
            combined.frame_barrier = crate::dynamic_names::DynamicNameBarrier::OPAQUE_SCRIPT;
            return combined;
        };
        if self.widen_oo_dispatch
            && let Some(tokens) = tokens
            && let Some(binding) = tokens.source_binding.as_ref()
            && let Some(head) = summary_candidates::original_head(tokens)
            && binding
                .lookup_command_word(&head)
                .targets
                .iter()
                .any(|target| {
                    target.registry_backed
                        && matches!(
                            self.registry.method_dispatch_keyword(&target.command),
                            Some(
                                tcl_registry::MethodDispatchKind::SelfDispatch
                                    | tcl_registry::MethodDispatchKind::NextChain
                            )
                        )
                })
        {
            combined.has_unresolvable_target = true;
            combined.frame_barrier =
                combined
                    .frame_barrier
                    .union(crate::dynamic_names::DynamicNameBarrier {
                        writes: true,
                        destroys: false,
                        reads: false,
                    });
        }
        for (id, words) in calls {
            let facts = words
                .words
                .iter()
                .skip(1)
                .map(|word| {
                    crate::registry_invocation::effective_invocation_word(
                        word,
                        self.config.escapes,
                        tcl_syntax::word_rules::WordValueRules::from_config(&self.config),
                    )
                })
                .collect::<Vec<_>>();
            let arguments = facts
                .iter()
                .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
                .collect::<Vec<_>>();
            self.extend_upvar_effects(
                &mut combined,
                id,
                tcl_registry::InvocationArguments::structured(&arguments),
            );
        }
        combined
    }

    /// Fold one binding-resolved terminal user procedure into a caller-frame
    /// effect set. Both direct statements and parsed embedded commands enter
    /// here, so their parameter projection and barrier semantics cannot drift.
    fn extend_upvar_effects(
        &self,
        combined: &mut ResolvedUpvarEffects,
        id: crate::ir::ProcedureId,
        arguments: tcl_registry::InvocationArguments<'_>,
    ) {
        let (Some(info), Some(params)) = (self.upvar_procs.get(&id), self.proc_params.get(&id))
        else {
            combined.has_unresolvable_target = true;
            combined.opaque_arguments = true;
            combined.frame_barrier = combined
                .frame_barrier
                .union(crate::dynamic_names::DynamicNameBarrier::OPAQUE_SCRIPT);
            return;
        };
        let effects = info.caller_side_effects(arguments, params);
        combined.opaque_arguments |= effects.opaque;
        combined.has_unresolvable_target |= info.has_unresolvable_caller_target;
        combined.frame_barrier = combined.frame_barrier.union(info.caller_frame_barrier());
        for name in effects.defs {
            if !combined.defs.contains(&name) {
                combined.defs.push(name);
            }
        }
    }

    /// The outer-scope names the procedure a direct call statement reaches
    /// writes ([`GlobalWriteInfo::names`]).
    pub(super) fn direct_global_writes(&self, stmt: &Statement) -> Vec<String> {
        self.source_global_writes(self.original_tokens_for_statement(stmt))
            .0
    }

    pub(super) fn source_global_writes(
        &self,
        tokens: Option<&CommandTokens>,
    ) -> (Vec<String>, bool) {
        if tokens.is_some_and(|tokens| self.declares_no_caller_frame(tokens)) {
            return (Vec::new(), false);
        }
        let Some(calls) = tokens.and_then(|tokens| {
            self.procedure_summary_owners
                .may_calls(tokens, self.registry)
        }) else {
            return (Vec::new(), true);
        };
        let mut names = BTreeSet::new();
        let mut opaque = calls.unknown_alternatives;
        for id in calls.bodies {
            match self.global_write_procs.get(&id) {
                Some(info) => {
                    names.extend(info.names.iter().cloned());
                    opaque |= info.opaque_global_frame;
                }
                None => opaque = true,
            }
        }
        (names.into_iter().collect(), opaque)
    }

    /// Whole-frame blindness the statement's calls impose on *this*
    /// function, as distinct from the per-name `defs` widening
    /// [`Self::upvar_invalidated`] applies.
    ///
    /// A callee that runs `uplevel 1 $body` — or aliases a caller-frame name
    /// it cannot place — reaches names no `defs` list can enumerate.
    /// Recorded flow-insensitively on the function, in the same lattice and
    /// the same abstention direction as the dynamic-name barrier
    /// ([`crate::dynamic_names`]).
    /// Whether the call `tokens` reaches only a document-declared command
    /// whose declaration states it runs in its own frame or in none.
    fn declares_no_caller_frame(&self, tokens: &CommandTokens) -> bool {
        summary_candidates::original_head(tokens)
            .or_else(|| tokens.argv_texts.first().cloned())
            .is_some_and(|head| self.declared_head_has_no_caller_frame(&head))
    }

    /// [`Self::declares_no_caller_frame`] for a literal head.
    pub(super) fn declared_head_has_no_caller_frame(&self, head: &str) -> bool {
        let Some(namespace) = self.invocation_namespace.for_head_context(head) else {
            return false;
        };
        self.command_bindings
            .names_only_a_declared_command(head, namespace.as_ref())
            && matches!(
                self.command_bindings
                    .declared_frame_effects()
                    .get(&tcl_registry::rooted_command_descriptor_label(head)),
                Some(None)
            )
    }

    pub(super) fn upvar_effects_from_commands(
        &self,
        commands: &[Vec<crate::ir_helpers::CommandWord>],
        tokens: Option<&CommandTokens>,
    ) -> ResolvedUpvarEffects {
        let mut combined = ResolvedUpvarEffects::default();
        if commands.is_empty() && tokens.is_none() {
            return combined;
        }
        let Some(children) = tokens.and_then(|tokens| {
            self.procedure_summary_owners
                .children(tokens, self.registry)
        }) else {
            combined.opaque_arguments = true;
            combined.frame_barrier = crate::dynamic_names::DynamicNameBarrier::OPAQUE_SCRIPT;
            return combined;
        };
        for child in &children {
            let effects = self.source_upvar_effects(Some(child));
            combined.opaque_arguments |= effects.opaque_arguments;
            combined.has_unresolvable_target |= effects.has_unresolvable_target;
            combined.frame_barrier = combined.frame_barrier.union(effects.frame_barrier);
            for name in effects.defs {
                if !combined.defs.contains(&name) {
                    combined.defs.push(name);
                }
            }
        }
        combined
    }

    /// Heads of the statement's lifted calls that every binding the walk
    /// records for that spelling resolves only to source procedures, whose
    /// frame effects [`Self::upvar_effects_from_commands`] and
    /// [`Self::global_write_defs_from_commands`] read from their summaries.
    pub(super) fn summarised_embedded_heads(
        &self,
        tokens: Option<&CommandTokens>,
    ) -> FxHashSet<String> {
        let Some(children) = tokens.and_then(|tokens| {
            self.procedure_summary_owners
                .children(tokens, self.registry)
        }) else {
            return FxHashSet::default();
        };
        let mut summarised = FxHashSet::default();
        let mut other = FxHashSet::default();
        for child in &children {
            let Some(head) = summary_candidates::original_head(child) else {
                continue;
            };
            if child.source_binding.as_ref().is_some_and(
                super::super::command_binding::SourceInvocationBinding::targets_only_source_procedures,
            ) {
                summarised.insert(head);
            } else {
                other.insert(head);
            }
        }
        summarised.retain(|head| !other.contains(head));
        summarised
    }

    pub(super) fn global_write_defs_from_commands(
        &self,
        commands: &[Vec<crate::ir_helpers::CommandWord>],
        tokens: Option<&CommandTokens>,
    ) -> (Vec<String>, bool) {
        if commands.is_empty() && tokens.is_none() {
            return (Vec::new(), false);
        }
        let Some(children) = tokens.and_then(|tokens| {
            self.procedure_summary_owners
                .children(tokens, self.registry)
        }) else {
            return (Vec::new(), true);
        };
        let mut names = BTreeSet::new();
        let mut opaque = false;
        for child in &children {
            let (writes, unknown) = self.source_global_writes(Some(child));
            names.extend(writes);
            opaque |= unknown;
        }
        (names.into_iter().collect(), opaque)
    }
}
