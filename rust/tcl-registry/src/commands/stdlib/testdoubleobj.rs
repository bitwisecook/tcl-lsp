//! `testdoubleobj` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testdoubleobj",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test double Tcl_Obj operations.",
            &["testdoubleobj"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
