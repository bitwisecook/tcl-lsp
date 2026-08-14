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
    synopsis: "clock subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

// Calendar conversions can observe the host time-zone/locale database, Tcl's
// loaded clock packages, and environment-backed variables such as `env(TZ)`.
// The exact footprint varies by Tcl release and options, so the result key
// keeps the conservative union while the ordinary effect footprint remains
// undeclared.
const CLOCK_CONTEXT_DOMAINS: &[WorldStateDomain] = &[
    WorldStateDomain::HostCapabilities,
    WorldStateDomain::PackageState,
    WorldStateDomain::VariableStore,
];
const VERSIONED_CLOCK_CONTEXT_RESULT: SubCommand = SubCommand {
    result_stability: Some(ResultStability::ReadsVersionedWorld(CLOCK_CONTEXT_DOMAINS)),
    ..SubCommand::DEFAULT
};

/// Options accepted by `clock format`.
///
/// Tcl 8.4's `clock format` only ever took `-format`/`-gmt`
/// (`clock format clockValue ?-format string? ?-gmt boolean?`);
/// `-locale`/`-timezone` arrived with the Tcl 8.5 clock rewrite
/// (TIP 173). `clock format` never takes `-base` or `-validate` —
/// those are `clock scan`-only.
static FORMAT_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-format",
        // The value is the `clock` field string, not a generic value: the
        // `ArgRole::FormatString` role is what lets the LSP find it without
        // naming `clock` (issue #1185); the *family* comes from the
        // subcommand's `format_string_type`.
        value: OptionValue::Takes(OptionArg {
            role: ArgRole::FormatString,
            hint: "format",
            ..OptionArg::DEFAULT
        }),
        detail: "strftime-style output format string (default \"%a %b %d %H:%M:%S %Z %Y\").",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-gmt",
        value: OptionValue::boolean(),
        detail: "Format in UTC instead of the local time zone; superseded by -timezone :UTC but still accepted.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-locale",
        value: OptionValue::value("locale"),
        detail: "Locale for month / day-of-week names and the other locale-dependent format groups (%a, %c, ...).",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-timezone",
        value: OptionValue::value("tz"),
        detail: "Time zone to format the value in.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// Options accepted by `clock scan`.
