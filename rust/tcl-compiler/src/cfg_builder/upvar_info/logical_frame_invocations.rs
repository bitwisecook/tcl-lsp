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

//! Frame effects of invocations recovered from the original logical source words.

use super::super::frame_effects::{FrameContext, FrameTarget};
use super::body_walk::walk_script;
use super::frame_commands::widen_beyond_caller;
use super::selected_frame_bodies::record_upframe_body_at_depth;
use super::{LogicalFrameScan, UpvarBindings, UpvarInfo};
use crate::command_binding::ResolvedFrameBodySelection;
use crate::ir::{Script, Statement};
use tcl_registry::CommandRegistry;
use tcl_registry::frame_effect::{FrameArgLayout, FrameLevel};

pub(super) fn record_logical_frame_invocation(
    invocation: &crate::registry_invocation::OriginalLogicalFrameInvocation,
    context: LogicalFrameScan<'_>,
    info: &mut UpvarInfo,
) {
    use tcl_registry::frame_effect::FrameArgumentResolution;
    let Some(layout) = invocation.layout() else {
        return;
    };
    let selection = match invocation.arguments() {
        Some(FrameArgumentResolution::Valid {
            level,
            level_word_len,
        }) => (level, invocation.offset() + level_word_len),
        Some(FrameArgumentResolution::Invalid) => return,
        Some(FrameArgumentResolution::Unknown) | None => {
            if layout != FrameArgLayout::ScriptInCurrentFrame {
                widen_beyond_caller(info, FrameTarget::Unknown);
            }
            return;
        }
    };
    match layout {
        FrameArgLayout::AliasPairs => {
            record_logical_alias_pairs(invocation, selection, context, info);
        }
        FrameArgLayout::ScriptInCurrentFrame | FrameArgLayout::ScriptInSelectedFrame => {
            record_logical_frame_script(invocation, layout, selection, context, info);
        }
        FrameArgLayout::OpaqueCallerVars => {}
    }
}

fn record_logical_alias_pairs(
    invocation: &crate::registry_invocation::OriginalLogicalFrameInvocation,
    selection: (FrameLevel, usize),
    context: LogicalFrameScan<'_>,
    info: &mut UpvarInfo,
) {
    let (level, start) = selection;
    match FrameContext::select(context.current, ResolvedFrameBodySelection::Selected(level)) {
        FrameContext::Own | FrameContext::Global => return,
        FrameContext::Beyond(target) => {
            widen_beyond_caller(info, target);
            return;
        }
        // An alias made inside a body already running in the caller's
        // frame binds two of the caller's names; which ones its later
        // writes reach is not projected here.
        FrameContext::DirectCaller if context.current != FrameContext::Own => {
            info.caller_frame_opaque_writes = true;
            return;
        }
        FrameContext::DirectCaller => {}
    }
    for source in (start..invocation.count()).step_by(2) {
        let Some(destination) = source
            .checked_add(1)
            .filter(|index| *index < invocation.count())
        else {
            info.has_unresolvable_caller_target = true;
            return;
        };
        let local = invocation.literal(destination);
        if let Some(target) = invocation.literal(source) {
            if let Some(local) = local {
                info.literal_targets.insert(local, target);
            } else {
                info.unnameable_local_aliases.insert(target.clone());
                info.uplevel_literal_writes.insert(target);
            }
        } else if let Some(parameter) = invocation.parameter(
            source,
            context.params,
            context.bindings.input.lexer_config(),
        ) {
            if tcl_syntax::formal_params::is_variadic_formal_name(&parameter) {
                if let Some(local) = local {
                    info.args_tail_upvar.push(local);
                } else {
                    info.has_unresolvable_caller_target = true;
                }
            } else if let Some(local) = local {
                info.param_targets.insert(local, parameter);
            } else {
                info.uplevel_param_writes.insert(parameter);
            }
        } else {
            info.has_unresolvable_caller_target = true;
        }
    }
}

fn record_logical_frame_script(
    invocation: &crate::registry_invocation::OriginalLogicalFrameInvocation,
    layout: FrameArgLayout,
    selection: (FrameLevel, usize),
    context: LogicalFrameScan<'_>,
    info: &mut UpvarInfo,
) {
    let selected = if layout == FrameArgLayout::ScriptInCurrentFrame {
        context.current
    } else {
        FrameContext::select(
            context.current,
            ResolvedFrameBodySelection::Selected(selection.0),
        )
    };
    if selected == FrameContext::Global {
        return;
    }
    if let FrameContext::Beyond(target) = selected {
        widen_beyond_caller(info, target);
        return;
    }
    let start = if layout == FrameArgLayout::ScriptInCurrentFrame {
        invocation.offset()
    } else {
        selection.1
    };
    let body = (start.checked_add(1) == Some(invocation.count()))
        .then(|| {
            invocation.source_body(
                context.bindings.module,
                context.registry,
                context.tokens,
                start,
            )
        })
        .flatten();
    let Some(body) = body else {
        if selected == FrameContext::DirectCaller {
            info.caller_frame_opaque_writes = true;
            info.caller_frame_opaque_reads = true;
        }
        return;
    };
    if crate::optimiser::MAX_OPTIMISER_WALK_DEPTH.exceeded(context.depth) {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
        return;
    }
    if selected == FrameContext::Own {
        walk_script(
            &body,
            context.params,
            context.registry,
            Some(context.bindings),
            context.namespace,
            info,
        );
    } else {
        record_upframe_body_at_depth(
            &body,
            true,
            context.registry,
            Some(context.bindings),
            context.namespace,
            info,
            context.depth + 1,
        );
    }
}

pub(super) fn record_logical_frame_statement(
    script: &Script,
    stmt: &Statement,
    registry: &CommandRegistry,
    bindings: Option<UpvarBindings<'_>>,
    namespace: &crate::command_binding::SourceNamespaceKey,
    info: &mut UpvarInfo,
    depth: u32,
) -> bool {
    let Some(bindings) =
        bindings.filter(|bindings| bindings.input.has_logical_source_name_context())
    else {
        return false;
    };
    let selected = script.retained_source_tokens_for_statement(stmt)
        .and_then(|tokens| {
            let metadata = tokens.source_binding.as_ref()?
                .original_invocation_metadata_for_module(tokens, bindings.module, registry)?;
            let invocation = crate::registry_invocation::original_logical_frame_invocation_with_metadata_context(
                registry, metadata, tokens,
            )?;
            Some((tokens, invocation))
        });
    if let Some((tokens, invocation)) = selected {
        record_logical_frame_invocation(
            &invocation,
            LogicalFrameScan {
                tokens,
                current: FrameContext::DirectCaller,
                params: &[],
                registry,
                bindings,
                namespace,
                depth: depth + 1,
            },
            info,
        );
    } else {
        info.caller_frame_opaque_writes = true;
        info.caller_frame_opaque_reads = true;
    }
    true
}
