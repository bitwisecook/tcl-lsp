//! `canvas` command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
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
        arity: Arity::at_least(1),
        detail: "Associate a command with a canvas item event.",
        synopsis: "pathName bind tagOrId ?sequence? ?command?",
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
        name: "itemconfigure",
        arity: Arity::at_least(1),
        detail: "Query or modify configuration options of a canvas item.",
        synopsis: "pathName itemconfigure tagOrId ?option? ?value option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lower",
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
        name: "postscript",
        arity: Arity::at_least(0),
        detail: "Generate a Postscript representation of the canvas.",
        synopsis: "pathName postscript ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "raise",
        arity: Arity::new(1, 2),
        detail: "Raise the items given by tagOrId in the display list.",
        synopsis: "pathName raise tagOrId ?aboveThis?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scale",
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
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "",
        detail: "Desired width of the canvas in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-height",
        takes_value: true,
        value_hint: "",
        detail: "Desired height of the canvas in screen units.",
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
        name: "-background",
        takes_value: true,
        value_hint: "",
        detail: "Background colour of the canvas.",
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
        detail: "Width of the border around the canvas.",
        dialects: None,
    },
    OptionSpec {
        name: "-bd",
        takes_value: true,
        value_hint: "",
        detail: "Shorthand for -borderwidth.",
        dialects: None,
    },
    OptionSpec {
        name: "-scrollregion",
        takes_value: true,
        value_hint: "",
        detail: "Bounding box of the total scrollable area (left top right bottom).",
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
        name: "-yscrollcommand",
        takes_value: true,
        value_hint: "",
        detail: "Command prefix for communicating with vertical scrollbars.",
        dialects: None,
    },
    OptionSpec {
        name: "-xscrollincrement",
        takes_value: true,
        value_hint: "",
        detail: "Horizontal scrolling increment in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-yscrollincrement",
        takes_value: true,
        value_hint: "",
        detail: "Vertical scrolling increment in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-confine",
        takes_value: true,
        value_hint: "",
        detail: "Whether scrolling is confined to the scroll region.",
        dialects: None,
    },
    OptionSpec {
        name: "-selectbackground",
        takes_value: true,
        value_hint: "",
        detail: "Background colour for selected items.",
        dialects: None,
    },
    OptionSpec {
        name: "-selectborderwidth",
        takes_value: true,
        value_hint: "",
        detail: "Width of the border around selected items.",
        dialects: None,
    },
    OptionSpec {
        name: "-selectforeground",
        takes_value: true,
        value_hint: "",
        detail: "Foreground colour for selected items.",
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
        name: "-closeenough",
        takes_value: true,
        value_hint: "",
        detail: "Proximity threshold for mouse cursor to be considered over an item.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over the canvas.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "",
        detail: "Whether the canvas accepts focus during keyboard traversal.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the canvas does not have focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the canvas has focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        takes_value: true,
        value_hint: "",
        detail: "Width of the highlight rectangle drawn around the canvas.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "canvas pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "canvas",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a canvas widget.",
            &["canvas pathName ?option value ...?"],
            "F5",
        )),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
