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

//! `SpecTcl`: the member tables and grammars of the `.tclspec` spec-pack DSL's declaration bodies.

use super::{
    BODY0_ROLES, DefinerFamily, DefinitionBodyGrammar, MemberOptionValue, MemberSpec,
    OptionalMemberArgument, setting, setting_word,
};
use crate::arg_role::ArgRole;

// SpecTcl — the `.tclspec` spec-pack DSL's own declaration bodies.
//
// `speclib NAME VERSION { … }` is a definition body in exactly the sense the
// class definers are: its top-level words are declaration keywords, not
// commands, and they mean nothing outside it.  The DSL nests, so each block
// statement is *also* a definer whose own body has its own member vocabulary
// (`command … { … }` → the per-command keys, `hover { … }` → the six hover
// keys, …).  Nesting needs no new machinery: each block statement carries a
// `CommandSpec` whose `definition_body` is the inner grammar, and the shared
// walker's "an outer definer switches grammars" rule does the rest.
//
// The frozen syntax is `docs/design/spec-dsl-examples/README.md`; the member
// vocabulary below is its coverage matrix, one member per property word.
//
// Every field a class system uses to manufacture and dispatch instances is
// empty here — a spec pack creates no objects — so consumers keyed on those
// fields see nothing, and `DefinerFamily::SpecTcl` is the one-word way for a
// consumer that must not treat this as a class system to say so.

/// A `SpecTcl` block statement that names something and then takes a block:
/// `command NAME { … }`, `values NAME { … }`, `subcommand NAME { … }`.
pub(super) const SPECTCL_NAMED_BLOCK_ROLES: &[(u8, ArgRole)] =
    &[(0, ArgRole::Name), (1, ArgRole::Body)];

/// `hook NAME {params} { … }` — a shared hook body declared pack-wide, and
/// the same shape as a `proc`: name, parameter list, body.
pub(super) const SPECTCL_NAMED_HOOK_ROLES: &[(u8, ArgRole)] = &[
    (0, ArgRole::Name),
    (1, ArgRole::ParamList),
    (2, ArgRole::Body),
];

/// A hook *property* statement — `const_fold {words ctx} { … }`.  The
/// `-native ID` spelling of the same statement carries no parameter list or
/// body; a walker only ever applies the `ParamList` / `Body` roles to a
/// braced word, so the reference form is left alone by construction.
pub(super) const SPECTCL_HOOK_ROLES: &[(u8, ArgRole)] =
    &[(0, ArgRole::ParamList), (1, ArgRole::Body)];

/// `descriptor KEY NAME { … }` — the schema key it is a descriptor *for*,
/// the name it is declared under, then the block.
pub(super) const SPECTCL_DESCRIPTOR_ROLES: &[(u8, ArgRole)] = &[
    (0, ArgRole::Keyword),
    (1, ArgRole::Name),
    (2, ArgRole::Body),
];

/// `command NAME ?-override? { … }` — the one pack-level statement with an
/// optional word between its name and its block.  `-override` claims a name
/// a shipped spec already has (the load-policy section of the syntax memo),
/// and inserting it shifts the block one place right, which is exactly what
/// [`OptionalMemberArgument`] models.
pub(super) const SPECTCL_OVERRIDE_ARGUMENT: OptionalMemberArgument = OptionalMemberArgument {
    position: 1,
    values: &[MemberOptionValue {
        value: "-override",
        role: ArgRole::Option,
        surface: None,
        declared_visibility: None,
    }],
};

/// The five pack-level statements of a `speclib` body.
pub(super) const SPECTCL_PACK_MEMBERS: &[MemberSpec] = &[
    setting("command", SPECTCL_NAMED_BLOCK_ROLES).optional_argument(SPECTCL_OVERRIDE_ARGUMENT),
    setting("values", SPECTCL_NAMED_BLOCK_ROLES),
    setting("hook", SPECTCL_NAMED_HOOK_ROLES),
    setting("descriptor", SPECTCL_DESCRIPTOR_ROLES),
    // `default KEY VALUE…` sets one pack-wide availability/identity key; it
    // declares no name of its own and holds no script.
    setting_word("default"),
];

