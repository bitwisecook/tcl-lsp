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

//! `place` command.
use crate::prelude::*;

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Set or query the placement options for a window.",
        synopsis: "place configure window ?option? ?value option value ...?",
        options: CONFIGURE_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "content",
        arity: Arity::exact(1),
        detail: "Return a list of all slaves managed by the placer for the window (9.0+ name for `slaves`).",
        synopsis: "place content window",
        dialects: Some(DialectSet::TCL90_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::exact(1),
        detail: "Cause the placer to stop managing the geometry of the window.",
        synopsis: "place forget window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::exact(1),
        detail: "Return a list of the current configuration for the window.",
        synopsis: "place info window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "slaves",
        arity: Arity::exact(1),
        detail: "Return a list of all slaves managed by the placer for the window.",
        synopsis: "place slaves window",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

/// Options accepted by the `configure` subcommand. Kept in sync with the
/// top-level `OPTIONS` fallback below.
const CONFIGURE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-x",
        value: OptionValue::value("location"),
        detail: "Specifies the x-coordinate of the anchor point in the master window.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-y",
        value: OptionValue::value("location"),
        detail: "Specifies the y-coordinate of the anchor point in the master window.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-relx",
        value: OptionValue::value("location"),
        detail: "Specifies the x-coordinate as a fraction of the master width (0.0 to 1.0).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-rely",
        value: OptionValue::value("location"),
        detail: "Specifies the y-coordinate as a fraction of the master height (0.0 to 1.0).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value("size"),
        detail: "Specifies the width of the slave in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("size"),
        detail: "Specifies the height of the slave in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-relwidth",
        value: OptionValue::value("size"),
        detail: "Specifies the width as a fraction of the master width (0.0 to 1.0).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-relheight",
        value: OptionValue::value("size"),
        detail: "Specifies the height as a fraction of the master height (0.0 to 1.0).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-anchor",
        value: OptionValue::enumerated(super::common::ANCHOR, true, "anchor"),
        detail: "Specifies which point of the slave is positioned at the (x,y) location.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-bordermode",
        value: OptionValue::value("inside|outside|ignore"),
        detail: "Determines the degree to which borders within the master are used.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-in",
        value: OptionValue::value("master"),
        detail: "Specifies the master window relative to which the slave is placed.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-x",
        value: OptionValue::value("location"),
        detail: "Specifies the x-coordinate of the anchor point in the master window.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-y",
        value: OptionValue::value("location"),
        detail: "Specifies the y-coordinate of the anchor point in the master window.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-relx",
        value: OptionValue::value("location"),
        detail: "Specifies the x-coordinate as a fraction of the master width (0.0 to 1.0).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-rely",
        value: OptionValue::value("location"),
        detail: "Specifies the y-coordinate as a fraction of the master height (0.0 to 1.0).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value("size"),
        detail: "Specifies the width of the slave in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value("size"),
        detail: "Specifies the height of the slave in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-relwidth",
        value: OptionValue::value("size"),
        detail: "Specifies the width as a fraction of the master width (0.0 to 1.0).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-relheight",
        value: OptionValue::value("size"),
        detail: "Specifies the height as a fraction of the master height (0.0 to 1.0).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-anchor",
        value: OptionValue::enumerated(super::common::ANCHOR, true, "anchor"),
        detail: "Specifies which point of the slave is positioned at the (x,y) location.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-bordermode",
        value: OptionValue::value("inside|outside|ignore"),
        detail: "Determines the degree to which borders within the master are used.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-in",
        value: OptionValue::value("master"),
        detail: "Specifies the master window relative to which the slave is placed.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "place option arg ?arg ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "place",
        traits: Traits::TK_GEOMETRY_MANAGER,
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Geometry manager for fixed or rubber-sheet placement.",
            synopsis: &[
                "place window option value ?option value ...?",
                "place configure window ?option? ?value option value ...?",
                "place forget window",
                "place info window",
                "place slaves window",
            ],
            snippet: "",
            source: "Tk man page place.n",
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
