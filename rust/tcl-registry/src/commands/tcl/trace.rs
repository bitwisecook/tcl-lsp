//! `trace` — monitor variable accesses, command usages and executions.
use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::exact(4),
        detail: "Arrange for a command to be executed on the specified operation.",
        synopsis: "trace add type name ops commandPrefix",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::exact(2),
        detail: "Return trace info for the given name.",
        synopsis: "trace info type name",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        arity: Arity::exact(4),
        detail: "Remove a trace.",
        synopsis: "trace remove type name ops commandPrefix",
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "trace",
        traits: Traits::CREATES_BARRIER,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet::brief(
            "Monitor variable accesses, command usages and executions.",
            &["trace option ?arg arg ...?"],
            "Tcl trace(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
