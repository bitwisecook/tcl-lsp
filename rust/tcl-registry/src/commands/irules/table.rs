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

//! `table` iRules command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "set",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "table set ?-notouch? ?-subtable name | -georedundancy? ?-mustexist | -excl? key value ?timeout ?lifetime??",
        mutator: true,
        options: SET_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "add",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "table add ?-notouch? ?-subtable name | -georedundancy? key value ?timeout ?lifetime??",
        mutator: true,
        options: SUBTABLE_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "incr",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "table incr ?-notouch? ?-subtable name | -georedundancy? ?-mustexist? key ?delta?",
        mutator: true,
        options: MUSTEXIST_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "table delete ?-subtable name | -georedundancy? key | -all",
        mutator: true,
        options: DELETE_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "append",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "table append ?-notouch? ?-subtable name | -georedundancy? ?-mustexist? key string",
        mutator: true,
        options: MUSTEXIST_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "table replace ?-notouch? ?-subtable name | -georedundancy? key value ?timeout ?lifetime??",
        mutator: true,
        options: SUBTABLE_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "keys",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "table keys -subtable name ?-count | -notouch?",
        options: KEYS_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lifetime",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "table lifetime ?-subtable name | -georedundancy? key ?value | -remaining?",
        options: REMAINING_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "timeout",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "table timeout ?-subtable name | -georedundancy? key ?value | -remaining?",
        options: REMAINING_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lookup",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "table lookup ?-notouch? ?-subtable name | -georedundancy? key",
        options: SUBTABLE_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

/// Shared by `add`, `replace`, and `lookup`: `-notouch` plus the subtable
/// selector and the generic option-list terminator. Per
/// <https://clouddocs.f5.com/api/irules/table.html> these three take no
/// other options.
const SUBTABLE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-notouch",
        value: OptionValue::flag(),
        detail: "Do not reset lifetime/timeout on access.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-subtable",
        value: OptionValue::value(""),
        detail: "Operate on a named subtable.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-georedundancy",
        value: OptionValue::flag(),
        detail: "Enable geo-redundancy for this entry.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

/// `set`: `SUBTABLE_OPTIONS` plus the create-vs-replace guards.
const SET_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-notouch",
        value: OptionValue::flag(),
        detail: "Do not reset lifetime/timeout on access.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-subtable",
        value: OptionValue::value(""),
        detail: "Operate on a named subtable.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-georedundancy",
        value: OptionValue::flag(),
        detail: "Enable geo-redundancy for this entry.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-mustexist",
        value: OptionValue::flag(),
        detail: "Fail if key does not already exist.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-excl",
        value: OptionValue::flag(),
        detail: "Fail if key already exists.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

/// `incr` and `append`: `SUBTABLE_OPTIONS` plus `-mustexist` (no `-excl`).
const MUSTEXIST_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-notouch",
        value: OptionValue::flag(),
        detail: "Do not reset lifetime/timeout on access.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-subtable",
        value: OptionValue::value(""),
        detail: "Operate on a named subtable.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-georedundancy",
        value: OptionValue::flag(),
        detail: "Enable geo-redundancy for this entry.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-mustexist",
        value: OptionValue::flag(),
        detail: "Fail if key does not already exist.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

/// `delete`: no `-notouch` (a deletion doesn't touch a lifetime) — just the
/// subtable selector, `-all`, and the terminator.
const DELETE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-subtable",
        value: OptionValue::value(""),
        detail: "Operate on a named subtable.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-georedundancy",
        value: OptionValue::flag(),
        detail: "Enable geo-redundancy for this entry.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-all",
        value: OptionValue::flag(),
        detail: "Delete all keys in a subtable.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

/// `timeout` and `lifetime`: no `-notouch` — the subtable selector plus
/// `-remaining` for the query form.
const REMAINING_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-subtable",
        value: OptionValue::value(""),
        detail: "Operate on a named subtable.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-georedundancy",
        value: OptionValue::flag(),
        detail: "Enable geo-redundancy for this entry.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-remaining",
        value: OptionValue::flag(),
        detail: "Return remaining time.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

/// `keys`: `-subtable` is mandatory, plus the mutually exclusive
/// `-count`/`-notouch`.
const KEYS_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-subtable",
        value: OptionValue::value(""),
        detail: "Operate on a named subtable.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-count",
        value: OptionValue::flag(),
        detail: "Return count of matching keys.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-notouch",
        value: OptionValue::flag(),
        detail: "Do not reset lifetime/timeout on access.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

const OPTIONS_1: &[OptionSpec] = &[
    OptionSpec {
        name: "-mustexist",
        value: OptionValue::flag(),
        detail: "Fail if key does not already exist.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-excl",
        value: OptionValue::flag(),
        detail: "Fail if key already exists.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-notouch",
        value: OptionValue::flag(),
        detail: "Do not reset lifetime/timeout on access.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-subtable",
        value: OptionValue::value(""),
        detail: "Operate on a named subtable.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-georedundancy",
        value: OptionValue::flag(),
        detail: "Enable geo-redundancy for this entry.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-remaining",
        value: OptionValue::flag(),
        detail: "Return remaining time.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-count",
        value: OptionValue::flag(),
        detail: "Return count of matching keys.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-all",
        value: OptionValue::flag(),
        detail: "Delete all keys in a subtable.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "table",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Provides enhanced access to the session table.",
            synopsis: &[
                "table set (((-mustexist | -excl) -notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY VALUE (('indefinite' | POSITIVE_INTEGER) ('indefinite' | POSITIVE_INTEGER)?)?",
                "table add ((-notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY VALUE (('indefinite' | POSITIVE_INTEGER) ('indefinite' | POSITIVE_INTEGER)?)?",
                "table replace ((-notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY VALUE (('indefinite' | POSITIVE_INTEGER) ('indefinite' | POSITIVE_INTEGER)?)?",
                "table lookup ((-notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY",
            ],
            snippet: "The table command is a superset of the session command, with improved syntax for general purpose use. Please see the table command article series for detailed information on its use.\n\nThis command is not available to GTM.\n\nIf the table command is used on the standby system in a HA pair, the command will perform a no-op because the content of the standby unit's session db should be updated only through mirroring.",
            source: "https://clouddocs.f5.com/api/irules/table.html",
            examples: "when RULE_INIT {\n    set static::maxquery 100\n    set static::holdtime 600\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: true,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "table <subcommand> ?options? ?--? key ?value? ?lifetime? ?timeout?",
            dialects: None,
        }],
        options: OPTIONS_1,
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SessionTable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
            dialects: None,
        }],
        ..CommandSpec::DEFAULT
    }
}
