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

//! Seeding a draft from a live `CommandSpec`: identity, types, documentation, hooks, iRules events, options, taint and object facts.

use super::definition_grammar::{
    clause_grammar, definition_body, manufacturer_method, object_class, semantic_operation,
};
use super::field_values::{
    arg_type_map, arg_value_map, arity, arity_windows, body_interpreter, callback_taint_map,
    dialects, documentation_examples, hook_name, hover, index_list, insert_lifecycle, knowledge,
    nested_rows, opt_index, opt_str, prefix_map, presentation_map, role_list, role_map,
    semantics_value, setter_constraint, side_effect, stamp_windows, str_list, taint, traits,
    variable_receiver_positions,
};
use super::options_and_forms::{form_spec, option_rows, refinement_rows};
use super::rust_expressions::{
    byte_payload_expr, expr_or_null, handle_binding_expr, oo_context_facts_expr,
    option_effect_families_expr, option_relations_expr, remote_method_expr, repeated_args_expr,
    representation_effect_expr, return_elements_expr, symbol_def_expr, var_elements_effect_expr,
    var_write_typing_expr, versioned_arg_values_expr,
};
use super::subcommand_seed::subcommand_body;
use super::{Draft, UNRENDERABLE_KEY, Unrecovered};
use crate::catalogue;
use serde_json::{Value, json};
use tcl_registry::spec::CommandSpec;

/// Name, traits, availability, arity, and argument layout.
pub(super) fn command_identity(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert("name".into(), json!(spec.name));
    d.insert("traits".into(), traits(spec.traits));
    d.insert("surface".into(), dialects(spec.surface));
    d.insert("arity".into(), arity(spec.arity));
    d.insert("arity_windows".into(), arity_windows(spec.arity_windows));
    d.insert("arg_roles".into(), role_map(spec.arg_roles));
    d.insert(
        "variable_receivers".into(),
        variable_receiver_positions(spec.variable_receivers),
    );
    d.insert(
        "arg_presentation".into(),
        presentation_map(spec.arg_presentation),
    );
    d.insert(
        "repeated_args".into(),
        expr_or_null(!spec.repeated_args.is_empty(), || {
            repeated_args_expr(spec.repeated_args)
        }),
    );
    d.insert(
        "arg_role_resolver".into(),
        lost.expr("arg_role_resolver", spec.arg_role_resolver.is_some()),
    );
    d.insert(
        "arg_role_count_resolver".into(),
        lost.expr(
            "arg_role_count_resolver",
            spec.arg_role_count_resolver.is_some(),
        ),
    );
    d.insert(
        "arg_role_layout_resolver".into(),
        lost.expr(
            "arg_role_layout_resolver",
            spec.arg_role_layout_resolver.is_some(),
        ),
    );
    d.insert(
        "arg_role_resolver_roles".into(),
        role_list(spec.arg_role_resolver_roles),
    );
    d.insert("clause_grammar".into(), clause_grammar(spec.clause_grammar));
    d.insert(
        "frame_effect".into(),
        lost.expr("frame_effect", spec.frame_effect.is_some()),
    );
    d.insert("bpf_op".into(), lost.expr("bpf_op", spec.bpf_op.is_some()));
    d.insert(
        "native_lowering".into(),
        lost.expr("native_lowering", spec.native_lowering.is_some()),
    );
    d.insert(
        "native_lowering_windows".into(),
        lost.expr(
            "native_lowering_windows",
            !spec.native_lowering_windows.is_empty(),
        ),
    );
    d.insert("semantics".into(), semantics_value(spec.semantics, lost));
    d.insert(
        "clause_shape_check".into(),
        lost.expr("clause_shape_check", spec.clause_shape_check.is_some()),
    );
    d.insert("command_prefixes".into(), prefix_map(spec.command_prefixes));
    d.insert(
        "callback_taint_inputs".into(),
        callback_taint_map(spec.callback_taint_inputs),
    );
    d.insert(
        "command_prefix_resolver".into(),
        lost.expr(
            "command_prefix_resolver",
            spec.command_prefix_resolver.is_some(),
        ),
    );
    d.insert(
        "script_timing_resolver".into(),
        lost.expr(
            "script_timing_resolver",
            spec.script_timing_resolver.is_some(),
        ),
    );
}

