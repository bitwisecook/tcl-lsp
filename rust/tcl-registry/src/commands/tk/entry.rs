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
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-textvariable",
        value: OptionValue::var_name(),
        detail: "Name of a variable linked to the entry's contents.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value(""),
        detail: "Desired width of the entry in average-size characters.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value(""),
        detail: "State of the entry: normal, disabled, or readonly.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-show",
        value: OptionValue::value(""),
        detail: "Character to display instead of actual contents (e.g. '*' for passwords).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value(""),
        detail: "Font to use for text in the entry.",
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
        name: "-fg",
        value: OptionValue::value(""),
        detail: "Shorthand for -foreground.",
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
    OptionSpec {
        name: "-relief",
        value: OptionValue::enumerated(super::common::RELIEF, true, "relief"),
        detail: "3-D effect: flat, groove, raised, ridge, solid, or sunken.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-justify",
        value: OptionValue::enumerated(super::common::JUSTIFY, true, "justify"),
        detail: "Justification of text within the entry: left, center, or right.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-insertbackground",
        value: OptionValue::value(""),
        detail: "Colour of the insertion cursor.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-insertborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around the insertion cursor.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-insertofftime",
        value: OptionValue::value(""),
        detail: "Milliseconds the insertion cursor is off during blinking.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-insertontime",
        value: OptionValue::value(""),
        detail: "Milliseconds the insertion cursor is on during blinking.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-insertwidth",
        value: OptionValue::value(""),
        detail: "Width of the insertion cursor in screen units.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-selectbackground",
        value: OptionValue::value(""),
        detail: "Background colour for selected text.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-selectborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around selected text.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-selectforeground",
        value: OptionValue::value(""),
        detail: "Foreground colour for selected text.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-xscrollcommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for communicating with horizontal scrollbars.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-exportselection",
        value: OptionValue::value(""),
        detail: "Whether the selection is exported to the X selection.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-readonlybackground",
        value: OptionValue::value(""),
        detail: "Background colour when the entry is in readonly state.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-validate",
        value: OptionValue::value(""),
        detail: "Validation mode: none, focus, focusin, focusout, key, or all.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-validatecommand",
        value: OptionValue::script(),
        detail: "Script to evaluate when validation is triggered.",
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
        name: "-cursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the entry.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value(""),
        detail: "Whether the entry accepts focus during keyboard traversal.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the entry does not have focus.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the entry has focus.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        value: OptionValue::value(""),
        detail: "Width of the highlight rectangle drawn around the entry.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-disabledbackground",
        value: OptionValue::value("color"),
        detail: "Specifies the background color to use when the entry is disabled. If this option is the empty string, the.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-disabledforeground",
        value: OptionValue::value("color"),
        detail: "Specifies the foreground color to use when the entry is disabled. If this option is the empty string, the.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
];

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "bbox",
        arity: Arity::exact(1),
        detail: "Return the bounding box of the character at the given index.",
        synopsis: "pathName bbox index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::new(1, 2),
        detail: "Delete characters from first through last (or just the character at first).",
        synopsis: "pathName delete first ?last?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::exact(0),
        detail: "Return the entry's current string contents.",
        synopsis: "pathName get",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "icursor",
        arity: Arity::exact(1),
        detail: "Move the insertion cursor to just before the character at the given index.",
        synopsis: "pathName icursor index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(1),
        detail: "Return the numerical index corresponding to the given index.",
        synopsis: "pathName index index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::exact(2),
        detail: "Insert the string just before the character at the given index.",
        synopsis: "pathName insert index string",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::exact(2),
        detail: "Implement fast scanning/scrolling; option is mark or dragto.",
        synopsis: "pathName scan option arg",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "selection",
        arity: Arity::at_least(1),
        detail: "Manipulate the selection; option is adjust, clear, from, present, range, or to.",
        synopsis: "pathName selection option ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "validate",
        arity: Arity::exact(0),
        detail: "Force revalidation of the entry using its -validatecommand.",
        synopsis: "pathName validate",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xview",
        arity: Arity::at_least(0),
        detail: "Query or change the horizontal position of the text visible in the entry.",
        synopsis: "pathName xview ?args?",
        ..SubCommand::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "entry pathName ?option value ...?",
    dialects: None,
}];

/// `entry`'s instance command dispatches through the same subcommand
/// table as its own constructor spec (see
/// `docs/design/tk-widget-instance-typing.md`).
static ENTRY_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "entry",
    instance_methods: SUBCOMMANDS,
    superclasses: &[],
    allow_unknown_methods: false,
};

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
        subcommands: SUBCOMMANDS,
        object_class: Some(&ENTRY_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
