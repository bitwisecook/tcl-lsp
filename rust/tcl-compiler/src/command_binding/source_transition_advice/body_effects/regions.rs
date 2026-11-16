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

//! The body regions of an invocation: prepared branches and the possible effects they retain.

use super::super::{
    AdviceCell, AdviceGraph, AdviceInvocation, AdviceNamingPolicy,
    OriginalSourceCommandTransitionAdvice, SourceCommandTransitionObligation,
};
use super::{BodyWalk, MAX_DEPTH, boxed_body_advice, boxed_region_words, original_operand_effects};
#[cfg(test)]
use crate::command_binding::source_transition_advice::body_effects::trace_body_inventory;
use std::sync::Arc;
use tcl_lexer::NativeWord;

struct PreparedBodyRegionBranch {
    pub(super) graph: AdviceGraph,
    pub(super) logical_scope:
        Option<Arc<crate::command_binding::OriginalDeclaredLogicalBodyContext>>,
}

impl BodyWalk<'_, '_> {
    pub(super) fn normal_case_regions(
        &self,
        words: &crate::registry_invocation::source_structure::OriginalRegistryWords,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> Option<Vec<crate::registry_invocation::OriginalSourceScriptBody>> {
        self.context.normal_case_source_layout(schema)?;
        let owner = words.normal_source_case_bodies(self.context.context)?;
        Some(owner.bodies()?.to_vec())
    }

    pub(super) fn regions(
        &mut self,
        graph: &mut AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        depth: usize,
    ) -> Option<()> {
        if depth > MAX_DEPTH {
            return None;
        }
        let words = boxed_region_words(self.context, advice);
        #[cfg(test)]
        trace_body_inventory(
            "regions.original_vector",
            advice
                .original_words()
                .first()
                .map(|word| word.span().start()),
            words.is_some(),
            advice.original_words().len(),
        );
        let words = words?;
        let interpreter = schema.semantics.body_interpreter.resolve_with(|ordinal| {
            std::str::from_utf8(words.arguments().get(ordinal)?.literal_bytes()?).ok()
        });
        if interpreter != tcl_registry::world_effect::InterpreterScope::Current {
            return None;
        }
        let lifecycle = advice.obligations().contains(
            &SourceCommandTransitionObligation::ConditionalCapturedLifecycleSourceApplicability,
        ) && self
            .context
            .c_source_lifecycle_selection(graph, schema)
            .is_some();
        let regions = if lifecycle {
            Some(words.source_script_bodies(self.context.context))
        } else {
            words
                .source_script_bodies_for_mutation_coverage(self.context.context)
                .or_else(|| self.normal_case_regions(&words, schema))
        };
        #[cfg(test)]
        trace_body_inventory(
            "regions.coverage",
            advice
                .original_words()
                .first()
                .map(|word| word.span().start()),
            regions.is_some(),
            regions.as_ref().map_or(0, Vec::len),
        );
        let regions = regions?;
        // Checked empty coverage supplies no evaluated child mutation and remains
        // complete. Reference-only bodies are one such complete case.
        let original = graph.clone();
        let runs_once = self.body_runs_once(schema, lifecycle, regions.len());
        for region in regions {
            if !region.matches_source(self.context.origin.source_image(), self.context.config)
                || !region.matches_context(self.context.context)
            {
                return None;
            }
            if self.declares_tcloo_members_only(&region) {
                continue;
            }
            let span = region.content_span();
            // Changing namespace source coordinates retain their independently
            // selected policy; no frame or body execution follows.
            let mut prepared =
                self.prepare_region_branch(&original, advice, schema, &region, lifecycle)?;
            let catalogue = self.catalogue_body.clone();
            if catalogue.is_some()
                && (schema.semantics.body_kind != tcl_registry::BodyKind::Plain
                    || schema
                        .semantics
                        .traits
                        .intersects(tcl_registry::traits::FRAME_REACH_TRAITS)
                    || schema.facts().frame_effect.is_some())
            {
                self.catalogue_body = Some(super::super::OriginalSourceCatalogueBody::Script(
                    Arc::new(region.clone()),
                ));
            }
            let source_body = (self.inventory.is_some()
                && (schema.semantics.body_kind == tcl_registry::BodyKind::Plain
                    || prepared.graph.c_source_namespace.is_some()))
            .then(|| Arc::new(region));
            let parent = std::mem::replace(&mut self.source_body, source_body);
            let parent_scope =
                std::mem::replace(&mut self.logical_scope, prepared.logical_scope.take());
            let walked = self.script(&mut prepared.graph, span, depth);
            self.logical_scope = parent_scope;
            self.source_body = parent;
            self.catalogue_body = catalogue;
            walked?;
            if runs_once {
                graph.adopt_completed_body_world(prepared.graph);
            } else {
                Self::retain_prepared_region_effects(
                    graph,
                    &original,
                    prepared,
                    advice.original_words(),
                );
            }
        }
        Some(())
    }

    /// Whether this invocation is a `TclOO` definer whose every script region
    /// only declares members. Unavailable coverage answers no.
    pub(super) fn definition_declares_members_only(
        &self,
        graph: &AdviceGraph,
        invocation: AdviceInvocation<'_>,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
    ) -> bool {
        if schema
            .authored_source_definition_body_grammar()
            .is_none_or(|grammar| grammar.family != tcl_registry::definer::DefinerFamily::TclOo)
            || !original_operand_effects(invocation.native).is_empty()
        {
            return false;
        }
        boxed_body_advice(self.context, invocation, graph, schema)
            .and_then(|advice| boxed_region_words(self.context, &advice))
            .and_then(|words| {
                words.source_script_bodies_for_mutation_coverage(self.context.context)
            })
            .is_some_and(|regions| {
                regions
                    .iter()
                    .all(|region| self.declares_tcloo_members_only(region))
            })
    }

    /// A namespace body is its command's one script and runs once, in the
    /// same invocation: whatever follows the command in this script runs only
    /// after the body completed. Its command-world effects are then this
    /// script's own, where every other region stays a possibility.
    fn body_runs_once(
        &self,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        lifecycle: bool,
        regions: usize,
    ) -> bool {
        matches!(self.context.policy, AdviceNamingPolicy::Native(_))
            && !lifecycle
            && regions == 1
            && schema.semantics.body_kind == tcl_registry::BodyKind::Structural
            && schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::DECLARES_NAMESPACE)
    }

