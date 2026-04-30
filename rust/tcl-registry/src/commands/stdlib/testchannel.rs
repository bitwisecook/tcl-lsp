//! `testchannel` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testchannel",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test channel introspection and manipulation.",
            &["testchannel"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