/// The declaration keys of a `command` / `subcommand` body.
///
/// One member per DSL property word of the syntax memo's coverage matrix.
/// The two tables (`CommandSpec` and `SubCommand`) are unioned into one
/// grammar rather than split: they overlap on all but a handful of words,
/// and a `subcommand` block is lexically identical to a `command` block, so
/// splitting would buy nothing a loader does not already have to check.
pub(super) const SPECTCL_COMMAND_MEMBERS: &[MemberSpec] = &[
    // Nested blocks: each also a `CommandSpec` carrying the inner grammar,
    // so recursing into one switches vocabulary.
    setting("subcommand", SPECTCL_NAMED_BLOCK_ROLES),
    setting("hover", BODY0_ROLES),
    // `values NAME { … }` is deliberately absent: the memo makes the shared
    // value table a *pack-level* declaration, referenced from here by
    // `arg -values-from NAME` / `option -values-from NAME`.
    setting("case_list", BODY0_ROLES),
    setting("clause_grammar", BODY0_ROLES),
    setting("event_requires", BODY0_ROLES),
    setting("world_effects", BODY0_ROLES),
    setting("state_transitions", BODY0_ROLES),
    setting("definition_body", BODY0_ROLES),
    setting("body_scope", BODY0_ROLES),
    // `object_class NAME ?-superclass {…}? ?-allow-unknown? { … }` is
    // deliberately NOT a member row. Its block's index is a function of two
    // optional flags of *different widths*, which `OptionalMemberArgument`
    // (one optional word, one position) cannot express — and a member layout
    // that guessed index 1 would paint `-superclass`'s braced class list as a
    // definition block. It is instead a plain registered statement carrying
    // an `arg_role_resolver`, which reads the whole call; the word still
    // paints as a keyword (`Traits::LANGUAGE_KEYWORD`) and its block still
    // switches grammars (`CommandSpec::definition_body`), so nothing is lost
    // but the wrong answer.
    // Hook bodies: a proc-shaped `{words ctx} { … }` pair.
    setting("arg_role_resolver", SPECTCL_HOOK_ROLES),
    setting("command_prefix_resolver", SPECTCL_HOOK_ROLES),
    setting("const_fold", SPECTCL_HOOK_ROLES),
    setting("const_fold_versioned", SPECTCL_HOOK_ROLES),
    setting("taint_sink_gate", SPECTCL_HOOK_ROLES),
    setting("context_gate", SPECTCL_HOOK_ROLES),
    setting("literal_argument_validator", SPECTCL_HOOK_ROLES),
    setting("clause_shape_check", SPECTCL_HOOK_ROLES),
    // Row statements, under the singular-row rule: a field holding a list
    // of rows gets a singular statement, never a nested block.
    setting_word("arg"),
    setting_word("option"),
    setting_word("option_conflict"),
    setting_word("form"),
    setting_word("side_effect"),
    setting_word("repeat"),
    setting_word("manufacturer"),
    setting_word("setter_constraint"),
    setting_word("sub_subcommand"),
    setting_word("oo_context_fact"),
    setting_word("versioned_arg_value"),
    setting_word("event_requirement_form"),
    setting_word("defines_symbol"),
    setting_word("binds_handle"),
    setting_word("frame_effect"),
    setting_word("byte_array_payload"),
    setting_word("deprecation_fix"),
    setting_word("event_handler_priority"),
    // Scalar property words.
    setting_word("traits"),
    setting_word("dialects"),
    setting_word("arity"),
    setting_word("detail"),
    setting_word("synopsis"),
    setting_word("return_type"),
    setting_word("var_write_typing"),
    setting_word("return_elements"),
    setting_word("var_elements_effect"),
    setting_word("representation_effect"),
    setting_word("allow_unknown_subcommands"),
    setting_word("prefix_matching"),
    setting_word("default_form_first_word"),
    setting_word("semantic_operation"),
    setting_word("assigns_variable_at"),
    setting_word("safe_on_uninit"),
    setting_word("lowering_hook"),
    setting_word("codegen_hook"),
    setting_word("inline_codegen_hook"),
    setting_word("analyser_hook"),
    setting_word("bpf_op"),
    setting_word("data_collection"),
    setting_word("command_table_effect"),
    setting_word("result_stability"),
    setting_word("inferred_storage_type"),
    setting_word("required_package"),
    setting_word("excluded_events"),
    setting_word("unsafe_command"),
    setting_word("side_switch_target"),
    setting_word("reserved_trailing_words"),
    setting_word("body_kind"),
    setting_word("body_arg_implicit_args"),
    setting_word("taint_output_sink"),
    setting_word("taint_output_sink_subcommands"),
    setting_word("taint_log_sink"),
    setting_word("taint_network_sink_args"),
    setting_word("taint_code_sink_args"),
    setting_word("taint_numeric_coercion"),
    setting_word("taint_interp_eval_subcommands"),
    setting_word("taint_source"),
    setting_word("taint_transform"),
    setting_word("taint_double_encode_colour"),
    setting_word("taint_sink_safe_colour"),
    setting_word("channel_configuration"),
    setting_word("credential_options"),
    setting_word("credential_arg"),
    setting_word("header_write"),
    setting_word("sensitive_headers"),
    setting_word("pattern_type"),
    setting_word("format_string_type"),
    setting_word("tcllib_package"),
    setting_word("introduced_version"),
    setting_word("deprecated_version"),
    setting_word("retired_version"),
    setting_word("warn_missing_import"),
    setting_word("is_namespace_exported"),
    setting_word("xc_translatable"),
    setting_word("deprecated_replacement"),
    setting_word("deprecated_replacement_drop_in"),
    setting_word("byte_array_effect"),
    setting_word("self_receiver_words"),
    setting_word("creates_instance_at"),
    setting_word("defines_command_at"),
    setting_word("implementation_namespace"),
    setting_word("successful_handler"),
    setting_word("source_deprecation_advice"),
    // `SubCommand`-only keys.
    setting_word("pure"),
    setting_word("mutator"),
    setting_word("min_abbrev"),
    setting_word("loop_list_header"),
    setting_word("creates_scope_alias"),
    setting_word("arg_values_accept_prefix"),
    setting_word("destructive"),
    setting_word("returns_path"),
    setting_word("is_unescape"),
    setting_word("cfg_rewrite_name"),
    setting_word("max_leading_option_words"),
];

