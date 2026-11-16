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

//! The field-coverage gate: every `CommandSpec`-family field is surfaced in
//! the studio, or explicitly excluded.
//!
//! `CommandSpec` gains fields regularly — that is the registry's whole design
//! (`AGENTS.md`: "extend `CommandSpec` … rather than teaching a consumer about
//! the command by name").  A field the studio does not carry is **invisible in
//! the web UI**: an author cannot see or set it, and a spec drafted there
//! silently omits it.  Keeping the studio complete by hand is what let fields
//! drift out of it unnoticed, so this module makes it a build failure instead.
//!
//! ## How the gate works
//!
//! Each covered type has a pair:
//!
//! - a `witness_*` function holding an **exhaustive destructuring pattern**
//!   with no `..` rest — adding a field to the registry type fails to compile
//!   there, naming it — `error[E0027]: pattern does not mention field <name>` —
//!   and removing one fails too (`struct … has no field named …`);
//! - a `&[Field]` table saying, for every one of those fields, **where** the
//!   studio surfaces it — or why it is deliberately not surfaced.
//!
//! This is the field-level twin of [`crate::catalogue`]'s `covered_*`
//! witnesses, which use exhaustive `match`es so a new enum *variant* breaks the
//! build.
//!
//! ## What "surfaced" means
//!
//! The studio's four layers are chained, not independent:
//!
//! | layer | how a field reaches it |
//! |-------|------------------------|
//! | [`crate::schema`] | a [`crate::schema::FieldSchema`] entry keyed by the Rust field name |
//! | [`crate::draft`] | the seeder writes that key, so a drafted command carries the value |
//! | [`crate::render_rs`] | the renderer walks `schema::COMMAND_FIELDS`, so a schema key is emitted automatically |
//! | `tcl-spec-studio-wasm` | `schema()` serialises the same table and the front-end builds its form from it, so a schema key gets a browser editor automatically |
//!
//! So a [`Surface::Key`] entry whose key is in the schema **and** in the
//! default draft is carried by all four; the tests below check exactly that,
//! and render a real spec to prove the value survives the round trip.
//!
//! ## Adding a field to `CommandSpec` (or any type below)
//!
//! 1. Add a [`crate::schema::FieldSchema`] entry for it in `src/schema.rs`.
//! 2. Seed it in `src/draft.rs` so a drafted command carries it.
//! 3. Add it to this module's destructuring pattern **and** its `&[Field]`
//!    table.
//!
//! If the field genuinely should not be author-editable, skip 1–2 and record
//! it as [`Surface::Excluded`] with the reason — an excluded field is a stated
//! decision, not an oversight.

use tcl_registry::arity::Arity;
use tcl_registry::clause_grammar::{ClauseGrammarSpec, ClauseRow, ClauseSlot, DefaultClause};
use tcl_registry::definer::{
    BuiltinObjectMethod, DefinitionBodyGrammar, ManufacturerMethod, MemberBodyCommand,
    MemberOptionValue, MemberSpec, OptionalMemberArgument, SlotSpec, WrapperShift,
};
use tcl_registry::handle_binding::{HandleBindingSpec, HandleKeyword};
use tcl_registry::hooks::ArgTypeHint;
use tcl_registry::hover::{ArgValue, FormSpec, HoverSnippet, OptionArg, OptionSpec};
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::repeated::RepeatedArgLayout;
use tcl_registry::side_effects::SideEffect;
use tcl_registry::spec::{
    BytePayloadSpec, CaseListSpec, CommandSpec, ObjectClassSpec, OptionRelation, SubCommand,
    SubSubCommand, VersionedArgValue,
};
use tcl_registry::symbol_def::SymbolDef;
use tcl_registry::taint::SetterConstraint;

/// The four draft keys a [`Lifecycle`] is edited under.
///
/// One `Lifecycle` value, three independent releases and one quick-fix hook in the form — the
/// studio has always split it, and every surface (command, subcommand,
/// option) uses the same four keys.
pub const LIFECYCLE_KEYS: &[&str] = &[
    "introduced_version",
    "deprecated_version",
    "retired_version",
    "deprecation_fix",
];

/// Where the studio surfaces one registry field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// Edited under this draft key.
    ///
    /// For [`CommandSpec`] / [`SubCommand`] the key is also a
    /// [`crate::schema`] key, which is what gives it an editor in the browser
    /// and an emitted line in the rendered `.rs`.  For a nested type it is a
    /// key of the JSON object the draft builds for that type (an option row,
    /// a hover block, …), edited inside the parent field's row editor.
    Key(&'static str),
    /// Edited under several draft keys — a [`Lifecycle`]'s three releases.
    Keys(&'static [&'static str]),
    /// Rendered into the Rust expression the named draft key holds.
    ///
    /// The descriptor is plain data, so [`crate::draft`] writes it back out as
    /// a full struct literal and it round-trips; the field has no picker of its
    /// own because the whole descriptor is one `RustExpr` editor.
    Expression(&'static str),
    /// Deliberately not surfaced, with the reason.
    Excluded(&'static str),
}

/// One registry field and where the studio surfaces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    /// The Rust field name.
    pub name: &'static str,
    /// Where the studio carries it.
    pub surface: Surface,
}

const fn f(name: &'static str, surface: Surface) -> Field {
    Field { name, surface }
}

/// The reason a `&'static` descriptor reference is not editable field by field.
const NAMED_CONSTANT: &str = "the studio edits the whole descriptor as one Rust expression naming a shared \
     registry constant (`Some(&definer::SNIT_GRAMMAR)`); authoring a new one is an edit \
     to the registry module that owns it, not a form field";

// CommandSpec

