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
use tcl_registry::definer::{DefinitionBodyGrammar, MemberBodyCommand};
use tcl_registry::handle_binding::{HandleBindingSpec, HandleKeyword};
use tcl_registry::hooks::ArgTypeHint;
use tcl_registry::hover::{ArgValue, FormSpec, HoverSnippet, OptionArg, OptionSpec};
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::repeated::RepeatedArgLayout;
use tcl_registry::side_effects::SideEffect;
use tcl_registry::spec::{
    BytePayloadSpec, CaseListSpec, CommandSpec, ObjectClassSpec, SubCommand, SubSubCommand,
    VersionedArgValue,
};
use tcl_registry::symbol_def::SymbolDef;
use tcl_registry::taint::SetterConstraint;

/// The three draft keys a [`Lifecycle`] is edited under.
///
/// One `Lifecycle` value, three independent releases in the form — the
/// studio has always split it, and every surface (command, subcommand,
/// option) uses the same three keys.
pub const LIFECYCLE_KEYS: &[&str] = &[
    "introduced_version",
    "deprecated_version",
    "retired_version",
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

// ---------------------------------------------------------------------------
// CommandSpec
// ---------------------------------------------------------------------------

/// Compile-time witness that [`COMMAND_SPEC`] accounts for every
/// [`CommandSpec`] field.
///
/// The pattern below has **no `..` rest**: adding a field to `CommandSpec`
/// fails to compile here with `error[E0027]: pattern does not mention field
/// <name>`.  When that happens, surface the field — a `FieldSchema` entry in
/// `src/schema.rs` plus a seeder line in `src/draft.rs` — and add it to the
/// pattern and to [`COMMAND_SPEC`].  If it should not be author-editable, add
/// it as [`Surface::Excluded`] with the reason instead.
pub fn witness_command_spec(spec: &CommandSpec) {
    // THE GATE.  Do NOT add `..` to silence this — that is the drift it exists
    // to catch.  A new field belongs in FOUR places: `schema::COMMAND_FIELDS`
    // (gives it a browser editor and a rendered `.rs` line), `draft.rs` (seeds
    // it from a live spec), this pattern, and `COMMAND_SPEC` below.  If it must
    // not be author-editable, add it here and to `COMMAND_SPEC` as
    // `Surface::Excluded("why")` and skip the first two.
    let CommandSpec {
        name: _,
        traits: _,
        dialects: _,
        arity: _,
        arg_roles: _,
        arg_role_resolver: _,
        arg_presentation: _,
        repeated_args: _,
        frame_effect: _,
        clause_shape_check: _,
        command_prefixes: _,
        command_prefix_resolver: _,
        return_type: _,
        var_write_typing: _,
        return_elements: _,
        var_elements_effect: _,
        arg_types: _,
        subcommands: _,
        prefix_matching: _,
        allow_unknown_subcommands: _,
        default_form_first_word: _,
        hover: _,
        forms: _,
        command_forms: _,
        assigns_variable_at: _,
        safe_on_uninit: _,
        const_fold: _,
        const_fold_versioned: _,
        lowering_hook: _,
        bpf_op: _,
        codegen_hook: _,
        inline_codegen_hook: _,
        wasm_codegen_hook: _,
        analyser_hook: _,
        command_table_effect: _,
        side_effects: _,
        inferred_storage_type: _,
        required_package: _,
        excluded_events: _,
        unsafe_command: _,
        closed_value_args: _,
        event_requires: _,
        options: _,
        reserved_trailing_words: _,
        arg_values: _,
        body_kind: _,
        body_arg_implicit_args: _,
        taint_output_sink: _,
        taint_output_sink_subcommands: _,
        taint_log_sink: _,
        taint_network_sink_args: _,
        taint_code_sink_args: _,
        taint_interp_eval_subcommands: _,
        taint_source: _,
        taint_transform: _,
        taint_double_encode_colour: _,
        taint_sink_safe_colour: _,
        taint_sink_gate: _,
        credential_options: _,
        sensitive_headers: _,
        setter_constraints: _,
        pattern_type: _,
        format_string_type: _,
        tcllib_package: _,
        lifecycle: _,
        warn_missing_import: _,
        is_namespace_exported: _,
        xc_translatable: _,
        xc_operation: _,
        deprecated_replacement: _,
        deprecated_replacement_drop_in: _,
        byte_array_payload: _,
        byte_array_effect: _,
        definition_body: _,
        case_list: _,
        object_class: _,
        defines_symbol: _,
        body_scope: _,
        binds_handle: _,
        creates_instance_at: _,
        defines_command_at: _,
        context_gate: _,
        implementation_namespace: _,
        oo_context_facts: _,
    } = spec;
}

/// Where the studio surfaces each [`CommandSpec`] field, in
/// `CommandSpec::DEFAULT` order.
pub const COMMAND_SPEC: &[Field] = &[
    f("name", Surface::Key("name")),
    f("traits", Surface::Key("traits")),
    f("dialects", Surface::Key("dialects")),
    f("arity", Surface::Key("arity")),
    f("arg_roles", Surface::Key("arg_roles")),
    f("arg_role_resolver", Surface::Key("arg_role_resolver")),
    f("arg_presentation", Surface::Key("arg_presentation")),
    f("repeated_args", Surface::Key("repeated_args")),
    f("frame_effect", Surface::Key("frame_effect")),
    f("clause_shape_check", Surface::Key("clause_shape_check")),
    f("command_prefixes", Surface::Key("command_prefixes")),
    f(
        "command_prefix_resolver",
        Surface::Key("command_prefix_resolver"),
    ),
    f("return_type", Surface::Key("return_type")),
    f("var_write_typing", Surface::Key("var_write_typing")),
    f("return_elements", Surface::Key("return_elements")),
    f("var_elements_effect", Surface::Key("var_elements_effect")),
    f("arg_types", Surface::Key("arg_types")),
    f("subcommands", Surface::Key("subcommands")),
    f("prefix_matching", Surface::Key("prefix_matching")),
    f(
        "allow_unknown_subcommands",
        Surface::Key("allow_unknown_subcommands"),
    ),
    f(
        "default_form_first_word",
        Surface::Key("default_form_first_word"),
    ),
    f("hover", Surface::Key("hover")),
    f("forms", Surface::Key("forms")),
    f("command_forms", Surface::Key("command_forms")),
    f("assigns_variable_at", Surface::Key("assigns_variable_at")),
    f("safe_on_uninit", Surface::Key("safe_on_uninit")),
    f("const_fold", Surface::Key("const_fold")),
    f("const_fold_versioned", Surface::Key("const_fold_versioned")),
    f("lowering_hook", Surface::Key("lowering_hook")),
    f("bpf_op", Surface::Key("bpf_op")),
    f("codegen_hook", Surface::Key("codegen_hook")),
    f("inline_codegen_hook", Surface::Key("inline_codegen_hook")),
    f("wasm_codegen_hook", Surface::Key("wasm_codegen_hook")),
    f("analyser_hook", Surface::Key("analyser_hook")),
    f("command_table_effect", Surface::Key("command_table_effect")),
    f("side_effects", Surface::Key("side_effects")),
    f(
        "inferred_storage_type",
        Surface::Key("inferred_storage_type"),
    ),
    f("required_package", Surface::Key("required_package")),
    f("excluded_events", Surface::Key("excluded_events")),
    f("unsafe_command", Surface::Key("unsafe_command")),
    f("closed_value_args", Surface::Key("closed_value_args")),
    f("event_requires", Surface::Key("event_requires")),
    f("options", Surface::Key("options")),
    f(
        "reserved_trailing_words",
        Surface::Key("reserved_trailing_words"),
    ),
    f("arg_values", Surface::Key("arg_values")),
    f("body_kind", Surface::Key("body_kind")),
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
        "taint_interp_eval_subcommands",
        Surface::Key("taint_interp_eval_subcommands"),
    ),
    f("taint_source", Surface::Key("taint_source")),
    f("taint_transform", Surface::Key("taint_transform")),
    f(
        "taint_double_encode_colour",
        Surface::Key("taint_double_encode_colour"),
    ),
    f(
        "taint_sink_safe_colour",
        Surface::Key("taint_sink_safe_colour"),
    ),
    f("taint_sink_gate", Surface::Key("taint_sink_gate")),
    f("credential_options", Surface::Key("credential_options")),
    f("sensitive_headers", Surface::Key("sensitive_headers")),
    f("setter_constraints", Surface::Key("setter_constraints")),
    f("pattern_type", Surface::Key("pattern_type")),
    f("format_string_type", Surface::Key("format_string_type")),
    f("tcllib_package", Surface::Key("tcllib_package")),
    f("lifecycle", Surface::Keys(LIFECYCLE_KEYS)),
    f("warn_missing_import", Surface::Key("warn_missing_import")),
    f(
        "is_namespace_exported",
        Surface::Key("is_namespace_exported"),
    ),
    f("xc_translatable", Surface::Key("xc_translatable")),
    f("xc_operation", Surface::Key("xc_operation")),
    f(
        "deprecated_replacement",
        Surface::Key("deprecated_replacement"),
    ),
    f(
        "deprecated_replacement_drop_in",
        Surface::Key("deprecated_replacement_drop_in"),
    ),
    f("byte_array_payload", Surface::Key("byte_array_payload")),
    f("byte_array_effect", Surface::Key("byte_array_effect")),
    f("definition_body", Surface::Key("definition_body")),
    f("case_list", Surface::Key("case_list")),
    f("object_class", Surface::Key("object_class")),
    f("defines_symbol", Surface::Key("defines_symbol")),
    f("body_scope", Surface::Key("body_scope")),
    f("binds_handle", Surface::Key("binds_handle")),
    f("creates_instance_at", Surface::Key("creates_instance_at")),
    f("defines_command_at", Surface::Key("defines_command_at")),
    f("context_gate", Surface::Key("context_gate")),
    f(
        "implementation_namespace",
        Surface::Key("implementation_namespace"),
    ),
    f("oo_context_facts", Surface::Key("oo_context_facts")),
];

