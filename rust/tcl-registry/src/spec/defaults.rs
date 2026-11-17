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

//! The `DEFAULT` rows and stock presets every authored specification is spread from.

use super::CommandSpec;
use super::option_relations::OptionPlacement;
use super::subcommand::{SubCommand, SubSubCommand};
use crate::abbrev::PrefixMatching;
use crate::arity::Arity;
use crate::body_kind::{BodyInterpreter, BodyKind};
use crate::dispatch_stability::{DispatchDependencies, DispatchDependencyDescriptor};
use crate::lifecycle::Lifecycle;
use crate::state_transition::StateTransitionDescriptor;
use crate::traits::Traits;
use crate::types::VarWriteTyping;
use crate::value_transfer::SemanticsDeclaration;
use crate::world_effect::WorldEffectDescriptor;

impl CommandSpec {
    /// Default value for all fields — used with `..CommandSpec::DEFAULT`.
    pub const DEFAULT: Self = Self {
        name: "",
        pack_source_sandbox_helper: false,
        traits: Traits::empty(),
        surface: None,
        arity: Arity::any(),
        arity_windows: &[],
        arg_roles: &[],
        arg_role_resolver: None,
        arg_role_count_resolver: None,
        arg_role_layout_resolver: None,
        arg_role_resolver_roles: &[],
        clause_grammar: None,
        arg_presentation: &[],
        repeated_args: &[],
        frame_effect: None,
        clause_shape_check: None,
        command_prefixes: &[],
        command_prefix_resolver: None,
        script_timing_resolver: None,
        script_lookup_scope: None,
        callback_taint_inputs: &[],
        return_type: None,
        return_type_hook: None,
        var_write_typing: VarWriteTyping::ReturnValue,
        variable_write_min_args: None,
        return_elements: None,
        var_elements_effect: None,
        representation_effect: None,
        arg_types: &[],
        subcommands: &[],
        prefix_matching: PrefixMatching::Enabled,
        allow_unknown_subcommands: false,
        default_form_first_word: None,
        hover: None,
        documentation_examples: &[],
        forms: &[],
        command_forms: &[],
        semantic_operation: None,
        semantic_operation_windows: &[],
        completion: None,
        result_stability: None,
        native_result: None,
        assigns_variable_at: None,
        safe_on_uninit: None,
        const_fold: None,
        const_fold_versioned: None,
        lowering_hook: None,
        bpf_op: None,
        codegen_hook: None,
        codegen_hook_windows: &[],
        inline_codegen_hook: None,
        inline_codegen_hook_windows: &[],
        native_lowering: None,
        native_lowering_windows: &[],
        semantics: SemanticsDeclaration::Inherited,
        analyser_hook: None,
        command_table_effect: None,
        side_effects: crate::knowledge::Knowledge::Unknown,
        world_effects: crate::knowledge::Knowledge::Unknown,
        state_transitions: None,
        dispatch_dependencies: None,
        literal_argument_validator: None,
        source_index_bounds: None,
        inferred_storage_type: None,
        required_package: None,
        tk_geometry: None,
        excluded_events: &[],
        unsafe_command: false,
        closed_value_args: &[],
        arg_values_accept_prefix: false,
        event_requires: None,
        event_requirement_forms: &[],
        event_emits: None,
        event_emission_forms: &[],
        data_collection: None,
        irules_source_flow: None,
        side_switch_target: None,
        event_handler_priority: None,
        irules_top_level_effect: None,
        options: &[],
        option_prefix_words: 0,
        option_relations: &[],
        option_effect_families: &[],
        constraints: None,
        option_placement: OptionPlacement::Leading,
        reserved_trailing_words: 0,
        arg_values: &[],
        versioned_arg_values: &[],
        body_kind: BodyKind::Plain,
        body_execution: None,
        procedure_definition: None,
        native_compilation: None,
        successful_handler: None,
        variable_receivers: None,
        body_interpreter: BodyInterpreter::Current,
        body_arg_implicit_args: 0,
        taint_output_sink: None,
        taint_output_sink_subcommands: &[],
        taint_log_sink: None,
        taint_network_sink_args: None,
        taint_code_sink_args: None,
        taint_interp_eval_subcommands: &[],
        taint_source: None,
        taint_transform: None,
        taint_transform_when: None,
        taint_double_encode_colour: None,
        taint_sink_safe_colour: None,
        taint_sink_gate: None,
        taint_numeric_coercion: None,
        credential_options: &[],
        channel_configuration: None,
        sensitive_headers: &[],
        setter_constraints: &[],
        pattern_type: None,
        pattern_arg_resolver: None,
        format_string_type: None,
        tcllib_package: None,
        lifecycle: Lifecycle::UNSPECIFIED,
        warn_missing_import: true,
        is_namespace_exported: false,
        xc_translatable: None,
        deprecated_replacement: None,
        deprecated_replacement_drop_in: false,
        source_deprecation_advice: None,
        alias_of: None,
        runtime_backing: crate::runtime_backing::RuntimeBacking::None,
        byte_array_payload: None,
        byte_array_effect: crate::byte_array_effect::ByteArrayEffect::None,
        definition_body: None,
        manufacturer_methods: &[],
        case_list: None,
        object_class: None,
        defines_symbol: None,
        body_scope: None,
        binds_handle: None,
        remote_method: None,
        creates_instance_at: None,
        defines_command_at: None,
        teardown_argument_layout: None,
        context_gate: None,
        implementation_namespace: None,
        oo_context_facts: &[],
        self_receiver_words: &[],
    };

