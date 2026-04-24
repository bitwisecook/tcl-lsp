//! `testfevent` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testfevent",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test file event handling.",
            &["testfevent"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