/// The six documentation keys of a `hover { … }` block.  `synopsis` and
/// `example` are repeatable and keep their order; `description` / `example`
/// / `returns` are the three keys the memo deliberately renames from their
/// Rust field names (`snippet` / `examples` / `return_value`).
pub(super) const SPECTCL_HOVER_MEMBERS: &[MemberSpec] = &[
    setting_word("summary"),
    setting_word("synopsis"),
    setting_word("description"),
    setting_word("source"),
    setting_word("example"),
    setting_word("returns"),
];

/// A row whose first word is the thing it declares (`value V …`).
pub(super) const SPECTCL_NAME0_ROLES: &[(u8, ArgRole)] = &[(0, ArgRole::Name)];

/// The one repeatable row of a `values NAME { … }` table:
/// `value V ?-detail {…}? ?-min-tcl VER? ?-code N?`.
pub(super) const SPECTCL_VALUES_MEMBERS: &[MemberSpec] = &[setting("value", SPECTCL_NAME0_ROLES)];

/// The rows of a `clause_grammar { … }` block.  `head` / `repeated` /
/// `once` / `tail` each carry a braced *slot list*, which is a role
/// vocabulary rather than a script, and `group` a layout index, so no member
/// declares a `Body`; the last three state the chain-level rules.
pub(super) const SPECTCL_CLAUSE_GRAMMAR_MEMBERS: &[MemberSpec] = &[
    setting_word("head"),
    setting_word("repeated"),
    setting_word("once"),
    setting_word("group"),
    setting_word("tail"),
    setting_word("fallthrough_body"),
    setting_word("default_clause"),
    setting_word("selection"),
];

/// The plain-data fields of a `case_list { … }` block. The command-level
/// switches that pick the match mode, fold case, or end the option run are
/// the command's own option rows, each declaring its effect.
pub(super) const SPECTCL_CASE_LIST_MEMBERS: &[MemberSpec] = &[
    setting_word("subject_args"),
    setting_word("two_arg_optionless_surface"),
    setting_word("fallthrough_body"),
    setting_word("value_options_require_regex"),
    setting_word("clause_flags"),
    setting_word("clause_regex_flag"),
    setting_word("clause_value_flags"),
    setting_word("clause_end_options_flag"),
    setting_word("clause_force_inline_flag"),
    setting_word("clause_force_list_flag"),
    setting_word("clause_force_list_shape"),
    setting_word("allow_omitted_final_body"),
    setting_word("keyword_patterns"),
    setting_word("warn_unbraced_bodies"),
    setting_word("default_mode"),
    setting_word("pattern_words"),
    setting_word("exhaustive_keyword_patterns"),
    setting_word("optional_subject_separator"),
    setting_word("special_match_options"),
];

