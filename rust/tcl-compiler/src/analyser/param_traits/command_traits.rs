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

//! The traits one command gives the parameters it uses: evaluation and argument roles, selected aliases and definite writes.

use super::super::types::ProcArgTrait;
use super::{Aliases, ArgumentValues, ScanCtx};
use std::collections::{HashMap, HashSet};
use tcl_registry::arg_role::ArgRole;

/// Local scalar geometry under this invocation's retained lookup policy.
/// Unknown policy requires agreement across source policies, not a default.
pub(super) fn is_local_scalar_name(
    name: &str,
    selected: &crate::registry_invocation::ResolvedStatementInvocation,
) -> bool {
    tcl_syntax::naming::split_element_ref(name).is_none()
        && selected
            .dialect
            .and_then(|dialect| dialect.variable_lookup_policy)
            .map_or_else(
                || {
                    tcl_syntax::naming::source_variable_name_is_frame_local_for_all_policies(
                        name.as_bytes(),
                    )
                },
                |policy| {
                    tcl_syntax::naming::source_variable_name_is_frame_local(name.as_bytes(), policy)
                },
            )
}

pub(super) fn written_template_literal(
    view: &crate::registry_invocation::InvocationBodyAssistance,
    selected: &crate::registry_invocation::ResolvedStatementInvocation,
    written: usize,
) -> Option<String> {
    let mut matching = (0..selected.arguments.len())
        .filter(|index| selected.effective.written_argument(*index) == Some(written));
    let argument = matching.next()?;
    matching.next().is_none().then_some(())?;
    view.caller_template_literal_data(selected, argument)
}

/// Trait navigation is a May projection; alias and value-copy state require
/// the separate accepted, definite invocation and original written operands.
pub(super) fn scan_command<'p>(
    view: Option<&crate::registry_invocation::InvocationBodyAssistance>,
    cmd_args: &[String],
    braced: &[bool],
    values: &ArgumentValues<'p>,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    if let Some(p) = values.full.first().copied().flatten()
        && let Some(set) = traits.get_mut(p)
    {
        set.insert(ProcArgTrait::Command);
    }
    let Some(view) = view else {
        aliases.clear();
        return;
    };
    if let Some(shape) = view.conditional_logical_lambda_shape() {
        apply_arg_role_traits(
            &shape.written_argument_roles(),
            cmd_args,
            braced,
            values,
            traits,
        );
        apply_possible_evaluation_traits(
            std::slice::from_ref(shape),
            cmd_args,
            braced,
            values,
            traits,
        );
        aliases.clear();
        return;
    }
    if let Some(shape) = view.conditional_parameter_source_shape(ctx.parameters)
        && let Some(selected) = view.conditional_parameter_source_invocation(ctx.parameters)
    {
        // Conditional symbolic source usage is a separate May report. Actual
        // observer/lookup debt remains on its genuine prefix; no closed,
        // definite, successful alias or runtime write follows from this path.
        apply_arg_role_traits(
            &selected.written_argument_roles(),
            cmd_args,
            braced,
            values,
            traits,
        );
        apply_possible_evaluation_traits(
            std::slice::from_ref(shape),
            cmd_args,
            braced,
            values,
            traits,
        );
        let original = selected.retained_invocation();
        record_selected_aliases(view, original, values, traits, aliases);
        apply_definite_writes(view, original, values, traits, aliases);
        if shape.may_execute_parameter_body() {
            aliases.clear();
        }
        return;
    }
    if !view.parameter_role_advice().is_closed() {
        aliases.clear();
        apply_arg_role_traits(&view.declared_roles, cmd_args, braced, values, traits);
        return;
    }
    // An unresolved replacement may have no parameter-evaluation grammar.
    // Preserve declaration navigation independently of that execution residual.
    let advice = view.parameter_role_advice();
    apply_arg_role_traits(&advice.written_roles(), cmd_args, braced, values, traits);
    apply_arg_role_traits(&view.declared_roles, cmd_args, braced, values, traits);
    apply_possible_evaluation_traits(advice.candidates(), cmd_args, braced, values, traits);
    if let Some(selected) = view.parameter_invocation(ctx.parameters) {
        record_selected_aliases(view, selected, values, traits, aliases);
        apply_definite_writes(view, selected, values, traits, aliases);
        if advice.may_execute_body() {
            aliases.clear();
        }
    } else {
        aliases.clear();
    }
}

