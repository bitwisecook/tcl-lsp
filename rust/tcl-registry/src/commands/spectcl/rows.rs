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

//! The `SpecTcl` **row** statements.
//!
//! The syntax memo's singular-row rule: a schema key holding a *list of rows*
//! gets a singular row statement (`options` → `option`, `subcommands` →
//! `subcommand`, `manufacturer_methods` → `manufacturer`, …), and a new field
//! on a row type becomes a new flag on that statement — so the
//! unknown-word tolerance rule keeps working in both directions. A literal
//! `options { … }` block per key was rejected: it nests one level deeper for
//! no gain and makes every option a two-line edit.
//!
//! Rows carry flags rather than blocks, so their vocabulary is an
//! [`OptionSpec`] table — which is what makes flag completion, flag hover and
//! unknown-flag reporting work on them with no new machinery.
use crate::prelude::*;

use super::SOURCE;
use tcl_dialect::model::SpecSurface;

/// A row statement: a leading subject word, then flags.
fn row(
    name: &'static str,
    arity: Arity,
    arg_roles: &'static [(u8, ArgRole)],
    options: &'static [OptionSpec],
    summary: &'static str,
    snippet: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::LANGUAGE_KEYWORD,
        surface: Some(SpecSurface::SPECTCL),
        arity,
        hover: Some(HoverSnippet {
            summary,
            synopsis: &[],
            snippet,
            source: SOURCE,
            examples: "",
            return_value: "",
        }),
        options,
        arg_roles,
        ..CommandSpec::DEFAULT
    }
}

/// The subject-word role tables the rows below use.
const KEYWORD0: &[(u8, ArgRole)] = &[(0, ArgRole::Keyword)];
const NAME0: &[(u8, ArgRole)] = &[(0, ArgRole::Name)];
const INDEX0: &[(u8, ArgRole)] = &[(0, ArgRole::Index)];
const VALUE0: &[(u8, ArgRole)] = &[(0, ArgRole::Value)];

/// A flag that takes exactly one word.
const fn valued(name: &'static str, hint: &'static str, detail: &'static str) -> OptionSpec {
    OptionSpec {
        name,
        value: OptionValue::value(hint),
        detail,
        ..OptionSpec::DEFAULT
    }
}

/// A flag that takes no word.
const fn flag(name: &'static str, detail: &'static str) -> OptionSpec {
    OptionSpec {
        name,
        detail,
        ..OptionSpec::DEFAULT
    }
}

const FORM_OPTIONS: &[OptionSpec] = &[valued(
    "-dialects",
    "set",
    "restrict this form to a dialect set",
)];

const SIDE_EFFECT_OPTIONS: &[OptionSpec] = &[
    flag("-reads", "the command reads the target"),
    flag("-writes", "the command writes the target"),
    valued(
        "-side",
        "Client|Server|Both|None",
        "the connection side affected",
    ),
    valued("-dialects", "set", "restrict this effect to a dialect set"),
];

const REPEAT_OPTIONS: &[OptionSpec] = &[
    valued("-from", "n", "first index the layout covers"),
    valued("-stride", "n", "distance between covered indices"),
    valued(
        "-exclude-trailing",
        "n",
        "trailing words the layout does not cover",
    ),
    flag("-optional-leading", "the layout may start one word later"),
    flag("-conditional", "the layout applies only in some forms"),
];

const MANUFACTURER_OPTIONS: &[OptionSpec] = &[
    flag(
        "-unexported",
        "the method exists but ordinary dispatch cannot reach it",
    ),
    valued(
        "-names-instance-at",
        "n",
        "the word naming the new instance",
    ),
    valued(
        "-definition-body-at",
        "n",
        "the word carrying the definition body",
    ),
    valued(
        "-constructor-args-from",
        "n",
        "first word of the constructor's own arguments",
    ),
];

const OPTION_CONFLICT_OPTIONS: &[OptionSpec] = &[
    valued(
        "-dialects",
        "set",
        "restrict this relation to a dialect set",
    ),
    valued(
        "-message",
        "prose",
        "the library's own error text, quoted instead of generated",
    ),
];

const SETTER_CONSTRAINT_OPTIONS: &[OptionSpec] = &[
    valued("-prefix", "text", "the prefix the value must carry"),
    valued("-code", "CODE", "diagnostic code reported on violation"),
    valued(
        "-message",
        "prose",
        "diagnostic message reported on violation",
    ),
];

const SUB_SUBCOMMAND_OPTIONS: &[OptionSpec] = &[
    valued(
        "-detail",
        "prose",
        "the second-level word's one-line description",
    ),
    valued("-synopsis", "text", "the second-level word's synopsis"),
    valued("-dialects", "set", "restrict this word to a dialect set"),
];

const VERSIONED_ARG_VALUE_OPTIONS: &[OptionSpec] = &[
    valued("-introduced", "version", "Lifecycle.introduced"),
    valued("-deprecated", "version", "Lifecycle.deprecated"),
    valued("-retired", "version", "Lifecycle.retired"),
];

