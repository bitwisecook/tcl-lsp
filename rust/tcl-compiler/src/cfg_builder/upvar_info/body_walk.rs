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

//! The walk over a procedure body that records each statement's frame effects.

use super::super::frame_effects::FrameContext;
use super::frame_commands::{
    record_frame_effect, record_plain_call, record_resolved_alias_pairs, record_upframe_body,
    upframe_body_is_braced, widen_level_beyond_caller,
};
use super::logical_frame_invocations::record_logical_frame_invocation;
use super::selected_frame_bodies::record_resolved_frame_body;
use super::{FrameCommandContext, LogicalFrameScan, UpvarBindings, UpvarInfo};
use crate::ir::{CommandTokens, ExecutionNamespace, Script, Statement};
use std::collections::BTreeSet;
use tcl_registry::CommandRegistry;
use tcl_registry::frame_effect::{FrameArgLayout, FrameLevel};

pub(super) fn collect_upvar_targets_inner(
    body: &Script,
    params: &[String],
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &tcl_names::namespace::SourceNamespaceKey,
) -> UpvarInfo {
    let mut info = UpvarInfo::default();
    walk_script(body, params, registry, bindings, namespace, &mut info);
    info
}

pub(super) fn walk_script(
    script: &Script,
    params: &[String],
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &tcl_names::namespace::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    for stmt in &script.statements {
        walk_stmt(
            stmt,
            script.retained_source_tokens_for_statement(stmt),
            params,
            registry,
            bindings,
            namespace,
            info,
        );
        if let Some(bindings) = bindings
            && bindings.input.has_logical_source_name_context()
            && let Some(tokens) = script.retained_source_tokens_for_statement(stmt)
        {
            record_original_child_frame_effects(
                tokens, params, registry, bindings, namespace, info,
            );
        }
    }
}

fn record_original_child_frame_effects(
    tokens: &CommandTokens,
    params: &[String],
    registry: &CommandRegistry,
    bindings: UpvarBindings<'_>,
    namespace: &tcl_names::namespace::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    let mut pending = vec![(tokens.clone(), 0_u32)];
    let mut seen = BTreeSet::new();
    while let Some((parent, depth)) = pending.pop() {
        if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(depth) {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
            continue;
        }
        let Some(site) = parent
            .source_binding
            .as_ref()
            .and_then(|binding| binding.invocation_site())
        else {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
            continue;
        };
        if !seen.insert(site.offset) {
            continue;
        }
        let selected = crate::registry_invocation::original_logical_frame_invocation_for_module(
            bindings.module,
            registry,
            &parent,
        );
        let Some(selected) = selected else {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
            continue;
        };
        if depth != 0 {
            record_logical_frame_invocation(
                &selected,
                LogicalFrameScan {
                    tokens: &parent,
                    current: FrameContext::Own,
                    params,
                    registry,
                    bindings,
                    namespace,
                    depth,
                },
                info,
            );
        }
        if selected.layout().is_none() {
            if let Some(children) = crate::registry_invocation::source_structure::original_immediate_body_commands_for_module(
                bindings.module, registry, &parent,
            ) {
                pending.extend(children.into_iter().map(|child| (child, depth + 1)));
            } else {
                info.caller_frame_opaque_writes = true;
                info.caller_frame_opaque_reads = true;
            }
        }
        if let Some(children) =
            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                &parent,
                bindings.input.lexer_config(),
                registry,
                bindings.metadata,
            )
        {
            for child in children {
                if let Some(child) = child.tokens {
                    pending.push((child, depth + 1));
                } else {
                    info.caller_frame_opaque_writes = true;
                    info.caller_frame_opaque_reads = true;
                }
            }
        } else {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
        }
    }
}