fn apply_possible_evaluation_traits<'p>(
    candidates: &[crate::registry_invocation::RegistryInvocationShape],
    args: &[String],
    braced: &[bool],
    values: &ArgumentValues<'p>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
) {
    use tcl_registry::Traits;
    for candidate in candidates {
        for (index, role) in candidate.written_argument_roles() {
            if role == ArgRole::Body
                && !braced.get(index).copied().unwrap_or(false)
                && candidate.possible_traits.intersects(
                    Traits::EVALUATES_CODE | Traits::DYNAMIC_EVAL_BODY | Traits::DEFERS_BODY,
                )
                && let Some(param) = values.argument(index)
                && let Some(set) = traits.get_mut(param)
            {
                set.insert(ProcArgTrait::Eval);
            }
        }
        if candidate
            .possible_traits
            .contains(Traits::SCRIPT_CONCATENATES_ARGS)
            && candidate
                .possible_traits
                .intersects(Traits::EVALUATES_CODE | Traits::DEFERS_BODY)
            && let Some(first) = candidate
                .roles
                .iter()
                .filter_map(|&(index, role)| {
                    (role == ArgRole::Body)
                        .then_some(candidate.argument_offset + usize::from(index))
                })
                .min()
        {
            for effective in first..candidate.effective.words.len() {
                if let Some(index) = candidate.effective.written_argument(effective)
                    && !braced.get(index).copied().unwrap_or(false)
                    && let Some(param) = values.argument(index)
                    && let Some(set) = traits.get_mut(param)
                {
                    set.insert(ProcArgTrait::Eval);
                }
            }
        }
        if candidate
            .possible_traits
            .contains(Traits::PERFORMS_SUBSTITUTION)
        {
            for index in 0..args.len() {
                if braced.get(index).copied().unwrap_or(false) {
                    continue;
                }
                if let Some(param) = values.argument(index)
                    && let Some(set) = traits.get_mut(param)
                {
                    set.insert(ProcArgTrait::Eval);
                }
            }
        }
        if !candidate.possible_traits.contains(Traits::LOOP_LIST_HEADER) {
            continue;
        }
        for &(index, role) in &candidate.roles {
            if role != ArgRole::LoopVarList {
                continue;
            }
            let effective = candidate.argument_offset + usize::from(index) + 1;
            if let Some(param) = candidate
                .effective
                .written_argument(effective)
                .and_then(|original| values.argument(original))
                && let Some(set) = traits.get_mut(param)
            {
                set.insert(ProcArgTrait::LoopList);
            }
        }
    }
}

fn record_selected_aliases<'p>(
    view: &crate::registry_invocation::InvocationBodyAssistance,
    selected: &crate::registry_invocation::ResolvedStatementInvocation,
    values: &ArgumentValues<'p>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    let Some((_, first)) = selected_alias_layout(view, selected) else {
        return;
    };
    for source in (first..selected.arguments.len()).step_by(2) {
        let destination = source + 1;
        let input = selected
            .effective
            .written_argument(source)
            .and_then(|original| values.argument(original));
        if let Some(param) = input
            && let Some(set) = traits.get_mut(param)
        {
            set.insert(ProcArgTrait::VarRead);
        }
        if let Some(param) = selected
            .effective
            .written_argument(destination)
            .and_then(|original| values.argument(original))
            && let Some(set) = traits.get_mut(param)
        {
            mark_dynamic_name_local(set);
        }
        let Some(local) = view.caller_template_literal_data(selected, destination) else {
            aliases.clear();
            continue;
        };
        aliases.value_copies.remove(&local);
        aliases.upvar.remove(&local);
        aliases.retired_inputs.insert(local.clone());
        if let Some(param) = input
            && is_local_scalar_name(&local, selected)
        {
            aliases.upvar.insert(local, param);
        }
    }
}

