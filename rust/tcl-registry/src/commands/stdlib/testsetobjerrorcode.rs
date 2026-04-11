//! `testsetobjerrorcode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testsetobjerrorcode",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_SetObjErrorCode.",
            &["testsetobjerrorcode"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
