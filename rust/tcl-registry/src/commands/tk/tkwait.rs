//! `tkwait` command.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: false,
    connection_side: ConnectionSide::None,
}];

const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "variable",
        arity: Arity::exact(1),
        detail: "Wait until the global variable name is set.",
        synopsis: "tkwait variable name",
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "visibility",
        arity: Arity::exact(1),
        detail: "Wait until the visibility state of window changes.",
        synopsis: "tkwait visibility window",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "window",
        arity: Arity::exact(1),
        detail: "Wait until window is destroyed.",
        synopsis: "tkwait window window",
        ..SubCommand::DEFAULT
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "tkwait variable|visibility|window name",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tkwait",
        // `tkwait variable` reads a global variable by name through the
        // frame hash bucket — hence the FRAME_HASH_BUILTIN trait the
        // var-escape slot resolver keys on.
        traits: Traits::FRAME_HASH_BUILTIN,
        dialects: Some(DialectSet::TK_AND_TCL),
        arity: Arity::exact(2),
        subcommands: SUBCOMMANDS,
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Wait for an event (variable set, window visibility, or destruction).",
            synopsis: &[
                "tkwait variable name",
                "tkwait visibility window",
                "tkwait window window",
            ],
            snippet: "",
            source: "Tk man page tkwait.n",
            examples: "",
            return_value: "",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
