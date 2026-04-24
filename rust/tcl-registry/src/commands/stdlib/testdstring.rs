//! `testdstring` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testdstring",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_DString operations.",
            &["testdstring"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
