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

//! Seeding a draft from a live `SubCommand`: identity, types, hooks, the option surface and execution fields.

use super::definition_grammar::{clause_grammar, semantic_operation};
use super::field_values::{
    arg_type_map, arg_value_map, arity, arity_windows, body_interpreter, callback_taint_map,
    dialects, hook_name, hover, index_list, insert_lifecycle, knowledge, nested_rows, opt_index,
    opt_str, prefix_map, presentation_map, role_list, role_map, semantics_value, side_effect,
    stamp_windows, str_list, taint, traits, variable_receiver_positions,
};
use super::options_and_forms::{option_rows, refinement_rows};
use super::rust_expressions::{
    expr_or_null, option_effect_families_expr, option_relations_expr, repeated_args_expr,
    representation_effect_expr, return_elements_expr, var_elements_effect_expr,
    var_write_typing_expr, versioned_arg_values_expr,
};
use super::{Draft, Unrecovered};
use crate::catalogue;
use serde_json::{Map, Value, json};
use tcl_registry::spec::{SubCommand, SubSubCommand};

pub(crate) fn sub_subcommand(sub: &SubSubCommand) -> (Value, bool) {
    let mut d = Map::new();
    d.insert("option_prefix_words".into(), json!(sub.option_prefix_words));
    d.insert("name".into(), json!(sub.name));
    d.insert("detail".into(), json!(sub.detail));
    d.insert("synopsis".into(), json!(sub.synopsis));
    d.insert("surface".into(), dialects(sub.surface));
    d.insert(
        "native_compilation".into(),
        sub.native_compilation
            .map_or(Value::Null, |_| json!("excluded native worker proof")),
    );
    let mut lost = Unrecovered::default();
    // `null` — declares nothing, inherits the subcommand's table — is a
    // different draft value from `[]`, which declares that there are no
    // options here at all.
    d.insert(
        "options".into(),
        sub.options
            .map_or(Value::Null, |options| option_rows(options, &mut lost)),
    );
    let complete = insert_lifecycle(&mut d, sub.lifecycle)
        && lost.is_empty()
        && sub.native_compilation.is_none();
    (Value::Object(d), complete)
}

fn subcommand_value_and_effect_fields(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    d.insert(
        "closed_value_args".into(),
        index_list(sub.closed_value_args),
    );
    d.insert(
        "arg_values_accept_prefix".into(),
        json!(sub.arg_values_accept_prefix),
    );
    d.insert(
        "body_arg_implicit_args".into(),
        json!(sub.body_arg_implicit_args),
    );
    d.insert("taint_transform".into(), taint(sub.taint_transform));
    d.insert(
        "taint_transform_when".into(),
        sub.taint_transform_when
            .map_or(Value::Null, |c| json!(catalogue::variant_name(&c))),
    );
    d.insert(
        "taint_double_encode_colour".into(),
        taint(sub.taint_double_encode_colour),
    );
    d.insert("taint_output_sink".into(), opt_str(sub.taint_output_sink));
    d.insert(
        "channel_configuration".into(),
        sub.channel_configuration.map_or(Value::Null, |descriptor| {
            json!(catalogue::variant_name(&descriptor))
        }),
    );
    d.insert("credential_arg".into(), opt_index(sub.credential_arg));
    d.insert("sensitive_headers".into(), str_list(sub.sensitive_headers));
    d.insert(
        "pattern_type".into(),
        sub.pattern_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "format_string_type".into(),
        sub.format_string_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "side_effects".into(),
        nested_rows(
            sub.side_effects.lower_bound(),
            "side_effects",
            lost,
            side_effect,
        ),
    );
    d.insert(
        "side_effects_knowledge".into(),
        knowledge(&sub.side_effects),
    );
    d.insert("destructive".into(), json!(sub.destructive));
    d.insert("returns_path".into(), json!(sub.returns_path));
    d.insert(
        "source_index_bounds".into(),
        sub.source_index_bounds.map_or(Value::Null, |operation| {
            json!(catalogue::variant_name(&operation))
        }),
    );
    d.insert(
        "source_path_operation".into(),
        sub.source_path_operation.map_or(Value::Null, |operation| {
            json!(catalogue::variant_name(&operation))
        }),
    );
    d.insert(
        "source_configuration_reference".into(),
        sub.source_configuration_reference
            .map_or(Value::Null, |layout| {
                json!(catalogue::variant_name(&layout))
            }),
    );
    d.insert("is_unescape".into(), json!(sub.is_unescape));
    d.insert("cfg_rewrite_name".into(), opt_str(sub.cfg_rewrite_name));
    d.insert(
        "sub_subcommands".into(),
        nested_rows(sub.sub_subcommands, "sub_subcommands", lost, sub_subcommand),
    );
    d.insert(
        "defines_command_at".into(),
        opt_index(sub.defines_command_at),
    );
    d.insert(
        "max_leading_option_words".into(),
        opt_index(sub.max_leading_option_words),
    );
}