/// The six scalars of an `event_requires { … }` block.
pub(super) const SPECTCL_EVENT_REQUIRES_MEMBERS: &[MemberSpec] = &[
    setting_word("client_side"),
    setting_word("server_side"),
    setting_word("transport"),
    setting_word("profiles"),
    setting_word("also_in"),
    setting_word("flow"),
];

/// The rows of a `world_effects { … }` block.  `resolver` is reference-only
/// (`-native ID`, `none`, or a derivation keyword) — the memo excludes an
/// authored resolver, but the *word* is still part of the block's grammar.
pub(super) const SPECTCL_WORLD_EFFECTS_MEMBERS: &[MemberSpec] = &[
    setting_word("composition"),
    setting_word("access"),
    setting_word("callback"),
    setting_word("resolver"),
    setting_word("dynamic_fallback"),
];

/// The rows of a `state_transitions { … }` block.
pub(super) const SPECTCL_STATE_TRANSITIONS_MEMBERS: &[MemberSpec] = &[
    setting_word("composition"),
    setting_word("argument_shape"),
    setting_word("resolver"),
    setting_word("widen"),
    setting_word("covers"),
    setting_word("commit"),
];

/// The rows of a `definition_body { … }` block — `DefinitionBodyGrammar`'s
/// own fields, which is what makes the DSL able to describe *this* module's
/// data structure in itself.
pub(super) const SPECTCL_DEFINITION_BODY_MEMBERS: &[MemberSpec] = &[
    setting_word("family"),
    setting_word("member"),
    setting_word("member_option"),
    setting_word("implicit_vars"),
    setting_word("implicit_receiver_vars"),
    setting_word("member_body_namespace_path"),
    setting_word("builtin_type_methods"),
    setting_word("builtin_object_method"),
    setting_word("builtin_terminating_methods"),
    setting_word("member_body_command"),
    setting_word("bare_word_construction"),
    setting_word("dynamic_method_dispatch"),
    setting_word("manufacturer"),
    setting_word("unknown_dispatch_method"),
    setting_word("property_accessor_methods"),
];

/// The rows of a `body_scope { … }` block.  Its `command NAME { … }` is a
/// *scoped-command* row, not a pack-level command declaration — the same
/// context rule that makes `method` a member row inside `object_class` —
/// which is why it lives in this grammar with its own roles rather than
/// being borrowed from the pack grammar.
pub(super) const SPECTCL_BODY_SCOPE_MEMBERS: &[MemberSpec] = &[
    setting_word("name"),
    setting_word("include_sibling_definitions"),
    setting_word("allow_unknown_commands"),
    setting("command", SPECTCL_NAMED_BLOCK_ROLES),
];

/// The rows of an `object_class NAME { … }` block: `method NAME { … }`
/// bodies, which reuse the `subcommand` body grammar unchanged.
pub(super) const SPECTCL_OBJECT_CLASS_MEMBERS: &[MemberSpec] =
    &[setting("method", SPECTCL_NAMED_BLOCK_ROLES)];

/// Build one `SpecTcl` grammar from its member table.  Every field a class
/// system uses is empty: a spec pack manufactures nothing, dispatches
/// nothing, and injects no commands or variables into its blocks.
pub(super) const fn spectcl_grammar(members: &'static [MemberSpec]) -> DefinitionBodyGrammar {
    DefinitionBodyGrammar {
        family: DefinerFamily::SpecTcl,
        members,
        implicit_vars: &[],
        implicit_receiver_vars: &[],
        member_body_namespace_path: &[],
        builtin_type_methods: &[],
        builtin_object_methods: &[],
        builtin_terminating_methods: &[],
        member_body_commands: &[],
        bare_word_construction: false,
        bare_word_construction_hint: None,
        bare_word_construction_source_hint: None,
        dynamic_method_dispatch: false,
        manufacturers: &[],
        unknown_dispatch_method: None,
        property_accessor_methods: &[],
    }
}