/// Compile-time witness that [`COMMAND_SPEC`] accounts for every
/// [`CommandSpec`] field.
///
/// The pattern below has **no `..` rest**: adding a field to `CommandSpec`
/// fails to compile here with `error[E0027]: pattern does not mention field
/// <name>`.  When that happens, surface the field — a `FieldSchema` entry in
/// `src/schema.rs` plus a seeder line in `src/draft.rs` — and add it to the
/// pattern and to [`COMMAND_SPEC`].  If it should not be author-editable, add
/// it as [`Surface::Excluded`] with the reason instead.
///
/// `rustfmt` is held off this one item: `CommandSpec` has enough fields that
/// one pattern line each pushes the body past `clippy::pedantic`'s
/// hundred-line budget, and an exhaustive pattern is the whole point — so a
/// few closely-related fields share a line to keep the gate and the lint both
/// satisfied. Keep new fields one per line until the budget forces another
/// pairing.
#[rustfmt::skip]
#[allow(clippy::too_many_lines)]
pub fn witness_command_spec(spec: &CommandSpec) {
    // THE GATE.  Do NOT add `..` to silence this — that is the drift it exists
    // to catch.  A new field belongs in FOUR places: `schema::COMMAND_FIELDS`
    // (gives it a browser editor and a rendered `.rs` line), `draft.rs` (seeds
    // it from a live spec), this pattern, and `COMMAND_SPEC` below.  If it must
    // not be author-editable, add it here and to `COMMAND_SPEC` as
    // `Surface::Excluded("why")` and skip the first two.
    let CommandSpec {
        name: _,
        pack_source_sandbox_helper: _, traits: _,
        surface: _,
        arity: _, arity_windows: _,
        arg_roles: _,
        arg_role_resolver: _,
        arg_role_count_resolver: _,
        arg_role_layout_resolver: _,
        arg_role_resolver_roles: _,
        arg_presentation: _,
        repeated_args: _,
        frame_effect: _,
        clause_shape_check: _,
        command_prefixes: _,
        command_prefix_resolver: _,
        script_timing_resolver: _,
        script_lookup_scope: _,
        callback_taint_inputs: _,
        return_type: _,
        return_type_hook: _,
        var_write_typing: _,
        variable_write_min_args: _,
        return_elements: _,
        var_elements_effect: _,
        representation_effect: _,
        arg_types: _,
        subcommands: _,
        prefix_matching: _,
        option_prefix_words: _,
        allow_unknown_subcommands: _,
        default_form_first_word: _,
        hover: _,
        documentation_examples: _,
        forms: _,
        command_forms: _,
        semantic_operation: _,
        semantic_operation_windows: _,
        completion: _,
        assigns_variable_at: _,
        safe_on_uninit: _,
        const_fold: _,
        const_fold_versioned: _,
        lowering_hook: _,
        bpf_op: _,
        codegen_hook: _,
        codegen_hook_windows: _,
        inline_codegen_hook: _,
        inline_codegen_hook_windows: _,
        native_lowering: _,
        native_lowering_windows: _,
        semantics: _,
        analyser_hook: _,
        command_table_effect: _,
        side_effects: _,
        world_effects: _,
        state_transitions: _,
        dispatch_dependencies: _,
        result_stability: _,
        native_result: _,
        literal_argument_validator: _,
        source_index_bounds: _,
        inferred_storage_type: _,
        required_package: _, tk_geometry: _,
        excluded_events: _,
        unsafe_command: _,
        closed_value_args: _,
        arg_values_accept_prefix: _,
        event_requires: _,
        event_requirement_forms: _,
        event_emits: _, event_emission_forms: _,
        data_collection: _,
        irules_source_flow: _,
        side_switch_target: _,
        event_handler_priority: _,
        irules_top_level_effect: _,
        options: _,
        option_relations: _,
        option_effect_families: _,
        clause_grammar: _,
        constraints: _,
        option_placement: _,
        reserved_trailing_words: _,
        arg_values: _, versioned_arg_values: _,
        body_kind: _,
        body_interpreter: _, body_execution: _, procedure_definition: _, native_compilation: _, successful_handler: _, variable_receivers: _,
        body_arg_implicit_args: _,
        taint_output_sink: _,
        taint_output_sink_subcommands: _,
        taint_log_sink: _,
        taint_network_sink_args: _,
        taint_code_sink_args: _,
        taint_interp_eval_subcommands: _,
        taint_source: _,
        taint_transform: _, taint_transform_when: _, taint_double_encode_colour: _,
        taint_sink_safe_colour: _,
        taint_sink_gate: _,
        taint_numeric_coercion: _,
        credential_options: _,
        channel_configuration: _,
        sensitive_headers: _,
        setter_constraints: _,
        pattern_type: _,
        pattern_arg_resolver: _,
        format_string_type: _,
        tcllib_package: _,
        lifecycle: _,
        warn_missing_import: _,
        is_namespace_exported: _,
        xc_translatable: _,
        deprecated_replacement: _,
        deprecated_replacement_drop_in: _,
        source_deprecation_advice: _,
        alias_of: _,
        runtime_backing: _,
        byte_array_payload: _,
        byte_array_effect: _,
        definition_body: _,
        manufacturer_methods: _,
        case_list: _,
        object_class: _,
        defines_symbol: _,
        body_scope: _,
        binds_handle: _,
        remote_method: _,
        creates_instance_at: _,
        defines_command_at: _,
        teardown_argument_layout: _,
        context_gate: _,
        implementation_namespace: _,
        oo_context_facts: _,
        self_receiver_words: _,
    } = spec;
}

