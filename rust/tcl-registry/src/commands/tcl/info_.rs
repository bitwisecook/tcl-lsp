//! `info` — information about the state of the Tcl interpreter.

use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "args",
        arity: Arity::exact(1),
        detail: "Returns the names of the parameters to the procedure named procname.",
        synopsis: "info args procname",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "body",
        arity: Arity::exact(1),
        detail: "Returns the body of the procedure named procname.",
        synopsis: "info body procname",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "class",
        arity: Arity::at_least(2),
        detail: "Returns information about the class.",
        synopsis: "info class subcommand class ?arg ...?",
        pure: true,
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cmdcount",
        arity: Arity::exact(0),
        detail: "Returns the total number of commands evaluated in this interpreter.",
        synopsis: "info cmdcount",
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cmdtype",
        arity: Arity::exact(1),
        detail: "Returns the type of the command named commandName.",
        synopsis: "info cmdtype commandName",
        pure: true,
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL90),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "commands",
        arity: Arity::new(0, 1),
        detail: "Returns the names of all commands visible in the current namespace.",
        synopsis: "info commands ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "complete",
        arity: Arity::exact(1),
        detail: "Returns 1 if command is a complete command, and 0 otherwise.",
        synopsis: "info complete command",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "constant",
        arity: Arity::exact(1),
        detail: "Returns 1 if varName is a constant variable and 0 otherwise.",
        synopsis: "info constant varName",
        pure: true,
        return_type: Some(TclType::Boolean),
        dialects: Some(DialectSet::TCL90),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "consts",
        arity: Arity::new(0, 1),
        detail: "Returns the list of constant variables in the current scope.",
        synopsis: "info consts ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        dialects: Some(DialectSet::TCL90),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "coroutine",
        arity: Arity::exact(0),
        detail: "Returns the name of the current coroutine, or the empty string if there is no current coroutine.",
        synopsis: "info coroutine",
        pure: true,
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "default",
        arity: Arity::exact(3),
        detail: "If the parameter has a default value, stores that value in varname and returns 1.",
        synopsis: "info default procname parameter varname",
        return_type: Some(TclType::Boolean),
        arg_roles: &[(2, ArgRole::VarWrite)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "errorstack",
        arity: Arity::new(0, 1),
        detail: "Returns a description of the active command at each level from the call stack of the last error.",
        synopsis: "info errorstack ?interp?",
        pure: true,
        return_type: Some(TclType::List),
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Returns 1 if a variable named varName is visible and has been defined, and 0 otherwise.",
        synopsis: "info exists varName",
        pure: true,
        return_type: Some(TclType::Boolean),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "frame",
        arity: Arity::new(0, 1),
        detail: "Returns the depth of the call to info frame itself.",
        synopsis: "info frame ?depth?",
        pure: true,
        return_type: Some(TclType::Dict),
        dialects: Some(DialectSet::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "functions",
        arity: Arity::new(0, 1),
        detail: "Returns a list of all the math functions currently defined.",
        synopsis: "info functions ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "globals",
        arity: Arity::new(0, 1),
        detail: "Returns a list of all the names of currently-defined global variables.",
        synopsis: "info globals ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hostname",
        arity: Arity::exact(0),
        detail: "Returns the name of the current host.",
        synopsis: "info hostname",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "level",
        arity: Arity::new(0, 1),
        detail: "Returns the level this routine was called from.",
        synopsis: "info level ?level?",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "library",
        arity: Arity::exact(0),
        detail: "Returns the name of the library directory in which standard Tcl scripts are stored.",
        synopsis: "info library",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "loaded",
        arity: Arity::new(0, 2),
        detail: "Returns the name of each file loaded in interp by the load command.",
        synopsis: "info loaded ?interp? ?prefix?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "locals",
        arity: Arity::new(0, 1),
        detail: "Returns the name of each local variable matching pattern.",
        synopsis: "info locals ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nameofexecutable",
        arity: Arity::exact(0),
        detail: "Returns the absolute pathname of the program for the current interpreter.",
        synopsis: "info nameofexecutable",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "object",
        arity: Arity::at_least(2),
        detail: "Returns information about the object.",
        synopsis: "info object subcommand object ?arg ...?",
        pure: true,
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL86_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "patchlevel",
        arity: Arity::exact(0),
        detail: "Returns the value of the global variable tcl_patchLevel.",
        synopsis: "info patchlevel",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "procs",
        arity: Arity::new(0, 1),
        detail: "Returns the names of all visible procedures.",
        synopsis: "info procs ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "script",
        arity: Arity::new(0, 1),
        detail: "Returns the pathname of the innermost script currently being evaluated.",
        synopsis: "info script ?filename?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sharedlibextension",
        arity: Arity::exact(0),
        detail: "Returns the extension used on this platform for shared libraries.",
        synopsis: "info sharedlibextension",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tclversion",
        arity: Arity::exact(0),
        detail: "Returns the major and minor version of the Tcl library.",
        synopsis: "info tclversion",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vars",
        arity: Arity::new(0, 1),
        detail: "Returns the names of all visible variables.",
        synopsis: "info vars ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `info`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "info",
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Information about the state of the Tcl interpreter.",
            &["info option ?arg arg ...?"],
            "Tcl info(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
