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

//! `clock` — time and date operations.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "clock subcommand ?arg ...?",
    dialects: None,
}];

/// Options for `clock scan` / `clock format` / `clock add`.
/// `-validate` is Tcl 9.0+ (TIP 688) and dialect-gated; the
/// others exist since Tcl 8.5.
static SCAN_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-base",
        value: OptionValue::value("seconds"),
        detail: "Base date/time used for partial input.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "Explicit format string (defaults to free-form parser).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-gmt",
        value: OptionValue::value("boolean"),
        detail: "Use UTC instead of local time.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-locale",
        value: OptionValue::value("locale"),
        detail: "Locale for month / day-of-week names.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-timezone",
        value: OptionValue::value("tz"),
        detail: "Time zone for interpretation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    // `-validate` is Tcl 9.0+ (TIP 688).
    OptionSpec {
        name: "-validate",
        value: OptionValue::value("boolean"),
        detail: "Validate the input date/time strictly (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        // Added in Tcl 8.5 (the clock rewrite, TIP 173).
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        detail: "Add duration to a time.",
        synopsis: "clock add timeVal ?count unit ...?",
        return_type: Some(TclType::Int),
        arg_values: &[
            (
                1,
                &[
                    ArgValue {
                        value: "seconds",
                        detail: "Seconds.",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "minutes",
                        detail: "Minutes (60 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "hours",
                        detail: "Hours (3600 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "days",
                        detail: "Days (86400 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "weekdays",
                        detail: "Weekdays (skipping Saturday and Sunday).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "weeks",
                        detail: "Weeks (7 days).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "months",
                        detail: "Calendar months.",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "years",
                        detail: "Calendar years.",
                        min_tcl: None,
                        code: None,
                    },
                ],
            ),
            (
                3,
                &[
                    ArgValue {
                        value: "seconds",
                        detail: "Seconds.",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "minutes",
                        detail: "Minutes (60 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "hours",
                        detail: "Hours (3600 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "days",
                        detail: "Days (86400 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "weekdays",
                        detail: "Weekdays (skipping Saturday and Sunday).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "weeks",
                        detail: "Weeks (7 days).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "months",
                        detail: "Calendar months.",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "years",
                        detail: "Calendar years.",
                        min_tcl: None,
                        code: None,
                    },
                ],
            ),
            (
                5,
                &[
                    ArgValue {
                        value: "seconds",
                        detail: "Seconds.",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "minutes",
                        detail: "Minutes (60 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "hours",
                        detail: "Hours (3600 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "days",
                        detail: "Days (86400 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "weekdays",
                        detail: "Weekdays (skipping Saturday and Sunday).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "weeks",
                        detail: "Weeks (7 days).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "months",
                        detail: "Calendar months.",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "years",
                        detail: "Calendar years.",
                        min_tcl: None,
                        code: None,
                    },
                ],
            ),
            (
                7,
                &[
                    ArgValue {
                        value: "seconds",
                        detail: "Seconds.",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "minutes",
                        detail: "Minutes (60 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "hours",
                        detail: "Hours (3600 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "days",
                        detail: "Days (86400 seconds).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "weekdays",
                        detail: "Weekdays (skipping Saturday and Sunday).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "weeks",
                        detail: "Weeks (7 days).",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "months",
                        detail: "Calendar months.",
                        min_tcl: None,
                        code: None,
                    },
                    ArgValue {
                        value: "years",
                        detail: "Calendar years.",
                        min_tcl: None,
                        code: None,
                    },
                ],
            ),
        ],
        pure: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "clicks",
        arity: Arity::new(0, 1),
        detail: "Return hi-res clock value.",
        synopsis: "clock clicks ?-option?",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "format",
        arity: Arity::at_least(1),
        detail: "Format a time value.",
        synopsis: "clock format timeVal ?-option value ...?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "microseconds",
        // Added in Tcl 8.5 (TIP 173).
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::exact(0),
        detail: "Return current time in microseconds.",
        synopsis: "clock microseconds",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "milliseconds",
        // Added in Tcl 8.5 (TIP 173).
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::exact(0),
        detail: "Return current time in milliseconds.",
        synopsis: "clock milliseconds",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::at_least(1),
        detail: "Parse a date/time string.",
        synopsis: "clock scan inputString ?-option value ...?",
        return_type: Some(TclType::Int),
        options: SCAN_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "seconds",
        arity: Arity::exact(0),
        detail: "Return current time in seconds.",
        synopsis: "clock seconds",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
];

/// Command-level `clock format`/`scan` options (the `clock format`/`scan`
/// form options) — surfaced at the command level for completion consistency.
const CMD_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-base",
        value: OptionValue::value("timeVal"),
        detail: "Base time for relative scanning.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "strftime-style format string.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-gmt",
        value: OptionValue::value("boolean"),
        detail: "Use GMT instead of local time.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-locale",
        value: OptionValue::value("locale"),
        detail: "Locale for month/day names.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-timezone",
        value: OptionValue::value("zone"),
        detail: "Time zone for conversion.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    // `-validate` is Tcl 9.0+, same as `SCAN_OPTIONS`'s entry above — this
    // duplicate table had drifted to `dialects: None`, silently omitting the
    // gate for the top-level (pre-subcommand-resolution) completion/hover path.
    OptionSpec {
        name: "-validate",
        value: OptionValue::value("boolean"),
        detail: "Validate date fields strictly (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clock",
        traits: Traits::BYTE_COMPILED | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        options: CMD_OPTIONS,
        hover: Some(HoverSnippet {
            summary: "Obtain and manipulate dates and times.",
            synopsis: &[
                "clock add timeVal count unit ?...?",
                "clock format timeVal ?-option value ...?",
                "clock scan inputString ?-option value ...?",
                "clock seconds",
                "clock subcommand ?arg ...?",
            ],
            snippet: "Use `clock seconds` for epoch time, `clock format` to display, `clock scan` to parse.",
            source: "Tcl man page clock.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
