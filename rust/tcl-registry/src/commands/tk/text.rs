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

/// `text` `tag` sub-subcommand roles. `pathName tag bind tagName sequence script`
/// binds a deferred event-handler script (run from the Tk event loop) as its
/// trailing word. Args here are those AFTER the `tag` subcommand word:
/// `bind`(0) tagName(1) sequence(2) script(3).
fn text_tag_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.first() == Some(&"bind") && args.len() == 4 {
        vec![(3, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

/// The command's subcommands.
static SUBCOMMANDS: [SubCommand; 24] = [
    SubCommand {
        name: "bbox",
        arity: Arity::exact(1),
        detail: "Return the bounding box of the character at the given index.",
        synopsis: "pathName bbox index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "compare",
        arity: Arity::exact(3),
        detail: "Compare two indices according to a relational operator.",
        synopsis: "pathName compare index1 op index2",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "count",
        arity: Arity::at_least(2),
        detail: "Count the number of items between two indices.",
        synopsis: "pathName count ?option ...? index1 index2",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "debug",
        arity: Arity::new(0, 1),
        detail: "Enable or query consistency checking of the B-tree code.",
        synopsis: "pathName debug ?boolean?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(1),
        detail: "Delete a range of characters from the text.",
        synopsis: "pathName delete index1 ?index2 ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dlineinfo",
        arity: Arity::exact(1),
        detail: "Return display information for the display line containing index.",
        synopsis: "pathName dlineinfo index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dump",
        arity: Arity::at_least(1),
        detail: "Return the contents of the text widget in a parseable form.",
        synopsis: "pathName dump ?switches? index1 ?index2?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "edit",
        arity: Arity::at_least(1),
        detail: "Control the undo/redo mechanism and modified flag.",
        synopsis: "pathName edit option ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::at_least(1),
        detail: "Return the text from the widget between the given indices.",
        synopsis: "pathName get ?-displaychars? ?--? index1 ?index2 ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "image",
        arity: Arity::at_least(1),
        detail: "Manipulate images embedded in the text widget.",
        synopsis: "pathName image option ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(1),
        detail: "Return the position of index in line.char form.",
        synopsis: "pathName index index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::at_least(2),
        detail: "Insert text at the given index.",
        synopsis: "pathName insert index chars ?tagList chars tagList ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mark",
        arity: Arity::at_least(1),
        detail: "Manipulate marks within the text widget.",
        synopsis: "pathName mark option ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "peer",
        arity: Arity::at_least(1),
        detail: "Create or list peer text widgets.",
        synopsis: "pathName peer option ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pendingsync",
        arity: Arity::exact(0),
        detail: "Return whether asynchronous line-height calculations are pending.",
        synopsis: "pathName pendingsync",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "replace",
        arity: Arity::at_least(2),
        detail: "Replace a range of text with new text.",
        synopsis: "pathName replace index1 index2 chars ?tagList chars tagList ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::at_least(2),
        detail: "Implement scanning (fast dragging) of the text widget.",
        synopsis: "pathName scan option args",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "search",
        arity: Arity::at_least(2),
        detail: "Search for text matching a pattern within the widget.",
        synopsis: "pathName search ?switches? pattern index ?stopIndex?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "see",
        arity: Arity::exact(1),
        detail: "Scroll the widget so the character at index is visible.",
        synopsis: "pathName see index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sync",
        arity: Arity::at_least(0),
        detail: "Control or query asynchronous line-height calculations.",
        synopsis: "pathName sync ?-command command?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tag",
        arity: Arity::at_least(1),
        detail: "Manipulate tags applied to ranges of text.",
        synopsis: "pathName tag option ?arg ...?",
        arg_role_resolver: Some(text_tag_arg_roles),
        body_kind: BodyKind::Structural,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "window",
        arity: Arity::at_least(1),
        detail: "Manipulate embedded windows within the text widget.",
        synopsis: "pathName window option ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xview",
        arity: Arity::at_least(0),
        detail: "Query or change the horizontal position of the text in the window.",
        synopsis: "pathName xview ?args?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "yview",
        arity: Arity::at_least(0),
        detail: "Query or change the vertical position of the text in the window.",
        synopsis: "pathName yview ?args?",
        ..SubCommand::DEFAULT
    },
];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-width",
        value: OptionValue::value(""),
        detail: "Desired width of the text widget in characters.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value(""),
        detail: "Desired height of the text widget in lines.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-wrap",
        value: OptionValue::value(""),
        detail: "Line wrapping mode: none, char, or word.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value(""),
        detail: "State of the text widget: normal or disabled.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-font",
        value: OptionValue::value(""),
        detail: "Font to use for text in the widget.",
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
        name: "-spacing1",
        value: OptionValue::value(""),
        detail: "Extra space above each line of text, in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-spacing2",
        value: OptionValue::value(""),
        detail: "Extra space between display lines within a logical line, in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-spacing3",
        value: OptionValue::value(""),
        detail: "Extra space below each line of text, in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-tabs",
        value: OptionValue::value(""),
        detail: "Tab stop positions and alignment for the text widget.",
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
        name: "-padx",
        value: OptionValue::value(""),
        detail: "Extra horizontal padding inside the text widget.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-pady",
        value: OptionValue::value(""),
        detail: "Extra vertical padding inside the text widget.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-undo",
        value: OptionValue::value(""),
        detail: "Whether the undo mechanism is active.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-maxundo",
        value: OptionValue::value(""),
        detail: "Maximum number of compound undo actions on the undo stack.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-autoseparators",
        value: OptionValue::value(""),
        detail: "Whether undo separators are inserted automatically.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the text widget.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value(""),
        detail: "Whether the text widget accepts focus during keyboard traversal.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the widget does not have focus.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the widget has focus.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        value: OptionValue::value(""),
        detail: "Width of the highlight rectangle drawn around the widget.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-blockcursor",
        value: OptionValue::boolean(),
        detail: "Specifies a boolean that says whether the blinking insertion cursor should be drawn as a character-sized.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-endline",
        value: OptionValue::value("line"),
        detail: "Specifies an integer line index representing the line of the underlying textual data store that should be.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-inactiveselectbackground",
        value: OptionValue::value("color"),
        detail: "Specifies the colour to use for the selection (the sel tag) when the window does not have the input focus. If.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-insertunfocussed",
        value: OptionValue::boolean(),
        detail: ".",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-startline",
        value: OptionValue::value("line"),
        detail: "Specifies an integer line index representing the first line of the underlying textual data store that should.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-tabstyle",
        value: OptionValue::value("style"),
        detail: "Specifies how to interpret the relationship between tab stops on a line and tabs in the text of that line.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "text pathName ?option value ...?",
    dialects: None,
}];

/// `text`'s instance command dispatches through the same subcommand table
/// as its own constructor spec (see
/// `docs/design/tk-widget-instance-typing.md`).
static TEXT_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "text",
    instance_methods: &SUBCOMMANDS,
    superclasses: &[],
    allow_unknown_methods: false,
};

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
        subcommands: &SUBCOMMANDS,
        object_class: Some(&TEXT_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