#[allow(clippy::too_many_lines)] // One exhaustive IR walk keeps frame-context transitions coupled.
fn walk_stmt(
    stmt: &Statement,
    tokens: Option<&CommandTokens>,
    params: &[String],
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &tcl_names::namespace::SourceNamespaceKey,
    info: &mut UpvarInfo,
) {
    if let Some(bindings) = bindings
        && bindings.input.has_logical_source_name_context()
        && matches!(stmt, Statement::Call { .. } | Statement::Barrier { .. })
    {
        let selected = tokens.and_then(|tokens| {
            let metadata = tokens
                .source_binding
                .as_ref()?
                .original_invocation_metadata_for_module(tokens, bindings.module, registry)?;
            crate::registry_invocation::original_logical_frame_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
        });
        if let (Some(tokens), Some(selected)) = (tokens, selected) {
            record_logical_frame_invocation(
                &selected,
                LogicalFrameScan {
                    tokens,
                    current: FrameContext::Own,
                    params,
                    registry,
                    bindings,
                    namespace,
                    depth: 0,
                },
                info,
            );
        } else {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
        }
        return;
    }
    if let (Some(bindings), Some(tokens)) = (bindings, tokens)
        && bindings.input.has_logical_source_name_context()
        && matches!(
            stmt,
            Statement::UpFrame {
                absolute: false,
                frame_shift: 1,
                ..
            }
        )
    {
        let candidates = tokens.source_binding.as_ref().and_then(|binding| {
            let metadata = binding.original_invocation_metadata_for_module(
                tokens,
                bindings.module,
                registry,
            )?;
            crate::registry_invocation::original_logical_frame_invocation_with_metadata_context(
                registry, metadata, tokens,
            )?
            .unknown_caller_variable_candidates(bindings.module, registry, tokens)
        });
        if let Some(candidates) = candidates {
            info.uplevel_literal_writes.extend(candidates.bindings);
        }
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return;
    }
    match stmt {
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
            if let Some(bindings) = bindings {
                // Ordinary-call composition needs the effective retained user
                // procedure, not the source spelling. An alias, namespace
                // fallback, or user-defined unknown handler can all select a
                // different terminal procedure while adding argv of their own.
                bindings.for_each_resolved_invocation(stmt, namespace, |target, _| {
                    if !target.registry_backed {
                        record_plain_call(&target.command, info);
                    }
                });
                // The source spelling may be an alias of `upvar`/`uplevel`.
                // Resolve its terminal registry descriptors and their
                // alias-prepended argv before interpreting the frame grammar;
                // consulting the source head would drop a leading frame level
                // or miss the effect entirely.
                for invocation in bindings.resolve_statement(stmt, registry, namespace) {
                    let Some(spec) = invocation.facts.frame_effect else {
                        continue;
                    };
                    match spec.layout {
                        FrameArgLayout::AliasPairs => {
                            record_resolved_alias_pairs(
                                &invocation,
                                spec,
                                params,
                                registry,
                                bindings.invocation_dialect(),
                                info,
                            );
                        }
                        FrameArgLayout::OpaqueCallerVars => {}
                        FrameArgLayout::ScriptInCurrentFrame
                        | FrameArgLayout::ScriptInSelectedFrame => {
                            record_resolved_frame_body(
                                &invocation,
                                FrameContext::Own,
                                params,
                                registry,
                                bindings,
                                namespace,
                                &ExecutionNamespace::SourceContext(namespace.clone()),
                                info,
                                0,
                            );
                        }
                    }
                }
                // A closed may-binding still has an unknown alternative when
                // it cannot enumerate the invoked implementation. That
                // implementation may select any registry-declared frame
                // effect (or arbitrary caller-frame Tcl), so preserve the
                // known alternatives above and widen for the unknown one.
                if bindings.target_resolution_may_be_unknown(command, namespace) {
                    info.caller_frame_opaque_writes = true;
                    info.caller_frame_opaque_reads = true;
                }
                return;
            }
            if let Some(spec) = registry.frame_effect(command) {
                record_frame_effect(
                    spec,
                    args,
                    params,
                    registry,
                    FrameCommandContext {
                        bindings: None,
                        tokens,
                        dialect: stmt
                            .tokens()
                            .and_then(|tokens| tokens.source_binding.as_ref())
                            .and_then(|binding| binding.variable_context.invocation_dialect),
                    },
                    namespace,
                    info,
                );
            } else {
                record_plain_call(command, info);
            }
        }
        // `uplevel <level> {literal body}` — the lowering already proved the
        // body readable and inlined it here. Its writes land in the frame
        // `frame_shift`/`absolute` select, so only the caller-frame form
        // contributes directly; `#0` / `0` miss the caller entirely, and a
        // deeper level lands past it (recorded so the one-hop composition in
        // `detect_upvar_procs` can carry it out to the frame it really
        // reaches).
        Statement::UpFrame {
            frame_shift,
            absolute,
            body,
            tokens,
            ..
        } => match (*absolute, *frame_shift) {
            (false, 1) => {
                record_upframe_body(
                    body,
                    upframe_body_is_braced(tokens.as_ref()),
                    registry,
                    bindings,
                    namespace,
                    info,
                );
            }
            // `uplevel 0` preserves the callee frame. Its direct writes stay
            // local, but a nested `uplevel 1` still reaches this procedure's
            // caller and must participate in the summary.
            (false, 0) => {
                walk_script(body, params, registry, bindings, namespace, info);
            }
            // `uplevel #0` selects the global frame, which is owned by the
            // global-write summary rather than this caller-frame projection.
            (true, 0) => {}
            (absolute, shift) => widen_level_beyond_caller(
                info,
                match u32::try_from(shift) {
                    Ok(shift) if absolute => FrameLevel::Absolute(shift),
                    Ok(shift) => FrameLevel::Relative(shift),
                    Err(_) => FrameLevel::Dynamic,
                },
            ),
        },
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                walk_script(&c.body, params, registry, bindings, namespace, info);
            }
            if let Some(e) = else_body {
                walk_script(e, params, registry, bindings, namespace, info);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            walk_script(init, params, registry, bindings, namespace, info);
            walk_script(next, params, registry, bindings, namespace, info);
            walk_script(body, params, registry, bindings, namespace, info);
        }
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. } => {
            walk_script(body, params, registry, bindings, namespace, info);
        }
        Statement::Block {
            body,
            namespace: body_namespace,
            ..
        } => {
            if let Some(context) = body.namespace_context.as_deref() {
                walk_script(body, params, registry, bindings, context, info);
            } else if bindings.is_none() {
                walk_script(
                    body,
                    params,
                    registry,
                    bindings,
                    &tcl_names::namespace::SourceNamespaceKey::authored(body_namespace),
                    info,
                );
            } else {
                info.caller_frame_opaque_writes = true;
                info.caller_frame_opaque_reads = true;
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for arm in arms {
                if let Some(b) = &arm.body {
                    walk_script(b, params, registry, bindings, namespace, info);
                }
            }
            if let Some(b) = default_body {
                walk_script(b, params, registry, bindings, namespace, info);
            }
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            walk_script(body, params, registry, bindings, namespace, info);
            for h in handlers {
                walk_script(&h.body, params, registry, bindings, namespace, info);
            }
            if let Some(f) = finally_body {
                walk_script(f, params, registry, bindings, namespace, info);
            }
        }
        _ => {}
    }
}
