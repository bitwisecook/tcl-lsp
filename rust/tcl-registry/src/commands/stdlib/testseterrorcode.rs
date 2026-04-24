//! `testseterrorcode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testseterrorcode",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_SetErrorCode.",
            &["testseterrorcode"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
