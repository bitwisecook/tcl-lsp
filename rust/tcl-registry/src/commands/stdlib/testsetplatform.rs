//! `testsetplatform` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testsetplatform",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_SetPlatform.",
            &["testsetplatform"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
