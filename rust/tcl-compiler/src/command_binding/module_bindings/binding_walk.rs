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

//! The IR walk that collects binding states, with the transitions of embedded substitutions.

use super::super::{BindingWalkOutcome, InvocationBindingContext, ModuleCommandBindings};
use super::invocation_transitions::{
    apply_may_invocation_transitions, apply_resolved_may_transitions,
};
use super::{BindingWalkContext, RetainedBindingRoots};
use crate::ir::{Script, Statement};
use crate::ir_helpers::{evaluated_command_substitutions_with_replay, nested_bodies};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_names::namespace::SourceNamespaceKey;
use tcl_registry::CommandRegistry;

impl ModuleCommandBindings {
    /// Registry head and prepended arguments for the shared expression-word
    /// descent. CFG projection and binding replay use the same alias answer.
    pub(crate) fn resolved_embedded_head(
        &self,
        head: &str,
        namespace: &crate::ir_helpers::ExecutionNamespace,
    ) -> Option<crate::ir_helpers::ResolvedEmbeddedHead> {
        let namespace = namespace.for_head_context(head)?;
        if self.target_resolution_may_be_unknown(head, namespace.as_ref()) {
            return None;
        }
        let mut found = self.targets(head, namespace.as_ref()).into_iter();
        let target = found.next()?;
        if found.next().is_some() || !target.registry_backed {
            return None;
        }
        Some(crate::ir_helpers::ResolvedEmbeddedHead {
            command: target.command,
            prepended: target
                .prepended
                .iter()
                .map(|word| word.as_registry_word().literal().map(str::to_owned))
                .collect::<Option<Vec<_>>>()?,
        })
    }
}

/// Explicit compatibility is separate from a supplied metadata refusal.
#[derive(Clone, Copy)]
pub(in super::super) enum BindingInvocationMetadata<'a> {
    Standalone,
    Supplied(Option<crate::registry_invocation::InvocationMetadataContext<'a>>),
}

pub(super) fn observe_binding_state(
    observed: &mut Option<ModuleCommandBindings>,
    current: &ModuleCommandBindings,
) {
    if let Some(state) = observed {
        // A call with no command-table transition is still an executable
        // observation (the caller's final fallback covers that state), but
        // it must not repeatedly merge an identical sparse lattice point.
        if !state.same_state(current) {
            state.join(current);
        }
    } else {
        *observed = Some(current.clone());
    }
}

// Flow-sensitive recursive join over every structured IR form.
pub(super) fn collect_normal_binding_states(
    script: &Script,
    registry: &CommandRegistry,
    initial: &ModuleCommandBindings,
    namespace: &crate::ir::ExecutionNamespace,
    retained_roots: &mut RetainedBindingRoots,
) -> BindingWalkOutcome {
    let mut context = BindingWalkContext {
        registry,
        retained_roots,
        timeline: None,
        source_order_mode: false,
    };
    collect_binding_states(script, &mut context, initial, namespace)
}

