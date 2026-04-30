//! `testfile` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testfile",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test file system operations.",
            &["testfile"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
