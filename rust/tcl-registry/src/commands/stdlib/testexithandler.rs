//! `testexithandler` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testexithandler",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_CreateExitHandler.",
            &["testexithandler"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
