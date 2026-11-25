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

//! Select direct caller-template events from original invocation assistance.

use super::command_traits::{
    is_local_scalar_name, selected_alias_layout, written_template_literal,
};
use super::{Aliases, ScanCtx, TraitScanEnv, infer_param_traits_with_caller_frame};
use crate::cfg_builder::frame_effects::DirectCallerEffects;
use crate::registry_invocation::{ConditionalParameterSourceInvocation, InvocationBodyAssistance};
use std::collections::{HashMap, HashSet};
use tcl_registry::arg_role::ArgRole;

/// Parameters whose value names a variable in the immediate caller's frame.
/// The retained declaration supplies a symbolic name, conditional on entry.
/// This selects an explicit level 1 or the implicit caller level. The caller
/// combines it with the independent parameter read or write trait.
#[must_use]
pub fn caller_frame_upvar_params(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashSet<String> {
    infer_param_traits_with_caller_frame(params, body_source, env).caller_frame_params
}

/// Literal caller-frame scalars and whether a direct command writes through
/// their alias. Both ends of the link require literal local-scalar geometry.
/// A write describes the source template and grants no completed store.
#[must_use]
pub fn caller_frame_literal_targets(
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashMap<String, bool> {
    infer_param_traits_with_caller_frame(&[], body_source, env).caller_frame_literals
}

/// Both caller-template projections from the shallow procedure analysis.
#[must_use]
pub fn caller_frame_facts(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> (HashSet<String>, HashMap<String, bool>) {
    let analysis = infer_param_traits_with_caller_frame(params, body_source, env);
    (analysis.caller_frame_params, analysis.caller_frame_literals)
}

pub(super) fn record_direct_caller_effects(
    effects: &mut DirectCallerEffects,
    view: &InvocationBodyAssistance,
    source: &str,
    command: &crate::segmenter::SegmentedCommand,
    ctx: &ScanCtx<'_, '_>,
) {
    record_parameter_sources(effects, view, source, command, ctx);
    record_literal_effects(effects, view);
}

fn record_parameter_sources(
    effects: &mut DirectCallerEffects,
    view: &InvocationBodyAssistance,
    source: &str,
    command: &crate::segmenter::SegmentedCommand,
    ctx: &ScanCtx<'_, '_>,
) {
    let params = ctx.parameters;
    if params.is_empty() {
        return;
    }
    let conditional = view.conditional_parameter_source_invocation(params);
    let Some(selected) = view.parameter_invocation(params).or_else(|| {
        conditional
            .as_ref()
            .map(ConditionalParameterSourceInvocation::retained_invocation)
    }) else {
        return;
    };
    let Some((level, first)) = selected_alias_layout(view, selected) else {
        return;
    };
    if !level.is_caller_frame() {
        return;
    }
    // Capture the declaration's own input, independent of the trait pass's
    // lexical value-copy aliases and any concrete entered values.
    let values = ctx.argument_values(source, command, &Aliases::default());
    for argument in (first..selected.arguments.len()).step_by(2) {
        if let Some(parameter) = selected
            .effective
            .written_argument(argument)
            .and_then(|original| values.argument(original))
        {
            effects.record_parameter(parameter);
        }
    }
}

fn record_literal_effects(effects: &mut DirectCallerEffects, view: &InvocationBodyAssistance) {
    let Some(selected) = view.caller_name_template_invocation() else {
        return;
    };
    if let Some((level, first)) = selected_alias_layout(view, selected) {
        if !level.is_caller_frame() {
            return;
        }
        for source in (first..selected.arguments.len()).step_by(2) {
            let (Some(name), Some(local)) = (
                view.caller_template_literal_data(selected, source),
                view.caller_template_literal_data(selected, source + 1),
            ) else {
                continue;
            };
            if is_local_scalar_name(&name, selected) && is_local_scalar_name(&local, selected) {
                effects.record_literal_alias(local, name);
            }
        }
        return;
    }
    for (index, role) in selected.written_argument_roles() {
        if role == ArgRole::VarWrite
            && let Some(local) = written_template_literal(view, selected, index)
        {
            effects.record_write(&local);
        }
    }
}
