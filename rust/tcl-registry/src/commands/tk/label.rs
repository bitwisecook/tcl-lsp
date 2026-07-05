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

//! `label` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-text",
        takes_value: true,
        value_hint: "",
        detail: "Text string to be displayed in the label.",
        dialects: None,
    },
    OptionSpec {
        name: "-textvariable",
        takes_value: true,
        value_hint: "",
        detail: "Name of a variable whose value will be used as the label text.",
        dialects: None,
    },
    OptionSpec {
        name: "-image",
        takes_value: true,
        value_hint: "",
        detail: "Image to display in the label.",
        dialects: None,
    },
    OptionSpec {
        name: "-bitmap",
        takes_value: true,
        value_hint: "",
        detail: "Bitmap to display in the label.",
        dialects: None,
    },
    OptionSpec {
        name: "-compound",
        takes_value: true,
        value_hint: "",
        detail: "Whether to display both image and text: none, bottom, top, left, right, or center.",
        dialects: None,
    },
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "",
        detail: "Desired width of the label in characters (text) or pixels (image).",
        dialects: None,
    },
    OptionSpec {
        name: "-height",
        takes_value: true,
        value_hint: "",
        detail: "Desired height of the label in lines (text) or pixels (image).",
        dialects: None,
    },
    OptionSpec {
        name: "-anchor",
        takes_value: true,
        value_hint: "",
        detail: "How information is positioned: n, ne, e, se, s, sw, w, nw, or center.",
        dialects: None,
    },
    OptionSpec {
        name: "-justify",
        takes_value: true,
        value_hint: "",
        detail: "Justification of multi-line text: left, center, or right.",
        dialects: None,
    },
    OptionSpec {
        name: "-wraplength",
        takes_value: true,
        value_hint: "",
        detail: "Maximum line length for word wrapping, in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-relief",
        takes_value: true,
        value_hint: "",
        detail: "3-D effect: flat, groove, raised, ridge, solid, or sunken.",
        dialects: None,
    },
    OptionSpec {
        name: "-bg",
        takes_value: true,
        value_hint: "",
        detail: "Shorthand for -background.",
        dialects: None,
    },
    OptionSpec {
        name: "-fg",
        takes_value: true,
        value_hint: "",
        detail: "Shorthand for -foreground.",
        dialects: None,
    },
    OptionSpec {
        name: "-font",
        takes_value: true,
        value_hint: "",
        detail: "Font to use for the label text.",
        dialects: None,
    },
    OptionSpec {
        name: "-padx",
        takes_value: true,
        value_hint: "",
        detail: "Extra horizontal padding inside the label.",
        dialects: None,
    },
    OptionSpec {
        name: "-pady",
        takes_value: true,
        value_hint: "",
        detail: "Extra vertical padding inside the label.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over the label.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "",
        detail: "Whether the label accepts focus during keyboard traversal.",
        dialects: None,
    },
    OptionSpec {
        name: "-underline",
        takes_value: true,
        value_hint: "",
        detail: "Index of character to underline for keyboard traversal (0-based).",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the label does not have focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the label has focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        takes_value: true,
        value_hint: "",
        detail: "Width of the highlight rectangle drawn around the label.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "label pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "label",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a label widget.",
            synopsis: &["label pathName ?option value ...?"],
            snippet: "Displays a textual string, bitmap, or image. A label is a non-interactive widget.",
            source: "Tk man page label.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