pub(super) fn selected_alias_layout(
    view: &crate::registry_invocation::InvocationBodyAssistance,
    selected: &crate::registry_invocation::ResolvedStatementInvocation,
) -> Option<(tcl_registry::frame_effect::FrameLevel, usize)> {
    use tcl_registry::frame_effect::{FrameArgLayout, FrameArgumentResolution, FrameLevel};
    if let Some(layout) = view.conditional_parameter_source_alias_layout(selected) {
        return layout.into_option();
    }
    let namespace_alias = selected
        .facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::VariableCellAlias(alias)
                if matches!(
                    alias.target,
                    tcl_registry::VariableAliasTarget::Namespace { .. }
                ) =>
            {
                Some(alias.words.target)
            }
            _ => None,
        })
        .min();
    if let Some(first) = namespace_alias {
        return Some((FrameLevel::Dynamic, first));
    }
    let effect = selected.facts.frame_effect?;
    if effect.layout != FrameArgLayout::AliasPairs {
        return None;
    }
    let words: Vec<_> = selected
        .evaluated_words
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
        .collect();
    let arguments =
        tcl_registry::InvocationArguments::structured(&words).with_dialect(selected.dialect?);
    match effect.successful_layout(arguments).layout {
        FrameArgumentResolution::Valid {
            level,
            level_word_len,
        } => Some((level, level_word_len)),
        FrameArgumentResolution::Invalid | FrameArgumentResolution::Unknown => None,
    }
}

fn apply_definite_writes<'p>(
    view: &crate::registry_invocation::InvocationBodyAssistance,
    selected: &crate::registry_invocation::ResolvedStatementInvocation,
    values: &ArgumentValues<'p>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    if selected_alias_layout(view, selected).is_some() {
        return;
    }
    // Freeze the read-side copy before applying this command's writes.
    let copy = (selected.facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Set,
        )
        && selected.arguments.len() == 2)
        .then(|| {
            let target = view.caller_template_literal_data(selected, 0)?;
            if !is_local_scalar_name(&target, selected) {
                return None;
            }
            let param = selected
                .effective
                .written_argument(1)
                .and_then(|index| values.argument(index))?;
            Some((target, param))
        })
        .flatten();
    for (index, role) in selected.written_argument_roles() {
        if role != ArgRole::VarWrite {
            continue;
        }
        let Some(name) = written_template_literal(view, selected, index) else {
            aliases.clear();
            continue;
        };
        if let Some(param) = aliases.upvar.get(name.as_str())
            && let Some(set) = traits.get_mut(param)
        {
            set.insert(ProcArgTrait::VarWrite);
        }
        aliases.value_copies.remove(name.as_str());
        aliases.retired_inputs.insert(name.clone());
    }
    if let Some((target, param)) = copy {
        aliases.value_copies.insert(target, param);
    }
}

