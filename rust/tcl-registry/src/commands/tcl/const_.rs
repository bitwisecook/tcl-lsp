//! `const` — define a constant variable (Tcl 9 / TIP 590).

use crate::prelude::*;

/// Command spec for `const`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "const",
        dialects: Some(DialectSet::TCL90),
        arity: Arity::new(2, 2),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Define a constant variable.",
            &["const varName value"],
            "Tcl const(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
