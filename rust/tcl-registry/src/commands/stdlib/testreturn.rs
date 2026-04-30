//! `testreturn` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testreturn",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_SetReturnOptions.",
            &["testreturn"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
