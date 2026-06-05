//! `scrollbar` command.
use crate::prelude::*;
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
        detail: "Orientation of the scrollbar: horizontal or vertical.",
        dialects: None,
    },
    OptionSpec {
        name: "-command",
        takes_value: true,
        value_hint: "",
        detail: "Command prefix to invoke when the scrollbar is moved.",
        dialects: None,
    },
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "",
        detail: "Desired narrow dimension of the scrollbar in screen units.",
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
        detail: "Background colour of the scrollbar.",
        dialects: None,
    },
    OptionSpec {
        name: "-activebackground",
        takes_value: true,
        value_hint: "",
        detail: "Background colour when the mouse is over the scrollbar elements.",
        dialects: None,
    },
    OptionSpec {
        name: "-troughcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the trough area behind the slider.",
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
        detail: "Width of the border around the scrollbar.",
        dialects: None,
    },
    OptionSpec {
        name: "-elementborderwidth",
        takes_value: true,
        value_hint: "",
        detail: "Width of the borders around the internal elements of the scrollbar.",
        dialects: None,
    },
    OptionSpec {
        name: "-jump",
        takes_value: true,
        value_hint: "",
        detail: "Whether to delay updates until the mouse button is released.",
        dialects: None,
    },
    OptionSpec {
        name: "-activerelief",
        takes_value: true,
        value_hint: "",
        detail: "Relief to use for the active element of the scrollbar.",
        dialects: None,
    },
    OptionSpec {
        name: "-cursor",
        takes_value: true,
        value_hint: "",
        detail: "Cursor to display when the mouse is over the scrollbar.",
        dialects: None,
    },
    OptionSpec {
        name: "-takefocus",
        takes_value: true,
        value_hint: "",
        detail: "Whether the scrollbar accepts focus during keyboard traversal.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightbackground",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the scrollbar does not have focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightcolor",
        takes_value: true,
        value_hint: "",
        detail: "Colour of the highlight region when the scrollbar has focus.",
        dialects: None,
    },
    OptionSpec {
        name: "-highlightthickness",
        takes_value: true,
        value_hint: "",
        detail: "Width of the highlight rectangle drawn around the scrollbar.",
        dialects: None,
    },
    OptionSpec {
        name: "-repeatdelay",
        takes_value: true,
        value_hint: "",
        detail: "Milliseconds before auto-repeat begins when an arrow is held.",
        dialects: None,
    },
    OptionSpec {
        name: "-repeatinterval",
        takes_value: true,
        value_hint: "",
        detail: "Milliseconds between auto-repeat invocations.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "scrollbar pathName ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "scrollbar",
        dialects: Some(DialectSet::TK),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate a scrollbar widget.",
            &["scrollbar pathName ?option value ...?"],
            "F5",
        )),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
