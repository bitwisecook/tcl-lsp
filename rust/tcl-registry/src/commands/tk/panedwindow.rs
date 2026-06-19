//! `panedwindow` command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(1),
        detail: "Add one or more child windows to the panedwindow.",
        synopsis: "pathName add window ?window ...? ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::exact(1),
        detail: "Remove the specified pane from the panedwindow.",
        synopsis: "pathName forget window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "identify",
        arity: Arity::exact(2),
        detail: "Identify the panedwindow component at the given coordinates.",
        synopsis: "pathName identify x y",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "panecget",
        arity: Arity::exact(2),
        detail: "Return the value of a configuration option for a pane.",
        synopsis: "pathName panecget window option",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "paneconfigure",
        arity: Arity::at_least(1),
        detail: "Query or modify configuration options of a pane.",
        synopsis: "pathName paneconfigure window ?option? ?value option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "panes",
        arity: Arity::exact(0),
        detail: "Return a list of all panes in the panedwindow.",
        synopsis: "pathName panes",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "proxy",
        arity: Arity::at_least(0),
        detail: "Query or manipulate the sash-drag proxy.",
        synopsis: "pathName proxy ?args?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sash",
        arity: Arity::at_least(1),
        detail: "Query or adjust sash positions.",
        synopsis: "pathName sash option ?arg ...?",
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
        name: "-orient",
        takes_value: true,
        value_hint: "",
        detail: "Orientation of the panedwindow: horizontal or vertical.",
        dialects: None,
    },
    OptionSpec {
        name: "-sashwidth",
        takes_value: true,
        value_hint: "",
        detail: "Width of each sash in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-sashrelief",
        takes_value: true,
        value_hint: "",
        detail: "Relief of the sashes: flat, groove, raised, ridge, solid, or sunken.",
        dialects: None,
    },
    OptionSpec {
        name: "-sashpad",
        takes_value: true,
        value_hint: "",
        detail: "Extra padding on each side of a sash.",
        dialects: None,
    },
    OptionSpec {
        name: "-sashcursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over a sash.",
        dialects: None,
    },
    OptionSpec {
        name: "-showhandle",
        takes_value: true,
        value_hint: "",
        detail: "Whether to display handles on the sashes.",
        dialects: None,
    },
    OptionSpec {
        name: "-handlesize",
        takes_value: true,
        value_hint: "",
        detail: "Size of the sash handle in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-handlepad",
        takes_value: true,
        value_hint: "",
        detail: "Distance from the top or left end of a sash to the handle.",
        dialects: None,
    },
    OptionSpec {
        name: "-opaqueresize",
        takes_value: true,
        value_hint: "",
        detail: "Whether panes are resized continuously as the sash is dragged.",
        dialects: None,
    },
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "",
        detail: "Desired width of the panedwindow in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-height",
        takes_value: true,
        value_hint: "",
        detail: "Desired height of the panedwindow in screen units.",
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
        detail: "Background colour of the panedwindow.",
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
        detail: "Width of the border around the panedwindow.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over the panedwindow.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "",
        detail: "Whether the panedwindow accepts focus during keyboard traversal.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the panedwindow does not have focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the panedwindow has focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        takes_value: true,
        value_hint: "",
        detail: "Width of the highlight rectangle drawn around the panedwindow.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "panedwindow pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "panedwindow",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a panedwindow widget.",
            synopsis: &["panedwindow pathName ?option value ...?"],
            snippet: "Displays a container that divides its space among child widgets separated by movable sashes.",
            source: "Tk man page panedwindow.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        subcommands: SUBCOMMANDS,
        ..CommandSpec::DEFAULT
    }
}
