//! `namespace` — create and manipulate contexts for commands and variables.

use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "children",
        arity: Arity::new(0, 2),
        detail: "Returns a list of all child namespaces.",
        synopsis: "namespace children ?namespace? ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "code",
        arity: Arity::exact(1),
        detail: "Captures the current namespace context for later execution.",
        synopsis: "namespace code script",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "current",
        arity: Arity::exact(0),
        detail: "Returns the fully-qualified name for the current namespace.",
        synopsis: "namespace current",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "delete",
        arity: Arity::any(),
        detail: "Delete namespaces and their contents.",
        synopsis: "namespace delete ?namespace namespace ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "ensemble",
        arity: Arity::at_least(1),
        detail: "Creates and manipulates a command ensemble.",
        synopsis: "namespace ensemble subcommand ?arg ...?",
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eval",
        arity: Arity::at_least(2),
        detail: "Evaluate a script in a namespace context.",
        synopsis: "namespace eval namespace arg ?arg ...?",
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
        lowering_hook: Some(crate::hooks::LoweringHookId::NamespaceEval),
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Test whether a namespace exists.",
        synopsis: "namespace exists namespace",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "export",
        arity: Arity::any(),
        detail: "Specifies which commands are exported from a namespace.",
        synopsis: "namespace export ?-clear? ?pattern pattern ...?",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::any(),
        detail: "Removes previously imported commands from a namespace.",
        synopsis: "namespace forget ?pattern pattern ...?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "import",
        arity: Arity::any(),
        detail: "Imports commands into a namespace.",
        synopsis: "namespace import ?-force? ?pattern pattern ...?",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "inscope",
        arity: Arity::at_least(2),
        detail: "Executes a script in the context of the specified namespace.",
        synopsis: "namespace inscope namespace script ?arg ...?",
        arg_roles: &[(0, ArgRole::Name), (1, ArgRole::Body)],
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "origin",
        arity: Arity::exact(1),
        detail: "Returns the fully-qualified name of the original command.",
        synopsis: "namespace origin command",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "parent",
        arity: Arity::new(0, 1),
        detail: "Returns the fully-qualified name of the parent namespace.",
        synopsis: "namespace parent ?namespace?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "path",
        arity: Arity::new(0, 1),
        detail: "Returns the command resolution path of the current namespace.",
        synopsis: "namespace path ?namespaceList?",
        return_type: Some(TclType::List),
        dialects: Some(DialectSet::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "qualifiers",
        arity: Arity::exact(1),
        detail: "Returns any leading namespace qualifiers for string.",
        synopsis: "namespace qualifiers string",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tail",
        arity: Arity::exact(1),
        detail: "Returns the simple name at the end of a qualified string.",
        synopsis: "namespace tail string",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unknown",
        arity: Arity::new(0, 1),
        detail: "Sets or returns the unknown command handler for the current namespace.",
        synopsis: "namespace unknown ?script?",
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "upvar",
        arity: Arity::at_least(1),
        detail: "Arrange local variables to refer to namespace variables.",
        synopsis: "namespace upvar namespace ?otherVar myVar ...?",
        return_type: Some(TclType::String),
        creates_scope_alias: true,
        dialects: Some(DialectSet::TCL85_PLUS),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "which",
        arity: Arity::at_least(1),
        detail: "Looks up name as either a command or variable.",
        synopsis: "namespace which ?-command? ?-variable? name",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `namespace`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "namespace",
        traits: Traits::LANGUAGE_KEYWORD | Traits::NEVER_INLINE_BODY | Traits::HAS_DESTRUCTIVE_OPS,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Create and manipulate contexts for commands and variables.",
            &["namespace subcommand ?arg ...?"],
            "Tcl namespace(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
