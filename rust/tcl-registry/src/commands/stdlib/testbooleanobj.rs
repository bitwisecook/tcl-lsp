//! `testbooleanobj` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testbooleanobj",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test boolean Tcl_Obj operations.",
            &["testbooleanobj"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
