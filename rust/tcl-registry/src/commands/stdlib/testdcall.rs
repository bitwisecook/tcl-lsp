//! `testdcall` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testdcall",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_CallWhenDeleted.",
            &["testdcall"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