    /// The conservative fact for a command a native extension registers:
    /// every axis at its top, stated once in [`crate::extension_default`].
    ///
    /// Unknown arity; every argument may be a script or a variable name at any
    /// level (a dynamic barrier); unknown reads and writes; may establish a
    /// variable trace; a taint sink and source; unsafe and hidden in a safe
    /// interpreter; never pure; may complete with any code, a normal
    /// completion among them; and [`RuntimeBacking::HostNative`], so no
    /// procedure-binding check is ever emitted for it. It names no stamp and no
    /// window, so a call is dispatched plain at every release.
    ///
    /// It declares no `command_table_effect`, `state_transitions` or
    /// `world_effects` on purpose: a command with none resolves to
    /// `StateTransitionKnowledge::UnknownInvocation`, the wildcard over every
    /// identity domain that a closed statement could only narrow.
    #[must_use]
    pub const fn extension_default(name: &'static str) -> Self {
        Self {
            name,
            traits: crate::extension_default::TRAITS,
            side_effects: crate::knowledge::Knowledge::declared(
                crate::extension_default::SIDE_EFFECTS,
            ),
            completion: Some(crate::completion::CompletionDescriptor::CONSERVATIVE),
            runtime_backing: crate::runtime_backing::RuntimeBacking::HostNative,
            ..Self::DEFAULT
        }
    }

    /// Reusable base for a command whose successful result depends only on
    /// its evaluated arguments and which has no mutable-world effects or
    /// tracked state transitions.
    ///
    /// Command specs still declare their behavioural traits explicitly. This
    /// base only closes the four independent semantic proof obligations, so
    /// `PURE` and `CSE_CANDIDATE` cannot be acquired accidentally.
    ///
    /// The dispatch declaration replaces the conservative default with
    /// [`DispatchDependencies::NONE`], which
    /// [`crate::ResolvedDispatchDependencies::resolve`] restores to
    /// [`DispatchDependencies::BASE`]. That is the accurate claim for such a
    /// command: its dispatch depends only on the irreducible domains — the
    /// command binding, namespace lookup, command/execution traces, and
    /// interpreter policy. It is not an object method, and, being resolvable,
    /// it never reaches `unknown` fallback handling.
    pub const CLOSED_REFERENTIALLY_TRANSPARENT: Self = Self {
        result_stability: Some(crate::result_stability::ResultStability::ReferentiallyTransparent),
        world_effects: crate::knowledge::Knowledge::Partial(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(StateTransitionDescriptor::EMPTY),
        dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(
            DispatchDependencies::NONE,
        )),
        ..Self::DEFAULT
    };
}

impl SubSubCommand {
    /// Default value for all fields — used with `..SubSubCommand::DEFAULT`.
    pub const DEFAULT: Self = Self {
        native_compilation: None,
        name: "",
        detail: "",
        synopsis: "",
        surface: None,
        lifecycle: Lifecycle::UNSPECIFIED,
        options: None,
        option_prefix_words: 0,
    };
}

