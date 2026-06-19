//! `listbox` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-listvariable",
        takes_value: true,
        value_hint: "",
        detail: "Name of a variable containing the list of values to display.",
        dialects: None,
    },
    OptionSpec {
        name: "-selectmode",
        takes_value: true,
        value_hint: "",
        detail: "Selection mode: single, browse, multiple, or extended.",
        dialects: None,
    },
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "",
        detail: "Desired width of the listbox in characters.",
        dialects: None,
    },
    OptionSpec {
        name: "-height",
        takes_value: true,
        value_hint: "",
        detail: "Desired height of the listbox in lines.",
        dialects: None,
    },
    OptionSpec {
        name: "-font",
        takes_value: true,
        value_hint: "",
        detail: "Font to use for text in the listbox.",
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
        name: "-fg",
        takes_value: true,
        value_hint: "",
        detail: "Shorthand for -foreground.",
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
        name: "-exportselection",
        takes_value: true,
        value_hint: "",
        detail: "Whether the selection is exported to the X selection.",
        dialects: None,
    },
    OptionSpec {
        name: "-setgrid",
        takes_value: true,
        value_hint: "",
        detail: "Whether this widget controls the resizing grid for its toplevel.",
        dialects: None,
    },
    OptionSpec {
        name: "-activestyle",
        takes_value: true,
        value_hint: "",
        detail: "Style for the active element: dotbox, none, or underline.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over the listbox.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "",
        detail: "Whether the listbox accepts focus during keyboard traversal.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the listbox does not have focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the listbox has focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        takes_value: true,
        value_hint: "",
        detail: "Width of the highlight rectangle drawn around the listbox.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "listbox pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "listbox",
        dialects: None,
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
        ..CommandSpec::DEFAULT
    }
}
