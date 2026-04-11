//! `testsocket` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "testsocket",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Test socket operations (9.0+).",
            &["testsocket"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
