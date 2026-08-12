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

//! The SpecTcl **row** statements.
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

/// A row statement: a leading subject word, then flags.
fn row(
    name: &'static str,
    arity: Arity,
    subject: ArgRole,
    options: &'static [OptionSpec],
    summary: &'static str,
    snippet: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::SPECTCL),
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
        arg_roles: &[(0, subject)],
        ..CommandSpec::DEFAULT
    }
}

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

const FORM_OPTIONS: &[OptionSpec] = &[valued("-dialects", "set", "restrict this form to a dialect set")];

const SIDE_EFFECT_OPTIONS: &[OptionSpec] = &[
    flag("-reads", "the command reads the target"),
    flag("-writes", "the command writes the target"),
    valued("-side", "Client|Server|Both|None", "the connection side affected"),
    valued("-dialects", "set", "restrict this effect to a dialect set"),
];

const REPEAT_OPTIONS: &[OptionSpec] = &[
    valued("-from", "n", "first index the layout covers"),
    valued("-stride", "n", "distance between covered indices"),
    valued("-exclude-trailing", "n", "trailing words the layout does not cover"),
    flag("-optional-leading", "the layout may start one word later"),
    flag("-conditional", "the layout applies only in some forms"),
];

const MANUFACTURER_OPTIONS: &[OptionSpec] = &[
    flag("-unexported", "the method exists but ordinary dispatch cannot reach it"),
    valued("-names-instance-at", "n", "the word naming the new instance"),
    valued("-definition-body-at", "n", "the word carrying the definition body"),
    valued("-constructor-args-from", "n", "first word of the constructor's own arguments"),
];

const OPTION_CONFLICT_OPTIONS: &[OptionSpec] =
    &[valued("-dialects", "set", "restrict this constraint to a dialect set")];

const SETTER_CONSTRAINT_OPTIONS: &[OptionSpec] = &[
    valued("-prefix", "text", "the prefix the value must carry"),
    valued("-code", "CODE", "diagnostic code reported on violation"),
    valued("-message", "prose", "diagnostic message reported on violation"),
];

const SUB_SUBCOMMAND_OPTIONS: &[OptionSpec] = &[
    valued("-detail", "prose", "the second-level word's one-line description"),
    valued("-synopsis", "text", "the second-level word's synopsis"),
    valued("-dialects", "set", "restrict this word to a dialect set"),
];

const VERSIONED_ARG_VALUE_OPTIONS: &[OptionSpec] = &[
    valued("-introduced", "version", "Lifecycle.introduced"),
    valued("-deprecated", "version", "Lifecycle.deprecated"),
    valued("-retired", "version", "Lifecycle.retired"),
];

const EVENT_REQUIREMENT_FORM_OPTIONS: &[OptionSpec] =
    &[valued("-only-in", "events", "events this form is restricted to")];

const DEFINES_SYMBOL_OPTIONS: &[OptionSpec] = &[
    valued("-name-arg", "n", "the word naming the defined symbol"),
    valued("-detail-arg", "n", "the word carrying the symbol's detail"),
    valued("-requires-arg", "n", "the word naming what the symbol requires"),
    valued("-kind", "KIND", "the kind of symbol defined"),
];

const BINDS_HANDLE_OPTIONS: &[OptionSpec] = &[
    valued("-name-from", "{Word N}", "where the handle's name comes from"),
    valued("-class-from", "{Word N}", "where the handle's class comes from"),
    valued("-keyword", "{N WORD}", "a keyword word the form requires"),
];

const FRAME_EFFECT_OPTIONS: &[OptionSpec] = &[
    valued("-level-word", "policy", "which word (if any) selects the frame"),
    valued("-layout", "layout", "how the remaining words are read"),
];

