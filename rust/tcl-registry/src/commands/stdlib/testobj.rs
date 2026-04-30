//! `testobj` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testobj",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_Obj type operations.",
            &["testobj"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