impl SubCommand {
    /// Default value for all fields.
    pub const DEFAULT: Self = Self {
        name: "",
        traits: Traits::empty(),
        arity: Arity::any(),
        arity_windows: &[],
        detail: "",
        synopsis: "",
        hover: None,
        arg_roles: &[],
        arg_role_resolver: None,
        arg_role_count_resolver: None,
        arg_role_layout_resolver: None,
        arg_role_resolver_roles: &[],
        clause_grammar: None,
        arg_presentation: &[],
        repeated_args: &[],
        command_prefixes: &[],
        command_prefix_resolver: None,
        script_timing_resolver: None,
        script_lookup_scope: None,
        callback_taint_inputs: &[],
        return_type: None,
        var_write_typing: VarWriteTyping::ReturnValue,
        return_elements: None,
        var_elements_effect: None,
        representation_effect: None,
        arg_types: &[],
        pure: false,
        mutator: false,
        const_fold: None,
        const_fold_versioned: None,
        lowering_hook: None,
        codegen_hook: None,
        codegen_hook_windows: &[],
        inline_codegen_hook: None,
        inline_codegen_hook_windows: &[],
        analyser_hook: None,
        semantics: SemanticsDeclaration::Inherited,
        command_table_effect: None,
        options: &[],
        option_prefix_words: 0,
        reserved_trailing_words: 0,
        option_relations: &[],
        option_effect_families: &[],
        constraints: None,
        option_placement: OptionPlacement::Leading,
        min_abbrev: None,
        prefix_matching: PrefixMatching::Enabled,
        arg_values: &[],
        versioned_arg_values: &[],
        subcommand_forms: &[],
        semantic_operation: None,
        semantic_operation_windows: &[],
        completion: None,
        result_stability: None,
        native_result: None,
        surface: None,
        lifecycle: Lifecycle::UNSPECIFIED,
        safe_on_uninit: None,
        loop_list_header: false,
        creates_scope_alias: false,
        inferred_storage_type: None,
        body_kind: BodyKind::Plain,
        body_execution: None,
        native_compilation: None,
        successful_handler: None,
        variable_receivers: None,
        body_interpreter: BodyInterpreter::Current,
        byte_array_effect: crate::byte_array_effect::ByteArrayEffect::None,
        closed_value_args: &[],
        arg_values_accept_prefix: false,
        body_arg_implicit_args: 0,
        taint_transform: None,
        taint_transform_when: None,
        taint_double_encode_colour: None,
        taint_output_sink: None,
        credential_arg: None,
        channel_configuration: None,
        sensitive_headers: &[],
        pattern_type: None,
        format_string_type: None,
        side_effects: crate::knowledge::Knowledge::Unknown,
        world_effects: crate::knowledge::Knowledge::Unknown,
        state_transitions: None,
        dispatch_dependencies: None,
        literal_argument_validator: None,
        source_index_bounds: None,
        destructive: false,
        returns_path: false,
        source_path_operation: None,
        source_package_query: None,
        source_configuration_reference: None,
        is_unescape: false,
        cfg_rewrite_name: None,
        sub_subcommands: &[],
        defines_command_at: None,
        max_leading_option_words: None,
    };

    /// Reusable base for a subcommand whose successful result depends only on
    /// its evaluated arguments and which has no mutable-world effects or
    /// tracked state transitions.
    ///
    /// As with [`CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT`], the dispatch
    /// declaration replaces the conservative default with
    /// [`DispatchDependencies::NONE`], which resolution restores to
    /// [`DispatchDependencies::BASE`] — the irreducible binding, namespace
    /// lookup, trace, and interpreter-policy domains. Such a subcommand is not
    /// an object method and, being resolvable, never reaches `unknown`
    /// fallback handling.
    pub const CLOSED_REFERENTIALLY_TRANSPARENT: Self = Self {
        result_stability: Some(crate::result_stability::ResultStability::ReferentiallyTransparent),
        world_effects: crate::knowledge::Knowledge::Partial(WorldEffectDescriptor::EMPTY),
        state_transitions: Some(StateTransitionDescriptor::EMPTY),
        dispatch_dependencies: Some(DispatchDependencyDescriptor::replace(
            DispatchDependencies::NONE,
        )),
        ..Self::DEFAULT
    };

    /// Reusable base for a subcommand whose result changes outside the tracked
    /// Tcl world, such as wall-clock or entropy observations.
    ///
    /// This closes only the result-dependency question. Effects and state
    /// transitions remain conservative unless the command spec independently
    /// declares them.
    pub const VOLATILE_RESULT: Self = Self {
        result_stability: Some(crate::result_stability::ResultStability::Volatile),
        ..Self::DEFAULT
    };
}
