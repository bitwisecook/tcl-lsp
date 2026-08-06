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

//! `font` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "actual",
        arity: Arity::at_least(1),
        detail: "Return the actual attributes of a font on the display.",
        synopsis: "font actual font ?-displayof window? ?option? ?--? ?char?",
        options: ACTUAL_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Query or modify the desired attributes of a named font.",
        synopsis: "font configure fontname ?option? ?value option value ...?",
        options: ATTRIBUTE_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::at_least(0),
        detail: "Create a new named font with the given options.",
        synopsis: "font create ?fontname? ?option value ...?",
        options: ATTRIBUTE_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(1),
        detail: "Delete one or more named fonts.",
        synopsis: "font delete fontname ?fontname ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "families",
        arity: Arity::new(0, 2),
        detail: "Return a list of all font families available on the display.",
        synopsis: "font families ?-displayof window?",
        options: DISPLAYOF_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "measure",
        arity: Arity::at_least(2),
        detail: "Measure the width of the text string when rendered in the given font.",
        synopsis: "font measure font ?-displayof window? text",
        options: DISPLAYOF_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "metrics",
        arity: Arity::at_least(1),
        detail: "Return metric information for the given font.",
        synopsis: "font metrics font ?-displayof window? ?option?",
        options: DISPLAYOF_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Return a list of all named fonts currently defined.",
        synopsis: "font names",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-family",
        value: OptionValue::value("name"),
        detail: "Font family name (e.g. Courier, Times, Helvetica).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-size",
        value: OptionValue::value("size"),
        detail: "Desired size of the font in points (positive) or pixels (negative).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-weight",
        value: OptionValue::value("normal|bold"),
        detail: "Weight of the font: normal or bold.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-slant",
        value: OptionValue::value("roman|italic"),
        detail: "Slant of the font: roman or italic.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-underline",
        value: OptionValue::boolean(),
        detail: "Whether to draw an underline beneath the text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-overstrike",
        value: OptionValue::boolean(),
        detail: "Whether to draw a horizontal line through the text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-displayof",
        value: OptionValue::value("window"),
        detail: "Specifies the display for the font query.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// `families` and `measure`: per the Tk manual (`font families ?-displayof
/// window?`, `font measure font ?-displayof window? text`) these two take
/// only the display selector — no font-attribute options.
const DISPLAYOF_OPTIONS: &[OptionSpec] = &[OptionSpec {
    name: "-displayof",
    value: OptionValue::value("window"),
    detail: "Specifies the display for the font query.",
    dialects: None,
    aliases: &[],
    lifecycle: Lifecycle::UNSPECIFIED,
    min_abbrev: None,
}];

/// `configure` and `create`: per the manual (`font configure fontname
/// ?option? ?value option value ...?`, `font create ?fontname? ?option value
/// ...?`) these take the six font-attribute options as option/value pairs —
/// no `-displayof` (there is no display argument to either form).
const ATTRIBUTE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-family",
        value: OptionValue::value("name"),
        detail: "Font family name (e.g. Courier, Times, Helvetica).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-size",
        value: OptionValue::value("size"),
        detail: "Desired size of the font in points (positive) or pixels (negative).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-weight",
        value: OptionValue::value("normal|bold"),
        detail: "Weight of the font: normal or bold.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-slant",
        value: OptionValue::value("roman|italic"),
        detail: "Slant of the font: roman or italic.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-underline",
        value: OptionValue::boolean(),
        detail: "Whether to draw an underline beneath the text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-overstrike",
        value: OptionValue::boolean(),
        detail: "Whether to draw a horizontal line through the text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// `actual`: per the manual (`font actual font ?-displayof window? ?option?
/// ?--? ?char?`) this takes `-displayof` plus the same six font-attribute
/// options, used here as the `option` argument to select a single resolved
/// attribute to return.
const ACTUAL_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-displayof",
        value: OptionValue::value("window"),
        detail: "Specifies the display for the font query.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-family",
        value: OptionValue::value("name"),
        detail: "Font family name (e.g. Courier, Times, Helvetica).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-size",
        value: OptionValue::value("size"),
        detail: "Desired size of the font in points (positive) or pixels (negative).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-weight",
        value: OptionValue::value("normal|bold"),
        detail: "Weight of the font: normal or bold.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-slant",
        value: OptionValue::value("roman|italic"),
        detail: "Slant of the font: roman or italic.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-underline",
        value: OptionValue::boolean(),
        detail: "Whether to draw an underline beneath the text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-overstrike",
        value: OptionValue::boolean(),
        detail: "Whether to draw a horizontal line through the text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "font option ?arg ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "font",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and inspect fonts.",
            synopsis: &[
                "font actual font ?-displayof window? ?option? ?--? ?char?",
                "font configure fontname ?option? ?value option value ...?",
                "font create ?fontname? ?option value ...?",
                "font delete fontname ?fontname ...?",
                "font families ?-displayof window?",
                "font measure font ?-displayof window? text",
                "font metrics font ?-displayof window? ?option?",
                "font names",
            ],
            snippet: "",
            source: "Tk man page font.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