/// Per-arg role-driven trait recording — apply
/// ``ArgRole::Body`` / ``Expr`` / ``VarWrite`` / ``VarRead`` to
/// the matching parameter trait set when an arg is a simple
/// ``$param`` reference (or aliases an upvar'd one).
///
/// `braced[i]` is `true` when argument `i` was a braced (`{…}`) literal word,
/// in which case **no substitution occurs**: `set {arr($k)} 1` names the literal
/// variable `arr($k)`, so the `$k` inside must not be read as a substitution.
/// The braced guard applies only to the variable-name
/// roles; a braced `Body` / `Expr` word still substitutes internally when Tcl
/// evaluates it, and the deep pass recurses into it.
fn apply_arg_role_traits<'p>(
    roles: &[(usize, ArgRole)],
    cmd_args: &[String],
    braced: &[bool],
    values: &ArgumentValues<'p>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
) {
    for idx in 0..cmd_args.len() {
        for role in roles
            .iter()
            .filter_map(|&(index, role)| (index == idx).then_some(role))
        {
            match role {
                // Body / Expr apply only to a bare `$param` (or upvar-aliased) word:
                // the whole argument *is* the param's value, evaluated as a script /
                // expression.
                ArgRole::Body => {
                    if let Some(p) = values.argument(idx)
                        && let Some(set) = traits.get_mut(p)
                    {
                        set.insert(ProcArgTrait::Body);
                    }
                }
                ArgRole::Expr => {
                    if let Some(p) = values.argument(idx)
                        && let Some(set) = traits.get_mut(p)
                    {
                        set.insert(ProcArgTrait::Expr);
                    }
                }
                // A registry / stub `VarWrite` / `VarRead` role names a variable.
                // Every param whose value flows into that name — the whole `$p`, or
                // a `$p` component of a compound dynamic name (`${p}($k)`,
                // `$p($k)`) — names a CALLEE-LOCAL variable (`set $p 1`,
                // `set ${v}($k)`, `incr $p`), NOT a caller-frame alias.  Record the
                // callee-local refinement (never `VarWrite`; only an `upvar`-aliased
                // write-back is a genuine caller write).  A braced word is a literal
                // name — no substitution — so no param flows into it.
                ArgRole::VarWrite | ArgRole::VarRead
                    if !braced.get(idx).copied().unwrap_or(false) =>
                {
                    for p in values.argument_components(idx) {
                        if let Some(set) = traits.get_mut(p) {
                            mark_dynamic_name_local(set);
                        }
                    }
                }
                // A registry / stub `CommandPrefix` role names a command — a callback
                // command prefix (`tcltest::customMatch`, `selection handle`, a stub
                // `:command_prefix` arg).  A `$param` (or component of a dynamic
                // name) flowing there means the param's value is a command name.
                // Braced words are literal — no substitution.
                ArgRole::CommandPrefix if !braced.get(idx).copied().unwrap_or(false) => {
                    for p in values.argument_components(idx) {
                        if let Some(set) = traits.get_mut(p) {
                            set.insert(ProcArgTrait::Command);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Map a substituted name to a parameter value, directly or through an
/// accepted value copy. An upvar's contents belong to the referenced cell;
/// they are not the parameter string that named that cell.
pub(super) fn lookup_param<'p>(
    var_name: &str,
    param_set: &HashSet<&'p str>,
    aliases: &Aliases<'p>,
) -> Option<&'p str> {
    aliases.value_copies.get(var_name).copied().or_else(|| {
        (aliases.direct_inputs_known && !aliases.retired_inputs.contains(var_name))
            .then(|| param_set.get(var_name).copied())
            .flatten()
    })
}

/// Record the callee-local dynamic-name use of a param — its **value**
/// is used as a variable *name* in the proc's own scope (``set $p 1``,
/// ``scan $s %d $p``, ``lassign $l $p``, ``regsub … $p``).  Emits
/// [`ProcArgTrait::DynamicNameLocal`] (the refinement that stops
/// caller-side dead-store / unused-variable suppression from treating
/// the call site as consuming the caller's variable) plus
/// [`ProcArgTrait::VarRead`] (the param's string *is* read).  Never
/// `VarWrite` — only an ``upvar``-aliased write-back is a genuine
/// caller-frame write.
fn mark_dynamic_name_local(set: &mut HashSet<ProcArgTrait>) {
    set.insert(ProcArgTrait::DynamicNameLocal);
    set.insert(ProcArgTrait::VarRead);
}
