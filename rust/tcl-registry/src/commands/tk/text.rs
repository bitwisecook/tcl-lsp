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

//! `text` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "",
        detail: "Desired width of the text widget in characters.",
        dialects: None,
    },
    OptionSpec {
        name: "-height",
        takes_value: true,
        value_hint: "",
        detail: "Desired height of the text widget in lines.",
        dialects: None,
    },
    OptionSpec {
        name: "-wrap",
        takes_value: true,
        value_hint: "",
        detail: "Line wrapping mode: none, char, or word.",
        dialects: None,
    },
    OptionSpec {
        name: "-state",
        takes_value: true,
        value_hint: "",
        detail: "State of the text widget: normal or disabled.",
        dialects: None,
    },
    OptionSpec {
        name: "-font",
        takes_value: true,
        value_hint: "",
        detail: "Font to use for text in the widget.",
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
        name: "-relief",
        takes_value: true,
        value_hint: "",
        detail: "3-D effect: flat, groove, raised, ridge, solid, or sunken.",
        dialects: None,
    },
    OptionSpec {
        name: "-spacing1",
        takes_value: true,
        value_hint: "",
        detail: "Extra space above each line of text, in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-spacing2",
        takes_value: true,
        value_hint: "",
        detail: "Extra space between display lines within a logical line, in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-spacing3",
        takes_value: true,
        value_hint: "",
        detail: "Extra space below each line of text, in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-tabs",
        takes_value: true,
        value_hint: "",
        detail: "Tab stop positions and alignment for the text widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-insertbackground",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the insertion cursor.",
        dialects: None,
    },
    OptionSpec {
        name: "-insertborderwidth",
        takes_value: true,
        value_hint: "",
        detail: "Width of the border around the insertion cursor.",
        dialects: None,
    },
    OptionSpec {
        name: "-insertofftime",
        takes_value: true,
        value_hint: "",
        detail: "Milliseconds the insertion cursor is off during blinking.",
        dialects: None,
    },
    OptionSpec {
        name: "-insertontime",
        takes_value: true,
        value_hint: "",
        detail: "Milliseconds the insertion cursor is on during blinking.",
        dialects: None,
    },
    OptionSpec {
        name: "-insertwidth",
        takes_value: true,
        value_hint: "",
        detail: "Width of the insertion cursor in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-selectbackground",
        takes_value: true,
        value_hint: "",
        detail: "Background colour for selected text.",
        dialects: None,
    },
    OptionSpec {
        name: "-selectborderwidth",
        takes_value: true,
        value_hint: "",
        detail: "Width of the border around selected text.",
        dialects: None,
    },
    OptionSpec {
        name: "-selectforeground",
        takes_value: true,
        value_hint: "",
        detail: "Foreground colour for selected text.",
        dialects: None,
    },
    OptionSpec {
        name: "-xscrollcommand",
        takes_value: true,
        value_hint: "",
        detail: "Command prefix for communicating with horizontal scrollbars.",
        dialects: None,
    },
    OptionSpec {
        name: "-yscrollcommand",
        takes_value: true,
        value_hint: "",
        detail: "Command prefix for communicating with vertical scrollbars.",
        dialects: None,
    },
    OptionSpec {
        name: "-exportselection",
        takes_value: true,
        value_hint: "",
        detail: "Whether the selection is exported to the X selection.",
        dialects: None,
    },
    OptionSpec {
        name: "-setgrid",
        takes_value: true,
        value_hint: "",
        detail: "Whether this widget controls the resizing grid for its toplevel.",
        dialects: None,
    },
    OptionSpec {
        name: "-padx",
        takes_value: true,
        value_hint: "",
        detail: "Extra horizontal padding inside the text widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-pady",
        takes_value: true,
        value_hint: "",
        detail: "Extra vertical padding inside the text widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-undo",
        takes_value: true,
        value_hint: "",
        detail: "Whether the undo mechanism is active.",
        dialects: None,
    },
    OptionSpec {
        name: "-maxundo",
        takes_value: true,
        value_hint: "",
        detail: "Maximum number of compound undo actions on the undo stack.",
        dialects: None,
    },
    OptionSpec {
        name: "-autoseparators",
        takes_value: true,
        value_hint: "",
        detail: "Whether undo separators are inserted automatically.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over the text widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "",
        detail: "Whether the text widget accepts focus during keyboard traversal.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the widget does not have focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the widget has focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        takes_value: true,
        value_hint: "",
        detail: "Width of the highlight rectangle drawn around the widget.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "text pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "text",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a multi-line text widget.",
            synopsis: &["text pathName ?option value ...?"],
            snippet: "Displays one or more lines of text and allows the user to edit them. Supports embedded images and windows.",
            source: "Tk man page text.n",
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
