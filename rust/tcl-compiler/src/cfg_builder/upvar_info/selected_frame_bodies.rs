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

//! Bodies that run in a selected frame: their writes, the aliases they make and nested `uplevel` selections, composed into the caller-effect summary.

use super::super::frame_effects::{FrameContext, FrameTarget};
use super::body_walk::walk_script;
use super::frame_commands::{upframe_body_is_braced, widen_beyond_caller};
use super::logical_frame_invocations::record_logical_frame_statement;
use super::{UpvarBindings, UpvarInfo};
use crate::command_binding::{ResolvedBindingInvocation, ResolvedFrameBody};
use crate::ir::{CommandTokens, ExecutionNamespace, Script, Statement};
use std::collections::BTreeSet;
use tcl_registry::CommandRegistry;
use tcl_registry::frame_effect::FrameLevel;

/// Consume the shared binding/registry projection of an evaluated body.  The
/// command-binding layer alone owns alias-prefix insertion and body-word
/// positioning; this module only composes the selected variable frame into a
/// caller-effect summary.
#[allow(clippy::too_many_arguments)]
pub(super) fn record_resolved_frame_body(
    invocation: &ResolvedBindingInvocation,
    current: FrameContext,
    params: &[String],
    registry: &CommandRegistry,
    bindings: UpvarBindings<'_>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    execution_namespace: &ExecutionNamespace,
    info: &mut UpvarInfo,
    depth: u32,
) {
    let body = invocation.resolved_frame_body_with_metadata_context(
        registry,
        bindings.source,
        execution_namespace,
        Some(bindings.metadata),
    );
    match body {
        ResolvedFrameBody::Readable { source, selection } => {
            let selected = FrameContext::select(current, selection);
            record_readable_frame_body(
                &source, selected, params, registry, bindings, namespace, info, depth,
            );
        }
        ResolvedFrameBody::Opaque { selection } => {
            match FrameContext::select(current, selection) {
                // An arbitrary script evaluated directly in the caller can
                // read, write, or unset any caller cell.
                FrameContext::DirectCaller => {
                    info.caller_frame_opaque_writes = true;
                    info.caller_frame_opaque_reads = true;
                }
                // The selected frame is further out: opaque there, and
                // carried out to it by interprocedural closure.
                FrameContext::Beyond(target) => widen_beyond_caller(info, target),
                // Current-frame opacity is owned by dynamic_names; #0 is
                // owned by global_write_info.
                FrameContext::Own | FrameContext::Global => {}
            }
        }
        ResolvedFrameBody::NotApplicable | ResolvedFrameBody::KnownError => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn record_readable_frame_body(
    source: &str,
    selected: FrameContext,
    params: &[String],
    registry: &CommandRegistry,
    bindings: UpvarBindings<'_>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
    depth: u32,
) {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return;
    }
    match selected {
        FrameContext::Global => {}
        // A readable body further out is still summarised as opaque at its
        // frame: its own nested frame selections are not composed here.
        FrameContext::Beyond(target) => widen_beyond_caller(info, target),
        FrameContext::Own | FrameContext::DirectCaller => {
            let config = bindings.input.lexer_config();
            let mut lowerer = crate::lowering::Lowerer::with_config(registry, config)
                .with_dialect(Some(bindings.input.unit_profile()))
                .with_context_registry(bindings.input.context_registry());
            let module = lowerer.lower(source).clone();
            if selected == FrameContext::Own {
                walk_script(
                    &module.top_level,
                    params,
                    registry,
                    Some(bindings),
                    namespace,
                    info,
                );
            } else {
                record_upframe_body_at_depth(
                    &module.top_level,
                    false,
                    registry,
                    Some(bindings),
                    namespace,
                    info,
                    depth + 1,
                );
            }
        }
    }
}

/// Record a body that is already known to execute in the direct caller frame.
/// Nested `uplevel 0` preserves that frame; nested relative levels beyond zero
/// reach past it and must not disappear from the summary.
pub(super) fn record_upframe_body_at_depth(
    body: &Script,
    source_body_is_braced: bool,
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
    depth: u32,
) {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return;
    }
    let projection = bindings.map_or_else(
        || tcl_registry::VariableWriteProjection {
            literal_names: crate::ir_helpers::defs_from_ir_script(body),
            read_before_write_names: Vec::new(),
            opaque_variable_frame: super::super::global_write_info::script_has_dynamic_write_target(
                body,
            ),
        },
        |bindings| {
            super::super::global_write_info::script_value_write_projection_with_metadata_context(
                body,
                registry,
                bindings.source,
                namespace,
                Some(bindings.metadata),
                bindings.input.lexer_config(),
            )
        },
    );
    let requires_runtime_resolution =
        crate::ir_helpers::requires_runtime_command_namespace(body, registry);
    let projected_names: BTreeSet<String> = if source_body_is_braced || !requires_runtime_resolution
    {
        projection.literal_names.into_iter().collect()
    } else {
        BTreeSet::new()
    };
    if let Some(bindings) = bindings {
        compose_selected_frame_alias_writes(
            body,
            &projected_names,
            projection.opaque_variable_frame,
            registry,
            bindings,
            info,
            depth + 1,
        );
    }
    info.uplevel_literal_writes.extend(projected_names);
    info.caller_frame_opaque_writes |= projection.opaque_variable_frame;

    // A relative `uplevel` evaluates in the *target frame's* namespace. The
    // procedure's defining namespace is therefore not a sound substitute for
    // resolving an unqualified head: a caller-local alias can turn it into an
    // arbitrary variable write. Keep literal names as useful hints, but widen
    // unless every directly evaluated command spelling is absolute.
    if requires_runtime_resolution {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
    }
    record_nested_upframe_effects(body, registry, bindings, namespace, info, depth + 1);
}