pub(super) fn subcommand_body(sub: &SubCommand, lost: &mut Unrecovered) -> Draft {
    if sub.source_package_query.is_some() {
        lost.note("source_package_query");
    }
    let mut d = Map::new();
    subcommand_identity(&mut d, sub, lost);
    subcommand_types(&mut d, sub);
    subcommand_hooks(&mut d, sub, lost);
    subcommand_rest(&mut d, sub, lost);
    d
}

/// Name, traits, arity, documentation, and argument layout.
fn subcommand_identity(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    d.insert("name".into(), json!(sub.name));
    d.insert("traits".into(), traits(sub.traits));
    d.insert("arity".into(), arity(sub.arity));
    d.insert("arity_windows".into(), arity_windows(sub.arity_windows));
    d.insert("detail".into(), json!(sub.detail));
    d.insert("synopsis".into(), json!(sub.synopsis));
    d.insert("hover".into(), hover(sub.hover));
    d.insert("arg_roles".into(), role_map(sub.arg_roles));
    d.insert(
        "variable_receivers".into(),
        variable_receiver_positions(sub.variable_receivers),
    );
    d.insert(
        "arg_presentation".into(),
        presentation_map(sub.arg_presentation),
    );
    d.insert(
        "repeated_args".into(),
        expr_or_null(!sub.repeated_args.is_empty(), || {
            repeated_args_expr(sub.repeated_args)
        }),
    );
    d.insert(
        "arg_role_resolver".into(),
        lost.expr("arg_role_resolver", sub.arg_role_resolver.is_some()),
    );
    d.insert(
        "arg_role_count_resolver".into(),
        lost.expr(
            "arg_role_count_resolver",
            sub.arg_role_count_resolver.is_some(),
        ),
    );
    d.insert(
        "arg_role_layout_resolver".into(),
        lost.expr(
            "arg_role_layout_resolver",
            sub.arg_role_layout_resolver.is_some(),
        ),
    );
    d.insert(
        "arg_role_resolver_roles".into(),
        role_list(sub.arg_role_resolver_roles),
    );
    d.insert("clause_grammar".into(), clause_grammar(sub.clause_grammar));
    d.insert("command_prefixes".into(), prefix_map(sub.command_prefixes));
    d.insert(
        "callback_taint_inputs".into(),
        callback_taint_map(sub.callback_taint_inputs),
    );
    d.insert(
        "command_prefix_resolver".into(),
        lost.expr(
            "command_prefix_resolver",
            sub.command_prefix_resolver.is_some(),
        ),
    );
    d.insert(
        "script_timing_resolver".into(),
        lost.expr(
            "script_timing_resolver",
            sub.script_timing_resolver.is_some(),
        ),
    );
}

/// Return type, element structure, and per-argument type hints.
fn subcommand_types(d: &mut Draft, sub: &SubCommand) {
    d.insert(
        "return_type".into(),
        sub.return_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "var_write_typing".into(),
        json!(var_write_typing_expr(sub.var_write_typing)),
    );
    d.insert(
        "return_elements".into(),
        sub.return_elements
            .map_or(Value::Null, |e| json!(return_elements_expr(e))),
    );
    d.insert(
        "var_elements_effect".into(),
        sub.var_elements_effect
            .map_or(Value::Null, |e| json!(var_elements_effect_expr(e))),
    );
    d.insert(
        "representation_effect".into(),
        sub.representation_effect
            .map_or(Value::Null, |e| json!(representation_effect_expr(e))),
    );
    d.insert("arg_types".into(), arg_type_map(sub.arg_types));
    d.insert("pure".into(), json!(sub.pure));
    d.insert("mutator".into(), json!(sub.mutator));
}

/// Constant folders and the compiler / analyser hook IDs.
fn subcommand_hooks(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    d.insert(
        "const_fold".into(),
        lost.expr("const_fold", sub.const_fold.is_some()),
    );
    d.insert(
        "const_fold_versioned".into(),
        lost.expr("const_fold_versioned", sub.const_fold_versioned.is_some()),
    );
    d.insert(
        "semantic_operation".into(),
        semantic_operation(sub.semantic_operation),
    );
    d.insert(
        "semantic_operation_windows".into(),
        stamp_windows(sub.semantic_operation_windows, |operation| {
            semantic_operation(Some(operation))
        }),
    );
    d.insert(
        "completion".into(),
        lost.expr("completion", sub.completion.is_some()),
    );
    d.insert(
        "lowering_hook".into(),
        sub.lowering_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "codegen_hook".into(),
        sub.codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "inline_codegen_hook".into(),
        sub.inline_codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "codegen_hook_windows".into(),
        stamp_windows(sub.codegen_hook_windows, hook_name),
    );
    d.insert(
        "inline_codegen_hook_windows".into(),
        stamp_windows(sub.inline_codegen_hook_windows, hook_name),
    );
    d.insert("semantics".into(), semantics_value(sub.semantics, lost));
    d.insert(
        "analyser_hook".into(),
        sub.analyser_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "command_table_effect".into(),
        sub.command_table_effect
            .map_or(Value::Null, |e| json!(catalogue::variant_name(&e))),
    );
    d.insert(
        "world_effects".into(),
        lost.expr("world_effects", sub.world_effects.is_some()),
    );
    d.insert(
        "state_transitions".into(),
        lost.expr("state_transitions", sub.state_transitions.is_some()),
    );
    d.insert(
        "dispatch_dependencies".into(),
        lost.expr("dispatch_dependencies", sub.dispatch_dependencies.is_some()),
    );
    d.insert(
        "result_stability".into(),
        lost.expr("result_stability", sub.result_stability.is_some()),
    );
    d.insert(
        "native_result".into(),
        lost.expr("native_result", sub.native_result.is_some()),
    );
    d.insert(
        "constraints".into(),
        lost.expr("constraints", sub.constraints.is_some()),
    );
    d.insert(
        "literal_argument_validator".into(),
        lost.expr(
            "literal_argument_validator",
            sub.literal_argument_validator.is_some(),
        ),
    );
}