const EVENT_REQUIREMENT_FORM_OPTIONS: &[OptionSpec] = &[valued(
    "-only-in",
    "events",
    "events this form is restricted to",
)];

const DEFINES_SYMBOL_OPTIONS: &[OptionSpec] = &[
    valued("-name-arg", "n", "the word naming the defined symbol"),
    valued("-detail-arg", "n", "the word carrying the symbol's detail"),
    valued(
        "-requires-arg",
        "n",
        "the word naming what the symbol requires",
    ),
    valued("-kind", "KIND", "the kind of symbol defined"),
];

const BINDS_HANDLE_OPTIONS: &[OptionSpec] = &[
    valued(
        "-name-from",
        "{Word N}",
        "where the handle's name comes from",
    ),
    valued(
        "-class-from",
        "{Word N}",
        "where the handle's class comes from",
    ),
    valued("-keyword", "{N WORD}", "a keyword word the form requires"),
];

const FRAME_EFFECT_OPTIONS: &[OptionSpec] = &[
    valued(
        "-level-word",
        "policy",
        "which word (if any) selects the frame",
    ),
    valued("-layout", "layout", "how the remaining words are read"),
];

const BYTE_ARRAY_PAYLOAD_OPTIONS: &[OptionSpec] = &[
    valued("-replace-data-index", "n", "the payload word"),
    flag(
        "-message-flag-shift",
        "a leading message flag shifts the payload word",
    ),
];

const DEPRECATION_FIX_OPTIONS: &[OptionSpec] = &[
    valued("-replace", "word", "the replacement command word"),
    valued("-description", "prose", "what the fix does"),
    valued("-safety", "safety", "how safe the automatic fix is"),
];

const EVENT_HANDLER_PRIORITY_OPTIONS: &[OptionSpec] = &[
    valued("-default", "n", "the default handler priority"),
    flag("-warn-implicit", "warn when the priority is left implicit"),
];

/// The rows that describe a command's *call shape* — its forms, effects,
/// argument layouts, and option constraints.
fn call_shape_rows() -> Vec<CommandSpec> {
    vec![
        row(
            "form",
            Arity::at_least(2),
            KEYWORD0,
            FORM_OPTIONS,
            "Declare one documented call form.",
            "`form KIND {synopsis} ?-dialects {…}?`. This row documents synopsis and lifecycle only. The structured `command_forms` descriptor also carries semantic and native compiler routing; it is excluded from SpecTcl as one all-or-nothing value rather than partially authored here.",
        ),
        row(
            "side_effect",
            Arity::at_least(1),
            KEYWORD0,
            SIDE_EFFECT_OPTIONS,
            "Declare one side effect of the command.",
            "One row per effect: the target, then whether it is read, written, and on which connection side.",
        ),
        row(
            "repeat",
            Arity::at_least(1),
            KEYWORD0,
            REPEAT_OPTIONS,
            "Declare a role that recurs at a fixed stride over the argument tail.",
            "One row per layout — `global a b c`, `foreach v1 l1 v2 l2 body`, `upvar ?level? o l o l`.",
        ),
        row(
            "manufacturer",
            Arity::at_least(1),
            KEYWORD0,
            MANUFACTURER_OPTIONS,
            "Declare one instance-manufacturing method of the command.",
            "`-names-instance-at` and `-constructor-args-from` are read by other consumers; only `-definition-body-at` feeds `arg_role_resolver from-manufacturers`, and only as a `Body` role.",
        ),
    ]
}

/// The four E-R14 option-relation rows (redesign §11.1 O1).
///
/// Their own function because they share one shape and one flag set: a
/// statement word, an optional subject term, a term list, and the flags every
/// relation row takes. Splitting them out is also what keeps
/// [`call_shape_rows`] readable now that there are four of them, not one.
fn option_relation_rows() -> Vec<CommandSpec> {
    vec![
        row(
            "option_conflict",
            Arity::at_least(1),
            VALUE0,
            OPTION_CONFLICT_OPTIONS,
            "Declare a set of options that may not co-occur.",
            "`option_conflict {-a -b}` — the symmetric relation, and the only one 1.x could express. A term is `-name`, `{-name value}`, `{arg N}` or `{arg N value}`.",
        ),
        row(
            "option_requires",
            Arity::at_least(2),
            VALUE0,
            OPTION_CONFLICT_OPTIONS,
            "Declare that one option or argument requires every term of a set.",
            "`option_requires SUBJECT {TERM …}` — E-R14's directional relation (`bibtex::parse -command` requires `-channel`). Checked natively in Rust; no hook, no VM.",
        ),
        row(
            "option_requires_one_of",
            Arity::at_least(2),
            VALUE0,
            OPTION_CONFLICT_OPTIONS,
            "Declare that one option or argument requires at least one term of a set.",
            "`option_requires_one_of SUBJECT {TERM …}`. An empty subject (`{}`) makes the relation unconditional — `bibtex::parse` needs `-channel` or a text argument on every call.",
        ),
        row(
            "option_forbids",
            Arity::at_least(2),
            VALUE0,
            OPTION_CONFLICT_OPTIONS,
            "Declare that one option or argument excludes every term of a set.",
            "`option_forbids SUBJECT {TERM …}` — the asymmetric exclusion a symmetric set cannot phrase (`struct::tree walk -order in` is illegal with `-type bfs`).",
        ),
    ]
}

