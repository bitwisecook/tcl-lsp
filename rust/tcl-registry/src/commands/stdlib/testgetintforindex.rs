//! `testgetintforindex` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testgetintforindex",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_GetIntForIndex (9.0+).",
            &["testgetintforindex"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