// ---------------------------------------------------------------------------
// SubCommand
// ---------------------------------------------------------------------------

/// Compile-time witness that [`SUB_COMMAND`] accounts for every
/// [`SubCommand`] field — see [`witness_command_spec`] for what to do when it
/// stops compiling.
pub fn witness_sub_command(sub: &SubCommand) {
    // THE GATE — see `witness_command_spec`. No `..`, ever.
    let SubCommand {
        name: _,
        traits: _,
        arity: _,
        detail: _,
        synopsis: _,
        hover: _,
        arg_roles: _,
        arg_role_resolver: _,
        arg_presentation: _,
        repeated_args: _,
        command_prefixes: _,
        command_prefix_resolver: _,
        return_type: _,
        var_write_typing: _,
        return_elements: _,
        var_elements_effect: _,
        arg_types: _,
        pure: _,
        mutator: _,
        const_fold: _,
        const_fold_versioned: _,
        lowering_hook: _,
        codegen_hook: _,
        inline_codegen_hook: _,
        wasm_codegen_hook: _,
        analyser_hook: _,
        command_table_effect: _,
        options: _,
        min_abbrev: _,
        prefix_matching: _,
        arg_values: _,
        versioned_arg_values: _,
        subcommand_forms: _,
        dialects: _,
        lifecycle: _,
        safe_on_uninit: _,
        loop_list_header: _,
        creates_scope_alias: _,
        inferred_storage_type: _,
        body_kind: _,
        byte_array_effect: _,
        closed_value_args: _,
        arg_values_accept_prefix: _,
        body_arg_implicit_args: _,
        taint_transform: _,
        taint_double_encode_colour: _,
        taint_output_sink: _,
        credential_arg: _,
        sensitive_headers: _,
        pattern_type: _,
        format_string_type: _,
        xc_operation: _,
        side_effects: _,
        destructive: _,
        returns_path: _,
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
    f("detail", Surface::Key("detail")),
    f("synopsis", Surface::Key("synopsis")),
    f("hover", Surface::Key("hover")),
    f("arg_roles", Surface::Key("arg_roles")),
    f("arg_role_resolver", Surface::Key("arg_role_resolver")),
    f("arg_presentation", Surface::Key("arg_presentation")),
    f("repeated_args", Surface::Key("repeated_args")),
    f("command_prefixes", Surface::Key("command_prefixes")),
    f(
        "command_prefix_resolver",
        Surface::Key("command_prefix_resolver"),
    ),
    f("return_type", Surface::Key("return_type")),
    f("var_write_typing", Surface::Key("var_write_typing")),
    f("return_elements", Surface::Key("return_elements")),
    f("var_elements_effect", Surface::Key("var_elements_effect")),
    f("arg_types", Surface::Key("arg_types")),
    f("pure", Surface::Key("pure")),
    f("mutator", Surface::Key("mutator")),
    f("const_fold", Surface::Key("const_fold")),
    f("const_fold_versioned", Surface::Key("const_fold_versioned")),
    f("lowering_hook", Surface::Key("lowering_hook")),
    f("codegen_hook", Surface::Key("codegen_hook")),
    f("inline_codegen_hook", Surface::Key("inline_codegen_hook")),
    f("wasm_codegen_hook", Surface::Key("wasm_codegen_hook")),
    f("analyser_hook", Surface::Key("analyser_hook")),
    f("command_table_effect", Surface::Key("command_table_effect")),
    f("options", Surface::Key("options")),
    f("min_abbrev", Surface::Key("min_abbrev")),
    f("prefix_matching", Surface::Key("prefix_matching")),
    f("arg_values", Surface::Key("arg_values")),
    f("versioned_arg_values", Surface::Key("versioned_arg_values")),
    f("subcommand_forms", Surface::Key("subcommand_forms")),
    f("dialects", Surface::Key("dialects")),
    f("lifecycle", Surface::Keys(LIFECYCLE_KEYS)),
    f("safe_on_uninit", Surface::Key("safe_on_uninit")),
    f("loop_list_header", Surface::Key("loop_list_header")),
    f("creates_scope_alias", Surface::Key("creates_scope_alias")),
    f(
        "inferred_storage_type",
        Surface::Key("inferred_storage_type"),
    ),
    f("body_kind", Surface::Key("body_kind")),
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
    f(
        "taint_double_encode_colour",
        Surface::Key("taint_double_encode_colour"),
    ),
    f("taint_output_sink", Surface::Key("taint_output_sink")),
    f("credential_arg", Surface::Key("credential_arg")),
    f("sensitive_headers", Surface::Key("sensitive_headers")),
    f("pattern_type", Surface::Key("pattern_type")),
    f("format_string_type", Surface::Key("format_string_type")),
    f("xc_operation", Surface::Key("xc_operation")),
    f("side_effects", Surface::Key("side_effects")),
    f("destructive", Surface::Key("destructive")),
    f("returns_path", Surface::Key("returns_path")),
    f("is_unescape", Surface::Key("is_unescape")),
    f("cfg_rewrite_name", Surface::Key("cfg_rewrite_name")),
    f("sub_subcommands", Surface::Key("sub_subcommands")),
    f("defines_command_at", Surface::Key("defines_command_at")),
    f(
        "max_leading_option_words",
        Surface::Key("max_leading_option_words"),
    ),
];

// ---------------------------------------------------------------------------
// The nested types a draft carries structurally.
//
// These have no `FieldSchema` entry of their own: they are edited inside the
// row editor of the parent field that holds them (`options`, `hover`,
// `side_effects`, …), so their draft keys are keys of the JSON object
// `crate::draft` builds for the type.  Same gate, same rule — a new field on
// any of them fails to compile until it is carried or excluded.
// ---------------------------------------------------------------------------

/// Compile-time witness for [`SUB_SUB_COMMAND`].
pub fn witness_sub_sub_command(sub: &SubSubCommand) {
    let SubSubCommand {
        name: _,
        detail: _,
        synopsis: _,
        dialects: _,
    } = sub;
}

/// Where the studio surfaces each [`SubSubCommand`] field.
pub const SUB_SUB_COMMAND: &[Field] = &[
    f("name", Surface::Key("name")),
    f("detail", Surface::Key("detail")),
    f("synopsis", Surface::Key("synopsis")),
    f("dialects", Surface::Key("dialects")),
];

/// Compile-time witness for [`OPTION_SPEC`].
pub fn witness_option_spec(opt: &OptionSpec) {
    let OptionSpec {
        name: _,
        value: _,
        detail: _,
        dialects: _,
        aliases: _,
        lifecycle: _,
        min_abbrev: _,
    } = opt;
}

/// Where the studio surfaces each [`OptionSpec`] field.
pub const OPTION_SPEC: &[Field] = &[
    f("name", Surface::Key("name")),
    f("value", Surface::Key("value")),
    f("detail", Surface::Key("detail")),
    f("dialects", Surface::Key("dialects")),
    f("aliases", Surface::Key("aliases")),
    f("lifecycle", Surface::Keys(LIFECYCLE_KEYS)),
    f("min_abbrev", Surface::Key("min_abbrev")),
];

/// Compile-time witness for [`OPTION_ARG`].
pub fn witness_option_arg(arg: &OptionArg) {
    let OptionArg {
        arity: _,
        role: _,
        also_role: _,
        body_kind: _,
        values: _,
        closed: _,
        integer: _,
        hint: _,
        appended_arity: _,
    } = arg;
}

/// Where the studio surfaces each [`OptionArg`] field — the keys of the
/// `value` object inside an option row.
pub const OPTION_ARG: &[Field] = &[
    f("arity", Surface::Key("arity")),
    f("role", Surface::Key("role")),
    f("also_role", Surface::Key("also_role")),
    f("body_kind", Surface::Key("body_kind")),
    f("values", Surface::Key("values")),
    f("closed", Surface::Key("closed")),
    f("integer", Surface::Key("integer")),
    f("hint", Surface::Key("hint")),
    f("appended_arity", Surface::Key("appended_arity")),
];

/// Compile-time witness for [`ARG_VALUE`].
pub fn witness_arg_value(value: &ArgValue) {
    let ArgValue {
        value: _,
        detail: _,
        min_tcl: _,
        code: _,
    } = value;
}

/// Where the studio surfaces each [`ArgValue`] field.
pub const ARG_VALUE: &[Field] = &[
    f("value", Surface::Key("value")),
    f("detail", Surface::Key("detail")),
    f("min_tcl", Surface::Key("min_tcl")),
    f("code", Surface::Key("code")),
];

/// Compile-time witness for [`FORM_SPEC`].
pub fn witness_form_spec(form: &FormSpec) {
    let FormSpec {
        kind: _,
        synopsis: _,
        dialects: _,
    } = form;
}

/// Where the studio surfaces each [`FormSpec`] field.
pub const FORM_SPEC: &[Field] = &[
    f("kind", Surface::Key("kind")),
    f("synopsis", Surface::Key("synopsis")),
    f("dialects", Surface::Key("dialects")),
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
        dialects: _,
    } = effect;
}

