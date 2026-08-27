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

//! The `SpecTcl` **scalar property** statements — one word per schema key that
//! holds a single value.
//!
//! spec-packs.md promises "that key is the DSL property name", and this table
//! is where that promise is kept: every scalar key of both schema tables, in
//! coverage-matrix order, with the arity its value shape implies. A new
//! `CommandSpec` field becomes one row here, never a new module — which is
//! also what lets the studio's schema-coverage gate be the thing that catches
//! a missing word.
//!
//! Words that only mean something *inside* a nested block (`summary` inside
//! `hover`, `family` inside `definition_body`, `widen` inside
//! `state_transitions`) deliberately get **no** `CommandSpec`: they are
//! members of their block's grammar and nothing else, exactly as `method` is
//! a member of the snit grammar and not a command. That is what stops
//! `source` — a real Tcl command — from being shadowed by a hover key, and it
//! is the whole reason the vocabulary is context-sensitive.
use crate::arity::Arity;
use crate::spec::CommandSpec;

/// `(word, arity, summary, snippet)`.
type Row = (&'static str, Arity, &'static str, &'static str);

/// A property word taking exactly one value word.
const fn one(name: &'static str, summary: &'static str, snippet: &'static str) -> Row {
    (name, Arity::exact(1), summary, snippet)
}

/// A property word whose value is an optional `yes` / `no` — the bare word
/// means `yes`, and *absent* means the field's default.
const fn boolean(name: &'static str, summary: &'static str) -> Row {
    (
        name,
        Arity::new(0, 1),
        summary,
        "A bool: the bare word means `yes`; absent means the field's default.",
    )
}

/// A property word whose value is a variant word followed by its payload
/// fields in declaration order.
const fn variant(name: &'static str, summary: &'static str) -> Row {
    (
        name,
        Arity::at_least(1),
        summary,
        "An enum with a payload: the variant word first, then its fields in declaration order.",
    )
}

/// `<field> -native <id>` — a closed compiler-hook catalogue. A pack may
/// *name* one; it cannot add to one.
const fn native(name: &'static str, summary: &'static str) -> Row {
    (
        name,
        Arity::exact(2),
        summary,
        "A closed compiler-hook catalogue: `-native ID` names a shipped implementation. A pack reuses named hooks, it cannot add to them; naming a lowering or codegen hook is reported at load, since it changes how the compiler translates the command rather than what the editor knows about it.",
    )
}

const ROWS: &[Row] = &[
    // --- identity / availability -----------------------------------------
    (
        "traits",
        Arity::exact(1),
        "Declare the command's trait bits.",
        "Trait words verbatim from the traits vocabulary; repeatable, and unioned.",
    ),
    (
        "dialects",
        Arity::exact(1),
        "Declare which dialects the command exists in.",
        "Members verbatim (`tcl8.4` … `tcl9.1`, `f5-irules`, `f5-iapps`, `tk`, `expect`, `bpf`, `f5-tmsh`, `f5-bigip`), plus two shorthands: a `+` suffix on a Tcl version means \"and later\", and `all-tcl` / `tcl8.x` name the two common closures. On a `subcommand`, absent inherits the parent command's set.",
    ),
    (
        "available",
        Arity::at_least(1),
        "Declare where the command is available, on the provider's own axis.",
        "`SpecTcl` 2.0's spelling of `dialects`, and the same claim: one or more `{PROVIDER SPEC…}` rows, where PROVIDER is `tcl RANGE`, `f5-irules`, `jim RANGE`, or `package NAME ?RANGE?` and RANGE is Tcl requirement syntax — `8.6-` for \"and later\", `8.4-9.0` for a half-open window (maximum exclusive), or a bare `8.5` for that one release line. `available {tcl 8.6-} {package Tk}`. Both spellings load to the same spec, so a pack may use either; `tcl spec upgrade` rewrites 1.x sources mechanically.",
    ),
    (
        "arity",
        Arity::at_least(1),
        "Declare how many argument words the call takes.",
        "One range word plus modifiers: `3`, `1..2`, `1..`, `..2`, `..`, then optional `-step S` and `-also N`. Counted after the command name, or after the subcommand word inside a `subcommand` block. SpecTcl 1.2 adds the three lifecycle flags (`-introduced` / `-deprecated` / `-retired`), which turn the row into a per-release signature *window*; a bare row with no lifecycle stays the unbounded default, and the window the resolved package floor covers is the one the arity check uses.",
    ),
    one(
        "required_package",
        "The package that must be loaded for the command to exist.",
        "Also settable pack-wide with `default required_package NAME`.",
    ),
    (
        "tk_geometry",
        Arity::new(1, 3),
        "Declare static Tk geometry-manager container semantics.",
        "`Exclusive` or `Independent`, optionally followed by `-container-option OPTION` (normally `-in`). SpecTcl 1.2.",
    ),
    one(
        "tcllib_package",
        "The tcllib package the command belongs to.",
        "",
    ),
    one(
        "implementation_namespace",
        "The namespace the ensemble's implementation lives in.",
        "",
    ),
    boolean(
        "warn_missing_import",
        "Warn when the command is used without its import.",
    ),
    boolean(
        "is_namespace_exported",
        "Whether the command's namespace exports it.",
    ),
    one(
        "introduced_version",
        "The package version the command first appears in.",
        "`Lifecycle.introduced` — the owning *package*'s version axis, orthogonal to `dialects`.",
    ),
    one(
        "deprecated_version",
        "The package version the command is deprecated in.",
        "`Lifecycle.deprecated`.",
    ),
    one(
        "retired_version",
        "The package version the command is removed in.",
        "`Lifecycle.retired`.",
    ),
    one(
        "deprecated_replacement",
        "The command that replaces this deprecated one.",
        "",
    ),
    boolean(
        "deprecated_replacement_drop_in",
        "Whether the replacement is a drop-in rename.",
    ),
    // --- documentation ----------------------------------------------------
    one(
        "detail",
        "The subcommand's one-line description.",
        "Braced prose. Every byte between the braces is the value, newlines included — nothing is stripped, joined, dedented, or wrapped, because the equivalence gate compares the loaded field with the compiled string byte for byte.",
    ),
    one(
        "synopsis",
        "The subcommand's synopsis line.",
        "`SubCommand::synopsis` is a single string. The *repeatable* `synopsis` row is the one inside a `hover` block, where the field really is a list.",
    ),
    // --- shape ------------------------------------------------------------
    boolean(
        "allow_unknown_subcommands",
        "Whether an unrecognised subcommand word is accepted.",
    ),
    one(
        "prefix_matching",
        "How subcommand abbreviation resolves.",
        "`Enabled` (any unique prefix) or `Strict`.",
    ),
    one("min_abbrev", "Documented minimum abbreviation length.", ""),
    one(
        "default_form_first_word",
        "The word kind that selects the default form.",
        "",
    ),
    one(
        "reserved_trailing_words",
        "How many trailing words the command reserves.",
        "",
    ),
    one(
        "max_leading_option_words",
        "How many leading option words may precede the operands.",
        "",
    ),
    boolean(
        "arg_values_accept_prefix",
        "Whether a declared argument value resolves by unique prefix.",
    ),
    // --- results and typing -----------------------------------------------
    one("return_type", "The command's result type.", ""),
    variant("var_write_typing", "How a written variable is typed."),
    variant("return_elements", "How the result's elements are typed."),
    variant(
        "var_elements_effect",
        "How the command changes a variable's element typing.",
    ),
    variant(
        "representation_effect",
        "How the command changes a value's internal representation.",
    ),
    variant(
        "byte_array_effect",
        "What the command does to a byte array.",
    ),
    one("inferred_storage_type", "`Dict`, `List`, or `Array`.", ""),
    variant("result_stability", "How stable the command's result is."),
    one(
        "assigns_variable_at",
        "The argument index naming a variable the command assigns.",
        "",
    ),
    one(
        "safe_on_uninit",
        "Dialects in which reading an uninitialised variable here is safe.",
        "",
    ),
    one(
        "creates_instance_at",
        "The argument index naming an instance the command creates.",
        "",
    ),
    one(
        "defines_command_at",
        "The argument index naming a command the command defines.",
        "",
    ),
    one("self_receiver_words", "Words that dispatch on `self`.", ""),
    // --- bodies -----------------------------------------------------------
    one(
        "body_kind",
        "Whether a body argument runs in the caller's frame.",
        "`Plain` shares the caller's frame; `Structural` is a separate definition / dispatch scope.",
    ),
    one(
        "body_arg_implicit_args",
        "How many arguments the body is invoked with implicitly.",
        "",
    ),
    boolean(
        "loop_list_header",
        "Whether the call heads a loop over a list.",
    ),
    boolean(
        "creates_scope_alias",
        "Whether the call aliases a caller variable into this scope.",
    ),
    // --- purity and effects ------------------------------------------------
    boolean("pure", "Whether the subcommand is free of side effects."),
    boolean("mutator", "Whether the subcommand mutates its subject."),
    boolean(
        "destructive",
        "Whether the subcommand destroys its subject.",
    ),
    boolean("returns_path", "Whether the subcommand's result is a path."),
    boolean(
        "is_unescape",
        "Whether the subcommand reverses an escaping.",
    ),
    boolean(
        "unsafe_command",
        "Whether the command is unsafe in a safe interp.",
    ),
    one(
        "command_table_effect",
        "How the call changes the interpreter's command table.",
        "`DefinesProcedure`, `RenamesCommands`, or `CreatesAliases`.",
    ),
    one(
        "cfg_rewrite_name",
        "The `::tcl::…` name this rewrites to.",
        "",
    ),
    // --- compiler-facing --------------------------------------------------
    variant(
        "semantic_operation",
        "The operation identity the command performs.",
    ),
    native("lowering_hook", "The shipped lowering hook, by name."),
    native("codegen_hook", "The shipped codegen hook, by name."),
    native(
        "inline_codegen_hook",
        "The shipped inline-codegen hook, by name.",
    ),
    native("analyser_hook", "The shipped analyser hook, by name."),
    native("bpf_op", "The shipped BPF operation, by name."),
    native(
        "data_collection",
        "The shipped collect/release descriptor, by name.",
    ),
    // --- patterns and formats ----------------------------------------------
    one("pattern_type", "`Glob` or `Regex`.", ""),
    one(
        "format_string_type",
        "`Sprintf`, `Clock`, `Binary`, or `Regsub`.",
        "",
    ),
    // --- events ------------------------------------------------------------
    one(
        "excluded_events",
        "Events the command may not be called from.",
        "",
    ),
    one("side_switch_target", "`Client` or `Server`.", ""),
    // --- taint --------------------------------------------------------------
    one(
        "taint_output_sink",
        "The diagnostic code of the output sink.",
        "",
    ),
    one(
        "taint_output_sink_subcommands",
        "Which subcommands are output sinks.",
        "",
    ),
    one("taint_log_sink", "The diagnostic code of the log sink.", ""),
    one(
        "taint_network_sink_args",
        "Argument indices that reach the network.",
        "Tri-state: absent is *unset*, `{}` is *declared empty*.",
    ),
    one(
        "taint_code_sink_args",
        "Argument indices evaluated as code.",
        "Tri-state: absent is *unset*, `{}` is *declared empty*.",
    ),
    one(
        "taint_interp_eval_subcommands",
        "Subcommands that evaluate code in another interp.",
        "",
    ),
    one(
        "taint_source",
        "Colours the command's result is tainted with.",
        "",
    ),
    one("taint_transform", "Colours the command transforms.", ""),
    one(
        "taint_double_encode_colour",
        "Colours whose double encoding is a finding.",
        "",
    ),
    one(
        "taint_sink_safe_colour",
        "Colours that make the sink safe.",
        "",
    ),
    one("credential_options", "Options that carry a credential.", ""),
    one(
        "credential_arg",
        "The argument index carrying a credential.",
        "**Verbatim coordinates**: the W310 consumer counts the subcommand word itself as index 0, unlike every other per-index subcommand field, so a loader must store this value as written and never re-base it.",
    ),
    one(
        "sensitive_headers",
        "Header names treated as sensitive.",
        "",
    ),
    // --- translation --------------------------------------------------------
    (
        "xc_translatable",
        Arity::exact(1),
        "Whether the command is translatable.",
        "Tri-state: the argument is **required** — absent is `unset`, which is not `false`.",
    ),
];

pub(super) fn specs() -> Vec<CommandSpec> {
    ROWS.iter()
        .map(|&(name, arity, summary, snippet)| super::statement(name, arity, summary, snippet))
        .collect()
}
