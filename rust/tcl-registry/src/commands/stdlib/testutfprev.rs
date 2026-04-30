//! `testutfprev` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testutfprev",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_UtfPrev.",
            &["testutfprev"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