#[allow(clippy::too_many_lines)]
pub(super) fn collect_binding_states(
    script: &Script,
    context: &mut BindingWalkContext<'_>,
    initial: &ModuleCommandBindings,
    namespace: &crate::ir::ExecutionNamespace,
) -> BindingWalkOutcome {
    // Recursive walker keeps branch joins and side effects together.
    #[allow(clippy::too_many_lines)]
    fn walk(
        script: &Script,
        context: &mut BindingWalkContext<'_>,
        current: &mut ModuleCommandBindings,
        observed: &mut Option<ModuleCommandBindings>,
        namespace: &crate::ir_helpers::ExecutionNamespace,
        depth: u32,
    ) {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            current.mark_opaque_binding_mutation();
            observe_binding_state(observed, current);
            return;
        }
        // Typed-lowering sites are sparse sidecars, but a large generated
        // script can have many statements and many sites. Index them once for
        // this script instead of scanning every site for every statement.
        // Keep the index borrowed: a repeated synthetic statement span still
        // observes every site attached to that source command, exactly as the
        // former filter loop did.
        let mut binding_sites_by_span: HashMap<_, Vec<_>> = HashMap::new();
        for site in script.command_binding_sites.iter() {
            binding_sites_by_span
                .entry(site.span)
                .or_default()
                .push(site);
        }
        let statement_spans: HashSet<_> = script.statements.iter().map(Statement::span).collect();
        for stmt in &script.statements {
            if !stmt.is_executable_invocation() {
                continue;
            }
            let source_order_entry = context.source_order_mode.then(|| current.clone());
            if let Some(timeline) = &mut context.timeline {
                timeline.record_before_substitutions(stmt.span(), current);
            }
            let conditional_substitutions = context.source_order_mode
                && crate::ir_helpers::evaluated_command_substitution_surfaces(
                    stmt,
                    context.registry,
                )
                .conditional;
            if apply_embedded_transitions(
                stmt,
                context.registry,
                current,
                namespace,
                context.source_order_mode,
            ) {
                observe_binding_state(observed, current);
            }
            if conditional_substitutions
                && source_order_entry
                    .as_ref()
                    .is_some_and(|before| !before.same_state(current))
            {
                // `&&`, `||`, ternary arms, and later elseif conditions have
                // a skipped path. A flat recovered invocation list cannot
                // select one, so retain only the bounded region uncertainty.
                current.mark_opaque_binding_mutation();
                // Embedded-barrier projection replays from this statement's
                // pre-substitution snapshot. Join the uncertainty there too,
                // so a later substitution in the same expression cannot rely
                // on the flat replay having taken an optional transition.
                if let Some(timeline) = &mut context.timeline {
                    timeline.record_before_substitutions(stmt.span(), current);
                }
            }
            if let Some(timeline) = &mut context.timeline {
                timeline.record_before_direct_call(stmt.span(), current);
            }
            let mut binding_site_widened = false;
            for site in binding_sites_by_span
                .get(&stmt.span())
                .into_iter()
                .flatten()
            {
                if !command_binding_site_is_exact(site, current, namespace) {
                    current.mark_opaque_resolution();
                    binding_site_widened = true;
                }
            }
            if binding_site_widened {
                observe_binding_state(observed, current);
            }
            if let Statement::Call { command, .. } | Statement::Barrier { command, .. } = stmt {
                if let Some(statement_namespace) = namespace.for_head_context(command) {
                    let mut invocation_context = InvocationBindingContext {
                        retained_roots: context.retained_roots,
                        source_order_mode: context.source_order_mode,
                        metadata: BindingInvocationMetadata::Standalone,
                    };
                    let observation_was_updated = apply_may_invocation_transitions(
                        stmt,
                        context.registry,
                        current,
                        observed,
                        statement_namespace.as_ref(),
                        namespace,
                        &mut invocation_context,
                    );
                    if !observation_was_updated {
                        observe_binding_state(observed, current);
                    }
                } else {
                    current.mark_opaque_resolution();
                    observe_binding_state(observed, current);
                }
            }
            match stmt {
                Statement::Block { .. } | Statement::UpFrame { .. } => {
                    for (body, body_namespace) in
                        crate::ir_helpers::nested_execution_bodies(stmt, namespace)
                    {
                        walk(body, context, current, observed, &body_namespace, depth + 1);
                    }
                }
                Statement::If {
                    clauses, else_body, ..
                } => {
                    let incoming = current.clone();
                    let mut joined: Option<ModuleCommandBindings> = None;
                    for body in clauses.iter().map(|clause| &clause.body).chain(else_body) {
                        let mut branch = incoming.clone();
                        walk(body, context, &mut branch, observed, namespace, depth + 1);
                        if let Some(state) = &mut joined {
                            if !state.same_state(&branch) {
                                state.join(&branch);
                            }
                        } else {
                            joined = Some(branch);
                        }
                    }
                    if else_body.is_none() {
                        if let Some(state) = &mut joined {
                            if !state.same_state(&incoming) {
                                state.join(&incoming);
                            }
                        } else {
                            joined = Some(incoming.clone());
                        }
                    }
                    *current = joined.unwrap_or(incoming);
                    observe_binding_state(observed, current);
                }
                _ => {
                    for body in nested_bodies(stmt) {
                        let incoming = current.clone();
                        let mut branch = incoming.clone();
                        walk(body, context, &mut branch, observed, namespace, depth + 1);
                        if !current.same_state(&branch) {
                            current.join(&branch);
                        }
                        observe_binding_state(observed, current);
                    }
                }
            }
            if context.source_order_mode
                && matches!(
                    stmt,
                    Statement::If { .. }
                        | Statement::For { .. }
                        | Statement::While { .. }
                        | Statement::Foreach { .. }
                )
            {
                if matches!(
                    stmt,
                    Statement::For { .. } | Statement::While { .. } | Statement::Foreach { .. }
                ) && let Some(before) = source_order_entry
                    .as_ref()
                    .filter(|before| !before.same_state(current))
                {
                    // One source pass cannot close a loop-carried command
                    // binding state. Publish an explicitly opaque region
                    // state rather than mistaking the first post-state for a
                    // fixed point. A loop that leaves the command world as
                    // it found it changed only variables: those it may
                    // write on a later pass are what stays unknown.
                    if before.same_command_world(current)
                        && before.logical_alias_prefixes == current.logical_alias_prefixes
                        && before.command_observers == current.command_observers
                    {
                        Arc::make_mut(&mut current.source_variables).widen();
                        current.ordinary_literal_pool = None;
                    } else {
                        current.mark_opaque_binding_mutation();
                    }
                }
                // A structured node can execute its condition/body more than
                // once or select a later branch. Join the post-region state
                // back at the source span so CFG consumers never reuse an
                // earlier precise binding after a transition on another path.
                if let Some(timeline) = &mut context.timeline {
                    timeline.record_before_substitutions(stmt.span(), current);
                    timeline.record_before_direct_call(stmt.span(), current);
                    for body in nested_bodies(stmt) {
                        timeline.join_region(body, current);
                    }
                }
            }
        }
        // A typed dependency without a surviving owner statement cannot be
        // ordered against command-table transitions. Fail closed rather than
        // silently trusting an orphaned sidecar after an IR transform.
        if binding_sites_by_span
            .keys()
            .any(|span| !statement_spans.contains(span))
        {
            current.mark_opaque_resolution();
            observe_binding_state(observed, current);
        }
    }

    let mut current = initial.clone();
    let mut observed = None;
    walk(script, context, &mut current, &mut observed, namespace, 0);
    BindingWalkOutcome {
        observed: observed.unwrap_or_else(|| current.clone()),
        post: current,
    }
}

