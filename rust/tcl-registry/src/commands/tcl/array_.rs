//! `array` — manipulate array variables.

use crate::hooks::CodegenHookId;
use crate::prelude::*;

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "anymore",
        arity: Arity::exact(2),
        detail: "Returns 1 if there are any more elements left to be processed in an array search, 0 if all elements have already been returned.",
        synopsis: "array anymore arrayName searchId",
        return_type: Some(TclType::Boolean),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "default",
        arity: Arity::at_least(2),
        detail: "Manages the default value of the array.",
        synopsis: "array default subcommand arrayName args...",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        dialects: Some(DialectSet::TCL90),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "donesearch",
        arity: Arity::exact(2),
        detail: "Terminates an array search and destroys all the state associated with that search.",
        synopsis: "array donesearch arrayName searchId",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Returns 1 if arrayName is an array variable, 0 if there is no variable by that name or if it is a scalar variable.",
        synopsis: "array exists arrayName",
        return_type: Some(TclType::Boolean),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "for",
        arity: Arity::exact(3),
        detail: "Iterates over array entries. The first argument is a two-element list of variable names for the key and value of each entry.",
        synopsis: "array for {keyVariable valueVariable} arrayName body",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarRead), (2, ArgRole::Body)],
        loop_list_header: true,
        dialects: Some(DialectSet::TCL90),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "get",
        arity: Arity::new(1, 2),
        detail: "Returns a list containing pairs of elements.",
        synopsis: "array get arrayName ?pattern?",
        return_type: Some(TclType::List),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::new(1, 3),
        detail: "Returns a list containing the names of all of the elements in the array that match pattern.",
        synopsis: "array names arrayName ?mode? ?pattern?",
        return_type: Some(TclType::List),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nextelement",
        arity: Arity::exact(2),
        detail: "Returns the name of the next element in arrayName, or an empty string if all elements have already been returned in this search.",
        synopsis: "array nextelement arrayName searchId",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "set",
        arity: Arity::exact(2),
        detail: "Sets the values of one or more elements in arrayName.",
        synopsis: "array set arrayName list",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarWrite)],
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        arity: Arity::exact(1),
        detail: "Returns a decimal string giving the number of elements in the array.",
        synopsis: "array size arrayName",
        return_type: Some(TclType::Int),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "startsearch",
        arity: Arity::exact(1),
        detail: "Initializes an element-by-element search through the array given by arrayName.",
        synopsis: "array startsearch arrayName",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "statistics",
        arity: Arity::exact(1),
        detail: "Returns statistics about the distribution of data within the hashtable that represents the array.",
        synopsis: "array statistics arrayName",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarRead)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unset",
        arity: Arity::new(1, 2),
        detail: "Unsets all of the elements in the array that match pattern.",
        synopsis: "array unset arrayName ?pattern?",
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::VarWrite)],
        mutator: true,
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `array`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "array",
        traits: Traits::NOT_PROC_FACTORY | Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        assigns_variable_at: Some(1),
        inferred_storage_type: Some(StorageType::Array),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Manipulate array variables.",
            &["array option arrayName ?arg arg ...?"],
            "Tcl array(1)",
        )),
        codegen_hook: Some(CodegenHookId::Array),
        ..CommandSpec::DEFAULT
    }
}
