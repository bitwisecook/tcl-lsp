//! `testmainthread` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testmainthread",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return the ID of the main thread.",
            &["testmainthread"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
