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

//! `scale` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-from",
        takes_value: true,
        value_hint: "",
        detail: "Starting value of the range (a real number).",
        dialects: None,
    },
    OptionSpec {
        name: "-to",
        takes_value: true,
        value_hint: "",
        detail: "Ending value of the range (a real number).",
        dialects: None,
    },
    OptionSpec {
        name: "-variable",
        takes_value: true,
        value_hint: "",
        detail: "Name of a variable linked to the scale's current value.",
        dialects: None,
    },
    OptionSpec {
        name: "-orient",
        takes_value: true,
        value_hint: "",
        detail: "Orientation of the scale: horizontal or vertical.",
        dialects: None,
    },
    OptionSpec {
        name: "-resolution",
        takes_value: true,
        value_hint: "",
        detail: "Resolution (step size) for the scale value.",
        dialects: None,
    },
    OptionSpec {
        name: "-tickinterval",
        takes_value: true,
        value_hint: "",
        detail: "Spacing between numerical tick marks displayed along the scale.",
        dialects: None,
    },
    OptionSpec {
        name: "-label",
        takes_value: true,
        value_hint: "",
        detail: "Text label to display alongside the scale.",
        dialects: None,
    },
    OptionSpec {
        name: "-length",
        takes_value: true,
        value_hint: "",
        detail: "Desired long dimension of the scale in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "",
        detail: "Desired narrow dimension of the trough in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-sliderlength",
        takes_value: true,
        value_hint: "",
        detail: "Length of the slider along the long dimension in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-sliderrelief",
        takes_value: true,
        value_hint: "",
        detail: "Relief of the slider: flat, groove, raised, ridge, solid, or sunken.",
        dialects: None,
    },
    OptionSpec {
        name: "-showvalue",
        takes_value: true,
        value_hint: "",
        detail: "Whether to display the current value next to the slider.",
        dialects: None,
    },
    OptionSpec {
        name: "-digits",
        takes_value: true,
        value_hint: "",
        detail: "Number of significant digits for the scale value.",
        dialects: None,
    },
    OptionSpec {
        name: "-bigincrement",
        takes_value: true,
        value_hint: "",
        detail: "Large increment used for Control-arrow key bindings.",
        dialects: None,
    },
    OptionSpec {
        name: "-command",
        takes_value: true,
        value_hint: "",
        detail: "Tcl command prefix invoked when the scale value changes.",
        dialects: None,
    },
    OptionSpec {
        name: "-state",
        takes_value: true,
        value_hint: "",
        detail: "State of the scale: normal, active, or disabled.",
        dialects: None,
    },
    OptionSpec {
        name: "-font",
        takes_value: true,
        value_hint: "",
        detail: "Font to use for the label and value display.",
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
        name: "-troughcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the trough area.",
        dialects: None,
    },
    OptionSpec {
        name: "-activebackground",
        takes_value: true,
        value_hint: "",
        detail: "Background colour when the slider is active (mouse over).",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the scale does not have focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the scale has focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        takes_value: true,
        value_hint: "",
        detail: "Width of the highlight rectangle drawn around the scale.",
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
        name: "-borderwidth",
        takes_value: true,
        value_hint: "",
        detail: "Width of the border around the scale.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over the scale.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "",
        detail: "Whether the scale accepts focus during keyboard traversal.",
        dialects: None,
    },
    OptionSpec {
        name: "-repeatdelay",
        takes_value: true,
        value_hint: "",
        detail: "Milliseconds before auto-repeat begins when trough is held.",
        dialects: None,
    },
    OptionSpec {
        name: "-repeatinterval",
        takes_value: true,
        value_hint: "",
        detail: "Milliseconds between auto-repeat invocations.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "scale pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "scale",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a scale (slider) widget.",
            synopsis: &["scale pathName ?option value ...?"],
            snippet: "Displays a slider that allows the user to select a numerical value from a specified range.",
            source: "Tk man page scale.n",
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