/// Where the studio surfaces each [`SideEffect`] field.
pub const SIDE_EFFECT: &[Field] = &[
    f("target", Surface::Key("target")),
    f("reads", Surface::Key("reads")),
    f("writes", Surface::Key("writes")),
    f("connection_side", Surface::Key("connection_side")),
    f("dialects", Surface::Key("dialects")),
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
        max: _,
        step: _,
        also_exact: _,
    } = arity;
}

/// Where the studio surfaces each [`Arity`] field.
pub const ARITY: &[Field] = &[
    f("min", Surface::Key("min")),
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
    } = lifecycle;
}

/// Where the studio surfaces each [`Lifecycle`] field.
pub const LIFECYCLE: &[Field] = &[
    f("introduced", Surface::Key("introduced_version")),
    f("deprecated", Surface::Key("deprecated_version")),
    f("retired", Surface::Key("retired_version")),
];

// ---------------------------------------------------------------------------
// Plain-data descriptors.
//
// Each of these is reachable only through one `RustExpr` field, but is pure
// data — so `crate::draft` renders it back out as a full struct literal and it
// round-trips.  `Surface::Expression` names the field that carries it, and
// `descriptor_literals_name_every_field` proves the rendered literal really
// mentions each one.
// ---------------------------------------------------------------------------

