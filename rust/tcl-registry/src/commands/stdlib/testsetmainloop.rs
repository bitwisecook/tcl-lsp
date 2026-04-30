//! `testsetmainloop` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testsetmainloop",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_SetMainLoop.",
            &["testsetmainloop"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
