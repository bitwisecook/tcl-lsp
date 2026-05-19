//! `lpop` — get and remove an element from a list variable (Tcl 9.0+, TIP 323).

use crate::prelude::*;

/// Command spec for `lpop`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lpop",
        dialects: Some(DialectSet::TCL90),
        arity: Arity::at_least(1),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        return_type: Some(TclType::String),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet::brief(
            "Get and remove an element in a list variable.",
            &["lpop varName ?index ...?"],
            "Tcl 9 man page lpop.n",
        )),
        ..CommandSpec::DEFAULT
    }
}
