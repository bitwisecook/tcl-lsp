//! `testinterpresolver` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testinterpresolver",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test the namespace / command resolver.",
            &["testinterpresolver"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
