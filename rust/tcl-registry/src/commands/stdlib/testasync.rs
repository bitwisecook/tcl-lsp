//! `testasync` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testasync",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test asynchronous event handlers.",
            &["testasync"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