/// Whether one typed-lowering dependency still denotes exactly the registry
/// implementation consumed by the compiler at this execution point.
fn command_binding_site_is_exact(
    site: &crate::ir::CommandBindingSite,
    bindings: &ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
) -> bool {
    let Some(source_namespace) = namespace.for_head_context(&site.binding.name) else {
        return false;
    };
    if let Some(retained) = &site.binding.namespace_context {
        if source_namespace.to_compiled_context().as_ref() != Some(retained) {
            return false;
        }
    } else if !matches!(source_namespace.as_ref(), SourceNamespaceKey::Authored(_)) {
        return false;
    }
    if bindings.target_may_be_unknown(&site.binding.name, source_namespace.as_ref()) {
        return false;
    }
    let targets = bindings.targets(&site.binding.name, source_namespace.as_ref());
    let mut targets = targets.iter();
    let Some(target) = targets.next() else {
        return false;
    };
    if targets.next().is_some() {
        return false;
    }
    target.registry_backed
        && target.prepended.is_empty()
        && tcl_registry::command_descriptor_labels_match(&target.command, &site.binding.identity)
}

/// Apply command-table effects from evaluated `[...]` words before the outer
/// statement. The shared inventory is intentionally value-conservative: an
/// incomplete parse or computed command head widens the command domain.
fn apply_embedded_transitions(
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    source_order_mode: bool,
) -> bool {
    apply_embedded_transitions_in(
        stmt,
        registry,
        bindings,
        namespace,
        source_order_mode,
        (BindingInvocationMetadata::Standalone, None),
    )
}

