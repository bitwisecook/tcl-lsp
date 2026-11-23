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

//! Option rows and invocation forms: an option's arity, lifecycle edit and effect, and the refinement rows of a command or subcommand.

use super::field_values::{
    appended_arity, arg_value, arity, dialects, insert_lifecycle, integer_domain, knowledge,
    opt_index, opt_str, role_map, side_effect, str_list, traits, variable_receiver_positions,
};
use super::rust_expressions::{option_relations_expr, representation_effect_expr};
use super::{
    OPTION_DEPRECATION_FIX_HOOK_KEY, OPTION_DEPRECATION_FIX_UNRECOVERABLE_KEY, OPTION_HOOK_KEY,
    Unrecovered,
};
use crate::catalogue;
use crate::render_rs::rust_string;
use serde_json::{Map, Value, json};
use tcl_registry::deprecation::{DeprecationFixHook, DeprecationFixSafety};
use tcl_registry::forms::CommandForm;
use tcl_registry::hover::{FormSpec, OptionArity, OptionSpec, OptionValue};
use tcl_registry::option_effect::{EffectAxis, OptionEffect, OptionEffectKind};

fn option_arity(value: OptionArity) -> (Value, bool) {
    match value {
        OptionArity::One => (json!({ "kind": "One" }), true),
        OptionArity::Fixed(n) => (json!({ "kind": "Fixed", "n": n }), true),
        OptionArity::OptionalOneOf(values) => {
            (json!({ "kind": "OptionalOneOf", "values": values }), true)
        }
        OptionArity::OptionalNotOption => (json!({ "kind": "OptionalNotOption" }), true),
        // The hook is a function pointer: the *shape* round-trips, but the
        // expression that produced it does not. `hook` is the slot the author
        // fills in under the option's editor; it seeds empty, and the option
        // is complete once it holds an expression.
        OptionArity::Hook(_) => (json!({ "kind": "Hook", "hook": Value::Null }), false),
    }
}

fn deprecation_fix_safety_expr(safety: DeprecationFixSafety) -> &'static str {
    match safety {
        DeprecationFixSafety::SemanticsEquivalent => "DeprecationFixSafety::SemanticsEquivalent",
        DeprecationFixSafety::RequiresReview => "DeprecationFixSafety::RequiresReview",
    }
}

/// Recover a declarative lifecycle edit exactly. Callback function pointers
/// deliberately abstain: Rust does not expose the symbol path needed in a
/// rendered source file.
pub(super) fn deprecation_fix_expr(hook: DeprecationFixHook) -> Option<String> {
    let expr = match hook {
        DeprecationFixHook::ReplaceMatchedWord {
            replacement,
            description,
            safety,
        } => format!(
            "DeprecationFixHook::ReplaceMatchedWord {{ replacement: {}, description: {}, safety: {} }}",
            rust_string(replacement),
            rust_string(description),
            deprecation_fix_safety_expr(safety),
        ),
        DeprecationFixHook::ReplaceArgument {
            index,
            replacement,
            description,
            safety,
        } => format!(
            "DeprecationFixHook::ReplaceArgument {{ index: {index}, replacement: {}, description: {}, safety: {} }}",
            rust_string(replacement),
            rust_string(description),
            deprecation_fix_safety_expr(safety),
        ),
        DeprecationFixHook::ReplaceInvocation {
            replacement,
            description,
            safety,
        } => format!(
            "DeprecationFixHook::ReplaceInvocation {{ replacement: {}, description: {}, safety: {} }}",
            rust_string(replacement),
            rust_string(description),
            deprecation_fix_safety_expr(safety),
        ),
        DeprecationFixHook::Custom { .. } => return None,
    };
    Some(format!("Some({expr})"))
}

pub(super) fn deprecation_fix_value(hook: Option<DeprecationFixHook>) -> (Value, bool) {
    match hook {
        None => (Value::Null, true),
        Some(hook) => match deprecation_fix_expr(hook) {
            Some(expr) => (json!(expr), true),
            None => (Value::Null, false),
        },
    }
}

/// The draft form of one [`EffectAxis`]: its axis word, and its value word
/// when the axis carries one (`case-sensitivity` does not).
fn insert_axis(d: &mut Map<String, Value>, axis: EffectAxis) {
    d.insert("axis".into(), json!(axis.axis_word()));
    d.insert(
        "value".into(),
        axis.value_word().map_or(Value::Null, |word| json!(word)),
    );
}

