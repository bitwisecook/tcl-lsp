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

//! `ttk::progressbar` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-orient",
        takes_value: true,
        value_hint: "orientation",
        detail: "Orientation of the progress bar (horizontal or vertical).",
        dialects: None,
    },
    OptionSpec {
        name: "-length",
        takes_value: true,
        value_hint: "length",
        detail: "Length of the long axis of the progress bar.",
        dialects: None,
    },
    OptionSpec {
        name: "-mode",
        takes_value: true,
        value_hint: "progressMode",
        detail: "Mode of the progress bar (determinate or indeterminate).",
        dialects: None,
    },
    OptionSpec {
        name: "-maximum",
        takes_value: true,
        value_hint: "maximum",
        detail: "Maximum value of the progress bar.",
        dialects: None,
    },
    OptionSpec {
        name: "-value",
        takes_value: true,
        value_hint: "value",
        detail: "Current value of the progress bar.",
        dialects: None,
    },
    OptionSpec {
        name: "-variable",
        takes_value: true,
        value_hint: "varName",
        detail: "Variable linked to the progress bar value.",
        dialects: None,
    },
    OptionSpec {
        name: "-phase",
        takes_value: true,
        value_hint: "phase",
        detail: "Read-only value used by the theme engine for animation.",
        dialects: None,
    },
    OptionSpec {
        name: "-style",
        takes_value: true,
        value_hint: "style",
        detail: "Style to use for the widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-class",
        takes_value: true,
        value_hint: "className",
        detail: "Widget class name for option-database lookups.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "cursor",
        detail: "Cursor to display when the pointer is over the widget.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "focusSpec",
        detail: "Whether the widget accepts focus during keyboard traversal.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ttk::progressbar pathName ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::progressbar",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed progress indicator widget.",
            synopsis: &["ttk::progressbar pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_progressbar.n",
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