/// The rows that constrain individual values and second-level words.
fn value_shape_rows() -> Vec<CommandSpec> {
    vec![
        row(
            "setter_constraint",
            Arity::at_least(1),
            INDEX0,
            SETTER_CONSTRAINT_OPTIONS,
            "Constrain the value a setter form accepts at one index.",
            "`setter_constraint N -prefix P -code CODE -message {…}`.",
        ),
        row(
            "sub_subcommand",
            Arity::at_least(1),
            NAME0,
            SUB_SUBCOMMAND_OPTIONS,
            "Declare one second-level word of the enclosing subcommand.",
            "The singular-row rule again: `sub_subcommands` is a list of rows, so it gets a row statement, exactly like `options` → `option`.",
        ),
        row(
            "oo_context_fact",
            Arity::exact(2),
            KEYWORD0,
            &[],
            "Declare what one word of the call means in an object context.",
            "`oo_context_fact WORD FACT`, one row per fact.",
        ),
        row(
            "versioned_arg_value",
            Arity::at_least(2),
            INDEX0,
            VERSIONED_ARG_VALUE_OPTIONS,
            "Gate one accepted argument value on a package version.",
            "`versioned_arg_value N VALUE ?-introduced V? …`, one row per gate.",
        ),
        row(
            "event_requirement_form",
            Arity::at_least(1),
            VALUE0,
            EVENT_REQUIREMENT_FORM_OPTIONS,
            "Attach event requirements to one argument form.",
            "`event_requirement_form {word …} ?-only-in {E …}? ?{ … }?` — the trailing block is a nested `event_requires`.",
        ),
    ]
}

/// The rows that describe what a call *means* to a consumer — the symbols
/// and handles it defines, its frame and payload shapes, its lifecycle fix,
/// and the `value` row of a shared table.
fn semantic_rows() -> Vec<CommandSpec> {
    vec![
        row(
            "defines_symbol",
            Arity::at_least(2),
            KEYWORD0,
            DEFINES_SYMBOL_OPTIONS,
            "Declare that the command defines a named symbol.",
            "Four plain-data fields, which is why it is declarative rather than a hook.",
        ),
        row(
            "binds_handle",
            Arity::at_least(2),
            KEYWORD0,
            BINDS_HANDLE_OPTIONS,
            "Declare that the command binds a named handle to a class.",
            "Three plain-data fields — the shape snit's `install NAME using CLASS …` needs.",
        ),
        row(
            "frame_effect",
            Arity::at_least(2),
            KEYWORD0,
            FRAME_EFFECT_OPTIONS,
            "Declare how the command crosses stack frames.",
            "Both payloads are closed enums, so the whole field is declarative. `state_transitions … resolver from-frame-effect` derives its resolver from this row.",
        ),
        row(
            "byte_array_payload",
            Arity::at_least(2),
            KEYWORD0,
            BYTE_ARRAY_PAYLOAD_OPTIONS,
            "Declare which argument carries a byte-array payload.",
            "Two plain-data fields.",
        ),
        row(
            "deprecation_fix",
            Arity::at_least(2),
            KEYWORD0,
            DEPRECATION_FIX_OPTIONS,
            "Declare the quick fix for a deprecated command.",
            "`Lifecycle.deprecation_fix`; the contextual-callback variant of the field is reference-only.",
        ),
        row(
            "event_handler_priority",
            Arity::at_least(2),
            KEYWORD0,
            EVENT_HANDLER_PRIORITY_OPTIONS,
            "Declare the command's default event-handler priority.",
            "`event_handler_priority -default N ?-warn-implicit?`.",
        ),
        row(
            "value",
            Arity::at_least(1),
            VALUE0,
            VALUE_OPTIONS,
            "Declare one member of a `values` table.",
            "`value V ?-detail {…}? ?-min-tcl VER? ?-code N?`, repeatable, inside a `values NAME { … }` block.",
        ),
    ]
}

pub(super) fn specs() -> Vec<CommandSpec> {
    let mut specs = call_shape_rows();
    specs.extend(option_relation_rows());
    specs.extend(value_shape_rows());
    specs.extend(semantic_rows());
    specs
}

const VALUE_OPTIONS: &[OptionSpec] = &[
    valued("-detail", "prose", "what this value means"),
    valued(
        "-min-tcl",
        "version",
        "the Tcl version this value first appears in",
    ),
    valued("-code", "n", "the completion code this value names"),
];