/// Where the studio surfaces each [`CommandSpec`] field, in
/// `CommandSpec::DEFAULT` order.
pub const COMMAND_SPEC: &[Field] = &[
    f("name", Surface::Key("name")),
    f(
        "pack_source_sandbox_helper",
        Surface::Excluded(
            "Stock engine provisioning capability; custom source descriptors cannot author host retention policy.",
        ),
    ),
    f("traits", Surface::Key("traits")),
    f("surface", Surface::Key("surface")),
    f("arity", Surface::Key("arity")),
    f("arity_windows", Surface::Key("arity_windows")),
    f("arg_roles", Surface::Key("arg_roles")),
    f("arg_role_resolver", Surface::Key("arg_role_resolver")),
    f(
        "arg_role_count_resolver",
        Surface::Key("arg_role_count_resolver"),
    ),
    f(
        "arg_role_layout_resolver",
        Surface::Key("arg_role_layout_resolver"),
    ),
    f(
        "arg_role_resolver_roles",
        Surface::Key("arg_role_resolver_roles"),
    ),
    f("arg_presentation", Surface::Key("arg_presentation")),
    f("repeated_args", Surface::Key("repeated_args")),
    f("frame_effect", Surface::Key("frame_effect")),
    f("clause_shape_check", Surface::Key("clause_shape_check")),
    f("command_prefixes", Surface::Key("command_prefixes")),
    f(
        "callback_taint_inputs",
        Surface::Key("callback_taint_inputs"),
    ),
    f(
        "command_prefix_resolver",
        Surface::Key("command_prefix_resolver"),
    ),
    f(
        "script_timing_resolver",
        Surface::Key("script_timing_resolver"),
    ),
    f("script_lookup_scope", Surface::Key("script_lookup_scope")),
    f("return_type", Surface::Key("return_type")),
    f("return_type_hook", Surface::Key("return_type_hook")),
    f("var_write_typing", Surface::Key("var_write_typing")),
    f(
        "variable_write_min_args",
        Surface::Key("variable_write_min_args"),
    ),
    f("return_elements", Surface::Key("return_elements")),
    f("var_elements_effect", Surface::Key("var_elements_effect")),
    f(
        "representation_effect",
        Surface::Key("representation_effect"),
    ),
    f("arg_types", Surface::Key("arg_types")),
    f("subcommands", Surface::Key("subcommands")),
    f("prefix_matching", Surface::Key("prefix_matching")),
    f("option_prefix_words", Surface::Key("option_prefix_words")),
    f(
        "allow_unknown_subcommands",
        Surface::Key("allow_unknown_subcommands"),
    ),
    f(
        "default_form_first_word",
        Surface::Key("default_form_first_word"),
    ),
    f("hover", Surface::Key("hover")),
    f(
        "documentation_examples",
        Surface::Key("documentation_examples"),
    ),
    f("forms", Surface::Key("forms")),
    f("command_forms", Surface::Key("command_forms")),
    f("semantic_operation", Surface::Key("semantic_operation")),
    f(
        "semantic_operation_windows",
        Surface::Key("semantic_operation_windows"),
    ),
    f("completion", Surface::Key("completion")),
    f("assigns_variable_at", Surface::Key("assigns_variable_at")),
    f("safe_on_uninit", Surface::Key("safe_on_uninit")),
    f("const_fold", Surface::Key("const_fold")),
    f("const_fold_versioned", Surface::Key("const_fold_versioned")),
    f("lowering_hook", Surface::Key("lowering_hook")),
    f("bpf_op", Surface::Key("bpf_op")),
    f("codegen_hook", Surface::Key("codegen_hook")),
    f("codegen_hook_windows", Surface::Key("codegen_hook_windows")),
    f("inline_codegen_hook", Surface::Key("inline_codegen_hook")),
    f(
        "inline_codegen_hook_windows",
        Surface::Key("inline_codegen_hook_windows"),
    ),
    f("native_lowering", Surface::Key("native_lowering")),
    f(
        "native_lowering_windows",
        Surface::Key("native_lowering_windows"),
    ),
    f("semantics", Surface::Key("semantics")),
    f("analyser_hook", Surface::Key("analyser_hook")),
    f("command_table_effect", Surface::Key("command_table_effect")),
    f(
        "side_effects",
        Surface::Keys(&["side_effects", "side_effects_knowledge"]),
    ),
    f("world_effects", Surface::Key("world_effects")),
    f("state_transitions", Surface::Key("state_transitions")),
    f(
        "dispatch_dependencies",
        Surface::Key("dispatch_dependencies"),
    ),
    f("result_stability", Surface::Key("result_stability")),
    f("native_result", Surface::Key("native_result")),
    f(
        "literal_argument_validator",
        Surface::Key("literal_argument_validator"),
    ),
    f("source_index_bounds", Surface::Key("source_index_bounds")),
    f(
        "inferred_storage_type",
        Surface::Key("inferred_storage_type"),
    ),
    f("required_package", Surface::Key("required_package")),
    f("tk_geometry", Surface::Key("tk_geometry")),
    f("excluded_events", Surface::Key("excluded_events")),
    f("unsafe_command", Surface::Key("unsafe_command")),
    f("closed_value_args", Surface::Key("closed_value_args")),
    f(
        "arg_values_accept_prefix",
        Surface::Key("arg_values_accept_prefix"),
    ),
    f("event_requires", Surface::Key("event_requires")),
    f(
        "event_requirement_forms",
        Surface::Key("event_requirement_forms"),
    ),
    f("event_emits", Surface::Key("event_emits")),
    f("event_emission_forms", Surface::Key("event_emission_forms")),
    f("data_collection", Surface::Key("data_collection")),
    f("irules_source_flow", Surface::Key("irules_source_flow")),
    f("side_switch_target", Surface::Key("side_switch_target")),
    f(
        "event_handler_priority",
        Surface::Key("event_handler_priority"),
    ),
    f(
        "irules_top_level_effect",
        Surface::Key("irules_top_level_effect"),
    ),
    f("options", Surface::Key("options")),
    f("option_relations", Surface::Key("option_relations")),
    f("constraints", Surface::Key("constraints")),
    f("option_placement", Surface::Key("option_placement")),
    f(
        "reserved_trailing_words",
        Surface::Key("reserved_trailing_words"),
    ),
    f("arg_values", Surface::Key("arg_values")),
    f("versioned_arg_values", Surface::Key("versioned_arg_values")),
    f("body_kind", Surface::Key("body_kind")),
    f("body_interpreter", Surface::Key("body_interpreter")),
    f("body_execution", Surface::Key("body_execution")),
    f("procedure_definition", Surface::Key("procedure_definition")),
    f("native_compilation", Surface::Key("native_compilation")),
    f("successful_handler", Surface::Key("successful_handler")),
    f("variable_receivers", Surface::Key("variable_receivers")),
    f(
        "body_arg_implicit_args",
        Surface::Key("body_arg_implicit_args"),
    ),
    f("taint_output_sink", Surface::Key("taint_output_sink")),
    f(
        "taint_output_sink_subcommands",
        Surface::Key("taint_output_sink_subcommands"),
    ),
    f("taint_log_sink", Surface::Key("taint_log_sink")),
    f(
        "taint_network_sink_args",
        Surface::Key("taint_network_sink_args"),
    ),
    f("taint_code_sink_args", Surface::Key("taint_code_sink_args")),
    f(
        "taint_numeric_coercion",
        Surface::Key("taint_numeric_coercion"),
    ),
    f(
        "taint_interp_eval_subcommands",
        Surface::Key("taint_interp_eval_subcommands"),
    ),
    f("taint_source", Surface::Key("taint_source")),
    f("taint_transform", Surface::Key("taint_transform")),
    f("taint_transform_when", Surface::Key("taint_transform_when")),
    f(
        "taint_double_encode_colour",
        Surface::Key("taint_double_encode_colour"),
    ),
    f(
        "taint_sink_safe_colour",
        Surface::Key("taint_sink_safe_colour"),
    ),
    f("taint_sink_gate", Surface::Key("taint_sink_gate")),
    f(
        "channel_configuration",
        Surface::Key("channel_configuration"),
    ),
    f("credential_options", Surface::Key("credential_options")),
    f("sensitive_headers", Surface::Key("sensitive_headers")),
    f("setter_constraints", Surface::Key("setter_constraints")),
    f("pattern_type", Surface::Key("pattern_type")),
    f("pattern_arg_resolver", Surface::Key("pattern_arg_resolver")),
    f(
        "option_effect_families",
        Surface::Key("option_effect_families"),
    ),
    f("format_string_type", Surface::Key("format_string_type")),
    f("tcllib_package", Surface::Key("tcllib_package")),
    f("lifecycle", Surface::Keys(LIFECYCLE_KEYS)),
    f("warn_missing_import", Surface::Key("warn_missing_import")),
    f(
        "is_namespace_exported",
        Surface::Key("is_namespace_exported"),
    ),
    f("xc_translatable", Surface::Key("xc_translatable")),
    f(
        "deprecated_replacement",
        Surface::Key("deprecated_replacement"),
    ),
    f(
        "deprecated_replacement_drop_in",
        Surface::Key("deprecated_replacement_drop_in"),
    ),
    f(
        "source_deprecation_advice",
        Surface::Key("source_deprecation_advice"),
    ),
    f("alias_of", Surface::Key("alias_of")),
    f("runtime_backing", Surface::Key("runtime_backing")),
    f("byte_array_payload", Surface::Key("byte_array_payload")),
    f("byte_array_effect", Surface::Key("byte_array_effect")),
    f("definition_body", Surface::Key("definition_body")),
    f("manufacturer_methods", Surface::Key("manufacturer_methods")),
    f("case_list", Surface::Key("case_list")),
    f("clause_grammar", Surface::Key("clause_grammar")),
    f("object_class", Surface::Key("object_class")),
    f("defines_symbol", Surface::Key("defines_symbol")),
    f("body_scope", Surface::Key("body_scope")),
    f("binds_handle", Surface::Key("binds_handle")),
    f("remote_method", Surface::Key("remote_method")),
    f("creates_instance_at", Surface::Key("creates_instance_at")),
    f("defines_command_at", Surface::Key("defines_command_at")),
    f(
        "teardown_argument_layout",
        Surface::Excluded(
            "Registry-only effective argv layout derived from a selected teardown command; SpecTcl has no authored source layout",
        ),
    ),
    f("context_gate", Surface::Key("context_gate")),
    f(
        "implementation_namespace",
        Surface::Key("implementation_namespace"),
    ),
    f("oo_context_facts", Surface::Key("oo_context_facts")),
    f("self_receiver_words", Surface::Key("self_receiver_words")),
];

// SubCommand

