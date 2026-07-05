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

//! `tk_getSaveFile` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-confirmoverwrite",
        takes_value: true,
        value_hint: "boolean",
        detail: "Prompt for confirmation if the file already exists.",
        dialects: None,
    },
    OptionSpec {
        name: "-defaultextension",
        takes_value: true,
        value_hint: "extension",
        detail: "Default extension to append if the user does not type one.",
        dialects: None,
    },
    OptionSpec {
        name: "-filetypes",
        takes_value: true,
        value_hint: "filePatternList",
        detail: "List of file type patterns to display in the filter.",
        dialects: None,
    },
    OptionSpec {
        name: "-initialdir",
        takes_value: true,
        value_hint: "dirName",
        detail: "Initial directory to display.",
        dialects: None,
    },
    OptionSpec {
        name: "-initialfile",
        takes_value: true,
        value_hint: "fileName",
        detail: "Initial file name to populate in the dialogue.",
        dialects: None,
    },
    OptionSpec {
        name: "-parent",
        takes_value: true,
        value_hint: "window",
        detail: "Parent window for the dialogue.",
        dialects: None,
    },
    OptionSpec {
        name: "-title",
        takes_value: true,
        value_hint: "titleString",
        detail: "Title string for the dialogue window.",
        dialects: None,
    },
    OptionSpec {
        name: "-typevariable",
        takes_value: true,
        value_hint: "varName",
        detail: "Variable to store the selected file type.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tk_getSaveFile ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tk_getSaveFile",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Pop up a dialogue for the user to select a file to save.",
            synopsis: &["tk_getSaveFile ?option value ...?"],
            snippet: "",
            source: "Tk man page tk_getSaveFile.n",
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
