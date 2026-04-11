//! `testservicemode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testservicemode",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test Tcl_SetServiceMode.",
            &["testservicemode"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
