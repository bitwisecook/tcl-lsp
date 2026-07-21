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

//! `button` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-text",
        value: OptionValue::value(""),
        detail: "Text to display in the button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-textvariable",
        value: OptionValue::var_name(),
        detail: "Name of a variable whose value will be used as the button text.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::script(),
        detail: "Tcl command to invoke when the button is pressed.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value(""),
        detail: "State of the button: normal, active, or disabled.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value(""),
        detail: "Desired width of the button in characters (text) or pixels (image).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value(""),
        detail: "Desired height of the button in lines (text) or pixels (image).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-relief",
        value: OptionValue::enumerated(super::common::RELIEF, true, "relief"),
        detail: "3-D effect: flat, groove, raised, ridge, solid, or sunken.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-bg",
        value: OptionValue::value(""),
        detail: "Shorthand for -background.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-background",
        value: OptionValue::value(""),
        detail: "Normal background colour of the button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-fg",
        value: OptionValue::value(""),
        detail: "Shorthand for -foreground.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-foreground",
        value: OptionValue::value(""),
        detail: "Normal foreground colour of the button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value(""),
        detail: "Font to use for the button text.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-image",
        value: OptionValue::value(""),
        detail: "Image to display in the button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-bitmap",
        value: OptionValue::value(""),
        detail: "Bitmap to display in the button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-compound",
        value: OptionValue::value(""),
        detail: "Whether to display both image and text: none, bottom, top, left, right, or center.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-padx",
        value: OptionValue::value(""),
        detail: "Extra horizontal padding inside the button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-pady",
        value: OptionValue::value(""),
        detail: "Extra vertical padding inside the button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-anchor",
        value: OptionValue::enumerated(super::common::ANCHOR, true, "anchor"),
        detail: "How information is positioned: n, ne, e, se, s, sw, w, nw, or center.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-justify",
        value: OptionValue::enumerated(super::common::JUSTIFY, true, "justify"),
        detail: "Justification of multi-line text: left, center, or right.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-wraplength",
        value: OptionValue::value(""),
        detail: "Maximum line length for word wrapping, in screen units.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-underline",
        value: OptionValue::value(""),
        detail: "Index of character to underline for keyboard traversal (0-based).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-activebackground",
        value: OptionValue::value(""),
        detail: "Background colour when the button is active (mouse over).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-activeforeground",
        value: OptionValue::value(""),
        detail: "Foreground colour when the button is active.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-disabledforeground",
        value: OptionValue::value(""),
        detail: "Foreground colour when the button is disabled.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the button does not have focus.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the button has focus.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        value: OptionValue::value(""),
        detail: "Width of the highlight rectangle drawn around the button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value(""),
        detail: "Whether the button accepts focus during keyboard traversal.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-repeatdelay",
        value: OptionValue::value(""),
        detail: "Milliseconds before auto-repeat begins when button is held.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-repeatinterval",
        value: OptionValue::value(""),
        detail: "Milliseconds between auto-repeat invocations.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-overrelief",
        value: OptionValue::value(""),
        detail: "Relief to use when the mouse cursor is over the button.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-default",
        value: OptionValue::value(""),
        detail: "Default ring state: normal, active, or disabled.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "button pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "button",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a button widget.",
            synopsis: &["button pathName ?option value ...?"],
            snippet: "Displays a textual string, bitmap, or image. When pressed, invokes a Tcl command.",
            source: "Tk man page button.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
