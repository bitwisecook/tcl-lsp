//! `trace` — monitor variable accesses, command usages and executions.
use crate::prelude::*;

/// SYNC4: arg-role resolver for `trace add`.
///
/// `trace add variable name ops commandPrefix` writes to `name` —
/// the trace handler can rewrite the variable at runtime, so SSA
/// must see `name` as a definition site.  Mirrors Python's
/// registry stamp post-`01326b40` ("ssa: route `trace add variable`
/// defs through registry for #249").
///
/// The resolver only fires for the `variable` form so
/// `trace add execution` and `trace add command` (which take a
/// command name, not a variable) don't appear as SSA defs.
fn trace_add_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.first() == Some(&"variable") && args.len() >= 2 {
        return vec![(1, ArgRole::VarWrite)];
    }
    Vec::new()
}

/// Same arg-role pattern for `trace remove variable` — keeps
/// registry parity with `trace add variable` so consumers can
/// query both spellings via the same `ArgRole::VarWrite` lookup.
fn trace_remove_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    if args.first() == Some(&"variable") && args.len() >= 2 {
        return vec![(1, ArgRole::VarWrite)];
    }
    Vec::new()
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "add",
        arity: Arity::exact(4),
        detail: "Arrange for a command to be executed on the specified operation.",
        synopsis: "trace add type name ops commandPrefix",
        arg_role_resolver: Some(trace_add_arg_roles),
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
        arg_role_resolver: Some(trace_remove_arg_roles),
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
