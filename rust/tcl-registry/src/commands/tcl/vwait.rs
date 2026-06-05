//! `vwait` — wait for a variable to be modified.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "--",
        takes_value: false,
        value_hint: "",
        detail: "Marks the end of options.",
        dialects: None,
    },
    OptionSpec {
        name: "-all",
        takes_value: false,
        value_hint: "",
        detail: "All conditions for the wait operation must be met to complete the wait operation.",
        dialects: None,
    },
    OptionSpec {
        name: "-extended",
        takes_value: false,
        value_hint: "",
        detail: "An extended result in list form is returned, see below for explanation.",
        dialects: None,
    },
    OptionSpec {
        name: "-nofileevents",
        takes_value: false,
        value_hint: "",
        detail: "File events are not handled in the wait operation.",
        dialects: None,
    },
    OptionSpec {
        name: "-noidleevents",
        takes_value: false,
        value_hint: "",
        detail: "Idle handlers are not invoked during the wait operation.",
        dialects: None,
    },
    OptionSpec {
        name: "-notimerevents",
        takes_value: false,
        value_hint: "",
        detail: "Timer handlers are not serviced during the wait operation.",
        dialects: None,
    },
    OptionSpec {
        name: "-nowindowevents",
        takes_value: false,
        value_hint: "",
        detail: "Events of the windowing system are not handled during the wait operation.",
        dialects: None,
    },
    OptionSpec {
        name: "-readable",
        takes_value: true,
        value_hint: "",
        detail: "Channel must name a Tcl channel open for reading.",
        dialects: None,
    },
    OptionSpec {
        name: "-timeout",
        takes_value: true,
        value_hint: "",
        detail: "The wait operation is constrained to milliseconds.",
        dialects: None,
    },
    OptionSpec {
        name: "-variable",
        takes_value: true,
        value_hint: "",
        detail: "VarName must be the name of a global variable.",
        dialects: None,
    },
    OptionSpec {
        name: "-writable",
        takes_value: true,
        value_hint: "",
        detail: "Channel must name a Tcl channel open for writing.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "vwait varName",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vwait",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::VarRead)],
        return_type: Some(TclType::String),
hover: Some(HoverSnippet {
    summary: "Process events until a variable is written",
    synopsis: &["vwait varName", "vwait ?options? ?varName ...?"],
    snippet: "This command enters the Tcl event loop to process events, blocking the application if no events are ready.",
    source: "Tcl man page vwait.n",
    examples: "",
    return_value: "",
}),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
