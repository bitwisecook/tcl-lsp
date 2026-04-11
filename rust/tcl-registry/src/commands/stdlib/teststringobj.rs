//! `teststringobj` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "teststringobj",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test string Tcl_Obj operations.",
            &["teststringobj"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
