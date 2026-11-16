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

//! The parameters and literal names a body binds in its caller's frame through `upvar`.

use super::command_traits::{
    is_local_scalar_name, selected_alias_layout, written_template_literal,
};
use super::{Aliases, ScanCtx, TraitScanEnv};
use crate::segmenter::segment_commands_with_offset_and_config;
use std::collections::{HashMap, HashSet};
use tcl_registry::arg_role::ArgRole;

/// The parameters whose **value names a variable in the immediate caller's
/// frame** — `upvar 1 $param local` and its default-level spelling `upvar
/// $param local`, and nothing else.
///
/// This symbolic layout is conditional on entry to the retained declaration.
/// It names no caller cell and grants no read, store or successful activation.
/// Call-site navigation selects the actual callee and argument independently.
/// It is strictly narrower
/// than [`ProcArgTrait::VarWrite`] / [`ProcArgTrait::VarRead`], which record
/// only *that* a parameter's value is used as a variable name through an
/// `upvar`, never *which frame* the alias lands in.  Every other level names a
/// different frame, pinned identical on tclsh 9.0.4 and 8.6.14:
///
/// | callee body | `p x` from a proc writes … |
/// |---|---|
/// | `upvar 1 $n a; set a 1` | the caller's `x` |
/// | `upvar 0 $n a; set a 1` | the **callee's own** `x` — caller's `x` never exists |
/// | `upvar #0 $n a; set a 1` | the **global** `::x` |
/// | `upvar 2 $n a; set a 1` | the caller's **caller** — the immediate caller is skipped |
///
/// `namespace upvar ns $token local` is deliberately absent too: it aliases a
/// *namespace* variable, so a call site passing `token` creates nothing in the
/// calling frame either.
///
/// Only the top-level command scan is walked, matching the shallow trait pass
/// exactly — a consumer that requires both facts is then never widened by a
/// disagreement between them, only narrowed.
#[must_use]
pub fn caller_frame_upvar_params(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashSet<String> {
    caller_frame_upvar_params_over(params, body_source, env, &std::cell::OnceCell::new())
}

/// One top-level command of a body with its assistance.
type TopLevelView = (
    crate::segmenter::SegmentedCommand,
    Option<Box<crate::registry_invocation::InvocationBodyAssistance>>,
);

/// Every top-level command of `body_source` with its assistance. The
/// assistance reads the scan environment and never the parameters, so the
/// caller-frame scans of one body share it.
fn top_level_views(body_source: &str, ctx: &ScanCtx<'_, '_>) -> Vec<TopLevelView> {
    segment_commands_with_offset_and_config(body_source, 0, ctx.config.nested())
        .into_iter()
        .map(|seg| {
            let view = ctx.body_assistance(body_source, &seg);
            (seg, view)
        })
        .collect()
}

/// [`caller_frame_upvar_params`] and [`caller_frame_literal_targets`] of one
/// body, reading each top-level command's assistance once.
#[must_use]
pub fn caller_frame_facts(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> (HashSet<String>, HashMap<String, bool>) {
    let views = std::cell::OnceCell::new();
    (
        caller_frame_upvar_params_over(params, body_source, env, &views),
        caller_frame_literal_targets_over(body_source, env, &views),
    )
}

fn caller_frame_upvar_params_over(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
    views: &std::cell::OnceCell<Vec<TopLevelView>>,
) -> HashSet<String> {
    let mut out = HashSet::new();
    if params.is_empty()
        || body_source.trim().is_empty()
        || !env.matches_source(body_source)
        || !env.matches_parameters(params)
    {
        return out;
    }
    let param_set: HashSet<&str> = params.iter().copied().collect();
    let ctx = ScanCtx {
        param_set: &param_set,
        parameters: params,
        surface: env.surface,
        config: env.config,
        identities: env.identities,
        executed_source: env.executed_source,
        original_logical_body: env.original_logical_body,
        document: None,
        original_logical_lambda: None,
        selected_source_child_body: None,
        source_base: env
            .executed_source
            .map(crate::command_binding::ExecutedScriptSource::base)
            .or_else(|| {
                env.original_logical_body
                    .map(|body| body.scope().scope_body().content_span().start())
            }),
    };
    for (seg, view) in views.get_or_init(|| top_level_views(body_source, &ctx)) {
        let Some(view) = view.as_deref() else {
            continue;
        };
        let source = view.conditional_parameter_source_invocation(params);
        let Some(selected) = view.parameter_invocation(params).or_else(|| {
            source
                .as_ref()
                .map(super::super::super::registry_invocation::ConditionalParameterSourceInvocation::retained_invocation)
        }) else {
            continue;
        };
        let Some((level, first)) = selected_alias_layout(view, selected) else {
            continue;
        };
        if !level.is_caller_frame() {
            continue;
        }
        let values = ctx.argument_values(body_source, seg, &Aliases::default());
        for source in (first..selected.arguments.len()).step_by(2) {
            if let Some(param) = selected
                .effective
                .written_argument(source)
                .and_then(|original| values.argument(original))
            {
                out.insert(param.to_owned());
            }
        }
    }
    out
}

/// The **literal caller-frame targets** a proc body binds through `upvar` —
/// caller-frame names the callee spells in its *own* text, with no call-site
/// argument word to key on (`upvar 1 name name`).  Returns
/// `caller_name → written-through-alias`.
///
/// Strictly the complement of [`caller_frame_upvar_params`]: that fact keys
/// on a `$param` **source** (the name arrives from the call site); this one
/// keys on a *literal* source (the name is fixed by the callee).  tclsh
/// 9.0.4 / 8.6.14, identical: with `proc np {} {upvar name name; set name
/// W1}`, a caller running `np; puts $name` prints `W1` — `name` exists in
/// the caller's frame although the caller never assigns it and no call-site
/// word spells it.
///
/// This is an owned callee template, conditional on its entry and normal alias
/// operations. It identifies no actual caller cell and proves no completed
/// write. Navigation instantiates it only with a separate original
/// call/allocation/frame receipt; execution and value consumers use actual
/// source access and continuation proofs instead.
///
/// Exclusions, each the abstaining direction:
///
/// * a non-caller frame level (`upvar 0` / `#0` / `2` / `$lvl`) — a
///   different frame entirely, see [`caller_frame_upvar_params`]'s table;
/// * a `::`-qualified source (`upvar 1 ::ns::x local`) — a fixed global/
///   namespace cell, level-independent, already linked by the analyser's
///   alias consumer (`apply_state_transitions`) `otherVar` link;
/// * an array element or any substituted/computed source — not a plain
///   caller-frame scalar name this scan can claim;
/// * a dynamic **local** side — with no alias name, the write-through scan
///   below has nothing to match, so the pair contributes nothing here (the
///   compiler-side summary still widens the call site).
///
/// The write-through upgrade mirrors [`record_upvar_pairs`]'s rule for
/// params: a later registry-declared `VarWrite` on the alias local (`set
/// name …`, `lassign … name`) marks the caller name as *created* by a call;
/// an alias only ever read leaves it `false` (a referencing call site, not a
/// creating one).  Only the top-level command scan is walked, matching the
/// shallow trait pass exactly.
#[must_use]
pub fn caller_frame_literal_targets(
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashMap<String, bool> {
    caller_frame_literal_targets_over(body_source, env, &std::cell::OnceCell::new())
}

fn caller_frame_literal_targets_over(
    body_source: &str,
    env: TraitScanEnv<'_>,
    views: &std::cell::OnceCell<Vec<TopLevelView>>,
) -> HashMap<String, bool> {
    if !env.matches_source(body_source) {
        return HashMap::new();
    }
    let params = HashSet::new();
    let ctx = ScanCtx {
        param_set: &params,
        parameters: &[],
        surface: env.surface,
        config: env.config,
        identities: env.identities,
        executed_source: env.executed_source,
        original_logical_body: env.original_logical_body,
        document: None,
        original_logical_lambda: None,
        selected_source_child_body: None,
        source_base: env
            .executed_source
            .map(crate::command_binding::ExecutedScriptSource::base)
            .or_else(|| {
                env.original_logical_body
                    .map(|body| body.scope().scope_body().content_span().start())
            }),
    };
    let mut targets = HashMap::new();
    if body_source.trim().is_empty() || !env.matches_source(body_source) {
        return targets;
    }
    let mut alias_to_target = HashMap::new();
    for (_, view) in views.get_or_init(|| top_level_views(body_source, &ctx)) {
        let Some(view) = view.as_deref() else {
            continue;
        };
        let Some(selected) = view.caller_name_template_invocation() else {
            continue;
        };
        if let Some((level, first)) = selected_alias_layout(view, selected) {
            if !level.is_caller_frame() {
                continue;
            }
            for source in (first..selected.arguments.len()).step_by(2) {
                let (Some(name), Some(local)) = (
                    view.caller_template_literal_data(selected, source),
                    view.caller_template_literal_data(selected, source + 1),
                ) else {
                    continue;
                };
                if is_local_scalar_name(&name, selected) && is_local_scalar_name(&local, selected) {
                    targets.entry(name.clone()).or_insert(false);
                    alias_to_target.insert(local, name);
                }
            }
            continue;
        }
        for (index, role) in selected.written_argument_roles() {
            if role != ArgRole::VarWrite {
                continue;
            }
            if let Some(name) = written_template_literal(view, selected, index)
                && let Some(caller_name) = alias_to_target.get(&name)
            {
                targets.insert(caller_name.clone(), true);
            }
        }
    }
    targets
}
