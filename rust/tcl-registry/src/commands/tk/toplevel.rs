//! `toplevel` command.
use crate::prelude::*;
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
        detail: "Desired width of the toplevel in screen units.",
        dialects: None,
    },
    OptionSpec {
        name: "-height",
        takes_value: true,
        value_hint: "",
        detail: "Desired height of the toplevel in screen units.",
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
        detail: "Background colour of the toplevel window.",
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
        detail: "Width of the border around the toplevel.",
        dialects: None,
    },
    OptionSpec {
        name: "-menu",
        takes_value: true,
        value_hint: "",
        detail: "Path name of a menu widget to use as the toplevel's menu bar.",
        dialects: None,
    },
    OptionSpec {
        name: "-screen",
        takes_value: true,
        value_hint: "",
        detail: "Screen on which to place the toplevel window.",
        dialects: None,
    },
    OptionSpec {
        name: "-use",
        takes_value: true,
        value_hint: "",
        detail: "Window identifier of a container in which to embed the toplevel.",
        dialects: None,
    },
    OptionSpec {
        name: "-class",
        takes_value: true,
        value_hint: "",
        detail: "Class name for the toplevel, used in option database lookups.",
        dialects: None,
    },
    OptionSpec {
        name: "-colormap",
        takes_value: true,
        value_hint: "",
        detail: "Colourmap to use for the toplevel: new or inherited from a window.",
        dialects: None,
    },
    OptionSpec {
        name: "-container",
        takes_value: true,
        value_hint: "",
        detail: "Whether the toplevel will be a container for an embedded application.",
        dialects: None,
    },
    OptionSpec {
        name: "-visual",
        takes_value: true,
        value_hint: "",
        detail: "Visual information for the toplevel window.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over the toplevel.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "",
        detail: "Whether the toplevel accepts focus during keyboard traversal.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the toplevel does not have focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the toplevel has focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        takes_value: true,
        value_hint: "",
        detail: "Width of the highlight rectangle drawn around the toplevel.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "toplevel pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "toplevel",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Create and manipulate a toplevel widget.",
            synopsis: &["toplevel pathName ?option value ...?"],
            snippet: "Creates a new toplevel window that acts as a separate window manager frame, suitable for dialogue boxes and secondary windows.",
            source: "Tk man page toplevel.n",
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