/// Compose variable-cell aliases established inside a body which already runs
/// in the direct caller frame.  The write projector deliberately reports the
/// local spelling (`upvar 0 x local; set local 1` reports `local`); registry
/// transitions supply the cell identity needed to add `x`.  We retain the raw
/// local as a conservative may-write because the summary is path-insensitive
/// and the write may precede the alias on another path.
#[allow(clippy::too_many_arguments)]
fn compose_selected_frame_alias_writes(
    script: &Script,
    projected_names: &BTreeSet<String>,
    opaque_write: bool,
    registry: &CommandRegistry,
    bindings: UpvarBindings<'_>,
    info: &mut UpvarInfo,
    depth: u32,
) {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        info.caller_frame_opaque_writes = true;
        return;
    }
    let any_write = opaque_write || !projected_names.is_empty();
    for stmt in &script.statements {
        if !stmt.is_executable_invocation() {
            continue;
        }
        compose_selected_statement_aliases(
            stmt,
            script.retained_source_tokens_for_statement(stmt),
            projected_names,
            any_write,
            registry,
            bindings,
            info,
        );

        let embedded = crate::ir_helpers::evaluated_command_substitutions(stmt, registry);
        for words in embedded.commands {
            let Some(head) = words
                .first()
                .and_then(crate::ir_helpers::CommandWord::literal)
            else {
                continue;
            };
            let execution_namespace = ExecutionNamespace::RuntimeSelected;
            let Some(command_namespace) = execution_namespace.for_head_context(head) else {
                continue;
            };
            for facts in
                bindings.resolve_command_words(&words, registry, command_namespace.as_ref())
            {
                compose_selected_alias_facts(
                    &facts,
                    projected_names,
                    any_write,
                    registry,
                    bindings.invocation_dialect(),
                    info,
                );
            }
        }

        // A nested frame selector installs aliases in a different frame. All
        // other structural bodies preserve the caller frame and participate
        // in this conservative, path-insensitive union.
        if matches!(stmt, Statement::UpFrame { .. }) {
            continue;
        }
        for nested in crate::ir_helpers::nested_bodies(stmt) {
            compose_selected_frame_alias_writes(
                nested,
                projected_names,
                opaque_write,
                registry,
                bindings,
                info,
                depth + 1,
            );
        }
    }
}