    /// A `TclOO` definition body whose every command is a written member of
    /// its grammar with no substitution. Such a body declares methods,
    /// relations and state on the class; it publishes no command and runs no
    /// member body, so it leaves the command world as it found it.
    fn declares_tcloo_members_only(
        &self,
        region: &crate::registry_invocation::OriginalSourceScriptBody,
    ) -> bool {
        let Some(members) =
            region.definition_member_region(self.context.context, region.content_span())
        else {
            return false;
        };
        members.grammar().family == tcl_registry::definer::DefinerFamily::TclOo
            && members
                .complete_original_commands()
                .is_some_and(|commands| {
                    commands.iter().all(|command| {
                        original_operand_effects(command).is_empty()
                            && members.script_bodies(command).is_some()
                    })
                })
    }

    #[inline(never)]
    fn prepare_region_branch(
        &self,
        original: &AdviceGraph,
        advice: &OriginalSourceCommandTransitionAdvice,
        schema: &tcl_registry::ResolvedInvocation<'_, '_>,
        region: &crate::registry_invocation::OriginalSourceScriptBody,
        lifecycle: bool,
    ) -> Option<Box<PreparedBodyRegionBranch>> {
        let span = region.content_span();
        let mut branch = original.clone();
        let mut child_scope = self.logical_scope.clone();
        if span.start() != span.end()
            && schema.semantics.body_kind != tcl_registry::BodyKind::Plain
            && !lifecycle
        {
            let invocation = AdviceInvocation {
                native: advice.original_words(),
                head: advice.original_head(),
                arguments: advice.arguments(),
                lineage: advice.lineage(),
            };
            match self.context.policy {
                AdviceNamingPolicy::Logical(input) => {
                    if schema.semantics.frame_effect.is_some_and(|frame| {
                        frame.layout == tcl_registry::FrameArgLayout::ScriptInSelectedFrame
                    }) {
                        let scope = self.logical_scope.as_ref()?;
                        if !scope.matches_input(input)
                            || !scope.owns_words(advice.original_words())
                            || original.logical_namespace.as_ref() != Some(scope.namespace_recipe())
                        {
                            return None;
                        }
                        let tcl_registry::frame_effect::FrameArgumentResolution::Valid {
                            level,
                            ..
                        } = schema.authored_logical_source_frame_argument_layout()?
                        else {
                            return None;
                        };
                        let body = Arc::new(region.clone());
                        let child = if level.is_global_frame() {
                            scope.with_global_body(body)?
                        } else if level.is_current_frame() {
                            scope.with_body(body)?
                        } else {
                            return None;
                        };
                        branch.logical_namespace = Some(child.namespace_recipe().clone());
                        child_scope = Some(Arc::new(child));
                    } else {
                        let selected = self
                            .context
                            .logical_namespace_scope(original, invocation, schema)?;
                        let child = selected.body_scope(
                            Arc::new(region.clone()),
                            input,
                            self.logical_scope.clone(),
                        )?;
                        branch.logical_namespace = Some(child.namespace_recipe().clone());
                        child_scope = Some(child);
                    }
                }
                AdviceNamingPolicy::Native(_) => {
                    branch.c_source_namespace = Some(
                        self.context
                            .c_source_namespace_scope(original, invocation, schema)?,
                    );
                }
            }
        }
        Some(Box::new(PreparedBodyRegionBranch {
            graph: branch,
            logical_scope: child_scope,
        }))
    }

