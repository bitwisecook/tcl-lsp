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

//! `canvas` command.
use crate::prelude::*;

/// Dynamic arg-role resolver for the canvas `bind` subcommand.
///
/// `pathName bind tagOrId ?sequence? ?command?` — like the top-level
/// `bind` command, only the full three-argument form binds a script,
/// supplied as the trailing (third) argument.  It is a deferred
/// event-handler body run later from the Tk event loop, so `body_kind`
/// is `Structural`.  Args here are those *after* the `bind` subcommand
/// word: `tagOrId`(0) `sequence`(1) `command`(2).
fn canvas_bind_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.len() == 3 {
        vec![(2, ArgRole::Body)]
    } else {
        Vec::new()
    }
}

/// The command's subcommands.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "addtag",
        arity: Arity::at_least(2),
        detail: "Add a tag to items matching a search specification.",
        synopsis: "pathName addtag tag searchCommand ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "bbox",
        arity: Arity::at_least(1),
        detail: "Return the bounding box of the items given by the tagOrIds.",
        synopsis: "pathName bbox tagOrId ?tagOrId ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "bind",
        dialects: None,
        arity: Arity::new(1, 3),
        detail: "Associate a command with a canvas item event.",
        synopsis: "pathName bind tagOrId ?sequence? ?command?",
        arg_role_resolver: Some(canvas_bind_arg_roles),
        body_kind: BodyKind::Structural,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "canvasx",
        arity: Arity::new(1, 2),
        detail: "Convert a window x-coordinate to a canvas x-coordinate.",
        synopsis: "pathName canvasx screenx ?gridspacing?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "canvasy",
        arity: Arity::new(1, 2),
        detail: "Convert a window y-coordinate to a canvas y-coordinate.",
        synopsis: "pathName canvasy screeny ?gridspacing?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "coords",
        arity: Arity::at_least(1),
        detail: "Query or set the coordinates of an item.",
        synopsis: "pathName coords tagOrId ?x y ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::at_least(3),
        detail: "Create a new canvas item of the specified type.",
        synopsis: "pathName create type x y ?x y ...? ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dchars",
        arity: Arity::at_least(2),
        detail: "Delete characters between the first and last positions in a text or line item.",
        synopsis: "pathName dchars tagOrId first ?last?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(0),
        detail: "Delete the items given by each tagOrId.",
        synopsis: "pathName delete ?tagOrId ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dtag",
        arity: Arity::new(1, 2),
        detail: "Remove a tag from the items given by tagOrId.",
        synopsis: "pathName dtag tagOrId ?tagToDelete?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "find",
        arity: Arity::at_least(1),
        detail: "Return item IDs matching a search specification.",
        synopsis: "pathName find searchCommand ?arg ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "focus",
        dialects: None,
        arity: Arity::new(0, 1),
        detail: "Set or query the focus item for the canvas.",
        synopsis: "pathName focus ?tagOrId?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "gettags",
        arity: Arity::exact(1),
        detail: "Return the tags associated with the item.",
        synopsis: "pathName gettags tagOrId",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "icursor",
        arity: Arity::exact(2),
        detail: "Set the insertion cursor of a text, line, or polygon item just before the given index.",
        synopsis: "pathName icursor tagOrId index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "index",
        arity: Arity::exact(2),
        detail: "Return the numerical index within an item corresponding to the given index.",
        synopsis: "pathName index tagOrId index",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "insert",
        arity: Arity::exact(3),
        detail: "Insert a string into a text, line, or polygon item before the given index.",
        synopsis: "pathName insert tagOrId beforeThis string",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "itemcget",
        arity: Arity::exact(2),
        detail: "Return the value of a configuration option for a canvas item.",
        synopsis: "pathName itemcget tagOrId option",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "itemconfigure",
        arity: Arity::at_least(1),
        detail: "Query or modify configuration options of a canvas item.",
        synopsis: "pathName itemconfigure tagOrId ?option? ?value option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lower",
        dialects: None,
        arity: Arity::new(1, 2),
        detail: "Lower the items given by tagOrId in the display list.",
        synopsis: "pathName lower tagOrId ?belowThis?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "move",
        arity: Arity::exact(3),
        detail: "Move each item by the given distance.",
        synopsis: "pathName move tagOrId xAmount yAmount",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "moveto",
        arity: Arity::exact(3),
        detail: "Move the items so their top-left corner is at the given position.",
        synopsis: "pathName moveto tagOrId x y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "postscript",
        arity: Arity::at_least(0),
        detail: "Generate a Postscript representation of the canvas.",
        synopsis: "pathName postscript ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "raise",
        dialects: None,
        arity: Arity::new(1, 2),
        detail: "Raise the items given by tagOrId in the display list.",
        synopsis: "pathName raise tagOrId ?aboveThis?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rchars",
        arity: Arity::exact(4),
        detail: "Replace the characters between first and last in a text or line item with the given string.",
        synopsis: "pathName rchars tagOrId first last string",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scale",
        dialects: None,
        arity: Arity::exact(5),
        detail: "Rescale the coordinates of items.",
        synopsis: "pathName scale tagOrId xOrigin yOrigin xScale yScale",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scan",
        arity: Arity::at_least(1),
        detail: "Implement scanning for the canvas.",
        synopsis: "pathName scan option args",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "select",
        arity: Arity::at_least(1),
        detail: "Manipulate the selection in text, line, or polygon items.",
        synopsis: "pathName select option ?tagOrId arg?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(1),
        detail: "Return the type of the item given by tagOrId.",
        synopsis: "pathName type tagOrId",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "xview",
        arity: Arity::at_least(0),
        detail: "Query or change the horizontal view position.",
        synopsis: "pathName xview ?args?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "yview",
        arity: Arity::at_least(0),
        detail: "Query or change the vertical view position.",
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
        detail: "Desired width of the canvas in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-height",
        value: OptionValue::value(""),
        detail: "Desired height of the canvas in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-bg",
        value: OptionValue::value(""),
        detail: "Shorthand for -background.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-background",
        value: OptionValue::value(""),
        detail: "Background colour of the canvas.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-relief",
        value: OptionValue::enumerated(super::common::RELIEF, true, "relief"),
        detail: "3-D effect: flat, groove, raised, ridge, solid, or sunken.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-borderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around the canvas.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-bd",
        value: OptionValue::value(""),
        detail: "Shorthand for -borderwidth.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-scrollregion",
        value: OptionValue::value(""),
        detail: "Bounding box of the total scrollable area (left top right bottom).",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-xscrollcommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for communicating with horizontal scrollbars.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-yscrollcommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Command prefix for communicating with vertical scrollbars.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-xscrollincrement",
        value: OptionValue::value(""),
        detail: "Horizontal scrolling increment in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-yscrollincrement",
        value: OptionValue::value(""),
        detail: "Vertical scrolling increment in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-confine",
        value: OptionValue::value(""),
        detail: "Whether scrolling is confined to the scroll region.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-selectbackground",
        value: OptionValue::value(""),
        detail: "Background colour for selected items.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-selectborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around selected items.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-selectforeground",
        value: OptionValue::value(""),
        detail: "Foreground colour for selected items.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-insertbackground",
        value: OptionValue::value(""),
        detail: "Colour of the insertion cursor.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-insertborderwidth",
        value: OptionValue::value(""),
        detail: "Width of the border around the insertion cursor.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-insertofftime",
        value: OptionValue::value(""),
        detail: "Milliseconds the insertion cursor is off during blinking.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-insertontime",
        value: OptionValue::value(""),
        detail: "Milliseconds the insertion cursor is on during blinking.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-insertwidth",
        value: OptionValue::value(""),
        detail: "Width of the insertion cursor in screen units.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-closeenough",
        value: OptionValue::value(""),
        detail: "Proximity threshold for mouse cursor to be considered over an item.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-cursor",
        value: OptionValue::value(""),
        detail: "Cursor to display when the mouse is over the canvas.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-takefocus",
        value: OptionValue::value(""),
        detail: "Whether the canvas accepts focus during keyboard traversal.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-highlightbackground",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the canvas does not have focus.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-highlightcolor",
        value: OptionValue::value(""),
        detail: "Colour of the highlight region when the canvas has focus.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-highlightthickness",
        value: OptionValue::value(""),
        detail: "Width of the highlight rectangle drawn around the canvas.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
    OptionSpec {
        name: "-state",
        value: OptionValue::value("stateSpec"),
        detail: "Modifies the default state of the canvas where state may be set to one of: normal, disabled, or hidden.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "canvas pathName ?option value ...?",
    dialects: None,
}];

/// `canvas`'s instance command dispatches through the same subcommand
/// table as its own constructor spec (see
/// `docs/design/tk-widget-instance-typing.md`).
static CANVAS_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "canvas",
    instance_methods: SUBCOMMANDS,
    superclasses: &[],
    allow_unknown_methods: false,
};

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "canvas",
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a canvas widget.",
            synopsis: &["canvas pathName ?option value ...?"],
            snippet: "Displays a rectangular area for drawing graphics, positioning widgets, and handling events on items.",
            source: "Tk man page canvas.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        object_class: Some(&CANVAS_CLASS),
        creates_instance_at: Some(0),
        ..CommandSpec::DEFAULT
    }
}
