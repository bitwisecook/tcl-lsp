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

//! `listbox` command.
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
        name: "-listvariable",
        value: OptionValue::var_name(),
        detail: "Name of a variable containing the list of values to display.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selectmode",
        value: OptionValue::value(""),
        detail: "Selection mode: single, browse, multiple, or extended.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value(""),
        detail: "Desired width of the listbox in characters.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value(""),
        detail: "Desired height of the listbox in lines.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value(""),
        detail: "Font to use for text in the listbox.",
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
        name: "-fg",
        value: OptionValue::value(""),
        detail: "Shorthand for -foreground.",
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
        name: "-selectbackground",
        value: OptionValue::value(""),
        detail: "Background colour for selected items.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selectborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around selected items.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selectforeground",
        value: OptionValue::value(""),
        detail: "Foreground colour for selected items.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-xscrollcommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for communicating with horizontal scrollbars.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-yscrollcommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for communicating with vertical scrollbars.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-exportselection",
        value: OptionValue::value(""),
        detail: "Whether the selection is exported to the X selection.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-setgrid",
        value: OptionValue::value(""),
        detail: "Whether this widget controls the resizing grid for its toplevel.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-activestyle",
        value: OptionValue::value(""),
        detail: "Style for the active element: dotbox, none, or underline.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the listbox.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value(""),
        detail: "Whether the listbox accepts focus during keyboard traversal.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the listbox does not have focus.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the listbox has focus.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        value: OptionValue::value(""),
        detail: "Width of the highlight rectangle drawn around the listbox.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-disabledforeground",
        value: OptionValue::value("color"),
        detail: "Specifies foreground color to use when drawing a disabled element. If the option is specified as an empty.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-justify",
        value: OptionValue::value("justify"),
        detail: "When there are multiple lines of text displayed in a widget, this option determines how the lines line up.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Specifies one of two states for the listbox: normal or disabled. If the listbox is disabled then items may.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// The command's subcommands.
static SUBCOMMANDS: [SubCommand; 16] = [
    SubCommand {
        name: "activate",
        arity: Arity::exact(1),
        detail: "Set the active element to the one at the given index.",
        synopsis: "pathName activate index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "bbox",
        arity: Arity::exact(1),
        detail: "Return the bounding box of the text of the element at the given index.",
        synopsis: "pathName bbox index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "curselection",
        arity: Arity::exact(0),
        detail: "Return a list of the indices of all currently selected elements.",
        synopsis: "pathName curselection",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::new(1, 2),
        detail: "Delete one or more elements in the range first through last inclusive.",
        synopsis: "pathName delete first ?last?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::new(1, 2),
        detail: "Return the contents of the elements in the range first through last inclusive.",
        synopsis: "pathName get first ?last?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(1),
        detail: "Return the integer index value corresponding to the given index.",
        synopsis: "pathName index index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(1),
        detail: "Insert zero or more new elements just before the element at the given index.",
        synopsis: "pathName insert index ?element ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "itemcget",
        arity: Arity::exact(2),
        detail: "Return the current value of the given configuration option for an item.",
        synopsis: "pathName itemcget index option",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "itemconfigure",
        arity: Arity::at_least(1),
        detail: "Query or modify the configuration options of an individual item.",
        synopsis: "pathName itemconfigure index ?option? ?value option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nearest",
        arity: Arity::exact(1),
        detail: "Return the index of the visible element nearest to the given y-coordinate.",
        synopsis: "pathName nearest y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::exact(3),
        detail: "Implement scanning: record scan mark, or scroll relative to a mark.",
        synopsis: "pathName scan mark|dragto x y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "see",
        arity: Arity::exact(1),
        detail: "Adjust the view so that the element at the given index is visible.",
        synopsis: "pathName see index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "selection",
        arity: Arity::at_least(1),
        detail: "Adjust or query the selection: anchor, clear, includes, or set.",
        synopsis: "pathName selection option first ?last?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        arity: Arity::exact(0),
        detail: "Return the number of elements in the listbox.",
        synopsis: "pathName size",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xview",
        arity: Arity::at_least(0),
        detail: "Query or change the horizontal position of the listbox's view.",
        synopsis: "pathName xview ?args?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "yview",
        arity: Arity::at_least(0),
        detail: "Query or change the vertical position of the listbox's view.",
        synopsis: "pathName yview ?args?",
        ..SubCommand::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "listbox pathName ?option value ...?",
    dialects: None,
}];

/// `listbox`'s instance command dispatches through the same subcommand
/// table as its own constructor spec (see
/// `docs/design/tk-widget-instance-typing.md`).
static LISTBOX_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "listbox",
    instance_methods: &SUBCOMMANDS,
    superclasses: &[],
    allow_unknown_methods: false,
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "listbox",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a listbox widget.",
            synopsis: &["listbox pathName ?option value ...?"],
            snippet: "Displays a list of strings, one per line, and allows the user to select one or more of them.",
            source: "Tk man page listbox.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: &SUBCOMMANDS,
        object_class: Some(&LISTBOX_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
