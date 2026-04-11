//! `testfindlast` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testfindlast",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_FindLast / hash iteration.",
            &["testfindlast"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
