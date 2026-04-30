//! `testevalobjv` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testevalobjv",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_EvalObjv.",
            &["testevalobjv"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
