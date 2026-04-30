//! `testinterpdelete` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testinterpdelete",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_DeleteInterp.",
            &["testinterpdelete"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
