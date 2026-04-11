//! `testsetbytearraylength` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testsetbytearraylength",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_SetByteArrayLength.",
            &["testsetbytearraylength"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
