//! `testgetint` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testgetint",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_GetInt.",
            &["testgetint"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