/// Compile-time witness that [`SUB_COMMAND`] accounts for every
/// [`SubCommand`] field — see [`witness_command_spec`] for what to do when it
/// stops compiling.
pub fn witness_sub_command(sub: &SubCommand) {
    // THE GATE — see `witness_command_spec`. No `..`, ever.
    let SubCommand {
        name: _,
        traits: _,
        arity: _,
        arity_windows: _,
        detail: _,
        synopsis: _,
        hover: _,
        arg_roles: _,
        arg_role_resolver: _,
        arg_role_count_resolver: _,
        arg_role_layout_resolver: _,
        arg_role_resolver_roles: _,
        arg_presentation: _,
        repeated_args: _,
        command_prefixes: _,
        command_prefix_resolver: _,
        script_timing_resolver: _,
        script_lookup_scope: _,
        callback_taint_inputs: _,
        return_type: _,
        var_write_typing: _,
        return_elements: _,
        var_elements_effect: _,
        representation_effect: _,
        arg_types: _,
        pure: _,
        mutator: _,
        const_fold: _,
        const_fold_versioned: _,
        lowering_hook: _,
        codegen_hook: _,
        codegen_hook_windows: _,
        inline_codegen_hook: _,
        inline_codegen_hook_windows: _,
        semantics: _,
        analyser_hook: _,
        command_table_effect: _,
        options: _,
        option_relations: _,
        option_effect_families: _,
        clause_grammar: _,
        constraints: _,
        option_placement: _,
        min_abbrev: _,
        prefix_matching: _,
        option_prefix_words: _,
        reserved_trailing_words: _,
        arg_values: _,
        versioned_arg_values: _,
        subcommand_forms: _,
        semantic_operation: _,
        semantic_operation_windows: _,
        completion: _,
        surface: _,
        lifecycle: _,
        safe_on_uninit: _,
        loop_list_header: _,
        creates_scope_alias: _,
        inferred_storage_type: _,
        body_kind: _,
        body_interpreter: _,
        body_execution: _,
        native_compilation: _,
        successful_handler: _,
        variable_receivers: _,
        byte_array_effect: _,
        closed_value_args: _,
        arg_values_accept_prefix: _,
        body_arg_implicit_args: _,
        taint_transform: _,
        taint_transform_when: _,
        taint_double_encode_colour: _,
        taint_output_sink: _,
        credential_arg: _,
        channel_configuration: _,
        sensitive_headers: _,
        pattern_type: _,
        format_string_type: _,
        side_effects: _,
        world_effects: _,
        state_transitions: _,
        dispatch_dependencies: _,
        result_stability: _,
        native_result: _,
        literal_argument_validator: _,
        source_index_bounds: _,
        destructive: _,
        returns_path: _,
        source_path_operation: _,
        source_package_query: _,
        source_configuration_reference: _,
        is_unescape: _,
        cfg_rewrite_name: _,
        sub_subcommands: _,
        defines_command_at: _,
        max_leading_option_words: _,
    } = sub;
}

/// Where the studio surfaces each [`SubCommand`] field, in
/// `SubCommand::DEFAULT` order.
pub const SUB_COMMAND: &[Field] = &[
    f("name", Surface::Key("name")),
    f("traits", Surface::Key("traits")),
    f("arity", Surface::Key("arity")),
    f("arity_windows", Surface::Key("arity_windows")),
    f("detail", Surface::Key("detail")),
    f("synopsis", Surface::Key("synopsis")),
    f("hover", Surface::Key("hover")),
    f("arg_roles", Surface::Key("arg_roles")),
    f("arg_role_resolver", Surface::Key("arg_role_resolver")),
    f(
        "arg_role_count_resolver",
        Surface::Key("arg_role_count_resolver"),
    ),
    f(
        "arg_role_layout_resolver",
        Surface::Key("arg_role_layout_resolver"),
    ),
    f(
        "arg_role_resolver_roles",
        Surface::Key("arg_role_resolver_roles"),
    ),
    f("arg_presentation", Surface::Key("arg_presentation")),
    f("repeated_args", Surface::Key("repeated_args")),
    f("command_prefixes", Surface::Key("command_prefixes")),
    f(
        "callback_taint_inputs",
        Surface::Key("callback_taint_inputs"),
    ),
    f(
        "command_prefix_resolver",
        Surface::Key("command_prefix_resolver"),
    ),
    f(
        "script_timing_resolver",
        Surface::Key("script_timing_resolver"),
    ),
    f("script_lookup_scope", Surface::Key("script_lookup_scope")),
    f("return_type", Surface::Key("return_type")),
    f("var_write_typing", Surface::Key("var_write_typing")),
    f("return_elements", Surface::Key("return_elements")),
    f("var_elements_effect", Surface::Key("var_elements_effect")),
    f(
        "representation_effect",
        Surface::Key("representation_effect"),
    ),
    f("arg_types", Surface::Key("arg_types")),
    f("pure", Surface::Key("pure")),
    f("mutator", Surface::Key("mutator")),
    f("const_fold", Surface::Key("const_fold")),
    f("const_fold_versioned", Surface::Key("const_fold_versioned")),
    f("lowering_hook", Surface::Key("lowering_hook")),
    f("codegen_hook", Surface::Key("codegen_hook")),
    f("codegen_hook_windows", Surface::Key("codegen_hook_windows")),
    f("inline_codegen_hook", Surface::Key("inline_codegen_hook")),
    f(
        "inline_codegen_hook_windows",
        Surface::Key("inline_codegen_hook_windows"),
    ),
    f("semantics", Surface::Key("semantics")),
    f("analyser_hook", Surface::Key("analyser_hook")),
    f("command_table_effect", Surface::Key("command_table_effect")),
    f("options", Surface::Key("options")),
    f("option_relations", Surface::Key("option_relations")),
    f(
        "option_effect_families",
        Surface::Key("option_effect_families"),
    ),
    f("clause_grammar", Surface::Key("clause_grammar")),
    f("constraints", Surface::Key("constraints")),
    f("option_placement", Surface::Key("option_placement")),
    f("min_abbrev", Surface::Key("min_abbrev")),
    f("prefix_matching", Surface::Key("prefix_matching")),
    f("option_prefix_words", Surface::Key("option_prefix_words")),
    f(
        "reserved_trailing_words",
        Surface::Key("reserved_trailing_words"),
    ),
    f("arg_values", Surface::Key("arg_values")),
    f("versioned_arg_values", Surface::Key("versioned_arg_values")),
    f("subcommand_forms", Surface::Key("subcommand_forms")),
    f("semantic_operation", Surface::Key("semantic_operation")),
    f(
        "semantic_operation_windows",
        Surface::Key("semantic_operation_windows"),
    ),
    f("completion", Surface::Key("completion")),
    f("surface", Surface::Key("surface")),
    f("lifecycle", Surface::Keys(LIFECYCLE_KEYS)),
    f("safe_on_uninit", Surface::Key("safe_on_uninit")),
    f("loop_list_header", Surface::Key("loop_list_header")),
    f("creates_scope_alias", Surface::Key("creates_scope_alias")),
    f(
        "inferred_storage_type",
        Surface::Key("inferred_storage_type"),
    ),
    f("body_kind", Surface::Key("body_kind")),
    f("body_interpreter", Surface::Key("body_interpreter")),
    f("body_execution", Surface::Key("body_execution")),
    f("native_compilation", Surface::Key("native_compilation")),
    f("successful_handler", Surface::Key("successful_handler")),
    f("variable_receivers", Surface::Key("variable_receivers")),
    f("byte_array_effect", Surface::Key("byte_array_effect")),
    f("closed_value_args", Surface::Key("closed_value_args")),
    f(
        "arg_values_accept_prefix",
        Surface::Key("arg_values_accept_prefix"),
    ),
    f(
        "body_arg_implicit_args",
        Surface::Key("body_arg_implicit_args"),
    ),
    f("taint_transform", Surface::Key("taint_transform")),
    f("taint_transform_when", Surface::Key("taint_transform_when")),
    f(
        "taint_double_encode_colour",
        Surface::Key("taint_double_encode_colour"),
    ),
    f("taint_output_sink", Surface::Key("taint_output_sink")),
    f(
        "channel_configuration",
        Surface::Key("channel_configuration"),
    ),
    f("credential_arg", Surface::Key("credential_arg")),
    f("sensitive_headers", Surface::Key("sensitive_headers")),
    f("pattern_type", Surface::Key("pattern_type")),
    f("format_string_type", Surface::Key("format_string_type")),
    f(
        "side_effects",
        Surface::Keys(&["side_effects", "side_effects_knowledge"]),
    ),
    f("world_effects", Surface::Key("world_effects")),
    f("state_transitions", Surface::Key("state_transitions")),
    f(
        "dispatch_dependencies",
        Surface::Key("dispatch_dependencies"),
    ),
    f("result_stability", Surface::Key("result_stability")),
    f("native_result", Surface::Key("native_result")),
    f(
        "literal_argument_validator",
        Surface::Key("literal_argument_validator"),
    ),
    f("source_index_bounds", Surface::Key("source_index_bounds")),
    f("destructive", Surface::Key("destructive")),
    f("returns_path", Surface::Key("returns_path")),
    f(
        "source_package_query",
        Surface::Excluded(
            "Builtin conditional target-survey algebra; custom packs have no provider/subject receipt",
        ),
    ),
    f(
        "source_path_operation",
        Surface::Key("source_path_operation"),
    ),
    f(
        "source_configuration_reference",
        Surface::Key("source_configuration_reference"),
    ),
    f("is_unescape", Surface::Key("is_unescape")),
    f("cfg_rewrite_name", Surface::Key("cfg_rewrite_name")),
    f("sub_subcommands", Surface::Key("sub_subcommands")),
    f("defines_command_at", Surface::Key("defines_command_at")),
    f(
        "max_leading_option_words",
        Surface::Key("max_leading_option_words"),
    ),
];

