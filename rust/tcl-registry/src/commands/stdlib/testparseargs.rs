//! `testparseargs` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testparseargs",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test argument parsing.",
            &["testparseargs"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