/// Options, values, availability, behaviour flags, taint, and effects.
/// The subcommand's option table and the E-R14 relation fields over it.
///
/// Split out of [`subcommand_rest`] so neither grows past the line budget:
/// the option surface is one topic and the rest of the descriptor is another.
fn subcommand_option_surface(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    d.insert("options".into(), option_rows(sub.options, lost));
    let option_relations = if sub.option_relations.is_empty() {
        Value::Null
    } else {
        option_relations_expr(sub.option_relations)
            .map_or_else(|| lost.expr("option_relations", true), |expr| json!(expr))
    };
    d.insert("option_relations".into(), option_relations);
    let option_effect_families = if sub.option_effect_families.is_empty() {
        Value::Null
    } else {
        json!(option_effect_families_expr(sub.option_effect_families))
    };
    d.insert("option_effect_families".into(), option_effect_families);
    d.insert(
        "option_placement".into(),
        json!(catalogue::variant_name(&sub.option_placement)),
    );
}

/// Reflect the selected body and handler contracts together, so a subcommand
/// draft cannot retain body metadata while omitting its execution policy.
fn subcommand_execution_fields(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    d.insert(
        "body_kind".into(),
        json!(catalogue::variant_name(&sub.body_kind)),
    );
    d.insert(
        "script_lookup_scope".into(),
        sub.script_lookup_scope
            .map_or(Value::Null, |scope| json!(catalogue::variant_name(&scope))),
    );
    d.insert(
        "body_execution".into(),
        lost.expr("body_execution", sub.body_execution.is_some()),
    );
    d.insert(
        "native_compilation".into(),
        lost.expr("native_compilation", sub.native_compilation.is_some()),
    );
    d.insert(
        "successful_handler".into(),
        sub.successful_handler.map_or(Value::Null, |contract| {
            json!(format!(
                "Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::{contract:?})"
            ))
        }),
    );
    d.insert(
        "body_interpreter".into(),
        body_interpreter(sub.body_interpreter),
    );
    d.insert(
        "byte_array_effect".into(),
        json!(catalogue::variant_name(&sub.byte_array_effect)),
    );
}

fn subcommand_rest(d: &mut Draft, sub: &SubCommand, lost: &mut Unrecovered) {
    subcommand_option_surface(d, sub, lost);
    d.insert("min_abbrev".into(), opt_index(sub.min_abbrev));
    d.insert(
        "prefix_matching".into(),
        json!(catalogue::variant_name(&sub.prefix_matching)),
    );
    d.insert("option_prefix_words".into(), json!(sub.option_prefix_words));
    d.insert(
        "reserved_trailing_words".into(),
        json!(sub.reserved_trailing_words),
    );
    d.insert("arg_values".into(), arg_value_map(sub.arg_values, lost));
    let versioned_arg_values = if sub.versioned_arg_values.is_empty() {
        Value::Null
    } else {
        versioned_arg_values_expr(sub.versioned_arg_values).map_or_else(
            || lost.expr("versioned_arg_values", true),
            |expr| json!(expr),
        )
    };
    d.insert("versioned_arg_values".into(), versioned_arg_values);
    d.insert(
        "subcommand_forms".into(),
        refinement_rows(sub.subcommand_forms, "subcommand_forms", lost),
    );
    d.insert("surface".into(), dialects(sub.surface));
    if !insert_lifecycle(d, sub.lifecycle) {
        lost.note("deprecation_fix");
    }
    d.insert("safe_on_uninit".into(), dialects(sub.safe_on_uninit));
    d.insert("loop_list_header".into(), json!(sub.loop_list_header));
    d.insert("creates_scope_alias".into(), json!(sub.creates_scope_alias));
    d.insert(
        "inferred_storage_type".into(),
        sub.inferred_storage_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    subcommand_execution_fields(d, sub, lost);
    subcommand_value_and_effect_fields(d, sub, lost);
}