/// Return type, element structure, and per-argument type hints.
pub(super) fn command_types(d: &mut Draft, spec: &CommandSpec, _lost: &mut Unrecovered) {
    d.insert(
        "return_type".into(),
        spec.return_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "var_write_typing".into(),
        json!(var_write_typing_expr(spec.var_write_typing)),
    );
    d.insert(
        "variable_write_min_args".into(),
        spec.variable_write_min_args
            .map_or(Value::Null, |n| json!(n)),
    );
    d.insert(
        "return_type_hook".into(),
        spec.return_type_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "return_elements".into(),
        spec.return_elements
            .map_or(Value::Null, |e| json!(return_elements_expr(e))),
    );
    d.insert(
        "var_elements_effect".into(),
        spec.var_elements_effect
            .map_or(Value::Null, |e| json!(var_elements_effect_expr(e))),
    );
    d.insert(
        "representation_effect".into(),
        spec.representation_effect
            .map_or(Value::Null, |e| json!(representation_effect_expr(e))),
    );
    d.insert("arg_types".into(), arg_type_map(spec.arg_types));
}

/// Subcommands, documentation, and invocation forms.
pub(super) fn command_docs(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    let subcommands: Vec<Value> = spec
        .subcommands
        .iter()
        .map(|sub| {
            let mut sub_lost = Unrecovered::default();
            let mut body = subcommand_body(sub, &mut sub_lost);
            body.insert(UNRENDERABLE_KEY.to_owned(), sub_lost.into_value());
            Value::Object(body)
        })
        .collect();
    d.insert("subcommands".into(), Value::Array(subcommands));
    d.insert(
        "allow_unknown_subcommands".into(),
        json!(spec.allow_unknown_subcommands),
    );
    d.insert(
        "option_placement".into(),
        json!(catalogue::variant_name(&spec.option_placement)),
    );
    d.insert(
        "prefix_matching".into(),
        json!(catalogue::variant_name(&spec.prefix_matching)),
    );
    d.insert(
        "default_form_first_word".into(),
        spec.default_form_first_word
            .map_or(Value::Null, |w| json!(catalogue::variant_name(&w))),
    );
    d.insert("hover".into(), hover(spec.hover));
    d.insert(
        "documentation_examples".into(),
        documentation_examples(spec.documentation_examples),
    );
    d.insert(
        "forms".into(),
        nested_rows(spec.forms, "forms", lost, form_spec),
    );
    d.insert(
        "command_forms".into(),
        refinement_rows(spec.command_forms, "command_forms", lost),
    );
    d.insert(
        "assigns_variable_at".into(),
        opt_index(spec.assigns_variable_at),
    );
    d.insert("safe_on_uninit".into(), dialects(spec.safe_on_uninit));
}

