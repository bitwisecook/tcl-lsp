//! `testutfnext` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testutfnext",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_UtfNext.",
            &["testutfnext"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