const BYTE_ARRAY_PAYLOAD_OPTIONS: &[OptionSpec] = &[
    valued("-replace-data-index", "n", "the payload word"),
    flag("-message-flag-shift", "a leading message flag shifts the payload word"),
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

pub(super) fn specs() -> Vec<CommandSpec> {
    vec![
        row(
            "form",
            Arity::at_least(2),
            ArgRole::Keyword,
            FORM_OPTIONS,
            "Declare one documented call form.",
            "`form KIND {synopsis} ?-dialects {…}?`. `forms` covers the getter/setter split every pack has needed; the structured `command_forms` is excluded — a command needing it is deep enough in the compiler to be a contribution.",
        ),
        row(
            "side_effect",
            Arity::at_least(1),
            ArgRole::Keyword,
            SIDE_EFFECT_OPTIONS,
            "Declare one side effect of the command.",
            "One row per effect: the target, then whether it is read, written, and on which connection side.",
        ),
        row(
            "repeat",
            Arity::at_least(1),
            ArgRole::Keyword,
            REPEAT_OPTIONS,
            "Declare a role that recurs at a fixed stride over the argument tail.",
            "One row per layout — `global a b c`, `foreach v1 l1 v2 l2 body`, `upvar ?level? o l o l`.",
        ),
        row(
            "manufacturer",
            Arity::at_least(1),
            ArgRole::Keyword,
            MANUFACTURER_OPTIONS,
            "Declare one instance-manufacturing method of the command.",
            "`-names-instance-at` and `-constructor-args-from` are read by other consumers; only `-definition-body-at` feeds `arg_role_resolver from-manufacturers`, and only as a `Body` role.",
        ),
        row(
            "option_conflict",
            Arity::at_least(1),
            ArgRole::Value,
            OPTION_CONFLICT_OPTIONS,
            "Declare a set of options that may not co-occur.",
            "`OptionConstraint` is a flat may-not-co-occur set with no directionality, so only the `forbid` half of a real library's constraints maps. Option *requirement* relationships are a recorded registry gap, not DSL syntax.",
        ),
        row(
            "setter_constraint",
            Arity::at_least(1),
            ArgRole::Index,
            SETTER_CONSTRAINT_OPTIONS,
            "Constrain the value a setter form accepts at one index.",
            "`setter_constraint N -prefix P -code CODE -message {…}`.",
        ),
        row(
            "sub_subcommand",
            Arity::at_least(1),
            ArgRole::Name,
            SUB_SUBCOMMAND_OPTIONS,
            "Declare one second-level word of the enclosing subcommand.",
            "The singular-row rule again: `sub_subcommands` is a list of rows, so it gets a row statement, exactly like `options` → `option`.",
        ),
        row(
            "oo_context_fact",
            Arity::exact(2),
            ArgRole::Keyword,
            &[],
            "Declare what one word of the call means in an object context.",
            "`oo_context_fact WORD FACT`, one row per fact.",
        ),
        row(
            "versioned_arg_value",
            Arity::at_least(2),
            ArgRole::Index,
            VERSIONED_ARG_VALUE_OPTIONS,
            "Gate one accepted argument value on a package version.",
            "`versioned_arg_value N VALUE ?-introduced V? …`, one row per gate.",
        ),
        row(
            "event_requirement_form",
            Arity::at_least(1),
            ArgRole::Value,
            EVENT_REQUIREMENT_FORM_OPTIONS,
            "Attach event requirements to one argument form.",
            "`event_requirement_form {word …} ?-only-in {E …}? ?{ … }?` — the trailing block is a nested `event_requires`.",
        ),
        row(
            "defines_symbol",
            Arity::at_least(2),
            ArgRole::Keyword,
            DEFINES_SYMBOL_OPTIONS,
            "Declare that the command defines a named symbol.",
            "Four plain-data fields, which is why it is declarative rather than a hook.",
        ),
        row(
            "binds_handle",
            Arity::at_least(2),
            ArgRole::Keyword,
            BINDS_HANDLE_OPTIONS,
            "Declare that the command binds a named handle to a class.",
            "Three plain-data fields — the shape snit's `install NAME using CLASS …` needs.",
        ),
        row(
            "frame_effect",
            Arity::at_least(2),
            ArgRole::Keyword,
            FRAME_EFFECT_OPTIONS,
            "Declare how the command crosses stack frames.",
            "Both payloads are closed enums, so the whole field is declarative. `state_transitions … resolver from-frame-effect` derives its resolver from this row.",
        ),
        row(
            "byte_array_payload",
            Arity::at_least(2),
            ArgRole::Keyword,
            BYTE_ARRAY_PAYLOAD_OPTIONS,
            "Declare which argument carries a byte-array payload.",
            "Two plain-data fields.",
        ),
        row(
            "deprecation_fix",
            Arity::at_least(2),
            ArgRole::Keyword,
            DEPRECATION_FIX_OPTIONS,
            "Declare the quick fix for a deprecated command.",
            "`Lifecycle.deprecation_fix`; the contextual-callback variant of the field is reference-only.",
        ),
        row(
            "event_handler_priority",
            Arity::at_least(2),
            ArgRole::Keyword,
            EVENT_HANDLER_PRIORITY_OPTIONS,
            "Declare the command's default event-handler priority.",
            "`event_handler_priority -default N ?-warn-implicit?`.",
        ),
        row(
            "value",
            Arity::at_least(1),
            ArgRole::Value,
            VALUE_OPTIONS,
            "Declare one member of a `values` table.",
            "`value V ?-detail {…}? ?-min-tcl VER? ?-code N?`, repeatable, inside a `values NAME { … }` block.",
        ),
    ]
}

const VALUE_OPTIONS: &[OptionSpec] = &[
    valued("-detail", "prose", "what this value means"),
    valued("-min-tcl", "version", "the Tcl version this value first appears in"),
    valued("-code", "n", "the completion code this value names"),
];
