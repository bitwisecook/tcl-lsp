//! `interp` — create and manipulate Tcl interpreters.
use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "alias",
        arity: Arity::at_least(2),
        detail: "Manage command aliases.",
        synopsis: "interp alias path cmd",
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
        arity: Arity::new(1, 2),
        detail: "Get or set background error handler.",
        synopsis: "interp bgerror path ?cmdPrefix?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cancel",
        arity: Arity::at_least(0),
        detail: "Cancel a script evaluation.",
        synopsis: "interp cancel ?-unwind? ?--? ?result?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::new(0, 2),
        detail: "Create a child interpreter.",
        synopsis: "interp create ?-safe? ?--? ?name?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "debug",
        arity: Arity::at_least(1),
        detail: "Control debug mode.",
        synopsis: "interp debug path ?-frame ?bool??",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::at_least(0),
        detail: "Delete interpreters.",
        synopsis: "interp delete ?path ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eval",
        arity: Arity::at_least(2),
        detail: "Evaluate script in another interpreter.",
        synopsis: "interp eval path arg ?arg ...?",
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "invokehidden",
        arity: Arity::at_least(2),
        detail: "Invoke a hidden command.",
        synopsis: "interp invokehidden path ?-option ...? hiddenCmdName ?arg ...?",
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "marktrusted",
        arity: Arity::exact(1),
        detail: "Mark interpreter as trusted.",
        synopsis: "interp marktrusted path",
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
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "interp",
        traits: Traits::HAS_INTERP_EVAL | Traits::HAS_DESTRUCTIVE_OPS,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet::brief(
            "Create and manipulate Tcl interpreters.",
            &["interp subcommand ?arg ...?"],
            "Tcl interp(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