/// Compile-time witness for [`REPEATED_ARG_LAYOUT`].
pub fn witness_repeated_arg_layout(layout: &RepeatedArgLayout) {
    let RepeatedArgLayout {
        role: _,
        start: _,
        stride: _,
        exclude_trailing: _,
        optional_leading_word: _,
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
];

/// Compile-time witness for [`HANDLE_BINDING_SPEC`].
pub fn witness_handle_binding_spec(spec: &HandleBindingSpec) {
    let HandleBindingSpec {
        name_from: _,
        class_from: _,
        keyword: _,
    } = spec;
}

/// Where the studio surfaces each [`HandleBindingSpec`] field (issue #1185).
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

// ---------------------------------------------------------------------------
// Shared `&'static` descriptors.
//
// A grammar / class / clause-list descriptor is written once in the registry
// module that owns it and referenced by every command that needs it, so the
// studio's editor takes the *constant's path* rather than its contents.  Every
// field is still listed here: the gate makes a new one a stated decision.
// ---------------------------------------------------------------------------

/// Compile-time witness for [`DEFINITION_BODY_GRAMMAR`].
pub fn witness_definition_body_grammar(grammar: &DefinitionBodyGrammar) {
    let DefinitionBodyGrammar {
        family: _,
        members: _,
        implicit_vars: _,
        member_body_namespace_path: _,
        builtin_type_methods: _,
        member_body_commands: _,
        bare_word_construction: _,
    } = grammar;
}

/// Where the studio surfaces each [`DefinitionBodyGrammar`] field.
pub const DEFINITION_BODY_GRAMMAR: &[Field] = &[
    f("family", Surface::Excluded(NAMED_CONSTANT)),
    f("members", Surface::Excluded(NAMED_CONSTANT)),
    f("implicit_vars", Surface::Excluded(NAMED_CONSTANT)),
    f(
        "member_body_namespace_path",
        Surface::Excluded(NAMED_CONSTANT),
    ),
    f("builtin_type_methods", Surface::Excluded(NAMED_CONSTANT)),
    f("member_body_commands", Surface::Excluded(NAMED_CONSTANT)),
    f("bare_word_construction", Surface::Excluded(NAMED_CONSTANT)),
];

/// Compile-time witness for [`MEMBER_BODY_COMMAND`].
pub fn witness_member_body_command(command: &MemberBodyCommand) {
    let MemberBodyCommand {
        name: _,
        detail: _,
        binds_handle: _,
    } = command;
}

/// Where the studio surfaces each [`MemberBodyCommand`] field.
pub const MEMBER_BODY_COMMAND: &[Field] = &[
    f("name", Surface::Excluded(NAMED_CONSTANT)),
    f("detail", Surface::Excluded(NAMED_CONSTANT)),
    f("binds_handle", Surface::Excluded(NAMED_CONSTANT)),
];

/// Compile-time witness for [`OBJECT_CLASS_SPEC`].
pub fn witness_object_class_spec(spec: &ObjectClassSpec) {
    let ObjectClassSpec {
        class_name: _,
        instance_methods: _,
        superclasses: _,
        allow_unknown_methods: _,
    } = spec;
}

/// Where the studio surfaces each [`ObjectClassSpec`] field.
pub const OBJECT_CLASS_SPEC: &[Field] = &[
    f("class_name", Surface::Excluded(NAMED_CONSTANT)),
    f("instance_methods", Surface::Excluded(NAMED_CONSTANT)),
    f("superclasses", Surface::Excluded(NAMED_CONSTANT)),
    f("allow_unknown_methods", Surface::Excluded(NAMED_CONSTANT)),
];

/// Compile-time witness for [`CASE_LIST_SPEC`].
pub fn witness_case_list_spec(spec: &CaseListSpec) {
    let CaseListSpec {
        subject_args: _,
        regex_option: _,
        value_options: _,
        clause_flags: _,
        clause_regex_flag: _,
        clause_value_flags: _,
        keyword_patterns: _,
    } = spec;
}

/// Where the studio surfaces each [`CaseListSpec`] field.
pub const CASE_LIST_SPEC: &[Field] = &[
    f("subject_args", Surface::Excluded(NAMED_CONSTANT)),
    f("regex_option", Surface::Excluded(NAMED_CONSTANT)),
    f("value_options", Surface::Excluded(NAMED_CONSTANT)),
    f("clause_flags", Surface::Excluded(NAMED_CONSTANT)),
    f("clause_regex_flag", Surface::Excluded(NAMED_CONSTANT)),
    f("clause_value_flags", Surface::Excluded(NAMED_CONSTANT)),
    f("keyword_patterns", Surface::Excluded(NAMED_CONSTANT)),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{draft, render_rs, schema};
    use serde_json::{Map, Value, json};
    use tcl_core_types::DiagCode;
    use tcl_registry::arg_role::ArgRole;
    use tcl_registry::handle_binding::{HandleClassSource, HandleName};
    use tcl_registry::hover::{FormKind, OptionArity, OptionValue};
    use tcl_registry::side_effects::{ConnectionSide, SideEffectTarget};
    use tcl_registry::symbol_def::DefinedSymbolKind;

    /// The draft keys a coverage entry claims; empty for an exclusion.
    fn claimed(surface: Surface) -> Vec<&'static str> {
        match surface {
            Surface::Key(k) | Surface::Expression(k) => vec![k],
            Surface::Keys(ks) => ks.to_vec(),
            Surface::Excluded(_) => Vec::new(),
        }
    }

    /// The keys of a JSON object, or a panic naming `what` when it is not one.
    fn object_keys<'a>(what: &str, value: &'a Value) -> Vec<&'a str> {
        value
            .as_object()
            .unwrap_or_else(|| panic!("{what} seeds a JSON object"))
            .keys()
            .map(String::as_str)
            .collect()
    }

    /// Every `Key` / `Keys` entry names a key the seeded draft really holds.
    fn assert_carried(what: &str, table: &[Field], keys: &[&str]) {
        for field in table {
            if matches!(field.surface, Surface::Excluded(_) | Surface::Expression(_)) {
                continue;
            }
            for key in claimed(field.surface) {
                assert!(
                    keys.contains(&key),
                    "{what}::{} claims draft key `{key}`, which the seeder never writes",
                    field.name
                );
            }
        }
    }

    /// Every `Key` / `Keys` entry names a schema field, so the browser renders
    /// an editor for it and `render_rs` emits it.
    fn assert_schema_has(
        what: &str,
        table: &[Field],
        lookup: fn(&str) -> Option<&'static schema::FieldSchema>,
    ) {
        for field in table {
            if matches!(field.surface, Surface::Excluded(_)) {
                continue;
            }
            for key in claimed(field.surface) {
                assert!(
                    lookup(key).is_some(),
                    "{what}::{} claims schema key `{key}`, which schema.rs does not declare",
                    field.name
                );
            }
        }
    }

    /// No schema field without a coverage entry — catches a schema key left
    /// behind by a registry field that has since been removed.
    fn assert_no_orphan_schema_keys(what: &str, table: &[Field], fields: &[schema::FieldSchema]) {
        let claimed_keys: Vec<&str> = table.iter().flat_map(|f| claimed(f.surface)).collect();
        for field in fields {
            assert!(
                claimed_keys.contains(&field.key),
                "schema declares {what} field `{}` that no coverage entry claims — remove it \
                 from schema.rs, or add back the registry field it stands for",
                field.key
            );
        }
    }

    #[test]
    fn command_spec_is_fully_surfaced() {
        witness_command_spec(&CommandSpec::DEFAULT);
        let drafted = draft::default_command_draft();
        let keys: Vec<&str> = drafted.keys().map(String::as_str).collect();
        assert_carried("CommandSpec", COMMAND_SPEC, &keys);
        assert_schema_has("CommandSpec", COMMAND_SPEC, schema::command_field);
        assert_no_orphan_schema_keys("CommandSpec", COMMAND_SPEC, schema::COMMAND_FIELDS);
    }

    #[test]
    fn sub_command_is_fully_surfaced() {
        witness_sub_command(&SubCommand::DEFAULT);
        let drafted = draft::default_subcommand_draft();
        let keys: Vec<&str> = drafted.keys().map(String::as_str).collect();
        assert_carried("SubCommand", SUB_COMMAND, &keys);
        assert_schema_has("SubCommand", SUB_COMMAND, schema::subcommand_field);
        assert_no_orphan_schema_keys("SubCommand", SUB_COMMAND, schema::SUBCOMMAND_FIELDS);
    }

    /// A `SetterConstraint` to seed from — the type has no `DEFAULT`, and its
    /// `code` is a real diagnostic code rather than a string.
    const SETTER: SetterConstraint = SetterConstraint {
        arg_index: 0,
        required_prefix: "/",
        code: DiagCode::Irule3101,
        message: "",
    };

    /// The nested types have no `FieldSchema` of their own: each is edited
    /// inside its parent field's row editor, so what has to hold is that the
    /// draft's JSON object for the type carries a key per field.
    #[test]
    fn the_nested_row_types_are_fully_surfaced() {
        witness_sub_sub_command(&SubSubCommand {
            name: "",
            detail: "",
            synopsis: "",
            dialects: None,
        });
        let sub_sub = draft::sub_subcommand(&SubSubCommand {
            name: "",
            detail: "",
            synopsis: "",
            dialects: None,
        });
        assert_carried(
            "SubSubCommand",
            SUB_SUB_COMMAND,
            &object_keys("SubSubCommand", &sub_sub),
        );

        // An option row, with a value so the nested `OptionArg` object exists.
        let opt = OptionSpec {
            value: OptionValue::Takes(OptionArg {
                arity: OptionArity::One,
                ..OptionArg::DEFAULT
            }),
            ..OptionSpec::DEFAULT
        };
        witness_option_spec(&opt);
        witness_option_arg(&OptionArg::DEFAULT);
        let (row, _) = draft::option_spec(&opt);
        assert_carried("OptionSpec", OPTION_SPEC, &object_keys("OptionSpec", &row));
        assert_carried(
            "OptionArg",
            OPTION_ARG,
            &object_keys("OptionArg", &row["value"]),
        );

        witness_arg_value(&ArgValue::DEFAULT);
        let value = draft::arg_value(&ArgValue::DEFAULT);
        assert_carried("ArgValue", ARG_VALUE, &object_keys("ArgValue", &value));

        witness_form_spec(&FormSpec::DEFAULT);
        let form = draft::form_spec(&FormSpec::DEFAULT);
        assert_carried("FormSpec", FORM_SPEC, &object_keys("FormSpec", &form));

        let snippet = HoverSnippet::brief("", &[], "");
        witness_hover_snippet(&snippet);
        let hover = draft::hover(Some(snippet));
        assert_carried(
            "HoverSnippet",
            HOVER_SNIPPET,
            &object_keys("HoverSnippet", &hover),
        );

        witness_side_effect(&SideEffect::DEFAULT);
        let effect = draft::side_effect(&SideEffect::DEFAULT);
        assert_carried(
            "SideEffect",
            SIDE_EFFECT,
            &object_keys("SideEffect", &effect),
        );

        witness_setter_constraint(&SETTER);
        let constraint = draft::setter_constraint(&SETTER);
        assert_carried(
            "SetterConstraint",
            SETTER_CONSTRAINT,
            &object_keys("SetterConstraint", &constraint),
        );

        witness_arity(&Arity::any());
        let arity = draft::arity(Arity::any());
        assert_carried("Arity", ARITY, &object_keys("Arity", &arity));

        let hint = ArgTypeHint {
            expected: None,
            shimmers: false,
            transparent_from: &[],
        };
        witness_arg_type_hint(&hint);
        let hints = draft::arg_type_map(&[(0, hint)]);
        assert_carried(
            "ArgTypeHint",
            ARG_TYPE_HINT,
            &object_keys("ArgTypeHint", &hints[0]),
        );

        witness_lifecycle(&Lifecycle::UNSPECIFIED);
        let mut life = Map::new();
        draft::insert_lifecycle(&mut life, Lifecycle::UNSPECIFIED);
        let life_keys: Vec<&str> = life.keys().map(String::as_str).collect();
        assert_carried("Lifecycle", LIFECYCLE, &life_keys);
    }

    /// Every exclusion states a reason, and names a type whose fields really
    /// are reached through one shared-constant editor.
    #[test]
    fn every_exclusion_carries_a_reason() {
        witness_definition_body_grammar(&tcl_registry::definer::SNIT_GRAMMAR);
        witness_member_body_command(
            tcl_registry::definer::SNIT_GRAMMAR
                .member_body_command("install")
                .expect("snit injects `install` into every member body"),
        );
        witness_object_class_spec(&ObjectClassSpec {
            class_name: "",
            instance_methods: &[],
            superclasses: &[],
            allow_unknown_methods: false,
        });
        witness_case_list_spec(&CaseListSpec::SWITCH);
        for (what, table) in [
            ("DefinitionBodyGrammar", DEFINITION_BODY_GRAMMAR),
            ("MemberBodyCommand", MEMBER_BODY_COMMAND),
            ("ObjectClassSpec", OBJECT_CLASS_SPEC),
            ("CaseListSpec", CASE_LIST_SPEC),
        ] {
            for field in table {
                let Surface::Excluded(reason) = field.surface else {
                    panic!("{what}::{} is not an exclusion", field.name);
                };
                assert!(!reason.is_empty(), "{what}::{} has no reason", field.name);
            }
            // The whole descriptor is one editor on some command field, so the
            // schema must still offer a way to name it.
            assert!(!table.is_empty(), "{what} has no coverage entries");
        }
        for key in ["definition_body", "object_class", "case_list", "body_scope"] {
            assert!(
                schema::command_field(key).is_some(),
                "`{key}` must stay an editable field even though its descriptor is a constant"
            );
        }
    }

    // -- the plain-data descriptors really round-trip ------------------------
    //
    // Each literal below is written twice: once as Rust the compiler accepts,
    // and once as the string the studio renders for it. If the two ever
    // disagree the studio is emitting a spec that does not compile — and a
    // descriptor field added without a renderer line shows up as a missing
    // `name:` in the expected text.

    const WITNESS_BINDING: HandleBindingSpec = HandleBindingSpec {
        name_from: HandleName::Word(0),
        class_from: HandleClassSource::Word(2),
        keyword: Some(HandleKeyword {
            at: 1,
            word: "using",
        }),
    };

    const WITNESS_REPEAT: RepeatedArgLayout = RepeatedArgLayout {
        role: ArgRole::VarWrite,
        start: 1,
        stride: 2,
        exclude_trailing: 1,
        optional_leading_word: true,
    };

    const WITNESS_SYMBOL: SymbolDef = SymbolDef {
        name_arg: 0,
        detail_arg: Some(1),
        requires_arg: None,
        kind: DefinedSymbolKind::Test,
    };

    const WITNESS_PAYLOAD: BytePayloadSpec = BytePayloadSpec {
        replace_data_index: 3,
        message_flag_shift: true,
    };

    const WITNESS_GATE: VersionedArgValue = VersionedArgValue {
        index: 0,
        value: "utf-8",
        lifecycle: Lifecycle::UNSPECIFIED,
    };

    const WITNESS_SUBS: &[SubCommand] = &[SubCommand {
        name: "encoding",
        versioned_arg_values: &[WITNESS_GATE],
        ..SubCommand::DEFAULT
    }];

    fn witness_spec() -> CommandSpec {
        CommandSpec {
            name: "witness",
            repeated_args: &[WITNESS_REPEAT],
            binds_handle: Some(&WITNESS_BINDING),
            byte_array_payload: Some(WITNESS_PAYLOAD),
            defines_symbol: Some(WITNESS_SYMBOL),
            oo_context_facts: &[("class", OoContextFactWitness::FACT)],
            subcommands: WITNESS_SUBS,
            ..CommandSpec::DEFAULT
        }
    }

    /// `OoContextFact` has one variant; naming it through a constant keeps the
    /// witness readable next to the string it must render as.
    struct OoContextFactWitness;
    impl OoContextFactWitness {
        const FACT: tcl_registry::spec::OoContextFact =
            tcl_registry::spec::OoContextFact::DefiningClass;
    }

    #[test]
    fn descriptor_literals_name_every_field() {
        let d = draft::from_command_spec(&witness_spec());
        assert_eq!(
            d["binds_handle"],
            json!(
                "Some(&HandleBindingSpec { name_from: HandleName::Word(0), \
                 class_from: HandleClassSource::Word(2), \
                 keyword: Some(HandleKeyword { at: 1, word: \"using\" }) })"
            )
        );
        assert_eq!(
            d["repeated_args"],
            json!(
                "&[RepeatedArgLayout { role: ArgRole::VarWrite, start: 1, stride: 2, \
                 exclude_trailing: 1, optional_leading_word: true }]"
            )
        );
        assert_eq!(
            d["defines_symbol"],
            json!(
                "Some(SymbolDef { name_arg: 0, detail_arg: Some(1), requires_arg: None, \
                 kind: DefinedSymbolKind::Test })"
            )
        );
        assert_eq!(
            d["byte_array_payload"],
            json!("Some(BytePayloadSpec { replace_data_index: 3, message_flag_shift: true })")
        );
        assert_eq!(
            d["oo_context_facts"],
            json!("&[(\"class\", OoContextFact::DefiningClass)]")
        );
        assert_eq!(
            d["subcommands"][0]["versioned_arg_values"],
            json!(
                "&[VersionedArgValue { index: 0, value: \"utf-8\", lifecycle: Lifecycle { \
                 introduced: None, deprecated: None, retired: None } }]"
            )
        );
    }

    /// The `binds_handle` expression above, written as the Rust a rendered
    /// module contains: an inline `&`-borrow of a constant struct literal,
    /// inside a `fn` returning the spec.
    ///
    /// The point is that this *compiles*. `binds_handle` wants a `&'static`,
    /// and only rvalue static promotion makes an inline literal satisfy it —
    /// so if the renderer's chosen shape ever stopped being promotable, the
    /// build breaks here rather than in the file an author downloads.
    fn promoted_binding_spec() -> CommandSpec {
        CommandSpec {
            binds_handle: Some(&HandleBindingSpec {
                name_from: HandleName::Word(0),
                class_from: HandleClassSource::Word(2),
                keyword: Some(HandleKeyword {
                    at: 1,
                    word: "using",
                }),
            }),
            ..CommandSpec::DEFAULT
        }
    }

    #[test]
    fn the_rendered_descriptor_expression_is_the_rust_that_compiles() {
        assert_eq!(
            draft::from_command_spec(&promoted_binding_spec())["binds_handle"],
            draft::from_command_spec(&witness_spec())["binds_handle"],
            "the promotable literal above and the rendered expression must be the same tokens"
        );
    }

    /// Every `Expression` entry's field name really appears in the literal the
    /// renderer emits — the mechanical half of the assertions above, so a new
    /// descriptor field that nobody wired into `draft.rs` fails here too.
    #[test]
    fn every_expression_field_reaches_the_rendered_spec() {
        let source = render_rs::render(&draft::from_command_spec(&witness_spec()));
        for (what, table) in [
            ("RepeatedArgLayout", REPEATED_ARG_LAYOUT),
            ("HandleBindingSpec", HANDLE_BINDING_SPEC),
            ("HandleKeyword", HANDLE_KEYWORD),
            ("SymbolDef", SYMBOL_DEF),
            ("BytePayloadSpec", BYTE_PAYLOAD_SPEC),
            ("VersionedArgValue", VERSIONED_ARG_VALUE),
        ] {
            for field in table {
                let Surface::Expression(key) = field.surface else {
                    panic!("{what}::{} is not rendered as an expression", field.name);
                };
                assert!(
                    schema::command_field(key).is_some() || schema::subcommand_field(key).is_some(),
                    "{what}::{} names `{key}`, which is not a schema field",
                    field.name
                );
                assert!(
                    source.contains(&format!("{}: ", field.name)),
                    "{what}::{} never reaches the rendered spec:\n{source}",
                    field.name
                );
            }
        }
        // The whole point: a seeded descriptor renders instead of leaving a
        // `TODO` for the author to retype.
        assert!(
            !source.contains("`binds_handle` is set on the source command"),
            "binds_handle must round-trip, not be reported unrecoverable:\n{source}"
        );
    }

    /// A descriptor the registry really declares survives the round trip —
    /// `set`'s handle binding (issue #1185), seeded from the live registry.
    #[test]
    fn a_live_registry_binding_round_trips() {
        let registry = tcl_registry::registry::CommandRegistry::build_default();
        let spec = registry.get("set").expect("set is a core command");
        assert!(
            spec.binds_handle.is_some(),
            "`set` declares the handle binding this test exists to round-trip"
        );
        let source = render_rs::render(&draft::from_command_spec(spec));
        assert!(
            source.contains(
                "binds_handle: Some(&HandleBindingSpec { name_from: HandleName::Word(0), "
            ),
            "the live binding must render as a literal:\n{source}"
        );
    }

    /// Guard the imports the assertions above rely on staying meaningful.
    #[test]
    fn witness_constants_are_the_shapes_they_claim() {
        assert_eq!(WITNESS_BINDING.class_from.index(), 2);
        // start 1, stride 2, one trailing word excluded: 5 words cover 1 and 3.
        assert_eq!(WITNESS_REPEAT.indices(5), vec![1, 3]);
        assert_eq!(SETTER.code, DiagCode::Irule3101);
        assert_eq!(FormKind::Default, FormSpec::DEFAULT.kind);
        assert_eq!(ConnectionSide::None, SideEffect::DEFAULT.connection_side);
        assert_eq!(SideEffectTarget::Unknown, SideEffect::DEFAULT.target);
    }
}
