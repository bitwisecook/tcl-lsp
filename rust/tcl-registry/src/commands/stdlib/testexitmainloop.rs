//! `testexitmainloop` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testexitmainloop",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_SetMainLoop exit.",
            &["testexitmainloop"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
