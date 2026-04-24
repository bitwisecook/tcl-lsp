//! `testparsevar` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testparsevar",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_ParseVar.",
            &["testparsevar"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