pub(in super::super) fn apply_embedded_transitions_in(
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    source_order_mode: bool,
    selection: (
        BindingInvocationMetadata<'_>,
        Option<tcl_lexer::LexerConfig>,
    ),
) -> bool {
    let state = std::cell::RefCell::new(bindings);
    let resolve = |head: &str| state.borrow().resolved_embedded_head(head, namespace);
    let observe = |words: &[crate::ir_helpers::CommandWord], conditional: bool| {
        let skipped = conditional.then(|| (**state.borrow()).clone());
        apply_embedded_command_transition(
            words,
            registry,
            &mut state.borrow_mut(),
            namespace,
            source_order_mode,
            selection.0,
        );
        if let Some(skipped) = skipped {
            state.borrow_mut().join(&skipped);
        }
    };
    let embedded = match selection {
        (BindingInvocationMetadata::Standalone, _) => evaluated_command_substitutions_with_replay(
            stmt,
            registry,
            Some(&resolve),
            Some(&observe),
        ),
        (BindingInvocationMetadata::Supplied(metadata), Some(config)) => {
            crate::ir_helpers::evaluated_command_substitutions_with_replay_and_metadata_context(
                stmt,
                registry,
                Some(&resolve),
                Some(&observe),
                metadata,
                config,
            )
        }
        (BindingInvocationMetadata::Supplied(_), None) => {
            state.borrow_mut().mark_opaque_binding_mutation();
            return true;
        }
    };
    let observed = embedded.opaque || embedded.all_commands().next().is_some();
    if embedded.opaque {
        state.borrow_mut().mark_opaque_binding_mutation();
    }
    observed
}

fn apply_embedded_command_transition(
    words: &[crate::ir_helpers::CommandWord],
    registry: &CommandRegistry,
    bindings: &mut ModuleCommandBindings,
    namespace: &crate::ir_helpers::ExecutionNamespace,
    source_order_mode: bool,
    metadata: BindingInvocationMetadata<'_>,
) {
    let Some(head) = words.first() else {
        return;
    };
    let Some(head_name) = head.literal() else {
        bindings.mark_opaque_binding_mutation();
        return;
    };
    let Some(command_namespace) = namespace.for_head_context(head_name) else {
        bindings.mark_opaque_binding_mutation();
        return;
    };
    if bindings.names_only_a_declared_command(head_name, command_namespace.as_ref()) {
        // A document-declared command leaves the command world intact, as a
        // direct call to it does.
        return;
    }
    let source_may_be_unknown =
        bindings.target_may_be_unknown(head_name, command_namespace.as_ref());
    if source_order_mode
        && bindings
            .targets(head_name, command_namespace.as_ref())
            .iter()
            .any(|target| !target.registry_backed)
    {
        bindings.mark_source_order_user_procedure_call();
    }
    let facts = match metadata {
        BindingInvocationMetadata::Standalone => {
            bindings.resolve_command_words(words, registry, command_namespace.as_ref())
        }
        BindingInvocationMetadata::Supplied(metadata) => bindings
            .resolve_command_words_with_metadata_context(
                words,
                registry,
                metadata,
                command_namespace.as_ref(),
            ),
    };
    apply_resolved_may_transitions(
        facts,
        source_may_be_unknown,
        // A recovered invocation has no structured IR body. If it can
        // evaluate Tcl text, that text may change any command binding.
        true,
        bindings,
        namespace,
    );
}
