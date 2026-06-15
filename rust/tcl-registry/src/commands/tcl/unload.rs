//! `unload` — unload a shared library extension.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-nocomplain",
        takes_value: false,
        value_hint: "",
        detail: "Suppresses all error messages.",
        dialects: None,
    },
    OptionSpec {
        name: "-keeplibrary",
        takes_value: false,
        value_hint: "",
        detail: "This switch will prevent unload from issuing the operating system call that will unload the library from the process.",
        dialects: None,
    },
    OptionSpec {
        name: "--",
        takes_value: false,
        value_hint: "",
        detail: "Marks the end of switches.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "unload ?switches? fileName",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unload",
        arity: Arity::new(1, 3),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Unload machine code",
            synopsis: &[
                "unload ?switches? fileName",
                "unload ?switches? fileName prefix",
                "unload ?switches? fileName prefix interp",
                "unload ?-keeplibrary? ?-nocomplain? fileName ?prefix? ?interp?",
            ],
            snippet: "This command tries to unload shared libraries previously loaded with load from the application's address space.",
            source: "Tcl man page unload.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
