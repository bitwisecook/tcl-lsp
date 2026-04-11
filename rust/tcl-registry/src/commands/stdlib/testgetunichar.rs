//! `testgetunichar` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testgetunichar",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_GetUniChar.",
            &["testgetunichar"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