// The nested types a draft carries structurally.
//
// These have no `FieldSchema` entry of their own: they are edited inside the
// row editor of the parent field that holds them (`options`, `hover`,
// `side_effects`, …), so their draft keys are keys of the JSON object
// `crate::draft` builds for the type.  Same gate, same rule — a new field on
// any of them fails to compile until it is carried or excluded.

/// Compile-time witness for [`SUB_SUB_COMMAND`].
pub fn witness_sub_sub_command(sub: &SubSubCommand) {
    let SubSubCommand {
        native_compilation: _,
        name: _,
        detail: _,
        synopsis: _,
        surface: _,
        lifecycle: _,
        options: _,
        option_prefix_words: _,
    } = sub;
}

/// Where the studio surfaces each [`SubSubCommand`] field.
pub const SUB_SUB_COMMAND: &[Field] = &[
    f(
        "native_compilation",
        Surface::Excluded(
            "Nested native compiler registrations require actual interpreter and worker identity proofs; pack authoring cannot supply them.",
        ),
    ),
    f("name", Surface::Key("name")),
    f("detail", Surface::Key("detail")),
    f("synopsis", Surface::Key("synopsis")),
    f("surface", Surface::Key("surface")),
    f("lifecycle", Surface::Keys(LIFECYCLE_KEYS)),
    f("options", Surface::Key("options")),
    f("option_prefix_words", Surface::Key("option_prefix_words")),
];

/// Compile-time witness for [`OPTION_SPEC`].
pub fn witness_option_spec(opt: &OptionSpec) {
    let OptionSpec {
        name: _,
        value: _,
        detail: _,
        surface: _,
        aliases: _,
        lifecycle: _,
        min_abbrev: _,
        effect: _,
    } = opt;
}

/// Where the studio surfaces each [`OptionSpec`] field.
pub const OPTION_SPEC: &[Field] = &[
    f("name", Surface::Key("name")),
    f("value", Surface::Key("value")),
    f("detail", Surface::Key("detail")),
    f("surface", Surface::Key("surface")),
    f("aliases", Surface::Key("aliases")),
    f("lifecycle", Surface::Keys(LIFECYCLE_KEYS)),
    f("min_abbrev", Surface::Key("min_abbrev")),
    f("effect", Surface::Key("effect")),
];

/// Compile-time witness for [`OPTION_ARG`].
pub fn witness_option_arg(arg: &OptionArg) {
    let OptionArg {
        arity: _,
        role: _,
        also_role: _,
        body_kind: _,
        script_timing: _,
        callback_taint_inputs: _,
        values: _,
        closed: _,
        integer: _,
        hint: _,
        appended_arity: _,
        taints_var_write: _,
        variable_scope: _,
    } = arg;
}

/// Where the studio surfaces each [`OptionArg`] field — the keys of the
/// `value` object inside an option row.
pub const OPTION_ARG: &[Field] = &[
    f("arity", Surface::Key("arity")),
    f("role", Surface::Key("role")),
    f("also_role", Surface::Key("also_role")),
    f("body_kind", Surface::Key("body_kind")),
    f("script_timing", Surface::Key("script_timing")),
    f(
        "callback_taint_inputs",
        Surface::Key("callback_taint_inputs"),
    ),
    f("values", Surface::Key("values")),
    f("closed", Surface::Key("closed")),
    f("integer", Surface::Key("integer")),
    f("hint", Surface::Key("hint")),
    f("appended_arity", Surface::Key("appended_arity")),
    f("taints_var_write", Surface::Key("taints_var_write")),
    f("variable_scope", Surface::Key("variable_scope")),
];

/// Compile-time witness for [`ARG_VALUE`].
pub fn witness_arg_value(value: &ArgValue) {
    let ArgValue {
        value: _,
        detail: _,
        min_tcl: _,
        lifecycle: _,
        code: _,
    } = value;
}

/// Where the studio surfaces each [`ArgValue`] field.
pub const ARG_VALUE: &[Field] = &[
    f("value", Surface::Key("value")),
    f("detail", Surface::Key("detail")),
    f("min_tcl", Surface::Key("min_tcl")),
    f("lifecycle", Surface::Keys(LIFECYCLE_KEYS)),
    f("code", Surface::Key("code")),
];

/// Compile-time witness for [`FORM_SPEC`].
pub fn witness_form_spec(form: &FormSpec) {
    let FormSpec {
        kind: _,
        synopsis: _,
        surface: _,
        lifecycle: _,
    } = form;
}

/// Where the studio surfaces each [`FormSpec`] field.
pub const FORM_SPEC: &[Field] = &[
    f("kind", Surface::Key("kind")),
    f("synopsis", Surface::Key("synopsis")),
    f("surface", Surface::Key("surface")),
    f("lifecycle", Surface::Keys(LIFECYCLE_KEYS)),
];

/// Compile-time witness for [`HOVER_SNIPPET`].
pub fn witness_hover_snippet(hover: &HoverSnippet) {
    let HoverSnippet {
        summary: _,
        synopsis: _,
        snippet: _,
        source: _,
        examples: _,
        return_value: _,
    } = hover;
}

/// Where the studio surfaces each [`HoverSnippet`] field.
pub const HOVER_SNIPPET: &[Field] = &[
    f("summary", Surface::Key("summary")),
    f("synopsis", Surface::Key("synopsis")),
    f("snippet", Surface::Key("snippet")),
    f("source", Surface::Key("source")),
    f("examples", Surface::Key("examples")),
    f("return_value", Surface::Key("return_value")),
];

/// Compile-time witness for [`SIDE_EFFECT`].
pub fn witness_side_effect(effect: &SideEffect) {
    let SideEffect {
        target: _,
        reads: _,
        writes: _,
        connection_side: _,
        surface: _,
        lifecycle: _,
    } = effect;
}

/// Where the studio surfaces each [`SideEffect`] field.
pub const SIDE_EFFECT: &[Field] = &[
    f("target", Surface::Key("target")),
    f("reads", Surface::Key("reads")),
    f("writes", Surface::Key("writes")),
    f("connection_side", Surface::Key("connection_side")),
    f("surface", Surface::Key("surface")),
    f("lifecycle", Surface::Keys(LIFECYCLE_KEYS)),
];

