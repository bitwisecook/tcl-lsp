//! `testlutil` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testlutil",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test list utility functions (9.0+).",
            &["testlutil"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