///
/// Tcl 8.4's `clock scan` only ever took `-base`/`-gmt` and always used
/// the free-form parser (`clock scan dateString ?-base clockVal? ?-gmt
/// boolean?`); `-format` (strict-format scanning), `-locale`, and
/// `-timezone` arrived with the Tcl 8.5 clock rewrite (TIP 173).
/// `-validate` is Tcl 9.0+ (TIP 688).
static SCAN_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-base",
        value: OptionValue::value("seconds"),
        detail: "Base date/time used to fill in fields the input string doesn't specify.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-format",
        // As for `clock format` — the role locates the field string, the
        // subcommand's `format_string_type` names its family.
        value: OptionValue::Takes(OptionArg {
            role: ArgRole::ScanFormat,
            hint: "format",
            ..OptionArg::DEFAULT
        }),
        detail: "Explicit input format string for strict parsing; omitting it falls back to the deprecated free-form scanner.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-gmt",
        value: OptionValue::boolean(),
        detail: "Interpret the input as UTC instead of local time; superseded by -timezone :UTC but still accepted.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-locale",
        value: OptionValue::value("locale"),
        detail: "Locale for month / day-of-week names.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-timezone",
        value: OptionValue::value("tz"),
        detail: "Time zone for interpretation.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    // `-validate` is Tcl 9.0+ (TIP 688).
    OptionSpec {
        name: "-validate",
        value: OptionValue::boolean(),
        detail: "When true (the default), an out-of-range field (e.g. day 31 in a 30-day month) raises an error; when false, it is clamped into range instead.",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// Options accepted by `clock add`: time-zone/locale/UTC context for the
/// arithmetic, present since the subcommand's own Tcl 8.5 debut (TIP 173).
/// `clock add` never takes `-base`, `-format`, or `-validate`.
static ADD_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-gmt",
        value: OptionValue::boolean(),
        detail: "Do the arithmetic in UTC instead of the local time zone; superseded by -timezone :UTC but still accepted.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-locale",
        value: OptionValue::value("locale"),
        detail: "Locale used to resolve month/weekday names.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-timezone",
        value: OptionValue::value("tz"),
        detail: "Time zone the calendar arithmetic (days/weekdays/weeks/months/years) is done in.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// Options accepted by `clock clicks`: two obsolete boolean synonyms for
/// `clock milliseconds`/`clock microseconds`. `-milliseconds` dates to
/// Tcl 8.4 (`clock clicks ?-milliseconds?`); `-microseconds` was added
/// alongside `clock microseconds` itself in the Tcl 8.5 clock rewrite
/// (TIP 173).
static CLICKS_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-milliseconds",
        value: OptionValue::flag(),
        detail: "Return the same value as `clock milliseconds` instead of the raw high-resolution counter.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-microseconds",
        value: OptionValue::flag(),
        detail: "Return the same value as `clock microseconds` instead of the raw high-resolution counter.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// Legal `unit` words for `clock add`'s repeating `count unit` pairs —
/// shared across every pair position since the same vocabulary applies
/// throughout. `weekdays` (skipping Saturday and Sunday) was added in
/// Tcl 9.0 — absent from the 8.6.18 clock.n unit list ("seconds, minutes,
/// hours, days, weeks, months, or years") and first appears in the 9.0.4
/// page's list ("seconds, minutes, hours, days, weekdays, weeks, months,
/// or years"); the other seven date to `clock add`'s own Tcl 8.5 debut
/// (TIP 173, the clock rewrite) — none of them exist in Tcl 8.4, which
/// has no `clock add` at all.
const ADD_UNIT_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "seconds",
        detail: "Seconds.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "minutes",
        detail: "Minutes (60 seconds).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "hours",
        detail: "Hours (3600 seconds).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "days",
        detail: "Days (86400 seconds).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "weekdays",
        detail: "Weekdays (skipping Saturday and Sunday).",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "weeks",
        detail: "Weeks (7 days).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "months",
        detail: "Calendar months (clamped to the last day of a shorter target month).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "years",
        detail: "Calendar years (clamped to the last day of a shorter target month, e.g. leap-day arithmetic).",
        ..ArgValue::DEFAULT
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        // Added in Tcl 8.5 (the clock rewrite, TIP 173); the `weekdays`
        // unit (see `ADD_UNIT_VALUES`) followed in Tcl 9.0, not 8.6 (the
        // 8.6.18 clock.n page's unit list has no `weekdays`; it first
        // appears in the 9.0.4 page).
        dialects: Some(DialectSet::TCL85_PLUS),
        arity: Arity::at_least(1),
        detail: "Add duration to a time.",
        synopsis: "clock add timeVal ?count unit ...?",
        return_type: Some(TclType::Int),
        options: ADD_OPTIONS,
        // `timeVal` is index 0; each `count unit` pair follows as
        // (integer, unit-word), so the enumerable `unit` words land at
        // 2, 4, 6, and 8 — not at the `count` slots (1, 3, 5, 7).
        arg_values: &[
            (2, ADD_UNIT_VALUES),
            (4, ADD_UNIT_VALUES),
            (6, ADD_UNIT_VALUES),
            (8, ADD_UNIT_VALUES),
        ],
        pure: true,
        ..VERSIONED_CLOCK_CONTEXT_RESULT
    },
    SubCommand {
        name: "clicks",
        arity: Arity::new(0, 1),
        detail: "Return hi-res clock value.",
        synopsis: "clock clicks ?-milliseconds? ?-microseconds?",
        options: CLICKS_OPTIONS,
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::VOLATILE_RESULT
    },
    SubCommand {
        name: "format",
        arity: Arity::at_least(1),
        detail: "Format a time value.",
        synopsis: "clock format timeVal ?-option value ...?",
        options: FORMAT_OPTIONS,
        // Populates the previously-dead `format_string_type` field: the
        // family of this call's format string. Paired with the
        // `FormatString` / `ScanFormat` argument role that locates the
        // word, it is the whole registry answer the LSP needs (#1185).
        format_string_type: Some(FormatType::Clock),
        pure: true,
        return_type: Some(TclType::String),
        ..VERSIONED_CLOCK_CONTEXT_RESULT
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
        ..SubCommand::VOLATILE_RESULT
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
        ..SubCommand::VOLATILE_RESULT
    },
    SubCommand {
        name: "monotonic",
        // Added in Tcl 9.1 — absent from the Tcl 9.0 synopsis and
        // description, present in 9.1's.  A genuinely new subcommand,
        // not merely a documentation change, so it needs its own
        // single-bit gate rather than TCL90_PLUS.
        dialects: Some(DialectSet::TCL91),
        arity: Arity::exact(0),
        detail: "Return current monotonic clock value in microseconds.",
        synopsis: "clock monotonic",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::VOLATILE_RESULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::at_least(1),
        detail: "Parse a date/time string.",
        synopsis: "clock scan inputString ?-option value ...?",
        return_type: Some(TclType::Int),
        options: SCAN_OPTIONS,
        // Populates the previously-dead `format_string_type` field: the
        // family of this call's format string. Paired with the
        // `FormatString` / `ScanFormat` argument role that locates the
        // word, it is the whole registry answer the LSP needs (#1185).
        format_string_type: Some(FormatType::Clock),
        ..SubCommand::VOLATILE_RESULT
    },
    SubCommand {
        name: "seconds",
        arity: Arity::exact(0),
        detail: "Return current time in seconds.",
        synopsis: "clock seconds",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::VOLATILE_RESULT
    },
];