/// Compile-time witness for [`SETTER_CONSTRAINT`].
pub fn witness_setter_constraint(constraint: &SetterConstraint) {
    let SetterConstraint {
        arg_index: _,
        required_prefix: _,
        code: _,
        message: _,
    } = constraint;
}

/// Where the studio surfaces each [`SetterConstraint`] field.
pub const SETTER_CONSTRAINT: &[Field] = &[
    f("arg_index", Surface::Key("arg_index")),
    f("required_prefix", Surface::Key("required_prefix")),
    f("code", Surface::Key("code")),
    f("message", Surface::Key("message")),
];

/// Compile-time witness for [`ARITY`].
pub fn witness_arity(arity: &Arity) {
    let Arity {
        min: _,
        count: _,
        max: _,
        step: _,
        also_exact: _,
    } = arity;
}

/// Where the studio surfaces each [`Arity`] field.
pub const ARITY: &[Field] = &[
    f("min", Surface::Key("min")),
    f("count", Surface::Key("count")),
    f("max", Surface::Key("max")),
    f("step", Surface::Key("step")),
    f("also_exact", Surface::Key("also_exact")),
];

/// Compile-time witness for [`ARG_TYPE_HINT`].
pub fn witness_arg_type_hint(hint: &ArgTypeHint) {
    let ArgTypeHint {
        expected: _,
        shimmers: _,
        transparent_from: _,
    } = hint;
}

/// Where the studio surfaces each [`ArgTypeHint`] field.
pub const ARG_TYPE_HINT: &[Field] = &[
    f("expected", Surface::Key("expected")),
    f("shimmers", Surface::Key("shimmers")),
    f("transparent_from", Surface::Key("transparent_from")),
];

/// Compile-time witness for [`LIFECYCLE`].
pub fn witness_lifecycle(lifecycle: &Lifecycle) {
    let Lifecycle {
        introduced: _,
        deprecated: _,
        retired: _,
        deprecation_fix: _,
    } = lifecycle;
}

/// Where the studio surfaces each [`Lifecycle`] field.
pub const LIFECYCLE: &[Field] = &[
    f("introduced", Surface::Key("introduced_version")),
    f("deprecated", Surface::Key("deprecated_version")),
    f("retired", Surface::Key("retired_version")),
    f("deprecation_fix", Surface::Key("deprecation_fix")),
];

// Plain-data descriptors.
//
// Each of these is pure data, so `crate::draft` renders it back out as a full
// struct literal and it round-trips. `Surface::Expression` names the field
// that carries it, and
// `descriptor_literals_name_every_field` proves the rendered literal really
// mentions each one.

/// Compile-time witness for [`REPEATED_ARG_LAYOUT`].
pub fn witness_repeated_arg_layout(layout: &RepeatedArgLayout) {
    let RepeatedArgLayout {
        role: _,
        start: _,
        stride: _,
        exclude_trailing: _,
        optional_leading_word: _,
        conditional_binding: _,
    } = layout;
}

/// Where the studio surfaces each [`RepeatedArgLayout`] field.
pub const REPEATED_ARG_LAYOUT: &[Field] = &[
    f("role", Surface::Expression("repeated_args")),
    f("start", Surface::Expression("repeated_args")),
    f("stride", Surface::Expression("repeated_args")),
    f("exclude_trailing", Surface::Expression("repeated_args")),
    f(
        "optional_leading_word",
        Surface::Expression("repeated_args"),
    ),
    f("conditional_binding", Surface::Expression("repeated_args")),
];

/// Compile-time witness for [`HANDLE_BINDING_SPEC`].
pub fn witness_handle_binding_spec(spec: &HandleBindingSpec) {
    let HandleBindingSpec {
        name_from: _,
        class_from: _,
        keyword: _,
    } = spec;
}

/// Where the studio surfaces each [`HandleBindingSpec`] field.
pub const HANDLE_BINDING_SPEC: &[Field] = &[
    f("name_from", Surface::Expression("binds_handle")),
    f("class_from", Surface::Expression("binds_handle")),
    f("keyword", Surface::Expression("binds_handle")),
];

/// Compile-time witness for [`HANDLE_KEYWORD`].
pub fn witness_handle_keyword(keyword: &HandleKeyword) {
    let HandleKeyword { at: _, word: _ } = keyword;
}

/// Where the studio surfaces each [`HandleKeyword`] field.
pub const HANDLE_KEYWORD: &[Field] = &[
    f("at", Surface::Expression("binds_handle")),
    f("word", Surface::Expression("binds_handle")),
];

/// Compile-time witness for [`SYMBOL_DEF`].
pub fn witness_symbol_def(def: &SymbolDef) {
    let SymbolDef {
        name_arg: _,
        detail_arg: _,
        requires_arg: _,
        kind: _,
    } = def;
}

/// Where the studio surfaces each [`SymbolDef`] field.
pub const SYMBOL_DEF: &[Field] = &[
    f("name_arg", Surface::Expression("defines_symbol")),
    f("detail_arg", Surface::Expression("defines_symbol")),
    f("requires_arg", Surface::Expression("defines_symbol")),
    f("kind", Surface::Expression("defines_symbol")),
];

/// Compile-time witness for [`BYTE_PAYLOAD_SPEC`].
pub fn witness_byte_payload_spec(spec: &BytePayloadSpec) {
    let BytePayloadSpec {
        replace_data_index: _,
        message_flag_shift: _,
    } = spec;
}

/// Where the studio surfaces each [`BytePayloadSpec`] field.
pub const BYTE_PAYLOAD_SPEC: &[Field] = &[
    f(
        "replace_data_index",
        Surface::Expression("byte_array_payload"),
    ),
    f(
        "message_flag_shift",
        Surface::Expression("byte_array_payload"),
    ),
];

/// Compile-time witness for [`OPTION_CONSTRAINT`].
pub fn witness_option_constraint(constraint: &OptionRelation) {
    let OptionRelation {
        kind: _,
        mode: _,
        subject: _,
        terms: _,
        surface: _,
        lifecycle: _,
        message: _,
    } = constraint;
}

/// Where the studio surfaces each [`OptionRelation`] field.
///
/// The whole constraint is one `RustExpr` — `option_relations` holds a
/// rendered `&[OptionRelation { … }]` — so every field is spelled in that
/// literal rather than getting a picker of its own.
pub const OPTION_CONSTRAINT: &[Field] = &[
    f("kind", Surface::Expression("option_relations")),
    f("mode", Surface::Expression("option_relations")),
    f("subject", Surface::Expression("option_relations")),
    f("terms", Surface::Expression("option_relations")),
    f("surface", Surface::Expression("option_relations")),
    f("lifecycle", Surface::Expression("option_relations")),
    f("message", Surface::Expression("option_relations")),
];

/// Compile-time witness for [`VERSIONED_ARG_VALUE`].
pub fn witness_versioned_arg_value(gate: &VersionedArgValue) {
    let VersionedArgValue {
        index: _,
        value: _,
        lifecycle: _,
    } = gate;
}

/// Where the studio surfaces each [`VersionedArgValue`] field.
pub const VERSIONED_ARG_VALUE: &[Field] = &[
    f("index", Surface::Expression("versioned_arg_values")),
    f("value", Surface::Expression("versioned_arg_values")),
    f("lifecycle", Surface::Expression("versioned_arg_values")),
];

// Shared `&'static` descriptors.
//
// A class / clause-list descriptor is written once in the registry module
// that owns it and referenced by every command that needs it, so the studio's
// editor takes the *constant's path* rather than its contents.  Every field is
// still listed here: the gate makes a new one a stated decision.
//
// A definition-body grammar left that bucket: once every member states its
// effect the grammar is plain data all the way down, so the draft carries it
// field by field — as the name of the shipped grammar it equals, or whole —
// and the `SpecTcl` renderer writes it back out.

