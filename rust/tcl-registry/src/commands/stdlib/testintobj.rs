//! `testintobj` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testintobj",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test integer Tcl_Obj operations.",
            &["testintobj"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
