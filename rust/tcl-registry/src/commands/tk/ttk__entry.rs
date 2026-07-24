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

//! `ttk::entry` command.
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
        name: "-textvariable",
        value: OptionValue::var_name(),
        detail: "Variable linked to the entry value.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value("width"),
        detail: "Desired width of the entry in characters.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Widget state (normal, disabled, or readonly).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-show",
        value: OptionValue::value("char"),
        detail: "Character to display instead of actual contents (e.g. for passwords).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-validate",
        value: OptionValue::value("validateMode"),
        detail: "When to run validation (none, focus, focusin, focusout, key, all).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-validatecommand",
        value: OptionValue::script(),
        detail: "Script to evaluate for input validation.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-invalidcommand",
        value: OptionValue::script(),
        detail: "Script to evaluate when validation fails.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-xscrollcommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for horizontal scroll communication.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-exportselection",
        value: OptionValue::value("boolean"),
        detail: "Whether the selection is exported to the X selection.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value("font"),
        detail: "Font to use for the entry text.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-foreground",
        value: OptionValue::value("colour"),
        detail: "Foreground colour for the entry text.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-justify",
        value: OptionValue::enumerated(super::common::JUSTIFY, true, "justify"),
        detail: "How to justify the text within the entry.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-style",
        value: OptionValue::value("style"),
        detail: "Style to use for the widget.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-class",
        value: OptionValue::value("className"),
        detail: "Widget class name for option-database lookups.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value("cursor"),
        detail: "Cursor to display when the pointer is over the widget.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value("focusSpec"),
        detail: "Whether the widget accepts focus during keyboard traversal.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-placeholder",
        value: OptionValue::value("text"),
        detail: "Help text shown when the entry is empty (Tk 8.7+).",
        dialects: None,
        aliases: &[],
        min_version: Some("8.7"),
    },
    OptionSpec {
        name: "-placeholderforeground",
        value: OptionValue::value("color"),
        detail: "Foreground colour of the placeholder text (Tk 8.7+).",
        dialects: None,
        aliases: &[],
        min_version: Some("8.7"),
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ttk::entry pathName ?options?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ttk::entry",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a themed text entry widget.",
            synopsis: &["ttk::entry pathName ?options?"],
            snippet: "",
            source: "Tk man page ttk_entry.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        min_version: Some("8.5"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
