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

//! `spinbox` command.
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
        name: "-from",
        value: OptionValue::value(""),
        detail: "Starting value for the numeric range.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-to",
        value: OptionValue::value(""),
        detail: "Ending value for the numeric range.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-increment",
        value: OptionValue::value(""),
        detail: "Amount to increment or decrement the value on each arrow press.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-values",
        value: OptionValue::value(""),
        detail: "List of values to cycle through instead of a numeric range.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-textvariable",
        value: OptionValue::var_name(),
        detail: "Name of a variable linked to the spinbox's contents.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-width",
        value: OptionValue::value(""),
        detail: "Desired width of the spinbox in average-size characters.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value(""),
        detail: "State of the spinbox: normal, disabled, or readonly.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-format",
        value: OptionValue::value(""),
        detail: "Format string for displaying the value (e.g. %5.2f).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-wrap",
        value: OptionValue::value(""),
        detail: "Whether the value wraps around when the range limit is reached.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::script(),
        detail: "Tcl command to invoke when the value is changed via the arrows.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-validate",
        value: OptionValue::value(""),
        detail: "Validation mode: none, focus, focusin, focusout, key, or all.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-validatecommand",
        value: OptionValue::script(),
        detail: "Script to evaluate when validation is triggered.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-invalidcommand",
        value: OptionValue::script(),
        detail: "Script to evaluate when validation fails.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value(""),
        detail: "Font to use for text in the spinbox.",
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
        name: "-readonlybackground",
        value: OptionValue::value(""),
        detail: "Background colour when the spinbox is in readonly state.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-buttonbackground",
        value: OptionValue::value(""),
        detail: "Background colour of the increment/decrement buttons.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-buttoncursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the buttons.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-buttondownrelief",
        value: OptionValue::value(""),
        detail: "Relief of the down (decrement) button.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-buttonuprelief",
        value: OptionValue::value(""),
        detail: "Relief of the up (increment) button.",
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
        detail: "Background colour for selected text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selectborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around selected text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-selectforeground",
        value: OptionValue::value(""),
        detail: "Foreground colour for selected text.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertbackground",
        value: OptionValue::value(""),
        detail: "Colour of the insertion cursor.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around the insertion cursor.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertofftime",
        value: OptionValue::value(""),
        detail: "Milliseconds the insertion cursor is off during blinking.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertontime",
        value: OptionValue::value(""),
        detail: "Milliseconds the insertion cursor is on during blinking.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertwidth",
        value: OptionValue::value(""),
        detail: "Width of the insertion cursor in screen units.",
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
        name: "-exportselection",
        value: OptionValue::value(""),
        detail: "Whether the selection is exported to the X selection.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the spinbox.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value(""),
        detail: "Whether the spinbox accepts focus during keyboard traversal.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the spinbox does not have focus.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the spinbox has focus.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        value: OptionValue::value(""),
        detail: "Width of the highlight rectangle drawn around the spinbox.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-activebackground",
        value: OptionValue::value("color"),
        detail: "Specifies background color to use when drawing active elements. An element (a widget or portion of a widget).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-repeatinterval",
        value: OptionValue::value("milliseconds"),
        detail: "Used in conjunction with -repeatdelay: once auto-repeat begins, this option determines the number of.",
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
        name: "-repeatdelay",
        value: OptionValue::value("milliseconds"),
        detail: "Specifies the number of milliseconds a button or key must be held down before it begins to auto-repeat. Used.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-disabledbackground",
        value: OptionValue::value("color"),
        detail: "Specifies the background color to use when the spinbox is disabled. If this option is the empty string, the.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-disabledforeground",
        value: OptionValue::value("color"),
        detail: "Specifies the foreground color to use when the spinbox is disabled. If this option is the empty string, the.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

/// The command's subcommands.
static SUBCOMMANDS: [SubCommand; 13] = [
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
        detail: "Return the spinbox's current string contents.",
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
        name: "identify",
        arity: Arity::exact(2),
        detail: "Return the name of the spinbox element at the given coordinates.",
        synopsis: "pathName identify x y",
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
        name: "invoke",
        arity: Arity::exact(1),
        detail: "Invoke the up or down button, incrementing or decrementing the value.",
        synopsis: "pathName invoke element",
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
        detail: "Manipulate the selection; option is adjust, clear, element, from, present, range, or to.",
        synopsis: "pathName selection option ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::new(0, 1),
        detail: "Query or set the spinbox's string value.",
        synopsis: "pathName set ?string?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "validate",
        arity: Arity::exact(0),
        detail: "Force revalidation of the spinbox using its -validatecommand.",
        synopsis: "pathName validate",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xview",
        arity: Arity::at_least(0),
        detail: "Query or change the horizontal position of the text visible in the spinbox.",
        synopsis: "pathName xview ?args?",
        ..SubCommand::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "spinbox pathName ?option value ...?",
    dialects: None,
}];

/// `spinbox`'s instance command dispatches through the same subcommand
/// table as its own constructor spec (see
/// `docs/design/tk-widget-instance-typing.md`).
static SPINBOX_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "spinbox",
    instance_methods: &SUBCOMMANDS,
    superclasses: &[],
    allow_unknown_methods: false,
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "spinbox",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a spinbox widget.",
            synopsis: &["spinbox pathName ?option value ...?"],
            snippet: "Displays a single-line text field with increment and decrement arrows for cycling through a range of values.",
            source: "Tk man page spinbox.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: &SUBCOMMANDS,
        object_class: Some(&SPINBOX_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