/// Command-level `clock format`/`clock scan` options — surfaced at the
/// command level (before the subcommand word is resolved) for completion
/// consistency; `FORMAT_OPTIONS`/`SCAN_OPTIONS` carry the precise
/// per-subcommand set (`clock format` never takes `-base`/`-validate`;
/// `clock scan` alone takes `-validate`).  Each entry's `dialects` here is
/// the earliest version at which *any* subcommand accepts that flag, since
/// this flattened list can't itself distinguish which subcommand is being
/// completed.
const CMD_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-base",
        value: OptionValue::value("timeVal"),
        detail: "Base time for relative scanning.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-format",
        value: OptionValue::value("format"),
        detail: "strftime-style format string.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-gmt",
        value: OptionValue::boolean(),
        detail: "Use GMT instead of local time.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    // `-locale`/`-timezone` arrived with the Tcl 8.5 clock rewrite
    // (TIP 173) — absent from both `clock format` and `clock scan` in
    // Tcl 8.4. This duplicate table had drifted to `dialects: None`,
    // silently offering both under an 8.4 dialect filter.
    OptionSpec {
        name: "-locale",
        value: OptionValue::value("locale"),
        detail: "Locale for month/day names.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-timezone",
        value: OptionValue::value("zone"),
        detail: "Time zone for conversion.",
        dialects: Some(DialectSet::TCL85_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    // `-validate` is Tcl 9.0+, same as `SCAN_OPTIONS`'s entry above — this
    // duplicate table had drifted to `dialects: None`, silently omitting the
    // gate for the top-level (pre-subcommand-resolution) completion/hover path.
    OptionSpec {
        name: "-validate",
        value: OptionValue::boolean(),
        detail: "Validate date fields strictly (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clock",
        dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)),
        traits: Traits::BYTE_COMPILED | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        options: CMD_OPTIONS,
        hover: Some(HoverSnippet {
            summary: "Obtain and manipulate dates and times.",
            synopsis: &[
                "clock add timeVal ?count unit ...?",
                "clock clicks ?-milliseconds? ?-microseconds?",
                "clock format timeVal ?-option value ...?",
                "clock microseconds",
                "clock milliseconds",
                "clock monotonic",
                "clock scan inputString ?-option value ...?",
                "clock seconds",
            ],
            snippet: "`clock seconds`, `clock milliseconds`, `clock microseconds`, and `clock clicks` read the current wall-clock time; `clock monotonic` (Tcl 9.1+) instead reads a clock guaranteed never to run backwards, unlike wall-clock time which can jump on an NTP correction. `clock format` converts a seconds-since-epoch value to text and `clock scan` parses text back into one; `clock add` performs calendar-aware arithmetic on a seconds value (day/week arithmetic preserves local time across DST transitions; month/year arithmetic clamps to the last day of a shorter target month). Before the Tcl 8.5 clock rewrite (TIP 173), `clock format` accepted only -format/-gmt and `clock scan` accepted only -base/-gmt, always via free-form parsing, and there was no `clock add`/`clock microseconds`/`clock milliseconds` at all — `-locale`, `-timezone`, and `clock scan`'s strict -format parsing all arrived with that rewrite. `-validate` (Tcl 9.0+, TIP 688) controls whether an out-of-range field during a strict `clock scan` raises an error (the default) or is clamped into range.",
            source: "Tcl clock(n)",
            examples: "clock format [clock seconds] -format {%Y-%m-%d %H:%M:%S}\nclock scan {2024-01-15} -format {%Y-%m-%d}\nclock add [clock seconds] 1 day -timezone :UTC",
            return_value: "An integer for seconds, milliseconds, microseconds, clicks, monotonic, scan, and add; a formatted string for format.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
