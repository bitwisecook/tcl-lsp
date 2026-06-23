//! `interp` — create and manipulate Tcl interpreters.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "interp subcommand ?arg arg ...?",
}];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "alias",
        arity: Arity::at_least(2),
        detail: "Manage command aliases.",
        synopsis: "interp alias path cmd",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "aliases",
        arity: Arity::new(0, 1),
        detail: "List aliases.",
        synopsis: "interp aliases ?path?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "bgerror",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        arity: Arity::new(1, 2),
        detail: "Get or set background error handler.",
        synopsis: "interp bgerror path ?cmdPrefix?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cancel",
        arity: Arity::at_least(0),
        detail: "Cancel a script evaluation.",
        synopsis: "interp cancel ?-unwind? ?--? ?result?",
        return_type: Some(TclType::String),
        options: &[
            OptionSpec {
                name: "-unwind",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::new(0, 2),
        detail: "Create a child interpreter.",
        synopsis: "interp create ?-safe? ?--? ?name?",
        return_type: Some(TclType::String),
        options: &[
            OptionSpec {
                name: "-safe",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "debug",
        arity: Arity::at_least(1),
        detail: "Control debug mode.",
        synopsis: "interp debug path ?-frame ?bool??",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(0),
        detail: "Delete interpreters.",
        synopsis: "interp delete ?path ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eval",
        arity: Arity::at_least(2),
        detail: "Evaluate script in another interpreter.",
        synopsis: "interp eval path arg ?arg ...?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::Body)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Check if interpreter exists.",
        synopsis: "interp exists path",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "expose",
        arity: Arity::new(2, 3),
        detail: "Expose a hidden command.",
        synopsis: "interp expose path hiddenCmdName ?exposedCmdName?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hidden",
        arity: Arity::exact(1),
        detail: "List hidden commands.",
        synopsis: "interp hidden path",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hide",
        arity: Arity::new(2, 3),
        detail: "Hide a command.",
        synopsis: "interp hide path exposedCmdName ?hiddenCmdName?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "invokehidden",
        arity: Arity::at_least(2),
        detail: "Invoke a hidden command.",
        synopsis: "interp invokehidden path ?-option ...? hiddenCmdName ?arg ...?",
        return_type: Some(TclType::String),
        options: &[
            OptionSpec {
                name: "-global",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-namespace",
                takes_value: true,
                value_hint: "ns",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "issafe",
        arity: Arity::exact(1),
        detail: "Check if interpreter is safe.",
        synopsis: "interp issafe path",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "limit",
        arity: Arity::at_least(2),
        detail: "Get or set resource limits.",
        synopsis: "interp limit path limitType ?-option value ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "marktrusted",
        arity: Arity::exact(1),
        detail: "Mark interpreter as trusted.",
        synopsis: "interp marktrusted path",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "recursionlimit",
        arity: Arity::new(1, 2),
        detail: "Get or set recursion limit.",
        synopsis: "interp recursionlimit path ?newlimit?",
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "share",
        arity: Arity::exact(3),
        detail: "Share a channel.",
        synopsis: "interp share srcPath channelId destPath",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "target",
        arity: Arity::exact(2),
        detail: "Get alias target.",
        synopsis: "interp target path alias",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "transfer",
        arity: Arity::exact(3),
        detail: "Transfer a channel.",
        synopsis: "interp transfer srcPath channelId destPath",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "slaves",
        arity: Arity::new(0, 1),
        detail: "Returns a Tcl list of the names of all the child interpreters.",
        synopsis: "interp slaves ?path?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "children",
        arity: Arity::new(0, 1),
        detail: "Returns a Tcl list of the names of all the child interpreters associated with the interpreter identified by path.",
        synopsis: "interp children ?path?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "interp",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::HAS_INTERP_EVAL
            | Traits::HAS_DESTRUCTIVE_OPS
            | Traits::LANGUAGE_KEYWORD
            | Traits::DYNAMIC_EVAL_BODY,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Create and manipulate Tcl interpreters",
            synopsis: &[
                "interp subcommand ?arg arg ...?",
                "interp subcommand ?arg ...?",
            ],
            snippet: "This command makes it possible to create one or more new Tcl interpreters that co-exist with the creating interpreter in the same application.",
            source: "Tcl man page interp.n",
            examples: "",
            return_value: "",
        }),
        // `interp eval` / `interp invokehidden` run code in
        // another interpreter — cross-interp code injection (T105).
        taint_interp_eval_subcommands: &["eval", "invokehidden"],
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