    #[inline(never)]
    fn retain_prepared_region_effects(
        graph: &mut AdviceGraph,
        original: &AdviceGraph,
        prepared: Box<PreparedBodyRegionBranch>,
        operation: &[NativeWord],
    ) {
        Self::retain_possible_effects(graph, original, prepared.graph, operation);
    }

    pub(super) fn retain_possible_effects(
        graph: &mut AdviceGraph,
        original: &AdviceGraph,
        branch: AdviceGraph,
        operation: &[NativeWord],
    ) {
        // An untouched branch still shares the original's cells: nothing to compare.
        let cells_changed = !branch.cells.shares(&original.cells);
        for (key, cell) in original.cells.iter().filter(|_| cells_changed) {
            if branch.cells.get(key) != Some(cell) {
                // Possible original source mutations block an earlier schema;
                // they do not prove that a delete or replacement succeeded.
                if graph.native_baseline
                    && matches!(cell, AdviceCell::Descriptor(_, lineage) if lineage.is_empty())
                {
                    // The authentic entry schema is a source possibility only;
                    // a conditional branch does not prove its replacement ran.
                    graph.record_uncertainty(operation);
                } else {
                    graph.cells.insert(key.clone(), AdviceCell::Shadowed);
                }
            }
        }
        // A command first created by a possible branch is not installed, but
        // a later lookup may no longer fall through its coordinate.
        for key in branch.cells.keys().filter(|_| cells_changed) {
            if !original.cells.contains_key(key) {
                graph.possible_occupants.insert(key.clone());
            }
        }
        graph.possible_occupants.extend(branch.possible_occupants);
        for namespace in original
            .namespace_paths
            .keys()
            .chain(branch.namespace_paths.keys())
        {
            if original.namespace_paths.get(namespace) != branch.namespace_paths.get(namespace) {
                graph.unknown_paths.insert(namespace.clone());
                graph.namespace_paths.remove(namespace);
            }
        }
        for namespace in original
            .namespace_exports
            .keys()
            .chain(branch.namespace_exports.keys())
        {
            if original.namespace_exports.get(namespace) != branch.namespace_exports.get(namespace)
            {
                graph.unknown_exports.insert(namespace.clone());
            }
        }
        graph.unknown_paths.extend(branch.unknown_paths);
        graph.unknown_exports.extend(branch.unknown_exports);
        for debt in branch.arity_debts {
            if !graph.arity_debts.contains(&debt) {
                graph.arity_debts.push(debt);
            }
        }
        for operation in branch.uncertain_operations {
            if !graph.uncertain_operations.contains(&operation) {
                graph.uncertain_operations.push(operation);
            }
        }
    }
}
