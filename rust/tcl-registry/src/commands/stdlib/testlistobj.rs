//! `testlistobj` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testlistobj",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test list Tcl_Obj operations.",
            &["testlistobj"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
