//! `testupvar` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testupvar",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_UpVar / Tcl_UpVar2.",
            &["testupvar"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
