//! `testset2` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testset2",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_SetVar2.",
            &["testset2"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
