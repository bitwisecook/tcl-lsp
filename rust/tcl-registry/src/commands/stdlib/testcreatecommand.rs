//! `testcreatecommand` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testcreatecommand",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_CreateCommand.",
            &["testcreatecommand"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