/// The draft form of an [`OptionSpec::effect`]: `null`, or a flat tagged
/// object naming the kind, its own payload (an axis and value, a role, or a
/// count), and the family — the option-row form's own two controls, the
/// inverse of `render_spectcl.rs`'s `option_effect_kind`.
pub(super) fn option_effect(effect: Option<OptionEffect>) -> Value {
    let Some(OptionEffect { kind, family }) = effect else {
        return Value::Null;
    };
    let mut d = Map::new();
    d.insert("family".into(), json!(family));
    match kind {
        OptionEffectKind::Disables(axis) => {
            d.insert("kind".into(), json!("disables"));
            insert_axis(&mut d, axis);
        }
        OptionEffectKind::Selects(axis) => {
            d.insert("kind".into(), json!("selects"));
            insert_axis(&mut d, axis);
        }
        OptionEffectKind::SuppressesRole(role) => {
            d.insert("kind".into(), json!("suppresses-role"));
            d.insert("role".into(), json!(catalogue::variant_name(&role)));
        }
        OptionEffectKind::ReservesTrailingWords(n) => {
            d.insert("kind".into(), json!("reserves-trailing-words"));
            d.insert("n".into(), json!(n));
        }
        OptionEffectKind::EndsOptions => {
            d.insert("kind".into(), json!("ends-options"));
        }
    }
    Value::Object(d)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OptionDraftCompleteness {
    pub(crate) arity_hook: bool,
    pub(crate) deprecation_fix_hook: bool,
}

/// Draft form of an option, plus whether it round-tripped completely.
pub(crate) fn option_spec(opt: &OptionSpec) -> (Value, OptionDraftCompleteness) {
    // A value row's own quick-fix hook is unrecoverable for the same reason the
    // option's is, and is filled in at the same place — inside this option's
    // row editor — so it rides the same completeness flag.
    let mut values_complete = true;
    let values: Vec<Value> = match opt.value {
        OptionValue::Takes(arg) => arg
            .values
            .iter()
            .map(|value| {
                let (row, complete) = arg_value(value);
                values_complete &= complete;
                row
            })
            .collect(),
        OptionValue::Flag => Vec::new(),
    };
    let (value, arity_hook) = match opt.value {
        OptionValue::Flag => (Value::Null, true),
        OptionValue::Takes(arg) => {
            let (arity_json, arity_complete) = option_arity(arg.arity);
            (
                json!({
                    "arity": arity_json,
                    "role": catalogue::variant_name(&arg.role),
                    "also_role": arg.also_role.map_or(Value::Null, |r| json!(catalogue::variant_name(&r))),
                    "body_kind": catalogue::variant_name(&arg.body_kind),
                    "script_timing": catalogue::variant_name(&arg.script_timing),
                    "callback_taint_inputs": arg.callback_taint_inputs.iter().map(|input| input.spelling()).collect::<Vec<_>>(),
                    "variable_scope": catalogue::variant_name(&arg.variable_scope),
                    "values": Value::Array(values),
                    "closed": arg.closed,
                    "integer": integer_domain(arg.integer),
                    "hint": arg.hint,
                    "appended_arity": appended_arity(arg.appended_arity),
                    "taints_var_write": arg.taints_var_write,
                }),
                arity_complete,
            )
        }
    };
    let (deprecation_fix, deprecation_fix_hook) =
        deprecation_fix_value(opt.lifecycle.deprecation_fix);
    (
        json!({
            "name": opt.name,
            "detail": opt.detail,
            "surface": dialects(opt.surface),
            "aliases": str_list(opt.aliases),
            "introduced_version": opt_str(opt.lifecycle.introduced),
            "deprecated_version": opt_str(opt.lifecycle.deprecated),
            "retired_version": opt_str(opt.lifecycle.retired),
            "deprecation_fix": deprecation_fix,
            OPTION_DEPRECATION_FIX_UNRECOVERABLE_KEY: !deprecation_fix_hook,
            "min_abbrev": opt_index(opt.min_abbrev),
            "value": value,
            "effect": option_effect(opt.effect),
        }),
        OptionDraftCompleteness {
            arity_hook,
            deprecation_fix_hook: deprecation_fix_hook && values_complete,
        },
    )
}

pub(crate) fn form_spec(form: &FormSpec) -> (Value, bool) {
    let mut d = Map::new();
    d.insert("kind".into(), json!(catalogue::variant_name(&form.kind)));
    d.insert("synopsis".into(), json!(form.synopsis));
    d.insert("surface".into(), dialects(form.surface));
    let complete = insert_lifecycle(&mut d, form.lifecycle);
    (Value::Object(d), complete)
}

/// The `command_forms` / `subcommand_forms` table as draft rows.
pub(super) fn refinement_rows(
    forms: &[CommandForm],
    key: &'static str,
    lost: &mut Unrecovered,
) -> Value {
    Value::Array(
        forms
            .iter()
            .map(|form| {
                let (row, complete) = command_form(form, lost);
                if !complete {
                    lost.note(key);
                }
                row
            })
            .collect(),
    )
}

/// One invocation refinement (`CommandForm` / `SubCommandForm`) as draft rows
/// (design Q12/D2).
///
/// The overlay fields are tri-state — absent inherits, present replaces — so a
/// `null` here is a real declaration, not a missing one. The descriptor's
/// remaining native halves (completion, dispatch proofs, the literal-argument
/// validator, the compiler hook ids) have no recoverable expression, exactly
/// as they do not at command scope, so a form that carries one is noted lost
/// rather than silently thinned.
fn command_form(form: &CommandForm, lost: &mut Unrecovered) -> (Value, bool) {
    let mut d = Map::new();
    d.insert("name".into(), json!(form.name));
    d.insert("arity".into(), arity(form.arity));
    d.insert(
        "selector".into(),
        form.literal_argument_prefix.map_or(Value::Null, |prefix| {
            json!({
                "words": str_list(prefix.words),
                "prefix_matching": catalogue::variant_name(&prefix.prefix_matching),
            })
        }),
    );
    d.insert("arg_roles".into(), role_map(form.arg_roles));
    d.insert(
        "variable_receivers".into(),
        variable_receiver_positions(form.variable_receivers),
    );
    d.insert("options".into(), option_rows(form.options, lost));
    d.insert(
        "option_relations".into(),
        if form.option_relations.is_empty() {
            Value::Null
        } else {
            option_relations_expr(form.option_relations).map_or(Value::Null, |expr| json!(expr))
        },
    );
    d.insert("surface".into(), dialects(form.surface));
    d.insert("traits".into(), form.traits.map_or(Value::Null, traits));
    d.insert(
        "mutator".into(),
        form.mutator.map_or(Value::Null, |flag| json!(flag)),
    );
    d.insert("header_write".into(), form.header_write.map_or(Value::Null, |write| {
        json!({"name": write.name, "value": write.value, "returns_empty": write.returns_empty})
    }));
    d.insert(
        "side_effects".into(),
        form.side_effects.map_or(Value::Null, |effects| {
            Value::Array(
                effects
                    .lower_bound()
                    .iter()
                    .map(|effect| side_effect(effect).0)
                    .collect(),
            )
        }),
    );
    d.insert(
        "side_effects_knowledge".into(),
        form.side_effects.as_ref().map_or(Value::Null, knowledge),
    );
    d.insert(
        "representation_effect".into(),
        form.representation_effect
            .map_or(Value::Null, |e| json!(representation_effect_expr(e))),
    );
    d.insert(
        "lowering_hook".into(),
        form.lowering_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "codegen_hook".into(),
        form.codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    // What is left of the descriptor is native: a compiler proof or a
    // function pointer, with no expression to recover.
    if form.source_package_query.is_some() {
        lost.note("source_package_query");
    }
    let native = form.source_package_query.is_some()
        || form.semantic_operation.is_some()
        || !form.completion.is_unknown()
        || !form.result_stability.is_unknown()
        || form.native_result.is_some()
        || !form.world_effects.is_unknown()
        || !form.state_transitions.is_unknown()
        || form.dispatch_dependencies.is_some()
        || form.literal_argument_validator.is_some();
    let value_overrides = form.return_type.is_some()
        || form.arg_types.is_some()
        || form.byte_array_effect.is_some()
        || form.var_elements_effect.is_some()
        || form.safe_on_uninit.is_some();
    if value_overrides {
        lost.note("form_value_effects");
    }
    (Value::Object(d), !native && !value_overrides)
}

pub(super) fn option_rows(options: &[OptionSpec], lost: &mut Unrecovered) -> Value {
    Value::Array(
        options
            .iter()
            .map(|option| {
                let (row, complete) = option_spec(option);
                if !complete.arity_hook {
                    lost.note(OPTION_HOOK_KEY);
                }
                if !complete.deprecation_fix_hook {
                    lost.note(OPTION_DEPRECATION_FIX_HOOK_KEY);
                }
                row
            })
            .collect(),
    )
}
