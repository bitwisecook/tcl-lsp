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

//! `panedwindow` command.
use crate::prelude::*;

/// The command's subcommands.
static SUBCOMMANDS: [SubCommand; 8] = [
    SubCommand {
        name: "add",
        arity: Arity::at_least(1),
        detail: "Add one or more child windows to the panedwindow.",
        synopsis: "pathName add window ?window ...? ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::exact(1),
        detail: "Remove the specified pane from the panedwindow.",
        synopsis: "pathName forget window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "identify",
        arity: Arity::exact(2),
        detail: "Identify the panedwindow component at the given coordinates.",
        synopsis: "pathName identify x y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "panecget",
        arity: Arity::exact(2),
        detail: "Return the value of a configuration option for a pane.",
        synopsis: "pathName panecget window option",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "paneconfigure",
        arity: Arity::at_least(1),
        detail: "Query or modify configuration options of a pane.",
        synopsis: "pathName paneconfigure window ?option? ?value option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "panes",
        arity: Arity::exact(0),
        detail: "Return a list of all panes in the panedwindow.",
        synopsis: "pathName panes",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "proxy",
        arity: Arity::at_least(0),
        detail: "Query or manipulate the sash-drag proxy.",
        synopsis: "pathName proxy ?args?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sash",
        arity: Arity::at_least(1),
        detail: "Query or adjust sash positions.",
        synopsis: "pathName sash option ?arg ...?",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    writes: true,
    ..SideEffect::DEFAULT
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-orient",
        value: OptionValue::value(""),
        detail: "Orientation of the panedwindow: horizontal or vertical.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-sashwidth",
        value: OptionValue::value(""),
        detail: "Width of each sash in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-sashrelief",
        value: OptionValue::value(""),
        detail: "Relief of the sashes: flat, groove, raised, ridge, solid, or sunken.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-sashpad",
        value: OptionValue::value(""),
        detail: "Extra padding on each side of a sash.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-sashcursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over a sash.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-showhandle",
        value: OptionValue::value(""),
        detail: "Whether to display handles on the sashes.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-handlesize",
        value: OptionValue::value(""),
        detail: "Size of the sash handle in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-handlepad",
        value: OptionValue::value(""),
        detail: "Distance from the top or left end of a sash to the handle.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-opaqueresize",
        value: OptionValue::value(""),
        detail: "Whether panes are resized continuously as the sash is dragged.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value(""),
        detail: "Desired width of the panedwindow in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value(""),
        detail: "Desired height of the panedwindow in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-bg",
        value: OptionValue::value(""),
        detail: "Shorthand for -background.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-background",
        value: OptionValue::value(""),
        detail: "Background colour of the panedwindow.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-relief",
        value: OptionValue::enumerated(super::common::RELIEF, true, "relief"),
        detail: "3-D effect: flat, groove, raised, ridge, solid, or sunken.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-borderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around the panedwindow.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the panedwindow.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value(""),
        detail: "Whether the panedwindow accepts focus during keyboard traversal.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the panedwindow does not have focus.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the panedwindow has focus.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        value: OptionValue::value(""),
        detail: "Width of the highlight rectangle drawn around the panedwindow.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "panedwindow pathName ?option value ...?",
    ..FormSpec::DEFAULT
}];

/// `panedwindow`'s instance command dispatches through the same subcommand
/// table as its own constructor spec (see
/// `docs/design/tk-widget-instance-typing.md`).
static PANEDWINDOW_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "panedwindow",
    instance_methods: &SUBCOMMANDS,
    superclasses: &[],
    allow_unknown_methods: false,
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "panedwindow",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a panedwindow widget.",
            synopsis: &["panedwindow pathName ?option value ...?"],
            snippet: "Displays a container that divides its space among child widgets separated by movable sashes.",
            source: "Tk man page panedwindow.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: &SUBCOMMANDS,
        object_class: Some(&PANEDWINDOW_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