/// Compile-time witness for [`DEFINITION_BODY_GRAMMAR`].
pub fn witness_definition_body_grammar(grammar: &DefinitionBodyGrammar) {
    let DefinitionBodyGrammar {
        family: _,
        members: _,
        implicit_vars: _,
        implicit_receiver_vars: _,
        member_body_namespace_path: _,
        builtin_type_methods: _,
        builtin_object_methods: _,
        builtin_terminating_methods: _,
        member_body_commands: _,
        bare_word_construction: _,
        bare_word_construction_hint: _,
        bare_word_construction_source_hint: _,
        dynamic_method_dispatch: _,
        manufacturers: _,
        unknown_dispatch_method: _,
        property_accessor_methods: _,
    } = grammar;
}

/// Where the studio surfaces each [`DefinitionBodyGrammar`] field: a key of
/// the inline `definition_body` value. The bare-word construction hint is a
/// function pointer, so its key records only whether one is set.
pub const DEFINITION_BODY_GRAMMAR: &[Field] = &[
    f("family", Surface::Key("family")),
    f("members", Surface::Key("members")),
    f("implicit_vars", Surface::Key("implicit_vars")),
    f(
        "implicit_receiver_vars",
        Surface::Key("implicit_receiver_vars"),
    ),
    f(
        "member_body_namespace_path",
        Surface::Key("member_body_namespace_path"),
    ),
    f("builtin_type_methods", Surface::Key("builtin_type_methods")),
    f(
        "builtin_object_methods",
        Surface::Key("builtin_object_methods"),
    ),
    f(
        "builtin_terminating_methods",
        Surface::Key("builtin_terminating_methods"),
    ),
    f("member_body_commands", Surface::Key("member_body_commands")),
    f(
        "bare_word_construction",
        Surface::Key("bare_word_construction"),
    ),
    f(
        "bare_word_construction_hint",
        Surface::Key("bare_word_construction_hint"),
    ),
    f(
        "bare_word_construction_source_hint",
        Surface::Key("bare_word_construction_source_hint"),
    ),
    f(
        "dynamic_method_dispatch",
        Surface::Key("dynamic_method_dispatch"),
    ),
    f("manufacturers", Surface::Key("manufacturers")),
    f(
        "unknown_dispatch_method",
        Surface::Key("unknown_dispatch_method"),
    ),
    f(
        "property_accessor_methods",
        Surface::Key("property_accessor_methods"),
    ),
];

/// Compile-time witness for [`MEMBER_SPEC`].
pub fn witness_member_spec(member: &MemberSpec) {
    let MemberSpec {
        keyword: _,
        arg_roles: _,
        optional_argument: _,
        all_args_var: _,
        all_args_ref: _,
        kind: _,
        wrapper_block_body: _,
        surface: _,
        retraction: _,
        slot: _,
        visibility_effect: _,
        effect: _,
        wrapper_shift: _,
    } = member;
}

/// Where the studio surfaces each [`MemberSpec`] field: a key of one
/// `members` row. `effect` is the member-effect descriptor, spelt in the
/// `.tclspec` vocabulary (`{kind: callable, receiver: instance, …}`).
pub const MEMBER_SPEC: &[Field] = &[
    f("keyword", Surface::Key("keyword")),
    f("arg_roles", Surface::Key("arg_roles")),
    f("optional_argument", Surface::Key("optional_argument")),
    f("all_args_var", Surface::Key("all_args_var")),
    f("all_args_ref", Surface::Key("all_args_ref")),
    f("kind", Surface::Key("kind")),
    f("wrapper_block_body", Surface::Key("wrapper_block_body")),
    f("surface", Surface::Key("surface")),
    f("retraction", Surface::Key("retraction")),
    f("slot", Surface::Key("slot")),
    f("visibility_effect", Surface::Key("visibility_effect")),
    f("effect", Surface::Key("effect")),
    f("wrapper_shift", Surface::Key("wrapper_shift")),
];

/// Compile-time witness for [`OPTIONAL_MEMBER_ARGUMENT`].
pub fn witness_optional_member_argument(optional: &OptionalMemberArgument) {
    let OptionalMemberArgument {
        position: _,
        values: _,
    } = optional;
}

/// Where the studio surfaces each [`OptionalMemberArgument`] field.
pub const OPTIONAL_MEMBER_ARGUMENT: &[Field] = &[
    f("position", Surface::Key("position")),
    f("values", Surface::Key("values")),
];

/// Compile-time witness for [`MEMBER_OPTION_VALUE`].
pub fn witness_member_option_value(value: &MemberOptionValue) {
    let MemberOptionValue {
        value: _,
        role: _,
        surface: _,
        declared_visibility: _,
    } = value;
}

/// Where the studio surfaces each [`MemberOptionValue`] field.
pub const MEMBER_OPTION_VALUE: &[Field] = &[
    f("value", Surface::Key("value")),
    f("role", Surface::Key("role")),
    f("surface", Surface::Key("surface")),
    f("declared_visibility", Surface::Key("declared_visibility")),
];

/// Compile-time witness for [`SLOT_SPEC`].
pub fn witness_slot_spec(slot: &SlotSpec) {
    let SlotSpec {
        default_op: _,
        dedup: _,
    } = slot;
}

/// Where the studio surfaces each [`SlotSpec`] field.
pub const SLOT_SPEC: &[Field] = &[
    f("default_op", Surface::Key("default_op")),
    f("dedup", Surface::Key("dedup")),
];

/// Compile-time witness for [`WRAPPER_SHIFT`].
pub fn witness_wrapper_shift(shift: &WrapperShift) {
    let WrapperShift {
        receiver: _,
        visibility: _,
    } = shift;
}

/// Where the studio surfaces each [`WrapperShift`] field.
pub const WRAPPER_SHIFT: &[Field] = &[
    f("receiver", Surface::Key("receiver")),
    f("visibility", Surface::Key("visibility")),
];

/// Compile-time witness for [`BUILTIN_OBJECT_METHOD`].
pub fn witness_builtin_object_method(method: &BuiltinObjectMethod) {
    let BuiltinObjectMethod {
        name: _,
        visibility: _,
        receiver: _,
        operation: _,
        detail: _,
    } = method;
}

/// Where the studio surfaces each [`BuiltinObjectMethod`] field: a key of one
/// `builtin_object_methods` row.
pub const BUILTIN_OBJECT_METHOD: &[Field] = &[
    f("name", Surface::Key("name")),
    f("visibility", Surface::Key("visibility")),
    f("receiver", Surface::Key("receiver")),
    f("operation", Surface::Excluded(NAMED_CONSTANT)),
    f("detail", Surface::Key("detail")),
];

/// Compile-time witness for [`MEMBER_BODY_COMMAND`].
pub fn witness_member_body_command(command: &MemberBodyCommand) {
    let MemberBodyCommand {
        name: _,
        detail: _,
        binds_handle: _,
    } = command;
}

/// Compile-time witness for [`MANUFACTURER_METHOD`].
pub fn witness_manufacturer_method(manufacturer: &ManufacturerMethod) {
    let ManufacturerMethod {
        keyword: _,
        visibility: _,
        names_instance_at: _,
        definition_body_at: _,
        constructor_args_from: _,
    } = manufacturer;
}

