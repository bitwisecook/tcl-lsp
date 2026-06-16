//! `event` command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::at_least(2),
        detail: "Associate a virtual event with one or more physical event sequences.",
        synopsis: "event add <<virtual>> sequence ?sequence ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(1),
        detail: "Delete physical event sequences from a virtual event.",
        synopsis: "event delete <<virtual>> ?sequence sequence ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "generate",
        arity: Arity::at_least(2),
        detail: "Generate an event and arrange for it to be processed.",
        synopsis: "event generate window event ?option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(0, 1),
        detail: "Return information about virtual events.",
        synopsis: "event info ?<<virtual>>?",
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
        name: "-above",
        takes_value: true,
        value_hint: "window",
        detail: "Specifies the above field for the event (generate).",
        dialects: None,
    },
    OptionSpec {
        name: "-borderwidth",
        takes_value: true,
        value_hint: "size",
        detail: "Specifies the border width for the event (generate).",
        dialects: None,
    },
    OptionSpec {
        name: "-button",
        takes_value: true,
        value_hint: "number",
        detail: "Specifies the button number for a ButtonPress or ButtonRelease event.",
        dialects: None,
    },
    OptionSpec {
        name: "-count",
        takes_value: true,
        value_hint: "number",
        detail: "Specifies the count field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-data",
        takes_value: true,
        value_hint: "string",
        detail: "Specifies user data for a virtual event.",
        dialects: None,
    },
    OptionSpec {
        name: "-delta",
        takes_value: true,
        value_hint: "number",
        detail: "Specifies the delta field for a MouseWheel event.",
        dialects: None,
    },
    OptionSpec {
        name: "-detail",
        takes_value: true,
        value_hint: "detail",
        detail: "Specifies the detail field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-focus",
        takes_value: true,
        value_hint: "boolean",
        detail: "Specifies the focus field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-height",
        takes_value: true,
        value_hint: "size",
        detail: "Specifies the height field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-keycode",
        takes_value: true,
        value_hint: "number",
        detail: "Specifies the keycode field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-keysym",
        takes_value: true,
        value_hint: "name",
        detail: "Specifies the keysym field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-mode",
        takes_value: true,
        value_hint: "notify",
        detail: "Specifies the mode field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-override",
        takes_value: true,
        value_hint: "boolean",
        detail: "Specifies the override-redirect field.",
        dialects: None,
    },
    OptionSpec {
        name: "-place",
        takes_value: true,
        value_hint: "where",
        detail: "Specifies the place field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-root",
        takes_value: true,
        value_hint: "window",
        detail: "Specifies the root field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-rootx",
        takes_value: true,
        value_hint: "coord",
        detail: "Specifies the x-coordinate relative to the root window.",
        dialects: None,
    },
    OptionSpec {
        name: "-rooty",
        takes_value: true,
        value_hint: "coord",
        detail: "Specifies the y-coordinate relative to the root window.",
        dialects: None,
    },
    OptionSpec {
        name: "-sendevent",
        takes_value: true,
        value_hint: "boolean",
        detail: "Specifies the send-event field.",
        dialects: None,
    },
    OptionSpec {
        name: "-serial",
        takes_value: true,
        value_hint: "number",
        detail: "Specifies the serial number field.",
        dialects: None,
    },
    OptionSpec {
        name: "-state",
        takes_value: true,
        value_hint: "state",
        detail: "Specifies the state field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-subwindow",
        takes_value: true,
        value_hint: "window",
        detail: "Specifies the sub-window field.",
        dialects: None,
    },
    OptionSpec {
        name: "-time",
        takes_value: true,
        value_hint: "integer",
        detail: "Specifies the time field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-warp",
        takes_value: true,
        value_hint: "boolean",
        detail: "Specifies whether the screen pointer should be warped.",
        dialects: None,
    },
    OptionSpec {
        name: "-width",
        takes_value: true,
        value_hint: "size",
        detail: "Specifies the width field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-when",
        takes_value: true,
        value_hint: "now|tail|head|mark",
        detail: "Specifies when the event is processed.",
        dialects: None,
    },
    OptionSpec {
        name: "-x",
        takes_value: true,
        value_hint: "coord",
        detail: "Specifies the x field for the event.",
        dialects: None,
    },
    OptionSpec {
        name: "-y",
        takes_value: true,
        value_hint: "coord",
        detail: "Specifies the y field for the event.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "event option ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "event",
        dialects: None,
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Generate, manage, and inspect virtual events.",
            synopsis: &[
                "event add <<virtual>> sequence ?sequence ...?",
                "event delete <<virtual>> ?sequence sequence ...?",
                "event generate window event ?option value ...?",
                "event info ?<<virtual>>?",
            ],
            snippet: "",
            source: "Tk man page event.n",
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