fn compose_selected_statement_aliases(
    stmt: &Statement,
    tokens: Option<&CommandTokens>,
    projected_names: &BTreeSet<String>,
    any_write: bool,
    registry: &CommandRegistry,
    bindings: UpvarBindings<'_>,
    info: &mut UpvarInfo,
) {
    if bindings.input.has_logical_source_name_context() {
        if let Some((_, aliases)) = tokens.and_then(|tokens| {
            let metadata = tokens
                .source_binding
                .as_ref()?
                .original_invocation_metadata_for_module(tokens, bindings.module, registry)?;
            crate::registry_invocation::original_logical_variable_footprint_with_metadata_context(
                registry, metadata, tokens,
            )
        }) {
            for alias in &aliases {
                compose_selected_alias(
                    alias,
                    projected_names,
                    any_write,
                    registry,
                    bindings.invocation_dialect(),
                    info,
                );
            }
        }
        return;
    }
    if let Statement::Call { command, .. } | Statement::Barrier { command, .. } = stmt {
        let execution_namespace = ExecutionNamespace::RuntimeSelected;
        if let Some(command_namespace) =
            execution_namespace.for_invocation_context(command, stmt.tokens())
        {
            let normal = stmt.tokens().and_then(|tokens| {
                crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                    registry,
                    Some(bindings.metadata),
                    tokens,
                )
            });
            if let Some(normal) = normal {
                // A compiler-hook choice is independent of a normal alias
                // declaration. Consume only the active alias projection.
                for alias in normal.variable_alias_transitions() {
                    compose_selected_alias(
                        alias,
                        projected_names,
                        any_write,
                        registry,
                        bindings.invocation_dialect(),
                        info,
                    );
                }
            }
            if let Some(possible) = stmt.tokens().and_then(|tokens| {
                crate::registry_invocation::possible_variable_alias_transitions_with_metadata_context(
                    registry,
                    Some(bindings.metadata),
                    tokens,
                )
            }) {
                // This summary records may-writes across possible callers.
                // Unknown physical frame identity cannot erase a selected
                // native alias candidate's outward exposure footprint.
                for alias in possible.aliases() {
                    compose_selected_alias(
                        alias,
                        projected_names,
                        any_write,
                        registry,
                        bindings.invocation_dialect(),
                        info,
                    );
                }
            }
            for invocation in bindings.resolve_statement(stmt, registry, command_namespace.as_ref())
            {
                compose_selected_alias_facts(
                    &invocation.facts,
                    projected_names,
                    any_write,
                    registry,
                    bindings.invocation_dialect(),
                    info,
                );
            }
        }
    }
}

/// Where an alias made in the caller's frame points, relative to that frame.
enum SelectedAliasTarget {
    SameExact(String),
    SameOpaque,
    /// This many levels out of the caller's frame (at least one).
    Past(u32),
    Unplaced,
    Other,
}

fn selected_alias_target(
    target: &tcl_registry::VariableAliasTarget,
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
) -> SelectedAliasTarget {
    use crate::cfg_builder::frame_effects::{CallerAliasSelection, caller_alias_selection};
    let selected = caller_alias_selection(target, |_, level| {
        dialect.map_or_else(
            || FrameLevel::parse_in(level, registry),
            |dialect| FrameLevel::parse_for_dialect(level, dialect),
        )
    });
    match selected {
        CallerAliasSelection::Selected(FrameLevel::Relative(0), variable) => {
            variable.map_or(SelectedAliasTarget::SameOpaque, |name| {
                SelectedAliasTarget::SameExact(
                    crate::naming::split_element_ref(name)
                        .map_or(name, |(root, _)| root)
                        .to_owned(),
                )
            })
        }
        CallerAliasSelection::Selected(FrameLevel::Relative(levels), _) => {
            SelectedAliasTarget::Past(levels)
        }
        CallerAliasSelection::Dynamic
        | CallerAliasSelection::Selected(FrameLevel::Dynamic | FrameLevel::Absolute(1..), _) => {
            SelectedAliasTarget::Unplaced
        }
        CallerAliasSelection::Other
        | CallerAliasSelection::Invalid(_)
        | CallerAliasSelection::Selected(FrameLevel::Absolute(0), _) => SelectedAliasTarget::Other,
    }
}

fn compose_selected_alias_facts(
    facts: &tcl_registry::InvocationFacts,
    projected_names: &BTreeSet<String>,
    any_write: bool,
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
    info: &mut UpvarInfo,
) {
    let Some(transitions) = facts.state_transitions.declared() else {
        return;
    };
    for fact in transitions.facts() {
        let tcl_registry::StateTransition::VariableCellAlias(alias) = &fact.transition else {
            continue;
        };
        compose_selected_alias(alias, projected_names, any_write, registry, dialect, info);
    }
}

