//! `testcmdinfo` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testcmdinfo",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_GetCommandInfo / Tcl_SetCommandInfo.",
            &["testcmdinfo"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
