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

//! `arg` — the per-argument-position row.
//!
//! Six schema keys are indexed by the same argument position; the DSL
//! collapses them into one row per index, which is the single biggest
//! readability win the format has over the `.rs` form.
use crate::prelude::*;

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-role",
        value: OptionValue::value("role"),
        detail: "the argument's ArgRole (fills `arg_roles`)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value("type"),
        detail: "the argument's value type (fills `arg_types`)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-shimmers",
        detail: "the argument's representation may shimmer",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-transparent",
        value: OptionValue::value("types"),
        detail: "representations the argument passes through unchanged",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-values",
        value: OptionValue::value("values"),
        detail: "inline enumerable value set (fills `arg_values`)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-values-from",
        value: OptionValue::value("table"),
        detail: "name a pack-level `values` table",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-closed",
        detail: "the value set is exhaustive (fills `closed_value_args`)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-layout",
        value: OptionValue::value("BlockScript|InlineScript"),
        detail: "formatter presentation (fills `arg_presentation`)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-appends",
        value: OptionValue::value("{Exactly N}"),
        detail: "callback arity; implies `-role CommandPrefix`",
        ..OptionSpec::DEFAULT
    },
    // SpecTcl 1.2: an argument is a gateable fact like every other row, so it
    // carries the same three lifecycle flags. They must be listed here as well
    // as read by the loader — an option table that is non-empty is scanned for
    // unknown flags, so a flag the loader accepts and this table omits is
    // reported against valid syntax.
    OptionSpec {
        name: "-introduced",
        value: OptionValue::value("version"),
        detail: "the release the argument first appeared in (`Lifecycle.introduced`)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-deprecated",
        value: OptionValue::value("version"),
        detail: "the release the argument was deprecated in (`Lifecycle.deprecated`)",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-retired",
        value: OptionValue::value("version"),
        detail: "the release the argument was removed in (`Lifecycle.retired`)",
        ..OptionSpec::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "arg index ?-role R? ?-type T? ?-values {…}? ?-closed? ?-layout L? …",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "arg",
        traits: Traits::LANGUAGE_KEYWORD,
        dialects: Some(DialectSet::SPECTCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Declare the facts about one argument position.",
            synopsis: &[
                "arg index ?-role R? ?-type T? ?-values {…}? ?-closed? ?-layout L? ?-appends {Exactly N}?",
            ],
            snippet: "Indices are 0-based after the command name, or after the subcommand word inside a `subcommand` block — the same coordinates the registry uses. An index above 255 is dropped with a notice, matching the u8 tables. There is deliberately no `-detail`: no per-argument prose field exists, and argument documentation reaches the user through `synopsis` and `hover`'s `description`.",
            source: "SpecTcl (docs/design/spec-packs.md)",
            examples: "arg 0 -type String -shimmers -transparent {ByteArray} -values-from is-classes -closed\narg 1 -role Body -layout InlineScript\narg 2 -appends {Exactly 2}",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        arg_roles: &[(0, ArgRole::Index)],
        ..CommandSpec::DEFAULT
    }
}
