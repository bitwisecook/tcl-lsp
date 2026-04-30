//! `vwait` — wait for a variable to be modified.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "vwait",
        arity: Arity::exact(1),
        arg_roles: &[(0, ArgRole::VarRead)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Wait for a variable to be modified.",
            &["vwait varName"],
            "Tcl vwait(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}