/// Where the studio surfaces each [`ManufacturerMethod`] field.
pub const MANUFACTURER_METHOD: &[Field] = &[
    f("keyword", Surface::Expression("manufacturer_methods")),
    f("visibility", Surface::Expression("manufacturer_methods")),
    f(
        "names_instance_at",
        Surface::Expression("manufacturer_methods"),
    ),
    f(
        "definition_body_at",
        Surface::Expression("manufacturer_methods"),
    ),
    f(
        "constructor_args_from",
        Surface::Expression("manufacturer_methods"),
    ),
];

/// Where the studio surfaces each [`MemberBodyCommand`] field: a key of one
/// `member_body_commands` row.
pub const MEMBER_BODY_COMMAND: &[Field] = &[
    f("name", Surface::Key("name")),
    f("detail", Surface::Key("detail")),
    f("binds_handle", Surface::Key("binds_handle")),
];

/// Compile-time witness for [`OBJECT_CLASS_SPEC`].
pub fn witness_object_class_spec(spec: &ObjectClassSpec) {
    let ObjectClassSpec {
        class_name: _,
        instance_methods: _,
        superclasses: _,
        allow_unknown_methods: _,
        method_prefix_matching: _,
    } = spec;
}

/// Where the studio surfaces each [`ObjectClassSpec`] field.
///
/// Unlike the other `&'static` descriptors, this one is plain data all the way
/// down — a name, a flag, a list of superclass names, and a method table that
/// *is* `&[SubCommand]` — so the studio models it field by field inside the
/// `object_class` editor rather than as one opaque Rust expression, and the
/// `SpecTcl` renderer writes the whole thing back out.
pub const OBJECT_CLASS_SPEC: &[Field] = &[
    f("class_name", Surface::Key("class_name")),
    f("instance_methods", Surface::Key("instance_methods")),
    f("superclasses", Surface::Key("superclasses")),
    f(
        "allow_unknown_methods",
        Surface::Key("allow_unknown_methods"),
    ),
    f(
        "method_prefix_matching",
        Surface::Key("method_prefix_matching"),
    ),
];

/// Compile-time witness for [`CLAUSE_GRAMMAR_SPEC`].
pub fn witness_clause_grammar_spec(grammar: &ClauseGrammarSpec) {
    let ClauseGrammarSpec {
        head: _,
        rows: _,
        tail: _,
        fallthrough_body: _,
        default_clause: _,
        selection: _,
        surface: _,
    } = grammar;
}

/// Where the studio surfaces each [`ClauseGrammarSpec`] field.
///
/// Like `object_class`, the descriptor is plain data all the way down, so the
/// draft carries it field by field inside the `clause_grammar` value and the
/// `SpecTcl` renderer writes the whole block back out.
pub const CLAUSE_GRAMMAR_SPEC: &[Field] = &[
    f("head", Surface::Key("head")),
    f("rows", Surface::Key("rows")),
    f("tail", Surface::Key("tail")),
    f("fallthrough_body", Surface::Key("fallthrough_body")),
    f("default_clause", Surface::Key("default_clause")),
    f("selection", Surface::Key("selection")),
    f("surface", Surface::Key("surface")),
];

/// Compile-time witness for [`CLAUSE_ROW`].
pub fn witness_clause_row(row: &ClauseRow) {
    let ClauseRow {
        keyword: _,
        keyword_required: _,
        shape: _,
        timing: _,
        surface: _,
    } = row;
}

/// Where the studio surfaces each [`ClauseRow`] field. The shape is three
/// keys: which shape, the slots of a `repeated` / `once` row, and the layout
/// index a `group` row cites.
pub const CLAUSE_ROW: &[Field] = &[
    f("keyword", Surface::Key("keyword")),
    f("keyword_required", Surface::Key("keyword_required")),
    f("shape", Surface::Keys(&["shape", "slots", "layout"])),
    f("timing", Surface::Key("timing")),
    f("surface", Surface::Key("surface")),
];

/// Compile-time witness for [`CLAUSE_SLOT`].
pub fn witness_clause_slot(slot: &ClauseSlot) {
    let ClauseSlot {
        role: _,
        noise: _,
        handler: _,
        conditional_binding: _,
        optional: _,
    } = slot;
}

/// Where the studio surfaces each [`ClauseSlot`] field.
pub const CLAUSE_SLOT: &[Field] = &[
    f("role", Surface::Key("role")),
    f("noise", Surface::Key("noise")),
    f("handler", Surface::Key("handler")),
    f("conditional_binding", Surface::Key("conditional_binding")),
    f("optional", Surface::Key("optional")),
];

/// Compile-time witness for [`DEFAULT_CLAUSE`].
pub fn witness_default_clause(default: &DefaultClause) {
    let DefaultClause {
        row: _,
        final_only: _,
    } = default;
}

/// Where the studio surfaces each [`DefaultClause`] field.
pub const DEFAULT_CLAUSE: &[Field] = &[
    f("row", Surface::Key("row")),
    f("final_only", Surface::Key("final_only")),
];

/// Compile-time witness for [`CASE_LIST_SPEC`].
pub fn witness_case_list_spec(spec: &CaseListSpec) {
    let CaseListSpec {
        subject_args: _,
        two_arg_optionless_surface: _,
        fallthrough_body: _,
        value_options_require_regex: _,
        special_match_options: _,
        clause_flags: _,
        clause_regex_flag: _,
        clause_value_flags: _,
        clause_end_options_flag: _,
        clause_force_inline_flag: _,
        clause_force_list_flag: _,
        clause_force_list_shape: _,
        allow_omitted_final_body: _,
        keyword_patterns: _,
        keyword_patterns_require_final: _,
        exhaustive_keyword_patterns: _,
        optional_subject_separator: _,
        warn_unbraced_bodies: _,
        default_mode: _,
        pattern_words: _,
    } = spec;
}

/// Where the studio surfaces each [`CaseListSpec`] field.
pub const CASE_LIST_SPEC: &[Field] = &[
    f("subject_args", Surface::Excluded(NAMED_CONSTANT)),
    f(
        "two_arg_optionless_surface",
        Surface::Excluded(NAMED_CONSTANT),
    ),
    f("fallthrough_body", Surface::Excluded(NAMED_CONSTANT)),
    f(
        "value_options_require_regex",
        Surface::Excluded(NAMED_CONSTANT),
    ),
    f("special_match_options", Surface::Excluded(NAMED_CONSTANT)),
    f("clause_flags", Surface::Excluded(NAMED_CONSTANT)),
    f("clause_regex_flag", Surface::Excluded(NAMED_CONSTANT)),
    f("clause_value_flags", Surface::Excluded(NAMED_CONSTANT)),
    f("clause_end_options_flag", Surface::Excluded(NAMED_CONSTANT)),
    f(
        "clause_force_inline_flag",
        Surface::Excluded(NAMED_CONSTANT),
    ),
    f("clause_force_list_flag", Surface::Excluded(NAMED_CONSTANT)),
    f("clause_force_list_shape", Surface::Excluded(NAMED_CONSTANT)),
    f(
        "allow_omitted_final_body",
        Surface::Excluded(NAMED_CONSTANT),
    ),
    f("keyword_patterns", Surface::Excluded(NAMED_CONSTANT)),
    f(
        "exhaustive_keyword_patterns",
        Surface::Excluded(NAMED_CONSTANT),
    ),
    f(
        "keyword_patterns_require_final",
        Surface::Excluded(NAMED_CONSTANT),
    ),
    f(
        "optional_subject_separator",
        Surface::Excluded(NAMED_CONSTANT),
    ),
    f("warn_unbraced_bodies", Surface::Excluded(NAMED_CONSTANT)),
    f("default_mode", Surface::Excluded(NAMED_CONSTANT)),
    f("pattern_words", Surface::Excluded(NAMED_CONSTANT)),
];

#[cfg(test)]
mod tests;
