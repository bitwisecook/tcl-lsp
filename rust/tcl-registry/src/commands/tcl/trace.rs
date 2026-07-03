//! `trace` — monitor variable accesses, command usages and executions.
use crate::prelude::*;

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "trace option ?arg arg ...?",
}];

/// Arg-role resolver for `trace add`.
///
/// `trace add variable name ops commandPrefix` writes to `name` —
/// the trace handler can rewrite the variable at runtime, so SSA
/// must see `name` as a definition site.
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
/// registry consistency with `trace add variable` so consumers can
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
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(4),
        detail: "Arrange for a command to be executed on the specified operation.",
        synopsis: "trace add type name ops commandPrefix",
        arg_role_resolver: Some(trace_add_arg_roles),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(2),
        detail: "Return trace info for the given name.",
        synopsis: "trace info type name",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remove",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(4),
        detail: "Remove a trace.",
        synopsis: "trace remove type name ops commandPrefix",
        arg_role_resolver: Some(trace_remove_arg_roles),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "variable",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(3),
        detail: "Arrange for command to be executed whenever variable name is accessed. Deprecated in favour of trace add variable.",
        synopsis: "trace variable name ops command",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vdelete",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(3),
        detail: "Delete a variable trace. Deprecated in favour of trace remove variable.",
        synopsis: "trace vdelete name ops command",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vinfo",
        traits: Traits::TARGETS_VARIABLE_BY_NAME,
        arity: Arity::exact(1),
        detail: "Return trace information for the given variable. Deprecated in favour of trace info variable.",
        synopsis: "trace vinfo name",
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "trace",
        traits: Traits::CREATES_BARRIER | Traits::CREATES_DYNAMIC_BARRIER,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet {
            summary: "Monitor variable accesses, command usages and command executions",
            synopsis: &["trace option ?arg arg ...?"],
            snippet: "Arranges for commands to be executed whenever certain operations are invoked.",
            source: "Tcl man page trace.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}