/// Constant folders, compiler / analyser hooks, and effects.
pub(super) fn command_hooks(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert(
        "const_fold".into(),
        lost.expr("const_fold", spec.const_fold.is_some()),
    );
    d.insert(
        "const_fold_versioned".into(),
        lost.expr("const_fold_versioned", spec.const_fold_versioned.is_some()),
    );
    d.insert(
        "semantic_operation".into(),
        semantic_operation(spec.semantic_operation),
    );
    d.insert(
        "semantic_operation_windows".into(),
        stamp_windows(spec.semantic_operation_windows, |operation| {
            semantic_operation(Some(operation))
        }),
    );
    d.insert(
        "completion".into(),
        lost.expr("completion", !spec.completion.is_unknown()),
    );
    d.insert("completion_knowledge".into(), knowledge(&spec.completion));
    d.insert(
        "lowering_hook".into(),
        spec.lowering_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    command_codegen_hooks(d, spec);
    d.insert(
        "analyser_hook".into(),
        spec.analyser_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "command_table_effect".into(),
        spec.command_table_effect
            .map_or(Value::Null, |e| json!(catalogue::variant_name(&e))),
    );
    command_effects(d, spec, lost);
    d.insert(
        "native_result".into(),
        lost.expr("native_result", spec.native_result.is_some()),
    );
    d.insert(
        "constraints".into(),
        lost.expr("constraints", spec.constraints.is_some()),
    );
    d.insert(
        "source_index_bounds".into(),
        spec.source_index_bounds.map_or(Value::Null, |operation| {
            json!(catalogue::variant_name(&operation))
        }),
    );
    d.insert(
        "literal_argument_validator".into(),
        lost.expr(
            "literal_argument_validator",
            spec.literal_argument_validator.is_some(),
        ),
    );
    d.insert(
        "inferred_storage_type".into(),
        spec.inferred_storage_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
}

/// The effect domains, and how completely each is known (issue #1712).
fn command_effects(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert(
        "side_effects".into(),
        nested_rows(
            spec.side_effects.lower_bound(),
            "side_effects",
            lost,
            side_effect,
        ),
    );
    d.insert(
        "side_effects_knowledge".into(),
        knowledge(&spec.side_effects),
    );
    d.insert(
        "world_effects".into(),
        lost.expr("world_effects", !spec.world_effects.is_unknown()),
    );
    d.insert(
        "world_effects_knowledge".into(),
        knowledge(&spec.world_effects),
    );
    d.insert(
        "state_transitions".into(),
        lost.expr("state_transitions", spec.state_transitions.is_some()),
    );
    d.insert(
        "dispatch_dependencies".into(),
        lost.expr(
            "dispatch_dependencies",
            spec.dispatch_dependencies.is_some(),
        ),
    );
    d.insert(
        "result_stability".into(),
        lost.expr("result_stability", spec.result_stability.is_some()),
    );
}

fn command_codegen_hooks(d: &mut Draft, spec: &CommandSpec) {
    d.insert(
        "codegen_hook".into(),
        spec.codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "inline_codegen_hook".into(),
        spec.inline_codegen_hook
            .map_or(Value::Null, |h| json!(catalogue::variant_name(&h))),
    );
    d.insert(
        "codegen_hook_windows".into(),
        stamp_windows(spec.codegen_hook_windows, hook_name),
    );
    d.insert(
        "inline_codegen_hook_windows".into(),
        stamp_windows(spec.inline_codegen_hook_windows, hook_name),
    );
}

/// The iRules event descriptors: what the surrounding event must provide, what
/// running the command raises, and the collect/release and handler facts.
///
/// Split out of [`command_options`] because that seeder crossed
/// `clippy::too_many_lines` when the emission fields were added, and this is
/// the group that comes out whole — every field here is one dialect's event
/// model, and `relations.rs` already clusters them together for the editor.
fn command_irules_events(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert(
        "event_requires".into(),
        lost.expr("event_requires", spec.event_requires.is_some()),
    );
    d.insert(
        "event_requirement_forms".into(),
        lost.expr(
            "event_requirement_forms",
            !spec.event_requirement_forms.is_empty(),
        ),
    );
    d.insert(
        "event_emits".into(),
        lost.expr("event_emits", spec.event_emits.is_some()),
    );
    d.insert(
        "event_emission_forms".into(),
        lost.expr(
            "event_emission_forms",
            !spec.event_emission_forms.is_empty(),
        ),
    );
    d.insert(
        "data_collection".into(),
        lost.expr("data_collection", spec.data_collection.is_some()),
    );
    d.insert(
        "irules_source_flow".into(),
        spec.irules_source_flow
            .map_or(Value::Null, |flow| json!(catalogue::variant_name(&flow))),
    );
    d.insert(
        "side_switch_target".into(),
        lost.expr("side_switch_target", spec.side_switch_target.is_some()),
    );
    d.insert(
        "event_handler_priority".into(),
        lost.expr(
            "event_handler_priority",
            spec.event_handler_priority.is_some(),
        ),
    );
    d.insert(
        "irules_top_level_effect".into(),
        lost.expr(
            "irules_top_level_effect",
            spec.irules_top_level_effect.is_some(),
        ),
    );
}

/// Options, enumerable values, and availability gating.
pub(super) fn command_options(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert("required_package".into(), opt_str(spec.required_package));
    d.insert(
        "tk_geometry".into(),
        spec.tk_geometry.map_or(Value::Null, |geometry| {
            use tcl_registry::tk_geometry::TkGeometryContainerPolicy;
            let container_policy = match geometry.container_policy {
                TkGeometryContainerPolicy::Exclusive => "Exclusive",
                TkGeometryContainerPolicy::Independent => "Independent",
            };
            json!({
                "container_policy": container_policy,
                "container_option": geometry.container_option,
                "direct_form": geometry.direct_form,
                "placement_subcommand": geometry.placement_subcommand,
                "release_subcommands": geometry.release_subcommands,
            })
        }),
    );
    d.insert("excluded_events".into(), str_list(spec.excluded_events));
    d.insert("unsafe_command".into(), json!(spec.unsafe_command));
    d.insert(
        "closed_value_args".into(),
        index_list(spec.closed_value_args),
    );
    d.insert(
        "arg_values_accept_prefix".into(),
        json!(spec.arg_values_accept_prefix),
    );
    command_irules_events(d, spec, lost);
    d.insert("options".into(), option_rows(spec.options, lost));
    let option_relations = if spec.option_relations.is_empty() {
        Value::Null
    } else {
        option_relations_expr(spec.option_relations)
            .map_or_else(|| lost.expr("option_relations", true), |expr| json!(expr))
    };
    d.insert("option_relations".into(), option_relations);
    let option_effect_families = if spec.option_effect_families.is_empty() {
        Value::Null
    } else {
        json!(option_effect_families_expr(spec.option_effect_families))
    };
    d.insert("option_effect_families".into(), option_effect_families);
    d.insert(
        "option_prefix_words".into(),
        json!(spec.option_prefix_words),
    );
    d.insert(
        "reserved_trailing_words".into(),
        json!(spec.reserved_trailing_words),
    );
    d.insert("arg_values".into(), arg_value_map(spec.arg_values, lost));
    let versioned_arg_values = if spec.versioned_arg_values.is_empty() {
        Value::Null
    } else {
        versioned_arg_values_expr(spec.versioned_arg_values).map_or_else(
            || lost.expr("versioned_arg_values", true),
            |expr| json!(expr),
        )
    };
    d.insert("versioned_arg_values".into(), versioned_arg_values);
    d.insert(
        "body_kind".into(),
        json!(catalogue::variant_name(&spec.body_kind)),
    );
    d.insert(
        "script_lookup_scope".into(),
        spec.script_lookup_scope
            .map_or(Value::Null, |scope| json!(catalogue::variant_name(&scope))),
    );
    d.insert(
        "body_execution".into(),
        lost.expr("body_execution", spec.body_execution.is_some()),
    );
    d.insert(
        "procedure_definition".into(),
        lost.expr("procedure_definition", spec.procedure_definition.is_some()),
    );
    d.insert(
        "native_compilation".into(),
        lost.expr("native_compilation", spec.native_compilation.is_some()),
    );
    d.insert(
        "successful_handler".into(),
        spec.successful_handler.map_or(Value::Null, |contract| {
            json!(format!(
                "Some(tcl_registry::native_compilation::SuccessfulHandlerSpec::{contract:?})"
            ))
        }),
    );
    d.insert(
        "body_interpreter".into(),
        body_interpreter(spec.body_interpreter),
    );
    d.insert(
        "body_arg_implicit_args".into(),
        json!(spec.body_arg_implicit_args),
    );
}

/// Taint and credential metadata.
pub(super) fn command_taint(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert("taint_output_sink".into(), opt_str(spec.taint_output_sink));
    d.insert(
        "taint_output_sink_subcommands".into(),
        str_list(spec.taint_output_sink_subcommands),
    );
    d.insert("taint_log_sink".into(), opt_str(spec.taint_log_sink));
    d.insert(
        "taint_network_sink_args".into(),
        spec.taint_network_sink_args.map_or(Value::Null, index_list),
    );
    d.insert(
        "taint_code_sink_args".into(),
        spec.taint_code_sink_args.map_or(Value::Null, index_list),
    );
    d.insert(
        "taint_numeric_coercion".into(),
        spec.taint_numeric_coercion
            .map_or(Value::Null, |c| json!(catalogue::variant_name(&c))),
    );
    d.insert(
        "taint_interp_eval_subcommands".into(),
        str_list(spec.taint_interp_eval_subcommands),
    );
    d.insert("taint_source".into(), taint(spec.taint_source));
    d.insert("taint_transform".into(), taint(spec.taint_transform));
    d.insert(
        "taint_transform_when".into(),
        spec.taint_transform_when
            .map_or(Value::Null, |c| json!(catalogue::variant_name(&c))),
    );
    d.insert(
        "taint_double_encode_colour".into(),
        taint(spec.taint_double_encode_colour),
    );
    d.insert(
        "taint_sink_safe_colour".into(),
        taint(spec.taint_sink_safe_colour),
    );
    d.insert(
        "taint_sink_gate".into(),
        lost.expr("taint_sink_gate", spec.taint_sink_gate.is_some()),
    );
    d.insert(
        "channel_configuration".into(),
        spec.channel_configuration
            .map_or(Value::Null, |descriptor| {
                json!(catalogue::variant_name(&descriptor))
            }),
    );
    d.insert(
        "credential_options".into(),
        str_list(spec.credential_options),
    );
    d.insert("sensitive_headers".into(), str_list(spec.sensitive_headers));
    d.insert(
        "setter_constraints".into(),
        Value::Array(
            spec.setter_constraints
                .iter()
                .map(setter_constraint)
                .collect(),
        ),
    );
}

/// Deprecation, translation, and the descriptor references.
fn command_manufacturer_methods(d: &mut Draft, spec: &CommandSpec) {
    d.insert(
        "manufacturer_methods".into(),
        Value::Array(
            spec.manufacturer_methods
                .iter()
                .map(manufacturer_method)
                .collect(),
        ),
    );
}

pub(super) fn command_advanced(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert(
        "pattern_type".into(),
        spec.pattern_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert(
        "pattern_arg_resolver".into(),
        lost.expr("pattern_arg_resolver", spec.pattern_arg_resolver.is_some()),
    );
    d.insert(
        "format_string_type".into(),
        spec.format_string_type
            .map_or(Value::Null, |t| json!(catalogue::variant_name(&t))),
    );
    d.insert("tcllib_package".into(), opt_str(spec.tcllib_package));
    if !insert_lifecycle(d, spec.lifecycle) {
        lost.note("deprecation_fix");
    }
    d.insert(
        "warn_missing_import".into(),
        json!(spec.warn_missing_import),
    );
    d.insert(
        "is_namespace_exported".into(),
        json!(spec.is_namespace_exported),
    );
    d.insert(
        "xc_translatable".into(),
        spec.xc_translatable.map_or(Value::Null, |b| json!(b)),
    );
    d.insert(
        "deprecated_replacement".into(),
        opt_str(spec.deprecated_replacement),
    );
    d.insert(
        "source_deprecation_advice".into(),
        spec.source_deprecation_advice
            .map_or(Value::Null, |advice| {
                json!(catalogue::variant_name(&advice))
            }),
    );
    d.insert(
        "deprecated_replacement_drop_in".into(),
        json!(spec.deprecated_replacement_drop_in),
    );
    d.insert("alias_of".into(), opt_str(spec.alias_of));
    d.insert(
        "runtime_backing".into(),
        json!(tcl_spectcl::BackingSyntax::from_backing(spec.runtime_backing).spelling()),
    );
    d.insert(
        "byte_array_payload".into(),
        spec.byte_array_payload
            .map_or(Value::Null, |p| json!(byte_payload_expr(p))),
    );
    d.insert(
        "byte_array_effect".into(),
        json!(catalogue::variant_name(&spec.byte_array_effect)),
    );
    d.insert(
        "definition_body".into(),
        definition_body(spec.definition_body),
    );
    command_manufacturer_methods(d, spec);
    d.insert(
        "case_list".into(),
        lost.expr("case_list", spec.case_list.is_some()),
    );
    d.insert(
        "oo_context_facts".into(),
        expr_or_null(!spec.oo_context_facts.is_empty(), || {
            oo_context_facts_expr(spec.oo_context_facts)
        }),
    );
    d.insert(
        "self_receiver_words".into(),
        str_list(spec.self_receiver_words),
    );
    command_object_facts(d, spec, lost);
}

/// The object / definition half of the advanced group: what a command
/// *defines* (a class, an outline symbol, a handle, a remote method) and the
/// context it defines it in.
///
/// Split out of [`command_advanced`] only to keep each within the hundred-line
/// budget; the two are one pass over one group.
pub(super) fn command_object_facts(d: &mut Draft, spec: &CommandSpec, lost: &mut Unrecovered) {
    d.insert("object_class".into(), object_class(spec.object_class));
    d.insert(
        "defines_symbol".into(),
        spec.defines_symbol
            .map_or(Value::Null, |s| json!(symbol_def_expr(s))),
    );
    d.insert(
        "body_scope".into(),
        lost.expr("body_scope", spec.body_scope.is_some()),
    );
    d.insert(
        "binds_handle".into(),
        spec.binds_handle
            .map_or(Value::Null, |b| json!(handle_binding_expr(b))),
    );
    d.insert(
        "remote_method".into(),
        spec.remote_method
            .map_or(Value::Null, |role| json!(remote_method_expr(*role))),
    );
    d.insert(
        "creates_instance_at".into(),
        opt_index(spec.creates_instance_at),
    );
    d.insert(
        "defines_command_at".into(),
        opt_index(spec.defines_command_at),
    );
    d.insert(
        "context_gate".into(),
        lost.expr("context_gate", spec.context_gate.is_some()),
    );
    d.insert(
        "implementation_namespace".into(),
        opt_str(spec.implementation_namespace),
    );
}