/// The body of `speclib NAME VERSION { … }` — the pack itself.
pub const SPECTCL_PACK_GRAMMAR: DefinitionBodyGrammar = spectcl_grammar(SPECTCL_PACK_MEMBERS);

/// The body of `command NAME { … }` and of `subcommand NAME { … }`.
pub const SPECTCL_COMMAND_GRAMMAR: DefinitionBodyGrammar = spectcl_grammar(SPECTCL_COMMAND_MEMBERS);

/// The body of `hover { … }`.
pub const SPECTCL_HOVER_GRAMMAR: DefinitionBodyGrammar = spectcl_grammar(SPECTCL_HOVER_MEMBERS);

/// The body of `values NAME { … }`.
pub const SPECTCL_VALUES_GRAMMAR: DefinitionBodyGrammar = spectcl_grammar(SPECTCL_VALUES_MEMBERS);

/// The body of `clause_grammar { … }`.
pub const SPECTCL_CLAUSE_GRAMMAR_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_CLAUSE_GRAMMAR_MEMBERS);

/// The body of `case_list { … }`.
pub const SPECTCL_CASE_LIST_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_CASE_LIST_MEMBERS);

/// The body of `event_requires { … }`.
pub const SPECTCL_EVENT_REQUIRES_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_EVENT_REQUIRES_MEMBERS);

/// The body of `world_effects { … }`.
pub const SPECTCL_WORLD_EFFECTS_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_WORLD_EFFECTS_MEMBERS);

/// The body of `state_transitions { … }`.
pub const SPECTCL_STATE_TRANSITIONS_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_STATE_TRANSITIONS_MEMBERS);

/// The body of `definition_body { … }`.
pub const SPECTCL_DEFINITION_BODY_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_DEFINITION_BODY_MEMBERS);

/// The body of `body_scope { … }`.
pub const SPECTCL_BODY_SCOPE_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_BODY_SCOPE_MEMBERS);

/// The body of `object_class NAME { … }`.
pub const SPECTCL_OBJECT_CLASS_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_OBJECT_CLASS_MEMBERS);

/// The one declaration legal at the **root** of a `.tclspec` pack.
///
/// `speclib` is the DSL's only possible top-level word (`spec-packs.md`), so
/// the document grammar is a single row — and that is what lets completion
/// offer it, and only it, at the root of a pack.
pub(super) const SPECTCL_DOCUMENT_MEMBERS: &[MemberSpec] =
    &[setting("speclib", SPECTCL_SPECLIB_ROLES)];

/// `speclib NAME DSL-VERSION { … }` — the name first, the body third.
pub(super) const SPECTCL_SPECLIB_ROLES: &[(u8, ArgRole)] =
    &[(0, ArgRole::Name), (2, ArgRole::Body)];

/// The body of a `.tclspec` **document** — see [`SPECTCL_DOCUMENT_MEMBERS`].
pub const SPECTCL_DOCUMENT_GRAMMAR: DefinitionBodyGrammar =
    spectcl_grammar(SPECTCL_DOCUMENT_MEMBERS);

/// Every `SpecTcl` grammar, so a sweep can assert the family's invariants
/// without enumerating the constants by hand.
pub const SPECTCL_GRAMMARS: &[&DefinitionBodyGrammar] = &[
    &SPECTCL_DOCUMENT_GRAMMAR,
    &SPECTCL_PACK_GRAMMAR,
    &SPECTCL_COMMAND_GRAMMAR,
    &SPECTCL_HOVER_GRAMMAR,
    &SPECTCL_VALUES_GRAMMAR,
    &SPECTCL_CLAUSE_GRAMMAR_GRAMMAR,
    &SPECTCL_CASE_LIST_GRAMMAR,
    &SPECTCL_EVENT_REQUIRES_GRAMMAR,
    &SPECTCL_WORLD_EFFECTS_GRAMMAR,
    &SPECTCL_STATE_TRANSITIONS_GRAMMAR,
    &SPECTCL_DEFINITION_BODY_GRAMMAR,
    &SPECTCL_BODY_SCOPE_GRAMMAR,
    &SPECTCL_OBJECT_CLASS_GRAMMAR,
];
