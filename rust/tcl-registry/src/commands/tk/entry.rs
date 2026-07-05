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

//! `entry` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-textvariable",
        takes_value: true,
        value_hint: "",
        detail: "Name of a variable linked to the entry's contents.",
        dialects: None,
    },
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "",
        detail: "Desired width of the entry in average-size characters.",
        dialects: None,
    },
    OptionSpec {
        name: "-state",
        takes_value: true,
        value_hint: "",
        detail: "State of the entry: normal, disabled, or readonly.",
        dialects: None,
    },
    OptionSpec {
        name: "-show",
        takes_value: true,
        value_hint: "",
        detail: "Character to display instead of actual contents (e.g. '*' for passwords).",
        dialects: None,
    },
    OptionSpec {
        name: "-font",
        takes_value: true,
        value_hint: "",
        detail: "Font to use for text in the entry.",
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
        name: "-justify",
        takes_value: true,
        value_hint: "",
        detail: "Justification of text within the entry: left, center, or right.",
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
        name: "-exportselection",
        takes_value: true,
        value_hint: "",
        detail: "Whether the selection is exported to the X selection.",
        dialects: None,
    },
    OptionSpec {
        name: "-readonlybackground",
        takes_value: true,
        value_hint: "",
        detail: "Background colour when the entry is in readonly state.",
        dialects: None,
    },
    OptionSpec {
        name: "-validate",
        takes_value: true,
        value_hint: "",
        detail: "Validation mode: none, focus, focusin, focusout, key, or all.",
        dialects: None,
    },
    OptionSpec {
        name: "-validatecommand",
        takes_value: true,
        value_hint: "",
        detail: "Script to evaluate when validation is triggered.",
        dialects: None,
    },
    OptionSpec {
        name: "-invalidcommand",
        takes_value: true,
        value_hint: "",
        detail: "Script to evaluate when validation fails.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over the entry.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "",
        detail: "Whether the entry accepts focus during keyboard traversal.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the entry does not have focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the entry has focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        takes_value: true,
        value_hint: "",
        detail: "Width of the highlight rectangle drawn around the entry.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "entry pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "entry",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a single-line text entry widget.",
            synopsis: &["entry pathName ?option value ...?"],
            snippet: "Displays a one-line text string and allows the user to edit it using standard editing characters.",
            source: "Tk man page entry.n",
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
