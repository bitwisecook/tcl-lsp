//! `teststringbytes` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "teststringbytes",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_GetStringFromObj byte length.",
            &["teststringbytes"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