fn compose_selected_alias(
    alias: &tcl_registry::state_transition::VariableCellAliasTransition,
    projected_names: &BTreeSet<String>,
    any_write: bool,
    registry: &CommandRegistry,
    dialect: Option<tcl_registry::InvocationDialect>,
    info: &mut UpvarInfo,
) {
    let local_written = alias.writes_value
        || alias.local.literal().map_or(any_write, |local| {
            projected_names
                .contains(crate::naming::split_element_ref(local).map_or(local, |(root, _)| root))
        });
    if !local_written {
        return;
    }
    match selected_alias_target(&alias.target, registry, dialect) {
        SelectedAliasTarget::SameExact(target) => {
            info.uplevel_literal_writes.insert(target);
        }
        SelectedAliasTarget::SameOpaque => info.caller_frame_opaque_writes = true,
        SelectedAliasTarget::Past(levels) => widen_beyond_caller(
            info,
            FrameTarget::past_the_caller(levels.saturating_add(1)).unwrap_or(FrameTarget::Unknown),
        ),
        SelectedAliasTarget::Unplaced => widen_beyond_caller(info, FrameTarget::Unknown),
        SelectedAliasTarget::Other => {}
    }
}

/// Compose nested literal `uplevel` frame selections from a body already
/// running in the direct caller frame. The ordinary value-write projection
/// intentionally skips [`Statement::UpFrame`] children because their writes
/// are not in the enclosing script's frame; this companion supplies the
/// missing frame transform.
fn record_nested_upframe_effects(
    script: &Script,
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
    depth: u32,
) {
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return;
    }
    for stmt in &script.statements {
        if !stmt.is_executable_invocation() {
            continue;
        }
        if let Statement::Call { command, .. } | Statement::Barrier { command, .. } = stmt {
            if record_logical_frame_statement(
                script, stmt, registry, bindings, namespace, info, depth,
            ) {
                continue;
            }
            let execution_namespace = ExecutionNamespace::RuntimeSelected;
            if let Some(bindings) = bindings
                && let Some(command_namespace) =
                    execution_namespace.for_invocation_context(command, stmt.tokens())
            {
                // A call which lowering saw before its alias was installed may
                // terminally be `eval` or `uplevel`.  Resolve the effective
                // body through the shared helper, including every baked alias
                // prefix, then compose its selection from the caller frame in
                // which this enclosing body already executes.
                for invocation in
                    bindings.resolve_statement(stmt, registry, command_namespace.as_ref())
                {
                    record_resolved_frame_body(
                        &invocation,
                        FrameContext::DirectCaller,
                        &[],
                        registry,
                        bindings,
                        namespace,
                        &execution_namespace,
                        info,
                        depth + 1,
                    );
                }
            }
        }
        match stmt {
            Statement::UpFrame {
                absolute: false,
                frame_shift: 0,
                body,
                tokens,
                ..
            } => record_upframe_body_at_depth(
                body,
                upframe_body_is_braced(tokens.as_ref()),
                registry,
                bindings,
                namespace,
                info,
                depth + 1,
            ),
            Statement::UpFrame {
                absolute: true,
                frame_shift: 0,
                ..
            } => {
                // `#0` is global; global_write_info owns it, including its
                // nested level-zero bodies.
            }
            // Run from the caller's frame: `k` relative levels out of it.
            Statement::UpFrame {
                absolute,
                frame_shift,
                ..
            } => widen_beyond_caller(
                info,
                match u32::try_from(*frame_shift) {
                    Ok(shift) if !*absolute => {
                        FrameTarget::past_the_caller(shift.saturating_add(1))
                            .unwrap_or(FrameTarget::Unknown)
                    }
                    _ => FrameTarget::Unknown,
                },
            ),
            _ => {
                for body in crate::ir_helpers::nested_bodies(stmt) {
                    record_nested_upframe_effects(
                        body,
                        registry,
                        bindings,
                        namespace,
                        info,
                        depth + 1,
                    );
                }
            }
        }
    }
}
